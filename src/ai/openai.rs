//! OpenAI-compatible ChatCompletions provider.
//!
//! Used directly for OpenAI and reused for every OpenAI-compatible endpoint:
//! Groq, Together AI, DeepSeek, Mistral AI, and OpenRouter.  The base URL and
//! JSON-mode flag are supplied by [`crate::ai::build_provider`] based on the
//! configured provider type.
//!
//! # Configuration — OpenAI
//!
//! ```toml
//! [ai]
//! provider    = "openai"
//! model       = "gpt-4o"     # or "gpt-4o-mini", "gpt-4-turbo"
//! max_tokens  = 4096
//! temperature = 0.2
//! # openai_base_url = "https://api.openai.com/v1/chat/completions"  # override if needed
//! ```
//!
//! # Configuration — Groq (example)
//!
//! ```toml
//! [ai]
//! provider    = "groq"
//! model       = "llama-3.3-70b-versatile"
//! ```
//!
//! # Required environment variables
//!
//! | Provider | Variable |
//! |---|---|
//! | OpenAI | `OPENAI_API_KEY` |
//! | Groq | `GROQ_API_KEY` |
//! | Together AI | `TOGETHER_API_KEY` |
//! | DeepSeek | `DEEPSEEK_API_KEY` |
//! | Mistral | `MISTRAL_API_KEY` |
//! | OpenRouter | `OPENROUTER_API_KEY` |

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use crate::ai::response::parse_review_response;
use crate::ai::{system_prompt, AiProvider, ReviewComment, ReviewContext};
use crate::config::AiConfig;
use crate::error::{MerlinError, Result};

// ── Public provider struct ────────────────────────────────────────────────────

/// AI provider backed by the OpenAI Chat Completions API or any compatible endpoint.
///
/// Construct via [`crate::ai::build_provider`].
pub struct OpenAiProvider {
    api_key: String,
    config: AiConfig,
    client: reqwest::Client,
    /// The full chat completions endpoint URL.
    base_url: String,
    /// Whether to request `response_format: json_object` on review calls.
    /// Disable for providers / models that do not support strict JSON mode.
    json_mode: bool,
}

impl OpenAiProvider {
    /// Create a new provider.
    ///
    /// * `base_url`  — full chat completions URL (e.g. `https://api.groq.com/openai/v1/chat/completions`)
    /// * `json_mode` — set to `true` for providers that support `response_format: json_object`
    pub fn new(api_key: String, config: AiConfig, base_url: String, json_mode: bool) -> Self {
        Self {
            api_key,
            config,
            client: reqwest::Client::new(),
            base_url,
            json_mode,
        }
    }
}

// ── Wire types (private) ──────────────────────────────────────────────────────

#[derive(Serialize)]
struct OpenAiRequest {
    model: String,
    max_tokens: u32,
    temperature: f32,
    messages: Vec<ChatMessage>,
    /// Omitted when `None` — allows use with providers that do not support
    /// strict JSON mode (Together AI, Mistral, OpenRouter, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
}

#[derive(Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: &'static str,
}

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChatMessage,
}

// ── AiProvider implementation ─────────────────────────────────────────────────

#[async_trait]
impl AiProvider for OpenAiProvider {
    #[instrument(skip(self, system, user), fields(model = %self.config.model))]
    async fn generate(&self, system: &str, user: &str) -> Result<String> {
        let request = OpenAiRequest {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user.to_string(),
                },
            ],
            response_format: None,
        };

        let response = self
            .client
            .post(&self.base_url)
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(MerlinError::AiProvider(format!(
                "API error {status}: {body}"
            )));
        }

        let api_response: OpenAiResponse = response.json().await?;
        first_choice_content(api_response)
    }

    #[instrument(skip(self, ctx), fields(file = %ctx.file, model = %self.config.model))]
    async fn review(&self, ctx: &ReviewContext) -> Result<Vec<ReviewComment>> {
        let user_content = format!(
            "Review the following diff for file `{}`:\n\n```diff\n{}\n```",
            ctx.file, ctx.diff_hunk
        );

        // Request strict JSON output only when the provider supports it.
        // The response parser handles plain-text JSON responses as well.
        let response_format = if self.json_mode {
            Some(ResponseFormat {
                format_type: "json_object",
            })
        } else {
            None
        };

        let request = OpenAiRequest {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt(&self.config.review_focus()),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_content,
                },
            ],
            response_format,
        };

        debug!(file = %ctx.file, "Sending review request to {}", self.base_url);

        let response = self
            .client
            .post(&self.base_url)
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(MerlinError::AiProvider(format!(
                "API error {status}: {body}"
            )));
        }

        let api_response: OpenAiResponse = response.json().await?;
        let text = first_choice_content(api_response)?;
        parse_review_response(&text)
    }
}

/// Extract the content of the first choice from an OpenAI response.
fn first_choice_content(response: OpenAiResponse) -> Result<String> {
    response
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .ok_or_else(|| MerlinError::AiProvider("Empty choices in OpenAI response".to_string()))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::ai::response::parse_review_response;

    #[test]
    fn parse_direct_array() {
        let json = r#"[{"file":"lib.rs","line":5,"severity":"low","category":"style","title":"Fmt","body":"Trailing whitespace","suggestion":null}]"#;
        assert_eq!(parse_review_response(json).unwrap().len(), 1);
    }

    #[test]
    fn parse_wrapped_object() {
        let json = r#"{"comments":[{"file":"lib.rs","line":5,"severity":"low","category":"style","title":"Fmt","body":"whitespace","suggestion":null}]}"#;
        assert_eq!(parse_review_response(json).unwrap().len(), 1);
    }

    #[test]
    fn parse_empty_array() {
        assert!(parse_review_response("[]").unwrap().is_empty());
    }
}
