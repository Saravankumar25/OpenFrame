//! The shooting schedule, Day Out of Days, call sheets, sides and the production reports, all from
//! `production.json`.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::Ctx;
use crate::gold::{Gold, arr, s};

fn clock(t: &str) -> Value {
    let mut it = t.split(':');
    match (
        it.next().and_then(|h| h.parse::<i64>().ok()),
        it.next().and_then(|m| m.parse::<i64>().ok()),
    ) {
        (Some(h), Some(m)) => json!(h * 60 + m),
        _ => Value::Null,
    }
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

fn day_id(ctx: &Ctx, n: &Value) -> String {
    ctx.days
        .get(&(n.as_u64().unwrap_or(0) as u32))
        .cloned()
        .unwrap_or_default()
}

/// Long days and planned moves are kept; other warnings are reviewed and dismissed.
fn decide_warnings(g: &Gold, sid: &str, view: &Value) {
    for w in arr(&view["warnings"]) {
        if !w["decision"].is_null() {
            continue;
        }
        let decision = match s(&w["kind"]).as_str() {
            "duration_overflow" | "travel_overflow" | "actor_conflict" | "cast_unavailable" => {
                "Kept"
            }
            _ => "Dismissed",
        };
        g.ok(
            "schedule.decide_warning",
            json!({ "scheduleId": sid, "key": w["key"], "decision": decision }),
        );
    }
}

fn markers_of(g: &Gold, day: &str) -> Vec<Value> {
    let v = g.ok("schedule.get", json!({}));
    arr(&v["days"])
        .iter()
        .filter(|d| s(&d["id"]) == day)
        .flat_map(|d| arr(&d["items"]))
        .filter(|it| !it["marker"].is_null())
        .map(|it| it["marker"].clone())
        .collect()
}

pub fn run(g: &Gold, ctx: &mut Ctx) {
    g.phase("Shooting schedule");
    let sch = ctx.production["schedule"].clone();
    let rough = g.id(&g.ok("schedule.create", json!({ "name": "Rough one-liner" })));
    g.ok("schedule.delete", json!({ "scheduleId": rough }));
    let sid = g.id(&g.ok("schedule.create", json!({ "name": sch["name"] })));
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
        g.ok(
            "schedule.move_strips",
            json!({ "stripIds": ids, "dayId": id, "keepAnyway": true }),
        );
        for m in arr(&d["markers"]) {
            g.ok(
                "schedule.add_marker",
                json!({ "dayId": id, "markerType": m["type"], "label": m["label"], "atTime": m["at"], "durationMinutes": m["minutes"],
                        "notes": null, "index": null }),
            );
        }
    }
    for sc in arr(&ctx.breakdown["scenes"]) {
        let Some(strip) = strip_of.get(&ctx.scene(&sc["n"])) else {
            continue;
        };
        g.ok(
            "schedule.set_strip_pages",
            json!({ "stripId": strip, "eighths": sc["eighths"] }),
        );
        g.ok(
            "schedule.set_strip_estimate",
            json!({ "stripId": strip, "minutes": sc["estimateMinutes"] }),
        );
    }
    for f in arr(&sch["fixedStarts"]) {
        if let Some(strip) = strip_of.get(&ctx.scene(&f["scene"])) {
            g.ok(
                "schedule.set_strip_start",
                json!({ "stripId": strip, "minutes": clock(&s(&f["at"])) }),
            );
        }
    }
    for l in arr(&sch["longDays"]) {
        g.ok(
            "schedule.set_day_target",
            json!({ "dayId": day_id(ctx, &l["day"]), "minutes": l["minutes"] }),
        );
    }
    for mv in arr(&sch["moves"]) {
        if let Some(strip) = strip_of.get(&ctx.scene(&mv["scene"])) {
            g.ok(
                "schedule.move_strip",
                json!({ "stripId": strip, "dayId": day_id(ctx, &mv["toDay"]), "index": 0, "keepAnyway": true }),
            );
        }
    }
    let fo = &sch["focus"];
    if let Some(strip) = strip_of.get(&ctx.scene(&fo["longMoveScene"])) {
        g.ok(
            "schedule.move_strip",
            json!({ "stripId": strip, "dayId": day_id(ctx, &fo["longMoveDay"]), "index": null, "keepAnyway": false }),
        );
    }
    // A rest day moved for a read of the schedule and moved back.
    let rest = fo["restMoveDay"].as_u64().unwrap_or(1) as i64;
    let rest_id = day_id(ctx, &fo["restMoveDay"]);
    g.ok(
        "schedule.move_day",
        json!({ "dayId": rest_id, "index": rest }),
    );
    g.ok(
        "schedule.move_day",
        json!({ "dayId": rest_id, "index": rest - 1 }),
    );
    g.ok(
        "schedule.set_off_day",
        json!({ "dayId": rest_id, "offDay": true }),
    );
    g.ok(
        "schedule.set_day_notes",
        json!({ "dayId": rest_id, "notes": "Rest day. Invented for this demo." }),
    );
    // One lunch moved, and a duplicate marker removed.
    let marker_day = day_id(ctx, &fo["extraMarkerDay"]);
    g.ok("schedule.daily", json!({ "dayId": marker_day }));
    if let Some(lunch) = markers_of(g, &marker_day)
        .iter()
        .find(|m| s(&m["markerType"]) == "Meal")
    {
        g.ok(
            "schedule.update_marker",
            json!({ "markerId": lunch["id"], "markerType": "Meal", "label": "Lunch (late)", "atTime": "13:30", "durationMinutes": 45, "notes": "Moved for a long setup." }),
        );
        g.ok(
            "schedule.move_marker",
            json!({ "markerId": lunch["id"], "dayId": marker_day, "index": 0 }),
        );
    }
    let extra = g.id(&g.ok(
        "schedule.add_marker",
        json!({ "dayId": marker_day, "markerType": "Meal", "label": "Lunch", "atTime": "14:00", "durationMinutes": 45, "notes": null, "index": null }),
    ));
    g.ok("schedule.delete_marker", json!({ "markerId": extra }));

    let view = g.ok("schedule.get", json!({}));
    ctx.facts.insert(
        "scheduleWarnings".into(),
        json!(
            arr(&view["warnings"])
                .iter()
                .map(|w| format!("{}: {}", s(&w["kind"]), s(&w["message"])))
                .collect::<Vec<_>>()
        ),
    );
    decide_warnings(g, &sid, &view);
    if let Some(w) = arr(&view["warnings"]).first() {
        g.ok(
            "schedule.decide_warning",
            json!({ "scheduleId": sid, "key": w["key"], "decision": null }),
        );
        g.ok(
            "schedule.decide_warning",
            json!({ "scheduleId": sid, "key": w["key"], "decision": "Kept" }),
        );
    }
    g.ok("schedule.suggestions", json!({}));

    let dood = g.ok("schedule.dood", json!({}));
    let keys: Vec<(String, String)> = arr(&dood["rows"])
        .iter()
        .chain(arr(&dood["cast"]).iter())
        .map(|r| {
            (
                s(&r["castKey"]) + &s(&r["key"]),
                s(&r["name"]) + &s(&r["character"]),
            )
        })
        .collect();
    ctx.facts.insert("doodRows".into(), json!(keys.len()));
    let key_of = |name: &str| {
        keys.iter()
            .find(|(_, n)| n.to_uppercase().contains(name))
            .map(|(k, _)| k.clone())
    };
    if let Some(k) = key_of(&s(&fo["rehearsalKey"])) {
        g.ok(
            "schedule.set_dood_override",
            json!({ "scheduleId": sid, "castKey": k, "dayId": day_id(ctx, &fo["rehearsalDay"]), "code": "R", "note": "Rehearsal, invented." }),
        );
    }
    if let Some(k) = key_of(&s(&fo["fittingKey"])) {
        let day = day_id(ctx, &fo["fittingDay"]);
        g.ok("schedule.set_dood_override", json!({ "scheduleId": sid, "castKey": k, "dayId": day, "code": "F", "note": "Costume fitting, invented." }));
        g.ok(
            "schedule.set_dood_override",
            json!({ "scheduleId": sid, "castKey": k, "dayId": day, "code": null, "note": null }),
        );
        g.ok("schedule.set_dood_override", json!({ "scheduleId": sid, "castKey": k, "dayId": day, "code": "F", "note": "Costume fitting, afternoon, invented." }));
    }
    if let Some(k) = key_of(&s(&fo["travelKey"])) {
        g.ok(
            "schedule.set_dood_override",
            json!({ "scheduleId": sid, "castKey": k, "dayId": day_id(ctx, &fo["travelDay"]), "code": "T", "note": "Travel day, invented." }),
        );
    }
    let week = arr(&fo["firstWeekDays"]);
    g.ok(
        "schedule.dood",
        json!({ "fromDayId": day_id(ctx, &week[0]), "toDayId": day_id(ctx, &week[week.len() - 1]) }),
    );
    g.ok(
        "schedule.set_status",
        json!({ "scheduleId": sid, "status": "Active" }),
    );
    let v = g.ok("schedule.get", json!({}));
    ctx.facts.insert(
        "schedule".into(),
        json!({ "days": arr(&v["days"]).len(), "unscheduled": arr(&v["unscheduled"]).len(), "counts": v["counts"] }),
    );
    g.ok(
        "schedule.daily",
        json!({ "dayId": day_id(ctx, &fo["dailyDay"]) }),
    );
}

fn extras_for_day(ctx: &Ctx, spec: &Value) -> Vec<Value> {
    let scenes = arr(&spec["scenes"]);
    let mut out = Vec::new();
    for sc in arr(&ctx.breakdown["scenes"]) {
        if !scenes.iter().any(|n| n.as_u64() == sc["n"].as_u64()) {
            continue;
        }
        for e in arr(&sc["elements"]) {
            if s(&e["category"]) == "Extras / Background" {
                out.push(json!({ "key": (out.len() + 1).to_string(), "description": e["name"], "quantity": s(&e["notes"]),
                    "callTime": spec["crewCall"], "onSet": spec["start"], "notes": format!("Scene {}", sc["n"]) }));
            }
        }
    }
    out
}

pub fn paperwork(g: &Gold, ctx: &mut Ctx) {
    g.phase("Call sheets");
    let header = ctx.production["header"].clone();
    let cfg = ctx.production["callSheet"].clone();
    let sch = ctx.production["schedule"].clone();
    let fo = sch["focus"].clone();
    let days: Vec<(u32, String)> = ctx.days.iter().map(|(n, id)| (*n, id.clone())).collect();
    let sch_days = arr(&sch["days"]);
    let total = sch_days
        .iter()
        .filter(|d| !d["off"].as_bool().unwrap_or(false))
        .count();
    let tpl = g.ok(
        "templates.create",
        json!({ "scope": "project", "templateType": "call_sheet", "name": cfg["templateName"],
                "content": { "description": cfg["templateDescription"], "sections": ["weather", "crew", "communication"],
                             "practical": { "safety": cfg["templateSafety"], "meals": cfg["templateMeals"] } } }),
    );
    let tpl_id = g.id(&tpl);
    let template_day = fo["templateDay"].as_u64().unwrap_or(0) as u32;
    let attach_days: Vec<u64> = arr(&fo["attachDays"])
        .iter()
        .filter_map(Value::as_u64)
        .collect();
    let mut shoot_no = 0;
    for (n, day) in days {
        let spec = sch_days
            .iter()
            .find(|d| d["n"].as_u64() == Some(n as u64))
            .cloned()
            .unwrap_or(Value::Null);
        if spec["off"].as_bool().unwrap_or(false) {
            continue;
        }
        shoot_no += 1;
        let cs = if n == template_day {
            let applied = g.ok(
                "templates.apply",
                json!({ "scope": "project", "id": tpl_id, "dayId": day }),
            );
            s(&applied["nav"]["callSheetId"])
        } else {
            g.id(&g.ok("callsheets.create", json!({ "dayId": day })))
        };
        if cs.is_empty() {
            continue;
        }
        ctx.call_sheets.insert(n, cs.clone());
        for section in [
            "weather",
            "extras",
            "departments",
            "advance",
            "crew",
            "communication",
            "specialNotes",
            "attachments",
            "referenceImages",
        ] {
            g.ok(
                "callsheets.add_section",
                json!({ "id": cs, "section": section }),
            );
        }
        let got = g.ok("callsheets.get", json!({ "id": cs }));
        let mut doc = got["document"].clone();
        doc["title"] = cfg["title"].clone();
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
                    l["meetingPoint"] = cfg["meetingPoint"].clone();
                }
                if s(&l["parking"]).is_empty() {
                    l["parking"] = cfg["parking"].clone();
                }
            }
        }
        if let Some(cast) = doc["cast"].as_array_mut() {
            for (i, c) in cast.iter_mut().enumerate() {
                if s(&c["castId"]).is_empty() {
                    c["castId"] = json!((i + 1).to_string());
                }
                c["pickup"] = cfg["pickup"].clone();
                c["makeup"] = cfg["makeup"].clone();
            }
        }
        doc["practical"]["safety"] = cfg["safety"].clone();
        doc["practical"]["emergencyContact"] = cfg["emergency"].clone();
        doc["practical"]["mealBreak"] = cfg["mealBreak"].clone();
        doc["practical"]["travelNotes"] = cfg["travel"].clone();
        let special = arr(&spec["scenes"])
            .iter()
            .find_map(|sc| cfg["special"].get(sc.to_string()).cloned())
            .unwrap_or_else(|| cfg["normal"].clone());
        doc["practical"]["specialInstructions"] = special;
        let opt = &mut doc["optional"];
        opt["weather"] = cfg["weatherNormal"].clone();
        opt["sunrise"] = cfg["sunrise"].clone();
        opt["sunset"] = cfg["sunset"].clone();
        opt["specialNotes"] = cfg["specialNotes"].clone();
        opt["communication"] = cfg["communication"].clone();
        opt["departments"] = cfg["departments"].clone();
        let crew: Vec<Value> = arr(&cfg["crew"])
            .into_iter()
            .map(|mut c| {
                c["callTime"] = spec["crewCall"].clone();
                c
            })
            .collect();
        opt["crew"] = json!(crew);
        let extras = extras_for_day(ctx, &spec);
        if !extras.is_empty() {
            opt["extras"] = json!(extras);
        }
        let next = sch_days
            .iter()
            .skip_while(|d| d["n"].as_u64() != Some(n as u64))
            .nth(1)
            .cloned();
        if let Some(nx) = next {
            opt["advance"] = json!({ "day": format!("Next: {}", s(&nx["date"])), "location": s(&nx["notes"]), "cast": "", "props": "", "wardrobe": "", "preparation": "" });
        }
        doc["extraFields"] = json!([cfg["extraField"]]);
        g.ok(
            "callsheets.update",
            json!({ "id": cs, "document": doc, "expectedRev": got["rev"] }),
        );
        if attach_days.contains(&(n as u64)) {
            g.ok(
                "callsheets.attach",
                json!({ "id": cs, "section": "attachments", "path": ctx.media_path(&s(&cfg["attach"]["file"])) }),
            );
            g.ok(
                "callsheets.attach",
                json!({ "id": cs, "section": "referenceImages", "path": ctx.media_path(&s(&cfg["attach"]["image"])) }),
            );
        }
        g.ok(
            "callsheets.finalize",
            json!({ "id": cs, "acknowledgeStale": false }),
        );
    }
    // A pickup day pencilled in with its own call sheet, then cancelled.
    let pickup = g.id(&g.ok(
        "schedule.duplicate_day",
        json!({ "dayId": day_id(ctx, &fo["templateDay"]) }),
    ));
    g.ok(
        "schedule.set_day_date",
        json!({ "dayId": pickup, "date": fo["pickupDate"] }),
    );
    let pickup_sheet = g.id(&g.ok(
        "callsheets.create",
        json!({ "dayId": pickup, "template": { "scope": "project", "id": tpl_id } }),
    ));
    g.ok(
        "schedule.set_day_notes",
        json!({ "dayId": pickup, "notes": fo["pickupNote"] }),
    );
    if !pickup_sheet.is_empty() {
        g.ok("callsheets.delete", json!({ "id": pickup_sheet }));
    }
    g.ok("schedule.delete_day", json!({ "dayId": pickup }));
    // The forecast changed after the sheet went out: a revision is issued.
    let rev_day = fo["revisionDay"].as_u64().unwrap_or(0) as u32;
    if let Some(sheet) = ctx.call_sheets.get(&rev_day).cloned() {
        let rev = g.id(&g.ok("callsheets.new_revision", json!({ "id": sheet })));
        if !rev.is_empty() {
            let got = g.ok("callsheets.get", json!({ "id": rev }));
            let mut doc = got["document"].clone();
            doc["optional"]["weather"] = cfg["revisionWeather"].clone();
            doc["times"]["estimatedWrap"] = cfg["revisionWrap"].clone();
            g.ok(
                "callsheets.update",
                json!({ "id": rev, "document": doc, "expectedRev": got["rev"] }),
            );
            g.ok(
                "callsheets.finalize",
                json!({ "id": rev, "acknowledgeStale": false }),
            );
            ctx.set("callSheetRevision", rev);
        }
    }
    g.ok("callsheets.list", json!({}));

    g.phase("Sides");
    let sides_day = day_id(ctx, &fo["sidesDay"]);
    g.ok(
        "sides.preview",
        json!({ "dayId": sides_day, "stripIds": [], "castKeys": null, "includeCover": true, "showDraftName": true, "title": null }),
    );
    let first = g.id(&g.ok(
        "sides.create",
        json!({ "dayId": sides_day, "stripIds": [], "castKeys": null, "includeCover": true, "showDraftName": true, "title": cfg["sidesTitle"] }),
    ));
    ctx.set("sidesFirst", first.clone());
    let cast_day = day_id(ctx, &fo["castSidesDay"]);
    let dood = g.ok("schedule.dood", json!({}));
    let wanted = s(&fo["castSidesKey"]);
    let key = arr(&dood["rows"])
        .iter()
        .chain(arr(&dood["cast"]).iter())
        .find(|r| {
            (s(&r["name"]) + &s(&r["character"]))
                .to_uppercase()
                .contains(&wanted)
        })
        .map(|r| s(&r["castKey"]) + &s(&r["key"]));
    let cast_sides = g.id(&g.ok(
        "sides.create",
        json!({ "dayId": cast_day, "stripIds": [], "castKeys": key.map(|k| vec![k]), "includeCover": true, "showDraftName": true, "title": cfg["castSidesTitle"] }),
    ));
    ctx.set("sidesCast", cast_sides);
    let tmp = g.id(&g.ok(
        "sides.create",
        json!({ "dayId": cast_day, "stripIds": [], "castKeys": null, "includeCover": false, "showDraftName": false, "title": "draft" }),
    ));
    g.ok("sides.delete", json!({ "id": tmp }));
    g.ok("sides.list", json!({}));
    g.ok("sides.get", json!({ "id": first }));

    g.phase("Production reports");
    for r in arr(&ctx.production["reports"]) {
        let ty = s(&r["type"]);
        g.ok(
            "reports.generate",
            json!({ "reportType": ty, "title": r["title"], "filter": null }),
        );
        let id = g.id(&g.ok(
            "reports.save",
            json!({ "reportType": ty, "title": format!("{}, white pages", s(&r["title"])), "filter": null }),
        ));
        ctx.set(&format!("report:{ty}"), id);
    }
    let filter = ctx.breakdown["reportFilterProp"].clone();
    g.ok(
        "reports.generate",
        json!({ "reportType": "prop", "title": "Props filtered", "filter": filter }),
    );
    let tmp = g.id(&g.ok(
        "reports.save",
        json!({ "reportType": "location", "title": "temp", "filter": null }),
    ));
    g.ok("reports.delete", json!({ "id": tmp }));
    g.ok("reports.list", json!({}));
    g.ok("reports.get", json!({ "id": ctx.id("report:scene") }));
}

/// After the revision: reconcile, place the new scene, acknowledge strips, refresh call sheets.
pub fn after_revision(g: &Gold, ctx: &mut Ctx) {
    g.phase("Schedule after the revision");
    let sid = ctx.id("schedule");
    let r = g.ok("schedule.reconcile", json!({ "scheduleId": sid }));
    ctx.facts.insert("reconcile".into(), r);
    let (strip_of, view) = strips(g);
    let b = ctx.script["blue"].clone();
    if let Some(strip) = strip_of.get(&ctx.id("newScene")) {
        g.ok(
            "schedule.move_strip",
            json!({ "stripId": strip, "dayId": day_id(ctx, &b["newSceneDay"]), "index": null, "keepAnyway": true }),
        );
        g.ok(
            "schedule.set_strip_estimate",
            json!({ "stripId": strip, "minutes": b["newScene"]["estimate"] }),
        );
        g.ok(
            "schedule.set_strip_pages",
            json!({ "stripId": strip, "eighths": b["newScene"]["eighths"] }),
        );
    }
    for d in arr(&view["days"]) {
        for it in arr(&d["items"]) {
            let st = &it["strip"];
            if st.is_null() {
                continue;
            }
            match s(&st["sourceState"]).as_str() {
                "Changed" | "New" => {
                    g.ok(
                        "schedule.acknowledge_change",
                        json!({ "stripId": st["id"] }),
                    );
                }
                "Removed" => {
                    g.ok("schedule.confirm_removal", json!({ "stripId": st["id"] }));
                }
                _ => {}
            }
        }
    }
    let v = g.ok("schedule.get", json!({}));
    let stale_days: Vec<String> = arr(&v["warnings"])
        .iter()
        .filter(|w| s(&w["kind"]) == "call_sheet_stale")
        .map(|w| s(&w["dayId"]))
        .collect();
    let rev_base = ctx.id("callSheetRevision");
    let rev_day = ctx.production["schedule"]["focus"]["revisionDay"]
        .as_u64()
        .unwrap_or(0) as u32;
    let mut reissued = 0;
    for (n, cs) in ctx.call_sheets.clone() {
        let day = ctx.days.get(&n).cloned().unwrap_or_default();
        g.ok("callsheets.refresh_preview", json!({ "id": cs }));
        if !stale_days.contains(&day) {
            continue;
        }
        let base = if n == rev_day && !rev_base.is_empty() {
            rev_base.clone()
        } else {
            cs.clone()
        };
        let rev = g.id(&g.ok("callsheets.new_revision", json!({ "id": base })));
        if !rev.is_empty() {
            g.ok("callsheets.refresh", json!({ "id": rev }));
            g.ok(
                "callsheets.finalize",
                json!({ "id": rev, "acknowledgeStale": false }),
            );
            ctx.call_sheets.insert(n, rev);
            reissued += 1;
        }
    }
    ctx.facts
        .insert("callSheetsReissued".into(), json!(reissued));
    let sides = g.id(&g.ok(
        "sides.create",
        json!({ "dayId": day_id(ctx, &b["newSceneDay"]), "stripIds": [], "castKeys": null, "includeCover": true, "showDraftName": true, "title": "Sides, revision pages" }),
    ));
    ctx.set("sidesRevision", sides);
    let v = g.ok("schedule.get", json!({}));
    decide_warnings(g, &sid, &v);
    let v = g.ok("schedule.get", json!({}));
    ctx.facts.insert(
        "scheduleAfterRevision".into(),
        json!({ "days": arr(&v["days"]).len(), "unscheduled": arr(&v["unscheduled"]).len(), "counts": v["counts"], "openWarnings": v["openWarningCount"] }),
    );
}
