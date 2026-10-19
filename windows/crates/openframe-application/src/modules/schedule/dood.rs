//! Day Out of Days (Screenplay and Production Depth spec section 28; contract
//! doc 33 section 4; feature DOOD-001).
//!
//! A derived report: rows are cast members, columns are shooting days, cells
//! say who works when. It is built from the cast the confirmed breakdown
//! assigns to each scheduled scene and from the schedule's day order. The only
//! stored data is the user's manual corrections (`dood_override`, one row per
//! cast member and day). Nothing here writes the screenplay, the breakdown or
//! the schedule, and the report never moves a scene.

use std::collections::{HashMap, HashSet};

use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::board::{Board, DayRow, current_schedule, load_schedule};
use super::fmt::date_short;
use super::ops::{ensure_editable, read, write};
use super::source::ScheduleCastRef;
use crate::core::AppCore;
use crate::registry::Registry;
use crate::store::MutationMeta;
use crate::util::optional_text;

pub fn register(r: &mut Registry) {
    use crate::registry::OperationMetadata as M;
    r.module("Schedule");
    r.query("schedule.dood", dood).meta(M::compute(
        "Day Out of Days: which cast member works on which shooting day, with manual corrections.",
    ));
    // Correction notes are part of their day's search document, also after undo and redo.
    r.index_parent("dood_override", "shooting_day", "day_id");
    r.command("schedule.set_dood_override", set_override).meta(
        M::edit("Correct one Day Out of Days cell, or return it to the derived value.").hidden(
            "A manual Day Out of Days correction records what the user knows about holds, travel, fittings and rehearsals; the assistant reads the derived report instead.",
        ),
    );
}

/// Codes a cell can hold, with their meaning (legend order).
pub const CODES: [(&str, &str); 10] = [
    ("SW", "Start work"),
    ("W", "Work"),
    ("WF", "Work finish"),
    ("SWF", "Start, work and finish on the same day"),
    ("H", "Hold (paid, not working)"),
    ("T", "Travel"),
    ("R", "Rehearsal"),
    ("F", "Fitting"),
    ("D", "Drop"),
    ("-", "Not working"),
];

fn is_work(code: &str) -> bool {
    matches!(code, "SW" | "W" | "WF" | "SWF")
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct DoodCode {
    pub code: String,
    pub meaning: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct DoodDay {
    pub id: String,
    /// "Day 4" or "Off".
    pub short_label: String,
    /// "Shoot Day 4 · Mon 14 Jun 2027".
    pub title: String,
    pub date: Option<String>,
    pub date_label: Option<String>,
    pub is_off_day: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct DoodCell {
    pub day_id: String,
    /// What the report shows (the correction when there is one); "" = blank.
    pub code: String,
    /// What the schedule alone gives.
    pub derived_code: String,
    pub overridden: bool,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct DoodRow {
    /// Stable cast key (`char:<id>` or `name:<NAME>`).
    pub key: String,
    /// Cast number in the report (order of first work day, then name).
    #[ts(type = "number")]
    pub number: i64,
    pub character: String,
    pub actor: Option<String>,
    /// "Karthik Menon (Arjun)" or "Meera".
    pub label: String,
    pub cells: Vec<DoodCell>,
    #[ts(type = "number")]
    pub work_days: i64,
    #[ts(type = "number")]
    pub hold_days: i64,
    #[ts(type = "number")]
    pub travel_days: i64,
    #[ts(type = "number")]
    pub other_days: i64,
    /// First and last work day in the whole schedule.
    pub start_label: Option<String>,
    pub finish_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct DoodCastOption {
    pub key: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleDoodView {
    pub has_schedule: bool,
    pub schedule_id: Option<String>,
    /// "industry" or "simple".
    pub code_style: String,
    pub legend: Vec<DoodCode>,
    /// Columns in range.
    pub days: Vec<DoodDay>,
    /// Every day of the schedule (for the range pickers).
    pub all_days: Vec<DoodDay>,
    pub rows: Vec<DoodRow>,
    /// Every cast member the schedule knows (for the cast picker).
    pub all_cast: Vec<DoodCastOption>,
    /// Corrections whose cast member or day is no longer in the schedule (kept, not shown).
    #[ts(type = "number")]
    pub stale_corrections: i64,
}

#[derive(Debug, Clone)]
pub struct OverrideRow {
    pub cast_key: String,
    pub day_id: String,
    pub code: String,
    pub note: Option<String>,
}

pub fn load_overrides(c: &Connection, schedule_id: &str) -> AppResult<Vec<OverrideRow>> {
    let mut st = c.prepare(
        "SELECT cast_key, day_id, code, note FROM dood_override WHERE schedule_id = ?1 ORDER BY cast_key, day_id",
    )?;
    let rows = st.query_map([schedule_id], |r| {
        Ok(OverrideRow {
            cast_key: r.get(0)?,
            day_id: r.get(1)?,
            code: r.get(2)?,
            note: r.get(3)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn day_entry(d: &DayRow) -> DoodDay {
    DoodDay {
        id: d.id.clone(),
        short_label: match d.number {
            Some(n) => format!("Day {n}"),
            None => "Off".into(),
        },
        title: d.title(),
        date: d.date.clone(),
        date_label: d.date.as_deref().map(date_short),
        is_off_day: d.off,
    }
}

fn cast_label(c: &ScheduleCastRef) -> String {
    match &c.actor {
        Some(a) => format!("{a} ({})", c.character),
        None => c.character.clone(),
    }
}

/// Derived codes for one cast member across every day of the schedule.
pub fn derive_codes(days: &[DayRow], works: &[bool], style: &str) -> Vec<String> {
    let first = works.iter().position(|w| *w);
    let last = works.iter().rposition(|w| *w);
    days.iter()
        .enumerate()
        .map(|(i, d)| {
            if d.off {
                return String::new();
            }
            let w = works[i];
            if style == "simple" {
                return if w { "W" } else { "-" }.to_string();
            }
            match (first, last) {
                (Some(f), Some(l)) if w && f == l => "SWF".into(),
                (Some(f), _) if w && i == f => "SW".into(),
                (_, Some(l)) if w && i == l => "WF".into(),
                _ if w => "W".into(),
                (Some(f), Some(l)) if f < i && i < l => "H".into(),
                _ => String::new(),
            }
        })
        .collect()
}

/// Build the report. `cast_keys` (None = everyone) and the day range only pick
/// what is shown; codes and cast numbers always come from the whole schedule.
pub fn build(
    board: &Board,
    overrides: &[OverrideRow],
    cast_keys: Option<&[String]>,
    from_day: Option<&str>,
    to_day: Option<&str>,
) -> AppResult<ScheduleDoodView> {
    let days = &board.days;
    let idx = |id: Option<&str>, fallback: usize| -> AppResult<usize> {
        match id {
            None => Ok(fallback),
            Some(id) => days
                .iter()
                .position(|d| d.id == id)
                .ok_or_else(|| AppError::not_found("shooting day")),
        }
    };
    let (mut lo, mut hi) = (
        idx(from_day, 0)?,
        idx(to_day, days.len().saturating_sub(1))?,
    );
    if lo > hi {
        std::mem::swap(&mut lo, &mut hi);
    }

    // Every cast member of the schedule, with the days they work.
    let mut cast: Vec<(ScheduleCastRef, Vec<bool>)> = Vec::new();
    let mut seen: HashMap<String, usize> = HashMap::new();
    for (i, d) in days.iter().enumerate() {
        if d.off {
            continue;
        }
        for s in board.day_strips(&d.id) {
            for c in board.cast(s) {
                let at = *seen.entry(c.key.clone()).or_insert_with(|| {
                    cast.push((c.clone(), vec![false; days.len()]));
                    cast.len() - 1
                });
                cast[at].1[i] = true;
            }
        }
    }
    for s in board.unscheduled() {
        for c in board.cast(s) {
            if !seen.contains_key(&c.key) {
                seen.insert(c.key.clone(), cast.len());
                cast.push((c.clone(), vec![false; days.len()]));
            }
        }
    }
    cast.sort_by(|a, b| {
        let fa = a.1.iter().position(|w| *w).unwrap_or(usize::MAX);
        let fb = b.1.iter().position(|w| *w).unwrap_or(usize::MAX);
        (fa, a.0.character.to_lowercase(), &a.0.key).cmp(&(
            fb,
            b.0.character.to_lowercase(),
            &b.0.key,
        ))
    });

    let corrections: HashMap<(&str, &str), &OverrideRow> = overrides
        .iter()
        .map(|o| ((o.cast_key.as_str(), o.day_id.as_str()), o))
        .collect();
    let keys: HashSet<&str> = cast.iter().map(|(c, _)| c.key.as_str()).collect();
    let day_ids: HashSet<&str> = days.iter().map(|d| d.id.as_str()).collect();
    let stale = overrides
        .iter()
        .filter(|o| !keys.contains(o.cast_key.as_str()) || !day_ids.contains(o.day_id.as_str()))
        .count() as i64;
    let wanted: Option<HashSet<&str>> = cast_keys.map(|ks| ks.iter().map(|k| k.as_str()).collect());

    let style = board.schedule.dood_style.as_str();
    let mut rows = Vec::new();
    for (n, (c, works)) in cast.iter().enumerate() {
        if wanted.as_ref().is_some_and(|w| !w.contains(c.key.as_str())) {
            continue;
        }
        let derived = derive_codes(days, works, style);
        let cells: Vec<DoodCell> = days
            .iter()
            .zip(derived.iter())
            .map(|(d, dc)| {
                let o = corrections.get(&(c.key.as_str(), d.id.as_str()));
                DoodCell {
                    day_id: d.id.clone(),
                    code: o.map(|o| o.code.clone()).unwrap_or_else(|| dc.clone()),
                    derived_code: dc.clone(),
                    overridden: o.is_some(),
                    note: o.and_then(|o| o.note.clone()),
                }
            })
            .collect();
        let first = cells.iter().position(|x| is_work(&x.code));
        let last = cells.iter().rposition(|x| is_work(&x.code));
        let shown = &cells[lo.min(cells.len())..(hi + 1).min(cells.len())];
        let count = |f: &dyn Fn(&str) -> bool| shown.iter().filter(|x| f(&x.code)).count() as i64;
        rows.push(DoodRow {
            key: c.key.clone(),
            number: n as i64 + 1,
            character: c.character.clone(),
            actor: c.actor.clone(),
            label: cast_label(c),
            work_days: count(&is_work),
            hold_days: count(&|x| x == "H"),
            travel_days: count(&|x| x == "T"),
            other_days: count(&|x| matches!(x, "R" | "F" | "D")),
            start_label: first.map(|i| days[i].label()),
            finish_label: last.map(|i| days[i].label()),
            cells: shown.to_vec(),
        });
    }
    let all_days: Vec<DoodDay> = days.iter().map(day_entry).collect();
    Ok(ScheduleDoodView {
        has_schedule: true,
        schedule_id: Some(board.schedule.id.clone()),
        code_style: style.to_string(),
        legend: CODES
            .iter()
            .filter(|(c, _)| {
                style != "simple" || matches!(*c, "W" | "-" | "H" | "T" | "R" | "F" | "D")
            })
            .map(|(c, m)| DoodCode {
                code: c.to_string(),
                meaning: m.to_string(),
            })
            .collect(),
        days: if days.is_empty() {
            Vec::new()
        } else {
            all_days[lo..=hi].to_vec()
        },
        all_days,
        rows,
        all_cast: cast
            .iter()
            .map(|(c, _)| DoodCastOption {
                key: c.key.clone(),
                label: cast_label(c),
            })
            .collect(),
        stale_corrections: stale,
    })
}

// ------------------------------------------------------------------ operations

#[derive(Debug, Default, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleDoodArgs {
    /// Cast keys to show; omitted = everyone.
    #[serde(default)]
    #[ts(optional)]
    pub cast_keys: Option<Vec<String>>,
    /// First and last day of the range (inclusive); omitted = whole schedule.
    #[serde(default)]
    #[ts(optional)]
    pub from_day_id: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub to_day_id: Option<String>,
}

pub(crate) fn check_args(a: &ScheduleDoodArgs) -> AppResult<()> {
    if a.cast_keys.as_ref().is_some_and(|k| k.len() > 1000) {
        return Err(AppError::invalid_input("Choose at most 1000 cast members."));
    }
    Ok(())
}

/// Load the board and corrections, then build the report (one read pass).
pub(crate) fn report(
    c: &Connection,
    a: &ScheduleDoodArgs,
) -> AppResult<Option<(Board, ScheduleDoodView)>> {
    let Some(sched) = current_schedule(c)? else {
        return Ok(None);
    };
    let board = Board::load(c, sched)?;
    let overrides = load_overrides(c, &board.schedule.id)?;
    let view = build(
        &board,
        &overrides,
        a.cast_keys.as_deref(),
        a.from_day_id.as_deref(),
        a.to_day_id.as_deref(),
    )?;
    Ok(Some((board, view)))
}

fn dood(core: &AppCore, actor: &Actor, a: ScheduleDoodArgs) -> AppResult<ScheduleDoodView> {
    check_args(&a)?;
    read(core, actor, |c| {
        Ok(match report(c, &a)? {
            Some((_, v)) => v,
            None => ScheduleDoodView {
                has_schedule: false,
                schedule_id: None,
                code_style: "industry".into(),
                legend: Vec::new(),
                days: Vec::new(),
                all_days: Vec::new(),
                rows: Vec::new(),
                all_cast: Vec::new(),
                stale_corrections: 0,
            },
        })
    })
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleDoodOverrideArgs {
    pub schedule_id: String,
    pub cast_key: String,
    pub day_id: String,
    /// One of the Day Out of Days codes; null = back to the derived value.
    pub code: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

fn set_override(core: &AppCore, actor: &Actor, a: ScheduleDoodOverrideArgs) -> AppResult<()> {
    let code = match a.code.as_deref().map(str::trim) {
        None => None,
        Some(c) if CODES.iter().any(|(k, _)| *k == c) => Some(c.to_string()),
        Some(_) => {
            return Err(AppError::invalid_input(
                "Choose one of the Day Out of Days codes: SW, W, WF, SWF, H, T, R, F, D or -.",
            ));
        }
    };
    let key = a.cast_key.trim().to_string();
    if key.is_empty() || key.chars().count() > 300 {
        return Err(AppError::invalid_input("Choose a cast member."));
    }
    let note = optional_text(a.note, "Note", 500)?;
    let (who, day_label) = read(core, actor, |c| {
        let sched = load_schedule(c, &a.schedule_id)?;
        let board = Board::load(c, sched)?;
        let day = board
            .day(&a.day_id)
            .ok_or_else(|| AppError::not_found("shooting day"))?;
        let who = board
            .strips
            .iter()
            .flat_map(|s| board.cast(s))
            .find(|c| c.key == key)
            .map(|c| cast_label(&c))
            .ok_or_else(|| AppError::not_found("cast member"))?;
        Ok((who, day.label()))
    })?;
    let summary = match &code {
        Some(c) => format!("Set Day Out of Days for {who} on {day_label} to {c}"),
        None => format!("Cleared the Day Out of Days correction for {who} on {day_label}"),
    };
    let meta = MutationMeta::new("schedule.set_dood_override", summary, Capability::Edit)
        .target("shooting_day", &a.day_id);
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        let s = load_schedule(c, &a.schedule_id)?;
        ensure_editable(&s)?;
        let day_ok: bool = c.query_row(
            "SELECT EXISTS(SELECT 1 FROM shooting_day WHERE id = ?1 AND schedule_id = ?2 AND deleted_at IS NULL)",
            params![a.day_id, s.id],
            |r| r.get(0),
        )?;
        if !day_ok {
            return Err(AppError::not_found("shooting day"));
        }
        let existing: Option<String> = c
            .query_row(
                "SELECT id FROM dood_override WHERE schedule_id = ?1 AND cast_key = ?2 AND day_id = ?3",
                params![s.id, key, a.day_id],
                |r| r.get(0),
            )
            .optional()?;
        let now = now_ms();
        match (&code, existing) {
            (None, None) => {}
            (None, Some(id)) => {
                c.execute("DELETE FROM dood_override WHERE id = ?1", [&id])?;
            }
            (Some(code), Some(id)) => {
                c.execute(
                    "UPDATE dood_override SET code = ?1, note = ?2, updated_at = ?3, rev = rev + 1 WHERE id = ?4",
                    params![code, note, now, id],
                )?;
            }
            (Some(code), None) => {
                c.execute(
                    "INSERT INTO dood_override(id, schedule_id, cast_key, day_id, code, note, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                    params![new_id(), s.id, key, a.day_id, code, note, now],
                )?;
            }
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(id: &str, off: bool) -> DayRow {
        DayRow {
            id: id.into(),
            schedule_id: "s".into(),
            position: 0,
            date: None,
            notes: None,
            planned_minutes: None,
            off,
            rev: 1,
            number: None,
            crew_call: None,
            shooting_start: None,
            target_wrap: None,
        }
    }

    #[test]
    fn industry_codes_mark_start_finish_and_holds() {
        let days: Vec<DayRow> = ["a", "b", "c", "d", "e"]
            .iter()
            .map(|d| day(d, *d == "c"))
            .collect();
        let got = derive_codes(&days, &[false, true, false, false, true], "industry");
        assert_eq!(got, ["", "SW", "", "H", "WF"]);
        let got = derive_codes(&days, &[false, false, false, true, false], "industry");
        assert_eq!(got, ["", "", "", "SWF", ""]);
        let got = derive_codes(&days, &[true, true, false, true, true], "industry");
        assert_eq!(got, ["SW", "W", "", "W", "WF"]);
        let got = derive_codes(&days, &[false, true, false, false, true], "simple");
        assert_eq!(got, ["-", "W", "", "-", "W"]);
        let none = derive_codes(&days, &[false; 5], "industry");
        assert!(none.iter().all(|c| c.is_empty()));
    }
}
