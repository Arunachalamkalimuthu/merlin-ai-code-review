//! ChromaDB vector store backend — REST API v1.
//!
//! ## Setup
//! ```bash
//! docker run -p 8000:8000 chromadb/chroma
//! # or: pip install chromadb && chroma run
//! ```
//!
//! Docs: <https://docs.trychroma.com/reference/rest-api>

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::error::{MerlinError, Result};
use crate::rag::{Document, Embedding, RetrievedDoc, VectorStore};

/// Vector store backed by a ChromaDB REST API.
pub struct ChromaStore {
    base_url: String,
    client: reqwest::Client,
}

impl ChromaStore {
    /// Create a new ChromaDB store connecting to `base_url`.
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
    }

    fn api(&self, path: &str) -> String {
        format!("{}/api/v1{path}", self.base_url)
    }
}

// ── ChromaDB REST types ───────────────────────────────────────────────────────

#[derive(Serialize)]
struct CreateCollectionBody {
    name: String,
    get_or_create: bool,
}

#[derive(Deserialize)]
struct ChromaCollection {
    id: String,
}

#[derive(Serialize)]
struct AddBody {
    ids: Vec<String>,
    embeddings: Vec<Vec<f32>>,
    documents: Vec<String>,
    metadatas: Vec<serde_json::Value>,
}

#[derive(Serialize)]
struct QueryBody {
    query_embeddings: Vec<Vec<f32>>,
    n_results: usize,
    include: Vec<&'static str>,
}

#[derive(Deserialize)]
struct QueryResponse {
    ids: Vec<Vec<String>>,
    distances: Vec<Vec<f32>>,
    documents: Vec<Vec<Option<String>>>,
    metadatas: Vec<Vec<Option<serde_json::Value>>>,
}

// ── Implementation ────────────────────────────────────────────────────────────

#[async_trait]
impl VectorStore for ChromaStore {
    async fn ensure_collection(&self, collection: &str, _dimension: usize) -> Result<()> {
        let body = CreateCollectionBody {
            name: collection.to_string(),
            get_or_create: true,
        };
        let resp = self
            .client
            .post(self.api("/collections"))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(MerlinError::Platform(format!(
                "Chroma create collection error {status}: {text}"
            )));
        }
        debug!("Chroma: collection '{collection}' ready");
        Ok(())
    }

    async fn upsert(&self, collection: &str, docs: &[(Document, Embedding)]) -> Result<()> {
        // Resolve collection ID
        let col_id = self.get_collection_id(collection).await?;

        for chunk in docs.chunks(100) {
            let body = AddBody {
                ids: chunk.iter().map(|(d, _)| d.id.clone()).collect(),
                embeddings: chunk.iter().map(|(_, e)| e.clone()).collect(),
                documents: chunk.iter().map(|(d, _)| d.content.clone()).collect(),
                metadatas: chunk
                    .iter()
                    .map(|(d, _)| {
                        serde_json::json!({
                            "source":   d.source,
                            "metadata": d.metadata,
                        })
                    })
                    .collect(),
            };

            // Chroma uses /upsert which handles both add and update
            let resp = self
                .client
                .post(self.api(&format!("/collections/{col_id}/upsert")))
                .json(&body)
                .send()
                .await?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                warn!("Chroma upsert error {status}: {text}");
                return Err(MerlinError::Platform(format!(
                    "Chroma upsert error {status}: {text}"
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
        let col_id = self.get_collection_id(collection).await?;
        let body = QueryBody {
            query_embeddings: vec![query_vec.clone()],
            n_results: limit,
            include: vec!["documents", "metadatas", "distances"],
        };

        let resp = self
            .client
            .post(self.api(&format!("/collections/{col_id}/query")))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(MerlinError::Platform(format!(
                "Chroma query error {status}: {text}"
            )));
        }

        let r: QueryResponse = resp.json().await?;

        // Chroma returns L2 distance; convert to cosine-like similarity (1 - distance/2)
        // For cosine space: distance = 1 - cosine_sim → cosine_sim = 1 - distance
        let results = r
            .ids
            .into_iter()
            .zip(r.distances)
            .zip(r.documents)
            .zip(r.metadatas)
            .flat_map(|(((_, dists), docs), metas)| dists.into_iter().zip(docs).zip(metas))
            .filter_map(|((dist, doc_opt), meta_opt)| {
                let content = doc_opt?;
                let score = 1.0 - dist; // convert distance → similarity
                if score < min_score {
                    return None;
                }
                let meta = meta_opt.unwrap_or(serde_json::json!({}));
                let source = meta["source"].as_str().unwrap_or("unknown").to_string();
                Some(RetrievedDoc {
                    content,
                    source,
                    score,
                    metadata: meta["metadata"].clone(),
                })
            })
            .collect();

        Ok(results)
    }

    async fn clear(&self, collection: &str) -> Result<()> {
        let resp = self
            .client
            .delete(self.api(&format!("/collections/{collection}")))
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(MerlinError::Platform(format!(
                "Chroma delete collection error {status}: {text}"
            )));
        }
        info!("Chroma: deleted collection '{collection}'");
        Ok(())
    }

    async fn count(&self, collection: &str) -> Result<usize> {
        let col_id = match self.get_collection_id(collection).await {
            Ok(id) => id,
            Err(_) => return Ok(0),
        };
        let resp = self
            .client
            .get(self.api(&format!("/collections/{col_id}/count")))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Ok(0);
        }
        Ok(resp.json::<usize>().await.unwrap_or(0))
    }
}

impl ChromaStore {
    async fn get_collection_id(&self, name: &str) -> Result<String> {
        let resp = self
            .client
            .get(self.api(&format!("/collections/{name}")))
            .send()
            .await?;

        if !resp.status().is_success() {
            // Try to create it
            self.ensure_collection(name, 0).await?;
            let resp2 = self
                .client
                .get(self.api(&format!("/collections/{name}")))
                .send()
                .await?;
            let col: ChromaCollection = resp2.json().await?;
            return Ok(col.id);
        }

        let col: ChromaCollection = resp.json().await?;
        Ok(col.id)
    }
}
