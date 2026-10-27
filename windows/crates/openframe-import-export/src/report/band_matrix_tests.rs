//! The table band, the spanning lead cell, the labelled cell item, keep-whole tables and the Day Out
//! of Days matrix (PDF design package v2: Shooting Schedule and Day Out of Days). Every test reads
//! the text runs of the real PDF with their page and position.

use super::*;

#[derive(Debug, Clone)]
struct Run {
    page: usize,
    x: f32,
    y: f32,
    size: f32,
    text: String,
}

fn runs(bytes: &[u8]) -> Vec<Run> {
    let doc = lopdf::Document::load_mem(bytes).expect("PDF parses");
    let num = |o: &lopdf::Object| o.as_float().unwrap_or(0.0);
    let mut out = Vec::new();
    for (n, (_, page)) in doc.get_pages().into_iter().enumerate() {
        let content = lopdf::content::Content::decode(&doc.get_page_content(page)).unwrap();
        let (mut size, mut at) = (0.0f32, (0.0f32, 0.0f32));
        for op in content.operations {
            let a = &op.operands;
            match op.operator.as_str() {
                "Tf" => size = num(&a[1]),
                "Tm" => at = (num(&a[4]), num(&a[5])),
                "Tj" => {
                    if let lopdf::Object::String(b, _) = &a[0] {
                        out.push(Run {
                            page: n + 1,
                            x: at.0,
                            y: at.1,
                            size,
                            text: b.iter().map(|x| crate::winansi::decode_byte(*x)).collect(),
                        });
                    }
                }
                _ => {}
            }
        }
    }
    out
}

fn pages_with(rs: &[Run], text: &str) -> Vec<usize> {
    let mut p: Vec<usize> = rs
        .iter()
        .filter(|r| r.text == text)
        .map(|r| r.page)
        .collect();
    p.dedup();
    p
}

fn branded(landscape: bool, blocks: Vec<Block>) -> Document {
    let mut doc = Document::new("Bands")
        .brand(BRAND_LINE)
        .landscape(landscape);
    for b in blocks {
        doc.push(b);
    }
    doc
}

fn band_table(title: &str, rows: usize, row_h: f32) -> StyledTable {
    let mut t = StyledTable::new(vec![Col::pt("No", 40.0), Col::weight("Item", 1.0)]);
    let mut band = TableBand::new(title);
    band.right = Some("MONDAY · 11 JANUARY 2027".into());
    band.facts = vec![
        FactLine::Runs(vec![
            FactRun::bold("2 scenes"),
            FactRun::plain(" · arranged in production order"),
        ]),
        FactLine::Runs(vec![
            FactRun::bold("4 pages total"),
            FactRun::plain(" · "),
            FactRun::pill("over the 11h target", Tone::Warn),
        ]),
        FactLine::Labelled {
            label: "Locations".into(),
            text: "Coast Radio Station".into(),
        },
    ];
    t.band = Some(band);
    t.boxed = true;
    for i in 0..rows {
        t.rows.push(Row::new(vec![
            Cell::text(Txt::new(format!("{i}"))),
            Cell::text(Txt::new(format!("{title} row {i:02}"))).item(CellItem::Gap(row_h)),
        ]));
    }
    t
}

#[test]
fn a_band_repeats_on_every_page_and_says_continued_after_the_first() {
    let pdf = branded(
        false,
        vec![Block::Styled(band_table("Shoot Day 5", 40, 30.0))],
    )
    .to_pdf();
    let rs = runs(&pdf.bytes);
    assert!(pdf.pages >= 3, "{} pages", pdf.pages);
    assert_eq!(pages_with(&rs, "Shoot Day 5").len(), pdf.pages);
    for page in 1..=pdf.pages {
        let on: Vec<&str> = rs
            .iter()
            .filter(|r| r.page == page)
            .map(|r| r.text.as_str())
            .collect();
        assert!(on.contains(&"2 scenes"), "facts on page {page}");
        assert!(on.contains(&"over the 11h target"), "pill on page {page}");
        assert!(on.contains(&"LOCATIONS"), "locations on page {page}");
        assert_eq!(
            on.contains(&" · continued"),
            page > 1,
            "continued marker on page {page}"
        );
    }
}

#[test]
fn a_table_that_fits_one_page_but_not_the_space_left_moves_whole() {
    let mut first = band_table("Day A", 6, 40.0);
    first.keep_whole = true;
    let mut second = band_table("Day B", 6, 40.0);
    second.keep_whole = true;
    // Each day is about 400 pt: the second one cannot share the page and must not split.
    let pdf = branded(false, vec![Block::Styled(first), Block::Styled(second)]).to_pdf();
    let rs = runs(&pdf.bytes);
    assert_eq!(pdf.pages, 2);
    assert_eq!(pages_with(&rs, "Day A"), vec![1]);
    assert_eq!(pages_with(&rs, "Day B"), vec![2]);
    assert!(
        !rs.iter().any(|r| r.text == " · continued"),
        "nothing split"
    );
}

#[test]
fn a_table_taller_than_a_page_starts_where_the_last_one_ended() {
    let short = band_table("Short", 2, 10.0);
    let mut long = band_table("Long", 30, 40.0);
    long.keep_whole = true;
    let pdf = branded(false, vec![Block::Styled(short), Block::Styled(long)]).to_pdf();
    let rs = runs(&pdf.bytes);
    assert_eq!(
        pages_with(&rs, "Long").first(),
        Some(&1),
        "starts on page 1"
    );
    assert!(pdf.pages >= 2);
}

#[test]
fn the_lead_cell_spans_two_columns_and_the_rest_follow() {
    let mut t = StyledTable::new(vec![
        Col::pt("A", 60.0),
        Col::pt("B", 60.0),
        Col::weight("C", 1.0),
    ]);
    t.header = HeaderStyle::None;
    let mut r = Row::new(vec![
        Cell::text(Txt::new("COMPANY MOVE LONG LABEL ONE LINE")),
        Cell::text(Txt::new("third")),
    ]);
    r.lead_span = 2;
    t.rows.push(r);
    t.rows.push(Row::new(vec![
        Cell::text(Txt::new("a")),
        Cell::text(Txt::new("b")),
        Cell::text(Txt::new("c")),
    ]));
    let pdf = branded(false, vec![Block::Styled(t)]).to_pdf();
    let rs = runs(&pdf.bytes);
    let find = |t: &str| {
        rs.iter()
            .find(|r| r.text == t)
            .unwrap_or_else(|| panic!("{t}"))
            .clone()
    };
    // The long label stays on one line (it needs more than one column) and "third" sits where
    // "c" does.
    let lead: Vec<&Run> = rs
        .iter()
        .filter(|r| r.text.starts_with("COMPANY"))
        .collect();
    assert_eq!(lead.len(), 1, "one line: {rs:?}");
    assert!((find("third").x - find("c").x).abs() < 0.01);
}

#[test]
fn a_labelled_item_hangs_its_text_beside_the_label() {
    let mut t = StyledTable::new(vec![Col::weight("A", 1.0)]);
    t.header = HeaderStyle::None;
    t.rows
        .push(Row::spanning(Cell::new().item(CellItem::Labelled {
            label: Txt::new("DAY NOTES").size(6.6).bold().track(0.5),
            text: Txt::new(
                "one two three four five six seven eight nine ten eleven twelve ".repeat(6),
            ),
        })));
    let pdf = branded(false, vec![Block::Styled(t)]).to_pdf();
    let rs = runs(&pdf.bytes);
    let label = rs.iter().find(|r| r.text == "DAY NOTES").unwrap();
    let body: Vec<&Run> = rs
        .iter()
        .filter(|r| r.text.contains("eleven") && r.size > 8.0)
        .collect();
    assert!(body.len() >= 2, "the text wraps");
    assert!(body.iter().all(|r| r.x > label.x + 30.0), "hanging indent");
    assert!((body[0].y - label.y).abs() < 0.5, "same first baseline");
}

// ------------------------------------------------------------------ matrix

fn code(text: &str) -> MatrixCode {
    MatrixCode::new(text, Some(Rgb::hex(0xb9e0ee)), pal::NAVY)
}

fn matrix(days: usize, rows: usize) -> Matrix {
    Matrix {
        title: "Cast commitments by shooting day".into(),
        hint: Some("test".into()),
        legend_label: "Code legend".into(),
        legend: vec![
            MatrixKey {
                swatch: code("W"),
                label: "Work".into(),
            },
            MatrixKey {
                swatch: code("SWF"),
                label: "Start, work and finish on the same day".into(),
            },
        ],
        line_labels: ["Shoot day".into(), "Date".into(), "Weekday".into()],
        frozen_labels: ["#".into(), "Cast member".into()],
        days: (0..days)
            .map(|i| MatrixDay {
                top: format!("S{:02}", i + 1),
                date: format!("{}", i % 28 + 1),
                weekday: "MON".into(),
                month: Some(("JANUARY 2027".into(), "JAN".into())),
                weekend: i % 7 == 5,
                off: i % 9 == 8,
            })
            .collect(),
        rows: (0..rows)
            .map(|r| MatrixRow {
                number: format!("{}", r + 1),
                name: format!("Actor {r:02}"),
                detail: "(CHARACTER)".into(),
                cells: (0..days).map(|_| code("W")).collect(),
                totals: vec!["7".into(), "Day 3".into()],
            })
            .collect(),
        counts: vec![MatrixCount {
            label: "Cast working".into(),
            values: vec!["3".into(); days],
            totals: vec!["9".into(), String::new()],
        }],
        totals: Some(MatrixTotals {
            heading: "Totals".into(),
            columns: vec![("Work".into(), 30.0, false), ("Start".into(), 30.0, true)],
        }),
        guide_every: 5,
    }
}

fn matrix_pdf(days: usize, rows: usize) -> (Vec<Run>, usize) {
    let pdf = branded(true, vec![Block::Matrix(matrix(days, rows))]).to_pdf();
    (runs(&pdf.bytes), pdf.pages)
}

#[test]
fn a_small_matrix_is_one_group_on_one_page() {
    let (rs, pages) = matrix_pdf(12, 10);
    assert_eq!(pages, 1);
    for d in 1..=12 {
        let n = rs.iter().filter(|r| r.text == format!("S{d:02}")).count();
        assert_eq!(n, 1, "day {d} once");
    }
    assert!(rs.iter().any(|r| r.text == "TOTALS"));
    assert!(!rs.iter().any(|r| r.text.contains("Columns group")));
}

#[test]
fn many_days_split_into_groups_that_repeat_the_cast_and_keep_one_column_width() {
    let (rs, pages) = matrix_pdf(70, 8);
    // 70 days beside a 60 pt totals block: four groups of 18, 18, 18 and 16 columns at 30 pt.
    assert_eq!(pages, 4, "one page per group");
    for d in 1..=70 {
        assert_eq!(
            rs.iter().filter(|r| r.text == format!("S{d:02}")).count(),
            1,
            "day {d} exactly once"
        );
    }
    for r in 0..8 {
        assert_eq!(
            rs.iter()
                .filter(|x| x.text == format!("Actor {r:02}"))
                .count(),
            4,
            "cast row {r} repeats in every group"
        );
    }
    // One column width everywhere: 30 pt between neighbouring day headers, in every group.
    let x = |t: &str| rs.iter().find(|r| r.text == t).unwrap().x;
    for (a, b) in [
        ("S02", "S01"),
        ("S20", "S19"),
        ("S38", "S37"),
        ("S56", "S55"),
    ] {
        assert!(((x(a) - x(b)) - 30.0).abs() < 0.2, "{a} {b}");
    }
    // The totals belong to the last group only.
    assert_eq!(rs.iter().filter(|r| r.text == "TOTALS").count(), 1);
    assert_eq!(pages_with(&rs, "TOTALS"), vec![4]);
    // Every group names itself and carries the legend.
    for p in 1..=4 {
        let hint = format!("test · Columns group {p} of 4");
        assert!(rs.iter().any(|r| r.page == p && r.text == hint), "{hint}");
        assert!(rs.iter().any(|r| r.page == p && r.text == "CODE LEGEND"));
    }
}

#[test]
fn many_rows_paginate_with_the_header_repeated_and_the_counts_kept_with_two_rows() {
    let (rs, pages) = matrix_pdf(10, 60);
    assert!(pages >= 3, "{pages} pages");
    for p in 1..=pages {
        assert!(
            rs.iter().any(|r| r.page == p && r.text == "CODE LEGEND"),
            "legend on {p}"
        );
        assert!(
            rs.iter().any(|r| r.page == p && r.text == "S01"),
            "day header on {p}"
        );
    }
    // The title says "continued" after the first page only.
    for p in 2..=pages {
        assert!(
            rs.iter().any(|r| r.page == p && r.text == " · continued"),
            "page {p}"
        );
    }
    assert!(!rs.iter().any(|r| r.page == 1 && r.text == " · continued"));
    // The counts and the last two rows share the last page; every row is printed once.
    let last = pages;
    for r in [58, 59] {
        assert!(
            rs.iter()
                .any(|x| x.page == last && x.text == format!("Actor {r:02}")),
            "row {r} on the last page"
        );
    }
    assert!(
        rs.iter()
            .any(|x| x.page == last && x.text == "CAST WORKING")
    );
    for r in 0..60 {
        assert_eq!(
            rs.iter()
                .filter(|x| x.text == format!("Actor {r:02}"))
                .count(),
            1
        );
    }
}

#[test]
fn nothing_in_the_matrix_is_set_below_the_type_floor_and_the_output_is_stable() {
    let (rs, _) = matrix_pdf(40, 30);
    for r in rs.iter().filter(|r| r.y > 66.0) {
        assert!(r.size >= 6.4 - 0.01, "{} is {} pt", r.text, r.size);
    }
    let a = branded(true, vec![Block::Matrix(matrix(40, 30))]).to_pdf();
    let b = branded(true, vec![Block::Matrix(matrix(40, 30))]).to_pdf();
    assert_eq!(a.bytes, b.bytes);
}

#[test]
fn a_long_cast_name_wraps_and_the_row_grows_without_touching_the_next_row() {
    let mut m = matrix(16, 3);
    m.rows[0].name = "A very long actor name that cannot possibly fit on one line".into();
    m.rows[0].detail = "(A CHARACTER WITH AN EQUALLY LONG NAME)".into();
    let pdf = branded(true, vec![Block::Matrix(m)]).to_pdf();
    let rs = runs(&pdf.bytes);
    let next = rs.iter().find(|r| r.text == "Actor 01").unwrap();
    let mut name: Vec<&Run> = rs
        .iter()
        .filter(|r| r.size == 7.6 && r.y > next.y + 2.0 && r.y < 560.0)
        .collect();
    name.sort_by(|a, b| b.y.partial_cmp(&a.y).unwrap());
    let text: String = name
        .iter()
        .map(|r| r.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(text.contains("cannot possibly fit"), "{text}");
    assert!(text.contains("EQUALLY LONG NAME)"), "{text}");
    let mut lines: Vec<i32> = name.iter().map(|r| r.y.round() as i32).collect();
    lines.dedup();
    assert!(lines.len() >= 3, "wrapped over {} lines", lines.len());
    // The last line of the long name sits above the next row's baseline by more than a line.
    assert!(name.last().unwrap().y - next.y > 9.0, "no overlap");
}

// ------------------------------------------------------------------ breakdown and budget items

fn plain_entry(name: &str) -> Entry {
    Entry {
        name: Txt::new(name).size(8.2).bold(),
        note: None,
        tag: None,
        tint: None,
    }
}

fn entry_table(cols: u8, entries: Vec<Entry>) -> StyledTable {
    let mut t = StyledTable::new(vec![Col::pt("Label", 92.0), Col::weight("Entries", 1.0)]);
    t.header = HeaderStyle::None;
    t.pad_x = 9.0;
    t.rows.push(Row::new(vec![
        Cell::text(Txt::new("PROPS")),
        Cell::new().item(CellItem::Entries {
            cols,
            gap: 14.0,
            entries,
        }),
    ]));
    t
}

#[test]
fn entries_flow_top_to_bottom_in_even_columns_without_splitting_an_entry() {
    let names: Vec<String> = (1..=7).map(|i| format!("Element {i}")).collect();
    let t = entry_table(2, names.iter().map(|n| plain_entry(n)).collect());
    let pdf = branded(false, vec![Block::Styled(t)]).to_pdf();
    let rs = runs(&pdf.bytes);
    let at = |t: &str| rs.iter().find(|r| r.text == t).unwrap().clone();
    // Seven equal entries make 4 + 3: the first column holds 1 to 4, the second 5 to 7.
    let x1 = at("Element 1").x;
    let x5 = at("Element 5").x;
    assert!(x5 > x1 + 100.0, "second column starts at {x5}");
    for n in [2, 3, 4] {
        assert!((at(&format!("Element {n}")).x - x1).abs() < 0.01);
    }
    for n in [6, 7] {
        assert!((at(&format!("Element {n}")).x - x5).abs() < 0.01);
    }
    assert!(at("Element 2").y < at("Element 1").y);
    assert!((at("Element 5").y - at("Element 1").y).abs() < 0.01);
}

#[test]
fn a_tall_entry_sets_the_split_so_the_columns_stay_as_even_as_the_order_allows() {
    let mut es = vec![plain_entry("One")];
    es.push(Entry {
        note: Some(Txt::new("a note ".repeat(30)).size(7.6)),
        ..plain_entry("Two")
    });
    es.push(plain_entry("Three"));
    es.push(plain_entry("Four"));
    let pdf = branded(false, vec![Block::Styled(entry_table(2, es))]).to_pdf();
    let rs = runs(&pdf.bytes);
    let x = |t: &str| rs.iter().find(|r| r.text == t).unwrap().x;
    // The tall entry shares its column with "One"; "Three" and "Four" go to the other column.
    assert!((x("One") - x("Two")).abs() < 0.01);
    assert!((x("Three") - x("Four")).abs() < 0.01);
    assert!(x("Three") > x("One") + 100.0);
}

#[test]
fn a_tag_sits_in_a_fixed_gutter_at_the_column_edge_and_names_wrap_before_it() {
    let tagged = |name: &str, tag: &str| Entry {
        tag: Some(Chip::label(tag, Tone::Info)),
        ..plain_entry(name)
    };
    let long = "Long name of an element that has to wrap before the tag. ".repeat(3);
    let t = entry_table(
        1,
        vec![tagged("Short", "Manual"), tagged(&long, "Suggested")],
    );
    let pdf = branded(false, vec![Block::Styled(t)]).to_pdf();
    let rs = runs(&pdf.bytes);
    let tag = |t: &str| {
        rs.iter()
            .find(|r| r.size == 6.6 && r.text == t)
            .unwrap_or_else(|| panic!("{t}"))
            .clone()
    };
    let (suggested, manual) = (tag("SUGGESTED"), tag("MANUAL"));
    // Both tags end at the column's right edge (the chip text has 5 pt of padding to its edge).
    let end = |r: &Run| r.x + text_width(&r.text, r.size, true) + 0.5 * r.text.len() as f32 + 5.0;
    assert!((end(&suggested) - end(&manual)).abs() < 0.1);
    assert!(
        (end(&suggested) - (46.0 + 520.0 - 9.0)).abs() < 0.2,
        "{}",
        end(&suggested)
    );
    // Names end before the widest tag starts.
    let gutter_left = suggested.x - 5.0;
    for r in rs.iter().filter(|r| r.size == 8.2) {
        assert!(
            r.x + text_width(&r.text, 8.2, true) <= gutter_left + 0.5,
            "{} runs into the gutter",
            r.text
        );
    }
    assert!(
        rs.iter().filter(|r| r.size == 8.2).count() >= 3,
        "the long name wraps"
    );
}

#[test]
fn a_tinted_entry_draws_its_tint_and_bar() {
    let mut e = plain_entry("Suggested one");
    e.tint = Some((pal::AMBER_FILL, pal::ORANGE));
    let flat = branded(
        false,
        vec![Block::Styled(entry_table(
            1,
            vec![plain_entry("Suggested one")],
        ))],
    )
    .to_pdf();
    let tinted = branded(false, vec![Block::Styled(entry_table(1, vec![e]))]).to_pdf();
    assert_ne!(tinted.bytes, flat.bytes);
}

fn head_table(rows: usize, row_h: f32) -> StyledTable {
    let mut t = StyledTable::new(vec![Col::pt("Aa", 60.0), Col::weight("Bb", 1.0)]);
    t.header = HeaderStyle::Panel;
    t.boxed = true;
    let mut h = TableHead::new("INT. A LONG ROOM - NIGHT");
    h.eyebrow = Some("Breakdown scene".into());
    h.tile = Some(Tile {
        top: Some("SCENE".into()),
        text: "7".into(),
        fill: pal::NAVY,
        line: None,
        color: pal::WHITE,
        w: 38.0,
    });
    h.right = Some(Txt::new("12 elements").size(9.0).bold());
    h.chip = Some(Chip::label("3 suggested", Tone::Warn));
    t.head = Some(Box::new(h));
    for i in 0..rows {
        t.rows.push(Row::new(vec![
            Cell::text(Txt::new(format!("r{i}"))),
            Cell::text(Txt::new(format!("row {i:02}"))).item(CellItem::Gap(row_h)),
        ]));
    }
    t
}

#[test]
fn a_table_head_repeats_on_every_page_with_continued_after_the_first() {
    let pdf = branded(false, vec![Block::Styled(head_table(40, 30.0))]).to_pdf();
    let rs = runs(&pdf.bytes);
    assert!(pdf.pages >= 3);
    assert_eq!(pages_with(&rs, "INT. A LONG ROOM - NIGHT").len(), pdf.pages);
    for page in 1..=pdf.pages {
        let on = |t: &str| rs.iter().any(|r| r.page == page && r.text == t);
        assert!(on("SCENE") && on("7"), "tile on page {page}");
        assert!(
            on("12 elements") && on("3 SUGGESTED"),
            "counts on page {page}"
        );
        assert!(on("AA") && on("BB"), "column labels on page {page}");
        assert_eq!(
            on(" · continued"),
            page > 1,
            "continued mark on page {page}"
        );
    }
}

#[test]
fn a_head_table_keeps_its_head_with_at_least_two_rows() {
    // The filler leaves too little room for the head and two rows: the table moves whole.
    let mut filler = StyledTable::new(vec![Col::weight("A", 1.0)]);
    filler.header = HeaderStyle::None;
    filler.rows.push(Row::new(vec![
        Cell::text(Txt::new("filler")).item(CellItem::Gap(600.0)),
    ]));
    let pdf = branded(
        false,
        vec![Block::Styled(filler), Block::Styled(head_table(3, 20.0))],
    )
    .to_pdf();
    let rs = runs(&pdf.bytes);
    assert_eq!(pages_with(&rs, "INT. A LONG ROOM - NIGHT"), vec![2]);
}

#[test]
fn a_name_wraps_under_a_hanging_indent_with_a_small_text_after_the_last_line() {
    let mut t = StyledTable::new(vec![Col::pt("A", 92.0), Col::weight("B", 1.0)]);
    t.header = HeaderStyle::None;
    t.rows.push(Row::new(vec![
        Cell::new().item(CellItem::Name {
            dot: Some(pal::category("Props")),
            text: Txt::new("EXTRAS / BACKGROUND").size(6.9).bold().track(0.5),
            small: Some(Txt::new("cont.").size(6.9)),
        }),
        Cell::text(Txt::new("x")),
    ]));
    let pdf = branded(false, vec![Block::Styled(t)]).to_pdf();
    let rs = runs(&pdf.bytes);
    let lines: Vec<&Run> = rs
        .iter()
        .filter(|r| r.size == 6.9 && r.text != "cont.")
        .collect();
    assert_eq!(lines.len(), 2, "two lines: {lines:?}");
    assert!((lines[0].x - lines[1].x).abs() < 0.01, "hanging indent");
    // The second line leaves no room beside it: the small text goes under it.
    let cont = rs.iter().find(|r| r.text == "cont.").unwrap();
    assert!(cont.y < lines[1].y - 5.0, "under the last line");
    assert!((cont.x - lines[1].x).abs() < 0.01);
}

#[test]
fn a_meter_puts_the_number_at_the_right_edge_and_inset_right_keeps_room() {
    let mut t = StyledTable::new(vec![
        Col::weight("A", 1.0),
        Col::pt("B", 100.0).align(Align::Right),
    ]);
    t.header = HeaderStyle::None;
    t.rows.push(Row::new(vec![
        Cell::text(Txt::new("a")),
        Cell::new().item(CellItem::Meter {
            share: 0.5,
            bar_w: 30.0,
            color: pal::NAVY,
            value: Txt::new("12").size(9.6).bold(),
        }),
    ]));
    t.rows.push(Row::new(vec![
        Cell::text(Txt::new("b")),
        Cell::text(Txt::new("1,500").size(8.4).align(Align::Right)).inset_right(10.0),
    ]));
    t.rows.push(Row::new(vec![
        Cell::text(Txt::new("c")),
        Cell::text(Txt::new("1,500").size(8.4).align(Align::Right)),
    ]));
    let pdf = branded(false, vec![Block::Styled(t)]).to_pdf();
    let rs = runs(&pdf.bytes);
    let col_right = 46.0 + 520.0 - 7.0;
    let twelve = rs.iter().find(|r| r.text == "12").unwrap();
    assert!((twelve.x + text_width("12", 9.6, true) - col_right).abs() < 0.1);
    let amounts: Vec<&Run> = rs.iter().filter(|r| r.text == "1,500").collect();
    assert_eq!(amounts.len(), 2);
    assert!(
        (amounts[1].x - amounts[0].x - 10.0).abs() < 0.1,
        "the inset moves it left"
    );
}

#[test]
fn a_continued_header_replaces_the_header_cells_after_the_first_page() {
    let mut t = StyledTable::new(vec![
        Col::weight("A", 1.0),
        Col::pt("B", 80.0).align(Align::Right),
    ]);
    t.header = HeaderStyle::Tint;
    let band = |suffix: &str| {
        vec![
            Cell::new().item(CellItem::Name {
                dot: Some(pal::category("Cast")),
                text: Txt::new("Cast").size(10.0).bold(),
                small: Some(Txt::new(format!("40 lines{suffix}")).size(7.4)),
            }),
            Cell::text(Txt::new("PLANNED").size(7.0).bold()),
        ]
    };
    t.header_cells = Some(band(""));
    t.header_continued = Some(band(" · continued"));
    for i in 0..40 {
        t.rows.push(Row::new(vec![
            Cell::text(Txt::new(format!("line {i}"))).item(CellItem::Gap(30.0)),
            Cell::text(Txt::new("1")),
        ]));
    }
    let pdf = branded(false, vec![Block::Styled(t)]).to_pdf();
    let rs = runs(&pdf.bytes);
    assert!(pdf.pages >= 3);
    assert_eq!(pages_with(&rs, "40 lines"), vec![1]);
    assert_eq!(pages_with(&rs, "40 lines · continued").len(), pdf.pages - 1);
    assert_eq!(pages_with(&rs, "PLANNED").len(), pdf.pages);
}

#[test]
fn a_section_kicker_prints_before_the_heading() {
    let doc = branded(
        false,
        vec![Block::Section(
            Section::new("Catalog", Some("hint".into())).kicker("Part 2"),
        )],
    );
    let pdf = doc.to_pdf();
    let rs = runs(&pdf.bytes);
    let k = rs.iter().find(|r| r.text == "PART 2").unwrap();
    let h = rs.iter().find(|r| r.text == "CATALOG").unwrap();
    assert!(k.x < h.x && (k.y - h.y).abs() < 0.5);
}
