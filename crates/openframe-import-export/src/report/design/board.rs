//! The Beat Board blocks (PDF design package v2: Beat Board outline and card board).
//!
//! One block type, [`Board`], covers the whole hierarchy of the export: count tiles, the key,
//! episode, act and area bars, the sequence band, the "not in a sequence" label and the items
//! themselves, either as outline rows (number tile and text) or as boxed cards in columns.
//!
//! Items are laid out as stacked "units" (a card header, a few lines of text, a note, a comment).
//! A row of cells is placed whole; a row taller than a page is split between units, all cells of
//! the row together, and a text longer than a share of the page is cut between lines into several
//! units, so no text is ever cut off. Every piece that can start a page inside a container carries
//! the container's "continued" strip and draws it when it opens a page.

use std::collections::VecDeque;

use super::*;

const LINE_STRONG: Rgb = Rgb::hex(0xb9c2d1);
const MUTED: Rgb = Rgb::hex(0x66728a);
const BEAT_HEAD: Rgb = Rgb::hex(0xffe3c2);
/// Gap between the cards of a row and between rows of cards.
const CARD_GAP: f32 = 8.0;
const CARD_PAD: f32 = 6.4;
const HINT_SHARE: f32 = 0.4;
const SUFFIX: &str = " \u{b7} continued";

// ------------------------------------------------------------------ data

/// A comment thread on an item: the status text is always printed.
#[derive(Debug, Clone)]
pub struct BoardComment {
    pub status: String,
    pub author: String,
    pub body: String,
    /// Replies in order: author and body.
    pub replies: Vec<(String, String)>,
}

/// What hangs under an item: its own note, or a comment thread.
#[derive(Debug, Clone)]
pub enum BoardExtra {
    Note(String),
    Comment(BoardComment),
}

/// A beat or a scene card.
#[derive(Debug, Clone)]
pub struct BoardItem {
    /// The card number in board order; None in the Parking Lot (and for beats).
    pub number: Option<usize>,
    pub beat: bool,
    /// The slugline of a card.
    pub heading: String,
    /// The card description, or the text of a beat.
    pub text: String,
    pub extras: Vec<BoardExtra>,
}

#[derive(Debug, Clone)]
pub enum BoardLayout {
    /// Rows with a number tile; `nested` indents them under a sequence with a rule at the left.
    List { nested: bool },
    /// Boxed cards, `columns` to a row.
    Cards { columns: usize },
}

/// Items in one run, with the strip printed when the run opens a page.
#[derive(Debug, Clone)]
pub struct BoardItems {
    pub items: Vec<BoardItem>,
    pub layout: BoardLayout,
    /// Parking Lot items: unnumbered and tinted orange.
    pub parked: bool,
    pub strip: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BoardCount {
    pub label: String,
    pub value: String,
    /// A small line under the value ("+ 3 in Parking Lot").
    pub sub: Option<String>,
}

/// The key and the "includes" line of a printed board.
#[derive(Debug, Clone)]
pub struct BoardKey {
    pub notes: bool,
    pub comments: bool,
    /// Chips after "Includes": empty prints "Board only".
    pub includes: Vec<String>,
    pub foot: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Board {
    Counts(Vec<BoardCount>),
    Key(BoardKey),
    /// Opens a board of a project export; the film board has no eyebrow.
    Episode {
        eyebrow: Option<String>,
        title: String,
    },
    /// A navy act bar with its hint and the act's notes and comments under it.
    Act {
        title: String,
        hint: Option<String>,
        extras: Vec<BoardExtra>,
    },
    /// The tinted sequence band with its notes and comments inside it.
    Sequence {
        title: String,
        hint: Option<String>,
        extras: Vec<BoardExtra>,
        strip: Option<String>,
    },
    /// "Not in a sequence": a small label with a hairline.
    Label {
        text: String,
        strip: Option<String>,
    },
    /// Unassigned (outlined) or Parking Lot (orange) area bar.
    Area {
        title: String,
        hint: Option<String>,
        parking: bool,
        strip: Option<String>,
    },
    Items(BoardItems),
}

// ------------------------------------------------------------------ lines and units

/// One line of a unit, drawn relative to its own top left corner.
struct L {
    f: Frag,
    h: f32,
}

fn text_l(x: f32, size: f32, bold: bool, color: Rgb, track: f32, lead: f32, text: &str) -> L {
    let mut f = Frag::new();
    f.text(x, size, text, size, bold, color, track);
    L { f, h: size * lead }
}

fn wrap_l(text: &str, x: f32, w: f32, size: f32, bold: bool, color: Rgb, lead: f32) -> Vec<L> {
    wrap(text, w.max(8.0), size, bold)
        .iter()
        .map(|l| text_l(x, size, bold, color, 0.0, lead, l))
        .collect()
}

/// The text with every word wider than `width` broken into pieces that fit: the renderer's own
/// wrap breaks such a word only at the start of a line, and an unbroken string must never run
/// past the margin. Text in the embedded scripts is left to the shaped wrap, which breaks
/// between grapheme clusters.
fn fit_text(text: &str, width: f32, size: f32, bold: bool) -> String {
    let width = width.max(8.0);
    text.split('\n')
        .map(|para| {
            para.split_whitespace()
                .map(|word| {
                    if needs_fonts(word) || text_width(word, size, bold) <= width {
                        return word.to_string();
                    }
                    let mut pieces: Vec<String> = Vec::new();
                    let mut cur = String::new();
                    for ch in word.chars() {
                        let mut next = cur.clone();
                        next.push(ch);
                        if text_width(&next, size, bold) > width && !cur.is_empty() {
                            pieces.push(std::mem::take(&mut cur));
                        }
                        cur.push(ch);
                    }
                    pieces.push(cur);
                    pieces.join(" ")
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn wrap(text: &str, width: f32, size: f32, bold: bool) -> Vec<String> {
    wrap_pt(
        &fit_text(text, width, size, bold),
        width.max(8.0),
        size,
        bold,
    )
}

type Deco<'a> = &'a dyn Fn(&mut Frag, f32, bool, bool);

/// Lines cut into units no taller than `cap`; `deco(frag, height, first, last)` draws what sits
/// behind the lines of one unit. The first unit starts `top` below its edge, the last one ends
/// `bottom` above it.
fn chunk(lines: Vec<L>, cap: f32, top: f32, bottom: f32, deco: Deco) -> Vec<Frag> {
    let mut groups: Vec<Vec<L>> = vec![Vec::new()];
    let mut acc = 0.0f32;
    for l in lines {
        if acc + l.h > cap && !groups.last().unwrap().is_empty() {
            groups.push(Vec::new());
            acc = 0.0;
        }
        acc += l.h;
        groups.last_mut().unwrap().push(l);
    }
    let n = groups.len();
    groups
        .into_iter()
        .enumerate()
        .map(|(i, g)| {
            let (first, last) = (i == 0, i + 1 == n);
            let t = if first { top } else { 1.0 };
            let b = if last { bottom } else { 1.0 };
            let h = t + g.iter().map(|l| l.h).sum::<f32>() + b;
            let mut f = Frag::new();
            deco(&mut f, h, first, last);
            let mut y = t;
            for l in g {
                f.put(l.f, 0.0, y);
                y += l.h;
            }
            f.h = h;
            f
        })
        .collect()
}

fn no_deco(_: &mut Frag, _: f32, _: bool, _: bool) {}

/// A small status chip (one line high) at (x, dy); returns its width.
fn small_chip(f: &mut Frag, x: f32, dy: f32, text: &str, tone: Tone) -> f32 {
    let t = text.to_uppercase();
    let w = tw(&t, 6.6, true, 0.5) + 8.0;
    f.shape(
        x,
        dy,
        w,
        10.4,
        3.0,
        Some(tone.fill()),
        Some((tone.line(), 0.6)),
    );
    f.text(x + 4.0, dy + 7.6, &t, 6.6, true, tone.text(), 0.5);
    w
}

fn status_tone(status: &str) -> Tone {
    match status.trim().to_lowercase().as_str() {
        "open" => Tone::Warn,
        "in discussion" => Tone::Info,
        "resolved" => Tone::Ok,
        _ => Tone::Neutral,
    }
}

/// Where extras sit: the left edge, the width up to the right edge, and the small inline style of
/// a card against the wide style of the outline.
struct Geo {
    x: f32,
    w: f32,
    compact: bool,
    rule: Rgb,
}

/// An item's own note: a label, the text and a rule at the left.
fn note_units(text: &str, g: &Geo, cap: f32) -> Vec<Frag> {
    let (size, margin) = if g.compact { (7.6, 3.4) } else { (8.0, 2.6) };
    let lead = 1.3;
    let label = "NOTE";
    let lw = tw(label, 6.6, true, 0.6);
    let tx = g.x + 5.5 + if g.compact { 0.0 } else { 29.0 };
    let wt = g.x + g.w - tx;
    let mut lines: Vec<L> = Vec::new();
    if g.compact {
        // The label sits inline: the first line leaves room for it.
        let dash = text_width("-", size, false).max(0.5);
        let filler = "-".repeat(((lw + 4.0) / dash).ceil() as usize);
        let wrapped = wrap(&format!("{filler} {text}"), wt, size, false);
        for (i, l) in wrapped.iter().enumerate() {
            if i == 0 {
                let rest = l.strip_prefix(filler.as_str()).unwrap_or(l).trim_start();
                let mut first = text_l(tx + lw + 4.0, size, false, pal::SLATE, 0.0, lead, rest);
                first
                    .f
                    .text(tx, 6.6 + 0.6, label, 6.6, true, pal::SLATE, 0.6);
                lines.push(first);
            } else {
                lines.push(text_l(tx, size, false, pal::SLATE, 0.0, lead, l));
            }
        }
    } else {
        for (i, l) in wrap(text, wt, size, false).iter().enumerate() {
            let mut line = text_l(tx, size, false, pal::SLATE, 0.0, lead, l);
            if i == 0 {
                line.f
                    .text(g.x + 5.5, 6.6 + 1.2, label, 6.6, true, pal::SLATE, 0.6);
            }
            lines.push(line);
        }
    }
    let rule = g.rule;
    let x = g.x;
    let deco = move |f: &mut Frag, h: f32, first: bool, _last: bool| {
        let dy = if first { margin } else { 0.0 };
        f.shape(x, dy, 1.5, h - dy, 0.0, Some(rule), None);
    };
    chunk(lines, cap, margin, 0.0, &deco)
}

/// A comment thread: a white box with a teal rule, status chip, author, body and replies.
fn comment_units(c: &BoardComment, g: &Geo, cap: f32) -> Vec<Frag> {
    let (size, margin) = if g.compact { (7.6, 3.6) } else { (8.0, 3.2) };
    let lead = 1.3;
    let ix = g.x + 8.5;
    let iw = (g.w - 8.5 - 6.0).max(30.0);
    let mut lines: Vec<L> = Vec::new();
    let head =
        |label: &str, chip: Option<(&str, Tone)>, author: &str, x0: f32, w0: f32| -> Vec<L> {
            let lw = tw(label, 6.6, true, 0.6);
            let mut f = Frag::new();
            f.text(x0, 8.2, label, 6.6, true, pal::SLATE, 0.6);
            let mut x = x0 + lw + 5.0;
            if let Some((t, tone)) = chip {
                x += small_chip(&mut f, x, 0.8, t, tone) + 5.0;
            }
            let author = author.trim();
            let mut out = Vec::new();
            if !author.is_empty() && x + tw(author, 7.8, true, 0.0) <= x0 + w0 {
                f.text(x, 8.4, author, 7.8, true, pal::NAVY, 0.0);
                out.push(L { f, h: 11.6 });
            } else {
                out.push(L { f, h: 11.6 });
                for l in wrap(author, w0, 7.8, true) {
                    if !l.is_empty() {
                        out.push(text_l(x0, 7.8, true, pal::NAVY, 0.0, 1.3, &l));
                    }
                }
            }
            out
        };
    lines.extend(head(
        "COMMENT",
        Some((&c.status, status_tone(&c.status))),
        &c.author,
        ix,
        iw,
    ));
    lines.extend(wrap_l(c.body.trim(), ix, iw, size, false, pal::INK, lead));
    for (author, body) in &c.replies {
        let rx = ix + 7.0;
        let rw = iw - 7.0;
        let mut block = head("REPLY", None, author, rx, rw);
        block.extend(wrap_l(body.trim(), rx, rw, size, false, pal::INK, lead));
        for mut l in block {
            // The reply rule runs through every line of the reply.
            l.f.line(rx - 3.5, 0.0, rx - 3.5, l.h, LINE_STRONG, 0.5);
            lines.push(l);
        }
    }
    let x = g.x;
    let w = g.w;
    let deco = move |f: &mut Frag, h: f32, first: bool, last: bool| {
        let dy = if first { margin } else { 0.0 };
        let bh = h - dy;
        f.shape_c(
            x,
            dy,
            w,
            bh,
            3.0,
            first,
            last,
            Some(pal::WHITE),
            Some((LINE_STRONG, 0.5)),
        );
        f.shape(x, dy, 2.5, bh, 1.2, Some(pal::TEAL), None);
    };
    chunk(lines, cap, margin + 3.6, 4.2, &deco)
}

fn extras_units(extras: &[BoardExtra], g: &Geo, cap: f32) -> Vec<Frag> {
    let mut out = Vec::new();
    for e in extras {
        match e {
            BoardExtra::Note(t) => out.extend(note_units(t, g, cap)),
            BoardExtra::Comment(c) => out.extend(comment_units(c, g, cap)),
        }
    }
    out
}

// ------------------------------------------------------------------ items

/// How a cell looks behind and around its units.
#[derive(Clone)]
enum Look {
    Card {
        fill: Rgb,
        border: Rgb,
    },
    Row {
        nested: bool,
        beat: bool,
        /// Left edge of the row's own content (the indent under a sequence).
        ix: f32,
    },
}

struct Cell {
    units: Vec<Frag>,
    look: Look,
}

fn tile_number(f: &mut Frag, x: f32, dy: f32, n: usize, small: bool) -> f32 {
    let t = n.to_string();
    let size = if small { 7.6 } else { 8.4 };
    let w = (tw(&t, size, true, 0.0) + 10.0).max(if small { 20.0 } else { 26.0 });
    let h = if small { 11.4 } else { 12.0 };
    f.shape(x, dy, w, h, 3.0, Some(pal::NAVY), None);
    f.text(
        x + (w - tw(&t, size, true, 0.0)) / 2.0,
        dy + h / 2.0 + size * 0.36,
        &t,
        size,
        true,
        pal::WHITE,
        0.0,
    );
    w
}

/// An outline row: tile at the left, heading and description, then notes and comments.
fn row_cell(it: &BoardItem, nested: bool, cw: f32, cap: f32, parked: bool) -> Cell {
    let ix = if nested { 10.0 } else { 0.0 };
    let pad = if it.beat { 6.0 } else { 0.0 };
    let tx = ix + pad + 36.0;
    let wt = cw - tx - pad;
    let mut lines: Vec<L> = Vec::new();
    if it.beat {
        lines.extend(wrap_l(&it.text, tx, wt, 9.0, true, pal::NAVY, 1.3));
    } else {
        if !it.heading.trim().is_empty() {
            lines.extend(wrap_l(&it.heading, tx, wt, 8.8, true, pal::NAVY, 1.25));
        }
        if !it.text.trim().is_empty() {
            lines.extend(wrap_l(&it.text, tx, wt, 8.8, false, pal::INK, 1.3));
        }
        if lines.is_empty() {
            lines.push(text_l(
                tx,
                8.8,
                false,
                MUTED,
                0.0,
                1.3,
                "No heading or description",
            ));
        }
    }
    let mut units = chunk(lines, cap, 5.0, 1.6, &no_deco);
    // The tile sits beside the first line.
    let mut tile = Frag::new();
    match (it.beat, it.number) {
        (true, _) => {
            tile.shape(
                ix + pad,
                5.4,
                26.0,
                11.2,
                3.0,
                Some(pal::WHITE),
                Some((pal::ORANGE_LINE, 0.6)),
            );
            let w = tw("BEAT", 6.6, true, 0.6);
            tile.text(
                ix + pad + (26.0 - w) / 2.0,
                5.4 + 8.0,
                "BEAT",
                6.6,
                true,
                pal::ORANGE_DEEP,
                0.6,
            );
        }
        (false, Some(n)) if !parked => {
            tile_number(&mut tile, ix, 5.2, n, false);
        }
        _ => {
            tile.shape(
                ix,
                5.4,
                26.0,
                11.2,
                3.0,
                Some(pal::WHITE),
                Some((LINE_STRONG, 0.6)),
            );
            let w = tw("CARD", 6.6, true, 0.6);
            tile.text(
                ix + (26.0 - w) / 2.0,
                5.4 + 8.0,
                "CARD",
                6.6,
                true,
                pal::NAVY,
                0.6,
            );
        }
    }
    units[0].put(tile, 0.0, 0.0);
    let geo = Geo {
        x: tx,
        w: wt,
        compact: false,
        rule: LINE_STRONG,
    };
    units.extend(extras_units(&it.extras, &geo, cap));
    if let Some(last) = units.last_mut() {
        last.h += 4.0;
    }
    Cell {
        units,
        look: Look::Row {
            nested,
            beat: it.beat,
            ix,
        },
    }
}

/// A boxed card: header strip with the number tile, then the text, notes and comments.
fn card_cell(it: &BoardItem, w: f32, cap: f32, parked: bool) -> Cell {
    let (fill, border, head_fill, head_line, label, label_color) = if it.beat {
        (
            pal::ORANGE_SOFT,
            pal::ORANGE_LINE,
            BEAT_HEAD,
            pal::ORANGE_LINE,
            "BEAT",
            pal::ORANGE_DEEP,
        )
    } else if parked {
        (
            pal::WHITE,
            pal::ORANGE_LINE,
            pal::ORANGE_SOFT,
            pal::ORANGE_LINE,
            "SCENE CARD",
            pal::NAVY,
        )
    } else {
        (
            pal::WHITE,
            LINE_STRONG,
            pal::TINT,
            LINE_STRONG,
            "SCENE CARD",
            pal::NAVY,
        )
    };
    let hh = 17.6;
    let mut head = Frag::new();
    head.shape_c(0.0, 0.0, w, hh, 6.0, true, false, Some(head_fill), None);
    head.line(0.0, hh, w, hh, head_line, 0.5);
    let mut x = CARD_PAD;
    if !it.beat
        && !parked
        && let Some(n) = it.number
    {
        x += tile_number(&mut head, x, (hh - 11.4) / 2.0, n, true) + 5.0;
    }
    head.text(x, hh / 2.0 + 2.5, label, 6.8, true, label_color, 0.68);
    head.h = hh;

    let wt = w - 2.0 * CARD_PAD;
    let mut lines: Vec<L> = Vec::new();
    if it.beat {
        lines.extend(wrap_l(&it.text, CARD_PAD, wt, 8.6, true, pal::NAVY, 1.3));
    } else {
        if !it.heading.trim().is_empty() {
            lines.extend(wrap_l(
                &it.heading,
                CARD_PAD,
                wt,
                8.4,
                true,
                pal::NAVY,
                1.25,
            ));
        }
        if !it.text.trim().is_empty() {
            lines.extend(wrap_l(&it.text, CARD_PAD, wt, 8.4, false, pal::INK, 1.3));
        }
        if lines.is_empty() {
            lines.push(text_l(
                CARD_PAD,
                8.4,
                false,
                MUTED,
                0.0,
                1.3,
                "No heading or description",
            ));
        }
    }
    let mut units = vec![head];
    units.extend(chunk(lines, cap, 4.6, 1.0, &no_deco));
    let geo = Geo {
        x: CARD_PAD,
        w: wt,
        compact: true,
        rule: if it.beat || parked {
            pal::ORANGE_LINE
        } else {
            LINE_STRONG
        },
    };
    units.extend(extras_units(&it.extras, &geo, cap));
    if let Some(last) = units.last_mut() {
        last.h += 4.4;
    }
    Cell {
        units,
        look: Look::Card { fill, border },
    }
}

// ------------------------------------------------------------------ placing

/// The strip that names the container a page starts in.
fn strip_frag(text: &str, cw: f32) -> Frag {
    let t = Txt::new(fit_text(&text.to_uppercase(), cw * 0.9, 6.8, true))
        .size(6.8)
        .bold()
        .track(0.54);
    let mut lines = tracked_lines(&t, cw);
    let sw = tw(SUFFIX, 6.8, false, 0.2);
    let last_w = lines.last().map_or(0.0, |l| tw(l, 6.8, true, 0.54));
    let mut f = Frag::new();
    let lh = 6.8 * 1.3;
    let mut suffix_at = (last_w, lines.len() as f32 - 1.0);
    if last_w + sw > cw {
        lines.push(String::new());
        suffix_at = (0.0, lines.len() as f32 - 1.0);
    }
    for (k, l) in lines.iter().enumerate() {
        f.text(0.0, 6.8 + k as f32 * lh, l, 6.8, true, pal::SLATE, 0.54);
    }
    f.text(
        suffix_at.0,
        6.8 + suffix_at.1 * lh,
        SUFFIX,
        6.8,
        false,
        pal::SLATE,
        0.2,
    );
    let rule = lines.len() as f32 * lh + 2.6;
    f.line(0.0, rule, cw, rule, LINE_STRONG, 0.5);
    f.h = rule + 7.0;
    f
}

/// Make room for `need` points; a page that opens here starts with the strip.
fn begin(p: &mut Pager, strip: Option<&str>, need: f32) {
    let sf = strip.map(|s| strip_frag(s, p.content_w()));
    let sh = sf.as_ref().map_or(0.0, |f| f.h);
    let extra = if p.fresh() { sh } else { 0.0 };
    if p.room() < need + extra {
        p.new_page();
    }
    if let Some(f) = sf
        && p.fresh()
    {
        let (top, h) = (p.y, f.h);
        p.emit(f, MARGIN, top);
        p.y -= h;
    }
}

/// Cap of one unit: no unit may be taller than this, so one always fits an empty page.
fn unit_cap(p: &Pager) -> f32 {
    (p.body_h() * 0.4).clamp(60.0, 260.0)
}

fn paint_cell(
    look: &Look,
    units: Vec<Frag>,
    w: f32,
    h: f32,
    cw_row: f32,
    first: bool,
    last: bool,
) -> Frag {
    let mut f = Frag::new();
    match look {
        Look::Card { fill, .. } => {
            f.shape_c(0.0, 0.0, w, h, 6.0, first, last, Some(*fill), None);
        }
        Look::Row { beat: true, ix, .. } => {
            f.shape(
                *ix,
                1.5,
                cw_row - ix,
                h - 3.0,
                3.0,
                Some(pal::ORANGE_SOFT),
                Some((pal::ORANGE_LINE, 0.6)),
            );
        }
        Look::Row { .. } => {}
    }
    let mut y = 0.0;
    for u in units {
        let uh = u.h;
        f.put(u, 0.0, y);
        y += uh;
    }
    match look {
        Look::Card { border, .. } => {
            f.shape_c(0.0, 0.0, w, h, 6.0, first, last, None, Some((*border, 0.5)));
        }
        Look::Row { nested, beat, ix } => {
            if *nested {
                f.line(0.6, 0.0, 0.6, h, LINE_STRONG, 1.2);
            }
            if !*beat && last {
                f.line(*ix, h - 0.25, cw_row, h - 0.25, pal::LINE, 0.5);
            }
        }
    }
    f.h = h;
    f
}

/// Place a row of cells: whole when it fits a page, split between units when it does not.
fn place_row(p: &mut Pager, strip: Option<&str>, cells: Vec<Cell>, w: f32, gap_y: f32) {
    let strip_h = strip.map_or(0.0, |s| strip_frag(s, p.content_w()).h);
    let row_w = p.content_w();
    let cap_page = p.body_h() - strip_h;
    let n = cells.len();
    let row_h = cells
        .iter()
        .map(|c| c.units.iter().map(|u| u.h).sum::<f32>())
        .fold(0.0, f32::max);
    let xs: Vec<f32> = (0..n).map(|i| MARGIN + i as f32 * (w + CARD_GAP)).collect();
    if row_h + 1.0 <= cap_page {
        begin(p, strip, row_h + 1.0);
        let top = p.y;
        for (i, c) in cells.into_iter().enumerate() {
            let f = paint_cell(&c.look, c.units, w, row_h, row_w, true, true);
            p.emit(f, xs[i], top);
        }
        p.y -= row_h + gap_y;
        return;
    }
    // Taller than a page: all cells continue together, page after page.
    let looks: Vec<Look> = cells.iter().map(|c| c.look.clone()).collect();
    let mut left: Vec<VecDeque<Frag>> = cells.into_iter().map(|c| c.units.into()).collect();
    let mut first = vec![true; n];
    loop {
        let need = left
            .iter()
            .filter_map(|q| q.front().map(|u| u.h))
            .fold(0.0, f32::max);
        begin(p, strip, need + 1.0);
        let avail = p.room();
        let mut taken: Vec<(usize, Vec<Frag>, bool, bool)> = Vec::new();
        let mut seg_h = 0.0f32;
        for (i, q) in left.iter_mut().enumerate() {
            let mut got: Vec<Frag> = Vec::new();
            let mut sum = 0.0;
            while q.front().is_some_and(|u| sum + u.h <= avail) {
                let u = q.pop_front().unwrap();
                sum += u.h;
                got.push(u);
            }
            if !got.is_empty() {
                seg_h = seg_h.max(sum);
                taken.push((i, got, first[i], q.is_empty()));
                first[i] = false;
            }
        }
        let top = p.y;
        for (i, units, f1, l1) in taken {
            let f = paint_cell(&looks[i], units, w, seg_h, row_w, f1, l1);
            p.emit(f, xs[i], top);
        }
        p.y -= seg_h;
        if left.iter().all(|q| q.is_empty()) {
            p.y -= gap_y;
            return;
        }
        p.new_page();
    }
}

// ------------------------------------------------------------------ bars

fn hint_lines(hint: Option<&str>, cw: f32, size: f32) -> (Vec<String>, f32) {
    match hint {
        Some(h) if !h.trim().is_empty() => {
            let w = tw(h, size, false, 0.0).min(cw * HINT_SHARE);
            (wrap(h, w, size, false), w)
        }
        _ => (Vec::new(), 0.0),
    }
}

/// A full-width bar: the act (navy), Unassigned (outlined) or the Parking Lot (orange).
fn bar_frag(cw: f32, title: &str, hint: Option<&str>, kind: u8) -> Frag {
    // kind: 0 act, 1 unassigned, 2 parking lot
    let (fill, stroke, color, hint_color, stripe) = match kind {
        0 => (pal::NAVY, None, pal::WHITE, pal::DARK_MUTED, 0.0),
        1 => (
            pal::WHITE,
            Some((pal::NAVY, 1.2)),
            pal::NAVY,
            pal::SLATE,
            0.0,
        ),
        _ => (
            pal::ORANGE_SOFT,
            Some((pal::ORANGE_LINE, 1.0)),
            pal::ORANGE_DEEP,
            pal::ORANGE_DEEP,
            5.0,
        ),
    };
    let (hl, hw) = hint_lines(hint, cw, 7.4);
    let x = 9.0 + stripe;
    let tw_avail = (cw - x - 9.0 - if hw > 0.0 { hw + 12.0 } else { 0.0 }).max(60.0);
    let t = Txt::new(fit_text(&title.to_uppercase(), tw_avail * 0.92, 10.5, true))
        .size(10.5)
        .bold()
        .track(0.63)
        .color(color)
        .leading(1.2);
    let lines = tracked_lines(&t, tw_avail);
    let h = (lines.len() as f32 * 12.6 + 11.5).max(hl.len() as f32 * 9.3 + 11.5);
    let mut f = Frag::new();
    f.shape(0.0, 0.0, cw, h, 3.0, Some(fill), stroke);
    if stripe > 0.0 {
        f.shape_c(
            0.0,
            0.0,
            stripe + 0.5,
            h,
            3.0,
            true,
            true,
            Some(pal::ORANGE),
            None,
        );
    }
    for (k, l) in lines.iter().enumerate() {
        f.text(x, 5.5 + 10.5 + k as f32 * 12.6, l, 10.5, true, color, 0.63);
    }
    for (k, l) in hl.iter().enumerate() {
        let lw = tw(l, 7.4, false, 0.0);
        f.text(
            cw - 9.0 - lw,
            16.0 + k as f32 * 9.3,
            l,
            7.4,
            false,
            hint_color,
            0.0,
        );
    }
    f.h = h;
    f
}

/// Place frags one after another, each moved to a new page when it does not fit.
fn flow(p: &mut Pager, strip: Option<&str>, units: Vec<Frag>, x: f32) {
    for u in units {
        begin(p, strip, u.h);
        let (top, h) = (p.y, u.h);
        p.emit(u, x, top);
        p.y -= h;
    }
}

fn act(p: &mut Pager, title: &str, hint: Option<&str>, extras: &[BoardExtra]) {
    let cw = p.content_w();
    let bar = bar_frag(cw, title, hint, 0);
    let gap = 12.0;
    begin(p, None, bar.h + gap + 2.0);
    p.y -= gap;
    let (top, h) = (p.y, bar.h);
    p.emit(bar, MARGIN, top);
    p.y -= h + 4.0;
    let geo = Geo {
        x: 0.0,
        w: cw,
        compact: false,
        rule: LINE_STRONG,
    };
    flow(p, None, extras_units(extras, &geo, unit_cap(p)), MARGIN);
}

fn area(p: &mut Pager, title: &str, hint: Option<&str>, parking: bool, strip: Option<&str>) {
    let cw = p.content_w();
    let bar = bar_frag(cw, title, hint, if parking { 2 } else { 1 });
    let gap = 16.0;
    begin(p, strip, bar.h + gap + 2.0);
    p.y -= gap;
    let (top, h) = (p.y, bar.h);
    p.emit(bar, MARGIN, top);
    p.y -= h + 6.0;
}

fn sequence(
    p: &mut Pager,
    title: &str,
    hint: Option<&str>,
    extras: &[BoardExtra],
    strip: Option<&str>,
) {
    let cw = p.content_w();
    let (hl, hw) = hint_lines(hint, cw, 7.4);
    let label = "SEQUENCE";
    let lw = tw(label, 6.6, true, 0.8);
    let x = 11.0;
    let tx = x + lw + 7.0;
    let wt = (cw - tx - 8.0 - if hw > 0.0 { hw + 12.0 } else { 0.0 }).max(60.0);
    let mut lines: Vec<L> = Vec::new();
    for (k, l) in wrap(title, wt, 9.8, true).iter().enumerate() {
        let mut line = text_l(tx, 9.8, true, pal::NAVY, 0.0, 1.22, l);
        if k == 0 {
            line.f.text(x, 9.8, label, 6.6, true, pal::SLATE, 0.8);
        }
        lines.push(line);
    }
    // The hint sits at the right, line by line beside the title.
    for (j, hline) in hl.iter().enumerate() {
        if lines.len() <= j {
            lines.push(L {
                f: Frag::new(),
                h: 9.8 * 1.22,
            });
        }
        let w = tw(hline, 7.4, false, 0.0);
        lines[j]
            .f
            .text(cw - 8.0 - w, 9.8, hline, 7.4, false, pal::SLATE, 0.0);
    }
    let geo = Geo {
        x: x - 2.5,
        w: cw - x - 5.5,
        compact: false,
        rule: LINE_STRONG,
    };
    let cap = unit_cap(p);
    let closes = extras.is_empty();
    let deco = move |f: &mut Frag, h: f32, first: bool, last: bool| {
        let last = last && closes;
        f.shape_c(0.0, 0.0, cw, h, 3.0, first, last, Some(pal::TINT), None);
        f.shape_c(0.0, 0.0, 3.0, h, 1.5, first, last, Some(pal::ORANGE), None);
    };
    let mut units = chunk(
        lines,
        cap,
        4.6,
        if extras.is_empty() { 5.0 } else { 1.0 },
        &deco,
    );
    if !extras.is_empty() {
        let mut ex = extras_units(extras, &geo, cap);
        // The extras sit inside the band: paint the band behind each of them.
        for u in ex.iter_mut() {
            let mut back = Frag::new();
            back.shape_c(0.0, 0.0, cw, u.h, 0.0, false, false, Some(pal::TINT), None);
            back.shape_c(
                0.0,
                0.0,
                3.0,
                u.h,
                0.0,
                false,
                false,
                Some(pal::ORANGE),
                None,
            );
            let ops = std::mem::take(&mut u.ops);
            back.ops.extend(ops);
            u.ops = back.ops;
        }
        if let Some(last) = ex.last_mut() {
            // Close the band under the last extra.
            let mut close = Frag::new();
            close.shape_c(
                0.0,
                last.h,
                cw,
                5.0,
                3.0,
                false,
                true,
                Some(pal::TINT),
                None,
            );
            close.shape_c(
                0.0,
                last.h,
                3.0,
                5.0,
                1.5,
                false,
                true,
                Some(pal::ORANGE),
                None,
            );
            last.ops.extend(close.ops);
            last.h += 5.0;
        }
        units.extend(ex);
    }
    // Space above the band.
    let gap = 9.0;
    let first_h = units.first().map_or(0.0, |u| u.h);
    begin(p, strip, first_h + gap + 2.0);
    p.y -= gap;
    flow(p, strip, units, MARGIN);
    p.y -= 3.0;
}

fn label(p: &mut Pager, text: &str, strip: Option<&str>) {
    let cw = p.content_w();
    let t = text.to_uppercase();
    let w = tw(&t, 6.6, true, 0.8);
    let mut f = Frag::new();
    f.text(0.0, 6.6 + 8.0, &t, 6.6, true, pal::SLATE, 0.8);
    f.line(w + 7.0, 8.0 + 3.4, cw, 8.0 + 3.4, LINE_STRONG, 0.5);
    f.h = 8.0 + 9.0 + 2.0;
    begin(p, strip, f.h + 2.0);
    let (top, h) = (p.y, f.h);
    p.emit(f, MARGIN, top);
    p.y -= h;
}

fn episode(p: &mut Pager, eyebrow: Option<&str>, title: &str) {
    let cw = p.content_w();
    let mut f = Frag::new();
    f.shape(0.0, 0.0, cw, 3.0, 0.0, Some(pal::ORANGE), None);
    let mut y = 8.0;
    if let Some(e) = eyebrow {
        f.text(0.0, y + 6.8, &e.to_uppercase(), 6.8, true, pal::SLATE, 0.95);
        y += 6.8 * 1.3 + 2.0;
    }
    let t = Txt::new(title.to_string())
        .size(15.0)
        .bold()
        .color(pal::NAVY)
        .leading(1.15);
    y += put_txt(&mut f, 0.0, y, cw, &t) + 5.0;
    f.line(0.0, y, cw, y, pal::NAVY, 1.2);
    f.h = y + 1.2;
    begin(p, None, f.h + 2.0);
    let (top, h) = (p.y, f.h);
    p.emit(f, MARGIN, top);
    p.y -= h + 6.0;
}

// ------------------------------------------------------------------ counts and key

fn counts(p: &mut Pager, items: &[BoardCount]) {
    if items.is_empty() {
        return;
    }
    let cw = p.content_w();
    let n = items.len();
    let gap = 8.0;
    let w = (cw - gap * (n as f32 - 1.0)) / n as f32;
    let any_sub = items.iter().any(|i| i.sub.is_some());
    let h = 4.5 + 8.84 + 1.5 + 15.0 + if any_sub { 10.0 } else { 0.0 } + 5.0;
    let mut f = Frag::new();
    for (i, it) in items.iter().enumerate() {
        let x = i as f32 * (w + gap);
        f.shape(x, 0.0, w, h, 4.0, Some(pal::PANEL), Some((pal::LINE, 0.5)));
        f.text(
            x + 8.0,
            4.5 + 6.8,
            &it.label.to_uppercase(),
            6.8,
            true,
            pal::SLATE,
            0.7,
        );
        f.text(
            x + 8.0,
            4.5 + 8.84 + 1.5 + 12.4,
            &it.value,
            13.0,
            true,
            pal::NAVY,
            0.0,
        );
        if let Some(s) = &it.sub {
            f.text(
                x + 8.0,
                4.5 + 8.84 + 1.5 + 15.0 + 7.2,
                s,
                7.0,
                false,
                pal::SLATE,
                0.0,
            );
        }
    }
    p.ensure(h + 8.0);
    let top = p.y;
    p.emit(f, MARGIN, top);
    p.y -= h + 8.0;
}

/// One element of a key line.
enum Part {
    Tile(String),
    Chip(&'static str, Tone),
    Label(&'static str),
    Text(String),
}

fn part_w(x: &Part) -> f32 {
    match x {
        Part::Tile(t) => (tw(t, 7.6, true, 0.0) + 10.0).max(20.0),
        Part::Chip(t, _) => tw(t, 6.6, true, 0.5) + 8.0,
        Part::Label(t) => tw(t, 6.6, true, 0.6),
        Part::Text(t) => tw(t, 7.6, false, 0.0),
    }
}

fn put_part(f: &mut Frag, x: f32, dy: f32, part: &Part) {
    match part {
        Part::Tile(t) => {
            let w = part_w(part);
            f.shape(x, dy, w, 11.4, 3.0, Some(pal::NAVY), None);
            f.text(
                x + (w - tw(t, 7.6, true, 0.0)) / 2.0,
                dy + 8.2,
                t,
                7.6,
                true,
                pal::WHITE,
                0.0,
            );
        }
        Part::Chip(t, tone) => {
            small_chip(f, x, dy + 0.5, t, *tone);
        }
        Part::Label(t) => f.text(x, dy + 8.0, t, 6.6, true, pal::SLATE, 0.6),
        Part::Text(t) => f.text(x, dy + 8.2, t, 7.6, false, pal::SLATE, 0.0),
    }
}

fn key(p: &mut Pager, k: &BoardKey) {
    let cw = p.content_w();
    let pad = 8.0;
    let col = 52.0;
    let avail = cw - pad - col - pad;
    let mut items: Vec<Vec<Part>> = vec![vec![
        Part::Tile("12".into()),
        Part::Text("Scene Card, in board order (not the screenplay scene number)".into()),
    ]];
    items.push(vec![
        Part::Chip("Beat", Tone::Accent),
        Part::Text("Story beat".into()),
    ]);
    if k.notes {
        items.push(vec![
            Part::Label("NOTE"),
            Part::Text("the item's own note".into()),
        ]);
    }
    if k.comments {
        items.push(vec![
            Part::Chip("Open", Tone::Warn),
            Part::Text("comment thread (status, author, replies)".into()),
        ]);
    }
    let mut f = Frag::new();
    let mut y = 6.0f32;
    // Row 1: the key, flowing and wrapping.
    f.text(pad, y + 8.0, "KEY", 6.6, true, pal::NAVY, 0.6);
    let (mut x, mut ly) = (0.0f32, y);
    for it in &items {
        let gaps = 5.0 * (it.len() as f32 - 1.0);
        let w: f32 = it.iter().map(part_w).sum::<f32>() + gaps;
        if x > 0.0 && x + w > avail {
            x = 0.0;
            ly += 14.0;
        }
        let mut px = x;
        for part in it {
            put_part(&mut f, pad + col + px, ly, part);
            px += part_w(part) + 5.0;
        }
        x += w + 14.0;
    }
    y = ly + 14.0 + 3.0;
    f.line(pad, y, cw - pad, y, pal::LINE, 0.5);
    y += 5.0;
    // Row 2: what the export includes.
    f.text(pad, y + 8.0, "INCLUDES", 6.6, true, pal::NAVY, 0.6);
    let foot = k.foot.as_deref().unwrap_or("");
    let fw = if foot.is_empty() {
        0.0
    } else {
        tw(foot, 7.6, false, 0.0)
    };
    let chips: Vec<String> = if k.includes.is_empty() {
        vec!["Board only".to_string()]
    } else {
        k.includes.clone()
    };
    let (mut cx, mut cy) = (0.0f32, y);
    let cav = (avail - fw - 10.0).max(80.0);
    for c in &chips {
        let w = tw(&c.to_uppercase(), 6.6, true, 0.5) + 8.0;
        if cx > 0.0 && cx + w > cav {
            cx = 0.0;
            cy += 14.0;
        }
        small_chip(&mut f, pad + col + cx, cy + 0.5, c, Tone::Neutral);
        cx += w + 4.0;
    }
    if fw > 0.0 {
        f.text(cw - pad - fw, y + 8.2, foot, 7.6, false, pal::SLATE, 0.0);
    }
    y = cy + 12.0 + 6.0;
    let mut boxf = Frag::new();
    boxf.shape(
        0.0,
        0.0,
        cw,
        y,
        4.0,
        Some(pal::WHITE),
        Some((pal::LINE, 0.5)),
    );
    boxf.put(f, 0.0, 0.0);
    boxf.h = y;
    p.ensure(y + 8.0);
    let top = p.y;
    p.emit(boxf, MARGIN, top);
    p.y -= y + 10.0;
}

// ------------------------------------------------------------------ entry points

pub(in crate::report) fn board(p: &mut Pager, b: &Board) {
    match b {
        Board::Counts(c) => counts(p, c),
        Board::Key(k) => key(p, k),
        Board::Episode { eyebrow, title } => episode(p, eyebrow.as_deref(), title),
        Board::Act {
            title,
            hint,
            extras,
        } => act(p, title, hint.as_deref(), extras),
        Board::Sequence {
            title,
            hint,
            extras,
            strip,
        } => sequence(p, title, hint.as_deref(), extras, strip.as_deref()),
        Board::Label { text, strip } => label(p, text, strip.as_deref()),
        Board::Area {
            title,
            hint,
            parking,
            strip,
        } => area(p, title, hint.as_deref(), *parking, strip.as_deref()),
        Board::Items(it) => items(p, it),
    }
}

fn items(p: &mut Pager, it: &BoardItems) {
    let cw = p.content_w();
    let cap = unit_cap(p);
    let strip = it.strip.as_deref();
    match &it.layout {
        BoardLayout::List { nested } => {
            for item in &it.items {
                let cell = row_cell(item, *nested, cw, cap, it.parked);
                place_row(p, strip, vec![cell], cw, 0.0);
            }
        }
        BoardLayout::Cards { columns } => {
            let cols = (*columns).clamp(1, 6);
            let w = (cw - CARD_GAP * (cols as f32 - 1.0)) / cols as f32;
            for row in it.items.chunks(cols) {
                let cells = row
                    .iter()
                    .map(|i| card_cell(i, w, cap, it.parked))
                    .collect();
                place_row(p, strip, cells, w, CARD_GAP);
            }
        }
    }
}

fn extras_scan(extras: &[BoardExtra], scan: &mut dyn FnMut(&str)) {
    for e in extras {
        match e {
            BoardExtra::Note(t) => scan(t),
            BoardExtra::Comment(c) => {
                scan(&c.status);
                scan(&c.author);
                scan(&c.body);
                for (a, b) in &c.replies {
                    scan(a);
                    scan(b);
                }
            }
        }
    }
}

pub(in crate::report) fn board_scan(b: &Board, scan: &mut dyn FnMut(&str)) {
    match b {
        Board::Counts(c) => c.iter().for_each(|i| {
            scan(&i.label);
            scan(&i.value);
            if let Some(s) = &i.sub {
                scan(s);
            }
        }),
        Board::Key(k) => {
            k.includes.iter().for_each(|i| scan(i));
            if let Some(f) = &k.foot {
                scan(f);
            }
        }
        Board::Episode { eyebrow, title } => {
            if let Some(e) = eyebrow {
                scan(e);
            }
            scan(title);
        }
        Board::Act {
            title,
            hint,
            extras,
        }
        | Board::Sequence {
            title,
            hint,
            extras,
            ..
        } => {
            scan(title);
            if let Some(h) = hint {
                scan(h);
            }
            extras_scan(extras, scan);
        }
        Board::Label { text, strip } => {
            scan(text);
            if let Some(s) = strip {
                scan(s);
            }
        }
        Board::Area {
            title, hint, strip, ..
        } => {
            scan(title);
            if let Some(h) = hint {
                scan(h);
            }
            if let Some(s) = strip {
                scan(s);
            }
        }
        Board::Items(it) => {
            if let Some(s) = &it.strip {
                scan(s);
            }
            for i in &it.items {
                scan(&i.heading);
                scan(&i.text);
                extras_scan(&i.extras, scan);
            }
        }
    }
}
