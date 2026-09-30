//! Best-effort screenplay recognition from PDF (Import/Export §4.6, FSD §19.4).
//!
//! PDF is a presentation format: it has positioned glyphs, not screenplay
//! elements. We extract text runs with their positions (lopdf content-stream
//! interpretation of the text operators), rebuild lines, strip page furniture
//! (page numbers, scene numbers in the margins, revision asterisks,
//! `(MORE)`/`(CONT'D)`), and classify each line by its indentation relative to
//! the action margin. Confidence is reported and never "certain"; scanned
//! PDFs (no text layer) are rejected with the standard message.

use std::collections::BTreeMap;

use lopdf::content::Content;
use lopdf::{Document, Object, ObjectId};
use openframe_domain::{AppError, AppResult};

use crate::heuristics::{
    ClassifyOptions, ConfidenceLevel, HeadingMatch, LineHint, RawLine, classify, heading_match,
};
use crate::model::{ElementKind, Warning};
use crate::{ImportOutcome, SourceFormat};

pub const MAX_PAGES: usize = 2_000;

fn unreadable(detail: impl Into<String>) -> AppError {
    AppError::import(
        "pdf_unreadable",
        "This PDF could not be read. It may be damaged or password-protected. Your current project was not changed.",
    )
    .with_detail(detail)
}

pub fn not_interpreted(detail: impl Into<String>) -> AppError {
    AppError::import(
        "pdf_not_screenplay",
        "The PDF could not be interpreted as a screenplay. Your current project was not changed.",
    )
    .with_detail(detail)
}

#[derive(Debug, Clone)]
struct Run {
    x: f32,
    y: f32,
    size: f32,
    text: String,
    /// Estimated advance width in points.
    width: f32,
}

type Matrix = [f32; 6];

fn mul(a: &Matrix, b: &Matrix) -> Matrix {
    [
        a[0] * b[0] + a[1] * b[2],
        a[0] * b[1] + a[1] * b[3],
        a[2] * b[0] + a[3] * b[2],
        a[2] * b[1] + a[3] * b[3],
        a[4] * b[0] + a[5] * b[2] + b[4],
        a[4] * b[1] + a[5] * b[3] + b[5],
    ]
}

const IDENTITY: Matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

fn num(o: &Object) -> f32 {
    o.as_float()
        .or_else(|_| o.as_i64().map(|i| i as f32))
        .unwrap_or(0.0)
}

struct FontInfo<'a> {
    encoding: Option<lopdf::Encoding<'a>>,
    monospace: bool,
}

fn decode(font: Option<&FontInfo<'_>>, bytes: &[u8]) -> String {
    if let Some(enc) = font.and_then(|f| f.encoding.as_ref())
        && let Ok(s) = Document::decode_text(enc, bytes)
    {
        return s;
    }
    bytes
        .iter()
        .map(|&b| crate::winansi::decode_byte(b))
        .collect()
}

fn page_runs(doc: &Document, page_id: ObjectId) -> Result<Vec<Run>, lopdf::Error> {
    let fonts_raw = doc.get_page_fonts(page_id).unwrap_or_default();
    let mut fonts: BTreeMap<Vec<u8>, FontInfo<'_>> = BTreeMap::new();
    for (name, dict) in fonts_raw {
        let base = dict
            .get(b"BaseFont")
            .and_then(Object::as_name)
            .map(|n| String::from_utf8_lossy(n).to_lowercase())
            .unwrap_or_default();
        fonts.insert(
            name,
            FontInfo {
                encoding: dict.get_font_encoding(doc).ok(),
                monospace: base.contains("courier") || base.contains("mono"),
            },
        );
    }
    let data = doc.get_page_content(page_id)?;
    let content = Content::decode(&data)?;
    let mut runs = Vec::new();
    let mut ctm_stack: Vec<Matrix> = Vec::new();
    let mut ctm = IDENTITY;
    let mut tm = IDENTITY;
    let mut tlm = IDENTITY;
    let mut leading = 0.0f32;
    let mut size = 12.0f32;
    let mut font: Option<Vec<u8>> = None;
    let emit = |tm: &mut Matrix,
                ctm: &Matrix,
                text: String,
                size: f32,
                mono: bool,
                runs: &mut Vec<Run>| {
        let m = mul(tm, ctm);
        let scale = (m[0] * m[0] + m[1] * m[1]).sqrt().max(0.01);
        let n = text.chars().count() as f32;
        let adv = n * size * if mono { 0.6 } else { 0.5 };
        runs.push(Run {
            x: m[4],
            y: m[5],
            size: size * scale,
            width: adv * scale,
            text,
        });
        // Advance the text matrix so following runs on the same line keep their order.
        tm[4] += adv * tm[0];
        tm[5] += adv * tm[1];
    };
    for op in &content.operations {
        let o = &op.operands;
        match op.operator.as_str() {
            "q" => ctm_stack.push(ctm),
            "Q" => ctm = ctm_stack.pop().unwrap_or(IDENTITY),
            "cm" if o.len() == 6 => {
                let m = [
                    num(&o[0]),
                    num(&o[1]),
                    num(&o[2]),
                    num(&o[3]),
                    num(&o[4]),
                    num(&o[5]),
                ];
                ctm = mul(&m, &ctm);
            }
            "BT" => {
                tm = IDENTITY;
                tlm = IDENTITY;
            }
            "Tf" if o.len() == 2 => {
                font = o[0].as_name().ok().map(|n| n.to_vec());
                size = num(&o[1]);
            }
            "TL" if !o.is_empty() => leading = num(&o[0]),
            "Td" | "TD" if o.len() == 2 => {
                let (tx, ty) = (num(&o[0]), num(&o[1]));
                if op.operator == "TD" {
                    leading = -ty;
                }
                tlm = mul(&[1.0, 0.0, 0.0, 1.0, tx, ty], &tlm);
                tm = tlm;
            }
            "Tm" if o.len() == 6 => {
                tlm = [
                    num(&o[0]),
                    num(&o[1]),
                    num(&o[2]),
                    num(&o[3]),
                    num(&o[4]),
                    num(&o[5]),
                ];
                tm = tlm;
            }
            "T*" => {
                tlm = mul(&[1.0, 0.0, 0.0, 1.0, 0.0, -leading], &tlm);
                tm = tlm;
            }
            "Tj" | "'" | "\"" => {
                if op.operator != "Tj" {
                    tlm = mul(&[1.0, 0.0, 0.0, 1.0, 0.0, -leading], &tlm);
                    tm = tlm;
                }
                let s = o.last().and_then(|x| x.as_str().ok()).unwrap_or(&[]);
                let f = font.as_ref().and_then(|n| fonts.get(n));
                let text = decode(f, s);
                if !text.is_empty() {
                    emit(
                        &mut tm,
                        &ctm,
                        text,
                        size,
                        f.map(|f| f.monospace).unwrap_or(true),
                        &mut runs,
                    );
                }
            }
            "TJ" if !o.is_empty() => {
                let f = font.as_ref().and_then(|n| fonts.get(n));
                let mut text = String::new();
                if let Ok(arr) = o[0].as_array() {
                    for item in arr {
                        match item {
                            Object::String(b, _) => text.push_str(&decode(f, b)),
                            other => {
                                // Large negative kerning is a word gap.
                                if num(other) < -200.0 && !text.ends_with(' ') {
                                    text.push(' ');
                                }
                            }
                        }
                    }
                }
                if !text.is_empty() {
                    emit(
                        &mut tm,
                        &ctm,
                        text,
                        size,
                        f.map(|f| f.monospace).unwrap_or(true),
                        &mut runs,
                    );
                }
            }
            _ => {}
        }
    }
    Ok(runs)
}

#[derive(Debug, Clone)]
struct Line {
    page: usize,
    y: f32,
    /// Runs sorted by x.
    runs: Vec<Run>,
}

impl Line {
    fn x(&self) -> f32 {
        self.runs.first().map(|r| r.x).unwrap_or(0.0)
    }
    fn text(&self) -> String {
        join_runs(&self.runs)
    }
}

fn join_runs(runs: &[Run]) -> String {
    let mut s = String::new();
    let mut end: Option<f32> = None;
    for r in runs {
        if let Some(e) = end
            && r.x - e > r.size * 0.25
            && !s.ends_with(' ')
            && !r.text.starts_with(' ')
        {
            s.push(' ');
        }
        s.push_str(&r.text);
        end = Some(r.x + r.width);
    }
    s.trim().to_string()
}

fn group_lines(page: usize, mut runs: Vec<Run>) -> Vec<Line> {
    runs.retain(|r| !r.text.trim().is_empty());
    runs.sort_by(|a, b| b.y.total_cmp(&a.y).then(a.x.total_cmp(&b.x)));
    let mut lines: Vec<Line> = Vec::new();
    for r in runs {
        match lines.last_mut() {
            Some(l) if (l.y - r.y).abs() < (r.size * 0.3).max(1.5) => l.runs.push(r),
            _ => lines.push(Line {
                page,
                y: r.y,
                runs: vec![r],
            }),
        }
    }
    for l in &mut lines {
        l.runs.sort_by(|a, b| a.x.total_cmp(&b.x));
    }
    lines
}

fn is_number_token(t: &str) -> bool {
    let t = t.trim().trim_end_matches('.').trim_matches('*');
    !t.is_empty()
        && t.len() <= 6
        && t.chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
        && t.chars().all(|c| c.is_ascii_alphanumeric())
}

pub fn parse(bytes: &[u8]) -> AppResult<ImportOutcome> {
    let head = &bytes[..bytes.len().min(1024)];
    if !head.windows(5).any(|w| w == b"%PDF-") {
        return Err(unreadable("missing %PDF header"));
    }
    let loaded = std::panic::catch_unwind(|| Document::load_mem(bytes));
    let doc = match loaded {
        Ok(Ok(d)) => d,
        Ok(Err(e)) => return Err(unreadable(e.to_string())),
        Err(_) => return Err(unreadable("parser panic")),
    };
    // lopdf opens documents protected with an empty user password and fails
    // (handled above) for anything that needs a real password.
    let pages = doc.get_pages();
    if pages.len() > MAX_PAGES {
        return Err(AppError::import(
            "too_large",
            format!(
                "This PDF has more than {MAX_PAGES} pages, which is too long for a screenplay. Your current project was not changed."
            ),
        ));
    }
    let extracted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut all: Vec<Line> = Vec::new();
        let mut heights = Vec::new();
        for (i, (_, id)) in pages.iter().enumerate() {
            let runs = page_runs(&doc, *id).unwrap_or_default();
            all.extend(group_lines(i, runs));
            let h = doc
                .get_dictionary(*id)
                .ok()
                .and_then(|d| {
                    d.get(b"MediaBox")
                        .ok()
                        .and_then(|m| m.as_array().ok())
                        .and_then(|a| a.get(3).map(num))
                })
                .unwrap_or(792.0);
            heights.push(h);
        }
        (all, heights)
    }));
    let (mut lines, heights) = extracted.map_err(|_| unreadable("text extraction panic"))?;
    if lines.iter().all(|l| l.text().trim().is_empty()) {
        return Err(not_interpreted("no text layer (scanned image?)"));
    }

    // Action margin: most common start x among long lines.
    let mut hist: BTreeMap<i32, usize> = BTreeMap::new();
    for l in &lines {
        if l.text().chars().count() >= 30 {
            *hist.entry((l.x() / 3.6).round() as i32).or_default() += 1;
        }
    }
    let margin = hist
        .iter()
        .max_by_key(|(_, c)| **c)
        .map(|(k, _)| *k as f32 * 3.6)
        .unwrap_or_else(|| lines.iter().map(|l| l.x()).fold(f32::MAX, f32::min));

    // Strip page furniture.
    let mut warnings = Vec::new();
    let mut revision_marks = 0usize;
    for l in &mut lines {
        let page_h = heights.get(l.page).copied().unwrap_or(792.0);
        // Page number: a lone number in the top 0.85".
        if l.y > page_h - 61.0 && l.runs.len() == 1 && is_number_token(&l.runs[0].text) {
            l.runs.clear();
            continue;
        }
        let before = l.runs.len();
        l.runs.retain(|r| {
            let t = r.text.trim();
            let left_margin_number = r.x < margin - 18.0 && is_number_token(t);
            let right_margin = r.x > 7.2 * 72.0 && (is_number_token(t) || t == "*");
            !(left_margin_number || right_margin)
        });
        if l.runs.len() < before {
            revision_marks += 1;
        }
        if l.text().to_uppercase().starts_with("CONTINUED")
            || l.text().eq_ignore_ascii_case("(CONTINUED)")
        {
            l.runs.clear();
        }
    }
    let _ = revision_marks;
    lines.retain(|l| !l.runs.is_empty());

    // Title page: first page without a scene heading, followed by more pages.
    let page_count = pages.len();
    let mut title_page = None;
    let first_page_lines: Vec<&Line> = lines.iter().filter(|l| l.page == 0).collect();
    if page_count > 1
        && !first_page_lines.is_empty()
        && first_page_lines.len() <= 30
        && !first_page_lines
            .iter()
            .any(|l| heading_match(&l.text()) == HeadingMatch::Strong)
    {
        let page_w = 612.0;
        let mut tl: Vec<(String, String)> = Vec::new();
        let mut prev_y: Option<f32> = None;
        for l in &first_page_lines {
            // Vertical space separates title-page blocks (title / credit / authors).
            if prev_y
                .map(|py| py - l.y > l.runs[0].size * 1.5)
                .unwrap_or(false)
            {
                tl.push(("Left".to_string(), String::new()));
            }
            prev_y = Some(l.y);
            tl.push({
                let l = *l;
                let t = l.text();
                let w: f32 = l.runs.iter().map(|r| r.width).sum();
                let start = l.x();
                let center = start + w / 2.0;
                let align = if (center - page_w / 2.0).abs() < 36.0 {
                    "Center"
                } else if start > page_w / 2.0 {
                    "Right"
                } else {
                    "Left"
                };
                (align.to_string(), t)
            });
        }
        title_page = Some(crate::fdx::title_page_from_lines(&tl));
        lines.retain(|l| l.page != 0);
    }

    // Typical line spacing for blank-line detection.
    let mut diffs: Vec<f32> = lines
        .windows(2)
        .filter(|w| w[0].page == w[1].page)
        .map(|w| w[0].y - w[1].y)
        .filter(|d| *d > 0.5)
        .collect();
    diffs.sort_by(|a, b| a.total_cmp(b));
    let spacing = diffs.get(diffs.len() / 4).copied().unwrap_or(12.0).max(4.0);

    // Build raw lines; detect dual-dialogue rows (two columns on one baseline).
    let mut raw: Vec<RawLine> = Vec::new();
    let mut prev: Option<&Line> = None;
    let mut in_dual = false;
    let mut dual_left: Vec<RawLine> = Vec::new();
    let mut dual_right: Vec<RawLine> = Vec::new();
    let flush_dual = |raw: &mut Vec<RawLine>, left: &mut Vec<RawLine>, right: &mut Vec<RawLine>| {
        for col in [std::mem::take(left), std::mem::take(right)] {
            for (i, mut l) in col.into_iter().enumerate() {
                let kind = if i == 0 {
                    ElementKind::Character
                } else if l.text.starts_with('(') {
                    ElementKind::Parenthetical
                } else {
                    ElementKind::Dialogue
                };
                l.hint = Some(LineHint::Kind(kind));
                l.dual = true;
                l.gap_before = i == 0;
                raw.push(l);
            }
        }
    };
    for l in &lines {
        let gap = match prev {
            Some(p) if p.page == l.page => p.y - l.y > spacing * 1.5,
            Some(_) => false,
            None => false,
        };
        let new_page = prev.map(|p| p.page != l.page).unwrap_or(true);
        // Split runs into columns where there is a wide gap.
        let mut cols: Vec<Vec<Run>> = vec![vec![]];
        let mut end: Option<f32> = None;
        for r in &l.runs {
            if let Some(e) = end
                && r.x - e > 0.8 * 72.0
            {
                cols.push(vec![]);
            }
            end = Some(r.x + r.width);
            cols.last_mut().unwrap().push(r.clone());
        }
        let right_col_x = margin + 2.9 * 72.0;
        let is_two_col = cols.len() == 2
            && cols[0][0].x < right_col_x
            && cols[1][0].x >= right_col_x
            && cols[1][0].x < 7.0 * 72.0;
        if is_two_col && (!in_dual || gap || new_page) {
            if in_dual {
                flush_dual(&mut raw, &mut dual_left, &mut dual_right);
            }
            in_dual = true;
        } else if in_dual && (gap || new_page) {
            flush_dual(&mut raw, &mut dual_left, &mut dual_right);
            in_dual = false;
        }
        if in_dual {
            for c in cols {
                let x = c[0].x;
                let t = join_runs(&c);
                let rl = RawLine {
                    text: t,
                    indent: None,
                    gap_before: false,
                    new_page: false,
                    hint: None,
                    dual: true,
                };
                if x >= right_col_x {
                    dual_right.push(rl)
                } else {
                    dual_left.push(rl)
                }
            }
        } else {
            raw.push(RawLine {
                text: l.text(),
                indent: Some(((l.x() - margin) / 72.0).max(0.0)),
                gap_before: gap,
                new_page,
                hint: None,
                dual: false,
            });
        }
        prev = Some(l);
    }
    if in_dual {
        flush_dual(&mut raw, &mut dual_left, &mut dual_right);
    }

    let (mut doc, stats) = classify(
        &raw,
        ClassifyOptions {
            join_wrapped: true,
            use_indentation: true,
        },
    );
    if stats.strong_headings + stats.uncertain_headings == 0 && stats.cues == 0 {
        return Err(not_interpreted("no screenplay structure recognised"));
    }
    if let Some(tp) = title_page {
        doc.title_page = tp;
    }
    let confidence = stats.confidence().capped(ConfidenceLevel::Medium, 0.79);
    warnings.push(Warning::info(
        "pdf_best_effort",
        "PDF files only describe how pages look, not which text is dialogue or action. OpenFrame recognised the structure from the page layout — review the result.",
    ));
    if confidence.level == ConfidenceLevel::Low {
        warnings.push(Warning::attention(
            "pdf_low_confidence",
            "This PDF was interpreted with limited certainty. Review the scenes below before importing.",
        ));
    }
    doc.warnings.splice(0..0, warnings);
    Ok(ImportOutcome {
        doc,
        confidence,
        format: SourceFormat::Pdf,
    })
}
