//! Shooting Schedule, Daily View, Call Sheets, Sides, Reports and Budget —
//! end-to-end through the operation registry (FSD §35–39, §56–58, §104–105,
//! §109–111, §125–126; FSD-SCH-*, FSD-CALL-*).
//!
//! Screenplay/production hub rows are owned by other modules; tests seed them
//! directly as fixtures.

use openframe_domain::Role;
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

// ------------------------------------------------------------------ fixtures

fn exec(env: &TestEnv, sql: &str) {
    let s = env.core.project().expect("project open");
    s.store
        .with_writer(|c| {
            c.execute_batch(sql).expect("fixture sql");
            Ok(())
        })
        .unwrap();
}

/// A locked draft with 4 scenes (+1 omitted), two characters, two locations and
/// confirmed breakdown elements, selected as the active Production Source.
fn seed(env: &TestEnv) {
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
         INSERT INTO breakdown_element(id,source_id,scene_id,scene_lineage_id,category,catalog_item_id,display_name,confirmation_state,created_at,updated_at) VALUES
            ('b1','ps1','s1','L1','Cast','ci_arjun','Arjun','Confirmed',1,1),
            ('b2','ps1','s1','L1','Cast','ci_meera','Meera','Confirmed',1,1),
            ('b3','ps1','s1','L1','Location / Set','ci_police','Police Station','Confirmed',1,1),
            ('b4','ps1','s1','L1','Props','ci_folder','Red Folder','Manual',1,1),
            ('b5','ps1','s2','L2','Cast','ci_arjun','Arjun','Confirmed',1,1),
            ('b6','ps1','s2','L2','Location / Set','ci_rail','Old Railway Station','Confirmed',1,1),
            ('b7','ps1','s3','L3','Cast','ci_meera','Meera','Confirmed',1,1),
            ('b8','ps1','s4','L4','Cast','ci_arjun','Arjun','Confirmed',1,1),
            ('b9','ps1','s4','L4','Location / Set','ci_rail','Old Railway Station','Confirmed',1,1),
            ('b10','ps1','s3','L3','Props','ci_folder','Sheet','Suggested',1,1);",
    );
}

fn env_with_schedule() -> (TestEnv, String) {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    seed(&env);
    let s = env.ok("schedule.create", json!({}));
    let id = s["id"].as_str().unwrap().to_string();
    (env, id)
}

fn view(env: &TestEnv) -> Value {
    env.ok("schedule.get", json!({}))
}

/// Strip id for a displayed scene number (anywhere on the board).
fn strip(env: &TestEnv, number: i64) -> String {
    let v = view(env);
    let mut all: Vec<Value> = v["unscheduled"].as_array().unwrap().clone();
    for d in v["days"].as_array().unwrap() {
        for it in d["items"].as_array().unwrap() {
            if it["kind"] == "strip" {
                all.push(it["strip"].clone());
            }
        }
    }
    all.iter()
        .find(|s| s["number"] == number)
        .unwrap_or_else(|| panic!("scene {number} not on board"))["id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn day<'a>(v: &'a Value, id: &str) -> &'a Value {
    v["days"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == id)
        .expect("day")
}

fn day_numbers(v: &Value, id: &str) -> Vec<i64> {
    day(v, id)["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["kind"] == "strip")
        .map(|i| i["strip"]["number"].as_i64().unwrap())
        .collect()
}

fn new_day(env: &TestEnv, sched: &str, date: Option<&str>) -> String {
    env.ok(
        "schedule.create_day",
        json!({ "scheduleId": sched, "date": date }),
    )
    .as_str()
    .unwrap()
    .to_string()
}

fn move_to(env: &TestEnv, number: i64, day: Option<&str>) {
    env.ok(
        "schedule.move_strip",
        json!({ "stripId": strip(env, number), "dayId": day }),
    );
}

// ------------------------------------------------------------------ schedule

#[test]
fn schedule_needs_a_production_source_first() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let v = view(&env);
    assert!(v["activeSource"].is_null());
    assert!(v["schedule"].is_null());
    assert_eq!(
        env.err("schedule.create", json!({})),
        "validation.no_source"
    );
}

#[test]
fn fsd_sch_001_all_scenes_enter_unscheduled() {
    let (env, _) = env_with_schedule();
    let v = view(&env);
    assert_eq!(
        v["schedule"]["source"]["label"],
        "Shooting Draft 6 (Locked)"
    );
    let un = v["unscheduled"].as_array().unwrap();
    assert_eq!(
        un.len(),
        4,
        "every (non-omitted) scene starts in Unscheduled"
    );
    assert_eq!(
        un.iter()
            .map(|s| s["number"].as_i64().unwrap())
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4]
    );
    assert_eq!(v["counts"]["scheduled"], 0);
    assert!(v["days"].as_array().unwrap().is_empty());
    // Strip anatomy derived from the screenplay + breakdown.
    let s1 = &un[0];
    assert_eq!(s1["ieLabel"], "INT·N");
    assert_eq!(s1["stripClass"], "in");
    assert_eq!(s1["locationName"], "Police Station");
    assert_eq!(s1["cast"].as_array().unwrap().len(), 2);
    assert_eq!(s1["cast"][0]["initials"], "AR");
    assert_eq!(un[1]["stripClass"], "xd");
    assert_eq!(un[3]["stripClass"], "xn");
    assert!(s1["pageEighths"].as_i64().unwrap() >= 1);
    assert_eq!(
        un[1]["synopsis"], "ARJUN walks the empty platform, photograph in hand.",
        "synopsis falls back to action"
    );
    // Only one schedule per project.
    assert_eq!(env.err("schedule.create", json!({})), "conflict.state");
    // Creation is undoable.
    env.undo();
    assert!(view(&env)["schedule"].is_null());
}

#[test]
fn fsd_sch_002_drag_into_day_is_undoable() {
    let (env, sched) = env_with_schedule();
    let d1 = new_day(&env, &sched, Some("2027-06-14"));
    let v = view(&env);
    assert_eq!(day(&v, &d1)["title"], "Shoot Day 1 — Mon 14 Jun 2027");
    assert_eq!(v["schedule"]["status"], "Active");

    let s1 = strip(&env, 1);
    env.ok("schedule.move_strip", json!({ "stripId": s1, "dayId": d1 }));
    let v = view(&env);
    assert_eq!(day_numbers(&v, &d1), vec![1]);
    assert_eq!(v["unscheduled"].as_array().unwrap().len(), 3);
    let sum = &day(&v, &d1)["summary"];
    assert_eq!(sum["sceneCount"], 1);
    assert_eq!(sum["cast"].as_array().unwrap().len(), 2);
    assert_eq!(sum["locations"][0]["name"], "Police Station");
    assert_eq!(sum["missingEstimates"], 1, "durations are never invented");

    let step = env.undo();
    assert!(
        step["label"]
            .as_str()
            .unwrap()
            .starts_with("Scheduled Scene 1 on Shoot Day 1")
    );
    let v = view(&env);
    assert!(day_numbers(&v, &d1).is_empty());
    assert_eq!(
        v["unscheduled"].as_array().unwrap().len(),
        4,
        "undo returns the scene to Unscheduled"
    );
    env.redo();
    assert_eq!(day_numbers(&view(&env), &d1), vec![1]);
}

#[test]
fn reorder_within_day_move_between_days_and_unschedule() {
    let (env, sched) = env_with_schedule();
    let d1 = new_day(&env, &sched, None);
    let d2 = new_day(&env, &sched, None);
    for n in [1, 2, 3] {
        move_to(&env, n, Some(&d1));
    }
    assert_eq!(day_numbers(&view(&env), &d1), vec![1, 2, 3]);
    // Reorder within the day = production order only.
    env.ok(
        "schedule.move_strip",
        json!({ "stripId": strip(&env, 3), "dayId": d1, "index": 0 }),
    );
    assert_eq!(day_numbers(&view(&env), &d1), vec![3, 1, 2]);
    // Move between days keeps identity.
    let id2 = strip(&env, 2);
    env.ok(
        "schedule.move_strip",
        json!({ "stripId": id2, "dayId": d2 }),
    );
    let v = view(&env);
    assert_eq!(day_numbers(&v, &d1), vec![3, 1]);
    assert_eq!(day_numbers(&v, &d2), vec![2]);
    assert_eq!(strip(&env, 2), id2);
    // Remove from day → back to Unscheduled (not deleted).
    env.ok(
        "schedule.move_strip",
        json!({ "stripId": id2, "dayId": null }),
    );
    let v = view(&env);
    assert!(day_numbers(&v, &d2).is_empty());
    assert!(
        v["unscheduled"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["id"] == id2)
    );
    assert_eq!(v["counts"]["total"], 4);
}

#[test]
fn markers_days_duplicate_off_day_and_delete_restore() {
    let (env, sched) = env_with_schedule();
    let d1 = new_day(&env, &sched, Some("2027-06-13"));
    move_to(&env, 1, Some(&d1));
    move_to(&env, 3, Some(&d1));
    let meal = env.ok(
        "schedule.add_marker",
        json!({ "dayId": d1, "markerType": "Meal", "atTime": "20:30", "durationMinutes": 45, "index": 1 }),
    );
    assert_eq!(
        env.err(
            "schedule.add_marker",
            json!({ "dayId": d1, "markerType": "Custom" })
        ),
        "validation.required"
    );
    assert_eq!(
        env.err(
            "schedule.add_marker",
            json!({ "dayId": d1, "markerType": "Lunch" })
        ),
        "validation.invalid_input"
    );
    let v = view(&env);
    let items = day(&v, &d1)["items"].as_array().unwrap();
    assert_eq!(
        items
            .iter()
            .map(|i| i["kind"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["strip", "marker", "strip"]
    );
    assert_eq!(items[1]["marker"]["label"], "Meal Break");
    assert_eq!(items[1]["marker"]["atTime"], "20:30");
    assert_eq!(
        day(&v, &d1)["summary"]["estimatedMinutes"],
        45,
        "break durations count toward the day"
    );

    // Duplicate day: shell + markers only, scenes never duplicated.
    let dup = env
        .ok("schedule.duplicate_day", json!({ "dayId": d1 }))
        .as_str()
        .unwrap()
        .to_string();
    let v = view(&env);
    let dd = day(&v, &dup);
    assert_eq!(dd["label"], "Shoot Day 2");
    assert!(dd["date"].is_null());
    assert_eq!(dd["items"].as_array().unwrap().len(), 1);
    assert_eq!(dd["items"][0]["kind"], "marker");
    assert_eq!(v["counts"]["total"], 4);

    // Off day: no scenes allowed, not numbered.
    assert_eq!(
        env.err(
            "schedule.set_off_day",
            json!({ "dayId": d1, "offDay": true })
        ),
        "validation.off_day_has_scenes"
    );
    env.ok(
        "schedule.set_off_day",
        json!({ "dayId": dup, "offDay": true }),
    );
    let v = view(&env);
    assert_eq!(day(&v, &dup)["label"], "Off Day");
    assert_eq!(
        env.err(
            "schedule.move_strip",
            json!({ "stripId": strip(&env, 2), "dayId": dup })
        ),
        "validation.off_day"
    );
    let off = new_day(&env, &sched, None);
    assert_eq!(day(&view(&env), &off)["label"], "Shoot Day 2");

    // Reschedule date.
    env.ok(
        "schedule.set_day_date",
        json!({ "dayId": d1, "date": "2027-06-20" }),
    );
    assert_eq!(day(&view(&env), &d1)["dateLabel"], "Sun 20 Jun 2027");
    assert_eq!(
        env.err(
            "schedule.set_day_date",
            json!({ "dayId": d1, "date": "2027-02-30" })
        ),
        "validation.date"
    );

    // Delete a day → its scenes return to Unscheduled; restore brings them back.
    env.ok("schedule.delete_day", json!({ "dayId": d1 }));
    let v = view(&env);
    assert_eq!(v["unscheduled"].as_array().unwrap().len(), 4);
    let trash = env.ok("trash.list", json!({}));
    let entry = trash
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["objectType"] == "shooting_day")
        .unwrap()
        .clone();
    env.ok("trash.restore", json!({ "id": entry["id"] }));
    assert_eq!(day_numbers(&view(&env), &d1), vec![1, 3]);

    // Marker delete + restore.
    env.ok("schedule.delete_marker", json!({ "markerId": meal }));
    assert_eq!(day(&view(&env), &d1)["items"].as_array().unwrap().len(), 2);
    let trash = env.ok("trash.list", json!({}));
    let entry = trash
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["objectType"] == "schedule_marker")
        .unwrap()
        .clone();
    env.ok("trash.restore", json!({ "id": entry["id"] }));
    assert_eq!(day(&view(&env), &d1)["items"].as_array().unwrap().len(), 3);
}

#[test]
fn fsd_sch_003_warnings_are_advisory_with_keep_anyway() {
    let (env, sched) = env_with_schedule();
    let d1 = new_day(&env, &sched, Some("2027-06-14"));
    // Arjun: scene 1 (Police Station) + scene 2 (Old Railway Station) on the same day.
    move_to(&env, 1, Some(&d1));
    move_to(&env, 2, Some(&d1));
    let v = view(&env);
    let w = v["warnings"].as_array().unwrap();
    let actor = w
        .iter()
        .find(|w| w["kind"] == "actor_conflict")
        .expect("actor conflict");
    assert!(
        actor["message"]
            .as_str()
            .unwrap()
            .contains("Karthik Menon (Arjun) is needed at two locations on Shoot Day 1")
    );
    assert_eq!(actor["blocking"], false, "warnings never block by default");
    // Scene 2's location has no address → missing location information.
    assert!(w.iter().any(|w| w["kind"] == "missing_location"));

    // Duration overflow against the day target.
    env.ok(
        "schedule.set_strip_estimate",
        json!({ "stripId": strip(&env, 1), "minutes": 400 }),
    );
    env.ok(
        "schedule.set_strip_estimate",
        json!({ "stripId": strip(&env, 2), "minutes": 260 }),
    );
    let v = view(&env);
    let over = v["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["kind"] == "duration_overflow")
        .unwrap()
        .clone();
    assert!(over["message"].as_str().unwrap().contains("(11h of 10h)"));
    assert_eq!(day(&v, &d1)["summary"]["overTarget"], true);

    // Keep anyway: acknowledged, not removed.
    env.ok(
        "schedule.decide_warning",
        json!({ "scheduleId": sched, "key": actor["key"], "decision": "Kept" }),
    );
    let v = view(&env);
    let kept = v["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["key"] == actor["key"])
        .unwrap()
        .clone();
    assert_eq!(kept["decision"], "Kept");
    assert_eq!(
        v["openWarningCount"].as_i64().unwrap(),
        v["warnings"].as_array().unwrap().len() as i64 - 1
    );
    // The system never moved anything.
    assert_eq!(day_numbers(&v, &d1), vec![1, 2]);

    // Strict validation (explicit setting) blocks a move that creates a new warning…
    env.ok(
        "schedule.update_settings",
        json!({ "scheduleId": sched, "name": "Shooting Schedule", "strictValidation": true, "dayDurationMinutes": 600 }),
    );
    let d2 = new_day(&env, &sched, Some("2027-06-15"));
    // Scene 3 (Morgue) has no confirmed location → a new missing-location warning.
    let code = env.err(
        "schedule.move_strip",
        json!({ "stripId": strip(&env, 3), "dayId": d2 }),
    );
    assert_eq!(code, "validation.schedule_warning");
    assert_eq!(
        day_numbers(&view(&env), &d2),
        Vec::<i64>::new(),
        "strict validation refused the move"
    );
}

#[test]
fn strict_validation_offers_keep_anyway() {
    let (env, sched) = env_with_schedule();
    env.ok(
        "schedule.update_settings",
        json!({ "scheduleId": sched, "name": "Main unit", "strictValidation": true, "dayDurationMinutes": 600 }),
    );
    let d1 = new_day(&env, &sched, None);
    move_to(&env, 1, Some(&d1));
    let code = env.err(
        "schedule.move_strip",
        json!({ "stripId": strip(&env, 2), "dayId": d1 }),
    );
    assert_eq!(code, "validation.schedule_warning");
    assert_eq!(day_numbers(&view(&env), &d1), vec![1], "nothing changed");
    env.ok(
        "schedule.move_strip",
        json!({ "stripId": strip(&env, 2), "dayId": d1, "keepAnyway": true }),
    );
    let v = view(&env);
    assert_eq!(day_numbers(&v, &d1), vec![1, 2]);
    let actor = v["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["kind"] == "actor_conflict")
        .unwrap()
        .clone();
    assert_eq!(actor["decision"], "Kept");
    assert_eq!(actor["blocking"], false);
}

#[test]
fn fsd_sched_014_grouping_suggestions_need_explicit_confirmation() {
    let (env, sched) = env_with_schedule();
    let d1 = new_day(&env, &sched, None);
    move_to(&env, 2, Some(&d1));
    let before = view(&env);
    let sg = env.ok("schedule.suggestions", json!({}));
    let list = sg["suggestions"].as_array().unwrap();
    let rail = list
        .iter()
        .find(|s| s["kind"] == "location")
        .expect("location suggestion");
    assert_eq!(rail["title"], "Scenes 2 and 4 all use the same location.");
    assert_eq!(rail["subject"], "Old Railway Station · confirmed");
    assert_eq!(rail["suggestedDayId"], d1);
    assert_eq!(rail["stripsToMove"].as_array().unwrap().len(), 1);
    // Asking for suggestions changes nothing.
    assert_eq!(view(&env)["days"], before["days"]);
    // Deterministic.
    assert_eq!(env.ok("schedule.suggestions", json!({})), sg);
    // Applying is an explicit, undoable user action.
    env.ok(
        "schedule.move_strips",
        json!({ "stripIds": rail["stripsToMove"], "dayId": d1 }),
    );
    assert_eq!(day_numbers(&view(&env), &d1), vec![2, 4]);
    env.undo();
    assert_eq!(day_numbers(&view(&env), &d1), vec![2]);
}

#[test]
fn schedule_finalized_blocks_edits_until_reopened() {
    let (env, sched) = env_with_schedule();
    let d1 = new_day(&env, &sched, None);
    env.ok(
        "schedule.set_status",
        json!({ "scheduleId": sched, "status": "Finalized" }),
    );
    assert_eq!(
        env.err(
            "schedule.move_strip",
            json!({ "stripId": strip(&env, 1), "dayId": d1 })
        ),
        "conflict.schedule_finalized"
    );
    let editor = env.other_user(Role::Editor);
    env.call_as(
        &editor,
        "schedule.set_status",
        json!({ "scheduleId": sched, "status": "Active" }),
    )
    .unwrap();
    move_to(&env, 1, Some(&d1));
}

// ------------------------------------------------------------------ call sheets

fn call_sheet_env() -> (TestEnv, String, String, String) {
    let (env, sched) = env_with_schedule();
    let d1 = new_day(&env, &sched, Some("2027-06-14"));
    let d2 = new_day(&env, &sched, Some("2027-06-15"));
    move_to(&env, 1, Some(&d1));
    move_to(&env, 3, Some(&d1));
    env.ok(
        "schedule.add_marker",
        json!({ "dayId": d1, "markerType": "Meal", "atTime": "20:30" }),
    );
    env.ok(
        "schedule.set_day_notes",
        json!({ "dayId": d1, "notes": "Night shoot inside — blackout the windows." }),
    );
    let cs = env
        .ok("callsheets.create", json!({ "dayId": d1 }))
        .as_str()
        .unwrap()
        .to_string();
    (env, d1, d2, cs)
}

#[test]
fn fsd_call_001_call_sheet_generated_from_shooting_day() {
    let (env, d1, _, cs) = call_sheet_env();
    let s = env.ok("callsheets.get", json!({ "id": cs }));
    assert_eq!(s["title"], "Call Sheet — Day 1");
    assert_eq!(s["status"], "Draft");
    assert_eq!(s["dayId"], d1);
    assert_eq!(s["dateLong"], "MONDAY 14 JUNE 2027");
    let doc = &s["document"];
    assert_eq!(doc["title"], "Black Rain");
    assert_eq!(doc["dayLabel"], "SHOOT DAY 1");
    assert_eq!(
        doc["scenes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x["number"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["1", "3"]
    );
    assert_eq!(doc["scenes"][0]["heading"], "INT. POLICE STATION - NIGHT");
    let cast = doc["cast"].as_array().unwrap();
    assert_eq!(cast[0]["actor"], "Karthik Menon");
    assert_eq!(cast[0]["character"], "ARJUN");
    assert_eq!(cast[1]["character"], "MEERA");
    assert_eq!(
        cast[1]["actor"], "",
        "missing values are blank placeholders, never invented"
    );
    assert_eq!(doc["locations"][0]["name"], "Police Station");
    assert_eq!(doc["locations"][0]["address"], "District Rd, Gudur");
    assert_eq!(doc["practical"]["mealBreak"], "Meal Break 20:30");
    assert_eq!(
        doc["dayNotes"],
        "Night shoot inside — blackout the windows."
    );
    assert!(
        doc["optional"]["weather"].is_null(),
        "optional sections hidden until added"
    );
    let missing: Vec<&str> = s["missing"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    assert!(missing.contains(&"crewCall"));
    assert!(missing.contains(&"cast.1.actor"));
    // One current working document per day.
    assert_eq!(
        env.err("callsheets.create", json!({ "dayId": d1 })),
        "conflict.call_sheet_exists"
    );
    // The day shows its call sheet status.
    assert_eq!(day(&view(&env), &d1)["callSheet"]["status"], "Draft");
    let list = env.ok("callsheets.list", json!({}));
    assert_eq!(list[0]["id"], cs);
    assert_eq!(list[0]["dateLabel"], "Mon 14 Jun 2027");
}

#[test]
fn fsd_call_002_call_sheet_edits_never_change_the_schedule() {
    let (env, _, _, cs) = call_sheet_env();
    let before = view(&env);
    let mut doc = env.ok("callsheets.get", json!({ "id": cs }))["document"].clone();
    doc["crewCall"] = json!("18:00");
    doc["cast"][0]["callTime"] = json!("18:15");
    doc["scenes"].as_array_mut().unwrap().remove(1);
    doc["dayNotes"] = json!("Changed only on the call sheet");
    doc["optional"]["weather"] = json!("Rain expected after 22:00");
    env.ok("callsheets.update", json!({ "id": cs, "document": doc }));
    let after = view(&env);
    assert_eq!(before["days"][0]["items"], after["days"][0]["items"]);
    assert_eq!(before["days"][0]["notes"], after["days"][0]["notes"]);
    assert_eq!(before["unscheduled"], after["unscheduled"]);
    let s = env.ok("callsheets.get", json!({ "id": cs }));
    assert_eq!(s["document"]["crewCall"], "18:00");
    assert_eq!(
        s["document"]["optional"]["weather"],
        "Rain expected after 22:00"
    );
    assert_eq!(
        s["status"], "Draft",
        "call-sheet-only edits don't make it stale"
    );
    // Unknown document fields are rejected.
    let mut bad = doc.clone();
    bad["surprise"] = json!(1);
    assert_eq!(
        env.err("callsheets.update", json!({ "id": cs, "document": bad })),
        "validation.invalid_input"
    );
}

#[test]
fn fsd_call_009_schedule_change_marks_call_sheet_needs_refresh() {
    let (env, d1, d2, cs) = call_sheet_env();
    let mut doc = env.ok("callsheets.get", json!({ "id": cs }))["document"].clone();
    doc["cast"][0]["callTime"] = json!("18:15");
    env.ok("callsheets.update", json!({ "id": cs, "document": doc }));

    // Moving a scene out of the day marks its call sheet stale.
    env.ok(
        "schedule.move_strip",
        json!({ "stripId": strip(&env, 3), "dayId": d2 }),
    );
    let s = env.ok("callsheets.get", json!({ "id": cs }));
    assert_eq!(s["status"], "Needs Refresh");
    assert_eq!(s["stale"]["areas"], json!(["Scenes", "Locations"]));
    assert_eq!(
        s["document"]["scenes"].as_array().unwrap().len(),
        2,
        "never silently rewritten"
    );
    let v = view(&env);
    assert_eq!(day(&v, &d1)["callSheet"]["status"], "Needs Refresh");
    assert!(
        v["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["kind"] == "call_sheet_stale")
    );
    assert_eq!(
        env.ok("callsheets.list", json!({}))[0]["status"],
        "Needs Refresh"
    );

    // Undo the move → no longer stale.
    env.undo();
    assert_eq!(
        env.ok("callsheets.get", json!({ "id": cs }))["status"],
        "Draft"
    );
    env.redo();

    // Refresh preview shows differences; nothing applied until refresh.
    let p = env.ok("callsheets.refresh_preview", json!({ "id": cs }));
    assert_eq!(p["hasChanges"], true);
    let scenes = p["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["area"] == "Scenes")
        .unwrap()
        .clone();
    assert_eq!(scenes["changed"], true);
    assert!(scenes["now"].as_str().unwrap().contains("INT. MORGUE"));
    assert!(!scenes["after"].as_str().unwrap().contains("INT. MORGUE"));
    assert_eq!(
        env.ok("callsheets.get", json!({ "id": cs }))["status"],
        "Needs Refresh"
    );
    env.ok("callsheets.refresh", json!({ "id": cs }));
    let s = env.ok("callsheets.get", json!({ "id": cs }));
    assert_eq!(s["status"], "Draft");
    assert_eq!(s["document"]["scenes"].as_array().unwrap().len(), 1);
    assert_eq!(
        s["document"]["cast"][0]["callTime"], "18:15",
        "edited call times are kept where they still apply"
    );

    // Rescheduling the date also marks it stale (FSD §37.5).
    env.ok(
        "schedule.set_day_date",
        json!({ "dayId": d1, "date": "2027-06-21" }),
    );
    let s = env.ok("callsheets.get", json!({ "id": cs }));
    assert_eq!(s["status"], "Needs Refresh");
    assert_eq!(s["stale"]["areas"], json!(["Date"]));
}

#[test]
fn fsd_call_011_finalized_call_sheet_is_immutable_and_revised() {
    let (env, d1, d2, cs) = call_sheet_env();
    assert_eq!(
        env.err("callsheets.finalize", json!({ "id": cs })),
        "validation.crew_call"
    );
    let mut doc = env.ok("callsheets.get", json!({ "id": cs }))["document"].clone();
    doc["crewCall"] = json!("18:00");
    env.ok(
        "callsheets.update",
        json!({ "id": cs, "document": doc.clone() }),
    );
    // Viewer / Commenter cannot finalize.
    let viewer = env.actor_with_role(Role::Viewer);
    assert_eq!(
        env.call_as(&viewer, "callsheets.finalize", json!({ "id": cs }))
            .unwrap_err()
            .code
            .0,
        "permission.denied"
    );
    let snap = env.ok("callsheets.finalize", json!({ "id": cs }));
    assert!(snap.as_str().is_some());
    let s = env.ok("callsheets.get", json!({ "id": cs }));
    assert_eq!(s["status"], "Final");
    assert_eq!(s["editable"], false);
    assert_eq!(s["snapshotId"], snap);
    let frozen = s["document"].clone();

    // Immutable: edits are refused.
    doc["crewCall"] = json!("07:00");
    assert_eq!(
        env.err("callsheets.update", json!({ "id": cs, "document": doc })),
        "conflict.call_sheet_final"
    );
    assert_eq!(
        env.err("callsheets.refresh", json!({ "id": cs })),
        "conflict.call_sheet_final"
    );

    // Issue.
    env.ok("callsheets.issue", json!({ "id": cs }));
    assert_eq!(
        env.ok("callsheets.get", json!({ "id": cs }))["status"],
        "Issued"
    );

    // A later schedule change leaves the issued document historical.
    env.ok(
        "schedule.move_strip",
        json!({ "stripId": strip(&env, 3), "dayId": d2 }),
    );
    let s = env.ok("callsheets.get", json!({ "id": cs }));
    assert_eq!(s["status"], "Issued");
    assert_eq!(s["sourceChanged"], true);
    assert_eq!(s["document"], frozen);

    // Refresh + reissue = a new revision; the old one is Superseded, unchanged.
    let v2 = env
        .ok("callsheets.new_revision", json!({ "id": cs }))
        .as_str()
        .unwrap()
        .to_string();
    let n = env.ok("callsheets.get", json!({ "id": v2 }));
    assert_eq!(n["revision"], 2);
    assert_eq!(n["title"], "Call Sheet — Day 1 (v2)");
    assert_eq!(n["status"], "Draft");
    assert_eq!(n["document"]["crewCall"], "18:00");
    assert_eq!(n["document"]["scenes"].as_array().unwrap().len(), 1);
    assert_eq!(n["previousId"], cs);
    let old = env.ok("callsheets.get", json!({ "id": cs }));
    assert_eq!(old["status"], "Superseded");
    assert_eq!(old["document"], frozen);
    assert_eq!(old["revisions"].as_array().unwrap().len(), 2);
    assert_eq!(day(&view(&env), &d1)["callSheet"]["id"], v2);
}

#[test]
fn stale_call_sheet_needs_explicit_choice_to_finalize() {
    let (env, _, d2, cs) = call_sheet_env();
    let mut doc = env.ok("callsheets.get", json!({ "id": cs }))["document"].clone();
    doc["crewCall"] = json!("18:00");
    env.ok("callsheets.update", json!({ "id": cs, "document": doc }));
    env.ok(
        "schedule.move_strip",
        json!({ "stripId": strip(&env, 3), "dayId": d2 }),
    );
    assert_eq!(
        env.err("callsheets.finalize", json!({ "id": cs })),
        "validation.call_sheet_stale"
    );
    env.ok(
        "callsheets.finalize",
        json!({ "id": cs, "acknowledgeStale": true }),
    );
}

#[test]
fn call_sheet_permissions_delete_and_search() {
    let (env, d1, _, cs) = call_sheet_env();
    let commenter = env.actor_with_role(Role::Commenter);
    let viewer = env.actor_with_role(Role::Viewer);
    assert_eq!(
        env.call_as(&commenter, "callsheets.create", json!({ "dayId": d1 }))
            .unwrap_err()
            .code
            .0,
        "permission.denied"
    );
    assert_eq!(
        env.call_as(
            &viewer,
            "schedule.move_strip",
            json!({ "stripId": strip(&env, 2), "dayId": d1 })
        )
        .unwrap_err()
        .code
        .0,
        "permission.denied"
    );
    assert!(env.call_as(&viewer, "schedule.get", json!({})).is_ok());
    assert!(
        env.call_as(&viewer, "callsheets.get", json!({ "id": cs }))
            .is_ok()
    );

    let hits = env.ok("search.query", json!({ "text": "Karthik" }));
    assert!(
        hits.as_array().unwrap().iter().any(|h| h["entityId"] == cs),
        "call sheets are searchable"
    );
    let hits = env.ok("search.query", json!({ "text": "blackout" }));
    assert!(
        hits.as_array().unwrap().iter().any(|h| h["entityId"] == d1),
        "shooting days are searchable"
    );

    env.ok("callsheets.delete", json!({ "id": cs }));
    assert!(
        env.ok("callsheets.list", json!({}))
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(view(&env)["days"][0]["callSheet"].is_null());
    let trash = env.ok("trash.list", json!({}));
    let entry = trash
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["objectType"] == "call_sheet")
        .unwrap()
        .clone();
    env.ok("trash.restore", json!({ "id": entry["id"] }));
    assert_eq!(
        env.ok("callsheets.list", json!({}))
            .as_array()
            .unwrap()
            .len(),
        1
    );
    env.ok("callsheets.delete", json!({ "id": cs }));
    let trash = env.ok("trash.list", json!({}));
    let entry = trash
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["objectType"] == "call_sheet")
        .unwrap()
        .clone();
    env.ok("trash.purge", json!({ "id": entry["id"] }));
    assert_eq!(
        env.err("callsheets.get", json!({ "id": cs })),
        "not_found.call_sheet"
    );
}

#[test]
fn schedule_and_call_sheets_survive_restart() {
    let (env, d1, _, cs) = call_sheet_env();
    let before = view(&env);
    let path = env.project_path();
    let env = env.restart();
    env.reopen_project(&path);
    let after = view(&env);
    assert_eq!(before["days"], after["days"]);
    assert_eq!(env.ok("callsheets.get", json!({ "id": cs }))["dayId"], d1);
}

// ------------------------------------------------------------------ reconciliation

#[test]
fn fsd_56_script_changes_flag_strips_and_never_delete() {
    let (env, sched) = env_with_schedule();
    let d1 = new_day(&env, &sched, None);
    move_to(&env, 1, Some(&d1));
    move_to(&env, 3, Some(&d1));
    let cs = env
        .ok("callsheets.create", json!({ "dayId": d1 }))
        .as_str()
        .unwrap()
        .to_string();
    // A revision draft: scene 1 heading changed, scene 3 removed, a new scene added.
    exec(
        &env,
        "INSERT INTO screenplay_draft(id,screenplay_id,name,status,created_from_draft_id,created_at,updated_at)
             VALUES ('d2','sp','Revision A','Revision','d1',2,2);
         INSERT INTO screenplay_scene(id,draft_id,lineage_id,position,heading,synopsis,created_at,updated_at) VALUES
            ('t1','d2','L1',1,'INT. POLICE STATION - CORRIDOR - NIGHT','Meera hands the file',2,2),
            ('t2','d2','L2',2,'EXT. OLD RAILWAY STATION - DAY',NULL,2,2),
            ('t4','d2','L4',3,'EXT. OLD RAILWAY STATION - NIGHT','Watcher on platform',2,2),
            ('t5','d2','L5',4,'INT. HOSPITAL - DAY','Maya arrives',2,2);
         INSERT INTO screenplay_element(id,scene_id,position,element_type,text,created_at,updated_at) VALUES
            ('f1','t1',1,'action','Rain lashes the windows. MEERA slides a red folder across the desk.',2,2),
            ('f2','t1',2,'character','MEERA',2,2),
            ('f3','t1',3,'dialogue','You will want to see this before anyone else does.',2,2),
            ('f4','t2',1,'action','ARJUN walks the empty platform, photograph in hand.',2,2),
            ('f6','t4',1,'action','A figure watches from the far platform.',2,2);
         UPDATE production_source SET active = 0 WHERE id = 'ps1';
         INSERT INTO production_source(id,draft_id,active,selected_at,created_at,updated_at) VALUES ('ps2','d2',1,2,2,2);",
    );
    let before = view(&env);
    let ch = &before["scriptChanges"];
    assert_eq!(ch["sourceSwitched"], true);
    assert_eq!(ch["changed"][0]["kinds"], json!(["Heading changed"]));
    assert_eq!(ch["removed"].as_array().unwrap().len(), 1);
    assert_eq!(ch["newScenes"], 1);
    // The schedule has not been changed yet.
    assert_eq!(day_numbers(&before, &d1), vec![1, 3]);

    let r = env.ok("schedule.reconcile", json!({ "scheduleId": sched }));
    assert_eq!(r, json!({ "changed": 1, "added": 1, "removed": 1 }));
    let v = view(&env);
    assert!(v["scriptChanges"].is_null());
    assert_eq!(v["schedule"]["source"]["label"], "Revision A (Revision)");
    let items = day(&v, &d1)["items"].as_array().unwrap();
    assert_eq!(
        items.len(),
        2,
        "assignments stay; the removed scene is not deleted"
    );
    let changed = &items[0]["strip"];
    assert_eq!(changed["sourceState"], "Changed");
    assert_eq!(changed["heading"], "INT. POLICE STATION - CORRIDOR - NIGHT");
    let removed = items[1]["strip"].clone();
    assert_eq!(removed["sourceState"], "Removed");
    assert_eq!(removed["heading"], "INT. MORGUE - DAY");
    assert!(
        v["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["kind"] == "script_removed")
    );
    let new_one = v["unscheduled"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["number"] == 4)
        .unwrap()
        .clone();
    assert_eq!(new_one["sourceState"], "New");
    assert_eq!(new_one["heading"], "INT. HOSPITAL - DAY");
    // The call sheet of the affected day needs refresh.
    assert_eq!(
        env.ok("callsheets.get", json!({ "id": cs }))["status"],
        "Needs Refresh"
    );

    // Review flags; confirm the removal (kept in history).
    env.ok(
        "schedule.acknowledge_change",
        json!({ "stripId": changed["id"] }),
    );
    assert_eq!(
        env.err(
            "schedule.acknowledge_change",
            json!({ "stripId": removed["id"] })
        ),
        "validation.invalid_input"
    );
    env.ok(
        "schedule.confirm_removal",
        json!({ "stripId": removed["id"] }),
    );
    let v = view(&env);
    assert_eq!(day_numbers(&v, &d1), vec![1]);
    assert_eq!(day(&v, &d1)["items"][0]["strip"]["sourceState"], "Current");
    assert_eq!(v["history"][0]["id"], removed["id"]);
    env.undo();
    assert_eq!(day(&view(&env), &d1)["items"].as_array().unwrap().len(), 2);
}

// ------------------------------------------------------------------ daily view

#[test]
fn fsd_prod_022_daily_view_is_derived() {
    let (env, d1, _, cs) = call_sheet_env();
    let mut doc = env.ok("callsheets.get", json!({ "id": cs }))["document"].clone();
    doc["crewCall"] = json!("18:00");
    doc["cast"][0]["callTime"] = json!("18:15");
    env.ok("callsheets.update", json!({ "id": cs, "document": doc }));
    let v = env.ok("schedule.daily", json!({ "dayId": d1 }));
    let d = &v["day"];
    assert_eq!(d["title"], "Shoot Day 1 — Mon 14 Jun 2027");
    assert_eq!(d["scenes"].as_array().unwrap().len(), 2);
    assert_eq!(d["crewCall"], "18:00");
    assert_eq!(d["cast"][0]["callTime"], "18:15");
    assert!(d["cast"][1]["callTime"].is_null());
    assert_eq!(d["callSheet"]["status"], "Draft");
    assert_eq!(d["breakdownItems"][0]["name"], "Red Folder");
    assert_eq!(d["locations"][0]["name"], "Police Station");
    assert_eq!(d["markers"][0]["label"], "Meal Break");
    assert_eq!(v["days"].as_array().unwrap().len(), 2);
    // No schedule → empty state.
    let env2 = TestEnv::with_project("Empty", "Short Film");
    assert_eq!(env2.ok("schedule.daily", json!({}))["hasSchedule"], false);
}

// ------------------------------------------------------------------ sides & reports

#[test]
fn fsd_prod_024_sides_are_script_snapshots() {
    let (env, d1, _, _) = call_sheet_env();
    let p = env.ok(
        "sides.preview",
        json!({ "dayId": d1, "includeCover": true }),
    );
    assert_eq!(
        p["header"],
        "BLACK RAIN · SIDES · SHOOT DAY 1 · Shooting Draft 6 (Locked)"
    );
    assert_eq!(p["scenes"].as_array().unwrap().len(), 2);
    assert_eq!(p["scenes"][0]["elements"][1]["text"], "MEERA");
    assert!(
        p["cover"]["locations"][0]
            .as_str()
            .unwrap()
            .starts_with("Police Station")
    );
    // Selected cast only: Arjun appears in scene 1 only.
    let arjun = env.ok(
        "sides.preview",
        json!({ "dayId": d1, "castKeys": ["char:arjun"] }),
    );
    assert_eq!(arjun["scenes"].as_array().unwrap().len(), 1);
    let id = env
        .ok("sides.create", json!({ "dayId": d1, "includeCover": true }))
        .as_str()
        .unwrap()
        .to_string();
    // Snapshot: later script edits don't change saved sides.
    exec(
        &env,
        "UPDATE screenplay_element SET text = 'CHANGED' WHERE id = 'e2';",
    );
    let saved = env.ok("sides.get", json!({ "id": id }));
    assert_eq!(saved["title"], "Sides — Shoot Day 1");
    assert_eq!(
        saved["content"]["scenes"][0]["elements"][1]["text"],
        "MEERA"
    );
    assert_eq!(env.ok("sides.list", json!({}))[0]["sceneCount"], 2);
    env.ok("sides.delete", json!({ "id": id }));
    assert!(
        env.ok("sides.list", json!({}))
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn fsd_prod_023_reports_are_read_only_projections() {
    let (env, _, _, _) = call_sheet_env();
    let before = view(&env);
    for t in [
        "scene",
        "location",
        "cast_scene",
        "prop",
        "schedule",
        "breakdown_completeness",
    ] {
        let r = env.ok("reports.generate", json!({ "reportType": t }));
        assert!(!r["columns"].as_array().unwrap().is_empty(), "{t}");
    }
    let cast = env.ok("reports.generate", json!({ "reportType": "cast_scene" }));
    let arjun = cast["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r[1] == "Arjun")
        .unwrap()
        .clone();
    assert_eq!(arjun[0], "Karthik Menon");
    assert_eq!(arjun[2], "1, 2, 4");
    let comp = env.ok(
        "reports.generate",
        json!({ "reportType": "breakdown_completeness" }),
    );
    assert_eq!(comp["rows"][2][4], "In progress");
    let props = env.ok("reports.generate", json!({ "reportType": "prop" }));
    assert_eq!(props["rows"][0][0], "Red Folder");
    assert_eq!(
        env.err("reports.generate", json!({ "reportType": "payroll" })),
        "validation.invalid_input"
    );
    assert_eq!(
        view(&env)["days"],
        before["days"],
        "reports never change data"
    );
    // Saved snapshot persists; generation alone does not.
    assert!(
        env.ok("reports.list", json!({}))
            .as_array()
            .unwrap()
            .is_empty()
    );
    let id = env
        .ok("reports.save", json!({ "reportType": "schedule" }))
        .as_str()
        .unwrap()
        .to_string();
    let saved = env.ok("reports.get", json!({ "id": id }));
    assert_eq!(saved["data"]["rows"][0][0], "Shoot Day 1");
    env.ok("reports.delete", json!({ "id": id }));
    assert!(
        env.ok("reports.list", json!({}))
            .as_array()
            .unwrap()
            .is_empty()
    );
}

// ------------------------------------------------------------------ budget

#[test]
fn fsd_prod_025_budget_is_advisory_and_validated() {
    let env = TestEnv::with_project("Short", "Short Film");
    let v = env.ok("budget.get", json!({}));
    assert!(v["current"].is_null());
    assert_eq!(
        v["categories"],
        json!([
            "Cast",
            "Crew",
            "Locations",
            "Equipment",
            "Art/Props",
            "Travel/Transport",
            "Food",
            "Post/Other",
            "Contingency"
        ])
    );
    let b = env
        .ok("budget.create", json!({ "currency": "inr" }))
        .as_str()
        .unwrap()
        .to_string();
    env.ok("budget.update", json!({ "id": b, "currency": "INR", "plannedTotal": 120_000_000, "contingencyMode": "percent", "contingencyValue": 1000, "notes": null }));
    let l1 = env.ok("budget.add_line", json!({ "budgetId": b, "category": "Cast", "description": "Lead actors", "amount": 40_000_000 }));
    env.ok("budget.add_line", json!({ "budgetId": b, "category": "Food", "description": "Catering", "amount": 5_000_000 }));
    assert_eq!(
        env.err(
            "budget.add_line",
            json!({ "budgetId": b, "category": "Food", "description": "Refund", "amount": -5 })
        ),
        "validation.amount"
    );
    assert_eq!(
        env.err(
            "budget.add_line",
            json!({ "budgetId": b, "category": "Food", "description": "  ", "amount": 5 })
        ),
        "validation.required"
    );
    assert_eq!(
        env.err(
            "budget.add_line",
            json!({ "budgetId": b, "category": "Payroll", "description": "x", "amount": 5 })
        ),
        "validation.invalid_input"
    );
    let cur = &env.ok("budget.get", json!({}))["current"];
    assert_eq!(cur["enteredTotal"], 45_000_000);
    assert_eq!(
        cur["contingencyAmount"], 12_000_000,
        "10% of the planned total"
    );
    assert_eq!(cur["categories"][0]["total"], 40_000_000);
    assert_eq!(
        cur["lines"].as_array().unwrap().len(),
        2,
        "valid lines are kept"
    );

    env.ok("budget.update_line", json!({ "id": l1, "category": "Cast", "description": "Lead actors", "amount": 45_000_000, "notes": null }));
    env.undo();
    assert_eq!(
        env.ok("budget.get", json!({}))["current"]["enteredTotal"],
        45_000_000
    );

    // Snapshot is frozen with new identities.
    let snap = env
        .ok(
            "budget.save_snapshot",
            json!({ "id": b, "label": "Pre-production estimate" }),
        )
        .as_str()
        .unwrap()
        .to_string();
    env.ok("budget.delete_line", json!({ "id": l1 }));
    let v = env.ok("budget.get", json!({}));
    assert_eq!(v["current"]["enteredTotal"], 5_000_000);
    assert_eq!(v["snapshots"][0]["enteredTotal"], 45_000_000);
    assert_eq!(
        env.ok("budget.get_snapshot", json!({ "id": snap }))["lines"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        env.err("budget.update", json!({ "id": snap, "currency": "INR", "plannedTotal": 1, "contingencyMode": "amount", "contingencyValue": 0, "notes": null })),
        "conflict.state"
    );
    // Deleted line restores from Recently Deleted.
    let trash = env.ok("trash.list", json!({}));
    let entry = trash
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["objectType"] == "budget_line")
        .unwrap()
        .clone();
    env.ok("trash.restore", json!({ "id": entry["id"] }));
    assert_eq!(
        env.ok("budget.get", json!({}))["current"]["enteredTotal"],
        45_000_000
    );
    // Viewer can't edit.
    let viewer = env.actor_with_role(Role::Viewer);
    assert!(
        env.call_as(
            &viewer,
            "budget.add_line",
            json!({ "budgetId": b, "category": "Food", "description": "x", "amount": 1 })
        )
        .is_err()
    );
}

#[test]
fn ux_3_33_report_filter_keeps_filters_and_says_no_matching_data() {
    let (env, _, _, _) = call_sheet_env();
    let r = env.ok(
        "reports.generate",
        json!({ "reportType": "scene", "filter": "  railway " }),
    );
    let rows = r["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 2, "scenes 2 and 4 are at the railway station");
    assert!(
        rows.iter()
            .all(|row| row[1].as_str().unwrap().contains("RAILWAY"))
    );
    assert_eq!(r["filter"], "railway");
    assert!(r["note"].is_null());
    let none = env.ok(
        "reports.generate",
        json!({ "reportType": "scene", "filter": "submarine" }),
    );
    assert!(none["rows"].as_array().unwrap().is_empty());
    assert_eq!(none["note"], "No matching data.");
    assert!(
        !none["columns"].as_array().unwrap().is_empty(),
        "columns stay visible"
    );
    // A blank filter is no filter.
    let all = env.ok(
        "reports.generate",
        json!({ "reportType": "scene", "filter": "   " }),
    );
    assert_eq!(all["rows"].as_array().unwrap().len(), 4);
    assert!(all["filter"].is_null());
    assert_eq!(
        env.err(
            "reports.generate",
            json!({ "reportType": "scene", "filter": "x".repeat(201) })
        ),
        "validation.filter"
    );
    // Saved snapshots keep the filter they were made with.
    let id = env
        .ok(
            "reports.save",
            json!({ "reportType": "scene", "filter": "morgue" }),
        )
        .as_str()
        .unwrap()
        .to_string();
    let saved = env.ok("reports.get", json!({ "id": id }));
    assert_eq!(saved["data"]["filter"], "morgue");
    assert_eq!(saved["data"]["rows"].as_array().unwrap().len(), 1);
    // Viewer may read reports but not save them.
    let viewer = env.actor_with_role(Role::Viewer);
    assert!(
        env.call_as(
            &viewer,
            "reports.generate",
            json!({ "reportType": "scene" })
        )
        .is_ok()
    );
    assert!(
        env.call_as(&viewer, "reports.save", json!({ "reportType": "scene" }))
            .is_err()
    );
}

#[test]
fn fsd_39_6_source_change_prompts_budget_review_without_changing_money() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    seed(&env);
    let b = env
        .ok("budget.create", json!({ "currency": "INR" }))
        .as_str()
        .unwrap()
        .to_string();
    env.ok("budget.add_line", json!({ "budgetId": b, "category": "Locations", "description": "Railway permission", "amount": 4_000_000 }));
    assert!(
        env.ok("budget.get", json!({}))["reviewReminder"].is_null(),
        "reviewed against the source it was created with"
    );

    // A new draft becomes the production source → a manual reminder only.
    exec(
        &env,
        "INSERT INTO screenplay_draft(id,screenplay_id,name,status,created_at,updated_at) VALUES ('d2','sp','Shooting Draft 7','Locked',2,2);
         UPDATE production_source SET active = 0;
         INSERT INTO production_source(id,draft_id,active,selected_at,created_at,updated_at) VALUES ('ps2','d2',1,2,2,2);",
    );
    let v = env.ok("budget.get", json!({}));
    let msg = v["reviewReminder"].as_str().expect("review reminder");
    assert!(msg.contains("Shooting Draft 7"), "{msg}");
    assert_eq!(
        v["current"]["enteredTotal"], 4_000_000,
        "amounts never change automatically"
    );

    env.ok("budget.mark_reviewed", json!({ "id": b }));
    assert!(env.ok("budget.get", json!({}))["reviewReminder"].is_null());
    env.undo();
    assert!(
        env.ok("budget.get", json!({}))["reviewReminder"].is_string(),
        "mark reviewed is undoable"
    );
    let viewer = env.actor_with_role(Role::Viewer);
    assert!(
        env.call_as(&viewer, "budget.mark_reviewed", json!({ "id": b }))
            .is_err()
    );
}

#[test]
fn budget_snapshots_can_be_deleted_restored_and_purged() {
    let env = TestEnv::with_project("Short", "Short Film");
    let b = env
        .ok("budget.create", json!({ "currency": "USD" }))
        .as_str()
        .unwrap()
        .to_string();
    env.ok(
        "budget.add_line",
        json!({ "budgetId": b, "category": "Food", "description": "Catering", "amount": 50_000 }),
    );
    let snap = env
        .ok(
            "budget.save_snapshot",
            json!({ "id": b, "label": "Draft estimate" }),
        )
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        env.err("budget.delete_snapshot", json!({ "id": b })),
        "validation.invalid_input",
        "the working budget stays"
    );
    env.ok("budget.delete_snapshot", json!({ "id": snap }));
    assert!(
        env.ok("budget.get", json!({}))["snapshots"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let entry = env
        .ok("trash.list", json!({}))
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["objectType"] == "budget_snapshot")
        .unwrap()
        .clone();
    assert_eq!(entry["title"], "Draft estimate");
    env.ok("trash.restore", json!({ "id": entry["id"] }));
    assert_eq!(
        env.ok("budget.get", json!({}))["snapshots"][0]["enteredTotal"],
        50_000
    );
    env.ok("budget.delete_snapshot", json!({ "id": snap }));
    let entry = env
        .ok("trash.list", json!({}))
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["objectType"] == "budget_snapshot")
        .unwrap()
        .clone();
    env.ok("trash.purge", json!({ "id": entry["id"] }));
    assert_eq!(
        env.err("budget.get_snapshot", json!({ "id": snap })),
        "not_found.budget"
    );
    assert_eq!(
        env.ok("budget.get", json!({}))["current"]["enteredTotal"],
        50_000,
        "working budget untouched"
    );
}
