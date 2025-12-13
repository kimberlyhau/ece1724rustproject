use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;

use crate::engine::ClientRequest;

pub struct AppState {
    pub db_conn: Arc<Mutex<rusqlite::Connection>>,
    // channel sender to send client /generate requests into the engine worker thread
    pub client_request_sender: Sender<ClientRequest>,
}

impl AppState {
    pub fn new(
        db_conn: rusqlite::Connection,
        client_request_sender: Sender<ClientRequest>,
    ) -> Self {
        Self {
            db_conn: Arc::new(Mutex::new(db_conn)),
            client_request_sender,
        }
    }
}

