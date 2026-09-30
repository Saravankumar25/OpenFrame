// Typed wrappers over the `vault.*` application operations.

import { call } from "../../ipc/client";
import { useOp } from "../../ipc/query";
import type { VaultAddFilesArgs } from "../../ipc/generated/VaultAddFilesArgs";
import type { VaultAddFilesResult } from "../../ipc/generated/VaultAddFilesResult";
import type { VaultCopyArgs } from "../../ipc/generated/VaultCopyArgs";
import type { VaultCopyResult } from "../../ipc/generated/VaultCopyResult";
import type { VaultCreateArgs } from "../../ipc/generated/VaultCreateArgs";
import type { VaultIngestBytesArgs } from "../../ipc/generated/VaultIngestBytesArgs";
import type { VaultSendToStoryArgs } from "../../ipc/generated/VaultSendToStoryArgs";
import type { VaultSendToStoryResult } from "../../ipc/generated/VaultSendToStoryResult";
import type { VaultStoryTargets } from "../../ipc/generated/VaultStoryTargets";
import type { VaultUpdateArgs } from "../../ipc/generated/VaultUpdateArgs";
import type { VaultCollectionDto } from "../../ipc/generated/VaultCollectionDto";
import type { VaultFolderDto } from "../../ipc/generated/VaultFolderDto";
import type { VaultItemDto } from "../../ipc/generated/VaultItemDto";
import type { VaultOverview } from "../../ipc/generated/VaultOverview";
import type { VaultRestoreResult } from "../../ipc/generated/VaultRestoreResult";
import type { VaultTrashRow } from "../../ipc/generated/VaultTrashRow";
import type { VaultView } from "../../ipc/generated/VaultView";
import { VAULT_TABLES, type Scope } from "./model";

/** Generated arg types list every optional field; callers pass only what they set. */
type Args<T, K extends keyof T> = Pick<T, K> & Partial<Omit<T, K>>;

export function useVaultOverview(scope: Scope) {
  return useOp<VaultOverview>("vault.overview", { store: scope }, VAULT_TABLES, { store: scope });
}

export function useVaultList(scope: Scope, view: VaultView, search: string) {
  return useOp<VaultItemDto[]>(
    "vault.list",
    { store: scope, view, search: search.trim() || null },
    VAULT_TABLES,
    { store: scope, placeholderData: (prev) => prev },
  );
}

export function useVaultItem(scope: Scope, id: string | null) {
  return useOp<VaultItemDto>("vault.get", { store: scope, id: id ?? "" }, VAULT_TABLES, { store: scope, enabled: !!id });
}

export function useVaultTrash(scope: Scope) {
  return useOp<VaultTrashRow[]>("vault.trash_list", { store: scope }, ["deleted_item", ...VAULT_TABLES], { store: scope });
}

export function useStoryTargets(enabled: boolean) {
  return useOp<VaultStoryTargets>("vault.story_targets", {}, ["story_act", "story_sequence"], { enabled });
}

export const vault = {
  create: (a: Args<VaultCreateArgs, "store" | "itemType">) => call<VaultItemDto>("vault.create", a),
  addFiles: (a: Args<VaultAddFilesArgs, "store" | "paths">) => call<VaultAddFilesResult>("vault.add_files", a),
  ingest: (a: Args<VaultIngestBytesArgs, "store" | "itemType" | "fileName" | "dataBase64">) => call<VaultItemDto>("vault.ingest_bytes", a),
  update: (a: Args<VaultUpdateArgs, "store" | "id">) => call<VaultItemDto>("vault.update", a),
  setPinned: (store: Scope, ids: string[], pinned: boolean) => call<void>("vault.set_pinned", { store, ids, pinned }),
  moveToFolder: (store: Scope, ids: string[], folderId: string | null) => call<void>("vault.move_to_folder", { store, ids, folderId }),
  addToCollection: (store: Scope, ids: string[], collectionId: string) => call<void>("vault.add_to_collection", { store, ids, collectionId }),
  removeFromCollection: (store: Scope, ids: string[], collectionId: string) =>
    call<void>("vault.remove_from_collection", { store, ids, collectionId }),
  addTags: (store: Scope, ids: string[], tags: string[]) => call<void>("vault.add_tags", { store, ids, tags }),
  removeTag: (store: Scope, ids: string[], tag: string) => call<void>("vault.remove_tag", { store, ids, tag }),
  remove: (store: Scope, ids: string[]) => call<void>("vault.delete", { store, ids }),
  relink: (store: Scope, id: string, path: string) => call<VaultItemDto>("vault.relink", { store, id, path }),
  createFolder: (store: Scope, name: string, parentId: string | null) => call<VaultFolderDto>("vault.create_folder", { store, name, parentId }),
  renameFolder: (store: Scope, id: string, name: string) => call<void>("vault.rename_folder", { store, id, name }),
  deleteFolder: (store: Scope, id: string) => call<void>("vault.delete_folder", { store, id }),
  createCollection: (store: Scope, name: string) => call<VaultCollectionDto>("vault.create_collection", { store, name }),
  renameCollection: (store: Scope, id: string, name: string) => call<void>("vault.rename_collection", { store, id, name }),
  deleteCollection: (store: Scope, id: string) => call<void>("vault.delete_collection", { store, id }),
  copy: (a: Args<VaultCopyArgs, "from" | "to" | "ids">) => call<VaultCopyResult>("vault.copy_to_store", a),
  sendToStory: (a: Args<VaultSendToStoryArgs, "store" | "id" | "target">) => call<VaultSendToStoryResult>("vault.send_to_story", a),
  restore: (store: Scope, id: string) => call<VaultRestoreResult>("vault.restore", { store, id }),
  purge: (store: Scope, id: string) => call<void>("vault.purge", { store, id }),
};
