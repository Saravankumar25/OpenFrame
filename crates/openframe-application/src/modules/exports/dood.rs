//! Day Out of Days export (Screenplay and Production Depth spec section 28:
//! "PDF export, spreadsheet export"; feature DOOD-001). A snapshot of the
//! derived report with the user's corrections; it changes nothing.
//!
//! Reproducible: the document carries the date of the schedule's last change,
//! never the export clock, so the same data gives byte-identical files.

use openframe_domain::{Actor, AppError, AppResult};
use openframe_import_export::report::{
    Align, Banner, Block, Cell, CellItem, Col, Column, Document, HeaderStyle, Legend, LegendItem,
    Masthead, Metric, Metrics, Rgb, Row, Section, Side, StyledTable, Table, Txt, pal,
};
use serde::Deserialize;
use ts_rs::TS;

use super::style::{cell, cell_bold, day_number, nb, short_day_parts};
use super::{
    Fmt, HINT_TABLES, Job, Output, date_from_ms, deliver, destination, footer, plural,
    require_export,
};
use crate::core::AppCore;
use crate::modules::schedule::board::revised_at;
use crate::modules::schedule::dood::{DoodDay, ScheduleDoodArgs, check_args, report};

/// Schedule-day columns per PDF band (off days count). A band is a layout unit, not a data limit:
/// longer schedules continue in further bands, each on its own page, so the day headers and
/// status cells stay readable at print size. Raising it needs a rendered proof at print size.
const PDF_DAYS_PER_TABLE: usize = 7;

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

/// "Days 01 to 06 + 1 off day": the range a band covers.
fn band_title(chunk: &[DoodDay]) -> String {
    let nums: Vec<i64> = chunk
        .iter()
        .filter(|d| !d.is_off_day)
        .filter_map(|d| day_number(&d.short_label).and_then(|n| n.parse().ok()))
        .collect();
    let off = chunk.iter().filter(|d| d.is_off_day).count();
    let mut s = match (nums.first(), nums.last()) {
        (Some(a), Some(b)) if a != b => format!("Days {a:02} to {b:02}"),
        (Some(a), _) => format!("Day {a:02}"),
        _ => "Off days".to_string(),
    };
    if off > 0 && !nums.is_empty() {
        s.push_str(&format!(" + {}", plural(off, "off day", "off days")));
    }
    s
}

/// Header of one day column: shoot number, date and weekday, in the schedule's own order.
fn day_header(d: &DoodDay) -> Cell {
    let off = d.is_off_day;
    let first = if off {
        "OFF DAY".to_string()
    } else {
        day_number(&d.short_label)
            .map(|n| format!("SHOOT {n}"))
            .unwrap_or_else(|| d.short_label.to_uppercase())
    };
    let mut c = Cell::new()
        .align(Align::Center)
        .middle()
        .item(CellItem::Text(
            Txt::new(first).size(7.0).bold().track(0.8).color(if off {
                pal::SLATE
            } else {
                pal::ORANGE
            }),
        ));
    if let Some((wd, day, mon)) = d.date.as_deref().and_then(short_day_parts) {
        c = c
            .item(CellItem::Text(
                Txt::new(day)
                    .size(17.0)
                    .bold()
                    .color(if off { pal::SLATE } else { pal::NAVY })
                    .leading(1.15),
            ))
            .item(CellItem::Text(
                Txt::new(format!("{wd} · {mon}"))
                    .size(6.6)
                    .bold()
                    .track(0.6)
                    .color(pal::SLATE),
            ));
    }
    if off {
        c = c.fill(Rgb::hex(0xe4e8ee));
    }
    c
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

    // ---- PDF: readable day bands (at most PDF_DAYS_PER_TABLE schedule-day columns each), then a
    // separate totals table for the whole schedule.
    let cell_text = |code: &str, overridden: bool| {
        if overridden {
            format!("{code}*")
        } else {
            code.to_string()
        }
    };
    let mut doc = Document::new(document)
        .landscape(true)
        .footer(footer(&project, document));
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
            Metric::new(shoot_days.to_string(), "Shooting days"),
            Metric::new(view.rows.len().to_string(), "Cast members"),
            Metric::new(off_days.to_string(), "Off days"),
            Metric::new(corrected.to_string(), "Manual corrections"),
        ],
        compact: true,
    }));
    let bands: Vec<&[DoodDay]> = view.days.chunks(PDF_DAYS_PER_TABLE).collect();
    doc.push(Block::Section(Section::new(
        "Cast commitments by shooting day",
        Some(format!(
            "{} · up to {PDF_DAYS_PER_TABLE} schedule-day columns per page",
            plural(bands.len(), "band", "bands")
        )),
    )));
    let legend = Legend {
        title: Some("Code legend · meanings stay readable without colour".into()),
        columns: 0,
        items: view
            .legend
            .iter()
            .map(|c| {
                let (fill, color) = code_colors(&c.code, false);
                LegendItem {
                    swatch: fill.unwrap_or(Rgb::hex(0xffffff)),
                    border: Some(pal::LINE),
                    code: Some((c.code.clone(), color)),
                    label: c.meaning.clone(),
                }
            })
            .chain(std::iter::once(LegendItem {
                swatch: Rgb::hex(0xffffff),
                border: Some(pal::LINE),
                code: None,
                label: "Blank: no code for that day".into(),
            }))
            .collect(),
    };
    for (bi, chunk) in bands.iter().enumerate() {
        let offset = bi * PDF_DAYS_PER_TABLE;
        if bi > 0 {
            doc.push(Block::PageBreak);
        }
        let mut b = Banner::new(band_title(chunk));
        b.eyebrow = Some(format!("Band {} of {}", bi + 1, bands.len()));
        b.right = Some(plural(
            chunk.len(),
            "schedule-day column",
            "schedule-day columns",
        ));
        b.keep_next = 140.0;
        doc.push(Block::Banner(b));
        // Day columns keep one width in every band; the cast column takes what is left.
        let mut cols = vec![Col::pt("#", 26.0), Col::weight("Cast member", 1.0)];
        cols.extend(chunk.iter().map(|d| Col::pt(d.short_label.clone(), 80.0)));
        let mut tbl = StyledTable::new(cols);
        tbl.grid = true;
        tbl.boxed = true;
        tbl.pad_y = 5.5;
        tbl.continued = Some(band_title(chunk));
        let mut header = vec![
            Cell::text(Txt::new("#").size(7.0).bold().color(pal::SLATE)).middle(),
            Cell::text(
                Txt::new("CAST MEMBER")
                    .size(7.0)
                    .bold()
                    .track(0.8)
                    .color(pal::SLATE),
            )
            .middle(),
        ];
        header.extend(chunk.iter().map(day_header));
        tbl.header_cells = Some(header);
        for r in &view.rows {
            let mut cells = vec![
                Cell::new()
                    .item(CellItem::Tile(r.number.to_string()))
                    .middle(),
                Cell::text(
                    Txt::new(r.label.to_uppercase())
                        .size(8.8)
                        .bold()
                        .color(pal::NAVY)
                        .leading(1.25),
                )
                .middle(),
            ];
            for (c, d) in r.cells[offset..offset + chunk.len()]
                .iter()
                .zip(chunk.iter())
            {
                let (fill, color) = code_colors(&c.code, d.is_off_day);
                let mut cell = Cell::new().align(Align::Center).middle();
                if !c.code.is_empty() {
                    cell = cell.item(CellItem::Text(
                        Txt::new(cell_text(&c.code, c.overridden))
                            .size(11.0)
                            .bold()
                            .color(color)
                            .align(Align::Center),
                    ));
                }
                if let Some(f) = fill {
                    cell = cell.fill(f);
                }
                cells.push(cell);
            }
            tbl.rows.push(Row::new(cells));
        }
        doc.push(Block::Styled(tbl));
        doc.push(Block::Legend(legend.clone()));
    }

    // Totals for the whole schedule, in their own table.
    doc.push(Block::Section(Section::new(
        "Cast totals across the full schedule",
        Some("Totals cover the whole schedule, not only the displayed range".into()),
    )));
    let mut totals = StyledTable::new(vec![
        Col::pt("#", 26.0),
        Col::weight("Cast member", 2.2),
        Col::weight("Work", 0.6).align(Align::Right),
        Col::weight("Hold", 0.6).align(Align::Right),
        Col::weight("Travel", 0.6).align(Align::Right),
        Col::weight("Other", 0.6).align(Align::Right),
        Col::weight("Start", 1.0),
        Col::weight("Finish", 1.0),
    ]);
    totals.header = HeaderStyle::Navy;
    totals.boxed = true;
    totals.zebra = true;
    totals.continued = Some("Cast totals".into());
    let num = |n: i64| {
        Cell::text(
            Txt::new(n.to_string())
                .size(9.4)
                .bold()
                .color(pal::NAVY)
                .align(Align::Right),
        )
    };
    for r in &view.rows {
        totals.rows.push(Row::new(vec![
            Cell::new().item(CellItem::Tile(r.number.to_string())),
            Cell::text(Txt::new(r.label.clone()).size(9.0).bold().color(pal::NAVY)),
            num(r.work_days),
            num(r.hold_days),
            num(r.travel_days),
            num(r.other_days),
            cell(r.start_label.clone().unwrap_or_default()),
            cell(r.finish_label.clone().unwrap_or_default()),
        ]));
    }
    doc.push(Block::Styled(totals));

    // Correction details, only when something was corrected.
    if corrected > 0 {
        doc.push(Block::Section(Section::new(
            "Correction details",
            Some("Entered by hand; the schedule alone gives the other code".into()),
        )));
        let mut ct = StyledTable::new(vec![
            Col::weight("Cast member", 1.6),
            Col::weight("Day", 1.4),
            Col::pt("Shown", 50.0),
            Col::pt("Schedule", 56.0),
            Col::weight("Note", 2.0),
        ]);
        ct.continued = Some("Correction details".into());
        for r in &view.rows {
            for (c, d) in r.cells.iter().zip(view.days.iter()) {
                if !c.overridden {
                    continue;
                }
                ct.rows.push(Row::new(vec![
                    cell_bold(r.label.clone()),
                    cell(d.title.clone()),
                    cell_bold(c.code.clone()),
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
