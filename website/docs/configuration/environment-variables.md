---
sidebar_position: 3
title: Environment Variables
---

# Environment Variables

All secrets should be passed as environment variables — never committed to `merlin.toml`.

## AI providers

| Variable | Used by |
|---|---|
| `ANTHROPIC_API_KEY` | `provider = "anthropic"` |
| `OPENAI_API_KEY` | `provider = "openai"` and/or `[rag] embedder = "openai"` |
| `GEMINI_API_KEY` | `provider = "gemini"` |
| `AZURE_OPENAI_API_KEY` | `provider = "azure-openai"` |
| `AWS_ACCESS_KEY_ID` | `provider = "bedrock"` |
| `AWS_SECRET_ACCESS_KEY` | `provider = "bedrock"` |
| `AWS_SESSION_TOKEN` | `provider = "bedrock"` (temporary credentials) |
| `CLAUDE_CODE_TOKEN` | `provider = "claude-code"` (CI headless) |

## VCS platforms

| Variable | Used by |
|---|---|
| `GITHUB_TOKEN` | GitHub platform (provided automatically in GitHub Actions) |
| `GITLAB_TOKEN` | GitLab platform (use `$CI_JOB_TOKEN` in GitLab CI) |
| `GITLAB_URL` | Self-hosted GitLab instance URL |
| `BITBUCKET_TOKEN` | Bitbucket platform (or `BITBUCKET_APP_PASSWORD`) |
| `AZURE_DEVOPS_TOKEN` | Azure DevOps (or `SYSTEM_ACCESSTOKEN` in Azure Pipelines) |
| `GITEA_TOKEN` | Gitea platform |
| `GITEA_URL` | Self-hosted Gitea instance URL |

## Integrations

| Variable | Used by |
|---|---|
| `PINECONE_API_KEY` | `[rag] store = "pinecone"` |
| `SNYK_TOKEN` | `/snyk` command |
| `JIRA_TOKEN` | `/link_jira` command |
| `LINEAR_API_KEY` | `/link_linear` command |

## Agent channels

| Variable | Used by |
|---|---|
| `SLACK_BOT_TOKEN` | `merlin agent --channel slack` |
| `DISCORD_BOT_TOKEN` | `merlin agent --channel discord` |
| `DISCORD_CHANNEL_ID` | `merlin agent --channel discord` |

## Webhook bot mode

| Variable | Description |
|---|---|
| `MERLIN_GITHUB_SECRET` | HMAC-SHA256 secret for GitHub webhook signature verification |
| `MERLIN_GITLAB_SECRET` | Token for GitLab webhook header verification |

## Setting secrets in CI

### GitHub Actions

Go to *Settings → Secrets and variables → Actions → New repository secret*.

```yaml
env:
  ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
  GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

### GitLab CI

Go to *Settings → CI/CD → Variables*.

```yaml
variables:
  ANTHROPIC_API_KEY: $ANTHROPIC_API_KEY
  GITLAB_TOKEN: $CI_JOB_TOKEN
```

### Bitbucket Pipelines

Go to *Repository Settings → Repository variables*.

### Azure DevOps

Go to *Pipelines → Library → Variable groups* or the pipeline editor.
