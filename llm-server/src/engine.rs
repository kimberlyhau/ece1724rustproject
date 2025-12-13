use std::{
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex},
    thread,
};
use anyhow::{anyhow, Context, Result};
use candle_core::{DType, Device, Tensor};
use candle_examples::token_output_stream::TokenOutputStream;
use candle_nn::VarBuilder;
use candle_transformers::generation::{LogitsProcessor, Sampling};
use candle_transformers::models::llama as llama_model;
use candle_transformers::models::llama::{Llama, LlamaConfig};
use hf_hub::{api::sync::Api, Repo, RepoType};
use tokenizers::Tokenizer;
use tokio::sync::mpsc::UnboundedSender;

use crate::types::GenerationParams;

pub const EXAMPLE_MODEL: &str = "TinyLlama/TinyLlama-1.1B-Chat-v1.0";

// in-memory Candle model plus tokenizer so we can reuse one instance for different prompts
/* 
 * InferenceEngine contains 2 models:
 * Prefill Llama model runs synchronously and fully processes each client request, then the
 * DecodeRuntimeManager round robins through decoding tokens for concurrent client requests 
 * on a separate decode model.
*/
pub struct InferenceEngine {
    tokenizer: Tokenizer,
    prefill_llama: Mutex<Llama>,
    decode_runtime_manager: Arc<DecodeRuntimeManager>,
    device: Device,
    dtype: DType,
    config: llama_model::Config,
}

#[derive(Debug)]
// events emitted to the server during llm streaming
pub enum EventToServer {
    Token {token: String, index: usize},
    Done {total_tokens: usize},
    Error {message: String},
}

// Client request sent from the /generate handler into the inference engine worker thread to generate LLM responses
pub struct ClientRequest {
    pub prompt: String,
    pub params: GenerationParams,
    pub sender: UnboundedSender<EventToServer>,
}

impl InferenceEngine {
    pub fn new() -> Result<Self> {
        Self::from_model(EXAMPLE_MODEL)
    }

    pub fn from_model(model_id: &str) -> Result<Self> {
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
        let weight_paths =
            candle_examples::hub_load_safetensors(&repo, "model.safetensors.index.json")
                .or_else(|_| repo.get("model.safetensors").map(|path| vec![path]))
                .context("download model weights")?;

        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|err| anyhow!("load tokenizer: {err}"))?;

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

        let llama_config: LlamaConfig =
            serde_json::from_slice(&std::fs::read(config_path)?).context("parse config.json")?;
        let config = llama_config.into_config(false);
        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&weight_paths, dtype, &device)? };
        let prefill_llama = Llama::load(vb.clone(), &config)?;
        let decode_llama = Llama::load(vb, &config)?;

        // start decode manager in background thread to allocate time slices to each client request's decode session
        let decode_runtime_manager = Arc::new(DecodeRuntimeManager::new());
        Arc::clone(&decode_runtime_manager).start(decode_llama, device.clone());

        Ok(Self {
            tokenizer,
            prefill_llama: Mutex::new(prefill_llama),
            decode_runtime_manager,
            device,
            dtype,
            config,
        })
    }

    // run token generation and push tokens over the axum SENDER CHANNEL which is then streamed to the client
    pub fn generate(
        &self,
        prompt: &str,
        params: &GenerationParams,
        sender: &UnboundedSender<EventToServer>,
    ) -> Result<()> {
        let tokens = self
            .tokenizer
            .encode(prompt, true)
            .map_err(anyhow::Error::msg)?
            .get_ids()
            .to_vec();
        let stream = TokenOutputStream::new(self.tokenizer.clone());
        let cache = llama_model::Cache::new(true, self.dtype, &self.config, &self.device)?;
        let sampler = LogitsProcessor::from_sampling(
            42,
            Sampling::TopP {
                p: 0.9,
                temperature: 0.7,
            },
        );
        let eos_token = stream.get_token("</s>");

        let mut cur_client_request = ClientRequestSession::new(
            tokens,
            cache,
            sampler,
            stream,
            sender.clone(),
            eos_token,
            params.max_tokens,
        );

        // prompts are consumed and first processed serially using prefill model
        let Ok(mut prefill_lock) = self.prefill_llama.lock() else {
            let _ = sender.send(EventToServer::Error {
                message: "failed to lock prefill model".to_string(),
            });
            return Ok(());
        };

        if !cur_client_request.run_prefill(&mut prefill_lock, &self.device)? {
            return Ok(());
        }

        // prompt prefill done, release model lock
        drop(prefill_lock);

        // add the request session to the queue of the DecodeRuntimeManager which will allocate processing time on decode model
        self.decode_runtime_manager.add_request(cur_client_request);
        Ok(())
    }
}

// Run on background thread to manage decoding tokens for concurrent client requests
struct DecodeRuntimeManager {
    queue: Mutex<VecDeque<ClientRequestSession>>,
    cond: Condvar,
}

impl DecodeRuntimeManager {
    fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            cond: Condvar::new(),
        }
    }

    fn start(self: Arc<Self>, mut llama: Llama, device: Device) {
        // loop through queued client requests and decode one token at a time for each
        thread::spawn(move || loop {
            let mut session = {
                let mut queue = self.queue.lock().unwrap();
                while queue.is_empty() {
                    queue = self.cond.wait(queue).unwrap();
                }
                queue.pop_front().unwrap()
            };

            match session.run_decode_step(&mut llama, &device) {
                // add to back of queue if not done LLM decoding yet
                Ok(true) => {
                    let mut queue = self.queue.lock().unwrap();
                    queue.push_back(session);
                }
                Ok(false) => {
                    // client request done processing
                }
                Err(err) => {
                    let _ = session.sender.send(EventToServer::Error {
                        message: err.to_string(),
                    });
                }
            }
        });
    }

    fn add_request(&self, session: ClientRequestSession) {
        let mut queue = self.queue.lock().unwrap();
        queue.push_back(session);
        self.cond.notify_one();
    }
}

// Client request session holds all state and functions needed to generate LLM responses and stream back to client
struct ClientRequestSession {
    tokens: Vec<u32>,
    cache: llama_model::Cache,
    sampler: LogitsProcessor,
    stream: TokenOutputStream,
    sender: UnboundedSender<EventToServer>,
    eos_token: Option<u32>,
    ctx_index: usize,
    tokens_streamed: usize,
    tokens_generated: usize,
    max_tokens: usize,
    done_streaming: bool,
}

impl ClientRequestSession {
    fn new(
        tokens: Vec<u32>,
        cache: llama_model::Cache,
        sampler: LogitsProcessor,
        stream: TokenOutputStream,
        sender: UnboundedSender<EventToServer>,
        eos_token: Option<u32>,
        max_tokens: usize,
    ) -> Self {
        Self {
            tokens,
            cache,
            sampler,
            stream,
            sender,
            eos_token,
            ctx_index: 0,
            tokens_streamed: 0,
            tokens_generated: 0,
            max_tokens,
            done_streaming: false,
        }
    }

    fn run_prefill(&mut self, llama: &mut Llama, device: &Device) -> Result<bool> {
        self.generate_single_token(llama, device, true)
    }

    fn run_decode_step(&mut self, llama: &mut Llama, device: &Device) -> Result<bool> {
        self.generate_single_token(llama, device, false)
    }

    fn generate_single_token(&mut self, llama: &mut Llama, device: &Device, is_prefill: bool) -> Result<bool> {
        if self.done_streaming {
            return Ok(false);
        }

        if self.tokens_generated >= self.max_tokens {
            self.stream_back_remaining()?;
            return Ok(false);
        }

        let (context_size, offset) = if is_prefill {
            // use full prompt on prefill to build KV cache of the input
            (self.tokens.len(), 0)
        } else {
            // on decode, utilize KV cache so add single token context when generating tokens
            (1, self.ctx_index)
        };

        // ctx holds either the prompt for prefill
        // or the last token for incremental decoding
        // for cached decoding the model only needs the new token so it can look up past state from the KV cache
        let ctx_start = self.tokens.len().saturating_sub(context_size);
        let ctx = &self.tokens[ctx_start..];
        let input = Tensor::new(ctx, device)?.unsqueeze(0)?;
        let logits = llama.forward(&input, offset, &mut self.cache)?;
        let mut logits = logits.squeeze(0)?;
        
        // penalize tokens we just emitted so sampling avoids getting stuck in repeats (e.g. "hello hello hello")
        if !self.tokens.is_empty() {
            let start = self.tokens.len().saturating_sub(64);
            logits = candle_transformers::utils::apply_repeat_penalty(
                &logits,
                1.1,
                &self.tokens[start..],
            )?;
        }

        // sample the next token from the logits
        self.ctx_index += ctx.len();
        let next = self.sampler.sample(&logits)?;
        self.tokens.push(next);
        self.tokens_generated += 1;

        // push the generated/decoded token to the stream object and send to the server

        // stream object needed to convert token ids to strings
        // token ids dont map 1:1 to utf-8 strings, so we need the stream object to handle producing strings as tokens are generated (buffer tokens until a valid utf-8 string can be produced)
        // otherwise, we would need to wait until all tokens are generated to convert to string using tokenizer.decode
        if let Some(piece) = self.stream.next_token(next)? {
            // if client has dropped, stop gen
            if self
                .sender
                .send(EventToServer::Token {
                    token: piece,
                    index: self.tokens_streamed,
                })
                .is_err()
            {
                self.done_streaming = true;
                return Ok(false);
            }
            self.tokens_streamed += 1;
        }

        if let Some(eos) = self.eos_token {
            if next == eos {
                self.stream_back_remaining()?;
                return Ok(false);
            }
        }

        if self.tokens_generated >= self.max_tokens {
            self.stream_back_remaining()?;
            return Ok(false);
        }

        Ok(true)
    }

    // stream remaining bytes (if any) and send DONE event to client
    fn stream_back_remaining(&mut self) -> Result<()> {
        if self.done_streaming {
            return Ok(());
        }

        if let Some(rest) = self.stream.decode_rest()? {
            let _ = self.sender.send(EventToServer::Token {
                token: rest,
                index: self.tokens_streamed,
            });
            self.tokens_streamed += 1;
        }

        let _ = self.sender.send(EventToServer::Done {
            total_tokens: self.tokens_streamed,
        });
        self.done_streaming = true;
        Ok(())
    }
}
