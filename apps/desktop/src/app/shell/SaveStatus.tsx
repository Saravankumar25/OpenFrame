// Save state UX (FSD §3.3–3.4, §62; UX §2.4, §2.13; mocks 017, 018, 019).
//
// * useLiveSaveState — the status-bar save state, kept live from Rust
//   `saveState` events (with a slow refetch as a safety net).
// * SaveErrorBanner / SaveErrorDialog — "Save error" never blocks editing; the
//   dialog appears once per failure with Keep Editing / Retry Save.
// * Leave guard — switching or closing a project while the latest changes are
//   not saved asks Stay / Retry Save / Close Without Saving (never OK/Cancel).

import { useEffect, useState } from "react";
import { create } from "zustand";
import { call, onAppEvent } from "../../ipc/client";
import { queryClient, reportError, useOp } from "../../ipc/query";
import type { SaveState } from "../../ipc/generated/SaveState";
import { Banner, Button, Dialog } from "../../design-system";
import { toast } from "../toast";

const SAVE_KEY = ["app.save_state", {}];

export function useLiveSaveState(): SaveState | undefined {
  const q = useOp<SaveState>("app.save_state", {}, ["*"], { refetchInterval: 30_000 });
  useEffect(() => {
    const un = onAppEvent((ev) => {
      if (ev.type === "saveState") {
        const { type: _t, ...state } = ev;
        queryClient.setQueryData<SaveState>(SAVE_KEY, state);
      }
    });
    return () => {
      void un.then((f) => f());
    };
  }, []);
  return q.data;
}

/** The save state without installing the event listener (the status bar does that). */
export function useSaveState(): SaveState | undefined {
  return useOp<SaveState>("app.save_state", {}, ["*"], { refetchInterval: 30_000 }).data;
}

/** The inline banner, only while saving is failing. Subscribes on its own so save events re-render just this. */
export function LiveSaveErrorBanner({ onOpenRecovery }: { onOpenRecovery: () => void }) {
  const save = useSaveState();
  return save?.status === "Error" ? <SaveErrorBanner onOpenRecovery={onOpenRecovery} /> : null;
}

/** The save-failure dialog, subscribed on its own (see LiveSaveErrorBanner). */
export function LiveSaveErrorDialog() {
  return <SaveErrorDialog state={useSaveState()} />;
}

/** Explicit Save (Ctrl+S / Retry Save). Returns true on success. */
export async function retrySave(): Promise<boolean> {
  try {
    await call("project.save");
    await queryClient.invalidateQueries({ queryKey: SAVE_KEY });
    toast.success("Saved. Everything is stored on this computer.");
    return true;
  } catch (e) {
    await queryClient.invalidateQueries({ queryKey: SAVE_KEY });
    reportError(e);
    return false;
  }
}

/** Mock 017 inline banner, shown above the workspace while saving is failing. */
export function SaveErrorBanner({ onOpenRecovery }: { onOpenRecovery: () => void }) {
  return (
    <div style={{ padding: "8px 16px 0" }}>
      <Banner
        tone="err"
        actions={
          <>
            <Button size="sm" onClick={() => void retrySave()}>Retry Save</Button>
            <Button size="sm" onClick={onOpenRecovery}>Open Recovery</Button>
          </>
        }
      >
        <b>Save error</b> — Your current work is still open. Retry save or open Recovery.
      </Banner>
    </div>
  );
}

/** Mock 017 dialog; shown once for each new save failure. */
export function SaveErrorDialog({ state }: { state: SaveState | undefined }) {
  const failureKey = state?.status === "Error" ? `${state.error?.code ?? "error"}:${state.lastSavedAt ?? 0}` : null;
  const [dismissed, setDismissed] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  if (!failureKey || dismissed === failureKey) return null;
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && setDismissed(failureKey)}
      title="OpenFrame could not save the latest changes to the project."
      size="md"
      footer={
        <>
          <Button onClick={() => setDismissed(failureKey)}>Keep Editing</Button>
          <Button
            variant="primary"
            disabled={busy}
            onClick={async () => {
              setBusy(true);
              const ok = await retrySave();
              setBusy(false);
              if (ok) setDismissed(failureKey);
            }}
          >
            {busy ? "Saving…" : "Retry Save"}
          </Button>
        </>
      }
    >
      <p style={{ marginTop: 0 }}>This is a problem with saving to your computer, not with the internet. Your edits are still on screen and have not been lost.</p>
      <p className="muted sm">Possible causes: the disk is full, the drive was disconnected, or the folder is read-only.</p>
      {state?.error?.message && <p className="sm" style={{ marginBottom: 0 }}>{state.error.message}</p>}
    </Dialog>
  );
}

// -------------------------------------------------------------- leave guard

interface LeaveGuardState {
  pending: { title: string; leave: () => void } | null;
  ask: (title: string, leave: () => void) => void;
  clear: () => void;
}

export const useLeaveGuard = create<LeaveGuardState>((set) => ({
  pending: null,
  ask: (title, leave) => set({ pending: { title, leave } }),
  clear: () => set({ pending: null }),
}));

/**
 * Run `leave` (switch/close project) — unless the latest changes are not saved,
 * in which case the user decides first (FSD §3.4: never silently abandon changes).
 */
export async function guardedLeave(projectTitle: string, leave: () => void | Promise<void>): Promise<void> {
  let status: SaveState["status"] = "Saved";
  try {
    status = (await call<SaveState>("app.save_state")).status;
  } catch {
    // If the state can't be read, fall through to the normal close path, which checkpoints.
  }
  const run = () => void Promise.resolve(leave()).catch(reportError);
  if (status === "Error") useLeaveGuard.getState().ask(projectTitle, run);
  else run();
}

/** Mock 018. */
export function LeaveGuardDialog() {
  const pending = useLeaveGuard((s) => s.pending);
  const clear = useLeaveGuard((s) => s.clear);
  const [busy, setBusy] = useState(false);
  if (!pending) return null;
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && clear()}
      title="This project has changes that are not saved yet."
      size="md"
      footer={
        <>
          <Button onClick={clear}>Stay</Button>
          <Button
            disabled={busy}
            onClick={async () => {
              setBusy(true);
              const ok = await retrySave();
              setBusy(false);
              if (ok) {
                clear();
                pending.leave();
              }
            }}
          >
            {busy ? "Saving…" : "Retry Save"}
          </Button>
          <Button
            variant="danger"
            onClick={() => {
              clear();
              pending.leave();
            }}
          >
            Close Without Saving
          </Button>
        </>
      }
    >
      <p style={{ marginTop: 0 }}>
        OpenFrame is still trying to save <b>{pending.title}</b>. If you leave now you may lose your latest edits.
      </p>
    </Dialog>
  );
}
