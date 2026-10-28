// The open project's place in Settings > Storage: open its folder, move it, and take over the
// private notes of a former owner (for example notes written under an earlier identity on this
// computer). Re-owning is explicit, one owner at a time, Owner only, and recorded in Activity
// (ADR-0038).

import { useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { FolderOpen } from "lucide-react";
import { call, revealLocation } from "../../ipc/client";
import { queryClient, reportError, useOp } from "../../ipc/query";
import type { AdoptedNotes } from "../../ipc/generated/AdoptedNotes";
import type { OrphanNoteOwner } from "../../ipc/generated/OrphanNoteOwner";
import type { ProjectMoveReport } from "../../ipc/generated/ProjectMoveReport";
import type { StorageTaskStarted } from "../../ipc/generated/StorageTaskStarted";
import { Button, ConfirmDialog } from "../../design-system";
import { toast } from "../../app/toast";
import { joinPath } from "../packages/api";
import { StorageTaskDialog, type StorageAction } from "./StorageTask";
import { refreshStorage } from "./refresh";

const folderName = (p: string) => p.split(/[\\/]/).filter(Boolean).pop() ?? p;

export function ProjectStorage({ path }: { path: string }) {
  const [action, setAction] = useState<StorageAction | null>(null);
  const [adopting, setAdopting] = useState<OrphanNoteOwner | null>(null);
  const [busy, setBusy] = useState(false);
  // Owner only; other roles get a permission error and the section stays hidden.
  const orphans = useOp<OrphanNoteOwner[]>("project.orphan_notes", {}, ["private_note", "project_member"], { retry: false });
  const owners = Array.isArray(orphans.data) ? orphans.data : [];

  const move = async () => {
    const dest = await openDialog({ directory: true, title: "Choose where this project should live" });
    if (typeof dest !== "string") return;
    const target = joinPath(dest, folderName(path));
    setAction({
      title: "Move This Project",
      confirm: (
        <>
          The project folder moves to {target}. The project closes for a moment, is copied and checked, then opens from the new place; only then is
          the old folder removed. If anything goes wrong, it stays where it is now. Linked files outside the project are not moved.
        </>
      ),
      confirmLabel: "Move Project",
      start: () => call<StorageTaskStarted>("project.move", { destParent: dest }),
      done: (r) => {
        const rep = r as unknown as ProjectMoveReport | null;
        return (
          <>
            The project is now at {rep?.path ?? target}.
            {rep && rep.linkedFiles > 0 && ` ${rep.linkedFiles} linked ${rep.linkedFiles === 1 ? "file stays" : "files stay"} where ${rep.linkedFiles === 1 ? "it was" : "they were"}.`}
            {rep?.sourceLeftAt && ` Some files could not be removed from ${rep.sourceLeftAt}; you can delete that folder yourself.`}
          </>
        );
      },
      onFinished: refreshStorage,
    });
  };

  const adopt = async () => {
    if (!adopting) return;
    setBusy(true);
    try {
      const r = await call<AdoptedNotes>("project.adopt_orphan_notes", { ownerUserId: adopting.ownerUserId });
      toast.success(r.adopted === 1 ? "1 private note is yours now." : `${r.adopted} private notes are yours now.`);
      void queryClient.invalidateQueries({ queryKey: ["project.orphan_notes"] });
      setAdopting(null);
    } catch (e) {
      reportError(e);
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="card pad" aria-label="This project" style={{ marginBottom: 12 }}>
      <div className="h4" style={{ margin: "0 0 4px" }}>This project</div>
      <div className="sm selectable-text" style={{ wordBreak: "break-all", marginBottom: 4 }}>{path}</div>
      <div className="xs muted" style={{ marginBottom: 8 }}>
        Everything in this project lives in this folder. Moving it keeps its history, members and private notes.
      </div>
      <div className="row" style={{ gap: 6, flexWrap: "wrap" }}>
        <Button size="sm" icon={<FolderOpen size={14} />} onClick={() => void revealLocation("project").catch(reportError)}>Open Folder</Button>
        <Button size="sm" onClick={() => void move()}>Move Project…</Button>
      </div>
      {owners.length > 0 && (
        <div style={{ marginTop: 12 }}>
          <div className="sm" style={{ fontWeight: 600 }}>Private notes of a former owner</div>
          <div className="xs muted" style={{ marginBottom: 6 }}>
            These private notes belong to someone who is no longer a member of this project, often you under an earlier setup of OpenFrame. Only
            take them over if they are yours.
          </div>
          {owners.map((o) => (
            <div key={o.ownerUserId} className="row" style={{ gap: 8, alignItems: "center", marginBottom: 4 }}>
              <span className="sm grow">{o.noteCount === 1 ? "1 private note" : `${o.noteCount} private notes`}</span>
              <Button size="sm" onClick={() => setAdopting(o)}>Make Them Mine…</Button>
            </div>
          ))}
        </div>
      )}
      {action && <StorageTaskDialog action={action} onClose={() => setAction(null)} />}
      <ConfirmDialog
        open={!!adopting}
        onOpenChange={(v) => !v && setAdopting(null)}
        title="Take over these private notes?"
        confirmLabel="Make Them Mine"
        busy={busy}
        onConfirm={() => void adopt()}
      >
        {adopting && (
          <>
            {adopting.noteCount === 1 ? "1 private note" : `${adopting.noteCount} private notes`} will become yours and visible only to you. This is
            recorded in Activity and cannot be undone.
          </>
        )}
      </ConfirmDialog>
    </section>
  );
}
