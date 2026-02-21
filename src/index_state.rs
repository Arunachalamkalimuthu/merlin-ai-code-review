//! Tracks when each repository's code index was last refreshed.
//!
//! Data is persisted to a JSONL file so the server survives restarts.
//! Each line in the file is one JSON-encoded [`IndexRecord`].

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

// ── Types ─────────────────────────────────────────────────────────────────────

/// Metadata recorded each time a repository is successfully indexed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexRecord {
    /// Repository identifier, e.g. `"owner/repo"` or a local path.
    pub repo: String,
    /// Unix epoch seconds at which indexing completed (as a string).
    pub indexed_at: String,
    /// Number of document chunks indexed.
    pub doc_count: usize,
    /// Who or what triggered the index: `"api"`, `"push"`, `"cli"`, etc.
    pub triggered_by: String,
}

// ── Store ─────────────────────────────────────────────────────────────────────

/// Thread-safe, JSONL-persisted store of index-freshness records.
pub struct IndexFreshnessStore {
    records: RwLock<HashMap<String, IndexRecord>>,
    path: String,
}

impl IndexFreshnessStore {
    /// Load existing records from `path` (creates an empty store if the file
    /// does not yet exist) and wrap in a new instance.
    pub fn new(path: &str) -> Self {
        let records = Self::load_from_file(path);
        IndexFreshnessStore {
            records: RwLock::new(records),
            path: path.to_string(),
        }
    }

    /// Look up the freshness record for `repo`.
    pub async fn get(&self, repo: &str) -> Option<IndexRecord> {
        self.records.read().await.get(repo).cloned()
    }

    /// Insert or replace the record for `repo` and persist to disk.
    pub async fn set(&self, repo: &str, record: IndexRecord) {
        {
            let mut guard = self.records.write().await;
            guard.insert(repo.to_string(), record);
        }
        self.persist().await;
    }

    /// Return all records (unordered).
    pub async fn list(&self) -> Vec<IndexRecord> {
        self.records.read().await.values().cloned().collect()
    }

    // ── private ───────────────────────────────────────────────────────────────

    fn load_from_file(path: &str) -> HashMap<String, IndexRecord> {
        let mut map = HashMap::new();
        let Ok(content) = std::fs::read_to_string(path) else {
            return map;
        };
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(record) = serde_json::from_str::<IndexRecord>(line) {
                map.insert(record.repo.clone(), record);
            }
        }
        map
    }

    async fn persist(&self) {
        let records = self.records.read().await;
        let mut content = String::new();
        for record in records.values() {
            if let Ok(line) = serde_json::to_string(record) {
                content.push_str(&line);
                content.push('\n');
            }
        }
        if let Err(e) = std::fs::write(&self.path, &content) {
            tracing::warn!("IndexFreshnessStore: failed to persist to {}: {e}", self.path);
        }
    }
}
