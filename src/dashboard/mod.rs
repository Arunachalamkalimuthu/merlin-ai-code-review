//! Web dashboard and admin panel.
//!
//! Exposes:
//!   GET /dashboard           — HTML audit log viewer
//!   GET /dashboard/events    — JSON API for recent events
//!   GET /health              — liveness probe
//!   GET /admin               — Admin panel (rules, RBAC)
//!   GET /admin/rules         — JSON API for rules
//!   POST /admin/rules        — Create rule
//!   DELETE /admin/rules/{id} — Delete rule
//!   GET /admin/users         — JSON API for users
//!   POST /admin/users        — Add/update user
//!   GET /admin/config        — View current config (redacted)

pub mod admin;

use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
};
use std::sync::Arc;

use crate::audit::AuditLogger;
use crate::config::AuditConfig;

#[derive(Clone)]
pub struct DashboardState {
    pub logger: Arc<AuditLogger>,
    pub version: &'static str,
}

/// Build the dashboard Axum router — merge this into the main webhook router.
pub fn router(state: DashboardState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/dashboard", get(dashboard_html))
        .route("/dashboard/events", get(dashboard_events))
        .with_state(state)
}

/// Liveness probe.
async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

/// JSON API — return last 100 audit events.
async fn dashboard_events(State(state): State<DashboardState>) -> impl IntoResponse {
    let events = state.logger.read_recent(100);
    Json(events)
}

/// HTML dashboard page.
async fn dashboard_html(State(state): State<DashboardState>) -> impl IntoResponse {
    let events = state.logger.read_recent(100);

    let rows: String = events.iter().rev().fold(String::new(), |mut acc, e| {
        let kind = format!("{:?}", e.kind);
        let command = e.command.as_deref().unwrap_or("—");
        let actor = e.actor.as_deref().unwrap_or("—");
        let result = e.result.as_deref().unwrap_or("");
        let error = e.error.as_deref().unwrap_or("");
        let pr = e
            .pr_url
            .as_deref()
            .map(|u| format!("<a href=\"{u}\" target=\"_blank\">{u}</a>"))
            .unwrap_or_else(|| "—".to_string());

        let status_cell = if error.is_empty() {
            format!("<td class=\"ok\">{result}</td>")
        } else {
            format!("<td class=\"err\">❌ {error}</td>")
        };

        acc.push_str(&format!(
            "<tr><td>{ts}</td><td>{kind}</td><td>{command}</td>\
                 <td>{actor}</td><td>{pr}</td>{status_cell}</tr>",
            ts = e.timestamp,
        ));
        acc
    });

    let version = state.version;
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <title>Merlin Dashboard</title>
  <style>
    body {{ font-family: system-ui, sans-serif; margin: 2rem; background: #0d1117; color: #e6edf3; }}
    h1 {{ display: flex; align-items: center; gap: .5rem; font-size: 1.6rem; }}
    h1 span.badge {{ background: #238636; color: #fff; font-size: .75rem; padding: 2px 8px; border-radius: 12px; }}
    table {{ width: 100%; border-collapse: collapse; font-size: .85rem; margin-top: 1rem; }}
    th {{ background: #161b22; padding: .6rem 1rem; text-align: left; color: #8b949e; font-weight: 500; }}
    td {{ border-bottom: 1px solid #21262d; padding: .6rem 1rem; vertical-align: top; word-break: break-all; }}
    tr:hover td {{ background: #161b22; }}
    td.ok {{ color: #3fb950; }}
    td.err {{ color: #f85149; }}
    a {{ color: #58a6ff; }}
    .empty {{ text-align: center; padding: 3rem; color: #8b949e; }}
    footer {{ margin-top: 2rem; color: #8b949e; font-size: .8rem; }}
  </style>
</head>
<body>
  <h1>🦡 Merlin Dashboard <span class="badge">v{version}</span></h1>
  <p>Showing the last 100 events. Refresh to update.</p>

  <table>
    <thead>
      <tr>
        <th>Timestamp</th>
        <th>Type</th>
        <th>Command</th>
        <th>Actor</th>
        <th>PR</th>
        <th>Result / Error</th>
      </tr>
    </thead>
    <tbody>
      {rows_or_empty}
    </tbody>
  </table>

  <footer>
    <a href="/dashboard/events">JSON API</a> · <a href="/health">Health</a> ·
    <a href="https://github.com/you/merlin" target="_blank">GitHub</a>
  </footer>
</body>
</html>"#,
        rows_or_empty = if rows.is_empty() {
            "<tr><td colspan=\"6\" class=\"empty\">No events yet. Run <code>merlin review</code> or use a slash command.</td></tr>".to_string()
        } else {
            rows
        }
    );

    Html(html)
}

/// Create a DashboardState from the global audit config.
pub fn from_audit_config(cfg: &AuditConfig) -> DashboardState {
    DashboardState {
        logger: Arc::new(AuditLogger::from_config(cfg)),
        version: env!("CARGO_PKG_VERSION"),
    }
}

/// Build the combined dashboard + admin router.
pub fn full_router(cfg: &AuditConfig) -> Router {
    let dashboard_state = from_audit_config(cfg);
    let admin_state = admin::SharedAdminState::new("merlin-admin.json".to_string());

    router(dashboard_state).merge(admin::admin_router(admin_state))
}
