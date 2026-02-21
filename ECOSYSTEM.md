# Merlin Ecosystem

A full map of everything that integrates with, extends, or works alongside Merlin.

---

## AI Providers

Merlin works with any of these AI backends. Switch by changing one line in `merlin.toml`.

| Provider | Model | `provider` value | Key required |
|---|---|---|---|
| **Anthropic Claude** | `claude-sonnet-4-6`, `claude-opus-4-6`, `claude-haiku-4-5` | `"anthropic"` | `ANTHROPIC_API_KEY` |
| **OpenAI** | `gpt-4o`, `gpt-4o-mini`, `gpt-4-turbo` | `"openai"` | `OPENAI_API_KEY` |
| **Claude Code CLI** | `claude-sonnet-4-6` | `"claude-code"` | none (subscription) |
| **Google Gemini** | `gemini-1.5-pro`, `gemini-1.5-flash` | `"gemini"` | `GEMINI_API_KEY` |
| **AWS Bedrock** | `anthropic.claude-sonnet-4-6-*` | `"bedrock"` | AWS credentials |
| **Ollama** | `llama3.1`, `mistral`, `codellama`, any | `"ollama"` | none (local) |

---

## VCS Platforms

Merlin auto-detects your platform from CI environment variables and posts comments natively.

| Platform | Auto-detect env var | Comment API |
|---|---|---|
| **GitHub** | `GITHUB_ACTIONS=true` | Pull Request Review Comments |
| **GitLab** | `GITLAB_CI=true` | MR Notes + Diff Notes |
| **Bitbucket** | `BITBUCKET_BUILD_NUMBER` | Inline comments |
| **Azure DevOps** | `TF_BUILD=True` | Thread comments |
| **Gitea** | `GITEA_ACTIONS=true` | Issue comments |

---

## CI/CD Platforms

Merlin runs as a single binary in any CI environment — no runtime dependencies.

| Platform | Example config |
|---|---|
| **GitHub Actions** | [`.github/workflows/review.yml`](.github/workflows/review.yml) |
| **GitLab CI** | [`.gitlab-ci.yml.example`](.gitlab-ci.yml.example) |
| **Bitbucket Pipelines** | `curl install.sh \| sh && merlin review` |
| **Azure Pipelines** | `curl install.sh \| sh && merlin review` |
| **CircleCI** | `curl install.sh \| sh && merlin review` |
| **Jenkins** | `curl install.sh \| sh && merlin review` |
| **Drone CI** | Docker image `ghcr.io/arunachalamkalimuthu/merlin-ai-code-review:latest` |
| **Woodpecker CI** | Docker image |

---

## Embedders (RAG)

Used to convert code into vectors for context-aware reviews. Set via `[rag] embedder`.

| Embedder | `embedder` value | Best for | Model |
|---|---|---|---|
| **Ollama** | `"ollama"` | Local dev — free, private | `nomic-embed-text` (recommended) |
| **OpenAI** | `"openai"` | CI/CD pipelines — works anywhere | `text-embedding-3-small` (recommended) |

### Ollama embedding models

| Model | Dimensions | Size | Notes |
|---|---|---|---|
| `nomic-embed-text` | 768 | 274 MB | Best balance — default |
| `mxbai-embed-large` | 1024 | 670 MB | Higher quality |
| `all-minilm` | 384 | 45 MB | Fastest |

### OpenAI embedding models

| Model | Dimensions | Cost per 1M tokens |
|---|---|---|
| `text-embedding-3-small` | 1536 | $0.020 |
| `text-embedding-3-large` | 3072 | $0.130 |
| `text-embedding-ada-002` | 1536 | $0.100 (legacy) |

---

## Vector Stores (RAG)

Persists embeddings and runs similarity search. Set via `[rag] store`.

| Store | `store` value | Setup | Best for |
|---|---|---|---|
| **Local JSONL** | `"local"` | None — flat file | Dev, CI with caching |
| **Memory** | `"memory"` | None — ephemeral | Testing |
| **Qdrant** | `"qdrant"` | `docker run -p 6333:6333 qdrant/qdrant` | Production self-hosted |
| **ChromaDB** | `"chroma"` | `docker run -p 8000:8000 chromadb/chroma` | Open-source alternative |
| **Pinecone** | `"pinecone"` | cloud.pinecone.io | Fully managed cloud |

---

## Notification Channels (Agent Mode)

| Channel | How to enable | Config |
|---|---|---|
| **CLI REPL** | `merlin agent` | default |
| **Slack** | `merlin agent --channel slack` | `SLACK_BOT_TOKEN` |
| **Discord** | `merlin agent --channel discord` | `DISCORD_BOT_TOKEN` + `DISCORD_CHANNEL_ID` |

---

## Third-party Service Integrations

These are activated by specific slash commands.

| Service | Command | Key required |
|---|---|---|
| **Snyk** | `/snyk` | `SNYK_TOKEN` |
| **Jira** | `/link_jira` | `JIRA_TOKEN` |
| **Linear** | `/link_linear` | `LINEAR_API_KEY` |
| **CodeTriage** | `/triage` | none |
| **Coveralls** *(planned)* | `/coverage` | `COVERALLS_TOKEN` |

---

## Slash Commands

All commands can be triggered from PR comments (`@merlin /command`) or directly from CI (`merlin run /command`).

| Command | Category | Output |
|---|---|---|
| `/review` | Review | Inline comments + summary |
| `/spec` | Documentation | PR description (technical spec) |
| `/describe` | Documentation | PR title + description |
| `/ask <question>` | Q&A | PR comment |
| `/improve` | Review | Suggestion blocks |
| `/security` | Security | Inline report |
| `/test` | Testing | Unit test stubs |
| `/explain` | Documentation | Plain-English walkthrough |
| `/approve` | Review | PR review verdict |
| `/generate_labels` | Automation | PR labels |
| `/update_changelog` | Automation | CHANGELOG.md commit |
| `/add_doc` | Documentation | Docstring suggestions |
| `/similar_issue` | Automation | Related issues table |
| `/commit_message` | Automation | 3 conventional commit options |
| `/snyk` | Security | Vulnerability report |
| `/coverage` | Testing | Coverage analysis |
| `/link_jira` | Integrations | Jira issue links |
| `/link_linear` | Integrations | Linear issue links |
| `/triage` | Integrations | CodeTriage matches |
| `/docs [mode]` | Documentation | readme / api / adr / module / wiki |

---

## Installation Methods

| Method | Command |
|---|---|
| **Shell (Linux/macOS)** | `curl -fsSL https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest/download/install.sh \| sh` |
| **PowerShell (Windows)** | `irm https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest/download/install.ps1 \| iex` |
| **Docker** | `docker pull ghcr.io/arunachalamkalimuthu/merlin-ai-code-review:latest` |
| **Cargo** | `cargo install merlin` *(coming soon)* |
| **From source** | `cargo build --release` |

---

## Docker Images

Published to GitHub Container Registry on every release.

| Tag | Description |
|---|---|
| `latest` | Latest stable release |
| `0.1.0` | Pinned version |
| `0.1` | Latest patch in minor |
| `0` | Latest in major |

```bash
docker pull ghcr.io/arunachalamkalimuthu/merlin-ai-code-review:latest
docker pull ghcr.io/arunachalamkalimuthu/merlin-ai-code-review:0.1.0
```

---

## Release Artifacts

Each release ships pre-built binaries for all major platforms.

| Asset | Platform |
|---|---|
| `merlin-linux-amd64` | Linux x86-64 (glibc) |
| `merlin-linux-arm64` | Linux ARM64 (glibc) |
| `merlin-linux-amd64-musl` | Linux x86-64 (fully static) |
| `merlin-linux-arm64-musl` | Linux ARM64 (fully static) |
| `merlin-darwin-amd64` | macOS Intel |
| `merlin-darwin-arm64` | macOS Apple Silicon |
| `merlin-windows-amd64.exe` | Windows x86-64 |
| `install.sh` | Shell installer (auto-detects platform) |
| `install.ps1` | PowerShell installer |

Each binary ships with a companion `.sha256` checksum file.

---

## Comparison with Similar Tools

| Feature | Merlin | CodeRabbit | Qodo | GitHub Copilot |
|---|---|---|---|---|
| Self-hosted | ✅ | ❌ | ❌ | ❌ |
| Open source | ✅ | ❌ | ❌ | ❌ |
| Bring your own API key | ✅ | ❌ | ❌ | ❌ |
| Local LLM (Ollama) | ✅ | ❌ | ❌ | ❌ |
| RAG codebase context | ✅ | ✅ | ✅ | ✅ |
| Slash commands | ✅ (19) | ✅ | ✅ | Limited |
| Auto-spec generation | ✅ | ❌ | ❌ | ❌ |
| Autonomous agent | ✅ | ❌ | ❌ | ❌ |
| GitHub + GitLab + Bitbucket | ✅ | ✅ | ✅ | GitHub only |
| Azure DevOps + Gitea | ✅ | ❌ | ❌ | ❌ |
| Pricing | Free + your API costs | $15–$29/seat/mo | $19/seat/mo | $10–$19/seat/mo |

---

## Links

| Resource | URL |
|---|---|
| Documentation | https://merlin-review.com/ |
| GitHub Repository | https://github.com/Arunachalamkalimuthu/merlin-ai-code-review |
| Releases | https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases |
| Docker Images | https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/pkgs/container/merlin-ai-code-review |
| Issues | https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/issues |
| Contributing | [CONTRIBUTING.md](CONTRIBUTING.md) |
