//! Visual planning: Moodboards, Storyboards, Shot Lists (FSD §31–34, §55, §101–103).
//! Tests go through the public operation registry, like the UI does. Screenplay
//! scenes are seeded directly as fixtures (the screenplay module owns their ops).

use openframe_application::MutationMeta;
use openframe_domain::{Capability, Role, new_id, now_ms};
use openframe_test_support::TestEnv;
use rusqlite::{Connection, params};
use serde_json::{Value, json};

/// 1×1 transparent PNG.
const PNG_B64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

fn png_bytes() -> Vec<u8> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(PNG_B64)
        .unwrap()
}

// ------------------------------------------------------------------ fixtures

#[derive(Clone, Debug)]
struct Scene {
    id: String,
    lineage: String,
}

#[derive(Clone, Debug)]
struct Script {
    screenplay: String,
    draft: String,
    scenes: Vec<Scene>,
}

fn fixture<R>(env: &TestEnv, f: impl FnOnce(&Connection) -> R) -> R {
    let s = env.core.project().unwrap();
    s.store
        .mutate(
            &env.actor(),
            MutationMeta::new("test.fixture", "Script fixture", Capability::Edit)
                .not_undoable()
                .quiet(),
            |tx| Ok(f(tx.conn())),
        )
        .unwrap()
}

fn seed_script(env: &TestEnv, scenes: &[(&str, &str)]) -> Script {
    fixture(env, |c| {
        let now = now_ms();
        let sp = new_id();
        let draft = new_id();
        c.execute(
            "INSERT INTO screenplay(id, title, current_draft_id, created_at, updated_at) VALUES (?1, 'Black Rain', ?2, ?3, ?3)",
            params![sp, draft, now],
        )
        .unwrap();
        c.execute(
            "INSERT INTO screenplay_draft(id, screenplay_id, name, created_at, updated_at) VALUES (?1, ?2, 'First Draft', ?3, ?3)",
            params![draft, sp, now],
        )
        .unwrap();
        let mut out = Vec::new();
        for (i, (heading, action)) in scenes.iter().enumerate() {
            let id = new_id();
            let lineage = new_id();
            c.execute(
                "INSERT INTO screenplay_scene(id, draft_id, lineage_id, position, heading, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                params![id, draft, lineage, i as i64 + 1, heading, now],
            )
            .unwrap();
            for (p, (t, text)) in [
                ("scene_heading", *heading),
                ("action", *action),
                ("character", "MEERA (V.O.)"),
                ("dialogue", "Who's there?"),
            ]
            .iter()
            .enumerate()
            {
                c.execute(
                    "INSERT INTO screenplay_element(id, scene_id, position, element_type, text, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                    params![new_id(), id, p as i64 + 1, t, text, now],
                )
                .unwrap();
            }
            out.push(Scene { id, lineage });
        }
        Script {
            screenplay: sp,
            draft,
            scenes: out,
        }
    })
}

/// A new draft copied from `from` (same scene identities, new scene rows).
fn new_draft(env: &TestEnv, from: &Script, make_current: bool) -> Script {
    fixture(env, |c| {
        let now = now_ms();
        let draft = new_id();
        c.execute(
            "INSERT INTO screenplay_draft(id, screenplay_id, name, created_from_draft_id, created_at, updated_at)
             VALUES (?1, ?2, 'Shooting Draft', ?3, ?4, ?4)",
            params![draft, from.screenplay, from.draft, now],
        )
        .unwrap();
        let mut out = Vec::new();
        for s in &from.scenes {
            let id = new_id();
            c.execute(
                "INSERT INTO screenplay_scene(id, draft_id, lineage_id, position, heading, created_at, updated_at)
                 SELECT ?1, ?2, lineage_id, position, heading, ?3, ?3 FROM screenplay_scene WHERE id=?4",
                params![id, draft, now, s.id],
            )
            .unwrap();
            let els: Vec<(i64, String, String)> = {
                let mut st = c.prepare("SELECT position, element_type, text FROM screenplay_element WHERE scene_id=?1").unwrap();
                st.query_map([&s.id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
                    .unwrap()
                    .map(|r| r.unwrap())
                    .collect()
            };
            for (p, t, text) in els {
                c.execute(
                    "INSERT INTO screenplay_element(id, scene_id, position, element_type, text, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                    params![new_id(), id, p, t, text, now],
                )
                .unwrap();
            }
            out.push(Scene {
                id,
                lineage: s.lineage.clone(),
            });
        }
        if make_current {
            c.execute(
                "UPDATE screenplay SET current_draft_id=?1 WHERE id=?2",
                params![draft, from.screenplay],
            )
            .unwrap();
        }
        Script {
            screenplay: from.screenplay.clone(),
            draft,
            scenes: out,
        }
    })
}

fn set_production_source(env: &TestEnv, draft: &str) {
    fixture(env, |c| {
        let now = now_ms();
        c.execute("UPDATE production_source SET active=0", [])
            .unwrap();
        c.execute(
            "INSERT INTO production_source(id, draft_id, active, selected_at, created_at, updated_at) VALUES (?1, ?2, 1, ?3, ?3, ?3)",
            params![new_id(), draft, now],
        )
        .unwrap();
    })
}

fn edit_action(env: &TestEnv, scene_id: &str, text: &str) {
    fixture(env, |c| {
        c.execute(
            "UPDATE screenplay_element SET text=?1, updated_at=?2, rev=rev+1 WHERE scene_id=?3 AND element_type='action'",
            params![text, now_ms() + 1, scene_id],
        )
        .unwrap();
    })
}

fn set_scene_position(env: &TestEnv, scene_id: &str, position: i64) {
    fixture(env, |c| {
        c.execute(
            "UPDATE screenplay_scene SET position=?1, rev=rev+1 WHERE id=?2",
            params![position, scene_id],
        )
        .unwrap();
    })
}

fn remove_scene(env: &TestEnv, scene_id: &str) {
    fixture(env, |c| {
        c.execute(
            "UPDATE screenplay_scene SET deleted_at=?1 WHERE id=?2",
            params![now_ms(), scene_id],
        )
        .unwrap();
    })
}

fn three_scenes(env: &TestEnv) -> Script {
    seed_script(
        env,
        &[
            (
                "EXT. RAILWAY PLATFORM - NIGHT",
                "Rain hammers the empty platform.",
            ),
            (
                "INT. POLICE STATION - NIGHT",
                "Arjun enters, rain on his coat.",
            ),
            ("INT. RAVI'S HOUSE - DAY", "Warm light through the window."),
        ],
    )
}

fn arr(v: &Value) -> &Vec<Value> {
    v.as_array().unwrap()
}

fn labels(v: &Value) -> Vec<String> {
    arr(v)
        .iter()
        .map(|s| s["label"].as_str().unwrap().to_string())
        .collect()
}

fn id(v: &Value) -> String {
    v["id"].as_str().unwrap().to_string()
}

fn add_shot(env: &TestEnv, scene: &Scene, desc: &str) -> String {
    id(&env.ok(
        "shot.create",
        json!({ "sceneId": scene.id, "description": desc }),
    ))
}

fn trash_entry(env: &TestEnv, object_id: &str) -> Value {
    arr(&env.ok("trash.list", json!({})))
        .iter()
        .find(|t| t["objectId"] == object_id)
        .cloned()
        .expect("in Recently Deleted")
}

// ------------------------------------------------------------------ shot lists

#[test]
fn fsd_shot_001_shots_are_ordered_by_drag_and_tied_to_scenes() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let script = three_scenes(&env);
    let sc = &script.scenes[1];
    let a = add_shot(&env, sc, "Arjun enters the station");
    let b = add_shot(&env, sc, "Meera looks up from the desk");
    let c = add_shot(&env, sc, "The red folder lands on the desk");
    let list = env.ok("shot.list", json!({ "sceneId": sc.id }));
    assert_eq!(
        labels(&list),
        ["2A", "2B", "2C"],
        "labels derive from scene number + order"
    );

    // Drag the insert to the top: identity kept, labels regenerated.
    env.ok("shot.reorder", json!({ "id": c, "index": 0 }));
    let list = env.ok("shot.list", json!({ "sceneId": sc.id }));
    assert_eq!(
        arr(&list).iter().map(id).collect::<Vec<_>>(),
        [c.clone(), a.clone(), b.clone()]
    );
    assert_eq!(labels(&list), ["2A", "2B", "2C"]);
    assert_eq!(list[0]["description"], "The red folder lands on the desk");

    // Undo / redo the drag.
    env.undo();
    let list = env.ok("shot.list", json!({ "sceneId": sc.id }));
    assert_eq!(
        arr(&list).iter().map(id).collect::<Vec<_>>(),
        [a.clone(), b.clone(), c.clone()]
    );
    env.redo();
    assert_eq!(env.ok("shot.list", json!({ "sceneId": sc.id }))[0]["id"], c);

    // Shots follow scene identity: moving the scene in the script renumbers labels.
    set_scene_position(&env, &script.scenes[1].id, 10);
    let list = env.ok("shot.list", json!({ "sceneId": sc.id }));
    assert_eq!(labels(&list), ["3A", "3B", "3C"]);
    assert_eq!(
        list[0]["needsReview"], false,
        "a reorder is not a content change (FSD §55)"
    );

    // Other scenes are independent coverage lists.
    let x = add_shot(&env, &script.scenes[0], "Wide of the platform");
    assert_eq!(env.ok("shot.get", json!({ "id": x }))["label"], "1A");
    let all = env.ok("shot.list", json!({}));
    assert_eq!(
        labels(&all),
        ["1A", "3A", "3B", "3C"],
        "whole-project list is grouped in script order"
    );
}

#[test]
fn fsd_prod_018_020_shot_needs_only_a_description_and_fields_are_optional() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let script = three_scenes(&env);
    let sc = &script.scenes[1];
    assert_eq!(
        env.err(
            "shot.create",
            json!({ "sceneId": sc.id, "description": "   " })
        ),
        "validation.required"
    );
    let shot = env.ok(
        "shot.create",
        json!({ "sceneId": sc.id, "description": "Meera looks up" }),
    );
    assert!(
        shot["size"].is_null()
            && shot["lens"].is_null()
            && shot["characters"].as_array().unwrap().is_empty()
    );
    assert_eq!(
        shot["sceneHeading"], "INT. POLICE STATION - NIGHT",
        "heading comes from the screenplay (FSD §34.2)"
    );

    let sid = id(&shot);
    let upd = env.ok(
        "shot.update",
        json!({ "id": sid, "size": "Medium", "movement": "Handheld", "angle": "Eye level", "lens": "35mm",
                "cameraNotes": "follow focus to the folder", "characters": ["MEERA", " meera ", "ARJUN", ""], "soundNote": "rain on window" }),
    );
    assert_eq!(upd["size"], "Medium");
    assert_eq!(upd["characters"], json!(["MEERA", "ARJUN"]));
    // Clearing an optional field.
    let upd = env.ok("shot.update", json!({ "id": sid, "lens": "" }));
    assert!(upd["lens"].is_null());
    assert_eq!(
        env.err("shot.update", json!({ "id": sid, "description": "" })),
        "validation.required"
    );
    assert_eq!(
        env.err("shot.update", json!({ "id": sid, "size": "x".repeat(500) })),
        "validation.invalid_input"
    );
    // Stale edit is refused.
    let rev = upd["rev"].as_i64().unwrap();
    assert_eq!(
        env.err(
            "shot.update",
            json!({ "id": sid, "lens": "50mm", "expectedRev": rev - 1 })
        ),
        "conflict.stale"
    );

    // Suggested characters come from the scene's cues.
    assert_eq!(
        env.ok("visual.scene_characters", json!({ "sceneId": sc.id })),
        json!(["MEERA"])
    );

    // Duplicate lands right after the original.
    let dup = env.ok("shot.duplicate", json!({ "id": sid }));
    assert_eq!(dup["label"], "2B");
    assert_eq!(dup["size"], "Medium");
    assert_ne!(dup["id"], upd["id"]);
}

#[test]
fn shot_can_move_to_another_scene_and_carry_a_reference_image() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let script = three_scenes(&env);
    let s = add_shot(&env, &script.scenes[0], "Insert of the ticket");
    let moved = env.ok(
        "shot.move",
        json!({ "id": s, "sceneId": script.scenes[2].id }),
    );
    assert_eq!(moved["label"], "3A");
    assert!(arr(&env.ok("shot.list", json!({ "sceneId": script.scenes[0].id }))).is_empty());

    let img = env.write_file("ref.png", &png_bytes());
    let with_ref = env.ok("shot.set_reference_image", json!({ "id": s, "path": img }));
    assert_eq!(with_ref["referenceAsset"]["mediaType"], "image/png");
    let txt = env.write_file("notes.txt", b"hello");
    assert_eq!(
        env.err("shot.set_reference_image", json!({ "id": s, "path": txt })),
        "validation.invalid_input"
    );
    let cleared = env.ok("shot.set_reference_image", json!({ "id": s }));
    assert!(cleared["referenceAsset"].is_null());
    env.undo();
    assert!(!env.ok("shot.get", json!({ "id": s }))["referenceAsset"].is_null());
}

// ------------------------------------------------------------------ storyboards

#[test]
fn fsd_prod_014_015_storyboard_for_a_scene_with_panels() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let script = three_scenes(&env);
    let sc = &script.scenes[1];
    let board = env.ok("storyboard.create", json!({ "sceneId": sc.id }));
    assert_eq!(
        board["name"], "INT. POLICE STATION - NIGHT",
        "named from the scene heading (no re-entry)"
    );
    assert_eq!(board["sceneNumber"], "2");
    let bid = id(&board);

    let p1 = env.ok("storyboard.add_panel", json!({ "storyboardId": bid, "visual": "placeholder", "description": "Wide — Arjun enters" }));
    assert_eq!(p1["number"], 1);
    assert_eq!(p1["visualKind"], "placeholder");
    let img = env.write_file("frame.png", &png_bytes());
    let p2 = env.ok(
        "storyboard.add_panel",
        json!({ "storyboardId": bid, "visual": "image", "path": img }),
    );
    assert_eq!(p2["visualKind"], "image");
    assert_eq!(p2["asset"]["available"], true);
    let p3 = env.ok("storyboard.add_panel", json!({ "storyboardId": bid, "visual": "sketch", "dataBase64": format!("data:image/png;base64,{PNG_B64}") }));
    assert_eq!(p3["visualKind"], "sketch");
    assert_eq!(p3["asset"]["mediaType"], "image/png");

    // A failed import leaves the panel intact (UX §45 errors and recovery).
    assert_eq!(
        env.err(
            "storyboard.set_panel_visual",
            json!({ "id": id(&p1), "visual": "sketch", "dataBase64": "bm90IGFuIGltYWdl" })
        ),
        "validation.invalid_input"
    );
    let still = env.ok("storyboard.get", json!({ "id": bid }));
    assert_eq!(still["panels"][0]["visualKind"], "placeholder");
    assert_eq!(still["panels"].as_array().unwrap().len(), 3);
    // Replace with another input method.
    let p1b = env.ok(
        "storyboard.set_panel_visual",
        json!({ "id": id(&p1), "visual": "image", "path": img }),
    );
    assert_eq!(p1b["visualKind"], "image");

    let upd = env.ok(
        "storyboard.update_panel",
        json!({ "id": id(&p2), "description": "Medium — Meera looks up", "framing": "Medium", "movement": "Static", "angle": "Low",
                "soundNote": "Phone rings", "durationMs": 3500, "note": "hold on her eyes" }),
    );
    assert_eq!(upd["durationMs"], 3500);
    assert_eq!(
        env.err(
            "storyboard.update_panel",
            json!({ "id": id(&p2), "durationMs": -5 })
        ),
        "validation.invalid_input"
    );
    let cleared = env.ok(
        "storyboard.update_panel",
        json!({ "id": id(&p2), "durationMs": 0 }),
    );
    assert!(cleared["durationMs"].is_null());

    let list = env.ok("storyboard.list", json!({}));
    assert_eq!(list[0]["panelCount"], 3);
    assert_eq!(list[0]["sceneHeading"], "INT. POLICE STATION - NIGHT");

    // Standalone storyboards need a name and no screenplay shot directions (FSD §32.6).
    assert_eq!(
        env.err("storyboard.create", json!({})),
        "validation.required"
    );
    let free = env.ok("storyboard.create", json!({ "name": "Title sequence" }));
    assert!(free["sceneId"].is_null());
}

#[test]
fn fsd_prod_016_panel_drag_reorder_recalculates_numbers_and_linked_shot_numbers() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let script = three_scenes(&env);
    let sc = &script.scenes[1];
    let s1 = add_shot(&env, sc, "Wide");
    let s2 = add_shot(&env, sc, "Medium");
    let s3 = add_shot(&env, sc, "Close");
    let bid = id(&env.ok("storyboard.create", json!({ "sceneId": sc.id })));
    let mut panels = Vec::new();
    for s in [&s1, &s2, &s3] {
        let p = id(&env.ok(
            "storyboard.add_panel",
            json!({ "storyboardId": bid, "visual": "placeholder" }),
        ));
        env.ok("storyboard.link_shot", json!({ "panelId": p, "shotId": s }));
        panels.push(p);
    }
    let unlinked = id(&env.ok(
        "storyboard.add_panel",
        json!({ "storyboardId": bid, "visual": "placeholder" }),
    ));
    let board = env.ok("storyboard.get", json!({ "id": bid }));
    let badges: Vec<Value> = board["panels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["shotLabel"].clone())
        .collect();
    assert_eq!(badges, [json!("2A"), json!("2B"), json!("2C"), Value::Null]);

    // Drag panel 3 to the front: panel numbers and shot numbers follow the panel order.
    env.ok(
        "storyboard.reorder_panel",
        json!({ "id": panels[2], "index": 0 }),
    );
    let board = env.ok("storyboard.get", json!({ "id": bid }));
    let p = board["panels"].as_array().unwrap();
    assert_eq!(p[0]["id"], panels[2]);
    assert_eq!(p[0]["number"], 1);
    assert_eq!(p[0]["shotLabel"], "2A");
    assert_eq!(p[1]["shotLabel"], "2B");
    assert_eq!(p[3]["id"], unlinked);
    let shots = env.ok("shot.list", json!({ "sceneId": sc.id }));
    assert_eq!(
        arr(&shots).iter().map(id).collect::<Vec<_>>(),
        [s3.clone(), s1.clone(), s2.clone()]
    );

    // One undo step restores both orders.
    env.undo();
    let shots = env.ok("shot.list", json!({ "sceneId": sc.id }));
    assert_eq!(arr(&shots).iter().map(id).collect::<Vec<_>>(), [s1, s2, s3]);
    assert_eq!(
        env.ok("storyboard.get", json!({ "id": bid }))["panels"][0]["id"],
        panels[0]
    );
}

#[test]
fn fsd_stb_001_panels_associate_with_shots_and_unlinking_deletes_neither() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let script = three_scenes(&env);
    let sc = &script.scenes[1];
    let shot = add_shot(&env, sc, "Meera looks up from the desk");
    let bid = id(&env.ok("storyboard.create", json!({ "sceneId": sc.id })));
    let p1 = id(&env.ok(
        "storyboard.add_panel",
        json!({ "storyboardId": bid, "visual": "placeholder" }),
    ));
    let p2 = id(&env.ok(
        "storyboard.add_panel",
        json!({ "storyboardId": bid, "visual": "placeholder" }),
    ));

    let s = env.ok(
        "shot.attach_panel",
        json!({ "shotId": shot, "panelId": p1 }),
    );
    assert_eq!(s["storyboardPanelId"], p1);
    // A second panel is an additional option and never replaces the first (FSD §102).
    let s = env.ok(
        "shot.attach_panel",
        json!({ "shotId": shot, "panelId": p2 }),
    );
    assert_eq!(s["storyboardPanelId"], p1);
    assert_eq!(s["panels"].as_array().unwrap().len(), 2);
    assert_eq!(s["panels"][1]["number"], 2);

    let s = env.ok(
        "shot.detach_panel",
        json!({ "shotId": shot, "panelId": p1 }),
    );
    assert_eq!(
        s["storyboardPanelId"], p2,
        "primary moves to the remaining panel"
    );
    assert_eq!(
        env.err(
            "shot.detach_panel",
            json!({ "shotId": shot, "panelId": p1 })
        ),
        "validation.invalid_input"
    );
    let p = env.ok(
        "storyboard.link_shot",
        json!({ "panelId": p2, "shotId": null }),
    );
    assert!(p["shotId"].is_null());
    assert_eq!(
        env.ok("storyboard.get", json!({ "id": bid }))["panels"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(
        env.ok("shot.get", json!({ "id": shot }))["panels"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        env.err(
            "storyboard.link_shot",
            json!({ "panelId": p2, "shotId": new_id() })
        ),
        "not_found.shot"
    );
}

// ------------------------------------------------------------------ §34 workflows

#[test]
fn fsd_34_path_a_script_to_shots_to_storyboard() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let script = three_scenes(&env);
    let sc = &script.scenes[1];
    let shot = add_shot(&env, sc, "Arjun enters, rain on his coat");
    env.ok(
        "shot.update",
        json!({ "id": shot, "size": "Wide", "movement": "Static" }),
    );
    let img = env.write_file("ref.png", &png_bytes());
    env.ok(
        "shot.set_reference_image",
        json!({ "id": shot, "path": img }),
    );
    let panel = env.ok("shot.create_panel", json!({ "id": shot }));
    assert_eq!(panel["shotLabel"], "2A");
    assert_eq!(panel["description"], "Arjun enters, rain on his coat");
    assert_eq!(panel["framing"], "Wide");
    assert_eq!(
        panel["visualKind"], "image",
        "the reference image becomes the panel visual"
    );
    // The scene storyboard was created on demand and is reused next time.
    let boards = env.ok("storyboard.list", json!({}));
    assert_eq!(arr(&boards).len(), 1);
    assert_eq!(boards[0]["sceneLineageId"], sc.lineage);
    let shot2 = add_shot(&env, sc, "Close on the folder");
    env.ok("shot.create_panel", json!({ "id": shot2 }));
    assert_eq!(arr(&env.ok("storyboard.list", json!({}))).len(), 1);
    assert_eq!(
        env.ok("storyboard.get", json!({ "id": boards[0]["id"] }))["panels"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn fsd_34_path_b_script_to_storyboard_to_shots() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let script = three_scenes(&env);
    let sc = &script.scenes[2];
    let bid = id(&env.ok("storyboard.create", json!({ "sceneId": sc.id })));
    let p = env.ok(
        "storyboard.add_panel",
        json!({ "storyboardId": bid, "visual": "placeholder", "description": "Warm window light on Ravi" }),
    );
    env.ok(
        "storyboard.update_panel",
        json!({ "id": id(&p), "framing": "Close", "angle": "Low" }),
    );
    let shot = env.ok(
        "storyboard.create_shot_from_panel",
        json!({ "panelId": id(&p) }),
    );
    assert_eq!(shot["label"], "3A");
    assert_eq!(shot["description"], "Warm window light on Ravi");
    assert_eq!(shot["size"], "Close");
    assert_eq!(shot["storyboardPanelId"], id(&p));
    // A standalone board needs a scene to be chosen.
    let free = id(&env.ok("storyboard.create", json!({ "name": "Ideas" })));
    let fp = id(&env.ok(
        "storyboard.add_panel",
        json!({ "storyboardId": free, "visual": "placeholder" }),
    ));
    assert_eq!(
        env.err(
            "storyboard.create_shot_from_panel",
            json!({ "panelId": fp })
        ),
        "validation.required"
    );
    let s = env.ok(
        "storyboard.create_shot_from_panel",
        json!({ "panelId": fp, "sceneId": script.scenes[0].id }),
    );
    assert_eq!(s["label"], "1A");
    assert_eq!(s["description"], "Shot from storyboard panel");
}

#[test]
fn fsd_34_path_c_shots_only_needs_no_storyboard() {
    let env = TestEnv::with_project("Short", "Short Film");
    let script = three_scenes(&env);
    add_shot(&env, &script.scenes[0], "Wide of the platform");
    assert!(arr(&env.ok("storyboard.list", json!({}))).is_empty());
    let scenes = env.ok("visual.scenes", json!({}));
    assert_eq!(scenes["sourceKind"], "currentDraft");
    assert_eq!(scenes["scenes"][0]["shotCount"], 1);
    assert_eq!(scenes["scenes"][0]["storyboardCount"], 0);
}

#[test]
fn no_screenplay_yet_offers_no_scenes_but_standalone_boards_work() {
    let env = TestEnv::with_project("Short", "Short Film");
    let scenes = env.ok("visual.scenes", json!({}));
    assert_eq!(scenes["sourceKind"], "none");
    assert!(arr(&scenes["scenes"]).is_empty());
    assert_eq!(
        env.err(
            "shot.create",
            json!({ "sceneId": new_id(), "description": "x" })
        ),
        "not_found.scene"
    );
    env.ok("storyboard.create", json!({ "name": "Opening titles" }));
    env.ok("moodboard.create", json!({ "name": "Overall Look" }));
}

// ------------------------------------------------------------------ §55 reconciliation

#[test]
fn fsd_55_scene_change_flags_review_and_never_deletes_planning() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let script = three_scenes(&env);
    let sc = &script.scenes[1];
    let shot = add_shot(&env, sc, "Arjun enters");
    let bid = id(&env.ok("storyboard.create", json!({ "sceneId": sc.id })));
    env.ok(
        "storyboard.add_panel",
        json!({ "storyboardId": bid, "visual": "placeholder" }),
    );

    assert_eq!(
        env.ok("shot.get", json!({ "id": shot }))["needsReview"],
        false
    );
    edit_action(&env, &sc.id, "Arjun enters, soaked, holding a red folder.");
    let s = env.ok("shot.get", json!({ "id": shot }));
    assert_eq!(s["needsReview"], true, "Scene changed since planning");
    assert_eq!(
        s["description"], "Arjun enters",
        "planning content is untouched"
    );
    assert_eq!(
        env.ok("storyboard.get", json!({ "id": bid }))["board"]["needsReview"],
        true
    );
    let scenes = env.ok("visual.scenes", json!({}));
    assert_eq!(scenes["scenes"][1]["needsReview"], true);
    assert!(
        scenes["scenes"][1]["changedAt"].as_i64().is_some(),
        "the indicator says when (FSD §125)"
    );
    assert_eq!(scenes["scenes"][0]["needsReview"], false);

    // Marking the scene reviewed clears the indicator for its shots and storyboards (undoable).
    env.ok(
        "visual.mark_scene_reviewed",
        json!({ "sceneLineageId": sc.lineage }),
    );
    assert_eq!(
        env.ok("shot.get", json!({ "id": shot }))["needsReview"],
        false
    );
    assert_eq!(
        env.ok("storyboard.get", json!({ "id": bid }))["board"]["needsReview"],
        false
    );
    env.undo();
    assert_eq!(
        env.ok("shot.get", json!({ "id": shot }))["needsReview"],
        true
    );
    env.redo();

    // Scene removed from the script: planning kept, hidden from active views unless inspected.
    remove_scene(&env, &sc.id);
    let scenes = env.ok("visual.scenes", json!({}));
    assert_eq!(arr(&scenes["scenes"]).len(), 2);
    assert_eq!(scenes["removed"][0]["lineageId"], sc.lineage);
    assert_eq!(
        scenes["removed"][0]["heading"],
        "INT. POLICE STATION - NIGHT"
    );
    assert_eq!(scenes["removed"][0]["shotCount"], 1);
    assert!(arr(&env.ok("shot.list", json!({}))).is_empty());
    let with_removed = env.ok("shot.list", json!({ "includeRemoved": true }));
    assert_eq!(with_removed[0]["sceneRemoved"], true);
    assert_eq!(with_removed[0]["label"], "A");
    assert_eq!(
        env.ok("storyboard.get", json!({ "id": bid }))["board"]["sceneRemoved"],
        true
    );
    assert_eq!(
        env.err(
            "visual.mark_scene_reviewed",
            json!({ "sceneLineageId": sc.lineage })
        ),
        "not_found.scene"
    );
    assert_eq!(
        env.err(
            "shot.create",
            json!({ "sceneId": sc.id, "description": "x" })
        ),
        "not_found.scene"
    );
}

#[test]
fn fsd_53_planning_follows_the_active_production_source_by_scene_identity() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let first = three_scenes(&env);
    let shot = add_shot(&env, &first.scenes[2], "Warm window light");
    // A new draft becomes current, but production is locked to the first draft.
    set_production_source(&env, &first.draft);
    let second = new_draft(&env, &first, true);
    let scenes = env.ok("visual.scenes", json!({}));
    assert_eq!(scenes["sourceKind"], "productionSource");
    assert_eq!(scenes["scenes"][2]["sceneId"], first.scenes[2].id);

    // Switching the production source to the new draft keeps planning by identity.
    set_production_source(&env, &second.draft);
    let s = env.ok("shot.get", json!({ "id": shot }));
    assert_eq!(s["sceneId"], second.scenes[2].id);
    assert_eq!(s["label"], "3A");
    assert_eq!(
        s["needsReview"], false,
        "identical content does not need review"
    );
    // New shots can be added from either scene row id (resolved by identity).
    let s2 = env.ok(
        "shot.create",
        json!({ "sceneId": first.scenes[2].id, "description": "Ravi at the table" }),
    );
    assert_eq!(s2["label"], "3B");
    assert_eq!(s2["sceneId"], second.scenes[2].id);
}

#[test]
fn fsd_55_copy_planning_to_a_duplicated_scene() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let script = three_scenes(&env);
    let from = &script.scenes[1];
    let s1 = add_shot(&env, from, "Wide");
    add_shot(&env, from, "Close");
    let bid = id(&env.ok("storyboard.create", json!({ "sceneId": from.id })));
    let p = id(&env.ok(
        "storyboard.add_panel",
        json!({ "storyboardId": bid, "visual": "placeholder", "description": "Wide" }),
    ));
    env.ok(
        "storyboard.link_shot",
        json!({ "panelId": p, "shotId": s1 }),
    );

    let res = env.ok(
        "shot.copy_planning",
        json!({ "fromSceneLineageId": from.lineage, "toSceneId": script.scenes[2].id }),
    );
    assert_eq!(res, json!({ "shots": 2, "storyboards": 1 }));
    let copied = env.ok("shot.list", json!({ "sceneId": script.scenes[2].id }));
    assert_eq!(labels(&copied), ["3A", "3B"]);
    assert_ne!(copied[0]["id"], s1, "copies are new identities");
    assert_eq!(
        copied[0]["panels"].as_array().unwrap().len(),
        1,
        "panel links are remapped to the copies"
    );
    assert_eq!(
        env.ok("shot.get", json!({ "id": s1 }))["panels"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        env.err(
            "shot.copy_planning",
            json!({ "fromSceneLineageId": from.lineage, "toSceneId": from.id })
        ),
        "validation.invalid_input"
    );
    env.undo();
    assert!(arr(&env.ok("shot.list", json!({ "sceneId": script.scenes[2].id }))).is_empty());
}

// ------------------------------------------------------------------ moodboards

#[test]
fn fsd_prod_011_012_moodboard_create_rename_and_add_move_items() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let sugg = env.ok("moodboard.suggestions", json!({}));
    assert_eq!(arr(&sugg).len(), 7);
    assert_eq!(
        env.err("moodboard.create", json!({ "name": "  " })),
        "validation.required"
    );
    let board = env.ok("moodboard.create", json!({ "name": "Overall Look" }));
    let bid = id(&board);
    assert!(
        !arr(&env.ok("moodboard.suggestions", json!({})))
            .iter()
            .any(|s| s == "Overall Look"),
        "suggestions only offer unused names"
    );
    env.ok("moodboard.create", json!({ "name": "My own name" }));
    let renamed = env.ok(
        "moodboard.rename",
        json!({ "id": bid, "name": "Look & Feel" }),
    );
    assert_eq!(renamed["name"], "Look & Feel");

    let img = env.write_file("rain.png", &png_bytes());
    let imgs = env.ok(
        "moodboard.add_images",
        json!({ "moodboardId": bid, "paths": [img], "x": 20, "y": 24 }),
    );
    let image = &imgs[0];
    assert_eq!(image["kind"], "image");
    assert_eq!(
        (image["x"].as_i64(), image["y"].as_i64()),
        (Some(20), Some(24))
    );
    assert_eq!(image["asset"]["available"], true);
    let doc = env.write_file("script.pdf", b"%PDF-1.4");
    assert_eq!(
        env.err(
            "moodboard.add_images",
            json!({ "moodboardId": bid, "paths": [doc] })
        ),
        "validation.invalid_input"
    );

    let pasted = env.ok(
        "moodboard.add_image_data",
        json!({ "moodboardId": bid, "dataBase64": PNG_B64 }),
    );
    assert_eq!(pasted["asset"]["mediaType"], "image/png");
    let note = env.ok("moodboard.add_note", json!({ "moodboardId": bid, "text": "Cold, wet, blue-grey. Warm light only from windows." }));
    let private = env.ok(
        "moodboard.add_note",
        json!({ "moodboardId": bid, "text": "Budget: rain machine hire", "isPrivate": true }),
    );
    assert_eq!(private["isPrivate"], true);
    let link = env.ok("moodboard.add_link", json!({ "moodboardId": bid, "url": "www.imdb.com/title/tt0353969", "title": "Reference: Memories of Murder" }));
    assert_eq!(link["url"], "https://www.imdb.com/title/tt0353969");
    assert_eq!(link["urlHost"], "imdb.com");
    assert_eq!(
        env.err(
            "moodboard.add_link",
            json!({ "moodboardId": bid, "url": "javascript:alert(1)" })
        ),
        "validation.invalid_input"
    );

    // Caption, move (multi-select), resize, stacking order.
    let cap = env.ok(
        "moodboard.update_item",
        json!({ "id": id(image), "caption": "Rain on glass — main tone" }),
    );
    assert_eq!(cap["caption"], "Rain on glass — main tone");
    assert_eq!(
        env.err(
            "moodboard.update_item",
            json!({ "id": id(image), "body": "x" })
        ),
        "validation.invalid_input"
    );
    env.ok("moodboard.move_items", json!({ "moves": [{ "id": id(image), "x": 290, "y": 24 }, { "id": id(&note), "x": 520, "y": 30 }] }));
    env.ok(
        "moodboard.resize_item",
        json!({ "id": id(image), "w": 5, "h": 110 }),
    );
    env.ok(
        "moodboard.arrange_item",
        json!({ "id": id(image), "placement": "front" }),
    );
    let got = env.ok("moodboard.get", json!({ "id": bid }));
    let items = arr(&got["items"]);
    assert_eq!(items.len(), 5);
    let last = items.last().unwrap();
    assert_eq!(last["id"], id(image), "brought to front = drawn last");
    assert_eq!(
        (last["x"].as_i64(), last["w"].as_i64(), last["h"].as_i64()),
        (Some(290), Some(40), Some(110)),
        "size is clamped"
    );

    // The group move is one undo step.
    env.undo(); // arrange
    env.undo(); // resize
    env.undo(); // move
    let got = env.ok("moodboard.get", json!({ "id": bid }));
    let img_now = arr(&got["items"])
        .iter()
        .find(|i| i["id"] == id(image))
        .unwrap()
        .clone();
    let note_now = arr(&got["items"])
        .iter()
        .find(|i| i["id"] == id(&note))
        .unwrap()
        .clone();
    assert_eq!(img_now["x"], 20);
    assert_ne!(note_now["x"], 520);

    let list = env.ok("moodboard.list", json!({}));
    assert_eq!(list[0]["itemCount"], 5);
    env.ok(
        "moodboard.reorder",
        json!({ "id": list[1]["id"], "index": 0 }),
    );
    assert_eq!(
        env.ok("moodboard.list", json!({}))[0]["name"],
        "My own name"
    );
}

#[test]
fn moodboard_can_reference_a_scene() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let script = three_scenes(&env);
    let b = env.ok(
        "moodboard.create",
        json!({ "name": "Station mood", "sceneId": script.scenes[1].id }),
    );
    assert_eq!(b["sceneNumber"], "2");
    assert_eq!(
        env.ok("visual.scenes", json!({}))["scenes"][1]["moodboardCount"],
        1
    );
}

#[test]
fn moodboard_images_from_the_idea_vault_are_guarded_and_copied() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let bid = id(&env.ok("moodboard.create", json!({ "name": "Overall Look" })));
    let s = env.core.project().unwrap();
    let vault_exists: bool = s
        .store
        .read(|c| {
            Ok(c.query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='vault_item')",
                [],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    if vault_exists {
        // The Idea Vault module owns this table; only the guarded not-found path is checked here.
        assert_eq!(
            env.err(
                "moodboard.add_vault_image",
                json!({ "moodboardId": bid, "vaultItemId": new_id() })
            ),
            "not_found.vault_item"
        );
        return;
    }
    assert!(
        arr(&env.ok("visual.vault_images", json!({}))).is_empty(),
        "no vault table → nothing to offer"
    );
    assert_eq!(
        env.err(
            "moodboard.add_vault_image",
            json!({ "moodboardId": bid, "vaultItemId": new_id() })
        ),
        "not_found.vault_item"
    );

    // Simulate a minimal vault table with an image item.
    s.store
        .with_writer(|c| {
            c.execute_batch("CREATE TABLE vault_item(id TEXT PRIMARY KEY, title TEXT, asset_id TEXT, deleted_at INTEGER);")?;
            Ok(())
        })
        .unwrap();
    let img = env.write_file("station.png", &png_bytes());
    let added = env.ok("files.add", json!({ "paths": [img] }));
    let asset_id = added[0]["asset"]["id"].as_str().unwrap().to_string();
    let vid = new_id();
    s.store
        .with_writer(|c| {
            c.execute("INSERT INTO vault_item(id, title, asset_id) VALUES (?1, 'Empty platform, 3 am', ?2)", params![vid, asset_id])?;
            Ok(())
        })
        .unwrap();
    let offered = env.ok("visual.vault_images", json!({}));
    assert_eq!(offered[0]["title"], "Empty platform, 3 am");
    let item = env.ok(
        "moodboard.add_vault_image",
        json!({ "moodboardId": bid, "vaultItemId": vid }),
    );
    assert_eq!(item["sourceVaultItemId"], vid);
    assert_eq!(item["caption"], "Empty platform, 3 am");
    assert_eq!(item["asset"]["available"], true);
}

// ------------------------------------------------------------------ delete / restore / purge

#[test]
fn fsd_52_shot_delete_restore_returns_to_its_place_and_purge_unlinks_panels() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let script = three_scenes(&env);
    let sc = &script.scenes[1];
    let a = add_shot(&env, sc, "Wide");
    let b = add_shot(&env, sc, "Medium");
    let c = add_shot(&env, sc, "Close");
    let img = env.write_file("ref.png", &png_bytes());
    let with_ref = env.ok("shot.set_reference_image", json!({ "id": b, "path": img }));
    let ref_path = with_ref["referenceAsset"]["path"]
        .as_str()
        .unwrap()
        .to_string();
    let panel = env.ok("shot.create_panel", json!({ "id": b }));

    env.ok("shot.delete", json!({ "ids": [b] }));
    assert_eq!(
        labels(&env.ok("shot.list", json!({ "sceneId": sc.id }))),
        ["2A", "2B"]
    );
    let bid = panel["storyboardId"].as_str().unwrap();
    assert!(
        env.ok("storyboard.get", json!({ "id": bid }))["panels"][0]["shotId"].is_null(),
        "link hidden while deleted"
    );
    let t = trash_entry(&env, &b);
    assert_eq!(t["title"], "Shot 2B — Medium");
    env.ok("trash.restore", json!({ "id": t["id"] }));
    let list = env.ok("shot.list", json!({ "sceneId": sc.id }));
    assert_eq!(
        arr(&list).iter().map(id).collect::<Vec<_>>(),
        [a.clone(), b.clone(), c.clone()],
        "back in its previous order"
    );
    assert_eq!(
        env.ok("storyboard.get", json!({ "id": bid }))["panels"][0]["shotLabel"],
        "2B",
        "link comes back"
    );

    // Purge: the panel survives without the link; the image (shared with the panel) stays until unreferenced.
    env.ok("shot.delete", json!({ "ids": [b] }));
    env.ok("trash.purge", json!({ "id": trash_entry(&env, &b)["id"] }));
    let board = env.ok("storyboard.get", json!({ "id": bid }));
    assert_eq!(board["panels"].as_array().unwrap().len(), 1);
    assert!(board["panels"][0]["shotId"].is_null());
    assert!(
        std::path::Path::new(&ref_path).exists(),
        "still used by the panel"
    );
    env.ok("storyboard.delete_panels", json!({ "ids": [panel["id"]] }));
    env.ok(
        "trash.purge",
        json!({ "id": trash_entry(&env, panel["id"].as_str().unwrap())["id"] }),
    );
    assert!(
        !std::path::Path::new(&ref_path).exists(),
        "purged once nothing references it"
    );
}

#[test]
fn fsd_52_panel_restore_goes_to_unassigned_when_its_storyboard_is_gone() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let bid = id(&env.ok("storyboard.create", json!({ "name": "Chase" })));
    let p1 = id(&env.ok(
        "storyboard.add_panel",
        json!({ "storyboardId": bid, "visual": "placeholder", "description": "Bike skids" }),
    ));
    let p2 = id(&env.ok(
        "storyboard.add_panel",
        json!({ "storyboardId": bid, "visual": "placeholder", "description": "Bus pulls out" }),
    ));
    env.ok("storyboard.delete_panels", json!({ "ids": [p1] }));
    let t = trash_entry(&env, &p1);
    assert_eq!(t["title"], "Panel 1 — Bike skids");
    env.ok("trash.restore", json!({ "id": t["id"] }));
    assert_eq!(
        env.ok("storyboard.get", json!({ "id": bid }))["panels"][0]["id"],
        p1,
        "restored to its previous order"
    );

    // Delete the panel, then the board and purge the board: the panel restores to "Unassigned panels".
    env.ok("storyboard.delete_panels", json!({ "ids": [p1] }));
    env.ok("storyboard.delete", json!({ "id": bid }));
    env.ok(
        "trash.purge",
        json!({ "id": trash_entry(&env, &bid)["id"] }),
    );
    assert!(
        arr(&env.ok("trash.list", json!({})))
            .iter()
            .all(|t| t["objectId"] != p2),
        "board purge removes its panels"
    );
    // p1 was already in the trash separately → it was purged with its board too.
    assert!(
        arr(&env.ok("trash.list", json!({})))
            .iter()
            .all(|t| t["objectId"] != p1)
    );

    // Now the unassigned path proper: panel deleted, board soft-deleted, panel restored.
    let b2 = id(&env.ok("storyboard.create", json!({ "name": "Rooftop" })));
    let q = id(&env.ok(
        "storyboard.add_panel",
        json!({ "storyboardId": b2, "visual": "placeholder", "description": "Wide roof" }),
    ));
    env.ok("storyboard.delete_panels", json!({ "ids": [q] }));
    env.ok("storyboard.delete", json!({ "id": b2 }));
    env.ok(
        "trash.restore",
        json!({ "id": trash_entry(&env, &q)["id"] }),
    );
    let boards = env.ok("storyboard.list", json!({}));
    assert_eq!(boards[0]["name"], "Unassigned panels");
    assert_eq!(
        env.ok("storyboard.get", json!({ "id": boards[0]["id"] }))["panels"][0]["id"],
        q
    );
}

#[test]
fn fsd_52_moodboard_delete_restore_and_item_to_unassigned() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let bid = id(&env.ok("moodboard.create", json!({ "name": "Costume" })));
    let img = env.write_file("coat.png", &png_bytes());
    let item = env.ok(
        "moodboard.add_images",
        json!({ "moodboardId": bid, "paths": [img] }),
    )[0]
    .clone();
    let path = item["asset"]["path"].as_str().unwrap().to_string();
    env.ok(
        "moodboard.update_item",
        json!({ "id": item["id"], "caption": "Arjun's raincoat" }),
    );
    let note = id(&env.ok(
        "moodboard.add_note",
        json!({ "moodboardId": bid, "text": "Wool, never leather" }),
    ));
    assert_eq!(
        arr(&env.ok("search.query", json!({ "text": "raincoat" }))).len(),
        1
    );

    // Board delete keeps items with it; search hides them; restore brings all back.
    env.ok("moodboard.delete", json!({ "id": bid }));
    assert!(arr(&env.ok("moodboard.list", json!({}))).is_empty());
    assert!(arr(&env.ok("search.query", json!({ "text": "raincoat" }))).is_empty());
    env.undo();
    assert_eq!(
        arr(&env.ok("search.query", json!({ "text": "raincoat" }))).len(),
        1,
        "undo re-indexes the items"
    );
    env.ok("moodboard.delete", json!({ "id": bid }));
    env.ok(
        "trash.restore",
        json!({ "id": trash_entry(&env, &bid)["id"] }),
    );
    assert_eq!(
        env.ok("moodboard.get", json!({ "id": bid }))["items"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        arr(&env.ok("search.query", json!({ "text": "raincoat" }))).len(),
        1
    );

    // Item deleted, then its board deleted: the item restores into "Unassigned".
    env.ok("moodboard.delete_items", json!({ "ids": [note] }));
    assert_eq!(
        trash_entry(&env, &note)["title"],
        "Note — Wool, never leather"
    );
    env.ok("moodboard.delete", json!({ "id": bid }));
    env.ok(
        "trash.restore",
        json!({ "id": trash_entry(&env, &note)["id"] }),
    );
    let boards = env.ok("moodboard.list", json!({}));
    assert_eq!(boards[0]["name"], "Unassigned");
    assert_eq!(
        env.ok("moodboard.get", json!({ "id": boards[0]["id"] }))["items"][0]["id"],
        note
    );

    // Purging the board removes its image file.
    assert!(std::path::Path::new(&path).exists());
    env.ok(
        "trash.purge",
        json!({ "id": trash_entry(&env, &bid)["id"] }),
    );
    assert!(!std::path::Path::new(&path).exists());
}

// ------------------------------------------------------------------ permissions / search / persistence

#[test]
fn viewers_and_commenters_cannot_change_visual_planning() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let script = three_scenes(&env);
    let shot = add_shot(&env, &script.scenes[0], "Wide");
    let bid = id(&env.ok("moodboard.create", json!({ "name": "Lighting" })));
    for role in [Role::Viewer, Role::Commenter] {
        let who = env.actor_with_role(role);
        for (op, args) in [
            (
                "shot.create",
                json!({ "sceneId": script.scenes[0].id, "description": "x" }),
            ),
            ("shot.update", json!({ "id": shot, "lens": "50mm" })),
            ("shot.reorder", json!({ "id": shot, "index": 0 })),
            ("shot.delete", json!({ "ids": [shot] })),
            ("moodboard.create", json!({ "name": "x" })),
            (
                "moodboard.add_note",
                json!({ "moodboardId": bid, "text": "x" }),
            ),
            ("storyboard.create", json!({ "name": "x" })),
            (
                "visual.mark_scene_reviewed",
                json!({ "sceneLineageId": script.scenes[0].lineage }),
            ),
        ] {
            let e = env.call_as(&who, op, args).unwrap_err();
            assert_eq!(e.code.0, "permission.denied", "{op} as {role:?}");
        }
        // Reading is allowed.
        assert_eq!(
            arr(&env.call_as(&who, "shot.list", json!({})).unwrap()).len(),
            1
        );
        env.call_as(&who, "moodboard.get", json!({ "id": bid }))
            .unwrap();
        env.call_as(&who, "visual.scenes", json!({})).unwrap();
    }
    assert_eq!(
        env.ok("shot.get", json!({ "id": shot }))["lens"],
        Value::Null
    );
}

#[test]
fn search_finds_shots_storyboards_and_moodboards_with_navigation() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let script = three_scenes(&env);
    let shot = add_shot(&env, &script.scenes[1], "Folder lands on the desk");
    let hit = &env.ok("search.query", json!({ "text": "folder lands" }))[0];
    assert_eq!(hit["entityType"], "shot");
    assert_eq!(
        hit["nav"],
        json!({ "workspace": "production", "sub": "shots", "shotId": shot })
    );

    let bid = id(&env.ok("storyboard.create", json!({ "name": "Motorbike chase" })));
    let p = id(&env.ok("storyboard.add_panel", json!({ "storyboardId": bid, "visual": "placeholder", "description": "Sparks under the wheel" })));
    let hit = &env.ok("search.query", json!({ "text": "motorbike" }))[0];
    assert_eq!(hit["nav"]["storyboardId"], bid);
    let hit = &env.ok("search.query", json!({ "text": "sparks" }))[0];
    assert_eq!(
        hit["nav"],
        json!({ "workspace": "production", "sub": "storyboards", "storyboardId": bid, "panelId": p })
    );

    let mid = id(&env.ok("moodboard.create", json!({ "name": "Production Design" })));
    let hit = &env.ok("search.query", json!({ "text": "production design" }))[0];
    assert_eq!(
        hit["nav"],
        json!({ "workspace": "production", "sub": "moodboards", "moodboardId": mid })
    );

    env.ok("shot.delete", json!({ "ids": [shot] }));
    assert!(
        arr(&env.ok("search.query", json!({ "text": "folder lands" }))).is_empty(),
        "deleted shots leave search"
    );
}

#[test]
fn visual_planning_persists_across_restart() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let script = three_scenes(&env);
    let shot = add_shot(&env, &script.scenes[1], "Arjun enters");
    let bid = id(&env.ok(
        "storyboard.create",
        json!({ "sceneId": script.scenes[1].id }),
    ));
    env.ok(
        "storyboard.add_panel",
        json!({ "storyboardId": bid, "visual": "sketch", "dataBase64": PNG_B64 }),
    );
    let mid = id(&env.ok("moodboard.create", json!({ "name": "Overall Look" })));
    env.ok(
        "moodboard.add_note",
        json!({ "moodboardId": mid, "text": "Blue-grey palette", "x": 40, "y": 60 }),
    );
    let path = env.project_path();
    let env = env.restart();
    env.reopen_project(&path);
    assert_eq!(env.ok("shot.get", json!({ "id": shot }))["label"], "2A");
    let board = env.ok("storyboard.get", json!({ "id": bid }));
    assert_eq!(board["panels"][0]["visualKind"], "sketch");
    assert_eq!(board["panels"][0]["asset"]["available"], true);
    let mb = env.ok("moodboard.get", json!({ "id": mid }));
    assert_eq!(
        (mb["items"][0]["x"].as_i64(), mb["items"][0]["y"].as_i64()),
        (Some(40), Some(60))
    );
}
