//! Stress fixture generator ("Feature-120", docs/engineering/21-performance-budgets.md Â§1).
//!
//! Builds a large, realistic project **through the operation registry only**
//! (every row is created by the same ops the desktop UI calls), so the data has
//! exactly the shape real use produces: search documents, activity, undo
//! history, assets on disk, breakdown/schedule derivations.
//!
//! All text is synthetic and deterministic (a fixed-seed generator), so runs
//! are comparable.

use std::time::Instant;

use serde_json::{Value, json};

use crate::TestEnv;

/// Sizes of the generated project.
#[derive(Debug, Clone)]
pub struct StressSpec {
    pub scenes: usize,
    /// Dialogue exchanges per scene (each adds character + dialogue + action,
    /// every third a parenthetical): ~3.3 elements per exchange.
    pub exchanges_per_scene: usize,
    pub acts: usize,
    pub sequences: usize,
    pub cards: usize,
    pub vault_notes: usize,
    pub vault_images: usize,
    pub catalog_items: usize,
    pub breakdown_elements: usize,
    pub locations: usize,
    pub cast: usize,
    pub crew: usize,
    pub shots: usize,
    pub storyboards: usize,
    pub panels_per_storyboard: usize,
    pub days: usize,
    pub call_sheets: usize,
    /// Total drafts (the imported one plus `drafts - 1` new drafts).
    pub drafts: usize,
}

impl StressSpec {
    /// The reference stress project: a 120-page feature and a full production.
    pub fn feature_120() -> Self {
        Self {
            scenes: 180,
            exchanges_per_scene: 10,
            acts: 3,
            sequences: 30,
            cards: 250,
            vault_notes: 1_200,
            vault_images: 300,
            catalog_items: 200,
            breakdown_elements: 800,
            locations: 60,
            cast: 40,
            crew: 40,
            shots: 900,
            storyboards: 40,
            panels_per_storyboard: 10,
            days: 30,
            call_sheets: 30,
            drafts: 5,
        }
    }

    /// A small variant with the same shape, for quick runs of the harness itself.
    pub fn small() -> Self {
        Self {
            scenes: 12,
            exchanges_per_scene: 4,
            acts: 3,
            sequences: 6,
            cards: 20,
            vault_notes: 30,
            vault_images: 6,
            catalog_items: 10,
            breakdown_elements: 24,
            locations: 5,
            cast: 6,
            crew: 4,
            shots: 24,
            storyboards: 3,
            panels_per_storyboard: 3,
            days: 3,
            call_sheets: 3,
            drafts: 2,
        }
    }
}

/// Identities of what was generated (for the measurements).
#[derive(Debug, Clone, Default)]
pub struct StressProject {
    pub project_path: String,
    pub screenplay_id: String,
    /// The draft production works from (the imported one).
    pub draft_id: String,
    pub draft_ids: Vec<String>,
    /// Scenes of `draft_id`, in order.
    pub scene_ids: Vec<String>,
    /// First action element of each scene of `draft_id`.
    pub action_ids: Vec<String>,
    pub element_count: usize,
    pub act_ids: Vec<String>,
    pub sequence_ids: Vec<String>,
    pub card_ids: Vec<String>,
    pub vault_ids: Vec<String>,
    pub catalog_ids: Vec<String>,
    pub location_ids: Vec<String>,
    pub shot_ids: Vec<String>,
    pub storyboard_ids: Vec<String>,
    pub schedule_id: String,
    pub day_ids: Vec<String>,
    pub call_sheet_ids: Vec<String>,
    /// Seconds spent per phase (reported by the perf suite).
    pub phases: Vec<(&'static str, f64)>,
}

// ------------------------------------------------------------ deterministic text

/// Small fixed-seed PRNG (SplitMix64) â€” no dependency, stable across platforms.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n.max(1) as u64) as usize
    }
    pub fn pick<'a>(&mut self, items: &'a [&'a str]) -> &'a str {
        items[self.below(items.len())]
    }
}

const WORDS: &[&str] = &[
    "rain",
    "platform",
    "window",
    "letter",
    "train",
    "shadow",
    "kettle",
    "street",
    "lamp",
    "river",
    "folder",
    "pistol",
    "umbrella",
    "ticket",
    "station",
    "market",
    "radio",
    "mirror",
    "ledger",
    "bicycle",
    "photograph",
    "harbour",
    "signal",
    "lantern",
    "bridge",
    "key",
    "suitcase",
    "candle",
    "notebook",
    "telephone",
    "courtyard",
    "stairwell",
    "rooftop",
    "ferry",
    "garden",
    "tram",
    "the",
    "a",
    "slowly",
    "again",
    "quietly",
    "across",
    "toward",
    "behind",
    "under",
    "beside",
    "waits",
    "turns",
    "listens",
    "runs",
    "hesitates",
    "watches",
    "smiles",
    "remembers",
    "pauses",
];

pub const CHARACTERS: &[&str] = &[
    "ARJUN", "MEERA", "RAVI", "NISHA", "KARTHIK", "LEELA", "VIKRAM", "ANANYA", "SURESH", "PRIYA",
    "DEV", "KAVYA", "MOHAN", "ISHA", "ROHAN", "TARA", "SAMEER", "DIVYA", "GOPAL", "RADHA",
    "ARVIND", "SITA", "KIRAN", "MAYA", "NAVEEN", "LATA", "RAJESH", "POOJA", "HARI", "USHA", "AMIT",
    "REKHA", "SUNIL", "GITA", "PRAKASH", "ASHA", "VINOD", "SHOBHA", "RAMESH", "LAKSHMI",
];

const PLACES: &[&str] = &[
    "BUS STOP",
    "POLICE STATION",
    "APARTMENT",
    "RAILWAY PLATFORM",
    "MARKET",
    "TEA STALL",
    "HOSPITAL CORRIDOR",
    "ROOFTOP",
    "HARBOUR",
    "OLD MILL",
    "SCHOOL YARD",
    "TEMPLE STEPS",
    "NEWSPAPER OFFICE",
    "COURTROOM",
    "FERRY DECK",
    "BRIDGE",
    "TAILOR SHOP",
    "CINEMA HALL",
    "GARAGE",
    "LIBRARY",
    "BUNGALOW",
    "HOSTEL ROOM",
    "BANK VAULT",
    "FISH MARKET",
    "BEACH",
    "HIGHWAY DHABA",
    "RAILWAY YARD",
    "PRINTING PRESS",
    "RADIO STATION",
    "WEDDING HALL",
];

const TIMES: &[&str] = &["DAY", "NIGHT", "MORNING", "EVENING", "DAWN", "DUSK"];

fn sentence(rng: &mut Rng, words: usize) -> String {
    let mut s = String::new();
    for i in 0..words {
        if i > 0 {
            s.push(' ');
        }
        s.push_str(rng.pick(WORDS));
    }
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str() + ".",
        None => s,
    }
}

/// Place name used for the heading of scene `i` (60 distinct sets).
pub fn scene_place(i: usize) -> String {
    let p = PLACES[i % PLACES.len()];
    if (i / PLACES.len()) % 2 == 1 {
        format!("{p} ANNEX")
    } else {
        p.to_string()
    }
}

/// A Fountain screenplay with `scenes` scenes of `exchanges` dialogue exchanges.
pub fn fountain(scenes: usize, exchanges: usize, seed: u64) -> String {
    let mut rng = Rng::new(seed);
    let mut out = String::from(
        "Title: Night Bus\nCredit: Written by\nAuthor: Stress Fixture\n\nFADE IN:\n\n",
    );
    for i in 0..scenes {
        let ie = if i % 3 == 0 { "EXT." } else { "INT." };
        let place = scene_place(i % 60);
        let time = TIMES[i % TIMES.len()];
        out.push_str(&format!("{ie} {place} - {time}\n\n"));
        out.push_str(&sentence(&mut rng, 14));
        out.push_str("\n\n");
        for x in 0..exchanges {
            let who = CHARACTERS[(i + x * 7 + rng.below(3)) % CHARACTERS.len()];
            out.push_str(who);
            out.push('\n');
            if x % 3 == 2 {
                out.push_str("(quietly)\n");
            }
            out.push_str(&sentence(&mut rng, 7));
            out.push_str("\n\n");
            out.push_str(&sentence(&mut rng, 9));
            out.push_str("\n\n");
        }
        if i % 10 == 9 {
            out.push_str("CUT TO:\n\n");
        }
    }
    out
}

// ------------------------------------------------------------------ tiny PNGs

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &d in data {
        a = (a + d as u32) % 65_521;
        b = (b + a) % 65_521;
    }
    (b << 16) | a
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut c = Vec::with_capacity(4 + data.len());
    c.extend_from_slice(kind);
    c.extend_from_slice(data);
    out.extend_from_slice(&c);
    out.extend_from_slice(&crc32(&c).to_be_bytes());
}

/// A small, valid, *distinct* RGB PNG (stored deflate blocks, no dependency).
pub fn png(width: u32, height: u32, seed: u32) -> Vec<u8> {
    let mut raw = Vec::with_capacity(((width * 3 + 1) * height) as usize);
    for y in 0..height {
        raw.push(0); // filter: none
        for x in 0..width {
            raw.push((x * 255 / width.max(1)) as u8 ^ (seed as u8));
            raw.push((y * 255 / height.max(1)) as u8 ^ ((seed >> 8) as u8));
            raw.push((seed.wrapping_mul(31) >> 3) as u8);
        }
    }
    let mut z = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = raw.chunks(65_535).collect();
    for (i, b) in blocks.iter().enumerate() {
        z.push(if i + 1 == blocks.len() { 1 } else { 0 });
        let len = b.len() as u16;
        z.extend_from_slice(&len.to_le_bytes());
        z.extend_from_slice(&(!len).to_le_bytes());
        z.extend_from_slice(b);
    }
    z.extend_from_slice(&adler32(&raw).to_be_bytes());
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &z);
    chunk(&mut out, b"IEND", &[]);
    out
}

// ------------------------------------------------------------------ builder

fn id_of(v: &Value) -> String {
    v.as_str()
        .map(str::to_string)
        .or_else(|| v["id"].as_str().map(str::to_string))
        .unwrap_or_else(|| panic!("no id in {v}"))
}

struct Phases {
    at: Instant,
    list: Vec<(&'static str, f64)>,
}

impl Phases {
    fn mark(&mut self, name: &'static str) {
        self.list.push((name, self.at.elapsed().as_secs_f64()));
        self.at = Instant::now();
    }
}

/// Build the stress project into the (already open) project of `env`.
pub fn build(env: &TestEnv, spec: &StressSpec) -> StressProject {
    use base64::Engine as _;
    let b64 = base64::engine::general_purpose::STANDARD;
    let mut ph = Phases {
        at: Instant::now(),
        list: vec![],
    };
    let mut out = StressProject {
        project_path: env.project_path(),
        ..Default::default()
    };
    let mut rng = Rng::new(0x000F_12A5);

    // --- Screenplay: one Fountain import (the path users take for an existing script).
    let text = fountain(spec.scenes, spec.exchanges_per_scene, 7);
    let p = env.ok("screenplay.import_preview", json!({ "pastedText": text }));
    let r = env.ok(
        "screenplay.import_apply",
        json!({ "previewId": p["previewId"], "mode": "new_screenplay" }),
    );
    out.screenplay_id = id_of(&r["screenplayId"]);
    let first = id_of(&r["draftId"]);
    out.draft_ids.push(first.clone());
    ph.mark("screenplay import");
    // Further drafts (each copies the whole script and records a history point);
    // the newest is current and is the one production and the editor work on.
    let mut src = first;
    for d in 1..spec.drafts {
        let nd = env.ok(
            "screenplay.new_draft",
            json!({ "sourceDraftId": src, "name": format!("Draft {}", d + 1) }),
        );
        src = id_of(&nd);
        out.draft_ids.push(src.clone());
    }
    out.draft_id = src;
    let doc = env.ok("screenplay.document", json!({ "draftId": out.draft_id }));
    for s in doc["scenes"].as_array().expect("scenes") {
        out.scene_ids.push(id_of(&s["id"]));
        let els = s["elements"].as_array().expect("elements");
        out.element_count += els.len();
        if let Some(a) = els.iter().find(|e| e["elementType"] == "action") {
            out.action_ids.push(id_of(&a["id"]));
        }
    }
    assert_eq!(out.scene_ids.len(), spec.scenes, "imported scene count");
    ph.mark("drafts");

    // --- Story: acts â†’ sequences â†’ scene cards.
    for a in 0..spec.acts {
        out.act_ids.push(id_of(&env.ok(
            "story.create_act",
            json!({ "title": format!("Act {}", a + 1) }),
        )));
    }
    for q in 0..spec.sequences {
        let act = &out.act_ids[q * spec.acts / spec.sequences.max(1)];
        out.sequence_ids.push(id_of(&env.ok(
            "story.create_sequence",
            json!({ "actId": act, "title": format!("Sequence {}", q + 1) }),
        )));
    }
    for c in 0..spec.cards {
        let seq = &out.sequence_ids[c * spec.sequences / spec.cards.max(1)];
        out.card_ids.push(id_of(&env.ok(
            "story.create_card",
            json!({ "parent": { "parentType": "sequence", "parentId": seq },
                    "shortDescription": sentence(&mut rng, 12) }),
        )));
    }
    ph.mark("story");

    // --- Idea Vault: notes and images.
    for n in 0..spec.vault_notes {
        out.vault_ids.push(id_of(&env.ok(
            "vault.create",
            json!({ "store": "project", "itemType": "note",
                    "title": format!("Idea {n}: {}", sentence(&mut rng, 4)),
                    "body": sentence(&mut rng, 40) }),
        )));
    }
    for n in 0..spec.vault_images {
        let bytes = png(48, 32, n as u32 + 1);
        out.vault_ids.push(id_of(&env.ok(
            "vault.ingest_bytes",
            json!({ "store": "project", "itemType": "image", "fileName": format!("ref-{n}.png"),
                    "mediaType": "image/png", "dataBase64": b64.encode(&bytes) }),
        )));
    }
    ph.mark("idea vault");

    // --- Production: source, catalog, breakdown, locations, cast, crew.
    env.ok(
        "production.set_source",
        json!({ "draftId": out.draft_id, "reason": "Shooting draft" }),
    );
    const CATS: &[&str] = &[
        "Props",
        "Wardrobe",
        "Vehicles",
        "Hair / Makeup",
        "Special Effects",
        "VFX",
        "Sound",
        "Animals",
        "Extras / Background",
    ];
    let mut catalog_names = Vec::new();
    for i in 0..spec.catalog_items {
        let cat = CATS[i % CATS.len()];
        let name = format!("{} {i}", rng.pick(WORDS));
        out.catalog_ids.push(id_of(
            &env.ok("catalog.create", json!({ "category": cat, "name": name })),
        ));
        catalog_names.push((cat, name));
    }
    // One Cast catalog entry per speaking character (what accepting cast suggestions creates).
    let cast_items: Vec<String> = CHARACTERS
        .iter()
        .map(|c| id_of(&env.ok("catalog.create", json!({ "category": "Cast", "name": c }))))
        .collect();
    for i in 0..spec.breakdown_elements {
        let scene = &out.scene_ids[i % out.scene_ids.len()];
        if i % 4 == 0 || catalog_names.is_empty() {
            let c = (i + i / out.scene_ids.len() * 7) % CHARACTERS.len();
            env.ok(
                "breakdown.add_element",
                json!({ "sceneId": scene, "category": "Cast", "name": CHARACTERS[c],
                        "catalog": { "mode": "existing", "catalogItemId": cast_items[c] } }),
            );
        } else {
            // Tagged against an existing catalog item (the breakdown's normal "use existing" choice).
            let k = i % catalog_names.len();
            let (cat, name) = &catalog_names[k];
            env.ok(
                "breakdown.add_element",
                json!({ "sceneId": scene, "category": cat, "name": name,
                        "catalog": { "mode": "existing", "catalogItemId": out.catalog_ids[k] } }),
            );
        }
    }
    ph.mark("catalog + breakdown");
    for i in 0..spec.locations {
        out.location_ids.push(id_of(&env.ok(
            "locations.create",
            json!({ "name": scene_place(i).to_lowercase(), "address": format!("{} Market Road", i + 1),
                    "status": "Confirmed" }),
        )));
    }
    for i in 0..spec.cast {
        env.ok(
            "cast.create",
            json!({ "personName": format!("Actor {i}"), "characterName": CHARACTERS[i % CHARACTERS.len()] }),
        );
    }
    for i in 0..spec.crew {
        env.ok(
            "crew.create",
            json!({ "personName": format!("Crew {i}"), "role": "Grip", "department": "Camera" }),
        );
    }
    ph.mark("locations + people");

    // --- Shots and storyboards.
    for i in 0..spec.shots {
        let scene = &out.scene_ids[i * out.scene_ids.len() / spec.shots.max(1)];
        out.shot_ids.push(id_of(&env.ok(
            "shot.create",
            json!({ "sceneId": scene, "description": sentence(&mut rng, 6) }),
        )));
    }
    for b in 0..spec.storyboards {
        let scene = &out.scene_ids[b * out.scene_ids.len() / spec.storyboards.max(1)];
        let board = id_of(&env.ok("storyboard.create", json!({ "sceneId": scene })));
        for _ in 0..spec.panels_per_storyboard {
            env.ok(
                "storyboard.add_panel",
                json!({ "storyboardId": board, "visual": "placeholder",
                        "description": sentence(&mut rng, 6) }),
            );
        }
        out.storyboard_ids.push(board);
    }
    ph.mark("shots + storyboards");

    // --- Schedule: days, every strip placed on a day, call sheets.
    out.schedule_id = id_of(&env.ok("schedule.create", json!({})));
    for d in 0..spec.days {
        out.day_ids.push(id_of(&env.ok(
            "schedule.create_day",
            json!({ "scheduleId": out.schedule_id,
                    "date": format!("2027-{:02}-{:02}", 3 + d / 28, (d % 28) + 1) }),
        )));
    }
    let sv = env.ok("schedule.get", json!({}));
    let strips: Vec<String> = sv["unscheduled"]
        .as_array()
        .expect("unscheduled")
        .iter()
        .map(|s| id_of(&s["id"]))
        .collect();
    let per_day = strips.len().div_ceil(spec.days.max(1)).max(1);
    for (d, chunk) in strips.chunks(per_day).enumerate() {
        env.ok(
            "schedule.move_strips",
            json!({ "stripIds": chunk, "dayId": out.day_ids[d] }),
        );
    }
    for d in 0..spec.call_sheets.min(out.day_ids.len()) {
        out.call_sheet_ids.push(id_of(
            &env.ok("callsheets.create", json!({ "dayId": out.day_ids[d] })),
        ));
    }
    ph.mark("schedule + call sheets");
    out.phases = ph.list;
    out
}
