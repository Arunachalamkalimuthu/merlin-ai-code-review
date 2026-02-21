//! Slash-command tool implementations.
//!
//! Each tool implements the [`MerlinTool`] trait and is reachable via a
//! `/command` string.  Tools are dispatched by [`route_command`] and invoked
//! via [`MerlinTool::run`].
//!
//! # Adding a new tool
//!
//! 1. Create `src/tools/my_tool.rs` and implement [`MerlinTool`].
//! 2. Add `pub mod my_tool;` to this file.
//! 3. Add a match arm to [`route_command`].
//! 4. Document the command in `website/docs/slash-commands/`.
//!
//! # Command parsing
//!
//! Use [`parse_command`] to extract the slash command and optional argument
//! from a raw PR comment body.  It recognises both `@merlin /cmd` and bare
//! `/cmd` forms, case-insensitively.

pub mod approve;
pub mod ask;
pub mod changelog;
pub mod commit_message;
pub mod coverage;
pub mod describe;
pub mod docs;
pub mod docstring;
pub mod explain;
pub mod improve;
pub mod labels;
pub mod link_jira;
pub mod link_linear;
pub mod security;
pub mod similar_issue;
pub mod snyk;
pub mod spec;
pub mod test_gen;
pub mod triage;

use async_trait::async_trait;
use std::sync::{Arc, OnceLock};

use crate::ai::AiProvider;
use crate::error::{MerlinError, Result};
use crate::platform::PlatformClient;

// ── Tool trait ────────────────────────────────────────────────────────────────

/// The shared context passed to every tool invocation.
pub struct ToolContext {
    /// The AI provider to use for generation.
    pub ai: Arc<dyn AiProvider>,
    /// The VCS platform client for posting comments and reading PR metadata.
    pub platform: Arc<dyn PlatformClient>,
    /// Optional free-text argument following the slash command.
    ///
    /// For `/ask Is this thread-safe?` the arg is `"Is this thread-safe?"`.
    pub arg: Option<String>,
}

/// Trait implemented by every slash-command tool.
///
/// The [`run`](MerlinTool::run) method receives a [`ToolContext`] and returns
/// a Markdown string that is posted as a PR comment.
#[async_trait]
pub trait MerlinTool: Send + Sync {
    /// Human-readable name used in log messages.
    fn name(&self) -> &'static str;

    /// Execute the tool and return a Markdown string to post as a PR comment.
    ///
    /// # Errors
    ///
    /// May return any [`crate::error::MerlinError`] variant.
    async fn run(&self, ctx: &ToolContext) -> Result<String>;
}

// ── Command router ────────────────────────────────────────────────────────────

/// Instantiate the tool for a given slash command string.
///
/// `command` should be a lowercase slash-prefixed string such as `"/review"`
/// or `"/ask"`.  The argument (if any) is passed separately via
/// [`ToolContext::arg`].
///
/// # Errors
///
/// Returns [`MerlinError::Other`] for unrecognised commands.
///
/// # Examples
///
/// ```rust
/// use merlin::tools::route_command;
///
/// let tool = route_command("/review").unwrap();
/// assert_eq!(tool.name(), "review");
/// ```
pub fn route_command(command: &str) -> Result<Box<dyn MerlinTool>> {
    let base = command.trim().to_lowercase();
    let base = base.split_whitespace().next().unwrap_or("");

    match base {
        "/review"           => Ok(Box::new(crate::review::ReviewTool)),
        "/describe"         => Ok(Box::new(describe::DescribeTool)),
        "/ask"              => Ok(Box::new(ask::AskTool)),
        "/improve"          => Ok(Box::new(improve::ImproveTool)),
        "/generate_labels"  => Ok(Box::new(labels::LabelsTool)),
        "/update_changelog" => Ok(Box::new(changelog::ChangelogTool)),
        "/add_doc"          => Ok(Box::new(docstring::DocstringTool)),
        "/similar_issue"    => Ok(Box::new(similar_issue::SimilarIssueTool)),
        "/test"             => Ok(Box::new(test_gen::TestGenTool)),
        "/explain"          => Ok(Box::new(explain::ExplainTool)),
        "/security"         => Ok(Box::new(security::SecurityTool)),
        "/approve"          => Ok(Box::new(approve::ApproveTool)),
        "/commit_message"   => Ok(Box::new(commit_message::CommitMessageTool)),
        "/docs"             => Ok(Box::new(docs::DocsTool)),
        "/coverage"         => Ok(Box::new(coverage::CoverageTool)),
        "/link_jira"        => Ok(Box::new(link_jira::LinkJiraTool)),
        "/link_linear"      => Ok(Box::new(link_linear::LinkLinearTool)),
        "/snyk"             => Ok(Box::new(snyk::SnykTool)),
        "/spec"             => Ok(Box::new(spec::SpecTool)),
        "/triage"           => Ok(Box::new(triage::TriageTool)),
        other => Err(MerlinError::Other(format!("Unknown command: {other}"))),
    }
}

// ── Command parser ────────────────────────────────────────────────────────────

/// The compiled command-parsing regex, initialised once at first call.
///
/// Pattern: optional `@merlin ` prefix, then `/word`, then optional arg.
/// The regex is compiled once using [`OnceLock`] to avoid recompilation on
/// every PR comment event.
static COMMAND_RE: OnceLock<regex::Regex> = OnceLock::new();

fn command_regex() -> &'static regex::Regex {
    COMMAND_RE.get_or_init(|| {
        regex::Regex::new(r"(?i)(?:@merlin\s+)?(/\w+)(.*)")
            .expect("COMMAND_RE is a valid regex literal")
    })
}

/// Extract a slash command and its optional argument from a PR comment body.
///
/// Recognises both `@merlin /cmd arg` and bare `/cmd arg` forms.
/// Command names are normalised to lowercase.
///
/// Returns `None` if no slash command is found in `body`.
///
/// # Examples
///
/// ```rust
/// use merlin::tools::parse_command;
///
/// let (cmd, arg) = parse_command("@merlin /ask Is this thread-safe?").unwrap();
/// assert_eq!(cmd, "/ask");
/// assert_eq!(arg.unwrap(), "Is this thread-safe?");
///
/// assert!(parse_command("just a regular comment").is_none());
/// ```
pub fn parse_command(body: &str) -> Option<(String, Option<String>)> {
    let caps = command_regex().captures(body)?;
    let command = caps.get(1)?.as_str().to_lowercase();
    let arg = caps
        .get(2)
        .map(|m| m.as_str().trim().to_string())
        .filter(|s| !s.is_empty());
    Some((command, arg))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bare_command() {
        let (cmd, arg) = parse_command("/review").unwrap();
        assert_eq!(cmd, "/review");
        assert!(arg.is_none());
    }

    #[test]
    fn parse_command_with_bot_mention() {
        let (cmd, arg) = parse_command("@merlin /ask Is this thread-safe?").unwrap();
        assert_eq!(cmd, "/ask");
        assert_eq!(arg.unwrap(), "Is this thread-safe?");
    }

    #[test]
    fn parse_command_case_insensitive() {
        let (cmd, _) = parse_command("@Merlin /DESCRIBE").unwrap();
        assert_eq!(cmd, "/describe");
    }

    #[test]
    fn parse_command_no_match() {
        assert!(parse_command("just a regular comment").is_none());
    }

    #[test]
    fn route_all_known_commands() {
        for cmd in &[
            "/review", "/describe", "/ask", "/improve",
            "/generate_labels", "/update_changelog", "/add_doc", "/similar_issue",
            "/test", "/explain", "/security", "/approve", "/commit_message", "/docs",
            "/coverage", "/link_jira", "/link_linear", "/snyk", "/spec", "/triage",
        ] {
            assert!(route_command(cmd).is_ok(), "Failed to route: {cmd}");
        }
    }

    #[test]
    fn route_unknown_command_returns_error() {
        assert!(route_command("/nonexistent").is_err());
    }

    #[test]
    fn regex_compiled_only_once() {
        // Call twice — must not panic and must return the same pointer
        let a = command_regex() as *const _;
        let b = command_regex() as *const _;
        assert_eq!(a, b);
    }
}
