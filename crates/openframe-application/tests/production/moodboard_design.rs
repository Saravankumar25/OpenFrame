//! Layout rules of the Moodboard PDF (PDF design package v2, Moodboard spec sections H and J),
//! read from the text runs and the picture placements of the real output. `write_*` writes the
//! samples and the stress boards to `release-out/pdf-samples` for the eye:
//!
//! ```text
//! node scripts/cargo-queued.mjs test -p openframe-application --test production \
//!     moodboard_design::write -- --ignored --nocapture
//! ```

use std::path::{Path, PathBuf};

use openframe_test_support::TestEnv;
use serde_json::{Value, json};

use super::pdf_samples::{id_of, still};

// ------------------------------------------------------------------ reading a PDF

#[derive(Debug, Clone)]
struct Run {
    page: usize,
    x: f32,
    y: f32,
    size: f32,
    text: String,
}

/// A picture as placed on its page: left, bottom, width, height.
#[derive(Debug, Clone, Copy)]
struct Pic {
    page: usize,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

struct Pdf {
    runs: Vec<Run>,
    pics: Vec<Pic>,
    pages: usize,
    width: f32,
}

fn read(path: &str) -> Pdf {
    let doc = lopdf::Document::load(path).expect("PDF parses");
    let pages = doc.get_pages();
    let num = |o: &lopdf::Object| o.as_float().unwrap_or(0.0);
    let (mut runs, mut pics) = (Vec::new(), Vec::new());
    let mut width = 0.0;
    for (n, (_, page)) in pages.iter().enumerate() {
        if let Ok(lopdf::Object::Array(b)) = doc
            .get_dictionary(*page)
            .and_then(|d| d.get(b"MediaBox"))
            .cloned()
        {
            width = num(&b[2]);
        }
        let content = lopdf::content::Content::decode(&doc.get_page_content(*page)).unwrap();
        let (mut x, mut y, mut size) = (0.0f32, 0.0f32, 0.0f32);
        for op in content.operations {
            let a = &op.operands;
            match op.operator.as_str() {
                "Tf" => size = num(&a[1]),
                "Tm" => (x, y) = (num(&a[4]), num(&a[5])),
                "cm" => pics.push(Pic {
                    page: n + 1,
                    x: num(&a[4]),
                    y: num(&a[5]),
                    w: num(&a[0]),
                    h: num(&a[3]),
                }),
                "Tj" => {
                    if let Some(lopdf::Object::String(b, _)) = a.first() {
                        runs.push(Run {
                            page: n + 1,
                            x,
                            y,
                            size,
                            text: b
                                .iter()
                                .map(|c| openframe_import_export::winansi::decode_byte(*c))
                                .collect(),
                        });
                    }
                }
                _ => {}
            }
        }
    }
    // The footer logo is the one picture in the bottom margin.
    pics.retain(|p| p.y > 60.0);
    Pdf {
        runs,
        pics,
        pages: pages.len(),
        width,
    }
}

impl Pdf {
    fn text(&self) -> String {
        self.runs
            .iter()
            .map(|r| r.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
    fn has(&self, t: &str) -> bool {
        self.runs.iter().any(|r| r.text == t)
    }
}

// ------------------------------------------------------------------ fixtures

const PAGE_H: f32 = 612.0;

fn export(env: &TestEnv, board: &str, mut args: Value, file: &str) -> (String, Pdf) {
    let path = env.dir.path().join(file).to_string_lossy().into_owned();
    args["moodboardId"] = json!(board);
    args["format"] = json!("pdf");
    args["path"] = json!(path);
    env.ok("moodboard.export_pdf", args);
    let pdf = read(&path);
    (path, pdf)
}

fn sample_dir() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repository root")
        .to_path_buf();
    let d = root.join("release-out").join("pdf-samples");
    std::fs::create_dir_all(&d).expect("sample folder");
    d
}

fn keep(path: &str, name: &str, pages: usize) {
    std::fs::copy(path, sample_dir().join(name)).expect("sample copied");
    println!("{name}: {pages} pages");
}

fn exec(env: &TestEnv, sql: &str) {
    env.core
        .project()
        .expect("project open")
        .store
        .with_writer(|c| {
            c.execute_batch(sql).expect("fixture sql");
            Ok(())
        })
        .unwrap();
}

fn board(env: &TestEnv, name: &str) -> String {
    id_of(&env.ok("moodboard.create", json!({ "name": name })))
}

/// An image on the board with its caption; returns the item id. Items are placed down the
/// canvas so the reading order is the order of the calls.
fn image(
    env: &TestEnv,
    board: &str,
    n: usize,
    png: &[u8],
    caption: Option<&str>,
    internal: bool,
) -> String {
    let p = env.write_file(&format!("img{n}.png"), png);
    let items = env.ok(
        "moodboard.add_images",
        json!({ "moodboardId": board, "paths": [p], "x": 20, "y": 24 + 80 * n as i64 }),
    );
    let id = id_of(&items[0]);
    if let Some(c) = caption {
        env.ok("moodboard.update_item", json!({ "id": id, "caption": c }));
    }
    if internal {
        env.ok(
            "moodboard.update_item",
            json!({ "id": id, "isPrivate": true }),
        );
    }
    id
}

fn note(env: &TestEnv, board: &str, text: &str, internal: bool) {
    env.ok(
        "moodboard.add_note",
        json!({ "moodboardId": board, "text": text, "isPrivate": internal }),
    );
}

const CAPTIONS: [&str; 4] = [
    "Low sun through monsoon haze",
    "Monsoon palette from the art department share",
    "Station at dusk: the hour we shoot scene 37",
    "",
];

/// The reference board: four pictures (one without a caption), a link, a note, an internal note
/// and internal board notes.
fn low_tide() -> (TestEnv, String) {
    let env = TestEnv::with_project("Low Tide Radio", "Feature Film");
    let b = board(&env, "Look of the film: monsoon night and the 1999 grade");
    for (i, c) in CAPTIONS.iter().enumerate() {
        image(
            &env,
            &b,
            i,
            &still(i, 600, 400),
            (!c.is_empty()).then_some(*c),
            false,
        );
    }
    env.ok(
        "moodboard.add_link",
        json!({ "moodboardId": b, "url": "www.example.com/monsoon-photography", "title": "Monsoon photography reference" }),
    );
    note(
        &env,
        &b,
        "Cold, wet, blue-grey, with one warm window in every scene.",
        false,
    );
    note(
        &env,
        &b,
        "Budget note for Vivek: balloon lights only on the beach night.",
        true,
    );
    env.ok(
        "moodboard.update_notes",
        json!({ "id": b, "notes": "Shared with the DOP and the design lead. Grade reference for the DI." }),
    );
    (env, b)
}

fn words(n: usize, seed: &str) -> String {
    let mut s = String::new();
    while s.chars().count() < n {
        s.push_str(seed);
        s.push(' ');
    }
    s.chars().take(n).collect::<String>().trim_end().to_string()
}

/// Twelve pictures of every shape and state (portrait, square, panorama, a missing file, an
/// unreadable file, no asset, an internal one, a very long caption), notes and a long link.
fn stress() -> (TestEnv, String) {
    let env = TestEnv::with_project("Harbour Lights", "Series");
    let b = board(&env, "Stress: twelve pictures");
    let shapes: [(u32, u32); 6] = [
        (640, 360),
        (400, 600),
        (500, 500),
        (1200, 300),
        (600, 400),
        (360, 640),
    ];
    for (i, (w, h)) in shapes.iter().enumerate() {
        image(
            &env,
            &b,
            i,
            &still(i, *w, *h),
            Some(&format!("Picture {} ({w} by {h})", i + 1)),
            i == 2,
        );
    }
    let missing = image(
        &env,
        &b,
        6,
        &still(1, 600, 400),
        Some("The file is gone"),
        false,
    );
    let broken = image(
        &env,
        &b,
        7,
        &still(2, 600, 400),
        Some("The file is not a picture"),
        false,
    );
    let got = env.ok("moodboard.get", json!({ "id": b }));
    let path_of = |id: &str| -> String {
        got["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["id"] == id)
            .and_then(|i| i["asset"]["path"].as_str())
            .expect("asset path")
            .to_string()
    };
    std::fs::remove_file(path_of(&missing)).unwrap();
    std::fs::write(path_of(&broken), b"this is not an image").unwrap();
    exec(
        &env,
        &format!(
            "INSERT INTO moodboard_item(id,moodboard_id,kind,x,y,w,h,z,caption,created_at,updated_at)
             VALUES ('noasset','{b}','image',20,{},200,150,99,'A tile with no picture record',1,1);",
            24 + 80 * 8
        ),
    );
    image(
        &env,
        &b,
        9,
        &still(3, 600, 400),
        Some(&words(
            300,
            "A very long caption that keeps going and going.",
        )),
        false,
    );
    image(&env, &b, 10, &still(4, 600, 400), None, false);
    image(
        &env,
        &b,
        11,
        &still(5, 600, 400),
        Some("Last picture"),
        false,
    );
    env.ok(
        "moodboard.add_note",
        json!({ "moodboardId": b, "text": "A note with a very long first line that has to wrap inside its card and never leave it.", "isPrivate": false }),
    );
    env.ok(
        "moodboard.add_link",
        json!({ "moodboardId": b, "url": "www.example.com/a/very/long/address/that/has/no/spaces/at/all/and/keeps/going/for/a/while/longer", "title": "A link with a long address" }),
    );
    env.ok(
        "moodboard.update_notes",
        json!({ "id": b, "notes": "Internal board notes." }),
    );
    (env, b)
}

// ------------------------------------------------------------------ samples

#[test]
#[ignore = "writes the sample PDFs to release-out/pdf-samples"]
fn write_moodboard_samples() {
    let (env, b) = low_tide();
    let (path, pdf) = export(&env, &b, json!({}), "m.pdf");
    keep(&path, "10-moodboard.pdf", pdf.pages);
    let (path, pdf) = export(&env, &b, json!({ "includeInternalNotes": true }), "mi.pdf");
    keep(&path, "10-moodboard-internal.pdf", pdf.pages);
    let (env, b) = stress();
    for (name, args) in [
        ("stress-all", json!({ "includeInternalNotes": true })),
        ("stress-images-off", json!({ "includeImages": false })),
        ("stress-title-off", json!({ "includeTitle": false })),
        ("stress-captions-off", json!({ "includeCaptions": false })),
    ] {
        let (path, pdf) = export(&env, &b, args, "s.pdf");
        keep(&path, &format!("10-moodboard-{name}.pdf"), pdf.pages);
    }
    // One picture, no pictures.
    let one = board(&env, "Stress: one picture");
    image(
        &env,
        &one,
        0,
        &still(0, 600, 400),
        Some("The only picture"),
        false,
    );
    let (path, pdf) = export(&env, &one, json!({}), "one.pdf");
    keep(&path, "10-moodboard-stress-one.pdf", pdf.pages);
    let none = board(&env, "Stress: no pictures");
    note(&env, &none, "Only a note on this board.", false);
    let (path, pdf) = export(&env, &none, json!({}), "none.pdf");
    keep(&path, "10-moodboard-stress-none.pdf", pdf.pages);
}

// ------------------------------------------------------------------ rules (spec J)

const HEADER_Y: f32 = PAGE_H - 40.0;
const FOOTER_Y: f32 = 66.0;

/// Frame width of a three column card: (700 - 2 x 10) / 3 - 2 x 8.
const FRAME_W3: f32 = 210.666_67;
const FRAME_H3: f32 = FRAME_W3 * 0.72;

fn check_furniture(pdf: &Pdf, header: &str) {
    for page in 1..=pdf.pages {
        let top: Vec<&str> = pdf
            .runs
            .iter()
            .filter(|r| r.page == page && r.y >= HEADER_Y)
            .map(|r| r.text.as_str())
            .collect();
        assert_eq!(top, vec![header], "running header on page {page}");
        let foot: Vec<&str> = pdf
            .runs
            .iter()
            .filter(|r| r.page == page && r.y < 40.0)
            .map(|r| r.text.as_str())
            .collect();
        assert!(foot.contains(&"Created using OpenFrame"), "{foot:?}");
        assert!(
            foot.contains(&format!("Page {page} of {}", pdf.pages).as_str()),
            "{foot:?}"
        );
        assert_eq!(foot.len(), 2, "nothing else in the footer: {foot:?}");
    }
    for r in &pdf.runs {
        assert!(r.size >= 6.4 - 0.01, "{} is {} pt", r.text, r.size);
        assert!(
            r.x >= 45.0 && r.x < 746.0,
            "{} starts at {} (outside the margins)",
            r.text,
            r.x
        );
        assert!(
            r.y >= FOOTER_Y || r.y >= HEADER_Y || r.y < 40.0,
            "{} sits in the footer zone at {}",
            r.text,
            r.y
        );
    }
}

#[test]
fn the_moodboard_is_landscape_with_the_running_header_and_the_footer() {
    let (env, b) = low_tide();
    let (_, pdf) = export(&env, &b, json!({}), "m.pdf");
    assert_eq!(pdf.width, 792.0, "landscape Letter");
    assert_eq!(pdf.pages, 2, "four pictures (3 and 1), then the notes");
    check_furniture(&pdf, "LOW TIDE RADIO \u{b7} MOODBOARD");
    // The board name is in the masthead and never in the header.
    assert!(pdf.has("Look of the film: monsoon night and the 1999 grade"));
    assert!(pdf.has("VISUAL DEVELOPMENT \u{b7} MOODBOARD"));
    assert!(pdf.has("4 images"));
    // Sentence case: the title is printed as typed.
    assert!(!pdf.has("LOOK OF THE FILM: MONSOON NIGHT AND THE 1999 GRADE"));
    for t in ["Link", "Note"] {
        assert!(pdf.runs.iter().any(|r| r.text.eq_ignore_ascii_case(t)));
    }
    // The masthead title is the compact 20 pt one.
    let title = pdf
        .runs
        .iter()
        .find(|r| r.text.starts_with("Look of the film"))
        .unwrap();
    assert!((title.size - 20.0).abs() < 0.01, "{}", title.size);
}

#[test]
fn private_items_and_board_notes_are_printed_only_with_internal_notes() {
    let (env, b) = low_tide();
    let (_, pdf) = export(&env, &b, json!({}), "m.pdf");
    let text = pdf.text();
    for secret in [
        "Budget note for Vivek",
        "balloon lights",
        "Grade reference for the DI",
        "Shared with the DOP",
        "INTERNAL",
        "Not for distribution",
    ] {
        assert!(!text.contains(secret), "{secret} leaked");
    }
    let (_, pdf) = export(&env, &b, json!({ "includeInternalNotes": true }), "mi.pdf");
    let text = pdf.text();
    assert!(text.contains("Budget note for Vivek: balloon lights only on the beach night."));
    assert!(text.contains("Grade reference for the DI."));
    assert!(pdf.has("Not for distribution"));
    assert!(pdf.has("INTERNAL"), "the chip on the private note");
    assert!(pdf.runs.iter().any(|r| r.text == "INTERNAL NOTES"));
    check_furniture(&pdf, "LOW TIDE RADIO \u{b7} MOODBOARD");
}

#[test]
fn no_picture_is_cropped_and_every_frame_has_the_same_shape() {
    let (env, b) = stress();
    let (_, pdf) = export(&env, &b, json!({ "includeInternalNotes": true }), "s.pdf");
    // Reading order: six shapes, then the long caption, the last two (all 600 by 400).
    let aspects = [
        640.0 / 360.0,
        400.0 / 600.0,
        1.0,
        4.0,
        1.5,
        360.0 / 640.0,
        1.5,
        1.5,
        1.5,
    ];
    assert_eq!(pdf.pics.len(), aspects.len(), "{:?}", pdf.pics);
    for (p, a) in pdf.pics.iter().zip(aspects) {
        assert!((p.w / p.h / a - 1.0).abs() < 0.01, "{p:?} against {a}");
        // Inside the 0.72 frame and touching it on the limiting side.
        assert!(p.w <= FRAME_W3 + 0.01 && p.h <= FRAME_H3 + 0.01, "{p:?}");
        let fit = (p.w / FRAME_W3).max(p.h / FRAME_H3);
        assert!(
            (fit - 1.0).abs() < 0.005,
            "{p:?} does not fill its frame ({fit})"
        );
    }
    // Three columns.
    let mut cols: Vec<i32> = pdf
        .pics
        .iter()
        .filter(|p| p.page == 1)
        .map(|p| ((p.x + p.w / 2.0) * 10.0).round() as i32)
        .collect();
    cols.sort();
    cols.dedup();
    assert_eq!(cols.len(), 3, "{cols:?}");
}

#[test]
fn missing_unreadable_and_unlinked_pictures_keep_their_frame_with_the_exact_wording() {
    let (env, b) = stress();
    let (_, pdf) = export(&env, &b, json!({}), "s.pdf");
    for t in [
        "Image unavailable: img6.png",
        "Image can't be printed",
        "Image unavailable",
        "The file is gone",
        "The file is not a picture",
        "A tile with no picture record",
    ] {
        assert!(pdf.has(t), "{t}");
    }
    // The placeholder text is the muted colour of the design, not the faint one: it is centred.
    let missing = pdf
        .runs
        .iter()
        .find(|r| r.text == "Image unavailable: img6.png")
        .unwrap();
    assert!(missing.size >= 8.4);
}

#[test]
fn one_or_two_pictures_use_two_columns_three_or_more_use_three_and_none_leave_no_gap() {
    let env = TestEnv::with_project("Low Tide Radio", "Feature Film");
    let one = board(&env, "One");
    image(
        &env,
        &one,
        0,
        &still(0, 600, 400),
        Some("The only one"),
        false,
    );
    let (_, pdf) = export(&env, &one, json!({}), "one.pdf");
    assert_eq!(pdf.pics.len(), 1);
    let fw2 = (700.0 - 10.0) / 2.0 - 16.0;
    assert!((pdf.pics[0].w - fw2).abs() < 0.05, "{:?}", pdf.pics[0]);
    assert!(pdf.has("1 image"));
    let two = board(&env, "Two");
    image(&env, &two, 0, &still(0, 600, 400), Some("A"), false);
    image(&env, &two, 1, &still(1, 600, 400), Some("B"), false);
    let (_, pdf) = export(&env, &two, json!({}), "two.pdf");
    assert_eq!(pdf.pics.len(), 2);
    assert!(pdf.pics.iter().all(|p| (p.w - fw2).abs() < 0.05));
    // No pictures: no frame, no gap, "0 images" and the notes right under the masthead rule.
    let none = board(&env, "None");
    note(&env, &none, "Only a note on this board.", false);
    let (_, pdf) = export(&env, &none, json!({}), "none.pdf");
    assert!(pdf.pics.is_empty());
    assert!(pdf.has("0 images"));
    let rule_y = pdf
        .runs
        .iter()
        .find(|r| r.text == "Low Tide Radio")
        .unwrap()
        .y;
    let heading = pdf
        .runs
        .iter()
        .find(|r| r.text == "NOTES AND LINKS")
        .unwrap();
    assert!(rule_y - heading.y < 70.0, "{} to {}", rule_y, heading.y);
}

#[test]
fn a_long_caption_wraps_inside_its_card_and_the_row_grows_without_touching_the_footer() {
    let (env, b) = stress();
    let (_, pdf) = export(&env, &b, json!({}), "s.pdf");
    let caption: Vec<&Run> = pdf
        .runs
        .iter()
        .filter(|r| r.text.contains("very long caption") || r.text == "very long")
        .collect();
    assert!(caption.len() >= 6, "{} lines", caption.len());
    let cw = (700.0 - 20.0) / 3.0;
    for r in &caption {
        let w = openframe_import_export::report::text_width(&r.text, r.size, false);
        let left = 46.0 + ((r.x - 46.0) / (cw + 10.0)).floor() * (cw + 10.0);
        assert!(
            r.x >= left && r.x + w <= left + cw + 0.5,
            "{} leaves its card ({} to {})",
            r.text,
            r.x,
            r.x + w
        );
        assert!(r.y > FOOTER_Y + 4.0, "{} meets the footer", r.text);
    }
    // The whole caption is printed.
    let flat: String = caption
        .iter()
        .map(|r| r.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(
        flat,
        words(300, "A very long caption that keeps going and going.")
    );
    assert!(
        pdf.runs
            .iter()
            .all(|r| r.y > FOOTER_Y || r.y < 40.0 || r.y >= HEADER_Y)
    );
}

#[test]
fn a_section_heading_never_ends_a_page() {
    let (env, b) = stress();
    for args in [
        json!({ "includeInternalNotes": true }),
        json!({ "includeImages": false }),
    ] {
        let (_, pdf) = export(&env, &b, args, "s.pdf");
        for h in ["NOTES AND LINKS", "INTERNAL NOTES"] {
            for r in pdf.runs.iter().filter(|r| r.text == h) {
                assert!(
                    pdf.runs
                        .iter()
                        .any(|x| x.page == r.page && x.y < r.y - 6.0 && x.y > FOOTER_Y),
                    "{h} ends page {}",
                    r.page
                );
            }
        }
    }
}

#[test]
fn the_title_option_off_removes_the_project_and_the_board_name_everywhere() {
    let (env, b) = stress();
    let (path, pdf) = export(&env, &b, json!({ "includeTitle": false }), "t.pdf");
    check_furniture(&pdf, "MOODBOARD");
    let text = pdf.text();
    for gone in [
        "Harbour Lights",
        "HARBOUR LIGHTS",
        "Stress: twelve pictures",
        "VISUAL DEVELOPMENT",
        "Exported",
        "images",
    ] {
        assert!(!text.contains(gone), "{gone}");
    }
    assert!(pdf.has("Moodboard"));
    let doc = lopdf::Document::load(&path).unwrap();
    let info = doc.trailer.get(b"Info").unwrap().as_reference().unwrap();
    let title = match doc.get_dictionary(info).unwrap().get(b"Title").unwrap() {
        lopdf::Object::String(b, _) => String::from_utf8_lossy(b).into_owned(),
        _ => panic!(),
    };
    assert_eq!(title, "Moodboard");
}

#[test]
fn the_images_option_off_keeps_captions_and_internal_labels_and_draws_no_frame() {
    let (env, b) = stress();
    let (_, pdf) = export(&env, &b, json!({ "includeImages": false }), "i.pdf");
    assert!(pdf.pics.is_empty());
    assert!(!pdf.text().contains("Image unavailable"));
    assert!(pdf.has("Picture 1 (640 by 360)") && pdf.has("Last picture"));
    // The caption-less picture is dropped (eleven captions: ten of them are public here).
    let captions = pdf.runs.iter().filter(|r| r.size == 9.0).count();
    assert!(captions >= 10, "{captions}");
    // The masthead counts what is printed.
    assert!(pdf.has("0 images"));
}

#[test]
fn the_captions_option_off_leaves_no_caption_and_titles_the_notes_note_and_link() {
    let (env, b) = stress();
    let (_, pdf) = export(&env, &b, json!({ "includeCaptions": false }), "c.pdf");
    assert!(!pdf.text().contains("Picture 1 (640 by 360)"));
    assert!(!pdf.has("Last picture"));
    assert_eq!(
        pdf.pics.len(),
        8,
        "pictures stay (the internal one is left out)"
    );
    for t in ["Note", "Link"] {
        assert!(pdf.has(t), "{t}");
    }
}

#[test]
fn a_board_with_nothing_to_print_says_so_and_the_export_is_deterministic() {
    let env = TestEnv::with_project("Low Tide Radio", "Feature Film");
    let b = board(&env, "Bare");
    image(&env, &b, 0, &still(0, 100, 100), None, false);
    let path = env.dir.path().join("x.pdf").to_string_lossy().into_owned();
    let err = env
        .call(
            "moodboard.export_pdf",
            json!({ "moodboardId": b, "format": "pdf", "path": path, "includeImages": false }),
        )
        .unwrap_err();
    assert!(
        format!("{err:?}")
            .contains("This moodboard has nothing to export with the selected options."),
        "{err:?}"
    );
    let (env, b) = low_tide();
    let (a, _) = export(&env, &b, json!({ "includeInternalNotes": true }), "a.pdf");
    let (c, _) = export(&env, &b, json!({ "includeInternalNotes": true }), "b.pdf");
    assert_eq!(std::fs::read(a).unwrap(), std::fs::read(c).unwrap());
}
