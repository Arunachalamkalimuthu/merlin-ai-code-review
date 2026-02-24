use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use super::{InlineCodeSuggestion, Issue, PlatformClient, PrInfo};
use crate::ai::ReviewComment;
use crate::error::{MerlinError, Result};

pub struct GitLabClient {
    token: String,
    base_url: String,
    project_id: String,
    mr_iid: u64,
    head_sha: String,
    client: reqwest::Client,
}

impl GitLabClient {
    pub fn new(
        token: String,
        base_url: String,
        project_id: String,
        mr_iid: u64,
        head_sha: String,
    ) -> Self {
        Self {
            token,
            base_url,
            project_id,
            mr_iid,
            head_sha,
            client: reqwest::Client::new(),
        }
    }

    pub fn from_env(token: String) -> Result<Self> {
        let base_url = std::env::var("CI_API_V4_URL")
            .unwrap_or_else(|_| "https://gitlab.com/api/v4".to_string());
        let project_id = std::env::var("CI_PROJECT_ID")
            .map_err(|_| MerlinError::EnvVar("CI_PROJECT_ID".to_string()))?;
        let mr_iid: u64 = std::env::var("CI_MERGE_REQUEST_IID")
            .map_err(|_| MerlinError::EnvVar("CI_MERGE_REQUEST_IID".to_string()))?
            .parse()
            .map_err(|_| MerlinError::Config("Invalid MR IID".to_string()))?;
        let head_sha = std::env::var("CI_COMMIT_SHA")
            .map_err(|_| MerlinError::EnvVar("CI_COMMIT_SHA".to_string()))?;
        Ok(Self::new(token, base_url, project_id, mr_iid, head_sha))
    }

    fn mr_url(&self, path: &str) -> String {
        format!(
            "{}/projects/{}/merge_requests/{}/{}",
            self.base_url, self.project_id, self.mr_iid, path
        )
    }

    fn proj_url(&self, path: &str) -> String {
        format!("{}/projects/{}/{}", self.base_url, self.project_id, path)
    }
}

// ── GitLab API types ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct GitLabDiffFile {
    old_path: String,
    new_path: String,
    diff: String,
}

#[derive(Deserialize)]
struct GitLabMr {
    iid: u64,
    title: String,
    description: Option<String>,
    sha: String,
    target_branch: String,
    source_branch: String,
    author: GitLabUser,
    work_in_progress: Option<bool>,
    labels: Vec<String>,
    changes_count: Option<String>,
}

#[derive(Deserialize)]
struct GitLabUser {
    username: String,
}

#[derive(Deserialize)]
struct GitLabIssue {
    iid: u64,
    title: String,
    description: Option<String>,
    labels: Vec<String>,
    web_url: String,
}

#[derive(Deserialize)]
struct GitLabFile {
    content: String,
    blob_id: String,
}

#[derive(Serialize)]
struct NoteBody<'a> {
    body: &'a str,
}

// ─────────────────────────────────────────────────────────────────────────────

#[async_trait]
impl PlatformClient for GitLabClient {
    #[instrument(skip(self))]
    async fn get_diff(&self) -> Result<String> {
        let url = self.mr_url("diffs");
        debug!("Fetching MR diffs from: {url}");

        let files: Vec<GitLabDiffFile> = self
            .client
            .get(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("GitLab API error: {e}")))?
            .json()
            .await?;

        let mut diff = String::new();
        for file in &files {
            diff.push_str(&format!(
                "--- a/{}\n+++ b/{}\n",
                file.old_path, file.new_path
            ));
            diff.push_str(&file.diff);
            diff.push('\n');
        }
        Ok(diff)
    }

    #[instrument(skip(self, comment))]
    async fn post_inline_comment(&self, comment: &ReviewComment) -> Result<()> {
        let url = self.mr_url("discussions");
        let emoji = severity_emoji(&comment.severity);
        let body_text = format_comment(emoji, comment);

        let payload = serde_json::json!({
            "body": body_text,
            "position": {
                "base_sha": self.head_sha,
                "start_sha": self.head_sha,
                "head_sha": self.head_sha,
                "position_type": "text",
                "new_path": comment.file,
                "new_line": comment.line
            }
        });

        self.client
            .post(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .json(&payload)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Failed to post inline comment: {e}")))?;
        Ok(())
    }

    #[instrument(skip(self, summary))]
    async fn post_summary(&self, summary: &str) -> Result<()> {
        let url = self.mr_url("notes");
        let payload = NoteBody { body: summary };

        self.client
            .post(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .json(&payload)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Failed to post summary: {e}")))?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_pr_info(&self) -> Result<PrInfo> {
        let url = format!(
            "{}/projects/{}/merge_requests/{}",
            self.base_url, self.project_id, self.mr_iid
        );
        let mr: GitLabMr = self
            .client
            .get(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Failed to get MR info: {e}")))?
            .json()
            .await?;

        Ok(PrInfo {
            number: mr.iid,
            title: mr.title,
            body: mr.description.unwrap_or_default(),
            head_sha: mr.sha,
            base_branch: mr.target_branch,
            head_branch: mr.source_branch,
            author: mr.author.username,
            is_draft: mr.work_in_progress.unwrap_or(false),
            labels: mr.labels,
            files_changed: mr.changes_count.and_then(|s| s.parse().ok()).unwrap_or(0),
            additions: 0,
            deletions: 0,
        })
    }

    #[instrument(skip(self))]
    async fn update_description(&self, title: &str, body: &str) -> Result<()> {
        let url = format!(
            "{}/projects/{}/merge_requests/{}",
            self.base_url, self.project_id, self.mr_iid
        );
        let payload = serde_json::json!({ "title": title, "description": body });

        self.client
            .put(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .json(&payload)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Failed to update MR description: {e}")))?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn set_labels(&self, labels: &[String]) -> Result<()> {
        let url = format!(
            "{}/projects/{}/merge_requests/{}",
            self.base_url, self.project_id, self.mr_iid
        );
        let label_str = labels.join(",");
        let payload = serde_json::json!({ "labels": label_str });

        self.client
            .put(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .json(&payload)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Failed to set labels: {e}")))?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn list_issues(&self, limit: usize) -> Result<Vec<Issue>> {
        let url = self.proj_url(&format!("issues?state=opened&per_page={limit}"));
        let issues: Vec<GitLabIssue> = self
            .client
            .get(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Failed to list issues: {e}")))?
            .json()
            .await?;

        Ok(issues
            .into_iter()
            .map(|i| Issue {
                number: i.iid,
                title: i.title,
                body: i.description.unwrap_or_default(),
                labels: i.labels,
                url: i.web_url,
            })
            .collect())
    }

    #[instrument(skip(self, suggestions))]
    async fn post_code_suggestions(&self, suggestions: &[InlineCodeSuggestion]) -> Result<()> {
        let url = self.mr_url("discussions");
        for s in suggestions {
            let body = format!(
                "{}\n\n```suggestion:-0+0\n{}\n```",
                s.description, s.suggestion
            );
            let payload = serde_json::json!({
                "body": body,
                "position": {
                    "base_sha": self.head_sha,
                    "start_sha": self.head_sha,
                    "head_sha": self.head_sha,
                    "position_type": "text",
                    "new_path": s.file,
                    "new_line": s.end_line
                }
            });
            self.client
                .post(&url)
                .header("PRIVATE-TOKEN", &self.token)
                .json(&payload)
                .send()
                .await?
                .error_for_status()
                .map_err(|e| MerlinError::Platform(format!("Failed to post suggestion: {e}")))?;
        }
        Ok(())
    }

    #[instrument(skip(self, content))]
    async fn update_file(
        &self,
        path: &str,
        content: &str,
        message: &str,
        _current_sha: Option<&str>,
    ) -> Result<()> {
        let encoded_path = path.replace('/', "%2F");
        let url = self.proj_url(&format!("repository/files/{}", encoded_path));

        let payload = serde_json::json!({
            "branch": "main",
            "content": content,
            "commit_message": message
        });

        // Try update first, then create
        let resp = self
            .client
            .put(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .json(&payload)
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            self.client
                .post(&url)
                .header("PRIVATE-TOKEN", &self.token)
                .json(&payload)
                .send()
                .await?
                .error_for_status()
                .map_err(|e| MerlinError::Platform(format!("Failed to create file: {e}")))?;
        } else {
            resp.error_for_status()
                .map_err(|e| MerlinError::Platform(format!("Failed to update file: {e}")))?;
        }
        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_file(&self, path: &str) -> Result<Option<(String, String)>> {
        use base64::{engine::general_purpose::STANDARD, Engine};
        let encoded_path = path.replace('/', "%2F");
        let url = self.proj_url(&format!("repository/files/{}?ref=main", encoded_path));

        let resp = self
            .client
            .get(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        let file: GitLabFile = resp
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Failed to get file: {e}")))?
            .json()
            .await?;

        let bytes = STANDARD
            .decode(&file.content)
            .map_err(|e| MerlinError::Platform(format!("Base64 decode: {e}")))?;
        Ok(Some((
            String::from_utf8_lossy(&bytes).into_owned(),
            file.blob_id,
        )))
    }
}

fn severity_emoji(severity: &crate::ai::Severity) -> &'static str {
    use crate::ai::Severity;
    match severity {
        Severity::Critical => "🔴",
        Severity::High => "🟠",
        Severity::Medium => "🟡",
        Severity::Low => "🔵",
        Severity::Info => "⚪",
    }
}

fn format_comment(emoji: &str, c: &ReviewComment) -> String {
    format!(
        "**{emoji} [{severity:?}] {title}**\n\n{body}{suggestion}",
        emoji = emoji,
        severity = c.severity,
        title = c.title,
        body = c.body,
        suggestion = c
            .suggestion
            .as_deref()
            .map(|s| {
                let s = crate::platform::strip_suggestion_fences(s);
                format!("\n\n**Suggestion:**\n```\n{s}\n```")
            })
            .unwrap_or_default(),
    )
}
