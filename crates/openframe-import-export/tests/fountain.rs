//! Fountain import/export: golden output and round-trip stability (IEX-006, FSD §117).

use std::path::PathBuf;

use openframe_import_export::{ElementKind, SourceFormat, fountain, import_file};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/screenplay")
        .join(name)
}

fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n")
}

#[test]
fn fountain_fixture_parses_every_construct() {
    let out = import_file(&fixture("sample.fountain"), None).unwrap();
    assert_eq!(out.format, SourceFormat::Fountain);
    let d = &out.doc;
    assert_eq!(d.title_page.title(), Some("Black Rain"));
    assert_eq!(d.title_page.author(), Some("Nisha Verma"));
    assert_eq!(
        d.title_page.get("Contact"),
        Some("Nisha Verma\nnisha@example.com")
    );

    // Opening material + 4 headed scenes.
    let headings: Vec<&str> = d.scenes.iter().map(|s| s.heading.as_str()).collect();
    assert_eq!(
        headings,
        vec![
            "",
            "EXT. BUS STOP — NIGHT",
            "INT. APARTMENT — MORNING",
            "FLASHBACK - RAILWAY PLATFORM",
            "INT. POLICE STATION — NIGHT"
        ]
    );
    assert_eq!(d.headed_scene_count(), 4);
    assert_eq!(d.scenes[1].number.as_deref(), Some("1"));

    let s1 = &d.scenes[1];
    let kinds: Vec<ElementKind> = s1.elements.iter().map(|e| e.kind).collect();
    assert_eq!(
        kinds,
        vec![
            ElementKind::Action,
            ElementKind::Action,
            ElementKind::Character,
            ElementKind::Dialogue,
            ElementKind::Action,
            ElementKind::Character,
            ElementKind::Parenthetical,
            ElementKind::Dialogue,
            ElementKind::Character,
            ElementKind::Dialogue,
            ElementKind::Note,
            ElementKind::Transition,
        ]
    );
    assert_eq!(
        s1.elements[1].text,
        "ARJUN (30s, soaked, restless) checks his watch.\nHe looks up the empty road."
    );
    assert_eq!(
        s1.elements[9].text, "I said a lot of things.\n\nMost of them were true.",
        "intentional blank line kept"
    );
    assert!(
        !s1.elements
            .iter()
            .any(|e| e.text.contains("alternate beat")),
        "boneyard removed"
    );

    let s2 = &d.scenes[2];
    assert_eq!(s2.elements[0].kind, ElementKind::Synopsis);
    let dual: Vec<(ElementKind, bool)> = s2.elements.iter().map(|e| (e.kind, e.dual)).collect();
    assert!(dual.contains(&(ElementKind::Character, true)));
    assert_eq!(
        s2.elements.iter().filter(|e| e.dual).count(),
        4,
        "both speakers of the dual block"
    );
    assert!(
        s2.elements
            .iter()
            .any(|e| e.kind == ElementKind::Centered && e.text == "THE KETTLE SCREAMS")
    );

    let s3 = &d.scenes[3];
    assert_eq!(s3.elements[0].kind, ElementKind::Action);
    assert!(s3.elements[0].text.starts_with("NOBODY MOVES."));
    assert!(
        s3.elements
            .iter()
            .any(|e| e.kind == ElementKind::Character && e.text == "McCLANE")
    );
    assert_eq!(
        s3.elements
            .iter()
            .filter(|e| e.kind == ElementKind::Lyric)
            .count(),
        1
    );
    assert!(
        s3.elements
            .iter()
            .any(|e| e.kind == ElementKind::Transition && e.text == "SMASH CUT TO BLACK.")
    );
    assert!(s3.elements.iter().any(|e| e.kind == ElementKind::PageBreak));

    let s4 = &d.scenes[4];
    assert_eq!(s4.elements[0].text, "Arjun enters carrying a pistol.");
    assert_eq!(s4.elements[1].kind, ElementKind::Note);
    assert_eq!(s4.elements[1].text, "prop: replica only");

    assert_eq!(
        d.characters(),
        vec!["MEERA", "ARJUN", "McCLANE".to_uppercase().as_str()]
    );
}

#[test]
fn fountain_writer_matches_golden_file() {
    let out = import_file(&fixture("sample.fountain"), None).unwrap();
    let written = fountain::write(&out.doc, true, false);
    let golden_path = fixture("sample.expected.fountain");
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&golden_path, &written).unwrap();
    }
    let golden = normalize(&std::fs::read_to_string(&golden_path).expect("golden file present"));
    assert_eq!(written, golden);
}

#[test]
fn fountain_round_trip_is_stable() {
    let first = import_file(&fixture("sample.fountain"), None).unwrap().doc;
    let text1 = fountain::write(&first, true, false);
    let second = fountain::parse(&text1).unwrap().doc;
    assert!(
        first.same_content(&second),
        "doc → Fountain → doc must be equivalent"
    );
    let text2 = fountain::write(&second, true, false);
    assert_eq!(
        text1, text2,
        "Fountain → doc → Fountain must be byte-stable"
    );
}

#[test]
fn fountain_export_without_notes_omits_internal_material() {
    let doc = import_file(&fixture("sample.fountain"), None).unwrap().doc;
    let clean = fountain::write(&doc, false, false);
    assert!(!clean.contains("[["));
    assert!(!clean.contains("\n= "));
    assert!(clean.contains("EXT. BUS STOP — NIGHT #1#"));
}

#[test]
fn forced_elements_survive_export() {
    // Content that would be misread without forcing marks.
    let src =
        "INT. LAB - DAY\n\n!INT. IS NOT A HEADING HERE\n\n@mcgee\nHello.\n\n> not a transition\n";
    let doc = fountain::parse(src).unwrap().doc;
    let out = fountain::write(&doc, false, false);
    let back = fountain::parse(&out).unwrap().doc;
    assert!(doc.same_content(&back), "{out}");
    assert_eq!(back.scenes[0].elements[0].kind, ElementKind::Action);
    assert_eq!(back.scenes[0].elements[1].kind, ElementKind::Character);
    assert_eq!(back.scenes[0].elements[3].kind, ElementKind::Transition);
}
