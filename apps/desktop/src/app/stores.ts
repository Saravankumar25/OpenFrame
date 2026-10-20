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
  /** Pages to go back to (oldest first) and pages to go forward to (nearest last). */
  history: Route[];
  future: Route[];
  go: (r: Route) => void;
  back: () => void;
  forward: () => void;
  reset: () => void;
}

/** The open page survives a reload of the window (owner change list item 1). Session only: a new
 * start of the app opens on Project Home. Opening another project calls reset(), which clears it. */
const ROUTE_KEY = "openframe.route";
const WORKSPACE_IDS: WorkspaceId[] = ["home", "vault", "story", "screenplay", "breakdown", "production", "callsheets", "files", "trash", "activity", "notes", "settings"];

function readRoute(): Route {
  try {
    const r = JSON.parse(window.sessionStorage.getItem(ROUTE_KEY) ?? "null") as Route | null;
    if (r && WORKSPACE_IDS.includes(r.workspace)) return r;
  } catch {
    // Unreadable view state: start on Project Home.
  }
  return { workspace: "home" };
}

function writeRoute(r: Route) {
  try {
    window.sessionStorage.setItem(ROUTE_KEY, JSON.stringify(r));
  } catch {
    // View state only.
  }
}

const same = (a: Route, b: Route) => JSON.stringify(a) === JSON.stringify(b);

export const useNav = create<NavState>((set) => ({
  route: readRoute(),
  history: [],
  future: [],
  go: (r) =>
    set((s) => {
      writeRoute(r);
      return same(r, s.route) ? { route: r } : { route: r, history: [...s.history.slice(-50), s.route], future: [] };
    }),
  back: () =>
    set((s) => {
      const prev = s.history[s.history.length - 1];
      if (!prev) return s;
      writeRoute(prev);
      return { route: prev, history: s.history.slice(0, -1), future: [...s.future, s.route] };
    }),
  forward: () =>
    set((s) => {
      const next = s.future[s.future.length - 1];
      if (!next) return s;
      writeRoute(next);
      return { route: next, history: [...s.history, s.route], future: s.future.slice(0, -1) };
    }),
  reset: () => {
    writeRoute({ workspace: "home" });
    set({ route: { workspace: "home" }, history: [], future: [] });
  },
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
 * the Beat Board zoom; never project data). */
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

const NAV_KEY = "openframe.nav.collapsed";
function readNavCollapsed(): boolean {
  try {
    return window.localStorage.getItem(NAV_KEY) === "true";
  } catch {
    return false;
  }
}

interface UiState {
  /** The left navigation is collapsed to an icon rail (owner request 2026-10-19); remembered per person. */
  navCollapsed: boolean;
  setNavCollapsed: (v: boolean) => void;
  /** Screenplay focus mode: everything but the page is hidden (owner change list item 13). */
  focusMode: boolean;
  setFocusMode: (v: boolean) => void;
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
  navCollapsed: readNavCollapsed(),
  setNavCollapsed: (v) => {
    try {
      window.localStorage.setItem(NAV_KEY, v ? "true" : "false");
    } catch {
      // Layout preference only.
    }
    set({ navCollapsed: v });
  },
  focusMode: false,
  setFocusMode: (v) => set({ focusMode: v }),
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
