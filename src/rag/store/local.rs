//! Local JSONL vector store — zero external dependencies.
//!
//! Stores vectors in a JSONL file (one JSON object per line).
//! On startup the file is loaded into memory; writes are appended.
//! Searches run brute-force cosine similarity over the in-memory index.
//!
//! Suitable for repos up to ~10 K files / ~100 K chunks.

use std::io::Write;
use std::sync::RwLock;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use super::cosine_similarity;
use crate::error::{MerlinError, Result};
use crate::rag::{Document, Embedding, RetrievedDoc, VectorStore};

// ── On-disk record ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
struct LocalRecord {
    collection: String,
    id: String,
    vector: Vec<f32>,
    content: String,
    source: String,
    metadata: serde_json::Value,
}

// ── In-memory entry (after load) ─────────────────────────────────────────────

struct Entry {
    id: String,
    vector: Vec<f32>,
    content: String,
    source: String,
    metadata: serde_json::Value,
}

// ── Store ─────────────────────────────────────────────────────────────────────

/// Zero-dependency vector store that persists embeddings as a JSONL file.
pub struct LocalStore {
    path: String,
    /// Per-collection in-memory index. Key = collection name.
    index: RwLock<std::collections::HashMap<String, Vec<Entry>>>,
}

impl LocalStore {
    /// Create a new local store backed by the JSONL file at `path`.
    pub fn new(path: String) -> Self {
        let index = load_from_file(&path);
        Self {
            path,
            index: RwLock::new(index),
        }
    }
}

fn load_from_file(path: &str) -> std::collections::HashMap<String, Vec<Entry>> {
    let mut map: std::collections::HashMap<String, Vec<Entry>> = std::collections::HashMap::new();

    let Ok(content) = std::fs::read_to_string(path) else {
        return map;
    };

    for line in content.lines() {
        if let Ok(rec) = serde_json::from_str::<LocalRecord>(line) {
            map.entry(rec.collection).or_default().push(Entry {
                id: rec.id,
                vector: rec.vector,
                content: rec.content,
                source: rec.source,
                metadata: rec.metadata,
            });
        }
    }
    map
}

#[async_trait]
impl VectorStore for LocalStore {
    async fn ensure_collection(&self, _collection: &str, _dimension: usize) -> Result<()> {
        // Local store creates collections implicitly on first upsert
        Ok(())
    }

    async fn upsert(&self, collection: &str, docs: &[(Document, Embedding)]) -> Result<()> {
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;

        let mut index = self
            .index
            .write()
            .map_err(|_| MerlinError::Other("LocalStore: RwLock poisoned".to_string()))?;
        let entries = index.entry(collection.to_string()).or_default();

        for (doc, emb) in docs {
            // Remove existing entry with same id (upsert semantics)
            entries.retain(|e| e.id != doc.id);

            let rec = LocalRecord {
                collection: collection.to_string(),
                id: doc.id.clone(),
                vector: emb.clone(),
                content: doc.content.clone(),
                source: doc.source.clone(),
                metadata: doc.metadata.clone(),
            };
            let line = serde_json::to_string(&rec)?;
            writeln!(f, "{line}")?;

            entries.push(Entry {
                id: doc.id.clone(),
                vector: emb.clone(),
                content: doc.content.clone(),
                source: doc.source.clone(),
                metadata: doc.metadata.clone(),
            });
        }

        debug!(
            "LocalStore: upserted {} docs into collection '{collection}'",
            docs.len()
        );
        Ok(())
    }

    async fn search(
        &self,
        collection: &str,
        query_vec: &Embedding,
        limit: usize,
        min_score: f32,
    ) -> Result<Vec<RetrievedDoc>> {
        let index = self
            .index
            .read()
            .map_err(|_| MerlinError::Other("LocalStore: RwLock poisoned".to_string()))?;

        let Some(entries) = index.get(collection) else {
            return Ok(vec![]);
        };

        // Compute cosine similarity for every entry
        let mut scored: Vec<(f32, &Entry)> = entries
            .iter()
            .filter_map(|e| {
                if e.vector.len() != query_vec.len() {
                    return None;
                }
                let score = cosine_similarity(&e.vector, query_vec);
                if score >= min_score {
                    Some((score, e))
                } else {
                    None
                }
            })
            .collect();

        // Sort descending by score
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        Ok(scored
            .into_iter()
            .map(|(score, e)| RetrievedDoc {
                content: e.content.clone(),
                source: e.source.clone(),
                score,
                metadata: e.metadata.clone(),
            })
            .collect())
    }

    async fn clear(&self, collection: &str) -> Result<()> {
        {
            let mut index = self
                .index
                .write()
                .map_err(|_| MerlinError::Other("LocalStore: RwLock poisoned".to_string()))?;
            index.remove(collection);
        }

        // Rewrite the file without this collection's records
        let index = self
            .index
            .read()
            .map_err(|_| MerlinError::Other("LocalStore: RwLock poisoned".to_string()))?;
        let mut f = std::fs::File::create(&self.path)?;
        for (col, entries) in index.iter() {
            for e in entries {
                let rec = LocalRecord {
                    collection: col.clone(),
                    id: e.id.clone(),
                    vector: e.vector.clone(),
                    content: e.content.clone(),
                    source: e.source.clone(),
                    metadata: e.metadata.clone(),
                };
                let line = serde_json::to_string(&rec)?;
                writeln!(f, "{line}")?;
            }
        }

        info!("LocalStore: cleared collection '{collection}'");
        Ok(())
    }

    async fn count(&self, collection: &str) -> Result<usize> {
        let index = self
            .index
            .read()
            .map_err(|_| MerlinError::Other("LocalStore: RwLock poisoned".to_string()))?;
        Ok(index.get(collection).map(|e| e.len()).unwrap_or(0))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rag::Document;

    fn make_doc(id: &str, content: &str) -> (Document, Embedding) {
        (
            Document {
                id: id.to_string(),
                content: content.to_string(),
                source: "codebase".to_string(),
                metadata: serde_json::json!({}),
            },
            vec![1.0f32, 0.0, 0.0],
        )
    }

    #[tokio::test]
    async fn test_local_upsert_and_search() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rag.jsonl").to_string_lossy().to_string();
        let store = LocalStore::new(path);

        let docs = vec![
            make_doc("doc1", "fn authenticate(user: &User) -> bool"),
            make_doc("doc2", "fn hash_password(pwd: &str) -> String"),
        ];
        store.upsert("test", &docs).await.unwrap();

        let query = vec![1.0f32, 0.0, 0.0]; // identical to stored vectors
        let results = store.search("test", &query, 5, 0.0).await.unwrap();
        assert_eq!(results.len(), 2);
        assert!((results[0].score - 1.0).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_local_count_and_clear() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rag.jsonl").to_string_lossy().to_string();
        let store = LocalStore::new(path);

        store
            .upsert("col", &[make_doc("a", "hello")])
            .await
            .unwrap();
        assert_eq!(store.count("col").await.unwrap(), 1);

        store.clear("col").await.unwrap();
        assert_eq!(store.count("col").await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_local_min_score_filter() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rag.jsonl").to_string_lossy().to_string();
        let store = LocalStore::new(path);

        let docs = vec![(
            Document {
                id: "d1".to_string(),
                content: "test".to_string(),
                source: "codebase".to_string(),
                metadata: serde_json::json!({}),
            },
            vec![0.0f32, 1.0, 0.0], // orthogonal to query
        )];
        store.upsert("col", &docs).await.unwrap();

        let query = vec![1.0f32, 0.0, 0.0];
        // Orthogonal vectors → score ≈ 0, which is below the threshold of 0.5
        let results = store.search("col", &query, 5, 0.5).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_local_persistence() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rag.jsonl").to_string_lossy().to_string();

        {
            let store = LocalStore::new(path.clone());
            store
                .upsert("col", &[make_doc("id1", "hello world")])
                .await
                .unwrap();
        }

        // Reload from disk
        let store2 = LocalStore::new(path);
        assert_eq!(store2.count("col").await.unwrap(), 1);
    }
}
