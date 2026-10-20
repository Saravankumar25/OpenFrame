//! Daily View export (Production > Daily View, FSD 57). One shooting day as a printed page that
//! shows exactly what the screen shows: the title, the date, the scenes, the location, the cast,
//! the call sheet, the important notes, the important breakdown items and the scene order.
//!
//! The data comes from the same read as the screen (`schedule::ops::daily_view`), so the document
//! cannot drift from the page. Reproducible: the document carries the date of the schedule's last
//! change, never the export clock. The day is only read; nothing in the project changes.

use openframe_domain::{Actor, AppError, AppResult};
use openframe_import_export::report::{
    Block, Chip, Document, Masthead, Metric, Metrics, Section, Side, Tone, pal,
};
use serde::Deserialize;
use ts_rs::TS;

use super::style::nb;
use super::{Fmt, Job, Output, date_from_ms, deliver, destination, footer, plural, require_export};
use crate::core::AppCore;
use crate::modules::schedule::board::{current_schedule, project_title, revised_at};
use crate::modules::schedule::fmt::minutes;
use crate::modules::schedule::ops::{ScheduleDailyDay, daily_view};

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

/// The sections in the order of the screen. The wording matches `daily.tsx`.
fn sections(d: &ScheduleDailyDay) -> Vec<(&'static str, Vec<Part>)> {
    let s = &d.summary;
    let number = |n: Option<i64>| n.map_or("Not numbered".to_string(), |n| n.to_string());

    let scenes = if d.scenes.is_empty() {
        vec![Part::Line("No scenes scheduled.".into())]
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
        vec![Part::Line("Location missing.".into())]
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
        vec![Part::Line("No cast confirmed for these scenes.".into())]
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
                    "Not entered yet".to_string()
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
            vec![facts(&[
                ("Status", word.to_string()),
                ("Details", line.into()),
            ])]
        }
        None if d.is_off_day => vec![Part::Line("Off days have no call sheet.".into())],
        None => vec![Part::Line("No call sheet yet for this day.".into())],
    };

    let mut notes = Vec::new();
    match d.notes.as_deref().and_then(nb) {
        Some(n) => notes.extend(n.lines().map(|l| Part::Line(l.to_string()))),
        None => notes.push(Part::Line("No day notes.".into())),
    }
    if !d.markers.is_empty() {
        let marks: Vec<String> = d
            .markers
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
            .collect();
        notes.push(Part::Line(marks.join(" · ")));
    }

    let items = if d.breakdown_items.is_empty() {
        vec![Part::Line(
            "No confirmed props, wardrobe or other items for these scenes.".into(),
        )]
    } else {
        let names: Vec<&str> = d.breakdown_items.iter().map(|i| i.name.as_str()).collect();
        vec![Part::Line(names.join(" · "))]
    };

    let order = if d.scenes.is_empty() {
        vec![Part::Line("Nothing scheduled yet.".into())]
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

    let mut doc = Document::new(&document).footer(footer(&project, &document));
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
    for (name, parts) in &secs {
        doc.push(Block::Section(Section::new(*name, None)));
        for p in parts {
            match p {
                Part::Facts(rows) => doc.push(Block::KeyValues(rows.clone())),
                Part::Line(l) => doc.push(Block::Paragraph(l.clone())),
                Part::Numbered(items) => {
                    for (i, it) in items.iter().enumerate() {
                        doc.push(Block::Paragraph(format!("{}. {it}", i + 1)));
                    }
                }
            }
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
