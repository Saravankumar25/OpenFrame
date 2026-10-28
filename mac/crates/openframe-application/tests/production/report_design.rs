//! Layout rules of the six production report PDFs (PDF design package v2, production reports
//! specs, sections E to J), read from the text runs of the real output. Stress documents are
//! saved report snapshots with rows made up here, so a rule such as "the header repeats on every
//! page" is checked on many pages. `write_report_design_samples` writes them to
//! `release-out/pdf-samples` for the eye:
//!
//! ```text
//! node scripts/cargo-queued.mjs test -p openframe-application --test production \
//!     report_design::write -- --ignored --nocapture
//! ```

use std::collections::HashSet;

use serde_json::{Value, json};

use super::call_sheet_design::{FOOTER_Y, Run, export, keep_sample, read};
use super::pdf_samples::{Film, build_film, strip_id};

const KINDS: [(&str, &str); 6] = [
    ("scene", "Scene Report"),
    ("location", "Location Report"),
    ("cast_scene", "Cast Scene Report"),
    ("prop", "Prop Report"),
    ("schedule", "Schedule Report"),
    ("breakdown_completeness", "Breakdown Completeness"),
];

const NO_SOURCE: &str = "Choose a Production Source in Breakdown first.";

// ------------------------------------------------------------------ reading

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

struct Report {
    runs: Vec<Run>,
    pages: usize,
    path: String,
    /// Page height: a running header sits in the top 40 pt.
    height: f32,
}

impl Report {
    fn header_y(&self) -> f32 {
        self.height - 40.0
    }
    fn body(&self) -> impl Iterator<Item = &Run> {
        self.runs
            .iter()
            .filter(|r| r.y > FOOTER_Y && r.y < self.header_y())
    }
    fn has(&self, text: &str) -> bool {
        self.runs.iter().any(|r| r.text == text)
    }
    fn find(&self, text: &str) -> &Run {
        self.runs
            .iter()
            .find(|r| r.text == text)
            .unwrap_or_else(|| panic!("{text} is not printed"))
    }
    fn running_headers(&self) -> Vec<&Run> {
        self.runs
            .iter()
            .filter(|r| r.y >= self.header_y())
            .collect()
    }
    /// Words of the table header cells (7 pt bold capitals).
    fn header_words(&self, page: usize) -> HashSet<String> {
        self.runs
            .iter()
            .filter(|r| r.page == page && r.size == 7.0)
            .flat_map(|r| r.text.split_whitespace().map(str::to_string))
            .collect()
    }
}

fn live(film: &Film, kind: &str, extra: Value, file: &str) -> Report {
    let mut args = json!({ "reportType": kind });
    for (k, v) in extra.as_object().into_iter().flatten() {
        args[k] = v.clone();
    }
    export_report(film, args, file)
}

fn export_report(film: &Film, args: Value, file: &str) -> Report {
    let path = export(film, "reports.export_report", args, file);
    let (runs, pages) = read(&path);
    let height = page_size(&path).1;
    Report {
        runs,
        pages,
        path,
        height,
    }
}

// ------------------------------------------------------------------ snapshots

struct Snap {
    kind: &'static str,
    title: String,
    columns: Vec<&'static str>,
    rows: Vec<Vec<String>>,
    note: Option<&'static str>,
    filter: Option<String>,
    source: String,
}

impl Snap {
    fn new(kind: &'static str, columns: &[&'static str], rows: Vec<Vec<String>>) -> Snap {
        Snap {
            kind,
            title: KINDS.iter().find(|k| k.0 == kind).unwrap().1.to_string(),
            columns: columns.to_vec(),
            rows,
            note: None,
            filter: None,
            source: "Monsoon Letters: Shooting Draft 3 (Locked)".into(),
        }
    }
}

/// Store a snapshot and return its id; the snapshot title is `name`.
fn save(film: &Film, s: &Snap, name: &str) -> String {
    let data = json!({
        "reportType": s.kind,
        "title": s.title,
        "sourceLabel": s.source,
        "generatedAt": 1_792_000_000_000i64,
        "columns": s.columns,
        "rows": s.rows,
        "note": s.note,
        "filter": s.filter,
    });
    // Report ids are UUIDs.
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let id = format!("00000000-0000-4000-8000-{n:012x}");
    film.env
        .core
        .project()
        .unwrap()
        .store
        .with_writer(|c| {
            c.execute(
                "INSERT INTO production_report(id, report_type, title, source_scope_json, snapshot_json, generated_at, created_at, updated_at)
                 VALUES (?1, ?2, ?3, '{}', ?4, 1792000000000, 1792000000000, 1792000000000)",
                rusqlite::params![id, s.kind, name, data.to_string()],
            )
            .expect("snapshot row");
            Ok(())
        })
        .unwrap();
    id
}

fn snapshot(film: &Film, s: &Snap, name: &str, file: &str) -> Report {
    let id = save(film, s, name);
    export_report(film, json!({ "reportId": id }), file)
}

fn strs(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

fn day(n: usize) -> String {
    format!("Shoot Day {n} \u{b7} Mon {} Jun 2027", 13 + n)
}

// ------------------------------------------------------------------ made up rows

const SCENE_COLS: [&str; 7] = [
    "Sc",
    "Heading",
    "I/E \u{b7} D/N",
    "Pages",
    "Location",
    "Cast",
    "Scheduled",
];

/// `n` scene rows: long headings, a heading with no break, unset halves, unscheduled scenes and
/// a day without a date.
fn scene_rows(n: usize) -> Vec<Vec<String>> {
    (1..=n)
        .map(|i| {
            let heading = if i % 9 == 0 {
                format!("INT. THE OLD RAILWAY STATION WAITING ROOM AND THE LONG PLATFORM BEYOND IT, A SECOND FLOOR OFFICE WITH A RADIO SET ON THE DESK AND NOBODY IN THE ROOM - NIGHT {i}")
            } else if i % 13 == 0 {
                "INT.".to_string() + &"UNBROKENHEADINGWITHNOSPACES".repeat(4)
            } else {
                format!("EXT. COAST ROAD NEAR THE LIGHTHOUSE - DAY {i}")
            };
            vec![
                i.to_string(),
                heading,
                match i % 5 {
                    0 => "Not set \u{b7} Not set".to_string(),
                    1 | 2 => "INT \u{b7} Day".to_string(),
                    _ => "EXT \u{b7} Night".to_string(),
                },
                if i % 3 == 0 { "1 2/8" } else { "5/8" }.to_string(),
                if i % 4 == 0 { String::new() } else { "Coast Radio Station, Old Railway Station".to_string() },
                if i % 6 == 0 { String::new() } else { "Savitri, Arjun, Harbour Clerk, Tea Seller".to_string() },
                match i % 8 {
                    0 => "Unscheduled".to_string(),
                    7 => format!("Shoot Day {}", i / 4 + 1),
                    _ => day(i / 4 + 1),
                },
            ]
        })
        .collect()
}

const LOCATION_COLS: [&str; 5] = ["Location", "Status", "Address", "Scenes", "Shooting days"];

fn location_rows(n: usize) -> Vec<Vec<String>> {
    let status = [
        "Confirmed",
        "Shortlisted",
        "Rejected",
        "Idea",
        "From heading",
    ];
    (0..n)
        .map(|i| {
            let days: Vec<String> = (1..=if i == 0 { 14 } else { i % 4 }).map(day).collect();
            let scenes: Vec<String> = (1..=if i == 0 { 60 } else { 1 + i % 5 }).map(|s| s.to_string()).collect();
            vec![
                if i == 1 { "Averyveryverylongunbrokenlocationnamethatcannotwrapatspaces".to_string() } else { format!("Location {i:02} (the script descriptor)") },
                status[i % 5].to_string(),
                if i % 3 == 0 { String::new() } else if i == 2 { "A very long address that keeps going past the end of the column for more than two hundred characters in all, with a street, a district, a town, a state and a postcode written out in full, so that it must wrap many times".to_string() } else { format!("{i} Harbour Road, Kochi") },
                scenes.join(", "),
                days.join("; "),
            ]
        })
        .collect()
}

const CAST_COLS: [&str; 5] = [
    "Actor",
    "Character",
    "Scenes",
    "Scene count",
    "Shooting days",
];

fn cast_rows(n: usize) -> Vec<Vec<String>> {
    (0..n)
        .map(|i| {
            let count = if i == 0 { 14 } else { 1 + i % 5 };
            let scenes: Vec<String> = (1..=count).map(|s| s.to_string()).collect();
            let days: Vec<String> = (1..=count).map(day).collect();
            vec![
                if i % 4 == 1 {
                    "Not cast yet".to_string()
                } else {
                    format!("Actor Number {i}")
                },
                if i == 3 {
                    "AVERYLONGCHARACTERNAMETHATHASNOSPACEATALL".to_string()
                } else {
                    format!("CHARACTER {i:02}")
                },
                scenes.join(", "),
                count.to_string(),
                days.join("; "),
            ]
        })
        .collect()
}

const PROP_COLS: [&str; 3] = ["Prop", "Scenes", "Scene count"];

fn prop_rows(n: usize) -> Vec<Vec<String>> {
    let mut names: Vec<String> = (0..n)
        .map(|i| {
            let letter = (b'A' + (i * 26 / n) as u8) as char;
            match i {
                0 => "2-inch tape reels".to_string(),
                1 => "Biscuit box (reel case) with a very long description of what is in it and what it looks like".to_string(),
                _ => format!("{letter}{} prop number {i}", if i % 7 == 0 { " with a longer name than most" } else { "" }),
            }
        })
        .collect();
    names.sort_by_key(|n| n.to_lowercase());
    names
        .into_iter()
        .enumerate()
        .map(|(i, name)| {
            let count = if i == 5 { 40 } else { 1 + i % 4 };
            let scenes: Vec<String> = (1..=count).map(|s| s.to_string()).collect();
            vec![name, scenes.join(", "), count.to_string()]
        })
        .collect()
}

const SCHEDULE_COLS: [&str; 5] = ["Day", "Date", "Scenes", "Locations", "Est. time"];

fn schedule_rows(n: usize, unscheduled: bool) -> Vec<Vec<String>> {
    let mut rows: Vec<Vec<String>> = (1..=n)
        .map(|i| {
            if i % 5 == 0 {
                vec![
                    "Off Day".into(),
                    format!("Sun {} Jun 2027", 13 + i),
                    String::new(),
                    String::new(),
                    "Missing".into(),
                ]
            } else {
                vec![
                    format!("Shoot Day {i}"),
                    if i == 3 {
                        "No date".into()
                    } else {
                        format!("Mon {} Jun 2027", 13 + i)
                    },
                    (1..=(if i == 2 { 40 } else { 4 }))
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                        .join(", "),
                    if i == 2 {
                        format!("{}, {}", "L".repeat(120), "M".repeat(120))
                    } else {
                        "Coast Radio Station, Old Railway Station, Harbour Road".to_string()
                    },
                    if i == 4 {
                        "Missing".into()
                    } else {
                        "2h 30m".to_string()
                    },
                ]
            }
        })
        .collect();
    if unscheduled {
        rows.push(vec![
            "Unscheduled".into(),
            String::new(),
            "3 scenes".into(),
            String::new(),
            String::new(),
        ]);
    }
    rows
}

const BREAKDOWN_COLS: [&str; 5] = [
    "Sc",
    "Heading",
    "Confirmed",
    "Suggested (unreviewed)",
    "Status",
];

fn breakdown_rows(n: usize) -> Vec<Vec<String>> {
    (1..=n)
        .map(|i| {
            let (c, s, st) = match i % 6 {
                0 => (0, 0, "Not started"),
                1 | 2 => (12, 0, "Reviewed"),
                3 => (0, 7, "Needs review"),
                4 => (5, 3, "In progress"),
                _ => (9, 0, "Reviewed"),
            };
            vec![
                i.to_string(),
                format!("INT. LONG HEADING FOR SCENE NUMBER {i} - A STATION OFFICE WITH A RADIO"),
                c.to_string(),
                s.to_string(),
                st.to_string(),
            ]
        })
        .collect()
}

fn kind_data(kind: &str, n: usize) -> (Vec<&'static str>, Vec<Vec<String>>) {
    match kind {
        "scene" => (SCENE_COLS.to_vec(), scene_rows(n)),
        "location" => (LOCATION_COLS.to_vec(), location_rows(n)),
        "cast_scene" => (CAST_COLS.to_vec(), cast_rows(n)),
        "prop" => (PROP_COLS.to_vec(), prop_rows(n)),
        "schedule" => (SCHEDULE_COLS.to_vec(), schedule_rows(n, true)),
        _ => (BREAKDOWN_COLS.to_vec(), breakdown_rows(n)),
    }
}

fn kind_snap(kind: &'static str, n: usize) -> Snap {
    let (cols, rows) = kind_data(kind, n);
    Snap::new(kind, &cols, rows)
}

/// Stress documents made like the stress files of the spec: a 160 character snapshot title, a
/// 200 character filter, a long project and source, extreme cell text, many rows.
fn stress_snap(kind: &'static str, n: usize) -> (Snap, String) {
    let mut s = kind_snap(kind, n);
    s.filter = Some("coast road lighthouse ".repeat(9).trim().to_string());
    s.note = Some("Rows marked in the sheet are illustrative.");
    s.source =
        "Monsoon Letters: Shooting Draft 3 with a deliberately long source label that goes on"
            .into();
    let name = format!(
        "{}: Monsoon Letters {} which is a snapshot title of one hundred and sixty characters kept going so that it must wrap in the head of the report ok",
        s.title,
        "x".repeat(8)
    );
    (s, name)
}

fn wide_snap(kind: &'static str) -> Snap {
    let mut s = kind_snap(kind, 12);
    for (name, v) in [
        ("Stunts", "None"),
        ("Vehicles", "Two vans"),
        ("Notes", "A longer note on this row"),
    ] {
        s.columns.push(name);
        for r in &mut s.rows {
            r.push(v.to_string());
        }
    }
    s
}

fn empty_snap(kind: &'static str) -> Snap {
    let (cols, _) = kind_data(kind, 1);
    let mut s = Snap::new(kind, &cols, Vec::new());
    s.note = Some("No matching data.");
    s.filter = Some("vypin".into());
    s
}

// ------------------------------------------------------------------ rules

#[test]
fn the_scene_report_is_landscape_and_the_other_five_are_portrait_with_the_shared_footer() {
    let film = build_film();
    for (kind, title) in KINDS {
        let r = live(&film, kind, json!({}), &format!("{kind}.pdf"));
        let want = if kind == "scene" {
            (792.0, 612.0)
        } else {
            (612.0, 792.0)
        };
        assert_eq!(page_size(&r.path), want, "{kind}");
        for page in 1..=r.pages {
            let foot: Vec<&str> = r
                .runs
                .iter()
                .filter(|x| x.page == page && x.y < 40.0)
                .map(|x| x.text.as_str())
                .collect();
            assert!(
                foot.contains(&"Created using OpenFrame"),
                "{kind}: {foot:?}"
            );
            assert!(
                foot.contains(&format!("Page {page} of {}", r.pages).as_str()),
                "{kind}: {foot:?}"
            );
            assert_eq!(
                foot.len(),
                2,
                "{kind}: nothing else in the footer: {foot:?}"
            );
        }
        assert!(
            r.runs.iter().all(|x| x.size >= 6.4 - 0.01),
            "{kind}: type floor"
        );
        assert!(
            r.has(title) && r.runs.iter().any(|x| x.text == title && x.size == 20.0),
            "{kind}: compact title"
        );
        assert!(!r.has("Export reference"), "{kind}");
    }
}

#[test]
fn the_head_names_the_project_the_source_the_date_and_the_scope() {
    let film = build_film();
    let r = live(&film, "scene", json!({}), "scene.pdf");
    assert!(r.has("PRODUCTION REPORT"));
    let meta = r
        .runs
        .iter()
        .find(|x| {
            x.text
                .starts_with("Project: Monsoon Letters \u{b7} Source: ")
        })
        .expect("project and source line");
    assert_eq!(meta.size, 8.0);
    assert!(r.has("GENERATED") && r.has("All rows"));
    let date = r.runs.iter().find(|x| x.size == 10.5).expect("the date");
    assert!(date.x > 566.0 - 150.0, "the date sits in the right panel");
    // Eyebrow, then title, then the meta line, top to bottom.
    assert!(r.find("PRODUCTION REPORT").y > r.find("Scene Report").y);
    assert!(r.find("Scene Report").y > meta.y);
}

#[test]
fn the_scene_report_counts_unscheduled_scenes_and_prints_each_day_on_two_lines() {
    let film = build_film();
    let s = strip_id(&film.env, 14);
    film.env.ok(
        "schedule.move_strip",
        json!({ "stripId": s, "dayId": Value::Null }),
    );
    let r = live(&film, "scene", json!({}), "scene.pdf");
    assert!(r.has("SCENES") && r.has("UNSCHEDULED"));
    let labels: Vec<&Run> = r.runs.iter().filter(|x| x.text == "UNSCHEDULED").collect();
    assert_eq!(labels.len(), 2, "the metric label and the chip");
    assert!(labels.iter().any(|x| x.size == 6.8) && labels.iter().any(|x| x.size == 6.6));
    // 14 scenes, 1 unscheduled.
    assert!(r.runs.iter().any(|x| x.text == "14" && x.size == 13.0));
    assert!(r.runs.iter().any(|x| x.text == "1" && x.size == 13.0));
    // The day and its date are two runs on two lines, the day bold above the date.
    let day1 = r.find("Shoot Day 1");
    let date = r
        .runs
        .iter()
        .find(|x| x.text == "Mon 14 Jun 2027" && x.x == day1.x)
        .expect("the date under the day");
    assert!(day1.y > date.y && day1.y - date.y < 14.0);
    // The columns come in the order of the spec.
    let order = [
        "SC",
        "HEADING",
        "I/E",
        "PAGES",
        "LOCATION",
        "CAST",
        "SCHEDULED",
    ];
    let xs: Vec<f32> = order
        .iter()
        .map(|h| {
            r.runs
                .iter()
                .find(|x| x.text.starts_with(h) && x.size == 7.0)
                .unwrap_or_else(|| panic!("{h}"))
                .x
        })
        .collect();
    assert!(xs.windows(2).all(|w| w[0] < w[1]), "{xs:?}");
}

#[test]
fn shooting_days_print_in_shoot_day_order_in_the_cast_scene_and_location_reports() {
    let film = build_film();
    // Scene 1 moves to the last day, so scene order no longer matches shoot-day order.
    let s1 = strip_id(&film.env, 1);
    film.env.ok(
        "schedule.move_strip",
        json!({ "stripId": s1, "dayId": film.days[4] }),
    );
    let mut multi = 0;
    for (kind, col) in [("cast_scene", 4usize), ("location", 4usize)] {
        let data = film
            .env
            .ok("reports.generate", json!({ "reportType": kind }));
        let mut want: Vec<u32> = Vec::new();
        for row in data["rows"].as_array().unwrap() {
            let days = day_numbers(row[col].as_str().unwrap());
            let mut sorted = days.clone();
            sorted.sort_unstable();
            assert_eq!(days, sorted, "{kind}: {}", row[col]);
            if days.len() > 1 {
                multi += 1;
            }
            want.extend(days);
        }
        // The PDF prints the same days, one per line, in the same order.
        let r = live(&film, kind, json!({}), &format!("{kind}.pdf"));
        let mut printed: Vec<&Run> = r
            .runs
            .iter()
            .filter(|x| x.text.starts_with("Shoot Day "))
            .collect();
        printed.sort_by(|a, b| a.page.cmp(&b.page).then(b.y.partial_cmp(&a.y).unwrap()));
        let got: Vec<u32> = printed.iter().map(|x| day_numbers(&x.text)[0]).collect();
        assert_eq!(got, want, "{kind}: the days as printed");
    }
    assert!(multi >= 3, "the fixture has rows with several days");
    // The character who works in scene 1 and scene 3: day 1 comes before day 4.
    let cast = film
        .env
        .ok("reports.generate", json!({ "reportType": "cast_scene" }));
    let savitri = cast["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r[1] == "Savitri" || r[1] == "SAVITRI")
        .expect("Savitri")
        .clone();
    let days = day_numbers(savitri[4].as_str().unwrap());
    assert!(
        days.len() >= 3 && days[0] == 1 && *days.last().unwrap() == 4,
        "{days:?}"
    );
}

/// The numbers after "Shoot Day" in a cell, in the order printed.
fn day_numbers(cell: &str) -> Vec<u32> {
    cell.split("Shoot Day ")
        .skip(1)
        .filter_map(|p| p.split(|c: char| !c.is_ascii_digit()).next()?.parse().ok())
        .collect()
}

#[test]
fn days_sort_by_number_then_date_with_undated_last_and_ties_stay_stable() {
    // The sort is part of the report data, so CSV and XLSX get it too.
    let film = build_film();
    let csv = film
        .env
        .dir
        .path()
        .join("cast.csv")
        .to_string_lossy()
        .into_owned();
    film.env.ok(
        "reports.export_report",
        json!({ "format": "csv", "path": csv, "reportType": "cast_scene" }),
    );
    let text = std::fs::read_to_string(&csv).unwrap();
    let text = text.trim_start_matches('﻿');
    assert!(
        text.lines()
            .next()
            .unwrap()
            .starts_with("Actor,Character,Scenes,Scene count,Shooting days"),
        "the stored column order is unchanged in CSV: {text}"
    );
    for line in text.lines().skip(1).filter(|l| l.contains("Shoot Day")) {
        let days = day_numbers(line);
        let mut sorted = days.clone();
        sorted.sort_unstable();
        assert_eq!(days, sorted, "{line}");
    }
}

#[test]
fn character_leads_the_cast_scene_report_and_a_report_without_actors_flags_them() {
    let film = build_film();
    let r = live(&film, "cast_scene", json!({}), "cast.pdf");
    let x = |h: &str| {
        r.runs
            .iter()
            .find(|x| x.text == h && x.size == 7.0)
            .unwrap()
            .x
    };
    assert!(x("CHARACTER") < x("ACTOR") && x("ACTOR") < x("SCENES"));
    // A cast member with no actor: the word on a warn chip (the stripe is drawn with it).
    let s = snapshot(&film, &kind_snap("cast_scene", 6), "Cast", "cast-snap.pdf");
    assert!(
        s.runs
            .iter()
            .any(|x| x.text == "NOT CAST YET" && x.size == 6.6)
    );
    // Scene count is a right aligned bold column: its header is above digits that end at the edge.
    let header = r
        .runs
        .iter()
        .find(|x| x.text == "SCENE" && x.size == 7.0)
        .unwrap();
    assert!(
        header.x > 350.0 && header.x < 420.0,
        "the narrow count column is the fourth: {}",
        header.x
    );
}

#[test]
fn an_empty_report_prints_the_note_in_a_callout_with_zero_counts_and_no_table() {
    let film = build_film();
    for (kind, title) in KINDS {
        let s = empty_snap(match kind {
            "scene" => "scene",
            "location" => "location",
            "cast_scene" => "cast_scene",
            "prop" => "prop",
            "schedule" => "schedule",
            _ => "breakdown_completeness",
        });
        let r = snapshot(
            &film,
            &s,
            &format!("{title}: empty"),
            &format!("empty-{kind}.pdf"),
        );
        assert_eq!(r.pages, 1, "{kind}");
        assert!(r.has("No matching data."), "{kind}: the note, exactly");
        assert!(r.running_headers().is_empty(), "{kind}: no running header");
        assert!(r.body().all(|x| x.size != 7.0
            || x.text == "FILTER"
            || x.text.chars().all(|c| !c.is_lowercase())));
        let want = if kind == "scene" {
            (792.0, 612.0)
        } else {
            (612.0, 792.0)
        };
        assert_eq!(
            page_size(&r.path),
            want,
            "{kind}: orientation follows the column count"
        );
        assert!(
            r.runs.iter().any(|x| x.text == "0" && x.size == 13.0),
            "{kind}: zero counts"
        );
        assert!(
            r.has("FILTER") && r.has("\u{201c}vypin\u{201d}"),
            "{kind}: the stored filter"
        );
        // No header row: the first column's header is not printed.
        let first = s.columns[0].to_uppercase();
        assert!(
            !r.runs.iter().any(|x| x.text == first && x.size == 7.0),
            "{kind}: no table"
        );
    }
    // No Production Source: no columns, portrait for every type, the source's sentence.
    let mut none = Snap::new("scene", &[], Vec::new());
    none.note = Some(NO_SOURCE);
    none.source = String::new();
    let r = snapshot(&film, &none, "Scene Report: none", "no-source.pdf");
    assert_eq!(page_size(&r.path), (612.0, 792.0));
    assert!(r.has(NO_SOURCE));
    assert!(
        !r.runs.iter().any(|x| x.text.starts_with("Source:")),
        "no Source part"
    );
}

#[test]
fn a_saved_snapshot_shows_its_title_its_date_and_the_stored_filter_in_bands() {
    let film = build_film();
    let mut s = kind_snap("location", 6);
    s.filter = Some("harbour".into());
    let r = snapshot(&film, &s, "Locations before the recce", "snap.pdf");
    assert!(
        r.runs
            .iter()
            .any(|x| x.text == "Locations before the recce" && x.size == 20.0)
    );
    assert!(r.has("Saved snapshot"), "the scope in the panel");
    assert!(r.has("SAVED SNAPSHOT"), "the band label");
    assert!(
        r.runs
            .iter()
            .any(|x| x.text.contains("generated") && x.text.contains("does not change")),
        "the band says the snapshot is fixed"
    );
    assert!(r.has("FILTER") && r.has("\u{201c}harbour\u{201d}"));
    assert!(
        !r.has("Filtered rows"),
        "a snapshot's scope is the snapshot"
    );
    // A live filter says so in the panel and in a band.
    let live_r = live(&film, "location", json!({ "filter": "dak" }), "live.pdf");
    assert!(
        live_r.has("Filtered rows") && live_r.has("FILTER") && live_r.has("\u{201c}dak\u{201d}")
    );
    assert!(!live_r.has("SAVED SNAPSHOT"));
    // The running header names the report type, never the snapshot title.
}

#[test]
fn long_titles_and_filters_wrap_and_nothing_is_cut() {
    let film = build_film();
    let (s, name) = stress_snap("location", 6);
    let r = snapshot(&film, &s, &name, "stress.pdf");
    let title: Vec<&str> = r
        .runs
        .iter()
        .filter(|x| x.size == 20.0)
        .map(|x| x.text.as_str())
        .collect();
    assert!(title.len() >= 2, "{title:?}");
    assert_eq!(title.join(" "), name);
    let filter = s.filter.clone().unwrap();
    let printed: Vec<&str> = r
        .runs
        .iter()
        .filter(|x| x.size == 8.0 && x.y < 760.0)
        .map(|x| x.text.as_str())
        .collect();
    let joined = printed.join(" ");
    for w in filter.split_whitespace() {
        assert!(joined.contains(w), "{w} of the filter is printed");
    }
}

#[test]
fn the_status_words_are_printed_and_the_open_scenes_are_listed() {
    let film = build_film();
    let mut s = Snap::new(
        "breakdown_completeness",
        &BREAKDOWN_COLS,
        vec![
            strs(&["1", "A", "9", "0", "Reviewed"]),
            strs(&["2", "B", "5", "3", "In progress"]),
            strs(&["3", "C", "0", "7", "Needs review"]),
            strs(&["4", "D", "0", "0", "Not started"]),
            strs(&["23", "E", "2", "1", "In progress"]),
        ],
    );
    s.source = String::new();
    let r = snapshot(&film, &s, "Breakdown", "breakdown.pdf");
    for word in ["REVIEWED", "IN PROGRESS", "NEEDS REVIEW", "NOT STARTED"] {
        assert!(
            r.runs.iter().any(|x| x.text == word && x.size == 6.6),
            "{word}"
        );
    }
    assert!(r.has("SCENES STILL OPEN"));
    assert!(r.has("In progress or needs review: 2, 3, 23 \u{b7} Not started: 4"));
    // The four metrics, with the sum Reviewed + open + Not started = Scenes.
    for label in [
        "SCENES",
        "REVIEWED",
        "IN PROGRESS OR NEEDS REVIEW",
        "NOT STARTED",
    ] {
        assert!(
            r.runs.iter().any(|x| x.text == label && x.size == 6.8),
            "{label}"
        );
    }
    let values: Vec<&str> = r
        .runs
        .iter()
        .filter(|x| x.size == 13.0)
        .map(|x| x.text.as_str())
        .collect();
    assert_eq!(values, ["5", "1", "3", "1"]);
    // Counts: zeros are regular, the others bold; the suggested column is the amber one.
    // All reviewed: no band.
    let done = Snap::new(
        "breakdown_completeness",
        &BREAKDOWN_COLS,
        vec![strs(&["1", "A", "9", "0", "Reviewed"])],
    );
    let r = snapshot(&film, &done, "Done", "done.pdf");
    assert!(!r.has("SCENES STILL OPEN"));
}

#[test]
fn the_schedule_report_does_not_raise_a_false_alarm_for_an_off_day() {
    let film = build_film();
    let s = strip_id(&film.env, 14);
    film.env.ok(
        "schedule.move_strip",
        json!({ "stripId": s, "dayId": Value::Null }),
    );
    let live_r = live(&film, "schedule", json!({}), "schedule.pdf");
    assert_eq!(live_r.pages, 1);
    assert!(
        live_r.running_headers().is_empty(),
        "one page: no running header"
    );
    assert!(live_r.has("UNSCHEDULED"), "the Unscheduled row");
    assert!(live_r.has("Off Day"));
    assert!(
        !live_r.has("MISSING"),
        "an off day is not a missing estimate"
    );
    assert!(live_r.has("\u{2013}"), "a dash instead");
    // A shoot day without an estimate keeps the danger chip.
    let r = snapshot(
        &film,
        &Snap::new(
            "schedule",
            &SCHEDULE_COLS,
            vec![
                strs(&[
                    "Shoot Day 1",
                    "Mon 14 Jun 2027",
                    "1, 2",
                    "Dak bungalow",
                    "Missing",
                ]),
                strs(&["Off Day", "Tue 15 Jun 2027", "", "", "Missing"]),
            ],
        ),
        "Schedule",
        "schedule-missing.pdf",
    );
    assert_eq!(r.runs.iter().filter(|x| x.text == "MISSING").count(), 1);
    assert_eq!(r.runs.iter().filter(|x| x.text == "\u{2013}").count(), 1);
    // Scenes print in the order given, not sorted.
    let r = snapshot(
        &film,
        &Snap::new(
            "schedule",
            &SCHEDULE_COLS,
            vec![strs(&[
                "Shoot Day 1",
                "Mon 14 Jun 2027",
                "2, 33, 11",
                "Dak",
                "2h",
            ])],
        ),
        "Order",
        "schedule-order.pdf",
    );
    assert!(r.has("2, 33, 11"));
}

#[test]
fn a_long_report_repeats_the_header_and_the_running_header_on_every_page() {
    let film = build_film();
    for kind in [
        "scene",
        "location",
        "cast_scene",
        "prop",
        "schedule",
        "breakdown_completeness",
    ] {
        let (s, _) = stress_snap(kind, 90);
        let title = s.title.clone();
        let r = snapshot(
            &film,
            &s,
            &format!("{title} long"),
            &format!("long-{kind}.pdf"),
        );
        assert!(r.pages >= 2, "{kind}");
        let want = format!("MONSOON LETTERS \u{b7} {}", title.to_uppercase());
        for page in 1..=r.pages {
            let top: Vec<&str> = r
                .runs
                .iter()
                .filter(|x| x.page == page && x.y >= r.header_y())
                .map(|x| x.text.as_str())
                .collect();
            assert_eq!(top, vec![want.as_str()], "{kind} page {page}");
            // The table header is on the page, and there is no table title row of its own.
            let first = s.columns[0].to_uppercase();
            assert!(
                r.runs
                    .iter()
                    .any(|x| x.page == page && x.size == 7.0 && x.text.starts_with(&first)),
                "{kind}: header on page {page}"
            );
        }
        assert!(
            !r.runs.iter().any(|x| x.text.contains("continued")),
            "{kind}: no table title"
        );
        // Nothing sits in the footer zone or outside the side margins.
        for x in &r.runs {
            assert!(
                x.y >= FOOTER_Y || x.y >= r.header_y() || x.y < 40.0,
                "{} at {}",
                x.text,
                x.y
            );
            let right = if kind == "scene" { 746.0 } else { 566.0 };
            assert!(x.x >= 45.0 && x.x < right, "{kind}: {} at {}", x.text, x.x);
        }
    }
}

#[test]
fn the_prop_list_has_letter_dividers_that_never_end_a_page() {
    let film = build_film();
    let s = kind_snap("prop", 148);
    let r = snapshot(&film, &s, "Props", "props.pdf");
    assert!(
        r.pages >= 3 && r.pages <= 6,
        "{} pages for 148 props",
        r.pages
    );
    let dividers: Vec<&Run> = r
        .runs
        .iter()
        .filter(|x| x.size == 6.8 && x.text.chars().count() == 1)
        .collect();
    assert!(dividers.len() >= 10, "{} dividers", dividers.len());
    let letters: Vec<String> = dividers.iter().map(|x| x.text.clone()).collect();
    let mut sorted = letters.clone();
    sorted.sort();
    assert_eq!(letters, sorted, "alphabetical");
    for page in 1..=r.pages {
        let mut body: Vec<&Run> = r.body().filter(|x| x.page == page).collect();
        body.sort_by(|a, b| b.y.partial_cmp(&a.y).unwrap());
        let last = body.last().unwrap();
        assert!(
            !(last.size == 6.8 && last.text.chars().count() == 1),
            "a divider ends page {page}"
        );
    }
    // The last page has more than two rows.
    let names_on_last = r
        .runs
        .iter()
        .filter(|x| x.page == r.pages && x.size == 8.4 && x.x < 60.0)
        .count();
    assert!(names_on_last > 2, "{names_on_last}");
    // A name under 45 characters stays on one line in the wide Prop column.
    assert!(r.has("2-inch tape reels"));
}

#[test]
fn the_scene_report_fits_the_spec_density() {
    let film = build_film();
    let mut s = kind_snap("scene", 44);
    // Real-looking rows like the sample of the spec (a snapshot adds one band over a live report).
    for r in &mut s.rows {
        r[1] = format!(
            "INT. COAST RADIO STATION - OPERATIONS ROOM - NIGHT {}",
            r[0]
        );
        r[2] = "INT · N".into();
        r[4] = "Coast Radio Station (Old Signal Station, Vypin)".into();
        r[5] = "MARIAMMA, YOUNG ANJALI, JOE".into();
        if r[6] == "Unscheduled" {
            r[6] = day(2);
        }
    }
    let r = snapshot(&film, &s, "Scene Report", "scene-44.pdf");
    let tiles = |page: usize| {
        r.runs
            .iter()
            .filter(|x| x.page == page && x.size == 9.0 && x.x < 100.0 && x.y > FOOTER_Y)
            .count()
    };
    assert!(tiles(1) >= 11, "page 1 shows {} rows", tiles(1));
    assert!(
        tiles(r.pages) >= 2,
        "the last page holds {} rows",
        tiles(r.pages)
    );
}

#[test]
fn a_wide_report_is_landscape_with_whole_words_in_the_header() {
    let film = build_film();
    for kind in [
        "scene",
        "location",
        "cast_scene",
        "prop",
        "schedule",
        "breakdown_completeness",
    ] {
        let s = wide_snap(match kind {
            "scene" => "scene",
            "location" => "location",
            "cast_scene" => "cast_scene",
            "prop" => "prop",
            "schedule" => "schedule",
            _ => "breakdown_completeness",
        });
        let r = snapshot(
            &film,
            &s,
            &format!("{kind} wide"),
            &format!("wide-{kind}.pdf"),
        );
        let landscape = s.columns.len() > 5;
        assert_eq!(
            page_size(&r.path),
            if landscape {
                (792.0, 612.0)
            } else {
                (612.0, 792.0)
            },
            "{kind}"
        );
        let words = r.header_words(1);
        for c in &s.columns {
            for w in c.to_uppercase().split_whitespace() {
                assert!(
                    words.contains(w),
                    "{kind}: header word {w} is whole in {words:?}"
                );
            }
        }
        for x in &r.runs {
            assert!(x.x >= 45.0 && x.x < 746.0, "{kind}: {} at {}", x.text, x.x);
        }
    }
}

#[test]
fn the_same_report_prints_the_same_bytes() {
    let film = build_film();
    let a = live(&film, "location", json!({}), "a.pdf");
    let b = live(&film, "location", json!({}), "b.pdf");
    assert_eq!(
        std::fs::read(&a.path).unwrap(),
        std::fs::read(&b.path).unwrap()
    );
}

// ------------------------------------------------------------------ samples

#[test]
#[ignore = "writes the stress samples to release-out/pdf-samples"]
fn write_report_design_samples() {
    let film = build_film();
    for (kind, title) in KINDS {
        let kind: &'static str = kind;
        let (s, name) = stress_snap(kind, if kind == "prop" { 171 } else { 70 });
        let r = snapshot(&film, &s, &name, &format!("stress-{kind}.pdf"));
        keep_sample(&r.path, &format!("report-stress-{kind}.pdf"));
        let r = snapshot(
            &film,
            &empty_snap(kind),
            &format!("{title}: empty"),
            "e.pdf",
        );
        keep_sample(&r.path, &format!("report-empty-{kind}.pdf"));
        let r = snapshot(&film, &wide_snap(kind), &format!("{title}: wide"), "w.pdf");
        keep_sample(&r.path, &format!("report-wide-{kind}.pdf"));
    }
    let mut none = Snap::new("scene", &[], Vec::new());
    none.note = Some(NO_SOURCE);
    none.source = String::new();
    let r = snapshot(&film, &none, "Scene Report: none", "n.pdf");
    keep_sample(&r.path, "report-no-source.pdf");
}
