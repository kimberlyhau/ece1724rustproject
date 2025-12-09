
use anyhow::{anyhow, Context, Result};
use candle_core::{DType, Device, Tensor};
use candle_examples::token_output_stream::TokenOutputStream;
use candle_nn::VarBuilder;
use candle_transformers::generation::{LogitsProcessor, Sampling};
use candle_transformers::models::llama as llama_model;
use candle_transformers::models::llama::{Llama, LlamaConfig};
use hf_hub::{api::sync::Api, Repo, RepoType};
use std::io::{self, Write};
use tokenizers::Tokenizer;
use itertools::Itertools;
use itertools::EitherOrBoth::{Both, Left, Right};
// use std::error::Error;
// use std::{io};
use crossterm::{
    event::{self, Event, KeyCode,KeyEventKind},
    terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    execute,
};
use ratatui::Terminal;
use ratatui::{
    layout::{Constraint, Layout, Position, Margin, Rect, Direction},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, List, ListItem, Borders, Paragraph,Scrollbar, ScrollbarOrientation, ScrollbarState,
        StatefulWidget,},
};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use ratatui::widgets::Wrap;

struct App {
    visible_text: Vec<String>,
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
    let user:&str = "You: ";
    let llm:&str = "LLM: ";
    let mut receiving = String::new();

    let mut scroll_offset: u16 = 0;
    loop {
        terminal.draw(|frame| {
            let vertical = Layout::vertical([
                Constraint::Length(1),
                // Constraint::Min(1),
                Constraint::Min(1),
                Constraint::Length(3),
            ]);
            let [help_area, response_area, input_area] = vertical.areas(frame.area());

            let horizontal = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(95), Constraint::Percentage(5)].as_ref())
                .split(response_area);

            let chat_area = horizontal[0];
            let scrollbar_area = horizontal[1];

            let (msg, style) = match app.input_mode {
                InputMode::Normal => (
                    vec![
                        "Press ".into(),
                        "q".bold(),
                        " to exit, ".into(),
                        "e".bold(),
                        " to enter a prompt.".bold(),
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
            let t = Text::from(Line::from(msg)).patch_style(style);
            let help_message = Paragraph::new(t);
            frame.render_widget(help_message, help_area);

            let input =  match app.input_mode {
                InputMode::Processing => (Paragraph::new("Wait for response...")
                    .style(Style::default())
                    .block(Block::bordered().title("Input"))),
                InputMode::Normal => (Paragraph::new("Enter a prompt!")
                    .style(Style::default())
                    .block(Block::bordered().title("Input"))),
                InputMode::Editing => (
                    Paragraph::new(app.input.as_str())
                    .style(Style::default().fg(Color::Yellow))
                    .block(Block::bordered().title("Input"))),
            };
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

            // let mut messages=Vec::new();
            // for item in app.messages.iter().zip_longest(app.visible_text.iter()) {
            //     match item {
            //         Both(a, b) => {
            //             messages.push(ListItem::new(user.to_string()+&a));
            //             messages.push(ListItem::new(llm.to_string()+&b));
            //         }
            //         Left(a) => messages.push(ListItem::new(user.to_string()+&a)),
            //         Right(b) => messages.push(ListItem::new(llm.to_string()+&b)),
            //     }
            // }
            // if !receiving.is_empty(){
            //     messages.push(ListItem::new(llm.to_string()+&receiving));
            // }
            // let message = List::new(messages).block(Block::bordered().title("Messages"));
            // frame.render_widget(message, response_area);

            let mut messages = "".to_string();
            for item in app.messages.iter().zip_longest(app.visible_text.iter()) {
                match item {
                    Both(a, b) => {
                        // messages.push_str(user.to_string()+&a+"\n".to_string());
                        // messages.push_str(llm.to_string()+&b+"\n".to_string());
                        messages+=&format!("{} {}\n",user, a);
                        messages+=&format!("{} {}\n",llm, b);
                    }
                    Left(a) => messages+=&format!("{} {}\n",user, a),//messages.push_str(user.to_string()+&a+"\n".to_string()),
                    Right(b) => messages+=&format!("{} {}\n",llm, b),//messages.push_str(llm.to_string()+&b+"\n".to_string()),
                }
            }
            if !receiving.is_empty(){
                messages+=&format!("{} {}",llm, receiving);
            }

            // Count total wrapped lines
            let total_lines = count_wrapped_lines(&messages, chat_area.width)+2;

            // Clamp scroll
            scroll_offset = scroll_offset.min(total_lines.saturating_sub(chat_area.height));

            let text = Text::from(messages);
            let paragraph = Paragraph::new(text.clone())
                .block(Block::default().borders(Borders::ALL).title("Chat"))
                .wrap(Wrap { trim: false })
                .scroll((scroll_offset, 0));

            frame.render_widget(paragraph, chat_area);
            draw_scrollbar(frame, scrollbar_area, scroll_offset, total_lines, chat_area.height);

        })?;


        if event::poll(Duration::from_millis(1))? {
            if let Event::Key(key) = event::read()? {
                // if key.code == KeyCode::Char('q') {
                //     break;
                // }
                match app.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('e') => {
                            app.input_mode = InputMode::Editing;
                        }
                        KeyCode::Up => scroll_offset = scroll_offset.saturating_sub(1),
                        KeyCode::Down => scroll_offset = scroll_offset.saturating_add(1),
                        KeyCode::PageUp => scroll_offset = scroll_offset.saturating_sub(5),
                        KeyCode::PageDown => scroll_offset = scroll_offset.saturating_add(5),
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
                app.visible_text.push(receiving);
                receiving = "".to_string();
            }
            else {
                // app.push_str(c);
                receiving.push_str(&c);
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

/// Count number of wrapped lines for given text and width
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

/// Draw vertical scrollbar
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
        Self { visible_text: Vec::new(),
        input: String::new(),
        input_mode: InputMode::Normal,
        messages: Vec::new(),
        character_index: 0, }
    }

    // fn push_char(&mut self, c: char) {
    //     self.visible_text.push(c);
    // }
    // fn push_str(&mut self, c: String) {
    //     self.visible_text.push_str(&c);
    // }
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
        // self.visible_text.push_str(&self.input.clone());
        // eprintln!("Debug information: {:?}", input);
        tokio::spawn(async move {
            let _ = run_llm(tx, input).await;
            // async_text_stream(tx, input);
        });
    }
}

/// async producer: simulates streaming text over time
async fn async_text_stream(tx: mpsc::Sender<String>, input:String) {
    let text = "Streaming text from async tasks…\nThis is running in the background.";
    for c in text.chars() {
        tx.send(c.to_string()).await.ok();
        sleep(Duration::from_millis(40)).await;
    }
}

async fn run_llm(tx: mpsc::Sender<String>, input:String) -> Result<()>{
    let text = "Streaming text from async tasks…\nThis is running in the background.";
    for c in text.chars() {
        // eprintln!("{c}");
        tx.send(c.to_string()).await.ok();
        sleep(Duration::from_millis(40)).await;
    }
    tx.send("Thread work complete!".to_string()).await.ok();
    Ok(())
// let prompt = "what is an llm?";
    // let model_id = "HuggingFaceTB/SmolLM2-135M";
    // let max_new_tokens = 16usize;
    // eprintln!("Debug information: {:?}", input);
    // let prompt = "<s>[INST] <<SYS>>You are a helpful assistant.<</SYS>> ".to_owned()+&input+"[/INST]";
    // // let prompt = format!("{} {} {}", "<s>[INST] <<SYS>>You are a helpful assistant.<</SYS>>", input, "[/INST]");

    // let model_id = "TinyLlama/TinyLlama-1.1B-Chat-v1.0";
    // let max_new_tokens = 256;

    // let api = Api::new()?;
    // let repo = api.repo(Repo::with_revision(
    //     model_id.to_string(),
    //     RepoType::Model,
    //     "main".to_string(),
    // ));

    // let tokenizer_path = repo
    //     .get("tokenizer.json")
    //     .context("download tokenizer.json")?;
    // let config_path = repo.get("config.json").context("download config.json")?;
    // let weight_paths = candle_examples::hub_load_safetensors(&repo, "model.safetensors.index.json")
    //     .or_else(|_| repo.get("model.safetensors").map(|path| vec![path]))
    //     .context("download model weights")?;

    // let tokenizer =
    //     Tokenizer::from_file(&tokenizer_path).map_err(|err| anyhow!("load tokenizer: {err}"))?;
    // let mut tokens = tokenizer
    //     .encode(prompt.clone(), true)
    //     .map_err(anyhow::Error::msg)?
    //     .get_ids()
    //     .to_vec();
    // let mut stream = TokenOutputStream::new(tokenizer);

    // #[cfg(feature = "metal")]
    // let device = match Device::new_metal(0) {
    //     Ok(device) => device,
    //     Err(err) => {
    //         eprintln!("Metal unavailable ({err}), falling back to CPU.");
    //         Device::Cpu
    //     }
    // };
    // #[cfg(not(feature = "metal"))]
    // let device = Device::Cpu;
    // let dtype = DType::F32;

    // let config: LlamaConfig =
    //     serde_json::from_slice(&std::fs::read(config_path)?).context("parse config.json")?;
    // let config = config.into_config(false);
    // let mut cache = llama_model::Cache::new(true, dtype, &config, &device)?;

    // let vb = unsafe { VarBuilder::from_mmaped_safetensors(&weight_paths, dtype, &device)? };
    // let llama = Llama::load(vb, &config)?;

    // tx.send(prompt.to_string()).await.ok();

    // let mut sampler = LogitsProcessor::from_sampling(
    //     42,
    //     Sampling::TopP {
    //         p: 0.9,
    //         temperature: 0.7,
    //     },
    // );
    // let eos_token = stream.get_token("</s>");
    // let mut ctx_index = 0usize;

    // for step in 0..max_new_tokens {
    //     let (context_size, offset) = if cache.use_kv_cache && step > 0 {
    //         (1, ctx_index)
    //     } else {
    //         (tokens.len(), 0)
    //     };
    //     let ctx = &tokens[tokens.len().saturating_sub(context_size)..];
    //     let input = Tensor::new(ctx, &device)?.unsqueeze(0)?;
    //     let logits = llama.forward(&input, offset, &mut cache)?;
    //     let mut logits = logits.squeeze(0)?;

    //     if !tokens.is_empty() {
    //         let start = tokens.len().saturating_sub(64);
    //         logits =
    //             candle_transformers::utils::apply_repeat_penalty(&logits, 1.1, &tokens[start..])?;
    //     }

    //     ctx_index += ctx.len();
    //     let next = sampler.sample(&logits)?;
    //     tokens.push(next);

    //     if let Some(eos) = eos_token {
    //         if next == eos {
    //             break;
    //         }
    //     }

    //     if let Some(piece) = stream.next_token(next)? {
    //          tx.send(piece).await.ok();
    //         // stdout.flush()?;
    //     }
    // }

    // if let Some(rest) = stream.decode_rest()? {
    //     tx.send(rest).await.ok();
    // }
    // tx.send("Thread work complete!".to_string()).await.ok();
    // Ok(())
}
