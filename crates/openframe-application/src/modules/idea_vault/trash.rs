//! Recently Deleted for the vault (FSD §5.11, §52; FSD-IDEA-018; mock 044):
//! restore returns items to their folder (or to All items, and says so);
//! permanent deletion is a deliberate second action and removes owned files.

use openframe_domain::enums::VaultItemType;
use openframe_domain::{Actor, AppError, AppResult, Capability, now_ms};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::model::type_label;
use super::{mutate_vault, read_vault};
use crate::core::AppCore;
use crate::modules::files::purge_asset_if_unreferenced;
use crate::store::{
    DeletedItemRow, MutationMeta, Tx, load_deleted, purge_deleted, restore_deleted,
};
use crate::util::StoreSel;

const VAULT_TYPES: &str = "'vault_item','vault_folder','vault_collection'";

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultTrashArgs {
    #[serde(default)]
    pub store: StoreSel,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultTrashItemArgs {
    #[serde(default)]
    pub store: StoreSel,
    /// deleted_item id.
    pub id: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct VaultTrashRow {
    pub deleted: DeletedItemRow,
    /// "Note", "Image", "Folder", "Collection"…
    pub type_label: String,
    /// "Idea Vault" or "Idea Vault · Locations" (mock 044 "Original place").
    pub original_place: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct VaultRestoreResult {
    pub restored: DeletedItemRow,
    /// Human confirmation; says where the item went when its place is gone.
    pub message: String,
}

fn folder_live(c: &Connection, id: &str) -> AppResult<bool> {
    Ok(c.query_row(
        "SELECT EXISTS(SELECT 1 FROM vault_folder WHERE id=?1 AND deleted_at IS NULL)",
        [id],
        |r| r.get(0),
    )?)
}

fn folder_name(c: &Connection, id: &str) -> Option<String> {
    c.query_row("SELECT name FROM vault_folder WHERE id=?1", [id], |r| {
        r.get(0)
    })
    .optional()
    .ok()
    .flatten()
}

pub(crate) fn list(
    core: &AppCore,
    actor: &Actor,
    args: VaultTrashArgs,
) -> AppResult<Vec<VaultTrashRow>> {
    read_vault(core, actor, args.store, |c, _| {
        let mut stmt = c.prepare(&format!(
            "SELECT id, object_type, object_id, table_name, title, parent_type, parent_id, position, deleted_at, deleted_by
             FROM deleted_item WHERE object_type IN ({VAULT_TYPES}) ORDER BY deleted_at DESC"
        ))?;
        let rows = stmt
            .query_map([], |r| {
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
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let type_label = match row.object_type.as_str() {
                "vault_item" => c
                    .query_row(
                        "SELECT item_type FROM vault_item WHERE id=?1",
                        [&row.object_id],
                        |r| r.get::<_, String>(0),
                    )
                    .optional()?
                    .and_then(|t| VaultItemType::parse(&t))
                    .map(type_label)
                    .unwrap_or("Item")
                    .to_string(),
                "vault_folder" => "Folder".into(),
                _ => "Collection".into(),
            };
            let original_place = match row.parent_id.as_deref().and_then(|p| folder_name(c, p)) {
                Some(f) => format!("Idea Vault · {f}"),
                None => "Idea Vault".into(),
            };
            out.push(VaultTrashRow {
                deleted: row,
                type_label,
                original_place,
            });
        }
        Ok(out)
    })
}

fn check_vault_row(c: &Connection, id: &str) -> AppResult<DeletedItemRow> {
    let row = load_deleted(c, id)?;
    if !matches!(
        row.object_type.as_str(),
        "vault_item" | "vault_folder" | "vault_collection"
    ) {
        return Err(AppError::not_found("deleted Idea Vault item"));
    }
    Ok(row)
}

pub(crate) fn restore(
    core: &AppCore,
    actor: &Actor,
    args: VaultTrashItemArgs,
) -> AppResult<VaultRestoreResult> {
    let (row, parent_gone) = read_vault(core, actor, args.store, |c, _| {
        let row = check_vault_row(c, &args.id)?;
        let gone = match &row.parent_id {
            Some(p) => !folder_live(c, p)?,
            None => false,
        };
        Ok((row, gone))
    })?;
    let title = row.title.clone().unwrap_or_else(|| "item".into());
    let restored = mutate_vault(
        core,
        actor,
        args.store,
        MutationMeta::new(
            "vault.restore",
            format!("Restored “{title}”"),
            Capability::SoftDelete,
        ),
        |tx| restore_deleted(tx, &args.id),
    )?;
    let message = if parent_gone {
        format!("Restored “{title}” to All items because its folder was deleted.")
    } else {
        format!("Restored “{title}”.")
    };
    Ok(VaultRestoreResult { restored, message })
}

pub(crate) fn purge(
    core: &AppCore,
    actor: &Actor,
    args: VaultTrashItemArgs,
) -> AppResult<DeletedItemRow> {
    let row = read_vault(core, actor, args.store, |c, _| check_vault_row(c, &args.id))?;
    let title = row.title.clone().unwrap_or_else(|| "item".into());
    mutate_vault(
        core,
        actor,
        args.store,
        MutationMeta::new(
            "vault.purge",
            format!("Permanently deleted “{title}”"),
            Capability::PermanentDelete,
        )
        .not_undoable(),
        |tx| purge_deleted(tx, &args.id),
    )
}

// -------------------------------------------------------------- trash handlers

/// Back to the original folder if it still exists; otherwise the top level.
pub(crate) fn restore_item(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let folder = match &row.parent_id {
        Some(f) if folder_live(tx.conn(), f)? => Some(f.clone()),
        _ => None,
    };
    tx.conn().execute(
        "UPDATE vault_item SET deleted_at=NULL, folder_id=?1, updated_at=?2, rev=rev+1 WHERE id=?3",
        params![folder, now_ms(), row.object_id],
    )?;
    Ok(())
}

pub(crate) fn purge_item(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let c = tx.conn();
    let asset_id: Option<String> = c
        .query_row(
            "SELECT asset_id FROM vault_item WHERE id=?1",
            [&row.object_id],
            |r| r.get(0),
        )
        .optional()?
        .flatten();
    c.execute(
        "DELETE FROM vault_item_tag WHERE item_id=?1",
        [&row.object_id],
    )?;
    c.execute(
        "DELETE FROM vault_item_collection WHERE item_id=?1",
        [&row.object_id],
    )?;
    c.execute("DELETE FROM vault_item WHERE id=?1", [&row.object_id])?;
    if let Some(a) = asset_id {
        purge_asset_if_unreferenced(tx, &a)?;
    }
    Ok(())
}

pub(crate) fn restore_folder(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let parent = match &row.parent_id {
        Some(p) if folder_live(tx.conn(), p)? => Some(p.clone()),
        _ => None,
    };
    tx.conn().execute(
        "UPDATE vault_folder SET deleted_at=NULL, parent_id=?1, updated_at=?2, rev=rev+1 WHERE id=?3",
        params![parent, now_ms(), row.object_id],
    )?;
    Ok(())
}

pub(crate) fn purge_folder(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let c = tx.conn();
    let now = now_ms();
    // Deleted items may still remember this folder; they will restore to the top level.
    c.execute(
        "UPDATE vault_item SET folder_id=NULL, updated_at=?1, rev=rev+1 WHERE folder_id=?2",
        params![now, row.object_id],
    )?;
    c.execute(
        "UPDATE vault_folder SET parent_id=NULL, updated_at=?1, rev=rev+1 WHERE parent_id=?2",
        params![now, row.object_id],
    )?;
    c.execute("DELETE FROM vault_folder WHERE id=?1", [&row.object_id])?;
    Ok(())
}

pub(crate) fn purge_collection(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let c = tx.conn();
    c.execute(
        "DELETE FROM vault_item_collection WHERE collection_id=?1",
        [&row.object_id],
    )?;
    c.execute("DELETE FROM vault_collection WHERE id=?1", [&row.object_id])?;
    Ok(())
}
