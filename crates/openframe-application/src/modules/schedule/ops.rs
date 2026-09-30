//! Shooting schedule operations (FSD §35–37, §56–57, §104, §145–146):
//! create from a Production Source, shooting days, strips, markers, warnings,
//! grouping suggestions, script-change reconciliation and the daily view.
//!
//! The human arrangement is authoritative: nothing here moves a scene unless
//! the user asked for exactly that move (FSD §35.10, §36.3).

use std::collections::{BTreeMap, HashMap, HashSet};

use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_persistence::rows::{
    int, next_position, opt_int, opt_text, place_at, sibling_ids, text, update_fields,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use super::board::{
    Board, CallSheetBrief, DayRow, Item, MARKER_COLS, ScheduleDto, ScheduleMarkerDto, ScheduleRow,
    ScheduleShootDayDto, ScheduleShootDaySummary, ScheduleStripDto, ScheduleWarningDto, StripRow,
    current_schedule, load_acks, load_days, load_schedule, load_strip, load_strips,
    marker_from_row, open_warning_keys,
};
use super::fmt::{clean_date, clean_time, date_short};
use super::source::{
    self, SceneInfo, ScheduleCastRef, ScheduleItemRef, ScheduleLocRef, ScheduleSourceInfo,
    SourceData,
};
use crate::core::AppCore;
use crate::registry::{Registry, SearchDoc, TrashHandler};
use crate::store::{DeleteSpec, DeletedItemRow, MutationMeta, Tx, soft_delete};
use crate::util::optional_text;

pub fn register(r: &mut Registry) {
    r.query("schedule.get", get);
    r.query("schedule.suggestions", suggestions);
    r.query("schedule.daily", daily);
    r.command("schedule.create", create);
    r.command("schedule.update_settings", update_settings);
    r.command("schedule.set_status", set_status);
    r.command("schedule.delete", delete_schedule);
    r.command("schedule.create_day", create_day);
    r.command("schedule.set_day_date", set_day_date);
    r.command("schedule.set_day_notes", set_day_notes);
    r.command("schedule.set_day_target", set_day_target);
    r.command("schedule.set_off_day", set_off_day);
    r.command("schedule.move_day", move_day);
    r.command("schedule.duplicate_day", duplicate_day);
    r.command("schedule.delete_day", delete_day);
    r.command("schedule.move_strip", move_strip);
    r.command("schedule.move_strips", move_strips);
    r.command("schedule.set_strip_estimate", set_strip_estimate);
    r.command("schedule.set_strip_pages", set_strip_pages);
    r.command("schedule.add_marker", add_marker);
    r.command("schedule.update_marker", update_marker);
    r.command("schedule.move_marker", move_marker);
    r.command("schedule.delete_marker", delete_marker);
    r.command("schedule.decide_warning", decide_warning);
    r.command("schedule.reconcile", reconcile);
    r.command("schedule.acknowledge_change", acknowledge_change);
    r.command("schedule.confirm_removal", confirm_removal);
    r.indexer("shooting_day", index_day);
    r.trash_handler(TrashHandler {
        object_type: "shooting_schedule",
        table: "shooting_schedule",
        label: "Shooting Schedule",
        restore: None,
        purge: purge_schedule,
    });
    r.trash_handler(TrashHandler {
        object_type: "shooting_day",
        table: "shooting_day",
        label: "Shooting Day",
        restore: Some(restore_day),
        purge: purge_day,
    });
    r.trash_handler(TrashHandler {
        object_type: "schedule_marker",
        table: "schedule_marker",
        label: "Schedule Marker",
        restore: Some(restore_marker),
        purge: purge_marker,
    });
}

pub(crate) const NO_SOURCE_MESSAGE: &str = "Choose a Production Source in Breakdown first.";

pub(crate) fn read<R>(
    core: &AppCore,
    actor: &Actor,
    f: impl FnOnce(&Connection) -> AppResult<R>,
) -> AppResult<R> {
    actor.require(Capability::View, "view the shooting schedule")?;
    core.project()?.store.read(f)
}

pub(crate) fn write<R>(
    core: &AppCore,
    actor: &Actor,
    meta: MutationMeta,
    f: impl FnOnce(&Tx<'_>) -> AppResult<R>,
) -> AppResult<R> {
    core.project()?.store.mutate(actor, meta, f)
}

/// Pre-read for labels/validation before the mutation (reader connection).
fn peek<R>(
    core: &AppCore,
    actor: &Actor,
    f: impl FnOnce(&Connection) -> AppResult<R>,
) -> AppResult<R> {
    actor.require(Capability::View, "view the shooting schedule")?;
    core.project()?.store.read(f)
}

fn ensure_editable(s: &ScheduleRow) -> AppResult<()> {
    if s.status == "Finalized" {
        return Err(AppError::new(
            "conflict.schedule_finalized",
            "This schedule is finalized. Reopen it to make changes.",
        ));
    }
    Ok(())
}

fn day_row(c: &Connection, day_id: &str) -> AppResult<(ScheduleRow, DayRow)> {
    let sid: Option<String> = c
        .query_row(
            "SELECT schedule_id FROM shooting_day WHERE id = ?1 AND deleted_at IS NULL",
            [day_id],
            |r| r.get(0),
        )
        .optional()?;
    let sid = sid.ok_or_else(|| AppError::not_found("shooting day"))?;
    let sched = load_schedule(c, &sid)?;
    let day = load_days(c, &sid)?
        .into_iter()
        .find(|d| d.id == day_id)
        .ok_or_else(|| AppError::not_found("shooting day"))?;
    Ok((sched, day))
}

fn scene_label_for(c: &Connection, s: &StripRow) -> AppResult<String> {
    let row: Option<(String, i64)> = c
        .query_row(
            "SELECT s.heading, (SELECT count(*) FROM screenplay_scene o WHERE o.draft_id = s.draft_id AND o.deleted_at IS NULL
                 AND (o.position < s.position OR (o.position = s.position AND o.id <= s.id)))
             FROM screenplay_scene s WHERE s.id = ?1 AND s.deleted_at IS NULL",
            [&s.scene_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    Ok(match row {
        Some((_, n)) => format!("Scene {n}"),
        None if !s.source_heading.is_empty() => format!("Scene “{}”", s.source_heading),
        None => "Scene".to_string(),
    })
}

// ------------------------------------------------------------------ view

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleScriptChangeRow {
    pub strip_id: String,
    #[ts(type = "number | null")]
    pub number: Option<i64>,
    pub heading: String,
    pub kinds: Vec<String>,
    pub day_label: Option<String>,
}

/// Pending (not yet applied) differences between the schedule and the current
/// Production Source (FSD §56). The schedule itself is unchanged until the user
/// applies the update.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleScriptChanges {
    pub source_switched: bool,
    pub new_source: Option<ScheduleSourceInfo>,
    pub changed: Vec<ScheduleScriptChangeRow>,
    pub removed: Vec<ScheduleScriptChangeRow>,
    #[ts(type = "number")]
    pub new_scenes: i64,
    #[ts(type = "number")]
    pub scheduled_affected: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleCounts {
    #[ts(type = "number")]
    pub total: i64,
    #[ts(type = "number")]
    pub scheduled: i64,
    #[ts(type = "number")]
    pub unscheduled: i64,
    #[ts(type = "number")]
    pub days: i64,
    #[ts(type = "number")]
    pub off_days: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleView {
    /// The project's active Production Source (None → ask the user to choose one in Breakdown).
    pub active_source: Option<ScheduleSourceInfo>,
    pub schedule: Option<ScheduleDto>,
    pub days: Vec<ScheduleShootDayDto>,
    pub unscheduled: Vec<ScheduleStripDto>,
    /// Strips removed from the active schedule after a confirmed script removal.
    pub history: Vec<ScheduleStripDto>,
    pub warnings: Vec<ScheduleWarningDto>,
    #[ts(type = "number")]
    pub open_warning_count: i64,
    pub script_changes: Option<ScheduleScriptChanges>,
    pub counts: ScheduleCounts,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleGetArgs {}

fn get(core: &AppCore, actor: &Actor, _: ScheduleGetArgs) -> AppResult<ScheduleView> {
    read(core, actor, |c| {
        let active = source::active_source(c)?;
        let Some(sched) = current_schedule(c)? else {
            return Ok(ScheduleView {
                active_source: active,
                schedule: None,
                days: Vec::new(),
                unscheduled: Vec::new(),
                history: Vec::new(),
                warnings: Vec::new(),
                open_warning_count: 0,
                script_changes: None,
                counts: ScheduleCounts {
                    total: 0,
                    scheduled: 0,
                    unscheduled: 0,
                    days: 0,
                    off_days: 0,
                },
            });
        };
        let board = Board::load(c, sched)?;
        let acks = load_acks(c, &board.schedule.id)?;
        let warnings = board.warnings(&acks);
        let days: Vec<ScheduleShootDayDto> = board
            .days
            .iter()
            .map(|d| board.day_dto(d, &warnings))
            .collect();
        let unscheduled: Vec<ScheduleStripDto> = board
            .unscheduled()
            .into_iter()
            .map(|s| board.strip_dto(s))
            .collect();
        let history = load_strips(c, &board.schedule.id, true)?
            .into_iter()
            .filter(|s| s.archived)
            .map(|s| board.strip_dto(&s))
            .collect();
        let script_changes = script_changes(c, &board, active.as_ref())?;
        let total = board.strips.len() as i64;
        let unscheduled_n = unscheduled.len() as i64;
        Ok(ScheduleView {
            counts: ScheduleCounts {
                total,
                scheduled: total - unscheduled_n,
                unscheduled: unscheduled_n,
                days: board.days.iter().filter(|d| !d.off).count() as i64,
                off_days: board.days.iter().filter(|d| d.off).count() as i64,
            },
            open_warning_count: warnings.iter().filter(|w| w.decision.is_none()).count() as i64,
            active_source: active,
            schedule: Some(board.schedule_dto()),
            days,
            unscheduled,
            history,
            warnings,
            script_changes,
        })
    })
}

struct ScriptDiff<'a> {
    changed: Vec<(&'a StripRow, Vec<String>, &'a SceneInfo)>,
    unchanged: Vec<(&'a StripRow, &'a SceneInfo)>,
    removed: Vec<&'a StripRow>,
    new_scenes: Vec<&'a SceneInfo>,
}

fn diff_script<'a>(strips: &'a [StripRow], target: &'a SourceData) -> ScriptDiff<'a> {
    let mut d = ScriptDiff {
        changed: Vec::new(),
        unchanged: Vec::new(),
        removed: Vec::new(),
        new_scenes: Vec::new(),
    };
    let mut used: HashSet<&str> = HashSet::new();
    for s in strips.iter().filter(|s| !s.archived) {
        let matches: Vec<&SceneInfo> = target
            .scenes_for_lineage(&s.lineage_id)
            .into_iter()
            .filter(|sc| !sc.omitted)
            .collect();
        let Some(first) = matches.first().copied() else {
            if s.source_state != "Removed" {
                d.removed.push(s);
            }
            continue;
        };
        used.insert(first.id.as_str());
        let mut kinds = Vec::new();
        if s.source_state == "Removed" {
            kinds.push("Restored in script".to_string());
        }
        if first.heading != s.source_heading {
            kinds.push("Heading changed".to_string());
        }
        if first.text_hash != s.source_text_hash {
            kinds.push("Text changed".to_string());
        }
        if first.page_eighths != s.source_page_eighths {
            kinds.push("Estimate may change".to_string());
        }
        if matches.len() > 1 {
            kinds.push("Scene split — review".to_string());
        }
        if kinds.is_empty() {
            d.unchanged.push((s, first));
        } else {
            d.changed.push((s, kinds, first));
        }
    }
    for sc in target.scenes.iter().filter(|sc| !sc.omitted) {
        if !used.contains(sc.id.as_str())
            && !strips.iter().any(|s| !s.archived && s.scene_id == sc.id)
        {
            d.new_scenes.push(sc);
        }
    }
    d
}

fn script_changes(
    c: &Connection,
    board: &Board,
    active: Option<&ScheduleSourceInfo>,
) -> AppResult<Option<ScheduleScriptChanges>> {
    let switched = active.is_some_and(|a| a.id != board.schedule.source_id);
    let loaded;
    let target: &SourceData = if switched {
        match source::load_source(c, &active.map(|a| a.id.clone()).unwrap_or_default())? {
            Some(s) => {
                loaded = s;
                &loaded
            }
            None => return Ok(None),
        }
    } else {
        match board.source.as_ref() {
            Some(s) => s,
            None => return Ok(None),
        }
    };
    let diff = diff_script(&board.strips, target);
    let remapped = diff.unchanged.iter().any(|(s, sc)| s.scene_id != sc.id);
    if !switched
        && !remapped
        && diff.changed.is_empty()
        && diff.removed.is_empty()
        && diff.new_scenes.is_empty()
    {
        return Ok(None);
    }
    let row = |s: &StripRow, kinds: Vec<String>, sc: Option<&SceneInfo>| ScheduleScriptChangeRow {
        strip_id: s.id.clone(),
        number: sc.map(|x| x.number).or_else(|| board.scene_number(s)),
        heading: sc
            .map(|x| x.heading.clone())
            .unwrap_or_else(|| s.source_heading.clone()),
        kinds,
        day_label: board
            .strip_day(s)
            .and_then(|d| board.day(d))
            .map(|d| d.title()),
    };
    let changed: Vec<ScheduleScriptChangeRow> = diff
        .changed
        .iter()
        .map(|(s, k, sc)| row(s, k.clone(), Some(sc)))
        .collect();
    let removed: Vec<ScheduleScriptChangeRow> = diff
        .removed
        .iter()
        .map(|s| row(s, vec!["Removed from script".to_string()], None))
        .collect();
    let scheduled_affected = changed
        .iter()
        .chain(removed.iter())
        .filter(|r| r.day_label.is_some())
        .count() as i64;
    Ok(Some(ScheduleScriptChanges {
        source_switched: switched,
        new_source: if switched { active.cloned() } else { None },
        changed,
        removed,
        new_scenes: diff.new_scenes.len() as i64,
        scheduled_affected,
    }))
}

// ------------------------------------------------------------------ create / settings

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleCreateArgs {
    #[serde(default)]
    pub name: Option<String>,
}

fn insert_strip(
    c: &Connection,
    schedule_id: &str,
    sc: &SceneInfo,
    state: &str,
    kinds: &[&str],
) -> AppResult<String> {
    let id = new_id();
    let now = now_ms();
    c.execute(
        "INSERT INTO schedule_strip(id, schedule_id, day_id, scene_id, scene_lineage_id, position, source_heading,
             source_text_hash, source_page_eighths, source_state, change_kinds, created_at, updated_at)
         VALUES (?1, ?2, NULL, ?3, ?4, 0, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
        params![
            id,
            schedule_id,
            sc.id,
            sc.lineage_id,
            sc.heading,
            sc.text_hash,
            sc.page_eighths,
            state,
            serde_json::to_string(kinds).unwrap_or_else(|_| "[]".into()),
            now
        ],
    )?;
    Ok(id)
}

/// FSD-SCH-001: a schedule created from the Production Source starts with every scene Unscheduled.
fn create(core: &AppCore, actor: &Actor, args: ScheduleCreateArgs) -> AppResult<ScheduleDto> {
    let name = optional_text(args.name, "Schedule name", 120)?
        .unwrap_or_else(|| "Shooting Schedule".into());
    let src = peek(core, actor, source::active_source)?
        .ok_or_else(|| AppError::new("validation.no_source", NO_SOURCE_MESSAGE))?;
    let summary = format!(
        "Created shooting schedule from {} ({} scenes unscheduled)",
        src.label, src.scene_count
    );
    write(
        core,
        actor,
        MutationMeta::new("schedule.create", summary, Capability::Edit),
        |tx| {
            let c = tx.conn();
            if current_schedule(c)?.is_some() {
                return Err(AppError::conflict(
                    "This project already has a shooting schedule.",
                ));
            }
            let data = source::load_source(c, &src.id)?
                .ok_or_else(|| AppError::new("validation.no_source", NO_SOURCE_MESSAGE))?;
            let id = new_id();
            let now = now_ms();
            c.execute(
            "INSERT INTO shooting_schedule(id, source_id, name, status, created_at, updated_at) VALUES (?1, ?2, ?3, 'Draft', ?4, ?4)",
            params![id, src.id, name, now],
        )?;
            for sc in data.scenes.iter().filter(|s| !s.omitted) {
                insert_strip(c, &id, sc, "Current", &[])?;
            }
            let board = Board::load(c, load_schedule(c, &id)?)?;
            Ok(board.schedule_dto())
        },
    )
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleSettingsArgs {
    pub schedule_id: String,
    pub name: String,
    pub strict_validation: bool,
    #[ts(type = "number")]
    pub day_duration_minutes: i64,
    #[serde(default)]
    #[ts(type = "number | null")]
    pub expected_rev: Option<i64>,
}

fn update_settings(core: &AppCore, actor: &Actor, a: ScheduleSettingsArgs) -> AppResult<()> {
    let name = crate::util::required_text(&a.name, "Schedule name", 120)?;
    if !(15..=24 * 60).contains(&a.day_duration_minutes) {
        return Err(AppError::invalid_input(
            "Enter a day length between 15 minutes and 24 hours.",
        ));
    }
    let meta = MutationMeta::new(
        "schedule.update_settings",
        "Changed schedule settings",
        Capability::Edit,
    )
    .target("shooting_schedule", &a.schedule_id);
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        load_schedule(c, &a.schedule_id)?;
        update_fields(
            c,
            "shooting_schedule",
            &a.schedule_id,
            &[
                ("name", text(name)),
                ("strict_validation", int(i64::from(a.strict_validation))),
                ("day_duration_minutes", int(a.day_duration_minutes)),
            ],
            &["name", "strict_validation", "day_duration_minutes"],
            a.expected_rev,
            "shooting schedule",
        )?;
        Ok(())
    })
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleStatusArgs {
    pub schedule_id: String,
    /// Draft / Active / Finalized.
    pub status: String,
}

fn set_status(core: &AppCore, actor: &Actor, a: ScheduleStatusArgs) -> AppResult<()> {
    if !matches!(a.status.as_str(), "Draft" | "Active" | "Finalized") {
        return Err(AppError::invalid_input(
            "Choose Draft, Active or Finalized.",
        ));
    }
    let current = peek(core, actor, |c| load_schedule(c, &a.schedule_id))?;
    // Finalizing, or reopening a finalized schedule, is a lock decision (Domain §1191).
    let cap = if a.status == "Finalized" || current.status == "Finalized" {
        Capability::LockOrFinalize
    } else {
        Capability::Edit
    };
    let meta = MutationMeta::new(
        "schedule.set_status",
        format!("Marked the shooting schedule {}", a.status),
        cap,
    )
    .target("shooting_schedule", &a.schedule_id);
    write(core, actor, meta, |tx| {
        update_fields(
            tx.conn(),
            "shooting_schedule",
            &a.schedule_id,
            &[("status", text(a.status.clone()))],
            &["status"],
            None,
            "shooting schedule",
        )?;
        Ok(())
    })
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleIdArgs {
    pub schedule_id: String,
}

fn delete_schedule(core: &AppCore, actor: &Actor, a: ScheduleIdArgs) -> AppResult<()> {
    let meta = MutationMeta::new(
        "schedule.delete",
        "Deleted the shooting schedule",
        Capability::SoftDelete,
    )
    .target("shooting_schedule", &a.schedule_id);
    write(core, actor, meta, |tx| {
        let s = load_schedule(tx.conn(), &a.schedule_id)?;
        soft_delete(
            tx,
            DeleteSpec {
                object_type: "shooting_schedule",
                table: "shooting_schedule",
                id: &s.id,
                title: Some(s.name.clone()),
                parent_type: None,
                parent_id: None,
                position: None,
            },
        )?;
        for d in load_days(tx.conn(), &s.id)? {
            tx.reindex("shooting_day", &d.id);
        }
        Ok(())
    })
}

fn purge_schedule(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let c = tx.conn();
    let id = &row.object_id;
    c.execute(
        "DELETE FROM schedule_marker WHERE day_id IN (SELECT id FROM shooting_day WHERE schedule_id = ?1)",
        [id],
    )?;
    c.execute(
        "DELETE FROM schedule_warning_ack WHERE schedule_id = ?1",
        [id],
    )?;
    c.execute("DELETE FROM schedule_strip WHERE schedule_id = ?1", [id])?;
    let day_ids: Vec<String> = {
        let mut st = c.prepare("SELECT id FROM shooting_day WHERE schedule_id = ?1")?;
        st.query_map([id], |r| r.get(0))?
            .collect::<Result<_, _>>()?
    };
    for d in &day_ids {
        c.execute(
            "DELETE FROM deleted_item WHERE table_name = 'shooting_day' AND object_id = ?1",
            [d],
        )?;
    }
    c.execute("DELETE FROM shooting_day WHERE schedule_id = ?1", [id])?;
    c.execute("DELETE FROM shooting_schedule WHERE id = ?1", [id])?;
    Ok(())
}

// ------------------------------------------------------------------ days

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleShootDayCreateArgs {
    pub schedule_id: String,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    /// Create an off day (no scenes, notes allowed).
    #[serde(default)]
    pub off_day: bool,
    /// Insert right after this day (default: at the end).
    #[serde(default)]
    pub after_day_id: Option<String>,
}

fn create_day(core: &AppCore, actor: &Actor, a: ScheduleShootDayCreateArgs) -> AppResult<String> {
    let date = clean_date(a.date)?;
    let notes = optional_text(a.notes, "Day notes", 4000)?;
    let label = peek(core, actor, |c| {
        let days = load_days(c, &a.schedule_id)?;
        if a.off_day {
            return Ok("Off Day".to_string());
        }
        let before = match &a.after_day_id {
            Some(after) => {
                let idx = days
                    .iter()
                    .position(|d| d.id == *after)
                    .ok_or_else(|| AppError::not_found("shooting day"))?;
                days[..=idx].iter().filter(|d| !d.off).count()
            }
            None => days.iter().filter(|d| !d.off).count(),
        };
        Ok(format!("Shoot Day {}", before + 1))
    })?;
    let summary = match &date {
        Some(d) => format!("Created {label} — {}", date_short(d)),
        None => format!("Created {label}"),
    };
    write(
        core,
        actor,
        MutationMeta::new("schedule.create_day", summary, Capability::Edit),
        |tx| {
            let c = tx.conn();
            let s = load_schedule(c, &a.schedule_id)?;
            ensure_editable(&s)?;
            let scope = [text(s.id.clone())];
            let id = new_id();
            let now = now_ms();
            let pos = next_position(
                c,
                "shooting_day",
                "schedule_id = ?1 AND deleted_at IS NULL",
                &scope,
            )?;
            c.execute(
            "INSERT INTO shooting_day(id, schedule_id, position, shoot_date, notes, is_off_day, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![id, s.id, pos, date, notes, i64::from(a.off_day), now],
        )?;
            if let Some(after) = &a.after_day_id {
                let sibs = sibling_ids(
                    c,
                    "shooting_day",
                    "schedule_id = ?1 AND deleted_at IS NULL",
                    &scope,
                )?;
                let idx = sibs
                    .iter()
                    .position(|x| x == after)
                    .ok_or_else(|| AppError::not_found("shooting day"))?;
                place_at(c, "shooting_day", sibs, &id, Some(idx + 1))?;
            }
            if s.status == "Draft" {
                c.execute("UPDATE shooting_schedule SET status = 'Active', rev = rev + 1, updated_at = ?1 WHERE id = ?2", params![now, s.id])?;
            }
            sync_call_sheets(tx, &s.id)?;
            Ok(id)
        },
    )
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleShootDateArgs {
    pub day_id: String,
    /// `YYYY-MM-DD`, or null to clear.
    pub date: Option<String>,
}

/// FSD §37.5: rescheduling updates the day and marks its call sheet as needing refresh.
fn set_day_date(core: &AppCore, actor: &Actor, a: ScheduleShootDateArgs) -> AppResult<()> {
    let date = clean_date(a.date)?;
    let (_, day) = peek(core, actor, |c| day_row(c, &a.day_id))?;
    let summary = match &date {
        Some(d) => format!("Rescheduled {} to {}", day.label(), date_short(d)),
        None => format!("Cleared the date of {}", day.label()),
    };
    let meta = MutationMeta::new("schedule.set_day_date", summary, Capability::Edit)
        .target("shooting_day", &a.day_id);
    write(core, actor, meta, |tx| {
        let (s, _) = day_row(tx.conn(), &a.day_id)?;
        ensure_editable(&s)?;
        update_fields(
            tx.conn(),
            "shooting_day",
            &a.day_id,
            &[("shoot_date", opt_text(date.clone()))],
            &["shoot_date"],
            None,
            "shooting day",
        )?;
        sync_call_sheets(tx, &s.id)?;
        Ok(())
    })
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleShootDayNotesArgs {
    pub day_id: String,
    pub notes: Option<String>,
}

fn set_day_notes(core: &AppCore, actor: &Actor, a: ScheduleShootDayNotesArgs) -> AppResult<()> {
    let notes = optional_text(a.notes, "Day notes", 4000)?;
    let (_, day) = peek(core, actor, |c| day_row(c, &a.day_id))?;
    let meta = MutationMeta::new(
        "schedule.set_day_notes",
        format!("Edited notes for {}", day.label()),
        Capability::Edit,
    )
    .target("shooting_day", &a.day_id)
    .coalesce(format!("day-notes:{}", a.day_id));
    write(core, actor, meta, |tx| {
        let (s, _) = day_row(tx.conn(), &a.day_id)?;
        ensure_editable(&s)?;
        update_fields(
            tx.conn(),
            "shooting_day",
            &a.day_id,
            &[("notes", opt_text(notes.clone()))],
            &["notes"],
            None,
            "shooting day",
        )?;
        sync_call_sheets(tx, &s.id)?;
        Ok(())
    })
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleShootDayTargetArgs {
    pub day_id: String,
    /// Target duration in minutes; null = use the schedule default.
    #[ts(type = "number | null")]
    pub minutes: Option<i64>,
}

fn set_day_target(core: &AppCore, actor: &Actor, a: ScheduleShootDayTargetArgs) -> AppResult<()> {
    if let Some(m) = a.minutes
        && !(15..=24 * 60).contains(&m)
    {
        return Err(AppError::invalid_input(
            "Enter a day length between 15 minutes and 24 hours.",
        ));
    }
    let (_, day) = peek(core, actor, |c| day_row(c, &a.day_id))?;
    let meta = MutationMeta::new(
        "schedule.set_day_target",
        format!("Changed the target length of {}", day.label()),
        Capability::Edit,
    )
    .target("shooting_day", &a.day_id);
    write(core, actor, meta, |tx| {
        let (s, _) = day_row(tx.conn(), &a.day_id)?;
        ensure_editable(&s)?;
        update_fields(
            tx.conn(),
            "shooting_day",
            &a.day_id,
            &[("planned_minutes", opt_int(a.minutes))],
            &["planned_minutes"],
            None,
            "shooting day",
        )?;
        Ok(())
    })
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleShootOffDayArgs {
    pub day_id: String,
    pub off_day: bool,
}

fn set_off_day(core: &AppCore, actor: &Actor, a: ScheduleShootOffDayArgs) -> AppResult<()> {
    let (_, day) = peek(core, actor, |c| day_row(c, &a.day_id))?;
    let summary = if a.off_day {
        format!("Set {} as an off day", day.title())
    } else {
        "Changed an off day into a shooting day".to_string()
    };
    let meta = MutationMeta::new("schedule.set_off_day", summary, Capability::Edit)
        .target("shooting_day", &a.day_id);
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        let (s, _) = day_row(c, &a.day_id)?;
        ensure_editable(&s)?;
        if a.off_day {
            let n: i64 = c.query_row(
                "SELECT count(*) FROM schedule_strip WHERE day_id = ?1 AND archived = 0 AND deleted_at IS NULL",
                [&a.day_id],
                |r| r.get(0),
            )?;
            if n > 0 {
                return Err(AppError::validation(
                    "off_day_has_scenes",
                    "An off day has no scenes. Move this day's scenes to another day or back to Unscheduled first.",
                ));
            }
        }
        update_fields(
            c,
            "shooting_day",
            &a.day_id,
            &[("is_off_day", int(i64::from(a.off_day)))],
            &["is_off_day"],
            None,
            "shooting day",
        )?;
        // Day numbers of later days change: reindex them and refresh call-sheet state.
        for d in load_days(c, &s.id)? {
            tx.reindex("shooting_day", &d.id);
        }
        sync_call_sheets(tx, &s.id)?;
        Ok(())
    })
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleShootDayMoveArgs {
    pub day_id: String,
    #[ts(type = "number")]
    pub index: i64,
}

fn move_day(core: &AppCore, actor: &Actor, a: ScheduleShootDayMoveArgs) -> AppResult<()> {
    let (_, day) = peek(core, actor, |c| day_row(c, &a.day_id))?;
    let meta = MutationMeta::new(
        "schedule.move_day",
        format!("Moved {} in the schedule", day.title()),
        Capability::Edit,
    )
    .target("shooting_day", &a.day_id);
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        let (s, _) = day_row(c, &a.day_id)?;
        ensure_editable(&s)?;
        let scope = [text(s.id.clone())];
        let sibs = sibling_ids(
            c,
            "shooting_day",
            "schedule_id = ?1 AND deleted_at IS NULL",
            &scope,
        )?;
        place_at(
            c,
            "shooting_day",
            sibs,
            &a.day_id,
            Some(a.index.max(0) as usize),
        )?;
        for d in load_days(c, &s.id)? {
            tx.reindex("shooting_day", &d.id);
        }
        sync_call_sheets(tx, &s.id)?;
        Ok(())
    })
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleShootDayIdArgs {
    pub day_id: String,
}

/// FSD §37.4 / Domain §1281: duplicates the day shell and its markers only —
/// scenes keep their identity and are never duplicated.
fn duplicate_day(core: &AppCore, actor: &Actor, a: ScheduleShootDayIdArgs) -> AppResult<String> {
    let (_, day) = peek(core, actor, |c| day_row(c, &a.day_id))?;
    let meta = MutationMeta::new(
        "schedule.duplicate_day",
        format!("Duplicated {} (breaks and notes only)", day.label()),
        Capability::Edit,
    )
    .target("shooting_day", &a.day_id);
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        let (s, day) = day_row(c, &a.day_id)?;
        ensure_editable(&s)?;
        let scope = [text(s.id.clone())];
        let id = new_id();
        let now = now_ms();
        let pos = next_position(
            c,
            "shooting_day",
            "schedule_id = ?1 AND deleted_at IS NULL",
            &scope,
        )?;
        c.execute(
            "INSERT INTO shooting_day(id, schedule_id, position, shoot_date, notes, planned_minutes, is_off_day, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7, ?7)",
            params![id, s.id, pos, day.notes, day.planned_minutes, i64::from(day.off), now],
        )?;
        let sibs = sibling_ids(
            c,
            "shooting_day",
            "schedule_id = ?1 AND deleted_at IS NULL",
            &scope,
        )?;
        let idx = sibs
            .iter()
            .position(|x| *x == a.day_id)
            .unwrap_or(sibs.len());
        place_at(c, "shooting_day", sibs, &id, Some(idx + 1))?;
        let markers = {
            let mut st = c.prepare(&format!(
                "SELECT {MARKER_COLS} FROM schedule_marker m WHERE m.day_id = ?1 AND m.deleted_at IS NULL ORDER BY m.position, m.id"
            ))?;
            st.query_map([&a.day_id], marker_from_row)?
                .collect::<Result<Vec<_>, _>>()?
        };
        for (i, m) in markers.iter().enumerate() {
            c.execute(
                "INSERT INTO schedule_marker(id, day_id, marker_type, label, at_time, duration_minutes, notes, position, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
                params![new_id(), id, m.marker_type, m.label, m.at_time, m.duration_minutes, m.notes, i as i64 + 1, now],
            )?;
        }
        for d in load_days(c, &s.id)? {
            tx.reindex("shooting_day", &d.id);
        }
        sync_call_sheets(tx, &s.id)?;
        Ok(id)
    })
}

/// Deleting a day returns its scenes to Unscheduled (they are never deleted).
/// Restoring the day brings back the scenes that were still waiting for it.
fn delete_day(core: &AppCore, actor: &Actor, a: ScheduleShootDayIdArgs) -> AppResult<()> {
    let (_, day) = peek(core, actor, |c| day_row(c, &a.day_id))?;
    let meta = MutationMeta::new(
        "schedule.delete_day",
        format!(
            "Deleted {} (its scenes returned to Unscheduled)",
            day.title()
        ),
        Capability::SoftDelete,
    )
    .target("shooting_day", &a.day_id);
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        let (s, day) = day_row(c, &a.day_id)?;
        ensure_editable(&s)?;
        soft_delete(
            tx,
            DeleteSpec {
                object_type: "shooting_day",
                table: "shooting_day",
                id: &day.id,
                title: Some(day.title()),
                parent_type: Some("shooting_schedule"),
                parent_id: Some(s.id.clone()),
                position: Some(day.position),
            },
        )?;
        for d in load_days(c, &s.id)? {
            tx.reindex("shooting_day", &d.id);
        }
        sync_call_sheets(tx, &s.id)?;
        Ok(())
    })
}

fn restore_day(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let c = tx.conn();
    let sched_ok: bool = c.query_row(
        "SELECT EXISTS(SELECT 1 FROM shooting_day d JOIN shooting_schedule s ON s.id = d.schedule_id
                       WHERE d.id = ?1 AND s.deleted_at IS NULL)",
        [&row.object_id],
        |r| r.get(0),
    )?;
    if !sched_ok {
        return Err(AppError::conflict(
            "Restore the shooting schedule first; this day belongs to it.",
        ));
    }
    c.execute(
        "UPDATE shooting_day SET deleted_at = NULL, updated_at = ?1, rev = rev + 1 WHERE id = ?2",
        params![now_ms(), row.object_id],
    )?;
    if let Some(sid) = super::board::day_schedule_id(c, &row.object_id)? {
        for d in load_days(c, &sid)? {
            tx.reindex("shooting_day", &d.id);
        }
        sync_call_sheets(tx, &sid)?;
    }
    Ok(())
}

fn purge_day(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let c = tx.conn();
    let now = now_ms();
    // Scenes still waiting for this day stay in Unscheduled.
    c.execute(
        "UPDATE schedule_strip SET day_id = NULL, rev = rev + 1, updated_at = ?1 WHERE day_id = ?2",
        params![now, row.object_id],
    )?;
    c.execute("DELETE FROM deleted_item WHERE table_name = 'schedule_marker' AND object_id IN (SELECT id FROM schedule_marker WHERE day_id = ?1)", [&row.object_id])?;
    c.execute(
        "DELETE FROM schedule_marker WHERE day_id = ?1",
        [&row.object_id],
    )?;
    c.execute("DELETE FROM shooting_day WHERE id = ?1", [&row.object_id])?;
    Ok(())
}

// ------------------------------------------------------------------ positions

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Strip,
    Marker,
}

fn day_item_ids(c: &Connection, day_id: &str) -> AppResult<Vec<(Kind, String)>> {
    let mut items: Vec<(i64, u8, String)> = Vec::new();
    {
        let mut st = c.prepare("SELECT position, id FROM schedule_strip WHERE day_id = ?1 AND archived = 0 AND deleted_at IS NULL")?;
        for r in st.query_map([day_id], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })? {
            let (p, id) = r?;
            items.push((p, 0, id));
        }
        let mut st = c.prepare(
            "SELECT position, id FROM schedule_marker WHERE day_id = ?1 AND deleted_at IS NULL",
        )?;
        for r in st.query_map([day_id], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })? {
            let (p, id) = r?;
            items.push((p, 1, id));
        }
    }
    items.sort();
    Ok(items
        .into_iter()
        .map(|(_, k, id)| (if k == 0 { Kind::Strip } else { Kind::Marker }, id))
        .collect())
}

/// Rewrite positions 1..n across strips and markers of one day, touching only rows that change.
fn write_positions(c: &Connection, items: &[(Kind, String)]) -> AppResult<()> {
    let now = now_ms();
    for (i, (k, id)) in items.iter().enumerate() {
        let table = match k {
            Kind::Strip => "schedule_strip",
            Kind::Marker => "schedule_marker",
        };
        c.execute(
            &format!("UPDATE {table} SET position = ?1, rev = rev + 1, updated_at = ?2 WHERE id = ?3 AND position IS NOT ?1"),
            params![(i + 1) as i64, now, id],
        )?;
    }
    Ok(())
}

fn place_item(
    c: &Connection,
    day_id: &str,
    kind: Kind,
    id: &str,
    index: Option<i64>,
) -> AppResult<()> {
    let mut items = day_item_ids(c, day_id)?;
    items.retain(|(_, x)| x != id);
    let idx = index
        .map(|i| i.max(0) as usize)
        .unwrap_or(items.len())
        .min(items.len());
    items.insert(idx, (kind, id.to_string()));
    write_positions(c, &items)
}

// ------------------------------------------------------------------ strips

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleStripMoveArgs {
    pub strip_id: String,
    /// Target day; null = back to Unscheduled.
    pub day_id: Option<String>,
    /// Position among the day's items (scenes and breaks); default = end.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub index: Option<i64>,
    /// Accept any new warning this move creates (strict validation only).
    #[serde(default)]
    pub keep_anyway: bool,
}

/// Warning keys nobody decided on, captured before a move (only when they can matter).
fn warnings_before(
    c: &Connection,
    s: &ScheduleRow,
    keep_anyway: bool,
) -> AppResult<Option<HashSet<String>>> {
    if !(s.strict || keep_anyway) {
        return Ok(None);
    }
    let board = Board::load(c, s.clone())?;
    Ok(Some(open_warning_keys(&board, &load_acks(c, &s.id)?)))
}

/// FSD §36.5–36.6: with strict validation, a move that creates a new warning is
/// refused unless the user chose "Keep Anyway"; that choice is remembered.
fn check_new_warnings(
    tx: &Tx<'_>,
    s: &ScheduleRow,
    before: Option<HashSet<String>>,
    keep_anyway: bool,
) -> AppResult<()> {
    let Some(before) = before else { return Ok(()) };
    let c = tx.conn();
    let board = Board::load(c, load_schedule(c, &s.id)?)?;
    let acks = load_acks(c, &s.id)?;
    let fresh: Vec<ScheduleWarningDto> = board
        .warnings(&acks)
        .into_iter()
        .filter(|w| w.decision.is_none() && !before.contains(&w.key) && blocking_kind(&w.kind))
        .collect();
    if fresh.is_empty() {
        return Ok(());
    }
    if keep_anyway {
        for w in &fresh {
            upsert_ack(tx, &s.id, &w.key, &w.detail_hash, "Kept")?;
        }
        return Ok(());
    }
    if s.strict {
        return Err(AppError::new(
            "validation.schedule_warning",
            format!(
                "{} Choose Keep Anyway to continue, or change the schedule.",
                fresh[0].message
            ),
        )
        .with_detail(
            fresh
                .iter()
                .map(|w| w.kind.as_str())
                .collect::<Vec<_>>()
                .join(","),
        ));
    }
    Ok(())
}

pub(crate) fn blocking_kind(kind: &str) -> bool {
    matches!(
        kind,
        "actor_conflict" | "location_conflict" | "missing_location" | "duration_overflow"
    )
}

fn upsert_ack(
    tx: &Tx<'_>,
    schedule_id: &str,
    key: &str,
    hash: &str,
    decision: &str,
) -> AppResult<()> {
    let now = now_ms();
    tx.conn().execute(
        "INSERT INTO schedule_warning_ack(id, schedule_id, warning_key, detail_hash, decision, decided_by, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
         ON CONFLICT(schedule_id, warning_key) DO UPDATE SET detail_hash = excluded.detail_hash,
             decision = excluded.decision, decided_by = excluded.decided_by, updated_at = excluded.updated_at, rev = rev + 1",
        params![new_id(), schedule_id, key, hash, decision, tx.actor().user_id, now],
    )?;
    Ok(())
}

fn day_label_of(c: &Connection, day_id: &str) -> AppResult<String> {
    Ok(day_row(c, day_id)?.1.label())
}

/// FSD-SCH-002 / §35.5–35.7: schedule, reorder, move between days, or unschedule.
/// Undoable; call sheets of every affected day are marked Needs Refresh.
fn move_strip(core: &AppCore, actor: &Actor, a: ScheduleStripMoveArgs) -> AppResult<()> {
    let summary = peek(core, actor, |c| {
        let s = load_strip(c, &a.strip_id)?;
        let label = scene_label_for(c, &s)?;
        let from = match &s.day_id {
            Some(d) => day_row(c, d).ok().map(|x| x.1),
            None => None,
        };
        Ok(match (&from, &a.day_id) {
            (_, None) => format!("Unscheduled {label}"),
            (Some(f), Some(t)) if f.id == *t => format!("Reordered {}", f.label()),
            (Some(_), Some(t)) => format!("Moved {label} to {}", day_label_of(c, t)?),
            (None, Some(t)) => format!("Scheduled {label} on {}", day_label_of(c, t)?),
        })
    })?;
    let meta = MutationMeta::new("schedule.move_strip", summary, Capability::Edit)
        .target("schedule_strip", &a.strip_id);
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        let s = load_strip(c, &a.strip_id)?;
        if s.archived {
            return Err(AppError::conflict(
                "This scene was removed from the schedule after a script change.",
            ));
        }
        let sched = load_schedule(c, &s.schedule_id)?;
        ensure_editable(&sched)?;
        let before = warnings_before(c, &sched, a.keep_anyway)?;
        apply_move(tx, &sched, &s, a.day_id.as_deref(), a.index)?;
        check_new_warnings(tx, &sched, before, a.keep_anyway)?;
        sync_call_sheets(tx, &sched.id)?;
        Ok(())
    })
}

fn apply_move(
    tx: &Tx<'_>,
    sched: &ScheduleRow,
    s: &StripRow,
    target: Option<&str>,
    index: Option<i64>,
) -> AppResult<()> {
    let c = tx.conn();
    let days = load_days(c, &sched.id)?;
    let from = s
        .day_id
        .as_deref()
        .filter(|d| days.iter().any(|x| x.id == *d));
    let now = now_ms();
    match target {
        Some(t) => {
            let day = days
                .iter()
                .find(|d| d.id == t)
                .ok_or_else(|| AppError::not_found("shooting day"))?;
            if day.off {
                return Err(AppError::validation(
                    "off_day",
                    "This is an off day. Change it back to a shooting day before adding scenes.",
                ));
            }
            if s.day_id.as_deref() != Some(t) {
                c.execute(
                    "UPDATE schedule_strip SET day_id = ?1, rev = rev + 1, updated_at = ?2 WHERE id = ?3",
                    params![t, now, s.id],
                )?;
            }
            place_item(c, t, Kind::Strip, &s.id, index)?;
            tx.reindex("shooting_day", t);
        }
        None => {
            if s.day_id.is_some() {
                c.execute(
                    "UPDATE schedule_strip SET day_id = NULL, rev = rev + 1, updated_at = ?1 WHERE id = ?2",
                    params![now, s.id],
                )?;
            }
        }
    }
    if let Some(f) = from
        && Some(f) != target
    {
        tx.reindex("shooting_day", f);
    }
    Ok(())
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleStripsMoveArgs {
    pub strip_ids: Vec<String>,
    pub day_id: String,
    #[serde(default)]
    pub keep_anyway: bool,
}

/// Apply a grouping the user explicitly confirmed (FSD §36.3): one undo step.
fn move_strips(core: &AppCore, actor: &Actor, a: ScheduleStripsMoveArgs) -> AppResult<()> {
    if a.strip_ids.is_empty() {
        return Err(AppError::required("At least one scene"));
    }
    let label = peek(core, actor, |c| day_label_of(c, &a.day_id))?;
    let n = a.strip_ids.len();
    let summary = format!(
        "Grouped {n} scene{} on {label}",
        if n == 1 { "" } else { "s" }
    );
    let meta = MutationMeta::new("schedule.move_strips", summary, Capability::Edit)
        .target("shooting_day", &a.day_id);
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        let (sched, _) = day_row(c, &a.day_id)?;
        ensure_editable(&sched)?;
        let before = warnings_before(c, &sched, a.keep_anyway)?;
        for id in &a.strip_ids {
            let s = load_strip(c, id)?;
            if s.schedule_id != sched.id || s.archived {
                return Err(AppError::not_found("scene strip"));
            }
            if s.day_id.as_deref() == Some(a.day_id.as_str()) {
                continue;
            }
            apply_move(tx, &sched, &s, Some(&a.day_id), None)?;
        }
        check_new_warnings(tx, &sched, before, a.keep_anyway)?;
        sync_call_sheets(tx, &sched.id)?;
        Ok(())
    })
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleStripEstimateArgs {
    pub strip_id: String,
    /// Entered shooting time in minutes; null = not estimated (shown as Missing).
    #[ts(type = "number | null")]
    pub minutes: Option<i64>,
}

fn set_strip_estimate(
    core: &AppCore,
    actor: &Actor,
    a: ScheduleStripEstimateArgs,
) -> AppResult<()> {
    if a.minutes.is_some_and(|m| !(0..=24 * 60).contains(&m)) {
        return Err(AppError::invalid_input(
            "Enter an estimate between 0 minutes and 24 hours.",
        ));
    }
    let label = peek(core, actor, |c| {
        scene_label_for(c, &load_strip(c, &a.strip_id)?)
    })?;
    let meta = MutationMeta::new(
        "schedule.set_strip_estimate",
        format!("Changed the time estimate of {label}"),
        Capability::Edit,
    )
    .target("schedule_strip", &a.strip_id)
    .coalesce(format!("strip-est:{}", a.strip_id));
    write(core, actor, meta, |tx| {
        let s = load_strip(tx.conn(), &a.strip_id)?;
        ensure_editable(&load_schedule(tx.conn(), &s.schedule_id)?)?;
        update_fields(
            tx.conn(),
            "schedule_strip",
            &s.id,
            &[("estimated_minutes", opt_int(a.minutes))],
            &["estimated_minutes"],
            None,
            "scene strip",
        )?;
        Ok(())
    })
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleStripPagesArgs {
    pub strip_id: String,
    /// Manual page count in eighths; null = use the screenplay estimate.
    #[ts(type = "number | null")]
    pub eighths: Option<i64>,
}

fn set_strip_pages(core: &AppCore, actor: &Actor, a: ScheduleStripPagesArgs) -> AppResult<()> {
    if a.eighths.is_some_and(|e| !(1..=8 * 200).contains(&e)) {
        return Err(AppError::invalid_input(
            "Enter a page count of at least 1/8 page.",
        ));
    }
    let label = peek(core, actor, |c| {
        scene_label_for(c, &load_strip(c, &a.strip_id)?)
    })?;
    let meta = MutationMeta::new(
        "schedule.set_strip_pages",
        format!("Changed the page count of {label}"),
        Capability::Edit,
    )
    .target("schedule_strip", &a.strip_id);
    write(core, actor, meta, |tx| {
        let s = load_strip(tx.conn(), &a.strip_id)?;
        ensure_editable(&load_schedule(tx.conn(), &s.schedule_id)?)?;
        update_fields(
            tx.conn(),
            "schedule_strip",
            &s.id,
            &[("page_eighths_override", opt_int(a.eighths))],
            &["page_eighths_override"],
            None,
            "scene strip",
        )?;
        sync_call_sheets(tx, &s.schedule_id)?;
        Ok(())
    })
}

// ------------------------------------------------------------------ markers

const MARKER_TYPES: [&str; 4] = ["Meal", "Travel", "Company Move", "Custom"];

fn default_marker_label(t: &str) -> &'static str {
    match t {
        "Meal" => "Meal Break",
        "Travel" => "Travel",
        "Company Move" => "Company Move",
        _ => "Note",
    }
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleMarkerAddArgs {
    pub day_id: String,
    /// Meal / Travel / Company Move / Custom.
    pub marker_type: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub at_time: Option<String>,
    #[serde(default)]
    #[ts(type = "number | null")]
    pub duration_minutes: Option<i64>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    #[ts(type = "number | null")]
    pub index: Option<i64>,
}

fn marker_fields(
    marker_type: &str,
    label: Option<String>,
    at_time: Option<String>,
    duration: Option<i64>,
    notes: Option<String>,
) -> AppResult<(String, Option<String>, Option<i64>, Option<String>)> {
    if !MARKER_TYPES.contains(&marker_type) {
        return Err(AppError::invalid_input(
            "Choose Meal, Travel, Company Move or Custom.",
        ));
    }
    let label = optional_text(label, "Label", 120)?;
    let label = match (marker_type, label) {
        ("Custom", None) => return Err(AppError::required("A note")),
        (_, Some(l)) => l,
        (t, None) => default_marker_label(t).to_string(),
    };
    if duration.is_some_and(|d| !(0..=24 * 60).contains(&d)) {
        return Err(AppError::invalid_input(
            "Enter a duration between 0 minutes and 24 hours.",
        ));
    }
    Ok((
        label,
        clean_time(at_time, "time")?,
        duration,
        optional_text(notes, "Notes", 2000)?,
    ))
}

fn add_marker(core: &AppCore, actor: &Actor, a: ScheduleMarkerAddArgs) -> AppResult<String> {
    let (label, at_time, duration, notes) = marker_fields(
        &a.marker_type,
        a.label,
        a.at_time,
        a.duration_minutes,
        a.notes,
    )?;
    let (_, day) = peek(core, actor, |c| day_row(c, &a.day_id))?;
    let meta = MutationMeta::new(
        "schedule.add_marker",
        format!("Added “{label}” to {}", day.label()),
        Capability::Edit,
    )
    .target("shooting_day", &a.day_id);
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        let (s, _) = day_row(c, &a.day_id)?;
        ensure_editable(&s)?;
        let id = new_id();
        let now = now_ms();
        c.execute(
            "INSERT INTO schedule_marker(id, day_id, marker_type, label, at_time, duration_minutes, notes, position, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?8)",
            params![id, a.day_id, a.marker_type, label, at_time, duration, notes, now],
        )?;
        place_item(c, &a.day_id, Kind::Marker, &id, a.index)?;
        sync_call_sheets(tx, &s.id)?;
        Ok(id)
    })
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleMarkerUpdateArgs {
    pub marker_id: String,
    pub marker_type: String,
    pub label: Option<String>,
    pub at_time: Option<String>,
    #[ts(type = "number | null")]
    pub duration_minutes: Option<i64>,
    pub notes: Option<String>,
}

fn marker_day(c: &Connection, marker_id: &str) -> AppResult<String> {
    c.query_row(
        "SELECT day_id FROM schedule_marker WHERE id = ?1 AND deleted_at IS NULL",
        [marker_id],
        |r| r.get(0),
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("schedule marker"))
}

fn update_marker(core: &AppCore, actor: &Actor, a: ScheduleMarkerUpdateArgs) -> AppResult<()> {
    let (label, at_time, duration, notes) = marker_fields(
        &a.marker_type,
        a.label,
        a.at_time,
        a.duration_minutes,
        a.notes,
    )?;
    let meta = MutationMeta::new(
        "schedule.update_marker",
        format!("Edited “{label}”"),
        Capability::Edit,
    )
    .target("schedule_marker", &a.marker_id);
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        let day_id = marker_day(c, &a.marker_id)?;
        let (s, _) = day_row(c, &day_id)?;
        ensure_editable(&s)?;
        update_fields(
            c,
            "schedule_marker",
            &a.marker_id,
            &[
                ("marker_type", text(a.marker_type.clone())),
                ("label", text(label.clone())),
                ("at_time", opt_text(at_time.clone())),
                ("duration_minutes", opt_int(duration)),
                ("notes", opt_text(notes.clone())),
            ],
            &[
                "marker_type",
                "label",
                "at_time",
                "duration_minutes",
                "notes",
            ],
            None,
            "schedule marker",
        )?;
        sync_call_sheets(tx, &s.id)?;
        Ok(())
    })
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleMarkerMoveArgs {
    pub marker_id: String,
    pub day_id: String,
    #[serde(default)]
    #[ts(type = "number | null")]
    pub index: Option<i64>,
}

fn move_marker(core: &AppCore, actor: &Actor, a: ScheduleMarkerMoveArgs) -> AppResult<()> {
    let label = peek(core, actor, |c| day_label_of(c, &a.day_id))?;
    let meta = MutationMeta::new(
        "schedule.move_marker",
        format!("Moved a break on {label}"),
        Capability::Edit,
    )
    .target("schedule_marker", &a.marker_id);
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        let from = marker_day(c, &a.marker_id)?;
        let (s, _) = day_row(c, &a.day_id)?;
        ensure_editable(&s)?;
        if from != a.day_id {
            let (fs, _) = day_row(c, &from)?;
            if fs.id != s.id {
                return Err(AppError::invalid_input(
                    "Breaks can only move within the same schedule.",
                ));
            }
            c.execute(
                "UPDATE schedule_marker SET day_id = ?1, rev = rev + 1, updated_at = ?2 WHERE id = ?3",
                params![a.day_id, now_ms(), a.marker_id],
            )?;
        }
        place_item(c, &a.day_id, Kind::Marker, &a.marker_id, a.index)?;
        sync_call_sheets(tx, &s.id)?;
        Ok(())
    })
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleMarkerIdArgs {
    pub marker_id: String,
}

fn delete_marker(core: &AppCore, actor: &Actor, a: ScheduleMarkerIdArgs) -> AppResult<()> {
    let meta = MutationMeta::new(
        "schedule.delete_marker",
        "Removed a break from the day",
        Capability::SoftDelete,
    )
    .target("schedule_marker", &a.marker_id);
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        let day_id = marker_day(c, &a.marker_id)?;
        let (s, day) = day_row(c, &day_id)?;
        ensure_editable(&s)?;
        let (label, position): (String, i64) = c.query_row(
            "SELECT label, position FROM schedule_marker WHERE id = ?1",
            [&a.marker_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        soft_delete(
            tx,
            DeleteSpec {
                object_type: "schedule_marker",
                table: "schedule_marker",
                id: &a.marker_id,
                title: Some(format!("{label} ({})", day.label())),
                parent_type: Some("shooting_day"),
                parent_id: Some(day_id.clone()),
                position: Some(position),
            },
        )?;
        sync_call_sheets(tx, &s.id)?;
        Ok(())
    })
}

fn restore_marker(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let c = tx.conn();
    let day_alive: bool = c.query_row(
        "SELECT EXISTS(SELECT 1 FROM schedule_marker m JOIN shooting_day d ON d.id = m.day_id WHERE m.id = ?1 AND d.deleted_at IS NULL)",
        [&row.object_id],
        |r| r.get(0),
    )?;
    if !day_alive {
        return Err(AppError::conflict(
            "Restore the shooting day first; this break belongs to it.",
        ));
    }
    c.execute(
        "UPDATE schedule_marker SET deleted_at = NULL, updated_at = ?1, rev = rev + 1 WHERE id = ?2",
        params![now_ms(), row.object_id],
    )?;
    if let Some(day_id) = &row.parent_id
        && let Some(sid) = super::board::day_schedule_id(c, day_id)?
    {
        sync_call_sheets(tx, &sid)?;
    }
    Ok(())
}

fn purge_marker(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    tx.conn().execute(
        "DELETE FROM schedule_marker WHERE id = ?1",
        [&row.object_id],
    )?;
    Ok(())
}

// ------------------------------------------------------------------ warnings

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleWarningDecisionArgs {
    pub schedule_id: String,
    pub key: String,
    /// "Kept" (Keep Anyway), "Dismissed", or null to show the warning again.
    pub decision: Option<String>,
}

fn decide_warning(core: &AppCore, actor: &Actor, a: ScheduleWarningDecisionArgs) -> AppResult<()> {
    if a.decision
        .as_deref()
        .is_some_and(|d| d != "Kept" && d != "Dismissed")
    {
        return Err(AppError::invalid_input("Choose Keep Anyway or Dismiss."));
    }
    let summary = match a.decision.as_deref() {
        Some("Kept") => "Kept the schedule despite a warning",
        Some(_) => "Dismissed a schedule warning",
        None => "Showed a schedule warning again",
    };
    let meta = MutationMeta::new("schedule.decide_warning", summary, Capability::Edit)
        .target("shooting_schedule", &a.schedule_id);
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        let s = load_schedule(c, &a.schedule_id)?;
        match &a.decision {
            None => {
                c.execute(
                    "DELETE FROM schedule_warning_ack WHERE schedule_id = ?1 AND warning_key = ?2",
                    params![s.id, a.key],
                )?;
            }
            Some(d) => {
                let board = Board::load(c, s.clone())?;
                let w = board
                    .warnings(&HashMap::new())
                    .into_iter()
                    .find(|w| w.key == a.key)
                    .ok_or_else(|| AppError::not_found("warning"))?;
                upsert_ack(tx, &s.id, &a.key, &w.detail_hash, d)?;
            }
        }
        Ok(())
    })
}

// ------------------------------------------------------------------ suggestions

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleGroupingSuggestion {
    pub key: String,
    /// "location" | "cast".
    pub kind: String,
    /// e.g. "Scenes 12, 18 and 21 all use the same location."
    pub title: String,
    /// e.g. "Old Railway Station · confirmed".
    pub subject: String,
    pub advice: String,
    pub strip_ids: Vec<String>,
    #[ts(type = "number[]")]
    pub scene_numbers: Vec<i64>,
    /// Strips that would move (those not already on the suggested day).
    pub strips_to_move: Vec<String>,
    pub suggested_day_id: Option<String>,
    pub suggested_day_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleGroupingSuggestions {
    pub suggestions: Vec<ScheduleGroupingSuggestion>,
    /// "No obvious grouping improvement found." when empty.
    pub message: Option<String>,
}

fn list_numbers(nums: &[i64]) -> String {
    let s: Vec<String> = nums.iter().map(|n| n.to_string()).collect();
    match s.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} and {last}", init.join(", ")),
    }
}

/// FSD §36.1–36.3: deterministic grouping advice by shared location or cast.
/// Advice only — applying it requires the user's explicit confirmation.
fn suggestions(
    core: &AppCore,
    actor: &Actor,
    _: ScheduleGetArgs,
) -> AppResult<ScheduleGroupingSuggestions> {
    read(core, actor, |c| {
        let Some(sched) = current_schedule(c)? else {
            return Ok(ScheduleGroupingSuggestions {
                suggestions: Vec::new(),
                message: Some("Create a shooting schedule first.".into()),
            });
        };
        let board = Board::load(c, sched)?;
        let mut out: Vec<ScheduleGroupingSuggestion> = Vec::new();
        let candidates: Vec<&StripRow> = board
            .strips
            .iter()
            .filter(|s| board.scene(s).is_some() && s.source_state != "Removed")
            .collect();
        let mut seen_sets: HashSet<Vec<String>> = HashSet::new();

        let make = |kind: &str,
                    key: String,
                    title: String,
                    subject: String,
                    advice: &str,
                    group: Vec<&StripRow>|
         -> Option<ScheduleGroupingSuggestion> {
            let places: HashSet<Option<&str>> = group.iter().map(|s| board.strip_day(s)).collect();
            if places.len() < 2 {
                return None;
            }
            // Suggested day: the shooting day already holding most of the group (earliest on ties).
            let mut counts: BTreeMap<usize, (usize, &DayRow)> = BTreeMap::new();
            for s in &group {
                if let Some(d) = board.strip_day(s).and_then(|d| board.day(d)) {
                    let idx = board
                        .days
                        .iter()
                        .position(|x| x.id == d.id)
                        .unwrap_or(usize::MAX);
                    counts.entry(idx).or_insert((0, d)).0 += 1;
                }
            }
            let best = counts
                .iter()
                .max_by(|a, b| a.1.0.cmp(&b.1.0).then(b.0.cmp(a.0)))
                .map(|(_, (_, d))| *d);
            let to_move: Vec<String> = group
                .iter()
                .filter(|s| best.is_none_or(|d| board.strip_day(s) != Some(d.id.as_str())))
                .map(|s| s.id.clone())
                .collect();
            Some(ScheduleGroupingSuggestion {
                key,
                kind: kind.to_string(),
                title,
                subject,
                advice: advice.to_string(),
                scene_numbers: group.iter().filter_map(|s| board.scene_number(s)).collect(),
                strip_ids: group.iter().map(|s| s.id.clone()).collect(),
                strips_to_move: to_move,
                suggested_day_id: best.map(|d| d.id.clone()),
                suggested_day_label: best.map(|d| d.title()),
            })
        };

        // Shared location.
        let mut by_loc: BTreeMap<String, (ScheduleLocRef, Vec<&StripRow>)> = BTreeMap::new();
        for s in &candidates {
            if let Some(l) = board.locations(s).into_iter().next() {
                by_loc
                    .entry(l.key.clone())
                    .or_insert_with(|| (l.clone(), Vec::new()))
                    .1
                    .push(s);
            }
        }
        for (key, (loc, mut group)) in by_loc {
            if group.len() < 2 {
                continue;
            }
            group.sort_by_key(|s| board.scene_number(s).unwrap_or(i64::MAX));
            let nums: Vec<i64> = group.iter().filter_map(|s| board.scene_number(s)).collect();
            let subject = match loc.status.as_deref() {
                Some(st) => format!("{} · {}", loc.name, st.to_lowercase()),
                None => loc.name.clone(),
            };
            let mut ids: Vec<String> = group.iter().map(|s| s.id.clone()).collect();
            ids.sort();
            if let Some(sg) = make(
                "location",
                format!("location:{key}"),
                format!("Scenes {} all use the same location.", list_numbers(&nums)),
                subject,
                "You may reduce location changes by grouping them.",
                group,
            ) {
                seen_sets.insert(ids);
                out.push(sg);
            }
        }

        // Identical cast.
        let mut by_cast: BTreeMap<Vec<String>, (Vec<ScheduleCastRef>, Vec<&StripRow>)> =
            BTreeMap::new();
        for s in &candidates {
            let cast = board.cast(s);
            if cast.is_empty() {
                continue;
            }
            let mut keys: Vec<String> = cast.iter().map(|c| c.key.clone()).collect();
            keys.sort();
            by_cast
                .entry(keys)
                .or_insert_with(|| (cast.clone(), Vec::new()))
                .1
                .push(s);
        }
        for (keys, (cast, mut group)) in by_cast {
            if group.len() < 2 {
                continue;
            }
            let mut ids: Vec<String> = group.iter().map(|s| s.id.clone()).collect();
            ids.sort();
            if seen_sets.contains(&ids) {
                continue;
            }
            group.sort_by_key(|s| board.scene_number(s).unwrap_or(i64::MAX));
            let nums: Vec<i64> = group.iter().filter_map(|s| board.scene_number(s)).collect();
            let who: Vec<String> = cast
                .iter()
                .map(|c| c.actor.clone().unwrap_or_else(|| c.character.clone()))
                .collect();
            if let Some(sg) = make(
                "cast",
                format!("cast:{}", keys.join("+")),
                format!("Scenes {} need exactly the same cast.", list_numbers(&nums)),
                who.join(" · "),
                "Grouping them may reduce the number of days these actors are called.",
                group,
            ) {
                out.push(sg);
            }
        }
        out.sort_by_key(|s| {
            (
                s.scene_numbers.first().copied().unwrap_or(i64::MAX),
                s.kind.clone(),
            )
        });
        let message = if out.is_empty() {
            Some("No obvious grouping improvement found.".to_string())
        } else {
            None
        };
        Ok(ScheduleGroupingSuggestions {
            suggestions: out,
            message,
        })
    })
}

// ------------------------------------------------------------------ reconciliation

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleReconcileResult {
    #[ts(type = "number")]
    pub changed: i64,
    #[ts(type = "number")]
    pub added: i64,
    #[ts(type = "number")]
    pub removed: i64,
}

/// FSD §56: apply the current Production Source to the schedule. Existing
/// assignments stay; changed scenes are flagged; new scenes enter Unscheduled;
/// removed scenes are flagged and stay until the user confirms removal.
fn reconcile(
    core: &AppCore,
    actor: &Actor,
    a: ScheduleIdArgs,
) -> AppResult<ScheduleReconcileResult> {
    let meta = MutationMeta::new(
        "schedule.reconcile",
        "Updated the schedule from the script changes",
        Capability::Edit,
    )
    .target("shooting_schedule", &a.schedule_id);
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        let sched = load_schedule(c, &a.schedule_id)?;
        ensure_editable(&sched)?;
        let active = source::active_source(c)?
            .ok_or_else(|| AppError::new("validation.no_source", NO_SOURCE_MESSAGE))?;
        let target = source::load_source(c, &active.id)?
            .ok_or_else(|| AppError::new("validation.no_source", NO_SOURCE_MESSAGE))?;
        let strips = load_strips(c, &sched.id, false)?;
        let diff = diff_script(&strips, &target);
        let now = now_ms();
        let set_state = |s: &StripRow,
                         sc: Option<&SceneInfo>,
                         state: &str,
                         kinds: &[String]|
         -> AppResult<()> {
            let mut all: Vec<String> = if s.source_state == "Changed" && state == "Changed" {
                s.change_kinds.clone()
            } else {
                Vec::new()
            };
            for k in kinds {
                if !all.contains(k) {
                    all.push(k.clone());
                }
            }
            let kinds_json = serde_json::to_string(&all).unwrap_or_else(|_| "[]".into());
            match sc {
                Some(sc) => c.execute(
                    "UPDATE schedule_strip SET scene_id = ?1, source_heading = ?2, source_text_hash = ?3, source_page_eighths = ?4,
                         source_state = ?5, change_kinds = ?6, rev = rev + 1, updated_at = ?7 WHERE id = ?8",
                    params![sc.id, sc.heading, sc.text_hash, sc.page_eighths, state, kinds_json, now, s.id],
                )?,
                None => c.execute(
                    "UPDATE schedule_strip SET source_state = ?1, change_kinds = ?2, rev = rev + 1, updated_at = ?3 WHERE id = ?4",
                    params![state, kinds_json, now, s.id],
                )?,
            };
            Ok(())
        };
        for (s, sc) in &diff.unchanged {
            if s.scene_id != sc.id {
                c.execute("UPDATE schedule_strip SET scene_id = ?1, rev = rev + 1, updated_at = ?2 WHERE id = ?3", params![sc.id, now, s.id])?;
            }
        }
        for (s, kinds, sc) in &diff.changed {
            set_state(s, Some(sc), "Changed", kinds)?;
        }
        for s in &diff.removed {
            set_state(s, None, "Removed", &["Removed from script".to_string()])?;
        }
        for sc in &diff.new_scenes {
            insert_strip(c, &sched.id, sc, "New", &["New scene"])?;
        }
        if sched.source_id != active.id {
            c.execute(
                "UPDATE shooting_schedule SET source_id = ?1, rev = rev + 1, updated_at = ?2 WHERE id = ?3",
                params![active.id, now, sched.id],
            )?;
        }
        for d in load_days(c, &sched.id)? {
            tx.reindex("shooting_day", &d.id);
        }
        sync_call_sheets(tx, &sched.id)?;
        Ok(ScheduleReconcileResult {
            changed: diff.changed.len() as i64,
            added: diff.new_scenes.len() as i64,
            removed: diff.removed.len() as i64,
        })
    })
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleStripIdArgs {
    pub strip_id: String,
}

fn acknowledge_change(core: &AppCore, actor: &Actor, a: ScheduleStripIdArgs) -> AppResult<()> {
    let label = peek(core, actor, |c| {
        scene_label_for(c, &load_strip(c, &a.strip_id)?)
    })?;
    let meta = MutationMeta::new(
        "schedule.acknowledge_change",
        format!("Reviewed the script change in {label}"),
        Capability::Edit,
    )
    .target("schedule_strip", &a.strip_id);
    write(core, actor, meta, |tx| {
        let s = load_strip(tx.conn(), &a.strip_id)?;
        if s.source_state == "Removed" {
            return Err(AppError::invalid_input(
                "This scene was removed from the script. Confirm the removal instead.",
            ));
        }
        update_fields(
            tx.conn(),
            "schedule_strip",
            &s.id,
            &[
                ("source_state", text("Current")),
                ("change_kinds", text("[]")),
            ],
            &["source_state", "change_kinds"],
            None,
            "scene strip",
        )?;
        Ok(())
    })
}

/// A removed scene leaves the active schedule only after this confirmation; it
/// stays in schedule history (never deleted).
fn confirm_removal(core: &AppCore, actor: &Actor, a: ScheduleStripIdArgs) -> AppResult<()> {
    let meta = MutationMeta::new(
        "schedule.confirm_removal",
        "Removed a deleted script scene from the schedule",
        Capability::Edit,
    )
    .target("schedule_strip", &a.strip_id);
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        let s = load_strip(c, &a.strip_id)?;
        if s.source_state != "Removed" {
            return Err(AppError::invalid_input(
                "Only scenes removed from the script can be taken off the schedule this way.",
            ));
        }
        ensure_editable(&load_schedule(c, &s.schedule_id)?)?;
        c.execute(
            "UPDATE schedule_strip SET archived = 1, day_id = NULL, rev = rev + 1, updated_at = ?1 WHERE id = ?2",
            params![now_ms(), s.id],
        )?;
        if let Some(d) = &s.day_id {
            tx.reindex("shooting_day", d);
        }
        sync_call_sheets(tx, &s.schedule_id)?;
        Ok(())
    })
}

// ------------------------------------------------------------------ call-sheet staleness

/// Persist "needs refresh" on every current call sheet of the schedule whose
/// source day no longer matches what it was generated from (FSD §35.7, §147).
/// Runs inside the schedule mutation, so undo restores it together with the move.
pub(crate) fn sync_call_sheets(tx: &Tx<'_>, schedule_id: &str) -> AppResult<()> {
    let c = tx.conn();
    let n: i64 = c.query_row(
        "SELECT count(*) FROM call_sheet WHERE schedule_id = ?1 AND deleted_at IS NULL AND status <> 'Superseded'",
        [schedule_id],
        |r| r.get(0),
    )?;
    if n == 0 {
        return Ok(());
    }
    let Some(sched) = c
        .query_row(
            "SELECT id FROM shooting_schedule WHERE id = ?1 AND deleted_at IS NULL",
            [schedule_id],
            |r| r.get::<_, String>(0),
        )
        .optional()?
    else {
        return Ok(());
    };
    let board = Board::load(c, load_schedule(c, &sched)?)?;
    let now = now_ms();
    for cs in &board.call_sheets {
        let changed = match board.day(&cs.day_id) {
            Some(d) => board.call_sheet_source(d).fingerprint() != cs.fingerprint,
            None => true,
        };
        if changed != cs.needs_refresh {
            c.execute(
                "UPDATE call_sheet SET needs_refresh = ?1, rev = rev + 1, updated_at = ?2 WHERE id = ?3",
                params![i64::from(changed), now, cs.id],
            )?;
        }
    }
    Ok(())
}

// ------------------------------------------------------------------ daily view

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleDailyDayOption {
    pub id: String,
    pub label: String,
    pub title: String,
    pub date: Option<String>,
    pub is_off_day: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleDailyCast {
    pub key: String,
    pub character: String,
    pub actor: Option<String>,
    /// Call time from the day's call sheet, when entered.
    pub call_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleDailyDay {
    pub id: String,
    pub label: String,
    pub title: String,
    pub date: Option<String>,
    pub date_label: Option<String>,
    pub is_today: bool,
    pub is_off_day: bool,
    pub notes: Option<String>,
    pub scenes: Vec<ScheduleStripDto>,
    pub markers: Vec<ScheduleMarkerDto>,
    pub summary: ScheduleShootDaySummary,
    pub cast: Vec<ScheduleDailyCast>,
    pub crew_call: Option<String>,
    pub locations: Vec<ScheduleLocRef>,
    pub call_sheet: Option<CallSheetBrief>,
    pub breakdown_items: Vec<ScheduleItemRef>,
    pub warnings: Vec<ScheduleWarningDto>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleDailyView {
    pub has_schedule: bool,
    pub days: Vec<ScheduleDailyDayOption>,
    pub day: Option<ScheduleDailyDay>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleDailyArgs {
    #[serde(default)]
    pub day_id: Option<String>,
}

pub(crate) fn today() -> String {
    let now = time::OffsetDateTime::now_local()
        .unwrap_or_else(|_| time::OffsetDateTime::now_utc())
        .date();
    format!(
        "{:04}-{:02}-{:02}",
        now.year(),
        u8::from(now.month()),
        now.day()
    )
}

/// FSD §57: a derived, mostly read-only view of one shooting day.
fn daily(core: &AppCore, actor: &Actor, a: ScheduleDailyArgs) -> AppResult<ScheduleDailyView> {
    read(core, actor, |c| {
        let Some(sched) = current_schedule(c)? else {
            return Ok(ScheduleDailyView {
                has_schedule: false,
                days: Vec::new(),
                day: None,
            });
        };
        let board = Board::load(c, sched)?;
        let today = today();
        let days: Vec<ScheduleDailyDayOption> = board
            .days
            .iter()
            .map(|d| ScheduleDailyDayOption {
                id: d.id.clone(),
                label: d.label(),
                title: d.title(),
                date: d.date.clone(),
                is_off_day: d.off,
            })
            .collect();
        let pick = a
            .day_id
            .as_deref()
            .and_then(|id| board.day(id))
            .or_else(|| {
                board
                    .days
                    .iter()
                    .find(|d| d.date.as_deref() == Some(today.as_str()))
            })
            .or_else(|| {
                board
                    .days
                    .iter()
                    .filter(|d| !d.off)
                    .find(|d| d.date.as_deref().is_some_and(|x| x >= today.as_str()))
            })
            .or_else(|| board.days.iter().find(|d| !d.off))
            .or_else(|| board.days.first());
        let Some(d) = pick else {
            return Ok(ScheduleDailyView {
                has_schedule: true,
                days,
                day: None,
            });
        };
        let strips = board.day_strips(&d.id);
        let acks = load_acks(c, &board.schedule.id)?;
        let warnings: Vec<ScheduleWarningDto> = board
            .warnings(&acks)
            .into_iter()
            .filter(|w| w.day_id.as_deref() == Some(d.id.as_str()))
            .collect();
        let brief = board.call_sheet_brief(d);
        let (crew_call, times) = match &brief {
            Some(b) => super::callsheet::call_times(c, &b.id)?,
            None => (None, HashMap::new()),
        };
        let summary = board.day_summary(d);
        let mut items: Vec<ScheduleItemRef> = Vec::new();
        for s in &strips {
            for it in board.items(s) {
                if !items
                    .iter()
                    .any(|x| x.category == it.category && x.name == it.name)
                {
                    items.push(it);
                }
            }
        }
        Ok(ScheduleDailyView {
            has_schedule: true,
            day: Some(ScheduleDailyDay {
                id: d.id.clone(),
                label: d.label(),
                title: d.title(),
                date: d.date.clone(),
                date_label: d.date.as_deref().map(date_short),
                is_today: d.date.as_deref() == Some(today.as_str()),
                is_off_day: d.off,
                notes: d.notes.clone(),
                scenes: strips.iter().map(|s| board.strip_dto(s)).collect(),
                markers: board
                    .day_items(&d.id)
                    .into_iter()
                    .filter_map(|i| match i {
                        Item::Marker(m) => Some(board.marker_dto(m)),
                        Item::Strip(_) => None,
                    })
                    .collect(),
                cast: summary
                    .cast
                    .iter()
                    .map(|c| ScheduleDailyCast {
                        key: c.key.clone(),
                        character: c.character.clone(),
                        actor: c.actor.clone(),
                        call_time: times.get(&c.key).cloned(),
                    })
                    .collect(),
                locations: summary.locations.clone(),
                crew_call,
                summary,
                call_sheet: brief,
                breakdown_items: items,
                warnings,
            }),
            days,
        })
    })
}

// ------------------------------------------------------------------ search

fn index_day(c: &Connection, id: &str) -> AppResult<Option<SearchDoc>> {
    let row: Option<(String, Option<i64>)> = c
        .query_row(
            "SELECT d.schedule_id, COALESCE(d.deleted_at, s.deleted_at) FROM shooting_day d
             JOIN shooting_schedule s ON s.id = d.schedule_id WHERE d.id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((sid, None)) = row else {
        return Ok(None);
    };
    let Some(day) = load_days(c, &sid)?.into_iter().find(|d| d.id == id) else {
        return Ok(None);
    };
    let mut st = c.prepare(
        "SELECT sc.heading FROM schedule_strip st JOIN screenplay_scene sc ON sc.id = st.scene_id
         WHERE st.day_id = ?1 AND st.archived = 0 AND st.deleted_at IS NULL ORDER BY st.position",
    )?;
    let headings: Vec<String> = st
        .query_map([id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let mut body = headings.join("\n");
    if let Some(n) = &day.notes {
        body.push('\n');
        body.push_str(n);
    }
    Ok(Some(SearchDoc {
        entity_type: "shooting_day".into(),
        title: day.title(),
        body,
        context: "Production".into(),
        nav: json!({ "workspace": "production", "sub": "schedule", "dayId": id }),
        owner_user_id: None,
    }))
}
