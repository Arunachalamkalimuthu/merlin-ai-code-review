//! PR digest: token budgeting, file prioritisation, size labelling, and complexity scoring.
//!
//! This module provides three complementary capabilities:
//!
//! - **Token budgeting** — [`prioritize_diffs`] ranks files by security sensitivity
//!   and drops the least important ones when the total token estimate would exceed
//!   `DEFAULT_TOKEN_BUDGET`, keeping AI costs predictable.
//! - **PR status** — [`build_pr_status`] combines raw [`crate::platform::PrInfo`]
//!   with parsed diffs to produce a [`PrStatus`] summary (size label, test coverage
//!   signal, secrets-risk flag).
//! - **Complexity scoring** — [`complexity_score`] returns a 0–100 composite
//!   [`ComplexityScore`] used by [`crate::review::ReviewEngine`] to annotate the
//!   PR summary comment.

use serde::Serialize;

use crate::diff::{FileDiff, LineKind};
use crate::platform::PrInfo;

/// Token budget constants (approximate, using 4 chars ≈ 1 token heuristic).
const CHARS_PER_TOKEN: usize = 4;
const DEFAULT_TOKEN_BUDGET: usize = 6_000; // conservative for 8k context models

/// Priority rank for a file — lower discriminant = reviewed first.
///
/// Assigned by [`classify_priority`] based on path heuristics.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FilePriority {
    /// Auth, secrets, crypto, JWT, OAuth, RBAC paths.
    Critical = 0,
    /// Core application code — API handlers, business logic.
    High = 1,
    /// Test files, mocks, fixtures, helpers.
    Medium = 2,
    /// Docs, lock files, generated code, binary assets.
    Low = 3,
}

/// A [`crate::diff::FileDiff`] annotated with its computed priority and token estimate.
///
/// Produced by [`prioritize_diffs`].
#[derive(Debug, Clone)]
pub struct PrioritizedDiff {
    /// The parsed file diff.
    pub file: FileDiff,
    /// Computed priority rank.
    pub priority: FilePriority,
    /// Approximate token cost for this diff.
    pub estimated_tokens: usize,
}

/// High-level status summary for a pull request, produced by [`build_pr_status`].
///
/// Used by slash-command tools (e.g. `/describe`, `/triage`) that need PR
/// metadata without calling the platform API again.
#[derive(Debug, Clone)]
pub struct PrStatus {
    /// PR/MR title.
    pub title: String,
    /// GitHub/GitLab login of the PR author.
    pub author: String,
    /// Whether the PR is in draft state.
    pub is_draft: bool,
    /// Total number of files changed.
    pub files_changed: u32,
    /// Total lines added.
    pub additions: u32,
    /// Total lines removed.
    pub deletions: u32,
    /// Labels currently applied to the PR.
    pub labels: Vec<String>,
    /// T-shirt size label based on lines changed.
    pub size_label: SizeLabel,
    /// Whether at least one test file was changed.
    pub has_tests: bool,
    /// Whether a database migration file was changed.
    pub has_migration: bool,
    /// Whether any security-sensitive file was changed.
    pub has_secrets_risk: bool,
    /// Contributing guidelines fetched from the repo, if any.
    pub contributing_guidelines: Option<String>,
}

/// T-shirt size label based on total lines changed (additions + deletions).
///
/// Convert from a line count with [`SizeLabel::from_lines`], then get the
/// GitHub label string with [`SizeLabel::as_str`].
#[derive(Debug, Clone, PartialEq)]
pub enum SizeLabel {
    /// ≤ 10 lines changed — `"size/XS"`.
    XSmall,
    /// ≤ 50 lines changed — `"size/S"`.
    Small,
    /// ≤ 250 lines changed — `"size/M"`.
    Medium,
    /// ≤ 1 000 lines changed — `"size/L"`.
    Large,
    /// > 1 000 lines changed — `"size/XL"`.
    XLarge,
}

impl SizeLabel {
    /// Convert a total line-change count to a size label.
    pub fn from_lines(lines: u32) -> Self {
        match lines {
            0..=10 => SizeLabel::XSmall,
            11..=50 => SizeLabel::Small,
            51..=250 => SizeLabel::Medium,
            251..=1000 => SizeLabel::Large,
            _ => SizeLabel::XLarge,
        }
    }

    /// Return the GitHub label string for this size (e.g. `"size/M"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            SizeLabel::XSmall => "size/XS",
            SizeLabel::Small => "size/S",
            SizeLabel::Medium => "size/M",
            SizeLabel::Large => "size/L",
            SizeLabel::XLarge => "size/XL",
        }
    }
}

/// Compute token budget estimate (rough: chars / 4).
pub fn estimate_tokens(text: &str) -> usize {
    text.len() / CHARS_PER_TOKEN + 1
}

/// Classify file priority based on path heuristics.
pub fn classify_priority(path: &str) -> FilePriority {
    let lower = path.to_lowercase();

    // Critical: security-sensitive files
    if lower.contains("auth")
        || lower.contains("secret")
        || lower.contains("crypto")
        || lower.contains("password")
        || lower.contains("token")
        || lower.contains("key")
        || lower.contains("oauth")
        || lower.contains("jwt")
        || lower.contains("permission")
        || lower.contains("rbac")
    {
        return FilePriority::Critical;
    }

    // Low: generated, lock, docs, assets
    if lower.ends_with(".lock")
        || lower.ends_with(".sum")
        || lower.contains("generated")
        || lower.contains("/vendor/")
        || lower.contains("/node_modules/")
        || lower.ends_with(".md")
        || lower.ends_with(".txt")
        || lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".svg")
    {
        return FilePriority::Low;
    }

    // Medium: tests
    if lower.contains("test")
        || lower.contains("spec")
        || lower.contains("mock")
        || lower.contains("fixture")
    {
        return FilePriority::Medium;
    }

    // High: everything else (core application code)
    FilePriority::High
}

/// Prioritize and token-budget a list of file diffs.
pub fn prioritize_diffs(files: Vec<FileDiff>, token_budget: Option<usize>) -> Vec<PrioritizedDiff> {
    let budget = token_budget.unwrap_or(DEFAULT_TOKEN_BUDGET);

    let mut prioritized: Vec<PrioritizedDiff> = files
        .into_iter()
        .map(|f| {
            let priority = classify_priority(f.path());
            let estimated_tokens = estimate_tokens(&f.diff_text());
            PrioritizedDiff {
                file: f,
                priority,
                estimated_tokens,
            }
        })
        .collect();

    // Sort: critical first, then by token cost (cheapest first within same priority)
    prioritized.sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .then(a.estimated_tokens.cmp(&b.estimated_tokens))
    });

    // Apply token budget — drop lowest-priority files if over budget
    let mut total = 0usize;
    prioritized.retain(|d| {
        if total + d.estimated_tokens <= budget {
            total += d.estimated_tokens;
            true
        } else {
            false
        }
    });

    prioritized
}

/// Build a PrStatus from raw PR info and parsed diffs.
pub fn build_pr_status(
    info: &PrInfo,
    files: &[FileDiff],
    contributing_guidelines: Option<String>,
) -> PrStatus {
    let total_changed = info.additions + info.deletions;
    let has_tests = files.iter().any(|f| {
        let lower = f.path().to_lowercase();
        lower.contains("test") || lower.contains("spec")
    });
    let has_migration = files.iter().any(|f| {
        let lower = f.path().to_lowercase();
        lower.contains("migration") || lower.contains("migrate") || lower.ends_with(".sql")
    });
    let has_secrets_risk = files
        .iter()
        .any(|f| classify_priority(f.path()) == FilePriority::Critical);

    PrStatus {
        title: info.title.clone(),
        author: info.author.clone(),
        is_draft: info.is_draft,
        files_changed: info.files_changed,
        additions: info.additions,
        deletions: info.deletions,
        labels: info.labels.clone(),
        size_label: SizeLabel::from_lines(total_changed),
        has_tests,
        has_migration,
        has_secrets_risk,
        contributing_guidelines,
    }
}

// ── PR Complexity Scoring ──────────────────────────────────────────────────────

/// Risk level derived from complexity score.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum RiskLevel {
    /// Score 0–24 — low complexity.
    Low,
    /// Score 25–49 — moderate complexity.
    Medium,
    /// Score 50–74 — high complexity.
    High,
    /// Score 75–100 — critical complexity.
    Critical,
}

impl RiskLevel {
    /// Return a coloured circle emoji representing this risk level.
    pub fn emoji(&self) -> &'static str {
        match self {
            RiskLevel::Low => "🟢",
            RiskLevel::Medium => "🟡",
            RiskLevel::High => "🟠",
            RiskLevel::Critical => "🔴",
        }
    }
    /// Return the human-readable risk level name.
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskLevel::Low => "Low",
            RiskLevel::Medium => "Medium",
            RiskLevel::High => "High",
            RiskLevel::Critical => "Critical",
        }
    }
}

/// Complexity metrics for a PR.
#[derive(Debug, Clone, Serialize)]
pub struct ComplexityScore {
    /// Number of files changed.
    pub total_files: usize,
    /// Lines added.
    pub total_additions: usize,
    /// Lines removed.
    pub total_deletions: usize,
    /// Estimated cyclomatic complexity increment (branch keywords in added lines).
    pub estimated_cyclomatic: u32,
    /// 0–100 composite score (higher = more complex).
    pub score: f32,
    /// Human-readable risk level.
    pub risk_level: RiskLevel,
}

impl ComplexityScore {
    /// One-line summary suitable for PR comments.
    pub fn summary_line(&self) -> String {
        format!(
            "{} **Complexity:** {:.0}/100 ({}) — {} file(s) changed, +{}/−{} lines, ~{} branch(es)",
            self.risk_level.emoji(),
            self.score,
            self.risk_level.as_str(),
            self.total_files,
            self.total_additions,
            self.total_deletions,
            self.estimated_cyclomatic,
        )
    }
}

/// Branch-introducing keywords in common languages.
static BRANCH_KEYWORDS: &[&str] = &[
    " if ",
    " else ",
    " elif ",
    " match ",
    " case ",
    " switch ",
    " for ",
    " while ",
    " loop ",
    " foreach ",
    " catch ",
    " except ",
    "?.",
    "||",
    "&&",
    " ? ",
];

/// Compute a composite complexity score for a set of file diffs.
pub fn complexity_score(files: &[FileDiff]) -> ComplexityScore {
    let total_files = files.len();
    let mut total_additions = 0usize;
    let mut total_deletions = 0usize;
    let mut estimated_cyclomatic = 0u32;

    for file in files {
        for hunk in &file.hunks {
            for line in &hunk.lines {
                match line.kind {
                    LineKind::Added => {
                        total_additions += 1;
                        for kw in BRANCH_KEYWORDS {
                            if line.content.contains(kw) {
                                estimated_cyclomatic += 1;
                            }
                        }
                    }
                    LineKind::Removed => {
                        total_deletions += 1;
                    }
                    LineKind::Context => {}
                }
            }
        }
    }

    // Composite score: weighted sum, clamped to 0–100
    let file_factor = (total_files as f32 * 2.0).min(20.0);
    let line_factor = ((total_additions + total_deletions) as f32 / 10.0).min(40.0);
    let branch_factor = (estimated_cyclomatic as f32 * 1.5).min(40.0);
    let score = (file_factor + line_factor + branch_factor).min(100.0);

    let risk_level = match score as u32 {
        0..=24 => RiskLevel::Low,
        25..=49 => RiskLevel::Medium,
        50..=74 => RiskLevel::High,
        _ => RiskLevel::Critical,
    };

    ComplexityScore {
        total_files,
        total_additions,
        total_deletions,
        estimated_cyclomatic,
        score,
        risk_level,
    }
}

/// Build a compact diff summary for injection into AI prompts alongside heavy context.
pub fn compress_diff(file: &FileDiff, max_added_lines: usize) -> String {
    let mut out = format!("--- {}\n+++ {}\n", file.old_path, file.new_path);
    let mut added_count = 0;
    let mut omitted = 0;

    for hunk in &file.hunks {
        out.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count
        ));
        for line in &hunk.lines {
            if line.kind == LineKind::Added {
                if added_count >= max_added_lines {
                    omitted += 1;
                    continue;
                }
                added_count += 1;
            }
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

    if omitted > 0 {
        out.push_str(&format!(
            "\n... [{omitted} lines omitted for token budget] ...\n"
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_priority() {
        assert_eq!(
            classify_priority("src/auth/handler.rs"),
            FilePriority::Critical
        );
        assert_eq!(classify_priority("Cargo.lock"), FilePriority::Low);
        assert_eq!(classify_priority("tests/unit.rs"), FilePriority::Medium);
        assert_eq!(classify_priority("src/api/routes.rs"), FilePriority::High);
    }

    #[test]
    fn test_size_label() {
        assert_eq!(SizeLabel::from_lines(5), SizeLabel::XSmall);
        assert_eq!(SizeLabel::from_lines(30), SizeLabel::Small);
        assert_eq!(SizeLabel::from_lines(100), SizeLabel::Medium);
        assert_eq!(SizeLabel::from_lines(500), SizeLabel::Large);
        assert_eq!(SizeLabel::from_lines(2000), SizeLabel::XLarge);
    }

    #[test]
    fn test_estimate_tokens() {
        let tokens = estimate_tokens("hello world");
        assert!(tokens >= 1);
    }

    #[test]
    fn test_complexity_score_empty() {
        let score = complexity_score(&[]);
        assert_eq!(score.total_files, 0);
        assert_eq!(score.score, 0.0);
        assert_eq!(score.risk_level, RiskLevel::Low);
    }

    #[test]
    fn test_risk_level_emoji() {
        assert_eq!(RiskLevel::Low.emoji(), "🟢");
        assert_eq!(RiskLevel::Critical.emoji(), "🔴");
    }
}
