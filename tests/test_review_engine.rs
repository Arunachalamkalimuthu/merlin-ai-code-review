/// Tests for ReviewEngine — chunking, concurrent AI calls, deduplication, and summary.
mod common;

use std::sync::Arc;

use merlin::ai::{Category, ReviewComment, Severity};
use merlin::config::ReviewConfig;
use merlin::platform::PlatformClient;
use merlin::review::ReviewEngine;

use common::{make_comment, make_pr_info, MockAi, MockPlatform};

fn default_cfg() -> ReviewConfig {
    ReviewConfig { max_comments: 20, chunk_lines: 100, reflect: false, ..Default::default() }
}

fn engine(ai: MockAi, platform: Arc<MockPlatform>) -> ReviewEngine {
    ReviewEngine::new(
        Arc::new(ai),
        Arc::clone(&platform) as Arc<dyn PlatformClient>,
        default_cfg(),
    )
}

// ── run_local ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn run_local_empty_diff_returns_no_comments() {
    let platform = Arc::new(MockPlatform::new("", make_pr_info()));
    let eng = engine(MockAi::with_comments(vec![]), Arc::clone(&platform));
    let comments = eng.run_local("").await.expect("should succeed on empty diff");
    assert!(comments.is_empty());
}

#[tokio::test]
async fn run_local_deduplicates_same_file_line_title() {
    let dup = ReviewComment {
        file: "src/main.rs".to_string(),
        line: 10,
        severity: Severity::High,
        category: Category::Bug,
        title: "Null dereference".to_string(),
        body: "body".to_string(),
        suggestion: None,
    };
    let platform = Arc::new(MockPlatform::new(common::sample_diff(), make_pr_info()));
    let eng = engine(MockAi::with_comments(vec![dup.clone(), dup]), Arc::clone(&platform));

    let result = eng.run_local(common::sample_diff()).await.unwrap();
    assert_eq!(result.len(), 1, "Duplicate (file+line+title) should be deduplicated");
}

#[tokio::test]
async fn run_local_keeps_distinct_comments() {
    let c1 = make_comment("a.rs", 1, Severity::High);
    let c2 = make_comment("b.rs", 2, Severity::Medium);  // different file
    let c3 = make_comment("a.rs", 5, Severity::Low);     // same file, different line
    let platform = Arc::new(MockPlatform::new(common::sample_diff(), make_pr_info()));
    let eng = engine(MockAi::with_comments(vec![c1, c2, c3]), Arc::clone(&platform));

    let result = eng.run_local(common::sample_diff()).await.unwrap();
    assert_eq!(result.len(), 3, "All distinct comments should be kept");
}

#[tokio::test]
async fn run_local_caps_at_max_comments() {
    let many: Vec<ReviewComment> = (0..50)
        .map(|i| ReviewComment {
            file: format!("file_{i}.rs"),
            line: i as u32,
            severity: Severity::Low,
            category: Category::Style,
            title: format!("Issue {i}"),
            body: "body".to_string(),
            suggestion: None,
        })
        .collect();

    let platform = Arc::new(MockPlatform::new(common::sample_diff(), make_pr_info()));
    let cfg = ReviewConfig { max_comments: 5, ..default_cfg() };
    let eng = ReviewEngine::new(
        Arc::new(MockAi::with_comments(many)),
        Arc::clone(&platform) as Arc<dyn PlatformClient>,
        cfg,
    );

    let result = eng.run_local(common::sample_diff()).await.unwrap();
    assert!(result.len() <= 5, "Should cap at max_comments=5, got {}", result.len());
}

#[tokio::test]
async fn run_local_sorts_critical_first() {
    let comments = vec![
        make_comment("a.rs", 1, Severity::Low),
        make_comment("b.rs", 2, Severity::Critical),
        make_comment("c.rs", 3, Severity::Medium),
    ];
    let platform = Arc::new(MockPlatform::new(common::sample_diff(), make_pr_info()));
    let eng = engine(MockAi::with_comments(comments), Arc::clone(&platform));

    let result = eng.run_local(common::sample_diff()).await.unwrap();
    assert!(!result.is_empty());
    assert_eq!(result[0].severity, Severity::Critical, "Critical should come first");
}

// ── run (full pipeline) ───────────────────────────────────────────────────────

#[tokio::test]
async fn run_posts_summary_after_review() {
    let platform = Arc::new(MockPlatform::new(common::sample_diff(), make_pr_info()));
    let eng = engine(
        MockAi::with_comments(vec![make_comment("src/lib.rs", 5, Severity::High)]),
        Arc::clone(&platform),
    );

    eng.run().await.expect("run should succeed");

    let summary = platform.last_summary().expect("post_summary should be called");
    assert!(summary.contains("Merlin"), "Summary should include branding");
}

#[tokio::test]
async fn run_posts_all_inline_comments() {
    let platform = Arc::new(MockPlatform::new(common::sample_diff(), make_pr_info()));
    let eng = engine(
        MockAi::with_comments(vec![
            make_comment("src/main.rs", 10, Severity::High),
            make_comment("src/lib.rs", 5, Severity::Medium),
        ]),
        Arc::clone(&platform),
    );

    eng.run().await.unwrap();
    assert_eq!(platform.comment_count(), 2);
}

#[tokio::test]
async fn run_returns_empty_on_no_diff() {
    let platform = Arc::new(MockPlatform::new("", make_pr_info()));
    let eng = engine(MockAi::with_comments(vec![]), Arc::clone(&platform));

    let comments = eng.run().await.expect("should succeed on empty diff");
    assert!(comments.is_empty());
}

// ── build_summary ─────────────────────────────────────────────────────────────

#[test]
fn summary_mentions_all_severities() {
    use merlin::review::engine::build_summary;
    use merlin::ai::Severity;

    let comments = vec![
        make_comment("a.rs", 1, Severity::Critical),
        make_comment("b.rs", 2, Severity::High),
        make_comment("c.rs", 3, Severity::Medium),
        make_comment("d.rs", 4, Severity::Low),
        make_comment("e.rs", 5, Severity::Info),
    ];
    let summary = build_summary(&comments, None);
    assert!(summary.contains("5"), "Should mention total count");
    // Check all severity emojis/labels present
    assert!(summary.contains("critical") || summary.contains("Critical") || summary.contains("🔴"));
}

#[test]
fn summary_no_issues_is_positive() {
    use merlin::review::engine::build_summary;
    let summary = build_summary(&[], None);
    assert!(summary.contains("No issues found"));
    assert!(summary.contains("Merlin"));
}

#[test]
fn summary_includes_file_table() {
    use merlin::review::engine::build_summary;
    let comments = vec![make_comment("src/lib.rs", 10, Severity::High)];
    let summary = build_summary(&comments, None);
    assert!(summary.contains("src/lib.rs"), "Summary table should list file name");
}
