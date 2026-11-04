//! The storyboard panel card: a navy number tile with "Panel · Shot 25A", a 16:9 picture frame
//! with a navy film-frame border (the picture is fitted inside and never cropped), chips for
//! framing, movement and angle, and a tinted caption band with bold labels. The band grows to
//! the height of the tallest card in its row so the bands line up.

use super::rich::{Rich, Run, put_rich, rich_height};
use super::*;

/// One panel of a storyboard sheet.
#[derive(Debug, Clone)]
pub struct Panel {
    pub number: String,
    /// "Shot 25A" when the panel is linked to a shot.
    pub shot: Option<String>,
    pub img: Option<ReportImage>,
    /// Printed in the frame when there is no picture.
    pub placeholder: Option<String>,
    pub chips: Vec<String>,
    /// The action line (unlabelled).
    pub action: Option<String>,
    /// Labelled caption lines ("Sound:", "Duration:").
    pub lines: Vec<(String, String)>,
    /// The shot note, set apart under a hairline.
    pub note: Option<(String, String)>,
}

impl Panel {
    pub fn new(number: impl Into<String>) -> Panel {
        Panel {
            number: number.into(),
            shot: None,
            img: None,
            placeholder: None,
            chips: Vec::new(),
            action: None,
            lines: Vec::new(),
            note: None,
        }
    }
}

const PAD: f32 = 6.0;
const GAP: f32 = 4.0;
const NEUTRAL: Rgb = Rgb::hex(0x3b4659);
const LINE_STRONG: Rgb = Rgb::hex(0xb9c2d1);
const FRAME_FILL: Rgb = Rgb::hex(0xe6ebf2);
const FRAME_BORDER: f32 = 1.4;

fn caption(p: &Panel, dense: bool) -> (Option<Txt>, Option<Rich>, Option<Rich>) {
    let small = if dense { 7.8 } else { 8.2 };
    let action = p.action.as_ref().map(|a| {
        Txt::new(a.clone())
            .size(if dense { 8.6 } else { 9.0 })
            .color(pal::INK)
            .leading(1.28)
    });
    let labelled = |pairs: &[(String, String)], color: Rgb| {
        let mut r = Rich::new(small).gap(1.5);
        for (l, v) in pairs {
            r = r.para(vec![
                Run::new(l.clone(), true, pal::NAVY),
                Run::new(v.clone(), false, color),
            ]);
        }
        r
    };
    let lines = (!p.lines.is_empty()).then(|| labelled(&p.lines, pal::SLATE));
    let note = p
        .note
        .as_ref()
        .map(|n| labelled(std::slice::from_ref(n), NEUTRAL));
    (action, lines, note)
}

/// Where one chip sits: position, size and its text lines.
struct ChipPlace {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    lines: Vec<String>,
}

/// Chips flowing left to right; returns the placements and the height.
fn chip_layout(chips: &[String], iw: f32, size: f32) -> (Vec<ChipPlace>, f32) {
    let (px, py, gap) = (4.6f32, 2.6f32, 2.5f32);
    let mut out = Vec::new();
    let (mut x, mut y, mut row_h) = (0.0f32, 0.0f32, 0.0f32);
    for c in chips {
        let one = tw(c, size, true, 0.0) + 2.0 * px;
        let (lines, w) = if one <= iw {
            (vec![c.clone()], one)
        } else {
            (wrap_pt(c, iw - 2.0 * px, size, true), iw)
        };
        let h = lines.len() as f32 * size * 1.2 + 2.0 * py + 1.0;
        if x > 0.0 && x + w > iw {
            y += row_h + gap;
            x = 0.0;
            row_h = 0.0;
        }
        out.push(ChipPlace { x, y, w, h, lines });
        x += w + gap;
        row_h = row_h.max(h);
    }
    (out, y + row_h)
}

/// The panel as one unit of a card `w` wide. `fill_to` is the height the card has to reach (the
/// tallest card of its row): the caption band takes the difference.
pub(super) fn panel_frag<'a>(
    p: &mut Pager<'a>,
    panel: &'a Panel,
    w: f32,
    fill_to: Option<f32>,
) -> Frag {
    let dense = w < 200.0;
    let iw = w - 2.0 * PAD;
    let mut f = Frag::new();
    let mut y = PAD;

    // Head: number tile, "Panel", "· Shot 25A".
    let num_size = 9.0;
    let tile_w = (tw(&panel.number, num_size, true, 0.0) + 8.0).max(15.0);
    let tile_h = 15.0;
    let label = "Panel";
    let label_w = tw(label, 7.6, false, 0.0);
    let shot_size = if dense { 8.6 } else { 9.0 };
    let x0 = PAD + tile_w + 6.0;
    let mut shot_lines: Vec<String> = Vec::new();
    let mut shot_on_first = true;
    if let Some(s) = &panel.shot {
        let text = format!("· {s}");
        let room = iw - tile_w - 6.0 - label_w - 4.0;
        if tw(&text, shot_size, true, 0.0) <= room {
            shot_lines.push(text);
        } else {
            shot_on_first = false;
            shot_lines = wrap_pt(&text, iw - tile_w - 6.0, shot_size, true);
        }
    }
    f.shape(PAD, y, tile_w, tile_h, 3.5, Some(pal::NAVY), None);
    f.text(
        PAD + (tile_w - tw(&panel.number, num_size, true, 0.0)) / 2.0,
        y + 10.6,
        &panel.number,
        num_size,
        true,
        pal::WHITE,
        0.0,
    );
    f.text(x0, y + 10.4, label, 7.6, false, pal::SLATE, 0.0);
    let mut head_h = tile_h;
    if shot_on_first {
        if let Some(t) = shot_lines.first() {
            f.text(
                x0 + label_w + 4.0,
                y + 10.6,
                t,
                shot_size,
                true,
                pal::TEAL,
                0.0,
            );
        }
    } else {
        for (k, l) in shot_lines.iter().enumerate() {
            let base = y + 10.4 + 11.0 + k as f32 * shot_size * 1.2;
            f.text(x0, base, l, shot_size, true, pal::TEAL, 0.0);
            head_h = head_h.max(base - y + 3.0);
        }
    }
    y += head_h + GAP;

    // Picture frame.
    let fh = iw * 9.0 / 16.0;
    f.shape(PAD, y, iw, fh, 2.5, Some(FRAME_FILL), None);
    match &panel.img {
        Some(img) => {
            let scale = (iw / img.width as f32).min(fh / img.height as f32);
            let (dw, dh) = (img.width as f32 * scale, img.height as f32 * scale);
            let index = p.image_index(img);
            f.ops.push(Op::Image {
                index,
                x: PAD + (iw - dw) / 2.0,
                y: -(y + (fh + dh) / 2.0),
                w: dw,
                h: dh,
            });
        }
        None => {
            if let Some(label) = &panel.placeholder {
                let t = Txt::new(label.clone())
                    .size(8.2)
                    .color(NEUTRAL)
                    .align(Align::Center);
                let th = txt_height(&t, iw - 16.0);
                put_txt(
                    &mut f,
                    PAD + 8.0,
                    y + ((fh - th) / 2.0).max(2.0),
                    iw - 16.0,
                    &t,
                );
            }
        }
    }
    f.shape(PAD, y, iw, fh, 2.5, None, Some((pal::NAVY, FRAME_BORDER)));
    y += fh;

    // Chips.
    if !panel.chips.is_empty() {
        y += GAP;
        let size = if dense { 7.2 } else { 7.6 };
        let (places, h) = chip_layout(&panel.chips, iw, size);
        for ChipPlace {
            x: cx,
            y: cy,
            w: cw,
            h: ch,
            lines,
        } in places
        {
            f.shape(
                PAD + cx,
                y + cy,
                cw,
                ch,
                3.0,
                Some(pal::TINT),
                Some((LINE_STRONG, 0.5)),
            );
            for (k, l) in lines.iter().enumerate() {
                f.text(
                    PAD + cx + 4.6,
                    y + cy + 2.6 + size * 0.95 + k as f32 * size * 1.2,
                    l,
                    size,
                    true,
                    pal::NAVY,
                    0.0,
                );
            }
        }
        y += h;
    }

    // Caption band.
    let (action, lines, note) = caption(panel, dense);
    let inner = iw - 12.0;
    let mut content = 0.0f32;
    if let Some(a) = &action {
        content += txt_height(a, inner);
    }
    if let Some(l) = &lines {
        if content > 0.0 {
            content += 2.5;
        }
        content += rich_height(l, inner);
    }
    if let Some(n) = &note {
        if content > 0.0 {
            content += 6.0;
        }
        content += rich_height(n, inner);
    }
    let has_band = action.is_some() || lines.is_some() || note.is_some();
    let mut band_h = if has_band { 4.5 + content + 5.0 } else { 0.0 };
    let below = PAD;
    if has_band {
        y += GAP;
    }
    if let Some(target) = fill_to {
        // The band takes whatever height the tallest card of the row has more.
        let used = y + band_h + below;
        if has_band && target > used {
            band_h += target - used;
        }
    }
    if has_band {
        f.shape(
            PAD,
            y,
            iw,
            band_h,
            4.0,
            Some(pal::PANEL),
            Some((pal::LINE, 0.5)),
        );
        let mut cy = y + 4.5;
        if let Some(a) = &action {
            cy += put_txt(&mut f, PAD + 6.0, cy, inner, a);
        }
        if let Some(l) = &lines {
            if action.is_some() {
                cy += 2.5;
            }
            cy += put_rich(&mut f, PAD + 6.0, cy, inner, l);
        }
        if let Some(n) = &note {
            if action.is_some() || lines.is_some() {
                f.line(
                    PAD + 6.0,
                    cy + 2.0,
                    PAD + iw - 6.0,
                    cy + 2.0,
                    pal::LINE,
                    0.5,
                );
                cy += 6.0;
            }
            put_rich(&mut f, PAD + 6.0, cy, inner, n);
        }
        y += band_h;
    }
    f.h = y + below;
    f
}

pub(super) fn panel_scan(p: &Panel, scan: &mut dyn FnMut(&str)) {
    scan(&p.number);
    for s in p.shot.iter().chain(&p.placeholder).chain(&p.action) {
        scan(s);
    }
    p.chips.iter().for_each(|c| scan(c));
    for (l, v) in p.lines.iter().chain(p.note.iter()) {
        scan(l);
        scan(v);
    }
}

// ------------------------------------------------------------------ scene banner

/// The band that opens a storyboard: a navy block with an orange accent bar, the storyboard name,
/// its scene line under it, the panel count and a status pill on the right.
#[derive(Debug, Clone)]
pub struct SceneBanner {
    pub eyebrow: String,
    pub title: String,
    pub scene_line: String,
    pub count: String,
    pub status: Option<(String, Tone)>,
}

const ACCENT_W: f32 = 5.0;

pub(in crate::report) fn scene_banner(p: &mut Pager, b: &SceneBanner) {
    let cw = p.content_w();
    let mut f = Frag::new();
    let left_x = ACCENT_W + 10.0;
    let count = b.count.to_uppercase();
    let count_w = tw(&count, 8.2, false, 0.6);
    let max_right = cw * 0.42;
    let pill = b
        .status
        .as_ref()
        .map(|(t, tone)| Chip::pill(t.clone(), *tone).small().bold());
    let pill_dims = pill.as_ref().map(|c| chip_dims(c, max_right));
    let right_w = pill_dims
        .as_ref()
        .map_or(count_w, |(_, w, _)| w.max(count_w));
    let avail = (cw - left_x - 12.0 - right_w - 16.0).max(80.0);
    let name = Txt::new(b.title.clone()).size(12.5).bold().leading(1.2);
    let name_lines = txt_lines(&name, avail);
    let scene = Txt::new(b.scene_line.clone())
        .size(8.2)
        .bold()
        .leading(1.25);
    let scene_lines = txt_lines(&scene, avail);
    let left_h = 6.0
        + 8.2
        + 2.0
        + name_lines.len() as f32 * 15.0
        + 2.0
        + scene_lines.len() as f32 * 10.25
        + 7.0;
    let right_h = 6.0 + 10.0 + pill_dims.as_ref().map_or(0.0, |(_, _, h)| h + 4.0) + 7.0;
    let h = left_h.max(right_h).max(40.0);
    f.shape(0.0, 0.0, cw, h, 6.0, Some(pal::NAVY), None);
    // The accent bar: an orange rounded block whose right part the navy covers again.
    f.shape(0.0, 0.0, ACCENT_W + 8.0, h, 6.0, Some(pal::ORANGE), None);
    f.shape(ACCENT_W, 0.0, 8.0, h, 0.0, Some(pal::NAVY), None);
    let mut y = 6.0;
    f.text(
        left_x,
        y + 6.8,
        &b.eyebrow.to_uppercase(),
        6.8,
        true,
        Rgb::hex(0xf5c27a),
        1.0,
    );
    y += 8.2 + 2.0;
    for (k, l) in name_lines.iter().enumerate() {
        f.text(
            left_x,
            y + 11.2 + k as f32 * 15.0,
            l,
            12.5,
            true,
            pal::WHITE,
            0.0,
        );
    }
    y += name_lines.len() as f32 * 15.0 + 2.0;
    for (k, l) in scene_lines.iter().enumerate() {
        f.text(
            left_x,
            y + 7.4 + k as f32 * 10.25,
            l,
            8.2,
            true,
            Rgb::hex(0xdbe3f0),
            0.0,
        );
    }
    f.text(
        cw - 12.0 - count_w,
        6.0 + 9.0,
        &count,
        8.2,
        false,
        pal::WHITE,
        0.6,
    );
    if let Some(c) = &pill {
        let (_, w, _) = chip_dims(c, max_right);
        put_chip_max(&mut f, cw - 12.0 - w, 6.0 + 14.0, max_right, c);
    }
    f.h = h;
    p.ensure(h + 4.0 + 30.0);
    let top = p.y;
    let start = p.ops().len();
    p.emit(f, MARGIN, top);
    p.y -= h + 8.0;
    p.keep_with_next(start, top);
}

pub(in crate::report) fn scene_banner_scan(b: &SceneBanner, scan: &mut dyn FnMut(&str)) {
    scan(&b.eyebrow);
    scan(&b.title);
    scan(&b.scene_line);
    scan(&b.count);
    if let Some((s, _)) = &b.status {
        scan(s);
    }
}

// ------------------------------------------------------------------ compact header

/// The one-block header at the top of a storyboard sheet: eyebrow, title with the project beside
/// it and an optional scene line on the left; the count and the export date at the right.
#[derive(Debug, Clone)]
pub struct CompactHeader {
    pub eyebrow: String,
    pub title: String,
    pub project: Option<String>,
    pub scene_line: Option<String>,
    /// Bold navy lead of the right side ("12 panels") and the quiet text after it.
    pub strong: String,
    pub quiet: String,
}

pub(in crate::report) fn compact_header(p: &mut Pager, h: &CompactHeader) {
    let cw = p.content_w();
    let mut f = Frag::new();
    let right_w = tw(&h.strong, 9.6, true, 0.0) + 8.0 + tw(&h.quiet, 8.2, false, 0.0);
    let avail = (cw - right_w - 16.0).max(120.0);
    let mut y = 0.0f32;
    f.text(
        0.0,
        y + 7.0,
        &h.eyebrow.to_uppercase(),
        7.0,
        true,
        pal::SLATE,
        1.2,
    );
    y += 11.0;
    let title = Txt::new(h.title.clone())
        .size(17.0)
        .bold()
        .color(pal::NAVY)
        .leading(1.15);
    let lines = txt_lines(&title, avail);
    let lead = 17.0 * 1.15;
    for (k, l) in lines.iter().enumerate() {
        f.text(
            0.0,
            y + 15.0 + k as f32 * lead,
            l,
            17.0,
            true,
            pal::NAVY,
            0.0,
        );
    }
    let last_base = y + 15.0 + (lines.len() as f32 - 1.0) * lead;
    if let Some(pr) = &h.project {
        let last_w = lines.last().map_or(0.0, |l| tw(l, 17.0, true, 0.0));
        if last_w + 10.0 + tw(pr, 9.0, false, 0.0) <= avail {
            f.text(last_w + 10.0, last_base, pr, 9.0, false, pal::SLATE, 0.0);
        }
    }
    y += lines.len() as f32 * lead + 3.0;
    if let Some(s) = &h.scene_line {
        let t = Txt::new(s.clone()).size(8.4).bold().color(pal::SLATE);
        y += put_txt(&mut f, 0.0, y, avail, &t) + 2.0;
    }
    let (sw, qw) = (tw(&h.strong, 9.6, true, 0.0), tw(&h.quiet, 8.2, false, 0.0));
    f.text(
        cw - qw - 8.0 - sw,
        last_base,
        &h.strong,
        9.6,
        true,
        pal::NAVY,
        0.0,
    );
    f.text(cw - qw, last_base, &h.quiet, 8.2, false, pal::SLATE, 0.0);
    y += 4.0;
    f.line(0.0, y, cw, y, pal::NAVY, 1.2);
    f.h = y + 1.2;
    let h_total = f.h + 8.0;
    p.ensure(h_total);
    let top = p.y;
    p.emit(f, MARGIN, top);
    p.y -= h_total;
}

pub(in crate::report) fn compact_header_scan(h: &CompactHeader, scan: &mut dyn FnMut(&str)) {
    for s in [&h.eyebrow, &h.title, &h.strong, &h.quiet] {
        scan(s);
    }
    for s in h.project.iter().chain(&h.scene_line) {
        scan(s);
    }
}
