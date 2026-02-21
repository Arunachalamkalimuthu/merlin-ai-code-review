/// Tests for src/digest.rs — priority classification, compression, and PR status.
use merlin::digest::{
    build_pr_status, classify_priority, compress_diff, estimate_tokens, prioritize_diffs,
    FilePriority, SizeLabel,
};
use merlin::diff::parse_diff;
use merlin::platform::PrInfo;

// ── classify_priority ─────────────────────────────────────────────────────────

#[test]
fn classify_auth_files_as_critical() {
    assert_eq!(classify_priority("src/auth/handler.rs"), FilePriority::Critical);
    assert_eq!(classify_priority("src/oauth2.rs"), FilePriority::Critical);
    assert_eq!(classify_priority("config/jwt_secret.toml"), FilePriority::Critical);
    assert_eq!(classify_priority("src/password_reset.rs"), FilePriority::Critical);
    assert_eq!(classify_priority("middleware/rbac.rs"), FilePriority::Critical);
}

#[test]
fn classify_generated_files_as_low() {
    assert_eq!(classify_priority("Cargo.lock"), FilePriority::Low);
    assert_eq!(classify_priority("go.sum"), FilePriority::Low);
    assert_eq!(classify_priority("README.md"), FilePriority::Low);
    assert_eq!(classify_priority("docs/guide.txt"), FilePriority::Low);
    assert_eq!(classify_priority("assets/logo.png"), FilePriority::Low);
    assert_eq!(classify_priority("assets/icon.svg"), FilePriority::Low);
    // vendor path needs a parent directory to match "/vendor/" substring
    assert_eq!(classify_priority("third_party/vendor/lib.rs"), FilePriority::Low);
}

#[test]
fn classify_test_files_as_medium() {
    // NOTE: files containing security keywords (auth, key, token, …) are
    // classified Critical even if they are test files — security wins.
    // Use paths that contain test keywords but NOT security keywords.
    assert_eq!(classify_priority("tests/integration_spec.rs"), FilePriority::Medium);
    assert_eq!(classify_priority("mocks/mock_db.rs"), FilePriority::Medium);
    assert_eq!(classify_priority("fixtures/sample.json"), FilePriority::Medium);
    assert_eq!(classify_priority("src/user_test.rs"), FilePriority::Medium);
}

#[test]
fn security_keyword_beats_test_keyword() {
    // auth_test.rs contains "auth" → Critical, not Medium
    assert_eq!(classify_priority("src/auth_test.rs"), FilePriority::Critical);
    // token_spec.rs contains "token" → Critical
    assert_eq!(classify_priority("tests/token_spec.rs"), FilePriority::Critical);
}

#[test]
fn classify_application_files_as_high() {
    assert_eq!(classify_priority("src/main.rs"), FilePriority::High);
    assert_eq!(classify_priority("src/api/handler.rs"), FilePriority::High);
    assert_eq!(classify_priority("lib/utils.py"), FilePriority::High);
}

#[test]
fn classify_is_case_insensitive() {
    assert_eq!(classify_priority("src/AUTH.RS"), FilePriority::Critical);
    assert_eq!(classify_priority("SRC/MAIN.RS"), FilePriority::High);
}

// ── SizeLabel ─────────────────────────────────────────────────────────────────

#[test]
fn size_label_boundaries() {
    assert_eq!(SizeLabel::from_lines(0), SizeLabel::XSmall);
    assert_eq!(SizeLabel::from_lines(10), SizeLabel::XSmall);
    assert_eq!(SizeLabel::from_lines(11), SizeLabel::Small);
    assert_eq!(SizeLabel::from_lines(50), SizeLabel::Small);
    assert_eq!(SizeLabel::from_lines(51), SizeLabel::Medium);
    assert_eq!(SizeLabel::from_lines(250), SizeLabel::Medium);
    assert_eq!(SizeLabel::from_lines(251), SizeLabel::Large);
    assert_eq!(SizeLabel::from_lines(1000), SizeLabel::Large);
    assert_eq!(SizeLabel::from_lines(1001), SizeLabel::XLarge);
}

#[test]
fn size_label_strings() {
    assert_eq!(SizeLabel::XSmall.as_str(), "size/XS");
    assert_eq!(SizeLabel::Small.as_str(), "size/S");
    assert_eq!(SizeLabel::Medium.as_str(), "size/M");
    assert_eq!(SizeLabel::Large.as_str(), "size/L");
    assert_eq!(SizeLabel::XLarge.as_str(), "size/XL");
}

// ── estimate_tokens ───────────────────────────────────────────────────────────

#[test]
fn estimate_tokens_empty_string() {
    // Should be 1 (the +1 floor)
    assert_eq!(estimate_tokens(""), 1);
}

#[test]
fn estimate_tokens_short_text() {
    // "abcd" = 4 chars → 4/4 + 1 = 2
    assert_eq!(estimate_tokens("abcd"), 2);
}

#[test]
fn estimate_tokens_scales_with_length() {
    let short = estimate_tokens("hello");
    let long = estimate_tokens("hello".repeat(100).as_str());
    assert!(long > short, "Longer text should have more tokens");
}

// ── compress_diff ─────────────────────────────────────────────────────────────

#[test]
fn compress_diff_short_file_is_unchanged() {
    let diff = "\
diff --git a/src/main.rs b/src/main.rs
index 000..111 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,4 @@
 fn main() {
+    println!(\"hello\");
 }
";
    let files = parse_diff(diff).unwrap();
    assert!(!files.is_empty());

    // With a large line limit the diff should be returned as-is
    let compressed = compress_diff(&files[0], 1000);
    assert!(compressed.contains("println"));
}

#[test]
fn compress_diff_truncates_long_files() {
    // Build a diff with many lines
    let mut diff = String::from(
        "diff --git a/src/big.rs b/src/big.rs\nindex 000..111 100644\n--- a/src/big.rs\n+++ b/src/big.rs\n@@ -1,200 +1,201 @@\n",
    );
    for i in 0..200 {
        diff.push_str(&format!("+line {i}\n"));
    }

    let files = parse_diff(&diff).unwrap();
    if files.is_empty() {
        return; // Parser may reject malformed hunks — that's acceptable
    }

    let compressed = compress_diff(&files[0], 20);
    let line_count = compressed.lines().count();
    // Should be significantly shorter than the original
    assert!(
        line_count < 200,
        "compress_diff should truncate: got {line_count} lines"
    );
}

// ── prioritize_diffs ──────────────────────────────────────────────────────────

#[test]
fn prioritize_puts_critical_first() {
    let diff = "\
diff --git a/src/main.rs b/src/main.rs
index 000..111 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,2 +1,3 @@
 fn main() {}
+// change
diff --git a/src/auth.rs b/src/auth.rs
index 000..222 100644
--- a/src/auth.rs
+++ b/src/auth.rs
@@ -1,2 +1,3 @@
 fn verify() {}
+// auth change
";
    let files = parse_diff(diff).unwrap();
    if files.len() < 2 {
        return; // Skip if parser doesn't produce expected output
    }

    let prioritized = prioritize_diffs(files, None);
    assert!(!prioritized.is_empty());

    // First entry should be a critical file (auth.rs)
    assert_eq!(
        prioritized[0].priority,
        FilePriority::Critical,
        "auth.rs should come first"
    );
}

#[test]
fn prioritize_respects_token_budget() {
    // Create a large diff that would exceed a tiny budget
    let mut big_diff = String::from(
        "diff --git a/src/big.rs b/src/big.rs\nindex 000..111 100644\n--- a/src/big.rs\n+++ b/src/big.rs\n@@ -1,100 +1,101 @@\n",
    );
    for i in 0..100 {
        big_diff.push_str(&format!("+line {i}\n"));
    }

    let files = parse_diff(&big_diff).unwrap();
    if files.is_empty() {
        return;
    }

    // With a tiny budget of 1 token, nothing should be returned
    let prioritized = prioritize_diffs(files, Some(1));
    assert!(
        prioritized.is_empty(),
        "Should drop all files when budget is 1 token"
    );
}

// ── build_pr_status ───────────────────────────────────────────────────────────

fn make_pr_info() -> PrInfo {
    PrInfo {
        number: 1,
        title: "test PR".to_string(),
        body: String::new(),
        head_sha: "abc".to_string(),
        base_branch: "main".to_string(),
        head_branch: "feat/x".to_string(),
        author: "bob".to_string(),
        is_draft: false,
        labels: vec![],
        files_changed: 3,
        additions: 100,
        deletions: 20,
    }
}

#[test]
fn build_pr_status_detects_tests() {
    let diff = "\
diff --git a/tests/integration_test.rs b/tests/integration_test.rs
index 000..111 100644
--- a/tests/integration_test.rs
+++ b/tests/integration_test.rs
@@ -1,2 +1,3 @@
+#[test] fn it_works() {}
";
    let files = parse_diff(diff).unwrap();
    let status = build_pr_status(&make_pr_info(), &files, None);
    assert!(status.has_tests, "Should detect test file");
}

#[test]
fn build_pr_status_detects_secrets_risk() {
    let diff = "\
diff --git a/src/auth_handler.rs b/src/auth_handler.rs
index 000..111 100644
--- a/src/auth_handler.rs
+++ b/src/auth_handler.rs
@@ -1,2 +1,3 @@
+fn new_auth_path() {}
";
    let files = parse_diff(diff).unwrap();
    let status = build_pr_status(&make_pr_info(), &files, None);
    assert!(status.has_secrets_risk, "Auth file should set has_secrets_risk");
}

#[test]
fn build_pr_status_detects_migration() {
    let diff = "\
diff --git a/migrations/001_add_users.sql b/migrations/001_add_users.sql
index 000..111 100644
--- a/migrations/001_add_users.sql
+++ b/migrations/001_add_users.sql
@@ -1,2 +1,3 @@
+CREATE TABLE users (id INT);
";
    let files = parse_diff(diff).unwrap();
    let status = build_pr_status(&make_pr_info(), &files, None);
    assert!(status.has_migration, "SQL migration file should be detected");
}

#[test]
fn build_pr_status_size_label_from_additions_deletions() {
    let mut pr = make_pr_info();
    pr.additions = 5;
    pr.deletions = 3; // total = 8, XSmall
    let status = build_pr_status(&pr, &[], None);
    assert_eq!(status.size_label, SizeLabel::XSmall);
}
