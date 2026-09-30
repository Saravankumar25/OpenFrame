import { describe, expect, it } from "vitest";
import type { StoryBoardState } from "../../ipc/generated/StoryBoardState";
import type { StoryItem } from "../../ipc/generated/StoryItem";
import {
  ACT,
  PARKING,
  SEQ,
  boardOrder,
  canDrop,
  childrenOf,
  dropBeside,
  dropInto,
  flattenVisible,
  headingIsValid,
  isNoop,
  locate,
  matchesFilter,
  moveDestinations,
  stepHint,
} from "./model";

const card = (id: string, desc: string, parentType: "act" | "sequence" | "parking", parentId: string | null, heading: string | null = null): StoryItem => ({
  kind: "card",
  id,
  episodeId: null,
  parentType,
  parentId,
  shortDescription: desc,
  sceneHeading: heading,
  notes: null,
  color: null,
  screenplaySceneId: null,
  sourceBeatId: null,
  parkedFromType: null,
  parkedFromId: null,
  commentCount: 0,
  attachments: [],
  characterIds: [],
  rev: 1,
  createdAt: 0,
  updatedAt: 0,
});

const beat = (id: string, text: string, parentType: "act" | "sequence" | "parking", parentId: string | null): StoryItem => ({
  kind: "beat",
  id,
  episodeId: null,
  parentType,
  parentId,
  text,
  note: null,
  color: null,
  state: "active",
  convertedSceneCardId: null,
  parkedFromType: null,
  parkedFromId: null,
  commentCount: 0,
  attachments: [],
  rev: 1,
});

function fixture(): StoryBoardState {
  const s1: StoryItem = {
    kind: "sequence",
    id: "s1",
    actId: "a1",
    episodeId: null,
    title: "Hero Introduction",
    note: null,
    cardCount: 3,
    attachments: [],
    commentCount: 0,
    rev: 1,
    items: [card("c1", "Bus", "sequence", "s1"), card("c2", "House", "sequence", "s1"), card("c3", "Police station", "sequence", "s1", "INT. POLICE — DAY")],
  };
  const s2: StoryItem = { ...s1, id: "s2", title: "Chase", cardCount: 1, items: [card("c4", "Ravi runs", "sequence", "s2")] } as StoryItem;
  return {
    episodeId: null,
    acts: [
      { id: "a1", episodeId: null, title: "Act 1", note: null, cardCount: 4, rev: 1, items: [s1, beat("b1", "turn", "act", "a1"), s2] },
      { id: "a2", episodeId: null, title: "Act 2", note: null, cardCount: 1, rev: 1, items: [card("c5", "Rain", "act", "a2")] },
    ],
    parking: [card("p1", "Dream at the lake", "parking", null)],
    unassigned: [],
    collapsedIds: [],
    cardCount: 6,
    beatCount: 1,
    sequenceCount: 2,
  };
}

const ref = (kind: "card" | "beat" | "sequence", id: string) => ({ kind, id });

describe("story model", () => {
  it("locates items and children in any container", () => {
    const b = fixture();
    expect(locate(b, ref("card", "c2"))?.index).toBe(1);
    expect(locate(b, ref("card", "c2"))?.container).toEqual(SEQ("s1"));
    expect(childrenOf(b, ACT("a1")).map((i) => i.id)).toEqual(["s1", "b1", "s2"]);
    expect(childrenOf(b, PARKING).map((i) => i.id)).toEqual(["p1"]);
    expect(locate(b, ref("card", "nope"))).toBeNull();
  });

  it("computes insertion before/after a sibling, skipping moving items", () => {
    const b = fixture();
    const moving = new Set(["card:c1"]);
    expect(dropBeside(b, ref("card", "c3"), "before", moving)).toEqual({ target: SEQ("s1"), before: ref("card", "c3") });
    expect(dropBeside(b, ref("card", "c3"), "after", moving)).toEqual({ target: SEQ("s1"), before: null });
    // Dropping after c2 when c1 is moving: before c3.
    expect(dropBeside(b, ref("card", "c2"), "after", moving)).toEqual({ target: SEQ("s1"), before: ref("card", "c3") });
    expect(dropInto(b, SEQ("s2"), true, moving)).toEqual({ target: SEQ("s2"), before: ref("card", "c4") });
    expect(dropInto(b, PARKING, false, moving)).toEqual({ target: PARKING, before: null });
  });

  it("only lets sequences into acts and never into themselves", () => {
    expect(canDrop(["sequence"], ACT("a2"), new Set(["sequence:s1"]))).toBe(true);
    expect(canDrop(["sequence"], SEQ("s2"), new Set(["sequence:s1"]))).toBe(false);
    expect(canDrop(["sequence"], PARKING, new Set(["sequence:s1"]))).toBe(false);
    expect(canDrop(["card"], PARKING, new Set(["card:c1"]))).toBe(true);
    expect(canDrop(["card", "sequence"], ACT("a1"), new Set())).toBe(false);
  });

  it("recognises drops that change nothing", () => {
    const b = fixture();
    expect(isNoop(b, [ref("card", "c1")], { target: SEQ("s1"), before: ref("card", "c2") })).toBe(true);
    expect(isNoop(b, [ref("card", "c3")], { target: SEQ("s1"), before: null })).toBe(true);
    expect(isNoop(b, [ref("card", "c1")], { target: SEQ("s1"), before: ref("card", "c3") })).toBe(false);
    expect(isNoop(b, [ref("card", "c1")], { target: SEQ("s2"), before: null })).toBe(false);
  });

  it("steps an item up and down among siblings (keyboard reorder)", () => {
    const b = fixture();
    expect(stepHint(b, ref("card", "c2"), -1)).toEqual({ target: SEQ("s1"), before: ref("card", "c1") });
    expect(stepHint(b, ref("card", "c2"), 1)).toEqual({ target: SEQ("s1"), before: null });
    expect(stepHint(b, ref("card", "c1"), 1)).toEqual({ target: SEQ("s1"), before: ref("card", "c3") });
    expect(stepHint(b, ref("card", "c1"), -1)).toBeNull();
    expect(stepHint(b, ref("card", "c3"), 1)).toBeNull();
  });

  it("sorts a selection into board order", () => {
    const b = fixture();
    expect(boardOrder(b, [ref("card", "p1"), ref("card", "c5"), ref("card", "c1"), ref("beat", "b1")]).map((r) => r.id)).toEqual(["c1", "b1", "c5", "p1"]);
  });

  it("flattens visible items and respects collapse", () => {
    const b = fixture();
    const all = flattenVisible(b, new Set(), () => true).map((f) => f.ref.id);
    expect(all).toEqual(["s1", "c1", "c2", "c3", "b1", "s2", "c4", "c5", "p1"]);
    const collapsed = flattenVisible(b, new Set(["s1", "a2"]), () => true).map((f) => f.ref.id);
    expect(collapsed).toEqual(["s1", "b1", "s2", "c4", "p1"]);
  });

  it("filters by text without dropping containers that hold matches", () => {
    const b = fixture();
    const s1 = b.acts[0].items[0];
    expect(matchesFilter(s1, "police")).toBe(true);
    expect(matchesFilter(s1, "lake")).toBe(false);
    expect(matchesFilter(b.parking[0], "LAKE")).toBe(true);
  });

  it("mirrors the Rust screenplay heading rule", () => {
    expect(headingIsValid("INT. POLICE STATION — NIGHT")).toBe(true);
    expect(headingIsValid("ext. street - day")).toBe(true);
    expect(headingIsValid("INT./EXT. CAR - MOVING")).toBe(true);
    expect(headingIsValid("I/E TRAIN")).toBe(true);
    expect(headingIsValid("INT.")).toBe(false);
    expect(headingIsValid("")).toBe(false);
    expect(headingIsValid(null)).toBe(false);
    expect(headingIsValid("Bus station at night")).toBe(false);
    expect(headingIsValid("INTERIOR")).toBe(false);
  });

  it("offers only acts as destinations for sequences", () => {
    const b = fixture();
    expect(moveDestinations(b, ["sequence"]).map((d) => d.label)).toEqual(["Act 1", "Act 2"]);
    expect(moveDestinations(b, ["card"]).map((d) => d.label)).toEqual(["Act 1", "Hero Introduction", "Chase", "Act 2", "Parking Lot"]);
  });
});
