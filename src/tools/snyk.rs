//! /snyk — Scan PR dependencies against the Snyk vulnerability database.

use async_trait::async_trait;
use tracing::info;

use super::{MerlinTool, ToolContext};
use crate::error::Result;
use crate::integrations::snyk::{extract_packages_from_diff, SnykClient};

pub struct SnykTool;

#[async_trait]
impl MerlinTool for SnykTool {
    fn name(&self) -> &'static str {
        "snyk"
    }

    async fn run(&self, ctx: &ToolContext) -> Result<String> {
        info!("Running /snyk");

        let snyk_cfg = crate::config::Config::load_default()
            .unwrap_or_default()
            .snyk;

        let snyk = match SnykClient::from_env(&snyk_cfg) {
            Ok(c) => c,
            Err(_) => {
                return Ok("## Merlin: Snyk Vulnerability Scan\n\n\
                           ⚠️ `SNYK_TOKEN` not set. Get a free API token at \
                           [snyk.io](https://app.snyk.io/account) and set it as `SNYK_TOKEN`.\n\n\
                           *[Merlin](https://github.com/you/merlin) 🦡*"
                    .to_string())
            }
        };

        let raw_diff = ctx.platform.get_diff().await?;

        // 1. Extract package changes from the diff
        let packages = extract_packages_from_diff(&raw_diff);

        let mut out = "## Merlin: Snyk Vulnerability Scan\n\n".to_string();

        if packages.is_empty() {
            out.push_str("No dependency file changes detected in this PR.\n\n");
            out.push_str("*[Merlin](https://github.com/you/merlin) 🦡*");
            return Ok(out);
        }

        out.push_str(&format!(
            "Scanning **{}** changed package(s) against [Snyk vulnerability database](https://security.snyk.io/)...\n\n",
            packages.len()
        ));

        // 2. Query Snyk for each package (cap at 20 to avoid rate limits)
        let mut all_vulns = Vec::new();
        for pkg in packages.iter().take(20) {
            let vulns = snyk
                .search_package_vulns(pkg.ecosystem, &pkg.name, pkg.version.as_deref())
                .await
                .unwrap_or_default();
            all_vulns.extend(vulns);
        }

        // Sort by severity (critical first)
        all_vulns.sort_by(|a, b| severity_order(&a.severity).cmp(&severity_order(&b.severity)));

        // 3. Format results
        if all_vulns.is_empty() {
            out.push_str("✅ No known vulnerabilities found for the changed dependencies.\n\n");
        } else {
            let critical_count = all_vulns
                .iter()
                .filter(|v| v.severity == "critical")
                .count();
            let high_count = all_vulns.iter().filter(|v| v.severity == "high").count();

            if critical_count > 0 || high_count > 0 {
                out.push_str(&format!(
                    "🔴 **{} critical / {} high severity vulnerabilities found!**\n\n",
                    critical_count, high_count
                ));
            }

            out.push_str(&SnykClient::format_vulns_table(&all_vulns));
            out.push('\n');

            // Post inline comments for critical/high vulns
            for vuln in all_vulns
                .iter()
                .filter(|v| v.severity == "critical" || v.severity == "high")
            {
                let comment = crate::ai::ReviewComment {
                    file: "package dependencies".to_string(),
                    line: 1,
                    severity: crate::ai::Severity::Critical,
                    category: crate::ai::Category::Security,
                    title: format!("Snyk: {} in {}", vuln.title, vuln.package),
                    body: format!(
                        "**[{}]({})** — {} severity vulnerability in `{}`.\n\
                         CWE: {}\n{}",
                        vuln.id,
                        vuln.snyk_url,
                        vuln.severity,
                        vuln.package,
                        vuln.cwe.join(", "),
                        vuln.fix
                            .as_deref()
                            .map(|f| format!("\n**Fix:** {f}"))
                            .unwrap_or_default()
                    ),
                    suggestion: vuln.fix.clone(),
                };
                let _ = ctx.platform.post_inline_comment(&comment).await;
            }
        }

        out.push_str("*Powered by [Snyk](https://security.snyk.io/) · [Merlin](https://github.com/you/merlin) 🦡*");
        Ok(out)
    }
}

fn severity_order(sev: &str) -> u8 {
    match sev {
        "critical" => 0,
        "high" => 1,
        "medium" => 2,
        "low" => 3,
        _ => 4,
    }
}
