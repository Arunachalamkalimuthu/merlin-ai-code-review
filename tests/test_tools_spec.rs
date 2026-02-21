/// Tests for the /spec slash command (SpecTool).
mod common;

use std::sync::Arc;

use merlin::platform::PlatformClient;
use merlin::tools::{spec::SpecTool, MerlinTool, ToolContext};

use common::{make_pr_info, MockAi, MockPlatform};

/// Build a ToolContext keeping a typed Arc so tests can inspect captured state.
fn make_ctx(ai: MockAi, platform: Arc<MockPlatform>) -> ToolContext {
    ToolContext {
        ai: Arc::new(ai),
        // Clone the Arc and coerce — original Arc still lives in caller
        platform: Arc::clone(&platform) as Arc<dyn PlatformClient>,
        arg: None,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn spec_updates_pr_description() {
    let ai = MockAi::with_generate(
        "# feat: add new feature\n\n## Overview\nAdds a shiny new feature.\n\n## Open Questions\nNone."
    );
    let platform = Arc::new(MockPlatform::new(common::sample_diff(), make_pr_info()));
    let ctx = make_ctx(ai, Arc::clone(&platform));

    SpecTool.run(&ctx).await.expect("SpecTool should succeed");

    let (title, body) = platform
        .last_description()
        .expect("update_description should have been called");

    assert_eq!(title, "feat: add new feature");
    assert!(!body.trim_start().starts_with("# "), "H1 must be stripped from the body");
    assert!(body.contains("Overview"), "Body should contain spec content");
}

#[tokio::test]
async fn spec_falls_back_to_pr_title_when_no_h1() {
    // AI response with no leading H1
    let ai = MockAi::with_generate("## Overview\nThis PR does something.");
    let mut pr = make_pr_info();
    pr.title = "original title from GitHub".to_string();
    let platform = Arc::new(MockPlatform::new(common::sample_diff(), pr));
    let ctx = make_ctx(ai, Arc::clone(&platform));

    SpecTool.run(&ctx).await.expect("should succeed without H1");

    let (title, _) = platform
        .last_description()
        .expect("update_description should have been called");

    assert_eq!(title, "original title from GitHub");
}

#[tokio::test]
async fn spec_handles_empty_existing_description() {
    let ai = MockAi::with_generate("# PR Title\n\n## Overview\nNew PR.");
    let mut pr = make_pr_info();
    pr.body = String::new();
    let platform = Arc::new(MockPlatform::new(common::sample_diff(), pr));
    let ctx = make_ctx(ai, Arc::clone(&platform));

    // Must not panic on empty body
    SpecTool.run(&ctx).await.expect("should handle empty PR body");
    assert!(platform.last_description().is_some());
}

#[tokio::test]
async fn spec_output_announces_update() {
    let ai = MockAi::with_generate("# feat: x\n\n## Overview\nSomething.");
    let platform = Arc::new(MockPlatform::new(common::sample_diff(), make_pr_info()));
    let ctx = make_ctx(ai, Arc::clone(&platform));

    let output = SpecTool.run(&ctx).await.unwrap();

    assert!(
        output.contains("Technical Specification") || output.contains("description"),
        "Output should confirm spec generation:\n{output}"
    );
}

#[tokio::test]
async fn spec_with_draft_pr_succeeds() {
    let ai = MockAi::with_generate("# draft: WIP\n\n## Overview\nWork in progress.");
    let mut pr = make_pr_info();
    pr.is_draft = true;
    let platform = Arc::new(MockPlatform::new(common::sample_diff(), pr));
    let ctx = make_ctx(ai, Arc::clone(&platform));

    SpecTool.run(&ctx).await.expect("should handle draft PR");
}

#[tokio::test]
async fn spec_with_pr_labels_succeeds() {
    let ai = MockAi::with_generate("# chore: cleanup\n\n## Overview\nCleanup.");
    let mut pr = make_pr_info();
    pr.labels = vec!["maintenance".to_string(), "size/S".to_string()];
    let platform = Arc::new(MockPlatform::new(common::sample_diff(), pr));
    let ctx = make_ctx(ai, Arc::clone(&platform));

    SpecTool.run(&ctx).await.expect("should handle PR with labels");
}

#[tokio::test]
async fn spec_tool_name() {
    assert_eq!(SpecTool.name(), "spec");
}

#[tokio::test]
async fn spec_strips_h1_from_posted_body() {
    let ai = MockAi::with_generate(
        "# My PR Title\n\n## Overview\nDetails here.\n\n## Open Questions\nNone."
    );
    let platform = Arc::new(MockPlatform::new(common::sample_diff(), make_pr_info()));
    let ctx = make_ctx(ai, Arc::clone(&platform));

    SpecTool.run(&ctx).await.unwrap();

    let (_, body) = platform.last_description().unwrap();
    // The first line of the body must NOT be an H1
    let first_line = body.lines().next().unwrap_or("");
    assert!(
        !first_line.starts_with("# "),
        "First line of posted body must not be H1: '{first_line}'"
    );
}
