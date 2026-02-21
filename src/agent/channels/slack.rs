//! Slack bot channel — receives tasks via Slack Events API (HTTP POST webhook).
//!
//! ## Setup
//!
//! 1. Create a Slack app at <https://api.slack.com/apps>
//! 2. Enable "Event Subscriptions" → Request URL: `https://your-host/slack/events`
//! 3. Subscribe to: `app_mention`, `message.im`
//! 4. Add OAuth scopes: `chat:write`, `app_mentions:read`, `im:history`
//! 5. Install the app and copy the **Bot User OAuth Token** (`xoxb-...`)
//! 6. Set environment variables:
//!    - `SLACK_BOT_TOKEN`       — xoxb-... token
//!    - `SLACK_SIGNING_SECRET`  — Signing secret (for optional request verification)
//!
//! ## Usage
//!   merlin agent --channel slack --port 8090

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::agent::{AgentChannel, AgentTask};

// ── Slack API types ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct SlackPayload {
    #[serde(rename = "type")]
    payload_type: String,
    /// URL verification challenge (sent by Slack when registering the endpoint).
    challenge: Option<String>,
    event: Option<SlackEvent>,
}

#[derive(Debug, Deserialize)]
struct SlackEvent {
    #[serde(rename = "type")]
    event_type: String,
    text: Option<String>,
    user: Option<String>,
    channel: Option<String>,
    bot_id: Option<String>,
}

// ── Channel ────────────────────────────────────────────────────────────────────

/// Slack bot channel that listens for Slack Events API webhooks.
pub struct SlackChannel {
    bot_token: String,
    task_rx: mpsc::Receiver<(AgentTask, String)>, // task + channel_id for reply
    client: reqwest::Client,
}

impl SlackChannel {
    /// Create a new Slack channel and start the Axum webhook server on `port`.
    pub async fn new(port: u16) -> crate::error::Result<Self> {
        let bot_token = std::env::var("SLACK_BOT_TOKEN")
            .map_err(|_| crate::error::MerlinError::EnvVar("SLACK_BOT_TOKEN".to_string()))?;

        let (task_tx, task_rx) = mpsc::channel::<(AgentTask, String)>(64);

        let tx = Arc::new(task_tx);
        tokio::spawn(run_slack_server(port, tx));

        info!("Slack Events webhook listening on port {port}");
        Ok(Self {
            bot_token,
            task_rx,
            client: reqwest::Client::new(),
        })
    }

    /// Post a message to a Slack channel.
    pub async fn post_to_channel(&self, channel_id: &str, text: &str) {
        let body = serde_json::json!({ "channel": channel_id, "text": text });
        match self
            .client
            .post("https://slack.com/api/chat.postMessage")
            .header("Authorization", format!("Bearer {}", self.bot_token))
            .json(&body)
            .send()
            .await
        {
            Ok(_) => debug!("Posted message to Slack channel {channel_id}"),
            Err(e) => warn!("Failed to post to Slack: {e}"),
        }
    }
}

#[async_trait]
impl AgentChannel for SlackChannel {
    fn name(&self) -> &str {
        "slack"
    }

    async fn recv(&mut self) -> Option<AgentTask> {
        // We receive (task, channel_id) pairs; store the channel_id in thread_id
        let (mut task, channel_id) = self.task_rx.recv().await?;
        task.thread_id = Some(channel_id);
        Some(task)
    }

    async fn send(&self, response: &str) {
        debug!("Slack send (no channel context): {response}");
    }

    async fn send_to(&self, response: &str, thread_id: &str) {
        self.post_to_channel(thread_id, response).await;
    }
}

// ── Axum webhook server ───────────────────────────────────────────────────────

async fn run_slack_server(port: u16, task_tx: Arc<mpsc::Sender<(AgentTask, String)>>) {
    use axum::{routing::post, Router};

    let app = Router::new()
        .route("/slack/events", post(slack_events_handler))
        .with_state(task_tx);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    info!("Slack webhook server starting on {addr}");

    match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => {
            if let Err(e) = axum::serve(listener, app).await {
                warn!("Slack server error: {e}");
            }
        }
        Err(e) => warn!("Failed to bind Slack server on {addr}: {e}"),
    }
}

async fn slack_events_handler(
    axum::extract::State(tx): axum::extract::State<Arc<mpsc::Sender<(AgentTask, String)>>>,
    axum::Json(payload): axum::Json<SlackPayload>,
) -> axum::Json<serde_json::Value> {
    // Respond to Slack URL verification challenge
    if payload.payload_type == "url_verification" {
        if let Some(challenge) = payload.challenge {
            return axum::Json(serde_json::json!({ "challenge": challenge }));
        }
    }

    if let Some(event) = payload.event {
        // Ignore bot messages to avoid infinite loops
        if event.bot_id.is_some() {
            return axum::Json(serde_json::json!({ "ok": true }));
        }

        let is_relevant = matches!(event.event_type.as_str(), "app_mention" | "message");
        if !is_relevant {
            return axum::Json(serde_json::json!({ "ok": true }));
        }

        if let (Some(text), Some(channel)) = (event.text, event.channel) {
            // Strip bot mention tokens like `<@U12345>`
            let content = regex::Regex::new(r"<@[A-Z0-9]+>\s*")
                .map(|re| re.replace_all(&text, "").trim().to_string())
                .unwrap_or_else(|_| text.trim().to_string());

            if !content.is_empty() {
                let task = AgentTask {
                    content,
                    sender: event.user,
                    thread_id: None, // filled in by recv()
                };
                let _ = tx.send((task, channel)).await;
            }
        }
    }

    axum::Json(serde_json::json!({ "ok": true }))
}
