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
pub struct InferenceEngine {
    tokenizer: Tokenizer,
    prefill_llama: Llama,
    decode_llama: Llama,
    device: Device,
    dtype: DType,
    config: llama_model::Config,
}

#[derive(Debug)]
// events emitted to the server during llm streaming
pub enum EventToServer {
    Token { token: String, index: usize },
    Done { total_tokens: usize },
    Error { message: String },
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

        Ok(Self {
            tokenizer,
            prefill_llama,
            decode_llama,
            device,
            dtype,
            config,
        })
    }

    // run token generation and push tokens over the axum SENDER CHANNEL which is then streamed to the client
    pub fn generate(
        &mut self,
        prompt: &str,
        params: &GenerationParams,
        sender: &UnboundedSender<EventToServer>,
    ) -> Result<()> {
        let mut tokens = self
            .tokenizer
            .encode(prompt, true)
            .map_err(anyhow::Error::msg)?
            .get_ids()
            .to_vec();
        let mut stream = TokenOutputStream::new(self.tokenizer.clone());
        let mut cache = llama_model::Cache::new(true, self.dtype, &self.config, &self.device)?;
        let mut sampler = LogitsProcessor::from_sampling(
            42,
            Sampling::TopP {
                p: 0.9,
                temperature: 0.7,
            },
        );
        let eos_token = stream.get_token("</s>");
        let mut ctx_index = 0usize;
        let mut generated = 0usize;

        for step in 0..params.max_tokens {
            let use_prefill = !(cache.use_kv_cache && step > 0);
            let (context_size, offset) = if use_prefill {
                // use full prompt on first pass to build KV cache
                (tokens.len(), 0)
            } else {
                // utilize KV cache so add single token context when generating tokens
                (1, ctx_index)
            };
            // ctx holds either the prompt for prefill
            // or the last token for incremental decoding
            // for cached decoding the model only needs the new token so it can look up past state from the KV cache
            let ctx = &tokens[tokens.len().saturating_sub(context_size)..];
            let input = Tensor::new(ctx, &self.device)?.unsqueeze(0)?;
            let llama = if use_prefill {
                &mut self.prefill_llama
            } else {
                &mut self.decode_llama
            };
            let logits = llama.forward(&input, offset, &mut cache)?;
            let mut logits = logits.squeeze(0)?;

            // penalize tokens we just emitted so sampling avoids getting stuck in repeats (e.g. "hello hello hello")
            if !tokens.is_empty() {
                let start = tokens.len().saturating_sub(64);
                logits = candle_transformers::utils::apply_repeat_penalty(
                    &logits,
                    1.1,
                    &tokens[start..],
                )?;
            }

            // sample the next token from the logits
            ctx_index += ctx.len();
            let next = sampler.sample(&logits)?;
            tokens.push(next);

            if let Some(eos) = eos_token {
                if next == eos {
                    break;
                }
            }

            // push the generated/decoded token to the stream object and send to the server

            // stream object needed to convert token ids to strings
            // token ids dont map 1:1 to utf-8 strings, so we need the stream object to handle producing strings as tokens are generated (buffer tokens until a valid utf-8 string can be produced)
            // otherwise, we would need to wait until all tokens are generated to convert to string using tokenizer.decode
            if let Some(piece) = stream.next_token(next)? {
                // if client has dropped, stop gen
                if sender
                    .send(EventToServer::Token {
                        token: piece,
                        index: generated,
                    })
                    .is_err()
                {
                    return Ok(());
                }
                generated += 1;
            }
        }

        if let Some(rest) = stream.decode_rest()? {
            // if client has dropped, stop gen
            if sender
                .send(EventToServer::Token {
                    token: rest,
                    index: generated,
                })
                .is_err()
            {
                return Ok(());
            }
            generated += 1;
        }

        let _ = sender.send(EventToServer::Done {
            total_tokens: generated,
        });
        Ok(())
    }
}
