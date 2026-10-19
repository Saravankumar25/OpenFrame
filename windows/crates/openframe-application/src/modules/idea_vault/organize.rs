//! Folders (one per item) and collections (N:M thematic groups) — FSD §5.6, §88.7.

use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_persistence::rows::{next_position, opt_text, text, update_fields};
use rusqlite::{OptionalExtension, params};
use serde::Deserialize;
use ts_rs::TS;

use super::items::check_folder;
use super::model::{VaultCollectionDto, VaultFolderDto};
use super::{mutate_vault, read_vault};
use crate::core::AppCore;
use crate::store::{DeleteSpec, MutationMeta, soft_delete};
use crate::util::{StoreSel, optional_text, required_text};

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultCreateFolderArgs {
    #[serde(default)]
    pub store: StoreSel,
    pub name: String,
    #[serde(default)]
    pub parent_id: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultRenameFolderArgs {
    #[serde(default)]
    pub store: StoreSel,
    pub id: String,
    pub name: String,
    #[serde(default)]
    #[ts(type = "number | null")]
    pub expected_rev: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultCreateCollectionArgs {
    #[serde(default)]
    pub store: StoreSel,
    pub name: String,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultRenameCollectionArgs {
    #[serde(default)]
    pub store: StoreSel,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    #[ts(type = "number | null")]
    pub expected_rev: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultStoreIdArgs {
    #[serde(default)]
    pub store: StoreSel,
    pub id: String,
}

const MAX_NAME: usize = 120;

pub(crate) fn create_folder(
    core: &AppCore,
    actor: &Actor,
    args: VaultCreateFolderArgs,
) -> AppResult<VaultFolderDto> {
    let name = required_text(&args.name, "Folder name", MAX_NAME)?;
    let id = new_id();
    mutate_vault(
        core,
        actor,
        args.store,
        MutationMeta::new(
            "vault.create_folder",
            format!("Created folder “{name}”"),
            Capability::Edit,
        ),
        |tx| {
            check_folder(tx.conn(), args.parent_id.as_deref())?;
            let now = now_ms();
            let pos = next_position(
                tx.conn(),
                "vault_folder",
                "parent_id IS ?1",
                &[opt_text(args.parent_id.clone())],
            )?;
            tx.conn().execute(
                "INSERT INTO vault_folder(id, name, parent_id, position, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                params![id, name, args.parent_id, pos, now],
            )?;
            Ok(())
        },
    )?;
    Ok(VaultFolderDto {
        id,
        name,
        parent_id: args.parent_id,
        item_count: 0,
        rev: 1,
    })
}

pub(crate) fn rename_folder(
    core: &AppCore,
    actor: &Actor,
    args: VaultRenameFolderArgs,
) -> AppResult<()> {
    let name = required_text(&args.name, "Folder name", MAX_NAME)?;
    mutate_vault(
        core,
        actor,
        args.store,
        MutationMeta::new(
            "vault.rename_folder",
            format!("Renamed folder to “{name}”"),
            Capability::Edit,
        )
        .target("vault_folder", &args.id),
        |tx| {
            check_folder(tx.conn(), Some(&args.id))?;
            update_fields(
                tx.conn(),
                "vault_folder",
                &args.id,
                &[("name", text(name.clone()))],
                &["name"],
                args.expected_rev,
                "folder",
            )?;
            Ok(())
        },
    )
}

/// Deleting a folder never deletes its items: items and sub-folders move up to
/// the folder's parent first (FSD §52.3), then the folder is recoverably deleted.
pub(crate) fn delete_folder(
    core: &AppCore,
    actor: &Actor,
    args: VaultStoreIdArgs,
) -> AppResult<()> {
    let (name, parent): (String, Option<String>) = read_vault(core, actor, args.store, |c, _| {
        c.query_row(
            "SELECT name, parent_id FROM vault_folder WHERE id=?1 AND deleted_at IS NULL",
            [&args.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("folder"))
    })?;
    mutate_vault(
        core,
        actor,
        args.store,
        MutationMeta::new(
            "vault.delete_folder",
            format!("Deleted folder “{name}”"),
            Capability::SoftDelete,
        ),
        |tx| {
            let now = now_ms();
            tx.conn().execute(
                "UPDATE vault_item SET folder_id=?1, updated_at=?2, rev=rev+1 WHERE folder_id=?3 AND deleted_at IS NULL",
                params![parent, now, args.id],
            )?;
            tx.conn().execute(
                "UPDATE vault_folder SET parent_id=?1, updated_at=?2, rev=rev+1 WHERE parent_id=?3 AND deleted_at IS NULL",
                params![parent, now, args.id],
            )?;
            soft_delete(
                tx,
                DeleteSpec {
                    object_type: "vault_folder",
                    table: "vault_folder",
                    id: &args.id,
                    title: Some(name.clone()),
                    parent_type: Some("vault_folder"),
                    parent_id: parent.clone(),
                    position: None,
                },
            )
        },
    )
}

pub(crate) fn create_collection(
    core: &AppCore,
    actor: &Actor,
    args: VaultCreateCollectionArgs,
) -> AppResult<VaultCollectionDto> {
    let name = required_text(&args.name, "Name", MAX_NAME)?;
    let note = optional_text(args.note, "Note", 2_000)?;
    let id = new_id();
    mutate_vault(
        core,
        actor,
        args.store,
        MutationMeta::new(
            "vault.create_collection",
            format!("Created collection “{name}”"),
            Capability::Edit,
        ),
        |tx| {
            let now = now_ms();
            let pos = next_position(tx.conn(), "vault_collection", "1=1", &[])?;
            tx.conn().execute(
                "INSERT INTO vault_collection(id, name, note, position, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                params![id, name, note, pos, now],
            )?;
            Ok(())
        },
    )?;
    Ok(VaultCollectionDto {
        id,
        name,
        note,
        item_count: 0,
        rev: 1,
    })
}

pub(crate) fn rename_collection(
    core: &AppCore,
    actor: &Actor,
    args: VaultRenameCollectionArgs,
) -> AppResult<()> {
    let name = required_text(&args.name, "Name", MAX_NAME)?;
    let note = optional_text(args.note, "Note", 2_000)?;
    mutate_vault(
        core,
        actor,
        args.store,
        MutationMeta::new(
            "vault.rename_collection",
            format!("Renamed collection to “{name}”"),
            Capability::Edit,
        )
        .target("vault_collection", &args.id),
        |tx| {
            super::items::check_collection(tx.conn(), &args.id)?;
            update_fields(
                tx.conn(),
                "vault_collection",
                &args.id,
                &[
                    ("name", text(name.clone())),
                    ("note", opt_text(note.clone())),
                ],
                &["name", "note"],
                args.expected_rev,
                "collection",
            )?;
            Ok(())
        },
    )
}

/// Deleting a collection never deletes its items (membership is a reference).
pub(crate) fn delete_collection(
    core: &AppCore,
    actor: &Actor,
    args: VaultStoreIdArgs,
) -> AppResult<()> {
    let name = read_vault(core, actor, args.store, |c, _| {
        super::items::check_collection(c, &args.id)
    })?;
    mutate_vault(
        core,
        actor,
        args.store,
        MutationMeta::new(
            "vault.delete_collection",
            format!("Deleted collection “{name}”"),
            Capability::SoftDelete,
        ),
        |tx| {
            soft_delete(
                tx,
                DeleteSpec {
                    object_type: "vault_collection",
                    table: "vault_collection",
                    id: &args.id,
                    title: Some(name.clone()),
                    parent_type: None,
                    parent_id: None,
                    position: None,
                },
            )
        },
    )
}
