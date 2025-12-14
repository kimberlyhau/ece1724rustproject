use rusqlite::{Connection, Result, params};
use std::error::Error;
use serde::{Serialize, Deserialize};
use axum::{
    extract::State,
    Json,
};
use std::sync::Arc;
use crate::state::AppState;

#[derive(Serialize, Deserialize, Clone)]
pub struct FetchRequest {
    pub username: String,
    pub chat_id: i32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HistoryRequest {
    pub username: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum FetchResponse {
    Success {messages: Vec<(i32, String, String)>},
    Error {message: String},
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HistoryResponse {
    pub chat_id: i32,
    pub latest_msg: String,
}

pub fn get_user_id(conn: &Connection, name: &str) -> Result<i32> {
    let user_id: i32 = conn.query_row(
        "SELECT id FROM users WHERE name = ?1",
        params![name],
        |row| row.get(0),
    )?;
    Ok(user_id)
}

// function to create user if
pub fn add_user(conn: &Connection, name: String) -> Result<()> {
    // check if user already exists
    let user_exists: bool = conn.query_row (
        "SELECT EXISTS(SELECT 1 FROM users WHERE name = ?1)",
        [name.as_str()],
        |row| row.get(0),
    )?;

    if user_exists {
        return Ok(());
    }

    conn.execute(
        "INSERT INTO users (name) VALUES (?1)",
        params![name],
    )?;
    Ok(())
}

// function to create model
pub fn add_model(conn: &Connection, model_name: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO models (model_name) VALUES (?1)",
        params![model_name],
    )?;
    Ok(())
}

pub fn next_chat_id(conn: &Connection, user_id: i32) -> Result<i32> {
    let next_id: i32 = conn.query_row(
        "SELECT COALESCE(MAX(chat_id), 0) + 1 FROM chats WHERE user_id = ?1",
        params![user_id],
        |row| row.get(0),
    )?;
    Ok(next_id)
}

pub fn add_message(conn: &Connection, username: String, model_id: i32, chat_id: i32, message: &str) -> Result<()> {
    let latest_msg_id: i32 = conn.query_row(
        "SELECT COALESCE(MAX(message_id), 0) FROM chats WHERE user_id = (SELECT id FROM users WHERE name = ?1) AND chat_id = ?2",
        params![username, chat_id],
        |row| row.get(0),
    )?;

    let user_id: i32 = conn.query_row(
        "SELECT id FROM users WHERE name = ?1",
        params![username],
        |row| row.get(0),
    )?;

    conn.execute(
        "INSERT INTO chats (user_id, model_id, chat_id, message_id, message, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, CURRENT_TIMESTAMP)",
        params![user_id, model_id, chat_id, latest_msg_id + 1, message],
    )?;
    Ok(())
}
 
pub fn retrieve_chat(conn: &Connection, user_id: i32, chat_id: i32) -> Result<Vec<(i32, String, String)>> {
    let mut messages: Vec<(i32, String, String)> = Vec::new();
    let mut stmt = conn.prepare(
        "SELECT message_id, message, timestamp FROM chats WHERE user_id = ?1 AND chat_id = ?2 ORDER BY message_id",
    )?;

    let message_iter = stmt.query_map([user_id, chat_id], |row| {
        Ok((
            row.get::<_, i32>(0)?,     // message_id
            row.get::<_, String>(1)?,  // message
            row.get::<_, String>(2)?,  // timestamp
        ))
    })?;

    for msg in message_iter {
        let (message_id, message, timestamp) = msg?;
        messages.push((message_id, message, timestamp));
    }
    Ok(messages)
}

pub async fn fetch_chat(
    State(state): State<Arc<AppState>>,
    Json(request): Json<FetchRequest>,
) -> Json<FetchResponse> {
    let conn = state.db_conn.lock().unwrap();
    // to validate fetch request, user exists in db, with valid chat id
    
    let user_id: i32 = match conn.query_row(
        "SELECT id FROM users WHERE name = ?1",
        params![request.username.as_str()],
        |row| row.get(0),
    ) {
        Ok(id) => id,
        Err(_) => {
            return Json(FetchResponse::Error{
                message : format!(
                    "no user exists with that name",
                ),
            });
        }
    };
    let chat_id: i32 = match conn.query_row(
        "SELECT DISTINCT chat_id FROM chats WHERE user_id = ?1 AND chat_id = ?2",
        params![user_id, request.chat_id],
        |row| row.get(0),
    ) {
        Ok(id) => id,
        Err(_) => {
            return Json(FetchResponse::Error{
                message : format!(
                    "no chat history found for user `{}` with chat ID `{}`",
                    request.username, request.chat_id
                ),
            });
        }
    };

    let messages = match retrieve_chat(&conn, user_id, chat_id) {
        Ok(msgs) => msgs,
        Err(_) => {
            return Json(FetchResponse::Error{
                message : format!(
                    "failed to retrieve chat history",
                )
            });
        }
    };

    Json(FetchResponse::Success { messages } )
}

pub async fn fetch_history(
    State(state): State<Arc<AppState>>,
    Json(request): Json<HistoryRequest>,
) -> Json<Vec<HistoryResponse>> {
    let conn = state.db_conn.lock().unwrap();
    let user_id: i32 = match get_user_id(&conn, &request.username) {
        Ok(id) => id,
        // If the user doesn't exist yet, they simply have no history.
        Err(_) => return Json(Vec::new()),
    };

    let mut stmt = match conn.prepare(
        "SELECT chat_id, message FROM chats
        WHERE user_id = ?1 AND message_id = 
            (SELECT MAX(message_id) 
            FROM chats AS c2
            WHERE c2.user_id = ?1 AND c2.chat_id = chats.chat_id
            )
        ORDER BY chat_id",
    ) {
        Ok(stmt) => stmt,
        Err(_) => return Json(Vec::new()),
    };

    let history_iter = match stmt.query_map([user_id], |row| {
        Ok(HistoryResponse {
            chat_id: row.get(0)?,
            latest_msg: row.get(1)?,
        })
    }) {
        Ok(iter) => iter,
        Err(_) => return Json(Vec::new()),
    };

    let mut history = Vec::new();

    for item in history_iter {
        if let Ok(record) = item {
            history.push(record);
        }
    }

    Json(history)
}
    
/* 
pub fn delete_chat(conn: &Connection, user_id: i32, chat_id: i32) -> Result<()> {
    let mut stmt = conn.prepare(
        "DELETE FROM chats WHERE user_id = ?1 AND chat_id = ?2",
    )?;

    stmt.execute([user_id, chat_id])?;

    Ok(())
}

// also deletes all chat history associated with the user
pub fn delete_user(conn: &Connection, user_id: i32) -> Result<()> {
    conn.execute(
        "DELETE FROM chats WHERE user_id = ?1", params![user_id],
    )?;
    conn.execute(
        "DELETE FROM users WHERE id = ?1", params![user_id],
    )?;
    Ok(())
}
*/

pub fn initialize_database() -> Result<Connection, Box<dyn Error>> {
    // initialize connection to database
    let conn = Connection::open("chats.sqlite")?;

    // create table to store users
    conn.execute( // execute runs SQL statement
        "
        CREATE TABLE IF NOT EXISTS users ( 
            id          INTEGER PRIMARY KEY,
            name        TEXT NOT NULL
        );
        ",
        [], // no parameters
    )?;

    // create table to store models
    conn.execute(
        "
        CREATE TABLE IF NOT EXISTS models (
            id          INTEGER PRIMARY KEY,
            model_name  TEXT NOT NULL
        );
        ",
        [],
    )?;

    // create table to store chats 
    conn.execute(
        "
        CREATE TABLE IF NOT EXISTS chats (
            id                      INTEGER PRIMARY KEY,
            user_id                 INTEGER NOT NULL,
            model_id                INTEGER NOT NULL,
            chat_id                 INTEGER NOT NULL,
            message_id              INTEGER NOT NULL,
            message                 TEXT NOT NULL,
            timestamp               DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(user_id)    REFERENCES users(id)
            FOREIGN KEY(model_id)   REFERENCES models(id)
        );
        ",
        [],
    )?;

    Ok(conn)
}
