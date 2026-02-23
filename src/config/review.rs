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

impl Default for ReviewConfig {
    fn default() -> Self {
        ReviewConfig {
            focus: default_focus(),
            max_comments: default_max_comments(),
            chunk_lines: default_chunk_lines(),
            reflect: false,
            persona: PersonaConfig::default(),
        }
    }
}
