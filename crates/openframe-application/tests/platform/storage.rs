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
