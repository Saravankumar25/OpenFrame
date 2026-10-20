//! Small helpers shared by the production document layouts (the report design system lives in
//! `openframe_import_export::report`). Presentation only: nothing here reads or changes data.

use openframe_import_export::report::{Cell, CellItem, Chip, Masthead, Side, Tone, Txt, pal};

use crate::modules::schedule::fmt::parse_date;

/// "Needs input" chip item for a missing value.
pub(super) fn missing() -> CellItem {
    CellItem::Missing("Needs input".into())
}

/// A plain table cell with one line of text.
pub(super) fn cell(text: impl Into<String>) -> Cell {
    Cell::text(Txt::new(text).size(8.6).color(pal::INK))
}

pub(super) fn cell_bold(text: impl Into<String>) -> Cell {
    Cell::text(Txt::new(text).size(8.6).bold().color(pal::NAVY))
}

pub(super) fn cell_missing() -> Cell {
    Cell::new().item(missing())
}

/// A text cell, or the "needs input" chip when the value is blank.
pub(super) fn cell_or_missing(text: &str, bold: bool) -> Cell {
    if text.trim().is_empty() {
        cell_missing()
    } else if bold {
        cell_bold(text.trim())
    } else {
        cell(text.trim())
    }
}

/// Cast ids, scene numbers and other short lists: "1, 2 3" gives three entries.
pub(super) fn split_list(s: &str) -> Vec<String> {
    s.split([',', ';', ' '])
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .map(str::to_string)
        .collect()
}

/// Number pills for a list of ids or scene numbers.
pub(super) fn number_pills(items: &[String], tone: Tone) -> Vec<Chip> {
    items
        .iter()
        .map(|n| Chip::pill(n.clone(), tone).small().bold())
        .collect()
}

/// ("Monday", "19 October 2026") from a stored `YYYY-MM-DD` date.
pub(super) fn long_date(date: &str) -> Option<(String, String)> {
    let d = parse_date(date).ok()?;
    Some((
        d.weekday().to_string(),
        format!("{} {} {}", d.day(), d.month(), d.year()),
    ))
}

/// The digits of a day label ("SHOOT DAY 4" gives "04").
pub(super) fn day_number(label: &str) -> Option<String> {
    let digits: String = label
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        None
    } else if digits.len() == 1 {
        Some(format!("0{digits}"))
    } else {
        Some(digits)
    }
}

/// Non-blank trimmed text.
pub(super) fn nb(s: &str) -> Option<&str> {
    let t = s.trim();
    (!t.is_empty()).then_some(t)
}

/// ("MON", "19", "OCT") for a matrix header, from a stored `YYYY-MM-DD` date.
pub(super) fn short_day_parts(date: &str) -> Option<(String, String, String)> {
    let d = parse_date(date).ok()?;
    let abbr = |s: String| s.chars().take(3).collect::<String>().to_uppercase();
    Some((
        abbr(d.weekday().to_string()),
        d.day().to_string(),
        abbr(d.month().to_string()),
    ))
}

/// The masthead of the breakdown family: lockup and document label on top, then the title, the
/// project and source, and a small export reference panel.
pub(super) fn production_masthead(
    top_bar: &str,
    eyebrow: &str,
    title: &str,
    project: &str,
    source: &str,
    date_line: &str,
    scope: &str,
) -> Masthead {
    let mut m = Masthead::new(title).eyebrow(eyebrow, pal::ORANGE);
    m.top_bar = Some(top_bar.to_string());
    m.title_size = 30.0;
    let mut line = Vec::new();
    if let Some(p) = nb(project) {
        line.push(format!("Project: {p}"));
    }
    if let Some(s) = nb(source) {
        line.push(format!("Source: {s}"));
    }
    if !line.is_empty() {
        m.lines.push(line.join(" · "));
    }
    m.side = Some(Side::Panel {
        lines: vec![
            (date_line.to_uppercase(), true),
            ("Export reference".to_string(), false),
            (scope.to_string(), false),
        ],
    });
    m
}
