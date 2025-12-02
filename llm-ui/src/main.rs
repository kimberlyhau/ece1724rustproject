
use anyhow::{anyhow, Context, Result};
use crossterm::{
    event::{self, Event, KeyCode,KeyEventKind},
    terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    execute,
};
use futures::StreamExt;
use ratatui::Terminal;
use ratatui::{
    layout::{Constraint, Layout, Position},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, List, ListItem, Borders, Paragraph},
};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::io;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use ratatui::widgets::Wrap;

struct App {
    visible_text: String,
    input: String,
    /// Position of cursor in the editor area.
    character_index: usize,
    /// Current input mode
    input_mode: InputMode,
    /// History of recorded messages
    messages: Vec<String>,
}

enum InputMode {
    Normal,
    Editing,
    Processing,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    color_eyre::install()?;
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Channel for streaming text
    let (tx, mut rx) = mpsc::channel::<String>(100);

    let mut app = App::new();

    loop {
        terminal.draw(|frame| {
            let vertical = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Min(1),
            ]);
            let [help_area, input_area, messages_area, response_area] = vertical.areas(frame.area());
            let (msg, style) = match app.input_mode {
                InputMode::Normal => (
                    vec![
                        "Press ".into(),
                        "q".bold(),
                        " to exit, ".into(),
                        "e".bold(),
                        " to start editing.".bold(),
                    ],
                    Style::default().add_modifier(Modifier::RAPID_BLINK),
                ),
                InputMode::Editing => (
                    vec![
                        "Press ".into(),
                        "Esc".bold(),
                        " to stop editing, ".into(),
                        "Enter".bold(),
                        " to record the message".into(),
                    ],
                    Style::default(),
                ),
                InputMode::Processing => (
                    vec![
                        "Processing ".into(),
                    ],
                    Style::default(),
                ),
            };
            let text = Text::from(Line::from(msg)).patch_style(style);
            let help_message = Paragraph::new(text);
            frame.render_widget(help_message, help_area);
            let input = Paragraph::new(app.input.as_str())
                .style(match app.input_mode {
                    InputMode::Normal => Style::default(),
                    InputMode::Editing => Style::default().fg(Color::Yellow),
                    InputMode::Processing => Style::default(),
                })
                .block(Block::bordered().title("Input"));
            frame.render_widget(input, input_area);
            match app.input_mode {
            // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
                InputMode::Normal => {}

                // Make the cursor visible and ask ratatui to put it at the specified coordinates after
                // rendering
                #[allow(clippy::cast_possible_truncation)]
                InputMode::Editing => frame.set_cursor_position(Position::new(
                    // Draw the cursor at the current position in the input field.
                    // This position is can be controlled via the left and right arrow key
                    input_area.x + app.character_index as u16 + 1,
                    // Move one line down, from the border to the input line
                    input_area.y + 1,
                )),
                InputMode::Processing => {}
            }

            let messages: Vec<ListItem> = app
                .messages
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    let content = Line::from(Span::raw(format!("{i}: {m}")));
                    ListItem::new(content)
                })
                .collect();
            let messages = List::new(messages).block(Block::bordered().title("Messages"));
            frame.render_widget(messages, messages_area);
            let paragraph = Paragraph::new(app.visible_text.clone())
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title("Async Stream Output"));

            frame.render_widget(paragraph, response_area);
        })?;


        // Handle keypress (quit on q)
        if event::poll(Duration::from_millis(1))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
                match app.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('e') => {
                            app.input_mode = InputMode::Editing;
                        }
                        KeyCode::Char('q') => {
                            break;
                        }
                        _ => {}
                    },
                    InputMode::Editing if key.kind == KeyEventKind::Press => match key.code {
                        KeyCode::Enter => {app.submit_message(tx.clone());
                        },
                        KeyCode::Char(to_insert) => app.enter_char(to_insert),
                        KeyCode::Backspace => app.delete_char(),
                        KeyCode::Left => app.move_cursor_left(),
                        KeyCode::Right => app.move_cursor_right(),
                        KeyCode::Esc => app.input_mode = InputMode::Normal,
                        _ => {}
                    },
                    InputMode::Editing => {},
                    InputMode::Processing => {},
                }
            }
        }

        // Non-blocking receive from async task
        while let Ok(c) = rx.try_recv() {
            if c=="Thread work complete!"{
                app.input_mode = InputMode::Normal;
            }
            else {
                app.push_str(c);
            }
        }

        // Tiny sleep to avoid hot loop
        tokio::time::sleep(Duration::from_millis(5)).await;
        
    }
 
    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    // ratatui::restore();

    Ok(())
}


impl App {
    fn new() -> Self {
        Self { visible_text: String::new(),
        input: String::new(),
        input_mode: InputMode::Normal,
        messages: Vec::new(),
        character_index: 0, }
    }

    fn push_char(&mut self, c: char) {
        self.visible_text.push(c);
    }
    fn push_str(&mut self, c: String) {
        self.visible_text.push_str(&c);
    }
    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    fn enter_char(&mut self, new_char: char) {
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

    fn delete_char(&mut self) {
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

    fn reset_cursor(&mut self) {
        self.character_index = 0;
    }

    fn submit_message(&mut self, tx: mpsc::Sender<String>) {
        self.input_mode = InputMode::Processing;
        self.messages.push(self.input.clone());
        let input = self.input.clone();
        self.input.clear();
        self.reset_cursor();
        // eprintln!("Debug information: {:?}", input);
        tokio::spawn(async move {
            run_llm(tx, input).await;
            // async_text_stream(tx, input);
        });
    }
}

// Struct for deserializing server responses
#[derive(Debug, Deserialize)]
struct ServerResponses {
    #[serde(default)]
    token: Option<String>,
    #[serde(default)]
    done: Option<bool>,
}

async fn run_llm(tx: mpsc::Sender<String>, input:String) -> Result<()>{
    let prompt = "<s>[INST] <<SYS>>You are a helpful assistant.<</SYS>> ".to_owned()+&input+"[/INST]";
    tx.send(prompt.to_string()).await.ok();

    // send HTTP POST request with prompt to llm-server
    let addr = "127.0.0.1:4000";
    let prompt_post_url = format!("http://{addr}/generate");
    let client = Client::new();
    let response = client
        .post(&prompt_post_url)
        .json(&json!({ "prompt": prompt }))
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
