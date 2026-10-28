//! Sample screenplay PDFs for visual and measured review of the v5 formatting standard (not a
//! regression test). Writes into `$SCREENPLAY_CHECK_DIR` (default `release-out/screenplay-check/`,
//! git ignored):
//!
//! ```text
//! node scripts/cargo-queued.mjs test -p openframe-import-export --test screenplay_check -- --ignored
//! ```

use std::path::PathBuf;

use openframe_import_export::layout::LayoutOptions;
use openframe_import_export::{
    Element, ElementKind, PageSetup, Scene, ScreenplayDoc, SourceFormat, import_bytes, pdf_export,
};

fn out_dir() -> PathBuf {
    let dir = std::env::var("SCREENPLAY_CHECK_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../release-out/screenplay-check")
        });
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn feature() -> ScreenplayDoc {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../templates/samples/Example-Feature.fountain");
    import_bytes(&std::fs::read(path).unwrap(), SourceFormat::Fountain)
        .unwrap()
        .doc
}

fn title_page_stress() -> ScreenplayDoc {
    let mut doc = ScreenplayDoc::default();
    doc.title_page.set("Title", "Low Tide Radio");
    doc.title_page.set("Author", "Asha Rao");
    doc.title_page.set(
        "Contact",
        "Kayal Pictures\n14 Example Road\nPune 411001\nasha@kayalpictures.example",
    );
    doc.title_page.set(
        "Copyright",
        "Copyright 2026 Kayal Pictures. All rights reserved.",
    );
    doc.title_page.set("Draft", "Second Draft");
    doc.title_page.set("Draft date", "24 October 2026");
    doc.title_page
        .set("Revision", "Blue revision, pages 4 to 19");
    doc.title_page.set(
        "Representation",
        "Representation: Lighthouse Talent, Mumbai",
    );
    let mut s = Scene::new("INT. RADIO ROOM - NIGHT");
    s.elements
        .push(Element::new(ElementKind::Action, "A single lamp burns."));
    doc.scenes.push(s);
    doc
}

/// Long speeches, transitions and shots placed to cross page boundaries.
fn stress() -> ScreenplayDoc {
    let mut doc = ScreenplayDoc::default();
    doc.title_page.set("Title", "Stress");
    doc.title_page.set("Author", "OpenFrame");
    let speech = "I kept the radio on all night because the silence was louder than the storm, and every time the signal broke I heard your voice in the static. ";
    for i in 0..14 {
        let mut s = Scene::new(format!("INT. ROOM {i} - NIGHT"));
        s.elements.push(Element::new(
            ElementKind::Action,
            "Rain on the glass. A kettle ticks as it cools. ".repeat(1 + (i * 3) % 5),
        ));
        s.elements
            .push(Element::new(ElementKind::Character, "MEERA"));
        s.elements
            .push(Element::new(ElementKind::Parenthetical, "(quietly)"));
        s.elements.push(Element::new(
            ElementKind::Dialogue,
            speech.repeat(3 + i % 4),
        ));
        s.elements
            .push(Element::new(ElementKind::Character, "ARJUN"));
        s.elements
            .push(Element::new(ElementKind::Dialogue, "Say it again."));
        s.elements
            .push(Element::new(ElementKind::Shot, "CLOSE ON THE DIAL"));
        s.elements
            .push(Element::new(ElementKind::Action, "The needle drifts."));
        s.elements
            .push(Element::new(ElementKind::Transition, "CUT TO:"));
        doc.scenes.push(s);
    }
    doc
}

#[test]
#[ignore = "writes sample PDFs for review"]
fn write_screenplay_check_pdfs() {
    let dir = out_dir();
    let write = |name: &str, doc: &ScreenplayDoc, opts: LayoutOptions| {
        let pdf = pdf_export::write(doc, &opts);
        std::fs::write(dir.join(name), pdf.bytes).unwrap();
    };
    let clean = LayoutOptions::default();
    write("a-clean-feature.pdf", &feature(), clean.clone());
    write(
        "b-shooting-draft.pdf",
        &feature(),
        LayoutOptions {
            scene_numbers: true,
            page: PageSetup {
                header: "THE TOWER AT PEDDA REVU - {draft} - {page}".into(),
                header_on_first_page: true,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    write("c-title-page.pdf", &title_page_stress(), clean.clone());
    write("d-stress.pdf", &stress(), clean);
}
