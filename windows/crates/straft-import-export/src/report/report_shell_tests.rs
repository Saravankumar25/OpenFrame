//! The shell of a production report (head with side panel, context bands, metrics strip) and the
//! status mark on a chip.

use super::*;

struct Page {
    /// (x, y, size, text) of every text run.
    runs: Vec<(f32, f32, f32, String)>,
}

fn pages(bytes: &[u8]) -> Vec<Page> {
    let doc = lopdf::Document::load_mem(bytes).expect("PDF parses");
    let num = |o: &lopdf::Object| o.as_float().unwrap_or(0.0);
    doc.get_pages()
        .into_values()
        .map(|page| {
            let content = lopdf::content::Content::decode(&doc.get_page_content(page)).unwrap();
            let (mut size, mut at) = (0.0f32, (0.0f32, 0.0f32));
            let mut runs = Vec::new();
            for op in content.operations {
                let a = &op.operands;
                match op.operator.as_str() {
                    "Tf" => size = num(&a[1]),
                    "Tm" => at = (num(&a[4]), num(&a[5])),
                    "Tj" => {
                        if let lopdf::Object::String(b, _) = &a[0] {
                            runs.push((
                                at.0,
                                at.1,
                                size,
                                b.iter().map(|x| crate::winansi::decode_byte(*x)).collect(),
                            ));
                        }
                    }
                    _ => {}
                }
            }
            Page { runs }
        })
        .collect()
}

fn run<'a>(p: &'a Page, text: &str) -> &'a (f32, f32, f32, String) {
    p.runs
        .iter()
        .find(|r| r.3 == text)
        .unwrap_or_else(|| panic!("{text} is not printed: {:?}", p.runs))
}

fn doc_with(blocks: Vec<Block>) -> Document {
    let mut d = Document::new("Shell").brand(BRAND_LINE);
    for b in blocks {
        d.push(b);
    }
    d
}

fn head(title: &str) -> ReportHead {
    ReportHead {
        eyebrow: "Production report".into(),
        title: title.into(),
        meta: Some("Project: Low Tide Radio \u{b7} Source: Blue Revision".into()),
        panel_label: "Generated".into(),
        panel_value: "22 Oct 2026".into(),
        panel_note: "All rows".into(),
    }
}

fn status_table(mark: Option<Mark>) -> StyledTable {
    let mut t = StyledTable::new(vec![Col::pt("Status", 120.0)]);
    let mut chip = Chip::label("Reviewed", Tone::Ok);
    chip.mark = mark;
    t.rows.push(Row::new(vec![
        Cell::new().item(CellItem::Chips(vec![chip])),
    ]));
    t
}

#[test]
fn a_chip_without_a_mark_is_unchanged_and_a_mark_moves_the_word_right() {
    let plain = doc_with(vec![Block::Styled(status_table(None))]).to_pdf();
    let again = doc_with(vec![Block::Styled(status_table(None))]).to_pdf();
    assert_eq!(plain.bytes, again.bytes, "deterministic");
    let x_plain = run(&pages(&plain.bytes)[0], "REVIEWED").0;
    for mark in [Mark::Disc, Mark::RingDot, Mark::Ring, Mark::Square] {
        let marked = doc_with(vec![Block::Styled(status_table(Some(mark)))]).to_pdf();
        let x = run(&pages(&marked.bytes)[0], "REVIEWED").0;
        assert!(
            (x - x_plain - 8.8).abs() < 0.01,
            "{mark:?}: the word moves right by the mark and its gap, not {}",
            x - x_plain
        );
        assert_ne!(marked.bytes, plain.bytes, "{mark:?} is drawn");
    }
}

#[test]
fn the_head_prints_the_eyebrow_title_meta_and_a_right_aligned_panel() {
    let pdf = doc_with(vec![Block::ReportHead(head("Scene Report"))]).to_pdf();
    let p = &pages(&pdf.bytes)[0];
    let eyebrow = run(p, "PRODUCTION REPORT");
    let title = run(p, "Scene Report");
    let meta = run(p, "Project: Low Tide Radio \u{b7} Source: Blue Revision");
    assert_eq!((title.2, meta.2, eyebrow.2), (20.0, 8.0, 7.0));
    assert!(eyebrow.1 > title.1 && title.1 > meta.1, "top to bottom");
    let label = run(p, "GENERATED");
    let date = run(p, "22 Oct 2026");
    let scope = run(p, "All rows");
    assert_eq!((label.2, date.2, scope.2), (6.8, 10.5, 8.0));
    assert!(label.1 > date.1 && date.1 > scope.1);
    // The panel sits at the right edge of the 520 pt content width (46 to 566).
    for r in [label, date, scope] {
        assert!(r.0 > 566.0 - 150.0 && r.0 < 566.0, "{} at {}", r.3, r.0);
    }
}

#[test]
fn a_long_title_wraps_and_the_panel_keeps_its_width() {
    let long = "Cast Scene Report: LOW TIDE RADIO: Blue Revision (Locked) with a very long snapshot name that keeps going";
    let pdf = doc_with(vec![Block::ReportHead(head(long))]).to_pdf();
    let p = &pages(&pdf.bytes)[0];
    let lines: Vec<_> = p.runs.iter().filter(|r| r.2 == 20.0).collect();
    assert!(lines.len() >= 2, "the title wraps: {lines:?}");
    assert_eq!(
        lines
            .iter()
            .map(|r| r.3.as_str())
            .collect::<Vec<_>>()
            .join(" "),
        long,
        "nothing is cut"
    );
    for r in &lines {
        assert!(
            r.0 < 566.0 - 104.0 - 14.0,
            "{} stays left of the panel",
            r.3
        );
    }
}

#[test]
fn a_note_band_wraps_and_a_metric_strip_keeps_its_labels() {
    let filter = "coast ".repeat(60);
    let band = NoteBand {
        label: "Filter".into(),
        runs: vec![
            Run::new(
                format!("\u{201c}{}\u{201d}", filter.trim()),
                true,
                pal::NAVY,
            ),
            Run::new("\u{b7} rows with this text in any cell", false, pal::SLATE),
        ],
        tone: Tone::Info,
        stripe: pal::TEAL,
    };
    let strip = MetricStrip {
        items: vec![
            Metric::new("44", "Scenes"),
            Metric::new("4", "Unscheduled").tone(Tone::Warn),
            Metric::new("0", "In progress or needs review"),
            Metric::new("12", "Not started").tone(Tone::Danger),
        ],
    };
    let pdf = doc_with(vec![
        Block::ReportHead(head("Scene Report")),
        Block::NoteBand(band),
        Block::MetricStrip(strip),
    ])
    .to_pdf();
    assert_eq!(pdf.pages, 1);
    let p = &pages(&pdf.bytes)[0];
    assert_eq!(run(p, "FILTER").2, 7.0);
    let words: Vec<_> = p
        .runs
        .iter()
        .filter(|r| r.2 == 8.0 && r.1 < 700.0)
        .collect();
    assert!(words.len() >= 3, "the filter text wraps over lines");
    for label in [
        "SCENES",
        "UNSCHEDULED",
        "IN PROGRESS OR NEEDS REVIEW",
        "NOT STARTED",
    ] {
        assert_eq!(run(p, label).2, 6.8, "{label}");
    }
    for value in ["44", "4", "0", "12"] {
        assert_eq!(run(p, value).2, 13.0);
    }
    // Values sit left to right in the order given, inside the content width.
    let xs: Vec<f32> = ["44", "4", "0", "12"].iter().map(|v| run(p, v).0).collect();
    assert!(
        xs.windows(2).all(|w| w[0] < w[1]) && xs[3] < 566.0,
        "{xs:?}"
    );
    // The tone of a count that needs attention is its text colour; the number always prints.
    assert_eq!(Tone::Warn.text(), pal::ORANGE_DEEP);
}

#[test]
fn a_metric_strip_wider_than_the_page_wraps_its_labels_inside_it() {
    let strip = MetricStrip {
        items: (0..6)
            .map(|i| {
                Metric::new(
                    format!("{i}"),
                    "A rather long label that must wrap onto lines",
                )
            })
            .collect(),
    };
    let pdf = doc_with(vec![Block::MetricStrip(strip)]).to_pdf();
    let p = &pages(&pdf.bytes)[0];
    let labels: Vec<_> = p
        .runs
        .iter()
        .filter(|r| r.2 == 6.8 && r.1 > 100.0)
        .collect();
    assert!(labels.len() > 6, "labels wrap");
    assert!(labels.iter().all(|r| r.0 >= 46.0 && r.0 < 566.0));
}

#[test]
fn the_new_blocks_are_checked_for_characters_the_fonts_cannot_print() {
    let mut h = head("Scene Report");
    h.title = "Scene \u{1f600}".into();
    let doc = doc_with(vec![Block::ReportHead(h)]);
    assert!(doc.unsupported_chars().contains(&'\u{1f600}'));
    let band = NoteBand {
        label: "Filter".into(),
        runs: vec![Run::new("\u{1f601}", false, pal::INK)],
        tone: Tone::Info,
        stripe: pal::TEAL,
    };
    assert!(
        doc_with(vec![Block::NoteBand(band)])
            .unsupported_chars()
            .contains(&'\u{1f601}')
    );
    let strip = MetricStrip {
        items: vec![Metric::new("1", "Sc\u{1f602}")],
    };
    assert!(
        doc_with(vec![Block::MetricStrip(strip)])
            .unsupported_chars()
            .contains(&'\u{1f602}')
    );
}
