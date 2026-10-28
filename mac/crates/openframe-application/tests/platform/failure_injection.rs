//! failure injection failure injection, storage and process (build plan item failure injection; doc 22 §7).
//!
//! Each scenario produces the condition with the real Windows mechanism where one exists on a
//! developer machine, and with a debug-only fault point where it does not:
//!
//! | Condition | How it is produced |
//! |---|---|
//! | Disk full | `store::fault` (`SQLITE_FULL` at commit) and `exports::fault` (`ERROR_DISK_FULL` on write), debug builds only |
//! | Device write failure (external drive pulled mid-commit) | `store::fault` (`SQLITE_IOERR_WRITE` at commit) |
//! | Permission denied | an ACL deny entry (`icacls /deny`) on the destination folder; the read-only attribute on `project.sqlite` |
//! | Path disappears | the destination folder deleted after it was chosen; the project folder moved while closed; `recovery/` deleted while open |
//! | External volume removed | the project lives on a `subst` drive letter that is removed while the project is open |
//! | Read-only destination | the read-only attribute on an existing export or backup file |
//! | Sleep and resume during indexing | the embedding runtime stops answering (stalled), or fails, for the "sleep", then resumes |
//! | Antivirus-locked file | another handle opened without `FILE_SHARE_DELETE`/`FILE_SHARE_WRITE` on the destination, the checkpoint, the database and an import source |
//! | Very long paths | a project folder, export and backup more than 260 characters deep |
//! | Non-ASCII project paths | `Résumé/Café/日本` folders and a non-ASCII project title |
//!
//! Sleep and resume during a download is in `crates/openframe-ai/tests/soak_sidecar.rs` (the
//! download client and its test server live in that crate).
//!
//! Expected outcome everywhere: a human error code and message, no partial file where the user
//! would find it, no data loss, and the project still opens with every acknowledged change.

// Some helpers and imports serve only the Windows-only tests below.
#![cfg_attr(not(windows), allow(dead_code))]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use openframe_application::events::AppEvent;
use openframe_domain::AppError;
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

// ------------------------------------------------------------------------------------ helpers

/// A small project with content in every table the checks look at.
struct Seeded {
    env: TestEnv,
    path: String,
    draft: String,
    element: String,
    card: String,
    note: String,
}

fn seed(env: TestEnv) -> Seeded {
    let sp = env.ok("screenplay.create", json!({}));
    let draft = sp["draft"]["id"].as_str().unwrap().to_string();
    let mut element = String::new();
    for i in 0..3 {
        let r = env.ok(
            "screenplay.create_scene",
            json!({ "draftId": draft, "heading": format!("INT. RAILWAY STATION {i} - NIGHT") }),
        );
        let scene = r["createdIds"][0].as_str().unwrap().to_string();
        let el = openframe_domain::new_id();
        env.ok(
            "screenplay.apply_edits",
            json!({ "draftId": draft, "ops": [
                { "op": "insertElement", "id": el, "sceneId": scene, "elementType": "action", "text": "Rain hammers the platform." },
                { "op": "insertElement", "sceneId": scene, "elementType": "character", "text": "MEERA" },
                { "op": "insertElement", "sceneId": scene, "elementType": "dialogue", "text": "The last train never comes." }
            ] }),
        );
        element = el;
    }
    let card = env.ok(
        "story.create_card",
        json!({ "shortDescription": "Meera waits on the platform" }),
    )["id"]
        .as_str()
        .unwrap()
        .to_string();
    let note = env.ok(
        "notes.create",
        json!({ "title": "Location", "body": "Night access from 5 am." }),
    )["id"]
        .as_str()
        .unwrap()
        .to_string();
    let path = env.project_path();
    Seeded {
        env,
        path,
        draft,
        element,
        card,
        note,
    }
}

fn seeded() -> Seeded {
    seed(TestEnv::with_project("Railway", "Feature Film"))
}

/// Everything a user would call "my work": cards, notes, trash and screenplay text.
fn content(env: &TestEnv) -> String {
    let store = &env.core.project().expect("project open").store;
    let mut out = String::new();
    for sql in [
        "SELECT id||'|'||short_description||'|'||coalesce(deleted_at,'') FROM story_scene_card ORDER BY id",
        "SELECT id||'|'||coalesce(title,'')||'|'||body||'|'||coalesce(deleted_at,'') FROM project_note ORDER BY id",
        "SELECT table_name||'|'||object_id FROM deleted_item ORDER BY 1",
        "SELECT id||'|'||element_type||'|'||text FROM screenplay_element ORDER BY id",
    ] {
        let rows: Vec<String> = store
            .read(|c| {
                let mut st = c.prepare(sql)?;
                let v = st
                    .query_map([], |r| r.get::<_, String>(0))?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(v)
            })
            .unwrap();
        for r in rows {
            out.push_str(&r);
            out.push('\n');
        }
    }
    out
}

fn scalar(env: &TestEnv, sql: &str) -> i64 {
    env.core
        .project()
        .unwrap()
        .store
        .read(|c| Ok(c.query_row(sql, [], |r| r.get(0))?))
        .unwrap()
}

fn integrity_ok(env: &TestEnv) {
    let ok: String = env
        .core
        .project()
        .unwrap()
        .store
        .read(|c| Ok(c.query_row("PRAGMA integrity_check", [], |r| r.get(0))?))
        .unwrap();
    assert_eq!(ok, "ok");
}

/// A message a user can act on: no raw OS, SQLite or Rust text, a full sentence.
#[track_caller]
fn assert_human(e: &AppError, case: &str) {
    assert_ne!(
        e.code_str(),
        "internal.unexpected",
        "{case}: unexpected internal error {e:?}"
    );
    assert!(e.message.len() > 15, "{case}: {:?}", e.message);
    assert!(
        e.message.ends_with('.') || e.message.ends_with('?'),
        "{case}: {:?}",
        e.message
    );
    for raw in [
        "os error",
        "\\\\?\\",
        "SQLITE",
        "sqlite",
        "rusqlite",
        "panicked",
        "unwrap",
        "Error {",
        "Access is denied",
        "The process cannot access",
    ] {
        assert!(
            !e.message.contains(raw),
            "{case}: raw text {raw:?} in {:?}",
            e.message
        );
    }
}

#[track_caller]
fn expect_err(env: &TestEnv, op: &str, args: Value, case: &str) -> AppError {
    match env.call(op, args) {
        Ok(v) => panic!("{case}: {op} unexpectedly succeeded: {v}"),
        Err(e) => {
            assert_human(&e, case);
            e
        }
    }
}

fn files_in(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// Files whose name marks an unfinished write.
fn partials_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                let n = e.file_name().to_string_lossy().to_lowercase();
                if n.ends_with(".partial") || n.ends_with(".tmp") || n.ends_with(".openframe-tmp") {
                    out.push(p);
                }
            }
        }
    }
    out
}

fn save_status(env: &TestEnv) -> String {
    env.ok("app.save_state", json!({}))["status"]
        .as_str()
        .unwrap()
        .to_string()
}

fn wait_task(env: &TestEnv, id: &str) -> (String, Option<AppError>) {
    let start = Instant::now();
    loop {
        {
            let events = env.sink.events.lock();
            for e in events.iter().rev() {
                if let AppEvent::Task(t) = e
                    && t.task_id == id
                    && matches!(t.state.as_str(), "completed" | "failed" | "cancelled")
                {
                    return (t.state.clone(), t.error.clone());
                }
            }
        }
        assert!(
            start.elapsed() < Duration::from_secs(120),
            "task {id} did not finish"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Close and reopen by path; the reopened project holds exactly `expected` content.
#[track_caller]
fn reopen_with(env: &TestEnv, path: &str, expected: &str) {
    env.ok("project.close", json!({}));
    env.reopen_project(path);
    integrity_ok(env);
    assert_eq!(content(env), expected, "content after reopening");
    env.ok("project.home", json!({}));
}

fn out_dir(env: &TestEnv, name: &str) -> PathBuf {
    let d = env.dir.path().join(name);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[cfg(windows)]
fn set_readonly(path: &Path, on: bool) {
    let mut p = std::fs::metadata(path).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    p.set_readonly(on);
    std::fs::set_permissions(path, p).unwrap();
}

/// Clears the read-only attribute even when an assertion fails.
#[cfg(windows)]
struct ReadOnly(PathBuf);

#[cfg(windows)]
impl ReadOnly {
    fn set(p: &Path) -> ReadOnly {
        set_readonly(p, true);
        ReadOnly(p.to_path_buf())
    }
}

#[cfg(windows)]
impl Drop for ReadOnly {
    fn drop(&mut self) {
        if self.0.exists() {
            set_readonly(&self.0, false);
        }
    }
}

/// A file held open the way an on-access scanner or backup agent holds it: read access,
/// sharing only reads (no `FILE_SHARE_WRITE`, no `FILE_SHARE_DELETE`). Released on drop.
#[cfg(windows)]
fn av_lock(p: &Path) -> std::fs::File {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_SHARE_READ: u32 = 0x1;
    std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(p)
        .unwrap_or_else(|e| panic!("lock {}: {e}", p.display()))
}

// ------------------------------------------------------------------------------- disk full

/// The drive fills up: every change is refused with a human message while the drive is full,
/// nothing is half-written, the save indicator shows the problem, and after space is freed
/// the same changes succeed and survive a reopen.
#[cfg(debug_assertions)]
#[test]
fn m1_11_disk_full_and_device_failure_at_commit_lose_nothing() {
    use openframe_application::store::fault::{self, CommitFault};

    let s = seeded();
    let env = &s.env;
    for (f, code) in [
        (CommitFault::DiskFull, "storage.disk_full"),
        (CommitFault::WriteFailed, "storage.write_failed"),
    ] {
        let case = format!("{f:?}");
        let before = content(env);
        let undo_before = scalar(env, "SELECT count(*) FROM sys_undo");
        let activity_before = scalar(env, "SELECT count(*) FROM sys_activity");
        fault::fail_commits(Some(f));
        let attempts = [
            (
                "notes.create",
                json!({ "title": "Lost?", "body": "Written while the drive is full." }),
            ),
            (
                "story.update_card",
                json!({ "id": s.card, "shortDescription": "Rewritten while full" }),
            ),
            (
                "screenplay.apply_edits",
                json!({ "draftId": s.draft, "ops": [{ "op": "updateElement", "id": s.element, "text": "Typed while full." }] }),
            ),
            ("history.undo", json!({})),
        ];
        for (op, args) in attempts.iter() {
            let e = expect_err(env, op, args.clone(), &format!("{case} {op}"));
            assert_eq!(e.code_str(), code, "{case} {op}: {}", e.message);
            assert!(e.retryable, "{case} {op}: the user can retry");
            assert_eq!(
                save_status(env),
                "Error",
                "{case} {op}: the save indicator shows it"
            );
        }
        if f == CommitFault::DiskFull {
            let e = env
                .call("notes.create", json!({ "body": "x" }))
                .unwrap_err();
            assert!(e.message.contains("drive is full"), "{}", e.message);
        }
        fault::fail_commits(None);
        // Nothing was written, not even partly.
        assert_eq!(content(env), before, "{case}: content unchanged");
        assert_eq!(
            scalar(env, "SELECT count(*) FROM sys_undo"),
            undo_before,
            "{case}: no undo step"
        );
        assert_eq!(
            scalar(env, "SELECT count(*) FROM sys_activity"),
            activity_before,
            "{case}: no activity entry"
        );
        integrity_ok(env);
        // Space freed: the same changes go through, and the indicator recovers.
        for (op, args) in attempts.iter().take(3) {
            env.ok(op, args.clone());
        }
        assert_eq!(save_status(env), "Saved", "{case}: indicator recovers");
        let after = content(env);
        assert_ne!(after, before);
        reopen_with(env, &s.path, &after);
    }
}

/// An export onto a full drive fails with the drive-full message, leaves the previous file and
/// no stray file, changes nothing in the project, and works once space is freed.
#[cfg(debug_assertions)]
#[test]
fn m1_11_export_to_a_full_drive_leaves_the_previous_file() {
    use openframe_application::modules::exports::fault;

    let s = seeded();
    let env = &s.env;
    let dir = out_dir(env, "exports");
    let dest = dir.join("Railway outline.pdf");
    std::fs::write(&dest, b"previous export").unwrap();
    let before = content(env);
    fault::disk_full(true);
    let e = expect_err(
        env,
        "story.export_board",
        json!({ "format": "pdf", "path": dest }),
        "export on a full drive",
    );
    fault::disk_full(false);
    assert_eq!(e.code_str(), "storage.disk_full");
    assert_eq!(std::fs::read(&dest).unwrap(), b"previous export");
    assert_eq!(files_in(&dir), vec!["Railway outline.pdf".to_string()]);
    assert_eq!(content(env), before);
    let ok = env.ok(
        "story.export_board",
        json!({ "format": "pdf", "path": dest }),
    );
    assert!(std::fs::read(&dest).unwrap().starts_with(b"%PDF"), "{ok}");
    assert_eq!(files_in(&dir), vec!["Railway outline.pdf".to_string()]);
}

// ---------------------------------------------------------------------- permission denied

/// RAII guard for an `icacls /deny` entry on a folder (Everyone may not write).
#[cfg(windows)]
struct DenyWrite(PathBuf);

#[cfg(windows)]
impl DenyWrite {
    fn set(dir: &Path) -> DenyWrite {
        let st = std::process::Command::new("icacls")
            .arg(dir)
            .args(["/deny", "*S-1-1-0:(W)"])
            .output()
            .expect("icacls");
        assert!(st.status.success(), "icacls /deny: {st:?}");
        DenyWrite(dir.to_path_buf())
    }
}

#[cfg(windows)]
impl Drop for DenyWrite {
    fn drop(&mut self) {
        let _ = std::process::Command::new("icacls")
            .arg(&self.0)
            .args(["/remove:d", "*S-1-1-0"])
            .output();
    }
}

/// The chosen export folder does not allow writing (ACL): a permission message, no file, no
/// project change, and the export works once permission is restored.
#[cfg(windows)]
#[test]
fn m1_11_permission_denied_on_the_export_folder() {
    let s = seeded();
    let env = &s.env;
    let dir = out_dir(env, "no-write");
    let before = content(env);
    let guard = DenyWrite::set(&dir);
    for (op, args) in [
        (
            "story.export_board",
            json!({ "format": "pdf", "path": dir.join("outline.pdf") }),
        ),
        (
            "screenplay.export",
            json!({ "draftId": s.draft, "format": "pdf", "path": dir.join("script.pdf") }),
        ),
    ] {
        let e = expect_err(env, op, args, op);
        assert_eq!(e.code_str(), "storage.access_denied", "{op}: {}", e.message);
        assert!(e.message.contains("permission"), "{}", e.message);
    }
    drop(guard);
    assert!(
        files_in(&dir).is_empty(),
        "no file was created: {:?}",
        files_in(&dir)
    );
    assert_eq!(content(env), before);
    env.ok(
        "screenplay.export",
        json!({ "draftId": s.draft, "format": "pdf", "path": dir.join("script.pdf") }),
    );
    assert_eq!(files_in(&dir), vec!["script.pdf".to_string()]);
}

/// The project database is read-only (attribute set by another tool or a restore from read-only
/// media): OpenFrame never changes the file, every refusal is a human message, and the project
/// opens normally with all content once the attribute is cleared.
#[cfg(windows)]
#[test]
fn m1_11_read_only_project_database_is_never_modified() {
    let s = seeded();
    let env = &s.env;
    let expected = content(env);
    env.ok("project.close", json!({}));
    let db = Path::new(&s.path).join("project.sqlite");
    let bytes_before = std::fs::read(&db).unwrap();
    {
        let _ro = ReadOnly::set(&db);
        match env.call("project.open", json!({ "path": s.path })) {
            Err(e) => {
                assert_human(&e, "open read-only database");
                assert!(
                    e.code_str().starts_with("storage."),
                    "{}: {}",
                    e.code_str(),
                    e.message
                );
            }
            Ok(_) => {
                // Opened: every change must be refused with the permission message.
                let e = expect_err(
                    env,
                    "notes.create",
                    json!({ "body": "Written to a read-only file." }),
                    "write to read-only database",
                );
                assert_eq!(e.code_str(), "storage.access_denied", "{}", e.message);
                let _ = env.call("project.close", json!({}));
            }
        }
        assert_eq!(
            std::fs::read(&db).unwrap(),
            bytes_before,
            "the read-only database was not modified"
        );
    }
    env.reopen_project(&s.path);
    integrity_ok(env);
    assert_eq!(content(env), expected);
    env.ok("notes.create", json!({ "body": "Writable again." }));
}

// ------------------------------------------------------------------------ path disappears

/// The export folder is deleted after the user chose it (Save dialog → write).
#[test]
fn m1_11_export_folder_deleted_before_the_write() {
    let s = seeded();
    let env = &s.env;
    let dir = out_dir(env, "gone");
    std::fs::remove_dir(&dir).unwrap();
    let before = content(env);
    for (op, args) in [
        (
            "story.export_board",
            json!({ "format": "pdf", "path": dir.join("outline.pdf") }),
        ),
        (
            "screenplay.export",
            json!({ "draftId": s.draft, "format": "pdf", "path": dir.join("script.pdf") }),
        ),
    ] {
        let e = expect_err(env, op, args, op);
        // Every export destination check (exports, interchange, diagnostics) uses this code,
        // and other suites pin it; the message carries the reconnect hint.
        assert_eq!(e.code_str(), "export.folder_missing", "{op}: {}", e.message);
        assert!(e.message.contains("reconnect"), "{}", e.message);
    }
    // The backup destination is checked before the task starts, so the refusal is normally
    // immediate (`not_found.folder` from the shared destination check); a task that started
    // anyway must fail with a human message.
    match env.call(
        "packages.create_backup",
        json!({ "path": dir.join("Railway.ofbackup") }),
    ) {
        Err(e) => {
            assert_human(&e, "backup into a deleted folder");
            assert_eq!(e.code_str(), "not_found.folder", "{}", e.message);
        }
        Ok(started) => {
            let id = started["taskId"].as_str().unwrap();
            let (state, err) = wait_task(env, id);
            assert_eq!(state, "failed");
            assert_human(err.as_ref().unwrap(), "backup into a deleted folder");
        }
    }
    assert!(
        !dir.exists(),
        "OpenFrame did not recreate a folder the user deleted"
    );
    assert_eq!(content(env), before);
}

/// The project folder is moved while the project is closed: opening the old location says it
/// can't be found (with the reconnect hint), and the new location opens with everything.
#[test]
fn m1_11_project_folder_moved_while_closed() {
    let s = seeded();
    let env = &s.env;
    let expected = content(env);
    env.ok("project.close", json!({}));
    let moved = env.dir.path().join("moved").join("Railway project");
    std::fs::create_dir_all(moved.parent().unwrap()).unwrap();
    std::fs::rename(&s.path, &moved).unwrap();
    let e = expect_err(
        env,
        "project.open",
        json!({ "path": s.path }),
        "open a moved project",
    );
    assert_eq!(e.code_str(), "not_found.project_folder");
    assert!(e.message.contains("reconnect"), "{}", e.message);
    env.reopen_project(&moved.to_string_lossy());
    integrity_ok(env);
    assert_eq!(content(env), expected);
}

/// `recovery/` (checkpoint and session marker) is deleted while the project is open (a cleanup
/// tool, a sync client): Save must still work, and nothing is lost.
#[test]
fn m1_11_recovery_folder_deleted_while_open() {
    let s = seeded();
    let env = &s.env;
    let recovery = Path::new(&s.path).join("recovery");
    assert!(recovery.is_dir());
    std::fs::remove_dir_all(&recovery).unwrap();
    env.ok(
        "notes.create",
        json!({ "body": "After the folder vanished." }),
    );
    let expected = content(env);
    let saved = env.call("project.save", json!({}));
    if let Err(e) = &saved {
        assert_human(e, "save after recovery/ vanished");
    }
    assert!(
        saved.is_ok(),
        "PRODUCT DEFECT (failure injection): Save fails once the project's recovery folder is gone ({:?}); \
         openframe-persistence backup_to opens `<recovery>/checkpoint.partial` without creating the folder",
        saved
            .as_ref()
            .err()
            .map(|e| (e.code_str().to_string(), e.message.clone()))
    );
    assert!(recovery.join("checkpoint.sqlite").is_file());
    reopen_with(env, &s.path, &expected);
}

// ---------------------------------------------------------------- external volume removed

/// RAII `subst` drive letter (a stand-in for an external drive that can be "unplugged").
#[cfg(windows)]
struct Subst {
    letter: char,
    mapped: bool,
}

#[cfg(windows)]
impl Subst {
    fn map(target: &Path) -> Subst {
        let letter = ('M'..='Z')
            .rev()
            .find(|l| !Path::new(&format!("{l}:\\")).exists())
            .expect("a free drive letter");
        let st = std::process::Command::new("subst")
            .arg(format!("{letter}:"))
            .arg(target)
            .output()
            .expect("subst");
        assert!(st.status.success(), "subst: {st:?}");
        Subst {
            letter,
            mapped: true,
        }
    }
    fn root(&self) -> PathBuf {
        PathBuf::from(format!("{}:\\", self.letter))
    }
    fn unplug(&mut self) {
        let st = std::process::Command::new("subst")
            .arg(format!("{}:", self.letter))
            .arg("/d")
            .output()
            .expect("subst /d");
        assert!(st.status.success(), "subst /d: {st:?}");
        self.mapped = false;
    }
}

#[cfg(windows)]
impl Drop for Subst {
    fn drop(&mut self) {
        if self.mapped {
            let _ = std::process::Command::new("subst")
                .arg(format!("{}:", self.letter))
                .arg("/d")
                .output();
        }
    }
}

/// A project on an external drive whose drive letter disappears while it is open: exports to
/// the drive fail with the reconnect message, the project can't be reopened from the old path
/// until the drive is back, and then it opens with every change that was acknowledged.
#[cfg(windows)]
#[test]
fn m1_11_external_volume_removed_while_open() {
    let env = TestEnv::new();
    let backing = env.dir.path().join("usb");
    std::fs::create_dir_all(&backing).unwrap();
    let mut drive = Subst::map(&backing);
    env.ok(
        "project.create",
        json!({ "title": "Railway", "projectType": "Feature Film", "parentDir": drive.root() }),
    );
    let s = seed(env);
    let env = &s.env;
    assert!(
        s.path.starts_with(&format!("{}:", drive.letter)),
        "{}",
        s.path
    );
    env.ok(
        "story.export_board",
        json!({ "format": "pdf", "path": drive.root().join("outline.pdf") }),
    );
    let before_unplug = content(env);

    drive.unplug();
    let e = expect_err(
        env,
        "story.export_board",
        json!({ "format": "pdf", "path": drive.root().join("outline-2.pdf") }),
        "export to a removed drive",
    );
    assert_eq!(e.code_str(), "export.folder_missing");
    assert!(e.message.contains("reconnect"), "{}", e.message);
    // Writes either still reach the (cached) volume or fail with a storage error; whatever
    // returned success must be there after the drive comes back.
    let acknowledged = env
        .call("notes.create", json!({ "body": "Typed after unplugging." }))
        .is_ok();
    if let Err(e) = env.call("project.close", json!({})) {
        assert_human(&e, "close with the drive removed");
    }
    let e = expect_err(
        env,
        "project.open",
        json!({ "path": s.path }),
        "open from a removed drive",
    );
    assert_eq!(e.code_str(), "not_found.project_folder");
    assert!(e.message.contains("reconnect"), "{}", e.message);

    // Plug the drive back in (same letter, same data).
    let back = std::process::Command::new("subst")
        .arg(format!("{}:", drive.letter))
        .arg(&backing)
        .output()
        .unwrap();
    assert!(back.status.success());
    drive.mapped = true;
    env.reopen_project(&s.path);
    integrity_ok(env);
    let now = content(env);
    let notes_after = scalar(
        env,
        "SELECT count(*) FROM project_note WHERE body='Typed after unplugging.'",
    );
    assert_eq!(
        notes_after, acknowledged as i64,
        "an acknowledged write is never lost; a refused one never appears"
    );
    for line in before_unplug.lines() {
        assert!(now.contains(line), "lost after the drive came back: {line}");
    }
    env.ok("project.close", json!({}));
}

// ------------------------------------------------------------------- read-only destination

/// Exports and backups over an existing read-only file: refused with a human message, the old
/// file is untouched and no temporary file is left next to it.
#[cfg(windows)]
#[test]
fn m1_11_read_only_destination_keeps_the_old_file() {
    let s = seeded();
    let env = &s.env;
    let dir = out_dir(env, "read-only");
    let before = content(env);
    for (name, op, args) in [
        (
            "script.pdf",
            "screenplay.export",
            json!({ "draftId": s.draft, "format": "pdf" }),
        ),
        (
            "outline.pdf",
            "story.export_board",
            json!({ "format": "pdf" }),
        ),
    ] {
        let dest = dir.join(name);
        std::fs::write(&dest, b"old").unwrap();
        let _ro = ReadOnly::set(&dest);
        let mut args = args;
        args["path"] = json!(dest);
        let e = expect_err(env, op, args, op);
        assert!(
            e.code_str().starts_with("storage.") || e.code_str().starts_with("export."),
            "{op}: {}",
            e.code_str()
        );
        assert_eq!(std::fs::read(&dest).unwrap(), b"old", "{op}");
    }
    let backup = dir.join("Railway.ofbackup");
    std::fs::write(&backup, b"old backup").unwrap();
    {
        let _ro = ReadOnly::set(&backup);
        let started = env.ok("packages.create_backup", json!({ "path": backup }));
        let (state, err) = wait_task(env, started["taskId"].as_str().unwrap());
        assert_eq!(state, "failed", "backup over a read-only file");
        assert_human(err.as_ref().unwrap(), "backup over a read-only file");
        assert_eq!(std::fs::read(&backup).unwrap(), b"old backup");
    }
    assert_eq!(
        files_in(&dir),
        vec![
            "Railway.ofbackup".to_string(),
            "outline.pdf".to_string(),
            "script.pdf".to_string()
        ],
        "no temporary file left next to the destination"
    );
    assert_eq!(content(env), before);
}

// ------------------------------------------------------------ sleep/resume during indexing

/// The computer sleeps while the intelligence index is embedding: the embedding runtime stops
/// answering (stalled) or comes back broken (failing). Edits and saves never wait for it, and
/// after resume the index catches up completely.
#[test]
fn m1_11_sleep_and_resume_during_indexing() {
    use std::sync::Arc;

    use openframe_application::modules::ai::intelligence::{self, IndexSettings};
    use openframe_application::modules::ai::retrieval::IndexState;
    use openframe_search::Embedder;
    use openframe_test_support::embedder::ConceptEmbedder;

    let env = TestEnv::with_project("Railway", "Feature Film");
    intelligence::service(&env.core).set_settings(IndexSettings {
        debounce: Duration::from_millis(0),
        max_latency: Duration::from_millis(0),
        embed_retry: Duration::from_millis(100),
        embed_batch: 2,
        ..IndexSettings::default()
    });
    let e = Arc::new(ConceptEmbedder::with_dim(64));
    intelligence::service(&env.core).set_embedder(Some(e.clone() as Arc<dyn Embedder>));
    let s = seed(env);
    let env = &s.env;
    let ix = intelligence::activate(&env.core).unwrap();
    assert!(ix.wait_idle(Duration::from_secs(60)));

    for (mode, stall) in [("stalled", true), ("failing", false)] {
        // Sleep: the runtime stops in the middle of a rebuild.
        if stall {
            e.set_stalled(true);
        } else {
            e.set_failing(true);
        }
        let calls = e.calls();
        env.ok("ai.index_rebuild", json!({}));
        let deadline = Instant::now() + Duration::from_secs(10);
        while e.calls() == calls && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        // While asleep: editing and saving never wait for the index.
        for i in 0..20 {
            let t = Instant::now();
            env.ok(
                "notes.create",
                json!({ "title": format!("While {mode} {i}"), "body": "The station is dark." }),
            );
            env.ok("project.save", json!({}));
            assert!(
                t.elapsed() < Duration::from_secs(2),
                "{mode}: an edit waited {:?} for the index",
                t.elapsed()
            );
        }
        assert_ne!(
            ix.snapshot().state,
            IndexState::Current,
            "{mode}: index not caught up yet"
        );
        // Resume.
        e.set_stalled(false);
        e.set_failing(false);
        // `wait_idle` counts "waiting for the next embedding retry" as idle (by design, so a
        // question never blocks on a broken runtime). After the failing phase the indexer is
        // on that retry timer, so poll the state until the backlog is embedded (bounded).
        let deadline = Instant::now() + Duration::from_secs(120);
        while ix.snapshot().state != IndexState::Current && Instant::now() < deadline {
            ix.wait_idle(Duration::from_millis(200));
            std::thread::sleep(Duration::from_millis(20));
        }
        let snap = ix.snapshot();
        assert_eq!(snap.state, IndexState::Current, "{mode}: {snap:?}");
        let expected = env
            .core
            .project()
            .unwrap()
            .store
            .read(|c| Ok(intelligence::documents::all_keys(c)?.len() as u64))
            .unwrap();
        assert_eq!(snap.counts.documents, expected, "{mode}");
        assert_eq!(snap.counts.embedded, snap.counts.chunks, "{mode}");
    }
    integrity_ok(env);
}

// ----------------------------------------------------------------- antivirus-locked files

/// An on-access scanner holds the export destination: the write is refused with a human
/// message, the old file stays, and no temporary file is left behind. Released: it works.
#[cfg(windows)]
#[test]
fn m1_11_antivirus_lock_on_the_export_destination() {
    let s = seeded();
    let env = &s.env;
    let dir = out_dir(env, "scanned");
    for (name, op, args) in [
        (
            "script.pdf",
            "screenplay.export",
            json!({ "draftId": s.draft, "format": "pdf" }),
        ),
        (
            "outline.pdf",
            "story.export_board",
            json!({ "format": "pdf" }),
        ),
    ] {
        let dest = dir.join(name);
        std::fs::write(&dest, b"old").unwrap();
        let mut args = args;
        args["path"] = json!(dest);
        {
            let _lock = av_lock(&dest);
            let e = expect_err(env, op, args.clone(), &format!("{op} onto a locked file"));
            assert!(
                e.code_str().starts_with("storage.") || e.code_str().starts_with("export."),
                "{op}: {}",
                e.code_str()
            );
            assert_eq!(std::fs::read(&dest).unwrap(), b"old");
            assert!(
                partials_under(&dir).is_empty(),
                "{op}: temporary file left: {:?}",
                partials_under(&dir)
            );
        }
        env.ok(op, args);
        assert!(std::fs::read(&dest).unwrap().starts_with(b"%PDF"), "{op}");
    }
    assert_eq!(
        files_in(&dir),
        vec!["outline.pdf".to_string(), "script.pdf".to_string()]
    );
}

/// A scanner holds the checkpoint while the user saves: Save reports a human error, the
/// previous checkpoint is intact, no data is lost, and the next Save (lock released) works and
/// leaves no temporary file.
#[cfg(windows)]
#[test]
fn m1_11_antivirus_lock_on_the_checkpoint_during_save() {
    let s = seeded();
    let env = &s.env;
    env.ok("project.save", json!({}));
    let checkpoint = Path::new(&s.path)
        .join("recovery")
        .join("checkpoint.sqlite");
    let old = std::fs::read(&checkpoint).unwrap();
    env.ok("notes.create", json!({ "body": "Saved while scanned." }));
    let expected = content(env);
    {
        let _lock = av_lock(&checkpoint);
        let e = expect_err(
            env,
            "project.save",
            json!({}),
            "save with a locked checkpoint",
        );
        assert!(e.code_str().starts_with("storage."), "{}", e.code_str());
        assert_eq!(
            std::fs::read(&checkpoint).unwrap(),
            old,
            "previous checkpoint intact"
        );
        // The change itself is already persisted; only the checkpoint copy failed.
        assert_eq!(content(env), expected);
    }
    env.ok("project.save", json!({}));
    assert!(
        partials_under(Path::new(&s.path)).is_empty(),
        "temporary file left: {:?}",
        partials_under(Path::new(&s.path))
    );
    reopen_with(env, &s.path, &expected);
}

/// A scanner holds `project.sqlite` (no write or delete sharing) while the user opens the
/// project: a human error, the file is not damaged, and it opens once the scan ends.
#[cfg(windows)]
#[test]
fn m1_11_antivirus_lock_on_the_database_at_open() {
    let s = seeded();
    let env = &s.env;
    let expected = content(env);
    env.ok("project.close", json!({}));
    let db = Path::new(&s.path).join("project.sqlite");
    let before = std::fs::read(&db).unwrap();
    {
        let _lock = av_lock(&db);
        let e = expect_err(
            env,
            "project.open",
            json!({ "path": s.path }),
            "open while scanned",
        );
        assert!(
            e.code_str().starts_with("storage.")
                || e.code_str().starts_with("project_format.in_use"),
            "{}: {}",
            e.code_str(),
            e.message
        );
        assert_ne!(
            e.code_str(),
            "project_format.corrupt",
            "a lock is not damage"
        );
    }
    assert_eq!(std::fs::read(&db).unwrap(), before);
    env.reopen_project(&s.path);
    integrity_ok(env);
    assert_eq!(content(env), expected);
}

/// The file the user imports is held exclusively by a scanner: a human error and nothing is
/// added to the project.
#[cfg(windows)]
#[test]
fn m1_11_antivirus_lock_on_an_import_source() {
    use std::os::windows::fs::OpenOptionsExt;

    let s = seeded();
    let env = &s.env;
    let src = env.write_file("still.png", &openframe_test_support::stress::png(8, 8, 1));
    let before = content(env);
    let assets = scalar(env, "SELECT count(*) FROM asset");
    {
        // Exclusive: no sharing at all, as some scanners hold a file being inspected.
        let _lock = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&src)
            .unwrap();
        let e = expect_err(
            env,
            "files.add",
            json!({ "paths": [src], "mode": "copy" }),
            "import a locked file",
        );
        assert!(
            e.code_str().starts_with("storage.") || e.code_str().starts_with("import."),
            "{}: {}",
            e.code_str(),
            e.message
        );
    }
    assert_eq!(content(env), before);
    assert_eq!(scalar(env, "SELECT count(*) FROM asset"), assets);
    assert!(partials_under(Path::new(&s.path)).is_empty());
}

// ---------------------------------------------------------------------------- long paths

/// A project more than 260 characters deep: create, edit, save, export, back up, close and
/// reopen all work (or, where Windows refuses, fail with a human message and leave nothing).
#[cfg(windows)]
#[test]
fn m1_11_very_long_paths() {
    let env = TestEnv::new();
    let mut parent = env.dir.path().to_path_buf();
    while parent.as_os_str().len() < 280 {
        parent.push("A folder with a deliberately long name for path tests");
    }
    std::fs::create_dir_all(&parent).unwrap();
    env.ok(
        "project.create",
        json!({ "title": "Railway", "projectType": "Feature Film", "parentDir": parent }),
    );
    let s = seed(env);
    let env = &s.env;
    assert!(s.path.len() > 260, "{}", s.path.len());
    env.ok("project.save", json!({}));
    let out = parent.join("exports for the long path test");
    std::fs::create_dir_all(&out).unwrap();
    let pdf = out.join("Railway screenplay export with a long file name.pdf");
    assert!(pdf.as_os_str().len() > 300);
    env.ok(
        "screenplay.export",
        json!({ "draftId": s.draft, "format": "pdf", "path": pdf }),
    );
    assert!(std::fs::read(&pdf).unwrap().starts_with(b"%PDF"));
    let backup = out.join("Railway backup with a long file name.ofbackup");
    let started = env.ok("packages.create_backup", json!({ "path": backup }));
    let (state, err) = wait_task(env, started["taskId"].as_str().unwrap());
    assert_eq!(state, "completed", "{err:?}");
    let expected = content(env);
    reopen_with(env, &s.path, &expected);
    assert!(partials_under(&parent).is_empty());
}

// ------------------------------------------------------------------- non-ASCII project paths

/// Projects in `Résumé/Café/日本` with a non-ASCII title: the full lifecycle works, search
/// finds the content, exports and backups land in non-ASCII folders, and a crash in that
/// folder is recovered.
#[test]
fn m1_11_non_ascii_project_paths() {
    let env = TestEnv::new();
    let parent = env.dir.path().join("Résumé").join("Café").join("日本");
    std::fs::create_dir_all(&parent).unwrap();
    env.ok(
        "project.create",
        json!({ "title": "Café Noir — Été", "projectType": "Feature Film", "parentDir": parent }),
    );
    let s = seed(env);
    let env = &s.env;
    assert!(
        s.path.contains("日本") && s.path.contains("Café"),
        "{}",
        s.path
    );
    env.ok(
        "notes.create",
        json!({ "title": "Déjà vu", "body": "Rendez-vous à la gare, 東京 at night." }),
    );
    let hits = env.ok("search.query", json!({ "text": "Déjà", "limit": 5 }));
    assert!(!hits.as_array().unwrap().is_empty(), "{hits}");
    env.ok("project.save", json!({}));
    let out = parent.join("Exportés");
    std::fs::create_dir_all(&out).unwrap();
    let pdf = out.join("Scénario.pdf");
    env.ok(
        "screenplay.export",
        json!({ "draftId": s.draft, "format": "pdf", "path": pdf }),
    );
    assert!(std::fs::read(&pdf).unwrap().starts_with(b"%PDF"));
    let backup = out.join("Sauvegarde 日本.ofbackup");
    let started = env.ok("packages.create_backup", json!({ "path": backup }));
    let (state, err) = wait_task(env, started["taskId"].as_str().unwrap());
    assert_eq!(state, "completed", "{err:?}");
    let expected = content(env);
    reopen_with(env, &s.path, &expected);
    let recent = env.ok("project.list_recent", json!({}));
    assert!(recent.to_string().contains("Café Noir"), "{recent}");
    // A crash in a non-ASCII folder is recovered with the latest changes.
    env.ok("notes.create", json!({ "body": "Juste avant la panne." }));
    let expected = content(env);
    let path = s.path.clone();
    let env = s.env.crash_and_restart();
    let opened = env.reopen_project(&path);
    assert!(!opened["recovery"].is_null(), "recovery offered: {opened}");
    integrity_ok(&env);
    assert_eq!(content(&env), expected);
    let _ = (s.card, s.note, s.element);
}

// ------------------------------------------------------------------ sleep notification

/// Sleep notification (platform power events): before the computer sleeps, every committed
/// change of the open project and of the Global Idea Vault is folded from the write-ahead log
/// into the database file, so a battery that runs out during sleep loses nothing.
#[test]
fn sleep_notification_folds_committed_work_into_the_database_files() {
    use openframe_platform::power::PowerEvent;
    let s = seeded();
    let env = &s.env;
    env.ok(
        "vault.create",
        json!({ "store": "global", "itemType": "note", "body": "Harbour at dawn." }),
    );
    env.ok(
        "notes.create",
        json!({ "body": "Written just before sleep." }),
    );
    let wal = |db: &Path| {
        let mut p = db.as_os_str().to_owned();
        p.push("-wal");
        std::fs::metadata(PathBuf::from(p))
            .map(|m| m.len())
            .unwrap_or(0)
    };
    let db = Path::new(&s.path).join("project.sqlite");
    let vault = env.core.config.global_vault_dir.join("vault.sqlite");
    assert!(wal(&db) > 0, "the edit is still in the project log");
    assert!(wal(&vault) > 0, "the vault note is still in the vault log");
    let expected = content(env);
    env.core.on_power_event(PowerEvent::Suspending);
    assert_eq!(wal(&db), 0, "project log folded before sleep");
    assert_eq!(wal(&vault), 0, "vault log folded before sleep");
    // Waking changes nothing and the work is intact.
    env.core.on_power_event(PowerEvent::Resumed);
    assert_eq!(content(env), expected);
    integrity_ok(env);
    let _ = (&s.draft, &s.element, &s.card, &s.note);
}
