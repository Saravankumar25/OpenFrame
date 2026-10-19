// Navigate to a backend-provided target (search hits, Continue, AI navigation).
// Targets look like { workspace: "story", sub?: "outline", sceneCardId: "…" }.

import { useNav, useUi, type WorkspaceId } from "./stores";

const KNOWN: WorkspaceId[] = ["home", "vault", "story", "screenplay", "breakdown", "production", "callsheets", "files", "trash", "activity", "notes", "settings"];

export function navigateTo(nav: unknown, store: "project" | "global" = "project"): void {
  if (!nav || typeof nav !== "object") return;
  const n = nav as Record<string, unknown>;
  const ws = String(n.workspace ?? "");
  if (!KNOWN.includes(ws as WorkspaceId)) return;
  const params: Record<string, string> = {};
  for (const [k, v] of Object.entries(n)) {
    if (k !== "workspace" && k !== "sub" && (typeof v === "string" || typeof v === "number")) params[k] = String(v);
  }
  if (store === "global") params.scope = "global";
  useUi.getState().setUndoScope(store);
  useNav.getState().go({ workspace: ws as WorkspaceId, sub: typeof n.sub === "string" ? n.sub : undefined, params });
}
