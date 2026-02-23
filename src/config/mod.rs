//! Configuration schema, TOML loading, and environment-variable credential helpers.
//!
//! [`Config`] is the root struct that maps directly to `merlin.toml`.  Load it
//! with [`Config::load`] or [`Config::load_default`]; all fields have sensible
//! defaults so an empty (or missing) file is valid.
//!
//! Secrets are read from environment variables — never stored in the config
//! file.  The helper methods on [`Config`] (e.g. [`Config::anthropic_api_key`])
//! return [`crate::error::MerlinError::EnvVar`] when the required variable is
//! absent, so callers get a clear error message instead of a panic.
//!
//! # Example — minimal `merlin.toml`
//!
//! ```toml
//! [ai]
//! provider   = "anthropic"
//! model      = "claude-sonnet-4-6"
//! max_tokens = 4096
//!
//! [review]
//! focus        = ["bugs", "security"]
//! max_comments = 20
//! reflect      = true
//! ```

pub mod ai;
pub mod integrations;
pub mod platform;
pub mod rag;
pub mod review;

pub use ai::{AiConfig, AiProviderType, PersonaConfig};
pub use integrations::{
    AgentConfig, AuditConfig, CoverageConfig, JiraConfig, LinearConfig, SnykConfig,
};
pub use platform::{PlatformConfig, PlatformType};
pub use rag::{EmbedderType, RagConfig, VectorStoreType};
pub use review::ReviewConfig;

use std::path::Path;

use crate::error::{MerlinError, Result};

/// Root configuration struct — the complete contents of `merlin.toml`.
///
/// Load with [`Config::load`] or [`Config::load_default`].  All sub-structs
/// have sensible defaults, so an empty file — or no file at all — is valid.
///
/// # Examples
///
/// ```no_run
/// use merlin::config::Config;
///
/// let cfg = Config::load_default().unwrap();
/// assert_eq!(cfg.ai.max_tokens, 4096);
/// ```
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
pub struct Config {
    /// AI provider and model settings.
    #[serde(default)]
    pub ai: AiConfig,
    /// Review behaviour settings (focus, chunk size, comment cap).
    #[serde(default)]
    pub review: ReviewConfig,
    /// VCS platform connection settings.
    #[serde(default)]
    pub platform: PlatformConfig,
    /// Jira integration settings.
    #[serde(default)]
    pub jira: JiraConfig,
    /// Linear integration settings.
    #[serde(default)]
    pub linear: LinearConfig,
    /// Code coverage report settings.
    #[serde(default)]
    pub coverage: CoverageConfig,
    /// Audit log settings.
    #[serde(default)]
    pub audit: AuditConfig,
    /// Snyk vulnerability scanning settings.
    #[serde(default)]
    pub snyk: SnykConfig,
    /// Autonomous agent runtime settings.
    #[serde(default)]
    pub agent: AgentConfig,
    /// RAG pipeline settings.
    #[serde(default)]
    pub rag: RagConfig,
}

impl Config {
    /// Load config from a TOML file, falling back to all-defaults if the file
    /// is not found.
    ///
    /// # Errors
    ///
    /// Returns [`MerlinError::Io`] if the file exists but cannot be read, or
    /// [`MerlinError::TomlDe`] if the file is not valid TOML.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            tracing::debug!("Config file not found at {:?}, using defaults", path);
            return Ok(Config::default());
        }
        let contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Load from the default path (`merlin.toml` in the current directory).
    ///
    /// Equivalent to `Config::load(Path::new("merlin.toml"))`.
    ///
    /// # Errors
    ///
    /// See [`Config::load`].
    pub fn load_default() -> Result<Self> {
        Self::load(Path::new("merlin.toml"))
    }

    /// Return the Anthropic API key from env (`ANTHROPIC_API_KEY`).
    pub fn anthropic_api_key() -> Result<String> {
        std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| MerlinError::EnvVar("ANTHROPIC_API_KEY".to_string()))
    }

    /// Return the OpenAI API key from env (`OPENAI_API_KEY`).
    pub fn openai_api_key() -> Result<String> {
        std::env::var("OPENAI_API_KEY")
            .map_err(|_| MerlinError::EnvVar("OPENAI_API_KEY".to_string()))
    }

    /// Return the Google Gemini API key from env (`GEMINI_API_KEY`).
    pub fn gemini_api_key() -> Result<String> {
        std::env::var("GEMINI_API_KEY")
            .map_err(|_| MerlinError::EnvVar("GEMINI_API_KEY".to_string()))
    }

    /// Return the Azure OpenAI API key from env (`AZURE_OPENAI_API_KEY`).
    pub fn azure_openai_api_key() -> Result<String> {
        std::env::var("AZURE_OPENAI_API_KEY")
            .map_err(|_| MerlinError::EnvVar("AZURE_OPENAI_API_KEY".to_string()))
    }

    /// Return the Groq API key from env (`GROQ_API_KEY`).
    pub fn groq_api_key() -> Result<String> {
        std::env::var("GROQ_API_KEY").map_err(|_| MerlinError::EnvVar("GROQ_API_KEY".to_string()))
    }

    /// Return the Together AI API key from env (`TOGETHER_API_KEY`).
    pub fn together_api_key() -> Result<String> {
        std::env::var("TOGETHER_API_KEY")
            .map_err(|_| MerlinError::EnvVar("TOGETHER_API_KEY".to_string()))
    }

    /// Return the DeepSeek API key from env (`DEEPSEEK_API_KEY`).
    pub fn deepseek_api_key() -> Result<String> {
        std::env::var("DEEPSEEK_API_KEY")
            .map_err(|_| MerlinError::EnvVar("DEEPSEEK_API_KEY".to_string()))
    }

    /// Return the Mistral AI API key from env (`MISTRAL_API_KEY`).
    pub fn mistral_api_key() -> Result<String> {
        std::env::var("MISTRAL_API_KEY")
            .map_err(|_| MerlinError::EnvVar("MISTRAL_API_KEY".to_string()))
    }

    /// Return the OpenRouter API key from env (`OPENROUTER_API_KEY`).
    pub fn openrouter_api_key() -> Result<String> {
        std::env::var("OPENROUTER_API_KEY")
            .map_err(|_| MerlinError::EnvVar("OPENROUTER_API_KEY".to_string()))
    }

    /// Return AWS credentials from env (access key, secret key, optional session token).
    pub fn aws_credentials() -> Result<(String, String, Option<String>)> {
        let access = std::env::var("AWS_ACCESS_KEY_ID")
            .map_err(|_| MerlinError::EnvVar("AWS_ACCESS_KEY_ID".to_string()))?;
        let secret = std::env::var("AWS_SECRET_ACCESS_KEY")
            .map_err(|_| MerlinError::EnvVar("AWS_SECRET_ACCESS_KEY".to_string()))?;
        let token = std::env::var("AWS_SESSION_TOKEN").ok();
        Ok((access, secret, token))
    }

    /// Return the Jira API token from env (`JIRA_TOKEN`).
    pub fn jira_token() -> Result<String> {
        std::env::var("JIRA_TOKEN").map_err(|_| MerlinError::EnvVar("JIRA_TOKEN".to_string()))
    }

    /// Return the Linear API key from env (`LINEAR_API_KEY`).
    pub fn linear_api_key() -> Result<String> {
        std::env::var("LINEAR_API_KEY")
            .map_err(|_| MerlinError::EnvVar("LINEAR_API_KEY".to_string()))
    }

    /// Return the GitHub token from env (`GITHUB_TOKEN`).
    pub fn github_token() -> Result<String> {
        std::env::var("GITHUB_TOKEN").map_err(|_| MerlinError::EnvVar("GITHUB_TOKEN".to_string()))
    }

    /// Return the GitLab token from env (`GITLAB_TOKEN`).
    pub fn gitlab_token() -> Result<String> {
        std::env::var("GITLAB_TOKEN").map_err(|_| MerlinError::EnvVar("GITLAB_TOKEN".to_string()))
    }

    /// Return the Bitbucket token from env (`BITBUCKET_TOKEN` or `BITBUCKET_APP_PASSWORD`).
    pub fn bitbucket_token() -> Result<String> {
        std::env::var("BITBUCKET_TOKEN")
            .or_else(|_| std::env::var("BITBUCKET_APP_PASSWORD"))
            .map_err(|_| MerlinError::EnvVar("BITBUCKET_TOKEN".to_string()))
    }

    /// Return the Azure DevOps PAT from env (`AZURE_DEVOPS_TOKEN` or `SYSTEM_ACCESSTOKEN`).
    pub fn azure_devops_token() -> Result<String> {
        std::env::var("AZURE_DEVOPS_TOKEN")
            .or_else(|_| std::env::var("SYSTEM_ACCESSTOKEN"))
            .map_err(|_| MerlinError::EnvVar("AZURE_DEVOPS_TOKEN".to_string()))
    }

    /// Return the Gitea token from env (`GITEA_TOKEN`).
    pub fn gitea_token() -> Result<String> {
        std::env::var("GITEA_TOKEN").map_err(|_| MerlinError::EnvVar("GITEA_TOKEN".to_string()))
    }

    /// Return the Snyk API token from env (`SNYK_TOKEN`).
    pub fn snyk_token() -> Result<String> {
        std::env::var("SNYK_TOKEN").map_err(|_| MerlinError::EnvVar("SNYK_TOKEN".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert_eq!(cfg.ai.max_tokens, 4096);
        assert_eq!(cfg.review.max_comments, 30);
        assert_eq!(cfg.review.chunk_lines, 200);
        assert_eq!(cfg.review.focus.len(), 4);
        assert!(!cfg.review.reflect);
        assert!(cfg.jira.base_url.is_none());
        assert!(cfg.coverage.threshold == 0.0);
        assert!(cfg.audit.enabled);
    }

    #[test]
    fn test_load_toml() {
        let toml = r#"
[ai]
provider = "openai"
model = "gpt-4o"
max_tokens = 2048
temperature = 0.5

[review]
max_comments = 10
chunk_lines = 100
focus = ["bugs", "security"]
reflect = true

[review.persona]
name = "security-focused"
system_prompt_extra = "Be extra strict about security."

[jira]
base_url = "https://company.atlassian.net"
project_key = "PROJ"
"#;
        let cfg: Config = toml::from_str(toml).expect("parse failed");
        assert!(matches!(cfg.ai.provider, AiProviderType::Openai));
        assert_eq!(cfg.ai.model, "gpt-4o");
        assert_eq!(cfg.review.max_comments, 10);
        assert_eq!(cfg.review.focus, vec!["bugs", "security"]);
        assert!(cfg.review.reflect);
        assert_eq!(cfg.review.persona.name.as_deref(), Some("security-focused"));
        assert_eq!(cfg.jira.project_key.as_deref(), Some("PROJ"));
    }

    #[test]
    fn test_load_from_missing_file() {
        let result = Config::load(Path::new("/tmp/merlin_nonexistent_xyz.toml"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_from_file() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        write!(f, "[ai]\nprovider = \"anthropic\"\n").unwrap();
        let cfg = Config::load(f.path()).unwrap();
        assert!(matches!(cfg.ai.provider, AiProviderType::Anthropic));
    }

    #[test]
    fn test_azure_openai_config() {
        let toml = r#"
[ai]
provider = "azure-openai"
model = "gpt-4o"
azure_openai_endpoint = "https://myresource.openai.azure.com"
azure_openai_api_version = "2024-02-01"
"#;
        let cfg: Config = toml::from_str(toml).expect("parse failed");
        assert!(matches!(cfg.ai.provider, AiProviderType::AzureOpenai));
        assert_eq!(
            cfg.ai.azure_openai_endpoint.as_deref(),
            Some("https://myresource.openai.azure.com")
        );
    }

    #[test]
    fn test_bedrock_config() {
        let toml = r#"
[ai]
provider = "bedrock"
model = "anthropic.claude-sonnet-4-6-20250514-v1:0"
bedrock_region = "us-east-1"
"#;
        let cfg: Config = toml::from_str(toml).expect("parse failed");
        assert!(matches!(cfg.ai.provider, AiProviderType::Bedrock));
        assert_eq!(cfg.ai.bedrock_region.as_deref(), Some("us-east-1"));
    }
}
