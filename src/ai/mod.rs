//! AI provider abstractions and backend implementations.
//!
//! # Overview
//!
//! The central abstraction is the [`AiProvider`] trait.  Each backend
//! implements two methods:
//!
//! - [`AiProvider::review`] — structured code review returning
//!   [`Vec<ReviewComment>`]
//! - [`AiProvider::generate`] — freeform text generation (used by slash
//!   commands such as `/describe`, `/spec`, etc.)
//!
//! Use [`build_provider`] to instantiate the correct backend from a
//! [`crate::config::AiConfig`].
//!
//! # Supported backends
//!
//! | Backend | Module | `provider` value |
//! |---------|--------|-----------------|
//! | Anthropic Claude | [`anthropic`] | `"anthropic"` |
//! | OpenAI GPT | [`openai`] | `"openai"` |
//! | Claude Code CLI | [`claude_code`] | `"claude-code"` |
//! | Google Gemini | [`gemini`] | `"gemini"` |
//! | AWS Bedrock | [`bedrock`] | `"bedrock"` |
//! | Azure OpenAI | [`azure_openai`] | `"azure-openai"` |
//! | Local Ollama | [`ollama`] | `"ollama"` |
//!
//! # Shared response parsing
//!
//! All providers delegate JSON parsing to [`response::parse_review_response`],
//! which handles bare arrays, markdown-fenced arrays, and wrapped objects.

pub mod anthropic;
pub mod azure_openai;
pub mod bedrock;
pub mod claude_code;
pub mod gemini;
pub mod ollama;
pub mod openai;
pub mod response;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::config::{AiConfig, AiProviderType, Config};
use crate::error::Result;

// ── Core data types ───────────────────────────────────────────────────────────

/// A focused slice of a diff sent to the AI for review.
///
/// Large files are split into multiple [`ReviewContext`] chunks at
/// `chunk_lines` boundaries (see [`crate::config::ReviewConfig::chunk_lines`])
/// before being submitted concurrently to the AI.
#[derive(Debug, Clone)]
pub struct ReviewContext {
    /// Path of the file being reviewed (relative to the repo root).
    pub file: String,
    /// The diff hunk(s) for this chunk, optionally prefixed with RAG context.
    pub diff_hunk: String,
    /// Full file content, when available — gives the AI more context.
    pub full_file: Option<String>,
}

/// Severity of a single review comment.
///
/// Variants are ordered from most to least severe so that `Critical < High`
/// (i.e. `Severity::Critical.cmp(&Severity::High) == Ordering::Less`), which
/// lets comments be sorted with the most important ones first.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// A showstopper: data loss, security breach, crash, or incorrect behaviour.
    Critical,
    /// Significant issue that should be fixed before merging.
    High,
    /// Worth addressing but not a blocker.
    Medium,
    /// Minor quality improvement.
    Low,
    /// Informational note with no action required.
    Info,
}

/// Category of a review comment, used to filter reviews via
/// [`crate::config::ReviewConfig::focus`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    /// Logic error, null dereference, off-by-one, incorrect algorithm.
    Bug,
    /// Vulnerability, exposed secret, injection risk, missing auth.
    Security,
    /// Naming, formatting, idiomatic code, unnecessary complexity.
    Style,
    /// Inefficient algorithm, unnecessary allocation, blocking I/O.
    Performance,
}

/// A single review comment produced by an [`AiProvider`].
///
/// Instances are serialised to JSON for the platform API and for the
/// Reflect & Review second pass (see [`crate::review::engine::ReviewEngine`]).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewComment {
    /// File path, relative to the repository root.
    pub file: String,
    /// Line number in the **new** version of the file.
    pub line: u32,
    /// How severe the issue is.
    pub severity: Severity,
    /// Which category the issue belongs to.
    pub category: Category,
    /// Short, human-readable title (shown in the PR comment header).
    pub title: String,
    /// Detailed explanation of the problem and why it matters.
    pub body: String,
    /// Optional concrete code fix.  When present, posted as a GitHub
    /// suggestion block so the author can apply it with one click.
    pub suggestion: Option<String>,
}

// ── Provider trait ────────────────────────────────────────────────────────────

/// Trait implemented by every AI backend.
///
/// Create an instance via [`build_provider`].
///
/// # Examples
///
/// ```no_run
/// use merlin::ai::{build_provider, ReviewContext};
/// use merlin::config::AiConfig;
///
/// # async fn example() -> merlin::error::Result<()> {
/// let cfg = AiConfig::default();
/// let provider = build_provider(&cfg)?;
/// let ctx = ReviewContext { file: "src/main.rs".into(), diff_hunk: "+fn foo() {}".into(), full_file: None };
/// let comments = provider.review(&ctx).await?;
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait AiProvider: Send + Sync {
    /// Perform a structured code review of `ctx`.
    ///
    /// Returns a (possibly empty) list of [`ReviewComment`]s.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::MerlinError::AiProvider`] if the upstream
    /// API call fails or the response cannot be parsed.
    async fn review(&self, ctx: &ReviewContext) -> Result<Vec<ReviewComment>>;

    /// Perform freeform text generation given a `system` and a `user` prompt.
    ///
    /// Used by slash-command tools (e.g. `/describe`, `/spec`) that need a
    /// plain-text or Markdown response rather than structured JSON.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::MerlinError::AiProvider`] if the call fails.
    async fn generate(&self, system: &str, user: &str) -> Result<String>;
}

// ── Prompt helpers ────────────────────────────────────────────────────────────

/// Build the default code-review system prompt for `focus` categories.
///
/// Delegates to [`system_prompt_with_persona`] with no persona override.
pub fn system_prompt(focus: &[String]) -> String {
    system_prompt_with_persona(focus, None)
}

/// Build the code-review system prompt, optionally applying a
/// [`crate::config::PersonaConfig`] override.
///
/// If a persona provides `focus_override`, those categories replace `focus`.
/// Custom `rules` are appended as a numbered list after the base prompt.
/// Additional `rule_directives` from the custom rules engine are also appended.
pub fn system_prompt_with_persona(
    focus: &[String],
    persona: Option<&crate::config::PersonaConfig>,
) -> String {
    system_prompt_with_rules(focus, persona, &[])
}

/// Build the code-review system prompt with persona overrides and custom rule directives.
pub fn system_prompt_with_rules(
    focus: &[String],
    persona: Option<&crate::config::PersonaConfig>,
    rule_directives: &[String],
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

    if let Some(p) = persona {
        if let Some(rules) = &p.rules {
            if !rules.is_empty() {
                prompt.push_str("\n\n## Review Rules\n\nApply these additional rules:\n");
                for (i, rule) in rules.iter().enumerate() {
                    prompt.push_str(&format!("{}. {}\n", i + 1, rule));
                }
            }
        }
        if let Some(extra) = &p.system_prompt_extra {
            if !extra.is_empty() {
                prompt.push_str(&format!("\n\n{extra}"));
            }
        }
    }

    // Append custom rules engine directives
    if !rule_directives.is_empty() {
        prompt.push_str("\n\n## Custom Team Rules\n\nEnforce these team-specific rules:\n");
        for (i, directive) in rule_directives.iter().enumerate() {
            prompt.push_str(&format!("{}. {}\n", i + 1, directive));
        }
    }

    prompt
}

// ── Factory ───────────────────────────────────────────────────────────────────

/// Instantiate the AI backend specified by `cfg`.
///
/// Reads the required API key or credentials from environment variables.
///
/// # Errors
///
/// Returns [`crate::error::MerlinError::Config`] when a required environment
/// variable is missing, or when the `claude` CLI binary is not on `PATH` for
/// the `claude-code` provider.
///
/// # Example
///
/// ```no_run
/// use merlin::ai::build_provider;
/// use merlin::config::AiConfig;
///
/// let cfg = AiConfig::default(); // provider = "anthropic"
/// let provider = build_provider(&cfg).unwrap();
/// ```
pub fn build_provider(cfg: &AiConfig) -> Result<Box<dyn AiProvider>> {
    match cfg.provider {
        AiProviderType::Anthropic => {
            let key = Config::anthropic_api_key()?;
            Ok(Box::new(anthropic::AnthropicProvider::new(
                key,
                cfg.clone(),
            )))
        }
        AiProviderType::Openai => {
            let key = Config::openai_api_key()?;
            let base_url = cfg
                .openai_base_url
                .clone()
                .unwrap_or_else(|| "https://api.openai.com/v1/chat/completions".to_string());
            Ok(Box::new(openai::OpenAiProvider::new(
                key,
                cfg.clone(),
                base_url,
                true,
            )))
        }
        AiProviderType::Groq => {
            let key = Config::groq_api_key()?;
            Ok(Box::new(openai::OpenAiProvider::new(
                key,
                cfg.clone(),
                "https://api.groq.com/openai/v1/chat/completions".to_string(),
                true, // Groq supports json_object for Llama 3 / Mixtral
            )))
        }
        AiProviderType::TogetherAi => {
            let key = Config::together_api_key()?;
            Ok(Box::new(openai::OpenAiProvider::new(
                key,
                cfg.clone(),
                "https://api.together.xyz/v1/chat/completions".to_string(),
                false, // JSON mode support varies per model on Together AI
            )))
        }
        AiProviderType::DeepSeek => {
            let key = Config::deepseek_api_key()?;
            Ok(Box::new(openai::OpenAiProvider::new(
                key,
                cfg.clone(),
                "https://api.deepseek.com/chat/completions".to_string(),
                true, // DeepSeek supports json_object mode
            )))
        }
        AiProviderType::Mistral => {
            let key = Config::mistral_api_key()?;
            Ok(Box::new(openai::OpenAiProvider::new(
                key,
                cfg.clone(),
                "https://api.mistral.ai/v1/chat/completions".to_string(),
                false, // Mistral JSON mode varies by model; use text fallback
            )))
        }
        AiProviderType::OpenRouter => {
            let key = Config::openrouter_api_key()?;
            Ok(Box::new(openai::OpenAiProvider::new(
                key,
                cfg.clone(),
                "https://openrouter.ai/api/v1/chat/completions".to_string(),
                false, // JSON mode depends on the underlying routed model
            )))
        }
        AiProviderType::ClaudeCode => {
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
                    "`claude` CLI not found on PATH. \
                     Install Claude Code: https://claude.ai/claude-code"
                        .to_string(),
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
            Ok(Box::new(azure_openai::AzureOpenAiProvider::new(
                key,
                cfg.clone(),
            )))
        }
        AiProviderType::Bedrock => {
            let (access, secret, token) = Config::aws_credentials()?;
            Ok(Box::new(bedrock::BedrockProvider::new(
                access,
                secret,
                token,
                cfg.clone(),
            )))
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_prompt_contains_focus() {
        let prompt = system_prompt(&["bugs".to_string(), "security".to_string()]);
        assert!(prompt.contains("bugs, security"));
        assert!(prompt.contains("JSON array"));
    }

    #[test]
    fn system_prompt_with_persona_overrides_focus() {
        use crate::config::PersonaConfig;
        let persona = PersonaConfig {
            name: Some("security expert".to_string()),
            system_prompt_extra: Some("Be extra strict.".to_string()),
            focus_override: Some(vec!["security".to_string()]),
            rules: Some(vec![
                "Never approve SQL without parameterisation.".to_string()
            ]),
        };
        let prompt = system_prompt_with_persona(&["bugs".to_string()], Some(&persona));
        assert!(prompt.contains("security expert"));
        assert!(prompt.contains("security"));
        assert!(
            !prompt.contains("bugs"),
            "focus override should suppress original focus"
        );
        assert!(prompt.contains("Review Rules"));
        assert!(prompt.contains("SQL without parameterisation"));
        assert!(prompt.contains("Be extra strict"));
    }

    #[test]
    fn severity_ordered_correctly() {
        assert!(Severity::Critical < Severity::High);
        assert!(Severity::High < Severity::Medium);
        assert!(Severity::Medium < Severity::Low);
        assert!(Severity::Low < Severity::Info);
    }
}
