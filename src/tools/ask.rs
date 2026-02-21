//! /ask — Q&A about the PR diff.

use async_trait::async_trait;
use tracing::info;

use super::{MerlinTool, ToolContext};
use crate::diff::parse_diff;
use crate::error::{MerlinError, Result};

pub struct AskTool;

#[async_trait]
impl MerlinTool for AskTool {
    fn name(&self) -> &'static str {
        "ask"
    }

    async fn run(&self, ctx: &ToolContext) -> Result<String> {
        info!("Running /ask");

        let question = ctx.arg.as_deref().ok_or_else(|| {
            MerlinError::Other(
                "/ask requires a question. Usage: @merlin /ask <your question>".to_string(),
            )
        })?;

        let raw_diff = ctx.platform.get_diff().await?;
        let pr_info = ctx.platform.get_pr_info().await?;
        let files = parse_diff(&raw_diff)?;

        let diff_summary = files
            .iter()
            .map(|f| crate::digest::compress_diff(f, 50))
            .collect::<Vec<_>>()
            .join("\n\n");

        let system = "You are a senior engineer and expert code reviewer. \
                      Answer questions about pull request diffs concisely and accurately. \
                      Use Markdown formatting in your response.";

        let user = format!(
            "PR: \"{title}\" by {author}\n\nDiff:\n```diff\n{diff}\n```\n\nQuestion: {q}",
            title = pr_info.title,
            author = pr_info.author,
            diff = diff_summary,
            q = question,
        );

        let answer = ctx.ai.generate(system, &user).await?;

        Ok(format!(
            "## Merlin: Q&A\n\n**Q:** {question}\n\n**A:** {answer}"
        ))
    }
}
