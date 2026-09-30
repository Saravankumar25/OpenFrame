//! A feature-length (120-page) screenplay imports and exports in a few seconds.

use std::time::{Duration, Instant};

use openframe_import_export::{
    Element, ElementKind, ExportFormat, ExportOptions, Scene, ScreenplayDoc, SourceFormat,
    fountain, import_bytes, import_file, layout, render_bytes,
};

const SCENES: usize = 185;

fn feature_length() -> ScreenplayDoc {
    let mut doc = ScreenplayDoc::default();
    doc.title_page.set("Title", "Performance");
    doc.title_page.set("Author", "OpenFrame");
    for i in 0..SCENES {
        let mut s = Scene::new(format!(
            "{}. PLACE NUMBER {i} - {}",
            if i % 2 == 0 { "INT" } else { "EXT" },
            if i % 3 == 0 { "DAY" } else { "NIGHT" }
        ));
        for k in 0..4 {
            s.elements.push(Element::new(
                ElementKind::Action,
                format!("Beat {k} of scene {i}. The crowd surges forward as the train doors open and a hundred umbrellas bloom at once."),
            ));
            s.elements.push(Element::new(
                ElementKind::Character,
                if k % 2 == 0 { "MEERA" } else { "ARJUN" },
            ));
            if k == 1 {
                s.elements.push(Element::new(
                    ElementKind::Parenthetical,
                    "(under his breath)",
                ));
            }
            s.elements.push(Element::new(
                ElementKind::Dialogue,
                "We keep telling ourselves the rain will stop, but it never does, does it?",
            ));
        }
        s.elements
            .push(Element::new(ElementKind::Transition, "CUT TO:"));
        doc.scenes.push(s);
    }
    doc
}

fn budget() -> Duration {
    // Generous enough for unoptimised debug builds on slow CI machines; release is far faster.
    Duration::from_secs(if cfg!(debug_assertions) { 8 } else { 3 })
}

#[test]
fn feature_length_script_round_trips_quickly() {
    let doc = feature_length();
    let pages = layout::layout(&doc, &ExportOptions::default().layout()).script_page_count();
    assert!(
        pages >= 120,
        "fixture must be feature length, got {pages} pages"
    );

    let t = Instant::now();
    let text = fountain::write(&doc, false, false);
    let parsed = fountain::parse(&text).unwrap().doc;
    assert_eq!(parsed.headed_scene_count(), SCENES);
    let fountain_time = t.elapsed();

    let t = Instant::now();
    let (pdf, _, _) = render_bytes(
        &doc,
        ExportFormat::Pdf,
        &ExportOptions {
            scene_numbers: true,
            ..Default::default()
        },
    )
    .unwrap();
    let pdf_export = t.elapsed();

    let t = Instant::now();
    let back = import_bytes(&pdf, SourceFormat::Pdf).unwrap().doc;
    assert_eq!(back.headed_scene_count(), SCENES);
    let pdf_import = t.elapsed();

    let t = Instant::now();
    let (fdx, _, _) = render_bytes(&doc, ExportFormat::Fdx, &ExportOptions::default()).unwrap();
    assert_eq!(
        import_bytes(&fdx, SourceFormat::Fdx)
            .unwrap()
            .doc
            .headed_scene_count(),
        SCENES
    );
    let fdx_time = t.elapsed();

    let t = Instant::now();
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("long.docx");
    openframe_import_export::export_to_path(
        &doc,
        ExportFormat::Docx,
        &ExportOptions::default(),
        &p,
    )
    .unwrap();
    assert_eq!(
        import_file(&p, None).unwrap().doc.headed_scene_count(),
        SCENES
    );
    let docx_time = t.elapsed();

    eprintln!(
        "{pages} pages: fountain {fountain_time:?}, pdf export {pdf_export:?}, pdf import {pdf_import:?}, fdx {fdx_time:?}, docx {docx_time:?}"
    );
    for (what, d) in [
        ("fountain", fountain_time),
        ("pdf export", pdf_export),
        ("pdf import", pdf_import),
        ("fdx", fdx_time),
        ("docx", docx_time),
    ] {
        assert!(d < budget(), "{what} took {d:?}");
    }
}
