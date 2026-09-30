//! Plain text (TXT files and pasted screenplay text): heuristic recognition
//! import and an indented plain-text writer.
//!
//! TXT has weak structural signals (Import/Export §4.8). We recognise scene
//! headings, upper-case cues, parentheticals and transitions, use leading
//! spaces as indentation when the text has screenplay-style indents, and
//! report confidence honestly. Nothing is guaranteed.

use crate::heuristics::{ClassifyOptions, RawLine, classify};
use crate::layout::{self, LayoutOptions};
use crate::model::{ScreenplayDoc, Warning};
use crate::{ImportOutcome, SourceFormat, not_a_screenplay};
use openframe_domain::AppResult;

/// Decode bytes as UTF-8 (BOM stripped), falling back to Windows-1252.
pub fn decode_text(bytes: &[u8]) -> (String, Option<Warning>) {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    if bytes.len() >= 2 && (bytes[..2] == [0xFF, 0xFE] || bytes[..2] == [0xFE, 0xFF]) {
        let le = bytes[0] == 0xFF;
        let units: Vec<u16> = bytes[2..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| {
                if le {
                    u16::from_le_bytes([c[0], c[1]])
                } else {
                    u16::from_be_bytes([c[0], c[1]])
                }
            })
            .collect();
        return (String::from_utf16_lossy(&units), None);
    }
    match std::str::from_utf8(bytes) {
        Ok(s) => (s.to_string(), None),
        Err(_) => {
            let s: String = bytes
                .iter()
                .map(|&b| crate::winansi::decode_byte(b))
                .collect();
            (
                s,
                Some(Warning::info(
                    "legacy_encoding",
                    "The text was not saved as UTF-8, so it was read with the Windows Western encoding. Check accented characters.",
                )),
            )
        }
    }
}

/// Split text into raw lines with indentation measured relative to the most
/// common left margin. Returns the lines and whether indentation looks like
/// screenplay layout.
pub fn raw_lines(src: &str) -> (Vec<RawLine>, bool) {
    let normalized = src.replace("\r\n", "\n").replace('\r', "\n");
    let mut out = Vec::new();
    let mut gap = false;
    let mut first = true;
    let mut new_page = false;
    let mut indents: Vec<usize> = Vec::new();
    for line in normalized.split('\n') {
        if line.contains('\u{000C}') {
            new_page = true;
        }
        let line = line.replace('\u{000C}', "");
        let expanded = line.replace('\t', "    ");
        if expanded.trim().is_empty() {
            gap = true;
            continue;
        }
        let mut lead = expanded.chars().take_while(|c| *c == ' ').count();
        let mut text = expanded.trim().to_string();
        // "12   INT. HOUSE - DAY   12": the heading's indentation is where the heading text starts.
        if let Some((num, rest)) = text.split_once("  ") {
            let rest_trim = rest.trim_start();
            if num
                .chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
                && num.len() <= 5
                && crate::heuristics::heading_match(rest_trim)
                    != crate::heuristics::HeadingMatch::No
            {
                lead +=
                    num.chars().count() + (rest.chars().count() - rest_trim.chars().count()) + 2;
                text = rest_trim.to_string();
            }
        }
        indents.push(lead);
        out.push(RawLine {
            text,
            indent: Some(lead as f32),
            gap_before: gap && !first,
            new_page,
            hint: None,
            dual: false,
        });
        gap = false;
        first = false;
        new_page = false;
    }
    // Action margin = most common indentation among long lines (action is the
    // widest element); fall back to the smallest indentation.
    let mut hist: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();
    for (l, ind) in out.iter().zip(&indents) {
        if l.text.chars().count() >= 20 {
            *hist.entry(*ind).or_default() += 1;
        }
    }
    let min = indents.iter().copied().min().unwrap_or(0);
    let margin = hist
        .iter()
        .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))
        .map(|(k, _)| *k)
        .unwrap_or(min);
    let deep = indents.iter().filter(|&&i| i >= margin + 8).count();
    let screenplay_indents = !indents.is_empty() && deep * 10 >= indents.len() && deep >= 2;
    for l in &mut out {
        l.indent = if screenplay_indents {
            l.indent.map(|i| ((i - margin as f32) / 10.0).max(0.0))
        } else {
            None
        };
    }
    (out, screenplay_indents)
}

/// Parse TXT or pasted screenplay text.
pub fn parse(src: &str, format: SourceFormat) -> AppResult<ImportOutcome> {
    if src.trim().is_empty() {
        return Err(openframe_domain::AppError::import(
            "empty_source",
            "There is no text to import. Your current project was not changed.",
        ));
    }
    let (lines, indented) = raw_lines(src);
    let (mut doc, stats) = classify(
        &lines,
        ClassifyOptions {
            join_wrapped: indented,
            use_indentation: indented,
        },
    );
    // Plain text never carries explicit structure: never report more than medium.
    let confidence = stats
        .confidence()
        .capped(crate::heuristics::ConfidenceLevel::Medium, 0.79);
    if stats.strong_headings + stats.uncertain_headings == 0 && stats.cues == 0 {
        return Err(not_a_screenplay(format));
    }
    doc.warnings.insert(
        0,
        Warning::info(
            "heuristic_parse",
            "Plain text has no built-in screenplay structure. Scene headings, character names and dialogue were recognised from their layout — review the preview before importing.",
        ),
    );
    if stats.strong_headings == 0 {
        doc.warnings.push(Warning::attention(
            "no_scene_headings",
            "No INT./EXT. scene headings were found.",
        ));
    }
    Ok(ImportOutcome {
        doc,
        confidence,
        format,
    })
}

/// Plain-text screenplay: Courier-style space indentation, no page furniture.
/// Re-imports through [`parse`].
pub fn write(doc: &ScreenplayDoc, opts: &LayoutOptions) -> String {
    let mut out = String::new();
    // Plain text cannot hold side-by-side columns: dual dialogue is written one speaker after the other.
    let mut sequential = doc.clone();
    for s in &mut sequential.scenes {
        for e in &mut s.elements {
            e.dual = false;
        }
    }
    let pages = layout::layout(&sequential, opts);
    for page in &pages.pages {
        if page.title_page {
            // Plain text title page: centered-ish block followed by a form feed.
            for l in &page.lines {
                if l.text.is_empty() {
                    out.push('\n');
                    continue;
                }
                let col = ((l.x_in - layout::LEFT_MARGIN_IN) * 10.0).round().max(0.0) as usize;
                out.push_str(&" ".repeat(col));
                out.push_str(&l.text);
                out.push('\n');
            }
            out.push('\u{000C}');
            out.push('\n');
            continue;
        }
        let mut last_row: i32 = -1;
        for l in page.lines.iter().filter(|l| {
            l.style != layout::LineStyle::PageNumber
                && l.style != layout::LineStyle::SceneNumber
                && l.style != layout::LineStyle::RevisionMark
        }) {
            let row = l.row;
            if last_row >= 0 {
                for _ in 0..(row - last_row - 1).max(0) {
                    out.push('\n');
                }
            }
            if row == last_row {
                // Same row (dual dialogue right column): append after padding.
                let current_len = out
                    .rsplit('\n')
                    .next()
                    .map(|s| s.chars().count())
                    .unwrap_or(0);
                let col = ((l.x_in - layout::LEFT_MARGIN_IN) * 10.0).round().max(0.0) as usize;
                out.push_str(&" ".repeat(col.saturating_sub(current_len).max(1)));
                out.push_str(&l.text);
                continue;
            }
            if last_row >= 0 {
                out.push('\n');
            }
            let col = ((l.x_in - layout::LEFT_MARGIN_IN) * 10.0).round().max(0.0) as usize;
            out.push_str(&" ".repeat(col));
            out.push_str(&l.text);
            last_row = row;
        }
        out.push_str("\n\n");
    }
    let trimmed = out.trim_end().to_string();
    trimmed + "\n"
}
