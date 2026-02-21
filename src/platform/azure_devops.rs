//! Azure DevOps platform client (REST API v7.1).
//!
//! Auth: PAT via `AZURE_DEVOPS_TOKEN` or `SYSTEM_ACCESSTOKEN` (Azure Pipelines).
//!
//! Auto-detected from Azure Pipelines env:
//!   TF_BUILD=True, SYSTEM_TEAMFOUNDATIONCOLLECTIONURI, SYSTEM_TEAMPROJECT,
//!   BUILD_REPOSITORY_ID, SYSTEM_PULLREQUEST_PULLREQUESTID, BUILD_SOURCEVERSION

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use super::{InlineCodeSuggestion, Issue, PlatformClient, PrInfo};
use crate::ai::ReviewComment;
use crate::error::{MerlinError, Result};

const API_VERSION: &str = "7.1";

pub struct AzureDevOpsClient {
    token: String,
    /// e.g. "https://dev.azure.com/myorg"
    org_url: String,
    project: String,
    repo_id: String,
    pr_id: u64,
    head_sha: String,
    target_branch: String,
    client: reqwest::Client,
}

impl AzureDevOpsClient {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        token: String,
        org_url: String,
        project: String,
        repo_id: String,
        pr_id: u64,
        head_sha: String,
        target_branch: String,
    ) -> Self {
        Self {
            token,
            org_url,
            project,
            repo_id,
            pr_id,
            head_sha,
            target_branch,
            client: reqwest::Client::new(),
        }
    }

    /// Build from Azure Pipelines environment variables.
    pub fn from_env(token: String) -> Result<Self> {
        // Collection URI e.g. https://dev.azure.com/myorg/
        let collection_uri = std::env::var("SYSTEM_TEAMFOUNDATIONCOLLECTIONURI")
            .map_err(|_| MerlinError::EnvVar("SYSTEM_TEAMFOUNDATIONCOLLECTIONURI".to_string()))?;
        let org_url = collection_uri.trim_end_matches('/').to_string();

        let project = std::env::var("SYSTEM_TEAMPROJECT")
            .map_err(|_| MerlinError::EnvVar("SYSTEM_TEAMPROJECT".to_string()))?;

        let repo_id = std::env::var("BUILD_REPOSITORY_ID")
            .or_else(|_| std::env::var("BUILD_REPOSITORY_NAME"))
            .map_err(|_| MerlinError::EnvVar("BUILD_REPOSITORY_ID".to_string()))?;

        let pr_id: u64 = std::env::var("SYSTEM_PULLREQUEST_PULLREQUESTID")
            .map_err(|_| MerlinError::EnvVar("SYSTEM_PULLREQUEST_PULLREQUESTID".to_string()))?
            .parse()
            .map_err(|_| {
                MerlinError::Config("Invalid SYSTEM_PULLREQUEST_PULLREQUESTID".to_string())
            })?;

        let head_sha = std::env::var("BUILD_SOURCEVERSION")
            .map_err(|_| MerlinError::EnvVar("BUILD_SOURCEVERSION".to_string()))?;

        let target_branch =
            std::env::var("SYSTEM_PULLREQUEST_TARGETBRANCH").unwrap_or_else(|_| "main".to_string());

        Ok(Self::new(
            token,
            org_url,
            project,
            repo_id,
            pr_id,
            head_sha,
            target_branch,
        ))
    }

    /// Base API URL for git operations.
    fn git_url(&self, path: &str) -> String {
        format!(
            "{}/{}/{}/_apis/git/repositories/{}/{}?api-version={API_VERSION}",
            self.org_url, self.project, self.project, self.repo_id, path
        )
    }

    fn git_url_extra(&self, path: &str, extra: &str) -> String {
        format!(
            "{}/{}/{}/_apis/git/repositories/{}/{}?api-version={API_VERSION}&{extra}",
            self.org_url, self.project, self.project, self.repo_id, path
        )
    }

    fn bearer(&self) -> String {
        // Azure DevOps PATs are sent as Basic auth with empty username
        use base64::{engine::general_purpose::STANDARD, Engine};
        format!("Basic {}", STANDARD.encode(format!(":{}", self.token)))
    }

    async fn get_pr_target_sha(&self) -> Option<String> {
        let url = self.git_url(&format!("pullRequests/{}", self.pr_id));
        #[derive(Deserialize)]
        struct Pr {
            #[serde(rename = "lastMergeTargetCommit")]
            last_merge_target_commit: Option<Commit>,
        }
        #[derive(Deserialize)]
        struct Commit {
            #[serde(rename = "commitId")]
            commit_id: String,
        }
        self.client
            .get(&url)
            .header("Authorization", self.bearer())
            .send()
            .await
            .ok()?
            .json::<Pr>()
            .await
            .ok()?
            .last_merge_target_commit
            .map(|c| c.commit_id)
    }
}

// ── Azure DevOps API types ────────────────────────────────────────────────────

#[derive(Deserialize)]
struct AdoPr {
    #[serde(rename = "pullRequestId")]
    pull_request_id: u64,
    title: String,
    description: Option<String>,
    #[serde(rename = "sourceRefName")]
    source_ref_name: String,
    #[serde(rename = "targetRefName")]
    target_ref_name: String,
    #[serde(rename = "createdBy")]
    created_by: AdoIdentity,
    #[serde(rename = "isDraft")]
    is_draft: Option<bool>,
    labels: Option<Vec<AdoLabel>>,
}

#[derive(Deserialize)]
struct AdoIdentity {
    #[serde(rename = "displayName")]
    display_name: String,
}

#[derive(Deserialize)]
struct AdoLabel {
    name: String,
}

#[derive(Deserialize)]
struct AdoIterationChanges {
    #[serde(rename = "changeEntries")]
    change_entries: Vec<AdoChangeEntry>,
}

#[derive(Deserialize)]
struct AdoChangeEntry {
    item: AdoItem,
    #[serde(rename = "changeType")]
    change_type: String,
}

#[derive(Deserialize, Clone)]
struct AdoItem {
    path: String,
    #[serde(rename = "isFolder")]
    is_folder: Option<bool>,
}

#[derive(Deserialize)]
struct AdoWorkItemQueryResult {
    #[serde(rename = "workItems")]
    work_items: Vec<AdoWorkItemRef>,
}

#[derive(Deserialize)]
struct AdoWorkItemRef {
    id: u64,
}

#[derive(Deserialize)]
struct AdoWorkItem {
    id: u64,
    fields: AdoWorkItemFields,
}

#[derive(Deserialize)]
struct AdoWorkItemFields {
    #[serde(rename = "System.Title")]
    title: String,
    #[serde(rename = "System.Description")]
    description: Option<String>,
    #[serde(rename = "System.Tags")]
    tags: Option<String>,
}

#[derive(Serialize)]
struct AdoCommentThread<'a> {
    comments: Vec<AdoComment<'a>>,
    status: u8, // 1 = active
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "threadContext")]
    thread_context: Option<AdoThreadContext<'a>>,
}

#[derive(Serialize)]
struct AdoComment<'a> {
    content: &'a str,
    #[serde(rename = "commentType")]
    comment_type: u8, // 1 = text
}

#[derive(Serialize)]
struct AdoThreadContext<'a> {
    #[serde(rename = "filePath")]
    file_path: &'a str,
    #[serde(rename = "rightFileStart")]
    right_file_start: AdoFilePosition,
    #[serde(rename = "rightFileEnd")]
    right_file_end: AdoFilePosition,
}

#[derive(Serialize)]
struct AdoFilePosition {
    line: u32,
    offset: u32,
}

#[derive(Serialize)]
struct AdoUpdatePr<'a> {
    title: &'a str,
    description: &'a str,
}

#[derive(Serialize)]
struct AdoLabelBody<'a> {
    name: &'a str,
}

#[derive(Serialize)]
struct AdoWiql<'a> {
    query: &'a str,
}

// ─────────────────────────────────────────────────────────────────────────────

#[async_trait]
impl PlatformClient for AzureDevOpsClient {
    #[instrument(skip(self))]
    async fn get_diff(&self) -> Result<String> {
        debug!("Fetching Azure DevOps PR changes for PR #{}", self.pr_id);

        // 1. Get latest iteration ID
        let iter_url = self.git_url(&format!("pullRequests/{}/iterations", self.pr_id));
        #[derive(Deserialize)]
        struct IterList {
            value: Vec<Iter>,
        }
        #[derive(Deserialize)]
        struct Iter {
            id: u64,
        }

        let iters: IterList = self
            .client
            .get(&iter_url)
            .header("Authorization", self.bearer())
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("ADO iterations: {e}")))?
            .json()
            .await?;

        let iter_id = iters.value.iter().map(|i| i.id).max().unwrap_or(1);

        // 2. Get changed files
        let changes_url = self.git_url(&format!(
            "pullRequests/{}/iterations/{}/changes",
            self.pr_id, iter_id
        ));
        let changes: AdoIterationChanges = self
            .client
            .get(&changes_url)
            .header("Authorization", self.bearer())
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("ADO changes: {e}")))?
            .json()
            .await?;

        let target_sha = self.get_pr_target_sha().await.unwrap_or_default();

        // 3. For each changed file, fetch content at source and target and build pseudo-diff
        let mut diff = String::new();
        for entry in &changes.change_entries {
            if entry.item.is_folder.unwrap_or(false) {
                continue;
            }
            let file_path = entry.item.path.trim_start_matches('/');
            let change_type = entry.change_type.to_lowercase();

            let new_content = self.fetch_file_at(file_path, &self.head_sha).await;
            let old_content = if !target_sha.is_empty() {
                self.fetch_file_at(file_path, &target_sha).await
            } else {
                None
            };

            diff.push_str(&build_pseudo_diff(
                file_path,
                old_content.as_deref(),
                new_content.as_deref(),
                &change_type,
            ));
        }

        Ok(diff)
    }

    #[instrument(skip(self, comment))]
    async fn post_inline_comment(&self, comment: &ReviewComment) -> Result<()> {
        let url = self.git_url(&format!("pullRequests/{}/threads", self.pr_id));
        let emoji = severity_emoji(&comment.severity);
        let body_text = format_comment(emoji, comment);

        let file_path = format!("/{}", comment.file.trim_start_matches('/'));
        let payload = AdoCommentThread {
            comments: vec![AdoComment {
                content: &body_text,
                comment_type: 1,
            }],
            status: 1,
            thread_context: Some(AdoThreadContext {
                file_path: &file_path,
                right_file_start: AdoFilePosition {
                    line: comment.line,
                    offset: 1,
                },
                right_file_end: AdoFilePosition {
                    line: comment.line,
                    offset: 1,
                },
            }),
        };

        self.client
            .post(&url)
            .header("Authorization", self.bearer())
            .json(&payload)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("ADO inline comment: {e}")))?;
        Ok(())
    }

    #[instrument(skip(self, summary))]
    async fn post_summary(&self, summary: &str) -> Result<()> {
        let url = self.git_url(&format!("pullRequests/{}/threads", self.pr_id));
        let payload = AdoCommentThread {
            comments: vec![AdoComment {
                content: summary,
                comment_type: 1,
            }],
            status: 1,
            thread_context: None,
        };

        self.client
            .post(&url)
            .header("Authorization", self.bearer())
            .json(&payload)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("ADO summary: {e}")))?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_pr_info(&self) -> Result<PrInfo> {
        let url = self.git_url(&format!("pullRequests/{}", self.pr_id));
        let pr: AdoPr = self
            .client
            .get(&url)
            .header("Authorization", self.bearer())
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("ADO PR info: {e}")))?
            .json()
            .await?;

        let strip_refs = |r: &str| r.replace("refs/heads/", "");

        Ok(PrInfo {
            number: pr.pull_request_id,
            title: pr.title,
            body: pr.description.unwrap_or_default(),
            head_sha: self.head_sha.clone(),
            base_branch: strip_refs(&pr.target_ref_name),
            head_branch: strip_refs(&pr.source_ref_name),
            author: pr.created_by.display_name,
            is_draft: pr.is_draft.unwrap_or(false),
            labels: pr
                .labels
                .unwrap_or_default()
                .into_iter()
                .map(|l| l.name)
                .collect(),
            files_changed: 0,
            additions: 0,
            deletions: 0,
        })
    }

    #[instrument(skip(self))]
    async fn update_description(&self, title: &str, body: &str) -> Result<()> {
        let url = self.git_url(&format!("pullRequests/{}", self.pr_id));
        let payload = AdoUpdatePr {
            title,
            description: body,
        };

        self.client
            .patch(&url)
            .header("Authorization", self.bearer())
            .json(&payload)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("ADO update PR: {e}")))?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn set_labels(&self, labels: &[String]) -> Result<()> {
        // Azure DevOps supports PR labels via the labels API (preview)
        let url = format!(
            "{}/{}/{}/_apis/git/repositories/{}/pullRequests/{}/labels?api-version=7.1-preview.1",
            self.org_url, self.project, self.project, self.repo_id, self.pr_id
        );
        for label in labels {
            let payload = AdoLabelBody { name: label };
            self.client
                .post(&url)
                .header("Authorization", self.bearer())
                .json(&payload)
                .send()
                .await?
                .error_for_status()
                .map_err(|e| MerlinError::Platform(format!("ADO set label '{label}': {e}")))?;
        }
        Ok(())
    }

    #[instrument(skip(self))]
    async fn list_issues(&self, limit: usize) -> Result<Vec<Issue>> {
        let wit_url = format!(
            "{}/{}/_apis/wit/wiql?api-version={API_VERSION}",
            self.org_url, self.project
        );
        let query = "SELECT [System.Id],[System.Title],[System.Description] FROM WorkItems \
             WHERE [System.State] IN ('Active','New','Open') \
             ORDER BY [System.ChangedDate] DESC";
        let body = AdoWiql { query };

        let result: AdoWorkItemQueryResult = self
            .client
            .post(&wit_url)
            .header("Authorization", self.bearer())
            .json(&body)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("ADO WIQL: {e}")))?
            .json()
            .await?;

        let ids: Vec<u64> = result.work_items.iter().take(limit).map(|w| w.id).collect();
        if ids.is_empty() {
            return Ok(vec![]);
        }

        let ids_str = ids
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let items_url = format!(
            "{}/{}/_apis/wit/workItems?ids={}&fields=System.Title,System.Description,System.Tags&api-version={API_VERSION}",
            self.org_url, self.project, ids_str
        );

        #[derive(Deserialize)]
        struct WorkItemList {
            value: Vec<AdoWorkItem>,
        }
        let items: WorkItemList = self
            .client
            .get(&items_url)
            .header("Authorization", self.bearer())
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("ADO work items: {e}")))?
            .json()
            .await?;

        Ok(items
            .value
            .into_iter()
            .map(|i| Issue {
                number: i.id,
                title: i.fields.title,
                body: i.fields.description.unwrap_or_default(),
                labels: i
                    .fields
                    .tags
                    .unwrap_or_default()
                    .split(';')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
                url: format!("{}/_workitems/edit/{}", self.org_url, i.id),
            })
            .collect())
    }

    async fn post_code_suggestions(&self, suggestions: &[InlineCodeSuggestion]) -> Result<()> {
        // Azure DevOps doesn't support one-click suggestion blocks; post as thread comments.
        for s in suggestions {
            let body = format!("{}\n\n```suggestion\n{}\n```", s.description, s.suggestion);
            let file_path = format!("/{}", s.file.trim_start_matches('/'));
            let url = self.git_url(&format!("pullRequests/{}/threads", self.pr_id));
            let payload = AdoCommentThread {
                comments: vec![AdoComment {
                    content: &body,
                    comment_type: 1,
                }],
                status: 1,
                thread_context: Some(AdoThreadContext {
                    file_path: &file_path,
                    right_file_start: AdoFilePosition {
                        line: s.start_line,
                        offset: 1,
                    },
                    right_file_end: AdoFilePosition {
                        line: s.end_line,
                        offset: 1,
                    },
                }),
            };
            self.client
                .post(&url)
                .header("Authorization", self.bearer())
                .json(&payload)
                .send()
                .await?
                .error_for_status()
                .map_err(|e| MerlinError::Platform(format!("ADO suggestion: {e}")))?;
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
        // Need to get the current branch SHA first
        let branch_url =
            self.git_url_extra(&format!("refs?filter=heads/{}", self.target_branch), "");
        #[derive(Deserialize)]
        struct RefList {
            value: Vec<GitRef>,
        }
        #[derive(Deserialize)]
        struct GitRef {
            #[serde(rename = "objectId")]
            object_id: String,
        }

        let refs: RefList = self
            .client
            .get(&branch_url)
            .header("Authorization", self.bearer())
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("ADO refs: {e}")))?
            .json()
            .await?;

        let old_sha = refs
            .value
            .first()
            .map(|r| r.object_id.clone())
            .unwrap_or_else(|| "0000000000000000000000000000000000000000".to_string());

        let push_url = self.git_url("pushes");
        let normalized_path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{path}")
        };

        let payload = serde_json::json!({
            "refUpdates": [{
                "name": format!("refs/heads/{}", self.target_branch),
                "oldObjectId": old_sha
            }],
            "commits": [{
                "comment": message,
                "changes": [{
                    "changeType": "edit",
                    "item": {"path": normalized_path},
                    "newContent": {
                        "content": content,
                        "contentType": "rawtext"
                    }
                }]
            }]
        });

        self.client
            .post(&push_url)
            .header("Authorization", self.bearer())
            .json(&payload)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("ADO update file: {e}")))?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_file(&self, path: &str) -> Result<Option<(String, String)>> {
        let normalized = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{path}")
        };
        let url =
            format!(
            "{}/{}/{}/_apis/git/repositories/{}/items?path={}&version={}&api-version={API_VERSION}",
            self.org_url, self.project, self.project, self.repo_id,
            urlencoding(&normalized), self.head_sha
        );

        let resp = self
            .client
            .get(&url)
            .header("Authorization", self.bearer())
            .send()
            .await?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let text = resp
            .error_for_status()
            .map_err(|e| MerlinError::Platform(format!("ADO get file: {e}")))?
            .text()
            .await?;

        Ok(Some((text, self.head_sha.clone())))
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

impl AzureDevOpsClient {
    async fn fetch_file_at(&self, path: &str, sha: &str) -> Option<String> {
        let normalized = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{path}")
        };
        let url =
            format!(
            "{}/{}/{}/_apis/git/repositories/{}/items?path={}&version={}&api-version={API_VERSION}",
            self.org_url, self.project, self.project, self.repo_id,
            urlencoding(&normalized), sha
        );
        self.client
            .get(&url)
            .header("Authorization", self.bearer())
            .send()
            .await
            .ok()?
            .text()
            .await
            .ok()
    }
}

fn urlencoding(s: &str) -> String {
    s.replace('/', "%2F").replace(' ', "%20")
}

/// Build a pseudo unified diff from old and new content strings.
fn build_pseudo_diff(
    file_path: &str,
    old: Option<&str>,
    new: Option<&str>,
    change_type: &str,
) -> String {
    match (change_type, old, new) {
        ("delete", Some(old_content), _) => {
            let mut diff = format!(
                "--- a/{file_path}\n+++ /dev/null\n@@ -1,{} +0,0 @@\n",
                old_content.lines().count()
            );
            for line in old_content.lines() {
                diff.push('-');
                diff.push_str(line);
                diff.push('\n');
            }
            diff
        }
        ("add", _, Some(new_content)) => {
            let mut diff = format!(
                "--- /dev/null\n+++ b/{file_path}\n@@ -0,0 +1,{} @@\n",
                new_content.lines().count()
            );
            for line in new_content.lines() {
                diff.push('+');
                diff.push_str(line);
                diff.push('\n');
            }
            diff
        }
        (_, Some(old_content), Some(new_content)) => {
            // Simple line-by-line diff: show removals then additions
            let old_lines: Vec<&str> = old_content.lines().collect();
            let new_lines: Vec<&str> = new_content.lines().collect();
            let mut diff = format!(
                "--- a/{file_path}\n+++ b/{file_path}\n@@ -1,{} +1,{} @@\n",
                old_lines.len(),
                new_lines.len()
            );
            // Show removed lines then added lines (simplified, not true LCS diff)
            for line in &old_lines {
                if !new_lines.contains(line) {
                    diff.push('-');
                    diff.push_str(line);
                    diff.push('\n');
                }
            }
            for line in &new_lines {
                if !old_lines.contains(line) {
                    diff.push('+');
                    diff.push_str(line);
                    diff.push('\n');
                }
            }
            diff
        }
        (_, _, Some(new_content)) => {
            let mut diff = format!(
                "--- /dev/null\n+++ b/{file_path}\n@@ -0,0 +1,{} @@\n",
                new_content.lines().count()
            );
            for line in new_content.lines() {
                diff.push('+');
                diff.push_str(line);
                diff.push('\n');
            }
            diff
        }
        _ => String::new(),
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
