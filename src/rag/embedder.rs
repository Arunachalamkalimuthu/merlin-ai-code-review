//! Embedding backends for the RAG pipeline.
//!
//! ## Backends
//!
//! | Backend | When to use |
//! |---------|-------------|
//! | `OllamaEmbedder` | Local dev — free, fully private, needs `ollama serve` |
//! | `OpenAiEmbedder` | CI/CD — works on any runner, needs `OPENAI_API_KEY` |

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::debug;

use super::{Embedder, Embedding};
use crate::error::{MerlinError, Result};

// ── Ollama ────────────────────────────────────────────────────────────────────

/// Embedding via a local Ollama instance (`POST /api/embeddings`).
///
/// Pull a model first: `ollama pull nomic-embed-text`
/// Ollama serves on `http://localhost:11434` by default.
pub struct OllamaEmbedder {
    base_url: String,
    model: String,
    client: reqwest::Client,
}

impl OllamaEmbedder {
    pub fn new(base_url: String, model: String) -> Self {
        Self {
            base_url,
            model,
            client: reqwest::Client::new(),
        }
    }
}

#[derive(Serialize)]
struct OllamaEmbedRequest<'a> {
    model: &'a str,
    prompt: &'a str,
}

#[derive(Deserialize)]
struct OllamaEmbedResponse {
    embedding: Vec<f32>,
}

#[async_trait]
impl Embedder for OllamaEmbedder {
    async fn embed(&self, text: &str) -> Result<Embedding> {
        debug!(
            "OllamaEmbedder: embedding {} chars with model '{}'",
            text.len(),
            self.model
        );

        let resp = self
            .client
            .post(format!("{}/api/embeddings", self.base_url))
            .json(&OllamaEmbedRequest {
                model: &self.model,
                prompt: text,
            })
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(MerlinError::AiProvider(format!(
                "Ollama embedding error {status}: {body}\n\
                 Ensure Ollama is running (`ollama serve`) and the model is pulled \
                 (`ollama pull {}`)",
                self.model
            )));
        }

        let r: OllamaEmbedResponse = resp.json().await?;
        Ok(r.embedding)
    }
}

// ── OpenAI ────────────────────────────────────────────────────────────────────

/// Embedding via the OpenAI Embeddings API (`POST /v1/embeddings`).
///
/// Requires `OPENAI_API_KEY` environment variable (or pass the key directly).
/// Default model: `text-embedding-3-small` (1 536 dims, ~$0.02 / 1 M tokens).
///
/// This is the recommended backend for CI/CD pipelines where Ollama is
/// not available.
pub struct OpenAiEmbedder {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl OpenAiEmbedder {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            client: reqwest::Client::new(),
        }
    }

    /// Build from environment — reads `OPENAI_API_KEY`.
    pub fn from_env(model: String) -> Result<Self> {
        let key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| MerlinError::EnvVar("OPENAI_API_KEY".to_string()))?;
        Ok(Self::new(key, model))
    }
}

#[derive(Serialize)]
struct OpenAiEmbedRequest<'a> {
    model: &'a str,
    input: &'a str,
}

#[derive(Deserialize)]
struct OpenAiEmbedResponse {
    data: Vec<OpenAiEmbedData>,
}

#[derive(Deserialize)]
struct OpenAiEmbedData {
    embedding: Vec<f32>,
}

#[async_trait]
impl Embedder for OpenAiEmbedder {
    async fn embed(&self, text: &str) -> Result<Embedding> {
        debug!(
            "OpenAiEmbedder: embedding {} chars with model '{}'",
            text.len(),
            self.model
        );

        let resp = self
            .client
            .post("https://api.openai.com/v1/embeddings")
            .bearer_auth(&self.api_key)
            .json(&OpenAiEmbedRequest {
                model: &self.model,
                input: text,
            })
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(MerlinError::AiProvider(format!(
                "OpenAI embedding error {status}: {body}"
            )));
        }

        let r: OpenAiEmbedResponse = resp.json().await?;
        r.data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .ok_or_else(|| {
                MerlinError::AiProvider("OpenAI embeddings response contained no data".to_string())
            })
    }
}
