//! TXT and pasted screenplay text (FSD §19.6, Import/Export §4.8, §4.11).

use std::path::PathBuf;

use openframe_import_export::{
    ElementKind, ExportFormat, ExportOptions, SourceFormat, import_bytes, import_file, import_text,
    render_bytes,
};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/screenplay")
        .join(name)
}

#[test]
fn pasted_text_is_recognised_with_honest_warnings() {
    let text = std::fs::read_to_string(fixture("pasted.txt")).unwrap();
    let out = import_text(&text, false).unwrap();
    assert_eq!(out.format, SourceFormat::Pasted);
    let d = &out.doc;
    let headings: Vec<&str> = d.scenes.iter().map(|s| s.heading.as_str()).collect();
    assert_eq!(
        headings,
        vec![
            "INT. POLICE STATION — NIGHT",
            "STREET — DAY",
            "EXT. RAILWAY PLATFORM - DUSK"
        ]
    );
    let kinds: Vec<ElementKind> = d.scenes[0].elements.iter().map(|e| e.kind).collect();
    assert_eq!(
        kinds,
        vec![
            ElementKind::Action,
            ElementKind::Character,
            ElementKind::Dialogue,
            ElementKind::Character,
            ElementKind::Parenthetical,
            ElementKind::Dialogue
        ]
    );
    assert_eq!(
        d.scenes[1].elements.last().unwrap().kind,
        ElementKind::Transition
    );
    // "STREET — DAY" has no INT./EXT.: flagged, never presented as certain.
    let w = d
        .warnings
        .iter()
        .find(|w| w.code == "uncertain_heading")
        .unwrap();
    assert_eq!(w.scene, Some(1));
    assert!(d.warnings.iter().any(|w| w.code == "heuristic_parse"));
    assert_eq!(d.characters(), vec!["MEERA", "ARJUN"]);
}

#[test]
fn indented_plain_text_uses_layout() {
    let out = import_file(&fixture("indented.txt"), None).unwrap();
    assert_eq!(out.format, SourceFormat::Txt);
    let d = &out.doc;
    assert_eq!(d.headed_scene_count(), 2);
    assert_eq!(d.scenes[1].heading, "EXT. BUS STOP - NIGHT");
    assert_eq!(d.scenes[1].number.as_deref(), Some("1"));
    let s = &d.scenes[1];
    assert_eq!(
        s.elements[0].text,
        "Rain hammers the tin roof. A single streetlight flickers above an empty road."
    );
    assert_eq!(s.elements[1].kind, ElementKind::Character);
    assert_eq!(s.elements[2].kind, ElementKind::Parenthetical);
    assert_eq!(
        s.elements[3].text,
        "You said you would never come back. Not after what happened."
    );
    assert_eq!(s.elements.last().unwrap().kind, ElementKind::Transition);
    assert_eq!(
        d.scenes[0].elements[0].kind,
        ElementKind::Transition,
        "FADE IN:"
    );
    assert!(out.confidence.score > 0.5);
}

#[test]
fn txt_export_reimports() {
    let doc = import_file(&fixture("sample.fountain"), None).unwrap().doc;
    let (bytes, _, _) = render_bytes(
        &doc,
        ExportFormat::Txt,
        &ExportOptions {
            title_page: false,
            ..Default::default()
        },
    )
    .unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(text.contains("                    MEERA"));
    let back = import_bytes(&bytes, SourceFormat::Txt).unwrap().doc;
    assert_eq!(back.headed_scene_count(), doc.headed_scene_count());
    assert_eq!(back.characters(), vec!["MEERA", "ARJUN", "MCCLANE"]);
}

#[test]
fn prose_and_empty_text_are_rejected() {
    let e = import_file(&fixture("not_a_screenplay.txt"), None).unwrap_err();
    assert_eq!(e.code_str(), "import.not_a_screenplay");
    assert!(e.message.ends_with("Your current project was not changed."));
    assert_eq!(
        import_text("   \n  ", false).unwrap_err().code_str(),
        "import.empty_source"
    );
}

#[test]
fn legacy_encodings_are_decoded() {
    let bytes = b"INT. CAF\xC9 - DAY\n\nJos\xE9 waits.\n".to_vec();
    let out = import_bytes(&bytes, SourceFormat::Txt).unwrap();
    assert_eq!(out.doc.scenes[0].heading, "INT. CAFÉ - DAY");
    assert!(out.doc.warnings.iter().any(|w| w.code == "legacy_encoding"));
}

#[test]
fn pasted_fountain_with_a_title_page_is_read_as_fountain() {
    let text = "Title: Black Rain\nAuthor: Nisha Verma\n\nINT. RAILWAY PLATFORM - DAWN\n\nMeera waits with a red umbrella.\n\nMEERA\nThe last train is gone.\n";
    let out = import_text(text, false).unwrap();
    assert_eq!(out.format, SourceFormat::Pasted);
    assert_eq!(out.doc.title_page.title(), Some("Black Rain"));
    assert_eq!(out.doc.scenes.len(), 1);
    let first = &out.doc.scenes[0];
    assert_eq!(first.heading, "INT. RAILWAY PLATFORM - DAWN");
    assert!(
        first.elements.iter().all(|e| !e.text.contains("Title:")),
        "the title page never becomes scene text: {:?}",
        first.elements
    );
    // Plain pasted text without a title page still uses the text heuristics.
    let plain = import_text("INT. ROOM - DAY\n\nTitle music plays.\n", false).unwrap();
    assert!(plain.doc.title_page.title().is_none());
}
