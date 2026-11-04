// Pure view logic for the AI Assistant panel (FSD §42; UX §3.40; mockups 174–181).
// No project truth lives here: everything shown comes from `ai.*` operations.

import type { Route, WorkspaceId } from "../app/stores";
import type { AiChangeSetDto } from "../ipc/generated/AiChangeSetDto";
import type { AiConfidence } from "../ipc/generated/AiConfidence";
import type { AiNavTarget } from "../ipc/generated/AiNavTarget";
import type { AiProvenance } from "../ipc/generated/AiProvenance";
import type { AiStepDto } from "../ipc/generated/AiStepDto";
import type { AiStatusDto } from "../ipc/generated/AiStatusDto";

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

export type PanelMode = "unavailable" | "off" | "download" | "ready";

/**
 * Which top-level view the panel shows (ADR-0026). OpenFrame AI is ready once its models are
 * downloaded; before that, and when a downloaded file is missing or damaged, the panel offers the
 * download ("download"); a missing or damaged engine needs a reinstall ("off", with the reason).
 */
export function panelMode(status: AiStatusDto | undefined): PanelMode {
  if (!status) return "unavailable";
  if (status.available) return "ready";
  return status.bundleState === "NotInstalled" || status.problem?.code === "ai.models_damaged" ? "download" : "off";
}

/** The reason to download again when a downloaded file is missing or damaged, else undefined. */
export function damagedModels(status: AiStatusDto): string | undefined {
  return status.problem?.code === "ai.models_damaged" ? status.problem.message : undefined;
}

/** Plain-language reason Offline AI is off (from the file check). */
export function offReason(status: AiStatusDto): string {
  return status.problem?.message ?? UNAVAILABLE_TEXT;
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
      return "Rejected. Nothing was changed.";
    case "Accepted":
      return "Applying…";
    case "Failed":
      return cs.errorMessage ?? "This proposal couldn't be applied. Nothing was changed.";
    default:
      return null;
  }
}

/** "1 change" / "3 changes": how many registry operations applying will run. */
export function changeCount(n: number): string {
  return n === 1 ? "1 change" : `${n} changes`;
}

export const STALE_HINT = "AI never applies a stale proposal blindly. It must be re-checked against the current project.";
export const UNAVAILABLE_TEXT = "AI is currently unavailable. OpenFrame's core workflows continue to work normally.";
export const PREPARING_TEXT = "Preparing project context…";
export const DEGRADED_TEXT = "AI can still answer some questions while project context is being prepared.";

/**
 * Project-context (search index) state as reported by `ai.index_status`: a plain
 * state name or an object carrying `state`. Anything else counts as unknown.
 */
export function indexStateOf(v: unknown): string | null {
  if (typeof v === "string") return v;
  if (v && typeof v === "object" && typeof (v as { state?: unknown }).state === "string") return (v as { state: string }).state;
  return null;
}

/**
 * The indexing notice to show, only when it matters to the user: while the project
 * context is being rebuilt the assistant still works, with fewer sources.
 * Small incremental updates and fallbacks stay silent.
 */
export function indexNotice(state: string | null): { preparing: string; degraded: string } | null {
  return state === "Rebuilding" || state === "Stale" ? { preparing: PREPARING_TEXT, degraded: DEGRADED_TEXT } : null;
}

export interface SourceChip {
  key: string;
  text: string;
  route: Route | null;
}

/** Provenance as chips; a chip navigates when its source can be opened. */
export function sourceChips(provenance: AiProvenance[]): SourceChip[] {
  return provenance.map((p, i) => ({
    key: `${i}:${p.kind}:${p.label}`,
    text: p.kind === "Scope" || p.kind === "Basis" ? p.label : `${p.kind}: ${p.label}`,
    route: toRoute(p.nav ?? null),
  }));
}

/** Short line describing the tool steps behind an answer (null for single-step answers). */
export function stepsSummary(steps: AiStepDto[]): string | null {
  if (steps.length < 2) return null;
  return `${steps.length} steps: ${steps.map((s) => s.label).join(" · ")}`;
}
