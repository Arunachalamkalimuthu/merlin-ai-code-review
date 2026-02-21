/// Tests for the /describe slash command (DescribeTool).
///
/// NOTE: DescribeTool uses `call_ai_raw` which calls `ai.review()` and pulls
/// the JSON out of `comments[0].body`. So MockAi must be set up with
/// `with_comments()`, not `with_generate()`.
mod common;

use std::sync::Arc;

use merlin::ai::{Category, ReviewComment, Severity};
use merlin::platform::PlatformClient;
use merlin::tools::{describe::DescribeTool, MerlinTool, ToolContext};

use common::{make_pr_info, MockAi, MockPlatform};

/// Build a MockAi that returns the given string as raw AI output (in comment body).
fn ai_returning(text: &str) -> MockAi {
    MockAi::with_comments(vec![ReviewComment {
        file: "x".to_string(),
        line: 1,
        severity: Severity::Info,
        category: Category::Style,
        title: "raw".to_string(),
        body: text.to_string(),
        suggestion: None,
    }])
}

fn make_ctx(ai: MockAi, platform: Arc<MockPlatform>) -> ToolContext {
    ToolContext {
        ai: Arc::new(ai),
        platform: Arc::clone(&platform) as Arc<dyn PlatformClient>,
        arg: None,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn describe_updates_pr_description_with_json_response() {
    let ai = ai_returning(
        "{\"title\": \"fix: correct null check\", \"description\": \"Summary: Fixes NPE\"}",
    );
    let platform = Arc::new(MockPlatform::new(common::sample_diff(), make_pr_info()));
    let ctx = make_ctx(ai, Arc::clone(&platform));

    DescribeTool.run(&ctx).await.expect("DescribeTool should succeed");

    let (title, body) = platform
        .last_description()
        .expect("update_description should have been called");

    assert_eq!(title, "fix: correct null check");
    assert!(body.contains("Summary"), "Body should contain description content");
}

#[tokio::test]
async fn describe_falls_back_gracefully_on_non_json() {
    // Empty comments → call_ai_raw falls back to returning the prompt,
    // which is not JSON → describe falls back to existing PR info
    let ai = MockAi::with_comments(vec![]);
    let platform = Arc::new(MockPlatform::new(common::sample_diff(), make_pr_info()));
    let ctx = make_ctx(ai, Arc::clone(&platform));

    let result = DescribeTool.run(&ctx).await;
    assert!(result.is_ok(), "Should not error on non-JSON AI response: {:?}", result);
    assert!(
        platform.last_description().is_some(),
        "update_description should still be called"
    );
}

#[tokio::test]
async fn describe_keeps_existing_title_when_json_omits_title() {
    let ai = ai_returning("{\"description\": \"Changed something\"}");
    let mut pr = make_pr_info();
    pr.title = "existing title".to_string();
    let platform = Arc::new(MockPlatform::new(common::sample_diff(), pr));
    let ctx = make_ctx(ai, Arc::clone(&platform));

    DescribeTool.run(&ctx).await.expect("should succeed");

    let (title, _) = platform.last_description().unwrap();
    assert_eq!(title, "existing title", "Should preserve existing PR title");
}

#[tokio::test]
async fn describe_tool_name() {
    assert_eq!(DescribeTool.name(), "describe");
}

#[tokio::test]
async fn describe_with_multifile_diff() {
    let diff = "\
diff --git a/src/auth.rs b/src/auth.rs
index 000..111 100644
--- a/src/auth.rs
+++ b/src/auth.rs
@@ -1,3 +1,4 @@
 fn verify_token(t: &str) -> bool {
+    // extra check
     t.len() > 10
 }
diff --git a/src/db.rs b/src/db.rs
index 000..222 100644
--- a/src/db.rs
+++ b/src/db.rs
@@ -1,2 +1,3 @@
 pub fn connect() {}
+pub fn disconnect() {}
";
    let ai = ai_returning(
        "{\"title\": \"feat: add disconnect\", \"description\": \"Added disconnect fn\"}",
    );
    let platform = Arc::new(MockPlatform::new(diff, make_pr_info()));
    let ctx = make_ctx(ai, Arc::clone(&platform));

    DescribeTool.run(&ctx).await.expect("should handle multiple files");
    assert!(platform.last_description().is_some());
}

#[tokio::test]
async fn describe_with_empty_diff_does_not_panic() {
    let ai = MockAi::with_comments(vec![]);
    let platform = Arc::new(MockPlatform::new("", make_pr_info()));
    let ctx = make_ctx(ai, Arc::clone(&platform));

    // May fail because the platform diff is empty; should not panic
    let _ = DescribeTool.run(&ctx).await;
}
