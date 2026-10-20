//! Project notes, tasks, templates and files; then comments on production records, history
//! (undo and redo), Recently Deleted and search.

use serde_json::{Value, json};

use crate::Ctx;
use crate::gold::{Gold, arr, s};

/// Milliseconds since the Unix epoch for a `YYYY-MM-DD` date at 10:00 IST (04:30 UTC).
pub fn date_ms(d: &str) -> i64 {
    let p: Vec<i64> = d.split('-').filter_map(|x| x.parse().ok()).collect();
    if p.len() != 3 {
        return 0;
    }
    let (y, m, day) = (p[0], p[1], p[2]);
    // Days from civil (Howard Hinnant's algorithm).
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = if y2 >= 0 { y2 } else { y2 - 399 } / 400;
    let yoe = y2 - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    days * 86_400_000 + (4 * 60 + 30) * 60_000
}

fn related(ctx: &Ctx, r: &Value) -> Value {
    let Some(r) = r.as_str() else {
        return Value::Null;
    };
    let (kind, key) = r.split_once(':').unwrap_or(("", ""));
    let (ty, id) = match kind {
        "catalog" => ("catalog_item", ctx.catalog.get(key).cloned()),
        "location" => ("location", ctx.locations.get(key).cloned()),
        "cast" => ("cast_member", ctx.cast.get(key).cloned()),
        "scene" => (
            "screenplay_scene",
            key.parse::<u32>()
                .ok()
                .and_then(|n| ctx.scenes.get(&n).cloned()),
        ),
        _ => ("", None),
    };
    match id {
        Some(id) if !id.is_empty() => json!({ "targetType": ty, "targetId": id }),
        _ => Value::Null,
    }
}

pub fn notes_tasks_templates_files(g: &Gold, ctx: &mut Ctx) {
    g.phase("Project notes");
    let mut note_ids = Vec::new();
    for n in arr(&ctx.production["notes"]) {
        let id = g.id(&g.ok(
            "notes.create",
            json!({ "title": n["title"], "body": n["body"], "pinned": n["pinned"] }),
        ));
        note_ids.push(id);
    }
    if let Some(first) = note_ids.first() {
        g.ok("notes.update", json!({ "id": first, "title": null, "body": format!("{}\n\nUpdated after the table read on 6 January 2027: the 1999 scenes stay in the same aspect ratio.", s(&arr(&ctx.production["notes"])[0]["body"])), "expectedRev": null }));
        g.ok("notes.get", json!({ "id": first }));
    }
    if let Some(n) = note_ids.get(3) {
        g.ok("notes.set_pinned", json!({ "id": n, "pinned": true }));
    }
    let stale = g.id(&g.ok("notes.create", json!({ "title": "Recce list (superseded)", "body": "Replaced by the location list in Production.", "pinned": false })));
    g.ok("notes.delete", json!({ "id": stale }));
    g.ok("notes.list", json!({ "text": null }));
    g.ok("notes.list", json!({ "text": "safety" }));
    ctx.set(
        "noteDirector",
        note_ids.first().cloned().unwrap_or_default(),
    );

    g.phase("Tasks");
    g.ok("tasks.relatable_types", json!({}));
    let me = g.ok("project.members", json!({}));
    let owner = arr(&me)
        .first()
        .map(|m| s(&m["userId"]))
        .unwrap_or_default();
    let mut task_ids = Vec::new();
    for t in arr(&ctx.production["tasks"]) {
        let id = g.id(&g.ok(
            "tasks.create",
            json!({ "title": t["title"], "dueAt": date_ms(&s(&t["due"])), "ownerUserId": if owner.is_empty() { Value::Null } else { json!(owner) },
                    "notes": t["notes"], "related": related(ctx, &t["related"]) }),
        ));
        if t["done"].as_bool().unwrap_or(false) {
            g.ok("tasks.set_done", json!({ "id": id, "done": true }));
        }
        task_ids.push(id);
    }
    if let Some(t) = task_ids.first() {
        g.ok(
            "tasks.update",
            json!({ "id": t, "title": null, "dueAt": date_ms("2027-01-03"), "clearDue": false, "ownerUserId": null,
                    "notes": "Pick-up by road on 2027-01-05; insured in transit. Moved a day earlier.", "related": null, "clearRelated": false, "expectedRev": null }),
        );
        g.ok("tasks.set_done", json!({ "id": t, "done": true }));
        g.ok("tasks.set_done", json!({ "id": t, "done": false }));
    }
    let dup = g.id(&g.ok("tasks.create", json!({ "title": "Book the Nagra (duplicate)", "dueAt": null, "ownerUserId": null, "notes": null, "related": null })));
    g.ok("tasks.delete", json!({ "id": dup }));
    g.ok(
        "tasks.list",
        json!({ "status": "Open", "ownerUserId": null }),
    );
    g.ok("tasks.list", json!({ "status": null, "ownerUserId": null }));

    g.phase("Templates");
    g.ok("templates.list", json!({ "templateType": null }));
    g.ok(
        "templates.get",
        json!({ "scope": "builtin", "id": "builtin:shot_list" }),
    );
    let mine = g.ok(
        "templates.create",
        json!({ "scope": "global", "templateType": "shot_list", "name": "Kayal dialogue coverage",
                "content": { "description": "Master, two singles, a reaction and the insert the scene turns on.",
                             "shots": [ { "description": "Master", "size": "Wide" }, { "description": "Single on the first speaker", "size": "Medium close-up" },
                                        { "description": "Single on the second speaker", "size": "Medium close-up" }, { "description": "Reaction", "size": "Close-up" },
                                        { "description": "Insert of the detail the scene turns on", "size": "Insert" } ] } }),
    );
    let mine_id = g.id(&mine);
    g.ok("templates.update", json!({ "scope": "global", "id": mine_id, "name": "Kayal dialogue coverage (five shots)", "content": null }));
    let in_project = g.ok(
        "templates.copy",
        json!({ "scope": "global", "id": mine_id, "toScope": "project", "name": null }),
    );
    g.ok(
        "templates.apply",
        json!({ "scope": "project", "id": g.id(&in_project), "sceneId": ctx.scenes[&19] }),
    );
    g.ok(
        "templates.apply",
        json!({ "scope": "builtin", "id": "builtin:shot_list", "sceneId": ctx.scenes[&8] }),
    );
    let marine = g.ok(
        "templates.create",
        json!({ "scope": "project", "templateType": "budget", "name": "Marine unit lines",
                "content": { "description": "Extra lines for any day at sea.", "lines": [
                    { "category": "Equipment", "description": "Marine: Coast Guard liaison officer (sea day)", "planned": 2500000 },
                    { "category": "Travel/Transport", "description": "Marine: fuel for the camera and support boats", "planned": 1800000 } ] } }),
    );
    g.ok(
        "templates.apply",
        json!({ "scope": "project", "id": g.id(&marine), "currency": "INR" }),
    );
    let old = g.ok("templates.create", json!({ "scope": "global", "templateType": "call_sheet", "name": "Old call sheet (2024 short)", "content": null }));
    g.ok("templates.copy", json!({ "scope": "builtin", "id": "builtin:call_sheet", "toScope": "global", "name": "Indie call sheet (my copy)" }));
    g.ok(
        "templates.delete",
        json!({ "scope": "global", "id": g.id(&old) }),
    );
    g.ok("templates.list", json!({ "templateType": "shot_list" }));

    g.phase("Files");
    let contracts = g.id(&g.ok(
        "files.create_folder",
        json!({ "name": "Contracts and releases", "parentId": null }),
    ));
    let permits = g.id(&g.ok(
        "files.create_folder",
        json!({ "name": "Permits", "parentId": contracts }),
    ));
    let music = g.id(&g.ok(
        "files.create_folder",
        json!({ "name": "Music", "parentId": null }),
    ));
    let empty = g.id(&g.ok(
        "files.create_folder",
        json!({ "name": "Scans", "parentId": null }),
    ));
    g.ok(
        "files.rename_folder",
        json!({ "id": contracts, "name": "Contracts, releases and permits", "expectedRev": null }),
    );
    let added = g.ok(
        "files.add",
        json!({ "paths": [ctx.media.path("location-release-coast-radio-station.pdf")], "folderId": contracts, "mode": "copy" }),
    );
    let release = arr(&added).first().map(|f| s(&f["id"])).unwrap_or_default();
    let cue = g.ok("files.add", json!({ "paths": [ctx.media.path("music-cue-sheet.csv")], "folderId": music, "mode": "copy" }));
    let cue_id = arr(&cue).first().map(|f| s(&f["id"])).unwrap_or_default();
    // The research PDF stays on the shared drive: linked, then relinked when the share moved.
    let share = g.root.join("production-share");
    std::fs::create_dir_all(&share).ok();
    let src = share.join("coast-radio-research.pdf");
    std::fs::copy(ctx.media.path("research-coast-radio-1999.pdf"), &src).ok();
    let linked = g.ok(
        "files.add",
        json!({ "paths": [src.to_string_lossy()], "folderId": null, "mode": "link" }),
    );
    let linked_id = arr(&linked)
        .first()
        .map(|f| s(&f["id"]))
        .unwrap_or_default();
    let moved = share.join("research").join("coast-radio-research-1999.pdf");
    std::fs::create_dir_all(moved.parent().unwrap()).ok();
    std::fs::rename(&src, &moved).ok();
    g.ok(
        "files.relink",
        json!({ "id": linked_id, "path": moved.to_string_lossy() }),
    );
    g.ok("files.move", json!({ "id": release, "folderId": permits }));
    g.ok("files.rename", json!({ "id": release, "name": "Location release, Coast Radio Station (signed).pdf", "expectedRev": null }));
    g.ok("files.update_notes", json!({ "id": release, "notes": "Signed by the Port Trust estate officer on 2026-12-18. Original in the production office safe." }));
    g.ok(
        "files.update_notes",
        json!({ "id": cue_id, "notes": "Draft cue sheet from Deepak Menon, 4 January 2027." }),
    );
    g.ok("files.export_copy", json!({ "id": release, "destPath": ctx.outp("files/location-release-coast-radio-station.pdf") }));
    let wrong = g.ok("files.add", json!({ "paths": [ctx.media.path("palette-1999.png")], "folderId": contracts, "mode": "copy" }));
    if let Some(w) = arr(&wrong).first() {
        g.ok("files.delete", json!({ "id": w["id"] }));
    }
    g.ok("files.delete_folder", json!({ "id": empty }));
    let listing = g.ok("files.list", json!({}));
    if let Some(asset) = arr(&listing["files"])
        .iter()
        .find_map(|f| f["asset"]["id"].as_str().map(str::to_string))
    {
        g.ok(
            "files.asset",
            json!({ "assetId": asset, "store": "project" }),
        );
    }
    ctx.facts
        .insert("filesCount".into(), json!(arr(&listing["files"]).len()));
}

pub fn comments_history_trash_search(g: &Gold, ctx: &mut Ctx) {
    g.phase("Comments on production records");
    let shots: Vec<String> = ctx
        .id("shots24")
        .split(',')
        .filter(|x| !x.is_empty())
        .map(str::to_string)
        .collect();
    let targets: Vec<(&str, String, &str)> = vec![
        (
            "catalog_item",
            ctx.catalog
                .get("Quarter-inch reel 14-11-1999")
                .cloned()
                .unwrap_or_default(),
            "Hero reel must never leave the props case between takes.",
        ),
        (
            "location",
            ctx.locations.get("breakwater").cloned().unwrap_or_default(),
            "Safety officer walked it on 8 January. Yellow line painted.",
        ),
        (
            "shot",
            shots.first().cloned().unwrap_or_default(),
            "Crane needs a level base on the harbour road; grip to bring sandbags.",
        ),
    ];
    for (ty, id, body) in targets {
        if id.is_empty() {
            continue;
        }
        let c = g.ok("comment.create", json!({ "targetType": ty, "targetId": id, "body": body, "anchor": null, "sceneId": null, "reviewRoundId": null }));
        if ty == "location" {
            g.ok(
                "comment.reply",
                json!({ "parentId": g.id(&c), "body": "Confirmed with Captain Joy Antony." }),
            );
        }
    }

    g.phase("History: undo and redo");
    g.ok("history.info", json!({ "store": "project" }));
    let note = ctx.id("noteDirector");
    g.ok("notes.update", json!({ "id": note, "title": "Director's statement (DRAFT)", "body": null, "expectedRev": null }));
    g.ok("history.undo", json!({ "store": "project" }));
    g.ok("history.redo", json!({ "store": "project" }));
    g.ok("history.undo", json!({ "store": "project" }));
    // Undo across modules: a shot moved, then the move undone.
    if let Some(sh) = shots.get(1) {
        g.ok("shot.reorder", json!({ "id": sh, "index": 0 }));
        g.ok("history.undo", json!({ "store": "project" }));
    }
    g.ok("history.info", json!({ "store": "global" }));
    let act = g.ok(
        "history.activity",
        json!({ "store": "project", "limit": 200 }),
    );
    ctx.facts
        .insert("activityEntries".into(), json!(arr(&act).len()));
    g.ok(
        "history.activity",
        json!({ "store": "global", "limit": 20 }),
    );

    g.phase("Recently Deleted");
    let trash = g.ok("trash.list", json!({ "store": "project" }));
    g.ok("project.deleted_items", json!({ "store": "project" }));
    let rows = arr(&trash);
    ctx.facts.insert(
        "trashTypes".into(),
        json!(
            rows.iter()
                .map(|r| s(&r["objectType"]) + &s(&r["label"]))
                .collect::<Vec<_>>()
        ),
    );
    // The deleted Recce note comes back (the location manager still wanted it); the duplicate
    // task is removed for good.
    for r in &rows {
        let label = format!("{} {} {}", s(&r["title"]), s(&r["name"]), s(&r["label"]));
        if label.contains("Recce list") {
            g.ok(
                "trash.restore",
                json!({ "store": "project", "id": r["id"] }),
            );
        }
        if label.contains("Book the Nagra (duplicate)") {
            g.ok("trash.purge", json!({ "store": "project", "id": r["id"] }));
        }
    }
    g.ok("trash.list", json!({ "store": "global" }));

    g.phase("Search");
    g.ok("search.rebuild", json!({}));
    for q in ["reel", "Mariamma", "breakwater", "Ambassador", "fifty-four"] {
        let hits = g.ok("search.query", json!({ "text": q, "includeGlobal": false, "entityTypes": null, "limit": 50, "globalOnly": false }));
        ctx.facts
            .insert(format!("search:{q}"), json!(arr(&hits).len()));
    }
    g.ok("search.query", json!({ "text": "static", "includeGlobal": true, "entityTypes": null, "limit": 20, "globalOnly": false }));
    g.ok("search.query", json!({ "text": "VHF", "includeGlobal": true, "entityTypes": null, "limit": 20, "globalOnly": true }));
    g.ok("search.query", json!({ "text": "1999", "includeGlobal": false, "entityTypes": ["vault_item"], "limit": 20, "globalOnly": false }));
}
