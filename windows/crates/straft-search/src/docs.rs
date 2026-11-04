//! Semantic documents and chunks (§13, §14, §31).
//!
//! One document per logical domain entity; chunks carry the embedding. Writes
//! are hash-guarded: an unchanged document is a no-op, and a changed document
//! only re-embeds chunks whose text changed (identical text anywhere in the
//! index re-uses its stored vector).

use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;

use crate::vector::SemanticIndex;
use crate::{SearchResult, content_hash};

#[derive(Debug, Clone, PartialEq)]
pub struct ChunkInput {
    pub text: String,
    pub metadata: Value,
}

/// A document as produced by an application builder.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentInput {
    pub doc_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub source_table: String,
    pub project_id: String,
    /// Human module label ("Screenplay", "Story", "Production", …).
    pub module: String,
    pub title: String,
    pub body: String,
    /// Private documents are visible to this user only.
    pub owner_user_id: Option<String>,
    pub source_rev: Option<i64>,
    /// Boundary key (e.g. the screenplay draft id) used by retrieval filters.
    pub scope_key: Option<String>,
    pub metadata: Value,
    pub chunks: Vec<ChunkInput>,
    /// Other canonical rows this document is built from (table, id).
    pub dependencies: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpsertOutcome {
    Unchanged,
    /// Written; `to_embed` chunks now wait for an embedding.
    Written {
        to_embed: usize,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoredDocument {
    pub doc_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub source_table: String,
    pub module: String,
    pub title: String,
    pub owner_user_id: Option<String>,
    pub source_rev: Option<i64>,
    pub scope_key: Option<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoredChunk {
    pub chunk_id: String,
    pub doc_id: String,
    pub ordinal: i64,
    pub text: String,
    pub vec_rowid: i64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IndexCounts {
    pub documents: u64,
    pub chunks: u64,
    /// Chunks embedded with the current model.
    pub embedded: u64,
    pub nodes: u64,
    pub edges: u64,
}

fn doc_hash(d: &DocumentInput) -> String {
    let meta = d.metadata.to_string();
    let rev = d.source_rev.map(|r| r.to_string()).unwrap_or_default();
    let mut parts: Vec<&str> = vec![
        &d.entity_type,
        &d.entity_id,
        &d.source_table,
        &d.module,
        &d.title,
        &d.body,
        d.owner_user_id.as_deref().unwrap_or(""),
        d.scope_key.as_deref().unwrap_or(""),
        &meta,
        &rev,
    ];
    let chunk_meta: Vec<String> = d.chunks.iter().map(|c| c.metadata.to_string()).collect();
    for (c, m) in d.chunks.iter().zip(&chunk_meta) {
        parts.push(&c.text);
        parts.push(m);
    }
    let deps: Vec<String> = d
        .dependencies
        .iter()
        .map(|(t, i)| format!("{t}:{i}"))
        .collect();
    for dep in &deps {
        parts.push(dep);
    }
    content_hash(&parts)
}

fn next_vec_rowid(conn: &Connection) -> SearchResult<i64> {
    Ok(conn.query_row(
        "SELECT coalesce(max(vec_rowid), 0) + 1 FROM semantic_chunk",
        [],
        |r| r.get(0),
    )?)
}

/// Try to re-use a stored vector of identical text for `model`.
fn reuse_vector(
    conn: &Connection,
    index: &dyn SemanticIndex,
    text_hash: &str,
    model: Option<&str>,
    target_rowid: i64,
) -> SearchResult<bool> {
    let Some(model) = model else { return Ok(false) };
    let donor: Option<i64> = conn
        .query_row(
            "SELECT vec_rowid FROM semantic_chunk WHERE text_hash=?1 AND embedding_model_id=?2 AND vec_rowid<>?3 LIMIT 1",
            params![text_hash, model, target_rowid],
            |r| r.get(0),
        )
        .optional()?;
    let Some(donor) = donor else { return Ok(false) };
    let Some(v) = index.get(conn, donor)? else {
        return Ok(false);
    };
    index.upsert(conn, target_rowid, &v)?;
    conn.execute(
        "UPDATE semantic_chunk SET embedding_model_id=?1, embedded_at=strftime('%s','now')*1000 WHERE vec_rowid=?2",
        params![model, target_rowid],
    )?;
    Ok(true)
}

/// Insert or update a document and its chunks. `model` is the current embedding
/// model id (for vector re-use), `now` the timestamp to record.
pub fn upsert_document(
    conn: &Connection,
    index: &dyn SemanticIndex,
    doc: &DocumentInput,
    model: Option<&str>,
    now: i64,
) -> SearchResult<UpsertOutcome> {
    let hash = doc_hash(doc);
    let existing: Option<String> = conn
        .query_row(
            "SELECT content_hash FROM semantic_document WHERE doc_id=?1",
            [&doc.doc_id],
            |r| r.get(0),
        )
        .optional()?;
    if existing.as_deref() == Some(hash.as_str()) {
        return Ok(UpsertOutcome::Unchanged);
    }
    conn.execute(
        "INSERT INTO semantic_document(doc_id, entity_type, entity_id, source_table, project_id, module, title, body,
                                       owner_user_id, source_rev, scope_key, content_hash, metadata_json, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
         ON CONFLICT(doc_id) DO UPDATE SET entity_type=excluded.entity_type, entity_id=excluded.entity_id,
             source_table=excluded.source_table, project_id=excluded.project_id, module=excluded.module,
             title=excluded.title, body=excluded.body, owner_user_id=excluded.owner_user_id,
             source_rev=excluded.source_rev, scope_key=excluded.scope_key, content_hash=excluded.content_hash,
             metadata_json=excluded.metadata_json, updated_at=excluded.updated_at",
        params![
            doc.doc_id,
            doc.entity_type,
            doc.entity_id,
            doc.source_table,
            doc.project_id,
            doc.module,
            doc.title,
            doc.body,
            doc.owner_user_id,
            doc.source_rev,
            doc.scope_key,
            hash,
            doc.metadata.to_string(),
            now
        ],
    )?;
    conn.execute("DELETE FROM doc_dependency WHERE doc_id=?1", [&doc.doc_id])?;
    for (t, i) in &doc.dependencies {
        conn.execute(
            "INSERT OR IGNORE INTO doc_dependency(source_table, source_id, doc_id) VALUES (?1, ?2, ?3)",
            params![t, i, doc.doc_id],
        )?;
    }
    let old: HashMap<i64, (String, String, i64)> = {
        let mut stmt = conn.prepare_cached(
            "SELECT ordinal, chunk_id, text_hash, vec_rowid FROM semantic_chunk WHERE doc_id=?1",
        )?;
        stmt.query_map([&doc.doc_id], |r| {
            Ok((r.get::<_, i64>(0)?, (r.get(1)?, r.get(2)?, r.get(3)?)))
        })?
        .collect::<Result<_, _>>()?
    };
    let mut to_embed = 0usize;
    for (i, chunk) in doc.chunks.iter().enumerate() {
        let ordinal = i as i64;
        let text_hash = content_hash(&[&chunk.text]);
        let tokens = crate::chunk::estimate_tokens(&chunk.text);
        let meta = chunk.metadata.to_string();
        match old.get(&ordinal) {
            Some((_, h, _)) if *h == text_hash => {
                conn.execute(
                    "UPDATE semantic_chunk SET metadata_json=?1 WHERE doc_id=?2 AND ordinal=?3",
                    params![meta, doc.doc_id, ordinal],
                )?;
            }
            Some((chunk_id, _, vec_rowid)) => {
                index.delete(conn, *vec_rowid)?;
                conn.execute(
                    "UPDATE semantic_chunk SET text=?1, text_hash=?2, token_estimate=?3, metadata_json=?4,
                         embedding_model_id=NULL, embedded_at=NULL WHERE chunk_id=?5",
                    params![chunk.text, text_hash, tokens, meta, chunk_id],
                )?;
                if !reuse_vector(conn, index, &text_hash, model, *vec_rowid)? {
                    to_embed += 1;
                }
            }
            None => {
                let vec_rowid = next_vec_rowid(conn)?;
                conn.execute(
                    "INSERT INTO semantic_chunk(chunk_id, doc_id, ordinal, text, text_hash, token_estimate, metadata_json, vec_rowid)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        format!("{}#{}", doc.doc_id, ordinal),
                        doc.doc_id,
                        ordinal,
                        chunk.text,
                        text_hash,
                        tokens,
                        meta,
                        vec_rowid
                    ],
                )?;
                if !reuse_vector(conn, index, &text_hash, model, vec_rowid)? {
                    to_embed += 1;
                }
            }
        }
    }
    for (ordinal, (chunk_id, _, vec_rowid)) in &old {
        if *ordinal >= doc.chunks.len() as i64 {
            index.delete(conn, *vec_rowid)?;
            conn.execute("DELETE FROM semantic_chunk WHERE chunk_id=?1", [chunk_id])?;
        }
    }
    Ok(UpsertOutcome::Written { to_embed })
}

/// Remove a document with its chunks, vectors and dependency rows. Returns true if it existed.
pub fn delete_document(
    conn: &Connection,
    index: &dyn SemanticIndex,
    doc_id: &str,
) -> SearchResult<bool> {
    let rowids: Vec<i64> = {
        let mut stmt =
            conn.prepare_cached("SELECT vec_rowid FROM semantic_chunk WHERE doc_id=?1")?;
        stmt.query_map([doc_id], |r| r.get(0))?
            .collect::<Result<_, _>>()?
    };
    for r in rowids {
        index.delete(conn, r)?;
    }
    conn.execute("DELETE FROM semantic_chunk WHERE doc_id=?1", [doc_id])?;
    conn.execute("DELETE FROM doc_dependency WHERE doc_id=?1", [doc_id])?;
    Ok(conn.execute("DELETE FROM semantic_document WHERE doc_id=?1", [doc_id])? > 0)
}

/// Documents built from the canonical row (table, id) besides their own entity.
pub fn docs_depending_on(conn: &Connection, table: &str, id: &str) -> SearchResult<Vec<String>> {
    let mut stmt = conn.prepare_cached(
        "SELECT doc_id FROM doc_dependency WHERE source_table=?1 AND source_id=?2",
    )?;
    let out = stmt
        .query_map(params![table, id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(out)
}

/// Every stored document id of one entity type.
pub fn doc_ids_of_type(conn: &Connection, entity_type: &str) -> SearchResult<Vec<String>> {
    let mut stmt = conn.prepare("SELECT doc_id FROM semantic_document WHERE entity_type=?1")?;
    let out = stmt
        .query_map([entity_type], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(out)
}

pub fn all_doc_ids(conn: &Connection) -> SearchResult<Vec<String>> {
    let mut stmt = conn.prepare("SELECT doc_id FROM semantic_document")?;
    let out = stmt
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(out)
}

fn map_doc(r: &rusqlite::Row<'_>) -> rusqlite::Result<StoredDocument> {
    let meta: String = r.get(9)?;
    Ok(StoredDocument {
        doc_id: r.get(0)?,
        entity_type: r.get(1)?,
        entity_id: r.get(2)?,
        source_table: r.get(3)?,
        module: r.get(4)?,
        title: r.get(5)?,
        owner_user_id: r.get(6)?,
        source_rev: r.get(7)?,
        scope_key: r.get(8)?,
        metadata: serde_json::from_str(&meta).unwrap_or(Value::Null),
    })
}

const DOC_COLS: &str = "doc_id, entity_type, entity_id, source_table, module, title, owner_user_id, source_rev, scope_key, metadata_json";

pub fn get_document(conn: &Connection, doc_id: &str) -> SearchResult<Option<StoredDocument>> {
    Ok(conn
        .query_row(
            &format!("SELECT {DOC_COLS} FROM semantic_document WHERE doc_id=?1"),
            [doc_id],
            map_doc,
        )
        .optional()?)
}

/// Chunks (with their documents) for vector hits, in the order given.
pub fn chunks_by_vec_rowids(
    conn: &Connection,
    rowids: &[i64],
) -> SearchResult<Vec<(StoredChunk, StoredDocument)>> {
    let mut out = Vec::with_capacity(rowids.len());
    let mut stmt = conn.prepare_cached(&format!(
        "SELECT c.chunk_id, c.doc_id, c.ordinal, c.text, c.vec_rowid,
                {}
         FROM semantic_chunk c JOIN semantic_document d ON d.doc_id = c.doc_id WHERE c.vec_rowid=?1",
        DOC_COLS
            .split(", ")
            .map(|c| format!("d.{c}"))
            .collect::<Vec<_>>()
            .join(", ")
    ))?;
    for id in rowids {
        let row = stmt
            .query_row([id], |r| {
                let chunk = StoredChunk {
                    chunk_id: r.get(0)?,
                    doc_id: r.get(1)?,
                    ordinal: r.get(2)?,
                    text: r.get(3)?,
                    vec_rowid: r.get(4)?,
                };
                let meta: String = r.get(14)?;
                let doc = StoredDocument {
                    doc_id: r.get(5)?,
                    entity_type: r.get(6)?,
                    entity_id: r.get(7)?,
                    source_table: r.get(8)?,
                    module: r.get(9)?,
                    title: r.get(10)?,
                    owner_user_id: r.get(11)?,
                    source_rev: r.get(12)?,
                    scope_key: r.get(13)?,
                    metadata: serde_json::from_str(&meta).unwrap_or(Value::Null),
                };
                Ok((chunk, doc))
            })
            .optional()?;
        if let Some(row) = row {
            out.push(row);
        }
    }
    Ok(out)
}

/// Chunks still waiting for an embedding by `model`: (vec_rowid, text).
pub fn pending_chunks(
    conn: &Connection,
    model: &str,
    limit: usize,
) -> SearchResult<Vec<(i64, String)>> {
    let mut stmt = conn.prepare_cached(
        "SELECT vec_rowid, text FROM semantic_chunk
         WHERE embedding_model_id IS NULL OR embedding_model_id<>?1 ORDER BY vec_rowid LIMIT ?2",
    )?;
    let out = stmt
        .query_map(params![model, limit as i64], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    Ok(out)
}

pub fn pending_count(conn: &Connection, model: Option<&str>) -> SearchResult<u64> {
    let n: i64 = match model {
        Some(m) => conn.query_row(
            "SELECT count(*) FROM semantic_chunk WHERE embedding_model_id IS NULL OR embedding_model_id<>?1",
            [m],
            |r| r.get(0),
        )?,
        None => conn.query_row("SELECT count(*) FROM semantic_chunk", [], |r| r.get(0))?,
    };
    Ok(n as u64)
}

/// Store an embedding for a chunk (skipped if the chunk changed meanwhile).
pub fn store_embedding(
    conn: &Connection,
    index: &dyn SemanticIndex,
    vec_rowid: i64,
    expected_text: &str,
    model: &str,
    vector: &[f32],
    now: i64,
) -> SearchResult<bool> {
    let current: Option<String> = conn
        .query_row(
            "SELECT text FROM semantic_chunk WHERE vec_rowid=?1",
            [vec_rowid],
            |r| r.get(0),
        )
        .optional()?;
    if current.as_deref() != Some(expected_text) {
        return Ok(false);
    }
    index.upsert(conn, vec_rowid, vector)?;
    conn.execute(
        "UPDATE semantic_chunk SET embedding_model_id=?1, embedded_at=?2 WHERE vec_rowid=?3",
        params![model, now, vec_rowid],
    )?;
    Ok(true)
}

pub fn counts(conn: &Connection, model: Option<&str>) -> SearchResult<IndexCounts> {
    let one = |sql: &str| -> SearchResult<u64> {
        let n: i64 = conn.query_row(sql, [], |r| r.get(0))?;
        Ok(n as u64)
    };
    let embedded = match model {
        Some(m) => {
            let n: i64 = conn.query_row(
                "SELECT count(*) FROM semantic_chunk WHERE embedding_model_id=?1",
                [m],
                |r| r.get(0),
            )?;
            n as u64
        }
        None => 0,
    };
    Ok(IndexCounts {
        documents: one("SELECT count(*) FROM semantic_document")?,
        chunks: one("SELECT count(*) FROM semantic_chunk")?,
        embedded,
        nodes: one("SELECT count(*) FROM context_node")?,
        edges: one("SELECT count(*) FROM context_edge")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{IndexIdentity, IntelligenceDb};
    use crate::vector::SqliteVecIndex;
    use serde_json::json;

    fn db() -> (tempfile::TempDir, IntelligenceDb) {
        let dir = tempfile::tempdir().unwrap();
        let (mut db, _) = IntelligenceDb::open(
            &dir.path().join("i.sqlite"),
            &IndexIdentity {
                project_id: "p".into(),
                canonical_schema_version: 1,
                builder_version: 1,
            },
        )
        .unwrap();
        db.ensure_embedding_model("m", 2).unwrap();
        (dir, db)
    }

    fn doc(id: &str, chunks: &[&str]) -> DocumentInput {
        DocumentInput {
            doc_id: format!("t:{id}"),
            entity_type: "t".into(),
            entity_id: id.into(),
            source_table: "t".into(),
            project_id: "p".into(),
            module: "M".into(),
            title: format!("Doc {id}"),
            body: chunks.join("\n"),
            owner_user_id: None,
            source_rev: Some(1),
            scope_key: None,
            metadata: json!({}),
            chunks: chunks
                .iter()
                .map(|t| ChunkInput {
                    text: t.to_string(),
                    metadata: json!({}),
                })
                .collect(),
            dependencies: vec![("dep".into(), "d1".into())],
        }
    }

    fn embed_all(db: &IntelligenceDb) -> usize {
        let pending = pending_chunks(db.conn(), "m", 100).unwrap();
        for (rowid, text) in &pending {
            let v = vec![text.len() as f32, 1.0];
            let mut v = v;
            crate::embed::l2_normalize(&mut v);
            store_embedding(db.conn(), &SqliteVecIndex, *rowid, text, "m", &v, 1).unwrap();
        }
        pending.len()
    }

    #[test]
    fn unchanged_documents_are_not_rewritten_and_changed_chunks_only_reembed() {
        let (_d, db) = db();
        let c = db.conn();
        let out = upsert_document(
            c,
            &SqliteVecIndex,
            &doc("1", &["alpha", "beta"]),
            Some("m"),
            1,
        )
        .unwrap();
        assert_eq!(out, UpsertOutcome::Written { to_embed: 2 });
        assert_eq!(embed_all(&db), 2);
        assert_eq!(
            upsert_document(
                c,
                &SqliteVecIndex,
                &doc("1", &["alpha", "beta"]),
                Some("m"),
                2
            )
            .unwrap(),
            UpsertOutcome::Unchanged
        );
        // Second chunk edited, third added: only those wait for embeddings.
        let out = upsert_document(
            c,
            &SqliteVecIndex,
            &doc("1", &["alpha", "gamma", "delta"]),
            Some("m"),
            3,
        )
        .unwrap();
        assert_eq!(out, UpsertOutcome::Written { to_embed: 2 });
        assert_eq!(pending_count(c, Some("m")).unwrap(), 2);
        embed_all(&db);
        // Shrinking removes trailing chunks and their vectors.
        upsert_document(c, &SqliteVecIndex, &doc("1", &["alpha"]), Some("m"), 4).unwrap();
        let n = counts(c, Some("m")).unwrap();
        assert_eq!((n.documents, n.chunks, n.embedded), (1, 1, 1));
        assert_eq!(
            docs_depending_on(c, "dep", "d1").unwrap(),
            vec!["t:1".to_string()]
        );
    }

    #[test]
    fn identical_text_reuses_a_stored_vector() {
        let (_d, db) = db();
        let c = db.conn();
        upsert_document(c, &SqliteVecIndex, &doc("1", &["same words"]), Some("m"), 1).unwrap();
        embed_all(&db);
        let out =
            upsert_document(c, &SqliteVecIndex, &doc("2", &["same words"]), Some("m"), 1).unwrap();
        assert_eq!(out, UpsertOutcome::Written { to_embed: 0 });
        let hits = SqliteVecIndex.search(c, &[1.0, 0.0], 5).unwrap();
        let rows =
            chunks_by_vec_rowids(c, &hits.iter().map(|h| h.vec_rowid).collect::<Vec<_>>()).unwrap();
        assert_eq!(rows.len(), 2);
        assert!(
            rows.iter()
                .all(|(ch, d)| ch.text == "same words" && d.module == "M")
        );
    }

    #[test]
    fn delete_removes_everything_derived() {
        let (_d, db) = db();
        let c = db.conn();
        upsert_document(c, &SqliteVecIndex, &doc("1", &["x", "y"]), Some("m"), 1).unwrap();
        embed_all(&db);
        assert!(delete_document(c, &SqliteVecIndex, "t:1").unwrap());
        assert!(!delete_document(c, &SqliteVecIndex, "t:1").unwrap());
        let n = counts(c, Some("m")).unwrap();
        assert_eq!((n.documents, n.chunks), (0, 0));
        assert!(SqliteVecIndex.search(c, &[1.0, 0.0], 5).unwrap().is_empty());
        assert!(docs_depending_on(c, "dep", "d1").unwrap().is_empty());
    }

    #[test]
    fn stale_embedding_results_are_discarded() {
        let (_d, db) = db();
        let c = db.conn();
        upsert_document(c, &SqliteVecIndex, &doc("1", &["old"]), Some("m"), 1).unwrap();
        let (rowid, _) = pending_chunks(c, "m", 10).unwrap()[0].clone();
        upsert_document(c, &SqliteVecIndex, &doc("1", &["new text"]), Some("m"), 2).unwrap();
        // The embedding computed for "old" arrives after the edit: dropped.
        assert!(!store_embedding(c, &SqliteVecIndex, rowid, "old", "m", &[1.0, 0.0], 3).unwrap());
        assert_eq!(pending_count(c, Some("m")).unwrap(), 1);
        assert!(get_document(c, "t:1").unwrap().is_some());
        assert_eq!(doc_ids_of_type(c, "t").unwrap(), vec!["t:1".to_string()]);
        assert_eq!(all_doc_ids(c).unwrap().len(), 1);
    }
}
