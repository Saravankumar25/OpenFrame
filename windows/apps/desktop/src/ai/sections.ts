// The sections a person picks before asking (spec ai-section-scoped-assistant, ADR-0040).
// Pure logic and the kept selection; the tree itself comes from the core (`ai.sections`), so no
// section list lives here. Nothing is picked by default: the person is the router.

import { create } from "zustand";
import type { AiLeafDto } from "../ipc/generated/AiLeafDto";
import type { AiSectionTreeDto } from "../ipc/generated/AiSectionTreeDto";

/** The most sections one question may read. */
export const MAX_PICKS = 3;
export const CAP_HINT = "Three sections at most. Remove one to add another.";
export const PICK_HINT = "Pick a section";
export const SCREENPLAY_HINT = "Choose a screenplay";
export const NOT_READY_HINT = "The sections are not ready yet";

/** A picked section with the instance choices made under it (the payload of `ai.ask`). */
export interface Pick {
  leaf: string;
  screenplayId?: string;
  draftId?: string;
  sceneId?: string;
  shootingDayId?: string;
  callSheetId?: string;
}

/** Instance keys a pick can hold; clearing a section clears them all. */
const INSTANCE_KEYS = ["screenplayId", "draftId", "sceneId", "shootingDayId", "callSheetId"] as const;

interface PicksState {
  picks: Pick[];
  /** The project the picks were made in. */
  projectId: string | null;
  set: (picks: Pick[]) => void;
  reset: () => void;
  /** Another project than the one the picks belong to empties them; `null` (nothing open yet) changes nothing. */
  bind: (projectId: string | null) => void;
}

/** The selection is kept for the conversation: it survives closing the panel, and "New" empties it.
 * It belongs to one project: ids from another project mean nothing. */
export const useAiPicks = create<PicksState>((set) => ({
  picks: [],
  projectId: null,
  set: (picks) => set({ picks }),
  reset: () => set({ picks: [] }),
  bind: (projectId) =>
    set((s) => (projectId === null || projectId === s.projectId ? s : { projectId, picks: s.projectId === null ? s.picks : [] })),
}));

export function leavesOf(tree: AiSectionTreeDto | undefined): AiLeafDto[] {
  return (tree?.branches ?? []).flatMap((b) => b.leaves);
}

export function leafById(tree: AiSectionTreeDto | undefined, id: string): AiLeafDto | undefined {
  return leavesOf(tree).find((l) => l.id === id);
}

export function isPicked(picks: Pick[], leaf: string): boolean {
  return picks.some((p) => p.leaf === leaf);
}

/** Pick or unpick a section. A fourth pick is ignored: the cap is a hard limit, not a warning. */
export function toggle(picks: Pick[], leaf: string, on: boolean): Pick[] {
  if (!on) return picks.filter((p) => p.leaf !== leaf);
  if (isPicked(picks, leaf) || picks.length >= MAX_PICKS) return picks;
  return [...picks, { leaf }];
}

/** Change the instance choices under a picked section; an empty value clears that choice. */
export function setInstance(picks: Pick[], leaf: string, key: (typeof INSTANCE_KEYS)[number], value: string): Pick[] {
  // The scene lists of every pick come from the screenplay and draft named, so a change there
  // clears the scene of all picks, not only of the one that changed.
  const scenesChange = key === "screenplayId" || key === "draftId";
  return picks.map((p) => {
    if (p.leaf !== leaf) {
      if (!scenesChange || !p.sceneId) return p;
      const { sceneId: _gone, ...rest } = p;
      return rest;
    }
    const next: Pick = { ...p };
    if (value) next[key] = value;
    else delete next[key];
    // A new screenplay has its own drafts and scenes.
    if (key === "screenplayId") delete next.draftId;
    if (scenesChange) delete next.sceneId;
    return next;
  });
}

/** Drop picks the tree no longer offers (a section the person may not read, Episodes on a film), and
 * the screenplay, draft, scene, day or call sheet a pick names that is gone (deleted since). Pass only
 * a tree that has really arrived, never the stale one shown while a new one loads. */
export function pruneToTree(tree: AiSectionTreeDto | undefined, picks: Pick[]): Pick[] {
  if (!tree) return picks;
  const known = (list: { id: string }[], id: string | undefined) => !id || list.some((x) => x.id === id);
  let changed = false;
  const kept: Pick[] = [];
  for (const p of picks) {
    if (!leafById(tree, p.leaf)) {
      changed = true;
      continue;
    }
    const next: Pick = { ...p };
    if (!known(tree.screenplays, p.screenplayId)) {
      delete next.screenplayId;
      delete next.draftId;
      delete next.sceneId;
    }
    if (next.draftId && !tree.screenplays.filter((s) => !next.screenplayId || s.id === next.screenplayId).some((s) => s.drafts.some((d) => d.id === next.draftId))) {
      delete next.draftId;
      delete next.sceneId;
    }
    if (next.sceneId && !tree.screenplays.some((s) => s.scenes.some((sc) => sc.id === next.sceneId))) delete next.sceneId;
    if (!known(tree.days, next.shootingDayId)) delete next.shootingDayId;
    if (!known(tree.callSheets, next.callSheetId)) delete next.callSheetId;
    const same = INSTANCE_KEYS.every((k) => next[k] === p[k]);
    if (!same) changed = true;
    kept.push(same ? p : next);
  }
  return changed ? kept : picks;
}

/** True when the project holds several screenplays and a Screenplay pick has not named one. */
export function needsScreenplay(tree: AiSectionTreeDto | undefined, picks: Pick[]): boolean {
  if (!tree || tree.screenplays.length < 2) return false;
  return picks.some((p) => leafById(tree, p.leaf)?.picker === "screenplay" && !p.screenplayId);
}

/** Why Send is disabled, in words; null when the question may be sent. */
export function sendHint(tree: AiSectionTreeDto | undefined, picks: Pick[]): string | null {
  if (picks.length === 0) return PICK_HINT;
  if (!tree) return NOT_READY_HINT;
  if (needsScreenplay(tree, picks)) return SCREENPLAY_HINT;
  return null;
}

/** The picks as the core takes them (`ai.ask` field `sections`). */
export function askSections(picks: Pick[]): Pick[] {
  return picks.map((p) => {
    const out: Pick = { leaf: p.leaf };
    for (const k of INSTANCE_KEYS) if (p[k]) out[k] = p[k];
    return out;
  });
}

/** The screenplay whose draft the scene lists show: the one a Screenplay pick names, else the only one. */
export function draftOfPicks(tree: AiSectionTreeDto | undefined, picks: Pick[]): string | null {
  const mine = picks.find((p) => leafById(tree, p.leaf)?.picker === "screenplay");
  return mine?.draftId ?? null;
}
