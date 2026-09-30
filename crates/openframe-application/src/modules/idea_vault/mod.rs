//! Idea Vault — Global and Project (FSD §5, §88; FSD-IDEA-001..020).
//!
//! One module serves two independent stores with identical item tables:
//! the Project Idea Vault (inside the open project) and the Global Idea Vault
//! (Documents/OpenFrame/Global Idea Vault, available with no project open).
//! Every operation takes `store: "project" | "global"`.
//!
//! Source-of-truth rules (FSD §5.8, §5.10, §88.9; Domain §9):
//! * Copies between stores create independent items with new identities; no sync.
//! * "Send to Story" creates a new Story object; the Vault original is untouched.
//! * Metadata is always optional; an untitled item is valid and generated display
//!   names are computed on read, never stored as the title.

use std::path::{Path, PathBuf};

use openframe_domain::auth::ActorOrigin;
use openframe_domain::{Actor, AppError, AppResult, Capability, Role};
use rusqlite::Connection;

use crate::core::AppCore;
use crate::registry::{Registry, TrashHandler};
use crate::store::{MutationMeta, Tx};
use crate::util::{StoreSel, with_store};

mod items;
mod model;
mod organize;
mod search;
mod transfer;
mod trash;

pub use items::{
    VaultAddFilesArgs, VaultAddFilesResult, VaultCreateArgs, VaultFailedFile, VaultIdsArgs,
    VaultIngestBytesArgs, VaultItemIdArgs, VaultMembershipArgs, VaultMoveArgs, VaultRelinkArgs,
    VaultRemoveTagArgs, VaultSetPinnedArgs, VaultTagsArgs, VaultUpdateArgs,
};
pub use model::{
    VaultCollectionDto, VaultFolderDto, VaultItemDto, VaultListArgs, VaultOverview,
    VaultOverviewArgs, VaultTagCount, VaultView,
};
pub use organize::{
    VaultCreateCollectionArgs, VaultCreateFolderArgs, VaultRenameCollectionArgs,
    VaultRenameFolderArgs, VaultStoreIdArgs,
};
pub use transfer::{
    VaultCopyArgs, VaultCopyResult, VaultSendToStoryArgs, VaultSendToStoryResult, VaultStoryAct,
    VaultStorySequence, VaultStoryTarget, VaultStoryTargets, VaultStoryTargetsArgs,
};
pub use trash::{VaultRestoreResult, VaultTrashArgs, VaultTrashItemArgs, VaultTrashRow};

pub fn register(r: &mut Registry) {
    r.query("vault.overview", model::overview);
    r.query("vault.list", model::list);
    r.query("vault.get", model::get);

    r.command("vault.create", items::create);
    r.command("vault.add_files", items::add_files);
    r.command("vault.ingest_bytes", items::ingest);
    r.command("vault.update", items::update);
    r.command("vault.set_pinned", items::set_pinned);
    r.command("vault.move_to_folder", items::move_to_folder);
    r.command("vault.add_to_collection", items::add_to_collection);
    r.command(
        "vault.remove_from_collection",
        items::remove_from_collection,
    );
    r.command("vault.add_tags", items::add_tags);
    r.command("vault.remove_tag", items::remove_tag);
    r.command("vault.delete", items::delete);
    r.command("vault.relink", items::relink);

    r.command("vault.create_folder", organize::create_folder);
    r.command("vault.rename_folder", organize::rename_folder);
    r.command("vault.delete_folder", organize::delete_folder);
    r.command("vault.create_collection", organize::create_collection);
    r.command("vault.rename_collection", organize::rename_collection);
    r.command("vault.delete_collection", organize::delete_collection);

    r.command("vault.copy_to_store", transfer::copy_to_store);
    r.query("vault.story_targets", transfer::story_targets);
    r.command("vault.send_to_story", transfer::send_to_story);

    r.query("vault.trash_list", trash::list);
    r.command("vault.restore", trash::restore);
    r.command("vault.purge", trash::purge);

    r.indexer("vault_item", search::index_item);

    r.trash_handler(TrashHandler {
        object_type: "vault_item",
        table: "vault_item",
        label: "Idea Vault item",
        restore: Some(trash::restore_item),
        purge: trash::purge_item,
    });
    r.trash_handler(TrashHandler {
        object_type: "vault_folder",
        table: "vault_folder",
        label: "Idea Vault folder",
        restore: Some(trash::restore_folder),
        purge: trash::purge_folder,
    });
    r.trash_handler(TrashHandler {
        object_type: "vault_collection",
        table: "vault_collection",
        label: "Idea Vault collection",
        restore: None,
        purge: trash::purge_collection,
    });
}

/// The actor used against a vault store.
///
/// The Project Idea Vault follows the project's roles (a Viewer cannot add).
/// The Global Idea Vault belongs to the filmmaker on this computer (Domain §4:
/// "filmmaker-owned, cross-project"), so the local user always owns it whatever
/// their role in the currently open project, while changes arriving from another
/// person's exchange package can never reach it (project boundaries stay intact).
pub(crate) fn vault_actor(actor: &Actor, store: StoreSel) -> AppResult<Actor> {
    match store {
        StoreSel::Project => Ok(actor.clone()),
        StoreSel::Global => match actor.origin {
            ActorOrigin::Local => Ok(Actor {
                role: Role::Owner,
                ..actor.clone()
            }),
            ActorOrigin::Exchange { .. } => Err(AppError::permission_denied(
                "use this computer's Global Idea Vault",
            )),
            ActorOrigin::Ai { .. } => Ok(actor.clone()),
        },
    }
}

/// Read from a vault store (requires View on it).
pub(crate) fn read_vault<R>(
    core: &AppCore,
    actor: &Actor,
    store: StoreSel,
    f: impl FnOnce(&Connection, &Path) -> AppResult<R>,
) -> AppResult<R> {
    vault_actor(actor, store)?.require(Capability::View, "view the Idea Vault")?;
    with_store(core, store, |s| {
        let root: PathBuf = s.root().to_path_buf();
        s.read(|c| f(c, &root))
    })
}

/// Mutate a vault store through the standard pipeline.
pub(crate) fn mutate_vault<R>(
    core: &AppCore,
    actor: &Actor,
    store: StoreSel,
    meta: MutationMeta,
    f: impl FnOnce(&Tx<'_>) -> AppResult<R>,
) -> AppResult<R> {
    let a = vault_actor(actor, store)?;
    with_store(core, store, |s| s.mutate(&a, meta, f))
}

/// Human name of a store for messages.
pub(crate) fn store_label(store: StoreSel) -> &'static str {
    match store {
        StoreSel::Project => "Project Idea Vault",
        StoreSel::Global => "Global Idea Vault",
    }
}
