//! Small files the app keeps survive bad input (package file 05, item U4; the table is in
//! `docs/release/update-simulation.md`).
//!
//! Files with their own tests elsewhere: `entitlement/notices.json` (`notices.rs`,
//! `entitlement.rs`), `library.json`
//! (`storage.rs`), the AI small records and `models/active.json` (`straft-ai`, `bundled.rs` and
//! `install.rs`). This file covers `app.sqlite` (settings, appearance, recents) and the library
//! pointer. A truncated copy and a garbage copy each: the app starts, the bad file is kept as
//! `*.bad`, nothing is deleted without a copy, and nothing hangs.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use serde_json::json;
use straft_application::events::RecordingSink;
use straft_application::modules::storage::library_ops;
use straft_application::{AppConfig, AppCore};
use straft_test_support::TestEnv;

fn start(root: &Path) -> Arc<AppCore> {
    AppCore::new(
        AppConfig::isolated(root),
        Arc::new(RecordingSink::default()),
    )
    .expect("a damaged settings file never stops the app from starting")
}

#[test]
fn a_damaged_app_database_is_kept_as_bad_and_the_app_starts_with_defaults() {
    let garbage: &[u8] = b"this is not a database at all, it is some text of about this length";
    // A real file cut in half is the other way a crash or a full disk leaves it.
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    env.ok("appearance.set_mode", json!({ "mode": "dark" }));
    let dir = env.dir.path().to_path_buf();
    let TestEnv {
        core, dir: keep, ..
    } = env;
    core.shutdown();
    drop(core);
    let db = dir.join("appdata").join("app.sqlite");
    for suffix in ["-wal", "-shm"] {
        let _ = fs::remove_file(dir.join("appdata").join(format!("app.sqlite{suffix}")));
    }
    let whole = fs::read(&db).unwrap();
    let truncated = whole[..whole.len() / 2].to_vec();
    assert!(truncated.len() > 4096, "cut in the middle of the database");
    for (label, bad) in [("truncated", truncated), ("garbage", garbage.to_vec())] {
        fs::write(&db, &bad).unwrap();
        let core = start(&dir);
        let actor = core.local_actor();
        let call = |op: &str, args| core.dispatch(&actor, op, args).unwrap();
        assert_eq!(
            fs::read(dir.join("appdata").join("app.sqlite.bad")).unwrap(),
            bad,
            "{label}: kept beside the new file"
        );
        // Defaults: System mode, no recents. The app works and keeps working after a restart.
        assert_eq!(
            call("appearance.get", json!({}))["mode"],
            "system",
            "{label}"
        );
        assert_eq!(call("project.list_recent", json!({})), json!([]), "{label}");
        call(
            "project.create",
            json!({ "title": "After the damage", "projectType": "Short Film" }),
        );
        core.shutdown();
        drop(core);
        let core = start(&dir);
        let recents = core
            .dispatch(&core.local_actor(), "project.list_recent", json!({}))
            .unwrap();
        assert_eq!(recents[0]["title"], "After the damage", "{label}");
        core.shutdown();
        drop(core);
        // The next round damages the new file the same way.
        let _ = fs::remove_file(dir.join("appdata").join("app.sqlite.bad"));
    }
    drop(keep);
}

#[test]
fn a_newer_app_database_is_not_treated_as_damage() {
    // Starting an older build over a newer database is a refusal, never a quiet reset.
    let dir = tempfile::tempdir().unwrap();
    let appdata = dir.path().join("appdata");
    fs::create_dir_all(&appdata).unwrap();
    {
        let c = rusqlite::Connection::open(appdata.join("app.sqlite")).unwrap();
        c.execute_batch("PRAGMA user_version = 999; CREATE TABLE sys_settings(key TEXT);")
            .unwrap();
    }
    let result = AppCore::new(
        AppConfig::isolated(dir.path()),
        Arc::new(RecordingSink::default()),
    );
    assert!(result.is_err());
    assert!(!appdata.join("app.sqlite.bad").exists());
}

#[test]
fn a_truncated_or_garbage_library_pointer_is_ignored_and_kept_as_bad() {
    let dir = tempfile::tempdir().unwrap();
    let pointer = dir.path().join("library-pointer.json");
    let good = format!(
        r#"{{ "format": 1, "libraryRoot": {}, "updatedAt": 1 }}"#,
        serde_json::to_string(&dir.path().to_string_lossy()).unwrap()
    );
    assert_eq!(
        library_ops::read_pointer(&{
            fs::write(&pointer, &good).unwrap();
            pointer.clone()
        }),
        Some(dir.path().to_path_buf()),
        "the sample is a pointer the app accepts"
    );
    assert!(!dir.path().join("library-pointer.json.bad").exists());
    for bad in [&good.as_bytes()[..good.len() - 20], b"\xff\xfe garbage"] {
        fs::write(&pointer, bad).unwrap();
        assert_eq!(library_ops::read_pointer(&pointer), None);
        assert_eq!(
            fs::read(dir.path().join("library-pointer.json.bad")).unwrap(),
            bad
        );
    }
}
