//! OpenAI ChatCompletions provider.
//!
//! Sends requests to `api.openai.com/v1/chat/completions`.
//! Uses `response_format: json_object` for the review endpoint to encourage
//! structured output, with fallback parsing for wrapped objects.
//!
//! # Configuration
//!
//! ```toml
//! [ai]
//! provider    = "openai"
//! model       = "gpt-4o"     # or "gpt-4o-mini", "gpt-4-turbo"
//! max_tokens  = 4096
//! temperature = 0.2
//! ```
//!
//! # Required environment variable
//!
//! `OPENAI_API_KEY` — obtain from <https://platform.openai.com/api-keys>.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use crate::ai::response::parse_review_response;
use crate::ai::{system_prompt, AiProvider, ReviewComment, ReviewContext};
use crate::config::AiConfig;
use crate::error::{MerlinError, Result};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

// ── Public provider struct ────────────────────────────────────────────────────

/// AI provider backed by the OpenAI Chat Completions API.
///
/// Construct via [`crate::ai::build_provider`].
pub struct OpenAiProvider {
    api_key: String,
    config: AiConfig,
    client: reqwest::Client,
}

impl OpenAiProvider {
    /// Create a new provider.
    pub fn new(api_key: String, config: AiConfig) -> Self {
        Self {
            api_key,
            config,
            client: reqwest::Client::new(),
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
    response_format: ResponseFormat,
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
            response_format: ResponseFormat {
                format_type: "text",
            },
        };

        let response = self
            .client
            .post(OPENAI_API_URL)
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(MerlinError::AiProvider(format!(
                "OpenAI API {status}: {body}"
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
            // json_object mode: model must output valid JSON (may still wrap array)
            response_format: ResponseFormat {
                format_type: "json_object",
            },
        };

        debug!(file = %ctx.file, "Sending review request to OpenAI");

        let response = self
            .client
            .post(OPENAI_API_URL)
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(MerlinError::AiProvider(format!(
                "OpenAI API {status}: {body}"
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
