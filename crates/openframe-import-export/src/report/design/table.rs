//! Styled tables: stripboard rows, status matrices, shot lists, catalog sections.
//!
//! One explicit column grid is shared by the header and every row, so headings align exactly
//! over their columns. Rows are never split and never truncated: a row grows to fit its text
//! and moves whole to the next page, where the header repeats.

use super::*;

#[derive(Debug, Clone, Copy)]
pub enum Width {
    /// Fixed width in points.
    Pt(f32),
    /// Share of what the fixed columns leave over.
    Weight(f32),
}

#[derive(Debug, Clone)]
pub struct Col {
    pub header: String,
    pub width: Width,
    pub align: Align,
}

impl Col {
    pub fn pt(header: impl Into<String>, w: f32) -> Col {
        Col {
            header: header.into(),
            width: Width::Pt(w),
            align: Align::Left,
        }
    }
    pub fn weight(header: impl Into<String>, w: f32) -> Col {
        Col {
            header: header.into(),
            width: Width::Weight(w),
            align: Align::Left,
        }
    }
    pub fn align(mut self, a: Align) -> Col {
        self.align = a;
        self
    }
}

/// One thing inside a table cell. Items stack top to bottom.
#[derive(Debug, Clone)]
pub enum CellItem {
    Text(Txt),
    Chips(Vec<Chip>),
    /// A rounded tile with a bold number (scene numbers, cast ids).
    Tile(String),
    /// The "needs input" state of a missing value.
    Missing(String),
    Image {
        img: ReportImage,
        w: f32,
        h: f32,
    },
    /// An empty circle to tick by hand.
    Check,
    /// An empty picture frame with a short note ("No image").
    Placeholder {
        w: f32,
        h: f32,
        label: String,
    },
    /// A thin proportion bar: `share` of the width (0 to 1) filled with `color`.
    Bar {
        share: f32,
        color: Rgb,
    },
    Gap(f32),
    /// Paragraphs of mixed bold and regular runs ("Sound: wind"), each wrapped as one paragraph.
    Rich(Rich),
    /// A band heading: coloured dot, bold text, optional right text.
    Heading {
        dot: Option<Rgb>,
        text: String,
        right: Option<String>,
    },
}

#[derive(Debug, Clone, Default)]
pub struct Cell {
    pub items: Vec<CellItem>,
    pub fill: Option<Rgb>,
    pub align: Option<Align>,
    /// Centre the content vertically.
    pub middle: bool,
}

impl Cell {
    pub fn new() -> Cell {
        Cell::default()
    }
    pub fn empty() -> Cell {
        Cell::default()
    }
    pub fn text(t: Txt) -> Cell {
        Cell {
            items: vec![CellItem::Text(t)],
            ..Cell::default()
        }
    }
    pub fn item(mut self, i: CellItem) -> Cell {
        self.items.push(i);
        self
    }
    pub fn fill(mut self, f: Rgb) -> Cell {
        self.fill = Some(f);
        self
    }
    pub fn align(mut self, a: Align) -> Cell {
        self.align = Some(a);
        self
    }
    pub fn middle(mut self) -> Cell {
        self.middle = true;
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct Row {
    pub cells: Vec<Cell>,
    pub fill: Option<Rgb>,
    /// A coloured bar on the left edge of the row.
    pub stripe: Option<Rgb>,
    /// The first cell spans every column.
    pub span: bool,
    /// Keep this row on the same page as the row after it (band headings, group header rows).
    pub keep_next: bool,
    /// Keep this row on the same page as the `n` rows before it (subtotals and totals travel with
    /// the last rows they sum up).
    pub keep_prev: u8,
}

impl Row {
    pub fn new(cells: Vec<Cell>) -> Row {
        Row {
            cells,
            ..Row::default()
        }
    }
    pub fn spanning(cell: Cell) -> Row {
        Row {
            cells: vec![cell],
            span: true,
            ..Row::default()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderStyle {
    Navy,
    Tint,
    Plain,
    None,
}

#[derive(Debug, Clone)]
pub struct StyledTable {
    /// The table's name, printed inside its repeating header so a page never shows the column
    /// labels without it; continuation pages add " · continued".
    pub title: Option<String>,
    /// Small text at the right end of the title row.
    pub title_hint: Option<String>,
    /// A group band printed in place of the title (its count on the first page, "Continued" on
    /// later ones). It repeats with the column header.
    pub band: Option<TableBand>,
    pub columns: Vec<Col>,
    pub header: HeaderStyle,
    /// Header cells with their own content (multi-line day headers); default is the column
    /// header texts.
    pub header_cells: Option<Vec<Cell>>,
    pub rows: Vec<Row>,
    pub zebra: bool,
    /// Vertical lines between columns.
    pub grid: bool,
    /// A rounded outline around the table (per page).
    pub boxed: bool,
    pub empty: Option<String>,
    /// Printed in a small bar when an untitled table continues on a new page (a titled table
    /// repeats its title with " · continued" instead).
    pub continued: Option<String>,
    pub pad_x: f32,
    pub pad_y: f32,
}

impl StyledTable {
    pub fn new(columns: Vec<Col>) -> StyledTable {
        StyledTable {
            title: None,
            title_hint: None,
            band: None,
            columns,
            header: HeaderStyle::Tint,
            header_cells: None,
            rows: Vec::new(),
            zebra: false,
            grid: false,
            boxed: false,
            empty: None,
            continued: None,
            pad_x: 7.0,
            pad_y: 6.0,
        }
    }
}

fn widths(t: &StyledTable, total: f32) -> Vec<f32> {
    let fixed: f32 = t
        .columns
        .iter()
        .map(|c| match c.width {
            Width::Pt(w) => w,
            _ => 0.0,
        })
        .sum();
    let weights: f32 = t
        .columns
        .iter()
        .map(|c| match c.width {
            Width::Weight(w) => w,
            _ => 0.0,
        })
        .sum::<f32>()
        .max(0.01);
    let rest = (total - fixed).max(0.0);
    t.columns
        .iter()
        .map(|c| match c.width {
            Width::Pt(w) => w,
            Width::Weight(w) => rest * w / weights,
        })
        .collect()
}

const ITEM_GAP: f32 = 3.0;

/// One cell's content, laid out in `w` points. The fragment height is the content height.
fn cell_content<'a>(p: &mut Pager<'a>, cell: &'a Cell, w: f32, default_align: Align) -> Frag {
    let align = cell.align.unwrap_or(default_align);
    let mut f = Frag::new();
    let mut y = 0.0f32;
    for (i, it) in cell.items.iter().enumerate() {
        if i > 0 && !matches!(it, CellItem::Gap(_)) {
            y += ITEM_GAP;
        }
        match it {
            CellItem::Text(t) => {
                let mut t = t.clone();
                if t.align == Align::Left {
                    t.align = align;
                }
                y += put_txt(&mut f, 0.0, y, w, &t);
            }
            CellItem::Chips(chips) => {
                y += put_chips(&mut f, 0.0, y, w, chips, align) + 1.0;
            }
            CellItem::Tile(text) => {
                let tw1 = tw(text, 9.0, true, 0.0);
                let tile_w = (tw1 + 14.0).max(28.0).min(w);
                let x = match align {
                    Align::Left => 0.0,
                    Align::Center => (w - tile_w) / 2.0,
                    Align::Right => w - tile_w,
                };
                f.shape(x, y, tile_w, 17.0, 4.0, Some(Rgb::hex(0xe8edf3)), None);
                f.text(
                    x + (tile_w - tw1) / 2.0,
                    y + 12.2,
                    text,
                    9.0,
                    true,
                    pal::NAVY,
                    0.0,
                );
                y += 17.0;
            }
            CellItem::Missing(text) => {
                let mut chip = Chip::label(text.clone(), Tone::Danger);
                chip.size = 6.6;
                let cw1 = w.min(chip.width().max(96.0));
                let h = chip.height() + 2.0;
                let x = match align {
                    Align::Left => 0.0,
                    Align::Center => (w - cw1) / 2.0,
                    Align::Right => w - cw1,
                };
                f.shape(
                    x,
                    y,
                    cw1,
                    h,
                    4.0,
                    Some(Tone::Danger.fill()),
                    Some((Tone::Danger.line(), 0.8)),
                );
                f.text(
                    x + 7.0,
                    y + (h + 0.72 * chip.size) / 2.0,
                    &chip.text,
                    chip.size,
                    true,
                    Tone::Danger.text(),
                    chip.track,
                );
                y += h;
            }
            CellItem::Image { img, w: iw, h: ih } => {
                let fw = iw.min(w);
                let x = match align {
                    Align::Left => 0.0,
                    Align::Center => (w - fw) / 2.0,
                    Align::Right => w - fw,
                };
                f.shape(
                    x,
                    y,
                    fw,
                    *ih,
                    2.0,
                    Some(Rgb::hex(0xeef1f5)),
                    Some((pal::LINE, 0.6)),
                );
                let scale = (fw / img.width as f32).min(ih / img.height as f32);
                let (dw, dh) = (img.width as f32 * scale, img.height as f32 * scale);
                let index = p.image_index(img);
                f.ops.push(Op::Image {
                    index,
                    x: x + (fw - dw) / 2.0,
                    y: -(y + (ih + dh) / 2.0),
                    w: dw,
                    h: dh,
                });
                y += ih;
            }
            CellItem::Check => {
                let x = match align {
                    Align::Left => 6.5,
                    Align::Center => w / 2.0,
                    Align::Right => w - 6.5,
                };
                f.circle(x, y + 7.0, 6.0, None, Some((pal::SLATE, 0.9)));
                y += 14.0;
            }
            CellItem::Placeholder {
                w: pw,
                h: ph,
                label,
            } => {
                let fw = pw.min(w);
                let x = match align {
                    Align::Left => 0.0,
                    Align::Center => (w - fw) / 2.0,
                    Align::Right => w - fw,
                };
                // The label wraps inside the frame; the frame grows in height, never in width.
                let lines = wrap_pt(label, (fw - 8.0).max(8.0), 7.0, false);
                let lh = 7.0 * 1.25;
                let ph = ph.max(lines.len() as f32 * lh + 10.0);
                f.shape(
                    x,
                    y,
                    fw,
                    ph,
                    2.0,
                    Some(Rgb::hex(0xf1f3f6)),
                    Some((pal::LINE, 0.6)),
                );
                let top = y + (ph - lines.len() as f32 * lh) / 2.0;
                for (k, l) in lines.iter().enumerate() {
                    let lw = tw(l, 7.0, false, 0.0);
                    f.text(
                        x + (fw - lw) / 2.0,
                        top + 7.0 + k as f32 * lh - 0.8,
                        l,
                        7.0,
                        false,
                        Rgb::hex(0x66728a),
                        0.0,
                    );
                }
                y += ph;
            }
            CellItem::Bar { share, color } => {
                f.shape(0.0, y + 1.0, w, 4.0, 2.0, Some(pal::LINE), None);
                let fill = (w * share.clamp(0.0, 1.0)).max(if *share > 0.0 { 2.0 } else { 0.0 });
                if fill > 0.0 {
                    f.shape(0.0, y + 1.0, fill, 4.0, 2.0, Some(*color), None);
                }
                y += 6.0;
            }
            CellItem::Gap(g) => y += g,
            CellItem::Rich(r) => y += put_rich(&mut f, 0.0, y, w, r),
            CellItem::Heading { dot, text, right } => {
                let mut tx = 0.0;
                if let Some(d) = dot {
                    f.circle(4.0, y + 7.5, 3.4, Some(*d), None);
                    tx = 13.0;
                }
                let right_w = right
                    .as_ref()
                    .map(|r| tw(r, 8.0, false, 0.0))
                    .unwrap_or(0.0);
                f.text(tx, y + 11.0, text, 11.0, true, pal::NAVY, 0.0);
                if let Some(r) = right {
                    f.text(w - right_w, y + 10.5, r, 8.0, false, pal::SLATE, 0.0);
                }
                y += 16.0;
            }
        }
    }
    f.h = y;
    f
}

struct RowFrag {
    f: Frag,
    h: f32,
}

#[allow(clippy::too_many_arguments)]
fn row_frag<'a>(
    p: &mut Pager<'a>,
    t: &StyledTable,
    row: &'a Row,
    widths: &[f32],
    xs: &[f32],
    total_w: f32,
    index: usize,
    min_h: f32,
) -> RowFrag {
    let mut contents: Vec<(f32, f32, Frag, &Cell)> = Vec::new();
    let mut h = min_h;
    if row.span {
        if let Some(cell) = row.cells.first() {
            let w = total_w - 2.0 * t.pad_x;
            let c = cell_content(p, cell, w, Align::Left);
            h = h.max(c.h + 2.0 * t.pad_y);
            contents.push((0.0, total_w, c, cell));
        }
    } else {
        for (i, w) in widths.iter().enumerate() {
            let Some(cell) = row.cells.get(i) else {
                continue;
            };
            let c = cell_content(p, cell, w - 2.0 * t.pad_x, t.columns[i].align);
            h = h.max(c.h + 2.0 * t.pad_y);
            contents.push((xs[i], *w, c, cell));
        }
    }
    let mut f = Frag::new();
    let zebra = t.zebra && index % 2 == 1;
    if let Some(fill) = row.fill.or(if zebra {
        Some(Rgb::hex(0xf8fafc))
    } else {
        None
    }) {
        f.shape(0.0, 0.0, total_w, h, 0.0, Some(fill), None);
    }
    for (x, w, _, cell) in &contents {
        if let Some(fill) = cell.fill {
            f.shape(*x, 0.0, *w, h, 0.0, Some(fill), None);
        }
    }
    if let Some(s) = row.stripe {
        f.shape(0.0, 0.0, 3.2, h, 0.0, Some(s), None);
    }
    for (x, w, c, cell) in contents {
        let off = if cell.middle {
            ((h - c.h) / 2.0).max(t.pad_y)
        } else {
            t.pad_y
        };
        f.put(c, x + t.pad_x, off);
        let _ = w;
    }
    if t.grid && !row.span {
        for x in xs.iter().skip(1) {
            f.line(*x, 0.0, *x, h, pal::LINE, 0.6);
        }
    }
    f.line(0.0, h, total_w, h, pal::LINE, 0.6);
    f.h = h;
    RowFrag { f, h }
}

fn header_frag<'a>(
    p: &mut Pager<'a>,
    t: &'a StyledTable,
    widths: &[f32],
    xs: &[f32],
    total_w: f32,
) -> Option<Frag> {
    if t.header == HeaderStyle::None {
        return None;
    }
    let (fill, color) = match t.header {
        HeaderStyle::Navy => (Some(pal::NAVY), pal::WHITE),
        HeaderStyle::Tint => (Some(pal::TINT), pal::SLATE),
        _ => (None, pal::SLATE),
    };
    let mut cells: Vec<Cell> = Vec::new();
    let owned: &[Cell] = match &t.header_cells {
        Some(c) => c,
        None => {
            for col in &t.columns {
                cells.push(Cell::text(
                    Txt::new(col.header.to_uppercase())
                        .size(7.0)
                        .bold()
                        .track(0.8)
                        .color(color),
                ));
            }
            &[]
        }
    };
    // The generated header cells live only for this call, so they are laid out here directly.
    let mut f = Frag::new();
    let mut h = 0.0f32;
    let mut parts: Vec<(f32, Frag)> = Vec::new();
    let sources: Vec<&Cell> = if owned.is_empty() {
        cells.iter().collect()
    } else {
        owned.iter().collect()
    };
    for (i, cell) in sources.iter().enumerate() {
        let Some(w) = widths.get(i) else { break };
        // Header cells never hold images, so a short lived clone-free layout is enough.
        let c = header_cell_content(cell, w - 2.0 * t.pad_x, t.columns[i].align);
        h = h.max(c.h + 14.0);
        parts.push((xs[i], c));
    }
    if let Some(fill) = fill {
        f.shape(0.0, 0.0, total_w, h, 0.0, Some(fill), None);
    }
    for (i, (x, c)) in parts.into_iter().enumerate() {
        if let Some(cf) = sources.get(i).and_then(|c| c.fill) {
            f.shape(x - t.pad_x * 0.0, 0.0, widths[i], h, 0.0, Some(cf), None);
        }
        let off = (h - c.h) / 2.0;
        f.put(c, x + t.pad_x, off);
    }
    if t.header == HeaderStyle::Plain {
        f.line(0.0, h, total_w, h, pal::LINE, 1.0);
    }
    if t.grid {
        for x in xs.iter().skip(1) {
            f.line(*x, 0.0, *x, h, pal::LINE, 0.6);
        }
    }
    f.h = h;
    let _ = p;
    Some(f)
}

/// Header cell layout: text and chips only.
fn header_cell_content(cell: &Cell, w: f32, default_align: Align) -> Frag {
    let align = cell.align.unwrap_or(default_align);
    let mut f = Frag::new();
    let mut y = 0.0f32;
    for (i, it) in cell.items.iter().enumerate() {
        if i > 0 {
            y += 1.5;
        }
        match it {
            CellItem::Text(t) => {
                let mut t = t.clone();
                if t.align == Align::Left {
                    t.align = align;
                }
                y += put_txt(&mut f, 0.0, y, w, &t);
            }
            CellItem::Chips(c) => y += put_chips(&mut f, 0.0, y, w, c, align),
            CellItem::Gap(g) => y += g,
            _ => {}
        }
    }
    f.h = y;
    f
}

/// The title row of a table's repeating header: its name in section style, a navy rule under it,
/// and " · continued" after the name on continuation pages. `None` for an untitled table.
fn title_frag(t: &StyledTable, total_w: f32, continued: bool) -> Option<Frag> {
    const SIZE: f32 = 10.0;
    const TRACK: f32 = 1.0;
    const LEAD: f32 = SIZE * 1.2;
    const SUFFIX: &str = " · continued";
    let title = t.title.as_ref()?;
    let hint_w = t
        .title_hint
        .as_ref()
        .map(|h| tw(h, 7.4, false, 0.0).min(total_w * 0.4))
        .unwrap_or(0.0);
    let avail = (total_w - hint_w - 12.0).max(60.0);
    let name = Txt::new(title.to_uppercase())
        .size(SIZE)
        .bold()
        .track(TRACK);
    let lines = txt_lines(&name, avail);
    let mut f = Frag::new();
    for (k, l) in lines.iter().enumerate() {
        f.text(0.0, SIZE + k as f32 * LEAD, l, SIZE, true, pal::NAVY, TRACK);
    }
    let mut h = lines.len() as f32 * LEAD;
    if continued {
        let last = lines.last().map(String::as_str).unwrap_or("");
        let lw = tw(last, SIZE, true, TRACK);
        let sw = tw(SUFFIX, 7.4, false, 0.0);
        let base = (lines.len() as f32 - 1.0) * LEAD + SIZE;
        if lw + sw <= avail {
            f.text(lw, base, SUFFIX, 7.4, false, pal::SLATE, 0.0);
        } else {
            f.text(0.0, base + 9.5, "· continued", 7.4, false, pal::SLATE, 0.0);
            h += 9.5;
        }
    }
    if let Some(hint) = &t.title_hint {
        let w = tw(hint, 7.4, false, 0.0).min(total_w * 0.4);
        f.text(total_w - w, SIZE, hint, 7.4, false, pal::SLATE, 0.0);
    }
    f.line(0.0, h + 3.0, total_w, h + 3.0, pal::NAVY, 1.2);
    f.h = h + 3.0 + 1.2 + 4.0;
    Some(f)
}

/// What leads the repeating header: the group band when there is one, else the title row.
fn lead_frag(t: &StyledTable, total_w: f32, continued: bool) -> Option<Frag> {
    match &t.band {
        Some(b) => Some(band_frag(b, total_w, continued)),
        None => title_frag(t, total_w, continued),
    }
}

/// Where the rows of a table start new pages.
///
/// `heights` are the row heights; `forbid[i]` means a page may not start at row `i` (the row
/// belongs with the one before it). `first_room` is the space for rows on the table's first page,
/// `full_room` on every later page; `fresh` says the table starts at the top of a page. The result
/// lists the rows that start a page. It holds 0 when the table must start on a new page because
/// its first rows (title, header and at least two rows) do not fit the space left, and it never
/// leaves a single row alone on the last page when moving the row before it avoids that.
fn plan_breaks(
    heights: &[f32],
    forbid: &[bool],
    first_room: f32,
    full_room: f32,
    fresh: bool,
) -> Vec<usize> {
    let n = heights.len();
    let mut breaks: Vec<usize> = Vec::new();
    let (mut start, mut room, mut used, mut i) = (0usize, first_room, 0.0f32, 0usize);
    let mut on_first = true;
    while i < n {
        if used + heights[i] <= room + 0.01 {
            used += heights[i];
            i += 1;
            continue;
        }
        // The row at `i` does not fit: break at the nearest earlier place that keeps groups whole.
        let mut b = i;
        while b > start && forbid[b] {
            b -= 1;
        }
        if b == start {
            if on_first && !fresh && start == 0 {
                // Title, header and the rows that must travel together do not fit: new page.
                breaks.push(0);
                on_first = false;
                (room, used, i) = (full_room, 0.0, 0);
                continue;
            }
            if i == start {
                // A row taller than a page: nothing better to do than place it.
                used += heights[i];
                i += 1;
                continue;
            }
            b = i;
        }
        breaks.push(b);
        (start, on_first, room, used, i) = (b, false, full_room, 0.0, b);
    }
    // No lone last row: let the row before it move along when the rules and the page allow.
    if let Some(&last) = breaks.last()
        && last > 0
        && n - last == 1
    {
        let starts = if breaks.first() == Some(&0) {
            breaks.len()
        } else {
            breaks.len() + 1
        };
        let prev = if breaks.len() >= 2 {
            breaks[breaks.len() - 2]
        } else {
            0
        };
        // The table's first page keeps at least two rows with the title; later pages one.
        let keep = if starts == 2 { 2 } else { 1 };
        if last > prev + keep && !forbid[last - 1] && heights[last - 1] + heights[last] <= full_room
        {
            let at = breaks.len() - 1;
            breaks[at] = last - 1;
        }
    }
    breaks
}

pub(in crate::report) fn styled_table<'a>(p: &mut Pager<'a>, t: &'a StyledTable) {
    let total_w = p.content_w();
    let ws = widths(t, total_w);
    let mut xs = Vec::new();
    let mut x = 0.0;
    for w in &ws {
        xs.push(x);
        x += w;
    }
    let header = header_frag(p, t, &ws, &xs, total_w);
    let header_h = header.as_ref().map(|h| h.h).unwrap_or(0.0);
    if t.rows.is_empty() {
        if let Some(tf) = lead_frag(t, total_w, false) {
            p.ensure(tf.h + 20.0);
            let top = p.y;
            let h = tf.h;
            p.emit(tf, MARGIN, top);
            p.y -= h;
        }
        if let Some(e) = &t.empty {
            text(
                p,
                &Txt::new(e.clone()).size(8.6).color(pal::FAINT).after(6.0),
            );
        }
        return;
    }
    let draw_box = |p: &mut Pager, seg_top: f32| {
        if t.boxed {
            let h = seg_top - p.y;
            let mut f = Frag::new();
            f.shape(0.0, 0.0, total_w, h, 0.0, None, Some((pal::LINE, 0.9)));
            p.emit(f, MARGIN, seg_top);
        }
    };
    // Rows are built once, in order; their heights decide where the pages break.
    let frags: Vec<RowFrag> = t
        .rows
        .iter()
        .enumerate()
        .map(|(i, row)| row_frag(p, t, row, &ws, &xs, total_w, i, 0.0))
        .collect();
    let n = frags.len();
    let heights: Vec<f32> = frags.iter().map(|f| f.h).collect();
    let mut forbid = vec![false; n + 1];
    if n >= 2 {
        // A title or heading always travels with at least two rows.
        forbid[1] = true;
    }
    for (j, row) in t.rows.iter().enumerate() {
        if row.keep_next && j + 1 < n {
            forbid[j + 1] = true;
        }
        if row.keep_prev > 0 {
            // A subtotal travels with the rows it sums up.
            let lo = (j + 1).saturating_sub(row.keep_prev as usize).max(1);
            forbid[lo..=j].fill(true);
        }
    }
    let title_h = |continued: bool| lead_frag(t, total_w, continued).map_or(0.0, |f| f.h);
    let legacy_band = t.title.is_none() && t.continued.is_some();
    let first_head = title_h(false) + header_h;
    let cont_head = title_h(true) + header_h + if legacy_band { 21.0 } else { 0.0 };
    let page_rows = p.body_h();
    let breaks = plan_breaks(
        &heights,
        &forbid,
        p.room() - first_head,
        page_rows - cont_head,
        p.fresh(),
    );
    let mut seg_top: Option<f32> = None;
    for (i, rf) in frags.into_iter().enumerate() {
        let new_page = breaks.contains(&i);
        if i == 0 || new_page {
            if let Some(st) = seg_top.take() {
                draw_box(p, st);
            }
            if new_page {
                p.new_page();
            }
            let continued = i > 0;
            if continued
                && legacy_band
                && let Some(label) = &t.continued
            {
                let mut f = Frag::new();
                f.shape(
                    0.0,
                    0.0,
                    total_w,
                    17.0,
                    4.0,
                    Some(pal::PANEL),
                    Some((pal::LINE, 0.6)),
                );
                let l = format!("{} (CONTINUED)", label.to_uppercase());
                f.text(8.0, 11.6, &l, 7.2, true, pal::SLATE, 0.7);
                let top = p.y;
                p.emit(f, MARGIN, top);
                p.y -= 21.0;
            }
            if let Some(tf) = lead_frag(t, total_w, continued) {
                let top = p.y;
                let h = tf.h;
                p.emit(tf, MARGIN, top);
                p.y -= h;
            }
            seg_top = Some(p.y);
            if let Some(h) = &header {
                let top = p.y;
                let hf = Frag {
                    h: h.h,
                    ops: h.ops.clone(),
                };
                p.emit(hf, MARGIN, top);
                p.y -= h.h;
            }
        }
        let top = p.y;
        p.emit(rf.f, MARGIN, top);
        p.y -= rf.h;
    }
    if let Some(st) = seg_top.take() {
        draw_box(p, st);
    }
    p.y -= 10.0;
}

pub(in crate::report) fn table_scan(t: &StyledTable, scan: &mut dyn FnMut(&str)) {
    for s in t.title.iter().chain(&t.title_hint) {
        scan(s);
    }
    if let Some(b) = &t.band {
        band_scan(b, scan);
    }
    for c in &t.columns {
        scan(&c.header);
    }
    if let Some(e) = &t.empty {
        scan(e);
    }
    if let Some(c) = &t.continued {
        scan(c);
    }
    let cell_scan = |cell: &Cell, scan: &mut dyn FnMut(&str)| {
        for it in &cell.items {
            match it {
                CellItem::Text(x) => scan(&x.text),
                CellItem::Chips(cs) => cs.iter().for_each(|c| scan(&c.text)),
                CellItem::Tile(s) | CellItem::Missing(s) => scan(s),
                CellItem::Rich(r) => rich_scan(r, scan),
                CellItem::Heading { text, right, .. } => {
                    scan(text);
                    if let Some(r) = right {
                        scan(r);
                    }
                }
                _ => {}
            }
        }
    };
    if let Some(hc) = &t.header_cells {
        hc.iter().for_each(|c| cell_scan(c, scan));
    }
    for r in &t.rows {
        r.cells.iter().for_each(|c| cell_scan(c, scan));
    }
}

#[cfg(test)]
mod tests {
    use super::plan_breaks;

    #[test]
    fn plan_breaks_follows_the_rules() {
        let h = vec![10.0; 10];
        let mut forbid = vec![false; 11];
        forbid[1] = true;
        // Everything fits: no breaks.
        assert!(plan_breaks(&h, &forbid, 100.0, 100.0, false).is_empty());
        // Not even two rows fit the first page: the table starts on a new page.
        assert_eq!(plan_breaks(&h, &forbid, 15.0, 100.0, false), vec![0]);
        // On a fresh page the same input just breaks where it must.
        assert_eq!(plan_breaks(&h, &forbid, 55.0, 55.0, true), vec![5]);
        // Rows 4..=5 are tied: the break backs up to row 4 (and the last page is not a lone row).
        forbid[5] = true;
        assert_eq!(plan_breaks(&h, &forbid, 55.0, 55.0, true), vec![4, 8]);
        // 11 rows of 10 on pages of 50: the last page would hold one row, so the row before joins it.
        let h = vec![10.0; 11];
        let forbid = vec![false; 12];
        assert_eq!(plan_breaks(&h, &forbid, 50.0, 50.0, true), vec![5, 9]);
        // A row taller than the page is placed, not looped on.
        let h = vec![10.0, 900.0, 10.0];
        assert_eq!(plan_breaks(&h, &[false; 4], 50.0, 50.0, true), vec![1, 2]);
    }
}
