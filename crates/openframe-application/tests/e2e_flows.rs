//! Cross-module end-to-end flows, driven through the public operation registry
//! exactly as the desktop UI does (every call is `op name + JSON args`).
//!
//! These tests exist to catch the SEAMS between modules that were built in
//! parallel: Idea Vault → Story → Build Screenplay → Screenplay editing →
//! Production Source → Breakdown → Catalog / Locations / Cast → Shots &
//! Storyboards → Schedule → Call Sheet; script revisions after production has
//! started; screenplay import; the offline AI's deterministic tools; undo/redo,
//! restart persistence and Recently Deleted across modules.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use openframe_ai::{ChatModel, ChatRequest};
use openframe_application::modules::ai;
use openframe_domain::{AppError, AppResult};
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

// ----------------------------------------------------------------- helpers

fn id(v: &Value) -> String {
    v.as_str()
        .map(str::to_string)
        .or_else(|| v["id"].as_str().map(str::to_string))
        .unwrap_or_else(|| panic!("no id in {v}"))
}

fn count(env: &TestEnv, sql: &str) -> i64 {
    env.core
        .project()
        .unwrap()
        .store
        .read(|c| Ok(c.query_row(sql, [], |r| r.get::<_, i64>(0))?))
        .unwrap()
}

fn arr(v: &Value) -> &Vec<Value> {
    v.as_array().unwrap_or_else(|| panic!("not an array: {v}"))
}

fn scenes(env: &TestEnv, draft: &str) -> Vec<Value> {
    arr(&env.ok("screenplay.document", json!({ "draftId": draft }))["scenes"]).clone()
}

fn search_types(env: &TestEnv, text: &str) -> Vec<(String, Value)> {
    arr(&env.ok("search.query", json!({ "text": text })))
        .iter()
        .map(|h| {
            (
                h["entityType"].as_str().unwrap_or("").to_string(),
                h["nav"].clone(),
            )
        })
        .collect()
}

/// Everything the main flow creates, for the follow-up flows.
struct Film {
    vault_item: String,
    act: String,
    seq: String,
    card_station: String,
    card_bus: String,
    screenplay: String,
    draft: String,
    scene_station: String,
    scene_bus: String,
    lineage_station: String,
    lineage_bus: String,
    pistol: String,
    location: String,
    cast: String,
    shot: String,
    storyboard: String,
    schedule: String,
    day: String,
    call_sheet: String,
}

/// Idea Vault → Story → Build Screenplay → edit → Production Source →
/// Breakdown → Catalog/Locations/Cast → Shots/Storyboard → Schedule → Call Sheet.
fn build_film(env: &TestEnv) -> Film {
    // --- Idea Vault
    let item = id(&env.ok(
        "vault.create",
        json!({ "store": "project", "itemType": "note", "title": "Bus stop reunion",
                "body": "Arjun meets Meera again at a rainy bus stop." }),
    ));

    // --- Story: act + sequence, the vault idea is SENT (copied) as a card.
    let act = id(&env.ok("story.create_act", json!({ "title": "Act 1" })));
    let seq = id(&env.ok(
        "story.create_sequence",
        json!({ "actId": act, "title": "The return" }),
    ));
    let targets = env.ok("vault.story_targets", json!({}));
    assert_eq!(targets["acts"][0]["id"], act.as_str());
    let sent = env.ok(
        "vault.send_to_story",
        json!({ "store": "project", "id": item, "target": "sceneCard", "parentType": "sequence", "parentId": seq }),
    );
    assert_eq!(sent["table"], "story_scene_card");
    let card_bus = sent["storyId"].as_str().unwrap().to_string();
    // A copy: the vault item keeps its own identity and text.
    let v = env.ok("vault.get", json!({ "store": "project", "id": item }));
    assert_eq!(v["body"], "Arjun meets Meera again at a rainy bus stop.");
    let card = env.ok("story.card", json!({ "id": card_bus }));
    assert_ne!(card["card"]["id"], item.as_str());

    let card_station = id(&env.ok(
        "story.create_card",
        json!({ "parent": { "parentType": "sequence", "parentId": seq },
                "shortDescription": "Arjun confesses at the police station" }),
    ));
    env.ok(
        "story.update_card",
        json!({ "id": card_bus, "sceneHeading": "EXT. BUS STOP — NIGHT" }),
    );
    env.ok(
        "story.update_card",
        json!({ "id": card_station, "sceneHeading": "INT. POLICE STATION — NIGHT" }),
    );
    // Reorder: the station card goes first. Identity is kept.
    env.ok(
        "story.move_items",
        json!({ "items": [{ "kind": "card", "id": card_station }],
                "target": { "parentType": "sequence", "parentId": seq },
                "before": { "kind": "card", "id": card_bus } }),
    );
    let preview = env.ok("story.build_preview", json!({}));
    let order: Vec<&str> = arr(&preview["rows"])
        .iter()
        .map(|r| r["cardId"].as_str().unwrap())
        .collect();
    assert_eq!(order, vec![card_station.as_str(), card_bus.as_str()]);

    // --- Build Screenplay
    let built = env.ok(
        "story.build_screenplay",
        json!({ "include": [{ "cardId": card_station }, { "cardId": card_bus }], "destination": "newScreenplay" }),
    );
    let screenplay = built["screenplayId"].as_str().unwrap().to_string();
    let draft = built["draftId"].as_str().unwrap().to_string();
    let sc = scenes(env, &draft);
    assert_eq!(sc.len(), 2);
    assert_eq!(sc[0]["heading"], "INT. POLICE STATION — NIGHT");
    assert_eq!(sc[1]["heading"], "EXT. BUS STOP — NIGHT");
    assert_eq!(built["firstSceneId"], sc[0]["id"]);
    for s in &sc {
        for e in arr(&s["elements"]) {
            assert_ne!(
                e["elementType"], "scene_heading",
                "headings live on the scene row"
            );
        }
    }
    let scene_station = sc[0]["id"].as_str().unwrap().to_string();
    let scene_bus = sc[1]["id"].as_str().unwrap().to_string();
    let lineage_station = sc[0]["lineageId"].as_str().unwrap().to_string();
    let lineage_bus = sc[1]["lineageId"].as_str().unwrap().to_string();
    // The overview (what the Screenplay workspace opens) shows this screenplay.
    let ov = env.ok("screenplay.overview", json!({}));
    assert_eq!(ov["screenplay"]["id"], screenplay.as_str());
    assert_eq!(ov["screenplay"]["currentDraftId"], draft.as_str());

    // --- Write the scenes (element edits through the editor's batch op)
    let action_station = arr(&sc[0]["elements"])
        .iter()
        .find(|e| e["elementType"] == "action")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let action_bus = arr(&sc[1]["elements"])
        .iter()
        .find(|e| e["elementType"] == "action")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    env.ok(
        "screenplay.apply_edits",
        json!({ "draftId": draft, "ops": [
            { "op": "updateElement", "id": action_station, "text": "Arjun enters carrying a pistol and a red folder." },
            { "op": "insertElement", "sceneId": scene_station, "elementType": "character", "text": "ARJUN" },
            { "op": "insertElement", "sceneId": scene_station, "elementType": "dialogue", "text": "I was there that night." },
            { "op": "updateElement", "id": action_bus, "text": "Rain hammers the tin roof. MEERA waits under an umbrella." },
            { "op": "insertElement", "sceneId": scene_bus, "elementType": "character", "text": "MEERA" },
            { "op": "insertElement", "sceneId": scene_bus, "elementType": "dialogue", "text": "You came back." },
        ] }),
    );
    let sc = scenes(env, &draft);
    let texts: Vec<&str> = arr(&sc[0]["elements"])
        .iter()
        .map(|e| e["text"].as_str().unwrap())
        .collect();
    assert!(texts.contains(&"I was there that night."), "{texts:?}");
    // The written text is searchable and navigates to the scene in its draft.
    let hits = search_types(env, "pistol");
    let hit = hits
        .iter()
        .find(|(t, _)| t == "screenplay_scene")
        .expect("scene hit");
    assert_eq!(hit.1["workspace"], "screenplay");
    assert_eq!(hit.1["sceneId"], scene_station.as_str());
    assert_eq!(hit.1["draftId"], draft.as_str());

    // --- Production Source (the draft production works from)
    env.ok(
        "production.set_source",
        json!({ "draftId": draft, "reason": "Shooting draft" }),
    );
    assert_eq!(
        env.ok("production.source", json!({}))["draft"]["id"],
        draft.as_str()
    );

    // --- Breakdown suggestions → accept → Catalog
    let bs = env.ok("breakdown.scenes", json!({}));
    assert_eq!(arr(&bs["scenes"]).len(), 2);
    assert_eq!(bs["scenes"][0]["sceneId"], scene_station.as_str());
    let added = env.ok("breakdown.suggest", json!({ "sceneId": scene_station }));
    assert!(
        added["added"].as_i64().unwrap() > 0,
        "suggestions for the station scene: {added}"
    );
    let detail = env.ok("breakdown.scene", json!({ "sceneId": scene_station }));
    let pistol_sug = arr(&detail["groups"])
        .iter()
        .flat_map(|g| arr(&g["suggestions"]).clone())
        .find(|s| {
            s["name"]
                .as_str()
                .unwrap_or("")
                .to_lowercase()
                .contains("pistol")
        })
        .unwrap_or_else(|| panic!("a pistol suggestion: {detail}"));
    let accepted = env.ok("breakdown.accept", json!({ "id": pistol_sug["id"] }));
    assert_eq!(accepted["state"], "Confirmed");
    let pistol = accepted["catalogItemId"]
        .as_str()
        .expect("catalog item created")
        .to_string();
    let cat = env.ok("catalog.get", json!({ "id": pistol }));
    assert!(
        cat["name"]
            .as_str()
            .unwrap()
            .to_lowercase()
            .contains("pistol")
    );
    assert!(
        !arr(&cat["usedIn"]).is_empty(),
        "catalog item knows its scene: {cat}"
    );
    // The breakdown element is searchable only through its catalog item; the
    // catalog hit navigates to the Catalog tab with the item focused.
    let hits = search_types(env, "pistol");
    let cat_hit = hits
        .iter()
        .find(|(t, _)| t == "catalog_item")
        .expect("catalog hit");
    assert_eq!(cat_hit.1["sub"], "catalog");
    assert_eq!(cat_hit.1["catalogItemId"], pistol.as_str());

    // The rest of the suggestions (cast, set, …) are accepted in bulk, for both scenes.
    env.ok("breakdown.suggest", json!({ "sceneId": scene_bus }));
    for s in [&scene_station, &scene_bus] {
        let d = env.ok("breakdown.scene", json!({ "sceneId": s }));
        let ids: Vec<Value> = arr(&d["groups"])
            .iter()
            .flat_map(|g| {
                arr(&g["suggestions"])
                    .iter()
                    .map(|x| x["id"].clone())
                    .collect::<Vec<_>>()
            })
            .collect();
        if !ids.is_empty() {
            let r = env.ok("breakdown.accept_many", json!({ "ids": ids }));
            assert!(r["accepted"].as_i64().unwrap() > 0, "{r}");
        }
        let d = env.ok("breakdown.scene", json!({ "sceneId": s }));
        let cast_group = arr(&d["groups"])
            .iter()
            .find(|g| g["category"] == "Cast")
            .unwrap_or_else(|| panic!("cast group: {d}"));
        assert!(
            !arr(&cast_group["confirmed"]).is_empty(),
            "speaking character confirmed as cast: {d}"
        );
    }

    // --- Locations & cast
    let location = id(&env.ok(
        "locations.create",
        json!({ "name": "Police Station", "address": "12 Market Rd", "status": "Confirmed" }),
    ));
    let cast = id(&env.ok(
        "cast.create",
        json!({ "personName": "Karthik Menon", "characterName": "arjun" }),
    ));
    let cl = env.ok("cast.list", json!({}));
    let arjun = arr(&cl["characters"])
        .iter()
        .find(|c| c["name"] == "ARJUN")
        .expect("ARJUN in cast");
    assert_eq!(arjun["primaryCastId"], cast.as_str());
    let member = arr(&cl["members"])
        .iter()
        .find(|m| m["id"] == cast.as_str())
        .unwrap();
    assert!(
        !arr(&member["scenes"]).is_empty(),
        "ARJUN speaks in the station scene: {member}"
    );

    // --- Shots & storyboard for the station scene
    let shot = id(&env.ok(
        "shot.create",
        json!({ "sceneId": scene_station, "description": "Wide: Arjun at the counter" }),
    ));
    let storyboard = id(&env.ok("storyboard.create", json!({ "sceneId": scene_station })));
    env.ok(
        "storyboard.add_panel",
        json!({ "storyboardId": storyboard, "visual": "placeholder", "description": "Arjun slides the folder across" }),
    );
    let vs = env.ok("visual.scenes", json!({}));
    let row = arr(&vs["scenes"])
        .iter()
        .find(|s| s["lineageId"] == lineage_station.as_str())
        .unwrap();
    assert_eq!(row["shotCount"], 1);
    assert_eq!(row["storyboardCount"], 1);

    // --- Schedule: every scene starts Unscheduled
    let schedule = id(&env.ok("schedule.create", json!({})));
    let sv = env.ok("schedule.get", json!({}));
    assert_eq!(arr(&sv["unscheduled"]).len(), 2);
    assert!(arr(&sv["days"]).is_empty());
    let day = id(&env.ok(
        "schedule.create_day",
        json!({ "scheduleId": schedule, "date": "2027-06-14" }),
    ));
    let strip = arr(&sv["unscheduled"])
        .iter()
        .find(|s| s["sceneId"] == scene_station.as_str())
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    env.ok(
        "schedule.move_strip",
        json!({ "stripId": strip, "dayId": day, "index": 0 }),
    );
    let sv = env.ok("schedule.get", json!({}));
    assert_eq!(arr(&sv["unscheduled"]).len(), 1);
    assert_eq!(
        sv["days"][0]["items"][0]["strip"]["sceneId"],
        scene_station.as_str()
    );

    // --- Call sheet for the day → finalize
    let call_sheet = id(&env.ok("callsheets.create", json!({ "dayId": day })));
    let cs = env.ok("callsheets.get", json!({ "id": call_sheet }));
    let doc_scenes = arr(&cs["document"]["scenes"]);
    assert_eq!(doc_scenes.len(), 1);
    assert_eq!(doc_scenes[0]["heading"], "INT. POLICE STATION — NIGHT");
    assert!(
        arr(&cs["document"]["cast"])
            .iter()
            .any(|c| c["character"] == "ARJUN"),
        "call sheet cast from the scene: {}",
        cs["document"]["cast"]
    );
    let mut doc = cs["document"].clone();
    doc["crewCall"] = json!("18:00");
    env.ok(
        "callsheets.update",
        json!({ "id": call_sheet, "document": doc }),
    );
    env.ok("callsheets.finalize", json!({ "id": call_sheet }));
    let cs = env.ok("callsheets.get", json!({ "id": call_sheet }));
    assert_eq!(cs["status"], "Final");
    assert_eq!(cs["editable"], false);

    Film {
        vault_item: item,
        act,
        seq,
        card_station,
        card_bus,
        screenplay,
        draft,
        scene_station,
        scene_bus,
        lineage_station,
        lineage_bus,
        pistol,
        location,
        cast,
        shot,
        storyboard,
        schedule,
        day,
        call_sheet,
    }
}

// ------------------------------------------------------------------- flows

#[test]
fn e2e_idea_to_call_sheet() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let f = build_film(&env);
    // The Story card remembers the scene it became; the scene its card.
    assert_eq!(
        env.ok("story.card", json!({ "id": f.card_station }))["card"]["screenplaySceneId"],
        f.scene_station.as_str()
    );
    let hub = env.ok(
        "screenplay.scene_hub",
        json!({ "sceneId": f.scene_station }),
    );
    let story_row = arr(&hub["rows"])
        .iter()
        .find(|r| r["key"] == "story")
        .unwrap();
    assert_eq!(story_row["nav"]["cardId"], f.card_station.as_str());
    let _ = (
        &f.act,
        &f.seq,
        &f.vault_item,
        &f.card_bus,
        &f.screenplay,
        &f.scene_bus,
        &f.lineage_bus,
    );
    let _ = (
        &f.location,
        &f.shot,
        &f.day,
        &f.schedule,
        &f.storyboard,
        &f.pistol,
        &f.cast,
    );
}

#[test]
fn e2e_script_revision_after_production_source_flags_everything_and_deletes_nothing() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let f = build_film(&env);
    let elements_before = count(
        &env,
        "SELECT count(*) FROM breakdown_element WHERE deleted_at IS NULL",
    );
    let shots_before = count(&env, "SELECT count(*) FROM shot WHERE deleted_at IS NULL");

    // Lock the shooting draft and start a revision (the normal way scripts change
    // once production has begun).
    env.ok("screenplay.lock_draft", json!({ "draftId": f.draft }));
    let rev = env.ok(
        "screenplay.start_revision",
        json!({ "draftId": f.draft, "label": "Revision A", "color": "blue", "reason": "Producer notes" }),
    );
    let rdraft = rev["id"].as_str().unwrap().to_string();
    let rs = scenes(&env, &rdraft);
    assert_eq!(rs.len(), 2);
    assert_eq!(
        rs[0]["lineageId"],
        f.lineage_station.as_str(),
        "revision keeps scene identity"
    );
    let station_action = arr(&rs[0]["elements"])
        .iter()
        .find(|e| e["elementType"] == "action")
        .unwrap()["id"]
        .clone();
    // Change the station scene, remove the bus stop scene, add a new scene.
    env.ok(
        "screenplay.apply_edits",
        json!({ "draftId": rdraft, "ops": [
            { "op": "updateElement", "id": station_action, "text": "Arjun enters carrying a shotgun." },
            { "op": "deleteScene", "id": rs[1]["id"] },
            { "op": "insertScene", "index": 1, "heading": "EXT. RAILWAY PLATFORM — DAWN" },
        ] }),
    );

    // Production notices a newer draft but nothing changes until applied.
    let src = env.ok("production.source", json!({}));
    assert_eq!(src["draft"]["id"], f.draft.as_str());
    let pu = env.ok("production.preview_update", json!({ "draftId": rdraft }));
    assert_eq!(pu["textChanged"], 1, "{pu}");
    assert_eq!(pu["removed"], 1, "{pu}");
    assert_eq!(pu["added"], 1, "{pu}");
    env.ok("production.apply_update", json!({ "draftId": rdraft }));
    assert_eq!(
        env.ok("production.source", json!({}))["draft"]["id"],
        rdraft.as_str()
    );

    // Breakdown: changed scene needs review; removed scene is historical; new scene needs breakdown.
    let bs = env.ok("breakdown.scenes", json!({}));
    let rows = arr(&bs["scenes"]);
    let station = rows
        .iter()
        .find(|r| r["lineageId"] == f.lineage_station.as_str())
        .expect("station scene");
    assert_eq!(station["needsReview"], true, "{station}");
    assert!(
        rows.iter()
            .any(|r| r["needsBreakdown"] == true && r["heading"] == "EXT. RAILWAY PLATFORM — DAWN")
    );
    assert!(
        rows.iter()
            .all(|r| r["lineageId"] != f.lineage_bus.as_str())
    );
    // Nothing is deleted: breakdown elements, shots, catalog, locations and cast survive.
    assert_eq!(
        count(
            &env,
            "SELECT count(*) FROM breakdown_element WHERE deleted_at IS NULL"
        ),
        elements_before
    );
    assert_eq!(
        count(&env, "SELECT count(*) FROM shot WHERE deleted_at IS NULL"),
        shots_before
    );
    assert!(env.ok("catalog.get", json!({ "id": f.pistol }))["name"].is_string());
    assert!(env.ok("locations.get", json!({ "id": f.location }))["name"].is_string());

    // Shots / storyboards on the changed scene need review.
    assert_eq!(
        env.ok("shot.get", json!({ "id": f.shot }))["needsReview"],
        true
    );
    assert_eq!(
        env.ok("storyboard.get", json!({ "id": f.storyboard }))["board"]["needsReview"],
        true
    );
    let vs = env.ok("visual.scenes", json!({}));
    assert!(
        arr(&vs["scenes"])
            .iter()
            .any(|s| s["lineageId"] == f.lineage_station.as_str() && s["needsReview"] == true)
    );

    // Schedule: changes are shown, then reconciled into strip states — the
    // removed scene's strip stays (flagged) until the user confirms.
    let sv = env.ok("schedule.get", json!({}));
    assert!(sv["scriptChanges"].is_object(), "{}", sv["scriptChanges"]);
    let r = env.ok("schedule.reconcile", json!({ "scheduleId": f.schedule }));
    assert_eq!(r["changed"], 1, "{r}");
    assert_eq!(r["removed"], 1, "{r}");
    assert_eq!(r["added"], 1, "{r}");
    let sv = env.ok("schedule.get", json!({}));
    let all: Vec<Value> = arr(&sv["unscheduled"])
        .iter()
        .cloned()
        .chain(arr(&sv["days"]).iter().flat_map(|d| {
            arr(&d["items"])
                .iter()
                .map(|i| i["strip"].clone())
                .collect::<Vec<_>>()
        }))
        .collect();
    let state = |lineage: &str| {
        all.iter()
            .find(|s| s["lineageId"] == lineage)
            .map(|s| s["sourceState"].clone())
    };
    assert_eq!(state(&f.lineage_station), Some(json!("Changed")));
    assert_eq!(state(&f.lineage_bus), Some(json!("Removed")));
    assert!(all.iter().any(|s| s["sourceState"] == "New"));

    // The finalized call sheet is never rewritten; it only says its source changed.
    let cs = env.ok("callsheets.get", json!({ "id": f.call_sheet }));
    assert_eq!(cs["status"], "Final");
    assert_eq!(
        cs["document"]["scenes"][0]["heading"],
        "INT. POLICE STATION — NIGHT"
    );
    assert_eq!(cs["sourceChanged"], true, "{cs}");

    // The locked draft's text never changed.
    let old = scenes(&env, &f.draft);
    assert_eq!(old.len(), 2);
}

/// Scripted stand-in for the local model: returns the tool call it is told to.
#[derive(Default)]
struct ScriptedModel {
    replies: Mutex<VecDeque<String>>,
}

impl ScriptedModel {
    fn call(&self, tool: &str, args: Value) {
        self.replies
            .lock()
            .unwrap()
            .push_back(json!({ "tool": tool, "arguments": args }).to_string());
    }
}

impl ChatModel for ScriptedModel {
    fn chat(&self, _req: &ChatRequest) -> AppResult<String> {
        self.replies
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| AppError::ai("unavailable", "AI is currently unavailable."))
    }
    fn model_reference(&self) -> String {
        "test-local-profile".into()
    }
}

#[test]
fn e2e_import_fountain_and_fdx_then_breakdown_and_ai_counts() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let fountain = env.write_file(
        "Black Rain.fountain",
        include_bytes!("../../openframe-import-export/tests/fixtures/screenplay/sample.fountain"),
    );
    let p = env.ok("screenplay.import_preview", json!({ "path": fountain }));
    assert_eq!(p["sourceFormat"], "fountain");
    assert_eq!(p["defaultMode"], "new_screenplay");
    let r = env.ok(
        "screenplay.import_apply",
        json!({ "previewId": p["previewId"], "mode": "new_screenplay", "keepSourceFile": true }),
    );
    let draft = r["draftId"].as_str().unwrap().to_string();
    let sc = scenes(&env, &draft);
    assert_eq!(sc.len() as i64, r["importedScenes"].as_i64().unwrap());
    assert!(
        sc.iter()
            .any(|s| s["heading"] == "INT. POLICE STATION — NIGHT")
    );
    // Screenplay workspace opens the imported screenplay.
    assert_eq!(
        env.ok("screenplay.overview", json!({}))["screenplay"]["id"],
        r["screenplayId"]
    );

    // Production works from it: breakdown suggests the pistol in the station scene.
    env.ok("production.set_source", json!({ "draftId": draft }));
    let station = sc
        .iter()
        .find(|s| s["heading"] == "INT. POLICE STATION — NIGHT")
        .unwrap()["id"]
        .clone();
    env.ok("breakdown.suggest", json!({ "sceneId": station }));
    let d = env.ok("breakdown.scene", json!({ "sceneId": station }));
    let sug: Vec<Value> = arr(&d["groups"])
        .iter()
        .flat_map(|g| arr(&g["suggestions"]).clone())
        .collect();
    let pistol = sug
        .iter()
        .find(|s| {
            s["name"]
                .as_str()
                .unwrap_or("")
                .to_lowercase()
                .contains("pistol")
        })
        .unwrap_or_else(|| panic!("pistol suggested: {sug:?}"));
    let acc = env.ok("breakdown.accept", json!({ "id": pistol["id"] }));
    assert!(acc["catalogItemId"].is_string());
    // Breakdown scene script text starts with the scene body, not a duplicated heading.
    let script = arr(&d["script"]);
    assert!(!script.is_empty());

    // FDX as a new draft of the same screenplay (never overwrites).
    let fdx = env.write_file(
        "Black Rain.fdx",
        include_bytes!("../../openframe-import-export/tests/fixtures/screenplay/sample.fdx"),
    );
    let p2 = env.ok("screenplay.import_preview", json!({ "path": fdx }));
    assert_eq!(p2["sourceFormat"], "fdx");
    assert_eq!(p2["defaultMode"], "new_draft");
    let r2 = env.ok(
        "screenplay.import_apply",
        json!({ "previewId": p2["previewId"], "mode": "new_draft" }),
    );
    assert_eq!(r2["screenplayId"], r["screenplayId"]);
    assert_eq!(
        env.ok("screenplay.overview", json!({}))["screenplay"]["currentDraftId"],
        draft.as_str(),
        "the current draft is unchanged"
    );
    let pu = env.ok(
        "production.preview_update",
        json!({ "draftId": r2["draftId"] }),
    );
    assert!(pu["unchanged"].is_number());

    // Offline AI: deterministic count tools answer from the project data.
    let model = Arc::new(ScriptedModel::default());
    ai::service(&env.core).attach_model(Some(model.clone() as Arc<dyn ChatModel>));
    model.call("count_scenes", json!({}));
    let a = env.ok(
        "ai.ask",
        json!({ "text": "How many scenes are in the script?", "scope": { "kind": "CurrentScreenplay" } }),
    );
    assert_eq!(a["result"]["kind"], "Answer", "{a}");
    assert_eq!(a["result"]["confidence"], "Exact");
    let content = a["result"]["content"].as_str().unwrap();
    assert!(content.contains(&sc.len().to_string()), "{content}");
    model.call("count_characters", json!({}));
    let a = env.ok(
        "ai.ask",
        json!({ "text": "How many characters speak?", "scope": { "kind": "CurrentScreenplay" } }),
    );
    assert_eq!(a["result"]["kind"], "Answer", "{a}");
    // Unscheduled scenes: every scene is unscheduled once a schedule exists.
    env.ok("schedule.create", json!({}));
    model.call("unscheduled_scenes", json!({}));
    let a = env.ok(
        "ai.ask",
        json!({ "text": "Which scenes are not scheduled?", "scope": { "kind": "WholeProject" } }),
    );
    assert_eq!(a["result"]["kind"], "Answer", "{a}");
    let items = arr(&a["result"]["items"]);
    assert_eq!(items.len(), sc.len(), "{a}");
    // AI never mutated anything.
    assert_eq!(count(&env, "SELECT count(*) FROM screenplay_draft"), 2);
}

fn exists(env: &TestEnv, table: &str, id: &str) -> bool {
    count(
        env,
        &format!("SELECT count(*) FROM {table} WHERE id='{id}' AND deleted_at IS NULL"),
    ) == 1
}

fn element_text(env: &TestEnv, draft: &str, scene: &str, element_type: &str) -> Vec<String> {
    scenes(env, draft)
        .iter()
        .filter(|s| s["id"] == scene)
        .flat_map(|s| arr(&s["elements"]).clone())
        .filter(|e| e["elementType"] == element_type)
        .map(|e| e["text"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn e2e_undo_redo_across_modules() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let f = build_film(&env);
    let sv = env.ok("schedule.get", json!({}));
    let bus_strip = arr(&sv["unscheduled"])[0]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // One edit in each of five modules …
    let note = id(&env.ok(
        "vault.create",
        json!({ "store": "project", "itemType": "note", "title": "Umbrella motif", "body": "Red umbrella." }),
    ));
    let card = id(&env.ok(
        "story.create_card",
        json!({ "parent": { "parentType": "sequence", "parentId": f.seq }, "shortDescription": "Epilogue" }),
    ));
    let dialogue = scenes(&env, &f.draft)[0]["elements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["elementType"] == "dialogue")
        .unwrap()["id"]
        .clone();
    env.ok(
        "screenplay.update_element",
        json!({ "id": dialogue, "text": "I was never there." }),
    );
    let prop = id(&env.ok(
        "catalog.create",
        json!({ "category": "Props", "name": "Red umbrella" }),
    ));
    env.ok(
        "schedule.move_strip",
        json!({ "stripId": bus_strip, "dayId": f.day, "index": 1 }),
    );
    let scheduled = |env: &TestEnv| {
        env.ok("schedule.get", json!({}))["unscheduled"]
            .as_array()
            .unwrap()
            .is_empty()
    };
    assert!(scheduled(&env));

    // … is undone newest-first, one module per step …
    env.undo();
    assert!(!scheduled(&env), "schedule move undone");
    env.undo();
    assert!(!exists(&env, "catalog_item", &prop), "catalog item undone");
    env.undo();
    assert_eq!(
        element_text(&env, &f.draft, &f.scene_station, "dialogue"),
        vec!["I was there that night."]
    );
    env.undo();
    assert!(!exists(&env, "story_scene_card", &card), "card undone");
    env.undo();
    assert!(!exists(&env, "vault_item", &note), "vault note undone");
    // Earlier work (the whole film) is untouched by those undos.
    assert!(exists(&env, "catalog_item", &f.pistol));
    assert_eq!(
        env.ok("callsheets.get", json!({ "id": f.call_sheet }))["status"],
        "Final"
    );

    // … and redone in order.
    for _ in 0..5 {
        env.redo();
    }
    assert!(exists(&env, "vault_item", &note));
    assert!(exists(&env, "story_scene_card", &card));
    assert_eq!(
        element_text(&env, &f.draft, &f.scene_station, "dialogue"),
        vec!["I was never there."]
    );
    assert!(exists(&env, "catalog_item", &prop));
    assert!(scheduled(&env));
    // Redone objects keep their identity and are searchable again.
    assert!(
        search_types(&env, "Epilogue")
            .iter()
            .any(|(t, _)| t == "story_scene_card")
    );
}

#[test]
fn e2e_everything_persists_across_restart() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let f = build_film(&env);
    let path = env.project_path();
    let env = env.restart();
    // App Home → open the project again (what the user does after relaunching).
    assert!(
        env.call("screenplay.overview", json!({})).is_err(),
        "no project is open after a relaunch"
    );
    env.reopen_project(&path);
    assert_eq!(
        element_text(&env, &f.draft, &f.scene_station, "dialogue"),
        vec!["I was there that night."]
    );
    assert_eq!(
        env.ok("screenplay.overview", json!({}))["screenplay"]["currentDraftId"],
        f.draft.as_str()
    );
    assert_eq!(
        env.ok("production.source", json!({}))["draft"]["id"],
        f.draft.as_str()
    );
    assert!(exists(&env, "vault_item", &f.vault_item));
    let board = env.ok("story.board", json!({}));
    assert_eq!(board["cardCount"], 2);
    assert!(exists(&env, "catalog_item", &f.pistol));
    assert!(exists(&env, "location", &f.location));
    assert!(exists(&env, "cast_member", &f.cast));
    assert_eq!(
        env.ok("shot.get", json!({ "id": f.shot }))["needsReview"],
        false
    );
    let sv = env.ok("schedule.get", json!({}));
    assert_eq!(
        sv["days"][0]["items"][0]["strip"]["sceneId"],
        f.scene_station.as_str()
    );
    assert_eq!(
        env.ok("callsheets.get", json!({ "id": f.call_sheet }))["status"],
        "Final"
    );
    // Search index survives too.
    assert!(
        search_types(&env, "pistol")
            .iter()
            .any(|(t, _)| t == "screenplay_scene")
    );
    // Undo history is per session: nothing to undo after a restart must not break anything.
    let _ = env.call("history.undo", json!({}));
    let _ = (
        &f.act,
        &f.card_bus,
        &f.card_station,
        &f.screenplay,
        &f.scene_bus,
        &f.lineage_bus,
        &f.lineage_station,
        &f.storyboard,
        &f.schedule,
    );
}

#[test]
fn e2e_recently_deleted_across_modules() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let f = build_film(&env);
    // Delete one object in each module …
    env.ok(
        "vault.delete",
        json!({ "store": "project", "ids": [f.vault_item] }),
    );
    env.ok(
        "story.delete_items",
        json!({ "items": [{ "kind": "card", "id": f.card_bus }] }),
    );
    let prop = id(&env.ok(
        "catalog.create",
        json!({ "category": "Props", "name": "Umbrella" }),
    ));
    env.ok("catalog.delete", json!({ "id": prop }));
    let loc = id(&env.ok("locations.create", json!({ "name": "Railway Platform" })));
    env.ok("locations.delete", json!({ "id": loc }));
    env.ok("shot.delete", json!({ "ids": [f.shot] }));
    env.ok("storyboard.delete", json!({ "id": f.storyboard }));
    let note = id(&env.ok(
        "notes.create",
        json!({ "title": "Location scout", "body": "Check the station roof." }),
    ));
    env.ok("notes.delete", json!({ "id": note }));

    // … and they all appear in Recently Deleted …
    let trash = env.ok("trash.list", json!({}));
    let types: Vec<&str> = arr(&trash)
        .iter()
        .map(|t| t["objectType"].as_str().unwrap())
        .collect();
    for t in [
        "vault_item",
        "story_scene_card",
        "catalog_item",
        "location",
        "shot",
        "storyboard",
        "project_note",
    ] {
        assert!(types.contains(&t), "{t} in Recently Deleted: {types:?}");
    }
    // The Idea Vault's own trash view lists the same deleted idea.
    let vault_trash = env.ok("vault.trash_list", json!({ "store": "project" }));
    assert_eq!(arr(&vault_trash).len(), 1);
    assert_eq!(vault_trash[0]["deleted"]["objectId"], f.vault_item.as_str());
    // Deleted objects are not searchable.
    assert!(
        !search_types(&env, "Railway Platform")
            .iter()
            .any(|(t, _)| t == "location")
    );

    // … and come back where they were.
    for t in arr(&trash).clone() {
        env.ok("trash.restore", json!({ "id": t["id"] }));
    }
    assert!(exists(&env, "vault_item", &f.vault_item));
    assert!(exists(&env, "story_scene_card", &f.card_bus));
    let board = env.ok("story.board", json!({}));
    let seq_cards: Vec<String> = arr(&board["acts"][0]["items"])
        .iter()
        .filter(|i| i["kind"] == "sequence")
        .flat_map(|s| {
            arr(&s["items"])
                .iter()
                .map(|c| c["id"].as_str().unwrap().to_string())
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(
        seq_cards.contains(&f.card_bus),
        "card back in its sequence: {seq_cards:?}"
    );
    assert!(exists(&env, "catalog_item", &prop));
    assert!(exists(&env, "location", &loc));
    assert!(exists(&env, "shot", &f.shot));
    assert!(exists(&env, "storyboard", &f.storyboard));
    assert!(exists(&env, "project_note", &note));
    assert!(
        search_types(&env, "Railway Platform")
            .iter()
            .any(|(t, _)| t == "location")
    );
    assert!(arr(&env.ok("trash.list", json!({}))).is_empty());
}

/// The search result's `nav` for an entity (first hit of that type for `text`).
fn nav_of(env: &TestEnv, text: &str, entity_type: &str) -> Value {
    search_types(env, text)
        .into_iter()
        .find(|(t, _)| t == entity_type)
        .unwrap_or_else(|| panic!("no {entity_type} hit for {text:?}"))
        .1
}

/// Assert a nav target is exactly what the destination workspace reads from
/// `useNav().route` (workspace, `sub`, focus parameter).
fn assert_nav(nav: &Value, workspace: &str, sub: Option<&str>, param: &str, value: &str) {
    assert_eq!(nav["workspace"], workspace, "{nav}");
    match sub {
        Some(s) => assert_eq!(nav["sub"], s, "{nav}"),
        None => assert!(nav.get("sub").is_none() || nav["sub"].is_null(), "{nav}"),
    }
    assert_eq!(nav[param], value, "{nav}");
}

#[test]
fn e2e_search_hits_navigate_to_the_workspace_that_shows_them() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let f = build_film(&env);
    // Frontend contract (apps/desktop/src/workspaces/**): which param each page reads.
    assert_nav(
        &nav_of(&env, "rainy bus stop", "vault_item"),
        "vault",
        None,
        "itemId",
        &f.vault_item,
    );
    assert_nav(
        &nav_of(&env, "Act 1", "story_act"),
        "story",
        None,
        "actId",
        &f.act,
    );
    assert_nav(
        &nav_of(&env, "The return", "story_sequence"),
        "story",
        None,
        "sequenceId",
        &f.seq,
    );
    assert_nav(
        &nav_of(&env, "confesses", "story_scene_card"),
        "story",
        None,
        "cardId",
        &f.card_station,
    );
    let ravi = id(&env.ok("story.create_character", json!({ "name": "Ravi" })));
    assert_nav(
        &nav_of(&env, "Ravi", "story_character"),
        "story",
        Some("characters"),
        "characterId",
        &ravi,
    );
    let scene = nav_of(&env, "pistol", "screenplay_scene");
    assert_nav(&scene, "screenplay", None, "sceneId", &f.scene_station);
    assert_eq!(scene["draftId"], f.draft.as_str());
    assert_nav(
        &nav_of(&env, "pistol", "catalog_item"),
        "production",
        Some("catalog"),
        "catalogItemId",
        &f.pistol,
    );
    assert_nav(
        &nav_of(&env, "Market Rd", "location"),
        "production",
        Some("locations"),
        "locationId",
        &f.location,
    );
    assert_nav(
        &nav_of(&env, "Karthik", "cast_member"),
        "production",
        Some("cast-crew"),
        "castMemberId",
        &f.cast,
    );
    let crew = id(&env.ok("crew.create", json!({ "personName": "Anu Thomas", "role": "Director of Photography", "department": "Camera" })));
    assert_nav(
        &nav_of(&env, "Anu Thomas", "crew_member"),
        "production",
        Some("cast-crew"),
        "crewMemberId",
        &crew,
    );
    assert_nav(
        &nav_of(&env, "counter", "shot"),
        "production",
        Some("shots"),
        "shotId",
        &f.shot,
    );
    let panel = nav_of(&env, "slides the folder", "storyboard_panel");
    assert_nav(
        &panel,
        "production",
        Some("storyboards"),
        "storyboardId",
        &f.storyboard,
    );
    assert!(panel["panelId"].is_string());
    let cs = nav_of(&env, "POLICE STATION", "call_sheet");
    assert_nav(&cs, "callsheets", None, "callSheetId", &f.call_sheet);
    let note = id(&env.ok(
        "notes.create",
        json!({ "title": "Scout the roof", "body": "Check the station roof." }),
    ));
    assert_nav(
        &nav_of(&env, "station roof", "project_note"),
        "notes",
        None,
        "noteId",
        &note,
    );

    // Comments open where their target lives.
    let el = scenes(&env, &f.draft)[0]["elements"][0]["id"].clone();
    env.ok(
        "comment.create",
        json!({ "targetType": "screenplay_element", "targetId": el, "body": "Tighten this line" }),
    );
    let c = nav_of(&env, "Tighten", "comment");
    assert_nav(&c, "screenplay", None, "sceneId", &f.scene_station);
    assert_eq!(c["draftId"], f.draft.as_str());
    assert!(c["commentId"].is_string());
    env.ok("comment.create", json!({ "targetType": "story_scene_card", "targetId": f.card_bus, "body": "Umbrella colour?" }));
    assert_nav(
        &nav_of(&env, "Umbrella colour", "comment"),
        "story",
        None,
        "cardId",
        &f.card_bus,
    );
    env.ok(
        "comment.create",
        json!({ "targetType": "shot", "targetId": f.shot, "body": "Use the 35mm" }),
    );
    assert_nav(
        &nav_of(&env, "35mm", "comment"),
        "production",
        Some("shots"),
        "shotId",
        &f.shot,
    );
}

#[test]
fn e2e_series_scene_links_carry_their_episode() {
    let env = TestEnv::with_project("Monsoon", "Series");
    let ep = id(&env.ok("story.create_episode", json!({ "title": "Pilot" })));
    let act = id(&env.ok(
        "story.create_act",
        json!({ "title": "Cold open", "episodeId": ep }),
    ));
    let card = id(&env.ok(
        "story.create_card",
        json!({ "parent": { "parentType": "act", "parentId": act }, "shortDescription": "Flooded street" }),
    ));
    env.ok(
        "story.update_card",
        json!({ "id": card, "sceneHeading": "EXT. FLOODED STREET — NIGHT" }),
    );
    let b = env.ok(
        "story.build_screenplay",
        json!({ "episodeId": ep, "include": [{ "cardId": card }], "destination": "newScreenplay" }),
    );
    // The episode's screenplay opens in the Screenplay workspace for that episode.
    let ov = env.ok("screenplay.overview", json!({ "episodeId": ep }));
    assert_eq!(ov["screenplay"]["id"], b["screenplayId"]);
    // Search / Continue / AI links to its scenes name the episode.
    let nav = nav_of(&env, "FLOODED STREET", "screenplay_scene");
    assert_eq!(nav["episodeId"], ep.as_str(), "{nav}");
    assert_eq!(nav["draftId"], b["draftId"]);
    let card_nav = nav_of(&env, "Flooded street", "story_scene_card");
    assert_eq!(card_nav["episodeId"], ep.as_str(), "{card_nav}");
    // Links that only carry a scene id (schedule strips, characters…) are
    // resolved by the Screenplay workspace through screenplay.locate.
    let scene = nav["sceneId"].as_str().unwrap();
    let loc = env.ok("screenplay.locate", json!({ "sceneId": scene }));
    assert_eq!(loc["episodeId"], ep.as_str());
    assert_eq!(loc["draftId"], b["draftId"]);
    assert_eq!(loc["screenplayId"], b["screenplayId"]);
    let loc = env.ok("screenplay.locate", json!({ "draftId": b["draftId"] }));
    assert_eq!(loc["episodeId"], ep.as_str());
    assert!(loc["sceneId"].is_null());
    assert_eq!(
        env.err("screenplay.locate", json!({})),
        "validation.required"
    );
}
