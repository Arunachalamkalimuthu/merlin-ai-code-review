//! /improve — Post inline code suggestion blocks.

use async_trait::async_trait;
use serde::Deserialize;
use tracing::info;

use super::{MerlinTool, ToolContext};
use crate::diff::parse_diff;
use crate::digest::prioritize_diffs;
use crate::error::Result;
use crate::platform::InlineCodeSuggestion;

pub struct ImproveTool;

#[derive(Deserialize)]
struct AiSuggestion {
    file: String,
    start_line: u32,
    end_line: u32,
    suggestion: String,
    description: String,
}

#[async_trait]
impl MerlinTool for ImproveTool {
    fn name(&self) -> &'static str {
        "improve"
    }

    async fn run(&self, ctx: &ToolContext) -> Result<String> {
        info!("Running /improve");

        let raw_diff = ctx.platform.get_diff().await?;
        let files = parse_diff(&raw_diff)?;
        let prioritized = prioritize_diffs(files, None);

        let system = "You are a senior code reviewer specializing in code quality improvements. \
                      Suggest concrete, actionable inline code changes.\n\n\
                      Respond ONLY with a JSON array:\n\
                      [{\"file\":\"path\",\"start_line\":1,\"end_line\":3,\
                      \"original\":\"old code\",\"suggestion\":\"improved code\",\
                      \"description\":\"Why this is better\"}]\n\
                      Return [] if no improvements are needed.";

        let mut all_suggestions: Vec<InlineCodeSuggestion> = Vec::new();
        let mut posted = 0;

        for pd in &prioritized {
            let diff_text = crate::digest::compress_diff(&pd.file, 100);
            let user = format!(
                "Suggest improvements for `{}`:\n\n```diff\n{}\n```",
                pd.file.path(),
                diff_text
            );

            let raw = ctx.ai.generate(system, &user).await.unwrap_or_default();
            let cleaned = raw
                .trim()
                .trim_start_matches("```json")
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim();

            if let Ok(suggestions) = serde_json::from_str::<Vec<AiSuggestion>>(cleaned) {
                for s in suggestions {
                    all_suggestions.push(InlineCodeSuggestion {
                        file: s.file,
                        start_line: s.start_line,
                        end_line: s.end_line,
                        suggestion: s.suggestion,
                        description: s.description,
                    });
                    posted += 1;
                    if posted >= 10 {
                        break;
                    }
                }
            }
            if posted >= 10 {
                break;
            }
        }

        if !all_suggestions.is_empty() {
            ctx.platform.post_code_suggestions(&all_suggestions).await?;
        }

        let count = all_suggestions.len();
        Ok(format!(
            "## Merlin: Code Improvements\n\nPosted **{count}** inline suggestion(s). \
             Review and apply them directly from the diff view.\n\n\
             *Reviewed by [Merlin](https://github.com/you/merlin) 🦡*"
        ))
    }
}
