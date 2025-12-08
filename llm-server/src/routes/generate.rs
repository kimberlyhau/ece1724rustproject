use std::{sync::Arc, time::Duration};

use axum::{
    extract::State,
    Error,
    response::sse::{Event, KeepAlive, Sse},
    Json,
};
use serde_json::json;
use tokio::sync::mpsc;
use tokio::task;
use tokio_stream::{wrappers::UnboundedReceiverStream, Stream, StreamExt};
use std::sync::Mutex;

use crate::{
    engine::{EventToServer, EXAMPLE_MODEL},
    state::AppState,
    types::GenerateRequest,
    chat_history::{add_message, add_user, next_chat_id, get_user_id},
};

// axum handler that bridges HTTP requests with the blocking inference engine
pub async fn generate(
    State(state): State<Arc<AppState>>,
    Json(request): Json<GenerateRequest>,
) -> Sse<impl Stream<Item = Result<Event, Error>>> {
    // Channel for engine (server) to send events and HTTP handler to read (client)
    let (sender, receiver) = mpsc::unbounded_channel();

    // VALIDATE USER REQUEST
    // 1. prompt not empty
    // 2. model is supported and loaded
    // send error events if invalid
    let mut invalid = false;
    if request.prompt.trim().is_empty() {
        let _ = sender.send(EventToServer::Error {
            message: "prompt must not be empty".to_string(),
        });
        invalid = true;
    } else if let Some(model) = request.model.as_deref() {
        // TODO: handle more models
        if model != EXAMPLE_MODEL {
            let _ = sender.send(EventToServer::Error {
                message: format!(
                    "requested model `{model}` is unavailable, only `{EXAMPLE_MODEL}` is loaded"
                ),
            });
            invalid = true;
        }
    }

    add_user(&state.db_conn.lock().unwrap(), request.username.clone()).unwrap();
    let user_id = get_user_id(&state.db_conn.lock().unwrap(), &request.username).unwrap();
    let chat_id = next_chat_id(&state.db_conn.lock().unwrap(), user_id).unwrap();

    // START GENERATION IN BLOCKING THREAD
    if !invalid {
        let blocking_state = Arc::clone(&state);
        let blocking_sender = sender.clone();
        let blocking_request = request.clone();
        let _ = task::spawn_blocking(move || {
            // store request in sqlite database
            add_message(&blocking_state.db_conn.lock().unwrap(), blocking_request.username.clone(), 1, chat_id, &blocking_request.prompt).unwrap();
            start_generation(blocking_state, blocking_request, blocking_sender);
        });
    }
    // close sender used for validation
    drop(sender);
    
    let streaming_buffer = Arc::new(Mutex::new(String::new()));
    let streaming_request = request.clone();

    // STREAM RESPONSES TO CLIENT
    // convert server events into SSE events (json payloads)
    let sse_stream = UnboundedReceiverStream::new(receiver).map(move |event| -> Result<Event, Error> {
        let mut buffer = streaming_buffer.lock().unwrap();
        // store generated tokens in buffer to store full response in database
        let payload = match event {
            EventToServer::Token { token, index } => {
                buffer.push_str(&token);
                json!({ "token": token, "index": index }).to_string()
            }
            EventToServer::Done { total_tokens } => {
                add_message(
                    &state.db_conn.lock().unwrap(), 
                    streaming_request.username.clone(), 
                    1,  // hardcoded to model 1 tiny llama ... for now
                    chat_id,
                    &buffer).unwrap();
                buffer.clear();
                json!({ "done": true, "total_tokens": total_tokens }).to_string()
            }
            EventToServer::Error { message } => json!({ "error": message }).to_string(),
        };
        Ok(Event::default().data(payload))
    });

    // Axum keeps the HTTP response open and pushes each SSE event so clients see streamed tokens
    Sse::new(sse_stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

fn start_generation(
    state: Arc<AppState>,
    request: GenerateRequest,
    sender: mpsc::UnboundedSender<EventToServer>,
) {
    // each thread increments ARC to access the shared engine
    let engine = Arc::clone(&state.engine);
    // lock engine for single prompt generation
    let Ok(mut engine_lock) = engine.lock() else {
        let _ = sender.send(EventToServer::Error {
            message: "failed to get inference engine".to_string(),
        });
        return;
    };
    // start gen
    if let Err(err) = engine_lock.generate(&request.prompt, &request.params, &sender) {
        let _ = sender.send(EventToServer::Error {
            message: err.to_string(),
        });
    }
}
