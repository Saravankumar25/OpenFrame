//! Story workspace acceptance tests (FSD §7–14, §18, §25, §89–91; FSD-STORY-001..030),
//! driven through the public operation registry like the UI.

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

fn card(env: &TestEnv, seq_id: &str, desc: &str) -> String {
    card_in(env, "sequence", Some(seq_id), desc)
}

fn board(env: &TestEnv) -> Value {
    env.ok("story.board", json!({}))
}

fn item(kind: &str, id: &str) -> Value {
    json!({ "kind": kind, "id": id })
}

/// Ids (in order) of the children of the container with `container_id`
/// (an act, a sequence), or of "parking"/"unassigned".
fn kids(b: &Value, container_id: &str) -> Vec<String> {
    fn ids(items: &Value) -> Vec<String> {
        items
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["id"].as_str().unwrap().to_string())
            .collect()
    }
    fn walk(items: &Value, target: &str) -> Option<Vec<String>> {
        for it in items.as_array().unwrap() {
            if it["kind"] == "sequence" {
                if it["id"] == target {
                    return Some(ids(&it["items"]));
                }
                if let Some(v) = walk(&it["items"], target) {
                    return Some(v);
                }
            }
        }
        None
    }
    if container_id == "parking" || container_id == "unassigned" {
        return ids(&b[container_id]);
    }
    for a in b["acts"].as_array().unwrap() {
        if a["id"] == container_id {
            return ids(&a["items"]);
        }
        if let Some(v) = walk(&a["items"], container_id) {
            return v;
        }
    }
    if let Some(v) = walk(&b["unassigned"], container_id) {
        return v;
    }
    panic!("container {container_id} not on board: {b}");
}

fn act_ids(b: &Value) -> Vec<String> {
    b["acts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["id"].as_str().unwrap().to_string())
        .collect()
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

// ------------------------------------------------------------ structure

#[test]
fn fsd_story_001_acts_contain_sequences_and_scenes() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let empty = board(&env);
    assert!(empty["acts"].as_array().unwrap().is_empty());
    let a1 = act(&env, "Act 1 — The Return");
    let s1 = seq(&env, &a1, "Hero Introduction");
    let direct = card_in(&env, "act", Some(&a1), "Arjun steps off the night bus.");
    let inner = card(&env, &s1, "Morning at the old house.");
    let b = board(&env);
    assert_eq!(b["acts"][0]["title"], "Act 1 — The Return");
    assert_eq!(
        kids(&b, &a1),
        vec![s1.clone(), direct.clone()],
        "sequence then direct card, in creation order"
    );
    assert_eq!(kids(&b, &s1), vec![inner]);
    assert_eq!(b["acts"][0]["cardCount"], 2);
    assert_eq!(b["cardCount"], 2);
    // FSD §8.1: a new act appears at the end.
    let a2 = act(&env, "Act 2");
    assert_eq!(act_ids(&board(&env)), vec![a1, a2]);
}

#[test]
fn fsd_story_002_028_sequence_needs_only_a_name() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 1");
    let s = seq(&env, &a, "Hero Introduction");
    assert_eq!(
        board(&env)["acts"][0]["items"][0]["title"],
        "Hero Introduction"
    );
    // No purpose/goal field exists; unknown fields are rejected, blank names are required.
    assert_eq!(
        env.err(
            "story.create_sequence",
            json!({ "actId": a, "title": "X", "purpose": "hook" })
        ),
        "validation.invalid_input"
    );
    assert_eq!(
        env.err(
            "story.create_sequence",
            json!({ "actId": a, "title": "  " })
        ),
        "validation.required"
    );
    env.ok(
        "story.update_sequence",
        json!({ "id": s, "title": "Railway Station Return" }),
    );
    assert_eq!(
        board(&env)["acts"][0]["items"][0]["title"],
        "Railway Station Return"
    );
}

#[test]
fn fsd_story_003_reorder_20_cards_keeps_identities_and_order_persists_after_restart() {
    let env = TestEnv::with_project("Twenty", "Feature Film");
    let a = act(&env, "Act 1");
    let s = seq(&env, &a, "Everything");
    let ids: Vec<String> = (1..=20)
        .map(|i| card(&env, &s, &format!("Card number {i}")))
        .collect();
    assert_eq!(kids(&board(&env), &s), ids);
    // Reverse the order one drag at a time (each moves a card to the front).
    for cid in &ids {
        let first = kids(&board(&env), &s)[0].clone();
        if &first == cid {
            continue;
        }
        env.ok(
            "story.move_items",
            json!({ "items": [item("card", cid)], "target": { "parentType": "sequence", "parentId": s }, "before": item("card", &first) }),
        );
    }
    let mut reversed = ids.clone();
    reversed.reverse();
    assert_eq!(
        kids(&board(&env), &s),
        reversed,
        "same identities, new order"
    );
    // No scene numbers anywhere on cards.
    let b = board(&env);
    let c0 = &b["acts"][0]["items"][0]["items"][0];
    assert!(c0.get("number").is_none() && c0.get("sceneNumber").is_none());

    let path = env.project_path();
    let env = env.restart();
    env.reopen_project(&path);
    assert_eq!(
        kids(&board(&env), &s),
        reversed,
        "order persists after restart"
    );
}

#[test]
fn fsd_story_004_015_drag_changes_order_and_undo_restores_it() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 1");
    let s1 = seq(&env, &a, "First Investigation");
    let s2 = seq(&env, &a, "Chase");
    let c1 = card(&env, &s1, "Crime scene in the rain");
    let c2 = card(&env, &s1, "Morgue");
    let c3 = card(&env, &s2, "Ravi runs");
    env.ok(
        "story.move_items",
        json!({ "items": [item("card", &c1)], "target": { "parentType": "sequence", "parentId": s2 }, "before": item("card", &c3) }),
    );
    let b = board(&env);
    assert_eq!(kids(&b, &s1), vec![c2.clone()]);
    assert_eq!(kids(&b, &s2), vec![c1.clone(), c3.clone()]);
    let step = env.undo();
    assert!(
        step["label"]
            .as_str()
            .unwrap()
            .starts_with("Moved Scene Card"),
        "{step}"
    );
    let b = board(&env);
    assert_eq!(kids(&b, &s1), vec![c1.clone(), c2.clone()]);
    assert_eq!(kids(&b, &s2), vec![c3.clone()]);
    env.redo();
    assert_eq!(kids(&board(&env), &s2), vec![c1, c3]);
}

#[test]
fn fsd_story_005_012_parking_lot_persists_and_restores_to_origin() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 1");
    let s = seq(&env, &a, "Hero Introduction");
    let c1 = card(&env, &s, "Bus");
    let c2 = card(&env, &s, "Wedding band — cool but doesn't fit");
    let c3 = card(&env, &s, "Police station");
    env.ok("story.park_items", json!({ "items": [item("card", &c2)] }));
    let b = board(&env);
    assert_eq!(kids(&b, &s), vec![c1.clone(), c3.clone()]);
    assert_eq!(kids(&b, "parking"), vec![c2.clone()]);
    assert_eq!(b["parking"][0]["parkedFromId"], s.as_str());
    // A beat can be created directly in the Parking Lot (FSD-STORY-004 / §10.3).
    let beat = id(&env.ok(
        "story.create_beat",
        json!({ "text": "Ravi's secret radio" }),
    ));

    let path = env.project_path();
    let env = env.restart();
    env.reopen_project(&path);
    let b = board(&env);
    assert_eq!(kids(&b, "parking"), vec![c2.clone(), beat.clone()]);

    let placed = env.ok(
        "story.restore_from_parking",
        json!({ "items": [item("card", &c2)] }),
    );
    assert_eq!(placed[0]["label"], "Sequence “Hero Introduction”");
    let b = board(&env);
    assert_eq!(
        kids(&b, &s),
        vec![c1, c2.clone(), c3],
        "returns to its former position"
    );
    assert!(b["acts"][0]["items"][0]["items"][1]["parkedFromId"].is_null());
    // A beat that never lived in the story goes to the end of the last act.
    let placed = env.ok(
        "story.restore_from_parking",
        json!({ "items": [item("beat", &beat)] }),
    );
    assert_eq!(placed[0]["container"]["parentType"], "act");
    assert!(kids(&board(&env), "parking").is_empty());
}

#[test]
fn fsd_story_006_long_description_is_stored_whole() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let long = "A very long description. ".repeat(40);
    let c = id(&env.ok(
        "story.create_card",
        json!({ "shortDescription": long.clone() }),
    ));
    let d = env.ok("story.card", json!({ "id": c }));
    assert_eq!(d["card"]["shortDescription"], long.trim_end());
    // FSD §11.5: with no act yet, the card lands in a new "Act 1".
    assert_eq!(d["location"], "Act 1");
}

#[test]
fn fsd_story_005_026_027_card_needs_only_description_blank_allowed() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 1");
    let blank = card_in(&env, "act", Some(&a), "");
    let d = env.ok("story.card", json!({ "id": blank }));
    assert_eq!(d["card"]["shortDescription"], "");
    assert!(d["card"]["sceneHeading"].is_null());
    // No character or story-day fields exist on cards.
    assert_eq!(
        env.err(
            "story.create_card",
            json!({ "shortDescription": "x", "storyDay": "1" })
        ),
        "validation.invalid_input"
    );
    assert_eq!(
        env.err(
            "story.update_card",
            json!({ "id": blank, "characters": ["RAVI"] })
        ),
        "validation.invalid_input"
    );
    env.ok("story.update_card", json!({ "id": blank, "shortDescription": "Typed later", "sceneHeading": "INT. HOUSE — DAY" }));
    let d = env.ok("story.card", json!({ "id": blank }));
    assert_eq!(d["card"]["sceneHeading"], "INT. HOUSE — DAY");
    // Typing into the same card coalesces into one undo step.
    env.ok(
        "story.update_card",
        json!({ "id": blank, "shortDescription": "Typed later, more" }),
    );
    env.undo();
    assert_eq!(
        env.ok("story.card", json!({ "id": blank }))["card"]["shortDescription"],
        ""
    );
}

#[test]
fn fsd_story_008_create_at_drop_position() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 1");
    let s = seq(&env, &a, "S");
    let c1 = card(&env, &s, "one");
    let c3 = card(&env, &s, "three");
    let c2 = id(&env.ok(
        "story.create_card",
        json!({ "parent": { "parentType": "sequence", "parentId": s }, "shortDescription": "two", "index": 1 }),
    ));
    assert_eq!(kids(&board(&env), &s), vec![c1, c2, c3]);
}

#[test]
fn fsd_story_009_move_sequence_between_acts_keeps_children() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a1 = act(&env, "Act 1");
    let a2 = act(&env, "Act 2");
    let s = seq(&env, &a1, "Chase");
    let c1 = card(&env, &s, "Ravi runs");
    let c2 = card(&env, &s, "Motorcycle");
    env.ok("story.move_items", json!({ "items": [item("sequence", &s)], "target": { "parentType": "act", "parentId": a2 } }));
    let b = board(&env);
    assert!(kids(&b, &a1).is_empty());
    assert_eq!(kids(&b, &a2), vec![s.clone()]);
    assert_eq!(
        kids(&b, &s),
        vec![c1, c2],
        "children keep identities and order"
    );
    // Sequences cannot go into a sequence or the Parking Lot.
    let s2 = seq(&env, &a2, "Other");
    assert_eq!(
        env.err("story.move_items", json!({ "items": [item("sequence", &s)], "target": { "parentType": "sequence", "parentId": s2 } })),
        "validation.invalid_input"
    );
    assert_eq!(
        env.err(
            "story.park_items",
            json!({ "items": [item("sequence", &s)] })
        ),
        "validation.invalid_input"
    );
}

#[test]
fn fsd_story_010_reorder_act_with_descendants() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a1 = act(&env, "Act 1");
    let a2 = act(&env, "Act 2");
    let a3 = act(&env, "Act 3");
    let s3 = seq(&env, &a3, "Final Fight");
    let c = card(&env, &s3, "Platform lights die");
    env.ok("story.move_act", json!({ "id": a3, "beforeId": a2 }));
    let b = board(&env);
    assert_eq!(act_ids(&b), vec![a1, a3.clone(), a2]);
    assert_eq!(kids(&b, &s3), vec![c]);
    env.undo();
    assert_eq!(act_ids(&board(&env))[2], a3);
}

#[test]
fn fsd_story_011_multi_select_move_preserves_relative_order() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 1");
    let s1 = seq(&env, &a, "A");
    let s2 = seq(&env, &a, "B");
    let c: Vec<String> = (0..5).map(|i| card(&env, &s1, &format!("c{i}"))).collect();
    let d = card(&env, &s2, "d");
    // Selection given in a scrambled order: c3, c0, c4.
    env.ok(
        "story.move_items",
        json!({ "items": [item("card", &c[3]), item("card", &c[0]), item("card", &c[4])],
                "target": { "parentType": "sequence", "parentId": s2 }, "before": item("card", &d) }),
    );
    let b = board(&env);
    assert_eq!(
        kids(&b, &s2),
        vec![c[0].clone(), c[3].clone(), c[4].clone(), d]
    );
    assert_eq!(kids(&b, &s1), vec![c[1].clone(), c[2].clone()]);
    // One undo step for the whole group.
    env.undo();
    assert_eq!(kids(&board(&env), &s1), c);
}

#[test]
fn fsd_story_013_duplicate_is_independent_and_has_no_screenplay_link() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 1");
    let s = seq(&env, &a, "S");
    let orig = card(&env, &s, "Original");
    env.ok(
        "story.update_card",
        json!({ "id": orig, "sceneHeading": "EXT. STREET — DAY", "notes": "keep it short" }),
    );
    env.ok(
        "story.build_screenplay",
        json!({ "include": [{ "cardId": orig }], "destination": "newScreenplay" }),
    );
    let built = env.ok("story.card", json!({ "id": orig }));
    assert!(built["card"]["screenplaySceneId"].is_string());

    let dup = env.ok(
        "story.duplicate_items",
        json!({ "items": [item("card", &orig)] }),
    )["ids"][0]
        .as_str()
        .unwrap()
        .to_string();
    assert_ne!(dup, orig);
    assert_eq!(
        kids(&board(&env), &s),
        vec![orig.clone(), dup.clone()],
        "placed right after the original"
    );
    let d = env.ok("story.card", json!({ "id": dup }));
    assert_eq!(d["card"]["shortDescription"], "Original");
    assert_eq!(d["card"]["sceneHeading"], "EXT. STREET — DAY");
    assert_eq!(d["card"]["notes"], "keep it short");
    assert!(
        d["card"]["screenplaySceneId"].is_null(),
        "duplicate never inherits screenplay identity"
    );
    env.ok(
        "story.update_card",
        json!({ "id": dup, "shortDescription": "Alternate" }),
    );
    assert_eq!(
        env.ok("story.card", json!({ "id": orig }))["card"]["shortDescription"],
        "Original"
    );
}

#[test]
fn fsd_story_016_delete_is_recoverable_and_undoable() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 1");
    let s = seq(&env, &a, "S");
    let c1 = card(&env, &s, "one");
    let c2 = card(&env, &s, "two");
    let c3 = card(&env, &s, "three");
    env.ok(
        "story.delete_items",
        json!({ "items": [item("card", &c2)] }),
    );
    assert_eq!(kids(&board(&env), &s), vec![c1.clone(), c3.clone()]);
    env.undo();
    assert_eq!(
        kids(&board(&env), &s),
        vec![c1.clone(), c2.clone(), c3.clone()]
    );
    // Delete → Recently Deleted → restore to the same position.
    env.ok(
        "story.delete_items",
        json!({ "items": [item("card", &c2)] }),
    );
    let entry = trash_entry(&env, &c2);
    env.ok("trash.restore", json!({ "id": entry }));
    assert_eq!(
        kids(&board(&env), &s),
        vec![c1.clone(), c2.clone(), c3.clone()]
    );
    // Container gone → restore goes to Unassigned.
    env.ok(
        "story.delete_items",
        json!({ "items": [item("card", &c2)] }),
    );
    env.ok(
        "story.delete_sequence",
        json!({ "id": s, "mode": "deleteAll" }),
    );
    let entry = trash_entry(&env, &c2);
    env.ok("trash.restore", json!({ "id": entry }));
    let b = board(&env);
    assert_eq!(kids(&b, "unassigned"), vec![c2.clone()]);
    assert_eq!(
        env.ok("story.card", json!({ "id": c2 }))["location"],
        "Unassigned"
    );
    // Purge removes it for good.
    env.ok(
        "story.delete_items",
        json!({ "items": [item("card", &c2)] }),
    );
    let entry = trash_entry(&env, &c2);
    env.ok("trash.purge", json!({ "id": entry }));
    assert_eq!(
        count(
            &env,
            &format!("SELECT count(*) FROM story_scene_card WHERE id='{c2}'")
        ),
        0
    );
}

#[test]
fn deleting_linked_card_keeps_the_screenplay_scene() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 1");
    let c = card_in(&env, "act", Some(&a), "Street chase");
    env.ok(
        "story.update_card",
        json!({ "id": c, "sceneHeading": "EXT. STREET — DAY" }),
    );
    env.ok(
        "story.build_screenplay",
        json!({ "include": [{ "cardId": c }], "destination": "newScreenplay" }),
    );
    let d = env.ok("story.card", json!({ "id": c }));
    assert_eq!(d["linkedScene"]["number"], 1);
    assert_eq!(d["linkedScene"]["heading"], "EXT. STREET — DAY");
    env.ok("story.delete_items", json!({ "items": [item("card", &c)] }));
    assert_eq!(
        count(
            &env,
            "SELECT count(*) FROM screenplay_scene WHERE deleted_at IS NULL"
        ),
        1
    );
}

#[test]
fn act_delete_choices_recommend_moving_children() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a1 = act(&env, "Act 1");
    let a2 = act(&env, "Act 2 — The Discovery");
    let s = seq(&env, &a2, "First Investigation");
    let c = card(&env, &s, "Body discovered");
    let loose = card_in(&env, "act", Some(&a2), "Loose card");
    // Non-empty: a choice is required.
    assert_eq!(
        env.err("story.delete_act", json!({ "id": a2 })),
        "conflict.state"
    );
    env.ok(
        "story.delete_act",
        json!({ "id": a2, "mode": "moveContents", "moveToActId": a1 }),
    );
    let b = board(&env);
    assert_eq!(act_ids(&b), vec![a1.clone()]);
    assert_eq!(kids(&b, &a1), vec![s.clone(), loose.clone()]);
    assert_eq!(kids(&b, &s), vec![c.clone()]);
    // Empty act deletes without confirmation.
    let empty = act(&env, "Empty");
    env.ok("story.delete_act", json!({ "id": empty }));
    // Delete everything → one trash entry; restore brings children back.
    env.ok("story.delete_act", json!({ "id": a1, "mode": "deleteAll" }));
    assert!(board(&env)["acts"].as_array().unwrap().is_empty());
    let t = env.ok("trash.list", json!({}));
    assert!(
        t.as_array().unwrap().iter().all(|r| r["objectId"] != c),
        "children have no separate entries"
    );
    assert!(
        env.ok("search.query", json!({ "text": "Body discovered" }))
            .as_array()
            .unwrap()
            .is_empty()
    );
    env.ok("trash.restore", json!({ "id": trash_entry(&env, &a1) }));
    let b = board(&env);
    assert_eq!(kids(&b, &a1), vec![s.clone(), loose]);
    assert_eq!(kids(&b, &s), vec![c.clone()]);
    assert_eq!(
        env.ok("search.query", json!({ "text": "Body discovered" }))[0]["entityId"],
        c.as_str()
    );
    // Purge with descendants.
    env.ok("story.delete_act", json!({ "id": a1, "mode": "deleteAll" }));
    env.ok("trash.purge", json!({ "id": trash_entry(&env, &a1) }));
    assert_eq!(count(&env, "SELECT count(*) FROM story_scene_card"), 0);
    assert_eq!(count(&env, "SELECT count(*) FROM story_sequence"), 0);
}

#[test]
fn sequence_delete_moves_children_into_its_act_by_default() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 1");
    let first = card_in(&env, "act", Some(&a), "before");
    let s = seq(&env, &a, "Chase");
    let after = card_in(&env, "act", Some(&a), "after");
    let c1 = card(&env, &s, "c1");
    let c2 = card(&env, &s, "c2");
    assert_eq!(
        env.err("story.delete_sequence", json!({ "id": s })),
        "conflict.state"
    );
    env.ok(
        "story.delete_sequence",
        json!({ "id": s, "mode": "moveContents" }),
    );
    assert_eq!(kids(&board(&env), &a), vec![first, c1, c2, after]);
    // Restoring the sequence brings back the (now empty) container.
    env.ok("trash.restore", json!({ "id": trash_entry(&env, &s) }));
    assert!(kids(&board(&env), &a).contains(&s));
}

#[test]
fn restored_sequence_whose_act_is_gone_goes_to_unassigned() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 1");
    let s = seq(&env, &a, "Orphan");
    let c = card(&env, &s, "inside");
    env.ok(
        "story.delete_sequence",
        json!({ "id": s, "mode": "deleteAll" }),
    );
    env.ok("story.delete_act", json!({ "id": a }));
    env.ok("trash.purge", json!({ "id": trash_entry(&env, &a) }));
    env.ok("trash.restore", json!({ "id": trash_entry(&env, &s) }));
    let b = board(&env);
    assert_eq!(kids(&b, "unassigned"), vec![s.clone()]);
    assert_eq!(kids(&b, &s), vec![c.clone()]);
    // User drags it back into a new act.
    let a2 = act(&env, "Act 1 again");
    env.ok("story.move_items", json!({ "items": [item("sequence", &s)], "target": { "parentType": "act", "parentId": a2 } }));
    assert!(kids(&board(&env), "unassigned").is_empty());
}

#[test]
fn fsd_story_018_019_collapse_is_view_state_not_story_data() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 1");
    let s = seq(&env, &a, "S");
    card(&env, &s, "c");
    let before = env.ok("history.info", json!({}));
    env.ok("story.set_collapsed", json!({ "id": a, "collapsed": true }));
    env.ok("story.set_collapsed", json!({ "id": s, "collapsed": true }));
    let b = board(&env);
    assert_eq!(b["collapsedIds"].as_array().unwrap().len(), 2);
    assert_eq!(kids(&b, &s).len(), 1, "data unchanged");
    assert_eq!(
        env.ok("history.info", json!({})),
        before,
        "collapse never enters undo history"
    );
    env.ok(
        "story.set_collapsed",
        json!({ "id": a, "collapsed": false }),
    );
    assert_eq!(board(&env)["collapsedIds"], json!([s]));
}

// ---------------------------------------------------------------- beats

#[test]
fn fsd_story_029_convert_beat_keeps_original_as_converted() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 2");
    let s = seq(&env, &a, "First Investigation");
    let beat = id(&env.ok(
        "story.create_beat",
        json!({ "parent": { "parentType": "sequence", "parentId": s }, "text": "Body discovered", "color": "blue" }),
    ));
    let c = card(&env, &s, "Crime scene");
    let new_card = id(&env.ok("story.convert_beat", json!({ "id": beat })));
    let b = board(&env);
    assert_eq!(kids(&b, &s), vec![beat.clone(), new_card.clone(), c]);
    let beat_json = &b["acts"][0]["items"][0]["items"][0];
    assert_eq!(beat_json["state"], "converted");
    assert_eq!(beat_json["convertedSceneCardId"], new_card.as_str());
    assert_eq!(beat_json["color"], "blue");
    let d = env.ok("story.card", json!({ "id": new_card }));
    assert_eq!(d["card"]["shortDescription"], "Body discovered");
    assert_eq!(d["card"]["sourceBeatId"], beat.as_str());
    env.ok(
        "story.update_card",
        json!({ "id": new_card, "shortDescription": "Body discovered in the canal" }),
    );
    assert_eq!(
        env.err("story.convert_beat", json!({ "id": beat })),
        "conflict.state"
    );
    // Undo the edit and conversion restores an active beat.
    env.undo();
    env.undo();
    let b = board(&env);
    assert_eq!(b["acts"][0]["items"][0]["items"][0]["state"], "active");
    assert_eq!(
        env.err("story.update_beat", json!({ "id": beat, "color": "neon" })),
        "validation.invalid_input"
    );
}

#[test]
fn fsd_story_030_scene_to_beat_retains_source() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 1");
    let c = card_in(&env, "act", Some(&a), "Maya reveals what she saw");
    let beat = id(&env.ok("story.card_to_beat", json!({ "id": c })));
    let b = board(&env);
    assert_eq!(kids(&b, &a), vec![c.clone(), beat]);
    assert_eq!(
        b["acts"][0]["items"][1]["text"],
        "Maya reveals what she saw"
    );
    assert_eq!(
        env.ok("story.card", json!({ "id": c }))["card"]["shortDescription"],
        "Maya reveals what she saw"
    );
}

#[test]
fn beats_drag_anywhere() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 1");
    let s = seq(&env, &a, "S");
    let beat = id(&env.ok(
        "story.create_beat",
        json!({ "parent": { "parentType": "act", "parentId": a }, "text": "turn" }),
    ));
    env.ok("story.move_items", json!({ "items": [item("beat", &beat)], "target": { "parentType": "sequence", "parentId": s } }));
    assert_eq!(kids(&board(&env), &s), vec![beat.clone()]);
    env.ok(
        "story.move_items",
        json!({ "items": [item("beat", &beat)], "target": { "parentType": "parking" } }),
    );
    assert_eq!(kids(&board(&env), "parking"), vec![beat]);
}

// ---------------------------------------------------- build screenplay

#[test]
fn fsd_story_020_021_022_build_preview_order_selection_and_missing_headings() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let a1 = act(&env, "Act 1");
    let a2 = act(&env, "Act 2");
    let s = seq(&env, &a1, "Hero Introduction");
    let c1 = card(&env, &s, "Arjun steps off the bus");
    let c2 = card(&env, &s, "Old house");
    let c3 = card_in(&env, "act", Some(&a2), "Police station");
    let parked = card(&env, &s, "Dream at the lake");
    env.ok(
        "story.park_items",
        json!({ "items": [item("card", &parked)] }),
    );
    env.ok(
        "story.update_card",
        json!({ "id": c1, "sceneHeading": "EXT. BUS STATION — NIGHT" }),
    );
    env.ok(
        "story.update_card",
        json!({ "id": c3, "sceneHeading": "INT. POLICE STATION — DAY" }),
    );
    // Reorder before preview: c2 first.
    env.ok("story.move_items", json!({ "items": [item("card", &c2)], "target": { "parentType": "sequence", "parentId": s }, "before": item("card", &c1) }));

    let p = env.ok("story.build_preview", json!({}));
    let rows = p["rows"].as_array().unwrap();
    let order: Vec<&str> = rows.iter().map(|r| r["cardId"].as_str().unwrap()).collect();
    assert_eq!(
        order,
        vec![c2.as_str(), c1.as_str(), c3.as_str()],
        "board order; parking excluded"
    );
    assert_eq!(p["parkedCount"], 1);
    assert_eq!(rows[0]["headingValid"], false, "Heading needed");
    assert_eq!(rows[1]["headingValid"], true);
    assert!(p["screenplays"].as_array().unwrap().is_empty());

    // Including a card without a heading is refused and nothing is created.
    let code = env.err(
        "story.build_screenplay",
        json!({ "include": [{ "cardId": c2 }, { "cardId": c1 }], "destination": "newScreenplay" }),
    );
    assert_eq!(code, "validation.heading_needed");
    assert_eq!(count(&env, "SELECT count(*) FROM screenplay"), 0);
    // Parking Lot cards cannot be built.
    assert_eq!(
        env.err("story.build_screenplay", json!({ "include": [{ "cardId": parked, "heading": "INT. LAKE — DAY" }], "destination": "newScreenplay" })),
        "validation.invalid_input"
    );

    // Subset (exclude c2), args given out of order → built in board order.
    let r = env.ok(
        "story.build_screenplay",
        json!({ "include": [{ "cardId": c3 }, { "cardId": c1 }], "destination": "newScreenplay", "descriptionMode": "actionText" }),
    );
    assert_eq!(r["sceneCount"], 2);
    assert_eq!(r["createdScreenplay"], true);
    let draft = r["draftId"].as_str().unwrap();
    let headings: Vec<String> = env
        .core
        .project()
        .unwrap()
        .store
        .read(|c| {
            let mut st = c.prepare(
                "SELECT heading FROM screenplay_scene WHERE draft_id=?1 ORDER BY position",
            )?;
            Ok(st
                .query_map([draft], |r| r.get(0))?
                .collect::<Result<Vec<String>, _>>()?)
        })
        .unwrap();
    assert_eq!(
        headings,
        vec!["EXT. BUS STATION — NIGHT", "INT. POLICE STATION — DAY"]
    );
    assert_eq!(
        count(
            &env,
            "SELECT count(*) FROM screenplay_element WHERE element_type='action' AND text='Arjun steps off the bus'"
        ),
        1,
        "description inserted as temporary action text"
    );
    assert_eq!(
        count(
            &env,
            &format!("SELECT count(*) FROM screenplay WHERE current_draft_id='{draft}'")
        ),
        1
    );
    let first = r["firstSceneId"].as_str().unwrap();
    assert_eq!(
        count(
            &env,
            &format!(
                "SELECT count(*) FROM screenplay_scene WHERE id='{first}' AND lineage_id <> ''"
            )
        ),
        1
    );
    assert_eq!(
        env.ok("story.card", json!({ "id": c1 }))["card"]["screenplaySceneId"],
        first
    );
}

#[test]
fn fsd_story_022_heading_supplied_inline_is_saved_on_the_card() {
    let env = TestEnv::with_project("Film", "Short Film");
    let a = act(&env, "Act 1");
    let c = card_in(&env, "act", Some(&a), "The chase ends");
    let r = env.ok(
        "story.build_screenplay",
        json!({ "include": [{ "cardId": c, "heading": "ext. level crossing - night" }], "destination": "newScreenplay" }),
    );
    assert_eq!(r["draftName"], "Draft 1 — from Story Board");
    assert_eq!(
        env.ok("story.card", json!({ "id": c }))["card"]["sceneHeading"],
        "ext. level crossing - night"
    );
    // Planning-note mode (default): the description is a note, action is left for writing.
    assert_eq!(
        count(
            &env,
            "SELECT count(*) FROM screenplay_element WHERE element_type='note' AND text='The chase ends'"
        ),
        1
    );
    // The heading lives on the scene row; the body never holds a heading
    // element (Screenplay module rule, shared with import).
    assert_eq!(
        count(
            &env,
            "SELECT count(*) FROM screenplay_scene WHERE heading='EXT. LEVEL CROSSING - NIGHT'"
        ),
        1
    );
    assert_eq!(
        count(
            &env,
            "SELECT count(*) FROM screenplay_element WHERE element_type='scene_heading'"
        ),
        0
    );
    // Body order: planning note, then an empty action line to write into.
    let types: Vec<String> = env
        .core
        .project()
        .unwrap()
        .store
        .read(|c| {
            let mut st =
                c.prepare("SELECT element_type FROM screenplay_element ORDER BY position")?;
            Ok(st
                .query_map([], |r| r.get(0))?
                .collect::<Result<Vec<String>, _>>()?)
        })
        .unwrap();
    assert_eq!(types, vec!["note", "action"]);
    // The built draft opens in the Screenplay workspace like any other draft.
    let doc = env.ok("screenplay.document", json!({ "draftId": r["draftId"] }));
    assert_eq!(doc["scenes"][0]["heading"], "EXT. LEVEL CROSSING - NIGHT");
    assert_eq!(
        count(&env, "SELECT count(*) FROM screenplay WHERE format='Short'"),
        1
    );
}

#[test]
fn fsd_story_023_repeat_build_creates_a_new_draft_never_overwrites() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 1");
    let c1 = card_in(&env, "act", Some(&a), "one");
    let c2 = card_in(&env, "act", Some(&a), "two");
    for c in [&c1, &c2] {
        env.ok(
            "story.update_card",
            json!({ "id": c, "sceneHeading": "INT. ROOM — DAY" }),
        );
    }
    let first = env.ok(
        "story.build_screenplay",
        json!({ "include": [{ "cardId": c1 }, { "cardId": c2 }], "destination": "newScreenplay" }),
    );
    let sp = first["screenplayId"].as_str().unwrap().to_string();
    let d1 = first["draftId"].as_str().unwrap().to_string();
    let elements_before = count(
        &env,
        &format!(
            "SELECT count(*) FROM screenplay_element e JOIN screenplay_scene s ON s.id=e.scene_id WHERE s.draft_id='{d1}'"
        ),
    );

    let p = env.ok("story.build_preview", json!({}));
    assert_eq!(p["screenplays"][0]["id"], sp.as_str());
    assert_eq!(
        p["screenplays"][0]["nextDraftName"],
        "Draft 2 — from Story Board"
    );
    // NewDraft requires the target screenplay.
    assert_eq!(
        env.err(
            "story.build_screenplay",
            json!({ "include": [{ "cardId": c1 }], "destination": "newDraft" })
        ),
        "validation.invalid_input"
    );
    let second = env.ok(
        "story.build_screenplay",
        json!({ "include": [{ "cardId": c2 }, { "cardId": c1 }], "destination": "newDraft", "screenplayId": sp }),
    );
    assert_eq!(second["createdScreenplay"], false);
    assert_eq!(second["screenplayId"], sp.as_str());
    assert_ne!(second["draftId"], d1.as_str());
    assert_eq!(count(&env, "SELECT count(*) FROM screenplay_draft"), 2);
    assert_eq!(
        count(
            &env,
            &format!(
                "SELECT count(*) FROM screenplay_element e JOIN screenplay_scene s ON s.id=e.scene_id WHERE s.draft_id='{d1}'"
            )
        ),
        elements_before,
        "first draft untouched"
    );
    assert_eq!(
        count(
            &env,
            &format!("SELECT count(*) FROM screenplay WHERE current_draft_id='{d1}'")
        ),
        1,
        "current draft unchanged"
    );
    // One screenplay per project: a second "new screenplay" is refused (the
    // Screenplay workspace shows exactly one) and nothing is created.
    assert_eq!(
        env.err(
            "story.build_screenplay",
            json!({ "include": [{ "cardId": c1 }], "destination": "newScreenplay" }),
        ),
        "validation.destination"
    );
    assert_eq!(count(&env, "SELECT count(*) FROM screenplay"), 1);
    // The whole build is one undo step.
    env.undo();
    assert_eq!(
        count(
            &env,
            "SELECT count(*) FROM screenplay_draft WHERE deleted_at IS NULL"
        ),
        1
    );
    assert_eq!(count(&env, "SELECT count(*) FROM screenplay"), 1);
}

#[test]
fn fsd_story_024_card_edits_after_build_do_not_change_the_script() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 1");
    let c = card_in(&env, "act", Some(&a), "Chase");
    env.ok(
        "story.update_card",
        json!({ "id": c, "sceneHeading": "EXT. STREET — DAY" }),
    );
    env.ok(
        "story.build_screenplay",
        json!({ "include": [{ "cardId": c }], "destination": "newScreenplay" }),
    );
    env.ok(
        "story.update_card",
        json!({ "id": c, "sceneHeading": "INT. CAR — NIGHT", "shortDescription": "Different" }),
    );
    assert_eq!(
        count(
            &env,
            "SELECT count(*) FROM screenplay_scene WHERE heading='EXT. STREET — DAY'"
        ),
        1
    );
    assert_eq!(
        count(
            &env,
            "SELECT count(*) FROM screenplay_element WHERE text='Different'"
        ),
        0
    );
}

#[test]
fn fsd_story_025_applying_board_order_to_a_written_script_requires_confirmation() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 1");
    let c1 = card_in(&env, "act", Some(&a), "one");
    let c2 = card_in(&env, "act", Some(&a), "two");
    for c in [&c1, &c2] {
        env.ok(
            "story.update_card",
            json!({ "id": c, "sceneHeading": "INT. ROOM — DAY" }),
        );
    }
    env.ok(
        "story.build_screenplay",
        json!({ "include": [{ "cardId": c1 }, { "cardId": c2 }], "destination": "newScreenplay", "descriptionMode": "actionText" }),
    );
    // Board reorder alone never touches the script.
    env.ok("story.move_items", json!({ "items": [item("card", &c2)], "target": { "parentType": "act", "parentId": a }, "before": item("card", &c1) }));
    let s1: String = env.ok("story.card", json!({ "id": c1 }))["card"]["screenplaySceneId"]
        .as_str()
        .unwrap()
        .into();
    assert_eq!(
        count(
            &env,
            &format!("SELECT position FROM screenplay_scene WHERE id='{s1}'")
        ),
        1
    );

    let p = env.ok("story.order_preview", json!({}));
    assert_eq!(p["hasWrittenScenes"], true);
    assert_eq!(p["moves"].as_array().unwrap().len(), 2);
    assert_eq!(
        env.err("story.apply_order", json!({})),
        "conflict.confirm_required"
    );
    env.ok("story.apply_order", json!({ "confirmed": true }));
    assert_eq!(
        count(
            &env,
            &format!("SELECT position FROM screenplay_scene WHERE id='{s1}'")
        ),
        2
    );
    assert!(
        env.ok("story.order_preview", json!({}))["moves"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

// ------------------------------------------------------------ characters

#[test]
fn characters_create_edit_relationships_links_and_scene_usage() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let arjun = id(&env.ok(
        "story.create_character",
        json!({ "name": "Arjun", "roleLabel": "Protagonist" }),
    ));
    let ravi = id(&env.ok(
        "story.create_character",
        json!({ "name": "Ravi", "description": "Arjun's son" }),
    ));
    assert_eq!(
        env.err("story.create_character", json!({ "name": " " })),
        "validation.required"
    );
    let list = env.ok("story.characters", json!({}));
    assert_eq!(list.as_array().unwrap().len(), 2);
    assert_eq!(list[0]["roleLabel"], "Protagonist");

    let rel = id(&env.ok(
        "story.create_relationship",
        json!({ "fromCharacterId": arjun, "toCharacterId": ravi, "relationshipType": "father of" }),
    ));
    assert_eq!(
        env.err(
            "story.create_relationship",
            json!({ "fromCharacterId": arjun, "toCharacterId": arjun, "relationshipType": "self" })
        ),
        "validation.invalid_input"
    );
    let rels = env.ok("story.relationships", json!({}));
    assert_eq!(rels[0]["fromName"], "Arjun");
    assert_eq!(rels[0]["relationshipType"], "father of");
    env.ok(
        "story.update_relationship",
        json!({ "id": rel, "relationshipType": "estranged father of" }),
    );

    // Manual card link (reference only).
    let a = act(&env, "Act 1");
    let c = card_in(&env, "act", Some(&a), "Arjun confronts Ravi");
    env.ok(
        "story.update_card",
        json!({ "id": c, "sceneHeading": "INT. HOUSE — NIGHT" }),
    );
    env.ok(
        "story.link_character_card",
        json!({ "characterId": ravi, "cardId": c }),
    );
    assert_eq!(
        env.ok("story.card", json!({ "id": c }))["card"]["characterIds"],
        json!([ravi])
    );

    // Scene usage is derived from screenplay character cues.
    let r = env.ok(
        "story.build_screenplay",
        json!({ "include": [{ "cardId": c }], "destination": "newScreenplay" }),
    );
    let scene = r["firstSceneId"].as_str().unwrap().to_string();
    env.core
        .project()
        .unwrap()
        .store
        .with_writer(|w| {
            w.execute(
                "INSERT INTO screenplay_element(id, scene_id, position, element_type, text, created_at, updated_at)
                 VALUES (?1, ?2, 10, 'character', 'RAVI (V.O.)', 0, 0)",
                [openframe_domain::new_id(), scene.clone()],
            )?;
            Ok(())
        })
        .unwrap();
    let d = env.ok("story.character", json!({ "id": ravi }));
    assert_eq!(d["character"]["sceneCount"], 1);
    assert_eq!(d["scenes"][0]["number"], 1);
    assert_eq!(
        d["relationships"][0]["relationshipType"],
        "estranged father of"
    );
    assert_eq!(d["linkedCards"][0]["cardId"], c.as_str());
    let usage = env.ok("story.character_usage", json!({ "id": ravi }));
    assert_eq!(usage["inUse"], true);
    assert_eq!(usage["screenplayScenes"], 1);

    // Rename never rewrites screenplay text.
    env.ok(
        "story.update_character",
        json!({ "id": ravi, "name": "Ravi Kumar" }),
    );
    assert_eq!(
        count(
            &env,
            "SELECT count(*) FROM screenplay_element WHERE text='RAVI (V.O.)'"
        ),
        1
    );

    // Archive instead of delete; archived hidden by default.
    env.ok(
        "story.set_character_archived",
        json!({ "id": ravi, "archived": true }),
    );
    assert_eq!(
        env.ok("story.characters", json!({}))
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        env.ok("story.characters", json!({ "includeArchived": true }))
            .as_array()
            .unwrap()
            .len(),
        2
    );

    // Search finds characters and cards.
    let hits = env.ok("search.query", json!({ "text": "Protagonist" }));
    assert_eq!(hits[0]["entityId"], arjun.as_str());
    let hits = env.ok("search.query", json!({ "text": "confronts" }));
    assert_eq!(hits[0]["entityId"], c.as_str());
    assert_eq!(hits[0]["nav"]["cardId"], c.as_str());

    // Delete → recoverable → purge removes relationships and links.
    env.ok("story.delete_character", json!({ "id": ravi }));
    assert!(
        env.ok("story.relationships", json!({}))
            .as_array()
            .unwrap()
            .is_empty()
    );
    env.ok("trash.purge", json!({ "id": trash_entry(&env, &ravi) }));
    assert_eq!(
        count(&env, "SELECT count(*) FROM story_character_relationship"),
        0
    );
    assert_eq!(
        count(&env, "SELECT count(*) FROM story_character_card_link"),
        0
    );
}

#[test]
fn character_image_is_a_managed_copy() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    png.extend_from_slice(&64u32.to_be_bytes());
    png.extend_from_slice(&48u32.to_be_bytes());
    let path = env.write_file("arjun.png", &png);
    let ch = id(&env.ok(
        "story.create_character",
        json!({ "name": "Arjun", "imagePath": path }),
    ));
    let list = env.ok("story.characters", json!({}));
    assert_eq!(list[0]["image"]["storageMode"], "managed");
    assert_eq!(list[0]["image"]["width"], 64);
    let txt = env.write_file("notes.txt", b"x");
    assert_eq!(
        env.err(
            "story.set_character_image",
            json!({ "id": ch, "path": txt })
        ),
        "validation.invalid_input"
    );
    env.ok("story.clear_character_image", json!({ "id": ch }));
    assert!(env.ok("story.characters", json!({}))[0]["image"].is_null());
    env.undo();
    assert_eq!(
        env.ok("story.characters", json!({}))[0]["image"]["available"],
        true,
        "undo brings the image back"
    );
}

// -------------------------------------------------------------- timeline

#[test]
fn story_timeline_groups_by_day_and_never_reorders() {
    let env = TestEnv::with_project("Film", "Feature Film");
    assert!(
        env.ok("story.timeline", json!({}))["groups"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let a = act(&env, "Act 1");
    let cards: Vec<String> = (0..3)
        .map(|i| card_in(&env, "act", Some(&a), &format!("scene {i}")))
        .collect();
    let include: Vec<Value> = cards
        .iter()
        .map(|c| json!({ "cardId": c, "heading": format!("INT. ROOM {c} — DAY") }))
        .collect();
    env.ok(
        "story.build_screenplay",
        json!({ "include": include, "destination": "newScreenplay" }),
    );
    let t = env.ok("story.timeline", json!({}));
    assert_eq!(t["groups"].as_array().unwrap().len(), 1);
    assert!(t["groups"][0]["storyDay"].is_null(), "Unassigned group");
    let scenes: Vec<String> = t["groups"][0]["scenes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["sceneId"].as_str().unwrap().to_string())
        .collect();
    env.ok(
        "story.assign_story_day",
        json!({ "sceneIds": [scenes[2]], "storyDay": "1" }),
    );
    env.ok(
        "story.assign_story_day",
        json!({ "sceneIds": [scenes[0]], "storyDay": "2" }),
    );
    env.ok(
        "story.set_time_note",
        json!({ "sceneId": scenes[0], "timeNote": "night" }),
    );
    let t = env.ok("story.timeline", json!({}));
    let g = t["groups"].as_array().unwrap();
    assert_eq!(g[0]["storyDay"], "1");
    assert_eq!(g[0]["scenes"][0]["number"], 3);
    assert_eq!(g[1]["storyDay"], "2");
    assert_eq!(g[1]["scenes"][0]["timeNote"], "night");
    assert!(g[2]["storyDay"].is_null());
    // Scene order is untouched.
    let positions: Vec<i64> = scenes
        .iter()
        .map(|s| {
            count(
                &env,
                &format!("SELECT position FROM screenplay_scene WHERE id='{s}'"),
            )
        })
        .collect();
    assert_eq!(positions, vec![1, 2, 3]);
    env.ok(
        "story.assign_story_day",
        json!({ "sceneIds": [scenes[2]], "storyDay": "" }),
    );
    assert_eq!(
        env.ok("story.timeline", json!({}))["groups"][0]["storyDay"],
        "2"
    );
}

// -------------------------------------------------------------- episodic

#[test]
fn episodes_scope_story_content_and_season_board_reorders() {
    let env = TestEnv::with_project("Monsoon Diaries", "Series");
    let series = env.ok("story.series", json!({}));
    assert_eq!(series["isEpisodic"], true);
    let s1 = id(&env.ok("story.create_season", json!({})));
    assert_eq!(
        env.ok("story.series", json!({}))["seasons"][0]["title"],
        "Season 1"
    );
    let e1 = id(&env.ok(
        "story.create_episode",
        json!({ "seasonId": s1, "title": "Pilot — First Rain" }),
    ));
    let e2 = id(&env.ok(
        "story.create_episode",
        json!({ "seasonId": s1, "title": "The Flood", "status": "writing" }),
    ));
    assert_eq!(
        env.err(
            "story.create_episode",
            json!({ "seasonId": s1, "title": "x", "status": "Banana" })
        ),
        "validation.invalid_input"
    );
    env.ok(
        "story.move_episode",
        json!({ "id": e2, "seasonId": s1, "beforeId": e1 }),
    );
    let eps = &env.ok("story.series", json!({}))["seasons"][0]["episodes"];
    assert_eq!(eps[0]["id"], e2.as_str());
    assert_eq!(eps[0]["number"], 1);
    assert_eq!(eps[0]["status"], "Writing");

    // Story content is scoped per episode.
    let a1 = id(&env.ok(
        "story.create_act",
        json!({ "episodeId": e1, "title": "Teaser" }),
    ));
    let c1 = id(&env.ok(
        "story.create_card",
        json!({ "episodeId": e1, "shortDescription": "Rain starts" }),
    ));
    env.ok(
        "story.create_act",
        json!({ "episodeId": e2, "title": "Cold open" }),
    );
    let b1 = env.ok("story.board", json!({ "episodeId": e1 }));
    assert_eq!(act_ids(&b1), vec![a1.clone()]);
    assert_eq!(kids(&b1, &a1), vec![c1.clone()]);
    assert_eq!(
        env.ok("story.board", json!({ "episodeId": e2 }))["acts"][0]["title"],
        "Cold open"
    );
    assert!(
        env.ok("story.board", json!({}))["acts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    // Cross-episode moves are refused.
    let b2 = env.ok("story.board", json!({ "episodeId": e2 }));
    let a2 = b2["acts"][0]["id"].as_str().unwrap();
    assert_eq!(
        env.err("story.move_items", json!({ "items": [item("card", &c1)], "target": { "parentType": "act", "parentId": a2 } })),
        "validation.invalid_input"
    );
    // Search nav carries the episode.
    let hit = &env.ok("search.query", json!({ "text": "Rain starts" }))[0];
    assert_eq!(hit["nav"]["episodeId"], e1.as_str());

    // Duplicate with story content: new identities.
    let dup = id(&env.ok(
        "story.duplicate_episode",
        json!({ "id": e1, "copyStory": true }),
    ));
    let bd = env.ok("story.board", json!({ "episodeId": dup }));
    assert_eq!(bd["acts"][0]["title"], "Teaser");
    assert_ne!(bd["acts"][0]["id"], a1.as_str());
    assert_eq!(bd["acts"][0]["items"][0]["shortDescription"], "Rain starts");
    assert_ne!(bd["acts"][0]["items"][0]["id"], c1.as_str());

    // Current-episode choice is per-user view state.
    env.ok("story.set_current_episode", json!({ "episodeId": e2 }));
    assert_eq!(
        env.ok("story.view_state", json!({}))["episodeId"],
        e2.as_str()
    );

    // Delete season keeps episodes (moved to no season); delete episode is recoverable.
    env.ok("story.delete_season", json!({ "id": s1 }));
    let series = env.ok("story.series", json!({}));
    assert!(series["seasons"].as_array().unwrap().is_empty());
    assert_eq!(series["unseasoned"].as_array().unwrap().len(), 3);
    env.ok("story.delete_episode", json!({ "id": e1 }));
    // Its content still exists, so it can't be purged.
    assert_eq!(
        env.err("trash.purge", json!({ "id": trash_entry(&env, &e1) })),
        "conflict.state"
    );
    env.ok("trash.restore", json!({ "id": trash_entry(&env, &e1) }));
    assert_eq!(
        env.ok("story.board", json!({ "episodeId": e1 }))["acts"][0]["id"],
        a1.as_str()
    );
}

// ------------------------------------------------- permissions & misc

#[test]
fn viewers_and_commenters_cannot_change_the_story() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 1");
    let c = card_in(&env, "act", Some(&a), "x");
    for role in [Role::Viewer, Role::Commenter] {
        let who = env.actor_with_role(role);
        let e = env
            .call_as(&who, "story.create_act", json!({ "title": "Nope" }))
            .unwrap_err();
        assert_eq!(e.code.0, "permission.denied");
        let e = env
            .call_as(
                &who,
                "story.delete_items",
                json!({ "items": [item("card", &c)] }),
            )
            .unwrap_err();
        assert_eq!(e.code.0, "permission.denied");
        let e = env
            .call_as(
                &who,
                "story.build_screenplay",
                json!({ "include": [{ "cardId": c }], "destination": "newScreenplay" }),
            )
            .unwrap_err();
        assert_eq!(e.code.0, "permission.denied");
        // Reading and personal collapse state are fine.
        assert!(env.call_as(&who, "story.board", json!({})).is_ok());
        assert!(
            env.call_as(
                &who,
                "story.set_collapsed",
                json!({ "id": a, "collapsed": true })
            )
            .is_ok()
        );
    }
}

#[test]
fn attachments_on_cards_are_copied_by_duplicate_and_survive_undo() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 1");
    let c = card_in(&env, "act", Some(&a), "Station");
    let f = env.write_file("station.jpg", b"\xFF\xD8\xFF\xE0fakejpeg");
    let att = env.ok(
        "story.add_attachment",
        json!({ "ownerType": "scene_card", "ownerId": c, "path": f }),
    );
    assert_eq!(att["asset"]["storageMode"], "managed");
    let dup = env.ok(
        "story.duplicate_items",
        json!({ "items": [item("card", &c)] }),
    )["ids"][0]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        env.ok("story.card", json!({ "id": dup }))["card"]["attachments"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    env.ok("story.remove_attachment", json!({ "id": att["id"] }));
    assert!(
        env.ok("story.card", json!({ "id": c }))["card"]["attachments"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    env.undo();
    assert_eq!(
        env.ok("story.card", json!({ "id": c }))["card"]["attachments"][0]["asset"]["available"],
        true
    );
}

#[test]
fn search_indexes_story_objects_and_drops_deleted_ones() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = act(&env, "Act 1 — The Return");
    let s = seq(&env, &a, "Railway Station Return");
    let b = id(&env.ok("story.create_beat", json!({ "parent": { "parentType": "sequence", "parentId": s }, "text": "Platform whistle" })));
    let hit = &env.ok("search.query", json!({ "text": "Railway" }))[0];
    assert_eq!(hit["entityId"], s.as_str());
    assert_eq!(hit["nav"]["workspace"], "story");
    assert_eq!(
        env.ok("search.query", json!({ "text": "whistle" }))[0]["nav"]["beatId"],
        b.as_str()
    );
    env.ok("story.delete_items", json!({ "items": [item("beat", &b)] }));
    assert!(
        env.ok("search.query", json!({ "text": "whistle" }))
            .as_array()
            .unwrap()
            .is_empty()
    );
    env.ok("story.update_act", json!({ "id": a, "title": "Act One" }));
    assert_eq!(
        env.ok("search.query", json!({ "text": "One" }))[0]["title"],
        "Act One"
    );
}
