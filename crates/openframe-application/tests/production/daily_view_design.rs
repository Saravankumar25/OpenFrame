//! Layout rules of the Daily View PDF (PDF design package v2, Daily_View spec sections H and J),
//! read from the text runs of the real output like the call sheet rules in `call_sheet_design.rs`.
//! `write_daily_view_design_samples` writes samples to `release-out/pdf-samples` for the eye.

use serde_json::json;

use super::call_sheet_design::{
    FOOTER_Y, HEADER_Y, Run, check_furniture, export, keep_sample, read, texts,
};
use super::pdf_samples::{Film, build_film, id_of};

const DAILY_HEADINGS: [&str; 7] = [
    "SCENES",
    "LOCATION",
    "CAST",
    "CALL SHEET",
    "IMPORTANT NOTES",
    "IMPORTANT BREAKDOWN ITEMS",
    "SCENE ORDER",
];

fn daily(film: &Film, day: usize, file: &str) -> (Vec<Run>, usize) {
    let path = export(
        film,
        "schedule.export_daily_view",
        json!({ "dayId": film.days[day] }),
        file,
    );
    read(&path)
}

fn daily_order(runs: &[Run]) -> Vec<&'static str> {
    let mut seen: Vec<(usize, &'static str)> = Vec::new();
    for h in DAILY_HEADINGS {
        if let Some(i) = runs
            .iter()
            .position(|r| r.text == h && r.y < HEADER_Y && r.y > FOOTER_Y)
        {
            seen.push((i, h));
        }
    }
    seen.sort();
    seen.into_iter().map(|(_, h)| h).collect()
}

/// A day with two scenes and one location, as most days are: it fits one page.
fn typical_day(film: &Film) -> (String, String) {
    let env = &film.env;
    let day = id_of(&env.ok(
        "schedule.create_day",
        json!({ "scheduleId": film.schedule, "date": "2027-07-01" }),
    ));
    for n in [2, 3] {
        let s = super::pdf_samples::strip_id(env, n);
        env.ok("schedule.move_strip", json!({ "stripId": s, "dayId": day }));
    }
    env.ok(
        "schedule.set_day_notes",
        json!({ "dayId": day, "notes": "Rain rig on standby." }),
    );
    let label = env.ok("schedule.daily", json!({ "dayId": day }))["day"]["label"]
        .as_str()
        .unwrap()
        .to_string();
    (day, label)
}

fn daily_of(film: &Film, day: &str, file: &str) -> (Vec<Run>, usize) {
    let path = export(
        film,
        "schedule.export_daily_view",
        json!({ "dayId": day }),
        file,
    );
    read(&path)
}

#[test]
fn a_typical_daily_view_is_one_page_without_a_running_header() {
    let film = build_film();
    let (day, label) = typical_day(&film);
    film.env.ok("callsheets.create", json!({ "dayId": day }));
    let (runs, pages) = daily_of(&film, &day, "daily.pdf");
    assert_eq!(pages, 1);
    assert!(
        !runs.iter().any(|r| r.y >= HEADER_Y),
        "a one page day has no running header"
    );
    assert_eq!(daily_order(&runs), DAILY_HEADINGS.to_vec());
    for must in [
        label.as_str(),
        "Rain rig on standby.",
        "Production order · numbers are scene numbers",
        "Created using OpenFrame",
        "Page 1 of 1",
        "DRAFT",
        "Total · 2 scenes",
    ] {
        assert!(
            runs.iter().any(|r| r.text.contains(must)),
            "{must} missing: {:?}",
            texts(&runs)
        );
    }
    assert!(
        !runs
            .iter()
            .any(|r| r.text.to_lowercase().contains("illustrative")),
        "no sample banner in a real export"
    );
    // Scene order prints the production order (1, 2) and the scene number (2, 3) apart.
    let order_y = runs.iter().find(|r| r.text == "SCENE ORDER").unwrap().y;
    let below: Vec<&str> = runs
        .iter()
        .filter(|r| r.y < order_y - 20.0 && r.y > FOOTER_Y && r.x < 130.0)
        .map(|r| r.text.as_str())
        .collect();
    assert_eq!(below, vec!["#", "SCENE", "1", "2", "2", "3"], "{below:?}");
    // Nothing is dated by the export clock: the same day gives the same bytes.
    let a = export(
        &film,
        "schedule.export_daily_view",
        json!({ "dayId": day }),
        "d1.pdf",
    );
    let b = export(
        &film,
        "schedule.export_daily_view",
        json!({ "dayId": day }),
        "d2.pdf",
    );
    assert_eq!(std::fs::read(a).unwrap(), std::fs::read(b).unwrap());
}

/// A day with every scene of the film and a long note: it runs onto a second page.
fn heavy_day(film: &Film) {
    let env = &film.env;
    for n in 5..=14 {
        let s = super::pdf_samples::strip_id(env, n);
        env.ok(
            "schedule.move_strip",
            json!({ "stripId": s, "dayId": film.days[0] }),
        );
    }
    let notes: Vec<String> = (1..=30)
        .map(|i| format!("Note line {i}: the unit moves between the causeway and the bus stand."))
        .collect();
    env.ok(
        "schedule.set_day_notes",
        json!({ "dayId": film.days[0], "notes": notes.join("\n") }),
    );
}

#[test]
fn a_heavy_daily_view_carries_the_running_header_and_repeats_table_titles() {
    let film = build_film();
    heavy_day(&film);
    let (runs, pages) = daily(&film, 0, "heavy.pdf");
    assert!(pages >= 2, "{pages} pages");
    let header = "MONSOON LETTERS \u{b7} SHOOT DAY 1 \u{b7} DAILY VIEW";
    check_furniture(&runs, pages, header, &DAILY_HEADINGS);
    assert_eq!(daily_order(&runs), DAILY_HEADINGS.to_vec());
    // The 14 scenes appear once each in the order table, which repeats its title when it spans.
    let order_pages: Vec<usize> = runs
        .iter()
        .filter(|r| r.text == "SCENE ORDER")
        .map(|r| r.page)
        .collect();
    let rows_pages: std::collections::BTreeSet<usize> = runs
        .iter()
        .filter(|r| r.text.starts_with("EXT.") || r.text.starts_with("INT."))
        .map(|r| r.page)
        .collect();
    for p in rows_pages {
        assert!(
            order_pages.contains(&p) || p == 1 && order_pages.is_empty(),
            "page {p} has scene rows but no table title"
        );
    }
}

#[test]
fn empty_and_off_days_print_the_sentences_of_the_screen() {
    let film = build_film();
    // The off day (third day) has nothing scheduled.
    let (runs, pages) = daily(&film, 2, "off.pdf");
    assert_eq!(pages, 1);
    for must in [
        "Off Day",
        "OFF DAY",
        "No scenes scheduled.",
        "LOCATION MISSING.",
        "No cast confirmed for these scenes.",
        "Off days have no call sheet.",
        "No day notes.",
        "No confirmed props, wardrobe or other items for these scenes.",
        "Nothing scheduled yet.",
    ] {
        assert!(
            runs.iter().any(|r| r.text == must),
            "{must} missing: {:?}",
            texts(&runs)
        );
    }
    // A day without a call sheet says so.
    let (runs, _) = daily(&film, 1, "nocs.pdf");
    assert!(
        runs.iter()
            .any(|r| r.text == "No call sheet yet for this day.")
    );
    assert!(runs.iter().any(|r| r.text.starts_with("Call times:")));
}

#[test]
#[ignore = "writes the sample PDFs to release-out/pdf-samples"]
fn write_daily_view_design_samples() {
    let film = build_film();
    film.env
        .ok("callsheets.create", json!({ "dayId": film.days[0] }));
    film.env.ok(
        "schedule.set_day_notes",
        json!({ "dayId": film.days[0], "notes": "Basement water pumped from 05:00." }),
    );
    let (_, _) = daily(&film, 0, "e.pdf");
    keep_sample(
        &film.env.dir.path().join("e.pdf").to_string_lossy(),
        "14-daily-view-v2.pdf",
    );
    let _ = daily(&film, 2, "f.pdf");
    keep_sample(
        &film.env.dir.path().join("f.pdf").to_string_lossy(),
        "15-daily-view-v2-off-day.pdf",
    );
    heavy_day(&film);
    let _ = daily(&film, 0, "g.pdf");
    keep_sample(
        &film.env.dir.path().join("g.pdf").to_string_lossy(),
        "16-daily-view-v2-heavy.pdf",
    );
}
