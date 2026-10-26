// Global Idea Vault actions in Settings > Storage: open, back up, export a package, restore,
// move and locate. Backup keeps everything for recovery on this computer; an exported package is
// for another computer and leaves out Recently Deleted items and history. Both are .ofvault files
// (docs/engineering/40-storage-locations-and-recovery.md).

import { useState } from "react";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { FolderOpen } from "lucide-react";
import { call, revealLocation } from "../../ipc/client";
import { reportError } from "../../ipc/query";
import type { JsonValue } from "../../ipc/generated/serde_json/JsonValue";
import type { StorageInfo } from "../../ipc/generated/StorageInfo";
import type { StorageTaskStarted } from "../../ipc/generated/StorageTaskStarted";
import type { VaultArchiveReport } from "../../ipc/generated/VaultArchiveReport";
import type { VaultInspection } from "../../ipc/generated/VaultInspection";
import type { VaultMoveReport } from "../../ipc/generated/VaultMoveReport";
import type { VaultRestoreReport } from "../../ipc/generated/VaultRestoreReport";
import { Button } from "../../design-system";
import { toast } from "../../app/toast";
import { formatBytes, joinPath } from "../packages/api";
import { StorageTaskDialog, type StorageAction } from "./StorageTask";
import { refreshStorage } from "./refresh";

const VAULT_FILTER = [{ name: "OpenFrame vault backup or package", extensions: ["ofvault"] }];

const today = () => new Date().toISOString().slice(0, 10);
const folderName = (p: string) => p.split(/[\\/]/).filter(Boolean).pop() ?? p;
const plural = (n: number, one: string, many: string) => `${n.toLocaleString()} ${n === 1 ? one : many}`;

export function VaultActions({ info }: { info: StorageInfo }) {
  const [action, setAction] = useState<StorageAction | null>(null);

  const archive = async (kind: "backup" | "package") => {
    const name = kind === "backup" ? `Global Idea Vault backup ${today()}.ofvault` : `Global Idea Vault ${today()}.ofvault`;
    const path = await saveDialog({
      title: kind === "backup" ? "Save the vault backup to" : "Save the vault package to",
      defaultPath: joinPath(info.libraryDir, name),
      filters: VAULT_FILTER,
    });
    if (typeof path !== "string") return;
    setAction({
      title: kind === "backup" ? "Back Up the Global Idea Vault" : "Export the Global Idea Vault",
      confirm:
        kind === "backup" ? (
          <>A full copy for recovery on this computer: every idea, picture and Recently Deleted item. Saved to {path}.</>
        ) : (
          <>A copy to move your vault to another computer. Recently Deleted items and the change history are left out. Saved to {path}.</>
        ),
      confirmLabel: kind === "backup" ? "Back Up" : "Export",
      start: () => call<StorageTaskStarted>(kind === "backup" ? "vault.backup" : "vault.export_package", { path }),
      done: (r: JsonValue | null) => {
        const rep = r as unknown as VaultArchiveReport | null;
        if (!rep) return "Saved.";
        return (
          <>
            Saved {rep.fileName} ({formatBytes(rep.bytes)}): {plural(rep.ideaCount, "item", "items")} and {plural(rep.assetCount, "file", "files")}.
            {rep.missingAssets > 0 && ` ${plural(rep.missingAssets, "file was", "files were")} already missing from the vault and could not be included.`}
          </>
        );
      },
    });
  };

  const restore = async () => {
    const path = await openDialog({ title: "Choose a vault backup or package", filters: VAULT_FILTER, multiple: false, directory: false });
    if (typeof path !== "string") return;
    let v: VaultInspection;
    try {
      v = await call<VaultInspection>("vault.inspect", { path });
    } catch (e) {
      return reportError(e);
    }
    setAction({
      title: "Restore the Global Idea Vault",
      confirm: (
        <>
          <p style={{ marginTop: 0 }}>
            {v.fileName} is a {v.kind === "backup" ? "backup" : "package"} from {new Date(v.createdAt).toLocaleString()} with{" "}
            {plural(v.ideaCount, "item", "items")} and {plural(v.assetCount, "file", "files")}.
          </p>
          <p style={{ marginBottom: 0 }}>
            It replaces your current vault. Your current vault is not deleted: it is kept beside it in a folder named
            &quot;(before restore)&quot;, so you can go back.
          </p>
        </>
      ),
      confirmLabel: "Replace and Restore",
      start: () => call<StorageTaskStarted>("vault.restore_backup", { path }),
      done: (r) => {
        const rep = r as unknown as VaultRestoreReport | null;
        return rep?.previousKeptAt ? <>Restored. Your previous vault is kept at {rep.previousKeptAt}.</> : <>Restored.</>;
      },
      onFinished: refreshStorage,
    });
  };

  const move = async () => {
    const dest = await openDialog({ directory: true, title: "Choose where the Global Idea Vault should live" });
    if (typeof dest !== "string") return;
    const target = joinPath(dest, folderName(info.vaultDir));
    setAction({
      title: "Move the Global Idea Vault",
      confirm: (
        <>
          The vault moves to {target}. It is copied and checked first; only then is the old copy removed. If anything goes wrong, it stays where
          it is now.
        </>
      ),
      confirmLabel: "Move Vault",
      start: () => call<StorageTaskStarted>("vault.move", { destParent: dest }),
      done: (r) => {
        const rep = r as unknown as VaultMoveReport | null;
        return (
          <>
            The vault is now at {rep?.path ?? target}.
            {rep?.sourceLeftAt && ` Some files could not be removed from ${rep.sourceLeftAt}; you can delete that folder yourself.`}
          </>
        );
      },
      onFinished: refreshStorage,
    });
  };

  const locate = async () => {
    const dir = await openDialog({ directory: true, title: "Choose the folder of an existing Global Idea Vault" });
    if (typeof dir !== "string") return;
    try {
      await call<StorageInfo>("vault.locate", { dir });
      refreshStorage();
      toast.success("The Global Idea Vault is connected. Nothing was moved or copied.");
    } catch (e) {
      reportError(e);
    }
  };

  return (
    <>
      <Button size="sm" icon={<FolderOpen size={14} />} disabled={!info.vaultExists} onClick={() => void revealLocation("globalVault").catch(reportError)}>
        Open Folder
      </Button>
      <Button size="sm" disabled={!info.vaultExists} onClick={() => void archive("backup")}>Back Up…</Button>
      <Button size="sm" disabled={!info.vaultExists} onClick={() => void archive("package")}>Export Package…</Button>
      <Button size="sm" onClick={() => void restore()}>Restore…</Button>
      <Button size="sm" disabled={!info.vaultExists} onClick={() => void move()}>Move…</Button>
      <Button size="sm" onClick={() => void locate()}>Locate Existing Vault…</Button>
      {action && <StorageTaskDialog action={action} onClose={() => setAction(null)} />}
    </>
  );
}
