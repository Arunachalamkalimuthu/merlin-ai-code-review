//! GitHub webhook handler and HMAC signature verification.

use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde::Deserialize;
use tracing::{info, warn};

use crate::platform::github::GitHubClient;
use crate::platform::PlatformClient;

use super::{dispatch_command, WebhookState};

// ── GitHub payload types ───────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(super) struct GitHubCommentEvent {
    pub(super) action: String,
    pub(super) issue: GitHubIssueRef,
    pub(super) comment: GitHubComment,
    pub(super) repository: GitHubRepo,
}

#[derive(Deserialize)]
pub(super) struct GitHubIssueRef {
    pub(super) number: u64,
    pub(super) pull_request: Option<serde_json::Value>, // present only if this is a PR
}

#[derive(Deserialize)]
pub(super) struct GitHubComment {
    pub(super) body: String,
    pub(super) user: GitHubUser,
}

#[derive(Deserialize)]
pub(super) struct GitHubUser {
    pub(super) login: String,
    #[serde(rename = "type")]
    pub(super) user_type: String,
}

#[derive(Deserialize)]
pub(super) struct GitHubRepo {
    pub(super) full_name: String,
}

// ── Handler ────────────────────────────────────────────────────────────────────

/// Axum handler for `POST /webhook/github`.
///
/// Verifies the HMAC-SHA256 signature (when a webhook secret is configured),
/// filters to `issue_comment` events on pull requests, and dispatches any
/// recognised slash command via `dispatch_command`.
pub async fn github_handler(
    State(state): State<Arc<WebhookState>>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // Verify HMAC signature if secret is configured
    if let Some(ref secret) = state.github_secret {
        let sig = headers
            .get("X-Hub-Signature-256")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !verify_github_signature(&body, secret, sig) {
            warn!("GitHub webhook signature verification failed");
            return StatusCode::UNAUTHORIZED;
        }
    }

    let event_type = headers
        .get("X-GitHub-Event")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // Only handle issue_comment events on PRs
    if event_type != "issue_comment" {
        return StatusCode::OK;
    }

    let event: GitHubCommentEvent = match serde_json::from_slice(&body) {
        Ok(e) => e,
        Err(e) => {
            warn!("Failed to parse GitHub event: {e}");
            return StatusCode::BAD_REQUEST;
        }
    };

    // Only handle PRs, not plain issues
    if event.issue.pull_request.is_none() {
        return StatusCode::OK;
    }

    // Ignore bot comments
    if event.comment.user.user_type == "Bot" || event.action != "created" {
        return StatusCode::OK;
    }

    let Some((command, arg)) = crate::tools::parse_command(&event.comment.body) else {
        return StatusCode::OK;
    };

    info!(
        "GitHub: @{} triggered {} on PR #{}",
        event.comment.user.login, command, event.issue.number
    );

    let Some(ref token) = state.github_token else {
        warn!("No GITHUB_TOKEN configured for webhook");
        return StatusCode::INTERNAL_SERVER_ERROR;
    };

    // We need the head SHA — fetch it lazily (use env fallback or a separate API call)
    let head_sha = std::env::var("GITHUB_SHA").unwrap_or_else(|_| "HEAD".to_string());
    let client = Arc::new(GitHubClient::new(
        token.clone(),
        event.repository.full_name,
        event.issue.number,
        head_sha,
    )) as Arc<dyn PlatformClient>;

    let ai = Arc::clone(&state.ai);
    let cmd = command.clone();
    tokio::spawn(async move {
        dispatch_command(&cmd, arg, ai, client).await;
    });

    StatusCode::OK
}

// ── HMAC signature verification ───────────────────────────────────────────────

/// Verify a GitHub HMAC-SHA256 webhook signature.
///
/// Returns `true` when `signature` (in `sha256=<hex>` format) matches the
/// HMAC computed from `body` using `secret`.
pub(super) fn verify_github_signature(body: &[u8], secret: &str, signature: &str) -> bool {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let sig_bytes = match signature.strip_prefix("sha256=") {
        Some(hex) => match hex::decode(hex) {
            Ok(b) => b,
            Err(_) => return false,
        },
        None => return false,
    };

    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key size");
    mac.update(body);
    mac.verify_slice(&sig_bytes).is_ok()
}
