//! Visual planning from `visual.json`: moodboards, shot lists and storyboards.

use std::collections::BTreeMap;

use base64::Engine;
use serde_json::{Value, json};

use crate::Ctx;
use crate::gold::{Gold, arr, s};

fn b64_file(ctx: &Ctx, name: &str) -> String {
    base64::engine::general_purpose::STANDARD
        .encode(std::fs::read(ctx.media.join(name)).unwrap_or_default())
}

pub fn run(g: &Gold, ctx: &mut Ctx) {
    moodboards(g, ctx);
    shots(g, ctx);
    storyboards(g, ctx);
    g.phase("Visual overview");
    let vs = g.ok("visual.scenes", json!({}));
    for n in arr(&ctx.visual["reviewedScenes"]) {
        if let Some(l) = ctx.lineage.get(&(n.as_u64().unwrap_or(0) as u32)) {
            g.ok("visual.mark_scene_reviewed", json!({ "sceneLineageId": l }));
        }
    }
    ctx.facts
        .insert("visualScenes".into(), json!(arr(&vs["scenes"]).len()));
}

fn moodboards(g: &Gold, ctx: &mut Ctx) {
    g.phase("Moodboards");
    let v = ctx.visual.clone();
    let mut ids: BTreeMap<String, String> = BTreeMap::new();
    for (i, b) in arr(&v["moodboards"]).iter().enumerate() {
        let mut args = json!({ "name": b["name"] });
        if !b["scene"].is_null() {
            args["sceneId"] = json!(ctx.scene(&b["scene"]));
        }
        let id = g.id(&g.ok("moodboard.create", args));
        ids.insert(s(&b["key"]), id.clone());
        ctx.set(&format!("mood:{i}"), id.clone());
        let paths: Vec<String> = arr(&b["images"])
            .iter()
            .map(|p| ctx.media_path(&s(p)))
            .collect();
        let imgs = g.ok(
            "moodboard.add_images",
            json!({ "moodboardId": id, "paths": paths, "x": 40, "y": 40 }),
        );
        let items: Vec<String> = arr(&imgs).iter().map(|x| s(&x["id"])).collect();
        if !b["pasted"].is_null() {
            g.ok(
                "moodboard.add_image_data",
                json!({ "moodboardId": id, "dataBase64": b64_file(ctx, &s(&b["pasted"])), "fileName": b["pastedName"], "x": 980, "y": 40 }),
            );
        }
        if b["vault"].as_bool().unwrap_or(false) && !ctx.id("vaultImage").is_empty() {
            g.ok(
                "moodboard.add_vault_image",
                json!({ "moodboardId": id, "vaultItemId": ctx.id("vaultImage"), "store": "project", "x": 980, "y": 600 }),
            );
        }
        if !b["link"].is_null() {
            let link = g.ok(
                "moodboard.add_link",
                json!({ "moodboardId": id, "url": b["link"]["url"], "title": b["link"]["title"], "x": 40, "y": 1100 }),
            );
            g.ok(
                "moodboard.update_item",
                json!({ "id": g.id(&link), "linkTitle": b["link"]["rename"] }),
            );
        }
        let note = g.ok(
            "moodboard.add_note",
            json!({ "moodboardId": id, "text": b["note"], "x": 600, "y": 1100 }),
        );
        if !b["noteEdit"].is_null() {
            g.ok(
                "moodboard.update_item",
                json!({ "id": g.id(&note), "body": b["noteEdit"] }),
            );
        }
        if !b["privateNote"].is_null() {
            g.ok(
                "moodboard.add_note",
                json!({ "moodboardId": id, "text": b["privateNote"], "isPrivate": true, "x": 1200, "y": 1100 }),
            );
        }
        if let (Some(first), false) = (items.first(), b["caption0"].is_null()) {
            g.ok(
                "moodboard.update_item",
                json!({ "id": first, "caption": b["caption0"] }),
            );
            g.ok(
                "moodboard.resize_item",
                json!({ "id": first, "w": 520, "h": 300 }),
            );
            g.ok(
                "moodboard.arrange_item",
                json!({ "id": first, "placement": "front" }),
            );
        }
        if let (Some(second), false) = (items.get(1), b["caption1"].is_null()) {
            g.ok(
                "moodboard.update_item",
                json!({ "id": second, "caption": b["caption1"] }),
            );
            g.ok(
                "moodboard.arrange_item",
                json!({ "id": second, "placement": "back" }),
            );
        }
        if !b["caption0"].is_null() {
            let moves: Vec<Value> = items
                .iter()
                .enumerate()
                .map(|(i, x)| json!({ "id": x, "x": 40 + i as i64 * 560, "y": 420 }))
                .collect();
            g.ok("moodboard.move_items", json!({ "moves": moves }));
        }
        if let Some(i) = b["dropIndex"].as_u64()
            && let Some(x) = items.get(i as usize)
        {
            g.ok("moodboard.delete_items", json!({ "ids": [x] }));
        }
        if !b["notes"].is_null() {
            g.ok(
                "moodboard.update_notes",
                json!({ "id": id, "notes": b["notes"] }),
            );
        }
        if !b["rename"].is_null() {
            g.ok("moodboard.rename", json!({ "id": id, "name": b["rename"] }));
        }
    }
    let gone = g.id(&g.ok("moodboard.create", json!({ "name": v["deletedMoodboard"] })));
    g.ok("moodboard.delete", json!({ "id": gone }));
    g.ok(
        "moodboard.reorder",
        json!({ "id": ids[&s(&v["reorder"]["key"])], "index": v["reorder"]["index"] }),
    );
    g.ok("moodboard.list", json!({}));
    g.ok("moodboard.suggestions", json!({}));
    g.ok("moodboard.get", json!({ "id": ctx.id("mood:0") }));
}

fn shots(g: &Gold, ctx: &mut Ctx) {
    g.phase("Shot lists");
    let v = ctx.visual.clone();
    let lists = v["shots"].as_object().cloned().unwrap_or_default();
    for (n, list) in &lists {
        let Ok(num) = n.parse::<u32>() else { continue };
        let Some(scene) = ctx.scenes.get(&num).cloned() else {
            continue;
        };
        for sh in arr(list) {
            g.ok(
                "shot.create",
                json!({ "sceneId": scene, "description": sh["desc"], "size": sh["size"], "movement": sh["movement"], "angle": sh["angle"],
                        "lens": sh["lens"], "cameraNotes": sh["notes"], "characters": sh["chars"], "soundNote": sh["sound"] }),
            );
        }
    }
    let f = &v["shotFocus"];
    let focus_scene = ctx.scene(&f["scene"]);
    let list = g.ok("shot.list", json!({ "sceneId": focus_scene }));
    let ids: Vec<String> = arr(&list).iter().map(|x| s(&x["id"])).collect();
    if let Some(sh) = ids.get(f["update"]["index"].as_u64().unwrap_or(1) as usize) {
        g.ok(
            "shot.update",
            json!({ "id": sh, "cameraNotes": f["update"]["notes"], "lens": f["update"]["lens"] }),
        );
    }
    let ro = arr(&f["reorder"]);
    if let Some(sh) = ids.get(ro[0].as_u64().unwrap_or(0) as usize) {
        g.ok("shot.reorder", json!({ "id": sh, "index": ro[1] }));
    }
    if let Some(sh) = ids.first() {
        let dup = g.ok("shot.duplicate", json!({ "id": sh }));
        g.ok(
            "shot.update",
            json!({ "id": g.id(&dup), "description": f["dup"]["desc"], "lens": f["dup"]["lens"] }),
        );
        g.ok(
            "shot.set_reference_image",
            json!({ "id": sh, "path": ctx.media_path(&s(&f["referenceImage"])) }),
        );
        g.ok("shot.get", json!({ "id": sh }));
    }
    let st = &f["stray"];
    let stray = g.ok(
        "shot.create",
        json!({ "sceneId": ctx.scene(&st["fromScene"]), "description": st["desc"], "size": st["size"], "movement": st["movement"], "characters": [] }),
    );
    g.ok(
        "shot.move",
        json!({ "id": g.id(&stray), "sceneId": focus_scene, "index": ids.len() }),
    );
    let spare = g.ok(
        "shot.create",
        json!({ "sceneId": focus_scene, "description": f["spare"]["desc"], "size": f["spare"]["size"] }),
    );
    g.ok("shot.delete", json!({ "ids": [g.id(&spare)] }));
    let cp = &v["copyPlanning"];
    let from = cp["fromScene"].as_u64().unwrap_or(0) as u32;
    if let Some(l) = ctx.lineage.get(&from) {
        g.ok(
            "shot.copy_planning",
            json!({ "fromSceneLineageId": l, "toSceneId": ctx.scene(&cp["toScene"]) }),
        );
    }
    g.ok("shot.list", json!({ "includeRemoved": true }));
    ctx.set("focusShots", ids.join(","));
}

fn panel_args(ctx: &Ctx, board: &str, p: &Value) -> Value {
    let visual = s(&p["visual"]);
    let mut a = json!({ "storyboardId": board, "visual": visual, "description": p["desc"] });
    match visual.as_str() {
        "sketch" => a["dataBase64"] = json!(b64_file(ctx, &s(&p["file"]))),
        "image" => a["path"] = json!(ctx.media_path(&s(&p["file"]))),
        _ => {}
    }
    a
}

fn storyboards(g: &Gold, ctx: &mut Ctx) {
    g.phase("Storyboards");
    let v = ctx.visual.clone();
    let mut boards: BTreeMap<String, String> = BTreeMap::new();
    let mut panels: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for b in arr(&v["storyboards"]) {
        let mut args = json!({});
        if !b["scene"].is_null() {
            args["sceneId"] = json!(ctx.scene(&b["scene"]));
        }
        if !b["name"].is_null() {
            args["name"] = b["name"].clone();
        }
        let id = g.id(&g.ok("storyboard.create", args));
        boards.insert(s(&b["key"]), id.clone());
        let mut ps = Vec::new();
        for p in arr(&b["panels"]) {
            let made = g.ok("storyboard.add_panel", panel_args(ctx, &id, &p));
            let pid = g.id(&made);
            if !p["framing"].is_null() {
                g.ok(
                    "storyboard.update_panel",
                    json!({ "id": pid, "framing": p["framing"], "movement": p["movement"], "angle": p["angle"], "soundNote": p["sound"],
                            "durationMs": p["ms"], "note": p["note"] }),
                );
            }
            if let Some(later) = p["laterSketch"].as_str() {
                g.ok(
                    "storyboard.set_panel_visual",
                    json!({ "id": pid, "visual": "sketch", "dataBase64": b64_file(ctx, later) }),
                );
            }
            ps.push(pid);
        }
        panels.insert(s(&b["key"]), ps);
    }
    let focus_key = s(&v["focusBoard"]);
    let focus = panels.get(&focus_key).cloned().unwrap_or_default();
    if focus.len() >= 4 {
        g.ok(
            "storyboard.reorder_panel",
            json!({ "id": focus[2], "index": 1 }),
        );
        g.ok(
            "storyboard.reorder_panel",
            json!({ "id": focus[2], "index": 2 }),
        );
        let shots: Vec<String> = ctx
            .id("focusShots")
            .split(',')
            .filter(|x| !x.is_empty())
            .map(str::to_string)
            .collect();
        if let Some(sh) = shots.first() {
            g.ok(
                "storyboard.link_shot",
                json!({ "panelId": focus[0], "shotId": sh }),
            );
            g.ok(
                "shot.attach_panel",
                json!({ "shotId": sh, "panelId": focus[1] }),
            );
            g.ok(
                "shot.detach_panel",
                json!({ "shotId": sh, "panelId": focus[1] }),
            );
        }
        g.ok(
            "storyboard.create_shot_from_panel",
            json!({ "panelId": focus[3] }),
        );
        if let Some(sh) = shots.get(2) {
            g.ok("shot.create_panel", json!({ "id": sh }));
        }
        g.ok(
            "storyboard.link_shot",
            json!({ "panelId": focus[2], "shotId": null }),
        );
    }
    for b in arr(&v["storyboards"]) {
        let id = boards[&s(&b["key"])].clone();
        if !b["straypanel"].is_null() {
            let sp = &b["straypanel"];
            let stray = g.ok(
                "storyboard.add_panel",
                json!({ "storyboardId": id, "visual": "image", "path": ctx.media_path(&s(&sp["file"])), "description": sp["desc"] }),
            );
            g.ok(
                "storyboard.move_panel",
                json!({ "id": g.id(&stray), "storyboardId": boards[&s(&sp["to"])], "index": 0 }),
            );
        }
        if !b["attachScene"].is_null() {
            g.ok(
                "storyboard.set_scene",
                json!({ "id": id, "sceneId": ctx.scene(&b["attachScene"]) }),
            );
        }
        if !b["rename"].is_null() {
            g.ok(
                "storyboard.rename",
                json!({ "id": id, "name": b["rename"] }),
            );
        }
    }
    let spare_board = boards[&s(&arr(&v["storyboards"])[1]["key"])].clone();
    let spare = g.ok(
        "storyboard.add_panel",
        json!({ "storyboardId": spare_board, "visual": "placeholder", "description": "spare" }),
    );
    g.ok("storyboard.delete_panels", json!({ "ids": [g.id(&spare)] }));
    let old = g.id(&g.ok("storyboard.create", json!({ "name": v["deletedBoard"] })));
    g.ok("storyboard.delete", json!({ "id": old }));
    g.ok("storyboard.list", json!({}));
    let focus_id = boards[&focus_key].clone();
    g.ok("storyboard.get", json!({ "id": focus_id }));
    ctx.set("boardFocus", focus_id);
}
