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
    use crate::registry::{FsEffect as Fs, OperationMetadata as M, hidden as h};
    use openframe_domain::Capability;
    r.module("Idea Vault");
    r.query("vault.overview", model::overview).meta(M::compute("Idea Vault overview: item counts, folders, collections and tags (project or Global Idea Vault)."));
    r.query("vault.list", model::list).meta(M::search("Idea Vault items in a view (all, folder, collection, tag, type, pinned) with optional search."));
    r.query("vault.get", model::get).meta(M::read(
        "One Idea Vault item with its text, tags, folder and collections.",
    ));

    r.command("vault.create", items::create).meta(M::edit(
        "Create a note, quote or link item in the Idea Vault.",
    ));
    r.command("vault.add_files", items::add_files).meta(
        M::edit("Add files the user picked to the Idea Vault.")
            .fs(Fs::ReadsUserFile)
            .hidden(h::USER_PATH),
    );
    r.command("vault.ingest_bytes", items::ingest).meta(
        M::edit("Store a recording, sketch or pasted image in the Idea Vault.")
            .hidden(h::MEDIA_INPUT),
    );
    r.command("vault.update", items::update).meta(M::edit(
        "Edit an Idea Vault item (title, text, caption, link, source).",
    ));
    r.command("vault.set_pinned", items::set_pinned)
        .meta(M::edit("Pin or unpin Idea Vault items."));
    r.command("vault.move_to_folder", items::move_to_folder)
        .meta(M::edit(
            "Move Idea Vault items to a folder (or the top level).",
        ));
    r.command("vault.add_to_collection", items::add_to_collection)
        .meta(M::edit("Add Idea Vault items to a collection."));
    r.command(
        "vault.remove_from_collection",
        items::remove_from_collection,
    )
    .meta(M::edit("Remove Idea Vault items from a collection."));
    r.command("vault.add_tags", items::add_tags)
        .meta(M::edit("Tag Idea Vault items."));
    r.command("vault.remove_tag", items::remove_tag)
        .meta(M::edit("Remove a tag from Idea Vault items."));
    r.command("vault.delete", items::delete)
        .meta(M::soft_delete("Move Idea Vault items to Recently Deleted."));
    r.command("vault.relink", items::relink).meta(
        M::edit("Relink a missing linked Idea Vault file.")
            .fs(Fs::ReadsUserFile)
            .hidden(h::USER_PATH),
    );

    r.command("vault.create_folder", organize::create_folder)
        .meta(M::edit("Create an Idea Vault folder."));
    r.command("vault.rename_folder", organize::rename_folder)
        .meta(M::edit("Rename an Idea Vault folder."));
    r.command("vault.delete_folder", organize::delete_folder)
        .meta(M::soft_delete(
            "Delete an Idea Vault folder (items move to the top level).",
        ));
    r.command("vault.create_collection", organize::create_collection)
        .meta(M::edit("Create an Idea Vault collection."));
    r.command("vault.rename_collection", organize::rename_collection)
        .meta(M::edit("Rename an Idea Vault collection or edit its note."));
    r.command("vault.delete_collection", organize::delete_collection)
        .meta(M::soft_delete(
            "Delete an Idea Vault collection (items are kept).",
        ));

    r.command("vault.copy_to_store", transfer::copy_to_store)
        .meta(
            M::edit("Copy items between the project Idea Vault and the Global Idea Vault.")
                .hidden(h::CROSS_PROJECT),
        );
    r.query("vault.story_targets", transfer::story_targets)
        .meta(
            M::read("Acts and sequences an Idea Vault item can be sent to (dialog helper).")
                .hidden(h::UI_FLOW),
        );
    r.command("vault.send_to_story", transfer::send_to_story).meta(M::edit("Send an Idea Vault item to Story as a beat, Scene Card, sequence or character (a copy)."));

    r.query("vault.trash_list", trash::list).meta(
        M::read("Deleted Idea Vault items (project or Global Idea Vault).").hidden(h::DUPLICATE),
    );
    r.command("vault.restore", trash::restore).meta(
        M::command(Capability::SoftDelete, "Restore a deleted Idea Vault item.")
            .hidden(h::DUPLICATE),
    );
    r.command("vault.purge", trash::purge).meta(
        M::command(
            Capability::PermanentDelete,
            "Delete an Idea Vault item forever.",
        )
        .destructive()
        .irreversible()
        .confirm()
        .hidden(h::DELETE_FOREVER),
    );

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
