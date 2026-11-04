//! The head of a table: a tinted bar with an optional tile, a small caption, a title and a count
//! (a scene of the breakdown, a category of the catalog). It sits in the repeating header, so a
//! table that runs over a page never loses its name, and it says " · continued" after the title on
//! the pages after the first.

use super::*;

#[derive(Debug, Clone)]
pub struct TableHead {
    pub fill: Rgb,
    /// A bar down the left edge (the category colour).
    pub stripe: Option<Rgb>,
    /// A dot before the title.
    pub dot: Option<Rgb>,
    pub tile: Option<Tile>,
    /// A small tracked caption over the title.
    pub eyebrow: Option<String>,
    pub title: String,
    pub title_size: f32,
    pub title_track: f32,
    /// Text at the right end (a count); the chip sits under it.
    pub right: Option<Txt>,
    pub chip: Option<Chip>,
}

impl TableHead {
    pub fn new(title: impl Into<String>) -> TableHead {
        TableHead {
            fill: pal::TINT,
            stripe: None,
            dot: None,
            tile: None,
            eyebrow: None,
            title: title.into(),
            title_size: 9.6,
            title_track: 0.0,
            right: None,
            chip: None,
        }
    }
}

const PAD: f32 = 10.0;
const SUFFIX: &str = " · continued";

pub(super) fn head_frag(h: &TableHead, w: f32, continued: bool) -> Frag {
    let tile_h = h
        .tile
        .as_ref()
        .map(|t| if t.top.is_some() { 31.0 } else { 24.0 })
        .unwrap_or(0.0);
    let mut x = PAD + if h.stripe.is_some() { 3.0 } else { 0.0 };
    if let Some(t) = &h.tile {
        x += t.w + 9.0;
    }
    if h.dot.is_some() {
        x += 11.0;
    }
    // The right column: the count over the chip, both right aligned.
    let chip_w = h.chip.as_ref().map_or(0.0, |c| c.width());
    let right_w = h
        .right
        .as_ref()
        .map_or(0.0, |r| tw(&r.text, r.size, r.bold, r.track));
    let col_w = right_w.max(chip_w);
    let avail = (w - x - PAD - if col_w > 0.0 { col_w + 10.0 } else { 0.0 }).max(60.0);

    let title = Txt::new(h.title.clone())
        .size(h.title_size)
        .bold()
        .track(h.title_track)
        .color(pal::NAVY)
        .leading(1.22);
    let lines = if h.title_track > 0.0 {
        tracked_lines(&title, avail)
    } else {
        txt_lines(&title, avail)
    };
    let lead = h.title_size * 1.22;
    let last_w = lines
        .last()
        .map_or(0.0, |l| tw(l, h.title_size, true, h.title_track));
    let suffix_w = tw(SUFFIX, 7.4, false, 0.0);
    let below = continued && last_w + suffix_w > avail;
    let mut text = Frag::new();
    let mut th = 0.0f32;
    if let Some(e) = &h.eyebrow {
        th += put_txt(
            &mut text,
            0.0,
            th,
            avail,
            &Txt::label(e.clone()).size(6.8).after(1.0),
        );
    }
    for (k, l) in lines.iter().enumerate() {
        text.text(
            0.0,
            th + h.title_size + k as f32 * lead,
            l,
            h.title_size,
            true,
            pal::NAVY,
            h.title_track,
        );
    }
    if continued {
        if below {
            text.text(
                0.0,
                th + lines.len() as f32 * lead + 7.4,
                "· continued",
                7.4,
                false,
                pal::SLATE,
                0.0,
            );
            th += 9.0;
        } else {
            text.text(
                last_w,
                th + h.title_size + (lines.len() as f32 - 1.0) * lead,
                SUFFIX,
                7.4,
                false,
                pal::SLATE,
                0.0,
            );
        }
    }
    th += lines.len() as f32 * lead;
    let chip_h = h.chip.as_ref().map_or(0.0, |c| c.height());
    let right_h = h.right.as_ref().map_or(0.0, |r| r.size * r.leading)
        + if chip_h > 0.0 && right_w > 0.0 {
            2.0
        } else {
            0.0
        }
        + chip_h;
    let inner = th.max(tile_h).max(right_h);
    let total = inner + 12.0;

    let mut f = Frag::new();
    f.shape_c(0.0, 0.0, w, total, 6.0, true, false, Some(h.fill), None);
    if let Some(s) = h.stripe {
        // Below the rounded corner, so the bar never sticks out of the curve.
        f.shape(0.0, 5.5, 3.2, total - 5.5, 0.0, Some(s), None);
    }
    let mut px = PAD + if h.stripe.is_some() { 3.0 } else { 0.0 };
    if let Some(t) = &h.tile {
        let ty = (total - tile_h) / 2.0;
        f.shape(
            px,
            ty,
            t.w,
            tile_h,
            4.0,
            Some(t.fill),
            t.line.map(|l| (l, 0.7)),
        );
        match &t.top {
            Some(top) => {
                let w1 = tw(top, 6.6, true, 0.8);
                f.text(
                    px + (t.w - w1) / 2.0,
                    ty + 9.5,
                    top,
                    6.6,
                    true,
                    t.color,
                    0.8,
                );
                let w2 = tw(&t.text, 15.0, true, 0.0);
                f.text(
                    px + (t.w - w2) / 2.0,
                    ty + 26.0,
                    &t.text,
                    15.0,
                    true,
                    t.color,
                    0.0,
                );
            }
            None => {
                let w2 = tw(&t.text, 10.0, true, 0.0);
                f.text(
                    px + (t.w - w2) / 2.0,
                    ty + 16.0,
                    &t.text,
                    10.0,
                    true,
                    t.color,
                    0.0,
                );
            }
        }
        px += t.w + 9.0;
    }
    if let Some(d) = h.dot {
        // Level with the middle of the first title line.
        let eyebrow = if h.eyebrow.is_some() {
            6.8 * 1.3 + 1.0
        } else {
            0.0
        };
        let cy = (total - th) / 2.0 + eyebrow + h.title_size * 0.62;
        f.circle(px + 3.5, cy, 3.3, Some(d), None);
        px += 11.0;
    }
    f.put(text, px, (total - th) / 2.0);
    let mut ry = (total - right_h) / 2.0;
    if let Some(r) = &h.right {
        let mut t = r.clone();
        t.align = Align::Right;
        ry += put_txt(&mut f, w - PAD - col_w - 1.0, ry, col_w + 1.0, &t) + 2.0;
    }
    if let Some(c) = &h.chip {
        put_chip(&mut f, w - PAD - c.width(), ry, c);
    }
    f.h = total;
    f
}

pub(super) fn head_scan(h: &TableHead, scan: &mut dyn FnMut(&str)) {
    scan(&h.title);
    scan(SUFFIX);
    if let Some(e) = &h.eyebrow {
        scan(e);
    }
    if let Some(r) = &h.right {
        scan(&r.text);
    }
    if let Some(c) = &h.chip {
        scan(&c.text);
    }
    if let Some(t) = &h.tile {
        scan(&t.text);
        if let Some(top) = &t.top {
            scan(top);
        }
    }
}
