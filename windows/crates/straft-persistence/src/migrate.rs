//! Schema migrations (Release/Migration spec; ESD locked decision "safety backup
//! first, then transactional migration with rollback/recovery path").
//!
//! The caller (project lifecycle) is responsible for taking the safety backup
//! *before* calling [`apply`] on an existing database. `apply` itself runs all
//! pending migrations inside ONE transaction, validates foreign keys and
//! structural integrity, and only then commits. A failure leaves the database
//! exactly as it was.

use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use straft_domain::{AppError, AppResult, now_ms};

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
    /// Written by a newer Straft. Must not be opened for writing.
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
        "This project was saved by a newer version of Straft. Update Straft to open it. The project has not been changed.",
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
    /// Straft's own `app.sqlite` (settings, recent projects).
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
                "The drive is full, so Straft could not upgrade {noun}. Free up some space and try again. Nothing was changed."
            ),
        )
        .with_detail(detail)
        .retryable()
    } else if err.is("storage.access_denied") {
        AppError::new(
            "storage.access_denied",
            format!(
                "Straft could not upgrade {noun} because its file or folder is read-only or Straft lacks permission to write there. Move it to a folder you can write to, or check its permissions, then try again. Nothing was changed."
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
                "The drive is full, so Straft could not make the safety copy it takes before it can upgrade {noun}. Free up some space and try again. Nothing was changed."
            ),
        )
        .with_detail(detail)
        .retryable()
    } else if err.is("storage.access_denied") {
        AppError::new(
            "storage.access_denied",
            format!(
                "Straft could not make the safety copy it takes before it can upgrade {noun}, because it lacks permission to write to the backups folder. Check the folder's permissions and try again. Nothing was changed."
            ),
        )
        .with_detail(detail)
    } else {
        AppError::new(
            "storage.backup_failed",
            format!(
                "Straft could not make the safety copy it takes before it can upgrade {noun}, so it did not upgrade it. Nothing was changed. Check that the folder can be written to and try again."
            ),
        )
        .with_detail(detail)
        .retryable()
    }
}

/// Debug-only fault injection for the migration failure fixtures (build plan item migration failure cases,
/// `crates/straft-application/tests/migration_failures.rs`). Same pattern as
/// `straft_application::modules::ai::change_set::fault`: a test arms a point on the thread
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

/// The migrations whose comments changed when the product was renamed. Only comments changed, so a
/// database that ran the earlier text has exactly the schema the new text gives.
struct RenamedMigration {
    version: u32,
    /// SHA-256 of the text this build ships (line endings as LF). An entry applies to that text
    /// only: editing the migration again ends the exception.
    now: &'static str,
    /// SHA-256 of the earlier text, as databases recorded it.
    before: &'static str,
    /// The same earlier text with CRLF line endings: a Windows checkout may compile it that way
    /// (`scripts/check-migrations.mjs` treats the two as equal for the same reason).
    before_crlf: &'static str,
}

// LEGACY: Open Frame name kept for migration
const RENAMED_MIGRATIONS: &[RenamedMigration] = &[
    // app/0001_app.sql
    RenamedMigration {
        version: 1,
        now: "16b7d4b6019a22e657759a213836c25a6048f1aea7a8203d735c7886b289f9eb",
        before: "fd3799b9f366a82ac79528f5bd576f7953d06eff9b9801ff399e35fb517fb24d",
        before_crlf: "b3d799624f4eab8fd0c142b1fb0d1140b5e817a0fbe83b60cf6e56907a6cc741",
    },
    // app/0002_appearance.sql
    RenamedMigration {
        version: 2,
        now: "faa57be4504cfd37e038ec49dfcf296796b3ac3714f89c5f5306d4c54288f2c6",
        before: "149a096edab8b03013e9d1c564069ba0feefad9a6abb9d37ce5d6250e594af7f",
        before_crlf: "735139ec1f7fa89ca1a3fa7e17e7e894fc16f2f46c9a080c3c1ebfc5b5efc56c",
    },
    // global/0001_core.sql
    RenamedMigration {
        version: 1,
        now: "b2fc89d194836a131442d244614b7cdb0224a709ff0c680245da26570aef7f56",
        before: "709e840386976c263df6cbe73f63b000ed7b689f38f3e0c5e1c23f7706e81b7a",
        before_crlf: "90ebe88ef0e2a446b2c7e21deddcbb1e04f41d0ba0d8cd1015d16d8cde48fc79",
    },
    // global/0002_idea_vault.sql
    RenamedMigration {
        version: 2,
        now: "4e89401ff4a0ddbdcbc40a9756a0c3b4e70289b48fa43c0815add5e1fb68db4d",
        before: "a112a6a1607084a24e61d0cb8aff0e58f672cb5e11376fcd52c5f6c74b88d7be",
        before_crlf: "291702aed1df8ffee278ce63ecde50ef47412f8e47c104a56540d6065dc4ac37",
    },
    // global/0004_vault_research.sql
    RenamedMigration {
        version: 4,
        now: "e6431af54a5cff20a46682dcd9258fa20168067982a4ffb20aa8672e4effd5ce",
        before: "3c7f9e7080b746dc0f623b8f18d82d0fdfe38a6e2482ea4d6b83f4c5ac5d4bd5",
        before_crlf: "50067fe770131f2b3fa61f0cff38543425f7337ede85e624cdfcb0e4ef6e4a4e",
    },
    // project/0001_core.sql
    RenamedMigration {
        version: 1,
        now: "c43b8193a0aa45800803dfc369435822ee5a019be8fcfcd83d464ea93e67ec45",
        before: "b1499b505d063f8e1f129f446abcb440f8b5a73b9fbab48d48ee897f6672099a",
        before_crlf: "24500b7cd88d2370629cd8fa63ce4580d98e4f6134e39f9a005525cd9ebe66d4",
    },
    // project/0002_idea_vault.sql
    RenamedMigration {
        version: 2,
        now: "d0cf07e462dfe401cc55e626f54c27c45e5ad2b6cd1dae02bfaff798e5860156",
        before: "2cad662bb14dbe4f0f95a24704026abc4c092d3916f5b720df2a3754dddf0f1c",
        before_crlf: "32a9fc31b60a7e3882437c9064d8b6aa2b6ec4a3ccd9ae4e7f55fed94fb95f44",
    },
    // project/0006_visual.sql
    RenamedMigration {
        version: 6,
        now: "d8541966acfc3133602fce2c7d67b0443a6ad44bda1ec3770b7958036985f61c",
        before: "fa89406361b681dcd5d077f7942eaa44984315fdb56dd77a0ed520bcc8099567",
        before_crlf: "fb31c4fcf1385f9b1eacdc197dc5f57694907d2b6e448e8107bf50769f6eb473",
    },
    // project/0010_performance.sql
    RenamedMigration {
        version: 10,
        now: "742ef816391965e3a6780c4d2c021ba662812dd074e2387b43237aa82356f12c",
        before: "516eed482685e5c0acc13490a0de6fb964f631044b02790f5b76961644057c51",
        before_crlf: "5820cf04413373edf45612a90019c9867bd74bbf65efa47faf2425cd8cb34e95",
    },
    // project/0012_story_card_and_vault_research.sql
    RenamedMigration {
        version: 12,
        now: "7992d5832d6828d690a1314bba82f5d1c120425ca7c65013bbce6e643c5f03c2",
        before: "6eabf04061f122affcd50f0a7893c5a15d73111fb0103c5e3da832244b1bd765",
        before_crlf: "578e046941c500d57a2ca070d0fa54ab4b606ffd0630da63dd53bba8a2c9e890",
    },
    // project/0019_story_page_format.sql
    RenamedMigration {
        version: 19,
        now: "446da16728e08f57d83a6d9943375ddd28b9dc209e74366dea2bdd31281e97e5",
        before: "7388aae3181554f749cfa4a4d88bdfb7c51132f535f3f15eb50c89c4ba9c5a8f",
        before_crlf: "b577407e67f75368971ded60f38c07be3c30e44d4d49cfc5cfdfe03b7f40a7e1",
    },
];

fn lf_checksum(sql: &str) -> String {
    checksum(&sql.replace("\r\n", "\n"))
}

/// A recorded checksum the shipped text may differ from, because the rename touched its comments.
fn renamed_from(m: &Migration, recorded: &str) -> bool {
    RENAMED_MIGRATIONS.iter().any(|r| {
        r.version == m.version
            && (r.before == recorded || r.before_crlf == recorded)
            && r.now == lf_checksum(m.sql)
    })
}

/// Applied migrations whose recorded checksum differs from the shipped text, leaving out the ones
/// the product rename explains ([`RENAMED_MIGRATIONS`]).
pub fn checksum_mismatches(conn: &Connection, migrations: &[Migration]) -> AppResult<Vec<u32>> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='_schema_migrations')",
        [],
        |r| r.get(0),
    )?;
    if !exists {
        return Ok(Vec::new());
    }
    let mut stmt = conn.prepare("SELECT version, checksum FROM _schema_migrations")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, u32>(0)?, r.get::<_, String>(1)?)))?;
    let mut wrong = Vec::new();
    for row in rows {
        let (version, sum) = row?;
        if let Some(m) = migrations.iter().find(|m| m.version == version)
            && checksum(m.sql) != sum
            && !renamed_from(m, &sum)
        {
            wrong.push(version);
        }
    }
    Ok(wrong)
}

/// Detect edited shipped migrations. Logged, never fatal: refusing to open a
/// user's project over a developer mistake would violate data ownership.
fn verify_checksums(conn: &Connection, migrations: &[Migration]) -> AppResult<()> {
    for version in checksum_mismatches(conn, migrations)? {
        let name = migrations
            .iter()
            .find(|m| m.version == version)
            .map_or("", |m| m.name);
        tracing::warn!(
            version,
            name,
            "applied migration checksum differs from shipped migration"
        );
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

    /// The real text of `migrations/app/0002_appearance.sql` as earlier versions ran it. Only the
    /// default theme's name in one comment differs.
    fn earlier_appearance_sql() -> String {
        include_str!("../../../migrations/app/0002_appearance.sql").replace(
            "Straft Default",
            &format!("{} Default", straft_platform::legacy_names::PRODUCT_NAME),
        )
    }

    fn appearance(sql: &'static str) -> Migration {
        Migration {
            version: 2,
            name: "appearance",
            sql,
        }
    }

    const APPEARANCE_NOW: &str = include_str!("../../../migrations/app/0002_appearance.sql");

    #[test]
    fn a_database_that_ran_the_earlier_text_opens_with_no_mismatch() {
        let earlier: &'static str = Box::leak(earlier_appearance_sql().into_boxed_str());
        assert_eq!(
            checksum(earlier),
            "149a096edab8b03013e9d1c564069ba0feefad9a6abb9d37ce5d6250e594af7f",
            "the reconstructed text is byte for byte what earlier versions shipped"
        );
        assert_ne!(checksum(earlier), checksum(APPEARANCE_NOW));
        let mut c = crate::open_in_memory().unwrap();
        apply(&mut c, &[M1, appearance(earlier)]).unwrap();
        let shipped = [M1, appearance(APPEARANCE_NOW)];
        assert_eq!(
            plan(&c, &shipped).unwrap(),
            MigrationPlan::UpToDate { version: 2 }
        );
        assert!(checksum_mismatches(&c, &shipped).unwrap().is_empty());
        assert!(apply(&mut c, &shipped).unwrap().is_empty());
    }

    #[test]
    fn a_windows_checkout_of_the_earlier_text_is_accepted_too() {
        let crlf: &'static str = Box::leak(
            earlier_appearance_sql()
                .replace('\n', "\r\n")
                .into_boxed_str(),
        );
        let mut c = crate::open_in_memory().unwrap();
        apply(&mut c, &[M1, appearance(crlf)]).unwrap();
        let shipped = [M1, appearance(APPEARANCE_NOW)];
        assert!(checksum_mismatches(&c, &shipped).unwrap().is_empty());
        // A build compiled from a CRLF checkout has the same text with other line endings.
        let shipped_crlf: &'static str =
            Box::leak(APPEARANCE_NOW.replace('\n', "\r\n").into_boxed_str());
        assert!(
            checksum_mismatches(&c, &[M1, appearance(shipped_crlf)])
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn any_other_checksum_difference_is_still_reported() {
        let mut c = crate::open_in_memory().unwrap();
        apply(&mut c, &[M1, appearance(APPEARANCE_NOW)]).unwrap();
        let shipped = [M1, appearance(APPEARANCE_NOW)];
        assert!(checksum_mismatches(&c, &shipped).unwrap().is_empty());
        // A tampered record on a migration the rename touched.
        let tamper = |c: &Connection, version: u32, sum: &str| {
            c.execute(
                "UPDATE _schema_migrations SET checksum=?1 WHERE version=?2",
                params![sum, version],
            )
            .unwrap();
        };
        tamper(&c, 2, &"0".repeat(64));
        assert_eq!(checksum_mismatches(&c, &shipped).unwrap(), vec![2]);
        // The old checksum on a migration that was edited again after the rename.
        tamper(
            &c,
            2,
            "149a096edab8b03013e9d1c564069ba0feefad9a6abb9d37ce5d6250e594af7f",
        );
        let edited = appearance("-- edited again\nSELECT 1;");
        assert_eq!(checksum_mismatches(&c, &[M1, edited]).unwrap(), vec![2]);
        // The old checksum of one file recorded for a migration with another number.
        tamper(&c, 2, &checksum(M1.sql));
        tamper(
            &c,
            1,
            "149a096edab8b03013e9d1c564069ba0feefad9a6abb9d37ce5d6250e594af7f",
        );
        assert_eq!(checksum_mismatches(&c, &shipped).unwrap(), vec![1, 2]);
        // A migration the rename did not touch has no exception at all.
        tamper(&c, 1, &"1".repeat(64));
        assert!(checksum_mismatches(&c, &[M1]).unwrap().contains(&1));
        // Reported, and still never fatal: a person's data must keep opening.
        assert!(apply(&mut c, &shipped).unwrap().is_empty());
    }

    #[test]
    fn the_allow_list_names_exactly_the_files_the_rename_changed() {
        assert_eq!(RENAMED_MIGRATIONS.len(), 11);
        for r in RENAMED_MIGRATIONS {
            assert!(
                r.before != r.now && r.before != r.before_crlf,
                "{}",
                r.version
            );
        }
    }

    #[test]
    fn non_contiguous_list_rejected() {
        let c = crate::open_in_memory().unwrap();
        assert!(plan(&c, &[M2]).is_err());
    }
}
