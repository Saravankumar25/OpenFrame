//! Shooting Schedule / stripboard export (FSD §60.2 "Schedule/Stripboard:
//! PDF"; Import/Export §6: "shooting days, scene assignments, markers, and
//! supported day information … It does not alter the schedule"; UX §3.29
//! Export dialog: Entire schedule / Current day only / Selected days, PDF or
//! spreadsheet, "Include unscheduled scenes").

use std::collections::HashSet;

use openframe_domain::{Actor, AppError, AppResult};
use openframe_import_export::report::{Block, Column, Document, Table};
use serde::Deserialize;
use ts_rs::TS;

use super::{
    Fmt, HINT_TABLES, Job, Output, deliver, destination, footer, nonblank, plural, require_export,
    subtitle,
};
use crate::core::AppCore;
use crate::modules::schedule::board::{Board, DayRow, Item, StripRow, current_schedule};
use crate::modules::schedule::fmt::{date_short, minutes, pages};

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
}

fn strip_text(b: &Board, s: &StripRow) -> StripText {
    let dto = b.strip_dto(s);
    StripText {
        number: dto
            .number
            .map(|n| n.to_string())
            .unwrap_or_else(|| "—".into()),
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
    }
}

fn day_heading(d: &DayRow) -> String {
    d.title()
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
    let board = session.store.read(|c| {
        let sched = current_schedule(c)?.ok_or_else(|| {
            AppError::new(
                "validation.no_schedule",
                "Create a shooting schedule first.",
            )
        })?;
        Board::load(c, sched)
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
        .subtitle(subtitle(
            &project,
            Some(&format!("{} · {}", board.schedule.name, source_label)),
        ))
        .footer(footer(&project, document));
    let shoot_days = days.iter().filter(|d| !d.off).count();
    doc.push(Block::KeyValues(vec![
        ("Schedule".into(), board.schedule.name.clone()),
        ("Production Source".into(), source_label.clone()),
        (
            "Days".into(),
            format!(
                "{}{}",
                plural(shoot_days, "shooting day", "shooting days"),
                match days.len() - shoot_days {
                    0 => String::new(),
                    n => format!(" · {}", plural(n, "off day", "off days")),
                }
            ),
        ),
        (
            "Scenes".into(),
            format!(
                "{} scheduled{}",
                scheduled,
                if unscheduled {
                    format!(" · {} unscheduled", pool.len())
                } else {
                    String::new()
                }
            ),
        ),
    ]));
    let pdf_columns = |notes: bool| {
        let mut c = vec![
            Column::new("Sc").weight(0.35),
            Column::new("I/E").weight(0.4),
            Column::new("D/N").weight(0.4),
            Column::new("Heading / location").weight(1.8),
            Column::new("Synopsis").weight(1.8),
            Column::new("Cast").weight(1.4),
            Column::right("Pages").weight(0.45),
            Column::right("Est.").weight(0.5),
        ];
        if notes {
            c.push(Column::new("Notes").weight(1.2));
        }
        c
    };
    let strip_row = |t: &StripText| {
        let mut r = vec![
            t.number.clone(),
            t.ie.clone(),
            t.dn.clone(),
            if t.location.is_empty() || t.heading.contains(&t.location.to_uppercase()) {
                t.heading.clone()
            } else {
                format!("{}\n{}", t.heading, t.location)
            },
            t.synopsis.clone(),
            t.cast.clone(),
            t.pages.clone(),
            t.est.clone(),
        ];
        if notes {
            r.push(t.notes.clone());
        }
        r
    };
    let marker_text = |m: &crate::modules::schedule::board::MarkerRow| {
        let mut s = m.label.to_uppercase();
        if let Some(t) = &m.at_time {
            s.push_str(&format!(" — {t}"));
        }
        if let Some(d) = m.duration_minutes.filter(|d| *d > 0) {
            s.push_str(&format!(" ({})", minutes(d)));
        }
        if let Some(n) = nonblank(m.notes.as_deref()) {
            s.push_str(&format!(" · {n}"));
        }
        s
    };
    for d in &days {
        doc.push(Block::Heading {
            text: day_heading(d),
            level: 2,
        });
        if d.off {
            doc.push(Block::Note("Off day — no shooting.".into()));
            if let Some(n) = nonblank(d.notes.as_deref()) {
                doc.push(Block::Paragraph(format!("Day notes: {n}")));
            }
            continue;
        }
        let sum = board.day_summary(d);
        let mut line = vec![
            sum.total_label.clone(),
            format!("{} pages", sum.pages_label),
        ];
        if sum.over_target {
            line.push(format!("over the {} target", minutes(sum.target_minutes)));
        }
        if let Some(cs) = board.call_sheet_brief(d) {
            line.push(format!("Call sheet: {}", cs.status));
        }
        doc.push(Block::Note(line.join(" · ")));
        if let Some(n) = nonblank(d.notes.as_deref()) {
            doc.push(Block::Paragraph(format!("Day notes: {n}")));
        }
        let items = board.day_items(&d.id);
        if items.is_empty() {
            doc.push(Block::Paragraph("No scenes scheduled.".into()));
            continue;
        }
        let mut t = Table::new("", pdf_columns(notes));
        for it in items {
            match it {
                Item::Strip(s) => t.push(strip_row(&strip_text(&board, s))),
                Item::Marker(m) => {
                    let mut r = vec![String::new(), String::new(), String::new(), marker_text(m)];
                    r.resize(t.columns.len(), String::new());
                    t.push(r);
                }
            }
        }
        doc.push(Block::Table(t));
    }
    if unscheduled {
        doc.push(Block::Heading {
            text: format!("Unscheduled ({})", pool.len()),
            level: 2,
        });
        if pool.is_empty() {
            doc.push(Block::Paragraph("Every scene is scheduled.".into()));
        } else {
            let mut t = Table::new("", pdf_columns(notes));
            for s in &pool {
                t.push(strip_row(&strip_text(&board, s)));
            }
            doc.push(Block::Table(t));
        }
    }

    // ---- spreadsheet
    let mut cols = vec![
        Column::new("Day"),
        Column::new("Date"),
        Column::right("Order"),
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
    let row =
        |day: &str, date: &str, order: usize, item: &str, t: Option<&StripText>, label: &str| {
            let mut r = vec![
                day.to_string(),
                date.to_string(),
                order.to_string(),
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
            Column::new("Locations"),
            Column::new("Cast"),
            Column::new("Call sheet"),
            Column::new("Day notes"),
        ],
    );
    for d in &days {
        let label = d.label();
        let date = d.date.as_deref().map(date_short).unwrap_or_default();
        if d.off {
            sheet.push(row(&label, &date, 0, "Off Day", None, "Off day"));
        } else {
            for (i, it) in board.day_items(&d.id).into_iter().enumerate() {
                match it {
                    Item::Strip(s) => sheet.push(row(
                        &label,
                        &date,
                        i + 1,
                        "Scene",
                        Some(&strip_text(&board, s)),
                        "",
                    )),
                    Item::Marker(m) => {
                        sheet.push(row(&label, &date, i + 1, &m.label, None, &marker_text(m)))
                    }
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
            "Scene",
            Some(&strip_text(&board, s)),
            "",
        ));
    }

    let mut out = Output::new(doc, HINT_TABLES);
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
