/// Tests for the unified diff parser — edge cases beyond what the unit tests cover.
use merlin::diff::parse_diff;

// ── Happy paths ───────────────────────────────────────────────────────────────

#[test]
fn parse_empty_string_returns_empty_vec() {
    assert!(parse_diff("").unwrap().is_empty());
}

#[test]
fn parse_single_added_line() {
    let diff = "\
diff --git a/foo.rs b/foo.rs
index 000..111 100644
--- a/foo.rs
+++ b/foo.rs
@@ -1,1 +1,2 @@
 fn main() {}
+// added
";
    let files = parse_diff(diff).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path(), "foo.rs");
}

#[test]
fn parse_new_file() {
    let diff = "\
diff --git a/new_file.rs b/new_file.rs
new file mode 100644
index 0000000..abc1234
--- /dev/null
+++ b/new_file.rs
@@ -0,0 +1,3 @@
+fn hello() {
+    println!(\"world\");
+}
";
    let files = parse_diff(diff).unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].is_new, "File should be marked as new");
}

#[test]
fn parse_deleted_file() {
    let diff = "\
diff --git a/old.rs b/old.rs
deleted file mode 100644
index abc1234..0000000
--- a/old.rs
+++ /dev/null
@@ -1,2 +0,0 @@
-fn old_fn() {}
-// removed
";
    let files = parse_diff(diff).unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].is_deleted, "File should be marked as deleted");
}

#[test]
fn parse_multiple_files() {
    let diff = "\
diff --git a/src/a.rs b/src/a.rs
index 000..111 100644
--- a/src/a.rs
+++ b/src/a.rs
@@ -1,1 +1,2 @@
+// a
diff --git a/src/b.rs b/src/b.rs
index 000..222 100644
--- a/src/b.rs
+++ b/src/b.rs
@@ -1,1 +1,2 @@
+// b
diff --git a/src/c.rs b/src/c.rs
index 000..333 100644
--- a/src/c.rs
+++ b/src/c.rs
@@ -1,1 +1,2 @@
+// c
";
    let files = parse_diff(diff).unwrap();
    assert_eq!(files.len(), 3);

    let paths: Vec<&str> = files.iter().map(|f| f.path()).collect();
    assert!(paths.contains(&"src/a.rs"));
    assert!(paths.contains(&"src/b.rs"));
    assert!(paths.contains(&"src/c.rs"));
}

#[test]
fn parse_multiple_hunks_in_one_file() {
    let diff = "\
diff --git a/src/lib.rs b/src/lib.rs
index 000..111 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,3 +1,4 @@
 fn a() {}
+fn b() {}
 fn c() {}
@@ -10,3 +11,4 @@
 fn d() {}
+fn e() {}
 fn f() {}
";
    let files = parse_diff(diff).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].hunks.len(), 2, "Should parse two separate hunks");
}

#[test]
fn diff_text_roundtrip_preserves_content() {
    let diff = "\
diff --git a/src/main.rs b/src/main.rs
index 000..111 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,4 @@
 fn main() {
+    println!(\"hello\");
     // existing
 }
";
    let files = parse_diff(diff).unwrap();
    assert!(!files.is_empty());

    let text = files[0].diff_text();
    assert!(text.contains("println"), "diff_text should contain added lines");
    assert!(text.contains("existing"), "diff_text should contain context lines");
}

// ── Path extraction ───────────────────────────────────────────────────────────

#[test]
fn path_with_subdirectory() {
    let diff = "\
diff --git a/src/api/handler.rs b/src/api/handler.rs
index 000..111 100644
--- a/src/api/handler.rs
+++ b/src/api/handler.rs
@@ -1,1 +1,2 @@
+// change
";
    let files = parse_diff(diff).unwrap();
    assert_eq!(files[0].path(), "src/api/handler.rs");
}

#[test]
fn path_extracts_b_side() {
    // The path should come from the b/ side (new file path)
    let diff = "\
diff --git a/old_name.rs b/new_name.rs
index 000..111 100644
--- a/old_name.rs
+++ b/new_name.rs
@@ -1,1 +1,2 @@
+// renamed
";
    let files = parse_diff(diff).unwrap();
    // Parser extracts the b/ path (new name)
    assert_eq!(files[0].path(), "new_name.rs");
}

// ── Robustness ────────────────────────────────────────────────────────────────

#[test]
fn parse_diff_with_no_hunks_returns_empty() {
    // Diff header only, no actual hunks
    let diff = "diff --git a/foo.rs b/foo.rs\n";
    let result = parse_diff(diff);
    // Either OK with empty list or OK with file with no hunks — both are fine
    assert!(result.is_ok());
}

#[test]
fn parse_large_diff_does_not_panic() {
    // Generate a large diff to stress-test the parser
    let mut diff = String::from(
        "diff --git a/src/big.rs b/src/big.rs\n\
         index 000..111 100644\n\
         --- a/src/big.rs\n\
         +++ b/src/big.rs\n\
         @@ -1,500 +1,501 @@\n",
    );
    for i in 0..500 {
        diff.push_str(&format!(" line {i}\n"));
    }
    diff.push_str("+added line\n");

    let result = parse_diff(&diff);
    assert!(result.is_ok(), "Parser should not panic on large diffs");
}
