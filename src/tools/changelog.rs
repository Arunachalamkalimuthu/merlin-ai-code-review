//! /update_changelog — Prepend a changelog entry to CHANGELOG.md.

use async_trait::async_trait;
use tracing::info;

use super::{MerlinTool, ToolContext};
use crate::diff::parse_diff;
use crate::error::Result;

const CHANGELOG_PATH: &str = "CHANGELOG.md";

/// Tool for the `/changelog` slash command — generates a CHANGELOG entry from the diff.
pub struct ChangelogTool;

#[async_trait]
impl MerlinTool for ChangelogTool {
    fn name(&self) -> &'static str {
        "update_changelog"
    }

    async fn run(&self, ctx: &ToolContext) -> Result<String> {
        info!("Running /update_changelog");

        let raw_diff = ctx.platform.get_diff().await?;
        let pr_info = ctx.platform.get_pr_info().await?;
        let files = parse_diff(&raw_diff)?;

        let diff_summary = files
            .iter()
            .map(|f| format!("- `{}`", f.path()))
            .collect::<Vec<_>>()
            .join("\n");

        let system = "You are a technical writer. Generate a concise CHANGELOG entry in \
                      Keep-a-Changelog format (https://keepachangelog.com). \
                      Use the sections Added, Changed, Fixed, Removed as appropriate. \
                      Return ONLY the raw Markdown for the entry (no version header, \
                      just the bullet points under the appropriate sections).";

        let user = format!(
            "PR #{num}: \"{title}\" by @{author}\n\nFiles changed:\n{files}",
            num = pr_info.number,
            title = pr_info.title,
            author = pr_info.author,
            files = diff_summary,
        );

        let entry_body = ctx.ai.generate(system, &user).await?;

        // Fetch existing changelog
        let (existing_content, existing_sha) = ctx
            .platform
            .get_file(CHANGELOG_PATH)
            .await?
            .unwrap_or_else(|| {
                (
                    "# Changelog\n\nAll notable changes to this project will be documented here.\n"
                        .to_string(),
                    String::new(),
                )
            });

        // Prepend new entry under an Unreleased section
        let today = chrono_today();
        let new_entry = format!(
            "## [Unreleased] - {today}\n\n{entry}\n\n",
            today = today,
            entry = entry_body.trim(),
        );

        let updated = if let Some(pos) = existing_content.find("\n## ") {
            format!(
                "{}\n{}\n{}",
                &existing_content[..pos],
                new_entry,
                &existing_content[pos + 1..]
            )
        } else {
            format!("{}\n\n{}", existing_content.trim_end(), new_entry)
        };

        let sha_opt = if existing_sha.is_empty() {
            None
        } else {
            Some(existing_sha.as_str())
        };
        ctx.platform
            .update_file(
                CHANGELOG_PATH,
                &updated,
                &format!("docs: update changelog for PR #{}", pr_info.number),
                sha_opt,
            )
            .await?;

        Ok(format!(
            "## Merlin: Changelog Updated\n\nPrepended entry to `{CHANGELOG_PATH}`:\n\n{entry_body}\n\n\
             *Updated by [Merlin](https://github.com/you/merlin) 🦡*"
        ))
    }
}

fn chrono_today() -> String {
    // Simple date without chrono dependency — read from system
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Approximate: seconds since epoch → YYYY-MM-DD
    let days = secs / 86400;
    let year = 1970 + days / 365;
    let day_of_year = days % 365;
    let month = (day_of_year / 30) + 1;
    let day = (day_of_year % 30) + 1;
    format!(
        "{:04}-{:02}-{:02}",
        year.min(9999),
        month.min(12),
        day.min(31)
    )
}
