// Find must bring the current match on screen (owner change list 2026-10-20, item 5): the scene list followed the
// match but the page stayed where it was, because the editor's own scrollIntoView follows the browser selection,
// which sits in the Find box. The page's scroll area is now moved directly. The real page is measured by
// tests/e2e/specs/42-writing-comfort.test.mjs.

import { describe, expect, it } from "vitest";
import { computeMatches, matchIndexFrom, revealDelta, scrollMatchIntoView } from "./find";
import { docFromSnapshot, type Snapshot } from "./model";

describe("revealDelta", () => {
  const box = { top: 100, bottom: 700 };
  it("does nothing for a line already well inside the box", () => {
    expect(revealDelta({ top: 300, bottom: 318 }, box)).toBe(0);
  });
  it("scrolls down to centre a line below the box, and up for a line above it", () => {
    expect(revealDelta({ top: 1500, bottom: 1518 }, box)).toBe(1509 - 400);
    expect(revealDelta({ top: -900, bottom: -882 }, box)).toBe(-891 - 400);
  });
  it("treats a line at the very edge as hidden (it would sit under the toolbar or the Find box)", () => {
    expect(revealDelta({ top: 101, bottom: 119 }, box)).not.toBe(0);
    expect(revealDelta({ top: 680, bottom: 698 }, box)).not.toBe(0);
  });
});

describe("scrollMatchIntoView", () => {
  function setup(coords: { top: number; bottom: number }) {
    const scroller = document.createElement("div");
    scroller.className = "spx-scroll";
    const dom = document.createElement("div");
    scroller.appendChild(dom);
    scroller.getBoundingClientRect = () => ({ top: 100, bottom: 700, left: 0, right: 800, width: 800, height: 600, x: 0, y: 100, toJSON: () => ({}) });
    let top = 0;
    Object.defineProperty(scroller, "scrollTop", { get: () => top, set: (v: number) => (top = v), configurable: true });
    const asked: number[] = [];
    const view = { dom, coordsAtPos: (pos: number) => (asked.push(pos), coords) };
    return { view, asked, top: () => top };
  }
  it("scrolls the page's own scroll area to a match far below the visible part", () => {
    const t = setup({ top: 1500, bottom: 1518 });
    expect(scrollMatchIntoView(t.view, 42)).toBe(true);
    expect(t.asked).toEqual([42]);
    expect(t.top()).toBe(1109);
  });
  it("scrolls back up to a match above the visible part", () => {
    const t = setup({ top: -300, bottom: -282 });
    scrollMatchIntoView(t.view, 7);
    expect(t.top()).toBe(-291 - 400);
  });
  it("leaves the page where it is when the match is already visible", () => {
    const t = setup({ top: 300, bottom: 318 });
    scrollMatchIntoView(t.view, 7);
    expect(t.top()).toBe(0);
  });
  it("reports false when the editor has no scroll area around it", () => {
    const dom = document.createElement("div");
    expect(scrollMatchIntoView({ dom, coordsAtPos: () => ({ top: 0, bottom: 0 }) }, 1)).toBe(false);
  });
});

describe("stepping across scenes and replacing", () => {
  const script: Snapshot = [
    { id: "s1", heading: "INT. DEPOT - NIGHT", elements: [{ id: "a", type: "action", text: "A torch burns." }] },
    { id: "s2", heading: "EXT. ROAD - NIGHT", elements: [{ id: "b", type: "action", text: "Torch light. A torch." }] },
  ];
  const doc = docFromSnapshot(script);
  const all = computeMatches(doc, { query: "torch", matchCase: false, wholeWord: false }, true);

  it("the whole script has a match in each scene, in order, so stepping crosses scenes", () => {
    expect(all).toHaveLength(3);
    expect(all[0].from).toBeLessThan(all[1].from);
    expect(all[1].from).toBeLessThan(all[2].from);
  });
  it("match case and whole word change the count", () => {
    expect(computeMatches(doc, { query: "Torch", matchCase: true, wholeWord: false }, true)).toHaveLength(1);
    expect(computeMatches(doc, { query: "torc", matchCase: false, wholeWord: true }, true)).toHaveLength(0);
    expect(computeMatches(doc, { query: "torc", matchCase: false, wholeWord: false }, true)).toHaveLength(3);
  });
  it("this scene keeps the matches of that scene only", () => {
    expect(computeMatches(doc, { query: "torch", matchCase: false, wholeWord: false, sceneId: "s2" }, true)).toHaveLength(2);
  });
  it("after Replace, the next match is the first one after the replaced text, wrapping to the first", () => {
    // The first match was replaced by "a torch lamp" (twelve characters): matches before the end of it are passed over.
    const end = all[0].from + "a torch lamp".length;
    expect(matchIndexFrom(all, end)).toBe(1);
    expect(matchIndexFrom(all, all[2].to + 1)).toBe(0);
    expect(matchIndexFrom([], 5)).toBe(0);
  });
});
