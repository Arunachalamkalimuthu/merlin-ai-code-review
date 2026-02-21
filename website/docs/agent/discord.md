---
sidebar_position: 3
title: Discord Integration
---

# Discord Integration

Send review summaries to a Discord channel via a webhook.

## Setup

### 1. Create a Discord webhook

1. In Discord, go to your server → right-click the target channel → **Edit Channel**
2. Go to **Integrations** → **Webhooks** → **New Webhook**
3. Name it "Merlin", optionally set an avatar
4. Copy the **Webhook URL**

### 2. Configure Merlin

```toml title="merlin.toml"
[agent.discord]
enabled     = true
# webhook_url is read from DISCORD_WEBHOOK_URL env var
```

```bash
export DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/...
```

Or inline (not recommended — avoid committing secrets):

```toml
[agent.discord]
enabled      = true
webhook_url  = "https://discord.com/api/webhooks/..."
```

## Notification format

Merlin sends a Discord embed:

```
🔍 Merlin Code Review
PR #42 · Add user authentication · main ← feature/auth

📁 3 files changed  💬 7 comments
🔴 1 critical  🟡 3 medium  🔵 3 info

⚠️ Critical: SQL injection vulnerability in login handler (auth.go:87)

[View PR](https://github.com/org/repo/pull/42)
```

## Configuration options

```toml
[agent.discord]
enabled          = true
notify_on        = ["critical", "high"]   # severity filter
include_summary  = true                    # attach full summary
username         = "Merlin"               # bot display name
avatar_url       = ""                     # custom avatar URL
```

## Testing the integration

```bash
merlin agent test-notification --platform discord
```

This sends a sample embed to your configured channel.

## Role mentions

To ping a role when critical issues are found:

```toml
[agent.discord]
enabled              = true
mention_role_id      = "123456789012345678"   # Discord role ID
mention_on_severity  = "critical"
```

To get a role ID: enable Developer Mode in Discord → right-click the role → **Copy ID**.
