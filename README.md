# Merlin 🧙

[![CI](https://img.shields.io/github/actions/workflow/status/Arunachalamkalimuthu/merlin-ai-code-review/ci.yml?branch=main&label=CI&logo=github&style=flat-square)](https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/actions/workflow/status/Arunachalamkalimuthu/merlin-ai-code-review/release.yml?label=Release&logo=github&style=flat-square)](https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/actions/workflows/release.yml)
[![Latest Release](https://img.shields.io/github/v/release/Arunachalamkalimuthu/merlin-ai-code-review?style=flat-square&color=f78166)](https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest)
[![License: MIT](https://img.shields.io/github/license/Arunachalamkalimuthu/merlin-ai-code-review?style=flat-square)](LICENSE)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-dea584?logo=rust&style=flat-square)](https://www.rust-lang.org/)
[![Docker](https://img.shields.io/badge/docker-ghcr.io-2496ed?logo=docker&logoColor=white&style=flat-square)](https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/pkgs/container/merlin-ai-code-review)
[![Stars](https://img.shields.io/github/stars/Arunachalamkalimuthu/merlin-ai-code-review?style=flat-square)](https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/stargazers)
[![Contributors](https://img.shields.io/github/contributors/Arunachalamkalimuthu/merlin-ai-code-review?style=flat-square)](https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/graphs/contributors)
[![Issues](https://img.shields.io/github/issues/Arunachalamkalimuthu/merlin-ai-code-review?style=flat-square)](https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/issues)
[![Known Vulnerabilities](https://snyk.io/test/github/Arunachalamkalimuthu/merlin-ai-code-review/badge.svg?style=flat-square)](https://snyk.io/test/github/Arunachalamkalimuthu/merlin-ai-code-review)
[![Docs](https://img.shields.io/badge/docs-merlin--review.com-f78166?style=flat-square)](https://merlin-review.com/)

**Self-hosted AI code review for GitHub, GitLab, Bitbucket, Azure DevOps, and Gitea** — open-source, bring-your-own-key, no code leaves your infrastructure.

Merlin parses PR/MR diffs, sends the code to a configurable AI provider, and posts inline review comments plus a summary back to the PR/MR. It also auto-generates technical specifications, runs an autonomous ReAct-loop agent, and maintains a RAG index of your codebase for context-aware reviews.

---

## Up and running in 60 seconds

### GitHub Actions

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
      - run: |
          curl -fsSL \
            https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest/download/install.sh \
            | sh
          merlin review
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
```

### GitLab CI

```yaml
merlin-review:
  image: ubuntu:22.04
  script:
    - curl -fsSL .../install.sh | sh
    - merlin review
  variables:
    GITLAB_TOKEN: $CI_JOB_TOKEN
    ANTHROPIC_API_KEY: $ANTHROPIC_API_KEY
  rules:
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"
```

See [`.gitlab-ci.yml.example`](.gitlab-ci.yml.example) for the full example with caching.

---

## Features

### Slash commands

Trigger any command by commenting on a PR — `@merlin /command` — or run them directly from CI with `merlin run /command`.

| Command | What it does | Output |
|---|---|---|
| `/review` | Full code review with inline comments | PR inline comments + summary |
| `/spec` | Generate a technical specification and update the PR description | Updates PR title + description |
| `/describe` | Auto-generate a structured PR title & description | Updates PR description |
| `/ask <question>` | Q&A about the PR diff | PR comment |
| `/improve` | Inline code suggestion blocks | PR suggestion comments |
| `/generate_labels` | Auto-label based on diff content + size | PR labels |
| `/update_changelog` | Prepend an entry to CHANGELOG.md | File commit |
| `/add_doc` | Generate missing docstrings | PR suggestion comments |
| `/similar_issue` | Find related open issues | PR comment table |
| `/test` | Generate unit tests for changed code | PR comment with test code |
| `/explain` | Plain-language walkthrough of the PR | PR comment |
| `/security` | Dedicated security scan (secrets + OWASP) | Inline + summary report |
| `/approve` | AI-assisted review verdict (approve / request changes) | PR review |
| `/commit_message` | Generate 3 conventional commit message options | PR comment |
| `/docs [mode]` | Documentation generator (`readme`/`api`/`adr`/`module`/`wiki`/`auto`) | PR comment or file commit |
| `/snyk` | Scan changed dependencies against the Snyk vulnerability database | PR comment |
| `/coverage` | Analyse test coverage for changed files | PR comment |
| `/link_jira` | Find related Jira issues and link them to the PR | PR comment |
| `/link_linear` | Find related Linear issues and link them to the PR | PR comment |
| `/triage` | Find similar open issues on CodeTriage for changed packages | PR comment |

### Auto-spec on PR open

When a PR is first opened, Merlin automatically generates a comprehensive **Technical Specification** and sets it as the PR description. The spec includes:

- Problem statement and motivation
- Technical approach and key design decisions
- Files changed and what each one does
- API / data model changes
- Testing strategy and rollout notes
- Open questions

Enable it by adding the `merlin-spec` job to your workflow (see [`review.yml`](.github/workflows/review.yml)).

### Autonomous Agent

Merlin includes a **ReAct-loop agent** (Reason → Act → Observe) that can autonomously plan and execute multiple tools to handle complex tasks.

- **CLI** — interactive REPL: `merlin agent`
- **Slack** — mention `@merlin` in any Slack message; Merlin replies in-thread
- **Discord** — mention `@Merlin`; Merlin replies to the message
- **Single-shot** — `merlin agent --task "summarise the open PRs"` (CI-friendly)
- **Conversation memory** — configurable ring-buffer with optional JSONL persistence across sessions

### RAG (context-aware reviews)

Index your codebase so Merlin has relevant context when reviewing each diff.

| Vector store | Setup | Best for |
|---|---|---|
| `local` | None — JSONL flat file | Dev, small repos, CI |
| `memory` | None — ephemeral RAM | Testing |
| `qdrant` | `docker run -p 6333:6333 qdrant/qdrant` | Production self-hosted |
| `chroma` | `docker run -p 8000:8000 chromadb/chroma` | Open-source alternative |
| `pinecone` | cloud.pinecone.io account | Managed cloud |

**Two embedding backends:**

| Embedder | When to use |
|---|---|
| `ollama` (default) | Local development — free, fully private, needs `ollama serve` |
| `openai` | CI/CD pipelines — works on any runner, needs `OPENAI_API_KEY` |

### Infrastructure

- **Bot mode** — webhook server (`merlin webhook`); GitHub/GitLab send PR comment events → Merlin dispatches commands automatically
- **Token-aware prioritisation** — files ranked by security sensitivity; token budget enforced before AI calls
- **6 AI backends** — Anthropic Claude, OpenAI GPT-4o, Claude Code CLI (no separate API key), Google Gemini, AWS Bedrock, Ollama (local)
- **5 VCS platforms** — GitHub, GitLab, Bitbucket, Azure DevOps, Gitea (auto-detected from CI env)
- **Concurrent reviews** — Tokio fan-out per diff chunk for speed
- **Reflect & Review** — optional second AI pass to filter false positives and correct severity
- **Local mode** — `merlin review --diff <file>` for offline testing
- **Configurable personas** — override the system prompt and review rules per project

---

## Configuration

Copy `config.example.toml` to `merlin.toml` in your repo root:

```toml
[ai]
# "anthropic" | "openai" | "claude-code" | "gemini" | "bedrock" | "ollama"
provider = "anthropic"
model    = "claude-sonnet-4-6"
max_tokens  = 4096
temperature = 0.2

[review]
focus        = ["bugs", "security", "style", "performance"]
max_comments = 30
chunk_lines  = 200
reflect      = false   # enable second-pass comment refinement

[rag]
enabled   = false
embedder  = "ollama"   # "ollama" (local) | "openai" (CI-friendly)
store     = "local"    # "local" | "memory" | "qdrant" | "chroma" | "pinecone"
embed_model = "nomic-embed-text"   # or "text-embedding-3-small" for openai
collection  = "merlin"
top_k       = 5
min_score   = 0.70
chunk_lines = 80
index_extensions = [".rs", ".ts", ".py", ".go", ".java", ".md"]

# ── Qdrant (store = "qdrant") ──────────────────────
# qdrant_url     = "http://localhost:6333"
# qdrant_api_key = ""   # optional, for Qdrant Cloud

# ── ChromaDB (store = "chroma") ────────────────────
# chroma_url = "http://localhost:8000"

# ── Pinecone (store = "pinecone") ──────────────────
# pinecone_host    = "https://my-index.svc.us-east1.pinecone.io"
# pinecone_api_key = ""   # or set PINECONE_API_KEY env var

[agent]
max_iterations     = 10
max_memory_messages = 50
# memory_file      = ".merlin-memory.jsonl"  # persist across runs
default_channel    = "cli"
port               = 8090
```

### Environment variables

| Variable | Purpose |
|---|---|
| `ANTHROPIC_API_KEY` | Claude API key (`provider = "anthropic"`) |
| `OPENAI_API_KEY` | OpenAI key — review (`provider = "openai"`) **and/or** RAG embeddings (`embedder = "openai"`) |
| `GEMINI_API_KEY` | Google Gemini API key |
| `AZURE_OPENAI_API_KEY` | Azure OpenAI key |
| `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` | AWS credentials for Bedrock |
| `GITHUB_TOKEN` | GitHub API token (provided automatically by Actions) |
| `GITLAB_TOKEN` | GitLab token (`$CI_JOB_TOKEN` in CI) |
| `BITBUCKET_TOKEN` | Bitbucket bearer token (or `BITBUCKET_APP_PASSWORD`) |
| `AZURE_DEVOPS_TOKEN` | Azure DevOps PAT (or `SYSTEM_ACCESSTOKEN`) |
| `GITEA_TOKEN` | Gitea API token |
| `PINECONE_API_KEY` | Pinecone API key (`store = "pinecone"`) |
| `SNYK_TOKEN` | Snyk API token (for `/snyk` command) |
| `JIRA_TOKEN` | Jira API token (for `/link_jira` command) |
| `LINEAR_API_KEY` | Linear API key (for `/link_linear` command) |
| `SLACK_BOT_TOKEN` | Slack bot token (`merlin agent --channel slack`) |
| `DISCORD_BOT_TOKEN` / `DISCORD_CHANNEL_ID` | Discord bot credentials |
| `MERLIN_GITHUB_SECRET` | Webhook HMAC secret (optional, bot mode) |
| `MERLIN_GITLAB_SECRET` | Webhook token (optional, bot mode) |

### AI provider setup

#### Anthropic Claude (default)
```toml
[ai]
provider = "anthropic"
model    = "claude-sonnet-4-6"
```
Set `ANTHROPIC_API_KEY`.

#### OpenAI GPT-4o
```toml
[ai]
provider = "openai"
model    = "gpt-4o"
```
Set `OPENAI_API_KEY`.

#### Claude Code CLI (no API key required)
```toml
[ai]
provider = "claude-code"
model    = "claude-sonnet-4-6"
```
```bash
claude auth login                             # developer machine
claude auth login --token $CLAUDE_CODE_TOKEN  # CI headless
```
Organisations on Claude Code subscriptions skip `ANTHROPIC_API_KEY` entirely.

#### Google Gemini
```toml
[ai]
provider = "gemini"
model    = "gemini-1.5-pro"
```
Set `GEMINI_API_KEY` from [Google AI Studio](https://aistudio.google.com/).

#### AWS Bedrock
```toml
[ai]
provider        = "bedrock"
model           = "anthropic.claude-sonnet-4-6-20250514-v1:0"
bedrock_region  = "us-east-1"
```
Set `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, and optionally `AWS_SESSION_TOKEN`.

#### Ollama (local, no API key)
```toml
[ai]
provider         = "ollama"
model            = "llama3.1"
ollama_base_url  = "http://localhost:11434"
```
Run `ollama serve` and pull a model: `ollama pull llama3.1`.

---

## RAG quick start

### Local development (Ollama embedder)

```toml
# merlin.toml
[rag]
enabled  = true
embedder = "ollama"
store    = "local"
```

```bash
ollama pull nomic-embed-text   # one-time
merlin rag index .             # index codebase (~seconds for most repos)
merlin review                  # reviews now include codebase context
```

### CI/CD (OpenAI embedder + cached index)

```toml
# merlin.toml
[rag]
enabled     = true
embedder    = "openai"              # reads OPENAI_API_KEY — no Ollama needed
embed_model = "text-embedding-3-small"
store       = "local"
```

```yaml
# In your GitHub Actions workflow:
- uses: actions/cache@v4
  with:
    path: merlin-rag.jsonl
    key: merlin-rag-${{ hashFiles('src/**', 'lib/**') }}
    restore-keys: merlin-rag-

- run: test -f merlin-rag.jsonl || merlin rag index .
  env:
    OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
```

The index is rebuilt only when source files change (cache key is a hash of your source tree). Indexing a typical 10 k-file repo costs around **$0.10** in OpenAI embedding credits.

### Production (Qdrant)

```bash
docker run -p 6333:6333 qdrant/qdrant
```

```toml
[rag]
enabled  = true
embedder = "openai"
store    = "qdrant"
qdrant_url = "http://localhost:6333"
```

---

## CLI reference

```bash
# ── Review ────────────────────────────────────────────────────────────────────
merlin review                                 # full CI review (auto-detects platform)
merlin review --diff path/to/changes.diff     # local review, no platform posting
merlin review --diff changes.diff --output json

# ── Slash commands ────────────────────────────────────────────────────────────
merlin run /spec
merlin run /describe
merlin run /review
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
merlin run /docs readme        # generate README section
merlin run /docs api           # generate API reference
merlin run /docs adr           # generate Architecture Decision Record
merlin run /docs module        # generate module docstrings
merlin run /docs wiki          # generate wiki page
merlin run /docs               # auto-detect best doc type

# ── Bot / webhook server ──────────────────────────────────────────────────────
merlin webhook --port 8080
# → GitHub: POST issue_comment events to http://host:8080/webhook/github
# → GitLab: POST Note Hook events to  http://host:8080/webhook/gitlab

# ── Autonomous agent ──────────────────────────────────────────────────────────
merlin agent                                  # CLI REPL (default)
merlin agent --channel slack                  # Slack Events API on --port 8090
merlin agent --channel discord                # Discord bot
merlin agent --task "summarise the open PRs"  # single-shot, CI-friendly

# ── RAG index management ──────────────────────────────────────────────────────
merlin rag index .                            # index current directory
merlin rag index src/                         # index a subdirectory
merlin rag search "auth bypass"               # query the index
merlin rag search "SQL injection" -k 10       # return up to 10 results
merlin rag count                              # number of indexed documents
merlin rag clear                              # delete all indexed data

# ── Debug ─────────────────────────────────────────────────────────────────────
merlin parse-diff path/to/changes.diff        # show parsed file structure + priority
```

---

## Building

```bash
# Development
cargo build

# Release binary
cargo build --release

# Run tests
cargo test
cargo clippy -- -D warnings

# Docker
docker build -t merlin .
docker run --rm \
  -e GITHUB_TOKEN=... \
  -e ANTHROPIC_API_KEY=... \
  -e GITHUB_ACTIONS=true \
  -e GITHUB_REPOSITORY=owner/repo \
  -e GITHUB_SHA=abc123 \
  merlin review
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
  │     │     └── prioritize_diffs()  (token-aware, security-ranked)
  │     ├── AiProvider  (Anthropic | OpenAI | Claude Code | Gemini | Bedrock | Ollama)
  │     │     └── review(ReviewContext) → Vec<ReviewComment>
  │     └── RagPipeline  (optional)
  │           ├── Embedder
  │           │     ├── OllamaEmbedder  (local dev)
  │           │     └── OpenAiEmbedder  (CI/CD)
  │           └── VectorStore  (local | memory | qdrant | chroma | pinecone)
  │                 └── search() → Vec<RetrievedDoc> → injected into AI prompt
  │
  ├── ToolRouter  (slash commands)
  │     ├── /spec, /review, /describe, /ask, /improve
  │     ├── /generate_labels, /update_changelog, /add_doc, /similar_issue
  │     ├── /test, /explain, /security, /approve
  │     ├── /commit_message, /docs
  │     ├── /snyk, /coverage, /link_jira, /link_linear, /triage
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

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full guide — bug reports, feature requests, development setup, coding standards, commit conventions, and walkthroughs for adding a new AI provider, VCS platform, slash command, vector store, or agent channel.

---

## License

MIT
