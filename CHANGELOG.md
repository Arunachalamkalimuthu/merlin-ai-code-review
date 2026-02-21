# Changelog

All notable changes to Merlin are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

---

## [0.1.4] — 2026-02-21

### Fixed

- **`cargo fmt`** — auto-formatted entire codebase (68 files); CI `cargo fmt --check` now passes
- **`cargo deny` schema** — migrated `deny.toml` to cargo-deny v2: removed deprecated `unsound`,
  `unlicensed`, and `licenses.deny` fields; changed `unmaintained = "warn"` → `"none"`
- **License allowlist** — added `Unicode-3.0` (icu_* crates via reqwest → idna) and
  `CDLA-Permissive-2.0` (webpki-roots via reqwest) to `deny.toml`; `cargo deny check` now passes
- **Clippy `--all-targets --all-features`** — fixed three additional lint errors caught only when
  compiling test targets: `format_collect` in `src/rag/indexer.rs`, `single_match` in
  `src/update.rs`, and `dead_code` in `tests/common/mod.rs`

---

## [0.1.3] — 2026-02-21

### Added

- **`merlin-ui`** — Ink-powered interactive terminal UI (React for CLIs, used
  by Claude Code, GitHub Copilot CLI, Cloudflare Wrangler):
  - `merlin-ui review` — dots spinner while reviewing, then bordered comment
    cards with severity colours, category tags, file:line location, body text,
    and green suggestion blocks; summary bar with per-severity emoji counts
  - `merlin-ui review --diff <file>` — local diff mode
  - `merlin-ui agent` — full interactive REPL with ink-text-input prompt,
    scrollable conversation history, magenta response boxes, thinking spinner
  - `merlin-ui update` — streams `merlin self-update` output live, green ✔ / red ✖
  - `merlin-ui update --check` — check only, no download
  - `merlin-ui status` — cyan bordered panel: installed version, binary path,
    latest release from GitHub, update prompt when outdated
  - Components: `Header`, `SeverityBadge`, `CommentCard`, `Summary`
  - Stack: React 18 + Ink 5 + ink-spinner + ink-text-input + execa + meow + TypeScript

---

## [0.1.2] — 2026-02-21

### Added

- **`merlin self-update`** — self-update command that downloads and installs the
  latest release binary for the current platform automatically:
  - Queries the GitHub Releases API for the latest version
  - Compares against the running version (semver-aware, ignores pre-release suffixes)
  - Downloads the platform-correct binary (`darwin-arm64`, `darwin-amd64`,
    `linux-amd64-musl`, `linux-arm64-musl`, `windows-amd64.exe`)
  - Verifies the SHA-256 checksum before replacing the binary
  - Atomically replaces the running executable via rename (safe on all platforms)
  - `merlin self-update --check` — check for updates without downloading
  - `merlin self-update --force` — re-install even if already on the latest version
  - `merlin update` — alias for `merlin self-update`

---

## [0.1.1] — 2026-02-21

### Fixed

- **Gemini API key security** — API key moved from URL query parameter to
  `x-goog-api-key` header, preventing it from appearing in server access logs
  and HTTP client traces

### Changed

- **Shared AI response parser** — extracted `src/ai/response.rs` with a single
  `parse_review_response()` used by all 6 AI providers; handles bare arrays,
  markdown-fenced arrays, and wrapped objects (`"comments"`, `"reviews"`,
  `"issues"`, `"results"` keys)
- **Pre-compiled regex** — `COMMAND_RE` in `tools/mod.rs` now compiled once at
  startup via `OnceLock<Regex>` instead of on every PR comment event

### Added

- **Comprehensive rustdoc** — full `//!` module docs, `///` item docs, `# Examples`,
  `# Errors`, and intra-doc links across all public modules following the
  Tangram Vision Rustdoc best practices guide
- **`INSTALLATION.md`** — step-by-step installation guide for macOS, Linux,
  Windows, Docker, and all major CI/CD platforms with troubleshooting section

### Removed

- **Docusaurus website** — docs site moved to a separate repository

---

## [0.1.0] — 2026-02-21

Initial public release of Merlin — a self-hosted, open-source AI code review CLI.

### Added

#### Core review engine
- Unified diff parser producing `FileDiff` structs with line mappings
- Token-aware file prioritisation (security-sensitive files ranked first)
- Concurrent Tokio fan-out per diff chunk for fast reviews
- Optional second-pass "Reflect & Review" mode to filter false positives
- `merlin review` — full CI review with inline comments and a PR summary
- `merlin review --diff <file>` — local offline mode, no platform posting required
- `--output json` flag for machine-readable review results

#### AI providers (6)
- **Anthropic Claude** — `claude-sonnet-4-6` default; supports all Claude models
- **OpenAI** — `gpt-4o`, `gpt-4o-mini`, `gpt-4-turbo`
- **Claude Code CLI** — uses the Claude Code subscription, no separate API key
- **Google Gemini** — `gemini-1.5-pro`, `gemini-1.5-flash`
- **AWS Bedrock** — Anthropic Claude models via Bedrock
- **Ollama** — any local model (`llama3.1`, `codellama`, `mistral`, etc.)

#### VCS platform clients (5)
- **GitHub** — PR Review Comments API, auto-detected from `GITHUB_ACTIONS`
- **GitLab** — MR Diff Notes API, auto-detected from `GITLAB_CI`
- **Bitbucket** — inline comments, auto-detected from `BITBUCKET_BUILD_NUMBER`
- **Azure DevOps** — thread comments, auto-detected from `TF_BUILD`
- **Gitea** — issue comments, auto-detected from `GITEA_ACTIONS`

#### Slash commands (19)
- `/review` — full code review
- `/spec` — generate a 10-section technical specification and set as PR description
- `/describe` — auto-generate PR title and description
- `/ask <question>` — Q&A about the PR diff
- `/improve` — inline code improvement suggestions
- `/security` — dedicated security scan (secrets + OWASP Top 10)
- `/test` — generate unit test stubs for changed code
- `/explain` — plain-English walkthrough of the PR
- `/approve` — AI-assisted review verdict (approve / request changes / comment)
- `/generate_labels` — auto-label PRs based on diff content and size
- `/update_changelog` — prepend a CHANGELOG.md entry
- `/add_doc` — generate missing docstrings
- `/similar_issue` — find related open issues
- `/commit_message` — generate 3 conventional commit message options
- `/snyk` — scan changed dependencies against the Snyk vulnerability database
- `/coverage` — analyse test coverage for changed files
- `/link_jira` — find and link related Jira issues
- `/link_linear` — find and link related Linear issues
- `/triage` — find similar open issues on CodeTriage
- `/docs [mode]` — documentation generator (readme / api / adr / module / wiki / auto)

#### RAG (context-aware reviews)
- `merlin rag index` — crawl and embed the codebase into a vector store
- `merlin rag search` — query the index from the CLI
- `merlin rag count` / `merlin rag clear` — index management
- **Ollama embedder** — local, free, private (`nomic-embed-text` default)
- **OpenAI embedder** — CI-friendly, works on any runner (`text-embedding-3-small` default)
- **Local JSONL store** — zero-infrastructure flat file store
- **Memory store** — ephemeral in-process store for testing
- **Qdrant store** — high-performance vector database (self-hosted or Qdrant Cloud)
- **ChromaDB store** — open-source alternative
- **Pinecone store** — managed cloud vector database

#### Autonomous agent
- `merlin agent` — interactive CLI REPL with ReAct-loop reasoning
- `merlin agent --channel slack` — Slack Events API integration
- `merlin agent --channel discord` — Discord bot integration
- `merlin agent --task "..."` — single-shot mode for CI
- Configurable ring-buffer memory with optional JSONL persistence across sessions

#### Bot / webhook mode
- `merlin webhook` — axum-based webhook listener for GitHub and GitLab
- Responds to `@merlin /command` comments in PRs automatically
- HMAC-SHA256 signature verification for GitHub and GitLab payloads

#### Configuration
- `merlin.toml` TOML config file with full schema
- `config.example.toml` — annotated example shipped with the binary
- All secrets via environment variables (never in config files)
- `RUST_LOG` env var for log level control

#### CI/CD
- GitHub Actions workflow example (`.github/workflows/review.yml`)
- GitLab CI example (`.gitlab-ci.yml.example`)
- Cross-platform installer: `install.sh` (Linux/macOS) and `install.ps1` (Windows)

#### Release infrastructure
- Multi-platform release workflow: 7 binary targets (Linux amd64/arm64 glibc+musl, macOS amd64/arm64, Windows amd64)
- Docker multi-arch images (linux/amd64 + linux/arm64) published to GHCR
- SHA-256 checksums for every release artifact
- Nightly builds from `main`
- Docusaurus v3 documentation site (`website/`) with GitHub Pages deploy workflow

---

[Unreleased]: https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/tag/v0.1.0
