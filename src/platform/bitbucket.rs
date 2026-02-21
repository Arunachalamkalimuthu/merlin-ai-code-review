//! Bitbucket Cloud platform client (REST API v2.0).
//!
//! Auth: Bearer token via `BITBUCKET_TOKEN`, or Basic auth via
//!       `BITBUCKET_USERNAME` + `BITBUCKET_APP_PASSWORD`.
//!
//! Auto-detected from Bitbucket Pipelines env:
//!   BITBUCKET_PIPELINE_UUID, BITBUCKET_WORKSPACE, BITBUCKET_REPO_SLUG,
//!   BITBUCKET_PR_ID, BITBUCKET_COMMIT

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument, warn};

use super::{InlineCodeSuggestion, Issue, PlatformClient, PrInfo};
use crate::ai::ReviewComment;
use crate::error::{MerlinError, Result};

const BB_API: &str = "https://api.bitbucket.org/2.0";

pub struct BitbucketClient {
    auth: BitbucketAuth,
    workspace: String,
    repo_slug: String,
    pr_id: u64,
    head_sha: String,
    client: reqwest::Client,
}

pub(crate) enum BitbucketAuth {
    Bearer(String),
    Basic { username: String, password: String },
}

impl BitbucketClient {
    pub(crate) fn new(
        auth: BitbucketAuth,
        workspace: String,
        repo_slug: String,
        pr_id: u64,
        head_sha: String,
    ) -> Self {
        Self {
            auth,
            workspace,
            repo_slug,
            pr_id,
            head_sha,
            client: reqwest::Client::new(),
        }
    }

    /// Build from Bitbucket Pipelines environment variables.
    pub fn from_env(token: String) -> Result<Self> {
        let workspace = std::env::var("BITBUCKET_WORKSPACE")
            .map_err(|_| MerlinError::EnvVar("BITBUCKET_WORKSPACE".to_string()))?;
        let repo_slug = std::env::var("BITBUCKET_REPO_SLUG")
            .map_err(|_| MerlinError::EnvVar("BITBUCKET_REPO_SLUG".to_string()))?;
        let pr_id: u64 = std::env::var("BITBUCKET_PR_ID")
            .map_err(|_| MerlinError::EnvVar("BITBUCKET_PR_ID".to_string()))?
            .parse()
            .map_err(|_| MerlinError::Config("Invalid BITBUCKET_PR_ID".to_string()))?;
        let head_sha = std::env::var("BITBUCKET_COMMIT")
            .map_err(|_| MerlinError::EnvVar("BITBUCKET_COMMIT".to_string()))?;

        let auth = if let Ok(username) = std::env::var("BITBUCKET_USERNAME") {
            BitbucketAuth::Basic {
                username,
                password: token,
            }
        } else {
            BitbucketAuth::Bearer(token)
        };

        Ok(Self::new(auth, workspace, repo_slug, pr_id, head_sha))
    }

    fn repo_url(&self, path: &str) -> String {
        format!(
            "{BB_API}/repositories/{}/{}/{}",
            self.workspace, self.repo_slug, path
        )
    }

    fn add_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.auth {
            BitbucketAuth::Bearer(token) => req.bearer_auth(token),
            BitbucketAuth::Basic { username, password } => req.basic_auth(username, Some(password)),
        }
    }
}

// ── Bitbucket API types ───────────────────────────────────────────────────────

#[derive(Deserialize)]
struct BbPr {
    id: u64,
    title: String,
    description: Option<String>,
    source: BbPrRef,
    destination: BbPrRef,
    author: BbUser,
    state: String,
}

#[derive(Deserialize)]
struct BbPrRef {
    branch: BbBranch,
    commit: BbCommit,
}

#[derive(Deserialize)]
struct BbBranch {
    name: String,
}

#[derive(Deserialize)]
struct BbCommit {
    hash: String,
}

#[derive(Deserialize)]
struct BbUser {
    display_name: String,
}

#[derive(Deserialize)]
struct BbDiffStat {
    values: Vec<BbFileStat>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct BbFileStat {
    #[serde(rename = "type")]
    stat_type: String,
    lines_added: Option<u32>,
    lines_removed: Option<u32>,
}

#[derive(Deserialize)]
struct BbIssue {
    id: u64,
    title: String,
    content: Option<BbContent>,
    component: Option<BbComponent>,
}

#[derive(Deserialize)]
struct BbContent {
    raw: String,
}

#[derive(Deserialize)]
struct BbComponent {
    name: String,
}

#[derive(Deserialize)]
struct BbIssueList {
    values: Vec<BbIssue>,
}

#[derive(Serialize)]
struct BbCommentBody<'a> {
    content: BbContentRaw<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inline: Option<BbInline<'a>>,
}

#[derive(Serialize)]
struct BbContentRaw<'a> {
    raw: &'a str,
}

#[derive(Serialize)]
struct BbInline<'a> {
    path: &'a str,
    to: u32,
}

#[derive(Serialize)]
struct BbUpdatePr<'a> {
    title: &'a str,
    description: &'a str,
}

// ─────────────────────────────────────────────────────────────────────────────

#[async_trait]
impl PlatformClient for BitbucketClient {
    #[instrument(skip(self))]
    async fn get_diff(&self) -> Result<String> {
        let url = self.repo_url(&format!("pullrequests/{}/diff", self.pr_id));
        debug!("Fetching Bitbucket PR diff from: {url}");

        let resp = self
            .add_auth(self.client.get(&url))
            .header("Accept", "text/plain")
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Bitbucket diff error: {e}")))?;

        Ok(resp.text().await?)
    }

    #[instrument(skip(self, comment))]
    async fn post_inline_comment(&self, comment: &ReviewComment) -> Result<()> {
        let url = self.repo_url(&format!("pullrequests/{}/comments", self.pr_id));
        let emoji = severity_emoji(&comment.severity);
        let body_text = format_comment(emoji, comment);

        let payload = BbCommentBody {
            content: BbContentRaw { raw: &body_text },
            inline: Some(BbInline {
                path: &comment.file,
                to: comment.line,
            }),
        };

        self.add_auth(self.client.post(&url))
            .json(&payload)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Bitbucket comment error: {e}")))?;
        Ok(())
    }

    #[instrument(skip(self, summary))]
    async fn post_summary(&self, summary: &str) -> Result<()> {
        let url = self.repo_url(&format!("pullrequests/{}/comments", self.pr_id));
        let payload = BbCommentBody {
            content: BbContentRaw { raw: summary },
            inline: None,
        };

        self.add_auth(self.client.post(&url))
            .json(&payload)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Bitbucket summary error: {e}")))?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_pr_info(&self) -> Result<PrInfo> {
        let url = self.repo_url(&format!("pullrequests/{}", self.pr_id));
        let pr: BbPr = self
            .add_auth(self.client.get(&url))
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Bitbucket PR info error: {e}")))?
            .json()
            .await?;

        // Get diff stats for addition/deletion counts
        let stats_url = self.repo_url(&format!("pullrequests/{}/diffstat", self.pr_id));
        let stats: BbDiffStat = async {
            self.add_auth(self.client.get(&stats_url))
                .send()
                .await?
                .json::<BbDiffStat>()
                .await
        }
        .await
        .unwrap_or(BbDiffStat { values: vec![] });

        let additions: u32 = stats.values.iter().filter_map(|s| s.lines_added).sum();
        let deletions: u32 = stats.values.iter().filter_map(|s| s.lines_removed).sum();

        Ok(PrInfo {
            number: pr.id,
            title: pr.title,
            body: pr.description.unwrap_or_default(),
            head_sha: pr.source.commit.hash,
            base_branch: pr.destination.branch.name,
            head_branch: pr.source.branch.name,
            author: pr.author.display_name,
            is_draft: pr.state == "DRAFT",
            labels: vec![], // Bitbucket Cloud has no PR labels
            files_changed: stats.values.len() as u32,
            additions,
            deletions,
        })
    }

    #[instrument(skip(self))]
    async fn update_description(&self, title: &str, body: &str) -> Result<()> {
        let url = self.repo_url(&format!("pullrequests/{}", self.pr_id));
        let payload = BbUpdatePr {
            title,
            description: body,
        };

        self.add_auth(self.client.put(&url))
            .json(&payload)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Bitbucket update PR error: {e}")))?;
        Ok(())
    }

    async fn set_labels(&self, labels: &[String]) -> Result<()> {
        // Bitbucket Cloud does not support PR labels natively.
        // Post as a comment instead so the label intent is visible.
        if labels.is_empty() {
            return Ok(());
        }
        warn!("Bitbucket Cloud does not support PR labels; posting as comment");
        let text = format!(
            "**Merlin suggested labels:** {}",
            labels
                .iter()
                .map(|l| format!("`{l}`"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        self.post_summary(&text).await
    }

    #[instrument(skip(self))]
    async fn list_issues(&self, limit: usize) -> Result<Vec<Issue>> {
        let url = self.repo_url(&format!(
            "issues?q=state=\"new\"+OR+state=\"open\"&pagelen={limit}"
        ));
        let list: BbIssueList = self
            .add_auth(self.client.get(&url))
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Bitbucket issues error: {e}")))?
            .json()
            .await?;

        Ok(list
            .values
            .into_iter()
            .map(|i| Issue {
                number: i.id,
                title: i.title,
                body: i.content.map(|c| c.raw).unwrap_or_default(),
                labels: i.component.map(|c| vec![c.name]).unwrap_or_default(),
                url: format!(
                    "https://bitbucket.org/{}/{}/issues/{}",
                    self.workspace, self.repo_slug, i.id
                ),
            })
            .collect())
    }

    async fn post_code_suggestions(&self, suggestions: &[InlineCodeSuggestion]) -> Result<()> {
        // Bitbucket has no native suggestion blocks; post as inline comment with code fence.
        for s in suggestions {
            let body = format!("{}\n\n```suggestion\n{}\n```", s.description, s.suggestion);
            let url = self.repo_url(&format!("pullrequests/{}/comments", self.pr_id));
            let payload = BbCommentBody {
                content: BbContentRaw { raw: &body },
                inline: Some(BbInline {
                    path: &s.file,
                    to: s.end_line,
                }),
            };
            self.add_auth(self.client.post(&url))
                .json(&payload)
                .send()
                .await?
                .error_for_status()
                .map_err(|e| MerlinError::Platform(format!("Bitbucket suggestion error: {e}")))?;
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
        // Bitbucket uses multipart form to update files
        let url = self.repo_url("src");
        let form = reqwest::multipart::Form::new()
            .text("message", message.to_string())
            .text(path.to_string(), content.to_string());

        self.add_auth(self.client.post(&url))
            .multipart(form)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Bitbucket update file error: {e}")))?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_file(&self, path: &str) -> Result<Option<(String, String)>> {
        let url = self.repo_url(&format!("src/{}/{}", self.head_sha, path));
        let resp = self.add_auth(self.client.get(&url)).send().await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        let text = resp
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("Bitbucket get file error: {e}")))?
            .text()
            .await?;

        // Bitbucket src API returns raw content; no separate SHA — use head commit
        Ok(Some((text, self.head_sha.clone())))
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
