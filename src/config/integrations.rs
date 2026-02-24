//! Third-party integration configs: Jira, Linear, Snyk, Coverage, Audit, and Agent.

use serde::{Deserialize, Serialize};

// ── Jira integration ───────────────────────────────────────────────────────────

/// Jira integration settings — maps to the `[jira]` table in `merlin.toml`.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct JiraConfig {
    /// Base URL of your Jira instance, e.g. `https://company.atlassian.net`.
    pub base_url: Option<String>,
    /// Jira project key to search in, e.g. `"PROJ"`.
    pub project_key: Option<String>,
    /// Jira user email (for Basic auth with `JIRA_TOKEN`).
    pub user_email: Option<String>,
}

impl JiraConfig {
    /// Returns `true` if a Jira base URL has been configured.
    pub fn is_configured(&self) -> bool {
        self.base_url.is_some()
    }
}

// ── Linear integration ─────────────────────────────────────────────────────────

/// Linear integration settings — maps to the `[linear]` table in `merlin.toml`.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct LinearConfig {
    /// Linear team ID to scope searches (optional).
    pub team_id: Option<String>,
}

impl LinearConfig {
    /// Returns `true` if `LINEAR_API_KEY` is set in the environment.
    pub fn is_configured(&self) -> bool {
        std::env::var("LINEAR_API_KEY").is_ok()
    }
}

// ── Coverage config ────────────────────────────────────────────────────────────

/// Code-coverage reporting settings — maps to the `[coverage]` table in `merlin.toml`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CoverageConfig {
    /// Coverage report format: `"lcov"` | `"cobertura"` | `"json"`.
    #[serde(default = "default_coverage_format")]
    pub format: String,
    /// Path to the coverage report file.
    #[serde(default = "default_coverage_report_path")]
    pub report_path: String,
    /// Minimum required coverage % (0–100). `0` disables threshold enforcement.
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

/// Audit-log settings — maps to the `[audit]` table in `merlin.toml`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuditConfig {
    /// Enable audit logging (default: `true`).
    #[serde(default = "default_audit_enabled")]
    pub enabled: bool,
    /// Path to the JSONL audit log file (default: `"merlin-audit.jsonl"`).
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

// ── Snyk config ────────────────────────────────────────────────────────────────

/// Snyk vulnerability database integration config — maps to `[snyk]` in `merlin.toml`.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct SnykConfig {
    /// Enable Snyk scanning (requires `SNYK_TOKEN` env var).
    #[serde(default)]
    pub enabled: bool,
    /// Snyk organization ID (optional — defaults to personal org of the token).
    pub org_id: Option<String>,
}

// ── Agent config ───────────────────────────────────────────────────────────────

/// Configuration for the autonomous agent runtime — maps to `[agent]` in `merlin.toml`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentConfig {
    /// Maximum ReAct iterations per task (default: 10). `None` uses the built-in default.
    pub max_iterations: Option<usize>,
    /// Maximum conversation messages to keep in memory (default: 50).
    #[serde(default = "default_max_memory_messages")]
    pub max_memory_messages: usize,
    /// Path to the JSONL memory persistence file. `None` = in-memory only.
    pub memory_file: Option<String>,
    /// Default channel: `"cli"` | `"slack"` | `"discord"` (default: `"cli"`).
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
