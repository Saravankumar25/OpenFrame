//! Review rounds (FSD §23.1, §23.7; UX §3.16; Domain "Review Round").
//!
//! A review round targets one draft and lists reviewers, an optional deadline
//! and its comments (comment.review_round_id). Starting a review on a plain
//! draft marks it "Review"; completing the round never locks the script.

use openframe_domain::enums::DraftStatus;
use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_persistence::rows::{opt_int, opt_text, text, update_fields};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::load_draft;
use crate::core::AppCore;
use crate::registry::Registry;
use crate::store::MutationMeta;
use crate::util::required_text;

pub fn register(r: &mut Registry) {
    use crate::registry::OperationMetadata as M;
    use openframe_domain::Capability as Cap;
    r.query("screenplay.review_rounds", review_rounds)
        .meta(M::read("Review rounds of a screenplay."));
    r.command("screenplay.start_review", start_review)
        .meta(M::edit("Start a review round on a draft."));
    r.command("screenplay.update_review", update_review)
        .meta(M::edit(
            "Edit a review round's name, reviewers or deadline.",
        ));
    r.command("screenplay.complete_review", complete_review)
        .meta(M::command(Cap::ResolveComments, "Complete a review round."));
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayReviewRoundDto {
    pub id: String,
    pub draft_id: String,
    pub draft_name: String,
    pub name: String,
    pub reviewers: Vec<String>,
    /// ISO date `YYYY-MM-DD`.
    pub deadline: Option<String>,
    /// "Open" or "Complete".
    pub status: String,
    pub created_by_name: Option<String>,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number | null")]
    pub completed_at: Option<i64>,
    pub open_count: u32,
    pub discussion_count: u32,
    pub resolved_count: u32,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayReviewRoundsArgs {
    pub screenplay_id: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayStartReviewArgs {
    pub draft_id: String,
    pub name: String,
    #[serde(default)]
    pub reviewers: Vec<String>,
    #[serde(default)]
    #[ts(optional)]
    pub deadline: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayUpdateReviewArgs {
    pub review_round_id: String,
    #[serde(default)]
    #[ts(optional)]
    pub name: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub reviewers: Option<Vec<String>>,
    /// Empty string clears the deadline.
    #[serde(default)]
    #[ts(optional)]
    pub deadline: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayReviewRoundIdArgs {
    pub review_round_id: String,
}

fn clean_reviewers(v: Vec<String>) -> AppResult<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    for r in v {
        let r = r.trim().to_string();
        if r.is_empty() {
            continue;
        }
        if r.chars().count() > 80 {
            return Err(AppError::invalid_input("A reviewer name is too long."));
        }
        if !out.iter().any(|x| x.eq_ignore_ascii_case(&r)) {
            out.push(r);
        }
    }
    if out.len() > 50 {
        return Err(AppError::invalid_input(
            "A review can have at most 50 reviewers.",
        ));
    }
    Ok(out)
}

/// Validate an ISO calendar date (`YYYY-MM-DD`); empty → None.
pub fn clean_deadline(v: Option<String>) -> AppResult<Option<String>> {
    let Some(v) = v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    let bad = || AppError::invalid_input("Enter the deadline as a date, for example 2026-09-30.");
    let parts: Vec<&str> = v.split('-').collect();
    if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        return Err(bad());
    }
    let y: i32 = parts[0].parse().map_err(|_| bad())?;
    let m: u8 = parts[1].parse().map_err(|_| bad())?;
    let d: u8 = parts[2].parse().map_err(|_| bad())?;
    let month = time::Month::try_from(m).map_err(|_| bad())?;
    time::Date::from_calendar_date(y, month, d).map_err(|_| bad())?;
    Ok(Some(v))
}

fn load_round(c: &Connection, id: &str) -> AppResult<ScreenplayReviewRoundDto> {
    let row = c
        .query_row(
            "SELECT r.id, r.draft_id, d.name, r.name, r.reviewers_json, r.deadline, r.status, r.created_by_name, r.created_at,
                    r.completed_at, r.rev
             FROM review_round r JOIN screenplay_draft d ON d.id = r.draft_id
             WHERE r.id=?1 AND r.deleted_at IS NULL",
            [id],
            |r| {
                let reviewers: String = r.get(4)?;
                Ok(ScreenplayReviewRoundDto {
                    id: r.get(0)?,
                    draft_id: r.get(1)?,
                    draft_name: r.get(2)?,
                    name: r.get(3)?,
                    reviewers: serde_json::from_str(&reviewers).unwrap_or_default(),
                    deadline: r.get(5)?,
                    status: r.get(6)?,
                    created_by_name: r.get(7)?,
                    created_at: r.get(8)?,
                    completed_at: r.get(9)?,
                    open_count: 0,
                    discussion_count: 0,
                    resolved_count: 0,
                    rev: r.get(10)?,
                })
            },
        )
        .optional()?;
    let mut dto = row.ok_or_else(|| AppError::not_found("review round"))?;
    let mut stmt = c.prepare(
        "SELECT status, count(*) FROM comment WHERE review_round_id=?1 AND parent_id IS NULL AND deleted_at IS NULL GROUP BY status",
    )?;
    for r in stmt.query_map([id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?)))? {
        let (status, n) = r?;
        match status.as_str() {
            "Open" => dto.open_count = n,
            "In Discussion" => dto.discussion_count = n,
            _ => dto.resolved_count = n,
        }
    }
    Ok(dto)
}

fn review_rounds(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayReviewRoundsArgs,
) -> AppResult<Vec<ScreenplayReviewRoundDto>> {
    actor.require(Capability::View, "view reviews")?;
    core.project()?.store.read(|c| {
        let mut stmt = c.prepare(
            "SELECT r.id FROM review_round r JOIN screenplay_draft d ON d.id=r.draft_id
             WHERE d.screenplay_id=?1 AND r.deleted_at IS NULL AND d.deleted_at IS NULL
             ORDER BY r.created_at DESC, r.id DESC",
        )?;
        let ids: Vec<String> = stmt
            .query_map([&a.screenplay_id], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        ids.iter().map(|id| load_round(c, id)).collect()
    })
}

/// "Start Review" on a draft (FSD §23.1).
fn start_review(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayStartReviewArgs,
) -> AppResult<ScreenplayReviewRoundDto> {
    let name = required_text(&a.name, "Review name", 120)?;
    let reviewers = clean_reviewers(a.reviewers)?;
    let deadline = clean_deadline(a.deadline)?;
    let s = core.project()?;
    let id = s.store.mutate(
        actor,
        MutationMeta::new("screenplay.start_review", format!("Started review “{name}”"), Capability::Edit)
            .target("screenplay_draft", &a.draft_id),
        |tx| {
            let c = tx.conn();
            let d = load_draft(c, &a.draft_id)?;
            let id = new_id();
            let now = now_ms();
            c.execute(
                "INSERT INTO review_round(id, draft_id, name, reviewers_json, deadline, status, created_by, created_by_name,
                                          created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 'Open', ?6, ?7, ?8, ?8)",
                params![
                    id,
                    d.id,
                    name,
                    serde_json::to_string(&reviewers).map_err(|e| AppError::internal(e.to_string()))?,
                    deadline,
                    actor.user_id,
                    actor.display_name,
                    now
                ],
            )?;
            if d.status == DraftStatus::Draft {
                update_fields(c, "screenplay_draft", &d.id, &[("status", text(DraftStatus::Review.as_str()))], &["status"], None, "draft")?;
            }
            Ok(id)
        },
    )?;
    s.store.read(|c| load_round(c, &id))
}

fn update_review(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayUpdateReviewArgs,
) -> AppResult<ScreenplayReviewRoundDto> {
    let name = a
        .name
        .map(|n| required_text(&n, "Review name", 120))
        .transpose()?;
    let reviewers = a.reviewers.map(clean_reviewers).transpose()?;
    let deadline = match a.deadline {
        Some(d) => Some(clean_deadline(Some(d))?),
        None => None,
    };
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "screenplay.update_review",
            "Changed review details",
            Capability::Edit,
        )
        .target("review_round", &a.review_round_id),
        |tx| {
            let c = tx.conn();
            load_round(c, &a.review_round_id)?;
            let mut fields = Vec::new();
            if let Some(n) = name.clone() {
                fields.push(("name", text(n)));
            }
            if let Some(r) = &reviewers {
                fields.push((
                    "reviewers_json",
                    text(serde_json::to_string(r).map_err(|e| AppError::internal(e.to_string()))?),
                ));
            }
            if let Some(d) = deadline.clone() {
                fields.push(("deadline", opt_text(d)));
            }
            update_fields(
                c,
                "review_round",
                &a.review_round_id,
                &fields,
                &["name", "reviewers_json", "deadline"],
                None,
                "review round",
            )?;
            Ok(())
        },
    )?;
    s.store.read(|c| load_round(c, &a.review_round_id))
}

/// Complete a review (FSD §23.7): permission-checked; never locks the script.
fn complete_review(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayReviewRoundIdArgs,
) -> AppResult<ScreenplayReviewRoundDto> {
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new("screenplay.complete_review", "Completed a review", Capability::ResolveComments)
            .target("review_round", &a.review_round_id),
        |tx| {
            let c = tx.conn();
            let r = load_round(c, &a.review_round_id)?;
            if r.status == "Complete" {
                return Ok(());
            }
            update_fields(
                c,
                "review_round",
                &r.id,
                &[("status", text("Complete")), ("completed_at", opt_int(Some(now_ms()))), ("completed_by", text(actor.display_name.clone()))],
                &["status", "completed_at", "completed_by"],
                None,
                "review round",
            )?;
            let d = load_draft(c, &r.draft_id)?;
            let still_open: bool = c.query_row(
                "SELECT EXISTS(SELECT 1 FROM review_round WHERE draft_id=?1 AND status='Open' AND deleted_at IS NULL)",
                [&d.id],
                |x| x.get(0),
            )?;
            if d.status == DraftStatus::Review && !still_open {
                update_fields(c, "screenplay_draft", &d.id, &[("status", text(DraftStatus::Draft.as_str()))], &["status"], None, "draft")?;
            }
            Ok(())
        },
    )?;
    s.store.read(|c| load_round(c, &a.review_round_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadlines_are_real_dates() {
        assert_eq!(
            clean_deadline(Some("2026-09-30".into()))
                .unwrap()
                .as_deref(),
            Some("2026-09-30")
        );
        assert_eq!(clean_deadline(Some(" ".into())).unwrap(), None);
        assert!(clean_deadline(Some("2026-02-30".into())).is_err());
        assert!(clean_deadline(Some("30 Sep".into())).is_err());
    }

    #[test]
    fn reviewers_are_deduplicated() {
        let r = clean_reviewers(vec![
            "Priya".into(),
            " priya ".into(),
            "".into(),
            "Suresh".into(),
        ])
        .unwrap();
        assert_eq!(r, vec!["Priya", "Suresh"]);
    }
}
