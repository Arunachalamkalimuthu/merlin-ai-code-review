//! RAG (Retrieval-Augmented Generation) pipeline configuration.

use serde::{Deserialize, Serialize};

/// Which embedding backend to use for RAG.
///
/// | Value    | Needs                           | Best for               |
/// |----------|---------------------------------|------------------------|
/// | `ollama` | `ollama serve` + pulled model   | Local dev (free)       |
/// | `openai` | `OPENAI_API_KEY` env var        | CI/CD (any runner)     |
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum EmbedderType {
    /// Local Ollama instance (default — zero cloud dependency).
    #[default]
    Ollama,
    /// OpenAI Embeddings API (`text-embedding-3-small` by default).
    Openai,
}

/// Which vector store backend to use for RAG.
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum VectorStoreType {
    /// Zero-setup: JSONL flat file with brute-force cosine similarity.
    /// Best for repos up to ~5 K files.
    #[default]
    Local,
    /// Ephemeral in-memory store — resets on restart. Useful for testing.
    Memory,
    /// Qdrant REST API (self-hosted or Qdrant Cloud).
    Qdrant,
    /// ChromaDB REST API (self-hosted).
    Chroma,
    /// Pinecone cloud vector database.
    Pinecone,
}

/// RAG (Retrieval-Augmented Generation) pipeline configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RagConfig {
    /// Enable RAG augmentation during code review and agent calls.
    #[serde(default)]
    pub enabled: bool,

    /// Embedding backend: `"ollama"` (local, default) or `"openai"` (CI-friendly).
    #[serde(default)]
    pub embedder: EmbedderType,

    /// Vector store backend.
    #[serde(default)]
    pub store: VectorStoreType,

    /// Collection / namespace / index name (default: `"merlin"`).
    #[serde(default = "default_rag_collection")]
    pub collection: String,

    /// Ollama embedding model (default: `"nomic-embed-text"`).
    #[serde(default = "default_embed_model")]
    pub embed_model: String,

    /// Ollama base URL for embeddings (default: `"http://localhost:11434"`).
    #[serde(default = "default_ollama_embed_url")]
    pub ollama_base_url: String,

    /// Number of documents to retrieve per query (default: 5).
    #[serde(default = "default_rag_top_k")]
    pub top_k: usize,

    /// Minimum cosine similarity score to include a result (default: 0.70).
    #[serde(default = "default_rag_min_score")]
    pub min_score: f32,

    /// Lines per file chunk when indexing (default: 80).
    #[serde(default = "default_rag_chunk_lines")]
    pub chunk_lines: usize,

    /// File extensions to index (default: Rust, Python, TS/JS, Go, Java, Markdown).
    #[serde(default = "default_index_extensions")]
    pub index_extensions: Vec<String>,

    // ── Local store ───────────────────────────────────────────────────────────
    /// Path to the JSONL vector store file (default: `"merlin-rag.jsonl"`).
    #[serde(default = "default_local_rag_path")]
    pub local_path: String,

    // ── Qdrant ────────────────────────────────────────────────────────────────
    /// Qdrant REST API URL (default: `"http://localhost:6333"`).
    #[serde(default = "default_qdrant_url")]
    pub qdrant_url: String,
    /// Qdrant API key (optional — required for Qdrant Cloud).
    pub qdrant_api_key: Option<String>,

    // ── ChromaDB ──────────────────────────────────────────────────────────────
    /// ChromaDB REST API URL (default: `"http://localhost:8000"`).
    #[serde(default = "default_chroma_url")]
    pub chroma_url: String,

    // ── Pinecone ──────────────────────────────────────────────────────────────
    /// Pinecone API key (from `PINECONE_API_KEY` env var or set here directly).
    pub pinecone_api_key: Option<String>,
    /// Pinecone index host URL (e.g. `https://my-index-xyz.svc.us-east1.pinecone.io`).
    pub pinecone_host: Option<String>,
}

fn default_rag_collection() -> String {
    "merlin".to_string()
}
fn default_embed_model() -> String {
    "nomic-embed-text".to_string()
}
fn default_ollama_embed_url() -> String {
    "http://localhost:11434".to_string()
}
fn default_rag_top_k() -> usize {
    5
}
fn default_rag_min_score() -> f32 {
    0.70
}
fn default_rag_chunk_lines() -> usize {
    80
}
fn default_local_rag_path() -> String {
    "merlin-rag.jsonl".to_string()
}
fn default_qdrant_url() -> String {
    "http://localhost:6333".to_string()
}
fn default_chroma_url() -> String {
    "http://localhost:8000".to_string()
}
fn default_index_extensions() -> Vec<String> {
    [
        ".rs", ".py", ".ts", ".js", ".tsx", ".jsx", ".go", ".java", ".kt", ".rb", ".md", ".toml",
        ".yaml", ".yml",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

impl Default for RagConfig {
    fn default() -> Self {
        RagConfig {
            enabled: false,
            embedder: EmbedderType::default(),
            store: VectorStoreType::default(),
            collection: default_rag_collection(),
            embed_model: default_embed_model(),
            ollama_base_url: default_ollama_embed_url(),
            top_k: default_rag_top_k(),
            min_score: default_rag_min_score(),
            chunk_lines: default_rag_chunk_lines(),
            index_extensions: default_index_extensions(),
            local_path: default_local_rag_path(),
            qdrant_url: default_qdrant_url(),
            qdrant_api_key: None,
            chroma_url: default_chroma_url(),
            pinecone_api_key: None,
            pinecone_host: None,
        }
    }
}
