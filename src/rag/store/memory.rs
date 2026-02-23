//! Ephemeral in-memory vector store.
//!
//! Resets on restart. Useful for testing and one-shot agent tasks.

use std::collections::HashMap;
use std::sync::RwLock;

use async_trait::async_trait;

use super::cosine_similarity;
use crate::error::{MerlinError, Result};
use crate::rag::{Document, Embedding, RetrievedDoc, VectorStore};

struct Entry {
    id: String,
    vector: Vec<f32>,
    content: String,
    source: String,
    metadata: serde_json::Value,
}

/// Ephemeral in-memory vector store — resets when the process exits.
pub struct MemoryStore {
    index: RwLock<HashMap<String, Vec<Entry>>>,
}

impl MemoryStore {
    /// Create a new empty in-memory store.
    pub fn new() -> Self {
        Self {
            index: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VectorStore for MemoryStore {
    async fn ensure_collection(&self, _collection: &str, _dimension: usize) -> Result<()> {
        Ok(())
    }

    async fn upsert(&self, collection: &str, docs: &[(Document, Embedding)]) -> Result<()> {
        let mut index = self
            .index
            .write()
            .map_err(|_| MerlinError::Other("MemoryStore: lock poisoned".to_string()))?;
        let entries = index.entry(collection.to_string()).or_default();
        for (doc, emb) in docs {
            entries.retain(|e| e.id != doc.id);
            entries.push(Entry {
                id: doc.id.clone(),
                vector: emb.clone(),
                content: doc.content.clone(),
                source: doc.source.clone(),
                metadata: doc.metadata.clone(),
            });
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
        let index = self
            .index
            .read()
            .map_err(|_| MerlinError::Other("MemoryStore: lock poisoned".to_string()))?;

        let Some(entries) = index.get(collection) else {
            return Ok(vec![]);
        };

        let mut scored: Vec<(f32, &Entry)> = entries
            .iter()
            .filter_map(|e| {
                if e.vector.len() != query_vec.len() {
                    return None;
                }
                let score = cosine_similarity(&e.vector, query_vec);
                (score >= min_score).then_some((score, e))
            })
            .collect();

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
        let mut index = self
            .index
            .write()
            .map_err(|_| MerlinError::Other("MemoryStore: lock poisoned".to_string()))?;
        index.remove(collection);
        Ok(())
    }

    async fn count(&self, collection: &str) -> Result<usize> {
        let index = self
            .index
            .read()
            .map_err(|_| MerlinError::Other("MemoryStore: lock poisoned".to_string()))?;
        Ok(index.get(collection).map(|e| e.len()).unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rag::Document;

    #[tokio::test]
    async fn test_memory_roundtrip() {
        let store = MemoryStore::new();
        let doc = Document {
            id: "a".to_string(),
            content: "hello".to_string(),
            source: "codebase".to_string(),
            metadata: serde_json::json!({}),
        };
        let emb = vec![1.0f32, 0.0];
        store.upsert("col", &[(doc, emb.clone())]).await.unwrap();

        let results = store.search("col", &emb, 1, 0.0).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "hello");
    }

    #[tokio::test]
    async fn test_memory_clear() {
        let store = MemoryStore::new();
        let doc = Document {
            id: "a".to_string(),
            content: "hi".to_string(),
            source: "codebase".to_string(),
            metadata: serde_json::json!({}),
        };
        store.upsert("col", &[(doc, vec![1.0f32])]).await.unwrap();
        assert_eq!(store.count("col").await.unwrap(), 1);
        store.clear("col").await.unwrap();
        assert_eq!(store.count("col").await.unwrap(), 0);
    }
}
