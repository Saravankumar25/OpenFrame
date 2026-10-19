//! Deleting a whole screenplay (owner decision 2026-10-08, ADR-0012 amendment).
//!
//! A screenplay goes to Recently Deleted like any other item: it stays there for 30 days and
//! the 30-day purge (`modules::retention`) then removes it. Only the `screenplay` row is marked
//! deleted. Its drafts, locked drafts, scenes, production numbers, page setup, custom elements,
//! character links and Story Board links are left exactly as they were, so a restore brings
//! everything back unchanged. While it is deleted, nothing reads or edits it: draft lookups
//! require a live screenplay (`load_draft`), pickers and lists skip it, and search drops its
//! scenes.
//!
//! Rules:
//! - The screenplay that holds the Production Source (Breakdown, Schedule and Call Sheets follow
//!   it) cannot be deleted until another source is chosen.
//! - Owner and Editor only (capability SoftDelete); the store lock refuses it like every other
//!   operation. One transaction, one undo step.
//! - The last screenplay of a project may be deleted: the project then has none, as a new project
//!   does, and the Screenplay workspace offers New Screenplay again.
//! - The assistant never deletes a screenplay (`hidden::WHOLE_SCREENPLAY`); it may propose
//!   restoring one from Recently Deleted.
//! - A restore whose title another screenplay took in the meantime gets a numbered title, and the
//!   restore result says so. A screenplay whose episode is in Recently Deleted waits for the
//!   episode.
//! - Permanent removal is refused while production planning still refers to the screenplay
//!   (an earlier Production Source, breakdown rows, schedule strips or sides), as it is for a
//!   single draft; the item then stays in Recently Deleted.

use openframe_domain::{Actor, AppError, AppResult, Capability, now_ms};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Deserialize;
use ts_rs::TS;

use super::drafts::purge_draft_tx;
use super::{load_screenplay, unique_screenplay_title};
use crate::core::AppCore;
use crate::modules::comments;
use crate::registry::{Registry, TrashHandler};
use crate::store::{DeleteSpec, DeletedItemRow, MutationMeta, Tx, soft_delete};

pub fn register(r: &mut Registry) {
    use crate::registry::{OperationMetadata as M, hidden as h};
    r.command("screenplay.delete", delete).meta(
        M::soft_delete("Move a whole screenplay, with all its drafts, to Recently Deleted.")
            .confirm()
            .hidden(h::WHOLE_SCREENPLAY),
    );
    r.trash_handler(TrashHandler {
        object_type: "screenplay",
        table: "screenplay",
        label: "Screenplay",
        restore: Some(restore_screenplay),
        purge: purge_screenplay,
    });
    r.search_dependents("screenplay", search_dependents);
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayDeleteArgs {
    pub screenplay_id: String,
}

/// Title of the screenplay whose draft is the active Production Source, when it is this one.
fn production_source_title(c: &Connection, screenplay_id: &str) -> AppResult<Option<String>> {
    Ok(c.query_row(
        "SELECT sp.title FROM production_source p
           JOIN screenplay_draft d ON d.id = p.draft_id JOIN screenplay sp ON sp.id = d.screenplay_id
          WHERE p.active = 1 AND sp.id = ?1 LIMIT 1",
        [screenplay_id],
        |r| r.get(0),
    )
    .optional()?)
}

fn delete(core: &AppCore, actor: &Actor, a: ScreenplayDeleteArgs) -> AppResult<()> {
    let s = core.project()?;
    let title = s
        .store
        .read(|c| Ok(load_screenplay(c, &a.screenplay_id)?.title))?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "screenplay.delete",
            format!("Deleted screenplay “{title}”"),
            Capability::SoftDelete,
        )
        .target("screenplay", &a.screenplay_id),
        |tx| {
            let c = tx.conn();
            // Read again inside the transaction: it may have changed since.
            let sp = load_screenplay(c, &a.screenplay_id)?;
            if production_source_title(c, &sp.id)?.is_some() {
                return Err(AppError::conflict(format!(
                    "“{}” is the Production Source for Breakdown, Schedule and Call Sheets, so it can't be deleted. Choose a draft of another screenplay as the Production Source first.",
                    sp.title
                )));
            }
            soft_delete(
                tx,
                DeleteSpec {
                    object_type: "screenplay",
                    table: "screenplay",
                    id: &sp.id,
                    title: Some(sp.title.clone()),
                    parent_type: sp.episode_id.as_ref().map(|_| "episode"),
                    parent_id: sp.episode_id.clone(),
                    position: None,
                },
            )
        },
    )
}

/// Bring a screenplay back with everything it had. Clears only the screenplay's own
/// `deleted_at`; its drafts never left.
fn restore_screenplay(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let c = tx.conn();
    let (episode_id, title): (Option<String>, String) = c
        .query_row(
            "SELECT episode_id, title FROM screenplay WHERE id=?1",
            [&row.object_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("screenplay"))?;
    if let Some(e) = &episode_id {
        let episode: Option<(String, bool)> = c
            .query_row(
                "SELECT title, deleted_at IS NOT NULL FROM episode WHERE id=?1",
                [e],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        match episode {
            Some((name, true)) => {
                return Err(AppError::conflict(format!(
                    "This screenplay belongs to the episode “{name}”, which is in Recently Deleted. Restore the episode first."
                )));
            }
            None => return Err(AppError::not_found("episode")),
            Some((_, false)) => {}
        }
    }
    // Pickers list screenplays by title, so a title taken in the meantime gets a number.
    let free = unique_screenplay_title(c, episode_id.as_deref(), &title)?;
    let now = now_ms();
    if free != title {
        let page: String = c.query_row(
            "SELECT title_page_json FROM screenplay WHERE id=?1",
            [&row.object_id],
            |r| r.get(0),
        )?;
        let mut page: super::ScreenplayTitlePage = serde_json::from_str(&page).unwrap_or_default();
        page.title = free.clone();
        let page = serde_json::to_string(&page).map_err(|e| AppError::internal(e.to_string()))?;
        c.execute(
            "UPDATE screenplay SET title=?1, title_page_json=?2, deleted_at=NULL, updated_at=?3, rev=rev+1 WHERE id=?4",
            params![free, page, now, row.object_id],
        )?;
    } else {
        c.execute(
            "UPDATE screenplay SET deleted_at=NULL, updated_at=?1, rev=rev+1 WHERE id=?2",
            params![now, row.object_id],
        )?;
    }
    Ok(())
}

/// Remove a deleted screenplay for good with every draft, scene, comment, history point,
/// custom element and character link it owns.
fn purge_screenplay(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let c = tx.conn();
    let id = &row.object_id;
    let used: bool = c.query_row(
        "WITH drafts AS (SELECT id FROM screenplay_draft WHERE screenplay_id=?1),
              scenes AS (SELECT id FROM screenplay_scene WHERE draft_id IN (SELECT id FROM drafts))
         SELECT EXISTS(SELECT 1 FROM production_source WHERE draft_id IN (SELECT id FROM drafts))
             OR EXISTS(SELECT 1 FROM breakdown_element WHERE scene_id IN (SELECT id FROM scenes))
             OR EXISTS(SELECT 1 FROM schedule_strip WHERE scene_id IN (SELECT id FROM scenes))
             OR EXISTS(SELECT 1 FROM side WHERE source_draft_id IN (SELECT id FROM drafts))",
        [id],
        |r| r.get(0),
    )?;
    if used {
        return Err(AppError::conflict(
            "This screenplay is referenced by production planning, so it can't be permanently deleted. Restore it instead.",
        ));
    }
    // Newest first, so each draft's later copies are already gone when it is removed.
    let drafts: Vec<String> = {
        let mut stmt = c.prepare(
            "SELECT id FROM screenplay_draft WHERE screenplay_id=?1 ORDER BY created_at DESC, id DESC",
        )?;
        stmt.query_map([id], |r| r.get(0))?
            .collect::<Result<_, _>>()?
    };
    for d in &drafts {
        purge_draft_tx(tx, d)?;
        c.execute(
            "DELETE FROM deleted_item WHERE table_name='screenplay_draft' AND object_id=?1",
            [d],
        )?;
    }
    comments::purge_for_targets(tx, "screenplay", std::slice::from_ref(id))?;
    comments::purge_private_notes_for_targets(tx, "screenplay", std::slice::from_ref(id))?;
    c.execute(
        "DELETE FROM screenplay_character_link WHERE screenplay_id=?1",
        [id],
    )?;
    c.execute(
        "DELETE FROM screenplay_custom_element WHERE screenplay_id=?1",
        [id],
    )?;
    c.execute("DELETE FROM screenplay WHERE id=?1", [id])?;
    Ok(())
}

/// The scenes search shows for a screenplay (those of its current draft) name it in their
/// context and disappear while it is deleted, so they follow every change of its row.
fn search_dependents(c: &Connection, screenplay_id: &str) -> AppResult<Vec<(String, String)>> {
    let mut stmt = c.prepare(
        "SELECT s.id FROM screenplay_scene s JOIN screenplay sp ON sp.current_draft_id = s.draft_id
          WHERE sp.id=?1 AND s.deleted_at IS NULL",
    )?;
    let ids = stmt
        .query_map([screenplay_id], |r| r.get::<_, String>(0))?
        .map(|id| id.map(|id| ("screenplay_scene".to_string(), id)))
        .collect::<Result<_, _>>()?;
    Ok(ids)
}
