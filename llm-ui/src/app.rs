use tokio::sync::mpsc;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use futures::StreamExt;
use anyhow::Result;
use ratatui::style::{Color, Style, Modifier};
use ratatui::widgets::ListState;
use std::time::Instant;

pub enum InputMode {
    Normal,
    Editing,
    Processing,
    ColourSelection,
    MainMenu,
    Fetching,
}

pub enum Screen {
    SignIn,
    History,
    Chat,
    ColourSelection,
    MainMenu,
}

pub enum ChatOutcome {
    Continue,
    Quit,
}

pub enum ButtonState {
    Normal,
    Focused,
}

pub static OPTIONS: &[Color] = &[
        Color::Red,
        Color::Green,
        Color::Yellow,
        Color::Blue,
        Color::Magenta,
        Color::Cyan,
        Color::Gray,
        Color::LightRed,
        Color::LightGreen,
        Color::LightYellow,
        Color::LightBlue,
        Color::LightMagenta,
        Color::LightCyan,
        Color::White,
    ];

// Struct for deserializing server responses
#[derive(Debug, Deserialize)]
struct ServerResponses {
    #[serde(default)]
    token: Option<String>,
    #[serde(default)]
    done: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct HistoryResponse {
    chat_id: i32,
    latest_msg: String,
}

#[derive(Debug, Deserialize)]
pub struct NextChatIdResponse {
    pub chat_id: i32,
}

#[derive(Debug, Deserialize)]
pub enum FetchResponses {
    Success {messages: Vec<(i32, String, String)>},
    Error {message: String},
}

pub struct Button {
    pub label: &'static str,
    pub state: ButtonState,
}
impl Button {
    pub fn new(label: &'static str) -> Self {
        Button {
            label,
            state: ButtonState::Normal,
        }
    }

    pub fn style(&self) -> Style {
        match self.state {
            ButtonState::Normal => Style::default().fg(Color::White),
            ButtonState::Focused => Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        }
    }
}


pub struct App {
    //pub visible_text: String,
    pub llm_messages: Vec<String>,
    pub input: String,
    /// Position of cursor in the editor area.
    pub character_index: usize,
    /// Current input mode
    pub input_mode: InputMode,
    /// History of recorded messages
    pub messages: Vec<String>,
    pub history_messages: Vec<String>,

    pub username: String,
    pub screen: Screen,

    pub user_colour:Color,
    pub llm_colour:Color,

    pub selected_flags: Vec<bool>,
    pub state: ListState,
    pub user_colour_pick:Option<Color>,
    
    pub scroll_offset: u16,
    pub receiving: String,
    pub chat_id: Option<i32>,

    // Tracking for current streamed response
    pub token_count: u64,
    pub stream_start: Option<Instant>,

    pub buttons: Vec<Button>,
    pub selected_button: usize,
}

impl App {
    pub fn new() -> Self {
        Self { llm_messages: Vec::new(),
        input: String::new(),
        input_mode: InputMode::Normal,
        messages: Vec::new(),
        history_messages: Vec::new(),
        character_index: 0,
        username: String::new(), 
        screen: Screen::SignIn,
        user_colour: Color::Red,
        llm_colour: Color::Green,
        selected_flags: Vec::new(),
        state: ListState::default(),
        user_colour_pick: None,
        scroll_offset: 0,
        receiving: String::new(),
        chat_id: None,
        token_count: 0,
        stream_start: None,
        buttons: vec![
                Button::new("New Chat"),
                Button::new("Resume Chat from History"),
                Button::new("Text Colour Selection"),
                Button::new("Quit"),
        ],
        selected_button: 0,
        }
    }

    fn update_button_states(&mut self) {
        for (i, btn) in self.buttons.iter_mut().enumerate() {
            if i == self.selected_button {
                btn.state = ButtonState::Focused;
            } else {
                btn.state = ButtonState::Normal;
            }
        }
    }

    pub fn next_button(&mut self) {
        self.selected_button = (self.selected_button + 1).min(self.buttons.len() - 1);
        self.update_button_states();
    }

    pub fn previous_button(&mut self) {
        if self.selected_button > 0 {
            self.selected_button -= 1;
        }
        self.update_button_states();
    }

    pub fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    pub fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    pub fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.input.insert(index, new_char);
        self.move_cursor_right();
    }

    /// Returns the byte index based on the character position.
    ///
    /// Since each character in a string can be contain multiple bytes, it's necessary to calculate
    /// the byte index based on the index of the character.
    fn byte_index(&self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input.len())
    }

    pub fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.character_index != 0;
        if is_not_cursor_leftmost {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.

            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;

            // Getting all characters before the selected character.
            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = self.input.chars().skip(current_index);

            // Put all characters together except the selected one.
            // By leaving the selected one out, it is forgotten and therefore deleted.
            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }
        fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.chars().count())
    }

    pub fn reset_cursor(&mut self) {
        self.character_index = 0;
    }

    pub fn submit_message(&mut self, tx: mpsc::Sender<String>) {
        self.input_mode = InputMode::Processing;
        self.messages.push(self.input.clone());
        // reset tok/s for new response
        self.token_count = 0;
        self.stream_start = None;
        let input = self.input.clone();
        self.input.clear();
        self.reset_cursor();
        let username = self.username.clone();
        let chat_id = self.chat_id;
        // eprintln!("Debug information: {:?}", input);
        tokio::spawn(async move {
            let _ = run_llm(tx, input, username, chat_id).await;
            // async_text_stream(tx, input);
        });
    }

    pub fn set_profile(&mut self, tx: mpsc::Sender<String>) {
        self.username = self.input.clone();
        self.input.clear();
        self.reset_cursor();
        self.input_mode = InputMode::Normal;
        let username = self.username.clone();
        tokio::spawn(async move {
            let _ = run_history(tx, username).await;
        });
    }

    pub fn start_new_chat(&mut self) {
        self.input_mode = InputMode::Normal;
        self.llm_messages.clear();
        self.input.clear();
        self.reset_cursor();
        self.screen = Screen::Chat;
    }

    pub fn fetch_chat(&mut self, tx: mpsc::Sender<String>) {
        self.input_mode = InputMode::Normal;
        self.llm_messages.clear();
        let input = self.input.clone();
        let chat_id: i32 = input.trim().parse().unwrap_or(0);
        self.chat_id = if chat_id > 0 { Some(chat_id) } else { None };
        self.input.clear();
        self.reset_cursor();
        let username = self.username.clone();
        tokio::spawn(async move {
            let _ = run_chat(tx, input, username).await;
        });
    }
}

pub async fn get_next_chat_id_for_user(username: String) -> Result<i32> {
    let addr = "127.0.0.1:4000";
    let url = format!("http://{addr}/next_chat_id");
    let client = Client::new();
    let response = client
        .get(&url)
        .query(&[("username", username)])
        .send()
        .await?;

    let body = response.json::<NextChatIdResponse>().await?;
    Ok(body.chat_id)
}

async fn run_llm(tx: mpsc::Sender<String>, input:String, username:String, chat_id: Option<i32>) -> Result<()>{
    let prompt = "<s>[INST] <<SYS>>You are a helpful assistant.<</SYS>> ".to_owned()+&input+" [/INST]";
    // tx.send(prompt.to_string()).await.ok();

    // send HTTP POST request with prompt to llm-server
    let addr = "127.0.0.1:4000";
    let prompt_post_url = format!("http://{addr}/generate");
    let client = Client::new();
    let response = client
        .post(&prompt_post_url)
        .json(&json!({ "prompt": prompt , "username": username, "chat_id": chat_id}))
        .send()
        .await?;

    // read server response stream
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let line = std::str::from_utf8(&chunk).unwrap_or("").trim();
        
        // process only responses that start with data and have content
        if line.is_empty() || !line.starts_with("data:") {
            continue;
        }
        let payload = line.trim_start_matches("data:").trim();
        if payload.is_empty() {
            continue;
        }

        // deserialize server response and send token to UI
        if let Ok(message) = serde_json::from_str::<ServerResponses>(payload) {
            if let Some(token) = message.token {
                tx.send(token).await.ok();
            }
            // response finished when done token received
            if message.done.unwrap_or(false) {
                tx.send("Thread work complete!".to_string()).await.ok();
                return Ok(());
            }
        }
    }
    tx.send("Thread work complete!".to_string()).await.ok();
    Ok(())
}


async fn run_history(tx: mpsc::Sender<String>, username:String) -> Result<()> {
    let addr = "127.0.0.1:4000";
    let prompt_post_url = format!("http://{addr}/history");
    let client = Client::new();
    let response = client
        .post(&prompt_post_url)
        .json(&json!({ "username": username}))
        .send()
        .await?;
    
    let fetched_history = response.json::<Vec<HistoryResponse>>().await?;

    for chat in fetched_history {
        let formatted = format!("Chat ID: {} | Latest Message: {}\n",
                                 chat.chat_id, chat.latest_msg);
        tx.send(formatted).await.ok();
    }
    tx.send("Thread work complete!".to_string()).await.ok();
    Ok(())
}

async fn run_chat(tx: mpsc::Sender<String>, input:String, username:String) -> Result<()> {
    let chat_id: i32 = input.trim().parse().unwrap_or(0);

    let addr = "127.0.0.1:4000";
    let prompt_post_url = format!("http://{addr}/fetch");
    let client = Client::new();
    let response = client
        .post(&prompt_post_url)
        .json(&json!({ "username": username, "chat_id": chat_id}))
        .send()
        .await?;
    let messages = response.json::<FetchResponses>().await?;

    match messages {
        FetchResponses::Success {messages} => {
            for (msg_id, msg, _timestamp) in messages {
                tx.send(format!("[{}]: {}\n", msg_id, msg)).await.ok();
            }
        },
        FetchResponses::Error {message} => {
            tx.send(format!("Error fetching chat history: {}", message)).await.ok();
        }
    }
    
    tx.send("Thread work complete!".to_string()).await.ok();
    Ok(())
}