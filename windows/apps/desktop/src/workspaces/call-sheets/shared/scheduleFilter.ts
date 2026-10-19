// Schedule filters (Screenplay and Production Depth spec section 27; feature
// SCHED-006). A filter is view state only: it reads the schedule view and
// returns which strips match. It never calls an operation and never changes
// the view it is given, so filtering can never change project data.

import type { ScheduleStripDto, ScheduleView } from "../../../api/schedule";

export interface ScheduleFilter {
  /** Location keys (any of). */
  locations: string[];
  /** Cast keys (any of). */
  cast: string[];
  /** "D" day, "N" night, "" any. */
  dayNight: "" | "D" | "N";
  /** Page length in eighths, inclusive; null = no bound. */
  minEighths: number | null;
  maxEighths: number | null;
  scheduled: "any" | "scheduled" | "unscheduled";
  breakdown: "any" | "complete" | "incomplete";
  /** Shooting day ids (any of). */
  days: string[];
  /** Script revision state: "any", "changed" (Changed, Removed or New) or "current". */
  revision: "any" | "changed" | "current";
}

export const EMPTY_FILTER: ScheduleFilter = {
  locations: [],
  cast: [],
  dayNight: "",
  minEighths: null,
  maxEighths: null,
  scheduled: "any",
  breakdown: "any",
  days: [],
  revision: "any",
};

export function isFilterActive(f: ScheduleFilter): boolean {
  return (
    f.locations.length > 0 ||
    f.cast.length > 0 ||
    f.dayNight !== "" ||
    f.minEighths !== null ||
    f.maxEighths !== null ||
    f.scheduled !== "any" ||
    f.breakdown !== "any" ||
    f.days.length > 0 ||
    f.revision !== "any"
  );
}

export function stripMatches(s: ScheduleStripDto, f: ScheduleFilter): boolean {
  if (f.locations.length && !s.locations.some((l) => f.locations.includes(l.key))) return false;
  if (f.cast.length && !s.cast.some((c) => f.cast.includes(c.key))) return false;
  if (f.dayNight && s.dayNight !== f.dayNight) return false;
  if (f.minEighths !== null && s.pageEighths < f.minEighths) return false;
  if (f.maxEighths !== null && s.pageEighths > f.maxEighths) return false;
  if (f.scheduled === "scheduled" && !s.dayId) return false;
  if (f.scheduled === "unscheduled" && s.dayId) return false;
  if (f.breakdown === "complete" && !s.breakdownComplete) return false;
  if (f.breakdown === "incomplete" && s.breakdownComplete) return false;
  if (f.days.length && !(s.dayId && f.days.includes(s.dayId))) return false;
  if (f.revision === "changed" && s.sourceState === "Current") return false;
  if (f.revision === "current" && s.sourceState !== "Current") return false;
  return true;
}

/** Every strip of the board (days in order, then Unscheduled). */
export function allStrips(view: ScheduleView): ScheduleStripDto[] {
  const out: ScheduleStripDto[] = [];
  for (const d of view.days) for (const i of d.items) if (i.strip) out.push(i.strip);
  out.push(...view.unscheduled);
  return out;
}

/** Ids of the strips that match. */
export function matchingStripIds(view: ScheduleView, f: ScheduleFilter): Set<string> {
  return new Set(allStrips(view).filter((s) => stripMatches(s, f)).map((s) => s.id));
}

/** Choices for the filter menus, derived from the board. */
export function filterOptions(view: ScheduleView) {
  const locations = new Map<string, string>();
  const cast = new Map<string, string>();
  for (const s of allStrips(view)) {
    for (const l of s.locations) locations.set(l.key, l.name);
    for (const c of s.cast) cast.set(c.key, c.actor ? `${c.character} (${c.actor})` : c.character);
  }
  const sort = (m: Map<string, string>) => [...m.entries()].map(([value, label]) => ({ value, label })).sort((a, b) => a.label.localeCompare(b.label));
  return {
    locations: sort(locations),
    cast: sort(cast),
    days: view.days.filter((d) => !d.isOffDay).map((d) => ({ value: d.id, label: d.title })),
  };
}
