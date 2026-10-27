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
    writable_side_files(path);
    let conn = open_sqlite(path, flags)?;
    // On Windows SQLite quietly falls back to a read-only handle when the file can't be
    // opened for writing (read-only attribute, or another program holding it without write
    // sharing). OpenFrame asked to write, so a read-only handle would only fail later on the
    // first change. Refuse here with the reason instead.
    if conn.is_readonly("main")? {
        drop(conn);
        let read_only_attribute = std::fs::metadata(path)
            .map(|m| m.permissions().readonly())
            .unwrap_or(false);
        return Err(if read_only_attribute {
            AppError::new(
                "storage.access_denied",
                "This file is marked read-only, so OpenFrame can't save to it. Clear the read-only setting in the file's properties, or copy it to a folder you can write to, then try again. Nothing was changed.",
            )
            .with_detail("database file has the read-only attribute")
        } else {
            AppError::file_locked("database file could only be opened read-only")
        });
    }
    apply_pragmas(&conn)?;
    Ok(conn)
}

/// On macOS and Linux SQLite creates `-wal` and `-shm` with the database file's permissions, so a
/// reader that opened the database while it was read-only leaves read-only side files behind. Once
/// the database itself is writable again they would still refuse every write ("attempt to write a
/// readonly database"). Give them back the owner's write permission, and nothing else; a database
/// that is itself read-only is left alone and refused below with its own message.
fn writable_side_files(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let writable = |p: &Path| {
            std::fs::symlink_metadata(p)
                .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o200 != 0)
        };
        if !writable(path) {
            return;
        }
        for suffix in ["-wal", "-shm"] {
            let mut side = path.as_os_str().to_os_string();
            side.push(suffix);
            let side = std::path::PathBuf::from(side);
            let Ok(meta) = std::fs::symlink_metadata(&side) else {
                continue;
            };
            let mode = meta.permissions().mode();
            if meta.is_file() && mode & 0o200 == 0 {
                let _ =
                    std::fs::set_permissions(&side, std::fs::Permissions::from_mode(mode | 0o200));
            }
        }
    }
    #[cfg(not(unix))]
    let _ = path;
}

/// `Connection::open_with_flags` that also works for paths longer than `MAX_PATH` on Windows.
///
/// The platform boundary says when a path needs another form (`openframe_platform::paths`): on
/// Windows a path of 240 characters or more is opened through the `win32-longpath` VFS with the
/// absolute extended-length form. Short paths are passed as they are, so error details and
/// behaviour stay the same for everyone else.
pub fn open_sqlite(path: &Path, flags: OpenFlags) -> rusqlite::Result<Connection> {
    if let Some((long, vfs)) = openframe_platform::paths::sqlite_long_path(path) {
        return Connection::open_with_flags_and_vfs(long, flags, vfs);
    }
    Connection::open_with_flags(path, flags)
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
/// safety copies). Writes a uniquely named staging file in the destination folder
/// (`openframe_platform::atomic::unique_temp_path`), verifies it, then renames it over the
/// destination with the shared atomic replace, so a crash never leaves a half-written backup
/// under the final name.
///
/// The destination folder is created when it is missing (a cleanup tool or sync client may
/// delete `recovery/` while a project is open). On any failure the staging file and its SQLite
/// side files are removed, so nothing is left beside the destination.
pub fn backup_to(conn: &Connection, dest: &Path) -> AppResult<()> {
    let parent = dest.parent().unwrap_or(Path::new(""));
    if !parent.as_os_str().is_empty() {
        std::fs::create_dir_all(parent)?;
    }
    // Earlier versions staged as `<name>.partial`; one left by a crash is ours to remove.
    let legacy = dest.with_extension("partial");
    if std::fs::symlink_metadata(&legacy).is_ok_and(|m| m.is_file()) {
        let _ = std::fs::remove_file(&legacy);
    }
    let tmp = openframe_platform::atomic::unique_temp_path(parent, dest)?;
    let result = write_verified_copy(conn, &tmp).and_then(|()| {
        // The rename replaces an existing destination atomically; deleting first would open a
        // crash window with no backup at `dest` (security baseline 2026-10-05, E5).
        openframe_platform::atomic::replace_from_staged(&tmp, dest).map_err(AppError::from)
    });
    if result.is_err() {
        remove_staging(&tmp);
    }
    result
}

/// Remove a staging database and the side files SQLite may have left next to it.
fn remove_staging(tmp: &Path) {
    let _ = std::fs::remove_file(tmp);
    for suffix in ["-journal", "-wal", "-shm"] {
        let mut side = tmp.as_os_str().to_owned();
        side.push(suffix);
        let _ = std::fs::remove_file(std::path::PathBuf::from(side));
    }
}

/// Copy `conn` into `tmp` with the online backup API and verify the copy.
fn write_verified_copy(conn: &Connection, tmp: &Path) -> AppResult<()> {
    {
        let mut out = open_sqlite(tmp, OpenFlags::default())?;
        let backup = rusqlite::backup::Backup::new(conn, &mut out)?;
        backup.run_to_completion(256, Duration::from_millis(0), None)?;
    }
    let check = open_sqlite(tmp, OpenFlags::default())?;
    check.execute_batch("PRAGMA journal_mode=DELETE;")?;
    let problems = integrity_problems(&check, false)?;
    if !problems.is_empty() {
        return Err(AppError::storage(format!(
            "backup verification failed: {}",
            problems.join("; ")
        )));
    }
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

    /// No staging file (current or legacy name) or SQLite side file is left in `dir`.
    fn assert_no_staging(dir: &Path) {
        let left: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| {
                n.contains(openframe_platform::atomic::TEMP_SUFFIX) || n.ends_with(".partial")
            })
            .collect();
        assert!(left.is_empty(), "staging files left behind: {left:?}");
    }

    /// macOS: a reader that opened the database while it was read-only left a read-only `-shm`
    /// (SQLite copies the database's permissions). Once the database is writable it opens for
    /// writing again; a database that is still read-only is refused as before.
    #[cfg(unix)]
    #[test]
    fn read_only_side_files_of_a_writable_database_do_not_block_writes() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("project.sqlite");
        {
            let c = open_connection(&db, true).unwrap();
            c.execute_batch("CREATE TABLE t(x); INSERT INTO t VALUES (1);")
                .unwrap();
        }
        let mode = |p: &Path, m: u32| {
            std::fs::set_permissions(p, std::fs::Permissions::from_mode(m)).unwrap()
        };
        mode(&db, 0o444);
        assert_eq!(
            open_connection(&db, false).unwrap_err().code_str(),
            "storage.access_denied"
        );
        {
            let reader =
                Connection::open_with_flags(&db, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
            let _: i64 = reader
                .query_row("SELECT x FROM t", [], |r| r.get(0))
                .unwrap();
            let mut shm = db.as_os_str().to_os_string();
            shm.push("-shm");
            let shm = std::path::PathBuf::from(shm);
            if shm.exists() {
                mode(&shm, 0o444);
            }
        }
        mode(&db, 0o644);
        let c = open_connection(&db, false).unwrap();
        c.execute("INSERT INTO t VALUES (2)", []).unwrap();
        let n: i64 = c
            .query_row("SELECT count(*) FROM t", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 2);
    }

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
        assert_no_staging(dir.path());
    }

    #[test]
    fn backup_over_an_existing_copy_replaces_it() {
        // `rename` replaces the destination in one step; no delete-then-rename window.
        let dir = tempfile::tempdir().unwrap();
        let conn = open_connection(&dir.path().join("a.sqlite"), true).unwrap();
        conn.execute_batch(
            "CREATE TABLE t(id TEXT PRIMARY KEY, v TEXT); INSERT INTO t VALUES('1','x');",
        )
        .unwrap();
        let dest = dir.path().join("b.sqlite");
        backup_to(&conn, &dest).unwrap();
        conn.execute("UPDATE t SET v='y'", []).unwrap();
        backup_to(&conn, &dest).unwrap();
        let copy = Connection::open(&dest).unwrap();
        let v: String = copy.query_row("SELECT v FROM t", [], |r| r.get(0)).unwrap();
        assert_eq!(v, "y");
        assert_no_staging(dir.path());
    }

    #[test]
    fn backup_into_a_deleted_folder_recreates_it() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_connection(&dir.path().join("a.sqlite"), true).unwrap();
        conn.execute_batch("CREATE TABLE t(id TEXT PRIMARY KEY);")
            .unwrap();
        let dest = dir.path().join("recovery").join("checkpoint.sqlite");
        backup_to(&conn, &dest).unwrap();
        assert!(dest.is_file());
        assert_no_staging(&dir.path().join("recovery"));
    }

    #[test]
    fn a_legacy_partial_file_from_a_crash_is_removed() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_connection(&dir.path().join("a.sqlite"), true).unwrap();
        conn.execute_batch("CREATE TABLE t(id TEXT PRIMARY KEY);")
            .unwrap();
        let dest = dir.path().join("checkpoint.sqlite");
        std::fs::write(dest.with_extension("partial"), b"half written").unwrap();
        backup_to(&conn, &dest).unwrap();
        assert!(dest.is_file());
        assert_no_staging(dir.path());
    }

    #[cfg(windows)]
    #[test]
    fn database_deeper_than_max_path_opens_and_backs_up() {
        let dir = tempfile::tempdir().unwrap();
        let mut deep = dir.path().to_path_buf();
        while deep.as_os_str().len() < 280 {
            deep.push("a folder with a long name for path tests");
        }
        std::fs::create_dir_all(&deep).unwrap();
        let conn = open_connection(&deep.join("project.sqlite"), true).unwrap();
        conn.execute_batch("CREATE TABLE t(id TEXT PRIMARY KEY); INSERT INTO t VALUES('1');")
            .unwrap();
        let dest = deep.join("recovery").join("checkpoint.sqlite");
        backup_to(&conn, &dest).unwrap();
        assert!(dest.is_file());
    }

    #[cfg(windows)]
    #[test]
    fn a_database_another_program_locks_is_refused_not_opened_read_only() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("p.sqlite");
        drop(open_connection(&path, true).unwrap());
        let _lock = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0x1) // FILE_SHARE_READ only
            .open(&path)
            .unwrap();
        let err = open_connection(&path, false).unwrap_err();
        assert_eq!(err.code_str(), "storage.file_locked");
        assert!(err.retryable);
    }

    #[test]
    fn failed_rename_leaves_no_staging_file() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_connection(&dir.path().join("a.sqlite"), true).unwrap();
        conn.execute_batch("CREATE TABLE t(id TEXT PRIMARY KEY);")
            .unwrap();
        // A folder at the destination name makes the final rename fail.
        let dest = dir.path().join("b.sqlite");
        std::fs::create_dir(&dest).unwrap();
        assert!(backup_to(&conn, &dest).is_err());
        assert_no_staging(dir.path());
        assert!(dest.is_dir());
    }

    #[test]
    fn missing_database_without_create_fails_humanely() {
        let dir = tempfile::tempdir().unwrap();
        let err = open_connection(&dir.path().join("missing.sqlite"), false).unwrap_err();
        assert!(err.is("storage"));
    }
}
