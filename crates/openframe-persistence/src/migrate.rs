//! Schema migrations (Release/Migration spec; ESD locked decision "safety backup
//! first, then transactional migration with rollback/recovery path").
//!
//! The caller (project lifecycle) is responsible for taking the safety backup
//! *before* calling [`apply`] on an existing database. `apply` itself runs all
//! pending migrations inside ONE transaction, validates foreign keys and
//! structural integrity, and only then commits — a failure leaves the database
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
        "This project was saved by a newer version of OpenFrame. Update OpenFrame to open it — the project has not been changed.",
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
            tx.execute_batch(m.sql).map_err(|e| {
                AppError::migration(format!("migration {} ({}) failed: {e}", m.version, m.name))
            })?;
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
        tx.commit()?;
        Ok(applied)
    })();
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;
    result
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

    #[test]
    fn non_contiguous_list_rejected() {
        let c = crate::open_in_memory().unwrap();
        assert!(plan(&c, &[M2]).is_err());
    }
}
