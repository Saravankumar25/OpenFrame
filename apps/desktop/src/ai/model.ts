// Pure view logic for the AI Assistant panel (FSD §42; UX §3.40; mockups 174–181).
// No project truth lives here: everything shown comes from `ai.*` operations.

import type { Route, WorkspaceId } from "../app/stores";
import type { AiChangeSetDto } from "../ipc/generated/AiChangeSetDto";
import type { AiConfidence } from "../ipc/generated/AiConfidence";
import type { AiInstallProgress } from "../ipc/generated/AiInstallProgress";
import type { AiNavTarget } from "../ipc/generated/AiNavTarget";
import type { AiScopeArgs } from "../ipc/generated/AiScopeArgs";
import type { AiScopeKind } from "../ipc/generated/AiScopeKind";
import type { AiStatusDto } from "../ipc/generated/AiStatusDto";

export const SCOPE_LABELS: Record<AiScopeKind, string> = {
  CurrentSelection: "Current Selection",
  CurrentScene: "Current Scene",
  CurrentScreenplay: "Current Screenplay",
  SpecificDraft: "Specific Draft",
  StoryBoard: "Story Board",
  IdeaVaultSelection: "Idea Vault Selection",
  Production: "Production Workspace",
  ShootingDay: "Shooting Day",
  CallSheet: "Call Sheet",
  WholeProject: "Whole Project",
};

/** Route params that identify selected objects (search / Continue / AI navigation targets). */
const SELECTION_PARAMS = ["cardId", "sceneCardId", "itemId", "vaultItemId", "characterId", "fileId", "beatId"];

function selectionIds(route: Route): string[] {
  const p = route.params ?? {};
  return SELECTION_PARAMS.map((k) => p[k]).filter((v): v is string => typeof v === "string" && v.length > 0);
}

/** Scopes that make sense right now (a scope that needs a target is offered only when one exists). */
export function scopeOptions(route: Route): AiScopeKind[] {
  const p = route.params ?? {};
  const out: AiScopeKind[] = [];
  const sel = selectionIds(route);
  if (sel.length > 0) out.push(route.workspace === "vault" ? "IdeaVaultSelection" : "CurrentSelection");
  if (p.sceneId) out.push("CurrentScene");
  out.push("CurrentScreenplay");
  if (p.draftId) out.push("SpecificDraft");
  out.push("StoryBoard", "Production");
  if (p.shootingDayId) out.push("ShootingDay");
  if (p.callSheetId) out.push("CallSheet");
  out.push("WholeProject");
  return out;
}

const WORKSPACE_DEFAULT: Partial<Record<WorkspaceId, AiScopeKind>> = {
  screenplay: "CurrentScreenplay",
  story: "StoryBoard",
  breakdown: "Production",
  production: "Production",
  callsheets: "Production",
};

/** Scope inherited from where the user is (AI spec §7.2); Whole Project otherwise. */
export function defaultScope(route: Route): AiScopeKind {
  const opts = scopeOptions(route);
  const p = route.params ?? {};
  if (route.workspace === "screenplay" && p.sceneId) return "CurrentScene";
  if (opts.includes("IdeaVaultSelection")) return "IdeaVaultSelection";
  if (opts.includes("CurrentSelection") && route.workspace !== "screenplay") return "CurrentSelection";
  if (route.workspace === "callsheets" && p.callSheetId) return "CallSheet";
  return WORKSPACE_DEFAULT[route.workspace] ?? "WholeProject";
}

/** Arguments for `ai.ask` (references only — Rust resolves and budgets the actual context). */
export function scopeArgs(kind: AiScopeKind, route: Route): AiScopeArgs {
  const p = route.params ?? {};
  const selection = kind === "CurrentSelection" || kind === "IdeaVaultSelection" ? selectionIds(route).map((id) => ({ id })) : [];
  return {
    kind,
    draftId: kind === "SpecificDraft" || kind === "CurrentScreenplay" ? p.draftId ?? null : null,
    sceneId: kind === "CurrentScene" ? p.sceneId ?? null : null,
    selection,
    shootingDayId: kind === "ShootingDay" ? p.shootingDayId ?? null : null,
    callSheetId: kind === "CallSheet" ? p.callSheetId ?? null : null,
  };
}

const KNOWN_WORKSPACES: WorkspaceId[] = [
  "home", "vault", "story", "screenplay", "breakdown", "production", "callsheets", "files", "trash", "activity", "notes", "settings",
];

/** Convert an assistant navigation target into a shell route (unknown workspaces are ignored). */
export function toRoute(nav: AiNavTarget | null | undefined): Route | null {
  if (!nav || !KNOWN_WORKSPACES.includes(nav.workspace as WorkspaceId)) return null;
  const r: Route = { workspace: nav.workspace as WorkspaceId };
  if (nav.sub) r.sub = nav.sub;
  const params: Record<string, string> = {};
  for (const [k, v] of Object.entries(nav.params ?? {})) {
    if (typeof v === "string") params[k] = v;
  }
  if (Object.keys(params).length > 0) r.params = params;
  return r;
}

export function formatBytes(n: number): string {
  const gb = 1024 ** 3;
  if (n >= gb) {
    const v = n / gb;
    return `${v >= 10 ? v.toFixed(0) : v.toFixed(1)} GB`;
  }
  return `${Math.max(1, Math.ceil(n / (1024 * 1024)))} MB`;
}

/** Whole-percent progress of the current download step (null when indeterminate). */
export function progressPercent(p: AiInstallProgress | null | undefined): number | null {
  if (!p || p.bytesTotal <= 0) return null;
  return Math.max(0, Math.min(100, Math.floor((p.bytesDone / p.bytesTotal) * 100)));
}

export type PanelMode = "unavailable" | "setup" | "installing" | "ready";

/** Which top-level view the panel shows. */
export function panelMode(status: AiStatusDto | undefined, setupRequested: boolean): PanelMode {
  if (!status) return "unavailable";
  if (status.install?.active) return "installing";
  if (status.installed) return "ready";
  if (setupRequested || (status.install && ["paused", "failed"].includes(status.install.phase))) return "setup";
  return "unavailable";
}

export function runtimeLabel(state: string): string | null {
  switch (state) {
    case "Starting":
    case "LoadingModel":
      return "Getting Offline AI ready on this computer…";
    case "Busy":
      return "Working on a request…";
    case "Stopping":
      return "Stopping…";
    default:
      return null;
  }
}

export function confidenceLabel(c: AiConfidence | null | undefined): { text: string; tone: "g" | "b" | "y" } | null {
  switch (c) {
    case "Exact":
      return { text: "Exact · from project data", tone: "g" };
    case "Inferred":
      return { text: "Inferred · written by Offline AI", tone: "b" };
    case "Unavailable":
      return { text: "Not available in project data", tone: "y" };
    default:
      return null;
  }
}

/** What the user can do with a Change Set in its current state. */
export function changeSetActions(cs: AiChangeSetDto): { apply: boolean; reject: boolean; recheck: boolean } {
  switch (cs.state) {
    case "Pending":
      return { apply: true, reject: true, recheck: false };
    case "Stale":
    case "Conflict":
      return { apply: false, reject: true, recheck: true };
    default:
      return { apply: false, reject: false, recheck: false };
  }
}

export function changeSetStateText(cs: AiChangeSetDto): string | null {
  switch (cs.state) {
    case "Applied":
      return `Applied ${cs.appliedOperations === 1 ? "1 change" : `${cs.appliedOperations} changes`}. Use Undo to reverse it.`;
    case "Rejected":
      return "Cancelled. Nothing was changed.";
    case "Accepted":
      return "Applying…";
    case "Failed":
      return cs.errorMessage ?? "This proposal couldn't be applied. Nothing was changed.";
    default:
      return null;
  }
}

export const STALE_HINT = "AI never applies a stale proposal blindly. It must be re-checked against the current project.";
export const UNAVAILABLE_TEXT = "AI is currently unavailable. OpenFrame's core workflows continue to work normally.";
