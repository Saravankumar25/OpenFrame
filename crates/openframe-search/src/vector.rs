//! Vector index behind the [`SemanticIndex`] trait (§11).
//!
//! Implementation: sqlite-vec's `vec0` virtual table, compiled into the binary
//! (the `sqlite-vec` crate builds its C source with `SQLITE_CORE`) and
//! initialised explicitly on OUR intelligence connections only — never through
//! `sqlite3_auto_extension`, so project databases and any other connection
//! never gain the module, and nothing is ever loaded from disk at runtime.
//! No other code depends on sqlite-vec SQL: swap the implementation here.

use std::ffi::{c_char, c_int};

use rusqlite::{Connection, OptionalExtension, ffi, params};

use crate::{SearchError, SearchResult};

/// The vec0 table (keyed by `semantic_chunk.vec_rowid`).
pub const VEC_TABLE: &str = "vec_chunk";

/// A nearest-neighbour hit: the chunk's vector row id and its cosine similarity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VectorHit {
    pub vec_rowid: i64,
    /// Cosine similarity in [-1, 1] (vectors are L2-normalised).
    pub similarity: f32,
}

/// Embedded, local vector storage + KNN. Vectors must be L2-normalised.
pub trait SemanticIndex: Send + Sync {
    /// Insert or replace the vector for `vec_rowid`.
    fn upsert(&self, conn: &Connection, vec_rowid: i64, vector: &[f32]) -> SearchResult<()>;
    fn delete(&self, conn: &Connection, vec_rowid: i64) -> SearchResult<()>;
    /// The `k` nearest vectors to `query`, most similar first.
    fn search(&self, conn: &Connection, query: &[f32], k: usize) -> SearchResult<Vec<VectorHit>>;
    /// Drop every vector and recreate an empty index of dimension `dim`.
    fn rebuild(&self, conn: &Connection, dim: usize) -> SearchResult<()>;
    /// The stored vector (re-used for identical chunk text instead of re-embedding).
    fn get(&self, conn: &Connection, vec_rowid: i64) -> SearchResult<Option<Vec<f32>>>;
}

type VecInit = unsafe extern "C" fn(
    *mut ffi::sqlite3,
    *mut *mut c_char,
    *const ffi::sqlite3_api_routines,
) -> c_int;

/// Register sqlite-vec's functions/modules on this one connection.
pub fn register(conn: &Connection) -> SearchResult<()> {
    // SAFETY: `sqlite3_vec_init` is the extension entry point with the standard
    // `(db, pzErrMsg, pApi)` signature (the crate declares it without arguments).
    // It is compiled with SQLITE_CORE, so `pApi` is unused and may be null, and
    // the SQLite it links against is the same bundled library rusqlite uses. The
    // handle is valid for the duration of the call because `conn` is borrowed.
    let rc = unsafe {
        let init: VecInit = std::mem::transmute(sqlite_vec::sqlite3_vec_init as *const ());
        init(conn.handle(), std::ptr::null_mut(), std::ptr::null())
    };
    if rc != ffi::SQLITE_OK {
        return Err(SearchError::Schema(format!(
            "vector module failed to initialise (code {rc})"
        )));
    }
    Ok(())
}

pub fn table_exists(conn: &Connection) -> SearchResult<bool> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1",
            [VEC_TABLE],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn to_blob(v: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(v.len() * 4);
    for x in v {
        out.extend_from_slice(&x.to_le_bytes());
    }
    out
}

fn from_blob(b: &[u8]) -> Vec<f32> {
    b.as_chunks::<4>()
        .0
        .iter()
        .map(|c| f32::from_le_bytes(*c))
        .collect()
}

/// sqlite-vec implementation (brute-force exact KNN; fast for project-sized corpora).
#[derive(Debug, Default, Clone, Copy)]
pub struct SqliteVecIndex;

impl SemanticIndex for SqliteVecIndex {
    fn upsert(&self, conn: &Connection, vec_rowid: i64, vector: &[f32]) -> SearchResult<()> {
        if vector.iter().any(|x| !x.is_finite()) {
            return Err(SearchError::Embedding("non-finite vector component".into()));
        }
        conn.execute(
            &format!("DELETE FROM {VEC_TABLE} WHERE rowid=?1"),
            [vec_rowid],
        )?;
        conn.execute(
            &format!("INSERT INTO {VEC_TABLE}(rowid, embedding) VALUES (?1, ?2)"),
            params![vec_rowid, to_blob(vector)],
        )?;
        Ok(())
    }

    fn delete(&self, conn: &Connection, vec_rowid: i64) -> SearchResult<()> {
        if table_exists(conn)? {
            conn.execute(
                &format!("DELETE FROM {VEC_TABLE} WHERE rowid=?1"),
                [vec_rowid],
            )?;
        }
        Ok(())
    }

    fn search(&self, conn: &Connection, query: &[f32], k: usize) -> SearchResult<Vec<VectorHit>> {
        if k == 0 || !table_exists(conn)? {
            return Ok(Vec::new());
        }
        let mut stmt = conn.prepare_cached(&format!(
            "SELECT rowid, distance FROM {VEC_TABLE} WHERE embedding MATCH ?1 AND k = ?2 ORDER BY distance"
        ))?;
        let hits = stmt
            .query_map(params![to_blob(query), k.min(4096) as i64], |r| {
                let d: f64 = r.get(1)?;
                Ok(VectorHit {
                    vec_rowid: r.get(0)?,
                    // L2 distance between unit vectors: d² = 2 − 2·cos.
                    similarity: (1.0 - (d * d) / 2.0) as f32,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(hits)
    }

    fn rebuild(&self, conn: &Connection, dim: usize) -> SearchResult<()> {
        conn.execute_batch(&format!(
            "DROP TABLE IF EXISTS {VEC_TABLE};
             CREATE VIRTUAL TABLE {VEC_TABLE} USING vec0(embedding float[{dim}]);"
        ))?;
        Ok(())
    }

    fn get(&self, conn: &Connection, vec_rowid: i64) -> SearchResult<Option<Vec<f32>>> {
        if !table_exists(conn)? {
            return Ok(None);
        }
        let blob: Option<Vec<u8>> = conn
            .query_row(
                &format!("SELECT embedding FROM {VEC_TABLE} WHERE rowid=?1"),
                [vec_rowid],
                |r| r.get(0),
            )
            .optional()?;
        Ok(blob.map(|b| from_blob(&b)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::l2_normalize;

    fn unit(v: &[f32]) -> Vec<f32> {
        let mut v = v.to_vec();
        l2_normalize(&mut v);
        v
    }

    #[test]
    fn knn_orders_by_cosine_similarity() {
        let conn = Connection::open_in_memory().unwrap();
        register(&conn).unwrap();
        let idx = SqliteVecIndex;
        idx.rebuild(&conn, 3).unwrap();
        idx.upsert(&conn, 1, &unit(&[1.0, 0.0, 0.0])).unwrap();
        idx.upsert(&conn, 2, &unit(&[0.7, 0.7, 0.0])).unwrap();
        idx.upsert(&conn, 3, &unit(&[0.0, 0.0, 1.0])).unwrap();
        let hits = idx.search(&conn, &unit(&[0.9, 0.1, 0.0]), 2).unwrap();
        assert_eq!(
            hits.iter().map(|h| h.vec_rowid).collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert!(hits[0].similarity > 0.98 && hits[0].similarity <= 1.0001);
        // Upsert replaces; delete removes.
        idx.upsert(&conn, 1, &unit(&[0.0, 0.0, 1.0])).unwrap();
        assert_eq!(
            idx.search(&conn, &unit(&[0.9, 0.1, 0.0]), 1).unwrap()[0].vec_rowid,
            2
        );
        idx.delete(&conn, 2).unwrap();
        assert!(idx.get(&conn, 2).unwrap().is_none());
        assert_eq!(idx.get(&conn, 3).unwrap().unwrap().len(), 3);
        assert!(idx.upsert(&conn, 9, &[f32::NAN, 0.0, 0.0]).is_err());
    }

    #[test]
    fn module_is_not_global() {
        let with = Connection::open_in_memory().unwrap();
        register(&with).unwrap();
        let without = Connection::open_in_memory().unwrap();
        assert!(
            without
                .execute_batch("CREATE VIRTUAL TABLE v USING vec0(e float[2])")
                .is_err()
        );
    }
}
