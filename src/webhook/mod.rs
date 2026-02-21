//! Webhook server — receive GitHub/GitLab PR comment events and dispatch slash commands.
//!
//! Start with: `merlin webhook --port 8080`
//! Configure your GitHub webhook to send `issue_comment` events to `http://host:8080/webhook/github`
//! Configure your GitLab webhook to send `Note Hook` events to `http://host:8080/webhook/gitlab`

use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
    Router,
};
use serde::Deserialize;
use tracing::{info, warn};

use crate::ai::AiProvider;
use crate::platform::{github::GitHubClient, gitlab::GitLabClient, PlatformClient};
use crate::tools::{parse_command, route_command, ToolContext};

/// Shared state for the webhook server.
pub struct WebhookState {
    pub ai: Arc<dyn AiProvider>,
    pub github_secret: Option<String>,
    pub gitlab_secret: Option<String>,
    pub github_token: Option<String>,
    pub gitlab_token: Option<String>,
}

pub async fn serve(state: Arc<WebhookState>, port: u16) {
    let app = Router::new()
        .route("/webhook/github", post(github_handler))
        .route("/webhook/gitlab", post(gitlab_handler))
        .route("/health", axum::routing::get(health))
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");
    info!("Merlin webhook server listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> &'static str {
    "OK"
}

// ── GitHub webhook ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct GitHubCommentEvent {
    action: String,
    issue: GitHubIssueRef,
    comment: GitHubComment,
    repository: GitHubRepo,
}

#[derive(Deserialize)]
struct GitHubIssueRef {
    number: u64,
    pull_request: Option<serde_json::Value>, // present only if this is a PR
}

#[derive(Deserialize)]
struct GitHubComment {
    body: String,
    user: GitHubUser,
}

#[derive(Deserialize)]
struct GitHubUser {
    login: String,
    #[serde(rename = "type")]
    user_type: String,
}

#[derive(Deserialize)]
struct GitHubRepo {
    full_name: String,
}

async fn github_handler(
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

    let Some((command, arg)) = parse_command(&event.comment.body) else {
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

// ── GitLab webhook ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct GitLabNoteEvent {
    object_kind: String,
    project: GitLabProject,
    merge_request: Option<GitLabMrRef>,
    object_attributes: GitLabNote,
    user: GitLabUser,
}

#[derive(Deserialize)]
struct GitLabProject {
    id: u64,
}

#[derive(Deserialize)]
struct GitLabMrRef {
    iid: u64,
    last_commit: GitLabCommit,
}

#[derive(Deserialize)]
struct GitLabCommit {
    id: String,
}

#[derive(Deserialize)]
struct GitLabNote {
    note: String,
}

#[derive(Deserialize)]
struct GitLabUser {
    username: String,
    bot: Option<bool>,
}

async fn gitlab_handler(
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

    let Some((command, arg)) = parse_command(&event.object_attributes.note) else {
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
        mr.last_commit.id.clone(),
    )) as Arc<dyn PlatformClient>;

    let ai = Arc::clone(&state.ai);
    let cmd = command.clone();
    tokio::spawn(async move {
        dispatch_command(&cmd, arg, ai, client).await;
    });

    StatusCode::OK
}

// ── Common dispatch ───────────────────────────────────────────────────────────

async fn dispatch_command(
    command: &str,
    arg: Option<String>,
    ai: Arc<dyn AiProvider>,
    platform: Arc<dyn PlatformClient>,
) {
    let tool = match route_command(command) {
        Ok(t) => t,
        Err(e) => {
            let msg = format!("**Merlin:** Unknown command `{command}`. {e}");
            let _ = platform.post_summary(&msg).await;
            return;
        }
    };

    let ctx = ToolContext {
        ai,
        platform: Arc::clone(&platform),
        arg,
    };

    match tool.run(&ctx).await {
        Ok(result) => {
            let _ = platform.post_summary(&result).await;
        }
        Err(e) => {
            let msg = format!("**Merlin error running `{command}`:** {e}");
            warn!("{msg}");
            let _ = platform.post_summary(&msg).await;
        }
    }
}

// ── HMAC signature verification ───────────────────────────────────────────────

fn verify_github_signature(body: &[u8], secret: &str, signature: &str) -> bool {
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
