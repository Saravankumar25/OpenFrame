// Global Idea Vault actions in Settings > Storage: open, back up, export a package, restore,
// move, locate, and add back files that are in the vault folder but in no item. Locate reads the
// chosen folder first and says what it found before anything changes. Backup keeps everything for recovery on this computer; an exported package is
// for another computer and leaves out Recently Deleted items and history. Both are .ofvault files
// (docs/engineering/40-storage-locations-and-recovery.md).

import { useState } from "react";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { FolderOpen } from "lucide-react";
import { call, revealLocation } from "../../ipc/client";
import { queryClient, reportError, useOp } from "../../ipc/query";
import type { JsonValue } from "../../ipc/generated/serde_json/JsonValue";
import type { StorageInfo } from "../../ipc/generated/StorageInfo";
import type { StorageTaskStarted } from "../../ipc/generated/StorageTaskStarted";
import type { VaultArchiveReport } from "../../ipc/generated/VaultArchiveReport";
import type { VaultFolderInspection } from "../../ipc/generated/VaultFolderInspection";
import type { VaultOrphanFiles } from "../../ipc/generated/VaultOrphanFiles";
import type { VaultRecoverResult } from "../../ipc/generated/VaultRecoverResult";
import type { VaultInspection } from "../../ipc/generated/VaultInspection";
import type { VaultMoveReport } from "../../ipc/generated/VaultMoveReport";
import type { VaultRestoreReport } from "../../ipc/generated/VaultRestoreReport";
import { Banner, Button, Dialog } from "../../design-system";
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
  const [found, setFound] = useState<VaultFolderInspection | null>(null);
  const [recovering, setRecovering] = useState(false);
  const orphans = useOp<VaultOrphanFiles>("vault.orphan_files", { store: "global" }, ["vault_item", "asset"], {
    store: "global",
    enabled: info.vaultExists,
  });
  const orphanCount = orphans.data?.count ?? 0;

  const recover = async () => {
    setRecovering(true);
    try {
      const r = await call<VaultRecoverResult>("vault.recover_files", { store: "global" });
      if (r.added > 0) toast.success(`Added ${plural(r.added, "file", "files")} back to the Global Idea Vault.`);
      if (r.failed > 0) toast.error(`${plural(r.failed, "file", "files")} could not be read. They may be in use by another program. Try again later.`);
      void orphans.refetch();
    } catch (e) {
      reportError(e);
    } finally {
      setRecovering(false);
    }
  };

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
            {rep?.sourceLeftAt && ` The old folder ${rep.sourceLeftAt} was kept: it holds your safety backups, or files that could not be removed. Delete it yourself when you no longer need them.`}
          </>
        );
      },
      onFinished: refreshStorage,
    });
  };

  const locate = async () => {
    const dir = await openDialog({ directory: true, title: "Choose the folder of an existing Global Idea Vault" });
    if (typeof dir !== "string") return;
    let v: VaultFolderInspection;
    try {
      v = await call<VaultFolderInspection>("vault.inspect_folder", { dir });
    } catch (e) {
      return reportError(e);
    }
    if (v.kind === "project") {
      toast.error("That folder is a project, not a vault. Choose the folder that holds your Global Idea Vault.");
    } else if (v.kind === "nothing" || !v.vaultDir) {
      toast.error("There is no Global Idea Vault in that folder. Choose the vault folder, or the OpenFrame folder that holds it.");
    } else if (v.active) {
      toast.info("This vault is already the one in use. Nothing was changed.");
    } else {
      setFound(v);
    }
  };

  const connect = async (v: VaultFolderInspection) => {
    try {
      await call<StorageInfo>("vault.locate", { dir: v.vaultDir });
      setFound(null);
      refreshStorage();
      void queryClient.invalidateQueries({ predicate: (q) => String(q.queryKey[0]).startsWith("vault.") });
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
      {orphanCount > 0 && (
        <div className="row sm" style={{ flexBasis: "100%", gap: 8, marginTop: 4 }}>
          <span>
            {plural(orphanCount, "file", "files")} in the vault folder {orphanCount === 1 ? "is" : "are"} not in the vault (
            {formatBytes(orphans.data?.bytes ?? 0)}).
          </span>
          <Button size="sm" variant="primary" disabled={recovering} onClick={() => void recover()}>
            Add Them Back
          </Button>
        </div>
      )}
      {action && <StorageTaskDialog action={action} onClose={() => setAction(null)} />}
      {found && <LocateDialog found={found} onConnect={() => connect(found)} onClose={() => setFound(null)} />}
    </>
  );
}

/** What Locate found, confirmed before the vault is connected. An empty vault needs a second yes. */
function LocateDialog({ found, onConnect, onClose }: { found: VaultFolderInspection; onConnect: () => Promise<void>; onClose: () => void }) {
  const [sureEmpty, setSureEmpty] = useState(false);
  const [busy, setBusy] = useState(false);
  const empty = found.ideas === 0;
  const files = found.filesPresent + found.filesMissing;
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && !busy && onClose()}
      title="Connect This Vault?"
      size="md"
      footer={
        <>
          <Button onClick={onClose} disabled={busy}>Cancel</Button>
          <Button
            variant="primary"
            disabled={busy || (empty && !sureEmpty)}
            onClick={() => {
              setBusy(true);
              void onConnect().finally(() => setBusy(false));
            }}
          >
            {empty ? "Connect Empty Vault" : "Connect Vault"}
          </Button>
        </>
      }
    >
      <div className="sm">
        <p style={{ marginTop: 0 }}>
          Found a vault at {found.vaultDir} with {plural(found.ideas, "idea", "ideas")} and {plural(files, "file", "files")}.
          {found.filesMissing > 0 && ` ${plural(found.filesMissing, "file is", "files are")} missing from its folder.`}
        </p>
        <p style={{ marginBottom: empty ? undefined : 0 }}>
          OpenFrame will use this vault from now on. Your current vault is not moved or deleted, and you can connect it again later.
        </p>
        {empty && (
          <Banner tone="warn">
            <label className="row" style={{ gap: 8 }}>
              <input type="checkbox" checked={sureEmpty} onChange={(e) => setSureEmpty(e.target.checked)} />
              This vault has no ideas. Connect it anyway.
            </label>
          </Banner>
        )}
      </div>
    </Dialog>
  );
}
