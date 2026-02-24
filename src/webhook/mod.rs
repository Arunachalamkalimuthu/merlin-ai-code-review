//! Webhook server — receive GitHub/GitLab PR comment events and dispatch slash commands.
//!
//! Start with: `merlin webhook --port 8080`
//!
//! Configure your GitHub webhook to send `issue_comment` events to
//! `http://host:8080/webhook/github` and your GitLab webhook to send
//! `Note Hook` events to `http://host:8080/webhook/gitlab`.
//!
//! # Sub-modules
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`github`] | GitHub payload parsing, HMAC verification, and handler |
//! | [`gitlab`] | GitLab payload parsing, token verification, and handler |

pub mod github;
pub mod gitlab;

use std::sync::Arc;

use axum::{routing::post, Router};
use tracing::{info, warn};

use crate::ai::AiProvider;
use crate::platform::PlatformClient;
use crate::tools::{route_command, ToolContext};

pub use github::github_handler;
pub use gitlab::gitlab_handler;

/// Shared state injected into every webhook handler by Axum.
pub struct WebhookState {
    /// The AI backend used to process slash-command requests.
    pub ai: Arc<dyn AiProvider>,
    /// Optional HMAC secret for verifying GitHub webhook signatures.
    pub github_secret: Option<String>,
    /// Optional token for verifying GitLab webhook headers.
    pub gitlab_secret: Option<String>,
    /// GitHub token used to post comments back to PRs.
    pub github_token: Option<String>,
    /// GitLab token used to post notes back to MRs.
    pub gitlab_token: Option<String>,
}

/// Start the Axum webhook server on the given port.
///
/// Routes:
/// - `POST /webhook/github` → [`github_handler`]
/// - `POST /webhook/gitlab` → [`gitlab_handler`]
/// - `GET  /health`         → `"OK"`
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

// ── Shared command dispatch ────────────────────────────────────────────────────

/// Route and execute a slash command, posting the result (or error) back to the platform.
///
/// Called from both the GitHub and GitLab handlers after verifying the request
/// and extracting the command name and optional argument.
pub(super) async fn dispatch_command(
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
