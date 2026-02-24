//! /link_linear — Find related Linear issues and link them to the PR.

use async_trait::async_trait;
use tracing::info;

use super::{MerlinTool, ToolContext};
use crate::config::Config;
use crate::error::Result;
use crate::integrations::linear::LinearClient;

/// Tool for the `/link-linear` slash command — links the PR to matching Linear issues.
pub struct LinkLinearTool;

#[async_trait]
impl MerlinTool for LinkLinearTool {
    fn name(&self) -> &'static str {
        "link_linear"
    }

    async fn run(&self, ctx: &ToolContext) -> Result<String> {
        info!("Running /link_linear");

        let api_key = match Config::linear_api_key() {
            Ok(k) => k,
            Err(_) => {
                return Ok("## Merlin: Linear\n\n⚠️ `LINEAR_API_KEY` not set. \
                           Please configure your Linear API key to enable this integration.\n\
                           *[Merlin](https://github.com/you/merlin) 🦡*"
                    .to_string())
            }
        };

        let pr_info = ctx.platform.get_pr_info().await?;
        let linear_cfg = crate::config::Config::load_default()
            .unwrap_or_default()
            .linear;

        let linear = LinearClient::new(linear_cfg, api_key);

        let search_text = format!("{} {}", pr_info.title, pr_info.body);

        // 1. Extract explicitly referenced Linear IDs (e.g. "ENG-123")
        let explicit_ids = LinearClient::extract_issue_identifiers(&search_text);

        // 2. Search for related issues by PR title keywords
        let keywords: String = pr_info
            .title
            .split_whitespace()
            .take(5)
            .collect::<Vec<_>>()
            .join(" ");
        let searched = linear.search_issues(&keywords, 5).await.unwrap_or_default();

        let mut out = "## Merlin: Linear Issue Links\n\n".to_string();

        if !explicit_ids.is_empty() {
            out.push_str(&format!(
                "### Referenced in PR description\n\n{}\n\n",
                explicit_ids
                    .iter()
                    .map(|id| format!("- **{id}**"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }

        out.push_str("### Related Issues (by keyword)\n\n");
        out.push_str(&LinearClient::format_issues_table(&searched));

        out.push_str("\n*[Merlin](https://github.com/you/merlin) 🦡*");
        Ok(out)
    }
}
