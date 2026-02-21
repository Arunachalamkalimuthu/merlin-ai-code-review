//! Context retrieval helpers — query the RAG pipeline and format results for AI prompts.

use tracing::debug;

use crate::error::Result;
use crate::rag::{RagPipeline, RetrievedDoc};

// ── Public API ─────────────────────────────────────────────────────────────────

/// Query the RAG pipeline and return formatted context ready for injection into
/// an AI prompt.  Returns `None` when RAG is disabled or returns no results.
pub async fn retrieve_context(
    pipeline: &RagPipeline,
    query: &str,
) -> Result<Option<String>> {
    let docs = pipeline.retrieve(query, pipeline.config.top_k).await?;
    if docs.is_empty() {
        debug!("RAG: no relevant context found for query ({} chars)", query.len());
        return Ok(None);
    }
    debug!("RAG: retrieved {} doc(s) for query ({} chars)", docs.len(), query.len());
    Ok(Some(format_rag_context(&docs)))
}

/// Format a list of retrieved documents into a Markdown block suitable for
/// injection into the beginning of an AI user prompt.
pub fn format_rag_context(docs: &[RetrievedDoc]) -> String {
    let mut out = String::from(
        "## Relevant context from the codebase (retrieved via RAG)\n\n\
         Use this context to inform your review. Do not repeat it verbatim.\n\n",
    );

    for (i, doc) in docs.iter().enumerate() {
        out.push_str(&format!(
            "### Context {} — {} (score: {:.2})\n\n```\n{}\n```\n\n",
            i + 1,
            doc.source,
            doc.score,
            doc.content.trim(),
        ));
    }

    out
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rag::RetrievedDoc;

    fn make_doc(content: &str, source: &str, score: f32) -> RetrievedDoc {
        RetrievedDoc {
            content: content.to_string(),
            source: source.to_string(),
            score,
            metadata: serde_json::json!({}),
        }
    }

    #[test]
    fn test_format_empty() {
        let out = format_rag_context(&[]);
        // Should still produce the header section, just no doc entries
        assert!(out.contains("RAG"));
    }

    #[test]
    fn test_format_single_doc() {
        let docs = vec![make_doc("fn foo() {}", "codebase", 0.92)];
        let out = format_rag_context(&docs);
        assert!(out.contains("fn foo()"));
        assert!(out.contains("codebase"));
        assert!(out.contains("0.92"));
    }

    #[test]
    fn test_format_multiple_docs() {
        let docs = vec![
            make_doc("fn auth() {}", "codebase", 0.95),
            make_doc("auth bypass risk", "review_comment", 0.80),
        ];
        let out = format_rag_context(&docs);
        assert!(out.contains("Context 1"));
        assert!(out.contains("Context 2"));
        assert!(out.contains("review_comment"));
    }
}
