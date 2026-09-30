// View state for the Share / Exchange, Backup and Portability dialogs.
// Only which dialog is showing (and which background task it follows) lives
// here — project truth stays in Rust.

import { useEffect } from "react";
import { create } from "zustand";
import { onAppEvent } from "../../ipc/client";
import type { PackageType } from "../../ipc/generated/PackageType";
import type { TaskProgress } from "../../ipc/generated/TaskProgress";

export type PackagesDialog =
  | { kind: "backup" }
  | { kind: "exportProject" }
  | { kind: "openProject" }
  | { kind: "exportExchange"; packageType: PackageType }
  | { kind: "importExchange"; sessionId?: string }
  | { kind: "viewer"; source: "review" | "session" | "record"; key: string; back?: PackagesDialog }
  | { kind: "queue" }
  | { kind: "history" };

interface PackagesUi {
  dialog: PackagesDialog | null;
  /** Background task the open dialog follows (survives the shell re-mounting while a project switches). */
  taskId: string | null;
  open: (d: PackagesDialog) => void;
  close: () => void;
  setTaskId: (id: string | null) => void;
}

export const usePackagesUi = create<PackagesUi>((set) => ({
  dialog: null,
  taskId: null,
  open: (d) => set({ dialog: d, taskId: null }),
  close: () => set({ dialog: null, taskId: null }),
  setTaskId: (id) => set({ taskId: id }),
}));

interface TaskStore {
  tasks: Record<string, TaskProgress>;
  put: (t: TaskProgress) => void;
}

export const useTaskStore = create<TaskStore>((set) => ({
  tasks: {},
  put: (t) => set((s) => ({ tasks: { ...s.tasks, [t.taskId]: t } })),
}));

let listening = false;

/**
 * Follow background task events (backup, project export/import). Installed once
 * when the package UI first mounts — before any task starts — and kept for the
 * app's lifetime, so a task that finishes while the shell re-mounts is not missed.
 */
export function ensureTaskListener(): void {
  if (listening) return;
  listening = true;
  onAppEvent((ev) => {
    if (ev.type !== "task") return;
    const { type: _type, ...t } = ev;
    useTaskStore.getState().put(t);
  }).catch(() => {
    listening = false;
  });
}

export function useTask(taskId: string | null): TaskProgress | undefined {
  useEffect(() => ensureTaskListener(), []);
  return useTaskStore((s) => (taskId ? s.tasks[taskId] : undefined));
}

export function isFinished(t: TaskProgress | undefined): boolean {
  return !!t && (t.state === "completed" || t.state === "failed" || t.state === "cancelled");
}
