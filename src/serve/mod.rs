//! REST API server for third-party integrations.
//!
//! Start with: `merlin serve [--port 3000]`
//!
//! ## Endpoints
//!
//! | Method | Path                | Auth | Description                          |
//! |--------|---------------------|------|--------------------------------------|
//! | GET    | `/health`           | No   | Liveness probe                       |
//! | POST   | `/v1/index`         | Yes  | Trigger background RAG indexing      |
//! | POST   | `/v1/search`        | Yes  | Query the RAG index                  |
//! | POST   | `/v1/review`        | Yes  | Review a diff, returns JSON comments |
//! | GET    | `/v1/index/status`  | Yes  | Index freshness for all repos        |
//!
//! ## Authentication
//!
//! Set `[serve] api_key` in merlin.toml **or** export `MERLIN_API_KEY`.
//! Pass the key as the `X-Merlin-Api-Key` request header.

pub mod auth;
pub mod types;

use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    extract::{Json as AxumJson, State},
    http::StatusCode,
    middleware,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use tracing::{info, warn};

use crate::ai::AiProvider;
use crate::config::Config;
use crate::index_state::{IndexFreshnessStore, IndexRecord};
use crate::platform::NoOpPlatform;
use crate::rag::RagPipeline;
use crate::review::ReviewEngine;

use self::types::{IndexRequest, ReviewRequest, SearchRequest, SearchResult};

// ── Shared state ─────────────────────────────────────────────────────────────

pub struct ServeState {
    pub config: Arc<Config>,
    pub ai: Arc<dyn AiProvider>,
    pub rag: Arc<RagPipeline>,
    pub index_store: Arc<IndexFreshnessStore>,
}

// ── Server entry point ────────────────────────────────────────────────────────

pub async fn serve(state: Arc<ServeState>, port: u16) {
    // Routes that require API-key auth
    let protected = Router::new()
        .route("/v1/index", post(index_handler))
        .route("/v1/search", post(search_handler))
        .route("/v1/review", post(review_handler))
        .route("/v1/index/status", get(index_status_handler))
        .route_layer(middleware::from_fn_with_state(
            Arc::clone(&state),
            auth::require_api_key,
        ));

    let app = Router::new()
        .route("/health", get(health_handler))
        .merge(protected)
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");
    info!("Merlin API server listening on {addr}");
    info!("  GET  /health");
    info!("  POST /v1/index        (X-Merlin-Api-Key required)");
    info!("  POST /v1/search       (X-Merlin-Api-Key required)");
    info!("  POST /v1/review       (X-Merlin-Api-Key required)");
    info!("  GET  /v1/index/status (X-Merlin-Api-Key required)");

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// ── Handlers ─────────────────────────────────────────────────────────────────

async fn health_handler() -> AxumJson<serde_json::Value> {
    AxumJson(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// POST /v1/index — queue a background RAG index run for `root`.
async fn index_handler(
    State(state): State<Arc<ServeState>>,
    AxumJson(req): AxumJson<IndexRequest>,
) -> impl IntoResponse {
    let root = PathBuf::from(&req.root);
    if !root.exists() {
        return (
            StatusCode::BAD_REQUEST,
            AxumJson(serde_json::json!({
                "error": format!("Directory not found: {}", req.root)
            })),
        )
            .into_response();
    }

    let rag = Arc::clone(&state.rag);
    let config = Arc::clone(&state.config);
    let store = Arc::clone(&state.index_store);
    let repo = req.repo.clone();
    let triggered_by = req.triggered_by.unwrap_or_else(|| "api".to_string());

    tokio::spawn(async move {
        match crate::rag::indexer::index_directory(&rag, &root, &config.rag).await {
            Ok(count) => {
                let record = IndexRecord {
                    repo: repo.clone(),
                    indexed_at: unix_now_str(),
                    doc_count: count,
                    triggered_by,
                };
                store.set(&repo, record).await;
                info!("Indexed {count} chunks for repo '{repo}'");
            }
            Err(e) => {
                warn!("Index failed for repo '{repo}': {e}");
            }
        }
    });

    (
        StatusCode::ACCEPTED,
        AxumJson(serde_json::json!({
            "status": "accepted",
            "repo": req.repo,
            "message": "Indexing started in the background. \
                        Poll GET /v1/index/status to check progress."
        })),
    )
        .into_response()
}

/// POST /v1/search — query the RAG vector index.
async fn search_handler(
    State(state): State<Arc<ServeState>>,
    AxumJson(req): AxumJson<SearchRequest>,
) -> impl IntoResponse {
    match state.rag.retrieve(&req.query, req.limit).await {
        Ok(docs) => {
            let results: Vec<SearchResult> = docs
                .into_iter()
                .map(|d| SearchResult { content: d.content, source: d.source, score: d.score })
                .collect();
            let count = results.len();
            (
                StatusCode::OK,
                AxumJson(serde_json::json!({ "results": results, "count": count })),
            )
                .into_response()
        }
        Err(e) => {
            warn!("Search failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                AxumJson(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

/// POST /v1/review — run AI code review on a raw diff, return JSON comments.
async fn review_handler(
    State(state): State<Arc<ServeState>>,
    AxumJson(req): AxumJson<ReviewRequest>,
) -> impl IntoResponse {
    let ai = Arc::clone(&state.ai);
    let platform = Arc::new(NoOpPlatform) as Arc<dyn crate::platform::PlatformClient>;
    let engine = ReviewEngine::new(ai, platform, state.config.review.clone());

    match engine.run_local(&req.diff).await {
        Ok(comments) => {
            let total = comments.len();
            let comments_json: Vec<serde_json::Value> = comments
                .iter()
                .map(|c| serde_json::to_value(c).unwrap_or_default())
                .collect();
            (
                StatusCode::OK,
                AxumJson(serde_json::json!({ "comments": comments_json, "total": total })),
            )
                .into_response()
        }
        Err(e) => {
            warn!("Review failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                AxumJson(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

/// GET /v1/index/status — return freshness metadata for all indexed repos.
async fn index_status_handler(
    State(state): State<Arc<ServeState>>,
) -> AxumJson<serde_json::Value> {
    let entries = state.index_store.list().await;
    let count = entries.len();
    AxumJson(serde_json::json!({ "entries": entries, "count": count }))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Return the current time as Unix epoch seconds (no external time crate needed).
fn unix_now_str() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}
