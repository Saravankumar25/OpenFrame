//! OpenFrame intelligence index — the DERIVED retrieval layer under
//! `<Project>.openframe/cache/intelligence.sqlite` (AI/RAG spec §9–§19, §31).
//!
//! ```text
//! project.sqlite (canonical) ──builders (openframe-application)──► IntelligenceDb
//!                                                                   ├─ semantic_document / semantic_chunk   (docs.rs)
//!                                                                   ├─ vec_chunk (sqlite-vec, vector.rs)     SemanticIndex
//!                                                                   ├─ context_node / context_edge (graph.rs)
//!                                                                   └─ index_meta (versions, identity)       (db.rs)
//! query ──► FTS5 (canonical search_fts) + vector KNN + graph expansion ──► rank.rs (RRF + boosts)
//! ```
//!
//! Nothing in this crate is a source of truth. Every table can be dropped and
//! rebuilt from `project.sqlite`; the application never writes canonical data
//! through it. This crate has no knowledge of OpenFrame domains — the
//! application layer decides what a document is, which rows it depends on and
//! who may see it.

pub mod chunk;
pub mod db;
pub mod docs;
pub mod embed;
pub mod graph;
pub mod rank;
pub mod vector;

pub use db::{IndexIdentity, IntelligenceDb, OpenOutcome, SCHEMA_VERSION};
pub use docs::{
    ChunkInput, DocumentInput, IndexCounts, StoredChunk, StoredDocument, UpsertOutcome,
};
pub use embed::{Embedder, QueryEmbeddingCache, l2_normalize};
pub use graph::{EdgeInput, EdgeProvenance, GraphHit, NodeInput, NodeRecord, TraversalLimits};
pub use rank::{Candidate, RankWeights, Ranked, rrf_fuse};
pub use vector::{SemanticIndex, SqliteVecIndex, VectorHit};

/// Errors of the derived index. The application maps every one of them to a
/// graceful fallback (FTS + SQL tools); none may endanger project access (§33).
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("intelligence index storage error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("intelligence index file error: {0}")]
    Io(#[from] std::io::Error),
    #[error("embedding failed: {0}")]
    Embedding(String),
    #[error("embedding has {got} dimensions; the index expects {expected}")]
    Dimension { expected: usize, got: usize },
    #[error("intelligence index schema problem: {0}")]
    Schema(String),
    #[error("semantic search is unavailable: {0}")]
    Unavailable(String),
}

pub type SearchResult<T> = Result<T, SearchError>;

/// Stable, short content hash (hex SHA-256 prefix) used to skip unchanged
/// documents/chunks so nothing is re-embedded needlessly (§14).
pub fn content_hash(parts: &[&str]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    for p in parts {
        h.update((p.len() as u64).to_le_bytes());
        h.update(p.as_bytes());
    }
    hex::encode(&h.finalize()[..16])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_hash_is_stable_and_boundary_aware() {
        assert_eq!(content_hash(&["a", "b"]), content_hash(&["a", "b"]));
        assert_ne!(content_hash(&["ab", ""]), content_hash(&["a", "b"]));
        assert_eq!(content_hash(&["x"]).len(), 32);
    }
}
