//! /add_doc — Generate and post docstring suggestions for new/modified functions.

use async_trait::async_trait;
use serde::Deserialize;
use tracing::info;

use super::{MerlinTool, ToolContext};
use crate::diff::{parse_diff, LineKind};
use crate::error::Result;
use crate::platform::InlineCodeSuggestion;

pub struct DocstringTool;

#[derive(Deserialize)]
struct DocSuggestion {
    file: String,
    line: u32,
    docstring: String,
    function_signature: String,
}

#[async_trait]
impl MerlinTool for DocstringTool {
    fn name(&self) -> &'static str {
        "add_doc"
    }

    async fn run(&self, ctx: &ToolContext) -> Result<String> {
        info!("Running /add_doc");

        let raw_diff = ctx.platform.get_diff().await?;
        let files = parse_diff(&raw_diff)?;

        let system = "You are a documentation expert. For each new or modified function/method \
                      in the diff that lacks a docstring, generate an appropriate doc comment \
                      (Rust /// style, Python \"\"\", JS JSDoc, etc., matching the language).\n\n\
                      Respond ONLY with a JSON array:\n\
                      [{\"file\":\"path\",\"line\":10,\
                      \"function_signature\":\"fn foo(x: u32) -> bool\",\
                      \"docstring\":\"/// doc comment here\"}]\n\
                      Return [] if all functions already have docs or no new functions exist.";

        let mut all_suggestions: Vec<InlineCodeSuggestion> = Vec::new();
        let mut posted = 0;

        for file in &files {
            // Only consider files with added lines
            let has_additions = file
                .hunks
                .iter()
                .any(|h| h.lines.iter().any(|l| l.kind == LineKind::Added));
            if !has_additions {
                continue;
            }

            let diff_text = crate::digest::compress_diff(file, 80);
            let user = format!(
                "Generate docstrings for new functions in `{}`:\n\n```diff\n{}\n```",
                file.path(),
                diff_text
            );

            let raw = ctx.ai.generate(system, &user).await.unwrap_or_default();
            let cleaned = raw
                .trim()
                .trim_start_matches("```json")
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim();

            if let Ok(docs) = serde_json::from_str::<Vec<DocSuggestion>>(cleaned) {
                for d in docs {
                    // Insert docstring above the function (line - 1)
                    let insert_line = d.line.saturating_sub(1).max(1);
                    all_suggestions.push(InlineCodeSuggestion {
                        file: d.file,
                        start_line: insert_line,
                        end_line: insert_line,
                        suggestion: format!("{}\n{}", d.docstring, d.function_signature),
                        description: format!("Add docstring for `{}`", d.function_signature),
                    });
                    posted += 1;
                    if posted >= 15 {
                        break;
                    }
                }
            }
            if posted >= 15 {
                break;
            }
        }

        if !all_suggestions.is_empty() {
            ctx.platform.post_code_suggestions(&all_suggestions).await?;
        }

        let count = all_suggestions.len();
        Ok(format!(
            "## Ferret: Documentation Suggestions\n\nGenerated **{count}** docstring suggestion(s). \
             Accept them from the diff view.\n\n\
             *Documented by [Merlin](https://github.com/you/ferret) 🦡*"
        ))
    }
}
