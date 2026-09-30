//! Breakdown report and Catalog export (FSD §60.2 "Breakdown: PDF/report";
//! Import/Export §6: "Suggested versus confirmed states should remain
//! distinguishable"; Catalog "CSV/XLSX-style"; "not a live connection").
//!
//! Scenes come from the active Production Source; elements are grouped by the
//! FSD §26.3 categories. The Catalog lists every (non-archived) item with the
//! scenes that use it (derived from production breakdown rows).

use std::collections::{HashMap, HashSet};

use openframe_domain::enums::BreakdownCategory;
use openframe_domain::{Actor, AppError, AppResult};
use openframe_import_export::report::{Block, Column, Document, Table};
use rusqlite::{Connection, OptionalExtension};
use serde::Deserialize;
use ts_rs::TS;

use super::{
    Fmt, HINT_TABLES, Job, Output, deliver, destination, footer, nonblank, plural, require_export,
    subtitle,
};
use crate::core::AppCore;
use crate::modules::production::{SourceScenes, active_source, catalog_usage};
use crate::modules::schedule::board::project_title;

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportBreakdownArgs {
    /// "pdf" | "csv" | "xlsx"
    pub format: String,
    pub path: String,
    /// "both" (default; PDF/XLSX), "scenes" or "catalog". CSV holds one table.
    #[serde(default)]
    #[ts(optional)]
    pub content: Option<String>,
    /// Selected scenes of the Production Source (None = every scene).
    #[serde(default)]
    #[ts(optional)]
    pub scene_ids: Option<Vec<String>>,
    /// Suggested (unconfirmed) elements, clearly marked. Default true.
    #[serde(default)]
    #[ts(optional)]
    pub include_suggested: Option<bool>,
    /// Breakdown element notes. Default false.
    #[serde(default)]
    #[ts(optional)]
    pub include_notes: Option<bool>,
}

struct El {
    scene_id: String,
    category: String,
    name: String,
    state: String,
    catalog: Option<String>,
    notes: Option<String>,
}

fn cat_order(cat: &str) -> usize {
    BreakdownCategory::ALL
        .iter()
        .position(|c| c.as_str() == cat)
        .unwrap_or(usize::MAX)
}

fn state_label(state: &str) -> &'static str {
    match state {
        "Suggested" => "Suggested",
        "Manual" => "Added manually",
        _ => "Confirmed",
    }
}

pub(super) fn export(
    core: &AppCore,
    actor: &Actor,
    a: ExportBreakdownArgs,
) -> AppResult<super::ExportResult> {
    require_export(actor)?;
    let fmt = Fmt::parse(&a.format, &[Fmt::Pdf, Fmt::Csv, Fmt::Xlsx])?;
    let content = match (a.content.as_deref(), fmt) {
        (None, Fmt::Csv) => "scenes",
        (None, _) => "both",
        (Some("both"), Fmt::Csv) => {
            return Err(AppError::invalid_input(
                "A CSV file holds one table. Choose Breakdown by scene or Catalog, or export Excel (XLSX) for both.",
            ));
        }
        (Some(c @ ("both" | "scenes" | "catalog")), _) => c,
        _ => {
            return Err(AppError::invalid_input(
                "Choose Breakdown by scene, Catalog or both.",
            ));
        }
    };
    let with_scenes = content != "catalog";
    let with_catalog = content != "scenes";
    let suggested = a.include_suggested.unwrap_or(true);
    let notes = a.include_notes.unwrap_or(false);
    if let Some(ids) = &a.scene_ids
        && ids.is_empty()
    {
        return Err(AppError::validation(
            "sceneIds",
            "Select at least one scene to export.",
        ));
    }
    let dest = destination(core, &a.path, fmt)?;
    let session = core.project()?;

    struct Data {
        project: String,
        source_label: String,
        scenes: Vec<(String, String, String, bool)>,
        elements: Vec<El>,
        catalog: Vec<(String, String, String, Option<String>, Vec<String>)>,
    }
    let data = session.store.read(|c: &Connection| {
        let src = active_source(c)?.ok_or_else(|| {
            AppError::new(
                "validation.no_production_source",
                "Choose a screenplay draft as the Production Source first. Your screenplay is not changed.",
            )
        })?;
        let (draft, title): (String, String) = c
            .query_row(
                "SELECT d.name, s.title FROM screenplay_draft d JOIN screenplay s ON s.id = d.screenplay_id WHERE d.id = ?1",
                [&src.draft_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?
            .unwrap_or_default();
        let src_scenes = SourceScenes::load(c, Some(&src.draft_id))?;
        let wanted: Option<HashSet<&str>> = a.scene_ids.as_ref().map(|v| v.iter().map(|s| s.as_str()).collect());
        if let Some(w) = &wanted
            && let Some(missing) = w.iter().find(|id| src_scenes.get(id).is_none()) {
                return Err(AppError::not_found("scene").with_detail(format!("scene {missing} not in the production source")));
            }
        let scenes: Vec<(String, String, String, bool)> = src_scenes
            .rows
            .iter()
            .filter(|r| wanted.as_ref().is_none_or(|w| w.contains(r.id.as_str())))
            .map(|r| (r.id.clone(), r.number.clone(), r.heading.clone(), r.omitted))
            .collect();
        let in_scope: HashSet<&str> = scenes.iter().map(|s| s.0.as_str()).collect();
        let mut st = c.prepare(
            "SELECT e.scene_id, e.category, COALESCE(ci.name, e.display_name), e.confirmation_state,
                    ci.name, e.notes
             FROM breakdown_element e LEFT JOIN catalog_item ci ON ci.id = e.catalog_item_id AND ci.deleted_at IS NULL
             WHERE e.deleted_at IS NULL AND e.archived = 0 AND e.confirmation_state <> 'Rejected'
             ORDER BY e.created_at, e.id",
        )?;
        let elements: Vec<El> = st
            .query_map([], |r| {
                Ok(El {
                    scene_id: r.get(0)?,
                    category: r.get(1)?,
                    name: r.get(2)?,
                    state: r.get(3)?,
                    catalog: r.get(4)?,
                    notes: r.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .filter(|e| in_scope.contains(e.scene_id.as_str()))
            .filter(|e| suggested || e.state != "Suggested")
            .collect();
        let usage = catalog_usage(c, &src_scenes)?;
        let mut st = c.prepare(
            "SELECT id, category, name, status, description FROM catalog_item
             WHERE deleted_at IS NULL AND archived = 0",
        )?;
        let mut catalog: Vec<(String, String, String, Option<String>, Vec<String>)> = st
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))?
            .collect::<Result<Vec<(String, String, String, String, Option<String>)>, _>>()?
            .into_iter()
            .filter_map(|(id, cat, name, status, desc)| {
                let refs = usage.get(&id).cloned().unwrap_or_default();
                let nums: Vec<String> = refs
                    .iter()
                    .filter(|r| in_scope.contains(r.scene_id.as_str()))
                    .filter_map(|r| r.number.clone())
                    .collect();
                // With a scene selection, only items used in those scenes.
                if wanted.is_some() && nums.is_empty() {
                    return None;
                }
                Some((cat, name, status, desc, nums))
            })
            .collect();
        catalog.sort_by(|x, y| {
            (cat_order(&x.0), x.1.to_lowercase()).cmp(&(cat_order(&y.0), y.1.to_lowercase()))
        });
        Ok(Data {
            project: project_title(c)?,
            source_label: if title.is_empty() { draft } else { format!("{title} — {draft}") },
            scenes,
            elements,
            catalog,
        })
    })?;

    if with_scenes && data.scenes.is_empty() {
        return Err(AppError::export(
            "nothing_to_export",
            "The Production Source has no scenes to export.",
        ));
    }

    let mut by_scene: HashMap<&str, Vec<&El>> = HashMap::new();
    for e in &data.elements {
        by_scene.entry(e.scene_id.as_str()).or_default().push(e);
    }
    for v in by_scene.values_mut() {
        v.sort_by_key(|e| cat_order(&e.category));
    }
    let scene_title = |num: &str, heading: &str, omitted: bool| {
        format!(
            "Scene {num} — {}",
            if omitted { "OMITTED" } else { heading }
        )
    };

    // ---- PDF
    let document = match content {
        "catalog" => "Catalog",
        _ => "Breakdown Report",
    };
    let mut doc = Document::new(document)
        .subtitle(subtitle(&data.project, Some(&data.source_label)))
        .footer(footer(&data.project, document));
    let confirmed = data
        .elements
        .iter()
        .filter(|e| e.state != "Suggested")
        .count();
    let sugg = data.elements.len() - confirmed;
    let mut facts = vec![
        ("Production Source".to_string(), data.source_label.clone()),
        ("Scenes".to_string(), data.scenes.len().to_string()),
    ];
    if with_scenes {
        facts.push(("Confirmed elements".into(), confirmed.to_string()));
        if suggested {
            facts.push(("Suggested (not confirmed)".into(), sugg.to_string()));
        }
    }
    if with_catalog {
        facts.push(("Catalog items".into(), data.catalog.len().to_string()));
    }
    doc.push(Block::KeyValues(facts));
    if with_scenes {
        doc.push(Block::Heading {
            text: "Breakdown by scene".into(),
            level: 1,
        });
        if suggested && sugg > 0 {
            doc.push(Block::Note(
                "Suggested elements are not confirmed yet and are marked “(suggested)”.".into(),
            ));
        }
        for (id, num, heading, omitted) in &data.scenes {
            doc.push(Block::Heading {
                text: scene_title(num, heading, *omitted),
                level: 2,
            });
            let els = by_scene.get(id.as_str()).cloned().unwrap_or_default();
            if els.is_empty() {
                doc.push(Block::Note("No breakdown elements yet.".into()));
                continue;
            }
            let mut kv: Vec<(String, String)> = Vec::new();
            for e in els {
                let mut label = e.name.clone();
                if e.state == "Suggested" {
                    label.push_str(" (suggested)");
                }
                if notes && let Some(n) = nonblank(e.notes.as_deref()) {
                    label.push_str(&format!(" — {n}"));
                }
                match kv.iter_mut().find(|(k, _)| *k == e.category) {
                    Some((_, v)) => {
                        v.push_str(if notes { "; " } else { ", " });
                        v.push_str(&label);
                    }
                    None => kv.push((e.category.clone(), label)),
                }
            }
            doc.push(Block::KeyValues(kv));
        }
    }
    let mut cat_table = Table::new(
        "Catalog",
        vec![
            Column::new("Category"),
            Column::new("Item").weight(1.4),
            Column::new("Status").weight(0.8),
            Column::new("Used in scenes").weight(1.4),
            Column::right("Scenes"),
            Column::new("Description").weight(1.8),
        ],
    );
    for (cat, name, status, desc, nums) in &data.catalog {
        cat_table.push(vec![
            cat.clone(),
            name.clone(),
            status.clone(),
            if nums.is_empty() {
                "Not used yet".into()
            } else {
                nums.join(", ")
            },
            nums.len().to_string(),
            desc.clone().unwrap_or_default(),
        ]);
    }
    if with_catalog {
        if with_scenes {
            doc.push(Block::PageBreak);
        }
        doc.push(Block::Heading {
            text: "Catalog".into(),
            level: 1,
        });
        if data.catalog.is_empty() {
            doc.push(Block::Note("The catalog is empty.".into()));
        } else {
            let mut t = cat_table.clone();
            t.name = String::new();
            doc.push(Block::Table(t));
        }
    }

    // ---- CSV / XLSX
    let mut cols = vec![
        Column::right("Scene"),
        Column::new("Heading"),
        Column::new("Category"),
        Column::new("Element"),
        Column::new("State"),
        Column::new("Catalog item"),
    ];
    if notes {
        cols.push(Column::new("Notes"));
    }
    let mut scene_table = Table::new("Breakdown by Scene", cols);
    for (id, num, heading, omitted) in &data.scenes {
        for e in by_scene.get(id.as_str()).cloned().unwrap_or_default() {
            let mut row = vec![
                num.clone(),
                if *omitted {
                    "OMITTED".into()
                } else {
                    heading.clone()
                },
                e.category.clone(),
                e.name.clone(),
                state_label(&e.state).into(),
                e.catalog.clone().unwrap_or_default(),
            ];
            if notes {
                row.push(e.notes.clone().unwrap_or_default());
            }
            scene_table.push(row);
        }
    }

    let mut out = Output::new(doc, HINT_TABLES);
    if with_scenes {
        out.tables.push(scene_table);
    }
    if with_catalog {
        out.tables.push(cat_table);
    }
    let scope_label = match &a.scene_ids {
        Some(ids) => plural(ids.len(), "selected scene", "selected scenes"),
        None => "Entire Production Source".to_string(),
    };
    let mut contents = Vec::new();
    if with_scenes {
        contents.push(plural(data.scenes.len(), "scene", "scenes"));
        contents.push(plural(data.elements.len(), "element", "elements"));
    }
    if with_catalog {
        contents.push(plural(data.catalog.len(), "catalog item", "catalog items"));
    }
    let job = Job {
        action: "breakdown.export_report",
        document: document.to_string(),
        source_label: data.source_label,
        scope_label,
        contents_label: contents.join(" · "),
        target: None,
    };
    deliver(core, actor, &dest, fmt, job, out)
}
