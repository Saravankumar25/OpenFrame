//! Production planning: cast, crew, locations and availability; the breakdown of all 44 scenes
//! with the shared catalog; the budget; and what changes when the blue pages arrive.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::Ctx;
use crate::gold::{Gold, arr, s};

fn to_paise(rupees: &Value) -> Value {
    match rupees.as_i64() {
        Some(r) => json!(r * 100),
        None => Value::Null,
    }
}

pub fn people_and_places(g: &Gold, ctx: &mut Ctx) {
    g.phase("Cast, crew and locations");
    let prod = ctx.production.clone();
    for c in arr(&prod["cast"]) {
        let ch = s(&c["character"]);
        let mut args = json!({ "personName": c["person"], "primary": true, "contact": c["contact"],
                               "availabilityNotes": c["availability"], "notes": c["notes"] });
        match ctx.chars.get(&ch) {
            Some(id) => args["characterId"] = json!(id),
            None => args["characterName"] = json!(ch),
        }
        let id = g.id(&g.ok("cast.create", args));
        if let Some(photo) = c["photo"].as_str() {
            g.ok("cast.set_photo", json!({ "id": id, "path": ctx.media.path(photo) }));
        }
        let windows: Vec<Value> = arr(&c["unavailable"])
            .iter()
            .map(|w| json!({ "startDate": w[0], "endDate": w[1], "kind": "Unavailable", "note": w[2] }))
            .collect();
        if !windows.is_empty() {
            g.ok("availability.set", json!({ "ownerType": "cast_member", "ownerId": id, "windows": windows }));
        }
        ctx.cast.insert(ch, id);
    }
    for a in arr(&prod["alternates"]) {
        let ch = s(&a["character"]);
        let id = g.id(&g.ok(
            "cast.create",
            json!({ "personName": a["person"], "characterId": ctx.chars.get(&ch), "primary": false, "contact": a["contact"], "notes": a["notes"] }),
        ));
        ctx.set("castAlternateMariamma", id);
    }
    // The first Port Officer dropped out; his entry is archived and the new actor made primary.
    let dropped = g.id(&g.ok(
        "cast.create",
        json!({ "personName": "Vijay Mohan", "characterName": "PORT OFFICER", "primary": false, "contact": "+91 94470 39999",
                "notes": "Dropped out on 2026-12-15 (film clash)." }),
    ));
    g.ok("cast.assign", json!({ "id": dropped, "characterName": "PORT OFFICER", "primary": true }));
    g.ok("cast.assign", json!({ "id": ctx.cast["PORT OFFICER"], "characterName": "PORT OFFICER", "primary": true }));
    g.ok("cast.set_archived", json!({ "id": dropped, "archived": true }));
    let dup = g.id(&g.ok("cast.create", json!({ "personName": "Nimisha Raghavan", "characterName": "ANJALI", "primary": false })));
    g.ok("cast.delete", json!({ "id": dup }));
    g.ok(
        "cast.update",
        json!({ "id": ctx.cast["ANJALI"], "notes": "Learns tape baking and Nagra threading from Farhan Ali in prep week. Malayalam coach for scene 10." }),
    );
    g.ok("cast.list", json!({ "includeArchived": true }));

    for c in arr(&prod["crew"]) {
        let id = g.id(&g.ok(
            "crew.create",
            json!({ "personName": c["person"], "role": c["role"], "department": c["department"], "contact": c["contact"] }),
        ));
        ctx.crew.insert(s(&c["role"]), id);
    }
    g.ok(
        "crew.update",
        json!({ "id": ctx.crew["Director of Photography"], "availabilityNotes": "Prep from 2027-01-02. Second unit plates on 2027-01-27.",
                "notes": "Owns the anamorphic lens set; rental credited in the budget." }),
    );
    let second_unit = g.id(&g.ok(
        "crew.create",
        json!({ "personName": "Rohan Das", "role": "Second Unit Director of Photography", "department": "Camera",
                "notes": "Left the production on 2026-12-28; plates moved to Vivek Anand." }),
    ));
    g.ok("crew.set_archived", json!({ "id": second_unit, "archived": true }));
    let dup = g.id(&g.ok("crew.create", json!({ "personName": "Priya Ramesh", "role": "First Assistant Director" })));
    g.ok("crew.delete", json!({ "id": dup }));
    g.ok(
        "availability.set",
        json!({ "ownerType": "crew_member", "ownerId": ctx.crew["Composer"], "windows": [
            { "startDate": "2027-01-11", "endDate": "2027-01-19", "kind": "Unavailable", "note": "Scoring another film in Chennai" },
            { "startDate": "2027-01-21", "endDate": "2027-01-21", "kind": "Available", "note": "Church choir session (scene 18)" }
        ] }),
    );
    g.ok("crew.list", json!({ "includeArchived": true }));

    for l in arr(&prod["locations"]) {
        let key = s(&l["key"]);
        let id = g.id(&g.ok(
            "locations.create",
            json!({ "name": l["name"], "address": l["address"], "contact": l["contact"], "nearestHospital": l["hospital"],
                    "emergencyContact": l["emergency"], "status": "Shortlisted" }),
        ));
        let n = &l["notes"];
        let notes = json!({ "access": n["access"], "parking": n["parking"], "noise": n["noise"], "power": n["power"],
            "permission": n["permission"], "toilets": n["toilets"], "facilities": n["facilities"], "travel": n["travel"], "general": n["general"] });
        g.ok("locations.update", json!({ "id": id, "notes": notes }));
        g.ok("locations.set_status", json!({ "id": id, "status": l["status"] }));
        let photos: Vec<String> = arr(&l["photos"]).iter().map(|p| ctx.media.path(&s(p))).collect();
        if !photos.is_empty() {
            g.ok("locations.add_photos", json!({ "id": id, "paths": photos }));
        }
        let mut windows: Vec<Value> = arr(&l["unavailable"])
            .iter()
            .map(|w| json!({ "startDate": w[0], "endDate": w[1], "kind": "Unavailable", "note": w[2] }))
            .collect();
        windows.extend(arr(&l["available"]).iter().map(|w| json!({ "startDate": w[0], "endDate": w[1], "kind": "Available", "note": w[2] })));
        if !windows.is_empty() {
            g.ok("availability.set", json!({ "ownerType": "location", "ownerId": id, "windows": windows }));
        }
        ctx.locations.insert(key, id);
    }
    // Station photos: the dusk frame leads, a wrong upload (the night frame twice) is removed.
    let station = ctx.locations["station"].clone();
    g.ok("locations.add_photos", json!({ "id": station, "paths": [ctx.media.path("station-night.png")] }));
    let got = g.ok("locations.get", json!({ "id": station }));
    let photos = arr(&got["photos"]);
    if let Some(dusk) = photos.iter().find(|p| s(&p["caption"]).contains("dusk") || s(&p["fileName"]).contains("dusk")).or(photos.get(1)) {
        g.ok("locations.move_photo", json!({ "photoId": dusk["id"], "index": 0 }));
    }
    if let Some(last) = photos.last() {
        g.ok("locations.remove_photo", json!({ "id": last["id"] }));
    }
    g.ok(
        "locations.update",
        json!({ "id": station, "contact": "Port Trust Estate Office, Mr. Hamid Kutty, +91 484 266 6400" }),
    );
    // A beach scouted in Fort Kochi was entered by mistake (it is the Cherai beach) and removed.
    let wrong = g.id(&g.ok("locations.create", json!({ "name": "Fort Kochi beach (scouted, not used)", "status": "Idea" })));
    g.ok("locations.delete", json!({ "id": wrong }));
    g.ok("locations.list", json!({ "status": null, "search": null, "includeArchived": true }));
    g.ok("locations.list", json!({ "status": "Confirmed", "search": "Vypin", "includeArchived": false }));
}

fn breakdown_scene(ctx: &Ctx, n: u32) -> Value {
    arr(&ctx.breakdown["scenes"]).into_iter().find(|s| s["n"].as_u64() == Some(n as u64)).unwrap_or(Value::Null)
}

fn location_key_for(ctx: &Ctx, n: u32) -> Option<String> {
    let map = ctx.production["sceneLocations"].as_object()?;
    map.iter().find(|(_, v)| arr(v).iter().any(|x| x.as_u64() == Some(n as u64))).map(|(k, _)| k.clone())
}

pub fn breakdown(g: &Gold, ctx: &mut Ctx) {
    g.phase("Breakdown and catalog");
    g.ok("breakdown.categories", json!({}));
    g.ok("breakdown.set_category_style", json!({ "category": "VFX", "color": "violet", "icon": "sparkles" }));
    g.ok("breakdown.set_category_style", json!({ "category": "Special Effects", "color": "orange", "icon": "flame" }));
    g.ok("breakdown.reset_category_styles", json!({}));
    g.ok("breakdown.set_category_style", json!({ "category": "VFX", "color": "violet", "icon": "sparkles" }));
    g.ok("breakdown.set_category_style", json!({ "category": "Animals", "color": "brown", "icon": "paw" }));
    g.ok("breakdown.scenes", json!({ "filter": "all" }));

    // Hero props and picture vehicles go into the catalog first, with status and contacts.
    let hero: Vec<(&str, &str, &str, &str, &str)> = vec![
        ("Props", "Quarter-inch reel 14-11-1999", "Confirmed", "Hero reel plus two stunt reels. Label in Mariamma's hand drawn by Sunil Babu.", "Props master: Sunil Babu"),
        ("Props", "Nagra IV-S reel-to-reel recorder", "Shortlisted", "Working unit with spare heads, from a Chennai rental house.", "Prakash Audio Rentals, Chennai, +91 44 2434 5566"),
        ("Props", "Biscuit-tin transmitter", "Confirmed", "Built by the props team with Aditya Nair. Non-transmitting; LEDs only.", null_str()),
        ("Props", "Station logbook 1999", "Confirmed", "Hero logbook with 112 written pages, aged.", null_str()),
        ("Vehicles", "Ambassador car (Joe's)", "Confirmed", "1994 Hindustan Ambassador, white, KL-07 plates. Precision driver available.", "Classic Cars Kochi, +91 98470 61111"),
        ("Vehicles", "Police jeep (Mahindra Bolero)", "Confirmed", "Picture vehicle with removable insignia.", "Kerala Picture Vehicles, +91 98470 62222"),
        ("Animals", "Kannan (dog)", "Confirmed", "Two trained Indian pariah dogs, Kannan A (hero) and Kannan B (double).", "Benny Thomas, wrangler"),
        ("Special Effects", "Rain towers and wind machines", "Confirmed", "Four towers, two Ritter fans. Fresh water tanker only.", "Dileep Raj SFX"),
    ];
    for (cat, name, status, desc, contact) in hero {
        let item = g.ok(
            "catalog.create",
            json!({ "category": cat, "name": name, "status": status, "description": desc, "notes": null, "contact": if contact.is_empty() { Value::Null } else { json!(contact) } }),
        );
        ctx.catalog.insert(name.to_string(), g.id(&item));
    }
    let reel = ctx.catalog["Quarter-inch reel 14-11-1999"].clone();
    g.ok("catalog.add_alias", json!({ "id": reel, "alias": "the reel" }));
    let tmp_alias = g.ok("catalog.add_alias", json!({ "id": reel, "alias": "tape" }));
    if let Some(a) = arr(&tmp_alias["aliases"]).iter().find(|a| s(&a["alias"]) == "tape" || s(&a["name"]) == "tape") {
        g.ok("catalog.remove_alias", json!({ "id": a["id"] }));
    }
    g.ok("catalog.set_image", json!({ "id": reel, "path": ctx.media.path("prop-reel.png") }));
    g.ok("catalog.set_image", json!({ "id": ctx.catalog["Ambassador car (Joe's)"], "path": ctx.media.path("vehicle-ambassador.png") }));
    g.ok(
        "catalog.update",
        json!({ "id": ctx.catalog["Nagra IV-S reel-to-reel recorder"], "status": "Confirmed", "notes": "Booked 2027-01-04. Insured in transit." }),
    );
    g.ok("catalog.find_matches", json!({ "category": "Props", "name": "Quarter inch reel", "excludeId": null }));
    // A stray duplicate created by the art department and removed.
    let stray = g.id(&g.ok("catalog.create", json!({ "category": "Props", "name": "Reel (old label)", "status": "Not Required" })));
    g.ok("catalog.delete", json!({ "id": stray }));

    // The rejected Alappuzha station was pencilled in for two scenes; replaced later.
    let coast = ctx.locations.get("coastguard").cloned().unwrap_or_default();

    let mut accepted_total = 0;
    let mut added_total = 0;
    let mut rejected_total = 0;
    let numbers: Vec<u32> = ctx.scenes.keys().copied().collect();
    for n in numbers {
        let scene = ctx.scenes[&n].clone();
        g.ok("breakdown.suggest", json!({ "sceneId": scene }));
        let bd = breakdown_scene(ctx, n);
        let mine: Vec<Value> = arr(&bd["elements"]);
        let detail = g.ok("breakdown.scene", json!({ "sceneId": scene }));
        let mut accept = Vec::new();
        let mut reject = Vec::new();
        for grp in arr(&detail["groups"]) {
            for sug in arr(&grp["suggestions"]) {
                let name = s(&sug["name"]).to_lowercase();
                let cat = s(&grp["category"]);
                // Set names from the headings are not taken: the practical location is added
                // below, with the set name in its notes.
                let wanted = cat == "Cast"
                    || (cat != "Location / Set" && mine.iter().any(|e| s(&e["category"]) == cat && s(&e["name"]).to_lowercase() == name));
                if wanted { accept.push(sug["id"].clone()) } else { reject.push(sug["id"].clone()) }
            }
        }
        if !accept.is_empty() {
            let r = g.ok("breakdown.accept_many", json!({ "ids": accept }));
            accepted_total += r["accepted"].as_i64().unwrap_or(0);
        }
        if !reject.is_empty() {
            let r = g.ok("breakdown.reject", json!({ "ids": reject }));
            rejected_total += r["count"].as_i64().unwrap_or(reject.len() as i64);
        }
        // Production location for the scene (the Location / Set entry links the location).
        if let Some(key) = location_key_for(ctx, n) {
            let loc_name = arr(&ctx.production["locations"]).iter().find(|l| s(&l["key"]) == key).map(|l| s(&l["name"])).unwrap_or_default();
            let set_note = mine.iter().find(|e| s(&e["category"]) == "Location / Set").map(|e| s(&e["name"]));
            let use_coast = (n == 2 || n == 11) && !coast.is_empty();
            let name = if use_coast {
                arr(&ctx.production["locations"]).iter().find(|l| s(&l["key"]) == "coastguard").map(|l| s(&l["name"])).unwrap_or_default()
            } else {
                loc_name
            };
            let catalog = catalog_choice(g, ctx, "Location / Set", &name);
            let r = g.ok(
                "breakdown.add_element",
                json!({ "sceneId": scene, "category": "Location / Set", "name": name, "notes": set_note, "catalog": catalog }),
            );
            if let Some(cid) = r["catalogItemId"].as_str() {
                ctx.catalog.entry(name.clone()).or_insert_with(|| cid.to_string());
            }
            if !r.is_null() {
                added_total += 1;
            }
        }
        let current = g.ok("breakdown.scene", json!({ "sceneId": scene }));
        let have: Vec<(String, String)> = arr(&current["groups"])
            .iter()
            .flat_map(|grp| arr(&grp["confirmed"]).into_iter().map(move |e| (s(&grp["category"]), s(&e["name"]).to_lowercase())))
            .collect();
        for e in &mine {
            let cat = s(&e["category"]);
            let name = s(&e["name"]);
            if cat == "Location / Set" {
                continue;
            }
            if have.iter().any(|(c, nm)| *c == cat && *nm == name.to_lowercase()) {
                continue;
            }
            let catalog = catalog_choice(g, ctx, &cat, &name);
            let r = g.ok(
                "breakdown.add_element",
                json!({ "sceneId": scene, "category": cat, "name": name, "notes": e["notes"], "catalog": catalog }),
            );
            if let Some(cid) = r["catalogItemId"].as_str() {
                ctx.catalog.entry(name.clone()).or_insert_with(|| cid.to_string());
            }
            if !r.is_null() {
                added_total += 1;
            }
        }
        g.ok("breakdown.mark_reviewed", json!({ "sceneId": scene }));
        g.ok("breakdown.set_complete", json!({ "sceneId": scene, "complete": true }));
    }
    ctx.facts.insert("breakdownTotals".into(), json!({ "acceptedSuggestions": accepted_total, "rejectedSuggestions": rejected_total, "addedByHand": added_total }));

    // A tagged span: the reel in the scene 11 action text.
    let doc = crate::phases::screenplay::document(g, &ctx.id("draftCurrent"));
    let s11 = ctx.scenes[&11].clone();
    if let Some((el, start, end)) = arr(&doc["scenes"]).iter().find(|sc| s(&sc["id"]) == s11).and_then(|sc| {
        arr(&sc["elements"]).into_iter().find_map(|e| {
            let t = s(&e["text"]);
            let lower = t.to_lowercase();
            lower.find("reel").map(|i| {
                let start = t[..i].encode_utf16().count();
                (s(&e["id"]), start, start + 4)
            })
        })
    }) {
        let tagged = g.ok(
            "breakdown.add_element",
            json!({ "sceneId": s11, "category": "Props", "name": "Biscuit box (reel case)", "notes": "The reel is found inside a Britannia-style biscuit box; props to clear the brand.",
                    "span": { "elementId": el, "start": start, "end": end }, "catalog": { "mode": "new", "catalogItemId": null } }),
        );
        let tid = g.id(&tagged);
        g.ok("breakdown.source_anchor", json!({ "id": tid }));
        g.ok("breakdown.update_element", json!({ "id": tid, "notes": "Unbranded tin with a hand-painted label; brand clearance not needed." }));
    }
    // A wrong element removed and an old one archived.
    let s3 = ctx.scenes[&3].clone();
    let wrong = g.ok(
        "breakdown.add_element",
        json!({ "sceneId": s3, "category": "Props", "name": "Detergent bottle", "notes": "Removed: the ad is only heard, never seen.", "catalog": { "mode": "new", "catalogItemId": null } }),
    );
    g.ok("breakdown.remove_element", json!({ "id": g.id(&wrong) }));
    let s4 = ctx.scenes[&4].clone();
    let old = g.ok(
        "breakdown.add_element",
        json!({ "sceneId": s4, "category": "Props", "name": "Portable CD player", "notes": "Replaced by the Walkman: the cassette is the point.", "catalog": { "mode": "new", "catalogItemId": null } }),
    );
    g.ok("breakdown.set_archived", json!({ "id": g.id(&old), "archived": true }));

    // Reuse: rain and wind for both storm scenes, and the reel wherever it plays.
    let s23 = ctx.scenes[&23].clone();
    let s24 = ctx.scenes[&24].clone();
    g.ok("breakdown.quick_reuse", json!({ "sceneId": s24 }));
    g.ok("breakdown.reuse", json!({ "sceneIds": [s23, s24], "catalogItemIds": [ctx.catalog["Rain towers and wind machines"]] }));
    let reel_scenes: Vec<String> = [11u32, 13, 14, 16, 33, 39, 40].iter().filter_map(|n| ctx.scenes.get(n).cloned()).collect();
    g.ok("breakdown.reuse", json!({ "sceneIds": reel_scenes, "catalogItemIds": [reel] }));

    // The Walkman was generic in early scenes; the hero model replaces it in scene 4.
    let generic = g.id(&g.ok("catalog.create", json!({ "category": "Sound", "name": "Cassette player", "status": "Required" })));
    g.ok("breakdown.add_element", json!({ "sceneId": s4, "category": "Sound", "name": "Cassette player", "catalog": { "mode": "existing", "catalogItemId": generic } }));
    let walkman = g.id(&g.ok(
        "catalog.create",
        json!({ "category": "Sound", "name": "Sony Walkman WM-10 with Mariamma cassette", "status": "Confirmed", "description": "Period-correct. The cassette label reads Weather, Nov 98." }),
    ));
    g.ok("catalog.replace_in_scenes", json!({ "fromId": generic, "toId": walkman, "sceneIds": [s4] }));
    g.ok("catalog.set_archived", json!({ "id": generic, "archived": true }));

    // The Alappuzha station is rejected; scenes 2 and 11 move to the Vypin station.
    if !coast.is_empty() {
        let ids: Vec<String> = [2u32, 11].iter().filter_map(|n| ctx.scenes.get(n).cloned()).collect();
        g.ok("locations.replace", json!({ "locationId": coast, "replacementId": ctx.locations["station"], "sceneIds": ids }));
        g.ok("locations.set_archived", json!({ "id": coast, "archived": true }));
    }

    g.ok("catalog.list", json!({ "category": null, "search": null, "includeArchived": true }));
    g.ok("catalog.list", json!({ "category": "Vehicles", "search": null, "includeArchived": false }));
    g.ok("catalog.get", json!({ "id": reel }));
    g.ok("production.references", json!({ "kind": "catalog_item", "id": reel }));
    g.ok("production.references", json!({ "kind": "location", "id": ctx.locations["station"] }));
    g.ok("production.references", json!({ "kind": "cast_member", "id": ctx.cast["ANJALI"] }));
    g.ok("production.references", json!({ "kind": "crew_member", "id": ctx.crew["Director of Photography"] }));
    for f in ["all", "unreviewed", "suggested", "missing", "needsReview", "complete", "incomplete"] {
        g.ok("breakdown.scenes", json!({ "filter": f }));
    }
    g.ok("breakdown.scenes", json!({ "category": "VFX" }));
    g.ok("breakdown.scenes", json!({ "catalogItemId": reel }));
    g.ok("breakdown.scene_changes", json!({ "sceneId": ctx.scenes[&24] }));
    g.ok("production.overview", json!({}));
    g.ok("visual.scene_characters", json!({ "sceneId": ctx.scenes[&39] }));
}

fn null_str() -> &'static str {
    ""
}

/// The catalog item a breakdown entry should use: the item of the same name when the catalog
/// has one (the "Use existing" choice), otherwise a new item.
fn catalog_choice(g: &Gold, ctx: &Ctx, category: &str, name: &str) -> Value {
    if let Some(id) = ctx.catalog.get(name) {
        return json!({ "mode": "existing", "catalogItemId": id });
    }
    let matches = g.ok("catalog.find_matches", json!({ "category": category, "name": name, "excludeId": null }));
    match arr(&matches).into_iter().find(|m| m["exact"] == json!(true)) {
        Some(m) => json!({ "mode": "existing", "catalogItemId": m["id"] }),
        None => json!({ "mode": "new", "catalogItemId": null }),
    }
}

pub fn budget(g: &Gold, ctx: &mut Ctx) {
    g.phase("Budget");
    let b = ctx.production["budget"].clone();
    let id = g.id(&g.ok("budget.create", json!({ "currency": b["currency"] })));
    ctx.set("budget", id.clone());
    let mut lines: BTreeMap<String, String> = BTreeMap::new();
    for l in arr(&b["lines"]) {
        let lid = g.id(&g.ok(
            "budget.add_line",
            json!({ "budgetId": id, "category": l[0], "description": l[1], "planned": to_paise(&l[2]), "actual": to_paise(&l[3]), "notes": l[4] }),
        ));
        lines.insert(s(&l[1]), lid);
    }
    g.ok(
        "budget.update",
        json!({ "id": id, "currency": b["currency"], "plannedTotal": to_paise(&b["plannedTotal"]), "contingencyMode": b["contingencyMode"],
                "contingencyValue": b["contingencyValue"].as_i64().unwrap_or(0) * 100, "notes": b["notes"], "expectedRev": null }),
    );
    let v1 = g.id(&g.ok("budget.save_snapshot", json!({ "id": id, "label": "Locked budget v1, 18 December 2026" })));
    ctx.set("budgetSnapshotV1", v1.clone());
    // Prep spending since the lock.
    let cam_desc = "Camera: Alexa 35 package with anamorphic lenses, 16 days";
    if let Some(cam) = lines.get(cam_desc) {
        g.ok(
            "budget.update_line",
            json!({ "id": cam, "category": "Equipment", "description": cam_desc, "planned": 192000000,
                    "actual": 60000000, "notes": "Deposit paid 2027-01-04.", "expectedRev": null }),
        );
    }
    let mistake = g.id(&g.ok(
        "budget.add_line",
        json!({ "budgetId": id, "category": "Equipment", "description": "Camera: drone unit", "planned": 18000000, "actual": null, "notes": "Entered twice." }),
    ));
    g.ok("budget.delete_line", json!({ "id": mistake }));
    g.ok("budget.mark_reviewed", json!({ "id": id }));
    // A rough snapshot made by mistake and deleted.
    let rough = g.id(&g.ok("budget.save_snapshot", json!({ "id": id, "label": "rough" })));
    g.ok("budget.delete_snapshot", json!({ "id": rough }));
    g.ok("budget.get_snapshot", json!({ "id": v1 }));
    let view = g.ok("budget.get", json!({}));
    let c = &view["current"];
    ctx.facts.insert("budget".into(), json!({ "currency": c["currency"], "plannedTotal": c["plannedTotal"], "plannedLinesTotal": c["plannedLinesTotal"],
        "actualTotal": c["actualTotal"], "contingencyAmount": c["contingencyAmount"], "difference": c["difference"], "lines": arr(&c["lines"]).len() }));
}

/// The blue pages reach production: preview, apply, review the changed scenes.
pub fn after_revision(g: &Gold, ctx: &mut Ctx) {
    g.phase("Production update to the blue pages");
    let blue = ctx.id("draftBlue");
    g.ok("production.preview_update", json!({ "draftId": blue }));
    g.ok("production.apply_update", json!({ "draftId": blue, "reason": "Blue Revision, 5 January 2027." }));
    ctx.set("draftCurrent", blue.clone());
    let old_lineage = ctx.lineage.clone();
    crate::phases::screenplay::map_scenes(g, ctx);
    ctx.lineage = old_lineage;
    let new = ctx.id("scene23A");
    if !new.is_empty() {
        g.ok("breakdown.suggest", json!({ "sceneId": new }));
        let d = g.ok("breakdown.scene", json!({ "sceneId": new }));
        // Heading set names are not taken (the jetty is part of the harbour location).
        let ids: Vec<Value> = arr(&d["groups"])
            .iter()
            .filter(|grp| s(&grp["category"]) != "Location / Set")
            .flat_map(|grp| arr(&grp["suggestions"]))
            .map(|x| x["id"].clone())
            .collect();
        let sets: Vec<Value> = arr(&d["groups"])
            .iter()
            .filter(|grp| s(&grp["category"]) == "Location / Set")
            .flat_map(|grp| arr(&grp["suggestions"]))
            .map(|x| x["id"].clone())
            .collect();
        if !sets.is_empty() {
            g.ok("breakdown.reject", json!({ "ids": sets }));
        }
        if let Some(first) = ids.first() {
            g.ok("breakdown.accept", json!({ "id": first, "category": null, "name": null, "catalog": { "mode": "auto", "catalogItemId": null } }));
        }
        if ids.len() > 1 {
            g.ok("breakdown.accept_many", json!({ "ids": ids[1..].to_vec() }));
        }
        g.ok(
            "breakdown.add_element",
            json!({ "sceneId": new, "category": "Location / Set", "name": "Munambam Fishing Harbour", "notes": "Kadalkkara jetty: last plank", "catalog": { "mode": "auto", "catalogItemId": null } }),
        );
        g.ok("breakdown.add_element", json!({ "sceneId": new, "category": "Special Effects", "name": "Rain towers and wind machines", "catalog": { "mode": "existing", "catalogItemId": ctx.catalog["Rain towers and wind machines"] } }));
        g.ok("breakdown.mark_reviewed", json!({ "sceneId": new }));
        g.ok("breakdown.set_complete", json!({ "sceneId": new, "complete": true }));
    }
    let changed = g.ok("breakdown.scenes", json!({ "filter": "needsReview" }));
    for sc in arr(&changed["scenes"]) {
        let id = s(&sc["sceneId"]);
        g.ok("breakdown.scene_changes", json!({ "sceneId": id }));
        g.ok("breakdown.apply_suggested_update", json!({ "sceneId": id }));
        g.ok("breakdown.mark_reviewed", json!({ "sceneId": id }));
    }
    // The line producer checks the budget against the blue pages: no cost change.
    g.ok("budget.mark_reviewed", json!({ "id": ctx.id("budget") }));
    g.ok("production.overview", json!({}));
}
