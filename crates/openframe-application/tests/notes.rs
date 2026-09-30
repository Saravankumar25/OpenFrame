//! Notes, Tasks and Templates (FSD §59, §106, §113, §158, §159) through the
//! public operation registry.

use openframe_domain::Role;
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

fn ids(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x["id"].as_str().unwrap().to_string())
        .collect()
}

/// Insert a Location row directly (the Production module owns its commands).
fn insert_location(env: &TestEnv, name: &str) -> String {
    let id = openframe_domain::new_id();
    let now = openframe_domain::now_ms();
    env.core
        .project()
        .unwrap()
        .store
        .with_writer(|c| {
            c.execute(
                "INSERT INTO location(id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
                rusqlite::params![id, name, now],
            )?;
            Ok(())
        })
        .unwrap();
    id
}

// ----------------------------------------------------------------- notes

#[test]
fn fsd_106_note_create_edit_pin_and_search() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = env.ok(
        "notes.create",
        json!({ "title": "Location meeting notes", "body": "Agreed a 5 am access time." }),
    );
    let b = env.ok(
        "notes.create",
        json!({ "body": "Second camera?\nRain machine or wait?" }),
    );
    assert_eq!(b["title"], Value::Null, "untitled notes are valid");
    // Pinned notes list first; otherwise most recently edited first.
    env.ok("notes.set_pinned", json!({ "id": a["id"], "pinned": true }));
    let list = env.ok("notes.list", json!({}));
    assert_eq!(
        ids(&list),
        vec![a["id"].as_str().unwrap(), b["id"].as_str().unwrap()]
    );
    assert_eq!(list[0]["pinned"], true);

    let edited = env.ok(
        "notes.update",
        json!({ "id": b["id"], "title": "Things to decide", "expectedRev": b["rev"] }),
    );
    assert_eq!(edited["title"], "Things to decide");
    assert_eq!(edited["body"], "Second camera?\nRain machine or wait?");
    // A stale edit is refused rather than overwriting newer text.
    assert_eq!(
        env.err(
            "notes.update",
            json!({ "id": b["id"], "body": "x", "expectedRev": b["rev"] })
        ),
        "conflict.stale"
    );

    let hits = env.ok("search.query", json!({ "text": "access time" }));
    assert_eq!(hits[0]["entityId"], a["id"]);
    assert_eq!(hits[0]["nav"]["workspace"], "notes");
    // Plain text filter in the list.
    assert_eq!(
        env.ok("notes.list", json!({ "text": "rain machine" }))
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn fsd_106_note_edits_are_undoable_and_coalesced() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let n = env.ok("notes.create", json!({ "title": "Plan", "body": "" }));
    env.ok("notes.update", json!({ "id": n["id"], "body": "A" }));
    env.ok("notes.update", json!({ "id": n["id"], "body": "AB" }));
    env.ok("notes.update", json!({ "id": n["id"], "body": "ABC" }));
    // Typing coalesces into one undo step.
    env.undo();
    assert_eq!(env.ok("notes.get", json!({ "id": n["id"] }))["body"], "");
    env.redo();
    assert_eq!(env.ok("notes.get", json!({ "id": n["id"] }))["body"], "ABC");
    env.undo();
    env.undo();
    assert!(
        env.ok("notes.list", json!({}))
            .as_array()
            .unwrap()
            .is_empty(),
        "undo removes the created note"
    );
}

#[test]
fn fsd_52_note_delete_restore_and_purge() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let n = env.ok(
        "notes.create",
        json!({ "title": "Producer call", "body": "Budget approved" }),
    );
    env.ok("notes.delete", json!({ "id": n["id"] }));
    assert!(
        env.ok("notes.list", json!({}))
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        env.ok("search.query", json!({ "text": "producer" }))
            .as_array()
            .unwrap()
            .is_empty()
    );
    let trash = env.ok("project.deleted_items", json!({}));
    assert_eq!(trash[0]["typeLabel"], "Note");
    assert_eq!(trash[0]["wasIn"], "Notes & Tasks");
    assert_eq!(trash[0]["title"], "Producer call");
    env.ok("trash.restore", json!({ "id": trash[0]["id"] }));
    assert_eq!(
        env.ok("notes.list", json!({}))[0]["id"],
        n["id"],
        "restore keeps the same identity"
    );
    env.ok("notes.delete", json!({ "id": n["id"] }));
    let trash = env.ok("trash.list", json!({}));
    env.ok("trash.purge", json!({ "id": trash[0]["id"] }));
    assert!(
        env.ok("trash.list", json!({}))
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        env.err("notes.get", json!({ "id": n["id"] })),
        "not_found.note"
    );
}

#[test]
fn notes_permissions() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let n = env.ok("notes.create", json!({ "title": "Mine" }));
    for role in [Role::Viewer, Role::Commenter] {
        let a = env.actor_with_role(role);
        assert_eq!(
            env.call_as(&a, "notes.create", json!({ "title": "x" }))
                .unwrap_err()
                .code
                .0,
            "permission.denied"
        );
        assert_eq!(
            env.call_as(&a, "notes.update", json!({ "id": n["id"], "body": "x" }))
                .unwrap_err()
                .code
                .0,
            "permission.denied"
        );
        assert_eq!(
            env.call_as(&a, "notes.delete", json!({ "id": n["id"] }))
                .unwrap_err()
                .code
                .0,
            "permission.denied"
        );
        assert!(
            env.call_as(&a, "notes.list", json!({})).is_ok(),
            "viewers can read notes"
        );
    }
    assert_eq!(env.ok("notes.get", json!({ "id": n["id"] }))["body"], "");
}

#[test]
fn notes_and_tasks_persist_across_restart() {
    let env = TestEnv::with_project("Film", "Feature Film");
    env.ok(
        "notes.create",
        json!({ "title": "Keep me", "pinned": true }),
    );
    env.ok("tasks.create", json!({ "title": "Get costume reference" }));
    let path = env.project_path();
    let env = env.restart();
    env.reopen_project(&path);
    assert_eq!(env.ok("notes.list", json!({}))[0]["title"], "Keep me");
    assert_eq!(
        env.ok("tasks.list", json!({}))[0]["title"],
        "Get costume reference"
    );
}

// ----------------------------------------------------------------- tasks

#[test]
fn fsd_158_task_requires_title_and_validates_owner_and_relation() {
    let env = TestEnv::with_project("Film", "Feature Film");
    assert_eq!(
        env.err("tasks.create", json!({ "title": "  " })),
        "validation.required"
    );
    assert_eq!(
        env.err(
            "tasks.create",
            json!({ "title": "X", "ownerUserId": "nobody" })
        ),
        "validation.owner"
    );
    let loc = insert_location(&env, "Old Railway Station");
    assert_eq!(
        env.err(
            "tasks.create",
            json!({ "title": "X", "related": { "targetType": "project", "targetId": loc } })
        ),
        "validation.invalid_input",
        "only supported object types can be related"
    );
    assert_eq!(
        env.err("tasks.create", json!({ "title": "X", "related": { "targetType": "location", "targetId": openframe_domain::new_id() } })),
        "not_found.related_item"
    );
    assert!(
        env.ok("tasks.list", json!({}))
            .as_array()
            .unwrap()
            .is_empty(),
        "nothing created by failed requests"
    );
}

#[test]
fn fsd_158_done_never_changes_related_object_and_reopen_works() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let me = env.ok("app.info", json!({}))["profile"]["userId"]
        .as_str()
        .unwrap()
        .to_string();
    let loc = insert_location(&env, "Police station");
    let due = 1_800_000_000_000_i64;
    let t = env.ok(
        "tasks.create",
        json!({ "title": "Confirm police station", "dueAt": due, "ownerUserId": me,
                "related": { "targetType": "location", "targetId": loc } }),
    );
    assert_eq!(t["status"], "Open");
    assert_eq!(t["dueAt"], due);
    assert_eq!(t["related"]["typeLabel"], "Location");
    assert_eq!(t["related"]["title"], "Police station");
    assert_eq!(t["related"]["available"], true);
    assert!(t["ownerName"].is_string());

    let before: (i64, i64) = env
        .core
        .project()
        .unwrap()
        .store
        .read(|c| {
            Ok(c.query_row(
                "SELECT rev, updated_at FROM location WHERE id=?1",
                [&loc],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?)
        })
        .unwrap();
    let done = env.ok("tasks.set_done", json!({ "id": t["id"], "done": true }));
    assert_eq!(done["status"], "Done");
    let after: (i64, i64) = env
        .core
        .project()
        .unwrap()
        .store
        .read(|c| {
            Ok(c.query_row(
                "SELECT rev, updated_at FROM location WHERE id=?1",
                [&loc],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?)
        })
        .unwrap();
    assert_eq!(
        before, after,
        "completing a task must not touch the related object"
    );

    // Filters by status/owner.
    assert!(
        env.ok("tasks.list", json!({ "status": "Open" }))
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        env.ok("tasks.list", json!({ "status": "Done", "ownerUserId": me }))
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let reopened = env.ok("tasks.set_done", json!({ "id": t["id"], "done": false }));
    assert_eq!(reopened["status"], "Open");
    env.undo();
    assert_eq!(
        env.ok("tasks.list", json!({}))[0]["status"],
        "Done",
        "Mark Done / Reopen is undoable"
    );
}

#[test]
fn fsd_158_task_edit_clear_fields_and_related_deleted_is_reported() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let loc = insert_location(&env, "Harbour");
    let t = env.ok("tasks.create", json!({ "title": "Scout", "dueAt": 1_800_000_000_000_i64, "notes": "bring camera",
                                            "related": { "targetType": "location", "targetId": loc } }));
    let e = env.ok(
        "tasks.update",
        json!({ "id": t["id"], "title": "Scout harbour", "clearDue": true, "notes": "" }),
    );
    assert_eq!(e["title"], "Scout harbour");
    assert!(e["dueAt"].is_null());
    assert!(e["notes"].is_null());
    // The related object disappears (deleted elsewhere): the task keeps the reference and says so.
    env.core
        .project()
        .unwrap()
        .store
        .with_writer(|c| {
            c.execute("UPDATE location SET deleted_at=1 WHERE id=?1", [&loc])?;
            Ok(())
        })
        .unwrap();
    let t2 = &env.ok("tasks.list", json!({}))[0];
    assert_eq!(t2["related"]["available"], false);
    let cleared = env.ok(
        "tasks.update",
        json!({ "id": t["id"], "clearRelated": true }),
    );
    assert!(cleared["related"].is_null());
}

#[test]
fn fsd_158_task_delete_is_recoverable_and_permissions_enforced() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let t = env.ok("tasks.create", json!({ "title": "Rewrite Scene 22" }));
    let viewer = env.actor_with_role(Role::Viewer);
    assert_eq!(
        env.call_as(
            &viewer,
            "tasks.set_done",
            json!({ "id": t["id"], "done": true })
        )
        .unwrap_err()
        .code
        .0,
        "permission.denied"
    );
    assert_eq!(
        env.call_as(&viewer, "tasks.delete", json!({ "id": t["id"] }))
            .unwrap_err()
            .code
            .0,
        "permission.denied"
    );
    let hits = env.ok("search.query", json!({ "text": "rewrite scene" }));
    assert_eq!(hits[0]["entityId"], t["id"]);
    env.ok("tasks.delete", json!({ "id": t["id"] }));
    assert!(
        env.ok("tasks.list", json!({}))
            .as_array()
            .unwrap()
            .is_empty()
    );
    let trash = env.ok("project.deleted_items", json!({}));
    assert_eq!(trash[0]["typeLabel"], "Task");
    env.ok("trash.restore", json!({ "id": trash[0]["id"] }));
    assert_eq!(env.ok("tasks.list", json!({}))[0]["id"], t["id"]);
}

// ------------------------------------------------------------- templates

#[test]
fn fsd_59_builtin_templates_are_listed_and_read_only() {
    let env = TestEnv::new();
    let l = env.ok("templates.list", json!({}));
    assert_eq!(l["projectOpen"], false);
    let builtin = l["builtin"].as_array().unwrap();
    assert!(builtin.iter().any(|t| t["templateType"] == "call_sheet"));
    let id = builtin[0]["id"].clone();
    assert_eq!(
        env.err(
            "templates.update",
            json!({ "scope": "builtin", "id": id, "name": "x" })
        ),
        "validation.invalid_input"
    );
    assert_eq!(
        env.err("templates.delete", json!({ "scope": "builtin", "id": id })),
        "validation.invalid_input"
    );
    // Filter by type (used by document workspaces).
    let only = env.ok("templates.list", json!({ "templateType": "shot_list" }));
    assert!(
        only["builtin"]
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["templateType"] == "shot_list")
    );
}

#[test]
fn fsd_59_global_templates_work_without_a_project_and_persist() {
    let env = TestEnv::new();
    assert_eq!(
        env.err(
            "templates.create",
            json!({ "scope": "global", "templateType": "opera", "name": "X" })
        ),
        "validation.type"
    );
    assert_eq!(
        env.err(
            "templates.create",
            json!({ "scope": "global", "templateType": "call_sheet", "name": " " })
        ),
        "validation.required"
    );
    let g = env.ok(
        "templates.create",
        json!({ "scope": "global", "templateType": "call_sheet", "name": "My short-film call sheet",
                "content": { "sections": ["General call", "Cast"], "description": "Two sections" } }),
    );
    assert_eq!(g["scope"], "global");
    assert_eq!(g["description"], "Two sections");
    let renamed = env.ok(
        "templates.update",
        json!({ "scope": "global", "id": g["id"], "name": "Short call sheet" }),
    );
    assert_eq!(renamed["name"], "Short call sheet");
    let env = env.restart();
    let l = env.ok("templates.list", json!({}));
    assert_eq!(l["global"][0]["name"], "Short call sheet");
    assert_eq!(l["global"][0]["content"]["sections"][1], "Cast");
    // Changes arriving from someone else (an exchange package) can never change this computer's templates.
    let other = env.other_user(Role::Owner);
    assert_eq!(
        env.call_as(
            &other,
            "templates.delete",
            json!({ "scope": "global", "id": g["id"] })
        )
        .unwrap_err()
        .code
        .0,
        "permission.denied"
    );
    env.ok(
        "templates.delete",
        json!({ "scope": "global", "id": g["id"] }),
    );
    assert!(
        env.ok("templates.list", json!({}))["global"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn fsd_113_using_a_template_copies_it_independently() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let g = env.ok(
        "templates.create",
        json!({ "scope": "global", "templateType": "moodboard", "name": "Scouting", "content": { "sections": ["Light"] } }),
    );
    let used = env.ok(
        "templates.copy",
        json!({ "scope": "global", "id": g["id"], "toScope": "project" }),
    );
    assert_eq!(used["scope"], "project");
    assert_ne!(used["id"], g["id"], "the project copy has its own identity");
    // Editing the global template never changes the project copy (and vice versa).
    env.ok(
        "templates.update",
        json!({ "scope": "global", "id": g["id"], "content": { "sections": ["Changed"] } }),
    );
    let p = env.ok(
        "templates.get",
        json!({ "scope": "project", "id": used["id"] }),
    );
    assert_eq!(p["content"]["sections"][0], "Light");
    env.ok(
        "templates.update",
        json!({ "scope": "project", "id": used["id"], "name": "Scouting (film)" }),
    );
    assert_eq!(
        env.ok("templates.get", json!({ "scope": "global", "id": g["id"] }))["name"],
        "Scouting"
    );
    // Built-ins can be used too; project templates can be kept for every project.
    let b = env.ok("templates.list", json!({ "templateType": "call_sheet" }))["builtin"][0]["id"]
        .clone();
    let from_builtin = env.ok(
        "templates.copy",
        json!({ "scope": "builtin", "id": b, "toScope": "project", "name": "Day sheet" }),
    );
    assert_eq!(from_builtin["name"], "Day sheet");
    let kept = env.ok(
        "templates.copy",
        json!({ "scope": "project", "id": from_builtin["id"], "toScope": "global" }),
    );
    assert_eq!(kept["scope"], "global");
    assert_eq!(
        env.ok("templates.list", json!({}))["project"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn fsd_52_project_template_delete_is_recoverable_and_undoable() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let t = env.ok(
        "templates.create",
        json!({ "scope": "project", "templateType": "report", "name": "Wrap report" }),
    );
    let hits = env.ok("search.query", json!({ "text": "wrap report" }));
    assert_eq!(hits[0]["entityId"], t["id"]);
    env.ok(
        "templates.delete",
        json!({ "scope": "project", "id": t["id"] }),
    );
    assert!(
        env.ok("templates.list", json!({}))["project"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let trash = env.ok("project.deleted_items", json!({}));
    assert_eq!(trash[0]["typeLabel"], "Template");
    env.undo();
    assert_eq!(
        env.ok("templates.list", json!({}))["project"][0]["id"],
        t["id"]
    );
    let viewer = env.actor_with_role(Role::Viewer);
    assert_eq!(
        env.call_as(
            &viewer,
            "templates.create",
            json!({ "scope": "project", "templateType": "report", "name": "x" })
        )
        .unwrap_err()
        .code
        .0,
        "permission.denied"
    );
}
