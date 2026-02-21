//! /security — Dedicated security scan: vulnerabilities, secrets, OWASP issues.

use async_trait::async_trait;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::info;

use super::{MerlinTool, ToolContext};
use crate::ai::ReviewComment;
use crate::diff::{parse_diff, LineKind};
use crate::error::Result;

pub struct SecurityTool;

/// A detected secret or credential in the diff.
#[derive(Debug, Serialize)]
pub struct DetectedSecret {
    pub file: String,
    pub line: u32,
    pub kind: String,
    pub snippet: String,
}

#[derive(Deserialize)]
struct AiSecurityFinding {
    file: String,
    line: u32,
    severity: String,
    issue: String,
    description: String,
    cwe: Option<String>,
    remediation: String,
}

#[async_trait]
impl MerlinTool for SecurityTool {
    fn name(&self) -> &'static str {
        "security"
    }

    async fn run(&self, ctx: &ToolContext) -> Result<String> {
        info!("Running /security");

        let raw_diff = ctx.platform.get_diff().await?;
        let files = parse_diff(&raw_diff)?;

        // 1. Static secret detection (regex-based, instant)
        let secrets = scan_for_secrets(&raw_diff, &files);

        // 2. AI-powered OWASP / vulnerability detection
        let ai_findings = run_ai_security_scan(ctx, &files).await;

        // 3. Build report
        let mut out = "## Merlin: Security Scan\n\n".to_string();

        // Secret findings
        if secrets.is_empty() {
            out.push_str("✅ No hardcoded secrets detected.\n\n");
        } else {
            out.push_str(&format!(
                "🔴 **{} potential secret(s) detected** — rotate these immediately!\n\n",
                secrets.len()
            ));
            out.push_str("| File | Line | Type | Snippet |\n");
            out.push_str("|------|------|------|---------|\n");
            for s in &secrets {
                out.push_str(&format!(
                    "| `{}` | {} | {} | `{}` |\n",
                    s.file, s.line, s.kind, s.snippet
                ));
            }
            out.push('\n');

            // Post inline comments for secrets
            for s in &secrets {
                let comment = ReviewComment {
                    file: s.file.clone(),
                    line: s.line,
                    severity: crate::ai::Severity::Critical,
                    category: crate::ai::Category::Security,
                    title: format!("Potential {} detected", s.kind),
                    body: format!(
                        "This line may contain a hardcoded secret (`{}`). \
                         Remove it immediately, rotate the credential, and use environment \
                         variables or a secrets manager instead.",
                        s.snippet
                    ),
                    suggestion: Some(format!(
                        "Replace with: std::env::var(\"{}_SECRET\")\n\
                         or use a secrets manager (Vault, AWS Secrets Manager, etc.)",
                        s.kind.to_uppercase().replace(' ', "_")
                    )),
                };
                let _ = ctx.platform.post_inline_comment(&comment).await;
            }
        }

        // AI vulnerability findings
        match ai_findings {
            Ok(findings) if !findings.is_empty() => {
                out.push_str(&format!(
                    "### AI Vulnerability Analysis ({} finding(s))\n\n",
                    findings.len()
                ));
                out.push_str("| Severity | File | Line | Issue | CWE |\n");
                out.push_str("|----------|------|------|-------|-----|\n");
                for f in &findings {
                    let emoji = severity_emoji_str(&f.severity);
                    out.push_str(&format!(
                        "| {emoji} {} | `{}` | {} | {} | {} |\n",
                        f.severity, f.file, f.line, f.issue,
                        f.cwe.as_deref().unwrap_or("-")
                    ));
                }
                out.push('\n');

                out.push_str("### Remediations\n\n");
                for f in &findings {
                    out.push_str(&format!(
                        "**[{}/{}:{}]** {}\n\n{}\n\n> **Fix:** {}\n\n",
                        f.severity, f.file, f.line, f.issue, f.description, f.remediation
                    ));
                }
            }
            Ok(_) => {
                out.push_str("✅ No OWASP/vulnerability issues detected by AI analysis.\n\n");
            }
            Err(e) => {
                out.push_str(&format!("⚠️ AI security scan failed: {e}\n\n"));
            }
        }

        out.push_str("*Scanned by [Merlin](https://github.com/you/merlin) 🦡*");
        Ok(out)
    }
}

// ── Static secret scanning ────────────────────────────────────────────────────

/// Regex patterns for common secrets.
fn secret_patterns() -> Vec<(&'static str, Regex)> {
    let patterns = [
        ("AWS Access Key",       r"AKIA[0-9A-Z]{16}"),
        ("AWS Secret Key",       r#"(?i)aws[_\-\s]?secret[_\-\s]?(?:access[_\-\s]?)?key\s*[=:]\s*['\"]?([A-Za-z0-9/+=]{40})"#),
        ("GitHub Token",         r"ghp_[A-Za-z0-9]{36}|github_pat_[A-Za-z0-9_]{82}"),
        ("Slack Token",          r"xox[baprs]-[A-Za-z0-9\-]+"),
        ("Generic API Key",      r#"(?i)api[_\-]?key\s*[=:]\s*['\"]?([A-Za-z0-9\-_]{20,})"#),
        ("Generic Secret",       r#"(?i)(?:password|passwd|secret|token|credential)\s*[=:]\s*['\"]([^'\"]{8,})['\"]"#),
        ("Private Key PEM",      r"-----BEGIN (?:RSA |EC |DSA )?PRIVATE KEY-----"),
        ("Stripe Key",           r"(?:sk|pk)_(?:live|test)_[A-Za-z0-9]{24,}"),
        ("Anthropic API Key",    r"sk-ant-[A-Za-z0-9\-_]{32,}"),
        ("OpenAI API Key",       r"sk-[A-Za-z0-9]{32,}"),
    ];
    patterns.iter()
        .filter_map(|(name, pat)| Regex::new(pat).ok().map(|r| (*name, r)))
        .collect()
}

fn scan_for_secrets(
    _raw_diff: &str,
    files: &[crate::diff::FileDiff],
) -> Vec<DetectedSecret> {
    let patterns = secret_patterns();
    let mut found = Vec::new();

    for file in files {
        for hunk in &file.hunks {
            for hunk_line in &hunk.lines {
                if hunk_line.kind != LineKind::Added {
                    continue;
                }
                for (kind, re) in &patterns {
                    if re.is_match(&hunk_line.content) {
                        let snippet: String = hunk_line.content.chars().take(40).collect();
                        found.push(DetectedSecret {
                            file: file.path().to_string(),
                            line: hunk_line.new_line.unwrap_or(0),
                            kind: kind.to_string(),
                            snippet,
                        });
                        break; // one finding per line
                    }
                }
            }
        }
    }

    found
}

// ── AI security scan ──────────────────────────────────────────────────────────

async fn run_ai_security_scan(
    ctx: &ToolContext,
    files: &[crate::diff::FileDiff],
) -> Result<Vec<AiSecurityFinding>> {
    let system = "You are a senior application security engineer. \
                  Analyze the diff for security vulnerabilities including:\n\
                  - OWASP Top 10 (injection, XSS, CSRF, auth issues, etc.)\n\
                  - Business logic flaws\n\
                  - Insecure deserialization\n\
                  - Improper error handling exposing sensitive data\n\
                  - Race conditions / TOCTOU\n\
                  - Dependency and supply chain risks\n\n\
                  Respond ONLY with a JSON array (no markdown fences):\n\
                  [{\"file\":\"path\",\"line\":10,\"severity\":\"critical|high|medium|low\",\
                  \"issue\":\"SQL Injection\",\"description\":\"...\",\
                  \"cwe\":\"CWE-89\",\"remediation\":\"Use parameterized queries\"}]\n\
                  Return [] if no security issues found.";

    let diff_text = files
        .iter()
        .map(|f| crate::digest::compress_diff(f, 80))
        .collect::<Vec<_>>()
        .join("\n\n");

    let raw = ctx.ai.generate(system, &diff_text).await?;
    let cleaned = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    Ok(serde_json::from_str(cleaned).unwrap_or_default())
}

fn severity_emoji_str(sev: &str) -> &'static str {
    match sev.to_lowercase().as_str() {
        "critical" => "🔴",
        "high" => "🟠",
        "medium" => "🟡",
        "low" => "🔵",
        _ => "⚪",
    }
}
