//! Ollama local model provider.
//!
//! Runs any model available in a local Ollama instance.
//! Default endpoint: http://localhost:11434
//!
//! Config:
//! ```toml
//! [ai]
//! provider = "ollama"
//! model = "llama3.1"       # any model pulled with `ollama pull`
//! ollama_base_url = "http://localhost:11434"  # optional override
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use super::{system_prompt, AiProvider, ReviewComment, ReviewContext};
use crate::config::AiConfig;
use crate::error::{MerlinError, Result};

pub struct OllamaProvider {
    config: AiConfig,
    base_url: String,
    client: reqwest::Client,
}

impl OllamaProvider {
    pub fn new(config: AiConfig, base_url: String) -> Self {
        Self { config, base_url, client: reqwest::Client::new() }
    }
}

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

#[async_trait]
impl AiProvider for OllamaProvider {
    #[instrument(skip(self, system, user))]
    async fn generate(&self, system: &str, user: &str) -> Result<String> {
        let url = format!("{}/api/chat", self.base_url);

        let request = OllamaRequest {
            model: self.config.model.clone(),
            messages: vec![
                OllamaMessage { role: "system".to_string(), content: system.to_string() },
                OllamaMessage { role: "user".to_string(), content: user.to_string() },
            ],
            stream: false,
            options: OllamaOptions {
                temperature: self.config.temperature,
                num_predict: self.config.max_tokens,
            },
        };

        debug!("Sending request to Ollama at {}", self.base_url);

        let resp = self.client.post(&url).json(&request).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(MerlinError::AiProvider(format!("Ollama error {status}: {body}")));
        }

        let result: OllamaResponse = resp.json().await?;
        Ok(result.message.content)
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
            MerlinError::AiProvider(format!("Failed to parse Ollama response: {e}\nRaw: {cleaned}"))
        })
    }
}
