# Contributing to Merlin

Thank you for taking the time to contribute. Merlin is an open-source project and every contribution — bug reports, documentation improvements, new platform integrations, and code fixes — makes it better for everyone.

Please read this document before opening an issue or submitting a pull request. It saves time for you and for the maintainers.

---

## Table of contents

1. [Code of conduct](#1-code-of-conduct)
2. [Ways to contribute](#2-ways-to-contribute)
3. [Reporting bugs](#3-reporting-bugs)
4. [Reporting security vulnerabilities](#4-reporting-security-vulnerabilities)
5. [Suggesting features](#5-suggesting-features)
6. [Setting up your development environment](#6-setting-up-your-development-environment)
7. [Project structure](#7-project-structure)
8. [Making changes](#8-making-changes)
9. [Coding standards](#9-coding-standards)
10. [Testing](#10-testing)
11. [Commit message conventions](#11-commit-message-conventions)
12. [Pull request process](#12-pull-request-process)
13. [Extension guide](#13-extension-guide)
14. [Maintainer notes](#14-maintainer-notes)

---

## 1. Code of conduct

This project follows the [Contributor Covenant Code of Conduct](https://www.contributor-covenant.org/version/2/1/code_of_conduct/). By participating you agree to abide by its terms. Report unacceptable behaviour to the maintainers via the contact listed in the repository.

---

## 2. Ways to contribute

You do not need to write code to contribute meaningfully:

| Type | How |
|---|---|
| **Bug report** | Open a GitHub issue using the bug report template |
| **Feature request** | Open a GitHub issue using the feature request template |
| **Documentation** | Fix typos, expand examples, add missing content |
| **New AI provider** | See [Extension guide → AI provider](#adding-a-new-ai-provider) |
| **New platform** | See [Extension guide → Platform](#adding-a-new-platform) |
| **New slash command** | See [Extension guide → Slash command](#adding-a-new-slash-command) |
| **New vector store** | See [Extension guide → Vector store](#adding-a-new-vector-store) |
| **New agent channel** | See [Extension guide → Agent channel](#adding-a-new-agent-channel) |
| **Performance** | Profiling results + a patch are very welcome |
| **Tests** | Extra test coverage is always appreciated |
| **Translations** | User-facing strings and documentation |

If you are unsure whether your idea fits the project, open a Discussion or a draft issue before writing code.

---

## 3. Reporting bugs

**Before filing a bug:**

1. Search [existing issues](https://github.com/you/merlin/issues) to avoid duplicates.
2. Make sure you are on the latest release.
3. Try to reproduce the problem with `RUST_LOG=merlin=debug merlin ...` so you have a trace to share.

**When filing a bug, include:**

- Merlin version (`merlin --version`)
- OS and architecture
- Platform (GitHub / GitLab / Bitbucket / Azure DevOps / Gitea)
- AI provider and model (`anthropic/claude-sonnet-4-6`, `openai/gpt-4o`, etc.)
- The command you ran and the full output (redact tokens)
- What you expected to happen vs. what actually happened
- A minimal `merlin.toml` that reproduces the issue (if relevant)
- A sample diff if the bug is in parsing or review output

Use the **Bug report** issue template. Incomplete reports will be closed with a request for the missing information.

---

## 4. Reporting security vulnerabilities

**Do not open a public issue for security vulnerabilities.**

Email the maintainers directly at `security@example.com` (replace with the real address before shipping). Include:

- A description of the vulnerability and its potential impact
- Steps to reproduce
- Any proof-of-concept code

You will receive an acknowledgement within 48 hours. We aim to release a patch within 14 days for critical issues and 30 days for others. We will credit you in the release notes unless you prefer to remain anonymous.

Areas of particular concern:

- **Webhook HMAC verification** (`src/webhook/mod.rs`) — timing-safe comparison is required.
- **Token handling** — API keys and webhook secrets must never appear in logs.
- **Shell injection** — the `claude-code` provider shells out; arguments must never be user-controlled.
- **Agent tool dispatch** — agent tools must not expose arbitrary command execution.
- **RAG content injection** — retrieved documents are injected into AI prompts; validate that content cannot override the system prompt.

---

## 5. Suggesting features

Open a GitHub Discussion or an issue using the **Feature request** template. Describe:

- The problem you are trying to solve (not just the solution)
- How you expect it to work from the user's perspective
- Any alternatives you considered

Large features (new platforms, new AI providers, significant CLI changes) should be discussed and acknowledged by a maintainer **before** you start writing code, to avoid wasted effort.

---

## 6. Setting up your development environment

### Prerequisites

| Tool | Version | Install |
|---|---|---|
| Rust | stable (≥ 1.78) | `rustup update stable` |
| Cargo | bundled with Rust | — |
| Git | any recent | — |
| Ollama | optional, for RAG / Ollama AI | [ollama.com](https://ollama.com) |
| Docker | optional, for Qdrant / ChromaDB | [docker.com](https://docker.com) |
| `claude` CLI | optional, for `claude-code` provider | [claude.ai/claude-code](https://claude.ai/claude-code) |

### Clone and build

```bash
git clone https://github.com/you/merlin.git
cd merlin
cargo build          # debug build — verifies everything compiles
cargo test           # run all tests
```

### Environment variables

Create a `.env` file (never commit it) or export these in your shell:

```bash
# Required for the platform you are testing against
export GITHUB_TOKEN=ghp_...
export GITLAB_TOKEN=glpat-...

# Required for the AI provider you are using
export ANTHROPIC_API_KEY=sk-ant-...
export OPENAI_API_KEY=sk-...

# Optional: Slack/Discord agent channels
export SLACK_BOT_TOKEN=xoxb-...
export DISCORD_BOT_TOKEN=...
export DISCORD_CHANNEL_ID=...

# Simulate a GitHub Actions environment locally
export GITHUB_ACTIONS=true
export GITHUB_REPOSITORY=owner/repo
export GITHUB_SHA=abc123
export GITHUB_REF=refs/pull/42/merge
```

### Running locally against a diff file

```bash
# No platform credentials needed — output goes to stdout only
cargo run -- review --diff tests/fixtures/sample.diff --output json
```

### Tracing / debug output

```bash
RUST_LOG=merlin=debug cargo run -- review
RUST_LOG=merlin=trace cargo run -- webhook --port 8080
RUST_LOG=merlin=debug cargo run -- agent --task "describe the open PRs"
```

### Starting optional backends for RAG

```bash
# Qdrant
docker run -p 6333:6333 qdrant/qdrant

# ChromaDB
docker run -p 8000:8000 chromadb/chroma

# Ollama embedding model
ollama pull nomic-embed-text
```

---

## 7. Project structure

```
merlin/
├── Cargo.toml               # Workspace manifest and dependencies
├── Cargo.lock               # Locked dependency tree (committed)
├── config.example.toml      # Example configuration for users
├── Dockerfile               # Multi-stage Docker build
├── CONTRIBUTING.md          # This file
│
├── src/
│   ├── main.rs              # CLI entry point — clap command definitions
│   ├── lib.rs               # Library root — re-exports all public modules
│   ├── config.rs            # Config struct, TOML loading, env-var helpers
│   ├── error.rs             # MerlinError enum + Result<T> alias
│   ├── digest.rs            # Token-aware diff prioritisation
│   │
│   ├── ai/                  # AI provider backends
│   │   ├── mod.rs           # AiProvider trait · ReviewComment · build_provider()
│   │   ├── anthropic.rs     # Anthropic Messages API
│   │   ├── openai.rs        # OpenAI Chat Completions API
│   │   ├── claude_code.rs   # Shells out to the `claude` CLI
│   │   ├── gemini.rs        # Google Gemini API
│   │   ├── bedrock.rs       # AWS Bedrock (SigV4 signing)
│   │   └── ollama.rs        # Ollama local inference
│   │
│   ├── diff/                # Unified-diff parsing
│   │   ├── mod.rs
│   │   └── parser.rs        # parse_diff() → Vec<FileDiff>
│   │
│   ├── platform/            # VCS platform clients
│   │   ├── mod.rs           # PlatformClient trait · build_client() · auto-detect
│   │   ├── github.rs
│   │   ├── gitlab.rs
│   │   ├── bitbucket.rs
│   │   ├── azure_devops.rs
│   │   └── gitea.rs
│   │
│   ├── review/              # Core orchestration
│   │   ├── mod.rs
│   │   └── engine.rs        # ReviewEngine: chunk → RAG enrich → fan-out → dedup → post
│   │
│   ├── tools/               # Slash-command implementations
│   │   ├── mod.rs           # MerlinTool trait · ToolContext · route_command()
│   │   ├── ask.rs           # /ask
│   │   ├── describe.rs      # /describe
│   │   ├── improve.rs       # /improve
│   │   ├── labels.rs        # /generate_labels
│   │   ├── changelog.rs     # /update_changelog
│   │   ├── docstring.rs     # /add_doc
│   │   ├── similar_issue.rs # /similar_issue
│   │   ├── test_gen.rs      # /test
│   │   ├── explain.rs       # /explain
│   │   ├── security.rs      # /security
│   │   ├── approve.rs       # /approve
│   │   ├── commit_msg.rs    # /commit_message
│   │   ├── docs.rs          # /docs [mode]
│   │   ├── coverage.rs      # /coverage
│   │   └── triage.rs        # /triage
│   │
│   ├── integrations/        # Third-party service integrations
│   │   ├── snyk.rs          # Snyk vulnerability scanning
│   │   ├── jira.rs          # Jira issue linking
│   │   ├── linear.rs        # Linear issue linking
│   │   └── codetriage.rs    # CodeTriage issue search
│   │
│   ├── rag/                 # RAG (Retrieval-Augmented Generation) pipeline
│   │   ├── mod.rs           # Embedder + VectorStore traits · RagPipeline · build_pipeline()
│   │   ├── embedder.rs      # OllamaEmbedder → POST /api/embeddings
│   │   ├── indexer.rs       # Walk source files → chunk → index
│   │   ├── retriever.rs     # retrieve_context() · format_rag_context()
│   │   └── store/
│   │       ├── mod.rs       # cosine_similarity · doc_id_to_u64
│   │       ├── local.rs     # JSONL flat file with brute-force cosine search
│   │       ├── memory.rs    # Ephemeral in-memory store
│   │       ├── qdrant.rs    # Qdrant REST API
│   │       ├── chroma.rs    # ChromaDB REST API
│   │       └── pinecone.rs  # Pinecone REST API
│   │
│   ├── agent/               # Autonomous ReAct-loop agent
│   │   ├── mod.rs           # Core types: AgentTool · AgentChannel · AgentContext
│   │   ├── memory.rs        # Ring-buffer conversation memory + JSONL persistence
│   │   ├── runtime.rs       # ReAct loop · parse_tool_calls() · run_channel()
│   │   ├── tools.rs         # Built-in agent tools (slash commands + RAG + platform)
│   │   └── channels/
│   │       ├── mod.rs       # Channel type exports
│   │       ├── cli.rs       # stdin/stdout REPL
│   │       ├── slack.rs     # Slack Events API webhook + chat.postMessage
│   │       └── discord.rs   # Discord REST polling + message reply
│   │
│   ├── dashboard/           # Admin dashboard (optional)
│   │   └── mod.rs
│   │
│   ├── audit/               # Audit logging
│   │   └── mod.rs
│   │
│   └── webhook/
│       └── mod.rs           # Axum server — GitHub & GitLab webhook handlers
│
└── tests/
    └── fixtures/            # Sample diffs used in tests
        ├── sample.diff
        └── large.diff
```

### Key types at a glance

| Type | File | Description |
|---|---|---|
| `Config` | `src/config.rs` | Top-level TOML config (AI, review, platform, rag, agent sections) |
| `AiProvider` | `src/ai/mod.rs` | Async trait: `review()` + `generate()` |
| `ReviewContext` | `src/ai/mod.rs` | Input to `AiProvider::review()` — file path + diff hunk |
| `ReviewComment` | `src/ai/mod.rs` | Single comment: file, line, severity, category, body, suggestion |
| `PlatformClient` | `src/platform/mod.rs` | Async trait: diff fetch, inline comment posting, label/file ops |
| `MerlinTool` | `src/tools/mod.rs` | Async trait for slash commands: `name()` + `run()` |
| `ToolContext` | `src/tools/mod.rs` | Shared context passed into every tool (AI + platform + arg) |
| `ReviewEngine` | `src/review/engine.rs` | Orchestrates the full review cycle; optionally holds a `RagPipeline` |
| `Embedder` | `src/rag/mod.rs` | Async trait: `embed(text) → Embedding` |
| `VectorStore` | `src/rag/mod.rs` | Async trait: `upsert`, `search`, `clear`, `count` |
| `RagPipeline` | `src/rag/mod.rs` | Ties embedder + store + config; `retrieve()` + `index_documents()` |
| `AgentTool` | `src/agent/mod.rs` | Async trait: `definition()` + `call(params, ctx)` |
| `AgentChannel` | `src/agent/mod.rs` | Async trait: `recv()` + `send()` + `send_to()` |
| `AgentContext` | `src/agent/mod.rs` | Cloneable context shared across agent tool calls (AI + platform + config) |
| `AgentRuntime` | `src/agent/runtime.rs` | Drives the ReAct loop and channel dispatch |
| `AgentMemory` | `src/agent/memory.rs` | Ring-buffer conversation history with optional persistence |

---

## 8. Making changes

### Branch naming

```
feat/<short-description>       # new features
fix/<short-description>        # bug fixes
docs/<short-description>       # documentation only
refactor/<short-description>   # no behaviour change
test/<short-description>       # tests only
chore/<short-description>      # tooling, CI, dependency bumps
```

### Workflow

1. Fork the repository and create your branch from `main`.
2. Make your changes (see [Coding standards](#9-coding-standards)).
3. Add or update tests (see [Testing](#10-testing)).
4. Run the full quality gate locally (see below).
5. Push and open a pull request.

### Local quality gate (must pass before pushing)

```bash
cargo fmt --check              # formatting
cargo clippy -- -D warnings    # lints (zero warnings required)
cargo test                     # all tests
```

Run formatting fixes with:

```bash
cargo fmt
```

---

## 9. Coding standards

### General

- Write idiomatic Rust. Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/).
- Prefer `thiserror` for library errors; propagate with `?`. Do not use `.unwrap()` in non-test code.
- Keep functions short and focused. If a function needs a long comment to explain what it does, consider splitting it.
- Avoid unnecessary `clone()` — borrow where possible. When cloning is required for async safety (e.g. across `await` points), prefer `Arc` over deep cloning.
- Use `tracing::info!` / `warn!` / `debug!` rather than `eprintln!`.

### Error handling

All errors flow through `MerlinError` in `src/error.rs`. Add a new variant when introducing a new failure category. Use the most specific variant available:

```rust
// Good
Err(MerlinError::Platform(format!("rate limited: {status}")))

// Avoid — loses context
Err(MerlinError::Other("something went wrong".to_string()))
```

### Async

The project uses `tokio` with `#[tokio::main]`. Use `JoinSet` for fan-out concurrency (see `ReviewEngine::run_ai_reviews`). Avoid blocking calls (`std::thread::sleep`, blocking I/O) on async tasks; use `tokio::time::sleep` and `tokio::fs` instead.

For agent tools: all context accessed across `await` points must be `Clone` or wrapped in `Arc`. The `AgentContext` type is `Clone` for this reason; clone it before spawning async work.

### Dependencies

- Minimise new dependencies. Open a discussion first if you need to add one.
- Prefer crates already in the dependency tree when possible.
- Never add a dependency with a GPL or copyleft license without maintainer sign-off.

### API keys and secrets

- Never log API keys, tokens, or webhook secrets — not even at `TRACE` level.
- Always read secrets from environment variables, never from config files committed to source control.

---

## 10. Testing

### Running tests

```bash
cargo test                          # all unit + integration tests
cargo test <module_name>            # tests in a specific module
cargo test -- --nocapture           # show println! output while testing
cargo test rag::                    # run only RAG tests
cargo test agent::                  # run only agent tests
```

### What to test

Every non-trivial change should include tests. The project uses in-module unit tests (`#[cfg(test)]` blocks) and integration tests under `tests/`.

| Change type | Expected test coverage |
|---|---|
| New AI provider | Unit test for prompt construction; mock HTTP responses |
| New platform client | Unit test env-var detection; mock API responses |
| New slash command | Unit test `run()` with mock AI + platform |
| New vector store | Unit test `upsert`, `search`, `clear`, `count` with known data |
| New agent channel | Unit test `recv()` routing and `send_to()` formatting |
| New agent tool | Unit test `definition()` metadata; async `call()` with mock context |
| Diff parser change | Unit test new or changed parsing behaviour |
| Config change | Unit test default values and TOML round-trip |
| Bug fix | Regression test that fails before the fix and passes after |

### Test fixtures

Diff fixtures live in `tests/fixtures/`. Add new `.diff` files there if your change requires a new parsing scenario.

### Mocking

Use trait objects (`Box<dyn AiProvider>`, `Box<dyn PlatformClient>`, `Box<dyn VectorStore>`) for mocking in tests — implement a minimal stub that returns controlled values. Do not make real network calls in unit tests.

The `MemoryStore` (`src/rag/store/memory.rs`) is the preferred vector store for RAG unit tests because it requires no setup and resets between tests.

---

## 11. Commit message conventions

Use [Conventional Commits](https://www.conventionalcommits.org/). This enables automated changelog generation and semantic versioning.

```
<type>(<optional scope>): <short summary in present tense>

<optional longer description>

<optional footer — breaking changes, issue refs>
```

### Types

| Type | When to use |
|---|---|
| `feat` | New feature visible to users |
| `fix` | Bug fix |
| `refactor` | Code change that neither fixes a bug nor adds a feature |
| `test` | Adding or correcting tests |
| `docs` | Documentation only |
| `chore` | Tooling, CI, dependency bumps, build changes |
| `perf` | Performance improvement |

### Examples

```
feat(rag): add Pinecone vector store backend

feat(agent): add Slack channel with thread-aware replies

fix(platform/github): handle 404 when PR has no diff yet

refactor(review): extract chunk boundary logic into separate function

docs: update README with RAG quick-start guide

chore: bump reqwest to 0.12.5
```

### Breaking changes

Append `!` after the type and add a `BREAKING CHANGE:` footer:

```
feat(config)!: rename chunk_lines to diff_chunk_lines

BREAKING CHANGE: The `chunk_lines` config key has been renamed to
`diff_chunk_lines`. Update your merlin.toml accordingly.
```

---

## 12. Pull request process

### Before you open a PR

- [ ] Your branch is up to date with `main` (`git rebase origin/main`)
- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes
- [ ] New behaviour is covered by tests
- [ ] You have updated relevant documentation (README, config examples, this file)

### PR description

Use this template:

```markdown
## What
<!-- One or two sentences describing what this PR does. -->

## Why
<!-- The problem this solves or the user need it addresses. -->

## How
<!-- Brief description of the approach. Call out any non-obvious decisions. -->

## Testing
<!-- How did you verify this? What test cases are included? -->

## Breaking changes
<!-- List any breaking changes and migration steps, or write "None". -->
```

### Review process

- A maintainer will review your PR within 5 business days.
- At least one maintainer approval is required to merge.
- Automated CI must be green before merging.
- Prefer squash-merge for small changes; merge commits for large features with meaningful history.
- If your PR sits unreviewed for more than 7 days, feel free to ping in the comments.

### Draft PRs

Open a draft PR early if you want early feedback on direction. Mark it ready for review once the quality gate passes.

---

## 13. Extension guide

### Adding a new AI provider

1. **Implement the trait.** Create `src/ai/<name>.rs`:

   ```rust
   use async_trait::async_trait;
   use crate::ai::{AiProvider, ReviewComment, ReviewContext};
   use crate::config::AiConfig;
   use crate::error::Result;

   pub struct MyProvider {
       api_key: String,
       config: AiConfig,
   }

   impl MyProvider {
       pub fn new(api_key: String, config: AiConfig) -> Self {
           Self { api_key, config }
       }
   }

   #[async_trait]
   impl AiProvider for MyProvider {
       async fn review(&self, ctx: &ReviewContext) -> Result<Vec<ReviewComment>> {
           // Call your API, parse the JSON response into Vec<ReviewComment>
           todo!()
       }

       async fn generate(&self, system: &str, user: &str) -> Result<String> {
           // Call your API, return raw text
           todo!()
       }
   }
   ```

2. **Add a config variant.** In `src/config.rs`:

   ```rust
   pub enum AiProviderType {
       Anthropic,
       Openai,
       ClaudeCode,
       Gemini,
       Bedrock,
       Ollama,
       MyProvider,   // add this
   }
   ```

   The TOML value is derived from the variant name in kebab-case: `provider = "my-provider"`.

3. **Wire up the factory.** In `src/ai/mod.rs`:

   ```rust
   pub mod myprovider;   // expose the module

   // in build_provider():
   AiProviderType::MyProvider => {
       let key = std::env::var("MY_PROVIDER_API_KEY")
           .map_err(|_| MerlinError::EnvVar("MY_PROVIDER_API_KEY".to_string()))?;
       Ok(Box::new(myprovider::MyProvider::new(key, cfg.clone())))
   }
   ```

4. **Document it.** Add the new provider to the README configuration table and the env-var secrets table.

5. **Test it.** Add unit tests in `src/ai/<name>.rs` covering prompt construction and response parsing. Use a mock HTTP server (e.g., `wiremock`) rather than real API calls.

---

### Adding a new platform

1. **Implement the trait.** Create `src/platform/<name>.rs` and implement all methods of `PlatformClient`:

   ```rust
   pub struct MyPlatformClient { /* token, repo, pr_number */ }

   #[async_trait]
   impl PlatformClient for MyPlatformClient {
       async fn get_diff(&self) -> Result<String> { todo!() }
       async fn post_inline_comment(&self, comment: &ReviewComment) -> Result<()> { todo!() }
       async fn post_summary(&self, summary: &str) -> Result<()> { todo!() }
       async fn get_pr_info(&self) -> Result<PrInfo> { todo!() }
       async fn update_description(&self, title: &str, body: &str) -> Result<()> { todo!() }
       async fn set_labels(&self, labels: &[String]) -> Result<()> { todo!() }
       async fn list_issues(&self, limit: usize) -> Result<Vec<Issue>> { todo!() }
       async fn post_code_suggestions(&self, suggestions: &[InlineCodeSuggestion]) -> Result<()> { todo!() }
       async fn update_file(&self, path: &str, content: &str, message: &str, sha: Option<&str>) -> Result<()> { todo!() }
       async fn get_file(&self, path: &str) -> Result<Option<(String, String)>> { todo!() }
   }
   ```

2. **Add a config variant** in `src/config.rs` under `PlatformType`.

3. **Add to the factory** in `src/platform/mod.rs` `build_client()`.

4. **Add auto-detection** in `detect_platform()` — check a unique environment variable set by that platform's CI system.

5. **Add a token helper** on `Config` (following the pattern of `Config::github_token()`).

6. **Document it** in the README and add the token env-var to the secrets table.

---

### Adding a new slash command

1. **Implement the trait.** Create `src/tools/<name>.rs`:

   ```rust
   use async_trait::async_trait;
   use crate::tools::{MerlinTool, ToolContext};
   use crate::error::Result;

   pub struct MyTool;

   #[async_trait]
   impl MerlinTool for MyTool {
       fn name(&self) -> &'static str { "my_tool" }

       async fn run(&self, ctx: &ToolContext) -> Result<String> {
           let diff = ctx.platform.get_diff().await?;
           // ctx.arg holds the text after the command (e.g. for /ask "why?")
           let result = ctx.ai.generate("your system prompt", &diff).await?;
           ctx.platform.post_summary(&result).await?;
           Ok(result)
       }
   }
   ```

2. **Register the command.** In `src/tools/mod.rs`:

   ```rust
   pub mod mycommand;   // expose module

   // in route_command():
   "/my_command" => Ok(Box::new(mycommand::MyTool)),
   ```

3. **Expose it to the agent.** The agent picks up all slash commands automatically via `builtin_tools()` in `src/agent/tools.rs`. Add an entry to the `slash_no_arg` or `slash_arg` list as appropriate.

4. **Document the command** in the README slash-commands table.

5. **Test it.** Add unit tests that exercise `run()` with stub implementations of `AiProvider` and `PlatformClient`.

---

### Adding a new vector store

1. **Implement the trait.** Create `src/rag/store/<name>.rs`:

   ```rust
   use async_trait::async_trait;
   use crate::error::Result;
   use crate::rag::{Document, Embedding, RetrievedDoc, VectorStore};

   pub struct MyStore { /* connection details */ }

   #[async_trait]
   impl VectorStore for MyStore {
       async fn ensure_collection(&self, collection: &str, dimension: usize) -> Result<()> {
           todo!()
       }
       async fn upsert(&self, collection: &str, docs: &[(Document, Embedding)]) -> Result<()> {
           todo!()
       }
       async fn search(
           &self, collection: &str, query_vec: &Embedding,
           limit: usize, min_score: f32,
       ) -> Result<Vec<RetrievedDoc>> {
           todo!()
       }
       async fn clear(&self, collection: &str) -> Result<()> { todo!() }
       async fn count(&self, collection: &str) -> Result<usize> { todo!() }
   }
   ```

2. **Register the module** in `src/rag/store/mod.rs`:

   ```rust
   pub mod mystore;
   ```

3. **Add a config variant** in `src/config.rs` under `VectorStoreType`.

4. **Add any new config fields** for connection details to `RagConfig`.

5. **Wire up the factory** in `src/rag/mod.rs` `build_pipeline()`:

   ```rust
   VectorStoreType::MyStore => Box::new(store::mystore::MyStore::new(...)),
   ```

6. **Document it** in the README RAG table and the `[rag]` config example.

7. **Test it.** Add unit tests in `src/rag/store/<name>.rs` covering upsert → search → clear using a real or mock backend. For unit tests without a running server, prefer the `MemoryStore` as a reference.

---

### Adding a new agent channel

1. **Implement the trait.** Create `src/agent/channels/<name>.rs`:

   ```rust
   use async_trait::async_trait;
   use crate::agent::{AgentChannel, AgentTask};
   use crate::error::Result;

   pub struct MyChannel { /* connection details */ }

   impl MyChannel {
       pub async fn new(/* config */) -> Result<Self> { todo!() }
   }

   #[async_trait]
   impl AgentChannel for MyChannel {
       fn name(&self) -> &str { "mychannel" }

       async fn recv(&mut self) -> Option<AgentTask> {
           // Block until a message arrives; return None to signal shutdown
           todo!()
       }

       async fn send(&self, response: &str) {
           // Send a reply to the last received message's source
           todo!()
       }

       // Override send_to() if your channel supports threading:
       async fn send_to(&self, response: &str, thread_id: &str) {
           todo!()
       }
   }
   ```

2. **Export the type** in `src/agent/channels/mod.rs`:

   ```rust
   pub mod mychannel;
   pub use mychannel::MyChannel;
   ```

3. **Add to the CLI.** In `src/main.rs`, add a match arm in the `Commands::Agent` handler:

   ```rust
   "mychannel" => {
       let mut ch = MyChannel::new(port).await?;
       runtime.run_channel(&mut ch).await?;
   }
   ```

4. **Add any required env-vars** to the secrets table in the README.

5. **Test it.** Unit-test `recv()` routing and `send_to()` formatting. For integration, the CLI channel tests are a good model.

---

## 14. Maintainer notes

This section is for project maintainers.

### Merging PRs

- Require CI green + at least one approval.
- Prefer **Squash and merge** for small/single-purpose PRs.
- Prefer **Merge commit** for large features with multiple meaningful commits.
- Never merge your own PR without a review, except for trivial typo fixes.

### Cutting a release

1. Update the version in `Cargo.toml`.
2. Run `cargo build --release` and verify the binary works.
3. Update `CHANGELOG.md` (use conventional commit history).
4. Tag: `git tag -s v0.x.y -m "v0.x.y"`.
5. Push tag: `git push origin v0.x.y`.
6. CI publishes the GitHub release and cross-compiled binaries.

### Dependency updates

Run `cargo update` monthly and review `cargo audit` output. Pin patch versions only when necessary to work around a known bug.

### Stale issues

Issues with no activity for 60 days will be labelled `stale`. If there is still no response after 14 more days, they will be closed. Maintainers may reopen any closed issue with new information.
