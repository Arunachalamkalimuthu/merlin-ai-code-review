//! VCS platform type and connection settings.

use serde::{Deserialize, Serialize};

/// Which VCS platform to target.
///
/// Serialises as a kebab-case string in TOML (e.g. `"azure-devops"`).
/// Set via `[platform] type = "github"` in `merlin.toml`, or Merlin will
/// auto-detect from environment variables at runtime.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlatformType {
    /// GitHub (github.com or GitHub Enterprise).
    Github,
    /// GitLab (gitlab.com or self-hosted).
    Gitlab,
    /// Atlassian Bitbucket Cloud.
    Bitbucket,
    /// Azure DevOps (dev.azure.com).
    AzureDevops,
    /// Gitea (self-hosted).
    Gitea,
}

/// VCS platform connection settings — maps to the `[platform]` table in `merlin.toml`.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PlatformConfig {
    /// Which VCS platform to use. When absent, Merlin auto-detects from CI environment variables.
    #[serde(rename = "type")]
    pub platform_type: Option<PlatformType>,
}
