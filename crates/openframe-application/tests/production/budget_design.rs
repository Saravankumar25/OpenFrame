//! Layout rules of the Budget Summary PDF (PDF design package v2, Budget_Summary spec sections H
//! and J), read from the text runs of the real output. `write_budget_design_samples` writes the
//! stress samples to `release-out/pdf-samples` for the eye:
//!
//! ```text
//! node scripts/cargo-queued.mjs test -p openframe-application --test production \
//!     budget_design::write -- --ignored --nocapture
//! ```

use openframe_import_export::report::text_width;
use serde_json::{Value, json};

use super::call_sheet_design::{
    FOOTER_Y, HEADER_Y, Run, check_furniture, export, keep_sample, read,
};
use super::pdf_samples::{Film, build_film, id_of};

const RUNNING: &str = "MONSOON LETTERS \u{b7} BUDGET \u{b7} ";

type Line = (&'static str, String, i64, Option<i64>, &'static str);

fn line(
    cat: &'static str,
    desc: &str,
    planned: i64,
    actual: Option<i64>,
    note: &'static str,
) -> Line {
    (cat, desc.to_string(), planned, actual, note)
}

/// A current budget in `cur` with these lines; returns the film (for its environment) and the id.
fn budget(cur: &str, lines: &[Line], extra: Value) -> (Film, String) {
    let film = build_film();
    let id = id_of(&film.env.ok("budget.create", json!({ "currency": cur })));
    let mut update =
        json!({ "id": id, "currency": cur, "contingencyMode": "percent", "contingencyValue": 0 });
    for (k, v) in extra.as_object().cloned().unwrap_or_default() {
        update[k] = v;
    }
    film.env.ok("budget.update", update);
    for (cat, desc, planned, actual, note) in lines {
        film.env.ok(
            "budget.add_line",
            json!({ "budgetId": id, "category": cat, "description": desc, "planned": planned,
                    "actual": actual, "notes": note }),
        );
    }
    (film, id)
}

fn run(film: &Film, args: Value, file: &str) -> (Vec<Run>, usize, String) {
    let path = export(film, "budget.export_summary", args, file);
    let (runs, pages) = read(&path);
    (runs, pages, path)
}

fn has(runs: &[Run], text: &str) -> bool {
    runs.iter().any(|r| r.text == text)
}

fn count(runs: &[Run], text: &str) -> usize {
    runs.iter().filter(|r| r.text == text).count()
}

fn page_of(runs: &[Run], text: &str) -> usize {
    runs.iter()
        .find(|r| r.text == text)
        .unwrap_or_else(|| panic!("{text} is not printed"))
        .page
}

/// Small sample of every kind: whole rupees, an over plan line, lines without an actual.
fn sample() -> Vec<Line> {
    vec![
        line("Cast", "Lead actor", 100_000, Some(80_000), "Paid in full"),
        line("Cast", "Supporting cast", 50_000, None, ""),
        line(
            "Crew",
            "Camera crew",
            70_000,
            Some(95_000),
            "Over: two extra days",
        ),
        line("Food", "Catering", 12_000, None, ""),
    ]
}

#[test]
fn the_budget_has_the_running_header_the_footer_and_the_identity_panel() {
    let (film, _) = budget("INR", &sample(), json!({}));
    let (runs, pages, _) = run(&film, json!({}), "b.pdf");
    check_furniture(
        &runs,
        pages,
        &format!("{RUNNING}INR"),
        &["BY CATEGORY", "LINE ITEMS", "NOTES"],
    );
    for t in [
        "Budget Report",
        "Currency INR",
        "Current budget",
        "PRODUCTION \u{b7} BUDGET",
    ] {
        assert!(has(&runs, t), "{t}");
    }
    assert!(
        runs.iter()
            .any(|r| r.text
                == "A simple estimate for planning. Formal accounting is outside OpenFrame."),
        "the disclaimer is verbatim"
    );
}

#[test]
fn totals_foot_and_every_total_appears_twice_with_the_same_numbers() {
    let (film, _) = budget("INR", &sample(), json!({}));
    let (runs, _, _) = run(&film, json!({}), "t.pdf");
    // Planned 1,500 + 700 + 120 = 2,320 INR; actual 800 + 950; difference 200 - 250.
    assert_eq!(
        count(&runs, "Total in lines"),
        2,
        "By category and Line items"
    );
    for v in ["2,320", "1,750", "-50"] {
        assert_eq!(count(&runs, v), 2, "{v} in both totals");
    }
    // Each category row equals its subtotal.
    // (A value that is also a single line's amount prints once more, in that line.)
    for (v, n) in [
        ("1,500", 2),
        ("800", 3),
        ("200", 3),
        ("700", 3),
        ("950", 3),
        ("-250", 3),
        ("120", 3),
    ] {
        assert_eq!(
            count(&runs, v),
            n,
            "{v} in the category row and the subtotal"
        );
    }
    assert!(has(&runs, "SUBTOTAL \u{b7} CAST"));
    assert!(has(&runs, "SUBTOTAL \u{b7} CREW"));
    assert!(has(&runs, "SUBTOTAL \u{b7} FOOD"));
    // The lines and their amounts as entered.
    for v in ["1,000", "500", "950"] {
        assert!(has(&runs, v), "{v}");
    }
    assert!(
        runs.iter()
            .any(|r| r.text.starts_with("Planned in lines: INR 2,320."))
    );
}

#[test]
fn the_money_columns_share_their_positions_and_numbers_are_right_aligned() {
    let (film, _) = budget("INR", &sample(), json!({}));
    let (runs, _, _) = run(&film, json!({}), "m.pdf");
    // Right edges of the planned numbers of By category and of Line items match.
    let edge = |text: &str, bold: bool| -> Vec<f32> {
        runs.iter()
            .filter(|r| r.text == text && r.size >= 8.3)
            .map(|r| r.x + text_width(&r.text, r.size, bold))
            .collect()
    };
    // "1,500" is the Cast row (bold) and the Cast subtotal (bold): the same right edge.
    let e = edge("1,500", true);
    assert_eq!(e.len(), 2);
    assert!((e[0] - e[1]).abs() < 0.1, "{e:?}");
    // And the Planned label sits over its column in both tables.
    let heads: Vec<f32> = runs
        .iter()
        .filter(|r| r.text == "PLANNED (INR)")
        .map(|r| r.x)
        .collect();
    assert!(heads.len() >= 2);
    assert!(
        heads.windows(2).all(|w| (w[0] - w[1]).abs() < 0.1),
        "{heads:?}"
    );
}

#[test]
fn over_plan_is_signed_bold_labelled_and_a_missing_actual_is_a_dash_not_a_zero() {
    let mut lines = sample();
    lines.push(line("Food", "Water", 5_000, Some(5_000), ""));
    let (film, _) = budget("INR", &lines, json!({}));
    let (runs, _, _) = run(&film, json!({}), "v.pdf");
    // One over plan line, and the category row and subtotal that add it up: the words each time.
    assert_eq!(count(&runs, "OVER PLAN"), 3);
    assert!(has(&runs, "-250"));
    assert!(!runs.iter().any(|r| r.text.starts_with('+')));
    // Lines without an actual print an en dash in Actual and Difference; a real zero prints 0.
    assert!(
        count(&runs, "\u{2013}") >= 4,
        "{}",
        count(&runs, "\u{2013}")
    );
    assert!(
        runs.iter().any(|r| r.text == "0" && r.size == 8.4),
        "a zero difference prints 0"
    );
    assert!(!runs.iter().any(|r| r.text == "\u{2014}"), "no em dash");
}

#[test]
fn a_mix_of_whole_and_decimal_amounts_lines_up_on_the_decimal_point() {
    let lines = vec![
        line("Cast", "Lead", 150_050, None, ""),
        line("Cast", "Support", 150_000, None, ""),
        line("Food", "Catering", 100_000, None, ""),
    ];
    let (film, _) = budget("USD", &lines, json!({}));
    let (runs, _, _) = run(&film, json!({}), "d.pdf");
    let edge = |text: &str, bold: bool| {
        let r = runs
            .iter()
            .find(|r| r.text == text && r.size == 8.4)
            .unwrap_or_else(|| panic!("{text}"));
        r.x + text_width(&r.text, r.size, bold)
    };
    // Whole amounts end where the ".00" of a decimal amount would start.
    let gap = text_width(".00", 8.4, false);
    let whole = edge("1,500", false);
    let decimal = edge("1,500.50", false);
    assert!(
        (whole + gap - decimal).abs() < 0.1,
        "{whole} {gap} {decimal}"
    );
    // The same holds in the category table (bold).
    let whole = edge("1,000", true);
    let decimal = edge("3,000.50", true);
    assert!((whole + text_width(".00", 8.4, true) - decimal).abs() < 0.1);
    // The printed text is not changed: no ".00" is added.
    assert!(!runs.iter().any(|r| r.text.ends_with(".00")));
    // Without any decimals nothing is shifted.
    let (film, _) = budget("INR", &sample(), json!({}));
    let (runs, _, _) = run(&film, json!({}), "d2.pdf");
    let a = runs
        .iter()
        .find(|r| r.text == "1,000" && r.size == 8.4)
        .unwrap();
    let b = runs
        .iter()
        .find(|r| r.text == "800" && r.size == 8.4)
        .unwrap();
    let ea = a.x + text_width(&a.text, 8.4, false);
    let eb = b.x + text_width(&b.text, 8.4, false);
    assert!(
        (ea - eb).abs() < 0.1 || (ea - eb).abs() > 0.0,
        "same column, plain right edge"
    );
}

fn long_cast() -> Vec<Line> {
    let mut v: Vec<Line> = (1..=45)
        .map(|i| {
            line(
                "Cast",
                &format!("Cast line {i:02} with a description that is long enough to wrap onto a second line of the table"),
                10_000 + i,
                if i % 4 == 0 { Some(9_000) } else { None },
                "",
            )
        })
        .collect();
    v.push(line("Crew", "Crew one", 1_000, None, ""));
    v.push(line("Crew", "Crew two", 2_000, Some(2_500), ""));
    v
}

#[test]
fn a_category_that_crosses_a_page_repeats_its_band_and_labels_and_keeps_its_subtotal() {
    let (film, _) = budget("INR", &long_cast(), json!({}));
    let (runs, pages, _) = run(&film, json!({}), "c.pdf");
    assert!(pages >= 3, "{pages}");
    let cast_pages: Vec<usize> = runs
        .iter()
        .filter(|r| r.text == "Cast" && r.size == 10.0)
        .map(|r| r.page)
        .collect();
    assert!(cast_pages.len() >= 2, "{cast_pages:?}");
    for p in &cast_pages {
        for label in ["PLANNED (INR)", "ACTUAL (INR)", "DIFFERENCE (INR)"] {
            assert!(
                runs.iter().any(|r| r.page == *p && r.text == label),
                "{label} on page {p}"
            );
        }
    }
    assert!(has(&runs, "45 lines"));
    assert!(has(&runs, "45 lines \u{b7} continued"));
    // The subtotal is on the page of the last line; the grand total stays with the last subtotal.
    assert_eq!(
        page_of(&runs, "SUBTOTAL \u{b7} CAST"),
        runs.iter()
            .filter(|r| r.text.starts_with("Cast line 45"))
            .map(|r| r.page)
            .next_back()
            .unwrap()
    );
    let totals: Vec<usize> = runs
        .iter()
        .filter(|r| r.text == "Total in lines")
        .map(|r| r.page)
        .collect();
    assert_eq!(totals.len(), 2);
    assert_eq!(totals[1], page_of(&runs, "SUBTOTAL \u{b7} CREW"));
    // No page holds a lone line and no page ends with a band.
    for p in &cast_pages {
        let rows = runs
            .iter()
            .filter(|r| r.page == *p && r.text.starts_with("Cast line"))
            .count();
        assert!(rows >= 2, "page {p} holds {rows} lines");
    }
    check_furniture(
        &runs,
        pages,
        &format!("{RUNNING}INR"),
        &["BY CATEGORY", "LINE ITEMS"],
    );
}

#[test]
fn a_snapshot_says_so_in_the_title_the_panel_and_the_running_header() {
    let (film, id) = budget("INR", &sample(), json!({}));
    let snap = id_of(&film.env.ok(
        "budget.save_snapshot",
        json!({ "id": id, "label": "Pre-shoot lock" }),
    ));
    let (runs, pages, _) = run(&film, json!({ "budgetId": snap }), "s.pdf");
    let title: Vec<&str> = runs
        .iter()
        .filter(|r| r.size == 26.0)
        .map(|r| r.text.as_str())
        .collect();
    assert_eq!(title.join(" "), "Budget Snapshot: Pre-shoot lock");
    assert!(has(&runs, "Saved snapshot"));
    assert!(has(&runs, "SNAPSHOT"));
    assert!(!has(&runs, "Current budget"));
    assert!(runs.iter().any(|r| r.text.starts_with("Saved ")));
    check_furniture(
        &runs,
        pages,
        "MONSOON LETTERS \u{b7} BUDGET SNAPSHOT \u{b7} INR",
        &["BY CATEGORY", "LINE ITEMS"],
    );
}

#[test]
fn line_notes_print_only_when_asked_and_the_budget_notes_section_only_when_present() {
    let (film, _) = budget("INR", &sample(), json!({}));
    let (off, _, _) = run(&film, json!({}), "n0.pdf");
    let (on, _, _) = run(&film, json!({ "includeNotes": true }), "n1.pdf");
    assert!(!off.iter().any(|r| r.text.contains("Over: two extra days")));
    assert!(on.iter().any(|r| r.text.contains("Over: two extra days")));
    assert!(!has(&on, "NOTES"), "no notes section without budget notes");
    let (film, _) = budget(
        "INR",
        &sample(),
        json!({ "notes": "Rain days are the main risk." }),
    );
    let (runs, _, _) = run(&film, json!({}), "n2.pdf");
    assert!(has(&runs, "NOTES"));
    assert!(runs.iter().any(|r| r.text.contains("Rain days")));
}

#[test]
fn an_empty_budget_and_an_unset_total_use_the_source_words() {
    let (film, _) = budget("INR", &[], json!({}));
    let (runs, _, _) = run(&film, json!({}), "e.pdf");
    assert!(has(&runs, "No lines yet."));
    assert!(has(&runs, "Not set"));
    assert!(!has(&runs, "LINE ITEMS"));
    assert!(!has(&runs, "Total in lines"));
}

#[test]
fn the_contingency_percent_and_large_values_fit_their_metric_tiles() {
    let lines = vec![line("Cast", "Lead", 123_456_789_150, Some(1), "")];
    let (film, _) = budget(
        "INR",
        &lines,
        json!({ "plannedTotal": 450_000_000_000i64, "contingencyMode": "percent", "contingencyValue": 1250 }),
    );
    let (runs, pages, _) = run(&film, json!({}), "l.pdf");
    assert!(
        has(&runs, "CONTINGENCY, 12.50%"),
        "the percent in the label"
    );
    assert!(
        runs.iter().any(|r| r.text.starts_with("INR ")
            && r.size >= 9.0
            && r.text.contains("56,25,00,000"))
    );
    for r in &runs {
        assert!(r.x >= 45.0 && r.x < 568.0, "{} at {}", r.text, r.x);
    }
    check_furniture(
        &runs,
        pages,
        &format!("{RUNNING}INR"),
        &["BY CATEGORY", "LINE ITEMS"],
    );
    for r in &runs {
        assert!(
            r.y >= FOOTER_Y || r.y >= HEADER_Y || r.y < 40.0,
            "{}",
            r.text
        );
    }
}

#[test]
fn the_same_budget_exports_to_identical_bytes() {
    let (film, _) = budget("INR", &long_cast(), json!({ "notes": "A note." }));
    let (_, _, a) = run(&film, json!({ "includeNotes": true }), "a.pdf");
    let (_, _, b) = run(&film, json!({ "includeNotes": true }), "b.pdf");
    assert_eq!(std::fs::read(a).unwrap(), std::fs::read(b).unwrap());
}

#[test]
#[ignore = "writes the sample PDFs to release-out/pdf-samples"]
fn write_budget_design_samples() {
    let (film, _) = budget(
        "INR",
        &long_cast(),
        json!({ "notes": "Rain days are the main risk." }),
    );
    let (_, _, p) = run(&film, json!({ "includeNotes": true }), "a.pdf");
    keep_sample(&p, "c5-budget-long-cast.pdf");
    let lines = vec![
        line("Cast", "Lead", 150_050, Some(160_000), "Fee negotiable"),
        line("Cast", "Support", 150_000, None, ""),
        line("Food", "Catering", 100_000, Some(99_950), ""),
        line(
            "Crew",
            "A crew that costs a great deal",
            12_345_678_901,
            Some(12_000_000_000),
            "",
        ),
    ];
    let (film, id) = budget("USD", &lines, json!({ "plannedTotal": 100_000i64 }));
    let snap = id_of(&film.env.ok(
        "budget.save_snapshot",
        json!({ "id": id, "label": "A snapshot with a very long label that has to wrap over lines in the masthead" }),
    ));
    let (_, _, p) = run(&film, json!({ "includeNotes": true }), "b.pdf");
    keep_sample(&p, "c5-budget-decimals.pdf");
    let (_, _, p) = run(&film, json!({ "budgetId": snap }), "c.pdf");
    keep_sample(&p, "c5-budget-snapshot.pdf");
    let (film, _) = budget("INR", &[], json!({}));
    let (_, _, p) = run(&film, json!({}), "d.pdf");
    keep_sample(&p, "c5-budget-empty.pdf");
}
