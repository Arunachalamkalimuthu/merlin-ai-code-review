use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use crate::ai::{system_prompt, AiProvider, ReviewComment, ReviewContext};
use crate::config::AiConfig;
use crate::error::{MerlinError, Result};

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

pub struct AnthropicProvider {
    api_key: String,
    config: AiConfig,
    client: reqwest::Client,
}

impl AnthropicProvider {
    pub fn new(api_key: String, config: AiConfig) -> Self {
        Self {
            api_key,
            config,
            client: reqwest::Client::new(),
        }
    }
}

// ── Anthropic request/response types ─────────────────────────────────────────

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

// ─────────────────────────────────────────────────────────────────────────────

#[async_trait]
impl AiProvider for AnthropicProvider {
    #[instrument(skip(self, system, user))]
    async fn generate(&self, system: &str, user: &str) -> Result<String> {
        let request = AnthropicRequest {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            system: system.to_string(),
            messages: vec![Message { role: "user".to_string(), content: user.to_string() }],
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
            return Err(MerlinError::AiProvider(format!("Anthropic API error {status}: {body}")));
        }

        let api_response: AnthropicResponse = response.json().await?;
        api_response
            .content
            .into_iter()
            .find(|b| b.block_type == "text")
            .and_then(|b| b.text)
            .ok_or_else(|| MerlinError::AiProvider("No text content in response".to_string()))
    }

    #[instrument(skip(self, ctx), fields(file = %ctx.file))]
    async fn review(&self, ctx: &ReviewContext) -> Result<Vec<ReviewComment>> {
        let user_content = format!(
            "Review the following diff for file `{}`:\n\n```diff\n{}\n```",
            ctx.file, ctx.diff_hunk
        );

        let request = AnthropicRequest {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            system: system_prompt(&["bugs".to_string(), "security".to_string(), "style".to_string(), "performance".to_string()]),
            messages: vec![Message {
                role: "user".to_string(),
                content: user_content,
            }],
        };

        debug!("Sending request to Anthropic API for file: {}", ctx.file);

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
                "Anthropic API error {status}: {body}"
            )));
        }

        let api_response: AnthropicResponse = response.json().await?;

        let text = api_response
            .content
            .into_iter()
            .find(|b| b.block_type == "text")
            .and_then(|b| b.text)
            .ok_or_else(|| MerlinError::AiProvider("No text content in response".to_string()))?;

        parse_ai_response(&text)
    }
}

fn parse_ai_response(text: &str) -> Result<Vec<ReviewComment>> {
    // Strip optional markdown code fences the model might add anyway
    let cleaned = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    serde_json::from_str(cleaned).map_err(|e| {
        MerlinError::AiProvider(format!(
            "Failed to parse AI response as ReviewComment array: {e}\nRaw: {cleaned}"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_response() {
        let json = r#"[{"file":"src/main.rs","line":10,"severity":"high","category":"bug","title":"Null deref","body":"Potential null dereference","suggestion":null}]"#;
        let comments = parse_ai_response(json).unwrap();
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].file, "src/main.rs");
        assert_eq!(comments[0].line, 10);
    }

    #[test]
    fn test_parse_empty_response() {
        let comments = parse_ai_response("[]").unwrap();
        assert!(comments.is_empty());
    }

    #[test]
    fn test_parse_with_markdown_fence() {
        let json = "```json\n[]\n```";
        let comments = parse_ai_response(json).unwrap();
        assert!(comments.is_empty());
    }
}
