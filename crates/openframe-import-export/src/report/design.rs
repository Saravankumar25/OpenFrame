//! The OpenFrame production document design system (change specs 02 to 05 and the shot list,
//! storyboard, moodboard, report and budget redesign).
//!
//! Navy ink structure on white paper, a warm orange accent, restrained status colours. Every
//! block here is opt-in: an exporter that pushes plain blocks keeps the grayscale generic look.
//! Layout is measured with the same Helvetica metrics that draw the text, so wrapping never
//! clips. Nothing is ever cut with an ellipsis: long text wraps and rows grow. Output stays
//! deterministic (no clock, no random).
//!
//! Building pieces ("fragments") are laid out relative to their own top left corner and placed
//! on the page afterwards, so cards can sit side by side and a row or card is moved whole to the
//! next page when it does not fit.

use super::*;

mod band;
mod blocks;
mod board;
mod cards;
mod entries;
mod group_band;
mod head;
mod matrix;
mod panel;
mod rich;
mod shell;
mod table;

pub use band::{FactLine, FactRun, TableBand};
pub use blocks::{
    Banner, Callout, CalloutCol, Legend, LegendItem, Masthead, Metric, Metrics, Section, Side,
};
pub use board::{
    Board, BoardComment, BoardCount, BoardExtra, BoardItem, BoardItems, BoardKey, BoardLayout,
};
pub use cards::{Card, CardHead, CardPart, Cards, Field, Group, GroupItem, Tile};
pub use entries::Entry;
pub use group_band::GroupBand;
pub use head::TableHead;
pub use matrix::{Matrix, MatrixCode, MatrixCount, MatrixDay, MatrixKey, MatrixRow, MatrixTotals};
pub use panel::{CompactHeader, Panel, SceneBanner};
pub use rich::{Rich, Run};
pub use shell::{MetricStrip, NoteBand, ReportHead};
pub use table::{Cell, CellItem, Col, HeaderStyle, Row, StyledTable, Width};

use band::{band_frag, band_scan};
use entries::{entries_frag, entries_scan, put_meter, put_name};
use group_band::{group_frag, group_scan};
use head::{head_frag, head_scan};
pub(super) use matrix::{matrix, matrix_scan};
pub(super) use panel::{compact_header, compact_header_scan, scene_banner, scene_banner_scan};
use panel::{panel_frag, panel_scan};
use rich::{put_rich, rich_scan};
pub(super) use shell::{
    metric_strip, metric_strip_scan, note_band, note_band_scan, report_head, report_head_scan,
};

pub(super) use blocks::{banner, callout, legend, masthead, metrics, scan_block, section, text};
pub(super) use board::{board, board_scan};
pub(super) use cards::cards;
use cards::cards_scan;
pub(super) use table::styled_table;
use table::table_scan;

// ------------------------------------------------------------------ colour

/// An RGB colour, each channel 0.0 to 1.0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgb(pub f32, pub f32, pub f32);

impl Rgb {
    pub const fn hex(v: u32) -> Rgb {
        Rgb(
            ((v >> 16) & 255) as f32 / 255.0,
            ((v >> 8) & 255) as f32 / 255.0,
            (v & 255) as f32 / 255.0,
        )
    }
    pub const fn gray(g: f32) -> Rgb {
        Rgb(g, g, g)
    }
}

/// The palette. Text on every fill passes a 4.5 to 1 contrast check by eye in print.
pub mod pal {
    use super::Rgb;
    pub const WHITE: Rgb = Rgb::hex(0xffffff);
    pub const INK: Rgb = Rgb::hex(0x1a2438);
    pub const NAVY: Rgb = Rgb::hex(0x1a2640);
    pub const SLATE: Rgb = Rgb::hex(0x566277);
    pub const FAINT: Rgb = Rgb::hex(0x8792a5);
    pub const LINE: Rgb = Rgb::hex(0xd8dee8);
    pub const PANEL: Rgb = Rgb::hex(0xf5f7fa);
    pub const TINT: Rgb = Rgb::hex(0xe9eff5);
    pub const TEAL: Rgb = Rgb::hex(0x2a6a8c);
    pub const ORANGE: Rgb = Rgb::hex(0xe07b1f);
    pub const ORANGE_DEEP: Rgb = Rgb::hex(0x8a4f00);
    pub const ORANGE_SOFT: Rgb = Rgb::hex(0xfff1e0);
    pub const ORANGE_LINE: Rgb = Rgb::hex(0xf2c28a);
    pub const AMBER_FILL: Rgb = Rgb::hex(0xfdf0d9);
    pub const AMBER_LINE: Rgb = Rgb::hex(0xefc57e);
    pub const RED_FILL: Rgb = Rgb::hex(0xfde9e4);
    pub const RED_LINE: Rgb = Rgb::hex(0xf0b3a4);
    pub const RED_TEXT: Rgb = Rgb::hex(0x9a3822);
    pub const GREEN_FILL: Rgb = Rgb::hex(0xe2f4e9);
    pub const GREEN_LINE: Rgb = Rgb::hex(0xa6d9b8);
    pub const GREEN_TEXT: Rgb = Rgb::hex(0x1c6a3b);
    pub const BLUE_FILL: Rgb = Rgb::hex(0xe5f0f8);
    pub const BLUE_LINE: Rgb = Rgb::hex(0xb3cfe2);
    pub const BLUE_TEXT: Rgb = Rgb::hex(0x245a7d);
    pub const DARK_MUTED: Rgb = Rgb::hex(0xa9b6cb);

    /// Breakdown category accent (dot and bar), by the canonical category name.
    pub fn category(name: &str) -> Rgb {
        match name {
            "Cast" => Rgb::hex(0x7a4cc2),
            "Extras / Background" => Rgb::hex(0x4f9a3d),
            "Location / Set" => Rgb::hex(0x1b84be),
            "Props" => Rgb::hex(0xc0623a),
            "Wardrobe" => Rgb::hex(0x1f9a8a),
            "Vehicles" => Rgb::hex(0x6b7a8f),
            "Hair / Makeup" => Rgb::hex(0xd6527a),
            "Special Effects" => Rgb::hex(0xd99a1a),
            "VFX" => Rgb::hex(0x4b63d1),
            "Sound" => Rgb::hex(0xc23a96),
            "Animals" => Rgb::hex(0x8a6a3b),
            _ => Rgb::hex(0x8792a5),
        }
    }
}

/// A status tone: fill, border and text colour of a chip, callout or badge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Neutral,
    Info,
    Warn,
    Danger,
    Ok,
    Navy,
    Accent,
}

impl Tone {
    pub fn fill(self) -> Rgb {
        match self {
            Tone::Neutral => pal::PANEL,
            Tone::Info => pal::BLUE_FILL,
            Tone::Warn => pal::AMBER_FILL,
            Tone::Danger => pal::RED_FILL,
            Tone::Ok => pal::GREEN_FILL,
            Tone::Navy => pal::NAVY,
            Tone::Accent => pal::ORANGE_SOFT,
        }
    }
    pub fn line(self) -> Rgb {
        match self {
            Tone::Neutral => pal::LINE,
            Tone::Info => pal::BLUE_LINE,
            Tone::Warn => pal::AMBER_LINE,
            Tone::Danger => pal::RED_LINE,
            Tone::Ok => pal::GREEN_LINE,
            Tone::Navy => pal::NAVY,
            Tone::Accent => pal::ORANGE_LINE,
        }
    }
    pub fn text(self) -> Rgb {
        match self {
            Tone::Neutral => Rgb::hex(0x3b4659),
            Tone::Info => pal::BLUE_TEXT,
            Tone::Warn => pal::ORANGE_DEEP,
            Tone::Danger => pal::RED_TEXT,
            Tone::Ok => pal::GREEN_TEXT,
            Tone::Navy => pal::WHITE,
            Tone::Accent => pal::ORANGE_DEEP,
        }
    }
}

// ------------------------------------------------------------------ text

/// A styled run of text that wraps inside a width.
#[derive(Debug, Clone)]
pub struct Txt {
    pub text: String,
    pub size: f32,
    pub bold: bool,
    pub color: Rgb,
    /// Extra space after every character, in points (small upper-case labels).
    pub track: f32,
    pub align: Align,
    /// Space added below the text.
    pub after: f32,
    /// Line height as a multiple of the size.
    pub leading: f32,
}

impl Txt {
    pub fn new(text: impl Into<String>) -> Txt {
        Txt {
            text: text.into(),
            size: 8.5,
            bold: false,
            color: pal::INK,
            track: 0.0,
            align: Align::Left,
            after: 0.0,
            leading: 1.3,
        }
    }
    pub fn size(mut self, s: f32) -> Txt {
        self.size = s;
        self
    }
    pub fn bold(mut self) -> Txt {
        self.bold = true;
        self
    }
    pub fn color(mut self, c: Rgb) -> Txt {
        self.color = c;
        self
    }
    pub fn track(mut self, t: f32) -> Txt {
        self.track = t;
        self
    }
    pub fn align(mut self, a: Align) -> Txt {
        self.align = a;
        self
    }
    pub fn after(mut self, a: f32) -> Txt {
        self.after = a;
        self
    }
    pub fn leading(mut self, l: f32) -> Txt {
        self.leading = l;
        self
    }
    /// A small upper-case label: bold, tracked, slate.
    pub fn label(text: impl Into<String>) -> Txt {
        Txt::new(text.into().to_uppercase())
            .size(7.0)
            .bold()
            .track(0.7)
            .color(pal::SLATE)
    }
}

fn tw(text: &str, size: f32, bold: bool, track: f32) -> f32 {
    text_width(text, size, bold) + track * text.chars().count() as f32
}

fn txt_lines(t: &Txt, width: f32) -> Vec<String> {
    let w = if t.track > 0.0 { width * 0.94 } else { width };
    wrap_pt(&t.text, w.max(8.0), t.size, t.bold)
}

/// Lines of tracked capitals that fit `width` with their tracking counted (the 94 percent rule of
/// [`txt_lines`] is only an estimate, and a long name can run past a card's edge by it).
fn tracked_lines(t: &Txt, width: f32) -> Vec<String> {
    let mut share = 0.94_f32;
    loop {
        let lines = wrap_pt(&t.text, (width * share).max(8.0), t.size, t.bold);
        let fits = lines
            .iter()
            .all(|l| tw(l, t.size, t.bold, t.track) <= width + 0.01);
        if fits || share <= 0.5 {
            return lines;
        }
        share -= 0.02;
    }
}

/// The shape before a status word. With the word it carries the state without colour: a solid
/// disc, a ring with a centre dot, a hollow ring or a solid square.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    Disc,
    RingDot,
    Ring,
    Square,
}

/// Width of a mark and the gap after it, in points.
const MARK_W: f32 = 5.0;
const MARK_GAP: f32 = 3.8;

/// A pill: a short label or value with a tone.
#[derive(Debug, Clone)]
pub struct Chip {
    pub text: String,
    pub tone: Tone,
    pub size: f32,
    pub bold: bool,
    pub track: f32,
    /// A shape printed before the word (status chips); none by default.
    pub mark: Option<Mark>,
}

impl Chip {
    /// Upper-case status label ("NEEDS INPUT", "DRAFT, NOT FINAL").
    pub fn label(text: impl Into<String>, tone: Tone) -> Chip {
        Chip {
            text: text.into().to_uppercase(),
            tone,
            size: 6.6,
            bold: true,
            track: 0.5,
            mark: None,
        }
    }
    /// Regular pill that keeps its case (cast ids, element names, scene numbers).
    pub fn pill(text: impl Into<String>, tone: Tone) -> Chip {
        Chip {
            text: text.into(),
            tone,
            size: 8.0,
            bold: false,
            track: 0.0,
            mark: None,
        }
    }
    pub fn small(mut self) -> Chip {
        self.size = 7.0;
        self
    }
    pub fn bold(mut self) -> Chip {
        self.bold = true;
        self
    }
    /// Print this shape before the word.
    pub fn mark(mut self, m: Mark) -> Chip {
        self.mark = Some(m);
        self
    }
    /// Room the mark takes inside the pill.
    fn mark_room(&self) -> f32 {
        if self.mark.is_some() {
            MARK_W + MARK_GAP
        } else {
            0.0
        }
    }
    fn height(&self) -> f32 {
        self.size * 1.45 + 3.5
    }
    fn width(&self) -> f32 {
        tw(&self.text, self.size, self.bold, self.track) + 10.0 + self.mark_room()
    }
}

// ------------------------------------------------------------------ fragments

/// Drawing operations relative to a fragment's top left corner (y grows downward here).
pub(super) struct Frag {
    pub h: f32,
    pub ops: Vec<Op>,
}

impl Frag {
    pub fn new() -> Frag {
        Frag {
            h: 0.0,
            ops: Vec::new(),
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn text(
        &mut self,
        x: f32,
        base: f32,
        text: &str,
        size: f32,
        bold: bool,
        color: Rgb,
        track: f32,
    ) {
        if text.is_empty() {
            return;
        }
        self.ops.push(Op::TextC {
            bold,
            size,
            x,
            y: -base,
            text: text.to_string(),
            color,
            track,
        });
    }
    #[allow(clippy::too_many_arguments)]
    pub fn shape(
        &mut self,
        x: f32,
        dy: f32,
        w: f32,
        h: f32,
        r: f32,
        fill: Option<Rgb>,
        stroke: Option<(Rgb, f32)>,
    ) {
        self.shape_c(x, dy, w, h, r, true, true, fill, stroke);
    }
    #[allow(clippy::too_many_arguments)]
    pub fn shape_c(
        &mut self,
        x: f32,
        dy: f32,
        w: f32,
        h: f32,
        r: f32,
        round_top: bool,
        round_bottom: bool,
        fill: Option<Rgb>,
        stroke: Option<(Rgb, f32)>,
    ) {
        self.ops.push(Op::Shape {
            x,
            y: -(dy + h),
            w,
            h,
            r,
            round_top,
            round_bottom,
            fill,
            stroke,
        });
    }
    pub fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, color: Rgb, width: f32) {
        self.ops.push(Op::LineC {
            x1,
            y1: -y1,
            x2,
            y2: -y2,
            color,
            width,
        });
    }
    pub fn circle(
        &mut self,
        cx: f32,
        cy: f32,
        r: f32,
        fill: Option<Rgb>,
        stroke: Option<(Rgb, f32)>,
    ) {
        self.ops.push(Op::Circle {
            x: cx,
            y: -cy,
            r,
            fill,
            stroke,
        });
    }
    /// Merge another fragment placed at (x, dy).
    pub fn put(&mut self, other: Frag, x: f32, dy: f32) {
        for mut op in other.ops {
            shift(&mut op, x, -dy);
            self.ops.push(op);
        }
    }
}

pub(super) fn shift(op: &mut Op, dx: f32, dy: f32) {
    match op {
        Op::Text { x, y, .. } | Op::TextC { x, y, .. } => {
            *x += dx;
            *y += dy;
        }
        Op::Rect { x, y, .. }
        | Op::Frame { x, y, .. }
        | Op::Image { x, y, .. }
        | Op::Shape { x, y, .. }
        | Op::Circle { x, y, .. } => {
            *x += dx;
            *y += dy;
        }
        Op::Line { x1, y1, x2, y2 } | Op::LineC { x1, y1, x2, y2, .. } => {
            *x1 += dx;
            *x2 += dx;
            *y1 += dy;
            *y2 += dy;
        }
    }
}

impl Pager<'_> {
    /// Place a fragment with its top left corner at (x, top) on the current page.
    pub(super) fn emit(&mut self, frag: Frag, x: f32, top: f32) {
        for mut op in frag.ops {
            shift(&mut op, x, top);
            self.ops().push(op);
        }
    }
    pub(super) fn content_w(&self) -> f32 {
        self.w - 2.0 * MARGIN
    }
    /// Height of the page body (what a fresh page offers).
    pub(super) fn body_h(&self) -> f32 {
        self.h - self.inset - self.bottom
    }
}

/// Write wrapped text into a fragment; returns the height used (including `after`).
fn put_txt(f: &mut Frag, x: f32, dy: f32, width: f32, t: &Txt) -> f32 {
    let lines = txt_lines(t, width);
    let lh = t.size * t.leading;
    for (k, l) in lines.iter().enumerate() {
        let lw = tw(l, t.size, t.bold, t.track);
        let lx = match t.align {
            Align::Left => x,
            Align::Center => x + ((width - lw) / 2.0).max(0.0),
            Align::Right => x + (width - lw).max(0.0),
        };
        f.text(
            lx,
            dy + t.size + k as f32 * lh,
            l,
            t.size,
            t.bold,
            t.color,
            t.track,
        );
    }
    lines.len() as f32 * lh + t.after
}

fn txt_height(t: &Txt, width: f32) -> f32 {
    txt_lines(t, width).len() as f32 * t.size * t.leading + t.after
}

/// Lines, width and height of a chip no wider than `maxw`: long text wraps inside the pill.
fn chip_dims(c: &Chip, maxw: f32) -> (Vec<String>, f32, f32) {
    // A status chip with a shape is one line: its word is never broken to fit a narrow cell.
    if c.width() <= maxw || c.mark.is_some() {
        return (vec![c.text.clone()], c.width(), c.height());
    }
    let lines = wrap_pt(
        &c.text,
        (maxw - 10.0 - c.mark_room()).max(20.0),
        c.size,
        c.bold,
    );
    let h = lines.len() as f32 * c.size * 1.25 + 7.0;
    (lines, maxw, h)
}

/// A chip drawn at (x, dy) no wider than `maxw`; returns its width.
fn put_chip_max(f: &mut Frag, x: f32, dy: f32, maxw: f32, c: &Chip) -> f32 {
    let (lines, w, h) = chip_dims(c, maxw);
    f.shape(
        x,
        dy,
        w,
        h,
        4.0,
        Some(c.tone.fill()),
        Some((c.tone.line(), 0.6)),
    );
    if let Some(m) = c.mark {
        let cy = if lines.len() == 1 {
            dy + h / 2.0
        } else {
            dy + 3.5 + c.size * 0.5
        };
        put_mark(f, x + 4.6, cy, m, c.tone.text());
    }
    let tx = x + 5.0 + c.mark_room();
    if lines.len() == 1 {
        let base = dy + (h + 0.72 * c.size) / 2.0;
        f.text(tx, base, &lines[0], c.size, c.bold, c.tone.text(), c.track);
    } else {
        for (k, l) in lines.iter().enumerate() {
            f.text(
                tx,
                dy + 3.5 + c.size + k as f32 * c.size * 1.25,
                l,
                c.size,
                c.bold,
                c.tone.text(),
                c.track,
            );
        }
    }
    w
}

/// The shape of a status mark with its left edge at `x` and centre line at `cy`.
fn put_mark(f: &mut Frag, x: f32, cy: f32, m: Mark, color: Rgb) {
    let r = MARK_W / 2.0;
    let cx = x + r;
    match m {
        Mark::Disc => f.circle(cx, cy, r, Some(color), None),
        Mark::Ring => f.circle(cx, cy, r - 0.45, None, Some((color, 0.9))),
        Mark::RingDot => {
            f.circle(cx, cy, r - 0.45, None, Some((color, 0.9)));
            f.circle(cx, cy, 0.95, Some(color), None);
        }
        Mark::Square => f.shape(
            x + 0.2,
            cy - r + 0.2,
            MARK_W - 0.4,
            MARK_W - 0.4,
            0.0,
            Some(color),
            None,
        ),
    }
}

/// A chip drawn at (x, dy); returns its width.
fn put_chip(f: &mut Frag, x: f32, dy: f32, c: &Chip) -> f32 {
    put_chip_max(f, x, dy, f32::MAX, c)
}

/// Chips flowing left to right and wrapping; returns the height.
fn put_chips(f: &mut Frag, x: f32, dy: f32, width: f32, chips: &[Chip], align: Align) -> f32 {
    let gap = 3.5;
    let mut rows: Vec<Vec<(&Chip, f32, f32)>> = vec![Vec::new()];
    let mut used = 0.0f32;
    for c in chips {
        let (_, w, h) = chip_dims(c, width);
        if !rows.last().unwrap().is_empty() && used + gap + w > width {
            rows.push(Vec::new());
            used = 0.0;
        }
        if !rows.last().unwrap().is_empty() {
            used += gap;
        }
        used += w;
        rows.last_mut().unwrap().push((c, w, h));
    }
    let mut y = dy;
    for row in rows.iter().filter(|r| !r.is_empty()) {
        let rh = row.iter().map(|c| c.2).fold(0.0, f32::max);
        let rw: f32 = row.iter().map(|c| c.1).sum::<f32>() + gap * (row.len() as f32 - 1.0);
        let mut cx = match align {
            Align::Left => x,
            Align::Center => x + (width - rw) / 2.0,
            Align::Right => x + width - rw,
        };
        for (c, w, h) in row {
            put_chip_max(f, cx, y + (rh - h) / 2.0, width, c);
            cx += w + gap;
        }
        y += rh + gap;
    }
    (y - dy - gap).max(0.0)
}

// ------------------------------------------------------------------ drawing

/// Rectangle path with optional rounded corners (top and bottom pairs).
#[allow(clippy::too_many_arguments)]
fn rpath(c: &mut Content, x: f32, y: f32, w: f32, h: f32, r: f32, rt: bool, rb: bool) {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    if r <= 0.0 || (!rt && !rb) {
        c.rect(x, y, w, h);
        return;
    }
    let k = 0.5523 * r;
    let (rtr, rbr) = (if rt { r } else { 0.0 }, if rb { r } else { 0.0 });
    // Start bottom left, go counter clockwise.
    c.move_to(x + rbr, y);
    c.line_to(x + w - rbr, y);
    if rb {
        c.cubic_to(x + w - rbr + k, y, x + w, y + rbr - k, x + w, y + rbr);
    }
    c.line_to(x + w, y + h - rtr);
    if rt {
        c.cubic_to(
            x + w,
            y + h - rtr + k,
            x + w - rtr + k,
            y + h,
            x + w - rtr,
            y + h,
        );
    }
    c.line_to(x + rtr, y + h);
    if rt {
        c.cubic_to(x + rtr - k, y + h, x, y + h - rtr + k, x, y + h - rtr);
    }
    c.line_to(x, y + rbr);
    if rb {
        c.cubic_to(x, y + rbr - k, x + rbr - k, y, x + rbr, y);
    }
    c.close_path();
}

fn paint(c: &mut Content, fill: Option<Rgb>, stroke: Option<(Rgb, f32)>) {
    if let Some(f) = fill {
        c.set_fill_rgb(f.0, f.1, f.2);
    }
    if let Some((s, w)) = stroke {
        c.set_stroke_rgb(s.0, s.1, s.2);
        c.set_line_width(w);
    }
    match (fill.is_some(), stroke.is_some()) {
        (true, true) => c.fill_nonzero_and_stroke(),
        (true, false) => c.fill_nonzero(),
        (false, true) => c.stroke(),
        (false, false) => c.end_path(),
    };
    c.set_fill_gray(0.0);
    c.set_stroke_gray(0.0);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_shape(
    c: &mut Content,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    rt: bool,
    rb: bool,
    fill: Option<Rgb>,
    stroke: Option<(Rgb, f32)>,
) {
    rpath(c, x, y, w, h, r, rt, rb);
    paint(c, fill, stroke);
}

pub(super) fn draw_circle(
    c: &mut Content,
    cx: f32,
    cy: f32,
    r: f32,
    fill: Option<Rgb>,
    stroke: Option<(Rgb, f32)>,
) {
    let k = 0.5523 * r;
    c.move_to(cx + r, cy);
    c.cubic_to(cx + r, cy + k, cx + k, cy + r, cx, cy + r);
    c.cubic_to(cx - k, cy + r, cx - r, cy + k, cx - r, cy);
    c.cubic_to(cx - r, cy - k, cx - k, cy - r, cx, cy - r);
    c.cubic_to(cx + k, cy - r, cx + r, cy - k, cx + r, cy);
    c.close_path();
    paint(c, fill, stroke);
}

/// Text with letter spacing for Helvetica runs; text that needs the embedded fonts ignores the
/// spacing (those scripts are never spaced out).
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_text_tracked(
    b: &mut PdfBuilder,
    c: &mut Content,
    shaped: &Shaped,
    pack: &Option<FontPack>,
    bold: bool,
    size: f32,
    x: f32,
    y: f32,
    text: &str,
    track: f32,
) {
    if track > 0.0 && !needs_fonts(text) {
        let font = if bold {
            pdfgen::HELVETICA_BOLD
        } else {
            pdfgen::HELVETICA
        };
        b.text_tracked(c, font, size, track, x, y, text);
    } else {
        draw_text(b, c, shaped, pack, bold, size, x, y, text);
    }
}
