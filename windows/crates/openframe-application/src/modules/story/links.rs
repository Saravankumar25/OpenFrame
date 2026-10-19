//! Scene Card to screenplay scene links that survive scene identity changes.
//!
//! Build Screenplay stores the id of the scene it created on the card (`screenplay_scene_id`).
//! Screenplay scene ids are per draft: a new draft copies every scene with a new id and the same
//! `lineage_id`. The stored id therefore points at the draft that was built, not at the draft the
//! writer works in now. Every read of the link goes through [`resolve`], which follows the
//! lineage:
//!
//! 1. the scene with the same lineage in the screenplay's current draft, when it is there;
//! 2. otherwise the stored scene, when it and its draft still exist;
//! 3. otherwise the newest other draft of that screenplay that still has the lineage;
//! 4. otherwise nothing: the scene was removed everywhere, so the card shows no link.
//!
//! The stored id is never rewritten here (reads stay reads); it remains the record of which scene
//! the card created.

use openframe_domain::AppResult;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use ts_rs::TS;

/// A live screenplay scene a card currently points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedScene {
    pub scene_id: String,
    pub lineage_id: String,
    pub draft_id: String,
    pub heading: String,
}

const LIVE_SCENE: &str = "SELECT s.id, s.lineage_id, s.draft_id, s.heading
     FROM screenplay_scene s JOIN screenplay_draft d ON d.id = s.draft_id
     JOIN screenplay p ON p.id = d.screenplay_id
     WHERE s.deleted_at IS NULL AND d.deleted_at IS NULL AND p.deleted_at IS NULL";

fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<ResolvedScene> {
    Ok(ResolvedScene {
        scene_id: r.get(0)?,
        lineage_id: r.get(1)?,
        draft_id: r.get(2)?,
        heading: r.get(3)?,
    })
}

/// Follow a stored scene link to the scene the card means today (see the module comment).
pub(crate) fn resolve(c: &Connection, stored_scene_id: &str) -> AppResult<Option<ResolvedScene>> {
    // The stored scene's lineage and screenplay, even when that scene or its draft was deleted.
    let origin: Option<(String, String, Option<String>)> = c
        .query_row(
            "SELECT s.lineage_id, d.screenplay_id, p.current_draft_id
             FROM screenplay_scene s JOIN screenplay_draft d ON d.id = s.draft_id
             JOIN screenplay p ON p.id = d.screenplay_id
             WHERE s.id=?1",
            [stored_scene_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let Some((lineage, screenplay_id, current)) = origin else {
        return Ok(None);
    };
    if let Some(current) = current {
        let found = c
            .prepare_cached(&format!(
                "{LIVE_SCENE} AND s.lineage_id=?1 AND s.draft_id=?2 ORDER BY s.position, s.id LIMIT 1"
            ))?
            .query_row(params![lineage, current], row)
            .optional()?;
        if found.is_some() {
            return Ok(found);
        }
    }
    let stored = c
        .prepare_cached(&format!("{LIVE_SCENE} AND s.id=?1"))?
        .query_row([stored_scene_id], row)
        .optional()?;
    if stored.is_some() {
        return Ok(stored);
    }
    Ok(c.prepare_cached(&format!(
        "{LIVE_SCENE} AND s.lineage_id=?1 AND d.screenplay_id=?2
         ORDER BY d.created_at DESC, d.id DESC, s.position, s.id LIMIT 1"
    ))?
    .query_row(params![lineage, screenplay_id], row)
    .optional()?)
}

/// Production state of the scene a card points at, for the Card Display toggles
/// Revision/Change, Breakdown Status and Shot/Storyboard Status. Derived on read, never stored.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryCardSceneStatus {
    /// The scene the link resolves to now (current draft first).
    pub scene_id: String,
    pub draft_id: String,
    pub draft_name: String,
    /// Lines in the scene carry a revision mark.
    pub revised: bool,
    /// Confirmed or manual breakdown elements for the scene.
    #[ts(type = "number")]
    pub breakdown_items: i64,
    /// The scene's breakdown is marked complete.
    pub breakdown_complete: bool,
    #[ts(type = "number")]
    pub shots: i64,
    #[ts(type = "number")]
    pub storyboards: i64,
}

pub(crate) fn scene_status(c: &Connection, s: &ResolvedScene) -> AppResult<StoryCardSceneStatus> {
    let count = |sql: &str, p: &str| -> AppResult<i64> {
        Ok(c.prepare_cached(sql)?.query_row([p], |r| r.get(0))?)
    };
    let draft_name: String = c
        .prepare_cached("SELECT name FROM screenplay_draft WHERE id=?1")?
        .query_row([&s.draft_id], |r| r.get(0))?;
    let revised = count(
        "SELECT count(*) FROM screenplay_element WHERE scene_id=?1 AND revision_mark IS NOT NULL AND revision_mark <> ''",
        &s.scene_id,
    )? > 0;
    let breakdown_items = count(
        "SELECT count(*) FROM breakdown_element WHERE scene_lineage_id=?1 AND deleted_at IS NULL
           AND confirmation_state IN ('Confirmed','Manual')",
        &s.lineage_id,
    )?;
    let breakdown_complete = count(
        "SELECT count(*) FROM production_scene_state WHERE scene_lineage_id=?1 AND complete=1",
        &s.lineage_id,
    )? > 0;
    let shots = count(
        "SELECT count(*) FROM shot WHERE scene_lineage_id=?1 AND deleted_at IS NULL",
        &s.lineage_id,
    )?;
    let storyboards = count(
        "SELECT count(*) FROM storyboard WHERE scene_lineage_id=?1 AND deleted_at IS NULL",
        &s.lineage_id,
    )?;
    Ok(StoryCardSceneStatus {
        scene_id: s.scene_id.clone(),
        draft_id: s.draft_id.clone(),
        draft_name,
        revised,
        breakdown_items,
        breakdown_complete,
        shots,
        storyboards,
    })
}
