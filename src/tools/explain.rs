//! /explain — Plain-language explanation of what the PR changes do.

use async_trait::async_trait;
use tracing::info;

use super::{MerlinTool, ToolContext};
use crate::diff::parse_diff;
use crate::digest::prioritize_diffs;
use crate::error::Result;

pub struct ExplainTool;

#[async_trait]
impl MerlinTool for ExplainTool {
    fn name(&self) -> &'static str {
        "explain"
    }

    async fn run(&self, ctx: &ToolContext) -> Result<String> {
        info!("Running /explain");

        let raw_diff = ctx.platform.get_diff().await?;
        let pr_info = ctx.platform.get_pr_info().await?;
        let files = parse_diff(&raw_diff)?;
        let prioritized = prioritize_diffs(files, None);

        let diff_summary = prioritized
            .iter()
            .map(|pd| {
                format!(
                    "### `{}`\n```diff\n{}\n```",
                    pd.file.path(),
                    crate::digest::compress_diff(&pd.file, 60)
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        let system = "You are a senior engineer explaining a pull request to a non-expert. \
                      Write a clear, jargon-free walkthrough of what the code changes do, \
                      why they matter, and how they work. Use:\n\
                      - A one-sentence TL;DR\n\
                      - Per-file explanation bullet points\n\
                      - Highlight any tricky or non-obvious parts\n\
                      Use Markdown. Keep it concise but complete.";

        let user = format!(
            "PR #{num}: \"{title}\"\n\nDiff:\n{diff}",
            num = pr_info.number,
            title = pr_info.title,
            diff = diff_summary,
        );

        let explanation = ctx.ai.generate(system, &user).await?;

        Ok(format!(
            "## Merlin: PR Explanation\n\n{explanation}\n\n\
             *Explained by [Merlin](https://github.com/you/merlin) 🦡*"
        ))
    }
}
