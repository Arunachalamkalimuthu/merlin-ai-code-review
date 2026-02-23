//! Diff-line validation, comment deduplication, and Reflect & Review pass.
//!
//! These helpers are filtering / post-processing concerns separated from the
//! orchestration logic in [`super::engine`].

use std::collections::{HashMap, HashSet};

use tracing::{info, warn};

use crate::ai::{AiProvider, ReviewComment};
use crate::diff::FileDiff;
use crate::error::Result;

// ── Diff-line validation ───────────────────────────────────────────────────────

/// Build a map of file path → sorted list of new-file line numbers present in the diff.
///
/// GitHub's inline PR comment API only accepts lines that appear as added (`+`)
/// or context (` `) lines in a hunk — i.e. lines with a `new_line` position.
/// Posting on any other line number yields a 422 Unprocessable Entity error.
pub fn build_valid_diff_lines(file_diffs: &[FileDiff]) -> HashMap<String, Vec<u32>> {
    let mut map: HashMap<String, Vec<u32>> = HashMap::new();
    for file in file_diffs {
        let mut lines: Vec<u32> = file
            .hunks
            .iter()
            .flat_map(|h| h.lines.iter())
            .filter_map(|l| l.new_line)
            .collect();
        lines.sort_unstable();
        lines.dedup();
        if !lines.is_empty() {
            map.insert(file.path().to_string(), lines);
        }
    }
    map
}

/// Return the nearest valid diff line for `file`, or `None` if the file has no
/// commentable lines (e.g. it was deleted or not present in the diff).
pub fn nearest_valid_line(
    target: u32,
    file: &str,
    valid_lines: &HashMap<String, Vec<u32>>,
) -> Option<u32> {
    let lines = valid_lines.get(file)?;
    if lines.is_empty() {
        return None;
    }
    let pos = lines.partition_point(|&l| l <= target);
    let best = match pos {
        0 => lines[0],
        n if n >= lines.len() => lines[lines.len() - 1],
        n => {
            let before = lines[n - 1];
            let after = lines[n];
            if target.abs_diff(before) <= target.abs_diff(after) {
                before
            } else {
                after
            }
        }
    };
    Some(best)
}

// ── Deduplication ─────────────────────────────────────────────────────────────

/// Remove duplicate comments (same file + line + title).
pub fn deduplicate(comments: Vec<ReviewComment>) -> Vec<ReviewComment> {
    let mut seen: HashSet<String> = HashSet::new();
    comments
        .into_iter()
        .filter(|c| {
            let key = format!("{}:{}:{}", c.file, c.line, c.title);
            seen.insert(key)
        })
        .collect()
}

// ── Reflect & Review ──────────────────────────────────────────────────────────

/// Second AI pass: send first-pass comments back to the AI for critique and filtering.
///
/// The AI is asked to remove false positives, merge duplicates, and confirm severity.
/// If the refined JSON cannot be parsed, the original comments are returned unchanged.
pub async fn reflect_and_review(
    ai: &dyn AiProvider,
    comments: Vec<ReviewComment>,
) -> Result<Vec<ReviewComment>> {
    let comments_json =
        serde_json::to_string_pretty(&comments).unwrap_or_else(|_| "[]".to_string());

    let system = "You are a senior code reviewer performing a quality check on a set of \
                  AI-generated code review comments. Your job is to:\n\
                  1. Remove false positives and nitpicks that aren't actionable\n\
                  2. Merge duplicate or overlapping comments into one\n\
                  3. Correct severity levels that seem too high or too low\n\
                  4. Ensure each comment has a clear, concise body\n\n\
                  Respond ONLY with the filtered/improved JSON array of review comments \
                  (same schema as input). Preserve all valid comments. \
                  Return [] only if all comments are false positives.";

    let user = format!(
        "Please review and refine these {} code review comment(s):\n\n{}",
        comments.len(),
        comments_json
    );

    let raw = ai.generate(system, &user).await?;
    let cleaned = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    match serde_json::from_str::<Vec<ReviewComment>>(cleaned) {
        Ok(refined) => {
            info!(
                "Reflect & Review: {} → {} comments after refinement",
                comments.len(),
                refined.len()
            );
            Ok(refined)
        }
        Err(e) => {
            warn!("Reflect & Review parse failed ({e}), using original comments");
            Ok(comments)
        }
    }
}
