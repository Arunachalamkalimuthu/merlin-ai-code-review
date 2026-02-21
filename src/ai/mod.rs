pub mod anthropic;
pub mod azure_openai;
pub mod bedrock;
pub mod claude_code;
pub mod gemini;
pub mod ollama;
pub mod openai;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::config::{AiConfig, AiProviderType, Config};
use crate::error::Result;

/// A focused context sent to the AI for review.
#[derive(Debug, Clone)]
pub struct ReviewContext {
    pub file: String,
    pub diff_hunk: String,
    pub full_file: Option<String>,
}

/// Severity of a review comment.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Category of a review comment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Bug,
    Security,
    Style,
    Performance,
}

/// A single review comment produced by the AI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewComment {
    pub file: String,
    pub line: u32,
    pub severity: Severity,
    pub category: Category,
    pub title: String,
    pub body: String,
    pub suggestion: Option<String>,
}

/// Trait implemented by all AI backends.
#[async_trait]
pub trait AiProvider: Send + Sync {
    /// Structured code review: returns parsed `ReviewComment` array.
    async fn review(&self, ctx: &ReviewContext) -> Result<Vec<ReviewComment>>;

    /// Freeform generation: system + user prompts, returns raw text.
    async fn generate(&self, system: &str, user: &str) -> Result<String>;
}

/// Build the system prompt for AI code review.
pub fn system_prompt(focus: &[String]) -> String {
    system_prompt_with_persona(focus, None)
}

/// Build the system prompt, optionally including persona overrides and custom rules.
pub fn system_prompt_with_persona(
    focus: &[String],
    persona: Option<&crate::config::PersonaConfig>,
) -> String {
    let effective_focus = persona
        .and_then(|p| p.focus_override.as_ref())
        .map_or(focus, |v| v);
    let focus_list = effective_focus.join(", ");

    let persona_name = persona
        .and_then(|p| p.name.as_deref())
        .unwrap_or("senior code reviewer");

    let mut prompt = format!(
        "You are a {persona_name}. Analyze the provided diff and identify issues in these \
         categories: {focus_list}.\n\n\
         Respond ONLY with a JSON array of objects (no markdown fences, no extra text):\n\
         [{{\n\
         \"file\": \"path/to/file.rs\",\n\
         \"line\": 42,\n\
         \"severity\": \"critical|high|medium|low|info\",\n\
         \"category\": \"bug|security|style|performance\",\n\
         \"title\": \"Short title\",\n\
         \"body\": \"Detailed explanation\",\n\
         \"suggestion\": \"Optional code fix or null\"\n\
         }}]\n\n\
         If there are no issues, respond with an empty array: []"
    );

    // Append custom rules
    if let Some(p) = persona {
        if let Some(rules) = &p.rules {
            if !rules.is_empty() {
                prompt.push_str("\n\n## Review Rules\n\nApply these additional rules:\n");
                for (i, rule) in rules.iter().enumerate() {
                    prompt.push_str(&format!("{}. {}\n", i + 1, rule));
                }
            }
        }
        // Append extra system prompt instructions
        if let Some(extra) = &p.system_prompt_extra {
            if !extra.is_empty() {
                prompt.push_str(&format!("\n\n{extra}"));
            }
        }
    }

    prompt
}

/// Factory: create the appropriate AI provider from config.
pub fn build_provider(cfg: &AiConfig) -> Result<Box<dyn AiProvider>> {
    match cfg.provider {
        AiProviderType::Anthropic => {
            let key = Config::anthropic_api_key()?;
            Ok(Box::new(anthropic::AnthropicProvider::new(key, cfg.clone())))
        }
        AiProviderType::Openai => {
            let key = Config::openai_api_key()?;
            Ok(Box::new(openai::OpenAiProvider::new(key, cfg.clone())))
        }
        AiProviderType::ClaudeCode => {
            // Optionally authenticate with a token in CI
            if let Some(ref token) = cfg.claude_code_token {
                if !token.is_empty() {
                    tracing::info!("Authenticating claude CLI with provided token");
                    let _ = std::process::Command::new("claude")
                        .args(["auth", "login", "--token", token])
                        .output();
                }
            }
            if !claude_code::ClaudeCodeProvider::is_available() {
                return Err(crate::error::MerlinError::Config(
                    "`claude` CLI not found. Install Claude Code: https://claude.ai/claude-code".to_string(),
                ));
            }
            Ok(Box::new(claude_code::ClaudeCodeProvider::new(cfg.clone())))
        }
        AiProviderType::Gemini => {
            let key = Config::gemini_api_key()?;
            Ok(Box::new(gemini::GeminiProvider::new(key, cfg.clone())))
        }
        AiProviderType::Ollama => {
            let base_url = cfg
                .ollama_base_url
                .clone()
                .unwrap_or_else(|| "http://localhost:11434".to_string());
            Ok(Box::new(ollama::OllamaProvider::new(cfg.clone(), base_url)))
        }
        AiProviderType::AzureOpenai => {
            let key = Config::azure_openai_api_key()?;
            Ok(Box::new(azure_openai::AzureOpenAiProvider::new(key, cfg.clone())))
        }
        AiProviderType::Bedrock => {
            let (access, secret, token) = Config::aws_credentials()?;
            Ok(Box::new(bedrock::BedrockProvider::new(access, secret, token, cfg.clone())))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_prompt_contains_focus() {
        let prompt = system_prompt(&["bugs".to_string(), "security".to_string()]);
        assert!(prompt.contains("bugs, security"));
        assert!(prompt.contains("JSON array"));
    }

    #[test]
    fn test_system_prompt_with_persona() {
        use crate::config::PersonaConfig;
        let persona = PersonaConfig {
            name: Some("security expert".to_string()),
            system_prompt_extra: Some("Be extra strict.".to_string()),
            focus_override: Some(vec!["security".to_string()]),
            rules: Some(vec!["Never approve SQL queries without parameterization.".to_string()]),
        };
        let prompt = system_prompt_with_persona(&["bugs".to_string()], Some(&persona));
        assert!(prompt.contains("security expert"));
        assert!(prompt.contains("security")); // focus override
        assert!(!prompt.contains("bugs")); // overridden
        assert!(prompt.contains("Review Rules"));
        assert!(prompt.contains("SQL queries"));
        assert!(prompt.contains("Be extra strict"));
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical < Severity::High);
        assert!(Severity::High < Severity::Medium);
        assert!(Severity::Medium < Severity::Low);
        assert!(Severity::Low < Severity::Info);
    }
}
