//! FDX round-trip suite (Screenplay and Production Depth Specification §37; feature XFER-003):
//! thirteen script classes, each with a source file, the expected parsed structure, the
//! expected exported FDX and the expected rendered PDF (page layout snapshot and the PDF's
//! SHA-256). For every class:
//!
//! 1. parsing `source.fdx` gives exactly `parsed.json` (elements, custom styles, scene
//!    numbers, title page, revisions and warnings);
//! 2. exporting that structure gives exactly `exported.fdx`;
//! 3. re-importing `exported.fdx` gives the same structure (custom elements, numbers, title
//!    data, revisions and page breaks survive; nothing is flattened);
//! 4. the PDF of that structure has exactly the pages of `pages.txt` and the same bytes.
//!
//! Regenerate the expected files deliberately, then review every diff before committing:
//!
//! ```text
//! OPENFRAME_UPDATE_GOLDEN=1 cargo test -p openframe-import-export --test fdx_suite
//! ```

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use openframe_import_export::{
    ElementKind, ExportFormat, ExportOptions, ScreenplayDoc, SourceFormat, import_bytes, layout,
    render_bytes,
};

const CLASSES: [&str; 13] = [
    "01_basic",
    "02_dialogue_heavy",
    "03_parenthetical_heavy",
    "04_transitions",
    "05_action_heavy",
    "06_multilingual",
    "07_custom_elements",
    "08_revision_marked",
    "09_scene_numbered_production_draft",
    "10_title_page",
    "11_explicit_page_breaks",
    "12_dialogue_spanning_pages",
    "13_unusual_legacy_constructs",
];

fn suite_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fdx-suite")
}

fn update_mode() -> bool {
    std::env::var("OPENFRAME_UPDATE_GOLDEN").is_ok_and(|v| v == "1")
}

/// Final Draft export: stored scene numbers are written, none are invented.
fn fdx_options() -> ExportOptions {
    ExportOptions {
        revision_marks: true,
        ..Default::default()
    }
}

/// PDF: scene numbers in the margins and revision asterisks, as a production draft prints.
fn options() -> ExportOptions {
    ExportOptions {
        scene_numbers: true,
        revision_marks: true,
        ..Default::default()
    }
}

/// The structure without import warnings (they describe the source, not the script) and
/// without notes: notes never print and an FDX export leaves them out unless the writer asks
/// for them (then they are visible "[Note: …]" paragraphs).
fn structure(doc: &ScreenplayDoc) -> serde_json::Value {
    let mut doc = doc.clone();
    for s in &mut doc.scenes {
        s.elements.retain(|e| e.kind != ElementKind::Note);
    }
    let mut v = serde_json::to_value(&doc).unwrap();
    v.as_object_mut().unwrap().remove("warnings");
    v
}

fn parsed_snapshot(doc: &ScreenplayDoc) -> String {
    serde_json::to_string_pretty(&serde_json::to_value(doc).unwrap()).unwrap() + "\n"
}

/// Every printed line: page, row, column in hundredths of an inch, style and text.
fn pages_snapshot(doc: &ScreenplayDoc, pdf: &[u8]) -> String {
    let lay = layout::layout(doc, &options().layout());
    let mut out = String::new();
    for p in &lay.pages {
        match (p.title_page, p.number) {
            (true, _) => out.push_str("=== title page\n"),
            (false, Some(n)) => writeln!(out, "=== page {n}").unwrap(),
            (false, None) => out.push_str("=== page\n"),
        }
        for l in &p.lines {
            writeln!(
                out,
                "{:>3} {:>4} {:<14} {}",
                l.row,
                (l.x_in * 100.0).round() as i32,
                format!("{:?}", l.style),
                l.text
            )
            .unwrap();
        }
    }
    writeln!(out, "pdf-sha256 {}", openframe_security::sha256_bytes(pdf)).unwrap();
    out
}

/// Compare with (or, in update mode, write) an expected file.
fn expect(dir: &Path, name: &str, actual: &[u8], failures: &mut Vec<String>, class: &str) {
    let path = dir.join(name);
    if update_mode() {
        std::fs::write(&path, actual).unwrap();
        return;
    }
    match std::fs::read(&path) {
        Ok(want) if want == actual => {}
        Ok(_) => {
            let got = path.with_extension(format!(
                "{}.actual",
                path.extension().unwrap().to_string_lossy()
            ));
            std::fs::write(&got, actual).unwrap();
            failures.push(format!(
                "{class}: {name} differs (actual output written to {})",
                got.display()
            ));
        }
        Err(_) => failures.push(format!(
            "{class}: {name} is missing (run with OPENFRAME_UPDATE_GOLDEN=1, then review it)"
        )),
    }
}

#[test]
fn thirteen_fdx_classes_parse_export_reimport_and_print_as_expected() {
    let mut failures = Vec::new();
    for class in CLASSES {
        let dir = suite_dir().join(class);
        let source = std::fs::read(dir.join("source.fdx")).unwrap();
        let parsed = import_bytes(&source, SourceFormat::Fdx)
            .unwrap_or_else(|e| panic!("{class}: {e:?}"))
            .doc;
        expect(
            &dir,
            "parsed.json",
            parsed_snapshot(&parsed).as_bytes(),
            &mut failures,
            class,
        );

        let (fdx, _, _) = render_bytes(&parsed, ExportFormat::Fdx, &fdx_options()).unwrap();
        expect(&dir, "exported.fdx", &fdx, &mut failures, class);

        let back = import_bytes(&fdx, SourceFormat::Fdx).unwrap().doc;
        if structure(&back) != structure(&parsed) {
            failures.push(format!(
                "{class}: re-importing the exported FDX changed the structure\nparsed: {}\nback:   {}",
                structure(&parsed),
                structure(&back)
            ));
        }

        let (pdf, _, _) = render_bytes(&parsed, ExportFormat::Pdf, &options()).unwrap();
        let (again, _, _) = render_bytes(&parsed, ExportFormat::Pdf, &options()).unwrap();
        assert_eq!(pdf, again, "{class}: PDF bytes are deterministic");
        expect(
            &dir,
            "pages.txt",
            pages_snapshot(&parsed, &pdf).as_bytes(),
            &mut failures,
            class,
        );
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// What each class is for: the suite would be worthless if a class silently lost the
/// construct it exists to test.
#[test]
fn each_class_exercises_its_construct() {
    let load = |class: &str| {
        let bytes = std::fs::read(suite_dir().join(class).join("source.fdx")).unwrap();
        import_bytes(&bytes, SourceFormat::Fdx).unwrap().doc
    };
    let kinds = |d: &ScreenplayDoc, k: ElementKind| {
        d.scenes
            .iter()
            .flat_map(|s| &s.elements)
            .filter(|e| e.kind == k)
            .count()
    };
    let lay = |d: &ScreenplayDoc| layout::layout(d, &options().layout());

    assert!(kinds(&load("02_dialogue_heavy"), ElementKind::Dialogue) >= 20);
    assert!(kinds(&load("03_parenthetical_heavy"), ElementKind::Parenthetical) >= 6);
    assert_eq!(kinds(&load("04_transitions"), ElementKind::Transition), 6);
    assert!(kinds(&load("05_action_heavy"), ElementKind::Action) >= 14);
    assert!(lay(&load("05_action_heavy")).script_page_count() >= 2);

    let multi = load("06_multilingual");
    let all: String = serde_json::to_string(&multi).unwrap();
    for c in ['మ', 'म', 'ந', 'അ', 'ನ', '\u{200C}', '\u{200D}'] {
        assert!(all.contains(c), "multilingual keeps {c:?}");
    }

    let custom = load("07_custom_elements");
    let styles: Vec<_> = custom
        .scenes
        .iter()
        .flat_map(|s| &s.elements)
        .filter_map(|e| e.custom.as_ref())
        .collect();
    assert_eq!(
        styles.len(),
        3,
        "custom paragraphs are never flattened into action"
    );
    let tel = styles
        .iter()
        .find(|s| s.export_name == "TELEPHONE")
        .unwrap();
    assert!(tel.italic && tel.uppercase);
    assert_eq!(tel.look, "dialogue");
    let lower = styles
        .iter()
        .find(|s| s.export_name == "Lower Third")
        .unwrap();
    assert_eq!(lower.look, "centered");
    assert_eq!(lower.space_before, 2);

    let revised = load("08_revision_marked");
    let marks: Vec<_> = revised
        .scenes
        .iter()
        .flat_map(|s| &s.elements)
        .filter_map(|e| e.revision.clone())
        .collect();
    assert!(
        marks.contains(&"Blue".to_string()) && marks.contains(&"Pink".to_string()),
        "{marks:?}"
    );
    assert!(
        lay(&revised)
            .pages
            .iter()
            .flat_map(|p| &p.lines)
            .any(|l| l.style == layout::LineStyle::RevisionMark)
    );

    let numbered = load("09_scene_numbered_production_draft");
    let numbers: Vec<_> = numbered
        .scenes
        .iter()
        .filter_map(|s| s.number.clone())
        .collect();
    assert_eq!(numbers, ["1", "2", "2A", "3", "4", "5"]);

    let title = load("10_title_page");
    assert_eq!(title.title_page.title(), Some("THE LAST LIGHT"));
    assert!(
        title
            .title_page
            .author()
            .unwrap_or_default()
            .contains("Asha Rao")
    );
    assert!(lay(&title).pages[0].title_page);

    let breaks = load("11_explicit_page_breaks");
    assert_eq!(kinds(&breaks, ElementKind::PageBreak), 2);

    let spanning = lay(&load("12_dialogue_spanning_pages"));
    let lines: Vec<_> = spanning.pages.iter().flat_map(|p| &p.lines).collect();
    assert!(lines.iter().any(|l| l.text == "(MORE)"));
    assert!(lines.iter().any(|l| l.text.contains("(CONT'D)")));

    let legacy = load("13_unusual_legacy_constructs");
    let text = serde_json::to_string(&legacy).unwrap();
    assert!(
        text.contains("THE FILE is gone"),
        "styled runs are joined: {text}"
    );
    assert!(text.contains("A paragraph without a type."));
    assert!(text.contains("& an ampersand é"));
    assert!(text.contains("Dust on the shelves."));
    assert!(
        legacy
            .scenes
            .iter()
            .flat_map(|s| &s.elements)
            .any(|e| e.kind == ElementKind::Note && e.text == "a note from the producer"),
        "an attached script note is kept as a note"
    );
    assert!(
        text.contains("Cast List"),
        "an unknown paragraph type is kept as a custom element"
    );
    assert_eq!(
        legacy
            .scenes
            .iter()
            .flat_map(|s| &s.elements)
            .filter(|e| e.dual)
            .count(),
        8,
        "dual dialogue inside a paragraph and straight under Content"
    );
    let codes: Vec<&str> = legacy.warnings.iter().map(|w| w.code.as_str()).collect();
    assert!(
        codes.contains(&"fdx_unknown_block")
            && codes.contains(&"fdx_script_notes_kept")
            && codes.contains(&"fdx_settings_not_used"),
        "an unread block is reported: {codes:?}"
    );
    assert_eq!(
        legacy.scenes[1].heading, "int. archive - later",
        "headings keep their text; capitals are display"
    );
}
