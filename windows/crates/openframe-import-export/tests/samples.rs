//! The four OpenFrame-owned sample scripts as regression fixtures (masterplan area "Call sheets,
//! budget, templates, samples and exports", box "Sample scripts"; `templates/samples/README.md`).
//!
//! Every sample must: be embedded in the app, carry the copyright note, follow the writing
//! standard (no em or en dashes), parse with the regular importer to the scene count the README
//! states, keep the constructs it exists to exercise, and survive a round trip through Fountain,
//! Final Draft and PDF export without losing a scene.

use std::path::Path;

use openframe_import_export::samples::{COPYRIGHT_NOTE, SAMPLES, find, import};
use openframe_import_export::{
    ExportFormat, ExportOptions, ScreenplayDoc, SourceFormat, import_bytes, render_bytes,
};

fn samples_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../templates/samples")
}

fn parse(id: &str) -> ScreenplayDoc {
    import(id)
        .unwrap_or_else(|e| panic!("{id} parses: {e}"))
        .1
        .doc
}

#[test]
fn the_four_samples_are_embedded_and_match_the_files_on_disk() {
    let ids: Vec<&str> = SAMPLES.iter().map(|s| s.id).collect();
    assert_eq!(
        ids,
        [
            "example-feature",
            "example-short",
            "example-scene",
            "fdx-interchange-test"
        ]
    );
    for s in SAMPLES {
        let on_disk = std::fs::read(samples_dir().join(s.file_name)).unwrap();
        assert_eq!(
            s.bytes,
            on_disk.as_slice(),
            "{} is embedded unchanged",
            s.id
        );
    }
    // Every script file in the folder is one of the shipped samples (no stray copies).
    for entry in std::fs::read_dir(samples_dir()).unwrap() {
        let name = entry.unwrap().file_name().to_string_lossy().into_owned();
        if name.ends_with(".md") {
            continue;
        }
        assert!(
            SAMPLES.iter().any(|s| s.file_name == name),
            "{name} in templates/samples is not a shipped sample"
        );
    }
    assert!(find("not-a-sample").is_none());
    assert!(import("not-a-sample").is_err());
}

#[test]
fn every_sample_carries_the_copyright_note_and_follows_the_writing_standard() {
    for s in SAMPLES {
        let text = String::from_utf8_lossy(s.bytes);
        assert!(
            text.contains(COPYRIGHT_NOTE),
            "{} carries \"{COPYRIGHT_NOTE}\"",
            s.id
        );
        assert!(
            !text.contains('\u{2014}') && !text.contains('\u{2013}'),
            "{} has no em or en dash",
            s.id
        );
    }
}

#[test]
fn every_sample_parses_to_the_scene_count_the_readme_states() {
    for s in SAMPLES {
        let (sample, outcome) = import(s.id).unwrap();
        assert_eq!(sample.id, s.id);
        assert_eq!(outcome.format, s.format);
        assert_eq!(
            outcome.doc.headed_scene_count(),
            s.scenes,
            "{} scene count",
            s.id
        );
        assert!(
            outcome.doc.element_count() > s.scenes,
            "{} has more than headings",
            s.id
        );
        let readme = std::fs::read_to_string(samples_dir().join("README.md")).unwrap();
        assert!(
            readme.contains(&format!("| {} |", s.scenes)),
            "README states {} scenes for {}",
            s.scenes,
            s.id
        );
    }
}

#[test]
fn the_feature_keeps_its_characters_locations_and_indian_scripts() {
    let doc = parse("example-feature");
    assert!(
        doc.characters().len() >= 10,
        "ten speaking parts: {:?}",
        doc.characters()
    );
    let mut sets: Vec<String> = doc
        .scenes
        .iter()
        .map(|s| {
            // The set is the heading without its time of day.
            s.heading
                .rsplit_once(" - ")
                .map(|(set, _)| set)
                .unwrap_or(&s.heading)
                .trim()
                .to_string()
        })
        .collect();
    sets.sort();
    sets.dedup();
    assert!(sets.len() > 20, "more than 20 locations: {}", sets.len());
    let all: String = doc
        .scenes
        .iter()
        .flat_map(|s| s.elements.iter().map(|e| e.text.clone()))
        .collect();
    assert!(
        all.chars().any(|c| ('\u{0C00}'..='\u{0C7F}').contains(&c)),
        "Telugu dialogue"
    );
    assert!(
        all.chars().any(|c| ('\u{0900}'..='\u{097F}').contains(&c)),
        "Hindi dialogue"
    );
    for tod in ["DAY", "NIGHT", "DUSK", "DAWN"] {
        assert!(
            doc.scenes.iter().any(|s| s.heading.ends_with(tod)),
            "a {tod} scene"
        );
    }
}

#[test]
fn the_fdx_test_keeps_lettered_scene_numbers_and_three_indian_scripts() {
    let doc = parse("fdx-interchange-test");
    let numbers: Vec<String> = doc.scenes.iter().filter_map(|s| s.number.clone()).collect();
    assert!(numbers.iter().any(|n| n == "12A"), "{numbers:?}");
    assert!(numbers.iter().any(|n| n == "12B"), "{numbers:?}");
    let all: String = doc
        .scenes
        .iter()
        .flat_map(|s| s.elements.iter().map(|e| e.text.clone()))
        .collect();
    for (name, range) in [
        ("Telugu", '\u{0C00}'..='\u{0C7F}'),
        ("Hindi", '\u{0900}'..='\u{097F}'),
        ("Tamil", '\u{0B80}'..='\u{0BFF}'),
    ] {
        assert!(all.chars().any(|c| range.contains(&c)), "{name} text");
    }
    assert!(
        doc.title_page.title().is_some(),
        "title page fields survive"
    );
}

#[test]
fn every_sample_round_trips_through_fountain_fdx_and_pdf_without_losing_a_scene() {
    for s in SAMPLES {
        let doc = parse(s.id);
        let opts = ExportOptions::default();
        let headings: Vec<String> = doc
            .scenes
            .iter()
            .filter(|x| !x.heading.trim().is_empty())
            .map(|x| x.heading.to_uppercase())
            .collect();
        for (format, back) in [
            (ExportFormat::Fountain, SourceFormat::Fountain),
            (ExportFormat::Fdx, SourceFormat::Fdx),
        ] {
            let (bytes, _, _) = render_bytes(&doc, format, &opts).unwrap();
            let again = import_bytes(&bytes, back).unwrap().doc;
            let again_headings: Vec<String> = again
                .scenes
                .iter()
                .filter(|x| !x.heading.trim().is_empty())
                .map(|x| x.heading.to_uppercase())
                .collect();
            assert_eq!(again_headings, headings, "{} through {format:?}", s.id);
        }
        let (pdf, pages, _) = render_bytes(&doc, ExportFormat::Pdf, &opts).unwrap();
        assert!(pdf.starts_with(b"%PDF-"), "{} exports a PDF", s.id);
        assert!(pages.unwrap_or(0) >= 1, "{} has pages", s.id);
        // Screenplay files carry no OpenFrame branding.
        assert!(
            !String::from_utf8_lossy(&pdf).contains("Created with OpenFrame Studio"),
            "{} screenplay PDF is unbranded",
            s.id
        );
    }
}
