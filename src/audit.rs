//! Audit log — records every Merlin action to a JSONL file.
//!
//! Each event is appended as a JSON line to `merlin-audit.jsonl` (configurable).
//! The log can be read by the `/dashboard` endpoint or exported for analysis.

use serde::{Deserialize, Serialize};
use std::io::Write;
use tracing::warn;

/// Type of audit event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Review,
    ToolRun,
    WebhookReceived,
    CommentPosted,
    Error,
}

/// A single audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// RFC 3339 timestamp (UTC).
    pub timestamp: String,
    pub kind: EventKind,
    /// Platform (github, gitlab, etc.)
    pub platform: Option<String>,
    /// PR/MR URL or identifier.
    pub pr_url: Option<String>,
    /// Slash command or action name (e.g. "/review", "/security").
    pub command: Option<String>,
    /// User/actor who triggered the event.
    pub actor: Option<String>,
    /// Result summary (e.g. "5 comments posted").
    pub result: Option<String>,
    /// Error message if the event failed.
    pub error: Option<String>,
}

impl AuditEvent {
    pub fn now() -> String {
        // Simple UTC timestamp without chrono
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        format_rfc3339(secs)
    }

    /// Create a new review event.
    pub fn review(pr_url: &str, comment_count: usize) -> Self {
        AuditEvent {
            timestamp: Self::now(),
            kind: EventKind::Review,
            platform: None,
            pr_url: Some(pr_url.to_string()),
            command: Some("/review".to_string()),
            actor: None,
            result: Some(format!("{comment_count} comments posted")),
            error: None,
        }
    }

    /// Create a new tool-run event.
    pub fn tool_run(command: &str, pr_url: Option<&str>, actor: Option<&str>) -> Self {
        AuditEvent {
            timestamp: Self::now(),
            kind: EventKind::ToolRun,
            platform: None,
            pr_url: pr_url.map(str::to_string),
            command: Some(command.to_string()),
            actor: actor.map(str::to_string),
            result: None,
            error: None,
        }
    }

    /// Create an error event.
    pub fn error(command: Option<&str>, err: &str) -> Self {
        AuditEvent {
            timestamp: Self::now(),
            kind: EventKind::Error,
            platform: None,
            pr_url: None,
            command: command.map(str::to_string),
            actor: None,
            result: None,
            error: Some(err.to_string()),
        }
    }
}

fn format_rfc3339(secs: u64) -> String {
    let mut remaining = secs;
    let seconds = remaining % 60; remaining /= 60;
    let minutes = remaining % 60; remaining /= 60;
    let hours = remaining % 24; remaining /= 24;

    let mut days = remaining as u32;
    let mut year = 1970u32;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if days < days_in_year { break; }
        days -= days_in_year;
        year += 1;
    }
    let month_days: [u32; 12] = [31, if is_leap(year) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 0usize;
    while month < 11 && days >= month_days[month] {
        days -= month_days[month];
        month += 1;
    }
    let day = days + 1;

    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}Z", month = month + 1)
}

fn is_leap(year: u32) -> bool {
    year.is_multiple_of(400) || (year.is_multiple_of(4) && !year.is_multiple_of(100))
}

// ── Logger ────────────────────────────────────────────────────────────────────

/// Appends audit events to a JSONL file.
pub struct AuditLogger {
    log_path: String,
    enabled: bool,
}

impl AuditLogger {
    pub fn new(log_path: String, enabled: bool) -> Self {
        Self { log_path, enabled }
    }

    pub fn from_config(cfg: &crate::config::AuditConfig) -> Self {
        Self::new(cfg.log_path.clone(), cfg.enabled)
    }

    /// Append an event to the log file.
    pub fn log(&self, mut event: AuditEvent) {
        if !self.enabled {
            return;
        }
        // Set timestamp if not already set
        if event.timestamp.is_empty() {
            event.timestamp = AuditEvent::now();
        }
        match serde_json::to_string(&event) {
            Ok(line) => {
                match std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&self.log_path)
                {
                    Ok(mut file) => {
                        if let Err(e) = writeln!(file, "{line}") {
                            warn!("Failed to write audit log: {e}");
                        }
                    }
                    Err(e) => warn!("Failed to open audit log at {}: {e}", self.log_path),
                }
            }
            Err(e) => warn!("Failed to serialize audit event: {e}"),
        }
    }

    /// Read the last N events from the log file.
    pub fn read_recent(&self, limit: usize) -> Vec<AuditEvent> {
        let content = match std::fs::read_to_string(&self.log_path) {
            Ok(c) => c,
            Err(_) => return vec![],
        };

        content
            .lines()
            .rev()
            .take(limit)
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_rfc3339() {
        // 2024-01-01T00:00:00Z
        assert_eq!(format_rfc3339(1_704_067_200), "2024-01-01T00:00:00Z");
    }

    #[test]
    fn test_audit_event_serialization() {
        let ev = AuditEvent::review("https://github.com/org/repo/pull/42", 5);
        let json = serde_json::to_string(&ev).unwrap();
        assert!(json.contains("review"));
        assert!(json.contains("5 comments posted"));
    }

    #[test]
    fn test_logger_disabled() {
        let logger = AuditLogger::new("/tmp/test_audit_disabled.jsonl".to_string(), false);
        // Should not create file when disabled
        logger.log(AuditEvent::review("https://example.com", 1));
        assert!(!std::path::Path::new("/tmp/test_audit_disabled.jsonl").exists());
    }
}
