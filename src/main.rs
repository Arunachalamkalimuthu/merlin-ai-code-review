use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use merlin::agent::channels::{CliChannel, DiscordChannel, SlackChannel};
use merlin::agent::runtime::AgentRuntime;
use merlin::agent::{AgentContext, AgentTask};
use merlin::ai::build_provider;
use merlin::config::Config;
use merlin::diff::parse_diff;
use merlin::error::Result;
use merlin::platform::build_client;
use merlin::rag::build_pipeline;
use merlin::review::ReviewEngine;
use merlin::tools::{route_command, ToolContext};
use merlin::update;
use merlin::webhook::{serve, WebhookState};

#[derive(Parser)]
#[command(
    name = "merlin",
    version,
    about = "AI-powered self-hosted code review for GitHub and GitLab",
    long_about = "Merlin parses PR/MR diffs, sends code to a configurable AI provider \
                  (Claude, OpenAI, or Claude Code CLI), and posts inline review comments,\n\
                  code suggestions, labels, and a summary back to the PR/MR.\n\n\
                  Trigger commands by commenting on a PR:\n\
                    @merlin /review\n\
                    @merlin /describe\n\
                    @merlin /ask <question>\n\
                    @merlin /improve\n\
                    @merlin /generate_labels\n\
                    @merlin /update_changelog\n\
                    @merlin /add_doc\n\
                    @merlin /similar_issue"
)]
struct Cli {
    /// Path to merlin.toml config file
    #[arg(short, long, global = true, default_value = "merlin.toml")]
    config: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a full PR/MR review (auto-detects platform from env)
    Review {
        /// Path to a local unified diff file (skips platform API call)
        #[arg(long)]
        diff: Option<PathBuf>,

        /// Output format
        #[arg(long, default_value = "text")]
        output: OutputFormat,
    },

    /// Run a specific slash command (e.g., `merlin run /describe`)
    Run {
        /// Slash command to execute: /review, /describe, /ask, /improve,
        /// /generate_labels, /update_changelog, /add_doc, /similar_issue
        #[arg(value_name = "COMMAND")]
        command: String,

        /// Optional argument (e.g., question for /ask)
        #[arg(value_name = "ARG", trailing_var_arg = true)]
        args: Vec<String>,

        /// Output format
        #[arg(long, default_value = "text")]
        output: OutputFormat,
    },

    /// Start the webhook server for bot mode (@merlin mention in PR comments)
    Webhook {
        /// Port to listen on
        #[arg(long, default_value = "8080")]
        port: u16,
    },

    /// Parse a diff file and print its structure (for debugging)
    ParseDiff {
        /// Path to unified diff file
        #[arg(required = true)]
        diff: PathBuf,
    },

    /// Manage the RAG (vector) index for context-aware code review
    Rag {
        #[command(subcommand)]
        action: RagAction,
    },

    /// Run the autonomous agent (ReAct loop with tool use)
    Agent {
        /// Channel: cli | slack | discord
        #[arg(long, default_value = "cli")]
        channel: String,

        /// Port for Slack/Discord webhook server
        #[arg(long, default_value = "8090")]
        port: u16,

        /// Run a single task non-interactively and exit
        #[arg(long)]
        task: Option<String>,
    },

    /// Update Merlin to the latest release
    #[command(name = "self-update", alias = "update")]
    SelfUpdate {
        /// Check for a new version without downloading
        #[arg(long)]
        check: bool,

        /// Download and install even if the version string matches
        #[arg(long)]
        force: bool,
    },
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Subcommand)]
enum RagAction {
    /// Index the codebase under a directory into the vector store
    Index {
        /// Root directory to walk (default: current directory)
        #[arg(default_value = ".")]
        root: PathBuf,
    },

    /// Search the RAG index with a natural-language query
    Search {
        /// Query string
        query: String,

        /// Maximum results to return
        #[arg(long, short = 'k', default_value = "5")]
        limit: usize,
    },

    /// Remove all documents from the configured collection
    Clear,

    /// Print the number of indexed documents
    Count,
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    let cli = Cli::parse();
    let config = Config::load(&cli.config)?;

    match cli.command {
        // ── merlin review ────────────────────────────────────────────────────
        Commands::Review { diff, output } => {
            let ai = Arc::from(build_provider(&config.ai)?);

            match diff {
                Some(diff_path) => {
                    let raw_diff = std::fs::read_to_string(&diff_path)?;
                    let platform = Arc::from(merlin::platform::NoOpPlatform)
                        as Arc<dyn merlin::platform::PlatformClient>;
                    let engine = ReviewEngine::new(ai, platform, config.review);
                    let comments = engine.run_local(&raw_diff).await?;
                    print_output(&comments, &output);
                }
                None => {
                    let platform = Arc::from(build_client(&config.platform)?);
                    let engine = ReviewEngine::new(ai, platform, config.review);
                    let comments = engine.run().await?;
                    print_output(&comments, &output);
                }
            }
        }

        // ── merlin run /command ──────────────────────────────────────────────
        Commands::Run {
            command,
            args,
            output,
        } => {
            let ai = Arc::from(build_provider(&config.ai)?);
            let platform = Arc::from(build_client(&config.platform)?);

            let tool = route_command(&command)?;
            let arg = if args.is_empty() {
                None
            } else {
                Some(args.join(" "))
            };
            let ctx = ToolContext { ai, platform, arg };

            let result = tool.run(&ctx).await?;

            match output {
                OutputFormat::Json => {
                    let json = serde_json::json!({"result": result});
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json).unwrap_or_default()
                    );
                }
                OutputFormat::Text => println!("{result}"),
            }
        }

        // ── merlin webhook ───────────────────────────────────────────────────
        Commands::Webhook { port } => {
            let ai = Arc::from(build_provider(&config.ai)?);

            let state = Arc::new(WebhookState {
                ai,
                github_secret: std::env::var("MERLIN_GITHUB_SECRET").ok(),
                gitlab_secret: std::env::var("MERLIN_GITLAB_SECRET").ok(),
                github_token: std::env::var("GITHUB_TOKEN").ok(),
                gitlab_token: std::env::var("GITLAB_TOKEN").ok(),
            });

            serve(state, port).await;
        }

        // ── merlin agent ─────────────────────────────────────────────────────
        Commands::Agent {
            channel,
            port,
            task,
        } => {
            let ai = std::sync::Arc::from(build_provider(&config.ai)?);

            // Platform is optional for the agent (not every task needs it)
            let platform = build_client(&config.platform)
                .ok()
                .map(std::sync::Arc::from);

            let mut agent_cfg = config.agent.clone();
            agent_cfg.port = port;

            let ctx = AgentContext {
                ai,
                platform,
                config: config.clone(),
            };
            let mut runtime = AgentRuntime::new(ctx, &agent_cfg);

            if let Some(task_str) = task {
                // Single-shot non-interactive mode
                let task = AgentTask {
                    content: task_str,
                    sender: Some("cli".to_string()),
                    thread_id: None,
                };
                let response = runtime.run(&task).await?;
                println!("{response}");
            } else {
                // Channel / interactive mode
                let ch = channel.to_lowercase();
                match ch.as_str() {
                    "slack" => {
                        let mut slack = SlackChannel::new(port).await?;
                        runtime.run_channel(&mut slack).await?;
                    }
                    "discord" => {
                        let mut discord = DiscordChannel::new().await?;
                        runtime.run_channel(&mut discord).await?;
                    }
                    _ => {
                        // Default: CLI REPL
                        println!(
                            "🧙 Merlin Agent — type your task and press Enter (\"exit\" to quit)"
                        );
                        let mut cli = CliChannel::new();
                        runtime.run_channel(&mut cli).await?;
                    }
                }
            }
        }

        // ── merlin rag ───────────────────────────────────────────────────────
        Commands::Rag { action } => {
            if !config.rag.enabled {
                eprintln!("RAG is disabled. Set `[rag] enabled = true` in merlin.toml.");
                std::process::exit(1);
            }
            let pipeline = build_pipeline(&config.rag);
            match action {
                RagAction::Index { root } => {
                    let n = merlin::rag::indexer::index_directory(&pipeline, &root, &config.rag)
                        .await?;
                    println!("Indexed {n} chunks from {:?}", root);
                }
                RagAction::Search { query, limit } => {
                    let docs = pipeline.retrieve(&query, limit).await?;
                    if docs.is_empty() {
                        println!("No results found for: {query}");
                    } else {
                        print!("{}", merlin::rag::retriever::format_rag_context(&docs));
                    }
                }
                RagAction::Clear => {
                    pipeline.clear().await?;
                    println!("Cleared collection '{}'.", config.rag.collection);
                }
                RagAction::Count => {
                    let n = pipeline.count().await?;
                    println!("{n} document(s) indexed in '{}'.", config.rag.collection);
                }
            }
        }

        // ── merlin self-update ───────────────────────────────────────────────
        Commands::SelfUpdate { check, force } => {
            if check {
                update::check_for_update().await?;
            } else {
                update::self_update(force).await?;
            }
        }

        // ── merlin parse-diff ────────────────────────────────────────────────
        Commands::ParseDiff { diff } => {
            let raw = std::fs::read_to_string(&diff)?;
            let files = parse_diff(&raw)?;
            println!("Parsed {} file(s):", files.len());
            for f in &files {
                let priority = merlin::digest::classify_priority(f.path());
                println!(
                    "  {} [{:?}] ({} hunk(s)){}{}",
                    f.path(),
                    priority,
                    f.hunks.len(),
                    if f.is_new { " [NEW]" } else { "" },
                    if f.is_deleted { " [DELETED]" } else { "" },
                );
            }
        }
    }

    Ok(())
}

fn print_output(comments: &[merlin::ai::ReviewComment], format: &OutputFormat) {
    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(comments).unwrap_or_default()
            );
        }
        OutputFormat::Text => {
            if comments.is_empty() {
                println!("No issues found.");
                return;
            }
            for c in comments {
                println!(
                    "[{:?}][{:?}] {}:{} — {}",
                    c.severity, c.category, c.file, c.line, c.title
                );
                println!("  {}", c.body);
                if let Some(ref s) = c.suggestion {
                    println!("  Suggestion: {s}");
                }
                println!();
            }
        }
    }
}
