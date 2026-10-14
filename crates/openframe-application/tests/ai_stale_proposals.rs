//! stale proposal tests: stale-proposal tests per domain (AI spec §20, AI-AC-014, ADR-0013, ADR-0019;
//! CONTRIBUTING §2 invariant 2).
//!
//! For every domain the cycle is the one a filmmaker can hit: the assistant retrieves the
//! project and prepares a Change Set through the real proposal tool; the user then edits a
//! row that proposal was built on through the normal operation; accepting must refuse
//! (Stale or Conflict) and change nothing; "Re-check & Review" must rebuild the proposal
//! against the user's new data, and only then can it be applied, keeping the user's edit.

use openframe_application::modules::ai::ChangeSetDraft;
use openframe_application::modules::ai::scope::{self, AiScopeArgs, AiScopeKind};
use openframe_application::modules::ai::toolbox::{self, ToolStep};
use openframe_application::modules::ai::{change_set, queries};
use openframe_test_support::TestEnv;
use rusqlite::types::ValueRef;
use serde_json::{Value, json};

// ------------------------------------------------------------------ helpers

/// The real proposal path: the tool the agent loop calls, with a whole-project scope.
#[track_caller]
fn propose(env: &TestEnv, tool: &str, args: Value) -> ChangeSetDraft {
    let actor = env.actor();
    let scope = env
        .core
        .project()
        .unwrap()
        .store
        .read(|c| {
            scope::resolve(
                c,
                &actor,
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
        .unwrap();
    match toolbox::run_tool_unlocked(&env.core, &actor, &scope, "test request", tool, &args) {
        Ok(ToolStep::Proposal(d)) => d,
        Ok(ToolStep::Output(o)) => panic!("{tool} returned output, not a proposal: {}", o.content),
        Err(e) => panic!("{tool} failed: {} — {} ({:?})", e.code, e.message, e.detail),
    }
}

fn scalar<T: rusqlite::types::FromSql>(env: &TestEnv, sql: &str, p: &[&str]) -> T {
    env.core
        .project()
        .unwrap()
        .store
        .read(|c| Ok(c.query_row(sql, rusqlite::params_from_iter(p), |r| r.get(0))?))
        .unwrap()
}

/// The scene number users (and the assistant) see for a scene: the product's own numbering.
fn scene_no(env: &TestEnv, scene_id: &str) -> u32 {
    env.core
        .project()
        .unwrap()
        .store
        .read(|c| queries::scene_by_id(c, scene_id))
        .unwrap()
        .expect("scene exists")
        .1
        .number as u32
}

/// The 1-based line number of a screenplay element within its scene (screenplay order).
fn line_no(env: &TestEnv, scene_id: &str, element_id: &str) -> u32 {
    let ids: Vec<String> = env
        .core
        .project()
        .unwrap()
        .store
        .read(|c| {
            let mut st = c.prepare(
                "SELECT id FROM screenplay_element WHERE scene_id=?1 ORDER BY position, id",
            )?;
            Ok(st
                .query_map([scene_id], |r| r.get(0))?
                .collect::<Result<_, _>>()?)
        })
        .unwrap();
    ids.iter()
        .position(|i| i == element_id)
        .expect("line exists") as u32
        + 1
}

/// Every row of every canonical table (the Change Set and AI bookkeeping records, the
/// derived search index and system journals excluded): "nothing changed" means equal.
fn canonical(env: &TestEnv) -> Vec<(String, Vec<String>)> {
    env.core
        .project()
        .unwrap()
        .store
        .read(|c| {
            let mut st = c.prepare(
                "SELECT name FROM sqlite_master WHERE type='table'
                   AND name NOT LIKE 'sqlite_%' AND name NOT LIKE 'search_%'
                   AND name NOT LIKE 'sys_%' AND name NOT LIKE 'ai_%' AND name NOT LIKE 'change_set%'
                 ORDER BY name",
            )?;
            let tables: Vec<String> = st.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
            let mut out = Vec::new();
            for t in tables {
                let mut q = c.prepare(&format!("SELECT * FROM \"{t}\""))?;
                let n = q.column_count();
                let mut rows: Vec<String> = q
                    .query_map([], |r| {
                        let mut s = String::new();
                        for i in 0..n {
                            match r.get_ref(i)? {
                                ValueRef::Null => s.push('∅'),
                                ValueRef::Integer(v) => s.push_str(&v.to_string()),
                                ValueRef::Real(v) => s.push_str(&v.to_string()),
                                ValueRef::Text(v) => s.push_str(&String::from_utf8_lossy(v)),
                                ValueRef::Blob(v) => s.push_str(&format!("{v:?}")),
                            }
                            s.push('|');
                        }
                        Ok(s)
                    })?
                    .collect::<Result<_, _>>()?;
                rows.sort();
                out.push((t, rows));
            }
            Ok(out)
        })
        .unwrap()
}

#[track_caller]
fn assert_unchanged(env: &TestEnv, before: &[(String, Vec<String>)], why: &str) {
    let now = canonical(env);
    for ((t, a), (_, b)) in before.iter().zip(&now) {
        assert_eq!(a, b, "{why}: table {t} changed");
    }
    assert_eq!(before.len(), now.len(), "{why}: tables changed");
}

/// Preview rows ("label: old → new") of a Change Set (`rows`) or of its first preparation
/// (`preparedRows`).
fn rows_text(rows: &Value) -> String {
    rows.as_array()
        .unwrap()
        .iter()
        .map(|r| {
            format!(
                "{}: {}",
                r["label"].as_str().unwrap_or(""),
                r["value"].as_str().unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Propose → user edit → accept refuses and changes nothing → recheck rebuilds.
/// Returns the re-checked Change Set (Pending again); the caller checks it used the new data,
/// then applies it.
#[track_caller]
fn stale_cycle(
    env: &TestEnv,
    tool: &str,
    args: Value,
    edited: (&str, &str),
    user_edit: impl FnOnce(&TestEnv),
) -> Value {
    // 1. Retrieve and propose through the real tool path.
    let d = propose(env, tool, args);
    assert!(!d.operations.is_empty(), "{tool}: empty proposal");
    assert!(
        d.base_rows
            .iter()
            .any(|(t, id)| t == edited.0 && id == edited.1),
        "{tool}: the proposal must be guarded by the {} row it changes: {:?}",
        edited.0,
        d.base_rows
    );
    assert!(
        d.operations.iter().any(|o| o
            .args
            .as_object()
            .is_some_and(|m| m.values().any(|v| v == edited.1))),
        "{tool}: the proposal must change the {} row the user edits: {:?}",
        edited.0,
        d.operations
    );
    let cs = change_set::create(&env.core, &env.actor(), &d).unwrap();
    assert_eq!(cs.state, "Pending");
    let id = cs.id.clone();
    let prepared_rows = serde_json::to_value(&cs.rows).unwrap();

    // 2. The user edits a row the proposal was built on, through the normal operation.
    user_edit(env);
    let before = canonical(env);

    // 3. Accepting refuses: never applied blindly, nothing changes.
    let r = env.ok("ai.change_set.accept", json!({ "id": id }));
    let state = r["state"].as_str().unwrap().to_string();
    assert!(
        state == "Stale" || state == "Conflict",
        "{tool}: a proposal built on edited data must not apply, got {state}: {r}"
    );
    assert_eq!(r["validationState"], "Needs Review", "{tool}");
    assert!(
        r["staleReason"]
            .as_str()
            .unwrap()
            .contains("has not been applied"),
        "{tool}: {r}"
    );
    assert_eq!(r["appliedOperations"], 0, "{tool}");
    assert!(r["appliedAt"].is_null(), "{tool}");
    assert_unchanged(env, &before, &format!("{tool}: refused accept"));
    // Accepting again is refused outright until it is re-checked.
    assert_eq!(
        env.err("ai.change_set.accept", json!({ "id": id })),
        "conflict.state",
        "{tool}"
    );
    assert_unchanged(env, &before, &format!("{tool}: second accept"));

    // 4. Re-check rebuilds the proposal against the current project; it applies nothing.
    let r = env.ok("ai.change_set.recheck", json!({ "id": id }));
    assert_eq!(r["state"], "Pending", "{tool}: {r}");
    assert_eq!(r["validationState"], "Valid", "{tool}");
    assert!(r["staleReason"].is_null(), "{tool}");
    assert_unchanged(env, &before, &format!("{tool}: recheck"));
    let mut r = r;
    r["preparedRows"] = prepared_rows;
    r
}

#[track_caller]
fn apply(env: &TestEnv, cs: &Value) {
    let r = env.ok("ai.change_set.accept", json!({ "id": cs["id"] }));
    assert_eq!(r["state"], "Applied", "{r}");
    assert_eq!(r["appliedOperations"], cs["operationCount"], "{r}");
}

// ------------------------------------------------------------------ fixture

struct Fx {
    scenes: Vec<String>,
    day: String,
    budget: String,
}

/// A small project built only through registry operations.
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
            vec![("character", "ANJALI"), ("dialogue", "The train is late.")],
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
    for name in ["Ravi", "Anjali"] {
        env.ok("story.create_character", json!({ "name": name }));
    }
    env.ok("production.set_source", json!({ "draftId": draft }));
    env.ok(
        "breakdown.add_element",
        json!({ "sceneId": scenes[0], "category": "Props", "name": "Red Car", "catalog": { "mode": "auto" } }),
    );
    env.ok(
        "cast.create",
        json!({ "personName": "Arjun Rao", "characterName": "RAVI" }),
    );
    env.ok(
        "crew.create",
        json!({ "personName": "Priya", "role": "Cinematographer" }),
    );
    env.ok("schedule.create", json!({}));
    let sched: String = scalar(env, "SELECT id FROM shooting_schedule LIMIT 1", &[]);
    env.ok(
        "schedule.create_day",
        json!({ "scheduleId": sched, "offDay": false }),
    );
    let day: String = scalar(
        env,
        "SELECT id FROM shooting_day WHERE schedule_id=?1 ORDER BY position LIMIT 1",
        &[&sched],
    );
    env.ok(
        "notes.create",
        json!({ "title": "Ideas", "body": "Night shoots are hard." }),
    );
    env.ok(
        "shot.create",
        json!({ "sceneId": scenes[0], "description": "Wide of the empty platform" }),
    );
    env.ok("budget.create", json!({ "currency": "USD" }));
    let budget: String = scalar(env, "SELECT id FROM budget_snapshot LIMIT 1", &[]);
    Fx {
        scenes,
        day,
        budget,
    }
}

fn setup() -> (TestEnv, Fx) {
    let env = TestEnv::with_project("Railway", "Feature Film");
    let fx = fixture(&env);
    (env, fx)
}

// ------------------------------------------------------------------ per domain

#[test]
fn m1_07_screenplay_line_edited_by_the_user_makes_the_proposal_stale() {
    let (env, fx) = setup();
    let line: String = scalar(
        &env,
        "SELECT id FROM screenplay_element WHERE text='Ravi waits by the red car.'",
        &[],
    );
    let scene_number = scene_no(&env, &fx.scenes[0]);
    let line_number = line_no(&env, &fx.scenes[0], &line);
    let cs = stale_cycle(
        &env,
        "propose_edit_screenplay_line",
        json!({ "sceneNumber": scene_number, "line": line_number, "text": "Ravi paces beside the red car." }),
        ("screenplay_element", &line),
        |env| {
            env.ok(
                "screenplay.update_element",
                json!({ "id": line, "text": "Ravi sits on his suitcase." }),
            );
        },
    );
    let text = || -> String {
        scalar(
            &env,
            "SELECT text FROM screenplay_element WHERE id=?1",
            &[&line],
        )
    };
    assert_eq!(
        text(),
        "Ravi sits on his suitcase.",
        "the user's edit stands"
    );
    assert!(
        rows_text(&cs["rows"])
            .contains("Ravi sits on his suitcase. → Ravi paces beside the red car."),
        "re-check compares against the user's text: {}",
        rows_text(&cs["rows"])
    );
    assert!(
        rows_text(&cs["preparedRows"])
            .contains("Ravi waits by the red car. → Ravi paces beside the red car."),
        "the first preparation compared against the original text"
    );
    apply(&env, &cs);
    assert_eq!(text(), "Ravi paces beside the red car.");
}

#[test]
fn m1_07_story_character_edited_by_the_user_makes_the_proposal_stale() {
    let (env, _fx) = setup();
    let ravi: String = scalar(
        &env,
        "SELECT id FROM story_character WHERE name='Ravi'",
        &[],
    );
    let cs = stale_cycle(
        &env,
        "propose_update_character",
        json!({ "character": "Ravi", "description": "A clerk who waits for every train." }),
        ("story_character", &ravi),
        |env| {
            env.ok(
                "story.update_character",
                json!({ "id": ravi, "description": "A retired stationmaster." }),
            );
        },
    );
    let col = |c: &str| -> String {
        scalar(
            &env,
            &format!("SELECT COALESCE({c}, '') FROM story_character WHERE id=?1"),
            &[&ravi],
        )
    };
    assert_eq!(col("description"), "A retired stationmaster.");
    assert!(
        rows_text(&cs["preparedRows"]).contains("(empty) → A clerk who waits for every train."),
        "{}",
        rows_text(&cs["preparedRows"])
    );
    assert!(
        rows_text(&cs["rows"])
            .contains("A retired stationmaster. → A clerk who waits for every train."),
        "{}",
        rows_text(&cs["rows"])
    );
    apply(&env, &cs);
    assert_eq!(col("description"), "A clerk who waits for every train.");
}

#[test]
fn m1_07_breakdown_element_edited_by_the_user_makes_the_proposal_stale() {
    let (env, fx) = setup();
    let el: String = scalar(
        &env,
        "SELECT id FROM breakdown_element WHERE deleted_at IS NULL LIMIT 1",
        &[],
    );
    let cs = stale_cycle(
        &env,
        "propose_breakdown_note",
        json!({ "sceneNumber": scene_no(&env, &fx.scenes[0]), "element": "Red Car", "notes": "Needs a working horn." }),
        ("breakdown_element", &el),
        |env| {
            env.ok(
                "breakdown.update_element",
                json!({ "id": el, "notes": "Borrowed from the museum." }),
            );
        },
    );
    let notes = || -> String {
        scalar(
            &env,
            "SELECT COALESCE(notes, '') FROM breakdown_element WHERE id=?1",
            &[&el],
        )
    };
    assert_eq!(notes(), "Borrowed from the museum.");
    assert!(
        rows_text(&cs["rows"]).contains("Borrowed from the museum. → Needs a working horn."),
        "{}",
        rows_text(&cs["rows"])
    );
    apply(&env, &cs);
    assert_eq!(notes(), "Needs a working horn.");
}

#[test]
fn m1_07_schedule_day_edited_by_the_user_makes_the_proposal_stale() {
    let (env, fx) = setup();
    let cs = stale_cycle(
        &env,
        "propose_update_shooting_day",
        json!({ "day": "Day 1", "notes": "Night shoot at the station." }),
        ("shooting_day", &fx.day),
        |env| {
            env.ok(
                "schedule.set_day_notes",
                json!({ "dayId": fx.day, "notes": "Rain cover booked." }),
            );
        },
    );
    let notes = || -> String {
        scalar(
            &env,
            "SELECT COALESCE(notes, '') FROM shooting_day WHERE id=?1",
            &[&fx.day],
        )
    };
    assert_eq!(notes(), "Rain cover booked.");
    assert!(
        rows_text(&cs["rows"]).contains("Rain cover booked. → Night shoot at the station."),
        "{}",
        rows_text(&cs["rows"])
    );
    apply(&env, &cs);
    assert_eq!(notes(), "Night shoot at the station.");
}

#[test]
fn m1_07_call_sheet_edited_by_the_user_makes_the_proposal_stale() {
    let (env, fx) = setup();
    env.ok("callsheets.create", json!({ "dayId": fx.day }));
    let (sheet, title): (String, String) = env
        .core
        .project()
        .unwrap()
        .store
        .read(|c| {
            Ok(c.query_row(
                "SELECT id, title FROM call_sheet WHERE deleted_at IS NULL LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?)
        })
        .unwrap();
    let doc = || -> Value {
        let s: String = scalar(
            &env,
            "SELECT document_json FROM call_sheet WHERE id=?1",
            &[&sheet],
        );
        serde_json::from_str(&s).unwrap()
    };
    // The assistant proposes the weather; the user then sets the crew call (another field of
    // the same call sheet document).
    let cs = stale_cycle(
        &env,
        "propose_update_call_sheet",
        json!({ "callSheet": title, "weather": "Clear, 12°C" }),
        ("call_sheet", &sheet),
        |env| {
            let mut d = doc();
            d["crewCall"] = json!("06:30");
            env.ok("callsheets.update", json!({ "id": sheet, "document": d }));
        },
    );
    assert_eq!(doc()["crewCall"], "06:30");
    apply(&env, &cs);
    let d = doc();
    assert_eq!(
        d["crewCall"], "06:30",
        "the rebuilt proposal keeps the user's crew call"
    );
    assert!(
        d.to_string().contains("Clear, 12°C"),
        "the weather was applied: {d}"
    );
}

#[test]
fn m1_07_cast_and_crew_member_edited_by_the_user_makes_the_proposal_stale() {
    let (env, _fx) = setup();
    let cast: String = scalar(
        &env,
        "SELECT id FROM cast_member WHERE person_name='Arjun Rao'",
        &[],
    );
    let cs = stale_cycle(
        &env,
        "propose_update_cast_member",
        json!({ "castMember": "Arjun Rao", "availability": "Weekends only" }),
        ("cast_member", &cast),
        |env| {
            env.ok("cast.update", json!({ "id": cast, "notes": "Agent: Sam" }));
        },
    );
    let col = |c: &str| -> String {
        scalar(
            &env,
            &format!("SELECT COALESCE({c}, '') FROM cast_member WHERE id=?1"),
            &[&cast],
        )
    };
    apply(&env, &cs);
    assert_eq!(col("availability_notes"), "Weekends only");
    assert_eq!(col("notes"), "Agent: Sam", "the user's edit is kept");

    // Crew: same cycle on a crew member.
    let crew: String = scalar(
        &env,
        "SELECT id FROM crew_member WHERE person_name='Priya'",
        &[],
    );
    let cs = stale_cycle(
        &env,
        "propose_update_crew_member",
        json!({ "crewMember": "Priya", "notes": "Has her own lenses" }),
        ("crew_member", &crew),
        |env| {
            env.ok(
                "crew.update",
                json!({ "id": crew, "notes": "Prefers night work" }),
            );
        },
    );
    assert!(
        rows_text(&cs["rows"]).contains("Prefers night work → Has her own lenses"),
        "{}",
        rows_text(&cs["rows"])
    );
    apply(&env, &cs);
    let notes: String = scalar(&env, "SELECT notes FROM crew_member WHERE id=?1", &[&crew]);
    assert_eq!(notes, "Has her own lenses");
}

#[test]
fn m1_07_budget_line_edited_by_the_user_makes_the_proposal_stale() {
    let (env, fx) = setup();
    env.ok(
        "budget.add_line",
        json!({ "budgetId": fx.budget, "category": "Locations", "description": "Station permit", "planned": 120000 }),
    );
    let line: String = scalar(
        &env,
        "SELECT id FROM budget_line WHERE description='Station permit'",
        &[],
    );
    // The assistant proposes a new amount; the user then adds notes. The budget operation
    // writes every field, so applying the old proposal would erase the user's notes.
    let cs = stale_cycle(
        &env,
        "propose_update_budget_line",
        json!({ "line": "Station permit", "planned": 1500 }),
        ("budget_line", &line),
        |env| {
            env.ok(
                "budget.update_line",
                json!({ "id": line, "category": "Locations", "description": "Station permit", "planned": 120000, "notes": "Quote from the railway office" }),
            );
        },
    );
    apply(&env, &cs);
    let (amount, notes): (i64, Option<String>) = env
        .core
        .project()
        .unwrap()
        .store
        .read(|c| {
            Ok(c.query_row(
                "SELECT amount, notes FROM budget_line WHERE id=?1",
                [&line],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?)
        })
        .unwrap();
    assert_eq!(amount, 150000);
    assert_eq!(
        notes.as_deref(),
        Some("Quote from the railway office"),
        "the rebuilt proposal carries the user's notes"
    );
}

#[test]
fn m1_07_project_note_edited_by_the_user_makes_the_proposal_stale() {
    let (env, _fx) = setup();
    let note: String = scalar(&env, "SELECT id FROM project_note WHERE title='Ideas'", &[]);
    let cs = stale_cycle(
        &env,
        "propose_update_project_note",
        json!({ "note": "Ideas", "body": "Night shoots need a second generator." }),
        ("project_note", &note),
        |env| {
            env.ok(
                "notes.update",
                json!({ "id": note, "body": "Night shoots are hard and cold." }),
            );
        },
    );
    let body = || -> String { scalar(&env, "SELECT body FROM project_note WHERE id=?1", &[&note]) };
    assert_eq!(body(), "Night shoots are hard and cold.");
    assert!(
        rows_text(&cs["rows"])
            .contains("Night shoots are hard and cold. → Night shoots need a second generator."),
        "{}",
        rows_text(&cs["rows"])
    );
    apply(&env, &cs);
    assert_eq!(body(), "Night shoots need a second generator.");
}

#[test]
fn m1_07_project_note_deleted_by_the_user_makes_the_proposal_a_conflict() {
    let (env, _fx) = setup();
    let note: String = scalar(&env, "SELECT id FROM project_note WHERE title='Ideas'", &[]);
    let d = propose(
        &env,
        "propose_update_project_note",
        json!({ "note": "Ideas", "body": "Night shoots need a second generator." }),
    );
    let cs = change_set::create(&env.core, &env.actor(), &d).unwrap();
    env.ok("notes.delete", json!({ "id": note }));
    let before = canonical(&env);
    let r = env.ok("ai.change_set.accept", json!({ "id": cs.id }));
    assert_eq!(r["state"], "Conflict", "{r}");
    assert!(
        r["staleReason"].as_str().unwrap().contains("deleted"),
        "{r}"
    );
    assert_unchanged(&env, &before, "deleted base row");
    // Re-check cannot rebuild a change to an item that no longer exists: still not applicable.
    let r = env.ok("ai.change_set.recheck", json!({ "id": cs.id }));
    assert_eq!(r["state"], "Conflict", "{r}");
    assert_eq!(r["validationState"], "Invalid", "{r}");
    assert_eq!(
        env.err("ai.change_set.accept", json!({ "id": cs.id })),
        "conflict.state"
    );
    assert_unchanged(&env, &before, "re-checked conflict");
}

#[test]
fn m1_07_visual_shot_edited_by_the_user_makes_the_proposal_stale() {
    let (env, fx) = setup();
    let shot: String = scalar(
        &env,
        "SELECT id FROM shot WHERE deleted_at IS NULL LIMIT 1",
        &[],
    );
    let cs = stale_cycle(
        &env,
        "propose_update_shot",
        json!({ "sceneNumber": scene_no(&env, &fx.scenes[0]), "shot": 1, "lens": "35mm" }),
        ("shot", &shot),
        |env| {
            env.ok(
                "shot.update",
                json!({ "id": shot, "cameraNotes": "Handheld, slow push" }),
            );
        },
    );
    apply(&env, &cs);
    let col = |c: &str| -> String {
        scalar(
            &env,
            &format!("SELECT COALESCE({c}, '') FROM shot WHERE id=?1"),
            &[&shot],
        )
    };
    assert_eq!(col("lens"), "35mm");
    assert_eq!(
        col("camera_notes"),
        "Handheld, slow push",
        "the user's edit is kept"
    );
}
