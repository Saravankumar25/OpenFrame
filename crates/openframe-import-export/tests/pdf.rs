//! Screenplay PDF export (FSD §20.3, §123) and best-effort PDF import
//! (FSD §19.4, IEX-004): our own PDF parses back with the same scene count.

use std::path::PathBuf;

use openframe_import_export::layout::{self, LineStyle};
use openframe_import_export::{
    ConfidenceLevel, Element, ElementKind, ExportFormat, ExportOptions, Scene, ScreenplayDoc,
    SourceFormat, export_to_path, import_bytes, import_file, render_bytes,
};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/screenplay")
        .join(name)
}

fn long_script(scenes: usize) -> ScreenplayDoc {
    let mut doc = ScreenplayDoc::default();
    doc.title_page.set("Title", "The Long Night");
    doc.title_page.set("Author", "Test Writer");
    doc.title_page.set("Contact", "writer@example.com");
    for i in 0..scenes {
        let mut s = Scene::new(format!(
            "{}. LOCATION {i} - {}",
            if i % 2 == 0 { "INT" } else { "EXT" },
            if i % 3 == 0 { "DAY" } else { "NIGHT" }
        ));
        s.elements.push(Element::new(ElementKind::Action, format!("Scene {i} begins. Rain streaks the windows while the city hums below, restless and awake long after midnight.")));
        s.elements
            .push(Element::new(ElementKind::Character, "MEERA"));
        s.elements
            .push(Element::new(ElementKind::Parenthetical, "(quietly)"));
        s.elements.push(Element::new(ElementKind::Dialogue, "You said you would never come back. Not after everything that happened on the platform that night."));
        s.elements.push(Element::new(
            ElementKind::Character,
            if i % 4 == 0 { "ARJUN (V.O.)" } else { "ARJUN" },
        ));
        s.elements.push(Element::new(
            ElementKind::Dialogue,
            "I said a lot of things.".repeat(1 + i % 5),
        ));
        if i % 5 == 4 {
            s.elements
                .push(Element::new(ElementKind::Transition, "CUT TO:"));
        }
        doc.scenes.push(s);
    }
    doc
}

#[test]
fn pdf_layout_follows_screenplay_conventions() {
    let doc = long_script(30);
    let lay = layout::layout(
        &doc,
        &ExportOptions {
            scene_numbers: true,
            ..Default::default()
        }
        .layout(),
    );
    assert!(lay.pages[0].title_page);
    assert!(
        lay.pages[0]
            .lines
            .iter()
            .any(|l| l.text == "THE LONG NIGHT" && l.style == LineStyle::TitleMain)
    );
    let script = &lay.pages[1..];
    assert!(script.len() > 3);
    // Page 1 unnumbered, page 2 "2." top right.
    assert!(
        !script[0]
            .lines
            .iter()
            .any(|l| l.style == LineStyle::PageNumber)
    );
    let pn = script[1]
        .lines
        .iter()
        .find(|l| l.style == LineStyle::PageNumber)
        .unwrap();
    assert_eq!(pn.text, "2.");
    assert!(pn.x_in > 7.0);
    // Standard indents.
    let first = &script[0].lines;
    let heading = first
        .iter()
        .find(|l| l.style == LineStyle::Heading)
        .unwrap();
    assert!((heading.x_in - 1.5).abs() < 0.01);
    let cue = first
        .iter()
        .find(|l| l.style == LineStyle::Character)
        .unwrap();
    assert!((cue.x_in - 3.7).abs() < 0.01);
    let dia = first
        .iter()
        .find(|l| l.style == LineStyle::Dialogue)
        .unwrap();
    assert!((dia.x_in - 2.5).abs() < 0.01);
    // Scene numbers on both sides of the heading row.
    let nums: Vec<_> = first
        .iter()
        .filter(|l| l.style == LineStyle::SceneNumber && l.row == heading.row)
        .collect();
    assert_eq!(nums.len(), 2);
    assert!(nums[0].x_in < 1.5 && nums[1].x_in > 7.5);
    // No page exceeds 55 body lines.
    for p in script {
        assert!(p.lines.iter().all(|l| l.row < 55));
    }
}

#[test]
fn exported_pdf_parses_back_with_same_scene_count() {
    let doc = import_file(&fixture("sample.fountain"), None).unwrap().doc;
    let opts = ExportOptions {
        scene_numbers: true,
        revision_marks: true,
        ..Default::default()
    };
    let (bytes, pages, warnings) = render_bytes(&doc, ExportFormat::Pdf, &opts).unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
    assert!(pages.unwrap() >= 1);
    assert!(
        warnings
            .iter()
            .all(|w| w.code != "pdf_unsupported_characters"),
        "{warnings:?}"
    );

    let back = import_bytes(&bytes, SourceFormat::Pdf).unwrap();
    assert_eq!(back.format, SourceFormat::Pdf);
    assert_eq!(back.doc.headed_scene_count(), doc.headed_scene_count());
    let headings: Vec<String> = back
        .doc
        .scenes
        .iter()
        .filter(|s| !s.heading.is_empty())
        .map(|s| s.heading.clone())
        .collect();
    assert_eq!(
        headings,
        vec![
            "EXT. BUS STOP — NIGHT",
            "INT. APARTMENT — MORNING",
            "FLASHBACK - RAILWAY PLATFORM",
            "INT. POLICE STATION — NIGHT"
        ]
    );
    assert_eq!(back.doc.characters(), vec!["MEERA", "ARJUN", "MCCLANE"]);
    assert_eq!(back.doc.title_page.title(), Some("BLACK RAIN"));
    // Never claimed as certain; always carries the best-effort warning.
    assert!(back.confidence.level <= ConfidenceLevel::Medium);
    assert!(
        back.doc
            .warnings
            .iter()
            .any(|w| w.code == "pdf_best_effort")
    );
}

#[test]
fn split_dialogue_is_rejoined_on_import() {
    let mut doc = long_script(1);
    doc.scenes[0].elements.insert(
        0,
        Element::new(ElementKind::Action, "Filler line.\n".repeat(38)),
    );
    doc.scenes[0].elements[4].text = "This is a very long speech that goes on and on. "
        .repeat(12)
        .trim()
        .to_string();
    let lay = layout::layout(&doc, &ExportOptions::default().layout());
    assert!(
        lay.pages
            .iter()
            .flat_map(|p| p.lines.iter())
            .any(|l| l.style == LineStyle::More),
        "the fixture splits a speech"
    );
    let (bytes, _, _) = render_bytes(&doc, ExportFormat::Pdf, &ExportOptions::default()).unwrap();
    let back = import_bytes(&bytes, SourceFormat::Pdf).unwrap().doc;
    let dialogue: Vec<&Element> = back
        .scenes
        .iter()
        .flat_map(|s| s.elements.iter())
        .filter(|e| e.kind == ElementKind::Dialogue)
        .collect();
    assert_eq!(
        dialogue.len(),
        2,
        "(MORE)/(CONT'D) merged back into one speech"
    );
    assert_eq!(dialogue[0].text, doc.scenes[0].elements[4].text);
    assert!(
        !back
            .scenes
            .iter()
            .flat_map(|s| s.elements.iter())
            .any(|e| e.text.contains("(MORE)") || e.text.contains("CONT'D"))
    );
}

#[test]
fn non_latin_text_is_a_clear_export_error_and_nothing_is_written() {
    let mut doc = ScreenplayDoc::default();
    let mut s = Scene::new("INT. घर - DAY");
    s.elements.push(Element::new(
        ElementKind::Action,
        "Café au lait. मीरा enters.",
    ));
    doc.scenes.push(s);
    let e = render_bytes(&doc, ExportFormat::Pdf, &ExportOptions::default()).unwrap_err();
    assert_eq!(e.code_str(), "export.pdf_unsupported_characters");
    assert!(e.message.contains("“घ”"), "{}", e.message);
    assert!(e.message.contains("first on page 1"), "{}", e.message);
    assert!(
        !e.message.contains("“é”"),
        "WinAnsi accents are printable: {}",
        e.message
    );

    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("out.pdf");
    assert!(export_to_path(&doc, ExportFormat::Pdf, &ExportOptions::default(), &dest).is_err());
    assert!(!dest.exists());
    assert_eq!(
        std::fs::read_dir(dir.path()).unwrap().count(),
        0,
        "no partial file left behind"
    );

    // Formats with full Unicode keep the text.
    for f in [
        ExportFormat::Fountain,
        ExportFormat::Fdx,
        ExportFormat::Docx,
        ExportFormat::Txt,
    ] {
        assert!(
            render_bytes(&doc, f, &ExportOptions::default()).is_ok(),
            "{f:?}"
        );
    }
}

#[test]
fn scanned_or_broken_pdfs_are_rejected_with_the_standard_message() {
    // A valid PDF with no text layer (like a scan).
    let mut b = openframe_import_export::pdfgen::PdfBuilder::new(612.0, 792.0);
    b.add_page(pdf_writer::Content::new());
    let empty = b.finish("scan", None);
    let e = import_bytes(&empty, SourceFormat::Pdf).unwrap_err();
    assert_eq!(e.code_str(), "import.pdf_not_screenplay");
    assert_eq!(
        e.message,
        "The PDF could not be interpreted as a screenplay. Your current project was not changed."
    );

    assert_eq!(
        import_bytes(b"%PDF-1.7 garbage garbage", SourceFormat::Pdf)
            .unwrap_err()
            .code_str(),
        "import.pdf_unreadable"
    );
    assert_eq!(
        import_bytes(b"hello", SourceFormat::Pdf)
            .unwrap_err()
            .code_str(),
        "import.pdf_unreadable"
    );

    // A report-style PDF with prose only.
    let mut r = openframe_import_export::report::Document::new("Minutes");
    r.push(openframe_import_export::report::Block::Paragraph(
        "We discussed the budget and agreed to meet again next week to finalise it.".into(),
    ));
    let prose = r.to_pdf().bytes;
    assert_eq!(
        import_bytes(&prose, SourceFormat::Pdf)
            .unwrap_err()
            .code_str(),
        "import.pdf_not_screenplay"
    );
}

#[test]
fn pdf_is_written_atomically() {
    let doc = long_script(3);
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("out.pdf");
    std::fs::write(&dest, b"old").unwrap();
    let res = export_to_path(&doc, ExportFormat::Pdf, &ExportOptions::default(), &dest).unwrap();
    assert_eq!(res.bytes_written, std::fs::metadata(&dest).unwrap().len());
    let names: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names.len(), 1, "no temporary files left behind: {names:?}");
    // Writing into a missing folder fails cleanly.
    let bad = dir.path().join("missing").join("x.pdf");
    assert!(export_to_path(&doc, ExportFormat::Pdf, &ExportOptions::default(), &bad).is_err());
}
