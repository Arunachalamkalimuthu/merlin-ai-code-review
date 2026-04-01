//! Review behaviour settings.

use serde::{Deserialize, Serialize};

use super::ai::PersonaConfig;

/// Review behaviour settings — maps to the `[review]` table in `merlin.toml`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReviewConfig {
    /// Categories to check: any combination of `"bugs"`, `"security"`,
    /// `"style"`, `"performance"` (default: all four).
    #[serde(default = "default_focus")]
    pub focus: Vec<String>,
    /// Maximum inline comments per review (default: 30).
    #[serde(default = "default_max_comments")]
    pub max_comments: usize,
    /// Lines per diff chunk sent to the AI in a single request (default: 200).
    #[serde(default = "default_chunk_lines")]
    pub chunk_lines: usize,
    /// Enable the "Reflect & Review" second AI pass that critiques the first-pass comments.
    #[serde(default)]
    pub reflect: bool,
    /// Custom review persona (overrides system prompt behaviour).
    #[serde(default)]
    pub persona: PersonaConfig,
    /// Only re-review files whose diff has changed since the last run.
    #[serde(default)]
    pub incremental: bool,
    /// Path to the incremental review cache file (default: `.merlin-cache.json`).
    #[serde(default = "default_cache_path")]
    pub cache_path: String,
    /// Enable adaptive feedback learning — suppresses comment patterns that
    /// are consistently rejected by reviewers (default: `false`).
    #[serde(default)]
    pub feedback_learning: bool,
    /// Path to the feedback data file (default: `.merlin-feedback.jsonl`).
    #[serde(default = "default_feedback_path")]
    pub feedback_path: String,
    /// Path to the custom rules file (default: `.merlin-rules.yaml`).
    #[serde(default = "default_rules_file")]
    pub rules_file: String,
}

fn default_focus() -> Vec<String> {
    vec![
        "bugs".to_string(),
        "security".to_string(),
        "style".to_string(),
        "performance".to_string(),
    ]
}

fn default_max_comments() -> usize {
    30
}

fn default_chunk_lines() -> usize {
    200
}

fn default_cache_path() -> String {
    ".merlin-cache.json".to_string()
}

fn default_feedback_path() -> String {
    ".merlin-feedback.jsonl".to_string()
}

fn default_rules_file() -> String {
    ".merlin-rules.yaml".to_string()
}

impl Default for ReviewConfig {
    fn default() -> Self {
        ReviewConfig {
            focus: default_focus(),
            max_comments: default_max_comments(),
            chunk_lines: default_chunk_lines(),
            reflect: false,
            persona: PersonaConfig::default(),
            incremental: false,
            cache_path: default_cache_path(),
            feedback_learning: false,
            feedback_path: default_feedback_path(),
            rules_file: default_rules_file(),
        }
    }
}
