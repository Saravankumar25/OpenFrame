//! Sample generator for the redesigned production PDFs (not a regression test).
//!
//! Builds a fictional short film ("Monsoon Letters") with a fuller data set than the regression
//! fixture and writes every redesigned document into `release-out/pdf-samples/` (git ignored) so
//! the pages can be rasterised and reviewed by eye:
//!
//! ```text
//! node scripts/cargo-queued.mjs test -p openframe-application --test production \
//!     pdf_samples:: -- --ignored --nocapture
//! ```

use std::path::{Path, PathBuf};

use openframe_test_support::TestEnv;
use serde_json::{Value, json};

fn out_dir() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repository root")
        .to_path_buf();
    let d = root.join("release-out").join("pdf-samples");
    std::fs::create_dir_all(&d).expect("sample folder");
    d
}

fn target(name: &str) -> String {
    out_dir().join(name).to_string_lossy().into_owned()
}

fn exec(env: &TestEnv, sql: &str) {
    env.core
        .project()
        .unwrap()
        .store
        .with_writer(|c| {
            c.execute_batch(sql).expect("fixture sql");
            Ok(())
        })
        .unwrap();
}

fn q(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

pub(crate) fn id_of(v: &Value) -> String {
    v["id"]
        .as_str()
        .or_else(|| v.as_str())
        .expect("id")
        .to_string()
}

// ------------------------------------------------------------------ pictures

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for b in data {
        crc ^= u32::from(*b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for x in data {
        a = (a + u32::from(*x)) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

fn png(w: u32, h: u32, px: impl Fn(u32, u32) -> [u8; 3]) -> Vec<u8> {
    let mut raw = Vec::with_capacity((w * h * 3 + h) as usize);
    for y in 0..h {
        raw.push(0);
        for x in 0..w {
            raw.extend(px(x, y));
        }
    }
    let mut z = vec![0x78, 0x01];
    let mut chunks = raw.chunks(65535).peekable();
    while let Some(c) = chunks.next() {
        z.push(u8::from(chunks.peek().is_none()));
        z.extend((c.len() as u16).to_le_bytes());
        z.extend((!(c.len() as u16)).to_le_bytes());
        z.extend(c);
    }
    z.extend(adler32(&raw).to_be_bytes());
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    let mut chunk = |kind: &[u8; 4], data: &[u8]| {
        out.extend((data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend(data);
        out.extend(&body);
        out.extend(crc32(&body).to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend(w.to_be_bytes());
    ihdr.extend(h.to_be_bytes());
    ihdr.extend([8, 2, 0, 0, 0]);
    chunk(b"IHDR", &ihdr);
    chunk(b"IDAT", &z);
    chunk(b"IEND", &[]);
    out
}

/// A simple cinematic still: sky gradient, a low sun or moon, a dark ground band and a figure.
pub(crate) fn still(theme: usize, w: u32, h: u32) -> Vec<u8> {
    const SKY: [([u8; 3], [u8; 3]); 6] = [
        ([24, 38, 66], [226, 140, 76]),
        ([40, 64, 92], [168, 196, 206]),
        ([18, 22, 34], [64, 78, 104]),
        ([92, 56, 48], [244, 188, 112]),
        ([30, 52, 48], [140, 176, 150]),
        ([60, 50, 84], [220, 170, 160]),
    ];
    let (top, bottom) = SKY[theme % SKY.len()];
    png(w, h, |x, y| {
        let t = y as f32 / h as f32;
        let mix = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * t) as u8;
        let mut p = [
            mix(top[0], bottom[0]),
            mix(top[1], bottom[1]),
            mix(top[2], bottom[2]),
        ];
        let (cx, cy) = (
            w as f32 * (0.25 + 0.12 * (theme % 5) as f32),
            h as f32 * 0.52,
        );
        let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
        if d < h as f32 * 0.11 {
            p = [250, 232, 190];
        }
        if y as f32 > h as f32 * 0.68 {
            let g = 26 + ((x / 7 + y / 5) % 9) as u8;
            p = [g, g + 4, g + 10];
        }
        let fx = w as f32 * 0.62;
        if (x as f32 - fx).abs() < h as f32 * 0.035
            && y as f32 > h as f32 * 0.45
            && (y as f32) < h as f32 * 0.7
        {
            p = [14, 14, 20];
        }
        p
    })
}

// ------------------------------------------------------------------ project

const SCENES: [(&str, &str, &str); 14] = [
    (
        "INT. SAVITRI'S KITCHEN - DAWN",
        "Savitri packs a tiffin for a journey she has not told her daughter about, while the radio reads out the flood warning for the river districts.",
        "Savitri wraps two rotis in newspaper and tucks a bundle of letters under the rice. The radio crackles with a flood warning. She switches it off and listens to the rain instead.",
    ),
    (
        "EXT. GUDUR BUS STAND - MORNING",
        "Savitri waits among sleeping porters. Arjun, the conductor, recognises her from thirty years ago and says nothing.",
        "A tin-roofed bus stand. Porters sleep on sacks. ARJUN (60s, khaki uniform gone soft with washing) punches tickets without looking up, then looks up.",
    ),
    (
        "INT. STATE BUS - DAY (MOVING)",
        "The bus climbs out of the town in heavy rain. Savitri reads the first letter aloud to a child who is not listening, and Arjun listens from the front without turning round.",
        "Rain hammers the roof. PINKY (8) kneels on the seat behind SAVITRI and draws on the fogged window. Savitri unfolds a yellowed letter and reads in a low voice.",
    ),
    (
        "EXT. FLOODED HIGHWAY - DAY",
        "The road disappears under brown water. Passengers push the bus through while Arjun walks ahead with a bamboo pole, testing the depth.",
        "Brown water to the axles. The passengers lean into the bus. Arjun wades ahead, probing with a bamboo pole. Savitri watches him from the window, the letter still in her hand.",
    ),
    (
        "INT. TEA STALL - DAY",
        "Stranded travellers share one stove. The tea-seller offers Savitri a blanket and a question she answers with a lie.",
        "A tarpaulin stall, one stove, a dozen wet strangers. The TEA-SELLER (50s) pours without being asked.",
    ),
    (
        "EXT. RIVER BRIDGE - EVENING",
        "The bridge is closed. Savitri crosses on foot, alone, against the advice of everyone, with the river loud beneath her.",
        "The river is the colour of tea. A barricade, a policeman who looks away. Savitri climbs over with her bundle.",
    ),
    (
        "INT. DAK BUNGALOW - NIGHT",
        "A caretaker lets her in out of the rain. The house is exactly as the letters described it, down to the cracked blue lamp.",
        "A dak bungalow with a veranda that sags. The CARETAKER (70s) holds a hurricane lamp high and squints.",
    ),
    (
        "INT. DAK BUNGALOW, LETTER ROOM - NIGHT",
        "Savitri finds the writing desk and the unsent replies, forty years of them, tied in jute twine.",
        "A narrow room, a desk, a window with no glass. Forty years of unsent replies, tied in jute twine. Savitri reads the top one and sits down on the floor.",
    ),
    (
        "EXT. DAK BUNGALOW GARDEN - NIGHT",
        "Arjun arrives on foot, soaked, holding the conductor's whistle he has carried since the day she left. They speak for the first time.",
        "Rain thins to drizzle. Arjun stands at the gate, unwilling to come in. Savitri comes out to meet him under a shared umbrella of banana leaf.",
    ),
    (
        "INT. MEERA'S APARTMENT - MORNING",
        "In the city, Meera finds her mother's empty bed and the radio still warm. She calls and calls.",
        "A modern flat, a made bed. MEERA (30s) holds the phone, listening to it ring in a drawer across the room.",
    ),
    (
        "INT. POST OFFICE - DAY",
        "The postmaster sorts a bundle of old letters that have just been delivered: forty years late, all in one morning.",
        "A dusty counter. The POSTMASTER (60s) turns an envelope over twice. The address is in his own handwriting from decades ago.",
    ),
    (
        "EXT. POST OFFICE STEPS - DAY",
        "Meera reads the first letter on the steps in the sun. She laughs, then stops laughing.",
        "Bright after the rain. Meera sits on the steps with the letters in her lap, reading.",
    ),
    (
        "INT/EXT. SAVITRI'S KITCHEN - DAWN (RETURN)",
        "Savitri is home before anyone wakes. The radio is on. Nothing in the kitchen has changed except the tiffin, which is lighter.",
        "Same kitchen, same dawn, different light. Savitri puts the empty tiffin on the shelf and turns the radio up.",
    ),
    (
        "EXT. GUDUR RAILWAY STATION - SUNSET",
        "Arjun hands in his uniform. On the platform a stranger asks him the time and he tells her the exact minute.",
        "Golden platform light. Arjun folds the khaki uniform, hands it across a counter, and walks out without his whistle.",
    ),
];

const CAST: [(&str, &str, &str); 6] = [
    ("savitri", "Savitri", "Lakshmi Devi"),
    ("arjun", "Arjun", "Karthik Menon"),
    ("meera", "Meera", "Divya Nair"),
    ("pinky", "Pinky", "Aarna Shah"),
    ("postmaster", "The Postmaster", "R. Venkat"),
    ("teaseller", "The Tea-Seller", "Anil Kumar"),
];

const LOCATIONS: [(&str, &str, &str); 6] = [
    (
        "l_house",
        "Savitri's House",
        "14 Temple Street, Gudur 524101",
    ),
    ("l_stand", "Gudur Bus Stand", "Main Road, Gudur 524101"),
    (
        "l_highway",
        "Nellore Road Causeway",
        "NH 16, 22 km south of Gudur",
    ),
    (
        "l_bungalow",
        "Kota Dak Bungalow",
        "Old Forest Road, Kota 524410",
    ),
    ("l_post", "Old Town Post Office", "Market Square, Old Town"),
    (
        "l_flat",
        "Meera's Apartment",
        "Block C, Lakeview Residency, Hyderabad",
    ),
];

/// (id, category, name, description, status, character or location link)
type CatalogSeed = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
);

const CATALOG: [CatalogSeed; 24] = [
    (
        "c_savitri",
        "Cast",
        "Savitri",
        "Lead, late 60s. Quiet, exact, stubborn.",
        "Confirmed",
        "savitri",
    ),
    (
        "c_arjun",
        "Cast",
        "Arjun",
        "Conductor, early 60s.",
        "Confirmed",
        "arjun",
    ),
    (
        "c_meera",
        "Cast",
        "Meera",
        "Daughter, mid 30s.",
        "Confirmed",
        "meera",
    ),
    ("c_pinky", "Cast", "Pinky", "", "Shortlisted", "pinky"),
    (
        "c_postmaster",
        "Cast",
        "The Postmaster",
        "",
        "Confirmed",
        "postmaster",
    ),
    (
        "c_teaseller",
        "Cast",
        "The Tea-Seller",
        "",
        "Searching",
        "teaseller",
    ),
    (
        "c_house",
        "Location / Set",
        "Savitri's House",
        "Kitchen set with a working stove.",
        "Confirmed",
        "l_house",
    ),
    (
        "c_stand",
        "Location / Set",
        "Gudur Bus Stand",
        "",
        "Confirmed",
        "l_stand",
    ),
    (
        "c_highway",
        "Location / Set",
        "Nellore Road Causeway",
        "Needs water safety plan.",
        "Shortlisted",
        "l_highway",
    ),
    (
        "c_bungalow",
        "Location / Set",
        "Kota Dak Bungalow",
        "Two nights, generator needed.",
        "Confirmed",
        "l_bungalow",
    ),
    (
        "c_tiffin",
        "Props",
        "Steel tiffin carrier",
        "Dented, three tiers.",
        "Required",
        "",
    ),
    (
        "c_letters",
        "Props",
        "Bundle of letters",
        "Forty years of replies, jute twine. Needs 30 hero letters and 80 background.",
        "Required",
        "",
    ),
    (
        "c_radio",
        "Props",
        "Transistor radio",
        "Working, battery powered.",
        "Confirmed",
        "",
    ),
    (
        "c_whistle",
        "Props",
        "Conductor's whistle",
        "",
        "Searching",
        "",
    ),
    ("c_lamp", "Props", "Cracked blue lamp", "", "Required", ""),
    (
        "c_umbrella",
        "Props",
        "Banana leaf umbrella",
        "",
        "Required",
        "",
    ),
    (
        "c_saree",
        "Wardrobe",
        "Savitri's grey cotton saree",
        "Three identical copies for rain and wet scenes.",
        "Confirmed",
        "",
    ),
    (
        "c_uniform",
        "Wardrobe",
        "Khaki conductor's uniform",
        "",
        "Required",
        "",
    ),
    (
        "c_bus",
        "Vehicles",
        "1998 state transport bus",
        "Hired with driver, two days.",
        "Confirmed",
        "",
    ),
    (
        "c_rain",
        "Special Effects",
        "Rain rig",
        "Hose rig and rain towers for the bus and veranda.",
        "Required",
        "",
    ),
    (
        "c_flood",
        "VFX",
        "Flood extension",
        "Extend the water level on the causeway plates.",
        "Required",
        "",
    ),
    (
        "c_radiosound",
        "Sound",
        "Radio flood warning",
        "Recorded bulletin, 40 seconds.",
        "Required",
        "",
    ),
    (
        "c_rainsound",
        "Sound",
        "Rain on tin roof",
        "",
        "Required",
        "",
    ),
    (
        "c_crows",
        "Animals",
        "Crows",
        "Wild, ambient only.",
        "Not Required",
        "",
    ),
];

/// Which catalog items each scene uses: (catalog id, state, note).
fn breakdown_plan() -> Vec<Vec<(&'static str, &'static str, &'static str)>> {
    let c = "Confirmed";
    let s = "Suggested";
    let m = "Manual";
    vec![
        vec![
            ("c_savitri", c, ""),
            ("c_house", c, ""),
            ("c_tiffin", c, "Packed with two rotis before the take"),
            ("c_letters", c, ""),
            ("c_radio", c, "Practical, must play the bulletin"),
            ("c_saree", c, ""),
            ("c_radiosound", c, ""),
            ("c_rainsound", s, ""),
        ],
        vec![
            ("c_savitri", c, ""),
            ("c_arjun", c, ""),
            ("c_stand", c, ""),
            ("c_uniform", c, ""),
            ("c_whistle", m, "Visible on his strap, never blown"),
            ("c_tiffin", c, ""),
            ("c_crows", s, ""),
        ],
        vec![
            ("c_savitri", c, ""),
            ("c_arjun", c, ""),
            ("c_pinky", c, ""),
            ("c_bus", c, "Interior unit with rain towers on the roof"),
            ("c_letters", c, ""),
            ("c_rain", c, ""),
            ("c_rainsound", c, ""),
        ],
        vec![
            ("c_savitri", c, ""),
            ("c_arjun", c, ""),
            ("c_pinky", c, ""),
            ("c_highway", c, ""),
            ("c_bus", c, ""),
            (
                "c_flood",
                c,
                "Plate shot at 40 metres, sky replacement later",
            ),
            ("c_rain", c, ""),
        ],
        vec![
            ("c_savitri", c, ""),
            ("c_teaseller", c, ""),
            ("c_pinky", s, ""),
            ("c_rainsound", c, ""),
        ],
        vec![("c_savitri", c, ""), ("c_flood", s, ""), ("c_rain", c, "")],
        vec![
            ("c_savitri", c, ""),
            ("c_bungalow", c, ""),
            ("c_lamp", c, "Must light on camera"),
            ("c_rain", c, ""),
        ],
        vec![
            ("c_savitri", c, ""),
            ("c_bungalow", c, ""),
            ("c_letters", c, "Hero bundle plus 80 background letters"),
            ("c_lamp", c, ""),
        ],
        vec![
            ("c_savitri", c, ""),
            ("c_arjun", c, ""),
            ("c_bungalow", c, ""),
            ("c_umbrella", c, ""),
            ("c_whistle", c, ""),
            ("c_rain", s, ""),
        ],
        vec![("c_meera", c, "")],
        vec![("c_postmaster", c, ""), ("c_letters", c, "")],
        vec![("c_meera", c, ""), ("c_letters", c, "")],
        vec![
            ("c_savitri", c, ""),
            ("c_house", c, ""),
            ("c_tiffin", c, ""),
            ("c_radio", c, ""),
            ("c_radiosound", c, ""),
        ],
        vec![
            ("c_arjun", c, ""),
            ("c_uniform", c, ""),
            ("c_whistle", c, ""),
        ],
    ]
}

pub(crate) struct Film {
    pub env: TestEnv,
    pub schedule: String,
    /// Shooting day ids in order (shoot days and off days).
    pub days: Vec<String>,
}

pub(crate) fn strip_id(env: &TestEnv, number: i64) -> String {
    let v = env.ok("schedule.get", json!({}));
    let mut all: Vec<Value> = v["unscheduled"].as_array().unwrap().clone();
    for d in v["days"].as_array().unwrap() {
        for it in d["items"].as_array().unwrap() {
            if it["kind"] == "strip" {
                all.push(it["strip"].clone());
            }
        }
    }
    all.iter().find(|s| s["number"] == number).expect("strip")["id"]
        .as_str()
        .unwrap()
        .to_string()
}

pub(crate) fn build_film() -> Film {
    build_film_named("Monsoon Letters")
}

fn build_film_named(title: &str) -> Film {
    let env = TestEnv::with_project(title, "Short Film");
    let mut sql = String::new();
    sql.push_str("INSERT INTO screenplay(id,title,format,created_at,updated_at) VALUES ('sp','Monsoon Letters','Short',1,1);");
    sql.push_str("INSERT INTO screenplay_draft(id,screenplay_id,name,status,created_at,updated_at) VALUES ('d1','sp','Shooting Draft 3','Locked',1,1);");
    for (i, (heading, synopsis, action)) in SCENES.iter().enumerate() {
        let n = i + 1;
        sql.push_str(&format!(
            "INSERT INTO screenplay_scene(id,draft_id,lineage_id,position,heading,synopsis,created_at,updated_at) VALUES ('s{n}','d1','L{n}',{n},{},{},1,1);",
            q(heading),
            q(synopsis)
        ));
        sql.push_str(&format!(
            "INSERT INTO screenplay_element(id,scene_id,position,element_type,text,created_at,updated_at) VALUES ('e{n}a','s{n}',1,'action',{},1,1);",
            q(&format!("{action} {action}"))
        ));
        if n % 2 == 1 {
            sql.push_str(&format!(
                "INSERT INTO screenplay_element(id,scene_id,position,element_type,text,created_at,updated_at) VALUES ('e{n}b','s{n}',2,'character','SAVITRI',1,1),('e{n}c','s{n}',3,'dialogue',{},1,1);",
                q("You wrote that the river was kind in July. It is not kind in July. It is not kind in any month. Why did you stop writing?")
            ));
        }
    }
    for (i, (id, name, _)) in CAST.iter().enumerate() {
        sql.push_str(&format!(
            "INSERT INTO story_character(id,name,position,created_at,updated_at) VALUES ('{id}',{},{},1,1);",
            q(name),
            i + 1
        ));
    }
    for (id, name, actor) in CAST.iter() {
        sql.push_str(&format!(
            "INSERT INTO cast_member(id,person_name,character_id,created_at,updated_at) VALUES ('cm_{id}',{},'{id}',1,1);",
            q(if *id == "teaseller" { "" } else { actor }),
        ));
        let _ = name;
    }
    for (id, name, address) in LOCATIONS {
        sql.push_str(&format!(
            "INSERT INTO location(id,name,address,status,created_at,updated_at) VALUES ('{id}',{},{},'Confirmed',1,1);",
            q(name),
            q(address)
        ));
    }
    for (id, cat, name, desc, status, link) in CATALOG {
        let (ch, loc) = if link.starts_with("l_") {
            ("NULL".to_string(), q(link))
        } else if link.is_empty() {
            ("NULL".to_string(), "NULL".to_string())
        } else {
            (q(link), "NULL".to_string())
        };
        sql.push_str(&format!(
            "INSERT INTO catalog_item(id,category,name,description,status,character_id,location_id,created_at,updated_at) VALUES ('{id}',{},{},{},{},{ch},{loc},1,1);",
            q(cat),
            q(name),
            if desc.is_empty() { "NULL".to_string() } else { q(desc) },
            q(status)
        ));
    }
    sql.push_str("INSERT INTO production_source(id,draft_id,active,selected_at,created_at,updated_at) VALUES ('ps1','d1',1,1,1,1);");
    let cat_of = |id: &str| CATALOG.iter().find(|c| c.0 == id).unwrap();
    let mut k = 0;
    for (i, items) in breakdown_plan().iter().enumerate() {
        let n = i + 1;
        for (cid, state, note) in items {
            k += 1;
            let c = cat_of(cid);
            sql.push_str(&format!(
                "INSERT INTO breakdown_element(id,source_id,scene_id,scene_lineage_id,category,catalog_item_id,display_name,confirmation_state,notes,created_at,updated_at) VALUES ('b{k}','ps1','s{n}','L{n}',{},'{cid}',{},{},{},{k},{k});",
                q(c.1),
                q(c.2),
                q(state),
                if note.is_empty() { "NULL".to_string() } else { q(note) }
            ));
        }
    }
    for (i, (name, role, dept, contact)) in [
        ("Asha Rao", "Director", "Direction", "98100 11001"),
        (
            "Rohan Iyer",
            "1st Assistant Director",
            "Direction",
            "98100 11002",
        ),
        (
            "Neha Kapoor",
            "Director of Photography",
            "Camera",
            "98100 11003",
        ),
        ("Imran Qureshi", "Focus Puller", "Camera", "98100 11004"),
        ("Tara Menon", "Production Designer", "Art", "98100 11005"),
        ("Vikram Das", "Sound Recordist", "Sound", "98100 11006"),
        (
            "Leela George",
            "Costume Designer",
            "Wardrobe",
            "98100 11007",
        ),
        (
            "Joseph Paul",
            "Production Manager",
            "Production",
            "98100 11008",
        ),
    ]
    .iter()
    .enumerate()
    {
        exec(
            &env,
            &format!(
                "INSERT INTO crew_member(id,person_name,role,department,contact,created_at,updated_at) VALUES ('cr{i}',{},{},{},{},1,1);",
                q(name),
                q(role),
                q(dept),
                q(contact)
            ),
        );
    }
    exec(&env, &sql);

    // Schedule: three shooting days, an off day and a fourth shooting day.
    let sched = id_of(&env.ok("schedule.create", json!({})));
    let mut days = Vec::new();
    let plan: [(&str, bool); 5] = [
        ("2027-06-14", false),
        ("2027-06-15", false),
        ("2027-06-16", true),
        ("2027-06-17", false),
        ("2027-06-18", false),
    ];
    for (date, off) in plan {
        days.push(id_of(&env.ok(
            "schedule.create_day",
            json!({ "scheduleId": sched, "date": date, "offDay": off }),
        )));
    }
    // Scenes per day, in the order the production wants them (not script order).
    let assign: [(usize, &[i64]); 4] = [
        (0, &[2, 3, 4, 1]),
        (1, &[5, 6, 7, 8]),
        (3, &[9, 10, 11]),
        (4, &[12, 13, 14]),
    ];
    for (di, scenes) in assign {
        for n in scenes {
            let s = strip_id(&env, *n);
            env.ok(
                "schedule.move_strip",
                json!({ "stripId": s, "dayId": days[di] }),
            );
            if n % 4 != 0 {
                env.ok(
                    "schedule.set_strip_estimate",
                    json!({ "stripId": s, "minutes": 45 + 15 * (n % 5) }),
                );
            }
        }
    }
    env.ok(
        "schedule.add_marker",
        json!({ "dayId": days[0], "markerType": "Meal", "durationMinutes": 45, "index": 2 }),
    );
    env.ok(
        "schedule.add_marker",
        json!({ "dayId": days[0], "markerType": "Company Move", "label": "Move from the bus stand to the causeway", "durationMinutes": 40, "index": 3, "notes": "Two vans, causeway permit in the production office" }),
    );
    env.ok(
        "schedule.add_marker",
        json!({ "dayId": days[1], "markerType": "Meal", "label": "Lunch at the tea stall", "durationMinutes": 40, "index": 2 }),
    );
    for (di, call) in [(0usize, 360i64), (1, 390), (3, 420), (4, 420)] {
        env.ok(
            "schedule.set_day_times",
            json!({ "dayId": days[di], "crewCall": call, "shootingStart": call + 60, "targetWrap": 18 * 60 }),
        );
    }
    Film {
        env,
        schedule: sched,
        days,
    }
}

// ------------------------------------------------------------------ samples

fn export(env: &TestEnv, op: &str, mut args: Value, file: &str) {
    args["format"] = json!("pdf");
    args["path"] = json!(target(file));
    let r = env.ok(op, args);
    println!("{file}: {} pages", r["pages"]);
}

#[test]
#[ignore = "writes the sample PDFs to release-out/pdf-samples"]
fn write_call_sheet_samples() {
    let film = build_film();
    let env = &film.env;

    // 1. A populated, final call sheet with every optional section.
    let cs = id_of(&env.ok("callsheets.create", json!({ "dayId": film.days[0] })));
    for section in [
        "weather",
        "extras",
        "departments",
        "advance",
        "crew",
        "communication",
        "specialNotes",
        "referenceImages",
        "attachments",
    ] {
        env.ok(
            "callsheets.add_section",
            json!({ "id": cs, "section": section }),
        );
    }
    for (i, theme) in [0usize, 2, 4].iter().enumerate() {
        let p = env.write_file(&format!("ref{i}.png"), &still(*theme, 480, 320));
        env.ok(
            "callsheets.attach",
            json!({ "id": cs, "section": "referenceImages", "path": p }),
        );
    }
    let notes = env.write_file("Location permit.pdf", b"%PDF-1.4 permit");
    env.ok(
        "callsheets.attach",
        json!({ "id": cs, "section": "attachments", "path": notes }),
    );
    let mut doc = env.ok("callsheets.get", json!({ "id": cs }))["document"].clone();
    doc["header"] = json!({
        "company": "Northlight Pictures",
        "address": "14 Example Road, Pune 411001",
        "producer": "Joseph Paul",
        "director": "Asha Rao",
        "firstAd": "Rohan Iyer",
        "contact": "Production office, 98100 11008"
    });
    doc["times"]["generalCall"] = json!("06:30");
    doc["times"]["breakfast"] = json!("06:00");
    doc["times"]["lunch"] = json!("13:00");
    doc["times"]["estimatedWrap"] = json!("18:30");
    doc["dayNotes"] = json!(
        "Heavy rain is expected and welcome. Keep the rain rig on standby, protect the camera package, \
         and do not start the causeway scene until the water safety lead has signed off the depth marks."
    );
    for (i, (actor, call, status)) in [
        ("Lakshmi Devi", "06:30", "SW"),
        ("Karthik Menon", "06:30", "SW"),
        ("Aarna Shah", "07:30", "W"),
        ("R. Venkat", "08:00", "W"),
    ]
    .iter()
    .enumerate()
    {
        if let Some(c) = doc["cast"].get_mut(i) {
            c["actor"] = json!(actor);
            c["callTime"] = json!(call);
            c["status"] = json!(status);
            c["makeup"] = json!(format!("05:{:02}", 30 + i * 10));
            c["contact"] = json!(format!("98200 3300{i}"));
        }
    }
    for l in doc["locations"].as_array_mut().unwrap() {
        l["hospital"] = json!("Government District Hospital, Gudur (14 minutes)");
        l["emergencyContact"] = json!("Rohan Iyer, 1st AD, 98100 11002");
        l["parking"] = json!("Unit base at the bus stand yard, 20 vehicles");
        l["meetingPoint"] = json!("Chai stall at the east gate");
    }
    doc["practical"]["emergencyContact"] = json!("Joseph Paul, Production Manager, 98100 11008");
    doc["practical"]["safety"] = json!(
        "Life jackets for everyone within 10 metres of the water. Medic on set from first call. \
         No one crosses the causeway without a buddy."
    );
    doc["practical"]["mealBreak"] = json!("Breakfast 06:00, lunch 13:00 at the tea stall");
    doc["practical"]["travelNotes"] = json!("Unit vans leave the base at 05:40 sharp");
    doc["optional"]["weather"] =
        json!("Heavy rain from 09:00, clearing by 16:00. Wind gusts to 40 km/h.");
    doc["optional"]["sunrise"] = json!("05:48");
    doc["optional"]["sunset"] = json!("18:52");
    doc["optional"]["specialNotes"] = json!(
        "Child actor on set: tutor and guardian present at all times, working hours capped. \
         The conductor's whistle is a hero prop and stays with the props master between takes."
    );
    doc["optional"]["communication"]["walkieChannels"] =
        json!("1 Production, 2 Camera, 3 Art, 4 Safety");
    if let Some(list) = doc["optional"]["crew"].as_array_mut() {
        for (i, c) in list.iter_mut().enumerate() {
            c["callTime"] = json!(if i % 2 == 0 { "06:00" } else { "06:30" });
            c["contact"] = json!(format!("98100 110{:02}", i + 1));
        }
    }
    env.ok("callsheets.update", json!({ "id": cs, "document": doc }));
    env.ok(
        "callsheets.finalize",
        json!({ "id": cs, "acknowledgeStale": true }),
    );
    export(
        env,
        "callsheets.export_pdf",
        json!({ "id": cs }),
        "01-call-sheet-final-full.pdf",
    );

    // 2. A fresh draft with nothing entered yet: every missing state shows.
    let draft = id_of(&env.ok("callsheets.create", json!({ "dayId": film.days[1] })));
    export(
        env,
        "callsheets.export_pdf",
        json!({ "id": draft }),
        "02-call-sheet-draft-incomplete.pdf",
    );
}

#[test]
#[ignore = "writes the sample PDFs to release-out/pdf-samples"]
fn write_schedule_samples() {
    let film = build_film();
    let env = &film.env;
    // The unscheduled pool is real data: leave scenes 13 and 14 out of the schedule.
    for n in [13, 14] {
        let s = strip_id(env, n);
        env.ok(
            "schedule.move_strip",
            json!({ "stripId": s, "dayId": Value::Null }),
        );
    }
    export(
        env,
        "schedule.export_schedule",
        json!({ "includeNotes": true }),
        "03-shooting-schedule.pdf",
    );
    export(
        env,
        "schedule.export_schedule",
        json!({ "dayIds": [film.days[0]] }),
        "04-shooting-schedule-one-day.pdf",
    );
}

#[test]
#[ignore = "writes the sample PDFs to release-out/pdf-samples"]
fn write_dood_samples() {
    let film = build_film();
    let env = &film.env;
    // A short schedule fits one band.
    export(
        env,
        "schedule.export_dood",
        json!({}),
        "06-day-out-of-days-short.pdf",
    );

    // A long schedule: 13 shooting days and 2 off days, scenes spread one or two a day.
    let mut days: Vec<String> = film.days.clone();
    for i in 0..9 {
        let date = format!("2027-06-{}", 19 + i);
        days.push(id_of(&env.ok(
            "schedule.create_day",
            json!({ "scheduleId": film.schedule, "date": date }),
        )));
    }
    let off2 = id_of(&env.ok(
        "schedule.create_day",
        json!({ "scheduleId": film.schedule, "offDay": true, "afterDayId": days[5] }),
    ));
    let shoot: Vec<&String> = days
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != 2)
        .map(|(_, d)| d)
        .collect();
    for n in 1..=14i64 {
        let s = strip_id(env, n);
        let target = shoot[((n - 1) as usize * shoot.len()) / 14];
        env.ok(
            "schedule.move_strip",
            json!({ "stripId": s, "dayId": target }),
        );
    }
    let v = env.ok("schedule.dood", json!({}));
    let first_cast = v["rows"][0]["key"].as_str().unwrap().to_string();
    let a_day = v["days"][1]["id"].as_str().unwrap().to_string();
    env.ok(
        "schedule.set_dood_override",
        json!({ "scheduleId": film.schedule, "castKey": first_cast, "dayId": a_day, "code": "H",
                "note": "Held for the causeway reshoot, paid hold day" }),
    );
    let _ = off2;
    export(
        env,
        "schedule.export_dood",
        json!({}),
        "05-day-out-of-days-long.pdf",
    );
}

#[test]
#[ignore = "writes the sample PDFs to release-out/pdf-samples"]
fn write_breakdown_samples() {
    let film = build_film();
    let env = &film.env;
    export(
        env,
        "breakdown.export_report",
        json!({ "content": "both", "includeNotes": true }),
        "07-breakdown-and-catalog.pdf",
    );
    export(
        env,
        "breakdown.export_report",
        json!({ "content": "catalog" }),
        "08-catalog.pdf",
    );
    export(
        env,
        "breakdown.export_report",
        json!({ "content": "summary" }),
        "09-breakdown-summary.pdf",
    );
}

#[test]
#[ignore = "writes the sample PDFs to release-out/pdf-samples"]
fn write_visual_samples() {
    let film = build_film();
    let env = &film.env;

    // Moodboard with images, captions, notes, a link and internal items.
    let board = id_of(&env.ok("moodboard.create", json!({ "name": "Overall Look" })));
    for (i, caption) in [
        "Low sun through monsoon haze",
        "Wet tin roofs, cool grey",
        "Night interiors lit by one lamp",
        "The bus as a moving room",
        "Faded blue and rust",
        "River colour: tea, not water",
        "Warm kitchen, early light",
    ]
    .iter()
    .enumerate()
    {
        let p = env.write_file(&format!("mood{i}.png"), &still(i, 600, 430));
        let imgs = env.ok(
            "moodboard.add_images",
            json!({ "moodboardId": board, "paths": [p], "x": 20 + 30 * i as i64, "y": 24 + 60 * i as i64 }),
        );
        env.ok(
            "moodboard.update_item",
            json!({ "id": id_of(&imgs[0]), "caption": caption }),
        );
    }
    env.ok(
        "moodboard.add_note",
        json!({ "moodboardId": board, "text": "Cold, wet, blue-grey, with one warm window in every scene. Faces always lit from the side the weather comes from." }),
    );
    env.ok(
        "moodboard.add_note",
        json!({ "moodboardId": board, "text": "Budget: rain machine hire is the biggest single risk.", "isPrivate": true }),
    );
    env.ok(
        "moodboard.add_link",
        json!({ "moodboardId": board, "url": "www.example.com/monsoon-photography", "title": "Monsoon photography reference" }),
    );
    env.ok(
        "moodboard.update_notes",
        json!({ "id": board, "notes": "Keep the palette to four colours per scene. Check every costume against the bus seat fabric." }),
    );
    export(
        env,
        "moodboard.export_pdf",
        json!({ "moodboardId": board, "includeInternalNotes": true }),
        "10-moodboard.pdf",
    );

    // Storyboard: scene 3, six panels, shots linked to some of them.
    let sb = id_of(&env.ok("storyboard.create", json!({ "sceneId": "s3" })));
    let panels = [
        (
            "Wide",
            "Static",
            "Eye level",
            "The bus climbs out of the town in heavy rain",
            "Rain on the roof, engine labouring",
            4000,
        ),
        (
            "Medium",
            "Pan",
            "Eye level",
            "Savitri unfolds the yellowed letter",
            "",
            3500,
        ),
        (
            "Close up",
            "Static",
            "Low",
            "Her thumb on the fold of the paper",
            "Paper, a long breath",
            2500,
        ),
        (
            "Over the shoulder",
            "Handheld",
            "Eye level",
            "Pinky draws on the fogged window behind her",
            "",
            3000,
        ),
        (
            "Medium",
            "Static",
            "High",
            "Arjun at the front, not turning round",
            "Whistle swinging on its strap",
            3500,
        ),
        (
            "Wide",
            "Dolly",
            "Eye level",
            "The aisle, everyone leaning with the bend in the road",
            "",
            5000,
        ),
    ];
    for (i, (size, movement, angle, desc, sound, ms)) in panels.iter().enumerate() {
        let shot = id_of(&env.ok(
            "shot.create",
            json!({ "sceneId": "s3", "description": desc, "size": size, "movement": movement, "angle": angle,
                    "lens": if i % 2 == 0 { "35mm" } else { "50mm" },
                    "characters": ["SAVITRI"],
                    "soundNote": sound, "cameraNotes": if i == 1 { "Rack to the letter on the pan" } else { "" } }),
        ));
        let p = env.write_file(&format!("panel{i}.png"), &still(i + 1, 640, 360));
        let panel = id_of(&env.ok(
            "storyboard.add_panel",
            json!({ "storyboardId": sb, "visual": if i == 5 { "placeholder" } else { "image" }, "path": p, "description": desc }),
        ));
        env.ok(
            "storyboard.update_panel",
            json!({ "id": panel, "description": desc, "framing": size, "movement": movement, "angle": angle,
                    "soundNote": sound, "durationMs": ms, "note": "Hold two beats before the cut" }),
        );
        env.ok(
            "storyboard.link_shot",
            json!({ "panelId": panel, "shotId": shot }),
        );
    }
    export(
        env,
        "storyboard.export_sheet",
        json!({ "storyboardIds": [sb], "includeNotes": true }),
        "11-storyboard.pdf",
    );

    // Shot list: scene 3 (with images from the storyboard) and a second scene without pictures.
    for (desc, size) in [
        ("Establishing: the empty bus stand at first light", "Wide"),
        ("Arjun punches the ticket and looks up", "Close up"),
    ] {
        env.ok(
            "shot.create",
            json!({ "sceneId": "s2", "description": desc, "size": size, "characters": ["ARJUN"] }),
        );
    }
    export(env, "shot.export_list", json!({}), "12-shot-list.pdf");
}

#[test]
#[ignore = "writes the sample PDFs to release-out/pdf-samples"]
fn write_report_and_budget_samples() {
    let film = build_film();
    let env = &film.env;
    // Leave one scene out of the schedule and one with no breakdown so the statuses vary.
    let s = strip_id(env, 14);
    env.ok(
        "schedule.move_strip",
        json!({ "stripId": s, "dayId": Value::Null }),
    );
    for (i, kind) in [
        "scene",
        "location",
        "cast_scene",
        "prop",
        "schedule",
        "breakdown_completeness",
    ]
    .iter()
    .enumerate()
    {
        export(
            env,
            "reports.export_report",
            json!({ "reportType": kind }),
            &format!("{:02}-report-{}.pdf", 13 + i, kind.replace('_', "-")),
        );
    }

    let budget = id_of(&env.ok("budget.create", json!({ "currency": "INR" })));
    env.ok(
        "budget.update",
        json!({ "id": budget, "currency": "INR", "plannedTotal": 450_000_000i64,
                "contingencyMode": "percent", "contingencyValue": 1000,
                "notes": "Rain days are the main risk: the contingency is held for two extra shooting days and the rain rig, not for post-production." }),
    );
    let lines: [(&str, &str, i64, Option<i64>, &str); 18] = [
        (
            "Cast",
            "Lead actor, Savitri (4 days)",
            40_000_000,
            Some(40_000_000),
            "",
        ),
        (
            "Cast",
            "Arjun and supporting cast",
            36_000_000,
            Some(39_500_000),
            "Two extra hold days",
        ),
        (
            "Cast",
            "Child actor, tutor and guardian",
            8_000_000,
            None,
            "",
        ),
        (
            "Crew",
            "Director of photography",
            30_000_000,
            Some(30_000_000),
            "",
        ),
        (
            "Crew",
            "Camera and sound crew (4 days)",
            62_000_000,
            Some(58_000_000),
            "Rates agreed at the day-rate card",
        ),
        (
            "Crew",
            "Production designer and art team",
            24_000_000,
            None,
            "",
        ),
        (
            "Locations",
            "Dak bungalow, two nights",
            18_000_000,
            Some(20_000_000),
            "Generator hire included",
        ),
        (
            "Locations",
            "Bus stand and causeway permits",
            9_500_000,
            Some(9_500_000),
            "",
        ),
        (
            "Equipment",
            "Camera, lenses and grip package",
            45_000_000,
            Some(44_000_000),
            "",
        ),
        (
            "Equipment",
            "Rain rig and water safety",
            28_000_000,
            None,
            "Quote pending from the supplier",
        ),
        (
            "Art/Props",
            "Letters, tiffin and period props",
            7_500_000,
            Some(6_800_000),
            "",
        ),
        (
            "Travel/Transport",
            "State bus hire with driver, two days",
            22_000_000,
            Some(22_000_000),
            "",
        ),
        (
            "Travel/Transport",
            "Unit vans and fuel",
            11_000_000,
            None,
            "",
        ),
        (
            "Food",
            "Unit catering, four shooting days",
            16_000_000,
            Some(15_200_000),
            "",
        ),
        (
            "Post/Other",
            "Edit, grade and sound mix",
            52_000_000,
            None,
            "Fixed price from the post house",
        ),
        ("Post/Other", "Insurance", 9_000_000, Some(9_000_000), ""),
        ("Post/Other", "Festival submissions", 4_000_000, None, ""),
        ("Contingency", "Held for rain days", 20_000_000, None, ""),
    ];
    for (cat, desc, planned, actual, note) in lines {
        env.ok(
            "budget.add_line",
            json!({ "budgetId": budget, "category": cat, "description": desc, "planned": planned,
                    "actual": actual, "notes": note }),
        );
    }
    export(
        env,
        "budget.export_summary",
        json!({ "includeNotes": true }),
        "19-budget.pdf",
    );
}

#[test]
#[ignore = "writes the sample PDFs to release-out/pdf-samples"]
fn write_sides_samples() {
    let film = build_film();
    let env = &film.env;
    export(
        env,
        "sides.export_pdf",
        json!({ "dayId": film.days[0], "includeCover": true }),
        "22-sides-day-1.pdf",
    );
    // A very long project name: the running header wraps at its separators.
    let long = build_film_named(
        "The Extraordinarily Long And Winding Journey Of Savitri Across The Flooded River Districts",
    );
    export(
        &long.env,
        "sides.export_pdf",
        json!({ "dayId": long.days[1] }),
        "23-sides-long-title.pdf",
    );
}
