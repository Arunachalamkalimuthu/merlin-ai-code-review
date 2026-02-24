//! Incremental review diff-hash cache.
//!
//! On each review run the engine computes a SHA-256 fingerprint of every
//! file's diff hunks.  If the fingerprint matches what was stored on the
//! previous run the file is skipped — saving AI tokens and avoiding duplicate
//! noise on unchanged code.
//!
//! # Storage format
//!
//! A flat JSON object `{ "path/to/file.rs": "<sha256-hex>", … }` written to
//! [`ReviewCache::path`].  The file is created on first save and silently
//! ignored when missing.
use std::collections::HashMap;

use hex;
use sha2::{Digest, Sha256};

use crate::diff::Hunk;

/// File-level diff-hash cache for incremental reviews.
///
/// Load with [`ReviewCache::load`] at the start of the review cycle and save
/// with [`ReviewCache::save`] after posting comments.
pub struct ReviewCache {
    /// Path to the on-disk JSON cache file.
    path: String,
    /// In-memory entries: file path → SHA-256 hex of its diff hunks.
    entries: HashMap<String, String>,
}

impl ReviewCache {
    /// Load the cache from `path`, or return an empty cache if the file does
    /// not exist or cannot be parsed.
    pub fn load(path: &str) -> Self {
        let entries: HashMap<String, String> = std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Self {
            path: path.to_string(),
            entries,
        }
    }

    /// Returns `true` when `key` is already cached with the same `hash`,
    /// meaning the file is unchanged since the last review run.
    pub fn is_fresh(&self, key: &str, hash: &str) -> bool {
        self.entries.get(key).is_some_and(|h| h == hash)
    }

    /// Insert or update the hash for `key`.
    pub fn update(&mut self, key: &str, hash: String) {
        self.entries.insert(key.to_string(), hash);
    }

    /// Persist the cache to disk.  Write errors are logged as warnings and
    /// silently swallowed so a cache miss is always safe.
    pub fn save(&self) {
        match serde_json::to_string_pretty(&self.entries) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&self.path, json) {
                    tracing::warn!("Could not write review cache to {}: {e}", self.path);
                }
            }
            Err(e) => tracing::warn!("Could not serialise review cache: {e}"),
        }
    }
}

/// Compute a stable SHA-256 hex fingerprint of a file's diff hunks.
///
/// The hash covers only added/removed/context line *content* — not line
/// numbers — so a pure rebase that shifts line numbers does not invalidate
/// the cache.
pub fn diff_hash(hunks: &[Hunk]) -> String {
    let mut hasher = Sha256::new();
    for hunk in hunks {
        for line in &hunk.lines {
            hasher.update(line.content.as_bytes());
        }
    }
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_hunks_produce_stable_hash() {
        let h1 = diff_hash(&[]);
        let h2 = diff_hash(&[]);
        assert_eq!(h1, h2);
    }

    #[test]
    fn different_content_produces_different_hash() {
        use crate::diff::{HunkLine, LineKind};
        let make_hunk = |content: &str| Hunk {
            old_start: 1,
            old_count: 1,
            new_start: 1,
            new_count: 1,
            header_suffix: String::new(),
            lines: vec![HunkLine {
                kind: LineKind::Added,
                content: content.to_string(),
                new_line: Some(1),
                old_line: None,
            }],
        };
        let h1 = diff_hash(&[make_hunk("foo")]);
        let h2 = diff_hash(&[make_hunk("bar")]);
        assert_ne!(h1, h2);
    }

    #[test]
    fn cache_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cache.json").to_string_lossy().into_owned();

        let mut cache = ReviewCache::load(&path);
        assert!(!cache.is_fresh("file.rs", "abc"));
        cache.update("file.rs", "abc".to_string());
        assert!(cache.is_fresh("file.rs", "abc"));
        assert!(!cache.is_fresh("file.rs", "xyz"));
        cache.save();

        // Reload from disk
        let cache2 = ReviewCache::load(&path);
        assert!(cache2.is_fresh("file.rs", "abc"));
    }
}
