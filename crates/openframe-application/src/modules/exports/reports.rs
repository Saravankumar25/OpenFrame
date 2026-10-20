//! Production reports export (FSD §58.1, §110; UX §3.33 mock 149): any of the
//! six report types, live (generated on demand from the current data) or a
//! saved report snapshot, as PDF, CSV or XLSX.

use openframe_domain::{Actor, AppError, AppResult};
use openframe_import_export::report::{
    Align, Block, Callout, CalloutCol, Cell, CellItem, Chip, Col, Column, Document, HeaderStyle,
    Metric, Metrics, Row, StyledTable, Table, Tone, Txt, pal,
};
use rusqlite::OptionalExtension;
use serde::Deserialize;
use serde_json::json;
use ts_rs::TS;

use super::style::{nb, production_masthead};
use super::{
    Fmt, HINT_TABLES, Job, Output, date_from_ms, deliver, destination, footer, plural,
    require_export,
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

fn is_numeric_column(rows: &[Vec<String>], i: usize) -> bool {
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

/// Column width and alignment for a report column, by what the column holds.
fn report_col(header: &str, numeric: bool) -> Col {
    let col = match header {
        "Sc" => Col::pt(header, 40.0),
        "Heading" => Col::weight(header, 2.4),
        "I/E · D/N" => Col::pt(header, 100.0),
        "Pages" => Col::pt(header, 48.0),
        "Scheduled" => Col::pt(header, 120.0),
        "Status" => Col::pt(header, 96.0),
        "Est. time" => Col::pt(header, 76.0),
        "Date" | "Day" => Col::pt(header, 92.0),
        "Scenes" => Col::weight(header, 1.5),
        _ => Col::weight(header, 1.2),
    };
    if numeric {
        col.align(Align::Right)
    } else {
        col
    }
}

/// The chip tone of a status word.
fn status_tone(v: &str) -> Tone {
    match v {
        "Reviewed" | "Confirmed" | "Required" => Tone::Ok,
        "Needs review" | "Shortlisted" | "Searching" | "Unscheduled" | "Not cast yet" => Tone::Warn,
        "In progress" | "From heading" => Tone::Info,
        "Missing" | "Rejected" => Tone::Danger,
        _ => Tone::Neutral,
    }
}

/// One report cell: scene numbers as tiles, statuses as chips, headings in bold.
fn report_cell(header: &str, v: &str, numeric: bool) -> Cell {
    let text = |t: &str| {
        Txt::new(t.to_string())
            .size(8.8)
            .color(pal::INK)
            .leading(1.3)
    };
    if v.trim().is_empty() {
        return Cell::empty();
    }
    match header {
        "Sc" => Cell::new().item(CellItem::Tile(v.to_string())),
        "Heading" => Cell::text(text(v).bold().color(pal::NAVY)),
        "I/E · D/N" => Cell::new().item(CellItem::Chips(
            v.split(" · ")
                .map(|p| Chip::label(p, Tone::Neutral).small())
                .collect(),
        )),
        "Status" => Cell::new().item(CellItem::Chips(vec![Chip::label(v, status_tone(v))])),
        "Scheduled" if v == "Unscheduled" => {
            Cell::new().item(CellItem::Chips(vec![Chip::label(v, Tone::Warn)]))
        }
        "Est. time" if v == "Missing" => {
            Cell::new().item(CellItem::Chips(vec![Chip::label(v, Tone::Danger)]))
        }
        "Actor" if v == "Not cast yet" => {
            Cell::new().item(CellItem::Chips(vec![Chip::label(v, Tone::Warn)]))
        }
        _ if numeric => Cell::text(text(v).bold().color(pal::NAVY).align(Align::Right)),
        _ => Cell::text(text(v)),
    }
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
    let mut doc = Document::new(&document)
        .landscape(data.columns.len() > 5)
        .footer(footer(&project, &data.title));
    let numeric: Vec<bool> = (0..data.columns.len())
        .map(|i| is_numeric_column(&data.rows, i))
        .collect();
    let columns: Vec<Column> = data
        .columns
        .iter()
        .zip(&numeric)
        .map(|(c, num)| {
            if *num {
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
    let scope = match (&saved_title, data.filter.as_deref().and_then(nb)) {
        (Some(_), _) => "Saved snapshot".to_string(),
        (None, Some(f)) => format!("Filtered by \u{201c}{f}\u{201d}"),
        (None, None) => "All rows".to_string(),
    };
    let mut mast = production_masthead(
        &format!("PRODUCTION / {}", data.title.to_uppercase()),
        "Production report",
        &document,
        &project,
        &data.source_label,
        &generated,
        &scope,
    );
    mast.title_size = 26.0;
    doc.push(Block::Masthead(mast));
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
    let total = data.rows.len();
    let mut metrics: Vec<Metric> = match data.report_type.as_str() {
        "scene" => vec![
            Metric::new(total.to_string(), "Scenes"),
            Metric::new(
                count_where("Scheduled", &|v| v == "Unscheduled").to_string(),
                "Unscheduled",
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
            Metric::new(
                count_where("Status", &|v| v == "In progress" || v == "Needs review").to_string(),
                "In progress or needs review",
            ),
            Metric::new(
                count_where("Status", &|v| v == "Not started").to_string(),
                "Not started",
            ),
        ],
        _ => vec![Metric::new(total.to_string(), "Rows")],
    };
    if metrics.is_empty() {
        metrics.push(Metric::new(total.to_string(), "Rows"));
    }
    doc.push(Block::Metrics(Metrics {
        items: metrics,
        compact: true,
    }));
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
        let mut t = StyledTable::new(
            data.columns
                .iter()
                .zip(&numeric)
                .map(|(h, num)| report_col(h, *num))
                .collect(),
        );
        t.header = HeaderStyle::Navy;
        t.boxed = true;
        t.zebra = true;
        t.continued = Some(data.title.clone());
        for r in &data.rows {
            t.rows.push(Row::new(
                data.columns
                    .iter()
                    .enumerate()
                    .map(|(i, h)| {
                        report_cell(h, r.get(i).map(String::as_str).unwrap_or(""), numeric[i])
                    })
                    .collect(),
            ));
        }
        doc.push(Block::Styled(t));
        if let Some(n) = data.note.as_deref().and_then(nb) {
            doc.push(Block::Text(
                Txt::new(n.to_string())
                    .size(8.4)
                    .color(pal::SLATE)
                    .after(4.0),
            ));
        }
    }
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
