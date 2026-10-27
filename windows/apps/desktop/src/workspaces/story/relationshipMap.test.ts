// The relationship maps: the same input always gives the same picture, no character box or label
// overlaps another, and the individual map holds only the character's direct relationships.

import { describe, expect, it } from "vitest";
import { layoutRelationshipMap, type MapNodeIn, type MapRelIn } from "./relationshipLayout";

const people = (n: number): MapNodeIn[] => Array.from({ length: n }, (_, i) => ({ id: "c" + i, name: "Person " + String.fromCharCode(65 + (i % 26)) + i }));
const rel = (id: string, from: string, to: string, label = "friend of"): MapRelIn => ({ id, from, to, label });

type R = { x: number; y: number; w: number; h: number };
const hit = (a: R, b: R) => Math.abs(a.x - b.x) * 2 < a.w + b.w && Math.abs(a.y - b.y) * 2 < a.h + b.h;

function expectClean(m: ReturnType<typeof layoutRelationshipMap>) {
  const boxes = [...m.boxes.values()];
  const labels = [...m.edges.values()].map((e) => e.label);
  for (let i = 0; i < boxes.length; i++) {
    expect(boxes[i].x - boxes[i].w / 2).toBeGreaterThanOrEqual(0);
    expect(boxes[i].x + boxes[i].w / 2).toBeLessThanOrEqual(m.W);
    expect(boxes[i].y - boxes[i].h / 2).toBeGreaterThanOrEqual(0);
    expect(boxes[i].y + boxes[i].h / 2).toBeLessThanOrEqual(m.H);
    for (let j = i + 1; j < boxes.length; j++) expect(hit(boxes[i], boxes[j])).toBe(false);
  }
  for (let i = 0; i < labels.length; i++) {
    for (const b of boxes) expect(hit(labels[i], b)).toBe(false);
    for (let j = i + 1; j < labels.length; j++) expect(hit(labels[i], labels[j])).toBe(false);
  }
}

describe("individual relationship map", () => {
  const nodes = people(6);
  const rels = [rel("a", "c0", "c1"), rel("b", "c2", "c0", "father of"), rel("c", "c1", "c3"), rel("d", "c4", "c5")];

  it("holds only the character and their direct relationships", () => {
    const m = layoutRelationshipMap(nodes, rels, "c0");
    expect([...m.boxes.keys()].sort()).toEqual(["c0", "c1", "c2"]);
    expect([...m.edges.keys()].sort()).toEqual(["a", "b"]);
  });

  it("puts the character in the middle", () => {
    const m = layoutRelationshipMap(nodes, rels, "c0");
    expect(m.boxes.get("c0")).toMatchObject({ x: m.W / 2, y: m.H / 2 });
  });

  it("gives the same picture for the same input in any input order", () => {
    const a = layoutRelationshipMap(nodes, rels, "c0");
    const b = layoutRelationshipMap([...nodes].reverse(), [...rels].reverse(), "c0");
    expect([...a.boxes.entries()].sort()).toEqual([...b.boxes.entries()].sort());
    expect(a.W).toBe(b.W);
  });

  it("keeps every box and label apart for 1 to 30 relationships", () => {
    for (const n of [1, 2, 3, 5, 8, 9, 14, 30]) {
      const ns = people(n + 1);
      const rs = ns.slice(1).map((x, i) => rel("r" + i, i % 2 ? x.id : "c0", i % 2 ? "c0" : x.id, i % 3 ? "colleague of" : "mother of"));
      expectClean(layoutRelationshipMap(ns, rs, "c0"));
    }
  });

  it("bends a second relationship between the same pair apart from the first", () => {
    const m = layoutRelationshipMap(people(2), [rel("1", "c0", "c1"), rel("2", "c1", "c0", "rival of")], "c0");
    expect(m.edges.get("1")!.path).not.toBe(m.edges.get("2")!.path);
    expectClean(m);
  });

  it("has no edges for a character without relationships", () => {
    const m = layoutRelationshipMap(nodes, rels, "c5-none");
    expect(m.edges.size).toBe(0);
  });
});

describe("whole-film relationship map", () => {
  it("places the character with the most relationships on the top layer, one layer per step", () => {
    const ns = people(5);
    const rs = [rel("1", "c0", "c1"), rel("2", "c0", "c2"), rel("3", "c0", "c3"), rel("4", "c3", "c4")];
    const m = layoutRelationshipMap(ns, rs);
    const y = (id: string) => m.boxes.get(id)!.y;
    expect(y("c0")).toBeLessThan(y("c1"));
    expect(y("c1")).toBe(y("c2"));
    expect(y("c3")).toBeLessThan(y("c4"));
    expectClean(m);
  });

  it("leaves out characters without relationships", () => {
    const m = layoutRelationshipMap(people(4), [rel("1", "c0", "c1")]);
    expect([...m.boxes.keys()].sort()).toEqual(["c0", "c1"]);
  });

  it("gives the same picture for the same input in any input order", () => {
    const ns = people(7);
    const rs = [rel("1", "c0", "c1"), rel("2", "c1", "c2"), rel("3", "c0", "c3"), rel("4", "c4", "c5"), rel("5", "c3", "c1"), rel("6", "c5", "c6")];
    const a = layoutRelationshipMap(ns, rs);
    const b = layoutRelationshipMap([...ns].reverse(), [...rs].reverse());
    expect([...a.boxes.entries()].sort()).toEqual([...b.boxes.entries()].sort());
    expect([...a.edges.keys()].sort()).toEqual([...b.edges.keys()].sort());
  });

  it("keeps boxes and labels apart for a star, a ring, parallel and same-layer relationships", () => {
    const ns = people(12);
    const star = ns.slice(1, 9).map((x, i) => rel("s" + i, "c0", x.id, i % 2 ? "friend of" : "colleague of"));
    const extra = [rel("p1", "c0", "c1", "rival of"), rel("p2", "c1", "c0", "mentor of"), rel("l1", "c1", "c2"), rel("l2", "c1", "c3"), rel("l3", "c9", "c10"), rel("l4", "c10", "c11")];
    expectClean(layoutRelationshipMap(ns, [...star, ...extra]));
  });

  it("shows every relationship and shortens a very long type", () => {
    const long = "is the second cousin once removed of";
    const m = layoutRelationshipMap(people(2), [rel("1", "c0", "c1", long)]);
    expect(m.edges.get("1")!.label.text.length).toBeLessThanOrEqual(24);
    expect(m.edges.size).toBe(1);
  });
});
