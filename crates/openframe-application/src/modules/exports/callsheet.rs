//! Call Sheet PDF (FSD §38.9 "PDF is required. The call sheet is a snapshot
//! suitable for sending"; Import/Export §6 "the exact Call Sheet document
//! state being exported … does not update the Shooting Schedule").
//!
//! Paper layout of mock 143: production header, day and date, crew call, then
//! dark section bars for Location, Cast (Actor | Character | Call), Scenes
//! (Sc | Heading | Description) and Practical notes, then the optional
//! sections the user added. A Final/Issued (or Superseded) sheet is exported
//! from its immutable finalize snapshot, never from later state.

use openframe_domain::{Actor, AppError, AppResult};
use openframe_import_export::report::{Block, Column, Document, Grid, GridCell, Table};
use rusqlite::OptionalExtension;
use serde::Deserialize;
use serde_json::Value;
use ts_rs::TS;

use super::{
    Fmt, HINT_EDIT, Job, Output, date_from_ms, deliver, destination, footer, image_warnings,
    load_image, nonblank, plural, require_export,
};
use crate::core::AppCore;
use crate::modules::schedule::board::project_title;
use crate::modules::schedule::callsheet::CallSheetDocument;
use crate::modules::schedule::fmt::date_long_upper;

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportCallSheetArgs {
    /// Call sheet id.
    pub id: String,
    /// "pdf"
    pub format: String,
    pub path: String,
    /// Optional sections the user added (weather, special notes, attachments
    /// list, reference images). Default true.
    #[serde(default)]
    #[ts(optional)]
    pub include_optional: Option<bool>,
}

pub(super) fn export(
    core: &AppCore,
    actor: &Actor,
    a: ExportCallSheetArgs,
) -> AppResult<super::ExportResult> {
    require_export(actor)?;
    let fmt = Fmt::parse(&a.format, &[Fmt::Pdf])?;
    crate::util::require_id(&a.id, "call sheet")?;
    let optional = a.include_optional.unwrap_or(true);
    let dest = destination(core, &a.path, fmt)?;
    let session = core.project()?;
    let root = session.layout.root().to_path_buf();

    struct Sheet {
        title: String,
        status: String,
        revision: i64,
        document: CallSheetDocument,
        source: Value,
        frozen_at: Option<i64>,
        from_snapshot: bool,
        project: String,
        images: Vec<(String, Option<crate::util::AssetInfo>)>,
    }
    let sheet = session.store.read(|c| {
        let row: Option<(String, String, i64, String, String, Option<String>, Option<i64>, Option<i64>)> = c
            .query_row(
                "SELECT title, status, revision, document_json, source_snapshot_json, snapshot_id, finalized_at, issued_at
                 FROM call_sheet WHERE id = ?1 AND deleted_at IS NULL",
                [&a.id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?)),
            )
            .optional()?;
        let (title, status, revision, doc_json, src_json, snapshot_id, finalized_at, issued_at) =
            row.ok_or_else(|| AppError::not_found("call sheet"))?;
        let frozen = matches!(status.as_str(), "Final" | "Issued" | "Superseded");
        // Issued/Final documents come from their immutable snapshot (FSD §38.8, §148).
        let snapshot: Option<Value> = match (&snapshot_id, frozen) {
            (Some(sid), true) => c
                .query_row("SELECT content_json FROM snapshot WHERE id = ?1", [sid], |r| r.get::<_, String>(0))
                .optional()?
                .and_then(|j| serde_json::from_str(&j).ok()),
            _ => None,
        };
        let (document, source, from_snapshot) = match snapshot {
            Some(v) => (
                serde_json::from_value::<CallSheetDocument>(v["document"].clone())
                    .map_err(|e| AppError::internal(format!("call sheet snapshot: {e}")))?,
                v["source"].clone(),
                true,
            ),
            None => (
                serde_json::from_str::<CallSheetDocument>(&doc_json).unwrap_or_default(),
                serde_json::from_str::<Value>(&src_json).unwrap_or(Value::Null),
                false,
            ),
        };
        let mut images = Vec::new();
        if optional {
            for att in document.optional.reference_images.iter().flatten() {
                let info = crate::util::load_asset(c, &root, &att.asset_id).ok();
                images.push((att.name.clone(), info));
            }
        }
        Ok(Sheet {
            title,
            status,
            revision,
            document,
            source,
            frozen_at: issued_at.or(finalized_at),
            from_snapshot,
            project: project_title(c)?,
            images,
        })
    })?;

    let d = &sheet.document;
    let production = sheet.source["productionTitle"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(&sheet.project)
        .to_uppercase();
    let source_label = sheet.source["sourceLabel"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let mut header = vec![d.day_label.trim().to_uppercase()];
    if let Some(date) = d.date.as_deref().filter(|s| !s.trim().is_empty()) {
        header.push(date_long_upper(date));
    }
    let header: Vec<String> = header.into_iter().filter(|s| !s.is_empty()).collect();
    let mut doc = Document::new(if production.is_empty() {
        "CALL SHEET".to_string()
    } else {
        production.clone()
    })
    .centered_header(true)
    .subtitle(header.join(" · "))
    .footer(footer(&sheet.project, &sheet.title));
    let crew = nonblank(Some(&d.crew_call));
    doc.push(Block::Centered {
        text: format!("CREW CALL: {}", crew.clone().unwrap_or_else(|| "—".into())),
        size: 13.0,
        bold: true,
    });
    let state = match sheet.status.as_str() {
        "Final" | "Issued" | "Superseded" => format!(
            "{}{}",
            sheet.status,
            sheet
                .frozen_at
                .map(|t| format!(" {}", date_from_ms(t)))
                .unwrap_or_default()
        ),
        _ => "DRAFT — not finalized".into(),
    };
    let mut status_line = vec![sheet.title.clone(), format!("v{}", sheet.revision), state];
    if !source_label.is_empty() {
        status_line.push(format!("Script: {source_label}"));
    }
    doc.push(Block::Centered {
        text: status_line.join(" · "),
        size: 8.5,
        bold: false,
    });
    doc.push(Block::Spacer(4.0));

    // Location
    doc.push(Block::SectionBar("Location".into()));
    if d.locations.is_empty() {
        doc.push(Block::Note("No location for this day.".into()));
    }
    for l in &d.locations {
        let mut kv = vec![("Location".to_string(), l.name.clone())];
        for (k, v) in [
            ("Address", &l.address),
            ("Meeting point", &l.meeting_point),
            ("Parking", &l.parking),
        ] {
            if let Some(v) = nonblank(Some(v)) {
                kv.push((k.to_string(), v));
            }
        }
        doc.push(Block::KeyValues(kv));
    }
    // Cast
    doc.push(Block::SectionBar("Cast".into()));
    if d.cast.is_empty() {
        doc.push(Block::Note("No cast called.".into()));
    } else {
        let has_notes = d.cast.iter().any(|c| !c.notes.trim().is_empty());
        let mut cols = vec![
            Column::new("Actor").weight(1.3),
            Column::new("Character").weight(1.3),
            Column::new("Call").weight(0.6),
        ];
        if has_notes {
            cols.push(Column::new("Notes").weight(1.6));
        }
        let mut t = Table::new("", cols);
        for c in &d.cast {
            let mut r = vec![
                nonblank(Some(&c.actor)).unwrap_or_else(|| "(not cast)".into()),
                c.character.clone(),
                nonblank(Some(&c.call_time)).unwrap_or_else(|| "—".into()),
            ];
            if has_notes {
                r.push(c.notes.clone());
            }
            t.push(r);
        }
        doc.push(Block::Table(t));
    }
    // Scenes
    doc.push(Block::SectionBar("Scenes".into()));
    if d.scenes.is_empty() {
        doc.push(Block::Note("No scenes on this day.".into()));
    } else {
        let mut t = Table::new(
            "",
            vec![
                Column::new("Sc").weight(0.35),
                Column::new("Heading").weight(1.8),
                Column::new("Description").weight(2.2),
                Column::right("Pages").weight(0.5),
            ],
        );
        for s in &d.scenes {
            t.push(vec![
                s.number.clone(),
                s.heading.clone(),
                s.description.clone(),
                s.pages.clone(),
            ]);
        }
        doc.push(Block::Table(t));
    }
    // Practical notes
    doc.push(Block::SectionBar("Practical notes".into()));
    let p = &d.practical;
    let mut kv: Vec<(String, String)> = [
        ("Parking", &p.parking),
        ("Meeting point", &p.meeting_point),
        ("Travel", &p.travel_notes),
        ("Meal break", &p.meal_break),
        ("Emergency contact", &p.emergency_contact),
        ("Production notes", &p.production_notes),
        ("Day notes", &d.day_notes),
    ]
    .into_iter()
    .filter_map(|(k, v)| nonblank(Some(v)).map(|v| (k.to_string(), v)))
    .collect();
    for f in &d.extra_fields {
        if let (Some(k), Some(v)) = (nonblank(Some(&f.label)), nonblank(Some(&f.value))) {
            kv.push((k, v));
        }
    }
    if kv.is_empty() {
        doc.push(Block::Note("None.".into()));
    } else {
        doc.push(Block::KeyValues(kv));
    }
    // Optional sections
    let mut missing = 0;
    let mut unreadable = 0;
    if optional {
        let o = &d.optional;
        if let Some(w) = nonblank(o.weather.as_deref()) {
            doc.push(Block::SectionBar("Weather".into()));
            doc.push(Block::Paragraph(w));
        }
        if let Some(s) = nonblank(o.special_notes.as_deref()) {
            doc.push(Block::SectionBar("Special notes".into()));
            doc.push(Block::Paragraph(s));
        }
        if let Some(list) = o.attachments.as_ref().filter(|l| !l.is_empty()) {
            doc.push(Block::SectionBar("Attachments".into()));
            doc.push(Block::Paragraph(format!(
                "Sent separately: {}",
                list.iter()
                    .map(|x| x.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
        if !sheet.images.is_empty() {
            doc.push(Block::SectionBar("Reference images".into()));
            let cells = sheet
                .images
                .iter()
                .map(|(name, info)| {
                    let mut cell = GridCell {
                        lines: vec![name.clone()],
                        ..Default::default()
                    };
                    match info.as_ref().map(load_image) {
                        Some(Ok(img)) => cell.image = Some(img),
                        Some(Err("unreadable")) | Some(Err("too large")) => {
                            unreadable += 1;
                            cell.placeholder = Some("Image can't be printed".into());
                        }
                        _ => {
                            missing += 1;
                            cell.placeholder = Some("Image unavailable".into());
                        }
                    }
                    cell
                })
                .collect();
            doc.push(Block::Grid(Grid {
                columns: 3,
                frame_ratio: 0.66,
                cells,
            }));
        }
    }

    let mut out = Output::new(doc, HINT_EDIT);
    out.warnings = image_warnings(missing, unreadable, "reference image");
    match sheet.status.as_str() {
        "Final" | "Issued" => {}
        "Superseded" => out.warnings.push(
            "This call sheet has been superseded by a newer revision. The PDF shows the version that was issued.".into(),
        ),
        "Needs Refresh" => out.warnings.push(
            "The schedule changed after this call sheet was prepared. The PDF shows the call sheet as it is now and is marked Draft.".into(),
        ),
        _ => out
            .warnings
            .push("This call sheet is not finalized yet, so the PDF is marked Draft.".into()),
    }
    if crew.is_none() {
        out.warnings.push("The crew call time is empty.".into());
    }
    let job = Job {
        action: "callsheets.export_pdf",
        document: sheet.title.clone(),
        source_label: if sheet.from_snapshot {
            format!("{} · v{} (issued snapshot)", sheet.title, sheet.revision)
        } else {
            format!("{} · v{}", sheet.title, sheet.revision)
        },
        scope_label: "Current call sheet".into(),
        contents_label: format!(
            "{} · {}",
            plural(d.scenes.len(), "scene", "scenes"),
            plural(d.cast.len(), "cast call", "cast calls")
        ),
        target: Some(("call_sheet", a.id.clone())),
    };
    deliver(core, actor, &dest, fmt, job, out)
}
