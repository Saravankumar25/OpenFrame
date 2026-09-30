// Application Home view state (which list is shown, pending dialogs). Client
// view state only — the project list itself always comes from Rust.
// The shell's project menu uses it for "All projects…" / "Archived…" / "New Project…".

import { create } from "zustand";

/** What Application Home shows: the project list, or the Global Idea Vault (no project open). */
export type HomeView = "projects" | "vault";

interface HomeViewState {
  view: HomeView;
  showArchived: boolean;
  /** Open the New Project dialog when Home appears (e.g. "+ > New Project" inside a project). */
  newProjectRequested: boolean;
  setView: (v: HomeView) => void;
  setShowArchived: (v: boolean) => void;
  requestNewProject: (v: boolean) => void;
}

export const useHomeView = create<HomeViewState>((set) => ({
  view: "projects",
  showArchived: false,
  newProjectRequested: false,
  setView: (v) => set({ view: v }),
  setShowArchived: (v) => set({ showArchived: v, view: "projects" }),
  requestNewProject: (v) => set({ newProjectRequested: v }),
}));
