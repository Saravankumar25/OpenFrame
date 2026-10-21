//! Daily View export: one shooting day as a document of its own, with what the Daily View page
//! shows (owner request 2026-10-20, "new changes" item 1). Before this, Export on the Daily View
//! wrote the Shooting Schedule. The document is named "Shoot Day N - Daily view" and holds the
//! scenes and their order, the location, the cast and their call times, the call sheet state,
//! the day's notes and breaks, the breakdown items to prepare and any schedule warnings.
//!
//! It reads the same query as the page (`schedule.daily`), so the document never says more or
//! less than the screen. It changes nothing. Reproducible: the document carries the date of the
//! schedule's last change, never the export clock.

use std::collections::BTreeMap;

use openframe_domain::{Actor, AppError, AppResult};
use openframe_import_export::report::{
    Align, Block, Card, CardHead, CardPart, Cards, Cell, CellItem, Chip, Col, Column, Document,
    Field, Group, GroupItem, HeaderStyle, Masthead, Metric, Metrics, Row, Section, Side,
    StyledTable, Table, Tone, Txt, pal,
};
use serde::Deserialize;
use ts_rs::TS;

use super::schedule::strip_palette;
use super::style::{long_date, nb};
use super::{
    Fmt, HINT_TABLES, Job, Output, date_from_ms, deliver, destination, footer, plural,
    require_export,
};
use crate::core::AppCore;
use crate::modules::schedule::board::{current_schedule, project_title, revised_at};
use crate::modules::schedule::fmt::minutes;
use crate::modules::schedule::ops::{ScheduleDailyArgs, ScheduleDailyDay, daily};

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportDailyArgs {
    /// "pdf" | "csv" | "xlsx"
    pub format: String,
    pub path: String,
    /// The shooting day shown on the Daily View.
    pub day_id: String,
}

/// The document name for a day: "Shoot Day 4 - Daily view".
pub fn document_name(day_label: &str) -> String {
    format!("{} - Daily view", day_label.trim())
}

fn call_sheet_line(status: &str, source_changed: bool) -> &'static str {
    if source_changed {
        return "The schedule changed after this call sheet was made. Refresh it before sending.";
    }
    match status {
        "Final" => "Finalized and matches the schedule.",
        "Needs refresh" => "Needs a refresh from the schedule.",
        _ => "Draft, not yet finalized.",
    }
}

pub(super) fn export(
    core: &AppCore,
    actor: &Actor,
    a: ExportDailyArgs,
) -> AppResult<super::ExportResult> {
    require_export(actor)?;
    let fmt = Fmt::parse(&a.format, &[Fmt::Pdf, Fmt::Csv, Fmt::Xlsx])?;
    if a.day_id.trim().is_empty() {
        return Err(AppError::validation(
            "dayId",
            "Choose a shooting day to export.",
        ));
    }
    let dest = destination(core, &a.path, fmt)?;
    let view = daily(
        core,
        actor,
        ScheduleDailyArgs {
            day_id: Some(a.day_id.clone()),
        },
    )?;
    if !view.has_schedule {
        return Err(AppError::new(
            "validation.no_schedule",
            "Create a shooting schedule first.",
        ));
    }
    let d = view
        .day
        .filter(|d| d.id == a.day_id)
        .ok_or_else(|| AppError::not_found("shooting day").with_detail(a.day_id.clone()))?;
    let session = core.project()?;
    let (project, schedule_name, schedule_id, revised) = session.store.read(|c| {
        let sched = current_schedule(c)?.ok_or_else(|| {
            AppError::new(
                "validation.no_schedule",
                "Create a shooting schedule first.",
            )
        })?;
        let revised = revised_at(c, &sched.id)?;
        Ok((
            project_title(c)?,
            sched.name.clone(),
            sched.id.clone(),
            revised,
        ))
    })?;
    let document = document_name(&d.label);
    let doc = build(&d, &project, &schedule_name, &document, revised);

    let mut out = Output::new(doc, HINT_TABLES);
    out.revised_at = Some(revised);
    out.tables.extend(tables(&d));
    let job = Job {
        action: "schedule.export_daily",
        document: document.clone(),
        source_label: schedule_name,
        scope_label: d.title.clone(),
        contents_label: plural(d.scenes.len(), "scene", "scenes"),
        target: Some(("shooting_schedule", schedule_id)),
    };
    deliver(core, actor, &dest, fmt, job, out)
}

/// The PDF: a portrait page in the production document style.
fn build(
    d: &ScheduleDailyDay,
    project: &str,
    schedule: &str,
    document: &str,
    revised: i64,
) -> Document {
    let mut doc = Document::new(document).footer(footer(project, document));
    let mut mast = Masthead::new(d.label.to_uppercase())
        .eyebrow("Production planning / Daily view", pal::ORANGE);
    if let Some(p) = nb(project) {
        mast.subtitle = Some(p.to_string());
    }
    let when = d
        .date
        .as_deref()
        .and_then(long_date)
        .map(|(weekday, date)| format!("{weekday} · {date}"))
        .unwrap_or_else(|| "No date yet".into());
    mast.lines.push(format!("{when} · Schedule: {schedule}"));
    mast.side = Some(Side::Badge {
        chip: Chip::label(
            if d.is_off_day {
                "Off day"
            } else {
                "Daily view"
            },
            Tone::Accent,
        ),
        lines: vec![
            (format!("Revised {}", date_from_ms(revised)), true),
            ("Shooting day · PDF export".into(), false),
        ],
    });
    mast.rule = true;
    doc.push(Block::Masthead(mast));

    let est = if d.summary.estimated_minutes > 0 {
        minutes(d.summary.estimated_minutes)
    } else {
        "Not entered".into()
    };
    doc.push(Block::Metrics(Metrics {
        items: vec![
            Metric::new(
                format!("{:02}", d.scenes.len()),
                if d.scenes.len() == 1 {
                    "Scene"
                } else {
                    "Scenes"
                },
            ),
            Metric::new(est, "Estimated time"),
            Metric::new(d.summary.pages_label.clone(), "Script pages"),
            Metric::new(
                format!("{:02}", d.cast.len()),
                if d.cast.len() == 1 {
                    "Cast member"
                } else {
                    "Cast members"
                },
            ),
            Metric::new(
                d.crew_call.clone().unwrap_or_else(|| "Not set".into()),
                "Crew call",
            ),
        ],
        compact: false,
    }));

    // ---- scene order, in the colours of the stripboard
    doc.push(Block::Section(Section::new(
        "Scene order",
        Some(if d.summary.missing_estimates > 0 {
            format!(
                "{} · {} without an estimate",
                plural(d.scenes.len(), "scene", "scenes"),
                d.summary.missing_estimates
            )
        } else {
            plural(d.scenes.len(), "scene", "scenes")
        }),
    )));
    // The start column only when the day has start times (as on the schedule export).
    let any_start = d
        .scenes
        .iter()
        .any(|s| s.start_label.as_deref().and_then(nb).is_some());
    let mut cols = Vec::new();
    if any_start {
        cols.push(Col::pt("Start", 46.0));
    }
    cols.extend([
        Col::pt("Sc.", 38.0),
        Col::pt("Type", 52.0),
        Col::weight("Heading / description", 1.0),
        Col::pt("Cast", 120.0),
        Col::pt("Pages", 42.0),
        Col::pt("Est.", 50.0),
    ]);
    let mut tbl = StyledTable::new(cols);
    tbl.header = HeaderStyle::Navy;
    tbl.empty = Some(if d.is_off_day {
        "Off day: no scenes are shot.".into()
    } else {
        "No scenes on this day yet.".into()
    });
    for s in &d.scenes {
        let p = strip_palette(&s.strip_class);
        let mut body = Cell::text(
            Txt::new(s.heading.to_uppercase())
                .size(9.2)
                .bold()
                .color(p.text)
                .leading(1.25),
        );
        if let Some(syn) = nb(&s.synopsis) {
            body = body.item(CellItem::Text(
                Txt::new(syn).size(8.6).color(p.muted).leading(1.3),
            ));
        }
        if let Some(n) = s.notes.as_deref().and_then(nb) {
            body = body.item(CellItem::Text(
                Txt::new(format!("Note: {n}"))
                    .size(8.2)
                    .bold()
                    .color(p.muted),
            ));
        }
        let mut cast = Cell::new();
        for c in &s.cast {
            let text = match &c.actor {
                Some(a) => format!("{} ({a})", c.character),
                None => c.character.clone(),
            };
            cast = cast.item(CellItem::Text(Txt::new(text).size(8.4).color(p.text)));
        }
        let mut cells = Vec::new();
        if any_start {
            cells.push(Cell::text(
                Txt::new(s.start_label.clone().unwrap_or_else(|| "\u{2013}".into()))
                    .size(9.0)
                    .bold()
                    .color(p.text),
            ));
        }
        cells.extend([
            Cell::new().item(CellItem::Tile(
                s.number
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "\u{2013}".into()),
            )),
            Cell::new().item(CellItem::Chips(vec![Chip::label(
                s.ie_label.clone(),
                Tone::Neutral,
            )])),
            body,
            cast,
            Cell::text(
                Txt::new(s.pages_label.clone())
                    .size(9.6)
                    .color(p.text)
                    .align(Align::Right),
            ),
            Cell::text(
                Txt::new(
                    s.estimated_minutes
                        .map(minutes)
                        .unwrap_or_else(|| "\u{2013}".into()),
                )
                .size(9.0)
                .color(p.text)
                .align(Align::Right),
            ),
        ]);
        let mut row = Row::new(cells);
        row.fill = Some(p.fill);
        row.stripe = Some(p.stripe);
        tbl.rows.push(row);
    }
    doc.push(Block::Styled(tbl));

    // ---- location, cast, call sheet
    let mut cards: Vec<Card> = Vec::new();
    let mut loc = card("Location");
    if d.locations.is_empty() {
        loc.parts
            .push(CardPart::Empty("No location for this day.".into()));
    }
    for l in &d.locations {
        loc.parts
            .push(CardPart::Lead(Txt::new(l.name.clone()).size(10.5).bold()));
        let mut fields = Vec::new();
        if let Some(s) = &l.status {
            fields.push(Field::new("Status", s.clone()));
        }
        if let Some(a) = l.address.as_deref().and_then(nb) {
            fields.push(Field::new("Address", a.to_string()));
        }
        if !fields.is_empty() {
            loc.parts.push(CardPart::Fields(fields));
        }
    }
    cards.push(loc);

    let mut sheet = card("Call sheet");
    match &d.call_sheet {
        None => sheet
            .parts
            .push(CardPart::Empty("No call sheet for this day yet.".into())),
        Some(cs) => {
            let line = call_sheet_line(&cs.status, cs.source_changed);
            sheet.parts.push(CardPart::Fields(vec![
                Field::new("Status", cs.status.clone()),
                Field::new("Revision", cs.revision.to_string()),
            ]));
            sheet.parts.push(CardPart::Text(Txt::new(line).size(8.8)));
        }
    }
    cards.push(sheet);
    doc.push(Block::Cards(Cards::new(2, cards)));

    doc.push(Block::Section(Section::new(
        "Cast",
        Some(match &d.crew_call {
            Some(t) => format!("Crew call {t}"),
            None => "Crew call not set".into(),
        }),
    )));
    let mut cast = StyledTable::new(vec![
        Col::weight("Character", 1.0),
        Col::weight("Actor", 1.0),
        Col::pt("Call", 70.0),
    ]);
    cast.header = HeaderStyle::Tint;
    cast.zebra = true;
    cast.empty = Some("No cast on this day.".into());
    for c in &d.cast {
        cast.rows.push(Row::new(vec![
            Cell::text(Txt::new(c.character.clone()).size(9.0).bold()),
            Cell::text(
                Txt::new(c.actor.clone().unwrap_or_else(|| "Not cast yet".into())).size(9.0),
            ),
            Cell::text(
                Txt::new(c.call_time.clone().unwrap_or_else(|| "\u{2013}".into()))
                    .size(9.0)
                    .align(Align::Right),
            ),
        ]));
    }
    doc.push(Block::Styled(cast));

    // ---- notes and breaks
    let notes = note_lines(d);
    if !notes.is_empty() {
        doc.push(Block::Section(Section::new("Important notes", None)));
        let mut c = Card::new();
        c.parts.push(CardPart::Lines(notes));
        doc.push(Block::Cards(Cards::new(1, vec![c])));
    }

    // ---- breakdown items to prepare, by category
    if !d.breakdown_items.is_empty() {
        doc.push(Block::Section(Section::new(
            "Important breakdown items",
            Some(plural(d.breakdown_items.len(), "item", "items")),
        )));
        let mut by: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for it in &d.breakdown_items {
            by.entry(it.category.clone())
                .or_default()
                .push(it.name.clone());
        }
        let groups = by
            .into_iter()
            .map(|(label, names)| Group {
                accent: pal::category(&label),
                label,
                items: names.into_iter().map(GroupItem::new).collect(),
            })
            .collect();
        let mut c = Card::new();
        c.parts.push(CardPart::Groups { groups, cols: 2 });
        doc.push(Block::Cards(Cards::new(1, vec![c])));
    }

    // ---- warnings the schedule raises for this day
    if !d.warnings.is_empty() {
        doc.push(Block::Section(Section::new(
            "Schedule warnings",
            Some(plural(d.warnings.len(), "warning", "warnings")),
        )));
        let mut c = Card::new();
        c.parts.push(CardPart::Lines(
            d.warnings
                .iter()
                .map(|w| match w.detail.as_deref().and_then(nb) {
                    Some(det) => format!("{} {det}", w.message),
                    None => w.message.clone(),
                })
                .collect(),
        ));
        doc.push(Block::Cards(Cards::new(1, vec![c])));
    }
    doc
}

fn card(title: &str) -> Card {
    let mut head = CardHead::new(title.to_uppercase());
    head.fill = pal::TINT;
    Card {
        head: Some(head),
        ..Card::new()
    }
}

/// The day's own notes, then each break with its time and length.
fn note_lines(d: &ScheduleDailyDay) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(n) = d.notes.as_deref().and_then(nb) {
        out.push(n.to_string());
    }
    for m in &d.markers {
        let mut s = m.label.clone();
        if let Some(t) = &m.at_time {
            s.push_str(&format!(" at {t}"));
        }
        if let Some(len) = m.duration_minutes.filter(|x| *x > 0) {
            s.push_str(&format!(" ({})", minutes(len)));
        }
        if let Some(n) = m.notes.as_deref().and_then(nb) {
            s.push_str(&format!(": {n}"));
        }
        out.push(s);
    }
    out
}

/// The spreadsheet form: the scene order, the cast and the items.
fn tables(d: &ScheduleDailyDay) -> Vec<Table> {
    let mut scenes = Table::new(
        "Scene order",
        [
            "Start", "Scene", "Type", "Heading", "Synopsis", "Cast", "Pages", "Estimate",
        ]
        .into_iter()
        .map(Column::new)
        .collect(),
    );
    for s in &d.scenes {
        scenes.push(vec![
            s.start_label.clone().unwrap_or_default(),
            s.number.map(|n| n.to_string()).unwrap_or_default(),
            s.ie_label.clone(),
            s.heading.clone(),
            s.synopsis.clone(),
            s.cast
                .iter()
                .map(|c| c.character.clone())
                .collect::<Vec<_>>()
                .join(", "),
            s.pages_label.clone(),
            s.estimated_minutes.map(minutes).unwrap_or_default(),
        ]);
    }
    let mut cast = Table::new(
        "Cast",
        ["Character", "Actor", "Call"]
            .into_iter()
            .map(Column::new)
            .collect(),
    );
    for c in &d.cast {
        cast.push(vec![
            c.character.clone(),
            c.actor.clone().unwrap_or_default(),
            c.call_time.clone().unwrap_or_default(),
        ]);
    }
    let mut items = Table::new(
        "Breakdown items",
        ["Category", "Item"].into_iter().map(Column::new).collect(),
    );
    for it in &d.breakdown_items {
        items.push(vec![it.category.clone(), it.name.clone()]);
    }
    let mut notes = Table::new("Notes", vec![Column::new("Note")]);
    for n in note_lines(d) {
        notes.push(vec![n]);
    }
    vec![scenes, cast, items, notes]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_document_is_named_after_the_day() {
        assert_eq!(document_name("Shoot Day 1"), "Shoot Day 1 - Daily view");
        assert_eq!(document_name(" Shoot Day 12 "), "Shoot Day 12 - Daily view");
    }

    #[test]
    fn a_call_sheet_line_says_whether_it_can_be_sent() {
        assert_eq!(
            call_sheet_line("Final", false),
            "Finalized and matches the schedule."
        );
        assert!(call_sheet_line("Final", true).starts_with("The schedule changed"));
        assert_eq!(call_sheet_line("Draft", false), "Draft, not yet finalized.");
    }
}
