//! Project notes, tasks, templates and files; then comments on production records, history,
//! Recently Deleted and search.

use serde_json::{Value, json};

use crate::Ctx;
use crate::gold::{Gold, arr, s};

/// Milliseconds since the Unix epoch for a `YYYY-MM-DD` date at 10:00 IST (04:30 UTC).
fn date_ms(d: &str) -> i64 {
    let p: Vec<i64> = d.split('-').filter_map(|x| x.parse().ok()).collect();
    if p.len() != 3 {
        return 0;
    }
    let (y, m, day) = (p[0], p[1], p[2]);
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
    let prod = ctx.production.clone();
    let mut note_ids = Vec::new();
    for n in arr(&prod["notes"]) {
        let id = g.id(&g.ok(
            "notes.create",
            json!({ "title": n["title"], "body": n["body"], "pinned": n["pinned"] }),
        ));
        note_ids.push((s(&n["title"]), id));
    }
    if let Some((_, first)) = note_ids.first() {
        g.ok("notes.get", json!({ "id": first }));
    }
    if let Some((_, second)) = note_ids.get(1) {
        ctx.set("noteHistory", second.clone());
    }
    if let Some((_, n)) = note_ids.get(3) {
        g.ok("notes.set_pinned", json!({ "id": n, "pinned": true }));
        g.ok("notes.set_pinned", json!({ "id": n, "pinned": false }));
    }
    let stale = g.id(&g.ok(
        "notes.create",
        json!({ "title": prod["trashRestoreTitle"], "body": prod["trashRestoreBody"], "pinned": false }),
    ));
    g.ok("notes.delete", json!({ "id": stale }));
    g.ok("notes.list", json!({ "text": null }));
    g.ok("notes.list", json!({ "text": "demo" }));

    g.phase("Tasks");
    g.ok("tasks.relatable_types", json!({}));
    let me = g.ok("project.members", json!({}));
    let owner = arr(&me)
        .first()
        .map(|m| s(&m["userId"]))
        .unwrap_or_default();
    let mut task_ids = Vec::new();
    for t in arr(&prod["tasks"]) {
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
            json!({ "id": t, "title": null, "dueAt": null, "clearDue": false, "ownerUserId": null,
                    "notes": "Moved a day earlier. Invented for this demo.", "related": null, "clearRelated": false, "expectedRev": null }),
        );
        g.ok("tasks.set_done", json!({ "id": t, "done": true }));
        g.ok("tasks.set_done", json!({ "id": t, "done": false }));
    }
    let dup = g.id(&g.ok(
        "tasks.create",
        json!({ "title": prod["dupTaskTitle"], "dueAt": null, "ownerUserId": null, "notes": null, "related": null }),
    ));
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
    let st = &prod["shotTemplate"];
    let mine = g.id(&g.ok(
        "templates.create",
        json!({ "scope": "global", "templateType": "shot_list", "name": st["draftName"],
                "content": { "description": st["description"], "shots": st["shots"] } }),
    ));
    g.ok(
        "templates.update",
        json!({ "scope": "global", "id": mine, "name": st["name"], "content": null }),
    );
    let in_project = g.ok(
        "templates.copy",
        json!({ "scope": "global", "id": mine, "toScope": "project", "name": null }),
    );
    g.ok(
        "templates.apply",
        json!({ "scope": "project", "id": g.id(&in_project), "sceneId": ctx.scene(&st["scene"]) }),
    );
    g.ok(
        "templates.apply",
        json!({ "scope": "builtin", "id": "builtin:shot_list", "sceneId": ctx.scene(&st["builtinScene"]) }),
    );
    let bt = &prod["budgetTemplate"];
    let lines = g.ok(
        "templates.create",
        json!({ "scope": "project", "templateType": "budget", "name": bt["name"], "content": { "description": bt["description"], "lines": bt["lines"] } }),
    );
    g.ok(
        "templates.apply",
        json!({ "scope": "project", "id": g.id(&lines), "currency": bt["currency"] }),
    );
    let old = g.ok(
        "templates.create",
        json!({ "scope": "global", "templateType": "call_sheet", "name": prod["oldTemplate"], "content": null }),
    );
    g.ok(
        "templates.copy",
        json!({ "scope": "builtin", "id": "builtin:call_sheet", "toScope": "global", "name": prod["copiedTemplate"] }),
    );
    g.ok(
        "templates.delete",
        json!({ "scope": "global", "id": g.id(&old) }),
    );
    g.ok("templates.list", json!({ "templateType": "shot_list" }));

    g.phase("Files");
    let f = &prod["files"];
    let fo = &f["folders"];
    let contracts = g.id(&g.ok(
        "files.create_folder",
        json!({ "name": fo["contracts"], "parentId": null }),
    ));
    let nested = g.id(&g.ok(
        "files.create_folder",
        json!({ "name": fo["nested"], "parentId": contracts }),
    ));
    let music = g.id(&g.ok(
        "files.create_folder",
        json!({ "name": fo["music"], "parentId": null }),
    ));
    let empty = g.id(&g.ok(
        "files.create_folder",
        json!({ "name": fo["empty"], "parentId": null }),
    ));
    g.ok(
        "files.rename_folder",
        json!({ "id": contracts, "name": fo["contractsNew"], "expectedRev": null }),
    );
    let rel = &f["release"];
    let added = g.ok(
        "files.add",
        json!({ "paths": [ctx.media_path(&s(&rel["file"]))], "folderId": contracts, "mode": "copy" }),
    );
    let release = arr(&added).first().map(|x| s(&x["id"])).unwrap_or_default();
    let cue = g.ok(
        "files.add",
        json!({ "paths": [ctx.media_path(&s(&f["cue"]["file"]))], "folderId": music, "mode": "copy" }),
    );
    let cue_id = arr(&cue).first().map(|x| s(&x["id"])).unwrap_or_default();
    // A document on a shared folder beside the output: linked, then relinked when it moved.
    let li = &f["linked"];
    let share = ctx.out.join("shared-drive");
    std::fs::create_dir_all(&share).ok();
    let src = share.join(s(&li["name"]));
    std::fs::copy(ctx.media_path(&s(&li["file"])), &src).ok();
    let linked = g.ok(
        "files.add",
        json!({ "paths": [src.to_string_lossy()], "folderId": null, "mode": "link" }),
    );
    let linked_id = arr(&linked)
        .first()
        .map(|x| s(&x["id"]))
        .unwrap_or_default();
    let moved = share.join(s(&li["folder"])).join(s(&li["moved"]));
    std::fs::create_dir_all(moved.parent().unwrap()).ok();
    std::fs::rename(&src, &moved).ok();
    g.ok(
        "files.relink",
        json!({ "id": linked_id, "path": moved.to_string_lossy() }),
    );
    g.ok("files.move", json!({ "id": release, "folderId": nested }));
    g.ok(
        "files.rename",
        json!({ "id": release, "name": rel["rename"], "expectedRev": null }),
    );
    g.ok(
        "files.update_notes",
        json!({ "id": release, "notes": rel["notes"] }),
    );
    g.ok(
        "files.update_notes",
        json!({ "id": cue_id, "notes": f["cue"]["notes"] }),
    );
    g.ok(
        "files.export_copy",
        json!({ "id": release, "destPath": ctx.outp(&format!("exports/files/{}", s(&rel["rename"]))) }),
    );
    let wrong = g.ok(
        "files.add",
        json!({ "paths": [ctx.media_path(&s(&f["wrong"]))], "folderId": contracts, "mode": "copy" }),
    );
    if let Some(w) = arr(&wrong).first() {
        g.ok("files.delete", json!({ "id": w["id"] }));
    }
    g.ok("files.delete_folder", json!({ "id": empty }));
    let listing = g.ok("files.list", json!({}));
    if let Some(asset) = arr(&listing["files"])
        .iter()
        .find_map(|x| x["asset"]["id"].as_str().map(str::to_string))
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
    let prod = ctx.production.clone();
    let cm = &prod["comments"];
    let shots: Vec<String> = ctx
        .id("focusShots")
        .split(',')
        .filter(|x| !x.is_empty())
        .map(str::to_string)
        .collect();
    let create = |ty: &str, id: &str, body: &Value| {
        g.ok(
            "comment.create",
            json!({ "targetType": ty, "targetId": id, "body": body, "anchor": null, "sceneId": null, "reviewRoundId": null }),
        )
    };
    if let Some(id) = ctx.catalog.get(&s(&cm["catalog"]["name"])) {
        create("catalog_item", id, &cm["catalog"]["body"]);
    }
    if let Some(id) = ctx.locations.get(&s(&cm["location"]["key"])) {
        let c = create("location", id, &cm["location"]["body"]);
        g.ok(
            "comment.reply",
            json!({ "parentId": g.id(&c), "body": cm["location"]["reply"] }),
        );
    }
    if let Some(id) = shots.first() {
        create("shot", id, &cm["shot"]["body"]);
    }

    g.phase("History: undo and redo");
    g.ok("history.info", json!({ "store": "project" }));
    let note = ctx.id("noteHistory");
    g.ok(
        "notes.update",
        json!({ "id": note, "title": prod["historyNoteTitle"], "body": null, "expectedRev": null }),
    );
    g.ok("history.undo", json!({ "store": "project" }));
    g.ok("history.redo", json!({ "store": "project" }));
    g.ok("history.undo", json!({ "store": "project" }));
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
        .insert("activityEntriesRead".into(), json!(arr(&act).len()));
    g.ok(
        "history.activity",
        json!({ "store": "global", "limit": 20 }),
    );

    g.phase("Recently Deleted");
    let trash = g.ok("trash.list", json!({ "store": "project" }));
    g.ok("project.deleted_items", json!({ "store": "project" }));
    let restore_title = s(&prod["trashRestoreTitle"]);
    let purge_title = s(&prod["dupTaskTitle"]);
    for r in arr(&trash) {
        let label = r.to_string();
        if label.contains(&restore_title) {
            g.ok(
                "trash.restore",
                json!({ "store": "project", "id": r["id"] }),
            );
        }
        if label.contains(&purge_title) {
            g.ok("trash.purge", json!({ "store": "project", "id": r["id"] }));
        }
    }
    // One note stays in Recently Deleted so the page has something to restore.
    let keep = g.id(&g.ok(
        "notes.create",
        json!({ "title": "Old call for extras (restore me)", "body": "Left in Recently Deleted on purpose, to show the Restore button.", "pinned": false }),
    ));
    g.ok("notes.delete", json!({ "id": keep }));
    let after = g.ok("trash.list", json!({ "store": "project" }));
    ctx.facts
        .insert("trashRowsLeft".into(), json!(arr(&after).len()));
    g.ok("trash.list", json!({ "store": "global" }));

    g.phase("Search");
    g.ok("search.rebuild", json!({}));
    for q in arr(&prod["searchTerms"]) {
        let hits = g.ok(
            "search.query",
            json!({ "text": q, "includeGlobal": false, "entityTypes": null, "limit": 50, "globalOnly": false }),
        );
        ctx.facts
            .insert(format!("search:{}", s(&q)), json!(arr(&hits).len()));
    }
    let gt = arr(&prod["searchGlobal"]);
    g.ok("search.query", json!({ "text": gt[0], "includeGlobal": true, "entityTypes": null, "limit": 20, "globalOnly": false }));
    g.ok("search.query", json!({ "text": gt[1], "includeGlobal": true, "entityTypes": null, "limit": 20, "globalOnly": true }));
    g.ok(
        "search.query",
        json!({ "text": prod["searchVaultTerm"], "includeGlobal": false, "entityTypes": ["vault_item"], "limit": 20, "globalOnly": false }),
    );
}
