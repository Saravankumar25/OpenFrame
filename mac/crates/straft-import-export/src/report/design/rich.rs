//! Paragraphs made of runs with their own weight and colour: a bold label followed by its text
//! ("Sound: wild wind"), wrapped as one paragraph so the label never sits alone at a line end.

use super::*;

/// A stretch of text in one weight and colour.
#[derive(Debug, Clone)]
pub struct Run {
    pub text: String,
    pub bold: bool,
    pub color: Rgb,
}

impl Run {
    pub fn new(text: impl Into<String>, bold: bool, color: Rgb) -> Run {
        Run {
            text: text.into(),
            bold,
            color,
        }
    }
}

/// Paragraphs of runs at one size, stacked with `gap` points between them.
#[derive(Debug, Clone)]
pub struct Rich {
    pub paras: Vec<Vec<Run>>,
    pub size: f32,
    pub leading: f32,
    pub gap: f32,
}

impl Rich {
    pub fn new(size: f32) -> Rich {
        Rich {
            paras: Vec::new(),
            size,
            leading: 1.3,
            gap: 2.0,
        }
    }
    pub fn para(mut self, runs: Vec<Run>) -> Rich {
        self.paras.push(runs);
        self
    }
    pub fn leading(mut self, l: f32) -> Rich {
        self.leading = l;
        self
    }
    pub fn gap(mut self, g: f32) -> Rich {
        self.gap = g;
        self
    }
}

/// A piece of one line: where it starts and how it is drawn.
struct Seg {
    x: f32,
    text: String,
    bold: bool,
    color: Rgb,
}

/// One paragraph broken into lines no wider than `width`. A word wider than the line is broken
/// by characters; nothing is cut.
fn lay_out(runs: &[Run], width: f32, size: f32) -> Vec<Vec<Seg>> {
    let width = width.max(8.0);
    let mut lines: Vec<Vec<Seg>> = vec![Vec::new()];
    let mut x = 0.0f32;
    for run in runs {
        let space = text_width(" ", size, run.bold);
        for word in run.text.split_whitespace() {
            let ww = text_width(word, size, run.bold);
            let first = lines.last().is_some_and(|l| l.is_empty());
            if !first && x + space + ww > width {
                lines.push(Vec::new());
                x = 0.0;
            }
            let pieces: Vec<String> = if ww > width {
                wrap_pt(word, width, size, run.bold)
            } else {
                vec![word.to_string()]
            };
            for (k, piece) in pieces.iter().enumerate() {
                let pw = text_width(piece, size, run.bold);
                if k > 0 {
                    lines.push(Vec::new());
                    x = 0.0;
                }
                let line = lines.last_mut().unwrap();
                match line.last_mut() {
                    // Same style as the piece before it on this line: one string.
                    Some(prev) if prev.bold == run.bold && prev.color == run.color && k == 0 => {
                        prev.text.push(' ');
                        prev.text.push_str(piece);
                        x += space + pw;
                    }
                    Some(_) if k == 0 => {
                        line.push(Seg {
                            x: x + space,
                            text: piece.clone(),
                            bold: run.bold,
                            color: run.color,
                        });
                        x += space + pw;
                    }
                    _ => {
                        line.push(Seg {
                            x,
                            text: piece.clone(),
                            bold: run.bold,
                            color: run.color,
                        });
                        x += pw;
                    }
                }
            }
        }
    }
    if lines.len() > 1 && lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines
}

/// Height of the paragraphs in `width`.
pub(super) fn rich_height(r: &Rich, width: f32) -> f32 {
    let lh = r.size * r.leading;
    let mut h = 0.0;
    for (i, p) in r.paras.iter().enumerate() {
        if i > 0 {
            h += r.gap;
        }
        h += lay_out(p, width, r.size).len() as f32 * lh;
    }
    h
}

/// Draw the paragraphs with their top left corner at (x, dy); returns the height used.
pub(super) fn put_rich(f: &mut Frag, x: f32, dy: f32, width: f32, r: &Rich) -> f32 {
    let lh = r.size * r.leading;
    let mut y = 0.0f32;
    for (i, p) in r.paras.iter().enumerate() {
        if i > 0 {
            y += r.gap;
        }
        for line in lay_out(p, width, r.size) {
            for s in &line {
                f.text(
                    x + s.x,
                    dy + y + r.size,
                    &s.text,
                    r.size,
                    s.bold,
                    s.color,
                    0.0,
                );
            }
            y += lh;
        }
    }
    y
}

pub(super) fn rich_scan(r: &Rich, scan: &mut dyn FnMut(&str)) {
    for p in &r.paras {
        for run in p {
            scan(&run.text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runs(label: &str, text: &str) -> Vec<Run> {
        vec![
            Run::new(label, true, pal::NAVY),
            Run::new(text, false, pal::SLATE),
        ]
    }

    #[test]
    fn a_label_and_its_text_wrap_as_one_paragraph() {
        let lines = lay_out(
            &runs("Sound:", "wild wind and engines under the deck"),
            90.0,
            8.0,
        );
        assert!(lines.len() >= 2);
        // The label starts the first line and the text follows it on the same line.
        assert_eq!(lines[0][0].text, "Sound:");
        assert!(lines[0][1].x > text_width("Sound:", 8.0, true));
        // Words keep their order across the lines.
        let joined: Vec<String> = lines
            .iter()
            .map(|l| {
                l.iter()
                    .map(|s| s.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect();
        assert_eq!(
            joined.join(" "),
            "Sound: wild wind and engines under the deck"
        );
        for l in &lines {
            let last = l.last().unwrap();
            assert!(last.x + text_width(&last.text, 8.0, last.bold) <= 90.0 + 0.01);
        }
    }

    #[test]
    fn a_word_wider_than_the_line_is_broken_not_cut() {
        let long = "x".repeat(60);
        let lines = lay_out(&runs("Camera:", &long), 60.0, 8.0);
        let all: String = lines
            .iter()
            .flat_map(|l| l.iter().map(|s| s.text.clone()))
            .collect();
        assert_eq!(all.matches('x').count(), 60);
        assert!(lines.len() > 2);
    }

    #[test]
    fn height_counts_lines_and_gaps() {
        let r = Rich::new(8.0)
            .para(runs("A:", "one"))
            .para(runs("B:", "two"));
        let lh = 8.0 * 1.3;
        assert!((rich_height(&r, 200.0) - (2.0 * lh + 2.0)).abs() < 0.01);
    }
}
