//! The group band, rich cell text, wrapped placeholders, storyboard panels, the scene banner and
//! per-part page headers of the shot list and storyboard designs.

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

fn has(p: &Page, needle: &str) -> bool {
    p.runs.iter().any(|r| r.3.contains(needle))
}

fn doc_with(blocks: Vec<Block>) -> Document {
    let mut d = Document::new("Visual").brand(BRAND_LINE);
    for b in blocks {
        d.push(b);
    }
    d
}

fn band(title: &str, right: &str) -> TableBand {
    let mut b = TableBand::new(title);
    b.right = right.into();
    b
}

fn shot_table(rows: usize, band: TableBand) -> StyledTable {
    let mut t = StyledTable::new(vec![Col::pt("Shot", 40.0), Col::weight("Description", 1.0)]);
    t.band = Some(band);
    t.pad_x = 4.0;
    t.pad_y = 5.0;
    for i in 0..rows {
        t.rows.push(Row::new(vec![
            Cell::text(Txt::new(format!("{}A", i + 1))),
            Cell::text(Txt::new(format!("Row {i:02}"))).item(CellItem::Gap(40.0)),
        ]));
    }
    t
}

#[test]
fn a_band_repeats_with_continued_and_keeps_its_status() {
    let mut band = TableBand::new("EXT. HARBOUR - DAWN");
    band.eyebrow = Some("Scene 7".into());
    band.right = "30 shots".into();
    band.status = Some(("Scene changed since planning".into(), Tone::Warn));
    band.strip = vec![("Lenses".into(), "24mm · 50mm".into())];
    let pdf = doc_with(vec![Block::Styled(shot_table(30, band))]).to_pdf();
    let ps = pages(&pdf.bytes);
    assert!(ps.len() >= 3, "{} pages", ps.len());
    // First page: the count and the strip; later pages: CONTINUED, the status, the column header.
    assert!(has(&ps[0], "30 SHOTS") && has(&ps[0], "LENSES"));
    for p in &ps[1..] {
        assert!(has(p, "CONTINUED") && !has(p, "30 SHOTS"));
        assert!(has(p, "Scene changed since planning"));
        assert!(has(p, "SHOT") && has(p, "DESCRIPTION"));
        assert!(!has(p, "LENSES"), "the strip is for the first page");
    }
    // No page leaves a band alone: a row follows it on every page.
    for p in &ps {
        assert!(has(p, "Row "), "rows on every page");
    }
}

#[test]
fn a_short_group_in_together_is_never_split() {
    let mut blocks = vec![Block::Styled(shot_table(9, band("FIRST", "9 shots")))];
    // 40 pt rows plus the band: three of them need about 200 pt, which the rest of page 1 lacks.
    blocks.push(Block::Together(vec![Block::Styled(shot_table(
        3,
        band("SECOND", "3 shots"),
    ))]));
    let ps = pages(&doc_with(blocks).to_pdf().bytes);
    let second: Vec<usize> = (0..ps.len()).filter(|i| has(&ps[*i], "SECOND")).collect();
    assert_eq!(second.len(), 1);
    assert!(!has(&ps[second[0]], "CONTINUED") || !has(&ps[second[0]], "FIRST"));
    let rows_on = |p: &Page| p.runs.iter().filter(|r| r.3.starts_with("Row ")).count();
    assert!(ps[second[0]].runs.iter().any(|r| r.3 == "SECOND"));
    assert!(rows_on(&ps[second[0]]) >= 3);
}

#[test]
fn a_placeholder_label_wraps_inside_its_frame() {
    let mut t = StyledTable::new(vec![Col::pt("Image", 88.0), Col::weight("Text", 1.0)]);
    t.pad_x = 4.0;
    t.rows.push(Row::new(vec![
        Cell::new().item(CellItem::Placeholder {
            w: 80.0,
            h: 45.0,
            label: "Image unavailable: a-very-long-original-file-name-from-the-camera-roll.png"
                .into(),
        }),
        Cell::text(Txt::new("x")),
    ]));
    let ps = pages(&doc_with(vec![Block::Styled(t)]).to_pdf().bytes);
    let label: Vec<&(f32, f32, f32, String)> = ps[0]
        .runs
        .iter()
        .filter(|r| r.2 == 7.0 && r.1 > 70.0 && !matches!(r.3.as_str(), "IMAGE" | "TEXT"))
        .collect();
    assert!(label.len() >= 3, "wrapped over several lines: {label:?}");
    for r in &label {
        assert!(r.0 >= MARGIN + 4.0 - 0.5, "{r:?} starts inside the frame");
        assert!(
            r.0 + text_width(&r.3, 7.0, false) <= MARGIN + 4.0 + 80.0 + 0.5,
            "{r:?} ends inside the frame"
        );
    }
}

#[test]
fn cell_runs_keep_a_bold_label_with_its_text() {
    let mut t = StyledTable::new(vec![Col::weight("Description", 1.0)]);
    let rich = Rich::new(8.0).para(vec![
        Run::new("Sound:", true, pal::NAVY),
        Run::new("wild wind and engines under the deck", false, pal::SLATE),
    ]);
    t.rows.push(Row::new(vec![
        Cell::text(Txt::new("Shot")).item(CellItem::Rich(rich)),
    ]));
    let ps = pages(&doc_with(vec![Block::Styled(t)]).to_pdf().bytes);
    let label = ps[0].runs.iter().find(|r| r.3 == "Sound:").expect("label");
    let text = ps[0]
        .runs
        .iter()
        .find(|r| r.3.starts_with("wild wind"))
        .expect("text");
    assert!((label.1 - text.1).abs() < 0.01, "same baseline");
    assert!(text.0 > label.0 + text_width("Sound:", 8.0, true));
}

fn panel_card(n: usize, caption: &str) -> Card {
    let mut p = Panel::new(n.to_string());
    p.shot = Some(format!("Shot {n}A"));
    p.placeholder = Some("Blank panel".into());
    p.chips = vec!["Wide".into(), "Static".into()];
    p.action = Some(caption.into());
    p.lines = vec![
        ("Sound:".into(), "rain".into()),
        ("Duration:".into(), "3s".into()),
    ];
    Card {
        parts: vec![CardPart::Panel(Box::new(p))],
        ..Card::new()
    }
}

#[test]
fn panels_in_a_row_share_one_height_and_columns_are_equal() {
    let long = "A long action line that wraps over many lines inside the caption band ".repeat(4);
    let cards = vec![panel_card(1, "Short"), panel_card(2, &long)];
    let ps = pages(
        &doc_with(vec![Block::Cards(Cards::new(2, cards))])
            .to_pdf()
            .bytes,
    );
    let tiles: Vec<f32> = ps[0]
        .runs
        .iter()
        .filter(|r| r.3 == "Panel")
        .map(|r| r.0)
        .collect();
    assert_eq!(tiles.len(), 2);
    // Two equal columns: the second card starts (520 - 10) / 2 + 10 = 265 pt after the first.
    assert!((tiles[1] - tiles[0] - 265.0).abs() < 0.5, "{tiles:?}");
    // Both captions' "Sound:" labels share a baseline only when the bands start at the same height.
    let sounds: Vec<f32> = ps[0]
        .runs
        .iter()
        .filter(|r| r.3 == "Sound:")
        .map(|r| r.1)
        .collect();
    assert_eq!(sounds.len(), 2);
    assert!(
        sounds[0] > sounds[1],
        "the long card's lines sit lower: {sounds:?}"
    );
}

#[test]
fn page_headers_follow_each_part_and_replace_the_one_before() {
    let mut blocks = Vec::new();
    for name in ["ALPHA", "BETA"] {
        if name == "BETA" {
            blocks.push(Block::PageBreak);
        }
        blocks.push(Block::PageHeader(format!("PROJECT · STORYBOARDS · {name}")));
        blocks.push(Block::Styled(shot_table(20, band(name, "20 shots"))));
    }
    let ps = pages(&doc_with(blocks).to_pdf().bytes);
    assert!(ps.len() >= 4);
    for p in &ps {
        let heads: Vec<&str> = p
            .runs
            .iter()
            .filter(|r| r.1 > 760.0)
            .map(|r| r.3.as_str())
            .collect();
        assert_eq!(
            heads.len(),
            1,
            "exactly one header line per page: {heads:?}"
        );
    }
    let first = ps.iter().position(|p| has(p, "· BETA")).unwrap();
    assert!(first >= 2);
    assert!(
        ps[..first]
            .iter()
            .all(|p| has(p, "· ALPHA") && !has(p, "· BETA"))
    );
    assert!(
        ps[first..]
            .iter()
            .all(|p| has(p, "· BETA") && !has(p, "· ALPHA"))
    );
}

#[test]
fn the_scene_banner_prints_name_scene_line_count_and_status() {
    let b = SceneBanner {
        eyebrow: "Storyboard".into(),
        title: "Scene 25: the whole length of a very long board name that wraps".into(),
        scene_line: "Scene 25: EXT. HARBOUR - DAWN".into(),
        count: "4 panels".into(),
        status: Some(("Scene changed since planning".into(), Tone::Warn)),
    };
    let ps = pages(&doc_with(vec![Block::SceneBanner(b)]).to_pdf().bytes);
    for want in [
        "STORYBOARD",
        "4 PANELS",
        "Scene changed since planning",
        "EXT. HARBOUR",
    ] {
        assert!(has(&ps[0], want), "missing {want}");
    }
}

#[test]
fn a_long_word_after_other_words_is_broken_to_fit() {
    let lines = wrap_pt(
        "Image unavailable: shot-01a16e89-9db4-728e-9066-40462e32ead9.png",
        70.0,
        7.0,
        false,
    );
    assert!(lines.len() >= 3, "{lines:?}");
    for l in &lines {
        assert!(
            text_width(l, 7.0, false) <= 70.0 + 0.01,
            "{l:?} is too wide"
        );
    }
    assert_eq!(
        lines.concat().replace(' ', ""),
        "Imageunavailable:shot-01a16e89-9db4-728e-9066-40462e32ead9.png"
    );
}
