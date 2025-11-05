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

fn main() -> Result<()> {
    // let prompt = "what is an llm?";
    // let model_id = "HuggingFaceTB/SmolLM2-135M";
    // let max_new_tokens = 16usize;
    
    // the prompt needs to defined as per the models expected format
    let prompt = "<s>[INST] <<SYS>>You are a helpful assistant.<</SYS>> what is a large language model? [/INST]";
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
    write!(stdout, "prompt:  {prompt}")?;
    stdout.flush()?;

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
        let (context_size, offset) = if !(cache.use_kv_cache && step > 0) {
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
        let input = Tensor::new(ctx, &device)?.unsqueeze(0)?;
        let logits = llama.forward(&input, offset, &mut cache)?;
        let mut logits = logits.squeeze(0)?;

        // penalize tokens we just emitted so sampling avoids getting stuck in repeats (e.g. "hello hello hello")
        if !tokens.is_empty() {
            let start = tokens.len().saturating_sub(64);
            logits =
                candle_transformers::utils::apply_repeat_penalty(&logits, 1.1, &tokens[start..])?;
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

        // push the generated/decoded token to the stream object and flush to stdout
        
        // stream object needed to convert token ids to strings
        // token ids dont map 1:1 to utf-8 strings, so we need the stream object to handle producing strings as tokens are generated (buffer tokens until a valid utf-8 string can be produced)
        // otherwise, we would need to wait until all tokens are generated to convert to string using tokenizer.decode
        if let Some(piece) = stream.next_token(next)? {
            write!(stdout, "{piece}")?;
            stdout.flush()?; // so that the writes are not buffered, and sent to terminal immediately
        }
    }

    if let Some(rest) = stream.decode_rest()? {
        write!(stdout, "{rest}")?;
    }
    writeln!(stdout)?;
    Ok(())
}
