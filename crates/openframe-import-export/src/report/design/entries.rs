//! Cell items for the breakdown and budget designs: a balanced flow of entries in columns (a name,
//! a note and a state tag in a fixed right gutter), a name with a dot and a small count, and a
//! number with a proportion bar. All of them are measured with the metrics that paint them.

use super::*;

/// One element of a category: its name, an optional note under it, a state tag and a tint.
#[derive(Debug, Clone)]
pub struct Entry {
    pub name: Txt,
    pub note: Option<Txt>,
    /// A small state label at the right edge of the column ("MANUAL", "SUGGESTED").
    pub tag: Option<Chip>,
    /// A tinted row with a bar on its left edge (an element that is only suggested).
    pub tint: Option<(Rgb, Rgb)>,
}

/// Vertical room around one entry.
const SLOT_PAD: f32 = 6.0;
const NAME_NOTE_GAP: f32 = 1.5;

fn entry_height(e: &Entry, text_w: f32) -> f32 {
    let mut h = txt_height(&e.name, text_w);
    if let Some(n) = &e.note {
        h += NAME_NOTE_GAP + txt_height(n, text_w);
    }
    h.max(e.tag.as_ref().map_or(0.0, |c| c.height()))
}

/// Split `heights` into at most `cols` runs in order, so that the tallest run is as short as it
/// can be. Returns the first index of each run after the first.
fn balance(heights: &[f32], cols: usize) -> Vec<usize> {
    let total: f32 = heights.iter().sum();
    let (mut lo, mut hi) = (heights.iter().cloned().fold(0.0, f32::max), total);
    let runs = |cap: f32| -> Option<Vec<usize>> {
        let mut starts = Vec::new();
        let mut used = 0.0f32;
        for (i, h) in heights.iter().enumerate() {
            if used + h > cap && used > 0.0 {
                starts.push(i);
                used = 0.0;
            }
            used += h;
        }
        (starts.len() < cols).then_some(starts)
    };
    for _ in 0..40 {
        let mid = (lo + hi) / 2.0;
        if runs(mid + 0.001).is_some() {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    runs(hi + 0.01).unwrap_or_default()
}

/// The entries in `cols` columns of one width, top to bottom, the columns as even as the order
/// allows. Entries never split across columns.
pub(super) fn entries_frag(w: f32, cols: usize, gap: f32, entries: &[Entry]) -> Frag {
    let cols = cols.max(1);
    let col_w = (w - gap * (cols as f32 - 1.0)) / cols as f32;
    let tag_w = entries
        .iter()
        .filter_map(|e| e.tag.as_ref())
        .map(|c| c.width())
        .fold(0.0, f32::max);
    let gutter = if tag_w > 0.0 { tag_w + 6.0 } else { 0.0 };
    let text_w = (col_w - gutter).max(20.0);
    let heights: Vec<f32> = entries
        .iter()
        .map(|e| entry_height(e, text_w) + SLOT_PAD)
        .collect();
    let starts = balance(&heights, cols);
    let mut f = Frag::new();
    let mut tallest = 0.0f32;
    let mut col = 0usize;
    let mut y = 0.0f32;
    for (i, e) in entries.iter().enumerate() {
        if starts.get(col).is_some_and(|s| *s == i) {
            col += 1;
            tallest = tallest.max(y);
            y = 0.0;
        }
        let x = col as f32 * (col_w + gap);
        let h = heights[i];
        if let Some((fill, bar)) = e.tint {
            f.shape(x - 5.0, y, col_w + 8.0, h - 1.0, 2.0, Some(fill), None);
            f.shape(x - 5.0, y, 2.0, h - 1.0, 0.0, Some(bar), None);
        }
        let top = y + SLOT_PAD / 2.0 - 0.5;
        let mut ty = top;
        ty += put_txt(&mut f, x, ty, text_w, &e.name);
        if let Some(n) = &e.note {
            put_txt(&mut f, x, ty + NAME_NOTE_GAP, text_w, n);
        }
        if let Some(c) = &e.tag {
            // Level with the first line of the name.
            let first = e.name.size * e.name.leading;
            put_chip(
                &mut f,
                x + col_w - c.width(),
                top + (first - c.height()) / 2.0,
                c,
            );
        }
        y += h;
    }
    f.h = tallest.max(y);
    f
}

pub(super) fn entries_scan(entries: &[Entry], scan: &mut dyn FnMut(&str)) {
    for e in entries {
        scan(&e.name.text);
        if let Some(n) = &e.note {
            scan(&n.text);
        }
        if let Some(c) = &e.tag {
            scan(&c.text);
        }
    }
}

/// A dot, a wrapping name under a hanging indent, and a small text after the last line.
pub(super) fn put_name(
    f: &mut Frag,
    y: f32,
    w: f32,
    dot: Option<Rgb>,
    text: &Txt,
    small: Option<&Txt>,
) -> f32 {
    let x = if dot.is_some() { 11.0 } else { 0.0 };
    let lines = if text.track > 0.0 {
        tracked_lines(text, w - x)
    } else {
        txt_lines(text, w - x)
    };
    let lh = text.size * text.leading;
    if let Some(d) = dot {
        f.circle(3.5, y + lh * 0.5, 3.0, Some(d), None);
    }
    for (k, l) in lines.iter().enumerate() {
        f.text(
            x,
            y + text.size + k as f32 * lh,
            l,
            text.size,
            text.bold,
            text.color,
            text.track,
        );
    }
    let mut h = lines.len() as f32 * lh;
    if let Some(s) = small {
        let last = lines
            .last()
            .map_or(0.0, |l| tw(l, text.size, text.bold, text.track));
        let sw = tw(&s.text, s.size, s.bold, s.track);
        if x + last + 6.0 + sw <= w {
            f.text(
                x + last + 6.0,
                y + text.size + (lines.len() as f32 - 1.0) * lh,
                &s.text,
                s.size,
                s.bold,
                s.color,
                s.track,
            );
        } else {
            f.text(x, y + h + s.size, &s.text, s.size, s.bold, s.color, s.track);
            h += s.size * 1.3;
        }
    }
    h
}

/// A number at the right with a short proportion bar before it.
pub(super) fn put_meter(
    f: &mut Frag,
    y: f32,
    w: f32,
    share: f32,
    bar_w: f32,
    color: Rgb,
    value: &Txt,
) -> f32 {
    let vw = tw(&value.text, value.size, value.bold, value.track);
    f.text(
        w - vw,
        y + value.size,
        &value.text,
        value.size,
        value.bold,
        value.color,
        value.track,
    );
    if bar_w > 0.0 {
        let bx = w - vw - 7.0 - bar_w;
        let by = y + value.size * 0.62 - 1.5;
        f.shape(bx, by, bar_w, 3.0, 1.5, Some(pal::LINE), None);
        let fill = bar_w * share.clamp(0.0, 1.0);
        if fill > 0.0 {
            f.shape(bx, by, fill.max(2.0), 3.0, 1.5, Some(color), None);
        }
    }
    value.size * value.leading
}
