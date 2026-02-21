//! Shared AI response parsing utilities.
//!
//! Every AI backend (Anthropic, OpenAI, Gemini, Ollama, Azure OpenAI, Bedrock,
//! Claude Code) produces the same output schema — a JSON array of
//! [`ReviewComment`] objects.  This module centralises the parsing logic so
//! each provider only needs to call [`parse_review_response`].

use crate::ai::ReviewComment;
use crate::error::{MerlinError, Result};

/// Parse a raw AI text response into a [`Vec<ReviewComment>`].
///
/// Handles the following formats automatically:
///
/// - **Bare JSON array** — `[{...}, {...}]`
/// - **Markdown-fenced array** — ```` ```json\n[...]\n``` ````
/// - **Wrapped object** — `{"comments": [...]}` (also tries `"reviews"`,
///   `"issues"`, `"results"` as wrapper keys, for providers that don't
///   support `json_object` response mode)
///
/// # Errors
///
/// Returns [`MerlinError::AiProvider`] if the text cannot be interpreted
/// as a valid array of review comments.
///
/// # Examples
///
/// ```rust
/// use merlin::ai::response::parse_review_response;
///
/// let raw = r#"[{
///   "file": "src/main.rs", "line": 10,
///   "severity": "high", "category": "bug",
///   "title": "Null check missing", "body": "ptr may be null", "suggestion": null
/// }]"#;
/// let comments = parse_review_response(raw).unwrap();
/// assert_eq!(comments.len(), 1);
/// assert_eq!(comments[0].file, "src/main.rs");
/// ```
pub fn parse_review_response(text: &str) -> Result<Vec<ReviewComment>> {
    // Strip optional markdown code fences the model might include
    let cleaned = strip_markdown_fences(text);

    // Fast path: bare JSON array
    if cleaned.starts_with('[') {
        return serde_json::from_str(cleaned).map_err(|e| {
            MerlinError::AiProvider(format!(
                "Failed to parse AI response as ReviewComment array: {e}\nRaw: {cleaned}"
            ))
        });
    }

    // Slow path: JSON object with a wrapper key
    let value: serde_json::Value = serde_json::from_str(cleaned).map_err(|e| {
        MerlinError::AiProvider(format!(
            "AI response is neither a JSON array nor a JSON object: {e}\nRaw: {cleaned}"
        ))
    })?;

    for key in &["comments", "reviews", "issues", "results"] {
        if let Some(arr) = value.get(key) {
            return serde_json::from_value(arr.clone()).map_err(|e| {
                MerlinError::AiProvider(format!("Failed to deserialise wrapped '{key}' array: {e}"))
            });
        }
    }

    Err(MerlinError::AiProvider(format!(
        "Could not locate a ReviewComment array in AI response.\nRaw: {cleaned}"
    )))
}

/// Remove leading/trailing markdown code fences (` ```json ` or ` ``` `).
fn strip_markdown_fences(text: &str) -> &str {
    text.trim()
        .strip_prefix("```json")
        .or_else(|| text.trim().strip_prefix("```"))
        .map(|s| s.trim_end_matches("```").trim())
        .unwrap_or_else(|| text.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bare_array() {
        let json = r#"[{"file":"a.rs","line":1,"severity":"low","category":"style","title":"T","body":"B","suggestion":null}]"#;
        let comments = parse_review_response(json).unwrap();
        assert_eq!(comments.len(), 1);
    }

    #[test]
    fn parse_empty_array() {
        assert!(parse_review_response("[]").unwrap().is_empty());
    }

    #[test]
    fn parse_markdown_fenced() {
        let s = "```json\n[]\n```";
        assert!(parse_review_response(s).unwrap().is_empty());
    }

    #[test]
    fn parse_plain_fence() {
        let s = "```\n[]\n```";
        assert!(parse_review_response(s).unwrap().is_empty());
    }

    #[test]
    fn parse_wrapped_comments_key() {
        let json = r#"{"comments":[{"file":"b.rs","line":2,"severity":"high","category":"bug","title":"T","body":"B","suggestion":null}]}"#;
        let comments = parse_review_response(json).unwrap();
        assert_eq!(comments.len(), 1);
    }

    #[test]
    fn parse_wrapped_reviews_key() {
        let json = r#"{"reviews":[{"file":"c.rs","line":3,"severity":"medium","category":"performance","title":"T","body":"B","suggestion":null}]}"#;
        let comments = parse_review_response(json).unwrap();
        assert_eq!(comments.len(), 1);
    }

    #[test]
    fn error_on_invalid_json() {
        assert!(parse_review_response("not json").is_err());
    }
}
