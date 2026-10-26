//! The Story page in a PDF: lines of formatted runs ([`crate::prose`]) wrapped to the text width.
//! Bold, italic, underline, a yellow highlight and a point size per run. Every line of the text is
//! one paragraph line, so blank lines keep their height and the page reads as it was written.

use super::*;
use crate::prose::{BODY_SIZE, ProseLine, ProseRun};

/// The marker colour behind highlighted text.
const HIGHLIGHT: Rgb = Rgb::hex(0xffe27a);
/// Line height as a multiple of the largest size on the row.
const LEADING: f32 = 1.35;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Style {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub highlight: bool,
    pub size: f32,
}

impl Style {
    fn of(r: &ProseRun) -> Style {
        Style {
            bold: r.b,
            italic: r.i,
            underline: r.u,
            highlight: r.h,
            size: r.size(),
        }
    }
}

/// A piece of one printed row: where it starts and what it prints.
#[derive(Debug, Clone)]
pub(super) struct Seg {
    pub x: f32,
    pub w: f32,
    pub text: String,
    pub st: Style,
}

/// One printed row and the largest size on it.
#[derive(Debug, Clone)]
pub(super) struct Row {
    pub segs: Vec<Seg>,
    pub size: f32,
}

impl Row {
    fn new() -> Row {
        Row {
            segs: Vec::new(),
            size: 0.0,
        }
    }
}

/// The text of a line split into chunks of one style that are either all spaces or no spaces.
/// Tabs print as four spaces.
fn chunks(line: &ProseLine) -> Vec<(String, Style, bool)> {
    let mut out: Vec<(String, Style, bool)> = Vec::new();
    for r in line {
        let st = Style::of(r);
        let text = r.t.replace('\t', "    ");
        let mut cur = String::new();
        let mut cur_space: Option<bool> = None;
        for ch in text.chars() {
            let sp = ch.is_whitespace();
            if let Some(prev) = cur_space
                && prev != sp
            {
                out.push((std::mem::take(&mut cur), st, prev));
            }
            cur_space = Some(sp);
            cur.push(ch);
        }
        if let Some(sp) = cur_space {
            out.push((cur, st, sp));
        }
    }
    out
}

fn width(text: &str, st: Style) -> f32 {
    text_width(text, st.size, st.bold)
}

fn push(row: &mut Row, x: f32, w: f32, text: String, st: Style) {
    row.size = row.size.max(st.size);
    // Same style right after the last piece: one string.
    if let Some(last) = row.segs.last_mut()
        && last.st == st
        && (last.x + last.w - x).abs() < 0.01
    {
        last.text.push_str(&text);
        last.w += w;
        return;
    }
    row.segs.push(Seg { x, w, text, st });
}

/// Break one line into rows no wider than `max`. Spaces at a break are dropped (the first row keeps
/// a leading indent); a word wider than a row is broken between characters, so nothing is cut.
pub(super) fn lay_out(line: &ProseLine, max: f32) -> Vec<Row> {
    let max = max.max(20.0);
    let mut rows: Vec<Row> = vec![Row::new()];
    let mut x = 0.0f32;
    let mut spaces: Vec<(String, Style)> = Vec::new();
    let all = chunks(line);
    let mut i = 0;
    while i < all.len() {
        if all[i].2 {
            spaces.push((all[i].0.clone(), all[i].1));
            i += 1;
            continue;
        }
        // One word: the chunks up to the next space, possibly in several styles.
        let mut word: Vec<(String, Style)> = Vec::new();
        while i < all.len() && !all[i].2 {
            word.push((all[i].0.clone(), all[i].1));
            i += 1;
        }
        let sw: f32 = spaces.iter().map(|(t, s)| width(t, *s)).sum();
        let ww: f32 = word.iter().map(|(t, s)| width(t, *s)).sum();
        let row_empty = rows.last().is_some_and(|r| r.segs.is_empty());
        if !row_empty && x + sw + ww > max {
            rows.push(Row::new());
            x = 0.0;
            spaces.clear();
        }
        for (t, st) in spaces.drain(..) {
            let w = width(&t, st);
            push(rows.last_mut().unwrap(), x, w, t, st);
            x += w;
        }
        if x + ww <= max {
            for (t, st) in word {
                let w = width(&t, st);
                push(rows.last_mut().unwrap(), x, w, t, st);
                x += w;
            }
            continue;
        }
        // Too long for the room left on any row: break between characters.
        for (t, st) in word {
            let mut piece = String::new();
            for ch in t.chars() {
                let next = format!("{piece}{ch}");
                if x + width(&next, st) > max && (!piece.is_empty() || x > 0.0) {
                    if !piece.is_empty() {
                        let w = width(&piece, st);
                        push(
                            rows.last_mut().unwrap(),
                            x,
                            w,
                            std::mem::take(&mut piece),
                            st,
                        );
                    }
                    rows.push(Row::new());
                    x = 0.0;
                }
                piece.push(ch);
            }
            if !piece.is_empty() {
                let w = width(&piece, st);
                push(rows.last_mut().unwrap(), x, w, piece, st);
                x += w;
            }
        }
    }
    // Spaces after the last word print only when the row holds nothing else (underlined blanks).
    if rows.len() == 1 && rows[0].segs.is_empty() {
        for (t, st) in spaces {
            let w = width(&t, st);
            push(&mut rows[0], x, w, t, st);
            x += w;
        }
    }
    let line_size = line.iter().map(ProseRun::size).fold(0.0, f32::max);
    for r in &mut rows {
        if r.size == 0.0 {
            r.size = if line_size > 0.0 {
                line_size
            } else {
                BODY_SIZE
            };
        }
    }
    rows
}

/// Print the lines at the current position, breaking pages between rows.
pub(super) fn render(p: &mut Pager, lines: &[ProseLine]) {
    let max = p.w - 2.0 * MARGIN;
    for line in lines {
        for row in lay_out(line, max) {
            let h = row.size * LEADING;
            p.ensure(h);
            p.y -= h;
            let base = p.y + row.size * (LEADING - 1.0);
            for s in &row.segs {
                let x = MARGIN + s.x;
                let size = s.st.size;
                if s.st.highlight {
                    p.ops().push(Op::Shape {
                        x,
                        y: base - size * 0.24,
                        w: s.w,
                        h: size * 1.08,
                        r: 0.0,
                        round_top: false,
                        round_bottom: false,
                        fill: Some(HIGHLIGHT),
                        stroke: None,
                    });
                }
                if !s.text.trim().is_empty() {
                    p.ops().push(Op::Styled {
                        bold: s.st.bold,
                        italic: s.st.italic,
                        size,
                        x,
                        y: base,
                        text: s.text.clone(),
                    });
                }
                if s.st.underline {
                    let y = base - size * 0.11;
                    p.ops().push(Op::LineC {
                        x1: x,
                        y1: y,
                        x2: x + s.w,
                        y2: y,
                        color: pal::INK,
                        width: (size * 0.055).max(0.5),
                    });
                }
            }
        }
    }
}

pub(super) fn scan(lines: &[ProseLine], scan: &mut dyn FnMut(&str)) {
    for r in lines.iter().flatten() {
        scan(&r.t);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(t: &str, b: bool, s: Option<f32>) -> ProseRun {
        ProseRun {
            t: t.into(),
            b,
            s,
            ..ProseRun::default()
        }
    }

    fn joined(rows: &[Row]) -> String {
        rows.iter()
            .map(|r| r.segs.iter().map(|s| s.text.as_str()).collect::<String>())
            .collect::<Vec<_>>()
            .join("|")
    }

    #[test]
    fn a_word_in_two_styles_stays_together_and_rows_fit() {
        let line = vec![
            run("The he", false, None),
            run("ro", true, None),
            run(" walks away in the age of timeless epics", false, None),
        ];
        let rows = lay_out(&line, 120.0);
        assert!(rows.len() > 1);
        assert!(joined(&rows).replace('|', " ").contains("hero"));
        for r in &rows {
            let last = r.segs.last().unwrap();
            assert!(last.x + last.w <= 120.01, "{rows:?}");
        }
        // A break drops the space: no row but the first starts with one.
        for r in &rows[1..] {
            assert!(!r.segs[0].text.starts_with(' '), "{rows:?}");
        }
    }

    #[test]
    fn the_row_height_follows_the_largest_size_and_empty_lines_keep_a_height() {
        let rows = lay_out(
            &vec![run("small ", false, None), run("BIG", false, Some(30.0))],
            400.0,
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].size, 30.0);
        let empty = lay_out(&Vec::new(), 400.0);
        assert_eq!(empty.len(), 1);
        assert_eq!(empty[0].size, BODY_SIZE);
    }

    #[test]
    fn an_overlong_word_is_broken_not_cut() {
        let long = "x".repeat(200);
        let rows = lay_out(&vec![run(&long, false, None)], 100.0);
        assert!(rows.len() > 2);
        assert_eq!(joined(&rows).replace('|', ""), long);
    }

    #[test]
    fn a_pdf_with_every_style_is_written() {
        let lines = vec![
            vec![
                run("Bold ", true, Some(18.0)),
                ProseRun {
                    t: "italic underlined".into(),
                    i: true,
                    u: true,
                    h: true,
                    ..ProseRun::default()
                },
            ],
            Vec::new(),
            vec![run(
                "\u{0D28}\u{0D2E}\u{0D38}\u{0D4D}\u{0D15}\u{0D3E}\u{0D30}\u{0D02} and more",
                false,
                None,
            )],
        ];
        let doc = Document::new("Story").with(Block::Prose(lines));
        let pdf = doc.to_pdf();
        let s = String::from_utf8_lossy(&pdf.bytes);
        assert!(
            s.contains("Helvetica-Oblique"),
            "the italic run uses the oblique face"
        );
        assert_eq!(pdf.pages, 1);
        assert_eq!(pdf.replaced_chars, 0);
    }
}
