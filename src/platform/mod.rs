//! VCS platform abstraction layer.
//!
//! The central abstraction is the [`PlatformClient`] trait.  Every method
//! that interacts with a VCS platform (fetching diffs, posting comments,
//! updating labels, reading files) goes through this trait so the rest of
//! Merlin is platform-agnostic.
//!
//! # Supported platforms
//!
//! | Platform | Module | Auto-detect env var |
//! |----------|--------|---------------------|
//! | GitHub | [`github`] | `GITHUB_ACTIONS=true` |
//! | GitLab | [`gitlab`] | `GITLAB_CI=true` |
//! | Bitbucket | [`bitbucket`] | `BITBUCKET_PIPELINE_UUID` |
//! | Azure DevOps | [`azure_devops`] | `TF_BUILD=True` |
//! | Gitea | [`gitea`] | `GITEA_ACTIONS=true` |
//!
//! Use [`build_client`] to auto-detect the platform from environment
//! variables and instantiate the correct backend.
//!
//! For local / offline mode (e.g. `merlin review --diff file.diff`) use
//! [`NoOpPlatform`], which silently discards all writes and returns empty reads.
pub mod azure_devops;
pub mod bitbucket;
pub mod gitea;
pub mod github;
pub mod gitlab;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::ai::ReviewComment;
use crate::config::{Config, PlatformConfig, PlatformType};
use crate::error::{MerlinError, Result};

// ── Shared data types ─────────────────────────────────────────────────────────

/// Metadata for a pull request or merge request.
///
/// Returned by [`PlatformClient::get_pr_info`].  Fields that are unavailable
/// on a particular platform are set to their zero values (`""`, `0`, `false`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrInfo {
    pub number: u64,
    pub title: String,
    pub body: String,
    pub head_sha: String,
    pub base_branch: String,
    pub head_branch: String,
    pub author: String,
    pub is_draft: bool,
    pub labels: Vec<String>,
    pub files_changed: u32,
    pub additions: u32,
    pub deletions: u32,
}

/// A repository issue, returned by [`PlatformClient::list_issues`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub number: u64,
    pub title: String,
    pub body: String,
    pub labels: Vec<String>,
    pub url: String,
}

/// A multi-line code suggestion to post as a suggestion block on a PR.
///
/// Passed to [`PlatformClient::post_code_suggestions`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineCodeSuggestion {
    pub file: String,
    pub start_line: u32,
    pub end_line: u32,
    pub suggestion: String,
    pub description: String,
}

// ── Platform trait ────────────────────────────────────────────────────────────

/// Trait implemented by all VCS platform backends.
#[async_trait]
pub trait PlatformClient: Send + Sync {
    // ── Core review ops ───────────────────────────────────────────────────
    /// Fetch the raw unified diff for the current PR/MR.
    async fn get_diff(&self) -> Result<String>;

    /// Post a single inline review comment at the specified file/line.
    async fn post_inline_comment(&self, comment: &ReviewComment) -> Result<()>;

    /// Post the overall review summary as a PR/MR comment.
    async fn post_summary(&self, summary: &str) -> Result<()>;

    // ── PR metadata ops ───────────────────────────────────────────────────
    /// Get PR/MR metadata (title, body, labels, stats).
    async fn get_pr_info(&self) -> Result<PrInfo>;

    /// Update the PR/MR title and description.
    async fn update_description(&self, title: &str, body: &str) -> Result<()>;

    /// Set labels on the PR/MR (replaces existing labels).
    async fn set_labels(&self, labels: &[String]) -> Result<()>;

    // ── Issue ops ─────────────────────────────────────────────────────────
    /// List recent open issues (for /similar_issue).
    async fn list_issues(&self, limit: usize) -> Result<Vec<Issue>>;

    // ── File ops ──────────────────────────────────────────────────────────
    /// Post inline code suggestions (multi-line suggestion blocks).
    async fn post_code_suggestions(&self, suggestions: &[InlineCodeSuggestion]) -> Result<()>;

    /// Update a file in the repository (for changelog etc.).
    async fn update_file(
        &self,
        path: &str,
        content: &str,
        message: &str,
        current_sha: Option<&str>,
    ) -> Result<()>;

    /// Get a file's content and SHA from the repo.
    async fn get_file(&self, path: &str) -> Result<Option<(String, String)>>;
}

// ── Factory ───────────────────────────────────────────────────────────────────

/// Auto-detect the CI platform from environment variables and build a client.
///
/// If `cfg.platform_type` is set, that value takes precedence over
/// auto-detection.  Otherwise the function inspects well-known CI environment
/// variables in priority order (see the table in the module docs).
///
/// # Errors
///
/// Returns [`MerlinError::Config`] when:
/// - No platform can be detected and none is specified in `cfg`.
/// - The required VCS token environment variable is not set.
pub fn build_client(cfg: &PlatformConfig) -> Result<Box<dyn PlatformClient>> {
    let platform_type = if let Some(ref t) = cfg.platform_type {
        t.clone()
    } else {
        detect_platform()?
    };

    match platform_type {
        PlatformType::Github => {
            let token = Config::github_token()?;
            Ok(Box::new(github::GitHubClient::from_env(token)?))
        }
        PlatformType::Gitlab => {
            let token = Config::gitlab_token()?;
            Ok(Box::new(gitlab::GitLabClient::from_env(token)?))
        }
        PlatformType::Bitbucket => {
            let token = Config::bitbucket_token()?;
            Ok(Box::new(bitbucket::BitbucketClient::from_env(token)?))
        }
        PlatformType::AzureDevops => {
            let token = Config::azure_devops_token()?;
            Ok(Box::new(azure_devops::AzureDevOpsClient::from_env(token)?))
        }
        PlatformType::Gitea => {
            let token = Config::gitea_token()?;
            Ok(Box::new(gitea::GiteaClient::from_env(token)?))
        }
    }
}

// ── No-op platform (for local/API mode) ──────────────────────────────────────

/// A `PlatformClient` that silently discards all writes and returns empty reads.
/// Used when running `merlin review --diff <file>` or the REST API review endpoint.
pub struct NoOpPlatform;

#[async_trait]
impl PlatformClient for NoOpPlatform {
    async fn get_diff(&self) -> Result<String> {
        Err(MerlinError::Config(
            "NoOpPlatform: get_diff not available in local mode".to_string(),
        ))
    }
    async fn post_inline_comment(&self, _comment: &ReviewComment) -> Result<()> {
        Ok(())
    }
    async fn post_summary(&self, _summary: &str) -> Result<()> {
        Ok(())
    }
    async fn get_pr_info(&self) -> Result<PrInfo> {
        Err(MerlinError::Config(
            "Not available in local mode".to_string(),
        ))
    }
    async fn update_description(&self, _title: &str, _body: &str) -> Result<()> {
        Ok(())
    }
    async fn set_labels(&self, _labels: &[String]) -> Result<()> {
        Ok(())
    }
    async fn list_issues(&self, _limit: usize) -> Result<Vec<Issue>> {
        Ok(vec![])
    }
    async fn post_code_suggestions(&self, _s: &[InlineCodeSuggestion]) -> Result<()> {
        Ok(())
    }
    async fn update_file(&self, _p: &str, _c: &str, _m: &str, _sha: Option<&str>) -> Result<()> {
        Ok(())
    }
    async fn get_file(&self, _path: &str) -> Result<Option<(String, String)>> {
        Ok(None)
    }
}

/// Strip any markdown code fences that the AI may have already added to a
/// suggestion string.  Models sometimes return suggestions wrapped in
/// ` ```lang … ``` ` blocks; if we naively re-wrap those in another fence
/// the rendered comment breaks (double fence / orphan empty block).
///
/// Rules applied in order:
/// 1. Trim surrounding whitespace.
/// 2. If the first line is a fence opener (` ``` ` or ` ```lang `), drop it.
/// 3. If the last non-empty line is a fence closer (` ``` `), drop it.
pub(crate) fn strip_suggestion_fences(s: &str) -> &str {
    let s = s.trim();
    // Strip leading fence line (``` or ```lang)
    let s = if s.starts_with("```") {
        s.split_once('\n').map(|x| x.1).unwrap_or(s).trim_start()
    } else {
        s
    };
    // Strip trailing fence
    let s = s.trim_end();
    if let Some(stripped) = s.strip_suffix("```") {
        stripped.trim_end()
    } else {
        s
    }
}

fn detect_platform() -> Result<PlatformType> {
    // GitHub Actions
    if std::env::var("GITHUB_ACTIONS").is_ok() {
        // Distinguish Gitea Actions (also sets GITHUB_ACTIONS) by GITEA_ACTIONS
        if std::env::var("GITEA_ACTIONS").is_ok() {
            return Ok(PlatformType::Gitea);
        }
        return Ok(PlatformType::Github);
    }
    // GitLab CI
    if std::env::var("GITLAB_CI").is_ok() {
        return Ok(PlatformType::Gitlab);
    }
    // Bitbucket Pipelines
    if std::env::var("BITBUCKET_PIPELINE_UUID").is_ok() {
        return Ok(PlatformType::Bitbucket);
    }
    // Azure Pipelines
    if std::env::var("TF_BUILD").is_ok() {
        return Ok(PlatformType::AzureDevops);
    }
    // Gitea Actions (older versions only set GITEA_ACTIONS, not GITHUB_ACTIONS)
    if std::env::var("GITEA_ACTIONS").is_ok() {
        return Ok(PlatformType::Gitea);
    }
    Err(MerlinError::Config(
        "Could not auto-detect CI platform. Set one of: GITHUB_ACTIONS, GITLAB_CI, \
         BITBUCKET_PIPELINE_UUID, TF_BUILD, or GITEA_ACTIONS. \
         Alternatively set [platform] type in merlin.toml"
            .to_string(),
    ))
}
