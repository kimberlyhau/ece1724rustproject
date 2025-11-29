// use anyhow::{anyhow, Context, Result};
// use candle_core::{DType, Device, Tensor};
// use candle_examples::token_output_stream::TokenOutputStream;
// use candle_nn::VarBuilder;
// use candle_transformers::generation::{LogitsProcessor, Sampling};
// use candle_transformers::models::llama as llama_model;
// use candle_transformers::models::llama::{Llama, LlamaConfig};
// use hf_hub::{api::sync::Api, Repo, RepoType};
// use std::io::{self, Write, Cursor};
// use tokenizers::Tokenizer;
// use std::error::Error;
// // use std::{io};
// use crossterm::{
//     event::{self, Event, KeyCode},
//     terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
//     execute,
// };
// use ratatui::{
//     prelude::*,
//     widgets::{Block, Borders, Paragraph},
// };
// use tokio::sync::mpsc;
// use tokio::time::{sleep, Duration};

// use std::{ time::{Instant}};


// struct App {
//     visible_text: String,
// }


// impl App {
//     fn new() -> Self {
//         Self { visible_text: String::new() }
//     }

//     fn push_char(&mut self, c: char) {
//         self.visible_text.push(c);
//     }
// }

// async fn async_text_stream(mut tx: mpsc::Sender<char>) {

//     match run_llm(){
//         Ok(text) => {
//             for c in text.chars() {
//                 tx.send(c).await.ok();
//                 sleep(Duration::from_millis(40)).await;
//             }
//         }
//         _ => {
//             let text = "error";
//             for c in text.chars() {
//                 tx.send(c).await.ok();
//                 sleep(Duration::from_millis(40)).await;
//             }
//         }
//     }

// }

// fn run_llm() -> Result<String, Box<dyn Error>>{
// // let prompt = "what is an llm?";
//     // let model_id = "HuggingFaceTB/SmolLM2-135M";
//     // let max_new_tokens = 16usize;
//     let prompt = "<s>[INST] <<SYS>>You are a helpful assistant.<</SYS>> what is a large language model? [/INST]";
//     let model_id = "TinyLlama/TinyLlama-1.1B-Chat-v1.0";
//     let max_new_tokens = 256;

//     // let args = Args::parse();

//     let api = Api::new()?;
//     let repo = api.repo(Repo::with_revision(
//         model_id.to_string(),
//         RepoType::Model,
//         "main".to_string(),
//     ));

//     let tokenizer_path = repo
//         .get("tokenizer.json")
//         .context("download tokenizer.json")?;
//     let config_path = repo.get("config.json").context("download config.json")?;
//     let weight_paths = candle_examples::hub_load_safetensors(&repo, "model.safetensors.index.json")
//         .or_else(|_| repo.get("model.safetensors").map(|path| vec![path]))
//         .context("download model weights")?;

//     let tokenizer =
//         Tokenizer::from_file(&tokenizer_path).map_err(|err| anyhow!("load tokenizer: {err}"))?;
//     let mut tokens = tokenizer
//         .encode(prompt, true)
//         .map_err(anyhow::Error::msg)?
//         .get_ids()
//         .to_vec();
//     let mut stream = TokenOutputStream::new(tokenizer);

//     #[cfg(feature = "metal")]
//     let device = match Device::new_metal(0) {
//         Ok(device) => device,
//         Err(err) => {
//             eprintln!("Metal unavailable ({err}), falling back to CPU.");
//             Device::Cpu
//         }
//     };
//     #[cfg(not(feature = "metal"))]
//     let device = Device::Cpu;
//     let dtype = DType::F32;

//     let config: LlamaConfig =
//         serde_json::from_slice(&std::fs::read(config_path)?).context("parse config.json")?;
//     let config = config.into_config(false);
//     let mut cache = llama_model::Cache::new(true, dtype, &config, &device)?;

//     let vb = unsafe { VarBuilder::from_mmaped_safetensors(&weight_paths, dtype, &device)? };
//     let llama = Llama::load(vb, &config)?;

//     let mut stdout = io::stdout();
//     let mut buffer = Cursor::new(Vec::new());
//     write!(buffer, "{prompt}")?;
//     // stdout.flush()?;

//     let mut sampler = LogitsProcessor::from_sampling(
//         42,
//         Sampling::TopP {
//             p: 0.9,
//             temperature: 0.7,
//         },
//     );
//     let eos_token = stream.get_token("</s>");
//     let mut ctx_index = 0usize;

//     for step in 0..max_new_tokens {
//         let (context_size, offset) = if cache.use_kv_cache && step > 0 {
//             (1, ctx_index)
//         } else {
//             (tokens.len(), 0)
//         };
//         let ctx = &tokens[tokens.len().saturating_sub(context_size)..];
//         let input = Tensor::new(ctx, &device)?.unsqueeze(0)?;
//         let logits = llama.forward(&input, offset, &mut cache)?;
//         let mut logits = logits.squeeze(0)?;

//         if !tokens.is_empty() {
//             let start = tokens.len().saturating_sub(64);
//             logits =
//                 candle_transformers::utils::apply_repeat_penalty(&logits, 1.1, &tokens[start..])?;
//         }

//         ctx_index += ctx.len();
//         let next = sampler.sample(&logits)?;
//         tokens.push(next);

//         if let Some(eos) = eos_token {
//             if next == eos {
//                 break;
//             }
//         }

//         if let Some(piece) = stream.next_token(next)? {
//             write!(buffer, "{piece}")?;
//             // stdout.flush()?;
//         }
//     }

//     if let Some(rest) = stream.decode_rest()? {
//         write!(buffer, "{rest}")?;
//     }
//     // writeln!(stdout)?;
//     let output_bytes = buffer.into_inner();

//     // Convert the bytes to a String
//     let output_string = String::from_utf8(output_bytes).expect("Output was not valid UTF-8");

//     println!("Captured output:\n{}", output_string);
//     Ok(output_string)
// }

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     // Setup terminal
//     enable_raw_mode()?;
//     let mut stdout = io::stdout();
//     execute!(stdout, EnterAlternateScreen)?;
//     let backend = ratatui::backend::CrosstermBackend::new(stdout);
//     let mut terminal = Terminal::new(backend)?;

//     // Channel for streaming text
//     let (tx, mut rx) = mpsc::channel::<char>(100);

//     // Spawn async text producing task
//     tokio::spawn(async move {
//         async_text_stream(tx).await;
//     });

//     let mut app = App::new();

//     loop {
//         // Draw UI
//         terminal.draw(|frame| {
//             let size = frame.size();
//             let paragraph = Paragraph::new(app.visible_text.clone())
//                 .block(Block::default().borders(Borders::ALL).title("Async Stream Output"));
//             frame.render_widget(paragraph, size);
//         })?;

//         // Handle keypress (quit on q)
//         if event::poll(Duration::from_millis(1))? {
//             if let Event::Key(key) = event::read()? {
//                 if key.code == KeyCode::Char('q') {
//                     break;
//                 }
//             }
//         }

//         // Non-blocking receive from async task
//         while let Ok(c) = rx.try_recv() {
//             app.push_char(c);
//         }

//         // Tiny sleep to avoid hot loop
//         tokio::time::sleep(Duration::from_millis(5)).await;
//     }

//     // Restore terminal
//     disable_raw_mode()?;
//     execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
//     terminal.show_cursor()?;

//     Ok(())
// }

// FROM HERE START
// fn run_llm() -> Result<String, Box<dyn Error>>{
// // let prompt = "what is an llm?";
//     // let model_id = "HuggingFaceTB/SmolLM2-135M";
//     // let max_new_tokens = 16usize;
//     let prompt = "<s>[INST] <<SYS>>You are a helpful assistant.<</SYS>> what is a large language model? [/INST]";
//     let model_id = "TinyLlama/TinyLlama-1.1B-Chat-v1.0";
//     let max_new_tokens = 256;

//     // let args = Args::parse();

//     let api = Api::new()?;
//     let repo = api.repo(Repo::with_revision(
//         model_id.to_string(),
//         RepoType::Model,
//         "main".to_string(),
//     ));

//     let tokenizer_path = repo
//         .get("tokenizer.json")
//         .context("download tokenizer.json")?;
//     let config_path = repo.get("config.json").context("download config.json")?;
//     let weight_paths = candle_examples::hub_load_safetensors(&repo, "model.safetensors.index.json")
//         .or_else(|_| repo.get("model.safetensors").map(|path| vec![path]))
//         .context("download model weights")?;

//     let tokenizer =
//         Tokenizer::from_file(&tokenizer_path).map_err(|err| anyhow!("load tokenizer: {err}"))?;
//     let mut tokens = tokenizer
//         .encode(prompt, true)
//         .map_err(anyhow::Error::msg)?
//         .get_ids()
//         .to_vec();
//     let mut stream = TokenOutputStream::new(tokenizer);

//     #[cfg(feature = "metal")]
//     let device = match Device::new_metal(0) {
//         Ok(device) => device,
//         Err(err) => {
//             eprintln!("Metal unavailable ({err}), falling back to CPU.");
//             Device::Cpu
//         }
//     };
//     #[cfg(not(feature = "metal"))]
//     let device = Device::Cpu;
//     let dtype = DType::F32;

//     let config: LlamaConfig =
//         serde_json::from_slice(&std::fs::read(config_path)?).context("parse config.json")?;
//     let config = config.into_config(false);
//     let mut cache = llama_model::Cache::new(true, dtype, &config, &device)?;

//     let vb = unsafe { VarBuilder::from_mmaped_safetensors(&weight_paths, dtype, &device)? };
//     let llama = Llama::load(vb, &config)?;

//     let mut stdout = io::stdout();
//     let mut buffer = Cursor::new(Vec::new());
//     write!(buffer, "{prompt}")?;
//     // stdout.flush()?;

//     let mut sampler = LogitsProcessor::from_sampling(
//         42,
//         Sampling::TopP {
//             p: 0.9,
//             temperature: 0.7,
//         },
//     );
//     let eos_token = stream.get_token("</s>");
//     let mut ctx_index = 0usize;

//     for step in 0..max_new_tokens {
//         let (context_size, offset) = if cache.use_kv_cache && step > 0 {
//             (1, ctx_index)
//         } else {
//             (tokens.len(), 0)
//         };
//         let ctx = &tokens[tokens.len().saturating_sub(context_size)..];
//         let input = Tensor::new(ctx, &device)?.unsqueeze(0)?;
//         let logits = llama.forward(&input, offset, &mut cache)?;
//         let mut logits = logits.squeeze(0)?;

//         if !tokens.is_empty() {
//             let start = tokens.len().saturating_sub(64);
//             logits =
//                 candle_transformers::utils::apply_repeat_penalty(&logits, 1.1, &tokens[start..])?;
//         }

//         ctx_index += ctx.len();
//         let next = sampler.sample(&logits)?;
//         tokens.push(next);

//         if let Some(eos) = eos_token {
//             if next == eos {
//                 break;
//             }
//         }

//         if let Some(piece) = stream.next_token(next)? {
//             write!(buffer, "{piece}")?;
//             // stdout.flush()?;
//         }
//     }

//     if let Some(rest) = stream.decode_rest()? {
//         write!(buffer, "{rest}")?;
//     }
//     // writeln!(stdout)?;
//     let output_bytes = buffer.into_inner();

//     // Convert the bytes to a String
//     let output_string = String::from_utf8(output_bytes).expect("Output was not valid UTF-8");

//     // println!("Captured output:\n{}", output_string);
//     Ok(output_string)
// }

// struct App {
//     full_text: String,
//     visible_text: String,
//     index: usize,
//     last_tick: Instant,
// }

// impl App {
//     fn new(text: &str) -> Self {
//         Self {
//             full_text: text.to_string(),
//             visible_text: String::new(),
//             index: 0,
//             last_tick: Instant::now(),
//         }
//     }

//     fn update(&mut self) {
//         // reveal one new character every 40ms
//         if self.index < self.full_text.len() && self.last_tick.elapsed() >= Duration::from_millis(40) {
//             let next_char = self.full_text.chars().nth(self.index).unwrap();
//             self.visible_text.push(next_char);
//             self.index += 1;
//             self.last_tick = Instant::now();
//         }
//     }
// }

// fn main() -> Result<(), Box<dyn std::error::Error>> {
//     // Setup terminal
//     enable_raw_mode()?;
//     let mut stdout = io::stdout();
//     execute!(stdout, EnterAlternateScreen)?;
//     let backend = CrosstermBackend::new(stdout);
//     let mut terminal = Terminal::new(backend)?;
//     // run_llm();
//     let mut print_str = "nothing";
//     let print_str = match run_llm(){
//         Ok(text) => text.clone(),
//         _ => "error".to_string(),
//     };
//     let mut app = App::new(&print_str);

//     loop {
//         terminal.draw(|frame| {
//             let size = frame.size();

//             let paragraph = Paragraph::new(app.visible_text.clone())
//                 .block(Block::default().borders(Borders::ALL).title("Output"));

//             frame.render_widget(paragraph, size);
//         })?;

//         // Handle input
//         if event::poll(Duration::from_millis(1))? {
//             if let Event::Key(key) = event::read()? {
//                 if key.code == KeyCode::Char('q') {
//                     break;
//                 }
//             }
//         }

//         app.update();
//     }

//     // Restore terminal
//     disable_raw_mode()?;
//     execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
//     terminal.show_cursor()?;

//     Ok(())
// }

// END

use anyhow::{anyhow, Context, Result};
use candle_core::{DType, Device, Tensor};
use candle_examples::token_output_stream::TokenOutputStream;
use candle_nn::VarBuilder;
use candle_transformers::generation::{LogitsProcessor, Sampling};
use candle_transformers::models::llama as llama_model;
use candle_transformers::models::llama::{Llama, LlamaConfig};
use hf_hub::{api::sync::Api, Repo, RepoType};
use std::io::{self, Write, Cursor};
use tokenizers::Tokenizer;
use std::error::Error;
// use std::{io};
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    execute,
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use ratatui::widgets::Wrap;

struct App {
    visible_text: String,
}

impl App {
    fn new() -> Self {
        Self { visible_text: String::new() }
    }

    fn push_char(&mut self, c: char) {
        self.visible_text.push(c);
    }
    fn push_str(&mut self, c: String) {
        self.visible_text.push_str(&c);
    }
}

/// async producer: simulates streaming text over time
async fn async_text_stream(mut tx: mpsc::Sender<char>) {
    let text = "Streaming text from async tasks…\nThis is running in the background.";
    for c in text.chars() {
        tx.send(c).await.ok();
        sleep(Duration::from_millis(40)).await;
    }
}

async fn run_llm(mut tx: mpsc::Sender<String>) -> Result<String, Box<dyn Error>>{
// let prompt = "what is an llm?";
    // let model_id = "HuggingFaceTB/SmolLM2-135M";
    // let max_new_tokens = 16usize;
    let prompt = "<s>[INST] <<SYS>>You are a helpful assistant.<</SYS>> what is a large language model? [/INST]";
    let model_id = "TinyLlama/TinyLlama-1.1B-Chat-v1.0";
    let max_new_tokens = 256;

    // let args = Args::parse();

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
        .encode(prompt, true)
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

    let mut stdout = io::stdout();
    let mut buffer = Cursor::new(Vec::new());
    write!(buffer, "{prompt}")?;
    // stdout.flush()?;
    let output_bytes = buffer.clone().into_inner();
    let mut output_string = String::from_utf8(output_bytes).expect("Output was not valid UTF-8");
    tx.send(output_string).await.ok();

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
            write!(buffer, "{piece}")?;
            let output_bytes = buffer.clone().into_inner();
    output_string = String::from_utf8(output_bytes).expect("Output was not valid UTF-8");
    tx.send(piece).await.ok();
            // stdout.flush()?;
        }
    }

    if let Some(rest) = stream.decode_rest()? {
        write!(buffer, "{rest}")?;
        let output_bytes = buffer.clone().into_inner();
    output_string = String::from_utf8(output_bytes).expect("Output was not valid UTF-8");
    tx.send(rest).await.ok();
    }
    // writeln!(stdout)?;
    let output_bytes = buffer.into_inner();

    // Convert the bytes to a String
    let output_string = String::from_utf8(output_bytes).expect("Output was not valid UTF-8");
    // tx.send(output_string.trim().to_string()).await.ok();
    // println!("Captured output:\n{}", output_string);
    Ok(output_string)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Channel for streaming text
    let (tx, mut rx) = mpsc::channel::<String>(100);

    // Spawn async text producing task
    tokio::spawn(async move {
        // async_text_stream(tx).await;
        run_llm(tx).await;
    });

    let mut app = App::new();

    loop {
        // Draw UI
        terminal.draw(|frame| {
            let size = frame.size();
            let paragraph = Paragraph::new(app.visible_text.clone())
            .wrap(Wrap { trim: true })
                .block(Block::default().borders(Borders::ALL).title("Async Stream Output"));
            frame.render_widget(paragraph, size);
        })?;

        // Handle keypress (quit on q)
        if event::poll(Duration::from_millis(1))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }

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

    Ok(())
}
