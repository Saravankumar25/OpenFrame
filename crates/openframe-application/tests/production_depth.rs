//! Breakdown, catalog, locations, cast and crew depth (masterplan area 13; Production depth
//! contract sections 2, 3 and 9; depth spec sections 23, 24 and 53).
//!
//! Category identity and Advanced customisation, bulk actions with one undo step per batch,
//! server-side filters, quick reuse, jump to source phrase, "catalog item to all its scenes",
//! the short breakdown report, script changes that never delete confirmed data, location and
//! people fields, availability windows, contact privacy by role and for the assistant, stable
//! identity and explicit references, search coverage and the migration from schema 13.

use openframe_application::MutationMeta;
use openframe_application::modules::ai::scope::{self, AiScopeArgs, AiScopeKind};
use openframe_application::modules::ai::toolbox::{self, ToolStep};
use openframe_domain::{Capability, Role, new_id, now_ms};
use openframe_test_support::TestEnv;
use rusqlite::params;
use serde_json::{Value, json};

// ---------------------------------------------------------------- fixtures

fn fixture<R>(env: &TestEnv, f: impl FnOnce(&rusqlite::Connection) -> R) -> R {
    let s = env.core.project().unwrap();
    s.store
        .mutate(
            &env.actor(),
            MutationMeta::new("test.fixture", "Screenplay fixture", Capability::Edit).quiet(),
            |tx| Ok(f(tx.conn())),
        )
        .unwrap()
}

type SceneSpec<'a> = (&'a str, &'a str, &'a [(&'a str, &'a str)]);

/// A screenplay draft with scenes (lineage, heading, elements); returns (draft, scene ids).
fn draft(
    env: &TestEnv,
    sp: &str,
    name: &str,
    order: i64,
    scenes: &[SceneSpec<'_>],
) -> (String, Vec<String>) {
    let draft = new_id();
    let mut ids = Vec::new();
    fixture(env, |c| {
        let t = 1_700_000_000_000 + order * 1000;
        c.execute(
            "INSERT INTO screenplay_draft(id, screenplay_id, name, status, created_at, updated_at) VALUES (?1, ?2, ?3, 'Draft', ?4, ?4)",
            params![draft, sp, name, t],
        )
        .unwrap();
        for (i, (lineage, heading, els)) in scenes.iter().enumerate() {
            let sid = new_id();
            c.execute(
                "INSERT INTO screenplay_scene(id, draft_id, lineage_id, position, heading, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                params![sid, draft, lineage, (i + 1) as i64, heading, t],
            )
            .unwrap();
            for (j, (ty, text)) in els.iter().enumerate() {
                c.execute(
                    "INSERT INTO screenplay_element(id, scene_id, position, element_type, text, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                    params![new_id(), sid, (j + 1) as i64, ty, text, t],
                )
                .unwrap();
            }
            ids.push(sid);
        }
    });
    (draft, ids)
}

const S1: &[(&str, &str)] = &[
    (
        "action",
        "Arjun enters carrying a pistol. Rain drips from his coat.",
    ),
    ("action", "He places the RED FOLDER on the desk."),
    ("character", "MEERA"),
    ("dialogue", "You said you would never come back."),
];
const S2: &[(&str, &str)] = &[
    ("action", "Arjun checks the pistol under a flickering lamp."),
    ("character", "ARJUN"),
    ("dialogue", "Not tonight."),
];
const S3: &[(&str, &str)] = &[(
    "action",
    "రవి కౌంటర్ దగ్గర ఎదురు చూస్తాడు. A crowd of passengers pushes past.",
)];
const S4: &[(&str, &str)] = &[("action", "An empty platform. रात की आखिरी ट्रेन।")];

/// Project with four scenes in "Draft 6" chosen as the Production Source.
fn setup() -> (TestEnv, String, Vec<String>) {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let sp = new_id();
    fixture(&env, |c| {
        c.execute(
            "INSERT INTO screenplay(id, title, created_at, updated_at) VALUES (?1, 'Black Rain', ?2, ?2)",
            params![sp, now_ms()],
        )
        .unwrap();
    });
    let (d, scenes) = draft(
        &env,
        &sp,
        "Draft 6",
        6,
        &[
            ("L1", "INT. POLICE STATION - NIGHT", S1),
            ("L2", "EXT. OLD RAILWAY STATION - NIGHT", S2),
            ("L3", "INT. TICKET OFFICE - DAY", S3),
            ("L4", "EXT. PLATFORM - NIGHT", S4),
        ],
    );
    env.ok("production.set_source", json!({ "draftId": d }));
    (env, sp, scenes)
}

fn add(env: &TestEnv, scene: &str, category: &str, name: &str) -> Value {
    env.ok(
        "breakdown.add_element",
        json!({ "sceneId": scene, "category": category, "name": name }),
    )
}

/// Input errors name the field (`validation.color`, `validation.startDate`) or are generic.
fn assert_validation(code: String) {
    assert!(
        code.starts_with("validation."),
        "expected a validation error, got {code}"
    );
}

fn count(env: &TestEnv, sql: &str) -> i64 {
    let s = env.core.project().unwrap();
    s.store
        .read(|c| Ok(c.query_row(sql, [], |r| r.get(0))?))
        .unwrap()
}

fn element_id(env: &TestEnv, scene_id: &str, position: i64) -> String {
    let s = env.core.project().unwrap();
    s.store
        .read(|c| {
            Ok(c.query_row(
                "SELECT id FROM screenplay_element WHERE scene_id=?1 AND position=?2",
                params![scene_id, position],
                |r| r.get(0),
            )?)
        })
        .unwrap()
}

fn edit_line(env: &TestEnv, scene_id: &str, position: i64, text: &str) {
    fixture(env, |c| {
        c.execute(
            "UPDATE screenplay_element SET text=?1, updated_at=?2, rev=rev+1 WHERE scene_id=?3 AND position=?4",
            params![text, now_ms(), scene_id, position],
        )
        .unwrap();
    });
}

fn numbers(view: &Value) -> Vec<String> {
    view["scenes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["number"].as_str().unwrap().to_string())
        .collect()
}

fn suggestion_ids(env: &TestEnv, scene: &str) -> Vec<String> {
    let d = env.ok("breakdown.scene", json!({ "sceneId": scene }));
    d["groups"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|g| g["suggestions"].as_array().unwrap().clone())
        .map(|s| s["id"].as_str().unwrap().to_string())
        .collect()
}

fn search_ids(env: &TestEnv, text: &str) -> Vec<String> {
    env.ok("search.query", json!({ "text": text }))
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h["entityId"].as_str().unwrap_or_default().to_string())
        .collect()
}

// ------------------------------------------------- category identity

#[test]
fn eleven_categories_have_colour_text_icon_and_tooltip_with_advanced_customisation() {
    let (env, _, _) = setup();
    let v = env.ok("breakdown.categories", json!({}));
    let cats = v["categories"].as_array().unwrap();
    assert_eq!(cats.len(), 11);
    assert_eq!(cats[0]["category"], "Cast");
    assert_eq!(cats[10]["category"], "Animals");
    for c in cats {
        assert_eq!(c["label"], c["category"], "text label is the category name");
        assert!(
            c["tooltip"]
                .as_str()
                .unwrap()
                .starts_with(c["label"].as_str().unwrap())
        );
        assert!(c["colorHex"].as_str().unwrap().starts_with('#'));
        assert!(!c["icon"].as_str().unwrap().is_empty());
        assert_eq!(c["customized"], false);
    }
    assert_eq!(v["palette"].as_array().unwrap().len(), 12);

    // Advanced: change the colour and icon of Props.
    let v = env.ok(
        "breakdown.set_category_style",
        json!({ "category": "Props", "color": "red", "icon": "star" }),
    );
    let props = v["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["category"] == "Props")
        .unwrap()
        .clone();
    assert_eq!(props["color"], "red");
    assert_eq!(props["colorHex"], "#c92a2a");
    assert_eq!(props["icon"], "star");
    assert_eq!(props["customized"], true);
    assert_eq!(props["label"], "Props", "the text never changes");

    // Validation: only palette tokens and known icons; unknown categories fail.
    assert_validation(env.err(
        "breakdown.set_category_style",
        json!({ "category": "Props", "color": "#123456" }),
    ));
    assert_validation(env.err(
        "breakdown.set_category_style",
        json!({ "category": "Props", "icon": "<script>" }),
    ));
    assert_validation(env.err(
        "breakdown.set_category_style",
        json!({ "category": "Weapons", "color": "red" }),
    ));
    // Permission: a Viewer reads the styles but cannot change them.
    let viewer = env.actor_with_role(Role::Viewer);
    assert!(
        env.call_as(&viewer, "breakdown.categories", json!({}))
            .is_ok()
    );
    let e = env
        .call_as(
            &viewer,
            "breakdown.set_category_style",
            json!({ "category": "Cast", "color": "blue" }),
        )
        .unwrap_err();
    assert_eq!(e.code.0, "permission.denied");

    // Undo restores the default; redo brings the custom look back.
    env.undo();
    let v = env.ok("breakdown.categories", json!({}));
    assert_eq!(v["categories"][3]["color"], "violet");
    env.redo();
    // Setting the default values again removes the override row.
    env.ok(
        "breakdown.set_category_style",
        json!({ "category": "Props", "color": "violet", "icon": "box" }),
    );
    assert_eq!(
        count(&env, "SELECT count(*) FROM breakdown_category_style"),
        0
    );
    env.ok(
        "breakdown.set_category_style",
        json!({ "category": "Cast", "color": "slate" }),
    );
    let path = env.project_path();
    let env = env.restart();
    env.reopen_project(&path);
    let v = env.ok("breakdown.categories", json!({}));
    assert_eq!(v["categories"][0]["color"], "slate", "kept after restart");
    let v = env.ok("breakdown.reset_category_styles", json!({}));
    assert!(
        v["categories"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["customized"] == false)
    );
    // An unknown stored value (a newer build, a hand-edited file) falls back to the default.
    fixture(&env, |c| {
        c.execute(
            "INSERT INTO breakdown_category_style(id, color, icon, created_at, updated_at) VALUES ('Sound', 'ultraviolet', 'rocket', 1, 1)",
            [],
        )
        .unwrap();
    });
    let v = env.ok("breakdown.categories", json!({}));
    let sound = v["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["category"] == "Sound")
        .unwrap()
        .clone();
    assert_eq!(sound["color"], "cyan");
    assert_eq!(sound["icon"], "volume");
}

// ------------------------------------------------------- bulk actions

#[test]
fn accept_and_reject_selected_are_one_undo_step_per_batch() {
    let (env, _, scenes) = setup();
    env.ok("breakdown.suggest", json!({ "sceneId": scenes[0] }));
    let ids = suggestion_ids(&env, &scenes[0]);
    assert!(
        ids.len() >= 3,
        "fixture yields several suggestions: {ids:?}"
    );
    let production = "SELECT count(*) FROM breakdown_element WHERE deleted_at IS NULL AND confirmation_state IN ('Confirmed','Manual')";

    let r = env.ok("breakdown.accept_many", json!({ "ids": ids[..2] }));
    assert_eq!(r["accepted"], 2);
    assert_eq!(count(&env, production), 2);
    let step = env.undo();
    assert!(
        step["label"]
            .as_str()
            .unwrap()
            .contains("Accepted 2 suggestions"),
        "{step}"
    );
    assert_eq!(
        count(&env, production),
        0,
        "one undo reverts the whole batch"
    );
    assert_eq!(suggestion_ids(&env, &scenes[0]).len(), ids.len());

    let r = env.ok("breakdown.reject", json!({ "ids": ids }));
    assert_eq!(r["count"], ids.len() as i64);
    assert!(suggestion_ids(&env, &scenes[0]).is_empty());
    env.undo();
    assert_eq!(
        suggestion_ids(&env, &scenes[0]).len(),
        ids.len(),
        "one undo restores every rejected suggestion"
    );
    env.redo();
    // Rejected suggestions are not offered again for this scene and source.
    env.ok("breakdown.suggest", json!({ "sceneId": scenes[0] }));
    assert!(suggestion_ids(&env, &scenes[0]).is_empty());
    assert_eq!(
        count(&env, production),
        0,
        "rejecting creates no production data"
    );
}

#[test]
fn quick_reuse_offers_neighbouring_items_and_reuse_is_one_step_for_many_scenes() {
    let (env, _, scenes) = setup();
    let pistol = add(&env, &scenes[0], "Props", "Pistol");
    add(&env, &scenes[0], "Wardrobe", "Wet grey coat");
    let lamp = add(&env, &scenes[3], "Props", "Lamp");
    // Scene 2 sits next to scene 1: scene 1's items come first; nothing scene 2 has is offered.
    add(&env, &scenes[1], "Props", "Pistol");
    let offer = env.ok("breakdown.quick_reuse", json!({ "sceneId": scenes[1] }));
    let names: Vec<&str> = offer
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["name"].as_str().unwrap())
        .collect();
    assert_eq!(names[0], "Wet grey coat", "neighbour first: {names:?}");
    assert!(names.contains(&"Lamp"));
    assert!(!names.contains(&"Pistol"), "already in the scene");

    let r = env.ok(
        "breakdown.reuse",
        json!({ "sceneIds": [scenes[1], scenes[2]], "catalogItemIds": [pistol["catalogItemId"], lamp["catalogItemId"]] }),
    );
    assert_eq!(r["count"], 3, "scene 2 already had the pistol");
    let item = env.ok("catalog.get", json!({ "id": pistol["catalogItemId"] }));
    assert_eq!(
        item["usedIn"].as_array().unwrap().len(),
        3,
        "the same catalog identity is reused"
    );
    let step = env.undo();
    assert!(
        step["label"]
            .as_str()
            .unwrap()
            .contains("Reused 2 catalog items in 2 scenes")
    );
    let item = env.ok("catalog.get", json!({ "id": pistol["catalogItemId"] }));
    assert_eq!(
        item["usedIn"].as_array().unwrap().len(),
        2,
        "one undo removes the whole batch"
    );

    // Archived items are refused and nothing is half-added.
    env.ok(
        "catalog.set_archived",
        json!({ "id": lamp["catalogItemId"], "archived": true }),
    );
    let before = count(
        &env,
        "SELECT count(*) FROM breakdown_element WHERE deleted_at IS NULL",
    );
    assert_validation(
        env.err(
            "breakdown.reuse",
            json!({ "sceneIds": [scenes[2]], "catalogItemIds": [pistol["catalogItemId"], lamp["catalogItemId"]] })
        ),
    );
    assert_eq!(
        count(
            &env,
            "SELECT count(*) FROM breakdown_element WHERE deleted_at IS NULL"
        ),
        before
    );
    let viewer = env.actor_with_role(Role::Viewer);
    let e = env
        .call_as(
            &viewer,
            "breakdown.reuse",
            json!({ "sceneIds": [scenes[2]], "catalogItemIds": [pistol["catalogItemId"]] }),
        )
        .unwrap_err();
    assert_eq!(e.code.0, "permission.denied");
}

// ------------------------------------------------------------ filters

#[test]
fn filters_unreviewed_suggested_missing_completion_category_and_catalog_item() {
    let (env, _, scenes) = setup();
    let pistol = add(&env, &scenes[0], "Props", "Pistol");
    add(&env, &scenes[0], "Cast", "Arjun");
    env.ok(
        "breakdown.set_complete",
        json!({ "sceneId": scenes[0], "complete": true }),
    );
    env.ok(
        "breakdown.add_element",
        json!({ "sceneId": scenes[1], "category": "Props", "name": "Pistol", "catalog": { "mode": "existing", "catalogItemId": pistol["catalogItemId"] } }),
    );
    env.ok("breakdown.suggest", json!({ "sceneId": scenes[2] }));
    // Scene 2's script changes after planning: Needs Review.
    edit_line(
        &env,
        &scenes[1],
        1,
        "Arjun cleans the pistol by candle light.",
    );

    let all = env.ok("breakdown.scenes", json!({}));
    assert_eq!(all["total"], 4);
    let c = &all["counts"];
    assert_eq!(
        (
            c["all"].as_i64(),
            c["complete"].as_i64(),
            c["incomplete"].as_i64()
        ),
        (Some(4), Some(1), Some(3))
    );
    assert_eq!(c["suggested"], 1);
    assert_eq!(c["needsReview"], 1);
    assert_eq!(c["unreviewed"], 2, "suggestions waiting or script changes");
    assert_eq!(
        c["missing"], 2,
        "scenes 3 and 4 have no production elements"
    );
    assert_eq!(all["scenes"][0]["categories"], json!(["Cast", "Props"]));

    let f = |args: Value| numbers(&env.ok("breakdown.scenes", args));
    assert_eq!(f(json!({ "filter": "unreviewed" })), ["2", "3"]);
    assert_eq!(f(json!({ "filter": "suggested" })), ["3"]);
    assert_eq!(f(json!({ "filter": "missing" })), ["3", "4"]);
    assert_eq!(f(json!({ "filter": "needsReview" })), ["2"]);
    assert_eq!(f(json!({ "filter": "complete" })), ["1"]);
    assert_eq!(f(json!({ "filter": "incomplete" })), ["2", "3", "4"]);
    assert_eq!(f(json!({ "category": "Cast" })), ["1"]);
    assert_eq!(
        f(json!({ "category": "Props", "filter": "incomplete" })),
        ["2"]
    );
    // Catalog item to all its scenes.
    let v = env.ok(
        "breakdown.scenes",
        json!({ "catalogItemId": pistol["catalogItemId"] }),
    );
    assert_eq!(numbers(&v), ["1", "2"]);
    assert_eq!(v["catalogFilter"]["name"], "Pistol");
    assert_eq!(v["total"], 4, "the total stays the whole source");
    assert_eq!(
        env.err("breakdown.scenes", json!({ "catalogItemId": "missing" })),
        "not_found.catalog_item"
    );
    assert_eq!(
        env.err("breakdown.scenes", json!({ "filter": "everything" })),
        "validation.invalid_input"
    );
}

// ------------------------------------------------- jump to source phrase

#[test]
fn source_phrase_is_traceable_after_edits_and_reports_when_it_is_gone() {
    let (env, _, scenes) = setup();
    let line = element_id(&env, &scenes[0], 1);
    // "pistol" = UTF-16 24..30 of "Arjun enters carrying a pistol. …"
    let el = env.ok(
        "breakdown.add_element",
        json!({ "sceneId": scenes[0], "category": "Props", "name": "Pistol", "span": { "elementId": line, "start": 24, "end": 30 } }),
    );
    let a = env.ok("breakdown.source_anchor", json!({ "id": el["id"] }));
    assert_eq!(
        (a["status"].as_str(), a["start"].as_i64(), a["end"].as_i64()),
        (Some("exact"), Some(24), Some(30))
    );
    assert_eq!(a["elementId"], line);
    assert_eq!(a["phrase"], "pistol");

    // The writer edits the line: the phrase moved and is found again.
    edit_line(
        &env,
        &scenes[0],
        1,
        "Soaked, Arjun enters carrying a pistol.",
    );
    let a = env.ok("breakdown.source_anchor", json!({ "id": el["id"] }));
    assert_eq!(
        (a["status"].as_str(), a["start"].as_i64()),
        (Some("moved"), Some(32))
    );
    // The phrase moves to another line of the scene.
    edit_line(&env, &scenes[0], 1, "Soaked, Arjun enters.");
    edit_line(&env, &scenes[0], 2, "He places the PISTOL on the desk.");
    let a = env.ok("breakdown.source_anchor", json!({ "id": el["id"] }));
    assert_eq!(a["status"], "moved");
    assert_eq!(a["elementId"], element_id(&env, &scenes[0], 2));
    // Gone from the script: reported, and the confirmed element is kept.
    edit_line(&env, &scenes[0], 2, "He places the folder on the desk.");
    let a = env.ok("breakdown.source_anchor", json!({ "id": el["id"] }));
    assert_eq!(a["status"], "missing");
    assert!(a["start"].is_null());
    assert_eq!(
        count(
            &env,
            "SELECT count(*) FROM breakdown_element WHERE deleted_at IS NULL AND display_name = 'Pistol'"
        ),
        1
    );

    // Manual elements without a phrase say so.
    let manual = add(&env, &scenes[0], "Wardrobe", "Wet coat");
    assert_eq!(
        env.ok("breakdown.source_anchor", json!({ "id": manual["id"] }))["status"],
        "none"
    );
    // Telugu text: UTF-16 offsets are exact.
    let tline = element_id(&env, &scenes[2], 1);
    let word = "కౌంటర్";
    let start = "రవి ".encode_utf16().count();
    let end = start + word.encode_utf16().count();
    let el = env.ok(
        "breakdown.add_element",
        json!({ "sceneId": scenes[2], "category": "Location / Set", "name": "Counter", "span": { "elementId": tline, "start": start, "end": end } }),
    );
    assert_eq!(el["evidence"], word);
    let a = env.ok("breakdown.source_anchor", json!({ "id": el["id"] }));
    assert_eq!(
        (a["status"].as_str(), a["start"].as_i64()),
        (Some("exact"), Some(start as i64))
    );
    assert_eq!(
        env.err("breakdown.source_anchor", json!({ "id": "nope" })),
        "not_found.breakdown_element"
    );
}

// --------------------------------------- script changes never delete confirmed data

#[test]
fn script_changes_never_silently_delete_confirmed_breakdown_data() {
    let (env, sp, scenes) = setup();
    let line = element_id(&env, &scenes[0], 1);
    env.ok(
        "breakdown.add_element",
        json!({ "sceneId": scenes[0], "category": "Props", "name": "Pistol", "span": { "elementId": line, "start": 24, "end": 30 } }),
    );
    add(&env, &scenes[0], "Props", "Red Folder");
    add(&env, &scenes[1], "Props", "Lamp");
    add(&env, &scenes[3], "Location / Set", "Platform");
    env.ok("breakdown.suggest", json!({ "sceneId": scenes[0] }));
    let confirmed = "SELECT count(*) FROM breakdown_element WHERE deleted_at IS NULL AND confirmation_state IN ('Confirmed','Manual')";
    let kept = count(&env, confirmed);
    assert_eq!(kept, 4);

    // 1. In-place edits remove the tagged words and a whole line.
    edit_line(
        &env,
        &scenes[0],
        1,
        "Arjun enters. Rain drips from his coat.",
    );
    fixture(&env, |c| {
        c.execute(
            "DELETE FROM screenplay_element WHERE scene_id = ?1 AND position = 2",
            [&scenes[0]],
        )
        .unwrap();
    });
    let changes = env.ok("breakdown.scene_changes", json!({ "sceneId": scenes[0] }));
    let review: Vec<&str> = changes["elementsToReview"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["name"].as_str().unwrap())
        .collect();
    assert!(
        review.contains(&"Pistol") && review.contains(&"Red Folder"),
        "{review:?}"
    );
    env.ok(
        "breakdown.apply_suggested_update",
        json!({ "sceneId": scenes[0] }),
    );
    assert_eq!(
        count(&env, confirmed),
        kept,
        "apply suggested update keeps confirmed elements"
    );
    env.ok("breakdown.mark_reviewed", json!({ "sceneId": scenes[0] }));
    assert_eq!(count(&env, confirmed), kept);

    // 2. A new draft drops scene 4; the source update keeps its breakdown as history.
    let (d7, _) = draft(
        &env,
        &sp,
        "Draft 7",
        7,
        &[
            ("L1", "INT. POLICE STATION - NIGHT", S1),
            ("L2", "EXT. OLD RAILWAY STATION - NIGHT", S2),
            ("L3", "INT. TICKET OFFICE - DAY", S3),
        ],
    );
    env.ok("production.apply_update", json!({ "draftId": d7 }));
    assert_eq!(
        count(&env, confirmed),
        kept,
        "nothing deleted by the source update"
    );
    let v = env.ok("breakdown.scenes", json!({}));
    let hist = v["historical"].as_array().unwrap();
    assert_eq!(hist.len(), 1);
    assert_eq!(hist[0]["elements"][0]["name"], "Platform");

    // 3. The scene disappears from the screenplay entirely: the rows are still there.
    fixture(&env, |c| {
        c.execute(
            "UPDATE screenplay_scene SET heading = 'INT. NEW PLACE - DAY' WHERE id = ?1",
            [&scenes[1]],
        )
        .unwrap();
    });
    assert_eq!(count(&env, confirmed), kept);
    // 4. Restart: still there.
    let path = env.project_path();
    let env = env.restart();
    env.reopen_project(&path);
    assert_eq!(count(&env, confirmed), kept);
}

// -------------------------------------------------- locations and people

#[test]
fn locations_hold_call_sheet_fields_availability_notes_and_photos() {
    let (env, _, _) = setup();
    let loc = env.ok(
        "locations.create",
        json!({ "name": "Old Railway Station", "address": "Platform Rd, Gudur", "contact": "Mr. Rao 98000 10012",
                "nearestHospital": "District Hospital, NH-16, 4 km", "emergencyContact": "Station office 0861 222 333" }),
    );
    assert_eq!(loc["nearestHospital"], "District Hospital, NH-16, 4 km");
    assert_eq!(loc["emergencyContact"], "Station office 0861 222 333");
    assert_eq!(loc["contactHidden"], false);
    let id = loc["id"].as_str().unwrap().to_string();
    let loc = env.ok(
        "locations.update",
        json!({ "id": id, "notes": { "access": "Gate B from 5 am", "parking": "12 cars, east gate" }, "nearestHospital": "Apollo Clinic" }),
    );
    assert_eq!(loc["notes"]["access"], "Gate B from 5 am");
    assert_eq!(loc["notes"]["parking"], "12 cars, east gate");
    assert_eq!(loc["nearestHospital"], "Apollo Clinic");
    assert_eq!(
        loc["emergencyContact"], "Station office 0861 222 333",
        "omitted fields are unchanged"
    );
    let photo = env.write_file(
        "platform.png",
        &openframe_test_support::stress::png(4, 4, 1),
    );
    let loc = env.ok(
        "locations.add_photos",
        json!({ "id": id, "paths": [photo] }),
    );
    assert_eq!(loc["photos"].as_array().unwrap().len(), 1);

    // Availability windows.
    let w = env.ok(
        "availability.set",
        json!({ "ownerType": "location", "ownerId": id, "windows": [
            { "startDate": "2027-06-20", "endDate": "2027-06-22", "kind": "Unavailable", "note": "Track repairs" },
            { "startDate": "2027-06-01", "endDate": "2027-06-19", "kind": "Available" }
        ] }),
    );
    assert_eq!(w[0]["startDate"], "2027-06-01", "sorted by first day");
    let loc = env.ok("locations.get", json!({ "id": id }));
    assert_eq!(loc["availability"].as_array().unwrap().len(), 2);
    for bad in [
        json!({ "startDate": "2027-02-30", "endDate": "2027-03-01", "kind": "Available" }),
        json!({ "startDate": "2027-06-10", "endDate": "2027-06-01", "kind": "Available" }),
        json!({ "startDate": "2027-06-10", "endDate": "2027-06-11", "kind": "Maybe" }),
    ] {
        assert_validation(env.err(
            "availability.set",
            json!({ "ownerType": "location", "ownerId": id, "windows": [bad] }),
        ));
    }
    assert_eq!(
        env.ok("locations.get", json!({ "id": id }))["availability"]
            .as_array()
            .unwrap()
            .len(),
        2,
        "a failed save changes nothing"
    );
    env.ok(
        "availability.set",
        json!({ "ownerType": "location", "ownerId": id, "windows": [] }),
    );
    assert!(
        env.ok("locations.get", json!({ "id": id }))["availability"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    env.undo();
    assert_eq!(
        env.ok("locations.get", json!({ "id": id }))["availability"]
            .as_array()
            .unwrap()
            .len(),
        2,
        "one undo step"
    );
    assert_eq!(
        env.err(
            "availability.set",
            json!({ "ownerType": "location", "ownerId": "nope", "windows": [] })
        ),
        "not_found.person_or_location"
    );

    // Search: hospital and availability notes are found; contacts never are.
    assert!(search_ids(&env, "Apollo").contains(&id));
    assert!(search_ids(&env, "repairs").contains(&id));
    assert!(!search_ids(&env, "98000").contains(&id));
    assert!(!search_ids(&env, "0861").contains(&id));

    // Restart keeps everything; delete and purge also remove the windows.
    let path = env.project_path();
    let env = env.restart();
    env.reopen_project(&path);
    let loc = env.ok("locations.get", json!({ "id": id }));
    assert_eq!(loc["availability"].as_array().unwrap().len(), 2);
    assert_eq!(loc["nearestHospital"], "Apollo Clinic");
    env.ok("locations.delete", json!({ "id": id }));
    let trash = env.ok("trash.list", json!({}));
    let t = trash
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["objectId"] == id.as_str())
        .unwrap()
        .clone();
    env.ok("trash.purge", json!({ "id": t["id"] }));
    assert_eq!(count(&env, "SELECT count(*) FROM availability_window"), 0);
}

#[test]
fn cast_and_crew_hold_contact_availability_notes_and_archive() {
    let (env, _, _) = setup();
    let cast = env.ok(
        "cast.create",
        json!({ "personName": "Karthik Menon", "characterName": "ARJUN", "contact": "98xxx 10001", "availabilityNotes": "Free in June", "notes": "Prefers early calls" }),
    );
    let cid = cast["id"].as_str().unwrap().to_string();
    env.ok(
        "availability.set",
        json!({ "ownerType": "cast_member", "ownerId": cid, "windows": [
            { "startDate": "2027-06-09", "endDate": "2027-06-20", "kind": "Unavailable", "note": "Exams" } ] }),
    );
    let crew = env.ok(
        "crew.create",
        json!({ "personName": "Priya Nambiar", "role": "Sound Recordist", "department": "Sound", "contact": "priya@example.com", "availabilityNotes": "Weekends only" }),
    );
    let wid = crew["id"].as_str().unwrap().to_string();
    assert_eq!(crew["availabilityNotes"], "Weekends only");
    env.ok(
        "availability.set",
        json!({ "ownerType": "crew_member", "ownerId": wid, "windows": [
            { "startDate": "2027-06-01", "endDate": "2027-06-30", "kind": "Available" } ] }),
    );
    let crew = env.ok(
        "crew.update",
        json!({ "id": wid, "availabilityNotes": "Any day" }),
    );
    assert_eq!(crew["availabilityNotes"], "Any day");
    assert_eq!(crew["availability"].as_array().unwrap().len(), 1);
    let dir = env.ok("cast.list", json!({}));
    let m = dir["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["id"] == cid.as_str())
        .unwrap()
        .clone();
    assert_eq!(m["availability"][0]["note"], "Exams");
    assert_eq!(m["contact"], "98xxx 10001");

    // Archive keeps the person and their availability.
    env.ok("crew.set_archived", json!({ "id": wid, "archived": true }));
    assert!(
        env.ok("crew.list", json!({}))
            .as_array()
            .unwrap()
            .is_empty()
    );
    let all = env.ok("crew.list", json!({ "includeArchived": true }));
    assert_eq!(all[0]["availability"].as_array().unwrap().len(), 1);

    // Search covers role, department and availability, never contact details.
    assert!(search_ids(&env, "Recordist").contains(&wid));
    assert!(search_ids(&env, "Exams").contains(&cid));
    assert!(!search_ids(&env, "example.com").contains(&wid));
    assert!(!search_ids(&env, "10001").contains(&cid));
}

#[test]
fn contact_data_is_hidden_by_role_from_the_assistant_and_from_exports() {
    let (env, _, scenes) = setup();
    let item = env.ok("catalog.create", json!({ "category": "Vehicles", "name": "Red Motorcycle", "contact": "Owner Suresh 98111 22222" }));
    env.ok(
        "breakdown.add_element",
        json!({ "sceneId": scenes[0], "category": "Vehicles", "name": "Red Motorcycle", "catalog": { "mode": "existing", "catalogItemId": item["id"] } }),
    );
    let loc = env.ok("locations.create", json!({ "name": "Police Station", "contact": "Inspector 98222 33333", "emergencyContact": "Control room 100 9999" }));
    env.ok(
        "cast.create",
        json!({ "personName": "Divya Nair", "characterName": "MEERA", "contact": "98xxx 10004" }),
    );
    env.ok(
        "crew.create",
        json!({ "personName": "Rohan Das", "role": "1st AD", "contact": "rohan@example.com" }),
    );
    let secrets = ["98111", "98222", "9999", "10004", "rohan@example.com"];

    for role in [Role::Commenter, Role::Viewer, Role::ExportOnly] {
        let a = env.actor_with_role(role);
        let out = [
            env.call_as(&a, "catalog.list", json!({})).unwrap(),
            env.call_as(&a, "catalog.get", json!({ "id": item["id"] }))
                .unwrap(),
            env.call_as(&a, "locations.list", json!({})).unwrap(),
            env.call_as(&a, "locations.get", json!({ "id": loc["id"] }))
                .unwrap(),
            env.call_as(&a, "cast.list", json!({})).unwrap(),
            env.call_as(&a, "crew.list", json!({})).unwrap(),
        ];
        let text = serde_json::to_string(&out).unwrap();
        for s in secrets {
            assert!(!text.contains(s), "{role:?} sees {s}");
        }
        assert!(
            text.contains("\"contactHidden\":true"),
            "{role:?}: hidden is shown as hidden"
        );
    }
    for role in [Role::Owner, Role::Editor] {
        let a = env.actor_with_role(role);
        let text = serde_json::to_string(
            &env.call_as(&a, "locations.get", json!({ "id": loc["id"] }))
                .unwrap(),
        )
        .unwrap();
        assert!(
            text.contains("98222") && text.contains("9999"),
            "{role:?} sees contacts"
        );
    }

    // The assistant's read tools never return contact fields, even for the Owner.
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
    for (tool, args) in [
        ("production_catalog", json!({ "item": "Red Motorcycle" })),
        (
            "production_locations",
            json!({ "location": "Police Station" }),
        ),
        ("cast_and_crew", json!({})),
    ] {
        let step =
            toolbox::run_tool_unlocked(&env.core, &actor, &scope, "who to call", tool, &args)
                .unwrap();
        let ToolStep::Output(o) = step else {
            panic!("{tool} returned a proposal")
        };
        let text = format!(
            "{} {:?} {:?}",
            o.content,
            o.details,
            o.items
                .iter()
                .map(|i| (&i.label, &i.detail))
                .collect::<Vec<_>>()
        );
        for s in secrets {
            assert!(!text.contains(s), "{tool} gave the assistant {s}");
        }
    }

    // Production exports never carry contact data.
    for (content, ext) in [("both", "xlsx"), ("catalog", "csv"), ("summary", "csv")] {
        let path = env.write_file(&format!("out-{content}.{ext}"), b"");
        std::fs::remove_file(&path).ok();
        env.ok(
            "breakdown.export_report",
            json!({ "format": ext, "path": path, "content": content }),
        );
        let bytes = std::fs::read(&path).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        for s in secrets {
            assert!(!text.contains(s), "{content} export contains {s}");
        }
    }
}

// -------------------------------------------- stable identity and references

#[test]
fn renames_keep_links_and_deletes_name_their_references() {
    let (env, _, scenes) = setup();
    let el = add(&env, &scenes[0], "Props", "Red Folder");
    add(&env, &scenes[1], "Props", "Red Folder");
    let item = el["catalogItemId"].as_str().unwrap().to_string();
    env.ok("catalog.update", json!({ "id": item, "name": "Case File" }));
    let d = env.ok("breakdown.scene", json!({ "sceneId": scenes[0] }));
    assert_eq!(
        d["groups"][0]["confirmed"][0]["name"], "Case File",
        "rename shows in every scene"
    );
    assert_eq!(
        d["groups"][0]["confirmed"][0]["catalogItemId"],
        item.as_str()
    );
    // The old name still finds the same identity (search and matching).
    assert!(search_ids(&env, "Red Folder").contains(&item));
    let m = env.ok(
        "catalog.find_matches",
        json!({ "category": "Props", "name": "red folder" }),
    );
    assert_eq!(m[0]["id"], item.as_str());

    let refs = env.ok(
        "production.references",
        json!({ "kind": "catalog_item", "id": item }),
    );
    assert_eq!(refs["name"], "Case File");
    assert_eq!(refs["scenes"].as_array().unwrap().len(), 2);
    assert!(
        refs["links"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l == "Also known as: Red Folder")
    );
    assert!(refs["consequence"].as_str().unwrap().contains("2 scenes"));

    // A Location / Set item and its location are one identity.
    let set = add(&env, &scenes[1], "Location / Set", "Old Railway Station");
    let loc_id = env.ok("catalog.get", json!({ "id": set["catalogItemId"] }))["locationId"]
        .as_str()
        .unwrap()
        .to_string();
    env.ok(
        "locations.update",
        json!({ "id": loc_id, "name": "Gudur Junction" }),
    );
    assert_eq!(
        env.ok("catalog.get", json!({ "id": set["catalogItemId"] }))["name"],
        "Gudur Junction"
    );
    let lref = env.ok(
        "production.references",
        json!({ "kind": "location", "id": loc_id }),
    );
    assert_eq!(lref["scenes"][0]["number"], "2");
    assert!(
        lref["links"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l == "Catalog item: Gudur Junction")
    );

    // Cast rename keeps the character link and scenes.
    let cast = env.ok(
        "cast.create",
        json!({ "personName": "K. Menon", "characterName": "ARJUN" }),
    );
    let cast = env.ok(
        "cast.update",
        json!({ "id": cast["id"], "personName": "Karthik Menon" }),
    );
    assert_eq!(cast["characterName"], "ARJUN");
    assert!(!cast["scenes"].as_array().unwrap().is_empty());
    let cref = env.ok(
        "production.references",
        json!({ "kind": "cast_member", "id": cast["id"] }),
    );
    assert!(
        cref["links"][0]
            .as_str()
            .unwrap()
            .starts_with("Plays ARJUN")
    );
    assert!(
        cref["consequence"]
            .as_str()
            .unwrap()
            .contains("Needs casting")
    );
    let crew = env.ok(
        "crew.create",
        json!({ "personName": "Imran", "role": "Cinematographer" }),
    );
    let wref = env.ok(
        "production.references",
        json!({ "kind": "crew_member", "id": crew["id"] }),
    );
    assert!(wref["scenes"].as_array().unwrap().is_empty());

    // Delete is explicit and recoverable: the scenes keep a visible "removed" entry.
    env.ok("catalog.delete", json!({ "id": item }));
    let d = env.ok("breakdown.scene", json!({ "sceneId": scenes[0] }));
    let e = d["groups"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|g| g["confirmed"].as_array().unwrap().clone())
        .find(|e| e["catalogItemId"] == item.as_str())
        .unwrap();
    assert_eq!(e["catalogRemoved"], true);
    let trash = env.ok("trash.list", json!({}));
    let t = trash
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["objectId"] == item.as_str())
        .unwrap()
        .clone();
    env.ok("trash.restore", json!({ "id": t["id"] }));
    let d = env.ok("breakdown.scene", json!({ "sceneId": scenes[0] }));
    assert_eq!(
        d["groups"][0]["confirmed"][0]["catalogRemoved"], false,
        "restore relinks"
    );
    assert_eq!(
        env.err(
            "production.references",
            json!({ "kind": "catalog_item", "id": "nope" })
        ),
        "not_found.record"
    );
}

// ------------------------------------------------------------- report

#[test]
fn short_breakdown_report_lists_totals_per_category() {
    let (env, _, scenes) = setup();
    add(&env, &scenes[0], "Props", "Pistol");
    add(&env, &scenes[1], "Props", "Pistol");
    add(&env, &scenes[1], "Props", "Lamp");
    add(&env, &scenes[0], "Cast", "Arjun");
    env.ok(
        "breakdown.set_complete",
        json!({ "sceneId": scenes[0], "complete": true }),
    );
    env.ok("breakdown.suggest", json!({ "sceneId": scenes[2] }));
    let path = env.write_file("summary.csv", b"");
    std::fs::remove_file(&path).ok();
    let r = env.ok(
        "breakdown.export_report",
        json!({ "format": "csv", "path": path, "content": "summary" }),
    );
    assert_eq!(r["document"], "Breakdown Summary");
    let text = std::fs::read_to_string(&path).unwrap();
    let text = text.trim_start_matches('\u{feff}');
    assert!(
        text.starts_with("Category,Items,Scenes,Items needed"),
        "{text}"
    );
    assert!(text.contains("Cast,1,1,Arjun"), "{text}");
    assert!(text.contains("Props,2,2,\"Lamp, Pistol\""), "{text}");
    let pdf = env.write_file("summary.pdf", b"");
    std::fs::remove_file(&pdf).ok();
    env.ok(
        "breakdown.export_report",
        json!({ "format": "pdf", "path": pdf, "content": "summary" }),
    );
    assert!(std::fs::metadata(&pdf).unwrap().len() > 500);
}

// ------------------------------------------------------------ migration

#[test]
fn schema_13_projects_upgrade_to_14_and_keep_production_data() {
    use openframe_application::schema::PROJECT_MIGRATIONS;
    use openframe_persistence::migrate;
    let v13 = PROJECT_MIGRATIONS
        .iter()
        .position(|m| m.version == 13)
        .unwrap()
        + 1;
    let mut c = openframe_persistence::open_in_memory().unwrap();
    migrate::apply(&mut c, &PROJECT_MIGRATIONS[..v13]).unwrap();
    c.execute(
        "INSERT INTO location(id, name, contact, notes_json, created_at, updated_at) VALUES ('l1', 'Station', 'Rao', '{\"parking\":\"East gate\"}', 1, 1)",
        [],
    )
    .unwrap();
    c.execute("INSERT INTO crew_member(id, person_name, role, contact, created_at, updated_at) VALUES ('c1', 'Imran', 'DOP', 'x@y.z', 1, 1)", []).unwrap();
    // Upgrade to exactly schema 14; later migrations have their own fixture tests.
    let v14 = PROJECT_MIGRATIONS
        .iter()
        .position(|m| m.version == 14)
        .unwrap()
        + 1;
    let applied = migrate::apply(&mut c, &PROJECT_MIGRATIONS[..v14]).unwrap();
    assert_eq!(applied, vec![14]);
    let (notes, contact, hospital): (String, String, Option<String>) = c
        .query_row(
            "SELECT notes_json, contact, nearest_hospital FROM location WHERE id='l1'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(
        (notes.as_str(), contact.as_str(), hospital),
        ("{\"parking\":\"East gate\"}", "Rao", None)
    );
    let avail: Option<String> = c
        .query_row(
            "SELECT availability_notes FROM crew_member WHERE id='c1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(avail.is_none());
    assert_eq!(
        c.query_row("SELECT count(*) FROM availability_window", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    openframe_persistence::undo::install_capture(&c).expect("new tables are undo-trackable");
}

// ------------------------------------------------- project package round trip

fn task_result(env: &TestEnv, started: &Value) -> Value {
    use openframe_application::events::AppEvent;
    let id = started["taskId"].as_str().unwrap().to_string();
    let t0 = std::time::Instant::now();
    loop {
        {
            let events = env.sink.events.lock();
            for e in events.iter().rev() {
                if let AppEvent::Task(t) = e
                    && t.task_id == id
                    && matches!(t.state.as_str(), "completed" | "failed" | "cancelled")
                {
                    assert_eq!(t.state, "completed", "task failed: {:?}", t.error);
                    return t.result.clone().unwrap_or(Value::Null);
                }
            }
        }
        assert!(
            t0.elapsed() < std::time::Duration::from_secs(120),
            "task {id} did not finish"
        );
        std::thread::sleep(std::time::Duration::from_millis(15));
    }
}

#[test]
fn availability_hospital_and_category_looks_travel_in_a_project_package() {
    let (a, _, _) = setup();
    let loc = a.ok(
        "locations.create",
        json!({ "name": "Signal Cabin", "nearestHospital": "Town Hospital" }),
    );
    a.ok(
        "availability.set",
        json!({ "ownerType": "location", "ownerId": loc["id"], "windows": [
            { "startDate": "2027-06-20", "endDate": "2027-06-22", "kind": "Unavailable", "note": "Track repairs" } ] }),
    );
    a.ok(
        "breakdown.set_category_style",
        json!({ "category": "Props", "color": "red" }),
    );
    let dir = a.dir.path().join("out");
    std::fs::create_dir_all(&dir).unwrap();
    let pkg = dir
        .join("Black Rain.ofproject")
        .to_string_lossy()
        .into_owned();
    let started = a.ok("packages.export_project", json!({ "path": pkg }));
    task_result(&a, &started);

    let b = TestEnv::new();
    let started = b.ok(
        "packages.import_project",
        json!({ "path": pkg, "mode": "open" }),
    );
    task_result(&b, &started);
    let got = b.ok("locations.get", json!({ "id": loc["id"] }));
    assert_eq!(got["nearestHospital"], "Town Hospital");
    assert_eq!(got["availability"][0]["note"], "Track repairs");
    let styles = b.ok("breakdown.categories", json!({}));
    assert_eq!(styles["categories"][3]["color"], "red");
}

// ------------------------------------------- edge cases: scripts, size, windows, drive

fn data_changed_tables(env: &TestEnv) -> Vec<String> {
    use openframe_application::events::AppEvent;
    env.sink
        .events
        .lock()
        .iter()
        .filter_map(|e| match e {
            AppEvent::DataChanged { tables, .. } => Some(tables.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

#[test]
fn telugu_and_hindi_records_huge_inputs_a_second_window_and_a_full_drive() {
    let (env, _, scenes) = setup();

    // Telugu and Hindi names, notes and hospital text are stored as typed and found by search.
    let loc = env.ok(
        "locations.create",
        json!({ "name": "పాత రైల్వే స్టేషన్", "nearestHospital": "जिला अस्पताल, गुडूर" }),
    );
    let cast = env.ok(
        "cast.create",
        json!({ "personName": "కార్తీక్ మేనన్", "characterName": "ARJUN", "contact": "98xxx 10001" }),
    );
    let crew = env.ok(
        "crew.create",
        json!({ "personName": "प्रिया नम्बियार", "role": "ध्वनि रिकॉर्डिस्ट", "department": "Sound" }),
    );
    env.ok(
        "availability.set",
        json!({ "ownerType": "cast_member", "ownerId": cast["id"], "windows": [
            { "startDate": "2027-06-09", "endDate": "2027-06-20", "kind": "Unavailable", "note": "పరీక్షలు" } ] }),
    );
    let got = env.ok("locations.get", json!({ "id": loc["id"] }));
    assert_eq!(got["name"], "పాత రైల్వే స్టేషన్");
    assert_eq!(got["nearestHospital"], "जिला अस्पताल, गुडूर");
    assert!(search_ids(&env, "రైల్వే").contains(&loc["id"].as_str().unwrap().to_string()));
    assert!(search_ids(&env, "प्रिया").contains(&crew["id"].as_str().unwrap().to_string()));
    assert!(
        !search_ids(&env, "10001").contains(&cast["id"].as_str().unwrap().to_string()),
        "contact data is never in the search index"
    );

    // Empty batches change nothing; huge batches are refused before anything is written.
    let before = count(&env, "SELECT count(*) FROM breakdown_element");
    assert_eq!(
        env.ok(
            "breakdown.reuse",
            json!({ "sceneIds": [], "catalogItemIds": [] })
        )["count"],
        0
    );
    let many_scenes: Vec<String> = (0..101).map(|i| format!("s{i}")).collect();
    let many_items: Vec<String> = (0..50).map(|i| format!("c{i}")).collect();
    assert_validation(env.err(
        "breakdown.reuse",
        json!({ "sceneIds": many_scenes, "catalogItemIds": many_items }),
    ));
    assert_eq!(
        count(&env, "SELECT count(*) FROM breakdown_element"),
        before
    );
    let too_many: Vec<Value> = (0..201)
        .map(|_| json!({ "startDate": "2027-06-01", "endDate": "2027-06-01", "kind": "Available" }))
        .collect();
    assert_validation(env.err(
        "availability.set",
        json!({ "ownerType": "location", "ownerId": loc["id"], "windows": too_many }),
    ));
    assert_validation(env.err(
        "availability.set",
        json!({ "ownerType": "location", "ownerId": loc["id"], "windows": [
            { "startDate": "2027-06-01", "endDate": "2027-06-01", "kind": "Available", "note": "x".repeat(501) } ] }),
    ));
    assert!(
        env.ok("locations.get", json!({ "id": loc["id"] }))["availability"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    // A second window learns about every change through the data-changed event and its tables,
    // and an edit made there after the record was deleted here is refused, not resurrected.
    env.sink.events.lock().clear();
    env.ok(
        "breakdown.set_category_style",
        json!({ "category": "Props", "color": "blue" }),
    );
    env.ok(
        "availability.set",
        json!({ "ownerType": "crew_member", "ownerId": crew["id"], "windows": [
            { "startDate": "2027-06-02", "endDate": "2027-06-02", "kind": "Available" } ] }),
    );
    let tables = data_changed_tables(&env);
    assert!(
        tables.iter().any(|t| t == "breakdown_category_style"),
        "{tables:?}"
    );
    assert!(
        tables.iter().any(|t| t == "availability_window"),
        "{tables:?}"
    );
    assert!(
        tables.iter().any(|t| t == "crew_member"),
        "the owner refreshes too: {tables:?}"
    );
    env.ok("locations.delete", json!({ "id": loc["id"] }));
    assert_eq!(
        env.err(
            "availability.set",
            json!({ "ownerType": "location", "ownerId": loc["id"], "windows": [] }),
        ),
        "not_found.person_or_location"
    );

    // A full drive: the batch is refused whole and the same batch succeeds once space is free.
    let pistol = add(&env, &scenes[0], "Props", "Pistol");
    let before = count(
        &env,
        "SELECT count(*) FROM breakdown_element WHERE deleted_at IS NULL",
    );
    #[cfg(debug_assertions)]
    {
        use openframe_application::store::fault::{self, CommitFault};
        fault::fail_commits(Some(CommitFault::DiskFull));
        let code = env.err(
            "breakdown.reuse",
            json!({ "sceneIds": [scenes[1], scenes[2]], "catalogItemIds": [pistol["catalogItemId"]] }),
        );
        let style = env.err(
            "breakdown.set_category_style",
            json!({ "category": "Sound", "color": "red" }),
        );
        fault::fail_commits(None);
        assert_eq!(code, "storage.disk_full");
        assert_eq!(style, "storage.disk_full");
        assert_eq!(
            count(
                &env,
                "SELECT count(*) FROM breakdown_element WHERE deleted_at IS NULL"
            ),
            before
        );
    }
    let r = env.ok(
        "breakdown.reuse",
        json!({ "sceneIds": [scenes[1], scenes[2]], "catalogItemIds": [pistol["catalogItemId"]] }),
    );
    assert_eq!(r["count"], 2);
}
