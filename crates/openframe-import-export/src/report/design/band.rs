//! A navy group band that heads a table and repeats with it: an eyebrow ("Scene 25"), a heading
//! that wraps, a count on the right ("3 shots", "continued" on later pages), an optional status
//! row and an optional strip of facts. Shot list scenes and other grouped tables use it.

use super::*;

/// What a table prints above its column header, inside the part that repeats on every page.
#[derive(Debug, Clone)]
pub struct TableBand {
    /// Small label before the heading ("Scene 25"); none for a removed scene.
    pub eyebrow: Option<String>,
    pub title: String,
    /// Right end of the band on the first page ("3 shots"); later pages print "Continued".
    pub right: String,
    pub fill: Rgb,
    /// A status sentence under the band: amber for `Tone::Warn`, red for `Tone::Danger`. It
    /// repeats with the band.
    pub status: Option<(String, Tone)>,
    /// Label and value cells of a strip under the band, on the first page only.
    pub strip: Vec<(String, String)>,
}

impl TableBand {
    pub fn new(title: impl Into<String>) -> TableBand {
        TableBand {
            eyebrow: None,
            title: title.into(),
            right: String::new(),
            fill: pal::NAVY,
            status: None,
            strip: Vec::new(),
        }
    }
}

const EYEBROW_COLOR: Rgb = Rgb::hex(0xf5c27a);
const LINE_STRONG: Rgb = Rgb::hex(0xb9c2d1);
const TITLE_SIZE: f32 = 14.0;
const TITLE_LEAD: f32 = 16.1;
const MIN_H: f32 = 34.0;
const STATUS_H: f32 = 22.0;

pub(super) fn band_frag(b: &TableBand, cw: f32, continued: bool) -> Frag {
    let mut f = Frag::new();
    let right = if continued {
        "CONTINUED".to_string()
    } else {
        b.right.to_uppercase()
    };
    let right_w = if right.is_empty() {
        0.0
    } else {
        tw(&right, 9.0, false, 0.5).min(cw * 0.42)
    };
    let mut x = 14.0f32;
    let eyebrow = b.eyebrow.as_ref().map(|e| e.to_uppercase());
    if let Some(e) = &eyebrow {
        x += tw(e, 8.0, true, 1.4) + 10.0;
    }
    let title = Txt::new(b.title.clone())
        .size(TITLE_SIZE)
        .bold()
        .color(pal::WHITE)
        .leading(1.15);
    let avail = (cw - right_w - 28.0 - x).max(80.0);
    let lines = txt_lines(&title, avail);
    let band_h = (lines.len() as f32 * TITLE_LEAD + 12.0).max(MIN_H);
    f.shape_c(0.0, 0.0, cw, band_h, 6.0, true, false, Some(b.fill), None);
    let first = (band_h - lines.len() as f32 * TITLE_LEAD) / 2.0 + 12.4;
    if let Some(e) = &eyebrow {
        f.text(14.0, first - 1.4, e, 8.0, true, EYEBROW_COLOR, 1.4);
    }
    for (k, l) in lines.iter().enumerate() {
        f.text(
            x,
            first + k as f32 * TITLE_LEAD,
            l,
            TITLE_SIZE,
            true,
            pal::WHITE,
            0.0,
        );
    }
    if !right.is_empty() {
        f.text(
            cw - 14.0 - tw(&right, 9.0, false, 0.5),
            first - 1.0,
            &right,
            9.0,
            false,
            pal::WHITE,
            0.5,
        );
    }
    let mut y = band_h;
    if let Some((text, tone)) = &b.status {
        f.shape_c(
            0.0,
            y,
            cw,
            STATUS_H,
            0.0,
            false,
            false,
            Some(tone.fill()),
            None,
        );
        let stripe = if *tone == Tone::Warn {
            pal::ORANGE
        } else {
            tone.text()
        };
        f.shape(0.0, y, 3.0, STATUS_H, 0.0, Some(stripe), None);
        f.text(
            14.0,
            y + STATUS_H / 2.0 + 3.0,
            text,
            8.6,
            true,
            tone.text(),
            0.0,
        );
        y += STATUS_H;
    }
    if !continued && !b.strip.is_empty() {
        y += strip(&mut f, &b.strip, cw, y);
    }
    f.h = y;
    f
}

/// Equal cells with a small label over a bold value that wraps; returns the height.
fn strip(f: &mut Frag, cells: &[(String, String)], cw: f32, top: f32) -> f32 {
    let n = cells.len() as f32;
    let cell_w = cw / n;
    let value = |v: &str| {
        Txt::new(v.to_string())
            .size(8.6)
            .bold()
            .color(pal::NAVY)
            .leading(1.3)
    };
    let h = cells
        .iter()
        .map(|(_, v)| 6.0 + 7.5 + 3.0 + txt_height(&value(v), cell_w - 20.0) + 7.0)
        .fold(0.0, f32::max);
    f.shape_c(0.0, top, cw, h, 0.0, false, false, Some(pal::PANEL), None);
    for (i, (label, v)) in cells.iter().enumerate() {
        let x = i as f32 * cell_w;
        if i > 0 {
            f.line(x, top, x, top + h, pal::LINE, 0.5);
        }
        f.text(
            x + 10.0,
            top + 6.0 + 6.8,
            &label.to_uppercase(),
            6.8,
            true,
            pal::SLATE,
            1.1,
        );
        put_txt(f, x + 10.0, top + 6.0 + 7.5 + 3.0, cell_w - 20.0, &value(v));
    }
    f.line(0.0, top + h, cw, top + h, LINE_STRONG, 0.5);
    h
}

pub(super) fn band_scan(b: &TableBand, scan: &mut dyn FnMut(&str)) {
    scan(&b.title);
    scan(&b.right);
    if let Some(e) = &b.eyebrow {
        scan(e);
    }
    if let Some((s, _)) = &b.status {
        scan(s);
    }
    for (l, v) in &b.strip {
        scan(l);
        scan(v);
    }
}
