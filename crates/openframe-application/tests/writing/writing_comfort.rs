//! Writing comfort acceptance tests (build plan area "Writing comfort"; features SCRIPT-004,
//! SCRIPT-017, SCRIPT-018, SCRIPT-019, SCRIPT-020), through the public operation registry: the
//! same path as the UI.

use openframe_domain::{Role, new_id};
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

struct Sp {
    env: TestEnv,
    draft: String,
}

fn setup() -> Sp {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let created = env.ok("screenplay.create", json!({}));
    Sp {
        draft: created["draft"]["id"].as_str().unwrap().to_string(),
        env,
    }
}

impl Sp {
    fn doc(&self) -> Value {
        self.env
            .ok("screenplay.document", json!({ "draftId": self.draft }))
    }
    fn edits(&self, ops: Value) {
        self.env.ok(
            "screenplay.apply_edits",
            json!({ "draftId": self.draft, "ops": ops }),
        );
    }
    /// Heading plus one action line; returns the scene id.
    fn scene(&self, heading: &str, action: &str) -> String {
        let id = new_id();
        self.edits(json!([
            { "op": "insertScene", "id": id, "heading": heading },
            { "op": "insertElement", "sceneId": id, "elementType": "action", "text": action },
        ]));
        id
    }
    fn texts(&self) -> Vec<String> {
        let d = self.doc();
        let mut out = Vec::new();
        for s in d["scenes"].as_array().unwrap() {
            out.push(s["heading"].as_str().unwrap().to_string());
            for e in s["elements"].as_array().unwrap() {
                out.push(e["text"].as_str().unwrap().to_string());
            }
        }
        out
    }
}

// ------------------------------------------------------------ find and replace scope

#[test]
fn fsd_script_004_replace_all_in_this_scene_only_and_one_undo() {
    let sp = setup();
    let a = sp.scene("INT. DEPOT - NIGHT", "The bus waits. The bus leaves.");
    let _b = sp.scene("EXT. ROAD - NIGHT", "Another bus passes.");
    let r = sp.env.ok(
        "screenplay.replace_all",
        json!({ "draftId": sp.draft, "query": "bus", "replacement": "van", "sceneId": a }),
    );
    assert_eq!(r["count"], 2, "only the two matches in the chosen scene");
    let t = sp.texts();
    assert!(
        t.contains(&"The van waits. The van leaves.".to_string()),
        "{t:?}"
    );
    assert!(t.contains(&"Another bus passes.".to_string()), "{t:?}");
    let step = sp.env.undo();
    assert!(
        step["label"].as_str().unwrap().contains("in one scene"),
        "{step}"
    );
    assert!(
        sp.texts()
            .contains(&"The bus waits. The bus leaves.".to_string())
    );
    sp.env.redo();
    assert!(
        sp.texts()
            .contains(&"The van waits. The van leaves.".to_string())
    );
    // Whole script still replaces everywhere, as one step.
    let r = sp.env.ok(
        "screenplay.replace_all",
        json!({ "draftId": sp.draft, "query": "van", "replacement": "bus" }),
    );
    assert_eq!(r["count"], 2);
    let r = sp.env.ok(
        "screenplay.replace_all",
        json!({ "draftId": sp.draft, "query": "bus", "replacement": "car" }),
    );
    assert_eq!(r["count"], 3);
}

#[test]
fn fsd_script_004_scene_scope_refuses_a_scene_of_another_draft_and_respects_roles() {
    let sp = setup();
    let a = sp.scene("INT. DEPOT - NIGHT", "The bus waits.");
    // A scene id that is not in this draft changes nothing.
    assert!(
        sp.env.err(
            "screenplay.replace_all",
            json!({ "draftId": sp.draft, "query": "bus", "replacement": "van", "sceneId": new_id() }),
        )
            .starts_with("not_found")
    );
    assert!(
        sp.env.err(
            "screenplay.replace_all",
            json!({ "draftId": sp.draft, "query": "bus", "replacement": "van", "sceneId": "x'; DROP" }),
        )
        .starts_with("not_found"),
        "a malformed id is refused before any query"
    );
    let viewer = sp.env.actor_with_role(Role::Viewer);
    assert!(
        sp.env
            .call_as(
                &viewer,
                "screenplay.replace_all",
                json!({ "draftId": sp.draft, "query": "bus", "replacement": "van", "sceneId": a }),
            )
            .is_err()
    );
    // Unicode: Telugu text is replaced inside the scene only.
    sp.edits(json!([{ "op": "insertElement", "sceneId": a, "elementType": "dialogue", "text": "బస్సు వచ్చింది" }]));
    let r = sp.env.ok(
        "screenplay.replace_all",
        json!({ "draftId": sp.draft, "query": "బస్సు", "replacement": "రైలు", "sceneId": a }),
    );
    assert_eq!(r["count"], 1);
    assert!(sp.texts().contains(&"రైలు వచ్చింది".to_string()));
    // Locked drafts refuse.
    sp.env
        .ok("screenplay.lock_draft", json!({ "draftId": sp.draft }));
    assert_eq!(
        sp.env.err(
            "screenplay.replace_all",
            json!({ "draftId": sp.draft, "query": "bus", "replacement": "van", "sceneId": a }),
        ),
        "permission.locked_draft"
    );
}

// ------------------------------------------------------------ smart suggestions

#[test]
fn fsd_script_018_suggestion_sources_come_from_story_cast_and_production() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let meera = env.ok("story.create_character", json!({ "name": "Meera" }));
    env.ok("story.create_character", json!({ "name": "Ravi Kumar" }));
    let old = env.ok("story.create_character", json!({ "name": "Old Friend" }));
    env.ok(
        "story.set_character_archived",
        json!({ "id": old["id"], "archived": true }),
    );
    env.ok(
        "cast.create",
        json!({ "personName": "Arjun Rao", "characterName": "conductor" }),
    );
    env.ok(
        "cast.create",
        json!({ "personName": "Lakshmi", "characterId": meera["id"] }),
    );
    env.ok("locations.create", json!({ "name": "Old Railway Station" }));
    env.ok("locations.create", json!({ "name": "రైల్వే స్టేషన్" }));
    env.ok(
        "story.create_card",
        json!({ "shortDescription": "Depot at night", "sceneHeading": "INT. BUS DEPOT - NIGHT" }),
    );
    env.ok(
        "story.create_card",
        json!({ "shortDescription": "Same place", "sceneHeading": "int. old railway station - day" }),
    );
    let s = env.ok("writing.suggestion_sources", json!({}));
    let names = |k: &str| -> Vec<(String, String)> {
        s[k].as_array()
            .unwrap()
            .iter()
            .map(|v| {
                (
                    v["name"].as_str().unwrap().to_string(),
                    v["source"].as_str().unwrap().to_string(),
                )
            })
            .collect()
    };
    assert_eq!(
        names("characters"),
        vec![
            ("MEERA".into(), "Story".into()),
            ("RAVI KUMAR".into(), "Story".into()),
            ("CONDUCTOR".into(), "Cast".into()),
        ],
        "archived characters are left out; a cast-linked character is not repeated"
    );
    assert_eq!(
        names("locations"),
        vec![
            ("OLD RAILWAY STATION".into(), "Production".into()),
            ("రైల్వే స్టేషన్".into(), "Production".into()),
            ("BUS DEPOT".into(), "Story".into()),
        ],
        "story headings add their location once, case-insensitively"
    );
    // Any role that can read the project can get the list; no AI involved.
    let viewer = env.actor_with_role(Role::Viewer);
    assert!(
        env.call_as(&viewer, "writing.suggestion_sources", json!({}))
            .is_ok()
    );
    assert!(
        env.call("writing.suggestion_sources", json!({ "extra": 1 }))
            .is_err(),
        "unknown arguments are refused"
    );
}

// ------------------------------------------------------------ navigator columns

#[test]
fn fsd_script_017_navigator_columns_pages_notes_and_absent_production_numbers() {
    let sp = setup();
    let first = sp.doc()["scenes"][0]["id"].as_str().unwrap().to_string();
    sp.edits(json!([{ "op": "updateScene", "id": first, "heading": "INT. DEPOT - NIGHT" }]));
    let long = sp.scene("EXT. ROAD - NIGHT", "Rain.");
    // Enough action to push the next scene onto a later page.
    let ops: Vec<Value> = (0..40)
        .map(|i| json!({ "op": "insertElement", "sceneId": long, "elementType": "action", "text": format!("The bus crawls through the storm, line {i}, wipers losing the fight.") }))
        .collect();
    sp.edits(Value::Array(ops));
    let last = sp.scene("INT. BUS - CONTINUOUS", "Meera sleeps.");
    sp.edits(json!([
        { "op": "insertElement", "sceneId": last, "elementType": "note", "text": "Check the bus model." },
        { "op": "insertElement", "sceneId": last, "elementType": "note", "text": "   " },
    ]));
    sp.env.ok(
        "screenplay.update_scene",
        json!({ "sceneId": last, "notes": "Night exterior lighting." }),
    );
    let cols = sp
        .env
        .ok("writing.navigator_columns", json!({ "draftId": sp.draft }));
    let rows = cols["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0]["page"], 1);
    assert_eq!(rows[1]["page"], 1);
    assert!(
        rows[2]["page"].as_u64().unwrap() >= 2,
        "the long scene pushes the last scene to a later page: {cols}"
    );
    assert_eq!(cols["pageCount"], rows[2]["page"]);
    assert_eq!(rows[2]["notes"], 2, "one script note plus the scene notes");
    assert_eq!(rows[0]["notes"], 0);
    assert_eq!(cols["hasRevisions"], false);
    assert_eq!(cols["hasProductionNumbers"], false);
    assert!(rows.iter().all(|r| r["productionNumber"].is_null()));
    // Viewer can read; unknown draft is not found.
    let viewer = sp.env.actor_with_role(Role::Viewer);
    assert!(
        sp.env
            .call_as(
                &viewer,
                "writing.navigator_columns",
                json!({ "draftId": sp.draft })
            )
            .is_ok()
    );
    assert!(
        sp.env
            .err("writing.navigator_columns", json!({ "draftId": new_id() }))
            .starts_with("not_found")
    );
}

#[test]
fn fsd_script_017_navigator_revision_column_and_production_numbers_when_present() {
    let sp = setup();
    let first = sp.doc()["scenes"][0]["id"].as_str().unwrap().to_string();
    sp.edits(json!([{ "op": "updateScene", "id": first, "heading": "INT. DEPOT - NIGHT" }]));
    sp.scene("EXT. ROAD - NIGHT", "Rain.");
    sp.env
        .ok("screenplay.lock_draft", json!({ "draftId": sp.draft }));
    let rev = sp.env.ok(
        "screenplay.start_revision",
        json!({ "draftId": sp.draft, "label": "Blue Revision", "color": "blue" }),
    );
    let rid = rev["id"].as_str().unwrap().to_string();
    let rdoc = sp.env.ok("screenplay.document", json!({ "draftId": rid }));
    let road = rdoc["scenes"][1]["id"].as_str().unwrap().to_string();
    let el = rdoc["scenes"][1]["elements"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    sp.env.ok(
        "screenplay.apply_edits",
        json!({ "draftId": rid, "ops": [{ "op": "updateElement", "id": el, "text": "Heavy rain." }] }),
    );
    let cols = sp
        .env
        .ok("writing.navigator_columns", json!({ "draftId": rid }));
    assert_eq!(cols["hasRevisions"], true);
    let rows = cols["rows"].as_array().unwrap();
    assert!(rows[0]["revision"].is_null(), "unchanged scene: {cols}");
    assert_eq!(rows[1]["sceneId"], road);
    assert_eq!(rows[1]["revision"]["kind"], "Changed");
    assert_eq!(rows[1]["revision"]["label"], "Blue Revision");

    // Production numbers (ADR-0015) were frozen when the draft was locked; a lettered label
    // shows as stored.
    let session = sp.env.core.project().unwrap();
    session
        .store
        .with_writer(|c| {
            c.execute(
                "UPDATE screenplay_scene SET production_number='12A' WHERE id=?1",
                [&road],
            )?;
            Ok(())
        })
        .unwrap();
    let cols = sp
        .env
        .ok("writing.navigator_columns", json!({ "draftId": rid }));
    assert_eq!(cols["hasProductionNumbers"], true);
    assert_eq!(cols["rows"][1]["productionNumber"], "12A");
    assert_eq!(
        cols["rows"][0]["productionNumber"], "1",
        "frozen at lock: {cols}"
    );
}

// ------------------------------------------------------------ proofing

#[test]
fn fsd_script_020_proofing_settings_are_undoable_validated_and_survive_restart() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let p = env.ok("writing.proofing", json!({}));
    assert_eq!(p["settings"]["enabled"], true);
    assert_eq!(p["settings"]["mainLanguage"], "en-US");
    assert!(p["settings"]["secondLanguage"].is_null());
    assert!(p["installed"].is_array());
    env.ok(
        "writing.set_proofing",
        json!({ "mainLanguage": "en-GB", "secondLanguage": "te-IN" }),
    );
    env.ok("writing.set_proofing", json!({ "enabled": false }));
    let s = &env.ok("writing.proofing", json!({}))["settings"];
    assert_eq!(s["enabled"], false);
    assert_eq!(s["mainLanguage"], "en-GB");
    assert_eq!(s["secondLanguage"], "te-IN");
    let step = env.undo();
    assert_eq!(step["label"], "Turned spell checking off");
    assert_eq!(
        env.ok("writing.proofing", json!({}))["settings"]["enabled"],
        true
    );
    // Clearing the second language; the same language twice collapses to one.
    env.ok("writing.set_proofing", json!({ "secondLanguage": "" }));
    assert!(env.ok("writing.proofing", json!({}))["settings"]["secondLanguage"].is_null());
    env.ok("writing.set_proofing", json!({ "secondLanguage": "en-GB" }));
    assert!(env.ok("writing.proofing", json!({}))["settings"]["secondLanguage"].is_null());
    for bad in ["english", "EN-us", "en_US", "", "x"] {
        assert!(
            env.err("writing.set_proofing", json!({ "mainLanguage": bad }))
                .starts_with("validation."),
            "{bad}"
        );
    }
    // Viewers and Commenters cannot change project proofing.
    for role in [Role::Viewer, Role::Commenter] {
        let a = env.actor_with_role(role);
        assert!(
            env.call_as(&a, "writing.set_proofing", json!({ "enabled": false }))
                .is_err()
        );
    }
    // Survives a restart and a crash.
    env.ok("writing.set_proofing", json!({ "mainLanguage": "hi-IN" }));
    let path = env.project_path();
    let env = env.restart();
    env.reopen_project(&path);
    assert_eq!(
        env.ok("writing.proofing", json!({}))["settings"]["mainLanguage"],
        "hi-IN"
    );
    let env = env.crash_and_restart();
    env.reopen_project(&path);
    assert_eq!(
        env.ok("writing.proofing", json!({}))["settings"]["mainLanguage"],
        "hi-IN"
    );
}

fn misses(r: &Value, key: &str) -> Vec<(u64, u64)> {
    r["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["key"] == key)
        .unwrap()["misses"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| (m["start"].as_u64().unwrap(), m["end"].as_u64().unwrap()))
        .collect()
}

#[test]
fn fsd_script_020_check_spelling_with_personal_dictionary_and_indian_scripts() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let installed: Vec<String> = env.ok("writing.proofing", json!({}))["installed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let telugu = "మీరా బస్సు ఎక్కింది"; // Telugu, usually no dictionary installed
    let hindi = "मीरा बस में चढ़ी";
    let items = json!([
        { "key": "a", "text": "Meera walks into teh depot." },
        { "key": "t", "text": telugu },
        { "key": "h", "text": hindi },
        { "key": "k", "text": "Meera meets Kothapeta Raju." },
        { "key": "e", "text": "" },
    ]);
    let r = env.ok("writing.check_spelling", json!({ "items": items }));
    assert_eq!(r["enabled"], true);
    if !installed.iter().any(|l| l.starts_with("te")) {
        assert!(
            misses(&r, "t").is_empty(),
            "no Telugu dictionary: no underlines"
        );
    }
    if !installed.iter().any(|l| l.starts_with("hi")) {
        assert!(
            misses(&r, "h").is_empty(),
            "no Hindi dictionary: no underlines"
        );
    }
    assert!(misses(&r, "e").is_empty());
    if installed.iter().any(|l| l == "en-US") {
        // UTF-16 offsets of "teh".
        assert!(misses(&r, "a").contains(&(17, 20)), "{r}");
        let before = misses(&r, "k");
        assert!(before.contains(&(12, 21)), "Kothapeta is unknown: {r}");
        env.ok("writing.dictionary_add", json!({ "word": "kothapeta" }));
        let r = env.ok("writing.check_spelling", json!({ "items": items }));
        assert!(
            !misses(&r, "k").contains(&(12, 21)),
            "personal dictionary words are not underlined, case-insensitively"
        );
        let sug = env.ok("writing.spelling_suggestions", json!({ "word": "teh" }));
        assert!(
            sug["suggestions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|s| s == "the"),
            "{sug}"
        );
        // Main language not installed: reported, nothing underlined.
        env.ok("writing.set_proofing", json!({ "mainLanguage": "zz-ZZ" }));
        let r = env.ok("writing.check_spelling", json!({ "items": items }));
        assert_eq!(r["missingLanguages"], json!(["zz-ZZ"]));
        assert!(misses(&r, "a").is_empty());
    }
    // Turned off: nothing is checked.
    env.ok("writing.set_proofing", json!({ "enabled": false }));
    let r = env.ok("writing.check_spelling", json!({ "items": items }));
    assert_eq!(r["enabled"], false);
    assert!(r["results"].as_array().unwrap().is_empty());
    // Bounds: too many items or too much text is refused, not truncated silently.
    env.ok("writing.set_proofing", json!({ "enabled": true }));
    let many: Vec<Value> = (0..401)
        .map(|i| json!({ "key": format!("k{i}"), "text": "x" }))
        .collect();
    assert!(
        env.err("writing.check_spelling", json!({ "items": many }))
            .starts_with("validation.")
    );
    let huge = "word ".repeat(5_000);
    assert!(
        env.err(
            "writing.check_spelling",
            json!({ "items": [{ "key": "x", "text": huge }] })
        )
        .starts_with("validation.")
    );
    // Viewers can check (it changes nothing).
    let viewer = env.actor_with_role(Role::Viewer);
    assert!(
        env.call_as(&viewer, "writing.check_spelling", json!({ "items": [] }))
            .is_ok()
    );
}

#[test]
fn fsd_script_020_personal_dictionary_is_personal_bounded_and_persistent() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    assert_eq!(env.ok("writing.dictionary", json!({}))["words"], json!([]));
    env.ok("writing.dictionary_add", json!({ "word": "Kothapeta" }));
    env.ok("writing.dictionary_add", json!({ "word": "KOTHAPETA" }));
    env.ok("writing.dictionary_add", json!({ "word": "నమస్తే" }));
    env.ok("writing.dictionary_add", json!({ "word": "  Anjali  " }));
    assert_eq!(
        env.ok("writing.dictionary", json!({}))["words"],
        json!(["Anjali", "Kothapeta", "నమస్తే"]),
        "sorted, trimmed, one entry per word regardless of case"
    );
    for bad in ["two words", "", "123", &"a".repeat(65)] {
        assert!(
            env.err("writing.dictionary_add", json!({ "word": bad }))
                .starts_with("validation."),
            "{bad}"
        );
    }
    // Someone else's package changes can never edit this person's dictionary.
    let other = env.other_user(Role::Editor);
    assert!(
        env.call_as(&other, "writing.dictionary_add", json!({ "word": "Spy" }))
            .is_err()
    );
    env.ok("writing.dictionary_remove", json!({ "word": "anjali" }));
    // The dictionary is application data: it survives restarts and works with no project open.
    let env = env.restart();
    assert_eq!(
        env.ok("writing.dictionary", json!({}))["words"],
        json!(["Kothapeta", "నమస్తే"])
    );
}

// ------------------------------------------------------------ keyboard shortcuts

#[test]
fn fsd_script_019_shortcuts_remap_remove_restore_and_persist() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    assert_eq!(env.ok("app.shortcuts", json!({}))["overrides"], json!([]));
    // Remap with two keys (several per command), canonical form stored.
    env.ok(
        "app.set_shortcuts",
        json!({ "changes": [{ "commandId": "screenplay.find", "keys": ["shift+mod+f", "F3"] }] }),
    );
    // Remove a shortcut (empty list) and move a key between commands in one step.
    env.ok(
        "app.set_shortcuts",
        json!({ "changes": [
            { "commandId": "app.assistant", "keys": [] },
            { "commandId": "screenplay.element_shot", "keys": ["Mod+Shift+7"] },
        ] }),
    );
    let o = env.ok("app.shortcuts", json!({}));
    assert_eq!(
        o["overrides"],
        json!([
            { "commandId": "app.assistant", "keys": [] },
            { "commandId": "screenplay.element_shot", "keys": ["Mod+Shift+7"] },
            { "commandId": "screenplay.find", "keys": ["Mod+Shift+F", "F3"] },
        ])
    );
    // A bad entry anywhere in a batch changes nothing.
    assert!(
        env.err(
            "app.set_shortcuts",
            json!({ "changes": [
                { "commandId": "screenplay.find", "keys": ["F4"] },
                { "commandId": "app.search", "keys": ["K"] },
            ] }),
        )
        .starts_with("validation.")
    );
    // Ctrl+Alt+1 to 9 belong to custom elements: refused with a conflict message.
    let e = env
        .call(
            "app.set_shortcuts",
            json!({ "changes": [{ "commandId": "screenplay.find", "keys": ["Mod+Alt+3"] }] }),
        )
        .unwrap_err();
    assert_eq!(e.code.0, "validation.keys");
    assert!(e.message.contains("custom elements"), "{}", e.message);
    // The page break is an ordinary command: it can move to another key.
    env.ok(
        "app.set_shortcuts",
        json!({ "changes": [{ "commandId": "screenplay.page_break", "keys": ["Mod+Shift+Enter"] }] }),
    );
    env.ok(
        "app.set_shortcuts",
        json!({ "changes": [{ "commandId": "screenplay.page_break", "keys": null }] }),
    );
    for bad in [
        json!({ "changes": [{ "commandId": "app.search", "keys": ["Mod+C"] }] }),
        json!({ "changes": [{ "commandId": "app.search", "keys": ["mod+alt+9"] }] }),
        json!({ "changes": [{ "commandId": "app.search", "keys": ["Mod+K", "mod+k"] }] }),
        json!({ "changes": [{ "commandId": "app.search", "keys": ["F1", "F2", "F3", "F4", "F5"] }] }),
        json!({ "changes": [{ "commandId": "Not An Id", "keys": ["F1"] }] }),
        json!({ "changes": [] }),
    ] {
        assert!(env.call("app.set_shortcuts", bad.clone()).is_err(), "{bad}");
    }
    assert_eq!(env.ok("app.shortcuts", json!({})), o);
    // Restore one default.
    env.ok(
        "app.set_shortcuts",
        json!({ "changes": [{ "commandId": "app.assistant", "keys": null }] }),
    );
    assert_eq!(
        env.ok("app.shortcuts", json!({}))["overrides"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    // Someone else's package changes can never change this person's shortcuts.
    let other = env.other_user(Role::Owner);
    assert!(
        env.call_as(&other, "app.reset_shortcuts", json!({}))
            .is_err()
    );
    // Survives a restart, then restore all.
    let env = env.restart();
    assert_eq!(
        env.ok("app.shortcuts", json!({}))["overrides"][1]["keys"],
        json!(["Mod+Shift+F", "F3"])
    );
    env.ok("app.reset_shortcuts", json!({}));
    assert_eq!(env.ok("app.shortcuts", json!({}))["overrides"], json!([]));
}

#[test]
fn fsd_script_019_damaged_stored_shortcuts_are_ignored_not_applied() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    env.core
        .with_app_db(|c| {
            c.execute(
                "INSERT INTO sys_settings(key, value_json, updated_at) VALUES ('keyboard.shortcuts', ?1, 0)",
                [r#"{"app.search":["K"],"screenplay.find":["Mod+F"],"bad id":["F1"]}"#],
            )?;
            Ok(())
        })
        .unwrap();
    assert_eq!(
        env.ok("app.shortcuts", json!({}))["overrides"],
        json!([{ "commandId": "screenplay.find", "keys": ["Mod+F"] }])
    );
    env.core
        .with_app_db(|c| {
            c.execute(
                "UPDATE sys_settings SET value_json='not json' WHERE key='keyboard.shortcuts'",
                [],
            )?;
            Ok(())
        })
        .unwrap();
    assert_eq!(env.ok("app.shortcuts", json!({}))["overrides"], json!([]));
}

#[test]
fn fsd_script_019_020_personal_preferences_work_with_no_project_open_and_are_bounded() {
    // Home shortcuts and the personal dictionary are application data: no project needed.
    let env = TestEnv::new();
    env.ok(
        "app.set_shortcuts",
        json!({ "changes": [{ "commandId": "home.new_project", "keys": ["Mod+Alt+N"] }] }),
    );
    assert_eq!(
        env.ok("app.shortcuts", json!({}))["overrides"],
        json!([{ "commandId": "home.new_project", "keys": ["Mod+Alt+N"] }])
    );
    env.ok("writing.dictionary_add", json!({ "word": "Kothapeta" }));
    assert_eq!(
        env.ok("writing.dictionary", json!({}))["words"],
        json!(["Kothapeta"])
    );
    // Project-level proofing needs a project.
    assert!(
        env.err("writing.proofing", json!({}))
            .starts_with("not_found")
    );
    // Someone else's package changes cannot set shortcuts either.
    let other = env.other_user(Role::Owner);
    assert!(
        env.call_as(
            &other,
            "app.set_shortcuts",
            json!({ "changes": [{ "commandId": "app.search", "keys": ["F2"] }] })
        )
        .is_err()
    );
    // One call changes at most 50 commands, and a refused call changes nothing.
    let many: Vec<Value> = (0..51)
        .map(|i| json!({ "commandId": format!("app.c{i}"), "keys": ["F2"] }))
        .collect();
    assert!(
        env.err("app.set_shortcuts", json!({ "changes": many }))
            .starts_with("validation.")
    );
    assert_eq!(
        env.ok("app.shortcuts", json!({}))["overrides"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
