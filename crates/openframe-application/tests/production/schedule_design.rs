//! Layout rules of the Shooting Schedule PDF (PDF design package v2, Shooting_Schedule spec
//! sections H and J), read from the text runs of the real output. `write_schedule_design_samples`
//! writes the stress samples to `release-out/pdf-samples` for the eye:
//!
//! ```text
//! node scripts/cargo-queued.mjs test -p openframe-application --test production \
//!     schedule_design::write -- --ignored --nocapture
//! ```

use serde_json::{Value, json};

use super::call_sheet_design::{
    FOOTER_Y, HEADER_Y, Run, check_furniture, export, keep_sample, read,
};
use super::pdf_samples::{Film, build_film, strip_id};

const RUNNING: &str = "MONSOON LETTERS \u{b7} SHOOTING SCHEDULE \u{b7} REVISED ";

/// The page size of the first page.
fn page_size(path: &str) -> (f32, f32) {
    let doc = lopdf::Document::load(path).expect("PDF parses");
    let page = *doc.get_pages().values().next().expect("a page");
    let mut node = doc.get_object(page).unwrap().as_dict().unwrap().clone();
    loop {
        if let Ok(mb) = node.get(b"MediaBox") {
            let a = mb.as_array().unwrap();
            return (a[2].as_float().unwrap(), a[3].as_float().unwrap());
        }
        let parent = node.get(b"Parent").unwrap().as_reference().unwrap();
        node = doc.get_object(parent).unwrap().as_dict().unwrap().clone();
    }
}

fn schedule(film: &Film, args: Value, file: &str) -> (Vec<Run>, usize, String) {
    let path = export(film, "schedule.export_schedule", args, file);
    let (runs, pages) = read(&path);
    (runs, pages, path)
}

/// The running header the export printed, which must be the same on every page.
fn running_header(runs: &[Run]) -> String {
    runs.iter()
        .find(|r| r.y >= HEADER_Y)
        .expect("a running header")
        .text
        .clone()
}

fn body(runs: &[Run]) -> impl Iterator<Item = &Run> {
    runs.iter().filter(|r| r.y > FOOTER_Y && r.y < HEADER_Y)
}

fn page_of(runs: &[Run], text: &str) -> usize {
    runs.iter()
        .find(|r| r.text == text)
        .unwrap_or_else(|| panic!("{text} is not printed"))
        .page
}

/// Headings of the scenes on a day, in the order of the stored strips.
fn headings(film: &Film, day: usize) -> Vec<String> {
    let v = film.env.ok("schedule.get", json!({}));
    v["days"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == film.days[day])
        .unwrap()["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["kind"] == "strip")
        .map(|i| i["strip"]["heading"].as_str().unwrap().to_uppercase())
        .collect()
}

#[test]
fn the_schedule_is_portrait_with_the_running_header_and_the_shared_footer() {
    let film = build_film();
    let (runs, pages, path) = schedule(&film, json!({}), "s.pdf");
    assert_eq!(page_size(&path), (612.0, 792.0), "US Letter portrait");
    let header = running_header(&runs);
    assert!(header.starts_with(RUNNING), "{header}");
    check_furniture(&runs, pages, &header, &["UNSCHEDULED \u{b7} 0"]);
}

#[test]
fn every_day_is_kept_whole_with_its_band_facts_and_strips_on_one_page() {
    let film = build_film();
    let (runs, pages, _) = schedule(&film, json!({}), "s.pdf");
    assert!(pages >= 2);
    assert!(
        !runs.iter().any(|r| r.text == " \u{b7} continued"),
        "no day of the sample splits"
    );
    for (n, day) in [(1, 0), (2, 1), (3, 3), (4, 4)] {
        let title = format!("Shoot Day {n}");
        let on = runs.iter().filter(|r| r.text == title).count();
        assert_eq!(on, 1, "{title} once");
        let page = page_of(&runs, &title);
        for h in headings(&film, day) {
            let start: String = h.chars().take(8).collect();
            assert!(
                runs.iter()
                    .any(|r| r.page == page && r.text.starts_with(&start)),
                "{h} is on the page of {title}"
            );
        }
    }
    // The key comes before the first strip, with the text codes.
    let p1: Vec<&Run> = runs.iter().filter(|r| r.page == 1).collect();
    let key = p1.iter().find(|r| r.text == "STRIP LEGEND").unwrap();
    assert!(p1.iter().any(|r| r.text.contains("EXT \u{b7} DAY")));
    assert!(p1.iter().any(|r| r.text.contains("counts as exterior")));
    assert!(
        p1.iter()
            .filter(|r| r.text.starts_with("INT.") || r.text.starts_with("EXT."))
            .all(|r| r.y < key.y)
    );
}

#[test]
fn the_seven_columns_have_the_portrait_widths_in_order() {
    let film = build_film();
    let (runs, _, _) = schedule(&film, json!({}), "s.pdf");
    let head = |t: &str| {
        runs.iter()
            .find(|r| r.text == t && r.size < 7.0)
            .unwrap_or_else(|| panic!("column {t}"))
            .x
    };
    let scene = head("SCENE");
    assert!((head("TYPE") - scene - 38.0).abs() < 0.1);
    assert!((head("START") - head("TYPE") - 48.0).abs() < 0.1);
    assert!((head("HEADING & SYNOPSIS") - head("START") - 38.0).abs() < 0.1);
    // 520 - 38 - 48 - 38 - 124 - 36 - 40 = 196 for the heading and synopsis.
    assert!((head("CAST") - head("HEADING & SYNOPSIS") - 196.0).abs() < 0.1);
    let order = [
        "SCENE",
        "TYPE",
        "START",
        "HEADING & SYNOPSIS",
        "CAST",
        "PAGES",
        "EST.",
    ];
    let xs: Vec<f32> = order.iter().map(|t| head(t)).collect();
    assert!(xs.windows(2).all(|w| w[0] < w[1]), "{xs:?}");
}

#[test]
fn facts_breaks_and_locations_use_the_words_of_the_source() {
    let film = build_film();
    let (runs, _, _) = schedule(&film, json!({}), "s.pdf");
    let has = |t: &str| runs.iter().any(|r| r.text == t);
    // Day 1: four scenes and the meal and the company move.
    assert!(has("4 scenes, 2 other items"));
    assert!(has(" \u{b7} arranged in production order"));
    assert!(runs.iter().any(|r| r.text.contains("06:00 crew call")));
    assert!(runs.iter().any(|r| r.text.contains("(target 18:00)")));
    assert!(runs.iter().any(|r| r.text.contains("pages total")));
    assert!(has("LOCATIONS"));
    // The type chip of a break is one run on one line; the label and its note follow.
    assert!(has("COMPANY MOVE"));
    assert!(has("MEAL"));
    assert!(runs.iter().any(|r| r.text.starts_with("MOVE FROM THE")));
    // The strips carry both text chips.
    assert!(has("EXT") && has("DAY"));
}

#[test]
fn nothing_is_smaller_than_the_floor_and_cell_text_is_at_least_eight_points() {
    let film = build_film();
    let (runs, _, _) = schedule(&film, json!({}), "s.pdf");
    for r in body(&runs) {
        assert!(r.size >= 6.4 - 0.01, "{} is {} pt", r.text, r.size);
        if r.size < 8.0 {
            let label = r.text.chars().all(|c| !c.is_lowercase() || c == '\u{b7}')
                || r.text == "(+1)"
                || r.text.starts_with("Exterior")
                || r.text.starts_with("Interior")
                || r.text.contains("counts as exterior")
                || r.text.starts_with("Scenes not on")
                || r.text.starts_with("over the")
                || r.text.starts_with("past the");
            assert!(label, "{} ({} pt) is not a label or a chip", r.text, r.size);
        }
    }
}

#[test]
fn an_empty_pool_sentence_stays_on_the_page_of_the_last_day() {
    let film = build_film();
    let (runs, pages, _) = schedule(&film, json!({}), "s.pdf");
    let sentence = page_of(&runs, "All source scenes are scheduled.");
    assert_eq!(sentence, page_of(&runs, "Shoot Day 4"));
    assert_eq!(sentence, pages, "the last day's page is the last page");
}

#[test]
fn unscheduled_scenes_print_in_a_titled_table_and_vanish_when_not_asked_for() {
    let film = build_film();
    for n in [13, 14] {
        let s = strip_id(&film.env, n);
        film.env.ok(
            "schedule.move_strip",
            json!({ "stripId": s, "dayId": Value::Null }),
        );
    }
    let (runs, _, _) = schedule(&film, json!({}), "u.pdf");
    let has = |t: &str| runs.iter().any(|r| r.text == t);
    assert!(has("UNSCHEDULED \u{b7} 2"));
    assert!(has("Scenes not on a shooting day"));
    assert!(has("UNSCHEDULED SCENES"), "the metric");
    assert!(!has("All source scenes are scheduled."));
    // Without the pool neither the metric nor the section is printed.
    let (runs, _, _) = schedule(&film, json!({ "includeUnscheduled": false }), "n.pdf");
    assert!(!runs.iter().any(|r| r.text.starts_with("UNSCHEDULED")));
}

#[test]
fn an_off_day_and_an_empty_day_print_their_own_sentences() {
    let film = build_film();
    film.env.ok(
        "schedule.set_day_notes",
        json!({ "dayId": film.days[2], "notes": "Travel to the coast." }),
    );
    film.env.ok(
        "schedule.create_day",
        json!({ "scheduleId": film.schedule, "date": "2027-06-19" }),
    );
    let (runs, _, _) = schedule(&film, json!({}), "o.pdf");
    let has = |t: &str| runs.iter().any(|r| r.text == t);
    assert!(has("Off day"));
    assert!(has("No shooting"));
    assert!(has("Travel to the coast."));
    assert!(has("DAY NOTES"));
    assert!(has("No scenes scheduled."));
}

/// All the film's scenes on the first day and a long note: the day is taller than a page.
fn heavy_day(film: &Film) {
    for n in 5..=14 {
        let s = strip_id(&film.env, n);
        film.env.ok(
            "schedule.move_strip",
            json!({ "stripId": s, "dayId": film.days[0] }),
        );
    }
    film.env.ok(
        "schedule.set_day_notes",
        json!({ "dayId": film.days[0], "notes": "Basement water pumped from 05:00." }),
    );
}

#[test]
fn a_day_taller_than_a_page_repeats_band_facts_and_header_and_prints_its_notes_once() {
    let film = build_film();
    heavy_day(&film);
    let (runs, pages, _) = schedule(&film, json!({ "dayIds": [film.days[0]] }), "h.pdf");
    assert!(pages >= 2, "{pages} pages");
    for p in 1..=pages {
        let on = |t: &str| runs.iter().any(|r| r.page == p && r.text == t);
        assert!(on("Shoot Day 1"), "band on page {p}");
        assert!(on("SCENE"), "column header on page {p}");
        assert!(on("LOCATIONS"), "facts on page {p}");
        assert_eq!(on(" \u{b7} continued"), p > 1, "continued mark on page {p}");
    }
    assert_eq!(
        runs.iter()
            .filter(|r| r.text == "Basement water pumped from 05:00.")
            .count(),
        1,
        "the notes row prints once"
    );
    // No page of the day holds a lone row: every page has at least two scene tiles.
    for p in 1..=pages {
        let tiles = runs
            .iter()
            .filter(|r| r.page == p && r.size == 9.0 && r.x < 100.0 && r.y > FOOTER_Y)
            .count();
        assert!(tiles >= 2, "page {p} holds {tiles} strips");
    }
}

#[test]
fn the_same_schedule_exports_to_identical_bytes() {
    let film = build_film();
    let (_, _, a) = schedule(&film, json!({}), "a.pdf");
    let (_, _, b) = schedule(&film, json!({}), "b.pdf");
    assert_eq!(std::fs::read(a).unwrap(), std::fs::read(b).unwrap());
}

#[test]
#[ignore = "writes the sample PDFs to release-out/pdf-samples"]
fn write_schedule_design_samples() {
    let film = build_film();
    for n in [13, 14] {
        let s = strip_id(&film.env, n);
        film.env.ok(
            "schedule.move_strip",
            json!({ "stripId": s, "dayId": Value::Null }),
        );
    }
    let (_, _, p) = schedule(&film, json!({}), "a.pdf");
    keep_sample(&p, "c4-schedule-pool.pdf");
    let film = build_film();
    heavy_day(&film);
    film.env.ok(
        "schedule.create_day",
        json!({ "scheduleId": film.schedule, "date": "2027-06-19" }),
    );
    film.env.ok(
        "schedule.set_day_notes",
        json!({ "dayId": film.days[2], "notes": "Travel to the coast. Unit hotel booked, call 06:30 at the lobby, buses leave 07:00." }),
    );
    let (_, _, p) = schedule(&film, json!({ "includeNotes": true }), "b.pdf");
    keep_sample(&p, "c4-schedule-stress.pdf");
    let (_, _, p) = schedule(&film, json!({ "dayIds": [film.days[0]] }), "c.pdf");
    keep_sample(&p, "c4-schedule-heavy-day.pdf");
}
