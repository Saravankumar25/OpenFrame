//! Toolbox (contract C3) tests: universal read and proposal tools, strict
//! arguments, permission filtering, privacy and the approval boundary
//! (agentic spec §6, §26–§28, §39–§40).
//!
//! No model is involved: tools are called directly the way the agent loop calls
//! them. Proposal tools must only ever return a Change Set draft; the fixture
//! test then dispatches every proposed operation through the normal registry
//! (exactly what an approved Change Set does) to prove the operations are valid.

use openframe_application::modules::ai::scope::{self, AiScopeArgs, AiScopeKind, ResolvedScope};
use openframe_application::modules::ai::toolbox::{self, ToolStep};
use openframe_application::modules::ai::{ChangeSetDraft, catalog, change_set};
use openframe_domain::{Actor, AppError, Role};
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

fn whole(env: &TestEnv, actor: &Actor) -> ResolvedScope {
    let s = env.core.project().unwrap();
    s.store
        .read(|c| {
            scope::resolve(
                c,
                actor,
                &AiScopeArgs {
                    kind: AiScopeKind::WholeProject,
                    draft_id: None,
                    scene_id: None,
                    selection: Vec::new(),
                    shooting_day_id: None,
                    call_sheet_id: None,
                },
            )
        })
        .unwrap()
}

fn run_as(env: &TestEnv, actor: &Actor, tool: &str, args: Value) -> Result<ToolStep, AppError> {
    let scope = whole(env, actor);
    toolbox::run_tool_unlocked(&env.core, actor, &scope, "test request", tool, &args)
}

fn run(env: &TestEnv, tool: &str, args: Value) -> Result<ToolStep, AppError> {
    run_as(env, &env.actor(), tool, args)
}

#[track_caller]
fn read(env: &TestEnv, tool: &str, args: Value) -> String {
    match run(env, tool, args) {
        Ok(ToolStep::Output(o)) => {
            let mut s = o.content.clone();
            for d in &o.details {
                s.push('\n');
                s.push_str(d);
            }
            for it in &o.items {
                s.push_str(&format!(
                    "\n- {} {}",
                    it.label,
                    it.detail.clone().unwrap_or_default()
                ));
            }
            s
        }
        Ok(ToolStep::Proposal(_)) => panic!("{tool} is a read tool but returned a proposal"),
        Err(e) => panic!("{tool} failed: {} — {} ({:?})", e.code, e.message, e.detail),
    }
}

#[track_caller]
fn propose(env: &TestEnv, tool: &str, args: Value) -> ChangeSetDraft {
    match run(env, tool, args) {
        Ok(ToolStep::Proposal(d)) => d,
        Ok(ToolStep::Output(o)) => panic!("{tool} returned output, not a proposal: {}", o.content),
        Err(e) => panic!("{tool} failed: {} — {} ({:?})", e.code, e.message, e.detail),
    }
}

/// What an approved Change Set does: every operation through the registry, as the user.
#[track_caller]
fn apply(env: &TestEnv, tool: &str, args: Value) -> ChangeSetDraft {
    let d = propose(env, tool, args);
    assert!(!d.operations.is_empty(), "{tool}: empty proposal");
    for op in &d.operations {
        assert!(!op.op.starts_with("ai."), "{tool}: assistant op {}", op.op);
        if let Err(e) = env.call(&op.op, op.args.clone()) {
            panic!(
                "{tool}: operation {} was refused: {} — {} ({:?}) args={}",
                op.op, e.code, e.message, e.detail, op.args
            );
        }
    }
    d
}

fn count(env: &TestEnv, sql: &str) -> i64 {
    env.core
        .project()
        .unwrap()
        .store
        .read(|c| Ok(c.query_row(sql, [], |r| r.get(0))?))
        .unwrap()
}

/// A fingerprint of all canonical content: row counts and revision sums of every table.
fn fingerprint(env: &TestEnv) -> Vec<(String, i64, i64)> {
    env.core
        .project()
        .unwrap()
        .store
        .read(|c| {
            let mut st = c.prepare(
                "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE 'search_%' ORDER BY name",
            )?;
            let tables: Vec<String> = st.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
            let mut out = Vec::new();
            for t in tables {
                let has_rev = c
                    .prepare(&format!("PRAGMA table_info(\"{t}\")"))?
                    .query_map([], |r| r.get::<_, String>(1))?
                    .filter_map(|x| x.ok())
                    .any(|n| n == "rev");
                let n: i64 = c.query_row(&format!("SELECT count(*) FROM \"{t}\""), [], |r| r.get(0))?;
                let revs: i64 = if has_rev {
                    c.query_row(&format!("SELECT COALESCE(sum(rev),0) FROM \"{t}\""), [], |r| r.get(0))?
                } else {
                    0
                };
                out.push((t, n, revs));
            }
            Ok(out)
        })
        .unwrap()
}

// ------------------------------------------------------------------ fixture

struct Fx {
    draft: String,
    scenes: Vec<String>,
}

fn created(v: &Value) -> String {
    v["id"]
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| v.as_str().unwrap_or_default().to_string())
}

/// A small but complete project, built only through registry operations.
fn fixture(env: &TestEnv) -> Fx {
    let sp = env.ok(
        "screenplay.create",
        json!({ "title": "Railway", "draftName": "Draft 1" }),
    );
    let draft = sp["draft"]["id"].as_str().unwrap().to_string();
    let mut scenes = Vec::new();
    for (heading, lines) in [
        (
            "INT. RAILWAY STATION - NIGHT",
            vec![
                ("action", "Ravi waits by the red car."),
                ("character", "RAVI"),
                ("dialogue", "Where is Anjali?"),
            ],
        ),
        (
            "EXT. PLATFORM - DAY",
            vec![
                ("character", "ANJALI"),
                ("dialogue", "The train is late."),
                ("character", "RAVI"),
                ("dialogue", "Again."),
            ],
        ),
        (
            "INT. RAVI'S HOUSE - NIGHT",
            vec![
                ("action", "Ravi reads a letter."),
                ("character", "MEERA"),
                ("dialogue", "Ravi is gone."),
            ],
        ),
    ] {
        let r = env.ok(
            "screenplay.create_scene",
            json!({ "draftId": draft, "heading": heading }),
        );
        let id = r["createdIds"][0].as_str().unwrap().to_string();
        for (t, text) in lines {
            env.ok(
                "screenplay.insert_element",
                json!({ "sceneId": id, "elementType": t, "text": text }),
            );
        }
        scenes.push(id);
    }
    let act = created(&env.ok("story.create_act", json!({ "title": "Act One" })));
    env.ok("story.create_act", json!({ "title": "Act Two" }));
    env.ok(
        "story.create_sequence",
        json!({ "actId": act, "title": "Arrival" }),
    );
    env.ok(
        "story.create_card",
        json!({ "parent": { "parentType": "act", "parentId": act }, "shortDescription": "Ravi waits at the station", "sceneHeading": "INT. RAILWAY STATION - NIGHT" }),
    );
    env.ok("story.create_beat", json!({ "text": "Ravi loses hope" }));
    for name in ["Ravi", "Anjali", "Meera"] {
        env.ok("story.create_character", json!({ "name": name }));
    }
    env.ok("production.set_source", json!({ "draftId": draft }));
    env.ok(
        "catalog.create",
        json!({ "category": "Props", "name": "Red Car" }),
    );
    env.ok(
        "catalog.create",
        json!({ "category": "Props", "name": "Letter" }),
    );
    env.ok("locations.create", json!({ "name": "Railway Station" }));
    env.ok("locations.create", json!({ "name": "Old Depot" }));
    env.ok(
        "cast.create",
        json!({ "personName": "Arjun Rao", "characterName": "RAVI" }),
    );
    env.ok(
        "crew.create",
        json!({ "personName": "Priya", "role": "Cinematographer" }),
    );
    env.ok(
        "breakdown.add_element",
        json!({ "sceneId": scenes[0], "category": "Props", "name": "Red Car", "catalog": { "mode": "auto" } }),
    );
    env.ok("schedule.create", json!({}));
    let sched: String = env
        .core
        .project()
        .unwrap()
        .store
        .read(|c| Ok(c.query_row("SELECT id FROM shooting_schedule LIMIT 1", [], |r| r.get(0))?))
        .unwrap();
    env.ok(
        "schedule.create_day",
        json!({ "scheduleId": sched, "offDay": false }),
    );
    env.ok(
        "schedule.create_day",
        json!({ "scheduleId": sched, "offDay": false }),
    );
    env.ok(
        "notes.create",
        json!({ "title": "Ideas", "body": "Night shoots are hard." }),
    );
    env.ok("tasks.create", json!({ "title": "Scout the station" }));
    env.ok("files.create_folder", json!({ "name": "Refs" }));
    let path = env.write_file("station-notes.txt", b"platform 3");
    env.ok("files.add", json!({ "paths": [path] }));
    env.ok(
        "vault.create",
        json!({ "itemType": "note", "title": "Isolation", "body": "A man alone on a platform." }),
    );
    env.ok("vault.create_folder", json!({ "name": "Themes" }));
    env.ok("vault.create_collection", json!({ "name": "Mood" }));
    env.ok("moodboard.create", json!({ "name": "Station mood" }));
    env.ok("storyboard.create", json!({ "name": "Opening" }));
    env.ok(
        "shot.create",
        json!({ "sceneId": scenes[0], "description": "Wide of the empty platform" }),
    );
    env.ok("budget.create", json!({ "currency": "USD" }));
    Fx { draft, scenes }
}

// ------------------------------------------------------------------ approval boundary

#[test]
fn no_tool_can_review_apply_or_run_assistant_operations() {
    let env = TestEnv::with_project("Railway", "Feature Film");
    let owner = env.actor();
    let tools = toolbox::tools_for(&env.core, &owner, &whole(&env, &owner));
    assert!(tools.len() > 100, "universal tool surface: {}", tools.len());
    for t in &tools {
        let n = t.name.as_str();
        assert!(
            !n.contains("change_set")
                && !n.contains("accept")
                && !n.contains("apply")
                && !n.starts_with("ai.")
                && n != "final_answer",
            "{n}"
        );
        assert_eq!(t.schema["additionalProperties"], false, "{n}");
    }
    for (_, _, ops) in toolbox::coverage() {
        assert!(ops.iter().all(|o| !o.starts_with("ai.")));
    }
    for op in [
        "ai.change_set.accept",
        "ai.change_set.reject",
        "ai.change_set.recheck",
        "ai.ask",
        "trash.purge",
        "vault.purge",
        "project.delete",
    ] {
        assert!(
            !catalog::allowed_op(op),
            "{op} must never be allowed in a Change Set"
        );
    }
    // A Change Set smuggling acceptance is refused at apply time, too.
    let cs = change_set::create(
        &env.core,
        &env.actor(),
        &ChangeSetDraft {
            title: "Sneaky".into(),
            summary: "Test".into(),
            operations: vec![openframe_application::modules::ai::OpCall {
                op: "ai.change_set.accept".into(),
                args: json!({ "id": "x" }),
                label: "Accept".into(),
            }],
            preview: Vec::new(),
            exclusions: Vec::new(),
            targets: Vec::new(),
            modules: vec!["Project".into()],
            base_rows: Vec::new(),
            source_tool: "propose_task".into(),
            source_args: json!({}),
            sources: Vec::new(),
        },
    )
    .unwrap();
    let r = env.ok("ai.change_set.accept", json!({ "id": cs.id }));
    assert_eq!(r["state"], "Failed");
}

#[test]
fn unknown_tools_and_bad_arguments_are_refused() {
    let env = TestEnv::with_project("Railway", "Feature Film");
    fixture(&env);
    let code = |r: Result<ToolStep, AppError>| {
        r.err()
            .map(|e| e.code_str().to_string())
            .unwrap_or_default()
    };
    assert_eq!(
        code(run(
            &env,
            "execute_sql",
            json!({ "sql": "DELETE FROM task" })
        )),
        "ai.unknown_tool"
    );
    assert_eq!(
        code(run(&env, "ai.change_set.accept", json!({ "id": "x" }))),
        "ai.unknown_tool"
    );
    // Unknown fields, wrong types, oversized values and non-object arguments.
    assert_eq!(
        code(run(
            &env,
            "propose_task",
            json!({ "title": "A", "sql": "DROP" })
        )),
        "ai.tool_arguments"
    );
    assert_eq!(
        code(run(&env, "project_tasks", json!({ "status": 3 }))),
        "ai.tool_arguments"
    );
    assert_eq!(
        code(run(
            &env,
            "propose_task",
            json!({ "title": "x".repeat(201) })
        )),
        "ai.tool_arguments"
    );
    assert_eq!(
        code(run(
            &env,
            "propose_project_note",
            json!({ "body": "x".repeat(9_000) })
        )),
        "ai.tool_arguments"
    );
    assert_eq!(
        code(run(&env, "propose_task", json!(["title"]))),
        "ai.tool_arguments"
    );
    assert_eq!(
        code(run(
            &env,
            "propose_delete_vault_items",
            json!({ "items": vec!["a"; 51] })
        )),
        "ai.tool_arguments"
    );
    assert_eq!(
        count(&env, "SELECT count(*) FROM task WHERE deleted_at IS NULL"),
        1,
        "nothing was written"
    );
}

#[test]
fn proposal_tools_only_build_drafts_and_never_write() {
    let env = TestEnv::with_project("Railway", "Feature Film");
    fixture(&env);
    let before = fingerprint(&env);
    let d = propose(
        &env,
        "propose_task",
        json!({ "title": "Book the station", "dueDate": "2026-10-14" }),
    );
    assert_eq!(d.operations.len(), 1);
    assert_eq!(d.operations[0].op, "tasks.create");
    let d = propose(
        &env,
        "propose_delete_character",
        json!({ "character": "Meera" }),
    );
    assert_eq!(d.operations[0].op, "story.delete_character");
    assert!(
        d.preview.iter().any(|r| r.tone == toolbox::TONE_CONFIRM),
        "deletes the UI confirms carry the confirmation into the review card"
    );
    assert!(toolbox::requires_confirmation(&d.operations));
    assert_eq!(fingerprint(&env), before, "proposals change nothing");
}

#[test]
fn read_tools_never_mutate_and_resist_injection_text() {
    let env = TestEnv::with_project("Railway", "Feature Film");
    fixture(&env);
    let before = fingerprint(&env);
    for r in toolbox::reads::READS {
        let args = match r.name {
            "story_card" => json!({ "card": "Ravi waits" }),
            "schedule_day" => json!({ "day": "Day 1" }),
            "production_report" => json!({ "type": "scene" }),
            "screenplay_scene" | "breakdown_scene" => json!({ "sceneNumber": 1 }),
            _ => json!({}),
        };
        let _ = read(&env, r.name, args);
    }
    let _ = read(
        &env,
        "project_notes",
        json!({ "query": "'; DROP TABLE project_note; --" }),
    );
    let _ = read(&env, "idea_vault", json!({ "query": "%' OR 1=1 --" }));
    let _ = read(
        &env,
        "retrieve_context",
        json!({ "query": "Ravi alone at the station" }),
    );
    let _ = read(&env, "search_project", json!({ "text": "Ravi" }));
    assert_eq!(fingerprint(&env), before, "read tools are read-only");
}

// ------------------------------------------------------------------ permissions & privacy

#[test]
fn tools_are_filtered_by_role_and_enforced_when_run() {
    let env = TestEnv::with_project("Railway", "Feature Film");
    fixture(&env);
    let names = |role: Role| -> Vec<String> {
        let a = env.actor_with_role(role);
        toolbox::tools_for(&env.core, &a, &whole(&env, &a))
            .into_iter()
            .filter(|t| t.mutating)
            .map(|t| t.name)
            .collect()
    };
    assert!(names(Role::Viewer).is_empty());
    assert!(
        names(Role::Commenter).is_empty(),
        "commenters can't apply Change Sets"
    );
    assert!(names(Role::ExportOnly).is_empty());
    let editor = names(Role::Editor);
    assert!(editor.contains(&"propose_task".to_string()));
    assert!(
        !editor.contains(&"propose_project_status".to_string()),
        "ManageProject is Owner-only"
    );
    assert!(!editor.contains(&"propose_project_settings".to_string()));
    assert!(names(Role::Owner).contains(&"propose_project_status".to_string()));

    let viewer = env.actor_with_role(Role::Viewer);
    let e = run_as(&env, &viewer, "propose_task", json!({ "title": "Sneak" })).unwrap_err();
    assert_eq!(e.code_str(), "permission.denied");
    let editor = env.actor_with_role(Role::Editor);
    let e = run_as(
        &env,
        &editor,
        "propose_project_status",
        json!({ "status": "Shooting" }),
    )
    .unwrap_err();
    assert_eq!(e.code_str(), "permission.denied");
    // Reads stay available to a Viewer.
    assert!(matches!(
        run_as(&env, &viewer, "project_tasks", json!({})),
        Ok(ToolStep::Output(_))
    ));
}

#[test]
fn other_users_private_notes_never_reach_a_tool() {
    let env = TestEnv::with_project("Railway", "Feature Film");
    fixture(&env);
    let other = env.other_user(Role::Editor);
    let theirs = env
        .call_as(
            &other,
            "private_note.create",
            json!({ "body": "SECRET plan for the ending" }),
        )
        .unwrap();
    let their_id = theirs["id"].as_str().unwrap().to_string();
    env.ok(
        "private_note.create",
        json!({ "body": "My own reminder about the ending" }),
    );
    // Retrieval, search and the private-notes read only ever show the user's own note.
    for (tool, args) in [
        ("retrieve_context", json!({ "query": "ending plan secret" })),
        ("search_project", json!({ "text": "ending" })),
        ("private_information", json!({ "whose": "mine" })),
    ] {
        let text = read(&env, tool, args);
        assert!(
            !text.contains("SECRET"),
            "{tool} leaked another user's note: {text}"
        );
    }
    // The other user's note can't be targeted by id or by text.
    for reference in [their_id.as_str(), "SECRET plan"] {
        let e = run(
            &env,
            "propose_update_private_note",
            json!({ "note": reference, "text": "mine now" }),
        )
        .unwrap_err();
        assert_eq!(e.code_str(), "ai.not_found", "{reference}");
    }
    env.call_as(&other, "private_note.delete", json!({ "id": their_id }))
        .unwrap();
    assert!(!read(&env, "recently_deleted", json!({})).contains("SECRET"));
    let e = run(
        &env,
        "propose_restore_deleted",
        json!({ "item": "SECRET plan for the ending" }),
    )
    .unwrap_err();
    assert_eq!(e.code_str(), "ai.not_found");
}

#[test]
fn ambiguous_references_ask_one_question_instead_of_guessing() {
    let env = TestEnv::with_project("Railway", "Feature Film");
    fixture(&env);
    let e = run(
        &env,
        "propose_location_status",
        json!({ "location": "o", "status": "Confirmed" }),
    )
    .unwrap_err();
    assert_eq!(e.code_str(), "ai.ambiguous");
    assert!(
        e.message.contains("Railway Station") && e.message.contains("Old Depot"),
        "{}",
        e.message
    );
}

// ------------------------------------------------------------------ every proposal is valid

#[test]
fn every_proposal_tool_builds_operations_the_registry_accepts() {
    let env = TestEnv::with_project("Railway", "Feature Film");
    let fx = fixture(&env);
    let _ = &fx.draft;
    // Project, files, notes, tasks, templates.
    apply(
        &env,
        "propose_project_settings",
        json!({ "logline": "A man waits for a train that never comes.", "genre": "Drama" }),
    );
    apply(
        &env,
        "propose_project_status",
        json!({ "status": "Pre-Production" }),
    );
    apply(&env, "propose_folder", json!({ "name": "Contracts" }));
    apply(
        &env,
        "propose_rename_folder",
        json!({ "folder": "Contracts", "name": "Agreements" }),
    );
    apply(
        &env,
        "propose_move_file",
        json!({ "file": "station-notes.txt", "folder": "Refs" }),
    );
    apply(
        &env,
        "propose_file_notes",
        json!({ "file": "station-notes.txt", "notes": "Platform 3 details" }),
    );
    apply(
        &env,
        "propose_rename_file",
        json!({ "currentName": "station-notes.txt", "newName": "platform.txt" }),
    );
    apply(
        &env,
        "propose_project_note",
        json!({ "title": "Tone", "body": "Quiet and tense." }),
    );
    apply(
        &env,
        "propose_update_project_note",
        json!({ "note": "Tone", "body": "Quiet, tense, lonely." }),
    );
    apply(
        &env,
        "propose_pin_project_note",
        json!({ "note": "Tone", "pinned": true }),
    );
    apply(
        &env,
        "propose_task",
        json!({ "title": "Book the station", "notes": "Weekday night", "dueDate": "2026-10-14" }),
    );
    apply(
        &env,
        "propose_update_task",
        json!({ "task": "Book the station", "title": "Book the station (night)", "clearDueDate": true }),
    );
    apply(
        &env,
        "propose_task_done",
        json!({ "task": "Scout the station", "done": true }),
    );
    apply(
        &env,
        "propose_project_template",
        json!({ "type": "report", "name": "Weekly report" }),
    );
    apply(
        &env,
        "propose_rename_template",
        json!({ "template": "Weekly report", "name": "Weekly production report" }),
    );
    // Idea Vault.
    apply(
        &env,
        "propose_vault_item",
        json!({ "type": "quote", "text": "Trains are clocks.", "tags": ["time"], "folder": "Themes" }),
    );
    apply(
        &env,
        "propose_update_vault_item",
        json!({ "item": "Isolation", "text": "A man alone on platform 3." }),
    );
    apply(
        &env,
        "propose_organize_vault_items",
        json!({ "items": ["Isolation"], "pin": true, "folder": "Themes", "addToCollection": "Mood", "addTags": ["loneliness"] }),
    );
    apply(
        &env,
        "propose_organize_vault_items",
        json!({ "items": ["Isolation"], "removeTag": "loneliness", "removeFromCollection": "Mood", "topLevel": true }),
    );
    apply(
        &env,
        "propose_vault_folder",
        json!({ "action": "create", "name": "Places" }),
    );
    apply(
        &env,
        "propose_vault_folder",
        json!({ "action": "rename", "folder": "Places", "name": "Locations" }),
    );
    apply(
        &env,
        "propose_vault_collection",
        json!({ "action": "rename", "collection": "Mood", "name": "Mood refs", "note": "for the DP" }),
    );
    apply(
        &env,
        "propose_send_vault_to_story",
        json!({ "item": "Isolation", "as": "beat" }),
    );
    // Story.
    apply(&env, "propose_act", json!({ "title": "Act Three" }));
    apply(
        &env,
        "propose_update_act",
        json!({ "act": "Act Three", "note": "Resolution", "moveBefore": "Act Two" }),
    );
    apply(
        &env,
        "propose_sequence",
        json!({ "act": "Act Two", "title": "The Wait" }),
    );
    apply(
        &env,
        "propose_update_sequence",
        json!({ "sequence": "The Wait", "title": "The Long Wait" }),
    );
    apply(
        &env,
        "propose_scene_card",
        json!({ "description": "Anjali never arrives", "act": "Act Two" }),
    );
    apply(
        &env,
        "propose_scene_card",
        json!({ "description": "Meera finds the letter", "sequence": "Arrival" }),
    );
    apply(
        &env,
        "propose_update_scene_card",
        json!({ "card": "Anjali never arrives", "heading": "EXT. PLATFORM - NIGHT", "color": "blue" }),
    );
    apply(
        &env,
        "propose_beat",
        json!({ "text": "Hope returns", "act": "Act Two" }),
    );
    apply(
        &env,
        "propose_update_beat",
        json!({ "beat": "Hope returns", "note": "Small moment" }),
    );
    apply(
        &env,
        "propose_convert_beat",
        json!({ "beat": "Hope returns" }),
    );
    apply(
        &env,
        "propose_move_story_items",
        json!({ "cards": ["Meera finds the letter"], "to": "act", "act": "Act Two" }),
    );
    apply(
        &env,
        "propose_park_story_items",
        json!({ "cards": ["Meera finds the letter"] }),
    );
    apply(
        &env,
        "propose_unpark_story_items",
        json!({ "cards": ["Meera finds the letter"] }),
    );
    apply(
        &env,
        "propose_duplicate_story_items",
        json!({ "cards": ["Anjali never arrives"] }),
    );
    apply(
        &env,
        "propose_convert_card_to_beat",
        json!({ "card": "Meera finds the letter" }),
    );
    apply(
        &env,
        "propose_character",
        json!({ "name": "Station Master", "role": "Supporting" }),
    );
    apply(
        &env,
        "propose_update_character",
        json!({ "character": "Station Master", "description": "Knows every train." }),
    );
    apply(
        &env,
        "propose_relationship",
        json!({ "from": "Ravi", "to": "Anjali", "type": "Lovers" }),
    );
    apply(
        &env,
        "propose_update_relationship",
        json!({ "from": "Ravi", "to": "Anjali", "type": "Estranged" }),
    );
    apply(
        &env,
        "propose_link_character_card",
        json!({ "character": "Ravi", "card": "Ravi waits at the station" }),
    );
    apply(
        &env,
        "propose_link_character_card",
        json!({ "character": "Ravi", "card": "Ravi waits at the station", "unlink": true }),
    );
    apply(
        &env,
        "propose_archive_character",
        json!({ "character": "Station Master", "archived": true }),
    );
    apply(
        &env,
        "propose_story_day",
        json!({ "sceneNumbers": [1, 2], "storyDay": "Day 1" }),
    );
    apply(
        &env,
        "propose_time_note",
        json!({ "sceneNumber": 3, "timeNote": "Three weeks later" }),
    );
    apply(&env, "propose_season", json!({ "title": "Season 1" }));
    apply(
        &env,
        "propose_update_season",
        json!({ "season": "Season 1", "note": "Pilot season" }),
    );
    apply(
        &env,
        "propose_episode",
        json!({ "title": "Pilot", "season": "Season 1" }),
    );
    apply(
        &env,
        "propose_update_episode",
        json!({ "episode": "Pilot", "summary": "Ravi waits." }),
    );
    apply(
        &env,
        "propose_duplicate_episode",
        json!({ "episode": "Pilot" }),
    );
    apply(
        &env,
        "propose_rename_character",
        json!({ "from": "Meera", "to": "Maya" }),
    );
    // Screenplay.
    apply(
        &env,
        "propose_screenplay_lines",
        json!({ "sceneNumber": 3, "lines": [{ "type": "character", "text": "RAVI" }, { "type": "dialogue", "text": "I'm here." }], "afterLine": 1 }),
    );
    apply(
        &env,
        "propose_edit_screenplay_line",
        json!({ "sceneNumber": 3, "line": 1, "text": "Ravi reads the letter twice." }),
    );
    apply(
        &env,
        "propose_delete_screenplay_line",
        json!({ "sceneNumber": 3, "line": 2 }),
    );
    apply(
        &env,
        "propose_update_scene",
        json!({ "sceneNumber": 2, "synopsis": "Anjali is late.", "storyDay": "Day 2" }),
    );
    apply(
        &env,
        "propose_new_scene",
        json!({ "heading": "EXT. TRACKS - DAWN", "position": 2 }),
    );
    apply(
        &env,
        "propose_move_scene",
        json!({ "sceneNumber": 2, "toPosition": 4 }),
    );
    apply(
        &env,
        "propose_replace_text",
        json!({ "find": "letter", "replace": "note", "wholeWord": true }),
    );
    apply(
        &env,
        "propose_title_page",
        json!({ "title": "Railway", "writtenBy": "A. Writer" }),
    );
    apply(
        &env,
        "propose_link_character_cue",
        json!({ "cue": "RAVI", "character": "Ravi" }),
    );
    apply(
        &env,
        "propose_link_character_cue",
        json!({ "cue": "RAVI", "clear": true }),
    );
    apply(
        &env,
        "propose_review_round",
        json!({ "name": "Producer notes", "reviewers": ["Sam"], "deadline": "2026-11-01" }),
    );
    apply(
        &env,
        "propose_update_review_round",
        json!({ "round": "Producer notes", "reviewers": ["Sam", "Lee"] }),
    );
    apply(
        &env,
        "propose_complete_review_round",
        json!({ "round": "Producer notes" }),
    );
    apply(
        &env,
        "propose_new_draft",
        json!({ "name": "Draft 2", "makeCurrent": false }),
    );
    apply(
        &env,
        "propose_rename_draft",
        json!({ "draft": "Draft 2", "name": "Draft 2 — Night", "note": "Night rewrite" }),
    );
    apply(
        &env,
        "propose_current_draft",
        json!({ "draft": "Draft 2 — Night" }),
    );
    apply(&env, "propose_restore_draft", json!({ "draft": "Draft 1" }));
    let e = run(
        &env,
        "propose_screenplay",
        json!({ "title": "Railway (short version)" }),
    )
    .unwrap_err();
    assert_eq!(
        e.code_str(),
        "ai.ambiguous",
        "one screenplay per project/episode; drafts instead"
    );
    apply(&env, "propose_current_draft", json!({ "draft": "Draft 1" }));
    // Comments & private notes.
    apply(
        &env,
        "propose_comment",
        json!({ "text": "Can we shoot this at dusk?", "sceneNumber": 1 }),
    );
    apply(
        &env,
        "propose_comment",
        json!({ "text": "Great location", "location": "Railway Station" }),
    );
    apply(
        &env,
        "propose_comment_reply",
        json!({ "comment": "Can we shoot this at dusk?", "text": "Yes." }),
    );
    apply(
        &env,
        "propose_update_comment",
        json!({ "comment": "Can we shoot this at dusk?", "status": "In Discussion" }),
    );
    apply(
        &env,
        "propose_resolve_comment",
        json!({ "comment": "Can we shoot this at dusk?" }),
    );
    apply(
        &env,
        "propose_resolve_comment",
        json!({ "comment": "Can we shoot this at dusk?", "reopen": true }),
    );
    apply(
        &env,
        "propose_private_note",
        json!({ "text": "Ask about the permit fee", "sceneNumber": 1 }),
    );
    apply(
        &env,
        "propose_update_private_note",
        json!({ "note": "Ask about the permit", "text": "Ask about the permit fee (night rate)" }),
    );
    // Production.
    apply(
        &env,
        "propose_breakdown_element",
        json!({ "sceneNumber": 3, "category": "Props", "name": "Letter" }),
    );
    apply(
        &env,
        "propose_breakdown_element",
        json!({ "sceneNumber": 2, "category": "Wardrobe", "name": "Station uniform" }),
    );
    apply(
        &env,
        "propose_breakdown_note",
        json!({ "sceneNumber": 3, "element": "Letter", "notes": "Aged paper" }),
    );
    apply(
        &env,
        "propose_archive_breakdown_element",
        json!({ "sceneNumber": 3, "element": "Letter", "archived": true }),
    );
    apply(
        &env,
        "propose_breakdown_suggestions",
        json!({ "sceneNumber": 1 }),
    );
    apply(
        &env,
        "propose_breakdown_complete",
        json!({ "sceneNumber": 1, "complete": true }),
    );
    apply(
        &env,
        "propose_breakdown_reviewed",
        json!({ "sceneNumber": 1 }),
    );
    apply(
        &env,
        "propose_catalog_item",
        json!({ "category": "Vehicles", "name": "Vintage Train", "status": "Searching" }),
    );
    apply(
        &env,
        "propose_update_catalog_item",
        json!({ "item": "Vintage Train", "notes": "Heritage railway society" }),
    );
    apply(
        &env,
        "propose_catalog_alias",
        json!({ "item": "Red Car", "alias": "Ravi's car" }),
    );
    apply(
        &env,
        "propose_catalog_alias",
        json!({ "item": "Red Car", "alias": "Ravi's car", "remove": true }),
    );
    apply(
        &env,
        "propose_archive_catalog_item",
        json!({ "item": "Vintage Train", "archived": true }),
    );
    apply(
        &env,
        "propose_location",
        json!({ "name": "Signal Box", "status": "Idea" }),
    );
    apply(
        &env,
        "propose_update_location",
        json!({ "location": "Railway Station", "address": "Platform Road", "parking": "Behind the ticket office" }),
    );
    apply(
        &env,
        "propose_location_status",
        json!({ "location": "Railway Station", "status": "Confirmed" }),
    );
    apply(
        &env,
        "propose_archive_location",
        json!({ "location": "Signal Box", "archived": true }),
    );
    apply(
        &env,
        "propose_cast_member",
        json!({ "person": "Leela Nair", "characterName": "ANJALI" }),
    );
    apply(
        &env,
        "propose_update_cast_member",
        json!({ "castMember": "Leela Nair", "availability": "Weekends only" }),
    );
    apply(
        &env,
        "propose_assign_cast",
        json!({ "castMember": "Leela Nair", "character": "Anjali" }),
    );
    apply(
        &env,
        "propose_crew_member",
        json!({ "person": "Kabir", "role": "Gaffer", "department": "Lighting" }),
    );
    apply(
        &env,
        "propose_update_crew_member",
        json!({ "crewMember": "Kabir", "notes": "Has his own truck" }),
    );
    apply(
        &env,
        "propose_archive_person",
        json!({ "kind": "crew", "person": "Kabir", "archived": true }),
    );
    // Visual planning.
    apply(
        &env,
        "propose_moodboard",
        json!({ "name": "Platform light", "sceneNumber": 1 }),
    );
    apply(
        &env,
        "propose_update_moodboard",
        json!({ "moodboard": "Platform light", "notes": "Sodium vapour" }),
    );
    apply(
        &env,
        "propose_moodboard_note",
        json!({ "moodboard": "Station mood", "text": "Cold, empty" }),
    );
    apply(
        &env,
        "propose_moodboard_link",
        json!({ "moodboard": "Station mood", "url": "https://example.com/ref", "title": "Ref" }),
    );
    apply(
        &env,
        "propose_update_moodboard_item",
        json!({ "moodboard": "Station mood", "item": "Cold, empty", "text": "Cold, empty, blue" }),
    );
    apply(&env, "propose_storyboard", json!({ "sceneNumber": 1 }));
    apply(
        &env,
        "propose_update_storyboard",
        json!({ "storyboard": "Opening", "name": "Opening sequence", "sceneNumber": 1 }),
    );
    apply(
        &env,
        "propose_storyboard_panel",
        json!({ "storyboard": "Opening sequence", "description": "Train lights in fog" }),
    );
    apply(
        &env,
        "propose_storyboard_panel",
        json!({ "storyboard": "Opening sequence", "description": "Ravi's face" }),
    );
    apply(
        &env,
        "propose_update_storyboard_panel",
        json!({ "storyboard": "Opening sequence", "panel": 1, "framing": "Wide", "durationSeconds": 4 }),
    );
    apply(
        &env,
        "propose_move_storyboard_panel",
        json!({ "storyboard": "Opening sequence", "panel": 2, "position": 1 }),
    );
    apply(
        &env,
        "propose_shot",
        json!({ "sceneNumber": 2, "description": "Close on Ravi", "size": "CU", "characters": ["RAVI"] }),
    );
    apply(
        &env,
        "propose_update_shot",
        json!({ "sceneNumber": 2, "shot": 2, "lens": "85mm" }),
    );
    apply(
        &env,
        "propose_move_shot",
        json!({ "sceneNumber": 2, "shot": 2, "toPosition": 1 }),
    );
    apply(
        &env,
        "propose_duplicate_shot",
        json!({ "sceneNumber": 2, "shot": 1 }),
    );
    apply(
        &env,
        "propose_link_panel_shot",
        json!({ "storyboard": "Opening sequence", "panel": 1, "sceneNumber": 2, "shot": 1 }),
    );
    apply(
        &env,
        "propose_link_panel_shot",
        json!({ "storyboard": "Opening sequence", "panel": 1, "unlink": true }),
    );
    apply(
        &env,
        "propose_shot_from_panel",
        json!({ "storyboard": "Opening sequence", "panel": 2 }),
    );
    apply(
        &env,
        "propose_panel_from_shot",
        json!({ "sceneNumber": 2, "shot": 1 }),
    );
    apply(
        &env,
        "propose_copy_planning",
        json!({ "fromSceneNumber": 2, "toSceneNumber": 3 }),
    );
    apply(
        &env,
        "propose_mark_planning_reviewed",
        json!({ "sceneNumber": 1 }),
    );
    // Schedule, call sheets, sides, reports, budget.
    apply(
        &env,
        "propose_schedule_settings",
        json!({ "name": "Main unit", "dayLengthMinutes": 660 }),
    );
    apply(
        &env,
        "propose_shooting_day",
        json!({ "date": "2026-10-20", "notes": "Night shoot" }),
    );
    apply(
        &env,
        "propose_update_shooting_day",
        json!({ "day": "Day 1", "date": "2026-10-18", "notes": "Station", "targetMinutes": 600 }),
    );
    apply(
        &env,
        "propose_schedule_scenes",
        json!({ "sceneNumbers": [1, 2], "day": "Day 1" }),
    );
    apply(
        &env,
        "propose_schedule_scenes",
        json!({ "sceneNumbers": [3], "day": "Day 2", "position": 1 }),
    );
    apply(
        &env,
        "propose_scene_timing",
        json!({ "sceneNumber": 1, "estimateMinutes": 90, "pageEighths": 12 }),
    );
    apply(
        &env,
        "propose_schedule_break",
        json!({ "day": "Day 1", "type": "Meal", "label": "Lunch", "time": "13:00", "durationMinutes": 45 }),
    );
    apply(
        &env,
        "propose_update_schedule_break",
        json!({ "break": "Lunch", "durationMinutes": 60 }),
    );
    apply(
        &env,
        "propose_move_schedule_break",
        json!({ "break": "Lunch", "day": "Day 2" }),
    );
    apply(
        &env,
        "propose_move_shooting_day",
        json!({ "day": "Day 3", "toPosition": 1 }),
    );
    apply(
        &env,
        "propose_duplicate_shooting_day",
        json!({ "day": "Day 2" }),
    );
    apply(&env, "propose_reconcile_schedule", json!({}));
    apply(
        &env,
        "propose_schedule_status",
        json!({ "status": "Draft" }),
    );
    apply(&env, "propose_call_sheet", json!({ "day": "Day 2" }));
    let cs = read(&env, "call_sheets", json!({}));
    let cs_title = cs
        .lines()
        .nth(1)
        .unwrap()
        .trim_start_matches("- ")
        .split(" Draft")
        .next()
        .unwrap()
        .trim()
        .to_string();
    apply(
        &env,
        "propose_update_call_sheet",
        json!({ "callSheet": cs_title, "crewCall": "06:30", "weather": "Clear, 12°C" }),
    );
    apply(
        &env,
        "propose_refresh_call_sheet",
        json!({ "callSheet": cs_title }),
    );
    apply(
        &env,
        "propose_call_sheet_ready",
        json!({ "callSheet": cs_title, "ready": true }),
    );
    apply(
        &env,
        "propose_finalize_call_sheet",
        json!({ "callSheet": cs_title, "acknowledgeStale": true }),
    );
    apply(
        &env,
        "propose_issue_call_sheet",
        json!({ "callSheet": cs_title }),
    );
    apply(
        &env,
        "propose_call_sheet_revision",
        json!({ "callSheet": cs_title }),
    );
    apply(
        &env,
        "propose_sides",
        json!({ "day": "Day 2", "title": "Day 2 sides", "includeCover": true }),
    );
    apply(
        &env,
        "propose_save_report",
        json!({ "type": "scene", "title": "Scene report" }),
    );
    apply(
        &env,
        "propose_budget_settings",
        json!({ "plannedTotal": 50000, "contingencyMode": "percent", "contingencyValue": 10 }),
    );
    apply(
        &env,
        "propose_budget_line",
        json!({ "category": "Locations", "description": "Station permit", "amount": 1200 }),
    );
    apply(
        &env,
        "propose_update_budget_line",
        json!({ "line": "Station permit", "amount": 1500 }),
    );
    apply(
        &env,
        "propose_budget_snapshot",
        json!({ "label": "First pass" }),
    );
    apply(&env, "propose_budget_reviewed", json!({}));
    apply(
        &env,
        "propose_build_screenplay",
        json!({ "destination": "new_draft", "draftName": "Built from Story" }),
    );
    apply(&env, "propose_story_order", json!({ "draft": "Draft 1" }));
    apply(
        &env,
        "propose_delete_call_sheet",
        json!({ "callSheet": cs_title }),
    );
    // Deletions last (all recoverable) and restore.
    apply(
        &env,
        "propose_delete_budget_snapshot",
        json!({ "snapshot": "First pass" }),
    );
    apply(
        &env,
        "propose_delete_budget_line",
        json!({ "line": "Station permit" }),
    );
    apply(
        &env,
        "propose_delete_report",
        json!({ "report": "Scene report" }),
    );
    apply(
        &env,
        "propose_delete_sides",
        json!({ "sides": "Day 2 sides" }),
    );
    apply(
        &env,
        "propose_delete_schedule_break",
        json!({ "break": "Lunch" }),
    );
    apply(
        &env,
        "propose_delete_shooting_day",
        json!({ "day": "Day 4" }),
    );
    apply(
        &env,
        "propose_delete_shots",
        json!({ "sceneNumber": 3, "shots": [1] }),
    );
    apply(
        &env,
        "propose_delete_storyboard_panels",
        json!({ "storyboard": "Opening sequence", "panels": [2] }),
    );
    apply(
        &env,
        "propose_delete_moodboard_items",
        json!({ "moodboard": "Station mood", "items": ["Ref"] }),
    );
    apply(
        &env,
        "propose_delete_moodboard",
        json!({ "moodboard": "Platform light" }),
    );
    apply(
        &env,
        "propose_delete_storyboard",
        json!({ "storyboard": "Opening sequence" }),
    );
    apply(
        &env,
        "propose_remove_breakdown_element",
        json!({ "sceneNumber": 2, "element": "Station uniform" }),
    );
    apply(
        &env,
        "propose_replace_catalog_item",
        json!({ "from": "Red Car", "to": "Letter" }),
    );
    apply(
        &env,
        "propose_delete_catalog_item",
        json!({ "item": "Vintage Train" }),
    );
    apply(
        &env,
        "propose_delete_location",
        json!({ "location": "Signal Box" }),
    );
    apply(
        &env,
        "propose_remove_person",
        json!({ "kind": "cast", "person": "Leela Nair" }),
    );
    apply(
        &env,
        "propose_delete_relationship",
        json!({ "from": "Ravi", "to": "Anjali" }),
    );
    apply(
        &env,
        "propose_delete_character",
        json!({ "character": "Station Master" }),
    );
    // Two cards share this description after the duplicate: one question, no guess.
    let e = run(
        &env,
        "propose_delete_story_items",
        json!({ "cards": ["Anjali never arrives"] }),
    )
    .unwrap_err();
    assert_eq!(e.code_str(), "ai.ambiguous");
    apply(
        &env,
        "propose_delete_story_items",
        json!({ "cards": ["Ravi waits at the station"] }),
    );
    apply(
        &env,
        "propose_delete_sequence",
        json!({ "sequence": "The Long Wait" }),
    );
    apply(&env, "propose_delete_act", json!({ "act": "Act Three" }));
    apply(
        &env,
        "propose_delete_episode",
        json!({ "episode": "Pilot" }),
    );
    apply(
        &env,
        "propose_delete_season",
        json!({ "season": "Season 1" }),
    );
    apply(&env, "propose_delete_scene", json!({ "sceneNumber": 2 }));
    apply(
        &env,
        "propose_delete_comment",
        json!({ "comment": "Great location" }),
    );
    apply(
        &env,
        "propose_delete_private_note",
        json!({ "note": "Ask about the permit" }),
    );
    apply(
        &env,
        "propose_delete_vault_items",
        json!({ "items": ["Trains are clocks."] }),
    );
    apply(
        &env,
        "propose_vault_folder",
        json!({ "action": "delete", "folder": "Locations" }),
    );
    apply(
        &env,
        "propose_vault_collection",
        json!({ "action": "delete", "collection": "Mood refs" }),
    );
    apply(
        &env,
        "propose_delete_template",
        json!({ "template": "Weekly production report" }),
    );
    apply(
        &env,
        "propose_delete_task",
        json!({ "task": "Scout the station" }),
    );
    apply(
        &env,
        "propose_delete_project_note",
        json!({ "note": "Tone" }),
    );
    apply(
        &env,
        "propose_delete_file",
        json!({ "file": "platform.txt" }),
    );
    apply(
        &env,
        "propose_delete_folder",
        json!({ "folder": "Agreements" }),
    );
    apply(
        &env,
        "propose_restore_deleted",
        json!({ "item": "platform.txt" }),
    );
    apply(
        &env,
        "propose_delete_draft",
        json!({ "draft": "Draft 2 — Night" }),
    );
    apply(&env, "propose_lock_draft", json!({ "draft": "Draft 1" }));
    apply(
        &env,
        "propose_revision",
        json!({ "draft": "Draft 1", "label": "Blue", "color": "blue" }),
    );
    let e = run(
        &env,
        "propose_production_source",
        json!({ "draft": "Built from Story" }),
    )
    .unwrap_err();
    assert_eq!(
        e.code_str(),
        "ai.ambiguous",
        "a source change goes through the Production update"
    );
    apply(
        &env,
        "propose_production_update",
        json!({ "draft": "Built from Story" }),
    );
    // A locked draft's text is refused up front.
    let e = run(
        &env,
        "propose_update_scene",
        json!({ "draft": "Draft 1", "sceneNumber": 1, "heading": "INT. NEW - DAY" }),
    )
    .unwrap_err();
    assert_eq!(e.code_str(), "ai.locked");
    assert!(fx.scenes.len() == 3);
}

/// Every proposal tool accepts every field its schema declares (schema and builder agree).
#[test]
fn every_proposal_schema_field_is_understood_by_its_builder() {
    let env = TestEnv::with_project("Railway", "Feature Film");
    fixture(&env);
    fn sample(key: &str, s: &Value) -> Value {
        if let Some(e) = s.get("enum").and_then(|e| e.as_array()) {
            return e[0].clone();
        }
        match s["type"].as_str() {
            Some("integer") => s["minimum"].clone(),
            Some("boolean") => json!(false),
            Some("array") => json!([sample(key, &s["items"])]),
            Some("object") => {
                let mut o = serde_json::Map::new();
                for (k, v) in s["properties"].as_object().unwrap() {
                    o.insert(k.clone(), sample(k, v));
                }
                Value::Object(o)
            }
            _ => {
                let k = key.to_lowercase();
                if k.contains("date") || k == "deadline" {
                    json!("2026-10-14")
                } else if k == "time" {
                    json!("10:00")
                } else if k == "day" || k == "afterday" {
                    json!("Day 1")
                } else if k == "currency" {
                    json!("USD")
                } else {
                    json!("Ravi")
                }
            }
        }
    }
    for p in catalog::PROPOSALS.iter() {
        let args = sample("", &(p.schema)());
        match run(&env, p.tool, args.clone()) {
            Ok(ToolStep::Proposal(d)) => assert!(!d.operations.is_empty(), "{}", p.tool),
            Ok(ToolStep::Output(_)) => panic!("{} returned output", p.tool),
            Err(e) => assert!(
                !e.is("ai.tool_arguments") && !e.is("internal"),
                "{} rejected its own schema's fields: {} {} ({:?}) args={args}",
                p.tool,
                e.code,
                e.message,
                e.detail
            ),
        }
    }
}

#[test]
fn request_relevant_tool_selection_fits_a_small_prompt() {
    let env = TestEnv::with_project("Railway", "Feature Film");
    let owner = env.actor();
    let scope = whole(&env, &owner);
    let picked = toolbox::tools_for_request(
        &env.core,
        &owner,
        &scope,
        "Move scenes 4 and 5 to shooting day 3",
        6_000,
    );
    let names: Vec<&str> = picked.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"propose_schedule_scenes"), "{names:?}");
    assert!(names.contains(&"retrieve_context"));
    let size: usize = picked
        .iter()
        .map(|t| t.name.len() + t.description.len() + t.schema.to_string().len())
        .sum();
    assert!(
        size <= 6_000 + 3_000,
        "core tools plus a bounded domain slice: {size}"
    );
    let reads = toolbox::tools_for_request(
        &env.core,
        &owner,
        &scope,
        "Which locations are confirmed?",
        6_000,
    );
    assert!(
        reads.iter().all(|t| !t.mutating),
        "questions get no proposal tools"
    );
    assert!(reads.iter().any(|t| t.name == "production_locations"));
}
