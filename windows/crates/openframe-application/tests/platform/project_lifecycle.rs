//! Project delete, locate, location check and start-up folders (Windows phase W2, "protect writers'
//! work"), through the public operation registry, the same path as Application Home.
//!
//! Deleted projects go into a test folder instead of the Recycle Bin (`project::fault::trash_into`,
//! debug builds only); the Home end-to-end step deletes through the real Recycle Bin.

use std::path::{Path, PathBuf};

use openframe_application::AppConfig;
use openframe_application::modules::project::fault;
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

fn test_trash() -> &'static Path {
    fault::trash_into(
        std::env::temp_dir().join(format!("openframe-test-trash-{}", std::process::id())),
    )
}

fn recent(env: &TestEnv) -> Vec<Value> {
    env.ok("project.list_recent", json!({}))
        .as_array()
        .unwrap()
        .clone()
}

fn recent_ids(env: &TestEnv) -> Vec<String> {
    recent(env)
        .iter()
        .map(|r| r["projectId"].as_str().unwrap().to_string())
        .collect()
}

/// A closed project: (id, folder).
fn closed_project(env: &TestEnv, title: &str) -> (String, PathBuf) {
    env.ok(
        "project.create",
        json!({ "title": title, "projectType": "Feature Film" }),
    );
    let id = env.core.project().unwrap().project_id();
    let path = PathBuf::from(env.project_path());
    env.ok("project.close", json!({}));
    (id, path)
}

#[test]
fn delete_moves_the_folder_away_and_forgets_the_project() {
    let trash = test_trash();
    let env = TestEnv::new();
    let (keep, keep_path) = closed_project(&env, "Keep Me");
    let (gone, gone_path) = closed_project(&env, "Delete Me");

    env.ok("project.delete", json!({ "projectId": gone }));
    assert!(!gone_path.exists(), "the folder left its place");
    let in_trash = std::fs::read_dir(trash)
        .unwrap()
        .filter_map(|e| e.ok())
        .any(|e| {
            e.file_name()
                .to_string_lossy()
                .ends_with("Delete Me.openframe")
        });
    assert!(in_trash, "the folder is in the bin, not destroyed");
    assert_eq!(
        recent_ids(&env),
        vec![keep.clone()],
        "only the deleted project left Home"
    );
    assert!(
        keep_path.join("openframe.json").is_file(),
        "the other project is untouched"
    );
}

#[test]
fn delete_refuses_the_open_project_and_a_folder_that_holds_another_project() {
    test_trash();
    let env = TestEnv::new();
    let (other, _) = closed_project(&env, "Other");
    let (id, path) = closed_project(&env, "Swapped");

    env.ok("project.open", json!({ "path": path }));
    assert_eq!(
        env.err("project.delete", json!({ "projectId": id })),
        "conflict.state"
    );
    assert!(path.exists());
    env.ok("project.close", json!({}));

    // The recent entry now points at a folder holding a different project: refuse, delete nothing.
    let other_path = PathBuf::from(
        recent(&env)
            .iter()
            .find(|r| r["projectId"] == other.as_str())
            .unwrap()["path"]
            .as_str()
            .unwrap(),
    );
    std::fs::rename(&path, path.with_extension("moved")).unwrap();
    std::fs::rename(&other_path, &path).unwrap();
    assert_eq!(
        env.err("project.delete", json!({ "projectId": id })),
        "conflict.state"
    );
    assert!(
        path.join("openframe.json").is_file(),
        "the other project was not deleted"
    );
    assert!(
        recent_ids(&env).contains(&id),
        "the entry stays until the user decides"
    );
}

#[test]
fn locate_points_a_moved_project_at_its_new_folder_and_opens_it() {
    let env = TestEnv::new();
    let (id, path) = closed_project(&env, "Wandering");
    let moved = env.dir.path().join("Elsewhere").join("Wandering.openframe");
    std::fs::create_dir_all(moved.parent().unwrap()).unwrap();
    std::fs::rename(&path, &moved).unwrap();

    // A folder with a different project is refused and the entry keeps its old path.
    let (_, stranger) = closed_project(&env, "Stranger");
    assert_eq!(
        env.err(
            "project.locate",
            json!({ "projectId": id, "path": stranger })
        ),
        "validation.different_project"
    );
    let entry = |env: &TestEnv| {
        recent(env)
            .into_iter()
            .find(|r| r["projectId"] == id.as_str())
            .unwrap()
    };
    assert_eq!(
        entry(&env)["path"].as_str().unwrap(),
        path.to_string_lossy()
    );

    // The manifest file itself is accepted as well as the folder.
    let opened = env.ok(
        "project.locate",
        json!({ "projectId": id, "path": moved.join("openframe.json") }),
    );
    assert_eq!(opened["project"]["id"], id.as_str());
    assert_eq!(env.project_path(), moved.to_string_lossy());
    assert_eq!(
        entry(&env)["path"].as_str().unwrap(),
        moved.to_string_lossy()
    );
}

#[test]
fn locate_refuses_a_network_share() {
    let env = TestEnv::new();
    let (id, _) = closed_project(&env, "Shared");
    let code = env.err(
        "project.locate",
        json!({ "projectId": id, "path": r"\\server\share\Shared.openframe" }),
    );
    assert!(
        code.starts_with("project_format") || code.starts_with("validation"),
        "{code}"
    );
}

#[test]
fn a_new_project_in_a_fake_onedrive_folder_is_flagged_and_existing_projects_stay() {
    let env = TestEnv::new();
    let (id, path) = closed_project(&env, "Already Here");
    let synced = env
        .dir
        .path()
        .join("OneDrive")
        .join("Documents")
        .join("OpenFrame")
        .join("Projects");
    let check = env.ok("project.location_check", json!({ "path": synced }));
    assert_eq!(check["cloudService"], "OneDrive");
    if let Some(local) = check["localAlternative"].as_str() {
        let again = env.ok("project.location_check", json!({ "path": local }));
        assert_eq!(
            again["cloudService"],
            Value::Null,
            "the offered folder is not synced"
        );
    }
    let plain = env.ok(
        "project.location_check",
        json!({ "path": env.dir.path().join("Projects") }),
    );
    assert_eq!(plain["cloudService"], Value::Null);
    // Checking never moves anything.
    assert!(path.join("openframe.json").is_file());
    assert!(recent_ids(&env).contains(&id));
}

#[test]
fn the_app_refuses_to_start_without_a_documents_folder() {
    let some = |p: &str| Some(PathBuf::from(p));
    let e =
        AppConfig::from_system_folders(some(r"C:\Data\OpenFrame"), None, None, "1.0").unwrap_err();
    assert_eq!(e.code.0, "storage.documents_unknown");
    assert!(
        e.message.contains("can't find your Documents folder"),
        "{}",
        e.message
    );
    assert!(
        !e.message.to_lowercase().contains("temp"),
        "never a temp folder"
    );

    let e = AppConfig::from_system_folders(None, some(r"C:\Docs\P"), some(r"C:\Docs\V"), "1.0")
        .unwrap_err();
    assert_eq!(e.code.0, "storage.app_data_unknown");

    let ok = AppConfig::from_system_folders(
        some(r"C:\Data"),
        some(r"C:\Docs\P"),
        some(r"C:\Docs\V"),
        "1.0",
    )
    .unwrap();
    assert_eq!(ok.projects_dir, PathBuf::from(r"C:\Docs\P"));
}
