//! Layout rules of the Day Out of Days PDF (PDF design package v2, Day_Out_of_Days spec sections
//! H and J), read from the text runs of the real output. `write_dood_design_samples` writes the
//! stress samples to `release-out/pdf-samples` for the eye:
//!
//! ```text
//! node scripts/cargo-queued.mjs test -p openframe-application --test production \
//!     dood_design::write -- --ignored --nocapture
//! ```

use openframe_import_export::report::text_width;
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

use super::call_sheet_design::{Run, export, keep_sample, read};
use super::pdf_samples::{Film, build_film, id_of, strip_id};

const PAGE_H: f32 = 612.0;
const HEADER_Y: f32 = PAGE_H - 40.0;
const FOOTER_Y: f32 = 66.0;
const RUNNING: &str = "MONSOON LETTERS \u{b7} DAY OUT OF DAYS";

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

/// 13 shooting days and 2 off days, scenes spread one or two a day, and one correction.
fn long_film() -> Film {
    let film = build_film();
    let env = &film.env;
    let mut days = film.days.clone();
    for i in 0..9 {
        days.push(id_of(&env.ok(
            "schedule.create_day",
            json!({ "scheduleId": film.schedule, "date": format!("2027-06-{}", 19 + i) }),
        )));
    }
    env.ok(
        "schedule.create_day",
        json!({ "scheduleId": film.schedule, "offDay": true, "afterDayId": days[5] }),
    );
    let shoot: Vec<&String> = days
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != 2)
        .map(|(_, d)| d)
        .collect();
    for n in 1..=14i64 {
        let s = strip_id(env, n);
        let target = shoot[((n - 1) as usize * shoot.len()) / 14];
        env.ok(
            "schedule.move_strip",
            json!({ "stripId": s, "dayId": target }),
        );
    }
    film
}

fn correct(film: &Film, row: usize, day: usize, code: &str, note: &str) {
    let v = film.env.ok("schedule.dood", json!({}));
    film.env.ok(
        "schedule.set_dood_override",
        json!({ "scheduleId": film.schedule,
                "castKey": v["rows"][row]["key"], "dayId": v["days"][day]["id"],
                "code": code, "note": note }),
    );
}

/// `n` more cast members, each confirmed in one scene, so the matrix has many rows.
fn add_cast(film: &Film, n: usize) {
    let mut sql = String::new();
    for i in 0..n {
        let k = i % 14 + 1;
        sql.push_str(&format!(
            "INSERT INTO story_character(id,name,position,created_at,updated_at) VALUES ('xc{i}','Crowd {i:02}',{},1,1);\
             INSERT INTO cast_member(id,person_name,character_id,created_at,updated_at) VALUES ('cm_xc{i}','Player {i:02}','xc{i}',1,1);\
             INSERT INTO catalog_item(id,category,name,character_id,location_id,created_at,updated_at) VALUES ('c_xc{i}','Cast','Crowd {i:02}','xc{i}',NULL,1,1);\
             INSERT INTO breakdown_element(id,source_id,scene_id,scene_lineage_id,category,catalog_item_id,display_name,confirmation_state,created_at,updated_at) VALUES ('bx{i}','ps1','s{k}','L{k}','Cast','c_xc{i}','Crowd {i:02}','Confirmed',1,1);",
            100 + i
        ));
    }
    exec(&film.env, &sql);
}

fn dood(film: &Film, args: Value, file: &str) -> (Vec<Run>, usize, String) {
    let path = export(film, "schedule.export_dood", args, file);
    let (runs, pages) = read(&path);
    (runs, pages, path)
}

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

fn centre(r: &Run) -> f32 {
    r.x + text_width(&r.text, r.size, true) / 2.0
}

/// A day header "01", "02", ... (7.2 pt bold) in drawing order.
fn day_tops(runs: &[Run]) -> Vec<&Run> {
    runs.iter()
        .filter(|r| {
            r.size == 7.2
                && r.y < HEADER_Y
                && (r.text == "OFF"
                    || (r.text.len() == 2 && r.text.chars().all(|c| c.is_ascii_digit())))
        })
        .collect()
}

fn count(runs: &[Run], text: &str) -> usize {
    runs.iter().filter(|r| r.text == text).count()
}

#[test]
fn landscape_with_the_running_header_the_shared_footer_and_the_legend_on_every_page() {
    let film = long_film();
    correct(&film, 0, 1, "H", "Held for the causeway reshoot");
    let (runs, pages, path) = dood(&film, json!({}), "d.pdf");
    assert_eq!(page_size(&path), (792.0, 612.0), "US Letter landscape");
    for p in 1..=pages {
        let top: Vec<&str> = runs
            .iter()
            .filter(|r| r.page == p && r.y >= HEADER_Y)
            .map(|r| r.text.as_str())
            .collect();
        assert_eq!(top, vec![RUNNING], "running header on page {p}");
        let foot: Vec<&str> = runs
            .iter()
            .filter(|r| r.page == p && r.y < 40.0)
            .map(|r| r.text.as_str())
            .collect();
        assert!(foot.contains(&"Created using OpenFrame"), "{foot:?}");
        assert!(
            foot.contains(&format!("Page {p} of {pages}").as_str()),
            "{foot:?}"
        );
        assert_eq!(foot.len(), 2, "nothing else in the footer: {foot:?}");
    }
    // The matrix pages carry the legend; the correction page (if any) does not need it.
    let matrix_pages: std::collections::BTreeSet<usize> =
        day_tops(&runs).iter().map(|r| r.page).collect();
    for p in matrix_pages {
        assert!(
            runs.iter().any(|r| r.page == p && r.text == "CODE LEGEND"),
            "legend on page {p}"
        );
    }
    for r in &runs {
        assert!(r.size >= 6.4 - 0.01, "{} is {} pt", r.text, r.size);
        assert!(r.x >= 45.0 && r.x < 748.0, "{} at {}", r.text, r.x);
        assert!(
            r.y >= FOOTER_Y || r.y >= HEADER_Y || r.y < 40.0,
            "{} sits in the footer zone at {}",
            r.text,
            r.y
        );
    }
}

#[test]
fn fifteen_days_fit_one_group_of_equal_columns_with_every_day_once() {
    let film = long_film();
    let (runs, _, _) = dood(&film, json!({}), "d.pdf");
    let tops = day_tops(&runs);
    assert_eq!(tops.len(), 15, "13 shooting days and 2 off days");
    for n in 1..=13 {
        assert_eq!(
            tops.iter().filter(|r| r.text == format!("{n:02}")).count(),
            1,
            "shoot day {n} once"
        );
    }
    assert_eq!(tops.iter().filter(|r| r.text == "OFF").count(), 2);
    assert!(!runs.iter().any(|r| r.text.contains("Columns group")));
    // One width: every neighbouring pair is the same distance apart.
    let xs: Vec<f32> = tops.iter().map(|r| centre(r)).collect();
    let d = xs[1] - xs[0];
    assert!((24.0..=38.0).contains(&d), "{d}");
    assert!(
        xs.windows(2).all(|w| (w[1] - w[0] - d).abs() < 0.05),
        "{xs:?}"
    );
    // The month band names the month and year over the days.
    assert!(runs.iter().any(|r| r.text == "JUNE 2027"));
    // Saturdays and Sundays that are shooting days carry the chip text.
    assert!(runs.iter().any(|r| r.text == "SAT"));
}

#[test]
fn the_legend_is_the_code_list_of_the_source_and_the_simple_style_prints_fewer() {
    let film = long_film();
    let (runs, _, _) = dood(&film, json!({}), "d.pdf");
    for meaning in [
        "Start work",
        "Work",
        "Work finish",
        "Start, work and finish on the same day",
        "Hold (paid, not working)",
        "Travel",
        "Rehearsal",
        "Fitting",
        "Drop",
        "Not working",
        "Blank: no code for that day",
        "Entered by hand (correction)",
        "Off day",
    ] {
        assert!(runs.iter().any(|r| r.text == meaning), "{meaning}");
    }
    film.env.ok(
        "schedule.update_settings",
        json!({ "scheduleId": film.schedule, "name": "Shooting Schedule", "strictValidation": false,
                "dayDurationMinutes": 600, "doodCodeStyle": "simple" }),
    );
    let (runs, _, _) = dood(&film, json!({}), "s.pdf");
    assert!(!runs.iter().any(|r| r.text == "Start work"));
    assert!(!runs.iter().any(|r| r.text.starts_with("Start, work")));
    assert!(runs.iter().any(|r| r.text == "Not working"));
    assert!(runs.iter().any(|r| r.text == "Hold (paid, not working)"));
}

/// Rows of the matrix read back from the page: the code of every day and the totals.
struct ReadRow {
    name: String,
    codes: Vec<String>,
    totals: Vec<String>,
}

fn read_rows(runs: &[Run], page: usize, names: &[String]) -> (Vec<ReadRow>, f32, f32, usize) {
    let tops: Vec<&Run> = day_tops(runs)
        .into_iter()
        .filter(|r| r.page == page)
        .collect();
    let xs: Vec<f32> = tops.iter().map(|r| centre(r)).collect();
    let d = xs[1] - xs[0];
    let x0 = xs[0] - d / 2.0;
    let x_totals = x0 + d * xs.len() as f32;
    let mut rows = Vec::new();
    for name in names {
        let Some(n) = runs.iter().find(|r| r.page == page && &r.text == name) else {
            continue;
        };
        let same: Vec<&Run> = runs
            .iter()
            .filter(|r| r.page == page && (r.y - n.y).abs() < 3.0 && r.x > x0)
            .collect();
        let mut codes = vec![String::new(); xs.len()];
        for r in same.iter().filter(|r| r.x < x_totals) {
            let j = ((centre(r) - x0) / d).floor() as usize;
            codes[j] = r.text.clone();
        }
        let mut totals: Vec<&Run> = same
            .iter()
            .filter(|r| r.x >= x_totals - 1.0)
            .copied()
            .collect();
        totals.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
        rows.push(ReadRow {
            name: name.clone(),
            codes,
            totals: totals.iter().map(|r| r.text.clone()).collect(),
        });
    }
    (rows, x0, d, xs.len())
}

fn cast_names(film: &Film) -> Vec<String> {
    let v = film.env.ok("schedule.dood", json!({}));
    v["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            r["actor"]
                .as_str()
                .filter(|a| !a.trim().is_empty())
                .unwrap_or_else(|| r["character"].as_str().unwrap())
                .to_string()
        })
        .collect()
}

#[test]
fn totals_counts_and_start_finish_agree_with_the_printed_cells() {
    let film = long_film();
    correct(&film, 1, 3, "T", "Travels from the coast");
    let (runs, pages, _) = dood(&film, json!({}), "d.pdf");
    assert!(pages >= 1);
    let names = cast_names(&film);
    let (rows, x0, d, n_days) = read_rows(&runs, 1, &names);
    assert_eq!(rows.len(), names.len(), "every cast member is a row");
    let work = |c: &str| matches!(c.trim_end_matches('*'), "SW" | "W" | "WF" | "SWF");
    for r in &rows {
        let n = |f: &dyn Fn(&str) -> bool| r.codes.iter().filter(|c| f(c)).count().to_string();
        assert_eq!(r.totals.len(), 6, "{}: {:?}", r.name, r.totals);
        assert_eq!(r.totals[0], n(&work), "{} work", r.name);
        assert_eq!(
            r.totals[1],
            n(&|c| c.trim_end_matches('*') == "H"),
            "{} hold",
            r.name
        );
        assert_eq!(
            r.totals[2],
            n(&|c| c.trim_end_matches('*') == "T"),
            "{} travel",
            r.name
        );
        assert_eq!(
            r.totals[3],
            n(&|c| matches!(c.trim_end_matches('*'), "R" | "F" | "D")),
            "{} other",
            r.name
        );
        // Start and finish are the first and last work day, printed "Day N".
        let tops: Vec<&Run> = day_tops(&runs)
            .into_iter()
            .filter(|t| t.page == 1)
            .collect();
        let day = |j: usize| tops[j].text.trim_start_matches('0').to_string();
        let first = r.codes.iter().position(|c| work(c));
        let last = r.codes.iter().rposition(|c| work(c));
        assert_eq!(
            r.totals[4],
            first.map(|j| format!("Day {}", day(j))).unwrap_or_default(),
            "{} start",
            r.name
        );
        assert_eq!(
            r.totals[5],
            last.map(|j| format!("Day {}", day(j))).unwrap_or_default(),
            "{} finish",
            r.name
        );
    }
    // Per-day counts: cast working, on hold, and travel, rehearsal, fitting and drop.
    let label_y = |t: &str| runs.iter().find(|r| r.page == 1 && r.text == t).unwrap().y;
    for (label, pick) in [
        ("CAST WORKING", &work as &dyn Fn(&str) -> bool),
        ("ON HOLD (H)", &|c: &str| c.trim_end_matches('*') == "H"),
        ("TRAVEL, REHEARSAL, FITTING, DROP", &|c: &str| {
            matches!(c.trim_end_matches('*'), "T" | "R" | "F" | "D")
        }),
    ] {
        let y = label_y(label);
        for j in 0..n_days {
            let printed: Vec<&Run> = runs
                .iter()
                .filter(|r| {
                    r.page == 1
                        && (r.y - y).abs() < 4.0
                        && ((centre(r) - x0) / d).floor() as i64 == j as i64
                        && r.x > x0
                })
                .collect();
            let expect = rows
                .iter()
                .filter(|r| pick(&r.codes[j]))
                .count()
                .to_string();
            assert_eq!(printed.len(), 1, "{label} day {j}");
            assert_eq!(printed[0].text, expect, "{label} day {j}");
        }
    }
    // The Work column sum is the sum of the row counts.
    let sum: usize = rows
        .iter()
        .map(|r| r.totals[0].parse::<usize>().unwrap())
        .sum();
    let y = label_y("CAST WORKING");
    let total = runs
        .iter()
        .find(|r| {
            r.page == 1 && (r.y - y).abs() < 4.0 && r.x >= x0 + d * n_days as f32 && r.size == 8.0
        })
        .unwrap();
    assert_eq!(total.text, sum.to_string());
}

#[test]
fn corrections_are_marked_in_the_cell_and_listed_with_the_code_of_the_schedule() {
    let film = long_film();
    let (runs, _, _) = dood(&film, json!({}), "none.pdf");
    assert!(!runs.iter().any(|r| r.text == "CORRECTION DETAILS"));
    assert!(
        runs.iter()
            .any(|r| r.text.contains("None; every cell comes from the schedule"))
    );
    correct(&film, 0, 1, "H", "Held for the causeway reshoot");
    correct(&film, 2, 0, "R", "Tape-baking rehearsal");
    let (runs, _, _) = dood(&film, json!({}), "two.pdf");
    assert_eq!(count(&runs, "CORRECTION DETAILS"), 1);
    assert!(runs.iter().any(|r| r.text == "H*"));
    assert!(runs.iter().any(|r| r.text == "R*"));
    assert!(
        runs.iter()
            .any(|r| r.text == "Held for the causeway reshoot")
    );
    assert!(runs.iter().any(|r| r.text == "Tape-baking rehearsal"));
    assert!(
        runs.iter()
            .any(|r| r.text.contains("2 cells entered by hand"))
    );
}

#[test]
fn many_days_and_a_large_cast_paginate_with_repeated_headers_and_no_orphans() {
    let film = long_film();
    add_cast(&film, 40);
    // 45 more days push the matrix over several column groups.
    for i in 0..30 {
        film.env.ok(
            "schedule.create_day",
            json!({ "scheduleId": film.schedule, "date": format!("2027-{:02}-{:02}", 7 + i / 28, i % 28 + 1) }),
        );
    }
    let (runs, pages, _) = dood(&film, json!({}), "big.pdf");
    let groups = count(&runs, "CODE LEGEND");
    assert!(pages >= 4, "{pages} pages");
    // Every day column prints once; the cast repeats once per column group.
    let names = cast_names(&film);
    assert!(names.len() >= 40);
    // Every day column belongs to exactly one group; a group's header repeats on each of its
    // pages, so the day sets of different groups never share a page.
    let tops = day_tops(&runs);
    let shoot: std::collections::BTreeSet<&str> = tops
        .iter()
        .filter(|r| r.text != "OFF")
        .map(|r| r.text.as_str())
        .collect();
    assert_eq!(shoot.len(), 43, "every shoot day prints");
    let pages_of = |t: &str| -> std::collections::BTreeSet<usize> {
        tops.iter()
            .filter(|r| r.text == t)
            .map(|r| r.page)
            .collect()
    };
    let (first, last) = (pages_of("01"), pages_of("43"));
    assert!(
        first.is_disjoint(&last),
        "the first and the last day are in different groups"
    );
    for t in &shoot {
        let p = pages_of(t);
        let group_starts: Vec<usize> = (1..=pages)
            .filter(|n| {
                !runs
                    .iter()
                    .any(|r| r.page == *n && r.text == " · continued")
            })
            .collect();
        assert!(
            p.iter().any(|n| group_starts.contains(n)),
            "day {t} has its group's first page"
        );
    }
    let first = &names[0];
    let per_group = count(&runs, first);
    assert!(
        per_group >= 2,
        "the first cast member repeats in every group: {per_group}"
    );
    for n in &names {
        assert_eq!(
            count(&runs, n),
            per_group,
            "{n} appears once in every group"
        );
    }
    assert!(
        groups >= per_group,
        "the legend repeats on every page of a group"
    );
    // The header of a group repeats on each of its pages, and no page of the matrix is left with
    // only the header.
    for p in 1..=pages {
        let rows_here = runs
            .iter()
            .filter(|r| r.page == p && names.contains(&r.text))
            .count();
        let has_head = runs.iter().any(|r| r.page == p && r.text == "CODE LEGEND");
        if has_head {
            assert!(rows_here >= 2, "page {p} holds {rows_here} cast rows");
        }
    }
    // The counts never stand alone: the page that prints "CAST WORKING" also prints two cast rows.
    for r in runs.iter().filter(|r| r.text == "CAST WORKING") {
        let here = runs
            .iter()
            .filter(|x| x.page == r.page && names.contains(&x.text) && x.y > r.y)
            .count();
        assert!(
            here >= 2,
            "the counts of page {} stand with {here} rows",
            r.page
        );
    }
}

#[test]
fn the_same_data_gives_the_same_bytes() {
    let film = long_film();
    correct(&film, 0, 1, "H", "x");
    let (_, _, a) = dood(&film, json!({}), "a.pdf");
    let (_, _, b) = dood(&film, json!({}), "b.pdf");
    assert_eq!(std::fs::read(a).unwrap(), std::fs::read(b).unwrap());
}

#[test]
#[ignore = "writes the sample PDFs to release-out/pdf-samples"]
fn write_dood_design_samples() {
    let film = long_film();
    correct(
        &film,
        0,
        1,
        "H",
        "Held for the causeway reshoot, paid hold day",
    );
    correct(&film, 3, 4, "T", "Travels from the coast");
    let (_, _, p) = dood(&film, json!({}), "a.pdf");
    keep_sample(&p, "c4-dood-long.pdf");
    add_cast(&film, 40);
    for i in 0..30 {
        film.env.ok(
            "schedule.create_day",
            json!({ "scheduleId": film.schedule, "date": format!("2027-{:02}-{:02}", 7 + i / 28, i % 28 + 1) }),
        );
    }
    let (_, _, p) = dood(&film, json!({}), "b.pdf");
    keep_sample(&p, "c4-dood-stress.pdf");
}
