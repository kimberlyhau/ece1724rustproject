use std::sync::{Arc, Mutex};

use crate::engine::InferenceEngine;

pub struct AppState {
    pub engine: Arc<InferenceEngine>,
    pub db_conn: Arc<Mutex<rusqlite::Connection>>,
}

impl AppState {
    pub fn new(engine: InferenceEngine, db_conn: rusqlite::Connection) -> Self {
        Self {
            // engine handles prefill/decode model syncronization between client requests using internal locking
            engine: Arc::new(engine),
            db_conn: Arc::new(Mutex::new(db_conn)),
        }
    }
}

