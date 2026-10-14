//! Production scene numbers (ADR-0015; Screenplay and Production Depth
//! Specification §12).
//!
//! A scene number is a label, never identity: breakdown, schedule, shots and
//! call sheets reference `screenplay_scene.id` / `lineage_id`, so changing a
//! label can never break them.
//!
//! - While no scene of a draft has a stored label, numbers are derived from
//!   order (1, 2, 3 …) and renumber on reorder, as before.
//! - Labels are stored when a script is imported with scene numbers
//!   (`imported`), when a draft is locked (`locked`: the derived numbers are
//!   frozen), when a later scene is inserted into a frozen draft (`inserted`,
//!   frozen at the next lock) or when the writer sets one (`manual`).
//! - In a draft with stored labels, a scene without one gets an inserted label
//!   by rule: after 12 comes 12A, then 12B; before the first scene A1, B1.
//!   Reordering never changes a stored label.
//! - Deleting a numbered scene from a revision of a locked draft keeps its
//!   label with `omitted = 1`; exports print it as OMITTED and it can be
//!   restored. In other drafts a deleted scene goes to Recently Deleted.
//!
//! Operations: `screenplay.set_scene_number` (with a dependency warning that
//! must be acknowledged when the scene is used in production planning),
//! `screenplay.renumber_scenes` (back to derived numbers, unlocked drafts that
//! are not revisions) and `screenplay.restore_omitted_scene`. All undoable,
//! all human-only (contract doc 32 §2.5: numbering overrides stay with people).

use std::collections::HashSet;

use openframe_domain::enums::DraftStatus;
use openframe_domain::{Actor, AppError, AppResult, Capability};
use openframe_persistence::rows::{int, opt_text, update_fields};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{bump_seq, load_draft};
use crate::core::AppCore;
use crate::registry::Registry;
use crate::store::{MutationMeta, Tx};

pub fn register(r: &mut Registry) {
    use crate::registry::{OperationMetadata as M, hidden as h};
    r.command("screenplay.set_scene_number", set_scene_number)
        .meta(
            M::edit("Set or clear a scene's production number (for example 12A).")
                .hidden(h::NUMBERING),
        );
    r.command("screenplay.renumber_scenes", renumber_scenes)
        .meta(
            M::edit("Return a draft to scene numbers derived from scene order.")
                .hidden(h::NUMBERING),
        );
    r.command("screenplay.restore_omitted_scene", restore_omitted_scene)
        .meta(
            M::edit("Bring back a scene that was omitted from a numbered draft.")
                .hidden(h::NUMBERING),
        );
}

/// Longest production number label.
pub const MAX_LABEL_CHARS: usize = 12;

/// One live scene, in draft order, as numbering sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NumberRow {
    pub stored: Option<String>,
}

/// Display labels for every live scene of a draft, in order (omitted scenes included).
pub(crate) fn production_labels(rows: &[NumberRow]) -> Vec<String> {
    if rows.iter().all(|r| r.stored.is_none()) {
        return (1..=rows.len()).map(|n| n.to_string()).collect();
    }
    let mut taken: HashSet<String> = rows
        .iter()
        .filter_map(|r| r.stored.as_ref().map(|s| s.to_ascii_uppercase()))
        .collect();
    let mut out: Vec<String> = Vec::with_capacity(rows.len());
    for (i, r) in rows.iter().enumerate() {
        if let Some(s) = &r.stored {
            out.push(s.clone());
            continue;
        }
        let label = match out.last() {
            Some(prev) => after(prev, &taken),
            None => {
                let next = rows[i..]
                    .iter()
                    .find_map(|r| r.stored.clone())
                    .unwrap_or_else(|| "1".into());
                before(&next, &taken)
            }
        };
        taken.insert(label.to_ascii_uppercase());
        out.push(label);
    }
    out
}

/// "A", "B", … "Z", "AA", "AB", … for n = 0, 1, …
fn letters(mut n: usize) -> String {
    let mut s = Vec::new();
    loop {
        s.push((b'A' + (n % 26) as u8) as char);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    s.iter().rev().collect()
}

/// The first free label after `prev`: 12 → 12A, 12A → 12B.
fn after(prev: &str, taken: &HashSet<String>) -> String {
    let digits_end = prev
        .char_indices()
        .rfind(|(_, c)| c.is_ascii_digit())
        .map(|(i, c)| i + c.len_utf8());
    let (base, suffix) = match digits_end {
        Some(end) if prev[end..].chars().all(|c| c.is_ascii_alphabetic()) => {
            (&prev[..end], &prev[end..])
        }
        _ => (prev, ""),
    };
    let start = (0..)
        .take(26 * 27)
        .position(|n| letters(n).eq_ignore_ascii_case(suffix))
        .map(|p| p + 1)
        .unwrap_or(0);
    (start..)
        .map(|n| format!("{base}{}", letters(n)))
        .find(|c| !taken.contains(&c.to_ascii_uppercase()))
        .expect("an unbounded sequence always has a free label")
}

/// The first free label before `next`: 1 → A1, B1.
fn before(next: &str, taken: &HashSet<String>) -> String {
    (0..)
        .map(|n| format!("{}{next}", letters(n)))
        .find(|c| !taken.contains(&c.to_ascii_uppercase()))
        .expect("an unbounded sequence always has a free label")
}

/// Stored labels of a draft's live scenes, in order: (scene id, stored label, omitted).
pub(crate) fn number_rows(
    c: &Connection,
    draft_id: &str,
) -> AppResult<Vec<(String, Option<String>, bool)>> {
    let mut stmt = c.prepare(
        "SELECT id, production_number, omitted FROM screenplay_scene
         WHERE draft_id=?1 AND deleted_at IS NULL ORDER BY position, id",
    )?;
    let rows = stmt
        .query_map([draft_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)? != 0))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// (scene id, display label) for every live scene of a draft.
pub(crate) fn draft_labels(c: &Connection, draft_id: &str) -> AppResult<Vec<(String, String)>> {
    let rows = number_rows(c, draft_id)?;
    let labels = production_labels(
        &rows
            .iter()
            .map(|(_, s, _)| NumberRow { stored: s.clone() })
            .collect::<Vec<_>>(),
    );
    Ok(rows.into_iter().map(|(id, _, _)| id).zip(labels).collect())
}

/// Whether a draft's numbers are frozen (any scene carries a stored label).
pub(crate) fn is_frozen(c: &Connection, draft_id: &str) -> AppResult<bool> {
    Ok(c.query_row(
        "SELECT EXISTS(SELECT 1 FROM screenplay_scene WHERE draft_id=?1 AND deleted_at IS NULL
                         AND production_number IS NOT NULL)",
        [draft_id],
        |r| r.get(0),
    )?)
}

/// Display label of one scene.
pub(crate) fn scene_label(c: &Connection, scene_id: &str) -> AppResult<Option<String>> {
    let draft: Option<String> = c
        .query_row(
            "SELECT draft_id FROM screenplay_scene WHERE id=?1 AND deleted_at IS NULL",
            [scene_id],
            |r| r.get(0),
        )
        .optional()?;
    let Some(draft) = draft else { return Ok(None) };
    Ok(draft_labels(c, &draft)?
        .into_iter()
        .find(|(id, _)| id == scene_id)
        .map(|(_, l)| l))
}

/// On lock: store every derived or inserted label (ADR-0015 decision 2).
pub(crate) fn freeze_tx(tx: &Tx<'_>, draft_id: &str) -> AppResult<()> {
    let c = tx.conn();
    let frozen = is_frozen(c, draft_id)?;
    let rows = number_rows(c, draft_id)?;
    let labels = draft_labels(c, draft_id)?;
    for ((id, stored, _), (_, label)) in rows.iter().zip(labels) {
        if stored.is_none() {
            let source = if frozen { "inserted" } else { "locked" };
            update_fields(
                c,
                "screenplay_scene",
                id,
                &[
                    ("production_number", opt_text(Some(label))),
                    (
                        "production_number_source",
                        opt_text(Some(source.to_string())),
                    ),
                ],
                &["production_number", "production_number_source"],
                None,
                "scene",
            )?;
        }
    }
    Ok(())
}

/// Validate a label the writer typed: letters, digits, '.' and '-'.
pub(crate) fn validate_label(label: &str) -> AppResult<String> {
    let l = label.trim().to_ascii_uppercase();
    let ok = !l.is_empty()
        && l.chars().count() <= MAX_LABEL_CHARS
        && l.chars().next().is_some_and(|c| c.is_ascii_alphanumeric())
        && l.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
    if !ok {
        return Err(AppError::validation(
            "number",
            format!(
                "A scene number uses letters and digits (for example 12 or 12A), up to {MAX_LABEL_CHARS} characters."
            ),
        ));
    }
    Ok(l)
}

/// Production planning that uses a scene (the Scene Hub's summary), if any.
pub(crate) fn dependency_summary(c: &Connection, scene_id: &str) -> AppResult<Option<String>> {
    let hub = super::extras::hub_for(c, scene_id)?;
    Ok(hub.used_in_production.then_some(hub.production_summary))
}

// ------------------------------------------------------------------ args

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplaySetSceneNumberArgs {
    pub scene_id: String,
    /// The new label (for example "12A"); None or empty clears a manual label.
    #[serde(default)]
    #[ts(optional)]
    pub number: Option<String>,
    /// The writer saw the production dependency warning and continues.
    #[serde(default)]
    #[ts(optional)]
    pub acknowledge_dependencies: Option<bool>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplaySceneNumberResult {
    pub scene_id: String,
    pub label: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplaySceneIdArgs {
    pub scene_id: String,
}

// -------------------------------------------------------------- commands

fn scene_draft(c: &Connection, scene_id: &str) -> AppResult<(String, Option<String>, bool)> {
    c.query_row(
        "SELECT draft_id, production_number, omitted FROM screenplay_scene WHERE id=?1 AND deleted_at IS NULL",
        [scene_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)? != 0)),
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("scene"))
}

fn set_scene_number(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplaySetSceneNumberArgs,
) -> AppResult<ScreenplaySceneNumberResult> {
    crate::util::require_id(&a.scene_id, "scene")?;
    let label = match a.number.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(n) => Some(validate_label(n)?),
        None => None,
    };
    let s = core.project()?;
    let summary = match &label {
        Some(l) => format!("Set a scene number to {l}"),
        None => "Cleared a scene number".to_string(),
    };
    s.store.mutate(
        actor,
        MutationMeta::new("screenplay.set_scene_number", summary, Capability::Edit)
            .target("screenplay_scene", &a.scene_id),
        |tx| {
            let c = tx.conn();
            let (draft_id, _, _) = scene_draft(c, &a.scene_id)?;
            if load_draft(c, &draft_id)?.status == DraftStatus::Locked {
                return Err(AppError::locked_draft());
            }
            if let Some(deps) = dependency_summary(c, &a.scene_id)?
                && !a.acknowledge_dependencies.unwrap_or(false)
            {
                return Err(AppError::validation(
                    "dependencies",
                    format!(
                        "This scene is already used in production planning ({deps}). Its new number will show everywhere; the production items stay linked to the scene."
                    ),
                ));
            }
            if let Some(l) = &label {
                let clash: Option<String> = draft_labels(c, &draft_id)?
                    .into_iter()
                    .find(|(id, x)| id != &a.scene_id && x.eq_ignore_ascii_case(l))
                    .map(|(_, x)| x);
                if clash.is_some() {
                    return Err(AppError::validation(
                        "number",
                        format!("Another scene in this draft is already number {l}."),
                    ));
                }
            }
            let source = label.as_ref().map(|_| "manual".to_string());
            update_fields(
                c,
                "screenplay_scene",
                &a.scene_id,
                &[
                    ("production_number", opt_text(label.clone())),
                    ("production_number_source", opt_text(source)),
                ],
                &["production_number", "production_number_source"],
                None,
                "scene",
            )?;
            for (id, _) in draft_labels(c, &draft_id)? {
                tx.reindex("screenplay_scene", &id);
            }
            bump_seq(c, &draft_id)?;
            Ok(())
        },
    )?;
    let label = s
        .store
        .read(|c| scene_label(c, &a.scene_id))?
        .unwrap_or_default();
    Ok(ScreenplaySceneNumberResult {
        scene_id: a.scene_id,
        label,
    })
}

fn renumber_scenes(
    core: &AppCore,
    actor: &Actor,
    a: super::ScreenplayDraftIdArgs,
) -> AppResult<()> {
    crate::util::require_id(&a.draft_id, "draft")?;
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "screenplay.renumber_scenes",
            "Renumbered scenes from their order",
            Capability::Edit,
        )
        .target("screenplay_draft", &a.draft_id),
        |tx| {
            let c = tx.conn();
            let d = load_draft(c, &a.draft_id)?;
            if matches!(d.status, DraftStatus::Locked | DraftStatus::Revision) {
                return Err(AppError::validation(
                    "status",
                    "Scene numbers of a locked draft or a revision stay frozen so production paperwork keeps matching.",
                ));
            }
            let omitted: i64 = c.query_row(
                "SELECT count(*) FROM screenplay_scene WHERE draft_id=?1 AND deleted_at IS NULL AND omitted=1",
                [&d.id],
                |r| r.get(0),
            )?;
            if omitted > 0 {
                return Err(AppError::validation(
                    "omitted",
                    "Restore or delete the omitted scenes before renumbering.",
                ));
            }
            for (id, stored, _) in number_rows(c, &d.id)? {
                if stored.is_some() {
                    update_fields(
                        c,
                        "screenplay_scene",
                        &id,
                        &[
                            ("production_number", opt_text(None::<String>)),
                            ("production_number_source", opt_text(None::<String>)),
                        ],
                        &["production_number", "production_number_source"],
                        None,
                        "scene",
                    )?;
                }
                tx.reindex("screenplay_scene", &id);
            }
            bump_seq(c, &d.id)?;
            Ok(())
        },
    )
}

fn restore_omitted_scene(core: &AppCore, actor: &Actor, a: ScreenplaySceneIdArgs) -> AppResult<()> {
    crate::util::require_id(&a.scene_id, "scene")?;
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "screenplay.restore_omitted_scene",
            "Restored an omitted scene",
            Capability::Edit,
        )
        .target("screenplay_scene", &a.scene_id),
        |tx| {
            let c = tx.conn();
            let (draft_id, _, omitted) = scene_draft(c, &a.scene_id)?;
            super::editable_draft(c, &draft_id)?;
            if !omitted {
                return Err(AppError::conflict("This scene is not omitted."));
            }
            update_fields(
                c,
                "screenplay_scene",
                &a.scene_id,
                &[("omitted", int(0))],
                &["omitted"],
                None,
                "scene",
            )?;
            tx.reindex("screenplay_scene", &a.scene_id);
            bump_seq(c, &draft_id)?;
            Ok(())
        },
    )
}

/// Mark a numbered scene of a frozen draft as omitted instead of deleting it
/// (ADR-0015 decision 3). Returns false when the scene should be deleted normally.
pub(crate) fn omit_instead_of_delete(tx: &Tx<'_>, scene_id: &str) -> AppResult<bool> {
    let c = tx.conn();
    let (draft_id, stored, _) = scene_draft(c, scene_id)?;
    if stored.is_none() || load_draft(c, &draft_id)?.status != DraftStatus::Revision {
        return Ok(false);
    }
    update_fields(
        c,
        "screenplay_scene",
        scene_id,
        &[("omitted", int(1))],
        &["omitted"],
        None,
        "scene",
    )?;
    tx.reindex("screenplay_scene", scene_id);
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(v: &[Option<&str>]) -> Vec<NumberRow> {
        v.iter()
            .map(|s| NumberRow {
                stored: s.map(str::to_string),
            })
            .collect()
    }

    #[test]
    fn unfrozen_drafts_number_by_order() {
        assert_eq!(
            production_labels(&rows(&[None, None, None])),
            ["1", "2", "3"]
        );
    }

    #[test]
    fn inserted_scenes_get_lettered_labels() {
        let r = rows(&[Some("1"), None, None, Some("2"), Some("12A"), None]);
        assert_eq!(production_labels(&r), ["1", "1A", "1B", "2", "12A", "12B"]);
    }

    #[test]
    fn inserted_labels_never_reuse_a_stored_label() {
        let r = rows(&[Some("5"), None, Some("5A")]);
        assert_eq!(production_labels(&r), ["5", "5B", "5A"]);
    }

    #[test]
    fn a_scene_before_the_first_gets_a_prefix() {
        let r = rows(&[None, Some("1"), Some("2")]);
        assert_eq!(production_labels(&r), ["A1", "1", "2"]);
    }

    #[test]
    fn letters_run_past_z() {
        assert_eq!(letters(0), "A");
        assert_eq!(letters(25), "Z");
        assert_eq!(letters(26), "AA");
        let mut taken = HashSet::new();
        for n in 0..30 {
            taken.insert(format!("7{}", letters(n)));
        }
        assert_eq!(after("7", &taken), "7AE");
    }

    #[test]
    fn labels_are_validated() {
        assert_eq!(validate_label(" 12a ").unwrap(), "12A");
        assert_eq!(validate_label("A1").unwrap(), "A1");
        assert!(validate_label("").is_err());
        assert!(validate_label("12 A").is_err());
        assert!(validate_label("-1").is_err());
        assert!(validate_label("1234567890123").is_err());
        assert!(
            validate_label("१२").is_err(),
            "ASCII digits only in production numbers"
        );
    }
}
