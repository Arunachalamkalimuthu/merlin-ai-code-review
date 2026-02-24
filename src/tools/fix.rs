//! `/fix` — auto-apply AI suggestions to a new branch.
//!
//! When a reviewer comments `/fix` on a PR, this tool:
//!
//! 1. Fetches the PR diff and runs a fresh AI review (local mode — no posting).
//! 2. Collects every comment that has a concrete code suggestion.
//! 3. Creates a new branch `merlin-fix/{pr_number}` from the head SHA.
//! 4. For each suggestion, fetches the file, replaces the target line,
//!    and commits the updated content to the new branch.
//! 5. Returns a Markdown summary listing every file changed.
use std::sync::Arc;

use async_trait::async_trait;
use tracing::{info, warn};

use super::{MerlinTool, ToolContext};
use crate::error::Result;
use crate::review::ReviewEngine;

/// Slash-command tool that applies AI suggestions to a dedicated fix branch.
pub struct FixTool;

#[async_trait]
impl MerlinTool for FixTool {
    fn name(&self) -> &'static str {
        "fix"
    }

    async fn run(&self, ctx: &ToolContext) -> Result<String> {
        info!("Running /fix");

        // 1. Get PR metadata (branch name + head SHA)
        let pr_info = ctx.platform.get_pr_info().await?;
        let head_sha = pr_info.head_sha.clone();
        let pr_number = pr_info.number;

        // 2. Run a local AI review to collect suggestions (no posting)
        let engine = ReviewEngine::new(
            Arc::clone(&ctx.ai),
            Arc::clone(&ctx.platform),
            crate::config::ReviewConfig::default(),
        );
        let all_comments = engine.run_local(&ctx.platform.get_diff().await?).await?;

        // 3. Filter to comments that carry a concrete code suggestion
        let with_suggestions: Vec<_> = all_comments
            .iter()
            .filter(|c| c.suggestion.is_some())
            .collect();

        if with_suggestions.is_empty() {
            return Ok(format!(
                "## Merlin: /fix\n\nNo actionable suggestions to auto-apply on PR #{}.\n\n\
                 *[Merlin](https://github.com/you/merlin) 🦡*",
                pr_number
            ));
        }

        // 4. Create the fix branch
        let branch_name = format!("merlin-fix/{pr_number}");
        ctx.platform
            .create_branch(&branch_name, &head_sha)
            .await?;
        info!("Created branch {branch_name} from {head_sha}");

        // 5. Apply each suggestion
        let mut applied: Vec<String> = Vec::new();
        for comment in &with_suggestions {
            let raw_suggestion = match &comment.suggestion {
                Some(s) => s,
                None => continue,
            };
            let suggestion = strip_fences(raw_suggestion);

            match ctx.platform.get_file(&comment.file).await {
                Ok(Some((content, sha))) => {
                    let mut lines: Vec<&str> = content.lines().collect();
                    let idx = (comment.line as usize).saturating_sub(1);
                    if idx < lines.len() {
                        lines[idx] = suggestion;
                        let new_content = lines.join("\n");
                        let commit_msg = format!(
                            "fix: apply Merlin suggestion in {} (line {})",
                            comment.file, comment.line
                        );
                        match ctx
                            .platform
                            .update_file(
                                &comment.file,
                                &new_content,
                                &commit_msg,
                                Some(&sha),
                                Some(&branch_name),
                            )
                            .await
                        {
                            Ok(()) => applied.push(format!(
                                "- `{}` line {} — *{}*",
                                comment.file, comment.line, comment.title
                            )),
                            Err(e) => warn!(
                                "Failed to update {} on {branch_name}: {e}",
                                comment.file
                            ),
                        }
                    } else {
                        warn!(
                            "Line {} out of range for {} ({} lines) — skipping",
                            comment.line,
                            comment.file,
                            lines.len()
                        );
                    }
                }
                Ok(None) => warn!("File not found: {}", comment.file),
                Err(e) => warn!("Failed to fetch {}: {e}", comment.file),
            }
        }

        if applied.is_empty() {
            return Ok(format!(
                "## Merlin: /fix\n\nBranch `{branch_name}` was created but no files \
                 could be updated (check the logs for details).\n\n\
                 *[Merlin](https://github.com/you/merlin) 🦡*"
            ));
        }

        Ok(format!(
            "## Merlin: /fix\n\nApplied **{}** suggestion(s) on branch `{branch_name}`:\n\n{}\n\n\
             *[Merlin](https://github.com/you/merlin) 🦡*",
            applied.len(),
            applied.join("\n"),
        ))
    }
}

/// Strip leading ` ```lang ` and trailing ` ``` ` fences from an AI suggestion.
fn strip_fences(s: &str) -> &str {
    let s = s.trim();
    let s = if s.starts_with("```") {
        s.split_once('\n').map(|x| x.1).unwrap_or(s).trim_start()
    } else {
        s
    };
    let s = s.trim_end();
    if let Some(stripped) = s.strip_suffix("```") {
        stripped.trim_end()
    } else {
        s
    }
}
