/// Tests for parse_command and route_command in src/tools/mod.rs.
use merlin::tools::{parse_command, route_command};

// ── parse_command ─────────────────────────────────────────────────────────────

#[test]
fn parse_simple_slash_command() {
    let (cmd, arg) = parse_command("/review").unwrap();
    assert_eq!(cmd, "/review");
    assert!(arg.is_none());
}

#[test]
fn parse_command_with_bot_mention() {
    let (cmd, arg) = parse_command("@merlin /ask Is this thread-safe?").unwrap();
    assert_eq!(cmd, "/ask");
    assert_eq!(arg.unwrap(), "Is this thread-safe?");
}

#[test]
fn parse_command_case_insensitive_command() {
    let (cmd, _) = parse_command("/DESCRIBE").unwrap();
    assert_eq!(cmd, "/describe");
}

#[test]
fn parse_command_case_insensitive_bot_mention() {
    let (cmd, _) = parse_command("@Merlin /Review").unwrap();
    assert_eq!(cmd, "/review");
}

#[test]
fn parse_command_with_multiword_arg() {
    let (cmd, arg) = parse_command("/ask what does this function do in detail?").unwrap();
    assert_eq!(cmd, "/ask");
    assert_eq!(arg.unwrap(), "what does this function do in detail?");
}

#[test]
fn parse_command_no_match_on_plain_text() {
    assert!(parse_command("just a regular PR comment").is_none());
    assert!(parse_command("").is_none());
    assert!(parse_command("LGTM!").is_none());
}

#[test]
fn parse_command_spec() {
    let (cmd, _) = parse_command("/spec").unwrap();
    assert_eq!(cmd, "/spec");
}

#[test]
fn parse_command_no_arg_when_none_given() {
    let (_, arg) = parse_command("@merlin /review").unwrap();
    assert!(arg.is_none());
}

// ── route_command ─────────────────────────────────────────────────────────────

#[test]
fn route_all_known_commands() {
    let commands = [
        "/review",
        "/describe",
        "/spec",
        "/ask",
        "/improve",
        "/generate_labels",
        "/update_changelog",
        "/add_doc",
        "/similar_issue",
        "/test",
        "/explain",
        "/security",
        "/approve",
        "/commit_message",
        "/docs",
        "/coverage",
        "/link_jira",
        "/link_linear",
        "/snyk",
        "/triage",
        "/fix",
    ];

    for cmd in &commands {
        assert!(
            route_command(cmd).is_ok(),
            "route_command should succeed for '{cmd}'"
        );
    }
}

#[test]
fn route_unknown_command_returns_error() {
    assert!(route_command("/nonexistent").is_err());
    assert!(route_command("").is_err());
    assert!(route_command("review").is_err()); // missing slash
}

#[test]
fn route_command_is_case_insensitive() {
    // Router lowercases the command before matching
    assert!(route_command("/REVIEW").is_ok());
    assert!(route_command("/Describe").is_ok());
    assert!(route_command("/SPEC").is_ok());
}

#[test]
fn route_spec_command() {
    assert!(route_command("/spec").is_ok());
}
