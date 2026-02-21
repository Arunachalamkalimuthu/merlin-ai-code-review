//! # Merlin — AI-powered self-hosted code review
//!
//! Merlin parses pull-request diffs, sends them to a configurable AI provider,
//! and posts structured inline comments plus a summary back to the PR/MR.
//!
//! # Features
//!
//! - **Multi-provider AI** — Anthropic Claude, OpenAI GPT-4o, Google Gemini,
//!   AWS Bedrock, Azure OpenAI, Ollama, Claude Code CLI
//! - **Multi-platform VCS** — GitHub, GitLab, Bitbucket, Azure DevOps, Gitea
//! - **Slash commands** — `/review`, `/security`, `/spec`, `/describe`, `/ask`, and more
//! - **RAG context injection** — embed your codebase, retrieve similar code per diff chunk
//! - **Reflect & Review** — optional second AI pass to filter false positives
//! - **Autonomous agent** — ReAct-loop bot reachable via CLI, Slack, or Discord
//! - **Audit log** — every action appended to a JSONL file for compliance
//!
//! # Quick Start
//!
//! ```no_run
//! use std::sync::Arc;
//! use merlin::config::Config;
//! use merlin::ai::build_provider;
//! use merlin::platform::build_client;
//! use merlin::review::ReviewEngine;
//!
//! # async fn run() -> merlin::error::Result<()> {
//! let cfg = Config::load_default()?;
//! let ai  = Arc::from(build_provider(&cfg.ai)?);
//! let platform = Arc::from(build_client(&cfg.platform)?);
//! let engine = ReviewEngine::new(ai, platform, cfg.review.clone());
//! let comments = engine.run().await?;
//! println!("Posted {} comments", comments.len());
//! # Ok(())
//! # }
//! ```
//!
//! For local diff review without a live CI environment:
//!
//! ```no_run
//! use std::sync::Arc;
//! use merlin::config::Config;
//! use merlin::ai::build_provider;
//! use merlin::platform::NoOpPlatform;
//! use merlin::review::ReviewEngine;
//!
//! # async fn run() -> merlin::error::Result<()> {
//! let cfg     = Config::load_default()?;
//! let ai      = Arc::from(build_provider(&cfg.ai)?);
//! let platform = Arc::new(NoOpPlatform);
//! let engine  = ReviewEngine::new(ai, platform, cfg.review.clone());
//! let diff    = std::fs::read_to_string("my.diff")?;
//! let comments = engine.run_local(&diff).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Architecture
//!
//! ```text
//! CLI (clap)
//!   ├── ReviewEngine          — orchestrates the full review cycle
//!   │     ├── PlatformClient  — fetches diffs, posts comments (GitHub/GitLab/…)
//!   │     ├── DiffParser      — parses unified diff → Vec<FileDiff>
//!   │     ├── AiProvider      — sends diff chunks to AI, parses comments
//!   │     └── RagPipeline     — optional codebase context injection
//!   │
//!   ├── ToolRouter            — dispatches slash-commands (/review, /spec, …)
//!   │
//!   └── AgentRuntime          — ReAct-loop autonomous agent
//!         ├── AgentMemory     — ring-buffer + optional JSONL persistence
//!         └── AgentChannel    — CLI REPL / Slack / Discord
//! ```
//!
//! # Module overview
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`ai`] | AI provider trait and all backend implementations |
//! | [`config`] | Configuration schema, TOML loading, env-var credential helpers |
//! | [`diff`] | Unified diff parser producing [`diff::FileDiff`] structs |
//! | [`digest`] | Token budgeting, file prioritisation, complexity scoring |
//! | [`platform`] | VCS clients: GitHub, GitLab, Bitbucket, Azure DevOps, Gitea |
//! | [`review`] | [`review::ReviewEngine`] orchestration and summary generation |
//! | [`tools`] | Slash-command implementations (`/describe`, `/spec`, `/security`, …) |
//! | [`agent`] | ReAct-loop autonomous agent runtime and memory |
//! | [`rag`] | RAG pipeline: embedders, vector stores, indexer, retriever |
//! | [`webhook`] | Axum-based webhook listener for bot/agent mode |
//! | [`audit`] | Append-only JSONL audit log |
//! | [`error`] | Unified [`error::MerlinError`] type and [`error::Result`] alias |
//! | [`integrations`] | Third-party clients: Jira, Linear, Snyk, CodeTriage |
//! | [`dashboard`] | Optional web dashboard for audit log visualisation |

pub mod agent;
pub mod ai;
pub mod audit;
pub mod config;
pub mod dashboard;
pub mod digest;
pub mod diff;
pub mod error;
pub mod integrations;
pub mod platform;
pub mod rag;
pub mod review;
pub mod tools;
pub mod webhook;
