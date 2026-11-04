//! The Day Out of Days matrix: a frozen number and name column on the left of every column group,
//! day columns of one width in every group, a month band over the day header, code cells that read
//! in grayscale (edge caps, outlines, the code printed as text), count rows under the cast and a
//! totals block on the last group. The legend sits in the repeating header, so every page has it.
//!
//! Layout is planned, not measured by the browser: the days are split into the fewest groups
//! whose columns keep one width of 24 to 38 points, each group starts on a new page, and the rows
//! of a group move to a new page only between rows, with the header repeated.

use super::band::{FactRun, LINE_STRONG, flow_runs, put_segs};
use super::table::{plan_breaks, title_block};
use super::*;

/// Width of the number column.
const NUM_W: f32 = 15.0;
/// Narrowest the cast column may be before the days are split into another group.
const CAST_MIN: f32 = 136.0;
const D_MIN: f32 = 24.0;
const D_MAX: f32 = 38.0;
const ROW_MIN: f32 = 16.0;
const COUNT_MIN: f32 = 13.0;
const MONTH_H: f32 = 11.0;
const DAY_H: f32 = 36.0;
const CAP: f32 = 3.0;
const CAST_SIZE: f32 = 7.6;
const OFF_FILL: Rgb = Rgb::hex(0xe9edf3);
const OFF_HEAD: Rgb = Rgb::hex(0xdfe4ec);
const GRID: Rgb = Rgb::hex(0xe3e8ef);
const MUTED: Rgb = Rgb::hex(0x66728a);

/// One day column.
#[derive(Debug, Clone)]
pub struct MatrixDay {
    /// Shoot day number ("01"), or "OFF".
    pub top: String,
    /// Day of the month ("11"); empty for an undated day.
    pub date: String,
    /// Weekday ("MON"); empty for an undated day.
    pub weekday: String,
    /// The month and year ("JANUARY 2027") with a short form for a narrow span ("JAN").
    pub month: Option<(String, String)>,
    /// Print the weekday in a filled chip.
    pub weekend: bool,
    /// An off day: the whole column is shaded.
    pub off: bool,
}

/// How a code looks. The text is always printed; fill, caps and outlines reinforce it.
#[derive(Debug, Clone)]
pub struct MatrixCode {
    pub text: String,
    pub fill: Option<Rgb>,
    pub color: Rgb,
    /// A dark cap on the left edge (start) and on the right edge (finish).
    pub cap_left: bool,
    pub cap_right: bool,
    /// A 1 pt outline in this colour (a hold).
    pub hollow: Option<Rgb>,
    /// Entered by hand: an ink outline inside the cell.
    pub corrected: bool,
    /// A hairline border (the blank swatch of the legend).
    pub border: Option<Rgb>,
}

impl MatrixCode {
    pub fn new(text: impl Into<String>, fill: Option<Rgb>, color: Rgb) -> MatrixCode {
        MatrixCode {
            text: text.into(),
            fill,
            color,
            cap_left: false,
            cap_right: false,
            hollow: None,
            corrected: false,
            border: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MatrixKey {
    pub swatch: MatrixCode,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct MatrixRow {
    pub number: String,
    /// Printed bold (the actor, or the character when nobody is cast).
    pub name: String,
    /// Printed regular after the name ("(ANJALI)").
    pub detail: String,
    /// One code per day column of the matrix.
    pub cells: Vec<MatrixCode>,
    /// One value per totals column.
    pub totals: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MatrixCount {
    pub label: String,
    /// One value per day column.
    pub values: Vec<String>,
    /// One value per totals column (empty text prints nothing).
    pub totals: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MatrixTotals {
    pub heading: String,
    /// Header, width in points and whether the value prints smaller (text such as "Day 14").
    pub columns: Vec<(String, f32, bool)>,
}

#[derive(Debug, Clone)]
pub struct Matrix {
    pub title: String,
    pub hint: Option<String>,
    pub legend_label: String,
    pub legend: Vec<MatrixKey>,
    /// The three labels beside the day header lines ("Shoot day", "Date", "Weekday").
    pub line_labels: [String; 3],
    /// The headers of the two frozen columns ("#", "Cast member").
    pub frozen_labels: [String; 2],
    pub days: Vec<MatrixDay>,
    pub rows: Vec<MatrixRow>,
    pub counts: Vec<MatrixCount>,
    pub totals: Option<MatrixTotals>,
    /// A darker line after every this many rows (0 = none).
    pub guide_every: usize,
}

/// The day columns split into groups: ranges of days and the one width of a day column.
///
/// The fewest groups come first; for that number of groups the widest whole-point column that
/// still holds every day wins. The first groups hold as many days as the width after the frozen
/// columns allows, the last one only what fits beside the totals block.
pub(super) fn plan_groups(
    n: usize,
    total_w: f32,
    totals_w: f32,
) -> (Vec<std::ops::Range<usize>>, f32) {
    let n = n.max(1);
    let full = total_w - NUM_W - CAST_MIN;
    let last = (full - totals_w).max(D_MIN);
    for k in 1usize.. {
        for d in (D_MIN as u32..=D_MAX as u32).rev() {
            let d = d as f32;
            let cap_full = ((full / d).floor() as usize).max(1);
            let cap_last = ((last / d).floor() as usize).max(1);
            if (k - 1) * cap_full + cap_last < n {
                continue;
            }
            let g = n.saturating_sub(cap_last).div_ceil((k - 1).max(1)).max(1);
            let mut out = Vec::new();
            let mut start = 0usize;
            while n - start > cap_last {
                let take = g.min(n - start - 1).max(1);
                out.push(start..start + take);
                start += take;
            }
            out.push(start..n);
            return (out, d);
        }
        if k > 4096 {
            break;
        }
    }
    (std::iter::once(0..n).collect(), D_MIN)
}

/// The part of a list that belongs to a group; a list that is too short gives fewer items.
fn within<'a, T>(v: &'a [T], r: &std::ops::Range<usize>) -> &'a [T] {
    v.get(r.start.min(v.len())..r.end.min(v.len()))
        .unwrap_or(&[])
}

fn fit_size(text: &str, w: f32, size: f32) -> f32 {
    let mut s = size;
    while s > 7.2 && text_width(text, s, true) > w {
        s -= 0.2;
    }
    s
}

/// A code cell at (x, y) of w by h: fill, caps, outline and the code text.
fn put_code(f: &mut Frag, x: f32, y: f32, w: f32, h: f32, c: &MatrixCode, size: f32) {
    if let Some(fill) = c.fill {
        f.shape(x, y, w, h, 0.0, Some(fill), None);
    }
    if let Some(b) = c.border {
        f.shape(
            x + 0.35,
            y + 0.35,
            w - 0.7,
            h - 0.7,
            0.0,
            Some(pal::WHITE),
            Some((b, 0.7)),
        );
    }
    if let Some(o) = c.hollow {
        f.shape(
            x + 0.5,
            y + 0.5,
            w - 1.0,
            h - 1.0,
            0.0,
            None,
            Some((o, 1.0)),
        );
    }
    if c.cap_left {
        f.shape(x, y, CAP, h, 0.0, Some(pal::NAVY), None);
    }
    if c.cap_right {
        f.shape(x + w - CAP, y, CAP, h, 0.0, Some(pal::NAVY), None);
    }
    if c.corrected {
        f.shape(
            x + 0.7,
            y + 0.7,
            w - 1.4,
            h - 1.4,
            0.0,
            None,
            Some((pal::NAVY, 1.4)),
        );
    }
    if !c.text.is_empty() {
        let s = fit_size(&c.text, w - 2.0, size);
        let tw1 = text_width(&c.text, s, true);
        f.text(
            x + (w - tw1) / 2.0,
            y + (h + 0.72 * s) / 2.0,
            &c.text,
            s,
            true,
            c.color,
            0.0,
        );
    }
}

/// The legend: a label, then the keys flowing and wrapping. Returns the height.
fn put_legend(f: &mut Frag, dy: f32, width: f32, m: &Matrix) -> f32 {
    const LINE: f32 = 15.0;
    let label = m.legend_label.to_uppercase();
    let mut x = 0.0f32;
    let mut row = 0usize;
    let lw = tw(&label, 6.8, true, 0.7) + 10.0;
    f.text(0.0, dy + 10.0, &label, 6.8, true, pal::SLATE, 0.7);
    x += lw;
    for k in &m.legend {
        let sw = if k.swatch.text.chars().count() > 2 {
            26.0
        } else {
            22.0
        };
        let w = sw + 5.0 + text_width(&k.label, 7.0, false) + 14.0;
        if x > 0.0 && x + w - 14.0 > width {
            x = 0.0;
            row += 1;
        }
        let top = dy + row as f32 * LINE + 2.0;
        put_code(f, x, top, sw, 12.0, &k.swatch, 7.0);
        f.text(
            x + sw + 5.0,
            top + 9.0,
            &k.label,
            7.0,
            false,
            pal::SLATE,
            0.0,
        );
        x += w;
    }
    (row + 1) as f32 * LINE + 4.0
}

/// Everything that stays the same across the days of one group.
struct Geometry {
    cast_w: f32,
    d: f32,
    days: usize,
    /// Width of the totals block (0 on groups without one).
    tot_w: f32,
    total_w: f32,
}

impl Geometry {
    fn day_x(&self, j: usize) -> f32 {
        NUM_W + self.cast_w + j as f32 * self.d
    }
    fn tot_x(&self) -> f32 {
        self.day_x(self.days)
    }
}

/// The repeating header: title, legend, month band and day header.
fn head(m: &Matrix, g: &Geometry, days: &[MatrixDay], hint: Option<&String>, cont: bool) -> Frag {
    let mut f = Frag::new();
    let tf = title_block(&m.title, hint, g.total_w, cont).unwrap_or_else(Frag::new);
    let mut y = tf.h;
    f.put(tf, 0.0, 0.0);
    y += put_legend(&mut f, y, g.total_w, m);
    // Month band: one cell per run of days in the same month.
    let mut j = 0;
    while j < days.len() {
        let key = days[j].month.clone();
        let mut e = j + 1;
        while e < days.len() && days[e].month == key {
            e += 1;
        }
        let (x, w) = (g.day_x(j), (e - j) as f32 * g.d);
        f.line(x, y, x, y + MONTH_H, pal::LINE, 0.6);
        if let Some((long, short)) = key {
            let fits = |t: &str| tw(t, 6.8, true, 0.5) + 5.0 <= w;
            let text = if fits(&long) { Some(long) } else { Some(short) };
            if let Some(t) = text.filter(|t| fits(t) || w >= 18.0) {
                f.text(x + 3.0, y + 8.0, &t, 6.8, true, pal::SLATE, 0.5);
            }
        }
        j = e;
    }
    if let Some(t) = &m.totals
        && g.tot_w > 0.0
    {
        let x = g.tot_x();
        f.shape(x, y, g.tot_w, MONTH_H, 0.0, Some(pal::TINT), None);
        let head = t.heading.to_uppercase();
        let hw = tw(&head, 6.8, true, 0.5);
        f.text(
            x + (g.tot_w - hw) / 2.0,
            y + 8.0,
            &head,
            6.8,
            true,
            pal::SLATE,
            0.5,
        );
    }
    f.line(NUM_W, y + MONTH_H, g.total_w, y + MONTH_H, pal::LINE, 0.6);
    y += MONTH_H;
    // Day header.
    let (base1, base2, base3) = (y + 10.0, y + 22.5, y + 32.0);
    for (j, d) in days.iter().enumerate() {
        let x = g.day_x(j);
        if d.off {
            f.shape(x, y, g.d, DAY_H, 0.0, Some(OFF_HEAD), None);
        }
        let mid = x + g.d / 2.0;
        let ink = if d.off { pal::SLATE } else { pal::NAVY };
        let centre = |f: &mut Frag, text: &str, size: f32, base: f32, color: Rgb| {
            if !text.is_empty() {
                let w = tw(text, size, true, 0.0);
                f.text(mid - w / 2.0, base, text, size, true, color, 0.0);
            }
        };
        centre(&mut f, &d.top, 7.2, base1, ink);
        centre(&mut f, &d.date, 10.0, base2, ink);
        if d.weekend && !d.weekday.is_empty() {
            let w = tw(&d.weekday, 6.6, true, 0.0) + 6.0;
            f.shape(mid - w / 2.0, y + 24.8, w, 9.4, 3.0, Some(pal::SLATE), None);
            centre(&mut f, &d.weekday, 6.6, base3, pal::WHITE);
        } else {
            centre(&mut f, &d.weekday, 6.6, base3, pal::SLATE);
        }
    }
    let sl = |f: &mut Frag, text: &str, right: bool, base: f32, x: f32| {
        let t = text.to_uppercase();
        let w = tw(&t, 6.4, true, 0.4);
        let at = if right { x - w } else { x };
        f.text(at, base, &t, 6.4, true, pal::SLATE, 0.4);
    };
    let edge = NUM_W + g.cast_w - 5.0;
    sl(&mut f, &m.line_labels[0], true, base1, edge);
    sl(&mut f, &m.line_labels[1], true, base2, edge);
    sl(&mut f, &m.line_labels[2], true, base3, edge);
    let lab = |f: &mut Frag, text: &str, x: f32, centre: bool| {
        let t = text.to_uppercase();
        let w = tw(&t, 7.0, true, 0.8);
        let at = if centre { x + (NUM_W - w) / 2.0 } else { x };
        f.text(at, y + DAY_H - 5.0, &t, 7.0, true, pal::SLATE, 0.8);
    };
    lab(&mut f, &m.frozen_labels[0], 0.0, true);
    lab(&mut f, &m.frozen_labels[1], NUM_W + 4.0, false);
    if let Some(t) = &m.totals
        && g.tot_w > 0.0
    {
        let mut x = g.tot_x();
        for (name, w, _) in &t.columns {
            let n = name.to_uppercase();
            let nw = tw(&n, 6.4, true, 0.2);
            f.text(
                x + (w - nw) / 2.0,
                y + DAY_H - 6.0,
                &n,
                6.4,
                true,
                pal::SLATE,
                0.2,
            );
            x += w;
        }
    }
    y += DAY_H;
    f.line(0.0, y, g.total_w, y, pal::NAVY, 0.9);
    f.h = y + 0.9;
    f
}

/// A cast row of `h` points.
#[allow(clippy::too_many_arguments)]
fn cast_row(
    m: &Matrix,
    g: &Geometry,
    row: &MatrixRow,
    cells: &[MatrixCode],
    lines: &[Vec<super::band::Seg>],
    h: f32,
    strong: bool,
    last: bool,
) -> Frag {
    let mut f = Frag::new();
    let tile = tw(&row.number, 6.8, true, 0.0);
    f.shape(
        1.0,
        (h - 12.0) / 2.0,
        NUM_W - 2.0,
        12.0,
        3.0,
        Some(Rgb::hex(0xe8edf3)),
        None,
    );
    f.text(
        (NUM_W - tile) / 2.0,
        (h + 0.72 * 6.8) / 2.0,
        &row.number,
        6.8,
        true,
        pal::NAVY,
        0.0,
    );
    let lh = CAST_SIZE * 1.25;
    let top = (h - lines.len() as f32 * lh) / 2.0;
    for (k, l) in lines.iter().enumerate() {
        let base = top + k as f32 * lh + CAST_SIZE + (lh - CAST_SIZE) / 2.0 - 0.6;
        put_segs(
            &mut f,
            NUM_W + 4.0,
            base,
            l,
            CAST_SIZE,
            (pal::SLATE, pal::NAVY),
        );
    }
    for (j, c) in cells.iter().enumerate() {
        put_code(&mut f, g.day_x(j) + 1.0, 2.0, g.d - 2.0, h - 4.0, c, 8.2);
    }
    if let Some(t) = &m.totals
        && g.tot_w > 0.0
    {
        let mut x = g.tot_x();
        for ((_, w, small), v) in t.columns.iter().zip(&row.totals) {
            let s = if *small { 7.2 } else { 8.0 };
            let vw = tw(v, s, true, 0.0);
            f.text(
                x + (w - vw) / 2.0,
                (h + 0.72 * s) / 2.0,
                v,
                s,
                true,
                pal::NAVY,
                0.0,
            );
            x += w;
        }
    }
    if !last {
        if strong {
            f.line(0.0, h, g.total_w, h, LINE_STRONG, 0.75);
        } else {
            f.line(0.0, h, g.total_w, h, pal::LINE, 0.5);
        }
    }
    f
}

/// A count row.
#[allow(clippy::too_many_arguments)]
fn count_row(
    m: &Matrix,
    g: &Geometry,
    c: &MatrixCount,
    values: &[String],
    days: &[MatrixDay],
    label_lines: &[String],
    h: f32,
    first: bool,
) -> Frag {
    let mut f = Frag::new();
    f.shape(0.0, 0.0, g.total_w, h, 0.0, Some(pal::PANEL), None);
    for (j, d) in days.iter().enumerate() {
        if d.off {
            f.shape(g.day_x(j), 0.0, g.d, h, 0.0, Some(OFF_FILL), None);
        }
    }
    let lh = 6.6 * 1.25;
    let top = (h - label_lines.len() as f32 * lh) / 2.0;
    let right = NUM_W + g.cast_w - 5.0;
    for (k, l) in label_lines.iter().enumerate() {
        let w = tw(l, 6.6, true, 0.4);
        f.text(
            right - w,
            top + k as f32 * lh + 6.6 + 0.3,
            l,
            6.6,
            true,
            pal::SLATE,
            0.4,
        );
    }
    for (j, v) in values.iter().enumerate() {
        let w = tw(v, 7.8, true, 0.0);
        let color = if v == "0" { MUTED } else { pal::NAVY };
        f.text(
            g.day_x(j) + (g.d - w) / 2.0,
            (h + 0.72 * 7.8) / 2.0,
            v,
            7.8,
            true,
            color,
            0.0,
        );
    }
    if let Some(t) = &m.totals
        && g.tot_w > 0.0
    {
        f.shape(g.tot_x(), 0.0, g.tot_w, h, 0.0, Some(pal::TINT), None);
        let mut x = g.tot_x();
        for ((_, w, _), v) in t.columns.iter().zip(&c.totals) {
            if !v.is_empty() {
                let vw = tw(v, 8.0, true, 0.0);
                f.text(
                    x + (w - vw) / 2.0,
                    (h + 0.72 * 8.0) / 2.0,
                    v,
                    8.0,
                    true,
                    pal::NAVY,
                    0.0,
                );
            }
            x += w;
        }
    }
    if first {
        f.line(0.0, 0.0, g.total_w, 0.0, pal::NAVY, 1.0);
    }
    f.line(0.0, h, g.total_w, h, pal::LINE, 0.5);
    f
}

pub(in crate::report) fn matrix(p: &mut Pager, m: &Matrix) {
    if m.days.is_empty() {
        return;
    }
    let total_w = p.content_w();
    let totals_w: f32 = m
        .totals
        .as_ref()
        .map_or(0.0, |t| t.columns.iter().map(|c| c.1).sum());
    let (groups, d) = plan_groups(m.days.len(), total_w, totals_w);
    for (gi, range) in groups.iter().enumerate() {
        if gi > 0 && !p.fresh() {
            p.new_page();
        }
        let last = gi + 1 == groups.len();
        let days = &m.days[range.clone()];
        let tot_w = if last { totals_w } else { 0.0 };
        let g = Geometry {
            cast_w: total_w - NUM_W - days.len() as f32 * d - tot_w,
            d,
            days: days.len(),
            tot_w,
            total_w,
        };
        let hint = match (&m.hint, groups.len()) {
            (h, 1) => h.clone(),
            (Some(h), n) => Some(format!("{h} · Columns group {} of {n}", gi + 1)),
            (None, n) => Some(format!("Columns group {} of {n}", gi + 1)),
        };
        group(p, m, &g, days, range.clone(), hint.as_ref());
    }
}

fn group(
    p: &mut Pager,
    m: &Matrix,
    g: &Geometry,
    days: &[MatrixDay],
    range: std::ops::Range<usize>,
    hint: Option<&String>,
) {
    // Row fragments, built once.
    let name_w = g.cast_w - 8.0;
    let n_rows = m.rows.len();
    let mut items: Vec<(Frag, f32)> = Vec::new();
    for (i, row) in m.rows.iter().enumerate() {
        let mut runs = vec![FactRun::bold(row.name.clone())];
        if !row.detail.is_empty() {
            runs.push(FactRun::plain(format!(" {}", row.detail)));
        }
        let lines = flow_runs(&runs, name_w, CAST_SIZE);
        let h = ROW_MIN.max(lines.len() as f32 * CAST_SIZE * 1.25 + 6.0);
        let strong = m.guide_every > 0 && (i + 1) % m.guide_every == 0;
        let cells = within(&row.cells, &range);
        items.push((
            cast_row(
                m,
                g,
                row,
                cells,
                &lines,
                h,
                strong,
                i + 1 == n_rows && m.counts.is_empty(),
            ),
            h,
        ));
    }
    for (i, c) in m.counts.iter().enumerate() {
        let label_w = NUM_W + g.cast_w - 10.0;
        let ll = wrap_pt(&c.label.to_uppercase(), label_w * 0.94, 6.6, true);
        let h = COUNT_MIN.max(ll.len() as f32 * 6.6 * 1.25 + 5.0);
        let values = within(&c.values, &range);
        items.push((count_row(m, g, c, values, days, &ll, h, i == 0), h));
    }
    let heights: Vec<f32> = items.iter().map(|x| x.1).collect();
    let n = heights.len();
    let mut forbid = vec![false; n + 1];
    if n >= 2 {
        forbid[1] = true;
    }
    // The last two cast rows travel with the counts, and the counts with each other.
    if n_rows >= 2 {
        forbid[n_rows - 1] = true;
    }
    for f in forbid.iter_mut().take(n).skip(n_rows) {
        *f = true;
    }
    let head_first = head(m, g, days, hint, false);
    let head_cont = head(m, g, days, hint, true);
    let breaks = plan_breaks(
        &heights,
        &forbid,
        p.room() - head_first.h,
        p.body_h() - head_cont.h,
        p.fresh(),
    );
    let mut starts = vec![0usize];
    starts.extend(breaks.iter().copied().filter(|b| *b != 0));
    if breaks.first() == Some(&0) {
        p.new_page();
    }
    let mut items = items.into_iter().map(Some).collect::<Vec<_>>();
    for (si, &s) in starts.iter().enumerate() {
        let e = starts.get(si + 1).copied().unwrap_or(n);
        if si > 0 {
            p.new_page();
        }
        let hd = if si > 0 { &head_cont } else { &head_first };
        let mut seg = Frag {
            h: 0.0,
            ops: hd.ops.clone(),
        };
        let body_h: f32 = heights[s..e].iter().sum();
        let top = hd.h;
        // Off-day columns are shaded behind the cast rows of this page.
        let cast_end = e.min(n_rows);
        if cast_end > s {
            let sh: f32 = heights[s..cast_end].iter().sum();
            for (j, d) in days.iter().enumerate() {
                if d.off {
                    seg.shape(g.day_x(j), top, g.d, sh, 0.0, Some(OFF_FILL), None);
                }
            }
        }
        let mut y = top;
        for item in items.iter_mut().take(e).skip(s) {
            if let Some((frag, h)) = item.take() {
                seg.put(frag, 0.0, y);
                y += h;
            }
        }
        // Reading guides between the day columns and at the totals block.
        for j in 0..=days.len() {
            let x = g.day_x(j);
            let edge = j == days.len() && g.tot_w > 0.0;
            if j < days.len() || edge {
                seg.line(
                    x,
                    top,
                    x,
                    top + body_h,
                    if edge { LINE_STRONG } else { GRID },
                    if edge { 0.6 } else { 0.4 },
                );
            }
        }
        seg.h = top + body_h;
        let at = p.y;
        let h = seg.h;
        p.emit(seg, MARGIN, at);
        p.y -= h;
    }
    p.y -= 10.0;
}

pub(in crate::report) fn matrix_scan(m: &Matrix, scan: &mut dyn FnMut(&str)) {
    scan(&m.title);
    if let Some(h) = &m.hint {
        scan(h);
    }
    scan(&m.legend_label);
    for k in &m.legend {
        scan(&k.label);
        scan(&k.swatch.text);
    }
    m.line_labels.iter().for_each(|s| scan(s));
    m.frozen_labels.iter().for_each(|s| scan(s));
    for d in &m.days {
        scan(&d.top);
        scan(&d.date);
        scan(&d.weekday);
        if let Some((a, b)) = &d.month {
            scan(a);
            scan(b);
        }
    }
    for r in &m.rows {
        scan(&r.number);
        scan(&r.name);
        scan(&r.detail);
        r.cells.iter().for_each(|c| scan(&c.text));
        r.totals.iter().for_each(|s| scan(s));
    }
    for c in &m.counts {
        scan(&c.label);
        c.values.iter().for_each(|s| scan(s));
        c.totals.iter().for_each(|s| scan(s));
    }
    if let Some(t) = &m.totals {
        scan(&t.heading);
        t.columns.iter().for_each(|c| scan(&c.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sixteen_days_fit_one_group_at_the_narrowest_width() {
        let (groups, d) = plan_groups(16, 700.0, 160.0);
        assert_eq!(groups, vec![0..16]);
        assert_eq!(d, 24.0);
    }

    #[test]
    fn few_days_get_wide_columns() {
        let (groups, d) = plan_groups(6, 700.0, 160.0);
        assert_eq!(groups, vec![0..6]);
        assert_eq!(d, 38.0);
    }

    #[test]
    fn thirty_six_days_split_into_the_fewest_groups() {
        let (groups, d) = plan_groups(36, 700.0, 160.0);
        assert_eq!(groups, vec![0..21, 21..36]);
        assert_eq!(d, 25.0);
    }

    #[test]
    fn every_day_lands_in_exactly_one_group_and_fits_its_width() {
        for n in 1..=200usize {
            for totals in [0.0f32, 160.0] {
                let (groups, d) = plan_groups(n, 700.0, totals);
                assert!((24.0..=38.0).contains(&d), "{n}: {d}");
                let mut next = 0;
                for (i, r) in groups.iter().enumerate() {
                    assert_eq!(r.start, next, "{n}: groups are contiguous");
                    assert!(!r.is_empty(), "{n}: no empty group");
                    let room =
                        700.0 - NUM_W - CAST_MIN - if i + 1 == groups.len() { totals } else { 0.0 };
                    assert!(r.len() as f32 * d <= room + 0.01, "{n}: group {i} fits");
                    next = r.end;
                }
                assert_eq!(next, n);
            }
        }
    }
}
