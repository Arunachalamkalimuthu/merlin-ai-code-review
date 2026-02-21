//! Google Gemini provider (Generative Language API).
//!
//! Calls `generativelanguage.googleapis.com` using the `generateContent`
//! endpoint.  The API key is passed via the `x-goog-api-key` request header
//! (not in the URL) to prevent accidental key exposure in logs.
//!
//! # Configuration
//!
//! ```toml
//! [ai]
//! provider = "gemini"
//! model    = "gemini-1.5-pro"   # or "gemini-1.5-flash"
//! ```
//!
//! # Required environment variable
//!
//! `GEMINI_API_KEY` — obtain from <https://aistudio.google.com/>.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use crate::ai::response::parse_review_response;
use crate::ai::{system_prompt, AiProvider, ReviewComment, ReviewContext};
use crate::config::AiConfig;
use crate::error::{MerlinError, Result};

const GEMINI_API_BASE: &str =
    "https://generativelanguage.googleapis.com/v1beta/models";

// ── Public provider struct ────────────────────────────────────────────────────

/// AI provider backed by the Google Gemini Generative Language API.
///
/// Construct via [`crate::ai::build_provider`].
pub struct GeminiProvider {
    api_key: String,
    config: AiConfig,
    client: reqwest::Client,
}

impl GeminiProvider {
    /// Create a new provider.
    pub fn new(api_key: String, config: AiConfig) -> Self {
        Self { api_key, config, client: reqwest::Client::new() }
    }

    /// Build the `generateContent` endpoint URL for the configured model.
    ///
    /// The API key is **not** included in the URL — it is passed as the
    /// `x-goog-api-key` request header to avoid leaking it in access logs.
    fn endpoint_url(&self) -> String {
        format!("{}/{}:generateContent", GEMINI_API_BASE, self.config.model)
    }
}

// ── Wire types (private) ──────────────────────────────────────────────────────

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

// ── AiProvider implementation ─────────────────────────────────────────────────

#[async_trait]
impl AiProvider for GeminiProvider {
    #[instrument(skip(self, system, user), fields(model = %self.config.model))]
    async fn generate(&self, system: &str, user: &str) -> Result<String> {
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

        debug!(model = %self.config.model, "Sending generate request to Gemini");

        let resp = self
            .client
            .post(self.endpoint_url())
            .header("x-goog-api-key", &self.api_key)  // key in header, NOT in URL
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(MerlinError::AiProvider(format!("Gemini API {status}: {body}")));
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

    #[instrument(skip(self, ctx), fields(file = %ctx.file, model = %self.config.model))]
    async fn review(&self, ctx: &ReviewContext) -> Result<Vec<ReviewComment>> {
        let system = system_prompt(&self.config.review_focus());
        let user = format!(
            "Review the following diff for file `{}`:\n\n```diff\n{}\n```",
            ctx.file, ctx.diff_hunk
        );
        let raw = self.generate(&system, &user).await?;
        parse_review_response(&raw)
    }
}
