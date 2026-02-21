//! Google Gemini provider (Generative Language API).
//!
//! Requires: `GEMINI_API_KEY` env var.
//!
//! Config:
//! ```toml
//! [ai]
//! provider = "gemini"
//! model = "gemini-1.5-pro"   # or "gemini-1.5-flash"
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use super::{system_prompt, AiProvider, ReviewComment, ReviewContext};
use crate::config::AiConfig;
use crate::error::{MerlinError, Result};

const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";

pub struct GeminiProvider {
    api_key: String,
    config: AiConfig,
    client: reqwest::Client,
}

impl GeminiProvider {
    pub fn new(api_key: String, config: AiConfig) -> Self {
        Self { api_key, config, client: reqwest::Client::new() }
    }
}

// ── Gemini request/response types ─────────────────────────────────────────────

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(rename = "systemInstruction")]
    system_instruction: Option<GeminiContent>,
    #[serde(rename = "generationConfig")]
    generation_config: GeminiGenerationConfig,
}

#[derive(Serialize, Deserialize)]
struct GeminiContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    parts: Vec<GeminiPart>,
}

#[derive(Serialize, Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Serialize)]
struct GeminiGenerationConfig {
    temperature: f32,
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: u32,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
}

// ─────────────────────────────────────────────────────────────────────────────

#[async_trait]
impl AiProvider for GeminiProvider {
    #[instrument(skip(self, system, user))]
    async fn generate(&self, system: &str, user: &str) -> Result<String> {
        let url = format!(
            "{}/{}:generateContent?key={}",
            GEMINI_API_BASE, self.config.model, self.api_key
        );

        let request = GeminiRequest {
            system_instruction: Some(GeminiContent {
                role: None,
                parts: vec![GeminiPart { text: system.to_string() }],
            }),
            contents: vec![GeminiContent {
                role: Some("user".to_string()),
                parts: vec![GeminiPart { text: user.to_string() }],
            }],
            generation_config: GeminiGenerationConfig {
                temperature: self.config.temperature,
                max_output_tokens: self.config.max_tokens,
            },
        };

        debug!("Sending request to Gemini API model: {}", self.config.model);

        let resp = self.client.post(&url).json(&request).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(MerlinError::AiProvider(format!("Gemini API error {status}: {body}")));
        }

        let result: GeminiResponse = resp.json().await?;
        result
            .candidates
            .into_iter()
            .next()
            .and_then(|c| c.content.parts.into_iter().next())
            .map(|p| p.text)
            .ok_or_else(|| MerlinError::AiProvider("Empty Gemini response".to_string()))
    }

    #[instrument(skip(self, ctx), fields(file = %ctx.file))]
    async fn review(&self, ctx: &ReviewContext) -> Result<Vec<ReviewComment>> {
        let system = system_prompt(&[
            "bugs".to_string(), "security".to_string(),
            "style".to_string(), "performance".to_string(),
        ]);
        let user = format!(
            "Review the following diff for file `{}`:\n\n```diff\n{}\n```",
            ctx.file, ctx.diff_hunk
        );
        let raw = self.generate(&system, &user).await?;
        let cleaned = raw.trim()
            .trim_start_matches("```json").trim_start_matches("```")
            .trim_end_matches("```").trim();
        serde_json::from_str(cleaned).map_err(|e| {
            MerlinError::AiProvider(format!("Failed to parse Gemini response: {e}\nRaw: {cleaned}"))
        })
    }
}
