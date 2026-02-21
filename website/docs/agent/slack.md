---
sidebar_position: 2
title: Slack Integration
---

# Slack Integration

Send review summaries and notifications to a Slack channel when Merlin finishes a review.

## Setup

### 1. Create a Slack app

1. Go to [api.slack.com/apps](https://api.slack.com/apps) → **Create New App** → **From scratch**
2. Name it "Merlin" and select your workspace
3. Go to **OAuth & Permissions** → add `chat:write` scope
4. Install the app to your workspace
5. Copy the **Bot User OAuth Token** (`xoxb-...`)

### 2. Configure Merlin

```toml title="merlin.toml"
[agent.slack]
enabled     = true
channel     = "#code-review"    # channel name or ID
# token is read from SLACK_BOT_TOKEN env var
```

```bash
export SLACK_BOT_TOKEN=xoxb-...
```

Or store in `.env`:

```bash
SLACK_BOT_TOKEN=xoxb-...
```

### 3. Invite the bot to the channel

In Slack, in your target channel:
```
/invite @Merlin
```

## Notification format

When a review completes, Merlin posts:

```
🔍 Merlin reviewed PR #42: Add user authentication
📁 3 files changed | 💬 7 comments | 🔴 1 critical, 🟡 3 medium, 🔵 3 info

Critical: SQL injection vulnerability in login handler (auth.go:87)
View PR → https://github.com/org/repo/pull/42
```

## Configuration options

```toml
[agent.slack]
enabled          = true
channel          = "#code-review"
notify_on        = ["critical", "high"]   # only notify for these severities
include_summary  = true                    # include full review summary
thread_replies   = true                    # post comments as thread replies
mention_author   = false                   # @mention the PR author
```

### `notify_on` severities

| Value | Description |
|---|---|
| `"critical"` | Only critical issues |
| `"high"` | High and above |
| `"medium"` | Medium and above (default) |
| `"all"` | All comments including info |

## Incoming webhook (simpler alternative)

If you don't need interactive features, use an incoming webhook instead:

1. In your Slack app, go to **Incoming Webhooks** → enable → **Add New Webhook to Workspace**
2. Copy the webhook URL

```toml
[agent.slack]
enabled      = true
webhook_url  = "https://hooks.slack.com/services/T.../B.../..."
```

```bash
export SLACK_WEBHOOK_URL=https://hooks.slack.com/services/...
```

No bot token required.

## Testing the integration

```bash
merlin agent test-notification --platform slack
```

This sends a sample notification to your configured channel.
