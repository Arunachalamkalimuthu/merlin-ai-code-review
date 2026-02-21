//! Claude Code authorization mode.
//!
//! When `auth = "claude-code"` is set in [ai] config, Merlin delegates API calls
//! through the `claude` CLI instead of using a raw API key. This lets organizations
//! use their existing Claude Code subscription without managing separate API keys.
//!
//! The `claude` CLI must be installed and authenticated on the host:
//!   https://claude.ai/claude-code
//!
//! Flow:
//!   1. Merlin writes the prompt to a temp file
//!   2. Invokes: `claude -p <prompt_file> --output-format json`
//!   3. Parses the JSON stdout
//!
//! This works for both CI runners (where `claude` is pre-authenticated via
//! `claude auth login --token $CLAUDE_CODE_TOKEN`) and developer machines.

use async_trait::async_trait;
use serde::Deserialize;
use tokio::process::Command;
use tracing::{debug, instrument};

use super::{system_prompt, AiProvider, ReviewComment, ReviewContext};
use crate::config::AiConfig;
use crate::error::{MerlinError, Result};

pub struct ClaudeCodeProvider {
    config: AiConfig,
}

impl ClaudeCodeProvider {
    pub fn new(config: AiConfig) -> Self {
        Self { config }
    }

    /// Check that `claude` is available on PATH.
    pub fn is_available() -> bool {
        std::process::Command::new("claude")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

#[derive(Deserialize)]
struct ClaudeCliOutput {
    result: Option<String>,
}

#[async_trait]
impl AiProvider for ClaudeCodeProvider {
    #[instrument(skip(self, ctx), fields(file = %ctx.file))]
    async fn review(&self, ctx: &ReviewContext) -> Result<Vec<ReviewComment>> {
        let system = system_prompt(&[
            "bugs".to_string(),
            "security".to_string(),
            "style".to_string(),
            "performance".to_string(),
        ]);
        let user = format!(
            "Review the following diff for file `{}`:\n\n```diff\n{}\n```",
            ctx.file, ctx.diff_hunk
        );
        let raw = self.call_cli(&system, &user).await?;
        parse_review_response(&raw)
    }

    #[instrument(skip(self, system, user))]
    async fn generate(&self, system: &str, user: &str) -> Result<String> {
        self.call_cli(system, user).await
    }
}

impl ClaudeCodeProvider {
    async fn call_cli(&self, system: &str, user: &str) -> Result<String> {
        // Write prompt to a temp file to avoid shell escaping issues
        let prompt = format!("{system}\n\n---\n\n{user}");
        let tmp = tempfile::NamedTempFile::new().map_err(MerlinError::Io)?;
        tokio::fs::write(tmp.path(), &prompt).await?;

        debug!("Invoking claude CLI for prompt ({} chars)", prompt.len());

        let output = Command::new("claude")
            .args([
                "-p",
                tmp.path().to_str().unwrap_or("/tmp/merlin_prompt"),
                "--output-format",
                "json",
                "--model",
                &self.config.model,
                "--max-tokens",
                &self.config.max_tokens.to_string(),
            ])
            .output()
            .await
            .map_err(|e| {
                MerlinError::AiProvider(format!(
                    "Failed to invoke `claude` CLI: {e}. \
                     Ensure claude-code is installed: https://claude.ai/claude-code"
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(MerlinError::AiProvider(format!(
                "`claude` CLI exited with error: {stderr}"
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse JSON output from claude CLI
        if let Ok(parsed) = serde_json::from_str::<ClaudeCliOutput>(&stdout) {
            if let Some(result) = parsed.result {
                return Ok(result);
            }
        }

        // Fallback: return raw stdout
        Ok(stdout.trim().to_string())
    }
}

fn parse_review_response(text: &str) -> Result<Vec<ReviewComment>> {
    let cleaned = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    serde_json::from_str(cleaned).map_err(|e| {
        MerlinError::AiProvider(format!(
            "Failed to parse claude CLI response as ReviewComment array: {e}\nRaw: {cleaned}"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_response() {
        let json = r#"[{"file":"main.rs","line":5,"severity":"high","category":"bug","title":"Test","body":"desc","suggestion":null}]"#;
        let comments = parse_review_response(json).unwrap();
        assert_eq!(comments.len(), 1);
    }

    #[test]
    fn test_parse_empty() {
        assert!(parse_review_response("[]").unwrap().is_empty());
    }
}
