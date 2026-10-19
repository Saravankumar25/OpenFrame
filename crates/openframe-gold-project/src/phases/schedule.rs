//! The shooting schedule (16 calendar days, 14 shooting days, 2 off days), Day Out of Days,
//! call sheets for every shooting day, sides and the six production reports.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::Ctx;
use crate::gold::{Gold, arr, s};

fn clock(t: &str) -> Value {
    let mut it = t.split(':');
    match (it.next().and_then(|h| h.parse::<i64>().ok()), it.next().and_then(|m| m.parse::<i64>().ok())) {
        (Some(h), Some(m)) => json!(h * 60 + m),
        _ => Value::Null,
    }
}

fn eighths(pages: &str) -> Option<i64> {
    let mut total = 0;
    for part in pages.split_whitespace() {
        if let Some((a, b)) = part.split_once('/') {
            let (a, b): (i64, i64) = (a.parse().ok()?, b.parse().ok()?);
            total += a * 8 / b.max(1);
        } else {
            total += part.parse::<i64>().ok()? * 8;
        }
    }
    Some(total.max(1))
}

/// Scene id -> strip id from the current schedule view.
fn strips(g: &Gold) -> (BTreeMap<String, String>, Value) {
    let v = g.ok("schedule.get", json!({}));
    let mut map = BTreeMap::new();
    for st in arr(&v["unscheduled"]) {
        map.insert(s(&st["sceneId"]), s(&st["id"]));
    }
    for d in arr(&v["days"]) {
        for it in arr(&d["items"]) {
            if !it["strip"].is_null() {
                map.insert(s(&it["strip"]["sceneId"]), s(&it["strip"]["id"]));
            }
        }
    }
    (map, v)
}

pub fn run(g: &Gold, ctx: &mut Ctx) {
    g.phase("Shooting schedule");
    let sch = ctx.production["schedule"].clone();
    // The line producer's rough one-liner was scrapped before the 1st AD started.
    let rough = g.id(&g.ok("schedule.create", json!({ "name": "Rough one-liner (line producer)" })));
    g.ok("schedule.delete", json!({ "scheduleId": rough }));
    let created = g.ok("schedule.create", json!({ "name": sch["name"] }));
    let sid = g.id(&created);
    ctx.set("schedule", sid.clone());
    g.ok(
        "schedule.update_settings",
        json!({ "scheduleId": sid, "name": sch["name"], "strictValidation": false, "dayDurationMinutes": sch["dayDurationMinutes"],
                "doodCodeStyle": "industry", "expectedRev": null }),
    );
    let (strip_of, _) = strips(g);
    let mut prev: Option<String> = None;
    for d in arr(&sch["days"]) {
        let n = d["n"].as_u64().unwrap_or(0) as u32;
        let off = d["off"].as_bool().unwrap_or(false);
        let id = g.id(&g.ok(
            "schedule.create_day",
            json!({ "scheduleId": sid, "date": d["date"], "notes": d["notes"], "offDay": off, "afterDayId": prev }),
        ));
        prev = Some(id.clone());
        ctx.days.insert(n, id.clone());
        if off {
            continue;
        }
        g.ok(
            "schedule.set_day_times",
            json!({ "dayId": id, "crewCall": clock(&s(&d["crewCall"])), "shootingStart": clock(&s(&d["start"])), "targetWrap": clock(&s(&d["wrap"])) }),
        );
        let ids: Vec<String> = arr(&d["scenes"])
            .iter()
            .filter_map(|x| ctx.scenes.get(&(x.as_u64().unwrap_or(0) as u32)))
            .filter_map(|sc| strip_of.get(sc).cloned())
            .collect();
        g.ok("schedule.move_strips", json!({ "stripIds": ids, "dayId": id, "keepAnyway": true }));
        for m in arr(&d["markers"]) {
            g.ok(
                "schedule.add_marker",
                json!({ "dayId": id, "markerType": m["type"], "label": m["label"], "atTime": m["at"], "durationMinutes": m["minutes"],
                        "notes": null, "index": null }),
            );
        }
    }
    // Page counts and time estimates from the 1st AD's breakdown sheets.
    for sc in arr(&ctx.breakdown["scenes"]) {
        let n = sc["n"].as_u64().unwrap_or(0) as u32;
        let Some(strip) = ctx.scenes.get(&n).and_then(|id| strip_of.get(id)) else { continue };
        if let Some(e) = eighths(&s(&sc["pages"])) {
            g.ok("schedule.set_strip_pages", json!({ "stripId": strip, "eighths": e }));
        }
        if let Some(m) = sc["estimateMinutes"].as_i64() {
            g.ok("schedule.set_strip_estimate", json!({ "stripId": strip, "minutes": m }));
        }
    }
    // Fixed start times where the sun or the tide decides.
    for (n, at) in [(5u32, "05:50"), (44, "08:00"), (43, "05:15")] {
        if let Some(strip) = ctx.scenes.get(&n).and_then(|id| strip_of.get(id)) {
            g.ok("schedule.set_strip_start", json!({ "stripId": strip, "minutes": clock(at) }));
        }
    }
    g.ok("schedule.set_day_target", json!({ "dayId": ctx.days[&9], "minutes": 720 }));
    // Long days on single-location blocks, with overtime in the budget.
    for (n, m) in [(2u32, 720), (4, 750), (5, 750)] {
        g.ok("schedule.set_day_target", json!({ "dayId": ctx.days[&n], "minutes": m }));
    }

    // Corrections, as the 1st AD makes them.
    let view = g.ok("schedule.get", json!({}));
    ctx.facts.insert("scheduleWarningsBefore".into(), json!(arr(&view["warnings"]).iter().map(|w| s(&w["kind"])).collect::<Vec<_>>()));
    for mv in arr(&sch["moves"]) {
        let n = mv["scene"].as_u64().unwrap_or(0) as u32;
        let to = mv["toDay"].as_u64().unwrap_or(0) as u32;
        if let (Some(strip), Some(day)) = (ctx.scenes.get(&n).and_then(|id| strip_of.get(id)), ctx.days.get(&to)) {
            g.ok("schedule.move_strip", json!({ "stripId": strip, "dayId": day, "index": 0, "keepAnyway": true }));
        }
    }
    // Scene 35 (Suresh's house) moved to the end of day 11 after the church ran long.
    if let Some(strip) = ctx.scenes.get(&35).and_then(|id| strip_of.get(id)) {
        g.ok("schedule.move_strip", json!({ "stripId": strip, "dayId": ctx.days[&11], "index": null, "keepAnyway": false }));
    }
    // The rest day moved for a read of the schedule and moved back.
    g.ok("schedule.move_day", json!({ "dayId": ctx.days[&7], "index": 7 }));
    g.ok("schedule.move_day", json!({ "dayId": ctx.days[&7], "index": 6 }));
    g.ok("schedule.set_off_day", json!({ "dayId": ctx.days[&14], "offDay": true }));
    g.ok("schedule.set_day_notes", json!({ "dayId": ctx.days[&14], "notes": "Rest day. Port Trust inspection of the station. Art department preps the demolition." }));
    // Day 12 lunch moved earlier, then the hospital move gets a note; a duplicate marker removed.
    let v = g.ok("schedule.daily", json!({ "dayId": ctx.days[&12] }));
    let _ = v;
    let markers = all_markers(g, &ctx.days[&12]);
    if let Some(lunch) = markers.iter().find(|m| s(&m["markerType"]) == "Meal") {
        g.ok(
            "schedule.update_marker",
            json!({ "markerId": lunch["id"], "markerType": "Meal", "label": "Lunch (studio canteen)", "atTime": "13:30", "durationMinutes": 45, "notes": "Canteen booked for 85." }),
        );
        g.ok("schedule.move_marker", json!({ "markerId": lunch["id"], "dayId": ctx.days[&12], "index": 2 }));
    }
    let extra = g.id(&g.ok(
        "schedule.add_marker",
        json!({ "dayId": ctx.days[&12], "markerType": "Meal", "label": "Lunch", "atTime": "14:00", "durationMinutes": 45, "notes": null, "index": null }),
    ));
    g.ok("schedule.delete_marker", json!({ "markerId": extra }));

    // Warnings: the child performer's night work is kept on purpose; the rest reviewed.
    let view = g.ok("schedule.get", json!({}));
    ctx.facts.insert("scheduleWarnings".into(), json!(arr(&view["warnings"]).iter().map(|w| format!("{}: {} {}", s(&w["kind"]), s(&w["message"]), s(&w["detail"]))).collect::<Vec<_>>()));
    decide_warnings(g, &sid, &view);
    if let Some(w) = arr(&view["warnings"]).first() {
        g.ok("schedule.decide_warning", json!({ "scheduleId": sid, "key": w["key"], "decision": null }));
        g.ok("schedule.decide_warning", json!({ "scheduleId": sid, "key": w["key"], "decision": "Kept" }));
    }
    g.ok("schedule.suggestions", json!({}));

    // Day Out of Days with the 1st AD's corrections.
    let dood = g.ok("schedule.dood", json!({}));
    let keys: Vec<(String, String)> = arr(&dood["rows"]).iter().chain(arr(&dood["cast"]).iter())
        .map(|r| (s(&r["castKey"]).to_string() + &s(&r["key"]), s(&r["name"]).to_string() + &s(&r["character"]))).collect();
    let key_of = |name: &str| keys.iter().find(|(_, n)| n.to_uppercase().contains(name)).map(|(k, _)| k.clone());
    if let Some(k) = key_of("ANJALI") {
        g.ok("schedule.set_dood_override", json!({ "scheduleId": sid, "castKey": k, "dayId": ctx.days[&7], "code": "R", "note": "Tape-baking rehearsal with the sound department." }));
    }
    if let Some(k) = key_of("MARIAMMA") {
        g.ok("schedule.set_dood_override", json!({ "scheduleId": sid, "castKey": k, "dayId": ctx.days[&1], "code": "F", "note": "Costume fitting, 1999 saris." }));
        g.ok("schedule.set_dood_override", json!({ "scheduleId": sid, "castKey": k, "dayId": ctx.days[&1], "code": null, "note": null }));
        g.ok("schedule.set_dood_override", json!({ "scheduleId": sid, "castKey": k, "dayId": ctx.days[&1], "code": "F", "note": "Costume fitting, 1999 saris (afternoon)." }));
    }
    if let Some(k) = key_of("STUDIO PRODUCER") {
        g.ok("schedule.set_dood_override", json!({ "scheduleId": sid, "castKey": k, "dayId": ctx.days[&12], "code": "T", "note": "Travels from Bengaluru." }));
    }
    let dood = g.ok("schedule.dood", json!({ "fromDayId": ctx.days[&1], "toDayId": ctx.days[&8] }));
    let _ = dood;
    g.ok("schedule.set_status", json!({ "scheduleId": sid, "status": "Active" }));
    let v = g.ok("schedule.get", json!({}));
    ctx.facts.insert("schedule".into(), json!({ "days": arr(&v["days"]).len(), "unscheduled": arr(&v["unscheduled"]).len(), "counts": v["counts"] }));
    for (n, id) in ctx.days.clone() {
        if n == 1 || n == 9 {
            g.ok("schedule.daily", json!({ "dayId": id }));
        }
    }
}

/// The 1st AD's decisions: long days and planned company moves are kept; a cast member marked
/// unavailable on a day where only their recorded voice plays back is kept as well.
fn decide_warnings(g: &Gold, sid: &str, view: &Value) {
    for w in arr(&view["warnings"]) {
        if !w["decision"].is_null() {
            continue;
        }
        let decision = match s(&w["kind"]).as_str() {
            "duration_overflow" | "travel_overflow" | "actor_conflict" | "cast_unavailable" => "Kept",
            _ => "Dismissed",
        };
        g.ok("schedule.decide_warning", json!({ "scheduleId": sid, "key": w["key"], "decision": decision }));
    }
}

fn all_markers(g: &Gold, day: &str) -> Vec<Value> {
    let v = g.ok("schedule.get", json!({}));
    arr(&v["days"])
        .iter()
        .filter(|d| s(&d["id"]) == day)
        .flat_map(|d| arr(&d["items"]))
        .filter(|it| !it["marker"].is_null())
        .map(|it| it["marker"].clone())
        .collect()
}

pub fn paperwork(g: &Gold, ctx: &mut Ctx) {
    g.phase("Call sheets");
    let header = ctx.production["header"].clone();
    let days: Vec<(u32, String)> = ctx.days.iter().map(|(n, id)| (*n, id.clone())).collect();
    let sch_days = arr(&ctx.production["schedule"]["days"]);
    let total = sch_days.iter().filter(|d| !d["off"].as_bool().unwrap_or(false)).count();
    let mut shoot_no = 0;
    // The house call sheet template, used for the demolition day.
    let tpl = g.ok("templates.create", json!({ "scope": "project", "templateType": "call_sheet", "name": "Low Tide Radio call sheet",
        "content": { "description": "House style: weather, crew, walkie channels and the marine safety line.", "sections": ["weather", "crew", "communication"],
                     "practical": { "safety": "Life jackets near water. Report any near miss to the 1st AD.", "meals": "Hot meals at unit base." } } }));
    let tpl_id = g.id(&tpl);
    ctx.set("templateCallSheet", tpl_id.clone());
    for (n, day) in days {
        let spec = sch_days.iter().find(|d| d["n"].as_u64() == Some(n as u64)).cloned().unwrap_or(Value::Null);
        if spec["off"].as_bool().unwrap_or(false) {
            continue;
        }
        shoot_no += 1;
        let cs = if n == 16 {
            let applied = g.ok("templates.apply", json!({ "scope": "project", "id": tpl_id, "dayId": day }));
            s(&applied["nav"]["callSheetId"])
        } else {
            g.id(&g.ok("callsheets.create", json!({ "dayId": day })))
        };
        if cs.is_empty() {
            continue;
        }
        ctx.call_sheets.insert(n, cs.clone());
        for section in ["weather", "extras", "departments", "advance", "crew", "communication", "specialNotes", "attachments", "referenceImages"] {
            g.ok("callsheets.add_section", json!({ "id": cs, "section": section }));
        }
        let got = g.ok("callsheets.get", json!({ "id": cs }));
        let mut doc = got["document"].clone();
        doc["title"] = json!("LOW TIDE RADIO");
        doc["header"] = header.clone();
        doc["dayOf"] = json!(format!("Day {shoot_no} of {total}"));
        doc["crewCall"] = spec["crewCall"].clone();
        doc["dayNotes"] = spec["notes"].clone();
        doc["times"] = json!({ "generalCall": spec["crewCall"], "shootingCall": spec["start"], "breakfast": "On arrival, unit base",
            "lunch": arr(&spec["markers"]).iter().find(|m| s(&m["type"]) == "Meal").map(|m| s(&m["at"])).unwrap_or_default(),
            "estimatedWrap": spec["wrap"] });
        if let Some(locs) = doc["locations"].as_array_mut() {
            for l in locs.iter_mut() {
                if s(&l["meetingPoint"]).is_empty() {
                    l["meetingPoint"] = json!("Unit base; follow the yellow OPENFRAME LTR signs");
                }
                if s(&l["parking"]).is_empty() {
                    l["parking"] = json!("Crew cars at unit base only");
                }
            }
        }
        if let Some(cast) = doc["cast"].as_array_mut() {
            for (i, c) in cast.iter_mut().enumerate() {
                if s(&c["castId"]).is_empty() {
                    c["castId"] = json!((i + 1).to_string());
                }
                c["pickup"] = json!("Unit hotel, 60 minutes before call");
                c["makeup"] = json!("30 minutes before call");
            }
        }
        doc["practical"]["safety"] = json!("Report any accident or near miss to the 1st AD straight away. Life jackets near water. Set medic: Dr. Asha Kurian, +91 98470 40024.");
        doc["practical"]["emergencyContact"] = json!("Gopan Nair (UPM) +91 98470 20114");
        doc["practical"]["mealBreak"] = json!("Hot meals at unit base. Vegetarian and non-vegetarian.");
        doc["practical"]["travelNotes"] = json!("Unit buses leave the Fort Kochi hotel 45 minutes before crew call.");
        doc["practical"]["specialInstructions"] = json!(if n == 9 { "Storm night: rain towers, wave machine. Harness past the yellow marker. Ambulance at the harbour road." } else { "Phones on silent on set." });
        let opt = &mut doc["optional"];
        opt["weather"] = json!(if n == 9 { "Clear night, wind 12 knots. All rain is ours." } else { "Dry, 24 to 31 C, humidity 70 percent." });
        opt["sunrise"] = json!("06:42");
        opt["sunset"] = json!("18:21");
        opt["specialNotes"] = json!("No visitors on set without the producer's approval.");
        opt["communication"] = json!({ "walkieChannels": "1 production, 2 camera, 3 grip and electric, 5 marine", "emergencyContact": "Dr. Asha Kurian +91 98470 40024", "productionContact": "Gopan Nair +91 98470 20114", "note": "" });
        opt["departments"] = json!([
            { "department": "Sound", "notes": "Playback cues from Farhan. Nagra on set for scenes 13, 16 and 33." },
            { "department": "Art", "notes": "Station logbook and hero reel with the props master." }
        ]);
        opt["crew"] = json!([
            { "key": "1", "name": "Priya Ramesh", "role": "First Assistant Director", "department": "Direction", "callTime": spec["crewCall"], "contact": "+91 98470 40005" },
            { "key": "2", "name": "Vivek Anand", "role": "Director of Photography", "department": "Camera", "callTime": spec["crewCall"], "contact": "+91 98470 40008" },
            { "key": "3", "name": "Farhan Ali", "role": "Production Sound Mixer", "department": "Sound", "callTime": spec["crewCall"], "contact": "+91 98470 40016" }
        ]);
        if n == 10 {
            opt["extras"] = json!([{ "key": "1", "description": "Villagers for the listening night", "quantity": "300", "callTime": "16:00", "onSet": "18:30", "notes": "Holding tent at the resort car park. Fed before rehearsal." }]);
        }
        let next = sch_days.iter().skip_while(|d| d["n"].as_u64() != Some(n as u64)).nth(1).cloned();
        if let Some(nx) = next {
            opt["advance"] = json!({ "day": format!("Next: {}", s(&nx["date"])), "location": s(&nx["notes"]), "cast": "", "props": "", "wardrobe": "", "preparation": "" });
        }
        doc["extraFields"] = json!([{ "label": "Production number", "value": "KP-2027-04" }]);
        g.ok("callsheets.update", json!({ "id": cs, "document": doc, "expectedRev": got["rev"] }));
        if n == 1 || n == 9 {
            g.ok("callsheets.attach", json!({ "id": cs, "section": "attachments", "path": ctx.media.path("location-release-coast-radio-station.pdf") }));
            g.ok("callsheets.attach", json!({ "id": cs, "section": "referenceImages", "path": ctx.media.path(if n == 9 { "board-breakwater-01.png" } else { "station-day.png" }) }));
        }
        g.ok("callsheets.finalize", json!({ "id": cs, "acknowledgeStale": false }));
    }
    // A pickup day for the mast fall was pencilled in with its own call sheet, then cancelled.
    let pickup = g.id(&g.ok("schedule.duplicate_day", json!({ "dayId": ctx.days[&16] })));
    g.ok("schedule.set_day_date", json!({ "dayId": pickup, "date": "2027-01-27" }));
    let pickup_sheet = g.id(&g.ok("callsheets.create", json!({ "dayId": pickup, "template": { "scope": "project", "id": tpl_id } })));
    g.ok("schedule.set_day_notes", json!({ "dayId": pickup, "notes": "Cancelled: the mast fall was clean in one take." }));
    if !pickup_sheet.is_empty() {
        g.ok("callsheets.delete", json!({ "id": pickup_sheet }));
    }
    g.ok("schedule.delete_day", json!({ "dayId": pickup }));
    // Storm day: the weather changed after the sheet went out. A revision is issued.
    let storm = ctx.call_sheets[&9].clone();
    let rev = g.id(&g.ok("callsheets.new_revision", json!({ "id": storm })));
    if !rev.is_empty() {
        let got = g.ok("callsheets.get", json!({ "id": rev }));
        let mut doc = got["document"].clone();
        doc["optional"]["weather"] = json!("Real rain forecast after 23:00 (IMD yellow alert). Rain towers stand by; wrap the breakwater by 01:30.");
        doc["times"]["estimatedWrap"] = json!("01:30");
        g.ok("callsheets.update", json!({ "id": rev, "document": doc, "expectedRev": got["rev"] }));
        g.ok("callsheets.finalize", json!({ "id": rev, "acknowledgeStale": false }));
        ctx.set("callSheetStormRevision", rev);
    }
    g.ok("callsheets.list", json!({}));

    g.phase("Sides");
    let day1 = ctx.days[&1].clone();
    let preview = g.ok("sides.preview", json!({ "dayId": day1, "stripIds": [], "castKeys": null, "includeCover": true, "showDraftName": true, "title": null }));
    let _ = preview;
    let sides1 = g.id(&g.ok("sides.create", json!({ "dayId": day1, "stripIds": [], "castKeys": null, "includeCover": true, "showDraftName": true, "title": "Day 1 sides" })));
    ctx.set("sidesDay1", sides1.clone());
    let d9 = ctx.days[&9].clone();
    let dood = g.ok("schedule.dood", json!({}));
    let suresh_key = arr(&dood["rows"]).iter().chain(arr(&dood["cast"]).iter())
        .find(|r| (s(&r["name"]) + &s(&r["character"])).to_uppercase().contains("SURESH"))
        .map(|r| s(&r["castKey"]) + &s(&r["key"]));
    let sides9 = g.id(&g.ok("sides.create", json!({ "dayId": d9, "stripIds": [], "castKeys": suresh_key.clone().map(|k| vec![k]), "includeCover": true, "showDraftName": true, "title": "Day 9 sides for Sajan (SURESH)" })));
    ctx.set("sidesDay9", sides9);
    let tmp = g.id(&g.ok("sides.create", json!({ "dayId": d9, "stripIds": [], "castKeys": null, "includeCover": false, "showDraftName": false, "title": "draft" })));
    g.ok("sides.delete", json!({ "id": tmp }));
    g.ok("sides.list", json!({}));
    g.ok("sides.get", json!({ "id": sides1 }));

    g.phase("Production reports");
    for (ty, title) in [
        ("scene", "Scene report"),
        ("location", "Location report"),
        ("cast_scene", "Cast by scene"),
        ("prop", "Props report"),
        ("schedule", "Schedule report"),
        ("breakdown_completeness", "Breakdown completeness"),
    ] {
        g.ok("reports.generate", json!({ "reportType": ty, "title": title, "filter": null }));
        let id = g.id(&g.ok("reports.save", json!({ "reportType": ty, "title": format!("{title}, white pages"), "filter": null })));
        ctx.set(&format!("report:{ty}"), id);
    }
    g.ok("reports.generate", json!({ "reportType": "prop", "title": "Props: the reel", "filter": "reel" }));
    let tmp = g.id(&g.ok("reports.save", json!({ "reportType": "location", "title": "temp", "filter": "Vypin" })));
    g.ok("reports.delete", json!({ "id": tmp }));
    g.ok("reports.list", json!({}));
    g.ok("reports.get", json!({ "id": ctx.id("report:scene") }));
}

/// After the blue pages: reconcile, acknowledge the changed strips, refresh the call sheets.
pub fn after_revision(g: &Gold, ctx: &mut Ctx) {
    g.phase("Schedule after the blue pages");
    let sid = ctx.id("schedule");
    let r = g.ok("schedule.reconcile", json!({ "scheduleId": sid }));
    ctx.facts.insert("reconcile".into(), r);
    let (strip_of, view) = strips(g);
    // Scene 23A goes to day 9 with the rest of the storm.
    if let Some(strip) = strip_of.get(&ctx.id("scene23A")) {
        g.ok("schedule.move_strip", json!({ "stripId": strip, "dayId": ctx.days[&9], "index": 1, "keepAnyway": true }));
        g.ok("schedule.set_strip_estimate", json!({ "stripId": strip, "minutes": 75 }));
        g.ok("schedule.set_strip_pages", json!({ "stripId": strip, "eighths": 4 }));
    }
    for d in arr(&view["days"]) {
        for it in arr(&d["items"]) {
            let st = &it["strip"];
            if st.is_null() {
                continue;
            }
            match s(&st["sourceState"]).as_str() {
                "Changed" | "New" => {
                    g.ok("schedule.acknowledge_change", json!({ "stripId": st["id"] }));
                }
                "Removed" => {
                    g.ok("schedule.confirm_removal", json!({ "stripId": st["id"] }));
                }
                _ => {}
            }
        }
    }
    // Every call sheet the blue pages made stale is reissued as a revision: a finalized sheet is
    // never edited in place.
    let v = g.ok("schedule.get", json!({}));
    let stale_days: Vec<String> = arr(&v["warnings"]).iter().filter(|w| s(&w["kind"]) == "call_sheet_stale").map(|w| s(&w["dayId"])).collect();
    let storm = ctx.id("callSheetStormRevision");
    let mut reissued = 0;
    for (n, cs) in ctx.call_sheets.clone() {
        let day = ctx.days.get(&n).cloned().unwrap_or_default();
        let pv = g.ok("callsheets.refresh_preview", json!({ "id": cs }));
        let _ = pv;
        if !stale_days.contains(&day) {
            continue;
        }
        let base = if n == 9 && !storm.is_empty() { storm.clone() } else { cs.clone() };
        let rev = g.id(&g.ok("callsheets.new_revision", json!({ "id": base })));
        if !rev.is_empty() {
            g.ok("callsheets.refresh", json!({ "id": rev }));
            g.ok("callsheets.finalize", json!({ "id": rev, "acknowledgeStale": false }));
            ctx.call_sheets.insert(n, rev);
            reissued += 1;
        }
    }
    ctx.facts.insert("callSheetsReissuedForBluePages".into(), json!(reissued));
    let sides = g.id(&g.ok("sides.create", json!({ "dayId": ctx.days[&9], "stripIds": [], "castKeys": null, "includeCover": true, "showDraftName": true, "title": "Day 9 sides, blue pages" })));
    ctx.set("sidesDay9Blue", sides);
    let v = g.ok("schedule.get", json!({}));
    decide_warnings(g, &sid, &v);
    let v = g.ok("schedule.get", json!({}));
    ctx.facts.insert("scheduleWarningsAfterBlue".into(), json!(arr(&v["warnings"]).iter().map(|w| format!("{} [{}]: {}", s(&w["kind"]), s(&w["decision"]), s(&w["message"]))).collect::<Vec<_>>()));
    ctx.facts.insert("scheduleAfterBlue".into(), json!({ "days": arr(&v["days"]).len(), "unscheduled": arr(&v["unscheduled"]).len(), "counts": v["counts"], "openWarnings": v["openWarningCount"] }));
}
