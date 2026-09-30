//! Final Draft import/export (FSD §19.7, §20.4; IEX-005).

use std::path::PathBuf;

use openframe_import_export::fdx::{self, FdxOptions};
use openframe_import_export::{
    ElementKind, ExportFormat, ExportOptions, SourceFormat, export_to_path, import_file,
};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/screenplay")
        .join(name)
}

#[test]
fn fdx_fixture_maps_paragraph_types() {
    let out = import_file(&fixture("sample.fdx"), None).unwrap();
    assert_eq!(out.format, SourceFormat::Fdx);
    let d = &out.doc;
    assert_eq!(d.headed_scene_count(), 2);
    assert_eq!(d.scenes[0].heading, "", "FADE IN: before the first heading");
    assert_eq!(d.scenes[0].elements[0].kind, ElementKind::Transition);
    let s1 = &d.scenes[1];
    assert_eq!(s1.heading, "EXT. BUS STOP - NIGHT");
    assert_eq!(s1.number.as_deref(), Some("1"));
    assert_eq!(
        s1.elements[0].text,
        "Rain hammers the tin roof. A single streetlight flickers & dies."
    );
    assert_eq!(s1.elements[3].kind, ElementKind::Dialogue);
    assert_eq!(
        s1.elements[3].revision.as_deref(),
        Some("Blue"),
        "revision metadata preserved"
    );
    // Cast List is unsupported: content kept as action and reported.
    assert!(
        s1.elements
            .iter()
            .any(|e| e.text == "MEERA, ARJUN" && e.kind == ElementKind::Action)
    );
    assert!(
        d.warnings
            .iter()
            .any(|w| w.code == "unsupported_fdx_paragraph")
    );
    assert!(
        d.warnings
            .iter()
            .any(|w| w.code == "fdx_script_notes_ignored")
    );

    let s2 = &d.scenes[2];
    assert_eq!(s2.elements.iter().filter(|e| e.dual).count(), 4);
    assert!(s2.elements.iter().any(|e| e.kind == ElementKind::Shot));
    assert!(
        s2.elements
            .iter()
            .any(|e| e.kind == ElementKind::Centered && e.text == "THE END")
    );
    // Declared entities are never expanded.
    assert!(s2.elements.iter().any(|e| e.text == "&boom;"));

    assert_eq!(d.title_page.title(), Some("BLACK RAIN"));
    assert_eq!(d.title_page.author(), Some("Nisha Verma"));
    assert_eq!(d.title_page.get("Contact"), Some("nisha@example.com"));
    assert_eq!(d.characters(), vec!["MEERA", "ARJUN"]);
}

#[test]
fn fdx_round_trip_preserves_structure() {
    let first = import_file(&fixture("sample.fdx"), None).unwrap().doc;
    let opts = FdxOptions {
        scene_numbers: false,
        include_notes: false,
        revision_marks: true,
        title_page: true,
    };
    let xml = fdx::write(&first, &opts);
    let second = fdx::parse(xml.as_bytes()).unwrap().doc;
    assert_eq!(
        first.scenes, second.scenes,
        "scenes, elements, dual flags and revisions survive"
    );
    assert_eq!(second.title_page.title(), first.title_page.title());
    assert_eq!(second.title_page.author(), first.title_page.author());
    // Stable on a second pass.
    assert_eq!(fdx::write(&second, &opts), xml);
}

#[test]
fn fountain_to_fdx_and_back_keeps_screenplay() {
    let doc = import_file(&fixture("sample.fountain"), None).unwrap().doc;
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("black rain.fdx");
    let res = export_to_path(&doc, ExportFormat::Fdx, &ExportOptions::default(), &dest).unwrap();
    assert!(res.bytes_written > 0);
    let back = import_file(&dest, None).unwrap().doc;
    assert_eq!(back.headed_scene_count(), doc.headed_scene_count());
    assert_eq!(back.characters(), doc.characters());
    for (a, b) in doc.scenes.iter().zip(&back.scenes) {
        assert_eq!(a.heading, b.heading);
        let speech = |s: &openframe_import_export::Scene| {
            s.elements
                .iter()
                .filter(|e| {
                    matches!(
                        e.kind,
                        ElementKind::Character
                            | ElementKind::Dialogue
                            | ElementKind::Parenthetical
                            | ElementKind::Lyric
                    )
                })
                .map(|e| e.text.to_uppercase())
                .collect::<Vec<_>>()
        };
        assert_eq!(speech(a), speech(b));
    }
}

#[test]
fn malformed_and_foreign_fdx_are_rejected_humanely() {
    let e = import_file(&fixture("malformed.fdx"), None).unwrap_err();
    assert_eq!(e.code_str(), "import.fdx_malformed");
    assert!(e.message.contains("Your current project was not changed"));

    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("fake.fdx");
    std::fs::write(&p, "<html><body>hello</body></html>").unwrap();
    assert_eq!(
        import_file(&p, None).unwrap_err().code_str(),
        "import.fdx_invalid"
    );

    let empty = dir.path().join("empty.fdx");
    std::fs::write(
        &empty,
        r#"<FinalDraft DocumentType="Script"><Content></Content></FinalDraft>"#,
    )
    .unwrap();
    assert_eq!(
        import_file(&empty, None).unwrap_err().code_str(),
        "import.fdx_empty"
    );
}

#[test]
fn deeply_nested_xml_is_refused() {
    let mut s = String::from("<FinalDraft><Content>");
    for _ in 0..400 {
        s.push_str("<a>");
    }
    for _ in 0..400 {
        s.push_str("</a>");
    }
    s.push_str("</Content></FinalDraft>");
    assert_eq!(
        fdx::parse(s.as_bytes()).unwrap_err().code_str(),
        "import.fdx_malformed"
    );
}
