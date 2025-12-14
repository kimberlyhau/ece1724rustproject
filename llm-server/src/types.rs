use serde::Deserialize;

const DEFAULT_MAX_TOKENS: usize = 256;

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateRequest {
    // text prompt to feed into the model
    pub prompt: String,
    #[serde(default)]
    // current user message without system prompt/history
    pub user_message: String,
    #[serde(default)]
    // to load other models
    pub model: Option<String>,
    #[serde(default)]
    // params that control llm gen
    pub params: GenerationParams,

    #[serde(default)]
    pub username: String,

    #[serde(default)]
    pub chat_id: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct GenerationParams {
    // max tokens to be generated
    pub max_tokens: usize,
}

impl Default for GenerationParams {
    fn default() -> Self {
        Self {
            max_tokens: DEFAULT_MAX_TOKENS,
        }
    }
}
