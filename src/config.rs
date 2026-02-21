use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::error::{MerlinError, Result};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AiProviderType {
    #[default]
    Anthropic,
    Openai,
    /// Use the `claude` CLI (Claude Code) for authentication instead of an API key.
    ClaudeCode,
    /// Google Gemini (requires GEMINI_API_KEY)
    Gemini,
    /// Local Ollama instance (no API key required)
    Ollama,
    /// Azure OpenAI Service (requires AZURE_OPENAI_API_KEY)
    AzureOpenai,
    /// Amazon Bedrock — Claude models (requires AWS credentials)
    Bedrock,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AiConfig {
    #[serde(default)]
    pub provider: AiProviderType,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// (claude-code) Token for `claude auth login --token <TOKEN>` in CI.
    pub claude_code_token: Option<String>,
    /// (ollama) Base URL of the Ollama server. Default: http://localhost:11434
    pub ollama_base_url: Option<String>,
    /// (azure-openai) Full endpoint URL, e.g. https://{resource}.openai.azure.com
    pub azure_openai_endpoint: Option<String>,
    /// (azure-openai) API version, e.g. "2024-02-01"
    pub azure_openai_api_version: Option<String>,
    /// (bedrock) AWS region, e.g. "us-east-1"
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
    /// Display name for the persona (e.g. "security-focused", "nitpicky").
    pub name: Option<String>,
    /// Extra instructions appended to the base system prompt.
    pub system_prompt_extra: Option<String>,
    /// Override the default focus categories for this persona.
    pub focus_override: Option<Vec<String>>,
    /// Custom review rules (appended as a numbered list to the system prompt).
    pub rules: Option<Vec<String>>,
}

// ── Review config ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReviewConfig {
    #[serde(default = "default_focus")]
    pub focus: Vec<String>,
    #[serde(default = "default_max_comments")]
    pub max_comments: usize,
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

// ── Jira integration ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct JiraConfig {
    /// Base URL of your Jira instance, e.g. https://company.atlassian.net
    pub base_url: Option<String>,
    /// Jira project key to search in, e.g. "PROJ"
    pub project_key: Option<String>,
    /// Jira user email (for Basic auth with JIRA_TOKEN)
    pub user_email: Option<String>,
}

impl JiraConfig {
    pub fn is_configured(&self) -> bool {
        self.base_url.is_some()
    }
}

// ── Linear integration ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct LinearConfig {
    /// Linear team ID to scope searches (optional)
    pub team_id: Option<String>,
}

impl LinearConfig {
    pub fn is_configured(&self) -> bool {
        std::env::var("LINEAR_API_KEY").is_ok()
    }
}

// ── Coverage config ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CoverageConfig {
    /// Coverage report format: "lcov" | "cobertura" | "json"
    #[serde(default = "default_coverage_format")]
    pub format: String,
    /// Path to the coverage report file
    #[serde(default = "default_coverage_report_path")]
    pub report_path: String,
    /// Minimum required coverage % (0–100). 0 disables threshold enforcement.
    #[serde(default)]
    pub threshold: f32,
}

fn default_coverage_format() -> String {
    "lcov".to_string()
}

fn default_coverage_report_path() -> String {
    "coverage/lcov.info".to_string()
}

impl Default for CoverageConfig {
    fn default() -> Self {
        CoverageConfig {
            format: default_coverage_format(),
            report_path: default_coverage_report_path(),
            threshold: 0.0,
        }
    }
}

// ── Audit log config ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuditConfig {
    /// Enable audit logging.
    #[serde(default = "default_audit_enabled")]
    pub enabled: bool,
    /// Path to the JSONL audit log file.
    #[serde(default = "default_audit_path")]
    pub log_path: String,
}

fn default_audit_enabled() -> bool {
    true
}

fn default_audit_path() -> String {
    "merlin-audit.jsonl".to_string()
}

impl Default for AuditConfig {
    fn default() -> Self {
        AuditConfig {
            enabled: default_audit_enabled(),
            log_path: default_audit_path(),
        }
    }
}

// ── Platform config ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlatformType {
    Github,
    Gitlab,
    Bitbucket,
    AzureDevops,
    Gitea,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PlatformConfig {
    #[serde(rename = "type")]
    pub platform_type: Option<PlatformType>,
}

// ── Snyk config ────────────────────────────────────────────────────────────────

/// Snyk vulnerability database integration config.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct SnykConfig {
    /// Enable Snyk scanning (requires SNYK_TOKEN env var).
    #[serde(default)]
    pub enabled: bool,
    /// Snyk organization ID (optional — defaults to personal org of the token).
    pub org_id: Option<String>,
}

// ── RAG config ─────────────────────────────────────────────────────────────────

/// Which embedding backend to use for RAG.
///
/// | Value    | Needs                           | Best for               |
/// |----------|---------------------------------|------------------------|
/// | `ollama` | `ollama serve` + pulled model   | Local dev (free)       |
/// | `openai` | `OPENAI_API_KEY` env var        | CI/CD (any runner)     |
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum EmbedderType {
    /// Local Ollama instance (default — zero cloud dependency).
    #[default]
    Ollama,
    /// OpenAI Embeddings API (`text-embedding-3-small` by default).
    Openai,
}

/// Which vector store backend to use for RAG.
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum VectorStoreType {
    /// Zero-setup: JSONL flat file with brute-force cosine similarity.
    /// Best for repos up to ~5 K files.
    #[default]
    Local,
    /// Ephemeral in-memory store — resets on restart. Useful for testing.
    Memory,
    /// Qdrant REST API (self-hosted or Qdrant Cloud).
    Qdrant,
    /// ChromaDB REST API (self-hosted).
    Chroma,
    /// Pinecone cloud vector database.
    Pinecone,
}

/// RAG (Retrieval-Augmented Generation) pipeline configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RagConfig {
    /// Enable RAG augmentation during code review and agent calls.
    #[serde(default)]
    pub enabled: bool,

    /// Embedding backend: `"ollama"` (local, default) or `"openai"` (CI-friendly).
    #[serde(default)]
    pub embedder: EmbedderType,

    /// Vector store backend.
    #[serde(default)]
    pub store: VectorStoreType,

    /// Collection / namespace / index name (default: "merlin").
    #[serde(default = "default_rag_collection")]
    pub collection: String,

    /// Ollama embedding model (default: "nomic-embed-text").
    #[serde(default = "default_embed_model")]
    pub embed_model: String,

    /// Ollama base URL for embeddings (default: "http://localhost:11434").
    #[serde(default = "default_ollama_embed_url")]
    pub ollama_base_url: String,

    /// Number of documents to retrieve per query (default: 5).
    #[serde(default = "default_rag_top_k")]
    pub top_k: usize,

    /// Minimum cosine similarity score to include a result (default: 0.70).
    #[serde(default = "default_rag_min_score")]
    pub min_score: f32,

    /// Lines per file chunk when indexing (default: 80).
    #[serde(default = "default_rag_chunk_lines")]
    pub chunk_lines: usize,

    /// File extensions to index (default: Rust, Python, TS/JS, Go, Java, Markdown).
    #[serde(default = "default_index_extensions")]
    pub index_extensions: Vec<String>,

    // ── Local store ───────────────────────────────────────────────────────────
    /// Path to the JSONL vector store file (default: "merlin-rag.jsonl").
    #[serde(default = "default_local_rag_path")]
    pub local_path: String,

    // ── Qdrant ────────────────────────────────────────────────────────────────
    /// Qdrant REST API URL (default: "http://localhost:6333").
    #[serde(default = "default_qdrant_url")]
    pub qdrant_url: String,
    /// Qdrant API key (optional — required for Qdrant Cloud).
    pub qdrant_api_key: Option<String>,

    // ── ChromaDB ──────────────────────────────────────────────────────────────
    /// ChromaDB REST API URL (default: "http://localhost:8000").
    #[serde(default = "default_chroma_url")]
    pub chroma_url: String,

    // ── Pinecone ──────────────────────────────────────────────────────────────
    /// Pinecone API key (from PINECONE_API_KEY env var or here).
    pub pinecone_api_key: Option<String>,
    /// Pinecone index host URL (e.g. "https://my-index-xyz.svc.us-east1.pinecone.io").
    pub pinecone_host: Option<String>,
}

fn default_rag_collection() -> String { "merlin".to_string() }
fn default_embed_model() -> String { "nomic-embed-text".to_string() }
fn default_ollama_embed_url() -> String { "http://localhost:11434".to_string() }
fn default_rag_top_k() -> usize { 5 }
fn default_rag_min_score() -> f32 { 0.70 }
fn default_rag_chunk_lines() -> usize { 80 }
fn default_local_rag_path() -> String { "merlin-rag.jsonl".to_string() }
fn default_qdrant_url() -> String { "http://localhost:6333".to_string() }
fn default_chroma_url() -> String { "http://localhost:8000".to_string() }
fn default_index_extensions() -> Vec<String> {
    [".rs", ".py", ".ts", ".js", ".tsx", ".jsx", ".go", ".java", ".kt", ".rb", ".md", ".toml", ".yaml", ".yml"]
        .iter().map(|s| s.to_string()).collect()
}

impl Default for RagConfig {
    fn default() -> Self {
        RagConfig {
            enabled: false,
            embedder: EmbedderType::default(),
            store: VectorStoreType::default(),
            collection: default_rag_collection(),
            embed_model: default_embed_model(),
            ollama_base_url: default_ollama_embed_url(),
            top_k: default_rag_top_k(),
            min_score: default_rag_min_score(),
            chunk_lines: default_rag_chunk_lines(),
            index_extensions: default_index_extensions(),
            local_path: default_local_rag_path(),
            qdrant_url: default_qdrant_url(),
            qdrant_api_key: None,
            chroma_url: default_chroma_url(),
            pinecone_api_key: None,
            pinecone_host: None,
        }
    }
}

// ── Agent config ───────────────────────────────────────────────────────────────

/// Configuration for the autonomous agent runtime.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentConfig {
    /// Maximum ReAct iterations per task (default: 10).
    pub max_iterations: Option<usize>,
    /// Maximum conversation messages to keep in memory (default: 50).
    #[serde(default = "default_max_memory_messages")]
    pub max_memory_messages: usize,
    /// Path to the JSONL memory persistence file. `None` = in-memory only.
    pub memory_file: Option<String>,
    /// Default channel: "cli" | "slack" | "discord" (default: "cli").
    #[serde(default = "default_agent_channel")]
    pub default_channel: String,
    /// HTTP port for Slack/Discord webhook servers (default: 8090).
    #[serde(default = "default_agent_port")]
    pub port: u16,
}

fn default_max_memory_messages() -> usize {
    50
}
fn default_agent_channel() -> String {
    "cli".to_string()
}
fn default_agent_port() -> u16 {
    8090
}

impl Default for AgentConfig {
    fn default() -> Self {
        AgentConfig {
            max_iterations: None,
            max_memory_messages: default_max_memory_messages(),
            memory_file: None,
            default_channel: default_agent_channel(),
            port: default_agent_port(),
        }
    }
}

// ── Root config ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    #[serde(default)]
    pub ai: AiConfig,
    #[serde(default)]
    pub review: ReviewConfig,
    #[serde(default)]
    pub platform: PlatformConfig,
    #[serde(default)]
    pub jira: JiraConfig,
    #[serde(default)]
    pub linear: LinearConfig,
    #[serde(default)]
    pub coverage: CoverageConfig,
    #[serde(default)]
    pub audit: AuditConfig,
    #[serde(default)]
    pub snyk: SnykConfig,
    #[serde(default)]
    pub agent: AgentConfig,
    #[serde(default)]
    pub rag: RagConfig,
}

impl Config {
    /// Load config from a TOML file, falling back to defaults if not found.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            tracing::debug!("Config file not found at {:?}, using defaults", path);
            return Ok(Config::default());
        }
        let contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Load from the default location (`merlin.toml` in the current directory).
    pub fn load_default() -> Result<Self> {
        Self::load(Path::new("merlin.toml"))
    }

    /// Return the Anthropic API key from env.
    pub fn anthropic_api_key() -> Result<String> {
        std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| MerlinError::EnvVar("ANTHROPIC_API_KEY".to_string()))
    }

    /// Return the OpenAI API key from env.
    pub fn openai_api_key() -> Result<String> {
        std::env::var("OPENAI_API_KEY")
            .map_err(|_| MerlinError::EnvVar("OPENAI_API_KEY".to_string()))
    }

    /// Return the Google Gemini API key from env.
    pub fn gemini_api_key() -> Result<String> {
        std::env::var("GEMINI_API_KEY")
            .map_err(|_| MerlinError::EnvVar("GEMINI_API_KEY".to_string()))
    }

    /// Return the Azure OpenAI API key from env.
    pub fn azure_openai_api_key() -> Result<String> {
        std::env::var("AZURE_OPENAI_API_KEY")
            .map_err(|_| MerlinError::EnvVar("AZURE_OPENAI_API_KEY".to_string()))
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

    /// Return the Jira API token from env.
    pub fn jira_token() -> Result<String> {
        std::env::var("JIRA_TOKEN")
            .map_err(|_| MerlinError::EnvVar("JIRA_TOKEN".to_string()))
    }

    /// Return the Linear API key from env.
    pub fn linear_api_key() -> Result<String> {
        std::env::var("LINEAR_API_KEY")
            .map_err(|_| MerlinError::EnvVar("LINEAR_API_KEY".to_string()))
    }

    /// Return the GitHub token from env.
    pub fn github_token() -> Result<String> {
        std::env::var("GITHUB_TOKEN")
            .map_err(|_| MerlinError::EnvVar("GITHUB_TOKEN".to_string()))
    }

    /// Return the GitLab token from env.
    pub fn gitlab_token() -> Result<String> {
        std::env::var("GITLAB_TOKEN")
            .map_err(|_| MerlinError::EnvVar("GITLAB_TOKEN".to_string()))
    }

    /// Return the Bitbucket token from env (bearer token or app password).
    pub fn bitbucket_token() -> Result<String> {
        std::env::var("BITBUCKET_TOKEN")
            .or_else(|_| std::env::var("BITBUCKET_APP_PASSWORD"))
            .map_err(|_| MerlinError::EnvVar("BITBUCKET_TOKEN".to_string()))
    }

    /// Return the Azure DevOps PAT from env.
    pub fn azure_devops_token() -> Result<String> {
        std::env::var("AZURE_DEVOPS_TOKEN")
            .or_else(|_| std::env::var("SYSTEM_ACCESSTOKEN"))
            .map_err(|_| MerlinError::EnvVar("AZURE_DEVOPS_TOKEN".to_string()))
    }

    /// Return the Gitea token from env.
    pub fn gitea_token() -> Result<String> {
        std::env::var("GITEA_TOKEN")
            .map_err(|_| MerlinError::EnvVar("GITEA_TOKEN".to_string()))
    }

    /// Return the Snyk API token from env.
    pub fn snyk_token() -> Result<String> {
        std::env::var("SNYK_TOKEN")
            .map_err(|_| MerlinError::EnvVar("SNYK_TOKEN".to_string()))
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
