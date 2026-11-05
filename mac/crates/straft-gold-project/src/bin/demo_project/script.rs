//! The screenplay: an outline built from the Beat Board, the Final Draft file imported through the
//! app's own import operation, a First Draft that carries the imported text and keeps the Beat Board
//! links, then corrected drafts, the review round, the lock and the revision.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::Ctx;
use crate::gold::{Gold, arr, s};

pub fn document(g: &Gold, draft: &str) -> Value {
    g.ok("screenplay.document", json!({ "draftId": draft }))
}

/// Scene number -> scene id in the current working draft, found by lineage or by the source card.
pub fn map_scenes(g: &Gold, ctx: &mut Ctx) {
    let doc = document(g, &ctx.id("draftCurrent"));
    let by_lineage: BTreeMap<String, u32> =
        ctx.lineage.iter().map(|(n, l)| (l.clone(), *n)).collect();
    let by_card: BTreeMap<String, u32> = ctx.cards.iter().map(|(n, c)| (c.clone(), *n)).collect();
    ctx.scenes.clear();
    for sc in arr(&doc["scenes"]) {
        let lineage = s(&sc["lineageId"]);
        let n = by_lineage
            .get(&lineage)
            .copied()
            .or_else(|| by_card.get(&s(&sc["sourceSceneCardId"])).copied());
        if let Some(n) = n {
            ctx.scenes.insert(n, s(&sc["id"]));
            ctx.lineage.insert(n, lineage);
        }
    }
}

pub fn build_outline(g: &Gold, ctx: &mut Ctx) {
    g.phase("Screenplay: outline built from the Beat Board");
    let sc = ctx.script.clone();
    let p = ctx.project.clone();
    let preview = g.ok("story.build_preview", json!({ "episodeId": null }));
    let include: Vec<Value> = arr(&preview["rows"])
        .iter()
        .map(|r| json!({ "cardId": r["cardId"], "heading": null }))
        .collect();
    let built = g.ok(
        "story.build_screenplay",
        json!({ "episodeId": null, "include": include, "descriptionMode": "planningNote", "destination": "newScreenplay",
                "screenplayId": null, "title": p["title"], "draftName": sc["outlineDraft"] }),
    );
    let sp = s(&built["screenplayId"]);
    let draft = s(&built["draftId"]);
    ctx.set("screenplay", sp.clone());
    ctx.set("draftOutline", draft.clone());
    ctx.set("draftCurrent", draft.clone());
    g.ok("screenplay.overview", json!({ "screenplayId": sp }));

    let setup = p["pageSetup"].clone();
    g.ok(
        "screenplay.update_page_setup",
        json!({ "screenplayId": sp, "setup": setup }),
    );
    let mut letter = setup.clone();
    letter["paper"] = json!("Letter");
    let old = g.ok(
        "screenplay.save_page_preset",
        json!({ "name": p["pagePresets"]["drop"], "setup": letter }),
    );
    g.ok(
        "screenplay.save_page_preset",
        json!({ "name": p["pagePresets"]["keep"], "setup": setup }),
    );
    g.ok("screenplay.page_setup", json!({ "screenplayId": sp }));
    if !g.id(&old).is_empty() {
        g.ok("screenplay.delete_page_preset", json!({ "id": g.id(&old) }));
    }
    let tp = p["titlePage"].clone();
    g.ok(
        "screenplay.title_page_preview",
        json!({ "screenplayId": sp, "titlePage": tp, "draftId": draft }),
    );
    g.ok(
        "screenplay.update_title_page",
        json!({ "screenplayId": sp, "titlePage": tp }),
    );

    let mut custom = p["customElement"].clone();
    custom["screenplayId"] = json!(sp);
    let custom_id = g.id(&g.ok("screenplay.create_custom_element", custom));
    ctx.set("customElement", custom_id.clone());
    g.ok(
        "screenplay.update_custom_element",
        json!({ "id": custom_id, "italic": true }),
    );
    let mut dropped = p["customElementDropped"].clone();
    dropped["screenplayId"] = json!(sp);
    let dropped_id = g.id(&g.ok("screenplay.create_custom_element", dropped));
    g.ok(
        "screenplay.delete_custom_element",
        json!({ "id": dropped_id }),
    );
    g.ok("screenplay.custom_elements", json!({ "screenplayId": sp }));
    map_scenes(g, ctx);
}

/// The Final Draft file enters through the real import. Its scenes have no link to the Beat Board
/// (the import cannot make one), so the First Draft is a copy of the outline, which keeps those
/// links, and its scene text is copied element by element from the imported draft.
pub fn import_and_write(g: &Gold, ctx: &mut Ctx) {
    g.phase("Screenplay: Final Draft import");
    let sc = ctx.script.clone();
    let sp = ctx.id("screenplay");
    g.ok("screenplay.samples", json!({}));
    let path = ctx
        .content
        .join(s(&sc["importFile"]))
        .to_string_lossy()
        .into_owned();
    let pv = g.ok(
        "screenplay.import_preview",
        json!({ "path": path, "pastedText": null, "format": "fdx" }),
    );
    ctx.facts.insert(
        "importPreview".into(),
        json!({ "scenes": pv["sceneCount"], "elements": pv["elementCount"], "characters": pv["characterCount"], "pages": pv["approxPages"],
                "confidence": pv["confidence"], "warnings": pv["warnings"] }),
    );
    let imported = g.ok(
        "screenplay.import_apply",
        json!({ "previewId": pv["previewId"], "path": null, "pastedText": null, "format": null, "mode": "new_draft",
                "screenplayId": sp, "episodeId": null, "draftName": sc["importDraft"], "keepSourceFile": true }),
    );
    let import_draft = s(&imported["draftId"]);
    ctx.set("draftImported", import_draft.clone());

    g.phase("Screenplay: First Draft from the imported text");
    let first = g.id(&g.ok(
        "screenplay.new_draft",
        json!({ "sourceDraftId": ctx.id("draftOutline"), "name": sc["firstDraft"],
                "note": "Scene text copied from the Final Draft import. Scenes keep their Beat Board links.", "makeCurrent": true }),
    ));
    ctx.set("draftFirst", first.clone());
    ctx.set("draftCurrent", first.clone());
    map_scenes(g, ctx);
    let source = document(g, &import_draft);
    let target = document(g, &first);
    let by_id: BTreeMap<String, Value> = arr(&target["scenes"])
        .into_iter()
        .map(|x| (s(&x["id"]), x))
        .collect();
    let mut mismatched = Vec::new();
    for (i, src) in arr(&source["scenes"]).iter().enumerate() {
        let n = (i + 1) as u32;
        let Some(cur) = ctx.scenes.get(&n).and_then(|id| by_id.get(id)) else {
            mismatched.push(format!("scene {n}: no matching card"));
            continue;
        };
        if s(&cur["heading"]).trim() != s(&src["heading"]).trim().to_uppercase() {
            mismatched.push(format!(
                "scene {n}: {:?} against {:?}",
                cur["heading"], src["heading"]
            ));
        }
        let scene_id = s(&cur["id"]);
        let mut ops: Vec<Value> = arr(&cur["elements"])
            .iter()
            .map(|e| json!({ "op": "deleteElement", "id": e["id"] }))
            .collect();
        for e in arr(&src["elements"]) {
            ops.push(json!({ "op": "insertElement", "sceneId": scene_id, "elementType": e["elementType"], "text": e["text"] }));
        }
        g.ok(
            "screenplay.apply_edits",
            json!({ "draftId": first, "ops": ops }),
        );
    }
    // The copy is compared with the import, element by element.
    let copied = document(g, &first);
    let sig = |doc: &Value| -> Vec<(String, String)> {
        arr(&doc["scenes"])
            .iter()
            .flat_map(|x| arr(&x["elements"]))
            .map(|e| (s(&e["elementType"]), s(&e["text"])))
            .collect()
    };
    let (a, b) = (sig(&source), sig(&copied));
    ctx.facts.insert(
        "importParity".into(),
        json!({ "importedScenes": arr(&source["scenes"]).len(), "copiedScenes": arr(&copied["scenes"]).len(),
                "importedElements": a.len(), "copiedElements": b.len(), "identical": a == b, "headingProblems": mismatched }),
    );

    let story = ctx.story.clone();
    for act in arr(&story["acts"]) {
        for seq in arr(&act["sequences"]) {
            for card in arr(&seq["scenes"]) {
                let id = ctx.scene(&card["n"]);
                if !id.is_empty() {
                    g.ok(
                        "screenplay.update_scene",
                        json!({ "sceneId": id, "synopsis": card["short"] }),
                    );
                }
            }
        }
    }
    for note in arr(&sc["sceneNotes"]) {
        let id = ctx.scene(&note["scene"]);
        if !id.is_empty() {
            g.ok(
                "screenplay.update_scene",
                json!({ "sceneId": id, "notes": note["note"] }),
            );
        }
    }
}

fn find_element(g: &Gold, draft: &str, scene: &str, starts: &str) -> String {
    arr(&document(g, draft)["scenes"])
        .iter()
        .filter(|x| s(&x["id"]) == scene)
        .flat_map(|x| arr(&x["elements"]))
        .find(|e| s(&e["text"]).starts_with(starts))
        .map(|e| s(&e["id"]))
        .unwrap_or_default()
}

fn find_scene_by_heading(g: &Gold, draft: &str, starts: &str) -> String {
    arr(&document(g, draft)["scenes"])
        .iter()
        .find(|x| s(&x["heading"]).starts_with(starts))
        .map(|x| s(&x["id"]))
        .unwrap_or_default()
}

pub fn drafts_and_review(g: &Gold, ctx: &mut Ctx) {
    g.phase("Screenplay: editing, drafts and review");
    let sp = ctx.id("screenplay");
    let first = ctx.id("draftFirst");
    let sc = ctx.script.clone();
    let p = ctx.project.clone();
    g.ok("writing.proofing", json!({}));
    g.ok(
        "writing.set_proofing",
        json!({ "enabled": true, "mainLanguage": p["proofing"]["mainLanguage"], "secondLanguage": p["proofing"]["secondLanguage"] }),
    );
    for w in arr(&p["dictionary"]) {
        g.ok("writing.dictionary_add", json!({ "word": w }));
    }
    g.ok(
        "writing.dictionary_remove",
        json!({ "word": p["dictionaryRemove"] }),
    );
    g.ok("writing.dictionary", json!({}));
    g.ok(
        "writing.spelling_suggestions",
        json!({ "word": p["spellingProbe"] }),
    );
    let doc = document(g, &first);
    let items: Vec<Value> = arr(&doc["scenes"])
        .iter()
        .take(3)
        .flat_map(|x| arr(&x["elements"]))
        .map(|e| json!({ "key": e["id"], "text": e["text"] }))
        .collect();
    g.ok("writing.check_spelling", json!({ "items": items }));
    g.ok("writing.navigator_columns", json!({ "draftId": first }));
    g.ok("writing.suggestion_sources", json!({}));
    g.ok(
        "screenplay.set_view_state",
        json!({ "state": { "mode": "page", "notesVisible": true, "openDraftId": first } }),
    );
    g.ok("screenplay.view_state", json!({}));

    // One-at-a-time editor operations.
    let one = &sc["oneOff"];
    let scene = ctx.scene(&one["scene"]);
    g.ok(
        "screenplay.insert_element",
        json!({ "sceneId": scene, "index": 0, "elementType": "action", "text": one["insert"] }),
    );
    let new_el = find_element(g, &first, &scene, &s(&one["insert"]));
    g.ok(
        "screenplay.update_element",
        json!({ "id": new_el, "text": one["edit"] }),
    );
    g.ok(
        "screenplay.move_element",
        json!({ "id": new_el, "sceneId": scene, "index": 1 }),
    );
    g.ok(
        "screenplay.insert_element",
        json!({ "sceneId": scene, "elementType": "note", "text": one["note"] }),
    );
    let note_id = find_element(g, &first, &scene, &s(&one["prefixNote"]));
    g.ok("screenplay.delete_element", json!({ "id": note_id }));
    // The trial line goes too, so the First Draft stays equal to the imported text.
    g.ok("screenplay.delete_element", json!({ "id": new_el }));

    let report = g.ok("screenplay.characters", json!({ "draftId": first }));
    for c in arr(&report["characters"])
        .iter()
        .chain(arr(&report["cues"]).iter())
    {
        let cue = s(&c["name"]) + &s(&c["cue"]);
        if let Some(cid) = ctx.chars.get(&cue) {
            g.ok(
                "screenplay.set_character_link",
                json!({ "screenplayId": sp, "cueName": cue, "characterId": cid, "ignored": false }),
            );
        }
    }
    let clear = s(&sc["clearLink"]);
    g.ok(
        "screenplay.clear_character_link",
        json!({ "screenplayId": sp, "cueName": clear }),
    );
    if let Some(cid) = ctx.chars.get(&clear) {
        g.ok(
            "screenplay.set_character_link",
            json!({ "screenplayId": sp, "cueName": clear, "characterId": cid, "ignored": false }),
        );
    }
    g.ok(
        "screenplay.set_character_link",
        json!({ "screenplayId": sp, "cueName": sc["ignoreLink"], "ignored": true }),
    );
    g.ok("screenplay.characters", json!({ "draftId": first }));
    g.ok("screenplay.pagination", json!({ "draftId": first }));
    g.ok(
        "screenplay.page_view",
        json!({ "draftId": first, "titlePage": true }),
    );
    let hub = ctx.scene(&sc["hubScene"]);
    g.ok("screenplay.scene_hub", json!({ "sceneId": hub }));
    g.ok("screenplay.story_reference", json!({ "sceneId": hub }));
    g.ok("screenplay.locate", json!({ "sceneId": hub }));
    g.ok("screenplay.locate", json!({ "draftId": first }));
    g.ok("screenplay.history_points", json!({ "draftId": first }));

    // Second Draft: real corrections of the imported file.
    let second = g.id(&g.ok(
        "screenplay.new_draft",
        json!({ "sourceDraftId": first, "name": sc["secondDraft"], "note": sc["secondNote"], "makeCurrent": true }),
    ));
    ctx.set("draftSecond", second.clone());
    ctx.set("draftCurrent", second.clone());
    map_scenes(g, ctx);
    g.ok(
        "screenplay.rename_draft",
        json!({ "draftId": second, "name": sc["secondDraft"], "note": sc["secondNoteFinal"] }),
    );
    let mut replaced = 0i64;
    for r in arr(&sc["replaceAll"]) {
        let scene_id = if r["scene"].is_null() {
            Value::Null
        } else {
            json!(ctx.scene(&r["scene"]))
        };
        let out = g.ok(
            "screenplay.replace_all",
            json!({ "draftId": second, "query": r["query"], "replacement": r["replacement"], "matchCase": r["matchCase"],
                    "wholeWord": r["wholeWord"], "includeNotes": false, "sceneId": scene_id }),
        );
        replaced += out["count"]
            .as_i64()
            .or(out["replaced"].as_i64())
            .unwrap_or(0);
    }
    ctx.facts
        .insert("secondDraftReplacements".into(), json!(replaced));
    let doc = document(g, &second);
    for f in arr(&sc["cueFix"]) {
        let id = ctx.scene(&f["scene"]);
        let el = arr(&doc["scenes"])
            .iter()
            .filter(|x| s(&x["id"]) == id)
            .flat_map(|x| arr(&x["elements"]))
            .find(|e| {
                s(&e["elementType"]) == s(&f["elementType"]) && s(&e["text"]) == s(&f["text"])
            });
        if let Some(e) = el {
            g.ok(
                "screenplay.update_element",
                json!({ "id": e["id"], "text": f["newText"] }),
            );
        }
    }
    for f in arr(&sc["toCustom"]) {
        let id = ctx.scene(&f["scene"]);
        let el = find_element(g, &second, &id, &s(&f["text"]));
        if !el.is_empty() {
            g.ok(
                "screenplay.apply_edits",
                json!({ "draftId": second, "ops": [
                    { "op": "deleteElement", "id": el },
                    { "op": "insertElement", "sceneId": id, "elementType": "custom", "customElementId": ctx.id("customElement"), "text": f["text"] }
                ] }),
            );
        }
    }
    // Split one scene at an element and merge it back.
    let split_scene = ctx.scene(&sc["splitScene"]);
    let at = sc["splitAt"].as_u64().unwrap_or(3) as usize;
    let doc = document(g, &second);
    if let Some(el) = arr(&doc["scenes"])
        .iter()
        .find(|x| s(&x["id"]) == split_scene)
        .and_then(|x| arr(&x["elements"]).get(at).cloned())
    {
        g.ok("screenplay.split_scene", json!({ "elementId": el["id"] }));
        let doc = document(g, &second);
        let pos = arr(&doc["scenes"])
            .iter()
            .position(|x| s(&x["id"]) == split_scene)
            .unwrap_or(0);
        if let Some(next) = arr(&doc["scenes"]).get(pos + 1) {
            g.ok("screenplay.merge_scene", json!({ "sceneId": next["id"] }));
        }
    }
    // A scene written and cut again.
    let t = &sc["tempScene"];
    let made = g.ok(
        "screenplay.create_scene",
        json!({ "draftId": second, "index": t["index"], "heading": t["heading"] }),
    );
    let mut temp = s(&made["sceneId"]) + &s(&made["createdSceneId"]);
    if temp.is_empty() {
        temp = find_scene_by_heading(g, &second, &s(&t["heading"]));
    }
    g.ok(
        "screenplay.update_scene",
        json!({ "sceneId": temp, "heading": t["rename"] }),
    );
    g.ok("screenplay.delete_scene", json!({ "sceneId": temp }));
    // Two scenes swapped for a read, then put back.
    let a = sc["swapScenes"][0].as_i64().unwrap_or(1);
    let b = sc["swapScenes"][1].as_u64().unwrap_or(1);
    let swap = ctx.scene(&json!(b));
    g.ok(
        "screenplay.move_scene",
        json!({ "sceneId": swap, "index": a - 1 }),
    );
    g.ok(
        "screenplay.move_scene",
        json!({ "sceneId": swap, "index": a }),
    );
    map_scenes(g, ctx);
    let cmp = g.ok(
        "screenplay.compare",
        json!({ "draftA": first, "draftB": second }),
    );
    ctx.facts.insert(
        "compareFirstSecond".into(),
        json!({ "summary": cmp["summary"] }),
    );

    review(g, ctx, &second);

    // A deleted draft comes back from Recently Deleted.
    let import_draft = ctx.id("draftImported");
    let c = g.ok(
        "screenplay.compare",
        json!({ "draftA": import_draft, "draftB": first }),
    );
    ctx.facts.insert(
        "compareImportFirst".into(),
        json!({ "summary": c["summary"] }),
    );
    g.ok(
        "screenplay.delete_draft",
        json!({ "draftId": import_draft }),
    );
    let trash = g.ok("trash.list", json!({ "store": "project" }));
    if let Some(row) = arr(&trash).iter().find(|r| {
        s(&r["objectId"]) == import_draft || r.to_string().contains(&s(&sc["importDraft"]))
    }) {
        g.ok(
            "trash.restore",
            json!({ "store": "project", "id": row["id"] }),
        );
    }
    let restored = g.id(&g.ok("screenplay.restore_draft", json!({ "draftId": first })));
    if !restored.is_empty() {
        g.ok(
            "screenplay.compare",
            json!({ "draftA": first, "draftB": restored }),
        );
        g.ok("screenplay.set_current_draft", json!({ "draftId": second }));
        g.ok("screenplay.delete_draft", json!({ "draftId": restored }));
    }
    g.ok("screenplay.set_current_draft", json!({ "draftId": second }));
    g.ok("screenplay.renumber_scenes", json!({ "draftId": first }));
    g.ok("interchange.sources", json!({}));
    g.ok("interchange.draft_scenes", json!({ "draftId": second }));
}

fn review(g: &Gold, ctx: &mut Ctx, draft: &str) {
    g.phase("Screenplay: review round");
    let r = ctx.script["review"].clone();
    let round = g.id(&g.ok(
        "screenplay.start_review",
        json!({ "draftId": draft, "name": r["name"], "reviewers": r["reviewers"], "deadline": r["deadline"] }),
    ));
    ctx.set("reviewRound", round.clone());
    g.ok(
        "screenplay.update_review",
        json!({ "reviewRoundId": round, "reviewers": r["reviewersFinal"] }),
    );
    let doc = document(g, draft);

    let sc = &r["sceneComment"];
    let c1 = g.id(&g.ok(
        "comment.create",
        json!({ "targetType": "screenplay_scene", "targetId": ctx.scene(&sc["scene"]), "body": sc["body"], "anchor": null, "sceneId": null, "reviewRoundId": round }),
    ));
    g.ok(
        "comment.reply",
        json!({ "parentId": c1, "body": sc["reply"] }),
    );
    g.ok("comment.resolve", json!({ "id": c1 }));

    let ec = &r["elementComment"];
    let scene_id = ctx.scene(&ec["scene"]);
    let (el, len) = arr(&doc["scenes"])
        .iter()
        .find(|x| s(&x["id"]) == scene_id)
        .and_then(|x| {
            arr(&x["elements"])
                .into_iter()
                .filter(|e| s(&e["elementType"]) == s(&ec["elementType"]))
                .max_by_key(|e| s(&e["text"]).len())
        })
        .map(|e| (s(&e["id"]), s(&e["text"]).encode_utf16().count()))
        .unwrap_or_default();
    let c2 = g.id(&g.ok(
        "comment.create",
        json!({ "targetType": "screenplay_element", "targetId": el, "body": ec["body"], "anchor": { "start": 0, "end": len.min(40) }, "sceneId": null, "reviewRoundId": round }),
    ));
    g.ok(
        "comment.update",
        json!({ "id": c2, "body": ec["update"], "status": ec["status"], "expectedRev": null }),
    );
    g.ok(
        "comment.reply",
        json!({ "parentId": c2, "body": ec["reply"] }),
    );
    g.ok("comment.resolve", json!({ "id": c2 }));
    g.ok("comment.reopen", json!({ "id": c2 }));
    g.ok("comment.resolve", json!({ "id": c2 }));

    let oc = &r["openComment"];
    g.ok(
        "comment.create",
        json!({ "targetType": "screenplay_scene", "targetId": ctx.scene(&oc["scene"]), "body": oc["body"], "anchor": null, "sceneId": null, "reviewRoundId": round }),
    );
    let ty = &r["typo"];
    let typo = g.id(&g.ok(
        "comment.create",
        json!({ "targetType": "screenplay_scene", "targetId": ctx.scene(&ty["scene"]), "body": ty["body"], "anchor": null, "sceneId": null, "reviewRoundId": round }),
    ));
    g.ok("comment.delete", json!({ "id": typo }));
    let st = &r["storyComment"];
    let card_n = st["scene"].as_u64().unwrap_or(0) as u32;
    g.ok(
        "comment.create",
        json!({ "targetType": "story_scene_card", "targetId": ctx.cards[&card_n], "body": st["body"], "anchor": null, "sceneId": null, "reviewRoundId": null }),
    );
    g.ok("comment.list", json!({ "targetType": null, "targetId": null, "draftId": draft, "sceneId": null, "reviewRoundId": round, "status": "all" }));
    g.ok("comment.list", json!({ "targetType": null, "targetId": null, "draftId": null, "sceneId": ctx.scene(&r["listScene"]), "reviewRoundId": null, "status": "open" }));
    g.ok(
        "screenplay.complete_review",
        json!({ "reviewRoundId": round }),
    );
    g.ok(
        "screenplay.review_rounds",
        json!({ "screenplayId": ctx.id("screenplay") }),
    );

    let pn = ctx.script["privateNotes"].clone();
    let p = g.id(&g.ok(
        "private_note.create",
        json!({ "targetType": "screenplay_scene", "targetId": ctx.scene(&pn["create"]["scene"]), "body": pn["create"]["body"] }),
    ));
    g.ok(
        "private_note.update",
        json!({ "id": p, "body": pn["update"] }),
    );
    g.ok(
        "private_note.create",
        json!({ "targetType": null, "targetId": null, "body": pn["free"] }),
    );
    let gone = g.id(&g.ok(
        "private_note.create",
        json!({ "targetType": "screenplay_scene", "targetId": ctx.scene(&pn["gone"]["scene"]), "body": pn["gone"]["body"] }),
    ));
    g.ok("private_note.delete", json!({ "id": gone }));
    g.ok(
        "private_note.list",
        json!({ "targetType": null, "targetId": null }),
    );
}

pub fn lock_for_production(g: &Gold, ctx: &mut Ctx) {
    g.phase("Screenplay: shooting draft and Production Source");
    let sc = ctx.script.clone();
    let second = ctx.id("draftSecond");
    g.ok("screenplay.lock_draft", json!({ "draftId": second }));
    g.ok("screenplay.unlock_draft", json!({ "draftId": second }));
    let shooting = g.id(&g.ok(
        "screenplay.new_draft",
        json!({ "sourceDraftId": second, "name": sc["shootingDraft"], "note": sc["lockNote"], "makeCurrent": true }),
    ));
    ctx.set("draftShooting", shooting.clone());
    ctx.set("draftCurrent", shooting.clone());
    g.ok("screenplay.lock_draft", json!({ "draftId": shooting }));
    g.ok("screenplay.lock_summary", json!({ "draftId": shooting }));
    map_scenes(g, ctx);
    g.ok("production.drafts", json!({}));
    g.ok(
        "production.set_source",
        json!({ "draftId": shooting, "reason": sc["lockReason"] }),
    );
    g.ok("production.source", json!({}));
    g.ok("production.overview", json!({}));
}

/// The revision pages of the locked shooting draft.
pub fn blue_revision(g: &Gold, ctx: &mut Ctx) {
    g.phase("Screenplay: revision");
    let shooting = ctx.id("draftShooting");
    let b = ctx.script["blue"].clone();
    let blue = g.id(&g.ok(
        "screenplay.start_revision",
        json!({ "draftId": shooting, "label": b["label"], "color": b["color"], "reason": b["reason"], "date": b["date"] }),
    ));
    ctx.set("draftBlue", blue.clone());
    g.ok(
        "screenplay.update_revision",
        json!({ "draftId": blue, "reason": b["reasonFinal"] }),
    );
    let doc = document(g, &blue);
    let by_lineage: BTreeMap<String, Value> = arr(&doc["scenes"])
        .into_iter()
        .map(|x| (s(&x["lineageId"]), x))
        .collect();
    let lineage = ctx.lineage.clone();
    let scene_in_blue = |n: &Value| -> String {
        let n = n.as_u64().unwrap_or(0) as u32;
        lineage
            .get(&n)
            .and_then(|l| by_lineage.get(l))
            .map(|x| s(&x["id"]))
            .unwrap_or_default()
    };
    let ns = &b["newScene"];
    let pos = arr(&doc["scenes"])
        .iter()
        .position(|x| s(&x["id"]) == scene_in_blue(&ns["afterScene"]))
        .unwrap_or(0);
    let made = g.ok(
        "screenplay.create_scene",
        json!({ "draftId": blue, "index": pos + 1, "heading": ns["heading"] }),
    );
    let mut new_scene = s(&made["sceneId"]);
    if new_scene.is_empty() {
        new_scene = find_scene_by_heading(g, &blue, &s(&ns["heading"]));
    }
    let ops: Vec<Value> = arr(&ns["elements"])
        .iter()
        .map(|e| json!({ "op": "insertElement", "sceneId": new_scene, "elementType": e["elementType"], "text": e["text"] }))
        .collect();
    g.ok(
        "screenplay.apply_edits",
        json!({ "draftId": blue, "ops": ops }),
    );
    let label = g.ok(
        "screenplay.set_scene_number",
        json!({ "sceneId": new_scene, "number": ns["number"] }),
    );
    if label.is_null() {
        g.ok(
            "screenplay.set_scene_number",
            json!({ "sceneId": new_scene, "number": ns["number"], "acknowledgeDependencies": true }),
        );
    }
    g.ok(
        "screenplay.update_scene",
        json!({ "sceneId": new_scene, "synopsis": ns["synopsis"], "storyDay": ns["storyDay"] }),
    );
    ctx.set("newScene", new_scene);
    let hf = &b["headingFix"];
    g.ok(
        "screenplay.update_scene",
        json!({ "sceneId": scene_in_blue(&hf["scene"]), "heading": hf["heading"] }),
    );
    let rf = &b["revisionFix"];
    g.ok(
        "screenplay.replace_all",
        json!({ "draftId": blue, "query": rf["query"], "replacement": rf["replacement"], "matchCase": true, "wholeWord": false,
                "includeNotes": false, "sceneId": scene_in_blue(&rf["scene"]) }),
    );
    // A scene omitted for a day and restored on the director's call.
    let om = scene_in_blue(&b["omitRestore"]);
    g.ok("screenplay.delete_scene", json!({ "sceneId": om }));
    let doc = document(g, &blue);
    ctx.facts.insert(
        "blueOmittedOnce".into(),
        json!(arr(&doc["omittedScenes"]).len()),
    );
    g.ok("screenplay.restore_omitted_scene", json!({ "sceneId": om }));
    g.ok("screenplay.revision_changes", json!({ "draftId": blue }));
    g.ok("screenplay.revision_pages", json!({ "draftId": blue }));
    g.ok(
        "screenplay.page_view",
        json!({ "draftId": blue, "titlePage": true, "revisionMarks": true }),
    );
    g.ok("screenplay.history_points", json!({ "draftId": blue }));
    g.ok("screenplay.lock_draft", json!({ "draftId": blue }));
    ctx.set("draftCurrent", blue.clone());
    let hp = g.ok(
        "screenplay.history_points",
        json!({ "draftId": ctx.id("draftFirst") }),
    );
    if let Some(p) = arr(&hp)
        .iter()
        .max_by_key(|p| p["elementCount"].as_i64().unwrap_or(0))
    {
        let d = g.id(&g.ok(
            "screenplay.restore_history_point",
            json!({ "historyPointId": p["id"] }),
        ));
        if !d.is_empty() {
            g.ok("screenplay.set_current_draft", json!({ "draftId": blue }));
            g.ok("screenplay.delete_draft", json!({ "draftId": d }));
        }
    }
    g.ok("screenplay.set_current_draft", json!({ "draftId": blue }));
}

/// A second, short screenplay in the same project. An empty one made by a stray click is removed.
pub fn teaser(g: &Gold, ctx: &mut Ctx) {
    g.phase("Screenplay: teaser");
    let t = ctx.script["teaser"].clone();
    let made = g.ok(
        "screenplay.create",
        json!({ "title": t["title"], "draftName": t["draft"] }),
    );
    let sp = s(&made["screenplayId"]) + &s(&made["screenplay"]["id"]);
    let draft = s(&made["draftId"]) + &s(&made["draft"]["id"]);
    if !draft.is_empty() {
        let doc = document(g, &draft);
        let first = arr(&doc["scenes"]).first().map(|x| s(&x["id"]));
        let mut ops: Vec<Value> = Vec::new();
        for (i, sc) in arr(&t["scenes"]).iter().enumerate() {
            let id = if i == 0 && first.is_some() {
                let id = first.clone().unwrap_or_default();
                ops.push(json!({ "op": "updateScene", "id": id, "heading": sc["heading"] }));
                id
            } else {
                let id = format!("{:08x}-0000-4000-8000-{:012x}", 0x7ea5e000u32 + i as u32, i);
                ops.push(json!({ "op": "insertScene", "id": id, "heading": sc["heading"] }));
                id
            };
            for e in arr(&sc["elements"]) {
                ops.push(json!({ "op": "insertElement", "sceneId": id, "elementType": e["type"], "text": e["text"] }));
            }
        }
        g.ok(
            "screenplay.apply_edits",
            json!({ "draftId": draft, "ops": ops }),
        );
    }
    g.ok(
        "screenplay.rename",
        json!({ "screenplayId": sp, "title": t["titleFinal"] }),
    );
    let stray = g.ok("screenplay.create", json!({}));
    let stray_id = s(&stray["screenplayId"]) + &s(&stray["screenplay"]["id"]);
    g.ok("screenplay.delete", json!({ "screenplayId": stray_id }));
    g.ok(
        "screenplay.overview",
        json!({ "screenplayId": ctx.id("screenplay") }),
    );
    g.ok(
        "screenplay.set_view_state",
        json!({ "state": { "mode": "page", "notesVisible": false, "screenplayId": ctx.id("screenplay"), "openDraftId": ctx.id("draftBlue") } }),
    );
}
