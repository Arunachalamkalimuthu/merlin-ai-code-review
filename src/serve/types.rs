//! Request and response DTOs for the Merlin REST API.

use serde::{Deserialize, Serialize};

// ── /v1/index ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct IndexRequest {
    /// Repository identifier (e.g. `"owner/repo"` or a local label).
    pub repo: String,
    /// Filesystem path to the directory that should be indexed.
    pub root: String,
    /// Optional: who triggered this index call (defaults to `"api"`).
    pub triggered_by: Option<String>,
}

// ── /v1/search ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    /// Natural-language query.
    pub query: String,
    /// Maximum number of results (default: 5).
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize { 5 }

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub content: String,
    pub source: String,
    pub score: f32,
}

// ── /v1/review ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ReviewRequest {
    /// Raw unified diff text to review.
    pub diff: String,
}

// ── Error wrapper ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}
