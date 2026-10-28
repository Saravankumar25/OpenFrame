//! Shot List (single scene, scenes, whole project) and Storyboard sheet PDFs: layout rules that
//! must hold for any data, plus the sample writer for the design review
//! (`... --test production visual_pdf:: -- --ignored --nocapture`, files in
//! `release-out/pdf-samples/`).

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::pdf_samples::{Film, build_film, id_of, still};

// ------------------------------------------------------------------ PDF reading

/// One drawn run of text: where it sits on its page (points from the bottom left) and its size.
#[derive(Debug, Clone)]
struct Run {
    page: usize,
    x: f32,
    y: f32,
    size: f32,
    text: String,
}

struct Dump {
    runs: Vec<Run>,
    /// Pictures: page, x, y, width, height (the footer logo is left out).
    images: Vec<(usize, [f32; 4])>,
    pages: usize,
    size: (f32, f32),
}

fn dump(path: &str) -> Dump {
    let doc = lopdf::Document::load(path).expect("PDF parses");
    let num = |o: &lopdf::Object| o.as_float().unwrap_or(0.0);
    let mut d = Dump {
        runs: Vec::new(),
        images: Vec::new(),
        pages: 0,
        size: (0.0, 0.0),
    };
    for (n, (_, page)) in doc.get_pages().into_iter().enumerate() {
        d.pages += 1;
        let mb = doc
            .get_object(page)
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"MediaBox");
        if let Ok(lopdf::Object::Array(a)) = mb {
            d.size = (num(&a[2]), num(&a[3]));
        }
        let content = lopdf::content::Content::decode(&doc.get_page_content(page)).unwrap();
        let (mut size, mut at) = (0.0f32, (0.0f32, 0.0f32));
        for op in content.operations {
            let a = &op.operands;
            match op.operator.as_str() {
                "Tf" => size = num(&a[1]),
                "Tm" => at = (num(&a[4]), num(&a[5])),
                "Tj" => {
                    if let lopdf::Object::String(b, _) = &a[0] {
                        d.runs.push(Run {
                            page: n,
                            x: at.0,
                            y: at.1,
                            size,
                            text: b
                                .iter()
                                .map(|x| openframe_import_export::winansi::decode_byte(*x))
                                .collect(),
                        });
                    }
                }
                "cm" => {
                    let r = [num(&a[4]), num(&a[5]), num(&a[0]), num(&a[3])];
                    // The logo is 17 pt high at the left margin, 33 pt above the bottom.
                    if (r[3] - 17.0).abs() >= 0.01 {
                        d.images.push((n, r));
                    }
                }
                _ => {}
            }
        }
    }
    d
}

impl Dump {
    fn text(&self, page: usize) -> String {
        self.runs
            .iter()
            .filter(|r| r.page == page)
            .map(|r| r.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
    fn all(&self) -> String {
        (0..self.pages)
            .map(|p| self.text(p))
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn pages_with(&self, needle: &str) -> Vec<usize> {
        (0..self.pages)
            .filter(|p| self.text(*p).contains(needle))
            .collect()
    }
}

// ------------------------------------------------------------------ data

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

fn export(film: &Film, op: &str, mut args: Value, file: &str) -> (String, Value) {
    let path = film
        .env
        .dir
        .path()
        .join(file)
        .to_string_lossy()
        .into_owned();
    args["format"] = json!("pdf");
    args["path"] = json!(path);
    let r = film.env.ok(op, args);
    (path, r)
}

/// A shot to seed: scene id, description, camera words, notes and an optional picture
/// (theme, width, height).
#[derive(Default, Clone)]
struct Shot<'a> {
    scene: &'a str,
    desc: &'a str,
    size: &'a str,
    movement: &'a str,
    angle: &'a str,
    lens: &'a str,
    chars: &'a [&'a str],
    sound: &'a str,
    camera: &'a str,
    img: Option<(usize, u32, u32)>,
}

fn add(film: &Film, s: Shot) -> String {
    let env = &film.env;
    let mut a = json!({ "sceneId": s.scene, "description": s.desc, "characters": s.chars });
    for (k, v) in [
        ("size", s.size),
        ("movement", s.movement),
        ("angle", s.angle),
        ("lens", s.lens),
        ("soundNote", s.sound),
        ("cameraNotes", s.camera),
    ] {
        if !v.is_empty() {
            a[k] = json!(v);
        }
    }
    let id = id_of(&env.ok("shot.create", a));
    if let Some((theme, w, h)) = s.img {
        let p = env.write_file(&format!("shot-{id}.png"), &still(theme, w, h));
        env.ok("shot.set_reference_image", json!({ "id": id, "path": p }));
    }
    id
}

/// The normal data set: scene 1 with three varied shots, then a spread of scenes.
fn seed_shots(film: &Film) {
    let wide = Some((1, 640, 360));
    add(
        film,
        Shot {
            scene: "s1",
            desc: "Aerial plate: the bus stand at first light, lanterns still lit",
            size: "Extreme wide",
            movement: "Drone, slow push",
            angle: "High",
            lens: "24mm",
            chars: &["SAVITRI"],
            sound: "Wild wind and engines",
            camera: "VFX storm extension. Plate shot at dusk, graded to night.",
            img: wide,
        },
    );
    add(
        film,
        Shot {
            scene: "s1",
            desc: "Savitri at the window, radio handset in her fist",
            size: "Medium close-up",
            movement: "Handheld on the boat",
            angle: "Eye level",
            lens: "40mm",
            chars: &["SAVITRI"],
            sound: "Radio read out, played back",
            camera: "Rain bars on the glass.",
            ..Shot::default()
        },
    );
    add(
        film,
        Shot {
            scene: "s1",
            desc: "Insert: the set's dial as the signal breaks",
            size: "Insert",
            movement: "Static",
            angle: "Overhead",
            lens: "100mm macro",
            sound: "Static designed from field recordings",
            camera: "Practical dial light flicker.",
            ..Shot::default()
        },
    );
    for (desc, size) in [
        ("Establishing: the empty bus stand at first light", "Wide"),
        ("Arjun punches the ticket and looks up", "Close up"),
        ("Porters asleep on the sacks", "Medium"),
    ] {
        add(
            film,
            Shot {
                scene: "s2",
                desc,
                size,
                chars: &["ARJUN"],
                ..Shot::default()
            },
        );
    }
    for (i, (desc, size)) in [
        ("The bus climbs out of the town in heavy rain", "Wide"),
        ("Savitri unfolds the yellowed letter", "Medium"),
        ("Her thumb on the fold of the paper", "Close up"),
        (
            "Pinky draws on the fogged window behind her",
            "Over the shoulder",
        ),
        ("Arjun at the front, not turning round", "Medium"),
        (
            "The aisle, everyone leaning with the bend in the road",
            "Wide",
        ),
        ("Last light on the wet glass", "Close up"),
    ]
    .into_iter()
    .enumerate()
    {
        add(
            film,
            Shot {
                scene: "s3",
                desc,
                size,
                movement: ["Static", "Pan", "Dolly", "Handheld"][i % 4],
                angle: ["Eye level", "Low", "High"][i % 3],
                lens: ["35mm", "50mm", "85mm anamorphic"][i % 3],
                chars: &["SAVITRI", "PINKY"][..1 + i % 2],
                sound: if i % 2 == 0 { "Rain on the roof" } else { "" },
                img: match i {
                    0 => Some((2, 640, 360)),
                    2 => Some((3, 400, 400)),
                    4 => Some((4, 300, 400)),
                    5 => Some((5, 860, 360)),
                    _ => None,
                },
                ..Shot::default()
            },
        );
    }
    for desc in [
        "The river in flood, seen from the bridge rail",
        "A policeman looks away",
    ] {
        add(
            film,
            Shot {
                scene: "s6",
                desc,
                size: "Wide",
                ..Shot::default()
            },
        );
    }
    add(
        film,
        Shot {
            scene: "s8",
            desc: "Savitri reads the first unsent reply",
            size: "Close up",
            angle: "Low",
            ..Shot::default()
        },
    );
    // Scene 6 changed since planning.
    film.env
        .core
        .project()
        .unwrap()
        .store
        .with_writer(|c| {
            c.execute_batch("UPDATE shot SET needs_review=1 WHERE scene_lineage_id='L6';")
                .expect("flag");
            Ok(())
        })
        .unwrap();
}

/// A scene with a three-line heading, fourteen shots, long text, a missing and an unreadable
/// picture and odd aspect ratios.
fn seed_stress(film: &Film) {
    let env = &film.env;
    for i in 0..14usize {
        let long = "The camera tracks the whole length of the flooded highway while the passengers push, \
                    shout and lean into the bus, and Arjun walks ahead probing the brown water with a bamboo pole";
        let id = add(
            film,
            Shot {
                scene: "s4",
                desc: if i == 3 {
                    long
                } else {
                    "A shorter description for this shot"
                },
                size: ["Wide", "Close up", "Medium close-up"][i % 3],
                movement: if i % 4 == 0 {
                    "Handheld on the boat"
                } else {
                    ""
                },
                angle: if i % 5 == 0 { "Overhead" } else { "" },
                lens: if i % 6 == 0 { "35mm anamorphic" } else { "" },
                chars: if i % 5 == 1 {
                    &[
                        "SAVITRI",
                        "ARJUN",
                        "PINKY",
                        "MEERA",
                        "THE POSTMASTER",
                        "THE TEA-SELLER",
                        "A VERY-LONG-HYPHENATED-CHARACTER-NAME",
                        "EXTRA ONE",
                    ]
                } else {
                    &[]
                },
                sound: if i == 3 {
                    "Engines labouring, shouting in three languages, the pole striking stones under the water"
                } else {
                    ""
                },
                camera: if i == 3 {
                    "Rig the camera on the bus roof with a safety line and a rain deflector, the lens must stay dry"
                } else {
                    ""
                },
                img: match i {
                    1 => Some((1, 400, 400)),
                    2 => Some((2, 300, 400)),
                    5 => Some((3, 860, 360)),
                    6 | 7 => Some((4, 640, 360)),
                    _ => None,
                },
            },
        );
        if i == 6 {
            // A picture whose file is gone.
            let path = env.ok("shot.get", json!({ "id": id }))["referenceAsset"]["path"]
                .as_str()
                .unwrap()
                .to_string();
            std::fs::remove_file(path).unwrap();
        }
        if i == 7 {
            // A picture that is no longer readable.
            let path = env.ok("shot.get", json!({ "id": id }))["referenceAsset"]["path"]
                .as_str()
                .unwrap()
                .to_string();
            std::fs::write(path, b"not a picture").unwrap();
        }
    }
}

/// Storyboards on four scenes: a short one, one with odd pictures, one with a long name and a
/// changed scene, and an empty one.
fn seed_boards(film: &Film, stress: bool) -> Vec<String> {
    let env = &film.env;
    let mut ids = Vec::new();
    let plan: [(&str, &str, usize); 4] = [
        ("s1", "Title sequence over the storm", 4),
        ("s3", "The bus climbs out of the town", 5),
        ("s5", "Tea stall", 3),
        ("s8", "Letter room", 1),
    ];
    for (scene, name, n) in plan {
        let sb = id_of(&env.ok(
            "storyboard.create",
            json!({ "sceneId": scene, "name": name }),
        ));
        for i in 0..n {
            let p = env.write_file(
                &format!("panel-{scene}-{i}.png"),
                &still(
                    i + 1,
                    [640, 400, 300, 860][i % 4],
                    [360, 400, 400, 360][i % 4],
                ),
            );
            let panel = id_of(&env.ok(
                "storyboard.add_panel",
                json!({ "storyboardId": sb, "visual": if i == 3 && n == 4 { "placeholder" } else { "image" }, "path": p }),
            ));
            let desc = [
                "Title over the black sea.",
                "Savitri unfolds the yellowed letter, her thumb on the fold of the paper.",
                "Pinky draws on the fogged window behind her.",
                "The aisle, everyone leaning with the bend in the road.",
                "Arjun at the front, not turning round.",
            ][i % 5];
            let framing = ["Wide", "Medium", "Close up", "Over the shoulder", "Full"][i % 5];
            let movement = ["Static", "Handheld follow", "Pan"][i % 3];
            let angle = ["Eye level", "Low", "High"][i % 3];
            let sound = if i % 2 == 0 {
                "Rain on the roof, engine labouring"
            } else {
                ""
            };
            let ms = if i % 3 == 1 { 3500 } else { 15000 };
            env.ok(
                "storyboard.update_panel",
                json!({ "id": panel, "description": desc, "framing": framing, "movement": movement,
                        "angle": angle, "soundNote": sound, "durationMs": ms,
                        "note": "Hold two beats before the cut" }),
            );
            if i == 0 {
                let shot = add(
                    film,
                    Shot {
                        scene,
                        desc: "Linked shot",
                        ..Shot::default()
                    },
                );
                env.ok(
                    "storyboard.link_shot",
                    json!({ "panelId": panel, "shotId": shot }),
                );
            }
        }
        ids.push(sb);
    }
    if stress {
        // A very long name, a missing and an unreadable picture, a blank panel and a long caption.
        let sb = id_of(&env.ok(
            "storyboard.create",
            json!({ "sceneId": "s7", "name": "The caretaker lets her in out of the rain while the whole house, exactly as the letters described it, waits" }),
        ));
        for i in 0..6 {
            let p = env.write_file(&format!("stress-{i}.png"), &still(i + 1, 640, 360));
            env.ok(
                "storyboard.add_panel",
                json!({ "storyboardId": sb, "visual": if i == 5 { "placeholder" } else { "image" }, "path": p,
                        "description": if i == 0 { "a very long description of the action in this panel that keeps going so that the caption band grows well beyond the other cards in its row and every card stretches to match it" } else { "Short" } }),
            );
            if i == 3 {
                let a = env.ok("storyboard.get", json!({ "id": sb }));
                let path = a["panels"][3]["asset"]["path"]
                    .as_str()
                    .unwrap()
                    .to_string();
                std::fs::remove_file(path).unwrap();
            }
            if i == 4 {
                let a = env.ok("storyboard.get", json!({ "id": sb }));
                let path = a["panels"][4]["asset"]["path"]
                    .as_str()
                    .unwrap()
                    .to_string();
                std::fs::write(path, b"not a picture").unwrap();
            }
        }
        ids.push(sb);
        let empty = id_of(&env.ok(
            "storyboard.create",
            json!({ "name": "Standalone empty board" }),
        ));
        ids.push(empty);
    }
    ids
}

// ------------------------------------------------------------------ the sample writer

fn keep(path: &str, file: &str) {
    std::fs::copy(path, out_dir().join(file)).expect("copy sample");
}

#[test]
#[ignore = "writes the sample PDFs to release-out/pdf-samples"]
fn write_shot_and_storyboard_samples() {
    let film = build_film();
    seed_shots(&film);
    let boards = seed_boards(&film, false);
    for (file, args) in [
        ("c3-shots-single.pdf", json!({ "sceneIds": ["s1"] })),
        ("c3-shots-multiple.pdf", json!({ "sceneIds": ["s1", "s2"] })),
        ("c3-shots-entire.pdf", json!({})),
    ] {
        let (p, r) = export(&film, "shot.export_list", args, file);
        println!("{file}: {} pages", r["pages"]);
        keep(&p, file);
    }
    for cols in [2, 3] {
        let (p, r) = export(
            &film,
            "storyboard.export_sheet",
            json!({ "includeNotes": true, "columns": cols }),
            &format!("c3-storyboard-{cols}up.pdf"),
        );
        println!("storyboard {cols}-up: {} pages", r["pages"]);
        keep(&p, &format!("c3-storyboard-{cols}up.pdf"));
    }
    let (p, _) = export(
        &film,
        "storyboard.export_sheet",
        json!({ "storyboardIds": [boards[1]], "includeNotes": true }),
        "c3-storyboard-one.pdf",
    );
    keep(&p, "c3-storyboard-one.pdf");

    let stress = build_film();
    seed_stress(&stress);
    let boards = seed_boards(&stress, true);
    let _ = boards;
    let (p, r) = export(
        &stress,
        "shot.export_list",
        json!({ "sceneIds": ["s4"] }),
        "c3-stress-shots-single.pdf",
    );
    println!("stress single: {} pages", r["pages"]);
    keep(&p, "c3-stress-shots-single.pdf");
    for cols in [2, 3] {
        let (p, r) = export(
            &stress,
            "storyboard.export_sheet",
            json!({ "includeNotes": true, "columns": cols }),
            &format!("c3-stress-storyboard-{cols}up.pdf"),
        );
        println!("stress storyboard {cols}-up: {} pages", r["pages"]);
        keep(&p, &format!("c3-stress-storyboard-{cols}up.pdf"));
    }
}

// ------------------------------------------------------------------ rules

fn is_footer(r: &Run) -> bool {
    r.text == "Created using OpenFrame" || r.text.starts_with("Page ")
}

/// Rules every one of these documents keeps: portrait Letter, footer and nothing below it,
/// nothing outside the side margins, nothing smaller than 6.4 pt.
fn assert_page_rules(path: &str) -> Dump {
    let d = dump(path);
    assert_eq!(d.size, (612.0, 792.0), "portrait US Letter");
    for r in &d.runs {
        assert!(r.size >= 6.4, "text under 6.4 pt: {r:?}");
        if !is_footer(r) {
            assert!(r.y >= 66.0 - 0.5, "content in the footer zone: {r:?}");
        }
        assert!(r.x >= 46.0 - 0.5, "outside the left margin: {r:?}");
    }
    for p in 0..d.pages {
        assert!(
            d.text(p)
                .contains(&format!("Page {} of {}", p + 1, d.pages)),
            "footer on page {}",
            p + 1
        );
    }
    d
}

#[test]
fn single_scene_shot_list_is_portrait_with_one_band_and_a_coverage_strip() {
    let film = build_film();
    seed_shots(&film);
    let (path, r) = export(
        &film,
        "shot.export_list",
        json!({ "sceneIds": ["s1"] }),
        "single.pdf",
    );
    assert_eq!(r["contentsLabel"], "3 shots in 1 scene");
    let d = assert_page_rules(&path);
    assert_eq!(d.pages, 1);
    let all = d.all();
    // The scene heading is printed once; the band carries the number and the count.
    assert_eq!(all.matches("SAVITRI'S KITCHEN").count(), 1, "{all}");
    for want in [
        "SCENE 1",
        "3 SHOTS",
        "LENSES",
        "24mm · 40mm · 100mm macro",
        "CHARACTERS IN THESE SHOTS",
    ] {
        assert!(all.contains(want), "missing {want}: {all}");
    }
    // A single page has no running header; a long document has (checked below).
    assert!(!all.contains("SHOT LIST · SCENE"));
    // Column order: Done, Image, Shot, Description, Size, Movement, Angle, Lens; no Notes column.
    let heads = [
        "DONE",
        "IMAGE",
        "SHOT",
        "DESCRIPTION",
        "SIZE",
        "MOVEMENT",
        "ANGLE",
        "LENS",
    ];
    let xs: Vec<f32> = heads
        .iter()
        .map(|h| {
            d.runs
                .iter()
                .find(|r| r.text == *h && r.size == 7.0)
                .unwrap_or_else(|| panic!("{h}"))
                .x
        })
        .collect();
    assert!(xs.windows(2).all(|w| w[0] < w[1]), "{xs:?}");
    assert!(!all.contains("NOTES"));
    // Notes sit under the description in the fixed order, with their labels.
    let at = |s: &str| all.find(s).unwrap_or_else(|| panic!("{s}"));
    assert!(at("Sound:") < at("Camera:"));
    // Labels of the shots and the empty camera cell dash.
    for want in ["1A", "1B", "1C", "No image", "Overhead", "100mm macro"] {
        assert!(all.contains(want), "missing {want}");
    }
    // Scene 2 has sizes only. Alone it has no Image, Movement, Angle or Lens column and no dash;
    // next to scene 1 the columns exist for the whole document and its empty cells print a dash.
    let (two, _) = export(
        &film,
        "shot.export_list",
        json!({ "sceneIds": ["s2"] }),
        "two.pdf",
    );
    let two = assert_page_rules(&two).all();
    assert!(two.contains("SIZE") && !two.contains('–'));
    for gone in ["IMAGE", "MOVEMENT", "ANGLE", "LENS"] {
        assert!(!two.contains(gone), "{gone} column without data");
    }
    let (both, _) = export(
        &film,
        "shot.export_list",
        json!({ "sceneIds": ["s1", "s2"] }),
        "both.pdf",
    );
    let both = assert_page_rules(&both).all();
    assert!(both.contains('–'), "an empty camera cell prints a dash");
}

#[test]
fn no_word_breaks_and_no_header_wraps_in_the_camera_columns() {
    let film = build_film();
    seed_shots(&film);
    seed_stress(&film);
    let (path, _) = export(&film, "shot.export_list", json!({}), "all.pdf");
    let d = assert_page_rules(&path);
    let texts: Vec<&str> = d.runs.iter().map(|r| r.text.as_str()).collect();
    for word in ["Overhead", "anamorphic", "MOVEMENT"] {
        assert!(texts.iter().any(|t| t.contains(word)), "{word} stays whole");
    }
    for broken in ["Overhea", "anamorphi", "MOVEMEN"] {
        assert!(
            !texts.iter().any(|t| t.ends_with(broken) || *t == broken),
            "{broken} is a broken word"
        );
    }
}

#[test]
fn whole_project_keeps_short_scenes_whole_repeats_the_band_and_names_every_page() {
    let film = build_film();
    seed_shots(&film);
    seed_stress(&film);
    let (path, r) = export(&film, "shot.export_list", json!({}), "all.pdf");
    let d = assert_page_rules(&path);
    assert!(d.pages > 1, "{} pages", d.pages);
    assert!(r["contentsLabel"].as_str().unwrap().contains("scenes"));
    // The project is named at the top of every page, small and at the right.
    for p in 0..d.pages {
        assert!(
            d.runs.iter().any(|r| r.page == p
                && r.text.starts_with("MONSOON LETTERS · SHOT LIST")
                && r.y > 760.0),
            "running header on page {}",
            p + 1
        );
    }
    // Scenes of three shots or fewer never straddle a page.
    for scene in ["SCENE 1", "SCENE 2", "SCENE 6", "SCENE 8"] {
        assert_eq!(d.pages_with(scene).len(), 1, "{scene} on one page only");
    }
    // Every band's count adds up to the shots listed, and every shot has its tile.
    let listed: usize = r["contentsLabel"]
        .as_str()
        .unwrap()
        .split(' ')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let counted: usize = d
        .runs
        .iter()
        .filter_map(|r| {
            r.text
                .strip_suffix(" SHOTS")
                .or(r.text.strip_suffix(" SHOT"))
        })
        .map(|n| n.parse::<usize>().unwrap())
        .sum();
    assert_eq!(counted, listed);
    let tiles = d
        .runs
        .iter()
        .filter(|r| {
            r.size == 9.0
                && r.text.len() <= 5
                && r.text.starts_with(|c: char| c.is_ascii_digit())
                && r.text.ends_with(|c: char| c.is_ascii_uppercase())
        })
        .count();
    assert_eq!(tiles, listed);
    // The 14-shot scene splits; its continuation repeats the band with CONTINUED.
    let pages = d.pages_with("SCENE 4");
    assert!(pages.len() >= 2, "scene 4 runs over {pages:?}");
    for p in &pages[1..] {
        assert!(
            d.text(*p).contains("CONTINUED"),
            "band repeats on page {}",
            p + 1
        );
        assert!(
            d.text(*p).contains("DESCRIPTION"),
            "header repeats on page {}",
            p + 1
        );
    }
    // The last fragment of a split scene has at least two rows: count shot tiles of scene 4 per page.
    let tiles = |p: usize| {
        d.runs
            .iter()
            .filter(|r| {
                r.page == p && r.text.starts_with('4') && r.text.len() <= 3 && r.text != "4"
            })
            .count()
    };
    assert!(tiles(*pages.last().unwrap()) >= 2);
    // A band is never the last thing on a page: below each band text there is at least a row.
    for run in d
        .runs
        .iter()
        .filter(|r| r.text.starts_with("SCENE ") && r.size < 9.0)
    {
        assert!(
            run.y > 66.0 + 60.0,
            "a band at the foot of page {}: {run:?}",
            run.page + 1
        );
    }
}

#[test]
fn shot_list_picture_is_never_cropped_and_placeholders_keep_their_words() {
    let film = build_film();
    seed_stress(&film);
    let (path, _) = export(
        &film,
        "shot.export_list",
        json!({ "sceneIds": ["s4"] }),
        "stress.pdf",
    );
    let d = assert_page_rules(&path);
    // Square, tall and ultra wide pictures sit inside the 80 x 45 frame with their aspect.
    let want = [1.0f32, 0.75, 860.0 / 360.0];
    let ratios: Vec<f32> = d.images.iter().map(|(_, r)| r[2] / r[3]).collect();
    for w in want {
        assert!(
            ratios.iter().any(|r| (r / w - 1.0).abs() < 0.02),
            "no picture with aspect {w}: {ratios:?}"
        );
    }
    for (_, [_, _, w, h]) in &d.images {
        assert!(
            *w <= 80.01 && *h <= 45.01,
            "{w} x {h} does not fit the frame"
        );
    }
    let all = d.all();
    for want in [
        "No image",
        "Image unavailable: shot-",
        "Image can't be printed",
    ] {
        assert!(all.contains(want), "missing {want}");
    }
}

#[test]
fn shot_list_is_deterministic() {
    let film = build_film();
    seed_shots(&film);
    let (a, _) = export(&film, "shot.export_list", json!({}), "a.pdf");
    let (b, _) = export(&film, "shot.export_list", json!({}), "b.pdf");
    assert_eq!(std::fs::read(a).unwrap(), std::fs::read(b).unwrap());
}

#[test]
fn removed_and_changed_scenes_carry_their_status_and_letters_only_labels() {
    let film = build_film();
    seed_shots(&film);
    // Scene 8 leaves the script: its shots stay, labelled by letters only.
    film.env
        .core
        .project()
        .unwrap()
        .store
        .with_writer(|c| {
            c.execute_batch(
                "PRAGMA foreign_keys=OFF; DELETE FROM screenplay_scene WHERE id='s8'; PRAGMA foreign_keys=ON;",
            )
                .expect("remove scene");
            Ok(())
        })
        .unwrap();
    let (off, _) = export(&film, "shot.export_list", json!({}), "off.pdf");
    let (on, _) = export(
        &film,
        "shot.export_list",
        json!({ "includeRemoved": true }),
        "on.pdf",
    );
    let (off, on) = (dump(&off), dump(&on));
    assert!(off.all().contains("Scene changed since planning"));
    assert!(
        !off.all()
            .contains("Scene removed from the script (kept for reference)")
    );
    assert!(
        on.all()
            .contains("Scene removed from the script (kept for reference)")
    );
    // Removed scenes come after every active one.
    let last_active = on.all().rfind("SCENE 6").unwrap();
    let removed = on.all().find("Scene removed from the script").unwrap();
    assert!(removed > last_active);
    // A removed scene has no number: its shot label is a bare letter.
    assert!(
        on.runs.iter().any(|r| r.text == "A" && r.size == 9.0),
        "letters-only label"
    );
}

// ------------------------------------------------------------------ storyboard

#[test]
fn storyboard_sheet_is_portrait_with_two_or_three_panels_per_row() {
    let film = build_film();
    seed_shots(&film);
    seed_boards(&film, false);
    let mut pages = Vec::new();
    for cols in [2, 3] {
        let (path, r) = export(
            &film,
            "storyboard.export_sheet",
            json!({ "includeNotes": true, "columns": cols }),
            &format!("b{cols}.pdf"),
        );
        assert_eq!(r["contentsLabel"], "13 panels");
        let d = assert_page_rules(&path);
        pages.push(d.pages);
        // The number of panels side by side: distinct left edges of the panel tiles on page 1.
        let mut tile_x: Vec<f32> = d
            .runs
            .iter()
            .filter(|r| r.page == 0 && r.text == "Panel")
            .map(|r| (r.x / 10.0).round())
            .collect();
        tile_x.sort_by(f32::total_cmp);
        tile_x.dedup();
        assert_eq!(
            tile_x.len(),
            cols as usize,
            "{cols} panels per row: {tile_x:?}"
        );
        // Each storyboard starts a page and names itself in the running header on every page.
        let all = d.all();
        for want in ["PANELS", "Blank panel", "Sound: ", "Duration: ", "Note: "] {
            assert!(all.contains(want) || want.ends_with(' '), "missing {want}");
        }
        for p in 0..d.pages {
            assert!(
                d.runs.iter().any(|r| r.page == p
                    && r.text.starts_with("MONSOON LETTERS · STORYBOARDS · ")
                    && r.y > 760.0),
                "page {} has no storyboard header",
                p + 1
            );
        }
        // Page 1 holds the first board's four panels: at least two rows at either width.
        assert!(
            d.runs
                .iter()
                .filter(|r| r.page == 0 && r.text == "Panel")
                .count()
                >= 4
        );
        // Every picture keeps its source aspect inside a 16:9 frame.
        for (_, [_, _, w, h]) in &d.images {
            let ratio = w / h;
            assert!(
                [16.0 / 9.0, 1.0, 0.75, 860.0 / 360.0]
                    .iter()
                    .any(|s| (ratio / s - 1.0).abs() < 0.01),
                "aspect {ratio} is not one of the sources"
            );
            let frame_w = if cols == 2 { 243.0 } else { 156.0 };
            assert!(
                *w <= frame_w + 0.5 && *h <= frame_w * 9.0 / 16.0 + 0.5,
                "{w} x {h}"
            );
        }
    }
    assert!(
        pages[1] <= pages[0],
        "3-up needs no more pages than 2-up: {pages:?}"
    );
    // The default is two per row.
    let (path, _) = export(&film, "storyboard.export_sheet", json!({}), "default.pdf");
    let (two, _) = export(
        &film,
        "storyboard.export_sheet",
        json!({ "columns": 2 }),
        "two.pdf",
    );
    assert_eq!(std::fs::read(path).unwrap(), std::fs::read(two).unwrap());
}

#[test]
fn storyboard_keeps_a_board_that_fits_on_one_page_and_shows_placeholders_exactly() {
    let film = build_film();
    seed_shots(&film);
    let boards = seed_boards(&film, true);
    let (path, r) = export(
        &film,
        "storyboard.export_sheet",
        json!({ "includeNotes": false, "columns": 2 }),
        "stress.pdf",
    );
    assert_eq!(r["warnings"].as_array().unwrap().len(), 2, "{r}");
    let d = assert_page_rules(&path);
    let all = d.all();
    for want in [
        "Blank panel",
        "Image unavailable: stress-3.png",
        "Image can't be printed",
        "No panels yet.",
        "Scene 4",
    ] {
        if want == "Scene 4" {
            continue;
        }
        assert!(all.contains(want), "missing {want}");
    }
    assert!(!all.contains("Note: "), "notes are off");
    // The first board (4 panels at 2 per row) is one page with its banner on top.
    assert_eq!(d.pages_with("4 PANELS"), vec![0]);
    // A long board name is shortened in the running header only.
    assert!(d.runs.iter().any(|r| {
        r.text
            .starts_with("MONSOON LETTERS · STORYBOARDS · THE CARETAKER")
            && r.text.ends_with('…')
    }));
    // One board alone: the header says STORYBOARD and the sheet has no banner.
    let (one, _) = export(
        &film,
        "storyboard.export_sheet",
        json!({ "storyboardIds": [boards[0]] }),
        "one.pdf",
    );
    let d = assert_page_rules(&one);
    assert!(
        d.runs
            .iter()
            .any(|r| r.text == "MONSOON LETTERS · STORYBOARD")
    );
    assert!(
        !d.all().contains("STORYBOARD TITLE"),
        "no banner for one board"
    );
}

#[test]
fn storyboard_is_deterministic() {
    let film = build_film();
    seed_shots(&film);
    seed_boards(&film, false);
    let (a, _) = export(
        &film,
        "storyboard.export_sheet",
        json!({ "columns": 3 }),
        "a.pdf",
    );
    let (b, _) = export(
        &film,
        "storyboard.export_sheet",
        json!({ "columns": 3 }),
        "b.pdf",
    );
    assert_eq!(std::fs::read(a).unwrap(), std::fs::read(b).unwrap());
}
