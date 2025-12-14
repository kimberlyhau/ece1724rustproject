mod engine;
mod routes;
mod state;
mod types;
mod chat_history;

use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

use axum::{routing::get, routing::post, Router};
use routes::generate::generate;
use state::AppState;
use tokio::net::TcpListener;
use chat_history::{add_model, fetch_chat, fetch_history};


async fn test() -> &'static str {
    "llm gen/streaming server is up and reachable"
}

#[tokio::main]
async fn main() {
    // shared inference engine instance used by the worker thread
    let engine = Arc::new(
        engine::InferenceEngine::new().expect("failed to initialize inference engine"),
    );
    let (client_request_sender, client_request_receiver) =
        mpsc::channel::<engine::ClientRequest>();

    // spawn an engine worker thread that receives client requests from /generate HTTP handler
    // engine internally manages model synchronization (prefill, decode) between concurrent requests
    let worker_engine = Arc::clone(&engine);
    thread::spawn(move || {
        while let Ok(client_request) = client_request_receiver.recv() {
            if let Err(err) = worker_engine.generate(
                &client_request.prompt,
                &client_request.params,
                &client_request.sender,
            ) {
                let _ = client_request.sender.send(engine::EventToServer::Error {
                    message: err.to_string(),
                });
            }
        }
    });

    // initialize sqlite database to store chat history
    let conn =
        chat_history::initialize_database().expect("failed to initialize sqlite database");
    // test example model
    add_model(&conn, "TinyLlama-1.1B-Chat-v1.0").unwrap();

    let state = Arc::new(AppState::new(conn, client_request_sender));

    // axum router: test route and generation route
    let router = Router::new()
        .route("/", get(test))
        .route("/generate", post(generate))
        .route("/fetch", post(fetch_chat))
        .route("/history", post(fetch_history))
        .with_state(Arc::clone(&state));

    let addr = "127.0.0.1:4000";
    println!("LLM streaming server listening on http://{addr}");

    let listener = TcpListener::bind(addr)
        .await
        .expect("failed to bind TCP listener");

    // serve requests so new prompts are forwarded to the engine
    axum::serve(listener, router)
        .await
        .expect("failed to start server");
}

