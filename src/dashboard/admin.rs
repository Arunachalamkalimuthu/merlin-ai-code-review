//! Admin dashboard — Rules system, RBAC configuration, and settings management.
//!
//! Routes:
//!   GET  /admin                    — admin dashboard HTML
//!   GET  /admin/rules              — list custom review rules (JSON)
//!   POST /admin/rules              — create/update a rule
//!   DELETE /admin/rules/{id}       — delete a rule
//!   GET  /admin/users              — list RBAC users/roles
//!   POST /admin/users              — add/update a user
//!   GET  /admin/config             — show current merlin.toml (redacted)

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{delete, get},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use tracing::info;

// ── RBAC ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Admin,
    Reviewer,
    Viewer,
}

impl Role {
    pub fn can_write(&self) -> bool {
        matches!(self, Role::Admin | Role::Reviewer)
    }
    pub fn can_admin(&self) -> bool {
        matches!(self, Role::Admin)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// GitHub/GitLab username or email.
    pub identity: String,
    pub role: Role,
    /// When this user was added (RFC 3339).
    pub added_at: String,
}

// ── Custom review rules ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewRule {
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// The rule instruction injected into the AI system prompt.
    pub instruction: String,
    /// Which severity to flag when this rule is violated.
    pub severity: String,
    /// Optional: only apply to files matching this glob pattern.
    pub file_pattern: Option<String>,
    pub enabled: bool,
    pub created_at: String,
}

// ── Persisted admin state ─────────────────────────────────────────────────────

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AdminState {
    pub rules: Vec<ReviewRule>,
    pub users: Vec<User>,
}

impl AdminState {
    pub fn load(path: &str) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &str) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }
}

// ── Shared state ──────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct SharedAdminState {
    pub inner: Arc<RwLock<AdminState>>,
    pub persist_path: String,
}

impl SharedAdminState {
    pub fn new(persist_path: String) -> Self {
        let inner = AdminState::load(&persist_path);
        Self {
            inner: Arc::new(RwLock::new(inner)),
            persist_path,
        }
    }

    fn save(&self) {
        if let Ok(state) = self.inner.read() {
            state.save(&self.persist_path);
        }
    }
}

// ── Request/response types ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateRuleRequest {
    pub name: String,
    pub instruction: String,
    pub severity: Option<String>,
    pub file_pattern: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub identity: String,
    pub role: Role,
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn admin_router(state: SharedAdminState) -> Router {
    Router::new()
        .route("/admin", get(admin_html))
        .route("/admin/rules", get(list_rules).post(create_rule))
        .route("/admin/rules/:id", delete(delete_rule))
        .route("/admin/users", get(list_users).post(create_user))
        .route("/admin/config", get(show_config))
        .with_state(state)
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn list_rules(State(state): State<SharedAdminState>) -> impl IntoResponse {
    let s = state.inner.read().unwrap();
    Json(s.rules.clone())
}

async fn create_rule(
    State(state): State<SharedAdminState>,
    Json(req): Json<CreateRuleRequest>,
) -> impl IntoResponse {
    let id = format!("rule-{}", uuid_simple());
    let now = crate::audit::AuditEvent::now();

    let rule = ReviewRule {
        id: id.clone(),
        name: req.name,
        instruction: req.instruction,
        severity: req.severity.unwrap_or_else(|| "medium".to_string()),
        file_pattern: req.file_pattern,
        enabled: true,
        created_at: now,
    };

    {
        let mut s = state.inner.write().unwrap();
        s.rules.push(rule.clone());
    }
    state.save();
    info!("Created review rule: {id}");
    (StatusCode::CREATED, Json(rule))
}

async fn delete_rule(
    State(state): State<SharedAdminState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let removed = {
        let mut s = state.inner.write().unwrap();
        let before = s.rules.len();
        s.rules.retain(|r| r.id != id);
        s.rules.len() < before
    };
    if removed {
        state.save();
        info!("Deleted review rule: {id}");
        (StatusCode::OK, Json(serde_json::json!({"deleted": id})))
    } else {
        (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "not found"})))
    }
}

async fn list_users(State(state): State<SharedAdminState>) -> impl IntoResponse {
    let s = state.inner.read().unwrap();
    Json(s.users.clone())
}

async fn create_user(
    State(state): State<SharedAdminState>,
    Json(req): Json<CreateUserRequest>,
) -> impl IntoResponse {
    let now = crate::audit::AuditEvent::now();
    let user = User { identity: req.identity.clone(), role: req.role, added_at: now };

    {
        let mut s = state.inner.write().unwrap();
        // Upsert: replace if identity already exists
        s.users.retain(|u| u.identity != req.identity);
        s.users.push(user.clone());
    }
    state.save();
    info!("Upserted user: {}", req.identity);
    (StatusCode::OK, Json(user))
}

async fn show_config() -> impl IntoResponse {
    // Load and redact sensitive fields
    let raw = std::fs::read_to_string("merlin.toml").unwrap_or_else(|_| {
        "# merlin.toml not found in current directory".to_string()
    });
    // Redact any lines containing token/key/password/secret
    let redacted = raw
        .lines()
        .map(|line| {
            let lower = line.to_lowercase();
            if lower.contains("token")
                || lower.contains("key")
                || lower.contains("secret")
                || lower.contains("password")
            {
                let eq_pos = line.find('=').map(|i| i + 1).unwrap_or(line.len());
                format!("{} <redacted>", &line[..eq_pos])
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    Json(serde_json::json!({ "config": redacted }))
}

async fn admin_html(State(state): State<SharedAdminState>) -> impl IntoResponse {
    let (rules, users) = {
        let s = state.inner.read().unwrap();
        (s.rules.clone(), s.users.clone())
    };

    let rules_rows = rules
        .iter()
        .map(|r| {
            let enabled = if r.enabled { "✅" } else { "❌" };
            let pattern = r.file_pattern.as_deref().unwrap_or("*");
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>\
                 <td><button onclick=\"deleteRule('{id}')\" class=\"del\">🗑</button></td></tr>",
                r.name, r.instruction, r.severity, pattern, enabled,
                id = r.id
            )
        })
        .collect::<String>();

    let users_rows = users
        .iter()
        .map(|u| {
            format!(
                "<tr><td>{}</td><td>{:?}</td><td>{}</td></tr>",
                u.identity, u.role, u.added_at
            )
        })
        .collect::<String>();

    Html(format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <title>Merlin Admin</title>
  <style>
    body {{ font-family: system-ui, sans-serif; margin: 0; background: #0d1117; color: #e6edf3; }}
    nav {{ background: #161b22; padding: 1rem 2rem; display: flex; gap: 2rem; align-items: center; border-bottom: 1px solid #21262d; }}
    nav a {{ color: #58a6ff; text-decoration: none; font-weight: 500; }}
    nav span {{ color: #e6edf3; font-size: 1.2rem; font-weight: 700; }}
    .page {{ padding: 2rem; max-width: 1100px; margin: auto; }}
    h2 {{ color: #f0f6fc; border-bottom: 1px solid #21262d; padding-bottom: .5rem; }}
    table {{ width: 100%; border-collapse: collapse; font-size: .85rem; margin: 1rem 0 2rem; }}
    th {{ background: #161b22; padding: .6rem 1rem; text-align: left; color: #8b949e; }}
    td {{ border-bottom: 1px solid #21262d; padding: .6rem 1rem; }}
    tr:hover td {{ background: #161b22; }}
    .card {{ background: #161b22; border: 1px solid #21262d; border-radius: 8px; padding: 1.5rem; margin-bottom: 2rem; }}
    label {{ display: block; margin-bottom: .25rem; font-size: .85rem; color: #8b949e; }}
    input, select, textarea {{ width: 100%; box-sizing: border-box; padding: .5rem .75rem; border: 1px solid #30363d;
      border-radius: 6px; background: #0d1117; color: #e6edf3; font-size: .9rem; margin-bottom: 1rem; }}
    textarea {{ height: 80px; resize: vertical; }}
    button {{ background: #238636; color: #fff; border: none; padding: .5rem 1.25rem; border-radius: 6px;
      cursor: pointer; font-size: .9rem; }}
    button:hover {{ background: #2ea043; }}
    button.del {{ background: #da3633; padding: .3rem .7rem; }}
    button.del:hover {{ background: #b62324; }}
    .grid {{ display: grid; grid-template-columns: 1fr 1fr; gap: 2rem; }}
    @media (max-width: 700px) {{ .grid {{ grid-template-columns: 1fr; }} }}
  </style>
</head>
<body>
<nav>
  <span>🦡 Merlin Admin</span>
  <a href="/dashboard">Dashboard</a>
  <a href="/admin">Rules & RBAC</a>
  <a href="/admin/config">Config</a>
  <a href="/health">Health</a>
</nav>
<div class="page">
  <div class="grid">
    <!-- Rules panel -->
    <div>
      <h2>Review Rules</h2>
      <div class="card">
        <label>Rule Name</label>
        <input id="rName" placeholder="e.g. No SQL without parameterization" />
        <label>Instruction (injected into AI prompt)</label>
        <textarea id="rInstr" placeholder="Flag any SQL query that is not parameterized as a critical security issue."></textarea>
        <label>Severity</label>
        <select id="rSev"><option>critical</option><option>high</option><option selected>medium</option><option>low</option></select>
        <label>File Pattern (optional, e.g. **/*.py)</label>
        <input id="rPat" placeholder="*" />
        <button onclick="createRule()">Add Rule</button>
      </div>
      <table>
        <thead><tr><th>Name</th><th>Instruction</th><th>Severity</th><th>Files</th><th>On</th><th></th></tr></thead>
        <tbody id="rulesBody">{rules_rows}</tbody>
      </table>
    </div>

    <!-- RBAC panel -->
    <div>
      <h2>Users & Roles</h2>
      <div class="card">
        <label>GitHub / GitLab username or email</label>
        <input id="uIdent" placeholder="octocat or admin@example.com" />
        <label>Role</label>
        <select id="uRole">
          <option value="admin">Admin — full access, can configure rules</option>
          <option value="reviewer">Reviewer — can run all slash commands</option>
          <option value="viewer" selected>Viewer — read-only dashboard</option>
        </select>
        <button onclick="addUser()">Add / Update User</button>
      </div>
      <table>
        <thead><tr><th>Identity</th><th>Role</th><th>Added</th></tr></thead>
        <tbody id="usersBody">{users_rows}</tbody>
      </table>
    </div>
  </div>
</div>

<script>
async function createRule() {{
  const body = {{
    name: document.getElementById('rName').value,
    instruction: document.getElementById('rInstr').value,
    severity: document.getElementById('rSev').value,
    file_pattern: document.getElementById('rPat').value || null,
  }};
  const r = await fetch('/admin/rules', {{method:'POST', headers:{{'Content-Type':'application/json'}}, body: JSON.stringify(body)}});
  if (r.ok) location.reload();
  else alert('Failed: ' + await r.text());
}}

async function deleteRule(id) {{
  if (!confirm('Delete this rule?')) return;
  const r = await fetch('/admin/rules/' + id, {{method:'DELETE'}});
  if (r.ok) location.reload();
  else alert('Failed: ' + await r.text());
}}

async function addUser() {{
  const body = {{
    identity: document.getElementById('uIdent').value,
    role: document.getElementById('uRole').value,
  }};
  const r = await fetch('/admin/users', {{method:'POST', headers:{{'Content-Type':'application/json'}}, body: JSON.stringify(body)}});
  if (r.ok) location.reload();
  else alert('Failed: ' + await r.text());
}}
</script>
</body>
</html>"#))
}

/// Generate a simple unique ID (timestamp-based, no UUID dep).
fn uuid_simple() -> String {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{t:x}")
}
