//! Shooting schedule depth (masterplan area "Shooting schedule and Day Out of
//! Days"; contract docs/engineering/33-production-depth-contract.md section 4;
//! Screenplay and Production Depth spec sections 25 to 28; features SCHED-003
//! to SCHED-009 and DOOD-001), end to end through the operation registry.
//!
//! Screenplay and production hub rows belong to other modules; the tests seed
//! them directly as fixtures, like `tests/schedule.rs`.

use std::path::PathBuf;

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

fn scalar(env: &TestEnv, sql: &str) -> String {
    let s = env.core.project().unwrap();
    s.store
        .read(|c| Ok(c.query_row(sql, [], |r| r.get::<_, String>(0))?))
        .unwrap()
}

/// A locked draft with 4 scenes, two characters (Arjun has an actor), two
/// locations and confirmed breakdown, selected as the Production Source.
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
         INSERT INTO screenplay_element(id,scene_id,position,element_type,text,created_at,updated_at) VALUES
            ('e1','s1',1,'action','Rain lashes the windows. MEERA slides a red folder across the desk.',1,1),
            ('e4','s2',1,'action','ARJUN walks the empty platform, photograph in hand.',1,1),
            ('e5','s3',1,'action','A sheet is pulled back.',1,1),
            ('e6','s4',1,'action','A figure watches from the far platform.',1,1);
         INSERT INTO story_character(id,name,position,created_at,updated_at) VALUES ('arjun','Arjun',1,1,1),('meera','Meera',2,1,1);
         INSERT INTO cast_member(id,person_name,character_id,created_at,updated_at) VALUES ('cm1','Karthik Menon','arjun',1,1);
         INSERT INTO location(id,name,address,status,created_at,updated_at) VALUES
            ('loc_police','Police Station','District Rd, Gudur','Confirmed',1,1),
            ('loc_rail','Old Railway Station','Station Rd','Confirmed',1,1);
         INSERT INTO catalog_item(id,category,name,character_id,location_id,created_at,updated_at) VALUES
            ('ci_arjun','Cast','Arjun','arjun',NULL,1,1),
            ('ci_meera','Cast','Meera','meera',NULL,1,1),
            ('ci_police','Location / Set','Police Station',NULL,'loc_police',1,1),
            ('ci_rail','Location / Set','Old Railway Station',NULL,'loc_rail',1,1);
         INSERT INTO production_source(id,draft_id,active,selected_at,created_at,updated_at) VALUES ('ps1','d1',1,1,1,1);
         INSERT INTO breakdown_element(id,source_id,scene_id,scene_lineage_id,category,catalog_item_id,display_name,confirmation_state,created_at,updated_at) VALUES
            ('b1','ps1','s1','L1','Cast','ci_arjun','Arjun','Confirmed',1,1),
            ('b2','ps1','s1','L1','Cast','ci_meera','Meera','Confirmed',1,1),
            ('b3','ps1','s1','L1','Location / Set','ci_police','Police Station','Confirmed',1,1),
            ('b5','ps1','s2','L2','Cast','ci_arjun','Arjun','Confirmed',1,1),
            ('b6','ps1','s2','L2','Location / Set','ci_rail','Old Railway Station','Confirmed',1,1),
            ('b7','ps1','s3','L3','Cast','ci_meera','Meera','Confirmed',1,1),
            ('b8','ps1','s4','L4','Cast','ci_arjun','Arjun','Confirmed',1,1),
            ('b9','ps1','s4','L4','Location / Set','ci_rail','Old Railway Station','Confirmed',1,1),
            ('b10','ps1','s3','L3','Props','ci_arjun','Sheet','Suggested',1,1);",
    );
}

fn env_with_schedule() -> (TestEnv, String) {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    seed(&env);
    let s = env.ok("schedule.create", json!({}));
    (env, s["id"].as_str().unwrap().to_string())
}

fn view(env: &TestEnv) -> Value {
    env.ok("schedule.get", json!({}))
}

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
        .expect("scene on board")["id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn day(v: &Value, id: &str) -> Value {
    v["days"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == id)
        .expect("day")
        .clone()
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

fn estimate(env: &TestEnv, number: i64, minutes: i64) {
    env.ok(
        "schedule.set_strip_estimate",
        json!({ "stripId": strip(env, number), "minutes": minutes }),
    );
}

fn hm(h: i64, m: i64) -> i64 {
    h * 60 + m
}

fn starts(d: &Value) -> Vec<String> {
    d["timeline"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            format!(
                "{} {}",
                r["startLabel"].as_str().unwrap_or("--"),
                r["label"].as_str().unwrap()
            )
        })
        .collect()
}

/// Where every strip sits: (strip id, day id, position), sorted.
fn placement(env: &TestEnv) -> String {
    scalar(
        env,
        "SELECT group_concat(id || '@' || COALESCE(day_id,'-') || '#' || position, ' ') FROM
           (SELECT id, day_id, position FROM schedule_strip ORDER BY id)",
    )
}

/// Fingerprint of the screenplay and breakdown tables (DOOD and the timeline must never write them).
fn source_fingerprint(env: &TestEnv) -> String {
    scalar(
        env,
        "SELECT (SELECT group_concat(id || ':' || rev || ':' || updated_at, ',') FROM screenplay_scene)
             || '|' || (SELECT group_concat(id || ':' || rev || ':' || updated_at, ',') FROM screenplay_element)
             || '|' || (SELECT group_concat(id || ':' || rev || ':' || updated_at, ',') FROM breakdown_element)
             || '|' || (SELECT group_concat(id || ':' || rev, ',') FROM cast_member)",
    )
}

fn out(env: &TestEnv, name: &str) -> String {
    let d: PathBuf = env.dir.path().join("exports");
    std::fs::create_dir_all(&d).unwrap();
    d.join(name).to_string_lossy().into_owned()
}

fn pdf_text(path: &str) -> String {
    let doc = lopdf::Document::load(path).expect("PDF parses");
    let mut out = String::new();
    for (_, page) in doc.get_pages() {
        let bytes = doc.get_page_content(page);
        let content = lopdf::content::Content::decode(&bytes).unwrap();
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

/// Day 1: scenes 1 and 2 with a meal between them; Day 2: scene 4.
fn planned(env: &TestEnv, sched: &str) -> (String, String) {
    let d1 = new_day(env, sched, Some("2027-06-14"));
    let d2 = new_day(env, sched, Some("2027-06-15"));
    move_to(env, 1, Some(&d1));
    move_to(env, 2, Some(&d1));
    move_to(env, 4, Some(&d2));
    estimate(env, 1, 90);
    estimate(env, 2, 120);
    estimate(env, 4, 60);
    env.ok(
        "schedule.add_marker",
        json!({ "dayId": d1, "markerType": "Meal", "durationMinutes": 60, "index": 1 }),
    );
    (d1, d2)
}

// ------------------------------------------------------------------ day timeline

#[test]
fn sched_003_day_times_are_stored_undoable_and_validated() {
    let (env, sched) = env_with_schedule();
    let (d1, _) = planned(&env, &sched);
    env.ok(
        "schedule.set_day_times",
        json!({ "dayId": d1, "crewCall": hm(7, 0), "shootingStart": hm(8, 0), "targetWrap": hm(19, 0) }),
    );
    let d = day(&view(&env), &d1);
    assert_eq!(
        (
            d["crewCall"].clone(),
            d["shootingStart"].clone(),
            d["targetWrap"].clone()
        ),
        (json!(420), json!(480), json!(1140))
    );
    assert_eq!(d["timeline"]["crewCallLabel"], "07:00");
    assert_eq!(d["timeline"]["targetWrapLabel"], "19:00");
    // Stored as integers, never display strings.
    assert_eq!(
        scalar(
            &env,
            &format!("SELECT typeof(crew_call) FROM shooting_day WHERE id = '{d1}'")
        ),
        "integer"
    );
    env.undo();
    assert!(day(&view(&env), &d1)["crewCall"].is_null());
    env.redo();
    assert_eq!(day(&view(&env), &d1)["crewCall"], 420);
    // Clearing one time keeps the others.
    env.ok(
        "schedule.set_day_times",
        json!({ "dayId": d1, "crewCall": null, "shootingStart": hm(8, 0), "targetWrap": hm(19, 0) }),
    );
    assert!(day(&view(&env), &d1)["crewCall"].is_null());
    for bad in [-1, 1440, 5000] {
        assert_eq!(
            env.err(
                "schedule.set_day_times",
                json!({ "dayId": d1, "crewCall": bad, "shootingStart": null, "targetWrap": null })
            ),
            "validation.invalid_input"
        );
    }
    let off = env
        .ok(
            "schedule.create_day",
            json!({ "scheduleId": sched, "offDay": true }),
        )
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        env.err(
            "schedule.set_day_times",
            json!({ "dayId": off, "crewCall": 420, "shootingStart": null, "targetWrap": null })
        ),
        "validation.off_day"
    );
    // A viewer cannot change times.
    let viewer = env.other_user(Role::Viewer);
    assert!(
        env.call_as(
            &viewer,
            "schedule.set_day_times",
            json!({ "dayId": d1, "crewCall": 420, "shootingStart": null, "targetWrap": null })
        )
        .is_err()
    );
}

#[test]
fn sched_004_scene_starts_and_breaks_feed_the_timeline_and_day_total() {
    let (env, sched) = env_with_schedule();
    let (d1, _) = planned(&env, &sched);
    env.ok(
        "schedule.set_day_times",
        json!({ "dayId": d1, "crewCall": hm(7, 0), "shootingStart": hm(8, 0), "targetWrap": hm(19, 0) }),
    );
    let d = day(&view(&env), &d1);
    assert_eq!(
        starts(&d),
        [
            "07:00 Crew call",
            "08:00 Shooting starts",
            "08:00 Scene 1",
            "09:30 Meal Break",
            "10:30 Scene 2",
            "12:30 Estimated wrap"
        ]
    );
    assert_eq!(d["timeline"]["workingMinutes"], 270);
    assert_eq!(d["summary"]["estimatedMinutes"], 270);
    assert_eq!(d["summary"]["totalLabel"], "2 scenes · 4h 30m · wrap 12:30");

    // A travel break between scenes moves the later start and the wrap.
    env.ok(
        "schedule.add_marker",
        json!({ "dayId": d1, "markerType": "Travel", "durationMinutes": 45, "index": 2 }),
    );
    let d = day(&view(&env), &d1);
    assert_eq!(d["timeline"]["estimatedWrapLabel"], "13:15");
    assert_eq!(d["timeline"]["travelMinutes"], 45);
    assert_eq!(d["summary"]["travelMinutes"], 45);

    // Pinning a start shows the gap; the scene does not move.
    let before = placement(&env);
    env.ok(
        "schedule.set_strip_start",
        json!({ "stripId": strip(&env, 2), "minutes": hm(14, 0) }),
    );
    assert_eq!(placement(&env), before, "a start time never moves a scene");
    let d = day(&view(&env), &d1);
    let row = d["timeline"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["label"] == "Scene 2")
        .unwrap()
        .clone();
    assert_eq!(
        (row["startLabel"].clone(), row["pinned"].clone()),
        (json!("14:00"), json!(true))
    );
    // Scene 2 would follow the travel break at 11:15; pinned at 14:00 it leaves 2h 45m free.
    assert_eq!(row["gapMinutes"], 165);
    assert_eq!(d["timeline"]["estimatedWrapLabel"], "16:00");
    env.undo();
    assert_eq!(
        day(&view(&env), &d1)["timeline"]["estimatedWrapLabel"],
        "13:15"
    );

    // Unscheduled scenes cannot get a start time; a missing estimate is flagged, never invented.
    assert_eq!(
        env.err(
            "schedule.set_strip_start",
            json!({ "stripId": strip(&env, 3), "minutes": 600 })
        ),
        "validation.unscheduled"
    );
    env.ok(
        "schedule.set_strip_estimate",
        json!({ "stripId": strip(&env, 1), "minutes": null }),
    );
    let d = day(&view(&env), &d1);
    let s1 = d["timeline"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["label"] == "Scene 1")
        .unwrap()
        .clone();
    assert_eq!(s1["missingEstimate"], true);
    assert_eq!(d["summary"]["missingEstimates"], 1);
}

#[test]
fn night_shoot_wraps_after_midnight() {
    let (env, sched) = env_with_schedule();
    let (_, d2) = planned(&env, &sched);
    estimate(&env, 4, 300);
    env.ok(
        "schedule.set_day_times",
        json!({ "dayId": d2, "crewCall": hm(22, 0), "shootingStart": hm(23, 0), "targetWrap": hm(5, 0) }),
    );
    let d = day(&view(&env), &d2);
    assert_eq!(d["timeline"]["estimatedWrapLabel"], "04:00 (+1)");
    assert_eq!(d["timeline"]["targetWrapLabel"], "05:00 (+1)");
    assert_eq!(d["summary"]["overTarget"], false);
}

// ------------------------------------------------------------------ warnings

fn warnings(env: &TestEnv) -> Vec<Value> {
    view(env)["warnings"].as_array().unwrap().clone()
}

#[test]
fn sched_005_over_target_and_travel_warnings_are_advice_only() {
    let (env, sched) = env_with_schedule();
    let (d1, _) = planned(&env, &sched);
    env.ok(
        "schedule.set_day_target",
        json!({ "dayId": d1, "minutes": 240 }),
    );
    // 90 + 60 meal + 120 = 270 > 240: plain over-target.
    let w = warnings(&env);
    assert!(w.iter().any(|w| w["kind"] == "duration_overflow"));
    assert!(!w.iter().any(|w| w["kind"] == "travel_overflow"));

    // With a 30 min target cushion the day fits until a travel break pushes it over.
    env.ok(
        "schedule.set_day_target",
        json!({ "dayId": d1, "minutes": 300 }),
    );
    assert!(
        !warnings(&env)
            .iter()
            .any(|w| w["kind"] == "duration_overflow")
    );
    env.ok(
        "schedule.add_marker",
        json!({ "dayId": d1, "markerType": "Company Move", "durationMinutes": 40 }),
    );
    let w = warnings(&env);
    let t = w
        .iter()
        .find(|w| w["kind"] == "travel_overflow")
        .expect("travel pushes the day over");
    assert!(
        t["message"]
            .as_str()
            .unwrap()
            .starts_with("Travel and company moves push Shoot Day 1 past its target")
    );
    assert!(!w.iter().any(|w| w["kind"] == "duration_overflow"));

    // Target wrap: the estimated wrap is compared with the entered wrap.
    env.ok(
        "schedule.set_day_target",
        json!({ "dayId": d1, "minutes": null }),
    );
    env.ok(
        "schedule.set_day_times",
        json!({ "dayId": d1, "crewCall": hm(7, 0), "shootingStart": hm(8, 0), "targetWrap": hm(12, 0) }),
    );
    let w = warnings(&env);
    assert!(
        w.iter()
            .any(|w| w["kind"] == "duration_overflow" || w["kind"] == "travel_overflow"),
        "{w:?}"
    );
    assert_eq!(day(&view(&env), &d1)["summary"]["overTargetWrap"], true);

    // Advice only: nothing moved, and Keep Anyway records the decision.
    let before = placement(&env);
    let key = warnings(&env)
        .iter()
        .find(|w| w["kind"] == "duration_overflow" || w["kind"] == "travel_overflow")
        .unwrap()["key"]
        .as_str()
        .unwrap()
        .to_string();
    env.ok(
        "schedule.decide_warning",
        json!({ "scheduleId": sched, "key": key, "decision": "Kept" }),
    );
    assert_eq!(placement(&env), before);
    // No dated availability is entered in this project, so there is no availability warning.
    assert!(
        !warnings(&env)
            .iter()
            .any(|w| w["kind"] == "cast_unavailable" || w["kind"] == "location_unavailable")
    );
}

#[test]
fn missing_location_still_warns() {
    let (env, sched) = env_with_schedule();
    let d1 = new_day(&env, &sched, None);
    move_to(&env, 3, Some(&d1));
    assert!(
        warnings(&env)
            .iter()
            .any(|w| w["kind"] == "missing_location")
    );
}

/// SCHED-005: dated availability entered on the cast member and the location
/// (area 13 `availability.set`) is read by the schedule as advice. Undated days,
/// Available windows and days outside the range never warn, and nothing moves.
#[test]
fn sched_005_cast_and_location_availability_warn_on_dated_days_only() {
    let (env, sched) = env_with_schedule();
    let d1 = new_day(&env, &sched, Some("2027-06-14"));
    let undated = new_day(&env, &sched, None);
    move_to(&env, 2, Some(&d1)); // Arjun at the Old Railway Station
    move_to(&env, 4, Some(&undated)); // Arjun at the Old Railway Station, no date
    let none = |env: &TestEnv| {
        !warnings(env)
            .iter()
            .any(|w| w["kind"] == "cast_unavailable" || w["kind"] == "location_unavailable")
    };
    assert!(none(&env), "no dated availability, no warning");

    // An Available window is not a conflict.
    env.ok(
        "availability.set",
        json!({ "ownerType": "cast_member", "ownerId": "cm1", "windows": [
            { "startDate": "2027-06-01", "endDate": "2027-06-30", "kind": "Available" }
        ] }),
    );
    assert!(none(&env));

    // Unavailable wins over the overlapping Available window. Telugu note kept as typed.
    env.ok(
        "availability.set",
        json!({ "ownerType": "cast_member", "ownerId": "cm1", "windows": [
            { "startDate": "2027-06-01", "endDate": "2027-06-30", "kind": "Available" },
            { "startDate": "2027-06-13", "endDate": "2027-06-15", "kind": "Unavailable", "note": "పరీక్షలు" }
        ] }),
    );
    env.ok(
        "availability.set",
        json!({ "ownerType": "location", "ownerId": "loc_rail", "windows": [
            { "startDate": "2027-06-14", "endDate": "2027-06-14", "kind": "Unavailable", "note": "Station closed" }
        ] }),
    );
    let before = placement(&env);
    let w = warnings(&env);
    let cast: Vec<&Value> = w
        .iter()
        .filter(|w| w["kind"] == "cast_unavailable")
        .collect();
    assert_eq!(cast.len(), 1, "the undated day never warns: {w:?}");
    assert_eq!(
        cast[0]["message"],
        "Karthik Menon (Arjun) is marked unavailable on Shoot Day 1."
    );
    assert_eq!(
        cast[0]["detail"],
        "Unavailable 2027-06-13 to 2027-06-15: పరీక్షలు"
    );
    let loc: Vec<&Value> = w
        .iter()
        .filter(|w| w["kind"] == "location_unavailable")
        .collect();
    assert_eq!(loc.len(), 1);
    assert_eq!(
        loc[0]["message"],
        "Old Railway Station is marked unavailable on Shoot Day 1."
    );

    // Keep Anyway records the decision; the scene stays where the user put it.
    let key = cast[0]["key"].as_str().unwrap().to_string();
    env.ok(
        "schedule.decide_warning",
        json!({ "scheduleId": sched, "key": key, "decision": "Kept" }),
    );
    assert_eq!(placement(&env), before);

    // Moving the day off the range clears both warnings.
    env.ok(
        "schedule.set_day_date",
        json!({ "dayId": d1, "date": "2027-06-20" }),
    );
    assert!(none(&env));
    assert_eq!(placement(&env), before);
}

// ------------------------------------------------------------------ no automatic planner

/// SCHED-009: every schedule operation that is not an explicit move leaves
/// every scene exactly where the user put it.
#[test]
fn sched_009_nothing_moves_a_scene_without_a_user_move() {
    let (env, sched) = env_with_schedule();
    let (d1, d2) = planned(&env, &sched);
    env.ok("callsheets.create", json!({ "dayId": d1 }));
    let before = placement(&env);
    let s1 = strip(&env, 1);
    let cast_key = "char:arjun";
    let steps: Vec<(&str, Value)> = vec![
        (
            "schedule.set_day_times",
            json!({ "dayId": d1, "crewCall": 420, "shootingStart": 480, "targetWrap": 600 }),
        ),
        (
            "schedule.set_strip_estimate",
            json!({ "stripId": s1, "minutes": 600 }),
        ),
        (
            "schedule.set_strip_start",
            json!({ "stripId": s1, "minutes": 900 }),
        ),
        (
            "schedule.set_strip_pages",
            json!({ "stripId": s1, "eighths": 12 }),
        ),
        (
            "schedule.set_day_target",
            json!({ "dayId": d1, "minutes": 60 }),
        ),
        (
            "schedule.set_day_date",
            json!({ "dayId": d2, "date": "2027-06-14" }),
        ),
        (
            "schedule.set_day_notes",
            json!({ "dayId": d1, "notes": "Rain cover" }),
        ),
        (
            "schedule.add_marker",
            json!({ "dayId": d2, "markerType": "Travel", "durationMinutes": 300 }),
        ),
        (
            "schedule.update_settings",
            json!({ "scheduleId": sched, "name": "Plan B", "strictValidation": true, "dayDurationMinutes": 60, "doodCodeStyle": "simple" }),
        ),
        (
            "schedule.set_dood_override",
            json!({ "scheduleId": sched, "castKey": cast_key, "dayId": d2, "code": "H" }),
        ),
        ("schedule.duplicate_day", json!({ "dayId": d1 })),
        ("schedule.reconcile", json!({ "scheduleId": sched })),
    ];
    for (op, args) in steps {
        env.ok(op, args);
        assert_eq!(placement(&env), before, "{op} moved a scene");
    }
    for (q, args) in [
        ("schedule.get", json!({})),
        ("schedule.suggestions", json!({})),
        ("schedule.daily", json!({})),
        ("schedule.dood", json!({})),
    ] {
        env.ok(q, args);
        assert_eq!(placement(&env), before, "{q} moved a scene");
    }
    // Suggestions exist but stay suggestions until the user confirms one.
    let sg = env.ok("schedule.suggestions", json!({}));
    assert!(!sg["suggestions"].as_array().unwrap().is_empty());
    assert_eq!(placement(&env), before);
}

// ------------------------------------------------------------------ stale state and call sheets

#[test]
fn sched_007_call_sheet_stale_flag_follows_day_times_and_starts() {
    let (env, sched) = env_with_schedule();
    let (d1, _) = planned(&env, &sched);
    let cs = env
        .ok("callsheets.create", json!({ "dayId": d1 }))
        .as_str()
        .unwrap()
        .to_string();
    let status = |env: &TestEnv| day(&view(env), &d1)["callSheet"]["status"].clone();
    assert_eq!(status(&env), "Draft", "a fresh sheet is not stale");
    // The scene table shows each scene's length, so a new estimate is a change to review.
    estimate(&env, 1, 100);
    assert_eq!(status(&env), "Needs refresh");
    env.ok("callsheets.refresh", json!({ "id": cs }));
    assert_eq!(status(&env), "Draft");
    env.ok(
        "schedule.set_day_times",
        json!({ "dayId": d1, "crewCall": hm(6, 30), "shootingStart": null, "targetWrap": null }),
    );
    assert_eq!(status(&env), "Needs refresh");
    assert!(
        warnings(&env)
            .iter()
            .any(|w| w["kind"] == "call_sheet_stale")
    );
    env.undo();
    assert_eq!(status(&env), "Draft", "undo restores the stale flag too");
    env.redo();
    let preview = env.ok("callsheets.refresh_preview", json!({ "id": cs }));
    assert_eq!(preview["hasChanges"], true);
    env.ok("callsheets.refresh", json!({ "id": cs }));
    assert_eq!(status(&env), "Draft");
    let doc = env.ok("callsheets.get", json!({ "id": cs }));
    assert_eq!(
        doc["document"]["crewCall"], "06:30",
        "crew call prefilled from the day"
    );
    // With times entered, a changed estimate moves later start times: the sheet needs a refresh.
    estimate(&env, 1, 30);
    assert_eq!(status(&env), "Needs refresh");
}

#[test]
fn sched_007_script_change_review_keeps_new_fields() {
    let (env, sched) = env_with_schedule();
    let (d1, _) = planned(&env, &sched);
    env.ok(
        "schedule.set_strip_start",
        json!({ "stripId": strip(&env, 1), "minutes": 600 }),
    );
    exec(
        &env,
        "UPDATE screenplay_scene SET heading = 'INT. POLICE STATION - DAY', rev = rev + 1 WHERE id = 's1';",
    );
    let v = view(&env);
    assert!(v["scriptChanges"]["changed"].as_array().unwrap().len() == 1);
    env.ok("schedule.reconcile", json!({ "scheduleId": sched }));
    let d = day(&view(&env), &d1);
    let s = d["items"][0]["strip"].clone();
    assert_eq!(s["sourceState"], "Changed");
    assert_eq!(
        s["startMinutes"], 600,
        "review never drops the user's start time"
    );
    env.ok("schedule.acknowledge_change", json!({ "stripId": s["id"] }));
    assert_eq!(
        day(&view(&env), &d1)["items"][0]["strip"]["sourceState"],
        "Current"
    );
}

// ------------------------------------------------------------------ Day Out of Days

fn dood(env: &TestEnv, args: Value) -> Value {
    env.ok("schedule.dood", args)
}

fn codes(v: &Value, key: &str) -> Vec<String> {
    v["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["key"] == key)
        .unwrap()["cells"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["code"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn dood_001_derived_from_cast_and_schedule_with_ranges_and_corrections() {
    let (env, sched) = env_with_schedule();
    assert_eq!(dood(&env, json!({}))["days"].as_array().unwrap().len(), 0);
    let d1 = new_day(&env, &sched, Some("2027-06-14"));
    let d2 = new_day(&env, &sched, Some("2027-06-15"));
    let d3 = new_day(&env, &sched, Some("2027-06-16"));
    move_to(&env, 1, Some(&d1)); // Arjun, Meera
    move_to(&env, 3, Some(&d2)); // Meera
    move_to(&env, 4, Some(&d3)); // Arjun
    let src = source_fingerprint(&env);
    let sched_before = placement(&env);

    let v = dood(&env, json!({}));
    assert_eq!(v["codeStyle"], "industry");
    assert_eq!(codes(&v, "char:arjun"), ["SW", "H", "WF"]);
    assert_eq!(codes(&v, "char:meera"), ["SW", "WF", ""]);
    let arjun = v["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["key"] == "char:arjun")
        .unwrap()
        .clone();
    assert_eq!(arjun["label"], "Karthik Menon (Arjun)");
    assert_eq!(
        (arjun["workDays"].clone(), arjun["holdDays"].clone()),
        (json!(2), json!(1))
    );
    assert_eq!(arjun["startLabel"], "Shoot Day 1");
    assert_eq!(arjun["finishLabel"], "Shoot Day 3");

    // Cast and day range selection: codes still come from the whole schedule.
    let v = dood(
        &env,
        json!({ "castKeys": ["char:arjun"], "fromDayId": d2, "toDayId": d3 }),
    );
    assert_eq!(v["rows"].as_array().unwrap().len(), 1);
    assert_eq!(v["days"].as_array().unwrap().len(), 2);
    assert_eq!(codes(&v, "char:arjun"), ["H", "WF"]);
    assert_eq!(v["allDays"].as_array().unwrap().len(), 3);
    assert_eq!(v["allCast"].as_array().unwrap().len(), 2);

    // Manual correction: one undo step, shown as corrected, back to derived on clear.
    env.ok(
        "schedule.set_dood_override",
        json!({ "scheduleId": sched, "castKey": "char:arjun", "dayId": d2, "code": "T", "note": "Flies in for Day 3" }),
    );
    let v = dood(&env, json!({}));
    assert_eq!(codes(&v, "char:arjun"), ["SW", "T", "WF"]);
    let cell = v["rows"][0]["cells"][1].clone();
    assert_eq!(
        (cell["overridden"].clone(), cell["derivedCode"].clone()),
        (json!(true), json!("H"))
    );
    env.undo();
    assert_eq!(
        codes(&dood(&env, json!({})), "char:arjun"),
        ["SW", "H", "WF"]
    );
    env.redo();
    env.ok(
        "schedule.set_dood_override",
        json!({ "scheduleId": sched, "castKey": "char:arjun", "dayId": d2, "code": null }),
    );
    assert_eq!(
        codes(&dood(&env, json!({})), "char:arjun"),
        ["SW", "H", "WF"]
    );
    assert_eq!(
        env.err(
            "schedule.set_dood_override",
            json!({ "scheduleId": sched, "castKey": "char:arjun", "dayId": d2, "code": "ZZ" })
        ),
        "validation.invalid_input"
    );
    assert_eq!(
        env.err(
            "schedule.set_dood_override",
            json!({ "scheduleId": sched, "castKey": "char:nobody", "dayId": d2, "code": "W" })
        ),
        "not_found.cast_member"
    );

    // Simple codes.
    env.ok(
        "schedule.update_settings",
        json!({ "scheduleId": sched, "name": "Shooting Schedule", "strictValidation": false, "dayDurationMinutes": 600, "doodCodeStyle": "simple" }),
    );
    assert_eq!(codes(&dood(&env, json!({})), "char:arjun"), ["W", "-", "W"]);

    // Moving a scene changes the derived report (and only the report reads it).
    move_to(&env, 4, Some(&d2));
    assert_eq!(codes(&dood(&env, json!({})), "char:arjun"), ["W", "W", "-"]);

    // A correction on a deleted day is kept but not shown.
    env.ok(
        "schedule.set_dood_override",
        json!({ "scheduleId": sched, "castKey": "char:meera", "dayId": d3, "code": "F", "note": "Costume fitting" }),
    );
    env.ok("schedule.delete_day", json!({ "dayId": d3 }));
    let v = dood(&env, json!({}));
    assert_eq!(v["staleCorrections"], 1);
    assert_eq!(v["days"].as_array().unwrap().len(), 2);

    // Read-only to the screenplay and the breakdown.
    assert_eq!(source_fingerprint(&env), src);
    let _ = sched_before;

    // Viewers can read the report but not correct it.
    let viewer = env.other_user(Role::Viewer);
    assert!(env.call_as(&viewer, "schedule.dood", json!({})).is_ok());
    assert!(
        env.call_as(
            &viewer,
            "schedule.set_dood_override",
            json!({ "scheduleId": sched, "castKey": "char:arjun", "dayId": d1, "code": "W" })
        )
        .is_err()
    );
}

#[test]
fn dood_corrections_and_day_times_survive_restart() {
    let (env, sched) = env_with_schedule();
    let (d1, d2) = planned(&env, &sched);
    env.ok(
        "schedule.set_day_times",
        json!({ "dayId": d1, "crewCall": 420, "shootingStart": 480, "targetWrap": 1140 }),
    );
    env.ok(
        "schedule.set_dood_override",
        json!({ "scheduleId": sched, "castKey": "char:meera", "dayId": d2, "code": "R" }),
    );
    let before_view = view(&env);
    let before_dood = dood(&env, json!({}));
    let path = env.project_path();
    let env = env.restart();
    env.reopen_project(&path);
    assert_eq!(view(&env)["days"], before_view["days"]);
    assert_eq!(dood(&env, json!({}))["rows"], before_dood["rows"]);
}

// ------------------------------------------------------------------ exports

#[test]
fn sched_008_schedule_and_dood_exports_are_byte_reproducible() {
    let (env, sched) = env_with_schedule();
    let (d1, d2) = planned(&env, &sched);
    env.ok(
        "schedule.set_day_times",
        json!({ "dayId": d1, "crewCall": hm(7, 0), "shootingStart": hm(8, 0), "targetWrap": hm(19, 0) }),
    );
    env.ok(
        "schedule.set_dood_override",
        json!({ "scheduleId": sched, "castKey": "char:meera", "dayId": d2, "code": "H" }),
    );
    for (op, base) in [
        ("schedule.export_schedule", "schedule"),
        ("schedule.export_dood", "dood"),
    ] {
        for fmt in ["pdf", "csv", "xlsx"] {
            let a = out(&env, &format!("{base}-a.{fmt}"));
            let b = out(&env, &format!("{base}-b.{fmt}"));
            env.ok(op, json!({ "format": fmt, "path": a }));
            std::thread::sleep(std::time::Duration::from_millis(1100));
            env.ok(op, json!({ "format": fmt, "path": b }));
            assert_eq!(
                std::fs::read(&a).unwrap(),
                std::fs::read(&b).unwrap(),
                "{op} {fmt} is not reproducible"
            );
        }
    }
    let t = pdf_text(&out(&env, "schedule-a.pdf"));
    for want in [
        "Shoot Day 1",
        "MONDAY · 14 JUNE 2027",
        "07:00 crew call",
        "08:00",
        "Revised",
        "START",
    ] {
        assert!(t.contains(want), "schedule PDF misses {want}: {t}");
    }
    let t = pdf_text(&out(&env, "dood-a.pdf"));
    for want in [
        "DAY OUT OF DAYS",
        "Karthik Menon",
        "(ARJUN)",
        "SW",
        "H*",
        "Work",
    ] {
        assert!(t.contains(want), "DOOD PDF misses {want}: {t}");
    }
    let csv = std::fs::read_to_string(out(&env, "dood-a.csv")).unwrap();
    assert!(csv.contains("Day 1 (Mon 14 Jun 2027)"), "{csv}");
    // Exports never change the project.
    let rev = scalar(&env, "SELECT group_concat(rev) FROM shooting_day");
    env.ok(
        "schedule.export_dood",
        json!({ "format": "pdf", "path": out(&env, "again.pdf") }),
    );
    assert_eq!(
        scalar(&env, "SELECT group_concat(rev) FROM shooting_day"),
        rev
    );
    // A different schedule gives a different file.
    estimate(&env, 2, 30);
    let c = out(&env, "schedule-c.pdf");
    env.ok(
        "schedule.export_schedule",
        json!({ "format": "pdf", "path": c }),
    );
    assert_ne!(
        std::fs::read(&c).unwrap(),
        std::fs::read(out(&env, "schedule-a.pdf")).unwrap()
    );
}

#[test]
fn dood_export_needs_days_and_cast() {
    let (env, _) = env_with_schedule();
    assert_eq!(
        env.err(
            "schedule.export_dood",
            json!({ "format": "pdf", "path": out(&env, "x.pdf") })
        ),
        "export.nothing_to_export"
    );
}

// ------------------------------------------------------------------ migration

/// A schedule made before migration 0015 keeps every row, gains empty times,
/// and its call sheets keep their fingerprint (no false "Needs refresh").
#[test]
fn migration_0015_upgrades_an_existing_schedule_without_marking_call_sheets_stale() {
    use openframe_application::schema::PROJECT_MIGRATIONS;
    use openframe_persistence::migrate;
    let mut c = openframe_persistence::open_in_memory().unwrap();
    migrate::apply(&mut c, &PROJECT_MIGRATIONS[..14]).unwrap();
    c.execute_batch(
        "INSERT INTO screenplay(id,title,format,created_at,updated_at) VALUES ('sp','T','Feature',1,1);
         INSERT INTO screenplay_draft(id,screenplay_id,name,status,created_at,updated_at) VALUES ('d1','sp','D','Draft',1,1);
         INSERT INTO screenplay_scene(id,draft_id,lineage_id,position,heading,created_at,updated_at) VALUES ('s1','d1','L1',1,'INT. A - DAY',1,1);
         INSERT INTO production_source(id,draft_id,active,selected_at,created_at,updated_at) VALUES ('ps','d1',1,1,1,1);
         INSERT INTO shooting_schedule(id,source_id,created_at,updated_at) VALUES ('sch','ps',1,1);
         INSERT INTO shooting_day(id,schedule_id,position,created_at,updated_at) VALUES ('day','sch',1,1,1);
         INSERT INTO schedule_strip(id,schedule_id,day_id,scene_id,scene_lineage_id,position,estimated_minutes,created_at,updated_at)
             VALUES ('st','sch','day','s1','L1',1,45,1,1);",
    )
    .unwrap();
    migrate::apply(&mut c, PROJECT_MIGRATIONS).unwrap();
    let row: (Option<i64>, Option<i64>, Option<i64>, Option<i64>, i64, String) = c
        .query_row(
            "SELECT d.crew_call, d.shooting_start, d.target_wrap, s.start_minutes, s.estimated_minutes, sc.dood_code_style
             FROM shooting_day d JOIN schedule_strip s ON s.day_id = d.id JOIN shooting_schedule sc ON sc.id = d.schedule_id",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
        )
        .unwrap();
    assert_eq!(row, (None, None, None, None, 45, "industry".to_string()));
    assert_eq!(
        c.query_row("SELECT count(*) FROM dood_override", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    // Out-of-range times and unknown codes are refused by the schema itself.
    assert!(
        c.execute("UPDATE shooting_day SET crew_call = 1440", [])
            .is_err()
    );
    assert!(
        c.execute(
            "INSERT INTO dood_override(id,schedule_id,cast_key,day_id,code,created_at,updated_at) VALUES ('o','sch','k','day','X',1,1)",
            []
        )
        .is_err()
    );
}

#[test]
fn call_sheet_snapshot_without_times_keeps_its_old_fingerprint() {
    use openframe_application::modules::schedule::board::CallSheetSource;
    use openframe_application::modules::schedule::fmt::fnv_hex;
    // A sheet stored before day times existed, with a day that still has none, is not stale.
    let old = r#"{"productionTitle":"","sourceLabel":"","dayId":"","dayNumber":null,"dayLabel":"","date":null,"dayNotes":null,"scenes":[],"cast":[],"locations":[],"breaks":[]}"#;
    assert!(CallSheetSource::default().matches(&fnv_hex(old), 0));
}

// ------------------------------------------------------------------ crash, lock, permissions, search

#[test]
fn day_times_starts_and_dood_corrections_survive_a_crash() {
    let (env, sched) = env_with_schedule();
    let (d1, d2) = planned(&env, &sched);
    env.ok(
        "schedule.set_day_times",
        json!({ "dayId": d1, "crewCall": hm(6, 30), "shootingStart": hm(7, 30), "targetWrap": hm(18, 0) }),
    );
    env.ok(
        "schedule.set_strip_start",
        json!({ "stripId": strip(&env, 2), "minutes": hm(11, 0) }),
    );
    env.ok(
        "schedule.set_dood_override",
        json!({ "scheduleId": sched, "castKey": "char:meera", "dayId": d2, "code": "T", "note": "Flies in" }),
    );
    let before_view = view(&env);
    let before_dood = dood(&env, json!({}));
    let path = env.project_path();
    let env = env.crash_and_restart();
    env.reopen_project(&path);
    assert_eq!(view(&env)["days"], before_view["days"]);
    assert_eq!(dood(&env, json!({}))["rows"], before_dood["rows"]);
    // The committed day times are back exactly.
    let d = day(&view(&env), &d1);
    assert_eq!(d["crewCall"], hm(6, 30));
}

#[test]
fn finalized_schedule_refuses_times_starts_and_corrections() {
    let (env, sched) = env_with_schedule();
    let (d1, d2) = planned(&env, &sched);
    let s2 = strip(&env, 2);
    env.ok(
        "schedule.set_status",
        json!({ "scheduleId": sched, "status": "Finalized" }),
    );
    assert_eq!(
        env.err(
            "schedule.set_day_times",
            json!({ "dayId": d1, "crewCall": 420, "shootingStart": null, "targetWrap": null })
        ),
        "conflict.schedule_finalized"
    );
    assert_eq!(
        env.err(
            "schedule.set_strip_start",
            json!({ "stripId": s2, "minutes": 600 })
        ),
        "conflict.schedule_finalized"
    );
    assert_eq!(
        env.err(
            "schedule.set_dood_override",
            json!({ "scheduleId": sched, "castKey": "char:meera", "dayId": d2, "code": "H" })
        ),
        "conflict.schedule_finalized"
    );
    // Reading the report is still allowed while finalized.
    assert!(dood(&env, json!({}))["rows"].as_array().is_some());
}

#[test]
fn commenters_and_viewers_cannot_change_times_starts_or_corrections() {
    let (env, sched) = env_with_schedule();
    let (d1, d2) = planned(&env, &sched);
    let s2 = strip(&env, 2);
    for role in [Role::Viewer, Role::Commenter] {
        let who = env.other_user(role);
        for (op, args) in [
            (
                "schedule.set_day_times",
                json!({ "dayId": d1, "crewCall": 420, "shootingStart": null, "targetWrap": null }),
            ),
            (
                "schedule.set_strip_start",
                json!({ "stripId": s2, "minutes": 600 }),
            ),
            (
                "schedule.set_dood_override",
                json!({ "scheduleId": sched, "castKey": "char:meera", "dayId": d2, "code": "H" }),
            ),
        ] {
            let e = env.call_as(&who, op, args).unwrap_err();
            assert_eq!(e.code.0, "permission.denied", "{role:?} {op}");
        }
        assert!(env.call_as(&who, "schedule.dood", json!({})).is_ok());
    }
    // Nothing changed.
    let d = day(&view(&env), &d1);
    assert!(d["crewCall"].is_null());
    assert_eq!(
        dood(&env, json!({}))["rows"][0]["cells"][1]["overridden"],
        false
    );
}

#[test]
fn dood_correction_notes_are_searchable_on_their_day() {
    let (env, sched) = env_with_schedule();
    let (_, d2) = planned(&env, &sched);
    env.ok(
        "schedule.set_dood_override",
        json!({ "scheduleId": sched, "castKey": "char:meera", "dayId": d2, "code": "F", "note": "Wardrobe fitting at Lakshmi Tailors" }),
    );
    let hit = |env: &TestEnv| {
        env.ok("search.query", json!({ "text": "Lakshmi" }))
            .as_array()
            .unwrap()
            .iter()
            .any(|h| h["entityId"] == d2)
    };
    assert!(hit(&env), "the correction note finds its shooting day");
    env.undo();
    assert!(!hit(&env), "undo removes the note from search");
    env.redo();
    assert!(hit(&env), "redo brings the note back to search");

    // A day lists its scenes in search; undoing a move takes the scene off that day again.
    let morgue = |env: &TestEnv| {
        env.ok("search.query", json!({ "text": "MORGUE" }))
            .as_array()
            .unwrap()
            .iter()
            .any(|h| h["entityId"] == d2)
    };
    assert!(!morgue(&env));
    move_to(&env, 3, Some(&d2));
    assert!(morgue(&env), "a moved scene is searchable on its new day");
    env.undo();
    assert!(!morgue(&env), "undoing the move reindexes the day");
}
