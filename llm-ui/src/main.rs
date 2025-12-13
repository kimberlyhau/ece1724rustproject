
use anyhow::{Result};
use ratatui::Terminal;
use std::io;
use tokio::sync::mpsc;
use crossterm::{
    terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    execute,
};

mod app;
mod chat_screen;
mod signin_screen;
mod history_screen;
mod colour_screen;
mod key_handler;

use app::{App, ChatOutcome, Screen};

use futures::StreamExt;
use ratatui::Terminal;
use ratatui::{
    layout::{Constraint, Layout, Position,  Rect, Direction},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, List, ListItem, Borders, Paragraph, ListState, Wrap},
    prelude::Alignment,
};
use tui_big_text::{BigText, PixelSize};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::io;
use tokio::sync::mpsc;
use tokio::time::{Duration};
use itertools::Itertools;
use itertools::EitherOrBoth::{Both, Left, Right};

struct App {
    // History of responses from llm
    llm_messages: Vec<String>,
    input: String,
    // Position of cursor in the editor area.
    character_index: usize,
    // Current input mode
    input_mode: InputMode,
    // History of prompts
    messages: Vec<String>,
    user_colour:Color,
    llm_colour:Color,
    // For Menu buttons
    buttons: Vec<Button>,
    selected_button: usize,
}

enum InputMode {
    Normal,
    Editing,
    Fetching,
    Processing,
    ColourSelection,
    MainMenu
}

enum ButtonState {
    Normal,
    Focused,
}

struct Button {
    label: &'static str,
    state: ButtonState,
}

impl Button {
    fn new(label: &'static str) -> Self {
        Button {
            label,
            state: ButtonState::Normal,
        }
    }

    fn style(&self) -> Style {
        match self.state {
            ButtonState::Normal => Style::default().fg(Color::White),
            ButtonState::Focused => Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        }
    }
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
    let user:&str = "You: ";
    let llm:&str = "LLM: ";
    let mut receiving = String::new();

    let mut scroll_offset: u16 = 0;
    let options = vec![
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
    //reset when colour picking
    let mut selected_flags = vec![false; options.len()];
    let mut state = ListState::default();
    state.select(Some(0));
    let mut user_colour_pick:Option<Color> = None;

    loop {
        match app.input_mode{
            InputMode::MainMenu => {
                terminal.draw(|frame| {
                    let vertical = Layout::vertical([
                            Constraint::Min(1),
                            Constraint::Min(12),
                        ]);
                    let [title_banner,buttons] = vertical.areas(frame.area());
                    let cols = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Percentage(35),
                            Constraint::Percentage(30), // center column
                            Constraint::Percentage(35),
                        ])
                        .split(buttons);

                    // Vertical stack of buttons inside center column
                    let button_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Percentage(25),
                            Constraint::Percentage(25),
                            Constraint::Percentage(25),
                            Constraint::Percentage(25),
                        ])
                        .split(cols[1]);
                    let title = BigText::builder().centered()
                                .pixel_size(PixelSize::Quadrant)
                                .style(Style::new().light_blue())
                                .lines(vec![
                                    "LLM Chat Interface".into(),
                                    "~~~~~~~".white().into(),
                                ])
                                .build();
                    frame.render_widget(title, title_banner);

                    for (i, btn) in app.buttons.iter().enumerate() {
                        let block = Block::default()
                            .borders(Borders::ALL)
                            .style(btn.style())
                            .border_style(btn.style());

                        let text = Paragraph::new(btn.label).style(btn.style()).alignment(Alignment::Center).block(block);

                        frame.render_widget(text, button_chunks[i]);
                    }
                })?;
            }
            InputMode::ColourSelection => {
                terminal.draw(|frame| {
                    let vertical = Layout::vertical([
                        Constraint::Min(1),
                        Constraint::Length(5),
                    ]).split(frame.area());

                    let list_area = vertical[0];
                    let display_area = vertical[1];

                    let items: Vec<ListItem> = options
                        .iter()
                        .enumerate()
                        .map(|(i, &opt)| {
                            let style = if selected_flags[i] {
                                Style::default().fg(Color::DarkGray)
                            } else {
                                Style::default().fg(opt)
                            };
                            ListItem::new(opt.to_string()).style(style)
                        })
                        .collect();

                    let list = List::new(items)
                        .block(Block::default().borders(Borders::ALL).title(
                            match selected_flags.iter().filter(|&n| *n == true).count(){
                                0 => "Select a Colour for You",
                                _ => "Select a Colour for LLM",
                            }))
                        .highlight_style(Style::default().bg(Color::Blue).fg(Color::White));

                    frame.render_stateful_widget(list, list_area, &mut state);
                    let colour_info = Paragraph::new(
                            if let Some(i) = state.selected() {
                                if let Some(user_colour_picked) = user_colour_pick {
                                    Text::from(vec![
                                        Line::from(vec![
                                            Span::raw("Selecting for user: "),
                                            Span::styled(format!("{}", user_colour_picked.to_string()), Style::default().fg(user_colour_picked)),
                                        ]),
                                        Line::from(vec![
                                            Span::raw("Selecting for LLM: "),
                                            Span::styled(format!("{}", options[i].to_string()), Style::default().fg(options[i])),
                                        ]),
                                        Line::from("Press 'ESC' to return to main menu."),
                                    ])
                                }else{
                                    Text::from(vec![
                                        Line::from(vec![
                                            Span::raw("Selecting for user: "),
                                            Span::styled(format!("{}", options[i].to_string()), Style::default().fg(options[i])),
                                        ]),
                                        Line::from("Press 'ESC' to return to chat"),
                                    ])
                                }
                            // format!("Selecting:{}\nPress 'ESC' to return to chat",options[i].to_string())
                            
                        } else {
                            //format!("Selecting for you...\nPress 'ESC' to return to chat")
                            Text::from(vec![
                                Line::from("Selecting..."),
                                Line::from("Press ESC to return to chat"),
                            ])
                        })
                        .block(Block::default().borders(Borders::ALL).title("Info"));

                    frame.render_widget(colour_info, display_area);
                })?;
            }
            _ => {
                terminal.draw(|frame| {
                let vertical = Layout::vertical([
                    Constraint::Min(1),
                    Constraint::Length(1),
                    // Constraint::Min(1),
                    Constraint::Min(1),
                    Constraint::Length(3),
                ]);
                let [title_banner,help_area, response_area, input_area] = vertical.areas(frame.area());

                let horizontal = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(95), Constraint::Percentage(5)].as_ref())
                    .split(response_area);

                let chat_area = horizontal[0];
                let scrollbar_area = horizontal[1];

                let title = BigText::builder().centered()
                        .pixel_size(PixelSize::Quadrant)
                        .style(Style::new().light_blue())
                        .lines(vec![
                            "LLM Chat Interface".into(),
                        ])
                        .build();
                frame.render_widget(title, title_banner);

                let (msg, style) = match app.input_mode {
                    InputMode::Normal => (
                        vec![
                            "Press ".into(),
                            "e".bold(),
                            " to enter a prompt, ".into(),
                            "ESC".bold(),
                            " to return to main menu.".into(),
                        ],
                        Style::default().add_modifier(Modifier::RAPID_BLINK),
                    ),
                    InputMode::Editing => (
                        vec![
                            "Press ".into(),
                            "ESC".bold(),
                            " to stop editing, ".into(),
                            "Enter".bold(),
                            " to record the message, ".into(),
                        ],
                        Style::default(),
                    ),
                    InputMode::Processing => (
                        vec![
                            "Processing ".into(),
                        ],
                        Style::default(),
                    ),
                    InputMode::Fetching => (
                        vec![
                            "Press ".into(),
                            "ESC".bold(),
                            " to return to main menu, ".into(),
                            "enter chat ID to resume ".into(),
                        ],
                        Style::default(),
                    ),
                    InputMode::ColourSelection => (
                        vec![
                            "Processing colour selection".into(),
                        ],
                        Style::default(),
                    ),
                    InputMode::MainMenu => (
                        vec![
                            "Main menu".into(),
                        ],
                        Style::default(),
                    ),
                };
                let t = Text::from(Line::from(msg)).patch_style(style);
                let help_message = Paragraph::new(t);
                frame.render_widget(help_message, help_area);

                let input =  match app.input_mode {
                    InputMode::Processing => Paragraph::new("Wait for response...")
                        .style(Style::default())
                        .block(Block::bordered().title("Input")),
                    InputMode::Normal => Paragraph::new("Enter a prompt!")
                        .style(Style::default())
                        .block(Block::bordered().title("Input")),
                    InputMode::Editing => 
                        Paragraph::new(app.input.as_str())
                        .style(Style::default().fg(Color::Yellow))
                        .block(Block::bordered().title("Input")),
                    InputMode::Fetching => 
                        Paragraph::new(app.input.as_str())
                        .style(Style::default().fg(Color::Cyan))
                        .block(Block::bordered().title("Chat ID Input")),
                    InputMode::ColourSelection => 
                        Paragraph::new(app.input.as_str())
                        .style(Style::default().fg(Color::Yellow))
                        .block(Block::bordered().title("Colour Input")),
                    InputMode::MainMenu => 
                        Paragraph::new(app.input.as_str())
                        .style(Style::default().fg(Color::Yellow))
                        .block(Block::bordered().title("Main Menu")),
                };
                frame.render_widget(input, input_area);
                match app.input_mode {

                // Hide the cursor.
                    InputMode::Normal => {}
                    #[allow(clippy::cast_possible_truncation)]
                    InputMode::Editing => frame.set_cursor_position(Position::new(
                        // Draw the cursor at the current position in the input field.
                        input_area.x + app.character_index as u16 + 1,
                        // Move one line down, from the border to the input line
                        input_area.y + 1,
                    )),
                    InputMode::Processing => {},
                    InputMode::Fetching => frame.set_cursor_position(Position::new(
                        input_area.x + app.character_index as u16 + 1,
                        input_area.y + 1,
                    )),
                    InputMode::ColourSelection => {},
                    InputMode::MainMenu => {},
                }

                let mut spans=Vec::new();
                let mut messages = "".to_string();
                for item in app.messages.iter().zip_longest(app.llm_messages.iter()) {
                    match item {
                        Both(a, b) => {
                            // messages.push(ListItem::new(user.to_string()+&a));
                            let user_span = Span::styled(user.to_string()+&a, Style::default().fg(app.user_colour));
                            spans.push(Line::from(vec![user_span]));
                            let llm_span = Span::styled(llm.to_string()+&b, Style::default().fg(app.llm_colour));
                            spans.push(Line::from(vec![llm_span]));
                            messages+=&format!("{} {}\n",user, a);
                            messages+=&format!("{} {}\n",llm, b);
                            // messages.push(ListItem::new(llm.to_string()+&b));
                        }
                        Left(a) => {
                            let user_span = Span::styled(user.to_string()+&a, Style::default().fg(app.user_colour));
                            spans.push(Line::from(vec![user_span]));
                            messages+=&format!("{} {}\n",user, a);
                        }
                        Right(b) => {
                            let llm_span = Span::styled(llm.to_string()+&b, Style::default().fg(app.llm_colour));
                            spans.push(Line::from(vec![llm_span]));
                            messages+=&format!("{} {}\n",llm, b);
                        }
                    }
                }
                if !receiving.is_empty(){
                    let llm_span = Span::styled(llm.to_string()+&receiving, Style::default().fg(app.llm_colour));
                    spans.push(Line::from(vec![llm_span]));
                    messages+=&format!("{} {}\n",llm, receiving);

                }
                // Count total wrapped lines
                let total_lines = count_wrapped_lines(&messages, chat_area.width)+2;

                // Clamp scroll
                scroll_offset = scroll_offset.min(total_lines.saturating_sub(chat_area.height));

                let text = Text::from(spans);
                let paragraph = Paragraph::new(text.clone())
                    .block(Block::default().borders(Borders::ALL).title("Chat"))
                    .wrap(Wrap { trim: false })
                    .scroll((scroll_offset, 0));

                frame.render_widget(paragraph, chat_area);
                draw_scrollbar(frame, scrollbar_area, scroll_offset, total_lines, chat_area.height);

            })?;
            }
        }
        
        if event::poll(Duration::from_millis(1))? {
            if let Event::Key(key) = event::read()? {
                match app.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('e') => {
                            app.input_mode = InputMode::Editing;
                        }
                        // KeyCode::Char('r') => {
                        //     app.input_mode = InputMode::Fetching;
                        // }
                        KeyCode::Esc => {
                            app.selected_button=0;
                            app.input_mode = InputMode::MainMenu;
                        }
                        KeyCode::Up => scroll_offset = scroll_offset.saturating_sub(1),
                        KeyCode::Down => scroll_offset = scroll_offset.saturating_add(1),
                        KeyCode::PageUp => scroll_offset = scroll_offset.saturating_sub(5),
                        KeyCode::PageDown => scroll_offset = scroll_offset.saturating_add(5),
                        // KeyCode::Char('q') => {
                        //     break;
                        // }
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
                    InputMode::Processing => match key.code {
                        KeyCode::Up => scroll_offset = scroll_offset.saturating_sub(1),
                        KeyCode::Down => scroll_offset = scroll_offset.saturating_add(1),
                        KeyCode::PageUp => scroll_offset = scroll_offset.saturating_sub(5),
                        KeyCode::PageDown => scroll_offset = scroll_offset.saturating_add(5),
                        // KeyCode::Char('q') => {
                        //     break;
                        // }
                        _ => {}
                    },
                    InputMode::Fetching if key.kind == KeyEventKind::Press => match key.code {
                        KeyCode:: Enter => {app.fetch_chat(tx.clone())},
                        KeyCode:: Char(to_insert) => app.enter_char(to_insert),
                        KeyCode:: Backspace => app.delete_char(),
                        KeyCode:: Left => app.move_cursor_left(),
                        KeyCode:: Right => app.move_cursor_right(),
                        KeyCode:: Esc => {
                            app.selected_button=0;
                            app.input_mode = InputMode::MainMenu;
                        },
                        _ => {}
                    },
                    InputMode::Fetching => {},
                    InputMode::MainMenu => match key.code {
                        KeyCode::Up => app.previous_button(),
                        KeyCode::Down => app.next_button(),
                        KeyCode::Enter => {
                            if app.selected_button==0{
                                app.input_mode = InputMode::Normal;
                            } else if app.selected_button==1{
                                app.input_mode = InputMode::Fetching;
                            } else if app.selected_button==2{
                                selected_flags = vec![false; options.len()];
                                state = ListState::default();
                                state.select(Some(0));
                                user_colour_pick = None;
                                app.input_mode = InputMode::ColourSelection;
                            } else if app.selected_button==3{
                                break
                            }
                        },
                        _ => {}
                    },
                    InputMode::ColourSelection => match key.code {
                        // KeyCode::Char('q') => break,
                        KeyCode::Up => {
                            if let Some(mut i) = state.selected() {
                                i = previous_selectable(&selected_flags, i);
                                state.select(Some(i));
                            }
                        }
                        KeyCode::Down => {
                            if let Some(mut i) = state.selected() {
                                i = next_selectable(&selected_flags, i);
                                state.select(Some(i));
                            }
                        }
                        KeyCode::Esc => {
                            app.selected_button=0;
                            app.input_mode = InputMode::MainMenu;
                        }
                        KeyCode::Enter => {
                            if let Some(i) = state.selected() {
                                if !selected_flags[i] {
                                    // println!("Selected: {}", options[i]);
                                    selected_flags[i] = true;
                                    let count = selected_flags.iter().filter(|&n| *n == true).count();
                                    if count==2{
                                        app.llm_colour = options[i];
                                        if let Some(user_colour_picked) = user_colour_pick {
                                            app.user_colour=user_colour_picked;
                                        }
                                        app.input_mode = InputMode::Normal;
                                    }else if count==1{
                                        user_colour_pick=Some(options[i]);
                                    }
                                    let next = next_selectable(&selected_flags, i);
                                    state.select(Some(next));
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        
        while let Ok(c) = rx.try_recv() {
            if c=="Thread work complete!"{
                app.input_mode = InputMode::Normal;
                app.llm_messages.push(receiving);
                receiving = "".to_string();
            }
            else {
                receiving.push_str(&c);
            }
        }

        tokio::time::sleep(Duration::from_millis(5)).await;
        
    }
 
    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    // ratatui::restore();

    Ok(())
}

// Get next selectable index
fn next_selectable(selected_flags: &Vec<bool>, mut index: usize) -> usize {
    let len = selected_flags.len();
    for _ in 0..len {
        index = (index + 1) % len;
        if !selected_flags[index] {
            return index;
        }
    }
    index 
}

// Get previous selectable index
fn previous_selectable(selected_flags: &Vec<bool>, mut index: usize) -> usize {
    let len = selected_flags.len();
    for _ in 0..len {
        if index == 0 {
            index = len - 1;
        } else {
            index -= 1;
        }
        if !selected_flags[index] {
            return index;
        }
    }
    index
}

// Count number of wrapped lines for given text and width
fn count_wrapped_lines(text: &str, width: u16) -> u16 {
    let width = width as usize;
    let mut lines = 0;

    for raw_line in text.lines() {
        let mut remaining = raw_line.to_string(); // use String
        while !remaining.is_empty() {
            let take = std::cmp::min(width, remaining.chars().count());
            remaining = remaining.chars().skip(take).collect();
            lines += 1;
        }
    }

    lines
}

// Draw vertical scrollbar
fn draw_scrollbar(
    f: &mut ratatui::Frame,
    area: Rect,
    scroll: u16,
    total_lines: u16,
    viewport_height: u16,
) {
    if total_lines <= viewport_height {
        return;
    }

    let scrollbar_height = std::cmp::max(1, viewport_height * viewport_height / total_lines);
    let max_scroll = total_lines.saturating_sub(viewport_height);
    let scroll_pos = if max_scroll > 0 {
        scroll * (viewport_height - scrollbar_height) / max_scroll
    } else {
        0
    };

    for i in 0..scrollbar_height {
        let y = area.y + scroll_pos + i;
        if y < area.y + area.height {
            f.render_widget(
                Paragraph::new("█").style(Style::default().fg(Color::Gray)),
                Rect { x: area.x, y, width: 1, height: 1 },
            );
        }
    }
}


impl App {
    fn new() -> Self {
        Self { 
            llm_messages: Vec::new(),
            input: String::new(),
            input_mode: InputMode::MainMenu,
            messages: Vec::new(),
            character_index: 0, 
            user_colour:  Color::Red,
            llm_colour:  Color::Green,
            buttons: vec![
                Button::new("Chat Screen"),
                Button::new("Chat History"),
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

    fn next_button(&mut self) {
        self.selected_button = (self.selected_button + 1).min(self.buttons.len() - 1);
        self.update_button_states();
    }

    fn previous_button(&mut self) {
        if self.selected_button > 0 {
            self.selected_button -= 1;
        }
        self.update_button_states();
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
            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;

            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            let after_char_to_delete = self.input.chars().skip(current_index);

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
            let _ = run_llm(tx, input).await;
        });
    }

    fn fetch_chat(&mut self, tx: mpsc::Sender<String>) {
        self.input_mode = InputMode::Processing;
        self.messages.push("Fetching chat history for chat ID: ".to_string()+&self.input);
        let input = self.input.clone();
        self.input.clear();
        self.reset_cursor();
        tokio::spawn(async move {
            let _ = run_database(tx, input).await;
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
    let prompt = "<s>[INST] <<SYS>>You are a helpful assistant.<</SYS>> ".to_owned()+&input+" [/INST]";
    // tx.send(prompt.to_string()).await.ok();

    // send HTTP POST request with prompt to llm-server
    let addr = "127.0.0.1:4000";
    let prompt_post_url = format!("http://{addr}/generate");
    let client = Client::new();
    let response = client
        .post(&prompt_post_url)
        .json(&json!({ "prompt": prompt , "username": "Tester", "chat_id": 3}))
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


#[derive(Debug, Deserialize)]
enum FetchResponses {
    Success {messages: Vec<(i32, String, String)>},
    Error {message: String},
}

async fn run_database(tx: mpsc::Sender<String>, input:String) -> Result<()> {
    let chat_id: i32 = input.trim().parse().unwrap_or(0);

    let addr = "127.0.0.1:4000";
    let prompt_post_url = format!("http://{addr}/fetch");
    let client = Client::new();
    let response = client
        .post(&prompt_post_url)
        .json(&json!({ "username": "Tester", "chat_id": chat_id}))
        .send()
        .await?;
    let messages = response.json::<FetchResponses>().await?;

    match messages {
        FetchResponses::Success {messages} => {
            for (msg_id, msg, timestamp) in messages {
                let formatted_message = format!("[{}] {}: {}\n", msg_id, timestamp, msg);
                tx.send(formatted_message).await.ok();
            }
        },
        FetchResponses::Error {message} => {
            tx.send(format!("Error fetching chat history: {}", message)).await.ok();
        }
    }
    
    tx.send("Thread work complete!".to_string()).await.ok();
    Ok(())
}
