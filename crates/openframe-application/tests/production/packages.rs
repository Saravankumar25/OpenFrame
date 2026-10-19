//! Backup, Full Project Package and Exchange/Review/Response package acceptance
//! tests (FSD §44.5–44.8, §45, §47, §50, §118, §121; FSD-COL-001/002,
//! FSD-OFF-007/008; Import/Export IEX-011..032; Security §8.6), through the
//! public operation registry — the same path as the UI.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use openframe_application::events::AppEvent;
use openframe_application::modules::packages::backup::{
    DbSource, ProjectPackageOptions, ProjectPackageSource,
};
use openframe_application::modules::packages::format::PackageUser;
use openframe_application::modules::packages::{PackageType, build_project_package};
use openframe_domain::{Role, new_id};
use openframe_security::archive::{self, ArchiveLimits, EntrySource};
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

// ------------------------------------------------------------------ helpers

/// Wait for a background task; returns (state, result-or-error JSON).
fn wait_task(env: &TestEnv, id: &str) -> (String, Value) {
    let start = Instant::now();
    loop {
        {
            let events = env.sink.events.lock();
            for e in events.iter().rev() {
                if let AppEvent::Task(t) = e
                    && t.task_id == id
                    && matches!(t.state.as_str(), "completed" | "failed" | "cancelled")
                {
                    let v = match (&t.result, &t.error) {
                        (Some(r), _) => r.clone(),
                        (_, Some(err)) => {
                            json!({ "code": err.code.0, "message": err.message, "detail": err.detail })
                        }
                        _ => Value::Null,
                    };
                    return (t.state.clone(), v);
                }
            }
        }
        assert!(
            start.elapsed() < Duration::from_secs(120),
            "task {id} did not finish"
        );
        std::thread::sleep(Duration::from_millis(15));
    }
}

fn task_ok(env: &TestEnv, started: &Value) -> Value {
    let (state, v) = wait_task(env, started["taskId"].as_str().unwrap());
    assert_eq!(state, "completed", "task failed: {v}");
    v
}

fn scalar_i64(env: &TestEnv, sql: &str, p: &[&str]) -> i64 {
    env.core
        .project()
        .unwrap()
        .store
        .read(|c| {
            Ok(c.query_row(sql, rusqlite::params_from_iter(p.iter()), |r| {
                r.get::<_, i64>(0)
            })?)
        })
        .unwrap()
}

fn scalar_str(env: &TestEnv, sql: &str, p: &[&str]) -> String {
    env.core
        .project()
        .unwrap()
        .store
        .read(|c| {
            Ok(c.query_row(sql, rusqlite::params_from_iter(p.iter()), |r| {
                r.get::<_, String>(0)
            })?)
        })
        .unwrap()
}

fn strings(env: &TestEnv, sql: &str, p: &[&str]) -> Vec<String> {
    env.core
        .project()
        .unwrap()
        .store
        .read(|c| {
            let mut stmt = c.prepare(sql)?;
            let rows = stmt
                .query_map(rusqlite::params_from_iter(p.iter()), |r| {
                    r.get::<_, String>(0)
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })
        .unwrap()
}

fn out_path(env: &TestEnv, name: &str) -> String {
    let dir = env.dir.path().join("out");
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name).to_string_lossy().into_owned()
}

struct Seed {
    draft: String,
    scenes: Vec<String>,
    act: String,
    card: String,
    managed_bytes: Vec<u8>,
    external_path: PathBuf,
}

/// A project with a screenplay (3 scenes), a Story Board act + card, a comment,
/// a private note, a managed file and a linked external file.
fn seed(env: &TestEnv) -> Seed {
    let created = env.ok("screenplay.create", json!({}));
    let draft = created["draft"]["id"].as_str().unwrap().to_string();
    let mut scenes = vec![];
    for (heading, action) in [
        (
            "EXT. STREET — DAY",
            "The rain has stopped. Puddles hold the neon of a closed pharmacy.",
        ),
        (
            "INT. POLICE STATION — NIGHT",
            "Arjun places the RED FOLDER on the desk.",
        ),
        (
            "EXT. OLD RAILWAY STATION — NIGHT",
            "A train that never comes.",
        ),
    ] {
        let sid = new_id();
        env.ok(
            "screenplay.apply_edits",
            json!({ "draftId": draft, "ops": [
                { "op": "insertScene", "id": sid, "heading": heading },
                { "op": "insertElement", "sceneId": sid, "elementType": "action", "text": action },
                { "op": "insertElement", "sceneId": sid, "elementType": "character", "text": "MEERA" },
                { "op": "insertElement", "sceneId": sid, "elementType": "dialogue", "text": "You said you would never come back." },
            ]}),
        );
        scenes.push(sid);
    }
    let act = env.ok("story.create_act", json!({ "title": "Act One" }))["id"]
        .as_str()
        .unwrap()
        .to_string();
    let card = env.ok(
        "story.create_card",
        json!({ "parent": { "parentType": "act", "parentId": act }, "shortDescription": "Arjun returns to the city" }),
    )["id"]
        .as_str()
        .unwrap()
        .to_string();
    env.ok(
        "comment.create",
        json!({ "targetType": "screenplay_scene", "targetId": scenes[1], "body": "Tighten the folder beat." }),
    );
    env.ok(
        "private_note.create",
        json!({ "targetType": "screenplay_scene", "targetId": scenes[1], "body": "SECRET-PLAN-XYZ kill the producer" }),
    );
    let managed_bytes = b"moodboard image bytes".to_vec();
    let managed = env.write_file("still.png", &managed_bytes);
    env.ok("files.add", json!({ "paths": [managed], "mode": "copy" }));
    let external = env.write_file("location-scout.pdf", b"%PDF-1.4 scout notes");
    env.ok("files.add", json!({ "paths": [external], "mode": "link" }));
    Seed {
        draft,
        scenes,
        act,
        card,
        managed_bytes,
        external_path: PathBuf::from(external),
    }
}

fn managed_file_bytes(env: &TestEnv) -> Vec<u8> {
    let s = env.core.project().unwrap();
    let rel = scalar_str(
        env,
        "SELECT rel_path FROM asset WHERE storage_mode='managed' AND original_name='still.png'",
        &[],
    );
    std::fs::read(s.layout.root().join(rel)).unwrap()
}

fn project_id(env: &TestEnv) -> String {
    env.core.project().unwrap().project_id()
}

fn read_zip_texts(path: &str) -> Vec<(String, Vec<u8>)> {
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("x");
    let files = archive::extract_all(Path::new(path), &out, ArchiveLimits::PACKAGE).unwrap();
    files
        .into_iter()
        .map(|f| {
            (
                f.to_string_lossy().replace('\\', "/"),
                std::fs::read(out.join(&f)).unwrap(),
            )
        })
        .collect()
}

/// Rewrite a package: `edit` may change entries; checksums are optionally recomputed.
fn rewrite_package(
    src: &str,
    dest: &str,
    recompute: bool,
    edit: impl Fn(&str, Vec<u8>) -> Vec<u8>,
) {
    let mut entries: Vec<(String, Vec<u8>)> = read_zip_texts(src)
        .into_iter()
        .map(|(n, b)| {
            let b = edit(&n, b);
            (n, b)
        })
        .collect();
    if recompute {
        let mut files = serde_json::Map::new();
        for (n, b) in &entries {
            if n != "checksums.json" {
                files.insert(n.clone(), json!(openframe_security::sha256_bytes(b)));
            }
        }
        let sums = serde_json::to_vec(&json!({ "algorithm": "sha256", "files": files })).unwrap();
        entries.retain(|(n, _)| n != "checksums.json");
        entries.push(("checksums.json".into(), sums));
    }
    archive::write_zip(
        Path::new(dest),
        entries
            .iter()
            .map(|(n, b)| (n.clone(), EntrySource::Bytes(b.as_slice()))),
    )
    .unwrap();
}

/// Minimal stored-ZIP writer that allows hostile names (zip-slip fixtures).
fn raw_zip(path: &Path, entries: &[(&str, &[u8])]) {
    fn crc32(data: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for &b in data {
            crc ^= b as u32;
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }
    let mut out: Vec<u8> = vec![];
    let mut central: Vec<u8> = vec![];
    for (name, data) in entries {
        let offset = out.len() as u32;
        let crc = crc32(data);
        let len = data.len() as u32;
        out.extend(0x0403_4b50u32.to_le_bytes());
        out.extend(20u16.to_le_bytes());
        out.extend([0u8; 8]); // flags, method (stored), time, date
        out.extend(crc.to_le_bytes());
        out.extend(len.to_le_bytes());
        out.extend(len.to_le_bytes());
        out.extend((name.len() as u16).to_le_bytes());
        out.extend(0u16.to_le_bytes());
        out.extend(name.as_bytes());
        out.extend(*data);
        central.extend(0x0201_4b50u32.to_le_bytes());
        central.extend(20u16.to_le_bytes());
        central.extend(20u16.to_le_bytes());
        central.extend([0u8; 8]);
        central.extend(crc.to_le_bytes());
        central.extend(len.to_le_bytes());
        central.extend(len.to_le_bytes());
        central.extend((name.len() as u16).to_le_bytes());
        central.extend([0u8; 8]); // extra, comment, disk, internal attrs
        central.extend(0u32.to_le_bytes()); // external attrs
        central.extend(offset.to_le_bytes());
        central.extend(name.as_bytes());
    }
    let cd_offset = out.len() as u32;
    let cd_size = central.len() as u32;
    out.extend(central);
    out.extend(0x0605_4b50u32.to_le_bytes());
    out.extend([0u8; 4]);
    out.extend((entries.len() as u16).to_le_bytes());
    out.extend((entries.len() as u16).to_le_bytes());
    out.extend(cd_size.to_le_bytes());
    out.extend(cd_offset.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    std::fs::write(path, out).unwrap();
}

fn export_script_review(env: &TestEnv, draft: &str, name: &str) -> String {
    let path = out_path(env, name);
    let r = env.ok(
        "packages.export_exchange",
        json!({ "packageType": "scriptReview", "draftId": draft, "scope": "comments", "path": path }),
    );
    r["path"].as_str().unwrap().to_string()
}

// ================================================================== backup

#[test]
fn fsd_44_backup_round_trip_restores_as_copy_with_new_identity() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let seed = seed(&env);
    let original_id = project_id(&env);
    let original_path = env.project_path();

    let preview = env.ok("packages.backup_preview", json!({}));
    assert_eq!(preview["includedFiles"], 1);
    assert_eq!(preview["external"][0]["status"], "External");
    assert!(
        preview["baseName"]
            .as_str()
            .unwrap()
            .starts_with("Black Rain ")
    );

    let dest = out_path(&env, "Black Rain 2026-09-29 1042 before-rewrite.ofbackup");
    let started = env.ok(
        "packages.create_backup",
        json!({ "path": dest, "label": "before-rewrite" }),
    );
    let report = task_ok(&env, &started);
    assert!(Path::new(&dest).is_file());
    assert_eq!(report["includedFiles"], 1);
    // Never claims a linked file is backed up when it was not copied.
    assert_eq!(report["external"][0]["status"], "External");
    assert_eq!(
        env.core.save.snapshot().pending_operations,
        0,
        "pending package operation cleared"
    );
    let names: Vec<String> = read_zip_texts(&dest).into_iter().map(|(n, _)| n).collect();
    assert!(names.contains(&"manifest.json".to_string()));
    assert!(names.contains(&"checksums.json".to_string()));
    assert!(names.contains(&"project/project.sqlite".to_string()));
    let manifest: Value = serde_json::from_slice(
        &read_zip_texts(&dest)
            .into_iter()
            .find(|(n, _)| n == "manifest.json")
            .unwrap()
            .1,
    )
    .unwrap();
    assert_eq!(manifest["packageType"], "backup");
    assert_eq!(manifest["label"], "before-rewrite");
    assert_eq!(manifest["sourceProjectId"], original_id);

    // Restore as a copy: new identity, same content.
    let inspection = env.ok("packages.inspect_package", json!({ "path": dest }));
    assert_eq!(inspection["collision"]["isOpen"], true);
    assert_eq!(inspection["defaultMode"], "copy");
    let started = env.ok(
        "packages.import_project",
        json!({ "path": dest, "mode": "copy" }),
    );
    let rep = task_ok(&env, &started);
    assert_eq!(rep["identityKept"], false);
    let new_id = project_id(&env);
    assert_ne!(new_id, original_id);
    assert_eq!(rep["projectId"], new_id);
    assert!(rep["title"].as_str().unwrap().ends_with("(copy)"));
    assert!(
        rep["message"]
            .as_str()
            .unwrap()
            .contains("imported as a copy")
    );
    assert_ne!(env.project_path(), original_path);
    // Content equal.
    assert_eq!(
        scalar_str(
            &env,
            "SELECT title FROM story_act WHERE id=?1",
            &[&seed.act]
        ),
        "Act One"
    );
    assert_eq!(
        scalar_str(
            &env,
            "SELECT short_description FROM story_scene_card WHERE id=?1",
            &[&seed.card]
        ),
        "Arjun returns to the city"
    );
    assert_eq!(
        scalar_i64(
            &env,
            "SELECT count(*) FROM screenplay_scene WHERE draft_id=?1 AND deleted_at IS NULL",
            &[&seed.draft]
        ),
        scalar_i64(
            &env,
            "SELECT count(*) FROM screenplay_scene WHERE draft_id=?1",
            &[&seed.draft]
        )
    );
    assert_eq!(
        scalar_str(
            &env,
            "SELECT heading FROM screenplay_scene WHERE id=?1",
            &[&seed.scenes[1]]
        ),
        "INT. POLICE STATION — NIGHT"
    );
    assert_eq!(scalar_i64(&env, "SELECT count(*) FROM comment", &[]), 1);
    // Backups are private transfers: my private note is kept.
    let notes = env.ok("private_note.list", json!({}));
    assert_eq!(notes.as_array().unwrap().len(), 1);
    assert_eq!(managed_file_bytes(&env), seed.managed_bytes);
    // The original project is untouched and still opens with its own identity.
    env.reopen_project(&original_path);
    assert_eq!(project_id(&env), original_id);
}

#[test]
fn fsd_44_7_portable_backup_copies_linked_files_into_the_project() {
    let env = TestEnv::with_project("Portable", "Short Film");
    let seed = seed(&env);
    let dest = out_path(&env, "portable.ofbackup");
    let started = env.ok(
        "packages.create_backup",
        json!({ "path": dest, "includeExternal": true }),
    );
    let report = task_ok(&env, &started);
    assert_eq!(report["copiedExternal"], 1);
    assert_eq!(report["external"][0]["status"], "Copied");
    // Even if the linked original disappears, the restored copy has the file.
    std::fs::remove_file(&seed.external_path).unwrap();
    let started = env.ok(
        "packages.import_project",
        json!({ "path": dest, "mode": "copy" }),
    );
    let rep = task_ok(&env, &started);
    assert_eq!(rep["copiedExternal"], 1);
    assert_eq!(rep["missingExternal"].as_array().unwrap().len(), 0);
    assert_eq!(
        scalar_str(
            &env,
            "SELECT storage_mode FROM asset WHERE original_name='location-scout.pdf'",
            &[]
        ),
        "managed"
    );
}

#[test]
fn fsd_44_interrupted_backup_leaves_no_partial_file() {
    let env = TestEnv::with_project("Cancel Me", "Feature Film");
    seed(&env);
    let session = env.core.project().unwrap();
    let dir = env.dir.path().join("cancel-out");
    std::fs::create_dir_all(&dir).unwrap();
    let dest = dir.join("x.ofbackup");
    let user = PackageUser {
        user_id: env.actor().user_id,
        display_name: "Nisha".into(),
    };
    // Cancel at every checkpoint in turn until the build completes.
    let mut completed = false;
    for stop_at in 0..200 {
        let calls = std::cell::Cell::new(0usize);
        let src = ProjectPackageSource {
            layout: session.layout.clone(),
            manifest: session.manifest.lock().clone(),
            db: DbSource::Store(&session.store),
        };
        let opts = ProjectPackageOptions {
            kind: PackageType::Backup,
            dest: dest.clone(),
            label: None,
            include_external: true,
            user: user.clone(),
            app_version: "test".into(),
        };
        let out = build_project_package(&src, &opts, &|_, _| {}, &|| {
            calls.set(calls.get() + 1);
            calls.get() > stop_at
        })
        .unwrap();
        let leftovers: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        match out {
            None => assert!(
                leftovers.is_empty(),
                "cancel at {stop_at} left {leftovers:?}"
            ),
            Some(_) => {
                assert_eq!(leftovers, vec!["x.ofbackup".to_string()]);
                completed = true;
                break;
            }
        }
        let staging: Vec<_> = std::fs::read_dir(session.layout.cache())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("package-staging"))
            .collect();
        assert!(staging.is_empty(), "staging folder left behind");
    }
    assert!(completed);

    // Through the task API: cancelling right away leaves no file (or it completed first).
    let dest2 = out_path(&env, "cancelled.ofbackup");
    let started = env.ok("packages.create_backup", json!({ "path": dest2 }));
    env.ok("app.cancel_task", json!({ "taskId": started["taskId"] }));
    let (state, _) = wait_task(&env, started["taskId"].as_str().unwrap());
    if state == "cancelled" {
        assert!(!Path::new(&dest2).exists());
    } else {
        assert_eq!(state, "completed");
    }
    let names: Vec<String> = std::fs::read_dir(Path::new(&dest2).parent().unwrap())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        !names.iter().any(|n| n.ends_with(".tmp")
            || n.ends_with(".partial")
            || n.ends_with(".openframe-tmp")),
        "{names:?}"
    );
    // Background task end always clears the pending-package save state.
    let start = Instant::now();
    while env.core.save.snapshot().pending_operations != 0 {
        assert!(start.elapsed() < Duration::from_secs(10));
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn backup_permissions_and_destination_validation() {
    let env = TestEnv::with_project("Perms", "Feature Film");
    let viewer = env.actor_with_role(Role::Viewer);
    let err = env
        .call_as(
            &viewer,
            "packages.create_backup",
            json!({ "path": out_path(&env, "v.ofbackup") }),
        )
        .unwrap_err();
    assert_eq!(err.code.0, "permission.denied");
    // Export-only users may create packages (Security §6).
    let export_only = env.actor_with_role(Role::ExportOnly);
    let started = env
        .call_as(
            &export_only,
            "packages.create_backup",
            json!({ "path": out_path(&env, "e") }),
        )
        .unwrap();
    let rep = task_ok(&env, &started);
    assert!(
        rep["path"].as_str().unwrap().ends_with("e.ofbackup"),
        "extension added"
    );
    assert_eq!(
        env.err(
            "packages.create_backup",
            json!({ "path": "relative.ofbackup" })
        ),
        "validation.invalid_input"
    );
    assert_eq!(
        env.err(
            "packages.create_backup",
            json!({ "path": env.dir.path().join("nope").join("x.ofbackup").to_string_lossy() })
        ),
        "not_found.folder"
    );
}

// ====================================================== full project package

#[test]
fn fsd_45_project_package_round_trip_across_installations() {
    let a = TestEnv::with_project("Black Rain", "Feature Film");
    let seed = seed(&a);
    let a_id = project_id(&a);
    let pkg = out_path(&a, "Black Rain.ofproject");
    let started = a.ok("packages.export_project", json!({ "path": pkg }));
    task_ok(&a, &started);
    let manifest: Value = serde_json::from_slice(
        &read_zip_texts(&pkg)
            .into_iter()
            .find(|(n, _)| n == "manifest.json")
            .unwrap()
            .1,
    )
    .unwrap();
    assert_eq!(manifest["packageType"], "project");
    assert_eq!(
        manifest["privateNotesIncluded"], true,
        "private project transfer"
    );
    // The linked file is not on machine B.
    std::fs::remove_file(&seed.external_path).unwrap();

    let b = TestEnv::new();
    let inspection = b.ok("packages.inspect_package", json!({ "path": pkg }));
    assert!(inspection["collision"].is_null());
    assert_eq!(inspection["defaultMode"], "open");
    assert_eq!(inspection["external"][0]["status"], "Missing");
    let started = b.ok(
        "packages.import_project",
        json!({ "path": pkg, "mode": "open" }),
    );
    let rep = task_ok(&b, &started);
    assert_eq!(rep["identityKept"], true);
    assert_eq!(
        project_id(&b),
        a_id,
        "same portable project keeps its identity"
    );
    assert_eq!(rep["missingExternal"].as_array().unwrap().len(), 1);
    assert!(
        rep["message"]
            .as_str()
            .unwrap()
            .contains("missing on this PC (the project still opens)")
    );
    assert_eq!(
        scalar_str(&b, "SELECT title FROM story_act WHERE id=?1", &[&seed.act]),
        "Act One"
    );
    assert_eq!(managed_file_bytes(&b), seed.managed_bytes);
    assert!(scalar_i64(&b, "SELECT count(*) FROM screenplay_element", &[]) > 0);
    // It is a normal project on machine B: listed, editable, survives restart.
    b.ok("story.create_act", json!({ "title": "Machine B act" }));
    let path = b.project_path();
    let b = b.restart();
    b.reopen_project(&path);
    assert_eq!(
        scalar_i64(
            &b,
            "SELECT count(*) FROM story_act WHERE title='Machine B act'",
            &[]
        ),
        1
    );
}

#[test]
fn fsd_45_4_identity_collision_open_as_copy_or_replace_after_backup() {
    let env = TestEnv::with_project("Collide", "Feature Film");
    seed(&env);
    let id = project_id(&env);
    let original_path = env.project_path();
    let pkg = out_path(&env, "collide.ofproject");
    task_ok(
        &env,
        &env.ok("packages.export_project", json!({ "path": pkg })),
    );
    env.ok(
        "story.create_act",
        json!({ "title": "Written after export" }),
    );

    // Opening with the same identity is refused while the project exists here.
    let (state, err) = wait_task(
        &env,
        env.ok(
            "packages.import_project",
            json!({ "path": pkg, "mode": "open" }),
        )["taskId"]
            .as_str()
            .unwrap(),
    );
    assert_eq!(state, "failed");
    assert_eq!(err["code"], "conflict.state");

    // Open as Copy (default): original untouched.
    let rep = task_ok(
        &env,
        &env.ok(
            "packages.import_project",
            json!({ "path": pkg, "mode": "copy" }),
        ),
    );
    assert_ne!(rep["projectId"], id);
    assert_eq!(
        scalar_i64(
            &env,
            "SELECT count(*) FROM story_act WHERE title='Written after export'",
            &[]
        ),
        0
    );
    env.reopen_project(&original_path);
    assert_eq!(project_id(&env), id);
    assert_eq!(
        scalar_i64(
            &env,
            "SELECT count(*) FROM story_act WHERE title='Written after export'",
            &[]
        ),
        1
    );

    // Replace Existing After Backup: a safety backup first, same identity, package content.
    let rep = task_ok(
        &env,
        &env.ok(
            "packages.import_project",
            json!({ "path": pkg, "mode": "replace" }),
        ),
    );
    let backup = rep["safetyBackup"].as_str().unwrap().to_string();
    assert!(Path::new(&backup).is_file(), "safety backup written");
    assert_eq!(project_id(&env), id);
    assert_eq!(env.project_path(), original_path);
    assert_eq!(
        scalar_i64(
            &env,
            "SELECT count(*) FROM story_act WHERE title='Written after export'",
            &[]
        ),
        0
    );
    // The safety backup holds the replaced state and restores it as a copy.
    let rep = task_ok(
        &env,
        &env.ok(
            "packages.import_project",
            json!({ "path": backup, "mode": "copy" }),
        ),
    );
    assert_ne!(rep["projectId"], id);
    assert_eq!(
        scalar_i64(
            &env,
            "SELECT count(*) FROM story_act WHERE title='Written after export'",
            &[]
        ),
        1
    );
    // Replace without an existing project is refused.
    let other = TestEnv::new();
    let (state, err) = wait_task(
        &other,
        other.ok(
            "packages.import_project",
            json!({ "path": pkg, "mode": "replace" }),
        )["taskId"]
            .as_str()
            .unwrap(),
    );
    assert_eq!(state, "failed");
    assert_eq!(err["code"], "conflict.state");
}

// ============================================================ validation

#[test]
fn tampered_checksum_is_rejected_with_zero_mutation() {
    let env = TestEnv::with_project("Tamper", "Feature Film");
    let seed = seed(&env);
    let pkg = export_script_review(&env, &seed.draft, "review.ofscriptreview");
    let bad = out_path(&env, "tampered.ofscriptreview");
    rewrite_package(&pkg, &bad, false, |name, bytes| {
        if name == "content.json" {
            String::from_utf8(bytes)
                .unwrap()
                .replace("Tighten", "Delete")
                .into_bytes()
        } else {
            bytes
        }
    });
    let activity_before = scalar_i64(&env, "SELECT count(*) FROM sys_activity", &[]);
    assert_eq!(
        env.err("packages.open_exchange", json!({ "path": bad })),
        "import.checksum_mismatch"
    );
    assert_eq!(
        env.err("packages.review_open", json!({ "path": bad })),
        "import.checksum_mismatch"
    );
    assert_eq!(
        scalar_i64(&env, "SELECT count(*) FROM sys_import_session", &[]),
        0
    );
    assert_eq!(
        scalar_i64(&env, "SELECT count(*) FROM sys_activity", &[]),
        activity_before
    );

    // Same for a project package: nothing is created.
    let proj = out_path(&env, "p.ofproject");
    task_ok(
        &env,
        &env.ok("packages.export_project", json!({ "path": proj })),
    );
    let bad_proj = out_path(&env, "p-bad.ofproject");
    rewrite_package(&proj, &bad_proj, false, |name, bytes| {
        if name.starts_with("project/assets/") {
            b"swapped".to_vec()
        } else {
            bytes
        }
    });
    let other = TestEnv::new();
    let (state, err) = wait_task(
        &other,
        other.ok(
            "packages.import_project",
            json!({ "path": bad_proj, "mode": "open" }),
        )["taskId"]
            .as_str()
            .unwrap(),
    );
    assert_eq!(state, "failed");
    assert_eq!(err["code"], "import.checksum_mismatch");
    assert!(other.core.project_opt().is_none());
    let projects = other.core.config.projects_dir.clone();
    let left: Vec<_> = std::fs::read_dir(&projects)
        .map(|r| r.filter_map(|e| e.ok()).collect())
        .unwrap_or_default();
    assert!(left.is_empty(), "no project or staging folder left behind");
    // A missing entry is "incomplete".
    let partial = out_path(&env, "partial.ofscriptreview");
    let entries: Vec<(String, Vec<u8>)> = read_zip_texts(&pkg)
        .into_iter()
        .filter(|(n, _)| n != "content.json")
        .collect();
    archive::write_zip(
        Path::new(&partial),
        entries
            .iter()
            .map(|(n, b)| (n.clone(), EntrySource::Bytes(b.as_slice()))),
    )
    .unwrap();
    assert_eq!(
        env.err("packages.open_exchange", json!({ "path": partial })),
        "import.package_incomplete"
    );
}

#[test]
fn zip_slip_package_is_rejected_and_nothing_is_written() {
    let env = TestEnv::with_project("Slip", "Feature Film");
    let dir = env.dir.path().join("slip");
    std::fs::create_dir_all(&dir).unwrap();
    let evil = dir.join("evil.ofstory");
    let manifest = json!({
        "format": "openframe-package", "packageType": "story", "formatVersion": 1,
        "packageId": new_id(), "sourceProjectId": new_id(), "sourceProjectTitle": "X",
        "exportedAt": 1, "appVersion": "1", "scope": { "kind": "full", "label": "x" },
        "commentsIncluded": false, "attachmentsIncluded": false, "privateNotesIncluded": false,
        "originatingUser": { "userId": new_id(), "displayName": "Mallory" }
    });
    let m = serde_json::to_vec(&manifest).unwrap();
    raw_zip(
        &evil,
        &[
            ("manifest.json", &m),
            ("../../escaped.txt", b"pwned"),
            ("content.json", b"{}"),
        ],
    );
    assert_eq!(
        env.err(
            "packages.open_exchange",
            json!({ "path": evil.to_string_lossy() })
        ),
        "import.unsafe_archive"
    );
    assert_eq!(
        env.err(
            "packages.inspect_package",
            json!({ "path": evil.to_string_lossy() })
        ),
        "import.unsafe_archive"
    );
    assert!(!env.dir.path().join("escaped.txt").exists());
    assert!(!dir.parent().unwrap().join("escaped.txt").exists());
    // A file that isn't a package at all.
    let junk = dir.join("junk.ofstory");
    std::fs::write(&junk, b"hello").unwrap();
    assert_eq!(
        env.err(
            "packages.open_exchange",
            json!({ "path": junk.to_string_lossy() })
        ),
        "import.unsafe_archive"
    );
}

#[test]
fn too_new_package_format_is_refused_clearly() {
    let env = TestEnv::with_project("Future", "Feature Film");
    let seed = seed(&env);
    let pkg = export_script_review(&env, &seed.draft, "future.ofscriptreview");
    let newer = out_path(&env, "newer.ofscriptreview");
    rewrite_package(&pkg, &newer, true, |name, bytes| {
        if name == "manifest.json" {
            let mut v: Value = serde_json::from_slice(&bytes).unwrap();
            v["formatVersion"] = json!(99);
            v["packageType"] = json!("hologram");
            serde_json::to_vec(&v).unwrap()
        } else {
            bytes
        }
    });
    let err = env
        .call("packages.inspect_package", json!({ "path": newer }))
        .unwrap_err();
    assert_eq!(err.code.0, "import.package_too_new");
    assert!(err.message.contains("newer version of OpenFrame"));
    assert_eq!(
        env.err("packages.open_exchange", json!({ "path": newer })),
        "import.package_too_new"
    );
    let other = TestEnv::new();
    assert_eq!(
        other.err(
            "packages.import_project",
            json!({ "path": newer, "mode": "open" })
        ),
        "import.package_too_new"
    );
}

// ========================================================== exchange export

#[test]
fn fsd_46_7_private_notes_are_never_in_exchange_packages() {
    let env = TestEnv::with_project("Private", "Feature Film");
    let seed = seed(&env);
    env.ok(
        "private_note.create",
        json!({ "targetType": "story_scene_card", "targetId": seed.card, "body": "SECRET-PLAN-XYZ on the card" }),
    );
    let preview = env.ok(
        "packages.exchange_preview",
        json!({ "packageType": "scriptReview", "draftId": seed.draft, "scope": "comments" }),
    );
    assert_eq!(preview["privateNotesExcluded"], true);
    assert_eq!(preview["commentsIncluded"], true);
    assert_eq!(
        env.err(
            "packages.export_exchange",
            json!({ "packageType": "scriptReview", "includePrivateNotes": true, "path": out_path(&env, "x") })
        ),
        "validation.includePrivateNotes"
    );
    for (ty, name) in [("scriptReview", "a.ofscriptreview"), ("story", "b.ofstory")] {
        let path = out_path(&env, name);
        env.ok(
            "packages.export_exchange",
            json!({ "packageType": ty, "includeComments": true, "includeAttachments": true, "path": path }),
        );
        for (entry, bytes) in read_zip_texts(&path) {
            let text = String::from_utf8_lossy(&bytes);
            assert!(
                !text.contains("SECRET-PLAN-XYZ"),
                "{ty}: private note leaked into {entry}"
            );
            if entry == "manifest.json" {
                let m: Value = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(m["privateNotesIncluded"], false);
            }
        }
    }
    // Viewers can't create packages; the comment is included when selected.
    let viewer = env.actor_with_role(Role::Viewer);
    let err = env
        .call_as(
            &viewer,
            "packages.export_exchange",
            json!({ "packageType": "story", "path": out_path(&env, "v.ofstory") }),
        )
        .unwrap_err();
    assert_eq!(err.code.0, "permission.denied");
}

#[test]
fn fsd_col_001_exchange_package_can_be_inspected_in_another_installation() {
    let env = TestEnv::with_project("Inspect", "Feature Film");
    let seed = seed(&env);
    let pkg = export_script_review(&env, &seed.draft, "inspect.ofscriptreview");
    let log = env.ok("packages.log", json!({}));
    assert_eq!(log[0]["direction"], "export");
    assert!(
        log[0]["summary"]
            .as_str()
            .unwrap()
            .contains("private notes excluded")
    );
    let other = TestEnv::new();
    let info = other.ok("packages.inspect_package", json!({ "path": pkg }));
    assert_eq!(info["manifest"]["packageType"], "scriptReview");
    assert_eq!(info["manifest"]["sourceProjectTitle"], "Inspect");
    assert_eq!(info["manifest"]["commentsIncluded"], true);
    let counts = info["contentSummary"].as_array().unwrap();
    assert!(
        counts
            .iter()
            .any(|c| c["label"] == "Comments" && c["count"] == 1)
    );
    // The reviewer opens it read-only without any project.
    let ws = other.ok("packages.review_open", json!({ "path": pkg }));
    assert_eq!(ws["canComment"], true);
    assert!(ws["scenes"].as_array().unwrap().len() >= 3);
    assert_eq!(ws["comments"][0]["body"], "Tighten the folder beat.");
}

// ====================================================== review round trip

struct RoundTrip {
    env: TestEnv,
    seed: Seed,
    review: String,
}

fn reviewer_response(
    rt: &RoundTrip,
    comments: &[(&str, &str, Option<&str>)],
    name: &str,
) -> String {
    let reviewer = TestEnv::new();
    reviewer.core.set_display_name("Priya").unwrap();
    let ws = reviewer.ok("packages.review_open", json!({ "path": rt.review }));
    let pid = ws["key"].as_str().unwrap().to_string();
    for (scene, body, quote) in comments {
        reviewer.ok(
            "packages.review_comment",
            json!({ "packageId": pid, "sceneId": scene, "body": body, "quotedText": quote }),
        );
    }
    let dest = reviewer.dir.path().join(name);
    let r = reviewer.ok(
        "packages.review_export_response",
        json!({ "packageId": pid, "path": dest.to_string_lossy() }),
    );
    // Keep the file after the reviewer's environment is dropped.
    let keep = out_path(&rt.env, name);
    std::fs::copy(r["path"].as_str().unwrap(), &keep).unwrap();
    keep
}

fn round_trip(title: &str) -> RoundTrip {
    let env = TestEnv::with_project(title, "Feature Film");
    let seed = seed(&env);
    let review = export_script_review(&env, &seed.draft, "draft.ofscriptreview");
    RoundTrip { env, seed, review }
}

#[test]
fn fsd_47_7_response_comments_attach_to_matching_scenes() {
    let rt = round_trip("Round Trip");
    let s2 = rt.seed.scenes[1].clone();
    let s3 = rt.seed.scenes[2].clone();
    let resp = reviewer_response(
        &rt,
        &[
            (&s2, "Should the folder be red?", Some("RED FOLDER")),
            (&s3, "Love this ending.", None),
        ],
        "priya.ofresponse",
    );
    let env = &rt.env;
    let session = env.ok("packages.open_exchange", json!({ "path": resp }));
    assert_eq!(session["packageType"], "response");
    assert_eq!(session["compatibility"], "Valid");
    assert_eq!(session["sameProject"], true);
    assert_eq!(session["exportedBy"], "Priya");
    assert_eq!(session["status"], "Previewed");
    let safe: Vec<&Value> = session["changes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["group"] == "comments" && c["state"] == "safe")
        .collect();
    assert_eq!(safe.len(), 2);
    // Preview changed nothing (validation before mutation).
    assert_eq!(scalar_i64(env, "SELECT count(*) FROM comment", &[]), 1);

    let done = env.ok(
        "packages.apply_import",
        json!({ "sessionId": session["id"], "mode": "allSafe" }),
    );
    assert_eq!(done["status"], "Applied", "{}", done["result"]);
    assert!(
        done["result"]["message"]
            .as_str()
            .unwrap()
            .starts_with("Imported: 2 comments were applied")
    );
    // Anchored to the quoted text when it occurs once; author is the reviewer.
    let el_target = scalar_str(
        env,
        "SELECT target_type FROM comment WHERE body='Should the folder be red?'",
        &[],
    );
    assert_eq!(el_target, "screenplay_element");
    assert_eq!(
        scalar_str(
            env,
            "SELECT scene_id FROM comment WHERE body='Should the folder be red?'",
            &[]
        ),
        s2
    );
    assert_eq!(
        scalar_str(
            env,
            "SELECT quoted_text FROM comment WHERE body='Should the folder be red?'",
            &[]
        ),
        "RED FOLDER"
    );
    assert_eq!(
        scalar_str(
            env,
            "SELECT target_id FROM comment WHERE body='Love this ending.'",
            &[]
        ),
        s3
    );
    assert_eq!(
        scalar_str(
            env,
            "SELECT author_name FROM comment WHERE body='Love this ending.'",
            &[]
        ),
        "Priya"
    );
    // Opening the same response again doesn't duplicate anything.
    let again = env.ok("packages.open_exchange", json!({ "path": resp }));
    assert!(
        again["changes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["group"] != "comments")
    );
    // The whole import is undoable as one action (IEX-031).
    let undone = env.ok(
        "packages.undo_import",
        json!({ "sessionId": session["id"] }),
    );
    assert_eq!(undone["status"], "Undone");
    assert_eq!(
        scalar_i64(
            env,
            "SELECT count(*) FROM comment WHERE deleted_at IS NULL",
            &[]
        ),
        1
    );
    let sessions = env.ok("packages.sessions", json!({}));
    assert!(sessions.as_array().unwrap().len() >= 2);
}

#[test]
fn fsd_col_002_stale_response_is_imported_as_comments_only_without_changing_text() {
    let rt = round_trip("Stale");
    let s2 = rt.seed.scenes[1].clone();
    let resp = reviewer_response(
        &rt,
        &[(&s2, "Cut this exchange?", None)],
        "stale.ofresponse",
    );
    let env = &rt.env;
    // The author moved on: Draft 2 is current and scene 2 was edited.
    let d2 = env.ok(
        "screenplay.new_draft",
        json!({ "sourceDraftId": rt.seed.draft, "name": "Draft 2" }),
    );
    let d2 = d2["id"].as_str().unwrap().to_string();
    let lineage = scalar_str(
        env,
        "SELECT lineage_id FROM screenplay_scene WHERE id=?1",
        &[&s2],
    );
    let d2_scene = scalar_str(
        env,
        "SELECT id FROM screenplay_scene WHERE draft_id=?1 AND lineage_id=?2",
        &[&d2, &lineage],
    );
    let texts_before = strings(
        env,
        "SELECT id || ':' || text FROM screenplay_element ORDER BY id",
        &[],
    );
    let headings_before = strings(
        env,
        "SELECT id || ':' || heading FROM screenplay_scene ORDER BY id",
        &[],
    );

    let session = env.ok("packages.open_exchange", json!({ "path": resp }));
    assert_eq!(session["compatibility"], "Stale");
    assert_eq!(session["stale"], true);
    assert_eq!(
        session["staleTitle"],
        "This review was created from Draft 1. Your current project is Draft 2."
    );
    assert_eq!(session["sourceVersion"], "Draft 1");
    assert_eq!(session["hostVersion"], "Draft 2");
    let applied = env.ok(
        "packages.apply_import",
        json!({ "sessionId": session["id"], "mode": "commentsOnly" }),
    );
    assert!(matches!(
        applied["status"].as_str().unwrap(),
        "Applied" | "PartiallyApplied"
    ));
    // The comment attaches to the matching scene of the current draft …
    assert_eq!(
        scalar_str(
            env,
            "SELECT target_id FROM comment WHERE body='Cut this exchange?'",
            &[]
        ),
        d2_scene
    );
    // … and no screenplay text changed in any draft.
    assert_eq!(
        strings(
            env,
            "SELECT id || ':' || text FROM screenplay_element ORDER BY id",
            &[]
        ),
        texts_before
    );
    assert_eq!(
        strings(
            env,
            "SELECT id || ':' || heading FROM screenplay_scene ORDER BY id",
            &[]
        ),
        headings_before
    );

    // A stale package can instead be kept as a separate review record.
    let resp2 = reviewer_response(&rt, &[(&s2, "Another pass", None)], "record.ofresponse");
    let s2session = env.ok("packages.open_exchange", json!({ "path": resp2 }));
    let rec = env.ok(
        "packages.apply_import",
        json!({ "sessionId": s2session["id"], "mode": "record" }),
    );
    assert_eq!(rec["status"], "PendingReview");
    let records = env.ok("packages.records", json!({}));
    assert_eq!(records.as_array().unwrap().len(), 1);
    let view = env.ok(
        "packages.record_view",
        json!({ "recordId": records[0]["id"] }),
    );
    assert_eq!(view["comments"][0]["body"], "Another pass");
    assert_eq!(
        scalar_i64(
            env,
            "SELECT count(*) FROM comment WHERE body='Another pass'",
            &[]
        ),
        0
    );
    // "View the review without importing".
    let ws = env.ok(
        "packages.session_view",
        json!({ "sessionId": s2session["id"] }),
    );
    assert_eq!(ws["canComment"], false);
}

#[test]
fn fsd_col_015_016_ambiguous_and_unmatched_notes_go_to_the_review_queue() {
    let rt = round_trip("Queue");
    let s1 = rt.seed.scenes[0].clone();
    // Both notes are on a scene the author then deletes.
    let resp = reviewer_response(
        &rt,
        &[
            (&s1, "Should the neon flicker?", Some("MEERA")),
            (&s1, "Is this puddle real?", Some("Puddles hold the neon")),
        ],
        "queue.ofresponse",
    );
    let env = &rt.env;
    env.ok("screenplay.delete_scene", json!({ "sceneId": s1 }));
    let session = env.ok("packages.open_exchange", json!({ "path": resp }));
    let states: Vec<String> = session["changes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["kind"] == "comment")
        .map(|c| c["state"].as_str().unwrap().to_string())
        .collect();
    assert!(states.contains(&"ambiguous".to_string()), "{states:?}"); // "MEERA" is in several scenes
    assert!(states.contains(&"unmapped".to_string()), "{states:?}"); // the puddle line is gone
    let applied = env.ok(
        "packages.apply_import",
        json!({ "sessionId": session["id"], "mode": "allSafe" }),
    );
    assert!(
        applied["result"]["message"]
            .as_str()
            .unwrap()
            .contains("Review Queue; nothing was discarded")
    );
    let queue = env.ok("packages.review_queue", json!({}));
    let items = queue.as_array().unwrap();
    assert_eq!(items.len(), 2);
    let ambiguous = items.iter().find(|i| i["kind"] == "ambiguous").unwrap();
    assert!(ambiguous["candidates"].as_array().unwrap().len() >= 2);
    let unmatched = items.iter().find(|i| i["kind"] == "unmapped").unwrap();
    assert_eq!(unmatched["authorName"], "Priya");
    // Attach manually.
    let target = rt.seed.scenes[1].clone();
    let attached = env.ok(
        "packages.queue_attach",
        json!({ "itemId": unmatched["id"], "sceneId": target }),
    );
    assert_eq!(attached["status"], "Attached");
    assert!(
        attached["attachedLabel"]
            .as_str()
            .unwrap()
            .starts_with("Scene")
    );
    assert_eq!(
        scalar_str(
            env,
            "SELECT author_name FROM comment WHERE body LIKE '%Is this puddle real?%'",
            &[]
        ),
        "Priya"
    );
    assert_eq!(
        env.err(
            "packages.queue_attach",
            json!({ "itemId": unmatched["id"], "sceneId": target })
        ),
        "conflict.state"
    );
}

#[test]
fn fsd_121_cancel_leaves_the_project_unchanged_and_permissions_apply() {
    let rt = round_trip("Cancel");
    let s2 = rt.seed.scenes[1].clone();
    let resp = reviewer_response(&rt, &[(&s2, "Hmm", None)], "cancel.ofresponse");
    let env = &rt.env;
    let before = scalar_i64(env, "SELECT count(*) FROM comment", &[]);
    let session = env.ok("packages.open_exchange", json!({ "path": resp }));
    let viewer = env.actor_with_role(Role::Viewer);
    let err = env
        .call_as(
            &viewer,
            "packages.apply_import",
            json!({ "sessionId": session["id"], "mode": "allSafe" }),
        )
        .unwrap_err();
    assert_eq!(err.code.0, "permission.denied");
    let cancelled = env.ok(
        "packages.cancel_import",
        json!({ "sessionId": session["id"] }),
    );
    assert_eq!(cancelled["status"], "Cancelled");
    assert_eq!(scalar_i64(env, "SELECT count(*) FROM comment", &[]), before);
    assert_eq!(
        env.err(
            "packages.apply_import",
            json!({ "sessionId": session["id"], "mode": "allSafe" })
        ),
        "conflict.state"
    );
    // A Commenter can import comments (but not object changes).
    let session = env.ok("packages.open_exchange", json!({ "path": resp }));
    let commenter = env.actor_with_role(Role::Commenter);
    let done = env
        .call_as(
            &commenter,
            "packages.apply_import",
            json!({ "sessionId": session["id"], "mode": "commentsOnly", "createBackup": true }),
        )
        .unwrap();
    assert_eq!(done["status"], "Applied");
    assert!(
        Path::new(done["result"]["safetyBackup"].as_str().unwrap()).is_file(),
        "Create Backup Before Import"
    );
}

// ===================================================== other package types

#[test]
fn fsd_50_story_board_package_copy_as_new_and_same_project_changes() {
    let a = TestEnv::with_project("Story Source", "Feature Film");
    let seed = seed(&a);
    let seq = a.ok(
        "story.create_sequence",
        json!({ "actId": seed.act, "title": "The Chase" }),
    )["id"]
        .as_str()
        .unwrap()
        .to_string();
    a.ok(
        "story.create_card",
        json!({ "parent": { "parentType": "sequence", "parentId": seq }, "shortDescription": "Rooftop run" }),
    );
    a.ok(
        "comment.create",
        json!({ "targetType": "story_scene_card", "targetId": seed.card, "body": "Great opener" }),
    );
    let pkg = out_path(&a, "board.ofstory");
    a.ok(
        "packages.export_exchange",
        json!({ "packageType": "story", "includeComments": true, "path": pkg }),
    );

    // Another project: only a copy with new identities is possible.
    let b = TestEnv::with_project("Other", "Short Film");
    let session = b.ok("packages.open_exchange", json!({ "path": pkg }));
    assert_eq!(session["canCopyAsNew"], true);
    assert_eq!(session["sameProject"], false);
    let done = b.ok(
        "packages.apply_import",
        json!({ "sessionId": session["id"], "mode": "copyAsNew" }),
    );
    assert!(
        matches!(
            done["status"].as_str().unwrap(),
            "Applied" | "PartiallyApplied"
        ),
        "{}",
        done["result"]
    );
    assert_eq!(
        scalar_i64(
            &b,
            "SELECT count(*) FROM story_act WHERE title='Act One'",
            &[]
        ),
        1
    );
    assert_eq!(
        scalar_i64(
            &b,
            "SELECT count(*) FROM story_scene_card WHERE id=?1",
            &[&seed.card]
        ),
        0,
        "copies get new ids"
    );
    let copied = scalar_str(
        &b,
        "SELECT id FROM story_scene_card WHERE short_description='Arjun returns to the city'",
        &[],
    );
    assert_ne!(copied, seed.card);
    let chase_card_parent = scalar_str(
        &b,
        "SELECT parent_type FROM story_scene_card WHERE short_description='Rooftop run'",
        &[],
    );
    assert_eq!(chase_card_parent, "sequence");
    assert_eq!(
        scalar_str(
            &b,
            "SELECT target_id FROM comment WHERE body='Great opener'",
            &[]
        ),
        copied
    );

    // Same project (e.g. a collaborator's portable copy): changes by identity.
    a.ok(
        "story.update_card",
        json!({ "id": seed.card, "shortDescription": "Arjun returns — at night" }),
    );
    let pkg2 = out_path(&a, "board2.ofstory");
    a.ok(
        "packages.export_exchange",
        json!({ "packageType": "story", "path": pkg2 }),
    );
    a.ok(
        "story.update_card",
        json!({ "id": seed.card, "shortDescription": "Arjun returns to the city" }),
    );
    let session = a.ok("packages.open_exchange", json!({ "path": pkg2 }));
    assert_eq!(session["sameProject"], true);
    let card_change = session["changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == format!("card:{}", seed.card))
        .cloned()
        .unwrap();
    // The card changed here after export: a conflict, never applied silently.
    assert_eq!(card_change["state"], "conflict");
    assert_eq!(card_change["selectedByDefault"], false);
    let done = a.ok(
        "packages.apply_import",
        json!({ "sessionId": session["id"], "mode": "allSafe" }),
    );
    assert_ne!(done["status"], "Failed");
    assert_eq!(
        scalar_str(
            &a,
            "SELECT short_description FROM story_scene_card WHERE id=?1",
            &[&seed.card]
        ),
        "Arjun returns to the city"
    );
}

#[test]
fn removed_package_types_are_refused_on_export() {
    // Owner change list item 9 (2026-10-19): the Breakdown, Shot List, Schedule and Call Sheet
    // Review packages are no longer offered. Asking for one writes nothing.
    let env = TestEnv::with_project("Removed", "Feature Film");
    for ty in ["breakdown", "shots", "schedule", "callReview"] {
        let path = out_path(&env, &format!("x.{ty}"));
        let e = env
            .call(
                "packages.export_exchange",
                json!({ "packageType": ty, "path": path }),
            )
            .expect_err("a removed package type must be refused");
        assert_eq!(e.code.0, "validation.invalid_input", "{ty}");
        assert!(
            e.message.contains("no longer offered"),
            "{ty}: {}",
            e.message
        );
        assert!(!std::path::Path::new(&path).exists(), "{ty} wrote a file");
    }
}
#[test]
fn call_sheet_review_and_other_project_packages_are_handled_safely() {
    let env = TestEnv::with_project("Mismatch", "Feature Film");
    let seed = seed(&env);
    let pkg = export_script_review(&env, &seed.draft, "mine.ofscriptreview");
    let other = TestEnv::with_project("Somebody else", "Feature Film");
    let session = other.ok("packages.open_exchange", json!({ "path": pkg }));
    assert_eq!(session["compatibility"], "Rejected");
    assert!(
        session["rejection"]
            .as_str()
            .unwrap()
            .contains("another project")
    );
    assert_eq!(
        other.err(
            "packages.apply_import",
            json!({ "sessionId": session["id"], "mode": "allSafe" })
        ),
        "import.package_rejected"
    );
    assert_eq!(scalar_i64(&other, "SELECT count(*) FROM comment", &[]), 0);
    // A whole-project package is not an exchange package (distinct intents, IEX-013).
    let proj = out_path(&env, "whole.ofproject");
    task_ok(
        &env,
        &env.ok("packages.export_project", json!({ "path": proj })),
    );
    assert_eq!(
        env.err("packages.open_exchange", json!({ "path": proj })),
        "import.wrong_package_kind"
    );
    assert_eq!(
        env.err(
            "packages.import_project",
            json!({ "path": pkg, "mode": "open" })
        ),
        "import.wrong_package_kind"
    );
}

// ============================================== area 16: formats, restore, safety

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/packages")
}

fn manifest_of(path: &str) -> Value {
    serde_json::from_slice(
        &read_zip_texts(path)
            .into_iter()
            .find(|(n, _)| n == "manifest.json")
            .unwrap()
            .1,
    )
    .unwrap()
}

/// Names in a folder, sorted (empty when the folder does not exist).
fn entries(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .map(|r| {
            let mut v: Vec<String> = r
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect();
            v.sort();
            v
        })
        .unwrap_or_default()
}

/// Start `packages.import_project` and wait for the task: (state, result or error).
fn import_project(env: &TestEnv, path: &str, mode: &str) -> (String, Value) {
    match env.call(
        "packages.import_project",
        json!({ "path": path, "mode": mode }),
    ) {
        Ok(s) => wait_task(env, s["taskId"].as_str().unwrap()),
        Err(e) => (
            "refused".into(),
            json!({ "code": e.code.0, "message": e.message }),
        ),
    }
}

const TELUGU: &str = "అలలు తీరాన్ని తాకుతున్నాయి.";
const HINDI: &str = "लहरें किनारे को छू रही हैं।";

/// The fixture project: the common seed plus Telugu and Hindi screenplay text.
fn fixture_seed(env: &TestEnv) -> Seed {
    let seed = seed(env);
    env.ok(
        "screenplay.apply_edits",
        json!({ "draftId": seed.draft, "ops": [
            { "op": "insertElement", "sceneId": seed.scenes[0], "elementType": "action", "text": TELUGU },
            { "op": "insertElement", "sceneId": seed.scenes[0], "elementType": "action", "text": HINDI },
        ]}),
    );
    seed
}

/// Rewrite a current package the way the format 1 writer produced it: the same entries without
/// the manifest fields format 2 added, and checksums recomputed.
fn downgrade_to_format_1(src: &str, dest: &str) {
    rewrite_package(src, dest, true, |name, bytes| {
        if name != "manifest.json" {
            return bytes;
        }
        let mut v: Value = serde_json::from_slice(&bytes).unwrap();
        let o = v.as_object_mut().unwrap();
        o.remove("tableCounts");
        o.remove("carried");
        o.insert("formatVersion".into(), json!(1));
        serde_json::to_vec_pretty(&v).unwrap()
    });
}

const FIXTURE_FILES: [&str; 5] = [
    "harbour.ofbackup",
    "harbour.ofproject",
    "harbour.ofscriptreview",
    "harbour.ofstory",
    "harbour.ofresponse",
];

/// Writes the compatibility fixtures under `tests/fixtures/packages/format-N/`. Run once when a
/// package format is frozen (`cargo test -p openframe-application --test production packages::
/// write_package_fixtures -- --ignored`). Committed files of a shipped format are never
/// regenerated; a new format adds a new folder.
#[test]
#[ignore = "writes the committed package compatibility fixtures"]
fn write_package_fixtures() {
    let env = TestEnv::with_project("Harbour Lights", "Feature Film");
    let seed = fixture_seed(&env);
    let tmp = env.dir.path().join("fixtures");
    std::fs::create_dir_all(&tmp).unwrap();
    let p = |n: &str| tmp.join(n).to_string_lossy().into_owned();
    task_ok(
        &env,
        &env.ok(
            "packages.create_backup",
            json!({ "path": p("harbour.ofbackup") }),
        ),
    );
    task_ok(
        &env,
        &env.ok(
            "packages.export_project",
            json!({ "path": p("harbour.ofproject") }),
        ),
    );
    env.ok(
        "packages.export_exchange",
        json!({ "packageType": "scriptReview", "draftId": seed.draft, "scope": "comments", "path": p("harbour.ofscriptreview") }),
    );
    env.ok(
        "packages.export_exchange",
        json!({ "packageType": "story", "includeComments": true, "path": p("harbour.ofstory") }),
    );
    let rt = RoundTrip {
        env,
        seed,
        review: p("harbour.ofscriptreview"),
    };
    let resp = reviewer_response(
        &rt,
        &[(
            &rt.seed.scenes[1],
            "అర్జున్ ఇక్కడ ఆగాలి? Should Arjun pause here?",
            None,
        )],
        "harbour-reply.ofresponse",
    );
    std::fs::copy(&resp, p("harbour.ofresponse")).unwrap();
    let current = fixtures_root().join(format!(
        "format-{}",
        openframe_domain::PACKAGE_FORMAT_VERSION
    ));
    let v1 = fixtures_root().join("format-1");
    for dir in [&current, &v1] {
        std::fs::create_dir_all(dir).unwrap();
    }
    for f in FIXTURE_FILES {
        std::fs::copy(p(f), current.join(f)).unwrap();
        downgrade_to_format_1(&p(f), &v1.join(f).to_string_lossy());
    }
}

/// Every committed package format still opens: a backup restores on a new computer (its
/// database migrates from the schema it was written with), and the project, review, response
/// and Story Board packages of the same project open (packages carry the new data:
/// compatibility).
#[test]
fn older_package_formats_still_open() {
    let mut formats: Vec<PathBuf> = std::fs::read_dir(fixtures_root())
        .expect("package fixtures are committed")
        .map(|e| e.unwrap().path())
        .collect();
    formats.sort();
    assert!(
        formats.len() >= 2,
        "format 1 and the current format have fixtures"
    );
    for dir in formats {
        let version: u64 = dir
            .file_name()
            .unwrap()
            .to_string_lossy()
            .trim_start_matches("format-")
            .parse()
            .unwrap();
        let f = |n: &str| dir.join(n).to_string_lossy().into_owned();
        for n in FIXTURE_FILES {
            assert_eq!(manifest_of(&f(n))["formatVersion"], version, "{n}");
        }
        let b = TestEnv::new();
        let info = b.ok(
            "packages.inspect_package",
            json!({ "path": f("harbour.ofbackup") }),
        );
        assert_eq!(info["defaultMode"], "open", "format {version}");
        let (state, rep) = import_project(&b, &f("harbour.ofbackup"), "open");
        assert_eq!(state, "completed", "format {version}: {rep}");
        assert_eq!(rep["identityKept"], true);
        assert_eq!(
            scalar_i64(
                &b,
                "SELECT count(*) FROM story_act WHERE title='Act One'",
                &[]
            ),
            1
        );
        for text in [TELUGU, HINDI] {
            assert_eq!(
                scalar_i64(
                    &b,
                    "SELECT count(*) FROM screenplay_element WHERE text=?1",
                    &[text]
                ),
                1,
                "format {version}: Unicode text survives"
            );
        }
        assert_eq!(managed_file_bytes(&b), b"moodboard image bytes");
        // The private note travels intact and belongs to the person who restored it.
        assert_eq!(
            scalar_i64(
                &b,
                "SELECT count(*) FROM private_note WHERE body='SECRET-PLAN-XYZ kill the producer'",
                &[]
            ),
            1,
            "format {version}: private note kept"
        );
        assert_eq!(
            b.ok("private_note.list", json!({}))
                .as_array()
                .unwrap()
                .len(),
            1,
            "format {version}: the restorer sees the private note"
        );
        assert_eq!(
            scalar_i64(&b, "PRAGMA user_version", &[]) as u32,
            openframe_application::schema::project_schema_version(),
            "the restored database is migrated to the current schema"
        );
        let review = b.ok(
            "packages.open_exchange",
            json!({ "path": f("harbour.ofscriptreview") }),
        );
        assert_eq!(review["sameProject"], true, "format {version}");
        assert_ne!(review["compatibility"], "Rejected");
        b.ok(
            "packages.cancel_import",
            json!({ "sessionId": review["id"] }),
        );
        let before = scalar_i64(
            &b,
            "SELECT count(*) FROM comment WHERE deleted_at IS NULL",
            &[],
        );
        let resp = b.ok(
            "packages.open_exchange",
            json!({ "path": f("harbour.ofresponse") }),
        );
        assert_eq!(resp["packageType"], "response");
        let done = b.ok(
            "packages.apply_import",
            json!({ "sessionId": resp["id"], "mode": "allSafe" }),
        );
        assert_eq!(
            done["status"], "Applied",
            "format {version}: {}",
            done["result"]
        );
        assert_eq!(
            scalar_i64(
                &b,
                "SELECT count(*) FROM comment WHERE deleted_at IS NULL",
                &[]
            ),
            before + 1
        );
        let story = b.ok(
            "packages.open_exchange",
            json!({ "path": f("harbour.ofstory") }),
        );
        assert_eq!(story["sameProject"], true);
        assert_ne!(story["compatibility"], "Rejected");
        let c = TestEnv::new();
        let (state, rep) = import_project(&c, &f("harbour.ofproject"), "open");
        assert_eq!(state, "completed", "format {version}: {rep}");
        assert!(scalar_i64(&c, "SELECT count(*) FROM screenplay_element", &[]) > 0);
    }
}

#[test]
fn current_format_manifests_carry_counts_and_coverage() {
    let env = TestEnv::with_project("Counts", "Feature Film");
    let seed = seed(&env);
    let backup = out_path(&env, "c.ofbackup");
    task_ok(
        &env,
        &env.ok("packages.create_backup", json!({ "path": backup })),
    );
    let m = manifest_of(&backup);
    assert_eq!(m["formatVersion"], openframe_domain::PACKAGE_FORMAT_VERSION);
    assert_eq!(m["tableCounts"]["story_act"], 1);
    assert_eq!(
        m["tableCounts"]["screenplay_scene"],
        scalar_i64(&env, "SELECT count(*) FROM screenplay_scene", &[])
    );
    assert!(scalar_i64(&env, "SELECT count(*) FROM screenplay_scene", &[]) >= 3);
    assert!(m["tableCounts"].get("search_fts").is_none());
    let review = export_script_review(&env, &seed.draft, "c.ofscriptreview");
    let m = manifest_of(&review);
    let carried: Vec<&str> = m["carried"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(carried.contains(&"screenplay_element.text"));
    assert!(!carried.iter().any(|c| c.starts_with("private_note")));
}

/// Production numbers and custom elements (project schema 13) reach the reviewer in a review
/// package, come back in the response, and survive a backup restore and a Full Project Package
/// on another computer (masterplan area 16, "Packages carry the new data").
#[test]
fn production_numbers_and_custom_elements_travel_in_every_package_class() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let seed = seed(&env);
    let screenplay = scalar_str(
        &env,
        "SELECT screenplay_id FROM screenplay_draft WHERE id=?1",
        &[&seed.draft],
    );
    let tel = env.ok(
        "screenplay.create_custom_element",
        json!({ "screenplayId": screenplay, "name": "Telephone", "look": "dialogue",
                "uppercase": true, "italic": true }),
    );
    let tel_id = tel["id"].as_str().unwrap().to_string();
    let call = new_id();
    env.ok(
        "screenplay.apply_edits",
        json!({ "draftId": seed.draft, "ops": [{ "op": "insertElement", "id": call,
            "sceneId": seed.scenes[0], "elementType": "custom", "customElementId": tel_id,
            "text": "hello? who is this?" }] }),
    );
    env.ok(
        "screenplay.set_scene_number",
        json!({ "sceneId": seed.scenes[1], "number": "12A" }),
    );

    // Review package: the reviewer sees 12A and the Telephone element as the author laid it out.
    let review = export_script_review(&env, &seed.draft, "night-bus.ofscriptreview");
    let m = manifest_of(&review);
    let carried: Vec<&str> = m["carried"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(carried.contains(&"screenplay_scene.production_number"));
    assert!(carried.contains(&"screenplay_custom_element.look"));
    let reviewer = TestEnv::new();
    reviewer.core.set_display_name("Priya").unwrap();
    let ws = reviewer.ok("packages.review_open", json!({ "path": review }));
    let scenes = ws["scenes"].as_array().unwrap();
    let scene = |id: &str| scenes.iter().find(|s| s["id"] == id).unwrap().clone();
    assert_eq!(scene(&seed.scenes[1])["label"], "12A");
    // A scene without a stored number shows its place in the draft.
    let first = scene(&seed.scenes[0]);
    assert_eq!(first["label"], first["number"].to_string());
    let phone = first["elements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == call.as_str())
        .unwrap()
        .clone();
    assert_eq!(phone["customName"], "Telephone");
    assert_eq!(phone["elementType"], "dialogue");
    assert_eq!(phone["uppercase"], true);
    assert_eq!(phone["italic"], true);

    // The response keeps both, and the author's preview names the scene by its number.
    let pid = ws["key"].as_str().unwrap().to_string();
    reviewer.ok(
        "packages.review_comment",
        json!({ "packageId": pid, "sceneId": seed.scenes[1], "body": "Cut the folder." }),
    );
    let dest = reviewer.dir.path().join("priya.ofresponse");
    let r = reviewer.ok(
        "packages.review_export_response",
        json!({ "packageId": pid, "path": dest.to_string_lossy() }),
    );
    let resp = out_path(&env, "priya.ofresponse");
    std::fs::copy(r["path"].as_str().unwrap(), &resp).unwrap();
    let session = env.ok("packages.open_exchange", json!({ "path": resp }));
    let detail = session["changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["group"] == "comments")
        .and_then(|c| c["detail"].as_str())
        .unwrap()
        .to_string();
    assert!(detail.starts_with("Scene 12A · "), "{detail}");

    // Backup restored as a copy, and a Full Project Package on another computer.
    let backup = out_path(&env, "night-bus.ofbackup");
    task_ok(
        &env,
        &env.ok("packages.create_backup", json!({ "path": backup })),
    );
    let m = manifest_of(&backup);
    assert_eq!(m["tableCounts"]["screenplay_custom_element"], 1);
    let pkg = out_path(&env, "night-bus.ofproject");
    task_ok(
        &env,
        &env.ok("packages.export_project", json!({ "path": pkg })),
    );
    let check = |e: &TestEnv| {
        assert_eq!(
            scalar_str(
                e,
                "SELECT production_number FROM screenplay_scene WHERE id=?1",
                &[&seed.scenes[1]]
            ),
            "12A"
        );
        assert_eq!(
            scalar_str(
                e,
                "SELECT custom_element_id FROM screenplay_element WHERE id=?1",
                &[&call]
            ),
            tel_id
        );
        assert_eq!(
            scalar_str(
                e,
                "SELECT name || ':' || look || ':' || uppercase || ':' || italic FROM screenplay_custom_element WHERE id=?1",
                &[&tel_id]
            ),
            "Telephone:dialogue:1:1"
        );
    };
    let (state, rep) = import_project(&env, &backup, "copy");
    assert_eq!(state, "completed", "{rep}");
    check(&env);
    let other = TestEnv::new();
    let (state, rep) = import_project(&other, &pkg, "open");
    assert_eq!(state, "completed", "{rep}");
    check(&other);
}

// ---------------------------------------------------------------- restore drill

#[test]
fn backup_restores_on_a_new_computer_with_a_fresh_data_root() {
    let a = TestEnv::with_project("Salt Road", "Feature Film");
    let seed = fixture_seed(&a);
    let a_id = project_id(&a);
    let a_user = a.actor().user_id;
    let backup = out_path(&a, "Salt Road.ofbackup");
    task_ok(
        &a,
        &a.ok(
            "packages.create_backup",
            json!({ "path": backup, "includeExternal": true }),
        ),
    );
    // The new computer: its own app data, projects folder and settings; nothing of A.
    let b = TestEnv::new();
    assert!(b.core.project_opt().is_none());
    assert!(entries(&b.core.config.projects_dir).is_empty());
    let info = b.ok("packages.inspect_package", json!({ "path": backup }));
    assert!(info["collision"].is_null());
    assert_eq!(info["typeLabel"], "Backup Package");
    let (state, rep) = import_project(&b, &backup, "open");
    assert_eq!(state, "completed", "{rep}");
    assert_eq!(project_id(&b), a_id);
    assert_eq!(rep["copiedExternal"], 1);
    assert_eq!(rep["missingExternal"].as_array().unwrap().len(), 0);
    assert_eq!(
        scalar_str(
            &b,
            "SELECT heading FROM screenplay_scene WHERE id=?1",
            &[&seed.scenes[1]]
        ),
        "INT. POLICE STATION — NIGHT"
    );
    assert_eq!(
        scalar_i64(
            &b,
            "SELECT count(*) FROM screenplay_element WHERE text=?1",
            &[TELUGU]
        ),
        1
    );
    // The private note is restored intact and now belongs to the person who restored it
    // (ADR-0012 amendment of 2026-10-08); the report says so.
    let b_user = b.actor().user_id;
    assert_ne!(b_user, a_user, "the new computer has its own user");
    assert_eq!(
        scalar_str(
            &b,
            "SELECT body FROM private_note WHERE deleted_at IS NULL",
            &[]
        ),
        "SECRET-PLAN-XYZ kill the producer"
    );
    assert_eq!(
        scalar_str(&b, "SELECT owner_user_id FROM private_note", &[]),
        b_user
    );
    assert!(
        rep["message"]
            .as_str()
            .unwrap()
            .contains("1 private note from the package is now yours."),
        "{rep}"
    );
    let notes = b.ok("private_note.list", json!({}));
    assert_eq!(notes.as_array().unwrap().len(), 1, "{notes}");
    // It is an ordinary project there: edit, restart, reopen, listed on Home.
    b.ok("story.create_act", json!({ "title": "Restored act" }));
    let path = b.project_path();
    let b = b.restart();
    b.reopen_project(&path);
    assert_eq!(
        scalar_i64(
            &b,
            "SELECT count(*) FROM story_act WHERE title='Restored act'",
            &[]
        ),
        1
    );
    let recents = b.ok("project.list_recent", json!({}));
    assert!(recents.to_string().contains("Salt Road"));
}

/// A damaged backup is refused before anything is created: no project folder, no staging
/// folder, and the project that is open stays open and unchanged.
#[test]
fn corrupt_backups_are_detected_and_leave_nothing_behind() {
    let a = TestEnv::with_project("Broken", "Feature Film");
    seed(&a);
    let good = out_path(&a, "good.ofbackup");
    task_ok(&a, &a.ok("packages.create_backup", json!({ "path": good })));
    let bytes = std::fs::read(&good).unwrap();

    let truncated = out_path(&a, "truncated.ofbackup");
    std::fs::write(&truncated, &bytes[..bytes.len() / 2]).unwrap();

    let flipped = out_path(&a, "flipped.ofbackup");
    let mut f = bytes.clone();
    let mid = f.len() / 2;
    for b in &mut f[mid..mid + 64] {
        *b ^= 0x5A;
    }
    std::fs::write(&flipped, &f).unwrap();

    // Database pages damaged and the checksums recomputed (a disk fault before export).
    let damaged_db = out_path(&a, "damaged-db.ofbackup");
    rewrite_package(&good, &damaged_db, true, |name, mut b| {
        if name == "project/project.sqlite" {
            let page = 4096 * 3;
            for x in &mut b[page..page + 512] {
                *x = 0xFF;
            }
        }
        b
    });

    // Edit the packaged database with checksums recomputed.
    let edit_db = |dest: &str, work: &str, sql: &'static str| {
        let work = a.dir.path().join(work);
        std::fs::create_dir_all(&work).unwrap();
        rewrite_package(&good, dest, true, |name, b| {
            if name != "project/project.sqlite" {
                return b;
            }
            let db = work.join("p.sqlite");
            std::fs::write(&db, &b).unwrap();
            {
                let c = rusqlite::Connection::open(&db).unwrap();
                c.execute_batch("PRAGMA journal_mode=DELETE;").unwrap();
                c.execute_batch(sql).unwrap();
            }
            std::fs::read(&db).unwrap()
        });
    };
    // Rows removed: the manifest's table counts disagree.
    let rows_removed = out_path(&a, "rows-removed.ofbackup");
    edit_db(&rows_removed, "rows", "DELETE FROM story_scene_card;");
    // A foreign trigger planted in the database (a failure after extraction).
    let planted = out_path(&a, "planted.ofbackup");
    edit_db(
        &planted,
        "planted",
        "CREATE TRIGGER evil AFTER INSERT ON story_act BEGIN DELETE FROM story_scene_card; END;",
    );

    let b = TestEnv::with_project("Already open", "Short Film");
    let open_path = b.project_path();
    let projects = b.core.config.projects_dir.clone();
    let before = entries(&projects);
    for (pkg, what) in [
        (&truncated, "truncated"),
        (&flipped, "flipped bytes"),
        (&damaged_db, "damaged database"),
        (&rows_removed, "rows removed"),
        (&planted, "planted trigger"),
    ] {
        let (state, err) = import_project(&b, pkg, "copy");
        assert_ne!(state, "completed", "{what} was accepted");
        let code = err["code"].as_str().unwrap_or_default().to_string();
        assert!(
            code.starts_with("import.") || code.starts_with("persistence."),
            "{what}: {err}"
        );
        assert!(
            !err["message"].as_str().unwrap_or_default().is_empty(),
            "{what}: a message for the user"
        );
        assert_eq!(entries(&projects), before, "{what} left something behind");
        assert_eq!(
            b.project_path(),
            open_path,
            "{what}: the open project changed"
        );
    }
    let (_, err) = import_project(&b, &rows_removed, "copy");
    assert_eq!(err["code"], "import.package_incomplete", "{err}");
    // The undamaged backup still restores.
    let (state, rep) = import_project(&b, &good, "copy");
    assert_eq!(state, "completed", "{rep}");
}

/// Cancelling a restore at any point leaves no project, no staging folder and no change.
#[test]
fn interrupted_restore_leaves_nothing_behind() {
    use openframe_application::modules::packages::{PackageOpenMode, import_project_package};
    let a = TestEnv::with_project("Interrupted", "Feature Film");
    seed(&a);
    let pkg = out_path(&a, "i.ofproject");
    task_ok(&a, &a.ok("packages.export_project", json!({ "path": pkg })));
    let b = TestEnv::new();
    let projects = b.core.config.projects_dir.clone();
    std::fs::create_dir_all(&projects).unwrap();
    let mut completed = false;
    for stop_at in 0..50 {
        let calls = std::cell::Cell::new(0usize);
        let out = import_project_package(
            &b.core,
            &b.actor(),
            Path::new(&pkg),
            PackageOpenMode::Open,
            None,
            &|_, _| {},
            &|| {
                calls.set(calls.get() + 1);
                calls.get() > stop_at
            },
        )
        .unwrap();
        match out {
            None => {
                assert!(
                    entries(&projects).is_empty(),
                    "cancel at {stop_at}: {:?}",
                    entries(&projects)
                );
                assert!(b.core.project_opt().is_none());
            }
            Some(r) => {
                assert!(r.identity_kept);
                assert_eq!(entries(&projects).len(), 1);
                completed = true;
                break;
            }
        }
    }
    assert!(completed);
}

// ------------------------------------------------------- partial output on failure

/// Exchange and response packages written over a read-only file: refused, the old file kept and
/// no temporary file left (session 1 defect: partial output on failure).
#[cfg(windows)]
#[test]
fn failed_exchange_and_response_exports_leave_no_partial_file() {
    struct ReadOnly(PathBuf);
    impl ReadOnly {
        fn set(p: &Path) -> ReadOnly {
            let mut perms = std::fs::metadata(p).unwrap().permissions();
            perms.set_readonly(true);
            std::fs::set_permissions(p, perms).unwrap();
            ReadOnly(p.to_path_buf())
        }
    }
    impl Drop for ReadOnly {
        fn drop(&mut self) {
            let mut p = std::fs::metadata(&self.0).unwrap().permissions();
            #[allow(clippy::permissions_set_readonly_false)]
            p.set_readonly(false);
            std::fs::set_permissions(&self.0, p).unwrap();
        }
    }
    let rt = round_trip("Read Only");
    let env = &rt.env;
    let dir = env.dir.path().join("ro-out");
    std::fs::create_dir_all(&dir).unwrap();
    let story = dir.join("board.ofstory");
    std::fs::write(&story, b"old story").unwrap();
    {
        let _g = ReadOnly::set(&story);
        let err = env
            .call(
                "packages.export_exchange",
                json!({ "packageType": "story", "path": story }),
            )
            .unwrap_err();
        assert!(!err.message.is_empty());
        assert_eq!(std::fs::read(&story).unwrap(), b"old story");
    }
    let reviewer = TestEnv::new();
    let ws = reviewer.ok("packages.review_open", json!({ "path": rt.review }));
    let key = ws["key"].as_str().unwrap().to_string();
    reviewer.ok(
        "packages.review_comment",
        json!({ "packageId": key, "sceneId": rt.seed.scenes[1], "body": "Hmm" }),
    );
    let resp = dir.join("reply.ofresponse");
    std::fs::write(&resp, b"old response").unwrap();
    {
        let _g = ReadOnly::set(&resp);
        let err = reviewer
            .call(
                "packages.review_export_response",
                json!({ "packageId": key, "path": resp }),
            )
            .unwrap_err();
        assert!(!err.message.is_empty());
        assert_eq!(std::fs::read(&resp).unwrap(), b"old response");
    }
    assert_eq!(
        entries(&dir),
        vec!["board.ofstory".to_string(), "reply.ofresponse".to_string()],
        "no temporary file left next to the destination"
    );
}

// ------------------------------------------------- identity collisions and roles

/// An exchange package whose object identities collide with another project's objects never
/// overwrites them: a package from another project can only be copied with new identities.
#[test]
fn exchange_identity_collision_never_overwrites_another_project() {
    let a = TestEnv::with_project("Sender", "Feature Film");
    let a_seed = seed(&a);
    let pkg = out_path(&a, "a.ofstory");
    a.ok(
        "packages.export_exchange",
        json!({ "packageType": "story", "path": pkg }),
    );
    let b = TestEnv::with_project("Receiver", "Feature Film");
    let b_act = b.ok("story.create_act", json!({ "title": "Receiver act" }))["id"]
        .as_str()
        .unwrap()
        .to_string();
    let b_card = b.ok(
        "story.create_card",
        json!({ "parent": { "parentType": "act", "parentId": b_act }, "shortDescription": "Receiver card" }),
    )["id"]
        .as_str()
        .unwrap()
        .to_string();
    // Forge the package so its act and card carry the receiver's identities.
    let forged = out_path(&a, "forged.ofstory");
    rewrite_package(&pkg, &forged, true, |name, bytes| {
        if name != "content.json" {
            return bytes;
        }
        String::from_utf8(bytes)
            .unwrap()
            .replace(&a_seed.card, &b_card)
            .replace(&a_seed.act, &b_act)
            .into_bytes()
    });
    let session = b.ok("packages.open_exchange", json!({ "path": forged }));
    assert_eq!(session["sameProject"], false);
    assert_eq!(session["canCopyAsNew"], true);
    assert!(
        session["changes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["copyOnly"] == true || c["applicable"] == false),
        "nothing applies by identity from another project: {}",
        session["changes"]
    );
    let done = b.ok(
        "packages.apply_import",
        json!({ "sessionId": session["id"], "mode": "copyAsNew" }),
    );
    assert_ne!(done["status"], "Failed", "{}", done["result"]);
    assert_eq!(
        scalar_str(
            &b,
            "SELECT short_description FROM story_scene_card WHERE id=?1",
            &[&b_card]
        ),
        "Receiver card"
    );
    assert_eq!(
        scalar_str(&b, "SELECT title FROM story_act WHERE id=?1", &[&b_act]),
        "Receiver act"
    );
    assert_eq!(
        scalar_i64(
            &b,
            "SELECT count(*) FROM story_scene_card WHERE short_description='Arjun returns to the city'",
            &[]
        ),
        1,
        "the sender's card arrives as a new card"
    );
}

/// Roles decide what a member may send and what they may apply (ADR-0007, ADR-0009).
#[test]
fn package_roles_scope_what_each_member_can_send_and_apply() {
    let env = TestEnv::with_project("Roles", "Feature Film");
    let seed = seed(&env);
    for (role, allowed) in [
        (Role::Owner, true),
        (Role::Editor, true),
        (Role::ExportOnly, true),
        (Role::Commenter, false),
        (Role::Viewer, false),
    ] {
        let actor = env.actor_with_role(role);
        let r = env.call_as(
            &actor,
            "packages.export_exchange",
            json!({ "packageType": "story", "path": out_path(&env, &format!("{}.ofstory", role.as_str())) }),
        );
        assert_eq!(r.is_ok(), allowed, "{role:?} export");
        if let Err(e) = r {
            assert_eq!(e.code.0, "permission.denied");
        }
    }
    // A same-project Story Board package with a changed card: an object change, not a comment.
    let pkg = out_path(&env, "change.ofstory");
    env.ok(
        "story.update_card",
        json!({ "id": seed.card, "shortDescription": "Changed in the package" }),
    );
    env.ok(
        "packages.export_exchange",
        json!({ "packageType": "story", "path": pkg }),
    );
    env.undo();
    assert_eq!(
        scalar_str(
            &env,
            "SELECT short_description FROM story_scene_card WHERE id=?1",
            &[&seed.card]
        ),
        "Arjun returns to the city"
    );
    let session = env.ok("packages.open_exchange", json!({ "path": pkg }));
    let change = session["changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == format!("card:{}", seed.card))
        .cloned()
        .expect("the card change is previewed");
    for role in [Role::Commenter, Role::Viewer, Role::ExportOnly] {
        let err = env
            .call_as(
                &env.actor_with_role(role),
                "packages.apply_import",
                json!({ "sessionId": session["id"], "mode": "selected", "selected": [change["id"]] }),
            )
            .unwrap_err();
        assert_eq!(
            err.code.0, "permission.denied",
            "{role:?} applied an object change"
        );
    }
    assert_eq!(
        scalar_str(
            &env,
            "SELECT short_description FROM story_scene_card WHERE id=?1",
            &[&seed.card]
        ),
        "Arjun returns to the city"
    );
    let done = env
        .call_as(
            &env.actor_with_role(Role::Editor),
            "packages.apply_import",
            json!({ "sessionId": session["id"], "mode": "selected", "selected": [change["id"]] }),
        )
        .unwrap();
    assert_ne!(done["status"], "Failed", "{}", done["result"]);
    assert_eq!(
        scalar_str(
            &env,
            "SELECT short_description FROM story_scene_card WHERE id=?1",
            &[&seed.card]
        ),
        "Changed in the package"
    );
}

/// A Full Project Package carries the exporting member's private notes only; another member's
/// note leaves no trace in the database file, its search index or its free pages.
#[test]
fn full_project_package_holds_no_trace_of_other_members_private_notes() {
    let env = TestEnv::with_project("Privacy", "Feature Film");
    let seed = seed(&env);
    let other = env.other_user(Role::Editor);
    env.call_as(
        &other,
        "private_note.create",
        json!({ "targetType": "screenplay_scene", "targetId": seed.scenes[0], "body": "ZEBRAQUARTZ belongs to someone else" }),
    )
    .unwrap();
    let pkg = out_path(&env, "privacy.ofproject");
    task_ok(
        &env,
        &env.ok("packages.export_project", json!({ "path": pkg })),
    );
    let db = read_zip_texts(&pkg)
        .into_iter()
        .find(|(n, _)| n == "project/project.sqlite")
        .unwrap()
        .1;
    let hay = String::from_utf8_lossy(&db).to_lowercase();
    assert!(
        !hay.contains("zebraquartz"),
        "another member's private note is in the package"
    );
    assert!(
        hay.contains("secret-plan-xyz"),
        "the exporter's own private note travels"
    );
    // The owner's private backup keeps both.
    let backup = out_path(&env, "privacy.ofbackup");
    task_ok(
        &env,
        &env.ok("packages.create_backup", json!({ "path": backup })),
    );
    let b = TestEnv::new();
    let (state, rep) = import_project(&b, &backup, "open");
    assert_eq!(state, "completed", "{rep}");
    assert_eq!(scalar_i64(&b, "SELECT count(*) FROM private_note", &[]), 2);
}

/// ADR-0012 amendment of 2026-10-08: restoring a backup or a Full Project Package makes the
/// person restoring it the owner of the private notes it holds. They see the notes in their
/// list and in search, can edit and delete them, and the notes stay private: another member
/// of the restored project sees nothing and cannot touch them.
#[test]
fn restored_private_notes_belong_to_the_restorer_and_stay_private() {
    let a = TestEnv::with_project("Ownership", "Feature Film");
    let seed = seed(&a);
    let colleague = a.other_user(Role::Editor);
    a.call_as(
        &colleague,
        "private_note.create",
        json!({ "targetType": "screenplay_scene", "targetId": seed.scenes[0], "body": "ZEBRAQUARTZ from a colleague" }),
    )
    .unwrap();
    let backup = out_path(&a, "ownership.ofbackup");
    task_ok(
        &a,
        &a.ok("packages.create_backup", json!({ "path": backup })),
    );
    let package = out_path(&a, "ownership.ofproject");
    task_ok(
        &a,
        &a.ok("packages.export_project", json!({ "path": package })),
    );

    for (path, notes_in_file) in [(&backup, 2usize), (&package, 1usize)] {
        let b = TestEnv::new();
        let (state, rep) = import_project(&b, path, "open");
        assert_eq!(state, "completed", "{path}: {rep}");
        let me = b.actor();
        // Every note in the file is now the restorer's, in the table and in the search index.
        assert_eq!(
            scalar_i64(
                &b,
                "SELECT count(*) FROM private_note WHERE owner_user_id=?1",
                &[&me.user_id]
            ) as usize,
            notes_in_file,
            "{path}"
        );
        assert_eq!(
            scalar_i64(
                &b,
                "SELECT count(*) FROM search_doc WHERE source_table='private_note' AND owner_user_id IS NOT ?1",
                &[&me.user_id]
            ),
            0,
            "{path}: no index entry keeps the old owner"
        );
        let mine = b.ok("private_note.list", json!({}));
        let mine = mine.as_array().unwrap();
        assert_eq!(mine.len(), notes_in_file, "{path}: {mine:?}");
        let hits = b.ok("search.query", json!({ "text": "SECRET-PLAN-XYZ" }));
        assert!(
            hits.to_string().contains("private_note"),
            "{path}: the restorer finds their note in search: {hits}"
        );

        // Visible and editable by the restorer, and the edit survives a restart.
        let id = mine
            .iter()
            .find(|n| n["body"].as_str().unwrap().starts_with("SECRET-PLAN-XYZ"))
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();
        // No undo entry with the old owner's row image is left to hand the note back.
        assert_eq!(
            scalar_i64(
                &b,
                "SELECT count(*) FROM sys_undo WHERE instr(changes_json, ?1) > 0",
                &[&id]
            ),
            0,
            "{path}"
        );
        let upd = b.ok(
            "private_note.update",
            json!({ "id": id, "body": "Edited after restore" }),
        );
        assert_eq!(upd["body"], "Edited after restore");

        // Another member of the restored project sees none of it and cannot touch it.
        let other = b.other_user(Role::Editor);
        let theirs = b.call_as(&other, "private_note.list", json!({})).unwrap();
        assert!(theirs.as_array().unwrap().is_empty(), "{path}: {theirs}");
        let found = b
            .call_as(&other, "search.query", json!({ "text": "SECRET-PLAN-XYZ" }))
            .unwrap();
        assert!(
            !found.to_string().contains("private_note"),
            "{path}: {found}"
        );
        let found = b
            .call_as(
                &other,
                "search.query",
                json!({ "text": "Edited after restore" }),
            )
            .unwrap();
        assert!(
            !found.to_string().contains("private_note"),
            "{path}: {found}"
        );
        let e = b
            .call_as(
                &other,
                "private_note.update",
                json!({ "id": id, "body": "hijack" }),
            )
            .unwrap_err();
        assert_eq!(e.code.0, "not_found.private_note", "{path}");
        let e = b
            .call_as(&other, "private_note.delete", json!({ "id": id }))
            .unwrap_err();
        assert_eq!(e.code.0, "not_found.private_note", "{path}");

        let project = b.project_path();
        let b = b.restart();
        b.reopen_project(&project);
        let mine = b.ok("private_note.list", json!({}));
        assert!(
            mine.to_string().contains("Edited after restore"),
            "{path}: {mine}"
        );
    }
}

// ------------------------------------------------------------- no network listener

/// ADR-0007: OpenFrame opens no listening socket. After the core package workflow (edit,
/// backup, review round trip, restore on another installation) this process holds no listening
/// TCP socket and no bound UDP socket. The Offline AI engine, which the user starts on purpose,
/// is a separate process on 127.0.0.1 with a per-launch key and is not part of this workflow.
#[cfg(windows)]
#[test]
fn the_core_workflow_opens_no_network_listener() {
    let rt = round_trip("No Listener");
    let env = &rt.env;
    let backup = out_path(env, "nl.ofbackup");
    task_ok(
        env,
        &env.ok("packages.create_backup", json!({ "path": backup })),
    );
    let resp = reviewer_response(&rt, &[(&rt.seed.scenes[1], "Fine", None)], "nl.ofresponse");
    let s = env.ok("packages.open_exchange", json!({ "path": resp }));
    env.ok(
        "packages.apply_import",
        json!({ "sessionId": s["id"], "mode": "allSafe" }),
    );
    let other = TestEnv::new();
    let (state, _) = import_project(&other, &backup, "open");
    assert_eq!(state, "completed");
    let out = std::process::Command::new("netstat")
        .args(["-ano"])
        .output()
        .expect("netstat runs");
    let text = String::from_utf8_lossy(&out.stdout);
    let pid = std::process::id().to_string();
    let mine: Vec<&str> = text
        .lines()
        .filter(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            match f.as_slice() {
                ["TCP", _, _, "LISTENING", p] => *p == pid,
                ["UDP", _, _, p] => *p == pid,
                _ => false,
            }
        })
        .collect();
    assert!(text.contains("TCP"), "netstat output was read");
    assert!(mine.is_empty(), "OpenFrame opened a listener: {mine:?}");
}

/// The desktop app's dependency tree holds no network server, discovery or peer-to-peer crate.
/// Source code is checked separately by architecture lint rule R3.
///
/// The tree comes from `cargo tree`, which resolves features the way the build does.
/// `Cargo.lock` is not enough: it also lists optional crates that only a weak feature names
/// (reqwest 0.12 lists `quinn` for HTTP/3, which OpenFrame never enables), so reading the
/// lock file would report crates that are never compiled.
#[test]
fn the_desktop_app_depends_on_no_network_server_crate() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let out = std::process::Command::new(cargo)
        .current_dir(&root)
        .args([
            "tree",
            "-p",
            "openframe-desktop",
            "--target",
            "all",
            "-e",
            "normal,build",
            "--prefix",
            "none",
            "--format",
            "{p}",
            "--frozen",
        ])
        .output()
        .expect("cargo tree runs");
    assert!(
        out.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let seen: std::collections::BTreeSet<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .map(str::to_string)
        .collect();
    assert!(seen.len() > 50, "the dependency tree was read");
    let forbidden = [
        "axum",
        "actix-web",
        "actix-server",
        "warp",
        "tiny_http",
        "rouille",
        "tide",
        "poem",
        "rocket",
        "salvo",
        "gotham",
        "tokio-tungstenite",
        "tungstenite",
        "async-tungstenite",
        "mdns",
        "mdns-sd",
        "libmdns",
        "zeroconf",
        "libp2p",
        "quinn",
    ];
    let found: Vec<&str> = forbidden
        .iter()
        .copied()
        .filter(|f| seen.contains(*f))
        .collect();
    assert!(
        found.is_empty(),
        "network server crates in the app: {found:?}"
    );
}

/// LAN collaboration was removed (ADR-0007): no user-facing string in the app or the Rust core
/// offers it. Comments may still record the removal.
#[test]
fn no_lan_collaboration_wording_in_user_facing_text() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut hits = vec![];
    let mut stack = vec![root.join("apps/desktop/src"), root.join("crates")];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).unwrap() {
            let p = e.unwrap().path();
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            if p.is_dir() {
                if !matches!(
                    name.as_str(),
                    "tests" | "target" | "node_modules" | "fixtures"
                ) {
                    stack.push(p);
                }
                continue;
            }
            if !(name.ends_with(".rs") || name.ends_with(".ts") || name.ends_with(".tsx"))
                || name.contains(".test.")
            {
                continue;
            }
            let text = std::fs::read_to_string(&p).unwrap_or_default();
            for (i, line) in text.lines().enumerate() {
                let t = line.trim_start();
                if t.starts_with("#[cfg(test)]") {
                    break;
                }
                if t.starts_with("//") || t.starts_with('*') || t.starts_with("/*") {
                    continue;
                }
                if offers_lan(line) {
                    hits.push(format!("{}:{}: {}", p.display(), i + 1, t));
                }
            }
        }
    }
    assert!(
        hits.is_empty(),
        "LAN collaboration wording:\n{}",
        hits.join("\n")
    );
}

fn offers_lan(line: &str) -> bool {
    let word_lan = line.match_indices("LAN").any(|(i, _)| {
        let before = line[..i].chars().last();
        let after = line[i + 3..].chars().next();
        !before.is_some_and(|c| c.is_alphanumeric() || c == '_')
            && !after.is_some_and(|c| c.is_alphanumeric() || c == '_')
    });
    let lower = line.to_lowercase();
    word_lan
        || [
            "local network collaboration",
            "local-network collaboration",
            "network collaboration",
            "collaboration session",
            "host a session",
            "join a session",
            "live co-editing",
        ]
        .iter()
        .any(|p| lower.contains(p))
}

#[test]
fn the_lan_wording_check_catches_offers() {
    assert!(offers_lan("label: \"Host a LAN session\""));
    assert!(offers_lan("\"Start a local-network collaboration\""));
    assert!(!offers_lan("const PLAN = \"plan\"; // PLANS"));
    assert!(!offers_lan("\"Clean up\""));
}

/// Windows phase W2: a replace that fails, and then cannot put the old project back, says where
/// the old project is (the hidden `.replaced-` folder) instead of claiming nothing changed. The
/// project in that folder is whole: renaming it back opens it with work made after the export.
#[test]
fn a_failed_replace_that_cannot_put_the_project_back_names_the_hidden_folder() {
    use openframe_application::modules::packages::backup::fault;
    const TITLE: &str = "Stranded Replace";
    let env = TestEnv::with_project(TITLE, "Feature Film");
    let original = PathBuf::from(env.project_path());
    let pkg = out_path(&env, "stranded.ofproject");
    task_ok(
        &env,
        &env.ok("packages.export_project", json!({ "path": pkg })),
    );
    env.ok(
        "story.create_act",
        json!({ "title": "Only in the old copy" }),
    );

    fault::strand_replace_of(Some(TITLE));
    let started = env.ok(
        "packages.import_project",
        json!({ "path": pkg, "mode": "replace" }),
    );
    let (state, err) = wait_task(&env, started["taskId"].as_str().unwrap());
    fault::strand_replace_of(None);
    assert_eq!(state, "failed", "{err}");
    assert_eq!(err["code"], "storage.replace_rollback_failed", "{err}");

    let parent = original.parent().unwrap();
    let prefix = format!(
        ".{}.replaced-",
        original.file_name().unwrap().to_string_lossy()
    );
    let hidden: Vec<String> = std::fs::read_dir(parent)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with(&prefix))
        .collect();
    assert_eq!(hidden.len(), 1, "one hidden copy: {hidden:?}");
    let message = err["message"].as_str().unwrap();
    assert!(
        message.contains(&hidden[0]),
        "the message names the hidden folder: {message}"
    );
    assert!(!original.exists(), "the half-imported project was removed");

    std::fs::rename(parent.join(&hidden[0]), &original).unwrap();
    env.reopen_project(&original.to_string_lossy());
    assert_eq!(
        scalar_i64(
            &env,
            "SELECT count(*) FROM story_act WHERE title='Only in the old copy'",
            &[]
        ),
        1,
        "the old project, with the work made after the export"
    );
}
