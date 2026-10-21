//! Sample screenplay PDFs for visual review (not a regression test). Writes into
//! `release-out/pdf-samples/` (git ignored):
//!
//! ```text
//! node scripts/cargo-queued.mjs test -p openframe-import-export --test pdf_samples -- --ignored
//! ```

use std::path::Path;

use openframe_import_export::layout::{LayoutOptions, Paper};
use openframe_import_export::{Element, ElementKind, PageSetup, Scene, ScreenplayDoc, pdf_export};

fn film() -> ScreenplayDoc {
    let mut doc = ScreenplayDoc::default();
    doc.title_page.set("Title", "Monsoon Letters");
    doc.title_page.set("Credit", "Written by");
    doc.title_page.set("Author", "Asha Rao");
    doc.title_page.set(
        "Contact",
        "Northlight Pictures\n14 Example Road\nPune 411001",
    );
    doc.title_page.set("Draft date", "19 October 2026");
    let rows: [(&str, &[(ElementKind, &str)]); 3] = [
        (
            "INT. SAVITRI'S KITCHEN - DAWN",
            &[
                (
                    ElementKind::Action,
                    "Savitri wraps two rotis in newspaper and tucks a bundle of letters under the rice. The radio crackles with a flood warning. She switches it off and listens to the rain instead.",
                ),
                (ElementKind::Character, "SAVITRI"),
                (ElementKind::Parenthetical, "(to herself)"),
                (
                    ElementKind::Dialogue,
                    "You wrote that the river was kind in July. It is not kind in July. It is not kind in any month.",
                ),
                (ElementKind::Transition, "CUT TO:"),
            ],
        ),
        (
            "EXT. GUDUR BUS STAND - MORNING",
            &[
                (
                    ElementKind::Action,
                    "A tin-roofed bus stand. Porters sleep on sacks. ARJUN (60s, khaki uniform gone soft with washing) punches tickets without looking up, then looks up.",
                ),
                (ElementKind::Character, "ARJUN"),
                (ElementKind::Dialogue, "Bus to Kota. Rain permitting."),
                (ElementKind::Character, "SAVITRI"),
                (ElementKind::Dialogue, "It is always rain permitting."),
            ],
        ),
        (
            "INT. STATE BUS - DAY (MOVING)",
            &[
                (
                    ElementKind::Action,
                    "Rain hammers the roof. PINKY (8) kneels on the seat behind SAVITRI and draws on the fogged window.",
                ),
                (ElementKind::Character, "PINKY"),
                (
                    ElementKind::Dialogue,
                    "Are you reading to me or to the window?",
                ),
            ],
        ),
    ];
    for (n, (heading, els)) in rows.iter().enumerate() {
        let mut scene = Scene::new(*heading);
        scene.number = Some((n + 1).to_string());
        for (kind, text) in els.iter() {
            scene.elements.push(Element::new(*kind, *text));
        }
        doc.scenes.push(scene);
    }
    // Enough scenes to run onto several pages.
    for n in 4..=14 {
        let mut scene = Scene::new(format!("INT. DAK BUNGALOW, ROOM {n} - NIGHT"));
        scene.number = Some(n.to_string());
        scene.elements.push(Element::new(
            ElementKind::Action,
            "A narrow room, a desk, a window with no glass. Forty years of unsent replies, tied in jute twine. Savitri reads the top one and sits down on the floor. The lamp gutters and steadies.",
        ));
        scene
            .elements
            .push(Element::new(ElementKind::Character, "SAVITRI"));
        scene.elements.push(Element::new(
            ElementKind::Dialogue,
            "Forty years. You kept every one and sent none. I think I understand that better than I want to.",
        ));
        doc.scenes.push(scene);
    }
    doc
}

fn out(name: &str) -> std::path::PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .join("release-out")
        .join("pdf-samples");
    std::fs::create_dir_all(&root).unwrap();
    root.join(name)
}

#[test]
#[ignore = "writes the sample PDFs to release-out/pdf-samples"]
fn write_screenplay_samples() {
    let doc = film();
    let letter = LayoutOptions {
        scene_numbers: true,
        ..Default::default()
    };
    std::fs::write(
        out("20-screenplay-letter.pdf"),
        pdf_export::write(&doc, &letter).bytes,
    )
    .unwrap();
    let a4 = LayoutOptions {
        scene_numbers: true,
        page: PageSetup {
            paper: Paper::A4,
            ..Default::default()
        },
        ..Default::default()
    };
    std::fs::write(
        out("21-screenplay-a4.pdf"),
        pdf_export::write(&doc, &a4).bytes,
    )
    .unwrap();
}
