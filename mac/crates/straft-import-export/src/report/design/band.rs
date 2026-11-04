//! The band on top of a table's repeating header: a navy title bar and a tinted block of facts
//! (the day band of the shooting schedule). It repeats on every page the table runs over, with
//! " · continued" after the title, so a day that splits never loses its name or its facts.

use super::*;

/// One run of a fact line: text, optionally bold, or a pill with a tone.
#[derive(Debug, Clone)]
pub struct FactRun {
    pub text: String,
    pub bold: bool,
    pub pill: Option<Tone>,
}

impl FactRun {
    pub fn plain(text: impl Into<String>) -> FactRun {
        FactRun {
            text: text.into(),
            bold: false,
            pill: None,
        }
    }
    pub fn bold(text: impl Into<String>) -> FactRun {
        FactRun {
            text: text.into(),
            bold: true,
            pill: None,
        }
    }
    /// A short text in a bordered pill ("over the 11h target"); it never splits across lines.
    pub fn pill(text: impl Into<String>, tone: Tone) -> FactRun {
        FactRun {
            text: text.into(),
            bold: true,
            pill: Some(tone),
        }
    }
}

#[derive(Debug, Clone)]
pub enum FactLine {
    /// Runs flowing left to right, wrapping at the band width.
    Runs(Vec<FactRun>),
    /// A small tracked label with a value that wraps beside it, under a hairline.
    Labelled { label: String, text: String },
}

/// The band: title (left), a right text such as the date, and the fact lines under it.
#[derive(Debug, Clone)]
pub struct TableBand {
    pub title: String,
    pub right: Option<String>,
    pub fill: Rgb,
    pub facts: Vec<FactLine>,
}

impl TableBand {
    pub fn new(title: impl Into<String>) -> TableBand {
        TableBand {
            title: title.into(),
            right: None,
            fill: pal::NAVY,
            facts: Vec::new(),
        }
    }
}

/// The darker hairline of the design system (card borders, reading guides).
pub(super) const LINE_STRONG: Rgb = Rgb::hex(0xb9c2d1);

const FACT_SIZE: f32 = 8.2;
const FACT_LEAD: f32 = 12.4;
const FACT_PAD_X: f32 = 10.0;
const FACT_PAD_Y: f32 = 6.0;
const PILL_SIZE: f32 = 7.4;
const BAND_SUFFIX: &str = " · continued";

/// A styled piece of a laid out line: where it starts and what it looks like.
pub(super) struct Seg {
    pub x: f32,
    pub text: String,
    pub bold: bool,
    pub pill: Option<Tone>,
}

fn pill_width(text: &str) -> f32 {
    tw(text, PILL_SIZE, true, 0.0) + 10.0
}

/// Runs laid out in lines no wider than `width` (greedy, at spaces). Neighbouring words of one
/// style are merged into one segment; a pill is one unbreakable word.
pub(super) fn flow_runs(runs: &[FactRun], width: f32, size: f32) -> Vec<Vec<Seg>> {
    let mut lines: Vec<Vec<Seg>> = vec![Vec::new()];
    let mut x = 0.0f32;
    let push = |lines: &mut Vec<Vec<Seg>>, x: f32, text: &str, bold: bool, pill: Option<Tone>| {
        let line = lines.last_mut().unwrap();
        if pill.is_none()
            && let Some(last) = line.last_mut()
            && last.pill.is_none()
            && last.bold == bold
        {
            last.text.push_str(text);
            return;
        }
        line.push(Seg {
            x,
            text: text.to_string(),
            bold,
            pill,
        });
    };
    for run in runs {
        if run.text.is_empty() {
            continue;
        }
        if run.pill.is_some() {
            let w = pill_width(&run.text);
            if x > 0.0 && x + w > width {
                lines.push(Vec::new());
                x = 0.0;
            }
            push(&mut lines, x, &run.text, true, run.pill);
            x += w + 4.0;
            continue;
        }
        for word in run.text.split_inclusive(' ') {
            let trimmed = word.trim_end_matches(' ');
            let w = text_width(trimmed, size, run.bold);
            let at_start = lines.last().unwrap().is_empty();
            if !at_start && x + w > width {
                lines.push(Vec::new());
                x = 0.0;
            }
            // A space at the head of a line is dropped.
            let word = if lines.last().unwrap().is_empty() {
                word.trim_start_matches(' ')
            } else {
                word
            };
            if word.is_empty() {
                continue;
            }
            push(&mut lines, x, word, run.bold, None);
            x += text_width(word, size, run.bold);
        }
    }
    for l in &mut lines {
        if let Some(last) = l.last_mut()
            && last.pill.is_none()
        {
            last.text = last.text.trim_end().to_string();
        }
    }
    lines.retain(|l| !l.is_empty());
    lines
}

/// Draw a flowed line at (x0, base); returns nothing, the caller owns the vertical rhythm.
pub(super) fn put_segs(
    f: &mut Frag,
    x0: f32,
    base: f32,
    line: &[Seg],
    size: f32,
    (color, bold_color): (Rgb, Rgb),
) {
    for s in line {
        if let Some(tone) = s.pill {
            let w = pill_width(&s.text);
            let h = PILL_SIZE * 1.45 + 3.0;
            f.shape(
                x0 + s.x,
                base - PILL_SIZE * 0.78 - 3.4,
                w,
                h,
                4.0,
                Some(tone.fill()),
                Some((tone.line(), 0.6)),
            );
            f.text(
                x0 + s.x + 5.0,
                base,
                &s.text,
                PILL_SIZE,
                true,
                tone.text(),
                0.0,
            );
        } else {
            let c = if s.bold { bold_color } else { color };
            f.text(x0 + s.x, base, &s.text, size, s.bold, c, 0.0);
        }
    }
}

/// The band as a fragment: the bar, then the facts on a tint. `continued` adds " · continued".
pub(super) fn band_frag(b: &TableBand, total_w: f32, continued: bool) -> Frag {
    const TITLE: f32 = 13.0;
    const TITLE_LEAD: f32 = 15.6;
    const DATE: f32 = 8.4;
    let mut f = Frag::new();
    let right = b.right.as_deref().filter(|r| !r.trim().is_empty());
    let right_w = right.map_or(0.0, |r| tw(r, DATE, false, 0.34));
    let avail = (total_w - right_w - 3.0 * 10.0).max(80.0);
    let title = Txt::new(b.title.clone()).size(TITLE).bold();
    let lines = txt_lines(&title, avail);
    let suffix_w = text_width(BAND_SUFFIX, 7.4, false);
    let last_w = lines.last().map_or(0.0, |l| tw(l, TITLE, true, 0.0));
    let suffix_below = continued && last_w + suffix_w > avail;
    let n_lines = lines.len() + usize::from(suffix_below);
    let bar_h = (n_lines as f32 * TITLE_LEAD + 14.0).max(28.0);
    f.shape_c(
        0.0,
        0.0,
        total_w,
        bar_h,
        7.0,
        true,
        b.facts.is_empty(),
        Some(b.fill),
        None,
    );
    let first = (bar_h - n_lines as f32 * TITLE_LEAD) / 2.0 + 11.6;
    for (k, l) in lines.iter().enumerate() {
        f.text(
            10.0,
            first + k as f32 * TITLE_LEAD,
            l,
            TITLE,
            true,
            pal::WHITE,
            0.0,
        );
    }
    if continued {
        let (x, base) = if suffix_below {
            (10.0, first + lines.len() as f32 * TITLE_LEAD - 2.0)
        } else {
            (
                10.0 + last_w,
                first + (lines.len() as f32 - 1.0) * TITLE_LEAD,
            )
        };
        f.text(x, base, BAND_SUFFIX, 7.4, false, pal::DARK_MUTED, 0.0);
    }
    if let Some(r) = right {
        f.text(
            total_w - 10.0 - right_w,
            bar_h / 2.0 + DATE * 0.36,
            r,
            DATE,
            false,
            pal::WHITE,
            0.34,
        );
    }
    let mut y = bar_h;
    if !b.facts.is_empty() {
        let mut body = Frag::new();
        let mut by = FACT_PAD_Y;
        let inner = total_w - 2.0 * FACT_PAD_X;
        for line in &b.facts {
            match line {
                FactLine::Runs(runs) => {
                    for l in flow_runs(runs, inner, FACT_SIZE) {
                        put_segs(
                            &mut body,
                            FACT_PAD_X,
                            by + FACT_SIZE + 1.2,
                            &l,
                            FACT_SIZE,
                            (pal::INK, pal::INK),
                        );
                        by += FACT_LEAD;
                    }
                }
                FactLine::Labelled { label, text } => {
                    by += 2.0;
                    body.line(FACT_PAD_X, by, total_w - FACT_PAD_X, by, LINE_STRONG, 0.5);
                    by += 4.0;
                    let lab = label.to_uppercase();
                    let lw = tw(&lab, 6.6, true, 0.5) + 8.0;
                    let t = Txt::new(text.clone()).size(8.0).color(pal::SLATE);
                    body.text(FACT_PAD_X, by + 8.0, &lab, 6.6, true, pal::SLATE, 0.5);
                    let mut tmp = Frag::new();
                    let th = put_txt(&mut tmp, 0.0, 0.0, (inner - lw).max(60.0), &t);
                    body.put(tmp, FACT_PAD_X + lw, by);
                    by += th.max(FACT_LEAD) + 1.0;
                }
            }
        }
        by += FACT_PAD_Y - 1.0;
        f.shape_c(
            0.0,
            bar_h,
            total_w,
            by,
            0.0,
            false,
            false,
            Some(pal::TINT),
            None,
        );
        f.put(body, 0.0, bar_h);
        y += by;
    }
    f.h = y;
    f
}

pub(in crate::report) fn band_scan(b: &TableBand, scan: &mut dyn FnMut(&str)) {
    scan(&b.title);
    if let Some(r) = &b.right {
        scan(r);
    }
    for line in &b.facts {
        match line {
            FactLine::Runs(runs) => runs.iter().for_each(|r| scan(&r.text)),
            FactLine::Labelled { label, text } => {
                scan(label);
                scan(text);
            }
        }
    }
    scan(BAND_SUFFIX);
}
