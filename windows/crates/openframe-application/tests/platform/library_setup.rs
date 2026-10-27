//! The OpenFrame library on the first start, after a reinstall and when it moves (doc 40 section
//! 1, ADR-0038): nothing is created before the person chooses, an existing library is found and
//! connected with its identity, and a moved library takes every new file with it.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use base64::Engine;
use openframe_application::events::{AppEvent, RecordingSink};
use openframe_application::modules::storage::{library, library_ops};
use openframe_application::{AppConfig, AppCore};
use openframe_domain::AppError;
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

/// A computer where OpenFrame starts for the first time: everything lives under `dir`.
fn fresh(dir: tempfile::TempDir) -> TestEnv {
    let sink = Arc::new(RecordingSink::default());
    let core =
        AppCore::new(AppConfig::isolated_needing_setup(dir.path()), sink.clone()).expect("core");
    TestEnv { dir, core, sink }
}

fn new_computer() -> TestEnv {
    fresh(tempfile::tempdir().unwrap())
}

fn quit(env: TestEnv) -> tempfile::TempDir {
    env.core.shutdown();
    let TestEnv { dir, core, .. } = env;
    drop(core);
    dir
}

fn home_library(env: &TestEnv) -> PathBuf {
    env.dir.path().join("home").join("OpenFrame")
}

fn pointer(env: &TestEnv) -> PathBuf {
    env.dir
        .path()
        .join("roaming")
        .join("OpenFrame")
        .join(library_ops::POINTER_FILE)
}

fn text(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

fn same(a: &Value, b: &Path) -> bool {
    a.as_str()
        .is_some_and(|a| library::same_place(Path::new(a), b))
}

fn status(env: &TestEnv) -> Value {
    env.ok("library.status", json!({}))
}

fn finish(env: &TestEnv, started: &Value) -> Result<Value, AppError> {
    let id = started["taskId"].as_str().expect("a task id").to_string();
    let begun = Instant::now();
    loop {
        for e in env.sink.events.lock().iter().rev() {
            if let AppEvent::Task(t) = e
                && t.task_id == id
                && matches!(t.state.as_str(), "completed" | "failed" | "cancelled")
            {
                return match t.state.as_str() {
                    "completed" => Ok(t.result.clone().unwrap_or(Value::Null)),
                    _ => Err(t.error.clone().unwrap_or_else(AppError::cancelled)),
                };
            }
        }
        assert!(
            begun.elapsed() < Duration::from_secs(120),
            "the task did not finish"
        );
        std::thread::sleep(Duration::from_millis(15));
    }
}

/// Every file under `dir`, relative.
fn files_under(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(d) = pending.pop() {
        let Ok(entries) = fs::read_dir(&d) else {
            continue;
        };
        for e in entries.flatten() {
            if e.file_type().unwrap().is_dir() {
                pending.push(e.path());
            } else {
                out.push(text(e.path().strip_prefix(dir).unwrap()));
            }
        }
    }
    out.sort();
    out
}

fn add_idea_with_file(env: &TestEnv, bytes: &[u8]) -> PathBuf {
    env.ok(
        "vault.ingest_bytes",
        json!({
            "store": "global", "itemType": "voice", "fileName": "voice.webm",
            "mediaType": "audio/webm",
            "dataBase64": base64::engine::general_purpose::STANDARD.encode(bytes)
        }),
    );
    let items = env.ok("vault.list", json!({ "store": "global" }));
    items
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|i| i["asset"]["path"].as_str())
        .map(PathBuf::from)
        .find(|p| fs::read(p).is_ok_and(|b| b == bytes))
        .expect("the file of the new idea")
}

// --------------------------------------------------------------- first start

#[test]
fn a_new_computer_needs_setup_and_creates_nothing_before_a_choice() {
    let env = new_computer();
    let s = status(&env);
    assert_eq!(s["state"], "needsSetup");
    assert!(same(&s["suggested"], &home_library(&env)));
    assert_eq!(s["found"], json!([]));
    // The setup screen is the only thing that runs.
    for (op, args) in [
        (
            "project.create",
            json!({ "title": "Salt Road", "projectType": "Feature Film" }),
        ),
        ("vault.list", json!({ "store": "global" })),
        ("storage.info", json!({})),
        ("project.list_recent", json!({})),
    ] {
        let e = env.call(op, args).expect_err(op);
        assert_eq!(e.code_str(), "library.setup_required", "{op}");
    }
    assert!(env.core.global_store().is_err());
    env.core.startup_maintenance();
    // Nothing in the home folder, Documents or the pointer folder, and no saved profile.
    for d in ["home", "Documents", "roaming"] {
        assert!(!env.dir.path().join(d).exists(), "{d} was created");
    }
    let profiles: i64 = env
        .core
        .with_app_db(|c| {
            Ok(c.query_row("SELECT count(*) FROM sys_local_profile", [], |r| r.get(0))?)
        })
        .unwrap();
    assert_eq!(
        profiles, 0,
        "the identity is only saved once a library is chosen"
    );

    // Closing during setup and starting again asks again.
    let again = fresh(quit(env));
    assert_eq!(status(&again)["state"], "needsSetup");
}

#[test]
fn the_default_location_is_created_only_when_confirmed_and_is_remembered() {
    let env = new_computer();
    let s = env.ok("library.use_default", json!({}));
    let root = home_library(&env);
    assert_eq!(s["state"], "ready");
    assert!(same(&s["root"], &root));
    for part in ["library.json", "Projects", "Global Idea Vault"] {
        assert!(root.join(part).exists(), "{part}");
    }
    assert_eq!(
        library_ops::read_pointer(&pointer(&env))
            .as_deref()
            .map(text),
        Some(text(&root))
    );
    // New work lands in it.
    env.ok(
        "project.create",
        json!({ "title": "Salt Road", "projectType": "Feature Film" }),
    );
    assert!(library::is_inside(
        &root.join("Projects"),
        Path::new(&env.project_path())
    ));
    let info = env.ok("storage.info", json!({}));
    assert!(same(&info["libraryDir"], &root));
    // A second setup is refused: Settings moves or locates instead.
    assert_eq!(
        env.call("library.use_default", json!({}))
            .unwrap_err()
            .code_str(),
        "conflict.state"
    );

    let user = env.core.profile().user_id;
    let again = fresh(quit(env));
    assert_eq!(
        status(&again)["state"],
        "ready",
        "no question on a normal start"
    );
    assert_eq!(again.core.profile().user_id, user);
}

#[test]
fn a_new_library_in_a_chosen_folder_and_the_places_it_refuses() {
    let env = new_computer();
    let base = env.dir.path().join("drive-d");
    fs::create_dir_all(&base).unwrap();
    let s = env.ok("library.create_at", json!({ "parent": text(&base) }));
    let root = base.join("OpenFrame");
    assert!(same(&s["root"], &root));
    assert!(s["cloudService"].is_null());
    assert!(root.join("library.json").is_file());

    // Elsewhere: a folder inside a library, one that already holds a library, a synced folder.
    let other = new_computer();
    let e = other
        .call(
            "library.create_at",
            json!({ "parent": text(&root.join("Projects")) }),
        )
        .unwrap_err();
    assert_eq!(e.code_str(), "validation.invalid_input", "{}", e.message);
    let e = other
        .call("library.create_at", json!({ "parent": text(&base) }))
        .unwrap_err();
    assert_eq!(e.code_str(), "conflict.exists", "{}", e.message);
    assert_eq!(
        status(&other)["state"],
        "needsSetup",
        "a refusal changes nothing"
    );
    let synced = other.dir.path().join("OneDrive");
    fs::create_dir_all(&synced).unwrap();
    let s = other.ok("library.create_at", json!({ "parent": text(&synced) }));
    assert_eq!(s["cloudService"], "OneDrive", "a warning, not a refusal");
}

// ------------------------------------------------------- finding and connecting

/// A library made and used on another computer: a project with a private note, and a vault idea.
/// Returns its folder and the person's id.
fn used_library(dir: &Path) -> (tempfile::TempDir, PathBuf, String) {
    let env = new_computer();
    env.ok("library.use_default", json!({}));
    env.ok("app.set_display_name", json!({ "displayName": "Meera" }));
    env.ok(
        "project.create",
        json!({ "title": "Salt Road", "projectType": "Feature Film" }),
    );
    env.ok("private_note.create", json!({ "body": "REMEMBER-ME" }));
    env.ok(
        "vault.create",
        json!({ "store": "global", "itemType": "note", "body": "An idea" }),
    );
    let user = env.core.profile().user_id;
    let root = home_library(&env);
    let other = quit(env);
    fs::create_dir_all(dir.parent().unwrap()).unwrap();
    fs::rename(&root, dir).unwrap();
    (other, dir.to_path_buf(), user)
}

#[test]
fn an_earlier_library_in_documents_is_found_and_connected_with_its_identity() {
    let dir = tempfile::tempdir().unwrap();
    let docs_library = dir.path().join("Documents").join("OpenFrame");
    let (_old, _, user) = used_library(&docs_library);
    let env = fresh(dir);
    let s = status(&env);
    assert_eq!(s["state"], "needsSetup");
    let found = s["found"].as_array().unwrap();
    assert_eq!(found.len(), 1, "{s}");
    assert_eq!(found[0]["kind"], "libraryRoot");
    assert_eq!(found[0]["projectCount"], 1);
    assert_eq!(found[0]["ideaCount"], 1);
    assert_eq!(found[0]["ownerName"], "Meera");
    // Looking changed nothing.
    let before = files_under(&docs_library);

    let s = env.ok("library.connect", json!({ "dir": text(&docs_library) }));
    assert_eq!(s["state"], "ready");
    assert_eq!(env.core.profile().user_id, user, "the same person");
    let projects = env.ok("project.discover", json!({}));
    let path = projects[0]["path"].as_str().unwrap().to_string();
    env.reopen_project(&path);
    assert!(
        env.ok("private_note.list", json!({}))
            .to_string()
            .contains("REMEMBER-ME")
    );
    assert!(
        env.ok("vault.list", json!({ "store": "global" }))
            .to_string()
            .contains("An idea")
    );
    assert!(before.iter().all(|f| docs_library.join(f).exists()));
}

#[test]
fn inspect_explains_each_level_and_never_creates_anything() {
    let dir = tempfile::tempdir().unwrap();
    let lib = dir.path().join("drive").join("OpenFrame");
    let (_old, _, _) = used_library(&lib);
    let env = fresh(dir);
    let look = |p: &Path| env.ok("library.inspect", json!({ "dir": text(p) }));
    let project = fs::read_dir(lib.join("Projects"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let lone_vault = env.dir.path().join("lone");
    fs::create_dir_all(&lone_vault).unwrap();
    fs::copy(
        lib.join("Global Idea Vault").join("vault.sqlite"),
        lone_vault.join("vault.sqlite"),
    )
    .unwrap();
    let empty = env.dir.path().join("empty");
    fs::create_dir_all(&empty).unwrap();
    let other = env.dir.path().join("other");
    fs::create_dir_all(&other).unwrap();
    fs::write(other.join("notes.txt"), b"x").unwrap();
    let before = files_under(env.dir.path());

    for (picked, kind, root) in [
        (lib.clone(), "libraryRoot", Some(&lib)),
        (
            lib.parent().unwrap().to_path_buf(),
            "libraryParent",
            Some(&lib),
        ),
        (lib.join("Projects"), "insideLibrary", Some(&lib)),
        (lib.join("Global Idea Vault"), "insideLibrary", Some(&lib)),
        (project.clone(), "project", Some(&lib)),
        (lone_vault.clone(), "vault", None),
        (empty.clone(), "empty", None),
        (other.clone(), "other", None),
    ] {
        let i = look(&picked);
        assert_eq!(i["kind"], kind, "{picked:?}: {i}");
        match root {
            Some(r) => {
                assert!(same(&i["libraryRoot"], r), "{i}");
                assert_eq!(i["projectCount"], 1);
                assert_eq!(i["ideaCount"], 1);
            }
            None => assert!(i["libraryRoot"].is_null(), "{i}"),
        }
        assert!(!i["message"].as_str().unwrap().is_empty());
    }
    assert_eq!(
        look(&lone_vault)["ideaCount"],
        1,
        "a lone vault is counted too"
    );
    assert_eq!(files_under(env.dir.path()), before, "inspect never writes");

    // Picking the folder above the library connects the library itself.
    let s = env.ok(
        "library.connect",
        json!({ "dir": text(lib.parent().unwrap()) }),
    );
    assert!(same(&s["root"], &lib));
}

#[test]
fn a_failed_locate_stays_in_setup_and_creates_nothing() {
    let env = new_computer();
    let empty = env.dir.path().join("empty");
    fs::create_dir_all(&empty).unwrap();
    let e = env
        .call("library.connect", json!({ "dir": text(&empty) }))
        .unwrap_err();
    assert_eq!(e.code_str(), "not_found.library");
    assert!(e.message.contains("empty"), "{}", e.message);
    let e = env
        .call(
            "library.connect",
            json!({ "dir": text(&env.dir.path().join("nowhere")) }),
        )
        .unwrap_err();
    assert_eq!(e.code_str(), "not_found.folder");
    assert_eq!(status(&env)["state"], "needsSetup");
    assert_eq!(fs::read_dir(&empty).unwrap().count(), 0);
    assert!(!home_library(&env).exists());
}

#[test]
fn a_library_that_went_missing_is_asked_for_and_never_recreated() {
    let env = new_computer();
    env.ok("library.use_default", json!({}));
    let root = home_library(&env);
    let dir = quit(env);
    let moved = dir.path().join("usb").join("OpenFrame");
    fs::create_dir_all(moved.parent().unwrap()).unwrap();
    fs::rename(&root, &moved).unwrap();

    let env = fresh(dir);
    let s = status(&env);
    assert_eq!(s["state"], "missing");
    assert!(same(&s["root"], &root));
    assert!(!root.exists(), "never recreated empty");
    env.ok("library.connect", json!({ "dir": text(&moved) }));
    assert_eq!(status(&env)["state"], "ready");
    let again = fresh(quit(env));
    assert!(same(&status(&again)["root"], &moved));
}

#[test]
fn the_pointer_finds_the_library_after_the_app_data_is_deleted() {
    let env = new_computer();
    env.ok("library.use_default", json!({}));
    env.ok("app.set_display_name", json!({ "displayName": "Meera" }));
    let user = env.core.profile().user_id;
    let dir = quit(env);
    for suffix in ["", "-wal", "-shm"] {
        let _ = fs::remove_file(
            dir.path()
                .join("appdata")
                .join(format!("app.sqlite{suffix}")),
        );
    }
    let env = fresh(dir);
    let s = status(&env);
    assert_eq!(s["state"], "ready", "found through the pointer: {s}");
    assert!(same(&s["root"], &home_library(&env)));
    assert_eq!(env.core.profile().user_id, user);
    assert_eq!(env.core.profile().display_name, "Meera");
}

#[test]
fn connecting_the_library_of_someone_else_keeps_the_saved_identity_and_their_file() {
    let dir = tempfile::tempdir().unwrap();
    let theirs = dir.path().join("theirs").join("OpenFrame");
    let (_old, _, their_id) = used_library(&theirs);
    let env = fresh(dir);
    env.ok("library.use_default", json!({}));
    let mine = env.core.profile().user_id;
    env.ok("library.connect", json!({ "dir": text(&theirs) }));
    assert_eq!(
        env.core.profile().user_id,
        mine,
        "a saved profile wins (ADR-0038)"
    );
    let copies: Vec<_> = fs::read_dir(&theirs)
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().contains(".other-person-"))
        .collect();
    assert_eq!(copies.len(), 1);
    assert!(
        fs::read_to_string(copies[0].path())
            .unwrap()
            .contains(&their_id)
    );
}

// -------------------------------------------------------------------- move

#[test]
fn moving_the_library_takes_everything_and_every_new_file_goes_to_the_new_place() {
    let env = new_computer();
    env.ok("library.use_default", json!({}));
    let old = home_library(&env);
    // A project in the library, open, with a private note; a vault idea with a file.
    env.ok(
        "project.create",
        json!({ "title": "Salt Road", "projectType": "Feature Film" }),
    );
    let id = env.ok("project.current", json!({}))["id"]
        .as_str()
        .unwrap()
        .to_string();
    env.ok("private_note.create", json!({ "body": "REMEMBER-ME" }));
    add_idea_with_file(&env, b"first idea");
    // A project outside the library, and a custom folder for new projects.
    let outside = env.dir.path().join("elsewhere");
    fs::create_dir_all(&outside).unwrap();
    env.ok("project.close", json!({}));
    let far = env.ok(
        "project.create",
        json!({ "title": "Far Away", "projectType": "Short Film", "parentDir": text(&outside) }),
    );
    let far_path = PathBuf::from(far["project"]["path"].as_str().unwrap());
    env.ok("project.close", json!({}));
    let custom = env.dir.path().join("custom");
    fs::create_dir_all(&custom).unwrap();
    env.ok("storage.set_projects_dir", json!({ "path": text(&custom) }));
    let ours = env
        .ok("project.list_recent", json!({}))
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["projectId"] == id.as_str())
        .unwrap()["path"]
        .as_str()
        .unwrap()
        .to_string();
    env.reopen_project(&ours);

    let dest_parent = env.dir.path().join("drive-d");
    fs::create_dir_all(&dest_parent).unwrap();
    let started = env.ok("library.move", json!({ "destParent": text(&dest_parent) }));
    let done = finish(&env, &started).expect("the move finishes");
    let new = dest_parent.join("OpenFrame");
    assert!(same(&done["path"], &new));
    assert_eq!(done["projectsMoved"], 1);
    assert_eq!(done["vaultMoved"], true);
    assert!(done["sourceLeftAt"].is_null(), "{done}");
    assert!(!old.exists(), "the old library is gone");

    // Every record names the new place before the old copy went.
    assert_eq!(
        library_ops::read_pointer(&pointer(&env))
            .as_deref()
            .map(text),
        Some(text(&new))
    );
    assert!(
        fs::read_to_string(new.join("library.json"))
            .unwrap()
            .contains(&env.core.profile().user_id)
    );
    let info = env.ok("storage.info", json!({}));
    assert!(same(&info["libraryDir"], &new));
    assert!(
        same(&info["projectsDir"], &new.join("Projects")),
        "the default folder is the new library's: {info}"
    );
    assert_eq!(info["projectsDirIsDefault"], true);
    assert!(same(&info["vaultDir"], &new.join("Global Idea Vault")));
    // The open project follows, Recent Projects follow, the outside project stays.
    assert!(library::is_inside(&new, Path::new(&env.project_path())));
    assert!(
        env.ok("private_note.list", json!({}))
            .to_string()
            .contains("REMEMBER-ME")
    );
    let recents = env.ok("project.list_recent", json!({}));
    for p in recents.as_array().unwrap() {
        let path = PathBuf::from(p["path"].as_str().unwrap());
        assert!(!library::is_inside(&old, &path), "{path:?}");
    }
    assert!(recents.to_string().contains("Far Away"));
    assert!(
        far_path.join("openframe.json").is_file(),
        "the outside project did not move"
    );
    let first = env
        .ok("vault.list", json!({ "store": "global" }))
        .as_array()
        .unwrap()
        .iter()
        .find_map(|i| i["asset"]["path"].as_str().map(PathBuf::from))
        .expect("the first idea's file");
    assert!(library::is_inside(&new, &first), "{first:?}");
    assert_eq!(fs::read(&first).unwrap(), b"first idea");

    // New work goes only to the new place.
    env.ok("project.close", json!({}));
    let created = env.ok(
        "project.create",
        json!({ "title": "Next One", "projectType": "Short Film" }),
    );
    assert!(
        library::is_inside(&new.join("Projects"), Path::new(&env.project_path())),
        "{created}"
    );
    let file = add_idea_with_file(&env, b"second idea");
    assert!(
        library::is_inside(&new.join("Global Idea Vault"), &file),
        "{file:?}"
    );
    // A folder picked in New Project is still respected for that project only.
    env.ok("project.close", json!({}));
    env.ok(
        "project.create",
        json!({ "title": "Picked", "projectType": "Short Film", "parentDir": text(&outside) }),
    );
    assert!(library::is_inside(&outside, Path::new(&env.project_path())));
    assert!(!old.exists(), "nothing was written to the old place");

    // After a restart it opens at the new place without asking.
    let again = fresh(quit(env));
    let s = status(&again);
    assert_eq!(s["state"], "ready");
    assert!(same(&s["root"], &new));
}

#[test]
fn a_library_move_refuses_unsafe_places_and_a_failed_copy_keeps_the_source() {
    let env = new_computer();
    env.ok("library.use_default", json!({}));
    let old = home_library(&env);
    let move_to = |p: &Path| env.call("library.move", json!({ "destParent": text(p) }));
    assert_eq!(
        move_to(old.parent().unwrap()).unwrap_err().code_str(),
        "conflict.same_place"
    );
    assert!(move_to(&old.join("Projects")).is_err(), "not inside itself");
    let busy = env.dir.path().join("busy");
    fs::create_dir_all(busy.join("OpenFrame")).unwrap();
    fs::write(busy.join("OpenFrame").join("x.txt"), b"x").unwrap();
    assert_eq!(move_to(&busy).unwrap_err().code_str(), "conflict.exists");

    // A project whose database is damaged fails the check of the copy: nothing moves.
    env.ok(
        "project.create",
        json!({ "title": "Salt Road", "projectType": "Feature Film" }),
    );
    let good = PathBuf::from(env.project_path());
    env.ok("project.close", json!({}));
    let broken = old.join("Projects").join("Broken.openframe");
    fs::create_dir_all(&broken).unwrap();
    fs::copy(good.join("openframe.json"), broken.join("openframe.json")).unwrap();
    let db_name = openframe_project_format::ProjectLayout::new(&good).database();
    fs::write(broken.join(db_name.file_name().unwrap()), b"not a database").unwrap();
    let before = files_under(&old);
    let dest = env.dir.path().join("drive-d");
    fs::create_dir_all(&dest).unwrap();
    let started = env.ok("library.move", json!({ "destParent": text(&dest) }));
    let e = finish(&env, &started).expect_err("the copy check fails");
    assert!(!e.message.is_empty());
    assert_eq!(files_under(&old), before, "the source is untouched");
    assert_eq!(fs::read_dir(&dest).unwrap().count(), 0, "no copy is left");
    assert!(same(&status(&env)["root"], &old));
    assert_eq!(
        library_ops::read_pointer(&pointer(&env))
            .as_deref()
            .map(text),
        Some(text(&old))
    );
}
