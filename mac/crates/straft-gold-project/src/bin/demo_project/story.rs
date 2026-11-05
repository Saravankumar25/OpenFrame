//! Story: acts, sequences, a scene card for every scene, beats, the parking lot, characters and
//! relationships, card links, attachments, Story Days and the Story page.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::Ctx;
use crate::gold::{Gold, arr, s};

fn card_ref(id: &str) -> Value {
    json!({ "kind": "card", "id": id })
}

fn seq_ref(id: &str) -> Value {
    json!({ "kind": "sequence", "id": id })
}

fn create_card(g: &Gold, seq_id: &str, sc: &Value) -> String {
    let notes = format!(
        "{}\nStory day: {}. Characters: {}.",
        s(&sc["note"]),
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

fn find_scene(story: &Value, n: u64) -> Value {
    for act in arr(&story["acts"]) {
        for seq in arr(&act["sequences"]) {
            for sc in arr(&seq["scenes"]) {
                if sc["n"].as_u64() == Some(n) {
                    return sc;
                }
            }
        }
    }
    Value::Null
}

pub fn board(g: &Gold, ctx: &mut Ctx) {
    g.phase("Beat Board");
    g.ok("story.series", json!({}));
    let story = ctx.story.clone();

    // A first structure with an extra opening act, folded into the first act below.
    let prologue = g.id(&g.ok(
        "story.create_act",
        json!({ "episodeId": null, "title": "Opening (first structure)", "note": "Folded into the first act in the second pass.", "beforeId": null }),
    ));
    g.ok(
        "story.create_beat",
        json!({ "parent": { "parentType": "act", "parentId": prologue }, "episodeId": null,
                "text": "Tone first: no titles until the first shock.", "color": "Teal", "index": null }),
    );

    let late = story["misplaced"]["scene"].as_u64().unwrap_or(0);
    let after = story["misplaced"]["afterScene"].as_u64().unwrap_or(0);
    let mut act_ids = Vec::new();
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
            ctx.set(&format!("seq:{}", s(&seq["title"])), seq_id.clone());
            for sc in arr(&seq["scenes"]) {
                let n = sc["n"].as_u64().unwrap_or(0);
                // One card starts in the wrong place and is moved after the first build.
                if n == late {
                    continue;
                }
                let card = create_card(g, &seq_id, &sc);
                ctx.cards.insert(n as u32, card);
                if n == after {
                    let misplaced = find_scene(&story, late);
                    let c = create_card(g, &seq_id, &misplaced);
                    ctx.cards.insert(late as u32, c);
                }
            }
        }
    }
    let mid = &story["midpoint"];
    let mid_n = mid["scene"].as_u64().unwrap_or(0) as u32;
    g.ok(
        "story.update_card",
        json!({ "id": ctx.cards[&mid_n], "shortDescription": null, "longDescription": null, "sceneHeading": null,
                "notes": mid["note"], "color": mid["color"], "expectedRev": null }),
    );
    episode_probe(g, ctx);
    g.ok(
        "story.update_act",
        json!({ "id": act_ids[0], "title": null, "note": arr(&story["acts"])[0]["note"], "expectedRev": null }),
    );
    g.ok(
        "story.delete_act",
        json!({ "id": prologue, "mode": "moveContents", "moveToActId": act_ids[0] }),
    );

    for b in arr(&story["beats"]) {
        let act = &act_ids[b["act"].as_u64().unwrap_or(0) as usize];
        let id = g.id(&g.ok(
            "story.create_beat",
            json!({ "parent": { "parentType": "act", "parentId": act }, "episodeId": null, "text": b["text"],
                    "color": b["color"], "index": null }),
        ));
        if s(&b["text"]).starts_with(&s(&mid["beatPrefix"])) {
            ctx.set("beatMidpoint", id.clone());
            g.ok(
                "story.update_beat",
                json!({ "id": id, "text": null, "note": mid["beatNote"], "color": null, "expectedRev": null }),
            );
        }
    }
    for p in arr(&story["parking"]) {
        g.ok(
            "story.create_card",
            json!({ "parent": { "parentType": "parking", "parentId": null }, "episodeId": null, "shortDescription": p["short"],
                    "sceneHeading": p["heading"], "longDescription": p["long"], "notes": null, "index": null }),
        );
    }

    // A structural experiment, then back.
    let m = &story["moveExperiment"];
    let act_i = m["act"].as_u64().unwrap_or(0) as usize;
    let moving = ctx.id(&format!("seq:{}", s(&m["sequence"])));
    let before = ctx.id(&format!("seq:{}", s(&m["before"])));
    let parent = json!({ "parentType": "act", "parentId": act_ids[act_i] });
    g.ok(
        "story.move_items",
        json!({ "items": [seq_ref(&moving)], "target": parent, "before": seq_ref(&before), "episodeId": null }),
    );
    let order: Vec<String> = arr(&arr(&story["acts"])[act_i]["sequences"])
        .iter()
        .map(|q| s(&q["title"]))
        .collect();
    let pos = order
        .iter()
        .position(|t| *t == s(&m["sequence"]))
        .unwrap_or(0);
    let next = order
        .get(pos + 1)
        .map(|t| seq_ref(&ctx.id(&format!("seq:{t}"))));
    g.ok(
        "story.move_items",
        json!({ "items": [seq_ref(&moving)], "target": parent, "before": next, "episodeId": null }),
    );
    let last = act_ids.last().cloned().unwrap_or_default();
    g.ok(
        "story.move_act",
        json!({ "id": last, "beforeId": act_ids[1] }),
    );
    g.ok("story.move_act", json!({ "id": last, "beforeId": null }));

    let parked = story["parkExperiment"].as_u64().unwrap_or(0) as u32;
    let c = ctx.cards[&parked].clone();
    g.ok("story.park_items", json!({ "items": [card_ref(&c)] }));
    g.ok(
        "story.restore_from_parking",
        json!({ "items": [card_ref(&c)] }),
    );
    let dup_n = story["duplicateExperiment"].as_u64().unwrap_or(0) as u32;
    let dup = g.ok(
        "story.duplicate_items",
        json!({ "items": [card_ref(&ctx.cards[&dup_n])] }),
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
    crate::vault::send_to_story(g, ctx);
    let opening = ctx.id("parkedCard");
    if !opening.is_empty() {
        g.ok("story.card_to_beat", json!({ "id": opening }));
    }
    let parking = g.ok("story.board", json!({ "episodeId": null }));
    let quote_text = s(&ctx.vault["toStory"]["quoteText"]);
    if let Some(quote_beat) = arr(&parking["parking"])
        .iter()
        .find(|i| s(&i["kind"]) == "beat" && i.to_string().contains(&quote_text))
    {
        g.ok("story.convert_beat", json!({ "id": quote_beat["id"] }));
    }
    let cut = ctx.id("cutSequence");
    if !cut.is_empty() {
        g.ok(
            "story.delete_sequence",
            json!({ "id": cut, "mode": "deleteAll", "moveTo": null }),
        );
    }

    for a in arr(&story["attachments"]) {
        let (ty, owner) = if s(&a["ownerType"]) == "sequence" {
            ("sequence", ctx.id(&format!("seq:{}", s(&a["sequence"]))))
        } else {
            let n = a["scene"].as_u64().unwrap_or(0) as u32;
            ("scene_card", ctx.cards.get(&n).cloned().unwrap_or_default())
        };
        g.ok(
            "story.add_attachment",
            json!({ "ownerType": ty, "ownerId": owner, "path": ctx.media_path(&s(&a["file"])) }),
        );
    }
    let ba = &story["beatAttachment"];
    let tmp = g.ok(
        "story.add_attachment",
        json!({ "ownerType": "beat", "ownerId": ctx.id("beatMidpoint"), "path": ctx.media_path(&s(&ba["remove"])) }),
    );
    g.ok(
        "story.add_attachment",
        json!({ "ownerType": "beat", "ownerId": ctx.id("beatMidpoint"), "path": ctx.media_path(&s(&ba["keep"])) }),
    );
    if !g.id(&tmp).is_empty() {
        g.ok("story.remove_attachment", json!({ "id": g.id(&tmp) }));
    }

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
    g.ok("story.view_state", json!({ "episodeId": null }));
    g.ok("story.card", json!({ "id": ctx.cards[&1] }));
    let page = g.ok("story.page.get", json!({}));
    g.ok(
        "story.page.save",
        json!({ "text": ctx.project["storyPage"], "format": null, "expectedRev": page["rev"] }),
    );
    let b = g.ok("story.board", json!({ "episodeId": null }));
    ctx.facts.insert(
        "boardCounts".into(),
        json!({ "cards": b["cardCount"], "beats": b["beatCount"], "sequences": b["sequenceCount"] }),
    );
}

/// Seasons and episodes belong to episodic projects; the lifecycle runs once and is removed again,
/// leaving the film's board unchanged. The rows are purged from Recently Deleted.
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
    g.ok("story.update_season", json!({ "id": season, "title": "Probe season (to remove)", "note": "Removed after the check." }));
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
    for row in arr(&g.ok("trash.list", json!({ "store": "project" }))) {
        if row.to_string().contains("Probe") {
            g.ok(
                "trash.purge",
                json!({ "store": "project", "id": row["id"] }),
            );
        }
    }
}

fn characters(g: &Gold, ctx: &mut Ctx) {
    g.phase("Story characters");
    let story = ctx.story.clone();
    for c in arr(&story["characters"]) {
        let image = c["image"].as_str().map(|f| ctx.media_path(f));
        let id = g.id(&g.ok(
            "story.create_character",
            json!({ "name": c["name"], "roleLabel": c["roleLabel"], "description": c["description"], "notes": c["notes"],
                    "episodeId": null, "imagePath": image }),
        ));
        ctx.chars.insert(s(&c["key"]), id);
    }
    let sw = &story["charImageSwap"];
    let who = ctx.chars[&s(&sw["key"])].clone();
    g.ok(
        "story.set_character_image",
        json!({ "id": who, "path": ctx.media_path(&s(&sw["wrong"])) }),
    );
    g.ok("story.clear_character_image", json!({ "id": who }));
    g.ok(
        "story.set_character_image",
        json!({ "id": who, "path": ctx.media_path(&s(&sw["right"])) }),
    );
    let rn = &story["rename"];
    g.ok(
        "story.update_character",
        json!({ "id": ctx.chars[&s(&rn["key"])], "name": null, "roleLabel": rn["roleLabel"], "description": null, "notes": null, "expectedRev": null }),
    );
    let ar = &story["archivedCharacter"];
    let archived = g.id(&g.ok(
        "story.create_character",
        json!({ "name": ar["name"], "roleLabel": ar["roleLabel"], "description": ar["description"], "notes": null, "episodeId": null, "imagePath": null }),
    ));
    g.ok(
        "story.set_character_archived",
        json!({ "id": archived, "archived": true }),
    );
    let dl = &story["deletedCharacter"];
    let deleted = g.id(&g.ok(
        "story.create_character",
        json!({ "name": dl["name"], "roleLabel": null, "description": dl["description"], "notes": null, "episodeId": null, "imagePath": null }),
    ));
    g.ok("story.delete_character", json!({ "id": deleted }));

    let rename_type = s(&story["relationshipAntagonist"]["type"]);
    for r in arr(&story["relationships"]) {
        let (Some(from), Some(to)) = (ctx.chars.get(&s(&r["from"])), ctx.chars.get(&s(&r["to"])))
        else {
            continue;
        };
        let id = g.id(&g.ok(
            "story.create_relationship",
            json!({ "fromCharacterId": from, "toCharacterId": to, "relationshipType": r["type"], "note": r["note"] }),
        ));
        if s(&r["type"]) == rename_type {
            g.ok(
                "story.update_relationship",
                json!({ "id": id, "relationshipType": story["relationshipAntagonist"]["newType"], "note": null }),
            );
        }
    }
    let wr = &story["wrongRelationship"];
    let wrong = g.id(&g.ok(
        "story.create_relationship",
        json!({ "fromCharacterId": ctx.chars[&s(&wr["from"])], "toCharacterId": ctx.chars[&s(&wr["to"])], "relationshipType": wr["type"], "note": wr["note"] }),
    ));
    g.ok("story.delete_relationship", json!({ "id": wrong }));

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
    let wl = &story["wrongLink"];
    let wrong_scene = wl["scene"].as_u64().unwrap_or(0) as u32;
    g.ok(
        "story.link_character_card",
        json!({ "characterId": ctx.chars[&s(&wl["key"])], "cardId": ctx.cards[&wrong_scene] }),
    );
    g.ok(
        "story.unlink_character_card",
        json!({ "characterId": ctx.chars[&s(&wl["key"])], "cardId": ctx.cards[&wrong_scene] }),
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
        json!({ "id": ctx.chars[&s(&story["detailKey"])], "episodeId": null }),
    );
    g.ok(
        "story.character_usage",
        json!({ "id": ctx.chars[&s(&story["usageKey"])] }),
    );
}

/// After the first build: the misplaced card goes to its place and the order reaches the draft.
pub fn after_build(g: &Gold, ctx: &mut Ctx) {
    g.phase("Story after the first build");
    let m = &ctx.story["misplaced"];
    let late = m["scene"].as_u64().unwrap_or(0) as u32;
    let before = m["before"].as_u64().unwrap_or(0) as u32;
    let target = ctx.id(&format!("seq:{}", s(&m["sequence"])));
    g.ok(
        "story.move_items",
        json!({ "items": [card_ref(&ctx.cards[&late])], "target": { "parentType": "sequence", "parentId": target },
                "before": card_ref(&ctx.cards[&before]), "episodeId": null }),
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
    crate::script::map_scenes(g, ctx);
}

/// Story Days and time notes the timeline shows.
pub fn story_days(g: &Gold, ctx: &mut Ctx) {
    g.phase("Story Days");
    let story = ctx.story.clone();
    let mut by_day: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for act in arr(&story["acts"]) {
        for seq in arr(&act["sequences"]) {
            for sc in arr(&seq["scenes"]) {
                let id = ctx.scene(&sc["n"]);
                if !id.is_empty() {
                    by_day.entry(s(&sc["storyDay"])).or_default().push(id);
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
    for t in arr(&story["timeNotes"]) {
        let id = ctx.scene(&t["scene"]);
        if !id.is_empty() {
            g.ok(
                "story.set_time_note",
                json!({ "sceneId": id, "timeNote": t["note"] }),
            );
        }
    }
    let tl = g.ok(
        "story.timeline",
        json!({ "episodeId": null, "screenplayId": null }),
    );
    ctx.facts
        .insert("timelineGroups".into(), json!(arr(&tl["groups"]).len()));
    g.ok("story.build_preview", json!({ "episodeId": null }));
}
