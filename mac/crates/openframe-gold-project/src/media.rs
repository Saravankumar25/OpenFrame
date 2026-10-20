//! Real local media for the gold project: frames drawn in code (no stock images, nothing
//! downloaded), a spoken voice memo made with the macOS `say` and `afconvert` tools, and two small
//! documents. Everything is written once into `tests/gold-project/content/media` and reused.

use std::path::{Path, PathBuf};

use image::{Rgb, RgbImage};

type C = [u8; 3];

const NIGHT: C = [12, 20, 38];
const SEA: C = [22, 58, 84];
const SAND: C = [214, 190, 142];
const RUST: C = [150, 72, 40];
const CONCRETE: C = [150, 150, 140];
const LAMP: C = [250, 196, 90];
const WHITE: C = [240, 240, 232];
const INK: C = [30, 30, 30];
const PAPER: C = [246, 242, 230];
const LEAF: C = [40, 96, 52];

struct Canvas(RgbImage);

impl Canvas {
    fn new(w: u32, h: u32, c: C) -> Self {
        Canvas(RgbImage::from_pixel(w, h, Rgb(c)))
    }
    fn w(&self) -> i32 {
        self.0.width() as i32
    }
    fn h(&self) -> i32 {
        self.0.height() as i32
    }
    fn px(&mut self, x: i32, y: i32, c: C) {
        if x >= 0 && y >= 0 && x < self.w() && y < self.h() {
            self.0.put_pixel(x as u32, y as u32, Rgb(c));
        }
    }
    fn vgrad(&mut self, y0: i32, y1: i32, a: C, b: C) {
        for y in y0.max(0)..y1.min(self.h()) {
            let t = (y - y0) as f32 / (y1 - y0).max(1) as f32;
            let c = mix(a, b, t);
            for x in 0..self.w() {
                self.px(x, y, c);
            }
        }
    }
    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: C) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.px(xx, yy, c);
            }
        }
    }
    fn frame(&mut self, x: i32, y: i32, w: i32, h: i32, t: i32, c: C) {
        self.rect(x, y, w, t, c);
        self.rect(x, y + h - t, w, t, c);
        self.rect(x, y, t, h, c);
        self.rect(x + w - t, y, t, h, c);
    }
    fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, t: i32, c: C) {
        let n = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
        for i in 0..=n {
            let x = x0 + (x1 - x0) * i / n;
            let y = y0 + (y1 - y0) * i / n;
            self.rect(x - t / 2, y - t / 2, t.max(1), t.max(1), c);
        }
    }
    fn disc(&mut self, cx: i32, cy: i32, r: i32, c: C) {
        for y in -r..=r {
            for x in -r..=r {
                if x * x + y * y <= r * r {
                    self.px(cx + x, cy + y, c);
                }
            }
        }
    }
    fn waves(&mut self, y0: i32, rows: i32, c: C, seed: i32) {
        for r in 0..rows {
            let y = y0 + r * 14;
            let mut x = -((seed * 7 + r * 13) % 40);
            while x < self.w() {
                self.line(x, y + 4, x + 12, y, 2, c);
                self.line(x + 12, y, x + 24, y + 4, 2, c);
                x += 36;
            }
        }
    }
    /// Upper-case label in a 5x7 pixel font, scaled.
    fn text(&mut self, x: i32, y: i32, s: &str, scale: i32, c: C) {
        let mut cx = x;
        for ch in s.chars() {
            let g = glyph(ch.to_ascii_uppercase());
            for (row, bits) in g.iter().enumerate() {
                for col in 0..5 {
                    if bits & (1 << (4 - col)) != 0 {
                        self.rect(cx + col * scale, y + row as i32 * scale, scale, scale, c);
                    }
                }
            }
            cx += 6 * scale;
        }
    }
    fn png(&self) -> Vec<u8> {
        let mut out = std::io::Cursor::new(Vec::new());
        self.0
            .write_to(&mut out, image::ImageFormat::Png)
            .expect("png");
        out.into_inner()
    }
}

fn mix(a: C, b: C, t: f32) -> C {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    [m(a[0], b[0]), m(a[1], b[1]), m(a[2], b[2])]
}

fn glyph(c: char) -> [u8; 7] {
    match c {
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'G' => [14, 17, 16, 23, 17, 17, 15],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [14, 4, 4, 4, 4, 4, 14],
        'J' => [7, 2, 2, 2, 2, 18, 12],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 17, 25, 21, 19, 17, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'Q' => [14, 17, 17, 17, 21, 18, 13],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'W' => [17, 17, 17, 21, 21, 21, 10],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 30, 1, 1, 17, 14],
        '6' => [6, 8, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 2, 12],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        '.' => [0, 0, 0, 0, 0, 12, 12],
        ':' => [0, 12, 12, 0, 12, 12, 0],
        '/' => [1, 1, 2, 4, 8, 16, 16],
        '\'' => [4, 4, 8, 0, 0, 0, 0],
        _ => [0; 7],
    }
}

/// The coast radio station: concrete block, rusted mast, sky.
fn station(sky_top: C, sky_bottom: C, lit: bool) -> Canvas {
    let mut c = Canvas::new(960, 540, sky_top);
    c.vgrad(0, 400, sky_top, sky_bottom);
    c.rect(0, 400, 960, 140, SAND);
    for i in 0..12 {
        let x = 30 + i * 80;
        c.line(x, 400, x + 18, 300, 5, LEAF);
        c.disc(x + 18, 296, 22, LEAF);
    }
    c.rect(300, 280, 300, 130, CONCRETE);
    c.rect(330, 310, 50, 40, if lit { LAMP } else { INK });
    c.rect(420, 310, 50, 40, if lit { LAMP } else { INK });
    c.rect(510, 330, 50, 80, INK);
    // Lattice mast.
    c.line(640, 410, 690, 60, 4, RUST);
    c.line(740, 410, 690, 60, 4, RUST);
    let mut y = 380;
    while y > 80 {
        let half = (y - 60) * 50 / 350;
        c.line(690 - half, y, 690 + half, y - 30, 2, RUST);
        c.line(690 + half, y, 690 - half, y - 30, 2, RUST);
        y -= 30;
    }
    c.disc(690, 58, 6, [220, 40, 40]);
    c
}

fn sea_storm(seed: i32) -> Canvas {
    let mut c = Canvas::new(960, 540, NIGHT);
    c.vgrad(0, 260, [6, 8, 16], [30, 40, 60]);
    c.vgrad(260, 540, [20, 40, 60], [4, 12, 24]);
    c.waves(270, 18, [90, 120, 140], seed);
    // Boat with a lantern.
    let bx = 380 + seed * 40;
    c.line(bx, 330, bx + 160, 330, 6, [80, 50, 30]);
    c.line(bx + 20, 350, bx + 140, 350, 6, [80, 50, 30]);
    c.line(bx + 80, 330, bx + 80, 240, 3, [80, 50, 30]);
    c.disc(bx + 80, 236, 8, LAMP);
    for i in 0..60 {
        let x = (i * 53 + seed * 17) % 960;
        let y = (i * 31) % 260;
        c.line(x, y, x - 10, y + 30, 1, [120, 140, 170]);
    }
    c
}

fn harbour_dawn() -> Canvas {
    let mut c = Canvas::new(960, 540, SEA);
    c.vgrad(0, 300, [250, 170, 120], [140, 170, 200]);
    c.disc(760, 290, 44, [255, 220, 150]);
    c.vgrad(300, 540, [70, 110, 140], [20, 50, 70]);
    c.waves(310, 14, [150, 180, 200], 3);
    for i in 0..5 {
        let x = 60 + i * 170;
        c.line(x, 380, x + 110, 380, 8, [30, 30, 40]);
        c.line(x + 50, 380, x + 50, 300, 3, [30, 30, 40]);
    }
    c
}

fn beach_lanterns() -> Canvas {
    let mut c = Canvas::new(960, 540, NIGHT);
    c.vgrad(0, 300, [10, 14, 30], [30, 30, 60]);
    c.rect(0, 300, 960, 240, [60, 52, 40]);
    for i in 0..16 {
        let x = 30 + i * 60;
        let y = 150 + ((i * 37) % 40);
        c.line(x, y, x + 60, 150 + (((i + 1) * 37) % 40), 1, [90, 90, 90]);
        c.disc(x, y + 8, 7, LAMP);
    }
    for i in 0..40 {
        let x = 20 + i * 24;
        let h = 30 + (i * 7) % 20;
        c.rect(x, 420 - h, 14, h, [20, 18, 16]);
        c.disc(x + 7, 420 - h - 8, 7, [20, 18, 16]);
    }
    c.rect(420, 330, 120, 60, [40, 40, 40]);
    c.text(426, 350, "PA", 4, WHITE);
    c
}

fn operations_room_1999() -> Canvas {
    let mut c = Canvas::new(960, 540, [60, 52, 40]);
    c.vgrad(0, 540, [70, 60, 46], [30, 26, 20]);
    c.rect(80, 300, 800, 40, [90, 60, 36]);
    c.rect(120, 160, 260, 140, [50, 60, 50]);
    for i in 0..6 {
        c.disc(150 + i * 40, 200, 12, [200, 200, 180]);
    }
    c.rect(150, 240, 200, 30, [20, 30, 20]);
    c.text(160, 248, "VHF 16", 3, [120, 255, 120]);
    c.rect(520, 140, 200, 160, [70, 70, 70]);
    c.disc(570, 210, 34, INK);
    c.disc(670, 210, 34, INK);
    c.disc(570, 210, 8, [160, 160, 160]);
    c.disc(670, 210, 8, [160, 160, 160]);
    c.disc(800, 260, 20, LAMP);
    c.text(540, 110, "14-11-1999", 3, WHITE);
    c
}

fn swatches(title: &str, colours: &[C]) -> Canvas {
    let mut c = Canvas::new(960, 540, PAPER);
    c.text(40, 40, title, 5, INK);
    let w = 880 / colours.len() as i32;
    for (i, col) in colours.iter().enumerate() {
        c.rect(40 + i as i32 * w, 120, w - 10, 360, *col);
    }
    c
}

fn headshot(name: &str, shirt: C, skin: C) -> Canvas {
    let mut c = Canvas::new(480, 600, [200, 200, 196]);
    c.vgrad(0, 600, [220, 220, 214], [170, 170, 166]);
    c.disc(240, 230, 110, skin);
    c.rect(130, 120, 220, 50, INK);
    c.disc(240, 640, 220, shirt);
    c.rect(0, 540, 480, 60, [40, 40, 40]);
    c.text(20, 556, name, 4, WHITE);
    c
}

fn prop(kind: &str) -> Canvas {
    let mut c = Canvas::new(800, 600, PAPER);
    match kind {
        "reel" => {
            c.disc(400, 300, 220, [60, 60, 64]);
            c.disc(400, 300, 200, [120, 80, 50]);
            c.disc(400, 300, 70, [180, 180, 180]);
            for a in 0..3 {
                let (dx, dy) = [(0, -40), (35, 20), (-35, 20)][a];
                c.disc(400 + dx, 300 + dy, 14, PAPER);
            }
            c.rect(280, 520, 240, 50, WHITE);
            c.text(292, 534, "14-11-1999", 3, INK);
        }
        "handset" => {
            c.rect(300, 120, 200, 360, [30, 30, 30]);
            c.rect(330, 160, 140, 80, [60, 80, 60]);
            c.disc(400, 380, 40, [80, 80, 80]);
            c.line(500, 300, 640, 460, 6, [30, 30, 30]);
        }
        _ => {
            // The Ambassador car, side view.
            c.rect(140, 300, 520, 110, [235, 235, 225]);
            c.rect(240, 220, 300, 90, [235, 235, 225]);
            c.rect(260, 236, 120, 60, [120, 150, 170]);
            c.rect(400, 236, 120, 60, [120, 150, 170]);
            c.disc(250, 420, 50, INK);
            c.disc(550, 420, 50, INK);
            c.text(260, 500, "AMBASSADOR KL-07", 3, INK);
        }
    }
    c
}

/// A storyboard sketch: ink on paper, with the panel caption.
pub fn sketch(caption: &str, layout: u8) -> Vec<u8> {
    let mut c = Canvas::new(960, 540, PAPER);
    c.frame(10, 10, 940, 520, 4, INK);
    match layout {
        0 => {
            // Wide: breakwater running into a wave.
            c.line(20, 420, 700, 300, 6, INK);
            c.line(20, 470, 700, 330, 6, INK);
            for i in 0..14 {
                let x = 60 + i * 48;
                c.frame(x, 400 - i * 8, 30, 30, 2, INK);
            }
            c.line(700, 300, 900, 120, 3, INK);
            c.line(900, 120, 940, 260, 3, INK);
            c.line(760, 200, 840, 160, 2, INK);
        }
        1 => {
            // Medium: a figure with a torch at the edge.
            c.disc(480, 200, 30, INK);
            c.line(480, 230, 480, 360, 6, INK);
            c.line(480, 260, 560, 200, 5, INK);
            c.disc(570, 190, 12, INK);
            c.line(480, 360, 440, 460, 5, INK);
            c.line(480, 360, 520, 460, 5, INK);
            c.line(560, 200, 900, 120, 1, INK);
        }
        2 => {
            // Close: hands on a reel.
            c.disc(480, 280, 180, INK);
            c.disc(480, 280, 170, PAPER);
            c.disc(480, 280, 50, INK);
            c.line(200, 500, 360, 340, 14, INK);
            c.line(760, 500, 600, 340, 14, INK);
        }
        _ => {
            // Crowd on the beach facing a speaker stack.
            for i in 0..18 {
                let x = 60 + i * 48;
                c.disc(x, 380, 14, INK);
                c.line(x, 394, x, 460, 4, INK);
            }
            c.frame(420, 140, 120, 160, 4, INK);
            c.disc(480, 190, 30, INK);
            c.disc(480, 260, 20, INK);
        }
    }
    c.rect(10, 480, 940, 50, WHITE);
    c.text(24, 492, caption, 4, INK);
    c.png()
}

pub struct Media {
    pub dir: PathBuf,
}

impl Media {
    pub fn path(&self, name: &str) -> String {
        self.dir.join(name).to_string_lossy().into_owned()
    }
}

fn write(dir: &Path, name: &str, bytes: &[u8]) {
    let p = dir.join(name);
    if !p.exists() {
        std::fs::write(&p, bytes).expect("write media");
    }
}

/// Create every media file the project uses (kept when already there, so hashes are stable).
pub fn ensure(dir: &Path) -> Media {
    std::fs::create_dir_all(dir).expect("media dir");
    let frames: Vec<(&str, Canvas)> = vec![
        (
            "station-dusk.png",
            station([30, 40, 80], [240, 140, 90], true),
        ),
        (
            "station-day.png",
            station([120, 170, 220], [220, 230, 240], false),
        ),
        (
            "station-night.png",
            station([4, 6, 14], [20, 26, 50], false),
        ),
        ("sea-storm-1999.png", sea_storm(1)),
        ("sea-storm-turn.png", sea_storm(3)),
        ("harbour-dawn.png", harbour_dawn()),
        ("beach-lanterns.png", beach_lanterns()),
        ("operations-room-1999.png", operations_room_1999()),
        (
            "palette-monsoon.png",
            swatches(
                "MONSOON PALETTE",
                &[NIGHT, SEA, [90, 120, 140], RUST, SAND, LAMP],
            ),
        ),
        (
            "palette-1999.png",
            swatches(
                "1999 PALETTE",
                &[
                    [110, 90, 60],
                    [70, 60, 46],
                    [50, 60, 50],
                    LAMP,
                    [200, 200, 180],
                ],
            ),
        ),
        (
            "cast-anjali.png",
            headshot("ANJALI", [60, 70, 110], [176, 124, 92]),
        ),
        (
            "cast-appachan.png",
            headshot("APPACHAN", [236, 236, 230], [150, 104, 76]),
        ),
        (
            "cast-mariamma.png",
            headshot("MARIAMMA", [120, 40, 60], [170, 118, 88]),
        ),
        (
            "cast-fathima.png",
            headshot("FATHIMA", [40, 100, 90], [168, 120, 90]),
        ),
        (
            "cast-suresh.png",
            headshot("SURESH", [200, 160, 60], [140, 98, 70]),
        ),
        (
            "cast-joe.png",
            headshot("JOE", [90, 90, 90], [160, 110, 80]),
        ),
        (
            "cast-rahul.png",
            headshot("RAHUL", [200, 60, 50], [160, 112, 84]),
        ),
        (
            "cast-latha.png",
            headshot("LATHA", [140, 120, 80], [166, 116, 88]),
        ),
        ("prop-reel.png", prop("reel")),
        ("prop-handset.png", prop("handset")),
        ("vehicle-ambassador.png", prop("car")),
    ];
    for (name, c) in frames {
        write(dir, name, &c.png());
    }
    write(
        dir,
        "board-breakwater-01.png",
        &sketch("24 - WIDE BREAKWATER", 0),
    );
    write(
        dir,
        "board-breakwater-02.png",
        &sketch("24 - APPACHAN AT THE EDGE", 1),
    );
    write(
        dir,
        "board-reel-01.png",
        &sketch("33 - HANDS ON THE REEL", 2),
    );
    write(
        dir,
        "board-beach-01.png",
        &sketch("39 - LISTENING NIGHT", 3),
    );
    // A one-page research PDF and a location release form, written by hand (valid PDF 1.4).
    write(
        dir,
        "research-coast-radio-1999.pdf",
        &tiny_pdf("Coast radio procedures, Kerala 1999. Notes from the harbour office archive."),
    );
    write(
        dir,
        "location-release-coast-radio-station.pdf",
        &tiny_pdf(
            "Location release: Coast Radio Station, Kadalkkara. Port Trust permit KP-PT-2027-118.",
        ),
    );
    write(
        dir,
        "music-cue-sheet.csv",
        b"cue,scene,title,source,duration\n1M1,1,Storm Theme,Score,01:40\n2M3,18,Ave Maris Stella (choir),Live recording,02:10\n4M1,39,Listening Night,Score,03:05\n4M4,44,Low Tide (end titles),Score,04:20\n",
    );
    write(
        dir,
        "low-tide-radio-theme.json",
        br##"{
  "formatVersion": 1,
  "id": "low-tide-night",
  "name": "Low Tide Night",
  "description": "Night-sea surfaces with a lantern accent, for the Low Tide Radio grade sessions.",
  "author": "Kayal Pictures art department",
  "seeds": { "neutralHue": 235, "neutralTint": 0.03, "accentHue": 70, "accentChroma": 0.15, "accentLightnessNudge": 0 },
  "overrides": { "dark": { "accent": "oklch(76% 0.15 70)" } }
}"##,
    );
    voice_memo(dir);
    Media {
        dir: dir.to_path_buf(),
    }
}

/// The director's voice memo for the Idea Vault, spoken by the system voice.
fn voice_memo(dir: &Path) {
    let m4a = dir.join("director-memo-breakwater.m4a");
    if m4a.exists() {
        return;
    }
    let aiff = dir.join("director-memo-breakwater.aiff");
    let text = "Memo for the breakwater scene. I want the storm to feel close and cold. \
                One wide from the harbour lights, then stay on Suresh's hands on the tetrapods. \
                No music until he has Appachan in his arms. Only wind, rain and the dog.";
    let said = std::process::Command::new("say")
        .arg("-o")
        .arg(&aiff)
        .arg(text)
        .status();
    if matches!(said, Ok(s) if s.success()) {
        let _ = std::process::Command::new("afconvert")
            .args(["-f", "m4af", "-d", "aac"])
            .arg(&aiff)
            .arg(&m4a)
            .status();
        let _ = std::fs::remove_file(&aiff);
    }
}

pub fn tiny_pdf(text: &str) -> Vec<u8> {
    let content = format!(
        "BT /F1 12 Tf 72 720 Td ({}) Tj ET",
        text.replace('(', "[").replace(')', "]")
    );
    let objs = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_string(),
        format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), content),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ];
    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (i, o) in objs.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", i + 1, o).as_bytes());
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes());
    for o in offsets {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objs.len() + 1
        )
        .as_bytes(),
    );
    out
}
