// The breakdown scene filter is a per-user view preference kept in local storage, never project
// data. A stored value the backend would refuse (an old or damaged entry) falls back to "all".

import type { BreakdownSceneFilter } from "../../ipc/generated/BreakdownSceneFilter";
import type { BreakdownCategory } from "../../ipc/generated/BreakdownCategory";

const FILTER_KEY = "openframe.breakdown.sceneFilter";
export type SavedFilter = { filter: BreakdownSceneFilter; category: BreakdownCategory | null };
const FILTERS: readonly BreakdownSceneFilter[] = ["all", "unreviewed", "suggested", "missing", "needsReview", "complete", "incomplete"];
const CATEGORIES: readonly BreakdownCategory[] = [
  "Cast", "Extras / Background", "Location / Set", "Props", "Wardrobe", "Vehicles",
  "Hair / Makeup", "Special Effects", "VFX", "Sound", "Animals",
];
/** The saved filter, or the default when storage is empty, unreadable or holds an unknown value. */
export function readFilter(): SavedFilter {
  const fallback: SavedFilter = { filter: "all", category: null };
  try {
    const v = JSON.parse(window.localStorage.getItem(FILTER_KEY) ?? "null") as Partial<SavedFilter> | null;
    if (!v || !FILTERS.includes(v.filter as BreakdownSceneFilter)) return fallback;
    const category = CATEGORIES.includes(v.category as BreakdownCategory) ? (v.category as BreakdownCategory) : null;
    return { filter: v.filter as BreakdownSceneFilter, category };
  } catch {
    return fallback;
  }
}
export function writeFilter(v: SavedFilter) {
  try {
    window.localStorage.setItem(FILTER_KEY, JSON.stringify(v));
  } catch {
    // View preference only; ignore storage failures.
  }
}

const PANEL_KEY = "openframe.breakdown.panelHidden";
/** Whether the scene breakdown panel is hidden (owner item 10); view preference only. */
export function readPanelHidden(): boolean {
  try {
    return window.localStorage.getItem(PANEL_KEY) === "true";
  } catch {
    return false;
  }
}
export function writePanelHidden(v: boolean) {
  try {
    window.localStorage.setItem(PANEL_KEY, v ? "true" : "false");
  } catch {
    // View preference only; ignore storage failures.
  }
}

const ZOOM_KEY = "openframe.breakdown.zoom";
/** The Breakdown script zoom (owner item 14); view preference only, 0.6 to 2. */
export function readZoom(): number {
  try {
    const z = Number(window.localStorage.getItem(ZOOM_KEY));
    return Number.isFinite(z) && z >= 0.6 && z <= 2 ? z : 1;
  } catch {
    return 1;
  }
}
export function writeZoom(z: number): number {
  const v = Math.round(Math.min(2, Math.max(0.6, z)) * 100) / 100;
  try {
    window.localStorage.setItem(ZOOM_KEY, String(v));
  } catch {
    // View preference only; ignore storage failures.
  }
  return v;
}
