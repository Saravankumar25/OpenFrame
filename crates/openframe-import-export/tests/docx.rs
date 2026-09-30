//! DOCX import/export (FSD §19.5, §20.6; IEX-007) and archive safety.

use std::io::Write;
use std::path::{Path, PathBuf};

use openframe_import_export::{
    ElementKind, ExportFormat, ExportOptions, SourceFormat, export_to_path, import_file,
};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/screenplay")
        .join(name)
}

fn zip_file(path: &Path, entries: &[(&str, &[u8])]) {
    let f = std::fs::File::create(path).unwrap();
    let mut z = zip::ZipWriter::new(f);
    let o = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (n, b) in entries {
        z.start_file(*n, o).unwrap();
        z.write_all(b).unwrap();
    }
    z.finish().unwrap();
}

const CT: &[u8] = br#"<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"/>"#;

fn para(text: &str, left_twips: Option<u32>, jc: Option<&str>) -> String {
    let mut ppr = String::new();
    if let Some(l) = left_twips {
        ppr.push_str(&format!("<w:ind w:left=\"{l}\"/>"));
    }
    if let Some(j) = jc {
        ppr.push_str(&format!("<w:jc w:val=\"{j}\"/>"));
    }
    format!(
        "<w:p><w:pPr>{ppr}</w:pPr><w:r><w:rPr><w:b/></w:rPr><w:t xml:space=\"preserve\">{text}</w:t></w:r></w:p>"
    )
}

#[test]
fn docx_round_trip_keeps_readable_screenplay() {
    let doc = import_file(&fixture("sample.fountain"), None).unwrap().doc;
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("Black Rain.docx");
    let opts = ExportOptions {
        scene_numbers: true,
        ..Default::default()
    };
    export_to_path(&doc, ExportFormat::Docx, &opts, &dest).unwrap();
    let back = import_file(&dest, None).unwrap();
    assert_eq!(back.format, SourceFormat::Docx);
    assert_eq!(
        back.confidence.level,
        openframe_import_export::ConfidenceLevel::High
    );
    let b = back.doc;
    assert_eq!(b.headed_scene_count(), doc.headed_scene_count());
    let headings = |d: &openframe_import_export::ScreenplayDoc| {
        d.scenes
            .iter()
            .map(|s| s.heading.to_uppercase())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        headings(&b),
        headings(&doc),
        "scene numbers stripped back off headings"
    );
    assert_eq!(b.characters(), doc.characters());
    assert_eq!(b.title_page.title(), Some("BLACK RAIN"));
    assert_eq!(b.title_page.author(), Some("Nisha Verma"));
    // Dialogue text survives verbatim, dual dialogue comes back from the table layout.
    let dia = |d: &openframe_import_export::ScreenplayDoc| {
        d.scenes
            .iter()
            .flat_map(|s| s.elements.iter())
            .filter(|e| e.kind == ElementKind::Dialogue)
            .map(|e| e.text.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(dia(&b), dia(&doc));
    assert_eq!(b.scenes[2].elements.iter().filter(|e| e.dual).count(), 4);
}

#[test]
fn plain_word_document_is_classified_by_indentation() {
    let body = [
        para("INT. POLICE STATION - NIGHT", None, None),
        para("", None, None),
        para(
            "Arjun enters carrying a pistol. The room goes quiet.",
            None,
            None,
        ),
        para("", None, None),
        para("MEERA", Some(3168), None),
        para("(quietly)", Some(2304), None),
        para("You said you would never come back.", Some(1440), None),
        para("", None, None),
        para("CUT TO:", None, Some("right")),
        para("", None, None),
        para("EXT. STREET - DAY", None, None),
        para("", None, None),
        para(
            "Crowds. Horns. Meera pushes through the traffic.",
            None,
            None,
        ),
    ]
    .join("");
    let xml = format!(
        "<?xml version=\"1.0\"?><w:document xmlns:w=\"w\"><w:body>{body}</w:body></w:document>"
    );
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("plain.docx");
    zip_file(
        &p,
        &[
            ("[Content_Types].xml", CT),
            ("_rels/.rels", b"<Relationships/>"),
            ("word/document.xml", xml.as_bytes()),
        ],
    );
    let out = import_file(&p, None).unwrap();
    let d = out.doc;
    assert_eq!(d.headed_scene_count(), 2);
    let kinds: Vec<ElementKind> = d.scenes[0].elements.iter().map(|e| e.kind).collect();
    assert_eq!(
        kinds,
        vec![
            ElementKind::Action,
            ElementKind::Character,
            ElementKind::Parenthetical,
            ElementKind::Dialogue,
            ElementKind::Transition
        ]
    );
    assert!(
        d.warnings.iter().any(|w| w.code == "docx_heuristic"),
        "uncertainty is exposed"
    );
    assert!(out.confidence.level < openframe_import_export::ConfidenceLevel::High);
}

#[test]
fn zip_bomb_is_rejected_without_inflating_it() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("bomb.docx");
    let zeros = vec![b' '; 48 << 20];
    zip_file(
        &p,
        &[("[Content_Types].xml", CT), ("word/document.xml", &zeros)],
    );
    assert!(
        std::fs::metadata(&p).unwrap().len() < 1 << 20,
        "the bomb is small on disk"
    );
    let e = import_file(&p, None).unwrap_err();
    assert_eq!(e.code_str(), "import.unsafe_archive");
}

#[test]
fn garbage_and_non_word_archives_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let g = dir.path().join("garbage.docx");
    std::fs::write(&g, b"PK\x03\x04 this is not really a zip file at all").unwrap();
    assert!(
        import_file(&g, None)
            .unwrap_err()
            .code_str()
            .starts_with("import.")
    );

    let z = dir.path().join("photos.docx");
    zip_file(&z, &[("photo.jpg", b"\xFF\xD8\xFF")]);
    assert_eq!(
        import_file(&z, None).unwrap_err().code_str(),
        "import.docx_invalid"
    );

    let broken = dir.path().join("broken.docx");
    zip_file(
        &broken,
        &[
            ("[Content_Types].xml", CT),
            ("word/document.xml", b"<w:document><w:body><w:p>"),
        ],
    );
    assert_eq!(
        import_file(&broken, None).unwrap_err().code_str(),
        "import.docx_invalid"
    );

    let prose = dir.path().join("letter.docx");
    let xml = format!(
        "<w:document xmlns:w=\"w\"><w:body>{}</w:body></w:document>",
        para(
            "Dear Ravi, thanks for the lovely dinner last week.",
            None,
            None
        )
    );
    zip_file(
        &prose,
        &[
            ("[Content_Types].xml", CT),
            ("word/document.xml", xml.as_bytes()),
        ],
    );
    assert_eq!(
        import_file(&prose, None).unwrap_err().code_str(),
        "import.not_a_screenplay"
    );
}

#[test]
fn legacy_doc_is_explained() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("old.doc");
    std::fs::write(&p, [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1, 0, 0]).unwrap();
    let e = import_file(&p, None).unwrap_err();
    assert_eq!(e.code_str(), "import.unsupported_format");
    assert!(e.message.contains(".docx"));
}
