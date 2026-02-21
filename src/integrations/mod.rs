//! Third-party integrations.
//!
//! - Jira — requires `JIRA_TOKEN` + `[jira]` config
//! - Linear — requires `LINEAR_API_KEY`
//! - Snyk — vulnerability database, requires `SNYK_TOKEN`
//! - CodeTriage — open-source issue triage, no auth required

pub mod codetriage;
pub mod jira;
pub mod linear;
pub mod snyk;
