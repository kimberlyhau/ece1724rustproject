use std::sync::{Arc, Mutex};

use crate::engine::InferenceEngine;

pub struct AppState {
    pub engine: Arc<Mutex<InferenceEngine>>,
    pub db_conn: Arc<Mutex<rusqlite::Connection>>,
}

impl AppState {
    pub fn new(engine: InferenceEngine, db_conn: rusqlite::Connection) -> Self {
        Self {
            // mutex to lock the engine to single client prompt, 
            // now generation only happens one prompt at a time
            engine: Arc::new(Mutex::new(engine)),
            db_conn: Arc::new(Mutex::new(db_conn)),
        }
    }
}
