//! Story page (Story > Story tab): one plain writing page, saved in the project and exported as
//! plain text, PDF or Word. Driven through the public operation registry like the UI.

use std::io::Read as _;

use openframe_domain::Role;
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

const BOM: &str = "\u{feff}";
const SMILE: &str = "\u{1F600}";

fn page(env: &TestEnv) -> Value {
    env.ok("story.page.get", json!({}))
}

fn save(env: &TestEnv, text: &str, rev: i64) -> Value {
    env.ok(
        "story.page.save",
        json!({ "text": text, "expectedRev": rev }),
    )
}

fn dest(env: &TestEnv, name: &str) -> String {
    let d = env.dir.path().join("exports");
    std::fs::create_dir_all(&d).unwrap();
    d.join(name).to_string_lossy().into_owned()
}

fn export(env: &TestEnv, format: &str, name: &str) -> (Value, String) {
    let path = dest(env, name);
    let r = env.ok(
        "story.page.export",
        json!({ "format": format, "path": path }),
    );
    (r, path)
}

fn docx_part(path: &str, name: &str) -> String {
    let f = std::fs::File::open(path).unwrap();
    let mut z = zip::ZipArchive::new(f).expect("a Word file is a zip package");
    let mut s = String::new();
    z.by_name(name)
        .unwrap_or_else(|_| panic!("{name} in the Word file"))
        .read_to_string(&mut s)
        .unwrap();
    s
}

#[test]
fn story_page_starts_empty_and_round_trips() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let p = page(&env);
    assert_eq!(p["text"], "");
    assert_eq!(p["rev"], 0);

    let first = save(&env, "Once upon a night bus.\n\nIt did not stop.", 0);
    assert_eq!(first["rev"], 1);
    let p = page(&env);
    assert_eq!(p["text"], "Once upon a night bus.\n\nIt did not stop.");
    assert_eq!(p["rev"], 1);
    assert_eq!(p["updatedAt"], first["updatedAt"]);

    // The same text again changes nothing; new text moves the revision on. Windows line endings
    // are stored as plain line feeds.
    assert_eq!(
        save(&env, "Once upon a night bus.\r\n\r\nIt did not stop.", 1)["rev"],
        1
    );
    assert_eq!(save(&env, "Second version.\r\nTwo lines.", 1)["rev"], 2);
    assert_eq!(page(&env)["text"], "Second version.\nTwo lines.");
}

#[test]
fn story_page_refuses_a_save_based_on_old_text() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    save(&env, "one", 0);
    save(&env, "two", 1);
    let code = env.err(
        "story.page.save",
        json!({ "text": "stale", "expectedRev": 1 }),
    );
    assert_eq!(code, "conflict.stale");
    // Nothing was overwritten, and a save based on "nothing saved yet" is stale too.
    assert_eq!(page(&env)["text"], "two");
    assert_eq!(
        env.err("story.page.save", json!({ "text": "x", "expectedRev": 0 })),
        "conflict.stale"
    );
}

#[test]
fn story_page_has_a_size_limit_with_a_plain_message() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let limit = 5 * 1024 * 1024;
    save(&env, &"a".repeat(limit), 0);
    let too_long = "a".repeat(limit + 1);
    let e = env
        .call(
            "story.page.save",
            json!({ "text": too_long, "expectedRev": 1 }),
        )
        .unwrap_err();
    assert!(e.code.0.starts_with("validation"), "{}", e.code.0);
    assert!(e.message.contains("5 MB"), "{}", e.message);
    assert_eq!(page(&env)["text"].as_str().unwrap().len(), limit);
}

#[test]
fn story_page_survives_restart_and_a_crash() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let text = "नमस्ते कहानी\nతెలుగు கதை മലയാളം\nÉté “quotes” and ñ";
    save(&env, text, 0);
    let path = env.project_path();
    let env = env.restart();
    env.reopen_project(&path);
    assert_eq!(page(&env)["text"], text);
    save(&env, "after restart", 1);
    let env = env.crash_and_restart();
    env.reopen_project(&path);
    let p = page(&env);
    assert_eq!(p["text"], "after restart");
    assert_eq!(p["rev"], 2);
}

#[test]
fn story_page_saves_are_not_undo_steps_and_not_activity() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    env.ok("story.create_act", json!({ "title": "Act One" }));
    save(&env, "Private thoughts", 0);
    // Undo reverts the act, never the page text.
    env.undo();
    assert_eq!(page(&env)["text"], "Private thoughts");
    let rows: i64 = env
        .core
        .project()
        .unwrap()
        .store
        .read(|c| {
            Ok(c.query_row(
                "SELECT count(*) FROM sys_activity WHERE action = 'story.page.save'",
                [],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    assert_eq!(rows, 0, "typing is not an activity entry");
}

#[test]
fn story_page_needs_edit_to_save_and_view_to_read() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    save(&env, "visible", 0);
    for role in [Role::Viewer, Role::Commenter, Role::ExportOnly] {
        let who = env.actor_with_role(role);
        let e = env
            .call_as(
                &who,
                "story.page.save",
                json!({ "text": "no", "expectedRev": 1 }),
            )
            .unwrap_err();
        assert_eq!(e.code.0, "permission.denied", "{role:?}");
        let read = env.call_as(&who, "story.page.get", json!({})).unwrap();
        assert_eq!(read["text"], "visible");
    }
    assert_eq!(page(&env)["rev"], 1);
}

#[test]
fn story_page_is_not_offered_to_the_assistant() {
    let r = openframe_application::registry::catalog();
    for op in ["story.page.get", "story.page.save", "story.page.export"] {
        let m = r.metadata(op).unwrap_or_else(|| panic!("{op} registered"));
        assert!(m.ai_exposure.is_hidden(), "{op} must stay hidden");
    }
}

// ------------------------------------------------------------------ export

const SAMPLE: &str =
    "Chapter one\nstarts here & ends <there>.\n\nSecond paragraph.\n\n\nAfter a gap.";

#[test]
fn story_page_exports_plain_text_exactly() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    save(&env, SAMPLE, 0);
    let (r, path) = export(&env, "txt", "story.txt");
    assert_eq!(r["format"], "txt");
    assert_eq!(r["document"], "Story");
    let bytes = std::fs::read(&path).unwrap();
    let want = format!("{BOM}{}", SAMPLE.replace('\n', "\r\n"));
    assert_eq!(
        bytes,
        want.as_bytes(),
        "no brand line, Windows line endings"
    );
    assert_eq!(r["bytesWritten"].as_u64().unwrap(), bytes.len() as u64);
}

#[test]
fn story_page_keeps_indian_scripts_in_text_word_and_pdf() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let text = "नमस्ते दुनिया\n\nతెలుగు కథ\n\nதமிழ் கதை\n\nമലയാളം കഥ";
    save(&env, text, 0);
    let (_, txt) = export(&env, "txt", "indic.txt");
    let got = String::from_utf8(std::fs::read(&txt).unwrap()).unwrap();
    assert_eq!(got.trim_start_matches(BOM), text.replace('\n', "\r\n"));
    let (_, docx) = export(&env, "docx", "indic.docx");
    let xml = docx_part(&docx, "word/document.xml");
    for word in ["नमस्ते दुनिया", "తెలుగు కథ", "தமிழ் கதை", "മലയാളം കഥ"]
    {
        assert!(xml.contains(word), "{word} missing from the Word file");
    }
    // The PDF fonts cover these scripts too.
    let (_, pdf) = export(&env, "pdf", "indic.pdf");
    assert!(std::fs::read(&pdf).unwrap().starts_with(b"%PDF"));
}

#[test]
fn story_page_exports_a_pdf() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    save(&env, SAMPLE, 0);
    let (r, path) = export(&env, "pdf", "story.pdf");
    assert_eq!(r["format"], "pdf");
    assert!(r["pages"].as_u64().unwrap() >= 1);
    let bytes = std::fs::read(&path).unwrap();
    assert!(bytes.starts_with(b"%PDF"));
    let doc = lopdf::Document::load(&path).expect("PDF parses");
    assert!(!doc.get_pages().is_empty());
}

#[test]
fn story_page_pdf_refuses_characters_no_font_covers_but_text_and_word_keep_them() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    save(&env, &format!("Fine text and a smile {SMILE}"), 0);
    let path = dest(&env, "bad.pdf");
    let e = env.err(
        "story.page.export",
        json!({ "format": "pdf", "path": path }),
    );
    assert_eq!(e, "export.pdf_unsupported_characters");
    assert!(!std::path::Path::new(&path).exists(), "no partial PDF");
    let (_, txt) = export(&env, "txt", "ok.txt");
    assert!(std::fs::read_to_string(txt).unwrap().contains(SMILE));
    let (_, docx) = export(&env, "docx", "ok.docx");
    assert!(docx_part(&docx, "word/document.xml").contains(SMILE));
}

#[test]
fn story_page_exports_word_with_escaped_text_and_the_required_parts() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    save(&env, SAMPLE, 0);
    let (r, path) = export(&env, "docx", "story.docx");
    assert_eq!(r["format"], "docx");
    let f = std::fs::File::open(&path).unwrap();
    let mut z = zip::ZipArchive::new(f).unwrap();
    let names: Vec<String> = (0..z.len())
        .map(|i| z.by_index(i).unwrap().name().unwrap().into_owned())
        .collect();
    for part in [
        "[Content_Types].xml",
        "_rels/.rels",
        "word/document.xml",
        "word/styles.xml",
    ] {
        assert!(names.iter().any(|n| n == part), "{part} missing: {names:?}");
    }
    let xml = docx_part(&path, "word/document.xml");
    assert!(
        xml.contains("starts here &amp; ends &lt;there&gt;."),
        "{xml}"
    );
    // One Word paragraph per line of the page.
    assert!(xml.contains(">Chapter one<") && xml.contains(">starts here &amp;"));
    assert!(xml.contains("Second paragraph.") && xml.contains("After a gap."));
    assert!(docx_part(&path, "[Content_Types].xml").contains("/word/document.xml"));
}

#[test]
fn story_page_exports_an_empty_page_as_valid_documents() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let (_, txt) = export(&env, "txt", "empty.txt");
    assert_eq!(std::fs::read(txt).unwrap(), BOM.as_bytes());
    let (_, docx) = export(&env, "docx", "empty.docx");
    assert!(docx_part(&docx, "word/document.xml").contains("Story"));
    let (_, pdf) = export(&env, "pdf", "empty.pdf");
    assert!(std::fs::read(&pdf).unwrap().starts_with(b"%PDF"));
    lopdf::Document::load(&pdf).expect("an empty page still gives a readable PDF");
}

#[test]
fn story_page_export_checks_format_destination_and_permission() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    save(&env, "text", 0);
    let csv = dest(&env, "x.csv");
    assert_eq!(
        env.err("story.page.export", json!({ "format": "csv", "path": csv })),
        "validation.invalid_input"
    );
    assert_eq!(
        env.err(
            "story.page.export",
            json!({ "format": "txt", "path": "relative.txt" })
        ),
        "validation.invalid_input"
    );
    for role in [Role::Viewer, Role::Commenter] {
        let who = env.actor_with_role(role);
        let e = env
            .call_as(
                &who,
                "story.page.export",
                json!({ "format": "txt", "path": dest(&env, "denied.txt") }),
            )
            .unwrap_err();
        assert_eq!(e.code.0, "permission.denied", "{role:?}");
    }
    // The export is a snapshot: the page is unchanged.
    export(&env, "txt", "snap.txt");
    assert_eq!(page(&env)["rev"], 1);
}

// ------------------------------------------------------------------ formatting

const FORMATTED: &str = "Hero walks away\n\nThe end";

fn formatting() -> String {
    json!([
        [
            { "t": "Hero", "b": true, "s": 20 },
            { "t": " walks ", "i": true },
            { "t": "away", "u": true, "h": true }
        ],
        [],
        [{ "t": "The end" }]
    ])
    .to_string()
}

fn save_formatted(env: &TestEnv, text: &str, format: &str, rev: i64) -> Value {
    env.ok(
        "story.page.save",
        json!({ "text": text, "format": format, "expectedRev": rev }),
    )
}

#[test]
fn story_page_keeps_its_formatting_and_plain_text_side_by_side() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    save_formatted(&env, FORMATTED, &formatting(), 0);
    let p = page(&env);
    assert_eq!(p["text"], FORMATTED, "the plain text is unchanged");
    let stored: Value = serde_json::from_str(p["format"].as_str().unwrap()).unwrap();
    assert_eq!(stored[0][0]["b"], true);
    assert_eq!(stored[0][0]["s"], 20.0);
    assert_eq!(stored[0][2]["h"], true);
    // A save without formatting clears it; unformatted runs are stored as plain text.
    save(&env, FORMATTED, 1);
    assert!(page(&env)["format"].is_null());
    let plain = json!([[{ "t": "Hero walks away" }], [], [{ "t": "The end" }]]).to_string();
    save_formatted(&env, FORMATTED, &formatting(), 2);
    assert!(!page(&env)["format"].is_null());
    save_formatted(&env, FORMATTED, &plain, 3);
    assert!(page(&env)["format"].is_null());
    assert_eq!(page(&env)["rev"], 4);
}

#[test]
fn story_page_refuses_formatting_that_does_not_match_its_text() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    save(&env, "kept", 0);
    for bad in [
        // Different words.
        json!([[{ "t": "Other text", "b": true }]]).to_string(),
        // Wrong number of lines.
        json!([[{ "t": "Hero walks away", "b": true }]]).to_string(),
        // A size out of range.
        json!([[{ "t": "Hero walks away", "s": 500 }], [], [{ "t": "The end" }]]).to_string(),
        // Not formatting at all.
        "not json".to_string(),
    ] {
        let e = env
            .call(
                "story.page.save",
                json!({ "text": FORMATTED, "format": bad, "expectedRev": 1 }),
            )
            .unwrap_err();
        assert!(e.code.0.starts_with("validation"), "{}", e.code.0);
    }
    assert_eq!(page(&env)["text"], "kept", "nothing was overwritten");
}

#[test]
fn story_page_exports_its_formatting_to_word_and_pdf() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    save_formatted(&env, FORMATTED, &formatting(), 0);
    let (_, docx) = export(&env, "docx", "rich.docx");
    let xml = docx_part(&docx, "word/document.xml");
    for tag in [
        "<w:b/>",
        "<w:i/>",
        "<w:u w:val=\"single\"/>",
        "<w:highlight w:val=\"yellow\"/>",
        "<w:sz w:val=\"40\"/>",
    ] {
        assert!(xml.contains(tag), "{tag} missing: {xml}");
    }
    let (r, pdf) = export(&env, "pdf", "rich.pdf");
    assert_eq!(r["pages"], 1);
    let bytes = std::fs::read(&pdf).unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("Helvetica-Oblique"));
    lopdf::Document::load(&pdf).expect("PDF parses");
    // Plain text carries the words only.
    let (_, txt) = export(&env, "txt", "rich.txt");
    let got = String::from_utf8(std::fs::read(&txt).unwrap()).unwrap();
    assert_eq!(got.trim_start_matches(BOM), FORMATTED.replace('\n', "\r\n"));
}
