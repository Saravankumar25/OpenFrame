// Settings > Storage, "Your OpenFrame library": move the whole library, or connect another
// existing library (docs/engineering/40-storage-locations-and-recovery.md section 1). A move copies
// and checks everything before switching and keeps the old folder until the new one works.

import { useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { call } from "../../ipc/client";
import { queryClient } from "../../ipc/query";
import type { LibraryInspection } from "../../ipc/generated/LibraryInspection";
import type { LibraryMoveReport } from "../../ipc/generated/LibraryMoveReport";
import type { LibraryStatus } from "../../ipc/generated/LibraryStatus";
import type { StorageInfo } from "../../ipc/generated/StorageInfo";
import type { StorageTaskStarted } from "../../ipc/generated/StorageTaskStarted";
import { Banner, Button, Dialog } from "../../design-system";
import { toast } from "../../app/toast";
import { joinPath } from "../packages/api";
import { StorageTaskDialog, type StorageAction } from "../storage/StorageTask";
import { messageOf } from "./LibrarySetup";

const folderName = (p: string) => p.split(/[\\/]/).filter(Boolean).pop() ?? p;
const inside = (root: string, p: string) => {
  const key = (s: string) => s.replace(/\\/g, "/").replace(/\/+$/, "").toLowerCase();
  return key(p) === key(root) || key(p).startsWith(key(root) + "/");
};
// Every place changed: ask for everything again.
const refreshAll = () => void queryClient.invalidateQueries();

export function LibraryActions({ info }: { info: StorageInfo }) {
  const [action, setAction] = useState<StorageAction | null>(null);
  const [found, setFound] = useState<LibraryInspection | null>(null);

  const move = async () => {
    const dest = await openDialog({ directory: true, title: "Choose where your OpenFrame library should live" });
    if (typeof dest !== "string") return;
    const target = joinPath(dest, folderName(info.libraryDir));
    const moves = [
      "library.json",
      inside(info.libraryDir, info.vaultDir) && "the Global Idea Vault",
      "the Projects folder and every project in it",
      "anything else in the library folder",
    ].filter(Boolean);
    setAction({
      title: "Move Your OpenFrame Library",
      confirm: (
        <>
          <p style={{ marginTop: 0 }}>
            From {info.libraryDir}
            <br />
            To {target}
          </p>
          <p>This moves {moves.join(", ")}. New projects are then created in the Projects folder of the moved library.</p>
          <p style={{ marginBottom: 0 }}>
            Everything is copied and checked first. The old folder is removed only after OpenFrame works from the new one. Projects you keep
            outside the library stay where they are.
          </p>
        </>
      ),
      confirmLabel: "Move Library",
      start: () => call<StorageTaskStarted>("library.move", { destParent: dest }),
      done: (r) => {
        const rep = r as unknown as LibraryMoveReport | null;
        return (
          <>
            Your library is now at {rep?.path ?? target}.
            {rep?.sourceLeftAt && ` The old folder ${rep.sourceLeftAt} was kept because it still holds files OpenFrame did not copy. Delete it yourself when you no longer need them.`}
          </>
        );
      },
      onFinished: refreshAll,
    });
  };

  const locate = async () => {
    const dir = await openDialog({ directory: true, title: "Choose the folder of an existing OpenFrame library" });
    if (typeof dir !== "string") return;
    try {
      setFound(await call<LibraryInspection>("library.inspect", { dir }));
    } catch (e) {
      toast.error(messageOf(e));
    }
  };

  return (
    <>
      <Button size="sm" onClick={() => void move()}>Move Library…</Button>
      <Button size="sm" onClick={() => void locate()}>Locate Existing Library…</Button>
      {action && <StorageTaskDialog action={action} onClose={() => setAction(null)} />}
      {found && <ConnectLibraryDialog found={found} current={info.libraryDir} onClose={() => setFound(null)} />}
    </>
  );
}

function ConnectLibraryDialog({ found, current, onClose }: { found: LibraryInspection; current: string; onClose: () => void }) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const root = found.libraryRoot;
  const same = !!root && inside(current, root) && inside(root, current);
  const connect = async () => {
    if (!root) return;
    setBusy(true);
    setError(null);
    try {
      await call<LibraryStatus>("library.connect", { dir: root });
      refreshAll();
      toast.success("The library is connected. Nothing was moved or copied.");
      onClose();
    } catch (e) {
      setError(messageOf(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && !busy && onClose()}
      title="Connect This Library?"
      size="md"
      footer={
        <>
          <Button onClick={onClose} disabled={busy}>Cancel</Button>
          {root && !same && (
            <Button variant="primary" disabled={busy} onClick={() => void connect()}>Connect Library</Button>
          )}
        </>
      }
    >
      <div className="sm col" style={{ gap: 8 }}>
        <Banner tone={root ? "info" : "warn"}>
          {found.message}
          {found.ownerName && ` It belongs to ${found.ownerName}.`}
        </Banner>
        {same && <p style={{ margin: 0 }}>This is the library you use now. Nothing to change.</p>}
        {root && !same && (
          <p style={{ margin: 0 }}>
            OpenFrame will use this library from now on. Your current library at {current} is not moved or deleted, and you can connect it again
            later.
          </p>
        )}
        {error && <Banner tone="err">{error}</Banner>}
      </div>
    </Dialog>
  );
}
