//! Day Out of Days export (Screenplay and Production Depth spec section 28:
//! "PDF export, spreadsheet export"; feature DOOD-001). A snapshot of the
//! derived report with the user's corrections; it changes nothing.
//!
//! Reproducible: the document carries the date of the schedule's last change,
//! never the export clock, so the same data gives byte-identical files.

use openframe_domain::{Actor, AppError, AppResult};
use openframe_import_export::report::{Block, Column, Document, Table};
use serde::Deserialize;
use ts_rs::TS;

use super::{
    Fmt, HINT_TABLES, Job, Output, deliver, destination, footer, plural, require_export,
    revised_subtitle,
};
use crate::core::AppCore;
use crate::modules::schedule::board::revised_at;
use crate::modules::schedule::dood::{ScheduleDoodArgs, check_args, report};

/// Day columns per PDF table; wider schedules continue in further tables.
const PDF_DAYS_PER_TABLE: usize = 14;

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportDoodArgs {
    /// "pdf" | "csv" | "xlsx"
    pub format: String,
    pub path: String,
    #[serde(default)]
    #[ts(optional)]
    pub cast_keys: Option<Vec<String>>,
    #[serde(default)]
    #[ts(optional)]
    pub from_day_id: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub to_day_id: Option<String>,
}

pub(super) fn export(
    core: &AppCore,
    actor: &Actor,
    a: ExportDoodArgs,
) -> AppResult<super::ExportResult> {
    require_export(actor)?;
    let fmt = Fmt::parse(&a.format, &[Fmt::Pdf, Fmt::Csv, Fmt::Xlsx])?;
    let args = ScheduleDoodArgs {
        cast_keys: a.cast_keys,
        from_day_id: a.from_day_id,
        to_day_id: a.to_day_id,
    };
    check_args(&args)?;
    let dest = destination(core, &a.path, fmt)?;
    let session = core.project()?;
    let (board, view, revised) = session.store.read(|c| {
        let Some((board, view)) = report(c, &args)? else {
            return Err(AppError::new(
                "validation.no_schedule",
                "Create a shooting schedule first.",
            ));
        };
        let revised = revised_at(c, &board.schedule.id)?;
        Ok((board, view, revised))
    })?;
    if view.days.is_empty() || view.rows.is_empty() {
        return Err(AppError::export(
            "nothing_to_export",
            "The Day Out of Days has no shooting days or cast members to show yet.",
        ));
    }
    let document = "Day Out of Days";
    let project = board.project_title.clone();
    let source_label = board
        .source
        .as_ref()
        .map(|s| s.info.label.clone())
        .unwrap_or_else(|| "No production source".into());
    let range = match (view.days.first(), view.days.last()) {
        (Some(f), Some(l)) if f.id != l.id => format!("{} to {}", f.title, l.title),
        (Some(f), _) => f.title.clone(),
        _ => String::new(),
    };
    let corrected: usize = view
        .rows
        .iter()
        .map(|r| r.cells.iter().filter(|c| c.overridden).count())
        .sum();

    // ---- PDF
    let mut doc = Document::new(document)
        .landscape(true)
        .subtitle(revised_subtitle(
            &project,
            Some(&format!("{} · {}", board.schedule.name, source_label)),
            revised,
        ))
        .footer(footer(&project, document));
    doc.push(Block::KeyValues(vec![
        ("Days".into(), range.clone()),
        (
            "Cast".into(),
            plural(view.rows.len(), "cast member", "cast members"),
        ),
        (
            "Codes".into(),
            view.legend
                .iter()
                .map(|c| format!("{} {}", c.code, c.meaning.to_lowercase()))
                .collect::<Vec<_>>()
                .join(" · "),
        ),
        (
            "Corrections".into(),
            if corrected == 0 {
                "None; every cell comes from the schedule".into()
            } else {
                format!(
                    "{} entered by hand (marked *)",
                    plural(corrected, "cell", "cells")
                )
            },
        ),
    ]));
    let cell_text = |code: &str, overridden: bool| {
        if overridden {
            format!("{code}*")
        } else {
            code.to_string()
        }
    };
    for (chunk_i, chunk) in view.days.chunks(PDF_DAYS_PER_TABLE).enumerate() {
        let offset = chunk_i * PDF_DAYS_PER_TABLE;
        let mut cols = vec![
            Column::new("#").weight(0.3),
            Column::new("Cast").weight(2.0),
        ];
        for d in chunk {
            let head = match &d.date {
                Some(date) => format!("{}\n{}", d.short_label, &date[5..]),
                None => d.short_label.clone(),
            };
            cols.push(Column::new(head).weight(0.55));
        }
        let last = offset + chunk.len() >= view.days.len();
        if last {
            cols.extend([
                Column::right("Work").weight(0.5),
                Column::right("Hold").weight(0.5),
                Column::right("Travel").weight(0.55),
                Column::new("Start").weight(0.7),
                Column::new("Finish").weight(0.7),
            ]);
        }
        let mut t = Table::new("", cols);
        for r in &view.rows {
            let mut row = vec![r.number.to_string(), r.label.clone()];
            for c in &r.cells[offset..offset + chunk.len()] {
                row.push(cell_text(&c.code, c.overridden));
            }
            if last {
                row.extend([
                    r.work_days.to_string(),
                    r.hold_days.to_string(),
                    r.travel_days.to_string(),
                    r.start_label.clone().unwrap_or_default(),
                    r.finish_label.clone().unwrap_or_default(),
                ]);
            }
            t.push(row);
        }
        doc.push(Block::Table(t));
    }

    // ---- spreadsheet
    let mut cols = vec![
        Column::right("#"),
        Column::new("Cast"),
        Column::new("Character"),
    ];
    for d in &view.days {
        cols.push(Column::new(match &d.date_label {
            Some(dl) => format!("{} ({dl})", d.short_label),
            None => d.short_label.clone(),
        }));
    }
    cols.extend([
        Column::right("Work"),
        Column::right("Hold"),
        Column::right("Travel"),
        Column::right("Other"),
        Column::new("Start"),
        Column::new("Finish"),
    ]);
    let mut sheet = Table::new("Day Out of Days", cols);
    let mut notes = Table::new(
        "Corrections",
        vec![
            Column::new("Cast"),
            Column::new("Day"),
            Column::new("Code"),
            Column::new("From the schedule"),
            Column::new("Note"),
        ],
    );
    for r in &view.rows {
        let mut row = vec![
            r.number.to_string(),
            r.actor.clone().unwrap_or_default(),
            r.character.clone(),
        ];
        for (c, d) in r.cells.iter().zip(view.days.iter()) {
            row.push(cell_text(&c.code, c.overridden));
            if c.overridden {
                notes.push(vec![
                    r.label.clone(),
                    d.title.clone(),
                    c.code.clone(),
                    c.derived_code.clone(),
                    c.note.clone().unwrap_or_default(),
                ]);
            }
        }
        row.extend([
            r.work_days.to_string(),
            r.hold_days.to_string(),
            r.travel_days.to_string(),
            r.other_days.to_string(),
            r.start_label.clone().unwrap_or_default(),
            r.finish_label.clone().unwrap_or_default(),
        ]);
        sheet.push(row);
    }
    let mut legend = Table::new("Codes", vec![Column::new("Code"), Column::new("Meaning")]);
    for c in &view.legend {
        legend.push(vec![c.code.clone(), c.meaning.clone()]);
    }

    let mut out = Output::new(doc, HINT_TABLES);
    out.revised_at = Some(revised);
    out.tables.push(sheet);
    out.tables.push(legend);
    out.tables.push(notes);
    let job = Job {
        action: "schedule.export_dood",
        document: document.into(),
        source_label: format!("{} · {}", board.schedule.name, source_label),
        scope_label: range,
        contents_label: format!(
            "{} · {}",
            plural(view.rows.len(), "cast member", "cast members"),
            plural(view.days.len(), "day", "days")
        ),
        target: Some(("shooting_schedule", board.schedule.id.clone())),
    };
    deliver(core, actor, &dest, fmt, job, out)
}
