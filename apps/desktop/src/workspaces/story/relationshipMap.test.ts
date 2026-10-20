// The relationship map never draws two labels on top of each other (owner change list item 3).

import { describe, expect, it } from "vitest";
import { layoutRelationshipMap } from "./Characters";

describe("relationship map layout", () => {
  it("places the owner's example without overlapping labels", () => {
    // DIVYA friend of VIJAY, KARTHIK friend of DIVYA, RAMESH father of VIJAY: drawn in list order on a
    // circle, two lines crossed in the middle and their labels sat on the same point.
    const ids = ["divya", "vijay", "karthik", "ramesh"];
    const rels = [
      { id: "a", from: "divya", to: "vijay" },
      { id: "b", from: "karthik", to: "divya" },
      { id: "c", from: "ramesh", to: "vijay" },
    ];
    const m = layoutRelationshipMap(ids, rels);
    const labels = rels.map((r) => m.edges.get(r.id)!.label);
    for (let i = 0; i < labels.length; i++) {
      for (let j = i + 1; j < labels.length; j++) {
        const apart = Math.abs(labels[i].x - labels[j].x) > 70 || Math.abs(labels[i].y - labels[j].y) > 18;
        expect(apart).toBe(true);
      }
    }
  });

  it("bends a second relationship between the same pair away from the first", () => {
    const m = layoutRelationshipMap(["a", "b"], [
      { id: "1", from: "a", to: "b" },
      { id: "2", from: "b", to: "a" },
    ]);
    expect(m.edges.get("1")!.path).not.toBe(m.edges.get("2")!.path);
    const l1 = m.edges.get("1")!.label;
    const l2 = m.edges.get("2")!.label;
    expect(Math.hypot(l1.x - l2.x, l1.y - l2.y)).toBeGreaterThan(18);
  });

  it("keeps every character inside the drawing", () => {
    const ids = Array.from({ length: 12 }, (_, i) => "c" + i);
    const m = layoutRelationshipMap(ids, ids.slice(1).map((id, i) => ({ id: "r" + i, from: ids[0], to: id })));
    for (const p of m.pos.values()) {
      expect(p.x).toBeGreaterThan(20);
      expect(p.x).toBeLessThan(m.W - 20);
      expect(p.y).toBeGreaterThan(20);
      expect(p.y).toBeLessThan(m.H - 40);
    }
  });
});
