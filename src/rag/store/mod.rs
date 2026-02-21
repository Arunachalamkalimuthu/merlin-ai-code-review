//! Vector store backends.
//!
//! Each backend implements `VectorStore` from the parent module.
//!
//! | Module    | Backend                           |
//! |-----------|-----------------------------------|
//! | `local`   | JSONL flat file + cosine search   |
//! | `memory`  | In-memory Vec (ephemeral)         |
//! | `qdrant`  | Qdrant REST API                   |
//! | `chroma`  | ChromaDB REST API                 |
//! | `pinecone`| Pinecone REST API                 |

pub mod chroma;
pub mod local;
pub mod memory;
pub mod pinecone;
pub mod qdrant;

// ── Shared cosine similarity helper ───────────────────────────────────────────

/// Compute cosine similarity between two equal-length vectors.
/// Returns a value in `[-1.0, 1.0]`; 1.0 = identical direction.
pub(crate) fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len(), "Vector dimension mismatch");
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 {
        0.0
    } else {
        dot / (mag_a * mag_b)
    }
}

/// Hash a document ID string to a stable `u64` (for Qdrant integer point IDs).
pub(crate) fn doc_id_to_u64(id: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    id.hash(&mut h);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_identical() {
        let v = vec![1.0f32, 0.0, 0.0];
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_orthogonal() {
        let a = vec![1.0f32, 0.0];
        let b = vec![0.0f32, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-6);
    }

    #[test]
    fn test_cosine_zero_vector() {
        let a = vec![0.0f32, 0.0];
        let b = vec![1.0f32, 0.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_doc_id_stable() {
        let id1 = doc_id_to_u64("file:src/main.rs:0");
        let id2 = doc_id_to_u64("file:src/main.rs:0");
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_doc_id_distinct() {
        let id1 = doc_id_to_u64("file:src/main.rs:0");
        let id2 = doc_id_to_u64("file:src/lib.rs:0");
        assert_ne!(id1, id2);
    }
}
