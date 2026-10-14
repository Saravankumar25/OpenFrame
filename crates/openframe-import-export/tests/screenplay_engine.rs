//! Screenplay engine: page setup (Letter and A4, font size, spacing), one page
//! layout shared by the preview and the PDF, keep-together rules, dialogue
//! continuation, header and footer, title page wrapping, explicit page breaks,
//! custom elements, Final Draft round trip of the new data, and screenplay
//! files without OpenFrame branding (contract doc 32 §2.2 to §2.4; Screenplay
//! and Production Depth Specification §8, §9, §13 to §15, §17, §43).

use std::io::Read;

use openframe_import_export::fdx::{self, FdxOptions};
use openframe_import_export::layout::{
    self, CHAR_WIDTH_IN, Layout, LayoutOptions, LineSpacing, LineStyle, PageSetup, Paper,
    RIGHT_EDGE_IN,
};
use openframe_import_export::{
    CustomStyle, Element, ElementKind, ExportFormat, ExportOptions, Scene, ScreenplayDoc,
    SourceFormat, fountain, import_bytes, pdf_export, render_bytes,
};

fn el(kind: ElementKind, text: &str) -> Element {
    Element::new(kind, text)
}

fn opts(page: PageSetup) -> LayoutOptions {
    LayoutOptions {
        title_page: false,
        page,
        ..Default::default()
    }
}

fn a4() -> PageSetup {
    PageSetup {
        paper: Paper::A4,
        ..Default::default()
    }
}

/// A feature-like script: headings, action, speeches with parentheticals,
/// transitions, a dual dialogue block and a page break.
fn script(scenes: usize) -> ScreenplayDoc {
    let mut doc = ScreenplayDoc::default();
    doc.title_page.set("Title", "Black Rain");
    doc.title_page.set("Author", "Nisha Verma");
    doc.title_page.set("Draft", "Draft 6");
    for i in 0..scenes {
        let mut s = Scene::new(format!("INT. LOCATION {i} - NIGHT"));
        s.elements.push(el(
            ElementKind::Action,
            &"Rain streaks the windows while the city hums below. ".repeat(1 + i % 4),
        ));
        s.elements.push(el(ElementKind::Character, "MEERA"));
        if i % 2 == 0 {
            s.elements.push(el(ElementKind::Parenthetical, "(quietly)"));
        }
        s.elements.push(el(
            ElementKind::Dialogue,
            &"You said you would never come back. ".repeat(1 + i % 6),
        ));
        if i % 7 == 3 {
            s.elements
                .push(el(ElementKind::Character, "ARJUN").dual(true));
            s.elements
                .push(el(ElementKind::Dialogue, "Now!").dual(true));
            s.elements
                .push(el(ElementKind::Character, "RAVI").dual(true));
            s.elements
                .push(el(ElementKind::Dialogue, "Wait!").dual(true));
        }
        if i % 5 == 4 {
            s.elements.push(el(ElementKind::Transition, "CUT TO:"));
        }
        if i == scenes / 2 {
            s.elements.push(Element::page_break());
        }
        doc.scenes.push(s);
    }
    doc
}

fn body_lines(lay: &Layout) -> impl Iterator<Item = (usize, &layout::PageLine)> {
    lay.pages
        .iter()
        .enumerate()
        .filter(|(_, p)| !p.title_page)
        .flat_map(|(i, p)| p.lines.iter().map(move |l| (i, l)))
}

// ------------------------------------------------------------------ page setup

#[test]
fn a4_keeps_columns_and_gains_lines_per_page() {
    let doc = script(60);
    let letter = layout::layout(&doc, &opts(PageSetup::default()));
    let a4l = layout::layout(&doc, &opts(a4()));
    assert_eq!(letter.geometry.lines_per_page, 55);
    assert_eq!(a4l.geometry.lines_per_page, 59);
    assert!((a4l.geometry.page_width_in - 8.27).abs() < 1e-3);
    assert!(
        body_lines(&a4l).any(|(_, l)| l.row >= 55 && l.row < 59),
        "A4 pages use the extra lines"
    );
    assert!(
        a4l.script_page_count() <= letter.script_page_count(),
        "A4 {} pages, Letter {} pages",
        a4l.script_page_count(),
        letter.script_page_count()
    );
    // Same columns: every heading, cue and dialogue line starts at the same x.
    for lay in [&letter, &a4l] {
        for (_, l) in body_lines(lay) {
            match l.style {
                LineStyle::Heading | LineStyle::Action => assert!((l.x_in - 1.5).abs() < 1e-4),
                // Standard cue, or the cue of the left or right dual-dialogue column.
                LineStyle::Character => assert!(
                    [3.7, 2.3, 5.4].iter().any(|x| (l.x_in - x).abs() < 1e-4),
                    "cue at {}",
                    l.x_in
                ),
                _ => {}
            }
            assert!(l.row < lay.geometry.lines_per_page as i32 + 4);
        }
    }
}

#[test]
fn font_size_and_spacing_change_the_page_grid() {
    let doc = script(20);
    let base = layout::layout(&doc, &opts(PageSetup::default()));
    let small = layout::layout(
        &doc,
        &opts(PageSetup {
            font_size: 10,
            ..Default::default()
        }),
    );
    let double = layout::layout(
        &doc,
        &opts(PageSetup {
            line_spacing: LineSpacing::Double,
            ..Default::default()
        }),
    );
    assert_eq!(small.geometry.chars(6.0), 72);
    assert!(small.script_page_count() < base.script_page_count());
    assert_eq!(double.geometry.lines_per_page, 27);
    assert!(double.script_page_count() > base.script_page_count());
}

#[test]
fn defaults_reproduce_the_standard_layout() {
    // The standard preset equals an explicit Letter, 12 pt, single, page 1 setup.
    let doc = script(25);
    let a = layout::layout(&doc, &opts(PageSetup::default()));
    let explicit = PageSetup {
        paper: Paper::Letter,
        font: "Courier".into(),
        font_size: 12,
        line_spacing: LineSpacing::Single,
        first_page_number: 1,
        header: String::new(),
        footer: String::new(),
        header_on_first_page: false,
    };
    assert_eq!(a, layout::layout(&doc, &opts(explicit)));
}

// ------------------------------------------------------- preview equals PDF

/// One text run of a PDF: x pt, y pt, text.
type Run = (f32, f32, String);

/// Every text run of a PDF page by page, and the page sizes.
fn pdf_runs(bytes: &[u8]) -> (Vec<Vec<Run>>, Vec<(f32, f32)>) {
    let doc = lopdf::Document::load_mem(bytes).expect("valid PDF");
    let mut pages = Vec::new();
    let mut sizes = Vec::new();
    for (_, id) in doc.get_pages() {
        let page = doc.get_dictionary(id).unwrap();
        let mb = page.get(b"MediaBox").unwrap().as_array().unwrap();
        let num = |o: &lopdf::Object| match o {
            lopdf::Object::Integer(i) => *i as f32,
            lopdf::Object::Real(r) => *r,
            _ => 0.0,
        };
        sizes.push((num(&mb[2]), num(&mb[3])));
        let content = doc.get_and_decode_page_content(id).unwrap();
        let mut runs = Vec::new();
        let mut at = (0.0, 0.0);
        for op in content.operations {
            match op.operator.as_str() {
                "Tm" => at = (num(&op.operands[4]), num(&op.operands[5])),
                "Tj" => {
                    let s = op.operands[0].as_str().unwrap();
                    let text: String = s.iter().map(|b| *b as char).collect();
                    runs.push((at.0, at.1, text));
                }
                _ => {}
            }
        }
        pages.push(runs);
    }
    (pages, sizes)
}

fn assert_preview_matches_pdf(doc: &ScreenplayDoc, page: PageSetup) {
    let o = ExportOptions {
        scene_numbers: true,
        page,
        ..Default::default()
    };
    let lay = layout::layout(doc, &o.layout());
    let (bytes, pages, _) = render_bytes(doc, ExportFormat::Pdf, &o).unwrap();
    assert_eq!(pages, Some(lay.script_page_count()));
    let (runs, sizes) = pdf_runs(&bytes);
    assert_eq!(runs.len(), lay.pages.len(), "same page count");
    for ((page, runs), size) in lay.pages.iter().zip(&runs).zip(&sizes) {
        assert!((size.0 - lay.geometry.page_width_in * 72.0).abs() < 0.5);
        assert!((size.1 - lay.geometry.page_height_in * 72.0).abs() < 0.5);
        let expected: Vec<(f32, f32, &str)> = page
            .lines
            .iter()
            .filter(|l| !l.text.is_empty())
            .map(|l| {
                (
                    l.x_in * 72.0,
                    (lay.geometry.page_height_in - lay.line_y_in(l)) * 72.0,
                    l.text.as_str(),
                )
            })
            .collect();
        assert_eq!(expected.len(), runs.len(), "same lines on the page");
        for ((x, y, t), (px, py, pt)) in expected.iter().zip(runs) {
            assert!((x - px).abs() < 0.05 && (y - py).abs() < 0.05, "{t}");
            assert_eq!(*t, pt);
        }
    }
}

#[test]
fn print_preview_and_pdf_agree_on_letter() {
    assert_preview_matches_pdf(&script(40), PageSetup::default());
}

#[test]
fn print_preview_and_pdf_agree_on_a4_with_header_and_footer() {
    assert_preview_matches_pdf(
        &script(40),
        PageSetup {
            paper: Paper::A4,
            header: "{title} {draft}".into(),
            footer: "Page {page}".into(),
            first_page_number: 3,
            ..Default::default()
        },
    );
}

#[test]
fn layout_is_deterministic() {
    let doc = script(30);
    let o = ExportOptions {
        page: a4(),
        ..Default::default()
    };
    let a = render_bytes(&doc, ExportFormat::Pdf, &o).unwrap().0;
    let b = render_bytes(&doc, ExportFormat::Pdf, &o).unwrap().0;
    assert_eq!(a, b);
}

// --------------------------------------------------------------- keep-together

fn filler(lines: usize) -> Element {
    el(
        ElementKind::Action,
        &(0..lines).map(|_| "line").collect::<Vec<_>>().join("\n"),
    )
}

fn page_of(lay: &Layout, text: &str) -> usize {
    lay.pages
        .iter()
        .position(|p| p.lines.iter().any(|l| l.text == text))
        .unwrap_or_else(|| panic!("{text} not laid out"))
}

#[test]
fn heading_stays_with_a_speech_cue_and_two_lines() {
    let mut doc = ScreenplayDoc::default();
    let mut s = Scene::new("INT. A - DAY");
    s.elements.push(filler(49));
    doc.scenes.push(s);
    let mut s2 = Scene::new("INT. B - DAY");
    s2.elements.push(el(ElementKind::Character, "MEERA"));
    s2.elements
        .push(el(ElementKind::Dialogue, "One.\nTwo.\nThree.\nFour."));
    doc.scenes.push(s2);
    let lay = layout::layout(&doc, &opts(PageSetup::default()));
    // 49 + blank + heading + blank + cue + 2 lines = 55 would fit; with less room it moves.
    assert_eq!(page_of(&lay, "INT. B - DAY"), page_of(&lay, "MEERA"));
    assert_eq!(page_of(&lay, "MEERA"), page_of(&lay, "One."));
}

#[test]
fn character_cue_is_never_left_alone() {
    let mut doc = ScreenplayDoc::default();
    let mut s = Scene::new("INT. A - DAY");
    s.elements.push(filler(51));
    s.elements.push(el(ElementKind::Character, "MEERA"));
    s.elements
        .push(el(ElementKind::Dialogue, "Short line one and more words."));
    doc.scenes.push(s);
    let lay = layout::layout(&doc, &opts(PageSetup::default()));
    assert_eq!(
        page_of(&lay, "MEERA"),
        page_of(&lay, "Short line one and more words.")
    );
    assert!(
        !lay.pages
            .iter()
            .flat_map(|p| &p.lines)
            .any(|l| l.style == LineStyle::More),
        "a two-line speech is moved whole, never split"
    );
}

#[test]
fn parenthetical_stays_with_the_dialogue_it_introduces() {
    for fill in 40..54 {
        let mut doc = ScreenplayDoc::default();
        let mut s = Scene::new("INT. A - DAY");
        s.elements.push(filler(fill));
        s.elements.push(el(ElementKind::Character, "MEERA"));
        s.elements
            .push(el(ElementKind::Dialogue, "First.\nSecond.\nThird."));
        s.elements.push(el(ElementKind::Parenthetical, "(beat)"));
        s.elements
            .push(el(ElementKind::Dialogue, "Fourth.\nFifth.\nSixth."));
        doc.scenes.push(s);
        let lay = layout::layout(&doc, &opts(PageSetup::default()));
        assert_eq!(
            page_of(&lay, "(beat)"),
            page_of(&lay, "Fourth."),
            "fill {fill}"
        );
        for p in &lay.pages {
            // The last body line of a page is never a parenthetical or a lone cue.
            if let Some(last) = p
                .lines
                .iter()
                .filter(|l| l.row >= 0 && l.style != LineStyle::More)
                .max_by_key(|l| l.row)
            {
                assert_ne!(last.style, LineStyle::Parenthetical, "fill {fill}");
                assert_ne!(last.style, LineStyle::Character, "fill {fill}");
            }
        }
    }
}

#[test]
fn split_paragraphs_leave_at_least_two_lines_on_each_page() {
    for fill in 45..54 {
        let mut doc = ScreenplayDoc::default();
        let mut s = Scene::new("INT. A - DAY");
        s.elements.push(filler(fill));
        s.elements.push(el(
            ElementKind::Action,
            &(0..6)
                .map(|i| format!("para{i}"))
                .collect::<Vec<_>>()
                .join("\n"),
        ));
        doc.scenes.push(s);
        let lay = layout::layout(&doc, &opts(PageSetup::default()));
        let pages: Vec<usize> = (0..6).map(|i| page_of(&lay, &format!("para{i}"))).collect();
        for p in pages
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
        {
            let n = pages.iter().filter(|x| **x == p).count();
            assert!(
                n >= 2,
                "fill {fill}: {n} line(s) of the paragraph alone on a page"
            );
        }
    }
}

// ------------------------------------------------------- dialogue continuation

#[test]
fn one_split_speech_differs_from_two_speeches() {
    // One long speech split by the page boundary: (MORE) and CONT'D.
    let mut one = ScreenplayDoc::default();
    let mut s = Scene::new("INT. HALL - NIGHT");
    s.elements.push(filler(44));
    s.elements.push(el(ElementKind::Character, "MEERA (V.O.)"));
    s.elements
        .push(el(ElementKind::Dialogue, &"Talk. ".repeat(80)));
    one.scenes.push(s);
    let lay = layout::layout(&one, &opts(PageSetup::default()));
    let all: Vec<&layout::PageLine> = lay.pages.iter().flat_map(|p| &p.lines).collect();
    assert_eq!(all.iter().filter(|l| l.style == LineStyle::More).count(), 1);
    assert!(all.iter().any(|l| l.text == "MEERA (V.O.) (CONT'D)"));

    // Two speeches by the same character across the boundary: no continuation.
    let mut two = ScreenplayDoc::default();
    let mut s = Scene::new("INT. HALL - NIGHT");
    s.elements.push(filler(47));
    s.elements.push(el(ElementKind::Character, "MEERA"));
    s.elements
        .push(el(ElementKind::Dialogue, "First speech here."));
    s.elements.push(el(ElementKind::Action, "She turns."));
    s.elements.push(el(ElementKind::Character, "MEERA"));
    s.elements
        .push(el(ElementKind::Dialogue, &"Second speech. ".repeat(10)));
    two.scenes.push(s);
    let lay = layout::layout(&two, &opts(PageSetup::default()));
    assert!(lay.script_page_count() >= 2);
    let all: Vec<&layout::PageLine> = lay.pages.iter().flat_map(|p| &p.lines).collect();
    assert!(!all.iter().any(|l| l.style == LineStyle::More));
    assert!(!all.iter().any(|l| l.text.contains("CONT'D")));
}

// ------------------------------------------------------------- page breaks

#[test]
fn explicit_page_break_starts_a_new_page_and_never_a_blank_one() {
    let mut doc = ScreenplayDoc::default();
    let mut s = Scene::new("INT. A - DAY");
    s.elements.push(el(ElementKind::Action, "Before."));
    s.elements.push(Element::page_break());
    s.elements.push(Element::page_break());
    s.elements.push(el(ElementKind::Action, "After."));
    doc.scenes.push(s);
    let lay = layout::layout(&doc, &opts(PageSetup::default()));
    assert_eq!(lay.script_page_count(), 2);
    assert_eq!(page_of(&lay, "Before."), 0);
    assert_eq!(page_of(&lay, "After."), 1);
    // The scene spans both pages.
    assert_eq!(lay.scenes.len(), 1);
    assert_eq!((lay.scenes[0].first_page, lay.scenes[0].last_page), (1, 2));
}

#[test]
fn page_breaks_round_trip_through_fdx_and_fountain() {
    let mut doc = ScreenplayDoc::default();
    let mut s = Scene::new("INT. A - DAY");
    s.elements.push(el(ElementKind::Action, "Before."));
    s.elements.push(Element::page_break());
    s.elements.push(el(ElementKind::Action, "After."));
    doc.scenes.push(s);
    let mut s2 = Scene::new("INT. B - DAY");
    s2.elements.push(el(ElementKind::Action, "Then."));
    s2.elements.push(Element::page_break());
    doc.scenes.push(s2);
    let mut s3 = Scene::new("INT. C - DAY");
    s3.elements.push(el(ElementKind::Action, "End."));
    doc.scenes.push(s3);

    let xml = fdx::write(&doc, &FdxOptions::default());
    assert_eq!(xml.matches("StartsNewPage=\"Yes\"").count(), 2);
    let back = fdx::parse(xml.as_bytes()).unwrap().doc;
    let kinds = |d: &ScreenplayDoc| -> Vec<Vec<ElementKind>> {
        d.scenes
            .iter()
            .map(|s| s.elements.iter().map(|e| e.kind).collect())
            .collect()
    };
    assert_eq!(kinds(&back), kinds(&doc));

    let f = fountain::write(&doc, false, false);
    assert_eq!(f.matches("\n===\n").count(), 2);
    let back = import_bytes(f.as_bytes(), SourceFormat::Fountain)
        .unwrap()
        .doc;
    let breaks = back
        .scenes
        .iter()
        .flat_map(|s| &s.elements)
        .filter(|e| e.kind == ElementKind::PageBreak)
        .count();
    assert_eq!(breaks, 2);
}

// ---------------------------------------------------------- header and footer

#[test]
fn header_footer_placeholders_and_first_page_rule() {
    let mut doc = script(30);
    doc.title_page.set("Revision", "Blue Revision");
    doc.title_page.set("Date", "4 Oct 2026");
    let setup = PageSetup {
        header: "{title} | {draft} | {revision}".into(),
        footer: "{date} | Page {page}".into(),
        first_page_number: 5,
        ..Default::default()
    };
    let lay = layout::layout(&doc, &opts(setup.clone()));
    let first = &lay.pages[0];
    assert_eq!(first.number, Some(5));
    assert!(
        !first.lines.iter().any(|l| matches!(
            l.style,
            LineStyle::Header | LineStyle::Footer | LineStyle::PageNumber
        )),
        "nothing on the first page by default"
    );
    let second = &lay.pages[1];
    assert_eq!(second.number, Some(6));
    let header = second
        .lines
        .iter()
        .find(|l| l.style == LineStyle::Header)
        .unwrap();
    assert_eq!(header.text, "Black Rain | Draft 6 | Blue Revision");
    assert!(header.row < 0);
    let footer = second
        .lines
        .iter()
        .find(|l| l.style == LineStyle::Footer)
        .unwrap();
    assert_eq!(footer.text, "4 Oct 2026 | Page 6");
    assert!(footer.row >= lay.geometry.lines_per_page as i32);
    assert!(lay.line_y_in(footer) < lay.geometry.page_height_in - 0.3);
    let pn = second
        .lines
        .iter()
        .find(|l| l.style == LineStyle::PageNumber)
        .unwrap();
    assert_eq!(pn.text, "6.");

    let on_first = layout::layout(
        &doc,
        &opts(PageSetup {
            header_on_first_page: true,
            ..setup
        }),
    );
    assert!(
        on_first.pages[0]
            .lines
            .iter()
            .any(|l| l.style == LineStyle::Header)
    );
    assert!(on_first.pages[0].lines.iter().any(|l| l.text == "5."));
}

#[test]
fn a_long_header_never_runs_into_the_page_number() {
    let doc = script(10);
    let lay = layout::layout(
        &doc,
        &opts(PageSetup {
            header: "X".repeat(120),
            ..Default::default()
        }),
    );
    let p = &lay.pages[1];
    let h = p
        .lines
        .iter()
        .find(|l| l.style == LineStyle::Header)
        .unwrap();
    let n = p
        .lines
        .iter()
        .find(|l| l.style == LineStyle::PageNumber)
        .unwrap();
    assert!(h.x_in + h.text.chars().count() as f32 * CHAR_WIDTH_IN < n.x_in);
}

// ---------------------------------------------------------------- title page

#[test]
fn long_title_page_lines_are_wrapped_inside_the_page() {
    let mut doc = script(2);
    doc.title_page.set(
        "Draft",
        "Shooting Draft for the second unit, revised after the location scout in Hyderabad",
    );
    doc.title_page.set(
        "Revision",
        "Blue Revision, pages changed after the table read and the producer notes",
    );
    doc.title_page.set(
        "Contact",
        "Nisha Verma, Flat 4, Long Street Apartments, Banjara Hills, Hyderabad 500034, +91 98765 43210, nisha@example.com",
    );
    doc.title_page.set(
        "Representation",
        "Represented by an agency with a very long name indeed",
    );
    for setup in [PageSetup::default(), a4()] {
        let lay = layout::layout(
            &doc,
            &LayoutOptions {
                page: setup,
                ..Default::default()
            },
        );
        let tp = &lay.pages[0];
        assert!(tp.title_page);
        let w = lay.geometry.page_width_in;
        for l in &tp.lines {
            let end = l.x_in + l.text.chars().count() as f32 * lay.geometry.char_width_in;
            assert!(l.x_in >= 1.0 - 1e-4, "{} starts at {}", l.text, l.x_in);
            assert!(
                end <= RIGHT_EDGE_IN + 1e-3 && end < w,
                "{} ends at {end}",
                l.text
            );
            assert!(
                l.row < lay.geometry.lines_per_page as i32,
                "{} below the body",
                l.text
            );
        }
        // Every word of the long right-hand lines is still there.
        let right: String = tp
            .lines
            .iter()
            .filter(|l| l.x_in > 4.0)
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(right.contains("Hyderabad") && right.contains("producer notes"));
        // Left and right columns never overlap on a row.
        for a in tp.lines.iter().filter(|l| l.x_in < 2.0 && l.row > 30) {
            let a_end = a.x_in + a.text.chars().count() as f32 * CHAR_WIDTH_IN;
            for b in tp
                .lines
                .iter()
                .filter(|b| b.row == a.row && b.x_in > a.x_in)
            {
                assert!(a_end <= b.x_in + 1e-4, "{} overlaps {}", a.text, b.text);
            }
        }
    }
}

// ------------------------------------------------------------ custom elements

fn telephone() -> CustomStyle {
    CustomStyle {
        name: "Telephone".into(),
        export_name: "TELEPHONE".into(),
        look: "dialogue".into(),
        uppercase: true,
        italic: true,
        space_before: 2,
    }
}

#[test]
fn custom_elements_use_their_look_case_and_spacing() {
    let mut doc = ScreenplayDoc::default();
    let mut s = Scene::new("INT. A - DAY");
    s.elements.push(el(ElementKind::Action, "The phone rings."));
    s.elements
        .push(Element::custom("hello? who is this?", telephone()));
    doc.scenes.push(s);
    let lay = layout::layout(&doc, &opts(PageSetup::default()));
    let line = lay.pages[0]
        .lines
        .iter()
        .find(|l| l.text == "HELLO? WHO IS THIS?")
        .unwrap();
    assert_eq!(line.style, LineStyle::CustomItalic);
    assert!((line.x_in - 2.5).abs() < 1e-4, "dialogue column");
    let action = lay.pages[0]
        .lines
        .iter()
        .find(|l| l.text == "The phone rings.")
        .unwrap();
    assert_eq!(line.row - action.row, 3, "two blank lines before");
}

#[test]
fn custom_elements_round_trip_through_fdx_with_their_identity_and_look() {
    let mut doc = ScreenplayDoc::default();
    let mut s = Scene::new("INT. A - DAY");
    s.number = Some("12A".into());
    s.elements.push(Element::custom("Hello?", telephone()));
    let mut revised = el(ElementKind::Action, "A revised line.");
    revised.revision = Some("Blue".into());
    s.elements.push(revised);
    doc.scenes.push(s);
    let xml = fdx::write(
        &doc,
        &FdxOptions {
            revision_marks: true,
            ..Default::default()
        },
    );
    assert!(xml.contains("<Paragraph Type=\"TELEPHONE\">"));
    assert!(xml.contains("<ElementSettings Type=\"TELEPHONE\">"));
    assert!(xml.contains("Number=\"12A\""));
    let back = fdx::parse(xml.as_bytes()).unwrap().doc;
    let s = &back.scenes[0];
    assert_eq!(s.number.as_deref(), Some("12A"));
    assert_eq!(s.elements[0].kind, ElementKind::Custom);
    let c = s.elements[0].custom.as_ref().unwrap();
    assert_eq!(c.export_name, "TELEPHONE");
    assert_eq!(c.look, "dialogue");
    assert!(c.uppercase && c.italic);
    assert_eq!(c.space_before, 2);
    assert_eq!(s.elements[1].revision.as_deref(), Some("Blue"));
    assert!(back.warnings.iter().any(|w| w.code == "fdx_custom_element"));
}

#[test]
fn unknown_fdx_paragraph_types_are_never_flattened_into_action() {
    let src = r#"<?xml version="1.0" encoding="UTF-8"?>
<FinalDraft DocumentType="Script" Template="No" Version="5">
<Content>
<Paragraph Type="Scene Heading"><Text>INT. HOUSE - DAY</Text></Paragraph>
<Paragraph Type="Radio"><Text>This is the late news.</Text></Paragraph>
<Paragraph Type="Lower Third" Alignment="Center"><Text>HYDERABAD, 1994</Text></Paragraph>
</Content>
<ElementSettings Type="Radio"><FontSpec Style="Italic"/><ParagraphSpec Alignment="Left" LeftIndent="2.50" SpaceBefore="12"/></ElementSettings>
</FinalDraft>"#;
    let d = fdx::parse(src.as_bytes()).unwrap().doc;
    let els = &d.scenes[0].elements;
    assert!(els.iter().all(|e| e.kind == ElementKind::Custom));
    let radio = els[0].custom.as_ref().unwrap();
    assert_eq!((radio.look.as_str(), radio.italic), ("dialogue", true));
    assert_eq!(els[1].custom.as_ref().unwrap().look, "centered");
}

// ---------------------------------------------------------------- branding

#[test]
fn screenplay_files_carry_no_openframe_branding() {
    let mut doc = script(6);
    doc.scenes[0]
        .elements
        .push(Element::custom("Hello?", telephone()));
    let o = ExportOptions {
        scene_numbers: true,
        revision_marks: true,
        ..Default::default()
    };
    for format in [
        ExportFormat::Pdf,
        ExportFormat::Fdx,
        ExportFormat::Fountain,
        ExportFormat::Txt,
        ExportFormat::Docx,
    ] {
        let (bytes, _, _) = render_bytes(&doc, format, &o).unwrap();
        let mut texts: Vec<String> = Vec::new();
        if format == ExportFormat::Docx {
            let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
            for i in 0..zip.len() {
                let mut f = zip.by_index(i).unwrap();
                let mut s = String::new();
                f.read_to_string(&mut s).unwrap();
                texts.push(s);
            }
        } else {
            texts.push(String::from_utf8_lossy(&bytes).into_owned());
        }
        for t in texts {
            assert!(
                !t.to_lowercase().contains("openframe"),
                "{format:?} names OpenFrame"
            );
        }
    }
    // The shared PDF writer still brands production documents.
    let pdf = pdf_export::write(&doc, &LayoutOptions::default()).bytes;
    assert!(!String::from_utf8_lossy(&pdf).contains("OpenFrame"));
}

#[test]
fn wrapped_transitions_stay_right_aligned_on_a4() {
    let mut doc = ScreenplayDoc::default();
    let mut s = Scene::new("INT. A - DAY");
    s.elements.push(el(
        ElementKind::Transition,
        "SLOW DISSOLVE THROUGH THE RAIN AND THE NEON AND THE LONG NIGHT TO:",
    ));
    doc.scenes.push(s);
    let lay = layout::layout(&doc, &opts(a4()));
    let t: Vec<&layout::PageLine> = lay.pages[0]
        .lines
        .iter()
        .filter(|l| l.style == LineStyle::Transition)
        .collect();
    assert_eq!(t.len(), 2);
    for l in t {
        let end = l.x_in + l.text.chars().count() as f32 * CHAR_WIDTH_IN;
        assert!((end - RIGHT_EDGE_IN).abs() < 1e-4);
    }
}
