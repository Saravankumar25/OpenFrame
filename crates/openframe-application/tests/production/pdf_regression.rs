//! PDF regression for the six production documents (masterplan area "Call sheets, budget,
//! templates, samples and exports", box "PDF regression tests"): call sheet, shooting schedule,
//! Day Out of Days, breakdown, budget and sides.
//!
//! Each case seeds the same fixed project, exports the document through its real operation and
//! compares the PDF with a committed golden file in `tests/fixtures/pdf-regression/`. The golden
//! records, page by page, the paper size and every text run with its position and font size, so
//! a change to content, order, layout, wrapping, pagination or branding fails the test. Exporting
//! twice must give byte-identical files (reproducible exports). Dates that come from the clock
//! (the export date and the "Revised" date of data written by the test) read `<DATE>`, and their
//! line position is not compared because a centred line moves with the date's width.
//!
//! Regenerate deliberately after an intended layout change, then review the diff:
//!
//! ```text
//! OPENFRAME_UPDATE_GOLDEN=1 cargo test -p openframe-application --test production pdf_regression::
//! ```

use std::path::{Path, PathBuf};

use openframe_test_support::TestEnv;
use serde_json::{Value, json};

const UPDATE_ENV: &str = "OPENFRAME_UPDATE_GOLDEN";
const BRAND: &str = "Created with OpenFrame Studio";

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

/// A locked draft with four scenes, two cast members, two locations, props and extras as the
/// active Production Source; a schedule with one dated, timed shooting day (two scenes, a meal
/// and a travel row) and a second day for the advance schedule.
fn fixed_project() -> (TestEnv, String) {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    exec(
        &env,
        "INSERT INTO screenplay(id,title,format,created_at,updated_at) VALUES ('sp','Black Rain','Feature',1,1);
         INSERT INTO screenplay_draft(id,screenplay_id,name,status,created_at,updated_at) VALUES ('d1','sp','Shooting Draft 6','Locked',1,1);
         INSERT INTO screenplay_scene(id,draft_id,lineage_id,position,heading,synopsis,created_at,updated_at) VALUES
            ('s1','d1','L1',1,'INT. POLICE STATION - NIGHT','Meera hands the file',1,1),
            ('s2','d1','L2',2,'EXT. OLD RAILWAY STATION - DAY','Arjun walks the platform',1,1),
            ('s3','d1','L3',3,'INT. MORGUE - DAY','Time of death',1,1),
            ('s4','d1','L4',4,'EXT. OLD RAILWAY STATION - NIGHT','Watcher on platform',1,1);
         INSERT INTO screenplay_element(id,scene_id,position,element_type,text,created_at,updated_at) VALUES
            ('e1','s1',1,'action','Rain lashes the windows. MEERA slides a red folder across the desk.',1,1),
            ('e2','s1',2,'character','MEERA',1,1),
            ('e3','s1',3,'dialogue','You will want to see this before anyone else does.',1,1),
            ('e4','s2',1,'action','ARJUN walks the empty platform, photograph in hand.',1,1),
            ('e5','s3',1,'action','A sheet is pulled back.',1,1),
            ('e6','s4',1,'action','A figure watches from the far platform.',1,1);
         INSERT INTO story_character(id,name,position,created_at,updated_at) VALUES ('arjun','Arjun',1,1,1),('meera','Meera',2,1,1);
         INSERT INTO cast_member(id,person_name,character_id,created_at,updated_at) VALUES ('cm1','Karthik Menon','arjun',1,1),('cm2','Divya Nair','meera',1,1);
         INSERT INTO location(id,name,address,status,created_at,updated_at) VALUES
            ('loc_police','Police Station','District Rd, Gudur','Confirmed',1,1),
            ('loc_rail','Old Railway Station','Station Rd, Gudur','Confirmed',1,1);
         INSERT INTO catalog_item(id,category,name,character_id,location_id,created_at,updated_at) VALUES
            ('ci_arjun','Cast','Arjun','arjun',NULL,1,1),
            ('ci_meera','Cast','Meera','meera',NULL,1,1),
            ('ci_police','Location / Set','Police Station',NULL,'loc_police',1,1),
            ('ci_rail','Location / Set','Old Railway Station',NULL,'loc_rail',1,1),
            ('ci_folder','Props','Red Folder',NULL,NULL,1,1),
            ('ci_crowd','Extras / Background','Commuters',NULL,NULL,1,1);
         INSERT INTO production_source(id,draft_id,active,selected_at,created_at,updated_at) VALUES ('ps1','d1',1,1,1,1);
         INSERT INTO breakdown_element(id,source_id,scene_id,scene_lineage_id,category,catalog_item_id,display_name,confirmation_state,notes,created_at,updated_at) VALUES
            ('b1','ps1','s1','L1','Cast','ci_arjun','Arjun','Confirmed',NULL,1,1),
            ('b2','ps1','s1','L1','Cast','ci_meera','Meera','Confirmed',NULL,1,1),
            ('b3','ps1','s1','L1','Location / Set','ci_police','Police Station','Confirmed',NULL,1,1),
            ('b4','ps1','s1','L1','Props','ci_folder','Red Folder','Manual',NULL,1,1),
            ('b5','ps1','s2','L2','Cast','ci_arjun','Arjun','Confirmed',NULL,1,1),
            ('b6','ps1','s2','L2','Location / Set','ci_rail','Old Railway Station','Confirmed',NULL,1,1),
            ('b7','ps1','s2','L2','Extras / Background','ci_crowd','Commuters','Confirmed',NULL,1,1),
            ('b8','ps1','s3','L3','Cast','ci_meera','Meera','Confirmed',NULL,1,1),
            ('b9','ps1','s4','L4','Cast','ci_arjun','Arjun','Confirmed',NULL,1,1),
            ('b10','ps1','s4','L4','Location / Set','ci_rail','Old Railway Station','Confirmed',NULL,1,1);",
    );
    let sched = id(&env.ok("schedule.create", json!({})));
    let d1 = env
        .ok(
            "schedule.create_day",
            json!({ "scheduleId": sched, "date": "2027-06-14" }),
        )
        .as_str()
        .unwrap()
        .to_string();
    let d2 = env
        .ok(
            "schedule.create_day",
            json!({ "scheduleId": sched, "date": "2027-06-15" }),
        )
        .as_str()
        .unwrap()
        .to_string();
    for (n, day, minutes) in [(1, &d1, 90), (2, &d1, 120), (3, &d2, 60), (4, &d2, 45)] {
        let s = strip(&env, n);
        env.ok("schedule.move_strip", json!({ "stripId": s, "dayId": day }));
        env.ok(
            "schedule.set_strip_estimate",
            json!({ "stripId": s, "minutes": minutes }),
        );
    }
    env.ok(
        "schedule.add_marker",
        json!({ "dayId": d1, "markerType": "Meal", "durationMinutes": 45, "index": 1 }),
    );
    env.ok(
        "schedule.add_marker",
        json!({ "dayId": d1, "markerType": "Travel", "label": "Move to the railway station", "durationMinutes": 30, "index": 2 }),
    );
    env.ok(
        "schedule.set_day_times",
        json!({ "dayId": d1, "crewCall": 1080, "shootingStart": 1140, "targetWrap": 120 }),
    );
    (env, d1)
}

fn id(v: &Value) -> String {
    v["id"]
        .as_str()
        .or_else(|| v.as_str())
        .expect("id")
        .to_string()
}

fn strip(env: &TestEnv, number: i64) -> String {
    let v = env.ok("schedule.get", json!({}));
    let mut all: Vec<Value> = v["unscheduled"].as_array().unwrap().clone();
    for d in v["days"].as_array().unwrap() {
        for it in d["items"].as_array().unwrap() {
            if it["kind"] == "strip" {
                all.push(it["strip"].clone());
            }
        }
    }
    all.iter().find(|s| s["number"] == number).expect("strip")["id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn dest(env: &TestEnv, name: &str) -> String {
    let d = env.dir.path().join("pdf-regression");
    std::fs::create_dir_all(&d).unwrap();
    d.join(name).to_string_lossy().into_owned()
}

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pdf-regression")
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Replace clock dates ("8 Oct 2026") with `<DATE>`. Dates in the fixed data are in 2027 and
/// stay as they are, so the test still checks them.
fn mask_dates(text: &str) -> (String, bool) {
    let words: Vec<&str> = text.split(' ').collect();
    let mut out: Vec<String> = Vec::new();
    let mut masked = false;
    let mut i = 0;
    while i < words.len() {
        if i + 2 < words.len()
            && words[i].len() <= 2
            && words[i].chars().all(|c| c.is_ascii_digit())
            && MONTHS.contains(&words[i + 1])
            && words[i + 2]
                .trim_end_matches(|c: char| !c.is_ascii_digit())
                .len()
                == 4
            && !words[i + 2].starts_with("2027")
        {
            let tail: String = words[i + 2].chars().skip(4).collect();
            out.push(format!("<DATE>{tail}"));
            masked = true;
            i += 3;
            continue;
        }
        out.push(words[i].to_string());
        i += 1;
    }
    (out.join(" "), masked)
}

/// Every page's paper size and text runs ("x,y size text") in drawing order.
fn layout_of(path: &str) -> String {
    let doc = lopdf::Document::load(path).expect("PDF parses");
    let mut out = String::new();
    for (n, (_, page)) in doc.get_pages().into_iter().enumerate() {
        let dict = doc.get_dictionary(page).unwrap();
        let mb = dict.get(b"MediaBox").unwrap().as_array().unwrap();
        let w = mb[2].as_float().unwrap();
        let h = mb[3].as_float().unwrap();
        out.push_str(&format!("== page {} {w:.0}x{h:.0}\n", n + 1));
        let content = lopdf::content::Content::decode(&doc.get_page_content(page)).unwrap();
        let (mut x, mut y, mut size) = (0.0f32, 0.0f32, 0.0f32);
        for op in content.operations {
            let num = |i: usize| {
                op.operands
                    .get(i)
                    .and_then(|o| o.as_float().ok())
                    .unwrap_or(0.0)
            };
            match op.operator.as_str() {
                "Tf" => size = num(1),
                "Tm" => {
                    x = num(4);
                    y = num(5);
                }
                "Td" => {
                    x += num(0);
                    y += num(1);
                }
                "Tj" => {
                    if let Some(lopdf::Object::String(b, _)) = op.operands.first() {
                        let text: String = b
                            .iter()
                            .map(|c| openframe_import_export::winansi::decode_byte(*c))
                            .collect();
                        let (text, masked) = mask_dates(&text);
                        let at = if masked {
                            format!("*,{y:.0}")
                        } else {
                            format!("{x:.0},{y:.0}")
                        };
                        out.push_str(&format!("{at} {size:.1} {text}\n"));
                    }
                }
                _ => {}
            }
        }
    }
    out
}

/// Export with `op` twice, require identical bytes and the brand line, and compare the layout
/// with the golden file `name.txt`.
fn check(env: &TestEnv, name: &str, op: &str, args: Value) {
    let first = dest(env, &format!("{name}-1.pdf"));
    let second = dest(env, &format!("{name}-2.pdf"));
    for p in [&first, &second] {
        let mut a = args.clone();
        a["format"] = json!("pdf");
        a["path"] = json!(p);
        env.ok(op, a);
    }
    let bytes = std::fs::read(&first).unwrap();
    assert_eq!(
        bytes,
        std::fs::read(&second).unwrap(),
        "{name}: the same data must give a byte-identical PDF"
    );
    let actual = layout_of(&first);
    assert!(
        actual.contains(BRAND),
        "{name}: production PDFs carry \"{BRAND}\"\n{actual}"
    );
    let golden = golden_dir().join(format!("{name}.txt"));
    if std::env::var(UPDATE_ENV).as_deref() == Ok("1") {
        std::fs::create_dir_all(golden_dir()).unwrap();
        std::fs::write(&golden, &actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&golden).unwrap_or_else(|_| {
        panic!(
            "missing golden {}; run with {UPDATE_ENV}=1 and review it",
            golden.display()
        )
    });
    let expected = expected.replace("\r\n", "\n");
    if expected != actual {
        let first_diff = expected
            .lines()
            .zip(actual.lines())
            .position(|(e, a)| e != a)
            .unwrap_or_else(|| expected.lines().count().min(actual.lines().count()));
        let show = |s: &str| {
            s.lines()
                .skip(first_diff.saturating_sub(2))
                .take(6)
                .collect::<Vec<_>>()
                .join("\n")
        };
        panic!(
            "{name}: the PDF layout changed at line {}.\n--- expected\n{}\n--- actual\n{}\n\
             If the change is intended, run with {UPDATE_ENV}=1 and review the golden diff.",
            first_diff + 1,
            show(&expected),
            show(&actual)
        );
    }
}

#[test]
fn call_sheet_pdf_matches_its_golden() {
    let (env, day) = fixed_project();
    let cs = env
        .ok("callsheets.create", json!({ "dayId": day }))
        .as_str()
        .unwrap()
        .to_string();
    for section in ["weather", "extras", "advance", "communication"] {
        env.ok(
            "callsheets.add_section",
            json!({ "id": cs, "section": section }),
        );
    }
    let mut doc = env.ok("callsheets.get", json!({ "id": cs }))["document"].clone();
    doc["header"]["company"] = json!("Northlight Pictures");
    doc["header"]["director"] = json!("Asha Rao");
    doc["times"]["breakfast"] = json!("17:30");
    doc["cast"][0]["callTime"] = json!("17:45");
    doc["locations"][0]["hospital"] = json!("District Hospital, Gudur");
    doc["practical"]["emergencyContact"] = json!("Rohan, 1st AD, 98000 20002");
    doc["optional"]["weather"] = json!("Rain after 21:00");
    doc["optional"]["sunrise"] = json!("05:47");
    doc["optional"]["sunset"] = json!("18:51");
    doc["optional"]["communication"]["walkieChannels"] = json!("1 Production, 2 Camera");
    env.ok("callsheets.update", json!({ "id": cs, "document": doc }));
    check(
        &env,
        "call-sheet",
        "callsheets.export_pdf",
        json!({ "id": cs }),
    );
}

#[test]
fn schedule_pdf_matches_its_golden() {
    let (env, _) = fixed_project();
    check(&env, "schedule", "schedule.export_schedule", json!({}));
}

#[test]
fn day_out_of_days_pdf_matches_its_golden() {
    let (env, _) = fixed_project();
    check(&env, "day-out-of-days", "schedule.export_dood", json!({}));
}

#[test]
fn breakdown_pdf_matches_its_golden() {
    let (env, _) = fixed_project();
    check(&env, "breakdown", "breakdown.export_report", json!({}));
}

#[test]
fn budget_pdf_matches_its_golden() {
    let (env, _) = fixed_project();
    let b = env
        .ok("budget.create", json!({ "currency": "INR" }))
        .as_str()
        .unwrap()
        .to_string();
    for (cat, desc, planned, actual) in [
        ("Cast", "Lead actor", 5_000_000, Some(5_500_000)),
        ("Locations", "Railway permit", 800_000, Some(650_000)),
        ("Food", "Catering", 1_200_000, None),
    ] {
        env.ok(
            "budget.add_line",
            json!({ "budgetId": b, "category": cat, "description": desc, "planned": planned, "actual": actual }),
        );
    }
    env.ok(
        "budget.update",
        json!({ "id": b, "currency": "INR", "plannedTotal": 120_000_000, "contingencyMode": "percent", "contingencyValue": 1000 }),
    );
    check(&env, "budget", "budget.export_summary", json!({}));
}

#[test]
fn sides_pdf_matches_its_golden() {
    let (env, day) = fixed_project();
    check(
        &env,
        "sides",
        "sides.export_pdf",
        json!({ "dayId": day, "includeCover": true, "showDraftName": true }),
    );
}

#[test]
fn date_masking_only_touches_clock_dates() {
    assert_eq!(
        mask_dates("Black Rain · Exported 8 Oct 2026 · Source"),
        ("Black Rain · Exported <DATE> · Source".to_string(), true)
    );
    assert_eq!(
        mask_dates("Mon 14 Jun 2027"),
        ("Mon 14 Jun 2027".to_string(), false)
    );
}
