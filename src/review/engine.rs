use std::collections::HashSet;
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::{info, warn};

use crate::ai::{AiProvider, ReviewComment, ReviewContext, Severity};
use crate::config::ReviewConfig;
use crate::diff::{parse_diff, FileDiff};
use crate::digest::complexity_score;
use crate::error::Result;
use crate::platform::PlatformClient;
use crate::rag::RagPipeline;

pub struct ReviewEngine {
    pub ai: Arc<dyn AiProvider>,
    pub platform: Arc<dyn PlatformClient>,
    pub config: ReviewConfig,
    /// Optional RAG pipeline — when present, relevant codebase context is prepended
    /// to each AI review chunk.
    pub rag: Option<Arc<RagPipeline>>,
}

impl ReviewEngine {
    pub fn new(
        ai: Arc<dyn AiProvider>,
        platform: Arc<dyn PlatformClient>,
        config: ReviewConfig,
    ) -> Self {
        Self { ai, platform, config, rag: None }
    }

    pub fn with_rag(mut self, rag: Arc<RagPipeline>) -> Self {
        self.rag = Some(rag);
        self
    }

    /// Run the full review cycle: fetch diff → parse → AI review → post comments.
    pub async fn run(&self) -> Result<Vec<ReviewComment>> {
        // 1. Fetch raw diff from platform
        info!("Fetching diff from platform...");
        let raw_diff = self.platform.get_diff().await?;

        // 2. Parse diff
        let file_diffs = parse_diff(&raw_diff)?;
        info!("Parsed {} changed files", file_diffs.len());

        if file_diffs.is_empty() {
            info!("No changed files found — nothing to review.");
            return Ok(vec![]);
        }

        // 3. Compute PR complexity
        let complexity = complexity_score(&file_diffs);
        info!(
            "PR complexity score: {:.0}/100 ({})",
            complexity.score,
            complexity.risk_level.as_str()
        );

        // 4. Build review contexts (chunk large files)
        let contexts = self.build_contexts(&file_diffs);
        info!("Generated {} review chunks", contexts.len());

        // 5. Fan-out concurrent AI calls
        let mut comments = self.run_ai_reviews(contexts).await?;

        // 6. Optional: Reflect & Review second pass
        if self.config.reflect && !comments.is_empty() {
            info!("Running Reflect & Review second pass...");
            comments = self.reflect_and_review(comments).await?;
        }

        // 7. Deduplicate, sort, cap
        let mut comments = deduplicate(comments);
        comments.sort_by(|a, b| a.severity.cmp(&b.severity));
        if comments.len() > self.config.max_comments {
            warn!(
                "Capping {} comments to {}",
                comments.len(),
                self.config.max_comments
            );
            comments.truncate(self.config.max_comments);
        }

        // 8. Post inline comments
        for comment in &comments {
            if let Err(e) = self.platform.post_inline_comment(comment).await {
                warn!("Failed to post inline comment on {}: {e}", comment.file);
            }
        }

        // 9. Post summary (including complexity)
        let summary = build_summary(&comments, Some(&complexity));
        self.platform.post_summary(&summary).await?;

        info!("Review complete — {} comments posted", comments.len());
        Ok(comments)
    }

    /// Run review from a local diff string (for `--diff <file>` local mode).
    pub async fn run_local(&self, raw_diff: &str) -> Result<Vec<ReviewComment>> {
        let file_diffs = parse_diff(raw_diff)?;
        let contexts = self.build_contexts(&file_diffs);
        let mut comments = self.run_ai_reviews(contexts).await?;

        if self.config.reflect && !comments.is_empty() {
            comments = self.reflect_and_review(comments).await?;
        }

        let mut comments = deduplicate(comments);
        comments.sort_by(|a, b| a.severity.cmp(&b.severity));
        comments.truncate(self.config.max_comments);
        Ok(comments)
    }

    /// Second AI pass: send first-pass comments back to AI for critique and filtering.
    ///
    /// The AI is asked to remove false positives, merge duplicates, and confirm severity.
    async fn reflect_and_review(
        &self,
        comments: Vec<ReviewComment>,
    ) -> Result<Vec<ReviewComment>> {
        let comments_json = serde_json::to_string_pretty(&comments)
            .unwrap_or_else(|_| "[]".to_string());

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

        let raw = self.ai.generate(system, &user).await?;
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

    /// Split file diffs into chunks of at most `chunk_lines` lines.
    fn build_contexts(&self, files: &[FileDiff]) -> Vec<ReviewContext> {
        let mut contexts = Vec::new();
        for file in files {
            let diff_text = file.diff_text();
            let lines: Vec<&str> = diff_text.lines().collect();

            if lines.len() <= self.config.chunk_lines {
                contexts.push(ReviewContext {
                    file: file.path().to_string(),
                    diff_hunk: diff_text,
                    full_file: None,
                });
            } else {
                // Chunk at hunk boundaries when possible
                let mut chunk_start = 0;
                while chunk_start < lines.len() {
                    let chunk_end = (chunk_start + self.config.chunk_lines).min(lines.len());
                    let chunk = lines[chunk_start..chunk_end].join("\n");
                    contexts.push(ReviewContext {
                        file: file.path().to_string(),
                        diff_hunk: chunk,
                        full_file: None,
                    });
                    chunk_start = chunk_end;
                }
            }
        }
        contexts
    }

    /// Fan out AI review calls concurrently via Tokio JoinSet.
    async fn run_ai_reviews(&self, contexts: Vec<ReviewContext>) -> Result<Vec<ReviewComment>> {
        let mut join_set: JoinSet<Result<Vec<ReviewComment>>> = JoinSet::new();

        for mut ctx in contexts {
            // Enrich diff hunk with RAG context when pipeline is configured
            if let Some(rag) = &self.rag {
                match crate::rag::retriever::retrieve_context(rag, &ctx.diff_hunk).await {
                    Ok(Some(rag_ctx)) => {
                        ctx.diff_hunk = format!("{rag_ctx}---\n\n## Diff to review\n\n{}", ctx.diff_hunk);
                    }
                    Ok(None) => {}
                    Err(e) => warn!("RAG retrieve failed (continuing without context): {e}"),
                }
            }
            let ai = Arc::clone(&self.ai);
            join_set.spawn(async move { ai.review(&ctx).await });
        }

        let mut all_comments = Vec::new();
        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(Ok(comments)) => all_comments.extend(comments),
                Ok(Err(e)) => warn!("AI review chunk failed: {e}"),
                Err(e) => warn!("Task panicked: {e}"),
            }
        }

        Ok(all_comments)
    }
}

/// Remove duplicate comments (same file + line + title).
fn deduplicate(comments: Vec<ReviewComment>) -> Vec<ReviewComment> {
    let mut seen: HashSet<String> = HashSet::new();
    comments
        .into_iter()
        .filter(|c| {
            let key = format!("{}:{}:{}", c.file, c.line, c.title);
            seen.insert(key)
        })
        .collect()
}

/// Public alias so ReviewTool can call it.
pub fn build_summary_text(comments: &[ReviewComment]) -> String {
    build_summary(comments, None)
}

/// Build a Markdown summary from all comments, optionally including complexity.
pub fn build_summary(
    comments: &[ReviewComment],
    complexity: Option<&crate::digest::ComplexityScore>,
) -> String {
    let mut out = String::from("## Merlin Code Review\n\n");

    // Complexity line
    if let Some(cx) = complexity {
        out.push_str(&format!("{}\n\n", cx.summary_line()));
    }

    if comments.is_empty() {
        out.push_str("No issues found. Great work! ✅\n\n");
        out.push_str("---\n*Reviewed by [Merlin](https://github.com/you/merlin) 🦡*\n");
        return out;
    }

    let critical = count_severity(comments, &Severity::Critical);
    let high = count_severity(comments, &Severity::High);
    let medium = count_severity(comments, &Severity::Medium);
    let low = count_severity(comments, &Severity::Low);
    let info = count_severity(comments, &Severity::Info);

    out.push_str(&format!(
        "Found **{}** issue(s): 🔴 {} critical · 🟠 {} high · 🟡 {} medium · 🔵 {} low · ⚪ {} info\n\n",
        comments.len(), critical, high, medium, low, info
    ));

    out.push_str("### Issues\n\n");
    out.push_str("| Severity | File | Line | Title |\n");
    out.push_str("|----------|------|------|-------|\n");

    for c in comments {
        let emoji = match c.severity {
            Severity::Critical => "🔴",
            Severity::High => "🟠",
            Severity::Medium => "🟡",
            Severity::Low => "🔵",
            Severity::Info => "⚪",
        };
        out.push_str(&format!(
            "| {emoji} {:?} | `{}` | {} | {} |\n",
            c.severity, c.file, c.line, c.title
        ));
    }

    out.push_str("\n---\n*Reviewed by [Merlin](https://github.com/you/merlin) 🦡*\n");
    out
}

fn count_severity(comments: &[ReviewComment], sev: &Severity) -> usize {
    comments.iter().filter(|c| &c.severity == sev).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{Category, Severity};

    fn make_comment(file: &str, line: u32, sev: Severity) -> ReviewComment {
        ReviewComment {
            file: file.to_string(),
            line,
            severity: sev,
            category: Category::Bug,
            title: "Test".to_string(),
            body: "body".to_string(),
            suggestion: None,
        }
    }

    #[test]
    fn test_deduplicate() {
        let comments = vec![
            make_comment("a.rs", 1, Severity::High),
            make_comment("a.rs", 1, Severity::High), // duplicate
            make_comment("b.rs", 2, Severity::Low),
        ];
        let deduped = deduplicate(comments);
        assert_eq!(deduped.len(), 2);
    }

    #[test]
    fn test_build_summary_empty() {
        let summary = build_summary(&[], None);
        assert!(summary.contains("No issues found"));
    }

    #[test]
    fn test_build_summary_with_comments() {
        let comments = vec![
            make_comment("src/main.rs", 10, Severity::Critical),
            make_comment("src/lib.rs", 5, Severity::Low),
        ];
        let summary = build_summary(&comments, None);
        assert!(summary.contains("2"));
        assert!(summary.contains("src/main.rs"));
        assert!(summary.contains("Merlin"));
    }

    #[test]
    fn test_build_summary_with_complexity() {
        use crate::digest::{ComplexityScore, RiskLevel};
        let cx = ComplexityScore {
            total_files: 3,
            total_additions: 100,
            total_deletions: 20,
            estimated_cyclomatic: 5,
            score: 42.0,
            risk_level: RiskLevel::Medium,
        };
        let summary = build_summary(&[], Some(&cx));
        assert!(summary.contains("Complexity"));
        assert!(summary.contains("Medium"));
    }

    #[test]
    fn test_chunk_lines() {
        let diff = include_str!("../../tests/fixtures/large.diff");
        // Basic sanity: parse doesn't panic on larger diffs
        let _ = parse_diff(diff);
    }
}
