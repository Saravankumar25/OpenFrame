//! The Beat Board blocks and the card parts added for the moodboard (PDF design package v2):
//! a row split between pages keeps every word, a page that opens inside a container starts with
//! the strip, a long unbroken word never leaves the margin.

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

fn item(n: Option<usize>, heading: &str, text: &str, extras: Vec<BoardExtra>) -> BoardItem {
    BoardItem {
        number: n,
        beat: false,
        heading: heading.into(),
        text: text.into(),
        extras,
    }
}

fn doc_of(blocks: Vec<Block>) -> Document {
    let mut doc = Document::new("Board").brand(BRAND_LINE);
    for b in blocks {
        doc.push(b);
    }
    doc
}

fn cards(items: Vec<BoardItem>, strip: Option<&str>) -> Block {
    Block::Board(Board::Items(BoardItems {
        items,
        layout: BoardLayout::Cards { columns: 3 },
        parked: false,
        strip: strip.map(str::to_string),
    }))
}

fn words(n: usize) -> String {
    (1..=n).map(|i| format!("w{i} ")).collect()
}

#[test]
fn a_word_wider_than_the_line_is_broken_after_other_words_too() {
    let long = "W".repeat(80);
    for line in wrap_pt(&format!("short words {long} more"), 120.0, 9.0, false) {
        assert!(text_width(&line, 9.0, false) <= 120.5, "{line:?}");
    }
    assert_eq!(
        wrap_pt(&format!("a {long}"), 120.0, 9.0, false)
            .concat()
            .replace(' ', ""),
        format!("a{long}")
    );
}

#[test]
fn a_row_taller_than_a_page_continues_in_every_column_with_the_strip_and_no_word_lost() {
    // Column one holds a note of 1,600 words (several pages), column two a short note.
    let long = BoardExtra::Note(words(1600));
    let short = BoardExtra::Note("short note".into());
    let items = vec![
        item(Some(1), "INT. ONE - DAY", "First", vec![long]),
        item(Some(2), "INT. TWO - DAY", "Second", vec![short]),
        item(Some(3), "INT. THREE - DAY", "Third", vec![]),
    ];
    let pdf = doc_of(vec![
        Block::Masthead(Masthead::new("Board")),
        cards(items, Some("ACT ONE \u{203a} SEQUENCE: LONG")),
    ])
    .to_pdf();
    assert!(pdf.pages >= 4, "{} pages", pdf.pages);
    let r = runs(&pdf.bytes);
    let got: Vec<usize> = r
        .iter()
        .flat_map(|r| {
            r.text
                .split_whitespace()
                .filter_map(|w| w.strip_prefix('w').and_then(|n| n.parse().ok()))
                .collect::<Vec<usize>>()
        })
        .collect();
    assert_eq!(
        got,
        (1..=1600).collect::<Vec<_>>(),
        "every word once, in order"
    );
    // Every page after the first starts with the strip and its "continued" run.
    for page in 2..=pdf.pages {
        let top = r
            .iter()
            .filter(|x| x.page == page && x.y < 752.0 && x.y > 66.0)
            .map(|x| x.y)
            .fold(f32::MIN, f32::max);
        let first: Vec<&str> = r
            .iter()
            .filter(|x| x.page == page && (x.y - top).abs() < 2.0)
            .map(|x| x.text.as_str())
            .collect();
        assert!(
            first.iter().any(|t| t.contains("continued")),
            "page {page}: {first:?}"
        );
        assert!(first.iter().any(|t| t.starts_with("ACT ONE")), "{first:?}");
    }
    // The second column's short note is whole on the first page, the third card too.
    assert!(
        r.iter()
            .any(|x| x.page == 1 && x.text == "INT. THREE - DAY")
    );
}

#[test]
fn a_page_that_opens_with_a_heading_has_no_strip_and_text_stays_inside_the_margins() {
    let unbroken = "M".repeat(120);
    let items: Vec<BoardItem> = (1..=60)
        .map(|n| {
            item(
                Some(n),
                &format!("INT. PLACE {n} - DAY"),
                &format!("A description of card {n} with words that wrap. {unbroken}"),
                vec![BoardExtra::Comment(BoardComment {
                    status: "Open".into(),
                    author: "Ravi".into(),
                    body: "A question.".into(),
                    replies: vec![("Meera".into(), "An answer.".into())],
                })],
            )
        })
        .collect();
    let pdf = doc_of(vec![
        Block::Masthead(Masthead::new("Board")),
        Block::Together(vec![
            Block::Board(Board::Act {
                title: "Act One".into(),
                hint: Some("60 scene cards".into()),
                extras: vec![],
            }),
            cards(items[..3].to_vec(), None),
        ]),
        cards(items[3..].to_vec(), Some("ACT ONE")),
    ])
    .to_pdf();
    assert!(pdf.pages >= 3);
    let r = runs(&pdf.bytes);
    for x in r.iter().filter(|x| x.y > 66.0 && x.y < 752.0) {
        let w = text_width(&x.text, x.size, false);
        assert!(
            x.x >= 45.5 && x.x + w <= 566.5 + 12.0,
            "{:?} runs past a margin",
            x.text
        );
    }
    // Page 1 starts with the masthead, later pages with the strip.
    assert!(
        !r.iter()
            .any(|x| x.page == 1 && x.text.contains("continued"))
    );
    for page in 2..=pdf.pages {
        assert!(
            r.iter()
                .any(|x| x.page == page && x.text.contains("continued"))
        );
    }
}

#[test]
fn the_picture_card_part_fits_the_picture_in_the_frame_and_the_chip_is_centred() {
    let png = {
        // A 40 by 20 white PNG: wider than the 0.72 frame, so it touches the frame's sides.
        let img = image::RgbImage::from_pixel(40, 20, image::Rgb([255, 255, 255]));
        let mut b = Vec::new();
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut Cursor::new(&mut b), image::ImageFormat::Png)
            .unwrap();
        b
    };
    let pic = ReportImage::decode(&png).unwrap();
    let card = Card {
        parts: vec![
            CardPart::Picture {
                img: Some(pic),
                placeholder: None,
                ratio: 0.72,
            },
            CardPart::CenteredChip(Chip::label("Internal", Tone::Warn)),
        ],
        ..Card::new()
    };
    let gone = Card {
        parts: vec![CardPart::Picture {
            img: None,
            placeholder: Some("Image unavailable: a.png".into()),
            ratio: 0.72,
        }],
        ..Card::new()
    };
    let pdf = doc_of(vec![Block::Cards(Cards::new(2, vec![card, gone]))]).to_pdf();
    let doc = lopdf::Document::load_mem(&pdf.bytes).unwrap();
    let page = *doc.get_pages().values().next().unwrap();
    let content = lopdf::content::Content::decode(&doc.get_page_content(page)).unwrap();
    let mut pics = Vec::new();
    for op in content.operations {
        if op.operator == "cm" {
            let n = |i: usize| op.operands[i].as_float().unwrap_or(0.0);
            pics.push((n(0), n(3)));
        }
    }
    // The logo is a picture too (brand); the card picture is the one 2:1 wide.
    let card_pic = pics
        .iter()
        .find(|(w, h)| (w / h - 2.0).abs() < 0.01)
        .unwrap();
    let frame_w = (520.0 - 10.0) / 2.0 - 16.0;
    assert!(
        (card_pic.0 - frame_w).abs() < 0.05,
        "{card_pic:?} in a frame {frame_w} wide"
    );
    let r = runs(&pdf.bytes);
    assert!(r.iter().any(|x| x.text == "INTERNAL"));
    assert!(r.iter().any(|x| x.text == "Image unavailable: a.png"));
}

#[test]
fn a_huge_strip_and_a_huge_card_still_end_in_a_bounded_number_of_pages_with_all_text() {
    // A 2,000 character container name makes a strip of many lines unless it is bounded.
    let name = "THE LONG NAME ".repeat(145);
    let big = words(3000);
    let items = vec![
        item(
            Some(1),
            "INT. ONE - DAY",
            &big,
            vec![BoardExtra::Note(big.clone())],
        ),
        item(Some(2), "INT. TWO - DAY", "short", vec![]),
        item(Some(3), "INT. THREE - DAY", "short", vec![]),
        item(Some(4), "INT. FOUR - DAY", &big, vec![]),
    ];
    for layout in [
        BoardLayout::Cards { columns: 3 },
        BoardLayout::List { nested: true },
    ] {
        let pdf = doc_of(vec![
            Block::Masthead(Masthead::new("Board")),
            Block::Board(Board::Items(BoardItems {
                items: items.clone(),
                layout,
                parked: false,
                strip: Some(name.clone()),
            })),
        ])
        .to_pdf();
        assert!(pdf.pages > 2 && pdf.pages < 50, "{} pages", pdf.pages);
        let r = runs(&pdf.bytes);
        let got = r
            .iter()
            .flat_map(|x| x.text.split_whitespace())
            .filter(|w| w.starts_with('w') && w[1..].chars().all(|c| c.is_ascii_digit()))
            .count();
        // Card one prints its description and note, card four its description.
        assert_eq!(got, 3 * 3000);
        // The strip is two lines at most.
        let top: Vec<&Run> = r
            .iter()
            .filter(|x| x.page == 2 && x.size < 7.0 && x.y > 700.0)
            .collect();
        assert!(top.len() <= 3, "{} strip runs", top.len());
    }
}
