//! Codebase indexer — walks source files and produces `Document` chunks for RAG.

use std::path::{Path, PathBuf};

use tracing::{debug, info, warn};

use super::{Document, RagPipeline};
use crate::config::RagConfig;
use crate::error::Result;

const CHUNK_OVERLAP: usize = 10; // lines of overlap between adjacent chunks

// ── Public API ─────────────────────────────────────────────────────────────────

/// Walk `root`, chunk each matching file, embed them, and upsert into the store.
/// Returns the total number of document chunks indexed.
pub async fn index_directory(
    pipeline: &RagPipeline,
    root: &Path,
    config: &RagConfig,
) -> Result<usize> {
    let files = collect_files(root, &config.index_extensions);
    info!("Indexer: found {} files under {:?}", files.len(), root);

    let mut docs: Vec<Document> = Vec::new();
    for path in &files {
        match chunk_file(path, config.chunk_lines) {
            Ok(file_docs) => docs.extend(file_docs),
            Err(e) => warn!("Indexer: skipping {:?}: {e}", path),
        }
    }

    let total = docs.len();
    info!("Indexer: embedding {} chunks…", total);
    pipeline.index_documents(docs).await?;
    info!("Indexer: done — {total} chunks indexed");
    Ok(total)
}

/// Build a `Document` from a past AI review comment (for future retrieval).
pub fn comment_to_doc(
    pr_number: u64,
    file: &str,
    line: u32,
    comment_body: &str,
    severity: &str,
) -> Document {
    Document {
        id: format!("comment:PR#{pr_number}:{file}:{line}"),
        content: format!("[{severity}] {file}:{line}\n{comment_body}"),
        source: "review_comment".to_string(),
        metadata: serde_json::json!({
            "pr_number": pr_number,
            "file":      file,
            "line":      line,
            "severity":  severity,
        }),
    }
}

// ── File chunking ──────────────────────────────────────────────────────────────

/// Split a source file into overlapping line-based chunks.
pub fn chunk_file(path: &Path, chunk_lines: usize) -> Result<Vec<Document>> {
    let content = std::fs::read_to_string(path)?;
    let path_str = path.to_string_lossy();
    let lines: Vec<&str> = content.lines().collect();

    if lines.is_empty() {
        return Ok(vec![]);
    }

    let step = chunk_lines.saturating_sub(CHUNK_OVERLAP).max(1);
    let mut docs = Vec::new();
    let mut start = 0usize;
    let mut idx = 0usize;

    loop {
        let end = (start + chunk_lines).min(lines.len());
        let chunk = lines[start..end].join("\n");

        docs.push(Document {
            id: format!("file:{path_str}:{idx}"),
            content: format!(
                "// Source: {path_str} (lines {}-{})\n{chunk}",
                start + 1,
                end
            ),
            source: "codebase".to_string(),
            metadata: serde_json::json!({
                "file":       path_str,
                "start_line": start + 1,
                "end_line":   end,
                "chunk":      idx,
            }),
        });

        idx += 1;
        if end == lines.len() {
            break;
        }
        start += step;
    }

    debug!("Indexer: {:?} → {} chunk(s)", path, docs.len());
    Ok(docs)
}

// ── File collection ────────────────────────────────────────────────────────────

fn collect_files(root: &Path, extensions: &[String]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_recursive(root, extensions, &mut out);
    out.sort();
    out
}

fn collect_recursive(dir: &Path, extensions: &[String], out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            // Skip hidden dirs and well-known noise directories
            if name.starts_with('.')
                || matches!(
                    name,
                    "target" | "node_modules" | "vendor" | "__pycache__" | "dist" | "build"
                )
            {
                continue;
            }
        }
        if path.is_dir() {
            collect_recursive(&path, extensions, out);
        } else if matches_extension(&path, extensions) {
            out.push(path);
        }
    }
}

fn matches_extension(path: &Path, extensions: &[String]) -> bool {
    let ext = match path.extension().and_then(|e| e.to_str()) {
        Some(e) => e,
        None => {
            // Index well-known extensionless files
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            return matches!(name, "README" | "Makefile" | "Dockerfile");
        }
    };
    let with_dot = format!(".{ext}");
    extensions.iter().any(|e| e == &with_dot || e == ext)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_file_small() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("hello.rs");
        std::fs::write(&f, "fn main() {\n    println!(\"hi\");\n}\n").unwrap();
        let docs = chunk_file(&f, 100).unwrap();
        assert_eq!(docs.len(), 1);
        assert!(docs[0].content.contains("fn main"));
        assert_eq!(docs[0].source, "codebase");
    }

    #[test]
    fn test_chunk_file_multiple_chunks() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("big.rs");
        let content: String = (0..200).map(|i| format!("// line {i}\n")).collect();
        std::fs::write(&f, content).unwrap();
        let docs = chunk_file(&f, 50).unwrap();
        assert!(docs.len() > 1, "Expected multiple chunks");
    }

    #[test]
    fn test_comment_to_doc() {
        let doc = comment_to_doc(42, "src/auth.rs", 15, "SQL injection risk", "critical");
        assert_eq!(doc.source, "review_comment");
        assert!(doc.id.contains("PR#42"));
        assert!(doc.content.contains("SQL injection"));
        assert!(doc.content.contains("[critical]"));
    }

    #[test]
    fn test_matches_extension() {
        let exts = vec![".rs".to_string(), ".md".to_string()];
        assert!(matches_extension(Path::new("src/main.rs"), &exts));
        assert!(matches_extension(Path::new("README.md"), &exts));
        assert!(!matches_extension(Path::new("data.json"), &exts));
    }

    #[test]
    fn test_collect_skips_target() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target");
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("main.rs"), "fn main() {}").unwrap();
        std::fs::write(dir.path().join("lib.rs"), "// lib").unwrap();

        let files =
            collect_files(dir.path(), &[".rs".to_string()]);
        assert!(!files.iter().any(|p| p.to_string_lossy().contains("target")));
        assert!(files.iter().any(|p| p.to_string_lossy().ends_with("lib.rs")));
    }
}
