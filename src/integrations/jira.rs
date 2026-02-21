//! Jira integration — link PRs to Jira issues and add comments.
//!
//! Uses the Jira Cloud REST API v3.
//! Auth: HTTP Basic with `{email}:{JIRA_TOKEN}` (Base64).
//!
//! Required env: `JIRA_TOKEN`
//! Required config: `[jira] base_url`, `project_key`, `user_email`
//!
//! # Example merlin.toml
//! ```toml
//! [jira]
//! base_url = "https://company.atlassian.net"
//! project_key = "PROJ"
//! user_email = "bot@company.com"
//! ```

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::config::JiraConfig;
use crate::error::{MerlinError, Result};

pub struct JiraClient {
    config: JiraConfig,
    api_token: String,
    client: reqwest::Client,
}

/// A Jira issue summary returned by search.
#[derive(Debug, Clone)]
pub struct JiraIssue {
    pub key: String,
    pub summary: String,
    pub status: String,
    pub url: String,
}

// ── API types ─────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct JiraSearchResponse {
    issues: Vec<JiraIssueRaw>,
}

#[derive(Deserialize)]
struct JiraIssueRaw {
    key: String,
    fields: JiraFields,
}

#[derive(Deserialize)]
struct JiraFields {
    summary: String,
    status: JiraStatus,
}

#[derive(Deserialize)]
struct JiraStatus {
    name: String,
}

#[derive(Serialize)]
struct JiraCommentBody<'a> {
    body: JiraAdfDocument<'a>,
}

#[derive(Serialize)]
struct JiraAdfDocument<'a> {
    version: u8,
    #[serde(rename = "type")]
    doc_type: &'static str,
    content: Vec<JiraAdfParagraph<'a>>,
}

#[derive(Serialize)]
struct JiraAdfParagraph<'a> {
    #[serde(rename = "type")]
    para_type: &'static str,
    content: Vec<JiraAdfText<'a>>,
}

#[derive(Serialize)]
struct JiraAdfText<'a> {
    #[serde(rename = "type")]
    text_type: &'static str,
    text: &'a str,
}

// ─────────────────────────────────────────────────────────────────────────────

impl JiraClient {
    pub fn new(config: JiraConfig, api_token: String) -> Self {
        Self { config, api_token, client: reqwest::Client::new() }
    }

    fn auth_header(&self) -> Result<String> {
        let email = self.config.user_email.as_deref().unwrap_or("ferret-bot");
        let raw = format!("{email}:{}", self.api_token);
        Ok(format!("Basic {}", B64.encode(raw.as_bytes())))
    }

    fn base_url(&self) -> Result<&str> {
        self.config.base_url.as_deref().ok_or_else(|| {
            MerlinError::Config("jira.base_url is required".to_string())
        })
    }

    /// Search for Jira issues matching keywords from the PR title/body.
    pub async fn search_issues(&self, keywords: &str, max: usize) -> Result<Vec<JiraIssue>> {
        let base = self.base_url()?;
        let project = self.config.project_key.as_deref().unwrap_or("");
        let jql = if project.is_empty() {
            format!("text ~ \"{keywords}\" ORDER BY updated DESC")
        } else {
            format!("project = {project} AND text ~ \"{keywords}\" ORDER BY updated DESC")
        };

        let url = format!("{base}/rest/api/3/issue/search");
        let auth = self.auth_header()?;

        debug!("Searching Jira: {jql}");

        let resp = self
            .client
            .get(&url)
            .header("Authorization", auth)
            .header("Accept", "application/json")
            .query(&[("jql", jql.as_str()), ("maxResults", &max.to_string())])
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(MerlinError::Platform(format!("Jira search error {status}: {body}")));
        }

        let result: JiraSearchResponse = resp.json().await?;
        Ok(result
            .issues
            .into_iter()
            .map(|i| {
                let url = format!("{base}/browse/{}", i.key);
                JiraIssue {
                    key: i.key,
                    summary: i.fields.summary,
                    status: i.fields.status.name,
                    url,
                }
            })
            .collect())
    }

    /// Add a comment to a Jira issue (using Atlassian Document Format v3).
    pub async fn add_comment(&self, issue_key: &str, text: &str) -> Result<()> {
        let base = self.base_url()?;
        let url = format!("{base}/rest/api/3/issue/{issue_key}/comment");
        let auth = self.auth_header()?;

        let body = JiraCommentBody {
            body: JiraAdfDocument {
                version: 1,
                doc_type: "doc",
                content: vec![JiraAdfParagraph {
                    para_type: "paragraph",
                    content: vec![JiraAdfText { text_type: "text", text }],
                }],
            },
        };

        let resp = self
            .client
            .post(&url)
            .header("Authorization", auth)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            warn!("Failed to add Jira comment to {issue_key}: {status} — {body}");
        }

        Ok(())
    }

    /// Extract Jira issue keys from a text string (e.g. "PROJ-123" or "ABC-456").
    pub fn extract_issue_keys(text: &str, project_key: Option<&str>) -> Vec<String> {
        let pattern = if let Some(proj) = project_key {
            format!(r"\b{proj}-\d+\b")
        } else {
            r"\b[A-Z][A-Z0-9]+-\d+\b".to_string()
        };

        regex::Regex::new(&pattern)
            .map(|re| {
                re.find_iter(text)
                    .map(|m| m.as_str().to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Format found Jira issues as a Markdown table.
    pub fn format_issues_table(issues: &[JiraIssue]) -> String {
        if issues.is_empty() {
            return "No related Jira issues found.\n".to_string();
        }
        let mut out = "| Key | Summary | Status |\n|-----|---------|--------|\n".to_string();
        for i in issues {
            out.push_str(&format!(
                "| [{}]({}) | {} | {} |\n",
                i.key, i.url, i.summary, i.status
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_issue_keys() {
        let text = "Fixes PROJ-123 and also relates to ABC-456";
        let keys = JiraClient::extract_issue_keys(text, Some("PROJ"));
        assert_eq!(keys, vec!["PROJ-123"]);

        let keys_any = JiraClient::extract_issue_keys(text, None);
        assert!(keys_any.contains(&"PROJ-123".to_string()));
        assert!(keys_any.contains(&"ABC-456".to_string()));
    }

    #[test]
    fn test_format_issues_table_empty() {
        let table = JiraClient::format_issues_table(&[]);
        assert!(table.contains("No related"));
    }
}
