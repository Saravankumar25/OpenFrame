//! Generic report documents: PDF (Helvetica, tables, page breaks), CSV, XLSX.

use openframe_import_export::report::{Block, Column, Document, Table};

fn catalog(rows: usize) -> Table {
    let mut t = Table::new(
        "Catalog",
        vec![
            Column::new("Item").weight(2.0),
            Column::new("Category"),
            Column::right("Qty"),
        ],
    );
    for i in 0..rows {
        t.push(vec![
            format!("Item {i} — umbrella, black, “vintage”"),
            "Props".into(),
            (i % 7).to_string(),
        ]);
    }
    t
}

#[test]
fn report_pdf_paginates_tables_and_is_readable() {
    let doc = Document::new("Production Catalog")
        .subtitle("Black Rain · exported snapshot")
        .with(Block::Heading {
            text: "Summary".into(),
            level: 1,
        })
        .with(Block::KeyValues(vec![
            ("Items".into(), "180".into()),
            ("Source".into(), "Draft 6 — Shooting Draft".into()),
        ]))
        .with(Block::Paragraph(
            "Suggested and confirmed items are listed separately.".into(),
        ))
        .with(Block::Table(catalog(180)))
        .with(Block::PageBreak)
        .with(Block::Heading {
            text: "Notes".into(),
            level: 2,
        })
        .with(Block::Paragraph("मीरा".into()));
    let out = doc.to_pdf();
    assert!(out.bytes.starts_with(b"%PDF-"));
    assert!(
        out.pages >= 4,
        "180 rows span several pages; got {}",
        out.pages
    );
    assert_eq!(
        out.replaced_chars, 4,
        "non-WinAnsi text is counted for an honest warning"
    );
    assert_eq!(
        doc.to_pdf_checked().unwrap_err().code_str(),
        "export.pdf_unsupported_characters"
    );
    let parsed = lopdf::Document::load_mem(&out.bytes).unwrap();
    assert_eq!(parsed.get_pages().len(), out.pages);
    let text = parsed.extract_text(&[1]).unwrap();
    assert!(text.contains("Production Catalog"));
    // The table header is repeated on continuation pages.
    let page3 = parsed.extract_text(&[3]).unwrap();
    assert!(page3.contains("Category"), "{page3}");
    assert!(page3.contains(&format!("Page 3 of {}", out.pages)));
}

#[test]
fn csv_has_bom_header_and_quoting() {
    let t = catalog(2);
    let bytes = t.to_csv().unwrap();
    assert!(bytes.starts_with(b"\xEF\xBB\xBF"));
    let s = String::from_utf8(bytes[3..].to_vec()).unwrap();
    let mut lines = s.lines();
    assert_eq!(lines.next().unwrap(), "Item,Category,Qty");
    assert_eq!(
        lines.next().unwrap(),
        "\"Item 0 — umbrella, black, “vintage”\",Props,0"
    );
}

#[test]
fn xlsx_is_a_valid_workbook_with_one_sheet_per_table() {
    let mut cast = Table::new("Cast/Crew", vec![Column::new("Name"), Column::new("Role")]);
    cast.push(vec!["Nisha".into(), "Director".into()]);
    let doc = Document::new("Report")
        .with(Block::Table(catalog(5)))
        .with(Block::Table(cast));
    let bytes = doc.to_xlsx().unwrap();
    assert!(bytes.starts_with(b"PK"));
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let names: Vec<String> = (0..zip.len())
        .map(|i| zip.by_index(i).unwrap().name().to_string())
        .collect();
    assert!(names.iter().any(|n| n == "xl/worksheets/sheet1.xml"));
    assert!(names.iter().any(|n| n == "xl/worksheets/sheet2.xml"));
    let mut wb = String::new();
    std::io::Read::read_to_string(&mut zip.by_name("xl/workbook.xml").unwrap(), &mut wb).unwrap();
    assert!(wb.contains("name=\"Catalog\""));
    assert!(wb.contains("name=\"CastCrew\""));
}

fn png(w: u32, h: u32) -> Vec<u8> {
    let img = image::RgbaImage::from_fn(w, h, |x, y| {
        image::Rgba([(x * 7) as u8, (y * 5) as u8, 120, 255])
    });
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png).unwrap();
    out.into_inner()
}

#[test]
fn pictures_section_bars_and_centered_header_render_into_a_valid_pdf() {
    use openframe_import_export::report::{Grid, GridCell, ReportImage};
    let big = ReportImage::decode(&png(2400, 1200)).unwrap();
    assert_eq!(
        (big.width, big.height),
        (1600, 800),
        "large pictures are scaled down"
    );
    let small = ReportImage::decode(&png(40, 30)).unwrap();
    let cells = vec![
        GridCell {
            image: Some(big),
            title: Some("Panel 1 · Shot 12A".into()),
            lines: vec!["Wide — Arjun enters".into()],
            ..Default::default()
        },
        GridCell {
            placeholder: Some("Blank panel".into()),
            title: Some("Panel 2".into()),
            notes: vec!["Note: hold".into()],
            ..Default::default()
        },
        GridCell {
            image: Some(small),
            title: Some("Panel 3".into()),
            ..Default::default()
        },
        GridCell {
            lines: vec!["Caption only".into()],
            ..Default::default()
        },
    ];
    let doc = Document::new("BLACK RAIN")
        .subtitle("SHOOT DAY 4 · MONDAY 14 JUNE 2027")
        .centered_header(true)
        .landscape(true)
        .footer("Black Rain · Call Sheet — Day 4")
        .with(Block::Centered {
            text: "CREW CALL: 18:00".into(),
            size: 13.0,
            bold: true,
        })
        .with(Block::SectionBar("Location".into()))
        .with(Block::Indented {
            text: "Scene Card 1".into(),
            level: 1,
            muted: false,
        })
        .with(Block::Note("Private notes excluded.".into()))
        .with(Block::Spacer(6.0))
        .with(Block::Grid(Grid {
            columns: 3,
            frame_ratio: 9.0 / 16.0,
            cells,
        }));
    assert!(doc.unsupported_chars().is_empty());
    let out = doc.to_pdf_strict("Replace them.").unwrap();
    let parsed = lopdf::Document::load_mem(&out.bytes).unwrap();
    assert_eq!(parsed.get_pages().len(), out.pages);
    let images = parsed
        .objects
        .values()
        .filter(|o| matches!(o, lopdf::Object::Stream(s) if s.dict.get(b"Subtype").ok().and_then(|v| v.as_name().ok()) == Some(b"Image".as_slice())))
        .count();
    assert_eq!(images, 2, "each picture is one JPEG image XObject");
    let text = parsed.extract_text(&[1]).unwrap();
    assert!(
        text.contains("CREW CALL: 18:00")
            && text.contains("LOCATION")
            && text.contains("Blank panel"),
        "{text}"
    );
    assert!(text.contains("Page 1 of"));

    // Unreadable bytes are a clear export error, never a panic.
    assert_eq!(
        ReportImage::decode(b"not an image").unwrap_err().code_str(),
        "export.image_unreadable"
    );
}

#[test]
fn strict_pdf_names_unprintable_characters_and_suggests_an_alternative() {
    let doc = Document::new("Moodboard").with(Block::Grid(openframe_import_export::report::Grid {
        columns: 2,
        frame_ratio: 0.75,
        cells: vec![openframe_import_export::report::GridCell {
            lines: vec!["मीरा".into()],
            ..Default::default()
        }],
    }));
    assert_eq!(doc.unsupported_chars(), vec!['म', 'ी', 'र', 'ा']);
    let e = doc
        .to_pdf_strict("Replace those characters in the captions.")
        .unwrap_err();
    assert_eq!(e.code_str(), "export.pdf_unsupported_characters");
    assert!(e.message.contains("“म” “ी” “र” “ा”"), "{}", e.message);
    assert!(
        e.message
            .ends_with("Replace those characters in the captions.")
    );
}
