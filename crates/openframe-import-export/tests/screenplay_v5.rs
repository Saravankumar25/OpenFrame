//! Screenplay formatting standard v5 (docs/specs/screenplay-formatting-standard-v5.md): the
//! relationship gaps, the pagination invariants at every page offset, the generated
//! (MORE) / (CONT'D) markers, first-page numbering, the title page blocks and the rule that no
//! source text is lost, reordered or duplicated by layout.
//!
//! Gaps are baseline to baseline: one body row is 12 pt at the standard geometry.

use openframe_import_export::layout::{
    self, CHAR_WIDTH_IN, LaidOutPage, Layout, LayoutOptions, LineStyle, PageLine, PageSetup, Paper,
};
use openframe_import_export::{Element, ElementKind as K, Scene, ScreenplayDoc};

const ROW_PT: f32 = 12.0;

fn el(kind: K, text: &str) -> Element {
    Element::new(kind, text)
}

fn scene(heading: &str, els: Vec<Element>) -> Scene {
    let mut s = Scene::new(heading);
    s.elements = els;
    s
}

fn doc(scenes: Vec<Scene>) -> ScreenplayDoc {
    ScreenplayDoc {
        scenes,
        ..Default::default()
    }
}

fn opts() -> LayoutOptions {
    LayoutOptions {
        title_page: false,
        ..Default::default()
    }
}

fn a4_opts() -> LayoutOptions {
    LayoutOptions {
        page: PageSetup {
            paper: Paper::A4,
            ..Default::default()
        },
        ..opts()
    }
}

/// `n` action lines, one per row (explicit line breaks).
fn rows_of(n: usize) -> String {
    (0..n)
        .map(|i| format!("line{i}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Styles that carry script text (everything else is page furniture or a generated marker).
fn is_script(style: LineStyle) -> bool {
    !matches!(
        style,
        LineStyle::More
            | LineStyle::PageNumber
            | LineStyle::SceneNumber
            | LineStyle::RevisionMark
            | LineStyle::Header
            | LineStyle::Footer
            | LineStyle::PageTitle
            | LineStyle::TitleMain
            | LineStyle::TitleText
    )
}

fn body(page: &LaidOutPage) -> Vec<&PageLine> {
    page.lines
        .iter()
        .filter(|l| l.row >= 0 && is_script(l.style))
        .collect()
}

/// (page index, row) of the first line with exactly this text.
fn at(lay: &Layout, text: &str) -> (usize, i32) {
    for (pi, p) in lay.pages.iter().enumerate() {
        if let Some(l) = p
            .lines
            .iter()
            .find(|l| l.text == text && is_script(l.style))
        {
            return (pi, l.row);
        }
    }
    panic!("no line {text:?}");
}

fn gap_pt(lay: &Layout, from: &str, to: &str) -> f32 {
    let (pa, ra) = at(lay, from);
    let (pb, rb) = at(lay, to);
    assert_eq!(pa, pb, "{from:?} and {to:?} are on different pages");
    (rb - ra) as f32 * ROW_PT
}

// ------------------------------------------------------------------ relationship gaps

#[test]
fn gaps_follow_the_v5_matrix() {
    let d = doc(vec![
        scene(
            "INT. A - DAY",
            vec![
                el(K::Action, "Alpha one."),
                el(K::Action, "Alpha two."),
                el(K::Character, "MEERA"),
                el(K::Dialogue, "Hello."),
                el(K::Character, "ARJUN"),
                el(K::Parenthetical, "(softly)"),
                el(K::Dialogue, "Hi."),
                el(K::Action, "Beta."),
                el(K::Transition, "CUT TO:"),
            ],
        ),
        scene("INT. B - DAY", vec![el(K::Action, "Gamma.")]),
        scene(
            "EXT. C - DAY",
            vec![el(K::Shot, "CLOSE ON DOOR"), el(K::Action, "Desc.")],
        ),
        scene("EXT. D - DAY", vec![el(K::Action, "Delta.")]),
    ]);
    let lay = layout::layout(&d, &opts());
    assert_eq!(lay.pages.len(), 1);
    let g = |a: &str, b: &str| gap_pt(&lay, a, b);
    assert_eq!(g("INT. A - DAY", "Alpha one."), 24.0, "heading to action");
    assert_eq!(g("Alpha one.", "Alpha two."), 24.0, "action to action");
    assert_eq!(g("Alpha two.", "MEERA"), 24.0, "action to cue");
    assert_eq!(g("MEERA", "Hello."), 12.0, "cue to dialogue");
    assert_eq!(g("Hello.", "ARJUN"), 24.0, "dialogue to next cue");
    assert_eq!(g("ARJUN", "(softly)"), 12.0, "cue to parenthetical");
    assert_eq!(g("(softly)", "Hi."), 12.0, "parenthetical to dialogue");
    assert_eq!(g("Hi.", "Beta."), 24.0, "dialogue to action");
    assert_eq!(g("Beta.", "CUT TO:"), 24.0, "action to transition");
    assert_eq!(g("CUT TO:", "INT. B - DAY"), 36.0, "transition to heading");
    assert_eq!(g("INT. B - DAY", "Gamma."), 24.0);
    assert_eq!(g("Gamma.", "EXT. C - DAY"), 36.0, "direct scene change");
    assert_eq!(g("EXT. C - DAY", "CLOSE ON DOOR"), 24.0);
    assert_eq!(g("CLOSE ON DOOR", "Desc."), 24.0, "shot to description");
    assert_eq!(g("Desc.", "EXT. D - DAY"), 36.0);
}

#[test]
fn dialogue_to_transition_is_one_blank_line() {
    let d = doc(vec![scene(
        "INT. A - DAY",
        vec![
            el(K::Character, "MEERA"),
            el(K::Dialogue, "Go."),
            el(K::Transition, "SMASH CUT TO:"),
        ],
    )]);
    let lay = layout::layout(&d, &opts());
    assert_eq!(gap_pt(&lay, "Go.", "SMASH CUT TO:"), 24.0);
}

#[test]
fn the_first_element_on_a_page_has_no_space_before() {
    // Fill the first page so the next heading starts the second one.
    let d = doc(vec![
        scene("INT. A - DAY", vec![el(K::Action, &rows_of(52))]),
        scene("INT. B - DAY", vec![el(K::Action, "Next.")]),
    ]);
    let lay = layout::layout(&d, &opts());
    let (page, row) = at(&lay, "INT. B - DAY");
    assert_eq!((page, row), (1, 0));
}

#[test]
fn shooting_draft_scene_numbers_sit_on_the_heading_row() {
    let mut d = doc(vec![
        scene("INT. A - DAY", vec![el(K::Action, "One.")]),
        scene("INT. B - DAY", vec![el(K::Action, "Two.")]),
    ]);
    d.scenes[1].number = Some("7A".into());
    let lay = layout::layout(
        &d,
        &LayoutOptions {
            scene_numbers: true,
            ..opts()
        },
    );
    let (_, hrow) = at(&lay, "INT. B - DAY");
    let nums: Vec<&PageLine> = lay.pages[0]
        .lines
        .iter()
        .filter(|l| l.style == LineStyle::SceneNumber && l.text == "7A")
        .collect();
    assert_eq!(nums.len(), 2, "left and right of the heading");
    assert!(nums.iter().all(|l| l.row == hrow));
}

// ------------------------------------------------------------------ keep-with-next

#[test]
fn a_transition_stays_with_the_next_scene_heading() {
    let d = doc(vec![
        scene(
            "INT. A - DAY",
            // Heading row 0, 51 action lines on rows 2..52, the transition on the last row (54).
            vec![el(K::Action, &rows_of(51)), el(K::Transition, "CUT TO:")],
        ),
        scene("INT. B - DAY", vec![el(K::Action, "After.")]),
    ]);
    let lay = layout::layout(&d, &opts());
    let (tp, _) = at(&lay, "CUT TO:");
    let (hp, hrow) = at(&lay, "INT. B - DAY");
    assert_eq!(tp, hp, "transition and heading share a page");
    assert_eq!(at(&lay, "After.").0, hp);
    assert_eq!(
        at(&lay, "line50").0,
        0,
        "the action stays on the first page"
    );
    // The transition now opens the page, and the heading keeps its 36 pt gap after it.
    assert_eq!(at(&lay, "CUT TO:").1, 0);
    assert_eq!(hrow, 3);
}

#[test]
fn a_shot_stays_with_its_description() {
    let d = doc(vec![scene(
        "INT. A - DAY",
        vec![
            el(K::Action, &rows_of(51)),
            el(K::Shot, "CLOSE ON THE DIAL"),
            el(K::Action, "The needle drifts."),
        ],
    )]);
    let lay = layout::layout(&d, &opts());
    assert_eq!(
        at(&lay, "CLOSE ON THE DIAL").0,
        at(&lay, "The needle drifts.").0
    );
    assert_eq!(at(&lay, "CLOSE ON THE DIAL"), (1, 0));
}

// ------------------------------------------------------------------ speech continuation

fn dialogue(n: usize) -> String {
    (0..n)
        .map(|i| format!("Spoken line {i:03} of the speech here."))
        .collect::<Vec<_>>()
        .join(" ")
}

fn marker_pairs(lay: &Layout) -> (usize, usize) {
    let more = lay
        .pages
        .iter()
        .flat_map(|p| p.lines.iter())
        .filter(|l| l.style == LineStyle::More)
        .count();
    let contd = lay
        .pages
        .iter()
        .flat_map(|p| p.lines.iter())
        .filter(|l| l.style == LineStyle::Character && l.text.ends_with("(CONT'D)"))
        .count();
    (more, contd)
}

#[test]
fn every_page_split_of_one_speech_has_more_and_contd() {
    // 120 dialogue lines starting low on page one: the speech runs over four pages.
    let d = doc(vec![scene(
        "INT. A - DAY",
        vec![
            el(K::Action, &rows_of(40)),
            el(K::Character, "MEERA"),
            el(K::Dialogue, &dialogue(120)),
        ],
    )]);
    let lay = layout::layout(&d, &opts());
    assert!(lay.pages.len() >= 3);
    let (more, contd) = marker_pairs(&lay);
    assert_eq!(more, lay.pages.len() - 1);
    assert_eq!(contd, lay.pages.len() - 1);
    for (i, p) in lay.pages.iter().enumerate() {
        let b = body(p);
        if i > 0 {
            assert_eq!(b[0].text, "MEERA (CONT'D)", "page {i} opens with the cue");
            assert_eq!(b[0].row, 0);
            assert_eq!(b[1].style, LineStyle::Dialogue);
        }
        if i + 1 < lay.pages.len() {
            let last = b.last().unwrap();
            assert_eq!(
                last.style,
                LineStyle::Dialogue,
                "split after a dialogue line"
            );
            let more = p.lines.iter().find(|l| l.style == LineStyle::More).unwrap();
            assert_eq!(more.text, "(MORE)");
            assert_eq!(more.row, last.row + 1);
            let dialogue_lines = b.iter().filter(|l| l.style == LineStyle::Dialogue).count();
            assert!(dialogue_lines >= 2, "never a cue with a single line");
        }
    }
    check_invariants(&lay, "long speech");
    assert_text_preserved(&d, &lay, "long speech");
}

#[test]
fn a_speech_taller_than_a_fresh_page_still_gets_markers() {
    // Only two rows are left when the speech starts, so it moves to a fresh page, where it is
    // still taller than the page: the old code split it anywhere with no markers.
    let d = doc(vec![scene(
        "INT. A - DAY",
        vec![
            el(K::Action, &rows_of(51)),
            el(K::Character, "ARJUN (V.O.)"),
            el(K::Parenthetical, "(over the radio)"),
            el(K::Dialogue, &dialogue(110)),
        ],
    )]);
    let lay = layout::layout(&d, &opts());
    let (more, contd) = marker_pairs(&lay);
    assert!(lay.pages.len() >= 3);
    assert_eq!(more, contd);
    assert!(more >= 1);
    assert!(
        lay.pages
            .iter()
            .flat_map(|p| p.lines.iter())
            .any(|l| l.text == "ARJUN (V.O.) (CONT'D)")
    );
    check_invariants(&lay, "tall speech");
    assert_text_preserved(&d, &lay, "tall speech");
}

#[test]
fn an_imported_contd_is_not_doubled() {
    let d = doc(vec![scene(
        "INT. A - DAY",
        vec![
            el(K::Action, &rows_of(30)),
            el(K::Character, "MEERA (CONT'D)"),
            el(K::Dialogue, &dialogue(60)),
        ],
    )]);
    let lay = layout::layout(&d, &opts());
    for l in lay.pages.iter().flat_map(|p| p.lines.iter()) {
        assert!(
            l.text.matches("CONT'D").count() <= 1,
            "doubled marker: {}",
            l.text
        );
    }
    assert!(marker_pairs(&lay).0 >= 1);
}

#[test]
fn a_long_cue_wraps_inside_its_column_when_continued() {
    let cue = "THE VERY TALL WOMAN IN THE GREEN RAINCOAT (V.O.)";
    let d = doc(vec![scene(
        "INT. A - DAY",
        vec![
            el(K::Action, &rows_of(30)),
            el(K::Character, cue),
            el(K::Dialogue, &dialogue(60)),
        ],
    )]);
    let lay = layout::layout(&d, &opts());
    let cue_lines: Vec<&PageLine> = lay.pages[1]
        .lines
        .iter()
        .filter(|l| l.style == LineStyle::Character)
        .collect();
    assert_eq!(cue_lines.len(), 2, "the continued cue wraps");
    for l in cue_lines {
        let right = l.x_in + l.text.chars().count() as f32 * CHAR_WIDTH_IN;
        assert!(right <= 7.5 + 1e-3, "{} ends at {right}", l.text);
    }
    assert_text_preserved(&d, &lay, "long cue");
}

#[test]
fn contd_is_not_added_without_a_page_split() {
    let d = doc(vec![
        scene(
            "INT. A - DAY",
            vec![
                el(K::Character, "MEERA"),
                el(K::Dialogue, "One."),
                el(K::Action, "A door slams."),
                el(K::Character, "MEERA"),
                el(K::Dialogue, "Two."),
            ],
        ),
        scene(
            "INT. B - DAY",
            vec![el(K::Character, "MEERA"), el(K::Dialogue, "Three.")],
        ),
    ]);
    let lay = layout::layout(&d, &opts());
    let all: Vec<&str> = lay
        .pages
        .iter()
        .flat_map(|p| p.lines.iter())
        .map(|l| l.text.as_str())
        .collect();
    assert!(all.iter().all(|t| !t.contains("CONT'D") && *t != "(MORE)"));
}

// ------------------------------------------------------------------ pagination invariants

/// Validates the page-boundary rules of the standard on a finished layout.
fn check_invariants(lay: &Layout, ctx: &str) {
    let lpp = lay.geometry.lines_per_page as i32;
    let script: Vec<&LaidOutPage> = lay.pages.iter().filter(|p| !p.title_page).collect();
    for (i, p) in script.iter().enumerate() {
        let b = body(p);
        assert!(!b.is_empty(), "{ctx}: page {i} is empty");
        for l in &p.lines {
            assert!(
                l.row < lpp,
                "{ctx}: {:?} on row {} of page {i}",
                l.text,
                l.row
            );
        }
        let last = b.last().unwrap();
        assert!(
            !matches!(
                last.style,
                LineStyle::Heading
                    | LineStyle::Shot
                    | LineStyle::Character
                    | LineStyle::Parenthetical
            ),
            "{ctx}: page {i} ends with an orphaned {:?} {:?}",
            last.style,
            last.text
        );
        if let (LineStyle::Transition, Some(next)) = (last.style, script.get(i + 1)) {
            let first = body(next);
            assert!(
                first[0].style != LineStyle::Heading,
                "{ctx}: transition {:?} stranded before {:?}",
                last.text,
                first[0].text
            );
        }
        // (MORE) closes a page only after dialogue, and the next page opens with the continued cue.
        if let Some(more) = p.lines.iter().find(|l| l.style == LineStyle::More) {
            assert_eq!(
                last.style,
                LineStyle::Dialogue,
                "{ctx}: (MORE) after {:?}",
                last.text
            );
            assert_eq!(more.row, last.row + 1, "{ctx}");
            let next = script.get(i + 1).expect("(MORE) on the last page");
            let first = body(next)[0];
            assert!(
                first.style == LineStyle::Character && first.text.ends_with("(CONT'D)"),
                "{ctx}: page after (MORE) opens with {:?}",
                first.text
            );
        }
    }
}

/// Every non-note element's text appears in order, exactly once, in the laid-out lines
/// (whitespace aside, so wrapping and hard breaks do not matter; generated markers are ignored).
fn assert_text_preserved(d: &ScreenplayDoc, lay: &Layout, ctx: &str) {
    let mut expected = String::new();
    for s in &d.scenes {
        if !s.heading.trim().is_empty() {
            expected.push_str(&s.heading.to_uppercase());
        }
        for e in &s.elements {
            match e.kind {
                K::Note | K::PageBreak | K::Section | K::Synopsis => {}
                K::Character => {
                    expected.push_str(&e.text.trim().trim_end_matches('^').trim().to_uppercase())
                }
                K::Transition | K::Shot => expected.push_str(&e.text.trim().to_uppercase()),
                _ => expected.push_str(&e.text),
            }
        }
    }
    let mut actual = String::new();
    let script: Vec<&LaidOutPage> = lay.pages.iter().filter(|p| !p.title_page).collect();
    for (i, p) in script.iter().enumerate() {
        let continued = i > 0
            && script[i - 1]
                .lines
                .iter()
                .any(|l| l.style == LineStyle::More);
        let mut skipping = continued;
        for l in body(p) {
            if skipping && l.style == LineStyle::Character {
                continue; // the generated NAME (CONT'D) cue
            }
            skipping = false;
            actual.push_str(&l.text);
        }
    }
    let strip = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    assert_eq!(
        strip(&actual),
        strip(&expected),
        "{ctx}: script text changed"
    );
}

fn tails() -> Vec<(&'static str, Vec<Scene>)> {
    vec![
        (
            "heading chain",
            vec![scene(
                "INT. B - DAY",
                vec![el(K::Action, "Then it happens.")],
            )],
        ),
        (
            "transition then heading",
            vec![scene(
                "INT. B - DAY",
                vec![el(K::Action, "Then it happens.")],
            )],
        ),
        (
            "shot then action",
            vec![scene(
                "INT. B - DAY",
                vec![el(K::Shot, "ANGLE ON THE DOOR"), el(K::Action, "It opens.")],
            )],
        ),
        (
            "short speech with parenthetical",
            vec![scene(
                "INT. B - DAY",
                vec![
                    el(K::Character, "MEERA"),
                    el(K::Parenthetical, "(quietly)"),
                    el(K::Dialogue, &dialogue(2)),
                ],
            )],
        ),
        (
            "long speech",
            vec![scene(
                "INT. B - DAY",
                vec![el(K::Character, "MEERA"), el(K::Dialogue, &dialogue(70))],
            )],
        ),
        (
            "long action",
            vec![scene("INT. B - DAY", vec![el(K::Action, &rows_of(14))])],
        ),
        (
            "consecutive short scenes",
            vec![
                scene("INT. B - DAY", vec![el(K::Action, "One.")]),
                scene("INT. C - DAY", vec![el(K::Action, "Two.")]),
                scene("INT. D - DAY", vec![el(K::Action, "Three.")]),
            ],
        ),
    ]
}

/// Sweeps every fill height so each page-boundary offset meets each kind of element.
#[test]
fn pagination_rules_hold_at_every_page_offset() {
    for (label, o) in [("letter", opts()), ("a4", a4_opts())] {
        for (name, tail) in tails() {
            for fill in 1..=58 {
                let mut first = scene("INT. A - DAY", vec![el(K::Action, &rows_of(fill))]);
                if name == "transition then heading" {
                    first.elements.push(el(K::Transition, "CUT TO:"));
                }
                let mut scenes = vec![first];
                scenes.extend(tail.clone());
                let d = doc(scenes);
                let ctx = format!("{label} / {name} / fill {fill}");
                let lay = layout::layout(&d, &o);
                check_invariants(&lay, &ctx);
                assert_text_preserved(&d, &lay, &ctx);
                // Layout is deterministic.
                assert_eq!(lay, layout::layout(&d, &o), "{ctx}: not deterministic");
            }
        }
    }
}

#[test]
fn a_heading_a_cue_and_a_parenthetical_near_the_foot_move_with_their_text() {
    // Heading: rows 0..; leave exactly 2 rows then a heading and action.
    for fill in 45..=54 {
        let d = doc(vec![
            scene("INT. A - DAY", vec![el(K::Action, &rows_of(fill))]),
            scene(
                "INT. B - DAY",
                vec![
                    el(K::Character, "MEERA"),
                    el(K::Parenthetical, "(low)"),
                    el(K::Dialogue, "Stay."),
                ],
            ),
        ]);
        let lay = layout::layout(&d, &opts());
        let hp = at(&lay, "INT. B - DAY").0;
        assert_eq!(hp, at(&lay, "MEERA").0, "fill {fill}");
        assert_eq!(hp, at(&lay, "(low)").0, "fill {fill}");
        assert_eq!(hp, at(&lay, "Stay.").0, "fill {fill}");
    }
}

#[test]
fn a_long_action_splits_with_two_lines_on_each_side() {
    for fill in 30..=52 {
        let d = doc(vec![scene(
            "INT. A - DAY",
            vec![
                el(K::Action, &rows_of(fill)),
                el(K::Action, &rows_of(9).replace("line", "tail")),
            ],
        )]);
        let lay = layout::layout(&d, &opts());
        if lay.pages.len() < 2 {
            continue;
        }
        let tail_on = |p: usize| {
            body(&lay.pages[p])
                .iter()
                .filter(|l| l.text.starts_with("tail"))
                .count()
        };
        let first = tail_on(0);
        let second = tail_on(1);
        assert_eq!(first + second, 9, "fill {fill}");
        assert!(first == 0 || first >= 2, "fill {fill}: {first} on top");
        assert!(second == 0 || second >= 2, "fill {fill}: {second} below");
    }
}

#[test]
fn an_action_taller_than_a_page_leaves_no_single_line_behind() {
    let d = doc(vec![scene(
        "INT. A - DAY",
        vec![el(K::Action, &rows_of(54 + 55 + 1))],
    )]);
    let lay = layout::layout(&d, &opts());
    for (i, p) in lay.pages.iter().enumerate() {
        assert!(body(p).len() >= 2, "page {i} holds a single line");
    }
    assert_text_preserved(&d, &lay, "tall action");
}

#[test]
fn explicit_page_breaks_start_a_page_and_do_not_repeat() {
    let d = doc(vec![
        scene(
            "INT. A - DAY",
            vec![
                el(K::Action, "One."),
                Element::page_break(),
                Element::page_break(),
            ],
        ),
        scene("INT. B - DAY", vec![el(K::Action, "Two.")]),
    ]);
    let lay = layout::layout(&d, &opts());
    assert_eq!(lay.pages.len(), 2);
    assert_eq!(at(&lay, "INT. B - DAY"), (1, 0));
}

// ------------------------------------------------------------------ page numbers and furniture

#[test]
fn first_page_numbering_is_an_explicit_setting() {
    let d = doc(vec![scene(
        "INT. A - DAY",
        vec![el(K::Action, &rows_of(60))],
    )]);
    let numbers = |setup: PageSetup, title: bool| {
        let mut dd = d.clone();
        dd.title_page.set("Title", "Low Tide Radio");
        let lay = layout::layout(
            &dd,
            &LayoutOptions {
                title_page: title,
                page: setup,
                ..Default::default()
            },
        );
        lay.pages
            .iter()
            .map(|p| {
                p.lines
                    .iter()
                    .filter(|l| l.style == LineStyle::PageNumber)
                    .map(|l| l.text.clone())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    };
    // Default: unchanged from before, "1." on the first script page.
    assert!(PageSetup::default().number_first_page);
    let with = numbers(PageSetup::default(), false);
    assert_eq!(with[0], vec!["1."]);
    assert_eq!(with[1], vec!["2."]);
    // Off: the first script page is unnumbered and the next one prints "2.".
    let without = numbers(
        PageSetup {
            number_first_page: false,
            ..Default::default()
        },
        false,
    );
    assert!(without[0].is_empty());
    assert_eq!(without[1], vec!["2."]);
    // The title page is never numbered, with either setting.
    for setup in [
        PageSetup::default(),
        PageSetup {
            number_first_page: false,
            ..Default::default()
        },
    ] {
        let n = numbers(setup, true);
        assert!(n[0].is_empty(), "title page: {:?}", n[0]);
    }
    // A saved setup from before the setting existed reads as the old behaviour.
    let old: PageSetup = serde_json::from_str(r#"{"paper":"Letter","fontSize":12}"#).unwrap();
    assert!(old.number_first_page);
}

#[test]
fn header_and_footer_follow_their_own_first_page_switch() {
    let d = doc(vec![scene(
        "INT. A - DAY",
        vec![el(K::Action, &rows_of(60))],
    )]);
    for on in [false, true] {
        let lay = layout::layout(
            &d,
            &LayoutOptions {
                page: PageSetup {
                    header: "DRAFT".into(),
                    footer: "Page {page}".into(),
                    header_on_first_page: on,
                    ..Default::default()
                },
                ..opts()
            },
        );
        let has = |p: usize, s: LineStyle| lay.pages[p].lines.iter().any(|l| l.style == s);
        assert_eq!(has(0, LineStyle::Header), on);
        assert_eq!(has(0, LineStyle::Footer), on);
        assert!(has(1, LineStyle::Header) && has(1, LineStyle::Footer));
        assert!(has(0, LineStyle::PageNumber), "number is its own setting");
    }
}

// ------------------------------------------------------------------ title page

fn title_doc(entries: &[(&str, &str)]) -> ScreenplayDoc {
    let mut d = doc(vec![scene("INT. A - DAY", vec![el(K::Action, "Rain.")])]);
    d.title_page.set("Title", "Low Tide Radio");
    d.title_page.set("Author", "Asha Rao");
    for (k, v) in entries {
        d.title_page.set(k, *v);
    }
    d
}

fn title_pages(d: &ScreenplayDoc) -> Vec<LaidOutPage> {
    let lay = layout::layout(d, &LayoutOptions::default());
    lay.pages.into_iter().filter(|p| p.title_page).collect()
}

fn width_in(l: &PageLine) -> f32 {
    // These tests use ASCII text only: one 0.1 inch column per character.
    l.text.chars().count() as f32 * CHAR_WIDTH_IN
}

/// No two lines on the same row may overlap or sit closer than `gutter` inches.
fn assert_no_collisions(pages: &[LaidOutPage], gutter: f32) {
    for p in pages {
        for a in &p.lines {
            for b in &p.lines {
                if std::ptr::eq(a, b) || a.row != b.row || a.x_in > b.x_in {
                    continue;
                }
                assert!(
                    a.x_in + width_in(a) + gutter <= b.x_in + 1e-3,
                    "row {}: {:?} (to {:.2}) touches {:?} (from {:.2})",
                    a.row,
                    a.text,
                    a.x_in + width_in(a),
                    b.text,
                    b.x_in
                );
            }
        }
    }
}

fn assert_in_safe_area(pages: &[LaidOutPage], lpp: i32) {
    for p in pages {
        for l in &p.lines {
            assert!(l.row >= 0 && l.row < lpp, "{:?} on row {}", l.text, l.row);
            assert!(l.x_in >= 1.0 - 1e-3, "{:?} at x {}", l.text, l.x_in);
            assert!(
                l.x_in + width_in(l) <= 7.5 + 1e-3,
                "{:?} ends at {}",
                l.text,
                l.x_in + width_in(l)
            );
        }
    }
}

fn letters(s: &str) -> Vec<char> {
    let mut v: Vec<char> = s.chars().filter(|c| c.is_alphanumeric()).collect();
    v.sort_unstable();
    v
}

#[test]
fn title_page_blocks_keep_a_gutter_and_every_word() {
    let copyright = "Copyright 2026 Kayal Pictures. All rights reserved.";
    let rep = "Representation: Lighthouse Talent, Mumbai";
    let d = title_doc(&[
        ("Contact", "Kayal Pictures\n14 Example Road\nPune 411001"),
        ("Copyright", copyright),
        ("Draft", "Second Draft"),
        ("Draft date", "24 October 2026"),
        ("Revision", "Blue revision, pages 4 to 19"),
        ("Representation", rep),
    ]);
    let pages = title_pages(&d);
    assert_eq!(pages.len(), 1);
    assert_no_collisions(&pages, 0.3);
    assert_in_safe_area(&pages, 55);
    let printed: String = pages[0].lines.iter().map(|l| l.text.as_str()).collect();
    let wanted = format!(
        "LOW TIDE RADIOWritten byAsha RaoKayal Pictures14 Example RoadPune 411001{copyright}Second Draft24 October 2026Blue revision, pages 4 to 19{rep}"
    );
    assert_eq!(letters(&printed), letters(&wanted));
    // The old layout joined the two blocks: "AllRepresentation".
    assert!(!printed.contains("AllRepresentation"));
}

#[test]
fn the_left_block_widens_when_the_right_block_is_short() {
    let d = title_doc(&[
        ("Contact", "agent@example-literary-agency.example.com"),
        ("Draft", "Draft 3"),
    ]);
    let pages = title_pages(&d);
    assert_no_collisions(&pages, 0.3);
    // 41 characters do not fit 34: the address wraps, but nothing is dropped.
    let joined: String = pages[0].lines.iter().map(|l| l.text.as_str()).collect();
    assert!(joined.contains("example.com") || joined.contains("agency"));
    assert_eq!(
        letters(&joined),
        letters("LOW TIDE RADIOWritten byAsha Raoagent@example-literary-agency.example.comDraft 3")
    );
}

#[test]
fn very_long_blocks_wrap_inside_their_own_width_and_never_leave_the_page() {
    let long = "Kayal Pictures Private Limited, registered office at the old lighthouse road, Mumbai 400001, India";
    let d = title_doc(&[
        ("Contact", long),
        (
            "Copyright",
            &format!("Copyright 2026 {long}. All rights reserved worldwide."),
        ),
        ("Representation", &format!("Representation: {long}")),
        ("Draft", "Second Draft"),
    ]);
    let pages = title_pages(&d);
    assert_no_collisions(&pages, 0.3);
    assert_in_safe_area(&pages, 55);
}

#[test]
fn a_too_tall_title_page_moves_the_title_up_then_continues_on_a_second_page() {
    let notes = (0..70)
        .map(|i| format!("note {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let d = title_doc(&[("Notes", &notes), ("Draft", "Draft 1")]);
    let pages = title_pages(&d);
    assert_eq!(
        pages.len(),
        2,
        "the surplus continues on a second title page"
    );
    assert_no_collisions(&pages, 0.3);
    assert_in_safe_area(&pages, 55);
    let printed: String = pages
        .iter()
        .flat_map(|p| p.lines.iter())
        .map(|l| l.text.as_str())
        .collect();
    assert_eq!(
        letters(&printed),
        letters(&format!("LOW TIDE RADIOWritten byAsha Rao{notes}Draft 1"))
    );
    // Moderately tall blocks stay on one page by moving the title block up.
    let notes = (0..26)
        .map(|i| format!("note {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let pages = title_pages(&title_doc(&[("Notes", &notes)]));
    assert_eq!(pages.len(), 1);
    assert_no_collisions(&pages, 0.3);
    assert_in_safe_area(&pages, 55);
}

#[test]
fn title_page_on_and_off_changes_only_the_title_page() {
    let d = title_doc(&[("Draft", "Draft 1")]);
    let with = layout::layout(&d, &LayoutOptions::default());
    let without = layout::layout(&d, &opts());
    assert!(with.pages[0].title_page && !without.pages[0].title_page);
    assert_eq!(with.pages[1..], without.pages[..]);
}

// ------------------------------------------------------------------ content cases

#[test]
fn awkward_text_is_kept_whole_and_in_order() {
    let d = doc(vec![scene(
        "INT. CAFÉ NOÉMIE - NIGHT",
        vec![
            el(K::Action, "Zoë glances at the façade. “Naïve,” she says."),
            el(K::Character, "ZOË (O.S.)"),
            el(
                K::Parenthetical,
                "(a very long parenthetical that has to wrap inside its narrow column twice)",
            ),
            el(
                K::Dialogue,
                &format!("{} {}", "x".repeat(70), "మీరు ఎలా ఉన్నారు? नमस्ते दुनिया"),
            ),
            el(
                K::Character,
                "A CHARACTER WITH A VERY LONG NAME THAT NEEDS TWO LINES",
            ),
            el(K::Dialogue, "Fine."),
            el(K::Character, "MEERA").dual(true),
            el(K::Dialogue, "Left.").dual(true),
            el(K::Character, "ARJUN").dual(true),
            el(K::Dialogue, "Right.").dual(true),
            el(K::Note, "never printed"),
        ],
    )]);
    let o = opts();
    let lay = layout::layout(&d, &o);
    let mut printed = String::new();
    for l in lay
        .pages
        .iter()
        .flat_map(|p| p.lines.iter())
        .filter(|l| is_script(l.style))
    {
        printed.push_str(&l.text);
    }
    let printed_letters = letters(&printed);
    let mut expected = String::new();
    for e in &d.scenes[0].elements {
        if e.kind != K::Note {
            expected.push_str(&e.text.to_uppercase());
        }
    }
    expected.push_str(&d.scenes[0].heading.to_uppercase());
    // Case differs per element kind, so compare case-insensitively.
    let lower = |v: Vec<char>| {
        v.into_iter()
            .flat_map(|c| c.to_lowercase())
            .collect::<Vec<_>>()
    };
    let mut a = lower(printed_letters);
    let mut b = lower(letters(&expected));
    a.sort_unstable();
    b.sort_unstable();
    assert_eq!(a, b);
    check_invariants(&lay, "awkward");
}
