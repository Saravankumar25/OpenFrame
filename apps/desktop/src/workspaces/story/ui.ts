// Story workspace VIEW state only (selection, open drawer, dialogs, zoom).
// Story data always comes from Rust queries; nothing here is project truth.

import { create } from "zustand";
import type { StoryContainerRef } from "../../ipc/generated/StoryContainerRef";
import type { Ref } from "./model";

export type DrawerState = { kind: "card" | "beat" | "sequence"; id: string } | null;

export type DialogState =
  | { type: "deleteAct"; actId: string }
  | { type: "deleteSequence"; sequenceId: string }
  | { type: "deleteLinked"; refs: Ref[] }
  | { type: "move"; refs: Ref[] }
  | { type: "build"; preselect?: string[] }
  | { type: "applyOrder" }
  | { type: "actNote"; actId: string }
  | null;

export const ZOOM_LEVELS = [50, 70, 85, 100, 125, 150] as const;

function readLocal<T>(key: string, fallback: T): T {
  try {
    const v = window.localStorage.getItem(key);
    return v === null ? fallback : (JSON.parse(v) as T);
  } catch {
    return fallback;
  }
}
function writeLocal(key: string, value: unknown) {
  try {
    window.localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Presentation preference only; ignore storage failures.
  }
}

interface StoryUi {
  /** Selected item keys ("card:<id>", "beat:<id>", "sequence:<id>"). */
  selected: string[];
  /** Key of the item currently in inline text editing (new cards/beats). */
  editingKey: string | null;
  /** Inline rename target (act or sequence id). */
  renamingId: string | null;
  /** Container that shows the inline "new sequence" field. */
  addingSequenceIn: string | null;
  addingAct: boolean;
  drawer: DrawerState;
  dialog: DialogState;
  zoom: number;
  parkingOpen: boolean;
  filter: string;
  select: (keys: string[]) => void;
  toggle: (key: string) => void;
  clearSelection: () => void;
  setEditing: (key: string | null) => void;
  setRenaming: (id: string | null) => void;
  setAddingSequenceIn: (actId: string | null) => void;
  setAddingAct: (v: boolean) => void;
  openDrawer: (d: DrawerState) => void;
  openDialog: (d: DialogState) => void;
  setZoom: (z: number) => void;
  setParkingOpen: (v: boolean) => void;
  setFilter: (q: string) => void;
}

export const useStoryUi = create<StoryUi>((set) => ({
  selected: [],
  editingKey: null,
  renamingId: null,
  addingSequenceIn: null,
  addingAct: false,
  drawer: null,
  dialog: null,
  zoom: readLocal("of.story.zoom", 100),
  parkingOpen: readLocal("of.story.parkingOpen", true),
  filter: "",
  select: (keys) => set({ selected: keys }),
  toggle: (key) => set((s) => ({ selected: s.selected.includes(key) ? s.selected.filter((k) => k !== key) : [...s.selected, key] })),
  clearSelection: () => set({ selected: [] }),
  setEditing: (key) => set({ editingKey: key }),
  setRenaming: (id) => set({ renamingId: id }),
  setAddingSequenceIn: (actId) => set({ addingSequenceIn: actId }),
  setAddingAct: (v) => set({ addingAct: v }),
  openDrawer: (d) => set({ drawer: d }),
  openDialog: (d) => set({ dialog: d }),
  setZoom: (z) => {
    writeLocal("of.story.zoom", z);
    set({ zoom: z });
  },
  setParkingOpen: (v) => {
    writeLocal("of.story.parkingOpen", v);
    set({ parkingOpen: v });
  },
  setFilter: (q) => set({ filter: q }),
}));

/** Parse a selection key back into a ref. */
export function keyToRef(key: string): Ref {
  const i = key.indexOf(":");
  return { kind: key.slice(0, i) as Ref["kind"], id: key.slice(i + 1) };
}

/** Container a new card should go into when nothing more specific is chosen. */
export type CreateTarget = { container: StoryContainerRef | null; index?: number };
