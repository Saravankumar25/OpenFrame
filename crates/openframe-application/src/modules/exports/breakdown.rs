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
use openframe_import_export::report::{
    Align, Block, Card, CardHead, CardPart, Cards, Cell, CellItem, Chip, Col, Column, Document,
    Group, GroupItem, HeaderStyle, Metric, Metrics, Rgb, Row, Section, StyledTable, Table, Tile,
    Tone, Txt, pal,
};
use rusqlite::{Connection, OptionalExtension};
use serde::Deserialize;
use ts_rs::TS;

use super::style::{nb, production_masthead};
use super::{
    Fmt, HINT_TABLES, Job, Output, deliver, destination, footer, nonblank, plural, require_export,
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
    /// "both" (default; PDF/XLSX), "scenes", "catalog" or "summary" (the short breakdown
    /// report: totals per category and scene status on one page). CSV holds one table.
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
        (Some(c @ ("both" | "scenes" | "catalog" | "summary")), _) => c,
        _ => {
            return Err(AppError::invalid_input(
                "Choose Breakdown by scene, Catalog, both, or the short summary.",
            ));
        }
    };
    if content == "summary" {
        return export_summary(core, actor, a, fmt);
    }
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
            source_label: if title.is_empty() { draft } else { format!("{title}: {draft}") },
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

    // ---- PDF
    let document = match content {
        "catalog" => "Catalog",
        _ => "Breakdown Report",
    };
    let mut doc = Document::new(document).footer(footer(&data.project, document));
    let confirmed = data
        .elements
        .iter()
        .filter(|e| e.state != "Suggested")
        .count();
    let sugg = data.elements.len() - confirmed;
    let scope_text = match &a.scene_ids {
        Some(ids) => plural(ids.len(), "selected scene", "selected scenes"),
        None => "Entire production source".to_string(),
    };
    doc.push(Block::Masthead(production_masthead(
        if with_scenes {
            "PRODUCTION / BREAKDOWN REPORT"
        } else {
            "PRODUCTION / CATALOG"
        },
        if with_scenes {
            "Production breakdown"
        } else {
            "Production inventory"
        },
        if with_scenes {
            "Breakdown Report"
        } else {
            "Catalog"
        },
        &data.project,
        &data.source_label,
        &super::today_label(),
        &scope_text,
    )));
    let mut metrics = vec![Metric::new(data.scenes.len().to_string(), "Scenes")];
    if with_scenes {
        metrics.push(Metric::new(confirmed.to_string(), "Confirmed elements"));
        if suggested {
            metrics.push(Metric::new(sugg.to_string(), "Suggested"));
        }
    }
    if with_catalog {
        metrics.push(Metric::new(data.catalog.len().to_string(), "Catalog items"));
    }
    doc.push(Block::Metrics(Metrics {
        items: metrics,
        compact: false,
    }));
    if with_scenes {
        doc.push(Block::Section(Section::new(
            "Breakdown by scene",
            Some("Ordered by the Production Source".into()),
        )));
        if suggested && sugg > 0 {
            doc.push(Block::Text(
                Txt::new(
                    "Suggested elements are not confirmed yet and carry a SUGGESTED label. \
                     Elements added by hand carry a MANUAL label.",
                )
                .size(8.4)
                .color(pal::SLATE)
                .after(6.0),
            ));
        }
        let cards: Vec<Card> = data
            .scenes
            .iter()
            .map(|(id, num, heading, omitted)| {
                let mut head = CardHead::new(if *omitted {
                    "OMITTED".to_string()
                } else {
                    heading.to_uppercase()
                });
                head.eyebrow = Some("Breakdown scene".into());
                head.tile = Some(Tile {
                    top: Some("SCENE".into()),
                    text: num.clone(),
                    fill: if *omitted {
                        pal::PANEL
                    } else {
                        Rgb::hex(0xfdecc8)
                    },
                    line: Some(if *omitted {
                        pal::LINE
                    } else {
                        pal::ORANGE_LINE
                    }),
                    color: if *omitted {
                        pal::SLATE
                    } else {
                        pal::ORANGE_DEEP
                    },
                    w: 38.0,
                });
                let els = by_scene.get(id.as_str()).cloned().unwrap_or_default();
                let mut card = Card {
                    head: Some(head),
                    ..Card::new()
                };
                if els.is_empty() {
                    card.parts
                        .push(CardPart::Empty("No breakdown elements yet.".into()));
                    return card;
                }
                let mut groups: Vec<Group> = Vec::new();
                for e in els {
                    let mut item = GroupItem::new(if e.name.trim().is_empty() {
                        "Unnamed element".to_string()
                    } else {
                        e.name.clone()
                    });
                    match e.state.as_str() {
                        "Suggested" => {
                            item.tone = Tone::Warn;
                            item.tag = Some(Chip::label("Suggested", Tone::Warn));
                        }
                        "Manual" => item.tag = Some(Chip::label("Manual", Tone::Info)),
                        _ => {}
                    }
                    if notes {
                        item.note = nonblank(e.notes.as_deref());
                    }
                    match groups.iter_mut().find(|g| g.label == e.category) {
                        Some(g) => g.items.push(item),
                        None => groups.push(Group {
                            label: e.category.clone(),
                            accent: pal::category(&e.category),
                            items: vec![item],
                        }),
                    }
                }
                card.parts.push(CardPart::Groups { groups, cols: 3 });
                card
            })
            .collect();
        doc.push(Block::Cards(Cards::new(1, cards)));
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
        doc.push(Block::Section(Section::new(
            "Catalog",
            Some("Usage is derived from breakdown assignments".into()),
        )));
        if data.catalog.is_empty() {
            doc.push(Block::Text(
                Txt::new("The catalog is empty.")
                    .size(8.8)
                    .color(pal::FAINT)
                    .after(6.0),
            ));
        } else {
            let mut cats: Vec<String> = Vec::new();
            for (cat, ..) in &data.catalog {
                if !cats.contains(cat) {
                    cats.push(cat.clone());
                }
            }
            for cat in cats {
                let items: Vec<_> = data.catalog.iter().filter(|c| c.0 == cat).collect();
                let mut t = StyledTable::new(vec![
                    Col::weight("Item", 1.7),
                    Col::pt("Status", 86.0),
                    Col::weight("Used in scenes", 1.2),
                    Col::pt("Uses", 52.0).align(Align::Right),
                ]);
                t.header = HeaderStyle::None;
                t.boxed = true;
                t.continued = Some(cat.clone());
                let mut band = Row::spanning(Cell::new().item(CellItem::Heading {
                    dot: Some(pal::category(&cat)),
                    text: cat.clone(),
                    right: Some(plural(items.len(), "item", "items")),
                }));
                band.fill = Some(pal::PANEL);
                band.keep_next = true;
                t.rows.push(band);
                for (_, name, status, desc, nums) in items {
                    let mut item = Cell::text(
                        Txt::new(name.clone())
                            .size(9.4)
                            .bold()
                            .color(pal::NAVY)
                            .leading(1.25),
                    );
                    if let Some(d) = desc.as_deref().and_then(nb) {
                        item = item.item(CellItem::Text(
                            Txt::new(d).size(8.2).color(pal::SLATE).leading(1.3),
                        ));
                    }
                    let tone = match status.as_str() {
                        "Confirmed" | "Required" => Tone::Ok,
                        "Searching" | "Shortlisted" => Tone::Warn,
                        _ => Tone::Neutral,
                    };
                    let scenes_cell = if nums.is_empty() {
                        Cell::text(Txt::label("Used in scenes")).item(CellItem::Text(
                            Txt::new("Not used yet").size(8.6).color(pal::FAINT),
                        ))
                    } else {
                        Cell::text(Txt::label("Used in scenes")).item(CellItem::Text(
                            Txt::new(nums.join(", "))
                                .size(9.0)
                                .bold()
                                .color(pal::INK)
                                .leading(1.3),
                        ))
                    };
                    t.rows.push(Row::new(vec![
                        item,
                        Cell::new()
                            .item(CellItem::Chips(vec![Chip::label(status.clone(), tone)]))
                            .middle(),
                        scenes_cell,
                        Cell::text(
                            Txt::new(nums.len().to_string())
                                .size(15.0)
                                .bold()
                                .color(pal::NAVY)
                                .align(Align::Right),
                        )
                        .item(CellItem::Text(
                            Txt::label(if nums.len() == 1 { "Use" } else { "Uses" })
                                .size(6.0)
                                .align(Align::Right),
                        ))
                        .middle(),
                    ]));
                }
                doc.push(Block::Styled(t));
            }
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

/// The short breakdown report (depth spec section 24 "concise breakdown report"): one page
/// with the scene status totals and, per category, how many items and scenes there are.
/// Only confirmed production data is counted; suggestions are listed as a separate total.
fn export_summary(
    core: &AppCore,
    actor: &Actor,
    a: ExportBreakdownArgs,
    fmt: Fmt,
) -> AppResult<super::ExportResult> {
    use crate::modules::production::breakdown::{BreakdownSceneFilter as F, scene_statuses};
    let dest = destination(core, &a.path, fmt)?;
    let session = core.project()?;
    struct CatRow {
        category: String,
        items: Vec<String>,
        scenes: usize,
    }
    struct Data {
        project: String,
        source_label: String,
        scenes: usize,
        complete: usize,
        missing: usize,
        needs_review: usize,
        suggested: i64,
        rows: Vec<CatRow>,
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
        let statuses = scene_statuses(c, &src, &src_scenes)?;
        let in_source: HashSet<&str> = src_scenes.rows.iter().map(|r| r.id.as_str()).collect();
        let mut st = c.prepare(
            "SELECT e.scene_id, e.category, COALESCE(ci.name, e.display_name)
             FROM breakdown_element e LEFT JOIN catalog_item ci ON ci.id = e.catalog_item_id AND ci.deleted_at IS NULL
             WHERE e.deleted_at IS NULL AND e.archived = 0 AND e.confirmation_state IN ('Confirmed','Manual')",
        )?;
        let els: Vec<(String, String, String)> = st
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<Result<_, _>>()?;
        let mut rows: Vec<CatRow> = Vec::new();
        for cat in BreakdownCategory::ALL {
            let mine: Vec<&(String, String, String)> = els
                .iter()
                .filter(|e| e.1 == cat.as_str() && in_source.contains(e.0.as_str()))
                .collect();
            if mine.is_empty() {
                continue;
            }
            let mut items: Vec<String> = mine.iter().map(|e| e.2.clone()).collect();
            items.sort_by_key(|n| n.to_lowercase());
            items.dedup_by(|x, y| x.eq_ignore_ascii_case(y));
            let scenes: HashSet<&str> = mine.iter().map(|e| e.0.as_str()).collect();
            rows.push(CatRow {
                category: cat.as_str().to_string(),
                items,
                scenes: scenes.len(),
            });
        }
        let suggested: i64 = statuses.iter().map(|s| s.suggested).sum();
        let n = |f: F| statuses.iter().filter(|s| s.matches(f)).count();
        Ok(Data {
            project: project_title(c)?,
            source_label: if title.is_empty() { draft } else { format!("{title}: {draft}") },
            scenes: statuses.len(),
            complete: n(F::Complete),
            missing: n(F::Missing),
            needs_review: n(F::NeedsReview),
            suggested,
            rows,
        })
    })?;
    if data.scenes == 0 {
        return Err(AppError::export(
            "nothing_to_export",
            "The Production Source has no scenes to export.",
        ));
    }
    let document = "Breakdown Summary";
    let mut doc = Document::new(document).footer(footer(&data.project, document));
    doc.push(Block::Masthead(production_masthead(
        "PRODUCTION / BREAKDOWN REPORT",
        "Production breakdown",
        "Breakdown Summary",
        &data.project,
        &data.source_label,
        &super::today_label(),
        "Entire production source",
    )));
    doc.push(Block::Metrics(Metrics {
        items: vec![
            Metric::new(data.scenes.to_string(), "Scenes"),
            Metric::new(
                format!("{} of {}", data.complete, data.scenes),
                "Breakdown complete",
            ),
            Metric::new(data.missing.to_string(), "No breakdown yet"),
            Metric::new(data.needs_review.to_string(), "Changed since review"),
            Metric::new(data.suggested.to_string(), "Suggestions waiting"),
        ],
        compact: false,
    }));
    let mut table = Table::new(
        "Breakdown Summary",
        vec![
            Column::new("Category"),
            Column::right("Items"),
            Column::right("Scenes"),
            Column::new("Items needed").weight(3.0),
        ],
    );
    for r in &data.rows {
        let shown: Vec<&str> = r.items.iter().take(12).map(|s| s.as_str()).collect();
        let more = r.items.len().saturating_sub(shown.len());
        table.push(vec![
            r.category.clone(),
            r.items.len().to_string(),
            r.scenes.to_string(),
            if more > 0 {
                format!("{}, and {more} more", shown.join(", "))
            } else {
                shown.join(", ")
            },
        ]);
    }
    doc.push(Block::Section(Section::new(
        "Items needed by category",
        Some("Confirmed production data only".into()),
    )));
    if data.rows.is_empty() {
        doc.push(Block::Text(
            Txt::new("No confirmed breakdown elements yet.")
                .size(8.8)
                .color(pal::FAINT)
                .after(6.0),
        ));
    } else {
        let mut t = StyledTable::new(vec![
            Col::pt("Category", 128.0),
            Col::pt("Items", 46.0).align(Align::Right),
            Col::pt("Scenes", 54.0).align(Align::Right),
            Col::weight("Items needed", 1.0),
        ]);
        t.boxed = true;
        t.continued = Some("Items needed by category".into());
        for r in &data.rows {
            let num = |n: usize| {
                Cell::text(
                    Txt::new(n.to_string())
                        .size(11.0)
                        .bold()
                        .color(pal::NAVY)
                        .align(Align::Right),
                )
            };
            t.rows.push(Row::new(vec![
                Cell::new().item(CellItem::Heading {
                    dot: Some(pal::category(&r.category)),
                    text: r.category.clone(),
                    right: None,
                }),
                num(r.items.len()),
                num(r.scenes),
                Cell::new().item(CellItem::Chips(
                    r.items
                        .iter()
                        .map(|n| Chip::pill(n.clone(), Tone::Neutral))
                        .collect(),
                )),
            ]));
        }
        doc.push(Block::Styled(t));
    }
    let mut out = Output::new(doc, HINT_TABLES);
    out.tables.push(table);
    let job = Job {
        action: "breakdown.export_report",
        document: document.to_string(),
        source_label: data.source_label,
        scope_label: "Entire Production Source".to_string(),
        contents_label: format!(
            "{} · {}",
            plural(data.scenes, "scene", "scenes"),
            plural(data.rows.len(), "category", "categories")
        ),
        target: None,
    };
    deliver(core, actor, &dest, fmt, job, out)
}
