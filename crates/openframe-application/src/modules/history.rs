//! Undo/redo, activity history and Recently Deleted (FSD §51, §52, §106).

use openframe_domain::{Actor, AppResult, Capability};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::core::AppCore;
use crate::modules::project::{ActivityEntry, recent_activity};
use crate::registry::Registry;
use crate::store::{DeletedItemRow, MutationMeta, UndoInfo, purge_deleted, restore_deleted};
use crate::util::{StoreSel, with_store};

pub fn register(r: &mut Registry) {
    r.command("history.undo", undo);
    r.command("history.redo", redo);
    r.query("history.info", info);
    r.query("history.activity", activity);
    r.query("trash.list", trash_list);
    r.command("trash.restore", trash_restore);
    r.command("trash.purge", trash_purge);
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoreArgs {
    #[serde(default)]
    pub store: StoreSel,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StepResult {
    /// Label of the step that was undone/redone; None when there was nothing to do.
    pub label: Option<String>,
    pub info: UndoInfo,
}

fn undo(core: &AppCore, actor: &Actor, args: StoreArgs) -> AppResult<StepResult> {
    with_store(core, args.store, |s| {
        let label = s.undo(actor)?;
        Ok(StepResult {
            label,
            info: s.undo_info(actor)?,
        })
    })
}

fn redo(core: &AppCore, actor: &Actor, args: StoreArgs) -> AppResult<StepResult> {
    with_store(core, args.store, |s| {
        let label = s.redo(actor)?;
        Ok(StepResult {
            label,
            info: s.undo_info(actor)?,
        })
    })
}

fn info(core: &AppCore, actor: &Actor, args: StoreArgs) -> AppResult<UndoInfo> {
    with_store(core, args.store, |s| s.undo_info(actor))
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActivityArgs {
    #[serde(default)]
    pub store: StoreSel,
    #[serde(default)]
    #[ts(type = "number | null")]
    pub limit: Option<i64>,
}

fn activity(core: &AppCore, actor: &Actor, args: ActivityArgs) -> AppResult<Vec<ActivityEntry>> {
    actor.require(Capability::View, "view project activity")?;
    let limit = args.limit.unwrap_or(200).clamp(1, 1000);
    with_store(core, args.store, |s| s.read(|c| recent_activity(c, limit)))
}

fn trash_list(core: &AppCore, actor: &Actor, args: StoreArgs) -> AppResult<Vec<DeletedItemRow>> {
    actor.require(Capability::View, "view deleted items")?;
    with_store(core, args.store, |s| {
        s.read(|c| {
            let mut stmt = c.prepare(
                "SELECT id, object_type, object_id, table_name, title, parent_type, parent_id, position, deleted_at, deleted_by
                 FROM deleted_item ORDER BY deleted_at DESC",
            )?;
            let rows = stmt
                .query_map(params![], |r| {
                    Ok(DeletedItemRow {
                        id: r.get(0)?,
                        object_type: r.get(1)?,
                        object_id: r.get(2)?,
                        table_name: r.get(3)?,
                        title: r.get(4)?,
                        parent_type: r.get(5)?,
                        parent_id: r.get(6)?,
                        position: r.get(7)?,
                        deleted_at: r.get(8)?,
                        deleted_by: r.get(9)?,
                        restored_note: None,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            // Private notes in the trash are visible only to their owner (Security §8).
            let mut out = Vec::with_capacity(rows.len());
            for row in rows {
                if row.object_type == "private_note" {
                    let owner: Option<String> = c
                        .query_row("SELECT owner_user_id FROM private_note WHERE id=?1", [&row.object_id], |r| r.get(0))
                        .ok();
                    if owner.as_deref() != Some(actor.user_id.as_str()) {
                        continue;
                    }
                }
                out.push(row);
            }
            Ok(out)
        })
    })
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrashItemArgs {
    #[serde(default)]
    pub store: StoreSel,
    pub id: String,
}

fn trash_restore(core: &AppCore, actor: &Actor, args: TrashItemArgs) -> AppResult<DeletedItemRow> {
    with_store(core, args.store, |s| {
        let title = s
            .read(|c| crate::store::load_deleted(c, &args.id))?
            .title
            .unwrap_or_else(|| "item".into());
        s.mutate(
            actor,
            MutationMeta::new(
                "trash.restore",
                format!("Restored “{title}”"),
                Capability::SoftDelete,
            ),
            |tx| restore_deleted(tx, &args.id),
        )
    })
}

fn trash_purge(core: &AppCore, actor: &Actor, args: TrashItemArgs) -> AppResult<DeletedItemRow> {
    with_store(core, args.store, |s| {
        let title = s
            .read(|c| crate::store::load_deleted(c, &args.id))?
            .title
            .unwrap_or_else(|| "item".into());
        s.mutate(
            actor,
            MutationMeta::new(
                "trash.purge",
                format!("Permanently deleted “{title}”"),
                Capability::PermanentDelete,
            )
            .not_undoable(),
            |tx| purge_deleted(tx, &args.id),
        )
    })
}
