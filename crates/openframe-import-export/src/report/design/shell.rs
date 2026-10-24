//! The shell of a production report: a compact head with a side panel, context bands (snapshot,
//! filter, open scenes) and a metrics strip that is as wide as its content.

use super::*;

// ------------------------------------------------------------------ report head

/// The compact head of a report: an eyebrow over the title and a meta line on the left, a small
/// panel (label, value, note) on the right.
#[derive(Debug, Clone)]
pub struct ReportHead {
    pub eyebrow: String,
    pub title: String,
    /// The line under the title ("Project: X · Source: Y"); none when both parts are blank.
    pub meta: Option<String>,
    pub panel_label: String,
    pub panel_value: String,
    pub panel_note: String,
}

const EYEBROW_TRACK: f32 = 0.98;
const PANEL_TRACK: f32 = 0.95;

pub(in crate::report) fn report_head(p: &mut Pager, h: &ReportHead) {
    let cw = p.content_w();
    let label = h.panel_label.to_uppercase();
    let inner = tw(&label, 6.8, true, PANEL_TRACK)
        .max(tw(&h.panel_value, 10.5, true, 0.0))
        .max(tw(&h.panel_note, 8.0, false, 0.0));
    let panel_w = (inner + 22.0).clamp(104.0, 150.0);
    let left_w = (cw - panel_w - 14.0).max(120.0);

    let mut f = Frag::new();
    f.shape(0.0, 2.6, 14.0, 2.0, 0.0, Some(pal::ORANGE), None);
    f.text(
        19.0,
        7.0,
        &h.eyebrow.to_uppercase(),
        7.0,
        true,
        pal::SLATE,
        EYEBROW_TRACK,
    );
    let mut ly = 10.0 + 3.0;
    let title = Txt::new(h.title.clone())
        .size(20.0)
        .bold()
        .color(pal::NAVY)
        .leading(1.1);
    ly += put_txt(&mut f, 0.0, ly, left_w, &title) + 3.5;
    if let Some(m) = &h.meta {
        let t = Txt::new(m.clone()).size(8.0).color(pal::SLATE);
        ly += put_txt(&mut f, 0.0, ly, left_w, &t);
    }

    let mut panel = Frag::new();
    let (pad_x, mut py) = (11.0, 6.0);
    let put_right =
        |g: &mut Frag, t: &str, size: f32, bold: bool, track: f32, base: f32, c: Rgb| {
            g.text(
                panel_w - pad_x - tw(t, size, bold, track),
                base,
                t,
                size,
                bold,
                c,
                track,
            );
        };
    put_right(
        &mut panel,
        &label,
        6.8,
        true,
        PANEL_TRACK,
        py + 6.8,
        pal::SLATE,
    );
    py += 6.8 * 1.3 + 1.5;
    put_right(
        &mut panel,
        &h.panel_value,
        10.5,
        true,
        0.0,
        py + 10.5,
        pal::NAVY,
    );
    py += 10.5 * 1.2 + 1.5;
    put_right(
        &mut panel,
        &h.panel_note,
        8.0,
        false,
        0.0,
        py + 8.0,
        pal::SLATE,
    );
    py += 8.0 * 1.3 + 7.0;
    f.shape(
        cw - panel_w,
        0.0,
        panel_w,
        py,
        6.0,
        Some(pal::PANEL),
        Some((pal::LINE, 0.8)),
    );
    f.put(panel, cw - panel_w, 0.0);

    let total = ly.max(py) + 7.0;
    f.h = total;
    p.ensure(total);
    let top = p.y;
    p.emit(f, MARGIN, top);
    p.y -= total;
}

pub(in crate::report) fn report_head_scan(h: &ReportHead, scan: &mut dyn FnMut(&str)) {
    for s in [
        &h.eyebrow,
        &h.title,
        &h.panel_label,
        &h.panel_value,
        &h.panel_note,
    ] {
        scan(s);
    }
    if let Some(m) = &h.meta {
        scan(m);
    }
}

// ------------------------------------------------------------------ context band

/// A band under the head: a stripe, a small label and a line of text that wraps (the saved
/// snapshot note, the filter text, the scenes still open).
#[derive(Debug, Clone)]
pub struct NoteBand {
    pub label: String,
    pub runs: Vec<Run>,
    pub tone: Tone,
    /// The stripe colour at the left edge.
    pub stripe: Rgb,
}

pub(in crate::report) fn note_band(p: &mut Pager, b: &NoteBand) {
    let cw = p.content_w();
    let label = b.label.to_uppercase();
    let label_w = tw(&label, 7.0, true, 0.7);
    let text_x = 3.0 + 9.0 + label_w + 9.0;
    let rich = Rich::new(8.0).leading(1.3).para(b.runs.clone());
    let mut body = Frag::new();
    let th = put_rich(&mut body, 0.0, 0.0, (cw - text_x - 9.0).max(60.0), &rich);
    let h = th.max(7.0 * 1.3) + 10.0;
    let mut f = Frag::new();
    f.shape(0.0, 0.0, cw, h, 3.0, Some(b.tone.fill()), None);
    f.shape_c(0.0, 0.0, 3.0, h, 0.0, false, false, Some(b.stripe), None);
    f.text(12.0, 5.0 + 7.0, &label, 7.0, true, b.tone.text(), 0.7);
    f.put(body, text_x, 5.0);
    let total = h + 6.0;
    f.h = h;
    p.ensure(total);
    let top = p.y;
    p.emit(f, MARGIN, top);
    p.y -= total;
}

pub(in crate::report) fn note_band_scan(b: &NoteBand, scan: &mut dyn FnMut(&str)) {
    scan(&b.label);
    b.runs.iter().for_each(|r| scan(&r.text));
}

// ------------------------------------------------------------------ metrics strip

/// Counts in one rounded strip as wide as its content (value beside its label).
#[derive(Debug, Clone)]
pub struct MetricStrip {
    pub items: Vec<Metric>,
}

const STRIP_VALUE: f32 = 13.0;
const STRIP_LABEL: f32 = 6.8;
const STRIP_TRACK: f32 = 0.68;

pub(in crate::report) fn metric_strip(p: &mut Pager, m: &MetricStrip) {
    if m.items.is_empty() {
        return;
    }
    let cw = p.content_w();
    let parts: Vec<(f32, Txt)> = m
        .items
        .iter()
        .map(|i| {
            (
                tw(&i.value, STRIP_VALUE, true, 0.0),
                Txt::label(i.label.clone())
                    .size(STRIP_LABEL)
                    .track(STRIP_TRACK),
            )
        })
        .collect();
    // Chrome around each metric: padding 10 on both sides and 6 between value and label.
    let chrome = 20.0 + 6.0;
    let natural: Vec<f32> = parts
        .iter()
        .map(|(v, l)| v + chrome + tw(&l.text, l.size, true, l.track))
        .collect();
    let total_natural: f32 = natural.iter().sum();
    // Too wide for the page: the labels share what is left and wrap.
    let fixed: f32 = parts.iter().map(|(v, _)| v + chrome).sum();
    let squeeze = if total_natural > cw {
        ((cw - fixed) / (total_natural - fixed)).max(0.2)
    } else {
        1.0
    };
    let base = 5.5 + STRIP_VALUE * 0.8;
    let lead = STRIP_LABEL * 1.25;
    let mut cells: Vec<(f32, Vec<String>)> = Vec::new();
    let mut max_lines = 1usize;
    for ((v, l), nat) in parts.iter().zip(&natural) {
        let label_w = (nat - v - chrome) * squeeze;
        let lines = tracked_lines(l, label_w.max(20.0) + 0.5);
        max_lines = max_lines.max(lines.len());
        let w = v
            + chrome
            + lines
                .iter()
                .map(|x| tw(x, l.size, true, l.track))
                .fold(0.0, f32::max);
        cells.push((w, lines));
    }
    let h = base + lead * (max_lines as f32 - 1.0) + 6.0;
    let w: f32 = cells.iter().map(|c| c.0).sum();
    let mut f = Frag::new();
    f.shape(
        0.0,
        0.0,
        w,
        h,
        4.0,
        Some(pal::PANEL),
        Some((pal::LINE, 0.8)),
    );
    let mut x = 0.0f32;
    for (k, (it, ((vw, l), (cell_w, lines)))) in
        m.items.iter().zip(parts.iter().zip(&cells)).enumerate()
    {
        if k > 0 {
            f.line(x, 0.0, x, h, pal::LINE, 0.6);
        }
        let color = it.tone.map_or(pal::NAVY, |t| t.text());
        f.text(x + 10.0, base, &it.value, STRIP_VALUE, true, color, 0.0);
        for (j, line) in lines.iter().enumerate() {
            f.text(
                x + 10.0 + vw + 6.0,
                base + j as f32 * lead,
                line,
                l.size,
                true,
                l.color,
                l.track,
            );
        }
        x += cell_w;
    }
    let total = h + 9.0;
    f.h = h;
    p.ensure(total);
    let top = p.y;
    p.emit(f, MARGIN, top);
    p.y -= total;
}

pub(in crate::report) fn metric_strip_scan(m: &MetricStrip, scan: &mut dyn FnMut(&str)) {
    m.items.iter().for_each(|i| {
        scan(&i.value);
        scan(&i.label);
    });
}
