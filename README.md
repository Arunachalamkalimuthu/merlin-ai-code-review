# Merlin

[![CI](https://img.shields.io/github/actions/workflow/status/Arunachalamkalimuthu/merlin-ai-code-review/ci.yml?branch=main&label=CI&logo=github&style=flat-square)](https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/actions/workflow/status/Arunachalamkalimuthu/merlin-ai-code-review/release.yml?label=Release&logo=github&style=flat-square)](https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/actions/workflows/release.yml)
[![Latest Release](https://img.shields.io/github/v/release/Arunachalamkalimuthu/merlin-ai-code-review?style=flat-square&color=f78166)](https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest)
[![License: MIT](https://img.shields.io/github/license/Arunachalamkalimuthu/merlin-ai-code-review?style=flat-square)](LICENSE)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-dea584?logo=rust&style=flat-square)](https://www.rust-lang.org/)
[![Docker](https://img.shields.io/badge/docker-ghcr.io-2496ed?logo=docker&logoColor=white&style=flat-square)](https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/pkgs/container/merlin-ai-code-review)
[![Known Vulnerabilities](https://snyk.io/test/github/Arunachalamkalimuthu/merlin-ai-code-review/badge.svg?style=flat-square)](https://snyk.io/test/github/Arunachalamkalimuthu/merlin-ai-code-review)
[![Stars](https://img.shields.io/github/stars/Arunachalamkalimuthu/merlin-ai-code-review?style=flat-square)](https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/stargazers)

**Self-hosted AI code review for GitHub, GitLab, Bitbucket, Azure DevOps, and Gitea.**

Merlin runs inside your CI pipeline, reviews pull request diffs with the AI provider of your choice, and posts inline comments directly on the PR. No code ever leaves your infrastructure.

```
PR opened
    │
    ▼
CI pipeline triggers Merlin
    │
    ├── Fetch PR diff from platform API
    ├── (optional) Search RAG index for relevant codebase context
    ├── Send diff + context to AI provider
    └── Post inline review comments back to the PR
              │
              ▼
        github-actions[bot] commented:
        🔴 [Critical] SQL injection via unsanitized input ...
```

---

## Table of Contents

- [Features](#features)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Quick Start — 5 Minutes](#quick-start--5-minutes)
- [Platform Integration](#platform-integration)
  - [GitHub Actions](#github-actions)
  - [GitLab CI](#gitlab-ci)
  - [Bitbucket Pipelines](#bitbucket-pipelines)
  - [Azure DevOps](#azure-devops)
  - [Gitea Actions](#gitea-actions)
- [Permissions & Bot Identity](#permissions--bot-identity)
- [AI Providers](#ai-providers)
  - [Anthropic Claude](#anthropic-claude-default)
  - [OpenAI GPT-4o](#openai-gpt-4o)
  - [Google Gemini](#google-gemini)
  - [AWS Bedrock](#aws-bedrock)
  - [Azure OpenAI](#azure-openai)
  - [Ollama (local)](#ollama-local--fully-private)
  - [Claude Code CLI](#claude-code-cli)
- [RAG — Context-Aware Reviews](#rag--context-aware-reviews)
- [Slash Commands](#slash-commands)
- [Webhook & Bot Mode](#webhook--bot-mode)
- [Autonomous Agent](#autonomous-agent)
- [Configuration Reference](#configuration-reference)
- [Environment Variables](#environment-variables)
- [CLI Reference](#cli-reference)
- [Troubleshooting](#troubleshooting)
- [Architecture](#architecture)
- [Building from Source](#building-from-source)
- [Contributing](#contributing)
- [License](#license)

---

## Features

| Category | Details |
|---|---|
| **AI providers** | Anthropic Claude, OpenAI GPT-4o, Google Gemini, AWS Bedrock, Azure OpenAI, Ollama (local), Claude Code CLI |
| **VCS platforms** | GitHub, GitLab, Bitbucket, Azure DevOps, Gitea — auto-detected from CI environment |
| **Slash commands** | 20+ commands triggered from PR comments (`@merlin /review`) or CLI (`merlin run /spec`) |
| **RAG pipeline** | Index your codebase; reviews include semantically relevant file context |
| **Bot mode** | Persistent webhook server that reacts to PR comment events automatically |
| **Autonomous agent** | ReAct-loop agent with Slack, Discord, and CLI channels |
| **Security focus** | Files ranked by security sensitivity; dedicated `/security` scan for secrets + OWASP |
| **Reflect & Review** | Optional second AI pass to filter false positives and refine severity |
| **Local mode** | `merlin review --diff <file>` for offline testing without a VCS platform |
| **Zero lock-in** | Swap AI providers, vector stores, or VCS platforms via a single config line |

---

## Prerequisites

You need **one** of the following to provide AI reviews:

| Provider | What you need |
|---|---|
| Anthropic Claude (recommended) | `ANTHROPIC_API_KEY` from [console.anthropic.com](https://console.anthropic.com) |
| OpenAI | `OPENAI_API_KEY` from [platform.openai.com](https://platform.openai.com) |
| Google Gemini | `GEMINI_API_KEY` from [Google AI Studio](https://aistudio.google.com) |
| AWS Bedrock | AWS credentials with Bedrock access |
| Azure OpenAI | Azure OpenAI resource + deployment |
| Ollama | Local [Ollama](https://ollama.com) install — no API key |
| Claude Code CLI | Claude Code subscription — no API key |

Your VCS platform token (`GITHUB_TOKEN`, `CI_JOB_TOKEN`, etc.) is provided automatically by CI — **no manual setup needed**.

---

## Installation

Pick the method that fits your workflow. All methods produce the same binary.

### Option 1 — One-line installer (recommended)

```bash
# Linux / macOS
curl -fsSL \
  https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest/download/install.sh \
  | sh
```

```powershell
# Windows (PowerShell)
irm https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest/download/install.ps1 | iex
```

The installer auto-detects your OS and architecture and places the binary in `/usr/local/bin` (or `%LOCALAPPDATA%\Programs\merlin` on Windows).

### Option 2 — Docker image

```bash
docker pull ghcr.io/arunachalamkalimuthu/merlin-ai-code-review:latest
```

The image is multi-arch (`linux/amd64`, `linux/arm64`) and uses a fully-static musl binary — no libc issues on Alpine-based runners.

### Option 3 — Pre-built binary

Download the binary for your platform from the [latest release](https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest):

| Platform | Binary |
|---|---|
| Linux x86_64 (glibc) | `merlin-linux-amd64` |
| Linux x86_64 (musl / static) | `merlin-linux-amd64-musl` |
| Linux arm64 (glibc) | `merlin-linux-arm64` |
| Linux arm64 (musl / static) | `merlin-linux-arm64-musl` |
| macOS Intel | `merlin-darwin-amd64` |
| macOS Apple Silicon | `merlin-darwin-arm64` |
| Windows x86_64 | `merlin-windows-amd64.exe` |

> Use the `-musl` binaries on Alpine Linux or any musl-based distro.

### Option 4 — Build from source

```bash
# Requires Rust 1.85+
cargo install --git https://github.com/Arunachalamkalimuthu/merlin-ai-code-review
```

---

## Quick Start — 5 Minutes

This is the minimum setup to get Merlin reviewing PRs on GitHub with Anthropic Claude.

**Step 1 — Add your API key as a repository secret**

In your repository: **Settings → Secrets and variables → Actions → New repository secret**

- Name: `ANTHROPIC_API_KEY`
- Value: your key from [console.anthropic.com](https://console.anthropic.com)

**Step 2 — Create the workflow file**

Create `.github/workflows/merlin-review.yml` in your repository:

```yaml
name: Merlin AI Code Review

on:
  pull_request:
    types: [opened, synchronize, reopened]

permissions:
  contents: read
  pull-requests: write

jobs:
  merlin-review:
    name: Merlin AI Review
    runs-on: ubuntu-latest
    container:
      image: ghcr.io/arunachalamkalimuthu/merlin-ai-code-review:latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Run Merlin Review
        env:
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          PR_NUMBER: ${{ github.event.pull_request.number }}
          REPO: ${{ github.repository }}
        run: merlin review
```

**Step 3 — Open a pull request**

Merlin will automatically review the diff and post inline comments. Comments appear as `github-actions[bot]` — no bot account needed.

That's it. For other platforms or advanced configuration, read on.

---

## Platform Integration

### GitHub Actions

Two equivalent approaches — choose whichever fits your stack.

#### Option A — Docker container (simplest)

```yaml
# .github/workflows/merlin-review.yml
name: Merlin AI Code Review

on:
  pull_request:
    types: [opened, synchronize, reopened]

permissions:
  contents: read        # required for actions/checkout
  pull-requests: write  # required to read diff and post comments

jobs:
  merlin-review:
    name: Merlin AI Review
    runs-on: ubuntu-latest
    container:
      image: ghcr.io/arunachalamkalimuthu/merlin-ai-code-review:latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Run Merlin Review
        env:
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          PR_NUMBER: ${{ github.event.pull_request.number }}
          REPO: ${{ github.repository }}
        run: merlin review
```

#### Option B — Binary install (with RAG index caching)

```yaml
# .github/workflows/merlin-review.yml
name: Merlin AI Code Review

on:
  pull_request:
    types: [opened, synchronize, reopened]

permissions:
  contents: read
  pull-requests: write

jobs:
  merlin-review:
    name: Merlin AI Review
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Cache RAG index
        uses: actions/cache@v4
        with:
          path: merlin-rag.jsonl
          key: merlin-rag-${{ hashFiles('src/**', 'lib/**') }}
          restore-keys: merlin-rag-

      - name: Install Merlin
        run: |
          curl -fsSL \
            https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest/download/install.sh \
            | sh

      - name: Build RAG index (first run only)
        run: test -f merlin-rag.jsonl || merlin rag index .
        env:
          OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}

      - name: Run Merlin Review
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
          OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
        run: merlin review
```

> **Important:** The `permissions` block is mandatory. Without `pull-requests: write`, GitHub returns `403 Forbidden` when Merlin tries to fetch the PR diff or post comments.

#### Secrets to configure

| Secret | Required | Purpose |
|---|---|---|
| `ANTHROPIC_API_KEY` | Yes (if using Anthropic) | AI review provider |
| `OPENAI_API_KEY` | Only for RAG embeddings | Codebase indexing |

`GITHUB_TOKEN` is provided automatically — do not create it manually.

---

### GitLab CI

```yaml
# .gitlab-ci.yml
merlin-review:
  image: ghcr.io/arunachalamkalimuthu/merlin-ai-code-review:latest
  stage: review
  rules:
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"
  variables:
    GITLAB_TOKEN: $CI_JOB_TOKEN       # automatic — no setup needed
    ANTHROPIC_API_KEY: $ANTHROPIC_API_KEY
  script:
    - merlin review
```

**With RAG index caching:**

```yaml
merlin-review:
  image: ghcr.io/arunachalamkalimuthu/merlin-ai-code-review:latest
  stage: review
  rules:
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"
  cache:
    key: merlin-rag-$CI_DEFAULT_BRANCH
    paths:
      - merlin-rag.jsonl
  variables:
    GITLAB_TOKEN: $CI_JOB_TOKEN
    ANTHROPIC_API_KEY: $ANTHROPIC_API_KEY
    OPENAI_API_KEY: $OPENAI_API_KEY
  script:
    - test -f merlin-rag.jsonl || merlin rag index .
    - merlin review
```

**CI/CD variables to configure** (Settings → CI/CD → Variables):

| Variable | Required | Purpose |
|---|---|---|
| `ANTHROPIC_API_KEY` | Yes | AI review provider |
| `OPENAI_API_KEY` | Only for RAG | Codebase embeddings |

`CI_JOB_TOKEN` is injected automatically by GitLab. Comments appear as the **GitLab project bot**.

See [`.gitlab-ci.yml.example`](.gitlab-ci.yml.example) for all RAG embedding and vector store combinations:

| Setup | Embedder | Store | Extra requirements |
|---|---|---|---|
| A — Recommended | OpenAI | Local JSONL (cached) | `OPENAI_API_KEY` |
| B — Self-hosted | OpenAI | Qdrant (GitLab service) | `OPENAI_API_KEY` |
| C — Managed cloud | OpenAI | Pinecone | `OPENAI_API_KEY` + `PINECONE_API_KEY` |
| D — Fully private | Ollama (GitLab service) | Local JSONL | Privileged runner |
| E — No RAG | — | — | Nothing extra |

---

### Bitbucket Pipelines

```yaml
# bitbucket-pipelines.yml
pipelines:
  pull-requests:
    '**':
      - step:
          name: Merlin AI Review
          image: ghcr.io/arunachalamkalimuthu/merlin-ai-code-review:latest
          script:
            - merlin review
          variables:
            BITBUCKET_TOKEN: $BITBUCKET_STEP_TOKEN   # automatic — no setup needed
            ANTHROPIC_API_KEY: $ANTHROPIC_API_KEY
```

**With RAG index caching:**

```yaml
pipelines:
  pull-requests:
    '**':
      - step:
          name: Merlin AI Review
          image: ghcr.io/arunachalamkalimuthu/merlin-ai-code-review:latest
          caches:
            - merlin-rag
          script:
            - test -f merlin-rag.jsonl || merlin rag index .
            - merlin review
          variables:
            BITBUCKET_TOKEN: $BITBUCKET_STEP_TOKEN
            ANTHROPIC_API_KEY: $ANTHROPIC_API_KEY
            OPENAI_API_KEY: $OPENAI_API_KEY

definitions:
  caches:
    merlin-rag:
      key:
        files:
          - src/**
      path: merlin-rag.jsonl
```

**Repository variables to configure** (Repository settings → Pipelines → Repository variables):

| Variable | Required | Purpose |
|---|---|---|
| `ANTHROPIC_API_KEY` | Yes | AI review provider |
| `OPENAI_API_KEY` | Only for RAG | Codebase embeddings |

`BITBUCKET_STEP_TOKEN` is created automatically per step. Comments appear as the **Pipelines build service user** — no bot account needed.

---

### Azure DevOps

```yaml
# azure-pipelines.yml
trigger: none

pr:
  branches:
    include:
      - '*'

pool:
  vmImage: ubuntu-latest

container:
  image: ghcr.io/arunachalamkalimuthu/merlin-ai-code-review:latest

steps:
  - checkout: self
    fetchDepth: 0

  - script: merlin review
    displayName: Merlin AI Review
    env:
      AZURE_DEVOPS_TOKEN: $(System.AccessToken)
      ANTHROPIC_API_KEY: $(ANTHROPIC_API_KEY)
      SYSTEM_TEAMFOUNDATIONCOLLECTIONURI: $(System.TeamFoundationCollectionUri)
      SYSTEM_TEAMPROJECT: $(System.TeamProject)
      BUILD_REPOSITORY_NAME: $(Build.Repository.Name)
      BUILD_SOURCEBRANCH: $(Build.SourceBranch)
      SYSTEM_PULLREQUEST_PULLREQUESTID: $(System.PullRequest.PullRequestId)
```

**One-time pipeline setup:**

In the Azure DevOps pipeline editor, click **⋮ → Triggers → YAML → Get sources** and tick **"Allow scripts to access the OAuth token"**. This exposes `$(System.AccessToken)` to the script without requiring a PAT.

**Pipeline variables to configure** (Pipelines → Edit → Variables):

| Variable | Required | Purpose |
|---|---|---|
| `ANTHROPIC_API_KEY` | Yes | AI review provider |
| `OPENAI_API_KEY` | Only for RAG | Codebase embeddings |

Comments appear as **Project Collection Build Service ({org})** — no bot account needed.

---

### Gitea Actions

```yaml
# .gitea/workflows/merlin-review.yml
name: Merlin AI Code Review

on:
  pull_request:
    types: [opened, synchronize, reopened]

jobs:
  merlin-review:
    name: Merlin AI Review
    runs-on: ubuntu-latest
    container:
      image: ghcr.io/arunachalamkalimuthu/merlin-ai-code-review:latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Run Merlin Review
        env:
          GITEA_TOKEN: ${{ secrets.GITEA_TOKEN }}   # automatic (Gitea 1.21+)
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
          PR_NUMBER: ${{ github.event.pull_request.number }}
          REPO: ${{ github.repository }}
        run: merlin review
```

**Secrets to configure** (Repository Settings → Secrets):

| Secret | Required | Purpose |
|---|---|---|
| `ANTHROPIC_API_KEY` | Yes | AI review provider |

`secrets.GITEA_TOKEN` is created automatically by Gitea Actions (v1.21+). Comments appear as **`gitea-actions[bot]`** — no bot account needed.

---

## Permissions & Bot Identity

### All platforms: bot identity is automatic

Every platform provides a built-in CI token. Merlin uses it to post comments as a bot — no manual bot account or GitHub App required.

| Platform | Token to use | Comments appear as | Extra setup |
|---|---|---|---|
| GitHub Actions | `secrets.GITHUB_TOKEN` | `github-actions[bot]` | Add `permissions` block (see below) |
| GitLab CI | `CI_JOB_TOKEN` | GitLab project bot | None |
| Bitbucket Pipelines | `BITBUCKET_STEP_TOKEN` | Pipelines build service | None |
| Azure DevOps | `System.AccessToken` | Project Collection Build Service | Enable OAuth token in pipeline settings |
| Gitea Actions | `secrets.GITEA_TOKEN` | `gitea-actions[bot]` | None (Gitea 1.21+) |

> **Comments appearing under your personal account?** You are passing a Personal Access Token (PAT) instead of the platform's automatic token. Switch to the token in the table above and the bot identity is restored automatically.

### GitHub: required permissions block

GitHub defaults to a read-only token. Add this block at the workflow level or the API returns `403 Forbidden`:

```yaml
permissions:
  contents: read        # needed by actions/checkout
  pull-requests: write  # needed to read the PR diff and post inline comments
```

### GitHub: custom named bot (optional)

To post as `"Merlin AI Reviewer[bot]"` instead of `github-actions[bot]`:

1. Go to **GitHub Settings → Developer Settings → GitHub Apps → New GitHub App**
2. Set permissions: `Pull requests: Read & write`, `Contents: Read`. Disable webhooks.
3. Install the app on your repository.
4. Store the App ID and private key as secrets (`MERLIN_APP_ID`, `MERLIN_APP_PRIVATE_KEY`).
5. Generate a token in the workflow:

```yaml
permissions:
  contents: read
  pull-requests: write

jobs:
  merlin-review:
    runs-on: ubuntu-latest
    steps:
      - name: Generate bot token
        id: app-token
        uses: actions/create-github-app-token@v1
        with:
          app-id: ${{ secrets.MERLIN_APP_ID }}
          private-key: ${{ secrets.MERLIN_APP_PRIVATE_KEY }}

      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Run Merlin Review
        env:
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
          GITHUB_TOKEN: ${{ steps.app-token.outputs.token }}
          PR_NUMBER: ${{ github.event.pull_request.number }}
          REPO: ${{ github.repository }}
        run: merlin review
```

This step is optional. `github-actions[bot]` works out of the box with zero configuration.

---

## AI Providers

Merlin auto-detects which provider to use based on the environment variables present, or you can pin one in `merlin.toml`.

### Anthropic Claude (default)

```toml
# merlin.toml
[ai]
provider   = "anthropic"
model      = "claude-sonnet-4-6"   # or claude-opus-4-6, claude-haiku-4-5-20251001
max_tokens = 4096
```

```bash
export ANTHROPIC_API_KEY=sk-ant-...
merlin review
```

Get a key at [console.anthropic.com](https://console.anthropic.com).

---

### OpenAI GPT-4o

```toml
[ai]
provider = "openai"
model    = "gpt-4o"   # or gpt-4o-mini, gpt-4-turbo
```

```bash
export OPENAI_API_KEY=sk-...
merlin review
```

`OPENAI_API_KEY` also powers RAG embeddings when `embedder = "openai"` — one key for both.

---

### Google Gemini

```toml
[ai]
provider = "gemini"
model    = "gemini-1.5-pro"   # or gemini-2.0-flash, gemini-1.5-flash
```

```bash
export GEMINI_API_KEY=AIza...
merlin review
```

Get a key from [Google AI Studio](https://aistudio.google.com).

---

### AWS Bedrock

```toml
[ai]
provider        = "bedrock"
model           = "anthropic.claude-sonnet-4-6-20250514-v1:0"
bedrock_region  = "us-east-1"
```

```bash
export AWS_ACCESS_KEY_ID=AKIA...
export AWS_SECRET_ACCESS_KEY=...
export AWS_SESSION_TOKEN=...   # optional, for temporary credentials
merlin review
```

The IAM role/user needs the `bedrock:InvokeModel` permission for the chosen model ARN.

---

### Azure OpenAI

```toml
[ai]
provider                 = "azure-openai"
model                    = "gpt-4o"
azure_openai_endpoint    = "https://my-resource.openai.azure.com"
azure_openai_deployment  = "my-gpt4o-deployment"
```

```bash
export AZURE_OPENAI_API_KEY=...
merlin review
```

---

### Ollama (local — fully private)

No API key required. All processing stays on your machine.

```toml
[ai]
provider        = "ollama"
model           = "llama3.1"   # any model pulled with `ollama pull`
ollama_base_url = "http://localhost:11434"
```

```bash
ollama serve
ollama pull llama3.1
merlin review
```

Good local models for code review: `codellama`, `deepseek-coder`, `qwen2.5-coder`.

---

### Claude Code CLI

For teams with a Claude Code subscription — no `ANTHROPIC_API_KEY` needed.

```toml
[ai]
provider = "claude-code"
model    = "claude-sonnet-4-6"
```

```bash
# Developer machine (interactive)
claude auth login

# CI (headless)
claude auth login --token $CLAUDE_CODE_TOKEN
merlin review
```

Set `CLAUDE_CODE_TOKEN` as a CI secret. The token is obtained from your Claude Code account settings.

---

## RAG — Context-Aware Reviews

RAG (Retrieval-Augmented Generation) indexes your codebase into a vector store. When reviewing a PR, Merlin retrieves the most relevant files and injects them into the AI prompt — giving the reviewer full context beyond the diff alone.

### When to use RAG

- Large codebases where a diff touches shared utilities or interfaces
- Reviews that need to understand how changed code is called elsewhere
- Finding similar patterns or security issues across the repo

### Setup in merlin.toml

```toml
[rag]
enabled          = true
embedder         = "openai"              # "openai" for CI, "ollama" for local
store            = "local"              # see vector store table below
embed_model      = "text-embedding-3-small"
collection       = "merlin"
top_k            = 5                    # number of relevant chunks to inject
min_score        = 0.70                 # similarity threshold (0.0–1.0)
chunk_lines      = 80                   # lines per indexed chunk
index_extensions = [".rs", ".ts", ".py", ".go", ".java", ".md"]
local_path       = "merlin-rag.jsonl"  # local store file path
```

### Embedding backends

| Embedder | Best for | Model | Requires |
|---|---|---|---|
| `openai` | CI/CD — any runner | `text-embedding-3-small` | `OPENAI_API_KEY` |
| `ollama` | Local dev — free, fully private | `nomic-embed-text` | `ollama serve` |

### Vector stores

| Store | Setup | Best for |
|---|---|---|
| `local` | None — single JSONL file, cache in CI | Small/medium repos, CI |
| `memory` | None — ephemeral | Testing |
| `qdrant` | `docker run -p 6333:6333 qdrant/qdrant` | Production, self-hosted |
| `chroma` | `docker run -p 8000:8000 chromadb/chroma` | Open-source alternative |
| `pinecone` | Managed cloud account | Zero-ops managed |

### Local development (Ollama + local store)

```toml
# merlin.toml
[rag]
enabled  = true
embedder = "ollama"
store    = "local"
```

```bash
ollama pull nomic-embed-text
merlin rag index .         # index once (a few seconds for most repos)
merlin review              # reviews now include codebase context
```

### CI/CD (OpenAI + cached JSONL)

```toml
# merlin.toml
[rag]
enabled     = true
embedder    = "openai"
embed_model = "text-embedding-3-small"
store       = "local"
```

Add to your GitHub Actions workflow:

```yaml
- name: Cache RAG index
  uses: actions/cache@v4
  with:
    path: merlin-rag.jsonl
    key: merlin-rag-${{ hashFiles('src/**', 'lib/**') }}
    restore-keys: merlin-rag-

- name: Build RAG index (first run only)
  run: test -f merlin-rag.jsonl || merlin rag index .
  env:
    OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
```

Indexing a typical 10k-file repo costs around **$0.10** in OpenAI embedding credits and only re-runs when source files change.

### Production (Qdrant persistent store)

```toml
# merlin.toml
[rag]
enabled    = true
embedder   = "openai"
store      = "qdrant"
qdrant_url = "http://localhost:6333"
# qdrant_api_key = ""   # required for Qdrant Cloud
```

The index persists in Qdrant between CI runs — no file caching step needed.

---

## Slash Commands

Trigger commands from a PR comment using `@merlin /command`, or run them directly in CI with `merlin run /command`.

| Command | What it does | Output |
|---|---|---|
| `/review` | Full code review with inline comments | Inline comments + summary |
| `/spec` | Generate a technical specification | Updates PR description |
| `/describe` | Auto-generate PR title and description | Updates PR description |
| `/ask <question>` | Q&A about the PR diff | PR comment |
| `/improve` | Inline code suggestion blocks | PR suggestion comments |
| `/generate_labels` | Auto-label based on diff content and size | PR labels |
| `/update_changelog` | Prepend an entry to CHANGELOG.md | File commit |
| `/add_doc` | Generate missing docstrings | PR suggestion comments |
| `/similar_issue` | Find related open issues | PR comment table |
| `/test` | Generate unit tests for changed code | PR comment |
| `/explain` | Plain-language walkthrough of the PR | PR comment |
| `/security` | Dedicated security scan (secrets + OWASP Top 10) | Inline + summary report |
| `/approve` | AI-assisted review verdict | PR review submission |
| `/commit_message` | Generate 3 conventional commit message options | PR comment |
| `/docs [mode]` | Generate docs (`readme`/`api`/`adr`/`module`/`wiki`/`auto`) | PR comment or file commit |
| `/snyk` | Scan changed dependencies against Snyk database | PR comment |
| `/coverage` | Analyse test coverage for changed files | PR comment |
| `/link_jira` | Find and link related Jira issues | PR comment |
| `/link_linear` | Find and link related Linear issues | PR comment |
| `/triage` | Find similar open issues on CodeTriage | PR comment |

### Examples

```bash
# Run from CI
merlin run /review
merlin run /security
merlin run /ask "Is this change safe to deploy on Friday?"
merlin run /docs readme

# Trigger from a PR comment (requires webhook/bot mode)
@merlin /review
@merlin /ask "What is the performance impact of this change?"
@merlin /spec
```

---

## Webhook & Bot Mode

Bot mode runs Merlin as a persistent HTTP server. It listens for PR comment events and automatically dispatches slash commands — no CI trigger needed.

```bash
merlin webhook --port 8080
```

Configure your VCS platform to send webhook events to:

| Platform | Webhook URL | Event type |
|---|---|---|
| GitHub | `http://your-host:8080/webhook/github` | `issue_comment` |
| GitLab | `http://your-host:8080/webhook/gitlab` | Note Hook |

### Securing the webhook

```bash
# GitHub HMAC secret
export MERLIN_GITHUB_SECRET=your-secret
merlin webhook --port 8080

# GitLab token
export MERLIN_GITLAB_SECRET=your-token
merlin webhook --port 8080
```

Set the same secret in your platform's webhook settings under "Secret token".

### Running as a service

```yaml
# docker-compose.yml
services:
  merlin:
    image: ghcr.io/arunachalamkalimuthu/merlin-ai-code-review:latest
    command: webhook --port 8080
    ports:
      - "8080:8080"
    environment:
      GITHUB_TOKEN: your-token
      ANTHROPIC_API_KEY: your-key
      MERLIN_GITHUB_SECRET: your-secret
    restart: unless-stopped
```

---

## Autonomous Agent

The agent runs a ReAct (Reason + Act) loop — it plans, executes slash commands as tools, and iterates until the task is complete.

### CLI mode (interactive REPL)

```bash
merlin agent
# > Summarise the open PRs and flag any that touch auth code
# > Review PR #42 with a focus on SQL injection risks
```

### Single-shot (CI-friendly)

```bash
merlin agent --task "Review PR #42 and post a summary comment"
```

### Slack

```bash
export SLACK_BOT_TOKEN=xoxb-...
merlin agent --channel slack --port 8090
```

The agent listens on port 8090 for Slack Events API calls. Configure your Slack app's Event Subscriptions URL to `http://your-host:8090`.

### Discord

```bash
export DISCORD_BOT_TOKEN=...
export DISCORD_CHANNEL_ID=...
merlin agent --channel discord
```

### Agent configuration

```toml
# merlin.toml
[agent]
max_iterations      = 10          # max ReAct loop steps
max_memory_messages = 50          # context window for conversation history
memory_file         = ".merlin-memory.jsonl"   # persist memory across runs
default_channel     = "cli"
port                = 8090
```

---

## Configuration Reference

Copy `config.example.toml` to `merlin.toml` in your repo root. All fields are optional — Merlin works with zero configuration.

```toml
# merlin.toml

[ai]
# Provider: "anthropic" | "openai" | "claude-code" | "gemini" | "bedrock" | "azure-openai" | "ollama"
provider    = "anthropic"
model       = "claude-sonnet-4-6"
max_tokens  = 4096
temperature = 0.2   # lower = more consistent; 0.0–1.0

# Provider-specific options (uncomment as needed)
# ollama_base_url          = "http://localhost:11434"
# azure_openai_endpoint    = "https://my-resource.openai.azure.com"
# azure_openai_deployment  = "my-deployment"
# bedrock_region           = "us-east-1"

[review]
# Review categories to focus on
focus        = ["bugs", "security", "style", "performance"]

# Max inline comments per run (prevents PR spam)
max_comments = 30

# Lines per diff chunk sent to the AI
chunk_lines  = 200

# Second AI pass to filter false positives (slower but more accurate)
reflect      = false

[platform]
# Auto-detected from CI env vars. Override only if needed:
# type = "github"   # "github" | "gitlab" | "bitbucket" | "azure-devops" | "gitea"

[rag]
enabled          = false
embedder         = "openai"              # "openai" | "ollama"
store            = "local"              # "local" | "memory" | "qdrant" | "chroma" | "pinecone"
embed_model      = "text-embedding-3-small"
collection       = "merlin"
top_k            = 5
min_score        = 0.70
chunk_lines      = 80
local_path       = "merlin-rag.jsonl"
index_extensions = [".rs", ".ts", ".py", ".go", ".java", ".md"]

# Qdrant
# qdrant_url     = "http://localhost:6333"
# qdrant_api_key = ""

# ChromaDB
# chroma_url = "http://localhost:8000"

# Pinecone
# pinecone_host    = "https://my-index.svc.us-east1.pinecone.io"
# pinecone_api_key = ""   # or set PINECONE_API_KEY env var

[agent]
max_iterations      = 10
max_memory_messages = 50
# memory_file       = ".merlin-memory.jsonl"
default_channel     = "cli"
port                = 8090
```

---

## Environment Variables

All secrets are read from environment variables — never put them in `merlin.toml`.

### AI providers

| Variable | Provider |
|---|---|
| `ANTHROPIC_API_KEY` | Anthropic Claude |
| `OPENAI_API_KEY` | OpenAI (review and/or RAG embeddings) |
| `GEMINI_API_KEY` | Google Gemini |
| `AZURE_OPENAI_API_KEY` | Azure OpenAI |
| `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` | AWS Bedrock |
| `AWS_SESSION_TOKEN` | AWS Bedrock (temporary credentials) |
| `CLAUDE_CODE_TOKEN` | Claude Code CLI headless auth |

### VCS platforms

| Variable | Platform | Notes |
|---|---|---|
| `GITHUB_TOKEN` | GitHub | Auto-provided by Actions |
| `GITLAB_TOKEN` | GitLab | Use `$CI_JOB_TOKEN` in CI |
| `BITBUCKET_TOKEN` | Bitbucket | Use `$BITBUCKET_STEP_TOKEN` in CI |
| `AZURE_DEVOPS_TOKEN` | Azure DevOps | Use `$(System.AccessToken)` in CI |
| `GITEA_TOKEN` | Gitea | Auto-provided by Gitea Actions 1.21+ |

### Integrations

| Variable | Purpose |
|---|---|
| `PINECONE_API_KEY` | Pinecone vector store |
| `SNYK_TOKEN` | Snyk dependency scanning (`/snyk` command) |
| `JIRA_TOKEN` | Jira issue linking (`/link_jira` command) |
| `LINEAR_API_KEY` | Linear issue linking (`/link_linear` command) |
| `SLACK_BOT_TOKEN` | Slack agent channel |
| `DISCORD_BOT_TOKEN` | Discord agent channel |
| `DISCORD_CHANNEL_ID` | Discord channel to post in |
| `MERLIN_GITHUB_SECRET` | HMAC secret for GitHub webhook verification |
| `MERLIN_GITLAB_SECRET` | Token for GitLab webhook verification |

---

## CLI Reference

```bash
# ── Review ────────────────────────────────────────────────────────────────────
merlin review                                   # CI review (auto-detects platform)
merlin review --diff path/to/changes.diff       # local review, no platform posting
merlin review --diff changes.diff --output json # machine-readable output

# ── Slash commands ────────────────────────────────────────────────────────────
merlin run /review
merlin run /spec
merlin run /describe
merlin run /ask "Is this change thread-safe?"
merlin run /improve
merlin run /generate_labels
merlin run /update_changelog
merlin run /add_doc
merlin run /similar_issue
merlin run /test
merlin run /explain
merlin run /security
merlin run /approve
merlin run /commit_message
merlin run /snyk
merlin run /coverage
merlin run /link_jira
merlin run /link_linear
merlin run /triage
merlin run /docs              # auto-detect best doc type
merlin run /docs readme       # generate README section
merlin run /docs api          # generate API reference
merlin run /docs adr          # generate Architecture Decision Record
merlin run /docs module       # generate module docstrings
merlin run /docs wiki         # generate wiki page

# ── RAG index ─────────────────────────────────────────────────────────────────
merlin rag index .                              # index current directory
merlin rag index src/                           # index a subdirectory
merlin rag search "auth bypass"                 # semantic search
merlin rag search "SQL injection" -k 10         # return up to 10 results
merlin rag count                                # number of indexed documents
merlin rag clear                                # delete all indexed data

# ── Webhook server ────────────────────────────────────────────────────────────
merlin webhook --port 8080

# ── Autonomous agent ──────────────────────────────────────────────────────────
merlin agent                                    # CLI REPL
merlin agent --channel slack                    # Slack Events API on --port 8090
merlin agent --channel discord                  # Discord bot
merlin agent --task "summarise the open PRs"    # single-shot, CI-friendly

# ── Debug ─────────────────────────────────────────────────────────────────────
merlin parse-diff path/to/changes.diff          # show parsed file structure + priority
```

---

## Troubleshooting

### `merlin: not found` (exit code 127) in CI

The binary cannot be found in `PATH`. This happens when:

- Using the Docker container approach on Alpine: the glibc binary won't run on musl. Use the image from GHCR — it ships the correct musl static binary.
- Using the binary install approach: check that the install script ran successfully before the `merlin review` step.

```yaml
# Correct container approach — uses musl binary, works on any runner
container:
  image: ghcr.io/arunachalamkalimuthu/merlin-ai-code-review:latest
```

### `403 Forbidden` from GitHub API

Missing `permissions` block in the workflow. Add:

```yaml
permissions:
  contents: read
  pull-requests: write
```

### Comments appear under my username instead of a bot

You are passing a Personal Access Token (PAT) instead of the platform's automatic CI token. Use `${{ secrets.GITHUB_TOKEN }}` (GitHub), `$CI_JOB_TOKEN` (GitLab), or `$BITBUCKET_STEP_TOKEN` (Bitbucket). See [Permissions & Bot Identity](#permissions--bot-identity).

### `Platform API error: HTTP 401 Unauthorized`

The VCS token is missing or invalid. Ensure the correct env var is set:

- GitHub: `GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}`
- GitLab: `GITLAB_TOKEN: $CI_JOB_TOKEN`
- Bitbucket: `BITBUCKET_TOKEN: $BITBUCKET_STEP_TOKEN`
- Azure DevOps: `AZURE_DEVOPS_TOKEN: $(System.AccessToken)` (and OAuth token must be enabled in pipeline settings)

### RAG index is rebuilt on every CI run

The cache key or cache path is misconfigured. Ensure the `path` matches `local_path` in `merlin.toml` (default: `merlin-rag.jsonl`) and the `key` pattern covers your source directories.

### AI provider returns an error

- Check that the correct API key environment variable is set and not expired.
- Verify the model name is correct for your provider/region (especially for Bedrock where model IDs include region-specific suffixes).
- For Ollama, ensure `ollama serve` is running and the model has been pulled with `ollama pull <model>`.

---

## Architecture

```
CLI (clap)
  ├── ReviewEngine
  │     ├── PlatformClient  (GitHub | GitLab | Bitbucket | Azure DevOps | Gitea)
  │     │     ├── get_diff()
  │     │     ├── post_inline_comment()
  │     │     └── post_summary()
  │     ├── DiffParser → Vec<FileDiff>
  │     │     └── prioritize_diffs()   (token-aware, security-ranked)
  │     ├── AiProvider  (Anthropic | OpenAI | Claude Code | Gemini | Bedrock | Azure OpenAI | Ollama)
  │     │     └── review(ReviewContext) → Vec<ReviewComment>
  │     └── RagPipeline  (optional)
  │           ├── Embedder
  │           │     ├── OllamaEmbedder   (local dev — free, private)
  │           │     └── OpenAiEmbedder   (CI/CD — any runner)
  │           └── VectorStore  (local | memory | qdrant | chroma | pinecone)
  │                 └── search() → Vec<RetrievedDoc> → injected into AI prompt
  │
  ├── ToolRouter  (slash commands)
  │     ├── /spec, /review, /describe, /ask, /improve
  │     ├── /generate_labels, /update_changelog, /add_doc, /similar_issue
  │     ├── /test, /explain, /security, /approve, /commit_message
  │     ├── /docs, /snyk, /coverage, /link_jira, /link_linear, /triage
  │     └── Webhook server (axum) — dispatches commands from PR comments
  │
  └── AgentRuntime  (ReAct loop)
        ├── AgentMemory  (ring-buffer + optional JSONL persistence)
        ├── AgentTools   (all slash commands + post_comment + get_pr_info + rag_search)
        └── AgentChannel
              ├── CliChannel     (stdin/stdout REPL)
              ├── SlackChannel   (axum webhook + chat.postMessage)
              └── DiscordChannel (REST polling + message reply)
```

**Platform auto-detection** — Merlin reads CI environment variables to detect the active platform automatically:

| CI system | Detection variable |
|---|---|
| GitHub Actions | `GITHUB_ACTIONS=true` |
| GitLab CI | `GITLAB_CI=true` |
| Bitbucket Pipelines | `BITBUCKET_PIPELINE_UUID` |
| Azure DevOps | `TF_BUILD=True` |
| Gitea Actions | `GITEA_ACTIONS=true` |

---

## Building from Source

```bash
# Prerequisites: Rust 1.85+ (see rust-toolchain.toml)

# Clone
git clone https://github.com/Arunachalamkalimuthu/merlin-ai-code-review
cd merlin-ai-code-review

# Development build
cargo build

# Release binary
cargo build --release
./target/release/merlin --version

# Run tests
cargo test

# Lint
cargo clippy --all-targets --all-features -- -D warnings

# Docker (local build — uses glibc binary)
docker build -t merlin .

# Docker (CI image — uses musl static binary, requires pre-built dist/)
docker build -f Dockerfile.ci -t merlin-ci .
```

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full guide — bug reports, feature requests, development setup, coding standards, commit conventions, and walkthroughs for adding a new AI provider, VCS platform, slash command, vector store, or agent channel.

---

## License

MIT — see [LICENSE](LICENSE).
