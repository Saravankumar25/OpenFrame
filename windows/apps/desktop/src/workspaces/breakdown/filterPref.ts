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
