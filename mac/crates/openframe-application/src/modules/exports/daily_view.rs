//! Daily View export (Production > Daily View, FSD 57). One shooting day as a printed page that
//! shows exactly what the screen shows: the title, the date, the scenes, the location, the cast,
//! the call sheet, the important notes, the important breakdown items and the scene order.
//!
//! The data comes from the same read as the screen (`schedule::ops::daily_view`), so the document
//! cannot drift from the page. Reproducible: the document carries the date of the schedule's last
//! change, never the export clock. The day is only read; nothing in the project changes.

use openframe_domain::{Actor, AppError, AppResult};
use openframe_import_export::report::{
    Align, BRAND_LINE, Block, Cell, CellItem, Chip, Col, Document, HeaderStyle, Masthead, Metric,
    Metrics, Rgb, Row, Section, Side, StyledTable, Tone, Txt, pal, wrap_pt,
};
use serde::Deserialize;
use ts_rs::TS;

use super::style::nb;
use super::{Fmt, Job, Output, date_from_ms, deliver, destination, plural, require_export};
use crate::core::AppCore;
use crate::modules::schedule::board::CallSheetBrief;
use crate::modules::schedule::board::{current_schedule, project_title, revised_at};
use crate::modules::schedule::fmt::minutes;
use crate::modules::schedule::ops::{ScheduleDailyDay, daily_view};

/// Secondary text that passes 4.5:1 on white (the palette's FAINT is decoration only).
const MUTED: Rgb = Rgb::hex(0x66728a);

// The empty-state sentences, exactly as the Daily View screen prints them. The plain text twin
// and the PDF both use them.
const NO_SCENES: &str = "No scenes scheduled.";
const NO_LOCATION: &str = "Location missing.";
const NO_CAST: &str = "No cast confirmed for these scenes.";
const NO_CALL_SHEET: &str = "No call sheet yet for this day.";
const OFF_DAY_CALL_SHEET: &str = "Off days have no call sheet.";
const NO_NOTES: &str = "No day notes.";
const NO_ITEMS: &str = "No confirmed props, wardrobe or other items for these scenes.";
const NO_ORDER: &str = "Nothing scheduled yet.";
const NO_CALL_TIMES: &str = "Not entered yet";

const PDF_HINT: &str =
    "Export it as plain text to keep them, or replace those characters and export the PDF again.";

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportDailyViewArgs {
    /// "pdf" | "txt"
    pub format: String,
    pub path: String,
    /// The day shown on the screen (None = the day the screen picks by default).
    #[serde(default)]
    #[ts(optional)]
    pub day_id: Option<String>,
}

/// One block of a section, written once and printed as PDF and as plain text.
enum Part {
    /// Label and value pairs.
    Facts(Vec<(String, String)>),
    /// A wrapped line of text.
    Line(String),
    /// A numbered list.
    Numbered(Vec<String>),
}

fn facts(rows: &[(&str, String)]) -> Part {
    Part::Facts(
        rows.iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect(),
    )
}

/// "Final", "Needs refresh" or "Draft", and the sentence the screen prints under it.
fn call_sheet_state(cs: &CallSheetBrief) -> (&str, &'static str) {
    let word = match cs.status.as_str() {
        "Final" | "Needs refresh" => cs.status.as_str(),
        _ => "Draft",
    };
    let line = if cs.source_changed {
        "Available. The schedule changed after it was created."
    } else if word == "Final" {
        "Finalized and matches the schedule."
    } else {
        "Available and up to date with the schedule."
    };
    (word, line)
}

/// The day's breaks as the screen's chips read: "LUNCH at 13:00 (1h)".
fn marker_texts(d: &ScheduleDailyDay) -> Vec<String> {
    d.markers
        .iter()
        .map(|m| {
            let mut t = m.label.to_uppercase();
            if let Some(at) = &m.at_time {
                t.push_str(&format!(" at {at}"));
            }
            if let Some(mins) = m.duration_minutes.filter(|x| *x > 0) {
                t.push_str(&format!(" ({})", minutes(mins)));
            }
            t
        })
        .collect()
}

/// The sections in the order of the screen. The wording matches `daily.tsx`.
fn sections(d: &ScheduleDailyDay) -> Vec<(&'static str, Vec<Part>)> {
    let s = &d.summary;
    let number = |n: Option<i64>| n.map_or("Not numbered".to_string(), |n| n.to_string());

    let scenes = if d.scenes.is_empty() {
        vec![Part::Line(NO_SCENES.into())]
    } else {
        let mut total = plural(s.scene_count as usize, "scene", "scenes");
        if s.estimated_minutes > 0 {
            total.push_str(&format!(" · {} estimated", minutes(s.estimated_minutes)));
        }
        if s.missing_estimates > 0 {
            total.push_str(&format!(" · {} without estimate", s.missing_estimates));
        }
        let numbers: Vec<String> = d.scenes.iter().map(|x| number(x.number)).collect();
        vec![facts(&[("Scenes", numbers.join(", ")), ("Total", total)])]
    };

    let location = if d.locations.is_empty() {
        vec![Part::Line(NO_LOCATION.into())]
    } else {
        let mut rows: Vec<(String, String)> = Vec::new();
        for l in &d.locations {
            rows.push(("Location".into(), l.name.clone()));
            let detail: Vec<String> = [
                l.status.clone(),
                Some(l.address.clone().unwrap_or("Address missing".into())),
            ]
            .into_iter()
            .flatten()
            .collect();
            rows.push(("Details".into(), detail.join(" · ")));
        }
        vec![Part::Facts(rows)]
    };

    let cast = if d.cast.is_empty() {
        vec![Part::Line(NO_CAST.into())]
    } else {
        let names: Vec<String> = d
            .cast
            .iter()
            .map(|c| c.actor.clone().unwrap_or_else(|| c.character.clone()))
            .collect();
        let calls: Vec<&str> = d
            .cast
            .iter()
            .filter_map(|c| c.call_time.as_deref())
            .collect();
        let mut rows = vec![
            ("Cast", names.join(" · ")),
            (
                "Call times",
                if calls.is_empty() {
                    NO_CALL_TIMES.to_string()
                } else {
                    calls.join(" / ")
                },
            ),
        ];
        if let Some(c) = &d.crew_call {
            rows.push(("Crew call", c.clone()));
        }
        vec![facts(&rows)]
    };

    let call_sheet = match &d.call_sheet {
        Some(cs) => {
            let (word, line) = call_sheet_state(cs);
            vec![facts(&[
                ("Status", word.to_string()),
                ("Details", line.into()),
            ])]
        }
        None if d.is_off_day => vec![Part::Line(OFF_DAY_CALL_SHEET.into())],
        None => vec![Part::Line(NO_CALL_SHEET.into())],
    };

    let mut notes = Vec::new();
    match d.notes.as_deref().and_then(nb) {
        Some(n) => notes.extend(n.lines().map(|l| Part::Line(l.to_string()))),
        None => notes.push(Part::Line(NO_NOTES.into())),
    }
    let marks = marker_texts(d);
    if !marks.is_empty() {
        notes.push(Part::Line(marks.join(" · ")));
    }

    let items = if d.breakdown_items.is_empty() {
        vec![Part::Line(NO_ITEMS.into())]
    } else {
        let names: Vec<&str> = d.breakdown_items.iter().map(|i| i.name.as_str()).collect();
        vec![Part::Line(names.join(" · "))]
    };

    let order = if d.scenes.is_empty() {
        vec![Part::Line(NO_ORDER.into())]
    } else {
        vec![Part::Numbered(
            d.scenes
                .iter()
                .map(|x| {
                    let est = x
                        .estimated_minutes
                        .map(|m| format!(" ({})", minutes(m)))
                        .unwrap_or_default();
                    format!("{}: {}{est}", number(x.number), x.heading)
                })
                .collect(),
        )]
    };

    vec![
        ("Scenes", scenes),
        ("Location", location),
        ("Cast", cast),
        ("Call Sheet", call_sheet),
        ("Important Notes", notes),
        ("Important Breakdown Items", items),
        ("Scene Order", order),
    ]
}

/// Plain text of the document: the title, the date and every section.
fn text_of(title: &str, date: &str, secs: &[(&'static str, Vec<Part>)]) -> String {
    let mut t = format!("{title}\n{date}\n");
    for (name, parts) in secs {
        t.push_str(&format!("\n{}\n", name.to_uppercase()));
        for p in parts {
            match p {
                Part::Facts(rows) => {
                    for (k, v) in rows {
                        t.push_str(&format!("{k}: {v}\n"));
                    }
                }
                Part::Line(l) => t.push_str(&format!("{l}\n")),
                Part::Numbered(items) => {
                    for (i, it) in items.iter().enumerate() {
                        t.push_str(&format!("{}. {it}\n", i + 1));
                    }
                }
            }
        }
    }
    t
}

/// A paragraph of text in a table cell.
fn line(t: impl Into<String>) -> CellItem {
    CellItem::Text(Txt::new(t).size(8.8).color(pal::INK).leading(1.3))
}

/// What a section prints, one entry per line of content. Side by side, the entries share one
/// table cell; stacked, each is a table row of its own, so a long section can break between
/// rows instead of running past the page (a row never splits).
type Rows = Vec<Vec<CellItem>>;

fn merged(rows: Rows) -> Cell {
    rows.into_iter()
        .flatten()
        .fold(Cell::new(), |cell, item| cell.item(item))
}

/// A table of two sections side by side, their names in a navy header.
fn two_up(left: (&str, Rows), right: (&str, Rows)) -> Block {
    let mut t = StyledTable::new(vec![Col::weight(left.0, 1.0), Col::weight(right.0, 1.0)]);
    t.header = HeaderStyle::Navy;
    t.boxed = true;
    t.grid = true;
    t.rows.push(Row::new(vec![merged(left.1), merged(right.1)]));
    Block::Styled(t)
}

/// A section on its own: its name as the table title and one row per entry.
fn stacked(name: &str, rows: Rows) -> Block {
    let mut t = StyledTable::new(vec![Col::weight(name, 1.0)]);
    t.header = HeaderStyle::None;
    t.boxed = true;
    t.title = Some(name.into());
    t.rows = rows
        .into_iter()
        .map(|items| Row::new(vec![items.into_iter().fold(Cell::new(), Cell::item)]))
        .collect();
    Block::Styled(t)
}

/// A section that has nothing to show prints its name and the sentence of the screen.
fn empty_section(name: &str, sentence: &str) -> [Block; 2] {
    [
        Block::Section(Section::new(name, None)),
        Block::Text(Txt::new(sentence).size(8.8).color(MUTED).after(6.0)),
    ]
}

/// Pills in rows of at most `n`, so a long list breaks between rows.
fn pill_rows(chips: Vec<Chip>, n: usize) -> Rows {
    let mut out = Vec::new();
    let mut rest = chips;
    while !rest.is_empty() {
        let tail = rest.split_off(n.min(rest.len()));
        out.push(vec![CellItem::Chips(std::mem::replace(&mut rest, tail))]);
    }
    out
}

/// The body of the page: what the Daily View screen shows, in the order of the screen, with the
/// short sections side by side and the lists as tables.
fn day_blocks(d: &ScheduleDailyDay) -> Vec<Block> {
    let s = &d.summary;
    let number = |n: Option<i64>| n.map_or("Not numbered".to_string(), |n| n.to_string());
    let mut out: Vec<Block> = Vec::new();

    // Scenes
    let scenes: Rows = if d.scenes.is_empty() {
        vec![vec![line(NO_SCENES)]]
    } else {
        let tiles = d
            .scenes
            .iter()
            .map(|x| Chip::pill(number(x.number), Tone::Info).bold())
            .collect();
        let mut total = plural(s.scene_count as usize, "scene", "scenes");
        if s.estimated_minutes > 0 {
            total.push_str(&format!(
                " \u{b7} {} estimated",
                minutes(s.estimated_minutes)
            ));
        }
        if s.missing_estimates > 0 {
            total.push_str(&format!(" \u{b7} {} without estimate", s.missing_estimates));
        }
        let mut rows = pill_rows(tiles, 12);
        rows.push(vec![CellItem::Text(
            Txt::new(format!("Total \u{b7} {total}"))
                .size(8.0)
                .color(pal::SLATE),
        )]);
        rows
    };
    // Location
    let mut location: Rows = Vec::new();
    if d.locations.is_empty() {
        location.push(vec![CellItem::Missing(NO_LOCATION.into())]);
    }
    for l in &d.locations {
        let mut items = vec![CellItem::Text(
            Txt::new(l.name.clone()).size(9.6).bold().color(pal::INK),
        )];
        let status = l.status.as_deref().and_then(nb);
        match l.address.as_deref().and_then(nb) {
            Some(a) => {
                let detail = [status, Some(a)].into_iter().flatten().collect::<Vec<_>>();
                items.push(CellItem::Text(
                    Txt::new(detail.join(" \u{b7} "))
                        .size(8.0)
                        .color(pal::SLATE),
                ));
            }
            None => {
                if let Some(st) = status {
                    items.push(CellItem::Text(Txt::new(st).size(8.0).color(pal::SLATE)));
                }
                items.push(CellItem::Missing("Address missing".into()));
            }
        }
        location.push(items);
    }
    // Notes: the day note keeps its line breaks, the breaks of the day follow as pills.
    let mut notes: Rows = match d.notes.as_deref().and_then(nb) {
        Some(n) => n.lines().map(|l| vec![line(l)]).collect(),
        None => vec![vec![line(NO_NOTES)]],
    };
    let marks = marker_texts(d);
    let mark_count = marks.len();
    notes.extend(pill_rows(
        marks
            .into_iter()
            .map(|m| Chip::pill(m, Tone::Neutral).bold())
            .collect(),
        3,
    ));
    // Call sheet
    let call_sheet: Rows = match &d.call_sheet {
        Some(cs) => {
            let (word, sentence) = call_sheet_state(cs);
            let tone = match word {
                "Final" => Tone::Ok,
                "Needs refresh" => Tone::Warn,
                _ => Tone::Neutral,
            };
            vec![vec![
                CellItem::Chips(vec![Chip::label(word, tone)]),
                line(sentence),
            ]]
        }
        None if d.is_off_day => vec![vec![line(OFF_DAY_CALL_SHEET)]],
        None => vec![vec![line(NO_CALL_SHEET)]],
    };

    // Side by side while each half is short enough to stay on one page; a day with very many
    // locations, a long note or many breaks stacks them so they can flow across pages.
    let note_lines: usize = d.notes.as_deref().and_then(nb).map_or(1, |n| {
        n.lines().map(|l| wrap_pt(l, 240.0, 8.8, false).len()).sum()
    });
    let long = d.locations.len() > 8 || d.scenes.len() > 60 || note_lines > 14 || mark_count > 9;
    if long {
        out.push(stacked("Scenes", scenes));
        out.push(stacked("Location", location));
    } else {
        out.push(two_up(("Scenes", scenes), ("Location", location)));
    }

    // Cast: each call time beside its person
    if d.cast.is_empty() {
        out.extend(empty_section("Cast", NO_CAST));
    } else {
        let any_call = d.cast.iter().any(|c| c.call_time.is_some());
        let mut cols = vec![Col::weight("Cast", 1.0), Col::weight("Character", 1.0)];
        if any_call {
            cols.push(Col::pt("Call", 64.0).align(Align::Right));
        }
        let mut t = StyledTable::new(cols);
        t.header = HeaderStyle::Navy;
        t.boxed = true;
        t.zebra = true;
        t.title = Some("Cast".into());
        t.pad_x = 6.0;
        t.pad_y = 5.0;
        for c in &d.cast {
            // A blank actor name counts as no actor: the character takes the first column.
            let actor = c.actor.as_deref().and_then(nb);
            let mut cells = vec![
                Cell::text(
                    Txt::new(actor.unwrap_or(c.character.as_str()))
                        .size(8.6)
                        .bold()
                        .color(pal::NAVY),
                ),
                match actor {
                    Some(_) => Cell::text(Txt::new(c.character.clone()).size(8.6).color(pal::INK)),
                    None => Cell::empty(),
                },
            ];
            if any_call {
                cells.push(
                    Cell::text(
                        Txt::new(c.call_time.clone().unwrap_or_else(|| "\u{2013}".into()))
                            .size(8.6)
                            .color(pal::INK),
                    )
                    .align(Align::Right),
                );
            }
            t.rows.push(Row::new(cells));
        }
        out.push(Block::Styled(t));
        let mut note = Vec::new();
        if !any_call {
            note.push(format!("Call times: {NO_CALL_TIMES}"));
        }
        if let Some(c) = &d.crew_call {
            note.push(format!("Crew call: {c}"));
        }
        if !note.is_empty() {
            out.push(Block::Text(
                Txt::new(note.join(" \u{b7} "))
                    .size(8.4)
                    .bold()
                    .color(pal::SLATE)
                    .after(6.0),
            ));
        }
    }

    if long {
        out.push(stacked("Call Sheet", call_sheet));
        out.push(stacked("Important Notes", notes));
    } else {
        out.push(two_up(
            ("Call Sheet", call_sheet),
            ("Important Notes", notes),
        ));
    }

    // Important breakdown items
    if d.breakdown_items.is_empty() {
        out.extend(empty_section("Important Breakdown Items", NO_ITEMS));
    } else {
        let pills = d
            .breakdown_items
            .iter()
            .map(|i| Chip::pill(i.name.clone(), Tone::Neutral))
            .collect();
        let mut block = stacked("Important Breakdown Items", pill_rows(pills, 10));
        if let Block::Styled(t) = &mut block {
            t.boxed = false;
        }
        out.push(block);
    }

    // Scene order: the production order and the scene number apart (a numbered list repeats both)
    if d.scenes.is_empty() {
        out.extend(empty_section("Scene Order", NO_ORDER));
    } else {
        let mut t = StyledTable::new(vec![
            Col::pt("#", 28.0).align(Align::Right),
            Col::pt("Scene", 78.0),
            Col::weight("Heading", 1.0),
            Col::pt("Est.", 54.0).align(Align::Right),
        ]);
        t.header = HeaderStyle::Navy;
        t.boxed = true;
        t.zebra = true;
        t.title = Some("Scene Order".into());
        t.title_hint = Some("Production order \u{b7} numbers are scene numbers".into());
        t.pad_x = 6.0;
        t.pad_y = 5.0;
        for (i, x) in d.scenes.iter().enumerate() {
            t.rows.push(Row::new(vec![
                Cell::text(Txt::new((i + 1).to_string()).size(8.4).color(pal::SLATE))
                    .align(Align::Right),
                Cell::new().item(CellItem::Tile(number(x.number))),
                Cell::text(
                    Txt::new(x.heading.clone())
                        .size(8.6)
                        .bold()
                        .color(pal::NAVY),
                ),
                Cell::text(
                    Txt::new(
                        x.estimated_minutes
                            .map(minutes)
                            .unwrap_or_else(|| "\u{2013}".into()),
                    )
                    .size(8.4)
                    .color(pal::SLATE),
                )
                .align(Align::Right),
            ]));
        }
        out.push(Block::Styled(t));
    }
    out
}

pub(super) fn export(
    core: &AppCore,
    actor: &Actor,
    a: ExportDailyViewArgs,
) -> AppResult<super::ExportResult> {
    require_export(actor)?;
    let fmt = Fmt::parse(&a.format, &[Fmt::Pdf, Fmt::Txt])?;
    let dest = destination(core, &a.path, fmt)?;
    let (project, view, revised) = core.project()?.store.read(|c| {
        let sched = current_schedule(c)?.ok_or_else(|| {
            AppError::new(
                "validation.no_schedule",
                "Create a shooting schedule first.",
            )
        })?;
        let revised = revised_at(c, &sched.id)?;
        Ok((
            project_title(c)?,
            daily_view(c, a.day_id.as_deref())?,
            revised,
        ))
    })?;
    let d = view.day.ok_or_else(|| {
        AppError::export(
            "nothing_to_export",
            "The schedule has no shooting days yet.",
        )
    })?;
    // The screen falls back to another day when the id is unknown; a document never does.
    if a.day_id.as_deref().is_some_and(|id| id != d.id) {
        return Err(AppError::not_found("shooting day"));
    }

    let document = format!("{} - Daily View", d.label);
    let date = d.date_label.clone().unwrap_or_else(|| "No date yet".into());
    let secs = sections(&d);

    let mut doc = Document::new(&document);
    let mut mast =
        Masthead::new(d.label.clone()).eyebrow("Production planning / Daily View", pal::ORANGE);
    mast.subtitle = Some(date.clone());
    if let Some(p) = nb(&project) {
        mast.lines.push(format!("Project: {p}"));
    }
    if d.is_off_day {
        mast.badges.push(Chip::label("Off day", Tone::Neutral));
    }
    mast.side = Some(Side::Badge {
        chip: Chip::label("Daily View snapshot", Tone::Accent),
        lines: vec![
            (format!("Revised {}", date_from_ms(revised)), true),
            ("Shooting schedule · PDF export".into(), false),
        ],
    });
    mast.rule = true;
    doc.push(Block::Masthead(mast));
    doc.push(Block::Metrics(Metrics {
        items: vec![
            Metric::new(
                d.summary.scene_count.to_string(),
                if d.summary.scene_count == 1 {
                    "Scene"
                } else {
                    "Scenes"
                },
            ),
            Metric::new(minutes(d.summary.estimated_minutes), "Estimated"),
        ],
        compact: true,
    }));
    for b in day_blocks(&d) {
        doc.push(b);
    }
    // A day that runs onto a second page says whose page it is; a one page day carries no header.
    // Measured with the brand footer on, because the export takes the same bottom margin.
    if fmt == Fmt::Pdf {
        let mut probe = doc.clone();
        probe.brand = Some(BRAND_LINE.to_string());
        if probe.to_pdf().pages > 1 {
            let line: Vec<String> = [nb(&project), Some(d.label.as_str()), Some("Daily View")]
                .into_iter()
                .flatten()
                .map(str::to_uppercase)
                .collect();
            doc = doc.running_header(Some(line.join(" \u{b7} ")));
        }
    }

    let mut out = Output::new(doc, PDF_HINT);
    out.text = Some(text_of(&d.label, &date, &secs));
    out.revised_at = Some(revised);
    deliver(
        core,
        actor,
        &dest,
        fmt,
        Job {
            action: "schedule.export_daily_view",
            document,
            source_label: "Daily View".into(),
            scope_label: d.title.clone(),
            contents_label: plural(d.summary.scene_count as usize, "scene", "scenes"),
            target: Some(("shooting_day", d.id.clone())),
        },
        out,
    )
}
