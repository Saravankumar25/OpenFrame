//! People have data under the product's earlier name. After the update they must find all of it:
//! databases, the app data folder, the library, project folders, packages and the stored theme.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use straft_application::events::{AppEvent, RecordingSink};
use straft_application::modules::storage::legacy_migration::{Outcome, Plan, bring_over};
use straft_application::modules::storage::library_ops;
use straft_application::schema::{APP_MIGRATIONS, GLOBAL_MIGRATIONS, PROJECT_MIGRATIONS};
use straft_application::{AppConfig, AppCore};
use straft_persistence::migrate::{self, Migration};
use straft_platform::legacy_names;
use straft_test_support::TestEnv;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

fn copy_folder(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap().flatten() {
        if entry.file_type().unwrap().is_dir() {
            copy_folder(&entry.path(), &to.join(entry.file_name()));
        } else {
            fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
        }
    }
}

fn task(env: &TestEnv, started: &Value) -> (String, Value) {
    let id = started["taskId"].as_str().unwrap().to_string();
    let start = Instant::now();
    loop {
        for e in env.sink.events.lock().iter().rev() {
            if let AppEvent::Task(t) = e
                && t.task_id == id
                && matches!(t.state.as_str(), "completed" | "failed" | "cancelled")
            {
                return (
                    t.state.clone(),
                    json!({ "result": t.result, "error": t.error.as_ref().map(|e| e.code.0.clone()) }),
                );
            }
        }
        assert!(start.elapsed() < Duration::from_secs(120), "task timed out");
        std::thread::sleep(Duration::from_millis(20));
    }
}

// ----------------------------------------------------------------- migration checksums

/// The text an earlier version ran: the rename only changed these words in comments.
fn earlier_text(sql: &str) -> String {
    let earlier = legacy_names::PRODUCT_NAME;
    sql.replace(
        "%LOCALAPPDATA%/Straft/",
        &format!("%LOCALAPPDATA%/{earlier}/"),
    )
    .replace("Straft Studio", earlier)
    .replace("Straft Default", &format!("{earlier} Default"))
    .replace(
        "crates/straft-",
        &format!("crates/{}-", legacy_names::PROJECT_EXTENSION),
    )
}

fn earlier_list(list: &[Migration]) -> Vec<Migration> {
    list.iter()
        .map(|m| Migration {
            sql: Box::leak(earlier_text(m.sql).into_boxed_str()),
            ..*m
        })
        .collect()
}

#[test]
fn every_database_that_ran_the_earlier_migrations_has_no_mismatch() {
    let mut changed = 0;
    for list in [APP_MIGRATIONS, GLOBAL_MIGRATIONS, PROJECT_MIGRATIONS] {
        let earlier = earlier_list(list);
        changed += earlier
            .iter()
            .zip(list)
            .filter(|(a, b)| a.sql != b.sql)
            .count();
        let mut conn = straft_persistence::open_in_memory().unwrap();
        migrate::apply(&mut conn, &earlier).unwrap();
        assert!(
            migrate::checksum_mismatches(&conn, &earlier)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            migrate::checksum_mismatches(&conn, list).unwrap(),
            Vec::<u32>::new(),
            "the rename's own edits are accepted"
        );
        assert!(migrate::apply(&mut conn, list).unwrap().is_empty());
        // Comments are all that differs: the same list, tampered, still shows.
        conn.execute(
            "UPDATE _schema_migrations SET checksum=?1 WHERE version=1",
            [&"f".repeat(64)],
        )
        .unwrap();
        assert_eq!(migrate::checksum_mismatches(&conn, list).unwrap(), vec![1]);
    }
    assert_eq!(changed, 11, "exactly the files the rename changed");
}

// ----------------------------------------------------------------- the app data folder

fn at_app_data(root: &Path, app_data: &Path) -> AppConfig {
    AppConfig {
        app_data_dir: app_data.to_path_buf(),
        ..AppConfig::isolated(root)
    }
}

/// An earlier installation: a person with a name, a stored theme and downloaded models.
fn earlier_app_data(root: &Path) -> (PathBuf, String) {
    let old = root.join("Old App Data");
    let sink = Arc::new(RecordingSink::default());
    let core = AppCore::new(at_app_data(root, &old), sink).unwrap();
    core.dispatch(
        &core.local_actor(),
        "app.set_display_name",
        json!({ "displayName": "Meera" }),
    )
    .unwrap();
    let user = core.profile().user_id;
    core.shutdown();
    drop(core);
    let conn = straft_persistence::open_connection(&old.join("app.sqlite"), false).unwrap();
    conn.execute(
        "INSERT INTO sys_appearance_pref(key, value, updated_at) VALUES ('theme', ?1, 1)",
        [legacy_names::DEFAULT_THEME_ID],
    )
    .unwrap();
    // What an earlier version recorded when it ran `app/0002_appearance.sql`.
    conn.execute(
        "UPDATE _schema_migrations SET checksum=?1 WHERE version=2",
        ["149a096edab8b03013e9d1c564069ba0feefad9a6abb9d37ce5d6250e594af7f"],
    )
    .unwrap();
    drop(conn);
    let models = old.join("models");
    fs::create_dir_all(models.join("m-abc")).unwrap();
    fs::write(models.join("m-abc").join("chat.gguf"), "gigabytes").unwrap();
    fs::write(
        models.join("active.json"),
        r#"{"version":"m-abc","tier":"small","previous":null,"activatedAt":1}"#,
    )
    .unwrap();
    (old, user)
}

#[test]
fn an_earlier_installation_starts_with_everything_it_had() {
    let dir = tempfile::tempdir().unwrap();
    let (old, user) = earlier_app_data(dir.path());
    let new = dir.path().join("New App Data");
    assert_eq!(bring_over(&old, &new, &Plan::APP_DATA), Outcome::Copied);
    assert!(old.join("app.sqlite").is_file(), "the old folder stays");

    let sink = Arc::new(RecordingSink::default());
    let core = AppCore::new(at_app_data(dir.path(), &new), sink.clone()).unwrap();
    let conn = straft_persistence::open_connection(&new.join("app.sqlite"), false).unwrap();
    assert!(
        migrate::checksum_mismatches(&conn, APP_MIGRATIONS)
            .unwrap()
            .is_empty()
    );
    drop(conn);
    let env = TestEnv { dir, core, sink };
    let profile = env.core.profile();
    assert_eq!(profile.user_id, user, "the same person");
    assert_eq!(profile.display_name, "Meera");
    // The stored default theme is the same theme under its new id, with no notice about it.
    let look = env.ok("appearance.get", json!({}));
    assert_eq!(look["themeId"], "straft-default", "{look}");
    assert!(look["fallbackNotice"].is_null(), "{look}");
    // Asking for it by the earlier id works as well, and stores the new one.
    let set = env.ok(
        "appearance.set_theme",
        json!({ "themeId": legacy_names::DEFAULT_THEME_ID }),
    );
    assert_eq!(set["themeId"], "straft-default", "{set}");
    // The models came across by rename and the set in use is still the one in use.
    let models = straft_ai::model_download::models_dir(&new);
    assert_eq!(models.active().unwrap().version, "m-abc");
    assert!(new.join("models").join("m-abc").join("chat.gguf").is_file());
    assert!(!old.join("models").exists());
}

/// Changes still in the write-ahead log (the earlier version is running or crashed) are not lost.
#[test]
fn unfolded_changes_of_a_crashed_session_come_across() {
    let dir = tempfile::tempdir().unwrap();
    let (old, _) = earlier_app_data(dir.path());
    let conn = straft_persistence::open_connection(&old.join("app.sqlite"), false).unwrap();
    conn.execute_batch("PRAGMA wal_autocheckpoint=0;").unwrap();
    conn.execute(
        "INSERT INTO sys_settings(key, value_json, updated_at) VALUES ('crash.test', 'true', 1)",
        [],
    )
    .unwrap();
    assert!(
        fs::metadata(old.join("app.sqlite-wal")).unwrap().len() > 0,
        "the change is only in the log"
    );
    let new = dir.path().join("New App Data");
    // `conn` stays open: the earlier version is still running.
    assert_eq!(bring_over(&old, &new, &Plan::APP_DATA), Outcome::Copied);
    assert!(
        fs::metadata(old.join("app.sqlite-wal")).unwrap().len() > 0,
        "its log was not folded"
    );
    conn.execute(
        "INSERT INTO sys_settings(key, value_json, updated_at) VALUES ('still.running', 'true', 1)",
        [],
    )
    .expect("the earlier version keeps working");
    drop(conn);
    assert!(!new.join("app.sqlite-wal").exists(), "taken as one file");
    let back = straft_persistence::open_connection(&new.join("app.sqlite"), false).unwrap();
    let found: i64 = back
        .query_row(
            "SELECT count(*) FROM sys_settings WHERE key='crash.test'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(found, 1);
}

/// A folder an earlier failed start left, with no database: the data still comes across.
#[test]
fn a_new_folder_with_only_logs_still_gets_the_data() {
    let dir = tempfile::tempdir().unwrap();
    let (old, user) = earlier_app_data(dir.path());
    let new = dir.path().join("New App Data");
    fs::create_dir_all(new.join("logs")).unwrap();
    assert_eq!(bring_over(&old, &new, &Plan::APP_DATA), Outcome::Copied);
    let sink = Arc::new(RecordingSink::default());
    let core = AppCore::new(at_app_data(dir.path(), &new), sink).unwrap();
    assert_eq!(core.profile().user_id, user);
    core.shutdown();
    assert_eq!(
        bring_over(&old, &new, &Plan::APP_DATA),
        Outcome::AlreadyThere
    );
}

/// When the copy cannot be made the run uses the old folder, and it starts with all its data.
#[test]
fn the_app_starts_on_the_old_folder_when_the_copy_failed() {
    let dir = tempfile::tempdir().unwrap();
    let (old, user) = earlier_app_data(dir.path());
    // The new folder's place holds a file's parent that cannot exist.
    let blocked = dir.path().join("blocked");
    fs::write(&blocked, "a file, not a folder").unwrap();
    let new = blocked.join("New App Data");
    let outcome = bring_over(&old, &new, &Plan::APP_DATA);
    assert!(
        matches!(outcome, Outcome::UseEarlierFolder { .. }),
        "{outcome:?}"
    );
    assert!(old.join("models").join("m-abc").join("chat.gguf").is_file());
    let sink = Arc::new(RecordingSink::default());
    let core = AppCore::new(at_app_data(dir.path(), &old), sink).unwrap();
    assert_eq!(core.profile().user_id, user);
    core.shutdown();
}

// ----------------------------------------------------------------- the library

/// A used library, as an earlier version left it at `place`: found by the first start and connected
/// where it is, with its identity.
fn found_and_connected(place: impl Fn(&Path) -> PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let sink = Arc::new(RecordingSink::default());
    let core = AppCore::new(AppConfig::isolated_needing_setup(dir.path()), sink.clone()).unwrap();
    let source = TestEnv { dir, core, sink };
    source.ok("library.use_default", json!({}));
    source.ok("app.set_display_name", json!({ "displayName": "Meera" }));
    source.ok(
        "project.create",
        json!({ "title": "Salt Road", "projectType": "Feature Film" }),
    );
    let user = source.core.profile().user_id;
    let made = PathBuf::from(
        source.ok("library.status", json!({}))["root"]
            .as_str()
            .unwrap(),
    );
    source.core.shutdown();

    let dir = tempfile::tempdir().unwrap();
    let library = place(dir.path());
    fs::create_dir_all(library.parent().unwrap()).unwrap();
    copy_folder(&made, &library);
    let sink = Arc::new(RecordingSink::default());
    let core = AppCore::new(AppConfig::isolated_needing_setup(dir.path()), sink.clone()).unwrap();
    let env = TestEnv { dir, core, sink };
    let status = env.ok("library.status", json!({}));
    assert_eq!(status["state"], "needsSetup");
    let found = status["found"].as_array().unwrap();
    assert_eq!(found.len(), 1, "{status}");
    assert_eq!(found[0]["projectCount"], 1);
    assert_eq!(found[0]["ownerName"], "Meera");
    let connected = env.ok(
        "library.connect",
        json!({ "dir": library.to_string_lossy() }),
    );
    assert_eq!(connected["state"], "ready");
    assert_eq!(env.core.profile().user_id, user);
    assert!(library.is_dir(), "the person's folder is not moved");
}

#[test]
fn a_library_in_documents_under_the_earlier_name_is_found() {
    found_and_connected(|root| root.join("Documents").join(legacy_names::PRODUCT_NAME));
}

#[test]
fn a_library_in_the_home_folder_under_the_earlier_name_is_found() {
    found_and_connected(|root| root.join("home").join(legacy_names::PRODUCT_NAME));
}

#[test]
fn the_library_pointer_folder_comes_across_and_still_points_at_the_persons_folder() {
    let dir = tempfile::tempdir().unwrap();
    let library = dir
        .path()
        .join("Documents")
        .join(legacy_names::PRODUCT_NAME);
    fs::create_dir_all(library.join(library_ops::PROJECTS_FOLDER)).unwrap();
    fs::create_dir_all(library.join(library_ops::VAULT_FOLDER)).unwrap();
    let old = dir.path().join("roaming").join(legacy_names::PRODUCT_NAME);
    fs::create_dir_all(&old).unwrap();
    fs::write(
        old.join(library_ops::POINTER_FILE),
        serde_json::to_vec(&json!({
            "format": 1, "libraryRoot": library.to_string_lossy(), "updatedAt": 1,
        }))
        .unwrap(),
    )
    .unwrap();
    let new = dir.path().join("roaming").join(library_ops::LIBRARY_FOLDER);
    assert_eq!(bring_over(&old, &new, &Plan::POINTER), Outcome::Copied);
    assert_eq!(
        library_ops::read_pointer(&new.join(library_ops::POINTER_FILE)).unwrap(),
        library
    );
    // A new start finds the library through it, where it always was.
    let sink = Arc::new(RecordingSink::default());
    let core = AppCore::new(AppConfig::isolated_needing_setup(dir.path()), sink.clone()).unwrap();
    let env = TestEnv { dir, core, sink };
    assert_eq!(env.ok("library.status", json!({}))["state"], "ready");
    assert_eq!(env.core.library_place().unwrap().root, library);
}

// ----------------------------------------------------------------- project folders

/// The open project of `env`, closed and put back the way an earlier version left it: the earlier
/// extension and the earlier manifest file. Returns the folder.
fn as_earlier_project(env: &TestEnv) -> (PathBuf, String) {
    let path = PathBuf::from(env.project_path());
    let id = env.ok("project.current", json!({}))["id"]
        .as_str()
        .unwrap()
        .to_string();
    env.ok("project.close", json!({}));
    let renamed = path.with_extension(legacy_names::PROJECT_EXTENSION);
    fs::rename(&path, &renamed).unwrap();
    to_earlier_manifest(&renamed);
    (renamed, id)
}

fn to_earlier_manifest(folder: &Path) {
    let current = folder.join("straft.json");
    let mut m: Value = serde_json::from_slice(&fs::read(&current).unwrap()).unwrap();
    m["format"] = legacy_names::MANIFEST_FORMAT.into();
    fs::write(
        folder.join(legacy_names::MANIFEST_FILE),
        serde_json::to_vec_pretty(&m).unwrap(),
    )
    .unwrap();
    fs::remove_file(current).unwrap();
}

#[test]
fn an_earlier_project_folder_works_as_before() {
    let env = TestEnv::with_project("Harbour Light", "Feature Film");
    env.ok(
        "notes.create",
        json!({ "title": "Kept", "body": "A note from before the update" }),
    );
    let (folder, id) = as_earlier_project(&env);

    // Locating it: a folder with the earlier manifest is that project.
    env.ok(
        "project.locate",
        json!({ "projectId": id, "path": folder.to_string_lossy() }),
    );
    env.ok("project.close", json!({}));
    to_earlier_manifest(&folder);

    // Recent Projects shows it as available.
    let recent = env.ok("project.list_recent", json!({}));
    let row = recent
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["projectId"] == id.as_str())
        .expect("listed");
    assert_eq!(row["available"], true, "{row}");

    // The library scan finds it once it is not in Recent Projects.
    env.ok("project.remove_recent", json!({ "projectId": id }));
    let found = env.ok("project.discover", json!({}));
    assert_eq!(found[0]["projectId"], id.as_str(), "{found}");

    // Opening it writes the new manifest, removes the earlier file and keeps the folder's name.
    let opened = env.ok("project.open", json!({ "path": folder.to_string_lossy() }));
    assert_eq!(opened["project"]["title"], "Harbour Light");
    assert!(folder.join("straft.json").is_file());
    assert!(!folder.join(legacy_names::MANIFEST_FILE).exists());
    assert!(folder.is_dir(), "the person's folder keeps its name");
    assert!(
        env.ok("notes.list", json!({}))
            .to_string()
            .contains("A note from before the update")
    );

    // Back up, then duplicate and archive it.
    let backup = env.dir.path().join("harbour.ofbackup");
    let (state, _) = task(
        &env,
        &env.ok("packages.create_backup", json!({ "path": backup })),
    );
    assert_eq!(state, "completed");
    let copy = env.ok("project.duplicate", json!({ "projectId": id }));
    assert_ne!(copy["projectId"], id.as_str());
    env.ok("project.open", json!({ "path": folder.to_string_lossy() }));
    env.ok("project.close", json!({}));
    env.ok(
        "project.set_archived",
        json!({ "projectId": id, "archived": true }),
    );
    let all = env.ok("project.list_recent", json!({ "includeArchived": true }));
    assert!(
        all.as_array()
            .unwrap()
            .iter()
            .any(|r| r["projectId"] == id.as_str() && r["archived"] == true),
        "{all}"
    );
}

/// A duplicate of a folder that still holds the earlier file has one manifest, the new one.
#[test]
fn a_duplicate_of_an_earlier_project_has_only_the_new_manifest() {
    let env = TestEnv::with_project("Salt Road", "Feature Film");
    let (folder, id) = as_earlier_project(&env);
    env.ok(
        "project.locate",
        json!({ "projectId": id, "path": folder.to_string_lossy() }),
    );
    env.ok("project.close", json!({}));
    to_earlier_manifest(&folder);
    let copy = env.ok("project.duplicate", json!({ "projectId": id }));
    let copy_folder = PathBuf::from(copy["path"].as_str().unwrap());
    assert!(copy_folder.join("straft.json").is_file());
    assert!(!copy_folder.join(legacy_names::MANIFEST_FILE).exists());
    assert_eq!(copy_folder.extension().unwrap().to_string_lossy(), "straft");
}

/// The folder a person's project lives in may not hold a new projects folder.
#[test]
fn a_folder_inside_an_earlier_project_is_refused_as_a_home_for_projects() {
    let env = TestEnv::with_project("Salt Road", "Feature Film");
    let (folder, _) = as_earlier_project(&env);
    let inside = folder.join("assets");
    let refused = env
        .call(
            "storage.set_projects_dir",
            json!({ "path": inside.to_string_lossy() }),
        )
        .unwrap_err();
    assert!(refused.message.contains("inside a project"), "{refused:?}");
    // By its name alone, too, before it holds a manifest.
    let named = env
        .dir
        .path()
        .join(format!("Unfinished.{}", legacy_names::PROJECT_EXTENSION));
    fs::create_dir_all(named.join("assets")).unwrap();
    let refused = env
        .call(
            "storage.set_projects_dir",
            json!({ "path": named.join("assets").to_string_lossy() }),
        )
        .unwrap_err();
    assert!(refused.message.contains("inside a project"), "{refused:?}");
}

// ----------------------------------------------------------------- packages

/// The kept fixture: a backup an earlier build made, with the earlier format name and manifest file
/// inside the archive, restores.
#[test]
fn a_backup_from_an_earlier_build_restores() {
    let env = TestEnv::new();
    let backup = fixtures().join("packages/format-1/harbour.ofbackup");
    let inspected = env.ok(
        "packages.inspect_package",
        json!({ "path": backup.to_string_lossy() }),
    );
    assert_eq!(inspected["defaultMode"], "open", "{inspected}");
    let (state, report) = task(
        &env,
        &env.ok(
            "packages.import_project",
            json!({ "path": backup.to_string_lossy(), "mode": "open" }),
        ),
    );
    assert_eq!(state, "completed", "{report}");
    let folder = PathBuf::from(env.project_path());
    assert!(folder.join("straft.json").is_file());
    assert!(!folder.join(legacy_names::MANIFEST_FILE).exists());
    assert_eq!(folder.extension().unwrap().to_string_lossy(), "straft");
}
