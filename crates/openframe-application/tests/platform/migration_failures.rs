//! Migration failure fixtures (build plan item migration failure cases; doc 23 §4 rule 7; feature CORE-005).
//!
//! Every scenario starts from a real project that a user filled with content, closes it, and
//! turns its database back into a schema 10 database (what the previous build wrote), so the
//! next `project.open` must take the pre-migration safety backup and run the upgrade. Each
//! failure is then forced at a different point and the test checks what a user relies on:
//!
//! - the open is refused with a plain-language error (never `internal.*`, never raw SQLite text),
//! - `project.sqlite` and `openframe.json` are byte for byte what they were before the attempt,
//!   and a fresh read-only look at the database still shows schema 10 and every card,
//! - once the cause is gone, the same project opens, migrates to the current schema and keeps
//!   its content.
//!
//! Scenarios: full drive inside the migration transaction (debug-only fault hook
//! `openframe_persistence::migrate::fault`, which returns the `SQLITE_FULL` error SQLite itself
//! gives), read-only database file (permission denied), database written by a newer OpenFrame,
//! damaged database that needs a migration, and a pre-migration safety backup that cannot be
//! written. An *interrupted* migration (the process killed mid-upgrade) is covered by the real
//! process-kill matrix in `forced_kill.rs`
//! (`m1_03_forced_kill_during_migration_leaves_a_consistent_database`), which kills a worker
//! process while it repeatedly downgrades and migrates the same project.

use std::fs;
use std::path::{Path, PathBuf};

use openframe_application::schema::PROJECT_MIGRATIONS;
use openframe_domain::AppError;
use openframe_test_support::TestEnv;
use rusqlite::{Connection, OpenFlags};
use serde_json::{Value, json};

/// Schema the fixtures go back to. Migration 11 only adds `sys_ai_apply_journal` and its index;
/// migration 12 only adds `story_scene_card.long_description` and `vault_item.research_flag`;
/// migration 13 (screenplay engine) adds its own tables and columns and rebuilds
/// `screenplay_element` with two more element types, so going back rebuilds it the 0004 way.
const PREVIOUS: i64 = 10;
const CARDS: [&str; 3] = [
    "Ravi waits at the station",
    "Anjali misses the last bus",
    "The depot lights go out",
];

fn latest() -> i64 {
    PROJECT_MIGRATIONS.len() as i64
}

/// A closed project with content, its database turned back into a schema 10 database.
struct Fixture {
    env: TestEnv,
    project: PathBuf,
}

impl Fixture {
    fn new() -> Fixture {
        assert_eq!(
            latest(),
            18,
            "update `downgrade_to_previous` when a project migration is added"
        );
        let env = TestEnv::with_project("Night Bus", "Feature Film");
        let act = env.ok("story.create_act", json!({ "title": "Act One" }))["id"]
            .as_str()
            .unwrap()
            .to_string();
        for text in CARDS {
            env.ok(
                "story.create_card",
                json!({ "parent": { "parentType": "act", "parentId": act }, "shortDescription": text }),
            );
        }
        let project = PathBuf::from(env.project_path());
        env.ok("project.close", json!({}));
        let f = Fixture { env, project };
        f.downgrade_to_previous();
        assert_eq!(f.schema(), PREVIOUS);
        // A read-only look may leave an empty WAL beside the file; it must hold no frames.
        let wal = fs::metadata(f.wal()).map(|m| m.len()).unwrap_or(0);
        assert_eq!(wal, 0, "the fixture database is fully checkpointed");
        f
    }

    fn db(&self) -> PathBuf {
        self.project.join("project.sqlite")
    }

    fn wal(&self) -> PathBuf {
        self.project.join("project.sqlite-wal")
    }

    fn manifest(&self) -> PathBuf {
        self.project.join("openframe.json")
    }

    fn path(&self) -> String {
        self.project.to_string_lossy().into_owned()
    }

    /// Undo migrations 18 to 11 exactly (same SQL the forced-kill worker uses).
    fn downgrade_to_previous(&self) {
        let mut c = Connection::open(self.db()).unwrap();
        let tx = c.transaction().unwrap();
        tx.execute_batch(
            "DROP INDEX idx_ai_memory_user;
             DROP TABLE ai_memory;
             DROP TABLE story_page;
             ALTER TABLE budget_line DROP COLUMN actual;
             DROP INDEX idx_dood_override_key;
             DROP TABLE dood_override;
             ALTER TABLE shooting_schedule DROP COLUMN dood_code_style;
             ALTER TABLE schedule_strip DROP COLUMN start_minutes;
             ALTER TABLE shooting_day DROP COLUMN target_wrap;
             ALTER TABLE shooting_day DROP COLUMN shooting_start;
             ALTER TABLE shooting_day DROP COLUMN crew_call;
             DROP TABLE availability_window;
             DROP TABLE breakdown_category_style;
             ALTER TABLE crew_member DROP COLUMN availability_notes;
             ALTER TABLE location DROP COLUMN emergency_contact;
             ALTER TABLE location DROP COLUMN nearest_hospital;
             DROP TABLE screenplay_lock_baseline;
             DROP TABLE screenplay_revision;
             DROP TABLE screenplay_page_preset;
             ALTER TABLE screenplay DROP COLUMN page_setup_json;
             ALTER TABLE screenplay_scene DROP COLUMN production_number_source;
             ALTER TABLE screenplay_scene DROP COLUMN production_number;
             CREATE TABLE screenplay_element_old (
                 id              TEXT PRIMARY KEY,
                 scene_id        TEXT NOT NULL REFERENCES screenplay_scene(id),
                 position        INTEGER NOT NULL,
                 element_type    TEXT NOT NULL CHECK (element_type IN ('scene_heading','action','character','dialogue','parenthetical','transition','shot','note')),
                 text            TEXT NOT NULL DEFAULT '',
                 dual            INTEGER NOT NULL DEFAULT 0,
                 revision_mark   TEXT,
                 created_at      INTEGER NOT NULL,
                 updated_at      INTEGER NOT NULL,
                 rev             INTEGER NOT NULL DEFAULT 1
             );
             INSERT INTO screenplay_element_old
                 SELECT id, scene_id, position, element_type, text, dual, revision_mark, created_at, updated_at, rev
                 FROM screenplay_element;
             DROP TABLE screenplay_element;
             ALTER TABLE screenplay_element_old RENAME TO screenplay_element;
             CREATE INDEX idx_element_scene ON screenplay_element(scene_id, position);
             DROP TABLE screenplay_custom_element;
             DROP INDEX IF EXISTS idx_vault_item_research;
             ALTER TABLE vault_item DROP COLUMN research_flag;
             ALTER TABLE story_scene_card DROP COLUMN long_description;
             DROP INDEX IF EXISTS sys_ai_apply_journal_open;
             DROP TABLE IF EXISTS sys_ai_apply_journal;
             DELETE FROM _schema_migrations WHERE version IN (11, 12, 13, 14, 15, 16, 17, 18);
             PRAGMA user_version = 10;",
        )
        .unwrap();
        tx.commit().unwrap();
    }

    fn set_schema(&self, version: i64) {
        let c = Connection::open(self.db()).unwrap();
        c.pragma_update(None, "user_version", version).unwrap();
    }

    /// Read-only look at the database: never writes, never checkpoints into the main file.
    fn read_only(&self) -> Connection {
        Connection::open_with_flags(self.db(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap()
    }

    fn schema(&self) -> i64 {
        self.read_only()
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap()
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            db: fs::read(self.db()).unwrap(),
            manifest: fs::read(self.manifest()).unwrap(),
        }
    }

    fn open(&self) -> Result<Value, AppError> {
        self.env
            .call("project.open", json!({ "path": self.path() }))
    }

    fn safety_backups(&self) -> Vec<PathBuf> {
        let dir = self.project.join("backups");
        if !dir.is_dir() {
            return Vec::new();
        }
        let mut out: Vec<PathBuf> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("pre-migration-"))
            })
            .collect();
        out.sort();
        out
    }

    /// Nothing about the project changed: same bytes, no committed WAL frames, same content.
    #[track_caller]
    fn assert_untouched(&self, before: &Snapshot, case: &str) {
        assert!(
            fs::read(self.db()).unwrap() == before.db,
            "{case}: project.sqlite changed"
        );
        assert!(
            fs::read(self.manifest()).unwrap() == before.manifest,
            "{case}: openframe.json changed"
        );
        let wal = fs::metadata(self.wal()).map(|m| m.len()).unwrap_or(0);
        assert_eq!(
            wal, 0,
            "{case}: a WAL with frames was left beside the project"
        );
        assert_eq!(self.schema(), PREVIOUS, "{case}: schema version moved");
        assert_eq!(self.card_texts(), CARDS, "{case}: content changed");
        let has_journal: bool = self
            .read_only()
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='sys_ai_apply_journal')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(!has_journal, "{case}: half of migration 11 persisted");
    }

    fn card_texts(&self) -> Vec<String> {
        let c = self.read_only();
        let mut stmt = c
            .prepare("SELECT short_description FROM story_scene_card WHERE deleted_at IS NULL ORDER BY position")
            .unwrap();
        stmt.query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    /// With the cause removed the project opens, migrates from 10 and keeps every card.
    #[track_caller]
    fn assert_recovers(&self, case: &str) {
        let backups_before = self.safety_backups().len();
        let opened = self
            .open()
            .unwrap_or_else(|e| panic!("{case}: reopen failed: {} {:?}", e.code, e.detail));
        assert_eq!(opened["migratedFrom"], json!(PREVIOUS), "{case}: {opened}");
        let board = self.env.ok("story.board", json!({})).to_string();
        for text in CARDS {
            assert!(
                board.contains(text),
                "{case}: {text:?} missing from the board"
            );
        }
        self.env.ok("project.close", json!({}));
        assert_eq!(self.schema(), latest(), "{case}: not migrated");
        assert_eq!(self.card_texts(), CARDS, "{case}: content after migration");
        assert_eq!(
            self.safety_backups().len(),
            backups_before + 1,
            "{case}: the successful upgrade writes its own safety backup"
        );
    }
}

struct Snapshot {
    db: Vec<u8>,
    manifest: Vec<u8>,
}

/// The error a user sees is written for people: a stable non-internal code and a message that
/// carries no raw SQLite, OS or path text (that belongs in `detail`, for support).
#[track_caller]
fn assert_human(e: &AppError, case: &str) {
    // Shown with `--nocapture`; the release evidence records what users are told.
    eprintln!("{case}: refused with {} \"{}\"", e.code, e.message);
    let code = e.code.0.as_str();
    assert!(
        !code.starts_with("internal"),
        "{case}: internal error {code}"
    );
    assert!(
        e.message.len() > 20 && e.message.ends_with('.'),
        "{case}: {:?}",
        e.message
    );
    for raw in [
        "sqlite",
        "SQLITE",
        "rusqlite",
        "os error",
        "Error {",
        "\\\\?\\",
        "project.sqlite",
    ] {
        assert!(
            !e.message.contains(raw),
            "{case}: raw text {raw:?} in {:?}",
            e.message
        );
    }
}

/// Clears the read-only attribute even when an assertion fails, so the temp folder can be removed
/// and later steps are not affected.
struct ReadOnlyGuard(PathBuf);

impl ReadOnlyGuard {
    fn set(path: &Path) -> ReadOnlyGuard {
        set_readonly(path, true);
        ReadOnlyGuard(path.to_path_buf())
    }
}

impl Drop for ReadOnlyGuard {
    fn drop(&mut self) {
        set_readonly(&self.0, false);
    }
}

fn set_readonly(path: &Path, on: bool) {
    let mut p = fs::metadata(path).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    p.set_readonly(on);
    fs::set_permissions(path, p).unwrap();
}

// ------------------------------------------------------------------------------------ scenarios

#[cfg(debug_assertions)]
#[test]
fn m1_05_full_drive_during_migration_leaves_the_database_untouched() {
    use openframe_persistence::migrate::fault::{self, DiskFullAt};

    for point in [DiskFullAt::Step(11), DiskFullAt::Commit] {
        let case = format!("disk full at {point:?}");
        let f = Fixture::new();
        let before = f.snapshot();
        fault::disk_full_at(Some(point));
        let result = f.open();
        fault::disk_full_at(None);
        let e = result.expect_err(&case);
        assert_human(&e, &case);
        // The full drive is what the user must fix, so the error says so at every point.
        assert_eq!(e.code.0, "storage.disk_full", "{case}");
        assert!(e.message.contains("drive is full"), "{case}: {}", e.message);
        assert!(
            e.message.contains("upgrade this project"),
            "{case}: {}",
            e.message
        );
        assert!(
            e.detail.as_deref().unwrap_or("").contains("disk is full"),
            "{case}: cause missing from detail {:?}",
            e.detail
        );
        f.assert_untouched(&before, &case);
        // The safety backup taken before the attempt is kept and holds the schema 10 content.
        let backups = f.safety_backups();
        assert_eq!(backups.len(), 1, "{case}: {backups:?}");
        let b = Connection::open_with_flags(&backups[0], OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let v: i64 = b
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, PREVIOUS, "{case}: safety backup version");
        let ok: String = b
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))
            .unwrap();
        assert_eq!(ok, "ok", "{case}: safety backup integrity");
        let n: i64 = b
            .query_row("SELECT count(*) FROM story_scene_card", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, CARDS.len() as i64, "{case}: safety backup content");
        drop(b);
        f.assert_recovers(&case);
    }
}

#[test]
fn m1_05_read_only_database_is_refused_untouched_and_opens_once_writable() {
    let case = "read-only project.sqlite";
    let f = Fixture::new();
    let before = f.snapshot();
    {
        let _guard = ReadOnlyGuard::set(&f.db());
        assert!(fs::metadata(f.db()).unwrap().permissions().readonly());
        let e = f.open().expect_err(case);
        assert_human(&e, case);
        assert_eq!(e.code.0, "storage.access_denied", "{case}: {:?}", e.detail);
        assert!(e.message.contains("read-only"), "{case}: {}", e.message);
        f.assert_untouched(&before, case);
    }
    assert!(!fs::metadata(f.db()).unwrap().permissions().readonly());
    f.assert_recovers(case);
}

#[test]
fn m1_05_database_from_a_newer_openframe_is_refused_byte_for_byte_untouched() {
    let case = "newer schema";
    let f = Fixture::new();
    // What a newer build leaves behind: its schema version in the database and the manifest.
    f.set_schema(latest() + 1);
    let mut manifest: Value = serde_json::from_slice(&fs::read(f.manifest()).unwrap()).unwrap();
    manifest["schemaVersion"] = json!(latest() + 1);
    fs::write(f.manifest(), serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    let before = f.snapshot();
    for attempt in 1..=2 {
        let e = f.open().expect_err(case);
        assert_eq!(e.code.0, "project_format.too_new", "attempt {attempt}");
        assert_human(&e, case);
        assert!(e.message.contains("Update OpenFrame"), "{}", e.message);
        assert!(
            fs::read(f.db()).unwrap() == before.db,
            "attempt {attempt}: project.sqlite changed"
        );
        assert!(
            fs::read(f.manifest()).unwrap() == before.manifest,
            "attempt {attempt}: openframe.json changed"
        );
        assert_eq!(f.schema(), latest() + 1);
        assert!(
            f.safety_backups().is_empty(),
            "no migration, so no safety backup"
        );
    }
}

#[test]
fn m1_05_damaged_database_needing_migration_is_refused_before_any_backup_or_write() {
    // A damaged page inside the database, and a file whose SQLite header is gone.
    type Damage = (&'static str, fn(&mut [u8]));
    let damage: [Damage; 2] = [
        ("damaged page", |b| {
            for x in b.iter_mut().skip(4096).take(4096) {
                *x = 0xAB;
            }
        }),
        ("damaged header", |b| {
            for x in b.iter_mut().take(16) {
                *x = 0x00;
            }
        }),
    ];
    for (case, corrupt) in damage {
        let f = Fixture::new();
        let mut bytes = fs::read(f.db()).unwrap();
        assert!(bytes.len() > 8192, "fixture database spans several pages");
        corrupt(&mut bytes);
        fs::write(f.db(), &bytes).unwrap();
        let manifest = fs::read(f.manifest()).unwrap();
        let e = f.open().expect_err(case);
        assert_eq!(e.code.0, "project_format.corrupt", "{case}: {:?}", e.detail);
        assert_human(&e, case);
        assert!(
            fs::read(f.db()).unwrap() == bytes,
            "{case}: a damaged project must not be modified"
        );
        assert!(fs::read(f.manifest()).unwrap() == manifest, "{case}");
        let wal = fs::metadata(f.wal()).map(|m| m.len()).unwrap_or(0);
        assert_eq!(wal, 0, "{case}: nothing was written beside it");
        assert!(
            f.safety_backups().is_empty(),
            "{case}: refused before the safety backup and the migration"
        );
    }
}

#[test]
fn m1_05_safety_backup_that_cannot_be_written_stops_the_migration() {
    let case = "safety backup unwritable";
    let f = Fixture::new();
    let backups = f.project.join("backups");
    if backups.is_dir() {
        assert!(
            f.safety_backups().is_empty(),
            "a new project has no migration backups"
        );
        fs::remove_dir_all(&backups).unwrap();
    }
    // A file where the backup folder must go: the safety backup cannot be created.
    fs::write(&backups, b"not a folder").unwrap();
    let before = f.snapshot();
    let e = f.open().expect_err(case);
    assert_human(&e, case);
    assert!(
        e.code.0.starts_with("storage."),
        "{case}: {} {:?}",
        e.code,
        e.detail
    );
    // The user is told the safety copy failed, not that "this change" could not be saved.
    assert!(e.message.contains("safety copy"), "{case}: {}", e.message);
    assert!(
        !e.message.contains("save this change"),
        "{case}: {}",
        e.message
    );
    f.assert_untouched(&before, case);
    assert!(backups.is_file(), "the blocking file is left alone");
    fs::remove_file(&backups).unwrap();
    f.assert_recovers(case);
}
