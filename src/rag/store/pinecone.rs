//! Pinecone cloud vector database backend — REST API v1.
//!
//! ## Setup
//!
//! 1. Create an account at <https://app.pinecone.io>
//! 2. Create an index (choose **cosine** metric, match your embedding dimension)
//! 3. Copy the **API key** and the **index host URL**
//!
//! ```toml
//! [rag]
//! store = "pinecone"
//! pinecone_api_key = "pcsk_..."          # or set PINECONE_API_KEY env var
//! pinecone_host = "https://my-index-xyz.svc.us-east1.pinecone.io"
//! ```
//!
//! Docs: <https://docs.pinecone.io/reference/api/introduction>

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::error::{MerlinError, Result};
use crate::rag::{Document, Embedding, RetrievedDoc, VectorStore};

pub struct PineconeStore {
    api_key: Option<String>,
    host: Option<String>,
    client: reqwest::Client,
}

impl PineconeStore {
    pub fn new(api_key: Option<String>, host: Option<String>) -> Self {
        Self { api_key, host, client: reqwest::Client::new() }
    }

    fn host_url(&self) -> Result<&str> {
        self.host.as_deref().ok_or_else(|| {
            MerlinError::Config(
                "Pinecone: `pinecone_host` not configured in [rag] section of merlin.toml. \
                 Set it to your index host URL, e.g. \
                 https://my-index-xyz.svc.us-east1.pinecone.io"
                    .to_string(),
            )
        })
    }

    fn auth(&self) -> Result<String> {
        self.api_key.clone().ok_or_else(|| {
            MerlinError::EnvVar("PINECONE_API_KEY".to_string())
        })
    }

    fn req(&self, method: reqwest::Method, path: &str) -> Result<reqwest::RequestBuilder> {
        let url = format!("{}{path}", self.host_url()?);
        let key = self.auth()?;
        Ok(self.client.request(method, url).header("Api-Key", key))
    }
}

// ── Pinecone REST types ───────────────────────────────────────────────────────

#[derive(Serialize)]
struct UpsertBody {
    vectors: Vec<PineconeVector>,
    namespace: String,
}

#[derive(Serialize)]
struct PineconeVector {
    id: String,
    values: Vec<f32>,
    metadata: serde_json::Value,
}

#[derive(Serialize)]
struct QueryBody {
    vector: Vec<f32>,
    #[serde(rename = "topK")]
    top_k: usize,
    namespace: String,
    include_metadata: bool,
}

#[derive(Deserialize)]
struct QueryResponse {
    matches: Vec<PineconeMatch>,
}

#[derive(Deserialize)]
struct PineconeMatch {
    score: f32,
    metadata: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct StatsResponse {
    total_vector_count: Option<usize>,
    namespaces: Option<serde_json::Value>,
}

// ── Implementation ────────────────────────────────────────────────────────────

#[async_trait]
impl VectorStore for PineconeStore {
    async fn ensure_collection(&self, _collection: &str, _dimension: usize) -> Result<()> {
        // Pinecone indexes are created via the console or control-plane API.
        // Ferret uses existing indexes; `collection` maps to the namespace.
        Ok(())
    }

    async fn upsert(&self, collection: &str, docs: &[(Document, Embedding)]) -> Result<()> {
        for chunk in docs.chunks(100) {
            let vectors: Vec<PineconeVector> = chunk
                .iter()
                .map(|(doc, emb)| PineconeVector {
                    id: doc.id.clone(),
                    values: emb.clone(),
                    metadata: serde_json::json!({
                        "content": doc.content,
                        "source":  doc.source,
                        "meta":    doc.metadata,
                    }),
                })
                .collect();

            let body = UpsertBody {
                vectors,
                namespace: collection.to_string(),
            };

            let resp = self
                .req(reqwest::Method::POST, "/vectors/upsert")?
                .json(&body)
                .send()
                .await?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                warn!("Pinecone upsert error {status}: {text}");
                return Err(MerlinError::Platform(format!(
                    "Pinecone upsert error {status}: {text}"
                )));
            }
        }
        Ok(())
    }

    async fn search(
        &self,
        collection: &str,
        query_vec: &Embedding,
        limit: usize,
        min_score: f32,
    ) -> Result<Vec<RetrievedDoc>> {
        let body = QueryBody {
            vector: query_vec.clone(),
            top_k: limit,
            namespace: collection.to_string(),
            include_metadata: true,
        };

        let resp = self
            .req(reqwest::Method::POST, "/query")?
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(MerlinError::Platform(format!(
                "Pinecone query error {status}: {text}"
            )));
        }

        let r: QueryResponse = resp.json().await?;
        Ok(r.matches
            .into_iter()
            .filter(|m| m.score >= min_score)
            .filter_map(|m| {
                let meta = m.metadata?;
                let content = meta["content"].as_str()?.to_string();
                let source =
                    meta["source"].as_str().unwrap_or("unknown").to_string();
                Some(RetrievedDoc {
                    content,
                    source,
                    score: m.score,
                    metadata: meta["meta"].clone(),
                })
            })
            .collect())
    }

    async fn clear(&self, collection: &str) -> Result<()> {
        // Delete all vectors in the namespace
        let body = serde_json::json!({ "deleteAll": true, "namespace": collection });
        let resp = self
            .req(reqwest::Method::POST, "/vectors/delete")?
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(MerlinError::Platform(format!(
                "Pinecone delete namespace error {status}: {text}"
            )));
        }
        info!("Pinecone: cleared namespace '{collection}'");
        Ok(())
    }

    async fn count(&self, collection: &str) -> Result<usize> {
        let resp = self
            .req(reqwest::Method::GET, "/describe_index_stats")?
            .send()
            .await?;

        if !resp.status().is_success() {
            return Ok(0);
        }

        let stats: StatsResponse = resp.json().await.unwrap_or(StatsResponse {
            total_vector_count: None,
            namespaces: None,
        });

        // Try to get namespace-specific count
        if let Some(ns) = stats.namespaces {
            if let Some(n) = ns[collection]["vectorCount"].as_u64() {
                return Ok(n as usize);
            }
        }

        Ok(stats.total_vector_count.unwrap_or(0))
    }
}
