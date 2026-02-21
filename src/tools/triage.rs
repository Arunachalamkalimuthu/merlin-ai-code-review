//! /triage — Surface related open issues from CodeTriage for this PR.
//!
//! Finds open issues on CodeTriage that are related to the changed files and PR topic,
//! helping maintainers link PRs to open issues and prioritize their backlog.
//!
//! Usage:
//!   @ferret /triage             — auto-detect repo from CI env
//!   @ferret /triage owner/repo  — specify repo explicitly

use async_trait::async_trait;
use tracing::info;

use super::{MerlinTool, ToolContext};
use crate::error::Result;
use crate::integrations::codetriage::CodeTriageClient;

pub struct TriageTool;

#[async_trait]
impl MerlinTool for TriageTool {
    fn name(&self) -> &'static str {
        "triage"
    }

    async fn run(&self, ctx: &ToolContext) -> Result<String> {
        info!("Running /triage");

        let client = CodeTriageClient::new();

        // Resolve owner/repo
        let (owner, repo) = if let Some(ref arg) = ctx.arg {
            match CodeTriageClient::parse_github_repo(arg) {
                Some(pair) => pair,
                None => {
                    return Ok("## Ferret: CodeTriage\n\n\
                               ⚠️ Invalid repository format. Use `owner/repo`.\n\n\
                               *[Merlin](https://github.com/you/ferret) 🦡*"
                        .to_string())
                }
            }
        } else {
            // Auto-detect from CI env
            let github_repo =
                std::env::var("GITHUB_REPOSITORY").unwrap_or_default();
            match CodeTriageClient::parse_github_repo(&github_repo) {
                Some(pair) => pair,
                None => {
                    return Ok("## Ferret: CodeTriage\n\n\
                               ⚠️ Could not detect repository. Set `GITHUB_REPOSITORY` or pass `owner/repo` as argument.\n\n\
                               *[Merlin](https://github.com/you/ferret) 🦡*"
                        .to_string())
                }
            }
        };

        let pr_info = ctx.platform.get_pr_info().await?;

        // 1. Check if the repo is on CodeTriage
        let repo_info = client.get_repo(&owner, &repo).await.ok();

        let mut out = "## Ferret: CodeTriage Issue Linking\n\n".to_string();

        if let Some(ref info) = repo_info {
            out.push_str(&format!(
                "📋 **[{full_name}](https://www.codetriage.com/{owner}/{repo})** — \
                 {subs} subscriber(s), {issues} open issue(s) in triage\n\n",
                full_name = info.full_name,
                subs = info.subscribers_count,
                issues = info.open_issues,
            ));
        } else {
            out.push_str(&format!(
                "> This repository is not on CodeTriage yet. \
                 [Add it](https://www.codetriage.com/{owner}/{repo}) to get help triaging issues!\n\n"
            ));
        }

        // 2. Search for related issues using PR title keywords
        let keywords: String =
            pr_info.title.split_whitespace().take(5).collect::<Vec<_>>().join(" ");

        let issues = client
            .search_issues(&owner, &repo, &keywords, 10)
            .await
            .unwrap_or_default();

        out.push_str("### Related Open Issues\n\n");
        out.push_str(&CodeTriageClient::format_issues_table(&issues, &owner, &repo));

        // 3. AI suggestion for which issue this PR might close
        if !issues.is_empty() {
            let issues_text = issues
                .iter()
                .map(|i| format!("#{}: {}", i.number, i.title))
                .collect::<Vec<_>>()
                .join("\n");

            let system = "You are a helpful maintainer assistant. \
                          Given a PR title and a list of open issues, identify \
                          which issue(s) this PR most likely addresses. \
                          Be concise — one sentence per suggestion.";
            let user = format!(
                "PR: \"{}\"\n\nOpen issues:\n{}",
                pr_info.title, issues_text
            );
            let suggestion = ctx.ai.generate(system, &user).await.unwrap_or_default();
            if !suggestion.trim().is_empty() {
                out.push_str("\n### AI Suggestion\n\n");
                out.push_str(&suggestion);
                out.push('\n');
            }
        }

        out.push_str("\n*Powered by [CodeTriage](https://www.codetriage.com/) · \
                      [Merlin](https://github.com/you/ferret) 🦡*");
        Ok(out)
    }
}
