use std::sync::{Arc, Mutex};

use crate::engine::InferenceEngine;

pub struct AppState {
    pub engine: Arc<Mutex<InferenceEngine>>,
}

impl AppState {
    pub fn new(engine: InferenceEngine) -> Self {
        Self {
            // mutex to lock the engine to single client prompt, 
            // now generation only happens one prompt at a time
            engine: Arc::new(Mutex::new(engine)),
        }
    }
}
