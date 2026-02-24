//! Gitea platform client (Gitea API v1).
//!
//! Auth: Bearer token via `GITEA_TOKEN`.
//!
//! Auto-detected from Gitea Actions env:
//!   GITEA_ACTIONS=true, GITEA_SERVER_URL (or GITHUB_SERVER_URL), GITEA_REPO,
//!   GITEA_PR_NUMBER (or parsed from GITHUB_REF), GITEA_SHA / GITHUB_SHA
//!
//! Gitea Actions re-uses GitHub Actions variable names with identical semantics.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use super::{InlineCodeSuggestion, Issue, PlatformClient, PrInfo};
use crate::ai::ReviewComment;
use crate::error::{MerlinError, Result};

pub struct GiteaClient {
    token: String,
    /// e.g. `https://gitea.example.com/api/v1`
    api_base: String,
    owner: String,
    repo: String,
    pr_number: u64,
    head_sha: String,
    client: reqwest::Client,
}

impl GiteaClient {
    pub fn new(
        token: String,
        server_url: String,
        owner: String,
        repo: String,
        pr_number: u64,
        head_sha: String,
    ) -> Self {
        let api_base = format!("{}/api/v1", server_url.trim_end_matches('/'));
        Self {
            token,
            api_base,
            owner,
            repo,
            pr_number,
            head_sha,
            client: reqwest::Client::new(),
        }
    }

    /// Build from Gitea Actions environment variables.
    pub fn from_env(token: String) -> Result<Self> {
        // Gitea Actions sets GITEA_SERVER_URL; fallback to GITHUB_SERVER_URL for compat.
        let server_url = std::env::var("GITEA_SERVER_URL")
            .or_else(|_| std::env::var("GITHUB_SERVER_URL"))
            .map_err(|_| MerlinError::EnvVar("GITEA_SERVER_URL".to_string()))?;

        // GITEA_REPO or GITHUB_REPOSITORY → "owner/repo"
        let full_repo = std::env::var("GITEA_REPO")
            .or_else(|_| std::env::var("GITHUB_REPOSITORY"))
            .map_err(|_| MerlinError::EnvVar("GITEA_REPO".to_string()))?;

        let (owner, repo) = full_repo.split_once('/').ok_or_else(|| {
            MerlinError::Config(format!(
                "Invalid repo format '{full_repo}', expected owner/repo"
            ))
        })?;

        // PR number from GITEA_PR_NUMBER or parsed from GITHUB_REF (refs/pull/N/head)
        let pr_number: u64 = std::env::var("GITEA_PR_NUMBER")
            .or_else(|_| {
                std::env::var("GITHUB_REF").map(|r| r.split('/').nth(2).unwrap_or("0").to_string())
            })
            .map_err(|_| MerlinError::EnvVar("GITEA_PR_NUMBER".to_string()))?
            .parse()
            .map_err(|_| MerlinError::Config("Invalid PR number".to_string()))?;

        let head_sha = std::env::var("GITEA_SHA")
            .or_else(|_| std::env::var("GITHUB_SHA"))
            .map_err(|_| MerlinError::EnvVar("GITEA_SHA".to_string()))?;

        Ok(Self::new(
            token,
            server_url,
            owner.to_string(),
            repo.to_string(),
            pr_number,
            head_sha,
        ))
    }

    fn repo_url(&self, path: &str) -> String {
        format!(
            "{}/repos/{}/{}/{}",
            self.api_base, self.owner, self.repo, path
        )
    }

    fn auth_header(&self) -> String {
        format!("token {}", self.token)
    }
}

// ── Gitea API types (mirrors GitHub's closely) ───────────────────────────────

#[derive(Deserialize)]
struct GiteaPr {
    number: u64,
    title: String,
    body: Option<String>,
    head: GiteaRef,
    base: GiteaRef,
    user: GiteaUser,
    #[serde(rename = "draft")]
    draft: Option<bool>,
    labels: Vec<GiteaLabel>,
    additions: Option<u32>,
    deletions: Option<u32>,
    changed_files: Option<u32>,
}

#[derive(Deserialize)]
struct GiteaRef {
    sha: String,
    #[serde(rename = "ref")]
    branch: String,
}

#[derive(Deserialize)]
struct GiteaUser {
    login: String,
}

#[derive(Deserialize)]
struct GiteaLabel {
    name: String,
}

#[derive(Deserialize)]
struct GiteaIssue {
    number: u64,
    title: String,
    body: Option<String>,
    labels: Vec<GiteaLabel>,
    html_url: String,
}

#[derive(Serialize)]
struct GiteaCommentBody<'a> {
    body: &'a str,
}

#[derive(Serialize)]
struct GiteaUpdatePr<'a> {
    title: &'a str,
    body: &'a str,
}

#[derive(Serialize)]
struct GiteaLabelsBody {
    labels: Vec<u64>,
}

#[derive(Deserialize)]
struct GiteaLabelResp {
    id: u64,
    name: String,
}

#[derive(Serialize)]
struct GiteaCreateLabel<'a> {
    name: &'a str,
    color: &'a str,
}

#[derive(Deserialize)]
struct GiteaFileContent {
    content: String,
    sha: String,
}

// ─────────────────────────────────────────────────────────────────────────────

#[async_trait]
impl PlatformClient for GiteaClient {
    #[instrument(skip(self))]
    async fn get_diff(&self) -> Result<String> {
        // Gitea exposes a raw diff endpoint: GET /repos/{owner}/{repo}/pulls/{index}.diff
        let url = self.repo_url(&format!("pulls/{}.diff", self.pr_number));
        debug!("Fetching Gitea PR diff from: {url}");

        let diff = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Gitea diff error: {e}")))?
            .text()
            .await?;

        Ok(diff)
    }

    #[instrument(skip(self, comment))]
    async fn post_inline_comment(&self, comment: &ReviewComment) -> Result<()> {
        // Gitea review comments: POST /repos/{owner}/{repo}/pulls/{index}/reviews
        let url = self.repo_url(&format!("pulls/{}/reviews", self.pr_number));
        let emoji = severity_emoji(&comment.severity);
        let body_text = format_comment(emoji, comment);

        let payload = serde_json::json!({
            "commit_id": self.head_sha,
            "body": "",
            "comments": [{
                "path": comment.file,
                "new_position": comment.line,
                "body": body_text
            }]
        });

        self.client
            .post(&url)
            .header("Authorization", self.auth_header())
            .json(&payload)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Gitea inline comment: {e}")))?;
        Ok(())
    }

    #[instrument(skip(self, summary))]
    async fn post_summary(&self, summary: &str) -> Result<()> {
        let url = self.repo_url(&format!("issues/{}/comments", self.pr_number));
        let payload = GiteaCommentBody { body: summary };

        self.client
            .post(&url)
            .header("Authorization", self.auth_header())
            .json(&payload)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Gitea summary: {e}")))?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_pr_info(&self) -> Result<PrInfo> {
        let url = self.repo_url(&format!("pulls/{}", self.pr_number));
        let pr: GiteaPr = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Gitea PR info: {e}")))?
            .json()
            .await?;

        Ok(PrInfo {
            number: pr.number,
            title: pr.title,
            body: pr.body.unwrap_or_default(),
            head_sha: pr.head.sha,
            base_branch: pr.base.branch,
            head_branch: pr.head.branch,
            author: pr.user.login,
            is_draft: pr.draft.unwrap_or(false),
            labels: pr.labels.into_iter().map(|l| l.name).collect(),
            files_changed: pr.changed_files.unwrap_or(0),
            additions: pr.additions.unwrap_or(0),
            deletions: pr.deletions.unwrap_or(0),
        })
    }

    #[instrument(skip(self))]
    async fn update_description(&self, title: &str, body: &str) -> Result<()> {
        let url = self.repo_url(&format!("pulls/{}", self.pr_number));
        let payload = GiteaUpdatePr { title, body };

        self.client
            .patch(&url)
            .header("Authorization", self.auth_header())
            .json(&payload)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Gitea update PR: {e}")))?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn set_labels(&self, labels: &[String]) -> Result<()> {
        if labels.is_empty() {
            return Ok(());
        }

        // Resolve or create label IDs
        let existing_url = self.repo_url("labels");
        let existing: Vec<GiteaLabelResp> = self
            .client
            .get(&existing_url)
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .json()
            .await
            .unwrap_or_default();

        let mut ids: Vec<u64> = Vec::new();
        for label_name in labels {
            if let Some(existing_label) = existing.iter().find(|l| l.name == *label_name) {
                ids.push(existing_label.id);
            } else {
                // Create new label with a default color
                let create_payload = GiteaCreateLabel {
                    name: label_name,
                    color: "#ededed",
                };
                #[derive(Deserialize)]
                struct CreatedLabel {
                    id: u64,
                }
                if let Ok(resp) = self
                    .client
                    .post(&existing_url)
                    .header("Authorization", self.auth_header())
                    .json(&create_payload)
                    .send()
                    .await
                {
                    if let Ok(created) = resp.json::<CreatedLabel>().await {
                        ids.push(created.id);
                    }
                }
            }
        }

        let url = self.repo_url(&format!("issues/{}/labels", self.pr_number));
        let payload = GiteaLabelsBody { labels: ids };
        self.client
            .post(&url)
            .header("Authorization", self.auth_header())
            .json(&payload)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Gitea set labels: {e}")))?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn list_issues(&self, limit: usize) -> Result<Vec<Issue>> {
        let url = self.repo_url(&format!("issues?type=issues&state=open&limit={limit}"));
        let issues: Vec<GiteaIssue> = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Gitea issues: {e}")))?
            .json()
            .await?;

        Ok(issues
            .into_iter()
            .map(|i| Issue {
                number: i.number,
                title: i.title,
                body: i.body.unwrap_or_default(),
                labels: i.labels.into_iter().map(|l| l.name).collect(),
                url: i.html_url,
            })
            .collect())
    }

    async fn post_code_suggestions(&self, suggestions: &[InlineCodeSuggestion]) -> Result<()> {
        // Gitea supports suggestion blocks in review comments
        let url = self.repo_url(&format!("pulls/{}/reviews", self.pr_number));
        for s in suggestions {
            let body = format!("{}\n\n```suggestion\n{}\n```", s.description, s.suggestion);
            let payload = serde_json::json!({
                "commit_id": self.head_sha,
                "body": "",
                "comments": [{
                    "path": s.file,
                    "new_position": s.end_line,
                    "body": body
                }]
            });
            self.client
                .post(&url)
                .header("Authorization", self.auth_header())
                .json(&payload)
                .send()
                .await?
                .error_for_status()
                .map_err(|e| MerlinError::Platform(format!("Gitea suggestion: {e}")))?;
        }
        Ok(())
    }

    #[instrument(skip(self, content))]
    async fn update_file(
        &self,
        path: &str,
        content: &str,
        message: &str,
        current_sha: Option<&str>,
    ) -> Result<()> {
        use base64::{engine::general_purpose::STANDARD, Engine};
        let url = self.repo_url(&format!("contents/{}", path.trim_start_matches('/')));
        let encoded = STANDARD.encode(content.as_bytes());

        let mut payload = serde_json::json!({
            "message": message,
            "content": encoded,
        });
        if let Some(sha) = current_sha {
            payload["sha"] = serde_json::Value::String(sha.to_string());
        }

        // Try PUT (update); if 404, POST (create)
        let resp = self
            .client
            .put(&url)
            .header("Authorization", self.auth_header())
            .json(&payload)
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            self.client
                .post(&url)
                .header("Authorization", self.auth_header())
                .json(&payload)
                .send()
                .await?
                .error_for_status()
                .map_err(|e| MerlinError::Platform(format!("Gitea create file: {e}")))?;
        } else {
            resp.error_for_status()
                .map_err(|e| MerlinError::Platform(format!("Gitea update file: {e}")))?;
        }
        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_file(&self, path: &str) -> Result<Option<(String, String)>> {
        use base64::{engine::general_purpose::STANDARD, Engine};
        let url = self.repo_url(&format!("contents/{}", path.trim_start_matches('/')));
        let resp = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        let file: GiteaFileContent = resp
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Gitea get file: {e}")))?
            .json()
            .await?;

        let cleaned = file.content.replace('\n', "");
        let bytes = STANDARD
            .decode(&cleaned)
            .map_err(|e| MerlinError::Platform(format!("Gitea base64 decode: {e}")))?;
        Ok(Some((
            String::from_utf8_lossy(&bytes).into_owned(),
            file.sha,
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
