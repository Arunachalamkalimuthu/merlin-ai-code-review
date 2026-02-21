---
sidebar_position: 1
title: Overview
---

# Agent Mode

Agent mode turns Merlin into an autonomous code reviewer that watches your repository and responds to developer commands posted as PR comments — no manual CI step required.

## How it works

```
Developer posts: @merlin review
        │
        ▼
Merlin webhook listener receives the event
        │
        ▼
Fetch diff → RAG context → AI review
        │
        ▼
Post inline comments + summary to PR
```

The agent runs as a long-lived process (or as a Docker container) inside your infrastructure and registers a webhook with GitHub or GitLab.

## Starting the agent

```bash
merlin agent start
```

Merlin registers a webhook on your repo and begins listening for events.

### Required environment variables

| Variable | Description |
|---|---|
| `ANTHROPIC_API_KEY` | AI provider key |
| `GITHUB_TOKEN` | Personal access token with `repo` + `write:org` scopes |
| `MERLIN_WEBHOOK_SECRET` | Random string used to verify payloads |

Generate a webhook secret:
```bash
openssl rand -hex 32
```

## Configuration

```toml title="merlin.toml"
[agent]
enabled        = true
port           = 9000           # webhook listener port
webhook_secret = ""             # or set MERLIN_WEBHOOK_SECRET env var
auto_review    = true           # review every new PR automatically
review_on_push = false          # re-review on every push to open PRs

[agent.triggers]
# Respond to these comment commands
commands = ["@merlin review", "@merlin security", "@merlin improve"]
```

## Trigger commands

| Comment | Action |
|---|---|
| `@merlin review` | Full review of the PR diff |
| `@merlin security` | Security-focused review |
| `@merlin improve` | Suggest improvements and refactors |
| `@merlin describe` | Update PR description |
| `@merlin ask <question>` | Answer a question about the PR |
| `@merlin spec` | Generate/update the technical spec |

## Running as a service

### systemd

```ini title="/etc/systemd/system/merlin-agent.service"
[Unit]
Description=Merlin Code Review Agent
After=network.target

[Service]
ExecStart=/usr/local/bin/merlin agent start
EnvironmentFile=/etc/merlin/env
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl enable --now merlin-agent
```

### Docker

```bash
docker run -d \
  --name merlin-agent \
  --restart unless-stopped \
  -p 9000:9000 \
  -v $(pwd)/merlin.toml:/app/merlin.toml:ro \
  -e ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY \
  -e GITHUB_TOKEN=$GITHUB_TOKEN \
  -e MERLIN_WEBHOOK_SECRET=$MERLIN_WEBHOOK_SECRET \
  ghcr.io/you/merlin:latest agent start
```

### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: merlin-agent
spec:
  replicas: 1
  selector:
    matchLabels:
      app: merlin-agent
  template:
    metadata:
      labels:
        app: merlin-agent
    spec:
      containers:
        - name: merlin
          image: ghcr.io/you/merlin:latest
          args: ["agent", "start"]
          ports:
            - containerPort: 9000
          envFrom:
            - secretRef:
                name: merlin-secrets
```

## Webhook registration

Merlin auto-registers the webhook when you run `merlin agent start`, provided the token has `admin:repo_hook` scope.

To register manually:

```bash
merlin agent register-webhook --url https://merlin.example.com/webhook
```

## Next steps

- [Slack notifications](./slack) — get review summaries in Slack
- [Discord notifications](./discord) — get review summaries in Discord
