//! Qdrant vector store backend — REST API.
//!
//! ## Setup
//! ```bash
//! docker run -p 6333:6333 qdrant/qdrant
//! ```
//!
//! ## Qdrant Cloud
//! Set `qdrant_url` to your cluster URL and `qdrant_api_key` to your API key.
//!
//! Docs: <https://qdrant.tech/documentation/interfaces/rest-api/>

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use super::doc_id_to_u64;
use crate::error::{MerlinError, Result};
use crate::rag::{Document, Embedding, RetrievedDoc, VectorStore};

/// Vector store backed by a Qdrant REST API.
pub struct QdrantStore {
    base_url: String,
    api_key: Option<String>,
    client: reqwest::Client,
}

impl QdrantStore {
    /// Create a new Qdrant store connecting to `base_url` with optional API key.
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        Self {
            base_url,
            api_key,
            client: reqwest::Client::new(),
        }
    }

    fn req(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{path}", self.base_url);
        let mut r = self.client.request(method, url);
        if let Some(key) = &self.api_key {
            r = r.header("api-key", key);
        }
        r
    }
}

// ── Qdrant REST types ─────────────────────────────────────────────────────────

#[derive(Serialize)]
struct CreateCollection {
    vectors: VectorParams,
}

#[derive(Serialize)]
struct VectorParams {
    size: usize,
    distance: &'static str,
}

#[derive(Serialize)]
struct UpsertBody {
    points: Vec<QdrantPoint>,
}

#[derive(Serialize)]
struct QdrantPoint {
    id: u64,
    vector: Vec<f32>,
    payload: serde_json::Value,
}

#[derive(Serialize)]
struct SearchBody {
    vector: Vec<f32>,
    limit: usize,
    score_threshold: f32,
    with_payload: bool,
}

#[derive(Deserialize)]
struct SearchResponse {
    result: Vec<QdrantSearchResult>,
}

#[derive(Deserialize)]
struct QdrantSearchResult {
    score: f32,
    payload: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct CountResponse {
    result: CountResult,
}

#[derive(Deserialize)]
struct CountResult {
    count: usize,
}

// ── Implementation ────────────────────────────────────────────────────────────

#[async_trait]
impl VectorStore for QdrantStore {
    async fn ensure_collection(&self, collection: &str, dimension: usize) -> Result<()> {
        // Check existence first
        let resp = self
            .req(reqwest::Method::GET, &format!("/collections/{collection}"))
            .send()
            .await?;
        if resp.status().is_success() {
            debug!("Qdrant: collection '{collection}' already exists");
            return Ok(());
        }

        info!("Qdrant: creating collection '{collection}' (dim={dimension})");
        let body = CreateCollection {
            vectors: VectorParams {
                size: dimension,
                distance: "Cosine",
            },
        };
        let resp = self
            .req(reqwest::Method::PUT, &format!("/collections/{collection}"))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(MerlinError::Platform(format!(
                "Qdrant create collection error {status}: {text}"
            )));
        }
        Ok(())
    }

    async fn upsert(&self, collection: &str, docs: &[(Document, Embedding)]) -> Result<()> {
        for chunk in docs.chunks(100) {
            let points: Vec<QdrantPoint> = chunk
                .iter()
                .map(|(doc, emb)| QdrantPoint {
                    id: doc_id_to_u64(&doc.id),
                    vector: emb.clone(),
                    payload: serde_json::json!({
                        "doc_id":  doc.id,
                        "content": doc.content,
                        "source":  doc.source,
                        "metadata": doc.metadata,
                    }),
                })
                .collect();

            let resp = self
                .req(
                    reqwest::Method::PUT,
                    &format!("/collections/{collection}/points"),
                )
                .json(&UpsertBody { points })
                .send()
                .await?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                warn!("Qdrant upsert error {status}: {text}");
                return Err(MerlinError::Platform(format!(
                    "Qdrant upsert error {status}: {text}"
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
        let body = SearchBody {
            vector: query_vec.clone(),
            limit,
            score_threshold: min_score,
            with_payload: true,
        };
        let resp = self
            .req(
                reqwest::Method::POST,
                &format!("/collections/{collection}/points/search"),
            )
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(MerlinError::Platform(format!(
                "Qdrant search error {status}: {text}"
            )));
        }

        let r: SearchResponse = resp.json().await?;
        Ok(r.result
            .into_iter()
            .filter_map(|hit| {
                let p = hit.payload?;
                Some(RetrievedDoc {
                    content: p["content"].as_str()?.to_string(),
                    source: p["source"].as_str().unwrap_or("unknown").to_string(),
                    score: hit.score,
                    metadata: p["metadata"].clone(),
                })
            })
            .collect())
    }

    async fn clear(&self, collection: &str) -> Result<()> {
        let resp = self
            .req(
                reqwest::Method::DELETE,
                &format!("/collections/{collection}"),
            )
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(MerlinError::Platform(format!(
                "Qdrant delete collection error {status}: {text}"
            )));
        }
        info!("Qdrant: deleted collection '{collection}'");
        Ok(())
    }

    async fn count(&self, collection: &str) -> Result<usize> {
        let resp = self
            .req(
                reqwest::Method::POST,
                &format!("/collections/{collection}/points/count"),
            )
            .json(&serde_json::json!({}))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Ok(0);
        }
        let r: CountResponse = resp.json().await.unwrap_or(CountResponse {
            result: CountResult { count: 0 },
        });
        Ok(r.result.count)
    }
}
