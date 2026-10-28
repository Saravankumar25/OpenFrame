//! Page furniture and pagination rules of the PDF design system (sections 2, 4 and 9): footer with
//! the lossless logo, bottom margin, running header, table titles that repeat, headings that stay
//! with their content, and groups kept together.

use super::*;

/// One run of text as it sits on the page (points from the bottom left).
#[derive(Debug, Clone)]
struct Run {
    x: f32,
    y: f32,
    size: f32,
    text: String,
}

#[derive(Default, Debug)]
struct PageDump {
    runs: Vec<Run>,
    /// Image placements: x, y, width, height.
    images: Vec<[f32; 4]>,
    /// Bottom coordinates of every drawn shape and line.
    ys: Vec<f32>,
}

fn dump(bytes: &[u8]) -> Vec<PageDump> {
    let doc = lopdf::Document::load_mem(bytes).expect("PDF parses");
    let num = |o: &lopdf::Object| o.as_float().unwrap_or(0.0);
    doc.get_pages()
        .into_values()
        .map(|page| {
            let content = lopdf::content::Content::decode(&doc.get_page_content(page)).unwrap();
            let mut d = PageDump::default();
            let (mut size, mut at) = (0.0f32, (0.0f32, 0.0f32));
            for op in content.operations {
                let a = &op.operands;
                match op.operator.as_str() {
                    "Tf" => size = num(&a[1]),
                    "Tm" => at = (num(&a[4]), num(&a[5])),
                    "Tj" => {
                        if let lopdf::Object::String(b, _) = &a[0] {
                            d.runs.push(Run {
                                x: at.0,
                                y: at.1,
                                size,
                                text: b.iter().map(|x| crate::winansi::decode_byte(*x)).collect(),
                            });
                        }
                    }
                    "cm" => d
                        .images
                        .push([num(&a[4]), num(&a[5]), num(&a[0]), num(&a[3])]),
                    "re" | "m" | "l" => d.ys.push(num(&a[1])),
                    "c" => d.ys.extend([num(&a[1]), num(&a[3]), num(&a[5])]),
                    _ => {}
                }
            }
            d
        })
        .collect()
}

fn is_footer(r: &Run) -> bool {
    r.text == BRAND_LINE || r.text.starts_with("Page ")
}

fn on_page(pages: &[PageDump], needle: &str) -> Vec<usize> {
    pages
        .iter()
        .enumerate()
        .filter(|(_, p)| p.runs.iter().any(|r| r.text.contains(needle)))
        .map(|(i, _)| i)
        .collect()
}

fn lines_table(title: Option<&str>, rows: usize) -> StyledTable {
    let mut t = StyledTable::new(vec![Col::pt("No", 40.0), Col::weight("Item", 1.0)]);
    t.title = title.map(str::to_string);
    for i in 0..rows {
        t.rows.push(Row::new(vec![
            Cell::text(Txt::new(format!("{i}"))),
            Cell::text(Txt::new(format!("Row {i:03} of the table"))),
        ]));
    }
    t
}

/// Rows `row_h` points tall, tagged "<tag> <n>".
fn tall_table(tag: &str, rows: usize, row_h: f32) -> StyledTable {
    let mut t = StyledTable::new(vec![Col::weight("Item", 1.0)]);
    for i in 0..rows {
        t.rows.push(Row::new(vec![
            Cell::text(Txt::new(format!("{tag} {i}"))).item(CellItem::Gap(row_h)),
        ]));
    }
    t
}

fn branded(blocks: Vec<Block>) -> Document {
    let mut doc = Document::new("Furniture").brand(BRAND_LINE);
    for b in blocks {
        doc.push(b);
    }
    doc
}

#[test]
fn footer_embeds_the_logo_once_losslessly_with_a_soft_mask() {
    let pdf = branded(vec![Block::Styled(lines_table(Some("Items"), 160))]).to_pdf();
    assert!(pdf.pages >= 3, "{} pages", pdf.pages);
    let doc = lopdf::Document::load_mem(&pdf.bytes).unwrap();
    let mut logos = 0;
    let mut masks = 0;
    for obj in doc.objects.values() {
        if let lopdf::Object::Stream(s) = obj {
            let is_image = s.dict.get(b"Subtype").and_then(|o| o.as_name()).ok() == Some(b"Image");
            if !is_image {
                continue;
            }
            assert_eq!(
                s.dict.get(b"Filter").and_then(|o| o.as_name()).ok(),
                Some(&b"FlateDecode"[..]),
                "lossless, never JPEG"
            );
            if s.dict.has(b"SMask") {
                assert_eq!(s.dict.get(b"Width").unwrap().as_i64().unwrap(), 699);
                assert_eq!(s.dict.get(b"Height").unwrap().as_i64().unwrap(), 163);
                logos += 1;
            } else {
                masks += 1;
            }
        }
    }
    assert_eq!(
        (logos, masks),
        (1, 1),
        "one logo object and its mask for the whole file"
    );
    // Every page draws it, 17 pt high with its left edge on the margin and its foot 33 pt up.
    for (i, page) in dump(&pdf.bytes).iter().enumerate() {
        let [x, y, w, h] = page.images[0];
        assert_eq!((x, y, h), (46.0, 33.0, 17.0), "page {}", i + 1);
        assert!((w - 17.0 * 699.0 / 163.0).abs() < 0.01, "ratio kept: {w}");
    }
}

#[test]
fn footer_has_brand_line_and_real_page_numbers_on_every_page() {
    let pdf = branded(vec![Block::Styled(lines_table(None, 160))]).to_pdf();
    let pages = dump(&pdf.bytes);
    assert_eq!(pages.len(), pdf.pages);
    for (i, page) in pages.iter().enumerate() {
        let brand = page
            .runs
            .iter()
            .find(|r| r.text == BRAND_LINE)
            .expect("brand line");
        assert_eq!((brand.x, brand.y, brand.size), (46.0, 24.0, 6.8));
        let label = format!("Page {} of {}", i + 1, pdf.pages);
        let num = page
            .runs
            .iter()
            .find(|r| r.text == label)
            .expect("page number");
        assert_eq!((num.y, num.size), (24.0, 7.4));
        let right = num.x + text_width_pt(&label, 7.4, false);
        assert!((right - 566.0).abs() < 0.01, "right edge {right}");
        // Nothing else in the footer: no project text, no rule.
        assert_eq!(page.runs.iter().filter(|r| is_footer(r)).count(), 2);
        assert!(
            page.runs
                .iter()
                .filter(|r| !is_footer(r))
                .all(|r| r.y >= BOTTOM_MARGIN)
        );
    }
}

#[test]
fn content_never_reaches_below_the_bottom_margin() {
    let long = "A long synopsis that wraps inside its column. ".repeat(8);
    let mut cards = Vec::new();
    for i in 0..14 {
        cards.push(Card {
            head: Some(CardHead::new(format!("CARD {i}"))),
            parts: vec![CardPart::Fields(vec![Field::new("Note", long.clone())])],
            ..Card::new()
        });
    }
    let doc = branded(vec![
        Block::Cards(Cards::new(2, cards)),
        Block::Styled(lines_table(Some("Items"), 90)),
        Block::Grid(Grid {
            columns: 3,
            frame_ratio: 0.5625,
            cells: (0..24)
                .map(|i| GridCell {
                    placeholder: Some("No image".into()),
                    title: Some(format!("Panel {i}")),
                    ..GridCell::default()
                })
                .collect(),
        }),
    ]);
    let pdf = doc.to_pdf();
    assert!(pdf.pages >= 4);
    for (i, page) in dump(&pdf.bytes).iter().enumerate() {
        for y in &page.ys {
            // The logo is an image, not a shape; every shape and rule stays above 66 pt.
            assert!(
                *y >= BOTTOM_MARGIN - 0.01,
                "page {} draws at y = {y}",
                i + 1
            );
        }
        for r in page.runs.iter().filter(|r| !is_footer(r)) {
            assert!(r.y >= BOTTOM_MARGIN, "page {} text at y = {}", i + 1, r.y);
        }
    }
}

#[test]
fn unbranded_documents_keep_their_old_page_body() {
    let mut doc = Document::new("Manuscript");
    for i in 0..120 {
        doc.push(Block::Paragraph(format!("Line {i}")));
    }
    let pages = dump(&doc.to_pdf().bytes);
    assert!(pages.len() > 1);
    for page in &pages {
        let body = page.runs.iter().filter(|r| r.text.starts_with("Line "));
        assert!(body.into_iter().all(|r| r.y >= PLAIN_BOTTOM_MARGIN));
    }
}

#[test]
fn rendering_twice_gives_the_same_bytes() {
    let doc = branded(vec![
        Block::Section(Section::new("Items", None)),
        Block::Styled(lines_table(Some("Items"), 120)),
    ])
    .running_header(Some("LOW TIDE RADIO · SHOOT DAY 01".into()));
    assert_eq!(doc.to_pdf().bytes, doc.to_pdf().bytes);
}

#[test]
fn running_header_sits_top_right_and_wraps_inside_the_margin() {
    let one = branded(vec![Block::Paragraph("Body".into())])
        .running_header(Some("LOW TIDE RADIO · SHOOT DAY 01 · REVISION 2".into()))
        .to_pdf();
    let page = &dump(&one.bytes)[0];
    let r = page
        .runs
        .iter()
        .find(|r| r.text.starts_with("LOW TIDE"))
        .unwrap();
    assert_eq!((r.size, r.y), (6.8, 792.0 - 24.0));
    let w = text_width_pt(&r.text, 6.8, false) + HEADER_TRACK * r.text.chars().count() as f32;
    assert!((r.x + w - 566.0).abs() < 0.05, "right aligned: {}", r.x + w);

    // Two lines stay inside the 46 pt top margin and the body starts below them.
    let words = "SHOOT DAY 01 LOCATION NAME ".repeat(7);
    let two = branded(vec![Block::Paragraph("Body".into())])
        .running_header(Some(words))
        .to_pdf();
    let page = &dump(&two.bytes)[0];
    let heads: Vec<&Run> = page
        .runs
        .iter()
        .filter(|r| r.size == 6.8 && r.y > 740.0)
        .collect();
    assert_eq!(heads.len(), 2, "wrapped to a second line");
    assert!(heads[1].y > 792.0 - 46.0, "second line inside the margin");
    let body = page.runs.iter().find(|r| r.text == "Furniture").unwrap();
    assert!(body.y < heads[1].y, "body below the header");
    // Without a header nothing is printed there.
    let none = branded(vec![Block::Paragraph("Body".into())]).to_pdf();
    assert!(dump(&none.bytes)[0].runs.iter().all(|r| r.y < 740.0));
}

#[test]
fn a_very_long_running_header_pushes_the_body_down_instead_of_colliding() {
    let words = "NOTE ".repeat(400);
    let pdf = branded(vec![Block::Paragraph("Body".into())])
        .running_header(Some(words))
        .to_pdf();
    let page = &dump(&pdf.bytes)[0];
    let lowest_head = page
        .runs
        .iter()
        .filter(|r| r.size == 6.8 && r.text.starts_with("NOTE"))
        .map(|r| r.y)
        .fold(f32::MAX, f32::min);
    let body = page.runs.iter().find(|r| r.text == "Furniture").unwrap();
    assert!(
        body.y + 12.0 < lowest_head,
        "title {} vs header {}",
        body.y,
        lowest_head
    );
}

#[test]
fn a_table_title_repeats_with_continued_after_the_first_page() {
    let pdf = branded(vec![Block::Styled(lines_table(
        Some("Scene schedule"),
        160,
    ))])
    .to_pdf();
    let pages = dump(&pdf.bytes);
    assert!(pages.len() >= 3);
    for (i, page) in pages.iter().enumerate() {
        assert!(
            page.runs.iter().any(|r| r.text == "SCENE SCHEDULE"),
            "title on page {}",
            i + 1
        );
        let continued = page.runs.iter().any(|r| r.text == " · continued");
        assert_eq!(
            continued,
            i > 0,
            "continued only after page 1 (page {})",
            i + 1
        );
        // Title above the column labels above the first row.
        let y = |t: &str| page.runs.iter().find(|r| r.text == t).map(|r| r.y);
        assert!(y("SCENE SCHEDULE").unwrap() > y("ITEM").unwrap());
    }
}

#[test]
fn a_table_never_starts_with_a_lone_row_or_ends_with_one() {
    // Room for the title, header and exactly one row at the foot of page 1.
    let blocks = vec![
        Block::Spacer(600.0),
        Block::Styled(lines_table(Some("Items"), 3)),
    ];
    let pages = dump(&branded(blocks).to_pdf().bytes);
    let first = on_page(&pages, "Row 000 of the table");
    let second = on_page(&pages, "Row 001 of the table");
    assert_eq!(first, second, "the first two rows travel with the title");

    // The last page never holds a single row when the row before it can move.
    for rows in 40..70 {
        let pages = dump(
            &branded(vec![Block::Styled(lines_table(Some("Items"), rows))])
                .to_pdf()
                .bytes,
        );
        if pages.len() > 1 {
            let last = &pages[pages.len() - 1];
            let n = last
                .runs
                .iter()
                .filter(|r| r.text.starts_with("Row "))
                .count();
            assert!(n >= 2, "{rows} rows leave {n} row on the last page");
        }
    }
}

#[test]
fn a_heading_is_never_the_last_thing_on_a_page() {
    // Leave about 90 pt: the heading fits, the tall rows after it do not.
    let blocks = vec![
        Block::Spacer(560.0),
        Block::Heading {
            text: "KEEP ME WITH THE TABLE".into(),
            level: 1,
        },
        Block::Styled(tall_table("Tall", 2, 110.0)),
    ];
    let pages = dump(&branded(blocks).to_pdf().bytes);
    assert_eq!(
        on_page(&pages, "KEEP ME WITH THE TABLE"),
        on_page(&pages, "Tall 0")
    );
    assert_eq!(on_page(&pages, "KEEP ME WITH THE TABLE"), vec![1]);

    // The same for a section heading followed by a card too tall for the space left.
    let card = Card {
        head: Some(CardHead::new("BIG CARD")),
        parts: vec![CardPart::Fields(vec![Field::new(
            "Note",
            "word ".repeat(400),
        )])],
        ..Card::new()
    };
    let blocks = vec![
        Block::Spacer(520.0),
        Block::Section(Section::new("Locations", None)),
        Block::Cards(Cards::new(1, vec![card])),
    ];
    let pages = dump(&branded(blocks).to_pdf().bytes);
    assert_eq!(on_page(&pages, "LOCATIONS"), on_page(&pages, "BIG CARD"));
}

#[test]
fn a_keep_together_group_moves_whole_or_starts_fresh_when_taller_than_a_page() {
    let group = |rows: usize, row_h: f32| {
        Block::Together(vec![
            Block::Section(Section::new("Group", None)),
            Block::Styled(tall_table("Item", rows, row_h)),
        ])
    };
    // Fits a page but not the space left: the whole group moves to page 2.
    let pages = dump(
        &branded(vec![Block::Spacer(540.0), group(3, 60.0)])
            .to_pdf()
            .bytes,
    );
    assert_eq!(on_page(&pages, "GROUP"), vec![1]);
    assert_eq!(on_page(&pages, "Item 0"), vec![1]);
    assert_eq!(on_page(&pages, "Item 2"), vec![1]);
    // Fits the space left: it stays where it is.
    let pages = dump(
        &branded(vec![Block::Spacer(100.0), group(2, 40.0)])
            .to_pdf()
            .bytes,
    );
    assert_eq!(on_page(&pages, "Item 1"), vec![0]);
    // Taller than a page: starts on a fresh page and then splits between rows.
    let pages = dump(
        &branded(vec![Block::Spacer(540.0), group(5, 250.0)])
            .to_pdf()
            .bytes,
    );
    assert_eq!(on_page(&pages, "GROUP"), vec![1]);
    assert_eq!(on_page(&pages, "Item 0"), vec![1]);
    assert!(on_page(&pages, "Item 4")[0] >= 2);
    // A group that meets a fresh page does not push out an empty page first.
    let pages = dump(
        &branded(vec![Block::PageBreak, group(5, 250.0)])
            .to_pdf()
            .bytes,
    );
    assert_eq!(on_page(&pages, "GROUP"), vec![1]);
    assert_eq!(on_page(&pages, "Item 0"), vec![1]);
}

#[test]
fn group_header_and_subtotal_rows_keep_their_neighbours() {
    // Rows of 40 pt: 14 fit under the title on page 1 (680 pt, minus title and header). Put a
    // group header on row 13 and a subtotal after the last row so both rules bite at the break.
    let mut t = StyledTable::new(vec![Col::weight("Item", 1.0)]);
    t.title = Some("Budget".into());
    for i in 0..20 {
        let mut row = Row::new(vec![
            Cell::text(Txt::new(format!("Line {i:02}"))).item(CellItem::Gap(30.0)),
        ]);
        row.keep_next = i == 13;
        row.keep_prev = if i == 19 { 2 } else { 0 };
        t.rows.push(row);
    }
    let pages = dump(&branded(vec![Block::Styled(t)]).to_pdf().bytes);
    assert_eq!(
        on_page(&pages, "Line 13"),
        on_page(&pages, "Line 14"),
        "group header with its first row"
    );
    let last = on_page(&pages, "Line 19");
    assert_eq!(last, on_page(&pages, "Line 18"));
    assert_eq!(
        last,
        on_page(&pages, "Line 17"),
        "total with the two rows it sums up"
    );
}

#[test]
fn nothing_is_printed_below_6_4_points() {
    let mut tile = Tile::new("07", pal::TEAL, pal::WHITE);
    tile.top = Some("SCENE".into());
    tile.w = 34.0;
    let mut head = CardHead::new("A card");
    head.tile = Some(tile);
    head.eyebrow = Some("Eyebrow".into());
    let mut item = GroupItem::new("Rifle");
    item.tag = Some(Chip::label("Suggested", Tone::Warn));
    let card = Card {
        head: Some(head),
        parts: vec![CardPart::Groups {
            groups: vec![Group {
                label: "Props".into(),
                accent: pal::ORANGE,
                items: vec![item],
            }],
            cols: 1,
        }],
        ..Card::new()
    };
    let mut table = lines_table(Some("Items"), 3);
    table.rows[0].cells[0] = Cell::new().item(CellItem::Missing("Needs input".into()));
    let doc = branded(vec![
        Block::Masthead(Masthead::new("Title")),
        Block::Cards(Cards::new(1, vec![card])),
        Block::Styled(table),
    ]);
    let mut seen = 0;
    for page in dump(&doc.to_pdf().bytes) {
        for r in &page.runs {
            seen += 1;
            assert!(r.size >= 6.4, "{:?} is {} pt", r.text, r.size);
        }
    }
    assert!(seen > 10);
}

#[test]
fn tracked_labels_never_run_past_their_card() {
    // A long location name makes the label of a hospital card wrap; with the tracking the lines
    // must still fit the card's text width (255 pt wide card, 10 pt padding each side).
    let label = "Pozhikkara lighthouse and keeper's cottage, upper gallery and lamp room interiors \u{b7} Nearest hospital";
    let card = |l: &str| Card {
        parts: vec![CardPart::Labeled {
            label: l.into(),
            value: "District Hospital".into(),
            missing: false,
        }],
        ..Card::new()
    };
    let doc = branded(vec![Block::Cards(Cards::new(
        2,
        vec![card(label), card("Short")],
    ))]);
    let page = &dump(&doc.to_pdf().bytes)[0];
    let lines: Vec<&Run> = page.runs.iter().filter(|r| r.size == 7.0).collect();
    assert!(lines.len() >= 2, "{lines:?}");
    for r in lines {
        let w = text_width(&r.text, 7.0, true) + 0.7 * r.text.chars().count() as f32;
        assert!(w <= 235.0 + 0.1, "{:?} is {w} pt wide", r.text);
    }
}
