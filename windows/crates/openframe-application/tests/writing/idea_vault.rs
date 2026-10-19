//! Idea Vault acceptance tests (FSD §5, §88; FSD-IDEA-001..020) through the
//! public operation registry.

use base64::Engine as _;
use openframe_application::MutationMeta;
use openframe_domain::{Capability, Role};
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

fn png_bytes() -> Vec<u8> {
    let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    png.extend_from_slice(&640u32.to_be_bytes());
    png.extend_from_slice(&480u32.to_be_bytes());
    png.extend_from_slice(&[0u8; 32]);
    png
}

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn note(env: &TestEnv, store: &str, body: &str) -> Value {
    env.ok(
        "vault.create",
        json!({ "store": store, "itemType": "note", "body": body }),
    )
}

fn list(env: &TestEnv, store: &str, view: Value) -> Vec<Value> {
    env.ok("vault.list", json!({ "store": store, "view": view }))
        .as_array()
        .unwrap()
        .clone()
}

fn all(env: &TestEnv, store: &str) -> Vec<Value> {
    list(env, store, json!({ "kind": "all" }))
}

fn search(env: &TestEnv, store: &str, text: &str) -> Vec<Value> {
    env.ok("vault.list", json!({ "store": store, "search": text }))
        .as_array()
        .unwrap()
        .clone()
}

fn ids(items: &[Value]) -> Vec<String> {
    let mut v: Vec<String> = items
        .iter()
        .map(|i| i["id"].as_str().unwrap().to_string())
        .collect();
    v.sort();
    v
}

fn id(v: &Value) -> String {
    v["id"].as_str().unwrap().to_string()
}

// ------------------------------------------------------------ FSD-IDEA-001/004

#[test]
fn fsd_idea_001_untitled_note_without_metadata_persists() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let item = env.ok(
        "vault.create",
        json!({ "store": "project", "itemType": "note" }),
    );
    assert!(item["title"].is_null(), "an untitled item is valid");
    assert_eq!(item["displayName"], "Untitled note");
    assert!(item["tags"].as_array().unwrap().is_empty());
    let with_text = note(
        &env,
        "project",
        "What if the hero is investigating his own murder?",
    );
    assert!(
        with_text["title"].is_null(),
        "generated display name is never stored as the title"
    );
    assert_eq!(
        with_text["displayName"],
        "What if the hero is investigating his own murder?"
    );

    let path = env.project_path();
    let env = env.restart();
    env.reopen_project(&path);
    let items = all(&env, "project");
    assert_eq!(items.len(), 2);
    let reopened = env.ok("vault.get", json!({ "store": "project", "id": item["id"] }));
    assert!(reopened["title"].is_null());
    assert_eq!(reopened["displayName"], "Untitled note");
}

#[test]
fn fsd_idea_004_untitled_files_are_valid_and_reopen() {
    let env = TestEnv::with_project("Film", "Short Film");
    let img = env.write_file("IMG_0042.jpg", &png_bytes());
    let aud = env.write_file("memo.m4a", b"audio-bytes");
    let any = env.write_file("model.blend", b"blend");
    let r = env.ok(
        "vault.add_files",
        json!({ "store": "project", "paths": [img, aud, any] }),
    );
    assert_eq!(r["added"].as_array().unwrap().len(), 3);
    assert!(r["failed"].as_array().unwrap().is_empty());
    for i in r["added"].as_array().unwrap() {
        assert!(i["title"].is_null());
    }
    let path = env.project_path();
    let env = env.restart();
    env.reopen_project(&path);
    let items = all(&env, "project");
    let names: Vec<&str> = items
        .iter()
        .map(|i| i["displayName"].as_str().unwrap())
        .collect();
    assert!(
        names.contains(&"IMG_0042.jpg")
            && names.contains(&"memo.m4a")
            && names.contains(&"model.blend")
    );
    assert!(items.iter().all(|i| i["asset"]["available"] == true));
}

// ------------------------------------------------------------------ FSD-IDEA-002

#[test]
fn fsd_idea_002_every_item_type_can_be_added() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let s = "project";
    assert_eq!(note(&env, s, "text")["itemType"], "note");
    let url = env.ok("vault.create", json!({ "store": s, "itemType": "url", "url": "archive-mag.example/brutalist", "body": "Antagonist's house" }));
    assert_eq!(url["itemType"], "url");
    assert_eq!(url["url"], "https://archive-mag.example/brutalist");
    let quote = env.ok("vault.create", json!({ "store": s, "itemType": "quote", "body": "Rain doesn't wash anything away.", "sourceText": "overheard, bus stand" }));
    assert_eq!(quote["itemType"], "quote");

    let files = [
        ("rain.png", png_bytes(), "image"),
        ("Screenshot 2026-09-01.png", png_bytes(), "screenshot"),
        ("research.pdf", b"%PDF-1.4".to_vec(), "pdf"),
        ("character notes.docx", b"PK".to_vec(), "document"),
        ("score.mp3", b"ID3".to_vec(), "audio"),
        ("chase.mp4", b"mp4".to_vec(), "video"),
        ("storyboard.blend", b"x".to_vec(), "file"),
    ];
    for (name, bytes, ty) in files {
        let p = env.write_file(name, &bytes);
        let r = env.ok("vault.add_files", json!({ "store": s, "paths": [p] }));
        assert_eq!(r["added"][0]["itemType"], ty, "{name}");
        assert_eq!(r["added"][0]["asset"]["storageMode"], "managed");
        assert_eq!(r["added"][0]["asset"]["available"], true);
    }
    // A scanned drawing can be added as a sketch; a drawn sketch arrives as bytes.
    let scan = env.write_file("platform-drawing.jpg", &png_bytes());
    let r = env.ok(
        "vault.add_files",
        json!({ "store": s, "paths": [scan], "itemType": "sketch" }),
    );
    assert_eq!(r["added"][0]["itemType"], "sketch");
    let sketch = env.ok(
        "vault.ingest_bytes",
        json!({ "store": s, "itemType": "sketch", "fileName": "sketch.png", "mediaType": "image/png", "dataBase64": b64(&png_bytes()) }),
    );
    assert_eq!(sketch["itemType"], "sketch");
    assert_eq!(sketch["asset"]["width"], 640);
    assert_eq!(sketch["displayName"], "Sketch");
    let voice = env.ok(
        "vault.ingest_bytes",
        json!({ "store": s, "itemType": "voice", "fileName": "voice-note.webm", "mediaType": "audio/webm;codecs=opus", "dataBase64": b64(b"webm-audio"), "durationMs": 42_000 }),
    );
    assert_eq!(voice["itemType"], "voice");
    assert_eq!(voice["asset"]["mediaType"], "audio/webm");
    assert_eq!(voice["displayName"], "Voice note 0:42");
    assert_eq!(all(&env, s).len(), 13);
}

#[test]
fn add_flow_validates_only_what_is_needed() {
    let env = TestEnv::with_project("Film", "Feature Film");
    assert_eq!(
        env.err(
            "vault.create",
            json!({ "store": "project", "itemType": "image" })
        ),
        "validation.invalid_input"
    );
    assert_eq!(
        env.err(
            "vault.create",
            json!({ "store": "project", "itemType": "url" })
        ),
        "validation.required"
    );
    assert_eq!(
        env.err(
            "vault.create",
            json!({ "store": "project", "itemType": "url", "url": "javascript:alert(1)" })
        ),
        "validation.invalid_input"
    );
    assert_eq!(
        env.err(
            "vault.create",
            json!({ "store": "project", "itemType": "sticker" })
        ),
        "validation.invalid_input"
    );
    assert_eq!(
        env.err("vault.ingest_bytes", json!({ "store": "project", "itemType": "voice", "fileName": "v.webm", "dataBase64": "" })),
        "validation.invalid_input"
    );
    assert_eq!(
        env.err("vault.ingest_bytes", json!({ "store": "project", "itemType": "voice", "fileName": "v.webm", "dataBase64": "@@not base64@@" })),
        "validation.invalid_input"
    );
    assert!(
        all(&env, "project").is_empty(),
        "failed adds create nothing"
    );
}

#[test]
fn partial_add_keeps_successful_files() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = env.write_file("a.jpg", &png_bytes());
    let b = env.write_file("b.pdf", b"%PDF");
    let missing = env
        .dir
        .path()
        .join("inputs")
        .join("gone.mov")
        .to_string_lossy()
        .into_owned();
    let r = env.ok(
        "vault.add_files",
        json!({ "store": "project", "paths": [a, missing, b] }),
    );
    assert_eq!(r["added"].as_array().unwrap().len(), 2);
    let failed = r["failed"].as_array().unwrap();
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0]["name"], "gone.mov");
    assert!(!failed[0]["reason"].as_str().unwrap().is_empty());
    assert_eq!(all(&env, "project").len(), 2);
    // One undo step removes the whole drop.
    env.undo();
    assert!(all(&env, "project").is_empty());
}

// ------------------------------------------------------------ FSD-IDEA-003/014

#[test]
fn global_vault_works_with_no_project_open() {
    let env = TestEnv::new();
    let item = note(&env, "global", "A thriller entirely inside a bus.");
    assert_eq!(item["store"], "global");
    assert_eq!(
        env.err("vault.list", json!({ "store": "project" })),
        "not_found.project_not_open"
    );
    assert_eq!(all(&env, "global").len(), 1);
    let hits = env.ok(
        "search.query",
        json!({ "text": "thriller bus", "globalOnly": true }),
    );
    assert_eq!(hits[0]["entityId"], item["id"]);
    assert_eq!(hits[0]["store"], "global");
    // Global has its own undo history.
    env.ok("history.undo", json!({ "store": "global" }));
    assert!(all(&env, "global").is_empty());
    env.ok("history.redo", json!({ "store": "global" }));
    assert_eq!(all(&env, "global").len(), 1);
    let env = env.restart();
    assert_eq!(
        all(&env, "global")[0]["body"],
        "A thriller entirely inside a bus."
    );
}

#[test]
fn fsd_idea_003_014_global_copy_edited_leaves_global_unchanged() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let g = env.ok("vault.create", json!({ "store": "global", "itemType": "note", "title": "Bus", "body": "A thriller entirely inside a bus.", "tags": ["thriller"] }));
    let img = env.write_file("bus.jpg", &png_bytes());
    let gi = env.ok(
        "vault.add_files",
        json!({ "store": "global", "paths": [img] }),
    )["added"][0]
        .clone();
    assert!(all(&env, "project").is_empty(), "vaults are independent");

    let coll = env.ok(
        "vault.create_collection",
        json!({ "store": "project", "name": "Ending ideas" }),
    );
    let r = env.ok("vault.copy_to_store", json!({ "from": "global", "to": "project", "ids": [g["id"], gi["id"]], "collectionId": coll["id"] }));
    let copied = r["copied"].as_array().unwrap();
    assert_eq!(copied.len(), 2);
    assert!(!copied.contains(&g["id"]), "copies get new identities");
    let pcopy = env.ok("vault.get", json!({ "store": "project", "id": copied[0] }));
    assert_eq!(pcopy["sourceGlobalItemId"], g["id"]);
    assert_eq!(pcopy["tags"], json!(["thriller"]));
    assert_eq!(pcopy["collectionIds"], json!([coll["id"]]));

    // Edit the project copy: the Global original is untouched.
    env.ok("vault.update", json!({ "store": "project", "id": copied[0], "body": "A thriller inside a night bus.", "title": "" }));
    env.ok(
        "vault.add_tags",
        json!({ "store": "project", "ids": [copied[0]], "tags": ["night"] }),
    );
    let original = env.ok("vault.get", json!({ "store": "global", "id": g["id"] }));
    assert_eq!(original["body"], "A thriller entirely inside a bus.");
    assert_eq!(original["title"], "Bus");
    assert_eq!(original["tags"], json!(["thriller"]));

    // The file copy is independent: deleting the global original never breaks the copy.
    let pimg = env.ok("vault.get", json!({ "store": "project", "id": copied[1] }));
    assert_ne!(pimg["asset"]["path"], gi["asset"]["path"]);
    assert_eq!(pimg["asset"]["originalName"], "bus.jpg");
    env.ok(
        "vault.delete",
        json!({ "store": "global", "ids": [gi["id"]] }),
    );
    let t = env.ok("vault.trash_list", json!({ "store": "global" }));
    env.ok(
        "vault.purge",
        json!({ "store": "global", "id": t[0]["deleted"]["id"] }),
    );
    assert!(!std::path::Path::new(gi["asset"]["path"].as_str().unwrap()).exists());
    let pimg = env.ok("vault.get", json!({ "store": "project", "id": copied[1] }));
    assert_eq!(pimg["asset"]["available"], true);

    // Project → Global is also an independent copy.
    let p = note(&env, "project", "Project-only idea");
    let r = env.ok(
        "vault.copy_to_store",
        json!({ "from": "project", "to": "global", "ids": [p["id"]] }),
    );
    let gcopy = env.ok(
        "vault.get",
        json!({ "store": "global", "id": r["copied"][0] }),
    );
    assert!(gcopy["sourceGlobalItemId"].is_null());
    env.ok(
        "vault.update",
        json!({ "store": "global", "id": gcopy["id"], "body": "changed in global" }),
    );
    assert_eq!(
        env.ok("vault.get", json!({ "store": "project", "id": p["id"] }))["body"],
        "Project-only idea"
    );
    assert_eq!(
        env.err(
            "vault.copy_to_store",
            json!({ "from": "project", "to": "project", "ids": [p["id"]] })
        ),
        "validation.invalid_input"
    );
}

// ------------------------------------------------------- FSD-IDEA-004/015..017

fn story_mutate(env: &TestEnv, sql: &str, p: &[&dyn rusqlite::ToSql]) {
    let s = env.core.project().unwrap();
    s.store
        .mutate(
            &env.actor(),
            MutationMeta::new("test.story_edit", "Edited story", Capability::Edit),
            |tx| {
                tx.conn().execute(sql, p)?;
                Ok(())
            },
        )
        .unwrap();
}

fn story_text(env: &TestEnv, sql: &str, id: &str) -> Option<String> {
    let s = env.core.project().unwrap();
    use rusqlite::OptionalExtension;
    s.store
        .read(|c| {
            Ok(c.query_row(sql, [id], |r| r.get::<_, Option<String>>(0))
                .optional()?
                .flatten())
        })
        .unwrap()
}

#[test]
fn fsd_idea_015_016_017_send_to_story_copies_and_keeps_original() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let item = env.ok("vault.create", json!({ "store": "project", "itemType": "note", "body": "Train station confrontation — father and son, no words." }));
    let before = env.ok("vault.get", json!({ "store": "project", "id": item["id"] }));

    // Beat, left unplaced by default.
    let beat = env.ok(
        "vault.send_to_story",
        json!({ "store": "project", "id": item["id"], "target": "beat" }),
    );
    assert_eq!(beat["table"], "story_beat");
    assert_eq!(beat["parentType"], "unassigned");
    assert!(beat["message"].as_str().unwrap().contains("original stays"));
    let bid = beat["storyId"].as_str().unwrap();
    assert_eq!(
        story_text(&env, "SELECT text FROM story_beat WHERE id=?1", bid).unwrap(),
        "Train station confrontation — father and son, no words."
    );
    assert_eq!(
        story_text(
            &env,
            "SELECT source_vault_item_id FROM story_beat WHERE id=?1",
            bid
        )
        .unwrap(),
        id(&item)
    );

    // Scene Card in the Parking Lot.
    let card = env.ok("vault.send_to_story", json!({ "store": "project", "id": item["id"], "target": "sceneCard", "parentType": "parking" }));
    let cid = card["storyId"].as_str().unwrap().to_string();
    assert_eq!(
        story_text(
            &env,
            "SELECT parent_type FROM story_scene_card WHERE id=?1",
            &cid
        )
        .unwrap(),
        "parking"
    );

    // Placed inside a chosen act / sequence.
    let act_id = openframe_domain::new_id();
    let seq_id = openframe_domain::new_id();
    story_mutate(
        &env,
        "INSERT INTO story_act(id,title,position,created_at,updated_at) VALUES (?1,'Act 2 — The Discovery',1,0,0)",
        &[&act_id],
    );
    story_mutate(
        &env,
        "INSERT INTO story_sequence(id,act_id,title,position,created_at,updated_at) VALUES (?1,?2,'First Investigation',1,0,0)",
        &[&seq_id, &act_id],
    );
    let targets = env.ok("vault.story_targets", json!({}));
    assert_eq!(
        targets["acts"][0]["sequences"][0]["title"],
        "First Investigation"
    );
    let placed = env.ok("vault.send_to_story", json!({ "store": "project", "id": item["id"], "target": "sceneCard", "parentType": "sequence", "parentId": seq_id }));
    assert_eq!(placed["parentId"], seq_id.as_str());
    assert!(
        placed["message"]
            .as_str()
            .unwrap()
            .contains("First Investigation")
    );
    let seq = env.ok(
        "vault.send_to_story",
        json!({ "store": "project", "id": item["id"], "target": "sequence", "parentId": act_id }),
    );
    assert_eq!(seq["table"], "story_sequence");
    assert_eq!(
        env.err(
            "vault.send_to_story",
            json!({ "store": "project", "id": item["id"], "target": "sequence" })
        ),
        "validation.required"
    );
    let ch = env.ok(
        "vault.send_to_story",
        json!({ "store": "project", "id": item["id"], "target": "character" }),
    );
    assert_eq!(ch["table"], "story_character");
    assert_eq!(env.err("vault.send_to_story", json!({ "store": "project", "id": item["id"], "target": "beat", "parentType": "act", "parentId": "missing" })), "not_found.act");

    // Editing the Story copy never rewrites the Vault item (no live sync) …
    story_mutate(
        &env,
        "UPDATE story_scene_card SET short_description='Changed on the board', rev=rev+1 WHERE id=?1",
        &[&cid],
    );
    let after = env.ok("vault.get", json!({ "store": "project", "id": item["id"] }));
    assert_eq!(after["body"], before["body"]);
    assert_eq!(
        after["rev"], before["rev"],
        "the original was not touched at all"
    );
    assert_eq!(
        all(&env, "project").len(),
        1,
        "the original stays in the Vault"
    );
    // … and editing the Vault item never rewrites the Story copy.
    env.ok(
        "vault.update",
        json!({ "store": "project", "id": item["id"], "body": "Edited in the vault" }),
    );
    assert_eq!(
        story_text(
            &env,
            "SELECT short_description FROM story_scene_card WHERE id=?1",
            &cid
        )
        .unwrap(),
        "Changed on the board"
    );

    // Undo removes only the story copy.
    let step = env.undo();
    assert!(step["label"].as_str().unwrap().contains("Edited"));
    env.undo(); // story edit
    let step = env.undo();
    assert!(step["label"].as_str().unwrap().contains("Sent"));
    assert!(
        story_text(
            &env,
            "SELECT id FROM story_character WHERE id=?1",
            ch["storyId"].as_str().unwrap()
        )
        .is_none()
    );
}

#[test]
fn global_item_can_be_sent_to_story_of_open_project() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let img = env.write_file("face.jpg", &png_bytes());
    let g = env.ok(
        "vault.add_files",
        json!({ "store": "global", "paths": [img] }),
    )["added"][0]
        .clone();
    env.ok(
        "vault.update",
        json!({ "store": "global", "id": g["id"], "title": "Ravi" }),
    );
    let r = env.ok(
        "vault.send_to_story",
        json!({ "store": "global", "id": g["id"], "target": "character" }),
    );
    let cid = r["storyId"].as_str().unwrap();
    assert_eq!(
        story_text(&env, "SELECT name FROM story_character WHERE id=?1", cid).unwrap(),
        "Ravi"
    );
    let asset = story_text(
        &env,
        "SELECT image_asset_id FROM story_character WHERE id=?1",
        cid,
    )
    .unwrap();
    assert_ne!(
        asset,
        g["asset"]["id"].as_str().unwrap(),
        "the picture is copied into the project"
    );
    assert_eq!(
        env.ok("vault.get", json!({ "store": "global", "id": g["id"] }))["title"],
        "Ravi"
    );
}

// ---------------------------------------------------------- FSD-IDEA-005..008

#[test]
fn fsd_idea_005_views_present_the_same_items() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let folder = env.ok(
        "vault.create_folder",
        json!({ "store": "project", "name": "Scenes to explore" }),
    );
    let coll = env.ok(
        "vault.create_collection",
        json!({ "store": "project", "name": "Crazy scenes" }),
    );
    let a = note(&env, "project", "one");
    let b = note(&env, "project", "two");
    env.ok(
        "vault.move_to_folder",
        json!({ "store": "project", "ids": [a["id"]], "folderId": folder["id"] }),
    );
    env.ok(
        "vault.add_to_collection",
        json!({ "store": "project", "ids": [a["id"], b["id"]], "collectionId": coll["id"] }),
    );
    env.ok(
        "vault.set_pinned",
        json!({ "store": "project", "ids": [b["id"]], "pinned": true }),
    );
    let everything = all(&env, "project");
    assert_eq!(everything.len(), 2);
    let in_folder = list(
        &env,
        "project",
        json!({ "kind": "folder", "folderId": folder["id"] }),
    );
    let top = list(
        &env,
        "project",
        json!({ "kind": "folder", "folderId": null }),
    );
    assert_eq!(
        ids(&[in_folder.clone(), top.clone()].concat()),
        ids(&everything),
        "folder view shows each item once"
    );
    assert_eq!(
        ids(&list(
            &env,
            "project",
            json!({ "kind": "collection", "collectionId": coll["id"] })
        )),
        ids(&everything)
    );
    assert_eq!(
        list(&env, "project", json!({ "kind": "pinned" }))[0]["id"],
        b["id"]
    );
    assert_eq!(
        list(&env, "project", json!({ "kind": "recentlyAdded" })).len(),
        2
    );
    assert_eq!(
        list(&env, "project", json!({ "kind": "recentlyModified" })).len(),
        2
    );
    let ov = env.ok("vault.overview", json!({ "store": "project" }));
    assert_eq!(ov["total"], 2);
    assert_eq!(ov["pinned"], 1);
    assert_eq!(ov["unfiled"], 1);
    assert_eq!(ov["folders"][0]["itemCount"], 1);
    assert_eq!(ov["collections"][0]["itemCount"], 2);
    // Content is identical in every view.
    let from_coll = list(
        &env,
        "project",
        json!({ "kind": "collection", "collectionId": coll["id"] }),
    );
    let x = from_coll.iter().find(|i| i["id"] == a["id"]).unwrap();
    assert_eq!(x, &in_folder[0]);
}

#[test]
fn fsd_idea_006_folder_move_changes_organisation_only() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let parent = env.ok(
        "vault.create_folder",
        json!({ "store": "project", "name": "From the trip" }),
    );
    let child = env.ok(
        "vault.create_folder",
        json!({ "store": "project", "name": "Stations", "parentId": parent["id"] }),
    );
    let item = note(&env, "project", "Old platform at night");
    env.ok(
        "vault.move_to_folder",
        json!({ "store": "project", "ids": [item["id"]], "folderId": child["id"] }),
    );
    let moved = env.ok("vault.get", json!({ "store": "project", "id": item["id"] }));
    assert_eq!(moved["folderId"], child["id"]);
    assert_eq!(moved["body"], item["body"]);
    let hits = env.ok("search.query", json!({ "text": "platform" }));
    assert_eq!(
        hits[0]["entityId"], item["id"],
        "moved items remain searchable"
    );
    // Deleting a folder never deletes items: they move up to the parent.
    env.ok(
        "vault.delete_folder",
        json!({ "store": "project", "id": child["id"] }),
    );
    assert_eq!(
        env.ok("vault.get", json!({ "store": "project", "id": item["id"] }))["folderId"],
        parent["id"]
    );
    env.undo();
    assert_eq!(
        env.ok("vault.get", json!({ "store": "project", "id": item["id"] }))["folderId"],
        child["id"]
    );
    assert_eq!(
        env.err(
            "vault.move_to_folder",
            json!({ "store": "project", "ids": [item["id"]], "folderId": "nope" })
        ),
        "not_found.folder"
    );
}

#[test]
fn fsd_idea_007_collection_membership_never_duplicates_or_deletes() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let coll = env.ok(
        "vault.create_collection",
        json!({ "store": "project", "name": "Ending ideas" }),
    );
    let other = env.ok(
        "vault.create_collection",
        json!({ "store": "project", "name": "Music" }),
    );
    let item = note(&env, "project", "Ending where evidence burns.");
    env.ok(
        "vault.add_to_collection",
        json!({ "store": "project", "ids": [item["id"]], "collectionId": coll["id"] }),
    );
    env.ok(
        "vault.add_to_collection",
        json!({ "store": "project", "ids": [item["id"]], "collectionId": other["id"] }),
    );
    env.ok(
        "vault.add_to_collection",
        json!({ "store": "project", "ids": [item["id"]], "collectionId": coll["id"] }),
    );
    assert_eq!(
        all(&env, "project").len(),
        1,
        "membership is a reference, not a copy"
    );
    assert_eq!(
        env.ok("vault.get", json!({ "store": "project", "id": item["id"] }))["collectionIds"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    env.ok(
        "vault.remove_from_collection",
        json!({ "store": "project", "ids": [item["id"]], "collectionId": coll["id"] }),
    );
    assert_eq!(
        all(&env, "project").len(),
        1,
        "removing from a collection keeps the item"
    );
    env.ok(
        "vault.delete_collection",
        json!({ "store": "project", "id": other["id"] }),
    );
    assert_eq!(
        all(&env, "project").len(),
        1,
        "deleting a collection keeps its items"
    );
    assert!(
        env.ok("vault.get", json!({ "store": "project", "id": item["id"] }))["collectionIds"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    env.ok(
        "vault.rename_collection",
        json!({ "store": "project", "id": coll["id"], "name": "Endings" }),
    );
    assert_eq!(
        env.ok("vault.overview", json!({ "store": "project" }))["collections"][0]["name"],
        "Endings"
    );
    assert_eq!(
        env.err(
            "vault.create_collection",
            json!({ "store": "project", "name": "  " })
        ),
        "validation.required"
    );
}

#[test]
fn fsd_idea_008_pin_affects_display_only() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let img = env.write_file("rain.jpg", &png_bytes());
    let item = env.ok(
        "vault.add_files",
        json!({ "store": "project", "paths": [img] }),
    )["added"][0]
        .clone();
    env.ok(
        "vault.set_pinned",
        json!({ "store": "project", "ids": [item["id"]], "pinned": true }),
    );
    let pinned = env.ok("vault.get", json!({ "store": "project", "id": item["id"] }));
    assert_eq!(pinned["pinned"], true);
    assert_eq!(
        pinned["asset"], item["asset"],
        "file path and content unchanged"
    );
    assert_eq!(pinned["folderId"], item["folderId"]);
    env.undo();
    assert_eq!(
        env.ok("vault.get", json!({ "store": "project", "id": item["id"] }))["pinned"],
        false
    );
}

// ------------------------------------------------------------ FSD-IDEA-009/010

#[test]
fn fsd_idea_009_search_matches_title_text_caption_tags_filenames() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let a = env.ok("vault.create", json!({ "store": "project", "itemType": "note", "title": "Station reunion", "body": "father and son" }));
    let b = note(
        &env,
        "project",
        "The last train leaves the station before dawn.",
    );
    let img = env.write_file("old-platform.jpg", &png_bytes());
    let c = env.ok(
        "vault.add_files",
        json!({ "store": "project", "paths": [img] }),
    )["added"][0]
        .clone();
    env.ok(
        "vault.update",
        json!({ "store": "project", "id": c["id"], "caption": "Empty at night" }),
    );
    env.ok(
        "vault.add_tags",
        json!({ "store": "project", "ids": [c["id"]], "tags": ["#locations"] }),
    );

    let r = search(&env, "project", "station");
    assert_eq!(ids(&r), ids(&[a.clone(), b.clone()]));
    let reason_b = r.iter().find(|i| i["id"] == b["id"]).unwrap()["matchReason"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(reason_b, "Matched in text");
    let by_file = search(&env, "project", "platform");
    assert_eq!(by_file.len(), 1);
    assert_eq!(by_file[0]["matchReason"], "Matched in filename");
    let by_tag = search(&env, "project", "locations");
    assert_eq!(by_tag[0]["id"], c["id"]);
    assert_eq!(by_tag[0]["matchReason"], "Matched in tag");
    assert_eq!(
        search(&env, "project", "night")[0]["matchReason"],
        "Matched in caption"
    );
    assert_eq!(
        search(&env, "project", "father son").len(),
        1,
        "all words must match"
    );
    assert!(
        search(&env, "project", "NEAR(\"x\" OR").is_empty(),
        "user text cannot inject search syntax"
    );
    // Tag filter view and project-wide search.
    assert_eq!(
        list(
            &env,
            "project",
            json!({ "kind": "tag", "tag": "Locations" })
        )[0]["id"],
        c["id"]
    );
    let hits = env.ok(
        "search.query",
        json!({ "text": "locations", "entityTypes": ["vault_item"] }),
    );
    assert_eq!(hits[0]["entityId"], c["id"]);
    assert_eq!(hits[0]["nav"]["workspace"], "vault");
    // Undoing the tag removes it from search results.
    env.undo();
    assert!(search(&env, "project", "locations").is_empty());
    // Deleted items leave the search.
    env.ok(
        "vault.delete",
        json!({ "store": "project", "ids": [a["id"]] }),
    );
    assert_eq!(ids(&search(&env, "project", "station")), vec![id(&b)]);
}

#[test]
fn fsd_idea_010_previewable_media_exposes_local_paths() {
    let env = TestEnv::with_project("Film", "Feature Film");
    for (name, media) in [
        ("a.png", "image/png"),
        ("b.mp3", "audio/mpeg"),
        ("c.mp4", "video/mp4"),
        ("d.pdf", "application/pdf"),
    ] {
        let p = env.write_file(name, &png_bytes());
        let item = env.ok(
            "vault.add_files",
            json!({ "store": "project", "paths": [p] }),
        )["added"][0]
            .clone();
        assert_eq!(item["asset"]["mediaType"], media);
        let path = item["asset"]["path"].as_str().unwrap();
        assert!(
            std::path::Path::new(path).is_file(),
            "{name} can be previewed from its stored file"
        );
        let asset = env.ok("files.asset", json!({ "assetId": item["asset"]["id"] }));
        assert_eq!(asset["available"], true);
    }
}

// ------------------------------------------------------------ FSD-IDEA-011..013

#[test]
fn fsd_idea_011_note_autosave_coalesces_and_persists() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let item = env.ok(
        "vault.create",
        json!({ "store": "project", "itemType": "note" }),
    );
    for text in ["F", "Fi", "Final scene", "Final scene before sunrise."] {
        env.ok(
            "vault.update",
            json!({ "store": "project", "id": item["id"], "body": text }),
        );
    }
    env.ok(
        "vault.update",
        json!({ "store": "project", "id": item["id"], "title": "Ending" }),
    );
    let info = env.ok("history.info", json!({}));
    assert!(info["undoLabel"].as_str().unwrap().starts_with("Edited"));
    // All keystrokes collapse into one undo step; undo returns to the empty note.
    env.undo();
    let reverted = env.ok("vault.get", json!({ "store": "project", "id": item["id"] }));
    assert!(reverted["body"].is_null() && reverted["title"].is_null());
    env.redo();
    let path = env.project_path();
    let env = env.restart();
    env.reopen_project(&path);
    let reopened = env.ok("vault.get", json!({ "store": "project", "id": item["id"] }));
    assert_eq!(reopened["body"], "Final scene before sunrise.");
    assert_eq!(reopened["title"], "Ending");
    // Clearing the title makes it untitled again (generated name is not stored).
    env.ok(
        "vault.update",
        json!({ "store": "project", "id": item["id"], "title": "  " }),
    );
    let v = env.ok("vault.get", json!({ "store": "project", "id": item["id"] }));
    assert!(v["title"].is_null());
    assert_eq!(v["displayName"], "Final scene before sunrise.");
    // Stale edits are refused rather than overwriting newer text.
    assert_eq!(
        env.err(
            "vault.update",
            json!({ "store": "project", "id": item["id"], "body": "x", "expectedRev": 1 })
        ),
        "conflict.stale"
    );
}

#[test]
fn fsd_idea_012_url_is_authoritative_without_network() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let u = env.ok("vault.create", json!({ "store": "project", "itemType": "url", "url": "https://archive-mag.example/brutalist", "body": "Could work for the antagonist's house." }));
    assert_eq!(u["displayName"], "https://archive-mag.example/brutalist");
    assert_eq!(u["body"], "Could work for the antagonist's house.");
    assert_eq!(
        env.err(
            "vault.update",
            json!({ "store": "project", "id": u["id"], "url": "" })
        ),
        "validation.required"
    );
    env.ok(
        "vault.update",
        json!({ "store": "project", "id": u["id"], "url": "archive-mag.example/new" }),
    );
    assert_eq!(
        env.ok("vault.get", json!({ "store": "project", "id": u["id"] }))["url"],
        "https://archive-mag.example/new"
    );
    assert_eq!(search(&env, "project", "brutalist").len(), 0);
    assert_eq!(search(&env, "project", "antagonist").len(), 1);
}

#[test]
fn fsd_idea_013_voice_note_saved_and_playable_after_reopen() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let audio = b"\x1aE\xdf\xa3 fake webm opus payload".to_vec();
    let v = env.ok(
        "vault.ingest_bytes",
        json!({ "store": "project", "itemType": "voice", "fileName": "voice-note.webm", "mediaType": "audio/webm", "dataBase64": b64(&audio), "durationMs": 17_000 }),
    );
    assert!(v["title"].is_null());
    let path = env.project_path();
    let env = env.restart();
    env.reopen_project(&path);
    let v = env.ok("vault.get", json!({ "store": "project", "id": v["id"] }));
    assert_eq!(v["asset"]["durationMs"], 17_000);
    assert_eq!(v["displayName"], "Voice note 0:17");
    let bytes = std::fs::read(v["asset"]["path"].as_str().unwrap()).unwrap();
    assert_eq!(bytes, audio, "the recording is kept exactly");
}

// ------------------------------------------------------------------ FSD-IDEA-018

#[test]
fn fsd_idea_018_delete_is_recoverable_and_purge_is_deliberate() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let folder = env.ok(
        "vault.create_folder",
        json!({ "store": "project", "name": "Locations" }),
    );
    let img = env.write_file("platform-blurry.jpg", &png_bytes());
    let item = env.ok(
        "vault.add_files",
        json!({ "store": "project", "paths": [img], "folderId": folder["id"] }),
    )["added"][0]
        .clone();
    env.ok(
        "vault.delete",
        json!({ "store": "project", "ids": [item["id"]] }),
    );
    assert!(all(&env, "project").is_empty());
    assert_eq!(
        env.err("vault.get", json!({ "store": "project", "id": item["id"] })),
        "not_found.idea_vault_item"
    );
    let trash = env.ok("vault.trash_list", json!({ "store": "project" }));
    assert_eq!(trash[0]["typeLabel"], "Image");
    assert_eq!(trash[0]["originalPlace"], "Idea Vault · Locations");
    assert_eq!(
        env.ok("trash.list", json!({}))[0]["objectId"],
        item["id"],
        "also listed in the project's Recently Deleted"
    );

    // Restore returns it to its folder.
    let r = env.ok(
        "vault.restore",
        json!({ "store": "project", "id": trash[0]["deleted"]["id"] }),
    );
    assert_eq!(r["message"], "Restored “platform-blurry.jpg”.");
    assert_eq!(
        env.ok("vault.get", json!({ "store": "project", "id": item["id"] }))["folderId"],
        folder["id"]
    );

    // If the folder is gone, it returns to All items and says so.
    env.ok(
        "vault.delete",
        json!({ "store": "project", "ids": [item["id"]] }),
    );
    env.ok(
        "vault.delete_folder",
        json!({ "store": "project", "id": folder["id"] }),
    );
    let trash = env.ok("vault.trash_list", json!({ "store": "project" }));
    let row = trash
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["deleted"]["objectId"] == item["id"])
        .unwrap()
        .clone();
    let r = env.ok(
        "vault.restore",
        json!({ "store": "project", "id": row["deleted"]["id"] }),
    );
    assert!(r["message"].as_str().unwrap().contains("All items"));
    assert!(
        env.ok("vault.get", json!({ "store": "project", "id": item["id"] }))["folderId"].is_null()
    );

    // Permanent delete removes the stored file; an Editor may not purge.
    env.ok(
        "vault.delete",
        json!({ "store": "project", "ids": [item["id"]] }),
    );
    let trash = env.ok("vault.trash_list", json!({ "store": "project" }));
    let row = trash
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["deleted"]["objectId"] == item["id"])
        .unwrap()
        .clone();
    let editor = env.actor_with_role(Role::Editor);
    let e = env
        .call_as(
            &editor,
            "vault.purge",
            json!({ "store": "project", "id": row["deleted"]["id"] }),
        )
        .unwrap_err();
    assert_eq!(e.code.0, "permission.denied");
    let file = item["asset"]["path"].as_str().unwrap().to_string();
    assert!(std::path::Path::new(&file).exists());
    env.ok(
        "vault.purge",
        json!({ "store": "project", "id": row["deleted"]["id"] }),
    );
    assert!(!std::path::Path::new(&file).exists());
    assert!(
        env.ok("vault.trash_list", json!({ "store": "project" }))
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["deleted"]["objectId"] != item["id"])
    );
    // Purge of folders/collections works too.
    let coll = env.ok(
        "vault.create_collection",
        json!({ "store": "project", "name": "Old" }),
    );
    env.ok(
        "vault.delete_collection",
        json!({ "store": "project", "id": coll["id"] }),
    );
    for t in env
        .ok("vault.trash_list", json!({ "store": "project" }))
        .as_array()
        .unwrap()
    {
        env.ok(
            "vault.purge",
            json!({ "store": "project", "id": t["deleted"]["id"] }),
        );
    }
    assert!(
        env.ok("vault.trash_list", json!({ "store": "project" }))
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn global_trash_restore_and_undo() {
    let env = TestEnv::new();
    let n = note(&env, "global", "Silent film about a lighthouse keeper.");
    env.ok(
        "vault.delete",
        json!({ "store": "global", "ids": [n["id"]] }),
    );
    assert!(all(&env, "global").is_empty());
    env.ok("history.undo", json!({ "store": "global" }));
    assert_eq!(all(&env, "global").len(), 1);
    env.ok(
        "vault.delete",
        json!({ "store": "global", "ids": [n["id"]] }),
    );
    let t = env.ok("vault.trash_list", json!({ "store": "global" }));
    env.ok(
        "vault.restore",
        json!({ "store": "global", "id": t[0]["deleted"]["id"] }),
    );
    assert_eq!(all(&env, "global").len(), 1);
}

// ------------------------------------------------------------------ FSD-IDEA-019

#[test]
fn fsd_idea_019_multi_select_batch_organisation() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let items: Vec<Value> = (1..=5)
        .map(|i| note(&env, "project", &format!("idea {i}")))
        .collect();
    let sel: Vec<Value> = items.iter().map(|i| i["id"].clone()).collect();
    let coll = env.ok(
        "vault.create_collection",
        json!({ "store": "project", "name": "Crazy scenes" }),
    );
    let folder = env.ok(
        "vault.create_folder",
        json!({ "store": "project", "name": "Scenes to explore" }),
    );
    env.ok(
        "vault.add_to_collection",
        json!({ "store": "project", "ids": sel, "collectionId": coll["id"] }),
    );
    assert_eq!(
        list(
            &env,
            "project",
            json!({ "kind": "collection", "collectionId": coll["id"] })
        )
        .len(),
        5
    );
    env.ok(
        "vault.move_to_folder",
        json!({ "store": "project", "ids": sel, "folderId": folder["id"] }),
    );
    env.ok(
        "vault.add_tags",
        json!({ "store": "project", "ids": sel, "tags": ["chase", "night"] }),
    );
    let ov = env.ok("vault.overview", json!({ "store": "project" }));
    assert_eq!(ov["folders"][0]["itemCount"], 5);
    assert_eq!(
        ov["tags"],
        json!([{ "tag": "chase", "count": 5 }, { "tag": "night", "count": 5 }])
    );
    // Each batch is one undo step.
    env.undo();
    assert!(
        env.ok("vault.overview", json!({ "store": "project" }))["tags"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    env.ok(
        "vault.remove_tag",
        json!({ "store": "project", "ids": sel, "tag": "nothing" }),
    );
    env.ok("vault.delete", json!({ "store": "project", "ids": sel }));
    assert!(all(&env, "project").is_empty());
    assert_eq!(
        env.ok("vault.trash_list", json!({ "store": "project" }))
            .as_array()
            .unwrap()
            .len(),
        5
    );
    env.undo();
    assert_eq!(all(&env, "project").len(), 5);
    assert_eq!(
        env.err("vault.delete", json!({ "store": "project", "ids": [] })),
        "validation.invalid_input"
    );
}

// ------------------------------------------------------------------ FSD-IDEA-020

#[test]
fn fsd_idea_020_external_files_are_distinguishable_and_relinkable() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let ext = env.write_file("platform-2.jpg", &png_bytes());
    let item = env.ok(
        "vault.add_files",
        json!({ "store": "project", "paths": [ext.clone()], "mode": "link" }),
    )["added"][0]
        .clone();
    assert_eq!(item["asset"]["storageMode"], "external");
    assert_eq!(item["asset"]["available"], true);
    let managed = env.write_file("inside.jpg", &png_bytes());
    let m = env.ok(
        "vault.add_files",
        json!({ "store": "project", "paths": [managed] }),
    )["added"][0]
        .clone();
    assert_eq!(m["asset"]["storageMode"], "managed");

    std::fs::remove_file(&ext).unwrap();
    let gone = env.ok("vault.get", json!({ "store": "project", "id": item["id"] }));
    assert_eq!(
        gone["asset"]["available"], false,
        "missing external files stay visible as unavailable"
    );
    assert_eq!(all(&env, "project").len(), 2, "nothing was deleted");

    let moved = env.write_file("moved/platform-2.jpg", &png_bytes());
    let relinked = env.ok(
        "vault.relink",
        json!({ "store": "project", "id": item["id"], "path": moved }),
    );
    assert_eq!(relinked["asset"]["available"], true);
    assert_eq!(
        relinked["asset"]["id"], item["asset"]["id"],
        "relink keeps the same file identity"
    );
    assert_eq!(
        env.err(
            "vault.relink",
            json!({ "store": "project", "id": m["id"], "path": moved })
        ),
        "validation.invalid_input"
    );
}

// ------------------------------------------------------------------ permissions

#[test]
fn permissions_viewer_and_commenter_cannot_change_the_project_vault() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let item = note(&env, "project", "shared idea");
    let viewer = env.actor_with_role(Role::Viewer);
    let e = env
        .call_as(
            &viewer,
            "vault.create",
            json!({ "store": "project", "itemType": "note", "body": "x" }),
        )
        .unwrap_err();
    assert_eq!(e.code.0, "permission.denied");
    let p = env.write_file("a.jpg", &png_bytes());
    assert!(
        env.call_as(
            &viewer,
            "vault.add_files",
            json!({ "store": "project", "paths": [p] })
        )
        .is_err()
    );
    assert!(
        env.call_as(
            &viewer,
            "vault.delete",
            json!({ "store": "project", "ids": [item["id"]] })
        )
        .is_err()
    );
    let commenter = env.actor_with_role(Role::Commenter);
    assert!(
        env.call_as(
            &commenter,
            "vault.update",
            json!({ "store": "project", "id": item["id"], "body": "y" })
        )
        .is_err()
    );
    assert!(
        env.call_as(
            &commenter,
            "vault.send_to_story",
            json!({ "store": "project", "id": item["id"], "target": "beat" })
        )
        .is_err()
    );
    // Viewers can still look.
    assert_eq!(
        env.call_as(&viewer, "vault.list", json!({ "store": "project" }))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(all(&env, "project").len(), 1);
    assert_eq!(
        env.ok("vault.get", json!({ "store": "project", "id": item["id"] }))["body"],
        "shared idea"
    );

    // The Global Vault belongs to the person at this computer, whatever their project role …
    let g = env
        .call_as(
            &viewer,
            "vault.create",
            json!({ "store": "global", "itemType": "note", "body": "mine" }),
        )
        .unwrap();
    assert_eq!(g["store"], "global");
    // … and is never reachable by LAN participants.
    let lan = env.other_user(Role::Editor);
    assert_eq!(
        env.call_as(&lan, "vault.list", json!({ "store": "global" }))
            .unwrap_err()
            .code
            .0,
        "permission.denied"
    );
    assert_eq!(
        env.call_as(
            &lan,
            "vault.copy_to_store",
            json!({ "from": "global", "to": "project", "ids": [g["id"]] })
        )
        .unwrap_err()
        .code
        .0,
        "permission.denied"
    );
}

#[test]
fn undo_redo_of_create_and_pin_restore_same_identity() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let n = note(&env, "project", "Costume idea: the same grey jacket.");
    env.undo();
    assert!(all(&env, "project").is_empty());
    assert!(search(&env, "project", "jacket").is_empty());
    env.redo();
    assert_eq!(all(&env, "project")[0]["id"], n["id"]);
    assert_eq!(search(&env, "project", "jacket")[0]["id"], n["id"]);
}
