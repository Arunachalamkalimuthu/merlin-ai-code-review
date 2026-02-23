//! AI provider type, model configuration, and review persona.

use serde::{Deserialize, Serialize};

/// Which AI backend to use.
///
/// Serialises as a kebab-case string in TOML (e.g. `"azure-openai"`).
/// Each variant requires different environment variables — see
/// [`crate::ai::build_provider`] and the individual provider modules for
/// details.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AiProviderType {
    /// Anthropic Claude API (requires `ANTHROPIC_API_KEY`). Default provider.
    #[default]
    Anthropic,
    /// OpenAI GPT models (requires `OPENAI_API_KEY`).
    Openai,
    /// Use the `claude` CLI (Claude Code) for authentication instead of an API key.
    ClaudeCode,
    /// Google Gemini (requires `GEMINI_API_KEY`).
    Gemini,
    /// Local Ollama instance (no API key required).
    Ollama,
    /// Azure OpenAI Service (requires `AZURE_OPENAI_API_KEY`).
    AzureOpenai,
    /// Amazon Bedrock — Claude models (requires AWS credentials).
    Bedrock,
    /// Groq — ultra-fast inference for Llama 3, Mixtral, Gemma (requires `GROQ_API_KEY`).
    Groq,
    /// Together AI — hosted open-source models (requires `TOGETHER_API_KEY`).
    TogetherAi,
    /// DeepSeek — DeepSeek Coder / Chat models (requires `DEEPSEEK_API_KEY`).
    DeepSeek,
    /// Mistral AI — Mistral and Codestral models (requires `MISTRAL_API_KEY`).
    Mistral,
    /// OpenRouter — unified gateway to 100+ models (requires `OPENROUTER_API_KEY`).
    OpenRouter,
}

/// AI provider settings — maps to the `[ai]` table in `merlin.toml`.
///
/// Construct with [`AiConfig::default`] for a ready-to-use Anthropic/Claude
/// configuration, then override individual fields as needed.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AiConfig {
    /// Which AI backend to use (default: `"anthropic"`).
    #[serde(default)]
    pub provider: AiProviderType,
    /// Model name passed to the AI backend (default: `"claude-sonnet-4-6"`).
    #[serde(default = "default_model")]
    pub model: String,
    /// Maximum tokens the AI may generate per request (default: 4096).
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// Sampling temperature — lower values produce more deterministic output (default: 0.2).
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// (claude-code) Token for `claude auth login --token <TOKEN>` in CI.
    pub claude_code_token: Option<String>,
    /// (ollama) Base URL of the Ollama server. Default: `http://localhost:11434`.
    pub ollama_base_url: Option<String>,
    /// (openai) Override the default OpenAI endpoint. Use this to point the OpenAI
    /// provider at any OpenAI-compatible API (e.g. a local proxy or custom deployment).
    pub openai_base_url: Option<String>,
    /// (azure-openai) Full endpoint URL, e.g. `https://{resource}.openai.azure.com`.
    pub azure_openai_endpoint: Option<String>,
    /// (azure-openai) API version, e.g. `"2024-02-01"`.
    pub azure_openai_api_version: Option<String>,
    /// (bedrock) AWS region, e.g. `"us-east-1"`.
    pub bedrock_region: Option<String>,
}

fn default_model() -> String {
    "claude-sonnet-4-6".to_string()
}

fn default_max_tokens() -> u32 {
    4096
}

fn default_temperature() -> f32 {
    0.2
}

impl Default for AiConfig {
    fn default() -> Self {
        AiConfig {
            provider: AiProviderType::default(),
            model: default_model(),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            claude_code_token: None,
            ollama_base_url: None,
            openai_base_url: None,
            azure_openai_endpoint: None,
            azure_openai_api_version: None,
            bedrock_region: None,
        }
    }
}

impl AiConfig {
    /// Returns the review focus categories, defaulting to all four if not set.
    pub fn review_focus(&self) -> Vec<String> {
        // AiConfig doesn't own focus — that lives in ReviewConfig.
        // Providers use this default when no ReviewConfig is available.
        vec![
            "bugs".to_string(),
            "security".to_string(),
            "style".to_string(),
            "performance".to_string(),
        ]
    }
}

// ── Review persona ─────────────────────────────────────────────────────────────

/// A named review persona that overrides the default system prompt.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PersonaConfig {
    /// Display name for the persona (e.g. `"security-focused"`, `"nitpicky"`).
    pub name: Option<String>,
    /// Extra instructions appended to the base system prompt.
    pub system_prompt_extra: Option<String>,
    /// Override the default focus categories for this persona.
    pub focus_override: Option<Vec<String>>,
    /// Custom review rules (appended as a numbered list to the system prompt).
    pub rules: Option<Vec<String>>,
}
