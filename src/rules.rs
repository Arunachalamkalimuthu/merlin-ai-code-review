//! Custom review rules engine — `.merlin-rules.yaml` support.
//!
//! Teams define pattern-based review rules in a YAML file.  Rules can be:
//!
//! - **Natural-language directives** — injected into the AI system prompt
//!   (e.g. "All public API functions must have error handling")
//! - **Regex patterns** — matched against the diff to flag specific code patterns
//!   (e.g. `pattern: "unwrap\\(\\)"`, `message: "Avoid unwrap in production code"`)
//!
//! Rules are loaded from `.merlin-rules.yaml` (configurable via
//! `[review] rules_file` in `merlin.toml`) and combined with any inline rules
//! from `[review.persona] rules`.
//!
//! # File format
//!
//! ```yaml
//! rules:
//!   - name: no-unwrap
//!     pattern: "unwrap()"
//!     severity: medium
//!     message: "Avoid unwrap() in production code — use ? or expect() with context"
//!
//!   - name: require-error-handling
//!     directive: "All public API functions must handle errors explicitly"
//!
//!   - name: auth-review
//!     path_match: "src/auth/**"
//!     directive: "Flag any changes to authentication logic as Critical severity"
//! ```

use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{debug, info, warn};

// ── Rule types ──────────────────────────────────────────────────────────────

/// A single custom review rule.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReviewRule {
    /// Human-readable rule name (e.g. `"no-unwrap"`).
    pub name: String,

    /// Regex pattern to match against diff content.  When a match is found,
    /// the AI is told to pay special attention and `message` is surfaced.
    #[serde(default)]
    pub pattern: Option<String>,

    /// Natural-language directive injected into the AI system prompt.
    #[serde(default)]
    pub directive: Option<String>,

    /// Glob pattern to restrict this rule to specific file paths.
    /// Example: `"src/auth/**"`, `"*.rs"`, `"migrations/*"`.
    #[serde(default)]
    pub path_match: Option<String>,

    /// Severity to assign when this rule triggers (default: `"medium"`).
    #[serde(default = "default_severity")]
    pub severity: String,

    /// Human-readable message shown when a pattern rule matches.
    #[serde(default)]
    pub message: Option<String>,
}

fn default_severity() -> String {
    "medium".to_string()
}

/// Root structure of `.merlin-rules.yaml`.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct RulesFile {
    /// List of custom review rules.
    #[serde(default)]
    pub rules: Vec<ReviewRule>,
}

// ── Rules engine ────────────────────────────────────────────────────────────

/// Loaded and compiled rules engine.
#[derive(Debug)]
pub struct RulesEngine {
    rules: Vec<ReviewRule>,
    compiled_patterns: Vec<Option<regex::Regex>>,
}

impl RulesEngine {
    /// Load rules from a YAML file.  Returns an empty engine if the file doesn't exist.
    pub fn load(path: &str) -> Self {
        if !Path::new(path).exists() {
            debug!("No rules file at {path}, using empty rule set");
            return Self {
                rules: Vec::new(),
                compiled_patterns: Vec::new(),
            };
        }

        match std::fs::read_to_string(path) {
            Ok(contents) => match serde_yaml_ng::from_str::<RulesFile>(&contents) {
                Ok(file) => {
                    info!("Loaded {} custom rules from {path}", file.rules.len());
                    let compiled = file
                        .rules
                        .iter()
                        .map(|r| {
                            r.pattern.as_ref().and_then(|p| match regex::Regex::new(p) {
                                Ok(re) => Some(re),
                                Err(e) => {
                                    warn!("Invalid regex in rule '{}': {e}", r.name);
                                    None
                                }
                            })
                        })
                        .collect();
                    Self {
                        rules: file.rules,
                        compiled_patterns: compiled,
                    }
                }
                Err(e) => {
                    warn!("Failed to parse rules file {path}: {e}");
                    Self {
                        rules: Vec::new(),
                        compiled_patterns: Vec::new(),
                    }
                }
            },
            Err(e) => {
                warn!("Failed to read rules file {path}: {e}");
                Self {
                    rules: Vec::new(),
                    compiled_patterns: Vec::new(),
                }
            }
        }
    }

    /// Build a list of natural-language directives to inject into the AI system prompt.
    ///
    /// Includes both explicit `directive` rules and auto-generated instructions
    /// for `pattern` rules.
    pub fn prompt_directives(&self) -> Vec<String> {
        let mut directives = Vec::new();

        for rule in &self.rules {
            if let Some(ref directive) = rule.directive {
                let mut text = directive.clone();
                if let Some(ref path_match) = rule.path_match {
                    text = format!("[For files matching {path_match}] {text}");
                }
                directives.push(text);
            }

            if let Some(ref pattern) = rule.pattern {
                let msg = rule
                    .message
                    .as_deref()
                    .unwrap_or("This pattern violates a custom team rule");
                let mut text = format!(
                    "When you see code matching the pattern `{pattern}`, flag it as {} severity: {msg}",
                    rule.severity
                );
                if let Some(ref path_match) = rule.path_match {
                    text = format!("[For files matching {path_match}] {text}");
                }
                directives.push(text);
            }
        }

        directives
    }

    /// Check a diff hunk against all pattern rules, returning pre-match warnings
    /// that should be prepended to the AI context for that file.
    pub fn check_diff(&self, file_path: &str, diff_content: &str) -> Vec<PatternMatch> {
        let mut matches = Vec::new();

        for (i, rule) in self.rules.iter().enumerate() {
            // Check path_match filter
            if let Some(ref glob_pat) = rule.path_match {
                if !path_matches_glob(file_path, glob_pat) {
                    continue;
                }
            }

            // Check regex pattern
            if let Some(ref compiled) = self.compiled_patterns[i] {
                for mat in compiled.find_iter(diff_content) {
                    matches.push(PatternMatch {
                        rule_name: rule.name.clone(),
                        severity: rule.severity.clone(),
                        message: rule
                            .message
                            .clone()
                            .unwrap_or_else(|| format!("Matches rule '{}'", rule.name)),
                        matched_text: mat.as_str().to_string(),
                    });
                }
            }
        }

        matches
    }

    /// Number of loaded rules.
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Whether any rules are loaded.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

/// A pattern rule match found in a diff hunk.
#[derive(Debug, Clone)]
pub struct PatternMatch {
    /// Name of the rule that matched.
    pub rule_name: String,
    /// Configured severity level.
    pub severity: String,
    /// Human-readable message.
    pub message: String,
    /// The text that matched the pattern.
    pub matched_text: String,
}

impl PatternMatch {
    /// Format this match as a context hint for the AI prompt.
    pub fn as_prompt_hint(&self) -> String {
        format!(
            "**Rule `{}`** ({}): {} — matched: `{}`",
            self.rule_name, self.severity, self.message, self.matched_text
        )
    }
}

/// Format a list of pattern matches as a prompt section.
pub fn format_pattern_matches(matches: &[PatternMatch]) -> String {
    if matches.is_empty() {
        return String::new();
    }
    let mut out = String::from("\n## Custom Rule Matches\n\n");
    out.push_str(
        "The following custom team rules matched in this diff. \
                  Prioritise these in your review:\n\n",
    );
    for m in matches {
        out.push_str(&format!("- {}\n", m.as_prompt_hint()));
    }
    out.push('\n');
    out
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Simple glob-style path matching supporting `*` and `**`.
fn path_matches_glob(path: &str, glob: &str) -> bool {
    let glob_re = glob
        .replace('.', "\\.")
        .replace("**", "<<GLOBSTAR>>")
        .replace('*', "[^/]*")
        .replace("<<GLOBSTAR>>", ".*");
    regex::Regex::new(&format!("^{glob_re}$"))
        .map(|re| re.is_match(path))
        .unwrap_or(false)
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rules_yaml() {
        let yaml = r#"
rules:
  - name: no-unwrap
    pattern: "unwrap\\(\\)"
    severity: high
    message: "Avoid unwrap() in production code"
  - name: require-auth
    directive: "All new endpoints must include auth middleware"
    path_match: "src/api/**"
"#;
        let file: RulesFile = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(file.rules.len(), 2);
        assert_eq!(file.rules[0].name, "no-unwrap");
        assert!(file.rules[0].pattern.is_some());
        assert!(file.rules[1].directive.is_some());
    }

    #[test]
    fn prompt_directives_includes_both_types() {
        let yaml = r#"
rules:
  - name: no-unwrap
    pattern: "unwrap\\(\\)"
    severity: high
    message: "Avoid unwrap()"
  - name: auth-check
    directive: "Flag auth changes"
"#;
        let file: RulesFile = serde_yaml_ng::from_str(yaml).unwrap();
        let engine = RulesEngine {
            compiled_patterns: file
                .rules
                .iter()
                .map(|r| r.pattern.as_ref().and_then(|p| regex::Regex::new(p).ok()))
                .collect(),
            rules: file.rules,
        };
        let directives = engine.prompt_directives();
        assert_eq!(directives.len(), 2);
        assert!(directives[0].contains("unwrap"));
        assert!(directives[1].contains("Flag auth changes"));
    }

    #[test]
    fn check_diff_finds_pattern() {
        let yaml = r#"
rules:
  - name: no-unwrap
    pattern: "unwrap\\(\\)"
    severity: high
    message: "Don't use unwrap()"
"#;
        let file: RulesFile = serde_yaml_ng::from_str(yaml).unwrap();
        let engine = RulesEngine {
            compiled_patterns: file
                .rules
                .iter()
                .map(|r| r.pattern.as_ref().and_then(|p| regex::Regex::new(p).ok()))
                .collect(),
            rules: file.rules,
        };

        let matches = engine.check_diff("src/main.rs", "+    let x = foo.unwrap();");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].rule_name, "no-unwrap");
    }

    #[test]
    fn path_match_filters_correctly() {
        let yaml = r#"
rules:
  - name: auth-only
    pattern: "password"
    severity: critical
    message: "Password handling"
    path_match: "src/auth/**"
"#;
        let file: RulesFile = serde_yaml_ng::from_str(yaml).unwrap();
        let engine = RulesEngine {
            compiled_patterns: file
                .rules
                .iter()
                .map(|r| r.pattern.as_ref().and_then(|p| regex::Regex::new(p).ok()))
                .collect(),
            rules: file.rules,
        };

        // Matches in auth path
        let m = engine.check_diff("src/auth/login.rs", "let password = input;");
        assert_eq!(m.len(), 1);

        // Doesn't match outside auth path
        let m = engine.check_diff("src/main.rs", "let password = input;");
        assert!(m.is_empty());
    }

    #[test]
    fn glob_matching_works() {
        assert!(path_matches_glob("src/auth/login.rs", "src/auth/**"));
        assert!(path_matches_glob("src/auth/deep/nested.rs", "src/auth/**"));
        assert!(!path_matches_glob("src/main.rs", "src/auth/**"));
        assert!(path_matches_glob("test.rs", "*.rs"));
        assert!(!path_matches_glob("test.py", "*.rs"));
    }
}
