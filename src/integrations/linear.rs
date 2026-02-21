//! Linear integration — link PRs to Linear issues and add comments.
//!
//! Uses the Linear GraphQL API.
//! Auth: `LINEAR_API_KEY` env var (Bearer token).
//!
//! # Example merlin.toml
//! ```toml
//! [linear]
//! team_id = "TEAM-UUID"   # optional, scopes searches
//! ```

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::config::LinearConfig;
use crate::error::{MerlinError, Result};

const LINEAR_API_URL: &str = "https://api.linear.app/graphql";

pub struct LinearClient {
    config: LinearConfig,
    api_key: String,
    client: reqwest::Client,
}

/// A Linear issue summary.
#[derive(Debug, Clone)]
pub struct LinearIssue {
    pub id: String,
    pub identifier: String,
    pub title: String,
    pub state: String,
    pub url: String,
}

// ── GraphQL request/response types ───────────────────────────────────────────

#[derive(Serialize)]
struct GraphqlRequest {
    query: String,
    variables: serde_json::Value,
}

#[derive(Deserialize)]
struct GraphqlResponse {
    data: Option<serde_json::Value>,
    errors: Option<Vec<GraphqlError>>,
}

#[derive(Deserialize)]
struct GraphqlError {
    message: String,
}

// ─────────────────────────────────────────────────────────────────────────────

impl LinearClient {
    pub fn new(config: LinearConfig, api_key: String) -> Self {
        Self {
            config,
            api_key,
            client: reqwest::Client::new(),
        }
    }

    async fn graphql(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<serde_json::Value> {
        debug!("Sending Linear GraphQL request");

        let resp = self
            .client
            .post(LINEAR_API_URL)
            .header("Authorization", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&GraphqlRequest {
                query: query.to_string(),
                variables,
            })
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(MerlinError::Platform(format!(
                "Linear API error {status}: {body}"
            )));
        }

        let result: GraphqlResponse = resp.json().await?;

        if let Some(errors) = result.errors {
            let msg = errors
                .into_iter()
                .map(|e| e.message)
                .collect::<Vec<_>>()
                .join("; ");
            return Err(MerlinError::Platform(format!(
                "Linear GraphQL errors: {msg}"
            )));
        }

        result
            .data
            .ok_or_else(|| MerlinError::Platform("Empty Linear response".to_string()))
    }

    /// Search for Linear issues matching the given query text.
    pub async fn search_issues(&self, query: &str, max: usize) -> Result<Vec<LinearIssue>> {
        let gql = r#"
            query SearchIssues($query: String!, $first: Int) {
                issueSearch(query: $query, first: $first) {
                    nodes {
                        id
                        identifier
                        title
                        url
                        state { name }
                    }
                }
            }
        "#;

        let mut variables = serde_json::json!({
            "query": query,
            "first": max,
        });

        if let Some(ref team_id) = self.config.team_id {
            variables["teamId"] = serde_json::json!(team_id);
        }

        let data = self.graphql(gql, variables).await?;

        let nodes = data["issueSearch"]["nodes"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        Ok(nodes
            .into_iter()
            .filter_map(|n| {
                Some(LinearIssue {
                    id: n["id"].as_str()?.to_string(),
                    identifier: n["identifier"].as_str()?.to_string(),
                    title: n["title"].as_str()?.to_string(),
                    state: n["state"]["name"].as_str().unwrap_or("Unknown").to_string(),
                    url: n["url"].as_str()?.to_string(),
                })
            })
            .collect())
    }

    /// Add a comment to a Linear issue.
    pub async fn add_comment(&self, issue_id: &str, body: &str) -> Result<()> {
        let gql = r#"
            mutation CreateComment($issueId: String!, $body: String!) {
                commentCreate(input: { issueId: $issueId, body: $body }) {
                    success
                }
            }
        "#;

        let variables = serde_json::json!({
            "issueId": issue_id,
            "body": body,
        });

        let data = self.graphql(gql, variables).await?;
        let success = data["commentCreate"]["success"].as_bool().unwrap_or(false);
        if !success {
            warn!("Linear comment creation returned success=false for issue {issue_id}");
        }

        Ok(())
    }

    /// Extract Linear issue identifiers from text (e.g. "ENG-123", "ABC-456").
    pub fn extract_issue_identifiers(text: &str) -> Vec<String> {
        regex::Regex::new(r"\b([A-Z]{2,10}-\d+)\b")
            .map(|re| {
                re.captures_iter(text)
                    .filter_map(|cap| cap.get(1))
                    .map(|m| m.as_str().to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Format found Linear issues as a Markdown table.
    pub fn format_issues_table(issues: &[LinearIssue]) -> String {
        if issues.is_empty() {
            return "No related Linear issues found.\n".to_string();
        }
        let mut out = "| ID | Title | State |\n|----|-------|-------|\n".to_string();
        for i in issues {
            out.push_str(&format!(
                "| [{}]({}) | {} | {} |\n",
                i.identifier, i.url, i.title, i.state
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_identifiers() {
        let text = "Closes ENG-42 and relates to BACK-100";
        let ids = LinearClient::extract_issue_identifiers(text);
        assert!(ids.contains(&"ENG-42".to_string()));
        assert!(ids.contains(&"BACK-100".to_string()));
    }

    #[test]
    fn test_format_table_empty() {
        let table = LinearClient::format_issues_table(&[]);
        assert!(table.contains("No related"));
    }
}
