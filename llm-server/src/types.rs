use serde::Deserialize;

const DEFAULT_MAX_TOKENS: usize = 256;

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateRequest {
    // text prompt to feed into the model
    pub prompt: String,
    #[serde(default)]
    // to load other models
    pub model: Option<String>,
    #[serde(default)]
    // params that control llm gen
    pub params: GenerationParams,

    #[serde(default)]
    pub username: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenerationParams {
    #[serde(default = "default_max_tokens")]
    // max tokens to be generated
    pub max_tokens: usize,
}

impl Default for GenerationParams {
    fn default() -> Self {
        Self {
            max_tokens: default_max_tokens(),
        }
    }
}

const fn default_max_tokens() -> usize {
    DEFAULT_MAX_TOKENS
}
