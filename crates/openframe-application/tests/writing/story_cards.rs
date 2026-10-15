//! Story Board card redesign and hardening (Story Board and Idea Vault area of the build plan;
//! Screenplay and Production Depth Specification sections 21 and 52; owner decision 2026-10-07 on
//! Build Screenplay; docs/engineering/33-production-depth-contract.md section 1).
//!
//! * the long scene description is a real field: saved, undoable, searchable, readable by the
//!   assistant, copied by Duplicate, carried by exchange packages, kept across restart and crash;
//! * Card Display Settings are per-user view settings and never change scene or screenplay data;
//! * card capacity limits (description, long description, notes, attachments);
//! * card to screenplay links follow the scene lineage when scene identities change;
//! * Build Screenplay asks for a heading or for the card to be left out, never invents one;
//! * a batch delete restores in the original order whichever entry comes back first.

use openframe_application::modules::ai::scope::{self, AiScopeArgs, AiScopeKind};
use openframe_application::modules::ai::toolbox::{self, ToolStep};
use openframe_domain::Role;
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

// ------------------------------------------------------------------ helpers

fn id(v: &Value) -> String {
    v["id"].as_str().expect("id").to_string()
}

fn act(env: &TestEnv, title: &str) -> String {
    id(&env.ok("story.create_act", json!({ "title": title })))
}

fn seq(env: &TestEnv, act_id: &str, title: &str) -> String {
    id(&env.ok(
        "story.create_sequence",
        json!({ "actId": act_id, "title": title }),
    ))
}

fn card_in(env: &TestEnv, parent_type: &str, parent_id: Option<&str>, desc: &str) -> String {
    id(&env.ok(
        "story.create_card",
        json!({ "parent": { "parentType": parent_type, "parentId": parent_id }, "shortDescription": desc }),
    ))
}

fn item(kind: &str, id: &str) -> Value {
    json!({ "kind": kind, "id": id })
}

/// Quit cleanly, start again and reopen the same project.
fn restart(env: TestEnv) -> TestEnv {
    let path = env.project_path();
    let env = env.restart();
    env.reopen_project(&path);
    env
}

/// The process disappears without closing the project; start again and reopen it.
fn crash(env: TestEnv) -> TestEnv {
    let path = env.project_path();
    let env = env.crash_and_restart();
    env.reopen_project(&path);
    env
}

fn card(env: &TestEnv, id: &str) -> Value {
    env.ok("story.card", json!({ "id": id }))["card"].clone()
}

fn kids_of(items: &Value) -> Vec<String> {
    items
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_str().unwrap().to_string())
        .collect()
}

fn seq_kids(env: &TestEnv, act_id: &str, seq_id: &str) -> Vec<String> {
    let b = env.ok("story.board", json!({}));
    let a = b["acts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == act_id)
        .unwrap()
        .clone();
    let s = a["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"] == seq_id)
        .unwrap()
        .clone();
    kids_of(&s["items"])
}

fn act_kids(env: &TestEnv, act_id: &str) -> Vec<String> {
    let b = env.ok("story.board", json!({}));
    let a = b["acts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == act_id)
        .unwrap()
        .clone();
    kids_of(&a["items"])
}

fn trash_entry(env: &TestEnv, object_id: &str) -> String {
    let t = env.ok("trash.list", json!({}));
    t.as_array()
        .unwrap()
        .iter()
        .find(|r| r["objectId"] == object_id)
        .unwrap_or_else(|| panic!("{object_id} not in trash: {t}"))["id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn count(env: &TestEnv, sql: &str) -> i64 {
    env.core
        .project()
        .unwrap()
        .store
        .read(|c| Ok(c.query_row(sql, [], |r| r.get::<_, i64>(0))?))
        .unwrap()
}

/// Every row of every story, screenplay, production and visual table, as text: a fingerprint of
/// all content a Card Display change must leave alone.
fn content_fingerprint(env: &TestEnv) -> String {
    env.core
        .project()
        .unwrap()
        .store
        .read(|c| {
            let tables: Vec<String> = {
                let mut st = c.prepare(
                    "SELECT name FROM sqlite_master WHERE type='table' AND (name LIKE 'story_%' OR name LIKE 'screenplay%'
                       OR name LIKE 'breakdown_%' OR name IN ('shot','storyboard','deleted_item','comment'))
                     ORDER BY name",
                )?;
                st.query_map([], |r| r.get(0))?
                    .collect::<Result<_, _>>()?
            };
            let mut out = String::new();
            for t in tables {
                let mut st = c.prepare(&format!("SELECT * FROM \"{t}\" ORDER BY rowid"))?;
                let n = st.column_count();
                let rows = st.query_map([], |r| {
                    let mut line = String::new();
                    for i in 0..n {
                        let v: rusqlite::types::Value = r.get(i)?;
                        line.push_str(&format!("{v:?}|"));
                    }
                    Ok(line)
                })?;
                out.push_str(&t);
                out.push('\n');
                for r in rows {
                    out.push_str(&r?);
                    out.push('\n');
                }
            }
            Ok(out)
        })
        .unwrap()
}

fn ai_read(env: &TestEnv, tool: &str, args: Value) -> String {
    let actor = env.actor();
    let s = env.core.project().unwrap();
    let scope = s
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
        Ok(ToolStep::Proposal(_)) => panic!("{tool} proposed instead of reading"),
        Err(e) => panic!("{tool} failed: {} {}", e.code, e.message),
    }
}

const TELUGU: &str = "రైలు స్టేషన్‌లో అర్జున్ ఒంటరిగా నిలబడ్డాడు. వర్షం ఆగలేదు.";
const HINDI: &str = "अर्जुन प्लेटफ़ॉर्म पर अकेला खड़ा है। बारिश रुकती नहीं।";

// ------------------------------------------------------- long scene description

#[test]
fn long_scene_description_is_a_real_field_saved_undone_and_kept_after_restart() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let a = act(&env, "Act 1");
    let c = card_in(&env, "act", Some(&a), "Hero returns to the station");
    assert_eq!(card(&env, &c)["longDescription"], Value::Null);

    let long = format!("The platform is empty except for a flickering lamp.\n\n{TELUGU}\n{HINDI}");
    env.ok(
        "story.update_card",
        json!({ "id": c, "longDescription": long }),
    );
    let got = card(&env, &c);
    assert_eq!(got["longDescription"], long.as_str());
    assert_eq!(
        got["shortDescription"], "Hero returns to the station",
        "the short description is a separate field"
    );
    // The board carries it too (the expanded card and the preview read it from there).
    let b = env.ok("story.board", json!({}));
    assert_eq!(b["acts"][0]["items"][0]["longDescription"], long.as_str());

    // Undo and redo.
    env.undo();
    assert_eq!(card(&env, &c)["longDescription"], Value::Null);
    env.redo();
    assert_eq!(card(&env, &c)["longDescription"], long.as_str());

    // Blank clears it; trailing spaces are not stored.
    env.ok(
        "story.update_card",
        json!({ "id": c, "longDescription": "   " }),
    );
    assert_eq!(card(&env, &c)["longDescription"], Value::Null);
    env.ok(
        "story.update_card",
        json!({ "id": c, "longDescription": format!("{long}   \n") }),
    );
    assert_eq!(card(&env, &c)["longDescription"], long.as_str());

    // Restart and crash keep it.
    let env = restart(env);
    assert_eq!(card(&env, &c)["longDescription"], long.as_str());
    let env = crash(env);
    assert_eq!(card(&env, &c)["longDescription"], long.as_str());
}

#[test]
fn long_scene_description_can_be_given_when_the_card_is_created_and_is_copied_by_duplicate() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 1");
    let c = id(&env.ok(
        "story.create_card",
        json!({
            "parent": { "parentType": "act", "parentId": a },
            "shortDescription": "Chase at the level crossing",
            "longDescription": "Ravi runs; Arjun follows through the crossing as the gates come down.",
            "notes": "Night shoot, needs a rail permit."
        }),
    ));
    let got = card(&env, &c);
    assert_eq!(
        got["longDescription"],
        "Ravi runs; Arjun follows through the crossing as the gates come down."
    );
    assert_eq!(got["notes"], "Night shoot, needs a rail permit.");
    let dup = env.ok(
        "story.duplicate_items",
        json!({ "items": [item("card", &c)] }),
    )["ids"][0]
        .as_str()
        .unwrap()
        .to_string();
    assert_ne!(dup, c);
    assert_eq!(card(&env, &dup)["longDescription"], got["longDescription"]);
    // Independent copies: editing the copy leaves the original alone.
    env.ok(
        "story.update_card",
        json!({ "id": dup, "longDescription": "Changed in the copy." }),
    );
    assert_eq!(card(&env, &c)["longDescription"], got["longDescription"]);
}

#[test]
fn long_scene_description_is_searchable_and_leaves_search_when_cleared_or_deleted() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 1");
    let c = card_in(&env, "act", Some(&a), "Morgue scene");
    let hits = |q: &str| env.ok("search.query", json!({ "text": q }));
    assert!(hits("photograph").as_array().unwrap().is_empty());
    env.ok(
        "story.update_card",
        json!({ "id": c, "longDescription": "Arjun finds the childhood photograph inside the file." }),
    );
    let h = hits("photograph");
    assert_eq!(h[0]["entityId"], c.as_str());
    assert_eq!(h[0]["nav"]["cardId"], c.as_str());
    assert_eq!(h[0]["title"], "Morgue scene");
    // Indian scripts are searchable too.
    env.ok(
        "story.update_card",
        json!({ "id": c, "longDescription": TELUGU }),
    );
    assert_eq!(hits("అర్జున్")[0]["entityId"], c.as_str());
    assert!(hits("photograph").as_array().unwrap().is_empty());
    // A card with only a long description is still found and titled by it.
    let only = card_in(&env, "act", Some(&a), "");
    env.ok(
        "story.update_card",
        json!({ "id": only, "longDescription": "Lighthouse beam sweeps the bay" }),
    );
    let h = hits("Lighthouse");
    assert_eq!(h[0]["entityId"], only.as_str());
    assert!(h[0]["title"].as_str().unwrap().starts_with("Lighthouse"));
    env.ok(
        "story.delete_items",
        json!({ "items": [item("card", &only)] }),
    );
    assert!(hits("Lighthouse").as_array().unwrap().is_empty());
}

#[test]
fn long_scene_description_is_readable_by_the_assistant_and_editable_only_by_proposal() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 1");
    let c = card_in(&env, "act", Some(&a), "Ravi waits at the station");
    env.ok(
        "story.update_card",
        json!({ "id": c, "longDescription": "The last train is late and the platform lights flicker." }),
    );
    let out = ai_read(&env, "story_card", json!({ "card": "Ravi waits" }));
    assert!(
        out.contains("Long description") && out.contains("platform lights flicker"),
        "{out}"
    );
    // The proposal tool can prepare a long description edit; nothing is written until applied.
    let actor = env.actor();
    let s = env.core.project().unwrap();
    let scope = s
        .store
        .read(|cn| {
            scope::resolve(
                cn,
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
    let step = toolbox::run_tool_unlocked(
        &env.core,
        &actor,
        &scope,
        "test request",
        "propose_update_scene_card",
        &json!({ "card": "Ravi waits", "longDescription": "The train never comes." }),
    )
    .unwrap();
    let ToolStep::Proposal(d) = step else {
        panic!("expected a proposal")
    };
    assert_eq!(
        card(&env, &c)["longDescription"],
        "The last train is late and the platform lights flicker.",
        "a proposal writes nothing"
    );
    for op in &d.operations {
        env.call(&op.op, op.args.clone()).unwrap();
    }
    assert_eq!(card(&env, &c)["longDescription"], "The train never comes.");

    // A long description longer than the assistant can read is never replaced by it.
    let long = "Rain. ".repeat(1000);
    env.ok(
        "story.update_card",
        json!({ "id": c, "longDescription": long }),
    );
    let refused = toolbox::run_tool_unlocked(
        &env.core,
        &actor,
        &scope,
        "test request",
        "propose_update_scene_card",
        &json!({ "card": "Ravi waits", "longDescription": "Short." }),
    );
    assert!(
        refused.is_err(),
        "the assistant must not replace text it cannot see"
    );
    assert_eq!(card(&env, &c)["longDescription"], long.trim_end());
}

#[test]
fn viewers_cannot_edit_the_long_description() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 1");
    let c = card_in(&env, "act", Some(&a), "x");
    for role in [Role::Viewer, Role::Commenter] {
        let who = env.actor_with_role(role);
        let e = env
            .call_as(
                &who,
                "story.update_card",
                json!({ "id": c, "longDescription": "nope" }),
            )
            .unwrap_err();
        assert_eq!(e.code.0, "permission.denied");
    }
    assert_eq!(card(&env, &c)["longDescription"], Value::Null);
}

// ------------------------------------------------------------- capacity limits

#[test]
fn card_capacity_limits_are_enforced_with_clear_messages() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 1");
    let c = card_in(&env, "act", Some(&a), "Limits");

    // Long description: 100,000 characters, counted as characters (Telugu takes 3 bytes each).
    let at_limit: String = "అ".repeat(100_000);
    env.ok(
        "story.update_card",
        json!({ "id": c, "longDescription": at_limit }),
    );
    assert_eq!(
        card(&env, &c)["longDescription"]
            .as_str()
            .unwrap()
            .chars()
            .count(),
        100_000
    );
    let over = env
        .call(
            "story.update_card",
            json!({ "id": c, "longDescription": "a".repeat(100_001) }),
        )
        .unwrap_err();
    assert_eq!(over.code.0, "validation.invalid_input");
    assert!(
        over.message.contains("100,000 characters"),
        "{}",
        over.message
    );
    assert_eq!(
        card(&env, &c)["longDescription"]
            .as_str()
            .unwrap()
            .chars()
            .count(),
        100_000,
        "a refused edit changes nothing"
    );

    // Short description: 20,000 bytes.
    env.ok(
        "story.update_card",
        json!({ "id": c, "shortDescription": "b".repeat(20_000) }),
    );
    assert_eq!(
        env.err(
            "story.update_card",
            json!({ "id": c, "shortDescription": "b".repeat(20_001) })
        ),
        "validation.invalid_input"
    );

    // Notes: 500,000 bytes.
    env.ok(
        "story.update_card",
        json!({ "id": c, "notes": "n".repeat(500_000) }),
    );
    assert_eq!(
        env.err(
            "story.update_card",
            json!({ "id": c, "notes": "n".repeat(500_001) })
        ),
        "validation.invalid_input"
    );

    // Attachments: 50 per card, the 51st is refused and nothing is copied for it.
    let file = env.write_file("plan.jpg", b"\xFF\xD8\xFF\xE0 not really a jpeg");
    for _ in 0..50 {
        env.ok(
            "story.add_attachment",
            json!({ "ownerType": "scene_card", "ownerId": c, "path": file }),
        );
    }
    assert_eq!(card(&env, &c)["attachments"].as_array().unwrap().len(), 50);
    let assets_before = count(&env, "SELECT count(*) FROM asset");
    let e = env
        .call(
            "story.add_attachment",
            json!({ "ownerType": "scene_card", "ownerId": c, "path": file }),
        )
        .unwrap_err();
    assert_eq!(e.code.0, "validation.invalid_input");
    assert!(e.message.contains("up to 50 attachments"), "{}", e.message);
    assert_eq!(count(&env, "SELECT count(*) FROM asset"), assets_before);
    // Removing one makes room again.
    let first = card(&env, &c)["attachments"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    env.ok("story.remove_attachment", json!({ "id": first }));
    env.ok(
        "story.add_attachment",
        json!({ "ownerType": "scene_card", "ownerId": c, "path": file }),
    );
    // Everything still renders: the board and the card load with the full content.
    let b = env.ok("story.board", json!({}));
    assert_eq!(
        b["acts"][0]["items"][0]["attachments"]
            .as_array()
            .unwrap()
            .len(),
        50
    );
}

// ------------------------------------------------------- card display settings

#[test]
fn card_display_defaults_to_minimal_and_offers_three_presets_plus_custom() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let st = env.ok("story.card_display", json!({}));
    assert_eq!(st["current"]["preset"], "minimal");
    assert_eq!(st["current"]["toggles"]["shortDescription"], true);
    assert_eq!(st["current"]["toggles"]["sceneHeading"], false);
    let names: Vec<&str> = st["presets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["label"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Minimal", "Story", "Production"]);
    // All twelve toggles are present.
    assert_eq!(
        st["current"]["toggles"].as_object().unwrap().len(),
        12,
        "{st}"
    );

    let story = env.ok("story.set_card_display", json!({ "preset": "story" }));
    assert_eq!(story["current"]["preset"], "story");
    assert_eq!(story["current"]["toggles"]["longDescriptionPreview"], true);
    let prod = env.ok("story.set_card_display", json!({ "preset": "production" }));
    assert_eq!(prod["current"]["toggles"]["breakdownStatus"], true);
    assert_eq!(prod["current"]["toggles"]["shotStatus"], true);

    // Custom keeps exactly the boxes the user ticked.
    let custom = json!({
        "sceneHeading": false, "shortDescription": false, "longDescriptionPreview": true, "notes": false,
        "attachments": true, "characterCount": false, "location": true, "dayNight": false, "comments": false,
        "revisionChange": false, "breakdownStatus": false, "shotStatus": true
    });
    let got = env.ok(
        "story.set_card_display",
        json!({ "preset": "custom", "toggles": custom }),
    );
    assert_eq!(got["current"]["preset"], "custom");
    assert_eq!(got["current"]["toggles"], custom);
    assert_eq!(
        env.err("story.set_card_display", json!({ "preset": "custom" })),
        "validation.invalid_input"
    );

    // Saved per user and kept after restart and after a crash.
    let env = restart(env);
    assert_eq!(
        env.ok("story.card_display", json!({}))["current"]["toggles"],
        custom
    );
    let env = crash(env);
    assert_eq!(
        env.ok("story.card_display", json!({}))["current"]["toggles"],
        custom
    );
}

#[test]
fn card_display_is_per_user_and_viewers_may_choose_their_own() {
    let env = TestEnv::with_project("Film", "Feature Film");
    env.ok("story.set_card_display", json!({ "preset": "production" }));
    let viewer = env.other_user(Role::Viewer);
    let theirs = env
        .call_as(&viewer, "story.card_display", json!({}))
        .unwrap();
    assert_eq!(
        theirs["current"]["preset"], "minimal",
        "another user's view is their own"
    );
    env.call_as(
        &viewer,
        "story.set_card_display",
        json!({ "preset": "story" }),
    )
    .unwrap();
    assert_eq!(
        env.ok("story.card_display", json!({}))["current"]["preset"],
        "production"
    );
}

#[test]
fn changing_card_display_never_changes_scene_or_screenplay_data() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let a = act(&env, "Act 1");
    let s = seq(&env, &a, "Railway Station Return");
    let c1 = card_in(&env, "sequence", Some(&s), "Hero returns to the station");
    let c2 = card_in(&env, "sequence", Some(&s), "He finds the photograph");
    for (c, h) in [
        (&c1, "EXT. OLD RAILWAY STATION - NIGHT"),
        (&c2, "INT. WAITING ROOM - NIGHT"),
    ] {
        env.ok(
            "story.update_card",
            json!({ "id": c, "sceneHeading": h, "longDescription": "Long text", "notes": "A note" }),
        );
    }
    env.ok(
        "story.build_screenplay",
        json!({ "include": [{ "cardId": c1 }, { "cardId": c2 }], "destination": "newScreenplay" }),
    );
    let before = content_fingerprint(&env);
    let undo_before = count(&env, "SELECT count(*) FROM sys_undo");
    let activity_before = count(&env, "SELECT count(*) FROM sys_activity");
    for preset in ["minimal", "story", "production", "minimal"] {
        env.ok("story.set_card_display", json!({ "preset": preset }));
        // Reading the board with the new layout changes nothing either.
        env.ok("story.board", json!({}));
    }
    for flip in [true, false] {
        env.ok(
            "story.set_card_display",
            json!({ "preset": "custom", "toggles": {
                "sceneHeading": flip, "shortDescription": !flip, "longDescriptionPreview": flip, "notes": flip,
                "attachments": flip, "characterCount": flip, "location": flip, "dayNight": flip, "comments": flip,
                "revisionChange": flip, "breakdownStatus": flip, "shotStatus": flip
            }}),
        );
    }
    assert_eq!(
        content_fingerprint(&env),
        before,
        "card layout changed scene or screenplay data"
    );
    assert_eq!(
        count(&env, "SELECT count(*) FROM sys_undo"),
        undo_before,
        "display changes never enter undo"
    );
    assert_eq!(
        count(&env, "SELECT count(*) FROM sys_activity"),
        activity_before,
        "display changes are not project activity"
    );
}

#[test]
fn board_cards_carry_what_the_display_toggles_need() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 1");
    let c = card_in(&env, "act", Some(&a), "Crime scene in the rain");
    env.ok(
        "story.update_card",
        json!({ "id": c, "sceneHeading": "EXT. CRIME SCENE - NIGHT" }),
    );
    let got = card(&env, &c);
    assert_eq!(got["setName"], "Crime Scene");
    assert_eq!(got["timeOfDay"], "NIGHT");
    assert_eq!(got["sceneStatus"], Value::Null, "not built yet");
    env.ok(
        "story.build_screenplay",
        json!({ "include": [{ "cardId": c }], "destination": "newScreenplay" }),
    );
    let got = card(&env, &c);
    let st = &got["sceneStatus"];
    assert_eq!(st["draftName"], "Draft 1 from Story Board");
    assert_eq!(st["revised"], false);
    assert_eq!(st["breakdownItems"], 0);
    assert_eq!(st["shots"], 0);
    assert_eq!(st["storyboards"], 0);
    // A card without its own heading reads place and time from the linked scene.
    env.ok("story.update_card", json!({ "id": c, "sceneHeading": "" }));
    let got = card(&env, &c);
    assert_eq!(got["sceneHeading"], Value::Null);
    assert_eq!(got["setName"], "Crime Scene");
    assert_eq!(got["timeOfDay"], "NIGHT");
}

// ------------------------------------------------- links follow scene identity

#[test]
fn card_links_follow_the_scene_into_a_new_current_draft_and_drop_when_removed() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 1");
    let c1 = card_in(&env, "act", Some(&a), "Bus stop");
    let c2 = card_in(&env, "act", Some(&a), "Police station");
    for (c, h) in [
        (&c1, "EXT. BUS STOP - NIGHT"),
        (&c2, "INT. POLICE STATION - DAY"),
    ] {
        env.ok("story.update_card", json!({ "id": c, "sceneHeading": h }));
    }
    let built = env.ok(
        "story.build_screenplay",
        json!({ "include": [{ "cardId": c1 }, { "cardId": c2 }], "destination": "newScreenplay" }),
    );
    let d1 = built["draftId"].as_str().unwrap().to_string();
    let scene1_d1 = card(&env, &c1)["screenplaySceneId"]
        .as_str()
        .unwrap()
        .to_string();

    // A new draft copies every scene with a new id; the card now points into it.
    let d2 = env.ok(
        "screenplay.new_draft",
        json!({ "sourceDraftId": d1, "name": "Draft 2" }),
    )["id"]
        .as_str()
        .unwrap()
        .to_string();
    let resolved = card(&env, &c1)["screenplaySceneId"]
        .as_str()
        .unwrap()
        .to_string();
    assert_ne!(
        resolved, scene1_d1,
        "the link follows the scene to the current draft"
    );
    assert_eq!(
        count(
            &env,
            &format!(
                "SELECT count(*) FROM screenplay_scene WHERE id='{resolved}' AND draft_id='{d2}'"
            )
        ),
        1
    );
    let detail = env.ok("story.card", json!({ "id": c1 }));
    assert_eq!(detail["linkedScene"]["draftId"], d2.as_str());
    assert_eq!(detail["linkedScene"]["number"], 1);
    assert_eq!(detail["card"]["sceneStatus"]["draftName"], "Draft 2");
    // The stored record of what the card created is unchanged.
    assert_eq!(
        count(
            &env,
            &format!(
                "SELECT count(*) FROM story_scene_card WHERE id='{c1}' AND screenplay_scene_id='{scene1_d1}'"
            )
        ),
        1
    );
    // Search from the screenplay side still finds the card through the board.
    let board = env.ok("story.board", json!({}));
    assert_eq!(
        board["acts"][0]["items"][0]["screenplaySceneId"],
        resolved.as_str()
    );

    // Switching back to Draft 1 points the card at Draft 1 again.
    env.ok("screenplay.set_current_draft", json!({ "draftId": d1 }));
    assert_eq!(card(&env, &c1)["screenplaySceneId"], scene1_d1.as_str());
    env.ok("screenplay.set_current_draft", json!({ "draftId": d2 }));

    // The scene is deleted in the current draft: the link falls back to the draft that still has it.
    env.ok("screenplay.delete_scene", json!({ "sceneId": resolved }));
    assert_eq!(card(&env, &c1)["screenplaySceneId"], scene1_d1.as_str());
    assert_eq!(
        env.ok("story.card", json!({ "id": c1 }))["linkedScene"]["draftId"],
        d1.as_str()
    );
    // Removed from every draft: the card shows no link and deleting it needs no warning.
    let scene2_d1 = card(&env, &c2)["screenplaySceneId"].clone();
    assert!(scene2_d1.is_string());
    env.ok("screenplay.delete_scene", json!({ "sceneId": scene1_d1 }));
    assert_eq!(card(&env, &c1)["screenplaySceneId"], Value::Null);
    assert_eq!(
        env.ok("story.card", json!({ "id": c1 }))["linkedScene"],
        Value::Null
    );
    // Undo brings the scene and therefore the link back.
    env.undo();
    assert_eq!(card(&env, &c1)["screenplaySceneId"], scene1_d1.as_str());
    // The assistant reads the same link.
    let out = ai_read(&env, "story_card", json!({ "card": "Bus stop" }));
    assert!(out.contains("Screenplay:"), "{out}");
    // Survives restart.
    let env = restart(env);
    assert_eq!(card(&env, &c1)["screenplaySceneId"], scene1_d1.as_str());
}

// ---------------------------------------------------- Build Screenplay decision

#[test]
fn build_screenplay_asks_for_a_heading_or_leaving_the_card_out_and_never_invents_one() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 1");
    let with = card_in(&env, "act", Some(&a), "Bus stop");
    let without = card_in(&env, "act", Some(&a), "Morning at the old house");
    env.ok(
        "story.update_card",
        json!({ "id": with, "sceneHeading": "EXT. BUS STOP - NIGHT" }),
    );
    let p = env.ok("story.build_preview", json!({}));
    assert_eq!(p["rows"][1]["headingValid"], false);
    assert_eq!(p["rows"][1]["sceneHeading"], Value::Null);

    // Never builds silently: the card without a heading blocks the build, nothing is created.
    let e = env
        .call(
            "story.build_screenplay",
            json!({ "include": [{ "cardId": with }, { "cardId": without }], "destination": "newScreenplay" }),
        )
        .unwrap_err();
    assert_eq!(e.code.0, "validation.heading_needed");
    assert!(
        e.message.contains("Add a heading or exclude it"),
        "{}",
        e.message
    );
    assert!(!e.message.contains('\u{2014}'), "plain wording");
    assert_eq!(count(&env, "SELECT count(*) FROM screenplay"), 0);
    // Text that is not a heading is refused the same way.
    assert_eq!(
        env.err(
            "story.build_screenplay",
            json!({ "include": [{ "cardId": without, "heading": "the old house" }], "destination": "newScreenplay" }),
        ),
        "validation.heading_needed"
    );
    assert_eq!(count(&env, "SELECT count(*) FROM screenplay"), 0);

    // Leave the card out: only the headed card is built, the other stays as it was.
    let r = env.ok(
        "story.build_screenplay",
        json!({ "include": [{ "cardId": with }], "destination": "newScreenplay" }),
    );
    assert_eq!(r["sceneCount"], 1);
    assert_eq!(card(&env, &without)["sceneHeading"], Value::Null);
    assert_eq!(card(&env, &without)["screenplaySceneId"], Value::Null);
    let headings = |draft: &str| -> Vec<String> {
        env.core
            .project()
            .unwrap()
            .store
            .read(|c| {
                let mut st = c.prepare(
                    "SELECT heading FROM screenplay_scene WHERE draft_id=?1 AND deleted_at IS NULL ORDER BY position",
                )?;
                Ok(st
                    .query_map([draft], |r| r.get::<_, String>(0))?
                    .collect::<Result<Vec<_>, _>>()?)
            })
            .unwrap()
    };
    assert_eq!(
        headings(r["draftId"].as_str().unwrap()),
        ["EXT. BUS STOP - NIGHT"]
    );

    // Or add the heading in the preview: it is used exactly as typed (upper case) and saved on the card.
    let sp = r["screenplayId"].as_str().unwrap().to_string();
    let r2 = env.ok(
        "story.build_screenplay",
        json!({ "include": [{ "cardId": with }, { "cardId": without, "heading": "int. old house - morning" }],
                "destination": "newDraft", "screenplayId": sp }),
    );
    assert_eq!(r2["draftName"], "Draft 2 from Story Board");
    assert_eq!(
        headings(r2["draftId"].as_str().unwrap()),
        ["EXT. BUS STOP - NIGHT", "INT. OLD HOUSE - MORNING"]
    );
    assert_eq!(
        card(&env, &without)["sceneHeading"],
        "int. old house - morning"
    );
}

// ------------------------------------------------ batch delete restore order

/// Deleting several cards at once and restoring them one by one from Recently Deleted puts them
/// back in their original order, whichever entry the user restores first (session 1 defect).
#[test]
fn batch_delete_restores_the_original_order_in_any_restore_order() {
    let orders: [[usize; 3]; 6] = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    for order in orders {
        let env = TestEnv::with_project("Film", "Feature Film");
        let a = act(&env, "Act 1");
        let s = seq(&env, &a, "S");
        let cards: Vec<String> = ["one", "two", "three", "four", "five"]
            .iter()
            .map(|d| card_in(&env, "sequence", Some(&s), d))
            .collect();
        // Delete one, two and four together (not contiguous on purpose).
        let deleted = [cards[0].clone(), cards[1].clone(), cards[3].clone()];
        env.ok(
            "story.delete_items",
            json!({ "items": deleted.iter().map(|c| item("card", c)).collect::<Vec<_>>() }),
        );
        assert_eq!(
            seq_kids(&env, &a, &s),
            vec![cards[2].clone(), cards[4].clone()]
        );
        for &i in &order {
            let entry = trash_entry(&env, &deleted[i]);
            env.ok("trash.restore", json!({ "id": entry }));
        }
        assert_eq!(
            seq_kids(&env, &a, &s),
            cards,
            "restore order {order:?} changed the card order"
        );
    }
}

#[test]
fn partial_restore_of_a_batch_keeps_relative_order_and_mixed_beats_work() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 1");
    let c1 = card_in(&env, "act", Some(&a), "one");
    let b2 = id(&env.ok(
        "story.create_beat",
        json!({ "parent": { "parentType": "act", "parentId": a }, "text": "beat two" }),
    ));
    let c3 = card_in(&env, "act", Some(&a), "three");
    let c4 = card_in(&env, "act", Some(&a), "four");
    env.ok(
        "story.delete_items",
        json!({ "items": [item("card", &c1), item("beat", &b2), item("card", &c3)] }),
    );
    assert_eq!(act_kids(&env, &a), vec![c4.clone()]);
    // Restore the last one first, then the first; leave the beat in Recently Deleted.
    env.ok("trash.restore", json!({ "id": trash_entry(&env, &c3) }));
    assert_eq!(act_kids(&env, &a), vec![c3.clone(), c4.clone()]);
    env.ok("trash.restore", json!({ "id": trash_entry(&env, &c1) }));
    assert_eq!(act_kids(&env, &a), vec![c1.clone(), c3.clone(), c4.clone()]);
    env.ok("trash.restore", json!({ "id": trash_entry(&env, &b2) }));
    assert_eq!(act_kids(&env, &a), vec![c1, b2, c3, c4]);
}

#[test]
fn separate_deletes_still_restore_to_their_own_places() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 1");
    let c: Vec<String> = ["one", "two", "three"]
        .iter()
        .map(|d| card_in(&env, "act", Some(&a), d))
        .collect();
    env.ok(
        "story.delete_items",
        json!({ "items": [item("card", &c[0])] }),
    );
    env.ok(
        "story.delete_items",
        json!({ "items": [item("card", &c[2])] }),
    );
    env.ok("trash.restore", json!({ "id": trash_entry(&env, &c[2]) }));
    env.ok("trash.restore", json!({ "id": trash_entry(&env, &c[0]) }));
    assert_eq!(act_kids(&env, &a), c);
}
