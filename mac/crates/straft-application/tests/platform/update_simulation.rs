//! Update simulation (update safety U2): the bytes a person's data had at a shipped release,
//! opened by the build under test.
//!
//! `tests/fixtures/release-1/` is frozen: its bytes never change, and every release adds a folder
//! beside it (see its README). `generate_release_1_fixture` made it once and stays ignored.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use straft_application::events::RecordingSink;
use straft_application::{AppConfig, AppCore};
use straft_test_support::TestEnv;

fn fixture_dir(release: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(release)
}

const SCRIPT: &str = "Title: Harbour Light\n\nINT. LIGHTHOUSE KEEPER'S ROOM - NIGHT\n\nMEERA winds the old clock.\n\nMEERA\nThe storm will not wait for us.\n\nEXT. JETTY - DAWN\n\nKARTHIK pulls a rope from the grey water.\n\nKARTHIK\n(whispering)\nShe was here.\n";

/// Copy a folder, leaving out the files that only exist while an app has the project open.
fn copy_tree(from: &Path, to: &Path, skip: &dyn Fn(&Path) -> bool) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap().flatten() {
        let path = entry.path();
        if skip(&path) {
            continue;
        }
        let dest = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&path, &dest, skip);
        } else {
            fs::copy(&path, &dest).unwrap();
        }
    }
}

fn names(dir: &Path) -> Vec<String> {
    fs::read_dir(dir)
        .map(|d| {
            d.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// One-time generator. Run by hand with
/// `cargo test -p straft-application --test platform generate_release_1_fixture -- --ignored`
/// on the build that ships as release 1, and commit the result. Never run it again for release 1.
#[test]
#[ignore = "writes tests/fixtures/release-1; run once, by hand, on the release 1 build"]
fn generate_release_1_fixture() {
    let env = TestEnv::new();
    env.ok(
        "project.create",
        json!({ "title": "Harbour Light", "projectType": "Feature Film" }),
    );
    let preview = env.ok("screenplay.import_preview", json!({ "pastedText": SCRIPT }));
    env.ok(
        "screenplay.import_apply",
        json!({ "previewId": preview["previewId"], "mode": "new_screenplay" }),
    );
    env.ok("story.create_character", json!({ "name": "Meera" }));
    env.ok("story.create_character", json!({ "name": "Karthik" }));
    let budget = env.ok("budget.create", json!({ "currency": "INR" }));
    env.ok(
        "budget.add_line",
        json!({ "budgetId": budget, "category": "Cast", "description": "Lead actor",
                "planned": 5_000_000, "actual": 5_500_000, "notes": "Fee negotiable" }),
    );
    env.ok(
        "notes.create",
        json!({ "body": "Ask the harbour office about the jetty." }),
    );
    let root = PathBuf::from(env.project_path());
    // Closing folds the write-ahead log into the database file.
    env.ok("project.close", json!({}));
    env.core.shutdown();

    let out = fixture_dir("release-1");
    // Only the data folders are replaced; the README beside them stays.
    for part in ["project", "appdata"] {
        let _ = fs::remove_dir_all(out.join(part));
    }
    copy_tree(&root, &out.join("project"), &|p| {
        let name = p.file_name().unwrap().to_string_lossy().into_owned();
        name.ends_with("-wal")
            || name.ends_with("-shm")
            || name == ".lock"
            || name == "recovery"
            || name == "assets"
            || name == "cache"
            || name == "backups"
    });
    let notices = out.join("appdata").join("entitlement");
    fs::create_dir_all(&notices).unwrap();
    fs::write(
        notices.join("notices.json"),
        br#"{
  "reminderShownOn": "2026-11-02",
  "dataNoteAcknowledged": true,
  "tutorialCompleted": true,
  "welcomeShown": true
}
"#,
    )
    .unwrap();
    println!("wrote {}", out.display());
}

/// The fixture's project, as a person's folder after an update installs a newer build.
#[test]
fn update_simulation_opens_release_1_project() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = fixture_dir("release-1");
    let project = dir.path().join("Harbour Light.straft");
    copy_tree(&fixture.join("project"), &project, &|_| false);
    // The notice flags sit where this build reads them (`AppConfig::isolated` puts app data here).
    let appdata = dir.path().join("appdata");
    copy_tree(&fixture.join("appdata"), &appdata, &|_| false);

    let sink = Arc::new(RecordingSink::default());
    let core = AppCore::new(AppConfig::isolated(dir.path()), sink.clone()).expect("the app starts");
    let env = TestEnv { dir, core, sink };
    let opened = env.ok("project.open", json!({ "path": project }));
    assert_eq!(opened["project"]["title"], "Harbour Light");

    // A safety copy exists only when a migration ran, and then it is inside the project folder.
    let copies: Vec<String> = names(&project.join("backups"))
        .into_iter()
        .filter(|n| n.starts_with("pre-migration-"))
        .collect();
    if opened["migratedFrom"].is_null() {
        assert!(copies.is_empty(), "no migration ran, yet {copies:?}");
    } else {
        assert_eq!(
            copies.len(),
            1,
            "one migration, one safety copy: {copies:?}"
        );
    }

    // The records are all there.
    let overview = env.ok("screenplay.overview", json!({}));
    assert_eq!(overview["drafts"][0]["sceneCount"], 2);
    let draft = overview["drafts"][0]["id"].clone();
    let scenes = env
        .ok("screenplay.document", json!({ "draftId": draft }))
        .to_string();
    for text in ["LIGHTHOUSE", "JETTY", "storm will not wait", "She was here"] {
        assert!(scenes.contains(text), "{text} missing from {scenes}");
    }
    let characters = env.ok("story.characters", json!({})).to_string();
    assert!(
        characters.contains("Meera") && characters.contains("Karthik"),
        "{characters}"
    );
    let budget = env.ok("budget.get", json!({})).to_string();
    assert!(
        budget.contains("Lead actor") && budget.contains("5500000"),
        "{budget}"
    );
    let notes = env.ok("notes.list", json!({})).to_string();
    assert!(notes.contains("harbour office"), "{notes}");

    // Search is correct once the background rebuild after open has finished.
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let found = env.ok("search.query", json!({ "text": "jetty" }));
        if found.to_string().to_lowercase().contains("jetty") {
            break;
        }
        assert!(Instant::now() < deadline, "search never found the scene");
        std::thread::sleep(Duration::from_millis(25));
    }

    env.ok("project.close", json!({}));
    env.core.shutdown();
    // The notice flags are intact and were read, not set aside.
    let saved: Value =
        serde_json::from_slice(&fs::read(appdata.join("entitlement/notices.json")).unwrap())
            .unwrap();
    assert_eq!(saved["welcomeShown"], true);
    assert_eq!(saved["tutorialCompleted"], true);
    assert_eq!(saved["dataNoteAcknowledged"], true);
    assert!(!appdata.join("entitlement/notices.json.bad").exists());
}
