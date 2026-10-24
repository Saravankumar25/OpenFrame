//! Layout rules of the Beat Board PDFs (PDF design package v2, Beat_Board_Outline and
//! Beat_Board_Card_Board, sections H and J), read from the text runs of the real output. The
//! `write_*` test writes the samples and the stress fixtures to `release-out/pdf-samples` for the
//! eye:
//!
//! ```text
//! node scripts/cargo-queued.mjs test -p openframe-application --test production \
//!     story_design::write -- --ignored --nocapture
//! ```

use std::path::{Path, PathBuf};

use openframe_test_support::TestEnv;
use serde_json::{Value, json};

use super::call_sheet_design::{FOOTER_Y, HEADER_Y, Run, check_furniture, read};

// ------------------------------------------------------------------ fixture writer

fn q(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

fn opt(s: Option<&str>) -> String {
    s.map_or("NULL".to_string(), q)
}

/// SQL for one board (film or one episode): acts, sequences, beats, cards and comments.
struct Seed {
    sql: String,
    /// The episode the rows belong to, as SQL.
    ep: String,
    pos: i64,
}

impl Seed {
    fn new(episode: Option<&str>) -> Seed {
        Seed {
            sql: String::new(),
            ep: opt(episode),
            pos: 0,
        }
    }
    fn next(&mut self) -> i64 {
        self.pos += 1;
        self.pos
    }
    fn act(&mut self, id: &str, title: &str, note: Option<&str>) {
        let pos = self.next();
        self.sql.push_str(&format!(
            "INSERT INTO story_act(id,episode_id,title,note,position,created_at,updated_at) VALUES ({},{},{},{},{pos},1,1);",
            q(id),
            self.ep,
            q(title),
            opt(note)
        ));
    }
    fn seq(&mut self, id: &str, act: Option<&str>, title: &str, note: Option<&str>) {
        let pos = self.next();
        self.sql.push_str(&format!(
            "INSERT INTO story_sequence(id,episode_id,act_id,title,note,position,created_at,updated_at) VALUES ({},{},{},{},{},{pos},1,1);",
            q(id),
            self.ep,
            opt(act),
            q(title),
            opt(note)
        ));
    }
    #[allow(clippy::too_many_arguments)]
    fn card(
        &mut self,
        id: &str,
        parent: (&str, Option<&str>),
        heading: Option<&str>,
        desc: &str,
        notes: Option<&str>,
    ) {
        let pos = self.next();
        self.sql.push_str(&format!(
            "INSERT INTO story_scene_card(id,episode_id,parent_type,parent_id,short_description,scene_heading,notes,position,created_at,updated_at) VALUES ({},{},{},{},{},{},{},{pos},1,1);",
            q(id),
            self.ep,
            q(parent.0),
            opt(parent.1),
            q(desc),
            opt(heading),
            opt(notes)
        ));
    }
    fn beat(&mut self, id: &str, parent: (&str, Option<&str>), text: &str, note: Option<&str>) {
        let pos = self.next();
        self.sql.push_str(&format!(
            "INSERT INTO story_beat(id,episode_id,parent_type,parent_id,text,note,position,created_at,updated_at) VALUES ({},{},{},{},{},{},{pos},1,1);",
            q(id),
            self.ep,
            q(parent.0),
            opt(parent.1),
            q(text),
            opt(note)
        ));
    }
    #[allow(clippy::too_many_arguments)]
    fn comment(
        &mut self,
        id: &str,
        target: (&str, &str),
        parent: Option<&str>,
        author: &str,
        status: &str,
        body: &str,
        at: i64,
    ) {
        self.sql.push_str(&format!(
            "INSERT INTO comment(id,parent_id,target_type,target_id,body,status,author_user_id,author_name,created_at,updated_at) VALUES ({},{},{},{},{},{},'u2',{},{at},{at});",
            q(id),
            opt(parent),
            q(target.0),
            q(target.1),
            q(body),
            q(status),
            q(author)
        ));
    }
}

fn exec(env: &TestEnv, sql: &str) {
    env.core
        .project()
        .expect("project open")
        .store
        .with_writer(|c| {
            c.execute_batch(sql).expect("fixture sql");
            Ok(())
        })
        .unwrap();
}

fn sample_dir() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repository root")
        .to_path_buf();
    let d = root.join("release-out").join("pdf-samples");
    std::fs::create_dir_all(&d).expect("sample folder");
    d
}

fn export(env: &TestEnv, mut args: Value, file: &str) -> (String, Vec<Run>, usize) {
    let path = env.dir.path().join(file).to_string_lossy().into_owned();
    args["format"] = json!("pdf");
    args["path"] = json!(path);
    env.ok("story.export_board", args);
    let (runs, pages) = read(&path);
    (path, runs, pages)
}

fn keep(path: &str, name: &str) {
    std::fs::copy(path, sample_dir().join(name)).expect("sample copied");
}

// ------------------------------------------------------------------ the Low Tide Radio board

const SLUGS: [&str; 8] = [
    "EXT. OPEN SEA - NIGHT (1999)",
    "INT. COAST RADIO STATION - OPERATIONS ROOM - NIGHT (1999)",
    "INT. SOUNDWAVE STUDIO - BENGALURU - DAY",
    "INT. KSRTC BUS - MOVING - NIGHT",
    "EXT. COAST RADIO STATION - DAWN",
    "INT. THOMAS HOUSE - KITCHEN - DAY",
    "EXT. HARBOUR BREAKWATER - NIGHT",
    "INT. TALUK HOSPITAL - CORRIDOR - DAWN",
];

const LINES: [&str; 5] = [
    "Prologue. A storm. Fishing boats in the swell and a woman's voice on the radio reading positions.",
    "Mariamma at the transmitter. Young Anjali asleep under the desk. The lights die.",
    "Present day. Anjali scrubs breaths out of a detergent ad. Fathima calls: Appachan is missing.",
    "The overnight bus to Kochi. Anjali listens to an old cassette of her mother on air.",
    "Anjali arrives. Appachan sits at the foot of the mast with his dog Kannan. Inspector Latha waits by her jeep.",
];

fn story_day(i: usize) -> String {
    format!(
        "Story day: Day {}. Characters: ANJALI, FATHIMA, APPACHAN.",
        i % 7 + 1
    )
}

/// `n` cards in a sequence, each with a note.
fn cards(s: &mut Seed, prefix: &str, parent: (&str, Option<&str>), n: usize, first: usize) {
    for i in 0..n {
        let k = first + i;
        s.card(
            &format!("{prefix}{k}"),
            parent,
            Some(SLUGS[k % SLUGS.len()]),
            LINES[k % LINES.len()],
            Some(&story_day(k)),
        );
    }
}

/// The board of a small film: three acts, six sequences, act level beats and cards, Unassigned,
/// a Parking Lot and comment threads.
fn low_tide(env: &TestEnv) {
    let mut s = Seed::new(None);
    s.act(
        "a1",
        "Act One: The Call",
        Some("Setup. Ends when Anjali finds the reel."),
    );
    s.seq(
        "q1",
        Some("a1"),
        "Static",
        Some("1999 prologue, then Bengaluru and the journey home."),
    );
    cards(&mut s, "c", ("sequence", Some("q1")), 6, 1);
    s.seq("q2", Some("a1"), "Seven Days", None);
    cards(&mut s, "c", ("sequence", Some("q2")), 6, 7);
    s.beat(
        "b1",
        ("act", Some("a1")),
        "Tone: warm, local, a little funny until the storm.",
        None,
    );
    s.act("a2", "Act Two: The Reel", None);
    s.seq(
        "q3",
        Some("a2"),
        "Oxide",
        Some("The reel is played for the first time."),
    );
    cards(&mut s, "c", ("sequence", Some("q3")), 15, 13);
    s.seq(
        "q4",
        Some("a2"),
        "Storm Warning",
        Some("Midpoint. Appachan is lost in the storm and Suresh brings him back."),
    );
    cards(&mut s, "c", ("sequence", Some("q4")), 8, 28);
    s.comment(
        "m1",
        ("story_scene_card", "c32"),
        None,
        "Meenakshi Varma",
        "Open",
        "This is the midpoint. Protect the time for it on the schedule.",
        10,
    );
    s.comment(
        "m2",
        ("story_scene_card", "c32"),
        Some("m1"),
        "Ravi",
        "Open",
        "Agreed. Moving the marine safety briefing to the day before.",
        11,
    );
    s.comment(
        "m3",
        ("story_scene_card", "c30"),
        None,
        "Ravi",
        "Resolved",
        "Fixed the continuity of the lamp.",
        12,
    );
    s.comment(
        "m4",
        ("story_scene_card", "c29"),
        None,
        "Lakshmi",
        "In Discussion",
        "Could this be a single take?",
        13,
    );
    s.beat(
        "b2",
        ("act", Some("a2")),
        "Midpoint: the town learns the reel exists.",
        None,
    );
    s.card(
        "cx",
        ("act", Some("a2")),
        Some("INT. THOMAS HOUSE - NIGHT"),
        "A card that belongs to the act, not to a sequence.",
        None,
    );
    s.act("a3", "Act Three: The Reckoning", None);
    s.seq("q5", Some("a3"), "What Joe Did", None);
    cards(&mut s, "c", ("sequence", Some("q5")), 6, 36);
    s.seq("q6", Some("a3"), "Low Tide", None);
    cards(&mut s, "c", ("sequence", Some("q6")), 4, 42);
    s.beat(
        "b3",
        ("act", Some("a3")),
        "Climax: the station goes live for the last time.",
        None,
    );
    s.card(
        "cu",
        ("unassigned", None),
        Some("INT. UNDECIDED PLACE - DAY"),
        "A card not placed in any act yet.",
        None,
    );
    s.card("p1", ("parking", None), None, "Pappan on the storm", None);
    s.card(
        "p2",
        ("parking", None),
        Some("INT. COAST RADIO STATION - STAIRWELL - DAY"),
        "Cut idea: Anjali records the sound of the station before demolition and builds a sound map.",
        None,
    );
    s.card(
        "p3",
        ("parking", None),
        None,
        "Alternative opening",
        Some("Open on the demolition and tell the film as the week before it. Parked: it gives away the ending."),
    );
    s.beat("pb", ("parking", None), "Alternative opening", None);
    exec(env, &s.sql);
}

fn low_tide_env() -> TestEnv {
    let env = TestEnv::with_project("Low Tide Radio", "Feature Film");
    low_tide(&env);
    env
}

const ALL: fn() -> Value =
    || json!({ "includeParking": true, "includeNotes": true, "includeComments": true });

fn with(layout: &str, mut base: Value) -> Value {
    base["layout"] = json!(layout);
    base
}

// ------------------------------------------------------------------ samples

#[test]
#[ignore = "writes the sample PDFs to release-out/pdf-samples"]
fn write_beat_board_samples() {
    let env = low_tide_env();
    for (layout, name) in [
        ("outline", "12-beat-board-outline"),
        ("board", "13-beat-board"),
    ] {
        let (path, _, pages) = export(&env, with(layout, ALL()), "all.pdf");
        keep(&path, &format!("{name}.pdf"));
        println!("{name}: {pages} pages");
        let (path, _, pages) = export(&env, with(layout, json!({})), "off.pdf");
        keep(&path, &format!("{name}-options-off.pdf"));
        println!("{name}-options-off: {pages} pages");
    }
}

// ------------------------------------------------------------------ stress fixture

fn words(n: usize, seed: &str) -> String {
    let mut s = String::new();
    while s.len() < n {
        s.push_str(seed);
        s.push(' ');
    }
    s.chars().take(n).collect::<String>().trim_end().to_string()
}

const UNBROKEN: &str =
    "WWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWW";

/// A board that exercises every edge: very long titles, a three paragraph note, threads with all
/// three statuses and replies, unbroken strings, cards with missing parts, an empty act, an empty
/// sequence, a long sequence, Unassigned and the Parking Lot.
fn harbour_lights(env: &TestEnv, episode: Option<&str>) {
    let mut s = Seed::new(episode);
    let p = |x: &str| format!("{x}{}", episode.unwrap_or("f"));
    let (a1, a2, a3, a4) = (p("a1"), p("a2"), p("a3"), p("a4"));
    s.act(
        &a1,
        "Act One: The Light",
        Some("Setup.\n\nThe lighthouse keeper's death opens the film.\n\nThe town does not yet know why."),
    );
    let q1 = p("q1");
    s.seq(&q1, Some(&a1), "Night Shift", Some("The first night."));
    for i in 0..4 {
        s.card(
            &p(&format!("k{i}")),
            ("sequence", Some(&q1)),
            Some(SLUGS[i]),
            LINES[i],
            Some(&story_day(i)),
        );
    }
    s.act(
        &a2,
        &words(200, "THE LONG NIGHT OF THE HARBOUR"),
        Some("A long title act."),
    );
    let q2 = p("q2");
    s.seq(
        &q2,
        Some(&a2),
        &words(130, "The sequence with the very long title"),
        None,
    );
    // The tall card: three paragraph note, two comments, two replies.
    let tall = p("tall");
    s.card(
        &tall,
        ("sequence", Some(&q2)),
        Some(&words(190, "INT. THE OLD CANNERY - ABOVE THE BOILER HOUSE")),
        "A card with a very long slugline and a long note.",
        Some("First paragraph of the note, with a few words.\n\nSecond paragraph: the cannery smells of salt and diesel and the light is green.\n\nThird paragraph: nobody has been inside since the strike."),
    );
    s.comment(
        &p("m1"),
        ("story_scene_card", &tall),
        None,
        "Meenakshi Varma",
        "Open",
        "Open question about the boiler house. Can we shoot it in one day?",
        10,
    );
    s.comment(
        &p("m2"),
        ("story_scene_card", &tall),
        Some(&p("m1")),
        "Ravi",
        "Open",
        "Only with a second camera.",
        11,
    );
    s.comment(
        &p("m3"),
        ("story_scene_card", &tall),
        Some(&p("m1")),
        "Lakshmi",
        "Open",
        "Agreed, I will ask the DOP.",
        12,
    );
    s.comment(
        &p("m4"),
        ("story_scene_card", &tall),
        None,
        "Ravi",
        "Resolved",
        "The permit is in.",
        13,
    );
    s.card(
        &p("u1"),
        ("sequence", Some(&q2)),
        Some(UNBROKEN),
        UNBROKEN,
        Some(UNBROKEN),
    );
    s.card(
        &p("u2"),
        ("sequence", Some(&q2)),
        None,
        "A card with a description and no heading.",
        None,
    );
    s.card(
        &p("u3"),
        ("sequence", Some(&q2)),
        Some("EXT. A HEADING WITH NO DESCRIPTION"),
        "",
        None,
    );
    s.card(&p("u4"), ("sequence", Some(&q2)), None, "", None);
    s.beat(
        &p("bb"),
        ("sequence", Some(&q2)),
        &words(190, "A beat with a very long text"),
        Some("Beat note."),
    );
    s.comment(
        &p("m5"),
        ("story_beat", &p("bb")),
        None,
        "Lakshmi",
        "In Discussion",
        "Is this a beat or a scene?",
        14,
    );
    s.seq(&p("q3"), Some(&a2), "An empty sequence", None);
    s.act(&a3, "Act Three: An empty act", None);
    s.act(&a4, "Act Four: Twenty six cards", None);
    let q4 = p("q4");
    s.seq(&q4, Some(&a4), "The long sequence", None);
    for i in 0..26 {
        s.card(
            &p(&format!("L{i}")),
            ("sequence", Some(&q4)),
            Some(SLUGS[i % SLUGS.len()]),
            LINES[i % LINES.len()],
            Some(&story_day(i)),
        );
    }
    s.beat(&p("bl"), ("act", Some(&a4)), "An act level beat.", None);
    s.card(
        &p("cu"),
        ("unassigned", None),
        Some("INT. NOWHERE"),
        "Unassigned card.",
        None,
    );
    s.card(
        &p("cp"),
        ("parking", None),
        Some("INT. PARKED"),
        "A parked card.",
        Some("Parked note."),
    );
    exec(env, &s.sql);
}

fn project_env() -> TestEnv {
    let env = TestEnv::with_project("Harbour Lights", "Series");
    exec(
        &env,
        "INSERT INTO season(id,title,position,created_at,updated_at) VALUES ('s1','Season 1',1,1,1);
         INSERT INTO episode(id,season_id,title,position,created_at,updated_at) VALUES
            ('e1','s1','Episode 1: The Tide Table',1,1,1),('e2','s1','Episode 2: Empty',2,1,1);",
    );
    harbour_lights(&env, None);
    // The episode board is a smaller one without a Parking Lot.
    let mut s = Seed::new(Some("e1"));
    s.act("ea", "Act One", Some("Episode note."));
    s.seq("es", Some("ea"), "Pilot sequence", None);
    for i in 0..5 {
        s.card(
            &format!("ec{i}"),
            ("sequence", Some("es")),
            Some(SLUGS[i]),
            LINES[i],
            Some(&story_day(i)),
        );
    }
    exec(&env, &s.sql);
    env
}

#[test]
#[ignore = "writes the sample PDFs to release-out/pdf-samples"]
fn write_beat_board_stress_samples() {
    let env = project_env();
    for (layout, name) in [
        ("outline", "14-beat-board-outline-stress"),
        ("board", "15-beat-board-stress"),
    ] {
        let mut args = with(layout, ALL());
        args["scope"] = json!("project");
        let (path, _, pages) = export(&env, args, "stress.pdf");
        keep(&path, &format!("{name}.pdf"));
        println!("{name}: {pages} pages");
        let mut args = with(layout, json!({}));
        args["scope"] = json!("project");
        let (path, _, pages) = export(&env, args, "stress-off.pdf");
        keep(&path, &format!("{name}-options-off.pdf"));
        println!("{name}-options-off: {pages} pages");
    }
}

// ------------------------------------------------------------------ rules (spec J)

const BAR_PAGE_W: f32 = 612.0;
const MARGIN_L: f32 = 46.0;

fn body(runs: &[Run], page: usize) -> Vec<&Run> {
    runs.iter()
        .filter(|r| r.page == page && r.y > FOOTER_Y && r.y < HEADER_Y)
        .collect()
}

/// The runs of the top line of a page body.
fn top_line(runs: &[Run], page: usize) -> Vec<&Run> {
    let b = body(runs, page);
    let top = b.iter().map(|r| r.y).fold(f32::MIN, f32::max);
    b.into_iter().filter(|r| (r.y - top).abs() < 3.0).collect()
}

fn has(runs: &[Run], t: &str) -> bool {
    runs.iter().any(|r| r.text == t)
}

fn count(runs: &[Run], t: &str) -> usize {
    runs.iter().filter(|r| r.text == t).count()
}

fn all_text(runs: &[Run]) -> String {
    runs.iter()
        .map(|r| r.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Strips are printed at 6.8 pt in bold capitals and end with the "continued" run.
fn strips(runs: &[Run], page: usize) -> Vec<String> {
    let b = body(runs, page);
    b.iter()
        .filter(|r| r.text.contains('\u{203a}') || r.text.starts_with("ACT ") && r.size < 7.0)
        .map(|r| r.text.clone())
        .collect()
}

fn title_of(path: &str) -> String {
    let doc = lopdf::Document::load(path).unwrap();
    let info = doc.trailer.get(b"Info").unwrap().as_reference().unwrap();
    let d = doc.get_dictionary(info).unwrap();
    match d.get(b"Title").unwrap() {
        lopdf::Object::String(b, _) => String::from_utf8_lossy(b).into_owned(),
        _ => panic!("title"),
    }
}

/// Every run keeps inside the side margins (measured in its own font) and clear of the footer.
fn inside_margins(path: &str) {
    let doc = lopdf::Document::load(path).unwrap();
    for (_, page) in doc.get_pages() {
        let content = lopdf::content::Content::decode(&doc.get_page_content(page)).unwrap();
        let (mut x, mut y, mut size, mut bold) = (0.0f32, 0.0f32, 0.0f32, false);
        for op in content.operations {
            let num = |i: usize| {
                op.operands
                    .get(i)
                    .and_then(|o| o.as_float().ok())
                    .unwrap_or(0.0)
            };
            match op.operator.as_str() {
                "Tf" => {
                    size = num(1);
                    bold = op.operands[0].as_name().ok() == Some(b"F5");
                }
                "Tm" => (x, y) = (num(4), num(5)),
                "Tj" => {
                    if let Some(lopdf::Object::String(b, _)) = op.operands.first() {
                        if y <= FOOTER_Y || y >= HEADER_Y {
                            continue;
                        }
                        let t: String = b
                            .iter()
                            .map(|c| openframe_import_export::winansi::decode_byte(*c))
                            .collect();
                        let w = openframe_import_export::report::text_width(&t, size, bold);
                        assert!(
                            x >= MARGIN_L - 0.5 && x + w <= BAR_PAGE_W - MARGIN_L + 0.5,
                            "{t:?} runs past a margin ({x} to {})",
                            x + w
                        );
                    }
                }
                _ => {}
            }
        }
    }
}

/// The numbers printed in the number tiles, in order (the key's example tile is not one).
fn tile_numbers(runs: &[Run], layout: &str) -> Vec<String> {
    let tile = if layout == "board" { 7.6 } else { 8.4 };
    let first_bar = runs
        .iter()
        .position(|r| (r.size - 10.5).abs() < 0.05)
        .unwrap_or(0);
    runs[first_bar..]
        .iter()
        .filter(|r| (r.size - tile).abs() < 0.05 && r.text.parse::<usize>().is_ok())
        .map(|r| r.text.clone())
        .collect()
}

#[test]
fn both_layouts_are_portrait_with_the_footer_and_the_running_header() {
    let env = low_tide_env();
    for (layout, header, title) in [
        (
            "outline",
            "LOW TIDE RADIO \u{b7} BEAT BOARD OUTLINE",
            "Beat Board Outline",
        ),
        ("board", "LOW TIDE RADIO \u{b7} BEAT BOARD", "Beat Board"),
    ] {
        let (path, runs, pages) = export(&env, with(layout, ALL()), "x.pdf");
        assert!(pages >= 3, "{layout}: {pages} pages");
        check_furniture(&runs, pages, header, &[]);
        inside_margins(&path);
        assert_eq!(title_of(&path), title);
        let doc = lopdf::Document::load(&path).unwrap();
        let page = *doc.get_pages().values().next().unwrap();
        let mb = doc.get_dictionary(page).unwrap().get(b"MediaBox").unwrap();
        let mb: Vec<f32> = mb
            .as_array()
            .unwrap()
            .iter()
            .map(|o| o.as_float().unwrap())
            .collect();
        assert_eq!(
            (mb[2], mb[3]),
            (612.0, 792.0),
            "{layout} is portrait Letter"
        );
    }
}

#[test]
fn the_card_board_has_three_cards_to_a_row_168_wide() {
    let env = low_tide_env();
    let (_, runs, _) = export(&env, with("board", ALL()), "b.pdf");
    // Card text starts 6.4 pt inside the card: three columns of a 520 pt page, 8 pt apart.
    let mut xs: Vec<i32> = runs
        .iter()
        .filter(|r| r.size > 8.3 && r.size < 8.5 && r.y > FOOTER_Y && r.y < HEADER_Y)
        .map(|r| (r.x * 10.0).round() as i32)
        .collect();
    xs.sort();
    xs.dedup();
    let want = [524, 2284, 4044].map(|x| x as f32 / 10.0);
    assert_eq!(xs.len(), 3, "{xs:?}");
    for (x, w) in xs.iter().zip(want) {
        assert!(
            (*x as f32 / 10.0 - w).abs() < 0.2,
            "{xs:?} against {want:?}"
        );
    }
    // The tile of every first card of a row sits at the same left edge.
    let tiles: Vec<f32> = runs
        .iter()
        .filter(|r| r.text == "1" && r.size < 8.0)
        .map(|r| r.x)
        .collect();
    assert_eq!(tiles.len(), 1);
}

#[test]
fn cards_follow_board_order_with_their_numbers_and_the_parking_lot_has_none() {
    let env = low_tide_env();
    for layout in ["outline", "board"] {
        let (_, runs, _) = export(&env, with(layout, ALL()), "n.pdf");
        // Number tiles: 1 to 47 in order of appearance.
        let tile = if layout == "board" { 7.6 } else { 8.4 };
        assert_eq!(
            tile_numbers(&runs, layout),
            (1..=47).map(|n| n.to_string()).collect::<Vec<_>>(),
            "{layout}"
        );
        // Parked cards print the word instead of a number.
        let label = if layout == "board" {
            "SCENE CARD"
        } else {
            "CARD"
        };
        let bar = runs
            .iter()
            .position(|r| r.text == "PARKING LOT" && r.size > 10.0)
            .unwrap();
        let parked_from = runs[bar].page;
        assert!(parked_from >= 5);
        assert!(
            runs.iter()
                .any(|r| r.text == label && r.page >= parked_from)
        );
        // Parking Lot is last, Unassigned before it.
        let y = |t: &str| {
            let r = runs.iter().find(|r| r.text == t).unwrap();
            (r.page, -r.y as i32)
        };
        let _ = y;
        let unassigned = runs.iter().position(|r| r.text == "UNASSIGNED").unwrap();
        assert!(unassigned < bar);
        let act3 = runs
            .iter()
            .position(|r| r.text == "ACT THREE: THE RECKONING" && r.size > 10.0)
            .unwrap();
        assert!(act3 < unassigned);
        // No number tile is drawn after the Parking Lot bar.
        assert!(
            !runs[bar..]
                .iter()
                .any(|r| (r.size - tile).abs() < 0.05 && r.text.parse::<usize>().is_ok()),
            "a number in the Parking Lot of {layout}"
        );
    }
}

#[test]
fn selected_scope_keeps_the_original_numbers() {
    let env = low_tide_env();
    let mut args = ALL();
    args["scope"] = json!("selected");
    args["itemIds"] = json!(["c40", "c41"]);
    for layout in ["outline", "board"] {
        let (_, runs, _) = export(&env, with(layout, args.clone()), "s.pdf");
        assert_eq!(tile_numbers(&runs, layout), vec!["41", "42"], "{layout}");
        // The acts and sequences that lead to them are shown, with the third panel line.
        assert!(has(&runs, "ACT THREE: THE RECKONING"));
        assert!(all_text(&runs).contains("Acts and sequences shown for context"));
        assert!(has(&runs, "Selected items only"));
    }
}

#[test]
fn act_level_items_never_sit_in_a_lane_and_the_label_comes_only_with_sequences() {
    let env = low_tide_env();
    for layout in ["outline", "board"] {
        let (_, runs, _) = export(&env, with(layout, ALL()), "l.pdf");
        // Act One, Two and Three each have sequences and loose items: three labels.
        assert_eq!(count(&runs, "NOT IN A SEQUENCE"), 3, "{layout}");
        // An act with loose items and no sequence has no label.
        let env = TestEnv::with_project("Plain", "Short Film");
        let mut s = Seed::new(None);
        s.act("a", "Act One", None);
        s.beat("b", ("act", Some("a")), "A loose beat", None);
        s.card(
            "c",
            ("act", Some("a")),
            Some("INT. ROOM - DAY"),
            "A loose card",
            None,
        );
        exec(&env, &s.sql);
        let (_, runs, _) = export(&env, with(layout, json!({})), "p.pdf");
        assert_eq!(count(&runs, "NOT IN A SEQUENCE"), 0);
        assert!(has(&runs, "A loose beat") && has(&runs, "A loose card"));
        assert!(!runs.iter().any(|r| r.text == "SEQUENCE"));
    }
}

#[test]
fn a_bar_or_band_never_ends_a_page_and_pages_inside_a_container_start_with_the_strip() {
    let env = low_tide_env();
    let headings = [
        "ACT ONE: THE CALL",
        "ACT TWO: THE REEL",
        "ACT THREE: THE RECKONING",
        "UNASSIGNED",
        "PARKING LOT",
        "NOT IN A SEQUENCE",
    ];
    for layout in ["outline", "board"] {
        let (_, runs, pages) = export(&env, with(layout, ALL()), "h.pdf");
        for page in 1..=pages {
            let b = body(&runs, page);
            let lowest = b.iter().map(|r| r.y).fold(f32::MAX, f32::min);
            let last: Vec<&str> = b
                .iter()
                .filter(|r| (r.y - lowest).abs() < 40.0)
                .map(|r| r.text.as_str())
                .collect();
            for h in headings
                .iter()
                .chain(&["Static", "Seven Days", "Oxide", "Storm Warning"])
            {
                // A heading may sit near the foot only with its first row below it.
                if let Some(r) = b.iter().find(|r| r.text == *h && r.size >= 9.7) {
                    assert!(
                        b.iter().any(|x| x.y < r.y - 12.0 && x.size < 9.7),
                        "{h} ends page {page} of {layout}: {last:?}"
                    );
                }
            }
            if page == 1 {
                continue;
            }
            let top = top_line(&runs, page);
            let starts_with_bar = top.iter().any(|r| r.size > 10.4);
            let strip = strips(&runs, page);
            if starts_with_bar {
                assert!(
                    strip.is_empty(),
                    "page {page} starts with a heading and has a strip {strip:?}"
                );
            } else {
                assert!(
                    top.iter().any(|r| r.size < 7.0
                        && r.text
                            .chars()
                            .filter(|c| c.is_alphabetic())
                            .all(|c| !c.is_lowercase())),
                    "page {page} of {layout} starts inside a container without the strip: {:?}",
                    top.iter().map(|r| &r.text).collect::<Vec<_>>()
                );
                assert!(
                    has_continued(&runs, page),
                    "page {page} strip says continued"
                );
            }
        }
    }
}

fn has_continued(runs: &[Run], page: usize) -> bool {
    top_line(runs, page)
        .iter()
        .any(|r| r.text.contains("continued"))
}

#[test]
fn the_strip_names_the_act_and_the_sequence_a_page_starts_in() {
    let env = low_tide_env();
    let (_, runs, pages) = export(&env, with("board", ALL()), "h.pdf");
    let mut seen = Vec::new();
    for page in 2..=pages {
        for r in top_line(&runs, page) {
            if r.size < 7.0 && r.text.contains('\u{203a}') {
                seen.push(r.text.clone());
            }
        }
    }
    assert!(
        seen.iter()
            .any(|s| s == "ACT TWO: THE REEL \u{203a} SEQUENCE: OXIDE"),
        "{seen:?}"
    );
    assert!(
        seen.iter()
            .any(|s| s == "ACT TWO: THE REEL \u{203a} SEQUENCE: STORM WARNING"),
        "{seen:?}"
    );
}

#[test]
fn notes_and_comments_look_different_and_every_comment_shows_its_status() {
    let env = low_tide_env();
    for layout in ["outline", "board"] {
        let (_, runs, _) = export(&env, with(layout, ALL()), "c.pdf");
        assert!(count(&runs, "NOTE") >= 40, "{layout}");
        assert_eq!(count(&runs, "COMMENT"), 3, "{layout}");
        for status in ["OPEN", "IN DISCUSSION", "RESOLVED"] {
            assert!(has(&runs, status), "{status}");
        }
        assert_eq!(count(&runs, "REPLY"), 1);
        assert!(has(&runs, "Meenakshi Varma") && has(&runs, "Ravi"));
        // The legend of the key follows the options.
        assert!(has(&runs, "the item's own note"));
        assert!(has(&runs, "comment thread (status, author, replies)"));
    }
}

#[test]
fn options_off_leave_no_label_no_area_and_the_key_says_board_only() {
    let env = low_tide_env();
    for layout in ["outline", "board"] {
        let (_, runs, _) = export(&env, with(layout, json!({})), "o.pdf");
        for gone in [
            "NOTE",
            "COMMENT",
            "REPLY",
            "PARKING LOT",
            "Pappan on the storm",
            "Meenakshi Varma",
        ] {
            assert!(!has(&runs, gone), "{gone} in {layout}");
        }
        assert!(has(&runs, "BOARD ONLY"));
        assert!(!has(&runs, "the item's own note"));
        assert!(!runs.iter().any(|r| r.text.contains("in Parking Lot")));
        // Unassigned stays whatever the options say.
        assert!(has(&runs, "UNASSIGNED"));
    }
}

#[test]
fn parking_lot_included_but_empty_says_so_in_the_key() {
    let env = TestEnv::with_project("Plain", "Short Film");
    let mut s = Seed::new(None);
    s.act("a", "Act One", None);
    s.card(
        "c",
        ("act", Some("a")),
        Some("INT. ROOM - DAY"),
        "A card",
        None,
    );
    exec(&env, &s.sql);
    let (_, runs, _) = export(
        &env,
        with("board", json!({ "includeParking": true })),
        "e.pdf",
    );
    assert!(has(&runs, "PARKING LOT (NONE)"));
    assert!(!has(&runs, "PARKING LOT"));
}

#[test]
fn empty_containers_and_empty_cards_print_the_source_sentences() {
    let env = project_env();
    for layout in ["outline", "board"] {
        let mut args = with(layout, ALL());
        args["scope"] = json!("project");
        let (_, runs, _) = export(&env, args, "e.pdf");
        assert!(has(&runs, "No sequences, beats or scene cards yet."));
        assert!(has(&runs, "No beats or scene cards yet."));
        assert!(has(&runs, "No heading or description"));
    }
}

#[test]
fn long_text_wraps_whole_nothing_is_cut_and_nothing_leaves_the_margins() {
    let env = project_env();
    for layout in ["outline", "board"] {
        let mut args = with(layout, ALL());
        args["scope"] = json!("project");
        let (path, runs, _) = export(&env, args, "long.pdf");
        inside_margins(&path);
        let text = all_text(&runs);
        // The whole 200 character act title, the three paragraph note and the unbroken strings.
        let title = words(200, "THE LONG NIGHT OF THE HARBOUR");
        let flat: String = text.split_whitespace().collect();
        assert!(
            flat.contains(&title.split_whitespace().collect::<String>()),
            "{layout}: act title is whole"
        );
        for part in [
            "First paragraph of the note",
            "Second paragraph",
            "Third paragraph",
            "since the strike.",
        ] {
            assert!(
                flat.contains(&part.split_whitespace().collect::<String>()),
                "{part}"
            );
        }
        let w: String = runs
            .iter()
            .filter(|r| !r.text.is_empty() && r.text.chars().all(|c| c == 'W'))
            .map(|r| r.text.as_str())
            .collect();
        // The heading, the description and the note of the card hold the unbroken string each (the strip and
        // the act title do not use this letter alone).
        let lens: Vec<usize> = runs
            .iter()
            .filter(|r| !r.text.is_empty() && r.text.chars().all(|c| c == 'W'))
            .map(|r| r.text.len())
            .collect();
        assert_eq!(w.len(), UNBROKEN.len() * 3, "{layout} {lens:?}");
    }
}

#[test]
fn a_card_taller_than_a_page_continues_on_the_next_one_in_every_column() {
    let env = TestEnv::with_project("Plain", "Short Film");
    let mut s = Seed::new(None);
    s.act("a", "Act One", None);
    s.seq("q", Some("a"), "Long notes", None);
    let note: String = (1..=900).map(|i| format!("word{i} ")).collect();
    s.card(
        "c1",
        ("sequence", Some("q")),
        Some("INT. ONE"),
        "First",
        Some(note.trim()),
    );
    s.card(
        "c2",
        ("sequence", Some("q")),
        Some("INT. TWO"),
        "Second",
        Some("Short note."),
    );
    s.card(
        "c3",
        ("sequence", Some("q")),
        Some("INT. THREE"),
        "Third",
        None,
    );
    s.beat("b", ("sequence", Some("q")), "A beat", None);
    exec(&env, &s.sql);
    let (path, runs, pages) = export(
        &env,
        with("board", json!({ "includeNotes": true })),
        "tall.pdf",
    );
    assert!(pages >= 3, "{pages}");
    let flat = all_text(&runs);
    for i in [1, 2, 450, 899, 900] {
        assert!(
            flat.contains(&format!("word{i} ")) || flat.contains(&format!("word{i}")),
            "word{i}"
        );
    }
    let order: Vec<usize> = runs
        .iter()
        .filter(|r| r.text.contains("word"))
        .flat_map(|r| {
            r.text
                .split_whitespace()
                .filter_map(|w| w.strip_prefix("word").and_then(|n| n.parse().ok()))
                .collect::<Vec<usize>>()
        })
        .collect();
    assert_eq!(
        order,
        (1..=900).collect::<Vec<_>>(),
        "every word once, in order"
    );
    inside_margins(&path);
    // The short note of the second card and the beat are still printed.
    assert!(has(&runs, "INT. TWO") && has(&runs, "A beat"));
}

#[test]
fn an_extra_board_starts_a_new_page_with_its_episode_bar_and_empty_boards_are_left_out() {
    let env = project_env();
    let mut args = with("board", ALL());
    args["scope"] = json!("project");
    let (_, runs, _) = export(&env, args, "p.pdf");
    assert!(has(&runs, "Film Beat Board"));
    assert!(has(&runs, "Episode 1: The Tide Table"));
    assert!(!has(&runs, "Episode 2: Empty"));
    let ep = runs
        .iter()
        .find(|r| r.text == "Episode 1: The Tide Table")
        .unwrap();
    // The bar opens its page: nothing but the header and the bar's own eyebrow above it.
    assert!(body(&runs, ep.page).iter().all(|r| r.y <= ep.y + 30.0));
    // Numbers restart at 1 for the episode board.
    assert!(
        runs.iter()
            .any(|r| r.page == ep.page && r.text == "1" && r.size < 8.0)
    );
    // The outline keeps the boards on one flow (no forced page break).
    let mut args = with("outline", ALL());
    args["scope"] = json!("project");
    let (_, runs, pages) = export(&env, args, "o.pdf");
    assert!(has(&runs, "Episode 1: The Tide Table") && pages >= 3);
}

#[test]
fn the_export_is_deterministic_and_the_nothing_to_export_errors_are_kept() {
    let env = low_tide_env();
    for layout in ["outline", "board"] {
        let (a, _, _) = export(&env, with(layout, ALL()), "a.pdf");
        let (b, _, _) = export(&env, with(layout, ALL()), "b.pdf");
        assert_eq!(std::fs::read(a).unwrap(), std::fs::read(b).unwrap());
    }
    let empty = TestEnv::with_project("Empty", "Feature Film");
    assert_eq!(
        empty.err(
            "story.export_board",
            json!({ "format": "pdf", "layout": "board", "path": empty.dir.path().join("e.pdf").to_string_lossy() })
        ),
        "export.nothing_to_export"
    );
}

#[test]
fn a_2000_character_container_name_and_a_huge_card_end_in_a_bounded_number_of_pages() {
    let env = TestEnv::with_project("Plain", "Short Film");
    let mut s = Seed::new(None);
    s.act("a", &words(2000, "ACT NAME"), None);
    s.seq("q", Some("a"), &words(2000, "Sequence name"), None);
    let huge: String = (1..=4000).map(|i| format!("w{i} ")).collect();
    s.card(
        "c1",
        ("sequence", Some("q")),
        Some("INT. ONE"),
        huge.trim(),
        Some(huge.trim()),
    );
    s.card(
        "c2",
        ("sequence", Some("q")),
        Some("INT. TWO"),
        "Short",
        None,
    );
    exec(&env, &s.sql);
    for layout in ["outline", "board"] {
        let (path, runs, pages) = export(
            &env,
            with(layout, json!({ "includeNotes": true })),
            "big.pdf",
        );
        assert!(pages > 2 && pages < 50, "{layout}: {pages} pages");
        let n = runs
            .iter()
            .flat_map(|r| r.text.split_whitespace())
            .filter(|w| w.starts_with('w') && w[1..].chars().all(|c| c.is_ascii_digit()))
            .count();
        assert_eq!(n, 8000, "{layout}: the description and the note are whole");
        assert!(has(&runs, "INT. TWO"));
        inside_margins(&path);
    }
}
