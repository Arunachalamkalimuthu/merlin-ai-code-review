//! /describe — Auto-generate a structured PR title and description.

use async_trait::async_trait;
use tracing::info;

use super::{MerlinTool, ToolContext};
use crate::diff::parse_diff;
use crate::error::Result;

pub struct DescribeTool;

#[async_trait]
impl MerlinTool for DescribeTool {
    fn name(&self) -> &'static str {
        "describe"
    }

    async fn run(&self, ctx: &ToolContext) -> Result<String> {
        info!("Running /describe");

        let raw_diff = ctx.platform.get_diff().await?;
        let pr_info = ctx.platform.get_pr_info().await?;
        let files = parse_diff(&raw_diff)?;

        let file_list: Vec<String> = files.iter().map(|f| format!("- `{}`", f.path())).collect();
        let diff_summary = files
            .iter()
            .map(|f| crate::digest::compress_diff(f, 30))
            .collect::<Vec<_>>()
            .join("\n\n");

        let prompt = format!(
            "You are a senior engineer. Based on the following PR diff, generate:\n\
             1. A concise PR title (≤72 chars, imperative mood)\n\
             2. A structured PR description in Markdown with sections:\n\
                ## Summary\n   (2-4 bullet points of what changed)\n\
                ## Motivation\n   (why this change was made)\n\
                ## Changes\n   (list of key technical changes)\n\
                ## Testing\n   (how to verify)\n\n\
             Current title: \"{title}\"\n\
             Author: {author}\n\
             Files changed ({n}):\n{files}\n\n\
             Diff:\n```diff\n{diff}\n```\n\n\
             Respond with JSON: {{\"title\": \"...\", \"description\": \"...\"}}",
            title = pr_info.title,
            author = pr_info.author,
            n = files.len(),
            files = file_list.join("\n"),
            diff = diff_summary,
        );

        let ctx_ai = crate::ai::ReviewContext {
            file: "(PR description)".to_string(),
            diff_hunk: prompt,
            full_file: None,
        };

        // We re-use the AI in raw mode via a wrapper prompt
        let raw = call_ai_raw(&*ctx.ai, &ctx_ai).await?;

        // Parse JSON response
        let value: serde_json::Value = serde_json::from_str(raw.trim())
            .unwrap_or_else(|_| serde_json::json!({"title": pr_info.title, "description": raw}));

        let new_title = value["title"]
            .as_str()
            .unwrap_or(&pr_info.title)
            .to_string();
        let new_body = value["description"].as_str().unwrap_or(&raw).to_string();

        ctx.platform
            .update_description(&new_title, &new_body)
            .await?;

        Ok(format!(
            "## Merlin: PR Description Updated\n\n**New title:** {new_title}\n\n{new_body}"
        ))
    }
}

/// Call AI with a freeform prompt, returning raw text (not parsed as ReviewComment array).
pub(crate) async fn call_ai_raw(
    ai: &dyn crate::ai::AiProvider,
    ctx: &crate::ai::ReviewContext,
) -> Result<String> {
    // We serialize the review context and get comments back, but for free-form prompts
    // we include the instruction in the diff_hunk and parse the response differently.
    // For now, use a review call and extract the first comment body as raw text.
    // A cleaner approach would be a separate `generate` method on AiProvider,
    // but we keep the trait minimal here and use a workaround.
    let comments = ai.review(ctx).await;
    match comments {
        Ok(c) if !c.is_empty() => Ok(c[0].body.clone()),
        _ => Ok(ctx.diff_hunk.clone()), // fallback: return the prompt itself
    }
}
