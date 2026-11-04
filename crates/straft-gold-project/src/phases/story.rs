//! The Beat Board: three acts, seven sequences, 44 scene cards, beats, the Parking Lot,
//! characters with relationships and pictures, card links, attachments and display settings.

use serde_json::{Value, json};

use crate::Ctx;
use crate::gold::{Gold, arr, s};

fn card_ref(id: &str) -> Value {
    json!({ "kind": "card", "id": id })
}

pub fn board(g: &Gold, ctx: &mut Ctx) {
    g.phase("Beat Board");
    g.ok("story.series", json!({}));
    let story = ctx.story.clone();

    // A separate prologue act was the first structure; its tone beat moves into Act One when
    // the act is folded in (below).
    let prologue = g.id(&g.ok(
        "story.create_act",
        json!({ "episodeId": null, "title": "Prologue (1999)", "note": "Folded into Act One in the second pass.", "beforeId": null }),
    ));
    g.ok(
        "story.create_beat",
        json!({ "parent": { "parentType": "act", "parentId": prologue }, "episodeId": null,
                "text": "Tone: the sea before the story. No titles until the static cuts out.", "color": "Teal", "index": null }),
    );

    let mut act_ids = Vec::new();
    let mut seq_ids: Vec<(String, String)> = Vec::new();
    for (ai, act) in arr(&story["acts"]).iter().enumerate() {
        let act_id = g.id(&g.ok(
            "story.create_act",
            json!({ "episodeId": null, "title": act["title"], "note": act["note"], "beforeId": null }),
        ));
        ctx.set(&format!("act{}", ai + 1), act_id.clone());
        act_ids.push(act_id.clone());
        for seq in arr(&act["sequences"]) {
            let seq_id = g.id(&g.ok(
                "story.create_sequence",
                json!({ "actId": act_id, "title": seq["title"], "index": null }),
            ));
            g.ok(
                "story.update_sequence",
                json!({ "id": seq_id, "title": null, "note": seq["note"], "expectedRev": null }),
            );
            seq_ids.push((s(&seq["title"]), seq_id.clone()));
            ctx.set(&format!("seq:{}", s(&seq["title"])), seq_id.clone());
            for sc in arr(&seq["scenes"]) {
                let n = sc["n"].as_u64().unwrap_or(0) as u32;
                // The writer first placed the 1999 battery scene (32) right after the prologue
                // pair; it is moved to its place in Act Two after the first build.
                if n == 32 {
                    continue;
                }
                let card = create_card(g, &seq_id, &sc);
                ctx.cards.insert(n, card);
                if n == 2 {
                    let sc32 = find_scene(&story, 32);
                    let c32 = create_card(g, &seq_id, &sc32);
                    ctx.cards.insert(32, c32);
                }
            }
        }
    }
    // The midpoint card gets its board colour and a production note.
    g.ok(
        "story.update_card",
        json!({ "id": ctx.cards[&24], "shortDescription": null, "longDescription": null, "sceneHeading": null,
                "notes": "Midpoint. Stunt and marine safety plan applies. Protect a full night for it.", "color": "Blue", "expectedRev": null }),
    );
    episode_probe(g, ctx);
    g.ok("story.update_act", json!({ "id": act_ids[0], "title": null, "note": "Setup. Ends when Anjali finds the reel. 12 scenes.", "expectedRev": null }));

    // Fold the prologue act into Act One.
    g.ok(
        "story.delete_act",
        json!({ "id": prologue, "mode": "moveContents", "moveToActId": act_ids[0] }),
    );

    // Beats.
    for b in arr(&story["beats"]) {
        let act = &act_ids[b["act"].as_u64().unwrap_or(0) as usize];
        let id = g.id(&g.ok(
            "story.create_beat",
            json!({ "parent": { "parentType": "act", "parentId": act }, "episodeId": null, "text": b["text"],
                    "color": b["color"], "index": null }),
        ));
        if s(&b["text"]).starts_with("Midpoint") {
            ctx.set("beatMidpoint", id.clone());
            g.ok(
                "story.update_beat",
                json!({ "id": id, "text": null, "note": "Everything turns on Suresh choosing to help.", "color": "Blue", "expectedRev": null }),
            );
        }
    }
    // Parking Lot: the cut stairwell idea.
    for p in arr(&story["parking"]) {
        let id = g.id(&g.ok(
            "story.create_card",
            json!({ "parent": { "parentType": "parking", "parentId": null }, "episodeId": null, "shortDescription": p["short"],
                    "sceneHeading": p["heading"], "longDescription": p["long"], "notes": null, "index": null }),
        ));
        ctx.set("parkedStairwell", id);
    }

    // A structural experiment: the confession sequence before the storm, then back.
    let storm = ctx.id("seq:Storm Warning");
    let joe = ctx.id("seq:What Joe Did");
    g.ok(
        "story.move_items",
        json!({ "items": [{ "kind": "sequence", "id": joe }], "target": { "parentType": "act", "parentId": act_ids[1] },
                "before": { "kind": "sequence", "id": storm }, "episodeId": null }),
    );
    g.ok(
        "story.move_items",
        json!({ "items": [{ "kind": "sequence", "id": joe }], "target": { "parentType": "act", "parentId": act_ids[1] },
                "before": null, "episodeId": null }),
    );
    g.ok(
        "story.move_act",
        json!({ "id": act_ids[2], "beforeId": act_ids[1] }),
    );
    g.ok(
        "story.move_act",
        json!({ "id": act_ids[2], "beforeId": null }),
    );

    // Scene 40 (the roof broadcast) was nearly cut: parked, then restored.
    let c40 = ctx.cards[&40].clone();
    g.ok("story.park_items", json!({ "items": [card_ref(&c40)] }));
    g.ok(
        "story.restore_from_parking",
        json!({ "items": [card_ref(&c40)] }),
    );
    // Splitting the harbour search into two scenes was tried and dropped.
    let dup = g.ok(
        "story.duplicate_items",
        json!({ "items": [card_ref(&ctx.cards[&23])] }),
    );
    let dup_ids: Vec<Value> = arr(&dup["ids"])
        .iter()
        .chain(arr(&dup["created"]).iter())
        .map(|v| {
            let id = if v.is_string() { s(v) } else { g.id(v) };
            card_ref(&id)
        })
        .collect();
    if !dup_ids.is_empty() {
        g.ok("story.delete_items", json!({ "items": dup_ids }));
    }

    characters(g, ctx);
    crate::phases::vault::send_to_story(g, ctx);
    // The parked alternative opening becomes a beat; the parked quote beat becomes a card.
    let opening = ctx.id("parkedOpeningCard");
    if !opening.is_empty() {
        g.ok("story.card_to_beat", json!({ "id": opening }));
    }
    let parking = g.ok("story.board", json!({ "episodeId": null }));
    if let Some(quote_beat) = arr(&parking["parking"]).iter().find(|i| {
        s(&i["kind"]) == "beat"
            && (i.to_string().contains("came home on a voice") || i.to_string().contains("Pappan"))
    }) {
        g.ok("story.convert_beat", json!({ "id": quote_beat["id"] }));
    }
    // The coda sequence from the vault was cut.
    let coda = ctx.id("codaSequence");
    if !coda.is_empty() {
        g.ok(
            "story.delete_sequence",
            json!({ "id": coda, "mode": "deleteAll", "moveTo": null }),
        );
    }

    // Attachments.
    g.ok(
        "story.add_attachment",
        json!({ "ownerType": "scene_card", "ownerId": ctx.cards[&28], "path": ctx.media.path("research-coast-radio-1999.pdf") }),
    );
    g.ok(
        "story.add_attachment",
        json!({ "ownerType": "sequence", "ownerId": ctx.id("seq:Oxide"), "path": ctx.media.path("operations-room-1999.png") }),
    );
    let tmp = g.ok(
        "story.add_attachment",
        json!({ "ownerType": "beat", "ownerId": ctx.id("beatMidpoint"), "path": ctx.media.path("sea-storm-turn.png") }),
    );
    g.ok(
        "story.add_attachment",
        json!({ "ownerType": "beat", "ownerId": ctx.id("beatMidpoint"), "path": ctx.media.path("board-breakwater-02.png") }),
    );
    // The storm frame was the wrong reference for the midpoint; the thumbnail stays.
    if !g.id(&tmp).is_empty() {
        g.ok("story.remove_attachment", json!({ "id": g.id(&tmp) }));
    }

    // Display settings and view state.
    g.ok("story.card_display", json!({}));
    g.ok(
        "story.set_card_display",
        json!({ "preset": "story", "toggles": null }),
    );
    g.ok(
        "story.set_card_display",
        json!({ "preset": "custom", "toggles": { "sceneHeading": true, "shortDescription": true, "longDescriptionPreview": false,
                "notes": true, "attachments": true, "characterCount": true, "location": true, "dayNight": true, "comments": true,
                "revisionChange": true, "breakdownStatus": true, "shotStatus": true } }),
    );
    g.ok(
        "story.set_collapsed",
        json!({ "id": act_ids[0], "collapsed": true }),
    );
    g.ok(
        "story.set_collapsed",
        json!({ "id": act_ids[0], "collapsed": false }),
    );
    g.ok(
        "story.set_collapsed",
        json!({ "id": ctx.id("seq:Static"), "collapsed": true }),
    );
    g.ok("story.view_state", json!({ "episodeId": null }));
    g.ok("story.card", json!({ "id": ctx.cards[&1] }));
    let b = g.ok("story.board", json!({ "episodeId": null }));
    ctx.facts.insert("boardCounts".into(), json!({ "cards": b["cardCount"], "beats": b["beatCount"], "sequences": b["sequenceCount"] }));
}

/// Product check: seasons and episodes belong to episodic projects, yet a Feature Film accepts
/// them (reported in COVERAGE.md). The probe runs the whole episode lifecycle and removes
/// everything again, so the film's board is unchanged; the rows are purged from Recently Deleted.
fn episode_probe(g: &Gold, ctx: &mut Ctx) {
    let season = g.id(&g.ok("story.create_season", json!({ "title": "Probe season" })));
    let series = g.ok("story.series", json!({}));
    ctx.facts.insert(
        "featureFilmAcceptsSeasons".into(),
        json!({ "created": !season.is_empty(), "isEpisodic": series["isEpisodic"] }),
    );
    if season.is_empty() {
        return;
    }
    g.ok("story.update_season", json!({ "id": season, "title": "Probe season (to remove)", "note": "Feature films should not offer seasons." }));
    let other = g.id(&g.ok("story.create_season", json!({ "title": "Probe season 2" })));
    g.ok(
        "story.move_season",
        json!({ "id": other, "beforeId": season }),
    );
    let ep = g.id(&g.ok(
        "story.create_episode",
        json!({ "seasonId": season, "title": "Probe episode", "summary": null, "status": null }),
    ));
    g.ok("story.update_episode", json!({ "id": ep, "title": null, "summary": "Created only to check the project type rule.", "status": null, "expectedRev": null }));
    let copy = g.id(&g.ok(
        "story.duplicate_episode",
        json!({ "id": ep, "copyStory": false }),
    ));
    g.ok(
        "story.move_episode",
        json!({ "id": copy, "seasonId": other, "beforeId": null }),
    );
    g.ok("story.set_current_episode", json!({ "episodeId": ep }));
    g.ok("story.set_current_episode", json!({ "episodeId": null }));
    for id in [&copy, &ep] {
        g.ok("story.delete_episode", json!({ "id": id }));
    }
    for id in [&other, &season] {
        g.ok("story.delete_season", json!({ "id": id }));
    }
    let trash = g.ok("trash.list", json!({ "store": "project" }));
    for row in arr(&trash) {
        if row.to_string().contains("Probe") {
            g.ok(
                "trash.purge",
                json!({ "store": "project", "id": row["id"] }),
            );
        }
    }
}

fn find_scene(story: &Value, n: u32) -> Value {
    for act in arr(&story["acts"]) {
        for seq in arr(&act["sequences"]) {
            for sc in arr(&seq["scenes"]) {
                if sc["n"].as_u64() == Some(n as u64) {
                    return sc;
                }
            }
        }
    }
    Value::Null
}

fn create_card(g: &Gold, seq_id: &str, sc: &Value) -> String {
    let notes = format!(
        "Story day: {}. Characters: {}.",
        s(&sc["storyDay"]),
        arr(&sc["characters"])
            .iter()
            .map(s)
            .collect::<Vec<_>>()
            .join(", ")
    );
    g.id(&g.ok(
        "story.create_card",
        json!({ "parent": { "parentType": "sequence", "parentId": seq_id }, "episodeId": null,
                "shortDescription": sc["short"], "sceneHeading": sc["heading"], "longDescription": sc["long"],
                "notes": notes, "index": null }),
    ))
}

fn characters(g: &Gold, ctx: &mut Ctx) {
    g.phase("Story characters");
    let pics = [
        ("ANJALI", "cast-anjali.png"),
        ("APPACHAN", "cast-appachan.png"),
        ("MARIAMMA", "cast-mariamma.png"),
        ("FATHIMA", "cast-fathima.png"),
        ("SURESH", "cast-suresh.png"),
        ("JOE", "cast-joe.png"),
        ("RAHUL", "cast-rahul.png"),
    ];
    for c in arr(&ctx.story["characters"]) {
        let key = s(&c["key"]);
        let image = pics
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, f)| ctx.media.path(f));
        let id = g.id(&g.ok(
            "story.create_character",
            json!({ "name": c["name"], "roleLabel": c["roleLabel"], "description": c["description"], "notes": c["notes"],
                    "episodeId": null, "imagePath": image }),
        ));
        ctx.chars.insert(key, id);
    }
    // Latha's picture arrived after casting; a wrong file was set first and cleared.
    let latha = ctx.chars["LATHA"].clone();
    g.ok(
        "story.set_character_image",
        json!({ "id": latha, "path": ctx.media.path("cast-fathima.png") }),
    );
    g.ok("story.clear_character_image", json!({ "id": latha }));
    g.ok(
        "story.set_character_image",
        json!({ "id": latha, "path": ctx.media.path("cast-latha.png") }),
    );
    g.ok(
        "story.update_character",
        json!({ "id": ctx.chars["SURESH"], "name": null, "roleLabel": "Antagonist, then ally", "description": null, "notes": null, "expectedRev": null }),
    );
    // The baker had two lines in the first draft; the role was cut and the character archived.
    let baker = g.id(&g.ok(
        "story.create_character",
        json!({ "name": "BAKER", "roleLabel": "Cut role", "description": "Lends the convection oven in scene 14. Lines given to Fathima in the second draft.",
                "notes": null, "episodeId": null, "imagePath": null }),
    ));
    g.ok(
        "story.set_character_archived",
        json!({ "id": baker, "archived": true }),
    );
    // The harbour master was merged into the Port Officer.
    let hm = g.id(&g.ok(
        "story.create_character",
        json!({ "name": "HARBOUR MASTER", "roleLabel": null, "description": "Merged into PORT OFFICER.", "notes": null, "episodeId": null, "imagePath": null }),
    ));
    g.ok("story.delete_character", json!({ "id": hm }));

    for r in arr(&ctx.story["relationships"]) {
        let (Some(from), Some(to)) = (ctx.chars.get(&s(&r["from"])), ctx.chars.get(&s(&r["to"])))
        else {
            continue;
        };
        let id = g.id(&g.ok(
            "story.create_relationship",
            json!({ "fromCharacterId": from, "toCharacterId": to, "relationshipType": r["type"], "note": r["note"] }),
        ));
        if s(&r["type"]) == "Antagonists" {
            g.ok(
                "story.update_relationship",
                json!({ "id": id, "relationshipType": "Antagonists, then allies", "note": null }),
            );
        }
    }
    let wrong = g.id(&g.ok(
        "story.create_relationship",
        json!({ "fromCharacterId": ctx.chars["LATHA"], "toCharacterId": ctx.chars["SURESH"], "relationshipType": "Cousins",
                "note": "Considered in the first draft." }),
    ));
    g.ok("story.delete_relationship", json!({ "id": wrong }));

    // Card links: every card knows its characters.
    let story = ctx.story.clone();
    for act in arr(&story["acts"]) {
        for seq in arr(&act["sequences"]) {
            for sc in arr(&seq["scenes"]) {
                let n = sc["n"].as_u64().unwrap_or(0) as u32;
                let Some(card) = ctx.cards.get(&n) else {
                    continue;
                };
                for ch in arr(&sc["characters"]) {
                    if let Some(cid) = ctx.chars.get(&s(&ch)) {
                        g.ok(
                            "story.link_character_card",
                            json!({ "characterId": cid, "cardId": card }),
                        );
                    }
                }
            }
        }
    }
    // Joe was linked to scene 18 by mistake (he does not go to the mass).
    g.ok(
        "story.link_character_card",
        json!({ "characterId": ctx.chars["JOE"], "cardId": ctx.cards[&18] }),
    );
    g.ok(
        "story.unlink_character_card",
        json!({ "characterId": ctx.chars["JOE"], "cardId": ctx.cards[&18] }),
    );
    g.ok(
        "story.characters",
        json!({ "episodeId": null, "includeArchived": true }),
    );
    g.ok(
        "story.relationships",
        json!({ "episodeId": null, "includeArchived": false }),
    );
    g.ok(
        "story.character",
        json!({ "id": ctx.chars["ANJALI"], "episodeId": null }),
    );
    g.ok(
        "story.character_usage",
        json!({ "id": ctx.chars["APPACHAN"] }),
    );
}

/// After the first build: put scene 32 in its place, apply the order to the script, then the
/// Story Days and time notes the timeline shows.
pub fn after_script(g: &Gold, ctx: &mut Ctx) {
    g.phase("Story after the first build");
    let c32 = ctx.cards[&32].clone();
    let c33 = ctx.cards[&33].clone();
    g.ok(
        "story.move_items",
        json!({ "items": [card_ref(&c32)], "target": { "parentType": "sequence", "parentId": ctx.id("seq:What Joe Did") },
                "before": card_ref(&c33), "episodeId": null }),
    );
    g.ok(
        "story.order_preview",
        json!({ "episodeId": null, "draftId": null, "confirmed": false }),
    );
    g.refused(
        "story.apply_order",
        json!({ "episodeId": null, "draftId": null, "confirmed": false }),
    );
    g.ok(
        "story.apply_order",
        json!({ "episodeId": null, "draftId": null, "confirmed": true }),
    );
    crate::phases::screenplay::map_scenes(g, ctx);

    // Story Days and time notes.
    let story = ctx.story.clone();
    let mut by_day: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    for act in arr(&story["acts"]) {
        for seq in arr(&act["sequences"]) {
            for sc in arr(&seq["scenes"]) {
                let n = sc["n"].as_u64().unwrap_or(0) as u32;
                if let Some(id) = ctx.scenes.get(&n) {
                    by_day
                        .entry(s(&sc["storyDay"]))
                        .or_default()
                        .push(id.clone());
                }
            }
        }
    }
    for (day, ids) in &by_day {
        g.ok(
            "story.assign_story_day",
            json!({ "sceneIds": ids, "storyDay": day }),
        );
    }
    for (n, note) in [
        (1u32, "23:10, the storm peaks"),
        (2, "23:40, power cut"),
        (32, "23:45 to 00:30"),
        (34, "about 00:20"),
        (5, "06:10"),
        (43, "05:30"),
    ] {
        if let Some(id) = ctx.scenes.get(&n) {
            g.ok(
                "story.set_time_note",
                json!({ "sceneId": id, "timeNote": note }),
            );
        }
    }
    ctx.facts.insert(
        "timeline".into(),
        g.ok(
            "story.timeline",
            json!({ "episodeId": null, "screenplayId": null }),
        ),
    );
    g.ok("story.build_preview", json!({ "episodeId": null }));
}
