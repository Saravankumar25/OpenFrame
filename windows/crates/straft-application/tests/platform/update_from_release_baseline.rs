//! Update from a shipped release (package file 05, item U2; `docs/release/update-simulation.md`).
//!
//! `tests/fixtures/release-baseline-1/` is what a person's data looked like at the first shipped
//! schema: a project at schema 19, the Global Idea Vault at 4, `app.sqlite` at 9 with settings,
//! appearance and recents, and `entitlement/notices.json` in the shape the first release wrote.
//! The files are frozen. They are made once, on purpose, by `regenerate_release_baseline_1` (an
//! ignored test that refuses to run once any schema has moved on). Each release adds its own
//! baseline folder (`release-baseline-2`, and so on) and one more test below, taken from the build
//! that was shipped, and keeps the older ones: every release must open every earlier release's data.
//!
//! `update_from_release_baseline` copies the fixture to a temporary data root, opens it with the
//! current build through the real app core, lets it migrate, and checks every piece of content,
//! the pre-migration copy, settings, appearance, recents, notice flags and the reminder day.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{Value, json};
use straft_application::events::RecordingSink;
use straft_application::schema::{APP_MIGRATIONS, GLOBAL_MIGRATIONS, PROJECT_MIGRATIONS};
use straft_application::{AppConfig, AppCore};
use straft_entitlement::notices::{Notice, due};
use straft_entitlement::{CalendarDay, Channel, EntitlementState, NoticeStore};
use straft_test_support::TestEnv;

/// The schema of the first shipped release. Never changes: later releases get their own folder.
const BASELINE_PROJECT: usize = 19;
const BASELINE_GLOBAL: usize = 4;
const BASELINE_APP: usize = 9;

/// The path the fixture's recent list names. The test replaces it with the temporary data root.
// LEGACY: Open Frame name kept for migration
const PLACEHOLDER_ROOT: &str = "C:/OpenFrameBaseline1";
const PROJECT_FOLDER: &str = "Night Bus";
const THEME: &str = "midnight-reel";

fn baseline_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("fixtures")
        .join("release-baseline-1")
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn user_version(db: &Path) -> i64 {
    rusqlite::Connection::open(db)
        .unwrap()
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap()
}

fn id(v: &Value) -> String {
    v.as_str()
        .or_else(|| v["id"].as_str())
        .or_else(|| v["createdIds"][0].as_str())
        .unwrap_or_else(|| panic!("no id in {v}"))
        .to_string()
}

/// Fold the write-ahead log into the file and leave a single plain database file.
fn single_file(db: &Path) {
    let c = rusqlite::Connection::open(db).unwrap();
    c.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
        .unwrap();
    c.query_row("PRAGMA journal_mode=DELETE", [], |_| Ok(()))
        .unwrap();
}

// ------------------------------------------------------------------ the generator

/// Makes `tests/fixtures/release-baseline-1/`. Run it only on purpose, on the tree that was
/// shipped:
/// `node scripts/cargo-queued.mjs test -p straft-application --test platform regenerate_release_baseline_1 -- --ignored`
#[test]
#[ignore = "remakes frozen fixtures; run on purpose only"]
fn regenerate_release_baseline_1() {
    assert_eq!(
        (
            PROJECT_MIGRATIONS.len(),
            GLOBAL_MIGRATIONS.len(),
            APP_MIGRATIONS.len()
        ),
        (BASELINE_PROJECT, BASELINE_GLOBAL, BASELINE_APP),
        "baseline 1 is the first shipped schema; a later schema gets its own baseline folder"
    );
    let env = TestEnv::new();
    // Two other projects so the recent list has an order and a pinned entry.
    env.ok(
        "project.create",
        json!({ "title": "Rain Check", "projectType": "Short Film" }),
    );
    let rain = env.ok("project.current", json!({}))["id"]
        .as_str()
        .unwrap()
        .to_string();
    env.ok(
        "project.create",
        json!({ "title": "Second Reel", "projectType": "Short Film" }),
    );
    env.ok(
        "project.create",
        json!({ "title": PROJECT_FOLDER, "projectType": "Feature Film" }),
    );
    env.ok(
        "project.set_pinned",
        json!({ "projectId": rain, "pinned": true }),
    );

    // Story.
    let act = id(&env.ok("story.create_act", json!({ "title": "Act One" })));
    env.ok(
        "story.create_card",
        json!({ "parent": { "parentType": "act", "parentId": act }, "shortDescription": "Ravi waits at the station" }),
    );
    env.ok("story.create_character", json!({ "name": "Ravi" }));
    // Screenplay: three scenes with action, a character and dialogue.
    let sp = env.ok(
        "screenplay.create",
        json!({ "title": "Night Bus", "draftName": "Draft 1" }),
    );
    let draft = sp["draft"]["id"].as_str().unwrap().to_string();
    for (heading, action) in [
        (
            "INT. BUS DEPOT - NIGHT",
            "Rain hammers the roof of the depot.",
        ),
        (
            "EXT. OLD RAILWAY STATION - DAY",
            "Ravi walks the empty platform.",
        ),
        (
            "INT. POLICE STATION - NIGHT",
            "A red folder lies on the desk.",
        ),
    ] {
        let scene = id(&env.ok(
            "screenplay.create_scene",
            json!({ "draftId": draft, "heading": heading }),
        ));
        env.ok(
            "screenplay.insert_element",
            json!({ "sceneId": scene, "elementType": "action", "text": action }),
        );
        env.ok(
            "screenplay.insert_element",
            json!({ "sceneId": scene, "elementType": "character", "text": "RAVI" }),
        );
        env.ok(
            "screenplay.insert_element",
            json!({ "sceneId": scene, "elementType": "dialogue", "text": "The last bus has gone." }),
        );
    }
    // Production: a locked draft, a schedule with one shooting day, a budget with a line.
    env.ok("screenplay.lock_draft", json!({ "draftId": draft }));
    env.ok("production.set_source", json!({ "draftId": draft }));
    let schedule = id(&env.ok("schedule.create", json!({})));
    env.ok(
        "schedule.create_day",
        json!({ "scheduleId": schedule, "date": "2027-03-15" }),
    );
    let budget = id(&env.ok("budget.create", json!({ "currency": "INR" })));
    env.ok(
        "budget.add_line",
        json!({ "budgetId": budget, "category": "Cast", "description": "Lead actors", "planned": 40_000_000 }),
    );
    env.ok(
        "notes.create",
        json!({ "title": "Location recce", "body": "Check the depot roof before the rain scene." }),
    );
    env.ok(
        "vault.create",
        json!({ "store": "project", "itemType": "note", "body": "Lighthouse keeper idea" }),
    );
    // The Global Idea Vault.
    env.ok(
        "vault.create",
        json!({ "store": "global", "itemType": "note", "body": "A ferry that never arrives" }),
    );
    // Settings and appearance.
    env.ok(
        "app.set_display_name",
        json!({ "displayName": "Nisha Verma" }),
    );
    env.ok(
        "app.set_shortcuts",
        json!({ "changes": [{ "commandId": "screenplay.find", "keys": ["Shift+Mod+F"] }] }),
    );
    env.ok("appearance.set_mode", json!({ "mode": "dark" }));
    env.ok("appearance.set_theme", json!({ "themeId": THEME }));

    let root = env.dir.path().to_path_buf();
    let project = PathBuf::from(env.project_path());
    env.ok("project.close", json!({}));
    let TestEnv { dir, core, .. } = env;
    core.shutdown();
    drop(core);

    let out = baseline_dir();
    let _ = fs::remove_dir_all(&out);
    fs::create_dir_all(out.join("appdata").join("entitlement")).unwrap();
    // Recents name this temporary folder; the fixture names a placeholder the test rewrites.
    let app_db = out.join("appdata").join("app.sqlite");
    fs::copy(root.join("appdata").join("app.sqlite"), &app_db).unwrap();
    single_file(&app_db);
    {
        let c = rusqlite::Connection::open(&app_db).unwrap();
        c.execute(
            "UPDATE sys_recent_project SET path = ?1 || '/projects/' || title",
            [PLACEHOLDER_ROOT],
        )
        .unwrap();
        // Only the project with content is kept in the fixture; the other two are listed in the
        // recents, as projects moved or deleted since are.
    }
    // The notice record in the shape the first release wrote: four fields, no version.
    fs::write(
        out.join("appdata").join("entitlement").join("notices.json"),
        br#"{
  "reminderShownOn": "2027-01-19",
  "dataNoteAcknowledged": true,
  "tutorialCompleted": true,
  "welcomeShown": false
}
"#,
    )
    .unwrap();
    let kept = out.join("projects").join(PROJECT_FOLDER);
    fs::create_dir_all(&kept).unwrap();
    for name in ["straft.json", "project.sqlite"] {
        fs::copy(project.join(name), kept.join(name)).unwrap();
    }
    single_file(&kept.join("project.sqlite"));
    let vault = out.join("global-vault");
    fs::create_dir_all(&vault).unwrap();
    fs::copy(
        root.join("global-vault").join("vault.sqlite"),
        vault.join("vault.sqlite"),
    )
    .unwrap();
    single_file(&vault.join("vault.sqlite"));
    drop(dir);

    assert_eq!(user_version(&kept.join("project.sqlite")), 19);
    assert_eq!(user_version(&vault.join("vault.sqlite")), 4);
    assert_eq!(user_version(&app_db), 9);
}

// ------------------------------------------------------------------ the update test

fn row_count(core: &AppCore, sql: &str) -> i64 {
    core.project()
        .unwrap()
        .store
        .read(|c| Ok(c.query_row(sql, [], |r| r.get(0))?))
        .unwrap()
}

fn texts(core: &AppCore, sql: &str) -> Vec<String> {
    core.project()
        .unwrap()
        .store
        .read(|c| {
            let mut st = c.prepare(sql)?;
            let rows = st
                .query_map([], |r| r.get::<_, String>(0))?
                .collect::<Result<_, _>>()?;
            Ok(rows)
        })
        .unwrap()
}

#[test]
fn update_from_release_baseline() {
    let latest = (
        PROJECT_MIGRATIONS.len(),
        GLOBAL_MIGRATIONS.len(),
        APP_MIGRATIONS.len(),
    );
    let root_dir = tempfile::tempdir().unwrap();
    let root = root_dir.path();
    let fixture = baseline_dir();
    copy_tree(&fixture.join("appdata"), &root.join("appdata"));
    copy_tree(&fixture.join("projects"), &root.join("projects"));
    copy_tree(&fixture.join("global-vault"), &root.join("global-vault"));
    let project = root.join("projects").join(PROJECT_FOLDER);

    // The fixture itself is the first shipped schema, whatever the build now is.
    assert_eq!(user_version(&project.join("project.sqlite")), 19);
    assert_eq!(
        user_version(&root.join("global-vault").join("vault.sqlite")),
        4
    );
    assert_eq!(user_version(&root.join("appdata").join("app.sqlite")), 9);
    // Recents name the placeholder; point them at this temporary root.
    {
        let c = rusqlite::Connection::open(root.join("appdata").join("app.sqlite")).unwrap();
        c.execute(
            "UPDATE sys_recent_project SET path = ?1 || substr(path, ?2)",
            rusqlite::params![
                root.to_string_lossy().replace('\\', "/"),
                PLACEHOLDER_ROOT.len() as i64 + 1
            ],
        )
        .unwrap();
    }
    let notices_before = fs::read(
        root.join("appdata")
            .join("entitlement")
            .join("notices.json"),
    )
    .unwrap();

    let sink = Arc::new(RecordingSink::default());
    let core = AppCore::new(AppConfig::isolated(root), sink).expect("the current build starts");
    let actor = core.local_actor();
    let call = |op: &str, args: Value| {
        core.dispatch(&actor, op, args)
            .unwrap_or_else(|e| panic!("{op}: {} ({:?})", e.code_str(), e.detail))
    };

    // App database: migrated to the current version, settings, appearance and recents kept.
    assert_eq!(
        core.with_app_db(straft_persistence::migrate::current_version)
            .unwrap() as usize,
        latest.2
    );
    assert_eq!(call("app.info", json!({}))["displayNameSet"], true);
    let look = call("appearance.get", json!({}));
    assert_eq!(
        (look["mode"].as_str(), look["themeId"].as_str()),
        (Some("dark"), Some(THEME))
    );
    assert_eq!(
        call("app.shortcuts", json!({}))["overrides"],
        json!([{ "commandId": "screenplay.find", "keys": ["Mod+Shift+F"] }])
    );
    let recents = call("project.list_recent", json!({}));
    let titles: Vec<(&str, bool, bool)> = recents
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["title"].as_str().unwrap(),
                r["pinned"].as_bool().unwrap(),
                r["available"].as_bool().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        titles,
        vec![
            ("Rain Check", true, false),
            (PROJECT_FOLDER, false, true),
            ("Second Reel", false, false),
        ],
        "recent list: pinned first, then newest; only the project in the fixture is on disk"
    );

    // Project: opens, migrates from 19 when the build is newer, keeps every piece of content.
    let opened = call("project.open", json!({ "path": project.to_string_lossy() }));
    // The frozen project has the earlier manifest; opening gives it the current one.
    assert!(
        project.join(straft_project_format::MANIFEST_FILE).is_file(),
        "{opened}"
    );
    assert!(
        !project
            .join(straft_platform::legacy_names::MANIFEST_FILE)
            .exists()
    );
    let migrated = latest.0 > BASELINE_PROJECT;
    assert_eq!(
        opened["migratedFrom"],
        if migrated { json!(19) } else { Value::Null },
        "{opened}"
    );
    if migrated {
        let backup = fs::read_dir(project.join("backups"))
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .find(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("pre-migration-v19-")
            })
            .expect("a pre-migration copy inside the project folder");
        assert_eq!(user_version(&backup), 19);
        let c = rusqlite::Connection::open_with_flags(
            &backup,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let scenes: i64 = c
            .query_row("SELECT count(*) FROM screenplay_scene", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            scenes, 4,
            "the copy holds the project as the old version left it"
        );
    }
    assert_eq!(row_count(&core, "PRAGMA user_version") as usize, latest.0);
    let check: String = core
        .project()
        .unwrap()
        .store
        .read(|c| Ok(c.query_row("PRAGMA integrity_check", [], |r| r.get(0))?))
        .unwrap();
    assert_eq!(check, "ok");
    assert_eq!(
        texts(
            &core,
            "SELECT heading FROM screenplay_scene WHERE deleted_at IS NULL AND heading <> '' ORDER BY position"
        ),
        [
            "INT. BUS DEPOT - NIGHT",
            "EXT. OLD RAILWAY STATION - DAY",
            "INT. POLICE STATION - NIGHT"
        ]
    );
    assert_eq!(
        texts(
            &core,
            "SELECT text FROM screenplay_element WHERE element_type='action' AND text <> '' ORDER BY text"
        ),
        [
            "A red folder lies on the desk.",
            "Rain hammers the roof of the depot.",
            "Ravi walks the empty platform."
        ]
    );
    assert_eq!(
        row_count(
            &core,
            "SELECT count(*) FROM screenplay_element WHERE element_type='dialogue' AND text='The last bus has gone.'"
        ),
        3
    );
    assert_eq!(
        texts(
            &core,
            "SELECT name FROM story_character WHERE deleted_at IS NULL"
        ),
        ["Ravi"]
    );
    assert_eq!(
        texts(
            &core,
            "SELECT title FROM story_act WHERE deleted_at IS NULL"
        ),
        ["Act One"]
    );
    assert_eq!(
        texts(
            &core,
            "SELECT description FROM budget_line WHERE deleted_at IS NULL"
        ),
        ["Lead actors"]
    );
    assert_eq!(
        row_count(
            &core,
            "SELECT amount FROM budget_line WHERE description='Lead actors'"
        ),
        40_000_000
    );
    assert_eq!(
        texts(
            &core,
            "SELECT shoot_date FROM shooting_day WHERE deleted_at IS NULL"
        ),
        ["2027-03-15"]
    );
    assert_eq!(
        texts(
            &core,
            "SELECT title FROM project_note WHERE deleted_at IS NULL"
        ),
        ["Location recce"]
    );
    assert_eq!(
        texts(
            &core,
            "SELECT body FROM vault_item WHERE deleted_at IS NULL"
        ),
        ["Lighthouse keeper idea"]
    );
    // Search finds the old content (the index is derived and was rebuilt or reused).
    let hits = call("search.query", json!({ "text": "depot" }));
    assert!(
        hits.as_array().is_some_and(|h| !h.is_empty()),
        "search finds old content: {hits}"
    );
    // The migrated project takes new work and saves it.
    call(
        "notes.create",
        json!({ "title": "After the update", "body": "Written by the new build." }),
    );
    call("project.close", json!({}));

    // Global Idea Vault: opened lazily, migrated beside itself.
    let vault = call("vault.list", json!({ "store": "global" }));
    assert!(
        vault.to_string().contains("A ferry that never arrives"),
        "{vault}"
    );
    if latest.1 > BASELINE_GLOBAL {
        let has_copy = fs::read_dir(root.join("global-vault").join("backups"))
            .unwrap()
            .flatten()
            .any(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("pre-migration-v4-")
            });
        assert!(has_copy, "a pre-migration copy inside the vault folder");
    }

    // Notice flags and the reminder day survive; the file is not rewritten or set aside.
    let notices = NoticeStore::in_app_data(&root.join("appdata"));
    let loaded = notices.load();
    assert!(!loaded.replaced);
    let rec = loaded.record;
    assert!(rec.data_note_acknowledged && rec.tutorial_completed && !rec.welcome_shown);
    assert_eq!(rec.reminder_shown_on.as_deref(), Some("2027-01-19"));
    assert!(
        !root
            .join("appdata")
            .join("entitlement")
            .join("notices.json.bad")
            .exists()
    );
    assert_eq!(
        fs::read(
            root.join("appdata")
                .join("entitlement")
                .join("notices.json")
        )
        .unwrap(),
        notices_before,
        "reading the old record does not rewrite it"
    );
    let day = |s: &str| CalendarDay::parse(s).unwrap();
    assert_eq!(
        due(
            Channel::MicrosoftStore,
            EntitlementState::Purchased,
            &rec,
            None
        ),
        Notice::Welcome,
        "the data note and the tutorial are not asked again; the welcome was not shown yet"
    );
    assert_eq!(
        due(
            Channel::MicrosoftStore,
            EntitlementState::TrialActive { days_left: 2 },
            &rec,
            Some(&day("2027-01-19"))
        ),
        Notice::None,
        "the reminder already shown that day does not come back"
    );
    assert_eq!(
        due(
            Channel::MicrosoftStore,
            EntitlementState::TrialActive { days_left: 2 },
            &rec,
            Some(&day("2027-01-20"))
        ),
        Notice::Reminder { days_left: 2 }
    );
    core.shutdown();
}
