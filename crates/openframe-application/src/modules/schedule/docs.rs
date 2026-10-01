//! Sides (FSD §58, §111) and basic production reports (FSD §58, §110).
//!
//! Both are read-only projections. Sides are saved as snapshots of the chosen
//! scenes' screenplay text; editing them never changes the screenplay. Reports
//! are generated on demand and persist only when the user saves a snapshot.

use std::collections::{BTreeMap, HashSet};

use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use super::board::{Board, StripRow, current_schedule};
use super::fmt::{date_short, minutes, pages};
use super::ops::{NO_SOURCE_MESSAGE, read, write};
use super::source::{self, SourceData};
use crate::core::AppCore;
use crate::registry::{Registry, SearchDoc, TrashHandler};
use crate::store::{DeleteSpec, DeletedItemRow, MutationMeta, Tx, soft_delete};
use crate::util::optional_text;

pub fn register(r: &mut Registry) {
    use crate::registry::OperationMetadata as M;
    r.module("Sides & Reports");
    r.query("sides.preview", sides_preview).meta(M::compute(
        "Preview sides (script pages) for a shooting day.",
    ));
    r.query("sides.list", sides_list)
        .meta(M::read("Saved sides."));
    r.query("sides.get", sides_get)
        .meta(M::read("One saved sides document."));
    r.command("sides.create", sides_create)
        .meta(M::edit("Save sides for a shooting day."));
    r.command("sides.delete", sides_delete)
        .meta(M::soft_delete("Move saved sides to Recently Deleted."));
    r.query("reports.generate", reports_generate).meta(M::compute("Generate a production report (scene, location, cast/scene, prop, schedule, breakdown completeness)."));
    r.query("reports.list", reports_list)
        .meta(M::read("Saved production reports."));
    r.query("reports.get", reports_get)
        .meta(M::read("One saved production report."));
    r.command("reports.save", reports_save)
        .meta(M::edit("Save a production report snapshot."));
    r.command("reports.delete", reports_delete)
        .meta(M::soft_delete("Move a saved report to Recently Deleted."));
    r.indexer("side", index_side);
    r.trash_handler(TrashHandler {
        object_type: "side",
        table: "side",
        label: "Sides",
        restore: None,
        purge: purge_side,
    });
    r.trash_handler(TrashHandler {
        object_type: "production_report",
        table: "production_report",
        label: "Saved Report",
        restore: None,
        purge: purge_report,
    });
}

// ================================================================== sides

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleSidesElement {
    pub element_type: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleSidesScene {
    pub scene_id: String,
    pub number: String,
    pub heading: String,
    pub elements: Vec<ScheduleSidesElement>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleSidesCoverCast {
    pub character: String,
    pub actor: Option<String>,
    pub call_time: Option<String>,
}

/// Optional call-sheet cover page information.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleSidesCover {
    pub crew_call: Option<String>,
    pub locations: Vec<String>,
    pub cast: Vec<ScheduleSidesCoverCast>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleSidesContent {
    /// "BLACK RAIN · SIDES · SHOOT DAY 4 · Draft 6 — Shooting Draft".
    pub header: String,
    pub production_title: String,
    pub day_label: Option<String>,
    pub date_label: Option<String>,
    /// Screenplay source/draft name (FSD §111).
    pub source_label: String,
    pub show_draft_name: bool,
    pub cover: Option<ScheduleSidesCover>,
    pub scenes: Vec<ScheduleSidesScene>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleSidesArgs {
    /// Shooting day whose scenes to include.
    #[serde(default)]
    pub day_id: Option<String>,
    /// Selected scenes (strip ids). Empty = every scene of the day.
    #[serde(default)]
    pub strip_ids: Vec<String>,
    /// "Selected cast": only scenes with at least one of these cast keys.
    #[serde(default)]
    pub cast_keys: Option<Vec<String>>,
    #[serde(default)]
    pub include_cover: bool,
    #[serde(default = "yes")]
    pub show_draft_name: bool,
    /// Optional title when saving.
    #[serde(default)]
    pub title: Option<String>,
}

fn yes() -> bool {
    true
}

fn build_sides(
    c: &Connection,
    a: &ScheduleSidesArgs,
) -> AppResult<(ScheduleSidesContent, String, Option<String>)> {
    let sched = current_schedule(c)?.ok_or_else(|| {
        AppError::new(
            "validation.no_schedule",
            "Create a shooting schedule first.",
        )
    })?;
    let board = Board::load(c, sched)?;
    let Some(src) = board.source.as_ref() else {
        return Err(AppError::new("validation.no_source", NO_SOURCE_MESSAGE));
    };
    let day = match &a.day_id {
        Some(id) => Some(
            board
                .day(id)
                .ok_or_else(|| AppError::not_found("shooting day"))?,
        ),
        None => None,
    };
    let mut strips: Vec<&StripRow> = if a.strip_ids.is_empty() {
        match day {
            Some(d) => board.day_strips(&d.id),
            None => {
                return Err(AppError::required(
                    "A shooting day or a selection of scenes",
                ));
            }
        }
    } else {
        a.strip_ids
            .iter()
            .map(|id| {
                board
                    .strips
                    .iter()
                    .find(|s| s.id == *id)
                    .ok_or_else(|| AppError::not_found("scene strip"))
            })
            .collect::<AppResult<_>>()?
    };
    if let Some(keys) = &a.cast_keys {
        let wanted: HashSet<&str> = keys.iter().map(|k| k.as_str()).collect();
        strips.retain(|s| {
            board
                .cast(s)
                .iter()
                .any(|c| wanted.contains(c.key.as_str()))
        });
    }
    let mut scenes = Vec::new();
    for s in &strips {
        let Some(sc) = board.scene(s) else { continue };
        scenes.push(ScheduleSidesScene {
            scene_id: sc.id.clone(),
            number: sc.number.to_string(),
            heading: sc.heading.clone(),
            elements: source::scene_elements(c, &sc.id)?
                .into_iter()
                .filter(|e| e.element_type != "note")
                .map(|e| ScheduleSidesElement {
                    element_type: e.element_type,
                    text: e.text,
                })
                .collect(),
        });
    }
    if scenes.is_empty() {
        return Err(AppError::invalid_input("No scenes match this selection."));
    }
    let cover = if a.include_cover {
        day.map(|d| -> AppResult<ScheduleSidesCover> {
            let brief = board.call_sheet_brief(d);
            let (crew, times) = match &brief {
                Some(b) => super::callsheet::call_times(c, &b.id)?,
                None => (None, Default::default()),
            };
            let sum = board.day_summary(d);
            Ok(ScheduleSidesCover {
                crew_call: crew,
                locations: sum
                    .locations
                    .iter()
                    .map(|l| match &l.address {
                        Some(ad) => format!("{} — {ad}", l.name),
                        None => l.name.clone(),
                    })
                    .collect(),
                cast: sum
                    .cast
                    .iter()
                    .map(|x| ScheduleSidesCoverCast {
                        character: x.character.clone(),
                        actor: x.actor.clone(),
                        call_time: times.get(&x.key).cloned(),
                    })
                    .collect(),
                notes: d.notes.clone(),
            })
        })
        .transpose()?
    } else {
        None
    };
    let source_label = format!("{} — {}", src.info.screenplay_title, src.info.label);
    let mut header = vec![board.project_title.to_uppercase(), "SIDES".to_string()];
    if let Some(d) = day {
        header.push(d.label().to_uppercase());
    }
    if a.show_draft_name {
        header.push(src.info.label.clone());
    }
    Ok((
        ScheduleSidesContent {
            header: header.join(" · "),
            production_title: board.project_title.clone(),
            day_label: day.map(|d| d.label()),
            date_label: day.and_then(|d| d.date.as_deref().map(date_short)),
            source_label,
            show_draft_name: a.show_draft_name,
            cover,
            scenes,
        },
        src.info.draft_id.clone(),
        Some(board.schedule.id.clone()),
    ))
}

fn sides_preview(
    core: &AppCore,
    actor: &Actor,
    a: ScheduleSidesArgs,
) -> AppResult<ScheduleSidesContent> {
    read(core, actor, |c| Ok(build_sides(c, &a)?.0))
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleSideRow {
    pub id: String,
    pub title: String,
    pub source_label: String,
    #[ts(type = "number")]
    pub scene_count: i64,
    #[ts(type = "number")]
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleSideDetail {
    pub id: String,
    pub title: String,
    pub content: ScheduleSidesContent,
    #[ts(type = "number")]
    pub created_at: i64,
}

/// Save a Sides snapshot (the screenplay is never changed by it).
fn sides_create(core: &AppCore, actor: &Actor, a: ScheduleSidesArgs) -> AppResult<String> {
    let title = optional_text(a.title.clone(), "Title", 160)?;
    let meta = MutationMeta::new("sides.create", "Saved sides", Capability::Edit);
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        let (content, draft_id, schedule_id) = build_sides(c, &a)?;
        let title = title.unwrap_or_else(|| {
            let what = content
                .day_label
                .clone()
                .unwrap_or_else(|| format!("{} scenes", content.scenes.len()));
            format!("Sides — {what}")
        });
        let id = new_id();
        let now = now_ms();
        let scope = json!({ "dayId": a.day_id, "stripIds": a.strip_ids, "castKeys": a.cast_keys, "includeCover": a.include_cover });
        c.execute(
            "INSERT INTO side(id, schedule_id, shoot_day_id, title, scope_json, source_draft_id, source_label, content_json, created_by, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
            params![
                id,
                schedule_id,
                a.day_id,
                title,
                scope.to_string(),
                draft_id,
                content.source_label,
                serde_json::to_string(&content).map_err(|e| AppError::internal(e.to_string()))?,
                tx.actor().user_id,
                now
            ],
        )?;
        Ok(id)
    })
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleDocListArgs {}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleDocIdArgs {
    pub id: String,
}

fn sides_list(
    core: &AppCore,
    actor: &Actor,
    _: ScheduleDocListArgs,
) -> AppResult<Vec<ScheduleSideRow>> {
    read(core, actor, |c| {
        let mut st = c.prepare("SELECT id, title, source_label, content_json, created_at FROM side WHERE deleted_at IS NULL ORDER BY created_at DESC")?;
        let rows = st.query_map([], |r| {
            let content: String = r.get(3)?;
            let n = serde_json::from_str::<ScheduleSidesContent>(&content)
                .map(|x| x.scenes.len() as i64)
                .unwrap_or(0);
            Ok(ScheduleSideRow {
                id: r.get(0)?,
                title: r.get(1)?,
                source_label: r.get(2)?,
                scene_count: n,
                created_at: r.get(4)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    })
}

fn sides_get(core: &AppCore, actor: &Actor, a: ScheduleDocIdArgs) -> AppResult<ScheduleSideDetail> {
    read(core, actor, |c| {
        let row: Option<(String, String, String, i64)> = c
            .query_row(
                "SELECT id, title, content_json, created_at FROM side WHERE id = ?1 AND deleted_at IS NULL",
                [&a.id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        let (id, title, content, created_at) = row.ok_or_else(|| AppError::not_found("sides"))?;
        let content =
            serde_json::from_str(&content).map_err(|e| AppError::internal(e.to_string()))?;
        Ok(ScheduleSideDetail {
            id,
            title,
            content,
            created_at,
        })
    })
}

fn sides_delete(core: &AppCore, actor: &Actor, a: ScheduleDocIdArgs) -> AppResult<()> {
    let meta = MutationMeta::new(
        "sides.delete",
        "Deleted saved sides",
        Capability::SoftDelete,
    )
    .target("side", &a.id);
    write(core, actor, meta, |tx| {
        let title: String = tx
            .conn()
            .query_row(
                "SELECT title FROM side WHERE id = ?1 AND deleted_at IS NULL",
                [&a.id],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| AppError::not_found("sides"))?;
        soft_delete(
            tx,
            DeleteSpec {
                object_type: "side",
                table: "side",
                id: &a.id,
                title: Some(title),
                parent_type: None,
                parent_id: None,
                position: None,
            },
        )
    })
}

fn purge_side(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    tx.conn()
        .execute("DELETE FROM side WHERE id = ?1", [&row.object_id])?;
    Ok(())
}

fn index_side(c: &Connection, id: &str) -> AppResult<Option<SearchDoc>> {
    let row: Option<(String, String)> = c
        .query_row(
            "SELECT title, content_json FROM side WHERE id = ?1 AND deleted_at IS NULL",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((title, content)) = row else {
        return Ok(None);
    };
    let content: Option<ScheduleSidesContent> = serde_json::from_str(&content).ok();
    let body = content
        .map(|x| {
            x.scenes
                .iter()
                .map(|s| format!("{} {}", s.number, s.heading))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    Ok(Some(SearchDoc {
        entity_type: "side".into(),
        title,
        body,
        context: "Production".into(),
        nav: json!({ "workspace": "production", "sub": "sides", "sideId": id }),
        owner_user_id: None,
    }))
}

// ================================================================== reports

pub const REPORT_TYPES: [(&str, &str); 6] = [
    ("scene", "Scene Report"),
    ("location", "Location Report"),
    ("cast_scene", "Cast Scene Report"),
    ("prop", "Prop Report"),
    ("schedule", "Schedule Report"),
    ("breakdown_completeness", "Breakdown Completeness"),
];

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ReportData {
    pub report_type: String,
    pub title: String,
    pub source_label: String,
    #[ts(type = "number")]
    pub generated_at: i64,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    /// Shown when the table is empty ("No matching data.") or data is partial.
    pub note: Option<String>,
    /// The text filter the rows were narrowed by (kept with saved snapshots).
    #[serde(default)]
    pub filter: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReportArgs {
    pub report_type: String,
    #[serde(default)]
    pub title: Option<String>,
    /// Optional case-insensitive text filter: keeps rows where any cell contains it.
    #[serde(default)]
    pub filter: Option<String>,
}

/// Normalise a user-typed filter (trimmed, at most 200 characters; blank = none).
fn report_filter(f: Option<&str>) -> AppResult<Option<String>> {
    let f = f.map(str::trim).unwrap_or_default();
    if f.is_empty() {
        return Ok(None);
    }
    if f.chars().count() > 200 {
        return Err(AppError::validation(
            "filter",
            "Keep the filter under 200 characters.",
        ));
    }
    Ok(Some(f.to_string()))
}

fn report_title(t: &str) -> AppResult<&'static str> {
    REPORT_TYPES
        .iter()
        .find(|(k, _)| *k == t)
        .map(|(_, v)| *v)
        .ok_or_else(|| AppError::invalid_input("Choose one of the available reports."))
}

pub(crate) fn build_report(
    c: &Connection,
    report_type: &str,
    filter: Option<&str>,
) -> AppResult<ReportData> {
    let title = report_title(report_type)?.to_string();
    let filter = report_filter(filter)?;
    let board = match current_schedule(c)? {
        Some(s) => Some(Board::load(c, s)?),
        None => None,
    };
    let owned;
    let src: Option<&SourceData> = match board.as_ref().and_then(|b| b.source.as_ref()) {
        Some(s) => Some(s),
        None => match source::active_source(c)? {
            Some(info) => {
                owned = source::load_source(c, &info.id)?;
                owned.as_ref()
            }
            None => None,
        },
    };
    let now = now_ms();
    let Some(src) = src else {
        return Ok(ReportData {
            report_type: report_type.to_string(),
            title,
            source_label: String::new(),
            generated_at: now,
            columns: Vec::new(),
            rows: Vec::new(),
            note: Some(NO_SOURCE_MESSAGE.to_string()),
            filter,
        });
    };
    let day_of = |scene_id: &str| -> Option<String> {
        let b = board.as_ref()?;
        let s = b.strips.iter().find(|s| s.scene_id == scene_id)?;
        b.strip_day(s).and_then(|d| b.day(d)).map(|d| d.title())
    };
    let scenes: Vec<_> = src.scenes.iter().filter(|s| !s.omitted).collect();
    let (columns, rows): (Vec<&str>, Vec<Vec<String>>) = match report_type {
        "scene" => (
            vec![
                "Sc",
                "Heading",
                "I/E · D/N",
                "Pages",
                "Location",
                "Cast",
                "Scheduled",
            ],
            scenes
                .iter()
                .map(|sc| {
                    vec![
                        sc.number.to_string(),
                        sc.heading.clone(),
                        format!(
                            "{} · {}",
                            if sc.int_ext.is_empty() {
                                "—"
                            } else {
                                &sc.int_ext
                            },
                            if sc.day_night.is_empty() {
                                "—"
                            } else {
                                &sc.day_night
                            }
                        ),
                        pages(sc.page_eighths),
                        src.locations(sc)
                            .iter()
                            .map(|l| l.name.clone())
                            .collect::<Vec<_>>()
                            .join(", "),
                        src.cast(&sc.id)
                            .iter()
                            .map(|c| c.character.clone())
                            .collect::<Vec<_>>()
                            .join(", "),
                        day_of(&sc.id).unwrap_or_else(|| "Unscheduled".into()),
                    ]
                })
                .collect(),
        ),
        "location" => {
            let mut by: BTreeMap<String, (String, String, String, Vec<String>, Vec<String>)> =
                BTreeMap::new();
            for sc in &scenes {
                for l in src.locations(sc) {
                    let e = by.entry(l.name.to_lowercase()).or_insert_with(|| {
                        (
                            l.name.clone(),
                            l.status.clone().unwrap_or_else(|| "From heading".into()),
                            l.address.clone().unwrap_or_default(),
                            Vec::new(),
                            Vec::new(),
                        )
                    });
                    e.3.push(sc.number.to_string());
                    if let Some(d) = day_of(&sc.id)
                        && !e.4.contains(&d)
                    {
                        e.4.push(d);
                    }
                }
            }
            (
                vec!["Location", "Status", "Address", "Scenes", "Shooting days"],
                by.into_values()
                    .map(|(n, st, ad, sc, days)| vec![n, st, ad, sc.join(", "), days.join("; ")])
                    .collect(),
            )
        }
        "cast_scene" => {
            let mut by: BTreeMap<String, (String, String, Vec<String>, Vec<String>)> =
                BTreeMap::new();
            for sc in &scenes {
                for c in src.cast(&sc.id) {
                    let e = by.entry(c.key.clone()).or_insert_with(|| {
                        (
                            c.actor.clone().unwrap_or_else(|| "Not cast yet".into()),
                            c.character.clone(),
                            Vec::new(),
                            Vec::new(),
                        )
                    });
                    e.2.push(sc.number.to_string());
                    if let Some(d) = day_of(&sc.id)
                        && !e.3.contains(&d)
                    {
                        e.3.push(d);
                    }
                }
            }
            let mut rows: Vec<Vec<String>> = by
                .into_values()
                .map(|(a, ch, sc, days)| {
                    vec![a, ch, sc.join(", "), sc.len().to_string(), days.join("; ")]
                })
                .collect();
            rows.sort_by(|a, b| a[1].cmp(&b[1]));
            (
                vec![
                    "Actor",
                    "Character",
                    "Scenes",
                    "Scene count",
                    "Shooting days",
                ],
                rows,
            )
        }
        "prop" => {
            let mut by: BTreeMap<String, (String, Vec<String>)> = BTreeMap::new();
            for sc in &scenes {
                for it in src.items(&sc.id).iter().filter(|i| i.category == "Props") {
                    by.entry(it.name.to_lowercase())
                        .or_insert_with(|| (it.name.clone(), Vec::new()))
                        .1
                        .push(sc.number.to_string());
                }
            }
            (
                vec!["Prop", "Scenes", "Scene count"],
                by.into_values()
                    .map(|(n, s)| vec![n, s.join(", "), s.len().to_string()])
                    .collect(),
            )
        }
        "schedule" => {
            let mut rows = Vec::new();
            if let Some(b) = board.as_ref() {
                for d in &b.days {
                    let sum = b.day_summary(d);
                    let strips = b.day_strips(&d.id);
                    rows.push(vec![
                        d.label(),
                        d.date
                            .as_deref()
                            .map(date_short)
                            .unwrap_or_else(|| "No date".into()),
                        strips
                            .iter()
                            .filter_map(|s| b.scene_number(s))
                            .map(|n| n.to_string())
                            .collect::<Vec<_>>()
                            .join(", "),
                        sum.locations
                            .iter()
                            .map(|l| l.name.clone())
                            .collect::<Vec<_>>()
                            .join(", "),
                        if sum.estimated_minutes > 0 {
                            minutes(sum.estimated_minutes)
                        } else {
                            "Missing".into()
                        },
                    ]);
                }
                let un = b.unscheduled().len();
                if un > 0 {
                    rows.push(vec![
                        "Unscheduled".into(),
                        String::new(),
                        format!("{un} scenes"),
                        String::new(),
                        String::new(),
                    ]);
                }
            }
            (
                vec!["Day", "Date", "Scenes", "Locations", "Est. time"],
                rows,
            )
        }
        _ => (
            vec![
                "Sc",
                "Heading",
                "Confirmed",
                "Suggested (unreviewed)",
                "Status",
            ],
            scenes
                .iter()
                .map(|sc| {
                    let (conf, sug) = src.breakdown_counts(&sc.id);
                    let status = match (conf, sug) {
                        (0, 0) => "Not started",
                        (_, 0) => "Reviewed",
                        (0, _) => "Needs review",
                        _ => "In progress",
                    };
                    vec![
                        sc.number.to_string(),
                        sc.heading.clone(),
                        conf.to_string(),
                        sug.to_string(),
                        status.to_string(),
                    ]
                })
                .collect(),
        ),
    };
    let rows: Vec<Vec<String>> = match filter.as_deref() {
        Some(f) => {
            let needle = f.to_lowercase();
            rows.into_iter()
                .filter(|r| r.iter().any(|cell| cell.to_lowercase().contains(&needle)))
                .collect()
        }
        None => rows,
    };
    let note = if rows.is_empty() {
        Some("No matching data.".to_string())
    } else {
        None
    };
    Ok(ReportData {
        report_type: report_type.to_string(),
        title,
        source_label: format!("{} — {}", src.info.screenplay_title, src.info.label),
        generated_at: now,
        columns: columns.into_iter().map(String::from).collect(),
        rows,
        note,
        filter,
    })
}

fn reports_generate(core: &AppCore, actor: &Actor, a: ReportArgs) -> AppResult<ReportData> {
    read(core, actor, |c| {
        build_report(c, &a.report_type, a.filter.as_deref())
    })
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ReportSavedRow {
    pub id: String,
    pub title: String,
    pub report_type: String,
    #[ts(type = "number")]
    pub generated_at: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ReportSaved {
    pub id: String,
    pub title: String,
    pub data: ReportData,
}

/// Save the current projection as an immutable snapshot.
fn reports_save(core: &AppCore, actor: &Actor, a: ReportArgs) -> AppResult<String> {
    let base = report_title(&a.report_type)?;
    let title = optional_text(a.title.clone(), "Title", 160)?;
    let meta = MutationMeta::new(
        "reports.save",
        format!("Saved a {base} snapshot"),
        Capability::Edit,
    );
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        let data = build_report(c, &a.report_type, a.filter.as_deref())?;
        let id = new_id();
        let now = now_ms();
        let title = title.unwrap_or_else(|| format!("{base} — {}", data.source_label));
        c.execute(
            "INSERT INTO production_report(id, report_type, title, source_scope_json, snapshot_json, generated_at, created_by, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?6, ?6)",
            params![
                id,
                a.report_type,
                title,
                json!({ "source": data.source_label, "filter": data.filter }).to_string(),
                serde_json::to_string(&data).map_err(|e| AppError::internal(e.to_string()))?,
                now,
                tx.actor().user_id
            ],
        )?;
        Ok(id)
    })
}

fn reports_list(
    core: &AppCore,
    actor: &Actor,
    _: ScheduleDocListArgs,
) -> AppResult<Vec<ReportSavedRow>> {
    read(core, actor, |c| {
        let mut st = c.prepare(
            "SELECT id, title, report_type, generated_at FROM production_report WHERE deleted_at IS NULL ORDER BY generated_at DESC",
        )?;
        let rows = st.query_map([], |r| {
            Ok(ReportSavedRow {
                id: r.get(0)?,
                title: r.get(1)?,
                report_type: r.get(2)?,
                generated_at: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    })
}

fn reports_get(core: &AppCore, actor: &Actor, a: ScheduleDocIdArgs) -> AppResult<ReportSaved> {
    read(core, actor, |c| {
        let row: Option<(String, String, String)> = c
            .query_row(
                "SELECT id, title, snapshot_json FROM production_report WHERE id = ?1 AND deleted_at IS NULL",
                [&a.id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let (id, title, data) = row.ok_or_else(|| AppError::not_found("report"))?;
        Ok(ReportSaved {
            id,
            title,
            data: serde_json::from_str(&data).map_err(|e| AppError::internal(e.to_string()))?,
        })
    })
}

fn reports_delete(core: &AppCore, actor: &Actor, a: ScheduleDocIdArgs) -> AppResult<()> {
    let meta = MutationMeta::new(
        "reports.delete",
        "Deleted a saved report",
        Capability::SoftDelete,
    )
    .target("production_report", &a.id);
    write(core, actor, meta, |tx| {
        let title: String = tx
            .conn()
            .query_row(
                "SELECT title FROM production_report WHERE id = ?1 AND deleted_at IS NULL",
                [&a.id],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| AppError::not_found("report"))?;
        soft_delete(
            tx,
            DeleteSpec {
                object_type: "production_report",
                table: "production_report",
                id: &a.id,
                title: Some(title),
                parent_type: None,
                parent_id: None,
                position: None,
            },
        )
    })
}

fn purge_report(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    tx.conn().execute(
        "DELETE FROM production_report WHERE id = ?1",
        [&row.object_id],
    )?;
    Ok(())
}
