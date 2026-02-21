//! API-key authentication middleware for the Merlin REST API.
//!
//! Protected routes require an `X-Merlin-Api-Key` header whose value matches
//! either `[serve] api_key` in merlin.toml **or** the `MERLIN_API_KEY`
//! environment variable.  If neither is configured, all requests are allowed
//! through (useful for local / internal deployments).

use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};

use super::{types::ErrorResponse, ServeState};

/// Axum middleware: validates the `X-Merlin-Api-Key` header.
pub async fn require_api_key(
    State(state): State<Arc<ServeState>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let expected = state
        .config
        .serve
        .api_key
        .clone()
        .or_else(|| std::env::var("MERLIN_API_KEY").ok());

    if let Some(expected_key) = expected {
        let provided = req
            .headers()
            .get("X-Merlin-Api-Key")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if provided != expected_key {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Invalid or missing API key. \
                            Provide the X-Merlin-Api-Key header."
                        .to_string(),
                }),
            )
                .into_response();
        }
    }

    next.run(req).await
}
