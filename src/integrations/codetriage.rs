//! CodeTriage integration — surface open issues from CodeTriage for reviewed repos.
//!
//! CodeTriage (https://www.codetriage.com/) helps open-source maintainers
//! manage their GitHub issue backlog. This integration can:
//!   1. Look up whether a repository is listed on CodeTriage
//!   2. Fetch open triaged issues relevant to changed files
//!   3. Suggest linking new PRs to existing CodeTriage issues
//!
//! CodeTriage exposes a public JSON API:
//!   GET https://www.codetriage.com/{user}/{repo}.json
//!   GET https://www.codetriage.com/{user}/{repo}/issues.json?page=1
//!
//! No API key required for public repos.

use serde::Deserialize;
use tracing::debug;

use crate::error::{MerlinError, Result};

const CODETRIAGE_BASE: &str = "https://www.codetriage.com";

// ── API types ──────────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
pub struct CodeTriageRepo {
    pub full_name: String,
    pub description: Option<String>,
    pub subscribers_count: u32,
    pub open_issues: u32,
    pub github_url: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CodeTriageIssue {
    pub number: u64,
    pub title: String,
    pub html_url: String,
    pub state: String,
    #[serde(default)]
    pub labels: Vec<CodeTriageLabel>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CodeTriageLabel {
    pub name: String,
}

#[derive(Deserialize)]
struct CodeTriageIssuesResponse {
    issues: Vec<CodeTriageIssue>,
}

// ── Client ──────────────────────────────────────────────────────────────────────

pub struct CodeTriageClient {
    client: reqwest::Client,
}

impl CodeTriageClient {
    pub fn new() -> Self {
        Self { client: reqwest::Client::new() }
    }

    /// Check if a repo is registered on CodeTriage.
    pub async fn get_repo(&self, owner: &str, repo: &str) -> Result<CodeTriageRepo> {
        let url = format!("{CODETRIAGE_BASE}/{owner}/{repo}.json");
        debug!("Fetching CodeTriage repo info: {url}");

        let resp = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await?;

        if resp.status().as_u16() == 404 {
            return Err(MerlinError::Platform(format!(
                "Repository `{owner}/{repo}` is not registered on CodeTriage. \
                 Visit https://www.codetriage.com to add it."
            )));
        }

        if !resp.status().is_success() {
            let status = resp.status();
            return Err(MerlinError::Platform(format!("CodeTriage API error {status}")));
        }

        resp.json().await.map_err(|e| MerlinError::Platform(format!("CodeTriage parse error: {e}")))
    }

    /// Fetch open issues from CodeTriage (paginated, max `limit`).
    pub async fn get_issues(
        &self,
        owner: &str,
        repo: &str,
        limit: usize,
    ) -> Result<Vec<CodeTriageIssue>> {
        let mut all_issues = Vec::new();
        let mut page = 1u32;

        while all_issues.len() < limit {
            let url = format!(
                "{CODETRIAGE_BASE}/{owner}/{repo}/issues.json?page={page}"
            );
            debug!("Fetching CodeTriage issues page {page}");

            let resp = self
                .client
                .get(&url)
                .header("Accept", "application/json")
                .send()
                .await?;

            if !resp.status().is_success() {
                break;
            }

            let result: CodeTriageIssuesResponse = match resp.json().await {
                Ok(r) => r,
                Err(_) => break,
            };

            if result.issues.is_empty() {
                break;
            }

            all_issues.extend(result.issues);
            page += 1;
        }

        all_issues.truncate(limit);
        Ok(all_issues)
    }

    /// Search issues for those whose title/labels match a query string.
    pub async fn search_issues(
        &self,
        owner: &str,
        repo: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<CodeTriageIssue>> {
        let issues = self.get_issues(owner, repo, 100).await?;
        let query_lower = query.to_lowercase();

        let matched: Vec<CodeTriageIssue> = issues
            .into_iter()
            .filter(|i| {
                i.title.to_lowercase().contains(&query_lower)
                    || i.labels.iter().any(|l| l.name.to_lowercase().contains(&query_lower))
            })
            .take(limit)
            .collect();

        Ok(matched)
    }

    /// Format CodeTriage issues as a Markdown table.
    pub fn format_issues_table(
        issues: &[CodeTriageIssue],
        owner: &str,
        repo: &str,
    ) -> String {
        if issues.is_empty() {
            return format!(
                "No matching open issues found on [CodeTriage](https://www.codetriage.com/{owner}/{repo}).\n"
            );
        }

        let mut out = "| # | Title | Labels | CodeTriage |\n\
             |---|-------|--------|------------|\n"
            .to_string();
        for issue in issues {
            let labels = if issue.labels.is_empty() {
                "—".to_string()
            } else {
                issue.labels.iter().map(|l| format!("`{}`", l.name)).collect::<Vec<_>>().join(", ")
            };
            out.push_str(&format!(
                "| [#{}]({gh}) | {title} | {labels} | [View](https://www.codetriage.com/{owner}/{repo}/issues/{num}) |\n",
                issue.number,
                gh = issue.html_url,
                title = issue.title,
                num = issue.number,
            ));
        }
        out
    }

    /// Extract repo owner/name from a GitHub remote URL.
    /// Returns `None` if parsing fails.
    pub fn parse_github_repo(repo_field: &str) -> Option<(String, String)> {
        // Handles "owner/repo" format (from GITHUB_REPOSITORY env var)
        let parts: Vec<&str> = repo_field.trim().splitn(2, '/').collect();
        if parts.len() == 2 {
            return Some((parts[0].to_string(), parts[1].to_string()));
        }
        None
    }
}

impl Default for CodeTriageClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_github_repo() {
        let result = CodeTriageClient::parse_github_repo("octocat/Hello-World");
        assert_eq!(result, Some(("octocat".to_string(), "Hello-World".to_string())));
    }

    #[test]
    fn test_parse_github_repo_invalid() {
        assert!(CodeTriageClient::parse_github_repo("no-slash-here").is_none());
    }

    #[test]
    fn test_format_issues_table_empty() {
        let table = CodeTriageClient::format_issues_table(&[], "owner", "repo");
        assert!(table.contains("No matching"));
    }
}
