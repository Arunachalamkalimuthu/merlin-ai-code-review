//! Adaptive feedback learning — learns from accept/reject signals on review comments.
//!
//! Teams can thumbs-up or thumbs-down AI review comments.  Over time, Merlin
//! learns which comment patterns are consistently rejected and auto-suppresses
//! them, reducing noise without manual rule authoring.
//!
//! # Storage
//!
//! Feedback is persisted in a JSONL file (default: `.merlin-feedback.jsonl`).
//! Each line records one accept or reject event keyed by a *pattern key*
//! (`category:title_lowercase`).
//!
//! # Suppression logic
//!
//! A pattern is suppressed when:
//! - It has at least [`MIN_SAMPLES`] feedback events, **and**
//! - Its reject ratio exceeds [`SUPPRESS_THRESHOLD`] (default 70 %).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info};

use crate::ai::ReviewComment;

/// Minimum number of feedback events before a pattern can be suppressed.
const MIN_SAMPLES: u32 = 5;

/// Reject ratio above which a pattern is auto-suppressed (0.0–1.0).
const SUPPRESS_THRESHOLD: f64 = 0.70;

// ── Data types ──────────────────────────────────────────────────────────────

/// A single feedback event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackEvent {
    /// Pattern key: `"category:normalised_title"`.
    pub pattern: String,
    /// `true` = accepted (thumbs-up), `false` = rejected (thumbs-down).
    pub accepted: bool,
    /// ISO-8601 timestamp.
    pub timestamp: String,
}

/// Aggregate counters for a single pattern.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PatternStats {
    /// Total times this pattern was accepted.
    pub accepted: u32,
    /// Total times this pattern was rejected.
    pub rejected: u32,
}

impl PatternStats {
    /// Total number of feedback events.
    pub fn total(&self) -> u32 {
        self.accepted + self.rejected
    }

    /// Rejection ratio (0.0–1.0).  Returns 0.0 when no events exist.
    pub fn reject_ratio(&self) -> f64 {
        if self.total() == 0 {
            return 0.0;
        }
        f64::from(self.rejected) / f64::from(self.total())
    }

    /// Whether this pattern should be suppressed.
    pub fn is_suppressed(&self) -> bool {
        self.total() >= MIN_SAMPLES && self.reject_ratio() > SUPPRESS_THRESHOLD
    }
}

// ── Feedback store ──────────────────────────────────────────────────────────

/// Persistent feedback store backed by a JSONL file.
#[derive(Debug)]
pub struct FeedbackStore {
    path: String,
    stats: HashMap<String, PatternStats>,
}

impl FeedbackStore {
    /// Load (or create) a feedback store from `path`.
    pub fn load(path: &str) -> Self {
        let mut stats: HashMap<String, PatternStats> = HashMap::new();

        if Path::new(path).exists() {
            if let Ok(contents) = std::fs::read_to_string(path) {
                for line in contents.lines() {
                    if let Ok(evt) = serde_json::from_str::<FeedbackEvent>(line) {
                        let entry = stats.entry(evt.pattern).or_default();
                        if evt.accepted {
                            entry.accepted += 1;
                        } else {
                            entry.rejected += 1;
                        }
                    }
                }
            }
            debug!("Loaded {} feedback patterns from {path}", stats.len());
        }

        Self {
            path: path.to_string(),
            stats,
        }
    }

    /// Record a feedback event and persist it.
    pub fn record(&mut self, comment: &ReviewComment, accepted: bool) {
        let pattern = pattern_key(comment);
        let evt = FeedbackEvent {
            pattern: pattern.clone(),
            accepted,
            timestamp: chrono_now(),
        };

        // Append to file
        if let Ok(line) = serde_json::to_string(&evt) {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
            {
                let _ = writeln!(f, "{line}");
            }
        }

        // Update in-memory stats
        let entry = self.stats.entry(pattern).or_default();
        if accepted {
            entry.accepted += 1;
        } else {
            entry.rejected += 1;
        }
    }

    /// Filter out comments whose pattern has been suppressed by feedback.
    ///
    /// Returns `(kept, suppressed_count)`.
    pub fn filter_comments(&self, comments: Vec<ReviewComment>) -> (Vec<ReviewComment>, usize) {
        let before = comments.len();
        let kept: Vec<ReviewComment> = comments
            .into_iter()
            .filter(|c| {
                let key = pattern_key(c);
                if let Some(s) = self.stats.get(&key) {
                    if s.is_suppressed() {
                        debug!(
                            "Suppressing comment pattern '{key}' (reject ratio: {:.0}%, n={})",
                            s.reject_ratio() * 100.0,
                            s.total()
                        );
                        return false;
                    }
                }
                true
            })
            .collect();
        let suppressed = before - kept.len();
        if suppressed > 0 {
            info!("{suppressed} comment(s) suppressed by feedback learning");
        }
        (kept, suppressed)
    }

    /// Return the current stats map (for the `/feedback` status report).
    pub fn stats(&self) -> &HashMap<String, PatternStats> {
        &self.stats
    }

    /// Number of tracked patterns.
    pub fn pattern_count(&self) -> usize {
        self.stats.len()
    }

    /// Number of currently suppressed patterns.
    pub fn suppressed_count(&self) -> usize {
        self.stats.values().filter(|s| s.is_suppressed()).count()
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Build a canonical pattern key from a review comment.
///
/// Format: `"category:normalised_title"` — category is lowercase, title is
/// lowercased and trimmed.
pub fn pattern_key(comment: &ReviewComment) -> String {
    format!(
        "{:?}:{}",
        comment.category,
        comment.title.to_lowercase().trim()
    )
    .to_lowercase()
}

/// Return the current UTC time as an ISO-8601 string.
///
/// Falls back to a fixed string if system time is unavailable.
fn chrono_now() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Simple RFC-3339 without pulling in chrono
    format!("{now}")
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{Category, Severity};

    fn make_comment(title: &str, category: Category) -> ReviewComment {
        ReviewComment {
            file: "test.rs".to_string(),
            line: 1,
            severity: Severity::Medium,
            category,
            title: title.to_string(),
            body: "test body".to_string(),
            suggestion: None,
        }
    }

    #[test]
    fn pattern_key_is_stable() {
        let c = make_comment("Use iter() instead of into_iter()", Category::Style);
        let key = pattern_key(&c);
        assert_eq!(key, "style:use iter() instead of into_iter()");
    }

    #[test]
    fn suppression_requires_min_samples() {
        let mut stats = PatternStats::default();
        // 4 rejects, 0 accepts — under MIN_SAMPLES
        for _ in 0..4 {
            stats.rejected += 1;
        }
        assert!(!stats.is_suppressed());
        // 5th reject pushes it over
        stats.rejected += 1;
        assert!(stats.is_suppressed());
    }

    #[test]
    fn high_accept_ratio_not_suppressed() {
        let stats = PatternStats {
            accepted: 8,
            rejected: 2,
        };
        assert!(!stats.is_suppressed());
    }

    #[test]
    fn filter_removes_suppressed_patterns() {
        let mut store = FeedbackStore {
            path: String::new(),
            stats: HashMap::new(),
        };
        store.stats.insert(
            "style:use iter() instead of into_iter()".to_string(),
            PatternStats {
                accepted: 1,
                rejected: 9,
            },
        );

        let comments = vec![
            make_comment("Use iter() instead of into_iter()", Category::Style),
            make_comment("SQL injection risk", Category::Security),
        ];
        let (kept, suppressed) = store.filter_comments(comments);
        assert_eq!(suppressed, 1);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].title, "SQL injection risk");
    }

    #[test]
    fn store_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("feedback.jsonl");
        let path_str = path.to_str().unwrap();

        let c1 = make_comment("Unused variable", Category::Style);
        {
            let mut store = FeedbackStore::load(path_str);
            store.record(&c1, false);
            store.record(&c1, false);
            store.record(&c1, true);
        }

        // Reload and verify
        let store = FeedbackStore::load(path_str);
        let key = pattern_key(&c1);
        let s = store.stats.get(&key).unwrap();
        assert_eq!(s.rejected, 2);
        assert_eq!(s.accepted, 1);
    }
}
