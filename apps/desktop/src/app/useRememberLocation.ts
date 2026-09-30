// FSD §6.2 "Continue": the app remembers the last meaningful location per
// project, user and workspace; Project Home offers "Continue …" cards that
// navigate straight back there (via navigateTo, like search hits).
//
// Usage inside a workspace:
//   useRememberLocation(card ? { workspace: "story", label: `Scene Card “${card.title}”`, sceneCardId: card.id } : null);
//
// Only call it for meaningful places (an opened scene, card, day, document) —
// not for every scroll position. Updates are debounced and de-duplicated.

import { useEffect } from "react";
import { call } from "../ipc/client";
import type { WorkspaceId } from "./stores";

export interface ContinueLocation {
  workspace: WorkspaceId;
  /** Workspace-local page, e.g. "outline" or "locations". */
  sub?: string;
  /** Short human description shown on the Continue card, e.g. "Scene 12 · Draft 6". */
  label: string;
  /** Focus ids understood by the workspace, e.g. sceneCardId, sceneId, dayId. */
  [key: string]: string | undefined;
}

const DEBOUNCE_MS = 800;
let lastSent = "";

/** Remember `location` as the place "Continue" returns to. Pass null when nothing meaningful is open. */
export function useRememberLocation(location: ContinueLocation | null | undefined): void {
  const key = location ? JSON.stringify(location) : "";
  useEffect(() => {
    if (!key || key === lastSent) return;
    const t = window.setTimeout(() => {
      lastSent = key;
      call("project.set_last_location", { location: JSON.parse(key) as ContinueLocation }).catch((e: unknown) => {
        // Non-critical view state: never interrupt the user for it.
        lastSent = "";
        console.warn("could not remember location", e);
      });
    }, DEBOUNCE_MS);
    return () => window.clearTimeout(t);
  }, [key]);
}
