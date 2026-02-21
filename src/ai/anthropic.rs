//! Anthropic Claude provider (Messages API).
//!
//! Sends requests to `api.anthropic.com/v1/messages` using the official
//! Anthropic Messages format.
//!
//! # Configuration
//!
//! ```toml
//! [ai]
//! provider   = "anthropic"
//! model      = "claude-sonnet-4-6"   # or any claude-* model
//! max_tokens = 4096
//! temperature = 0.2
//! ```
//!
//! # Required environment variable
//!
//! `ANTHROPIC_API_KEY` — obtain from <https://console.anthropic.com/>.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use crate::ai::response::parse_review_response;
use crate::ai::{system_prompt, AiProvider, ReviewComment, ReviewContext};
use crate::config::AiConfig;
use crate::error::{MerlinError, Result};

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
/// Anthropic API version header value.  Pinned to keep behaviour stable.
const ANTHROPIC_VERSION: &str = "2023-06-01";

// ── Public provider struct ────────────────────────────────────────────────────

/// AI provider backed by the Anthropic Claude API.
///
/// Construct via [`crate::ai::build_provider`] — do not instantiate directly
/// in production code.
pub struct AnthropicProvider {
    api_key: String,
    config: AiConfig,
    client: reqwest::Client,
}

impl AnthropicProvider {
    /// Create a new provider with the given API key and configuration.
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
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    temperature: f32,
    system: String,
    messages: Vec<Message>,
}

#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}

// ── AiProvider implementation ─────────────────────────────────────────────────

#[async_trait]
impl AiProvider for AnthropicProvider {
    /// Send a freeform generation request to the Anthropic Messages API.
    ///
    /// # Errors
    ///
    /// Returns [`MerlinError::AiProvider`] on non-2xx HTTP status or if the
    /// response contains no `text` content block.
    #[instrument(skip(self, system, user), fields(model = %self.config.model))]
    async fn generate(&self, system: &str, user: &str) -> Result<String> {
        let request = AnthropicRequest {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            system: system.to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: user.to_string(),
            }],
        };

        let response = self
            .client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(MerlinError::AiProvider(format!(
                "Anthropic API {status}: {body}"
            )));
        }

        let api_response: AnthropicResponse = response.json().await?;
        extract_text(api_response)
    }

    /// Review a diff chunk and return structured [`ReviewComment`]s.
    ///
    /// # Errors
    ///
    /// Returns [`MerlinError::AiProvider`] on API failure or JSON parse failure.
    #[instrument(skip(self, ctx), fields(file = %ctx.file, model = %self.config.model))]
    async fn review(&self, ctx: &ReviewContext) -> Result<Vec<ReviewComment>> {
        let user_content = format!(
            "Review the following diff for file `{}`:\n\n```diff\n{}\n```",
            ctx.file, ctx.diff_hunk
        );

        let request = AnthropicRequest {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            system: system_prompt(&self.config.review_focus()),
            messages: vec![Message {
                role: "user".to_string(),
                content: user_content,
            }],
        };

        debug!(file = %ctx.file, "Sending review request to Anthropic");

        let response = self
            .client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(MerlinError::AiProvider(format!(
                "Anthropic API {status}: {body}"
            )));
        }

        let api_response: AnthropicResponse = response.json().await?;
        let text = extract_text(api_response)?;
        parse_review_response(&text)
    }
}

/// Extract the first `text` content block from an Anthropic response.
fn extract_text(response: AnthropicResponse) -> Result<String> {
    response
        .content
        .into_iter()
        .find(|b| b.block_type == "text")
        .and_then(|b| b.text)
        .ok_or_else(|| {
            MerlinError::AiProvider("No text content block in Anthropic response".to_string())
        })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::ai::response::parse_review_response;

    #[test]
    fn parse_valid_response() {
        let json = r#"[{"file":"src/main.rs","line":10,"severity":"high","category":"bug","title":"Null deref","body":"Potential null dereference","suggestion":null}]"#;
        let comments = parse_review_response(json).unwrap();
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].file, "src/main.rs");
        assert_eq!(comments[0].line, 10);
    }

    #[test]
    fn parse_empty_response() {
        assert!(parse_review_response("[]").unwrap().is_empty());
    }

    #[test]
    fn parse_with_markdown_fence() {
        assert!(parse_review_response("```json\n[]\n```")
            .unwrap()
            .is_empty());
    }
}
