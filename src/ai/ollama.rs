//! Local Ollama provider.
//!
//! Runs any model available in a local Ollama instance via the
//! `/api/chat` endpoint.  No API key required.
//!
//! # Configuration
//!
//! ```toml
//! [ai]
//! provider         = "ollama"
//! model            = "llama3.1"                    # any pulled model
//! ollama_base_url  = "http://localhost:11434"       # optional
//! ```
//!
//! # Prerequisites
//!
//! ```bash
//! ollama serve                 # must be running
//! ollama pull llama3.1         # pull at least one model
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use crate::ai::response::parse_review_response;
use crate::ai::{system_prompt, AiProvider, ReviewComment, ReviewContext};
use crate::config::AiConfig;
use crate::error::{MerlinError, Result};

// ── Public provider struct ────────────────────────────────────────────────────

/// AI provider backed by a local Ollama instance.
///
/// Construct via [`crate::ai::build_provider`].
pub struct OllamaProvider {
    config: AiConfig,
    base_url: String,
    client: reqwest::Client,
}

impl OllamaProvider {
    /// Create a new provider.
    ///
    /// `base_url` is typically `http://localhost:11434`.
    pub fn new(config: AiConfig, base_url: String) -> Self {
        Self {
            config,
            base_url,
            client: reqwest::Client::new(),
        }
    }
}

// ── Wire types (private) ──────────────────────────────────────────────────────

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: u32,
}

#[derive(Deserialize)]
struct OllamaResponse {
    message: OllamaMessage,
}

// ── AiProvider implementation ─────────────────────────────────────────────────

#[async_trait]
impl AiProvider for OllamaProvider {
    #[instrument(skip(self, system, user), fields(model = %self.config.model))]
    async fn generate(&self, system: &str, user: &str) -> Result<String> {
        let url = format!("{}/api/chat", self.base_url);

        let request = OllamaRequest {
            model: self.config.model.clone(),
            messages: vec![
                OllamaMessage {
                    role: "system".to_string(),
                    content: system.to_string(),
                },
                OllamaMessage {
                    role: "user".to_string(),
                    content: user.to_string(),
                },
            ],
            stream: false,
            options: OllamaOptions {
                temperature: self.config.temperature,
                num_predict: self.config.max_tokens,
            },
        };

        debug!(base_url = %self.base_url, model = %self.config.model, "Sending generate request to Ollama");

        let resp = self.client.post(&url).json(&request).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(MerlinError::AiProvider(format!("Ollama {status}: {body}")));
        }

        let result: OllamaResponse = resp.json().await?;
        Ok(result.message.content)
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
