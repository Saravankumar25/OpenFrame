//! Visual planning: moodboards (images, a pasted frame, vault references, links and notes),
//! storyboards (drawn sketches and imported frames) and shot lists with camera details.

use base64::Engine;
use serde_json::{Value, json};

use crate::Ctx;
use crate::gold::{Gold, arr, s};

fn b64_file(ctx: &Ctx, name: &str) -> String {
    base64::engine::general_purpose::STANDARD
        .encode(std::fs::read(ctx.media.dir.join(name)).unwrap_or_default())
}

pub fn run(g: &Gold, ctx: &mut Ctx) {
    moodboards(g, ctx);
    shots(g, ctx);
    storyboards(g, ctx);
    g.phase("Visual overview");
    let vs = g.ok("visual.scenes", json!({}));
    for n in [24u32, 33, 39] {
        if let Some(l) = ctx.lineage.get(&n) {
            g.ok("visual.mark_scene_reviewed", json!({ "sceneLineageId": l }));
        }
    }
    ctx.facts
        .insert("visualScenes".into(), json!(arr(&vs["scenes"]).len()));
}

fn moodboards(g: &Gold, ctx: &mut Ctx) {
    g.phase("Moodboards");
    let look = g.id(&g.ok(
        "moodboard.create",
        json!({ "name": "Look of the film: monsoon night" }),
    ));
    ctx.set("moodLook", look.clone());
    let imgs = g.ok(
        "moodboard.add_images",
        json!({ "moodboardId": look, "paths": [ctx.media.path("palette-monsoon.png"), ctx.media.path("station-dusk.png"), ctx.media.path("sea-storm-1999.png")], "x": 40, "y": 40 }),
    );
    let items: Vec<String> = arr(&imgs).iter().map(|i| s(&i["id"])).collect();
    let pasted = g.ok(
        "moodboard.add_image_data",
        json!({ "moodboardId": look, "dataBase64": b64_file(ctx, "harbour-dawn.png"), "fileName": "harbour-dawn-grab.png", "x": 980, "y": 40 }),
    );
    let vault_img = ctx.id("vaultOpsRoomImage");
    if !vault_img.is_empty() {
        g.ok("moodboard.add_vault_image", json!({ "moodboardId": look, "vaultItemId": vault_img, "store": "project", "x": 980, "y": 600 }));
    }
    let link = g.ok(
        "moodboard.add_link",
        json!({ "moodboardId": look, "url": "https://en.wikipedia.org/wiki/Kodak_Vision3", "title": "Film stock reference for the 1999 grade", "x": 40, "y": 1100 }),
    );
    let note = g.ok(
        "moodboard.add_note",
        json!({ "moodboardId": look, "text": "Night exteriors: sodium orange against sea blue. Never pure black; keep detail in the water.", "x": 600, "y": 1100 }),
    );
    let private = g.ok(
        "moodboard.add_note",
        json!({ "moodboardId": look, "text": "Budget note for Vivek: balloon lights only on the beach night.", "isPrivate": true, "x": 1200, "y": 1100 }),
    );
    let _ = private;
    if let Some(first) = items.first() {
        g.ok(
            "moodboard.update_item",
            json!({ "id": first, "caption": "Monsoon palette from the art department share" }),
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
    if let Some(second) = items.get(1) {
        g.ok(
            "moodboard.update_item",
            json!({ "id": second, "caption": "Station at dusk: the hour we shoot scene 37" }),
        );
        g.ok(
            "moodboard.arrange_item",
            json!({ "id": second, "placement": "back" }),
        );
    }
    g.ok(
        "moodboard.update_item",
        json!({ "id": g.id(&link), "linkTitle": "Kodak Vision3: reference for the 1999 grain" }),
    );
    g.ok("moodboard.update_item", json!({ "id": g.id(&note), "body": "Night exteriors: sodium orange against sea blue. Never pure black; keep detail in the water and in faces." }));
    let moves: Vec<Value> = items
        .iter()
        .enumerate()
        .map(|(i, id)| json!({ "id": id, "x": 40 + i as i64 * 560, "y": 420 }))
        .collect();
    g.ok("moodboard.move_items", json!({ "moves": moves }));
    if let Some(storm) = items.get(2) {
        // The storm frame went to the scene board instead.
        g.ok("moodboard.delete_items", json!({ "ids": [storm] }));
    }
    let _ = pasted;
    g.ok("moodboard.update_notes", json!({ "id": look, "notes": "Shared with Vivek Anand (DOP) and Lakshmi Iyer (design). Grade reference for the DI." }));
    g.ok(
        "moodboard.rename",
        json!({ "id": look, "name": "Look of the film: monsoon night and the 1999 grade" }),
    );

    let beach = g.id(&g.ok(
        "moodboard.create",
        json!({ "name": "Scene 39: listening night", "sceneId": ctx.scenes[&39] }),
    ));
    ctx.set("moodBeach", beach.clone());
    g.ok(
        "moodboard.add_images",
        json!({ "moodboardId": beach, "paths": [ctx.media.path("beach-lanterns.png")] }),
    );
    g.ok("moodboard.add_note", json!({ "moodboardId": beach, "text": "Three hundred faces lit only by lanterns and the PA truck. Speakers face the sea, not the crowd." }));
    let room = g.id(&g.ok(
        "moodboard.create",
        json!({ "name": "1999 operations room", "sceneId": ctx.scenes[&2] }),
    ));
    g.ok("moodboard.add_images", json!({ "moodboardId": room, "paths": [ctx.media.path("operations-room-1999.png"), ctx.media.path("palette-1999.png")] }));
    g.ok("moodboard.add_note", json!({ "moodboardId": room, "text": "Bakelite, green dials, a Godrej steel cupboard. One working practical lamp." }));
    g.ok("moodboard.update_notes", json!({ "id": room, "notes": "Built inside the shell of the real station. Strike on day 2." }));
    let costume = g.id(&g.ok(
        "moodboard.create",
        json!({ "name": "Costume (moved to the wardrobe file)" }),
    ));
    g.ok("moodboard.delete", json!({ "id": costume }));
    g.ok("moodboard.reorder", json!({ "id": room, "index": 1 }));
    g.ok("moodboard.list", json!({}));
    g.ok("moodboard.suggestions", json!({}));
    g.ok("moodboard.get", json!({ "id": look }));
}

struct Shot<'a> {
    desc: &'a str,
    size: &'a str,
    movement: &'a str,
    angle: &'a str,
    lens: &'a str,
    notes: &'a str,
    chars: &'a [&'a str],
    sound: &'a str,
}

fn shots(g: &Gold, ctx: &mut Ctx) {
    g.phase("Shot lists");
    let lists: Vec<(u32, Vec<Shot>)> = vec![
        (
            1,
            vec![
                Shot {
                    desc: "Aerial plate: eleven boats in the swell, lanterns only",
                    size: "Extreme wide",
                    movement: "Drone, slow push",
                    angle: "High",
                    lens: "24mm",
                    notes: "VFX storm extension. Plate shot at dusk, graded to night.",
                    chars: &[],
                    sound: "Wild wind and engines",
                },
                Shot {
                    desc: "Varghese at the wheel, radio handset in his fist",
                    size: "Medium close-up",
                    movement: "Handheld on the boat",
                    angle: "Eye level",
                    lens: "40mm",
                    notes: "Rain bars on the wheelhouse.",
                    chars: &["VARGHESE"],
                    sound: "Mariamma on radio, played back",
                },
                Shot {
                    desc: "Insert: the set's dial as the signal breaks",
                    size: "Insert",
                    movement: "Static",
                    angle: "Overhead",
                    lens: "100mm macro",
                    notes: "Practical dial light flicker.",
                    chars: &[],
                    sound: "Static designed from Munambam recordings",
                },
            ],
        ),
        (
            2,
            vec![
                Shot {
                    desc: "Master: Mariamma at two radios, Young Anjali under the desk",
                    size: "Wide",
                    movement: "Slow dolly left",
                    angle: "Eye level",
                    lens: "32mm",
                    notes: "Child performer: this setup first.",
                    chars: &["MARIAMMA", "YOUNG ANJALI"],
                    sound: "Live radio chatter",
                },
                Shot {
                    desc: "Joe in the doorway with the telegram",
                    size: "Medium",
                    movement: "Static",
                    angle: "Low",
                    lens: "50mm",
                    notes: "Backlit by the corridor bulb.",
                    chars: &["JOE"],
                    sound: "",
                },
                Shot {
                    desc: "The lights die",
                    size: "Wide",
                    movement: "Static",
                    angle: "Eye level",
                    lens: "32mm",
                    notes: "Gaffer cue on 'Little Flower'. Candle practical ready.",
                    chars: &["MARIAMMA", "JOE"],
                    sound: "Generator dies, hum drop",
                },
            ],
        ),
        (
            16,
            vec![
                Shot {
                    desc: "The club holds its breath around the Nagra",
                    size: "Wide",
                    movement: "Slow push in",
                    angle: "Eye level",
                    lens: "28mm",
                    notes: "Six kids, one lamp.",
                    chars: &["ANJALI", "RAHUL", "FATHIMA"],
                    sound: "Room tone only before playback",
                },
                Shot {
                    desc: "Close on the reel turning",
                    size: "Extreme close-up",
                    movement: "Static",
                    angle: "Top shot",
                    lens: "100mm macro",
                    notes: "Hero reel on the real Nagra.",
                    chars: &[],
                    sound: "Playback: Mariamma's laugh",
                },
                Shot {
                    desc: "Anjali's hand stops the machine",
                    size: "Insert",
                    movement: "Static",
                    angle: "Eye level",
                    lens: "65mm",
                    notes: "",
                    chars: &["ANJALI"],
                    sound: "Tape drag",
                },
            ],
        ),
        (
            24,
            vec![
                Shot {
                    desc: "Wide: the breakwater, Appachan a torch at the far end",
                    size: "Extreme wide",
                    movement: "Crane down",
                    angle: "High",
                    lens: "21mm",
                    notes: "Wave machine plus rain towers. VFX wave composite.",
                    chars: &["APPACHAN"],
                    sound: "Wind, surf",
                },
                Shot {
                    desc: "Suresh crawls over the tetrapods",
                    size: "Medium",
                    movement: "Handheld",
                    angle: "Low",
                    lens: "35mm",
                    notes: "Harness on the far side, painted out.",
                    chars: &["SURESH"],
                    sound: "Breath, rain",
                },
                Shot {
                    desc: "Suresh wraps Appachan in his jacket",
                    size: "Two-shot",
                    movement: "Handheld",
                    angle: "Eye level",
                    lens: "40mm",
                    notes: "",
                    chars: &["SURESH", "APPACHAN"],
                    sound: "Kannan barking off",
                },
                Shot {
                    desc: "A wave takes the torch",
                    size: "Insert",
                    movement: "Static",
                    angle: "Low",
                    lens: "50mm",
                    notes: "SFX dump tank.",
                    chars: &[],
                    sound: "Wave hit",
                },
            ],
        ),
        (
            33,
            vec![
                Shot {
                    desc: "The second half plays: faces around the Nagra",
                    size: "Medium wide",
                    movement: "Slow arc",
                    angle: "Eye level",
                    lens: "35mm",
                    notes: "One continuous take for the whole playback.",
                    chars: &["ANJALI", "RAHUL", "FATHIMA"],
                    sound: "Playback: Mariamma, then Varghese",
                },
                Shot {
                    desc: "Rahul cries",
                    size: "Close-up",
                    movement: "Static",
                    angle: "Eye level",
                    lens: "75mm",
                    notes: "",
                    chars: &["RAHUL"],
                    sound: "",
                },
            ],
        ),
        (
            39,
            vec![
                Shot {
                    desc: "Wide: three hundred people on the sand, lanterns",
                    size: "Extreme wide",
                    movement: "Drone pull back",
                    angle: "High",
                    lens: "24mm",
                    notes: "Balloon lights hidden in the boats.",
                    chars: &[],
                    sound: "Reel over the PA",
                },
                Shot {
                    desc: "Suresh stands and reads the names",
                    size: "Medium close-up",
                    movement: "Static",
                    angle: "Eye level",
                    lens: "65mm",
                    notes: "",
                    chars: &["SURESH"],
                    sound: "Live, PA bleed managed",
                },
                Shot {
                    desc: "Joe at the back, Latha beside him",
                    size: "Medium",
                    movement: "Static",
                    angle: "Eye level",
                    lens: "85mm",
                    notes: "",
                    chars: &["JOE", "LATHA"],
                    sound: "",
                },
            ],
        ),
        (
            41,
            vec![
                Shot {
                    desc: "Three cameras: the mast falls into the grove",
                    size: "Wide",
                    movement: "Static",
                    angle: "Low",
                    lens: "18mm / 50mm / 135mm",
                    notes: "One take. VFX clean-up of safety cables.",
                    chars: &[],
                    sound: "Recorded live with three recorders",
                },
                Shot {
                    desc: "Appachan claps once",
                    size: "Close-up",
                    movement: "Static",
                    angle: "Eye level",
                    lens: "85mm",
                    notes: "",
                    chars: &["APPACHAN"],
                    sound: "",
                },
            ],
        ),
    ];
    for (n, list) in &lists {
        let Some(scene) = ctx.scenes.get(n).cloned() else {
            continue;
        };
        for sh in list {
            let r = g.ok(
                "shot.create",
                json!({ "sceneId": scene, "description": sh.desc, "size": sh.size, "movement": sh.movement, "angle": sh.angle,
                        "lens": sh.lens, "cameraNotes": sh.notes, "characters": sh.chars, "soundNote": sh.sound }),
            );
            ctx.ids
                .entry(format!("shot:{n}"))
                .or_insert_with(|| g.id(&r));
        }
    }
    let s24 = ctx.scenes[&24].clone();
    let list24 = g.ok("shot.list", json!({ "sceneId": s24 }));
    let ids24: Vec<String> = arr(&list24).iter().map(|x| s(&x["id"])).collect();
    if ids24.len() >= 4 {
        g.ok("shot.update", json!({ "id": ids24[1], "cameraNotes": "Harness on the far side, painted out. Stunt coordinator signs off each take.", "lens": "35mm anamorphic" }));
        g.ok("shot.reorder", json!({ "id": ids24[3], "index": 2 }));
        let dup = g.ok("shot.duplicate", json!({ "id": ids24[0] }));
        g.ok("shot.update", json!({ "id": g.id(&dup), "description": "Safety wide from the harbour road (second camera)", "lens": "135mm" }));
        g.ok(
            "shot.set_reference_image",
            json!({ "id": ids24[0], "path": ctx.media.path("board-breakwater-01.png") }),
        );
        g.ok("shot.get", json!({ "id": ids24[0] }));
    }
    // A shot written in scene 23 belongs to 24; moved. A spare is deleted.
    let s23 = ctx.scenes[&23].clone();
    let stray = g.ok("shot.create", json!({ "sceneId": s23, "description": "Kannan running along the breakwater", "size": "Wide", "movement": "Tracking", "characters": [] }));
    g.ok(
        "shot.move",
        json!({ "id": g.id(&stray), "sceneId": s24, "index": 4 }),
    );
    let spare = g.ok(
        "shot.create",
        json!({ "sceneId": s24, "description": "Spare: torch POV", "size": "POV" }),
    );
    g.ok("shot.delete", json!({ "ids": [g.id(&spare)] }));
    // The reel playback coverage of scene 16 is the plan for scene 13's inspection too.
    if let (Some(from), Some(to)) = (ctx.lineage.get(&16), ctx.scenes.get(&13)) {
        g.ok(
            "shot.copy_planning",
            json!({ "fromSceneLineageId": from, "toSceneId": to }),
        );
    }
    g.ok("shot.list", json!({ "includeRemoved": true }));
    ctx.set("shots24", ids24.join(","));
}

fn storyboards(g: &Gold, ctx: &mut Ctx) {
    g.phase("Storyboards");
    let s24 = ctx.scenes[&24].clone();
    let board = g.ok("storyboard.create", json!({ "sceneId": s24 }));
    let b24 = g.id(&board);
    ctx.set("board24", b24.clone());
    let p1 = g.ok(
        "storyboard.add_panel",
        json!({ "storyboardId": b24, "visual": "sketch", "dataBase64": b64_file(ctx, "board-breakwater-01.png"), "description": "Wide: the breakwater, the torch at the far end." }),
    );
    let p2 = g.ok(
        "storyboard.add_panel",
        json!({ "storyboardId": b24, "visual": "sketch", "dataBase64": b64_file(ctx, "board-breakwater-02.png"), "description": "Appachan at the edge, signalling to boats that are not there." }),
    );
    let p3 = g.ok(
        "storyboard.add_panel",
        json!({ "storyboardId": b24, "visual": "image", "path": ctx.media.path("sea-storm-turn.png"), "description": "The wave: VFX reference frame." }),
    );
    let p4 = g.ok("storyboard.add_panel", json!({ "storyboardId": b24, "visual": "placeholder", "description": "Suresh carries him back. To draw." }));
    for (p, framing, movement, angle, sound, ms, note) in [
        (
            &p1,
            "Extreme wide",
            "Crane down",
            "High",
            "Wind and surf",
            6000,
            "Establish the yellow safety line out of frame.",
        ),
        (&p2, "Medium", "Handheld", "Low", "Kannan barking", 4000, ""),
        (
            &p3,
            "Wide",
            "Static",
            "Eye level",
            "Wave hit",
            2500,
            "Composite of the dump tank and a plate.",
        ),
        (
            &p4,
            "Two-shot",
            "Handheld follow",
            "Eye level",
            "Breath, rain",
            8000,
            "",
        ),
    ] {
        g.ok(
            "storyboard.update_panel",
            json!({ "id": g.id(p), "framing": framing, "movement": movement, "angle": angle, "soundNote": sound, "durationMs": ms, "note": note }),
        );
    }
    g.ok("storyboard.set_panel_visual", json!({ "id": g.id(&p4), "visual": "sketch", "dataBase64": b64_file(ctx, "board-breakwater-02.png") }));
    g.ok(
        "storyboard.reorder_panel",
        json!({ "id": g.id(&p3), "index": 1 }),
    );
    g.ok(
        "storyboard.reorder_panel",
        json!({ "id": g.id(&p3), "index": 2 }),
    );
    let ids24: Vec<String> = ctx
        .id("shots24")
        .split(',')
        .filter(|x| !x.is_empty())
        .map(str::to_string)
        .collect();
    if let Some(sh) = ids24.first() {
        g.ok(
            "storyboard.link_shot",
            json!({ "panelId": g.id(&p1), "shotId": sh }),
        );
        g.ok(
            "shot.attach_panel",
            json!({ "shotId": sh, "panelId": g.id(&p2) }),
        );
        g.ok(
            "shot.detach_panel",
            json!({ "shotId": sh, "panelId": g.id(&p2) }),
        );
    }
    g.ok(
        "storyboard.create_shot_from_panel",
        json!({ "panelId": g.id(&p4) }),
    );
    if let Some(sh) = ids24.get(2) {
        g.ok("shot.create_panel", json!({ "id": sh }));
    }
    g.ok(
        "storyboard.link_shot",
        json!({ "panelId": g.id(&p3), "shotId": null }),
    );

    let s33 = ctx.scenes[&33].clone();
    let b33 = g.id(&g.ok(
        "storyboard.create",
        json!({ "sceneId": s33, "name": "Scene 33: the second half of the reel" }),
    ));
    g.ok("storyboard.add_panel", json!({ "storyboardId": b33, "visual": "sketch", "dataBase64": b64_file(ctx, "board-reel-01.png"), "description": "Hands on the reel." }));
    let s39 = ctx.scenes[&39].clone();
    let b39 = g.id(&g.ok("storyboard.create", json!({ "sceneId": s39 })));
    g.ok("storyboard.add_panel", json!({ "storyboardId": b39, "visual": "image", "path": ctx.media.path("board-beach-01.png"), "description": "The crowd faces the speakers." }));
    // The title sequence board started standalone and was attached to scene 1.
    let titles = g.id(&g.ok("storyboard.create", json!({ "name": "Title sequence" })));
    let tp = g.ok("storyboard.add_panel", json!({ "storyboardId": titles, "visual": "image", "path": ctx.media.path("sea-storm-1999.png"), "description": "Title over the black sea." }));
    let stray = g.ok("storyboard.add_panel", json!({ "storyboardId": titles, "visual": "image", "path": ctx.media.path("beach-lanterns.png"), "description": "Lanterns: belongs to scene 39." }));
    g.ok(
        "storyboard.move_panel",
        json!({ "id": g.id(&stray), "storyboardId": b39, "index": 0 }),
    );
    g.ok(
        "storyboard.set_scene",
        json!({ "id": titles, "sceneId": ctx.scenes[&1] }),
    );
    g.ok(
        "storyboard.rename",
        json!({ "id": titles, "name": "Scene 1: title sequence over the storm" }),
    );
    let _ = tp;
    let spare = g.ok(
        "storyboard.add_panel",
        json!({ "storyboardId": b33, "visual": "placeholder", "description": "spare" }),
    );
    g.ok("storyboard.delete_panels", json!({ "ids": [g.id(&spare)] }));
    let old = g.id(&g.ok(
        "storyboard.create",
        json!({ "name": "Breakwater (old thumbnails)" }),
    ));
    g.ok("storyboard.delete", json!({ "id": old }));
    g.ok("storyboard.list", json!({}));
    g.ok("storyboard.get", json!({ "id": b24 }));
}
