//! /generate_labels — Auto-label PRs based on diff content.

use async_trait::async_trait;
use serde::Deserialize;
use tracing::info;

use super::{MerlinTool, ToolContext};
use crate::diff::parse_diff;
use crate::digest::build_pr_status;
use crate::error::Result;

/// Tool for the `/labels` slash command — suggests GitHub/GitLab labels for the PR.
pub struct LabelsTool;

#[derive(Deserialize)]
struct AiLabels {
    labels: Vec<String>,
}

#[async_trait]
impl MerlinTool for LabelsTool {
    fn name(&self) -> &'static str {
        "generate_labels"
    }

    async fn run(&self, ctx: &ToolContext) -> Result<String> {
        info!("Running /generate_labels");

        let raw_diff = ctx.platform.get_diff().await?;
        let pr_info = ctx.platform.get_pr_info().await?;
        let files = parse_diff(&raw_diff)?;
        let status = build_pr_status(&pr_info, &files, None);

        // Always include size label
        let mut labels: Vec<String> = vec![status.size_label.as_str().to_string()];

        let file_paths: Vec<String> = files.iter().map(|f| f.path().to_string()).collect();

        let system = "You are a PR triaging assistant. Based on the PR diff summary, \
                      suggest appropriate labels from this set: \
                      [bug, feature, refactor, docs, test, security, performance, \
                      breaking-change, dependencies, ci/cd, ui, api, database].\n\n\
                      Respond ONLY with JSON: {\"labels\": [\"label1\", \"label2\"]}";

        let user = format!(
            "PR: \"{title}\"\nFiles ({n}): {files}\n\
             Has tests: {tests}\nHas migration: {migration}\nSecurity risk: {sec}",
            title = pr_info.title,
            n = files.len(),
            files = file_paths.join(", "),
            tests = status.has_tests,
            migration = status.has_migration,
            sec = status.has_secrets_risk,
        );

        let raw = ctx.ai.generate(system, &user).await.unwrap_or_default();
        let cleaned = raw
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        if let Ok(ai_labels) = serde_json::from_str::<AiLabels>(cleaned) {
            labels.extend(ai_labels.labels);
        }

        // Dedup
        labels.dedup();
        labels.sort();

        ctx.platform.set_labels(&labels).await?;

        let label_list: Vec<String> = labels.iter().map(|l| format!("`{l}`")).collect();
        Ok(format!(
            "## Merlin: Labels Applied\n\nApplied labels: {}\n\n\
             *Labeled by [Merlin](https://github.com/you/merlin) 🦡*",
            label_list.join(", ")
        ))
    }
}
