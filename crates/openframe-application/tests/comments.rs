//! Comments (FSD §23, §94; FSD-SCRIPT-029..032) and Private Notes
//! (FSD-SCRIPT-033; Security §8, §10.2) through the public operation registry.

use openframe_application::MutationMeta;
use openframe_domain::{Capability, Role, new_id};
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

struct Ctx {
    env: TestEnv,
    draft: String,
    scene: String,
    element: String,
}

/// A screenplay with scene "INT. POLICE STATION — NIGHT" and the action line
/// "He places the RED FOLDER on the desk."
fn setup() -> Ctx {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let created = env.ok("screenplay.create", json!({}));
    let draft = created["draft"]["id"].as_str().unwrap().to_string();
    let doc = env.ok("screenplay.document", json!({ "draftId": draft }));
    let scene = doc["scenes"][0]["id"].as_str().unwrap().to_string();
    let element = doc["scenes"][0]["elements"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    env.ok(
        "screenplay.apply_edits",
        json!({ "draftId": draft, "ops": [
            { "op": "updateScene", "id": scene, "heading": "INT. POLICE STATION — NIGHT" },
            { "op": "updateElement", "id": element, "text": "He places the RED FOLDER on the desk." },
        ] }),
    );
    Ctx {
        env,
        draft,
        scene,
        element,
    }
}

impl Ctx {
    fn edit(&self, ops: Value) {
        self.env.ok(
            "screenplay.apply_edits",
            json!({ "draftId": self.draft, "ops": ops }),
        );
    }
    fn text_comment(&self) -> Value {
        // "RED FOLDER" = UTF-16 offsets 14..24
        self.env.ok(
            "comment.create",
            json!({ "targetType": "screenplay_element", "targetId": self.element, "anchor": { "start": 14, "end": 24 },
                    "body": "Should we establish the folder earlier, in Scene 9?" }),
        )
    }
    fn threads(&self) -> Vec<Value> {
        self.env
            .ok("comment.list", json!({ "draftId": self.draft }))
            .as_array()
            .unwrap()
            .clone()
    }
    fn thread(&self, id: &Value) -> Value {
        self.threads()
            .into_iter()
            .find(|t| &t["comment"]["id"] == id)
            .unwrap()
    }
}

#[test]
fn fsd_script_029_text_anchored_comment_follows_edits_and_flags_context_moved() {
    let c = setup();
    let t = c.text_comment();
    let cm = &t["comment"];
    assert_eq!(cm["quotedText"], "RED FOLDER");
    assert_eq!(cm["anchor"]["elementId"], c.element);
    assert_eq!(cm["anchor"]["start"], 14);
    assert_eq!(cm["sceneId"], c.scene);
    assert_eq!(cm["sceneNumber"], 1);
    assert_eq!(cm["contextMoved"], false);
    let id = cm["id"].clone();
    // Text inserted before the anchor: the anchor moves with the text.
    c.edit(json!([{ "op": "updateElement", "id": c.element, "text": "Slowly, he places the RED FOLDER on the desk." }]));
    let now = c.thread(&id);
    assert_eq!(now["comment"]["anchor"]["start"], 22);
    assert_eq!(now["comment"]["contextMoved"], false);
    // Enter splits the element: the quoted text now lives in another element of the scene.
    let tail = new_id();
    c.edit(json!([
        { "op": "updateElement", "id": c.element, "text": "Slowly, he places" },
        { "op": "insertElement", "id": tail, "sceneId": c.scene, "index": 1, "elementType": "action", "text": "the RED FOLDER on the desk." },
    ]));
    let now = c.thread(&id);
    assert_eq!(now["comment"]["targetId"], tail);
    assert_eq!(now["comment"]["anchor"]["start"], 4);
    assert_eq!(now["comment"]["contextMoved"], false);
    // The text disappears: flagged "context moved", linked to the nearest surviving scene, never lost.
    c.edit(json!([{ "op": "updateElement", "id": tail, "text": "the envelope on the desk." }]));
    let now = c.thread(&id);
    assert_eq!(now["comment"]["contextMoved"], true);
    assert_eq!(now["comment"]["nearestSceneId"], c.scene);
    assert_eq!(now["comment"]["quotedText"], "RED FOLDER");
    // Undo restores both the text and the anchor state.
    c.env.undo();
    assert_eq!(c.thread(&id)["comment"]["contextMoved"], false);
    // The element is deleted outright: still listed, flagged.
    c.edit(json!([{ "op": "deleteElement", "id": tail }]));
    let now = c.thread(&id);
    assert_eq!(now["comment"]["contextMoved"], true);
    assert_eq!(now["comment"]["targetDeleted"], true);
    // Typing the text again in the scene re-attaches it.
    c.edit(json!([{ "op": "updateElement", "id": c.element, "text": "He finds the RED FOLDER again." }]));
    let now = c.thread(&id);
    assert_eq!(now["comment"]["contextMoved"], false);
    assert_eq!(now["comment"]["targetId"], c.element);
    assert_eq!(now["comment"]["anchor"]["start"], 13);
}

#[test]
fn anchors_are_validated() {
    let c = setup();
    for anchor in [
        json!({ "start": 5, "end": 5 }),
        json!({ "start": 30, "end": 99 }),
    ] {
        assert_eq!(
            c.env.err("comment.create", json!({ "targetType": "screenplay_element", "targetId": c.element, "anchor": anchor, "body": "x" })),
            "validation.invalid_input"
        );
    }
    assert_eq!(
        c.env.err("comment.create", json!({ "targetType": "screenplay_scene", "targetId": c.scene, "anchor": { "start": 0, "end": 2 }, "body": "x" })),
        "validation.invalid_input"
    );
    assert_eq!(
        c.env.err(
            "comment.create",
            json!({ "targetType": "screenplay_scene", "targetId": c.scene, "body": "  " })
        ),
        "validation.required"
    );
    assert_eq!(
        c.env.err(
            "comment.create",
            json!({ "targetType": "sys_undo", "targetId": c.scene, "body": "x" })
        ),
        "validation.invalid_input"
    );
    assert_eq!(
        c.env.err(
            "comment.create",
            json!({ "targetType": "private_note", "targetId": c.scene, "body": "x" })
        ),
        "validation.invalid_input"
    );
    assert_eq!(
        c.env.err(
            "comment.create",
            json!({ "targetType": "screenplay_scene", "targetId": new_id(), "body": "x" })
        ),
        "not_found.item"
    );
}

#[test]
fn fsd_script_030_031_032_scene_comment_reply_resolve_keeps_history() {
    let c = setup();
    let t = c.env.ok("comment.create", json!({ "targetType": "screenplay_scene", "targetId": c.scene, "body": "Lighting: practical-only scene?" }));
    let id = t["comment"]["id"].clone();
    assert!(
        t["comment"]["anchor"].is_null(),
        "a scene comment has no text range"
    );
    let other = c.env.other_user(Role::Commenter);
    let r = c
        .env
        .call_as(
            &other,
            "comment.reply",
            json!({ "parentId": id, "body": "Yes — practicals only." }),
        )
        .unwrap();
    assert_eq!(r["replies"].as_array().unwrap().len(), 1);
    assert_eq!(r["replies"][0]["authorName"], "Collaborator");
    assert_eq!(r["replies"][0]["parentId"], id);
    // Replies can't be replied to (single-level threads).
    let reply_id = r["replies"][0]["id"].clone();
    assert_eq!(
        c.env.err(
            "comment.reply",
            json!({ "parentId": reply_id, "body": "x" })
        ),
        "validation.invalid_input"
    );
    // Open → In Discussion → Resolved → reopened.
    let d = c.env.ok(
        "comment.update",
        json!({ "id": id, "status": "In Discussion" }),
    );
    assert_eq!(d["comment"]["status"], "In Discussion");
    assert_eq!(
        c.env
            .err("comment.update", json!({ "id": id, "status": "Resolved" })),
        "validation.invalid_input"
    );
    let res = c.env.ok("comment.resolve", json!({ "id": id }));
    assert_eq!(res["comment"]["status"], "Resolved");
    assert!(res["comment"]["resolvedAt"].as_i64().is_some());
    let open = c.env.ok(
        "comment.list",
        json!({ "draftId": c.draft, "status": "open" }),
    );
    assert!(open.as_array().unwrap().is_empty());
    let resolved = c.env.ok(
        "comment.list",
        json!({ "draftId": c.draft, "status": "resolved" }),
    );
    assert_eq!(
        resolved.as_array().unwrap().len(),
        1,
        "resolved comments stay in history"
    );
    assert_eq!(resolved[0]["replies"].as_array().unwrap().len(), 1);
    let re = c.env.ok("comment.reopen", json!({ "id": id }));
    assert_eq!(re["comment"]["status"], "Open");
    assert!(re["comment"]["resolvedAt"].is_null());
    // Undo/redo of resolve.
    c.env.ok("comment.resolve", json!({ "id": id }));
    c.env.undo();
    assert_eq!(c.thread(&id)["comment"]["status"], "Open");
    c.env.redo();
    assert_eq!(c.thread(&id)["comment"]["status"], "Resolved");
}

#[test]
fn comment_permissions_follow_roles() {
    let c = setup();
    let viewer = c.env.actor_with_role(Role::Viewer);
    let e = c
        .env
        .call_as(
            &viewer,
            "comment.create",
            json!({ "targetType": "screenplay_scene", "targetId": c.scene, "body": "x" }),
        )
        .unwrap_err();
    assert_eq!(e.code.0, "permission.denied", "viewers cannot comment");
    let commenter = c.env.other_user(Role::Commenter);
    let t = c
        .env
        .call_as(
            &commenter,
            "comment.create",
            json!({ "targetType": "screenplay_scene", "targetId": c.scene, "body": "Mine" }),
        )
        .unwrap();
    let id = t["comment"]["id"].clone();
    assert_eq!(t["comment"]["isMine"], true);
    // The owner can't rewrite someone else's words but may resolve and delete.
    assert_eq!(
        c.env
            .err("comment.update", json!({ "id": id, "body": "Rewritten" })),
        "permission.denied"
    );
    c.env
        .call_as(
            &commenter,
            "comment.update",
            json!({ "id": id, "body": "Edited by me" }),
        )
        .unwrap();
    c.env
        .call_as(&commenter, "comment.resolve", json!({ "id": id }))
        .unwrap();
    // Another commenter can't delete it; the author can.
    let other = c.env.other_user(Role::Commenter);
    assert_eq!(
        c.env
            .call_as(&other, "comment.delete", json!({ "id": id }))
            .unwrap_err()
            .code
            .0,
        "permission.denied"
    );
    c.env
        .call_as(&commenter, "comment.delete", json!({ "id": id }))
        .unwrap();
    assert!(c.threads().is_empty());
}

#[test]
fn deleted_comments_are_recoverable_and_searchable_when_live() {
    let c = setup();
    let t = c.env.ok("comment.create", json!({ "targetType": "screenplay_scene", "targetId": c.scene, "body": "Chase geography is confusing." }));
    let id = t["comment"]["id"].clone();
    c.env.ok(
        "comment.reply",
        json!({ "parentId": id, "body": "Add an establishing drone shot." }),
    );
    let hits = c
        .env
        .ok("search.query", json!({ "text": "drone establishing" }));
    assert_eq!(hits[0]["entityType"], "comment");
    assert_eq!(hits[0]["nav"]["sceneId"], c.scene);
    c.env.ok("comment.delete", json!({ "id": id }));
    assert!(
        c.env
            .ok("search.query", json!({ "text": "geography" }))
            .as_array()
            .unwrap()
            .iter()
            .all(|h| h["entityType"] != "comment")
    );
    let trash = c.env.ok("trash.list", json!({}));
    let item = trash
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["objectId"] == id)
        .unwrap()
        .clone();
    c.env.ok("trash.restore", json!({ "id": item["id"] }));
    assert_eq!(c.threads().len(), 1);
    assert_eq!(c.threads()[0]["replies"].as_array().unwrap().len(), 1);
    c.env.ok("comment.delete", json!({ "id": id }));
    let trash = c.env.ok("trash.list", json!({}));
    let item = trash
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["objectId"] == id)
        .unwrap()
        .clone();
    c.env.ok("trash.purge", json!({ "id": item["id"] }));
    assert!(
        c.env
            .ok(
                "comment.list",
                json!({ "targetType": "screenplay_scene", "targetId": c.scene })
            )
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn comments_attach_to_any_supported_object() {
    let c = setup();
    let card = new_id();
    let s = c.env.core.project().unwrap();
    s.store
        .mutate(&c.env.actor(), MutationMeta::new("test.seed", "Seed", Capability::Edit), |tx| {
            tx.conn().execute(
                "INSERT INTO story_scene_card(id, parent_type, short_description, position, created_at, updated_at)
                 VALUES (?1, 'unassigned', 'Railway Station Return', 1, 0, 0)",
                [&card],
            )?;
            Ok(())
        })
        .unwrap();
    let t = c.env.ok(
        "comment.create",
        json!({ "targetType": "story_scene_card", "targetId": card, "body": "Great card" }),
    );
    assert_eq!(t["comment"]["targetType"], "story_scene_card");
    assert!(t["comment"]["sceneId"].is_null());
    let list = c.env.ok(
        "comment.list",
        json!({ "targetType": "story_scene_card", "targetId": card }),
    );
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(
        c.env.err("comment.list", json!({})),
        "validation.invalid_input"
    );
    // A draft-level comment is part of the draft's list.
    c.env.ok("comment.create", json!({ "targetType": "screenplay_draft", "targetId": c.draft, "body": "Overall: tighten act two." }));
    assert_eq!(c.threads().len(), 1);
}

#[test]
fn fsd_script_033_private_notes_are_invisible_to_other_users_and_their_search() {
    let c = setup();
    let note = c.env.ok(
        "private_note.create",
        json!({ "targetType": "screenplay_scene", "targetId": c.scene, "body": "I don't trust the producer's suggestion about this exchange." }),
    );
    let nid = note["id"].as_str().unwrap().to_string();
    // Owner sees it, in lists and in search.
    let mine = c.env.ok(
        "private_note.list",
        json!({ "targetType": "screenplay_scene", "targetId": c.scene }),
    );
    assert_eq!(mine.as_array().unwrap().len(), 1);
    let hits = c
        .env
        .ok("search.query", json!({ "text": "producer suggestion" }));
    assert_eq!(hits.as_array().unwrap().len(), 1);
    assert_eq!(hits[0]["entityType"], "private_note");
    // Another user — even an Owner-level collaborator — sees nothing and can't touch it.
    let other = c.env.other_user(Role::Owner);
    let theirs = c
        .env
        .call_as(
            &other,
            "private_note.list",
            json!({ "targetType": "screenplay_scene", "targetId": c.scene }),
        )
        .unwrap();
    assert!(theirs.as_array().unwrap().is_empty());
    let hits = c
        .env
        .call_as(
            &other,
            "search.query",
            json!({ "text": "producer suggestion" }),
        )
        .unwrap();
    assert!(
        hits.as_array().unwrap().is_empty(),
        "private notes never surface in another user's search"
    );
    let e = c
        .env
        .call_as(
            &other,
            "private_note.update",
            json!({ "id": nid, "body": "hijack" }),
        )
        .unwrap_err();
    assert_eq!(
        e.code.0, "not_found.private_note",
        "existence is not revealed"
    );
    let e = c
        .env
        .call_as(&other, "private_note.delete", json!({ "id": nid }))
        .unwrap_err();
    assert_eq!(e.code.0, "not_found.private_note");
    // No activity entry reveals it.
    let activity = c.env.ok("history.activity", json!({}));
    assert!(activity.as_array().unwrap().iter().all(|a| {
        !a["summary"]
            .as_str()
            .unwrap()
            .to_lowercase()
            .contains("private")
    }));
    // Owner edits and deletes; the trash entry is only visible to (and restorable by) the owner.
    let upd = c.env.ok(
        "private_note.update",
        json!({ "id": nid, "body": "Revisit later." }),
    );
    assert_eq!(upd["body"], "Revisit later.");
    c.env.ok("private_note.delete", json!({ "id": nid }));
    let trash = c.env.ok("trash.list", json!({}));
    let item = trash
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["objectId"] == nid)
        .unwrap()
        .clone();
    let their_trash = c.env.call_as(&other, "trash.list", json!({})).unwrap();
    assert!(
        their_trash
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["objectId"] != nid)
    );
    assert!(
        c.env
            .call_as(&other, "trash.restore", json!({ "id": item["id"] }))
            .is_err()
    );
    assert!(
        c.env
            .call_as(&other, "trash.purge", json!({ "id": item["id"] }))
            .is_err()
    );
    c.env.ok("trash.restore", json!({ "id": item["id"] }));
    assert_eq!(
        c.env
            .ok("private_note.list", json!({}))
            .as_array()
            .unwrap()
            .len(),
        1
    );
    // Private notes are not comments: comment lists never include them.
    assert!(c.threads().is_empty());
}
