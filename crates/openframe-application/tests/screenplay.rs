//! Screenplay workspace acceptance tests (FSD §15–17, §21–24, §92–95;
//! FSD-SCRIPT-001..037 except import/export), through the public operation
//! registry — the same path as the UI.

use openframe_application::MutationMeta;
use openframe_domain::{Capability, Role, new_id};
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

struct Sp {
    env: TestEnv,
    screenplay: String,
    draft: String,
}

fn setup(project_type: &str) -> Sp {
    let env = TestEnv::with_project("Black Rain", project_type);
    let created = env.ok("screenplay.create", json!({}));
    Sp {
        screenplay: created["screenplay"]["id"].as_str().unwrap().to_string(),
        draft: created["draft"]["id"].as_str().unwrap().to_string(),
        env,
    }
}

impl Sp {
    fn doc(&self) -> Value {
        self.doc_of(&self.draft)
    }
    fn doc_of(&self, draft: &str) -> Value {
        self.env
            .ok("screenplay.document", json!({ "draftId": draft }))
    }
    fn edits(&self, ops: Value) -> Value {
        self.env.ok(
            "screenplay.apply_edits",
            json!({ "draftId": self.draft, "ops": ops }),
        )
    }
    fn scene_ids(&self) -> Vec<String> {
        self.doc()["scenes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["id"].as_str().unwrap().to_string())
            .collect()
    }
    fn first_scene(&self) -> String {
        self.scene_ids()[0].clone()
    }
    /// Write a small scene: heading + action + character + dialogue.
    fn write_scene(&self, heading: &str, action: &str) -> (String, String) {
        let scene = new_id();
        let el = new_id();
        self.edits(json!([
            { "op": "insertScene", "id": scene, "heading": heading },
            { "op": "insertElement", "id": el, "sceneId": scene, "elementType": "action", "text": action },
            { "op": "insertElement", "sceneId": scene, "elementType": "character", "text": "MEERA" },
            { "op": "insertElement", "sceneId": scene, "elementType": "dialogue", "text": "You said you would never come back." },
        ]));
        (scene, el)
    }
    fn overview(&self) -> Value {
        self.env.ok("screenplay.overview", json!({}))
    }
    fn draft_named(&self, name: &str) -> Value {
        self.overview()["drafts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["name"] == name)
            .cloned()
            .unwrap_or(Value::Null)
    }
}

/// Seed rows owned by other modules (episodes, Story characters) through the pipeline.
fn seed(env: &TestEnv, sql: &str, params: &[&str]) {
    let s = env.core.project().unwrap();
    s.store
        .mutate(
            &env.actor(),
            MutationMeta::new("test.seed", "Seed", Capability::Edit),
            |tx| {
                tx.conn().execute(sql, rusqlite_params(params))?;
                Ok(())
            },
        )
        .unwrap();
}

fn rusqlite_params<'a>(p: &'a [&'a str]) -> impl rusqlite::Params + 'a {
    rusqlite::params_from_iter(p.iter())
}

fn element_texts(scene: &Value) -> Vec<String> {
    scene["elements"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["text"].as_str().unwrap().to_string())
        .collect()
}

// ------------------------------------------------------------ creation

#[test]
fn fsd_script_001_empty_screenplay_without_story_board() {
    let sp = setup("Feature Film");
    let ov = sp.overview();
    assert_eq!(ov["screenplay"]["format"], "Feature");
    assert_eq!(ov["drafts"].as_array().unwrap().len(), 1);
    assert_eq!(ov["drafts"][0]["name"], "Draft 1");
    assert_eq!(ov["drafts"][0]["isCurrent"], true);
    assert_eq!(ov["screenplay"]["titlePage"]["title"], "Black Rain");
    let doc = sp.doc();
    assert_eq!(
        doc["scenes"].as_array().unwrap().len(),
        1,
        "a new screenplay is ready to type into"
    );
    assert_eq!(doc["scenes"][0]["number"], 1);
    assert_eq!(doc["scenes"][0]["elements"][0]["elementType"], "action");
    assert_eq!(doc["canEdit"], true);
    // Only one screenplay per scope: a second create never overwrites.
    assert_eq!(sp.env.err("screenplay.create", json!({})), "conflict.state");
}

#[test]
fn fsd_script_002_003_004_format_follows_project_type() {
    let short = setup("Short Film");
    assert_eq!(short.overview()["screenplay"]["format"], "Short");

    let env = TestEnv::with_project("Series", "Episodic");
    let ov = env.ok("screenplay.overview", json!({}));
    assert_eq!(ov["episodic"], true);
    assert!(ov["screenplay"].is_null());
    assert_eq!(
        env.err("screenplay.create", json!({})),
        "validation.episode",
        "the script lives inside an episode"
    );
    let ep = new_id();
    seed(
        &env,
        "INSERT INTO episode(id, title, position, created_at, updated_at) VALUES (?1, 'Pilot', 1, 0, 0)",
        &[&ep],
    );
    let created = env.ok("screenplay.create", json!({ "episodeId": ep }));
    assert_eq!(created["screenplay"]["format"], "Episodic");
    assert_eq!(created["screenplay"]["episodeId"], ep);
    assert_eq!(created["screenplay"]["title"], "Pilot");
    let ov = env.ok("screenplay.overview", json!({ "episodeId": ep }));
    assert_eq!(ov["screenplay"]["id"], created["screenplay"]["id"]);
}

// ------------------------------------------------------------ elements

#[test]
fn fsd_script_005_011_all_element_types_are_supported_and_validated() {
    let sp = setup("Feature Film");
    let scene = sp.first_scene();
    sp.edits(
        json!([{ "op": "updateScene", "id": scene, "heading": "INT. POLICE STATION — NIGHT" }]),
    );
    for (t, text) in [
        ("action", "Arjun enters carrying a pistol."),
        ("character", "MEERA"),
        ("parenthetical", "(quietly)"),
        ("dialogue", "You said you would never come back."),
        ("transition", "CUT TO:"),
        ("shot", "CLOSE ON THE FOLDER"),
        ("note", "Remember the rain."),
    ] {
        sp.edits(
            json!([{ "op": "insertElement", "sceneId": scene, "elementType": t, "text": text }]),
        );
    }
    let doc = sp.doc();
    let types: Vec<&str> = doc["scenes"][0]["elements"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["elementType"].as_str().unwrap())
        .collect();
    assert_eq!(
        types,
        [
            "action",
            "action",
            "character",
            "parenthetical",
            "dialogue",
            "transition",
            "shot",
            "note"
        ]
    );
    assert_eq!(doc["scenes"][0]["heading"], "INT. POLICE STATION — NIGHT");
    // Server-side element validation.
    let bad = sp.env.err(
        "screenplay.apply_edits",
        json!({ "draftId": sp.draft, "ops": [{ "op": "insertElement", "sceneId": scene, "elementType": "scene_heading", "text": "INT. X" }] }),
    );
    assert_eq!(bad, "validation.element_type");
    let bad = sp.env.err(
        "screenplay.apply_edits",
        json!({ "draftId": sp.draft, "ops": [{ "op": "insertElement", "sceneId": scene, "elementType": "character", "text": "A\nB" }] }),
    );
    assert_eq!(bad, "validation.element_text");
    let bad = sp.env.err(
        "screenplay.apply_edits",
        json!({ "draftId": sp.draft, "ops": [{ "op": "insertElement", "sceneId": scene, "elementType": "sparkle", "text": "x" }] }),
    );
    assert_eq!(bad, "validation.invalid_input");
    // A failed batch changes nothing (atomic).
    let before = sp.doc()["scenes"][0]["elements"].as_array().unwrap().len();
    let bad = sp.env.err(
        "screenplay.apply_edits",
        json!({ "draftId": sp.draft, "ops": [
            { "op": "insertElement", "sceneId": scene, "elementType": "action", "text": "ok" },
            { "op": "insertElement", "sceneId": scene, "elementType": "transition", "text": "BAD\nLINE" },
        ] }),
    );
    assert_eq!(bad, "validation.element_text");
    assert_eq!(
        sp.doc()["scenes"][0]["elements"].as_array().unwrap().len(),
        before
    );
}

#[test]
fn fsd_script_012_manual_element_switching_is_overridable() {
    let sp = setup("Feature Film");
    let el = sp.doc()["scenes"][0]["elements"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    sp.edits(json!([{ "op": "updateElement", "id": el, "text": "Why are you here?" }]));
    sp.edits(json!([{ "op": "updateElement", "id": el, "elementType": "dialogue" }]));
    let doc = sp.doc();
    assert_eq!(doc["scenes"][0]["elements"][0]["elementType"], "dialogue");
    assert_eq!(
        doc["scenes"][0]["elements"][0]["id"], el,
        "switching type keeps identity"
    );
    // Converting a body element into a scene heading splits the scene.
    let (_, action) = sp.write_scene("EXT. STREET — DAY", "The rain has stopped.");
    sp.env
        .ok("screenplay.split_scene", json!({ "elementId": action }));
    let doc = sp.doc();
    assert_eq!(doc["scenes"].as_array().unwrap().len(), 3);
    assert_eq!(doc["scenes"][2]["heading"], "The rain has stopped.");
    assert_eq!(
        element_texts(&doc["scenes"][2]),
        ["MEERA", "You said you would never come back."]
    );
    assert!(element_texts(&doc["scenes"][1]).is_empty());
    // Merge it back: the heading becomes an action line in the previous scene.
    let third = doc["scenes"][2]["id"].as_str().unwrap().to_string();
    sp.env
        .ok("screenplay.merge_scene", json!({ "sceneId": third }));
    let doc = sp.doc();
    assert_eq!(doc["scenes"].as_array().unwrap().len(), 2);
    assert_eq!(
        element_texts(&doc["scenes"][1]),
        [
            "The rain has stopped.",
            "MEERA",
            "You said you would never come back."
        ]
    );
    let first = doc["scenes"][0]["id"].as_str().unwrap().to_string();
    assert_eq!(
        sp.env
            .err("screenplay.merge_scene", json!({ "sceneId": first })),
        "validation.scene"
    );
}

#[test]
fn fsd_script_014_050_navigator_numbers_derive_from_order() {
    let sp = setup("Feature Film");
    let (a, _) = sp.write_scene("EXT. STREET — DAY", "A");
    let (b, _) = sp.write_scene("INT. POLICE STATION — NIGHT", "B");
    let ids = sp.scene_ids();
    assert_eq!(ids.len(), 3);
    assert_eq!(sp.doc()["scenes"][2]["number"], 3);
    // Move scene "b" to the top: identities stay, numbers follow order.
    sp.env
        .ok("screenplay.move_scene", json!({ "sceneId": b, "index": 0 }));
    let doc = sp.doc();
    assert_eq!(doc["scenes"][0]["id"], b);
    assert_eq!(doc["scenes"][0]["number"], 1);
    assert_eq!(doc["scenes"][2]["id"], a);
    assert_eq!(doc["scenes"][2]["number"], 3);
    // New scene from the navigator at a position; heading typed, number generated.
    let r = sp.env.ok(
        "screenplay.create_scene",
        json!({ "draftId": sp.draft, "index": 1, "heading": "EXT. LEVEL CROSSING — DAWN" }),
    );
    let created = r["createdIds"][0].as_str().unwrap();
    let doc = sp.doc();
    assert_eq!(doc["scenes"][1]["id"], created);
    assert_eq!(doc["scenes"][1]["number"], 2);
    assert_eq!(doc["scenes"][1]["elements"][0]["elementType"], "action");
    // Undo the move: order restored exactly.
    sp.env.undo();
    sp.env.undo();
    assert_eq!(sp.scene_ids(), ids);
}

#[test]
fn scene_delete_is_recoverable_and_restores_position() {
    let sp = setup("Feature Film");
    let (a, _) = sp.write_scene("EXT. STREET — DAY", "Puddles.");
    sp.write_scene("INT. MORGUE — MORNING", "Cold light.");
    sp.env
        .ok("screenplay.delete_scene", json!({ "sceneId": a }));
    assert_eq!(sp.scene_ids().len(), 2);
    let trash = sp.env.ok("trash.list", json!({}));
    let item = trash
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["objectId"] == a)
        .unwrap()
        .clone();
    assert!(item["title"].as_str().unwrap().contains("EXT. STREET"));
    sp.env.ok("trash.restore", json!({ "id": item["id"] }));
    let doc = sp.doc();
    assert_eq!(
        doc["scenes"][1]["id"], a,
        "restored to its previous position"
    );
    assert_eq!(
        element_texts(&doc["scenes"][1])[0],
        "Puddles.",
        "content comes back with the scene"
    );
    // Purge after delete removes it for good.
    sp.env
        .ok("screenplay.delete_scene", json!({ "sceneId": a }));
    let trash = sp.env.ok("trash.list", json!({}));
    let item = trash
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["objectId"] == a)
        .unwrap()
        .clone();
    sp.env.ok("trash.purge", json!({ "id": item["id"] }));
    assert_eq!(sp.scene_ids().len(), 2);
    // The last scene can't be deleted.
    let ids = sp.scene_ids();
    sp.env
        .ok("screenplay.delete_scene", json!({ "sceneId": ids[0] }));
    assert_eq!(
        sp.env
            .err("screenplay.delete_scene", json!({ "sceneId": ids[1] })),
        "validation.scene"
    );
}

// ---------------------------------------------------------- find / replace

#[test]
fn fsd_script_015_016_replace_all_is_one_undoable_change() {
    let sp = setup("Feature Film");
    let scene = sp.first_scene();
    sp.edits(json!([
        { "op": "updateScene", "id": scene, "heading": "INT. FOLDER ROOM — NIGHT" },
        { "op": "insertElement", "sceneId": scene, "elementType": "action", "text": "He places the RED FOLDER on the folder desk." },
        { "op": "insertElement", "sceneId": scene, "elementType": "dialogue", "text": "Neither did the folder. Folders!" },
        { "op": "insertElement", "sceneId": scene, "elementType": "note", "text": "folder note" },
    ]));
    let r = sp.env.ok(
        "screenplay.replace_all",
        json!({ "draftId": sp.draft, "query": "folder", "replacement": "file", "matchCase": false, "wholeWord": true, "includeNotes": false }),
    );
    assert_eq!(
        r["count"], 4,
        "heading, two in action, one in dialogue; 'Folders' is not a whole word; notes excluded"
    );
    let doc = sp.doc();
    assert_eq!(doc["scenes"][0]["heading"], "INT. file ROOM — NIGHT");
    assert_eq!(
        element_texts(&doc["scenes"][0])[1],
        "He places the RED file on the file desk."
    );
    assert_eq!(element_texts(&doc["scenes"][0])[3], "folder note");
    let step = sp.env.undo();
    assert!(step["label"].as_str().unwrap().starts_with("Replaced"));
    let doc = sp.doc();
    assert_eq!(
        doc["scenes"][0]["heading"], "INT. FOLDER ROOM — NIGHT",
        "one undo restores every replacement"
    );
    assert_eq!(
        element_texts(&doc["scenes"][0])[2],
        "Neither did the folder. Folders!"
    );
    let r = sp.env.ok(
        "screenplay.replace_all",
        json!({ "draftId": sp.draft, "query": "FOLDER", "replacement": "file", "matchCase": true, "wholeWord": false }),
    );
    assert_eq!(r["count"], 2);
}

// ----------------------------------------------------- autosave, undo/redo

#[test]
fn fsd_script_021_autosave_persisted_on_restart_and_crash() {
    let sp = setup("Feature Film");
    let (_, el) = sp.write_scene(
        "EXT. OLD RAILWAY STATION — NIGHT",
        "A train that isn't there.",
    );
    sp.edits(json!([{ "op": "updateElement", "id": el, "text": "A train that isn't there. Its whistle." }]));
    let path = sp.env.project_path();
    let draft = sp.draft.clone();
    let env = sp.env.restart();
    env.reopen_project(&path);
    let doc = env.ok("screenplay.document", json!({ "draftId": draft }));
    assert_eq!(
        doc["scenes"][1]["heading"],
        "EXT. OLD RAILWAY STATION — NIGHT"
    );
    assert_eq!(
        doc["scenes"][1]["elements"][0]["text"],
        "A train that isn't there. Its whistle."
    );
    env.ok("screenplay.apply_edits", json!({ "draftId": draft, "ops": [{ "op": "updateElement", "id": el, "text": "Edited before the crash." }] }));
    let env = env.crash_and_restart();
    env.reopen_project(&path);
    let doc = env.ok("screenplay.document", json!({ "draftId": draft }));
    assert_eq!(
        doc["scenes"][1]["elements"][0]["text"], "Edited before the crash.",
        "every edit is committed immediately"
    );
}

#[test]
fn fsd_script_022_undo_redo_groups_typing() {
    let sp = setup("Feature Film");
    let el = sp.doc()["scenes"][0]["elements"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    // Debounced batches while typing in one element coalesce into one undo step.
    for t in [
        "Arjun",
        "Arjun enters",
        "Arjun enters carrying",
        "Arjun enters carrying a pistol.",
    ] {
        let r = sp.edits(json!([{ "op": "updateElement", "id": el, "text": t }]));
        assert!(r["seq"].as_i64().unwrap() > 0);
    }
    let info = sp.env.ok("history.info", json!({}));
    assert!(
        info["undoLabel"]
            .as_str()
            .unwrap()
            .starts_with("Typing in Scene 1")
    );
    sp.env.undo();
    assert_eq!(
        sp.doc()["scenes"][0]["elements"][0]["text"],
        "",
        "one undo removes the whole typing run"
    );
    sp.env.redo();
    assert_eq!(
        sp.doc()["scenes"][0]["elements"][0]["text"],
        "Arjun enters carrying a pistol."
    );
    // A new element inserted with its first characters, then typed into, is one step too.
    let scene = sp.first_scene();
    let e2 = new_id();
    sp.edits(json!([{ "op": "insertElement", "id": e2, "sceneId": scene, "elementType": "character", "text": "M" }]));
    sp.edits(json!([{ "op": "updateElement", "id": e2, "text": "MEERA" }]));
    sp.env.undo();
    assert_eq!(
        sp.doc()["scenes"][0]["elements"].as_array().unwrap().len(),
        1
    );
    // Structural batches are separate steps.
    sp.edits(json!([{ "op": "insertScene", "heading": "EXT. STREET — DAY" }]));
    sp.env.undo();
    assert_eq!(sp.scene_ids().len(), 1);
    assert_eq!(
        sp.doc()["scenes"][0]["elements"][0]["text"],
        "Arjun enters carrying a pistol."
    );
}

// ---------------------------------------------------------------- drafts

#[test]
fn fsd_script_023_024_named_drafts_preserve_prior_versions_with_lineage() {
    let sp = setup("Feature Film");
    let (scene, el) = sp.write_scene("INT. POLICE STATION — NIGHT", "Original line.");
    let d2 = sp.env.ok(
        "screenplay.new_draft",
        json!({ "sourceDraftId": sp.draft, "name": "Draft 2", "note": "Producer pass" }),
    );
    assert_eq!(d2["isCurrent"], true, "the new draft becomes current");
    assert_eq!(d2["note"], "Producer pass");
    let d2id = d2["id"].as_str().unwrap().to_string();
    let d3 = sp.env.ok(
        "screenplay.new_draft",
        json!({ "sourceDraftId": d2id, "name": "Draft 3 — Director Rewrite" }),
    );
    let lineage: Vec<&str> = d3["lineage"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        lineage,
        ["Draft 1", "Draft 2", "Draft 3 — Director Rewrite"]
    );
    // Copies have new identities but the same scene lineage.
    let doc2 = sp.doc_of(&d2id);
    let doc1 = sp.doc();
    assert_ne!(doc2["scenes"][1]["id"], doc1["scenes"][1]["id"]);
    assert_eq!(
        doc2["scenes"][1]["lineageId"],
        doc1["scenes"][1]["lineageId"]
    );
    // Editing the new draft never changes the source.
    let el2 = doc2["scenes"][1]["elements"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    sp.env.ok("screenplay.apply_edits", json!({ "draftId": d2id, "ops": [{ "op": "updateElement", "id": el2, "text": "Rewritten line." }] }));
    assert_eq!(
        sp.doc()["scenes"][1]["elements"][0]["text"],
        "Original line."
    );
    assert_eq!(sp.doc()["scenes"][1]["id"], scene);
    assert_eq!(sp.doc()["scenes"][1]["elements"][0]["id"], el);
    // Duplicate names are refused; rename changes only the label.
    assert_eq!(
        sp.env.err(
            "screenplay.new_draft",
            json!({ "sourceDraftId": sp.draft, "name": "draft 2" })
        ),
        "validation.duplicate"
    );
    let renamed = sp.env.ok(
        "screenplay.rename_draft",
        json!({ "draftId": d2id, "name": "Draft 2 — Producer" }),
    );
    assert_eq!(renamed["name"], "Draft 2 — Producer");
    assert_eq!(
        sp.doc_of(&d2id)["scenes"][1]["elements"][0]["text"],
        "Rewritten line."
    );
}

#[test]
fn fsd_script_025_exactly_one_current_draft() {
    let sp = setup("Feature Film");
    let d2 = sp.env.ok(
        "screenplay.new_draft",
        json!({ "sourceDraftId": sp.draft, "name": "Draft 2" }),
    )["id"]
        .as_str()
        .unwrap()
        .to_string();
    let d3 = sp.env.ok(
        "screenplay.new_draft",
        json!({ "sourceDraftId": d2, "name": "Draft 3", "makeCurrent": false }),
    )["id"]
        .as_str()
        .unwrap()
        .to_string();
    let count_current = |sp: &Sp| {
        sp.overview()["drafts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|d| d["isCurrent"] == true)
            .count()
    };
    assert_eq!(count_current(&sp), 1);
    assert_eq!(sp.overview()["screenplay"]["currentDraftId"], d2);
    sp.env
        .ok("screenplay.set_current_draft", json!({ "draftId": d3 }));
    assert_eq!(count_current(&sp), 1);
    assert_eq!(sp.overview()["screenplay"]["currentDraftId"], d3);
    // The current draft can't be deleted until another is current.
    assert_eq!(
        sp.env
            .err("screenplay.delete_draft", json!({ "draftId": d3 })),
        "conflict.state"
    );
    sp.env
        .ok("screenplay.delete_draft", json!({ "draftId": d2 }));
    assert!(sp.draft_named("Draft 2").is_null());
    assert_eq!(count_current(&sp), 1);
    // Recently Deleted → restore.
    let trash = sp.env.ok("trash.list", json!({}));
    let item = trash
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["objectId"] == d2)
        .unwrap()
        .clone();
    sp.env.ok("trash.restore", json!({ "id": item["id"] }));
    assert!(!sp.draft_named("Draft 2").is_null());
    // Purge a deleted draft: lineage of later drafts stays readable.
    sp.env
        .ok("screenplay.delete_draft", json!({ "draftId": d2 }));
    let trash = sp.env.ok("trash.list", json!({}));
    let item = trash
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["objectId"] == d2)
        .unwrap()
        .clone();
    sp.env.ok("trash.purge", json!({ "id": item["id"] }));
    let d3v = sp.draft_named("Draft 3");
    let lineage: Vec<&str> = d3v["lineage"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["name"].as_str().unwrap())
        .collect();
    assert_eq!(lineage, ["Draft 1", "Draft 3"]);
    // Undo of making current works through the generic pipeline.
    sp.env.ok(
        "screenplay.set_current_draft",
        json!({ "draftId": sp.draft }),
    );
    sp.env.undo();
    assert_eq!(sp.overview()["screenplay"]["currentDraftId"], d3);
}

#[test]
fn fsd_script_021_7_restoring_an_old_draft_creates_a_new_draft() {
    let sp = setup("Feature Film");
    sp.write_scene("INT. MORGUE — MORNING", "Version one.");
    let d2 = sp.env.ok(
        "screenplay.new_draft",
        json!({ "sourceDraftId": sp.draft, "name": "Draft 2" }),
    )["id"]
        .as_str()
        .unwrap()
        .to_string();
    let restored = sp
        .env
        .ok("screenplay.restore_draft", json!({ "draftId": sp.draft }));
    assert_eq!(restored["name"], "Restored from Draft 1");
    assert_eq!(restored["isCurrent"], true);
    assert_eq!(restored["createdFromDraftId"], sp.draft);
    assert_eq!(
        sp.overview()["drafts"].as_array().unwrap().len(),
        3,
        "nothing was replaced"
    );
    assert!(!sp.draft_named("Draft 2").is_null());
    let again = sp
        .env
        .ok("screenplay.restore_draft", json!({ "draftId": sp.draft }));
    assert_eq!(again["name"], "Restored from Draft 1 (2)");
    let _ = d2;
}

#[test]
fn fsd_script_026_automatic_history_is_separate_from_named_drafts() {
    let sp = setup("Feature Film");
    let points = sp
        .env
        .ok("screenplay.history_points", json!({ "draftId": sp.draft }));
    assert_eq!(points.as_array().unwrap().len(), 1);
    assert_eq!(points[0]["reason"], "draft_created");
    // Editing right after creation doesn't add a point before the interval passes.
    let (_, el) = sp.write_scene("EXT. STREET — DAY", "Before.");
    assert_eq!(
        sp.env
            .ok("screenplay.history_points", json!({ "draftId": sp.draft }))
            .as_array()
            .unwrap()
            .len(),
        1
    );
    sp.edits(json!([{ "op": "updateElement", "id": el, "text": "After." }]));
    // Automatic points are not drafts.
    assert_eq!(sp.overview()["drafts"].as_array().unwrap().len(), 1);
    // Restoring a point creates a new draft with the recorded content (the initial blank state).
    let restored = sp.env.ok(
        "screenplay.restore_history_point",
        json!({ "historyPointId": points[0]["id"] }),
    );
    assert_eq!(restored["name"], "Restored from Draft 1 (automatic point)");
    assert_eq!(restored["isCurrent"], true);
    let doc = sp.doc_of(restored["id"].as_str().unwrap());
    assert_eq!(doc["scenes"].as_array().unwrap().len(), 1);
    assert_eq!(
        sp.doc()["scenes"][1]["elements"][0]["text"],
        "After.",
        "the source draft is untouched"
    );
    // The restored draft got its own creation point.
    let pts = sp.env.ok(
        "screenplay.history_points",
        json!({ "draftId": restored["id"] }),
    );
    assert_eq!(pts.as_array().unwrap().len(), 1);
}

// --------------------------------------------------------------- compare

#[test]
fn fsd_script_027_compare_at_scene_and_text_level() {
    let sp = setup("Feature Film");
    let first = sp.first_scene();
    sp.edits(json!([{ "op": "updateScene", "id": first, "heading": "EXT. STREET — DAY" }]));
    sp.write_scene("INT. POLICE STATION — NIGHT", "Arjun enters.");
    sp.write_scene("EXT. LEVEL CROSSING — DAWN", "A train passes.");
    sp.write_scene("INT. MORGUE — MORNING", "Cold light.");
    let d2 = sp.env.ok(
        "screenplay.new_draft",
        json!({ "sourceDraftId": sp.draft, "name": "Draft 2" }),
    )["id"]
        .as_str()
        .unwrap()
        .to_string();
    let doc = sp.doc_of(&d2);
    let ids: Vec<String> = doc["scenes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap().to_string())
        .collect();
    let police_action = doc["scenes"][1]["elements"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    sp.env.ok(
        "screenplay.apply_edits",
        json!({ "draftId": d2, "ops": [
            { "op": "updateElement", "id": police_action, "text": "Arjun enters carrying a pistol." },
            { "op": "moveScene", "id": ids[2], "index": 0 },
            { "op": "deleteScene", "id": ids[3] },
            { "op": "insertScene", "heading": "EXT. HOSPITAL — DAY" },
        ] }),
    );
    let cmp = sp.env.ok(
        "screenplay.compare",
        json!({ "draftA": sp.draft, "draftB": d2 }),
    );
    let s = &cmp["summary"];
    assert_eq!(
        (
            s["added"].as_u64(),
            s["removed"].as_u64(),
            s["changed"].as_u64(),
            s["moved"].as_u64(),
            s["unchanged"].as_u64()
        ),
        (Some(1), Some(1), Some(1), Some(1), Some(1)),
        "{s}"
    );
    let changed = cmp["scenes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["kind"] == "changed")
        .unwrap();
    assert_eq!(changed["matchedBy"], "identity");
    assert_eq!(changed["linesChanged"], 1);
    let row = changed["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["kind"] == "changed")
        .unwrap();
    assert_eq!(row["left"]["text"], "Arjun enters.");
    assert_eq!(row["right"]["text"], "Arjun enters carrying a pistol.");
    assert!(
        row["right"]["segments"]
            .as_array()
            .unwrap()
            .iter()
            .any(|g| g["kind"] == "added")
    );
    // Read-only: comparing changed nothing.
    assert_eq!(sp.doc()["scenes"].as_array().unwrap().len(), 4);
    assert_eq!(
        sp.env
            .err("screenplay.compare", json!({ "draftA": d2, "draftB": d2 })),
        "validation.invalid_input"
    );
}

// --------------------------------------------------------------- review

#[test]
fn fsd_script_028_review_round_linked_to_draft_and_completing_never_locks() {
    let sp = setup("Feature Film");
    let scene = sp.first_scene();
    let round = sp.env.ok(
        "screenplay.start_review",
        json!({ "draftId": sp.draft, "name": "Draft 1 — Producer Review", "reviewers": ["Suresh", "Priya", "priya"], "deadline": "2026-10-20" }),
    );
    assert_eq!(round["reviewers"], json!(["Suresh", "Priya"]));
    assert_eq!(round["status"], "Open");
    assert_eq!(sp.draft_named("Draft 1")["status"], "Review");
    // Comments on the draft join the open round automatically.
    let c = sp.env.ok("comment.create", json!({ "targetType": "screenplay_scene", "targetId": scene, "body": "Lighting: practical-only?" }));
    assert_eq!(c["comment"]["reviewRoundId"], round["id"]);
    let rounds = sp.env.ok(
        "screenplay.review_rounds",
        json!({ "screenplayId": sp.screenplay }),
    );
    assert_eq!(rounds[0]["openCount"], 1);
    assert_eq!(
        sp.env.err(
            "screenplay.start_review",
            json!({ "draftId": sp.draft, "name": "x", "deadline": "2026-02-30" })
        ),
        "validation.invalid_input"
    );
    // Viewers can't complete a review; editors can.
    let viewer = sp.env.actor_with_role(Role::Viewer);
    assert!(
        sp.env
            .call_as(
                &viewer,
                "screenplay.complete_review",
                json!({ "reviewRoundId": round["id"] })
            )
            .is_err()
    );
    let done = sp.env.ok(
        "screenplay.complete_review",
        json!({ "reviewRoundId": round["id"] }),
    );
    assert_eq!(done["status"], "Complete");
    let d = sp.draft_named("Draft 1");
    assert_eq!(
        d["status"], "Draft",
        "completing a review does not lock the script"
    );
    assert_eq!(sp.doc()["canEdit"], true);
}

// --------------------------------------------------------- lock, revision

#[test]
fn fsd_script_034_035_locked_draft_rejects_edits() {
    let sp = setup("Feature Film");
    let (scene, el) = sp.write_scene("INT. POLICE STATION — NIGHT", "Locked text.");
    sp.env.ok(
        "comment.create",
        json!({ "targetType": "screenplay_scene", "targetId": scene, "body": "Open note" }),
    );
    let summary = sp
        .env
        .ok("screenplay.lock_summary", json!({ "draftId": sp.draft }));
    assert_eq!(summary["draftName"], "Draft 1");
    assert_eq!(summary["status"], "Draft");
    assert_eq!(summary["openComments"], 1);
    assert_eq!(summary["sceneCount"], 2);
    assert!(summary["lastModified"].as_i64().unwrap() > 0);
    assert!(
        summary["openCommentSamples"][0]
            .as_str()
            .unwrap()
            .starts_with("Scene 2: Open note")
    );
    let locked = sp
        .env
        .ok("screenplay.lock_draft", json!({ "draftId": sp.draft }));
    assert_eq!(locked["status"], "Locked");
    assert!(locked["lockedAt"].as_i64().is_some());
    assert!(locked["lockedByName"].is_string());
    let doc = sp.doc();
    assert_eq!(doc["canEdit"], false);
    for ops in [
        json!([{ "op": "updateElement", "id": el, "text": "Changed" }]),
        json!([{ "op": "insertScene", "heading": "EXT. X" }]),
        json!([{ "op": "deleteElement", "id": el }]),
    ] {
        assert_eq!(
            sp.env.err(
                "screenplay.apply_edits",
                json!({ "draftId": sp.draft, "ops": ops })
            ),
            "permission.locked_draft"
        );
    }
    assert_eq!(
        sp.env.err(
            "screenplay.update_scene",
            json!({ "sceneId": scene, "notes": "x" })
        ),
        "permission.locked_draft"
    );
    assert_eq!(
        sp.env.err(
            "screenplay.replace_all",
            json!({ "draftId": sp.draft, "query": "Locked", "replacement": "x" })
        ),
        "permission.locked_draft"
    );
    assert_eq!(
        sp.env
            .err("screenplay.delete_scene", json!({ "sceneId": scene })),
        "permission.locked_draft"
    );
    assert_eq!(sp.doc()["scenes"][1]["elements"][0]["text"], "Locked text.");
    // Comments are still allowed on a locked draft.
    sp.env.ok(
        "comment.create",
        json!({ "targetType": "screenplay_scene", "targetId": scene, "body": "Still reviewable" }),
    );
    // Commenters can't lock; the only locked baseline can't be deleted.
    let commenter = sp.env.actor_with_role(Role::Commenter);
    assert_eq!(
        sp.env
            .call_as(
                &commenter,
                "screenplay.unlock_draft",
                json!({ "draftId": sp.draft })
            )
            .unwrap_err()
            .code
            .0,
        "permission.denied"
    );
    sp.env.ok(
        "screenplay.new_draft",
        json!({ "sourceDraftId": sp.draft, "name": "Scratch" }),
    );
    assert_eq!(
        sp.env
            .err("screenplay.delete_draft", json!({ "draftId": sp.draft })),
        "conflict.state"
    );
}

#[test]
fn fsd_script_036_037_revision_leaves_locked_draft_unchanged() {
    let sp = setup("Feature Film");
    let (_, _) = sp.write_scene("INT. POLICE STATION — NIGHT", "Baseline line.");
    sp.write_scene("EXT. LEVEL CROSSING — DAWN", "Train.");
    sp.env
        .ok("screenplay.lock_draft", json!({ "draftId": sp.draft }));
    let before = sp.doc();
    assert_eq!(
        sp.env.err(
            "screenplay.start_revision",
            json!({ "draftId": sp.draft, "label": "Revision A", "color": "Sparkly" })
        ),
        "validation.invalid_input"
    );
    let rev = sp.env.ok(
        "screenplay.start_revision",
        json!({ "draftId": sp.draft, "label": "Revision A", "color": "blue", "reason": "Location change for the chase" }),
    );
    assert_eq!(rev["status"], "Revision");
    assert_eq!(rev["revisionLabel"], "Revision A");
    assert_eq!(rev["revisionColor"], "Blue");
    assert_eq!(rev["revisionReason"], "Location change for the chase");
    assert_eq!(rev["createdFromDraftId"], sp.draft);
    assert_eq!(rev["isCurrent"], true);
    let rid = rev["id"].as_str().unwrap().to_string();
    let rdoc = sp.doc_of(&rid);
    assert_eq!(rdoc["canEdit"], true);
    let el = rdoc["scenes"][1]["elements"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let train = rdoc["scenes"][2]["id"].as_str().unwrap().to_string();
    sp.env.ok(
        "screenplay.apply_edits",
        json!({ "draftId": rid, "ops": [
            { "op": "updateElement", "id": el, "text": "Revised line." },
            { "op": "deleteScene", "id": train },
            { "op": "insertScene", "heading": "EXT. BRIDGE — NIGHT" },
        ] }),
    );
    // Locked source is exactly as it was.
    let after = sp.doc();
    assert_eq!(before["scenes"], after["scenes"]);
    assert_eq!(sp.draft_named("Draft 1")["status"], "Locked");
    // The revision view lists the changed scenes.
    let changes = sp
        .env
        .ok("screenplay.revision_changes", json!({ "draftId": rid }));
    let kinds: Vec<&str> = changes["scenes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["kind"].as_str().unwrap())
        .collect();
    assert_eq!(changes["summary"]["changed"], 1);
    assert_eq!(changes["summary"]["removed"], 1);
    assert_eq!(changes["summary"]["added"], 1);
    assert!(!kinds.contains(&"unchanged"));
    let upd = sp.env.ok(
        "screenplay.update_revision",
        json!({ "draftId": rid, "color": "#FF66AA" }),
    );
    assert_eq!(upd["revisionColor"], "#ff66aa");
    // Start Revision needs a locked source.
    assert_eq!(
        sp.env.err(
            "screenplay.start_revision",
            json!({ "draftId": rid, "label": "Revision B" })
        ),
        "validation.status"
    );
}

// ----------------------------------------------------- permissions, search

#[test]
fn viewers_and_commenters_cannot_edit_the_script() {
    let sp = setup("Feature Film");
    let el = sp.doc()["scenes"][0]["elements"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    for role in [Role::Viewer, Role::Commenter, Role::ExportOnly] {
        let a = sp.env.actor_with_role(role);
        let e = sp
            .env
            .call_as(&a, "screenplay.apply_edits", json!({ "draftId": sp.draft, "ops": [{ "op": "updateElement", "id": el, "text": "x" }] }))
            .unwrap_err();
        assert_eq!(e.code.0, "permission.denied", "{role:?}");
        assert!(
            sp.env
                .call_as(
                    &a,
                    "screenplay.new_draft",
                    json!({ "sourceDraftId": sp.draft, "name": "X" })
                )
                .is_err()
        );
        // Reading is allowed.
        let doc = sp
            .env
            .call_as(&a, "screenplay.document", json!({ "draftId": sp.draft }))
            .unwrap();
        assert_eq!(doc["canEdit"], false);
    }
    assert_eq!(sp.doc()["scenes"][0]["elements"][0]["text"], "");
}

#[test]
fn scenes_of_the_current_draft_are_searchable() {
    let sp = setup("Feature Film");
    let (scene, _) = sp.write_scene(
        "INT. POLICE STATION — NIGHT",
        "He places the crimson dossier on the desk.",
    );
    let hits = sp
        .env
        .ok("search.query", json!({ "text": "crimson dossier" }));
    assert_eq!(hits.as_array().unwrap().len(), 1);
    assert_eq!(hits[0]["entityType"], "screenplay_scene");
    assert_eq!(hits[0]["title"], "Scene 2 — INT. POLICE STATION — NIGHT");
    assert_eq!(hits[0]["nav"]["workspace"], "screenplay");
    assert_eq!(hits[0]["nav"]["sceneId"], scene);
    // A new current draft: hits point at its scenes, never duplicates from old drafts.
    let d2 = sp.env.ok(
        "screenplay.new_draft",
        json!({ "sourceDraftId": sp.draft, "name": "Draft 2" }),
    );
    let hits = sp.env.ok("search.query", json!({ "text": "crimson" }));
    assert_eq!(hits.as_array().unwrap().len(), 1);
    assert_eq!(hits[0]["nav"]["draftId"], d2["id"]);
    // Numbers in titles follow reorders.
    let d2doc = sp.doc_of(d2["id"].as_str().unwrap());
    let s2 = d2doc["scenes"][1]["id"].as_str().unwrap();
    sp.env.ok(
        "screenplay.move_scene",
        json!({ "sceneId": s2, "index": 0 }),
    );
    let hits = sp.env.ok("search.query", json!({ "text": "crimson" }));
    assert_eq!(hits[0]["title"], "Scene 1 — INT. POLICE STATION — NIGHT");
}

// ------------------------------------------------ notes, title, characters

#[test]
fn fsd_script_020_scene_notes_are_non_printing_scene_metadata() {
    let sp = setup("Feature Film");
    let scene = sp.first_scene();
    sp.env.ok(
        "screenplay.update_scene",
        json!({ "sceneId": scene, "notes": "Need a stronger ending to this exchange." }),
    );
    let doc = sp.doc();
    assert_eq!(
        doc["scenes"][0]["notes"],
        "Need a stronger ending to this exchange."
    );
    assert!(
        doc["scenes"][0]["elements"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["elementType"] != "note"),
        "notes never insert into the script"
    );
    sp.env.ok(
        "screenplay.update_scene",
        json!({ "sceneId": scene, "notes": "" }),
    );
    assert!(sp.doc()["scenes"][0]["notes"].is_null());
}

#[test]
fn fsd_script_049_title_page_fields() {
    let sp = setup("Feature Film");
    let tp = json!({ "title": "BLACK RAIN", "writtenBy": "Nisha Verma", "contact": "nisha@example.com", "draftLine": "Draft 6 — Shooting Draft", "notes": "" });
    let r = sp.env.ok(
        "screenplay.update_title_page",
        json!({ "screenplayId": sp.screenplay, "titlePage": tp }),
    );
    assert_eq!(r["titlePage"]["writtenBy"], "Nisha Verma");
    assert_eq!(r["title"], "BLACK RAIN");
    let bad = json!({ "title": " ", "writtenBy": "", "contact": "", "draftLine": "", "notes": "" });
    assert_eq!(
        sp.env.err(
            "screenplay.update_title_page",
            json!({ "screenplayId": sp.screenplay, "titlePage": bad })
        ),
        "validation.required"
    );
}

#[test]
fn character_usage_detection_with_corrections() {
    let sp = setup("Feature Film");
    let meera = new_id();
    let arjun = new_id();
    seed(
        &sp.env,
        "INSERT INTO story_character(id, name, position, created_at, updated_at) VALUES (?1, 'Meera', 1, 0, 0), (?2, 'Arjun Rao', 2, 0, 0)",
        &[&meera, &arjun],
    );
    let (s1, _) = sp.write_scene("INT. POLICE STATION — NIGHT", "x");
    let scene2 = new_id();
    sp.edits(json!([
        { "op": "insertScene", "id": scene2, "heading": "EXT. STREET — DAY" },
        { "op": "insertElement", "sceneId": scene2, "elementType": "character", "text": "Meera (V.O.)" },
        { "op": "insertElement", "sceneId": scene2, "elementType": "dialogue", "text": "Hello." },
        { "op": "insertElement", "sceneId": scene2, "elementType": "character", "text": "ARJUN" },
        { "op": "insertElement", "sceneId": scene2, "elementType": "character", "text": "RADIO" },
    ]));
    let rep = sp
        .env
        .ok("screenplay.characters", json!({ "draftId": sp.draft }));
    let cue = |name: &str| {
        rep["cues"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["cueName"] == name)
            .unwrap()
            .clone()
    };
    let m = cue("MEERA");
    assert_eq!(m["characterId"], meera);
    assert_eq!(m["matchKind"], "name");
    assert_eq!(m["speeches"], 2);
    let scene_ids: Vec<&str> = m["scenes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["sceneId"].as_str().unwrap())
        .collect();
    assert_eq!(scene_ids, [s1.as_str(), scene2.as_str()]);
    assert_eq!(cue("ARJUN")["matchKind"], "first_name");
    assert_eq!(cue("RADIO")["matchKind"], "none");
    // Corrections.
    sp.env.ok(
        "screenplay.set_character_link",
        json!({ "screenplayId": sp.screenplay, "cueName": "radio", "ignored": true }),
    );
    sp.env.ok(
        "screenplay.set_character_link",
        json!({ "screenplayId": sp.screenplay, "cueName": "ARJUN", "characterId": arjun }),
    );
    let rep = sp
        .env
        .ok("screenplay.characters", json!({ "draftId": sp.draft }));
    let cue = |name: &str| {
        rep["cues"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["cueName"] == name)
            .unwrap()
            .clone()
    };
    assert_eq!(cue("RADIO")["matchKind"], "ignored");
    assert_eq!(cue("ARJUN")["matchKind"], "manual");
    assert_eq!(cue("ARJUN")["characterName"], "Arjun Rao");
    sp.env.ok(
        "screenplay.clear_character_link",
        json!({ "screenplayId": sp.screenplay, "cueName": "RADIO" }),
    );
    let rep = sp
        .env
        .ok("screenplay.characters", json!({ "draftId": sp.draft }));
    assert_eq!(
        rep["cues"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["cueName"] == "RADIO")
            .unwrap()["matchKind"],
        "none"
    );
    assert_eq!(rep["characters"].as_array().unwrap().len(), 2);
}

#[test]
fn layout_is_remembered_per_user() {
    let sp = setup("Feature Film");
    assert!(sp.env.ok("screenplay.view_state", json!({})).is_null());
    sp.env.ok("screenplay.set_view_state", json!({ "state": { "mode": "writingRoom", "panels": ["notes", "comments"], "showNotes": false } }));
    let path = sp.env.project_path();
    let env = sp.env.restart();
    env.reopen_project(&path);
    let v = env.ok("screenplay.view_state", json!({}));
    assert_eq!(v["mode"], "writingRoom");
    assert_eq!(v["panels"], json!(["notes", "comments"]));
    let other = env.other_user(Role::Editor);
    assert!(
        env.call_as(&other, "screenplay.view_state", json!({}))
            .unwrap()
            .is_null()
    );
    assert_eq!(
        env.err("screenplay.set_view_state", json!({ "state": [1, 2] })),
        "validation.invalid_input"
    );
}

#[test]
fn scene_hub_reports_context_without_editing() {
    let sp = setup("Feature Film");
    let (scene, _) = sp.write_scene("INT. POLICE STATION — NIGHT", "x");
    sp.env.ok(
        "comment.create",
        json!({ "targetType": "screenplay_scene", "targetId": scene, "body": "One" }),
    );
    let hub = sp
        .env
        .ok("screenplay.scene_hub", json!({ "sceneId": scene }));
    assert_eq!(hub["number"], 2);
    assert_eq!(hub["draftName"], "Draft 1");
    let comments = hub["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["key"] == "comments")
        .unwrap();
    assert_eq!(comments["label"], "Comments (1)");
    assert_eq!(comments["detail"], "1 open");
    assert_eq!(hub["usedInProduction"], false);
    let refs = sp
        .env
        .ok("screenplay.story_reference", json!({ "sceneId": scene }));
    assert!(refs["linked"].is_null());
}

#[test]
fn stale_reads_are_detectable_by_edit_sequence() {
    let sp = setup("Feature Film");
    let s0 = sp.doc()["seq"].as_i64().unwrap();
    let el = sp.doc()["scenes"][0]["elements"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let r = sp.edits(json!([{ "op": "updateElement", "id": el, "text": "x" }]));
    assert!(r["seq"].as_i64().unwrap() > s0);
    assert_eq!(sp.doc()["seq"], r["seq"]);
    // An empty batch reports the current sequence without a mutation.
    let r2 = sp.edits(json!([]));
    assert_eq!(r2["seq"], r["seq"]);
}
