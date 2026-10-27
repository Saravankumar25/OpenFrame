//! Production reports export (FSD §58.1, §110; UX §3.33 mock 149): any of the
//! six report types, live (generated on demand from the current data) or a
//! saved report snapshot, as PDF, CSV or XLSX.
//!
//! The PDF follows the PDF design package v2 (production reports, `Report_Scene.md` F.1 for the
//! shell shared by all six): a compact head with a side panel, context bands, a metrics strip, one
//! table with status chips that carry a shape as well as a word, and a running header on every
//! page of a report that runs past one page. CSV and XLSX print the data as it is stored.

use openframe_domain::{Actor, AppError, AppResult};
use openframe_import_export::report::{
    Align, BRAND_LINE, Block, Callout, CalloutCol, Cell, CellItem, Chip, Col, Column, Document,
    HeaderStyle, Mark, Metric, MetricStrip, NoteBand, ReportHead, Rgb, Rich, Row, Run, StyledTable,
    Table, Tone, Txt, pal,
};
use rusqlite::OptionalExtension;
use serde::Deserialize;
use serde_json::json;
use ts_rs::TS;

use super::style::nb;
use super::{
    Fmt, HINT_TABLES, Job, Output, date_from_ms, deliver, destination, plural, require_export,
};
use crate::core::AppCore;
use crate::modules::schedule::board::project_title;
use crate::modules::schedule::docs::ReportData;

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportReportArgs {
    /// "pdf" | "csv" | "xlsx"
    pub format: String,
    pub path: String,
    /// Live report: "scene" | "location" | "cast_scene" | "prop" | "schedule" | "breakdown_completeness".
    #[serde(default)]
    #[ts(optional)]
    pub report_type: Option<String>,
    /// Live report text filter (as shown in the Reports view).
    #[serde(default)]
    #[ts(optional)]
    pub filter: Option<String>,
    /// A saved report snapshot instead of a live report.
    #[serde(default)]
    #[ts(optional)]
    pub report_id: Option<String>,
}

/// Table text: 8.4 pt on a 1.28 line.
const BODY: f32 = 8.4;
const LEAD: f32 = 1.28;
/// Zero counts and the dash of an off day.
const MUTED: Rgb = Rgb::hex(0x66728a);
/// A flexible column of a known report never goes narrower than this; an unknown one never
/// narrower than [`UNKNOWN`].
const FLOOR: f32 = 44.0;
const UNKNOWN: f32 = 76.0;

// ------------------------------------------------------------------ columns

/// How a column is laid out: a fixed width, or a share of what is left (weight) down to a floor.
#[derive(Clone, Copy)]
struct Spec {
    width: f32,
    fixed: bool,
    floor: f32,
}

const fn fixed(w: f32) -> Spec {
    Spec {
        width: w,
        fixed: true,
        floor: w,
    }
}

const fn flex(w: f32) -> Spec {
    Spec {
        width: w,
        fixed: false,
        floor: FLOOR,
    }
}

/// The design width of a column of a known report (section E of each report spec).
fn known(report_type: &str, header: &str) -> Option<Spec> {
    Some(match (report_type, header) {
        ("scene", "Sc") => fixed(34.0),
        ("scene", "Heading") => flex(186.0),
        ("scene", "I/E · D/N") => fixed(60.0),
        ("scene", "Pages") => fixed(40.0),
        ("scene", "Location") => flex(150.0),
        ("scene", "Cast") => flex(140.0),
        ("scene", "Scheduled") => fixed(90.0),
        ("location", "Location") => flex(120.0),
        ("location", "Status") => fixed(90.0),
        ("location", "Address") => flex(102.0),
        ("location", "Scenes") => flex(58.0),
        ("location", "Shooting days") => flex(150.0),
        ("cast_scene", "Character") => flex(108.0),
        ("cast_scene", "Actor") => flex(96.0),
        ("cast_scene", "Scenes") => flex(104.0),
        ("cast_scene", "Scene count") => fixed(44.0),
        ("cast_scene", "Shooting days") => flex(168.0),
        ("prop", "Prop") => flex(250.0),
        ("prop", "Scenes") => flex(200.0),
        ("prop", "Scene count") => fixed(70.0),
        ("schedule", "Day") => fixed(84.0),
        ("schedule", "Date") => fixed(86.0),
        ("schedule", "Scenes") => flex(96.0),
        ("schedule", "Locations") => flex(178.0),
        ("schedule", "Est. time") => fixed(76.0),
        ("breakdown_completeness", "Sc") => fixed(36.0),
        ("breakdown_completeness", "Heading") => flex(222.0),
        ("breakdown_completeness", "Confirmed") => fixed(60.0),
        ("breakdown_completeness", "Suggested (unreviewed)") => fixed(90.0),
        ("breakdown_completeness", "Status") => fixed(112.0),
        _ => return None,
    })
}

/// Column widths for `total` points: fixed columns keep their width, flexible ones share the rest
/// by weight and none goes under its floor. When even the floors do not fit, everything shrinks
/// evenly so the table stays inside the page.
fn widths(specs: &[Spec], total: f32) -> Vec<f32> {
    let mut out: Vec<Option<f32>> = specs.iter().map(|s| s.fixed.then_some(s.width)).collect();
    loop {
        let used: f32 = out.iter().flatten().sum();
        let weights: f32 = specs
            .iter()
            .zip(&out)
            .filter(|(_, o)| o.is_none())
            .map(|(s, _)| s.width)
            .sum();
        let share = |s: &Spec| (total - used).max(0.0) * s.width / weights.max(0.01);
        let under: Vec<usize> = (0..specs.len())
            .filter(|&i| out[i].is_none() && share(&specs[i]) < specs[i].floor)
            .collect();
        if under.is_empty() {
            for i in 0..specs.len() {
                if out[i].is_none() {
                    out[i] = Some(share(&specs[i]));
                }
            }
            break;
        }
        for i in under {
            out[i] = Some(specs[i].floor);
        }
    }
    let mut w: Vec<f32> = out.into_iter().flatten().collect();
    let sum: f32 = w.iter().sum();
    if sum > total {
        w.iter_mut().for_each(|x| *x *= total / sum);
    }
    w
}

/// Columns whose cells hold counts: right aligned and bold when every filled cell is a number.
/// The text columns of the known reports (scene numbers, headings) are never counted.
fn is_count_column(report_type: &str, header: &str, rows: &[Vec<String>], i: usize) -> bool {
    if known(report_type, header).is_some()
        && !matches!(
            header,
            "Scene count" | "Confirmed" | "Suggested (unreviewed)"
        )
    {
        return false;
    }
    let mut any = false;
    for r in rows {
        let v = r.get(i).map(|s| s.trim()).unwrap_or("");
        if v.is_empty() {
            continue;
        }
        if v.parse::<f64>().is_err() {
            return false;
        }
        any = true;
    }
    any
}

// ------------------------------------------------------------------ cells

/// The chip tone of a status word.
fn status_tone(v: &str) -> Tone {
    match v {
        "Reviewed" | "Confirmed" | "Required" => Tone::Ok,
        "Needs review" | "Shortlisted" | "Searching" | "Unscheduled" | "Not cast yet" => Tone::Warn,
        "In progress" | "From heading" => Tone::Info,
        "Missing" | "Rejected" | "Not started" => Tone::Danger,
        _ => Tone::Neutral,
    }
}

/// The shape a tone prints before its word, so a status reads without colour.
fn mark_of(tone: Tone) -> Option<Mark> {
    match tone {
        Tone::Ok => Some(Mark::Disc),
        Tone::Info => Some(Mark::RingDot),
        Tone::Warn => Some(Mark::Ring),
        Tone::Danger => Some(Mark::Square),
        _ => None,
    }
}

/// A status chip: the word in capitals with the shape of its tone.
fn status_chip(word: &str, tone: Tone) -> Cell {
    let mut chip = Chip::label(word, tone);
    chip.mark = mark_of(tone);
    Cell::new().item(CellItem::Chips(vec![chip]))
}

fn body(t: &str) -> Txt {
    Txt::new(t.to_string())
        .size(BODY)
        .color(pal::INK)
        .leading(LEAD)
}

/// A cell of plain text; `quiet` rows (off days) print slate.
fn plain(v: &str, quiet: bool) -> Cell {
    Cell::text(body(v).color(if quiet { pal::SLATE } else { pal::INK }))
}

/// The row key: heading, location, character, prop or day.
fn key(v: &str) -> Cell {
    Cell::text(body(v).bold().color(pal::NAVY))
}

/// "Shoot Day 12 · Sat 23 Jan 2027" as a bold day over a slate date (one line when no date).
fn day_and_date(v: &str) -> Cell {
    let (day, date) = v.split_once(" · ").unwrap_or((v, ""));
    let mut r = Rich::new(BODY).leading(LEAD).gap(0.0);
    r = r.para(vec![Run::new(day, true, pal::NAVY)]);
    if !date.is_empty() {
        r = r.para(vec![Run::new(date, false, pal::SLATE)]);
    }
    Cell::new().item(CellItem::Rich(r))
}

/// A list of shoot days, one per line with the day label bold.
fn day_list(v: &str) -> Cell {
    let mut r = Rich::new(BODY).leading(1.25).gap(1.4);
    for d in v.split("; ").filter(|d| !d.trim().is_empty()) {
        let runs = match d.split_once(" · ") {
            Some((day, date)) => vec![
                Run::new(day, true, pal::NAVY),
                Run::new(format!("· {date}"), false, pal::INK),
            ],
            None => vec![Run::new(d, true, pal::NAVY)],
        };
        r = r.para(runs);
    }
    if r.paras.is_empty() {
        return Cell::empty();
    }
    Cell::new().item(CellItem::Rich(r))
}

/// What a row needs to be drawn besides its cells.
#[derive(Default)]
struct RowLook {
    stripe: Option<Rgb>,
    quiet: bool,
}

/// One report cell, by the column it sits in.
fn report_cell(report_type: &str, header: &str, v: &str, count: bool, row: &[String]) -> Cell {
    let quiet = report_type == "schedule" && row.first().is_some_and(|d| d == "Off Day");
    if v.trim().is_empty() {
        return Cell::empty();
    }
    match (report_type, header) {
        (_, "Sc") => Cell::new().item(CellItem::Tile(v.to_string())),
        (_, "Heading" | "Character" | "Prop") | ("location", "Location") => key(v),
        ("scene", "I/E · D/N") => Cell::new().item(CellItem::Chips(
            v.split(" · ")
                .map(|p| Chip::label(p, Tone::Neutral))
                .collect(),
        )),
        ("scene", "Scheduled") => {
            if v == "Unscheduled" {
                status_chip(v, Tone::Warn)
            } else {
                day_and_date(v)
            }
        }
        (_, "Shooting days") => day_list(v),
        (_, "Status") => status_chip(v, status_tone(v)),
        ("cast_scene", "Actor") if v == "Not cast yet" => status_chip(v, Tone::Warn),
        ("schedule", "Day") => match v {
            "Unscheduled" => status_chip(v, Tone::Warn),
            "Off Day" => plain(v, true),
            _ => key(v),
        },
        ("schedule", "Est. time") if v == "Missing" => {
            let no_scenes = row.get(2).is_none_or(|s| s.trim().is_empty());
            if quiet && no_scenes {
                Cell::text(body("\u{2013}").color(MUTED))
            } else {
                status_chip(v, Tone::Danger)
            }
        }
        ("breakdown_completeness", "Confirmed") => {
            if v.trim() == "0" {
                Cell::text(body(v).color(MUTED).align(Align::Right))
            } else {
                Cell::text(body(v).bold().color(pal::NAVY).align(Align::Right))
            }
        }
        ("breakdown_completeness", "Suggested (unreviewed)") => {
            if v.trim() == "0" {
                Cell::text(body(v).color(MUTED).align(Align::Right))
            } else {
                Cell::text(body(v).bold().color(Tone::Warn.text()).align(Align::Right))
            }
        }
        _ if count => Cell::text(body(v).bold().color(pal::NAVY).align(Align::Right)),
        _ => plain(v, quiet),
    }
}

/// The stripe, fill and zebra a row gets.
fn row_look(report_type: &str, columns: &[String], row: &[String]) -> RowLook {
    let col = |name: &str| {
        columns
            .iter()
            .position(|c| c == name)
            .and_then(|i| row.get(i))
            .map(String::as_str)
            .unwrap_or("")
    };
    match report_type {
        "scene" if col("Scheduled") == "Unscheduled" => RowLook {
            stripe: Some(pal::ORANGE),
            quiet: false,
        },
        "cast_scene" if col("Actor") == "Not cast yet" => RowLook {
            stripe: Some(pal::ORANGE),
            quiet: false,
        },
        "schedule" => match col("Day") {
            "Unscheduled" => RowLook {
                stripe: Some(pal::ORANGE),
                quiet: false,
            },
            "Off Day" => RowLook {
                stripe: None,
                quiet: true,
            },
            _ => RowLook::default(),
        },
        "breakdown_completeness" => match col("Status") {
            "Reviewed" | "" => RowLook::default(),
            "Not started" => RowLook {
                stripe: Some(pal::RED_TEXT),
                quiet: false,
            },
            _ => RowLook {
                stripe: Some(pal::ORANGE),
                quiet: false,
            },
        },
        _ => RowLook::default(),
    }
}

/// The letter a prop is filed under in the divider rows; names that do not start with a letter
/// go under `#`.
fn letter_of(name: &str) -> String {
    match name.trim().chars().next() {
        Some(c) if c.is_alphabetic() => c.to_uppercase().collect(),
        _ => "#".to_string(),
    }
}

/// The scenes in the "still open" band: in progress or needing review, then not started.
fn open_scenes(data: &ReportData) -> Option<NoteBand> {
    let col = |n: &str| data.columns.iter().position(|c| c == n);
    let (sc, st) = (col("Sc")?, col("Status")?);
    let list = |f: &dyn Fn(&str) -> bool| -> Vec<&str> {
        data.rows
            .iter()
            .filter(|r| r.get(st).is_some_and(|v| f(v)))
            .filter_map(|r| r.get(sc).map(String::as_str))
            .collect()
    };
    let open = list(&|v| v == "In progress" || v == "Needs review");
    let not_started = list(&|v| v == "Not started");
    if open.is_empty() && not_started.is_empty() {
        return None;
    }
    let mut parts = Vec::new();
    if !open.is_empty() {
        parts.push(format!("In progress or needs review: {}", open.join(", ")));
    }
    if !not_started.is_empty() {
        parts.push(format!("Not started: {}", not_started.join(", ")));
    }
    Some(NoteBand {
        label: "Scenes still open".into(),
        runs: vec![Run::new(parts.join(" \u{b7} "), false, pal::INK)],
        tone: Tone::Warn,
        stripe: pal::ORANGE,
    })
}

// ------------------------------------------------------------------ export

/// The document with a one-line running header when it runs to more than one page. The probe
/// render is branded like the real one, so both have the same page body.
fn running_header_when_paged(mut doc: Document, line: String) -> Document {
    let mut probe = doc.clone();
    probe.brand = Some(BRAND_LINE.to_string());
    if probe.to_pdf().pages > 1 {
        doc.running_header = Some(line);
    }
    doc
}

/// The metrics of a report, counted over the rows that print.
fn metrics_of(data: &ReportData) -> Vec<Metric> {
    let col_of = |name: &str| data.columns.iter().position(|c| c == name);
    let count_where = |name: &str, f: &dyn Fn(&str) -> bool| -> usize {
        col_of(name)
            .map(|i| {
                data.rows
                    .iter()
                    .filter(|r| r.get(i).is_some_and(|v| f(v)))
                    .count()
            })
            .unwrap_or(0)
    };
    // A count that needs attention prints in its tone; a zero stays navy.
    let toned = |n: usize, label: &str, tone: Tone| {
        let m = Metric::new(n.to_string(), label);
        if n > 0 { m.tone(tone) } else { m }
    };
    let total = data.rows.len();
    match data.report_type.as_str() {
        "scene" => vec![
            Metric::new(total.to_string(), "Scenes"),
            toned(
                count_where("Scheduled", &|v| v == "Unscheduled"),
                "Unscheduled",
                Tone::Warn,
            ),
        ],
        "location" => vec![Metric::new(total.to_string(), "Locations")],
        "cast_scene" => vec![Metric::new(total.to_string(), "Cast members")],
        "prop" => vec![Metric::new(total.to_string(), "Props")],
        "schedule" => vec![Metric::new(total.to_string(), "Schedule rows")],
        "breakdown_completeness" => vec![
            Metric::new(total.to_string(), "Scenes"),
            Metric::new(
                count_where("Status", &|v| v == "Reviewed").to_string(),
                "Reviewed",
            ),
            toned(
                count_where("Status", &|v| v == "In progress" || v == "Needs review"),
                "In progress or needs review",
                Tone::Warn,
            ),
            toned(
                count_where("Status", &|v| v == "Not started"),
                "Not started",
                Tone::Danger,
            ),
        ],
        _ => vec![Metric::new(total.to_string(), "Rows")],
    }
}

/// The table of a report with rows, in the column order the page shows.
fn report_table(data: &ReportData, order: &[usize], content_w: f32) -> StyledTable {
    let rt = data.report_type.as_str();
    let headers: Vec<&String> = order.iter().map(|&i| &data.columns[i]).collect();
    let counts: Vec<bool> = order
        .iter()
        .map(|&i| is_count_column(rt, &data.columns[i], &data.rows, i))
        .collect();
    let specs: Vec<Spec> = headers
        .iter()
        .map(|h| {
            known(rt, h).unwrap_or(Spec {
                width: UNKNOWN,
                fixed: false,
                floor: UNKNOWN,
            })
        })
        .collect();
    let cols: Vec<Col> = widths(&specs, content_w)
        .into_iter()
        .zip(headers.iter().zip(&counts))
        .map(|(w, (h, count))| {
            let c = Col::pt(h.as_str(), w);
            if *count { c.align(Align::Right) } else { c }
        })
        .collect();
    let dense = rt == "prop";
    let mut t = StyledTable::new(cols);
    t.header = HeaderStyle::Navy;
    t.boxed = true;
    t.pad_x = 6.0;
    t.pad_y = if dense { 2.5 } else { 3.6 };
    let divided = dense && data.columns.first().is_some_and(|c| c == "Prop");
    let mut letter = String::new();
    let mut zebra = 0usize;
    for r in &data.rows {
        if divided {
            let l = letter_of(r.first().map(String::as_str).unwrap_or(""));
            if l != letter {
                let mut d = Row::spanning(Cell::text(
                    Txt::new(l.clone())
                        .size(6.8)
                        .bold()
                        .track(0.8)
                        .color(pal::NAVY),
                ));
                d.fill = Some(pal::TINT);
                d.keep_next = true;
                t.rows.push(d);
                letter = l;
                zebra = 0;
            }
        }
        let look = row_look(rt, &data.columns, r);
        let cells = order
            .iter()
            .zip(&counts)
            .map(|(&i, count)| {
                report_cell(
                    rt,
                    &data.columns[i],
                    r.get(i).map(String::as_str).unwrap_or(""),
                    *count,
                    r,
                )
            })
            .collect();
        let mut row = Row::new(cells);
        row.stripe = look.stripe;
        if look.quiet {
            row.fill = Some(pal::PANEL);
        } else {
            if zebra % 2 == 1 {
                row.fill = Some(pal::PANEL);
            }
            zebra += 1;
        }
        t.rows.push(row);
    }
    t
}

pub(super) fn export(
    core: &AppCore,
    actor: &Actor,
    a: ExportReportArgs,
) -> AppResult<super::ExportResult> {
    require_export(actor)?;
    let fmt = Fmt::parse(&a.format, &[Fmt::Pdf, Fmt::Csv, Fmt::Xlsx])?;
    let dest = destination(core, &a.path, fmt)?;
    let session = core.project()?;
    let project = session.store.read(project_title)?;
    let (data, saved_title) = match (&a.report_id, &a.report_type) {
        (Some(id), _) => {
            crate::util::require_id(id, "report")?;
            let row: Option<(String, String)> = session.store.read(|c| {
                Ok(c.query_row(
                    "SELECT title, snapshot_json FROM production_report WHERE id = ?1 AND deleted_at IS NULL",
                    [id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?)
            })?;
            let (title, json) = row.ok_or_else(|| AppError::not_found("report"))?;
            let data: ReportData = serde_json::from_str(&json)
                .map_err(|e| AppError::internal(format!("saved report: {e}")))?;
            (data, Some(title))
        }
        (None, Some(t)) => {
            let v = core.dispatch(
                actor,
                "reports.generate",
                json!({ "reportType": t, "filter": a.filter }),
            )?;
            let data: ReportData = serde_json::from_value(v)
                .map_err(|e| AppError::internal(format!("report: {e}")))?;
            (data, None)
        }
        (None, None) => return Err(AppError::required("A report")),
    };
    let document = saved_title.clone().unwrap_or_else(|| data.title.clone());
    let generated = date_from_ms(data.generated_at);
    // Orientation is decided once from the column count: the Scene Report (7 columns) is
    // landscape, a report with no columns (no Production Source) is portrait.
    let landscape = data.columns.len() > 5;
    let mut doc = Document::new(&document).landscape(landscape);
    let content_w = if landscape { 700.0 } else { 520.0 };

    // The CSV and XLSX table: the columns exactly as stored.
    let columns: Vec<Column> = data
        .columns
        .iter()
        .enumerate()
        .map(|(i, c)| {
            if is_count_column(&data.report_type, c, &data.rows, i) {
                Column::right(c.clone())
            } else {
                Column::new(c.clone())
            }
        })
        .collect();
    let mut table = Table::new(data.title.clone(), columns);
    for r in &data.rows {
        table.push(r.clone());
    }

    // ---- head
    let scope = if saved_title.is_some() {
        "Saved snapshot"
    } else if data.filter.as_deref().and_then(nb).is_some() {
        "Filtered rows"
    } else {
        "All rows"
    };
    let meta: Vec<String> = [
        nb(&project).map(|p| format!("Project: {p}")),
        nb(&data.source_label).map(|s| format!("Source: {s}")),
    ]
    .into_iter()
    .flatten()
    .collect();
    doc.push(Block::ReportHead(ReportHead {
        eyebrow: "Production report".into(),
        title: document.clone(),
        meta: (!meta.is_empty()).then(|| meta.join(" \u{b7} ")),
        panel_label: "Generated".into(),
        panel_value: generated.clone(),
        panel_note: scope.into(),
    }));
    if saved_title.is_some() {
        doc.push(Block::NoteBand(NoteBand {
            label: "Saved snapshot".into(),
            runs: vec![
                Run::new(&document, true, pal::NAVY),
                Run::new(
                    format!(
                        "\u{b7} generated {generated}. It does not change when production data changes."
                    ),
                    false,
                    pal::SLATE,
                ),
            ],
            tone: Tone::Info,
            stripe: pal::TEAL,
        }));
    }
    if let Some(f) = data.filter.as_deref().and_then(nb) {
        doc.push(Block::NoteBand(NoteBand {
            label: "Filter".into(),
            runs: vec![
                Run::new(format!("\u{201c}{f}\u{201d}"), true, pal::NAVY),
                Run::new(
                    "\u{b7} rows with this text in any cell (not case-sensitive)",
                    false,
                    pal::SLATE,
                ),
            ],
            tone: Tone::Info,
            stripe: pal::TEAL,
        }));
    }
    doc.push(Block::MetricStrip(MetricStrip {
        items: metrics_of(&data),
    }));
    if data.report_type == "breakdown_completeness"
        && let Some(band) = open_scenes(&data)
    {
        doc.push(Block::NoteBand(band));
    }

    // ---- body
    if data.rows.is_empty() {
        doc.push(Block::Callout(Callout {
            tone: Tone::Info,
            cols: vec![{
                let mut c = CalloutCol::new(1.0);
                c.lines.push(
                    Txt::new(
                        data.note
                            .clone()
                            .unwrap_or_else(|| "No matching data.".into()),
                    )
                    .size(9.0)
                    .color(Tone::Info.text()),
                );
                c
            }],
        }));
    } else {
        // Character leads the Cast Scene Report: the rows are sorted by it.
        let mut order: Vec<usize> = (0..data.columns.len()).collect();
        if data.columns.len() >= 2 && data.columns[0] == "Actor" && data.columns[1] == "Character" {
            order.swap(0, 1);
        }
        doc.push(Block::Styled(report_table(&data, &order, content_w)));
        if let Some(n) = data.note.as_deref().and_then(nb) {
            doc.push(Block::Text(
                Txt::new(n.to_string())
                    .size(8.4)
                    .color(pal::SLATE)
                    .after(4.0),
            ));
        }
    }
    let mut header: Vec<String> = nb(&project).map(str::to_uppercase).into_iter().collect();
    header.push(data.title.to_uppercase());
    let doc = running_header_when_paged(doc, header.join(" \u{b7} "));

    let mut out = Output::new(doc, HINT_TABLES);
    out.tables.push(table);
    let job = Job {
        action: "reports.export_report",
        document,
        source_label: data.source_label.clone(),
        scope_label: if saved_title.is_some() {
            format!("Saved report ({generated})")
        } else if data.filter.is_some() {
            "Filtered rows".into()
        } else {
            "All rows".into()
        },
        contents_label: plural(data.rows.len(), "row", "rows"),
        target: a.report_id.clone().map(|id| ("production_report", id)),
    };
    deliver(core, actor, &dest, fmt, job, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolved(kind: &str, headers: &[&str], total: f32) -> Vec<f32> {
        let specs: Vec<Spec> = headers
            .iter()
            .map(|h| {
                known(kind, h).unwrap_or(Spec {
                    width: UNKNOWN,
                    fixed: false,
                    floor: UNKNOWN,
                })
            })
            .collect();
        widths(&specs, total)
    }

    #[test]
    fn the_known_column_sets_add_up_to_the_content_width() {
        let cases: [(&str, &[&str], f32); 6] = [
            (
                "scene",
                &[
                    "Sc",
                    "Heading",
                    "I/E · D/N",
                    "Pages",
                    "Location",
                    "Cast",
                    "Scheduled",
                ],
                700.0,
            ),
            (
                "location",
                &["Location", "Status", "Address", "Scenes", "Shooting days"],
                520.0,
            ),
            (
                "cast_scene",
                &[
                    "Character",
                    "Actor",
                    "Scenes",
                    "Scene count",
                    "Shooting days",
                ],
                520.0,
            ),
            ("prop", &["Prop", "Scenes", "Scene count"], 520.0),
            (
                "schedule",
                &["Day", "Date", "Scenes", "Locations", "Est. time"],
                520.0,
            ),
            (
                "breakdown_completeness",
                &[
                    "Sc",
                    "Heading",
                    "Confirmed",
                    "Suggested (unreviewed)",
                    "Status",
                ],
                520.0,
            ),
        ];
        for (kind, headers, total) in cases {
            let w = resolved(kind, headers, total);
            assert!(
                (w.iter().sum::<f32>() - total).abs() < 0.01,
                "{kind}: {w:?}"
            );
        }
        // The widths the spec prints for the Scene Report.
        let w = resolved(
            "scene",
            &[
                "Sc",
                "Heading",
                "I/E · D/N",
                "Pages",
                "Location",
                "Cast",
                "Scheduled",
            ],
            700.0,
        );
        assert_eq!(w, vec![34.0, 186.0, 60.0, 40.0, 150.0, 140.0, 90.0]);
    }

    #[test]
    fn unknown_columns_keep_76_points_and_the_known_ones_shrink_towards_their_floor() {
        // The Scene Report with Stunts, Vehicles and Notes added (spec: stress file WIDE).
        let w = resolved(
            "scene",
            &[
                "Sc",
                "Heading",
                "I/E · D/N",
                "Pages",
                "Location",
                "Cast",
                "Scheduled",
                "Stunts",
                "Vehicles",
                "Notes",
            ],
            700.0,
        );
        let at = |i: usize| (w[i] * 10.0).round() / 10.0;
        assert_eq!((at(1), at(4), at(5)), (96.9, 78.2, 72.9));
        assert_eq!((w[7], w[8], w[9]), (76.0, 76.0, 76.0));
        assert!((w.iter().sum::<f32>() - 700.0).abs() < 0.01);
        // So many unknown columns that even the floors do not fit: all shrink, nothing overflows.
        let many: Vec<&str> = vec!["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k"];
        let w = resolved("scene", &many, 700.0);
        assert!((w.iter().sum::<f32>() - 700.0).abs() < 0.01, "{w:?}");
    }

    #[test]
    fn counts_are_numbers_and_text_columns_never_are() {
        let rows = vec![vec!["5".to_string()], vec!["9".to_string()]];
        assert!(is_count_column("prop", "Scene count", &rows, 0));
        assert!(
            !is_count_column("prop", "Scenes", &rows, 0),
            "scene numbers stay text"
        );
        assert!(!is_count_column("scene", "Sc", &rows, 0));
        assert!(
            is_count_column("other", "Anything", &rows, 0),
            "unknown columns follow the rule"
        );
        let text = vec![vec!["5".to_string()], vec!["1, 2".to_string()]];
        assert!(!is_count_column("other", "Anything", &text, 0));
    }

    #[test]
    fn props_are_filed_under_their_first_letter_or_a_hash() {
        assert_eq!(letter_of("apple"), "A");
        assert_eq!(letter_of("  éclair"), "\u{c9}");
        assert_eq!(letter_of("2-inch reels"), "#");
        assert_eq!(letter_of(""), "#");
    }

    #[test]
    fn a_status_word_has_a_tone_and_a_shape() {
        for (word, mark) in [
            ("Reviewed", Some(Mark::Disc)),
            ("In progress", Some(Mark::RingDot)),
            ("Needs review", Some(Mark::Ring)),
            ("Not started", Some(Mark::Square)),
            ("Idea", None),
        ] {
            assert_eq!(mark_of(status_tone(word)), mark, "{word}");
        }
    }
}
