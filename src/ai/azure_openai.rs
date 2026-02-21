//! Azure OpenAI Service provider.
//!
//! Uses the same Chat Completions format as OpenAI but with:
//! - A custom endpoint URL per deployment
//! - `api-key` header instead of `Authorization: Bearer`
//!
//! Requires: `AZURE_OPENAI_API_KEY` env var.
//!
//! Config:
//! ```toml
//! [ai]
//! provider = "azure-openai"
//! model = "gpt-4o"                # deployment name in Azure
//! azure_openai_endpoint = "https://{resource}.openai.azure.com"
//! azure_openai_api_version = "2024-02-01"   # optional, defaults to this
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use super::{system_prompt, AiProvider, ReviewComment, ReviewContext};
use crate::config::AiConfig;
use crate::error::{MerlinError, Result};

const DEFAULT_API_VERSION: &str = "2024-02-01";

pub struct AzureOpenAiProvider {
    api_key: String,
    config: AiConfig,
    client: reqwest::Client,
}

impl AzureOpenAiProvider {
    pub fn new(api_key: String, config: AiConfig) -> Self {
        Self {
            api_key,
            config,
            client: reqwest::Client::new(),
        }
    }

    fn endpoint_url(&self) -> Result<String> {
        let base = self
            .config
            .azure_openai_endpoint
            .as_deref()
            .ok_or_else(|| {
                MerlinError::Config(
                    "azure_openai_endpoint is required for azure-openai provider".to_string(),
                )
            })?;
        let version = self
            .config
            .azure_openai_api_version
            .as_deref()
            .unwrap_or(DEFAULT_API_VERSION);
        Ok(format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            base.trim_end_matches('/'),
            self.config.model,
            version
        ))
    }
}

// ── Request / Response types (same schema as OpenAI) ─────────────────────────

#[derive(Serialize)]
struct AzureRequest {
    messages: Vec<AzureMessage>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Serialize, Deserialize)]
struct AzureMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AzureResponse {
    choices: Vec<AzureChoice>,
}

#[derive(Deserialize)]
struct AzureChoice {
    message: AzureMessage,
}

// ─────────────────────────────────────────────────────────────────────────────

#[async_trait]
impl AiProvider for AzureOpenAiProvider {
    #[instrument(skip(self, system, user))]
    async fn generate(&self, system: &str, user: &str) -> Result<String> {
        let url = self.endpoint_url()?;

        let request = AzureRequest {
            messages: vec![
                AzureMessage {
                    role: "system".to_string(),
                    content: system.to_string(),
                },
                AzureMessage {
                    role: "user".to_string(),
                    content: user.to_string(),
                },
            ],
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
        };

        debug!(
            "Sending request to Azure OpenAI deployment: {}",
            self.config.model
        );

        let resp = self
            .client
            .post(&url)
            .header("api-key", &self.api_key)
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(MerlinError::AiProvider(format!(
                "Azure OpenAI error {status}: {body}"
            )));
        }

        let result: AzureResponse = resp.json().await?;
        result
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| MerlinError::AiProvider("Empty Azure OpenAI response".to_string()))
    }

    #[instrument(skip(self, ctx), fields(file = %ctx.file))]
    async fn review(&self, ctx: &ReviewContext) -> Result<Vec<ReviewComment>> {
        let system = system_prompt(&[
            "bugs".to_string(),
            "security".to_string(),
            "style".to_string(),
            "performance".to_string(),
        ]);
        let user = format!(
            "Review the following diff for file `{}`:\n\n```diff\n{}\n```",
            ctx.file, ctx.diff_hunk
        );
        let raw = self.generate(&system, &user).await?;
        let cleaned = raw
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();
        serde_json::from_str(cleaned).map_err(|e| {
            MerlinError::AiProvider(format!(
                "Failed to parse Azure OpenAI response: {e}\nRaw: {cleaned}"
            ))
        })
    }
}
