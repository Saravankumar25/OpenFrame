// Typed wrappers and pure helpers for backup, project and exchange packages
// (FSD §44–47, §50, §121; UX §3.39). All work happens in Rust: the webview
// only passes user-chosen paths and decisions.

import { call } from "../../ipc/client";
import type { PackageType } from "../../ipc/generated/PackageType";
import type { PackageChange } from "../../ipc/generated/PackageChange";
import type { PackageChangeState } from "../../ipc/generated/PackageChangeState";
import type { PackageImportSession } from "../../ipc/generated/PackageImportSession";
import type { PackageApplyMode } from "../../ipc/generated/PackageApplyMode";
import type { PackageTaskStarted } from "../../ipc/generated/PackageTaskStarted";
import type { PackageInspection } from "../../ipc/generated/PackageInspection";
import type { PackageOpenMode } from "../../ipc/generated/PackageOpenMode";
import type { PackageReviewWorkspace } from "../../ipc/generated/PackageReviewWorkspace";
import type { PackageResponseExported } from "../../ipc/generated/PackageResponseExported";
import type { PackageExportResult } from "../../ipc/generated/PackageExportResult";
import type { PackageExchangeArgs } from "../../ipc/generated/PackageExchangeArgs";
import type { PackageQueueItem } from "../../ipc/generated/PackageQueueItem";

/** The package intents. They are never interchangeable (Import/Export §1). */
export const PACKAGE_INFO: Record<PackageType, { label: string; extension: string; menuLabel: string }> = {
  backup: { label: "Backup Package", extension: "ofbackup", menuLabel: "Create Backup…" },
  project: { label: "Full Project Package", extension: "ofproject", menuLabel: "Export Full Project…" },
  scriptReview: { label: "Screenplay review package", extension: "ofscriptreview", menuLabel: "Export Review Package (script)…" },
  story: { label: "Story Board package", extension: "ofstory", menuLabel: "Story Board Package…" },
  breakdown: { label: "Breakdown package", extension: "ofbreakdown", menuLabel: "Breakdown Package…" },
  shots: { label: "Shot List package", extension: "ofshots", menuLabel: "Shot List Package…" },
  schedule: { label: "Schedule package", extension: "ofschedule", menuLabel: "Schedule Package…" },
  callReview: { label: "Call Sheet review package", extension: "ofcallreview", menuLabel: "Call Sheet Review Package…" },
  response: { label: "Response package", extension: "ofresponse", menuLabel: "Response Package…" },
};

/** Exchange package types that can be exported from a project (menu order, UX §3.39). */
export const EXCHANGE_EXPORT_TYPES: PackageType[] = ["scriptReview", "story", "breakdown", "shots", "schedule", "callReview"];

export function isExchangeType(t: PackageType): boolean {
  return t !== "backup" && t !== "project";
}

/** Save/open dialog filters. */
export function packageFilter(types: PackageType[], name: string): { name: string; extensions: string[] }[] {
  return [{ name, extensions: types.map((t) => PACKAGE_INFO[t].extension) }];
}

export const EXCHANGE_OPEN_FILTER = packageFilter(
  ["scriptReview", "story", "breakdown", "shots", "schedule", "callReview", "response"],
  "OpenFrame exchange or review package",
);
export const PROJECT_OPEN_FILTER = packageFilter(["project", "backup"], "OpenFrame project or backup package");
export const REVIEW_OPEN_FILTER = packageFilter(["scriptReview", "response"], "OpenFrame review package");

/** Join a folder and a file name for the save dialog's default path (Windows or POSIX). */
export function joinPath(dir: string, name: string): string {
  if (!dir) return name;
  const sep = dir.includes("\\") ? "\\" : "/";
  return dir.endsWith(sep) ? dir + name : dir + sep + name;
}

/** Backup file name: project name + date/time + optional label (FSD §44.8). */
export function backupFileName(baseName: string, label: string): string {
  const clean = label.trim().replace(/[<>:"/\\|?*]/g, "_");
  return `${baseName}${clean ? ` ${clean}` : ""}.ofbackup`;
}

export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(0)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

/** UI label + chip tone for a change state (mockup 161). */
export function stateChip(state: PackageChangeState): { label: string; tone: "g" | "y" | "r" | "b" | "default" } {
  switch (state) {
    case "safe":
      return { label: "Safe mapping", tone: "g" };
    case "review":
      return { label: "Review required", tone: "b" };
    case "conflict":
      return { label: "Conflict", tone: "r" };
    case "ambiguous":
      return { label: "Ambiguous", tone: "y" };
    case "unmapped":
      return { label: "Unmapped", tone: "r" };
    default:
      return { label: "None", tone: "default" };
  }
}

export function compatibilityTone(c: string): "g" | "y" | "r" {
  return c === "Valid" ? "g" : c === "Stale" ? "y" : "r";
}

/** Human result state (Import/Export §20: never "Import complete" for a subset). */
export function resultLabel(status: string): string {
  switch (status) {
    case "Applied":
      return "Applied";
    case "PartiallyApplied":
      return "Partially Applied";
    case "PendingReview":
      return "Pending Review";
    case "Previewed":
      return "Previewed — nothing applied";
    default:
      return status;
  }
}

/** The changes "Import Selected" starts with: what the preview proposes by default. */
export function defaultSelection(changes: PackageChange[]): Set<string> {
  return new Set(changes.filter((c) => c.applicable && !c.copyOnly && c.selectedByDefault).map((c) => c.id));
}

/** Toggle every applicable (non-copy) change of a group. */
export function toggleGroup(selected: Set<string>, changes: PackageChange[], group: string, on: boolean): Set<string> {
  const next = new Set(selected);
  for (const c of changes) {
    if (c.group !== group || !c.applicable || c.copyOnly) continue;
    if (on) next.add(c.id);
    else next.delete(c.id);
  }
  return next;
}

export function groupSelection(selected: Set<string>, changes: PackageChange[], group: string): "all" | "some" | "none" {
  const items = changes.filter((c) => c.group === group && c.applicable && !c.copyOnly);
  const n = items.filter((c) => selected.has(c.id)).length;
  return n === 0 ? "none" : n === items.length ? "all" : "some";
}

/** Default choice for a stale review package: comments only (FSD §47.9). */
export function staleDefault(session: PackageImportSession): PackageApplyMode {
  return session.changes.some((c) => c.kind === "comment" && c.applicable) ? "commentsOnly" : "record";
}

// ------------------------------------------------------------------ wrappers

export const packagesApi = {
  inspect: (path: string) => call<PackageInspection>("packages.inspect_package", { path }),
  importProject: (path: string, mode: PackageOpenMode, parentDir?: string | null) =>
    call<PackageTaskStarted>("packages.import_project", { path, mode, parentDir: parentDir ?? null }),
  createBackup: (path: string, label: string, includeExternal: boolean) =>
    call<PackageTaskStarted>("packages.create_backup", { path, label: label.trim() || null, includeExternal }),
  exportProject: (path: string, includeExternal: boolean) =>
    call<PackageTaskStarted>("packages.export_project", { path, includeExternal }),
  exportExchange: (args: PackageExchangeArgs) => call<PackageExportResult>("packages.export_exchange", args),
  openExchange: (path: string) => call<PackageImportSession>("packages.open_exchange", { path }),
  apply: (sessionId: string, mode: PackageApplyMode, selected: string[], createBackup: boolean) =>
    call<PackageImportSession>("packages.apply_import", { sessionId, mode, selected, createBackup }),
  cancel: (sessionId: string) => call<PackageImportSession>("packages.cancel_import", { sessionId }),
  undo: (sessionId: string) => call<PackageImportSession>("packages.undo_import", { sessionId }),
  reviewOpen: (path: string) => call<PackageReviewWorkspace>("packages.review_open", { path }),
  reviewComment: (packageId: string, sceneId: string, body: string, quotedText: string | null) =>
    call<PackageReviewWorkspace>("packages.review_comment", { packageId, sceneId, body, quotedText }),
  reviewDeleteComment: (packageId: string, commentId: string) =>
    call<PackageReviewWorkspace>("packages.review_delete_comment", { packageId, commentId }),
  reviewExportResponse: (packageId: string, path: string) =>
    call<PackageResponseExported>("packages.review_export_response", { packageId, path }),
  queueAttach: (itemId: string, sceneId: string) => call<PackageQueueItem>("packages.queue_attach", { itemId, sceneId }),
  cancelTask: (taskId: string) => call<boolean>("app.cancel_task", { taskId }),
};
