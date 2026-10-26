//! Storage places and durable identity (doc 40, ADR-0038): the library file, the folder for new
//! projects, projects found on disk, private notes left by people who are gone, and the vault
//! that is opened on first use and never recreated empty.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use openframe_application::events::RecordingSink;
use openframe_application::modules::storage::library;
use openframe_application::{AppConfig, AppCore};
use openframe_domain::Role;
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

const OTHER_ID: &str = "0192f0a1-7b3c-7d4e-8f50-123456789abc";

fn root(env: &TestEnv) -> PathBuf {
    env.dir.path().to_path_buf()
}

fn library_text(root: &Path) -> String {
    fs::read_to_string(root.join("library.json")).expect("the library file exists")
}

/// A new core on the same root, as after a quit and a start. The old core must be gone first.
fn start_again(dir: tempfile::TempDir) -> TestEnv {
    let sink = Arc::new(RecordingSink::default());
    let core = AppCore::new(AppConfig::isolated(dir.path()), sink.clone()).expect("core");
    TestEnv { dir, core, sink }
}

fn quit(env: TestEnv) -> tempfile::TempDir {
    env.core.shutdown();
    let TestEnv { dir, core, .. } = env;
    drop(core);
    dir
}

fn delete_app_database(root: &Path) {
    for suffix in ["", "-wal", "-shm"] {
        let _ = fs::remove_file(root.join("appdata").join(format!("app.sqlite{suffix}")));
    }
}

fn paths_equal(a: &str, b: &Path) -> bool {
    library::same_place(Path::new(a), b)
}

// ------------------------------------------------------------------ identity

#[test]
fn identity_survives_deleting_the_app_data() {
    let env = TestEnv::with_project("Salt Road", "Feature Film");
    env.ok("app.set_display_name", json!({ "displayName": "Meera" }));
    let user = env.actor().user_id;
    env.ok("private_note.create", json!({ "body": "REMEMBER-ME" }));
    let project = env.project_path();
    let root = root(&env);
    // The library file carries the identity next to the projects, not inside the app data.
    let text = library_text(&root);
    assert!(text.contains(&user), "{text}");
    assert!(text.contains("Meera"), "{text}");

    let dir = quit(env);
    delete_app_database(dir.path());
    let again = start_again(dir);

    assert_eq!(
        again.core.profile().user_id,
        user,
        "same person after a reset"
    );
    assert_eq!(again.core.profile().display_name, "Meera");
    again.reopen_project(&project);
    let mine = again.ok("private_note.list", json!({}));
    assert!(
        mine.to_string().contains("REMEMBER-ME"),
        "the private note is still theirs: {mine}"
    );
}

#[test]
fn a_new_computer_gets_a_new_identity_and_a_library_file() {
    let a = TestEnv::new();
    let b = TestEnv::new();
    assert_ne!(a.core.profile().user_id, b.core.profile().user_id);
    for env in [&a, &b] {
        let text = library_text(env.dir.path());
        assert!(text.contains(&env.core.profile().user_id), "{text}");
    }
}

#[test]
fn an_app_database_with_a_profile_wins_over_the_library_file() {
    let env = TestEnv::new();
    let user = env.core.profile().user_id;
    let root = root(&env);
    library::write(
        &root.join("library.json"),
        OTHER_ID,
        "Someone Else",
        None,
        None,
    )
    .unwrap();
    let again = start_again(quit(env));
    assert_eq!(again.core.profile().user_id, user);
    // And the file is rewritten to say so.
    assert!(library_text(&root).contains(&user));
}

#[test]
fn an_invalid_library_file_is_ignored_never_fatal() {
    for bad in [
        "not json at all".to_string(),
        r#"{ "format": 1, "userId": "meera@example.com", "displayName": "Meera", "updatedAt": 1 }"#
            .to_string(),
        format!(
            r#"{{ "format": 9, "userId": "{OTHER_ID}", "displayName": "Meera", "updatedAt": 1 }}"#
        ),
        "x".repeat(200_000),
    ] {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("library.json"), &bad).unwrap();
        let env = start_again(dir);
        let user = env.core.profile().user_id;
        assert_ne!(user, OTHER_ID);
        assert!(uuid_like(&user), "a fresh, valid id: {user}");
        // The damaged file was replaced by a good one.
        let text = library_text(env.dir.path());
        assert!(text.contains(&user), "{text}");
    }
}

#[test]
fn a_bad_folder_in_the_library_file_keeps_the_identity_and_the_default() {
    let dir = tempfile::tempdir().unwrap();
    let text = format!(
        r#"{{ "format": 1, "userId": "{OTHER_ID}", "displayName": "Meera", "projectsDir": "relative/films", "updatedAt": 1 }}"#
    );
    fs::write(dir.path().join("library.json"), text).unwrap();
    let env = start_again(dir);
    assert_eq!(env.core.profile().user_id, OTHER_ID);
    let info = env.ok("storage.info", json!({}));
    assert_eq!(info["projectsDirIsDefault"], true, "{info}");
}

fn uuid_like(s: &str) -> bool {
    s.len() == 36 && s.matches('-').count() == 4
}

// ------------------------------------------------------------ projects folder

#[test]
fn storage_info_names_every_place() {
    let env = TestEnv::with_project("Info", "Short Film");
    let info = env.ok("storage.info", json!({}));
    let root = root(&env);
    assert!(paths_equal(
        info["appDataDir"].as_str().unwrap(),
        &root.join("appdata")
    ));
    assert!(paths_equal(info["libraryDir"].as_str().unwrap(), &root));
    assert!(paths_equal(
        info["libraryFile"].as_str().unwrap(),
        &root.join("library.json")
    ));
    assert!(paths_equal(
        info["projectsDir"].as_str().unwrap(),
        &root.join("projects")
    ));
    assert_eq!(info["projectsDirIsDefault"], true);
    assert!(paths_equal(
        info["vaultDir"].as_str().unwrap(),
        &root.join("global-vault")
    ));
    assert_eq!(info["vaultDirIsDefault"], true);
    assert_eq!(
        info["vaultExists"], false,
        "the vault is created on first use"
    );
    assert_eq!(info["projectDir"].as_str().unwrap(), env.project_path());
}

#[test]
fn the_projects_folder_is_validated_and_never_moves_projects() {
    let env = TestEnv::with_project("Stays Put", "Feature Film");
    let root = root(&env);
    let project = env.project_path();

    // Relative, missing, a file, inside app data, inside the open project, inside a project.
    assert_eq!(
        env.err(
            "storage.set_projects_dir",
            json!({ "path": "relative/films" })
        ),
        "validation.invalid_input"
    );
    assert_eq!(
        env.err(
            "storage.set_projects_dir",
            json!({ "path": root.join("nowhere").to_string_lossy() })
        ),
        "not_found.folder"
    );
    let file = root.join("a-file.txt");
    fs::write(&file, b"x").unwrap();
    assert_eq!(
        env.err(
            "storage.set_projects_dir",
            json!({ "path": file.to_string_lossy() })
        ),
        "not_found.folder"
    );
    let inside_app_data = root.join("appdata").join("sub");
    fs::create_dir_all(&inside_app_data).unwrap();
    assert_eq!(
        env.err(
            "storage.set_projects_dir",
            json!({ "path": inside_app_data.to_string_lossy() })
        ),
        "validation.invalid_input"
    );
    let inside_project = Path::new(&project).join("assets");
    assert_eq!(
        env.err(
            "storage.set_projects_dir",
            json!({ "path": inside_project.to_string_lossy() })
        ),
        "validation.invalid_input"
    );
    let closed_project = root.join("Old Film.openframe").join("notes");
    fs::create_dir_all(&closed_project).unwrap();
    assert_eq!(
        env.err(
            "storage.set_projects_dir",
            json!({ "path": closed_project.to_string_lossy() })
        ),
        "validation.invalid_input"
    );
    // Nothing changed.
    assert_eq!(
        env.ok("storage.info", json!({}))["projectsDirIsDefault"],
        true
    );

    // A good folder: applies at once, is remembered, and leaves the existing project alone.
    let films = root.join("Films");
    fs::create_dir_all(&films).unwrap();
    let info = env.ok(
        "storage.set_projects_dir",
        json!({ "path": films.to_string_lossy() }),
    );
    assert!(
        paths_equal(info["projectsDir"].as_str().unwrap(), &films),
        "{info}"
    );
    assert_eq!(info["projectsDirIsDefault"], false);
    assert!(
        Path::new(&project).join("openframe.json").is_file(),
        "the project did not move"
    );
    assert_eq!(env.project_path(), project);
    assert!(
        fs::read_dir(&films).unwrap().next().is_none(),
        "no write test file is left behind"
    );
    assert!(library_text(&root).contains("Films"));

    // New projects go there, without a restart.
    env.ok("project.close", json!({}));
    let created = env.ok(
        "project.create",
        json!({ "title": "New One", "projectType": "Short Film" }),
    );
    let path = created["project"]["path"].as_str().unwrap();
    assert!(Path::new(path).starts_with(&films), "{path}");

    // The choice outlives a restart, and choosing the default again forgets it.
    let again = start_again(quit(env));
    let info = again.ok("storage.info", json!({}));
    assert!(
        paths_equal(info["projectsDir"].as_str().unwrap(), &films),
        "{info}"
    );
    let default = root.join("projects");
    fs::create_dir_all(&default).unwrap();
    let info = again.ok(
        "storage.set_projects_dir",
        json!({ "path": default.to_string_lossy() }),
    );
    assert_eq!(info["projectsDirIsDefault"], true);
    assert!(!library_text(&root).contains("projectsDir"));
}

// ----------------------------------------------------------------- discovery

fn create_in(env: &TestEnv, title: &str, parent: &Path) -> Value {
    fs::create_dir_all(parent).unwrap();
    let created = env.ok(
        "project.create",
        json!({ "title": title, "projectType": "Short Film", "parentDir": parent.to_string_lossy() }),
    );
    env.ok("project.close", json!({}));
    created
}

#[test]
fn discover_lists_unlisted_projects_two_levels_down_and_skips_listed_ones() {
    let env = TestEnv::new();
    let projects = env.core.projects_dir();
    create_in(&env, "Listed", &projects);
    let unlisted = create_in(&env, "Unlisted", &projects);
    let nested = create_in(&env, "Nested", &projects.join("Group"));
    // Unlisted too, but three levels down.
    let deep = create_in(&env, "Too Deep", &projects.join("Group").join("Deeper"));
    for gone in [&unlisted, &nested, &deep] {
        env.ok(
            "project.remove_recent",
            json!({ "projectId": gone["project"]["id"] }),
        );
    }
    // A folder that is not a project, and an unreadable manifest, are skipped quietly.
    fs::create_dir_all(projects.join("Just A Folder")).unwrap();
    fs::create_dir_all(projects.join("Broken")).unwrap();
    fs::write(projects.join("Broken").join("openframe.json"), b"{ nope").unwrap();

    let found = env.ok("project.discover", json!({}));
    let found = found.as_array().unwrap();
    let titles: Vec<&str> = found.iter().map(|p| p["title"].as_str().unwrap()).collect();
    assert!(titles.contains(&"Unlisted"), "{titles:?}");
    assert!(titles.contains(&"Nested"), "{titles:?}");
    assert!(!titles.contains(&"Listed"), "{titles:?}");
    assert!(!titles.contains(&"Too Deep"), "{titles:?}");
    assert_eq!(found.len(), 2, "{titles:?}");
    let entry = found.iter().find(|p| p["title"] == "Unlisted").unwrap();
    assert_eq!(entry["projectId"], unlisted["project"]["id"]);
    assert!(
        Path::new(entry["path"].as_str().unwrap())
            .join("openframe.json")
            .is_file()
    );
    assert!(entry["modifiedAt"].is_number());

    // Opening one lists it again, so it is no longer "found".
    env.ok("project.open", json!({ "path": entry["path"] }));
    let after = env.ok("project.discover", json!({}));
    assert_eq!(after.as_array().unwrap().len(), 1, "{after}");
}

#[test]
fn discover_on_a_missing_projects_folder_is_empty() {
    let env = TestEnv::new();
    assert!(!env.core.projects_dir().exists());
    assert_eq!(env.ok("project.discover", json!({})), json!([]));
}

// ------------------------------------------------------------- orphan notes

/// A person on the project (a row in `project_member`), as a collaborator who joined by package.
fn add_member(env: &TestEnv, user_id: &str) {
    let session = env.core.project().unwrap();
    session
        .store
        .with_writer(|c| {
            c.execute(
                "INSERT INTO project_member(id, user_id, display_name, role, created_at, updated_at)
                 VALUES (?1, ?2, 'Member', 'Editor', 1, 1)",
                rusqlite::params![openframe_domain::new_id(), user_id],
            )?;
            Ok(())
        })
        .unwrap();
}

fn scalar(env: &TestEnv, sql: &str) -> i64 {
    env.core
        .project()
        .unwrap()
        .store
        .read(|c| Ok(c.query_row(sql, [], |r| r.get::<_, i64>(0))?))
        .unwrap()
}

#[test]
fn orphan_notes_are_listed_and_only_the_owner_can_take_them_over() {
    let env = TestEnv::with_project("Ownership", "Feature Film");
    let gone = env.other_user(Role::Editor);
    let member = env.other_user(Role::Editor);
    add_member(&env, &member.user_id);
    env.ok("private_note.create", json!({ "body": "mine" }));
    for body in ["gone one ZEBRAQUARTZ", "gone two"] {
        env.call_as(&gone, "private_note.create", json!({ "body": body }))
            .unwrap();
    }
    env.call_as(
        &member,
        "private_note.create",
        json!({ "body": "member note" }),
    )
    .unwrap();

    // Only people who are neither me nor on the project are listed.
    let listed = env.ok("project.orphan_notes", json!({}));
    assert_eq!(
        listed,
        json!([{ "ownerUserId": gone.user_id, "noteCount": 2 }])
    );

    // Not the Owner: refused, for the list and for the take-over.
    let editor = env.actor_with_role(Role::Editor);
    for (op, args) in [
        ("project.orphan_notes", json!({})),
        (
            "project.adopt_orphan_notes",
            json!({ "ownerUserId": gone.user_id }),
        ),
    ] {
        assert_eq!(
            env.call_as(&editor, op, args).unwrap_err().code.0,
            "permission.denied"
        );
    }
    // An active member's notes are never taken, and neither are mine.
    assert_eq!(
        env.err(
            "project.adopt_orphan_notes",
            json!({ "ownerUserId": member.user_id })
        ),
        "conflict.state"
    );
    assert_eq!(
        env.err(
            "project.adopt_orphan_notes",
            json!({ "ownerUserId": env.actor().user_id })
        ),
        "validation.invalid_input"
    );

    // The Owner takes one person's notes: they are mine now, in the table, in search, in my list.
    let done = env.ok(
        "project.adopt_orphan_notes",
        json!({ "ownerUserId": gone.user_id }),
    );
    assert_eq!(done, json!({ "adopted": 2 }));
    assert_eq!(env.ok("project.orphan_notes", json!({})), json!([]));
    let me = env.actor().user_id;
    let sql = format!("SELECT count(*) FROM private_note WHERE owner_user_id='{me}'");
    assert_eq!(scalar(&env, &sql), 3);
    let mine = env.ok("private_note.list", json!({}));
    assert!(mine.to_string().contains("gone one ZEBRAQUARTZ"), "{mine}");
    let hits = env.ok("search.query", json!({ "text": "ZEBRAQUARTZ" }));
    assert!(hits.to_string().contains("private_note"), "{hits}");
    // The member's note stayed with the member.
    let sql = format!(
        "SELECT count(*) FROM private_note WHERE owner_user_id='{}'",
        member.user_id
    );
    assert_eq!(scalar(&env, &sql), 1);
    // The take-over is on record without saying what the notes hold, and is not an undo step.
    assert_eq!(
        scalar(
            &env,
            "SELECT count(*) FROM sys_activity WHERE action='project.adopt_orphan_notes'"
        ),
        1
    );
    assert_eq!(
        scalar(
            &env,
            "SELECT count(*) FROM sys_undo WHERE label LIKE 'Took over%'"
        ),
        0
    );
    // Nothing is left to take.
    assert_eq!(
        env.err(
            "project.adopt_orphan_notes",
            json!({ "ownerUserId": gone.user_id })
        ),
        "not_found.private_notes"
    );
}

// ---------------------------------------------------------------- lazy vault

#[test]
fn the_default_vault_is_created_on_first_use_not_at_start() {
    let env = TestEnv::new();
    env.core.startup_maintenance();
    let vault = root(&env).join("global-vault");
    assert!(
        !vault.join("vault.sqlite").exists(),
        "start does not create a vault"
    );
    env.core
        .global_store()
        .expect("the default vault is created on first use");
    assert!(vault.join("vault.sqlite").is_file());
    assert!(vault.join("assets").is_dir());
    // From now on start maintenance opens it (and its purge runs).
    env.core.close_global_store();
    env.core.startup_maintenance();
    env.core.global_store().unwrap();
}

#[test]
fn a_missing_chosen_vault_is_never_recreated_empty() {
    let dir = tempfile::tempdir().unwrap();
    let elsewhere = dir.path().join("drive-d").join("Vault");
    library::write(
        &dir.path().join("library.json"),
        OTHER_ID,
        "Meera",
        None,
        Some(&elsewhere),
    )
    .unwrap();
    let env = start_again(dir);
    assert!(paths_equal(
        env.core.vault_dir().to_str().unwrap(),
        &elsewhere
    ));
    env.core.startup_maintenance();
    let err = env.core.global_store().err().expect("refused");
    assert_eq!(err.code.0, "storage.vault_missing");
    assert!(
        err.message.contains("Settings > Storage"),
        "{}",
        err.message
    );
    assert!(!elsewhere.exists(), "nothing was created");
    assert!(!elsewhere.parent().unwrap().exists());
    let info = env.ok("storage.info", json!({}));
    assert_eq!(info["vaultDirIsDefault"], false);
    assert_eq!(info["vaultExists"], false);
    // The vault operations fail the same way rather than starting a second, empty vault.
    assert_eq!(
        env.err("vault.list", json!({ "store": "global" })).as_str(),
        "storage.vault_missing"
    );

    // Once a vault is there (the drive is back), it opens.
    let back = TestEnv::new();
    back.core.global_store().unwrap();
    fs::create_dir_all(&elsewhere).unwrap();
    back.core.close_global_store();
    fs::copy(
        back.dir.path().join("global-vault").join("vault.sqlite"),
        elsewhere.join("vault.sqlite"),
    )
    .unwrap();
    env.core
        .global_store()
        .expect("opens once the vault is there");
}

// ------------------------------------------------------- interrupted work

#[test]
fn staging_folders_from_an_interrupted_move_are_removed_at_start() {
    let env = TestEnv::new();
    let root = root(&env);
    let projects = env.core.projects_dir();
    fs::create_dir_all(&projects).unwrap();
    let stale_vault = root.join("global-vault.openframe-staging-1793000000000");
    let stale_project = projects.join("Film.openframe.openframe-staging-42");
    let not_ours = root.join("global-vault.openframe-staging-later");
    let ordinary = projects.join("Film.openframe");
    for d in [&stale_vault, &stale_project, &not_ours, &ordinary] {
        fs::create_dir_all(d.join("assets")).unwrap();
        fs::write(d.join("assets").join("x.bin"), b"x").unwrap();
    }
    env.core.startup_maintenance();
    assert!(!stale_vault.exists());
    assert!(!stale_project.exists());
    assert!(not_ours.exists(), "only names ending in digits are ours");
    assert!(ordinary.exists());
}

// ====================================================== vault files and moves

use base64::Engine;
use openframe_application::events::AppEvent;
use openframe_domain::AppError;
use std::io::{Read, Write};
use std::time::{Duration, Instant};

/// Wait for a background task; its result, or the error it ended with.
fn finish(env: &TestEnv, started: &Value) -> Result<Value, AppError> {
    let id = started["taskId"].as_str().expect("a task id").to_string();
    let begun = Instant::now();
    loop {
        {
            let events = env.sink.events.lock();
            for e in events.iter().rev() {
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
        }
        assert!(
            begun.elapsed() < Duration::from_secs(120),
            "the task did not finish"
        );
        std::thread::sleep(Duration::from_millis(15));
    }
}

fn add_note(env: &TestEnv, body: &str) -> Value {
    env.ok(
        "vault.create",
        json!({ "store": "global", "itemType": "note", "body": body }),
    )
}

fn add_voice(env: &TestEnv, bytes: &[u8]) -> Value {
    env.ok(
        "vault.ingest_bytes",
        json!({
            "store": "global", "itemType": "voice", "fileName": "voice.webm",
            "mediaType": "audio/webm",
            "dataBase64": base64::engine::general_purpose::STANDARD.encode(bytes)
        }),
    )
}

fn global_items(env: &TestEnv) -> Vec<Value> {
    env.ok("vault.list", json!({ "store": "global" }))
        .as_array()
        .unwrap()
        .clone()
}

fn bodies(env: &TestEnv) -> Vec<String> {
    let mut all: Vec<String> = global_items(env)
        .iter()
        .filter_map(|i| i["body"].as_str().map(str::to_string))
        .collect();
    all.sort();
    all
}

/// The bytes of the one voice item's file, read from wherever the app says it is.
fn voice_bytes(env: &TestEnv) -> Vec<u8> {
    let items = global_items(env);
    let voice = items
        .iter()
        .find(|i| i["itemType"] == "voice")
        .expect("a voice item");
    fs::read(voice["asset"]["path"].as_str().unwrap()).expect("the file is readable")
}

fn out_dir(env: &TestEnv) -> PathBuf {
    let dir = root(env).join("out");
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn text(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

/// Every file and folder under `dir`, relative and sorted: what a change would show up in.
fn listing(dir: &Path) -> Vec<String> {
    fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) {
        for e in fs::read_dir(dir).unwrap().flatten() {
            out.push(
                e.path()
                    .strip_prefix(base)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
            );
            if e.file_type().unwrap().is_dir() {
                walk(&e.path(), base, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for e in fs::read_dir(from).unwrap().flatten() {
        let target = to.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_tree(&e.path(), &target);
        } else {
            fs::copy(e.path(), target).unwrap();
        }
    }
}

/// A copy of the zip at `src` with every entry passed through `edit` (name, bytes), plus extra
/// entries. `checksums.json` is left as it was unless `edit` changes it.
fn rewrite_zip(
    src: &str,
    dst: &Path,
    edit: impl Fn(&str, Vec<u8>) -> Vec<u8>,
    extra: &[(&str, &[u8])],
) -> String {
    let mut input = zip::ZipArchive::new(fs::File::open(src).unwrap()).unwrap();
    let mut output = zip::ZipWriter::new(fs::File::create(dst).unwrap());
    let options = zip::write::SimpleFileOptions::default();
    for i in 0..input.len() {
        let mut entry = input.by_index(i).unwrap();
        let name = entry.name().unwrap().to_string();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        output.start_file(&name, options).unwrap();
        output.write_all(&edit(&name, bytes)).unwrap();
    }
    for (name, bytes) in extra {
        output.start_file(*name, options).unwrap();
        output.write_all(bytes).unwrap();
    }
    output.finish().unwrap();
    text(dst)
}

/// Back up the global vault into the out folder and return the file.
fn back_up(env: &TestEnv, name: &str) -> (String, Value) {
    let started = env.ok(
        "vault.backup",
        json!({ "path": text(&out_dir(env).join(name)) }),
    );
    let report = finish(env, &started).expect("the backup finishes");
    (report["path"].as_str().unwrap().to_string(), report)
}

fn assert_no_leftovers(dir: &Path) {
    let found: Vec<String> = listing(dir)
        .into_iter()
        .filter(|n| n.contains(".openframe-staging-") || n.contains("(before restore"))
        .collect();
    assert!(found.is_empty(), "left behind: {found:?}");
}

// ------------------------------------------------------- backup and restore

#[test]
fn a_backup_restores_ideas_and_files_and_keeps_the_vault_it_replaced() {
    let env = TestEnv::new();
    add_note(&env, "ALPHA harbour");
    add_voice(&env, b"voice-bytes");
    let (file, report) = back_up(&env, "vault backup");
    assert!(file.ends_with("vault backup.ofvault"), "{file}");
    assert_eq!(report["kind"], "backup");
    assert_eq!(report["ideaCount"], 2);
    assert_eq!(report["assetCount"], 1);

    let info = env.ok("vault.inspect", json!({ "path": file }));
    assert_eq!(info["kind"], "backup");
    assert_eq!(info["ideaCount"], 2);
    assert_eq!(info["assetCount"], 1);
    assert!(info["bytes"].as_i64().unwrap() > 0);

    add_note(&env, "BETA written after the backup");
    let vault = env.core.vault_dir();
    let started = env.ok("vault.restore_backup", json!({ "path": file }));
    let done = finish(&env, &started).expect("the restore finishes");

    // The vault is the backed up one, files included, and it is the open one.
    assert_eq!(bodies(&env), vec!["ALPHA harbour".to_string()]);
    assert_eq!(voice_bytes(&env), b"voice-bytes");
    // The one it replaced is kept beside it, complete, with the later idea.
    let kept = PathBuf::from(
        done["previousKeptAt"]
            .as_str()
            .expect("the old vault is kept"),
    );
    assert_eq!(kept.parent(), vault.parent());
    assert!(
        kept.file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("global-vault (before restore "),
        "{kept:?}"
    );
    assert!(kept.join("vault.sqlite").is_file());
    let kept_db = fs::read(kept.join("vault.sqlite")).unwrap();
    assert!(
        kept_db.windows(4).any(|w| w == b"BETA"),
        "the kept vault still holds the later idea"
    );
    // A second restore keeps a second copy under its own name.
    let again = finish(
        &env,
        &env.ok("vault.restore_backup", json!({ "path": file })),
    )
    .unwrap();
    assert_ne!(again["previousKeptAt"], done["previousKeptAt"]);
}

#[test]
fn restore_installs_a_vault_that_does_not_exist_yet() {
    let source = TestEnv::new();
    add_note(&source, "FROM-ELSEWHERE");
    let (file, _) = back_up(&source, "moving house");
    let fresh = TestEnv::new();
    assert!(!fresh.core.vault_dir().join("vault.sqlite").exists());
    let done = finish(
        &fresh,
        &fresh.ok("vault.restore_backup", json!({ "path": file })),
    )
    .unwrap();
    assert!(done["previousKeptAt"].is_null());
    assert_eq!(bodies(&fresh), vec!["FROM-ELSEWHERE".to_string()]);
    assert_no_leftovers(&root(&fresh));
}

#[test]
fn a_package_leaves_out_recently_deleted_items_and_their_files() {
    let env = TestEnv::new();
    add_note(&env, "KEEP-ME");
    let gone = add_note(&env, "SECRET-DELETED-TEXT");
    let gone_voice = add_voice(&env, b"secret-voice");
    env.ok(
        "vault.delete",
        json!({ "store": "global", "ids": [gone["id"], gone_voice["id"]] }),
    );
    let started = env.ok(
        "vault.export_package",
        json!({ "path": text(&out_dir(&env).join("for the laptop")) }),
    );
    let report = finish(&env, &started).unwrap();
    assert_eq!(report["kind"], "package");
    assert_eq!(report["ideaCount"], 1);
    assert_eq!(
        report["assetCount"], 0,
        "the deleted idea's file is not carried"
    );
    let package = report["path"].as_str().unwrap().to_string();

    let other = TestEnv::new();
    finish(
        &other,
        &other.ok("vault.restore_backup", json!({ "path": package })),
    )
    .unwrap();
    assert_eq!(bodies(&other), vec!["KEEP-ME".to_string()]);
    assert!(
        other
            .ok("vault.trash_list", json!({ "store": "global" }))
            .as_array()
            .unwrap()
            .is_empty()
    );
    other.core.close_global_store();
    let db = fs::read(other.core.vault_dir().join("vault.sqlite")).unwrap();
    for needle in [&b"SECRET-DELETED-TEXT"[..], b"secret"] {
        assert!(
            !db.windows(needle.len()).any(|w| w == needle),
            "the package database still holds {:?}",
            String::from_utf8_lossy(needle)
        );
    }
    let files: Vec<String> = listing(&other.core.vault_dir().join("assets"))
        .into_iter()
        .filter(|n| n.contains('.'))
        .collect();
    assert!(files.is_empty(), "no file came along: {files:?}");

    // A backup, by contrast, is the whole vault: Recently Deleted comes back with it.
    let (backup, report) = back_up(&env, "everything");
    assert_eq!(
        report["assetCount"], 1,
        "the deleted idea's file is in a backup"
    );
    let third = TestEnv::new();
    finish(
        &third,
        &third.ok("vault.restore_backup", json!({ "path": backup })),
    )
    .unwrap();
    assert_eq!(
        third
            .ok("vault.trash_list", json!({ "store": "global" }))
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn a_package_does_not_carry_the_local_path_of_a_linked_file() {
    let env = TestEnv::new();
    let linked = env.write_file(
        "private-folder-name/linked-photo.png",
        &[0x89, b'P', b'N', b'G'],
    );
    env.ok(
        "vault.add_files",
        json!({ "store": "global", "paths": [linked.clone()], "mode": "link" }),
    );
    let started = env.ok(
        "vault.export_package",
        json!({ "path": text(&out_dir(&env).join("portable")) }),
    );
    let package = finish(&env, &started).unwrap()["path"]
        .as_str()
        .unwrap()
        .to_string();
    let other = TestEnv::new();
    finish(
        &other,
        &other.ok("vault.restore_backup", json!({ "path": package })),
    )
    .unwrap();
    other.core.close_global_store();
    let db = fs::read(other.core.vault_dir().join("vault.sqlite")).unwrap();
    let needle = b"private-folder-name";
    assert!(
        !db.windows(needle.len()).any(|w| w == needle),
        "the package database still names the folder of the linked file"
    );
    // A backup stays on this computer and keeps the link as it was.
    let (backup, _) = back_up(&env, "with links");
    let third = TestEnv::new();
    finish(
        &third,
        &third.ok("vault.restore_backup", json!({ "path": backup })),
    )
    .unwrap();
    third.core.close_global_store();
    let db = fs::read(third.core.vault_dir().join("vault.sqlite")).unwrap();
    assert!(db.windows(needle.len()).any(|w| w == needle));
}

#[test]
fn restore_refuses_a_damaged_or_unsafe_file_and_leaves_the_vault_alone() {
    let env = TestEnv::new();
    add_note(&env, "ALPHA");
    let (file, _) = back_up(&env, "good");
    let bad = out_dir(&env).join("bad.ofvault");
    let sha = |b: &[u8]| openframe_security::sha256_bytes(b);

    // One byte changed inside the database: the hash no longer matches.
    let tampered = rewrite_zip(
        &file,
        &bad,
        |name, mut bytes| {
            if name == "vault.sqlite" {
                bytes.push(0);
            }
            bytes
        },
        &[],
    );
    let started = env.ok("vault.restore_backup", json!({ "path": tampered }));
    let e = finish(&env, &started).expect_err("a changed file is refused");
    assert_eq!(e.code.0, "import.vault_checksum_mismatch", "{}", e.message);

    // A database that is not one, with every hash made to match: the data checks still refuse it.
    let hollow = rewrite_zip(
        &file,
        &bad,
        |name, bytes| match name {
            "vault.sqlite" => b"this is not a database".to_vec(),
            "checksums.json" => {
                let mut sums: Value = serde_json::from_slice(&bytes).unwrap();
                sums["files"]["vault.sqlite"] = json!(sha(b"this is not a database"));
                serde_json::to_vec(&sums).unwrap()
            }
            _ => bytes,
        },
        &[],
    );
    let started = env.ok("vault.restore_backup", json!({ "path": hollow }));
    let e = finish(&env, &started).expect_err("not a vault");
    assert_eq!(e.code.0, "storage.not_a_vault", "{}", e.message);

    // A path that climbs out of the folder.
    let slip = rewrite_zip(&file, &bad, |_, b| b, &[("../escape.txt", b"x")]);
    assert_eq!(
        env.err("vault.restore_backup", json!({ "path": slip })),
        "import.unsafe_archive"
    );
    assert!(!root(&env).join("escape.txt").exists());
    assert!(!root(&env).parent().unwrap().join("escape.txt").exists());

    // A write-ahead log that SQLite would replay on top of the database.
    let wal = rewrite_zip(&file, &bad, |_, b| b, &[("vault.sqlite-wal", b"x")]);
    assert_eq!(
        env.err("vault.restore_backup", json!({ "path": wal })),
        "import.vault_incomplete"
    );

    // Made by a newer OpenFrame.
    let newer = rewrite_zip(
        &file,
        &bad,
        |name, bytes| {
            if name == "manifest.json" {
                let mut m: Value = serde_json::from_slice(&bytes).unwrap();
                m["schemaVersion"] = json!(999);
                serde_json::to_vec(&m).unwrap()
            } else {
                bytes
            }
        },
        &[],
    );
    assert_eq!(
        env.err("vault.restore_backup", json!({ "path": newer })),
        "import.vault_too_new"
    );
    assert_eq!(
        env.err("vault.inspect", json!({ "path": newer })),
        "import.vault_too_new"
    );

    // Not a vault file at all.
    let plain = env.write_file("notes.txt", b"just words");
    assert!(
        env.call("vault.restore_backup", json!({ "path": plain }))
            .is_err()
    );

    // Through all of it the vault is exactly as it was, and nothing was left beside it.
    assert_eq!(bodies(&env), vec!["ALPHA".to_string()]);
    assert_no_leftovers(&root(&env));
}

#[test]
fn backup_needs_a_vault_and_a_good_destination() {
    let env = TestEnv::new();
    let out = text(&out_dir(&env).join("x"));
    assert_eq!(
        env.err("vault.backup", json!({ "path": out })),
        "not_found.vault"
    );
    assert_eq!(
        env.err("vault.export_package", json!({ "path": out })),
        "not_found.vault"
    );
    add_note(&env, "ALPHA");
    // Not inside the vault itself, and not a relative name.
    let inside = text(&env.core.vault_dir().join("x"));
    assert!(env.call("vault.backup", json!({ "path": inside })).is_err());
    assert!(
        env.call("vault.backup", json!({ "path": "relative.ofvault" }))
            .is_err()
    );
}

// ---------------------------------------------------------------- vault move

#[test]
fn moving_the_vault_switches_the_app_and_removes_the_old_folder() {
    let env = TestEnv::new();
    add_note(&env, "ALPHA");
    add_voice(&env, b"voice-bytes");
    let old = env.core.vault_dir();
    let drive = root(&env).join("drive-d");
    // An empty folder of the same name is replaced, not refused.
    fs::create_dir_all(drive.join("global-vault")).unwrap();

    let started = env.ok("vault.move", json!({ "destParent": text(&drive) }));
    let done = finish(&env, &started).expect("the move finishes");

    let new = drive.join("global-vault");
    assert!(paths_equal(done["path"].as_str().unwrap(), &new));
    assert!(done["sourceLeftAt"].is_null());
    assert!(paths_equal(env.core.vault_dir().to_str().unwrap(), &new));
    assert!(!old.exists(), "the old folder is gone");
    assert!(new.join("vault.sqlite").is_file());
    assert!(library_text(&root(&env)).contains("drive-d"));
    assert_eq!(bodies(&env), vec!["ALPHA".to_string()]);
    assert_eq!(voice_bytes(&env), b"voice-bytes");
    assert_no_leftovers(&root(&env));
    assert_no_leftovers(&drive);
    let info = env.ok("storage.info", json!({}));
    assert_eq!(info["vaultDirIsDefault"], false);
    assert_eq!(info["vaultExists"], true);

    // It is still there after a restart.
    let again = start_again(quit(env));
    assert!(paths_equal(again.core.vault_dir().to_str().unwrap(), &new));
    assert_eq!(bodies(&again), vec!["ALPHA".to_string()]);

    // And it can go back to the default place.
    let home = root(&again);
    let started = again.ok("vault.move", json!({ "destParent": text(&home) }));
    finish(&again, &started).unwrap();
    assert!(paths_equal(
        again.core.vault_dir().to_str().unwrap(),
        &home.join("global-vault")
    ));
    assert_eq!(
        again.ok("storage.info", json!({}))["vaultDirIsDefault"],
        true
    );
    assert_eq!(bodies(&again), vec!["ALPHA".to_string()]);
}

#[test]
fn a_vault_move_refuses_places_that_would_lose_or_mix_data() {
    let env = TestEnv::new();
    let drive = root(&env).join("drive-d");
    fs::create_dir_all(&drive).unwrap();
    assert_eq!(
        env.err("vault.move", json!({ "destParent": text(&drive) })),
        "not_found.vault",
        "there is nothing to move before the first idea"
    );
    add_note(&env, "ALPHA");
    let vault = env.core.vault_dir();

    // A destination that already holds something.
    fs::create_dir_all(drive.join("global-vault")).unwrap();
    fs::write(drive.join("global-vault").join("mine.txt"), b"mine").unwrap();
    assert_eq!(
        env.err("vault.move", json!({ "destParent": text(&drive) })),
        "conflict.exists"
    );
    assert_eq!(
        fs::read(drive.join("global-vault").join("mine.txt")).unwrap(),
        b"mine"
    );

    // The same place, inside the vault, a folder that is not there.
    assert_eq!(
        env.err(
            "vault.move",
            json!({ "destParent": text(vault.parent().unwrap()) })
        ),
        "conflict.same_place"
    );
    assert!(
        env.call(
            "vault.move",
            json!({ "destParent": text(&vault.join("assets")) })
        )
        .is_err()
    );
    assert_eq!(
        env.err(
            "vault.move",
            json!({ "destParent": text(&root(&env).join("nowhere")) })
        ),
        "not_found.folder"
    );
    assert!(
        env.call("vault.move", json!({ "destParent": "relative" }))
            .is_err()
    );

    // Nothing moved.
    assert!(paths_equal(env.core.vault_dir().to_str().unwrap(), &vault));
    assert_eq!(bodies(&env), vec!["ALPHA".to_string()]);
    assert_no_leftovers(&root(&env));
}

/// A file another program holds open makes the copy fail half way. The vault stays where it was,
/// complete, and nothing is left beside it.
#[cfg(windows)]
#[test]
fn a_vault_move_that_fails_leaves_the_vault_where_it_was() {
    use std::os::windows::fs::OpenOptionsExt;
    let env = TestEnv::new();
    add_note(&env, "ALPHA");
    add_voice(&env, b"voice-bytes");
    let items = global_items(&env);
    let voice = items.iter().find(|i| i["itemType"] == "voice").unwrap();
    let file = voice["asset"]["path"].as_str().unwrap().to_string();
    let vault = env.core.vault_dir();
    let drive = root(&env).join("drive-d");
    fs::create_dir_all(&drive).unwrap();

    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&file)
        .unwrap();
    let started = env.ok("vault.move", json!({ "destParent": text(&drive) }));
    let failed = finish(&env, &started).expect_err("the copy cannot read a held file");
    drop(held);

    assert!(!failed.message.is_empty());
    assert!(paths_equal(env.core.vault_dir().to_str().unwrap(), &vault));
    assert!(vault.join("vault.sqlite").is_file());
    assert_eq!(bodies(&env), vec!["ALPHA".to_string()]);
    assert_eq!(voice_bytes(&env), b"voice-bytes");
    assert!(
        listing(&drive).is_empty(),
        "no staging folder or half copy: {:?}",
        listing(&drive)
    );
    assert!(!library_text(&root(&env)).contains("drive-d"));
}

// -------------------------------------------------------------- vault locate

#[test]
fn locating_a_vault_folder_connects_it_without_moving_anything() {
    let a = TestEnv::new();
    add_note(&a, "ALPHA");
    add_voice(&a, b"voice-bytes");
    a.core.close_global_store();
    let usb = a.dir.path().join("usb").join("My Vault");
    copy_tree(&a.core.vault_dir(), &usb);

    let b = TestEnv::new();
    let before = listing(&usb);
    let info = b.ok("vault.locate", json!({ "dir": text(&usb) }));
    assert!(paths_equal(info["vaultDir"].as_str().unwrap(), &usb));
    assert_eq!(info["vaultExists"], true);
    assert!(library_text(b.dir.path()).contains("My Vault"));
    assert_eq!(bodies(&b), vec!["ALPHA".to_string()]);
    assert_eq!(voice_bytes(&b), b"voice-bytes");
    b.core.close_global_store();
    assert_eq!(listing(&usb), before, "nothing was moved, added or removed");
    assert!(
        !b.core.config.global_vault_dir.exists(),
        "no empty vault was made"
    );

    // Locating the place it already is in changes nothing.
    b.ok("vault.locate", json!({ "dir": text(&usb) }));
    assert_eq!(bodies(&b), vec!["ALPHA".to_string()]);
}

#[test]
fn locate_refuses_what_is_not_a_usable_vault() {
    let env = TestEnv::new();
    let base = env.dir.path().join("candidates");
    let refuse = |dir: &Path| env.err("vault.locate", json!({ "dir": text(dir) }));

    let empty = base.join("empty");
    fs::create_dir_all(&empty).unwrap();
    assert_eq!(refuse(&empty), "not_found.vault");
    assert_eq!(refuse(&base.join("missing")), "not_found.folder");

    // A project folder is not a vault.
    let created = create_in(&env, "Salt Road", &base.join("projects"));
    let project = PathBuf::from(created["project"]["path"].as_str().unwrap());
    assert_eq!(refuse(&project), "validation.invalid_input");

    let garbage = base.join("garbage");
    fs::create_dir_all(&garbage).unwrap();
    fs::write(garbage.join("vault.sqlite"), b"not a database").unwrap();
    assert_eq!(refuse(&garbage), "storage.not_a_vault");

    // A real database that is not a vault.
    let other = base.join("other");
    fs::create_dir_all(&other).unwrap();
    rusqlite::Connection::open(other.join("vault.sqlite"))
        .unwrap()
        .execute_batch("CREATE TABLE notes(x); INSERT INTO notes VALUES (1);")
        .unwrap();
    assert_eq!(refuse(&other), "storage.not_a_vault");

    // A vault saved by a newer OpenFrame.
    let maker = TestEnv::new();
    add_note(&maker, "ALPHA");
    maker.core.close_global_store();
    let newer = base.join("newer");
    copy_tree(&maker.core.vault_dir(), &newer);
    rusqlite::Connection::open(newer.join("vault.sqlite"))
        .unwrap()
        .execute_batch("PRAGMA journal_mode=DELETE; PRAGMA user_version = 999;")
        .unwrap();
    assert_eq!(refuse(&newer), "storage.vault_too_new");

    // The app still points where it did.
    assert!(paths_equal(
        env.core.vault_dir().to_str().unwrap(),
        &env.core.config.global_vault_dir
    ));
    assert!(!library_text(env.dir.path()).contains("candidates"));
}

// -------------------------------------------------------------- project move

fn recent_path(env: &TestEnv, id: &str) -> String {
    env.ok("project.list_recent", json!({}))
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["projectId"] == id)
        .expect("listed")["path"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn moving_the_open_project_keeps_everything_and_follows_it() {
    let env = TestEnv::with_project("Salt Road", "Feature Film");
    env.ok("app.set_display_name", json!({ "displayName": "Meera" }));
    let id = env.ok("project.current", json!({}))["id"]
        .as_str()
        .unwrap()
        .to_string();
    env.ok("private_note.create", json!({ "body": "REMEMBER-ME" }));
    env.ok("notes.create", json!({ "body": "A shared note" }));
    let kept = env.write_file("kept.txt", b"copied into the project");
    let linked = env.write_file("linked.txt", b"stays where it is");
    env.ok("files.add", json!({ "paths": [kept], "mode": "copy" }));
    env.ok(
        "files.add",
        json!({ "paths": [linked.clone()], "mode": "link" }),
    );
    let old = PathBuf::from(env.project_path());
    let name = old.file_name().unwrap().to_owned();
    let elsewhere = root(&env).join("elsewhere");
    fs::create_dir_all(&elsewhere).unwrap();

    let started = env.ok("project.move", json!({ "destParent": text(&elsewhere) }));
    let done = finish(&env, &started).expect("the move finishes");

    let new = elsewhere.join(&name);
    assert!(paths_equal(done["path"].as_str().unwrap(), &new));
    assert_eq!(done["linkedFiles"], 1, "the linked file is reported");
    assert!(done["sourceLeftAt"].is_null());
    assert!(!old.exists(), "the old folder is gone");
    assert!(new.join("openframe.json").is_file());
    // The same project is open, from the new place.
    assert!(paths_equal(&env.project_path(), &new));
    assert_eq!(env.ok("project.current", json!({}))["id"], id);
    assert!(paths_equal(&recent_path(&env, &id), &new));
    // Content, private notes and files are all there.
    assert!(
        env.ok("private_note.list", json!({}))
            .to_string()
            .contains("REMEMBER-ME")
    );
    assert!(
        env.ok("notes.list", json!({}))
            .to_string()
            .contains("A shared note")
    );
    let files = env.ok("files.list", json!({}));
    let managed = files["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["asset"]["storageMode"] == "managed")
        .expect("the copied file");
    assert_eq!(
        fs::read(managed["asset"]["path"].as_str().unwrap()).unwrap(),
        b"copied into the project"
    );
    // The linked file was not touched and still points at its own place.
    let still: String = env
        .core
        .project()
        .unwrap()
        .store
        .read(|c| {
            Ok(c.query_row(
                "SELECT external_path FROM asset WHERE storage_mode = 'external'",
                [],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    assert_eq!(
        fs::canonicalize(&still).unwrap(),
        fs::canonicalize(&linked).unwrap(),
        "the link still names the original file"
    );
    assert_eq!(fs::read(&linked).unwrap(), b"stays where it is");
    assert_no_leftovers(&elsewhere);
    assert_no_leftovers(&env.core.projects_dir());

    // After a restart it opens from where it now is.
    let again = start_again(quit(env));
    again.reopen_project(&text(&new));
    assert!(
        again
            .ok("private_note.list", json!({}))
            .to_string()
            .contains("REMEMBER-ME")
    );
}

#[test]
fn a_project_move_refuses_places_that_would_lose_or_mix_data() {
    let env = TestEnv::with_project("Salt Road", "Feature Film");
    env.ok("notes.create", json!({ "body": "keep" }));
    let old = PathBuf::from(env.project_path());
    let name = old.file_name().unwrap().to_owned();
    let elsewhere = root(&env).join("elsewhere");
    fs::create_dir_all(elsewhere.join(&name)).unwrap();
    fs::write(elsewhere.join(&name).join("mine.txt"), b"mine").unwrap();

    assert_eq!(
        env.err("project.move", json!({ "destParent": text(&elsewhere) })),
        "conflict.exists"
    );
    assert_eq!(
        env.err(
            "project.move",
            json!({ "destParent": text(old.parent().unwrap()) })
        ),
        "conflict.same_place"
    );
    for inside in [
        old.join("assets"),
        env.core.config.app_data_dir.clone(),
        env.core.vault_dir(),
    ] {
        assert!(
            env.call("project.move", json!({ "destParent": text(&inside) }))
                .is_err(),
            "{inside:?}"
        );
    }
    assert_eq!(
        env.err(
            "project.move",
            json!({ "destParent": text(&root(&env).join("nowhere")) })
        ),
        "not_found.folder"
    );
    // Only someone who can manage the project may move it.
    let viewer = env.actor_with_role(Role::Viewer);
    let denied = env
        .call_as(
            &viewer,
            "project.move",
            json!({ "destParent": text(&elsewhere) }),
        )
        .expect_err("a viewer cannot move the project");
    assert_eq!(denied.code.0, "permission.denied");
    // Nothing happened.
    assert!(old.join("openframe.json").is_file());
    assert!(env.ok("notes.list", json!({})).to_string().contains("keep"));
    assert_eq!(
        fs::read(elsewhere.join(&name).join("mine.txt")).unwrap(),
        b"mine"
    );
    // No project open, nothing to move.
    env.ok("project.close", json!({}));
    assert_eq!(
        env.err("project.move", json!({ "destParent": text(&elsewhere) })),
        "not_found.project_not_open"
    );
}

/// A file another program holds open makes the copy fail half way. The project is reopened at its
/// old place with everything in it, and no half copy is left.
#[cfg(windows)]
#[test]
fn a_project_move_that_fails_leaves_the_project_at_its_old_place() {
    use std::os::windows::fs::OpenOptionsExt;
    let env = TestEnv::with_project("Salt Road", "Feature Film");
    env.ok("private_note.create", json!({ "body": "REMEMBER-ME" }));
    let kept = env.write_file("kept.txt", b"copied into the project");
    env.ok("files.add", json!({ "paths": [kept], "mode": "copy" }));
    let id = env.ok("project.current", json!({}))["id"]
        .as_str()
        .unwrap()
        .to_string();
    let old = PathBuf::from(env.project_path());
    let held_path = fs::read_dir(old.join("assets"))
        .unwrap()
        .flatten()
        .flat_map(|d| fs::read_dir(d.path()).unwrap().flatten())
        .map(|f| f.path())
        .next()
        .expect("the copied file is in assets");
    let elsewhere = root(&env).join("elsewhere");
    fs::create_dir_all(&elsewhere).unwrap();

    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&held_path)
        .unwrap();
    let started = env.ok("project.move", json!({ "destParent": text(&elsewhere) }));
    let failed = finish(&env, &started).expect_err("the copy cannot read a held file");
    drop(held);

    assert!(!failed.message.is_empty());
    assert!(
        paths_equal(&env.project_path(), &old),
        "reopened where it was"
    );
    assert_eq!(env.ok("project.current", json!({}))["id"], id);
    assert!(paths_equal(&recent_path(&env, &id), &old));
    assert!(
        env.ok("private_note.list", json!({}))
            .to_string()
            .contains("REMEMBER-ME")
    );
    assert!(
        listing(&elsewhere).is_empty(),
        "no half copy: {:?}",
        listing(&elsewhere)
    );
}
