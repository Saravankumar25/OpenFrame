//! Cards: location cards, breakdown scene cards, department notes, storyboard panels.
//!
//! A card is a stack of "units" (header, then parts). A row of cards is placed whole on a page;
//! a card taller than a page is split between units and its border repeats on each page.

use super::*;

/// A small tile in a card header: a number or ordinal, optionally under a tiny caption.
#[derive(Debug, Clone)]
pub struct Tile {
    pub top: Option<String>,
    pub text: String,
    pub fill: Rgb,
    pub line: Option<Rgb>,
    pub color: Rgb,
    pub w: f32,
}

impl Tile {
    pub fn new(text: impl Into<String>, fill: Rgb, color: Rgb) -> Tile {
        Tile {
            top: None,
            text: text.into(),
            fill,
            line: None,
            color,
            w: 27.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CardHead {
    pub tile: Option<Tile>,
    pub eyebrow: Option<String>,
    pub title: String,
    pub right: Option<Chip>,
    pub fill: Rgb,
    pub color: Rgb,
}

impl CardHead {
    pub fn new(title: impl Into<String>) -> CardHead {
        CardHead {
            tile: None,
            eyebrow: None,
            title: title.into(),
            right: None,
            fill: pal::PANEL,
            color: pal::NAVY,
        }
    }
    /// Navy header with white text (department notes).
    pub fn dark(title: impl Into<String>) -> CardHead {
        CardHead {
            fill: pal::NAVY,
            color: pal::WHITE,
            ..CardHead::new(title)
        }
    }
}

#[derive(Debug, Clone)]
pub struct Field {
    pub label: String,
    pub value: String,
    /// The value is missing: print a "needs input" chip instead.
    pub missing: bool,
}

impl Field {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Field {
        Field {
            label: label.into(),
            value: value.into(),
            missing: false,
        }
    }
    pub fn missing(label: impl Into<String>, text: impl Into<String>) -> Field {
        Field {
            label: label.into(),
            value: text.into(),
            missing: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GroupItem {
    pub text: String,
    /// Small status label after the pill ("SUGGESTED", "MANUAL").
    pub tag: Option<Chip>,
    /// A note printed under the pill.
    pub note: Option<String>,
    pub tone: Tone,
}

impl GroupItem {
    pub fn new(text: impl Into<String>) -> GroupItem {
        GroupItem {
            text: text.into(),
            tag: None,
            note: None,
            tone: Tone::Neutral,
        }
    }
}

/// A labelled set of pills with an accent bar (a breakdown category).
#[derive(Debug, Clone)]
pub struct Group {
    pub label: String,
    pub accent: Rgb,
    pub items: Vec<GroupItem>,
}

#[derive(Debug, Clone)]
pub enum CardPart {
    /// A bold lead line (a street address).
    Lead(Txt),
    /// Label and value rows.
    Fields(Vec<Field>),
    /// Groups of pills in `cols` columns.
    Groups {
        groups: Vec<Group>,
        cols: usize,
    },
    /// A label (blue, small caps) with lines of text beside it.
    Rows(Vec<(String, Vec<String>)>),
    Text(Txt),
    /// A picture frame with the picture fitted inside (or a placeholder note).
    Image {
        img: Option<ReportImage>,
        placeholder: Option<String>,
        ratio: f32,
    },
    /// Centred lines of text.
    Centered(Vec<Txt>),
    /// A quiet line such as "No breakdown elements yet".
    Empty(String),
    /// A small label over a value; a missing value prints the "needs input" chip instead.
    Labeled {
        label: String,
        value: String,
        missing: bool,
    },
    /// A picture frame like `Image`, with a muted placeholder text (4.5 to 1 on the frame) and an
    /// 8 pt inset: the moodboard.
    Picture {
        img: Option<ReportImage>,
        placeholder: Option<String>,
        ratio: f32,
    },
    /// One chip centred in the card (the "Internal" label under a picture).
    CenteredChip(Chip),
    /// Lines of text, each with a small accent bullet and a hairline between them.
    Lines(Vec<String>),
}

#[derive(Debug, Clone)]
pub struct Card {
    pub head: Option<CardHead>,
    pub parts: Vec<CardPart>,
    pub fill: Rgb,
    pub border: Rgb,
}

impl Card {
    pub fn new() -> Card {
        Card {
            head: None,
            parts: Vec::new(),
            fill: pal::WHITE,
            border: pal::LINE,
        }
    }
}

impl Default for Card {
    fn default() -> Self {
        Card::new()
    }
}

/// Cards laid out in `columns` columns, rows kept whole.
#[derive(Debug, Clone)]
pub struct Cards {
    pub columns: usize,
    pub gap: f32,
    pub cards: Vec<Card>,
}

impl Cards {
    pub fn new(columns: usize, cards: Vec<Card>) -> Cards {
        Cards {
            columns: columns.max(1),
            gap: 10.0,
            cards,
        }
    }
}

const PAD: f32 = 10.0;

fn card_head(h: &CardHead, w: f32) -> Frag {
    let mut f = Frag::new();
    let tile_w = h.tile.as_ref().map(|t| t.w).unwrap_or(0.0);
    let tile_h = h
        .tile
        .as_ref()
        .map(|t| if t.top.is_some() { 34.0 } else { 26.0 })
        .unwrap_or(0.0);
    let tx = PAD + if tile_w > 0.0 { tile_w + 9.0 } else { 0.0 };
    let right_w = h.right.as_ref().map(|c| c.width()).unwrap_or(0.0);
    let avail = (w - tx - PAD - if right_w > 0.0 { right_w + 8.0 } else { 0.0 }).max(40.0);
    let mut text = Frag::new();
    let mut th = 0.0f32;
    if let Some(e) = &h.eyebrow {
        th += put_txt(
            &mut text,
            0.0,
            th,
            avail,
            &Txt::label(e.clone()).size(6.6).after(1.0),
        );
    }
    let title = Txt::new(h.title.clone())
        .size(10.0)
        .bold()
        .color(h.color)
        .leading(1.2);
    th += put_txt(&mut text, 0.0, th, avail, &title);
    let inner = th.max(tile_h);
    let total = inner + 18.0;
    f.shape_c(0.0, 0.0, w, total, 6.0, true, false, Some(h.fill), None);
    if h.fill == pal::PANEL {
        f.line(0.0, total, w, total, pal::LINE, 0.8);
    }
    if let Some(t) = &h.tile {
        let ty = 9.0 + (inner - tile_h) / 2.0;
        f.shape(
            PAD,
            ty,
            t.w,
            tile_h,
            5.0,
            Some(t.fill),
            t.line.map(|l| (l, 0.9)),
        );
        match &t.top {
            Some(top) => {
                let tw1 = tw(top, 6.4, true, 0.3);
                f.text(
                    PAD + (t.w - tw1) / 2.0,
                    ty + 9.0,
                    top,
                    6.4,
                    true,
                    t.color,
                    0.3,
                );
                let nw = tw(&t.text, 15.0, false, 0.0);
                f.text(
                    PAD + (t.w - nw) / 2.0,
                    ty + 27.0,
                    &t.text,
                    15.0,
                    false,
                    t.color,
                    0.0,
                );
            }
            None => {
                let nw = tw(&t.text, 10.0, true, 0.0);
                f.text(
                    PAD + (t.w - nw) / 2.0,
                    ty + 17.0,
                    &t.text,
                    10.0,
                    true,
                    t.color,
                    0.0,
                );
            }
        }
    }
    f.put(text, tx, 9.0 + (inner - th) / 2.0);
    if let Some(c) = &h.right {
        let cy = (total - c.height()) / 2.0;
        put_chip(&mut f, w - PAD - right_w, cy, c);
    }
    f.h = total;
    f
}

fn field_rows(fields: &[Field], w: f32) -> Frag {
    let mut f = Frag::new();
    let iw = w - 2.0 * PAD;
    let label_w = iw * 0.40;
    let mut y = 4.0f32;
    for (i, fl) in fields.iter().enumerate() {
        let label = Txt::new(fl.label.clone()).size(8.2).color(pal::SLATE);
        let lh = txt_height(&label, label_w - 4.0);
        let top = y;
        let row_h;
        if fl.missing {
            let mut chip = Chip::label(fl.value.clone(), Tone::Danger);
            chip.size = 6.6;
            let ch = chip.height();
            row_h = lh.max(ch) + 7.0;
            put_txt(&mut f, PAD, top + 3.5, label_w - 4.0, &label);
            put_chip(
                &mut f,
                w - PAD - chip.width(),
                top + (row_h - ch) / 2.0,
                &chip,
            );
        } else {
            let val = Txt::new(fl.value.clone())
                .size(8.6)
                .bold()
                .color(pal::INK)
                .align(Align::Right);
            let vw = iw - label_w;
            let stacked = fl.value.contains('\n') || tw(&fl.value, 8.6, true, 0.0) > vw * 0.95;
            if stacked {
                let lh2 = put_txt(&mut f, PAD, top + 3.5, iw, &label);
                let val_left = Txt {
                    align: Align::Left,
                    ..val
                };
                let vh = put_txt(&mut f, PAD, top + 3.5 + lh2, iw, &val_left);
                row_h = lh2 + vh + 7.0;
            } else {
                let vh = put_txt(&mut f, PAD + label_w, top + 3.5, vw, &val);
                put_txt(&mut f, PAD, top + 3.5, label_w - 4.0, &label);
                row_h = lh.max(vh) + 7.0;
            }
        }
        y += row_h;
        if i + 1 < fields.len() {
            f.line(PAD, y, w - PAD, y, pal::LINE, 0.5);
        }
    }
    f.h = y + 4.0;
    f
}

fn group_frag(g: &Group, gw: f32) -> Frag {
    let mut f = Frag::new();
    f.circle(9.5, 6.0, 2.8, Some(g.accent), None);
    f.text(
        16.0,
        8.4,
        &g.label.to_uppercase(),
        7.0,
        true,
        pal::NAVY,
        0.7,
    );
    let mut y = 15.0f32;
    let width = gw - 10.0;
    let mut pending: Vec<Chip> = Vec::new();
    let flush = |f: &mut Frag, pending: &mut Vec<Chip>, y: &mut f32| {
        if !pending.is_empty() {
            let h = put_chips(f, 8.0, *y, width, pending, Align::Left);
            *y += h + 4.0;
            pending.clear();
        }
    };
    for it in &g.items {
        pending.push(Chip::pill(it.text.clone(), it.tone));
        if let Some(t) = &it.tag {
            let mut t = t.clone();
            t.size = 6.4;
            pending.push(t);
        }
        if let Some(n) = &it.note {
            flush(&mut f, &mut pending, &mut y);
            let t = Txt::new(n.clone())
                .size(7.6)
                .color(pal::SLATE)
                .leading(1.25);
            y += put_txt(&mut f, 12.0, y - 1.0, width - 4.0, &t) + 3.0;
        }
    }
    flush(&mut f, &mut pending, &mut y);
    f.h = y;
    f.shape(0.0, 0.0, 2.2, y.max(14.0) - 3.0, 0.0, Some(g.accent), None);
    f
}

fn groups_frag(groups: &[Group], cols: usize, w: f32) -> Frag {
    let mut f = Frag::new();
    let cols = cols.clamp(1, 4);
    let gap = 14.0;
    let iw = w - 2.0 * PAD;
    let gw = (iw - gap * (cols as f32 - 1.0)) / cols as f32;
    let mut y = 8.0f32;
    for row in groups.chunks(cols) {
        let frags: Vec<Frag> = row.iter().map(|g| group_frag(g, gw)).collect();
        let rh = frags.iter().map(|g| g.h).fold(0.0, f32::max);
        for (i, mut g) in frags.into_iter().enumerate() {
            // Equal bar height across a row.
            for op in g.ops.iter_mut() {
                if let Op::Shape {
                    x, w: sw, y: sy, h, ..
                } = op
                    && *x == 0.0
                    && *sw == 2.2
                {
                    *h = rh.max(14.0) - 3.0;
                    *sy = -(rh.max(14.0) - 3.0);
                }
            }
            f.put(g, PAD + i as f32 * (gw + gap), y);
        }
        y += rh + 8.0;
    }
    f.h = y;
    f
}

fn rows_frag(rows: &[(String, Vec<String>)], w: f32) -> Frag {
    let mut f = Frag::new();
    let label_w = 64.0f32.min(w * 0.3);
    let iw = w - 2.0 * PAD;
    let mut y = 6.0f32;
    for (i, (label, lines)) in rows.iter().enumerate() {
        let lt = Txt::new(label.to_uppercase())
            .size(6.8)
            .bold()
            .track(0.5)
            .color(pal::TEAL);
        put_txt(&mut f, PAD, y + 3.0, label_w, &lt);
        let mut h = 0.0f32;
        for l in lines {
            h += put_txt(
                &mut f,
                PAD + label_w,
                y + 2.0 + h,
                iw - label_w,
                &Txt::new(l.clone()).size(8.6).color(pal::INK).leading(1.3),
            );
        }
        y += h.max(11.0) + 8.0;
        if i + 1 < rows.len() {
            f.line(PAD, y - 3.5, w - PAD, y - 3.5, pal::LINE, 0.5);
        }
    }
    f.h = y;
    f
}

/// A picture frame with the picture fitted inside (never cropped), or a placeholder note.
fn picture_frag<'a>(
    p: &mut Pager<'a>,
    img: &'a Option<ReportImage>,
    placeholder: &Option<String>,
    ratio: f32,
    w: f32,
    pad: f32,
    text: Rgb,
) -> Frag {
    let mut f = Frag::new();
    let fw = w - 2.0 * pad;
    let fh = fw * ratio.clamp(0.2, 2.0);
    f.shape(
        pad,
        8.0,
        fw,
        fh,
        2.0,
        Some(Rgb::hex(0xeef1f5)),
        Some((pal::LINE, 0.6)),
    );
    match img {
        Some(img) => {
            let scale = (fw / img.width as f32).min(fh / img.height as f32);
            let (iw, ih) = (img.width as f32 * scale, img.height as f32 * scale);
            let index = p.image_index(img);
            f.ops.push(Op::Image {
                index,
                x: pad + (fw - iw) / 2.0,
                y: -(8.0 + (fh + ih) / 2.0),
                w: iw,
                h: ih,
            });
        }
        None => {
            if let Some(label) = placeholder {
                let t = Txt::new(label.clone())
                    .size(8.5)
                    .color(text)
                    .align(Align::Center);
                let th = txt_height(&t, fw - 16.0);
                put_txt(&mut f, pad + 8.0, 8.0 + (fh - th) / 2.0, fw - 16.0, &t);
            }
        }
    }
    f.h = fh + 14.0;
    f
}

fn units<'a>(p: &mut Pager<'a>, card: &'a Card, w: f32) -> Vec<Frag> {
    let mut out = Vec::new();
    if let Some(h) = &card.head {
        out.push(card_head(h, w));
    }
    for part in &card.parts {
        let mut f = Frag::new();
        match part {
            CardPart::Lead(t) => {
                let h = put_txt(&mut f, PAD, 7.0, w - 2.0 * PAD, t);
                f.h = h + 9.0;
            }
            CardPart::Fields(fields) => f = field_rows(fields, w),
            CardPart::Groups { groups, cols } => f = groups_frag(groups, *cols, w),
            CardPart::Rows(rows) => f = rows_frag(rows, w),
            CardPart::Text(t) => {
                let h = put_txt(&mut f, PAD, 6.0, w - 2.0 * PAD, t);
                f.h = h + 9.0;
            }
            CardPart::Empty(text) => {
                let t = Txt::new(text.clone()).size(8.2).color(pal::FAINT);
                let h = put_txt(&mut f, PAD, 9.0, w - 2.0 * PAD, &t);
                f.h = h + 15.0;
            }
            CardPart::Labeled {
                label,
                value,
                missing,
            } => {
                let iw = w - 2.0 * PAD;
                let mut y = 9.0;
                // A long location name makes a long label: it wraps inside the card.
                let tag = Txt::label(label.clone());
                let lh = tag.size * tag.leading;
                let lines = tracked_lines(&tag, iw);
                for (k, l) in lines.iter().enumerate() {
                    f.text(
                        PAD,
                        y + tag.size + k as f32 * lh,
                        l,
                        tag.size,
                        tag.bold,
                        tag.color,
                        tag.track,
                    );
                }
                y += lines.len() as f32 * lh + 5.0;
                if *missing {
                    let mut chip = Chip::label(value.clone(), Tone::Danger);
                    chip.size = 6.8;
                    let cw1 = iw.min(chip.width().max(iw));
                    let h = chip.height() + 3.0;
                    f.shape(
                        PAD,
                        y,
                        cw1,
                        h,
                        4.0,
                        Some(Tone::Danger.fill()),
                        Some((Tone::Danger.line(), 0.8)),
                    );
                    f.text(
                        PAD + 8.0,
                        y + (h + 0.72 * chip.size) / 2.0,
                        &chip.text,
                        chip.size,
                        true,
                        Tone::Danger.text(),
                        chip.track,
                    );
                    y += h;
                } else {
                    y += put_txt(
                        &mut f,
                        PAD,
                        y,
                        iw,
                        &Txt::new(value.clone())
                            .size(9.6)
                            .bold()
                            .color(pal::INK)
                            .leading(1.3),
                    );
                }
                f.h = y + 11.0;
            }
            CardPart::Lines(lines) => {
                let iw = w - 2.0 * PAD - 12.0;
                let mut y = 6.0;
                for (i, l) in lines.iter().enumerate() {
                    let t = Txt::new(l.clone()).size(8.8).color(pal::INK).leading(1.3);
                    let h = txt_height(&t, iw);
                    f.circle(PAD + 2.5, y + 7.2, 1.7, Some(pal::ORANGE), None);
                    put_txt(&mut f, PAD + 12.0, y + 2.0, iw, &t);
                    y += h + 6.0;
                    if i + 1 < lines.len() {
                        f.line(PAD, y - 2.5, w - PAD, y - 2.5, pal::LINE, 0.5);
                    }
                }
                f.h = y + 3.0;
            }
            CardPart::Centered(lines) => {
                let mut y = 3.0;
                for t in lines {
                    y += put_txt(&mut f, PAD, y, w - 2.0 * PAD, t);
                }
                f.h = y + 7.0;
            }
            CardPart::Image {
                img,
                placeholder,
                ratio,
            } => f = picture_frag(p, img, placeholder, *ratio, w, PAD, pal::FAINT),
            CardPart::Picture {
                img,
                placeholder,
                ratio,
            } => f = picture_frag(p, img, placeholder, *ratio, w, 8.0, Rgb::hex(0x66728a)),
            CardPart::CenteredChip(chip) => {
                put_chip(&mut f, (w - chip.width()) / 2.0, 3.0, chip);
                f.h = chip.height() + 11.0;
            }
        }
        out.push(f);
    }
    out
}

pub(in crate::report) fn cards<'a>(p: &mut Pager<'a>, c: &'a Cards) {
    let cw = p.content_w();
    let cols = c.columns.max(1);
    let w = (cw - c.gap * (cols as f32 - 1.0)) / cols as f32;
    for chunk in c.cards.chunks(cols) {
        let all: Vec<Vec<Frag>> = chunk.iter().map(|card| units(p, card, w)).collect();
        let heights: Vec<f32> = all.iter().map(|u| u.iter().map(|f| f.h).sum()).collect();
        let row_h = heights.iter().cloned().fold(0.0, f32::max);
        if row_h > p.body_h() {
            for (card, us) in chunk.iter().zip(all) {
                split_card(p, card, us, w);
            }
            continue;
        }
        p.ensure(row_h + 2.0);
        let top = p.y;
        for ((card, us), i) in chunk.iter().zip(all).zip(0..) {
            let x = MARGIN + i as f32 * (w + c.gap);
            let mut f = Frag::new();
            f.shape(0.0, 0.0, w, row_h, 6.0, Some(card.fill), None);
            let mut y = 0.0;
            for u in us {
                let h = u.h;
                f.put(u, 0.0, y);
                y += h;
            }
            f.shape(0.0, 0.0, w, row_h, 6.0, None, Some((card.border, 0.9)));
            p.emit(f, x, top);
        }
        p.y -= row_h + c.gap;
    }
}

/// A card taller than a page: units flow onto the next pages, each page with its own border.
fn split_card(p: &mut Pager, card: &Card, us: Vec<Frag>, w: f32) {
    let mut seg_top = p.y;
    let mut seg: Vec<Frag> = Vec::new();
    let close = |p: &mut Pager, seg: &mut Vec<Frag>, seg_top: f32| {
        let h: f32 = seg.iter().map(|f| f.h).sum();
        if h <= 0.0 {
            return;
        }
        let mut f = Frag::new();
        let mut y = 0.0;
        for u in seg.drain(..) {
            let uh = u.h;
            f.put(u, 0.0, y);
            y += uh;
        }
        f.shape(0.0, 0.0, w, h, 6.0, None, Some((card.border, 0.9)));
        p.emit(f, MARGIN, seg_top);
        p.y = seg_top - h;
    };
    for u in us {
        if p.room() - seg.iter().map(|f| f.h).sum::<f32>() < u.h {
            close(p, &mut seg, seg_top);
            p.new_page();
            seg_top = p.y;
        }
        seg.push(u);
    }
    close(p, &mut seg, seg_top);
    p.y -= 10.0;
}

pub(in crate::report) fn cards_scan(c: &Cards, scan: &mut dyn FnMut(&str)) {
    for card in &c.cards {
        if let Some(h) = &card.head {
            scan(&h.title);
            if let Some(e) = &h.eyebrow {
                scan(e);
            }
            if let Some(t) = &h.tile {
                scan(&t.text);
                if let Some(top) = &t.top {
                    scan(top);
                }
            }
            if let Some(r) = &h.right {
                scan(&r.text);
            }
        }
        for part in &card.parts {
            match part {
                CardPart::Lead(t) | CardPart::Text(t) => scan(&t.text),
                CardPart::Fields(fs) => fs.iter().for_each(|f| {
                    scan(&f.label);
                    scan(&f.value);
                }),
                CardPart::Groups { groups, .. } => groups.iter().for_each(|g| {
                    scan(&g.label);
                    for it in &g.items {
                        scan(&it.text);
                        if let Some(t) = &it.tag {
                            scan(&t.text);
                        }
                        if let Some(n) = &it.note {
                            scan(n);
                        }
                    }
                }),
                CardPart::Rows(rows) => rows.iter().for_each(|(l, ls)| {
                    scan(l);
                    ls.iter().for_each(|x| scan(x));
                }),
                CardPart::Centered(ls) => ls.iter().for_each(|t| scan(&t.text)),
                CardPart::Empty(t) => scan(t),
                CardPart::CenteredChip(c) => scan(&c.text),
                CardPart::Labeled { label, value, .. } => {
                    scan(label);
                    scan(value);
                }
                CardPart::Lines(ls) => ls.iter().for_each(|x| scan(x)),
                CardPart::Image { placeholder, .. } | CardPart::Picture { placeholder, .. } => {
                    if let Some(t) = placeholder {
                        scan(t);
                    }
                }
            }
        }
    }
}
