//! /coverage — Test coverage analysis for PR-changed files.
//!
//! Parses a coverage report (LCOV, Cobertura, or JSON) and reports
//! line-level coverage for every file touched in the PR diff.
//!
//! Usage:
//!   @merlin /coverage               — uses merlin.toml coverage config
//!   @merlin /coverage lcov          — force LCOV format
//!   @merlin /coverage cobertura     — force Cobertura/JaCoCo XML format
//!   @merlin /coverage json          — force generic JSON format

use async_trait::async_trait;
use tracing::info;

use super::{MerlinTool, ToolContext};
use crate::diff::parse_diff;
use crate::error::Result;

pub struct CoverageTool;

#[async_trait]
impl MerlinTool for CoverageTool {
    fn name(&self) -> &'static str {
        "coverage"
    }

    async fn run(&self, ctx: &ToolContext) -> Result<String> {
        info!("Running /coverage");

        let cfg = crate::config::Config::load_default()
            .unwrap_or_default()
            .coverage;
        let format_override = ctx.arg.as_deref();
        let format = format_override.unwrap_or(cfg.format.as_str());
        let report_path = &cfg.report_path;

        // Read the coverage report file
        let report_content = match std::fs::read_to_string(report_path) {
            Ok(c) => c,
            Err(e) => {
                return Ok(format!(
                    "## Merlin: Coverage\n\n⚠️ Could not read coverage report at `{report_path}`: {e}\n\n\
                     Make sure to run your test suite with coverage before running `/coverage`.\n\n\
                     *[Merlin](https://github.com/you/merlin) 🦡*"
                ))
            }
        };

        // Parse the report into per-file line coverage data
        let coverage_map = match format {
            "lcov" => parse_lcov(&report_content),
            "cobertura" => parse_cobertura(&report_content),
            "json" => parse_json_coverage(&report_content),
            other => {
                return Ok(format!(
                    "## Merlin: Coverage\n\n⚠️ Unknown coverage format: `{other}`. \
                     Supported: `lcov`, `cobertura`, `json`.\n\n\
                     *[Merlin](https://github.com/you/merlin) 🦡*"
                ))
            }
        };

        // Get changed files from the PR diff
        let raw_diff = ctx.platform.get_diff().await?;
        let files = parse_diff(&raw_diff)?;
        let changed_paths: Vec<&str> = files.iter().map(|f| f.path()).collect();

        let mut out = "## Merlin: Coverage Analysis\n\n".to_string();

        if coverage_map.is_empty() {
            out.push_str("⚠️ No coverage data found in the report.\n\n");
            out.push_str("*[Merlin](https://github.com/you/merlin) 🦡*");
            return Ok(out);
        }

        // Compute overall project coverage
        let (total_covered, total_lines) = coverage_map.values().fold((0u32, 0u32), |acc, fc| {
            (acc.0 + fc.covered_lines, acc.1 + fc.total_lines)
        });
        let overall_pct = if total_lines > 0 {
            (total_covered as f32 / total_lines as f32) * 100.0
        } else {
            0.0
        };

        // Threshold check
        let threshold_line = if cfg.threshold > 0.0 {
            if overall_pct >= cfg.threshold {
                format!(
                    "✅ Coverage {:.1}% meets threshold of {:.1}%\n\n",
                    overall_pct, cfg.threshold
                )
            } else {
                format!(
                    "🔴 Coverage {:.1}% is **below threshold of {:.1}%**\n\n",
                    overall_pct, cfg.threshold
                )
            }
        } else {
            format!(
                "**Overall coverage: {overall_pct:.1}%** ({total_covered}/{total_lines} lines)\n\n"
            )
        };
        out.push_str(&threshold_line);

        // Per-file coverage for changed files
        out.push_str("### Coverage for changed files\n\n");
        out.push_str("| File | Coverage | Covered | Total | Status |\n");
        out.push_str("|------|----------|---------|-------|--------|\n");

        let mut found_any = false;
        for path in &changed_paths {
            // Try exact match and suffix match (coverage tools may use different path roots)
            let file_cov = coverage_map.iter().find(|(k, _)| {
                k.as_str() == *path || k.ends_with(path) || path.ends_with(k.as_str())
            });

            if let Some((_, fc)) = file_cov {
                found_any = true;
                let pct = if fc.total_lines > 0 {
                    (fc.covered_lines as f32 / fc.total_lines as f32) * 100.0
                } else {
                    100.0
                };
                let status = coverage_status_emoji(pct);
                out.push_str(&format!(
                    "| `{path}` | {pct:.1}% | {} | {} | {status} |\n",
                    fc.covered_lines, fc.total_lines
                ));
            } else {
                out.push_str(&format!("| `{path}` | — | — | — | ⚪ not in report |\n"));
            }
        }

        if !found_any {
            out.push_str("\n⚠️ None of the changed files appear in the coverage report. ");
            out.push_str("Check that paths in the report match the repository root.\n");
        }

        // AI-powered coverage gap analysis
        let coverage_summary = changed_paths
            .iter()
            .filter_map(|path| {
                coverage_map
                    .iter()
                    .find(|(k, _)| {
                        k.as_str() == *path || k.ends_with(path) || path.ends_with(k.as_str())
                    })
                    .map(|(_, fc)| {
                        let pct = if fc.total_lines > 0 {
                            (fc.covered_lines as f32 / fc.total_lines as f32) * 100.0
                        } else {
                            100.0
                        };
                        format!(
                            "{path}: {pct:.1}% covered ({}/{} lines)",
                            fc.covered_lines, fc.total_lines
                        )
                    })
            })
            .collect::<Vec<_>>()
            .join("\n");

        if !coverage_summary.is_empty() {
            let system = "You are a test quality expert. Given coverage data for PR-changed files, \
                          identify which files need more tests and suggest what scenarios to cover. \
                          Be concise — 2-3 bullet points per low-coverage file. \
                          Only comment on files with < 80% coverage.";
            let ai_analysis = ctx
                .ai
                .generate(system, &coverage_summary)
                .await
                .unwrap_or_default();
            if !ai_analysis.trim().is_empty() {
                out.push_str("\n### AI Coverage Recommendations\n\n");
                out.push_str(&ai_analysis);
                out.push('\n');
            }
        }

        out.push_str("\n*[Merlin](https://github.com/you/merlin) 🦡*");
        Ok(out)
    }
}

fn coverage_status_emoji(pct: f32) -> &'static str {
    match pct as u32 {
        90..=100 => "✅ Excellent",
        75..=89 => "🟡 Good",
        50..=74 => "🟠 Fair",
        _ => "🔴 Poor",
    }
}

// ── Per-file coverage data ────────────────────────────────────────────────────

#[derive(Debug, Default)]
struct FileCoverage {
    covered_lines: u32,
    total_lines: u32,
}

// ── LCOV parser ───────────────────────────────────────────────────────────────

fn parse_lcov(content: &str) -> std::collections::HashMap<String, FileCoverage> {
    let mut map: std::collections::HashMap<String, FileCoverage> = std::collections::HashMap::new();
    let mut current_file: Option<String> = None;

    for line in content.lines() {
        if let Some(path) = line.strip_prefix("SF:") {
            current_file = Some(path.trim().to_string());
        } else if let Some(da) = line.strip_prefix("DA:") {
            if let Some(ref file) = current_file {
                let mut parts = da.splitn(2, ',');
                let _line_no = parts.next();
                if let Some(hit_str) = parts.next() {
                    let hit: u32 = hit_str.trim().parse().unwrap_or(0);
                    let fc = map.entry(file.clone()).or_default();
                    fc.total_lines += 1;
                    if hit > 0 {
                        fc.covered_lines += 1;
                    }
                }
            }
        } else if line == "end_of_record" {
            current_file = None;
        }
    }
    map
}

// ── Cobertura / JaCoCo XML parser ─────────────────────────────────────────────

fn parse_cobertura(content: &str) -> std::collections::HashMap<String, FileCoverage> {
    let mut map: std::collections::HashMap<String, FileCoverage> = std::collections::HashMap::new();

    // Simple regex-based extraction (no full XML parser dep)
    let file_re = regex::Regex::new(r#"<class[^>]+filename="([^"]+)"[^>]*>"#).unwrap();
    let line_re = regex::Regex::new(r#"<line[^>]+hits="(\d+)"[^/]*/>"#).unwrap();

    let mut current_file: Option<String> = None;
    let mut file_start = 0usize;

    for m in file_re.find_iter(content) {
        // Process previous file's lines
        if let Some(ref file) = current_file {
            let fc = map.entry(file.clone()).or_default();
            for cap in line_re.captures_iter(&content[file_start..m.start()]) {
                fc.total_lines += 1;
                if cap[1].parse::<u32>().unwrap_or(0) > 0 {
                    fc.covered_lines += 1;
                }
            }
        }
        if let Some(cap) = file_re.captures(&content[m.start()..]) {
            current_file = Some(cap[1].to_string());
        }
        file_start = m.end();
    }

    // Handle last file
    if let Some(ref file) = current_file {
        let fc = map.entry(file.clone()).or_default();
        for cap in line_re.captures_iter(&content[file_start..]) {
            fc.total_lines += 1;
            if cap[1].parse::<u32>().unwrap_or(0) > 0 {
                fc.covered_lines += 1;
            }
        }
    }

    map
}

// ── JSON coverage format parser ───────────────────────────────────────────────
// Supports Istanbul/NYC JSON output: { "path/to/file.js": { "s": {...}, "b": {...} } }

fn parse_json_coverage(content: &str) -> std::collections::HashMap<String, FileCoverage> {
    let mut map: std::collections::HashMap<String, FileCoverage> = std::collections::HashMap::new();

    let v: serde_json::Value = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(_) => return map,
    };

    if let Some(obj) = v.as_object() {
        for (path, file_data) in obj {
            let mut fc = FileCoverage::default();

            // Statement coverage (Istanbul format)
            if let Some(stmts) = file_data.get("s").and_then(|s| s.as_object()) {
                for hit in stmts.values() {
                    fc.total_lines += 1;
                    if hit.as_u64().unwrap_or(0) > 0 {
                        fc.covered_lines += 1;
                    }
                }
            }

            // Line coverage (alternative format: { "path": { "lines": { "found": N, "hit": M } } })
            if fc.total_lines == 0 {
                if let Some(lines) = file_data.get("lines") {
                    let found = lines.get("found").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    let hit = lines.get("hit").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    fc.total_lines = found;
                    fc.covered_lines = hit;
                }
            }

            if fc.total_lines > 0 {
                map.insert(path.clone(), fc);
            }
        }
    }

    map
}
