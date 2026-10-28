//! Day Out of Days export (Screenplay and Production Depth spec section 28:
//! "PDF export, spreadsheet export"; feature DOOD-001). A snapshot of the
//! derived report with the user's corrections; it changes nothing.
//!
//! Reproducible: the document carries the date of the schedule's last change,
//! never the export clock, so the same data gives byte-identical files.

use openframe_domain::{Actor, AppError, AppResult};
use openframe_import_export::report::{
    Align, Block, Cell, Col, Column, Document, HeaderStyle, Masthead, Matrix, MatrixCode,
    MatrixCount, MatrixDay, MatrixKey, MatrixRow, MatrixTotals, Metric, Metrics, Rgb, Row, Side,
    StyledTable, Table, Txt, pal,
};
use serde::Deserialize;
use time::Weekday;
use ts_rs::TS;

use super::style::{cell, cell_bold, day_number, nb, short_day_parts};
use super::{
    Fmt, HINT_TABLES, Job, Output, date_from_ms, deliver, destination, plural, require_export,
};
use crate::core::AppCore;
use crate::modules::schedule::board::revised_at;
use crate::modules::schedule::dood::{DoodDay, ScheduleDoodArgs, check_args, report};
use crate::modules::schedule::fmt::parse_date;

/// Fill and text colour of a status code. Colour reinforces the printed code, never replaces it.
fn code_colors(code: &str, off_day: bool) -> (Option<Rgb>, Rgb) {
    let h = Rgb::hex;
    match code {
        "SW" => (Some(h(0x1d78b0)), h(0xffffff)),
        "W" => (Some(h(0xb9e0ee)), h(0x1b4f66)),
        "WF" => (Some(h(0x62cf9a)), h(0x14472f)),
        "SWF" => (Some(h(0x16809f)), h(0xffffff)),
        "H" => (Some(h(0xffd199)), h(0x7a4300)),
        "T" => (Some(h(0xd5c4f2)), h(0x4a2f7a)),
        "R" => (Some(h(0xf7e07a)), h(0x5a4a00)),
        "F" => (Some(h(0xf4b6d0)), h(0x6b1f45)),
        "D" => (Some(h(0xeab0b0)), h(0x7a1f1f)),
        "-" => (Some(h(0xeceff3)), pal::SLATE),
        _ => (off_day.then(|| h(0xeceff3)), pal::SLATE),
    }
}

/// A code as the matrix draws it: the text, the fill and the shape that survives a black and
/// white print (caps on the edges that start and finish work, an outline on a hold).
fn code_look(code: &str, corrected: bool) -> MatrixCode {
    let (fill, color) = code_colors(code, false);
    let mut c = MatrixCode::new(
        if corrected {
            format!("{code}*")
        } else {
            code.to_string()
        },
        fill,
        color,
    );
    c.cap_left = matches!(code, "SW" | "SWF");
    c.cap_right = matches!(code, "WF" | "SWF");
    c.hollow = (code == "H").then_some(color);
    c.corrected = corrected;
    c
}

/// "LOW TIDE RADIO · DAY OUT OF DAYS": names the production on every page of a long matrix.
fn running_header(project: &str) -> String {
    match nb(project) {
        Some(p) => format!("{} · DAY OUT OF DAYS", p.to_uppercase()),
        None => "DAY OUT OF DAYS".into(),
    }
}

fn is_work(code: &str) -> bool {
    matches!(code, "SW" | "W" | "WF" | "SWF")
}

/// One day column of the matrix: shoot number, date, weekday and the month it belongs to.
fn matrix_day(d: &DoodDay) -> MatrixDay {
    let parts = d.date.as_deref().and_then(short_day_parts);
    let parsed = d.date.as_deref().and_then(|s| parse_date(s).ok());
    MatrixDay {
        top: if d.is_off_day {
            "OFF".into()
        } else {
            day_number(&d.short_label).unwrap_or_else(|| d.short_label.to_uppercase())
        },
        date: parts
            .as_ref()
            .map(|(_, day, _)| format!("{day:0>2}"))
            .unwrap_or_default(),
        weekday: parts.map(|(wd, _, _)| wd).unwrap_or_default(),
        month: parsed.map(|p| {
            let name = p.month().to_string().to_uppercase();
            (
                format!("{name} {}", p.year()),
                name.chars().take(3).collect(),
            )
        }),
        // Saturday and Sunday are a design default, not a rule of the schedule.
        weekend: !d.is_off_day
            && matches!(
                parsed.map(|p| p.weekday()),
                Some(Weekday::Saturday | Weekday::Sunday)
            ),
        off: d.is_off_day,
    }
}

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

    // ---- PDF: one matrix, the cast frozen on the left of every column group, the legend in the
    // repeating header and the totals inside the matrix.
    let cell_text = |code: &str, overridden: bool| {
        if overridden {
            format!("{code}*")
        } else {
            code.to_string()
        }
    };
    let mut doc = Document::new(document)
        .landscape(true)
        .running_header(Some(running_header(&project)));
    let shoot_days = view.days.iter().filter(|d| !d.is_off_day).count();
    let off_days = view.days.len() - shoot_days;
    let mut mast = Masthead::new("DAY OUT OF DAYS")
        .eyebrow("Production planning · Cast commitments", pal::ORANGE);
    if let Some(p) = nb(&project) {
        mast.subtitle = Some(format!("{p} · Revised {}", date_from_ms(revised)));
    }
    mast.side = Some(Side::Facts(vec![
        ("Schedule".into(), board.schedule.name.clone()),
        ("Production source".into(), source_label.clone()),
        ("Displayed range".into(), range.clone()),
        (
            "Corrections".into(),
            if corrected == 0 {
                "None; every cell comes from the schedule".to_string()
            } else {
                format!(
                    "{} entered by hand (marked *)",
                    plural(corrected, "cell", "cells")
                )
            },
        ),
    ]));
    doc.push(Block::Masthead(mast));
    doc.push(Block::Metrics(Metrics {
        items: vec![
            Metric::new(
                shoot_days.to_string(),
                if shoot_days == 1 {
                    "Shooting day"
                } else {
                    "Shooting days"
                },
            ),
            Metric::new(
                view.rows.len().to_string(),
                if view.rows.len() == 1 {
                    "Cast member"
                } else {
                    "Cast members"
                },
            ),
            Metric::new(
                off_days.to_string(),
                if off_days == 1 { "Off day" } else { "Off days" },
            ),
            Metric::new(
                corrected.to_string(),
                if corrected == 1 {
                    "Manual correction"
                } else {
                    "Manual corrections"
                },
            ),
        ],
        compact: true,
    }));

    // The legend: the codes of the schedule's style, then the three marks of the matrix.
    let mut legend: Vec<MatrixKey> = view
        .legend
        .iter()
        .map(|c| MatrixKey {
            swatch: code_look(&c.code, false),
            label: c.meaning.clone(),
        })
        .collect();
    let mut blank = MatrixCode::new("", Some(Rgb::hex(0xffffff)), pal::SLATE);
    blank.border = Some(pal::LINE);
    legend.push(MatrixKey {
        swatch: blank,
        label: "Blank: no code for that day".into(),
    });
    legend.push(MatrixKey {
        swatch: code_look("W", true),
        label: "Entered by hand (correction)".into(),
    });
    legend.push(MatrixKey {
        swatch: MatrixCode::new("OFF", Some(Rgb::hex(0xe9edf3)), pal::SLATE),
        label: "Off day".into(),
    });

    let days: Vec<MatrixDay> = view.days.iter().map(matrix_day).collect();
    let rows: Vec<MatrixRow> = view
        .rows
        .iter()
        .map(|r| {
            let (name, detail) = match r.actor.as_deref().and_then(nb) {
                Some(actor) => (
                    actor.to_string(),
                    nb(&r.character)
                        .map(|c| format!("({})", c.to_uppercase()))
                        .unwrap_or_default(),
                ),
                None => (r.character.clone(), String::new()),
            };
            // "Shoot Day 3" prints as "Day 3" in the narrow totals columns.
            let short = |l: &Option<String>| {
                l.as_deref()
                    .map(|l| l.strip_prefix("Shoot ").unwrap_or(l).to_string())
                    .unwrap_or_default()
            };
            MatrixRow {
                number: r.number.to_string(),
                name,
                detail,
                cells: r
                    .cells
                    .iter()
                    .map(|c| {
                        if c.code.is_empty() {
                            MatrixCode::new("", None, pal::SLATE)
                        } else {
                            code_look(&c.code, c.overridden)
                        }
                    })
                    .collect(),
                totals: vec![
                    r.work_days.to_string(),
                    r.hold_days.to_string(),
                    r.travel_days.to_string(),
                    r.other_days.to_string(),
                    short(&r.start_label),
                    short(&r.finish_label),
                ],
            }
        })
        .collect();
    // Per-day counts of the cast shown, and the sums of the totals columns.
    let per_day = |f: &dyn Fn(&str) -> bool| -> Vec<String> {
        (0..view.days.len())
            .map(|j| {
                view.rows
                    .iter()
                    .filter(|r| f(&r.cells[j].code))
                    .count()
                    .to_string()
            })
            .collect()
    };
    let sum = |f: &dyn Fn(&crate::modules::schedule::dood::DoodRow) -> i64| -> String {
        view.rows.iter().map(f).sum::<i64>().to_string()
    };
    let blanks = |at: usize, v: String| -> Vec<String> {
        let mut out = vec![String::new(); 6];
        out[at] = v;
        out
    };
    let counts = vec![
        MatrixCount {
            label: "Cast working".into(),
            values: per_day(&is_work),
            totals: blanks(0, sum(&|r| r.work_days)),
        },
        MatrixCount {
            label: "On hold (H)".into(),
            values: per_day(&|c| c == "H"),
            totals: blanks(1, sum(&|r| r.hold_days)),
        },
        MatrixCount {
            label: "Travel, rehearsal, fitting, drop".into(),
            values: per_day(&|c| matches!(c, "T" | "R" | "F" | "D")),
            totals: {
                let mut t = blanks(2, sum(&|r| r.travel_days));
                t[3] = sum(&|r| r.other_days);
                t
            },
        },
    ];
    let mut hint = format!(
        "{} + {}",
        plural(shoot_days, "shooting day", "shooting days"),
        plural(off_days, "off day", "off days")
    );
    if view.days.len() < view.all_days.len() {
        hint.push_str(" · Start and Finish cover the whole schedule");
    }
    doc.push(Block::Matrix(Matrix {
        title: "Cast commitments by shooting day".into(),
        hint: Some(hint),
        legend_label: "Code legend".into(),
        legend,
        line_labels: ["Shoot day".into(), "Date".into(), "Weekday".into()],
        frozen_labels: ["#".into(), "Cast member".into()],
        days,
        rows,
        counts,
        totals: Some(MatrixTotals {
            heading: "Totals".into(),
            columns: vec![
                ("Work".into(), 24.0, false),
                ("Hold".into(), 24.0, false),
                ("Travel".into(), 28.0, false),
                ("Other".into(), 26.0, false),
                ("Start".into(), 28.0, true),
                ("Finish".into(), 30.0, true),
            ],
        }),
        guide_every: 5,
    }));

    // Correction details, only when something was corrected.
    if corrected > 0 {
        let mut ct = StyledTable::new(vec![
            Col::weight("Cast member", 1.6),
            Col::weight("Day", 1.4),
            Col::pt("Shown", 50.0),
            Col::pt("Schedule", 56.0),
            Col::weight("Note", 2.0),
        ]);
        ct.title = Some("Correction details".into());
        ct.title_hint = Some("Entered by hand; the schedule alone gives the other code".into());
        ct.header = HeaderStyle::Navy;
        ct.boxed = true;
        for r in &view.rows {
            for (c, d) in r.cells.iter().zip(view.days.iter()) {
                if !c.overridden {
                    continue;
                }
                let (fill, color) = code_colors(&c.code, false);
                let mut shown = Cell::text(
                    Txt::new(c.code.clone())
                        .size(8.6)
                        .bold()
                        .color(color)
                        .align(Align::Center),
                );
                if let Some(f) = fill {
                    shown = shown.fill(f);
                }
                // The source label reads "(Character)" when the actor is blank; print the name alone.
                let who = if r.actor.as_deref().and_then(nb).is_some() {
                    r.label.clone()
                } else {
                    r.character.clone()
                };
                ct.rows.push(Row::new(vec![
                    cell_bold(who),
                    cell(d.title.clone()),
                    shown.middle(),
                    cell(if c.derived_code.is_empty() {
                        "blank".to_string()
                    } else {
                        c.derived_code.clone()
                    }),
                    cell(c.note.clone().unwrap_or_default()),
                ]));
            }
        }
        doc.push(Block::Styled(ct));
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
