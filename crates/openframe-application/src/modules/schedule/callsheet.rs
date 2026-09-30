//! Call sheets (FSD §38, §105, §147–148; FSD-CALL-001…012).
//!
//! A call sheet is created from one shooting day and prefilled from the
//! schedule and project data. From then on it is its own document: edits never
//! flow back into the schedule (FSD-CALL-002 / §38.7). When the day changes the
//! sheet shows "Schedule changed — Refresh"; refreshing previews differences
//! and only applies them on request. Finalize freezes an immutable snapshot;
//! later changes create a new revision (FSD §38.8, §126).

use std::collections::HashMap;
use std::path::PathBuf;

use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_persistence::rows::{int, text, update_fields};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use super::board::{Board, CallSheetSource, effective_status, load_days, load_schedule};
use super::fmt::{date_long_upper, date_short};
use super::ops::{read, write};
use crate::core::AppCore;
use crate::registry::{Registry, SearchDoc, TrashHandler};
use crate::store::{DeleteSpec, DeletedItemRow, MutationMeta, Tx, soft_delete};
use crate::util::ingest_file;

pub fn register(r: &mut Registry) {
    r.query("callsheets.list", list);
    r.query("callsheets.get", get);
    r.query("callsheets.refresh_preview", refresh_preview);
    r.command("callsheets.create", create);
    r.command("callsheets.update", update);
    r.command("callsheets.set_ready", set_ready);
    r.command("callsheets.refresh", refresh);
    r.command("callsheets.finalize", finalize);
    r.command("callsheets.issue", issue);
    r.command("callsheets.new_revision", new_revision);
    r.command("callsheets.attach", attach);
    r.command("callsheets.delete", delete);
    r.indexer("call_sheet", index_call_sheet);
    r.trash_handler(TrashHandler {
        object_type: "call_sheet",
        table: "call_sheet",
        label: "Call Sheet",
        restore: None,
        purge: purge_call_sheet,
    });
}

// ------------------------------------------------------------------ document

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct CallSheetDocCast {
    pub key: String,
    pub actor: String,
    pub character: String,
    pub call_time: String,
    pub notes: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct CallSheetDocScene {
    pub key: String,
    pub number: String,
    pub heading: String,
    pub description: String,
    pub pages: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct CallSheetDocLocation {
    pub key: String,
    pub name: String,
    pub address: String,
    pub meeting_point: String,
    pub parking: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct CallSheetPractical {
    pub parking: String,
    pub meeting_point: String,
    pub travel_notes: String,
    pub meal_break: String,
    pub emergency_contact: String,
    pub production_notes: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct CallSheetAttachment {
    pub asset_id: String,
    pub name: String,
    pub media_type: String,
}

/// Optional sections stay hidden (None) until the user adds them (FSD §38.4).
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct CallSheetOptional {
    pub weather: Option<String>,
    pub special_notes: Option<String>,
    pub attachments: Option<Vec<CallSheetAttachment>>,
    pub reference_images: Option<Vec<CallSheetAttachment>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct CallSheetExtraField {
    pub label: String,
    pub value: String,
}

/// Editable call-sheet content.
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct CallSheetDocument {
    pub title: String,
    /// "SHOOT DAY 4".
    pub day_label: String,
    pub date: Option<String>,
    pub crew_call: String,
    pub day_notes: String,
    pub locations: Vec<CallSheetDocLocation>,
    pub cast: Vec<CallSheetDocCast>,
    pub scenes: Vec<CallSheetDocScene>,
    pub practical: CallSheetPractical,
    pub optional: CallSheetOptional,
    pub extra_fields: Vec<CallSheetExtraField>,
}

fn breaks_text(src: &CallSheetSource, types: &[&str]) -> String {
    src.breaks
        .iter()
        .filter(|b| types.contains(&b.marker_type.as_str()))
        .map(|b| match &b.at_time {
            Some(t) => format!("{} {t}", b.label),
            None => b.label.clone(),
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

/// FSD-CALL-001…004: prefill from the shooting day; nothing is re-entered.
pub fn prefill(src: &CallSheetSource) -> CallSheetDocument {
    CallSheetDocument {
        title: src.production_title.clone(),
        day_label: src.day_label.to_uppercase(),
        date: src.date.clone(),
        crew_call: String::new(),
        day_notes: src.day_notes.clone().unwrap_or_default(),
        locations: src
            .locations
            .iter()
            .map(|l| CallSheetDocLocation {
                key: l.key.clone(),
                name: l.name.clone(),
                address: l.address.clone().unwrap_or_default(),
                ..Default::default()
            })
            .collect(),
        cast: src
            .cast
            .iter()
            .map(|c| CallSheetDocCast {
                key: c.key.clone(),
                actor: c.actor.clone().unwrap_or_default(),
                character: c.character.clone(),
                ..Default::default()
            })
            .collect(),
        scenes: src
            .scenes
            .iter()
            .map(|s| CallSheetDocScene {
                key: s.key.clone(),
                number: s.number.clone(),
                heading: s.heading.clone(),
                description: s.description.clone(),
                pages: s.pages.clone(),
            })
            .collect(),
        practical: CallSheetPractical {
            meal_break: breaks_text(src, &["Meal"]),
            travel_notes: breaks_text(src, &["Travel", "Company Move"]),
            ..Default::default()
        },
        optional: CallSheetOptional::default(),
        extra_fields: Vec::new(),
    }
}

/// Apply a newer source to an edited document. Derived data (scenes, cast,
/// locations, date) follows the schedule; the user's call times, notes and
/// practical fields are kept where they still apply.
pub fn merge(
    doc: &CallSheetDocument,
    old: &CallSheetSource,
    new: &CallSheetSource,
) -> CallSheetDocument {
    let fresh = prefill(new);
    let before = prefill(old);
    let edited = |current: &str, derived_before: &str, derived_now: &str| -> String {
        if current == derived_before {
            derived_now.to_string()
        } else {
            current.to_string()
        }
    };
    let cast = fresh
        .cast
        .iter()
        .map(|c| match doc.cast.iter().find(|x| x.key == c.key) {
            Some(prev) => CallSheetDocCast {
                call_time: prev.call_time.clone(),
                notes: prev.notes.clone(),
                actor: if c.actor.is_empty() {
                    prev.actor.clone()
                } else {
                    c.actor.clone()
                },
                ..c.clone()
            },
            None => c.clone(),
        })
        .collect();
    let locations = fresh
        .locations
        .iter()
        .map(|l| match doc.locations.iter().find(|x| x.key == l.key) {
            Some(prev) => CallSheetDocLocation {
                address: if l.address.is_empty() {
                    prev.address.clone()
                } else {
                    l.address.clone()
                },
                meeting_point: prev.meeting_point.clone(),
                parking: prev.parking.clone(),
                ..l.clone()
            },
            None => l.clone(),
        })
        .collect();
    CallSheetDocument {
        title: edited(&doc.title, &before.title, &fresh.title),
        day_label: fresh.day_label,
        date: fresh.date,
        crew_call: doc.crew_call.clone(),
        day_notes: edited(&doc.day_notes, &before.day_notes, &fresh.day_notes),
        locations,
        cast,
        scenes: fresh.scenes,
        practical: CallSheetPractical {
            meal_break: edited(
                &doc.practical.meal_break,
                &before.practical.meal_break,
                &fresh.practical.meal_break,
            ),
            travel_notes: edited(
                &doc.practical.travel_notes,
                &before.practical.travel_notes,
                &fresh.practical.travel_notes,
            ),
            ..doc.practical.clone()
        },
        optional: doc.optional.clone(),
        extra_fields: doc.extra_fields.clone(),
    }
}

/// Human list of what differs between two sources (FSD §125: say what changed).
fn changed_areas(old: &CallSheetSource, new: &CallSheetSource) -> Vec<String> {
    let mut v = Vec::new();
    if old.date != new.date {
        v.push("Date".to_string());
    }
    if old.day_number != new.day_number {
        v.push("Day number".to_string());
    }
    if old.scenes != new.scenes {
        v.push("Scenes".to_string());
    }
    if old.cast != new.cast {
        v.push("Cast".to_string());
    }
    if old.locations != new.locations {
        v.push("Locations".to_string());
    }
    if old.day_notes != new.day_notes {
        v.push("Day notes".to_string());
    }
    if old.breaks != new.breaks {
        v.push("Breaks".to_string());
    }
    if old.production_title != new.production_title {
        v.push("Production title".to_string());
    }
    v
}

// ------------------------------------------------------------------ rows

struct SheetRow {
    id: String,
    schedule_id: String,
    day_id: String,
    revision: i64,
    title: String,
    status: String,
    needs_refresh: bool,
    source: CallSheetSource,
    fingerprint: String,
    captured_at: i64,
    document: CallSheetDocument,
    snapshot_id: Option<String>,
    finalized_at: Option<i64>,
    issued_at: Option<i64>,
    superseded_by: Option<String>,
    previous_id: Option<String>,
    updated_at: i64,
    rev: i64,
}

fn load_sheet(c: &Connection, id: &str) -> AppResult<SheetRow> {
    c.query_row(
        "SELECT id, schedule_id, shoot_day_id, revision, title, status, needs_refresh, source_snapshot_json,
                source_fingerprint, source_captured_at, document_json, snapshot_id, finalized_at, issued_at,
                superseded_by, previous_id, updated_at, rev
         FROM call_sheet WHERE id = ?1 AND deleted_at IS NULL",
        [id],
        |r| {
            let src: String = r.get(7)?;
            let doc: String = r.get(10)?;
            Ok(SheetRow {
                id: r.get(0)?,
                schedule_id: r.get(1)?,
                day_id: r.get(2)?,
                revision: r.get(3)?,
                title: r.get(4)?,
                status: r.get(5)?,
                needs_refresh: r.get::<_, i64>(6)? != 0,
                source: serde_json::from_str(&src).unwrap_or_else(|_| empty_source()),
                fingerprint: r.get(8)?,
                captured_at: r.get(9)?,
                document: serde_json::from_str(&doc).unwrap_or_default(),
                snapshot_id: r.get(11)?,
                finalized_at: r.get(12)?,
                issued_at: r.get(13)?,
                superseded_by: r.get(14)?,
                previous_id: r.get(15)?,
                updated_at: r.get(16)?,
                rev: r.get(17)?,
            })
        },
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("call sheet"))
}

fn empty_source() -> CallSheetSource {
    CallSheetSource {
        production_title: String::new(),
        source_label: String::new(),
        day_id: String::new(),
        day_number: None,
        day_label: String::new(),
        date: None,
        day_notes: None,
        scenes: Vec::new(),
        cast: Vec::new(),
        locations: Vec::new(),
        breaks: Vec::new(),
    }
}

fn is_editable(status: &str) -> bool {
    matches!(status, "Draft" | "Ready" | "Needs Refresh")
}

fn ensure_editable(s: &SheetRow) -> AppResult<()> {
    if !is_editable(&s.status) {
        return Err(AppError::new(
            "conflict.call_sheet_final",
            "This call sheet is finalized. Create a new revision to make changes.",
        ));
    }
    Ok(())
}

/// The current source of a sheet's day, or None when the day (or schedule) no longer exists.
fn current_source(c: &Connection, s: &SheetRow) -> AppResult<Option<CallSheetSource>> {
    let sched = match load_schedule(c, &s.schedule_id) {
        Ok(x) => x,
        Err(e) if e.is("not_found") => return Ok(None),
        Err(e) => return Err(e),
    };
    let board = Board::load(c, sched)?;
    Ok(board.day(&s.day_id).map(|d| board.call_sheet_source(d)))
}

fn title_for(src: &CallSheetSource, revision: i64) -> String {
    let base = format!("Call Sheet — {}", src.day_label.replace("Shoot Day", "Day"));
    if revision > 1 {
        format!("{base} (v{revision})")
    } else {
        base
    }
}

/// Call times from a call sheet (for the Daily View).
pub(crate) fn call_times(
    c: &Connection,
    id: &str,
) -> AppResult<(Option<String>, HashMap<String, String>)> {
    let doc: Option<String> = c
        .query_row(
            "SELECT document_json FROM call_sheet WHERE id = ?1",
            [id],
            |r| r.get(0),
        )
        .optional()?;
    let doc: CallSheetDocument = doc
        .and_then(|d| serde_json::from_str(&d).ok())
        .unwrap_or_default();
    let crew = Some(doc.crew_call.trim().to_string()).filter(|s| !s.is_empty());
    let times = doc
        .cast
        .into_iter()
        .filter(|c| !c.call_time.trim().is_empty())
        .map(|c| (c.key, c.call_time.trim().to_string()))
        .collect();
    Ok((crew, times))
}

/// Paths of values that still need input (shown with the red "needs input" style).
fn missing_values(doc: &CallSheetDocument) -> Vec<String> {
    let mut v = Vec::new();
    if doc.crew_call.trim().is_empty() {
        v.push("crewCall".to_string());
    }
    if doc.date.is_none() {
        v.push("date".to_string());
    }
    for (i, c) in doc.cast.iter().enumerate() {
        if c.call_time.trim().is_empty() {
            v.push(format!("cast.{i}.callTime"));
        }
        if c.actor.trim().is_empty() {
            v.push(format!("cast.{i}.actor"));
        }
    }
    for (i, l) in doc.locations.iter().enumerate() {
        if l.address.trim().is_empty() {
            v.push(format!("locations.{i}.address"));
        }
    }
    if doc.practical.emergency_contact.trim().is_empty() {
        v.push("practical.emergencyContact".to_string());
    }
    v
}

// ------------------------------------------------------------------ DTOs

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CallSheetListRow {
    pub id: String,
    pub title: String,
    pub day_id: String,
    pub day_label: String,
    pub date: Option<String>,
    pub date_label: Option<String>,
    #[ts(type = "number")]
    pub revision: i64,
    /// Draft / Needs Refresh / Ready / Final / Issued / Superseded.
    pub status: String,
    pub source_changed: bool,
    pub day_exists: bool,
    #[ts(type = "number")]
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CallSheetStale {
    /// What changed, e.g. ["Scenes", "Date"].
    pub areas: Vec<String>,
    /// When the sheet was prepared from the schedule.
    #[ts(type = "number")]
    pub prepared_at: i64,
    pub day_deleted: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CallSheetRevisionRef {
    pub id: String,
    #[ts(type = "number")]
    pub revision: i64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CallSheetDetail {
    pub id: String,
    pub title: String,
    #[ts(type = "number")]
    pub revision: i64,
    pub status: String,
    pub editable: bool,
    pub source_changed: bool,
    pub stale: Option<CallSheetStale>,
    pub day_id: String,
    /// "Shoot Day 4 — Mon 14 Jun 2027".
    pub day_title: String,
    pub date_long: Option<String>,
    pub document: CallSheetDocument,
    pub source: CallSheetSource,
    pub missing: Vec<String>,
    pub snapshot_id: Option<String>,
    #[ts(type = "number | null")]
    pub finalized_at: Option<i64>,
    #[ts(type = "number | null")]
    pub issued_at: Option<i64>,
    pub superseded_by: Option<String>,
    pub previous_id: Option<String>,
    pub revisions: Vec<CallSheetRevisionRef>,
    #[ts(type = "number")]
    pub updated_at: i64,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CallSheetListArgs {}

fn list(core: &AppCore, actor: &Actor, _: CallSheetListArgs) -> AppResult<Vec<CallSheetListRow>> {
    read(core, actor, |c| {
        let mut st = c.prepare(
            "SELECT id, schedule_id FROM call_sheet WHERE deleted_at IS NULL ORDER BY created_at, revision",
        )?;
        let ids: Vec<(String, String)> = st
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        let mut boards: HashMap<String, Option<Board>> = HashMap::new();
        let mut out = Vec::new();
        for (id, sid) in ids {
            let sheet = load_sheet(c, &id)?;
            if !boards.contains_key(&sid) {
                let b = match load_schedule(c, &sid) {
                    Ok(s) => Some(Board::load(c, s)?),
                    Err(e) if e.is("not_found") => None,
                    Err(e) => return Err(e),
                };
                boards.insert(sid.clone(), b);
            }
            let board = boards.get(&sid).and_then(|b| b.as_ref());
            let day = board.and_then(|b| b.day(&sheet.day_id));
            let changed = match (board, day) {
                (Some(b), Some(d)) => {
                    sheet.needs_refresh || b.call_sheet_source(d).fingerprint() != sheet.fingerprint
                }
                _ => true,
            };
            let date = day
                .and_then(|d| d.date.clone())
                .or(sheet.source.date.clone());
            out.push(CallSheetListRow {
                id: sheet.id,
                title: sheet.title,
                day_id: sheet.day_id,
                day_label: day
                    .map(|d| d.label())
                    .unwrap_or_else(|| sheet.source.day_label.clone()),
                date_label: date.as_deref().map(date_short),
                date,
                revision: sheet.revision,
                status: effective_status(&sheet.status, changed),
                source_changed: changed,
                day_exists: day.is_some(),
                updated_at: sheet.updated_at,
            });
        }
        out.sort_by(|a, b| {
            (a.date.is_none(), &a.date, &a.day_label, -a.revision).cmp(&(
                b.date.is_none(),
                &b.date,
                &b.day_label,
                -b.revision,
            ))
        });
        Ok(out)
    })
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CallSheetIdArgs {
    pub id: String,
}

fn detail(c: &Connection, id: &str) -> AppResult<CallSheetDetail> {
    let s = load_sheet(c, id)?;
    let current = current_source(c, &s)?;
    let (changed, stale) = match &current {
        Some(src) => {
            let changed = s.needs_refresh || src.fingerprint() != s.fingerprint;
            let stale = changed.then(|| CallSheetStale {
                areas: changed_areas(&s.source, src),
                prepared_at: s.captured_at,
                day_deleted: false,
            });
            (changed, stale)
        }
        None => (
            true,
            Some(CallSheetStale {
                areas: Vec::new(),
                prepared_at: s.captured_at,
                day_deleted: true,
            }),
        ),
    };
    let mut st = c.prepare(
        "SELECT id, revision, status FROM call_sheet WHERE shoot_day_id = ?1 AND deleted_at IS NULL ORDER BY revision DESC",
    )?;
    let revisions = st
        .query_map([&s.day_id], |r| {
            Ok(CallSheetRevisionRef {
                id: r.get(0)?,
                revision: r.get(1)?,
                status: r.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let day_title = match &current {
        Some(src) => match &src.date {
            Some(d) => format!("{} — {}", src.day_label, date_short(d)),
            None => src.day_label.clone(),
        },
        None => format!("{} (deleted)", s.source.day_label),
    };
    Ok(CallSheetDetail {
        editable: is_editable(&s.status),
        status: effective_status(&s.status, changed),
        source_changed: changed,
        stale,
        day_title,
        date_long: s.document.date.as_deref().map(date_long_upper),
        missing: missing_values(&s.document),
        id: s.id,
        title: s.title,
        revision: s.revision,
        day_id: s.day_id,
        document: s.document,
        source: s.source,
        snapshot_id: s.snapshot_id,
        finalized_at: s.finalized_at,
        issued_at: s.issued_at,
        superseded_by: s.superseded_by,
        previous_id: s.previous_id,
        revisions,
        updated_at: s.updated_at,
        rev: s.rev,
    })
}

fn get(core: &AppCore, actor: &Actor, a: CallSheetIdArgs) -> AppResult<CallSheetDetail> {
    read(core, actor, |c| detail(c, &a.id))
}

// ------------------------------------------------------------------ create

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CallSheetCreateArgs {
    pub day_id: String,
}

fn insert_sheet(
    tx: &Tx<'_>,
    schedule_id: &str,
    src: &CallSheetSource,
    doc: &CallSheetDocument,
    revision: i64,
    previous_id: Option<&str>,
) -> AppResult<String> {
    let id = new_id();
    let now = now_ms();
    tx.conn().execute(
        "INSERT INTO call_sheet(id, schedule_id, shoot_day_id, revision, title, status, needs_refresh, source_snapshot_json,
             source_fingerprint, source_captured_at, document_json, previous_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'Draft', 0, ?6, ?7, ?8, ?9, ?10, ?8, ?8)",
        params![
            id,
            schedule_id,
            src.day_id,
            revision,
            title_for(src, revision),
            serde_json::to_string(src).map_err(|e| AppError::internal(e.to_string()))?,
            src.fingerprint(),
            now,
            serde_json::to_string(doc).map_err(|e| AppError::internal(e.to_string()))?,
            previous_id,
        ],
    )?;
    Ok(id)
}

/// FSD-CALL-001: "Create Call Sheet" from one shooting day.
fn create(core: &AppCore, actor: &Actor, a: CallSheetCreateArgs) -> AppResult<String> {
    let label = super::ops::read(core, actor, |c| {
        let sid = super::board::day_schedule_id(c, &a.day_id)?
            .ok_or_else(|| AppError::not_found("shooting day"))?;
        let d = load_days(c, &sid)?
            .into_iter()
            .find(|d| d.id == a.day_id)
            .ok_or_else(|| AppError::not_found("shooting day"))?;
        Ok(d.label())
    })?;
    let meta = MutationMeta::new(
        "callsheets.create",
        format!("Created a call sheet for {label}"),
        Capability::Edit,
    )
    .target("shooting_day", &a.day_id);
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        let sid = super::board::day_schedule_id(c, &a.day_id)?
            .ok_or_else(|| AppError::not_found("shooting day"))?;
        let board = Board::load(c, load_schedule(c, &sid)?)?;
        let day = board
            .day(&a.day_id)
            .ok_or_else(|| AppError::not_found("shooting day"))?;
        if day.off {
            return Err(AppError::validation(
                "off_day",
                "Off days have no call sheet. Choose a shooting day.",
            ));
        }
        if let Some(existing) = board.call_sheet_for_day(&day.id) {
            return Err(AppError::new(
                "conflict.call_sheet_exists",
                format!(
                    "{} already has a call sheet. Open it to make changes.",
                    day.label()
                ),
            )
            .with_detail(existing.id.clone()));
        }
        let src = board.call_sheet_source(day);
        let doc = prefill(&src);
        let prior: i64 = c.query_row(
            "SELECT COALESCE(MAX(revision), 0) FROM call_sheet WHERE shoot_day_id = ?1",
            [&day.id],
            |r| r.get(0),
        )?;
        insert_sheet(tx, &sid, &src, &doc, prior + 1, None)
    })
}

// ------------------------------------------------------------------ edit

const MAX_DOC_BYTES: usize = 512 * 1024;

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CallSheetUpdateArgs {
    pub id: String,
    pub document: CallSheetDocument,
    #[serde(default)]
    #[ts(type = "number | null")]
    pub expected_rev: Option<i64>,
}

/// FSD-CALL-005/006/008: edit the document only — the schedule is never touched.
fn update(core: &AppCore, actor: &Actor, a: CallSheetUpdateArgs) -> AppResult<i64> {
    let json = serde_json::to_string(&a.document).map_err(|e| AppError::internal(e.to_string()))?;
    if json.len() > MAX_DOC_BYTES {
        return Err(AppError::invalid_input(
            "This call sheet is too large. Shorten the notes or move text into attachments.",
        ));
    }
    let meta = MutationMeta::new("callsheets.update", "Edited a call sheet", Capability::Edit)
        .target("call_sheet", &a.id)
        .coalesce(format!("callsheet:{}", a.id));
    write(core, actor, meta, |tx| {
        let s = load_sheet(tx.conn(), &a.id)?;
        ensure_editable(&s)?;
        update_fields(
            tx.conn(),
            "call_sheet",
            &a.id,
            &[("document_json", text(json.clone()))],
            &["document_json"],
            a.expected_rev,
            "call sheet",
        )
    })
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CallSheetReadyArgs {
    pub id: String,
    pub ready: bool,
}

fn set_ready(core: &AppCore, actor: &Actor, a: CallSheetReadyArgs) -> AppResult<()> {
    let summary = if a.ready {
        "Marked a call sheet Ready"
    } else {
        "Returned a call sheet to Draft"
    };
    let meta = MutationMeta::new("callsheets.set_ready", summary, Capability::Edit)
        .target("call_sheet", &a.id);
    write(core, actor, meta, |tx| {
        let s = load_sheet(tx.conn(), &a.id)?;
        ensure_editable(&s)?;
        let status = if a.ready { "Ready" } else { "Draft" };
        update_fields(
            tx.conn(),
            "call_sheet",
            &a.id,
            &[("status", text(status))],
            &["status"],
            None,
            "call sheet",
        )?;
        Ok(())
    })
}

// ------------------------------------------------------------------ refresh

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CallSheetRefreshRow {
    pub area: String,
    pub now: String,
    pub after: String,
    pub changed: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CallSheetRefreshPreview {
    pub rows: Vec<CallSheetRefreshRow>,
    pub has_changes: bool,
    /// True for Final/Issued sheets: applying creates a new revision.
    pub creates_revision: bool,
    pub day_deleted: bool,
}

fn summarize(doc: &CallSheetDocument) -> Vec<(String, String)> {
    let join = |v: Vec<String>| {
        if v.is_empty() {
            "—".to_string()
        } else {
            v.join("\n")
        }
    };
    vec![
        (
            "Date".into(),
            doc.date
                .as_deref()
                .map(date_short)
                .unwrap_or_else(|| "No date".into()),
        ),
        ("Day".into(), doc.day_label.clone()),
        (
            "Scenes".into(),
            join(
                doc.scenes
                    .iter()
                    .map(|s| format!("{} {}", s.number, s.heading))
                    .collect(),
            ),
        ),
        (
            "Cast".into(),
            join(
                doc.cast
                    .iter()
                    .map(|c| {
                        let who = if c.actor.is_empty() {
                            c.character.clone()
                        } else {
                            format!("{} ({})", c.actor, c.character)
                        };
                        if c.call_time.is_empty() {
                            who
                        } else {
                            format!("{who} · {}", c.call_time)
                        }
                    })
                    .collect(),
            ),
        ),
        (
            "Locations".into(),
            join(
                doc.locations
                    .iter()
                    .map(|l| {
                        if l.address.is_empty() {
                            l.name.clone()
                        } else {
                            format!("{} — {}", l.name, l.address)
                        }
                    })
                    .collect(),
            ),
        ),
        (
            "Day notes".into(),
            if doc.day_notes.is_empty() {
                "—".into()
            } else {
                doc.day_notes.clone()
            },
        ),
        (
            "Meal / breaks".into(),
            if doc.practical.meal_break.is_empty() {
                "—".into()
            } else {
                doc.practical.meal_break.clone()
            },
        ),
        (
            "Travel".into(),
            if doc.practical.travel_notes.is_empty() {
                "—".into()
            } else {
                doc.practical.travel_notes.clone()
            },
        ),
    ]
}

/// FSD-CALL-010: preview differences; nothing is applied until "Apply Update".
fn refresh_preview(
    core: &AppCore,
    actor: &Actor,
    a: CallSheetIdArgs,
) -> AppResult<CallSheetRefreshPreview> {
    read(core, actor, |c| {
        let s = load_sheet(c, &a.id)?;
        let Some(src) = current_source(c, &s)? else {
            return Ok(CallSheetRefreshPreview {
                rows: Vec::new(),
                has_changes: false,
                creates_revision: false,
                day_deleted: true,
            });
        };
        let after = merge(&s.document, &s.source, &src);
        let rows: Vec<CallSheetRefreshRow> = summarize(&s.document)
            .into_iter()
            .zip(summarize(&after))
            .map(|((area, now), (_, aft))| CallSheetRefreshRow {
                changed: now != aft,
                area,
                now,
                after: aft,
            })
            .collect();
        Ok(CallSheetRefreshPreview {
            has_changes: rows.iter().any(|r| r.changed) || src.fingerprint() != s.fingerprint,
            rows,
            creates_revision: !is_editable(&s.status),
            day_deleted: false,
        })
    })
}

/// Apply the refresh to an editable sheet (Final/Issued sheets use a new revision).
fn refresh(core: &AppCore, actor: &Actor, a: CallSheetIdArgs) -> AppResult<()> {
    let meta = MutationMeta::new(
        "callsheets.refresh",
        "Updated a call sheet from the schedule",
        Capability::Edit,
    )
    .target("call_sheet", &a.id);
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        let s = load_sheet(c, &a.id)?;
        ensure_editable(&s)?;
        let src = current_source(c, &s)?
            .ok_or_else(|| AppError::conflict("The shooting day of this call sheet was deleted, so there is nothing to refresh from."))?;
        let doc = merge(&s.document, &s.source, &src);
        let status = if s.status == "Needs Refresh" {
            "Draft".to_string()
        } else {
            s.status.clone()
        };
        update_fields(
            c,
            "call_sheet",
            &a.id,
            &[
                (
                    "document_json",
                    text(
                        serde_json::to_string(&doc)
                            .map_err(|e| AppError::internal(e.to_string()))?,
                    ),
                ),
                (
                    "source_snapshot_json",
                    text(
                        serde_json::to_string(&src)
                            .map_err(|e| AppError::internal(e.to_string()))?,
                    ),
                ),
                ("source_fingerprint", text(src.fingerprint())),
                ("source_captured_at", int(now_ms())),
                ("needs_refresh", int(0)),
                ("status", text(status)),
                ("title", text(title_for(&src, s.revision))),
            ],
            &[
                "document_json",
                "source_snapshot_json",
                "source_fingerprint",
                "source_captured_at",
                "needs_refresh",
                "status",
                "title",
            ],
            None,
            "call sheet",
        )?;
        Ok(())
    })
}

// ------------------------------------------------------------------ finalize / issue / revise

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CallSheetFinalizeArgs {
    pub id: String,
    /// Finalize even though the schedule changed after this sheet was prepared.
    #[serde(default)]
    pub acknowledge_stale: bool,
}

/// FSD §38.8 / §148: freeze an immutable snapshot of exactly this document.
fn finalize(core: &AppCore, actor: &Actor, a: CallSheetFinalizeArgs) -> AppResult<String> {
    let meta = MutationMeta::new(
        "callsheets.finalize",
        "Finalized a call sheet",
        Capability::LockOrFinalize,
    )
    .target("call_sheet", &a.id);
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        let s = load_sheet(c, &a.id)?;
        ensure_editable(&s)?;
        if s.document.crew_call.trim().is_empty() {
            return Err(AppError::validation(
                "crew_call",
                "Enter the crew call time before finalizing.",
            ));
        }
        let changed = match current_source(c, &s)? {
            Some(src) => s.needs_refresh || src.fingerprint() != s.fingerprint,
            None => true,
        };
        if changed && !a.acknowledge_stale {
            return Err(AppError::new(
                "validation.call_sheet_stale",
                "The schedule changed after this call sheet was prepared. Refresh it first, or finalize this version anyway.",
            ));
        }
        let now = now_ms();
        let snapshot_id = new_id();
        let label = format!("{} · v{}", s.title, s.revision);
        let content = json!({ "document": s.document, "source": s.source, "finalizedAt": now, "revision": s.revision });
        c.execute(
            "INSERT INTO snapshot(id, snapshot_type, source_type, source_id, source_version, label, content_json, created_by, created_at, updated_at)
             VALUES (?1, 'Call Sheet', 'call_sheet', ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![snapshot_id, s.id, format!("v{}", s.revision), label, content.to_string(), tx.actor().user_id, now],
        )?;
        c.execute(
            "UPDATE call_sheet SET status = 'Final', finalized_at = ?1, finalized_by = ?2, snapshot_id = ?3, rev = rev + 1, updated_at = ?1 WHERE id = ?4",
            params![now, tx.actor().user_id, snapshot_id, s.id],
        )?;
        Ok(snapshot_id)
    })
}

fn issue(core: &AppCore, actor: &Actor, a: CallSheetIdArgs) -> AppResult<()> {
    let meta = MutationMeta::new(
        "callsheets.issue",
        "Issued a call sheet",
        Capability::LockOrFinalize,
    )
    .target("call_sheet", &a.id);
    write(core, actor, meta, |tx| {
        let s = load_sheet(tx.conn(), &a.id)?;
        if s.status != "Final" {
            return Err(AppError::invalid_input(
                "Finalize the call sheet before marking it issued.",
            ));
        }
        let now = now_ms();
        tx.conn().execute(
            "UPDATE call_sheet SET status = 'Issued', issued_at = ?1, rev = rev + 1, updated_at = ?1 WHERE id = ?2",
            params![now, s.id],
        )?;
        Ok(())
    })
}

/// Refresh + reissue: a new revision document from the current day; the issued
/// one stays unchanged and is marked Superseded (FSD §148, §126).
fn new_revision(core: &AppCore, actor: &Actor, a: CallSheetIdArgs) -> AppResult<String> {
    let meta = MutationMeta::new(
        "callsheets.new_revision",
        "Started a new call sheet revision",
        Capability::Edit,
    )
    .target("call_sheet", &a.id);
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        let s = load_sheet(c, &a.id)?;
        if is_editable(&s.status) {
            return Err(AppError::invalid_input(
                "This call sheet can still be edited. Use Refresh from schedule instead.",
            ));
        }
        if s.status == "Superseded" {
            return Err(AppError::conflict(
                "A newer revision of this call sheet already exists.",
            ));
        }
        let src = current_source(c, &s)?
            .ok_or_else(|| AppError::conflict("The shooting day of this call sheet was deleted, so a new revision can't be prepared."))?;
        let doc = merge(&s.document, &s.source, &src);
        let prior: i64 = c.query_row(
            "SELECT COALESCE(MAX(revision), 0) FROM call_sheet WHERE shoot_day_id = ?1",
            [&s.day_id],
            |r| r.get(0),
        )?;
        let id = insert_sheet(tx, &s.schedule_id, &src, &doc, prior + 1, Some(&s.id))?;
        c.execute(
            "UPDATE call_sheet SET status = 'Superseded', superseded_by = ?1, rev = rev + 1, updated_at = ?2 WHERE id = ?3",
            params![id, now_ms(), s.id],
        )?;
        Ok(id)
    })
}

// ------------------------------------------------------------------ attachments

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CallSheetAttachArgs {
    pub id: String,
    /// "attachments" | "referenceImages".
    pub section: String,
    pub path: String,
}

fn attach(core: &AppCore, actor: &Actor, a: CallSheetAttachArgs) -> AppResult<CallSheetAttachment> {
    if a.section != "attachments" && a.section != "referenceImages" {
        return Err(AppError::invalid_input(
            "Choose Attachments or Reference images.",
        ));
    }
    let path = PathBuf::from(&a.path);
    if !path.is_file() {
        return Err(AppError::invalid_input("That file could not be found."));
    }
    let meta = MutationMeta::new(
        "callsheets.attach",
        "Attached a file to a call sheet",
        Capability::Edit,
    )
    .target("call_sheet", &a.id);
    write(core, actor, meta, |tx| {
        let s = load_sheet(tx.conn(), &a.id)?;
        ensure_editable(&s)?;
        let asset = ingest_file(tx, &path)?;
        if a.section == "referenceImages" && !asset.media_type.starts_with("image/") {
            return Err(AppError::invalid_input(
                "Reference images must be image files (JPEG, PNG, WebP…).",
            ));
        }
        let att = CallSheetAttachment {
            asset_id: asset.id.clone(),
            name: asset.original_name.clone(),
            media_type: asset.media_type.clone(),
        };
        let mut doc = s.document.clone();
        let list = if a.section == "attachments" {
            &mut doc.optional.attachments
        } else {
            &mut doc.optional.reference_images
        };
        list.get_or_insert_with(Vec::new).push(att.clone());
        update_fields(
            tx.conn(),
            "call_sheet",
            &a.id,
            &[(
                "document_json",
                text(serde_json::to_string(&doc).map_err(|e| AppError::internal(e.to_string()))?),
            )],
            &["document_json"],
            None,
            "call sheet",
        )?;
        Ok(att)
    })
}

// ------------------------------------------------------------------ delete

fn delete(core: &AppCore, actor: &Actor, a: CallSheetIdArgs) -> AppResult<()> {
    let meta = MutationMeta::new(
        "callsheets.delete",
        "Deleted a call sheet",
        Capability::SoftDelete,
    )
    .target("call_sheet", &a.id);
    write(core, actor, meta, |tx| {
        let s = load_sheet(tx.conn(), &a.id)?;
        soft_delete(
            tx,
            DeleteSpec {
                object_type: "call_sheet",
                table: "call_sheet",
                id: &s.id,
                title: Some(s.title.clone()),
                parent_type: Some("shooting_day"),
                parent_id: Some(s.day_id.clone()),
                position: None,
            },
        )?;
        tx.reindex("shooting_day", &s.day_id);
        Ok(())
    })
}

fn purge_call_sheet(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let c = tx.conn();
    let now = now_ms();
    c.execute("UPDATE call_sheet SET superseded_by = NULL, rev = rev + 1, updated_at = ?1 WHERE superseded_by = ?2", params![now, row.object_id])?;
    c.execute("UPDATE call_sheet SET previous_id = NULL, rev = rev + 1, updated_at = ?1 WHERE previous_id = ?2", params![now, row.object_id])?;
    // The issued snapshot (if any) stays: issued documents are historical (FSD §126).
    c.execute("DELETE FROM call_sheet WHERE id = ?1", [&row.object_id])?;
    Ok(())
}

// ------------------------------------------------------------------ search

fn index_call_sheet(c: &Connection, id: &str) -> AppResult<Option<SearchDoc>> {
    let Ok(s) = load_sheet(c, id) else {
        return Ok(None);
    };
    let d = &s.document;
    let mut body: Vec<String> = Vec::new();
    body.extend(
        d.scenes
            .iter()
            .map(|x| format!("{} {} {}", x.number, x.heading, x.description)),
    );
    body.extend(
        d.cast
            .iter()
            .map(|x| format!("{} {}", x.actor, x.character)),
    );
    body.extend(
        d.locations
            .iter()
            .map(|x| format!("{} {}", x.name, x.address)),
    );
    body.push(d.day_notes.clone());
    body.push(d.practical.production_notes.clone());
    if let Some(n) = &d.optional.special_notes {
        body.push(n.clone());
    }
    let when = d
        .date
        .as_deref()
        .map(date_short)
        .map(|x| format!(" · {x}"))
        .unwrap_or_default();
    Ok(Some(SearchDoc {
        entity_type: "call_sheet".into(),
        title: format!("{}{when}", s.title),
        body: body
            .into_iter()
            .filter(|x| !x.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n"),
        context: "Call Sheets".into(),
        nav: json!({ "workspace": "callsheets", "callSheetId": id }),
        owner_user_id: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::schedule::board::{CallSheetSourceCast, CallSheetSourceScene};

    fn src(scenes: &[&str], cast: &[(&str, &str)], date: Option<&str>) -> CallSheetSource {
        CallSheetSource {
            production_title: "Black Rain".into(),
            source_label: "Draft 6".into(),
            day_id: "d".into(),
            day_number: Some(4),
            day_label: "Shoot Day 4".into(),
            date: date.map(|d| d.to_string()),
            day_notes: None,
            scenes: scenes
                .iter()
                .map(|n| CallSheetSourceScene {
                    key: format!("L{n}"),
                    number: n.to_string(),
                    heading: format!("INT. X {n}"),
                    description: String::new(),
                    pages: "1".into(),
                })
                .collect(),
            cast: cast
                .iter()
                .map(|(k, a)| CallSheetSourceCast {
                    key: k.to_string(),
                    character: k.to_uppercase(),
                    actor: Some(a.to_string()),
                })
                .collect(),
            locations: Vec::new(),
            breaks: Vec::new(),
        }
    }

    #[test]
    fn merge_keeps_call_times_and_follows_schedule() {
        let old = src(&["12", "17"], &[("arjun", "Karthik")], Some("2027-06-14"));
        let mut doc = prefill(&old);
        doc.crew_call = "18:00".into();
        doc.cast[0].call_time = "18:15".into();
        doc.day_notes = "Blackout the windows".into();
        let new = src(
            &["12", "19"],
            &[("arjun", "Karthik"), ("meera", "Divya")],
            Some("2027-06-15"),
        );
        let merged = merge(&doc, &old, &new);
        assert_eq!(merged.crew_call, "18:00");
        assert_eq!(
            merged.cast[0].call_time, "18:15",
            "edited call times are kept where they still apply"
        );
        assert_eq!(
            merged.cast[1].call_time, "",
            "new cast gets a blank (needs input) call time"
        );
        assert_eq!(
            merged
                .scenes
                .iter()
                .map(|s| s.number.as_str())
                .collect::<Vec<_>>(),
            vec!["12", "19"]
        );
        assert_eq!(merged.date.as_deref(), Some("2027-06-15"));
        assert_eq!(
            merged.day_notes, "Blackout the windows",
            "user-edited notes are kept"
        );
        assert_eq!(changed_areas(&old, &new), vec!["Date", "Scenes", "Cast"]);
    }
}
