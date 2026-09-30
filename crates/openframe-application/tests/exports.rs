//! Document exports (FSD §20.7–20.8, §31.5, §32.7, §33.6, §38.9, §58, §60,
//! §109.6, §110–111, §122–123; Import/Export §3, §6) end-to-end through the
//! operation registry: every export writes a valid file (PDFs parse with
//! lopdf and contain the expected text; CSV/XLSX are readable), exports are
//! snapshots, private notes never appear, permissions follow the role matrix,
//! unprintable PDF text is a human error, and failures leave no partial file.
//!
//! Screenplay/production/story hub rows are owned by other modules; tests seed
//! them directly as fixtures (like the schedule tests).

use std::io::Read as _;
use std::path::{Path, PathBuf};

use base64::Engine as _;
use openframe_domain::Role;
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

// ------------------------------------------------------------------ fixtures

/// 1×1 transparent PNG.
const PNG_B64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

fn png_bytes() -> Vec<u8> {
    base64::engine::general_purpose::STANDARD
        .decode(PNG_B64)
        .unwrap()
}

fn exec(env: &TestEnv, sql: &str) {
    let s = env.core.project().expect("project open");
    s.store
        .with_writer(|c| {
            c.execute_batch(sql).expect("fixture sql");
            Ok(())
        })
        .unwrap();
}

fn query_one(env: &TestEnv, sql: &str) -> String {
    let s = env.core.project().unwrap();
    s.store
        .read(|c| Ok(c.query_row(sql, [], |r| r.get::<_, String>(0))?))
        .unwrap()
}

/// Locked draft with 4 scenes (+1 omitted), cast, locations, catalog and
/// breakdown (one suggested element) as the active Production Source.
fn seed_production(env: &TestEnv) {
    exec(
        env,
        "INSERT INTO screenplay(id,title,format,created_at,updated_at) VALUES ('sp','Black Rain','Feature',1,1);
         INSERT INTO screenplay_draft(id,screenplay_id,name,status,created_at,updated_at) VALUES ('d1','sp','Shooting Draft 6','Locked',1,1);
         INSERT INTO screenplay_scene(id,draft_id,lineage_id,position,heading,synopsis,created_at,updated_at) VALUES
            ('s1','d1','L1',1,'INT. POLICE STATION - NIGHT','Meera hands the file',1,1),
            ('s2','d1','L2',2,'EXT. OLD RAILWAY STATION - DAY',NULL,1,1),
            ('s3','d1','L3',3,'INT. MORGUE - DAY','Time of death',1,1),
            ('s4','d1','L4',4,'EXT. OLD RAILWAY STATION - NIGHT','Watcher on platform',1,1);
         INSERT INTO screenplay_scene(id,draft_id,lineage_id,position,heading,omitted,created_at,updated_at) VALUES
            ('s9','d1','L9',5,'EXT. NOWHERE - DAY',1,1,1);
         INSERT INTO screenplay_element(id,scene_id,position,element_type,text,created_at,updated_at) VALUES
            ('e1','s1',1,'action','Rain lashes the windows. MEERA slides a red folder across the desk.',1,1),
            ('e2','s1',2,'character','MEERA',1,1),
            ('e3','s1',3,'dialogue','You will want to see this before anyone else does.',1,1),
            ('e4','s2',1,'action','ARJUN walks the empty platform, photograph in hand.',1,1),
            ('e5','s3',1,'action','A sheet is pulled back.',1,1),
            ('e6','s4',1,'action','A figure watches from the far platform.',1,1);
         INSERT INTO story_character(id,name,position,created_at,updated_at) VALUES ('arjun','Arjun',1,1,1),('meera','Meera',2,1,1);
         INSERT INTO cast_member(id,person_name,character_id,created_at,updated_at) VALUES ('cm1','Karthik Menon','arjun',1,1);
         INSERT INTO location(id,name,address,status,created_at,updated_at) VALUES
            ('loc_police','Police Station','District Rd, Gudur','Confirmed',1,1),
            ('loc_rail','Old Railway Station',NULL,'Confirmed',1,1);
         INSERT INTO catalog_item(id,category,name,character_id,location_id,created_at,updated_at) VALUES
            ('ci_arjun','Cast','Arjun','arjun',NULL,1,1),
            ('ci_meera','Cast','Meera','meera',NULL,1,1),
            ('ci_police','Location / Set','Police Station',NULL,'loc_police',1,1),
            ('ci_rail','Location / Set','Old Railway Station',NULL,'loc_rail',1,1),
            ('ci_folder','Props','Red Folder',NULL,NULL,1,1);
         INSERT INTO production_source(id,draft_id,active,selected_at,created_at,updated_at) VALUES ('ps1','d1',1,1,1,1);
         INSERT INTO breakdown_element(id,source_id,scene_id,scene_lineage_id,category,catalog_item_id,display_name,confirmation_state,notes,created_at,updated_at) VALUES
            ('b1','ps1','s1','L1','Cast','ci_arjun','Arjun','Confirmed',NULL,1,1),
            ('b2','ps1','s1','L1','Cast','ci_meera','Meera','Confirmed',NULL,1,1),
            ('b3','ps1','s1','L1','Location / Set','ci_police','Police Station','Confirmed',NULL,1,1),
            ('b4','ps1','s1','L1','Props','ci_folder','Red Folder','Manual','Must be water-stained',1,1),
            ('b5','ps1','s2','L2','Cast','ci_arjun','Arjun','Confirmed',NULL,1,1),
            ('b6','ps1','s2','L2','Location / Set','ci_rail','Old Railway Station','Confirmed',NULL,1,1),
            ('b7','ps1','s3','L3','Cast','ci_meera','Meera','Confirmed',NULL,1,1),
            ('b8','ps1','s4','L4','Cast','ci_arjun','Arjun','Confirmed',NULL,1,1),
            ('b9','ps1','s4','L4','Location / Set','ci_rail','Old Railway Station','Confirmed',NULL,1,1),
            ('b10','ps1','s3','L3','Props',NULL,'Hospital Sheet','Suggested',NULL,1,1);",
    );
}

/// Story Board: two acts, a sequence with a card and a beat, a card directly
/// in Act One, a Parking Lot card, notes, a comment and a private note.
fn seed_story(env: &TestEnv) {
    let me = env.actor().user_id;
    exec(
        env,
        &format!(
            "INSERT INTO story_act(id,title,note,position,created_at,updated_at) VALUES
                ('a1','Act One','ACT-NOTE-ONE',1,1,1),('a2','Act Two',NULL,2,1,1);
             INSERT INTO story_sequence(id,act_id,title,note,position,created_at,updated_at) VALUES
                ('q1','a1','The Arrival',NULL,1,1,1);
             INSERT INTO story_scene_card(id,parent_type,parent_id,short_description,scene_heading,notes,position,created_at,updated_at) VALUES
                ('c1','sequence','q1','Arjun steps off the night bus','EXT. BUS STAND - NIGHT','CARD-NOTE-VISIBLE-WHEN-ASKED',1,1,1),
                ('c2','act','a1','Meera calls the station',NULL,NULL,2,1,1),
                ('c3','act','a2','The morgue reveal',NULL,NULL,1,1,1),
                ('c4','parking',NULL,'Unused dream sequence',NULL,NULL,1,1,1);
             INSERT INTO story_beat(id,parent_type,parent_id,text,position,created_at,updated_at) VALUES
                ('bt1','sequence','q1','Meera sees him from the window',2,1,1);
             INSERT INTO comment(id,target_type,target_id,body,author_user_id,author_name,created_at,updated_at) VALUES
                ('cm1','story_scene_card','c1','Tighten this arrival','u2','Ravi',1,1);
             INSERT INTO private_note(id,owner_user_id,target_type,target_id,body,created_at,updated_at) VALUES
                ('pn1','{me}','story_scene_card','c1','PRIVATE-SECRET-XYZ',1,1);"
        ),
    );
}

fn out_dir(env: &TestEnv) -> PathBuf {
    let d = env.dir.path().join("exports");
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn dest(env: &TestEnv, name: &str) -> String {
    out_dir(env).join(name).to_string_lossy().into_owned()
}

/// Debugging aid: `OPENFRAME_KEEP_EXPORTS=<folder>` keeps a copy of every PDF
/// the tests read, for visual review.
fn keep(path: &str) {
    if let Ok(dir) = std::env::var("OPENFRAME_KEEP_EXPORTS") {
        let name = Path::new(path).file_name().unwrap();
        let _ = std::fs::copy(path, Path::new(&dir).join(name));
    }
}

/// All text shown on the PDF's pages (Tj strings, WinAnsi-decoded), whitespace-normalised.
fn pdf_text(path: &str) -> String {
    keep(path);
    let doc = lopdf::Document::load(path).expect("PDF parses with lopdf");
    let mut out = String::new();
    for (_, page) in doc.get_pages() {
        let bytes = doc.get_page_content(page);
        let content = lopdf::content::Content::decode(&bytes).expect("content stream");
        for op in content.operations {
            if op.operator == "Tj"
                && let Some(lopdf::Object::String(b, _)) = op.operands.first()
            {
                out.extend(
                    b.iter()
                        .map(|x| openframe_import_export::winansi::decode_byte(*x)),
                );
                out.push(' ');
            }
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn pdf_pages(path: &str) -> usize {
    lopdf::Document::load(path).unwrap().get_pages().len()
}

fn pdf_page_width(path: &str) -> f32 {
    let doc = lopdf::Document::load(path).unwrap();
    let (_, page) = doc.get_pages().into_iter().next().unwrap();
    let dict = doc.get_dictionary(page).unwrap();
    let mb = dict.get(b"MediaBox").unwrap().as_array().unwrap();
    mb[2].as_float().unwrap()
}

fn pdf_images(path: &str) -> usize {
    let doc = lopdf::Document::load(path).unwrap();
    doc.objects
        .values()
        .filter(|o| match o {
            lopdf::Object::Stream(s) => {
                s.dict.get(b"Subtype").ok().and_then(|v| v.as_name().ok())
                    == Some(b"Image".as_slice())
            }
            _ => false,
        })
        .count()
}

fn csv_text(path: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    assert!(
        bytes.starts_with(b"\xEF\xBB\xBF"),
        "CSV is UTF-8 with a BOM"
    );
    String::from_utf8(bytes[3..].to_vec()).unwrap()
}

/// Sheet names and all strings of an XLSX workbook.
fn xlsx(path: &str) -> (Vec<String>, String) {
    let f = std::fs::File::open(path).unwrap();
    let mut z = zip::ZipArchive::new(f).expect("XLSX is a readable zip package");
    let mut workbook = String::new();
    z.by_name("xl/workbook.xml")
        .unwrap()
        .read_to_string(&mut workbook)
        .unwrap();
    let sheets: Vec<String> = workbook
        .split("<sheet ")
        .skip(1)
        .filter_map(|s| {
            s.split("name=\"")
                .nth(1)
                .and_then(|x| x.split('"').next())
                .map(str::to_string)
        })
        .collect();
    let mut text = String::new();
    for i in 0..z.len() {
        let mut e = z.by_index(i).unwrap();
        let name = e.name().to_string();
        if name.starts_with("xl/") && name.ends_with(".xml") {
            let mut s = String::new();
            e.read_to_string(&mut s).unwrap();
            text.push_str(&s);
        }
    }
    (sheets, text)
}

fn files_in(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

fn id(v: &Value) -> String {
    v["id"].as_str().expect("id").to_string()
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

/// Schedule with Shoot Day 1 (scenes 1, 2 and a meal break) and scenes 3–4 unscheduled.
fn seed_schedule(env: &TestEnv) -> String {
    let sched = id(&env.ok("schedule.create", json!({})));
    let day = env
        .ok(
            "schedule.create_day",
            json!({ "scheduleId": sched, "date": "2027-06-14" }),
        )
        .as_str()
        .unwrap()
        .to_string();
    for n in [1, 2] {
        env.ok(
            "schedule.move_strip",
            json!({ "stripId": strip(env, n), "dayId": day }),
        );
    }
    env.ok(
        "schedule.add_marker",
        json!({ "dayId": day, "markerType": "Meal", "atTime": "20:30", "durationMinutes": 45, "index": 1 }),
    );
    day
}

fn production_env() -> TestEnv {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    seed_production(&env);
    env
}

// ================================================================== Story Board

#[test]
fn story_board_outline_exports_in_board_order_with_optional_notes() {
    let env = production_env();
    seed_story(&env);
    let pdf = dest(&env, "Black Rain - Story Board.pdf");
    let r = env.ok(
        "story.export_board",
        json!({ "format": "pdf", "path": pdf }),
    );
    assert_eq!(r["format"], "pdf");
    assert_eq!(r["privateNotesExcluded"], true);
    assert!(
        r["summary"]
            .as_str()
            .unwrap()
            .contains("Private notes excluded")
    );
    assert_eq!(
        r["bytesWritten"].as_u64().unwrap(),
        std::fs::metadata(&pdf).unwrap().len()
    );
    let text = pdf_text(&pdf);
    for want in [
        "Story Board Outline",
        "Act One",
        "Sequence — The Arrival",
        "Arjun steps off the night bus",
        "Beat: Meera sees him",
        "Act Two",
        "BLACK RAIN",
    ] {
        assert!(
            text.to_uppercase().contains(&want.to_uppercase()),
            "missing {want:?} in {text}"
        );
    }
    let pos = |s: &str| text.find(s).unwrap_or_else(|| panic!("{s} not in PDF"));
    assert!(pos("Arjun steps off") < pos("Meera sees him"));
    assert!(pos("Meera sees him") < pos("Meera calls the station"));
    assert!(pos("Meera calls the station") < pos("The morgue reveal"));
    for absent in [
        "Unused dream",
        "CARD-NOTE",
        "ACT-NOTE-ONE",
        "Tighten this",
        "PRIVATE-SECRET",
    ] {
        assert!(
            !text.contains(absent),
            "{absent} must not be exported by default"
        );
    }

    // Explicit selection adds Parking Lot, notes and comments — never private notes.
    let pdf2 = dest(&env, "outline-all.pdf");
    env.ok(
        "story.export_board",
        json!({ "format": "pdf", "path": pdf2, "includeParking": true, "includeNotes": true, "includeComments": true }),
    );
    let text = pdf_text(&pdf2);
    for want in [
        "Parking Lot",
        "Unused dream sequence",
        "CARD-NOTE-VISIBLE-WHEN-ASKED",
        "ACT-NOTE-ONE",
        "Tighten this arrival",
    ] {
        assert!(text.contains(want), "missing {want}");
    }
    assert!(!text.contains("PRIVATE-SECRET"));

    // Board layout is a landscape card grid.
    let board = dest(&env, "board.pdf");
    let r = env.ok(
        "story.export_board",
        json!({ "format": "pdf", "path": board, "layout": "board" }),
    );
    assert_eq!(r["document"], "Story Board");
    assert!(pdf_page_width(&board) > 700.0, "landscape");
    assert!(pdf_text(&board).contains("Scene Card 1"));

    // TXT and CSV carry the same outline.
    let txt = dest(&env, "outline.txt");
    env.ok(
        "story.export_board",
        json!({ "format": "txt", "path": txt }),
    );
    let bytes = std::fs::read(&txt).unwrap();
    assert!(bytes.starts_with(b"\xEF\xBB\xBF"));
    let t = String::from_utf8(bytes[3..].to_vec()).unwrap();
    assert!(
        t.contains("ACT ONE")
            && t.contains("SEQUENCE — The Arrival")
            && t.contains("Scene Card 1 · EXT. BUS STAND - NIGHT")
    );
    assert!(t.contains("Private notes excluded."));
    let csv = dest(&env, "outline.csv");
    env.ok(
        "story.export_board",
        json!({ "format": "csv", "path": csv }),
    );
    let c = csv_text(&csv);
    assert!(c.starts_with("Episode,Area,Act,Sequence,Type,Card #,Heading,Description"));
    assert!(c.contains("Scene Card") && c.contains("Beat") && c.contains("The morgue reveal"));
    assert!(!c.contains("PRIVATE-SECRET"));
    // Board layout is PDF only.
    assert_eq!(
        env.err(
            "story.export_board",
            json!({ "format": "csv", "path": dest(&env, "x.csv"), "layout": "board" })
        ),
        "validation.invalid_input"
    );
}

#[test]
fn story_board_selected_items_scope() {
    let env = production_env();
    seed_story(&env);
    let pdf = dest(&env, "selected.pdf");
    let r = env.ok(
        "story.export_board",
        json!({ "format": "pdf", "path": pdf, "scope": "selected", "itemIds": ["q1"] }),
    );
    assert_eq!(r["scopeLabel"], "Selected items only");
    let text = pdf_text(&pdf);
    assert!(text.contains("Arjun steps off") && text.contains("Meera sees him"));
    assert!(!text.contains("The morgue reveal") && !text.contains("Meera calls the station"));
    assert_eq!(
        env.err(
            "story.export_board",
            json!({ "format": "pdf", "path": pdf, "scope": "selected", "itemIds": [] })
        ),
        "validation.itemIds"
    );
    // An empty board has nothing to export.
    let empty = TestEnv::with_project("Empty", "Feature Film");
    assert_eq!(
        empty.err(
            "story.export_board",
            json!({ "format": "pdf", "path": dest(&empty, "e.pdf") })
        ),
        "export.nothing_to_export"
    );
}

// ================================================================== Breakdown

#[test]
fn breakdown_report_marks_suggested_and_lists_catalog_usage() {
    let env = production_env();
    let pdf = dest(&env, "breakdown.pdf");
    let r = env.ok(
        "breakdown.export_report",
        json!({ "format": "pdf", "path": pdf }),
    );
    assert_eq!(r["sourceLabel"], "Black Rain — Shooting Draft 6");
    let text = pdf_text(&pdf);
    for want in [
        "Breakdown Report",
        "Scene 1 — INT. POLICE STATION - NIGHT",
        "Red Folder",
        "Hospital Sheet (suggested)",
        "Catalog",
        "Used in scenes",
        "Scene 5 — OMITTED",
    ] {
        assert!(text.contains(want), "missing {want}: {text}");
    }
    assert!(
        !text.contains("water-stained"),
        "element notes only when selected"
    );

    // CSV: breakdown by scene (default) and catalog.
    let csv = dest(&env, "breakdown.csv");
    env.ok(
        "breakdown.export_report",
        json!({ "format": "csv", "path": csv, "includeNotes": true }),
    );
    let c = csv_text(&csv);
    assert!(c.starts_with("Scene,Heading,Category,Element,State,Catalog item,Notes"));
    assert!(
        c.contains("Suggested")
            && c.contains("Added manually")
            && c.contains("Must be water-stained")
    );
    let cat = dest(&env, "catalog.csv");
    env.ok(
        "breakdown.export_report",
        json!({ "format": "csv", "path": cat, "content": "catalog" }),
    );
    let c = csv_text(&cat);
    assert!(c.starts_with("Category,Item,Status,Used in scenes,Scenes,Description"));
    assert!(
        c.lines()
            .any(|l| l.starts_with("Location / Set,Old Railway Station") && l.contains("\"2, 4\"")),
        "{c}"
    );
    assert_eq!(
        env.err(
            "breakdown.export_report",
            json!({ "format": "csv", "path": cat, "content": "both" })
        ),
        "validation.invalid_input"
    );

    // XLSX: one sheet per table.
    let x = dest(&env, "breakdown.xlsx");
    env.ok(
        "breakdown.export_report",
        json!({ "format": "xlsx", "path": x }),
    );
    let (sheets, text) = xlsx(&x);
    assert_eq!(sheets, vec!["Breakdown by Scene", "Catalog"]);
    assert!(text.contains("Red Folder") && text.contains("Old Railway Station"));

    // Suggested elements can be left out; a scene selection narrows scenes and catalog.
    let p2 = dest(&env, "confirmed-s2.pdf");
    let r = env.ok(
        "breakdown.export_report",
        json!({ "format": "pdf", "path": p2, "includeSuggested": false, "sceneIds": ["s2"] }),
    );
    assert_eq!(r["scopeLabel"], "1 selected scene");
    let t = pdf_text(&p2);
    assert!(t.contains("Scene 2 — EXT. OLD RAILWAY STATION - DAY"));
    assert!(!t.contains("Scene 1 —") && !t.contains("Hospital Sheet") && !t.contains("Red Folder"));
    assert_eq!(
        env.err(
            "breakdown.export_report",
            json!({ "format": "pdf", "path": p2, "sceneIds": ["nope"] })
        ),
        "not_found.scene"
    );
}

// ================================================================== Moodboard

#[test]
fn moodboard_pdf_embeds_images_and_excludes_internal_notes_unless_selected() {
    let env = production_env();
    let bid = id(&env.ok("moodboard.create", json!({ "name": "Overall Look" })));
    let img = env.write_file("rain.png", &png_bytes());
    let imgs = env.ok(
        "moodboard.add_images",
        json!({ "moodboardId": bid, "paths": [img], "x": 20, "y": 24 }),
    );
    env.ok(
        "moodboard.update_item",
        json!({ "id": id(&imgs[0]), "caption": "Rain on glass — main tone" }),
    );
    env.ok(
        "moodboard.add_note",
        json!({ "moodboardId": bid, "text": "Cold, wet, blue-grey." }),
    );
    env.ok(
        "moodboard.add_note",
        json!({ "moodboardId": bid, "text": "Budget: rain machine hire", "isPrivate": true }),
    );
    env.ok("moodboard.add_link", json!({ "moodboardId": bid, "url": "www.imdb.com/title/tt0353969", "title": "Memories of Murder" }));
    env.ok(
        "moodboard.update_notes",
        json!({ "id": bid, "notes": "INTERNAL-BOARD-NOTE" }),
    );

    let pdf = dest(&env, "mood.pdf");
    let r = env.ok(
        "moodboard.export_pdf",
        json!({ "moodboardId": bid, "format": "pdf", "path": pdf }),
    );
    assert_eq!(r["document"], "Moodboard — Overall Look");
    assert_eq!(pdf_images(&pdf), 1, "the image is embedded");
    let text = pdf_text(&pdf);
    for want in [
        "Moodboard — Overall Look",
        "Rain on glass — main tone",
        "Cold, wet, blue-grey.",
        "Memories of Murder",
    ] {
        assert!(text.contains(want), "missing {want}");
    }
    assert!(
        !text.contains("rain machine") && !text.contains("INTERNAL-BOARD-NOTE"),
        "internal notes excluded by default"
    );

    let with = dest(&env, "mood-internal.pdf");
    env.ok(
        "moodboard.export_pdf",
        json!({ "moodboardId": bid, "format": "pdf", "path": with, "includeInternalNotes": true }),
    );
    let text = pdf_text(&with);
    assert!(text.contains("Budget: rain machine hire") && text.contains("INTERNAL-BOARD-NOTE"));

    let bare = dest(&env, "mood-bare.pdf");
    env.ok(
        "moodboard.export_pdf",
        json!({ "moodboardId": bid, "format": "pdf", "path": bare, "includeImages": false, "includeCaptions": false }),
    );
    assert_eq!(pdf_images(&bare), 0);
    assert!(!pdf_text(&bare).contains("Rain on glass"));
    // PDF is the only format.
    assert_eq!(
        env.err(
            "moodboard.export_pdf",
            json!({ "moodboardId": bid, "format": "csv", "path": dest(&env, "m.csv") })
        ),
        "validation.invalid_input"
    );
}

// ================================================================== Storyboard

#[test]
fn storyboard_sheet_has_panels_shot_numbers_framing_and_optional_notes() {
    let env = production_env();
    let bid = id(&env.ok("storyboard.create", json!({ "sceneId": "s1" })));
    let shot = id(&env.ok(
        "shot.create",
        json!({ "sceneId": "s1", "description": "Establishing" }),
    ));
    let p1 = id(&env.ok(
        "storyboard.add_panel",
        json!({ "storyboardId": bid, "visual": "placeholder", "description": "Wide — Arjun enters" }),
    ));
    env.ok(
        "storyboard.link_shot",
        json!({ "panelId": p1, "shotId": shot }),
    );
    let img = env.write_file("frame.png", &png_bytes());
    let p2 = env.ok(
        "storyboard.add_panel",
        json!({ "storyboardId": bid, "visual": "image", "path": img }),
    );
    env.ok(
        "storyboard.update_panel",
        json!({ "id": id(&p2), "description": "Meera looks up", "framing": "Medium", "movement": "Static", "angle": "Low",
                "soundNote": "Phone rings", "durationMs": 3500, "note": "hold on her eyes" }),
    );

    let pdf = dest(&env, "board.pdf");
    let r = env.ok(
        "storyboard.export_sheet",
        json!({ "storyboardIds": [bid], "format": "pdf", "path": pdf }),
    );
    assert_eq!(r["contentsLabel"], "2 panels");
    assert!(pdf_page_width(&pdf) > 700.0, "landscape sheet");
    assert_eq!(pdf_images(&pdf), 1);
    let text = pdf_text(&pdf);
    for want in [
        "Storyboard — INT. POLICE STATION - NIGHT",
        "Scene 1 — INT. POLICE STATION - NIGHT",
        "Panel 1 · Shot 1A",
        "Blank panel",
        "Wide — Arjun enters",
        "Panel 2",
        "Medium · Static · Low",
        "Sound: Phone rings",
        "Duration: 3.5s",
    ] {
        assert!(text.contains(want), "missing {want}: {text}");
    }
    assert!(
        !text.contains("hold on her eyes"),
        "shot notes are optional"
    );
    let with = dest(&env, "board-notes.pdf");
    env.ok(
        "storyboard.export_sheet",
        json!({ "format": "pdf", "path": with, "includeNotes": true }),
    );
    assert!(pdf_text(&with).contains("Note: hold on her eyes"));

    // A missing image file is reported and shown as a placeholder; the panel keeps its reference.
    let path = p2["asset"]["path"].as_str().unwrap().to_string();
    std::fs::remove_file(&path).unwrap();
    let missing = dest(&env, "board-missing.pdf");
    let r = env.ok(
        "storyboard.export_sheet",
        json!({ "storyboardIds": [bid], "format": "pdf", "path": missing }),
    );
    assert_eq!(r["warnings"].as_array().unwrap().len(), 1, "{r}");
    assert!(pdf_text(&missing).contains("Image unavailable"));
    assert_eq!(
        env.ok("storyboard.get", json!({ "id": bid }))["panels"][1]["visualKind"],
        "image"
    );
}

// ================================================================== Shot List

#[test]
fn shot_list_exports_by_scene_selection_or_whole_project() {
    let env = production_env();
    for (scene, desc, size) in [
        ("s1", "Wide on the station", "Wide"),
        ("s1", "Meera hands the file", "Medium"),
        ("s2", "Arjun on the platform", "Wide"),
    ] {
        env.ok(
            "shot.create",
            json!({ "sceneId": scene, "description": desc, "size": size, "characters": ["ARJUN"] }),
        );
    }
    let pdf = dest(&env, "shots.pdf");
    let r = env.ok("shot.export_list", json!({ "format": "pdf", "path": pdf }));
    assert_eq!(r["scopeLabel"], "Whole project");
    assert_eq!(r["contentsLabel"], "3 shots in 2 scenes");
    let text = pdf_text(&pdf);
    for want in [
        "Shot List",
        "Scene 1 — INT. POLICE STATION - NIGHT",
        "1A",
        "1B",
        "2A",
        "Meera hands the file",
        "ARJUN",
    ] {
        assert!(text.contains(want), "missing {want}");
    }
    let csv = dest(&env, "shots-s2.csv");
    let r = env.ok(
        "shot.export_list",
        json!({ "format": "csv", "path": csv, "sceneIds": ["s2"] }),
    );
    assert_eq!(r["scopeLabel"], "Current scene");
    let c = csv_text(&csv);
    assert!(c.contains("2A") && c.contains("Arjun on the platform") && !c.contains("1A"));
    let x = dest(&env, "shots.xlsx");
    env.ok(
        "shot.export_list",
        json!({ "format": "xlsx", "path": x, "sceneIds": ["s1", "s2"] }),
    );
    let (sheets, text) = xlsx(&x);
    assert_eq!(sheets, vec!["Shot List"]);
    assert!(text.contains("Wide on the station"));
    assert_eq!(
        env.err(
            "shot.export_list",
            json!({ "format": "pdf", "path": pdf, "sceneIds": ["s3"] })
        ),
        "export.nothing_to_export"
    );
}

// ================================================================== Schedule

#[test]
fn schedule_export_has_days_markers_summaries_and_unscheduled() {
    let env = production_env();
    let day = seed_schedule(&env);
    let pdf = dest(&env, "schedule.pdf");
    let r = env.ok(
        "schedule.export_schedule",
        json!({ "format": "pdf", "path": pdf }),
    );
    assert_eq!(r["scopeLabel"], "Entire schedule");
    assert!(pdf_page_width(&pdf) > 700.0, "stripboard is landscape");
    let text = pdf_text(&pdf);
    for want in [
        "Shooting Schedule",
        "Shoot Day 1 — Mon 14 Jun 2027",
        "INT. POLICE STATION - NIGHT",
        "MEAL",
        "20:30",
        "Unscheduled",
        "INT. MORGUE - DAY",
        "Shooting Draft 6",
    ] {
        assert!(text.contains(want), "missing {want}: {text}");
    }
    let csv = dest(&env, "schedule.csv");
    env.ok(
        "schedule.export_schedule",
        json!({ "format": "csv", "path": csv }),
    );
    let c = csv_text(&csv);
    assert!(c.starts_with(
        "Day,Date,Order,Item,Scene,I/E,D/N,Heading,Location,Synopsis,Cast,Pages,Est. time"
    ));
    assert!(c.contains("Shoot Day 1") && c.contains("Unscheduled") && c.contains("Meal"));
    let x = dest(&env, "schedule.xlsx");
    env.ok(
        "schedule.export_schedule",
        json!({ "format": "xlsx", "path": x }),
    );
    assert_eq!(xlsx(&x).0, vec!["Schedule", "Days"]);

    let one = dest(&env, "day.pdf");
    let r = env.ok(
        "schedule.export_schedule",
        json!({ "format": "pdf", "path": one, "dayIds": [day], "includeUnscheduled": false }),
    );
    assert_eq!(r["scopeLabel"], "Current day only");
    let t = pdf_text(&one);
    assert!(t.contains("Shoot Day 1") && !t.contains("Unscheduled") && !t.contains("MORGUE"));
    assert_eq!(
        env.err(
            "schedule.export_schedule",
            json!({ "format": "pdf", "path": one, "dayIds": [] })
        ),
        "validation.dayIds"
    );
}

#[test]
fn exports_are_snapshots_and_never_change_the_project() {
    let env = production_env();
    let day = seed_schedule(&env);
    let before_view = env.ok("schedule.get", json!({}));
    let before_undo = env.ok("history.info", json!({}));
    let csv = dest(&env, "snap.csv");
    env.ok(
        "schedule.export_schedule",
        json!({ "format": "csv", "path": csv }),
    );
    let pdf = dest(&env, "snap.pdf");
    env.ok(
        "breakdown.export_report",
        json!({ "format": "pdf", "path": pdf }),
    );
    // Exporting mutates nothing: same schedule, no new undo step.
    assert_eq!(before_view, env.ok("schedule.get", json!({})));
    assert_eq!(before_undo, env.ok("history.info", json!({})));

    // Later edits never touch the written files.
    let snap = std::fs::read(&csv).unwrap();
    env.ok(
        "schedule.move_strip",
        json!({ "stripId": strip(&env, 3), "dayId": day }),
    );
    exec(
        &env,
        "UPDATE screenplay_scene SET heading = 'INT. NEW PLACE - DAY' WHERE id = 's1'",
    );
    assert_eq!(std::fs::read(&csv).unwrap(), snap);
    // …and a new export reflects the new state.
    let csv2 = dest(&env, "snap2.csv");
    env.ok(
        "schedule.export_schedule",
        json!({ "format": "csv", "path": csv2 }),
    );
    assert!(csv_text(&csv2).contains("INT. NEW PLACE - DAY"));
    assert!(!csv_text(&csv).contains("INT. NEW PLACE - DAY"));
    // Re-exporting over an existing file replaces it atomically (no leftovers).
    env.ok(
        "schedule.export_schedule",
        json!({ "format": "csv", "path": csv }),
    );
    assert!(
        files_in(&out_dir(&env))
            .iter()
            .all(|f| !f.contains("partial"))
    );
}

// ================================================================== Call Sheet

#[test]
fn call_sheet_pdf_has_paper_layout_and_issued_sheet_uses_its_snapshot() {
    let env = production_env();
    let day = seed_schedule(&env);
    let cs = env
        .ok("callsheets.create", json!({ "dayId": day }))
        .as_str()
        .unwrap()
        .to_string();
    let mut doc = env.ok("callsheets.get", json!({ "id": cs }))["document"].clone();
    doc["crewCall"] = json!("18:00");
    doc["practical"]["parking"] = json!("Behind the station");
    doc["optional"]["weather"] = json!("Rain expected after 21:00");
    env.ok("callsheets.update", json!({ "id": cs, "document": doc }));

    let draft = dest(&env, "call-draft.pdf");
    let r = env.ok(
        "callsheets.export_pdf",
        json!({ "id": cs, "format": "pdf", "path": draft }),
    );
    assert!(
        r["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w.as_str().unwrap().contains("not finalized"))
    );
    let text = pdf_text(&draft);
    for want in [
        "BLACK RAIN",
        "SHOOT DAY 1",
        "MONDAY 14 JUNE 2027",
        "CREW CALL: 18:00",
        "LOCATION",
        "Police Station",
        "CAST",
        "Karthik Menon",
        "SCENES",
        "INT. POLICE STATION - NIGHT",
        "PRACTICAL NOTES",
        "Behind the station",
        "WEATHER",
        "Rain expected",
        "DRAFT — not finalized",
    ] {
        assert!(text.contains(want), "missing {want}: {text}");
    }
    let no_optional = dest(&env, "call-plain.pdf");
    env.ok(
        "callsheets.export_pdf",
        json!({ "id": cs, "format": "pdf", "path": no_optional, "includeOptional": false }),
    );
    assert!(!pdf_text(&no_optional).contains("Rain expected"));

    env.ok("callsheets.finalize", json!({ "id": cs }));
    env.ok("callsheets.issue", json!({ "id": cs }));
    // Even if the stored working copy were changed later, the issued PDF comes from the frozen snapshot.
    exec(
        &env,
        &format!(
            "UPDATE call_sheet SET document_json = replace(document_json, '18:00', '23:59') WHERE id = '{cs}'"
        ),
    );
    let issued = dest(&env, "call-issued.pdf");
    let r = env.ok(
        "callsheets.export_pdf",
        json!({ "id": cs, "format": "pdf", "path": issued }),
    );
    assert!(
        r["sourceLabel"]
            .as_str()
            .unwrap()
            .contains("issued snapshot")
    );
    assert!(r["warnings"].as_array().unwrap().is_empty(), "{r}");
    let text = pdf_text(&issued);
    assert!(text.contains("CREW CALL: 18:00") && !text.contains("23:59"));
    assert!(text.contains("Issued"));
    assert_eq!(
        env.err(
            "callsheets.export_pdf",
            json!({ "id": "missing", "format": "pdf", "path": issued })
        ),
        "not_found.call_sheet"
    );
}

// ================================================================== Sides

#[test]
fn sides_pdf_contains_only_the_days_scenes_in_screenplay_format() {
    let env = production_env();
    let day = seed_schedule(&env);
    let pdf = dest(&env, "sides.pdf");
    let r = env.ok(
        "sides.export_pdf",
        json!({ "format": "pdf", "path": pdf, "dayId": day, "includeCover": true, "showDraftName": true }),
    );
    assert_eq!(r["contentsLabel"], "2 scenes");
    assert!(
        r["sourceLabel"]
            .as_str()
            .unwrap()
            .contains("Shooting Draft 6")
    );
    let text = pdf_text(&pdf);
    for want in [
        "BLACK RAIN · SIDES · SHOOT DAY 1",
        "Shooting Draft 6",
        "LOCATION",
        "INT. POLICE STATION - NIGHT",
        "Rain lashes the windows",
        "MEERA",
        "ARJUN walks the empty platform",
    ] {
        assert!(text.contains(want), "missing {want}: {text}");
    }
    assert!(
        !text.contains("A sheet is pulled back"),
        "only the day's scenes"
    );
    assert!(pdf_pages(&pdf) >= 2, "cover page + script page");

    // A saved sides snapshot keeps its text after later screenplay edits.
    let side = env
        .ok("sides.create", json!({ "dayId": day }))
        .as_str()
        .unwrap()
        .to_string();
    exec(
        &env,
        "UPDATE screenplay_element SET text = 'CHANGED SCRIPT TEXT' WHERE id = 'e1'",
    );
    let saved = dest(&env, "sides-saved.pdf");
    let r = env.ok(
        "sides.export_pdf",
        json!({ "format": "pdf", "path": saved, "sideId": side }),
    );
    assert_eq!(r["scopeLabel"], "Saved sides");
    let t = pdf_text(&saved);
    assert!(t.contains("Rain lashes the windows") && !t.contains("CHANGED SCRIPT TEXT"));
    let live = dest(&env, "sides-live.pdf");
    env.ok(
        "sides.export_pdf",
        json!({ "format": "pdf", "path": live, "dayId": day }),
    );
    assert!(pdf_text(&live).contains("CHANGED SCRIPT TEXT"));
    assert_eq!(
        env.err("sides.export_pdf", json!({ "format": "pdf", "path": live })),
        "validation.required"
    );
}

// ================================================================== Reports & Budget

#[test]
fn reports_export_live_and_saved_in_every_format() {
    let env = production_env();
    seed_schedule(&env);
    let csv = dest(&env, "props.csv");
    let r = env.ok(
        "reports.export_report",
        json!({ "format": "csv", "path": csv, "reportType": "prop" }),
    );
    assert_eq!(r["document"], "Prop Report");
    assert!(csv_text(&csv).contains("Red Folder"));
    for t in [
        "scene",
        "location",
        "cast_scene",
        "prop",
        "schedule",
        "breakdown_completeness",
    ] {
        let p = dest(&env, &format!("{t}.pdf"));
        env.ok(
            "reports.export_report",
            json!({ "format": "pdf", "path": p, "reportType": t }),
        );
        assert!(pdf_pages(&p) >= 1);
    }
    let saved = env
        .ok("reports.save", json!({ "reportType": "location" }))
        .as_str()
        .unwrap()
        .to_string();
    let x = dest(&env, "locations.xlsx");
    let r = env.ok(
        "reports.export_report",
        json!({ "format": "xlsx", "path": x, "reportId": saved }),
    );
    assert!(
        r["scopeLabel"]
            .as_str()
            .unwrap()
            .starts_with("Saved report")
    );
    assert!(xlsx(&x).1.contains("Old Railway Station"));
    assert_eq!(
        env.err(
            "reports.export_report",
            json!({ "format": "pdf", "path": x, "reportType": "nope" })
        ),
        "validation.invalid_input"
    );
}

#[test]
fn budget_summary_exports_totals_and_lines() {
    let env = production_env();
    let b = env
        .ok("budget.create", json!({ "currency": "INR" }))
        .as_str()
        .unwrap()
        .to_string();
    env.ok("budget.add_line", json!({ "budgetId": b, "category": "Cast", "description": "Lead actor", "amount": 5_000_000, "notes": "Fee negotiable" }));
    env.ok("budget.add_line", json!({ "budgetId": b, "category": "Food", "description": "Catering", "amount": 1_200_000 }));
    env.ok(
        "budget.update",
        json!({ "id": b, "currency": "INR", "plannedTotal": 120_000_000, "contingencyMode": "percent", "contingencyValue": 1000 }),
    );
    let pdf = dest(&env, "budget.pdf");
    env.ok(
        "budget.export_summary",
        json!({ "format": "pdf", "path": pdf }),
    );
    let text = pdf_text(&pdf);
    for want in [
        "Budget Summary",
        "INR 12,00,000",
        "10% · INR 1,20,000",
        "Entered so far",
        "INR 62,000",
        "Lead actor",
        "50,000",
        "Catering",
    ] {
        assert!(text.contains(want), "missing {want}: {text}");
    }
    assert!(!text.contains("Fee negotiable"));
    let csv = dest(&env, "budget.csv");
    env.ok(
        "budget.export_summary",
        json!({ "format": "csv", "path": csv, "includeNotes": true }),
    );
    let c = csv_text(&csv);
    assert!(c.starts_with("Category,Line item,Amount (INR),Notes"));
    assert!(c.contains("Lead actor,50000.00,Fee negotiable"));
    let x = dest(&env, "budget.xlsx");
    env.ok(
        "budget.export_summary",
        json!({ "format": "xlsx", "path": x }),
    );
    assert_eq!(xlsx(&x).0, vec!["Line items", "Categories", "Summary"]);
    let none = TestEnv::with_project("No Budget", "Short Film");
    assert_eq!(
        none.err(
            "budget.export_summary",
            json!({ "format": "pdf", "path": dest(&none, "b.pdf") })
        ),
        "export.nothing_to_export"
    );
}

// ================================================================== permissions

#[test]
fn only_roles_with_export_capability_can_export() {
    let env = production_env();
    seed_story(&env);
    seed_schedule(&env);
    let cases = [
        ("story.export_board", json!({ "format": "pdf" })),
        ("breakdown.export_report", json!({ "format": "pdf" })),
        ("schedule.export_schedule", json!({ "format": "csv" })),
        (
            "reports.export_report",
            json!({ "format": "csv", "reportType": "scene" }),
        ),
    ];
    for role in [Role::Viewer, Role::Commenter] {
        let who = env.actor_with_role(role);
        for (op, args) in &cases {
            let mut a = args.clone();
            let ext = a["format"].as_str().unwrap().to_string();
            a["path"] = json!(dest(&env, &format!("denied-{op}.{ext}")));
            let e = env.call_as(&who, op, a).unwrap_err();
            assert_eq!(e.code.0, "permission.denied", "{role:?} {op}");
        }
    }
    assert!(
        files_in(&out_dir(&env)).is_empty(),
        "nothing written for denied roles"
    );
    let export_only = env.actor_with_role(Role::ExportOnly);
    for (op, args) in &cases {
        let mut a = args.clone();
        let ext = a["format"].as_str().unwrap().to_string();
        a["path"] = json!(dest(&env, &format!("ok-{op}.{ext}")));
        env.call_as(&export_only, op, a)
            .unwrap_or_else(|e| panic!("{op}: {}", e.message));
    }
    // Sides go through the live preview; Export-only can read it.
    let day = env.ok("schedule.get", json!({}))["days"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    env.call_as(
        &export_only,
        "sides.export_pdf",
        json!({ "format": "pdf", "path": dest(&env, "ok-sides.pdf"), "dayId": day }),
    )
    .unwrap();
}

// ================================================================== errors & safety

#[test]
fn text_the_pdf_font_cannot_print_is_a_human_error_and_writes_nothing() {
    let env = production_env();
    exec(
        &env,
        "INSERT INTO story_act(id,title,position,created_at,updated_at) VALUES ('a1','Act One',1,1,1);
         INSERT INTO story_scene_card(id,parent_type,parent_id,short_description,position,created_at,updated_at)
            VALUES ('c1','act','a1','मीरा का सपना',1,1,1);",
    );
    let pdf = dest(&env, "hindi.pdf");
    let e = env
        .call(
            "story.export_board",
            json!({ "format": "pdf", "path": pdf }),
        )
        .unwrap_err();
    assert_eq!(e.code.0, "export.pdf_unsupported_characters");
    assert!(
        e.message.contains("can't print") && e.message.contains("No PDF was written"),
        "{}",
        e.message
    );
    assert!(
        e.message.contains("“म”"),
        "names the characters: {}",
        e.message
    );
    assert!(!e.message.contains('?'), "never a silent '?'");
    assert!(
        files_in(&out_dir(&env)).is_empty(),
        "no file and no partial file"
    );
    // Formats that keep Unicode still work.
    let csv = dest(&env, "hindi.csv");
    env.ok(
        "story.export_board",
        json!({ "format": "csv", "path": csv }),
    );
    assert!(csv_text(&csv).contains("मीरा का सपना"));

    // Sides use the screenplay font and report the same way.
    seed_schedule(&env);
    exec(
        &env,
        "UPDATE screenplay_element SET text = 'मीरा' WHERE id = 'e2'",
    );
    let day = env.ok("schedule.get", json!({}))["days"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        env.err(
            "sides.export_pdf",
            json!({ "format": "pdf", "path": dest(&env, "s.pdf"), "dayId": day })
        ),
        "export.pdf_unsupported_characters"
    );
    assert!(!out_dir(&env).join("s.pdf").exists());
}

#[test]
fn destination_must_be_a_real_location_outside_the_project() {
    let env = production_env();
    assert_eq!(
        env.err(
            "breakdown.export_report",
            json!({ "format": "pdf", "path": "relative.pdf" })
        ),
        "validation.invalid_input"
    );
    let missing = env.dir.path().join("no-such-folder").join("x.pdf");
    assert_eq!(
        env.err(
            "breakdown.export_report",
            json!({ "format": "pdf", "path": missing.to_string_lossy() })
        ),
        "export.folder_missing"
    );
    let inside = PathBuf::from(env.project_path()).join("report.pdf");
    assert_eq!(
        env.err(
            "breakdown.export_report",
            json!({ "format": "pdf", "path": inside.to_string_lossy() })
        ),
        "export.inside_project"
    );
    // The format's extension is added when missing.
    let r = env.ok(
        "breakdown.export_report",
        json!({ "format": "xlsx", "path": dest(&env, "Black Rain - Breakdown") }),
    );
    assert!(r["fileName"].as_str().unwrap().ends_with(".xlsx"));
    assert_eq!(
        env.err(
            "breakdown.export_report",
            json!({ "format": "docx", "path": dest(&env, "x.docx") })
        ),
        "validation.invalid_input"
    );
}

#[cfg(windows)]
#[test]
fn a_failed_write_leaves_the_previous_file_and_no_partial_file() {
    let env = production_env();
    let path = dest(&env, "locked.pdf");
    std::fs::write(&path, b"old").unwrap();
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&path, perms.clone()).unwrap();
    let e = env
        .call(
            "breakdown.export_report",
            json!({ "format": "pdf", "path": path }),
        )
        .unwrap_err();
    assert!(
        e.code.0.starts_with("storage.") || e.code.0.starts_with("export."),
        "{}",
        e.code.0
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"old");
    assert_eq!(
        files_in(&out_dir(&env)),
        vec!["locked.pdf".to_string()],
        "no partial file left behind"
    );
    #[allow(clippy::permissions_set_readonly_false)]
    perms.set_readonly(false);
    std::fs::set_permissions(&path, perms).unwrap();
}

#[test]
fn each_export_records_one_activity_entry() {
    let env = production_env();
    seed_schedule(&env);
    env.ok(
        "schedule.export_schedule",
        json!({ "format": "pdf", "path": dest(&env, "a.pdf") }),
    );
    let summary = query_one(
        &env,
        "SELECT summary FROM sys_activity WHERE action = 'schedule.export_schedule'",
    );
    assert_eq!(summary, "Exported Shooting Schedule as PDF");
    let activity = env.ok("history.activity", json!({}));
    assert!(
        activity
            .to_string()
            .contains("Exported Shooting Schedule as PDF")
    );
    // Nothing is undoable about an export.
    let undo = env.ok("history.info", json!({}));
    assert_ne!(
        undo["label"].as_str().unwrap_or_default(),
        "Exported Shooting Schedule as PDF"
    );
}

#[test]
fn every_export_op_is_registered_as_a_read_only_query() {
    let env = TestEnv::new();
    for op in [
        "story.export_board",
        "breakdown.export_report",
        "moodboard.export_pdf",
        "storyboard.export_sheet",
        "shot.export_list",
        "schedule.export_schedule",
        "callsheets.export_pdf",
        "sides.export_pdf",
        "reports.export_report",
        "budget.export_summary",
    ] {
        let entry = env
            .core
            .registry
            .get(op)
            .unwrap_or_else(|| panic!("{op} registered"));
        assert_eq!(
            entry.kind,
            openframe_application::OpKind::Query,
            "{op} never mutates canonical data"
        );
    }
}
