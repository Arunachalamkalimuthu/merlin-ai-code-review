# Merlin 🦡

**Self-hosted AI code review for GitHub, GitLab, Bitbucket, Azure DevOps, and Gitea** — open-source, bring-your-own-key, comparable to CodeRabbit and Qodo.

Merlin parses PR/MR diffs, sends the code to a configurable AI provider, and posts inline review comments plus a summary back to the PR/MR. It also ships an autonomous ReAct-loop agent and a RAG pipeline for context-aware reviews.

---

## Features

### Slash commands (mention `@merlin /command` in any PR comment)

| Command | Description | Output |
|---|---|---|
| `/review` | Full code review with inline comments | PR inline comments + summary |
| `/describe` | Auto-generate structured PR title & description | Updates PR description |
| `/ask <question>` | Q&A about the PR diff | PR comment |
| `/improve` | Inline code suggestion blocks | PR suggestion comments |
| `/generate_labels` | Auto-label based on diff content + size | PR labels |
| `/update_changelog` | Prepend entry to CHANGELOG.md | File commit |
| `/add_doc` | Generate missing docstrings | PR suggestion comments |
| `/similar_issue` | Find related open issues | PR comment table |
| `/test` | Generate unit tests for changed code | PR comment with test code |
| `/explain` | Plain-language walkthrough of the PR | PR comment |
| `/security` | Dedicated security scan (secrets + OWASP) | Inline + summary report |
| `/approve` | AI-assisted review verdict (approve / request changes) | PR comment |
| `/commit_message` | Generate 3 conventional commit message options | PR comment |
| `/docs [mode]` | Documentation generator (readme/api/adr/module/wiki/auto) | PR comment or file commit |
| `/snyk` | Scan changed dependencies against the Snyk vulnerability database | PR comment |
| `/coverage` | Analyse test coverage for changed files | PR comment |
| `/link_jira` | Find related Jira issues and link them to the PR | PR comment |
| `/link_linear` | Find related Linear issues and link them to the PR | PR comment |
| `/triage` | Find similar open issues on CodeTriage for changed packages | PR comment |

### Autonomous Agent

Merlin includes a **ReAct-loop agent** (Reason → Act → Observe) that can autonomously plan and run multiple tools to handle complex tasks.

- **CLI channel** — interactive REPL: `merlin agent --channel cli`
- **Slack channel** — mention `@merlin` in any Slack message; Merlin replies in-thread
- **Discord channel** — mention `@Merlin` or prefix with `merlin`; Merlin replies to the message
- **Single-shot mode** — `merlin agent --task "review and summarise the open PRs"` (non-interactive, CI-friendly)
- **Conversation memory** — configurable ring-buffer with optional JSONL persistence across sessions
- All slash commands are available as agent tools; the agent also has `post_comment`, `get_pr_info`, and `rag_search`

### RAG (Retrieval-Augmented Generation)

Index your codebase and past review comments so the AI has relevant context when reviewing diffs.

| Store | Setup | Best for |
|---|---|---|
| `local` | None — JSONL flat file | Small repos, dev / CI |
| `memory` | None — ephemeral RAM | Testing |
| `qdrant` | `docker run -p 6333:6333 qdrant/qdrant` | Production self-hosted |
| `chroma` | `docker run -p 8000:8000 chromadb/chroma` | Open-source alternative |
| `pinecone` | cloud.pinecone.io account | Managed cloud |

Embeddings are produced locally via **Ollama** (no embedding API cost).

### Infrastructure

- **Bot mode** — webhook server: `merlin webhook --port 8080`; GitHub/GitLab send PR comment events → Merlin dispatches commands automatically
- **Token-aware prioritization** — files ranked by security risk; token budget enforced before AI calls
- **5 AI backends** — Anthropic Claude, OpenAI GPT-4o, Claude Code CLI (no separate API key), Google Gemini, AWS Bedrock, Ollama (local models)
- **5 VCS platforms** — GitHub, GitLab, Bitbucket, Azure DevOps, Gitea (auto-detected from CI env)
- **Concurrent reviews** — Tokio fan-out per diff chunk for speed
- **Reflect & Review** — optional second AI pass to filter false positives and correct severity
- **Local mode** — `merlin review --diff <file>` for offline testing
- **Configurable** — `merlin.toml` for focus areas, comment caps, chunk sizes, and all backends

---

## Quick Start

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
          curl -L https://github.com/you/merlin/releases/latest/download/merlin-linux-amd64 -o merlin
          chmod +x merlin && ./merlin review
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
```

### GitLab CI

See [`.gitlab-ci.yml.example`](.gitlab-ci.yml.example).

---

## Configuration

Copy `config.example.toml` to `merlin.toml` in your repo root:

```toml
[ai]
# "anthropic" | "openai" | "claude-code" | "gemini" | "bedrock" | "ollama"
provider = "anthropic"
model = "claude-sonnet-4-6"
max_tokens = 4096
temperature = 0.2

[review]
focus = ["bugs", "security", "style", "performance"]
max_comments = 30
chunk_lines = 200
reflect = false          # enable second-pass comment refinement

[rag]
enabled = false
store = "local"          # "local" | "memory" | "qdrant" | "chroma" | "pinecone"
embed_model = "nomic-embed-text"
ollama_base_url = "http://localhost:11434"
collection = "merlin"
top_k = 5
min_score = 0.70
chunk_lines = 80
index_extensions = [".rs", ".ts", ".py", ".go", ".java", ".md"]

# ── Qdrant (store = "qdrant") ─────────────────────
# qdrant_url = "http://localhost:6333"
# qdrant_api_key = ""     # optional, for Qdrant Cloud

# ── ChromaDB (store = "chroma") ───────────────────
# chroma_url = "http://localhost:8000"

# ── Pinecone (store = "pinecone") ─────────────────
# pinecone_host = "https://my-index-xyz.svc.us-east1.pinecone.io"
# pinecone_api_key = ""   # or set PINECONE_API_KEY env var

[agent]
max_iterations = 10
max_memory_messages = 50
# memory_file = ".merlin-memory.jsonl"   # persist conversation across runs
default_channel = "cli"
port = 8090
```

**Secrets via environment variables:**

| Variable | Purpose |
|---|---|
| `ANTHROPIC_API_KEY` | Claude API key (`provider = "anthropic"`) |
| `OPENAI_API_KEY` | OpenAI API key (`provider = "openai"`) |
| `GEMINI_API_KEY` | Google Gemini API key (`provider = "gemini"`) |
| `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` | AWS credentials (`provider = "bedrock"`) |
| `GITHUB_TOKEN` | GitHub API token (provided automatically by Actions) |
| `GITLAB_TOKEN` | GitLab token (`$CI_JOB_TOKEN` in CI) |
| `BITBUCKET_TOKEN` | Bitbucket bearer token (or `BITBUCKET_APP_PASSWORD`) |
| `AZURE_DEVOPS_TOKEN` | Azure DevOps PAT (or `SYSTEM_ACCESSTOKEN`) |
| `GITEA_TOKEN` | Gitea API token |
| `PINECONE_API_KEY` | Pinecone API key (`store = "pinecone"`) |
| `SNYK_TOKEN` | Snyk API token (for `/snyk` command) |
| `JIRA_API_TOKEN` | Jira API token (for `/link_jira` command) |
| `LINEAR_API_KEY` | Linear API key (for `/link_linear` command) |
| `SLACK_BOT_TOKEN` | Slack bot token (for `merlin agent --channel slack`) |
| `DISCORD_BOT_TOKEN` / `DISCORD_CHANNEL_ID` | Discord bot credentials |
| `FERRET_GITHUB_SECRET` | Webhook HMAC secret (optional, bot mode) |
| `FERRET_GITLAB_SECRET` | Webhook token (optional, bot mode) |

### AI provider setup

#### Claude (API key)
```toml
[ai]
provider = "anthropic"
model = "claude-sonnet-4-6"
```
Set `ANTHROPIC_API_KEY`.

#### Claude Code CLI (no API key required)
```toml
[ai]
provider = "claude-code"
model = "claude-sonnet-4-6"
```
```bash
claude auth login                          # developer machine
claude auth login --token $CLAUDE_CODE_TOKEN  # CI headless
```
Organizations on Claude Code subscriptions skip `ANTHROPIC_API_KEY` entirely.

#### Google Gemini
```toml
[ai]
provider = "gemini"
model = "gemini-1.5-pro"
```
Set `GEMINI_API_KEY` from [Google AI Studio](https://aistudio.google.com/).

#### AWS Bedrock
```toml
[ai]
provider = "bedrock"
model = "anthropic.claude-3-5-sonnet-20241022-v2:0"
bedrock_region = "us-east-1"
```
Set `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, and optionally `AWS_SESSION_TOKEN`.

#### Ollama (local, no API key)
```toml
[ai]
provider = "ollama"
model = "llama3.1"
ollama_base_url = "http://localhost:11434"
```
Run `ollama serve` and pull a model: `ollama pull llama3.1`.

---

## CLI Usage

```bash
# Full CI review (auto-detects platform)
merlin review

# Local review of a diff file (no platform posting)
merlin review --diff path/to/changes.diff --output json

# Run any slash command from CI or locally
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
merlin run /docs readme      # generate README section
merlin run /docs api         # generate API reference
merlin run /docs adr         # generate Architecture Decision Record
merlin run /docs module      # generate module docstrings
merlin run /docs wiki        # generate wiki page
merlin run /docs             # auto-detect best doc type

# Start webhook server for bot mode
merlin webhook --port 8080
# → GitHub: POST issue_comment events to http://host:8080/webhook/github
# → GitLab: POST Note Hook events to http://host:8080/webhook/gitlab

# Autonomous agent
merlin agent                            # CLI REPL (default)
merlin agent --channel cli              # explicit CLI REPL
merlin agent --channel slack            # Slack Events API webhook on --port 8090
merlin agent --channel discord          # Discord bot via REST polling
merlin agent --task "summarise the PR"  # single-shot, non-interactive

# RAG index management
ollama pull nomic-embed-text            # one-time: pull the embedding model
merlin rag index .                      # index current directory
merlin rag index src/                   # index a subdirectory
merlin rag search "auth bypass"         # query the index
merlin rag search "SQL injection" -k 10 # return up to 10 results
merlin rag count                        # show number of indexed documents
merlin rag clear                        # delete all indexed data

# Debug: parse a diff and show file structure + priority
merlin parse-diff path/to/changes.diff
```

---

## RAG Quick Start

```toml
# merlin.toml
[rag]
enabled = true
store = "local"   # zero infra required
```

```bash
ollama pull nomic-embed-text   # one-time
merlin rag index .             # index codebase
merlin review                  # review now gets RAG context automatically
```

To switch to Qdrant for production:

```bash
docker run -p 6333:6333 qdrant/qdrant
```

```toml
[rag]
enabled = true
store = "qdrant"
qdrant_url = "http://localhost:6333"
```

---

## Building

```bash
# Development
cargo build

# Release binary
cargo build --release

# Docker
docker build -t merlin .
docker run --rm \
  -e GITHUB_TOKEN=... \
  -e ANTHROPIC_API_KEY=... \
  -e GITHUB_ACTIONS=true \
  -e GITHUB_REPOSITORY=owner/repo \
  -e GITHUB_SHA=abc123 \
  -e GITHUB_REF=refs/pull/42/merge \
  merlin review
```

---

## Running Tests

```bash
cargo test
cargo clippy -- -D warnings
```

---

## Architecture

```
CLI (clap)
  ├── ReviewEngine
  │     ├── PlatformClient (GitHub | GitLab | Bitbucket | Azure DevOps | Gitea)
  │     │     ├── get_diff()
  │     │     ├── post_inline_comment()
  │     │     └── post_summary()
  │     ├── DiffParser → Vec<FileDiff>
  │     │     └── prioritize_diffs() (token-aware)
  │     ├── AiProvider (Anthropic | OpenAI | Claude Code | Gemini | Bedrock | Ollama)
  │     │     └── review(ReviewContext) → Vec<ReviewComment>
  │     └── RagPipeline (optional)
  │           ├── OllamaEmbedder → Embedding
  │           └── VectorStore (local | memory | qdrant | chroma | pinecone)
  │                 └── search() → Vec<RetrievedDoc> → injected into AI prompt
  │
  ├── ToolRouter (slash commands)
  │     ├── /review, /describe, /ask, /improve
  │     ├── /generate_labels, /update_changelog, /add_doc, /similar_issue
  │     ├── /test, /explain, /security, /approve
  │     ├── /commit_message, /docs
  │     ├── /snyk, /coverage, /link_jira, /link_linear, /triage
  │     └── Webhook server (axum) → dispatches commands from PR comments
  │
  └── AgentRuntime (ReAct loop)
        ├── AgentMemory (ring-buffer + optional JSONL persistence)
        ├── AgentTools (all slash commands + post_comment + get_pr_info + rag_search)
        └── AgentChannel
              ├── CliChannel (stdin/stdout REPL)
              ├── SlackChannel (axum webhook + chat.postMessage)
              └── DiscordChannel (REST polling + message reply)
```

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full guide — bug reports, feature requests, development setup, coding standards, commit conventions, extension walkthroughs (new AI provider / platform / slash command / vector store / agent channel), and the PR process.

---

## License

MIT
