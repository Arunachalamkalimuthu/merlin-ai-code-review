//! Shared test helpers: mock AI provider and mock platform client.
//!
//! Import in any integration test file with:
//!   mod common;
//!   use common::{MockAi, MockPlatform, make_pr_info, make_comment};
#![allow(dead_code)]
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use merlin::ai::{AiProvider, Category, ReviewComment, ReviewContext, Severity};
use merlin::error::Result;
use merlin::platform::{InlineCodeSuggestion, Issue, PlatformClient, PrInfo};

// ── Mock AI provider ──────────────────────────────────────────────────────────

/// Configurable mock that returns fixed review comments or generate text.
pub struct MockAi {
    /// Returned by `review()`
    pub comments: Vec<ReviewComment>,
    /// Returned by `generate()`
    pub generate_text: String,
}

impl MockAi {
    pub fn new() -> Self {
        MockAi {
            comments: vec![],
            generate_text: String::new(),
        }
    }

    pub fn with_generate(text: impl Into<String>) -> Self {
        MockAi {
            comments: vec![],
            generate_text: text.into(),
        }
    }

    pub fn with_comments(comments: Vec<ReviewComment>) -> Self {
        MockAi {
            comments,
            generate_text: String::new(),
        }
    }
}

#[async_trait]
impl AiProvider for MockAi {
    async fn review(&self, _ctx: &ReviewContext) -> Result<Vec<ReviewComment>> {
        Ok(self.comments.clone())
    }

    async fn generate(&self, _system: &str, _user: &str) -> Result<String> {
        Ok(self.generate_text.clone())
    }
}

// ── Mock platform ─────────────────────────────────────────────────────────────

/// Captures calls and returns configurable canned values.
pub struct MockPlatform {
    pub diff: String,
    pub pr_info: PrInfo,

    // Capture what was actually written
    pub posted_summary: Mutex<Option<String>>,
    pub posted_comments: Mutex<Vec<ReviewComment>>,
    pub updated_description: Mutex<Option<(String, String)>>,
    pub set_labels: Mutex<Option<Vec<String>>>,
    pub updated_file: Mutex<Option<(String, String)>>,
}

impl MockPlatform {
    pub fn new(diff: impl Into<String>, pr_info: PrInfo) -> Self {
        MockPlatform {
            diff: diff.into(),
            pr_info,
            posted_summary: Mutex::new(None),
            posted_comments: Mutex::new(vec![]),
            updated_description: Mutex::new(None),
            set_labels: Mutex::new(None),
            updated_file: Mutex::new(None),
        }
    }

    pub fn last_description(&self) -> Option<(String, String)> {
        self.updated_description.lock().unwrap().clone()
    }

    pub fn last_summary(&self) -> Option<String> {
        self.posted_summary.lock().unwrap().clone()
    }

    pub fn comment_count(&self) -> usize {
        self.posted_comments.lock().unwrap().len()
    }
}

#[async_trait]
impl PlatformClient for MockPlatform {
    async fn get_diff(&self) -> Result<String> {
        Ok(self.diff.clone())
    }

    async fn post_inline_comment(&self, comment: &ReviewComment) -> Result<()> {
        self.posted_comments.lock().unwrap().push(comment.clone());
        Ok(())
    }

    async fn post_summary(&self, summary: &str) -> Result<()> {
        *self.posted_summary.lock().unwrap() = Some(summary.to_string());
        Ok(())
    }

    async fn get_pr_info(&self) -> Result<PrInfo> {
        Ok(self.pr_info.clone())
    }

    async fn update_description(&self, title: &str, body: &str) -> Result<()> {
        *self.updated_description.lock().unwrap() = Some((title.to_string(), body.to_string()));
        Ok(())
    }

    async fn set_labels(&self, labels: &[String]) -> Result<()> {
        *self.set_labels.lock().unwrap() = Some(labels.to_vec());
        Ok(())
    }

    async fn list_issues(&self, _limit: usize) -> Result<Vec<Issue>> {
        Ok(vec![])
    }

    async fn post_code_suggestions(&self, _suggestions: &[InlineCodeSuggestion]) -> Result<()> {
        Ok(())
    }

    async fn update_file(
        &self,
        path: &str,
        content: &str,
        _message: &str,
        _current_sha: Option<&str>,
        _branch: Option<&str>,
    ) -> Result<()> {
        *self.updated_file.lock().unwrap() = Some((path.to_string(), content.to_string()));
        Ok(())
    }

    async fn get_file(&self, _path: &str) -> Result<Option<(String, String)>> {
        Ok(None)
    }
}

// ── Builders ──────────────────────────────────────────────────────────────────

pub fn make_pr_info() -> PrInfo {
    PrInfo {
        number: 42,
        title: "feat: add new feature".to_string(),
        body: String::new(),
        head_sha: "abc123".to_string(),
        base_branch: "main".to_string(),
        head_branch: "feat/new-feature".to_string(),
        author: "alice".to_string(),
        is_draft: false,
        labels: vec![],
        files_changed: 2,
        additions: 50,
        deletions: 10,
    }
}

pub fn make_comment(file: &str, line: u32, sev: Severity) -> ReviewComment {
    ReviewComment {
        file: file.to_string(),
        line,
        severity: sev,
        category: Category::Bug,
        title: "Test issue".to_string(),
        body: "This is a test issue body.".to_string(),
        suggestion: None,
    }
}

/// A minimal valid unified diff touching two files.
pub fn sample_diff() -> &'static str {
    "\
diff --git a/src/main.rs b/src/main.rs
index 000..111 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,4 @@
 fn main() {
+    println!(\"hello\");
     // existing code
 }
diff --git a/src/lib.rs b/src/lib.rs
index 000..222 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,2 +1,3 @@
 pub mod utils;
+pub mod new_module;
"
}

pub fn arc_ai(ai: MockAi) -> Arc<dyn AiProvider> {
    Arc::new(ai)
}

pub fn arc_platform(p: MockPlatform) -> Arc<dyn PlatformClient> {
    Arc::new(p)
}
