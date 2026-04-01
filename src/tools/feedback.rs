//! /feedback — Show adaptive feedback learning status and stats.
//!
//! Displays how many comment patterns are tracked, how many are suppressed,
//! and the top suppressed/accepted patterns.

use async_trait::async_trait;
use tracing::info;

use super::{MerlinTool, ToolContext};
use crate::error::Result;
use crate::feedback::FeedbackStore;

/// Tool for the `/feedback` slash command — reports feedback learning status.
pub struct FeedbackTool;

#[async_trait]
impl MerlinTool for FeedbackTool {
    fn name(&self) -> &'static str {
        "feedback"
    }

    async fn run(&self, ctx: &ToolContext) -> Result<String> {
        info!("Running /feedback");

        let store = FeedbackStore::load(".merlin-feedback.jsonl");
        let stats = store.stats();

        if stats.is_empty() {
            return Ok(
                "## Merlin: Feedback Learning\n\n\
                 No feedback recorded yet. React to review comments with \
                 👍 (accept) or 👎 (reject) to start training.\n\n\
                 Once a pattern accumulates enough rejects, Merlin will \
                 auto-suppress similar comments in future reviews."
                    .to_string(),
            );
        }

        let mut out = String::from("## Merlin: Feedback Learning Status\n\n");
        out.push_str(&format!(
            "**{}** patterns tracked · **{}** currently suppressed\n\n",
            store.pattern_count(),
            store.suppressed_count()
        ));

        // Suppressed patterns
        let mut suppressed: Vec<_> = stats
            .iter()
            .filter(|(_, s)| s.is_suppressed())
            .collect();
        suppressed.sort_by(|a, b| {
            b.1.reject_ratio()
                .partial_cmp(&a.1.reject_ratio())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        if !suppressed.is_empty() {
            out.push_str("### Suppressed Patterns\n\n");
            out.push_str("| Pattern | Accepted | Rejected | Reject % |\n");
            out.push_str("|---------|----------|----------|----------|\n");
            for (pattern, s) in &suppressed {
                out.push_str(&format!(
                    "| `{}` | {} | {} | {:.0}% |\n",
                    pattern,
                    s.accepted,
                    s.rejected,
                    s.reject_ratio() * 100.0
                ));
            }
            out.push('\n');
        }

        // Top accepted patterns
        let mut accepted: Vec<_> = stats
            .iter()
            .filter(|(_, s)| !s.is_suppressed() && s.total() >= 3)
            .collect();
        accepted.sort_by(|a, b| b.1.accepted.cmp(&a.1.accepted));
        accepted.truncate(10);

        if !accepted.is_empty() {
            out.push_str("### Top Accepted Patterns\n\n");
            out.push_str("| Pattern | Accepted | Rejected | Accept % |\n");
            out.push_str("|---------|----------|----------|----------|\n");
            for (pattern, s) in &accepted {
                let accept_ratio = if s.total() > 0 {
                    f64::from(s.accepted) / f64::from(s.total()) * 100.0
                } else {
                    0.0
                };
                out.push_str(&format!(
                    "| `{}` | {} | {} | {:.0}% |\n",
                    pattern, s.accepted, s.rejected, accept_ratio
                ));
            }
        }

        out.push_str(
            "\n---\n*React to review comments with 👍/👎 to improve future reviews.*\n",
        );

        // Post as PR comment if we have a platform
        let _ = ctx.platform.post_summary(&out).await;

        Ok(out)
    }
}
