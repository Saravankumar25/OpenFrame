//! Placeholder media drawn from `media.json`: labelled cards, palettes and storyboard panels, a
//! spoken memo from the macOS `say` tool, and small documents. Nothing comes from the web.

use std::path::Path;

use serde_json::Value;

use crate::gold::{arr, s};
use crate::media::{C, Canvas, sketch, tiny_pdf};

const INK: C = [30, 30, 30];
const PAPER: C = [246, 242, 230];
const WHITE: C = [240, 240, 232];

fn colour(v: &Value) -> C {
    let a = arr(v);
    let n = |i: usize| a.get(i).and_then(Value::as_u64).unwrap_or(128) as u8;
    [n(0), n(1), n(2)]
}

fn card(spec: &Value) -> Canvas {
    let pal = arr(&spec["palette"]);
    let (top, bottom) = (colour(&pal[0]), colour(&pal[1]));
    let mut c = Canvas::new(960, 540, top);
    c.vgrad(0, 540, top, bottom);
    match s(&spec["motif"]).as_str() {
        "sea" => {
            c.disc(740, 250, 46, [255, 226, 160]);
            c.vgrad(300, 540, [70, 130, 160], [20, 60, 90]);
            c.waves(310, 15, [170, 200, 215], 3);
        }
        "road" => {
            c.rect(0, 320, 960, 220, [70, 70, 72]);
            c.line(480, 320, 180, 540, 4, WHITE);
            c.line(480, 320, 780, 540, 4, WHITE);
            c.line(480, 330, 480, 540, 3, [240, 200, 90]);
        }
        "city" => {
            for i in 0..14 {
                let h = 90 + (i * 53) % 170;
                let x = i * 70;
                c.rect(x, 400 - h, 62, h, [(70 + (i * 7) % 40) as u8, 70, 84]);
                c.rect(x + 12, 410 - h, 10, 14, [240, 210, 130]);
            }
            c.rect(0, 400, 960, 140, [60, 60, 64]);
        }
        _ => {
            c.rect(0, 400, 960, 140, [50, 40, 36]);
            c.frame(620, 100, 240, 200, 8, WHITE);
            c.rect(80, 300, 340, 100, [90, 60, 40]);
            c.disc(250, 110, 22, [250, 210, 120]);
        }
    }
    c.rect(0, 448, 960, 92, [20, 20, 24]);
    c.text(24, 458, &s(&spec["title"]), 4, WHITE);
    c.text(24, 502, &s(&spec["subtitle"]), 2, [200, 200, 190]);
    c
}

fn character(spec: &Value) -> Canvas {
    let mut c = Canvas::new(480, 600, [200, 200, 196]);
    c.vgrad(0, 600, [226, 224, 216], [176, 174, 168]);
    c.disc(240, 220, 110, [120, 120, 118]);
    c.disc(240, 620, 220, colour(&spec["shirt"]));
    c.rect(0, 516, 480, 84, [40, 40, 40]);
    c.text(20, 526, &s(&spec["title"]), 3, WHITE);
    c.text(20, 566, &s(&spec["subtitle"]), 2, [200, 200, 190]);
    c
}

fn swatches(spec: &Value) -> Canvas {
    let mut c = Canvas::new(960, 540, PAPER);
    c.text(40, 40, &s(&spec["title"]), 5, INK);
    let cols = arr(&spec["colours"]);
    let w = 880 / cols.len().max(1) as i32;
    for (i, col) in cols.iter().enumerate() {
        c.rect(40 + i as i32 * w, 120, w - 10, 360, colour(col));
    }
    c
}

fn prop(spec: &Value) -> Canvas {
    let mut c = Canvas::new(800, 600, PAPER);
    match s(&spec["motif"]).as_str() {
        "ring" => {
            c.disc(400, 330, 130, [190, 170, 90]);
            c.disc(400, 330, 104, PAPER);
            c.line(360, 196, 400, 150, 8, [150, 200, 230]);
            c.line(440, 196, 400, 150, 8, [150, 200, 230]);
            c.line(360, 196, 440, 196, 8, [150, 200, 230]);
        }
        "lens" => {
            c.rect(250, 220, 300, 180, [40, 40, 44]);
            c.frame(250, 220, 300, 180, 6, [120, 120, 120]);
            c.disc(400, 310, 80, [24, 30, 60]);
            c.disc(380, 290, 18, [140, 170, 220]);
        }
        _ => {
            c.rect(140, 300, 520, 110, [50, 80, 120]);
            c.rect(240, 220, 300, 90, [50, 80, 120]);
            c.rect(260, 236, 120, 60, [170, 200, 220]);
            c.rect(400, 236, 120, 60, [170, 200, 220]);
            c.disc(250, 420, 50, INK);
            c.disc(550, 420, 50, INK);
        }
    }
    c.text(40, 40, &s(&spec["title"]), 4, INK);
    c.text(40, 540, "DEMO PLACEHOLDER", 3, INK);
    c
}

fn voice(text: &str, dest: &Path) {
    let aiff = dest.with_extension("aiff");
    let said = std::process::Command::new("say")
        .arg("-o")
        .arg(&aiff)
        .arg(text)
        .status();
    if matches!(said, Ok(st) if st.success()) {
        let _ = std::process::Command::new("afconvert")
            .args(["-f", "m4af", "-d", "aac"])
            .arg(&aiff)
            .arg(dest)
            .status();
        let _ = std::fs::remove_file(&aiff);
    }
}

/// Write every listed file that is missing, so a rerun keeps the same bytes.
pub fn ensure(specs: &Value, dir: &Path) {
    std::fs::create_dir_all(dir).expect("media folder");
    for spec in arr(specs) {
        let dest = dir.join(s(&spec["file"]));
        if dest.exists() {
            continue;
        }
        let bytes: Vec<u8> = match s(&spec["kind"]).as_str() {
            "card" => card(&spec).png(),
            "character" => character(&spec).png(),
            "swatches" => swatches(&spec).png(),
            "prop" => prop(&spec).png(),
            "sketch" => sketch(
                &s(&spec["caption"]),
                spec["layout"].as_u64().unwrap_or(0) as u8,
            ),
            "pdf" => tiny_pdf(&s(&spec["text"])),
            "text" => s(&spec["text"]).into_bytes(),
            "voice" => {
                voice(&s(&spec["text"]), &dest);
                continue;
            }
            other => panic!("unknown media kind {other}"),
        };
        std::fs::write(&dest, bytes).expect("write media");
    }
}
