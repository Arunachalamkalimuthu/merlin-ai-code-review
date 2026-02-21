//! /approve — AI-assisted PR approval with a final summary judgment.

use async_trait::async_trait;
use tracing::info;

use super::{MerlinTool, ToolContext};
use crate::diff::parse_diff;
use crate::error::Result;

pub struct ApproveTool;

#[async_trait]
impl MerlinTool for ApproveTool {
    fn name(&self) -> &'static str {
        "approve"
    }

    async fn run(&self, ctx: &ToolContext) -> Result<String> {
        info!("Running /approve");

        let raw_diff = ctx.platform.get_diff().await?;
        let pr_info = ctx.platform.get_pr_info().await?;
        let files = parse_diff(&raw_diff)?;

        let diff_text = files
            .iter()
            .map(|f| crate::digest::compress_diff(f, 60))
            .collect::<Vec<_>>()
            .join("\n\n");

        let system = "You are a senior code reviewer making a final approval decision. \
                      After reviewing the diff:\n\
                      1. State APPROVE, REQUEST_CHANGES, or COMMENT (no changes needed but no formal approval)\n\
                      2. Give a confidence score 0-100\n\
                      3. List blockers (if any) that prevent approval\n\
                      4. List concerns (non-blocking issues)\n\
                      5. Provide 2-3 positive observations\n\n\
                      Respond with JSON:\n\
                      {\"verdict\":\"APPROVE\",\"confidence\":85,\
                      \"blockers\":[],\"concerns\":[\"string\"],\
                      \"positives\":[\"string\"],\"summary\":\"Overall assessment\"}";

        let user = format!(
            "PR #{num}: \"{title}\" by @{author}\n\nDiff:\n{diff}",
            num = pr_info.number,
            title = pr_info.title,
            author = pr_info.author,
            diff = diff_text,
        );

        let raw = ctx.ai.generate(system, &user).await?;
        let cleaned = raw
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let value: serde_json::Value = serde_json::from_str(cleaned).unwrap_or_else(|_| {
            serde_json::json!({"verdict": "COMMENT", "confidence": 50, "summary": raw})
        });

        let verdict = value["verdict"].as_str().unwrap_or("COMMENT");
        let confidence = value["confidence"].as_u64().unwrap_or(50);
        let summary = value["summary"].as_str().unwrap_or("");

        let verdict_emoji = match verdict {
            "APPROVE" => "✅",
            "REQUEST_CHANGES" => "🚫",
            _ => "💬",
        };

        let mut out = format!(
            "## Ferret: Approval Assessment {verdict_emoji}\n\n\
             **Verdict:** {verdict} (confidence: {confidence}%)\n\n\
             **Summary:** {summary}\n\n"
        );

        if let Some(blockers) = value["blockers"].as_array() {
            if !blockers.is_empty() {
                out.push_str("### 🚫 Blockers (must fix before merge)\n");
                for b in blockers {
                    out.push_str(&format!("- {}\n", b.as_str().unwrap_or("")));
                }
                out.push('\n');
            }
        }

        if let Some(concerns) = value["concerns"].as_array() {
            if !concerns.is_empty() {
                out.push_str("### ⚠️ Concerns (non-blocking)\n");
                for c in concerns {
                    out.push_str(&format!("- {}\n", c.as_str().unwrap_or("")));
                }
                out.push('\n');
            }
        }

        if let Some(positives) = value["positives"].as_array() {
            if !positives.is_empty() {
                out.push_str("### ✅ Positives\n");
                for p in positives {
                    out.push_str(&format!("- {}\n", p.as_str().unwrap_or("")));
                }
                out.push('\n');
            }
        }

        out.push_str("*Assessed by [Merlin](https://github.com/you/ferret) 🦡*");
        Ok(out)
    }
}
