//! Forced-kill matrix (build plan item forced stop tests; feature CORE-003, CORE-004, CORE-005, XFER-008, XFER-012).
//!
//! Each scenario respawns this test binary as a worker (the ignored `worker` test, selected with
//! `--exact` and an environment variable). The worker opens the project and repeats one operation
//! (save, migration, export, backup, package import), appending every operation that returned
//! successfully to `committed.log` with a flush to disk. The parent kills it hard
//! (`TerminateProcess` on Windows) at several delays, then reopens the same data folder in a fresh
//! `AppCore` and checks what a user would rely on: the project opens, a recovery offer appears
//! after the unclean exit, SQLite integrity passes, no committed edit is missing, and no export,
//! backup or imported project is left half-written where a user would find it.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use openframe_application::events::{AppEvent, RecordingSink};
use openframe_application::{AppConfig, AppCore};
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

const WORKER_ENV: &str = "OPENFRAME_KILL_WORKER";
const DIR_ENV: &str = "OPENFRAME_KILL_DIR";
const PROJECT_ENV: &str = "OPENFRAME_KILL_PROJECT";
const PACKAGE_ENV: &str = "OPENFRAME_KILL_PACKAGE";
/// When each run kills the worker. The delays vary so kills land in different phases. The last
/// one waits for the worker's first finished operation before it counts, so that kill always
/// lands in the middle of real work, however slow or loaded the machine is.
const KILLS: &[KillAt] = &[
    KillAt::AfterReady(60),
    KillAt::AfterReady(250),
    KillAt::AfterFirstCommit(250),
];
/// Upper bound for the worker to become ready or finish its first operation. Generous on purpose:
/// it only turns a hung worker into a failure, it never decides when the kill happens.
const PROGRESS_BOUND: Duration = Duration::from_secs(180);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KillAt {
    /// Kill this many milliseconds after the worker reports ready.
    AfterReady(u64),
    /// Kill this many milliseconds after the worker has finished at least one operation.
    AfterFirstCommit(u64),
}

// ----------------------------------------------------------------------------------- worker side

struct Worker {
    core: Arc<AppCore>,
    sink: Arc<RecordingSink>,
    dir: PathBuf,
    project: String,
}

impl Worker {
    fn call(&self, op: &str, args: Value) -> Value {
        self.core
            .dispatch(&self.core.local_actor(), op, args)
            .unwrap_or_else(|e| panic!("{op} failed in worker: {} {:?}", e.message, e.detail))
    }

    /// Wait for a background task started by `op` and require it to complete.
    fn task(&self, started: &Value) {
        let id = started["taskId"].as_str().expect("taskId").to_string();
        let start = Instant::now();
        loop {
            {
                let events = self.sink.events.lock();
                for e in events.iter().rev() {
                    if let AppEvent::Task(t) = e
                        && t.task_id == id
                        && matches!(t.state.as_str(), "completed" | "failed" | "cancelled")
                    {
                        assert_eq!(t.state, "completed", "worker task failed: {:?}", t.error);
                        return;
                    }
                }
            }
            assert!(
                start.elapsed() < Duration::from_secs(120),
                "worker task hung"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn committed(&self, line: &str) {
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.dir.join("committed.log"))
            .unwrap();
        writeln!(f, "{line}").unwrap();
        f.sync_all().unwrap();
    }

    fn open(&self) {
        self.call("project.open", json!({ "path": self.project }));
    }

    fn ready(&self) {
        fs::write(self.dir.join("ready"), b"ready").unwrap();
    }
}

#[test]
#[ignore = "child process of the forced-kill tests; they spawn it"]
fn worker() {
    let Ok(scenario) = std::env::var(WORKER_ENV) else {
        return;
    };
    let dir = PathBuf::from(std::env::var(DIR_ENV).unwrap());
    let sink = Arc::new(RecordingSink::default());
    let core = AppCore::new(AppConfig::isolated(&dir), sink.clone()).unwrap();
    let w = Worker {
        core,
        sink,
        dir: dir.clone(),
        project: std::env::var(PROJECT_ENV).unwrap(),
    };
    let out = dir.join("out");
    fs::create_dir_all(&out).unwrap();
    match scenario.as_str() {
        "save" => {
            w.open();
            w.ready();
            for i in 0..u64::MAX {
                w.call(
                    "story.create_card",
                    json!({ "parent": { "parentType": "parking" }, "shortDescription": format!("k{i}") }),
                );
                w.committed(&format!("k{i}"));
                if i % 5 == 4 {
                    w.call("project.save", json!({}));
                }
            }
        }
        "migration" => {
            w.ready();
            loop {
                downgrade_to_v10(&w.project);
                w.open();
                w.committed("migrated");
                w.call("project.close", json!({}));
            }
        }
        "export" => {
            w.open();
            w.ready();
            let pdf = out.join("Board.pdf");
            for i in 0..u64::MAX {
                w.call(
                    "story.export_board",
                    json!({ "format": "pdf", "path": pdf }),
                );
                w.committed(&format!("export{i}"));
            }
        }
        "backup" => {
            w.open();
            w.ready();
            let dest = out.join("Night Bus.ofbackup");
            for i in 0..u64::MAX {
                let started = w.call("packages.create_backup", json!({ "path": dest }));
                w.task(&started);
                w.committed(&format!("backup{i}"));
            }
        }
        "import" => {
            w.ready();
            let package = std::env::var(PACKAGE_ENV).unwrap();
            for i in 0..u64::MAX {
                let started = w.call(
                    "packages.import_project",
                    json!({ "path": package, "mode": "copy" }),
                );
                w.task(&started);
                w.committed(&format!("import{i}"));
                w.call("project.close", json!({}));
            }
        }
        other => panic!("unknown scenario {other}"),
    }
}

/// Turn a schema-16 project database back into a genuine schema-10 one (0016 only adds
/// `budget_line.actual` and maps call sheet status words, which the older schema also allows; 0015 only adds the
/// schedule day times, strip start, Day Out of Days code style and corrections table; 0014 only adds the
/// breakdown category styles, availability windows and three location and crew columns; 0011 only adds the
/// apply journal; 0012 only adds `story_scene_card.long_description` and
/// `vault_item.research_flag`; 0013 adds the screenplay engine tables and columns and rebuilds
/// `screenplay_element`), so opening it runs real migrations. Test scaffolding, one transaction.
fn downgrade_to_v10(project: &str) {
    let db = Path::new(project).join("project.sqlite");
    let mut c = rusqlite::Connection::open(db).unwrap();
    let mut v: i64 = c
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    if v == 16 {
        let tx = c.transaction().unwrap();
        tx.execute_batch(
            "ALTER TABLE budget_line DROP COLUMN actual;
             DELETE FROM _schema_migrations WHERE version = 16;
             PRAGMA user_version = 15;",
        )
        .unwrap();
        tx.commit().unwrap();
        v = 15;
    }
    if v == 15 {
        let tx = c.transaction().unwrap();
        tx.execute_batch(
            "DROP INDEX idx_dood_override_key;
             DROP TABLE dood_override;
             ALTER TABLE shooting_schedule DROP COLUMN dood_code_style;
             ALTER TABLE schedule_strip DROP COLUMN start_minutes;
             ALTER TABLE shooting_day DROP COLUMN target_wrap;
             ALTER TABLE shooting_day DROP COLUMN shooting_start;
             ALTER TABLE shooting_day DROP COLUMN crew_call;
             DELETE FROM _schema_migrations WHERE version = 15;
             PRAGMA user_version = 14;",
        )
        .unwrap();
        tx.commit().unwrap();
        v = 14;
    }
    if v == 14 {
        let tx = c.transaction().unwrap();
        tx.execute_batch(
            "DROP TABLE availability_window;
             DROP TABLE breakdown_category_style;
             ALTER TABLE crew_member DROP COLUMN availability_notes;
             ALTER TABLE location DROP COLUMN emergency_contact;
             ALTER TABLE location DROP COLUMN nearest_hospital;
             DELETE FROM _schema_migrations WHERE version = 14;
             PRAGMA user_version = 13;",
        )
        .unwrap();
        tx.commit().unwrap();
        v = 13;
    }
    if v == 13 {
        let tx = c.transaction().unwrap();
        tx.execute_batch(
            "DROP TABLE screenplay_lock_baseline;
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
             DELETE FROM _schema_migrations WHERE version IN (11, 12, 13);
             PRAGMA user_version = 10;",
        )
        .unwrap();
        tx.commit().unwrap();
    }
}

// ----------------------------------------------------------------------------------- parent side

struct Harness {
    env: TestEnv,
    project: String,
}

impl Harness {
    /// A project with baseline content, closed so the worker can take the single-writer lock.
    fn new() -> Harness {
        let env = TestEnv::with_project("Night Bus", "Feature Film");
        // In an act, so the Story Board has content to export.
        let act = env.ok("story.create_act", json!({ "title": "Act 1" }))["id"]
            .as_str()
            .unwrap()
            .to_string();
        for text in ["Baseline one", "Baseline two"] {
            env.ok(
                "story.create_card",
                json!({ "parent": { "parentType": "act", "parentId": act }, "shortDescription": text }),
            );
        }
        let project = env.project_path();
        env.ok("project.close", json!({}));
        Harness { env, project }
    }

    fn dir(&self) -> &Path {
        self.env.dir.path()
    }

    fn spawn(&self, scenario: &str, extra: &[(&str, String)]) -> Child {
        let _ = fs::remove_file(self.dir().join("ready"));
        let mut cmd = Command::new(std::env::current_exe().unwrap());
        cmd.args([
            "worker",
            "--exact",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(WORKER_ENV, scenario)
        .env(DIR_ENV, self.dir())
        .env(PROJECT_ENV, &self.project)
        .stdout(Stdio::null())
        .stderr(fs::File::create(self.dir().join("worker.log")).unwrap());
        for (k, v) in extra {
            cmd.env(k, v);
        }
        cmd.spawn().unwrap()
    }

    /// Wait until `done` holds, polling, while the worker is still running. Fails with the
    /// worker's log when it exits early or when nothing happens within [`PROGRESS_BOUND`].
    fn wait_for(&self, child: &mut Child, what: &str, done: impl Fn(&Harness) -> bool) {
        let start = Instant::now();
        while !done(self) {
            if let Some(status) = child.try_wait().unwrap() {
                panic!(
                    "worker exited before it {what}: {status}\n{}",
                    self.worker_log()
                );
            }
            assert!(
                start.elapsed() < PROGRESS_BOUND,
                "worker never {what} in {PROGRESS_BOUND:?}\n{}",
                self.worker_log()
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Wait for the worker to report ready (and, for [`KillAt::AfterFirstCommit`], to finish its
    /// first operation), let it work for the delay, then kill it hard.
    fn kill_at(&self, mut child: Child, at: KillAt) {
        self.wait_for(&mut child, "became ready", |h| {
            h.dir().join("ready").exists()
        });
        let ms = match at {
            KillAt::AfterReady(ms) => ms,
            KillAt::AfterFirstCommit(ms) => {
                self.wait_for(&mut child, "finished an operation", |h| {
                    !h.committed().is_empty()
                });
                ms
            }
        };
        std::thread::sleep(Duration::from_millis(ms));
        if let Some(status) = child.try_wait().unwrap() {
            panic!("worker stopped on its own: {status}\n{}", self.worker_log());
        }
        child.kill().unwrap();
        child.wait().unwrap();
    }

    /// The last lines the worker wrote to stderr (its panic message, if any).
    fn worker_log(&self) -> String {
        let log = fs::read_to_string(self.dir().join("worker.log")).unwrap_or_default();
        let lines: Vec<&str> = log
            .lines()
            .filter(|l| !l.trim_start().starts_with("at "))
            .collect();
        lines[lines.len().saturating_sub(12)..].join("\n")
    }

    fn committed(&self) -> Vec<String> {
        fs::read_to_string(self.dir().join("committed.log"))
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }

    /// Reopen in a fresh core, as the next app launch would. Returns the core and open result.
    fn reopen(&self) -> (Arc<AppCore>, Value) {
        let core = AppCore::new(
            AppConfig::isolated(self.dir()),
            Arc::new(RecordingSink::default()),
        )
        .unwrap();
        let opened = core
            .dispatch(
                &core.local_actor(),
                "project.open",
                json!({ "path": self.project }),
            )
            .unwrap_or_else(|e| {
                panic!(
                    "project does not open after a forced kill: {} {:?}",
                    e.message, e.detail
                )
            });
        (core, opened)
    }
}

fn integrity_ok(core: &AppCore) {
    let check: String = core
        .project()
        .unwrap()
        .store
        .read(|c| Ok(c.query_row("PRAGMA integrity_check", [], |r| r.get(0))?))
        .unwrap();
    assert_eq!(check, "ok");
}

fn card_texts(core: &AppCore) -> Vec<String> {
    core.project()
        .unwrap()
        .store
        .read(|c| {
            let mut st = c.prepare(
                "SELECT short_description FROM story_scene_card WHERE deleted_at IS NULL",
            )?;
            let rows = st
                .query_map([], |r| r.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })
        .unwrap()
}

fn close(core: &AppCore) {
    core.dispatch(&core.local_actor(), "project.close", json!({}))
        .unwrap();
}

fn wait_task(env: &TestEnv, started: &Value) {
    let id = started["taskId"].as_str().unwrap().to_string();
    let start = Instant::now();
    loop {
        {
            let events = env.sink.events.lock();
            if events.iter().any(
                |e| matches!(e, AppEvent::Task(t) if t.task_id == id && t.state == "completed"),
            ) {
                return;
            }
        }
        assert!(
            start.elapsed() < Duration::from_secs(120),
            "task did not finish"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn m1_03_forced_kill_during_save_loses_no_committed_edit_and_offers_recovery() {
    for &at in KILLS {
        let h = Harness::new();
        let child = h.spawn("save", &[]);
        h.kill_at(child, at);
        let committed = h.committed();
        if matches!(at, KillAt::AfterFirstCommit(_)) {
            assert!(
                !committed.is_empty(),
                "the save worker committed nothing; the kill hit no real work"
            );
        }
        let (core, opened) = h.reopen();
        assert!(
            !opened["recovery"].is_null(),
            "no recovery offer after a kill ({at:?}): {opened}"
        );
        integrity_ok(&core);
        let cards = card_texts(&core);
        for text in ["Baseline one", "Baseline two"]
            .iter()
            .map(|s| s.to_string())
            .chain(committed.iter().cloned())
        {
            assert!(
                cards.contains(&text),
                "committed edit {text} lost after a kill ({at:?})"
            );
        }
        close(&core);
    }
}

#[test]
fn m1_03_forced_kill_during_migration_leaves_a_consistent_database() {
    for &at in KILLS {
        let h = Harness::new();
        let child = h.spawn("migration", &[]);
        h.kill_at(child, at);
        if matches!(at, KillAt::AfterFirstCommit(_)) {
            // At least one full downgrade, migrate and close cycle finished before the kill, so
            // the kill landed while the worker was migrating again (or about to).
            assert!(
                !h.committed().is_empty(),
                "the migration worker completed no cycle before the kill"
            );
        }
        // Before the app touches it: a complete schema 10 to 16 (each migration commits whole).
        let raw = rusqlite::Connection::open(Path::new(&h.project).join("project.sqlite")).unwrap();
        let version: i64 = raw
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        let journal: bool = raw
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='sys_ai_apply_journal')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            matches!((version, journal), (10, false) | (11..=16, true)),
            "half-migrated: v{version}, journal {journal}"
        );
        drop(raw);
        let (core, opened) = h.reopen();
        integrity_ok(&core);
        let schema: i64 = core
            .project()
            .unwrap()
            .store
            .read(|c| Ok(c.query_row("PRAGMA user_version", [], |r| r.get(0))?))
            .unwrap();
        assert_eq!(schema, 16, "{opened}");
        let cards = card_texts(&core);
        assert!(
            cards.contains(&"Baseline one".to_string())
                && cards.contains(&"Baseline two".to_string())
        );
        close(&core);
    }
}

#[test]
fn m1_03_forced_kill_during_export_never_leaves_a_broken_file_or_touches_the_project() {
    for &at in KILLS {
        let h = Harness::new();
        let child = h.spawn("export", &[]);
        h.kill_at(child, at);
        let pdf = h.dir().join("out").join("Board.pdf");
        if pdf.exists() {
            let bytes = fs::read(&pdf).unwrap();
            assert!(
                bytes.starts_with(b"%PDF-"),
                "export killed {at:?} is not a PDF"
            );
            let tail =
                String::from_utf8_lossy(&bytes[bytes.len().saturating_sub(64)..]).to_string();
            assert!(
                tail.trim_end().ends_with("%%EOF"),
                "export killed {at:?} is truncated"
            );
        }
        let (core, _) = h.reopen();
        integrity_ok(&core);
        assert_eq!(card_texts(&core).len(), 2, "an export changed the project");
        close(&core);
    }
}

#[test]
fn m1_03_forced_kill_during_backup_never_leaves_a_broken_backup() {
    use openframe_security::archive;
    for &at in KILLS {
        let h = Harness::new();
        let child = h.spawn("backup", &[]);
        h.kill_at(child, at);
        let dest = h.dir().join("out").join("Night Bus.ofbackup");
        if dest.exists() {
            for entry in ["manifest.json", "checksums.json", "project/project.sqlite"] {
                assert!(
                    archive::read_entry(&dest, entry, 512 << 20)
                        .unwrap()
                        .is_some(),
                    "backup killed {at:?} lacks {entry}"
                );
            }
        }
        let (core, _) = h.reopen();
        integrity_ok(&core);
        assert_eq!(card_texts(&core).len(), 2, "a backup changed the project");
        close(&core);
    }
}

#[test]
fn m1_03_forced_kill_during_package_import_leaves_no_broken_project() {
    for &at in KILLS {
        let h = Harness::new();
        // A real backup of the project to import as copies.
        h.env.ok("project.open", json!({ "path": h.project }));
        let package = h.dir().join("source.ofbackup");
        let started = h
            .env
            .ok("packages.create_backup", json!({ "path": package }));
        wait_task(&h.env, &started);
        h.env.ok("project.close", json!({}));

        let child = h.spawn(
            "import",
            &[(PACKAGE_ENV, package.to_string_lossy().into_owned())],
        );
        h.kill_at(child, at);

        // The original still opens intact, and every project the app lists opens and passes integrity.
        let (core, _) = h.reopen();
        integrity_ok(&core);
        assert_eq!(card_texts(&core).len(), 2);
        close(&core);
        let recents = core
            .dispatch(&core.local_actor(), "project.list_recent", json!({}))
            .unwrap();
        for p in recents.as_array().unwrap() {
            let path = p["path"].as_str().unwrap();
            core.dispatch(&core.local_actor(), "project.open", json!({ "path": path }))
                .unwrap_or_else(|e| {
                    panic!(
                        "listed project {path} does not open after a kill ({at:?}): {}",
                        e.message
                    )
                });
            integrity_ok(&core);
            close(&core);
        }
    }
}
