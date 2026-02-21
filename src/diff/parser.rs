//! Unified diff parser.
//!
//! Parses unified diff output (as produced by `git diff`) into structured
//! [`FileDiff`] / [`Hunk`] / [`HunkLine`] types that the rest of Merlin can
//! work with.
//!
//! # Supported formats
//!
//! - Standard unified diff (`--- a/…`, `+++ b/…`, `@@ … @@`)
//! - Git diffs with `a/` / `b/` path prefixes (stripped automatically)
//! - New-file diffs (`--- /dev/null`)
//! - Deleted-file diffs (`+++ /dev/null`)
//! - Multi-file diffs
//!
//! # Limitations
//!
//! - Binary file markers (`Binary files … differ`) are silently skipped.
//! - `\ No newline at end of file` lines are treated as context.
//! - Rename detection (output of `git diff --find-renames`) is not implemented.
//!
//! # Example
//!
//! ```rust
//! use merlin::diff::parse_diff;
//!
//! let diff = "--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1 +1 @@\n-old\n+new\n";
//! let files = parse_diff(diff).unwrap();
//! assert_eq!(files[0].path(), "src/main.rs");
//! ```
use crate::error::{MerlinError, Result};

/// A single line within a hunk, tagged with its kind and source line numbers.
#[derive(Debug, Clone, PartialEq)]
pub struct HunkLine {
    /// Whether this line was added, removed, or unchanged.
    pub kind: LineKind,
    /// Raw line content without the leading `+`/`-`/` ` prefix character.
    pub content: String,
    /// Line number in the *new* file — `None` for removed lines.
    pub new_line: Option<u32>,
    /// Line number in the *old* file — `None` for added lines.
    pub old_line: Option<u32>,
}

/// Classification of a single diff line.
#[derive(Debug, Clone, PartialEq)]
pub enum LineKind {
    /// Unchanged line present in both old and new file (` ` prefix).
    Context,
    /// Line added in the new file (`+` prefix).
    Added,
    /// Line removed from the old file (`-` prefix).
    Removed,
}

/// A single diff hunk.
#[derive(Debug, Clone)]
pub struct Hunk {
    pub old_start: u32,
    pub old_count: u32,
    pub new_start: u32,
    pub new_count: u32,
    pub header_suffix: String, // optional function context after @@
    pub lines: Vec<HunkLine>,
}

/// Represents all hunks for one file in the diff.
#[derive(Debug, Clone)]
pub struct FileDiff {
    pub old_path: String,
    pub new_path: String,
    pub hunks: Vec<Hunk>,
    /// True when the file is newly created.
    pub is_new: bool,
    /// True when the file is deleted.
    pub is_deleted: bool,
}

impl FileDiff {
    /// Convenience: return the effective path (new_path for renames/new files).
    pub fn path(&self) -> &str {
        &self.new_path
    }

    /// Flatten all hunk lines into a single string suitable for AI review.
    pub fn diff_text(&self) -> String {
        let mut out = format!("--- {}\n+++ {}\n", self.old_path, self.new_path);
        for h in &self.hunks {
            out.push_str(&format!(
                "@@ -{},{} +{},{} @@ {}\n",
                h.old_start, h.old_count, h.new_start, h.new_count, h.header_suffix
            ));
            for line in &h.lines {
                let prefix = match line.kind {
                    LineKind::Context => ' ',
                    LineKind::Added => '+',
                    LineKind::Removed => '-',
                };
                out.push(prefix);
                out.push_str(&line.content);
                out.push('\n');
            }
        }
        out
    }
}

/// Parse a unified diff string into a list of [`FileDiff`] structs.
///
/// # Errors
///
/// Returns [`crate::error::MerlinError::DiffParse`] if a hunk header (`@@ … @@`)
/// is malformed or contains unparseable range values.
pub fn parse_diff(input: &str) -> Result<Vec<FileDiff>> {
    let mut files: Vec<FileDiff> = Vec::new();
    let mut current: Option<FileDiff> = None;
    let mut current_hunk: Option<Hunk> = None;
    let mut old_line = 0u32;
    let mut new_line = 0u32;

    for raw_line in input.lines() {
        // --- a/path
        if let Some(rest) = raw_line.strip_prefix("--- ") {
            // Flush previous file
            if let Some(mut f) = current.take() {
                if let Some(h) = current_hunk.take() {
                    f.hunks.push(h);
                }
                files.push(f);
            }
            let old_path = strip_git_prefix(rest);
            current = Some(FileDiff {
                old_path: old_path.to_string(),
                new_path: String::new(),
                hunks: Vec::new(),
                is_new: old_path == "/dev/null",
                is_deleted: false,
            });
            current_hunk = None;
            continue;
        }

        // +++ b/path
        if let Some(rest) = raw_line.strip_prefix("+++ ") {
            if let Some(ref mut f) = current {
                let new_path = strip_git_prefix(rest);
                f.new_path = new_path.to_string();
                f.is_deleted = new_path == "/dev/null";
            }
            continue;
        }

        // @@ -old_start[,old_count] +new_start[,new_count] @@ [suffix]
        if raw_line.starts_with("@@") {
            if let Some(ref mut f) = current {
                if let Some(h) = current_hunk.take() {
                    f.hunks.push(h);
                }
            }
            let (hunk, ol, nl) = parse_hunk_header(raw_line)?;
            old_line = ol;
            new_line = nl;
            current_hunk = Some(hunk);
            continue;
        }

        // Hunk body lines
        if let Some(ref mut hunk) = current_hunk {
            if let Some(rest) = raw_line.strip_prefix('+') {
                hunk.lines.push(HunkLine {
                    kind: LineKind::Added,
                    content: rest.to_string(),
                    new_line: Some(new_line),
                    old_line: None,
                });
                new_line += 1;
            } else if let Some(rest) = raw_line.strip_prefix('-') {
                hunk.lines.push(HunkLine {
                    kind: LineKind::Removed,
                    content: rest.to_string(),
                    new_line: None,
                    old_line: Some(old_line),
                });
                old_line += 1;
            } else {
                // Context line (starts with ' ' or is empty)
                let content = raw_line.strip_prefix(' ').unwrap_or(raw_line);
                hunk.lines.push(HunkLine {
                    kind: LineKind::Context,
                    content: content.to_string(),
                    new_line: Some(new_line),
                    old_line: Some(old_line),
                });
                old_line += 1;
                new_line += 1;
            }
            continue;
        }

        // Skip git header lines (diff --git, index, new file mode, etc.)
    }

    // Flush last file
    if let Some(mut f) = current.take() {
        if let Some(h) = current_hunk.take() {
            f.hunks.push(h);
        }
        files.push(f);
    }

    Ok(files)
}

fn strip_git_prefix(s: &str) -> &str {
    s.strip_prefix("a/")
        .or_else(|| s.strip_prefix("b/"))
        .unwrap_or(s)
}

/// Parse `@@ -old_start[,old_count] +new_start[,new_count] @@ [suffix]`
fn parse_hunk_header(line: &str) -> Result<(Hunk, u32, u32)> {
    // Drop leading "@@" and find closing "@@"
    let inner = line
        .strip_prefix("@@")
        .ok_or_else(|| MerlinError::DiffParse(format!("bad hunk header: {line}")))?;

    let end = inner
        .find("@@")
        .ok_or_else(|| MerlinError::DiffParse(format!("unclosed @@ in: {line}")))?;

    let coords = inner[..end].trim();
    let suffix = inner[end + 2..].trim().to_string();

    let mut parts = coords.split_whitespace();
    let old_part = parts
        .next()
        .ok_or_else(|| MerlinError::DiffParse("missing old range".to_string()))?;
    let new_part = parts
        .next()
        .ok_or_else(|| MerlinError::DiffParse("missing new range".to_string()))?;

    let (old_start, old_count) = parse_range(old_part.trim_start_matches('-'))?;
    let (new_start, new_count) = parse_range(new_part.trim_start_matches('+'))?;

    Ok((
        Hunk {
            old_start,
            old_count,
            new_start,
            new_count,
            header_suffix: suffix,
            lines: Vec::new(),
        },
        old_start,
        new_start,
    ))
}

fn parse_range(s: &str) -> Result<(u32, u32)> {
    if let Some((start, count)) = s.split_once(',') {
        let start = start
            .parse()
            .map_err(|_| MerlinError::DiffParse(format!("bad range start: {s}")))?;
        let count = count
            .parse()
            .map_err(|_| MerlinError::DiffParse(format!("bad range count: {s}")))?;
        Ok((start, count))
    } else {
        let start = s
            .parse()
            .map_err(|_| MerlinError::DiffParse(format!("bad range: {s}")))?;
        Ok((start, 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_DIFF: &str = r#"diff --git a/src/main.rs b/src/main.rs
index abc1234..def5678 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,7 +1,9 @@ fn main() {
 fn main() {
-    println!("hello");
+    println!("Hello, Merlin!");
+    let x = 42;
+    println!("{}", x);
 }
"#;

    #[test]
    fn test_parse_basic_diff() {
        let files = parse_diff(SAMPLE_DIFF).expect("parse failed");
        assert_eq!(files.len(), 1);
        let f = &files[0];
        assert_eq!(f.old_path, "src/main.rs");
        assert_eq!(f.new_path, "src/main.rs");
        assert_eq!(f.hunks.len(), 1);
        let h = &f.hunks[0];
        assert_eq!(h.old_start, 1);
        assert_eq!(h.new_start, 1);
    }

    #[test]
    fn test_hunk_lines() {
        let files = parse_diff(SAMPLE_DIFF).unwrap();
        let lines = &files[0].hunks[0].lines;
        assert!(lines.iter().any(|l| l.kind == LineKind::Removed));
        assert!(lines.iter().any(|l| l.kind == LineKind::Added));
        assert!(lines.iter().any(|l| l.kind == LineKind::Context));
    }

    #[test]
    fn test_new_file_diff() {
        let diff = "--- /dev/null\n+++ b/new_file.rs\n@@ -0,0 +1,2 @@\n+fn foo() {}\n+fn bar() {}\n";
        let files = parse_diff(diff).unwrap();
        assert!(files[0].is_new);
        assert_eq!(files[0].new_path, "new_file.rs");
    }

    #[test]
    fn test_deleted_file_diff() {
        let diff = "--- a/old.rs\n+++ /dev/null\n@@ -1,1 +0,0 @@\n-fn old() {}\n";
        let files = parse_diff(diff).unwrap();
        assert!(files[0].is_deleted);
    }

    #[test]
    fn test_diff_text_roundtrip() {
        let files = parse_diff(SAMPLE_DIFF).unwrap();
        let text = files[0].diff_text();
        assert!(text.contains("+++ src/main.rs"));
        assert!(text.contains("+    println!(\"Hello, Merlin!\");"));
    }

    #[test]
    fn test_multi_file_diff() {
        let diff = concat!(
            "--- a/a.rs\n+++ b/a.rs\n@@ -1,1 +1,1 @@\n-old\n+new\n",
            "--- a/b.rs\n+++ b/b.rs\n@@ -1,1 +1,1 @@\n-x\n+y\n"
        );
        let files = parse_diff(diff).unwrap();
        assert_eq!(files.len(), 2);
    }
}
