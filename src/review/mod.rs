pub mod engine;

pub use engine::ReviewEngine;

use async_trait::async_trait;
use std::sync::Arc;
use tracing::info;

use crate::error::Result;
use crate::tools::{MerlinTool, ToolContext};

/// MerlinTool adapter for the full /review flow.
pub struct ReviewTool;

#[async_trait]
impl MerlinTool for ReviewTool {
    fn name(&self) -> &'static str {
        "review"
    }

    async fn run(&self, ctx: &ToolContext) -> Result<String> {
        info!("Running /review via ReviewTool");
        let engine = ReviewEngine::new(
            Arc::clone(&ctx.ai),
            Arc::clone(&ctx.platform),
            crate::config::ReviewConfig::default(),
        );
        let comments = engine.run().await?;
        let summary = engine::build_summary_text(&comments);
        Ok(summary)
    }
}
