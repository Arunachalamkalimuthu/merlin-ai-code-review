//! /similar_issue — Find open issues similar to this PR.

use async_trait::async_trait;
use serde::Deserialize;
use tracing::info;

use super::{MerlinTool, ToolContext};
use crate::error::Result;

pub struct SimilarIssueTool;

#[derive(Deserialize)]
struct SimilarityResult {
    issue_number: u64,
    similarity_score: f32, // 0.0 – 1.0
    reason: String,
}

#[async_trait]
impl MerlinTool for SimilarIssueTool {
    fn name(&self) -> &'static str {
        "similar_issue"
    }

    async fn run(&self, ctx: &ToolContext) -> Result<String> {
        info!("Running /similar_issue");

        let pr_info = ctx.platform.get_pr_info().await?;
        let issues = ctx.platform.list_issues(50).await?;

        if issues.is_empty() {
            return Ok("## Merlin: Similar Issues\n\nNo open issues found.".to_string());
        }

        // Build compact issue list for AI
        let issues_text = issues
            .iter()
            .map(|i| {
                format!(
                    "#{}: {} — {}",
                    i.number,
                    i.title,
                    i.body.chars().take(200).collect::<String>()
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let system = "You are a software project manager. Find the most similar open issues \
                      to the given PR. Consider title, description, and code areas affected.\n\n\
                      Respond ONLY with JSON (top 5 max, sorted by similarity desc):\n\
                      [{\"issue_number\":42,\"similarity_score\":0.85,\
                      \"reason\":\"Both address the same auth bug\"}]\n\
                      Return [] if no issues are similar (score < 0.3).";

        let user = format!(
            "PR #{num}: \"{title}\"\nDescription: {body}\n\nOpen Issues:\n{issues}",
            num = pr_info.number,
            title = pr_info.title,
            body = pr_info.body.chars().take(500).collect::<String>(),
            issues = issues_text,
        );

        let raw = ctx.ai.generate(system, &user).await?;
        let cleaned = raw
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let similar: Vec<SimilarityResult> =
            serde_json::from_str(cleaned).unwrap_or_default();

        if similar.is_empty() {
            return Ok("## Merlin: Similar Issues\n\nNo closely related issues found.".to_string());
        }

        let mut out = "## Merlin: Similar Issues\n\n".to_string();
        out.push_str("| Score | Issue | Reason |\n");
        out.push_str("|-------|-------|--------|\n");

        for r in &similar {
            if let Some(issue) = issues.iter().find(|i| i.number == r.issue_number) {
                out.push_str(&format!(
                    "| {:.0}% | [#{num} {title}]({url}) | {reason} |\n",
                    r.similarity_score * 100.0,
                    num = issue.number,
                    title = issue.title,
                    url = issue.url,
                    reason = r.reason,
                ));
            }
        }

        out.push_str("\n*Found by [Merlin](https://github.com/you/merlin) 🦡*");
        Ok(out)
    }
}
