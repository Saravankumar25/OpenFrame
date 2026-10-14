//! Screenplay engine through the operation registry (the same path as the UI):
//! page setup and presets, title page, header and footer, explicit page
//! breaks, custom elements, production scene numbers, lock and revisions with
//! changed pages, page facts, and screenplay files without branding
//! (Screenplay and Production Depth Specification §8 to §17, §43; ADR-0015;
//! contract doc 32).

use std::path::PathBuf;

use openframe_application::MutationMeta;
use openframe_domain::{Capability, Role, new_id};
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

struct Sp {
    env: TestEnv,
    screenplay: String,
    draft: String,
}

fn setup() -> Sp {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let created = env.ok("screenplay.create", json!({ "title": "Black Rain" }));
    Sp {
        screenplay: created["screenplay"]["id"].as_str().unwrap().to_string(),
        draft: created["draft"]["id"].as_str().unwrap().to_string(),
        env,
    }
}

/// The kind of an error code ("validation.fontSize" is a "validation" error).
#[track_caller]
fn err_kind(env: &TestEnv, op: &str, args: Value) -> String {
    let code = env.err(op, args);
    code.split('.').next().unwrap_or_default().to_string()
}

fn s(v: &Value) -> String {
    v.as_str()
        .unwrap_or_else(|| panic!("expected string, got {v}"))
        .to_string()
}

impl Sp {
    fn doc(&self) -> Value {
        self.doc_of(&self.draft)
    }
    fn doc_of(&self, draft: &str) -> Value {
        self.env
            .ok("screenplay.document", json!({ "draftId": draft }))
    }
    fn edits_on(&self, draft: &str, ops: Value) -> Value {
        self.env.ok(
            "screenplay.apply_edits",
            json!({ "draftId": draft, "ops": ops }),
        )
    }
    fn edits(&self, ops: Value) -> Value {
        self.edits_on(&self.draft, ops)
    }
    /// Replace the blank first scene with `n` written scenes; returns their ids.
    fn write(&self, n: usize) -> Vec<String> {
        let blank = s(&self.doc()["scenes"][0]["id"]);
        let mut ops = Vec::new();
        let mut ids = Vec::new();
        for i in 0..n {
            let id = new_id();
            ids.push(id.clone());
            ops.push(json!({ "op": "insertScene", "id": id, "index": i, "heading": format!("INT. ROOM {i} - NIGHT") }));
            ops.push(
                json!({ "op": "insertElement", "sceneId": id, "elementType": "action",
                "text": "Rain hammers the window. ".repeat(3 + i % 5) }),
            );
            ops.push(json!({ "op": "insertElement", "sceneId": id, "elementType": "character", "text": "MEERA" }));
            ops.push(
                json!({ "op": "insertElement", "sceneId": id, "elementType": "dialogue",
                "text": "You said you would never come back. ".repeat(1 + i % 4) }),
            );
        }
        ops.push(json!({ "op": "deleteScene", "id": blank }));
        self.edits(Value::Array(ops));
        ids
    }
    fn labels(&self, draft: &str) -> Vec<String> {
        self.doc_of(draft)["scenes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| s(&x["label"]))
            .collect()
    }
    fn out(&self, name: &str) -> String {
        let dir: PathBuf = self.env.dir.path().join("exports");
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name).to_string_lossy().into_owned()
    }
    fn export(&self, draft: &str, format: &str, name: &str, options: Value) -> (Value, Vec<u8>) {
        let path = self.out(name);
        let r = self.env.ok(
            "screenplay.export",
            json!({ "draftId": draft, "format": format, "path": path, "options": options }),
        );
        let bytes = std::fs::read(s(&r["path"])).unwrap();
        (r, bytes)
    }
    fn count(&self, sql: &str) -> i64 {
        self.env
            .core
            .project()
            .unwrap()
            .store
            .read(|c| Ok(c.query_row(sql, [], |r| r.get::<_, i64>(0))?))
            .unwrap()
    }
}

// ------------------------------------------------------------------ page setup

#[test]
fn page_setup_is_saved_with_the_screenplay_undone_and_survives_restart() {
    let sp = setup();
    sp.write(30);
    let info = sp.env.ok(
        "screenplay.page_setup",
        json!({ "screenplayId": sp.screenplay }),
    );
    assert_eq!(info["setup"]["paper"], "Letter");
    assert_eq!(info["linesPerPage"], 55);
    assert!(info["presets"].as_array().unwrap().len() >= 2);
    let letter_pages = sp
        .env
        .ok("screenplay.pagination", json!({ "draftId": sp.draft }))["pageCount"]
        .as_u64()
        .unwrap();

    let mut setup = info["setup"].clone();
    setup["paper"] = json!("A4");
    setup["header"] = json!("{title} | {draft}");
    setup["firstPageNumber"] = json!(2);
    sp.env.ok(
        "screenplay.update_page_setup",
        json!({ "screenplayId": sp.screenplay, "setup": setup }),
    );
    let p = sp
        .env
        .ok("screenplay.pagination", json!({ "draftId": sp.draft }));
    assert_eq!(p["paper"], "A4");
    assert_eq!(p["linesPerPage"], 59);
    assert_eq!(p["firstPageNumber"], 2);
    assert!(p["pageCount"].as_u64().unwrap() <= letter_pages);

    // The PDF follows the page setup: A4 media box and the header on page 2 onwards.
    let (r, pdf) = sp.export(&sp.draft, "pdf", "a4.pdf", json!({ "titlePage": false }));
    assert_eq!(r["pages"], p["pageCount"]);
    let text = String::from_utf8_lossy(&pdf);
    assert!(
        text.contains("595.44") || text.contains("595.4"),
        "A4 width in points"
    );
    assert!(
        !text.contains("OpenFrame"),
        "screenplay PDFs carry no branding"
    );

    sp.env.undo();
    let info = sp.env.ok(
        "screenplay.page_setup",
        json!({ "screenplayId": sp.screenplay }),
    );
    assert_eq!(info["setup"]["paper"], "Letter");
    sp.env.redo();

    // Validation.
    let mut bad = setup.clone();
    bad["fontSize"] = json!(30);
    assert_eq!(
        err_kind(
            &sp.env,
            "screenplay.update_page_setup",
            json!({ "screenplayId": sp.screenplay, "setup": bad })
        ),
        "validation"
    );
    let mut bad = setup.clone();
    bad["font"] = json!("Arial");
    assert_eq!(
        err_kind(
            &sp.env,
            "screenplay.update_page_setup",
            json!({ "screenplayId": sp.screenplay, "setup": bad })
        ),
        "validation"
    );

    // Saved presets: save, name clash, delete to Recently Deleted, undo.
    sp.env.ok(
        "screenplay.save_page_preset",
        json!({ "name": "Festival A4", "setup": setup }),
    );
    assert_eq!(
        err_kind(
            &sp.env,
            "screenplay.save_page_preset",
            json!({ "name": "festival a4", "setup": setup })
        ),
        "validation"
    );
    let presets = sp.env.ok(
        "screenplay.page_setup",
        json!({ "screenplayId": sp.screenplay }),
    )["presets"]
        .clone();
    let preset = presets
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "Festival A4")
        .unwrap()
        .clone();
    assert_eq!(preset["setup"]["paper"], "A4");
    sp.env.ok(
        "screenplay.delete_page_preset",
        json!({ "id": preset["id"] }),
    );
    sp.env.undo();
    assert_eq!(
        sp.count("SELECT count(*) FROM screenplay_page_preset WHERE deleted_at IS NULL"),
        1
    );

    // A viewer can read but not change the page setup.
    let viewer = sp.env.actor_with_role(Role::Viewer);
    assert!(
        sp.env
            .call_as(
                &viewer,
                "screenplay.update_page_setup",
                json!({ "screenplayId": sp.screenplay, "setup": setup })
            )
            .is_err()
    );
    let screenplay = sp.screenplay.clone();
    let path = sp.env.project_path();
    let env = sp.env.restart();
    env.reopen_project(&path);
    let info = env.ok(
        "screenplay.page_setup",
        json!({ "screenplayId": screenplay }),
    );
    assert_eq!(info["setup"]["paper"], "A4");
    assert_eq!(info["setup"]["header"], "{title} | {draft}");
}

// ------------------------------------------------------------------ title page

#[test]
fn title_page_fields_preview_and_export_switch() {
    let sp = setup();
    sp.write(2);
    let tp = json!({
        "title": "Black Rain",
        "writtenBy": "Nisha Verma",
        "contactName": "Nisha Verma",
        "address": "Flat 4, Long Street, Banjara Hills, Hyderabad 500034",
        "phone": "+91 98765 43210",
        "email": "nisha@example.com",
        "draftLine": "Shooting Draft for the second unit after the location scout",
        "revisionLine": "Blue Revision",
        "date": "4 October 2026",
        "representation": "Represented by Long Name Talent Agency"
    });
    let preview = sp.env.ok(
        "screenplay.title_page_preview",
        json!({ "screenplayId": sp.screenplay, "titlePage": tp }),
    );
    let pages = preview["pages"].as_array().unwrap();
    assert_eq!(pages.len(), 1);
    let lines = pages[0]["lines"].as_array().unwrap();
    let all: String = lines
        .iter()
        .map(|l| s(&l["text"]))
        .collect::<Vec<_>>()
        .join(" ");
    for want in [
        "BLACK RAIN",
        "nisha@example.com",
        "+91 98765 43210",
        "4 October 2026",
        "location scout",
        "Agency",
    ] {
        assert!(all.contains(want), "{want} missing from {all}");
    }
    for l in lines {
        let end = l["xIn"].as_f64().unwrap() + s(&l["text"]).chars().count() as f64 * 0.1;
        assert!(end <= 7.5 + 1e-3, "{} runs past the margin", l["text"]);
    }
    // Saving keeps every field; the export switch decides whether it prints.
    sp.env.ok(
        "screenplay.update_title_page",
        json!({ "screenplayId": sp.screenplay, "titlePage": tp }),
    );
    let doc = sp.doc();
    assert_eq!(doc["screenplay"]["titlePage"]["phone"], "+91 98765 43210");
    let (with, _) = sp.export(&sp.draft, "pdf", "with.pdf", json!({ "titlePage": true }));
    let (without, _) = sp.export(
        &sp.draft,
        "pdf",
        "without.pdf",
        json!({ "titlePage": false }),
    );
    assert_eq!(
        with["pages"], without["pages"],
        "script pages exclude the title page"
    );
    let (_, fdx) = sp.export(&sp.draft, "fdx", "t.fdx", json!({ "titlePage": true }));
    let fdx = String::from_utf8(fdx).unwrap();
    assert!(fdx.contains("nisha@example.com") && fdx.contains("4 October 2026"));
    sp.env.undo();
    assert_eq!(sp.doc()["screenplay"]["titlePage"]["phone"], "");
}

// ------------------------------------------------------------------ page breaks

#[test]
fn explicit_page_break_insert_remove_undo_and_export() {
    let sp = setup();
    let scenes = sp.write(3);
    let before = sp
        .env
        .ok("screenplay.pagination", json!({ "draftId": sp.draft }))["pageCount"]
        .as_u64()
        .unwrap();
    let pb = new_id();
    sp.edits(json!([{ "op": "insertElement", "id": pb, "sceneId": scenes[0], "elementType": "page_break" }]));
    let doc = sp.doc();
    let last_type = |d: &Value| {
        d["scenes"][0]["elements"]
            .as_array()
            .unwrap()
            .last()
            .unwrap()["elementType"]
            .clone()
    };
    assert_eq!(last_type(&doc), "page_break");
    let p = sp
        .env
        .ok("screenplay.pagination", json!({ "draftId": sp.draft }));
    assert_eq!(p["pageCount"].as_u64().unwrap(), before + 1);
    assert_eq!(
        p["scenes"][1]["firstPage"], 2,
        "scene 2 starts on the new page"
    );
    // Text in a page break is refused.
    assert_eq!(
        err_kind(
            &sp.env,
            "screenplay.apply_edits",
            json!({ "draftId": sp.draft, "ops": [{ "op": "updateElement", "id": pb, "text": "x" }] })
        ),
        "validation"
    );
    let (_, fdx) = sp.export(&sp.draft, "fdx", "pb.fdx", json!({}));
    assert!(
        String::from_utf8(fdx)
            .unwrap()
            .contains("StartsNewPage=\"Yes\"")
    );
    let (_, fountain) = sp.export(&sp.draft, "fountain", "pb.fountain", json!({}));
    assert!(String::from_utf8(fountain).unwrap().contains("\n===\n"));
    // Re-import keeps the page break.
    let path = sp
        .env
        .write_file("pb.fdx", &std::fs::read(sp.out("pb.fdx")).unwrap());
    let prev = sp
        .env
        .ok("screenplay.import_preview", json!({ "path": path }));
    let rep = sp.env.ok(
        "screenplay.import_apply",
        json!({ "previewId": prev["previewId"], "mode": "new_draft", "screenplayId": sp.screenplay }),
    );
    let imported = sp.doc_of(&s(&rep["draftId"]));
    let breaks = imported["scenes"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|x| x["elements"].as_array().unwrap().clone())
        .filter(|e| e["elementType"] == "page_break")
        .count();
    assert_eq!(breaks, 1);
    // Remove and undo.
    sp.edits(json!([{ "op": "deleteElement", "id": pb }]));
    assert_ne!(last_type(&sp.doc()), "page_break");
    sp.env.undo();
    assert_eq!(last_type(&sp.doc()), "page_break");
}

// ------------------------------------------------------------- custom elements

#[test]
fn custom_elements_define_use_export_reimport_and_protect() {
    let sp = setup();
    let scenes = sp.write(2);
    let tel = sp.env.ok(
        "screenplay.create_custom_element",
        json!({ "screenplayId": sp.screenplay, "name": "Telephone", "shortcut": "Ctrl+Alt+1",
                "look": "dialogue", "uppercase": true, "italic": true, "nextElement": "action" }),
    );
    assert_eq!(tel["exportName"], "TELEPHONE");
    let tel_id = s(&tel["id"]);
    // Clashes: built-in name, same name, same shortcut, bad shortcut, bad look.
    for args in [
        json!({ "screenplayId": sp.screenplay, "name": "Dialogue" }),
        json!({ "screenplayId": sp.screenplay, "name": "telephone" }),
        json!({ "screenplayId": sp.screenplay, "name": "Radio", "shortcut": "Ctrl+Alt+1" }),
        json!({ "screenplayId": sp.screenplay, "name": "Radio", "shortcut": "Ctrl+2" }),
        json!({ "screenplayId": sp.screenplay, "name": "Radio", "look": "sideways" }),
        json!({ "screenplayId": sp.screenplay, "name": "Radio", "exportName": "Action" }),
    ] {
        assert_eq!(
            err_kind(&sp.env, "screenplay.create_custom_element", args),
            "validation"
        );
    }
    // A Unicode name is fine.
    sp.env.ok(
        "screenplay.create_custom_element",
        json!({ "screenplayId": sp.screenplay, "name": "ఫోన్ కాల్", "exportName": "PHONE CALL TE" }),
    );

    let el = new_id();
    sp.edits(
        json!([{ "op": "insertElement", "id": el, "sceneId": scenes[0], "elementType": "custom",
                      "customElementId": tel_id, "text": "hello? who is this?" }]),
    );
    // Custom without an id, or an id with another type, is refused.
    assert_eq!(
        err_kind(
            &sp.env,
            "screenplay.apply_edits",
            json!({ "draftId": sp.draft, "ops": [{ "op": "insertElement", "sceneId": scenes[0], "elementType": "custom", "text": "x" }] })
        ),
        "validation"
    );
    assert_eq!(
        err_kind(
            &sp.env,
            "screenplay.apply_edits",
            json!({ "draftId": sp.draft, "ops": [{ "op": "insertElement", "sceneId": scenes[0], "elementType": "action", "customElementId": tel_id, "text": "x" }] })
        ),
        "validation"
    );
    let doc = sp.doc();
    let found = doc["scenes"][0]["elements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == el.as_str())
        .unwrap()
        .clone();
    assert_eq!(found["customElementId"], tel_id.as_str());
    assert_eq!(doc["customElements"].as_array().unwrap().len(), 2);

    // The page view prints it with its look; FDX keeps its identity.
    let view = sp.env.ok(
        "screenplay.page_view",
        json!({ "draftId": sp.draft, "titlePage": false }),
    );
    let line = view["pages"][0]["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["text"] == "HELLO? WHO IS THIS?")
        .cloned()
        .expect("custom element printed in capitals");
    assert_eq!(line["style"], "custom_italic");
    assert!((line["xIn"].as_f64().unwrap() - 2.5).abs() < 1e-3);
    let (_, fdx) = sp.export(&sp.draft, "fdx", "custom.fdx", json!({}));
    let fdx_text = String::from_utf8(fdx.clone()).unwrap();
    assert!(fdx_text.contains("<Paragraph Type=\"TELEPHONE\">"));
    assert!(fdx_text.contains("<ElementSettings Type=\"TELEPHONE\">"));
    let (fountain, _) = sp.export(&sp.draft, "fountain", "custom.fountain", json!({}));
    assert!(
        fountain["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["code"] == "custom_as_action")
    );

    // Re-import into a new screenplay: the custom element comes back as a custom element.
    let env2 = TestEnv::with_project("Second", "Feature Film");
    let path = env2.write_file("custom.fdx", &fdx);
    let prev = env2.ok("screenplay.import_preview", json!({ "path": path }));
    let rep = env2.ok(
        "screenplay.import_apply",
        json!({ "previewId": prev["previewId"], "mode": "new_screenplay" }),
    );
    let doc2 = env2.ok("screenplay.document", json!({ "draftId": rep["draftId"] }));
    let defs = doc2["customElements"].as_array().unwrap();
    let def = defs
        .iter()
        .find(|d| d["exportName"] == "TELEPHONE")
        .unwrap();
    assert_eq!(def["look"], "dialogue");
    assert_eq!(def["uppercase"], true);
    let reimported = doc2["scenes"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|x| x["elements"].as_array().unwrap().clone())
        .find(|e| e["text"] == "hello? who is this?")
        .unwrap();
    assert_eq!(reimported["elementType"], "custom");
    assert_eq!(reimported["customElementId"], def["id"]);

    // Changing its look updates every use; deleting it while used is refused.
    sp.env.ok(
        "screenplay.update_custom_element",
        json!({ "id": tel_id, "look": "centered", "uppercase": false }),
    );
    let view = sp.env.ok(
        "screenplay.page_view",
        json!({ "draftId": sp.draft, "titlePage": false }),
    );
    assert!(
        view["pages"][0]["lines"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l["text"] == "hello? who is this?")
    );
    assert_eq!(
        err_kind(
            &sp.env,
            "screenplay.delete_custom_element",
            json!({ "id": tel_id })
        ),
        "conflict"
    );
    // Changing the element to action clears the reference, then delete works and undoes.
    sp.edits(json!([{ "op": "updateElement", "id": el, "elementType": "action" }]));
    let after = sp.doc();
    let e = after["scenes"][0]["elements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == el.as_str())
        .unwrap()
        .clone();
    assert!(e["customElementId"].is_null());
    // The draft still has the element in automatic history? Deleting checks live text only.
    sp.env
        .ok("screenplay.delete_custom_element", json!({ "id": tel_id }));
    assert_eq!(sp.doc()["customElements"].as_array().unwrap().len(), 1);
    sp.env.undo();
    assert_eq!(sp.doc()["customElements"].as_array().unwrap().len(), 2);
}

/// Security review 2026-10-07: a hostile FDX with 10,000 custom paragraph types whose names
/// are identical after the 40-character name limit must import quickly, keep at most the
/// per-screenplay cap as custom elements, send the rest to review and still suffix names.
#[test]
fn hostile_fdx_with_thousands_of_look_alike_custom_styles_is_bounded() {
    let env = TestEnv::with_project("Hostile", "Feature Film");
    let mut fdx = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<FinalDraft DocumentType=\"Script\" Template=\"No\" Version=\"5\">\n<Content>\n<Paragraph Type=\"Scene Heading\"><Text>INT. HALL - NIGHT</Text></Paragraph>\n",
    );
    let long = "T".repeat(45);
    for i in 0..10_000 {
        fdx.push_str(&format!(
            "<Paragraph Type=\"{long}{i}\"><Text>Line {i}</Text></Paragraph>\n"
        ));
    }
    fdx.push_str("</Content>\n");
    for i in 0..10_000 {
        fdx.push_str(&format!(
            "<ElementSettings Type=\"{long}{i}\"><ParagraphSpec Alignment=\"Center\"/></ElementSettings>\n"
        ));
    }
    fdx.push_str("</FinalDraft>\n");
    let started = std::time::Instant::now();
    let path = env.write_file("hostile.fdx", fdx.as_bytes());
    let prev = env.ok("screenplay.import_preview", json!({ "path": path }));
    assert!(
        prev["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["code"] == "custom_elements_over_limit")
    );
    let rep = env.ok(
        "screenplay.import_apply",
        json!({ "previewId": prev["previewId"], "mode": "new_screenplay" }),
    );
    assert!(
        started.elapsed() < std::time::Duration::from_secs(60),
        "took {:?}",
        started.elapsed()
    );
    let doc = env.ok("screenplay.document", json!({ "draftId": rep["draftId"] }));
    let defs = doc["customElements"].as_array().unwrap();
    assert_eq!(defs.len(), 64, "at most the per-screenplay cap");
    let names: std::collections::HashSet<String> = defs.iter().map(|d| s(&d["name"])).collect();
    assert_eq!(names.len(), 64, "names are unique");
    assert!(names.contains(&"T".repeat(40)));
    assert!(names.contains(&format!("{} (2)", "T".repeat(35))));
    // Every line is kept: 64 kinds as custom elements, the rest as action for review.
    let els: Vec<Value> = doc["scenes"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|x| x["elements"].as_array().unwrap().clone())
        .collect();
    assert_eq!(els.len(), 10_000);
    assert_eq!(
        els.iter().filter(|e| e["elementType"] == "custom").count(),
        64
    );
}

/// A genuine Final Draft file with more custom styles than a screenplay can hold is never
/// refused: the styles that fit become custom elements, every other paragraph is imported as
/// action with its text, and the report says how many paragraphs and which styles. This holds
/// for a new screenplay (64 of 70 styles fit) and for a new draft of a screenplay that already
/// has 60 custom elements (4 more fit).
#[test]
fn a_file_with_more_custom_styles_than_fit_imports_with_every_line_and_a_warning() {
    const STYLES: usize = 70;
    let mut fdx = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<FinalDraft DocumentType=\"Script\" Version=\"5\">
<Content>
<Paragraph Type=\"Scene Heading\"><Text>EXT. ROOF - DAY</Text></Paragraph>
",
    );
    for i in 0..STYLES {
        fdx.push_str(&format!(
            "<Paragraph Type=\"RADIO {i}\"><Text>Static {i}</Text></Paragraph>
"
        ));
    }
    fdx.push_str(
        "</Content>
</FinalDraft>
",
    );
    let texts = |doc: &Value| -> Vec<(String, String)> {
        doc["scenes"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|x| x["elements"].as_array().unwrap().clone())
            .map(|e| (s(&e["elementType"]), s(&e["text"])))
            .collect()
    };
    let warned = |rep: &Value, code: &str| {
        rep["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["code"] == code)
    };

    // 1. A new screenplay: 64 custom elements, the other 6 styles as action.
    let env = TestEnv::with_project("Roof", "Feature Film");
    let path = env.write_file("radio.fdx", fdx.as_bytes());
    let prev = env.ok("screenplay.import_preview", json!({ "path": path }));
    let rep = env.ok(
        "screenplay.import_apply",
        json!({ "previewId": prev["previewId"], "mode": "new_screenplay" }),
    );
    assert!(warned(&rep, "custom_elements_over_limit"), "{rep}");
    let doc = env.ok("screenplay.document", json!({ "draftId": rep["draftId"] }));
    assert_eq!(doc["customElements"].as_array().unwrap().len(), 64);
    let lines = texts(&doc);
    assert_eq!(lines.len(), STYLES, "every paragraph is kept");
    for i in 0..STYLES {
        assert!(lines.iter().any(|(_, t)| *t == format!("Static {i}")));
    }
    assert_eq!(lines.iter().filter(|(k, _)| k == "custom").count(), 64);
    assert_eq!(
        lines.iter().filter(|(k, _)| k == "action").count(),
        STYLES - 64
    );

    // 2. A new draft of a screenplay that already has 60 custom elements: 4 more fit.
    let sp = setup();
    sp.write(1);
    for i in 0..60 {
        sp.env.ok(
            "screenplay.create_custom_element",
            json!({ "screenplayId": sp.screenplay, "name": format!("Style {i}") }),
        );
    }
    let path = sp.env.write_file("radio.fdx", fdx.as_bytes());
    let prev = sp
        .env
        .ok("screenplay.import_preview", json!({ "path": path }));
    let rep = sp.env.ok(
        "screenplay.import_apply",
        json!({ "previewId": prev["previewId"], "mode": "new_draft", "screenplayId": sp.screenplay }),
    );
    assert!(warned(&rep, "custom_elements_no_room"), "{rep}");
    let note = rep["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["code"] == "custom_elements_no_room")
        .unwrap()["message"]
        .clone();
    assert!(s(&note).contains("60 paragraph(s)"), "{note}");
    assert!(
        s(&note).contains("“RADIO 4”") && s(&note).contains("and 40 more"),
        "{note}"
    );
    assert!(rep["warningsNeedingAttention"].as_u64().unwrap() >= 2);
    assert_eq!(
        sp.count("SELECT count(*) FROM screenplay_custom_element WHERE deleted_at IS NULL"),
        64
    );
    let doc = sp.doc_of(&s(&rep["draftId"]));
    let lines = texts(&doc);
    assert_eq!(lines.len(), STYLES);
    assert_eq!(lines.iter().filter(|(k, _)| k == "custom").count(), 4);
    for i in 0..STYLES {
        assert!(lines.iter().any(|(_, t)| *t == format!("Static {i}")));
    }
    // The import is one undo step.
    sp.env.undo();
    assert_eq!(
        sp.count("SELECT count(*) FROM screenplay_custom_element WHERE deleted_at IS NULL"),
        60
    );
}

// ------------------------------------------------------- production numbers

#[test]
fn production_numbers_freeze_at_lock_insert_letters_and_omit() {
    let sp = setup();
    let scenes = sp.write(4);
    assert_eq!(sp.labels(&sp.draft), ["1", "2", "3", "4"]);
    // Reordering an unlocked draft renumbers.
    sp.edits(json!([{ "op": "moveScene", "id": scenes[3], "index": 0 }]));
    assert_eq!(sp.doc()["scenes"][0]["label"], "1");
    assert_eq!(sp.doc()["scenes"][0]["id"], scenes[3].as_str());
    sp.edits(json!([{ "op": "moveScene", "id": scenes[3], "index": 3 }]));

    sp.env
        .ok("screenplay.lock_draft", json!({ "draftId": sp.draft }));
    assert_eq!(
        sp.count("SELECT count(*) FROM screenplay_scene WHERE production_number_source='locked' AND deleted_at IS NULL"),
        4
    );
    assert_eq!(sp.count("SELECT count(*) FROM screenplay_lock_baseline"), 1);
    // A locked draft refuses a manual number.
    assert_eq!(
        err_kind(
            &sp.env,
            "screenplay.set_scene_number",
            json!({ "sceneId": scenes[0], "number": "9" })
        ),
        "permission"
    );
    let rev = sp.env.ok(
        "screenplay.start_revision",
        json!({ "draftId": sp.draft, "label": "Blue Revision", "color": "Blue", "reason": "Location change", "date": "2026-10-04" }),
    );
    let rd = s(&rev["id"]);
    assert_eq!(sp.labels(&rd), ["1", "2", "3", "4"]);
    let rscenes: Vec<String> = sp.doc_of(&rd)["scenes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| s(&x["id"]))
        .collect();
    // A scene inserted after 2 becomes 2A; reordering never changes frozen labels.
    let new_scene = new_id();
    sp.edits_on(
        &rd,
        json!([{ "op": "insertScene", "id": new_scene, "index": 2, "heading": "EXT. CROSSING - DAWN" },
               { "op": "insertElement", "sceneId": new_scene, "elementType": "action", "text": "A train passes." }]),
    );
    assert_eq!(sp.labels(&rd), ["1", "2", "2A", "3", "4"]);
    sp.edits_on(
        &rd,
        json!([{ "op": "moveScene", "id": rscenes[0], "index": 4 }]),
    );
    assert_eq!(sp.labels(&rd), ["2", "2A", "3", "4", "1"]);
    sp.env.undo();
    assert_eq!(sp.labels(&rd), ["1", "2", "2A", "3", "4"]);

    // Deleting a numbered scene of a revision keeps its number as OMITTED.
    sp.edits_on(&rd, json!([{ "op": "deleteScene", "id": rscenes[2] }]));
    let d = sp.doc_of(&rd);
    assert_eq!(sp.labels(&rd), ["1", "2", "2A", "4"]);
    assert_eq!(d["omittedScenes"][0]["label"], "3");
    let (_, pdf) = sp.export(&rd, "fdx", "omit.fdx", json!({ "sceneNumbers": true }));
    let fdx = String::from_utf8(pdf).unwrap();
    assert!(fdx.contains("Number=\"3\"") && fdx.contains("OMITTED"));
    // The editor's indexes skip the omitted scene: index 3 is still before scene 4.
    let late = new_id();
    sp.edits_on(
        &rd,
        json!([{ "op": "insertScene", "id": late, "index": 3, "heading": "INT. LATE - NIGHT" }]),
    );
    let doc = sp.doc_of(&rd);
    assert_eq!(doc["scenes"][3]["id"], late.as_str());
    assert_eq!(doc["scenes"][4]["label"], "4");
    sp.env.undo();
    sp.env.ok(
        "screenplay.restore_omitted_scene",
        json!({ "sceneId": rscenes[2] }),
    );
    assert_eq!(sp.labels(&rd), ["1", "2", "2A", "3", "4"]);

    // Manual numbers: valid, unique, alphanumeric, undoable.
    sp.env.ok(
        "screenplay.set_scene_number",
        json!({ "sceneId": new_scene, "number": "2b" }),
    );
    assert_eq!(sp.labels(&rd)[2], "2B");
    assert_eq!(
        err_kind(
            &sp.env,
            "screenplay.set_scene_number",
            json!({ "sceneId": new_scene, "number": "3" })
        ),
        "validation"
    );
    assert_eq!(
        err_kind(
            &sp.env,
            "screenplay.set_scene_number",
            json!({ "sceneId": new_scene, "number": "2 B" })
        ),
        "validation"
    );
    sp.env.undo();
    assert_eq!(sp.labels(&rd)[2], "2A");
    // Renumbering a revision is refused (frozen for production paperwork).
    assert_eq!(
        err_kind(
            &sp.env,
            "screenplay.renumber_scenes",
            json!({ "draftId": rd })
        ),
        "validation"
    );
}

#[test]
fn changing_a_number_used_in_production_needs_acknowledgement() {
    let sp = setup();
    let scenes = sp.write(2);
    let lineage: String = sp.doc()["scenes"][0]["lineageId"]
        .as_str()
        .unwrap()
        .to_string();
    let store = &sp.env.core.project().unwrap().store;
    store
        .mutate(
            &sp.env.actor(),
            MutationMeta::new("test.seed", "Seed a shot", Capability::Edit),
            |tx| {
                tx.conn().execute(
                    "INSERT INTO shot(id, scene_id, scene_lineage_id, position, description, created_at, updated_at)
                     VALUES (?1, ?2, ?3, 1, 'Wide on the window', 1, 1)",
                    rusqlite::params![new_id(), scenes[0], lineage],
                )?;
                Ok(())
            },
        )
        .unwrap();
    let err = sp
        .env
        .call(
            "screenplay.set_scene_number",
            json!({ "sceneId": scenes[0], "number": "1A" }),
        )
        .unwrap_err();
    // The Scene number dialog branches on this exact code.
    assert_eq!(err.code.0, "validation.dependencies");
    assert!(
        err.message.contains("production planning"),
        "{}",
        err.message
    );
    sp.env.ok(
        "screenplay.set_scene_number",
        json!({ "sceneId": scenes[0], "number": "1A", "acknowledgeDependencies": true }),
    );
    // The shot still points at the same scene: numbers are labels, never identity.
    assert_eq!(
        sp.count(&format!(
            "SELECT count(*) FROM shot WHERE scene_id='{}'",
            scenes[0]
        )),
        1
    );
    assert_eq!(sp.labels(&sp.draft), ["1A", "1B"]);
    // Renumbering an unlocked draft returns to scene order.
    sp.env
        .ok("screenplay.renumber_scenes", json!({ "draftId": sp.draft }));
    assert_eq!(sp.labels(&sp.draft), ["1", "2"]);
}

#[test]
fn imported_scene_numbers_are_kept_as_labels() {
    let env = TestEnv::with_project("Imported", "Feature Film");
    let fdx = r#"<?xml version="1.0" encoding="UTF-8"?>
<FinalDraft DocumentType="Script" Template="No" Version="5">
<Content>
<Paragraph Number="12" Type="Scene Heading"><Text>INT. HOUSE - DAY</Text></Paragraph>
<Paragraph Type="Action"><Text>Rain.</Text></Paragraph>
<Paragraph Number="12A" Type="Scene Heading"><Text>EXT. ROAD - DAY</Text></Paragraph>
<Paragraph Type="Action"><Text>Mud.</Text></Paragraph>
<Paragraph Type="Scene Heading"><Text>INT. CAR - DAY</Text></Paragraph>
<Paragraph Type="Action"><Text>Silence.</Text></Paragraph>
</Content>
</FinalDraft>"#;
    let path = env.write_file("numbers.fdx", fdx.as_bytes());
    let prev = env.ok("screenplay.import_preview", json!({ "path": path }));
    let rep = env.ok(
        "screenplay.import_apply",
        json!({ "previewId": prev["previewId"], "mode": "new_screenplay" }),
    );
    let doc = env.ok("screenplay.document", json!({ "draftId": rep["draftId"] }));
    let labels: Vec<String> = doc["scenes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| s(&x["label"]))
        .collect();
    assert_eq!(labels, ["12", "12A", "12B"]);
    assert_eq!(doc["scenes"][0]["numberSource"], "imported");
    assert_eq!(doc["numberingFrozen"], true);
}

// ------------------------------------------------------- lock and revisions

#[test]
fn revisions_mark_changes_and_list_changed_pages_and_scenes() {
    let sp = setup();
    let scenes = sp.write(30);
    sp.env
        .ok("screenplay.lock_draft", json!({ "draftId": sp.draft }));
    let rev = sp.env.ok(
        "screenplay.start_revision",
        json!({ "draftId": sp.draft, "label": "Revision A", "color": "Pink", "reason": "Notes", "date": "2026-10-04" }),
    );
    let rd = s(&rev["id"]);
    let untouched = sp
        .env
        .ok("screenplay.revision_pages", json!({ "draftId": rd }));
    assert_eq!(untouched["baselineDraftId"], sp.draft.as_str());
    assert!(untouched["changedPages"].as_array().unwrap().is_empty());
    assert!(untouched["scenes"].as_array().unwrap().is_empty());
    assert_eq!(untouched["revisionDate"], "2026-10-04");

    // Edit one line near the end: its element carries the revision mark.
    let doc = sp.doc_of(&rd);
    let last = doc["scenes"][29]["elements"][0].clone();
    sp.edits_on(
        &rd,
        json!([{ "op": "updateElement", "id": last["id"], "text": "The storm is over." }]),
    );
    let doc = sp.doc_of(&rd);
    assert_eq!(
        doc["scenes"][29]["elements"][0]["revisionMark"],
        "Revision A"
    );
    assert!(doc["scenes"][0]["elements"][0]["revisionMark"].is_null());
    // Remove a scene from the revision (numbered: it becomes OMITTED).
    let gone = s(&doc["scenes"][20]["id"]);
    sp.edits_on(&rd, json!([{ "op": "deleteScene", "id": gone }]));
    let r = sp
        .env
        .ok("screenplay.revision_pages", json!({ "draftId": rd }));
    let changed: Vec<u64> = r["changedPages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap())
        .collect();
    assert!(!changed.is_empty());
    assert!(!changed.contains(&1), "page 1 is unchanged: {changed:?}");
    let kinds: Vec<(String, String)> = r["scenes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| (s(&x["label"]), s(&x["change"])))
        .collect();
    assert!(
        kinds.contains(&("30".into(), "changed".into())),
        "{kinds:?}"
    );
    assert!(
        kinds.contains(&("21".into(), "omitted".into())),
        "{kinds:?}"
    );
    // Revision output prints asterisks; clean output does not.
    let marked = sp.env.ok(
        "screenplay.page_view",
        json!({ "draftId": rd, "revisionMarks": true }),
    );
    let has_mark = |v: &Value| {
        v["pages"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|p| p["lines"].as_array().unwrap().clone())
            .any(|l| l["style"] == "revision_mark")
    };
    assert!(has_mark(&marked));
    let clean = sp.env.ok(
        "screenplay.page_view",
        json!({ "draftId": rd, "revisionMarks": false }),
    );
    assert!(!has_mark(&clean));
    // Date change is undoable; an invalid date is refused.
    sp.env.ok(
        "screenplay.update_revision",
        json!({ "draftId": rd, "date": "2026-10-09" }),
    );
    assert_eq!(
        sp.env
            .ok("screenplay.revision_pages", json!({ "draftId": rd }))["revisionDate"],
        "2026-10-09"
    );
    sp.env.undo();
    assert_eq!(
        err_kind(
            &sp.env,
            "screenplay.update_revision",
            json!({ "draftId": rd, "date": "9 Oct" })
        ),
        "validation"
    );
    // Unlocking the shooting draft drops its page baseline; locking again records it.
    sp.env
        .ok("screenplay.unlock_draft", json!({ "draftId": sp.draft }));
    assert_eq!(sp.count("SELECT count(*) FROM screenplay_lock_baseline"), 0);
    sp.env.undo();
    assert_eq!(sp.count("SELECT count(*) FROM screenplay_lock_baseline"), 1);
    let _ = scenes;
}

// ------------------------------------------------------- one page layout

#[test]
fn page_view_print_preview_pagination_and_pdf_agree() {
    let sp = setup();
    sp.write(40);
    let mut setup = sp.env.ok(
        "screenplay.page_setup",
        json!({ "screenplayId": sp.screenplay }),
    )["setup"]
        .clone();
    setup["paper"] = json!("A4");
    setup["footer"] = json!("{title} page {page}");
    sp.env.ok(
        "screenplay.update_page_setup",
        json!({ "screenplayId": sp.screenplay, "setup": setup }),
    );
    let options = json!({ "titlePage": true, "sceneNumbers": false });
    let view = sp.env.ok(
        "screenplay.page_view",
        json!({ "draftId": sp.draft, "titlePage": true }),
    );
    let preview = sp.env.ok(
        "interchange.export_preview",
        json!({ "draftId": sp.draft, "options": options }),
    );
    assert_eq!(
        view["pages"], preview["pages"],
        "page view equals print preview"
    );
    assert_eq!(view["paper"], "A4");
    let p = sp
        .env
        .ok("screenplay.pagination", json!({ "draftId": sp.draft }));
    assert_eq!(p["pageCount"], preview["scriptPageCount"]);
    let (r, pdf) = sp.export(&sp.draft, "pdf", "agree.pdf", options);
    assert_eq!(r["pages"], preview["scriptPageCount"]);
    // The PDF holds the title page plus every script page of the preview.
    let pdf_doc = lopdf::Document::load_mem(&pdf).unwrap();
    assert_eq!(
        pdf_doc.get_pages().len(),
        preview["pages"].as_array().unwrap().len()
    );
    // Every scene's pages are inside the document and in order.
    let mut last = 0;
    for sc in p["scenes"].as_array().unwrap() {
        let first = sc["firstPage"].as_u64().unwrap();
        assert!(first >= last);
        assert!(sc["lastPage"].as_u64().unwrap() <= p["pageCount"].as_u64().unwrap());
        last = first;
    }
}

#[test]
fn ai_reads_page_facts_but_never_changes_layout_or_numbers() {
    use openframe_application::registry::{self, AiExposure};
    let reg = registry::catalog();
    for op in [
        "screenplay.update_page_setup",
        "screenplay.save_page_preset",
        "screenplay.delete_page_preset",
        "screenplay.create_custom_element",
        "screenplay.update_custom_element",
        "screenplay.delete_custom_element",
        "screenplay.set_scene_number",
        "screenplay.renumber_scenes",
        "screenplay.restore_omitted_scene",
    ] {
        let m = reg
            .metadata(op)
            .unwrap_or_else(|| panic!("{op} registered"));
        assert!(
            matches!(m.ai_exposure, AiExposure::Hidden(_)),
            "{op} is human-only"
        );
    }
    for op in [
        "screenplay.pagination",
        "screenplay.page_setup",
        "screenplay.custom_elements",
        "screenplay.revision_pages",
    ] {
        assert_eq!(
            reg.metadata(op).unwrap().ai_exposure,
            AiExposure::Tool,
            "{op}"
        );
    }
}

/// Performance budget (doc 21 P4 query budget, 150 ms in a release build): page facts and the
/// page view of a feature-length script are recomputed after saves, so they must stay fast.
/// Debug builds get ten times the budget.
#[test]
fn page_facts_of_a_feature_length_script_stay_inside_the_query_budget() {
    let sp = setup();
    sp.write(560);
    let budget = std::time::Duration::from_millis(if cfg!(debug_assertions) { 1_500 } else { 150 });
    // Warm the statement cache once, then measure.
    sp.env
        .ok("screenplay.pagination", json!({ "draftId": sp.draft }));
    let t = std::time::Instant::now();
    let p = sp
        .env
        .ok("screenplay.pagination", json!({ "draftId": sp.draft }));
    let pagination = t.elapsed();
    let t = std::time::Instant::now();
    let view = sp.env.ok(
        "screenplay.page_view",
        json!({ "draftId": sp.draft, "titlePage": false }),
    );
    let page_view = t.elapsed();
    let pages = p["pageCount"].as_u64().unwrap();
    assert!(pages >= 100, "{pages} pages");
    assert_eq!(view["pages"].as_array().unwrap().len() as u64, pages);
    assert!(
        pagination < budget,
        "pagination of {pages} pages took {pagination:?}"
    );
    assert!(
        page_view < budget * 2,
        "page view of {pages} pages took {page_view:?}"
    );
}
