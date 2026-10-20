//! The screenplay: built from the Beat Board, written scene by scene from
//! `content/screenplay.fountain` through the editor's own edit operations, then drafts, a
//! review round, the shooting draft lock and the blue revision.

use std::collections::BTreeMap;

use openframe_import_export::model::ElementKind;
use serde_json::{Value, json};

use crate::Ctx;
use crate::gold::{Gold, arr, s};

const SHOT_PREFIXES: &[&str] = &[
    "CLOSE ON",
    "ANGLE ON",
    "INSERT",
    "WIDE ON",
    "POV",
    "UNDERWATER",
    "AERIAL",
    "BACK TO SCENE",
];

pub fn document(g: &Gold, draft: &str) -> Value {
    g.ok("screenplay.document", json!({ "draftId": draft }))
}

/// Story scene number -> scene id in the production draft, found by scene lineage (stable across
/// drafts) or, the first time, by the Story card the scene was built from.
pub fn map_scenes(g: &Gold, ctx: &mut Ctx) {
    let draft = ctx.id("draftCurrent");
    let doc = document(g, &draft);
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

/// Scene label ("23A") -> scene id for any draft.
fn scenes_by_lineage(g: &Gold, draft: &str) -> BTreeMap<String, Value> {
    let doc = document(g, draft);
    arr(&doc["scenes"])
        .into_iter()
        .map(|sc| (s(&sc["lineageId"]), sc))
        .collect()
}

pub fn build_and_write(g: &Gold, ctx: &mut Ctx) {
    g.phase("Screenplay: build from the Beat Board");
    let preview = g.ok("story.build_preview", json!({ "episodeId": null }));
    let include: Vec<Value> = arr(&preview["rows"])
        .iter()
        .map(|r| json!({ "cardId": r["cardId"], "heading": null }))
        .collect();
    let built = g.ok(
        "story.build_screenplay",
        json!({ "episodeId": null, "include": include, "descriptionMode": "planningNote", "destination": "newScreenplay",
                "screenplayId": null, "title": "Low Tide Radio", "draftName": "First Draft" }),
    );
    let sp = s(&built["screenplayId"]);
    let draft = s(&built["draftId"]);
    ctx.set("screenplay", sp.clone());
    ctx.set("draftFirst", draft.clone());
    ctx.set("draftCurrent", draft.clone());
    g.ok("screenplay.overview", json!({ "screenplayId": sp }));

    // Page setup, a saved preset, the title page and the custom element for logbook inserts.
    let setup = json!({ "paper": "A4", "font": "Courier", "fontSize": 12, "lineSpacing": "single", "firstPageNumber": 1,
        "header": "{title} / {draft} / {revision}", "footer": "Kayal Pictures. Confidential.", "headerOnFirstPage": false,
        "includeTitlePage": true, "sceneNumbers": true, "revisionMarks": true });
    g.ok(
        "screenplay.update_page_setup",
        json!({ "screenplayId": sp, "setup": setup }),
    );
    let mut letter = setup.clone();
    letter["paper"] = json!("Letter");
    let old = g.ok(
        "screenplay.save_page_preset",
        json!({ "name": "US Letter for the sales agent", "setup": letter }),
    );
    g.ok(
        "screenplay.save_page_preset",
        json!({ "name": "Kayal A4 production", "setup": setup }),
    );
    g.ok("screenplay.page_setup", json!({ "screenplayId": sp }));
    if !g.id(&old).is_empty() {
        g.ok("screenplay.delete_page_preset", json!({ "id": g.id(&old) }));
    }
    let tp = ctx.story["titlePage"].clone();
    g.ok(
        "screenplay.title_page_preview",
        json!({ "screenplayId": sp, "titlePage": tp, "draftId": draft }),
    );
    g.ok(
        "screenplay.update_title_page",
        json!({ "screenplayId": sp, "titlePage": tp }),
    );

    let log = g.ok(
        "screenplay.create_custom_element",
        json!({ "screenplayId": sp, "name": "Logbook Entry", "shortcut": "Ctrl+Alt+1", "look": "action", "uppercase": true,
                "italic": false, "spaceBefore": 1, "nextElement": "action", "exportName": "LOGBOOK" }),
    );
    let log_id = g.id(&log);
    ctx.set("customLogbook", log_id.clone());
    g.ok(
        "screenplay.update_custom_element",
        json!({ "id": log_id, "italic": true }),
    );
    // A "Radio" element was tried; cues with (ON RADIO) read better, so it was removed.
    let radio = g.ok(
        "screenplay.create_custom_element",
        json!({ "screenplayId": sp, "name": "Radio Transmission", "look": "dialogue", "uppercase": false, "italic": true,
                "spaceBefore": 0, "nextElement": "dialogue", "exportName": "RADIO" }),
    );
    g.ok(
        "screenplay.delete_custom_element",
        json!({ "id": g.id(&radio) }),
    );
    g.ok("screenplay.custom_elements", json!({ "screenplayId": sp }));

    map_scenes(g, ctx);
    write_text(g, ctx, &draft, &log_id);
}

/// Replace the build's planning notes with the full scene text from the Fountain file.
fn write_text(g: &Gold, ctx: &mut Ctx, draft: &str, log_id: &str) {
    g.phase("Screenplay: write the scenes");
    let src = std::fs::read_to_string(ctx.content.join("screenplay.fountain"))
        .expect("screenplay.fountain");
    let parsed = openframe_import_export::fountain::parse(&src).expect("fountain parses");
    let mut preamble: Vec<Value> = Vec::new();
    let mut n = 0u32;
    let doc = document(g, draft);
    let by_id: BTreeMap<String, Value> = arr(&doc["scenes"])
        .into_iter()
        .map(|sc| (s(&sc["id"]), sc))
        .collect();
    let existing: BTreeMap<u32, Value> = ctx
        .scenes
        .iter()
        .filter_map(|(n, id)| by_id.get(id).map(|sc| (*n, sc.clone())))
        .collect();
    let mut words = 0usize;
    for scene in &parsed.doc.scenes {
        if scene.heading.trim().is_empty() {
            for e in &scene.elements {
                if let Some(op) = element_op(e, "", log_id) {
                    preamble.push(op);
                }
            }
            continue;
        }
        n += 1;
        let Some(cur) = existing.get(&n) else {
            eprintln!("  the Fountain file has more scenes than the Beat Board ({n})");
            continue;
        };
        if s(&cur["heading"]).trim() != scene.heading.trim() {
            eprintln!(
                "  scene {n}: Fountain heading {:?} differs from the card {:?}",
                scene.heading, cur["heading"]
            );
        }
        let scene_id = s(&cur["id"]);
        let mut ops: Vec<Value> = arr(&cur["elements"])
            .iter()
            .map(|e| json!({ "op": "deleteElement", "id": e["id"] }))
            .collect();
        if n == 1 {
            for mut op in preamble.drain(..) {
                op["sceneId"] = json!(scene_id);
                ops.push(op);
            }
        }
        let mut synopsis = None;
        for e in &scene.elements {
            if e.kind == ElementKind::Synopsis {
                synopsis = Some(e.text.clone());
                continue;
            }
            words += e.text.split_whitespace().count();
            if let Some(op) = element_op(e, &scene_id, log_id) {
                ops.push(op);
            }
        }
        g.ok(
            "screenplay.apply_edits",
            json!({ "draftId": draft, "ops": ops }),
        );
        if let Some(sy) = synopsis {
            g.ok(
                "screenplay.update_scene",
                json!({ "sceneId": scene_id, "synopsis": sy }),
            );
        }
    }
    ctx.facts.insert("fountainScenes".into(), json!(n));
    ctx.facts.insert("fountainWords".into(), json!(words));
    // Scene synopses from the Beat Board where the Fountain file has none, and scene notes for
    // departments.
    let story = ctx.story.clone();
    for act in arr(&story["acts"]) {
        for seq in arr(&act["sequences"]) {
            for sc in arr(&seq["scenes"]) {
                let n = sc["n"].as_u64().unwrap_or(0) as u32;
                if let Some(id) = ctx.scenes.get(&n) {
                    g.ok(
                        "screenplay.update_scene",
                        json!({ "sceneId": id, "synopsis": sc["short"] }),
                    );
                }
            }
        }
    }
    for (n, note) in [
        (
            16u32,
            "Sound: playback from the real Nagra. Record live, no score.",
        ),
        (
            24u32,
            "Stunt and marine safety plan applies. Harness past the yellow marker.",
        ),
        (
            33u32,
            "Playback cue sheet from Farhan. Varghese's line is pre-recorded.",
        ),
        (41u32, "One take for the mast fall. Three cameras."),
    ] {
        if let Some(id) = ctx.scenes.get(&n) {
            g.ok(
                "screenplay.update_scene",
                json!({ "sceneId": id, "notes": note }),
            );
        }
    }
}

fn element_op(
    e: &openframe_import_export::model::Element,
    scene_id: &str,
    log_id: &str,
) -> Option<Value> {
    let text = e.text.trim().to_string();
    let (ty, custom) = match e.kind {
        ElementKind::Action | ElementKind::Centered => {
            let upper = text.to_uppercase() == text;
            if text.starts_with("LOG ") {
                ("custom", Some(log_id.to_string()))
            } else if upper && SHOT_PREFIXES.iter().any(|p| text.starts_with(p)) {
                ("shot", None)
            } else {
                ("action", None)
            }
        }
        ElementKind::Character => ("character", None),
        ElementKind::Dialogue | ElementKind::Lyric => ("dialogue", None),
        ElementKind::Parenthetical => ("parenthetical", None),
        ElementKind::Transition => ("transition", None),
        ElementKind::Shot => ("shot", None),
        ElementKind::Note => ("note", None),
        ElementKind::PageBreak => ("page_break", None),
        ElementKind::Custom => ("custom", Some(log_id.to_string())),
        ElementKind::Section | ElementKind::Synopsis => return None,
    };
    let text = if ty == "custom" {
        text.trim_start_matches("LOG ").to_string()
    } else {
        text
    };
    let mut op =
        json!({ "op": "insertElement", "sceneId": scene_id, "elementType": ty, "text": text });
    if let Some(c) = custom {
        op["customElementId"] = json!(c);
    }
    Some(op)
}

/// Editor operations one at a time, the second draft, the review round with comments, the
/// writer's own Fountain delivery imported as a draft, and comparisons.
pub fn drafts_and_review(g: &Gold, ctx: &mut Ctx) {
    g.phase("Screenplay: editing, drafts and review");
    let sp = ctx.id("screenplay");
    let first = ctx.id("draftFirst");
    // Writing comfort: proofing languages, the project dictionary, the navigator.
    g.ok("writing.proofing", json!({}));
    g.ok(
        "writing.set_proofing",
        json!({ "enabled": true, "mainLanguage": "en-GB", "secondLanguage": "ml" }),
    );
    for w in ["Kadalkkara", "Appachan", "Mariamma", "Nagra", "Kadalkara"] {
        g.ok("writing.dictionary_add", json!({ "word": w }));
    }
    g.ok("writing.dictionary_remove", json!({ "word": "Kadalkara" }));
    g.ok("writing.dictionary", json!({}));
    g.ok(
        "writing.spelling_suggestions",
        json!({ "word": "Kadalkara" }),
    );
    let doc = document(g, &first);
    let items: Vec<Value> = arr(&doc["scenes"])
        .iter()
        .take(3)
        .flat_map(|sc| arr(&sc["elements"]))
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

    // One-at-a-time editor operations on the first draft (scene 9: the biscuit tin).
    let s9 = ctx.scenes[&9].clone();
    let ins = g.ok(
        "screenplay.insert_element",
        json!({ "sceneId": s9, "index": 0, "elementType": "action", "text": "Rain ticks on the tin roof. Inside, six radios are tuned to six different stations." }),
    );
    let new_el = s(&ins["changedElementIds"][0]);
    let new_el = if new_el.is_empty() {
        find_element(g, &first, &s9, "Rain ticks on the tin roof")
    } else {
        new_el
    };
    g.ok("screenplay.update_element", json!({ "id": new_el, "text": "Rain ticks on the tin roof. Inside, six radios are tuned to six different stations at once." }));
    g.ok(
        "screenplay.move_element",
        json!({ "id": new_el, "sceneId": s9, "index": 1 }),
    );
    let note = g.ok(
        "screenplay.insert_element",
        json!({ "sceneId": s9, "elementType": "note", "text": "Check with props: is the biscuit tin Britannia or a local brand? Clearance." }),
    );
    let note_id = find_element(g, &first, &s9, "Check with props");
    let _ = note;
    g.ok("screenplay.delete_element", json!({ "id": note_id }));

    // Characters: cue report and links to the Story characters.
    let report = g.ok("screenplay.characters", json!({ "draftId": first }));
    for c in arr(&report["characters"])
        .iter()
        .chain(arr(&report["cues"]).iter())
    {
        let cue = s(&c["name"]).to_string() + &s(&c["cue"]);
        if let Some(cid) = ctx.chars.get(&cue) {
            g.ok(
                "screenplay.set_character_link",
                json!({ "screenplayId": sp, "cueName": cue, "characterId": cid, "ignored": false }),
            );
        }
    }
    g.ok(
        "screenplay.clear_character_link",
        json!({ "screenplayId": sp, "cueName": "RAHUL" }),
    );
    if let Some(cid) = ctx.chars.get("RAHUL") {
        g.ok(
            "screenplay.set_character_link",
            json!({ "screenplayId": sp, "cueName": "RAHUL", "characterId": cid, "ignored": false }),
        );
    }
    g.ok(
        "screenplay.set_character_link",
        json!({ "screenplayId": sp, "cueName": "HARBOUR CLERK", "ignored": true }),
    );
    g.ok("screenplay.characters", json!({ "draftId": first }));
    g.ok("screenplay.pagination", json!({ "draftId": first }));
    g.ok(
        "screenplay.page_view",
        json!({ "draftId": first, "titlePage": true }),
    );
    g.ok(
        "screenplay.scene_hub",
        json!({ "sceneId": ctx.scenes[&28] }),
    );
    g.ok(
        "screenplay.story_reference",
        json!({ "sceneId": ctx.scenes[&28] }),
    );
    g.ok("screenplay.locate", json!({ "sceneId": ctx.scenes[&28] }));
    g.ok("screenplay.locate", json!({ "draftId": first }));
    g.ok("screenplay.history_points", json!({ "draftId": first }));

    // Second draft: tightened, with structural experiments that were undone.
    let second = g.ok(
        "screenplay.new_draft",
        json!({ "sourceDraftId": first, "name": "Second Draft", "note": "After the producer's notes of 20 November 2026.", "makeCurrent": true }),
    );
    let second = g.id(&second);
    ctx.set("draftSecond", second.clone());
    ctx.set("draftCurrent", second.clone());
    map_scenes(g, ctx);
    g.ok("screenplay.rename_draft", json!({ "draftId": second, "name": "Second Draft", "note": "Producer's notes of 20 November 2026 addressed. Baker cut." }));
    let s14 = ctx.scenes[&14].clone();
    g.ok(
        "screenplay.replace_all",
        json!({ "draftId": second, "query": "54 degrees", "replacement": "fifty-four degrees", "matchCase": false,
                "wholeWord": false, "includeNotes": false, "sceneId": s14 }),
    );
    g.ok(
        "screenplay.replace_all",
        json!({ "draftId": second, "query": "Mr. Krishnadas", "replacement": "Krishnadas", "matchCase": true,
                "wholeWord": true, "includeNotes": false, "sceneId": null }),
    );
    // Split the breakwater scene at its third element and merge it back.
    let s24 = ctx.scenes[&24].clone();
    let doc = document(g, &second);
    if let Some(el) = arr(&doc["scenes"])
        .iter()
        .find(|sc| s(&sc["id"]) == s24)
        .and_then(|sc| arr(&sc["elements"]).get(3).cloned())
    {
        let split = g.ok("screenplay.split_scene", json!({ "elementId": el["id"] }));
        let _ = split;
        let doc = document(g, &second);
        let pos = arr(&doc["scenes"])
            .iter()
            .position(|sc| s(&sc["id"]) == s24)
            .unwrap_or(0);
        if let Some(next) = arr(&doc["scenes"]).get(pos + 1) {
            g.ok("screenplay.merge_scene", json!({ "sceneId": next["id"] }));
        }
    }
    // A truck interior was written and cut again.
    let truck = g.ok(
        "screenplay.create_scene",
        json!({ "draftId": second, "index": 23, "heading": "INT. SITE TRUCK - MOVING - NIGHT" }),
    );
    let truck_scene = s(&truck["sceneId"]).to_string() + &s(&truck["createdSceneId"]);
    let truck_scene = if truck_scene.is_empty() {
        find_scene_by_heading(g, &second, "INT. SITE TRUCK")
    } else {
        truck_scene
    };
    g.ok(
        "screenplay.update_scene",
        json!({ "sceneId": truck_scene, "heading": "INT. SITE TRUCK - MOVING - NIGHT (RAIN)" }),
    );
    g.ok("screenplay.delete_scene", json!({ "sceneId": truck_scene }));
    // Scenes 14 and 15 swapped for a read, then put back.
    g.ok(
        "screenplay.move_scene",
        json!({ "sceneId": ctx.scenes[&15], "index": 13 }),
    );
    g.ok(
        "screenplay.move_scene",
        json!({ "sceneId": ctx.scenes[&15], "index": 14 }),
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

    // The writer also delivered the script as a Fountain file. It is imported as its own draft
    // and compared with the in-app draft; the import keeps dual dialogue.
    g.ok("screenplay.samples", json!({}));
    g.ok(
        "screenplay.import_preview",
        json!({ "path": null, "pastedText": null, "format": null, "sample": "example-feature" }),
    );
    let path = ctx
        .content
        .join("screenplay.fountain")
        .to_string_lossy()
        .into_owned();
    let pv = g.ok(
        "screenplay.import_preview",
        json!({ "path": path, "pastedText": null, "format": "fountain" }),
    );
    let imported = g.ok(
        "screenplay.import_apply",
        json!({ "previewId": pv["previewId"], "path": null, "pastedText": null, "format": null, "mode": "new_draft",
                "screenplayId": sp, "episodeId": null, "draftName": "Writer's Fountain delivery", "keepSourceFile": true }),
    );
    let imp_draft = s(&imported["draftId"]);
    ctx.set("draftImported", imp_draft.clone());
    if !imp_draft.is_empty() {
        let d = document(g, &imp_draft);
        let dual = arr(&d["scenes"])
            .iter()
            .flat_map(|sc| arr(&sc["elements"]))
            .filter(|e| e["dual"] == json!(true))
            .count();
        ctx.facts.insert("importedDualElements".into(), json!(dual));
        ctx.facts
            .insert("importedScenes".into(), json!(arr(&d["scenes"]).len()));
        let c = g.ok(
            "screenplay.compare",
            json!({ "draftA": imp_draft, "draftB": second }),
        );
        ctx.facts.insert(
            "compareImportedSecond".into(),
            json!({ "summary": c["summary"] }),
        );
        // Deleted from the draft list by mistake and brought back from Recently Deleted.
        g.ok("screenplay.delete_draft", json!({ "draftId": imp_draft }));
        let trash = g.ok("trash.list", json!({ "store": "project" }));
        if let Some(row) = arr(&trash).iter().find(|r| {
            s(&r["objectId"]) == imp_draft
                || format!("{}", r).contains("Writer's Fountain delivery")
        }) {
            g.ok(
                "trash.restore",
                json!({ "store": "project", "id": row["id"] }),
            );
        }
    }
    // The first draft's version restored as a new draft for a side-by-side read.
    let restored = g.id(&g.ok("screenplay.restore_draft", json!({ "draftId": first })));
    if !restored.is_empty() {
        g.ok(
            "screenplay.compare",
            json!({ "draftA": first, "draftB": restored }),
        );
        // A restored draft opens as the current draft; the second draft goes back to current.
        g.ok("screenplay.set_current_draft", json!({ "draftId": second }));
        g.ok("screenplay.delete_draft", json!({ "draftId": restored }));
    }
    g.ok("screenplay.set_current_draft", json!({ "draftId": second }));
    // Renumber the first draft (working copy) after the experiments.
    g.ok("screenplay.renumber_scenes", json!({ "draftId": first }));
    g.ok("interchange.sources", json!({}));
    g.ok("interchange.draft_scenes", json!({ "draftId": second }));
}

fn find_element(g: &Gold, draft: &str, scene: &str, starts: &str) -> String {
    let doc = document(g, draft);
    arr(&doc["scenes"])
        .iter()
        .filter(|sc| s(&sc["id"]) == scene)
        .flat_map(|sc| arr(&sc["elements"]))
        .find(|e| s(&e["text"]).starts_with(starts))
        .map(|e| s(&e["id"]))
        .unwrap_or_default()
}

fn find_scene_by_heading(g: &Gold, draft: &str, starts: &str) -> String {
    let doc = document(g, draft);
    arr(&doc["scenes"])
        .iter()
        .find(|sc| s(&sc["heading"]).starts_with(starts))
        .map(|sc| s(&sc["id"]))
        .unwrap_or_default()
}

fn review(g: &Gold, ctx: &mut Ctx, draft: &str) {
    g.phase("Screenplay: producer's review round");
    let round = g.ok(
        "screenplay.start_review",
        json!({ "draftId": draft, "name": "Producer read, second draft", "reviewers": ["Arun Joseph", "Thomas Mathew"], "deadline": "2026-11-30" }),
    );
    let round_id = g.id(&round);
    ctx.set("reviewRound", round_id.clone());
    g.ok(
        "screenplay.update_review",
        json!({ "reviewRoundId": round_id, "reviewers": ["Arun Joseph", "Thomas Mathew", "Priya Ramesh"] }),
    );
    let doc = document(g, draft);
    let scene = |n: u32| ctx.scenes.get(&n).cloned().unwrap_or_default();
    let el = |n: u32, ty: &str| -> (String, usize) {
        arr(&doc["scenes"])
            .iter()
            .find(|sc| s(&sc["id"]) == scene(n))
            .and_then(|sc| {
                arr(&sc["elements"])
                    .into_iter()
                    .find(|e| s(&e["elementType"]) == ty)
            })
            .map(|e| (s(&e["id"]), s(&e["text"]).encode_utf16().count()))
            .unwrap_or_default()
    };
    let c1 = g.ok(
        "comment.create",
        json!({ "targetType": "screenplay_scene", "targetId": scene(14), "body": "Can we afford a real bakery oven? Props say a convection oven hire is 4,000 rupees a day.",
                "anchor": null, "sceneId": null, "reviewRoundId": round_id }),
    );
    let c1 = g.id(&c1);
    g.ok(
        "comment.reply",
        json!({ "parentId": c1, "body": "Yes. Lakshmi found one in Mattancherry. Keep it." }),
    );
    g.ok("comment.resolve", json!({ "id": c1 }));
    let (d31, len31) = el(31, "dialogue");
    let c2 = g.id(&g.ok(
        "comment.create",
        json!({ "targetType": "screenplay_element", "targetId": d31, "body": "Joe's confession is one speech too long. Can we lose the second half?",
                "anchor": { "start": 0, "end": len31.min(40) }, "sceneId": null, "reviewRoundId": round_id }),
    ));
    g.ok("comment.update", json!({ "id": c2, "body": "Joe's confession runs long. Trim, but keep 'I went home.'", "status": "In Discussion", "expectedRev": null }));
    g.ok(
        "comment.reply",
        json!({ "parentId": c2, "body": "Trimmed in the shooting draft. 'I went home' stays." }),
    );
    g.ok("comment.resolve", json!({ "id": c2 }));
    g.ok("comment.reopen", json!({ "id": c2 }));
    g.ok("comment.resolve", json!({ "id": c2 }));
    let c3 = g.id(&g.ok(
        "comment.create",
        json!({ "targetType": "screenplay_scene", "targetId": scene(39), "body": "Three hundred extras: confirm the budget line before we lock this.",
                "anchor": null, "sceneId": null, "reviewRoundId": round_id }),
    ));
    let _ = c3;
    let typo = g.id(&g.ok(
        "comment.create",
        json!({ "targetType": "screenplay_scene", "targetId": scene(39), "body": "dup", "anchor": null, "sceneId": null, "reviewRoundId": round_id }),
    ));
    g.ok("comment.delete", json!({ "id": typo }));
    // A Beat Board comment and a schedule-free scene note.
    g.ok(
        "comment.create",
        json!({ "targetType": "story_scene_card", "targetId": ctx.cards[&24], "body": "This is the midpoint. Protect the time for it on the schedule.",
                "anchor": null, "sceneId": null, "reviewRoundId": null }),
    );
    g.ok("comment.list", json!({ "targetType": null, "targetId": null, "draftId": draft, "sceneId": null, "reviewRoundId": round_id, "status": "all" }));
    g.ok("comment.list", json!({ "targetType": null, "targetId": null, "draftId": null, "sceneId": scene(39), "reviewRoundId": null, "status": "open" }));
    g.ok(
        "screenplay.complete_review",
        json!({ "reviewRoundId": round_id }),
    );
    g.ok(
        "screenplay.review_rounds",
        json!({ "screenplayId": ctx.id("screenplay") }),
    );

    // Private notes: the director's own, never exported.
    let p = g.id(&g.ok(
        "private_note.create",
        json!({ "targetType": "screenplay_scene", "targetId": scene(38), "body": "Talk to Gopinathan alone before this one. His father had dementia." }),
    ));
    g.ok("private_note.update", json!({ "id": p, "body": "Talk to Gopinathan alone before scene 38. His father had dementia; let him lead." }));
    g.ok("private_note.create", json!({ "targetType": null, "targetId": null, "body": "Ask Arun about the second co-producer before the budget lock." }));
    let gone = g.id(&g.ok("private_note.create", json!({ "targetType": "screenplay_scene", "targetId": scene(3), "body": "Bengaluru plate" })));
    g.ok("private_note.delete", json!({ "id": gone }));
    g.ok(
        "private_note.list",
        json!({ "targetType": null, "targetId": null }),
    );
}

/// The shooting draft: copied from the second draft, locked with frozen scene numbers, and made
/// the Production Source.
pub fn lock_for_production(g: &Gold, ctx: &mut Ctx) {
    g.phase("Screenplay: shooting draft and Production Source");
    let second = ctx.id("draftSecond");
    // Locked by mistake for a read, and unlocked again.
    g.ok("screenplay.lock_draft", json!({ "draftId": second }));
    g.ok("screenplay.unlock_draft", json!({ "draftId": second }));
    let shooting = g.id(&g.ok(
        "screenplay.new_draft",
        json!({ "sourceDraftId": second, "name": "Shooting Draft", "note": "White pages, 2 December 2026.", "makeCurrent": true }),
    ));
    ctx.set("draftShooting", shooting.clone());
    ctx.set("draftCurrent", shooting.clone());
    g.ok("screenplay.lock_draft", json!({ "draftId": shooting }));
    g.ok("screenplay.lock_summary", json!({ "draftId": shooting }));
    map_scenes(g, ctx);
    g.ok("production.drafts", json!({}));
    g.ok(
        "production.set_source",
        json!({ "draftId": shooting, "reason": "Shooting draft locked 2 December 2026." }),
    );
    g.ok("production.source", json!({}));
    g.ok("production.overview", json!({}));
    crate::phases::app::safety_copy(g, ctx);
}

/// Blue pages: a revision of the locked shooting draft with a new scene 23A, trimmed lines in 31
/// and a changed action line in 39; scene 12 was omitted and restored.
pub fn blue_revision(g: &Gold, ctx: &mut Ctx) {
    g.phase("Screenplay: blue revision");
    let shooting = ctx.id("draftShooting");
    let blue = g.id(&g.ok(
        "screenplay.start_revision",
        json!({ "draftId": shooting, "label": "Blue Revision", "color": "Blue", "reason": "Producer notes and the breakwater safety plan.", "date": "2027-01-05" }),
    ));
    ctx.set("draftBlue", blue.clone());
    g.ok("screenplay.update_revision", json!({ "draftId": blue, "reason": "Producer notes: scene 22 omitted (day 4 too long), new jetty scene 23A, Joe's confession trimmed, the breakwater safety plan." }));
    let by_lineage = scenes_by_lineage(g, &blue);
    let lineage = ctx.lineage.clone();
    let scene_in_blue = |n: u32| -> String {
        lineage
            .get(&n)
            .and_then(|l| by_lineage.get(l))
            .map(|sc| s(&sc["id"]))
            .unwrap_or_default()
    };
    // New scene 23A: the jetty where Mariamma was last seen.
    let pos23 = arr(&document(g, &blue)["scenes"])
        .iter()
        .position(|sc| s(&sc["id"]) == scene_in_blue(23))
        .unwrap_or(22);
    let made = g.ok(
        "screenplay.create_scene",
        json!({ "draftId": blue, "index": pos23 + 1, "heading": "EXT. KADALKKARA JETTY - NIGHT" }),
    );
    let new_scene = {
        let id = s(&made["sceneId"]);
        if id.is_empty() {
            find_scene_by_heading(g, &blue, "EXT. KADALKKARA JETTY")
        } else {
            id
        }
    };
    g.ok(
        "screenplay.apply_edits",
        json!({ "draftId": blue, "ops": [
            { "op": "insertElement", "sceneId": new_scene, "elementType": "action", "text": "The search reaches the jetty. Anjali stops at the last plank, where the police file says her mother was last seen. Rain on the water. She shines her torch down into the dark and does not move." },
            { "op": "insertElement", "sceneId": new_scene, "elementType": "character", "text": "FATHIMA" },
            { "op": "insertElement", "sceneId": new_scene, "elementType": "parenthetical", "text": "(gently)" },
            { "op": "insertElement", "sceneId": new_scene, "elementType": "dialogue", "text": "He is not here. Come. Suresh has the lights on the breakwater." },
            { "op": "insertElement", "sceneId": new_scene, "elementType": "action", "text": "Anjali turns away from the water and runs." }
        ] }),
    );
    let label = g.ok(
        "screenplay.set_scene_number",
        json!({ "sceneId": new_scene, "number": "23A" }),
    );
    if label.is_null() {
        g.ok(
            "screenplay.set_scene_number",
            json!({ "sceneId": new_scene, "number": "23A", "acknowledgeDependencies": true }),
        );
    }
    g.ok("screenplay.update_scene", json!({ "sceneId": new_scene, "synopsis": "Anjali at the jetty where her mother was last seen.", "storyDay": "Day 6" }));
    ctx.set("scene23A", new_scene.clone());
    // Trim Joe's confession (scene 31) and adjust the beach action (scene 39).
    let doc = document(g, &blue);
    let s31 = scene_in_blue(31);
    if let Some(e) = arr(&doc["scenes"])
        .iter()
        .find(|sc| s(&sc["id"]) == s31)
        .and_then(|sc| {
            arr(&sc["elements"])
                .into_iter()
                .filter(|e| s(&e["elementType"]) == "dialogue")
                .max_by_key(|e| s(&e["text"]).len())
        })
    {
        let text = s(&e["text"]);
        let cut: String = text.split(". ").take(3).collect::<Vec<_>>().join(". ");
        let cut = if cut.ends_with('.') {
            cut
        } else {
            format!("{cut}.")
        };
        g.ok(
            "screenplay.update_element",
            json!({ "id": e["id"], "text": cut }),
        );
    }
    let s39 = scene_in_blue(39);
    g.ok(
        "screenplay.insert_element",
        json!({ "sceneId": s39, "index": 0, "elementType": "action", "text": "Rain has stopped. The generator truck hums behind the dunes. Three hundred people sit on the wet sand facing two speaker stacks." }),
    );
    // Scene 22 omitted to shorten day 4: the empty room moves to the top of scene 23.
    let s22 = scene_in_blue(22);
    g.ok("screenplay.delete_scene", json!({ "sceneId": s22 }));
    let s23 = scene_in_blue(23);
    g.ok(
        "screenplay.insert_element",
        json!({ "sceneId": s23, "index": 0, "elementType": "action", "text": "Anjali runs out of the lane with a torch, shouting for Appachan. Behind her the front door of the house bangs open and shut. The cot inside is empty; Kannan's lead lies on the floor." }),
    );
    // Scene 12 omitted for a day, then restored on the director's call.
    let s12 = scene_in_blue(12);
    g.ok("screenplay.delete_scene", json!({ "sceneId": s12 }));
    let doc = document(g, &blue);
    ctx.facts.insert(
        "blueOmitted".into(),
        json!(arr(&doc["omittedScenes"]).len()),
    );
    g.ok(
        "screenplay.restore_omitted_scene",
        json!({ "sceneId": s12 }),
    );
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
    // Restoring an earlier history point of the first draft (the writer's working copy).
    let points = arr(&hp);
    // The fullest automatic point of the first draft, opened as a draft for a read, then removed.
    if let Some(p) = points
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

/// The teaser trailer script: a second, short screenplay in the same project, cut from the
/// film's own lines. An empty screenplay made by a stray click is removed.
pub fn teaser(g: &Gold, ctx: &mut Ctx) {
    g.phase("Screenplay: teaser trailer script");
    let made = g.ok(
        "screenplay.create",
        json!({ "title": "Low Tide Radio teaser", "draftName": "Teaser v1" }),
    );
    let sp = s(&made["screenplayId"]).to_string() + &s(&made["screenplay"]["id"]);
    let draft = s(&made["draftId"]).to_string() + &s(&made["draft"]["id"]);
    ctx.set("teaserScreenplay", sp.clone());
    if !draft.is_empty() {
        let doc = document(g, &draft);
        let mut ops: Vec<Value> = Vec::new();
        // A new screenplay may start with an empty scene; it becomes the first teaser scene.
        let first = arr(&doc["scenes"]).first().map(|sc| s(&sc["id"]));
        let scenes = [
            (
                "EXT. OPEN SEA - NIGHT (1999)",
                vec![
                    ("action", "Black water. A lantern swings on a mast."),
                    ("character", "MARIAMMA (V.O.)"),
                    (
                        "dialogue",
                        "Little Flower, Little Flower, this is Kadalkkara.",
                    ),
                ],
            ),
            (
                "INT. FISHERIES SCHOOL - RADIO CLUB ROOM - NIGHT",
                vec![
                    (
                        "action",
                        "A reel turns on an old Nagra. Six kids hold their breath.",
                    ),
                    ("character", "ANJALI"),
                    ("dialogue", "One play. Maybe."),
                    ("transition", "SMASH CUT TO:"),
                ],
            ),
            (
                "EXT. KADALKKARA BEACH - NIGHT",
                vec![
                    (
                        "action",
                        "Three hundred people on the sand. Lanterns. A voice from the speakers.",
                    ),
                    ("centered", "LOW TIDE RADIO"),
                ],
            ),
        ];
        for (i, (heading, els)) in scenes.iter().enumerate() {
            let scene_id = if i == 0 && first.is_some() {
                let id = first.clone().unwrap();
                ops.push(json!({ "op": "updateScene", "id": id, "heading": heading }));
                id
            } else {
                let id = format!("{:08x}-0000-4000-8000-{:012x}", 0x7ea5e000u32 + i as u32, i);
                ops.push(json!({ "op": "insertScene", "id": id, "heading": heading }));
                id
            };
            for (ty, text) in els {
                let ty = if *ty == "centered" { "action" } else { *ty };
                ops.push(json!({ "op": "insertElement", "sceneId": scene_id, "elementType": ty, "text": text }));
            }
        }
        g.ok(
            "screenplay.apply_edits",
            json!({ "draftId": draft, "ops": ops }),
        );
        ctx.set("teaserDraft", draft.clone());
    }
    g.ok(
        "screenplay.rename",
        json!({ "screenplayId": sp, "title": "Low Tide Radio teaser trailer" }),
    );
    let stray = g.ok("screenplay.create", json!({}));
    let stray_id = s(&stray["screenplayId"]).to_string() + &s(&stray["screenplay"]["id"]);
    g.ok("screenplay.delete", json!({ "screenplayId": stray_id }));
    // Back to the film in the Screenplay workspace.
    g.ok(
        "screenplay.overview",
        json!({ "screenplayId": ctx.id("screenplay") }),
    );
    g.ok("screenplay.set_view_state", json!({ "state": { "mode": "page", "notesVisible": false, "screenplayId": ctx.id("screenplay"), "openDraftId": ctx.id("draftBlue") } }));
}
