//! The "Using: ..." line of an answer (spec section 12.3): built by code from the sections that
//! were really read, never from what the person was looking at. For example
//! `Using: Screenplay (Draft 2, all scenes) + Cast & Crew`.

use rusqlite::{Connection, OptionalExtension};
use straft_domain::{AppError, AppResult};

use super::{Picker, SectionPick, leaf};
use crate::modules::ai::{index, queries};
use crate::modules::schedule::board;

/// The line for a set of picks, in the order they were picked.
pub fn using_label(c: &Connection, picks: &[SectionPick]) -> AppResult<String> {
    let mut parts = Vec::new();
    for p in picks {
        let l = leaf(&p.leaf).ok_or_else(|| unknown(&p.leaf))?;
        let mut part = l.short_label();
        let inner = instance(c, l.picker, p)?;
        if !inner.is_empty() {
            part = format!("{part} ({})", inner.join(", "));
        }
        parts.push(part);
    }
    Ok(format!("Using: {}", parts.join(" + ")))
}

pub(super) fn unknown(id: &str) -> AppError {
    AppError::invalid_input(format!(
        "\u{201C}{id}\u{201D} is not a section Straft AI can read."
    ))
}

fn gone(what: &str) -> AppError {
    AppError::ai(
        "scope_missing",
        format!("That {what} is no longer available."),
    )
}

/// What the person chose under a section, as short words.
fn instance(c: &Connection, picker: Picker, p: &SectionPick) -> AppResult<Vec<String>> {
    let mut out = Vec::new();
    match picker {
        Picker::None => {}
        Picker::Screenplay => {
            let draft = if let Some(id) = &p.draft_id {
                Some(queries::draft_by_id(c, id)?.ok_or_else(|| gone("draft"))?)
            } else if let Some(id) = &p.screenplay_id {
                Some(index::draft_for_screenplay(c, id)?.ok_or_else(|| gone("screenplay"))?)
            } else {
                queries::current_draft(c)?
            };
            if let Some(d) = draft {
                if d.several_screenplays {
                    out.push(format!("\u{201C}{}\u{201D}", d.screenplay_title));
                }
                out.push(d.name);
            }
            out.push(scene_word(c, p)?.unwrap_or_else(|| "all scenes".into()));
        }
        Picker::Scene => out.extend(scene_word(c, p)?),
        Picker::ShootingDay => out.extend(day_word(c, p)?),
        Picker::DayAndScene => {
            out.extend(day_word(c, p)?);
            out.extend(scene_word(c, p)?);
        }
        Picker::CallSheet => {
            if let Some(id) = &p.call_sheet_id {
                let title: Option<String> = c
                    .query_row(
                        "SELECT title FROM call_sheet WHERE id = ?1 AND deleted_at IS NULL",
                        [id],
                        |r| r.get(0),
                    )
                    .optional()?;
                out.push(title.ok_or_else(|| gone("call sheet"))?);
            } else {
                out.extend(day_word(c, p)?);
            }
        }
    }
    Ok(out)
}

fn scene_word(c: &Connection, p: &SectionPick) -> AppResult<Option<String>> {
    let Some(id) = &p.scene_id else {
        return Ok(None);
    };
    let (_, s) = queries::scene_by_id(c, id)?.ok_or_else(|| gone("scene"))?;
    Ok(Some(format!("Scene {}", s.number)))
}

fn day_word(c: &Connection, p: &SectionPick) -> AppResult<Option<String>> {
    let Some(id) = &p.shooting_day_id else {
        return Ok(None);
    };
    let sched = board::current_schedule(c)?.ok_or_else(|| gone("shooting day"))?;
    let days = board::load_days(c, &sched.id)?;
    let d = days
        .iter()
        .find(|d| d.id == *id)
        .ok_or_else(|| gone("shooting day"))?;
    Ok(Some(d.label()))
}
