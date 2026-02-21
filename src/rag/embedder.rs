//! Ollama embedding backend — POST /api/embeddings.
//!
//! Pull a model first: `ollama pull nomic-embed-text`
//! Ollama serves on `http://localhost:11434` by default.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::debug;

use super::{Embedder, Embedding};
use crate::error::{MerlinError, Result};

pub struct OllamaEmbedder {
    base_url: String,
    model: String,
    client: reqwest::Client,
}

impl OllamaEmbedder {
    pub fn new(base_url: String, model: String) -> Self {
        Self { base_url, model, client: reqwest::Client::new() }
    }
}

#[derive(Serialize)]
struct EmbedRequest<'a> {
    model: &'a str,
    prompt: &'a str,
}

#[derive(Deserialize)]
struct EmbedResponse {
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
            .json(&EmbedRequest { model: &self.model, prompt: text })
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

        let r: EmbedResponse = resp.json().await?;
        Ok(r.embedding)
    }
}
