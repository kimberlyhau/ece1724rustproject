use rusqlite::{params, Connection, Result};
use std::error::Error;

// function to create user
pub fn add_user(conn: &Connection, name: &str) -> Result<()> {
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
