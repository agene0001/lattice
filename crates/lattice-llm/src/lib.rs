//! `lattice-llm` — the shared BYOK LLM client for Lattice's AI features.
//!
//! Rust has no first-party Anthropic SDK, so each provider's REST endpoint is
//! called directly with `reqwest`. Both `lattice-diagnosis` (misconception
//! classification) and `lattice-content` (AI problem generation) go through the
//! single [`complete`] entry point; key/model selection is the caller's (BYOK).

use serde::{Deserialize, Serialize};

/// Which LLM provider to call. Keys are the user's own (BYOK).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Anthropic,
    OpenAi,
    Gemini,
}

impl Provider {
    /// A sensible cheap/fast default model. User-overridable.
    pub fn default_model(&self) -> &'static str {
        match self {
            Provider::Anthropic => "claude-haiku-4-5",
            Provider::OpenAi => "gpt-4o-mini",
            Provider::Gemini => "gemini-2.0-flash",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Provider::Anthropic => "Anthropic",
            Provider::OpenAi => "OpenAI",
            Provider::Gemini => "Google Gemini",
        }
    }
}

/// Provider + the user's key + the chosen model. Built by the app from the
/// keychain (key) and settings (provider/model).
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub provider: Provider,
    pub api_key: String,
    pub model: String,
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("network error contacting the provider: {0}")]
    Http(#[from] reqwest::Error),
    #[error("provider returned an error: {0}")]
    Api(String),
    #[error("the model returned no text")]
    Empty,
}

/// One-shot completion: a system + user prompt in, the assistant's text out.
pub async fn complete(
    config: &ProviderConfig,
    system: &str,
    user: &str,
) -> Result<String, LlmError> {
    match config.provider {
        Provider::Anthropic => call_anthropic(config, system, user).await,
        Provider::OpenAi => call_openai(config, system, user).await,
        Provider::Gemini => call_gemini(config, system, user).await,
    }
}

/// Slice out the outermost `{ … }` so leading/trailing prose (some models add it)
/// doesn't break JSON parsing.
pub fn extract_json_object(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    (end > start).then(|| &s[start..=end])
}

async fn call_anthropic(
    config: &ProviderConfig,
    system: &str,
    user: &str,
) -> Result<String, LlmError> {
    let body = serde_json::json!({
        "model": config.model,
        "max_tokens": 1024,
        "system": system,
        "messages": [{ "role": "user", "content": user }],
    });
    let resp = reqwest::Client::new()
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &config.api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await?;
    let status = resp.status();
    let value: serde_json::Value = resp.json().await?;
    if !status.is_success() {
        return Err(LlmError::Api(api_error_message(&value)));
    }
    value["content"][0]["text"]
        .as_str()
        .map(str::to_string)
        .ok_or(LlmError::Empty)
}

async fn call_openai(
    config: &ProviderConfig,
    system: &str,
    user: &str,
) -> Result<String, LlmError> {
    let body = serde_json::json!({
        "model": config.model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user },
        ],
    });
    let resp = reqwest::Client::new()
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(&config.api_key)
        .json(&body)
        .send()
        .await?;
    let status = resp.status();
    let value: serde_json::Value = resp.json().await?;
    if !status.is_success() {
        return Err(LlmError::Api(api_error_message(&value)));
    }
    value["choices"][0]["message"]["content"]
        .as_str()
        .map(str::to_string)
        .ok_or(LlmError::Empty)
}

async fn call_gemini(
    config: &ProviderConfig,
    system: &str,
    user: &str,
) -> Result<String, LlmError> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
        config.model
    );
    let body = serde_json::json!({
        "systemInstruction": { "parts": [{ "text": system }] },
        "contents": [{ "role": "user", "parts": [{ "text": user }] }],
    });
    let resp = reqwest::Client::new()
        .post(&url)
        .header("x-goog-api-key", &config.api_key)
        .json(&body)
        .send()
        .await?;
    let status = resp.status();
    let value: serde_json::Value = resp.json().await?;
    if !status.is_success() {
        return Err(LlmError::Api(api_error_message(&value)));
    }
    value["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .map(str::to_string)
        .ok_or(LlmError::Empty)
}

fn api_error_message(value: &serde_json::Value) -> String {
    value["error"]["message"]
        .as_str()
        .or_else(|| value["error"].as_str())
        .or_else(|| value["message"].as_str())
        .unwrap_or("unknown error")
        .to_string()
}
