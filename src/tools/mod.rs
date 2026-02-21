//! Slash-command tools — each implements the `MerlinTool` trait.

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
use std::sync::Arc;

use crate::ai::AiProvider;
use crate::error::{MerlinError, Result};
use crate::platform::PlatformClient;

// ── Tool trait ────────────────────────────────────────────────────────────────

/// Context passed to every tool invocation.
pub struct ToolContext {
    pub ai: Arc<dyn AiProvider>,
    pub platform: Arc<dyn PlatformClient>,
    /// Optional free-text argument following the slash command (e.g. `/ask Is this thread-safe?`)
    pub arg: Option<String>,
}

#[async_trait]
pub trait MerlinTool: Send + Sync {
    /// Human-readable name of the tool (for logging).
    fn name(&self) -> &'static str;

    /// Execute the tool and return a Markdown result string (posted as a PR comment).
    async fn run(&self, ctx: &ToolContext) -> Result<String>;
}

// ── Command router ────────────────────────────────────────────────────────────

/// Parse a slash command string and dispatch to the right tool.
pub fn route_command(command: &str) -> Result<Box<dyn MerlinTool>> {
    let cmd = command.trim().to_lowercase();
    let base = cmd.split_whitespace().next().unwrap_or("");

    match base {
        "/review" => Ok(Box::new(crate::review::ReviewTool)),
        "/describe" => Ok(Box::new(describe::DescribeTool)),
        "/ask" => Ok(Box::new(ask::AskTool)),
        "/improve" => Ok(Box::new(improve::ImproveTool)),
        "/generate_labels" => Ok(Box::new(labels::LabelsTool)),
        "/update_changelog" => Ok(Box::new(changelog::ChangelogTool)),
        "/add_doc" => Ok(Box::new(docstring::DocstringTool)),
        "/similar_issue" => Ok(Box::new(similar_issue::SimilarIssueTool)),
        "/test" => Ok(Box::new(test_gen::TestGenTool)),
        "/explain" => Ok(Box::new(explain::ExplainTool)),
        "/security" => Ok(Box::new(security::SecurityTool)),
        "/approve" => Ok(Box::new(approve::ApproveTool)),
        "/commit_message" => Ok(Box::new(commit_message::CommitMessageTool)),
        "/docs" => Ok(Box::new(docs::DocsTool)),
        "/coverage" => Ok(Box::new(coverage::CoverageTool)),
        "/link_jira" => Ok(Box::new(link_jira::LinkJiraTool)),
        "/link_linear" => Ok(Box::new(link_linear::LinkLinearTool)),
        "/snyk" => Ok(Box::new(snyk::SnykTool)),
        "/spec" => Ok(Box::new(spec::SpecTool)),
        "/triage" => Ok(Box::new(triage::TriageTool)),
        other => Err(MerlinError::Other(format!("Unknown command: {other}"))),
    }
}

/// Extract slash command + optional arg from a comment body.
/// Matches `@merlin /command args` or just `/command args`.
pub fn parse_command(body: &str) -> Option<(String, Option<String>)> {
    let re = regex::Regex::new(r"(?i)(?:@merlin\s+)?(/\w+)(.*)").ok()?;
    let caps = re.captures(body)?;
    let command = caps.get(1)?.as_str().to_lowercase();
    let arg = caps
        .get(2)
        .map(|m| m.as_str().trim().to_string())
        .filter(|s| !s.is_empty());
    Some((command, arg))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command_simple() {
        let (cmd, arg) = parse_command("/review").unwrap();
        assert_eq!(cmd, "/review");
        assert!(arg.is_none());
    }

    #[test]
    fn test_parse_command_with_bot_mention() {
        let (cmd, arg) = parse_command("@merlin /ask Is this thread-safe?").unwrap();
        assert_eq!(cmd, "/ask");
        assert_eq!(arg.unwrap(), "Is this thread-safe?");
    }

    #[test]
    fn test_parse_command_case_insensitive() {
        let (cmd, _) = parse_command("@Merlin /DESCRIBE").unwrap();
        assert_eq!(cmd, "/describe");
    }

    #[test]
    fn test_parse_command_no_match() {
        assert!(parse_command("just a regular comment").is_none());
    }

    #[test]
    fn test_route_known_commands() {
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
    fn test_route_unknown_command() {
        assert!(route_command("/nonexistent").is_err());
    }
}
