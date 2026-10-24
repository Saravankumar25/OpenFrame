//! Layout rules of the Call Sheet PDF (PDF design package v2, spec sections H and J). Every test reads the text runs with their page and position, so a rule such as "the
//! running header is on every page" or "no heading ends a page" is checked on the real output.
//! Visual quality is judged on the rendered pages; `write_*` below write the stress samples to
//! `release-out/pdf-samples` for that (the Daily View twin is `daily_view_design.rs`):
//!
//! ```text
//! node scripts/cargo-queued.mjs test -p openframe-application --test production \
//!     call_sheet_design::write -- --ignored --nocapture
//! ```

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::pdf_samples::{Film, build_film, id_of, still};

// ------------------------------------------------------------------ reading a PDF

#[derive(Debug, Clone)]
pub(crate) struct Run {
    pub page: usize,
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub text: String,
}

/// Every text run of the PDF with its page (from 1), baseline position and size, in drawing
/// order, and the page count.
pub(crate) fn read(path: &str) -> (Vec<Run>, usize) {
    let doc = lopdf::Document::load(path).expect("PDF parses");
    let pages = doc.get_pages();
    let mut out = Vec::new();
    for (n, (_, page)) in pages.iter().enumerate() {
        let content = lopdf::content::Content::decode(&doc.get_page_content(*page)).unwrap();
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
                "Tm" => (x, y) = (num(4), num(5)),
                "Td" => {
                    x += num(0);
                    y += num(1);
                }
                "Tj" => {
                    if let Some(lopdf::Object::String(b, _)) = op.operands.first() {
                        out.push(Run {
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
    (out, pages.len())
}

const PAGE_H: f32 = 792.0;
/// Above this the page's top margin holds the running header.
pub(crate) const HEADER_Y: f32 = PAGE_H - 40.0;
/// Below this the page's bottom margin holds the footer.
pub(crate) const FOOTER_Y: f32 = 66.0;

fn body(runs: &[Run], page: usize) -> Vec<&Run> {
    runs.iter()
        .filter(|r| r.page == page && r.y > FOOTER_Y && r.y < HEADER_Y)
        .collect()
}

pub(crate) fn texts(runs: &[Run]) -> Vec<&str> {
    runs.iter().map(|r| r.text.as_str()).collect()
}

fn first_page_of(runs: &[Run], text: &str) -> usize {
    runs.iter()
        .find(|r| r.text == text)
        .unwrap_or_else(|| panic!("{text} is not printed"))
        .page
}

// ------------------------------------------------------------------ exporting

pub(crate) fn export(film: &Film, op: &str, mut args: Value, file: &str) -> String {
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

pub(crate) fn keep_sample(path: &str, name: &str) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repository root")
        .to_path_buf();
    let dir: PathBuf = root.join("release-out").join("pdf-samples");
    std::fs::create_dir_all(&dir).expect("sample folder");
    std::fs::copy(path, dir.join(name)).expect("sample copied");
    let (_, pages) = read(path);
    println!("{name}: {pages} pages");
}

// ------------------------------------------------------------------ call sheet fixtures

const CALL_SHEET_HEADINGS: [&str; 15] = [
    "PRODUCTION",
    "LOCATIONS",
    "SCENE SCHEDULE",
    "PEOPLE ON THE DAY",
    "TALENT / CAST",
    "EXTRAS",
    "CREW",
    "DEPARTMENT NOTES",
    "DAY-OF LOGISTICS",
    "PRACTICAL / SAFETY INFORMATION",
    "WEATHER",
    "ADVANCE SCHEDULE",
    "WALKIE CHANNELS AND CONTACTS",
    "SPECIAL NOTES",
    "ATTACHMENTS & REFERENCE IMAGES",
];

/// Headings that must never end a page: the section names and the two group titles.
const CALL_SHEET_NAMED: [&str; 17] = [
    "PRODUCTION",
    "LOCATIONS",
    "SCENE SCHEDULE",
    "PEOPLE ON THE DAY",
    "TALENT / CAST",
    "EXTRAS",
    "CREW",
    "DEPARTMENT NOTES",
    "DAY-OF LOGISTICS",
    "PRACTICAL / SAFETY INFORMATION",
    "WEATHER",
    "ADVANCE SCHEDULE",
    "WALKIE CHANNELS AND CONTACTS",
    "SPECIAL NOTES",
    "ATTACHMENTS & REFERENCE IMAGES",
    "Cast & Crew",
    "Production notes",
];

fn all_sections(env: &Film, cs: &str) {
    for section in [
        "weather",
        "extras",
        "departments",
        "advance",
        "crew",
        "communication",
        "specialNotes",
        "referenceImages",
        "attachments",
    ] {
        env.env.ok(
            "callsheets.add_section",
            json!({ "id": cs, "section": section }),
        );
    }
}

/// A populated call sheet for the first shooting day with every optional section, three
/// reference pictures and one attachment.
fn full_sheet(film: &Film) -> String {
    let env = &film.env;
    let cs = id_of(&env.ok("callsheets.create", json!({ "dayId": film.days[0] })));
    all_sections(film, &cs);
    for (i, theme) in [0usize, 2, 4].iter().enumerate() {
        let p = env.write_file(&format!("ref{i}.png"), &still(*theme, 480, 320));
        env.ok(
            "callsheets.attach",
            json!({ "id": cs, "section": "referenceImages", "path": p }),
        );
    }
    let permit = env.write_file("Location permit.pdf", b"%PDF-1.4 permit");
    env.ok(
        "callsheets.attach",
        json!({ "id": cs, "section": "attachments", "path": permit }),
    );
    let mut doc = env.ok("callsheets.get", json!({ "id": cs }))["document"].clone();
    // One location, as most days have: the schedule starts on page 1.
    let first = doc["locations"][0].clone();
    doc["locations"] = json!([first]);
    doc["title"] = json!("Monsoon Letters");
    doc["dayLabel"] = json!("SHOOT DAY 1");
    doc["dayOf"] = json!("Day 1 of 14");
    doc["header"] = json!({
        "company": "Northlight Pictures",
        "address": "14 Example Road, Pune 411001",
        "producer": "Joseph Paul",
        "director": "Asha Rao",
        "firstAd": "Rohan Iyer",
        "contact": "Production office, 98100 11008"
    });
    doc["times"]["generalCall"] = json!("06:30");
    doc["times"]["lunch"] = json!("13:00");
    doc["times"]["estimatedWrap"] = json!("18:30");
    for (i, c) in doc["cast"].as_array_mut().unwrap().iter_mut().enumerate() {
        c["actor"] = json!(format!("Actor Number {}", i + 1));
        c["callTime"] = json!("06:30");
        c["status"] = json!("SW");
        c["makeup"] = json!("05:30");
        c["pickup"] = json!("Unit hotel, 60 minutes before call");
        c["contact"] = json!(format!("98200 3300{i}"));
    }
    for l in doc["locations"].as_array_mut().unwrap() {
        l["hospital"] = json!("Government District Hospital, Gudur (14 minutes)");
        l["emergencyContact"] = json!("Rohan Iyer, 1st AD, 98100 11002");
    }
    doc["practical"]["emergencyContact"] = json!("Joseph Paul, Production Manager, 98100 11008");
    doc["practical"]["safety"] = json!("Life jackets for everyone within 10 metres of the water.");
    doc["optional"]["weather"] = json!("Heavy rain from 09:00, clearing by 16:00.");
    doc["optional"]["sunrise"] = json!("05:48");
    doc["optional"]["sunset"] = json!("18:52");
    doc["optional"]["specialNotes"] = json!("Child actor on set: guardian present at all times.");
    doc["optional"]["communication"]["walkieChannels"] = json!("1 Production, 2 Camera, 3 Art");
    env.ok("callsheets.update", json!({ "id": cs, "document": doc }));
    cs
}

/// The long stress case of the design package: an 80 character working title, two locations
/// with very long text, 13 schedule rows (breaks and estimated starts), 14 cast with every
/// optional field, 24 crew, all optional sections and 6 pictures of which one is missing.
fn long_sheet(film: &Film) -> String {
    let env = &film.env;
    let cs = id_of(&env.ok("callsheets.create", json!({ "dayId": film.days[0] })));
    all_sections(film, &cs);
    for i in 0..5usize {
        let p = env.write_file(&format!("long{i}.png"), &still(i, 480, 320));
        env.ok(
            "callsheets.attach",
            json!({ "id": cs, "section": "referenceImages", "path": p }),
        );
    }
    let mut doc = env.ok("callsheets.get", json!({ "id": cs }))["document"].clone();
    doc["title"] = json!(
        "The Long Night of the Lighthouse Keeper's Daughter (Working Title: A Second Season of Tides)"
    );
    doc["dayLabel"] = json!("SHOOT DAY 7");
    doc["dayOf"] = json!("Day 7 of 21");
    doc["date"] = json!("2026-09-30");
    doc["crewCall"] = json!("05:30");
    doc["times"] = json!({
        "generalCall": "05:30", "shootingCall": "07:15",
        "breakfast": "06:00, catering tent by the harbour wall", "lunch": "13:30",
        "estimatedWrap": "20:45 (+1)"
    });
    doc["header"] = json!({
        "company": "Tidal Wave Moving Pictures and Associated Coastal Productions Pvt. Ltd.",
        "address": "Unit 4B, Harbour Road Industrial Estate, Near the Old Customs Warehouse, Mattancherry, Kochi, Kerala 682002, India",
        "producer": "Anjali Menon-Varghese",
        "director": "Dr. Sreekumar Parameswaran Nair",
        "firstAd": "",
        "contact": "Rekha Pillai, Line Producer and Unit Production Manager, +91 98470 11122 (office hours) / +91 98470 11133 (night)"
    });
    let long_text = "Single track road, 4 km of unmetered surface after the village. Vehicles over 3.5 tonnes cannot proceed beyond the Co-operative building and must transfer equipment to the 12-seat shuttles which run every 20 minutes from 05:00 until wrap. Generator trucks have a special permit obtained from the Port Trust.";
    doc["locations"] = json!([
        {
            "key": "l1",
            "name": "Pozhikkara Lighthouse and Keeper's Cottage, Upper Gallery and Lamp Room Interiors",
            "address": "Lighthouse Road, Pozhikkara Headland, Near the Fishermen's Co-operative Society Building, Kollam District, Kerala 691302",
            "meetingPoint": "Unit base at the car park; follow the red OPENFRAME signs",
            "access": long_text,
            "parking": "Forty cars at the Co-operative ground; overflow at the school playground 1.2 km away.",
            "hospital": "District Hospital Kollam, +91 474 2744100, 22 km",
            "emergencyContact": "Port Trust Estate Office, Mr. Hamid Kutty, +91 484 266 6400",
            "notes": "Lighthouse interior access limited to 6 people at a time. Hard hats mandatory from the cottage onwards."
        },
        {
            "key": "l2",
            "name": "Beach Road Fish Market",
            "address": "",
            "meetingPoint": "",
            "access": "",
            "parking": "",
            "hospital": "",
            "emergencyContact": "",
            "notes": ""
        }
    ]);
    let kinds = [
        "",
        "",
        "Meal",
        "",
        "",
        "Company Move",
        "",
        "",
        "Travel",
        "",
        "Custom",
        "",
        "",
    ];
    let scenes: Vec<Value> = kinds
        .iter()
        .enumerate()
        .map(|(i, k)| {
            json!({
                "key": format!("sc{i}"),
                "number": if k.is_empty() { format!("{}", 100 + i) } else { String::new() },
                "heading": if k.is_empty() { format!("INT. LIGHTHOUSE LAMP ROOM - NIGHT {i}") } else { format!("{k} row {i}") },
                "description": if k.is_empty() { "The keeper's daughter climbs the spiral stair with the storm lamp." } else { "" },
                "pages": if k.is_empty() { "1 2/8" } else { "" },
                "kind": k,
                "start": if i < 3 { format!("0{}:30", 6 + i) } else { String::new() },
                "intExt": if k.is_empty() { "INT" } else { "" },
                "dayNight": if k.is_empty() { "N" } else { "" },
                "castIds": if k.is_empty() { "1, 3, 7" } else { "" },
                "location": "",
                "minutes": 45 + 15 * (i as i64 % 4)
            })
        })
        .collect();
    doc["scenes"] = json!(scenes);
    let cast: Vec<Value> = (1..=14)
        .map(|i| {
            json!({
                "key": format!("c{i}"),
                "castId": i.to_string(),
                "actor": format!("Cast Member Number {i} of the Long Night"),
                "character": format!("CHARACTER {i}"),
                "status": STATUSES[i % 4],
                "pickup": if i % 2 == 0 { "Self" } else { "Unit hotel lobby, 45 min before call. Driver Mr. Shaji 98470 00011." },
                "makeup": if i % 2 == 0 { "30 min before call" } else { "60 min before call" },
                "callTime": if i % 5 == 3 { String::new() } else { format!("0{}:{}", 4 + i % 5, if i % 2 == 0 { "45" } else { "00" }) },
                "contact": format!("+91 98470 5{:04}", i),
                "notes": if i == 3 { "Child performer: guardian on set at all times; works 3 hours maximum." } else { "" }
            })
        })
        .collect();
    doc["cast"] = json!(cast);
    let crew: Vec<Value> = (0..24)
        .map(|i| {
            json!({
                "key": format!("cr{i}"),
                "name": format!("Crew Person {i}"),
                "role": format!("Role {i}"),
                "department": DEPARTMENTS[i % 6],
                "callTime": if i % 3 == 0 { "05:30" } else { "" },
                "contact": format!("+91 98470 4{:04}", i)
            })
        })
        .collect();
    doc["optional"]["crew"] = json!(crew);
    let mut refs = doc["optional"]["referenceImages"]
        .as_array()
        .unwrap()
        .clone();
    refs.push(json!({ "assetId": "asset-that-is-gone", "name": "missing-shot.png", "mediaType": "image/png" }));
    doc["optional"]["referenceImages"] = json!(refs);
    doc["optional"]["weather"] =
        json!("Heavy rain from 09:00 with gusts to 40 km/h, clearing by 16:00.");
    doc["optional"]["sunrise"] = json!("05:48");
    doc["optional"]["sunset"] = json!("18:52");
    doc["optional"]["specialNotes"] =
        json!("No visitors on set without the approval of the producer.");
    doc["practical"]["emergencyContact"] = json!("Anjali Menon-Varghese, +91 98470 11111");
    doc["practical"]["safety"] = json!(long_text);
    env.ok("callsheets.update", json!({ "id": cs, "document": doc }));
    cs
}

const STATUSES: [&str; 4] = ["SW", "W", "WF", "H"];
const DEPARTMENTS: [&str; 6] = ["Camera", "Direction", "Sound", "Art", "Grip", "Makeup"];

/// A draft with nothing entered yet, on a new shooting day without scenes.
fn minimal_sheet(film: &Film) -> String {
    let day = id_of(&film.env.ok(
        "schedule.create_day",
        json!({ "scheduleId": film.schedule }),
    ));
    let cs = id_of(&film.env.ok("callsheets.create", json!({ "dayId": day })));
    // The sheet starts with the project's crew records; nothing is entered in this case.
    let mut doc = film.env.ok("callsheets.get", json!({ "id": cs }))["document"].clone();
    doc["optional"] = json!({});
    film.env
        .ok("callsheets.update", json!({ "id": cs, "document": doc }));
    cs
}

// ------------------------------------------------------------------ call sheet rules

/// Rules every call sheet PDF follows, whatever its content: the running header and the footer
/// on every page, nothing outside the margins or in the footer zone, nothing under 6.4 pt, and
/// no heading as the last thing on a page.
pub(crate) fn check_furniture(runs: &[Run], pages: usize, header: &str, headings: &[&str]) {
    for page in 1..=pages {
        let top: Vec<&str> = runs
            .iter()
            .filter(|r| r.page == page && r.y >= HEADER_Y)
            .map(|r| r.text.as_str())
            .collect();
        assert_eq!(top, vec![header], "running header on page {page}");
        let foot: Vec<&str> = runs
            .iter()
            .filter(|r| r.page == page && r.y < 40.0)
            .map(|r| r.text.as_str())
            .collect();
        assert!(
            foot.contains(&"Created using OpenFrame"),
            "brand line on page {page}: {foot:?}"
        );
        assert!(
            foot.contains(&format!("Page {page} of {pages}").as_str()),
            "page number on page {page}: {foot:?}"
        );
        assert_eq!(foot.len(), 2, "nothing else in the footer: {foot:?}");
    }
    for r in runs {
        assert!(r.size >= 6.4 - 0.01, "{} is {} pt", r.text, r.size);
        assert!(
            r.x >= 45.0 && r.x < 568.0,
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
    // A section or table heading is never the last thing on its page.
    for page in 1..=pages {
        let b = body(runs, page);
        for h in b
            .iter()
            .filter(|r| r.size >= 9.9 && headings.contains(&r.text.as_str()))
        {
            assert!(
                b.iter().any(|r| r.y < h.y - 3.0),
                "{} ends page {page}",
                h.text
            );
        }
    }
}

fn heading_order(runs: &[Run]) -> Vec<&'static str> {
    let mut seen: Vec<(usize, &'static str)> = Vec::new();
    for h in CALL_SHEET_HEADINGS {
        if let Some(i) = runs
            .iter()
            .position(|r| r.text == h && r.size >= 7.0 && r.y < HEADER_Y)
        {
            seen.push((i, h));
        }
    }
    seen.sort();
    seen.into_iter().map(|(_, h)| h).collect()
}

#[test]
fn full_call_sheet_follows_the_v2_order_groups_and_page_furniture() {
    let film = build_film();
    let cs = full_sheet(&film);
    film.env.ok(
        "callsheets.finalize",
        json!({ "id": cs, "acknowledgeStale": true }),
    );
    let path = export(
        &film,
        "callsheets.export_pdf",
        json!({ "id": cs }),
        "full.pdf",
    );
    let (runs, pages) = read(&path);
    let header = "MONSOON LETTERS \u{b7} SHOOT DAY 01 \u{b7} REVISION 1";
    check_furniture(&runs, pages, header, &CALL_SHEET_NAMED);
    // Title, production details and the day block are on page 1 only.
    for w in ["PRODUCTION", "SHOOT DAY"] {
        assert!(
            runs.iter()
                .filter(|r| r.text == w && r.size >= 7.0 && r.y < HEADER_Y)
                .all(|r| r.page == 1),
            "{w} appears beyond page 1"
        );
    }
    assert!(
        texts(&runs)
            .iter()
            .any(|t| t.starts_with("FINAL \u{b7}") || t.starts_with("Final \u{b7}")),
        "final badge"
    );
    // Section order of the design (E, v2), with no separate "Practical notes" heading.
    assert_eq!(
        heading_order(&runs),
        vec![
            "PRODUCTION",
            "LOCATIONS",
            "SCENE SCHEDULE",
            "PEOPLE ON THE DAY",
            "TALENT / CAST",
            "EXTRAS",
            "CREW",
            "DEPARTMENT NOTES",
            "DAY-OF LOGISTICS",
            "PRACTICAL / SAFETY INFORMATION",
            "WEATHER",
            "ADVANCE SCHEDULE",
            "WALKIE CHANNELS AND CONTACTS",
            "SPECIAL NOTES",
            "ATTACHMENTS & REFERENCE IMAGES",
        ]
        .into_iter()
        .filter(|h| runs.iter().any(|r| r.text == *h))
        .collect::<Vec<_>>()
    );
    for must in [
        "PEOPLE ON THE DAY",
        "TALENT / CAST",
        "DAY-OF LOGISTICS",
        "PRACTICAL / SAFETY INFORMATION",
        "WEATHER",
        "SPECIAL NOTES",
        "ATTACHMENTS & REFERENCE IMAGES",
    ] {
        assert!(runs.iter().any(|r| r.text == must), "{must} is missing");
    }
    assert!(!runs.iter().any(|r| r.text == "PRACTICAL NOTES"));
    assert!(runs.iter().any(|r| r.text == "Cast & Crew"));
    assert!(runs.iter().any(|r| r.text == "Production notes"));
    // Each group is kept together: it starts at the top of a page, or it fits the page it is on.
    // On a full sheet the logistics group does not share the page the people group ends on.
    let people = first_page_of(&runs, "PEOPLE ON THE DAY");
    let logistics = first_page_of(&runs, "DAY-OF LOGISTICS");
    for (eyebrow, last) in [
        ("PEOPLE ON THE DAY", "DEPARTMENT NOTES"),
        ("DAY-OF LOGISTICS", "SPECIAL NOTES"),
    ] {
        let start = runs.iter().find(|r| r.text == eyebrow).unwrap();
        let end = runs.iter().rev().find(|r| r.text == last).unwrap();
        assert!(
            start.y > 700.0 || start.page == end.page,
            "{eyebrow} starts mid page {} and ends on page {}",
            start.page,
            end.page
        );
    }
    assert!(logistics > people, "groups in order");
    // The attachments section is whole: its heading, names and pictures share a page.
    assert_eq!(
        first_page_of(&runs, "ATTACHMENTS & REFERENCE IMAGES"),
        first_page_of(&runs, "ref2.png")
    );
    // Contact folds under the actor's name: same left edge, no Contact column in the talent table.
    let actor = runs.iter().find(|r| r.text == "Actor Number 1").unwrap();
    let contact = runs.iter().find(|r| r.text == "98200 33000").unwrap();
    assert_eq!((actor.page, actor.x), (contact.page, contact.x));
    assert!(contact.y < actor.y);
    // The same document and settings give the same bytes.
    let again = export(
        &film,
        "callsheets.export_pdf",
        json!({ "id": cs }),
        "full2.pdf",
    );
    assert_eq!(
        std::fs::read(&path).unwrap(),
        std::fs::read(&again).unwrap()
    );
}

#[test]
fn a_new_revision_shows_in_the_running_header() {
    let film = build_film();
    let cs = full_sheet(&film);
    film.env.ok(
        "callsheets.finalize",
        json!({ "id": cs, "acknowledgeStale": true }),
    );
    let next = id_of(&film.env.ok("callsheets.new_revision", json!({ "id": cs })));
    let path = export(
        &film,
        "callsheets.export_pdf",
        json!({ "id": next }),
        "rev2.pdf",
    );
    let (runs, pages) = read(&path);
    assert!(
        runs.iter()
            .any(|r| r.y >= HEADER_Y && r.text.ends_with("REVISION 2")),
        "{:?}",
        texts(&runs)
    );
    assert!(runs.iter().any(|r| r.text.starts_with("DRAFT")), "{pages}");
}

#[test]
fn long_call_sheet_flows_without_clipping_and_repeats_its_table_titles() {
    let film = build_film();
    let cs = long_sheet(&film);
    let path = export(
        &film,
        "callsheets.export_pdf",
        json!({ "id": cs }),
        "long.pdf",
    );
    let (runs, pages) = read(&path);
    assert!(pages >= 5, "{pages} pages");
    let title = "THE LONG NIGHT OF THE LIGHTHOUSE KEEPER'S DAUGHTER (WORKING TITLE: A SECOND SEASON OF TIDES)";
    check_furniture(
        &runs,
        pages,
        &format!("{title} \u{b7} SHOOT DAY 07 \u{b7} REVISION 1"),
        &CALL_SHEET_NAMED,
    );
    // The long title wraps onto at most four lines at a smaller size, never clipped.
    let lines: Vec<&Run> = runs
        .iter()
        .filter(|r| r.page == 1 && r.size >= 22.0 && r.size <= 28.0 && r.y > 560.0 && r.x < 300.0)
        .collect();
    assert!(
        (2..=4).contains(&lines.len()),
        "{} title lines",
        lines.len()
    );
    let joined = lines
        .iter()
        .map(|r| r.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(joined, title);
    // Both locations are one card per row, and the second prints its needs-input states.
    assert!(runs.iter().any(|r| r.text == "BEACH ROAD FISH MARKET"));
    assert!(
        runs.iter().filter(|r| r.text == "NEEDS INPUT").count() >= 6,
        "needs-input chips"
    );
    // The 13 schedule rows: estimated starts carry the label, breaks keep their kind chip.
    assert!(runs.iter().any(|r| r.text == "ESTIMATED"));
    for kind in ["MEAL", "COMPANY MOVE", "TRAVEL", "CUSTOM"] {
        assert!(runs.iter().any(|r| r.text == kind), "{kind} chip");
    }
    // A table that spans pages repeats its title with "continued" and never splits a row: every
    // page that holds talent or crew rows also holds the table's title.
    for (title, probe) in [
        ("TALENT / CAST", "CHARACTER 1"),
        ("TALENT / CAST", "CHARACTER 14"),
        ("CREW", "Crew Person 0"),
        ("CREW", "Crew Person 23"),
    ] {
        let page = runs
            .iter()
            .find(|r| r.text == probe)
            .unwrap_or_else(|| panic!("{probe}"))
            .page;
        assert!(
            runs.iter().any(|r| r.page == page && r.text == title),
            "{title} missing on page {page} where {probe} is"
        );
    }
    assert!(
        runs.iter().any(|r| r.text.trim() == "\u{b7} continued"),
        "continuation marker"
    );
    // The oversize people group starts a fresh page (spec H).
    let people = runs.iter().find(|r| r.text == "PEOPLE ON THE DAY").unwrap();
    assert!(people.y > 700.0, "starts at the top, not at {}", people.y);
    // Images: the missing one prints its sentence, and the others are on the page.
    assert!(runs.iter().any(|r| r.text == "Image unavailable"));
    assert!(runs.iter().any(|r| r.text == "missing-shot.png"));
}

#[test]
fn minimal_draft_is_one_page_with_every_empty_state_and_needs_input() {
    let film = build_film();
    let cs = minimal_sheet(&film);
    let path = export(
        &film,
        "callsheets.export_pdf",
        json!({ "id": cs }),
        "min.pdf",
    );
    let (runs, pages) = read(&path);
    assert_eq!(pages, 1);
    let header = runs
        .iter()
        .find(|r| r.y >= HEADER_Y)
        .expect("running header")
        .text
        .clone();
    assert!(header.ends_with("\u{b7} REVISION 1"), "{header}");
    assert!(header.contains("\u{b7} SHOOT DAY 0"), "{header}");
    check_furniture(&runs, pages, &header, &CALL_SHEET_NAMED);
    for must in [
        "No location for this day.",
        "No scenes on this day.",
        "No cast called.",
        "DRAFT \u{b7} NOT FINAL",
        "Not ready to issue",
        "CREW CALL \u{b7} REQUIRED",
        "PEOPLE ON THE DAY",
        "DAY-OF LOGISTICS",
        "PRODUCTION EMERGENCY CONTACT",
    ] {
        assert!(runs.iter().any(|r| r.text == must), "{must} missing");
    }
    assert!(
        runs.iter().any(|r| r.text.starts_with("Before issuing")),
        "review callout"
    );
    assert!(runs.iter().filter(|r| r.text == "NEEDS INPUT").count() >= 2);
    // Optional sections that nobody added leave no heading.
    for gone in ["EXTRAS", "CREW", "WEATHER", "SPECIAL NOTES", "PRODUCTION"] {
        assert!(!runs.iter().any(|r| r.text == gone), "{gone} is printed");
    }
}

#[test]
fn without_the_optional_sections_only_the_six_default_sections_print() {
    let film = build_film();
    let cs = full_sheet(&film);
    let path = export(
        &film,
        "callsheets.export_pdf",
        json!({ "id": cs, "includeOptional": false }),
        "six.pdf",
    );
    let (runs, pages) = read(&path);
    check_furniture(
        &runs,
        pages,
        "MONSOON LETTERS \u{b7} SHOOT DAY 01 \u{b7} REVISION 1",
        &CALL_SHEET_NAMED,
    );
    let printed = heading_order(&runs);
    assert_eq!(
        printed,
        vec![
            "PRODUCTION",
            "LOCATIONS",
            "SCENE SCHEDULE",
            "PEOPLE ON THE DAY",
            "TALENT / CAST",
            "DAY-OF LOGISTICS",
            "PRACTICAL / SAFETY INFORMATION",
        ]
    );
    for gone in ["Image unavailable", "Weather", "Heavy rain"] {
        assert!(!runs.iter().any(|r| r.text.contains(gone)), "{gone}");
    }
}

// ------------------------------------------------------------------ sample writers

#[test]
#[ignore = "writes the sample PDFs to release-out/pdf-samples"]
fn write_call_sheet_design_samples() {
    let film = build_film();
    let full = full_sheet(&film);
    film.env.ok(
        "callsheets.finalize",
        json!({ "id": full, "acknowledgeStale": true }),
    );
    let p = export(
        &film,
        "callsheets.export_pdf",
        json!({ "id": full }),
        "a.pdf",
    );
    keep_sample(&p, "10-call-sheet-v2-final-full.pdf");
    let film = build_film();
    let long = long_sheet(&film);
    let p = export(
        &film,
        "callsheets.export_pdf",
        json!({ "id": long }),
        "b.pdf",
    );
    keep_sample(&p, "11-call-sheet-v2-stress-long.pdf");
    let min = minimal_sheet(&film);
    let p = export(
        &film,
        "callsheets.export_pdf",
        json!({ "id": min }),
        "c.pdf",
    );
    keep_sample(&p, "12-call-sheet-v2-stress-minimal.pdf");
    let film = build_film();
    let sheet = full_sheet(&film);
    let p = export(
        &film,
        "callsheets.export_pdf",
        json!({ "id": sheet, "includeOptional": false }),
        "d.pdf",
    );
    keep_sample(&p, "13-call-sheet-v2-six-default-sections.pdf");
}
