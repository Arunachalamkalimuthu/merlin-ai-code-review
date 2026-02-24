use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use super::{ExistingComment, InlineCodeSuggestion, Issue, PlatformClient, PrInfo, ReviewAction};
use crate::ai::ReviewComment;
use crate::error::{MerlinError, Result};

const GITHUB_API: &str = "https://api.github.com";

pub struct GitHubClient {
    token: String,
    repo: String, // "owner/repo"
    pr_number: u64,
    /// SHA of the head commit (needed for inline review comments)
    head_sha: String,
    client: reqwest::Client,
}

impl GitHubClient {
    pub fn new(token: String, repo: String, pr_number: u64, head_sha: String) -> Self {
        Self {
            token,
            repo,
            pr_number,
            head_sha,
            client: reqwest::Client::new(),
        }
    }

    /// Build from standard GitHub Actions environment variables.
    pub fn from_env(token: String) -> Result<Self> {
        let repo = std::env::var("GITHUB_REPOSITORY")
            .map_err(|_| MerlinError::EnvVar("GITHUB_REPOSITORY".to_string()))?;

        let pr_number: u64 = std::env::var("GITHUB_PR_NUMBER")
            .or_else(|_| {
                std::env::var("GITHUB_REF").map(|r| r.split('/').nth(2).unwrap_or("0").to_string())
            })
            .map_err(|_| MerlinError::EnvVar("GITHUB_PR_NUMBER".to_string()))?
            .parse()
            .map_err(|_| MerlinError::Config("Invalid PR number".to_string()))?;

        let head_sha = std::env::var("GITHUB_SHA")
            .map_err(|_| MerlinError::EnvVar("GITHUB_SHA".to_string()))?;

        Ok(Self::new(token, repo, pr_number, head_sha))
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.token)
    }

    fn api(&self, path: &str) -> String {
        format!("{GITHUB_API}/{path}")
    }
}

// ── GitHub API response types ─────────────────────────────────────────────────

#[derive(Deserialize)]
struct GitHubFile {
    filename: String,
    patch: Option<String>,
}

#[derive(Deserialize)]
struct GitHubPr {
    number: u64,
    title: String,
    body: Option<String>,
    head: GitHubRef,
    base: GitHubRef,
    draft: Option<bool>,
    labels: Vec<GitHubLabel>,
    user: GitHubUser,
    changed_files: u32,
    additions: u32,
    deletions: u32,
}

#[derive(Deserialize)]
struct GitHubRef {
    sha: String,
    #[serde(rename = "ref")]
    branch: String,
}

#[derive(Deserialize)]
struct GitHubLabel {
    name: String,
}

#[derive(Deserialize)]
struct GitHubUser {
    login: String,
}

#[derive(Deserialize)]
struct GitHubIssue {
    number: u64,
    title: String,
    body: Option<String>,
    labels: Vec<GitHubLabel>,
    html_url: String,
}

#[derive(Deserialize)]
struct GitHubFileContent {
    content: String,
    sha: String,
}

/// A single existing PR review comment returned by
/// `GET /repos/{repo}/pulls/{pr}/comments`.
#[derive(Deserialize)]
struct GitHubReviewComment {
    path: String,
    /// Line in the **new** file (`null` for comments on removed lines).
    line: Option<u32>,
    body: String,
}

#[derive(Serialize)]
struct ReviewCommentBody<'a> {
    body: &'a str,
    commit_id: &'a str,
    path: &'a str,
    line: u32,
    side: &'a str,
}

#[derive(Serialize)]
struct IssueCommentBody<'a> {
    body: &'a str,
}

#[derive(Serialize)]
struct UpdatePrBody<'a> {
    title: &'a str,
    body: &'a str,
}

#[derive(Serialize)]
struct SetLabelsBody<'a> {
    labels: &'a [String],
}

#[derive(Serialize)]
struct UpdateFileBody<'a> {
    message: &'a str,
    content: String, // base64
    #[serde(skip_serializing_if = "Option::is_none")]
    sha: Option<&'a str>,
}

/// A single inline comment included in a batch pull-request review.
#[derive(Serialize)]
struct BatchReviewComment {
    path: String,
    line: u32,
    side: String,
    body: String,
}

/// Request body for `POST /repos/{repo}/pulls/{pr}/reviews`.
#[derive(Serialize)]
struct BatchReviewBody {
    commit_id: String,
    body: String,
    event: String,
    comments: Vec<BatchReviewComment>,
}

/// Request body for `POST /repos/{repo}/check-runs` (create).
#[derive(Serialize)]
struct CheckRunCreate {
    name: String,
    head_sha: String,
    status: String,
}

/// A single annotation in a GitHub Checks check-run output.
#[derive(Serialize)]
struct CheckAnnotation {
    path: String,
    start_line: u32,
    end_line: u32,
    annotation_level: String,
    message: String,
    title: String,
}

/// Output block for a GitHub Checks check-run.
#[derive(Serialize)]
struct CheckRunOutput {
    title: String,
    summary: String,
    annotations: Vec<CheckAnnotation>,
}

/// Request body for `PATCH /repos/{repo}/check-runs/{id}` (complete).
#[derive(Serialize)]
struct CheckRunComplete {
    status: String,
    conclusion: String,
    output: CheckRunOutput,
}

/// Response from `POST /repos/{repo}/check-runs`.
#[derive(Deserialize)]
struct CheckRunCreated {
    id: u64,
}

// ─────────────────────────────────────────────────────────────────────────────

#[async_trait]
impl PlatformClient for GitHubClient {
    #[instrument(skip(self))]
    async fn get_diff(&self) -> Result<String> {
        let url = self.api(&format!(
            "repos/{}/pulls/{}/files",
            self.repo, self.pr_number
        ));
        debug!("Fetching PR files from: {url}");

        let files: Vec<GitHubFile> = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "merlin-review/0.1")
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("GitHub API error: {e}")))?
            .json()
            .await?;

        let mut diff = String::new();
        for file in &files {
            if let Some(patch) = &file.patch {
                diff.push_str(&format!(
                    "--- a/{}\n+++ b/{}\n",
                    file.filename, file.filename
                ));
                diff.push_str(patch);
                diff.push('\n');
            }
        }
        Ok(diff)
    }

    #[instrument(skip(self, comment))]
    async fn post_inline_comment(&self, comment: &ReviewComment) -> Result<()> {
        let url = self.api(&format!(
            "repos/{}/pulls/{}/comments",
            self.repo, self.pr_number
        ));
        let emoji = severity_emoji(&comment.severity);
        let body_text = format_comment(emoji, comment);

        let payload = ReviewCommentBody {
            body: &body_text,
            commit_id: &self.head_sha,
            path: &comment.file,
            line: comment.line,
            side: "RIGHT",
        };

        self.client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "merlin-review/0.1")
            .json(&payload)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Failed to post inline comment: {e}")))?;
        Ok(())
    }

    #[instrument(skip(self, summary))]
    async fn post_summary(&self, summary: &str) -> Result<()> {
        let url = self.api(&format!(
            "repos/{}/issues/{}/comments",
            self.repo, self.pr_number
        ));
        let payload = IssueCommentBody { body: summary };

        self.client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "merlin-review/0.1")
            .json(&payload)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Failed to post summary: {e}")))?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_pr_info(&self) -> Result<PrInfo> {
        let url = self.api(&format!("repos/{}/pulls/{}", self.repo, self.pr_number));
        let pr: GitHubPr = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "merlin-review/0.1")
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Failed to get PR info: {e}")))?
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
            files_changed: pr.changed_files,
            additions: pr.additions,
            deletions: pr.deletions,
        })
    }

    #[instrument(skip(self))]
    async fn update_description(&self, title: &str, body: &str) -> Result<()> {
        let url = self.api(&format!("repos/{}/pulls/{}", self.repo, self.pr_number));
        let payload = UpdatePrBody { title, body };

        self.client
            .patch(&url)
            .header("Authorization", self.auth_header())
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "merlin-review/0.1")
            .json(&payload)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Failed to update PR description: {e}")))?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn set_labels(&self, labels: &[String]) -> Result<()> {
        let url = self.api(&format!(
            "repos/{}/issues/{}/labels",
            self.repo, self.pr_number
        ));
        let payload = SetLabelsBody { labels };

        self.client
            .put(&url)
            .header("Authorization", self.auth_header())
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "merlin-review/0.1")
            .json(&payload)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Failed to set labels: {e}")))?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn list_issues(&self, limit: usize) -> Result<Vec<Issue>> {
        let url = self.api(&format!(
            "repos/{}/issues?state=open&per_page={}",
            self.repo, limit
        ));
        let issues: Vec<GitHubIssue> = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "merlin-review/0.1")
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Failed to list issues: {e}")))?
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

    #[instrument(skip(self, suggestions))]
    async fn post_code_suggestions(&self, suggestions: &[InlineCodeSuggestion]) -> Result<()> {
        let url = self.api(&format!(
            "repos/{}/pulls/{}/comments",
            self.repo, self.pr_number
        ));

        for s in suggestions {
            let body = format!("{}\n\n```suggestion\n{}\n```", s.description, s.suggestion);
            let payload = serde_json::json!({
                "body": body,
                "commit_id": self.head_sha,
                "path": s.file,
                "start_line": s.start_line,
                "line": s.end_line,
                "side": "RIGHT",
                "start_side": "RIGHT",
            });

            self.client
                .post(&url)
                .header("Authorization", self.auth_header())
                .header("Accept", "application/vnd.github.v3+json")
                .header("User-Agent", "merlin-review/0.1")
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
        current_sha: Option<&str>,
    ) -> Result<()> {
        use base64::{engine::general_purpose::STANDARD, Engine};
        let url = self.api(&format!("repos/{}/contents/{}", self.repo, path));
        let encoded = STANDARD.encode(content.as_bytes());
        let payload = UpdateFileBody {
            message,
            content: encoded,
            sha: current_sha,
        };

        self.client
            .put(&url)
            .header("Authorization", self.auth_header())
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "merlin-review/0.1")
            .json(&payload)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Failed to update file: {e}")))?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_file(&self, path: &str) -> Result<Option<(String, String)>> {
        use base64::{engine::general_purpose::STANDARD, Engine};
        let url = self.api(&format!("repos/{}/contents/{}", self.repo, path));

        let resp = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "merlin-review/0.1")
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        let file: GitHubFileContent = resp
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Failed to get file: {e}")))?
            .json()
            .await?;

        let cleaned = file.content.replace('\n', "");
        let bytes = STANDARD
            .decode(cleaned)
            .map_err(|e| MerlinError::Platform(format!("Base64 decode error: {e}")))?;
        let content = String::from_utf8_lossy(&bytes).into_owned();
        Ok(Some((content, file.sha)))
    }

    #[instrument(skip(self))]
    async fn list_review_comments(&self) -> Result<Vec<ExistingComment>> {
        let url = self.api(&format!(
            "repos/{}/pulls/{}/comments?per_page=100",
            self.repo, self.pr_number
        ));
        let raw: Vec<GitHubReviewComment> = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "merlin-review/0.1")
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Failed to list review comments: {e}")))?
            .json()
            .await?;

        Ok(raw
            .into_iter()
            .filter_map(|c| {
                c.line.map(|line| ExistingComment {
                    file: c.path,
                    line,
                    body_snippet: c.body.chars().take(80).collect(),
                })
            })
            .collect())
    }

    #[instrument(skip(self, comments, summary))]
    async fn post_check_run(
        &self,
        comments: &[ReviewComment],
        summary: &str,
    ) -> Result<()> {
        use crate::ai::Severity;

        // 1. Create the check-run in "in_progress" state
        let create_url = self.api(&format!("repos/{}/check-runs", self.repo));
        let create_payload = CheckRunCreate {
            name: "Merlin Code Review".to_string(),
            head_sha: self.head_sha.clone(),
            status: "in_progress".to_string(),
        };
        let created: CheckRunCreated = self
            .client
            .post(&create_url)
            .header("Authorization", self.auth_header())
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "merlin-review/0.1")
            .json(&create_payload)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Failed to create check run: {e}")))?
            .json()
            .await?;

        // 2. Map comments → annotations (cap at 50 — GitHub limit per request)
        let annotations: Vec<CheckAnnotation> = comments
            .iter()
            .filter(|c| c.line > 0)
            .take(50)
            .map(|c| {
                let level = match c.severity {
                    Severity::Critical | Severity::High => "failure",
                    Severity::Medium => "warning",
                    Severity::Low | Severity::Info => "notice",
                };
                CheckAnnotation {
                    path: c.file.clone(),
                    start_line: c.line,
                    end_line: c.line,
                    annotation_level: level.to_string(),
                    message: c.body.clone(),
                    title: c.title.clone(),
                }
            })
            .collect();

        // 3. Determine overall conclusion
        let conclusion = if comments
            .iter()
            .any(|c| matches!(c.severity, Severity::Critical | Severity::High))
        {
            "failure"
        } else {
            "success"
        };

        // 4. Complete the check-run with annotations
        let patch_url = self.api(&format!("repos/{}/check-runs/{}", self.repo, created.id));
        let complete_payload = CheckRunComplete {
            status: "completed".to_string(),
            conclusion: conclusion.to_string(),
            output: CheckRunOutput {
                title: "Merlin Code Review".to_string(),
                summary: summary.to_string(),
                annotations,
            },
        };
        self.client
            .patch(&patch_url)
            .header("Authorization", self.auth_header())
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "merlin-review/0.1")
            .json(&complete_payload)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Failed to complete check run: {e}")))?;

        Ok(())
    }

    #[instrument(skip(self, comments, summary))]
    async fn submit_review(
        &self,
        comments: &[ReviewComment],
        summary: &str,
        action: ReviewAction,
    ) -> Result<()> {
        let url = self.api(&format!(
            "repos/{}/pulls/{}/reviews",
            self.repo, self.pr_number
        ));

        let event = match action {
            ReviewAction::Approve => "APPROVE",
            ReviewAction::RequestChanges => "REQUEST_CHANGES",
            ReviewAction::Comment => "COMMENT",
        };

        let batch_comments: Vec<BatchReviewComment> = comments
            .iter()
            .filter(|c| c.line > 0)
            .map(|c| {
                let emoji = severity_emoji(&c.severity);
                let body = format_comment(emoji, c);
                BatchReviewComment {
                    path: c.file.clone(),
                    line: c.line,
                    side: "RIGHT".to_string(),
                    body,
                }
            })
            .collect();

        let payload = BatchReviewBody {
            commit_id: self.head_sha.clone(),
            body: summary.to_string(),
            event: event.to_string(),
            comments: batch_comments,
        };

        self.client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "merlin-review/0.1")
            .json(&payload)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| {
                MerlinError::Platform(format!("Failed to submit batch review: {e}"))
            })?;
        Ok(())
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
            .map(|s| format!("\n\n**Suggestion:**\n```\n{s}\n```"))
            .unwrap_or_default(),
    )
}
