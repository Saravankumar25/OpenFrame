import { describe, expect, it } from "vitest";
import type { ScheduleStripDto, ScheduleView } from "../../../api/schedule";
import { EMPTY_FILTER, filterOptions, isFilterActive, matchingStripIds, stripMatches } from "./scheduleFilter";

function strip(id: string, over: Partial<ScheduleStripDto> = {}): ScheduleStripDto {
  return {
    id,
    sceneId: `sc-${id}`,
    lineageId: `L-${id}`,
    number: Number(id.replace(/\D/g, "")) || 1,
    heading: "INT. POLICE STATION - NIGHT",
    intExt: "INT",
    dayNight: "N",
    ieLabel: "INT·N",
    stripClass: "in",
    locationName: "Police Station",
    locations: [{ key: "loc:police", name: "Police Station", address: null, status: "Confirmed", locationId: "police", fromBreakdown: true }],
    synopsis: "",
    cast: [{ key: "char:arjun", character: "Arjun", actor: "Karthik Menon", initials: "AR" }],
    pageEighths: 8,
    pagesLabel: "1",
    pagesOverridden: false,
    estimatedMinutes: 60,
    dayId: null,
    sourceState: "Current",
    changeKinds: [],
    missingFromSource: false,
    notes: null,
    startMinutes: null,
    startLabel: null,
    breakdownComplete: true,
    rev: 1,
    ...over,
  };
}

const s1 = strip("s1", { dayId: "d1" });
const s2 = strip("s2", {
  dayNight: "D",
  locations: [{ key: "loc:rail", name: "Old Railway Station", address: null, status: null, locationId: null, fromBreakdown: true }],
  cast: [{ key: "char:meera", character: "Meera", actor: null, initials: "ME" }],
  pageEighths: 30,
  breakdownComplete: false,
  sourceState: "Changed",
});

const view = {
  days: [{ id: "d1", title: "Shoot Day 1", isOffDay: false, items: [{ kind: "strip", strip: s1, marker: null }] }],
  unscheduled: [s2],
} as unknown as ScheduleView;

describe("schedule filters", () => {
  it("match on every dimension of spec section 27", () => {
    expect(stripMatches(s1, { ...EMPTY_FILTER, locations: ["loc:police"] })).toBe(true);
    expect(stripMatches(s2, { ...EMPTY_FILTER, locations: ["loc:police"] })).toBe(false);
    expect(stripMatches(s2, { ...EMPTY_FILTER, cast: ["char:meera"] })).toBe(true);
    expect(stripMatches(s1, { ...EMPTY_FILTER, dayNight: "D" })).toBe(false);
    expect(stripMatches(s2, { ...EMPTY_FILTER, minEighths: 25 })).toBe(true);
    expect(stripMatches(s1, { ...EMPTY_FILTER, maxEighths: 8 })).toBe(true);
    expect(stripMatches(s2, { ...EMPTY_FILTER, scheduled: "scheduled" })).toBe(false);
    expect(stripMatches(s2, { ...EMPTY_FILTER, scheduled: "unscheduled" })).toBe(true);
    expect(stripMatches(s2, { ...EMPTY_FILTER, breakdown: "incomplete" })).toBe(true);
    expect(stripMatches(s1, { ...EMPTY_FILTER, days: ["d1"] })).toBe(true);
    expect(stripMatches(s2, { ...EMPTY_FILTER, days: ["d1"] })).toBe(false);
    expect(stripMatches(s2, { ...EMPTY_FILTER, revision: "changed" })).toBe(true);
    expect(stripMatches(s1, { ...EMPTY_FILTER, revision: "changed" })).toBe(false);
    expect(isFilterActive(EMPTY_FILTER)).toBe(false);
  });

  it("never changes the schedule it filters", () => {
    const frozen = structuredClone(view);
    const deepFreeze = (o: unknown): void => {
      if (o && typeof o === "object") {
        Object.freeze(o);
        for (const v of Object.values(o)) deepFreeze(v);
      }
    };
    deepFreeze(view);
    const ids = matchingStripIds(view, { ...EMPTY_FILTER, cast: ["char:arjun"], dayNight: "N" });
    expect([...ids]).toEqual(["s1"]);
    expect(filterOptions(view).cast.map((c) => c.label)).toEqual(["Arjun (Karthik Menon)", "Meera"]);
    expect(view).toEqual(frozen);
  });
});
