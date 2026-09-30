//! Production reports export (FSD §58.1, §110; UX §3.33 mock 149): any of the
//! six report types — live (generated on demand from the current data) or a
//! saved report snapshot — as PDF, CSV or XLSX.

use openframe_domain::{Actor, AppError, AppResult};
use openframe_import_export::report::{Block, Column, Document, Table};
use rusqlite::OptionalExtension;
use serde::Deserialize;
use serde_json::json;
use ts_rs::TS;

use super::{
    Fmt, HINT_TABLES, Job, Output, date_from_ms, deliver, destination, footer, plural,
    require_export, subtitle,
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
        .subtitle(subtitle(&project, Some(&data.source_label)))
        .footer(footer(&project, &data.title));
    let mut facts = vec![("Report".to_string(), data.title.clone())];
    facts.push((
        if saved_title.is_some() {
            "Saved snapshot from".into()
        } else {
            "Generated".into()
        },
        generated.clone(),
    ));
    if let Some(f) = data.filter.as_deref().filter(|f| !f.trim().is_empty()) {
        facts.push(("Filtered by".into(), format!("“{f}”")));
    }
    doc.push(Block::KeyValues(facts));
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
    if data.rows.is_empty() {
        doc.push(Block::Paragraph(
            data.note
                .clone()
                .unwrap_or_else(|| "No matching data.".into()),
        ));
    } else {
        let mut t = table.clone();
        t.name = String::new();
        doc.push(Block::Table(t));
        if let Some(n) = data.note.as_deref().filter(|n| !n.trim().is_empty()) {
            doc.push(Block::Note(n.to_string()));
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
