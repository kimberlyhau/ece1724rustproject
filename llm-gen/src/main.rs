
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
// use std::error::Error;
// use std::{io};
use crossterm::{
    event::{self, Event, KeyCode,KeyEventKind},
    terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    execute,
};
use ratatui::Terminal;
use ratatui::{
    layout::{Constraint, Layout, Position},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, List, ListItem, Borders, Paragraph},
};
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

    // Spawn async text producing task
    // tokio::spawn(async move {
    //     // async_text_stream(tx).await;
    //     run_llm(tx).await;
    // });

    let mut app = App::new();

    loop {
        // Draw UI
        // terminal.draw(|frame| {
        //     let size = frame.size();
        //     let paragraph = Paragraph::new(app.visible_text.clone())
        //     .wrap(Wrap { trim: true })
        //         .block(Block::default().borders(Borders::ALL).title("Async Stream Output"));
        //     frame.render_widget(paragraph, size);
        // })?;
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
            };
            let text = Text::from(Line::from(msg)).patch_style(style);
            let help_message = Paragraph::new(text);
            frame.render_widget(help_message, help_area);
            let input = Paragraph::new(app.input.as_str())
                .style(match app.input_mode {
                    InputMode::Normal => Style::default(),
                    InputMode::Editing => Style::default().fg(Color::Yellow),
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
                }
            }
        }
        // if let Event::Key(key) = event::read()? {
        //     match app.input_mode {
        //         InputMode::Normal => match key.code {
        //             KeyCode::Char('e') => {
        //                 app.input_mode = InputMode::Editing;
        //             }
        //             KeyCode::Char('q') => {
        //                 break;
        //             }
        //             _ => {}
        //         },
        //         InputMode::Editing if key.kind == KeyEventKind::Press => match key.code {
        //             KeyCode::Enter => app.submit_message(),
        //             KeyCode::Char(to_insert) => app.enter_char(to_insert),
        //             KeyCode::Backspace => app.delete_char(),
        //             KeyCode::Left => app.move_cursor_left(),
        //             KeyCode::Right => app.move_cursor_right(),
        //             KeyCode::Esc => app.input_mode = InputMode::Normal,
        //             _ => {}
        //         },
        //         InputMode::Editing => {}
        //     }
        // }

        // Non-blocking receive from async task
        while let Ok(c) = rx.try_recv() {
            app.push_str(c);
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

/// async producer: simulates streaming text over time
async fn async_text_stream(tx: mpsc::Sender<String>, input:String) {
    let text = "Streaming text from async tasks…\nThis is running in the background.";
    for c in text.chars() {
        tx.send(c.to_string()).await.ok();
        sleep(Duration::from_millis(40)).await;
    }
}

async fn run_llm(tx: mpsc::Sender<String>, input:String) -> Result<()>{
// let prompt = "what is an llm?";
    // let model_id = "HuggingFaceTB/SmolLM2-135M";
    // let max_new_tokens = 16usize;
    // eprintln!("Debug information: {:?}", input);
    let prompt = "<s>[INST] <<SYS>>You are a helpful assistant.<</SYS>> ".to_owned()+&input+"[/INST]";
    // let prompt = format!("{} {} {}", "<s>[INST] <<SYS>>You are a helpful assistant.<</SYS>>", input, "[/INST]");

    let model_id = "TinyLlama/TinyLlama-1.1B-Chat-v1.0";
    let max_new_tokens = 256;

    let api = Api::new()?;
    let repo = api.repo(Repo::with_revision(
        model_id.to_string(),
        RepoType::Model,
        "main".to_string(),
    ));

    let tokenizer_path = repo
        .get("tokenizer.json")
        .context("download tokenizer.json")?;
    let config_path = repo.get("config.json").context("download config.json")?;
    let weight_paths = candle_examples::hub_load_safetensors(&repo, "model.safetensors.index.json")
        .or_else(|_| repo.get("model.safetensors").map(|path| vec![path]))
        .context("download model weights")?;

    let tokenizer =
        Tokenizer::from_file(&tokenizer_path).map_err(|err| anyhow!("load tokenizer: {err}"))?;
    let mut tokens = tokenizer
        .encode(prompt.clone(), true)
        .map_err(anyhow::Error::msg)?
        .get_ids()
        .to_vec();
    let mut stream = TokenOutputStream::new(tokenizer);

    #[cfg(feature = "metal")]
    let device = match Device::new_metal(0) {
        Ok(device) => device,
        Err(err) => {
            eprintln!("Metal unavailable ({err}), falling back to CPU.");
            Device::Cpu
        }
    };
    #[cfg(not(feature = "metal"))]
    let device = Device::Cpu;
    let dtype = DType::F32;

    let config: LlamaConfig =
        serde_json::from_slice(&std::fs::read(config_path)?).context("parse config.json")?;
    let config = config.into_config(false);
    let mut cache = llama_model::Cache::new(true, dtype, &config, &device)?;

    let vb = unsafe { VarBuilder::from_mmaped_safetensors(&weight_paths, dtype, &device)? };
    let llama = Llama::load(vb, &config)?;

    // let mut stdout = io::stdout();
    // let mut buffer = Cursor::new(Vec::new());
    // write!(buffer, "{prompt}")?;
    // stdout.flush()?;
    // let output_bytes = buffer.clone().into_inner();
    // let mut output_string = String::from_utf8(prompt.into()).expect("Output was not valid UTF-8");
    tx.send(prompt.to_string()).await.ok();

    let mut sampler = LogitsProcessor::from_sampling(
        42,
        Sampling::TopP {
            p: 0.9,
            temperature: 0.7,
        },
    );
    let eos_token = stream.get_token("</s>");
    let mut ctx_index = 0usize;

    for step in 0..max_new_tokens {
        let (context_size, offset) = if cache.use_kv_cache && step > 0 {
            (1, ctx_index)
        } else {
            (tokens.len(), 0)
        };
        let ctx = &tokens[tokens.len().saturating_sub(context_size)..];
        let input = Tensor::new(ctx, &device)?.unsqueeze(0)?;
        let logits = llama.forward(&input, offset, &mut cache)?;
        let mut logits = logits.squeeze(0)?;

        if !tokens.is_empty() {
            let start = tokens.len().saturating_sub(64);
            logits =
                candle_transformers::utils::apply_repeat_penalty(&logits, 1.1, &tokens[start..])?;
        }

        ctx_index += ctx.len();
        let next = sampler.sample(&logits)?;
        tokens.push(next);

        if let Some(eos) = eos_token {
            if next == eos {
                break;
            }
        }

        if let Some(piece) = stream.next_token(next)? {
            // write!(buffer, "{piece}")?;
            // let output_bytes = buffer.clone().into_inner();
            // output_string = String::from_utf8(output_bytes).expect("Output was not valid UTF-8");
            tx.send(piece).await.ok();
            // stdout.flush()?;
        }
    }

    if let Some(rest) = stream.decode_rest()? {
        // write!(buffer, "{rest}")?;
        // let output_bytes = buffer.clone().into_inner();
        // output_string = String::from_utf8(output_bytes).expect("Output was not valid UTF-8");
        tx.send(rest).await.ok();
    }
    // writeln!(stdout)?;
    // let output_bytes = buffer.into_inner();

    // Convert the bytes to a String
    // let output_string = String::from_utf8(output_bytes).expect("Output was not valid UTF-8");
    // tx.send(output_string.trim().to_string()).await.ok();
    // println!("Captured output:\n{}", output_string);
    Ok(())
}
