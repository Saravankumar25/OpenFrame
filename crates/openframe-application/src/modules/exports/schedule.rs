//! Shooting Schedule and stripboard export (FSD 60.2 "Schedule/Stripboard:
//! PDF"; Import/Export 6: "shooting days, scene assignments, markers, and
//! supported day information … It does not alter the schedule"; UX 3.29
//! Export dialog: Entire schedule / Current day only / Selected days, PDF or
//! spreadsheet, "Include unscheduled scenes").
//!
//! Reproducible: the document carries the date of the schedule's last change,
//! never the export clock, so exporting the same data twice gives byte-identical
//! PDF, CSV and XLSX files.

use std::collections::HashSet;

use openframe_domain::{Actor, AppError, AppResult};
use openframe_import_export::report::{
    Align, Banner, Block, Cell, CellItem, Chip, Col, Column, Document, Legend, LegendItem,
    Masthead, Metric, Metrics, Rgb, Row, Section, Side, StyledTable, Table, Tone, Txt, pal,
};
use serde::Deserialize;
use ts_rs::TS;

use super::style::{long_date, nb};
use super::{
    Fmt, HINT_TABLES, Job, Output, date_from_ms, deliver, destination, footer, nonblank, plural,
    require_export,
};
use crate::core::AppCore;
use crate::modules::schedule::board::{
    Board, DayRow, Item, StripRow, current_schedule, revised_at,
};
use crate::modules::schedule::fmt::{date_short, minutes, pages};
use crate::modules::schedule::timeline::ScheduleDayTimeline;

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportScheduleArgs {
    /// "pdf" | "csv" | "xlsx"
    pub format: String,
    pub path: String,
    /// Days to include in order (None = entire schedule).
    #[serde(default)]
    #[ts(optional)]
    pub day_ids: Option<Vec<String>>,
    /// Unscheduled scenes pool (default true for the entire schedule, false for selected days).
    #[serde(default)]
    #[ts(optional)]
    pub include_unscheduled: Option<bool>,
    /// Scene-strip notes (schedule notes; never private notes). Default false.
    #[serde(default)]
    #[ts(optional)]
    pub include_notes: Option<bool>,
}

struct StripText {
    start: String,
    number: String,
    ie: String,
    dn: String,
    heading: String,
    location: String,
    synopsis: String,
    cast: String,
    pages: String,
    est: String,
    notes: String,
    /// Stripboard colour class from the schedule: xd, xn, id or in.
    class: String,
    /// The page count was entered by hand.
    overridden: bool,
    /// Current, Changed, Removed or New (script reconciliation).
    state: String,
    /// One entry per cast member, "Character (Actor)" when an actor is cast.
    cast_list: Vec<String>,
}

struct StripPalette {
    fill: Rgb,
    stripe: Rgb,
    text: Rgb,
    muted: Rgb,
}

/// Classic stripboard colours (owner decision 2026-10-20): EXT + Day yellow, EXT + Night green,
/// INT + Day white, INT + Night light blue (INT/EXT counts as EXT). The ink stays dark on every
/// fill so the print is readable; labels carry the meaning, colour reinforces it.
fn strip_palette(class: &str) -> StripPalette {
    let (fill, stripe, text, muted) = match class {
        "xd" => (0xfbe98f, 0xd9a400, 0x1a2640, 0x5a4a1f),
        "xn" => (0xc9e7b8, 0x4c9a3a, 0x1a2640, 0x2f5a25),
        "id" => (0xffffff, 0xa9b6c6, 0x1a2640, 0x566277),
        _ => (0xcfe4f6, 0x4a8cc4, 0x1a2640, 0x2f4d6b),
    };
    StripPalette {
        fill: Rgb::hex(fill),
        stripe: Rgb::hex(stripe),
        text: Rgb::hex(text),
        muted: Rgb::hex(muted),
    }
}

/// Estimated start of an item on its day ("08:00"), when the day has times.
fn start_of(tl: Option<&ScheduleDayTimeline>, id: &str) -> String {
    tl.and_then(|t| t.rows.iter().find(|r| r.item_id.as_deref() == Some(id)))
        .and_then(|r| r.start_label.clone())
        .unwrap_or_default()
}

fn strip_text(b: &Board, s: &StripRow, tl: Option<&ScheduleDayTimeline>) -> StripText {
    let dto = b.strip_dto(s);
    StripText {
        start: start_of(tl, &s.id),
        number: dto
            .number
            .map(|n| n.to_string())
            .unwrap_or_else(|| "-".into()),
        ie: dto.int_ext.clone(),
        dn: match dto.day_night.as_str() {
            "D" => "Day".into(),
            "N" => "Night".into(),
            other => other.to_string(),
        },
        heading: if dto.missing_from_source {
            format!("{} (no longer in the script)", dto.heading)
        } else {
            dto.heading.clone()
        },
        location: dto
            .locations
            .iter()
            .map(|l| l.name.clone())
            .collect::<Vec<_>>()
            .join(", "),
        synopsis: dto.synopsis.clone(),
        cast: dto
            .cast
            .iter()
            .map(|c| match &c.actor {
                Some(a) => format!("{} ({a})", c.character),
                None => c.character.clone(),
            })
            .collect::<Vec<_>>()
            .join(", "),
        pages: dto.pages_label.clone(),
        est: dto.estimated_minutes.map(minutes).unwrap_or_default(),
        notes: dto.notes.clone().unwrap_or_default(),
        class: dto.strip_class.clone(),
        overridden: dto.pages_overridden,
        state: dto.source_state.clone(),
        cast_list: dto
            .cast
            .iter()
            .map(|c| match c.actor.as_deref().and_then(nb) {
                Some(a) => format!("{} ({a})", c.character),
                None => c.character.clone(),
            })
            .collect(),
    }
}

/// The complete synopsis of every scene, as stored: the scene's own synopsis, else its first
/// action paragraph. The schedule model keeps only the first 90 characters of it (with an
/// ellipsis), which is right for a strip on screen but would leave the printed schedule
/// incomplete.
fn full_synopses(c: &rusqlite::Connection) -> AppResult<std::collections::HashMap<String, String>> {
    let mut st = c.prepare(
        "SELECT s.id, s.synopsis,
                (SELECT e.text FROM screenplay_element e
                  WHERE e.scene_id = s.id AND e.element_type = 'action' AND trim(e.text) <> ''
                  ORDER BY e.position, e.id LIMIT 1)
         FROM screenplay_scene s WHERE s.deleted_at IS NULL",
    )?;
    let rows = st.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, Option<String>>(1)?,
            r.get::<_, Option<String>>(2)?,
        ))
    })?;
    let mut out = std::collections::HashMap::new();
    for row in rows {
        let (id, synopsis, action) = row?;
        let text = synopsis
            .filter(|t| !t.trim().is_empty())
            .or(action)
            .map(|t| t.split_whitespace().collect::<Vec<_>>().join(" "))
            .unwrap_or_default();
        if !text.is_empty() {
            out.insert(id, text);
        }
    }
    Ok(out)
}

pub(super) fn export(
    core: &AppCore,
    actor: &Actor,
    a: ExportScheduleArgs,
) -> AppResult<super::ExportResult> {
    require_export(actor)?;
    let fmt = Fmt::parse(&a.format, &[Fmt::Pdf, Fmt::Csv, Fmt::Xlsx])?;
    if a.day_ids.as_ref().is_some_and(|v| v.is_empty()) {
        return Err(AppError::validation(
            "dayIds",
            "Select at least one shooting day to export.",
        ));
    }
    let notes = a.include_notes.unwrap_or(false);
    let unscheduled = a.include_unscheduled.unwrap_or(a.day_ids.is_none());
    let dest = destination(core, &a.path, fmt)?;
    let session = core.project()?;
    let (board, revised, full_synopsis) = session.store.read(|c| {
        let sched = current_schedule(c)?.ok_or_else(|| {
            AppError::new(
                "validation.no_schedule",
                "Create a shooting schedule first.",
            )
        })?;
        let revised = revised_at(c, &sched.id)?;
        Ok((Board::load(c, sched)?, revised, full_synopses(c)?))
    })?;
    let days: Vec<&DayRow> = match &a.day_ids {
        None => board.days.iter().collect(),
        Some(ids) => {
            let wanted: HashSet<&str> = ids.iter().map(|s| s.as_str()).collect();
            if let Some(missing) = wanted.iter().find(|id| board.day(id).is_none()) {
                return Err(
                    AppError::not_found("shooting day").with_detail(format!("day {missing}"))
                );
            }
            // Selected days keep schedule order.
            board
                .days
                .iter()
                .filter(|d| wanted.contains(d.id.as_str()))
                .collect()
        }
    };
    let pool: Vec<&StripRow> = if unscheduled {
        board.unscheduled()
    } else {
        Vec::new()
    };
    if days.is_empty() && pool.is_empty() {
        return Err(AppError::export(
            "nothing_to_export",
            "The schedule has no shooting days yet.",
        ));
    }
    let source_label = board
        .source
        .as_ref()
        .map(|s| s.info.label.clone())
        .unwrap_or_else(|| "No production source".into());
    let document = "Shooting Schedule";
    let project = board.project_title.clone();
    let scheduled: usize = days.iter().map(|d| board.day_strips(&d.id).len()).sum();

    // ---- PDF (landscape stripboard)
    let mut doc = Document::new(document)
        .landscape(true)
        .footer(footer(&project, document));
    let shoot_days = days.iter().filter(|d| !d.off).count();
    let total_eighths: i64 = days
        .iter()
        .filter(|d| !d.off)
        .map(|d| board.day_summary(d).page_eighths)
        .sum();
    let mut mast =
        Masthead::new("SHOOTING SCHEDULE").eyebrow("Production planning / Schedule", pal::ORANGE);
    if let Some(p) = nb(&project) {
        mast.subtitle = Some(p.to_string());
    }
    mast.lines.push(format!(
        "Production source: {source_label} · Schedule: {}",
        board.schedule.name
    ));
    mast.side = Some(Side::Badge {
        chip: Chip::label("Schedule snapshot", Tone::Accent),
        lines: vec![
            (format!("Revised {}", date_from_ms(revised)), true),
            ("Project schedule · PDF export".into(), false),
        ],
    });
    mast.rule = true;
    doc.push(Block::Masthead(mast));
    let mut metrics = vec![
        Metric::new(
            format!("{shoot_days:02}"),
            if shoot_days == 1 {
                "Shooting day"
            } else {
                "Shooting days"
            },
        ),
        Metric::new(
            format!("{scheduled:02}"),
            if scheduled == 1 {
                "Scheduled scene"
            } else {
                "Scheduled scenes"
            },
        ),
        Metric::new(pages(total_eighths), "Script pages"),
    ];
    if unscheduled {
        metrics.push(Metric::new(
            format!("{:02}", pool.len()),
            if pool.len() == 1 {
                "Unscheduled scene"
            } else {
                "Unscheduled scenes"
            },
        ));
    }
    doc.push(Block::Metrics(Metrics {
        items: metrics,
        compact: false,
    }));

    // Strip text for the PDF: the same fields as the spreadsheet, with the full synopsis.
    let pdf_strip = |s: &StripRow, tl: Option<&ScheduleDayTimeline>| {
        let mut t = strip_text(&board, s, tl);
        if let Some(full) = full_synopsis.get(&s.scene_id) {
            t.synopsis = full.clone();
        }
        t
    };
    let marker_text = |m: &crate::modules::schedule::board::MarkerRow| {
        let mut s = m.label.to_uppercase();
        if let Some(t) = &m.at_time {
            s.push_str(&format!(" at {t}"));
        }
        if let Some(d) = m.duration_minutes.filter(|d| *d > 0) {
            s.push_str(&format!(" ({})", minutes(d)));
        }
        if let Some(n) = nonblank(m.notes.as_deref()) {
            s.push_str(&format!(" · {n}"));
        }
        s
    };
    let strip_row = |t: &StripText, any_start: bool, any_est: bool| -> Row {
        let pal_ = strip_palette(&t.class);
        let ink = Txt::new("").color(pal_.text);
        let mut cells: Vec<Cell> = Vec::new();
        cells.push(Cell::new().item(CellItem::Tile(t.number.clone())));
        let mut chips = Vec::new();
        if !t.ie.trim().is_empty() {
            chips.push(Chip::label(t.ie.clone(), Tone::Neutral));
        }
        if !t.dn.trim().is_empty() {
            chips.push(Chip::label(t.dn.clone(), Tone::Neutral));
        }
        let mut type_cell = Cell::new();
        if !chips.is_empty() {
            type_cell = type_cell.item(CellItem::Chips(chips));
        }
        cells.push(type_cell);
        if any_start {
            cells.push(Cell::text(
                Txt::new(if t.start.is_empty() {
                    "\u{2013}".to_string()
                } else {
                    t.start.clone()
                })
                .size(9.0)
                .bold()
                .color(pal_.text),
            ));
        }
        let mut body = Cell::text(
            Txt::new(t.heading.to_uppercase())
                .size(9.6)
                .bold()
                .color(pal_.text)
                .leading(1.25),
        );
        if let Some(l) = nb(&t.location)
            && !t.heading.to_uppercase().contains(&l.to_uppercase())
        {
            body = body.item(CellItem::Text(
                Txt::new(l).size(8.0).bold().color(pal_.muted),
            ));
        }
        if let Some(s) = nb(&t.synopsis) {
            body = body.item(CellItem::Text(
                Txt::new(s).size(8.8).color(pal_.muted).leading(1.3),
            ));
        }
        if let Some(n) = nb(&t.notes) {
            body = body.item(CellItem::Text(
                Txt::new(format!("Note: {n}"))
                    .size(8.2)
                    .bold()
                    .color(pal_.muted),
            ));
        }
        let mut flags = Vec::new();
        match t.state.as_str() {
            "Changed" => flags.push(Chip::label("Changed in the script", Tone::Warn).small()),
            "New" => flags.push(Chip::label("New in the script", Tone::Info).small()),
            "Removed" => flags.push(Chip::label("Removed from the script", Tone::Danger).small()),
            _ => {}
        }
        if !flags.is_empty() {
            body = body.item(CellItem::Chips(flags));
        }
        cells.push(body);
        let mut cast = Cell::new();
        for c in &t.cast_list {
            cast = cast.item(CellItem::Text(
                Txt::new(c.clone()).size(8.8).color(pal_.text).leading(1.25),
            ));
        }
        cells.push(cast);
        let mut pg = Cell::text(
            Txt::new(t.pages.clone())
                .size(11.0)
                .color(pal_.text)
                .align(Align::Right),
        );
        if t.overridden {
            pg = pg.item(CellItem::Text(
                Txt::new("MANUAL")
                    .size(6.4)
                    .bold()
                    .track(0.5)
                    .color(pal_.muted)
                    .align(Align::Right),
            ));
        }
        cells.push(pg);
        if any_est {
            cells.push(Cell::text(
                Txt::new(if t.est.is_empty() {
                    "\u{2013}".to_string()
                } else {
                    t.est.clone()
                })
                .size(9.0)
                .color(pal_.text)
                .align(Align::Right),
            ));
        }
        let _ = ink;
        let mut row = Row::new(cells);
        row.fill = Some(pal_.fill);
        row.stripe = Some(pal_.stripe);
        row
    };

    for d in &days {
        let (number, date_right) = (
            d.number,
            d.date
                .as_deref()
                .and_then(long_date)
                .map(|(wd, rest)| format!("{} · {}", wd.to_uppercase(), rest.to_uppercase())),
        );
        let mut b = Banner::new(if d.off {
            "Off day".to_string()
        } else {
            format!("Day {}", number.unwrap_or(0))
        });
        b.eyebrow = Some(if d.off {
            "Off day".to_string()
        } else {
            format!("Shoot day {:02}", number.unwrap_or(0))
        });
        b.right = date_right.clone();
        if d.off {
            b.fill = Rgb::hex(0x56627a);
            b.sub_left = vec![("No shooting".into(), true)];
            doc.push(Block::Banner(b));
            if let Some(n) = nonblank(d.notes.as_deref()) {
                doc.push(Block::Text(
                    Txt::new(format!("Day notes: {n}"))
                        .size(8.8)
                        .color(pal::SLATE)
                        .after(6.0),
                ));
            }
            continue;
        }
        let sum = board.day_summary(d);
        let tl = board.day_timeline(d);
        // Scene strips only; meals, moves and other rows are counted apart.
        let others = board.markers.iter().filter(|m| m.day_id == d.id).count();
        let mut counts = plural(sum.scene_count as usize, "scene", "scenes");
        if others > 0 {
            counts.push_str(&format!(
                ", {}",
                plural(others, "other item", "other items")
            ));
        }
        b.sub_left = vec![
            (counts, true),
            (" · arranged in production order".into(), false),
        ];
        let page_label = pages(sum.page_eighths);
        let mut right = vec![
            (page_label.clone(), true),
            (
                if page_label == "1" {
                    " page total".to_string()
                } else {
                    " pages total".to_string()
                },
                false,
            ),
        ];
        if !tl.summary.is_empty() && tl.anchored {
            right.push((format!(" · {}", tl.summary), false));
        }
        if sum.over_target {
            right.push((
                if tl.over_target_wrap {
                    " · past the target wrap".to_string()
                } else {
                    format!(" · over the {} target", minutes(sum.target_minutes))
                },
                true,
            ));
        }
        if let Some(cs) = board.call_sheet_brief(d) {
            right.push((format!(" · Call sheet: {}", cs.status), false));
        }
        b.sub_right = right;
        b.keep_next = 110.0;
        doc.push(Block::Banner(b));
        if let Some(n) = nonblank(d.notes.as_deref()) {
            doc.push(Block::Text(
                Txt::new(format!("Day notes: {n}"))
                    .size(8.8)
                    .color(pal::SLATE)
                    .after(6.0),
            ));
        }
        let items = board.day_items(&d.id);
        if items.is_empty() {
            doc.push(Block::Text(
                Txt::new("No scenes scheduled.")
                    .size(8.8)
                    .color(pal::FAINT)
                    .after(6.0),
            ));
            continue;
        }
        let texts: Vec<Option<StripText>> = items
            .iter()
            .map(|it| match it {
                Item::Strip(s) => Some(pdf_strip(s, Some(&tl))),
                Item::Marker(_) => None,
            })
            .collect();
        let any_start = texts.iter().flatten().any(|t| !t.start.is_empty())
            || items
                .iter()
                .any(|it| matches!(it, Item::Marker(m) if !start_of(Some(&tl), &m.id).is_empty()));
        let any_est = texts.iter().flatten().any(|t| !t.est.is_empty())
            || items.iter().any(
                |it| matches!(it, Item::Marker(m) if m.duration_minutes.is_some_and(|d| d > 0)),
            );
        let mut cols = vec![Col::pt("Scene", 48.0), Col::pt("Type", 64.0)];
        if any_start {
            cols.push(Col::pt("Start", 50.0));
        }
        cols.push(Col::weight("Heading & synopsis", 1.0));
        cols.push(Col::pt("Cast", 150.0));
        cols.push(Col::pt("Pages", 56.0).align(Align::Right));
        if any_est {
            cols.push(Col::pt("Est.", 52.0).align(Align::Right));
        }
        let mut tbl = StyledTable::new(cols);
        tbl.continued = Some(format!(
            "Shoot day {:02}{}",
            number.unwrap_or(0),
            date_right.map(|r| format!(" · {r}")).unwrap_or_default()
        ));
        tbl.boxed = true;
        for (it, text) in items.iter().zip(&texts) {
            match (it, text) {
                (Item::Strip(_), Some(t)) => tbl.rows.push(strip_row(t, any_start, any_est)),
                (Item::Marker(m), _) => {
                    // A break sits on the same grid as the scenes, in a neutral row.
                    let start = start_of(Some(&tl), &m.id);
                    let dur = m.duration_minutes.filter(|d| *d > 0).map(minutes);
                    let mut label = m.label.to_uppercase();
                    if !any_start && let Some(t) = &m.at_time {
                        label.push_str(&format!(" at {t}"));
                    }
                    if !any_est && let Some(d) = &dur {
                        label.push_str(&format!(" ({d})"));
                    }
                    let mut body = Cell::text(Txt::new(label).size(9.2).bold().color(pal::SLATE));
                    if let Some(n) = nonblank(m.notes.as_deref()) {
                        body = body.item(CellItem::Text(
                            Txt::new(n).size(8.4).color(pal::SLATE).leading(1.3),
                        ));
                    }
                    let mut cells = vec![
                        Cell::empty(),
                        Cell::new().item(CellItem::Chips(vec![
                            Chip::label(m.marker_type.clone(), Tone::Neutral).small(),
                        ])),
                    ];
                    if any_start {
                        cells.push(Cell::text(
                            Txt::new(if start.is_empty() {
                                m.at_time.clone().unwrap_or_else(|| "–".into())
                            } else {
                                start
                            })
                            .size(9.0)
                            .bold()
                            .color(pal::SLATE),
                        ));
                    }
                    cells.push(body);
                    cells.push(Cell::empty());
                    cells.push(Cell::empty());
                    if any_est {
                        cells.push(Cell::text(
                            Txt::new(dur.unwrap_or_else(|| "–".into()))
                                .size(9.0)
                                .color(pal::SLATE)
                                .align(Align::Right),
                        ));
                    }
                    let mut row = Row::new(cells);
                    row.fill = Some(Rgb::hex(0xf1f3f6));
                    row.stripe = Some(pal::FAINT);
                    tbl.rows.push(row);
                }
                _ => {}
            }
        }
        doc.push(Block::Styled(tbl));
    }
    doc.push(Block::Legend(Legend {
        title: Some("Strip legend".into()),
        columns: 0,
        items: vec![
            ("xd", "Exterior / Day"),
            ("xn", "Exterior / Night"),
            ("id", "Interior / Day"),
            ("in", "Interior / Night"),
        ]
        .into_iter()
        .map(|(class, label)| {
            let p = strip_palette(class);
            LegendItem {
                swatch: p.fill,
                border: Some(p.stripe),
                code: None,
                label: label.into(),
            }
        })
        .collect(),
    }));
    if unscheduled {
        doc.push(Block::Section(Section::new(
            format!("Unscheduled · {}", pool.len()),
            None,
        )));
        if pool.is_empty() {
            doc.push(Block::Text(
                Txt::new("All source scenes are scheduled.")
                    .size(8.8)
                    .color(pal::SLATE)
                    .after(6.0),
            ));
        } else {
            let texts: Vec<StripText> = pool.iter().map(|s| pdf_strip(s, None)).collect();
            let any_est = texts.iter().any(|t| !t.est.is_empty());
            let mut cols = vec![
                Col::pt("Scene", 48.0),
                Col::pt("Type", 64.0),
                Col::weight("Heading & synopsis", 1.0),
                Col::pt("Cast", 150.0),
                Col::pt("Pages", 56.0).align(Align::Right),
            ];
            if any_est {
                cols.push(Col::pt("Est.", 52.0).align(Align::Right));
            }
            let mut tbl = StyledTable::new(cols);
            tbl.continued = Some("Unscheduled".into());
            tbl.boxed = true;
            for t in &texts {
                tbl.rows.push(strip_row(t, false, any_est));
            }
            doc.push(Block::Styled(tbl));
        }
    }

    // ---- spreadsheet
    let mut cols = vec![
        Column::new("Day"),
        Column::new("Date"),
        Column::right("Order"),
        Column::new("Start"),
        Column::new("Item"),
        Column::right("Scene"),
        Column::new("I/E"),
        Column::new("D/N"),
        Column::new("Heading"),
        Column::new("Location"),
        Column::new("Synopsis"),
        Column::new("Cast"),
        Column::new("Pages"),
        Column::new("Est. time"),
    ];
    if notes {
        cols.push(Column::new("Notes"));
    }
    let mut sheet = Table::new("Schedule", cols);
    let row = |day: &str,
               date: &str,
               order: usize,
               start: String,
               item: &str,
               t: Option<&StripText>,
               label: &str| {
        let mut r = vec![
            day.to_string(),
            date.to_string(),
            order.to_string(),
            start,
            item.to_string(),
        ];
        match t {
            Some(t) => r.extend([
                t.number.clone(),
                t.ie.clone(),
                t.dn.clone(),
                t.heading.clone(),
                t.location.clone(),
                t.synopsis.clone(),
                t.cast.clone(),
                t.pages.clone(),
                t.est.clone(),
            ]),
            None => {
                r.extend([
                    String::new(),
                    String::new(),
                    String::new(),
                    label.to_string(),
                ]);
                r.extend(std::iter::repeat_n(String::new(), 5));
            }
        }
        if notes {
            r.push(t.map(|t| t.notes.clone()).unwrap_or_default());
        }
        r
    };
    let mut days_sheet = Table::new(
        "Days",
        vec![
            Column::new("Day"),
            Column::new("Date"),
            Column::right("Scenes"),
            Column::new("Pages"),
            Column::new("Estimated"),
            Column::new("Crew call"),
            Column::new("Shooting start"),
            Column::new("Target wrap"),
            Column::new("Estimated wrap"),
            Column::new("Locations"),
            Column::new("Cast"),
            Column::new("Call sheet"),
            Column::new("Day notes"),
        ],
    );
    for d in &days {
        let label = d.label();
        let date = d.date.as_deref().map(date_short).unwrap_or_default();
        let tl = board.day_timeline(d);
        if d.off {
            sheet.push(row(
                &label,
                &date,
                0,
                String::new(),
                "Off Day",
                None,
                "Off day",
            ));
        } else {
            for (i, it) in board.day_items(&d.id).into_iter().enumerate() {
                match it {
                    Item::Strip(s) => sheet.push(row(
                        &label,
                        &date,
                        i + 1,
                        start_of(Some(&tl), &s.id),
                        "Scene",
                        Some(&strip_text(&board, s, Some(&tl))),
                        "",
                    )),
                    Item::Marker(m) => sheet.push(row(
                        &label,
                        &date,
                        i + 1,
                        start_of(Some(&tl), &m.id),
                        &m.label,
                        None,
                        &marker_text(m),
                    )),
                }
            }
        }
        let sum = board.day_summary(d);
        days_sheet.push(vec![
            label,
            date,
            sum.scene_count.to_string(),
            pages(sum.page_eighths),
            if sum.estimated_minutes > 0 {
                minutes(sum.estimated_minutes)
            } else {
                String::new()
            },
            tl.crew_call_label.clone().unwrap_or_default(),
            tl.shooting_start_label.clone().unwrap_or_default(),
            tl.target_wrap_label.clone().unwrap_or_default(),
            tl.estimated_wrap_label
                .clone()
                .filter(|_| tl.anchored)
                .unwrap_or_default(),
            sum.locations
                .iter()
                .map(|l| l.name.clone())
                .collect::<Vec<_>>()
                .join(", "),
            sum.cast
                .iter()
                .map(|c| c.character.clone())
                .collect::<Vec<_>>()
                .join(", "),
            board
                .call_sheet_brief(d)
                .map(|c| c.status)
                .unwrap_or_default(),
            d.notes.clone().unwrap_or_default(),
        ]);
    }
    for (i, s) in pool.iter().enumerate() {
        sheet.push(row(
            "Unscheduled",
            "",
            i + 1,
            String::new(),
            "Scene",
            Some(&strip_text(&board, s, None)),
            "",
        ));
    }

    let mut out = Output::new(doc, HINT_TABLES);
    out.revised_at = Some(revised);
    out.tables.push(sheet);
    out.tables.push(days_sheet);
    let scope_label = match &a.day_ids {
        None => "Entire schedule".to_string(),
        Some(v) if v.len() == 1 => "Current day only".to_string(),
        Some(v) => plural(v.len(), "selected day", "selected days"),
    };
    let job = Job {
        action: "schedule.export_schedule",
        document: document.into(),
        source_label: format!("{} · {}", board.schedule.name, source_label),
        scope_label,
        contents_label: format!(
            "{} · {}",
            plural(days.len(), "day", "days"),
            plural(scheduled + pool.len(), "scene", "scenes")
        ),
        target: Some(("shooting_schedule", board.schedule.id.clone())),
    };
    deliver(core, actor, &dest, fmt, job, out)
}
