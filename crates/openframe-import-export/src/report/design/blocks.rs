//! Masthead, metrics, section heading, banner, callout, legend and text blocks.

use super::*;

// ------------------------------------------------------------------ masthead

/// The block at the top of a production document.
#[derive(Debug, Clone)]
pub struct Masthead {
    /// Print the lockup at the top left and this label at the top right, above a hairline
    /// (breakdown and catalog style).
    pub top_bar: Option<String>,
    pub eyebrow: Option<Txt>,
    pub title: String,
    pub title_size: f32,
    pub subtitle: Option<String>,
    pub badges: Vec<Chip>,
    /// Muted text on the badge row, after the badges.
    pub badge_note: Option<String>,
    /// Muted detail lines under the title block.
    pub lines: Vec<String>,
    pub side: Option<Side>,
    /// A navy rule under the masthead.
    pub rule: bool,
}

impl Masthead {
    pub fn new(title: impl Into<String>) -> Masthead {
        Masthead {
            top_bar: None,
            eyebrow: None,
            title: title.into(),
            title_size: 26.0,
            subtitle: None,
            badges: Vec::new(),
            badge_note: None,
            lines: Vec::new(),
            side: None,
            rule: false,
        }
    }
    pub fn eyebrow(mut self, text: impl Into<String>, color: Rgb) -> Masthead {
        self.eyebrow = Some(
            Txt::new(text.into().to_uppercase())
                .size(7.8)
                .bold()
                .track(1.5)
                .color(color),
        );
        self
    }
}

/// What the right hand side of a masthead shows.
#[derive(Debug, Clone)]
pub enum Side {
    /// Navy card: small label, big value, lines (text, bold), foot line.
    Dark {
        label: String,
        big: Option<String>,
        lines: Vec<(String, bool)>,
        foot: Option<String>,
    },
    /// Light panel with right aligned lines (text, bold).
    Panel { lines: Vec<(String, bool)> },
    /// Label and value pairs beside a thin rule.
    Facts(Vec<(String, String)>),
    /// A chip with right aligned lines under it.
    Badge {
        chip: Chip,
        lines: Vec<(String, bool)>,
    },
}

pub(in crate::report) fn masthead(p: &mut Pager, m: &Masthead) {
    let cw = p.content_w();
    let mut f = Frag::new();
    let mut y = 0.0f32;
    if let Some(bar) = &m.top_bar {
        let mut lock = Frag::new();
        lock.ops.push(Op::Mark {
            x: 0.0,
            y: -19.0,
            size: 19.0,
            color: pal::ORANGE,
        });
        lock.text(23.0, 9.0, "Open", 11.0, false, pal::NAVY, 0.0);
        let ow = text_width_pt("Open", 11.0, false);
        lock.text(23.0 + ow, 9.0, "Frame", 11.0, true, pal::NAVY, 0.0);
        lock.text(23.5, 18.0, "STUDIO", 4.2, true, pal::ORANGE, 2.6);
        f.put(lock, 0.0, 0.0);
        let label = bar.to_uppercase();
        let lw = tw(&label, 8.0, true, 1.4);
        f.text(cw - lw, 12.0, &label, 8.0, true, pal::SLATE, 1.4);
        y = 26.0;
        f.line(0.0, y, cw, y, pal::LINE, 0.8);
        y += 12.0;
    }
    let side_w = match &m.side {
        None => 0.0,
        Some(Side::Dark { .. }) => 150.0,
        Some(Side::Panel { .. }) => 160.0,
        Some(Side::Facts(_)) => (cw * 0.34).min(250.0),
        Some(Side::Badge { .. }) => (cw * 0.30).min(220.0),
    };
    let left_w = if side_w > 0.0 { cw - side_w - 18.0 } else { cw };
    let mut ly = 0.0f32;
    let mut left = Frag::new();
    if let Some(e) = &m.eyebrow {
        ly += put_txt(&mut left, 0.0, ly, left_w, e) + 3.0;
    }
    let title = Txt::new(m.title.clone())
        .size(m.title_size)
        .bold()
        .color(pal::NAVY)
        .leading(1.12);
    ly += put_txt(&mut left, 0.0, ly, left_w, &title) + 5.0;
    if let Some(s) = &m.subtitle {
        let t = Txt::new(s.clone())
            .size(12.5)
            .color(pal::SLATE)
            .leading(1.25);
        ly += put_txt(&mut left, 0.0, ly, left_w, &t) + 4.0;
    }
    if !m.badges.is_empty() || m.badge_note.is_some() {
        let mut bx = 0.0f32;
        let mut bh = 0.0f32;
        for b in &m.badges {
            let mut chip = b.clone();
            chip.size = 7.2;
            // A larger, bordered pill so the draft or final state is found at once.
            let h = chip.height() + 5.0;
            let w = chip.width() + 12.0;
            left.shape(
                bx,
                ly,
                w,
                h,
                5.0,
                Some(chip.tone.fill()),
                Some((chip.tone.line(), 1.1)),
            );
            left.text(
                bx + 11.0,
                ly + (h + 0.72 * chip.size) / 2.0,
                &chip.text,
                chip.size,
                true,
                chip.tone.text(),
                chip.track,
            );
            bx += w + 9.0;
            bh = bh.max(h);
        }
        if let Some(n) = &m.badge_note {
            let t = Txt::new(n.clone()).size(9.0).color(pal::SLATE);
            let nw = (left_w - bx).max(80.0);
            let mut tmp = Frag::new();
            let nh = put_txt(&mut tmp, 0.0, 0.0, nw, &t);
            let off = if bh > nh { (bh - nh) / 2.0 } else { 0.0 };
            left.put(tmp, bx, ly + off);
            bh = bh.max(nh);
        }
        ly += bh + 7.0;
    }
    for line in &m.lines {
        let t = Txt::new(line.clone()).size(8.6).color(pal::SLATE);
        ly += put_txt(&mut left, 0.0, ly, left_w, &t) + 1.5;
    }
    let mut side = Frag::new();
    let mut sh = 0.0f32;
    if let Some(s) = &m.side {
        match s {
            Side::Dark {
                label,
                big,
                lines,
                foot,
            } => {
                let pad = 14.0;
                let iw = side_w - 2.0 * pad;
                let mut cy = pad;
                let mut inner = Frag::new();
                cy += put_txt(
                    &mut inner,
                    pad,
                    cy,
                    iw,
                    &Txt::new(label.to_uppercase())
                        .size(7.0)
                        .bold()
                        .track(1.4)
                        .color(pal::DARK_MUTED)
                        .after(2.0),
                );
                if let Some(b) = big {
                    cy += put_txt(
                        &mut inner,
                        pad,
                        cy,
                        iw,
                        &Txt::new(b.clone())
                            .size(27.0)
                            .bold()
                            .color(pal::WHITE)
                            .leading(1.1)
                            .after(3.0),
                    );
                }
                for (t, bold) in lines {
                    let mut tx = Txt::new(t.clone())
                        .size(10.0)
                        .color(pal::WHITE)
                        .leading(1.25);
                    if *bold {
                        tx = tx.bold();
                    }
                    cy += put_txt(&mut inner, pad, cy, iw, &tx);
                }
                if let Some(ft) = foot {
                    cy += 5.0;
                    cy += put_txt(
                        &mut inner,
                        pad,
                        cy,
                        iw,
                        &Txt::new(ft.clone()).size(8.0).color(pal::DARK_MUTED),
                    );
                }
                sh = cy + pad;
                side.shape(0.0, 0.0, side_w, sh, 8.0, Some(pal::NAVY), None);
                side.put(inner, 0.0, 0.0);
            }
            Side::Panel { lines } => {
                let pad = 12.0;
                let iw = side_w - 2.0 * pad;
                let mut cy = pad;
                let mut inner = Frag::new();
                for (i, (t, bold)) in lines.iter().enumerate() {
                    let mut tx = Txt::new(t.clone())
                        .size(if i == 0 { 11.0 } else { 9.0 })
                        .color(if i == 0 { pal::NAVY } else { pal::SLATE })
                        .align(Align::Right)
                        .leading(1.35);
                    if *bold || i == 0 {
                        tx = tx.bold();
                    }
                    cy += put_txt(&mut inner, pad, cy, iw, &tx);
                }
                sh = cy + pad;
                side.shape(
                    0.0,
                    0.0,
                    side_w,
                    sh,
                    6.0,
                    Some(pal::PANEL),
                    Some((pal::LINE, 0.8)),
                );
                side.put(inner, 0.0, 0.0);
            }
            Side::Facts(pairs) => {
                let pad = 12.0;
                let iw = side_w - pad;
                let mut cy = 0.0f32;
                let mut inner = Frag::new();
                for (k, v) in pairs {
                    let kw = tw(k, 8.4, true, 0.0);
                    inner.text(pad, cy + 8.4, k, 8.4, true, pal::NAVY, 0.0);
                    let vt = Txt::new(format!("· {v}"))
                        .size(8.4)
                        .color(pal::SLATE)
                        .leading(1.35);
                    let vw = (iw - kw - 5.0).max(60.0);
                    let mut tmp = Frag::new();
                    let hh = put_txt(&mut tmp, 0.0, 0.0, vw, &vt);
                    inner.put(tmp, pad + kw + 4.0, cy);
                    cy += hh.max(8.4 * 1.35) + 3.0;
                }
                sh = (cy - 3.0).max(0.0);
                side.line(0.0, 0.0, 0.0, sh, pal::LINE, 1.2);
                side.put(inner, 0.0, 0.0);
            }
            Side::Badge { chip, lines } => {
                let mut cy = 0.0f32;
                let mut chip = chip.clone();
                chip.size = 7.4;
                let (w, h) = (chip.width() + 10.0, chip.height() + 4.0);
                side.shape(
                    side_w - w,
                    0.0,
                    w,
                    h,
                    5.0,
                    Some(chip.tone.fill()),
                    Some((chip.tone.line(), 1.0)),
                );
                side.text(
                    side_w - w + 10.0,
                    (h + 0.72 * chip.size) / 2.0,
                    &chip.text,
                    chip.size,
                    true,
                    chip.tone.text(),
                    chip.track,
                );
                cy += h + 8.0;
                for (i, (t, bold)) in lines.iter().enumerate() {
                    let mut tx = Txt::new(t.clone())
                        .size(if i == 0 { 11.0 } else { 8.6 })
                        .color(if i == 0 { pal::INK } else { pal::SLATE })
                        .align(Align::Right)
                        .leading(1.35);
                    if *bold {
                        tx = tx.bold();
                    }
                    cy += put_txt(&mut side, 0.0, cy, side_w, &tx) + 1.0;
                }
                sh = cy;
            }
        }
    }
    let bh = ly.max(sh);
    f.put(left, 0.0, y);
    if side_w > 0.0 {
        f.put(side, cw - side_w, y);
    }
    let mut total = y + bh;
    if m.rule {
        total += 9.0;
        f.line(0.0, total, cw, total, pal::NAVY, 1.6);
        total += 10.0;
    } else {
        total += 10.0;
    }
    f.h = total;
    p.ensure(total);
    let top = p.y;
    p.emit(f, MARGIN, top);
    p.y -= total;
}

// ------------------------------------------------------------------ metrics

#[derive(Debug, Clone)]
pub struct Metric {
    pub value: String,
    pub label: String,
    pub tone: Option<Tone>,
}

impl Metric {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Metric {
        Metric {
            value: value.into(),
            label: label.into(),
            tone: None,
        }
    }
    pub fn tone(mut self, t: Tone) -> Metric {
        self.tone = Some(t);
        self
    }
}

/// A row of summary cards (value over label).
#[derive(Debug, Clone)]
pub struct Metrics {
    pub items: Vec<Metric>,
    /// One thin line per card (value beside its label) for pages where height is precious.
    pub compact: bool,
}

pub(in crate::report) fn metrics(p: &mut Pager, m: &Metrics) {
    if m.items.is_empty() {
        return;
    }
    let cw = p.content_w();
    let n = m.items.len();
    let gap = 8.0;
    let w = (cw - gap * (n as f32 - 1.0)) / n as f32;
    if m.compact {
        let mut f = Frag::new();
        let h = 32.0;
        for (i, it) in m.items.iter().enumerate() {
            let x = i as f32 * (w + gap);
            f.shape(x, 0.0, w, h, 6.0, Some(pal::PANEL), Some((pal::LINE, 0.8)));
            let color = it.tone.map(|t| t.text()).unwrap_or(pal::NAVY);
            let vw = tw(&it.value, 16.0, true, 0.0);
            f.text(x + 10.0, 21.5, &it.value, 16.0, true, color, 0.0);
            let label = Txt::label(it.label.clone()).size(6.6);
            let lines = txt_lines(&label, w - vw - 30.0);
            let top = (h - lines.len() as f32 * 6.6 * 1.3) / 2.0;
            put_txt(&mut f, x + 10.0 + vw + 8.0, top, w - vw - 30.0, &label);
        }
        p.ensure(h + 10.0);
        let top = p.y;
        p.emit(f, MARGIN, top);
        p.y -= h + 10.0;
        return;
    }
    let mut f = Frag::new();
    let mut h = 0.0f32;
    let mut cells = Vec::new();
    for it in &m.items {
        let label = Txt::label(it.label.clone());
        h = h.max(32.0 + txt_height(&label, w - 20.0) + 8.0);
        cells.push((it, label));
    }
    for (i, (it, label)) in cells.iter().enumerate() {
        let x = i as f32 * (w + gap);
        f.shape(x, 0.0, w, h, 6.0, Some(pal::PANEL), Some((pal::LINE, 0.8)));
        let color = it.tone.map(|t| t.text()).unwrap_or(pal::NAVY);
        let mut size = 19.0f32;
        while size > 9.0 && tw(&it.value, size, true, 0.0) > w - 20.0 {
            size -= 1.0;
        }
        let v = Txt::new(it.value.clone())
            .size(size)
            .bold()
            .color(color)
            .leading(1.1);
        put_txt(&mut f, x + 10.0, 8.0 + (19.0 - size) / 2.0, w - 20.0, &v);
        put_txt(&mut f, x + 10.0, 32.0, w - 20.0, label);
    }
    let total = h + 12.0;
    p.ensure(total);
    let top = p.y;
    p.emit(f, MARGIN, top);
    p.y -= total;
}

// ------------------------------------------------------------------ section

/// A section heading: bold navy title, optional hint on the right, navy rule.
#[derive(Debug, Clone)]
pub struct Section {
    pub title: String,
    pub hint: Option<String>,
}

impl Section {
    pub fn new(title: impl Into<String>, hint: Option<String>) -> Section {
        Section {
            title: title.into(),
            hint,
        }
    }
}

pub(in crate::report) fn section(p: &mut Pager, s: &Section) {
    let cw = p.content_w();
    let title = Txt::new(s.title.to_uppercase())
        .size(11.5)
        .bold()
        .track(0.6)
        .color(pal::NAVY);
    let hint_w = s
        .hint
        .as_ref()
        .map(|h| tw(h, 8.0, true, 0.0).min(cw * 0.5))
        .unwrap_or(0.0);
    let mut f = Frag::new();
    let th = put_txt(&mut f, 0.0, 0.0, (cw - hint_w - 12.0).max(60.0), &title);
    if let Some(h) = &s.hint {
        let t = Txt::new(h.clone())
            .size(8.0)
            .bold()
            .color(pal::SLATE)
            .align(Align::Right);
        put_txt(&mut f, cw - hint_w, 3.0, hint_w, &t);
    }
    let rule = th + 3.0;
    f.line(0.0, rule, cw, rule, pal::NAVY, 1.4);
    let total = rule + 9.0;
    // Keep a heading with what follows it.
    p.ensure(total + 70.0);
    p.y -= 8.0;
    let top = p.y;
    p.emit(f, MARGIN, top);
    p.y -= total;
}

// ------------------------------------------------------------------ banner

/// A dark band: a shooting day, a date band, a section opener.
#[derive(Debug, Clone)]
pub struct Banner {
    pub eyebrow: Option<String>,
    pub title: String,
    pub right: Option<String>,
    /// Runs (text, bold) on the tinted line under the band, left and right.
    pub sub_left: Vec<(String, bool)>,
    pub sub_right: Vec<(String, bool)>,
    pub fill: Rgb,
    /// Height of content that must fit on the page with the band.
    pub keep_next: f32,
}

impl Banner {
    pub fn new(title: impl Into<String>) -> Banner {
        Banner {
            eyebrow: None,
            title: title.into(),
            right: None,
            sub_left: Vec::new(),
            sub_right: Vec::new(),
            fill: pal::NAVY,
            keep_next: 90.0,
        }
    }
}

fn runs_width(runs: &[(String, bool)], size: f32) -> f32 {
    runs.iter().map(|(t, b)| text_width(t, size, *b)).sum()
}

fn put_runs(f: &mut Frag, mut x: f32, base: f32, runs: &[(String, bool)], size: f32, color: Rgb) {
    for (t, b) in runs {
        f.text(x, base, t, size, *b, color, 0.0);
        x += text_width(t, size, *b);
    }
}

pub(in crate::report) fn banner(p: &mut Pager, b: &Banner) {
    let cw = p.content_w();
    let has_sub = !b.sub_left.is_empty() || !b.sub_right.is_empty();
    let mut f = Frag::new();
    let right_text = b.right.as_ref().map(|r| r.to_uppercase());
    let right_w = right_text
        .as_ref()
        .map(|r| tw(r, 9.0, false, 0.5).min(cw * 0.42))
        .unwrap_or(0.0);
    let right_lines: Vec<String> = right_text
        .as_ref()
        .map(|r| wrap_pt(r, right_w, 9.0, false))
        .unwrap_or_default();
    let left_w = cw - right_w - 40.0;
    let mut x = 14.0;
    let mut eyebrow = None;
    if let Some(e) = &b.eyebrow {
        let et = e.to_uppercase();
        let ew = tw(&et, 8.0, true, 1.4);
        eyebrow = Some(et);
        x += ew + 10.0;
    }
    let title = Txt::new(b.title.clone())
        .size(14.0)
        .bold()
        .color(pal::WHITE)
        .leading(1.15);
    let lines = txt_lines(&title, (left_w - (x - 14.0)).max(80.0));
    let band_h = (lines.len() as f32 * 16.1 + 19.0)
        .max(right_lines.len() as f32 * 11.5 + 18.0)
        .max(36.0);
    f.shape_c(
        0.0,
        0.0,
        cw,
        band_h,
        7.0,
        true,
        !has_sub,
        Some(b.fill),
        None,
    );
    if let Some(et) = eyebrow {
        f.text(
            14.0,
            band_h / 2.0 + 3.0,
            &et,
            8.0,
            true,
            Rgb::hex(0xf5c27a),
            1.4,
        );
    }
    let first = (band_h - lines.len() as f32 * 16.1) / 2.0 + 12.4;
    for (k, l) in lines.iter().enumerate() {
        f.text(x, first + k as f32 * 16.1, l, 14.0, true, pal::WHITE, 0.0);
    }
    let rh = right_lines.len() as f32 * 11.5;
    for (k, l) in right_lines.iter().enumerate() {
        let w = tw(l, 9.0, false, 0.5);
        f.text(
            cw - 14.0 - w,
            (band_h - rh) / 2.0 + 9.0 + k as f32 * 11.5,
            l,
            9.0,
            false,
            pal::WHITE,
            0.5,
        );
    }
    let mut total = band_h;
    if has_sub {
        let sh = 21.0;
        f.shape_c(
            0.0,
            band_h,
            cw,
            sh,
            0.0,
            false,
            false,
            Some(pal::TINT),
            None,
        );
        put_runs(&mut f, 14.0, band_h + 14.0, &b.sub_left, 8.6, pal::INK);
        let rw = runs_width(&b.sub_right, 8.6);
        put_runs(
            &mut f,
            cw - 14.0 - rw,
            band_h + 14.0,
            &b.sub_right,
            8.6,
            pal::INK,
        );
        total += sh;
    }
    p.ensure(total + b.keep_next + 4.0);
    let top = p.y - 4.0;
    p.emit(f, MARGIN, top);
    p.y -= total + 4.0;
}

// ------------------------------------------------------------------ callout

/// One column of a callout.
#[derive(Debug, Clone)]
pub struct CalloutCol {
    pub weight: f32,
    pub eyebrow: Option<Txt>,
    pub big: Option<Txt>,
    pub chip: Option<Chip>,
    pub lines: Vec<Txt>,
}

impl CalloutCol {
    pub fn new(weight: f32) -> CalloutCol {
        CalloutCol {
            weight,
            eyebrow: None,
            big: None,
            chip: None,
            lines: Vec::new(),
        }
    }
}

/// A tinted, bordered box with up to a few columns (crew call, notices).
#[derive(Debug, Clone)]
pub struct Callout {
    pub tone: Tone,
    pub cols: Vec<CalloutCol>,
}

pub(in crate::report) fn callout(p: &mut Pager, c: &Callout) {
    if c.cols.is_empty() {
        return;
    }
    let cw = p.content_w();
    let pad = 13.0;
    let gap = 16.0;
    let total_w: f32 = c.cols.iter().map(|x| x.weight).sum::<f32>().max(0.1);
    let avail = cw - 2.0 * pad - gap * (c.cols.len() as f32 - 1.0);
    let mut inner = Frag::new();
    let mut h = 0.0f32;
    let mut x = pad;
    for col in &c.cols {
        let w = avail * col.weight / total_w;
        let mut cy = pad - 2.0;
        if let Some(e) = &col.eyebrow {
            cy += put_txt(&mut inner, x, cy, w, e) + 3.0;
        }
        if let Some(b) = &col.big {
            cy += put_txt(&mut inner, x, cy, w, b) + 3.0;
        }
        if let Some(ch) = &col.chip {
            let mut ch = ch.clone();
            ch.size = 8.0;
            let chw = ch.width().max(w.min(140.0)).min(w);
            let chh = ch.height() + 3.0;
            inner.shape(
                x,
                cy,
                chw,
                chh,
                4.0,
                Some(ch.tone.fill()),
                Some((ch.tone.line(), 0.8)),
            );
            inner.text(
                x + 8.0,
                cy + (chh + 0.72 * ch.size) / 2.0,
                &ch.text,
                ch.size,
                true,
                ch.tone.text(),
                ch.track,
            );
            cy += chh + 4.0;
        }
        for l in &col.lines {
            cy += put_txt(&mut inner, x, cy, w, l);
        }
        h = h.max(cy + pad - 2.0);
        x += w + gap;
    }
    let mut f = Frag::new();
    f.shape(
        0.0,
        0.0,
        cw,
        h,
        7.0,
        Some(c.tone.fill()),
        Some((c.tone.line(), 1.0)),
    );
    let mut dx = pad;
    for col in c.cols.iter().take(c.cols.len() - 1) {
        dx += avail * col.weight / total_w + gap / 2.0;
        f.line(dx, 12.0, dx, h - 12.0, c.tone.line(), 0.7);
        dx += gap / 2.0;
    }
    f.put(inner, 0.0, 0.0);
    p.ensure(h + 8.0);
    let top = p.y;
    p.emit(f, MARGIN, top);
    p.y -= h + 10.0;
}

// ------------------------------------------------------------------ legend

#[derive(Debug, Clone)]
pub struct LegendItem {
    pub swatch: Rgb,
    pub border: Option<Rgb>,
    /// Short code printed inside the swatch.
    pub code: Option<(String, Rgb)>,
    pub label: String,
}

/// A key for colours and codes. `columns` of 0 flows items; otherwise an even grid of boxed
/// items.
#[derive(Debug, Clone)]
pub struct Legend {
    pub title: Option<String>,
    pub items: Vec<LegendItem>,
    pub columns: usize,
}

fn legend_swatch(f: &mut Frag, x: f32, dy: f32, it: &LegendItem) -> f32 {
    let (sw, sh) = (if it.code.is_some() { 24.0 } else { 15.0 }, 12.0);
    f.shape(
        x,
        dy,
        sw,
        sh,
        2.5,
        Some(it.swatch),
        it.border.map(|b| (b, 0.7)),
    );
    if let Some((code, color)) = &it.code {
        let cwid = tw(code, 6.8, true, 0.0);
        f.text(
            x + (sw - cwid) / 2.0,
            dy + 8.6,
            code,
            6.8,
            true,
            *color,
            0.0,
        );
    }
    sw
}

pub(in crate::report) fn legend(p: &mut Pager, l: &Legend) {
    if l.items.is_empty() {
        return;
    }
    let cw = p.content_w();
    let mut f = Frag::new();
    let mut y = 0.0f32;
    if let Some(t) = &l.title {
        y += put_txt(&mut f, 0.0, 0.0, cw, &Txt::label(t.clone()).after(4.0));
    }
    if l.columns > 0 {
        let n = l.columns;
        let gap = 6.0;
        let w = (cw - gap * (n as f32 - 1.0)) / n as f32;
        for row in l.items.chunks(n) {
            let mut rh = 0.0f32;
            let mut labels = Vec::new();
            for it in row {
                let sw = if it.code.is_some() { 24.0 } else { 15.0 };
                let t = Txt::new(it.label.clone())
                    .size(8.0)
                    .color(pal::SLATE)
                    .leading(1.25);
                rh = rh.max(txt_height(&t, w - sw - 20.0).max(12.0) + 10.0);
                labels.push(t);
            }
            for (i, it) in row.iter().enumerate() {
                let x = i as f32 * (w + gap);
                f.shape(x, y, w, rh, 4.0, Some(pal::WHITE), Some((pal::LINE, 0.8)));
                let sw = legend_swatch(&mut f, x + 6.0, y + 5.0, it);
                put_txt(
                    &mut f,
                    x + 6.0 + sw + 6.0,
                    y + 5.5,
                    w - sw - 20.0,
                    &labels[i],
                );
            }
            y += rh + gap;
        }
    } else {
        let mut x = 0.0f32;
        let mut rowtop = y;
        for it in &l.items {
            let sw = if it.code.is_some() { 24.0 } else { 15.0 };
            let w = sw + 5.0 + tw(&it.label, 7.6, false, 0.0) + 12.0;
            if x > 0.0 && x + w > cw {
                x = 0.0;
                rowtop += 16.0;
            }
            legend_swatch(&mut f, x, rowtop + 1.5, it);
            f.text(
                x + sw + 5.0,
                rowtop + 10.0,
                &it.label,
                7.6,
                false,
                pal::SLATE,
                0.0,
            );
            x += w;
        }
        y = rowtop + 16.0;
    }
    p.ensure(y + 4.0);
    let top = p.y - 2.0;
    p.emit(f, MARGIN, top);
    p.y -= y + 8.0;
}

// ------------------------------------------------------------------ text

pub(in crate::report) fn text(p: &mut Pager, t: &Txt) {
    let w = p.content_w();
    let lh = t.size * t.leading;
    for l in txt_lines(t, w) {
        p.ensure(lh);
        let mut f = Frag::new();
        let lw = tw(&l, t.size, t.bold, t.track);
        let x = match t.align {
            Align::Left => 0.0,
            Align::Center => ((w - lw) / 2.0).max(0.0),
            Align::Right => (w - lw).max(0.0),
        };
        f.text(x, t.size, &l, t.size, t.bold, t.color, t.track);
        let top = p.y;
        p.emit(f, MARGIN, top);
        p.y -= lh;
    }
    p.y -= t.after;
}

/// Every piece of text a design block prints, for the unsupported character scan.
pub(in crate::report) fn scan_block(b: &Block, scan: &mut dyn FnMut(&str)) {
    match b {
        Block::Masthead(m) => {
            if let Some(t) = &m.top_bar {
                scan(t);
            }
            if let Some(e) = &m.eyebrow {
                scan(&e.text);
            }
            scan(&m.title);
            if let Some(s) = &m.subtitle {
                scan(s);
            }
            for c in &m.badges {
                scan(&c.text);
            }
            if let Some(n) = &m.badge_note {
                scan(n);
            }
            for l in &m.lines {
                scan(l);
            }
            match &m.side {
                Some(Side::Dark {
                    label,
                    big,
                    lines,
                    foot,
                }) => {
                    scan(label);
                    if let Some(b) = big {
                        scan(b);
                    }
                    for (t, _) in lines {
                        scan(t);
                    }
                    if let Some(f) = foot {
                        scan(f);
                    }
                }
                Some(Side::Panel { lines }) => lines.iter().for_each(|(t, _)| scan(t)),
                Some(Side::Facts(p)) => p.iter().for_each(|(k, v)| {
                    scan(k);
                    scan(v);
                }),
                Some(Side::Badge { chip, lines }) => {
                    scan(&chip.text);
                    lines.iter().for_each(|(t, _)| scan(t));
                }
                None => {}
            }
        }
        Block::Metrics(m) => m.items.iter().for_each(|i| {
            scan(&i.value);
            scan(&i.label);
        }),
        Block::Section(s) => {
            scan(&s.title);
            if let Some(h) = &s.hint {
                scan(h);
            }
        }
        Block::Banner(b) => {
            if let Some(e) = &b.eyebrow {
                scan(e);
            }
            scan(&b.title);
            if let Some(r) = &b.right {
                scan(r);
            }
            b.sub_left
                .iter()
                .chain(&b.sub_right)
                .for_each(|(t, _)| scan(t));
        }
        Block::Callout(c) => c.cols.iter().for_each(|col| {
            if let Some(e) = &col.eyebrow {
                scan(&e.text);
            }
            if let Some(b) = &col.big {
                scan(&b.text);
            }
            if let Some(ch) = &col.chip {
                scan(&ch.text);
            }
            col.lines.iter().for_each(|l| scan(&l.text));
        }),
        Block::Legend(l) => {
            if let Some(t) = &l.title {
                scan(t);
            }
            l.items.iter().for_each(|i| {
                scan(&i.label);
                if let Some((c, _)) = &i.code {
                    scan(c);
                }
            });
        }
        Block::Text(t) => scan(&t.text),
        Block::Cards(c) => cards_scan(c, scan),
        Block::Styled(t) => table_scan(t, scan),
        _ => {}
    }
}
