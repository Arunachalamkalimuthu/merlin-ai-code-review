use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use crate::ai::{system_prompt, AiProvider, ReviewComment, ReviewContext};
use crate::config::AiConfig;
use crate::error::{MerlinError, Result};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

pub struct OpenAiProvider {
    api_key: String,
    config: AiConfig,
    client: reqwest::Client,
}

impl OpenAiProvider {
    pub fn new(api_key: String, config: AiConfig) -> Self {
        Self {
            api_key,
            config,
            client: reqwest::Client::new(),
        }
    }
}

// ── OpenAI request/response types ────────────────────────────────────────────

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
    format_type: String,
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

// ─────────────────────────────────────────────────────────────────────────────

#[async_trait]
impl AiProvider for OpenAiProvider {
    #[instrument(skip(self, system, user))]
    async fn generate(&self, system: &str, user: &str) -> Result<String> {
        let request = OpenAiRequest {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            messages: vec![
                ChatMessage { role: "system".to_string(), content: system.to_string() },
                ChatMessage { role: "user".to_string(), content: user.to_string() },
            ],
            response_format: ResponseFormat { format_type: "text".to_string() },
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
            return Err(MerlinError::AiProvider(format!("OpenAI API error {status}: {body}")));
        }

        let api_response: OpenAiResponse = response.json().await?;
        api_response
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| MerlinError::AiProvider("Empty choices in OpenAI response".to_string()))
    }

    #[instrument(skip(self, ctx), fields(file = %ctx.file))]
    async fn review(&self, ctx: &ReviewContext) -> Result<Vec<ReviewComment>> {
        let user_content = format!(
            "Review the following diff for file `{}`:\n\n```diff\n{}\n```",
            ctx.file, ctx.diff_hunk
        );

        let focus = vec![
            "bugs".to_string(),
            "security".to_string(),
            "style".to_string(),
            "performance".to_string(),
        ];

        let request = OpenAiRequest {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt(&focus),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_content,
                },
            ],
            response_format: ResponseFormat {
                format_type: "json_object".to_string(),
            },
        };

        debug!("Sending request to OpenAI API for file: {}", ctx.file);

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
                "OpenAI API error {status}: {body}"
            )));
        }

        let api_response: OpenAiResponse = response.json().await?;

        let text = api_response
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| MerlinError::AiProvider("Empty choices in OpenAI response".to_string()))?;

        parse_openai_response(&text)
    }
}

/// OpenAI json_object mode wraps the array, so we handle both `[...]` and `{"comments":[...]}`.
fn parse_openai_response(text: &str) -> Result<Vec<ReviewComment>> {
    let cleaned = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // Direct array
    if cleaned.starts_with('[') {
        return serde_json::from_str(cleaned).map_err(|e| {
            MerlinError::AiProvider(format!("Failed to parse OpenAI response: {e}\nRaw: {cleaned}"))
        });
    }

    // Wrapped object — try common keys
    let value: serde_json::Value = serde_json::from_str(cleaned).map_err(|e| {
        MerlinError::AiProvider(format!("Failed to parse OpenAI JSON: {e}\nRaw: {cleaned}"))
    })?;

    for key in &["comments", "reviews", "issues", "results"] {
        if let Some(arr) = value.get(key) {
            return serde_json::from_value(arr.clone()).map_err(|e| {
                MerlinError::AiProvider(format!("Failed to deserialize '{key}' array: {e}"))
            });
        }
    }

    // Fall back: try entire object as single comment
    Err(MerlinError::AiProvider(format!(
        "Could not extract comment array from OpenAI response: {cleaned}"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_direct_array() {
        let json = r#"[{"file":"lib.rs","line":5,"severity":"low","category":"style","title":"Formatting","body":"Trailing whitespace","suggestion":null}]"#;
        let comments = parse_openai_response(json).unwrap();
        assert_eq!(comments.len(), 1);
    }

    #[test]
    fn test_parse_wrapped_object() {
        let json = r#"{"comments":[{"file":"lib.rs","line":5,"severity":"low","category":"style","title":"Fmt","body":"whitespace","suggestion":null}]}"#;
        let comments = parse_openai_response(json).unwrap();
        assert_eq!(comments.len(), 1);
    }

    #[test]
    fn test_parse_empty_array() {
        let comments = parse_openai_response("[]").unwrap();
        assert!(comments.is_empty());
    }
}
