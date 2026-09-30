// Crash recovery (FSD §44.3–44.4; UX §2.13; mocks 016, 017, 019).
//
// * RecoveryOfferDialog — shown after opening a project that was not closed
//   normally. Not dismissible: the user must choose a version explicitly
//   ("Do not silently select one version").
//     Keep Saved Version → project.resolve_recovery { choice: "checkpoint" }
//     Open Recovery      → project.resolve_recovery { choice: "latest" }
// * OpenRecoveryDialog — "Open recovery" from the status panel / save-error
//   banner: return to the last confirmed saved version on purpose.

import { useState } from "react";
import { call } from "../../ipc/client";
import { queryClient, reportError, useOp } from "../../ipc/query";
import type { ProjectSummary } from "../../ipc/generated/ProjectSummary";
import type { RecoveryState } from "../../ipc/generated/RecoveryState";
import { Banner, Button, Dialog } from "../../design-system";
import { useNav } from "../stores";
import { toast } from "../toast";

/** "Today, 10:37", "Yesterday, 23:58", "12 Sep 2026, 10:02". */
export function whenLong(ms: number | null | undefined, now: number = Date.now()): string {
  if (ms == null) return "Unknown time";
  const d = new Date(ms);
  const t = d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  const start = new Date(now);
  start.setHours(0, 0, 0, 0);
  if (ms >= start.getTime()) return `Today, ${t}`;
  if (ms >= start.getTime() - 86_400_000) return `Yesterday, ${t}`;
  return `${d.toLocaleDateString([], { day: "numeric", month: "short", year: "numeric" })}, ${t}`;
}

async function resolve(choice: "latest" | "checkpoint"): Promise<void> {
  await call("project.resolve_recovery", { choice });
  if (choice === "checkpoint") useNav.getState().reset();
  // "latest" changes no project rows, so refresh the recovery state explicitly.
  await queryClient.invalidateQueries();
}

/** Recovery/checkpoint state of the open project (FSD §44). */
export function useRecoveryState(enabled: boolean) {
  return useOp<RecoveryState>("project.recovery_state", {}, ["project"], { enabled });
}

/** Mounted by the shell; renders only while a recovery offer is pending. */
export function RecoveryOffer({ project, state: s }: { project: ProjectSummary | null; state: RecoveryState | undefined }) {
  const [busy, setBusy] = useState<"latest" | "checkpoint" | null>(null);
  if (!project || !s?.offer) return null;
  const choose = async (choice: "latest" | "checkpoint") => {
    setBusy(choice);
    try {
      await resolve(choice);
      toast.success(choice === "latest" ? "Opened the recovery state. Your latest work is here." : "Opened the last saved version. The newer recovery state was kept as a safety copy in the project's backups folder.");
    } catch (e) {
      reportError(e);
    } finally {
      setBusy(null);
    }
  };
  const newer = s.changesSinceCheckpoint;
  return (
    <Dialog
      open
      dismissible={false}
      onOpenChange={() => undefined}
      title="We found a recent recovery state for this project."
      size="md"
      footer={
        <>
          <Button disabled={!s.hasCheckpoint || !!busy} onClick={() => void choose("checkpoint")} title={s.hasCheckpoint ? undefined : "There is no confirmed saved version to return to."}>
            {busy === "checkpoint" ? "Opening…" : "Keep Saved Version"}
          </Button>
          <Button variant="primary" disabled={!!busy} autoFocus onClick={() => void choose("latest")}>
            {busy === "latest" ? "Opening…" : "Open Recovery"}
          </Button>
        </>
      }
    >
      <p style={{ marginTop: 0 }}>
        <b>{project.title}</b> was not closed normally last time. OpenFrame kept a more recent automatic recovery state than your last confirmed save.
      </p>
      <div className="grid g2">
        <div className="card pad">
          <div className="h4">Last saved version</div>
          {s.hasCheckpoint ? <b>{whenLong(s.checkpointAt)}</b> : <b>No confirmed save yet</b>}
          <div className="xs muted">{s.hasCheckpoint ? "Your last confirmed save" : "Nothing to return to"}</div>
        </div>
        <div className="card pad">
          <div className="h4">Recovery state</div>
          <b>{whenLong(s.offer.lastAutosaveAt ?? s.lastChangeAt)}</b>
          <div className="xs muted">
            {newer != null && newer > 0 ? `${newer} change${newer === 1 ? "" : "s"} newer` : "Latest automatic save"}
          </div>
        </div>
      </div>
      <p className="muted sm" style={{ marginBottom: 0 }}>Nothing has been replaced. You choose which one to open.</p>
    </Dialog>
  );
}

/** "Open recovery": return to the last confirmed saved version on purpose. */
export function OpenRecoveryDialog({ onClose }: { onClose: () => void }) {
  const state = useOp<RecoveryState>("project.recovery_state", {}, ["project"], { refetchOnMount: "always" });
  const [busy, setBusy] = useState(false);
  const s = state.data;
  const open = async () => {
    setBusy(true);
    try {
      await resolve("checkpoint");
      toast.success("Opened the last saved version. The state you left was kept as a safety copy in the project's backups folder.");
      onClose();
    } catch (e) {
      reportError(e);
      setBusy(false);
    }
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Open recovery"
      sub="Go back to the last version you confirmed with Save."
      size="md"
      footer={
        <>
          <Button onClick={onClose}>Keep Current Work</Button>
          <Button variant="primary" disabled={!s?.hasCheckpoint || busy} onClick={() => void open()}>
            {busy ? "Opening…" : "Open Saved Version"}
          </Button>
        </>
      }
    >
      {s && !s.hasCheckpoint ? (
        <Banner tone="info">There is no confirmed saved version yet. Your work saves continuously; use <b>Save</b> (Ctrl+S) to create one.</Banner>
      ) : (
        <>
          <div className="grid g2">
            <div className="card pad">
              <div className="h4">Last saved version</div>
              <b>{whenLong(s?.checkpointAt)}</b>
            </div>
            <div className="card pad">
              <div className="h4">Current state</div>
              <b>{whenLong(s?.lastChangeAt)}</b>
              {s?.changesSinceCheckpoint != null && s.changesSinceCheckpoint > 0 && (
                <div className="xs muted">{s.changesSinceCheckpoint} change{s.changesSinceCheckpoint === 1 ? "" : "s"} newer</div>
              )}
            </div>
          </div>
          <p className="sm" style={{ marginBottom: 0 }}>
            Opening the saved version replaces what is on screen. Nothing is lost: the current state is kept as a safety copy in the project's backups folder.
          </p>
        </>
      )}
    </Dialog>
  );
}
