//! SQLite persistence core (ESD §2, Physical DB conventions in docs/engineering/11-physical-database.md).
//!
//! Responsibilities:
//! - connection policy (WAL, FULL sync, foreign keys, busy timeout);
//! - transactional, checksum-verified migrations with too-new detection;
//! - generic, trigger-based undo/redo capture over every canonical table;
//! - small validated row helpers (column-allow-listed updates, ordering).
//!
//! Only the application layer calls into this crate. React, AI and import
//! parsers never reach SQLite directly (Engineering Index invariants 1–3).

pub mod migrate;
pub mod rows;
pub mod undo;

use std::path::Path;
use std::time::Duration;

use openframe_domain::{AppError, AppResult};
use rusqlite::{Connection, OpenFlags};

pub use rusqlite;

/// Open (or create) a database with OpenFrame's durability policy.
pub fn open_connection(path: &Path, create: bool) -> AppResult<Connection> {
    let mut flags = OpenFlags::SQLITE_OPEN_READ_WRITE
        | OpenFlags::SQLITE_OPEN_NO_MUTEX
        | OpenFlags::SQLITE_OPEN_URI;
    if create {
        flags |= OpenFlags::SQLITE_OPEN_CREATE;
    }
    let conn = Connection::open_with_flags(path, flags)?;
    apply_pragmas(&conn)?;
    Ok(conn)
}

/// In-memory database with the same policy (tests, scratch validation).
pub fn open_in_memory() -> AppResult<Connection> {
    let conn = Connection::open_in_memory()?;
    apply_pragmas(&conn)?;
    Ok(conn)
}

fn apply_pragmas(conn: &Connection) -> AppResult<()> {
    conn.busy_timeout(Duration::from_secs(5))?;
    // WAL: readers never block the writer; crash-safe. FULL: a committed
    // autosave survives power loss (never silently lose work).
    let mode: String = conn.query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0))?;
    if !mode.eq_ignore_ascii_case("wal") && !mode.eq_ignore_ascii_case("memory") {
        tracing::warn!(
            mode,
            "WAL journal mode unavailable; continuing with fallback mode"
        );
    }
    conn.execute_batch(
        "PRAGMA synchronous=FULL;
         PRAGMA foreign_keys=ON;
         PRAGMA temp_store=MEMORY;
         PRAGMA recursive_triggers=OFF;
         PRAGMA trusted_schema=OFF;
         PRAGMA cache_size=-16000;",
    )?;
    // OpenFrame never attaches databases. With the limit at 0, SQL that reached a
    // connection from a received project's schema can't `ATTACH 'C:\…\x'` to create
    // files elsewhere on disk (Security review 2026-09-30, PKG-03).
    conn.set_limit(rusqlite::limits::Limit::SQLITE_LIMIT_ATTACHED, 0)?;
    Ok(())
}

fn safe_schema_name(n: &str) -> bool {
    !n.is_empty()
        && n.len() <= 128
        && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !n.starts_with(|c: char| c.is_ascii_digit())
}

fn squash(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Schema screening for databases that may come from someone else (received project or
/// backup packages, project folders copied from elsewhere). A project database is data,
/// but SQLite also stores *code* in it (triggers, views) that runs inside OpenFrame's
/// connection. Every schema object must have a plain identifier name, and every trigger
/// and view must be one OpenFrame's own migrations create (compared ignoring whitespace).
/// Returns the problems found (empty = acceptable).
pub fn untrusted_schema_problems(
    conn: &Connection,
    migrations_sql: &[&str],
) -> AppResult<Vec<String>> {
    let known = squash(&migrations_sql.concat());
    let mut stmt = conn.prepare("SELECT type, name, tbl_name, sql FROM main.sqlite_master")?;
    let rows: Vec<(String, String, String, Option<String>)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<Result<_, _>>()?;
    let mut problems = Vec::new();
    for (ty, name, tbl, sql) in rows {
        if !safe_schema_name(&name) || !safe_schema_name(&tbl) {
            problems.push(format!("{ty} with an unsafe name"));
            continue;
        }
        if matches!(ty.as_str(), "trigger" | "view") {
            let ok = sql
                .as_deref()
                .map(|s| {
                    let s = squash(s);
                    !s.is_empty() && known.contains(&s)
                })
                .unwrap_or(false);
            if !ok {
                problems.push(format!("unexpected {ty} {name}"));
            }
        }
    }
    Ok(problems)
}

/// `PRAGMA quick_check` / `integrity_check`. Returns the problems found (empty = healthy).
pub fn integrity_problems(conn: &Connection, full: bool) -> AppResult<Vec<String>> {
    let sql = if full {
        "PRAGMA integrity_check"
    } else {
        "PRAGMA quick_check"
    };
    let mut stmt = conn.prepare(sql)?;
    let rows: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<Result<_, _>>()?;
    Ok(rows.into_iter().filter(|r| r != "ok").collect())
}

/// Foreign-key violations as `table:rowid` strings (empty = healthy).
pub fn foreign_key_problems(conn: &Connection) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare("PRAGMA foreign_key_check")?;
    let rows = stmt
        .query_map([], |r| {
            Ok(format!(
                "{}:{}",
                r.get::<_, String>(0)?,
                r.get::<_, Option<i64>>(1)?.unwrap_or(-1)
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Consistent online copy of a live database (backups, checkpoints, pre-migration
/// safety copies). Writes a temporary sibling, verifies it, then renames, so a
/// crash never leaves a half-written backup under the final name.
pub fn backup_to(conn: &Connection, dest: &Path) -> AppResult<()> {
    let tmp = dest.with_extension("partial");
    if tmp.exists() {
        std::fs::remove_file(&tmp)?;
    }
    {
        let mut out = Connection::open(&tmp)?;
        let backup = rusqlite::backup::Backup::new(conn, &mut out)?;
        backup.run_to_completion(256, Duration::from_millis(0), None)?;
    }
    {
        let check = Connection::open(&tmp)?;
        check.execute_batch("PRAGMA journal_mode=DELETE;")?;
        let problems = integrity_problems(&check, false)?;
        if !problems.is_empty() {
            return Err(AppError::storage(format!(
                "backup verification failed: {}",
                problems.join("; ")
            )));
        }
    }
    if dest.exists() {
        std::fs::remove_file(dest)?;
    }
    std::fs::rename(&tmp, dest)?;
    Ok(())
}

/// Fold the WAL into the main database file (explicit Save / clean close).
pub fn checkpoint(conn: &Connection) -> AppResult<()> {
    conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pragmas_applied() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_connection(&dir.path().join("t.sqlite"), true).unwrap();
        let fk: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
            .unwrap();
        assert_eq!(fk, 1);
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mode.to_lowercase(), "wal");
    }

    #[test]
    fn backup_produces_verified_copy() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_connection(&dir.path().join("a.sqlite"), true).unwrap();
        conn.execute_batch(
            "CREATE TABLE t(id TEXT PRIMARY KEY, v TEXT); INSERT INTO t VALUES('1','x');",
        )
        .unwrap();
        let dest = dir.path().join("b.sqlite");
        backup_to(&conn, &dest).unwrap();
        let copy = Connection::open(&dest).unwrap();
        let v: String = copy.query_row("SELECT v FROM t", [], |r| r.get(0)).unwrap();
        assert_eq!(v, "x");
        assert!(!dest.with_extension("partial").exists());
    }

    #[test]
    fn missing_database_without_create_fails_humanely() {
        let dir = tempfile::tempdir().unwrap();
        let err = open_connection(&dir.path().join("missing.sqlite"), false).unwrap_err();
        assert!(err.is("storage"));
    }
}
