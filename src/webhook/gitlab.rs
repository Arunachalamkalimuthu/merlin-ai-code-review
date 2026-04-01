//! GitLab webhook handler and secret-token verification.

use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde::Deserialize;
use tracing::{info, warn};

use crate::platform::gitlab::GitLabClient;
use crate::platform::PlatformClient;

use super::{dispatch_command, WebhookState};

// ── GitLab payload types ───────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(super) struct GitLabNoteEvent {
    pub(super) object_kind: String,
    pub(super) project: GitLabProject,
    pub(super) merge_request: Option<GitLabMrRef>,
    pub(super) object_attributes: GitLabNote,
    pub(super) user: GitLabUser,
}

#[derive(Deserialize)]
pub(super) struct GitLabProject {
    pub(super) id: u64,
}

#[derive(Deserialize)]
pub(super) struct GitLabMrRef {
    pub(super) iid: u64,
}

#[derive(Deserialize)]
pub(super) struct GitLabNote {
    pub(super) note: String,
}

#[derive(Deserialize)]
pub(super) struct GitLabUser {
    pub(super) username: String,
    pub(super) bot: Option<bool>,
}

// ── Handler ────────────────────────────────────────────────────────────────────

/// Axum handler for `POST /webhook/gitlab`.
///
/// Verifies the `X-Gitlab-Token` header (when a webhook secret is configured),
/// filters to `note` events on merge requests, and dispatches any recognised
/// slash command via `dispatch_command`.
pub async fn gitlab_handler(
    State(state): State<Arc<WebhookState>>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // Verify GitLab token
    if let Some(ref secret) = state.gitlab_secret {
        let token = headers
            .get("X-Gitlab-Token")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if token != secret {
            warn!("GitLab webhook token mismatch");
            return StatusCode::UNAUTHORIZED;
        }
    }

    let event: GitLabNoteEvent = match serde_json::from_slice(&body) {
        Ok(e) => e,
        Err(e) => {
            warn!("Failed to parse GitLab event: {e}");
            return StatusCode::BAD_REQUEST;
        }
    };

    if event.object_kind != "note" {
        return StatusCode::OK;
    }

    // Only MR notes
    let Some(ref mr) = event.merge_request else {
        return StatusCode::OK;
    };

    if event.user.bot.unwrap_or(false) {
        return StatusCode::OK;
    }

    let Some((command, arg)) = crate::tools::parse_command(&event.object_attributes.note) else {
        return StatusCode::OK;
    };

    info!(
        "GitLab: @{} triggered {} on MR !{}",
        event.user.username, command, mr.iid
    );

    let Some(ref token) = state.gitlab_token else {
        warn!("No GITLAB_TOKEN configured for webhook");
        return StatusCode::INTERNAL_SERVER_ERROR;
    };

    let base_url =
        std::env::var("CI_API_V4_URL").unwrap_or_else(|_| "https://gitlab.com/api/v4".to_string());

    let client = Arc::new(GitLabClient::new(
        token.clone(),
        base_url,
        event.project.id.to_string(),
        mr.iid,
    )) as Arc<dyn PlatformClient>;

    let ai = Arc::clone(&state.ai);
    let cmd = command.clone();
    tokio::spawn(async move {
        dispatch_command(&cmd, arg, ai, client).await;
    });

    StatusCode::OK
}
