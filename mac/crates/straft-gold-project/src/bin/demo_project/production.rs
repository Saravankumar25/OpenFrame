//! Cast, crew and locations; the breakdown of every scene with the catalog; the budget; and the
//! production update to the revision pages.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::Ctx;
use crate::gold::{Gold, arr, s};

fn to_paise(rupees: &Value) -> Value {
    rupees
        .as_f64()
        .map_or(Value::Null, |r| json!((r * 100.0) as i64))
}

fn window(w: &Value, kind: &str) -> Value {
    json!({ "startDate": w[0], "endDate": w[1], "kind": kind, "note": w[2] })
}

pub fn people_and_places(g: &Gold, ctx: &mut Ctx) {
    g.phase("Cast, crew and locations");
    let prod = ctx.production.clone();
    let ex = &prod["extras"];
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
            g.ok(
                "cast.set_photo",
                json!({ "id": id, "path": ctx.media_path(photo) }),
            );
        }
        let windows: Vec<Value> = arr(&c["unavailable"])
            .iter()
            .map(|w| window(w, "Unavailable"))
            .collect();
        if !windows.is_empty() {
            g.ok(
                "availability.set",
                json!({ "ownerType": "cast_member", "ownerId": id, "windows": windows }),
            );
        }
        ctx.cast.insert(ch, id);
    }
    let a = &ex["alternate"];
    g.ok(
        "cast.create",
        json!({ "personName": a["person"], "characterId": ctx.chars.get(&s(&a["character"])), "primary": false, "contact": a["contact"], "notes": a["notes"] }),
    );
    let d = &ex["droppedCast"];
    let dropped = g.id(&g.ok(
        "cast.create",
        json!({ "personName": d["person"], "characterName": d["character"], "primary": false, "notes": d["notes"] }),
    ));
    g.ok(
        "cast.assign",
        json!({ "id": dropped, "characterName": d["character"], "primary": true }),
    );
    g.ok(
        "cast.assign",
        json!({ "id": ctx.cast[&s(&d["character"])], "characterName": d["character"], "primary": true }),
    );
    g.ok(
        "cast.set_archived",
        json!({ "id": dropped, "archived": true }),
    );
    let dup_key = s(&ex["dupCast"]["character"]);
    let person = arr(&prod["cast"])
        .iter()
        .find(|c| s(&c["character"]) == dup_key)
        .map(|c| s(&c["person"]))
        .unwrap_or_default();
    let dup = g.id(&g.ok(
        "cast.create",
        json!({ "personName": person, "characterName": dup_key, "primary": false }),
    ));
    g.ok("cast.delete", json!({ "id": dup }));
    g.ok(
        "cast.update",
        json!({ "id": ctx.cast[&s(&ex["castNotes"]["key"])], "notes": ex["castNotes"]["notes"] }),
    );
    g.ok("cast.list", json!({ "includeArchived": true }));

    for c in arr(&prod["crew"]) {
        let id = g.id(&g.ok(
            "crew.create",
            json!({ "personName": c["person"], "role": c["role"], "department": c["department"], "contact": c["contact"], "notes": c["notes"] }),
        ));
        ctx.crew.insert(s(&c["role"]), id);
    }
    let cu = &ex["crewUpdate"];
    g.ok(
        "crew.update",
        json!({ "id": ctx.crew[&s(&cu["role"])], "availabilityNotes": cu["availabilityNotes"], "notes": cu["notes"] }),
    );
    let ac = &ex["archivedCrew"];
    let left = g.id(&g.ok(
        "crew.create",
        json!({ "personName": ac["person"], "role": ac["role"], "department": ac["department"], "notes": ac["notes"] }),
    ));
    g.ok("crew.set_archived", json!({ "id": left, "archived": true }));
    let dc = &ex["dupCrew"];
    let dup = g.id(&g.ok(
        "crew.create",
        json!({ "personName": dc["person"], "role": dc["role"] }),
    ));
    g.ok("crew.delete", json!({ "id": dup }));
    let cw = &ex["composerWindows"];
    let windows: Vec<Value> = arr(&cw["windows"])
        .iter()
        .map(|w| json!({ "startDate": w[0], "endDate": w[1], "kind": w[2], "note": w[3] }))
        .collect();
    g.ok(
        "availability.set",
        json!({ "ownerType": "crew_member", "ownerId": ctx.crew[&s(&cw["role"])], "windows": windows }),
    );
    g.ok("crew.list", json!({ "includeArchived": true }));

    for l in arr(&prod["locations"]) {
        let id = g.id(&g.ok(
            "locations.create",
            json!({ "name": l["name"], "address": l["address"], "contact": l["contact"], "nearestHospital": l["hospital"],
                    "emergencyContact": l["emergency"], "status": "Shortlisted" }),
        ));
        g.ok("locations.update", json!({ "id": id, "notes": l["notes"] }));
        g.ok(
            "locations.set_status",
            json!({ "id": id, "status": l["status"] }),
        );
        let photos: Vec<String> = arr(&l["photos"])
            .iter()
            .map(|p| ctx.media_path(&s(p)))
            .collect();
        if !photos.is_empty() {
            g.ok("locations.add_photos", json!({ "id": id, "paths": photos }));
        }
        let mut windows: Vec<Value> = arr(&l["unavailable"])
            .iter()
            .map(|w| window(w, "Unavailable"))
            .collect();
        windows.extend(arr(&l["available"]).iter().map(|w| window(w, "Available")));
        if !windows.is_empty() {
            g.ok(
                "availability.set",
                json!({ "ownerType": "location", "ownerId": id, "windows": windows }),
            );
        }
        ctx.locations.insert(s(&l["key"]), id);
    }
    // A wrong photo is added, moved to the front, moved back and removed.
    let sw = &ex["locationPhotoSwap"];
    let loc = ctx.locations[&s(&sw["key"])].clone();
    g.ok(
        "locations.add_photos",
        json!({ "id": loc, "paths": [ctx.media_path(&s(&sw["extra"]))] }),
    );
    let got = g.ok("locations.get", json!({ "id": loc }));
    let photos = arr(&got["photos"]);
    if let Some(last) = photos.last() {
        g.ok(
            "locations.move_photo",
            json!({ "photoId": last["id"], "index": 0 }),
        );
        g.ok("locations.remove_photo", json!({ "id": last["id"] }));
    }
    g.ok(
        "locations.update",
        json!({ "id": loc, "contact": sw["contact"] }),
    );
    let wl = &ex["wrongLocation"];
    let wrong = g.id(&g.ok(
        "locations.create",
        json!({ "name": wl["name"], "status": wl["status"] }),
    ));
    g.ok("locations.delete", json!({ "id": wrong }));
    g.ok(
        "locations.list",
        json!({ "status": null, "search": null, "includeArchived": true }),
    );
    g.ok(
        "locations.list",
        json!({ "status": prod["statusFilter"], "search": ex["listSearch"], "includeArchived": false }),
    );
}

fn location_key_for(ctx: &Ctx, n: u32) -> Option<String> {
    let map = ctx.production["sceneLocations"].as_object()?;
    map.iter()
        .find(|(_, v)| arr(v).iter().any(|x| x.as_u64() == Some(n as u64)))
        .map(|(k, _)| k.clone())
}

fn location_name(ctx: &Ctx, key: &str) -> String {
    arr(&ctx.production["locations"])
        .iter()
        .find(|l| s(&l["key"]) == key)
        .map(|l| s(&l["name"]))
        .unwrap_or_default()
}

fn heading_of(ctx: &Ctx, n: u32) -> String {
    for act in arr(&ctx.story["acts"]) {
        for seq in arr(&act["sequences"]) {
            for sc in arr(&seq["scenes"]) {
                if sc["n"].as_u64() == Some(n as u64) {
                    return s(&sc["heading"]);
                }
            }
        }
    }
    String::new()
}

/// The catalog item a breakdown entry should use: the item of the same name when the catalog has
/// one, otherwise a new item.
fn catalog_choice(g: &Gold, ctx: &Ctx, category: &str, name: &str) -> Value {
    if let Some(id) = ctx.catalog.get(name) {
        return json!({ "mode": "existing", "catalogItemId": id });
    }
    let matches = g.ok(
        "catalog.find_matches",
        json!({ "category": category, "name": name, "excludeId": null }),
    );
    match arr(&matches)
        .into_iter()
        .find(|m| m["exact"] == json!(true))
    {
        Some(m) => json!({ "mode": "existing", "catalogItemId": m["id"] }),
        None => json!({ "mode": "new", "catalogItemId": null }),
    }
}

fn add_element(
    g: &Gold,
    ctx: &mut Ctx,
    scene: &str,
    cat: &str,
    name: &str,
    notes: &Value,
) -> Value {
    let catalog = catalog_choice(g, ctx, cat, name);
    let r = g.ok(
        "breakdown.add_element",
        json!({ "sceneId": scene, "category": cat, "name": name, "notes": notes, "catalog": catalog }),
    );
    if let Some(cid) = r["catalogItemId"].as_str() {
        ctx.catalog
            .entry(name.to_string())
            .or_insert_with(|| cid.to_string());
    }
    r
}

pub fn breakdown(g: &Gold, ctx: &mut Ctx) {
    g.phase("Breakdown and catalog");
    let b = ctx.breakdown.clone();
    g.ok("breakdown.categories", json!({}));
    for st in arr(&b["categoryStyles"]) {
        g.ok("breakdown.set_category_style", st);
    }
    g.ok("breakdown.reset_category_styles", json!({}));
    for st in arr(&b["categoryStyles"]) {
        g.ok("breakdown.set_category_style", st);
    }
    g.ok("breakdown.scenes", json!({ "filter": "all" }));

    for h in arr(&b["hero"]) {
        let item = g.ok(
            "catalog.create",
            json!({ "category": h["category"], "name": h["name"], "status": h["status"], "description": h["description"], "notes": null, "contact": h["contact"] }),
        );
        let id = g.id(&item);
        ctx.catalog.insert(s(&h["name"]), id.clone());
        for a in arr(&h["alias"]) {
            g.ok("catalog.add_alias", json!({ "id": id, "alias": a }));
        }
        if let Some(img) = h["image"].as_str() {
            g.ok(
                "catalog.set_image",
                json!({ "id": id, "path": ctx.media_path(img) }),
            );
        }
    }
    let ap = &b["aliasProbe"];
    let hero_id = ctx.catalog[&s(&ap["name"])].clone();
    let temp = g.ok(
        "catalog.add_alias",
        json!({ "id": hero_id, "alias": ap["temp"] }),
    );
    if let Some(a) = arr(&temp["aliases"])
        .iter()
        .find(|a| s(&a["alias"]) == s(&ap["temp"]) || s(&a["name"]) == s(&ap["temp"]))
    {
        g.ok("catalog.remove_alias", json!({ "id": a["id"] }));
    }
    let up = &b["statusUpdate"];
    g.ok(
        "catalog.update",
        json!({ "id": ctx.catalog[&s(&up["name"])], "status": up["status"], "notes": up["notes"] }),
    );
    let fp = &b["findProbe"];
    g.ok(
        "catalog.find_matches",
        json!({ "category": fp["category"], "name": fp["name"], "excludeId": null }),
    );
    let st = &b["strayCatalog"];
    let stray = g.id(&g.ok(
        "catalog.create",
        json!({ "category": st["category"], "name": st["name"], "status": st["status"] }),
    ));
    g.ok("catalog.delete", json!({ "id": stray }));

    let rej = ctx.production["extras"]["rejected"].clone();
    let rejected_scenes: Vec<u64> = arr(&rej["scenes"])
        .iter()
        .filter_map(Value::as_u64)
        .collect();
    let rejected_key = s(&rej["key"]);

    let mut accepted_total = 0;
    let mut added_total = 0;
    let mut rejected_total = 0;
    let specs: BTreeMap<u32, Value> = arr(&b["scenes"])
        .into_iter()
        .map(|x| (x["n"].as_u64().unwrap_or(0) as u32, x))
        .collect();
    let numbers: Vec<u32> = ctx.scenes.keys().copied().collect();
    for n in numbers {
        let scene = ctx.scenes[&n].clone();
        g.ok("breakdown.suggest", json!({ "sceneId": scene }));
        let mine: Vec<Value> = specs
            .get(&n)
            .map(|x| arr(&x["elements"]))
            .unwrap_or_default();
        let detail = g.ok("breakdown.scene", json!({ "sceneId": scene }));
        let mut accept = Vec::new();
        let mut reject = Vec::new();
        for grp in arr(&detail["groups"]) {
            let cat = s(&grp["category"]);
            for sug in arr(&grp["suggestions"]) {
                let name = s(&sug["name"]).to_lowercase();
                let wanted = cat != "Location / Set"
                    && mine
                        .iter()
                        .any(|e| s(&e["category"]) == cat && s(&e["name"]).to_lowercase() == name);
                if wanted {
                    accept.push(sug["id"].clone());
                } else {
                    reject.push(sug["id"].clone());
                }
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
        if let Some(key) = location_key_for(ctx, n) {
            let key = if rejected_scenes.contains(&(n as u64)) {
                rejected_key.clone()
            } else {
                key
            };
            let name = location_name(ctx, &key);
            let r = add_element(
                g,
                ctx,
                &scene,
                "Location / Set",
                &name,
                &json!(heading_of(ctx, n)),
            );
            if !r.is_null() {
                added_total += 1;
            }
        }
        let current = g.ok("breakdown.scene", json!({ "sceneId": scene }));
        let have: Vec<(String, String)> = arr(&current["groups"])
            .iter()
            .flat_map(|grp| {
                arr(&grp["confirmed"])
                    .into_iter()
                    .map(move |e| (s(&grp["category"]), s(&e["name"]).to_lowercase()))
            })
            .collect();
        for e in &mine {
            let cat = s(&e["category"]);
            let name = s(&e["name"]);
            if have
                .iter()
                .any(|(c, nm)| *c == cat && *nm == name.to_lowercase())
            {
                continue;
            }
            let r = add_element(g, ctx, &scene, &cat, &name, &e["notes"]);
            if !r.is_null() {
                added_total += 1;
            }
        }
        g.ok("breakdown.mark_reviewed", json!({ "sceneId": scene }));
        g.ok(
            "breakdown.set_complete",
            json!({ "sceneId": scene, "complete": true }),
        );
    }
    ctx.facts.insert(
        "breakdownTotals".into(),
        json!({ "acceptedSuggestions": accepted_total, "rejectedSuggestions": rejected_total, "addedByHand": added_total }),
    );

    // A tagged span in the action text.
    let ts = &b["taggedSpan"];
    let doc = crate::script::document(g, &ctx.id("draftCurrent"));
    let tag_scene = ctx.scene(&ts["scene"]);
    let word = s(&ts["word"]);
    if let Some((el, start, end)) = arr(&doc["scenes"])
        .iter()
        .find(|x| s(&x["id"]) == tag_scene)
        .and_then(|x| {
            arr(&x["elements"]).into_iter().find_map(|e| {
                let t = s(&e["text"]);
                t.to_lowercase().find(&word).map(|i| {
                    let start = t[..i].encode_utf16().count();
                    (s(&e["id"]), start, start + word.encode_utf16().count())
                })
            })
        })
    {
        let tagged = g.ok(
            "breakdown.add_element",
            json!({ "sceneId": tag_scene, "category": ts["category"], "name": ts["name"], "notes": ts["notes"],
                    "span": { "elementId": el, "start": start, "end": end }, "catalog": { "mode": "new", "catalogItemId": null } }),
        );
        let tid = g.id(&tagged);
        g.ok("breakdown.source_anchor", json!({ "id": tid }));
        g.ok(
            "breakdown.update_element",
            json!({ "id": tid, "notes": ts["update"] }),
        );
    }
    let we = &b["wrongElement"];
    let wrong = g.ok(
        "breakdown.add_element",
        json!({ "sceneId": ctx.scene(&we["scene"]), "category": we["category"], "name": we["name"], "notes": we["notes"], "catalog": { "mode": "new", "catalogItemId": null } }),
    );
    g.ok("breakdown.remove_element", json!({ "id": g.id(&wrong) }));
    let ae = &b["archivedElement"];
    let old = g.ok(
        "breakdown.add_element",
        json!({ "sceneId": ctx.scene(&ae["scene"]), "category": ae["category"], "name": ae["name"], "notes": ae["notes"], "catalog": { "mode": "new", "catalogItemId": null } }),
    );
    g.ok(
        "breakdown.set_archived",
        json!({ "id": g.id(&old), "archived": true }),
    );

    g.ok(
        "breakdown.quick_reuse",
        json!({ "sceneId": ctx.scene(&b["quickReuse"]["scene"]) }),
    );
    for r in arr(&b["reuse"]) {
        let ids: Vec<String> = arr(&r["scenes"]).iter().map(|n| ctx.scene(n)).collect();
        g.ok(
            "breakdown.reuse",
            json!({ "sceneIds": ids, "catalogItemIds": [ctx.catalog[&s(&r["item"])]] }),
        );
    }
    let rs = &b["replaceInScenes"];
    let rs_scene = ctx.scene(&rs["scene"]);
    let generic = g.id(&g.ok(
        "catalog.create",
        json!({ "category": rs["category"], "name": rs["generic"], "status": "Required" }),
    ));
    g.ok(
        "breakdown.add_element",
        json!({ "sceneId": rs_scene, "category": rs["category"], "name": rs["generic"], "catalog": { "mode": "existing", "catalogItemId": generic } }),
    );
    let specific = g.id(&g.ok(
        "catalog.create",
        json!({ "category": rs["category"], "name": rs["specific"], "status": "Confirmed", "description": rs["description"] }),
    ));
    g.ok(
        "catalog.replace_in_scenes",
        json!({ "fromId": generic, "toId": specific, "sceneIds": [rs_scene] }),
    );
    g.ok(
        "catalog.set_archived",
        json!({ "id": generic, "archived": true }),
    );

    if let Some(rejected_id) = ctx.locations.get(&rejected_key).cloned() {
        let ids: Vec<String> = arr(&rej["scenes"]).iter().map(|n| ctx.scene(n)).collect();
        g.ok(
            "locations.replace",
            json!({ "locationId": rejected_id, "replacementId": ctx.locations[&s(&rej["replacement"])], "sceneIds": ids }),
        );
        g.ok(
            "locations.set_archived",
            json!({ "id": rejected_id, "archived": true }),
        );
    }

    g.ok(
        "catalog.list",
        json!({ "category": null, "search": null, "includeArchived": true }),
    );
    g.ok(
        "catalog.list",
        json!({ "category": "Vehicles", "search": null, "includeArchived": false }),
    );
    let item = ctx.catalog[&s(&b["reportsItem"])].clone();
    g.ok("catalog.get", json!({ "id": item }));
    g.ok(
        "production.references",
        json!({ "kind": "catalog_item", "id": item }),
    );
    g.ok(
        "production.references",
        json!({ "kind": "location", "id": ctx.locations[&s(&b["referenceLocation"])] }),
    );
    g.ok(
        "production.references",
        json!({ "kind": "cast_member", "id": ctx.cast[&s(&b["referenceCast"])] }),
    );
    g.ok(
        "production.references",
        json!({ "kind": "crew_member", "id": ctx.crew[&s(&b["referenceCrew"])] }),
    );
    for f in [
        "all",
        "unreviewed",
        "suggested",
        "missing",
        "needsReview",
        "complete",
        "incomplete",
    ] {
        g.ok("breakdown.scenes", json!({ "filter": f }));
    }
    g.ok("breakdown.scenes", json!({ "category": "VFX" }));
    g.ok("breakdown.scenes", json!({ "catalogItemId": item }));
    g.ok(
        "breakdown.scene_changes",
        json!({ "sceneId": ctx.scene(&b["changesScene"]) }),
    );
    g.ok("production.overview", json!({}));
    g.ok(
        "visual.scene_characters",
        json!({ "sceneId": ctx.scene(&b["charactersScene"]) }),
    );
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
    let v1 = g.id(&g.ok(
        "budget.save_snapshot",
        json!({ "id": id, "label": b["snapshotLabel"] }),
    ));
    ctx.set("budgetSnapshotV1", v1.clone());
    let sp = &b["spend"];
    if let Some(line) = lines.get(&s(&sp["description"])) {
        g.ok(
            "budget.update_line",
            json!({ "id": line, "category": sp["category"], "description": sp["description"], "planned": sp["planned"],
                    "actual": sp["actual"], "notes": sp["notes"], "expectedRev": null }),
        );
    }
    let mi = &b["mistake"];
    let mistake = g.id(&g.ok(
        "budget.add_line",
        json!({ "budgetId": id, "category": mi["category"], "description": mi["description"], "planned": mi["planned"], "actual": null, "notes": mi["notes"] }),
    ));
    g.ok("budget.delete_line", json!({ "id": mistake }));
    g.ok("budget.mark_reviewed", json!({ "id": id }));
    let rough = g.id(&g.ok(
        "budget.save_snapshot",
        json!({ "id": id, "label": b["roughLabel"] }),
    ));
    g.ok("budget.delete_snapshot", json!({ "id": rough }));
    g.ok("budget.get_snapshot", json!({ "id": v1 }));
    let view = g.ok("budget.get", json!({}));
    let c = &view["current"];
    ctx.facts.insert(
        "budget".into(),
        json!({ "currency": c["currency"], "plannedTotal": c["plannedTotal"], "plannedLinesTotal": c["plannedLinesTotal"],
                "actualTotal": c["actualTotal"], "contingencyAmount": c["contingencyAmount"], "difference": c["difference"], "lines": arr(&c["lines"]).len() }),
    );
}

/// The revision pages reach production: preview, apply, break down the new scene, review changes.
pub fn after_revision(g: &Gold, ctx: &mut Ctx) {
    g.phase("Production update to the revision pages");
    let blue = ctx.id("draftBlue");
    g.ok("production.preview_update", json!({ "draftId": blue }));
    g.ok(
        "production.apply_update",
        json!({ "draftId": blue, "reason": ctx.script["blue"]["reasonFinal"] }),
    );
    ctx.set("draftCurrent", blue);
    let old = ctx.lineage.clone();
    crate::script::map_scenes(g, ctx);
    ctx.lineage = old;
    let new = ctx.id("newScene");
    let ns = ctx.script["blue"]["newScene"].clone();
    if !new.is_empty() {
        g.ok("breakdown.suggest", json!({ "sceneId": new }));
        let d = g.ok("breakdown.scene", json!({ "sceneId": new }));
        let suggested = |wanted: bool| -> Vec<Value> {
            arr(&d["groups"])
                .iter()
                .filter(|grp| (s(&grp["category"]) != "Location / Set") == wanted)
                .flat_map(|grp| arr(&grp["suggestions"]))
                .map(|x| x["id"].clone())
                .collect()
        };
        let sets = suggested(false);
        if !sets.is_empty() {
            g.ok("breakdown.reject", json!({ "ids": sets }));
        }
        let ids = suggested(true);
        if let Some(first) = ids.first() {
            g.ok(
                "breakdown.accept",
                json!({ "id": first, "category": null, "name": null, "catalog": { "mode": "auto", "catalogItemId": null } }),
            );
        }
        if ids.len() > 1 {
            g.ok("breakdown.accept_many", json!({ "ids": ids[1..].to_vec() }));
        }
        let name = location_name(ctx, &s(&ns["location"]));
        add_element(g, ctx, &new, "Location / Set", &name, &ns["heading"]);
        for e in arr(&ctx.script["blue"]["breakdownNew"]) {
            add_element(
                g,
                ctx,
                &new,
                &s(&e["category"]),
                &s(&e["name"]),
                &Value::Null,
            );
        }
        g.ok("breakdown.mark_reviewed", json!({ "sceneId": new }));
        g.ok(
            "breakdown.set_complete",
            json!({ "sceneId": new, "complete": true }),
        );
    }
    let changed = g.ok("breakdown.scenes", json!({ "filter": "needsReview" }));
    for sc in arr(&changed["scenes"]) {
        let id = s(&sc["sceneId"]);
        g.ok("breakdown.scene_changes", json!({ "sceneId": id }));
        g.ok("breakdown.apply_suggested_update", json!({ "sceneId": id }));
        g.ok("breakdown.mark_reviewed", json!({ "sceneId": id }));
    }
    g.ok("budget.mark_reviewed", json!({ "id": ctx.id("budget") }));
    g.ok("production.overview", json!({}));
}
