//! RAG (Retrieval-Augmented Generation) pipeline for context-aware code review.
//!
//! ## Architecture
//!
//! ```text
//!  Index time                           Query time
//!  ──────────                           ──────────
//!  Source files ─┐                      Diff chunk ──▶ embed ──▶ store.search()
//!  Past comments  ├──▶ embed ──▶ store  Top-K docs  ────────────────────────────▶ AI prompt
//!  Docs / issues ─┘                     (similar code, past comments, related issues)
//! ```
//!
//! ## Supported vector stores
//!
//! | Store    | Setup                              | Best for               |
//! |----------|------------------------------------|------------------------|
//! | `local`  | None — JSONL flat file             | Small repos, dev/CI    |
//! | `memory` | None — ephemeral RAM               | Testing                |
//! | `qdrant` | `docker run -p 6333:6333 qdrant/qdrant` | Production self-hosted |
//! | `chroma` | `docker run -p 8000:8000 chromadb/chroma` | Open-source alternative |
//! | `pinecone`| cloud.pinecone.io account         | Managed cloud          |
//!
//! ## Quick start
//!
//! ```toml
//! # merlin.toml
//! [rag]
//! enabled = true
//! store = "local"           # zero setup
//! embed_model = "nomic-embed-text"
//! ```
//!
//! ```bash
//! ollama pull nomic-embed-text   # one-time
//! merlin rag index               # index your codebase
//! merlin rag search "auth bypass"  # test retrieval
//! ```

pub mod embedder;
pub mod indexer;
pub mod retriever;
pub mod store;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::config::{EmbedderType, RagConfig, VectorStoreType};
use crate::error::Result;

// ── Core types ─────────────────────────────────────────────────────────────────

/// A dense embedding vector (f32 components).
pub type Embedding = Vec<f32>;

/// A document to be indexed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Stable unique identifier (e.g. `"file:src/main.rs:0"`, `"comment:PR#42:1"`).
    pub id: String,
    /// Plain-text content to embed and retrieve.
    pub content: String,
    /// Source category: `"codebase"`, `"review_comment"`, `"docs"`, `"issue"`.
    pub source: String,
    /// Extra metadata (file path, PR number, line range, etc.).
    pub metadata: serde_json::Value,
}

/// A single retrieval result from the vector store.
#[derive(Debug, Clone)]
pub struct RetrievedDoc {
    pub content: String,
    pub source: String,
    pub score: f32,
    pub metadata: serde_json::Value,
}

// ── Traits ─────────────────────────────────────────────────────────────────────

/// Produces dense embedding vectors from text.
#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Embedding>;
}

/// Stores and searches embedding vectors.
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Create the collection if it does not exist. `dimension` = vector size.
    async fn ensure_collection(&self, collection: &str, dimension: usize) -> Result<()>;

    /// Upsert a batch of `(Document, Embedding)` pairs.
    async fn upsert(&self, collection: &str, docs: &[(Document, Embedding)]) -> Result<()>;

    /// Return the `limit` closest documents to `query_vec` with score ≥ `min_score`.
    async fn search(
        &self,
        collection: &str,
        query_vec: &Embedding,
        limit: usize,
        min_score: f32,
    ) -> Result<Vec<RetrievedDoc>>;

    /// Delete all data in the collection.
    async fn clear(&self, collection: &str) -> Result<()>;

    /// Return number of indexed documents.
    async fn count(&self, collection: &str) -> Result<usize>;
}

// ── Pipeline ───────────────────────────────────────────────────────────────────

/// Top-level RAG pipeline: ties together embedder, store, and config.
pub struct RagPipeline {
    pub embedder: Box<dyn Embedder>,
    pub store: Box<dyn VectorStore>,
    pub config: RagConfig,
}

impl RagPipeline {
    pub fn new(
        embedder: Box<dyn Embedder>,
        store: Box<dyn VectorStore>,
        config: RagConfig,
    ) -> Self {
        Self { embedder, store, config }
    }

    /// Retrieve the most relevant documents for a query string.
    pub async fn retrieve(&self, query: &str, limit: usize) -> Result<Vec<RetrievedDoc>> {
        let q_vec = self.embedder.embed(query).await?;
        self.store
            .search(&self.config.collection, &q_vec, limit, self.config.min_score)
            .await
    }

    /// Embed and index a batch of documents.
    /// Returns the number of documents successfully indexed.
    pub async fn index_documents(&self, docs: Vec<Document>) -> Result<usize> {
        if docs.is_empty() {
            return Ok(0);
        }

        // Embed sequentially (Ollama /api/embeddings is one-at-a-time)
        let mut pairs: Vec<(Document, Embedding)> = Vec::with_capacity(docs.len());
        for doc in docs {
            let emb = self.embedder.embed(&doc.content).await?;
            pairs.push((doc, emb));
        }

        let dim = pairs[0].1.len();
        self.store.ensure_collection(&self.config.collection, dim).await?;
        let count = pairs.len();
        self.store.upsert(&self.config.collection, &pairs).await?;
        Ok(count)
    }

    /// Clear all indexed data.
    pub async fn clear(&self) -> Result<()> {
        self.store.clear(&self.config.collection).await
    }

    /// Return total document count.
    pub async fn count(&self) -> Result<usize> {
        self.store.count(&self.config.collection).await
    }
}

// ── Factory ────────────────────────────────────────────────────────────────────

/// Build a `RagPipeline` from config.
///
/// Embedder selection:
/// - `embedder = "ollama"` (default) — local Ollama instance
/// - `embedder = "openai"` — OpenAI Embeddings API, reads `OPENAI_API_KEY`
pub fn build_pipeline(config: &RagConfig) -> RagPipeline {
    let emb: Box<dyn Embedder> = match config.embedder {
        EmbedderType::Openai => {
            match embedder::OpenAiEmbedder::from_env(config.embed_model.clone()) {
                Ok(e) => {
                    tracing::info!(
                        "RAG: using OpenAI embedder (model={})",
                        config.embed_model
                    );
                    Box::new(e)
                }
                Err(e) => {
                    tracing::warn!(
                        "RAG: OPENAI_API_KEY not set ({e}), falling back to Ollama embedder"
                    );
                    Box::new(embedder::OllamaEmbedder::new(
                        config.ollama_base_url.clone(),
                        config.embed_model.clone(),
                    ))
                }
            }
        }
        EmbedderType::Ollama => {
            tracing::debug!(
                "RAG: using Ollama embedder (url={}, model={})",
                config.ollama_base_url,
                config.embed_model
            );
            Box::new(embedder::OllamaEmbedder::new(
                config.ollama_base_url.clone(),
                config.embed_model.clone(),
            ))
        }
    };

    let st: Box<dyn VectorStore> = match config.store {
        VectorStoreType::Local => {
            Box::new(store::local::LocalStore::new(config.local_path.clone()))
        }
        VectorStoreType::Memory => Box::new(store::memory::MemoryStore::new()),
        VectorStoreType::Qdrant => Box::new(store::qdrant::QdrantStore::new(
            config.qdrant_url.clone(),
            config.qdrant_api_key.clone(),
        )),
        VectorStoreType::Chroma => Box::new(store::chroma::ChromaStore::new(
            config.chroma_url.clone(),
        )),
        VectorStoreType::Pinecone => Box::new(store::pinecone::PineconeStore::new(
            config.pinecone_api_key.clone().or_else(|| {
                std::env::var("PINECONE_API_KEY").ok()
            }),
            config.pinecone_host.clone(),
        )),
    };

    RagPipeline::new(emb, st, config.clone())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::VectorStoreType;

    #[test]
    fn test_rag_config_defaults() {
        let cfg = RagConfig::default();
        assert!(!cfg.enabled);
        assert_eq!(cfg.store, VectorStoreType::Local);
        assert_eq!(cfg.top_k, 5);
        assert_eq!(cfg.embed_model, "nomic-embed-text");
    }

    #[test]
    fn test_build_pipeline_local() {
        let cfg = RagConfig::default();
        let pipeline = build_pipeline(&cfg);
        // Just ensures it builds without panicking
        assert_eq!(pipeline.config.collection, "merlin");
    }
}
