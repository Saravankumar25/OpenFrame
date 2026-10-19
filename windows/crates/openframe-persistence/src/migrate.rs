//! Schema migrations (Release/Migration spec; ESD locked decision "safety backup
//! first, then transactional migration with rollback/recovery path").
//!
//! The caller (project lifecycle) is responsible for taking the safety backup
//! *before* calling [`apply`] on an existing database. `apply` itself runs all
//! pending migrations inside ONE transaction, validates foreign keys and
//! structural integrity, and only then commits. A failure leaves the database
//! exactly as it was.

use openframe_domain::{AppError, AppResult, now_ms};
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy)]
pub struct Migration {
    pub version: u32,
    pub name: &'static str,
    pub sql: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationPlan {
    /// Brand-new database; all migrations will run.
    Fresh {
        to: u32,
    },
    UpToDate {
        version: u32,
    },
    /// Existing data will be migrated: a safety backup is required first.
    Upgrade {
        from: u32,
        to: u32,
    },
    /// Written by a newer OpenFrame. Must not be opened for writing.
    TooNew {
        found: u32,
        supported: u32,
    },
}

fn checksum(sql: &str) -> String {
    hex::encode(Sha256::digest(sql.as_bytes()))
}

fn latest(migrations: &[Migration]) -> u32 {
    migrations.iter().map(|m| m.version).max().unwrap_or(0)
}

fn validate_sequence(migrations: &[Migration]) -> AppResult<()> {
    for (i, m) in migrations.iter().enumerate() {
        if m.version != (i as u32) + 1 {
            return Err(AppError::internal(format!(
                "migration list must be contiguous from 1; found {} at position {}",
                m.version, i
            )));
        }
    }
    Ok(())
}

pub fn current_version(conn: &Connection) -> AppResult<u32> {
    Ok(conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))? as u32)
}

pub fn plan(conn: &Connection, migrations: &[Migration]) -> AppResult<MigrationPlan> {
    validate_sequence(migrations)?;
    let found = current_version(conn)?;
    let supported = latest(migrations);
    Ok(if found == 0 {
        MigrationPlan::Fresh { to: supported }
    } else if found == supported {
        MigrationPlan::UpToDate { version: found }
    } else if found > supported {
        MigrationPlan::TooNew { found, supported }
    } else {
        MigrationPlan::Upgrade {
            from: found,
            to: supported,
        }
    })
}

pub fn too_new_error(found: u32, supported: u32) -> AppError {
    AppError::new(
        "project_format.too_new",
        "This project was saved by a newer version of OpenFrame. Update OpenFrame to open it. The project has not been changed.",
    )
    .with_detail(format!("schema {found} > supported {supported}"))
}

/// Apply all pending migrations atomically. Returns the versions applied.
pub fn apply(conn: &mut Connection, migrations: &[Migration]) -> AppResult<Vec<u32>> {
    match plan(conn, migrations)? {
        MigrationPlan::TooNew { found, supported } => return Err(too_new_error(found, supported)),
        MigrationPlan::UpToDate { .. } => {
            verify_checksums(conn, migrations)?;
            return Ok(vec![]);
        }
        _ => {}
    }

    let from = current_version(conn)?;
    let to = latest(migrations);
    // Foreign keys cannot be toggled inside a transaction; table-rebuild
    // migrations need them off, then we re-validate explicitly before commit.
    conn.execute_batch("PRAGMA foreign_keys=OFF;")?;
    let result = (|| -> AppResult<Vec<u32>> {
        let tx = conn.transaction()?;
        tx.execute_batch(
            "CREATE TABLE IF NOT EXISTS _schema_migrations(
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                checksum TEXT NOT NULL,
                applied_at INTEGER NOT NULL
            );",
        )?;
        let mut applied = Vec::new();
        for m in migrations.iter().filter(|m| m.version > from) {
            tx.execute_batch(m.sql)
                .and_then(|()| fault::check(fault::DiskFullAt::Step(m.version)))
                .map_err(|e| step_error(m, e))?;
            tx.execute(
                "INSERT INTO _schema_migrations(version, name, checksum, applied_at) VALUES (?1, ?2, ?3, ?4)",
                params![m.version, m.name, checksum(m.sql), now_ms()],
            )?;
            applied.push(m.version);
        }
        let fk = crate::foreign_key_problems(&tx)?;
        if !fk.is_empty() {
            return Err(AppError::migration(format!(
                "foreign key check failed: {}",
                fk.join(", ")
            )));
        }
        let integrity = crate::integrity_problems(&tx, false)?;
        if !integrity.is_empty() {
            return Err(AppError::migration(format!(
                "integrity check failed: {}",
                integrity.join("; ")
            )));
        }
        tx.pragma_update(None, "user_version", to as i64)?;
        fault::check(fault::DiskFullAt::Commit)?;
        tx.commit()?;
        Ok(applied)
    })();
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;
    result
}

/// A failed migration statement. A full drive or a read-only file keeps its own code (the
/// user must fix the drive, not the project); anything else is `migration.failed`. The SQLite
/// text goes to the detail for support either way.
fn step_error(m: &Migration, e: rusqlite::Error) -> AppError {
    let detail = format!("migration {} ({}) failed: {e}", m.version, m.name);
    let mapped = AppError::from(e);
    if mapped.is("storage.disk_full") || mapped.is("storage.access_denied") {
        mapped.with_detail(detail)
    } else {
        AppError::migration(detail)
    }
}

/// Which data a migration or its safety copy belongs to, for the words the user sees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subject {
    /// A project's `project.sqlite`.
    Project,
    /// The Global Idea Vault database.
    GlobalIdeaVault,
    /// OpenFrame's own `app.sqlite` (settings, recent projects).
    AppData,
}

impl Subject {
    fn noun(self) -> &'static str {
        match self {
            Subject::Project => "this project",
            Subject::GlobalIdeaVault => "the Global Idea Vault",
            Subject::AppData => "its own app data",
        }
    }
}

/// The error a user sees when [`apply`] failed while upgrading existing data: a full drive and
/// a read-only file or folder each say so in plain words. Other errors pass through unchanged.
pub fn upgrade_failure(err: AppError, subject: Subject) -> AppError {
    let noun = subject.noun();
    let detail = err.detail.clone().unwrap_or_default();
    if err.is("storage.disk_full") {
        AppError::new(
            "storage.disk_full",
            format!(
                "The drive is full, so OpenFrame could not upgrade {noun}. Free up some space and try again. Nothing was changed."
            ),
        )
        .with_detail(detail)
        .retryable()
    } else if err.is("storage.access_denied") {
        AppError::new(
            "storage.access_denied",
            format!(
                "OpenFrame could not upgrade {noun} because its file or folder is read-only or OpenFrame lacks permission to write there. Move it to a folder you can write to, or check its permissions, then try again. Nothing was changed."
            ),
        )
        .with_detail(detail)
    } else {
        err
    }
}

/// The error a user sees when the safety copy taken before a migration could not be written.
/// The migration never starts in that case, so the data is untouched.
pub fn backup_failure(err: AppError, subject: Subject) -> AppError {
    let noun = subject.noun();
    let detail = match &err.detail {
        Some(d) => format!("{}: {d}", err.code_str()),
        None => err.code_str().to_string(),
    };
    if err.is("storage.disk_full") {
        AppError::new(
            "storage.disk_full",
            format!(
                "The drive is full, so OpenFrame could not make the safety copy it takes before it can upgrade {noun}. Free up some space and try again. Nothing was changed."
            ),
        )
        .with_detail(detail)
        .retryable()
    } else if err.is("storage.access_denied") {
        AppError::new(
            "storage.access_denied",
            format!(
                "OpenFrame could not make the safety copy it takes before it can upgrade {noun}, because it lacks permission to write to the backups folder. Check the folder's permissions and try again. Nothing was changed."
            ),
        )
        .with_detail(detail)
    } else {
        AppError::new(
            "storage.backup_failed",
            format!(
                "OpenFrame could not make the safety copy it takes before it can upgrade {noun}, so it did not upgrade it. Nothing was changed. Check that the folder can be written to and try again."
            ),
        )
        .with_detail(detail)
        .retryable()
    }
}

/// Debug-only fault injection for the migration failure fixtures (build plan item migration failure cases,
/// `crates/openframe-application/tests/migration_failures.rs`). Same pattern as
/// `openframe_application::modules::ai::change_set::fault`: a test arms a point on the thread
/// that runs [`apply`], the point fires once with the error SQLite itself returns when the drive
/// is full (`SQLITE_FULL`), and release builds compile it to a no-op.
pub mod fault {
    /// Where inside the migration transaction the simulated full drive is hit.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DiskFullAt {
        /// Right after migration `version` ran its SQL inside the transaction.
        Step(u32),
        /// After every step, check and the version bump, as the transaction commits.
        Commit,
    }

    #[cfg(debug_assertions)]
    thread_local! {
        static ARMED: std::cell::Cell<Option<(DiskFullAt, bool)>> = const { std::cell::Cell::new(None) };
    }

    /// Arm (`Some(point)`) or disarm (`None`) the simulated full drive for this thread.
    #[cfg(debug_assertions)]
    pub fn disk_full_at(point: Option<DiskFullAt>) {
        ARMED.with(|c| c.set(point.map(|p| (p, false))));
    }

    /// Arm (`Some(point)`) or disarm (`None`) a simulated read-only file (`SQLITE_READONLY`).
    #[cfg(debug_assertions)]
    pub fn read_only_at(point: Option<DiskFullAt>) {
        ARMED.with(|c| c.set(point.map(|p| (p, true))));
    }

    pub(super) fn check(point: DiskFullAt) -> rusqlite::Result<()> {
        #[cfg(debug_assertions)]
        if let Some((armed, read_only)) = ARMED.with(|c| c.get())
            && armed == point
        {
            ARMED.with(|c| c.set(None));
            let (code, text) = if read_only {
                (
                    rusqlite::ffi::SQLITE_READONLY,
                    "attempt to write a readonly database",
                )
            } else {
                (rusqlite::ffi::SQLITE_FULL, "database or disk is full")
            };
            return Err(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(code),
                Some(text.into()),
            ));
        }
        let _ = point;
        Ok(())
    }
}

/// Detect edited shipped migrations. Logged, never fatal: refusing to open a
/// user's project over a developer mistake would violate data ownership.
fn verify_checksums(conn: &Connection, migrations: &[Migration]) -> AppResult<()> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='_schema_migrations')",
        [],
        |r| r.get(0),
    )?;
    if !exists {
        return Ok(());
    }
    let mut stmt = conn.prepare("SELECT version, checksum FROM _schema_migrations")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, u32>(0)?, r.get::<_, String>(1)?)))?;
    for row in rows {
        let (version, sum) = row?;
        if let Some(m) = migrations.iter().find(|m| m.version == version)
            && checksum(m.sql) != sum
        {
            tracing::warn!(
                version,
                name = m.name,
                "applied migration checksum differs from shipped migration"
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const M1: Migration = Migration {
        version: 1,
        name: "init",
        sql: "CREATE TABLE a(id TEXT PRIMARY KEY);",
    };
    const M2: Migration = Migration {
        version: 2,
        name: "b",
        sql: "CREATE TABLE b(id TEXT PRIMARY KEY, a_id TEXT REFERENCES a(id));",
    };
    const BAD: Migration = Migration {
        version: 2,
        name: "bad",
        sql: "CREATE TABLE c(id TEXT PRIMARY KEY); SELECT * FROM nope;",
    };

    #[test]
    fn fresh_then_up_to_date() {
        let mut c = crate::open_in_memory().unwrap();
        assert_eq!(plan(&c, &[M1, M2]).unwrap(), MigrationPlan::Fresh { to: 2 });
        assert_eq!(apply(&mut c, &[M1, M2]).unwrap(), vec![1, 2]);
        assert_eq!(
            plan(&c, &[M1, M2]).unwrap(),
            MigrationPlan::UpToDate { version: 2 }
        );
        assert!(apply(&mut c, &[M1, M2]).unwrap().is_empty());
    }

    #[test]
    fn upgrade_only_runs_pending() {
        let mut c = crate::open_in_memory().unwrap();
        apply(&mut c, &[M1]).unwrap();
        assert_eq!(
            plan(&c, &[M1, M2]).unwrap(),
            MigrationPlan::Upgrade { from: 1, to: 2 }
        );
        assert_eq!(apply(&mut c, &[M1, M2]).unwrap(), vec![2]);
    }

    #[test]
    fn failed_migration_rolls_back_completely() {
        let mut c = crate::open_in_memory().unwrap();
        apply(&mut c, &[M1]).unwrap();
        let err = apply(&mut c, &[M1, BAD]).unwrap_err();
        assert!(err.is("migration"));
        assert_eq!(current_version(&c).unwrap(), 1);
        let has_c: bool = c
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='c')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(!has_c, "partial migration must not persist");
        let fk: i64 = c
            .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
            .unwrap();
        assert_eq!(fk, 1, "foreign keys restored after failure");
    }

    #[test]
    fn too_new_database_is_refused() {
        let mut c = crate::open_in_memory().unwrap();
        apply(&mut c, &[M1, M2]).unwrap();
        let err = apply(&mut c, &[M1]).unwrap_err();
        assert_eq!(err.code_str(), "project_format.too_new");
    }

    #[cfg(debug_assertions)]
    #[test]
    fn simulated_full_drive_rolls_the_migration_back_at_every_point() {
        use fault::DiskFullAt;
        for (point, code) in [
            (DiskFullAt::Step(2), "storage.disk_full"),
            (DiskFullAt::Commit, "storage.disk_full"),
        ] {
            let mut c = crate::open_in_memory().unwrap();
            apply(&mut c, &[M1]).unwrap();
            fault::disk_full_at(Some(point));
            let err = apply(&mut c, &[M1, M2]).unwrap_err();
            fault::disk_full_at(None);
            assert_eq!(err.code_str(), code, "{point:?}");
            assert!(
                format!("{:?}", err.detail).contains("disk is full"),
                "{point:?}: {err:?}"
            );
            assert_eq!(current_version(&c).unwrap(), 1, "{point:?}");
            let has_b: bool = c
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='b')",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert!(!has_b, "{point:?}: nothing from the failed run persists");
            // The point fired once: the next attempt succeeds.
            assert_eq!(apply(&mut c, &[M1, M2]).unwrap(), vec![2], "{point:?}");
        }
    }

    /// A full or read-only drive met while migrating keeps a specific code and says what
    /// happened in plain words, instead of the general "could not upgrade" message.
    #[cfg(debug_assertions)]
    #[test]
    fn full_or_read_only_drive_during_migration_says_so() {
        use fault::DiskFullAt;
        for point in [DiskFullAt::Step(2), DiskFullAt::Commit] {
            let mut c = crate::open_in_memory().unwrap();
            apply(&mut c, &[M1]).unwrap();
            fault::disk_full_at(Some(point));
            let err = upgrade_failure(apply(&mut c, &[M1, M2]).unwrap_err(), Subject::Project);
            fault::disk_full_at(None);
            assert_eq!(err.code_str(), "storage.disk_full", "{point:?}");
            assert!(err.message.contains("drive is full"), "{err:?}");
            assert!(err.message.contains("upgrade this project"), "{err:?}");
            assert!(err.retryable);

            let mut c = crate::open_in_memory().unwrap();
            apply(&mut c, &[M1]).unwrap();
            fault::read_only_at(Some(point));
            let err = upgrade_failure(apply(&mut c, &[M1, M2]).unwrap_err(), Subject::Project);
            fault::read_only_at(None);
            assert_eq!(err.code_str(), "storage.access_denied", "{point:?}");
            assert!(err.message.contains("read-only"), "{err:?}");
            assert!(
                format!("{:?}", err.detail).contains("readonly"),
                "the cause reaches support: {err:?}"
            );
            assert_eq!(current_version(&c).unwrap(), 1);
        }
    }

    #[test]
    fn other_migration_errors_keep_the_general_message() {
        let mut c = crate::open_in_memory().unwrap();
        apply(&mut c, &[M1]).unwrap();
        let err = upgrade_failure(apply(&mut c, &[M1, BAD]).unwrap_err(), Subject::Project);
        assert_eq!(err.code_str(), "migration.failed");
    }

    #[test]
    fn safety_backup_failures_say_the_backup_failed() {
        let full = backup_failure(AppError::disk_full(), Subject::Project);
        assert_eq!(full.code_str(), "storage.disk_full");
        assert!(full.message.contains("safety copy"), "{full:?}");
        let denied = backup_failure(AppError::access_denied("x"), Subject::AppData);
        assert_eq!(denied.code_str(), "storage.access_denied");
        assert!(denied.message.contains("safety copy"), "{denied:?}");
        let other = backup_failure(AppError::storage("io"), Subject::GlobalIdeaVault);
        assert_eq!(other.code_str(), "storage.backup_failed");
        assert!(other.message.contains("safety copy"), "{other:?}");
        assert!(!other.message.contains("save this change"), "{other:?}");
        for e in [&full, &denied, &other] {
            assert!(e.message.contains("Nothing was changed"), "{e:?}");
            assert!(!e.message.contains('\u{2014}'), "{e:?}");
        }
    }

    #[test]
    fn non_contiguous_list_rejected() {
        let c = crate::open_in_memory().unwrap();
        assert!(plan(&c, &[M2]).is_err());
    }
}
