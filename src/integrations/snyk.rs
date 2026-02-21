//! Snyk vulnerability database integration.
//!
//! Queries the Snyk REST API to look up known vulnerabilities for
//! packages and dependencies detected in the PR diff.
//!
//! Requires: `SNYK_TOKEN` env var (free Snyk account API token).
//!
//! Snyk API docs: https://apidocs.snyk.io/
//!
//! # Example merlin.toml
//! ```toml
//! [snyk]
//! enabled = true
//! org_id = "your-snyk-org-id"   # optional — uses personal org if omitted
//! ```
//!
//! Usage:
//!   @merlin /snyk            — scan dependencies in the PR diff
//!   @merlin /snyk [package]  — look up a specific package

use serde::Deserialize;
use tracing::{debug, warn};

use crate::config::SnykConfig;
use crate::error::{MerlinError, Result};

const SNYK_API_BASE: &str = "https://api.snyk.io/rest";
const SNYK_API_VERSION: &str = "2024-01-23";

// ── API types ──────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct SnykIssuesResponse {
    data: Vec<SnykIssue>,
}

#[derive(Deserialize)]
pub struct SnykIssue {
    pub id: String,
    pub attributes: SnykIssueAttributes,
}

#[derive(Deserialize)]
pub struct SnykIssueAttributes {
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub severity: String,          // "critical" | "high" | "medium" | "low"
    #[serde(default)]
    pub cwe: Vec<String>,
    #[serde(default)]
    pub cvss_v3: Option<f32>,
    #[serde(default)]
    pub exploit_maturity: Option<String>,
    pub coordinates: Option<Vec<SnykCoordinate>>,
}

#[derive(Deserialize)]
pub struct SnykCoordinate {
    pub remedies: Option<Vec<SnykRemedy>>,
}

#[derive(Deserialize)]
pub struct SnykRemedy {
    pub description: String,
    #[serde(rename = "type")]
    pub remedy_type: String,
}

/// Summary of a vulnerability returned for display.
#[derive(Debug, Clone)]
pub struct VulnSummary {
    pub id: String,
    pub package: String,
    pub title: String,
    pub severity: String,
    pub cwe: Vec<String>,
    pub cvss_v3: Option<f32>,
    pub snyk_url: String,
    pub fix: Option<String>,
}

// ── Client ──────────────────────────────────────────────────────────────────────

pub struct SnykClient {
    token: String,
    org_id: Option<String>,
    client: reqwest::Client,
}

impl SnykClient {
    pub fn new(token: String, org_id: Option<String>) -> Self {
        Self { token, org_id, client: reqwest::Client::new() }
    }

    pub fn from_env(config: &SnykConfig) -> Result<Self> {
        let token = std::env::var("SNYK_TOKEN")
            .map_err(|_| MerlinError::EnvVar("SNYK_TOKEN".to_string()))?;
        Ok(Self::new(token, config.org_id.clone()))
    }

    /// Search for vulnerabilities affecting a specific package.
    ///
    /// `ecosystem`: "npm", "pypi", "maven", "rubygems", "golang", "cargo", etc.
    /// `package_name`: e.g. "lodash", "requests", "log4j"
    /// `version`: optional version string
    pub async fn search_package_vulns(
        &self,
        ecosystem: &str,
        package_name: &str,
        version: Option<&str>,
    ) -> Result<Vec<VulnSummary>> {
        let org = match &self.org_id {
            Some(id) => id.clone(),
            None => self.get_personal_org_id().await?,
        };

        let url = format!(
            "{SNYK_API_BASE}/orgs/{org}/issues?version={SNYK_API_VERSION}&type=package_vulnerability&package_name={package_name}&package_ecosystem={ecosystem}"
        );

        let url = if let Some(ver) = version {
            format!("{url}&package_version={ver}")
        } else {
            url
        };

        debug!("Querying Snyk for {ecosystem}/{package_name}");

        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("token {}", self.token))
            .header("Content-Type", "application/vnd.api+json")
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            warn!("Snyk API error {status} for {package_name}: {body}");
            return Ok(vec![]);
        }

        let result: SnykIssuesResponse =
            resp.json().await.unwrap_or_else(|_| SnykIssuesResponse { data: vec![] });

        Ok(result
            .data
            .into_iter()
            .map(|issue| {
                let fix = issue
                    .attributes
                    .coordinates
                    .as_ref()
                    .and_then(|coords| coords.first())
                    .and_then(|c| c.remedies.as_ref())
                    .and_then(|r| r.first())
                    .map(|r| r.description.clone());

                VulnSummary {
                    id: issue.id.clone(),
                    package: package_name.to_string(),
                    title: issue.attributes.title,
                    severity: issue.attributes.severity,
                    cwe: issue.attributes.cwe,
                    cvss_v3: issue.attributes.cvss_v3,
                    snyk_url: format!("https://security.snyk.io/vuln/{}", issue.id),
                    fix,
                }
            })
            .collect())
    }

    /// Get the authenticated user's personal org ID.
    async fn get_personal_org_id(&self) -> Result<String> {
        let url = format!("{SNYK_API_BASE}/self?version={SNYK_API_VERSION}");
        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("token {}", self.token))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(MerlinError::Platform(
                "Could not fetch Snyk personal org ID. Set snyk.org_id in merlin.toml.".to_string(),
            ));
        }

        let data: serde_json::Value = resp.json().await?;
        data["data"]["attributes"]["default_org_context"]
            .as_str()
            .map(str::to_string)
            .or_else(|| data["data"]["id"].as_str().map(str::to_string))
            .ok_or_else(|| {
                MerlinError::Platform("Could not extract Snyk org ID from /self".to_string())
            })
    }

    /// Format vuln summaries as a Markdown table.
    pub fn format_vulns_table(vulns: &[VulnSummary]) -> String {
        if vulns.is_empty() {
            return "✅ No known vulnerabilities found in Snyk database.\n".to_string();
        }
        let mut out = "| Severity | Package | Vulnerability | CWE | CVSS | Snyk |\n\
                       |----------|---------|---------------|-----|------|------|\n"
            .to_string();
        for v in vulns {
            let sev_emoji = match v.severity.to_lowercase().as_str() {
                "critical" => "🔴 Critical",
                "high" => "🟠 High",
                "medium" => "🟡 Medium",
                "low" => "🔵 Low",
                _ => "⚪ Unknown",
            };
            let cwe = v.cwe.join(", ");
            let cvss = v
                .cvss_v3
                .map(|s| format!("{s:.1}"))
                .unwrap_or_else(|| "—".to_string());
            out.push_str(&format!(
                "| {sev_emoji} | `{}` | {} | {} | {} | [{}]({}) |\n",
                v.package, v.title, cwe, cvss, v.id, v.snyk_url
            ));
        }
        out
    }
}

// ── Dependency extractor ──────────────────────────────────────────────────────

/// Package detected in diff with its ecosystem.
#[derive(Debug, Clone)]
pub struct DetectedPackage {
    pub ecosystem: &'static str,
    pub name: String,
    pub version: Option<String>,
}

/// Scan diff content for dependency file changes and extract package references.
pub fn extract_packages_from_diff(diff_content: &str) -> Vec<DetectedPackage> {
    let mut packages = Vec::new();

    for line in diff_content.lines() {
        if !line.starts_with('+') || line.starts_with("+++") {
            continue;
        }
        let content = &line[1..];

        // npm / package.json
        if let Some(pkg) = parse_npm_dependency(content) {
            packages.push(pkg);
        }
        // Python / requirements.txt
        else if let Some(pkg) = parse_pypi_dependency(content) {
            packages.push(pkg);
        }
        // Cargo.toml
        else if let Some(pkg) = parse_cargo_dependency(content) {
            packages.push(pkg);
        }
        // Go mod
        else if let Some(pkg) = parse_go_dependency(content) {
            packages.push(pkg);
        }
    }

    // Deduplicate by name
    packages.sort_by(|a, b| a.name.cmp(&b.name));
    packages.dedup_by(|a, b| a.name == b.name && a.ecosystem == b.ecosystem);
    packages
}

fn parse_npm_dependency(line: &str) -> Option<DetectedPackage> {
    // Matches: "  "lodash": "^4.17.21"," in package.json
    let re = regex::Regex::new(r#""(@?[a-z0-9_/-]+)"\s*:\s*"([~^]?[\d*x.]+)""#).ok()?;
    let caps = re.captures(line)?;
    Some(DetectedPackage {
        ecosystem: "npm",
        name: caps[1].to_string(),
        version: Some(caps[2].trim_start_matches(['^', '~', '>']).to_string()),
    })
}

fn parse_pypi_dependency(line: &str) -> Option<DetectedPackage> {
    // Matches: "requests==2.31.0" or "flask>=2.0"
    let re = regex::Regex::new(r"^([a-zA-Z0-9_-]+)\s*(?:==|>=|~=|!=|<=|>|<)\s*([\d.]+)").ok()?;
    let caps = re.captures(line.trim())?;
    Some(DetectedPackage {
        ecosystem: "pypi",
        name: caps[1].to_lowercase(),
        version: Some(caps[2].to_string()),
    })
}

fn parse_cargo_dependency(line: &str) -> Option<DetectedPackage> {
    // Matches: 'serde = { version = "1.0" }' or 'serde = "1.0"'
    let re = regex::Regex::new(r#"^([a-z0-9_-]+)\s*=\s*(?:\{[^}]*version\s*=\s*"([^"]+)"|\s*"([^"]+)")"#).ok()?;
    let caps = re.captures(line.trim())?;
    let version = caps.get(2).or_else(|| caps.get(3)).map(|m| m.as_str().to_string());
    Some(DetectedPackage {
        ecosystem: "cargo",
        name: caps[1].to_string(),
        version,
    })
}

fn parse_go_dependency(line: &str) -> Option<DetectedPackage> {
    // Matches: "require github.com/some/pkg v1.2.3"
    let re = regex::Regex::new(r"(?:require\s+)?([a-zA-Z0-9_./:-]+)\s+v([\d.]+)").ok()?;
    let caps = re.captures(line.trim())?;
    let name = caps[1].to_string();
    if name.starts_with("go ") || name == "module" {
        return None;
    }
    Some(DetectedPackage {
        ecosystem: "golang",
        name,
        version: Some(caps[2].to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_npm() {
        let line = r#"    "lodash": "^4.17.21","#;
        let pkg = parse_npm_dependency(line).unwrap();
        assert_eq!(pkg.name, "lodash");
        assert_eq!(pkg.ecosystem, "npm");
    }

    #[test]
    fn test_parse_pypi() {
        let pkg = parse_pypi_dependency("requests==2.31.0").unwrap();
        assert_eq!(pkg.name, "requests");
        assert_eq!(pkg.version, Some("2.31.0".to_string()));
    }

    #[test]
    fn test_parse_cargo() {
        let pkg = parse_cargo_dependency(r#"serde = "1.0""#).unwrap();
        assert_eq!(pkg.name, "serde");
    }

    #[test]
    fn test_format_empty() {
        let table = SnykClient::format_vulns_table(&[]);
        assert!(table.contains("No known vulnerabilities"));
    }
}
