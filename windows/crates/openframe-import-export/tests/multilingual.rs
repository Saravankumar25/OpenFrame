//! Multilingual round trip (ADR-0016; feature SCRIPT-UNI-001): a script that mixes Telugu,
//! Hindi, Tamil, Malayalam, Kannada and English, with conjuncts, reordered vowel signs and
//! zero-width joiners, keeps every character through Fountain, Final Draft, DOCX, plain text and
//! PDF, and its PDF is byte-identical from run to run.

use openframe_import_export::{
    Element, ElementKind, ExportFormat, ExportOptions, Scene, ScreenplayDoc, SourceFormat,
    export_to_path, import_bytes, import_file, render_bytes,
};

const TELUGU: &str = "మీరా రైలు స్టేషన్‌లో వేచి ఉంది. క్షమించండి!";
const HINDI: &str = "मीरा ने कहा, “क्षमा करें, ट्रेन देर से है।”";
const TAMIL: &str = "நீ எங்கே போகிறாய்? ஸ்ரீ வருகிறார்.";
const MALAYALAM: &str = "അവന്‍ വന്നു. അവൻ പോയി.";
const KANNADA: &str = "ನಾನು ಬರುತ್ತೇನೆ, ಕ್ಷಮಿಸಿ.";
const MIXED: &str = "Rain on the platform. మీరా looks at अर्जुन, then at the clock.";

fn corpus() -> ScreenplayDoc {
    let mut doc = ScreenplayDoc::default();
    doc.title_page.set("Title", "రైలు - The Night Train");
    doc.title_page.set("Author", "నిషా వర్మ");
    let mut s1 = Scene::new("INT. రైల్వే స్టేషన్ - NIGHT");
    s1.elements.push(Element::new(ElementKind::Action, MIXED));
    s1.elements.push(Element::new(ElementKind::Character, "మీరా"));
    s1.elements
        .push(Element::new(ElementKind::Dialogue, TELUGU));
    s1.elements
        .push(Element::new(ElementKind::Character, "अर्जुन"));
    s1.elements
        .push(Element::new(ElementKind::Parenthetical, "(धीरे से)"));
    s1.elements.push(Element::new(ElementKind::Dialogue, HINDI));
    let mut s2 = Scene::new("EXT. சென்னை சாலை - DAY");
    s2.elements.push(Element::new(ElementKind::Action, TAMIL));
    s2.elements
        .push(Element::new(ElementKind::Action, MALAYALAM));
    s2.elements.push(Element::new(ElementKind::Character, "ರವಿ"));
    s2.elements
        .push(Element::new(ElementKind::Dialogue, KANNADA));
    s2.elements
        .push(Element::new(ElementKind::Transition, "CUT TO:"));
    doc.scenes = vec![s1, s2];
    doc
}

/// Every word of the corpus, in order.
fn words(doc: &ScreenplayDoc) -> Vec<String> {
    doc.scenes
        .iter()
        .flat_map(|s| {
            std::iter::once(s.heading.clone()).chain(s.elements.iter().map(|e| e.text.clone()))
        })
        .flat_map(|t| t.split_whitespace().map(str::to_string).collect::<Vec<_>>())
        .collect()
}

/// The words of `want` appear in `got` in the same order (case-insensitive for Latin
/// letters, since cues and headings print in capitals).
#[track_caller]
fn assert_keeps_every_word(format: &str, want: &[String], got: &[String]) {
    let got: Vec<String> = got.iter().map(|w| w.to_uppercase()).collect();
    let mut at = 0;
    for w in want {
        let w = w.to_uppercase();
        match got[at..].iter().position(|g| *g == w) {
            Some(i) => at += i + 1,
            None => panic!("{format}: lost or changed {w:?} (after word {at})\ngot: {got:?}"),
        }
    }
}

fn assert_has_every_script(format: &str, text: &str) {
    for (name, range) in [
        ("Telugu", '\u{0C00}'..='\u{0C7F}'),
        ("Devanagari", '\u{0900}'..='\u{097F}'),
        ("Tamil", '\u{0B80}'..='\u{0BFF}'),
        ("Malayalam", '\u{0D00}'..='\u{0D7F}'),
        ("Kannada", '\u{0C80}'..='\u{0CFF}'),
    ] {
        assert!(
            text.chars().any(|c| range.contains(&c)),
            "{format}: no {name}"
        );
    }
    assert!(
        text.contains('\u{200C}'),
        "{format}: zero-width non-joiner kept"
    );
    assert!(
        text.contains('\u{200D}'),
        "{format}: zero-width joiner kept"
    );
}

#[test]
fn every_format_keeps_every_character_of_a_mixed_indian_script() {
    let doc = corpus();
    let want = words(&doc);
    let opts = ExportOptions::default();
    for (format, source) in [
        (ExportFormat::Fountain, SourceFormat::Fountain),
        (ExportFormat::Fdx, SourceFormat::Fdx),
        (ExportFormat::Txt, SourceFormat::Txt),
        (ExportFormat::Pdf, SourceFormat::Pdf),
    ] {
        let (bytes, _, _) = render_bytes(&doc, format, &opts).unwrap();
        let back = import_bytes(&bytes, source)
            .unwrap_or_else(|e| panic!("{format:?} re-import: {e:?}"))
            .doc;
        let got = words(&back);
        assert_keeps_every_word(&format!("{format:?}"), &want, &got);
        assert_has_every_script(&format!("{format:?}"), &got.join(" "));
        assert_eq!(
            back.headed_scene_count(),
            doc.headed_scene_count(),
            "{format:?}"
        );
    }
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("multilingual.docx");
    export_to_path(&doc, ExportFormat::Docx, &opts, &dest).unwrap();
    let back = import_file(&dest, None).unwrap().doc;
    let got = words(&back);
    assert_keeps_every_word("Docx", &want, &got);
    assert_has_every_script("Docx", &got.join(" "));
}

#[test]
fn indian_script_runs_carry_word_font_and_language_hints_in_docx() {
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("hints.docx");
    export_to_path(
        &corpus(),
        ExportFormat::Docx,
        &ExportOptions::default(),
        &dest,
    )
    .unwrap();
    let mut z = zip::ZipArchive::new(std::fs::File::open(&dest).unwrap()).unwrap();
    let mut xml = String::new();
    std::io::Read::read_to_string(&mut z.by_name("word/document.xml").unwrap(), &mut xml).unwrap();
    for lang in ["te-IN", "hi-IN", "ta-IN", "ml-IN", "kn-IN"] {
        assert!(xml.contains(&format!("w:bidi=\"{lang}\"")), "{lang}");
    }
    assert!(xml.contains("w:cs=\"Nirmala UI\""));
    assert!(
        xml.contains(TELUGU) || xml.contains("మీరా రైలు"),
        "text stored as UTF-8"
    );
}

#[test]
fn the_multilingual_pdf_is_byte_identical_and_embeds_only_subsets() {
    let doc = corpus();
    let opts = ExportOptions::default();
    let (a, pages, _) = render_bytes(&doc, ExportFormat::Pdf, &opts).unwrap();
    let (b, _, _) = render_bytes(&doc, ExportFormat::Pdf, &opts).unwrap();
    assert_eq!(a, b, "same script and settings give the same bytes");
    assert!(pages.unwrap() >= 1);
    let raw = String::from_utf8_lossy(&a);
    for face in [
        "CourierPrime",
        "NotoSansTelugu",
        "NotoSansDevanagari",
        "NotoSansTamil",
        "NotoSansMalayalam",
        "NotoSansKannada",
    ] {
        assert!(raw.contains(face), "{face} embedded");
    }
    assert!(
        !raw.contains("CreationDate") && !raw.contains("ModDate"),
        "no clock in the file"
    );
    // Subsets: the whole PDF is far smaller than the eight bundled font files together.
    let bundled: usize = openframe_import_export::fonts::ALL_FACES
        .iter()
        .map(|f| f.data().len())
        .sum();
    assert!(a.len() < bundled / 3, "{} bytes", a.len());
}
