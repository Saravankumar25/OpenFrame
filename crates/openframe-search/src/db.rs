//! `cache/intelligence.sqlite`: versioned, validated, disposable (§9, §31, §32).
//!
//! The file lives inside a project folder, and project folders can come from
//! anywhere, so it is treated as untrusted on open: integrity check, an exact
//! allow-list of schema objects (no triggers/views), schema/builder/project
//! identity. Any mismatch deletes and recreates it — it never holds anything
//! that cannot be rebuilt from `project.sqlite`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, OpenFlags, OptionalExtension, params};

use crate::vector::{self, SqliteVecIndex};
use crate::{SearchError, SearchResult};

/// Version of THIS file's schema. Bump on any DDL change: old files are rebuilt.
pub const SCHEMA_VERSION: u32 = 1;

const SCHEMA: &str = r#"
CREATE TABLE index_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE semantic_document (
    doc_id        TEXT PRIMARY KEY,
    entity_type   TEXT NOT NULL,
    entity_id     TEXT NOT NULL,
    source_table  TEXT NOT NULL,
    project_id    TEXT NOT NULL,
    module        TEXT NOT NULL,
    title         TEXT NOT NULL,
    body          TEXT NOT NULL,
    owner_user_id TEXT,
    source_rev    INTEGER,
    scope_key     TEXT,
    content_hash  TEXT NOT NULL,
    metadata_json TEXT NOT NULL,
    updated_at    INTEGER NOT NULL
);
CREATE INDEX idx_semantic_document_entity ON semantic_document(entity_type, entity_id);
CREATE INDEX idx_semantic_document_source ON semantic_document(source_table, entity_id);
CREATE TABLE semantic_chunk (
    chunk_id           TEXT PRIMARY KEY,
    doc_id             TEXT NOT NULL,
    ordinal            INTEGER NOT NULL,
    text               TEXT NOT NULL,
    text_hash          TEXT NOT NULL,
    token_estimate     INTEGER NOT NULL,
    metadata_json      TEXT NOT NULL,
    vec_rowid          INTEGER NOT NULL UNIQUE,
    embedding_model_id TEXT,
    embedded_at        INTEGER
);
CREATE INDEX idx_semantic_chunk_doc ON semantic_chunk(doc_id, ordinal);
CREATE INDEX idx_semantic_chunk_hash ON semantic_chunk(text_hash);
CREATE INDEX idx_semantic_chunk_pending ON semantic_chunk(embedding_model_id);
CREATE TABLE doc_dependency (
    source_table TEXT NOT NULL,
    source_id    TEXT NOT NULL,
    doc_id       TEXT NOT NULL,
    PRIMARY KEY (source_table, source_id, doc_id)
) WITHOUT ROWID;
CREATE INDEX idx_doc_dependency_doc ON doc_dependency(doc_id);
CREATE TABLE context_node (
    node_id       TEXT PRIMARY KEY,
    entity_type   TEXT NOT NULL,
    entity_id     TEXT NOT NULL,
    label         TEXT NOT NULL,
    module        TEXT NOT NULL,
    owner_user_id TEXT,
    source_rev    INTEGER,
    metadata_json TEXT NOT NULL
);
CREATE UNIQUE INDEX idx_context_node_entity ON context_node(entity_type, entity_id);
CREATE TABLE context_edge (
    edge_id       TEXT PRIMARY KEY,
    from_node_id  TEXT NOT NULL,
    relation      TEXT NOT NULL,
    to_node_id    TEXT NOT NULL,
    provenance    TEXT NOT NULL CHECK (provenance IN ('Canonical','Derived','Inferred')),
    weight        REAL NOT NULL DEFAULT 1.0,
    source_rev    INTEGER,
    origin_key    TEXT NOT NULL,
    metadata_json TEXT NOT NULL
);
CREATE INDEX idx_context_edge_from ON context_edge(from_node_id, relation);
CREATE INDEX idx_context_edge_to ON context_edge(to_node_id, relation);
CREATE INDEX idx_context_edge_origin ON context_edge(origin_key);
"#;

/// Tables and indexes this file may contain (besides sqlite-vec's own shadow
/// tables `vec_chunk_*` and SQLite's automatic indexes).
const ALLOWED_OBJECTS: &[&str] = &[
    "index_meta",
    "semantic_document",
    "idx_semantic_document_entity",
    "idx_semantic_document_source",
    "semantic_chunk",
    "idx_semantic_chunk_doc",
    "idx_semantic_chunk_hash",
    "idx_semantic_chunk_pending",
    "doc_dependency",
    "idx_doc_dependency_doc",
    "context_node",
    "idx_context_node_entity",
    "context_edge",
    "idx_context_edge_from",
    "idx_context_edge_to",
    "idx_context_edge_origin",
    vector::VEC_TABLE,
];

/// What the index was built from. Any difference → the file is rebuilt (§32).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexIdentity {
    pub project_id: String,
    /// `project.sqlite` schema version (migration count) the documents were built against.
    pub canonical_schema_version: u32,
    /// Version of the application's document/graph builders.
    pub builder_version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenOutcome {
    /// The file was (re)created empty: everything must be rebuilt.
    pub created: bool,
    /// Why an existing file was discarded (diagnostics only; never project content).
    pub reason: Option<String>,
}

pub struct IntelligenceDb {
    conn: Connection,
    path: PathBuf,
}

fn configure(conn: &Connection) -> SearchResult<()> {
    conn.busy_timeout(Duration::from_secs(5))?;
    let _mode: String = conn.query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0))?;
    // Derived data: losing the last write on power loss only means re-indexing it.
    conn.execute_batch(
        "PRAGMA synchronous=NORMAL;
         PRAGMA foreign_keys=OFF;
         PRAGMA trusted_schema=OFF;
         PRAGMA temp_store=MEMORY;",
    )?;
    vector::register(conn)?;
    Ok(())
}

fn open_raw(path: &Path, create: bool) -> SearchResult<Connection> {
    let mut flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    if create {
        flags |= OpenFlags::SQLITE_OPEN_CREATE;
    }
    let conn = Connection::open_with_flags(path, flags)?;
    configure(&conn)?;
    Ok(conn)
}

fn remove_files(path: &Path) -> SearchResult<()> {
    for suffix in ["", "-wal", "-shm", "-journal"] {
        let p = PathBuf::from(format!("{}{}", path.display(), suffix));
        if p.exists() {
            std::fs::remove_file(&p)?;
        }
    }
    Ok(())
}

fn meta(conn: &Connection, key: &str) -> SearchResult<Option<String>> {
    Ok(conn
        .query_row("SELECT value FROM index_meta WHERE key=?1", [key], |r| {
            r.get(0)
        })
        .optional()?)
}

/// Reasons an existing file must not be reused; `None` = reusable.
fn validate(conn: &Connection, id: &IndexIdentity) -> SearchResult<Option<String>> {
    let check: String = conn.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
    if check != "ok" {
        return Ok(Some("integrity check failed".into()));
    }
    let mut stmt = conn.prepare("SELECT type, name FROM sqlite_master")?;
    let objects: Vec<(String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    for (kind, name) in &objects {
        let allowed = match kind.as_str() {
            "table" | "index" => {
                ALLOWED_OBJECTS.contains(&name.as_str())
                    || name.starts_with(&format!("{}_", vector::VEC_TABLE))
                    || name.starts_with("sqlite_autoindex_")
                    || name == "sqlite_sequence"
                    || name == "sqlite_stat1"
            }
            _ => false,
        };
        if !allowed {
            return Ok(Some(format!("unexpected schema object ({kind})")));
        }
    }
    let has_meta = objects.iter().any(|(_, n)| n == "index_meta");
    if !has_meta {
        return Ok(Some("not an intelligence index".into()));
    }
    let expect = [
        ("schema_version", SCHEMA_VERSION.to_string()),
        ("project_id", id.project_id.clone()),
        (
            "canonical_schema_version",
            id.canonical_schema_version.to_string(),
        ),
        ("builder_version", id.builder_version.to_string()),
    ];
    for (key, want) in expect {
        if meta(conn, key)?.as_deref() != Some(want.as_str()) {
            return Ok(Some(format!("{key} changed")));
        }
    }
    Ok(None)
}

impl IntelligenceDb {
    /// Open (or create) the index at `path`, discarding and recreating it when it
    /// is damaged, foreign, hostile or was built by another schema/builder/project.
    pub fn open(path: &Path, id: &IndexIdentity) -> SearchResult<(IntelligenceDb, OpenOutcome)> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut reason = None;
        if path.is_file() {
            let verdict = match open_raw(path, false) {
                Ok(conn) => match validate(&conn, id) {
                    Ok(None) => {
                        return Ok((
                            IntelligenceDb {
                                conn,
                                path: path.to_path_buf(),
                            },
                            OpenOutcome {
                                created: false,
                                reason: None,
                            },
                        ));
                    }
                    Ok(Some(r)) => r,
                    Err(e) => format!("unreadable ({e})"),
                },
                Err(e) => format!("unopenable ({e})"),
            };
            reason = Some(verdict);
        }
        remove_files(path)?;
        let conn = open_raw(path, true)?;
        conn.execute_batch(SCHEMA)?;
        let db = IntelligenceDb {
            conn,
            path: path.to_path_buf(),
        };
        db.set_meta("schema_version", &SCHEMA_VERSION.to_string())?;
        db.set_meta("project_id", &id.project_id)?;
        db.set_meta(
            "canonical_schema_version",
            &id.canonical_schema_version.to_string(),
        )?;
        db.set_meta("builder_version", &id.builder_version.to_string())?;
        Ok((
            db,
            OpenOutcome {
                created: true,
                reason,
            },
        ))
    }

    /// A query-only connection for retrieval (never blocks the indexer's writes).
    pub fn open_reader(path: &Path) -> SearchResult<Connection> {
        let conn = open_raw(path, false)?;
        conn.execute_batch("PRAGMA query_only=ON;")?;
        Ok(conn)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    pub fn meta(&self, key: &str) -> SearchResult<Option<String>> {
        meta(&self.conn, key)
    }

    pub fn set_meta(&self, key: &str, value: &str) -> SearchResult<()> {
        self.conn.execute(
            "INSERT INTO index_meta(key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    /// The embedding model the stored vectors belong to: (model id, dimension).
    pub fn embedding_model(&self) -> SearchResult<Option<(String, usize)>> {
        let id = self.meta("embedding_model_id")?;
        let dim = self
            .meta("embedding_dim")?
            .and_then(|d| d.parse::<usize>().ok());
        Ok(id.zip(dim))
    }

    /// Make the vector index match `model_id`/`dim`. A different model or
    /// dimension drops every stored vector (they are not comparable across
    /// models) and marks all chunks for re-embedding. Returns true when the
    /// vectors were reset.
    pub fn ensure_embedding_model(&mut self, model_id: &str, dim: usize) -> SearchResult<bool> {
        if dim == 0 || dim > 4096 {
            return Err(SearchError::Schema(format!(
                "unsupported embedding dimension {dim}"
            )));
        }
        let current = self.embedding_model()?;
        let index = SqliteVecIndex;
        if current.as_ref().map(|(m, d)| (m.as_str(), *d)) == Some((model_id, dim))
            && vector::table_exists(&self.conn)?
        {
            return Ok(false);
        }
        let tx = self.conn.transaction()?;
        crate::vector::SemanticIndex::rebuild(&index, &tx, dim)?;
        tx.execute(
            "UPDATE semantic_chunk SET embedding_model_id=NULL, embedded_at=NULL",
            [],
        )?;
        for (k, v) in [
            ("embedding_model_id", model_id.to_string()),
            ("embedding_dim", dim.to_string()),
        ] {
            tx.execute(
                "INSERT INTO index_meta(key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                params![k, v],
            )?;
        }
        tx.commit()?;
        Ok(true)
    }

    /// Remove every derived row (explicit "rebuild project context").
    pub fn clear_all(&mut self) -> SearchResult<()> {
        let tx = self.conn.transaction()?;
        tx.execute_batch(
            "DELETE FROM semantic_chunk; DELETE FROM semantic_document; DELETE FROM doc_dependency;
             DELETE FROM context_edge; DELETE FROM context_node;",
        )?;
        if let Some((_, dim)) = {
            let id: Option<String> = meta(&tx, "embedding_model_id")?;
            let dim = meta(&tx, "embedding_dim")?.and_then(|d| d.parse::<usize>().ok());
            id.zip(dim)
        } {
            crate::vector::SemanticIndex::rebuild(&SqliteVecIndex, &tx, dim)?;
        }
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ident(project: &str) -> IndexIdentity {
        IndexIdentity {
            project_id: project.into(),
            canonical_schema_version: 9,
            builder_version: 1,
        }
    }

    #[test]
    fn creates_then_reuses_then_rebuilds_on_identity_change() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cache").join("intelligence.sqlite");
        let (db, out) = IntelligenceDb::open(&path, &ident("p1")).unwrap();
        assert!(out.created);
        db.set_meta("marker", "kept").unwrap();
        drop(db);
        let (db, out) = IntelligenceDb::open(&path, &ident("p1")).unwrap();
        assert!(!out.created);
        assert_eq!(db.meta("marker").unwrap().as_deref(), Some("kept"));
        drop(db);
        // A copied project folder (other project id) never reuses the index.
        let (db, out) = IntelligenceDb::open(&path, &ident("p2")).unwrap();
        assert!(out.created);
        assert_eq!(out.reason.as_deref(), Some("project_id changed"));
        assert_eq!(db.meta("marker").unwrap(), None);
        drop(db);
        // A migrated canonical schema invalidates the documents.
        let mut id = ident("p2");
        id.canonical_schema_version = 10;
        let (_, out) = IntelligenceDb::open(&path, &id).unwrap();
        assert!(out.created);
    }

    #[test]
    fn hostile_or_garbage_files_are_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("intelligence.sqlite");
        std::fs::write(&path, b"this is not a database").unwrap();
        let (_, out) = IntelligenceDb::open(&path, &ident("p")).unwrap();
        assert!(out.created);
        // A trigger smuggled into a valid-looking index is refused.
        {
            let (db, _) = IntelligenceDb::open(&path, &ident("p")).unwrap();
            db.conn()
                .execute_batch(
                    "CREATE TRIGGER evil AFTER INSERT ON semantic_document BEGIN DELETE FROM index_meta; END;",
                )
                .unwrap();
        }
        let (db, out) = IntelligenceDb::open(&path, &ident("p")).unwrap();
        assert!(out.created);
        assert_eq!(
            out.reason.as_deref(),
            Some("unexpected schema object (trigger)")
        );
        let n: i64 = db
            .conn()
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='trigger'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn embedding_model_change_resets_vectors() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("intelligence.sqlite");
        let (mut db, _) = IntelligenceDb::open(&path, &ident("p")).unwrap();
        assert!(db.ensure_embedding_model("m1", 4).unwrap());
        assert!(!db.ensure_embedding_model("m1", 4).unwrap());
        assert!(db.ensure_embedding_model("m2", 8).unwrap());
        assert_eq!(db.embedding_model().unwrap(), Some(("m2".into(), 8)));
        drop(db);
        // vec0 shadow tables pass the schema allow-list on reopen.
        let (_, out) = IntelligenceDb::open(&path, &ident("p")).unwrap();
        assert!(!out.created, "{:?}", out.reason);
    }
}
