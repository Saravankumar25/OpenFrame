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
