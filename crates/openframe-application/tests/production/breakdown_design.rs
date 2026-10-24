//! Layout rules of the Breakdown, Breakdown and Catalog, and Catalog PDFs (PDF design package v2,
//! the breakdown specs, sections H and J), read from the text runs of the real output.
//! `write_breakdown_design_samples` writes the stress samples to `release-out/pdf-samples` for the
//! eye:
//!
//! ```text
//! node scripts/cargo-queued.mjs test -p openframe-application --test production \
//!     breakdown_design::write -- --ignored --nocapture
//! ```

use openframe_domain::enums::BreakdownCategory;
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

use super::call_sheet_design::{
    FOOTER_Y, HEADER_Y, Run, check_furniture, export, keep_sample, read,
};
use super::pdf_samples::{Film, build_film};

const RUNNING: &str = "MONSOON LETTERS \u{b7} BREAKDOWN REPORT";
const SENTENCE: &str = "Suggested elements are not confirmed yet and carry a SUGGESTED label. \
                        Elements added by hand carry a MANUAL label.";

fn exec(env: &TestEnv, sql: &str) {
    env.core
        .project()
        .unwrap()
        .store
        .with_writer(|c| {
            c.execute_batch(sql).expect("fixture sql");
            Ok(())
        })
        .unwrap();
}

fn breakdown(film: &Film, args: Value, file: &str) -> (Vec<Run>, usize, String) {
    let path = export(film, "breakdown.export_report", args, file);
    let (runs, pages) = read(&path);
    (runs, pages, path)
}

fn has(runs: &[Run], text: &str) -> bool {
    runs.iter().any(|r| r.text == text)
}

fn at<'a>(runs: &'a [Run], text: &str) -> &'a Run {
    runs.iter()
        .find(|r| r.text == text)
        .unwrap_or_else(|| panic!("{text} is not printed"))
}

/// Drawing order index of the first run with this text.
fn pos(runs: &[Run], text: &str) -> usize {
    runs.iter()
        .position(|r| r.text == text)
        .unwrap_or_else(|| panic!("{text} is not printed"))
}

/// A scene with 30 props, a blank name, an unbroken 92 character token and a long note; an
/// omitted scene, a scene with no element and a long heading; 70 more props in the catalog.
fn stress(film: &Film) {
    let mut sql = String::new();
    sql.push_str(
        "INSERT INTO screenplay_scene(id,draft_id,lineage_id,position,heading,synopsis,omitted,created_at,updated_at) VALUES \
         ('s15','d1','L15',15,'INT. CUT SCENE - DAY','x',1,1,1),\
         ('s16','d1','L16',16,'EXT. A HEADING THAT RUNS ON FOR A VERY LONG TIME BECAUSE THE WRITER WANTED THE WHOLE STREET, THE RIVER, THE BRIDGE AND THE OLD MILL IN ONE LINE - DUSK',\
          'x',0,1,1);",
    );
    let note = "The note keeps going so that it wraps over several lines. ".repeat(5);
    for i in 0..30 {
        sql.push_str(&format!(
            "INSERT INTO breakdown_element(id,source_id,scene_id,scene_lineage_id,category,catalog_item_id,display_name,notes,confirmation_state,created_at,updated_at) \
             VALUES ('xp{i}','ps1','s1','L1','Props',NULL,'Stress prop {i:02} with a longer descriptive name',{},'{}',{},{});",
            if i % 3 == 0 { format!("'{note}'") } else { "NULL".into() },
            ["Confirmed", "Manual", "Suggested"][i % 3],
            2000 + i,
            2000 + i
        ));
    }
    sql.push_str(&format!(
        "INSERT INTO breakdown_element(id,source_id,scene_id,scene_lineage_id,category,catalog_item_id,display_name,confirmation_state,created_at,updated_at) \
         VALUES ('xb','ps1','s2','L2','Sound',NULL,' ','Confirmed',3000,3000),\
                ('xt','ps1','s2','L2','Sound',NULL,'{}','Confirmed',3001,3001);",
        "W".repeat(92)
    ));
    for i in 0..70 {
        let scene = 2 + i % 5;
        sql.push_str(&format!(
            "INSERT INTO catalog_item(id,category,name,description,status,created_at,updated_at) VALUES ('cx{i}','Props','Stress item {i:02}','A description for item {i}',{},1,1);\
             INSERT INTO breakdown_element(id,source_id,scene_id,scene_lineage_id,category,catalog_item_id,display_name,confirmation_state,created_at,updated_at) \
             VALUES ('xc{i}','ps1','s{scene}','L{scene}','Props','cx{i}','Stress item {i:02}','Confirmed',{},{});",
            ["'Required'", "'Searching'", "'Shortlisted'", "'Not Required'"][i % 4],
            4000 + i,
            4000 + i
        ));
    }
    // A very long name, a long description and an unbroken 92 character name in the catalog.
    sql.push_str(&format!(
        "INSERT INTO catalog_item(id,category,name,description,status,created_at,updated_at) VALUES 
         ('cl1','Props','{}','{}','Required',1,1),('cl2','Props','{}',NULL,'Searching',1,1);",
        "An item with a very long name ".repeat(5),
        "A long description that keeps going. ".repeat(8),
        "T".repeat(92)
    ));
    sql.push_str(
        "INSERT INTO catalog_item(id,category,name,description,status,created_at,updated_at) VALUES ('cu','Wardrobe','Unused coat',NULL,'Required',1,1);",
    );
    exec(&film.env, &sql);
}

fn first_scene_id(film: &Film) -> Value {
    // Scene ids of the fixture are s1, s2, ...
    let _ = film;
    json!("s1")
}

#[test]
fn the_combined_report_keeps_the_order_labels_furniture_and_part_labels() {
    let film = build_film();
    let (runs, pages, _) = breakdown(&film, json!({ "includeNotes": true }), "b.pdf");
    assert!(pages >= 2);
    check_furniture(&runs, pages, RUNNING, &["BREAKDOWN BY SCENE", "CATALOG"]);
    // One eyebrow and the title; the old second label is gone.
    assert!(has(&runs, "PRODUCTION / BREAKDOWN REPORT"));
    assert!(!has(&runs, "PRODUCTION BREAKDOWN"));
    assert!(has(&runs, "Breakdown Report"));
    // The four metrics in the stated order, then the two parts in order.
    let order = [
        "PRODUCTION / BREAKDOWN REPORT",
        "SCENES",
        "CONFIRMED ELEMENTS",
        "SUGGESTED",
        "CATALOG ITEMS",
        "PART 1",
        "BREAKDOWN BY SCENE",
        "PART 2",
        "CATALOG",
    ];
    let idx: Vec<usize> = order
        .iter()
        .map(|t| {
            runs.iter()
                .position(|r| r.text == *t && r.size >= 6.6)
                .unwrap_or_else(|| panic!("{t}"))
        })
        .collect();
    assert!(idx.windows(2).all(|w| w[0] < w[1]), "{idx:?}");
    // The Catalog items metric equals the rows of the catalog tables.
    // A head that repeats on a later page says " · continued" next to its title: count it once.
    let items: usize = runs
        .iter()
        .filter(|r| {
            !runs
                .iter()
                .any(|c| c.page == r.page && c.text == " · continued" && (c.y - r.y).abs() < 3.0)
        })
        .filter_map(|r| {
            r.text
                .strip_suffix(" items")
                .or(r.text.strip_suffix(" item"))
        })
        .filter_map(|n| n.parse::<usize>().ok())
        .sum();
    let metric = runs
        .iter()
        .filter(|r| r.size > 15.0 && r.text.chars().all(|c| c.is_ascii_digit()))
        .nth(3)
        .map(|r| r.text.parse::<usize>().unwrap())
        .expect("the fourth metric");
    assert_eq!(items, metric, "item counts of the category heads");
}

#[test]
fn the_part_labels_exist_only_in_the_combined_report() {
    let film = build_film();
    for (content, parts) in [("scenes", false), ("catalog", false), ("both", true)] {
        let (runs, pages, _) = breakdown(&film, json!({ "content": content }), "p.pdf");
        assert_eq!(has(&runs, "PART 1"), parts, "{content}");
        assert_eq!(has(&runs, "PART 2"), parts, "{content}");
        let header = if content == "catalog" {
            "MONSOON LETTERS \u{b7} CATALOG"
        } else {
            RUNNING
        };
        check_furniture(&runs, pages, header, &["BREAKDOWN BY SCENE", "CATALOG"]);
    }
    let (runs, _, _) = breakdown(&film, json!({ "content": "catalog" }), "c.pdf");
    assert!(has(&runs, "PRODUCTION / CATALOG"));
    assert!(has(&runs, "Catalog"));
    assert!(!has(&runs, "CONFIRMED ELEMENTS"), "no element metrics");
}

#[test]
fn categories_print_in_canonical_order_each_row_with_its_name() {
    let film = build_film();
    let (runs, _, _) = breakdown(
        &film,
        json!({ "content": "scenes", "sceneIds": [first_scene_id(&film)] }),
        "o.pdf",
    );
    let names: Vec<String> = BreakdownCategory::ALL
        .iter()
        .map(|c| c.as_str().to_uppercase())
        .collect();
    // Category labels are 6.9 pt bold caps in the label column.
    let labels: Vec<&Run> = runs
        .iter()
        .filter(|r| r.size == 6.9 && r.x < 160.0 && names.contains(&r.text))
        .collect();
    assert!(labels.len() >= 3, "{labels:?}");
    let canon: Vec<usize> = labels
        .iter()
        .map(|r| names.iter().position(|n| *n == r.text).unwrap())
        .collect();
    assert!(canon.windows(2).all(|w| w[0] < w[1]), "{canon:?}");
    // A category the scene does not use prints no row.
    assert!(!has(&runs, "ANIMALS"));
}

#[test]
fn states_are_text_tags_and_the_suggested_option_removes_the_suggested_rows() {
    let film = build_film();
    let (all, _, _) = breakdown(&film, json!({ "content": "scenes" }), "s.pdf");
    let tags = |runs: &[Run], t: &str| runs.iter().filter(|r| r.size == 6.6 && r.text == t).count();
    // Five suggested elements and one sample in the key; one manual element and one sample.
    assert_eq!(tags(&all, "SUGGESTED"), 5 + 1);
    assert_eq!(tags(&all, "MANUAL"), 1 + 1);
    assert!(has(&all, "SUGGESTED"), "the metric");
    let sentence: String = all
        .iter()
        .filter(|r| r.size == 8.4 && r.y > FOOTER_Y)
        .map(|r| r.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        sentence.contains("Suggested elements are not confirmed yet"),
        "{sentence}"
    );
    assert!(SENTENCE.starts_with("Suggested elements are not confirmed yet"));
    // The scene heads count the suggestions.
    assert!(all.iter().any(|r| r.text == "1 SUGGESTED"));

    let (off, _, _) = breakdown(
        &film,
        json!({ "content": "scenes", "includeSuggested": false }),
        "n.pdf",
    );
    assert_eq!(tags(&off, "SUGGESTED"), 0);
    assert!(!has(&off, "SUGGESTED"), "no metric");
    assert!(
        !off.iter()
            .any(|r| r.text.contains("Suggested elements are not"))
    );
    assert!(!off.iter().any(|r| r.text.ends_with(" SUGGESTED")));
    assert_eq!(tags(&off, "MANUAL"), 1, "no key sample without suggestions");
    // The three element rows of the key are gone as well as the five elements.
    let names = |runs: &[Run]| runs.iter().filter(|r| r.size == 8.2).count();
    assert!(
        names(&all) - names(&off) >= 5 + 3,
        "{} {}",
        names(&all),
        names(&off)
    );
}

#[test]
fn notes_print_only_with_the_notes_option_and_change_the_column_count() {
    let film = build_film();
    let note = "Packed with two rotis before the take";
    let (with, _, _) = breakdown(
        &film,
        json!({ "content": "scenes", "includeNotes": true }),
        "w.pdf",
    );
    let (without, _, _) = breakdown(&film, json!({ "content": "scenes" }), "x.pdf");
    assert!(
        with.iter().any(|r| r.text.contains("Packed with two")),
        "{note}"
    );
    assert!(!without.iter().any(|r| r.text.contains("Packed with two")));
    // Names sit in two columns with notes and in three without.
    let columns = |runs: &[Run]| {
        let mut xs: Vec<i32> = runs
            .iter()
            .filter(|r| {
                r.size == 8.2 && r.page == 1 && r.y > FOOTER_Y && r.x > 140.0 && r.text != "Name"
            })
            .map(|r| r.x as i32)
            .collect();
        xs.sort();
        xs.dedup();
        xs.len()
    };
    assert_eq!(columns(&with), 2, "two columns with notes");
    assert!(columns(&without) >= 3, "three columns without notes");
}

#[test]
fn a_scene_that_splits_repeats_its_head_and_a_long_category_says_cont() {
    let film = build_film();
    stress(&film);
    let (runs, pages, _) = breakdown(
        &film,
        json!({ "content": "scenes", "sceneIds": ["s1"], "includeNotes": true }),
        "l.pdf",
    );
    assert!(pages >= 2, "{pages}");
    for p in 1..=pages {
        let on = |t: &str| runs.iter().any(|r| r.page == p && r.text == t);
        assert!(on("BREAKDOWN SCENE"), "the head on page {p}");
        assert_eq!(on(" \u{b7} continued"), p > 1, "continued mark on page {p}");
    }
    assert!(runs.iter().filter(|r| r.text == "PROPS").count() >= 2);
    // 33 props make three rows of at most twelve; the later ones say "cont.".
    let conts = runs.iter().filter(|r| r.text == "cont.").count();
    assert_eq!(conts, 2);
    // No page holds only the head: every page has at least two element names.
    for p in 1..=pages {
        let names = runs
            .iter()
            .filter(|r| r.page == p && r.size == 8.2 && r.y > FOOTER_Y)
            .count();
        assert!(names >= 2, "page {p} holds {names} names");
    }
    // Every prop is printed once.
    for i in 0..30 {
        let n = format!("Stress prop {i:02}");
        let c = runs.iter().filter(|r| r.text.starts_with(&n)).count();
        assert_eq!(c, 1, "{n}");
    }
}

#[test]
fn an_omitted_scene_an_empty_scene_and_a_blank_name_print_the_source_words() {
    let film = build_film();
    stress(&film);
    let (runs, pages, _) = breakdown(&film, json!({ "content": "scenes" }), "e.pdf");
    assert!(has(&runs, "OMITTED"));
    assert!(has(&runs, "No breakdown elements yet."));
    assert!(has(&runs, "Unnamed element"));
    // The unbroken token wraps and nothing leaves the page.
    let w: Vec<&Run> = runs.iter().filter(|r| r.text.starts_with("WWWW")).collect();
    assert!(w.len() >= 2, "the token wraps over several lines: {w:?}");
    for r in &runs {
        assert!(r.x >= 45.0 && r.x < 568.0, "{} at {}", r.text, r.x);
    }
    // The long heading wraps in the head.
    let heading: Vec<&Run> = runs
        .iter()
        .filter(|r| r.size == 9.6 && r.text.contains("HEADING") || r.text.contains("OLD MILL"))
        .collect();
    assert!(!heading.is_empty());
    check_furniture(&runs, pages, RUNNING, &["BREAKDOWN BY SCENE"]);
}

#[test]
fn catalog_tables_repeat_head_and_labels_and_never_print_a_continued_bar() {
    let film = build_film();
    stress(&film);
    let (runs, pages, _) = breakdown(&film, json!({ "content": "catalog" }), "k.pdf");
    assert!(pages >= 3, "{pages}");
    // The Props table runs over several pages: its head and column labels come back each time.
    let props: Vec<usize> = runs
        .iter()
        .filter(|r| r.text == "PROPS" && r.size == 9.6)
        .map(|r| r.page)
        .collect();
    assert!(props.len() >= 2, "{props:?}");
    for p in &props {
        for label in ["ITEM", "STATUS", "USED IN SCENES", "USES"] {
            assert!(
                runs.iter().any(|r| r.page == *p && r.text == label),
                "{label} on page {p}"
            );
        }
    }
    // The continued mark follows the name on every page after the table's first, and only there.
    let first = props[0];
    for p in &props {
        let mark = runs
            .iter()
            .any(|r| r.page == *p && r.text == " \u{b7} continued");
        assert_eq!(mark, *p != first, "page {p}");
    }
    assert!(!runs.iter().any(|r| r.text.contains("(CONTINUED)")));
    // Columns in the stated order.
    let x = |t: &str| at(&runs, t).x;
    assert!(x("ITEM") < x("STATUS") && x("STATUS") < x("USED IN SCENES"));
    assert!(x("USED IN SCENES") < x("USES"));
    // Item names are in order, case-insensitively, inside the table.
    let names: Vec<&str> = runs
        .iter()
        .filter(|r| r.text.starts_with("Stress item "))
        .map(|r| r.text.as_str())
        .collect();
    assert_eq!(names.len(), 70);
    assert!(
        names
            .windows(2)
            .all(|w| w[0].to_lowercase() <= w[1].to_lowercase())
    );
    // A table starts a page with at least three rows after its head, except that its last page
    // may hold just the last two rows (they stay together).
    for p in &props {
        let rows = runs
            .iter()
            .filter(|r| r.page == *p && r.text.starts_with("Stress item "))
            .count();
        let floor = if p == props.last().unwrap() { 2 } else { 3 };
        assert!(rows >= floor, "page {p} holds {rows} rows");
    }
    check_furniture(&runs, pages, "MONSOON LETTERS \u{b7} CATALOG", &["CATALOG"]);
}

#[test]
fn catalog_rows_list_scenes_ascending_count_them_and_say_not_used_yet() {
    let film = build_film();
    stress(&film);
    let (runs, _, _) = breakdown(&film, json!({ "content": "catalog" }), "u.pdf");
    // The Stress item rows are used in one scene each; the unused coat in none.
    assert!(has(&runs, "Unused coat"));
    assert!(has(&runs, "Not used yet"));
    // Savitri is used in many scenes: the list is complete, ascending and counted by "Uses".
    let sav = at(&runs, "Savitri");
    let row_y = sav.y;
    let list: Vec<&Run> = runs
        .iter()
        .filter(|r| r.page == sav.page && (r.y - row_y).abs() < 6.0 && r.x > 280.0 && r.x < 480.0)
        .collect();
    let text: String = list
        .iter()
        .map(|r| r.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let nums: Vec<usize> = text
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|n| n.parse().ok())
        .collect();
    assert!(nums.len() >= 5, "{text}");
    assert!(nums.windows(2).all(|w| w[0] < w[1]), "{nums:?}");
    let uses = runs
        .iter()
        .find(|r| r.page == sav.page && (r.y - row_y).abs() < 16.0 && r.x >= 480.0)
        .expect("the count");
    assert_eq!(uses.text.parse::<usize>().unwrap(), nums.len());
    // Status chips carry their words.
    for s in ["REQUIRED", "SEARCHING", "SHORTLISTED", "NOT REQUIRED"] {
        assert!(has(&runs, s), "{s}");
    }
}

#[test]
fn a_scene_selection_keeps_only_items_used_there_and_an_empty_result_says_so() {
    let film = build_film();
    stress(&film);
    let (runs, _, _) = breakdown(
        &film,
        json!({ "content": "catalog", "sceneIds": ["s11"] }),
        "f.pdf",
    );
    // Scene 11 uses the postmaster and the letters; unused items are gone.
    assert!(has(&runs, "The Postmaster"));
    assert!(!has(&runs, "Unused coat"));
    assert!(!has(&runs, "Not used yet"));
    assert!(has(&runs, "1 selected scene"));
    // Scene 16 has no element: the catalog is empty for it.
    let (none, _, _) = breakdown(
        &film,
        json!({ "content": "catalog", "sceneIds": ["s16"] }),
        "g.pdf",
    );
    assert!(has(&none, "The catalog is empty."));
    assert!(has(&none, "CATALOG"), "the heading stays");
    assert!(!has(&none, "ITEMS PER CATEGORY"));
}

#[test]
fn the_category_strip_counts_the_items_of_each_table() {
    let film = build_film();
    let (runs, _, _) = breakdown(&film, json!({ "content": "catalog" }), "i.pdf");
    assert!(has(&runs, "ITEMS PER CATEGORY"));
    // For every category head "N items" there is a strip count with the same number.
    let strip_y = at(&runs, "ITEMS PER CATEGORY").y;
    let strip: Vec<&Run> = runs
        .iter()
        .filter(|r| r.page == 1 && r.y < strip_y && r.y > strip_y - 60.0 && r.size == 8.4)
        .collect();
    let total: usize = strip
        .iter()
        .filter_map(|r| r.text.parse::<usize>().ok())
        .sum();
    let heads: usize = runs
        .iter()
        .filter(|r| {
            !runs
                .iter()
                .any(|c| c.page == r.page && c.text == " · continued" && (c.y - r.y).abs() < 3.0)
        })
        .filter_map(|r| {
            r.text
                .strip_suffix(" items")
                .or(r.text.strip_suffix(" item"))
        })
        .filter_map(|n| n.parse::<usize>().ok())
        .sum();
    assert_eq!(total, heads);
}

#[test]
fn the_same_data_exports_to_identical_bytes() {
    let film = build_film();
    for content in ["both", "scenes", "catalog"] {
        let (_, _, a) = breakdown(
            &film,
            json!({ "content": content, "includeNotes": true }),
            "a.pdf",
        );
        let (_, _, b) = breakdown(
            &film,
            json!({ "content": content, "includeNotes": true }),
            "b.pdf",
        );
        assert_eq!(
            std::fs::read(a).unwrap(),
            std::fs::read(b).unwrap(),
            "{content}"
        );
    }
}

#[test]
fn nothing_sits_in_the_footer_band_and_text_is_at_least_the_floor() {
    let film = build_film();
    stress(&film);
    let (runs, pages, _) = breakdown(&film, json!({ "includeNotes": true }), "z.pdf");
    for r in &runs {
        assert!(r.size >= 6.4 - 0.01, "{} is {} pt", r.text, r.size);
        assert!(
            r.y >= FOOTER_Y || r.y >= HEADER_Y || r.y < 40.0,
            "{} in the footer band at {}",
            r.text,
            r.y
        );
    }
    assert!(pos(&runs, "PART 1") < pos(&runs, "PART 2"));
    check_furniture(&runs, pages, RUNNING, &["BREAKDOWN BY SCENE", "CATALOG"]);
}

#[test]
#[ignore = "writes the sample PDFs to release-out/pdf-samples"]
fn write_breakdown_design_samples() {
    let film = build_film();
    let (_, _, p) = breakdown(
        &film,
        json!({ "content": "scenes", "includeNotes": true }),
        "a.pdf",
    );
    keep_sample(&p, "c5-breakdown-scenes.pdf");
    let (_, _, p) = breakdown(&film, json!({ "content": "scenes" }), "b.pdf");
    keep_sample(&p, "c5-breakdown-scenes-no-notes.pdf");
    stress(&film);
    let (_, _, p) = breakdown(&film, json!({ "includeNotes": true }), "c.pdf");
    keep_sample(&p, "c5-breakdown-stress-both.pdf");
    let (_, _, p) = breakdown(&film, json!({ "content": "scenes" }), "d.pdf");
    keep_sample(&p, "c5-breakdown-stress-scenes.pdf");
    let (_, _, p) = breakdown(&film, json!({ "content": "catalog" }), "e.pdf");
    keep_sample(&p, "c5-catalog-stress.pdf");
    let (_, _, p) = breakdown(
        &film,
        json!({ "content": "catalog", "sceneIds": ["s16"] }),
        "f.pdf",
    );
    keep_sample(&p, "c5-catalog-empty.pdf");
    let (_, _, p) = breakdown(&film, json!({ "includeSuggested": false }), "g.pdf");
    keep_sample(&p, "c5-breakdown-no-suggested.pdf");
}
