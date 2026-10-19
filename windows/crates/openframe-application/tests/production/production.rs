//! Production module acceptance tests: Production Source, Breakdown,
//! Suggestions, Catalog, Locations, Cast & Crew (FSD §26–30, §53–54, §96–100,
//! §108, §124–125, §143–144, §160–161).
//!
//! Screenplay content is seeded directly into the screenplay hub tables
//! (test fixtures only) through the store pipeline.

use openframe_application::MutationMeta;
use openframe_domain::{Capability, Role, new_id, now_ms};
use openframe_test_support::TestEnv;
use rusqlite::params;
use serde_json::{Value, json};

// ---------------------------------------------------------------- fixtures

struct Script {
    screenplay_id: String,
}

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

fn screenplay(env: &TestEnv) -> Script {
    let id = new_id();
    fixture(env, |c| {
        c.execute(
            "INSERT INTO screenplay(id, title, created_at, updated_at) VALUES (?1, 'Black Rain', ?2, ?2)",
            params![id, now_ms()],
        )
        .unwrap();
    });
    Script { screenplay_id: id }
}

type SceneSpec<'a> = (&'a str, &'a str, &'a [(&'a str, &'a str)]);

impl Script {
    /// Create a draft with scenes (lineage, heading, elements). `order` makes created_at increase.
    fn draft(
        &self,
        env: &TestEnv,
        name: &str,
        status: &str,
        order: i64,
        scenes: &[SceneSpec<'_>],
    ) -> (String, Vec<String>) {
        let draft = new_id();
        let mut ids = Vec::new();
        fixture(env, |c| {
            let t = 1_700_000_000_000 + order * 1000;
            c.execute(
                "INSERT INTO screenplay_draft(id, screenplay_id, name, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                params![draft, self.screenplay_id, name, status, t],
            )
            .unwrap();
            c.execute(
                "UPDATE screenplay SET current_draft_id = ?1 WHERE id = ?2",
                params![draft, self.screenplay_id],
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

fn edit_element(env: &TestEnv, scene_id: &str, position: i64, text: &str) {
    fixture(env, |c| {
        c.execute(
            "UPDATE screenplay_element SET text=?1, updated_at=?2, rev=rev+1 WHERE scene_id=?3 AND position=?4",
            params![text, now_ms(), scene_id, position],
        )
        .unwrap();
    });
}

/// Every screenplay row, to prove production never rewrites the script.
fn screenplay_snapshot(env: &TestEnv) -> Vec<String> {
    let s = env.core.project().unwrap();
    s.store
        .read(|c| {
            let mut out = Vec::new();
            for sql in [
                "SELECT id || '|' || heading || '|' || rev FROM screenplay_scene ORDER BY id",
                "SELECT id || '|' || text || '|' || rev FROM screenplay_element ORDER BY id",
                "SELECT id || '|' || name || '|' || status || '|' || rev FROM screenplay_draft ORDER BY id",
            ] {
                let mut stmt = c.prepare(sql)?;
                let rows: Vec<String> = stmt.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
                out.extend(rows);
            }
            Ok(out)
        })
        .unwrap()
}

fn count(env: &TestEnv, sql: &str) -> i64 {
    let s = env.core.project().unwrap();
    s.store
        .read(|c| Ok(c.query_row(sql, [], |r| r.get(0))?))
        .unwrap()
}

const SCENE_12: &[(&str, &str)] = &[
    (
        "action",
        "Arjun enters carrying a pistol. Rain drips from his coat.",
    ),
    ("action", "He places the RED FOLDER on the desk."),
    ("character", "MEERA"),
    ("dialogue", "You said you would never come back."),
    ("character", "ARJUN"),
    ("parenthetical", "(quietly)"),
    ("dialogue", "Neither did the folder."),
];
const SCENE_13: &[(&str, &str)] = &[
    ("action", "Arjun checks the pistol under a flickering lamp."),
    ("character", "ARJUN"),
    ("dialogue", "Not tonight."),
];
const SCENE_14: &[(&str, &str)] = &[
    (
        "action",
        "Ravi waits at the counter. A crowd of passengers pushes past.",
    ),
    ("character", "RAVI"),
    ("dialogue", "He's late."),
];

/// Project with a "Draft 6 — Shooting Draft" (3 scenes) set as Production Source.
fn setup() -> (TestEnv, Script, String, Vec<String>) {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let sp = screenplay(&env);
    let (draft, scenes) = sp.draft(
        &env,
        "Draft 6 — Shooting Draft",
        "Locked",
        6,
        &[
            ("L12", "INT. POLICE STATION — NIGHT", SCENE_12),
            ("L13", "EXT. OLD RAILWAY STATION — NIGHT", SCENE_13),
            ("L14", "INT. TICKET OFFICE - DAY", SCENE_14),
        ],
    );
    env.ok(
        "production.set_source",
        json!({ "draftId": draft, "reason": "Locked shooting draft" }),
    );
    (env, sp, draft, scenes)
}

fn add(env: &TestEnv, scene: &str, category: &str, name: &str) -> Value {
    env.ok(
        "breakdown.add_element",
        json!({ "sceneId": scene, "category": category, "name": name }),
    )
}

fn detail(env: &TestEnv, scene: &str) -> Value {
    env.ok("breakdown.scene", json!({ "sceneId": scene }))
}

fn suggestions(d: &Value) -> Vec<Value> {
    d["groups"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|g| g["suggestions"].as_array().unwrap().clone())
        .collect()
}

fn confirmed(d: &Value) -> Vec<Value> {
    d["groups"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|g| g["confirmed"].as_array().unwrap().clone())
        .collect()
}

// ------------------------------------------------------------ source

#[test]
fn fsd_brk_001_breakdown_requires_and_shows_the_selected_source() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let view = env.ok("breakdown.scenes", json!({}));
    assert!(
        view["source"].is_null(),
        "no source until the user chooses one"
    );
    assert!(view["scenes"].as_array().unwrap().is_empty());

    let sp = screenplay(&env);
    let (d6, _) = sp.draft(
        &env,
        "Draft 6 — Shooting Draft",
        "Locked",
        6,
        &[("L1", "INT. HOUSE - DAY", &[("action", "A cup.")])],
    );
    let (d7, _) = sp.draft(
        &env,
        "Draft 7 — Director Rewrite",
        "Draft",
        7,
        &[
            ("L1", "INT. HOUSE - DAY", &[("action", "A cup.")]),
            ("L2", "EXT. ROAD - NIGHT", &[]),
        ],
    );
    let opts = env.ok("production.drafts", json!({}));
    assert_eq!(opts["drafts"].as_array().unwrap().len(), 2);
    assert!(opts["activeDraftId"].is_null());

    // Choosing an older, locked draft is allowed; the latest is not assumed.
    let src = env.ok("production.set_source", json!({ "draftId": d6 }));
    assert_eq!(src["draft"]["name"], "Draft 6 — Shooting Draft");
    assert_eq!(src["draft"]["sceneCount"], 1);
    assert_eq!(src["reason"], Value::Null);
    assert_eq!(
        src["newer"]["id"], d7,
        "newer revision is reported, not applied"
    );

    let view = env.ok("breakdown.scenes", json!({}));
    assert_eq!(
        view["source"]["draft"]["id"], d6,
        "source visible at the top of Breakdown"
    );
    let scenes = view["scenes"].as_array().unwrap();
    assert_eq!(scenes.len(), 1);
    assert_eq!(scenes[0]["number"], "1");
    assert_eq!(scenes[0]["heading"], "INT. HOUSE - DAY");
    assert_eq!(scenes[0]["needsBreakdown"], false);

    // A newer draft never silently replaces the source.
    assert_eq!(
        env.err("production.set_source", json!({ "draftId": d7 })),
        "conflict.production_source_set"
    );
    assert_eq!(env.ok("production.source", json!({}))["draft"]["id"], d6);
    assert_eq!(
        env.err("production.set_source", json!({ "draftId": "nope" })),
        "not_found.draft"
    );
}

#[test]
fn source_selection_is_undoable_and_permission_checked() {
    let env = TestEnv::with_project("Short", "Short Film");
    let sp = screenplay(&env);
    let (d, _) = sp.draft(
        &env,
        "Draft 1",
        "Draft",
        1,
        &[("L1", "INT. ROOM - DAY", &[])],
    );
    let viewer = env.actor_with_role(Role::Viewer);
    let e = env
        .call_as(&viewer, "production.set_source", json!({ "draftId": d }))
        .unwrap_err();
    assert_eq!(e.code.0, "permission.denied");
    env.ok("production.set_source", json!({ "draftId": d }));
    let step = env.undo();
    assert!(
        step["label"]
            .as_str()
            .unwrap()
            .contains("Production Source")
    );
    assert!(env.ok("production.source", json!({})).is_null());
    env.redo();
    assert_eq!(env.ok("production.source", json!({}))["draft"]["id"], d);
}

// ------------------------------------------------------- manual tagging

#[test]
fn fsd_brk_002_manual_tag_from_selected_text_creates_scene_association() {
    let (env, _, _, scenes) = setup();
    let action = element_id(&env, &scenes[0], 1);
    // "pistol" = UTF-16 offsets 24..30 of "Arjun enters carrying a pistol. …"
    let el = env.ok(
        "breakdown.add_element",
        json!({ "sceneId": scenes[0], "category": "Props", "name": "Pistol", "span": { "elementId": action, "start": 24, "end": 30 } }),
    );
    assert_eq!(el["state"], "Manual");
    assert_eq!(el["origin"], "tagged");
    assert_eq!(el["evidence"], "pistol");
    assert_eq!(el["spanStart"], 24);
    assert!(
        el["catalogItemId"].is_string(),
        "new item added to the production catalog"
    );

    let d = detail(&env, &scenes[0]);
    let groups = d["groups"].as_array().unwrap();
    assert_eq!(groups.len(), 1, "empty categories are hidden");
    assert_eq!(groups[0]["category"], "Props");
    assert_eq!(d["confirmedCount"], 1);
    assert_eq!(d["facts"]["intExt"], "INT");
    assert_eq!(d["facts"]["setName"], "Police Station");
    assert_eq!(d["facts"]["timeOfDay"], "NIGHT");
    assert_eq!(
        d["script"].as_array().unwrap().len(),
        SCENE_12.len(),
        "script pane shows the scene text"
    );

    // Only the 11 defined categories.
    assert_eq!(
        env.err(
            "breakdown.add_element",
            json!({ "sceneId": scenes[0], "category": "Snacks", "name": "Tea" })
        ),
        "validation.category"
    );
    assert_eq!(
        env.err(
            "breakdown.add_element",
            json!({ "sceneId": scenes[0], "category": "Props", "name": "  " })
        ),
        "validation.required"
    );
    // Selection must belong to the scene.
    let other = element_id(&env, &scenes[1], 1);
    assert_eq!(
        env.err(
            "breakdown.add_element",
            json!({ "sceneId": scenes[0], "category": "Props", "name": "Lamp", "span": { "elementId": other, "start": 0, "end": 3 } })
        ),
        "validation.invalid_input"
    );
    // The same item twice in one scene is refused.
    assert_eq!(
        env.err(
            "breakdown.add_element",
            json!({ "sceneId": scenes[0], "category": "Props", "name": "pistol" })
        ),
        "conflict.state"
    );

    // Viewers and commenters can read but not tag.
    let viewer = env.actor_with_role(Role::Viewer);
    assert!(
        env.call_as(&viewer, "breakdown.scene", json!({ "sceneId": scenes[0] }))
            .is_ok()
    );
    let commenter = env.actor_with_role(Role::Commenter);
    let e = env
        .call_as(
            &commenter,
            "breakdown.add_element",
            json!({ "sceneId": scenes[0], "category": "Props", "name": "Badge" }),
        )
        .unwrap_err();
    assert_eq!(e.code.0, "permission.denied");

    // Undo removes the association; redo brings back the same identity.
    let before = el["id"].clone();
    env.undo();
    assert!(confirmed(&detail(&env, &scenes[0])).is_empty());
    env.redo();
    assert_eq!(confirmed(&detail(&env, &scenes[0]))[0]["id"], before);
}

#[test]
fn fsd_cat_001_same_prop_across_two_scenes_reuses_the_catalog_item() {
    let (env, _, _, scenes) = setup();
    let a = add(&env, &scenes[0], "Props", "Pistol");
    let b = add(&env, &scenes[1], "Props", "pistols");
    assert_eq!(
        a["catalogItemId"], b["catalogItemId"],
        "one production identity"
    );
    assert_eq!(
        count(
            &env,
            "SELECT count(*) FROM catalog_item WHERE category='Props'"
        ),
        1
    );

    let item = env.ok("catalog.get", json!({ "id": a["catalogItemId"] }));
    let used: Vec<&str> = item["usedIn"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["number"].as_str().unwrap())
        .collect();
    assert_eq!(used, vec!["1", "2"], "Used in Scenes is derived");
    assert_eq!(item["usedInLabel"], "Scenes 1, 2");
    // Renaming the catalog item updates every display.
    env.ok(
        "catalog.update",
        json!({ "id": a["catalogItemId"], "name": "Arjun's Pistol", "status": "Confirmed" }),
    );
    let d = detail(&env, &scenes[1]);
    assert_eq!(confirmed(&d)[0]["name"], "Arjun's Pistol");
    assert_eq!(confirmed(&d)[0]["catalogStatus"], "Confirmed");
    // The old name still matches (alias through the tagged name) for the next scene.
    let c = add(&env, &scenes[2], "Props", "Pistol");
    assert_eq!(c["catalogItemId"], a["catalogItemId"]);
}

#[test]
fn fsd_brk_012_013_ambiguous_matches_require_a_choice() {
    let (env, _, _, scenes) = setup();
    let folder = env.ok(
        "catalog.create",
        json!({ "category": "Props", "name": "Red Folder" }),
    );
    let m = env.ok(
        "catalog.find_matches",
        json!({ "category": "Props", "name": "folder" }),
    );
    assert_eq!(m.as_array().unwrap().len(), 1);
    assert_eq!(m[0]["exact"], false);
    // Similar-but-not-identical is never attached silently.
    assert_eq!(
        env.err(
            "breakdown.add_element",
            json!({ "sceneId": scenes[0], "category": "Props", "name": "Folder" })
        ),
        "validation.ambiguous_match"
    );
    // Use existing: links and remembers the name.
    let el = env.ok(
        "breakdown.add_element",
        json!({ "sceneId": scenes[0], "category": "Props", "name": "Folder", "catalog": { "mode": "existing", "catalogItemId": folder["id"] } }),
    );
    assert_eq!(el["catalogItemId"], folder["id"]);
    assert_eq!(el["name"], "Red Folder");
    let item = env.ok("catalog.get", json!({ "id": folder["id"] }));
    assert_eq!(item["aliases"][0]["alias"], "Folder");
    // …which makes "folder" an exact match later.
    let m = env.ok(
        "catalog.find_matches",
        json!({ "category": "Props", "name": "folder" }),
    );
    assert_eq!(m[0]["exact"], true);
    // Create New is always possible.
    let el2 = env.ok(
        "breakdown.add_element",
        json!({ "sceneId": scenes[1], "category": "Props", "name": "Red Folder", "catalog": { "mode": "new" } }),
    );
    assert_ne!(el2["catalogItemId"], folder["id"]);
    // Category must match an existing item.
    assert_eq!(
        env.err(
            "breakdown.add_element",
            json!({ "sceneId": scenes[2], "category": "Wardrobe", "name": "Folder", "catalog": { "mode": "existing", "catalogItemId": folder["id"] } })
        ),
        "validation.category"
    );
}

#[test]
fn fsd_brk_014_removing_an_element_keeps_the_catalog_item() {
    let (env, _, _, scenes) = setup();
    let el = add(&env, &scenes[0], "Vehicles", "Red Motorcycle");
    let item = el["catalogItemId"].as_str().unwrap().to_string();
    env.ok("breakdown.remove_element", json!({ "id": el["id"] }));
    assert!(confirmed(&detail(&env, &scenes[0])).is_empty());
    let cat = env.ok("catalog.get", json!({ "id": item }));
    assert_eq!(cat["name"], "Red Motorcycle", "catalog item not deleted");
    assert!(cat["usedIn"].as_array().unwrap().is_empty());
    // Recoverable from Recently Deleted.
    let trash = env.ok("trash.list", json!({}));
    assert_eq!(trash[0]["objectType"], "breakdown_element");
    env.ok("trash.restore", json!({ "id": trash[0]["id"] }));
    assert_eq!(confirmed(&detail(&env, &scenes[0]))[0]["id"], el["id"]);
    // Suggestions are rejected, not removed.
    env.ok("breakdown.suggest", json!({ "sceneId": scenes[0] }));
    let sug = suggestions(&detail(&env, &scenes[0]));
    assert_eq!(
        env.err("breakdown.remove_element", json!({ "id": sug[0]["id"] })),
        "validation.invalid_input"
    );
}

// ------------------------------------------------------------ suggestions

#[test]
fn fsd_brk_003_suggestions_never_auto_confirm() {
    let (env, _, _, scenes) = setup();
    let r = env.ok("breakdown.suggest", json!({ "sceneId": scenes[0] }));
    assert!(r["added"].as_i64().unwrap() >= 6);
    let d = detail(&env, &scenes[0]);
    let sug = suggestions(&d);
    assert_eq!(
        d["confirmedCount"], 0,
        "suggestions are not production data"
    );
    assert!(
        sug.iter()
            .all(|s| s["state"] == "Suggested" && s["catalogItemId"].is_null())
    );
    assert_eq!(
        count(&env, "SELECT count(*) FROM catalog_item"),
        0,
        "no catalog items from suggestions"
    );
    let ov = env.ok("production.overview", json!({}));
    assert_eq!(ov["plannedCount"], 0);
    assert!(ov["pendingSuggestions"].as_i64().unwrap() >= 6);

    let find = |cat: &str, name: &str| {
        sug.iter()
            .find(|s| s["category"] == cat && s["name"] == name)
            .cloned()
            .unwrap_or_else(|| panic!("{cat} {name} in {sug:?}"))
    };
    let pistol = find("Props", "Pistol");
    assert_eq!(
        pistol["evidence"], "carrying a pistol.",
        "matched phrase evidence"
    );
    assert_eq!(pistol["confidence"], "Likely", "plain-language confidence");
    assert_eq!(pistol["matchKind"], "none");
    find("Cast", "Meera");
    find("Cast", "Arjun");
    find("Location / Set", "Police Station");
    find("Props", "Red Folder");
    find("Special Effects", "Rain");
    find("Wardrobe", "Coat");

    // Accept → production data linked to a catalog item.
    let acc = env.ok("breakdown.accept", json!({ "id": pistol["id"] }));
    assert_eq!(acc["state"], "Confirmed");
    assert!(acc["catalogItemId"].is_string());
    // Edit before accepting: category and name.
    let coat = find("Wardrobe", "Coat");
    let acc2 = env.ok(
        "breakdown.accept",
        json!({ "id": coat["id"], "name": "Wet grey coat" }),
    );
    assert_eq!(acc2["name"], "Wet grey coat");
    // Accepting a Location / Set suggestion creates the location record too.
    let loc = find("Location / Set", "Police Station");
    env.ok("breakdown.accept", json!({ "id": loc["id"] }));
    let locs = env.ok("locations.list", json!({}));
    assert_eq!(locs[0]["name"], "Police Station");
    assert_eq!(locs[0]["status"], "Idea");
    assert_eq!(locs[0]["sceneCount"], 1);
    // Reject → gone, creates nothing, never re-suggested for this scene + source.
    let rain = find("Special Effects", "Rain");
    let items_before = count(&env, "SELECT count(*) FROM catalog_item");
    env.ok("breakdown.reject", json!({ "ids": [rain["id"]] }));
    assert_eq!(
        count(&env, "SELECT count(*) FROM catalog_item"),
        items_before,
        "FSD-BREAKDOWN-009 reject creates no data"
    );
    let again = env.ok("breakdown.suggest", json!({ "sceneId": scenes[0] }));
    assert_eq!(
        again["added"], 0,
        "rejected and accepted items are not suggested again"
    );
    let d = detail(&env, &scenes[0]);
    assert!(!suggestions(&d).iter().any(|s| s["name"] == "Rain"));
    assert!(!suggestions(&d).iter().any(|s| s["name"] == "Pistol"));
    // Rejecting a confirmed element is refused.
    assert_eq!(
        env.err("breakdown.reject", json!({ "ids": [acc["id"]] })),
        "conflict.state"
    );
    // Viewer can't run suggestions.
    let viewer = env.actor_with_role(Role::Viewer);
    assert_eq!(
        env.call_as(
            &viewer,
            "breakdown.suggest",
            json!({ "sceneId": scenes[1] })
        )
        .unwrap_err()
        .code
        .0,
        "permission.denied"
    );
}

#[test]
fn fsd_brk_011_batch_accept_asks_for_ambiguous_matches() {
    let (env, _, _, scenes) = setup();
    env.ok(
        "catalog.create",
        json!({ "category": "Props", "name": "Old Red Folder" }),
    );
    env.ok(
        "catalog.create",
        json!({ "category": "Cast", "name": "Meera" }),
    );
    env.ok("breakdown.suggest", json!({ "sceneId": scenes[0] }));
    let sug = suggestions(&detail(&env, &scenes[0]));
    let meera = sug.iter().find(|s| s["name"] == "Meera").unwrap();
    assert_eq!(meera["matchKind"], "exact", "obvious match offered first");
    let folder = sug.iter().find(|s| s["name"] == "Red Folder").unwrap();
    assert_eq!(folder["matchKind"], "ambiguous");
    let ids: Vec<Value> = sug.iter().map(|s| s["id"].clone()).collect();
    let r = env.ok("breakdown.accept_many", json!({ "ids": ids }));
    assert_eq!(r["needsChoice"], json!([folder["id"]]));
    assert_eq!(r["accepted"].as_i64().unwrap(), ids.len() as i64 - 1);
    let d = detail(&env, &scenes[0]);
    let meera_el = confirmed(&d)
        .into_iter()
        .find(|e| e["name"] == "Meera")
        .unwrap();
    assert_eq!(
        count(
            &env,
            "SELECT count(*) FROM catalog_item WHERE category='Cast' AND name='Meera'"
        ),
        1
    );
    assert!(meera_el["catalogItemId"].is_string());
    assert_eq!(
        suggestions(&d).len(),
        1,
        "the ambiguous one is still pending"
    );
    // One undo step for the batch.
    env.undo();
    assert_eq!(suggestions(&detail(&env, &scenes[0])).len(), ids.len());
}

#[test]
fn known_characters_in_action_and_crowds_are_suggested() {
    let (env, _, _, scenes) = setup();
    env.ok("breakdown.suggest", json!({ "sceneId": scenes[2] }));
    let sug = suggestions(&detail(&env, &scenes[2]));
    assert!(
        sug.iter()
            .any(|s| s["category"] == "Cast" && s["name"] == "Ravi")
    );
    assert!(
        sug.iter()
            .any(|s| s["category"] == "Extras / Background" && s["name"] == "Crowd")
    );
    assert!(
        sug.iter()
            .any(|s| s["category"] == "Extras / Background" && s["name"] == "Passengers")
    );
    // Arjun has cues in scenes 1 and 2 and is known everywhere in the source.
    env.ok("breakdown.suggest", json!({ "sceneId": scenes[1] }));
    let sug = suggestions(&detail(&env, &scenes[1]));
    assert!(
        sug.iter()
            .any(|s| s["category"] == "Cast" && s["name"] == "Arjun")
    );
}

// ---------------------------------------------------- reconciliation

#[test]
fn fsd_brk_016_017_018_production_source_update_never_deletes_breakdown_data() {
    let (env, sp, d6, scenes) = setup();
    add(&env, &scenes[0], "Props", "Pistol");
    add(&env, &scenes[0], "Cast", "Arjun");
    let removed_el = add(&env, &scenes[1], "Props", "Lamp");
    add(&env, &scenes[2], "Extras / Background", "Passengers");
    let total = count(
        &env,
        "SELECT count(*) FROM breakdown_element WHERE deleted_at IS NULL",
    );
    env.ok(
        "breakdown.set_complete",
        json!({ "sceneId": scenes[0], "complete": true }),
    );

    // Draft 7: scene L13 removed, L14 text changed, new L15, L12 unchanged.
    let changed_14: &[(&str, &str)] = &[
        ("action", "Ravi paces by the counter with a suitcase."),
        ("character", "RAVI"),
        ("dialogue", "He's late."),
    ];
    let (d7, s7) = sp.draft(
        &env,
        "Draft 7 — Director Rewrite",
        "Draft",
        7,
        &[
            ("L12", "INT. POLICE STATION — NIGHT", SCENE_12),
            ("L14", "INT. TICKET OFFICE - DAY", changed_14),
            (
                "L15",
                "EXT. LEVEL CROSSING - DAWN",
                &[("action", "A train thunders past.")],
            ),
        ],
    );
    let src = env.ok("production.source", json!({}));
    assert_eq!(src["draft"]["id"], d6, "never silently replaced");
    assert_eq!(src["newer"]["id"], d7);

    let before = screenplay_snapshot(&env);
    let p = env.ok("production.preview_update", json!({ "draftId": d7 }));
    assert_eq!(p["added"], 1);
    assert_eq!(p["removed"], 1);
    assert_eq!(p["textChanged"], 1);
    assert_eq!(p["headingChanged"], 0);
    assert_eq!(p["unchanged"], 1);
    assert_eq!(p["elementsKept"], total);
    let removed = p["changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["status"] == "removed")
        .unwrap();
    assert_eq!(removed["lineageId"], "L13");
    assert_eq!(removed["elementCount"], 1);
    // Preview changes nothing.
    assert_eq!(env.ok("production.source", json!({}))["draft"]["id"], d6);

    let r = env.ok("production.apply_update", json!({ "draftId": d7 }));
    assert_eq!(r["source"]["draft"]["id"], d7);
    assert_eq!(r["addedScenes"], 1);
    assert_eq!(r["historicalScenes"], 1);
    assert_eq!(
        screenplay_snapshot(&env),
        before,
        "the screenplay is never changed"
    );
    assert_eq!(
        count(
            &env,
            "SELECT count(*) FROM breakdown_element WHERE deleted_at IS NULL"
        ),
        total,
        "nothing deleted"
    );

    let view = env.ok("breakdown.scenes", json!({}));
    let rows = view["scenes"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    // Breakdown follows stable scene identity to the new draft's scene.
    assert_eq!(rows[0]["sceneId"], s7[0]);
    assert_eq!(rows[0]["confirmedCount"], 2);
    assert_eq!(rows[0]["complete"], true);
    assert_eq!(rows[0]["needsReview"], false);
    // Changed text → Needs Review with a stale message (FSD §125).
    assert_eq!(rows[1]["needsReview"], true);
    assert_eq!(rows[1]["textChanged"], true);
    assert_eq!(
        rows[1]["changeMessage"],
        "Scene 2 changed in Draft 7 — Director Rewrite. Review Breakdown."
    );
    // New scene → Needs Breakdown.
    assert_eq!(rows[2]["needsBreakdown"], true);
    // Removed scene keeps its breakdown as history.
    let hist = view["historical"].as_array().unwrap();
    assert_eq!(hist.len(), 1);
    assert_eq!(hist[0]["elements"][0]["id"], removed_el["id"]);
    assert_eq!(hist[0]["draftName"], "Draft 6 — Shooting Draft");
    // The user may archive historical rows.
    env.ok(
        "breakdown.set_archived",
        json!({ "id": removed_el["id"], "archived": true }),
    );
    assert_eq!(
        env.ok("breakdown.scenes", json!({}))["historical"][0]["elements"][0]["archived"],
        true
    );

    // Scene changes dialog: added candidate from the new text.
    let ch = env.ok("breakdown.scene_changes", json!({ "sceneId": s7[1] }));
    assert!(
        ch["addedCandidates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["name"] == "Suitcase")
    );
    assert!(
        ch["removedCandidates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["name"] == "Crowd")
    );
    assert_eq!(ch["elementsToReview"][0]["name"], "Passengers");
    let applied = env.ok(
        "breakdown.apply_suggested_update",
        json!({ "sceneId": s7[1] }),
    );
    assert!(applied["added"].as_i64().unwrap() >= 1);
    let row = env.ok("breakdown.scenes", json!({}))["scenes"][1].clone();
    assert_eq!(row["needsReview"], false);
    assert!(row["suggestedCount"].as_i64().unwrap() >= 1);
    assert_eq!(row["confirmedCount"], 1, "existing elements are kept");

    let ov = env.ok("production.overview", json!({}));
    assert_eq!(
        ov["historicalElements"], 0,
        "archived historical rows are not counted"
    );
    assert_eq!(ov["needsBreakdown"][0]["sceneId"], s7[2]);

    // Undo the update itself (several steps back): the source returns to Draft 6.
    for _ in 0..3 {
        env.undo();
    }
    assert_eq!(env.ok("production.source", json!({}))["draft"]["id"], d6);
    assert_eq!(
        env.ok("breakdown.scenes", json!({}))["scenes"][0]["sceneId"],
        scenes[0]
    );
}

#[test]
fn fsd_brk_019_heading_change_and_in_place_edits_flag_review() {
    let (env, _, _, scenes) = setup();
    add(&env, &scenes[1], "Props", "Lamp");
    // Unplanned scenes are not flagged; planned scenes are.
    edit_element(&env, &scenes[2], 1, "Ravi sleeps.");
    fixture(&env, |c| {
        c.execute("UPDATE screenplay_scene SET heading='EXT. OLD RAILWAY STATION — DAWN', updated_at=?1 WHERE id=?2", params![now_ms(), scenes[1]])
            .unwrap();
    });
    let rows = env.ok("breakdown.scenes", json!({}))["scenes"].clone();
    assert_eq!(rows[2]["needsReview"], false);
    assert_eq!(rows[1]["needsReview"], true);
    assert_eq!(rows[1]["headingChanged"], true);
    assert_eq!(rows[1]["textChanged"], false);
    assert!(
        rows[1]["changeMessage"]
            .as_str()
            .unwrap()
            .contains("heading changed")
    );
    // Existing associations stay until reviewed.
    assert_eq!(rows[1]["confirmedCount"], 1);
    let d = env.ok("breakdown.mark_reviewed", json!({ "sceneId": scenes[1] }));
    assert_eq!(d["needsReview"], false);
    assert_eq!(confirmed(&d).len(), 1);
}

// --------------------------------------------------------------- catalog

#[test]
fn catalog_crud_archive_delete_and_search() {
    let (env, _, _, scenes) = setup();
    let item = env.ok(
        "catalog.create",
        json!({ "category": "Props", "name": "Red Folder", "status": "Searching", "description": "Worn leather folder", "notes": "Must look old" }),
    );
    assert_eq!(item["status"], "Searching");
    assert_eq!(
        env.err("catalog.create", json!({ "category": "Props", "name": "" })),
        "validation.required"
    );
    assert_eq!(
        env.err(
            "catalog.create",
            json!({ "category": "Props", "name": "X", "status": "Bought" })
        ),
        "validation.invalid_input"
    );
    let hits = env.ok("search.query", json!({ "text": "leather folder" }));
    assert_eq!(hits[0]["entityId"], item["id"]);
    assert_eq!(hits[0]["nav"]["sub"], "catalog");

    let el = env.ok(
        "breakdown.add_element",
        json!({ "sceneId": scenes[0], "category": "Props", "name": "Red Folder", "catalog": { "mode": "existing", "catalogItemId": item["id"] } }),
    );
    // Stale revision is refused.
    assert_eq!(
        env.err(
            "catalog.update",
            json!({ "id": item["id"], "notes": "x", "expectedRev": 0 })
        ),
        "conflict.stale"
    );
    // Clear an optional field with empty text.
    let upd = env.ok("catalog.update", json!({ "id": item["id"], "notes": "" }));
    assert!(upd["notes"].is_null());

    // Archive: not offered for new selections, associations stay readable.
    env.ok(
        "catalog.set_archived",
        json!({ "id": item["id"], "archived": true }),
    );
    assert!(
        env.ok("catalog.list", json!({}))
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        env.ok("catalog.list", json!({ "includeArchived": true }))[0]["archived"],
        true
    );
    assert!(
        env.ok(
            "catalog.find_matches",
            json!({ "category": "Props", "name": "Red Folder" })
        )
        .as_array()
        .unwrap()
        .is_empty()
    );
    let d = detail(&env, &scenes[0]);
    assert_eq!(confirmed(&d)[0]["catalogArchived"], true);
    assert_eq!(
        env.err(
            "breakdown.add_element",
            json!({ "sceneId": scenes[1], "category": "Props", "name": "Red Folder", "catalog": { "mode": "existing", "catalogItemId": item["id"] } })
        ),
        "validation.catalog_item"
    );
    env.ok(
        "catalog.set_archived",
        json!({ "id": item["id"], "archived": false }),
    );

    // Delete is recoverable; affected entries keep a visible removed status.
    env.ok("catalog.delete", json!({ "id": item["id"] }));
    let d = detail(&env, &scenes[0]);
    assert_eq!(confirmed(&d)[0]["catalogRemoved"], true);
    assert_eq!(confirmed(&d)[0]["id"], el["id"]);
    assert!(
        env.ok("search.query", json!({ "text": "leather" }))
            .as_array()
            .unwrap()
            .is_empty()
    );
    let trash = env.ok("trash.list", json!({}));
    env.ok("trash.restore", json!({ "id": trash[0]["id"] }));
    assert_eq!(
        confirmed(&detail(&env, &scenes[0]))[0]["catalogRemoved"],
        false
    );
    // Permanent removal never leaves a dangling reference.
    env.ok("catalog.delete", json!({ "id": item["id"] }));
    let trash = env.ok("trash.list", json!({}));
    env.ok("trash.purge", json!({ "id": trash[0]["id"] }));
    let d = detail(&env, &scenes[0]);
    let e = &confirmed(&d)[0];
    assert!(e["catalogItemId"].is_null());
    assert_eq!(e["catalogRemoved"], true);
    assert_eq!(e["name"], "Red Folder");
}

#[test]
fn catalog_replace_in_selected_scenes_keeps_the_old_item() {
    let (env, _, _, scenes) = setup();
    let a = add(&env, &scenes[0], "Vehicles", "Jeep");
    add(&env, &scenes[1], "Vehicles", "Jeep");
    let b = env.ok(
        "catalog.create",
        json!({ "category": "Vehicles", "name": "Police Car" }),
    );
    let r = env.ok(
        "catalog.replace_in_scenes",
        json!({ "fromId": a["catalogItemId"], "toId": b["id"], "sceneIds": [scenes[0]] }),
    );
    assert_eq!(r["scenesUpdated"], 1);
    assert_eq!(
        confirmed(&detail(&env, &scenes[0]))[0]["name"],
        "Police Car"
    );
    assert_eq!(confirmed(&detail(&env, &scenes[1]))[0]["name"], "Jeep");
    assert_eq!(
        env.ok("catalog.get", json!({ "id": a["catalogItemId"] }))["usedInLabel"],
        "Scene 2"
    );
    let other = env.ok(
        "catalog.create",
        json!({ "category": "Props", "name": "Key" }),
    );
    assert_eq!(
        env.err(
            "catalog.replace_in_scenes",
            json!({ "fromId": a["catalogItemId"], "toId": other["id"], "sceneIds": [scenes[1]] })
        ),
        "validation.category"
    );
}

// ------------------------------------------------------------ locations

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0\0\x10\0\0\0\x10\x08\x02\0\0\0";

#[test]
fn fsd_loc_001_location_replacement_never_changes_screenplay_text() {
    let (env, sp, _, _) = setup();
    let (d8, s8) = sp.draft(
        &env,
        "Hospital draft",
        "Draft",
        8,
        &[
            (
                "H1",
                "EXT. HOSPITAL - DAY",
                &[("action", "The HOSPITAL gates.")],
            ),
            ("H2", "INT. HOSPITAL - NIGHT", &[]),
            ("H3", "INT. HOSPITAL - DAY", &[]),
        ],
    );
    env.ok("production.apply_update", json!({ "draftId": d8 }));
    for s in &s8 {
        add(&env, s, "Location / Set", "Hospital");
    }
    let before = screenplay_snapshot(&env);
    let locs = env.ok("locations.list", json!({}));
    assert_eq!(locs.as_array().unwrap().len(), 1);
    let hospital = locs[0].clone();
    assert_eq!(
        hospital["sceneCount"], 3,
        "scene usage derived from the breakdown"
    );
    env.ok(
        "locations.set_status",
        json!({ "id": hospital["id"], "status": "Rejected" }),
    );
    let clinic = env.ok(
        "locations.create",
        json!({ "name": "Town Clinic", "address": "12 Market Rd", "status": "Confirmed" }),
    );
    assert_eq!(
        clinic["sceneCount"], 0,
        "confirming does not auto-assign scenes"
    );
    let r = env.ok(
        "locations.replace",
        json!({ "locationId": hospital["id"], "replacementId": clinic["id"], "sceneIds": [s8[0], s8[1]] }),
    );
    assert_eq!(r["scenesUpdated"], 2);
    assert_eq!(
        screenplay_snapshot(&env),
        before,
        "screenplay text untouched"
    );
    let h = env.ok("locations.get", json!({ "id": hospital["id"] }));
    assert_eq!(h["sceneCount"], 1);
    assert_eq!(h["replacement"]["name"], "Town Clinic");
    let c = env.ok("locations.get", json!({ "id": clinic["id"] }));
    let nums: Vec<&str> = c["scenes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["number"].as_str().unwrap())
        .collect();
    assert_eq!(nums, vec!["1", "2"]);
    assert_eq!(confirmed(&detail(&env, &s8[0]))[0]["name"], "Town Clinic");
    // Script still says HOSPITAL.
    assert_eq!(detail(&env, &s8[0])["heading"], "EXT. HOSPITAL - DAY");
    assert_eq!(env.err("locations.replace", json!({ "locationId": clinic["id"], "replacementId": clinic["id"], "sceneIds": [s8[0]] })), "validation.invalid_input");
}

#[test]
fn locations_fields_notes_photos_and_trash() {
    let (env, _, _, _) = setup();
    assert_eq!(
        env.err("locations.create", json!({ "name": " " })),
        "validation.required"
    );
    let loc = env.ok("locations.create", json!({ "name": "Old Railway Station" }));
    assert_eq!(loc["status"], "Idea", "only the name is required");
    let loc = env.ok(
        "locations.update",
        json!({ "id": loc["id"], "address": "Platform road", "notes": { "parking": "Behind the goods shed", "noise": "Trains every 20 min", "power": "Generator needed" } }),
    );
    assert_eq!(loc["notes"]["parking"], "Behind the goods shed");
    assert!(loc["notes"]["access"].is_null());
    let hits = env.ok("search.query", json!({ "text": "goods shed" }));
    assert_eq!(hits[0]["entityId"], loc["id"]);

    let p1 = env.write_file("station.png", PNG);
    let p2 = env.write_file("platform.png", PNG);
    let loc = env.ok(
        "locations.add_photos",
        json!({ "id": loc["id"], "paths": [p1, p2] }),
    );
    let photos = loc["photos"].as_array().unwrap();
    assert_eq!(photos.len(), 2);
    assert_eq!(photos[0]["asset"]["width"], 16);
    assert_eq!(
        photos[0]["asset"]["storageMode"], "managed",
        "available offline"
    );
    let txt = env.write_file("notes.txt", b"hello");
    assert_eq!(
        env.err(
            "locations.add_photos",
            json!({ "id": loc["id"], "paths": [txt] })
        ),
        "validation.invalid_input"
    );
    // Reorder: the second becomes the thumbnail.
    env.ok(
        "locations.move_photo",
        json!({ "photoId": photos[1]["id"], "index": 0 }),
    );
    let got = env.ok("locations.get", json!({ "id": loc["id"] }));
    assert_eq!(got["photos"][0]["id"], photos[1]["id"]);
    // Remove → recoverable → restore at its place.
    env.ok("locations.remove_photo", json!({ "id": photos[0]["id"] }));
    assert_eq!(
        env.ok("locations.get", json!({ "id": loc["id"] }))["photos"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let trash = env.ok("trash.list", json!({}));
    env.ok("trash.restore", json!({ "id": trash[0]["id"] }));
    assert_eq!(
        env.ok("locations.get", json!({ "id": loc["id"] }))["photos"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    // Delete location → purge removes photo files.
    let path = got["photos"][0]["asset"]["path"]
        .as_str()
        .unwrap()
        .to_string();
    env.ok("locations.delete", json!({ "id": loc["id"] }));
    assert!(
        env.ok("locations.list", json!({}))
            .as_array()
            .unwrap()
            .is_empty()
    );
    let trash = env.ok("trash.list", json!({}));
    env.ok("trash.purge", json!({ "id": trash[0]["id"] }));
    assert!(!std::path::Path::new(&path).exists());
}

// ------------------------------------------------------------- cast & crew

#[test]
fn fsd_cast_001_cast_connects_actors_to_characters_and_scenes() {
    let (env, _, _, scenes) = setup();
    let dir = env.ok("cast.list", json!({}));
    let names: Vec<&str> = dir["characters"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        vec!["MEERA", "ARJUN", "RAVI"],
        "characters from cues in script order"
    );
    let ov = env.ok("production.overview", json!({}));
    assert_eq!(ov["unresolvedCharacters"].as_array().unwrap().len(), 3);

    assert_eq!(
        env.err("cast.create", json!({ "personName": "Karthik Menon" })),
        "validation.required",
        "character association required"
    );
    let k = env.ok(
        "cast.create",
        json!({ "personName": "Karthik Menon", "characterName": "arjun", "contact": "98xxx 10001", "availabilityNotes": "Free Jun 9–20" }),
    );
    assert_eq!(k["characterName"], "ARJUN");
    assert_eq!(k["isPrimary"], true);
    let nums: Vec<&str> = k["scenes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["number"].as_str().unwrap())
        .collect();
    assert_eq!(nums, vec!["1", "2"], "scenes via character cues");
    // An alternate by default when a primary exists; promoting demotes the other.
    let alt = env.ok(
        "cast.create",
        json!({ "personName": "Double", "characterName": "ARJUN" }),
    );
    assert_eq!(alt["isPrimary"], false);
    env.ok(
        "cast.assign",
        json!({ "id": alt["id"], "characterName": "ARJUN", "primary": true }),
    );
    let dir = env.ok("cast.list", json!({}));
    let arjun = dir["characters"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "ARJUN")
        .unwrap();
    assert_eq!(arjun["primaryCastId"], alt["id"]);
    assert_eq!(arjun["castIds"].as_array().unwrap().len(), 2);
    // A cast breakdown element also places the character in a scene.
    add(&env, &scenes[2], "Cast", "Arjun");
    let k2 = env.ok("cast.list", json!({}))["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["id"] == k["id"])
        .unwrap()
        .clone();
    assert_eq!(k2["scenes"].as_array().unwrap().len(), 3);
    // New character that isn't in the script yet.
    env.ok(
        "cast.create",
        json!({ "personName": "Sam", "characterName": "The Station Master" }),
    );
    let ov = env.ok("production.overview", json!({}));
    let unresolved: Vec<&str> = ov["unresolvedCharacters"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(unresolved, vec!["MEERA", "RAVI"]);
    let hits = env.ok("search.query", json!({ "text": "Karthik" }));
    assert_eq!(hits[0]["nav"]["sub"], "cast-crew");
    // Delete is recoverable.
    env.ok("cast.delete", json!({ "id": k["id"] }));
    assert_eq!(
        env.ok("trash.list", json!({}))[0]["objectType"],
        "cast_member"
    );
}

#[test]
fn crew_directory_requires_role() {
    let (env, _, _, _) = setup();
    assert_eq!(
        env.err("crew.create", json!({ "personName": "Anu", "role": "" })),
        "validation.required"
    );
    let c = env.ok(
        "crew.create",
        json!({ "personName": "Anu", "role": "Director of Photography", "department": "Camera" }),
    );
    env.ok(
        "crew.create",
        json!({ "personName": "Bala", "role": "Sound Recordist", "department": "Sound" }),
    );
    let list = env.ok("crew.list", json!({}));
    assert_eq!(list.as_array().unwrap().len(), 2);
    assert_eq!(list[0]["department"], "Camera");
    let c = env.ok(
        "crew.update",
        json!({ "id": c["id"], "contact": "anu@example.com" }),
    );
    assert_eq!(c["contact"], "anu@example.com");
    assert_eq!(
        env.ok("search.query", json!({ "text": "photography" }))[0]["entityId"],
        c["id"]
    );
    env.ok(
        "crew.set_archived",
        json!({ "id": c["id"], "archived": true }),
    );
    assert_eq!(env.ok("crew.list", json!({})).as_array().unwrap().len(), 1);
    env.ok("crew.delete", json!({ "id": c["id"] }));
    let viewer = env.actor_with_role(Role::Viewer);
    assert!(env.call_as(&viewer, "crew.list", json!({})).is_ok());
    assert_eq!(
        env.call_as(
            &viewer,
            "crew.create",
            json!({ "personName": "X", "role": "Grip" })
        )
        .unwrap_err()
        .code
        .0,
        "permission.denied"
    );
}

// ------------------------------------------------------------ overview

#[test]
fn fsd_108_overview_is_derived_and_breakdown_complete_is_manual() {
    let (env, _, _, scenes) = setup();
    let ov = env.ok("production.overview", json!({}));
    assert_eq!(ov["sceneCount"], 3);
    assert_eq!(ov["completeCount"], 0);
    assert_eq!(ov["source"]["draft"]["status"], "Locked");
    env.ok("locations.create", json!({ "name": "Ticket office" }));
    env.ok(
        "locations.create",
        json!({ "name": "Yard", "status": "Confirmed" }),
    );
    let d = env.ok(
        "breakdown.set_complete",
        json!({ "sceneId": scenes[0], "complete": true }),
    );
    assert_eq!(d["complete"], true);
    let ov = env.ok("production.overview", json!({}));
    assert_eq!(ov["completeCount"], 1);
    assert_eq!(ov["locationCount"], 2);
    assert_eq!(ov["unresolvedLocations"], 1);
    let d = env.ok(
        "breakdown.set_complete",
        json!({ "sceneId": scenes[0], "complete": false }),
    );
    assert_eq!(d["complete"], false, "complete scenes can be reopened");
}

#[test]
fn production_data_persists_across_restart() {
    let (env, _, d6, scenes) = setup();
    let el = add(&env, &scenes[0], "Props", "Pistol");
    env.ok("locations.create", json!({ "name": "Yard" }));
    let path = env.project_path();
    let env = env.restart();
    env.reopen_project(&path);
    assert_eq!(env.ok("production.source", json!({}))["draft"]["id"], d6);
    assert_eq!(confirmed(&detail(&env, &scenes[0]))[0]["id"], el["id"]);
    assert_eq!(
        env.ok("locations.list", json!({}))
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        env.ok("catalog.list", json!({})).as_array().unwrap().len(),
        1
    );
}

// ------------------------------------------------- additional acceptance

#[test]
fn fsd_breakdown_006_categories_follow_the_defined_order() {
    let (env, _, _, scenes) = setup();
    add(&env, &scenes[0], "Animals", "Stray Dog");
    add(&env, &scenes[0], "Props", "Pistol");
    add(&env, &scenes[0], "Cast", "Arjun");
    add(&env, &scenes[0], "Extras / Background", "Constables");
    let d = detail(&env, &scenes[0]);
    let cats: Vec<&str> = d["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["category"].as_str().unwrap())
        .collect();
    assert_eq!(
        cats,
        vec!["Cast", "Extras / Background", "Props", "Animals"],
        "BreakdownCategory order, empty categories hidden"
    );
}

#[test]
fn fsd_breakdown_010_edit_suggestion_category_and_name_before_accepting() {
    let (env, _, _, scenes) = setup();
    env.ok("breakdown.suggest", json!({ "sceneId": scenes[0] }));
    let sug = suggestions(&detail(&env, &scenes[0]));
    let coat = sug
        .iter()
        .find(|s| s["category"] == "Wardrobe" && s["name"] == "Coat")
        .unwrap();
    let acc = env.ok(
        "breakdown.accept",
        json!({ "id": coat["id"], "category": "Props", "name": "Umbrella" }),
    );
    assert_eq!(acc["state"], "Confirmed");
    assert_eq!(acc["category"], "Props");
    assert_eq!(acc["name"], "Umbrella");
    assert_eq!(
        count(
            &env,
            "SELECT count(*) FROM catalog_item WHERE category='Props' AND name='Umbrella'"
        ),
        1
    );
    assert_eq!(
        count(
            &env,
            "SELECT count(*) FROM catalog_item WHERE category='Wardrobe'"
        ),
        0
    );
    // An invalid category is refused and the suggestion stays pending.
    let pistol = sug.iter().find(|s| s["name"] == "Pistol").unwrap();
    assert_eq!(
        env.err(
            "breakdown.accept",
            json!({ "id": pistol["id"], "category": "Snacks" })
        ),
        "validation.category"
    );
    assert!(
        suggestions(&detail(&env, &scenes[0]))
            .iter()
            .any(|s| s["id"] == pistol["id"])
    );
}

#[test]
fn fsd_prod_004_007_location_status_flow_and_scene_usage() {
    let (env, _, _, scenes) = setup();
    add(&env, &scenes[1], "Location / Set", "Old Railway Station");
    let locs = env.ok("locations.list", json!({}));
    let loc = locs[0].clone();
    assert_eq!(loc["name"], "Old Railway Station");
    let nums: Vec<&str> = loc["scenes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["number"].as_str().unwrap())
        .collect();
    assert_eq!(nums, vec!["2"], "FSD-LOC-001 location displays its scenes");
    for status in ["Shortlisted", "Confirmed"] {
        let l = env.ok(
            "locations.set_status",
            json!({ "id": loc["id"], "status": status }),
        );
        assert_eq!(l["status"], status);
    }
    assert_eq!(
        env.ok("locations.list", json!({ "status": "Confirmed" }))
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(
        env.ok("locations.list", json!({ "status": "Idea" }))
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        env.err(
            "locations.set_status",
            json!({ "id": loc["id"], "status": "Booked" })
        ),
        "validation.invalid_input"
    );
    let commenter = env.actor_with_role(Role::Commenter);
    assert_eq!(
        env.call_as(
            &commenter,
            "locations.set_status",
            json!({ "id": loc["id"], "status": "Idea" })
        )
        .unwrap_err()
        .code
        .0,
        "permission.denied"
    );
    env.undo();
    assert_eq!(
        env.ok("locations.get", json!({ "id": loc["id"] }))["status"],
        "Shortlisted"
    );
}

#[test]
fn fsd_prod_008_010_cast_availability_is_recorded_as_notes() {
    let (env, _, _, _) = setup();
    let k = env.ok(
        "cast.create",
        json!({ "personName": "Karthik Menon", "characterName": "ARJUN" }),
    );
    let k2 = env.ok(
        "cast.update",
        json!({ "id": k["id"], "availabilityNotes": "Unavailable Jun 12–14" }),
    );
    assert_eq!(k2["availabilityNotes"], "Unavailable Jun 12–14");
    let commenter = env.actor_with_role(Role::Commenter);
    assert_eq!(
        env.call_as(
            &commenter,
            "cast.update",
            json!({ "id": k["id"], "notes": "x" })
        )
        .unwrap_err()
        .code
        .0,
        "permission.denied"
    );
    env.undo();
    let member = |env: &TestEnv| {
        env.ok("cast.list", json!({}))["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["id"] == k["id"])
            .cloned()
    };
    assert!(member(&env).unwrap()["availabilityNotes"].is_null());
    env.redo();
    // Delete → restore keeps identity and the character association.
    env.ok("cast.delete", json!({ "id": k["id"] }));
    assert!(member(&env).is_none());
    let trash = env.ok("trash.list", json!({}));
    env.ok("trash.restore", json!({ "id": trash[0]["id"] }));
    let m = member(&env).unwrap();
    assert_eq!(m["characterName"], "ARJUN");
    assert_eq!(m["availabilityNotes"], "Unavailable Jun 12–14");
}

#[test]
fn catalog_and_locations_are_read_only_for_viewers() {
    let (env, _, _, _) = setup();
    let item = env.ok(
        "catalog.create",
        json!({ "category": "Props", "name": "Lantern" }),
    );
    let viewer = env.actor_with_role(Role::Viewer);
    assert!(env.call_as(&viewer, "catalog.list", json!({})).is_ok());
    assert!(env.call_as(&viewer, "locations.list", json!({})).is_ok());
    assert!(
        env.call_as(&viewer, "production.overview", json!({}))
            .is_ok()
    );
    for (op, args) in [
        (
            "catalog.create",
            json!({ "category": "Props", "name": "Rope" }),
        ),
        (
            "catalog.update",
            json!({ "id": item["id"], "status": "Confirmed" }),
        ),
        (
            "catalog.set_archived",
            json!({ "id": item["id"], "archived": true }),
        ),
        ("catalog.delete", json!({ "id": item["id"] })),
        ("locations.create", json!({ "name": "Yard" })),
    ] {
        assert_eq!(
            env.call_as(&viewer, op, args).unwrap_err().code.0,
            "permission.denied",
            "{op}"
        );
    }
}
