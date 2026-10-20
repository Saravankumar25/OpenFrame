//! The Idea Vault: treatment, research, references, a voice memo and a sketch, organised in
//! folders, collections and tags; some ideas are sent to the Beat Board.

use base64::Engine;
use serde_json::{Value, json};

use crate::Ctx;
use crate::gold::{Gold, arr, s};

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

pub fn run(g: &Gold, ctx: &mut Ctx) {
    g.phase("Idea Vault");
    let p = &ctx.story["project"];
    let store = "project";

    let research = g.id(&g.ok("vault.create_folder", json!({ "store": store, "name": "Research", "parentId": null })));
    let storm = g.id(&g.ok("vault.create_folder", json!({ "store": store, "name": "1999 storm", "parentId": research })));
    let look = g.id(&g.ok("vault.create_folder", json!({ "store": store, "name": "Look and sound", "parentId": null })));
    let fragments = g.id(&g.ok("vault.create_folder", json!({ "store": store, "name": "Story fragments", "parentId": null })));
    g.ok("vault.rename_folder", json!({ "store": store, "id": look, "name": "Look, light and sound", "expectedRev": null }));
    let mariamma = g.id(&g.ok("vault.create_collection", json!({ "store": store, "name": "Mariamma's world", "note": "Everything about the 1999 station and her voice." })));
    let design = g.id(&g.ok("vault.create_collection", json!({ "store": store, "name": "Production design", "note": null })));
    let unsorted = g.id(&g.ok("vault.create_collection", json!({ "store": store, "name": "Scouting photos (unsorted)", "note": null })));
    g.ok("vault.rename_collection", json!({ "store": store, "id": design, "name": "Production design references", "note": "Shared with Lakshmi Iyer.", "expectedRev": null }));

    let treatment_body = format!(
        "LOGLINE\n{}\n\nSYNOPSIS\n{}\n\nTONE\nQuiet, specific, salt-stained. The film listens before it looks.",
        s(&p["logline"]),
        s(&p["synopsis"])
    );
    let treatment = g.id(&g.ok(
        "vault.create",
        json!({ "store": store, "itemType": "note", "title": "Treatment: Low Tide Radio", "body": treatment_body,
                "caption": null, "url": null, "sourceText": null, "folderId": fragments, "collectionId": null,
                "tags": ["treatment", "story"] }),
    ));
    ctx.set("vaultTreatment", treatment.clone());
    let night = g.id(&g.ok(
        "vault.create",
        json!({ "store": store, "itemType": "note", "title": "The night of 14 November 1999",
                "body": "Eleven boats out of Kadalkkara. Three lost: the Little Flower, the Seagull and the Morning Star. Eight men. The coast radio station went silent at 23:40 according to the police file. The village blames the operator, Mariamma Thomas, who walked to the jetty at midnight and was never found.",
                "folderId": storm, "collectionId": mariamma, "tags": ["1999", "research"] }),
    ));
    let quote = g.id(&g.ok(
        "vault.create",
        json!({ "store": store, "itemType": "quote", "title": "Pappan on the storm",
                "body": "Every boat that came home that night came home on a voice.",
                "sourceText": "Interview with Pappan Kuttan, Munambam harbour, 12 December 2026", "folderId": storm,
                "tags": ["1999", "interview"] }),
    ));
    let imd = g.id(&g.ok(
        "vault.create",
        json!({ "store": store, "itemType": "url", "title": "IMD cyclone archive",
                "url": "https://rsmcnewdelhi.imd.gov.in/", "caption": "Track and wind data for November 1999 depressions in the Arabian Sea.",
                "folderId": storm, "tags": ["research", "weather"] }),
    ));
    let tape = g.id(&g.ok(
        "vault.create",
        json!({ "store": store, "itemType": "url", "title": "Reel-to-reel tape and sticky-shed syndrome",
                "url": "https://en.wikipedia.org/wiki/Sticky-shed_syndrome", "caption": "Why the reel has to be baked before it can be played.",
                "folderId": research, "collectionId": mariamma, "tags": ["sound", "research"] }),
    ));
    let meenu = g.id(&g.ok(
        "vault.create",
        json!({ "store": store, "itemType": "note", "title": "MEENU",
                "body": "Meenu, 12. The youngest of the radio club. Asks the questions the adults avoid. Shouts over Rahul when the harbour answers.",
                "folderId": fragments, "tags": ["character"] }),
    ));
    let opening = g.id(&g.ok(
        "vault.create",
        json!({ "store": store, "itemType": "note", "title": "Alternative opening",
                "body": "Open on the demolition and tell the film as the week before it. Parked: it gives away the ending.",
                "folderId": fragments, "tags": ["story"] }),
    ));
    let coda = g.id(&g.ok(
        "vault.create",
        json!({ "store": store, "itemType": "note", "title": "Coda: one year later",
                "body": "A year later the radio club broadcasts the anniversary mass. Tried as a closing sequence; cut in the second draft.",
                "folderId": fragments, "tags": ["story"] }),
    ));
    let obsolete = g.id(&g.ok(
        "vault.create",
        json!({ "store": store, "itemType": "note", "title": "Working titles (obsolete)",
                "body": "Salt Signal. The Last Broadcast. Channel Sixteen.", "folderId": fragments }),
    ));
    let duplicate = g.id(&g.ok(
        "vault.create",
        json!({ "store": store, "itemType": "note", "title": "Breakwater idea (duplicate)",
                "body": "Suresh carries Appachan back. Same as the beat on the board.", "folderId": fragments }),
    ));

    // Files: research PDF and references copied into the vault; the palette linked from the
    // art department's shared folder.
    let added = g.ok(
        "vault.add_files",
        json!({ "store": store, "paths": [ctx.media.path("research-coast-radio-1999.pdf"), ctx.media.path("operations-room-1999.png"),
                ctx.media.path("sea-storm-1999.png"), ctx.media.path("station-dusk.png")],
                "mode": "copy", "folderId": look, "collectionId": design, "itemType": null }),
    );
    let files: Vec<String> = arr(&added["added"]).iter().map(|v| s(&v["id"])).collect();
    let shared = g.root.join("art-department-share");
    std::fs::create_dir_all(&shared).ok();
    let palette_src = shared.join("palette-monsoon.png");
    std::fs::copy(ctx.media.path("palette-monsoon.png"), &palette_src).ok();
    let linked = g.ok(
        "vault.add_files",
        json!({ "store": store, "paths": [palette_src.to_string_lossy()], "mode": "link", "folderId": look,
                "collectionId": unsorted, "itemType": null }),
    );
    let palette = arr(&linked["added"]).first().map(|v| s(&v["id"])).unwrap_or_default();
    // The share was reorganised: the palette moved, and the vault item is pointed at it again.
    let palette_new = shared.join("grade").join("palette-monsoon-v2.png");
    std::fs::create_dir_all(palette_new.parent().unwrap()).ok();
    std::fs::rename(&palette_src, &palette_new).ok();
    g.ok("vault.relink", json!({ "store": store, "id": palette, "path": palette_new.to_string_lossy() }));
    ctx.set("vaultPalette", palette.clone());
    if let Some(img) = files.get(1) {
        ctx.set("vaultOpsRoomImage", img.clone());
    }
    if let Some(img) = files.get(2) {
        ctx.set("vaultSeaImage", img.clone());
    }

    // The director's voice memo and a drawn sketch, as the app records them.
    let memo = std::fs::read(ctx.media.dir.join("director-memo-breakwater.m4a")).unwrap_or_default();
    let voice = if memo.is_empty() {
        Value::Null
    } else {
        g.ok(
            "vault.ingest_bytes",
            json!({ "store": store, "itemType": "voice", "fileName": "director-memo-breakwater.m4a", "mediaType": "audio/mp4",
                    "dataBase64": b64(&memo), "durationMs": 14_000, "title": "Director's memo: the breakwater",
                    "caption": "Recorded on the recce, 8 January 2027.", "folderId": look, "collectionId": null }),
        )
    };
    ctx.set("vaultVoice", g.id(&voice));
    let sketch = std::fs::read(ctx.media.dir.join("board-breakwater-01.png")).unwrap_or_default();
    let sk = g.ok(
        "vault.ingest_bytes",
        json!({ "store": store, "itemType": "sketch", "fileName": "breakwater-thumbnail.png", "mediaType": "image/png",
                "dataBase64": b64(&sketch), "title": "Breakwater thumbnail", "caption": "First thumbnail for scene 24.",
                "folderId": look, "collectionId": design }),
    );
    let sketch_id = g.id(&sk);

    // Organise.
    g.ok("vault.add_tags", json!({ "store": store, "ids": [treatment, night, quote], "tags": ["mariamma"] }));
    g.ok("vault.add_tags", json!({ "store": store, "ids": [sketch_id, palette], "tags": ["look", "temp"] }));
    g.ok("vault.remove_tag", json!({ "store": store, "ids": [sketch_id, palette], "tag": "temp" }));
    g.ok("vault.set_pinned", json!({ "store": store, "ids": [treatment, night], "pinned": true }));
    g.ok("vault.set_research_flag", json!({ "store": store, "ids": [imd], "flag": "checked" }));
    if let Some(pdf) = files.first() {
        g.ok("vault.set_research_flag", json!({ "store": store, "ids": [pdf], "flag": "needsExpertCheck" }));
    }
    g.ok("vault.set_research_flag", json!({ "store": store, "ids": [tape, quote], "flag": "unchecked" }));
    g.ok("vault.set_research_flag", json!({ "store": store, "ids": [quote], "flag": null }));
    g.ok("vault.add_to_collection", json!({ "store": store, "ids": [palette], "collectionId": design }));
    g.ok("vault.remove_from_collection", json!({ "store": store, "ids": [palette], "collectionId": unsorted }));
    g.ok("vault.delete_collection", json!({ "store": store, "id": unsorted }));
    g.ok("vault.move_to_folder", json!({ "store": store, "ids": [tape], "folderId": storm }));
    g.ok("vault.move_to_folder", json!({ "store": store, "ids": [tape], "folderId": research }));
    let temp = g.id(&g.ok("vault.create_folder", json!({ "store": store, "name": "Inbox", "parentId": null })));
    g.ok("vault.delete_folder", json!({ "store": store, "id": temp }));
    g.ok(
        "vault.update",
        json!({ "store": store, "id": treatment, "title": "Treatment: Low Tide Radio (v3)", "body": null, "caption": null,
                "url": null, "sourceText": null, "expectedRev": null }),
    );

    // Global Idea Vault: the writer's personal sound notes, copied into this project.
    let global_note = g.id(&g.ok(
        "vault.create",
        json!({ "store": "global", "itemType": "note", "title": "Field recording: VHF channel 16 static",
                "body": "Recorded at Munambam on 9 December 2026, 48 kHz, 11 minutes. Clean hiss with two fishermen's calls at 04:12 and 07:40.",
                "tags": ["sound"] }),
    ));
    g.ok("vault.overview", json!({ "store": "global" }));
    g.ok(
        "vault.copy_to_store",
        json!({ "from": "global", "to": "project", "ids": [global_note], "collectionId": mariamma, "folderId": look }),
    );
    g.ok("vault.list", json!({ "store": "global", "view": null, "search": null }));

    // Views and search, the way the left panel and the search box show them.
    for view in [
        json!({ "kind": "all" }),
        json!({ "kind": "pinned" }),
        json!({ "kind": "recentlyAdded" }),
        json!({ "kind": "recentlyModified" }),
        json!({ "kind": "folder", "folderId": storm }),
        json!({ "kind": "collection", "collectionId": mariamma }),
        json!({ "kind": "tag", "tag": "1999" }),
    ] {
        g.ok("vault.list", json!({ "store": store, "view": view, "search": null }));
    }
    g.ok("vault.list", json!({ "store": store, "view": null, "search": "tape" }));
    g.ok("vault.get", json!({ "store": store, "id": night }));
    g.ok("vault.overview", json!({ "store": store }));
    g.ok("visual.vault_images", json!({ "store": store }));

    // Clean-up: the obsolete titles list is removed for good; the duplicate idea is restored from
    // the vault's Recently Deleted because the producer wanted to keep its wording.
    g.ok("vault.delete", json!({ "store": store, "ids": [obsolete, duplicate] }));
    let trash = g.ok("vault.trash_list", json!({ "store": store }));
    for row in arr(&trash) {
        let title = s(&row["deleted"]["title"]);
        let id = s(&row["deleted"]["id"]);
        if title.contains("obsolete") {
            g.ok("vault.purge", json!({ "store": store, "id": id }));
        } else if title.contains("duplicate") {
            g.ok("vault.restore", json!({ "store": store, "id": id }));
        }
    }
    ctx.facts.insert("vaultTrashRows".into(), trash);

    ctx.set("vaultMeenu", meenu);
    ctx.set("vaultOpening", opening);
    ctx.set("vaultCoda", coda);
    ctx.set("vaultQuote", quote);
    ctx.set("vaultNight", night);
    ctx.set("vaultSketch", sketch_id);
    ctx.set("vaultFolderLook", look);
    ctx.set("vaultCollectionMariamma", mariamma);
}

/// Ideas sent from the vault to the Beat Board (copies; the vault keeps its own items).
pub fn send_to_story(g: &Gold, ctx: &mut Ctx) {
    g.phase("Idea Vault to Story");
    g.ok("vault.story_targets", json!({}));
    let store = "project";
    let meenu = g.ok(
        "vault.send_to_story",
        json!({ "store": store, "id": ctx.id("vaultMeenu"), "target": "character", "parentType": null, "parentId": null }),
    );
    ctx.chars.insert("MEENU".into(), s(&meenu["storyId"]));
    let opening = g.ok(
        "vault.send_to_story",
        json!({ "store": store, "id": ctx.id("vaultOpening"), "target": "sceneCard", "parentType": "parking", "parentId": null }),
    );
    ctx.set("parkedOpeningCard", s(&opening["storyId"]));
    g.ok(
        "vault.send_to_story",
        json!({ "store": store, "id": ctx.id("vaultQuote"), "target": "beat", "parentType": "parking", "parentId": null }),
    );
    let coda = g.ok(
        "vault.send_to_story",
        json!({ "store": store, "id": ctx.id("vaultCoda"), "target": "sequence", "parentType": "act", "parentId": ctx.id("act3") }),
    );
    ctx.set("codaSequence", s(&coda["storyId"]));
}
