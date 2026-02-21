//! /link_jira — Find related Jira issues and link them to the PR.
//!
//! Scans the PR title and description for Jira issue keys (e.g. PROJ-123).
//! Also searches Jira for issues that match the PR title keywords.
//! Posts a summary table as a PR comment.

use async_trait::async_trait;
use tracing::info;

use super::{MerlinTool, ToolContext};
use crate::config::Config;
use crate::error::Result;
use crate::integrations::jira::JiraClient;

pub struct LinkJiraTool;

#[async_trait]
impl MerlinTool for LinkJiraTool {
    fn name(&self) -> &'static str {
        "link_jira"
    }

    async fn run(&self, ctx: &ToolContext) -> Result<String> {
        info!("Running /link_jira");

        let api_token = match Config::jira_token() {
            Ok(t) => t,
            Err(_) => {
                return Ok("## Merlin: Jira\n\n⚠️ `JIRA_TOKEN` not set. \
                           Please configure your Jira API token to enable this integration.\n\
                           *[Merlin](https://github.com/you/merlin) 🦡*"
                    .to_string())
            }
        };

        let pr_info = ctx.platform.get_pr_info().await?;
        let jira_cfg = crate::config::Config::load_default()
            .unwrap_or_default()
            .jira;

        if !jira_cfg.is_configured() {
            return Ok("## Merlin: Jira\n\n⚠️ Jira is not configured. \
                       Add `[jira] base_url = \"...\"` to `merlin.toml`.\n\
                       *[Merlin](https://github.com/you/merlin) 🦡*"
                .to_string());
        }

        let jira = JiraClient::new(jira_cfg.clone(), api_token);

        let search_text = format!("{} {}", pr_info.title, pr_info.body);

        // 1. Find explicitly mentioned issue keys (e.g. "Fixes PROJ-123")
        let explicit_keys =
            JiraClient::extract_issue_keys(&search_text, jira_cfg.project_key.as_deref());

        // 2. Search for related issues by PR title keywords
        let keywords: String = pr_info.title.split_whitespace().take(5).collect::<Vec<_>>().join(" ");
        let searched = jira.search_issues(&keywords, 5).await.unwrap_or_default();

        let mut out = "## Merlin: Jira Issue Links\n\n".to_string();

        if !explicit_keys.is_empty() {
            out.push_str(&format!(
                "### Referenced in PR description\n\n{}\n\n",
                explicit_keys
                    .iter()
                    .map(|k| format!("- **{k}**"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }

        out.push_str("### Related Issues (by keyword)\n\n");
        out.push_str(&JiraClient::format_issues_table(&searched));

        out.push_str("\n*[Merlin](https://github.com/you/merlin) 🦡*");
        Ok(out)
    }
}
