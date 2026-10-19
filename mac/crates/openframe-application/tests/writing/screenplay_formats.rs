//! Screenplay file formats through the operation registry (the same path as the UI):
//!
//! - multilingual text survives typing, save, restart, search, assistant retrieval and the
//!   PDF, Final Draft, Fountain and DOCX round trips (ADR-0016; feature SCRIPT-UNI-001);
//! - the Final Draft migration definition of done (Screenplay and Production Depth
//!   Specification §50): every step a Final Draft user takes, except reopening the export in a
//!   licensed copy of Final Draft, which is a manual check.

use openframe_application::modules::ai::scope::{self, AiScopeArgs, AiScopeKind};
use openframe_application::modules::ai::toolbox::{self, ToolStep};
use openframe_domain::new_id;
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

const TELUGU: &str = "మీరా రైలు స్టేషన్\u{200C}లో వేచి ఉంది. క్షమించండి!";
const HINDI: &str = "मीरा ने कहा, “क्षमा करें, ट्रेन देर से है।”";
const TAMIL: &str = "நீ எங்கே போகிறாய்? ஸ்ரீ வருகிறார்.";
const MALAYALAM: &str = "അവന്\u{200D} വന്നു. അവൻ പോയി.";
const KANNADA: &str = "ನಾನು ಬರುತ್ತೇನೆ, ಕ್ಷಮಿಸಿ.";
const HEADING: &str = "INT. రైల్వే స్టేషన్ - NIGHT";

fn s(v: &Value) -> String {
    v.as_str()
        .unwrap_or_else(|| panic!("expected string, got {v}"))
        .to_string()
}

/// Every element text of a draft, in order, with the scene headings.
fn texts(env: &TestEnv, draft: &str) -> Vec<String> {
    let doc = env.ok("screenplay.document", json!({ "draftId": draft }));
    let mut out = Vec::new();
    for sc in doc["scenes"].as_array().unwrap() {
        out.push(s(&sc["heading"]));
        for e in sc["elements"].as_array().unwrap() {
            out.push(s(&e["text"]));
        }
    }
    out
}

fn words(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .flat_map(|l| l.split_whitespace().map(str::to_string).collect::<Vec<_>>())
        .collect()
}

#[track_caller]
fn assert_same_words(what: &str, want: &[String], got: &[String]) {
    let want = words(want);
    let got: Vec<String> = words(got).iter().map(|w| w.to_uppercase()).collect();
    let mut at = 0;
    for w in &want {
        let w = w.to_uppercase();
        match got[at..].iter().position(|g| *g == w) {
            Some(i) => at += i + 1,
            None => panic!("{what}: lost or changed {w:?}"),
        }
    }
}

fn written(env: &TestEnv) -> (String, String) {
    let created = env.ok("screenplay.create", json!({ "title": "రైలు" }));
    let draft = s(&created["draft"]["id"]);
    let blank = s(&env.ok("screenplay.document", json!({ "draftId": draft }))["scenes"][0]["id"]);
    let scene = new_id();
    env.ok(
        "screenplay.apply_edits",
        json!({ "draftId": draft, "ops": [
            { "op": "insertScene", "id": scene, "index": 0, "heading": HEADING },
            { "op": "insertElement", "sceneId": scene, "elementType": "action",
              "text": "Rain on the platform. మీరా looks at अर्जुन, then at the clock." },
            { "op": "insertElement", "sceneId": scene, "elementType": "character", "text": "మీరా" },
            { "op": "insertElement", "sceneId": scene, "elementType": "dialogue", "text": TELUGU },
            { "op": "insertElement", "sceneId": scene, "elementType": "character", "text": "अर्जुन" },
            { "op": "insertElement", "sceneId": scene, "elementType": "dialogue", "text": HINDI },
            { "op": "insertElement", "sceneId": scene, "elementType": "action", "text": TAMIL },
            { "op": "insertElement", "sceneId": scene, "elementType": "action", "text": MALAYALAM },
            { "op": "insertElement", "sceneId": scene, "elementType": "character", "text": "ರವಿ" },
            { "op": "insertElement", "sceneId": scene, "elementType": "dialogue", "text": KANNADA },
            { "op": "deleteScene", "id": blank },
        ] }),
    );
    (s(&created["screenplay"]["id"]), draft)
}

fn retrieve(env: &TestEnv, query: &str) -> String {
    let actor = env.actor();
    let sc = env
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
    match toolbox::run_tool_unlocked(
        &env.core,
        &actor,
        &sc,
        "test request",
        "retrieve_context",
        &json!({ "query": query }),
    ) {
        Ok(ToolStep::Output(o)) => {
            let mut out = o.content.clone();
            for d in &o.details {
                out.push('\n');
                out.push_str(d);
            }
            for it in &o.items {
                out.push('\n');
                out.push_str(&it.label);
                out.push(' ');
                out.push_str(&it.detail.clone().unwrap_or_default());
            }
            out
        }
        other => panic!("retrieve_context: {other:?}"),
    }
}

#[test]
fn multilingual_text_survives_save_restart_search_retrieval_and_every_format() {
    let env = TestEnv::with_project("రైలు", "Feature Film");
    let (screenplay, draft) = written(&env);
    let typed = texts(&env, &draft);
    assert!(typed.iter().any(|t| t == TELUGU) && typed.iter().any(|t| t == MALAYALAM));

    // Save, quit and reopen: every character, joiners included, comes back.
    env.ok("project.save", json!({}));
    let path = env.project_path();
    let env = env.restart();
    env.reopen_project(&path);
    assert_eq!(texts(&env, &draft), typed, "reload keeps every character");

    // Search finds a Telugu and a Kannada word; the assistant's retrieval returns the line.
    for word in ["స్టేషన్\u{200C}లో", "ಬರುತ್ತೇನೆ"] {
        let hits = env.ok("search.query", json!({ "text": word }));
        assert!(
            !hits.as_array().unwrap().is_empty(),
            "search finds {word}: {hits}"
        );
    }
    let found = retrieve(&env, "మీరా రైలు");
    assert!(
        found.contains("మీరా"),
        "retrieval returns the Telugu line: {found}"
    );

    // Every format: export, re-import as a new draft, compare.
    let out = env.dir.path().join("exports");
    std::fs::create_dir_all(&out).unwrap();
    for (format, ext) in [
        ("pdf", "pdf"),
        ("fdx", "fdx"),
        ("fountain", "fountain"),
        ("docx", "docx"),
    ] {
        let dest = out.join(format!("rail.{ext}"));
        env.ok(
            "screenplay.export",
            json!({ "draftId": draft, "format": format, "path": dest.to_string_lossy() }),
        );
        let prev = env.ok(
            "screenplay.import_preview",
            json!({ "path": dest.to_string_lossy() }),
        );
        let rep = env.ok(
            "screenplay.import_apply",
            json!({ "previewId": prev["previewId"], "mode": "new_draft", "screenplayId": screenplay }),
        );
        let back = texts(&env, &s(&rep["draftId"]));
        assert_same_words(format, &typed, &back);
        let joined = back.join(" ");
        assert!(
            joined.contains('\u{200C}') && joined.contains('\u{200D}'),
            "{format} keeps joiners"
        );
    }
}

// ------------------------------------------------------------- Final Draft migration (§50)

/// A Final Draft file with standard elements, two revision sets, scene numbers (one lettered),
/// two custom elements with their settings, a title page and dual dialogue.
const FINAL_DRAFT_FILE: &str = r##"<?xml version="1.0" encoding="UTF-8" standalone="no" ?>
<FinalDraft DocumentType="Script" Template="No" Version="5">
<Content>
<Paragraph Number="1" Type="Scene Heading"><SceneProperties Length="1/8" Page="1" Title=""/><Text>INT. SIGNAL BOX - NIGHT</Text></Paragraph>
<Paragraph Type="Action"><Text>A single lamp. ASHA watches the rails.</Text></Paragraph>
<Paragraph Type="Character"><Text>ASHA</Text></Paragraph>
<Paragraph Type="Parenthetical"><Text>(to herself)</Text></Paragraph>
<Paragraph Type="Dialogue"><Text>The last train is late again.</Text></Paragraph>
<Paragraph Type="TELEPHONE"><Text>this is control. clear the line.</Text></Paragraph>
<Paragraph Type="Action"><Text RevisionID="1">She lifts the receiver.</Text></Paragraph>
<Paragraph Number="1A" Type="Scene Heading"><SceneProperties Length="1/8" Page="1" Title=""/><Text>EXT. PLATFORM - CONTINUOUS</Text></Paragraph>
<Paragraph Type="Lower Third"><Text>Gudur Junction, 2:14 AM</Text></Paragraph>
<Paragraph><DualDialogue>
<Paragraph Type="Character"><Text>ASHA</Text></Paragraph>
<Paragraph Type="Dialogue"><Text>Where is it?</Text></Paragraph>
<Paragraph Type="Character"><Text>RAVI</Text></Paragraph>
<Paragraph Type="Dialogue"><Text>I never had it.</Text></Paragraph>
</DualDialogue></Paragraph>
<Paragraph Type="Transition"><Text>CUT TO:</Text></Paragraph>
<Paragraph Number="2" Type="Scene Heading"><SceneProperties Length="1/8" Page="2" Title=""/><Text>INT. WAITING ROOM - NIGHT</Text></Paragraph>
<Paragraph Type="Action"><Text RevisionID="2">Empty benches. A clock ticks.</Text></Paragraph>
</Content>
<TitlePage>
<Content>
<Paragraph Alignment="Center" Type="General"><Text>THE LAST SIGNAL</Text></Paragraph>
<Paragraph Alignment="Center" Type="General"><Text>Written by</Text></Paragraph>
<Paragraph Alignment="Center" Type="General"><Text>Asha Rao</Text></Paragraph>
<Paragraph Alignment="Right" Type="General"><Text>Blue Revision</Text></Paragraph>
</Content>
</TitlePage>
<ElementSettings Type="TELEPHONE"><FontSpec Style="Italic+AllCaps"/><ParagraphSpec Alignment="Left" LeftIndent="2.50" SpaceBefore="12"/></ElementSettings>
<ElementSettings Type="Lower Third"><FontSpec Style=""/><ParagraphSpec Alignment="Center" SpaceBefore="24"/></ElementSettings>
<Revisions ActiveSet="2" Location="1" RevisionMode="Off" ShowAllMarks="No" ShowAllSets="No">
<Revision Color="#0000FFFF0000" FullRevision="No" ID="1" Mark="*" Name="Blue Rev." Style=""/>
<Revision Color="#FFFF0000FFFF" FullRevision="No" ID="2" Mark="*" Name="Pink Rev." Style=""/>
</Revisions>
</FinalDraft>
"##;

/// The structure a migration must keep: per scene its number label and heading, and per
/// element its type, text, custom identity, dual flag and revision mark.
fn structure(env: &TestEnv, draft: &str) -> Vec<Value> {
    let doc = env.ok("screenplay.document", json!({ "draftId": draft }));
    let customs: Vec<Value> = doc["customElements"].as_array().unwrap().clone();
    let identity = |id: &Value| {
        customs
            .iter()
            .find(|c| c["id"] == *id)
            .map(|c| c["exportName"].clone())
            .unwrap_or(Value::Null)
    };
    doc["scenes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|sc| {
            json!({
                "label": sc["label"],
                "heading": sc["heading"],
                "elements": sc["elements"].as_array().unwrap().iter().map(|e| json!({
                    "type": e["elementType"],
                    "text": e["text"],
                    "custom": identity(&e["customElementId"]),
                    "dual": e["dual"],
                    "revision": e["revisionMark"],
                })).collect::<Vec<_>>(),
            })
        })
        .collect()
}

fn count(env: &TestEnv, sql: &str) -> i64 {
    env.core
        .project()
        .unwrap()
        .store
        .read(|c| Ok(c.query_row(sql, [], |r| r.get::<_, i64>(0))?))
        .unwrap()
}

#[test]
fn final_draft_migration_definition_of_done() {
    let env = TestEnv::with_project("The Last Signal", "Feature Film");
    let rows = "SELECT (SELECT count(*) FROM screenplay) + (SELECT count(*) FROM screenplay_draft)
        + (SELECT count(*) FROM screenplay_scene) + (SELECT count(*) FROM screenplay_element)";
    let before = count(&env, rows);
    let source = env.write_file("The Last Signal.fdx", FINAL_DRAFT_FILE.as_bytes());

    // Steps 1 to 5: the preview shows counts and warnings before anything is written.
    let prev = env.ok("screenplay.import_preview", json!({ "path": source }));
    assert_eq!(count(&env, rows), before, "the preview writes nothing");
    assert_eq!(prev["sceneCount"], 3, "{prev}");
    let codes: Vec<String> = prev["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| s(&w["code"]))
        .collect();
    assert!(
        codes.iter().any(|c| c == "fdx_custom_element"),
        "custom elements are named: {codes:?}"
    );
    let rep = env.ok(
        "screenplay.import_apply",
        json!({ "previewId": prev["previewId"], "mode": "new_screenplay" }),
    );
    let screenplay = s(&rep["screenplayId"]);
    let draft = s(&rep["draftId"]);
    let original = structure(&env, &draft);
    // Standard elements, revisions, numbers and custom elements are all kept.
    let all = serde_json::to_string(&original).unwrap();
    for want in [
        "\"label\":\"1A\"",
        "\"custom\":\"TELEPHONE\"",
        "\"custom\":\"Lower Third\"",
        "\"revision\":\"Blue\"",
        "\"revision\":\"Pink\"",
        "\"dual\":true",
        "\"type\":\"parenthetical\"",
        "\"type\":\"transition\"",
    ] {
        assert!(all.contains(want), "{want} missing from {all}");
    }
    let doc = env.ok("screenplay.document", json!({ "draftId": draft }));
    assert_eq!(doc["screenplay"]["titlePage"]["title"], "THE LAST SIGNAL");

    // Step 6: edit normally.
    let line = original[0]["elements"][3]["text"].clone();
    assert_eq!(line, "The last train is late again.");
    let el = s(&doc["scenes"][0]["elements"][3]["id"]);
    env.ok(
        "screenplay.apply_edits",
        json!({ "draftId": draft, "ops": [{ "op": "updateElement", "id": el, "text": "The last train is never on time." }] }),
    );

    // Step 7: export Final Draft. Step 8 (reopen in Final Draft) is a manual check; here the
    // file must be well-formed Final Draft XML with settings for every custom paragraph type.
    let out = env.dir.path().join("exports");
    std::fs::create_dir_all(&out).unwrap();
    let fdx = out.join("The Last Signal - edited.fdx");
    env.ok(
        "screenplay.export",
        json!({ "draftId": draft, "format": "fdx", "path": fdx.to_string_lossy(),
                "options": { "revisionMarks": true } }),
    );
    let text = std::fs::read_to_string(&fdx).unwrap();
    assert!(text.starts_with("<?xml"));
    for t in ["TELEPHONE", "Lower Third"] {
        assert!(text.contains(&format!("<Paragraph Type=\"{t}\">")), "{t}");
        assert!(
            text.contains(&format!("<ElementSettings Type=\"{t}\">")),
            "{t}"
        );
    }
    assert!(text.contains("Number=\"1A\""));
    assert!(text.contains("<Revisions"));

    // Step 9: re-import the exported file.
    let prev = env.ok(
        "screenplay.import_preview",
        json!({ "path": fdx.to_string_lossy() }),
    );
    let back = env.ok(
        "screenplay.import_apply",
        json!({ "previewId": prev["previewId"], "mode": "new_draft", "screenplayId": screenplay }),
    );
    let reimported = structure(&env, &s(&back["draftId"]));

    // Step 10: a clean PDF (no notes, no product name, title page, scene numbers).
    let pdf = out.join("The Last Signal.pdf");
    let r = env.ok(
        "screenplay.export",
        json!({ "draftId": draft, "format": "pdf", "path": pdf.to_string_lossy(),
                "options": { "sceneNumbers": true, "titlePage": true } }),
    );
    assert!(r["pages"].as_u64().unwrap() >= 1);
    let bytes = std::fs::read(&pdf).unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("OpenFrame"));

    // Step 11: the structure is the original's, with exactly the one edited line changed.
    let mut expected = original.clone();
    expected[0]["elements"][3]["text"] = json!("The last train is never on time.");
    assert_eq!(reimported, expected, "structure after the round trip");
}
