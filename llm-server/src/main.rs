mod engine;
mod routes;
mod state;
mod types;

use std::sync::Arc;

use axum::{routing::get, routing::post, Router};
use routes::generate::generate;
use state::AppState;
use tokio::net::TcpListener;

async fn test() -> &'static str {
    "llm gen/streaming server is up and reachable"
}

#[tokio::main]
async fn main() {
    // atomic ref count of app state for shared access across async tasks (if multiple clients send prompts)
    let engine = engine::InferenceEngine::new().expect("failed to initialize inference engine");
    let state = Arc::new(AppState::new(engine));

    // axum router: test route and generation route
    let router = Router::new()
        .route("/", get(test))
        .route("/generate", post(generate))
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
