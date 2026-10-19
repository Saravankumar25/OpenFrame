// Client-side VIEW state only (ESD §7: never project truth).

import { create } from "zustand";

/** Top-level workspaces in the left navigation (FSD §3.1, exact order). */
export type WorkspaceId =
  | "home"
  | "vault"
  | "story"
  | "screenplay"
  | "breakdown"
  | "production"
  | "callsheets"
  | "files"
  // secondary pages reachable from menus/status bar
  | "trash"
  | "activity"
  | "notes"
  | "settings";

export interface Route {
  workspace: WorkspaceId;
  /** Workspace-local page, e.g. production sub-tab "locations" or story view "outline". */
  sub?: string;
  /** Focus target, e.g. { sceneId } or { cardId }, used by search, Continue and AI navigation. */
  params?: Record<string, string>;
}

interface NavState {
  route: Route;
  history: Route[];
  go: (r: Route) => void;
  back: () => void;
  reset: () => void;
}

export const useNav = create<NavState>((set) => ({
  route: { workspace: "home" },
  history: [],
  go: (r) => set((s) => ({ route: r, history: [...s.history.slice(-30), s.route] })),
  back: () =>
    set((s) => {
      const prev = s.history[s.history.length - 1];
      return prev ? { route: prev, history: s.history.slice(0, -1) } : s;
    }),
  reset: () => set({ route: { workspace: "home" }, history: [] }),
}));

/**
 * The screenplay draft open in the Screenplay workspace. A project may hold
 * several screenplays, so the assistant's "Current Screenplay" scope sends
 * this draft instead of letting the app guess which screenplay is meant.
 */
interface OpenDraftState {
  draftId: string | null;
  setDraftId: (id: string | null) => void;
}

export const useOpenDraft = create<OpenDraftState>((set) => ({
  draftId: null,
  setDraftId: (draftId) => set({ draftId }),
}));

/** Where the AI Assistant panel remembers whether it is open (a per-user layout preference, like
 * the Story Board zoom; never project data). */
export const AI_PANEL_KEY = "openframe.ai.panelOpen";

/** The saved AI panel state. Empty, unreadable or unexpected storage means closed. */
export function readAiPanelOpen(): boolean {
  try {
    return window.localStorage.getItem(AI_PANEL_KEY) === "true";
  } catch {
    return false;
  }
}

function writeAiPanelOpen(open: boolean) {
  try {
    window.localStorage.setItem(AI_PANEL_KEY, open ? "true" : "false");
  } catch {
    // Layout preference only; ignore storage failures.
  }
}

interface UiState {
  searchOpen: boolean;
  /** The AI Assistant panel is docked beside the page (owner decision 2026-10-08). */
  aiOpen: boolean;
  quickOpen: boolean;
  /** Which store global undo/redo targets (the Global Idea Vault has its own history). */
  undoScope: "project" | "global";
  setSearchOpen: (v: boolean) => void;
  setAiOpen: (v: boolean) => void;
  setQuickOpen: (v: boolean) => void;
  setUndoScope: (v: "project" | "global") => void;
}

export const useUi = create<UiState>((set) => ({
  searchOpen: false,
  aiOpen: readAiPanelOpen(),
  quickOpen: false,
  undoScope: "project",
  setSearchOpen: (v) => set({ searchOpen: v }),
  setAiOpen: (v) => {
    writeAiPanelOpen(v);
    set({ aiOpen: v });
  },
  setQuickOpen: (v) => set({ quickOpen: v }),
  setUndoScope: (v) => set({ undoScope: v }),
}));

/** Pending "create" intents triggered from the quick actions menu; workspaces consume them. */
interface IntentState {
  intent: string | null;
  fire: (i: string) => void;
  consume: (i: string) => boolean;
}
export const useIntent = create<IntentState>((set, get) => ({
  intent: null,
  fire: (i) => set({ intent: i }),
  consume: (i) => {
    if (get().intent === i) {
      set({ intent: null });
      return true;
    }
    return false;
  },
}));
