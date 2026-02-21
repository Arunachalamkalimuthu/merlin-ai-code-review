---
sidebar_position: 1
title: Overview
---

# Bot Mode

Bot mode is a lightweight alternative to Agent mode. Instead of running a persistent webhook listener, Merlin responds to PR comment commands on-demand — triggered by CI, a cron job, or manually.

## Bot mode vs Agent mode

| Feature | Bot mode | Agent mode |
|---|---|---|
| Persistent process | No | Yes |
| Triggered by | CI job / cron | Webhook |
| Infrastructure | Zero (runs in CI) | Needs a server/container |
| Latency | Depends on CI queue | Near-instant |
| Auto-review new PRs | Via CI trigger | Yes |
| Interactive commands | Poll-based | Real-time |

**Use bot mode when:**
- You don't want to run a persistent server
- You're already using GitHub Actions or GitLab CI
- You want zero-infrastructure setup

**Use agent mode when:**
- You need instant responses to PR comments
- You want real-time Slack/Discord notifications
- You're managing many repos from one central service

## How it works

```
CI job runs on schedule / PR event
        │
        ▼
merlin bot run
        │
        ▼
Fetch open PRs → check for pending commands (@merlin review, etc.)
        │
        ▼
Execute commands → post responses back to PR
```

## Quick start

### GitHub Actions

```yaml title=".github/workflows/merlin-bot.yml"
on:
  issue_comment:
    types: [created]
  schedule:
    - cron: "*/5 * * * *"   # also poll every 5 minutes

jobs:
  merlin-bot:
    if: contains(github.event.comment.body, '@merlin')
    runs-on: ubuntu-latest
    permissions:
      pull-requests: write
      issues: write
    steps:
      - uses: actions/checkout@v4
      - name: Install Merlin
        run: curl -fsSL https://raw.githubusercontent.com/you/merlin/main/install.sh | sh
      - name: Run Merlin bot
        run: merlin bot run
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
```

### GitLab CI

```yaml title=".gitlab-ci.yml"
merlin-bot:
  stage: review
  script:
    - curl -fsSL https://raw.githubusercontent.com/you/merlin/main/install.sh | sh
    - merlin bot run
  variables:
    GITLAB_TOKEN: $CI_JOB_TOKEN
    ANTHROPIC_API_KEY: $ANTHROPIC_API_KEY
  rules:
    - if: '$CI_PIPELINE_SOURCE == "merge_request_event"'
    - if: '$CI_PIPELINE_SOURCE == "schedule"'
```

## Supported commands

When a PR comment contains any of these, Merlin responds:

| Comment | Response |
|---|---|
| `@merlin review` | Full review with inline comments |
| `@merlin security` | Security-focused review |
| `@merlin improve` | Improvement suggestions |
| `@merlin describe` | Regenerate PR description |
| `@merlin ask <question>` | Answer a question about the diff |
| `@merlin spec` | Generate technical specification |

## Configuration

```toml title="merlin.toml"
[bot]
enabled          = true
command_prefix   = "@merlin"
auto_review      = false       # review all open PRs automatically
max_open_prs     = 10          # cap for auto-review mode
```

## Polling vs event-driven

By default in GitHub Actions, `issue_comment` events fire immediately when a comment is posted — no polling needed. The `schedule` cron is a fallback for commands posted while the event trigger is disabled.

For GitLab, use a scheduled pipeline or MR event trigger:

```yaml
rules:
  - if: '$CI_PIPELINE_SOURCE == "merge_request_event"'
  - if: '$CI_PIPELINE_SOURCE == "schedule"'
```
