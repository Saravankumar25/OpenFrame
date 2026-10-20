//! End-to-end tests of the core pipeline through the public operation registry
//! (the same path the UI, exchange-package imports and AI Change Sets use).

use openframe_domain::Role;
use openframe_test_support::TestEnv;
use serde_json::json;

#[test]
fn create_project_opens_at_empty_home() {
    let env = TestEnv::with_project("Railway Nights", "Feature Film");
    let home = env.ok("project.home", json!({}));
    assert_eq!(home["project"]["title"], "Railway Nights");
    assert_eq!(home["project"]["projectType"], "Feature Film");
    assert_eq!(
        home["isEmpty"], true,
        "FSD §6.3: no fake metrics in a new project"
    );
    assert!(home["counts"].as_array().unwrap().is_empty());
    let recent = env.ok("project.list_recent", json!({}));
    assert_eq!(recent.as_array().unwrap().len(), 1);
    assert_eq!(recent[0]["available"], true);
}

#[test]
fn validation_errors_are_human_and_nothing_is_created() {
    let env = TestEnv::new();
    assert_eq!(
        env.err(
            "project.create",
            json!({ "title": "   ", "projectType": "Short Film" })
        ),
        "validation.required"
    );
    assert_eq!(
        env.err(
            "project.create",
            json!({ "title": "X", "projectType": "Opera" })
        ),
        "validation.invalid_input"
    );
    assert_eq!(
        env.err("nope.unknown", json!({})),
        "security.unknown_operation"
    );
    let dirs = std::fs::read_dir(env.dir.path().join("projects"))
        .map(|d| d.count())
        .unwrap_or(0);
    assert_eq!(dirs, 0, "a failed create must not leave a folder behind");
}

#[test]
fn file_add_search_undo_redo_and_trash() {
    let env = TestEnv::with_project("Short", "Short Film");
    let src = env.write_file("location-scout.txt", b"old mill by the river");
    let added = env.ok("files.add", json!({ "paths": [src], "mode": "copy" }));
    let file_id = added[0]["id"].as_str().unwrap().to_string();
    assert_eq!(added[0]["asset"]["storageMode"], "managed");
    assert_eq!(added[0]["asset"]["available"], true);

    // Search finds it by name.
    let hits = env.ok("search.query", json!({ "text": "location scout" }));
    assert_eq!(hits[0]["entityId"], file_id);

    // Undo removes it, redo restores the same identity.
    let step = env.undo();
    assert!(step["label"].as_str().unwrap().contains("Added file"));
    assert_eq!(
        env.ok("files.list", json!({}))["files"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert!(
        env.ok("search.query", json!({ "text": "scout" }))
            .as_array()
            .unwrap()
            .is_empty()
    );
    env.redo();
    let listing = env.ok("files.list", json!({}));
    assert_eq!(listing["files"][0]["id"], file_id);

    // Rename is undoable and search reflects it.
    env.ok(
        "files.rename",
        json!({ "id": file_id, "name": "Mill scout notes" }),
    );
    assert_eq!(
        env.ok("search.query", json!({ "text": "mill" }))[0]["title"],
        "Mill scout notes"
    );

    // Delete → Recently Deleted → restore → delete → purge.
    env.ok("files.delete", json!({ "id": file_id }));
    let trash = env.ok("trash.list", json!({}));
    assert_eq!(trash[0]["objectId"], file_id);
    env.ok("trash.restore", json!({ "id": trash[0]["id"] }));
    assert_eq!(
        env.ok("files.list", json!({}))["files"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    env.ok("files.delete", json!({ "id": file_id }));
    let trash = env.ok("trash.list", json!({}));
    let asset_path = listing["files"][0]["asset"]["path"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(std::path::Path::new(&asset_path).exists());
    env.ok("trash.purge", json!({ "id": trash[0]["id"] }));
    assert!(
        env.ok("trash.list", json!({}))
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        !std::path::Path::new(&asset_path).exists(),
        "purge removes the managed file"
    );
}

#[test]
fn undo_never_overwrites_later_work_by_someone_else() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let src = env.write_file("a.txt", b"x");
    let f = env.ok("files.add", json!({ "paths": [src] }));
    let id = f[0]["id"].as_str().unwrap();
    env.ok("files.rename", json!({ "id": id, "name": "Mine" }));
    // A collaborator (Editor) renames the same file afterwards.
    let other = env.other_user(Role::Editor);
    env.call_as(
        &other,
        "files.rename",
        json!({ "id": id, "name": "Theirs" }),
    )
    .unwrap();
    let err = env.err("history.undo", json!({}));
    assert_eq!(err, "conflict.undo");
    assert_eq!(
        env.ok("files.list", json!({}))["files"][0]["displayName"],
        "Theirs"
    );
}

#[test]
fn permissions_are_enforced_before_any_mutation() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let viewer = env.actor_with_role(Role::Viewer);
    let src = env.write_file("a.txt", b"x");
    let err = env
        .call_as(&viewer, "files.add", json!({ "paths": [src] }))
        .unwrap_err();
    assert_eq!(err.code.0, "permission.denied");
    assert_eq!(
        env.ok("files.list", json!({}))["files"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    let commenter = env.actor_with_role(Role::Commenter);
    assert!(
        env.call_as(
            &commenter,
            "project.set_status",
            json!({ "status": "Writing" })
        )
        .is_err()
    );
    let editor = env.actor_with_role(Role::Editor);
    assert!(
        env.call_as(
            &editor,
            "project.set_status",
            json!({ "status": "Writing" })
        )
        .is_err(),
        "settings are Owner-only"
    );
}

#[test]
fn status_is_manual_and_persists_across_restart() {
    let env = TestEnv::with_project("Film", "Feature Film");
    env.ok("project.set_status", json!({ "status": "Pre-Production" }));
    let path = env.project_path();
    let env = env.restart();
    let opened = env.reopen_project(&path);
    assert_eq!(opened["project"]["status"], "Pre-Production");
    assert!(
        opened["recovery"].is_null(),
        "clean close offers no recovery"
    );
}

#[test]
fn crash_offers_recovery_and_keeps_latest_autosave() {
    let env = TestEnv::with_project("Film", "Feature Film");
    env.ok("project.save", json!({}));
    env.ok(
        "project.update_settings",
        json!({ "logline": "A signalman hears a train that isn't there." }),
    );
    let path = env.project_path();
    let env = env.crash_and_restart();
    let opened = env.reopen_project(&path);
    assert!(
        !opened["recovery"].is_null(),
        "unclean shutdown must offer recovery"
    );
    assert_eq!(opened["recovery"]["hasCheckpoint"], true);
    // Autosave already persisted the logline.
    assert_eq!(
        opened["project"]["logline"],
        "A signalman hears a train that isn't there."
    );
    // Choosing the last confirmed save restores the checkpoint state, keeping a safety copy.
    let restored = env.ok(
        "project.resolve_recovery",
        json!({ "choice": "checkpoint" }),
    );
    assert!(restored["project"]["logline"].is_null());
    let backups = std::fs::read_dir(std::path::Path::new(&path).join("backups"))
        .unwrap()
        .count();
    assert!(
        backups >= 1,
        "the replaced state is kept as a safety backup"
    );
}

#[test]
fn project_is_single_writer() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let path = env.project_path();
    let second = TestEnv::new();
    assert_eq!(
        second.err("project.open", json!({ "path": path })),
        "project_format.in_use"
    );
}

#[test]
fn duplicate_creates_independent_identity() {
    let env = TestEnv::with_project("Original", "Short Film");
    let id = env.ok("project.current", json!({}))["id"]
        .as_str()
        .unwrap()
        .to_string();
    let copy = env.ok("project.duplicate", json!({ "projectId": id }));
    assert_ne!(copy["projectId"], id);
    assert_eq!(copy["title"], "Copy of Original");
    let opened = env.ok("project.open", json!({ "path": copy["path"] }));
    assert_eq!(opened["project"]["title"], "Copy of Original");
    assert_ne!(opened["project"]["id"], id);
}

#[test]
fn archive_hides_from_recent_list() {
    let env = TestEnv::with_project("Old", "Short Film");
    let id = env.ok("project.current", json!({}))["id"]
        .as_str()
        .unwrap()
        .to_string();
    env.ok(
        "project.set_archived",
        json!({ "projectId": id, "archived": true }),
    );
    assert!(
        env.ok("project.list_recent", json!({}))
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        env.ok("project.list_recent", json!({ "includeArchived": true }))
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn opening_a_non_project_folder_fails_safely() {
    let env = TestEnv::new();
    let not_project = env.dir.path().join("random");
    std::fs::create_dir_all(&not_project).unwrap();
    assert!(
        env.err("project.open", json!({ "path": not_project }))
            .starts_with("project_format")
    );
    assert_eq!(
        env.err(
            "project.open",
            json!({ "path": env.dir.path().join("missing") })
        ),
        "not_found.project_folder"
    );
}

// ------------------------------------------------ Home / lifecycle extensions

/// Create a project, close it, and return (env, project id, path).
fn closed_project(title: &str) -> (TestEnv, String, String) {
    let env = TestEnv::with_project(title, "Short Film");
    let id = env.ok("project.current", json!({}))["id"]
        .as_str()
        .unwrap()
        .to_string();
    let path = env.project_path();
    env.ok("project.close", json!({}));
    (env, id, path)
}

#[test]
fn fsd_3_5_rename_closed_project_updates_title_manifest_and_home() {
    let (env, id, path) = closed_project("Working Title");
    let r = env.ok(
        "project.rename",
        json!({ "projectId": id, "title": "  Salt Road " }),
    );
    assert_eq!(r["title"], "Salt Road");
    assert_eq!(
        env.ok("project.list_recent", json!({}))[0]["title"],
        "Salt Road"
    );
    let manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(std::path::Path::new(&path).join("openframe.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["title"], "Salt Road");
    assert_eq!(
        env.err("project.rename", json!({ "projectId": id, "title": " " })),
        "validation.required"
    );
    let opened = env.reopen_project(&path);
    assert_eq!(
        opened["project"]["title"], "Salt Road",
        "the project database carries the new title"
    );
    let activity = env.ok("history.activity", json!({}));
    assert!(
        activity
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["action"] == "project.rename")
    );
}

/// Application Home refreshes its list from this event; without it the card kept the old
/// title until Home was reopened.
#[test]
fn rename_closed_project_emits_a_project_list_change_event() {
    use openframe_application::events::{AppEvent, StoreKind};
    let (env, id, _) = closed_project("Harbour Lights");
    env.sink.events.lock().clear();
    env.ok(
        "project.rename",
        json!({ "projectId": id, "title": "Harbour Lights Redux" }),
    );
    let events = env.sink.events.lock();
    assert!(
        events.iter().any(|e| matches!(
            e,
            AppEvent::DataChanged { store: StoreKind::Project, tables, ids, origin }
                if tables.iter().any(|t| t == "sys_recent_project")
                    && ids.contains(&id)
                    && origin == "local"
        )),
        "rename must tell Application Home that the project list changed: {events:?}"
    );
    drop(events);
    env.sink.events.lock().clear();
    assert_eq!(
        env.err("project.rename", json!({ "projectId": id, "title": " " })),
        "validation.required"
    );
    assert!(
        env.sink.events.lock().is_empty(),
        "a refused rename changes nothing and emits nothing"
    );
}

/// Pin, Unpin, Archive, Restore, Duplicate and Remove from List change the same Home list; without the
/// event Application Home showed the old list until it was reopened (found by the WindowsHTML
/// capture on 2026-10-21).
#[test]
fn pin_archive_duplicate_and_remove_emit_a_project_list_change_event() {
    use openframe_application::events::{AppEvent, StoreKind};
    let (env, id, _) = closed_project("Monsoon Ferry");
    let calls = [
        (
            "project.set_pinned",
            json!({ "projectId": id, "pinned": true }),
        ),
        (
            "project.set_pinned",
            json!({ "projectId": id, "pinned": false }),
        ),
        (
            "project.set_archived",
            json!({ "projectId": id, "archived": true }),
        ),
        (
            "project.set_archived",
            json!({ "projectId": id, "archived": false }),
        ),
    ];
    for (op, args) in calls {
        env.sink.events.lock().clear();
        env.ok(op, args.clone());
        let events = env.sink.events.lock();
        assert!(
            events.iter().any(|e| matches!(
                e,
                AppEvent::DataChanged { store: StoreKind::Project, tables, ids, origin }
                    if tables.iter().any(|t| t == "sys_recent_project")
                        && ids.contains(&id)
                        && origin == "local"
            )),
            "{op} {args} must tell Application Home that the project list changed: {events:?}"
        );
    }
    env.sink.events.lock().clear();
    let copy = env.ok("project.duplicate", json!({ "projectId": id }));
    let copy_id = copy["projectId"].as_str().unwrap().to_string();
    assert!(
        env.sink.events.lock().iter().any(|e| matches!(
            e,
            AppEvent::DataChanged { tables, ids, .. }
                if tables.iter().any(|t| t == "sys_recent_project") && ids.contains(&copy_id)
        )),
        "a duplicate must appear on Application Home at once"
    );
    env.sink.events.lock().clear();
    env.ok("project.remove_recent", json!({ "projectId": copy_id }));
    assert!(
        env.sink.events.lock().iter().any(|e| matches!(
            e,
            AppEvent::DataChanged { tables, ids, .. }
                if tables.iter().any(|t| t == "sys_recent_project") && ids.contains(&copy_id)
        )),
        "Remove from List must take the card off Application Home at once"
    );
}

#[test]
fn rename_open_project_is_undoable_and_updates_home_list() {
    let env = TestEnv::with_project("Draft Title", "Feature Film");
    let id = env.ok("project.current", json!({}))["id"]
        .as_str()
        .unwrap()
        .to_string();
    env.ok(
        "project.rename",
        json!({ "projectId": id, "title": "BLACK RAIN" }),
    );
    assert_eq!(env.ok("project.current", json!({}))["title"], "BLACK RAIN");
    assert_eq!(
        env.ok("project.list_recent", json!({}))[0]["title"],
        "BLACK RAIN"
    );
    let step = env.undo();
    assert!(step["label"].as_str().unwrap().contains("Renamed project"));
    assert_eq!(env.ok("project.current", json!({}))["title"], "Draft Title");
}

#[test]
fn pin_unpin_orders_home_and_survives_reopen() {
    let (env, first, _) = closed_project("First");
    env.ok(
        "project.create",
        json!({ "title": "Second", "projectType": "Feature Film" }),
    );
    env.ok("project.close", json!({}));
    let recents = env.ok("project.list_recent", json!({}));
    assert_eq!(recents[0]["title"], "Second", "most recently opened first");
    let pinned = env.ok(
        "project.set_pinned",
        json!({ "projectId": first, "pinned": true }),
    );
    assert_eq!(pinned["pinned"], true);
    let recents = env.ok("project.list_recent", json!({}));
    assert_eq!(recents[0]["title"], "First", "pinned projects list first");
    // Opening a pinned project keeps it pinned.
    env.reopen_project(recents[0]["path"].as_str().unwrap());
    env.ok("project.close", json!({}));
    assert_eq!(env.ok("project.list_recent", json!({}))[0]["pinned"], true);
    env.ok(
        "project.set_pinned",
        json!({ "projectId": first, "pinned": false }),
    );
    let recents_after = env.ok("project.list_recent", json!({}));
    assert!(
        recents_after
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["pinned"] == false)
    );
    assert_eq!(
        recents_after[0]["title"], "First",
        "reopened last, so first once unpinned"
    );
    assert_eq!(
        env.err(
            "project.set_pinned",
            json!({ "projectId": "missing", "pinned": true })
        ),
        "not_found.project"
    );
    assert!(
        recents[0]["modifiedAt"].is_number(),
        "Home shows when the project last changed"
    );
}

#[test]
fn fsd_4_5_archive_of_closed_project_is_stored_in_the_project() {
    let (env, id, path) = closed_project("Old");
    env.ok(
        "project.set_archived",
        json!({ "projectId": id, "archived": true }),
    );
    // Opening an archived project (from the Archived view) must not silently restore it.
    let opened = env.reopen_project(&path);
    assert_eq!(opened["project"]["archived"], true);
    env.ok("project.close", json!({}));
    assert!(
        env.ok("project.list_recent", json!({}))
            .as_array()
            .unwrap()
            .is_empty()
    );
    // Restore (FSD §4.6) returns it to active projects with its content unaltered.
    env.ok(
        "project.set_archived",
        json!({ "projectId": id, "archived": false }),
    );
    assert_eq!(
        env.ok("project.list_recent", json!({}))[0]["archived"],
        false
    );
}

#[test]
fn closed_project_edits_respect_the_single_writer_lock() {
    let (env, id, path) = closed_project("Shared");
    let other = TestEnv::new();
    other.reopen_project(&path);
    assert_eq!(
        env.err(
            "project.rename",
            json!({ "projectId": id, "title": "Mine" })
        ),
        "project_format.in_use"
    );
    drop(other);
}

#[test]
fn first_run_display_name_is_confirmed_once() {
    let env = TestEnv::new();
    assert_eq!(env.ok("app.info", json!({}))["displayNameSet"], false);
    assert_eq!(
        env.err("app.set_display_name", json!({ "displayName": "  " })),
        "validation.required"
    );
    env.ok(
        "app.set_display_name",
        json!({ "displayName": "Nisha Verma" }),
    );
    let env = env.restart();
    let info = env.ok("app.info", json!({}));
    assert_eq!(info["displayNameSet"], true);
    assert_eq!(info["profile"]["displayName"], "Nisha Verma");
    let lan = env.other_user(Role::Owner);
    assert!(
        env.call_as(
            &lan,
            "app.set_display_name",
            json!({ "displayName": "Mallory" })
        )
        .is_err()
    );
}

#[test]
fn fsd_6_2_continue_remembers_last_location_per_workspace() {
    let env = TestEnv::with_project("Film", "Feature Film");
    env.ok("project.set_last_location", json!({ "location": { "workspace": "story", "label": "Act 2 · First Investigation", "sceneCardId": "c1" } }));
    env.ok("project.set_last_location", json!({ "location": { "workspace": "screenplay", "label": "Scene 12 · Draft 6", "sceneId": "s1" } }));
    env.ok(
        "project.set_last_location",
        json!({ "location": { "workspace": "story", "label": "Act 3", "sceneCardId": "c2" } }),
    );
    let home = env.ok("project.home", json!({}));
    let locs = home["continueLocations"].as_array().unwrap();
    assert_eq!(locs.len(), 2, "one entry per workspace");
    assert_eq!(locs[0]["workspace"], "story");
    assert_eq!(
        locs[0]["sceneCardId"], "c2",
        "the latest location in that workspace"
    );
    assert_eq!(home["continueLocation"]["label"], "Act 3");
    assert!(home["openedAt"].as_i64().unwrap() > 0);
    assert_eq!(
        env.err("project.set_last_location", json!({ "location": "story" })),
        "validation.invalid_input"
    );
    // Remembered per user across restarts.
    let path = env.project_path();
    let env = env.restart();
    env.reopen_project(&path);
    assert_eq!(
        env.ok("project.home", json!({}))["continueLocations"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn fsd_6_3_home_stays_empty_until_real_content_exists() {
    let env = TestEnv::with_project("Film", "Feature Film");
    env.ok("project.set_status", json!({ "status": "Writing" }));
    assert_eq!(
        env.ok("project.home", json!({}))["isEmpty"],
        true,
        "settings changes are not content"
    );
    env.ok("notes.create", json!({ "title": "First note" }));
    assert_eq!(env.ok("project.home", json!({}))["isEmpty"], false);
}

#[test]
fn activity_entries_offer_open_only_for_live_objects() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let src = env.write_file("pitch.pdf", b"%PDF");
    let f = env.ok("files.add", json!({ "paths": [src] }));
    let act = env.ok("history.activity", json!({}));
    let entry = act
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["action"] == "files.add")
        .unwrap()
        .clone();
    // files.add has no single target; rename does.
    assert!(entry["nav"].is_null());
    env.ok(
        "files.rename",
        json!({ "id": f[0]["id"], "name": "Pitch deck" }),
    );
    let act = env.ok("history.activity", json!({}));
    assert_eq!(act[0]["nav"]["workspace"], "files");
    env.ok("files.delete", json!({ "id": f[0]["id"] }));
    let act = env.ok("history.activity", json!({}));
    assert!(
        act.as_array().unwrap().iter().all(|a| a["nav"].is_null()),
        "deleted objects never offer Open"
    );
}

#[test]
fn quitting_while_a_close_is_under_way_waits_for_it_and_offers_no_recovery() {
    // Seen in E2E spec 76: the window showed Application Home as soon as the close had taken the
    // project, the app was quit straight away, and shutdown found no project and exited while the
    // close was still writing its checkpoint, so the crash marker stayed and the next open offered
    // recovery after a clean close.
    use std::sync::mpsc;
    use std::time::Duration;
    let env = TestEnv::with_project("Film", "Feature Film");
    let path = env.project_path();
    let session = env.core.project().unwrap();
    let marker = session.layout.session_marker();
    assert!(marker.exists());
    // Keep the writer busy so the close has to wait at its checkpoint.
    let (held, wait) = mpsc::channel();
    let holder = {
        let s = session.clone();
        std::thread::spawn(move || {
            s.store.with_writer(|_| {
                held.send(()).unwrap();
                std::thread::sleep(Duration::from_millis(400));
                Ok(())
            })
        })
    };
    wait.recv().unwrap();
    let closer = {
        let core = env.core.clone();
        std::thread::spawn(move || core.dispatch(&core.local_actor(), "project.close", json!({})))
    };
    while env.core.project().is_ok() {
        std::thread::sleep(Duration::from_millis(5));
    }
    // Quit now, as the person does when Home appears.
    env.core.shutdown();
    assert!(
        !marker.exists(),
        "shutdown returned before the close cleared the crash marker"
    );
    holder.join().unwrap().unwrap();
    closer.join().unwrap().unwrap();
    drop(session);
    let env = env.restart();
    let opened = env.reopen_project(&path);
    assert!(
        opened["recovery"].is_null(),
        "a clean close offers no recovery"
    );
}

#[test]
fn when_no_project_is_open_any_more_the_close_is_already_clean() {
    // Seen in E2E spec 76: closing waited for the AI index worker after the project had already
    // stopped being the open one, so Application Home showed while the crash marker was still on
    // disk; a process that ended then offered recovery after a clean close.
    use std::sync::mpsc;
    use std::time::Duration;
    let env = TestEnv::with_project("Film", "Feature Film");
    let session = env.core.project().unwrap();
    let marker = session.layout.session_marker();
    let (held, wait) = mpsc::channel();
    let holder = {
        let s = session.clone();
        std::thread::spawn(move || {
            s.store.with_writer(|_| {
                held.send(()).unwrap();
                std::thread::sleep(Duration::from_millis(400));
                Ok(())
            })
        })
    };
    wait.recv().unwrap();
    let closer = {
        let core = env.core.clone();
        std::thread::spawn(move || core.dispatch(&core.local_actor(), "project.close", json!({})))
    };
    while env.core.project().is_ok() {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(
        !marker.exists(),
        "the app said no project is open while the close was still unfinished"
    );
    holder.join().unwrap().unwrap();
    closer.join().unwrap().unwrap();
}

#[test]
fn fsd_44_recovery_state_reports_checkpoint_and_offer() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let st = env.ok("project.recovery_state", json!({}));
    assert!(st["offer"].is_null());
    env.ok("project.save", json!({}));
    let st = env.ok("project.recovery_state", json!({}));
    assert_eq!(st["hasCheckpoint"], true);
    assert!(st["checkpointAt"].is_number());
    env.ok("notes.create", json!({ "title": "after save" }));
    let path = env.project_path();
    let env = env.crash_and_restart();
    env.reopen_project(&path);
    let st = env.ok("project.recovery_state", json!({}));
    assert!(!st["offer"].is_null(), "recovery offered after a crash");
    assert!(st["lastChangeAt"].is_number());
    // "Open Recovery" keeps the latest autosaved state and clears the offer.
    env.ok("project.resolve_recovery", json!({ "choice": "latest" }));
    assert!(env.ok("project.recovery_state", json!({}))["offer"].is_null());
    assert_eq!(env.ok("notes.list", json!({}))[0]["title"], "after save");
}

#[test]
fn project_settings_include_project_notes() {
    let env = TestEnv::with_project("Film", "Feature Film");
    env.ok(
        "project.update_settings",
        json!({ "title": "BLACK RAIN", "genre": "Crime Thriller", "settings": { "projectNotes": "Shoot window: June 2027." } }),
    );
    let s = env.ok("project.get_settings", json!({}));
    assert_eq!(s["project"]["title"], "BLACK RAIN");
    assert_eq!(s["project"]["genre"], "Crime Thriller");
    assert_eq!(s["projectNotes"], "Shoot window: June 2027.");
    let long = "x".repeat(20_001);
    assert_eq!(
        env.err(
            "project.update_settings",
            json!({ "settings": { "projectNotes": long } })
        ),
        "validation.invalid_input"
    );
    let editor = env.actor_with_role(Role::Editor);
    assert!(
        env.call_as(&editor, "project.update_settings", json!({ "title": "X" }))
            .is_err(),
        "settings are Owner-only"
    );
}

#[test]
fn fsd_52_5_recently_deleted_explains_where_items_were_and_unassigned_restore() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let folder = env.ok("files.create_folder", json!({ "name": "Contracts" }));
    let src = env.write_file("permit.docx", b"doc");
    let f = env.ok(
        "files.add",
        json!({ "paths": [src], "folderId": folder["id"] }),
    );
    env.ok("files.delete", json!({ "id": f[0]["id"] }));
    let trash = env.ok("project.deleted_items", json!({}));
    assert_eq!(trash[0]["typeLabel"], "File");
    assert_eq!(trash[0]["wasIn"], "Files · Folder “Contracts”");
    assert_eq!(trash[0]["parentMissing"], false);
    assert!(trash[0]["deletedByName"].is_string());
    // The folder is deleted too: restoring the file puts it at the top level, and the UI is told.
    env.ok("files.delete_folder", json!({ "id": folder["id"] }));
    let trash = env.ok("project.deleted_items", json!({}));
    let file_row = trash
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["objectType"] == "project_file")
        .unwrap()
        .clone();
    assert_eq!(file_row["parentMissing"], true);
    env.ok("trash.restore", json!({ "id": file_row["id"] }));
    let listing = env.ok("files.list", json!({}));
    assert!(
        listing["files"][0]["folderId"].is_null(),
        "restored to the top level (Unassigned)"
    );
    // Permanent delete is not undoable and needs the PermanentDelete capability.
    let trash = env.ok("project.deleted_items", json!({}));
    let editor = env.actor_with_role(Role::Editor);
    assert_eq!(
        env.call_as(&editor, "trash.purge", json!({ "id": trash[0]["id"] }))
            .unwrap_err()
            .code
            .0,
        "permission.denied"
    );
}

#[test]
fn fsd_40_files_export_copy_and_relink() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let src = env.write_file("schedule.xlsx", b"sheet-v1");
    let linked = env.ok(
        "files.add",
        json!({ "paths": [src.clone()], "mode": "link" }),
    );
    assert_eq!(linked[0]["asset"]["storageMode"], "external");
    let dest = env.dir.path().join("exports").join("copy.xlsx");
    std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
    env.ok(
        "files.export_copy",
        json!({ "id": linked[0]["id"], "destPath": dest }),
    );
    assert_eq!(std::fs::read(&dest).unwrap(), b"sheet-v1");
    assert_eq!(
        env.err(
            "files.export_copy",
            json!({ "id": linked[0]["id"], "destPath": env.dir.path().join("nope").join("x.xlsx") })
        ),
        "not_found.folder"
    );
    // The linked source goes missing → unavailable; relink keeps the same identity.
    std::fs::remove_file(&src).unwrap();
    assert_eq!(
        env.ok("files.list", json!({}))["files"][0]["asset"]["available"],
        false
    );
    assert_eq!(
        env.err(
            "files.export_copy",
            json!({ "id": linked[0]["id"], "destPath": dest })
        ),
        "not_found.file"
    );
    let moved = env.write_file("moved/schedule.xlsx", b"sheet-v2");
    let relinked = env.ok(
        "files.relink",
        json!({ "id": linked[0]["id"], "path": moved }),
    );
    assert_eq!(relinked["id"], linked[0]["id"]);
    assert_eq!(relinked["asset"]["available"], true);
    let viewer = env.actor_with_role(Role::Viewer);
    assert!(
        env.call_as(
            &viewer,
            "files.export_copy",
            json!({ "id": linked[0]["id"], "destPath": dest })
        )
        .is_err()
    );
}

#[test]
fn corrupt_database_is_refused_without_modification() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let path = env.project_path();
    let env = env.restart();
    let db = std::path::Path::new(&path).join("project.sqlite");
    let mut bytes = std::fs::read(&db).unwrap();
    for b in bytes.iter_mut().skip(4096).take(4096) {
        *b = 0xAB;
    }
    std::fs::write(&db, &bytes).unwrap();
    let before = std::fs::read(&db).unwrap();
    let code = env.err("project.open", json!({ "path": path }));
    assert!(
        code.starts_with("project_format") || code.starts_with("storage"),
        "{code}"
    );
    assert_eq!(
        std::fs::read(&db).unwrap(),
        before,
        "a damaged project must not be modified"
    );
}
