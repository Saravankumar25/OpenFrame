//! Migration fixture corpus (build plan item migration test files; feature CORE-005, PLAT-005; doc 23 §4 rule 7).
//!
//! For every shipped schema version the test builds a genuine database at that version from the
//! frozen migration files (`schema::PROJECT_MIGRATIONS[..v]`, checked by
//! `scripts/check-migrations.mjs`), fills it with the content a user had created (copied from a
//! project seeded with story, screenplay and Idea Vault content, column by column wherever the
//! older schema has the table and column), and opens it with the current app. Each upgrade must
//! keep the content, write the verified pre-migration safety backup, pass `integrity_check`,
//! rebuild search, and leave the project exportable and backupable. The global Idea Vault is
//! checked the same way for every global version; the app database has a single version.
//!
//! The fixtures are generated from the frozen SQL rather than stored as binary files: until the
//! first external build nothing has shipped, and the SQL files are the exact input every earlier
//! version ran. When `migrations/frozen.json` turns `frozen`, the databases produced by the first
//! external build are added under `tests/fixtures/` as permanent fixtures (doc 23 §4).

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use openframe_application::events::{AppEvent, RecordingSink};
use openframe_application::schema::{GLOBAL_MIGRATIONS, PROJECT_MIGRATIONS};
use openframe_application::{AppConfig, AppCore};
use openframe_persistence::migrate::{self, Migration};
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

fn id(v: &Value) -> String {
    v["id"]
        .as_str()
        .or_else(|| v["createdIds"][0].as_str())
        .expect("id")
        .to_string()
}

/// Seed content touching the main tables a user fills first.
fn seed(env: &TestEnv) {
    let act = id(&env.ok("story.create_act", json!({ "title": "Act One" })));
    for text in ["Ravi waits at the station", "Anjali misses the last bus"] {
        env.ok(
            "story.create_card",
            json!({ "parent": { "parentType": "act", "parentId": act }, "shortDescription": text }),
        );
    }
    let sp = env.ok(
        "screenplay.create",
        json!({ "title": "Night Bus", "draftName": "Draft 1" }),
    );
    let draft = sp["draft"]["id"].as_str().unwrap().to_string();
    let scene = id(&env.ok(
        "screenplay.create_scene",
        json!({ "draftId": draft, "heading": "INT. BUS DEPOT - NIGHT" }),
    ));
    env.ok(
        "screenplay.insert_element",
        json!({ "sceneId": scene, "elementType": "action", "text": "Rain hammers the roof of the depot." }),
    );
    env.ok(
        "vault.create",
        json!({ "store": "project", "itemType": "note", "body": "Lighthouse keeper idea" }),
    );
}

/// Build a database at `version` from the frozen migrations and copy every row and column the
/// older schema can hold from `source`.
fn build_at_version(source: &Path, target: &Path, migrations: &[Migration], version: usize) {
    let _ = fs::remove_file(target);
    let mut c = rusqlite::Connection::open(target).unwrap();
    migrate::apply(&mut c, &migrations[..version]).unwrap();
    c.execute_batch("PRAGMA foreign_keys=OFF;").unwrap();
    c.execute("ATTACH DATABASE ?1 AS src", [source.to_string_lossy()])
        .unwrap();
    let tables: Vec<String> = {
        let mut st = c
            .prepare(
                "SELECT name FROM main.sqlite_master WHERE type='table'
                 AND name NOT LIKE 'sqlite_%' AND name NOT LIKE 'search_%' AND name <> '_schema_migrations'
                 AND name IN (SELECT name FROM src.sqlite_master WHERE type='table')",
            )
            .unwrap();
        st.query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    };
    let columns = |c: &rusqlite::Connection, schema: &str, table: &str| -> Vec<String> {
        let mut st = c
            .prepare(&format!("PRAGMA {schema}.table_info(\"{table}\")"))
            .unwrap();
        st.query_map([], |r| r.get::<_, String>(1))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    };
    for t in &tables {
        let theirs = columns(&c, "src", t);
        let common: Vec<String> = columns(&c, "main", t)
            .into_iter()
            .filter(|col| theirs.contains(col))
            .collect();
        let cols = common
            .iter()
            .map(|c| format!("\"{c}\""))
            .collect::<Vec<_>>()
            .join(", ");
        c.execute(&format!("DELETE FROM main.\"{t}\""), []).unwrap();
        c.execute(
            &format!("INSERT INTO main.\"{t}\" ({cols}) SELECT {cols} FROM src.\"{t}\""),
            [],
        )
        .unwrap_or_else(|e| panic!("v{version}: copying {t} into the older schema failed: {e}"));
    }
    c.execute_batch("DETACH DATABASE src; PRAGMA foreign_keys=ON;")
        .unwrap();
    let fk: Vec<String> = {
        let mut st = c.prepare("PRAGMA foreign_key_check").unwrap();
        st.query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    };
    assert!(
        fk.is_empty(),
        "v{version}: fixture violates foreign keys in {fk:?}"
    );
}

fn set_manifest_version(project: &Path, version: usize) {
    let path = project.join("openframe.json");
    let mut manifest: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    manifest["schemaVersion"] = json!(version);
    fs::write(&path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
}

fn remove_wal(db: &Path) {
    for ext in ["sqlite-wal", "sqlite-shm"] {
        let _ = fs::remove_file(db.with_extension(ext));
    }
}

fn wait_task(sink: &RecordingSink, started: &Value) -> (String, Value) {
    let id = started["taskId"].as_str().unwrap().to_string();
    let start = Instant::now();
    loop {
        for e in sink.events.lock().iter().rev() {
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
        assert!(
            start.elapsed() < Duration::from_secs(120),
            "task did not finish"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Every row of every table, as text, sorted (tables without a rowid have no natural order).
fn table_rows(c: &rusqlite::Connection) -> Vec<(String, Vec<String>)> {
    let tables: Vec<String> = {
        let mut st = c
            .prepare(
                "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
            )
            .unwrap();
        st.query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    };
    tables
        .into_iter()
        .map(|t| {
            let mut st = c.prepare(&format!("SELECT * FROM \"{t}\"")).unwrap();
            let n = st.column_count();
            let mut rows: Vec<String> = st
                .query_map([], |r| {
                    Ok((0..n)
                        .map(|i| format!("{:?}", r.get::<_, rusqlite::types::Value>(i).unwrap()))
                        .collect::<Vec<_>>()
                        .join("|"))
                })
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap();
            rows.sort();
            (t, rows)
        })
        .collect()
}

/// Open the pre-migration safety backup read-only, as the older version would, and compare it
/// with the fixture it was taken from: same schema version, a sound file, the same rows.
fn assert_backup_matches_fixture(backup: &Path, fixture: &Path, version: usize) {
    let open = |p: &Path| {
        rusqlite::Connection::open_with_flags(p, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap_or_else(|e| panic!("v{version}: {} does not open: {e}", p.display()))
    };
    let (b, f) = (open(backup), open(fixture));
    let user_version = |c: &rusqlite::Connection| -> i64 {
        c.query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap()
    };
    assert_eq!(
        user_version(&b),
        version as i64,
        "v{version}: backup schema version"
    );
    let check: String = b
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .unwrap();
    assert_eq!(check, "ok", "v{version}: backup integrity");
    assert_eq!(
        table_rows(&b),
        table_rows(&f),
        "v{version}: the safety backup differs from the database it was taken from"
    );
}

fn scalar(core: &AppCore, sql: &str) -> i64 {
    core.project()
        .unwrap()
        .store
        .read(|c| Ok(c.query_row(sql, [], |r| r.get(0))?))
        .unwrap()
}

#[test]
fn m1_04_every_project_schema_version_upgrades_with_content_backup_search_and_export() {
    let latest = PROJECT_MIGRATIONS.len();
    for version in 1..=latest {
        let env = TestEnv::with_project("Night Bus", "Feature Film");
        seed(&env);
        let project = env.project_path();
        let root = Path::new(&project).to_path_buf();
        env.ok("project.close", json!({}));
        let db = root.join("project.sqlite");
        remove_wal(&db);
        let expected: Vec<(&str, i64)> = {
            let c = rusqlite::Connection::open(&db).unwrap();
            ["story_scene_card", "screenplay_scene", "vault_item"]
                .into_iter()
                .map(|t| {
                    let n = c
                        .query_row(
                            &format!("SELECT count(*) FROM {t} WHERE deleted_at IS NULL"),
                            [],
                            |r| r.get(0),
                        )
                        .unwrap();
                    (t, n)
                })
                .collect()
        };
        // The table each count needs, by the migration that created it.
        let created_in = |t: &str| match t {
            "vault_item" => 2,
            "story_scene_card" => 3,
            _ => 4,
        };
        let fixture = env.dir.path().join(format!("schema-v{version}.sqlite"));
        build_at_version(&db, &fixture, PROJECT_MIGRATIONS, version);
        fs::copy(&fixture, &db).unwrap();
        remove_wal(&db);
        set_manifest_version(&root, version);

        let sink = Arc::new(RecordingSink::default());
        let core = AppCore::new(AppConfig::isolated(env.dir.path()), sink.clone()).unwrap();
        let actor = core.local_actor();
        let opened = core
            .dispatch(&actor, "project.open", json!({ "path": project }))
            .unwrap_or_else(|e| panic!("v{version} does not open: {} {:?}", e.message, e.detail));
        if version < latest {
            assert_eq!(
                opened["migratedFrom"],
                json!(version),
                "v{version}: {opened}"
            );
            let backups: Vec<String> = fs::read_dir(root.join("backups"))
                .map(|d| {
                    d.filter_map(|e| e.ok())
                        .map(|e| e.file_name().to_string_lossy().into_owned())
                        .collect()
                })
                .unwrap_or_default();
            let backup = backups
                .iter()
                .find(|n| n.starts_with(&format!("pre-migration-v{version}-")))
                .unwrap_or_else(|| panic!("v{version}: no safety backup in {backups:?}"));
            // Restore check: the backup is the database as the older version left it, so a reader
            // that knows only that version opens it and finds every row of the fixture.
            assert_backup_matches_fixture(&root.join("backups").join(backup), &fixture, version);
        }
        let check: String = core
            .project()
            .unwrap()
            .store
            .read(|c| Ok(c.query_row("PRAGMA integrity_check", [], |r| r.get(0))?))
            .unwrap();
        assert_eq!(check, "ok", "v{version}");
        assert_eq!(
            scalar(&core, "PRAGMA user_version"),
            latest as i64,
            "v{version}"
        );

        // Content created before the upgrade survives wherever the older schema could hold it.
        for (table, n) in &expected {
            if version >= created_in(table) {
                assert!(*n > 0, "seed created no {table}");
                assert_eq!(
                    scalar(
                        &core,
                        &format!("SELECT count(*) FROM {table} WHERE deleted_at IS NULL")
                    ),
                    *n,
                    "v{version}: {table}"
                );
            }
        }
        if version >= 3 {
            let hits = core
                .dispatch(&actor, "search.query", json!({ "text": "station" }))
                .unwrap();
            assert!(
                !hits.as_array().unwrap().is_empty(),
                "v{version}: search was not rebuilt"
            );
            core.dispatch(
                &actor,
                "story.export_board",
                json!({ "format": "pdf", "path": env.dir.path().join(format!("v{version}.pdf")) }),
            )
            .unwrap_or_else(|e| panic!("v{version}: export failed: {}", e.message));
        }
        // Backupable, and the derived AI index rebuilds from the upgraded data.
        let started = core
            .dispatch(
                &actor,
                "packages.create_backup",
                json!({ "path": env.dir.path().join(format!("v{version}.ofbackup")) }),
            )
            .unwrap();
        let (state, detail) = wait_task(&sink, &started);
        assert_eq!(state, "completed", "v{version}: backup {detail}");
        let started = core
            .dispatch(&actor, "ai.index_rebuild", json!({}))
            .unwrap();
        let (state, detail) = wait_task(&sink, &started);
        assert_eq!(state, "completed", "v{version}: AI index rebuild {detail}");
        core.dispatch(&actor, "project.close", json!({})).unwrap();
    }
}

#[test]
fn m1_04_every_global_vault_schema_version_upgrades_and_keeps_its_items() {
    let latest = GLOBAL_MIGRATIONS.len();
    for version in 1..=latest {
        let env = TestEnv::new();
        env.ok(
            "vault.create",
            json!({ "store": "global", "itemType": "note", "body": "Lighthouse keeper idea" }),
        );
        let vault_dir = env.dir.path().join("global-vault");
        let db = vault_dir.join("vault.sqlite");
        let TestEnv { dir, core, .. } = env;
        core.shutdown();
        drop(core);
        remove_wal(&db);
        let fixture = dir.path().join(format!("vault-v{version}.sqlite"));
        build_at_version(&db, &fixture, GLOBAL_MIGRATIONS, version);
        fs::copy(&fixture, &db).unwrap();
        remove_wal(&db);

        let core = AppCore::new(
            AppConfig::isolated(dir.path()),
            Arc::new(RecordingSink::default()),
        )
        .unwrap();
        let items = core
            .dispatch(
                &core.local_actor(),
                "vault.list",
                json!({ "store": "global" }),
            )
            .unwrap_or_else(|e| {
                panic!(
                    "vault v{version} does not open: {} {:?}",
                    e.message, e.detail
                )
            });
        if version >= 2 {
            assert_eq!(
                items.as_array().unwrap().len(),
                1,
                "vault v{version}: {items}"
            );
        }
        if version < latest {
            let backups: Vec<String> = fs::read_dir(vault_dir.join("backups"))
                .map(|d| {
                    d.filter_map(|e| e.ok())
                        .map(|e| e.file_name().to_string_lossy().into_owned())
                        .collect()
                })
                .unwrap_or_default();
            assert!(
                backups
                    .iter()
                    .any(|n| n.starts_with(&format!("pre-migration-v{version}-"))),
                "vault v{version}: no safety backup in {backups:?}"
            );
        }
    }
}

#[test]
fn every_app_database_version_upgrades_and_keeps_recents_and_settings() {
    // App migration 0002 (appearance) on a database created by version 1: the recent projects and
    // settings stay, and the appearance starts at the defaults (System, OpenFrame Default), so an
    // existing installation keeps today's look.
    use openframe_application::schema::APP_MIGRATIONS;
    for version in 1..APP_MIGRATIONS.len() {
        let dir = tempfile::tempdir().unwrap();
        let appdata = dir.path().join("appdata");
        fs::create_dir_all(&appdata).unwrap();
        {
            let mut c = rusqlite::Connection::open(appdata.join("app.sqlite")).unwrap();
            migrate::apply(&mut c, &APP_MIGRATIONS[..version]).unwrap();
            c.execute(
                "INSERT INTO sys_recent_project(project_id, path, title, project_type, status, archived, pinned, last_opened_at, updated_at)
                 VALUES ('p1', 'C:/films/Night Bus', 'Night Bus', 'Feature Film', 'Development', 0, 1, 5, 5)",
                [],
            )
            .unwrap();
            c.execute(
                "INSERT INTO sys_settings(key, value_json, updated_at) VALUES ('profile.display_name_confirmed', 'true', 5)",
                [],
            )
            .unwrap();
        }
        let core = AppCore::new(
            AppConfig::isolated(dir.path()),
            Arc::new(RecordingSink::default()),
        )
        .unwrap();
        let actor = core.local_actor();
        let recents = core
            .dispatch(&actor, "project.list_recent", json!({}))
            .unwrap();
        assert_eq!(recents[0]["title"], "Night Bus", "v{version}: {recents}");
        assert_eq!(recents[0]["pinned"], true);
        let app = core.dispatch(&actor, "app.info", json!({})).unwrap();
        assert_eq!(app["displayNameSet"], true, "v{version}: {app}");
        let look = core.dispatch(&actor, "appearance.get", json!({})).unwrap();
        assert_eq!(look["mode"], "system");
        assert_eq!(look["themeId"], "openframe-default");
        assert_eq!(
            core.with_app_db(openframe_persistence::migrate::current_version)
                .unwrap() as usize,
            APP_MIGRATIONS.len()
        );
        core.dispatch(&actor, "appearance.set_mode", json!({ "mode": "dark" }))
            .unwrap();
        core.shutdown();
    }
}

#[test]
fn m1_04_the_app_database_reopens_with_its_recent_projects() {
    // The app database has one schema version; it must reopen as-is and keep the recent list.
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    env.ok("project.close", json!({}));
    let TestEnv { dir, core, .. } = env;
    core.shutdown();
    drop(core);
    let core = AppCore::new(
        AppConfig::isolated(dir.path()),
        Arc::new(RecordingSink::default()),
    )
    .unwrap();
    let recents = core
        .dispatch(&core.local_actor(), "project.list_recent", json!({}))
        .unwrap();
    assert_eq!(recents.as_array().unwrap().len(), 1, "{recents}");
}

/// Project schema 11 to 12 (Story Board and Idea Vault area): the long scene description and the
/// research flag arrive empty, every card and vault item keeps its text, both new fields can be
/// written right after the upgrade, and the research flag column accepts only the three flags.
#[test]
fn v11_to_v12_adds_long_description_and_research_flag_without_touching_content() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    seed(&env);
    let project = env.project_path();
    let root = Path::new(&project).to_path_buf();
    env.ok("project.close", json!({}));
    let db = root.join("project.sqlite");
    remove_wal(&db);
    let fixture = env.dir.path().join("schema-v11.sqlite");
    build_at_version(&db, &fixture, PROJECT_MIGRATIONS, 11);
    fs::copy(&fixture, &db).unwrap();
    remove_wal(&db);
    set_manifest_version(&root, 11);

    let core = AppCore::new(
        AppConfig::isolated(env.dir.path()),
        Arc::new(RecordingSink::default()),
    )
    .unwrap();
    let actor = core.local_actor();
    let opened = core
        .dispatch(&actor, "project.open", json!({ "path": project }))
        .unwrap();
    assert_eq!(opened["migratedFrom"], json!(11), "{opened}");
    assert_eq!(
        scalar(&core, "PRAGMA user_version"),
        PROJECT_MIGRATIONS.len() as i64
    );
    assert_eq!(
        scalar(
            &core,
            "SELECT count(*) FROM story_scene_card WHERE long_description IS NULL AND short_description <> ''"
        ),
        2
    );
    assert_eq!(
        scalar(
            &core,
            "SELECT count(*) FROM vault_item WHERE research_flag IS NULL AND body = 'Lighthouse keeper idea'"
        ),
        1
    );
    let board = core.dispatch(&actor, "story.board", json!({})).unwrap();
    let card = board["acts"][0]["items"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    core.dispatch(
        &actor,
        "story.update_card",
        json!({ "id": card, "longDescription": "Written after the upgrade." }),
    )
    .unwrap();
    let item = core
        .dispatch(&actor, "vault.list", json!({ "store": "project" }))
        .unwrap()[0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    core.dispatch(
        &actor,
        "vault.set_research_flag",
        json!({ "store": "project", "ids": [item], "flag": "needsExpertCheck" }),
    )
    .unwrap();
    core.dispatch(&actor, "project.close", json!({})).unwrap();
    core.shutdown();
    drop(core);

    let c = rusqlite::Connection::open(&db).unwrap();
    let stored: (String, String) = c
        .query_row(
            "SELECT (SELECT long_description FROM story_scene_card WHERE id=?1),
                    (SELECT research_flag FROM vault_item WHERE id=?2)",
            [&card, &item],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(
        stored,
        (
            "Written after the upgrade.".to_string(),
            "needs_expert_check".to_string()
        )
    );
    assert!(
        c.execute(
            "UPDATE vault_item SET research_flag='true' WHERE id=?1",
            [&item]
        )
        .is_err(),
        "research_flag accepts only the three values"
    );
}

/// Migration 0013 (screenplay engine) rebuilds `screenplay_element`: every element keeps its id,
/// rev, timestamps, text, dual flag and revision mark; scenes and the screenplay gain the new
/// columns with values that reproduce the old behaviour (derived numbers, standard page setup).
#[test]
fn screenplay_engine_migration_keeps_elements_and_defaults() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    seed(&env);
    let project = env.project_path();
    let root = Path::new(&project).to_path_buf();
    env.ok("project.close", json!({}));
    let db = root.join("project.sqlite");
    remove_wal(&db);
    // The last version before 0013 (screenplay engine), whatever came after it.
    let before_version = 12;
    let fixture = env.dir.path().join("schema-before-engine.sqlite");
    build_at_version(&db, &fixture, PROJECT_MIGRATIONS, before_version);
    type Row = (String, String, i64, i64, i64, String, i64, Option<String>);
    let rows = |c: &rusqlite::Connection| -> Vec<Row> {
        let mut st = c
            .prepare("SELECT id, element_type, rev, created_at, updated_at, text, dual, revision_mark FROM screenplay_element ORDER BY id")
            .unwrap();
        st.query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                r.get(7)?,
            ))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
    };
    let before = {
        let c = rusqlite::Connection::open(&fixture).unwrap();
        c.execute(
            "UPDATE screenplay_element SET revision_mark = 'Blue', dual = 1",
            [],
        )
        .unwrap();
        rows(&c)
    };
    assert!(!before.is_empty());
    fs::copy(&fixture, &db).unwrap();
    remove_wal(&db);
    set_manifest_version(&root, before_version);
    let core = AppCore::new(
        AppConfig::isolated(env.dir.path()),
        Arc::new(RecordingSink::default()),
    )
    .unwrap();
    let actor = core.local_actor();
    core.dispatch(&actor, "project.open", json!({ "path": project }))
        .unwrap();
    let after = core.project().unwrap().store.read(|c| Ok(rows(c))).unwrap();
    assert_eq!(
        after, before,
        "elements keep id, rev, timestamps, text, dual and revision mark"
    );
    assert_eq!(
        scalar(
            &core,
            "SELECT count(*) FROM screenplay_scene WHERE production_number IS NOT NULL"
        ),
        0
    );
    assert_eq!(
        scalar(
            &core,
            "SELECT count(*) FROM screenplay WHERE page_setup_json <> '{}'"
        ),
        0
    );
    assert_eq!(
        scalar(&core, "SELECT count(*) FROM screenplay_custom_element"),
        0
    );
    let integrity: String = core
        .project()
        .unwrap()
        .store
        .read(|c| {
            Ok(
                c.query_row("PRAGMA foreign_key_check", [], |r| r.get::<_, String>(0))
                    .unwrap_or_else(|_| "ok".into()),
            )
        })
        .unwrap();
    assert_eq!(integrity, "ok");
    core.dispatch(&actor, "project.close", json!({})).unwrap();
}

/// Several screenplays per project (ADR-0012 amendment of 2026-10-08, masterplan
/// area 11): a project made before the change, at every schema version from
/// 0012 on, opens with its one screenplay intact and
/// chosen without being asked, and can then take a second screenplay without
/// touching the first.
#[test]
fn a_project_from_before_several_screenplays_opens_with_its_one_screenplay_intact() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    seed(&env);
    let before_overview = env.ok("screenplay.overview", json!({}));
    let sp_id = before_overview["screenplay"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let draft_id = before_overview["drafts"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let before_doc = env.ok("screenplay.document", json!({ "draftId": draft_id }));
    let project = env.project_path();
    let root = Path::new(&project).to_path_buf();
    env.ok("project.close", json!({}));
    let db = root.join("project.sqlite");
    remove_wal(&db);
    let pristine = env.dir.path().join("pristine.sqlite");
    fs::copy(&db, &pristine).unwrap();

    // Every version from the one before the screenplay engine (0012) up to
    // the current one.
    for version in 12..=PROJECT_MIGRATIONS.len() {
        let fixture = env
            .dir
            .path()
            .join(format!("one-screenplay-v{version}.sqlite"));
        build_at_version(&pristine, &fixture, PROJECT_MIGRATIONS, version);
        fs::copy(&fixture, &db).unwrap();
        remove_wal(&db);
        set_manifest_version(&root, version);
        let core = AppCore::new(
            AppConfig::isolated(env.dir.path()),
            Arc::new(RecordingSink::default()),
        )
        .unwrap();
        let actor = core.local_actor();
        core.dispatch(&actor, "project.open", json!({ "path": project }))
            .unwrap();

        let o = core
            .dispatch(&actor, "screenplay.overview", json!({}))
            .unwrap();
        let list = o["screenplays"].as_array().unwrap();
        assert_eq!(list.len(), 1, "v{version}: exactly the one screenplay");
        assert_eq!(list[0]["id"], sp_id.as_str(), "v{version}");
        assert_eq!(list[0]["title"], "Night Bus", "v{version}");
        assert_eq!(
            o["screenplay"]["id"],
            sp_id.as_str(),
            "v{version}: opened without asking"
        );
        assert_eq!(o["drafts"].as_array().unwrap().len(), 1, "v{version}");
        assert_eq!(o["drafts"][0]["isCurrent"], true, "v{version}");
        let doc = core
            .dispatch(
                &actor,
                "screenplay.document",
                json!({ "draftId": draft_id }),
            )
            .unwrap();
        assert_eq!(
            doc["scenes"], before_doc["scenes"],
            "v{version}: text intact"
        );
        let loc = core
            .dispatch(&actor, "screenplay.locate", json!({ "draftId": draft_id }))
            .unwrap();
        assert_eq!(loc["screenplayId"], sp_id.as_str(), "v{version}");

        // A second screenplay can now be added; the first does not change.
        let second = core
            .dispatch(&actor, "screenplay.create", json!({}))
            .unwrap();
        assert_eq!(second["screenplay"]["title"], "Night Bus (2)", "v{version}");
        let doc = core
            .dispatch(
                &actor,
                "screenplay.document",
                json!({ "draftId": draft_id }),
            )
            .unwrap();
        assert_eq!(
            doc["scenes"], before_doc["scenes"],
            "v{version}: first untouched"
        );
        assert_eq!(
            scalar(
                &core,
                "SELECT count(*) FROM screenplay WHERE deleted_at IS NULL"
            ),
            2,
            "v{version}"
        );
        core.dispatch(&actor, "project.close", json!({})).unwrap();
    }
}

/// Migration 0015 (schedule day times, strip start, Day Out of Days) on a schedule made with
/// schema 14: the days, strips and estimates stay as they were, the new times are empty (the
/// timeline falls back to order and durations), the Day Out of Days code style is the industry
/// default, and the new operations work on the upgraded project.
#[test]
fn v14_to_v15_keeps_an_existing_schedule_and_adds_empty_day_times() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    seed(&env);
    let project = env.project_path();
    let root = Path::new(&project).to_path_buf();
    env.ok("project.close", json!({}));
    let db = root.join("project.sqlite");
    remove_wal(&db);
    let fixture = env.dir.path().join("schema-v14.sqlite");
    build_at_version(&db, &fixture, PROJECT_MIGRATIONS, 14);
    {
        let c = rusqlite::Connection::open(&fixture).unwrap();
        let (scene, lineage, draft): (String, String, String) = c
            .query_row(
                "SELECT id, lineage_id, draft_id FROM screenplay_scene LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        c.execute_batch(&format!(
            "INSERT INTO production_source(id,draft_id,active,selected_at,created_at,updated_at) VALUES ('ps','{draft}',1,1,1,1);
             INSERT INTO shooting_schedule(id,source_id,created_at,updated_at) VALUES ('sch','ps',1,1);
             INSERT INTO shooting_day(id,schedule_id,position,shoot_date,created_at,updated_at) VALUES ('day','sch',1,'2027-06-14',1,1);
             INSERT INTO schedule_strip(id,schedule_id,day_id,scene_id,scene_lineage_id,position,estimated_minutes,created_at,updated_at)
                 VALUES ('st','sch','day','{scene}','{lineage}',1,45,1,1);"
        ))
        .unwrap();
    }
    fs::copy(&fixture, &db).unwrap();
    remove_wal(&db);
    set_manifest_version(&root, 14);

    let core = AppCore::new(
        AppConfig::isolated(env.dir.path()),
        Arc::new(RecordingSink::default()),
    )
    .unwrap();
    let actor = core.local_actor();
    let opened = core
        .dispatch(&actor, "project.open", json!({ "path": project }))
        .unwrap();
    assert_eq!(opened["migratedFrom"], json!(14), "{opened}");
    assert_eq!(
        scalar(&core, "PRAGMA user_version"),
        PROJECT_MIGRATIONS.len() as i64
    );
    assert_eq!(
        scalar(
            &core,
            "SELECT count(*) FROM shooting_day WHERE crew_call IS NULL AND shooting_start IS NULL AND target_wrap IS NULL"
        ),
        1
    );
    assert_eq!(
        scalar(
            &core,
            "SELECT count(*) FROM schedule_strip WHERE start_minutes IS NULL AND estimated_minutes = 45 AND day_id = 'day'"
        ),
        1
    );
    assert_eq!(
        scalar(
            &core,
            "SELECT count(*) FROM shooting_schedule WHERE dood_code_style = 'industry'"
        ),
        1
    );
    let view = core.dispatch(&actor, "schedule.get", json!({})).unwrap();
    let day = &view["days"][0];
    assert_eq!(day["id"], "day", "{view}");
    assert_eq!(day["items"][0]["strip"]["estimatedMinutes"], 45);
    assert!(day["timeline"].is_object(), "{day}");
    core.dispatch(
        &actor,
        "schedule.set_day_times",
        json!({ "dayId": "day", "crewCall": 420, "shootingStart": 480, "targetWrap": 1140 }),
    )
    .unwrap();
    assert_eq!(
        scalar(&core, "SELECT crew_call FROM shooting_day WHERE id = 'day'"),
        420
    );
    let dood = core.dispatch(&actor, "schedule.dood", json!({})).unwrap();
    assert_eq!(dood["codeStyle"], "industry", "{dood}");
    core.dispatch(&actor, "project.close", json!({})).unwrap();
}

/// Migration 0016 (call sheets, budget; masterplan area "Call sheets, budget, templates, samples
/// and exports") on a project made with schema 15: every budget amount entered before becomes the
/// Planned amount with no Actual, and the call sheet status words map onto Draft, Needs refresh
/// and Final (a replaced revision keeps its link). Content is never changed.
#[test]
fn v15_to_v16_keeps_amounts_as_planned_and_maps_call_sheet_status_words() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    seed(&env);
    let project = env.project_path();
    let root = Path::new(&project).to_path_buf();
    env.ok("project.close", json!({}));
    let db = root.join("project.sqlite");
    remove_wal(&db);
    let fixture = env.dir.path().join("schema-v15.sqlite");
    build_at_version(&db, &fixture, PROJECT_MIGRATIONS, 15);
    {
        let c = rusqlite::Connection::open(&fixture).unwrap();
        let draft: String = c
            .query_row("SELECT draft_id FROM screenplay_scene LIMIT 1", [], |r| {
                r.get(0)
            })
            .unwrap();
        let sheet = |id: &str, status: &str, superseded: &str| {
            format!(
                "INSERT INTO call_sheet(id,schedule_id,shoot_day_id,revision,title,status,source_snapshot_json,source_fingerprint,source_captured_at,document_json,superseded_by,created_at,updated_at)
                 VALUES ('{id}','sch','day',1,'Call Sheet {id}','{status}','{{}}','fp',1,'{{\"crewCall\":\"06:30\"}}',{superseded},1,1);"
            )
        };
        c.execute_batch(&format!(
            "INSERT INTO budget_snapshot(id,currency,created_at,updated_at) VALUES ('b','INR',1,1);
             INSERT INTO budget_line(id,budget_id,category,description,amount,position,created_at,updated_at) VALUES
                 ('l1','b','Cast','Lead actor',5000000,1,1,1),
                 ('l2','b','Food','Catering',0,2,1,1);
             INSERT INTO production_source(id,draft_id,active,selected_at,created_at,updated_at) VALUES ('ps','{draft}',1,1,1,1);
             INSERT INTO shooting_schedule(id,source_id,created_at,updated_at) VALUES ('sch','ps',1,1);
             INSERT INTO shooting_day(id,schedule_id,position,created_at,updated_at) VALUES ('day','sch',1,1,1);
             {}{}{}{}{}{}",
            sheet("new", "Draft", "NULL"),
            sheet("ready", "Ready", "NULL"),
            sheet("stale", "Needs Refresh", "NULL"),
            sheet("final", "Final", "NULL"),
            sheet("issued", "Issued", "NULL"),
            sheet("old", "Superseded", "'issued'"),
        ))
        .unwrap();
    }
    fs::copy(&fixture, &db).unwrap();
    remove_wal(&db);
    set_manifest_version(&root, 15);

    let core = AppCore::new(
        AppConfig::isolated(env.dir.path()),
        Arc::new(RecordingSink::default()),
    )
    .unwrap();
    let actor = core.local_actor();
    let opened = core
        .dispatch(&actor, "project.open", json!({ "path": project }))
        .unwrap();
    assert_eq!(opened["migratedFrom"], json!(15), "{opened}");
    assert_eq!(
        scalar(&core, "PRAGMA user_version"),
        PROJECT_MIGRATIONS.len() as i64
    );
    // Amounts become Planned exactly as entered; Actual starts empty.
    assert_eq!(
        scalar(&core, "SELECT amount FROM budget_line WHERE id = 'l1'"),
        5_000_000
    );
    assert_eq!(
        scalar(
            &core,
            "SELECT count(*) FROM budget_line WHERE actual IS NULL"
        ),
        2
    );
    let budget = core.dispatch(&actor, "budget.get", json!({})).unwrap();
    let lines = budget["current"]["lines"].as_array().unwrap();
    assert_eq!(lines[0]["planned"], 5_000_000, "{budget}");
    assert_eq!(lines[0]["actual"], Value::Null);
    assert_eq!(budget["current"]["plannedLinesTotal"], 5_000_000);
    assert_eq!(budget["current"]["actualCount"], 0);
    // Status words.
    let status = |id: &str| {
        let s: String = core
            .project()
            .unwrap()
            .store
            .read(|c| {
                Ok(c.query_row(
                    "SELECT status || ':' || needs_refresh FROM call_sheet WHERE id = ?1",
                    [id],
                    |r| r.get(0),
                )?)
            })
            .unwrap();
        s
    };
    assert_eq!(status("new"), "Draft:0");
    assert_eq!(status("ready"), "Draft:0");
    assert_eq!(
        status("stale"),
        "Draft:1",
        "Needs Refresh becomes a flagged Draft"
    );
    assert_eq!(status("final"), "Final:0");
    assert_eq!(status("issued"), "Final:0");
    assert_eq!(status("old"), "Final:0");
    assert_eq!(
        scalar(
            &core,
            "SELECT count(*) FROM call_sheet WHERE id = 'old' AND superseded_by = 'issued'"
        ),
        1,
        "a replaced revision keeps its link"
    );
    assert_eq!(
        scalar(
            &core,
            "SELECT count(*) FROM call_sheet WHERE document_json = '{\"crewCall\":\"06:30\"}'"
        ),
        6,
        "documents are never touched"
    );
    // The editable sheets still open and edit after the upgrade; the final ones stay frozen.
    let got = core
        .dispatch(&actor, "callsheets.get", json!({ "id": "stale" }))
        .unwrap();
    assert_eq!(got["status"], "Needs refresh", "{got}");
    assert_eq!(got["document"]["crewCall"], "06:30");
    let got = core
        .dispatch(&actor, "callsheets.get", json!({ "id": "issued" }))
        .unwrap();
    assert_eq!(got["status"], "Final");
    assert_eq!(got["editable"], false);
    core.dispatch(&actor, "project.close", json!({})).unwrap();
}
