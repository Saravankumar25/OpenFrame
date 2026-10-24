//! Behaviour tests for the redesigned production PDFs: layout rules that must hold for any data
//! (every day column printed once, order kept, full text kept, missing states shown,
//! counts that agree with what is listed). Visual quality is judged on the rendered pages (see
//! `pdf_samples.rs`); the byte-level layout goldens are in `pdf_regression.rs`.

use serde_json::{Value, json};

use super::pdf_samples::{Film, build_film, id_of, strip_id};

/// Every text run on the pages in drawing order: base-14 strings and ActualText spans.
fn runs(path: &str) -> Vec<String> {
    let doc = lopdf::Document::load(path).expect("PDF parses");
    let mut out = Vec::new();
    for (_, page) in doc.get_pages() {
        let content = lopdf::content::Content::decode(&doc.get_page_content(page)).unwrap();
        for op in content.operations {
            match (op.operator.as_str(), op.operands.as_slice()) {
                ("Tj", [lopdf::Object::String(b, _), ..]) => out.push(
                    b.iter()
                        .map(|x| openframe_import_export::winansi::decode_byte(*x))
                        .collect(),
                ),
                ("BDC", [_, lopdf::Object::Dictionary(d), ..]) => {
                    if let Ok(lopdf::Object::String(b, _)) = d.get(b"ActualText") {
                        out.push(match b.strip_prefix(&[0xFE, 0xFF]) {
                            Some(rest) => String::from_utf16_lossy(
                                &rest
                                    .chunks(2)
                                    .filter(|c| c.len() == 2)
                                    .map(|c| u16::from_be_bytes([c[0], c[1]]))
                                    .collect::<Vec<_>>(),
                            ),
                            None => b.iter().map(|x| *x as char).collect(),
                        });
                    }
                }
                _ => {}
            }
        }
    }
    out
}

fn pages(path: &str) -> usize {
    lopdf::Document::load(path).unwrap().get_pages().len()
}

fn export(film: &Film, op: &str, mut args: Value, file: &str) -> String {
    let path = film
        .env
        .dir
        .path()
        .join(file)
        .to_string_lossy()
        .into_owned();
    args["format"] = json!("pdf");
    args["path"] = json!(path);
    film.env.ok(op, args);
    path
}

#[test]
fn day_out_of_days_groups_hold_every_day_once() {
    // The v2 design replaced the fixed bands of seven day columns: the days are split into the
    // fewest column groups that keep one column width (see `dood_design.rs`). What the bands
    // guaranteed still holds: every day appears once, whatever the grouping.
    let film = build_film();
    let env = &film.env;
    // 9 more shooting days and a second off day: 13 shooting days and 2 off days in all.
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
    let path = export(&film, "schedule.export_dood", json!({}), "dood.pdf");
    let t = runs(&path);
    let count = |needle: &str| t.iter().filter(|r| r.as_str() == needle).count();
    // 15 day columns fit one group: one title, no group numbering.
    assert_eq!(count("CAST COMMITMENTS BY SHOOTING DAY"), 1);
    assert!(!t.iter().any(|r| r.contains("Columns group")));
    // Each day column prints once (checked by size and position in `dood_design.rs`); here the two
    // off days and the legend's off swatch are the three "OFF" texts.
    assert_eq!(count("OFF"), 3);
    // The totals sit inside the matrix, once; there is no second table repeating the names.
    assert_eq!(count("TOTALS"), 1);
    assert_eq!(count("CAST TOTALS ACROSS THE FULL SCHEDULE"), 0);
    // Identical data gives identical bytes.
    let again = export(&film, "schedule.export_dood", json!({}), "dood2.pdf");
    assert_eq!(
        std::fs::read(&path).unwrap(),
        std::fs::read(&again).unwrap()
    );
    assert!(pages(&path) >= 1);
}

#[test]
fn shooting_schedule_keeps_the_stored_order_and_the_whole_synopsis() {
    let film = build_film();
    let env = &film.env;
    let long =
        "The whole of this synopsis must reach the page, including its very last word endmark";
    env.core
        .project()
        .unwrap()
        .store
        .with_writer(|c| {
            c.execute(
                "UPDATE screenplay_scene SET synopsis = ?1 WHERE id = 's2'",
                [long],
            )
            .unwrap();
            Ok(())
        })
        .unwrap();
    let _ = strip_id(env, 2);
    let path = export(&film, "schedule.export_schedule", json!({}), "schedule.pdf");
    let t = runs(&path);
    assert!(
        t.iter().any(|r| r.contains("endmark")),
        "no synopsis is cut with an ellipsis"
    );
    // Day 1 was stored as scenes 2, 3, 4, 1: that order is the strip order on the page.
    let pos = |h: &str| {
        t.iter()
            .position(|r| r == h)
            .unwrap_or_else(|| panic!("{h}"))
    };
    let order = [
        pos("EXT. GUDUR BUS STAND - MORNING"),
        pos("INT. STATE BUS - DAY (MOVING)"),
        pos("EXT. FLOODED HIGHWAY - DAY"),
        pos("INT. SAVITRI'S KITCHEN - DAWN"),
    ];
    assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?}");
    assert!(
        !t.iter()
            .any(|r| r.contains('\u{2026}') && r.contains("endmark"))
    );
}

#[test]
fn breakdown_report_lists_every_counted_element() {
    let film = build_film();
    let path = export(
        &film,
        "breakdown.export_report",
        json!({ "content": "scenes" }),
        "breakdown.pdf",
    );
    let t = runs(&path);
    let value_before = |label: &str| -> usize {
        let i = t.iter().position(|r| r == label).expect(label);
        t[i - 1].parse().expect("a number")
    };
    let confirmed = value_before("CONFIRMED ELEMENTS");
    let suggested = value_before("SUGGESTED");
    let rows: Vec<(String, String)> = film
        .env
        .core
        .project()
        .unwrap()
        .store
        .read(|c| {
            let mut st = c.prepare(
                "SELECT e.confirmation_state, COALESCE(ci.name, e.display_name)
                 FROM breakdown_element e LEFT JOIN catalog_item ci ON ci.id = e.catalog_item_id
                 WHERE e.deleted_at IS NULL AND e.archived = 0 AND e.confirmation_state <> 'Rejected'",
            )?;
            Ok(st
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<Result<Vec<_>, _>>()?)
        })
        .unwrap();
    let in_db_suggested = rows.iter().filter(|r| r.0 == "Suggested").count();
    assert_eq!(suggested, in_db_suggested);
    assert_eq!(confirmed, rows.len() - in_db_suggested);
    // Every counted element is printed once per scene it belongs to.
    let printed: usize = rows
        .iter()
        .map(|(_, name)| t.iter().filter(|r| *r == name).count())
        .sum::<usize>();
    let distinct_names: std::collections::HashSet<&String> = rows.iter().map(|r| &r.1).collect();
    let expected: usize = distinct_names
        .iter()
        .map(|n| rows.iter().filter(|r| &&r.1 == n).count())
        .sum();
    assert!(printed >= expected, "printed {printed}, counted {expected}");
}

#[test]
fn call_sheet_shows_missing_values_as_a_state_and_never_invents_them() {
    let film = build_film();
    let env = &film.env;
    let cs = id_of(&env.ok("callsheets.create", json!({ "dayId": film.days[1] })));
    let path = export(
        &film,
        "callsheets.export_pdf",
        json!({ "id": cs }),
        "cs.pdf",
    );
    let t = runs(&path);
    // Hospital and emergency contact of every location, address and cast actor/call.
    assert!(
        t.iter().filter(|r| *r == "NEEDS INPUT").count() >= 8,
        "{t:?}"
    );
    assert!(t.iter().any(|r| r == "DRAFT \u{b7} NOT FINAL"));
    assert!(t.iter().any(|r| r.starts_with("Before issuing")));
    assert!(
        !t.iter().any(|r| r.contains("123")),
        "no sample value leaks in"
    );
}

/// The "Confirmed elements" figure and the number of printed "Unnamed element" entries.
fn confirmed_and_unnamed(film: &Film, file: &str) -> (usize, usize) {
    let path = export(
        film,
        "breakdown.export_report",
        json!({ "content": "scenes" }),
        file,
    );
    let t = runs(&path);
    let i = t
        .iter()
        .position(|r| r == "CONFIRMED ELEMENTS")
        .expect("metric");
    (
        t[i - 1].parse().expect("a number"),
        t.iter().filter(|r| *r == "Unnamed element").count(),
    )
}

#[test]
fn breakdown_report_prints_every_counted_element_even_without_a_name() {
    // Spec 05 reported "19 confirmed, 18 listed". The count and the listing come from one list,
    // so the only way a counted element could go missing from the page was a blank name, which
    // used to print as nothing. It now prints as "Unnamed element".
    let film = build_film();
    let (before, unnamed_before) = confirmed_and_unnamed(&film, "b1.pdf");
    assert_eq!(unnamed_before, 0);
    film.env
        .core
        .project()
        .unwrap()
        .store
        .with_writer(|c| {
            c.execute_batch(
                "INSERT INTO breakdown_element(id,source_id,scene_id,scene_lineage_id,category,display_name,confirmation_state,created_at,updated_at)
                 VALUES ('blank1','ps1','s2','L2','Props','','Confirmed',900,900),
                        ('blank2','ps1','s2','L2','Props','   ','Manual',901,901);",
            )
            .expect("fixture sql");
            Ok(())
        })
        .unwrap();
    let (after, unnamed) = confirmed_and_unnamed(&film, "b2.pdf");
    assert_eq!(after, before + 2, "both elements are counted");
    assert_eq!(unnamed, 2, "and both are printed");
}
