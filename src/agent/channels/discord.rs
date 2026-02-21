//! Discord bot channel — receives messages via Discord REST API polling.
//!
//! ## Setup
//!
//! 1. Create a bot at <https://discord.com/developers/applications>
//! 2. Bot → Reset Token → copy the token
//! 3. OAuth2 → URL Generator: select `bot` scope + `Send Messages` / `Read Message History`
//! 4. Set environment variables:
//!    - `DISCORD_BOT_TOKEN`   — bot token
//!    - `DISCORD_CHANNEL_IDS` — comma-separated channel IDs to monitor
//!
//! ## Message triggers
//!
//! The bot responds to messages that:
//! - Mention the bot (`@Ferret <task>`)
//! - Start with `ferret ` (case-insensitive)
//!
//! ## Usage
//!   ferret agent --channel discord

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::agent::{AgentChannel, AgentTask};

const DISCORD_API: &str = "https://discord.com/api/v10";

// ── Discord API types ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct DiscordMessage {
    id: String,
    content: String,
    author: DiscordUser,
    channel_id: String,
    #[serde(default)]
    mentions: Vec<DiscordUser>,
}

#[derive(Debug, Deserialize)]
struct DiscordUser {
    id: String,
    username: String,
    #[serde(default)]
    bot: bool,
}

// ── Channel ────────────────────────────────────────────────────────────────────

/// Discord bot channel using REST API polling (every 3 s).
pub struct DiscordChannel {
    token: String,
    task_rx: mpsc::Receiver<(AgentTask, String)>, // task + channel_id for reply
    client: reqwest::Client,
}

impl DiscordChannel {
    /// Create a new Discord channel and start the polling loop.
    pub async fn new() -> crate::error::Result<Self> {
        let token = std::env::var("DISCORD_BOT_TOKEN").map_err(|_| {
            crate::error::MerlinError::EnvVar("DISCORD_BOT_TOKEN".to_string())
        })?;

        let client = reqwest::Client::new();

        // Resolve the bot's own user ID (for mention detection)
        let bot_id = fetch_bot_id(&client, &token).await;
        info!("Discord bot user ID: {:?}", bot_id);

        let channel_ids: Vec<String> = std::env::var("DISCORD_CHANNEL_IDS")
            .unwrap_or_default()
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().to_string())
            .collect();

        if channel_ids.is_empty() {
            warn!(
                "DISCORD_CHANNEL_IDS not set — Discord channel will not receive any messages. \
                 Set it to a comma-separated list of channel IDs."
            );
        }

        let (task_tx, task_rx) = mpsc::channel::<(AgentTask, String)>(64);

        let tx = Arc::new(task_tx);
        let poll_client = client.clone();
        let poll_token = token.clone();
        tokio::spawn(poll_loop(poll_client, poll_token, bot_id, channel_ids, tx));

        Ok(Self { token, task_rx, client })
    }

    /// Post a message to a Discord channel.
    pub async fn post_to_channel(&self, channel_id: &str, content: &str) {
        // Discord has a 2000 char limit per message — truncate if needed
        let truncated: String = if content.len() > 1990 {
            format!("{}… *(truncated)*", &content[..1990])
        } else {
            content.to_string()
        };

        let body = serde_json::json!({ "content": truncated });
        match self
            .client
            .post(format!("{DISCORD_API}/channels/{channel_id}/messages"))
            .header("Authorization", format!("Bot {}", self.token))
            .json(&body)
            .send()
            .await
        {
            Ok(_) => debug!("Posted message to Discord channel {channel_id}"),
            Err(e) => warn!("Failed to post to Discord: {e}"),
        }
    }
}

#[async_trait]
impl AgentChannel for DiscordChannel {
    fn name(&self) -> &str {
        "discord"
    }

    async fn recv(&mut self) -> Option<AgentTask> {
        let (mut task, channel_id) = self.task_rx.recv().await?;
        task.thread_id = Some(channel_id);
        Some(task)
    }

    async fn send(&self, response: &str) {
        debug!("Discord send (no channel context): {response}");
    }

    async fn send_to(&self, response: &str, thread_id: &str) {
        self.post_to_channel(thread_id, response).await;
    }
}

// ── Polling loop ──────────────────────────────────────────────────────────────

async fn poll_loop(
    client: reqwest::Client,
    token: String,
    bot_id: Option<String>,
    channel_ids: Vec<String>,
    task_tx: Arc<mpsc::Sender<(AgentTask, String)>>,
) {
    let mut last_ids: HashMap<String, String> = HashMap::new();

    loop {
        for channel_id in &channel_ids {
            let messages =
                fetch_new_messages(&client, &token, channel_id, last_ids.get(channel_id)).await;

            for msg in messages {
                // Skip bot messages
                if msg.author.bot {
                    last_ids.insert(channel_id.clone(), msg.id.clone());
                    continue;
                }

                let is_mention = bot_id
                    .as_ref()
                    .map(|id| msg.mentions.iter().any(|u| &u.id == id))
                    .unwrap_or(false);

                let lower = msg.content.to_lowercase();
                let is_triggered = is_mention || lower.starts_with("ferret ");

                if is_triggered {
                    let content = strip_mention(&msg.content, bot_id.as_deref())
                        .trim_start_matches("ferret ")
                        .trim()
                        .to_string();

                    if !content.is_empty() {
                        let task = AgentTask {
                            content,
                            sender: Some(msg.author.username.clone()),
                            thread_id: None, // filled in by recv()
                        };
                        let _ = task_tx.send((task, msg.channel_id.clone())).await;
                    }
                }

                last_ids.insert(channel_id.clone(), msg.id.clone());
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    }
}

/// Fetch messages newer than `after_id` from a channel (oldest-first).
async fn fetch_new_messages(
    client: &reqwest::Client,
    token: &str,
    channel_id: &str,
    after_id: Option<&String>,
) -> Vec<DiscordMessage> {
    let mut url = format!("{DISCORD_API}/channels/{channel_id}/messages?limit=20");
    if let Some(id) = after_id {
        url.push_str(&format!("&after={id}"));
    }

    let resp = match client
        .get(&url)
        .header("Authorization", format!("Bot {token}"))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!("Discord poll error for channel {channel_id}: {e}");
            return vec![];
        }
    };

    let mut messages: Vec<DiscordMessage> = match resp.json().await {
        Ok(m) => m,
        Err(e) => {
            debug!("Discord JSON parse error: {e}");
            return vec![];
        }
    };

    // Discord returns newest-first; reverse for chronological processing
    messages.reverse();
    messages
}

/// Resolve the bot's own Discord user ID.
async fn fetch_bot_id(client: &reqwest::Client, token: &str) -> Option<String> {
    let resp = client
        .get(format!("{DISCORD_API}/users/@me"))
        .header("Authorization", format!("Bot {token}"))
        .send()
        .await
        .ok()?;

    let user: serde_json::Value = resp.json().await.ok()?;
    user["id"].as_str().map(str::to_string)
}

/// Strip `<@BOT_ID>` mention tokens from a message.
fn strip_mention(text: &str, bot_id: Option<&str>) -> String {
    if let Some(id) = bot_id {
        let pattern = format!(r"<@!?{id}>\s*");
        if let Ok(re) = regex::Regex::new(&pattern) {
            return re.replace_all(text, "").to_string();
        }
    }
    text.to_string()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_mention_with_id() {
        let text = "<@U12345678> review this";
        let stripped = strip_mention(text, Some("U12345678"));
        assert_eq!(stripped.trim(), "review this");
    }

    #[test]
    fn test_strip_mention_no_id() {
        let text = "ferret review please";
        let stripped = strip_mention(text, None);
        assert_eq!(stripped, text);
    }
}
