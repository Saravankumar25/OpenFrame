//! Golden PDF image-compare regression for screenplay PDF export
//! (build plan item PDF image comparison, feature XFER-010; ADR-0008 deterministic PDF).
//!
//! Each case exports a synthetic screenplay through the real export path,
//! renders every page with the Windows built-in PDF renderer and compares it
//! with the committed golden PNG in `tests/fixtures/pdf-golden/`. See
//! `pdf_golden_support/mod.rs` for the steps and `pdf_golden_support/compare.rs`
//! for the tolerance.
//!
//! Regenerate goldens deliberately (then review every PNG before committing):
//!
//! ```text
//! OPENFRAME_UPDATE_GOLDEN=1 cargo test -p openframe-import-export --test pdf_golden
//! ```
//!
//! Windows only: the renderer is `Windows.Data.Pdf`, and goldens are tied to
//! it. Paper sizes: US Letter for most cases and A4 for `a4_page_setup` (page setup with
//! header, footer, first page number, a custom element and an explicit page break).

#![cfg(windows)]

mod pdf_golden_support;

use openframe_import_export::layout::{self, LineStyle};
use openframe_import_export::{Element, ElementKind, ExportOptions, Scene, ScreenplayDoc};
use pdf_golden_support::compare::{self, TOLERANCE};
use pdf_golden_support::{
    Case, LETTER_PT, TEXT_AREA, WITH_MARGIN_MARKS, clipping_violations, export_pdf,
    golden_mismatches, render, run_case,
};

fn el(kind: ElementKind, text: &str) -> Element {
    Element::new(kind, text)
}

fn revised(kind: ElementKind, text: &str) -> Element {
    let mut e = Element::new(kind, text);
    e.revision = Some("blue".into());
    e
}

fn speech(scene: &mut Scene, cue: &str, paren: Option<&str>, line: &str) {
    scene.elements.push(el(ElementKind::Character, cue));
    if let Some(p) = paren {
        scene.elements.push(el(ElementKind::Parenthetical, p));
    }
    scene.elements.push(el(ElementKind::Dialogue, line));
}

// ------------------------------------------------------------------ corpus

/// Title page, scene numbers (including a lettered one), headings, action,
/// dialogue with parentheticals, a voice-over cue, transition, shot,
/// centered text, dual dialogue, a lyric, a note (oblique), WinAnsi accents,
/// and the page-number header on page 2.
fn title_page_and_elements() -> Case {
    let mut doc = ScreenplayDoc::default();
    doc.title_page.set("Title", "The Night Train");
    doc.title_page.set("Credit", "Written by");
    doc.title_page.set("Author", "Asha Rao");
    doc.title_page.set(
        "Source",
        "Based on the short story \"Last Stop\" by K. Menon",
    );
    doc.title_page.set(
        "Contact",
        "Northlight Pictures\n14 Example Road\nPune 411001\nscripts@example.com",
    );
    doc.title_page.set("Draft date", "5 October 2026");

    let mut s1 = Scene::new("EXT. RAILWAY PLATFORM - NIGHT");
    s1.elements.push(el(
        ElementKind::Action,
        "Rain hammers the tin roof. A single lamp swings above an empty bench. MEERA (30s, soaked, \
         determined) checks the timetable twice, then the clock.",
    ));
    speech(
        &mut s1,
        "MEERA",
        Some("(under her breath)"),
        "Eleven forty. It always comes at eleven forty.",
    );
    speech(
        &mut s1,
        "ARJUN (O.S.)",
        None,
        "Not tonight. They cancelled it an hour ago.",
    );
    s1.elements.push(el(
        ElementKind::Action,
        "She turns. ARJUN (40s) steps out of the shadow of the ticket office.",
    ));
    s1.elements.push(el(ElementKind::Transition, "CUT TO:"));

    let mut s2 = Scene::new("INT. STATION CAFÉ - CONTINUOUS");
    s2.number = Some("2A".into());
    s2.elements.push(el(
        ElementKind::Action,
        "A narrow café with three stools. The owner, déjà vu written on his face, pours two cups \
         without being asked.",
    ));
    s2.elements.push(el(ElementKind::Shot, "close on the cups"));
    s2.elements
        .push(el(ElementKind::Centered, "TWENTY MINUTES LATER"));
    s2.elements
        .push(el(ElementKind::Character, "MEERA").dual(true));
    s2.elements.push(
        el(
            ElementKind::Dialogue,
            "You knew it was cancelled and you let me wait.",
        )
        .dual(true),
    );
    s2.elements
        .push(el(ElementKind::Character, "ARJUN").dual(true));
    s2.elements.push(
        el(
            ElementKind::Dialogue,
            "I wanted to see if you would still come.",
        )
        .dual(true),
    );
    s2.elements.push(el(
        ElementKind::Note,
        "Check the cafe name with the location team.",
    ));
    s2.elements.push(el(ElementKind::Character, "OWNER"));
    s2.elements.push(el(
        ElementKind::Lyric,
        "Last train home, last light on the line",
    ));

    let mut doc_scenes = vec![s1, s2];
    for (i, (heading, cue, line)) in [
        (
            "EXT. LEVEL CROSSING - NIGHT",
            "MEERA",
            "If the barrier is down, the train is still coming.",
        ),
        (
            "INT. SIGNAL BOX - NIGHT",
            "SIGNALMAN",
            "Nothing has passed here since nine. Nothing is due till morning.",
        ),
        ("EXT. TRACKS - NIGHT", "ARJUN", "Then what is that light?"),
        (
            "INT. TRAIN CARRIAGE - MOVING - NIGHT",
            "CONDUCTOR",
            "Tickets, please. Both of you. This one does not stop until the end of the line.",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let mut s = Scene::new(heading);
        s.elements.push(el(
            ElementKind::Action,
            &format!(
                "Beat {}. The wind drops. Somewhere far off, a whistle answers the storm, \
                 long and low, and the rails begin to sing.",
                i + 1
            ),
        ));
        speech(&mut s, cue, (i % 2 == 0).then_some("(quietly)"), line);
        s.elements.push(el(
            ElementKind::Action,
            "Nobody moves. The lamp stops swinging.",
        ));
        doc_scenes.push(s);
    }
    doc.scenes = doc_scenes;

    Case {
        name: "title_page_and_elements",
        doc,
        options: ExportOptions {
            scene_numbers: true,
            include_notes: true,
            ..Default::default()
        },
        paper_pt: LETTER_PT,
        pdf_pages: 3,
        script_pages: 2,
        margins: WITH_MARGIN_MARKS,
    }
}

/// A speech that cannot fit at the foot of page 1 splits with `(MORE)` and
/// `NAME (CONT'D)`; a long action block continues over a page break; page
/// numbers "2." and "3." are the page header. No title page.
fn dialogue_continuation() -> Case {
    let mut doc = ScreenplayDoc::default();
    let mut s1 = Scene::new("INT. INTERROGATION ROOM - NIGHT");
    let filler: Vec<String> = (1..=40)
        .map(|n| format!("Line {n}: a fan turns, a pen taps, the tape spools on."))
        .collect();
    s1.elements
        .push(el(ElementKind::Action, &filler.join("\n")));
    s1.elements
        .push(el(ElementKind::Character, "INSPECTOR DAS"));
    s1.elements
        .push(el(ElementKind::Parenthetical, "(calm, almost kind)"));
    s1.elements.push(el(
        ElementKind::Dialogue,
        &"You will tell me where you were on the night of the ninth, and you will tell me \
          slowly, because I have all night and the tea here is terrible. "
            .repeat(5),
    ));
    let mut s2 = Scene::new("INT. CORRIDOR OUTSIDE - CONTINUOUS");
    let long_action: Vec<String> = (1..=70)
        .map(|n| format!("Corridor beat {n}: footsteps, a door, a cough, the lights."))
        .collect();
    s2.elements
        .push(el(ElementKind::Action, &long_action.join("\n")));
    speech(&mut s2, "CONSTABLE", None, "Sir. The lawyer is here.");
    doc.scenes = vec![s1, s2];

    Case {
        name: "dialogue_continuation",
        doc,
        options: ExportOptions {
            title_page: false,
            ..Default::default()
        },
        paper_pt: LETTER_PT,
        pdf_pages: 3,
        script_pages: 3,
        margins: TEXT_AREA,
    }
}

/// Revision asterisks in the right margin beside revised action, dialogue and
/// transition lines; revision line on the title page; scene numbers share the
/// right margin with the marks.
fn revision_marks() -> Case {
    let mut doc = ScreenplayDoc::default();
    doc.title_page.set("Title", "The Night Train");
    doc.title_page.set("Author", "Asha Rao");
    doc.title_page.set("Draft date", "5 October 2026");
    doc.title_page.set("Revision", "Blue Revision");

    let mut s1 = Scene::new("INT. TICKET OFFICE - NIGHT");
    s1.elements.push(el(
        ElementKind::Action,
        "An unrevised paragraph of action that stays exactly as it was in the white draft.",
    ));
    s1.elements.push(revised(
        ElementKind::Action,
        "A revised paragraph: the clerk now slides the ticket back under the glass, refusing \
         the money, and points at the clock.",
    ));
    s1.elements.push(el(ElementKind::Character, "CLERK"));
    s1.elements.push(revised(
        ElementKind::Dialogue,
        "Keep your money. Nobody pays for the last train.",
    ));
    s1.elements.push(el(ElementKind::Character, "MEERA"));
    s1.elements.push(el(
        ElementKind::Dialogue,
        "That line was not changed in this revision.",
    ));
    s1.elements
        .push(revised(ElementKind::Transition, "SMASH CUT TO:"));
    let mut s2 = Scene::new("EXT. PLATFORM - NIGHT");
    s2.elements.push(revised(
        ElementKind::Action,
        "Revised: the platform is empty now.",
    ));
    doc.scenes = vec![s1, s2];

    Case {
        name: "revision_marks",
        doc,
        options: ExportOptions {
            scene_numbers: true,
            revision_marks: true,
            ..Default::default()
        },
        paper_pt: LETTER_PT,
        pdf_pages: 2,
        script_pages: 1,
        margins: WITH_MARGIN_MARKS,
    }
}

const LONG_WORD: &str = "SUPERCALIFRAGILISTICEXPIALIDOCIOUSLYUNBROKENRAILWAYSIGNALLINGTELEGRAPHCODEWORD\
                         WITHNOSPACESATALLTHATMUSTWRAPANDNEVERRUNPASTTHERIGHTMARGINOFTHEPAGEEVER";

/// Words longer than every column (title, heading, action, cue, parenthetical,
/// dialogue, transition) are hard-broken inside their column; nothing runs
/// past the 7.5" right edge or off the page.
fn long_unbroken_word() -> Case {
    let mut doc = ScreenplayDoc::default();
    doc.title_page.set("Title", &LONG_WORD[..72]);
    doc.title_page.set("Author", "Asha Rao");
    doc.title_page.set(
        "Contact",
        "averyveryverylongemailaddresswithoutanybreaks@example.com",
    );

    let mut s = Scene::new(format!("INT. {}", &LONG_WORD[..70]));
    s.elements.push(el(ElementKind::Action, LONG_WORD));
    s.elements.push(el(
        ElementKind::Action,
        &format!("Short words first, then {LONG_WORD} and short words after."),
    ));
    s.elements
        .push(el(ElementKind::Character, &LONG_WORD[..50]));
    s.elements.push(el(
        ElementKind::Parenthetical,
        &format!("({})", &LONG_WORD[..40]),
    ));
    s.elements.push(el(ElementKind::Dialogue, &LONG_WORD[..90]));
    s.elements.push(el(
        ElementKind::Transition,
        &format!("{}:", &LONG_WORD[..64]),
    ));
    doc.scenes = vec![s];

    Case {
        name: "long_unbroken_word",
        doc,
        options: ExportOptions::default(),
        paper_pt: LETTER_PT,
        pdf_pages: 2,
        script_pages: 1,
        margins: TEXT_AREA,
    }
}

/// A4 page setup: header and footer with placeholders, page numbers from 3,
/// a custom element with its own look and an explicit page break.
fn a4_page_setup() -> Case {
    use openframe_import_export::{CustomStyle, PageSetup, Paper};
    let mut doc = ScreenplayDoc::default();
    doc.title_page.set("Title", "The Night Train");
    doc.title_page.set("Author", "Asha Rao");
    doc.title_page.set("Draft", "Shooting Draft");
    doc.title_page.set("Date", "4 October 2026");
    let mut s = Scene::new("INT. SIGNAL BOX - NIGHT");
    s.elements
        .push(el(ElementKind::Action, "The telephone rings twice."));
    s.elements.push(Element::custom(
        "this is control. clear the line.",
        CustomStyle {
            name: "Telephone".into(),
            export_name: "TELEPHONE".into(),
            look: "dialogue".into(),
            uppercase: true,
            italic: true,
            space_before: 1,
        },
    ));
    s.elements.push(Element::page_break());
    s.elements
        .push(el(ElementKind::Action, "Silence. The lamp swings."));
    doc.scenes = vec![s];
    Case {
        name: "a4_page_setup",
        doc,
        options: ExportOptions {
            page: PageSetup {
                paper: Paper::A4,
                header: "{title} - {draft}".into(),
                footer: "{date}".into(),
                first_page_number: 3,
                ..Default::default()
            },
            ..Default::default()
        },
        paper_pt: (8.27 * 72.0, 11.69 * 72.0),
        pdf_pages: 3,
        script_pages: 2,
        margins: pdf_golden_support::A4_WITH_FOOTER,
    }
}

/// Latin-only script with scene numbers off: Courier Prime is still embedded, scene headings are
/// bold, and the first script page carries the layout-only running title and the page number
/// "1." (change spec 01).
fn plain_headings_without_numbers() -> Case {
    let mut doc = ScreenplayDoc::default();
    doc.title_page.set("Title", "Monsoon Letters");
    doc.title_page.set("Author", "Asha Rao");
    let mut s1 = Scene::new("INT. SAVITRI'S KITCHEN - DAWN");
    s1.elements.push(el(
        ElementKind::Action,
        "Savitri wraps two rotis in newspaper and tucks a bundle of letters under the rice.",
    ));
    speech(
        &mut s1,
        "SAVITRI",
        Some("(to herself)"),
        "It is not kind in any month.",
    );
    let mut s2 = Scene::new("EXT. GUDUR BUS STAND - MORNING");
    s2.elements.push(el(
        ElementKind::Action,
        "A tin-roofed bus stand. Porters sleep on sacks.",
    ));
    s2.elements.push(el(ElementKind::Transition, "CUT TO:"));
    doc.scenes = vec![s1, s2];
    Case {
        name: "plain_headings_without_numbers",
        doc,
        options: ExportOptions::default(),
        paper_pt: LETTER_PT,
        pdf_pages: 2,
        script_pages: 1,
        margins: TEXT_AREA,
    }
}

/// ADR-0016: Telugu, Hindi, Tamil, Malayalam and Kannada shaped with the embedded fonts next to
/// Courier Prime: conjuncts, reordered vowel signs, a zero-width non-joiner and joiner, and
/// mixed English lines.
fn multilingual_scripts() -> Case {
    let mut doc = ScreenplayDoc::default();
    doc.title_page.set("Title", "రైలు - The Night Train");
    doc.title_page.set("Author", "నిషా వర్మ");
    let mut s1 = Scene::new("INT. రైల్వే స్టేషన్ - NIGHT");
    s1.elements.push(el(
        ElementKind::Action,
        "Rain on the platform. మీరా looks at अर्जुन, then at the clock.",
    ));
    speech(
        &mut s1,
        "మీరా",
        Some("(నిశ్శబ్దంగా)"),
        "మీరా రైలు స్టేషన్\u{200C}లో వేచి ఉంది. క్షమించండి!",
    );
    speech(
        &mut s1,
        "अर्जुन",
        Some("(धीरे से)"),
        "मीरा ने कहा, “क्षमा करें, ट्रेन देर से है।”",
    );
    let mut s2 = Scene::new("EXT. சென்னை சாலை - DAY");
    s2.elements.push(el(
        ElementKind::Action,
        "நீ எங்கே போகிறாய்? ஸ்ரீ வருகிறார். അവന്\u{200D} വന്നു. അവൻ പോയി.",
    ));
    speech(&mut s2, "ರವಿ", None, "ನಾನು ಬರುತ್ತೇನೆ, ಕ್ಷಮಿಸಿ.");
    s2.elements.push(el(ElementKind::Transition, "CUT TO:"));
    doc.scenes = vec![s1, s2];
    Case {
        name: "multilingual_scripts",
        doc,
        options: ExportOptions {
            scene_numbers: true,
            ..Default::default()
        },
        paper_pt: LETTER_PT,
        pdf_pages: 2,
        script_pages: 1,
        margins: WITH_MARGIN_MARKS,
    }
}

// ------------------------------------------------------------------ tests

#[test]
fn golden_latin_script_embeds_courier_prime_with_bold_headings_and_first_page_number() {
    let case = plain_headings_without_numbers();
    let lay = layout::layout(&case.doc, &case.options.layout());
    let script = lay.pages.last().expect("a script page");
    assert!(
        script
            .lines
            .iter()
            .any(|l| l.style == LineStyle::PageNumber && l.text == "1.")
    );
    assert!(
        script
            .lines
            .iter()
            .any(|l| l.style == LineStyle::PageTitle && l.text == "MONSOON LETTERS")
    );
    let (pdf, _) = export_pdf(&case.doc, &case.options);
    let text = String::from_utf8_lossy(&pdf);
    assert!(text.contains("CourierPrime-Bold") && text.contains("+CourierPrime"));
    assert!(
        !text.contains("/BaseFont/Courier") && !text.contains("/BaseFont /Courier"),
        "no unembedded base-14 Courier"
    );
    run_case(&case);
}

#[test]
fn golden_multilingual_scripts_shaped_with_embedded_fonts() {
    run_case(&multilingual_scripts());
}

#[test]
fn golden_a4_page_setup_header_footer_custom_element_page_break() {
    run_case(&a4_page_setup());
}

#[test]
fn m1_16_golden_title_page_scene_numbers_headings_dialogue() {
    run_case(&title_page_and_elements());
}

#[test]
fn m1_16_golden_dialogue_continuation_more_contd_and_page_headers() {
    let case = dialogue_continuation();
    // The corpus must really exercise continuation and headers.
    let lay = layout::layout(&case.doc, &case.options.layout());
    let all = || lay.pages.iter().flat_map(|p| p.lines.iter());
    assert!(all().any(|l| l.style == LineStyle::More && l.text == "(MORE)"));
    assert!(all().any(|l| l.text == "INSPECTOR DAS (CONT'D)"));
    let numbers: Vec<&str> = all()
        .filter(|l| l.style == LineStyle::PageNumber)
        .map(|l| l.text.as_str())
        .collect();
    assert_eq!(numbers, vec!["1.", "2.", "3."]);
    run_case(&case);
}

#[test]
fn m1_16_golden_revision_marks() {
    let case = revision_marks();
    let lay = layout::layout(&case.doc, &case.options.layout());
    let marks = lay
        .pages
        .iter()
        .flat_map(|p| p.lines.iter())
        .filter(|l| l.style == LineStyle::RevisionMark)
        .count();
    // 3 lines of revised action + 2 of revised dialogue + 1 transition + 1 action.
    assert_eq!(marks, 7, "revised lines carry an asterisk each");
    run_case(&case);
}

#[test]
fn m1_16_golden_long_unbroken_word_is_wrapped_not_clipped() {
    let case = long_unbroken_word();
    // Text level: the hard-broken pieces put back together are the whole word.
    let lay = layout::layout(&case.doc, &case.options.layout());
    let script = &lay.pages[1];
    let pieces: String = script
        .lines
        .iter()
        .filter(|l| l.style == LineStyle::Action)
        .take_while(|l| l.text.chars().all(|c| c.is_ascii_uppercase()))
        .map(|l| l.text.as_str())
        .collect();
    assert_eq!(pieces, LONG_WORD, "nothing of the long word is dropped");
    run_case(&case);
}

/// The tolerance is not so loose that a missing revision asterisk passes:
/// the revision case exported without marks must fail against its golden.
#[test]
fn m1_16_harness_detects_a_missing_revision_mark() {
    if pdf_golden_support::update_mode() {
        return;
    }
    let mut case = revision_marks();
    // Unmark only the one-line revised action in scene 2 (one asterisk).
    case.doc.scenes[1].elements[0].revision = None;
    let (pdf, _) = export_pdf(&case.doc, &case.options);
    let failures = golden_mismatches(case.name, &render(&pdf), &TOLERANCE, false);
    assert_eq!(
        failures.len(),
        1,
        "exactly the script page differs: {failures:?}"
    );
    assert!(failures[0].contains("page 2"), "{failures:?}");
}

/// One changed character in a page header ("2." becomes "3.") must fail.
#[test]
fn m1_16_harness_detects_a_one_character_change() {
    if pdf_golden_support::update_mode() {
        return;
    }
    let mut case = dialogue_continuation();
    // Same layout, one letter different in a mid-page action line.
    let action = &mut case.doc.scenes[0].elements[0].text;
    *action = action.replacen("Line 7: a fan", "Line 7: a fAn", 1);
    let (pdf, _) = export_pdf(&case.doc, &case.options);
    let failures = golden_mismatches(case.name, &render(&pdf), &TOLERANCE, false);
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert!(failures[0].contains("page 1"), "{failures:?}");
}

/// The tolerance absorbs a 1 px renderer shift but not a 2 px (0.01") one.
#[test]
fn m1_16_harness_tolerance_limits() {
    let case = dialogue_continuation();
    let (pdf, _) = export_pdf(&case.doc, &case.options);
    let page = &render(&pdf)[1].image;
    let same = compare::compare(page, page, &TOLERANCE);
    assert_eq!(same.differing, 0);
    let one = compare::compare(page, &compare::shifted(page, 1, 1), &TOLERANCE);
    assert!(one.passes(&TOLERANCE), "{}", one.summary(&TOLERANCE));
    let two = compare::compare(page, &compare::shifted(page, 2, 0), &TOLERANCE);
    assert!(!two.passes(&TOLERANCE), "{}", two.summary(&TOLERANCE));
}

/// Formerly a known defect (reported with PDF image comparison): title-page draft and
/// revision lines were right-aligned to 7.5" without wrapping and got clipped. Fixed in the
/// screenplay engine work: the right-hand block wraps at 26 characters.
#[test]
fn m1_16_long_title_page_draft_line_stays_inside_the_margins() {
    let mut doc = ScreenplayDoc::default();
    doc.title_page.set("Title", "The Night Train");
    doc.title_page.set(
        "Draft date",
        "Fifth revised shooting draft for the producers, the network and the insurance company",
    );
    let mut s = Scene::new("INT. ROOM - DAY");
    s.elements.push(el(ElementKind::Action, "Nothing happens."));
    doc.scenes.push(s);
    let (pdf, _) = export_pdf(&doc, &ExportOptions::default());
    let violations = clipping_violations(&render(&pdf), &TEXT_AREA);
    assert!(violations.is_empty(), "{violations:?}");
}
