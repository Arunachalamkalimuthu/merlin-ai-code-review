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

Merlin runs in your CI pipeline, reviews pull request diffs with your AI provider of choice, and posts inline comments directly on the PR — no code ever leaves your infrastructure. It also auto-generates technical specs, runs slash commands from PR comments, maintains a RAG index of your codebase for context-aware reviews, and ships an autonomous ReAct-loop agent for Slack, Discord, and the CLI.

---

## Table of Contents

- [Features](#features)
- [Installation](#installation)
- [Quick Start](#quick-start)
  - [GitHub Actions](#github-actions)
  - [GitLab CI](#gitlab-ci)
- [AI Providers](#ai-providers)
- [RAG — Context-Aware Reviews](#rag--context-aware-reviews)
- [Slash Commands](#slash-commands)
- [Configuration](#configuration)
- [CLI Reference](#cli-reference)
- [Architecture](#architecture)
- [Building from Source](#building-from-source)
- [Contributing](#contributing)
- [License](#license)

---

## Features

- **7 AI providers** — Anthropic Claude, OpenAI GPT-4o, Claude Code CLI, Google Gemini, AWS Bedrock, Azure OpenAI, Ollama (local)
- **5 VCS platforms** — GitHub, GitLab, Bitbucket, Azure DevOps, Gitea (auto-detected from CI env)
- **15 slash commands** — trigger from PR comments (`@merlin /review`) or directly in CI (`merlin run /spec`)
- **RAG pipeline** — index your codebase; reviews include semantically relevant file context
- **Autonomous agent** — ReAct-loop agent with Slack, Discord, and CLI channels
- **Bot / webhook mode** — persistent server that reacts to PR comment events automatically
- **Token-aware prioritisation** — files ranked by security sensitivity; token budget enforced before AI calls
- **Reflect & Review** — optional second AI pass to filter false positives and refine severity
- **Auto-spec** — generates a full technical specification as the PR description when a PR is opened
- **Local mode** — `merlin review --diff <file>` for offline testing without a VCS platform

---

## Installation

### Binary (Linux, macOS, Windows)

```bash
curl -fsSL \
  https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest/download/install.sh \
  | sh
```

The installer auto-detects your OS and architecture and places the binary in `/usr/local/bin`. On Windows, use the PowerShell installer:

```powershell
irm https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest/download/install.ps1 | iex
```

### Docker

```bash
docker pull ghcr.io/arunachalamkalimuthu/merlin-ai-code-review:latest
```

```bash
docker run --rm \
  -e GITHUB_TOKEN=... \
  -e ANTHROPIC_API_KEY=... \
  -e GITHUB_ACTIONS=true \
  -e GITHUB_REPOSITORY=owner/repo \
  -e GITHUB_SHA=abc123 \
  ghcr.io/arunachalamkalimuthu/merlin-ai-code-review:latest review
```

### Cargo

```bash
cargo install --git https://github.com/Arunachalamkalimuthu/merlin-ai-code-review
```

### Pre-built binaries

Download the binary for your platform directly from the [latest release](https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest):

| Platform | Binary |
|---|---|
| Linux x86_64 (glibc) | `merlin-linux-amd64` |
| Linux x86_64 (musl / static) | `merlin-linux-amd64-musl` |
| Linux arm64 (glibc) | `merlin-linux-arm64` |
| Linux arm64 (musl / static) | `merlin-linux-arm64-musl` |
| macOS Intel | `merlin-darwin-amd64` |
| macOS Apple Silicon | `merlin-darwin-arm64` |
| Windows x86_64 | `merlin-windows-amd64.exe` |

---

## Quick Start

### GitHub Actions

Add this workflow to your repository — no configuration file needed for a basic review:

```yaml
# .github/workflows/review.yml
on:
  pull_request:
    types: [opened, synchronize]

jobs:
  merlin-review:
    runs-on: ubuntu-latest
    permissions:
      pull-requests: write
    steps:
      - uses: actions/checkout@v4
        with: { fetch-depth: 0 }

      - name: Cache RAG index
        uses: actions/cache@v4
        with:
          path: merlin-rag.jsonl
          key: merlin-rag-${{ hashFiles('src/**', 'lib/**') }}
          restore-keys: merlin-rag-

      - run: |
          curl -fsSL \
            https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest/download/install.sh \
            | sh
          test -f merlin-rag.jsonl || merlin rag index .
          merlin review
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
          OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}   # for RAG embeddings
```

See the full workflow with auto-spec, security review, and bot mode in [`.github/workflows/review.yml`](.github/workflows/review.yml).

### GitLab CI

```yaml
merlin-review:
  image: ghcr.io/arunachalamkalimuthu/merlin-ai-code-review:latest
  cache:
    key: merlin-rag-$CI_DEFAULT_BRANCH
    paths:
      - merlin-rag.jsonl
  script:
    - merlin review
  variables:
    GITLAB_TOKEN: $CI_JOB_TOKEN
    ANTHROPIC_API_KEY: $ANTHROPIC_API_KEY
    OPENAI_API_KEY: $OPENAI_API_KEY     # for RAG embeddings
  rules:
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"
```

See [`.gitlab-ci.yml.example`](.gitlab-ci.yml.example) for all RAG embedding and vector store combinations:

| Setup | Embedder | Store | Requirements |
|---|---|---|---|
| **A — Recommended** | OpenAI | Local JSONL (cached) | `OPENAI_API_KEY` |
| **B — Self-hosted** | OpenAI | Qdrant (GitLab service) | `OPENAI_API_KEY` |
| **C — Managed cloud** | OpenAI | Pinecone | `OPENAI_API_KEY` + `PINECONE_API_KEY` |
| **D — Fully private** | Ollama (GitLab service) | Local JSONL (cached) | Privileged runner |
| **E — No RAG** | — | — | Nothing extra |

---

## AI Providers

Configure the provider in `merlin.toml` — API keys are always read from environment variables.

| Provider | `provider` value | Auth env var |
|---|---|---|
| Anthropic Claude | `"anthropic"` | `ANTHROPIC_API_KEY` |
| OpenAI GPT-4o | `"openai"` | `OPENAI_API_KEY` |
| Claude Code CLI | `"claude-code"` | `CLAUDE_CODE_TOKEN` (or `claude auth login`) |
| Google Gemini | `"gemini"` | `GEMINI_API_KEY` |
| AWS Bedrock | `"bedrock"` | `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` |
| Azure OpenAI | `"azure-openai"` | `AZURE_OPENAI_API_KEY` |
| Ollama (local) | `"ollama"` | None — runs locally |

### Anthropic Claude (default)

```toml
[ai]
provider = "anthropic"
model    = "claude-sonnet-4-6"
```

### OpenAI GPT-4o

```toml
[ai]
provider = "openai"
model    = "gpt-4o"
```

### Claude Code CLI (no API key required)

```toml
[ai]
provider = "claude-code"
model    = "claude-sonnet-4-6"
```

```bash
claude auth login                             # developer machine
claude auth login --token $CLAUDE_CODE_TOKEN  # CI headless
```

Teams on a Claude Code subscription skip `ANTHROPIC_API_KEY` entirely.

### Google Gemini

```toml
[ai]
provider = "gemini"
model    = "gemini-1.5-pro"
```

Get a key from [Google AI Studio](https://aistudio.google.com/), set `GEMINI_API_KEY`.

### AWS Bedrock

```toml
[ai]
provider        = "bedrock"
model           = "anthropic.claude-sonnet-4-6-20250514-v1:0"
bedrock_region  = "us-east-1"
```

Set `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, and optionally `AWS_SESSION_TOKEN`.

### Azure OpenAI

```toml
[ai]
provider              = "azure-openai"
model                 = "gpt-4o"
azure_openai_endpoint = "https://my-resource.openai.azure.com"
azure_openai_deployment = "my-gpt4o-deployment"
```

Set `AZURE_OPENAI_API_KEY`.

### Ollama (local, fully private)

```toml
[ai]
provider        = "ollama"
model           = "llama3.1"
ollama_base_url = "http://localhost:11434"
```

```bash
ollama serve
ollama pull llama3.1
merlin review
```

---

## RAG — Context-Aware Reviews

Merlin can index your codebase and inject relevant file snippets into the AI prompt, giving the reviewer full context for each diff. Configure it in `merlin.toml`:

```toml
[rag]
enabled          = true
embedder         = "openai"             # "ollama" (local) | "openai" (CI-friendly)
store            = "local"              # see vector store options below
embed_model      = "text-embedding-3-small"
collection       = "merlin"
top_k            = 5
min_score        = 0.70
chunk_lines      = 80
index_extensions = [".rs", ".ts", ".py", ".go", ".java", ".md"]
```

### Embedding backends

| Embedder | When to use | Model default |
|---|---|---|
| `ollama` | Local dev — free, fully private, needs `ollama serve` | `nomic-embed-text` |
| `openai` | CI/CD — works on any runner, needs `OPENAI_API_KEY` | `text-embedding-3-small` |

### Vector stores

| Store | Setup | Best for |
|---|---|---|
| `local` | None — JSONL flat file, cache it in CI | Small repos, dev, CI |
| `memory` | None — ephemeral RAM | Testing |
| `qdrant` | `docker run -p 6333:6333 qdrant/qdrant` | Production self-hosted |
| `chroma` | `docker run -p 8000:8000 chromadb/chroma` | Open-source alternative |
| `pinecone` | [cloud.pinecone.io](https://www.pinecone.io/) account | Managed cloud, zero ops |

### Local development (Ollama embedder)

```toml
# merlin.toml
[rag]
enabled  = true
embedder = "ollama"
store    = "local"
```

```bash
ollama pull nomic-embed-text
merlin rag index .        # index codebase (seconds for most repos)
merlin review             # reviews now include codebase context
```

### CI/CD (OpenAI embedder + cached index)

```toml
# merlin.toml
[rag]
enabled     = true
embedder    = "openai"
embed_model = "text-embedding-3-small"
store       = "local"
```

```yaml
# GitHub Actions — cache the index, rebuild only when source changes
- uses: actions/cache@v4
  with:
    path: merlin-rag.jsonl
    key: merlin-rag-${{ hashFiles('src/**', 'lib/**') }}
    restore-keys: merlin-rag-

- run: test -f merlin-rag.jsonl || merlin rag index .
  env:
    OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
```

Indexing a typical 10 k-file repo costs around **$0.10** in OpenAI embedding credits and only runs when source files change.

### Production (Qdrant)

```toml
# merlin.toml
[rag]
enabled    = true
embedder   = "openai"
store      = "qdrant"
qdrant_url = "http://localhost:6333"
# qdrant_api_key = ""   # required for Qdrant Cloud
```

The index persists in Qdrant between runs — no file caching needed.

---

## Slash Commands

Trigger any command by commenting on a PR — `@merlin /command` — or run directly from CI with `merlin run /command`.

| Command | What it does | Output |
|---|---|---|
| `/review` | Full code review with inline comments | Inline comments + summary |
| `/spec` | Generate a technical specification | Updates PR description |
| `/describe` | Auto-generate a structured PR title & description | Updates PR description |
| `/ask <question>` | Q&A about the PR diff | PR comment |
| `/improve` | Inline code suggestion blocks | PR suggestion comments |
| `/generate_labels` | Auto-label based on diff content + size | PR labels |
| `/update_changelog` | Prepend an entry to CHANGELOG.md | File commit |
| `/add_doc` | Generate missing docstrings | PR suggestion comments |
| `/similar_issue` | Find related open issues | PR comment table |
| `/test` | Generate unit tests for changed code | PR comment |
| `/explain` | Plain-language walkthrough of the PR | PR comment |
| `/security` | Dedicated security scan (secrets + OWASP) | Inline + summary report |
| `/approve` | AI-assisted review verdict | PR review |
| `/commit_message` | Generate 3 conventional commit message options | PR comment |
| `/docs [mode]` | Documentation generator (`readme`/`api`/`adr`/`module`/`wiki`/`auto`) | PR comment or file commit |
| `/snyk` | Scan changed dependencies against Snyk vulnerability database | PR comment |
| `/coverage` | Analyse test coverage for changed files | PR comment |
| `/link_jira` | Find and link related Jira issues | PR comment |
| `/link_linear` | Find and link related Linear issues | PR comment |
| `/triage` | Find similar open issues on CodeTriage | PR comment |

---

## Configuration

Copy `config.example.toml` to `merlin.toml` in your repo root. All fields are optional — Merlin works with sane defaults and no config file at all.

```toml
[ai]
# "anthropic" | "openai" | "claude-code" | "gemini" | "bedrock" | "azure-openai" | "ollama"
provider    = "anthropic"
model       = "claude-sonnet-4-6"
max_tokens  = 4096
temperature = 0.2

[review]
focus        = ["bugs", "security", "style", "performance"]
max_comments = 30
chunk_lines  = 200
reflect      = false   # second-pass comment refinement

[rag]
enabled          = false
embedder         = "ollama"             # "ollama" | "openai"
store            = "local"             # "local" | "memory" | "qdrant" | "chroma" | "pinecone"
embed_model      = "nomic-embed-text"  # or "text-embedding-3-small" for openai
collection       = "merlin"
top_k            = 5
min_score        = 0.70
chunk_lines      = 80
index_extensions = [".rs", ".ts", ".py", ".go", ".java", ".md"]
local_path       = "merlin-rag.jsonl"

# ── Qdrant ──────────────────────────────────────
# qdrant_url     = "http://localhost:6333"
# qdrant_api_key = ""   # optional, for Qdrant Cloud

# ── ChromaDB ────────────────────────────────────
# chroma_url = "http://localhost:8000"

# ── Pinecone ────────────────────────────────────
# pinecone_host    = "https://my-index.svc.us-east1.pinecone.io"
# pinecone_api_key = ""   # or set PINECONE_API_KEY env var

[agent]
max_iterations      = 10
max_memory_messages = 50
# memory_file       = ".merlin-memory.jsonl"   # persist across runs
default_channel     = "cli"
port                = 8090
```

### Environment variables

| Variable | Purpose |
|---|---|
| `ANTHROPIC_API_KEY` | Anthropic Claude (`provider = "anthropic"`) |
| `OPENAI_API_KEY` | OpenAI review (`provider = "openai"`) and/or RAG embeddings (`embedder = "openai"`) |
| `GEMINI_API_KEY` | Google Gemini (`provider = "gemini"`) |
| `AZURE_OPENAI_API_KEY` | Azure OpenAI (`provider = "azure-openai"`) |
| `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` | AWS credentials for Bedrock |
| `AWS_SESSION_TOKEN` | Optional — temporary AWS credentials |
| `CLAUDE_CODE_TOKEN` | Claude Code CLI headless auth |
| `GITHUB_TOKEN` | GitHub API token (provided automatically by Actions) |
| `GITLAB_TOKEN` | GitLab token (`$CI_JOB_TOKEN` in CI) |
| `BITBUCKET_TOKEN` | Bitbucket bearer token (or `BITBUCKET_APP_PASSWORD`) |
| `AZURE_DEVOPS_TOKEN` | Azure DevOps PAT (or `SYSTEM_ACCESSTOKEN`) |
| `GITEA_TOKEN` | Gitea API token |
| `PINECONE_API_KEY` | Pinecone vector store |
| `SNYK_TOKEN` | Snyk API token (for `/snyk` command) |
| `JIRA_TOKEN` | Jira API token (for `/link_jira` command) |
| `LINEAR_API_KEY` | Linear API key (for `/link_linear` command) |
| `SLACK_BOT_TOKEN` | Slack bot token (`merlin agent --channel slack`) |
| `DISCORD_BOT_TOKEN` / `DISCORD_CHANNEL_ID` | Discord bot credentials |
| `MERLIN_GITHUB_SECRET` | Webhook HMAC secret (optional, bot mode) |
| `MERLIN_GITLAB_SECRET` | Webhook token (optional, bot mode) |

---

## CLI Reference

```bash
# ── Review ────────────────────────────────────────────────────────────────────
merlin review                                  # full CI review (auto-detects platform)
merlin review --diff path/to/changes.diff      # local review, no platform posting
merlin review --diff changes.diff --output json

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
merlin run /docs readme     # generate README section
merlin run /docs api        # generate API reference
merlin run /docs adr        # generate Architecture Decision Record
merlin run /docs module     # generate module docstrings
merlin run /docs wiki       # generate wiki page
merlin run /docs            # auto-detect best doc type

# ── RAG index management ──────────────────────────────────────────────────────
merlin rag index .                             # index current directory
merlin rag index src/                          # index a subdirectory
merlin rag search "auth bypass"                # query the index
merlin rag search "SQL injection" -k 10        # return up to 10 results
merlin rag count                               # number of indexed documents
merlin rag clear                               # delete all indexed data

# ── Bot / webhook server ──────────────────────────────────────────────────────
merlin webhook --port 8080
# → GitHub: POST issue_comment events to http://host:8080/webhook/github
# → GitLab: POST Note Hook events to  http://host:8080/webhook/gitlab

# ── Autonomous agent ──────────────────────────────────────────────────────────
merlin agent                                   # CLI REPL (default)
merlin agent --channel slack                   # Slack Events API on --port 8090
merlin agent --channel discord                 # Discord bot
merlin agent --task "summarise the open PRs"   # single-shot, CI-friendly

# ── Debug ─────────────────────────────────────────────────────────────────────
merlin parse-diff path/to/changes.diff         # show parsed file structure + priority
```

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
  │     └── Webhook server (axum) → dispatches commands from PR comments
  │
  └── AgentRuntime  (ReAct loop)
        ├── AgentMemory  (ring-buffer + optional JSONL persistence)
        ├── AgentTools   (all slash commands + post_comment + get_pr_info + rag_search)
        └── AgentChannel
              ├── CliChannel     (stdin/stdout REPL)
              ├── SlackChannel   (axum webhook + chat.postMessage)
              └── DiscordChannel (REST polling + message reply)
```

---

## Building from Source

```bash
# Prerequisites: Rust 1.85+ (see rust-toolchain.toml)

# Development build
cargo build

# Release binary
cargo build --release

# Run tests
cargo test
cargo clippy --all-targets --all-features -- -D warnings

# Docker (local build)
docker build -t merlin .
```

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full guide — bug reports, feature requests, development setup, coding standards, commit conventions, and walkthroughs for adding a new AI provider, VCS platform, slash command, vector store, or agent channel.

---

## License

MIT — see [LICENSE](LICENSE).
