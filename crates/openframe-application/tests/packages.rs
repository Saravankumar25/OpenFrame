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
                        (_, Some(err)) => json!({ "code": err.code.0, "message": err.message }),
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
        !names
            .iter()
            .any(|n| n.ends_with(".tmp") || n.ends_with(".partial")),
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
            .contains("Review Queue — nothing was discarded")
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
fn fsd_50_schedule_package_never_deletes_local_days() {
    let env = TestEnv::with_project("Schedule", "Feature Film");
    let seed = seed(&env);
    env.ok("production.set_source", json!({ "draftId": seed.draft }));
    let sched = env.ok("schedule.create", json!({}));
    let schedule_id = sched["id"]
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| scalar_str(&env, "SELECT id FROM shooting_schedule LIMIT 1", &[]));
    let day1: String = env
        .ok(
            "schedule.create_day",
            json!({ "scheduleId": schedule_id, "notes": "Early call" }),
        )
        .as_str()
        .unwrap()
        .to_string();
    let pkg = out_path(&env, "sched.ofschedule");
    env.ok(
        "packages.export_exchange",
        json!({ "packageType": "schedule", "path": pkg }),
    );
    // Locally: a new day appears after export.
    env.ok(
        "schedule.create_day",
        json!({ "scheduleId": schedule_id, "notes": "Local only" }),
    );
    let days_before = scalar_i64(
        &env,
        "SELECT count(*) FROM shooting_day WHERE deleted_at IS NULL",
        &[],
    );
    let session = env.ok("packages.open_exchange", json!({ "path": pkg }));
    assert!(
        session["changes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["group"] == "schedule.kept")
    );
    env.ok(
        "packages.apply_import",
        json!({ "sessionId": session["id"], "mode": "allSafe" }),
    );
    assert_eq!(
        scalar_i64(
            &env,
            "SELECT count(*) FROM shooting_day WHERE deleted_at IS NULL",
            &[]
        ),
        days_before
    );
    assert_eq!(
        scalar_str(&env, "SELECT notes FROM shooting_day WHERE id=?1", &[&day1]),
        "Early call"
    );

    // All-or-nothing: when one selected change fails, the ones already applied are rolled back.
    let bad = out_path(&env, "bad-sched.ofschedule");
    rewrite_package(&pkg, &bad, true, |name, bytes| {
        if name == "content.json" {
            let mut v: Value = serde_json::from_slice(&bytes).unwrap();
            v["days"][0]["notes"] = json!("Changed by package");
            v["days"][0]["date"] = json!("2026-02-31");
            serde_json::to_vec(&v).unwrap()
        } else {
            bytes
        }
    });
    let session = env.ok("packages.open_exchange", json!({ "path": bad }));
    let ids: Vec<Value> = session["changes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["group"] == "schedule.notes" || c["group"] == "schedule.dates")
        .map(|c| c["id"].clone())
        .collect();
    assert_eq!(ids.len(), 2);
    let r = env.ok(
        "packages.apply_import",
        json!({ "sessionId": session["id"], "mode": "selected", "selected": ids }),
    );
    assert_eq!(r["status"], "Failed");
    assert!(
        r["result"]["message"]
            .as_str()
            .unwrap()
            .contains("your project was not changed"),
        "{}",
        r["result"]
    );
    assert_eq!(
        scalar_str(&env, "SELECT notes FROM shooting_day WHERE id=?1", &[&day1]),
        "Early call"
    );
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
