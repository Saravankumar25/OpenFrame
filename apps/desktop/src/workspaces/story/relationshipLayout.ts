// Layout of the character relationship maps. A pure function from characters and relationships to
// positions, so the same input always gives the same picture and the rules are testable:
//  - individual map (a focus character): the character in the middle, only their direct relationships
//    and the characters on the other end, on a ring (up to 8) or in two columns (more);
//  - whole-film map: layers from the character with the most relationships outward, each layer
//    ordered to keep lines from crossing, one block per group of connected characters.
// Characters are boxes holding their name, so no line can cross a name, and every relationship type
// sits on its own line in a spot clear of other labels and of the boxes.

export interface MapNodeIn {
  id: string;
  name: string;
}
export interface MapRelIn {
  id: string;
  from: string;
  to: string;
  label: string;
}
export interface MapBox {
  /** Centre of the box. */
  x: number;
  y: number;
  w: number;
  h: number;
  text: string;
}
export interface MapEdge {
  path: string;
  label: { x: number; y: number; w: number; h: number; text: string };
}
export interface RelationshipMapLayout {
  W: number;
  H: number;
  boxes: Map<string, MapBox>;
  edges: Map<string, MapEdge>;
}

const BOX_H = 34;
const LABEL_H = 16;
const MARGIN = 28;
const GAP_X = 30;
const LAYER_GAP = 170;
const COMPONENT_GAP = 56;
const MAX_NAME = 18;
const MAX_LABEL = 24;
const RING_MAX = 8;
const T_STEPS = [0.5, 0.44, 0.56, 0.38, 0.62, 0.32, 0.68, 0.26, 0.74, 0.2, 0.8];

type Pt = { x: number; y: number };
type Rect = { x: number; y: number; w: number; h: number };

const shortText = (s: string, max: number) => (s.length > max ? s.slice(0, max - 1) + "…" : s);
const boxWidth = (text: string) => 26 + 8 * text.length;
const labelWidth = (text: string) => Math.round(10 + 6.3 * text.length);

function makeBox(n: MapNodeIn, x: number, y: number): MapBox {
  const text = shortText(n.name.toUpperCase(), MAX_NAME);
  return { x, y, w: boxWidth(text), h: BOX_H, text };
}

/** Where a ray from the centre of the box towards a point leaves the box. */
function exitPoint(b: Rect, to: Pt): Pt {
  const dx = to.x - b.x;
  const dy = to.y - b.y;
  const s = Math.min(dx === 0 ? Infinity : b.w / 2 / Math.abs(dx), dy === 0 ? Infinity : b.h / 2 / Math.abs(dy));
  return Number.isFinite(s) ? { x: b.x + dx * s, y: b.y + dy * s } : { x: b.x, y: b.y };
}

const overlaps = (a: Rect, b: Rect, pad = 3) => Math.abs(a.x - b.x) * 2 < a.w + b.w + pad * 2 && Math.abs(a.y - b.y) * 2 < a.h + b.h + pad * 2;

const quad = (a: Pt, c: Pt, b: Pt) => `M ${a.x} ${a.y} Q ${c.x} ${c.y} ${b.x} ${b.y}`;
const quadAt = (a: Pt, c: Pt, b: Pt, t: number): Pt => ({
  x: (1 - t) * (1 - t) * a.x + 2 * (1 - t) * t * c.x + t * t * b.x,
  y: (1 - t) * (1 - t) * a.y + 2 * (1 - t) * t * c.y + t * t * b.y,
});

/** The first spot that keeps a label clear of the other labels and of every box. */
function pickSpot(spots: Pt[], w: number, placed: Rect[], boxes: Rect[]): Pt {
  for (const p of spots) {
    const r = { x: p.x, y: p.y, w, h: LABEL_H };
    if (!placed.some((q) => overlaps(r, q)) && !boxes.some((q) => overlaps(r, q))) return p;
  }
  return spots[0];
}

const byName = (a: MapNodeIn, b: MapNodeIn) => a.name.localeCompare(b.name) || a.id.localeCompare(b.id);

/** Lines between two boxes that end on the box edges; parallel relationships bend to alternate sides. */
function freeEdge(a: MapBox, b: MapBox, k: number, text: string, placed: Rect[], boxes: Rect[]): MapEdge {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const len = Math.hypot(dx, dy) || 1;
  const bend = k === 0 ? 0 : Math.ceil(k / 2) * 40 * (k % 2 ? 1 : -1);
  const c = { x: (a.x + b.x) / 2 + (-dy / len) * bend, y: (a.y + b.y) / 2 + (dx / len) * bend };
  const p0 = exitPoint(a, c);
  const p1 = exitPoint(b, c);
  const w = labelWidth(text);
  const at = pickSpot(
    T_STEPS.map((t) => quadAt(p0, c, p1, t)),
    w,
    placed,
    boxes,
  );
  placed.push({ x: at.x, y: at.y, w, h: LABEL_H });
  return { path: quad(p0, c, p1), label: { x: at.x, y: at.y, w, h: LABEL_H, text } };
}

function layoutFocus(nodes: MapNodeIn[], rels: MapRelIn[], focus: string): RelationshipMapLayout {
  const own = rels.filter((r) => r.from !== r.to && (r.from === focus || r.to === focus));
  const ends = new Set(own.map((r) => (r.from === focus ? r.to : r.from)));
  const centre = nodes.find((n) => n.id === focus);
  const others = nodes.filter((n) => ends.has(n.id)).sort(byName);
  const n = others.length;
  const boxes = new Map<string, MapBox>();
  let W: number;
  let H: number;
  if (n <= RING_MAX) {
    const rx = 330;
    const ry = n <= 2 ? 110 : 160;
    W = 2 * (rx + 90 + MARGIN);
    H = 2 * (ry + BOX_H / 2 + MARGIN);
    const start = n < 3 ? 0 : -Math.PI / 2;
    others.forEach((o, i) => boxes.set(o.id, makeBox(o, W / 2 + rx * Math.cos(start + (2 * Math.PI * i) / n), H / 2 + ry * Math.sin(start + (2 * Math.PI * i) / n))));
  } else {
    const D = 390;
    const rowGap = 62;
    const left = Math.ceil(n / 2);
    W = 2 * (D + 90 + MARGIN);
    H = left * rowGap + 2 * MARGIN;
    others.forEach((o, i) => {
      const onLeft = i < left;
      const idx = onLeft ? i : i - left;
      const cnt = onLeft ? left : n - left;
      boxes.set(o.id, makeBox(o, W / 2 + (onLeft ? -D : D), H / 2 + (idx - (cnt - 1) / 2) * rowGap));
    });
  }
  if (centre) boxes.set(centre.id, makeBox(centre, W / 2, H / 2));
  return { W, H, boxes, edges: routeFree(own, boxes) };
}

function routeFree(rels: MapRelIn[], boxes: Map<string, MapBox>): Map<string, MapEdge> {
  const all = [...boxes.values()];
  const placed: Rect[] = [];
  const pairs = new Map<string, number>();
  const edges = new Map<string, MapEdge>();
  for (const r of rels) {
    const a = boxes.get(r.from);
    const b = boxes.get(r.to);
    if (!a || !b || r.from === r.to) continue;
    const key = [r.from, r.to].sort().join("|");
    const k = pairs.get(key) ?? 0;
    pairs.set(key, k + 1);
    edges.set(r.id, freeEdge(a, b, k, shortText(r.label, MAX_LABEL), placed, all));
  }
  return edges;
}

function layoutLayers(nodes: MapNodeIn[], rels: MapRelIn[]): RelationshipMapLayout {
  const ids = new Set(nodes.map((n) => n.id));
  const live = rels.filter((r) => r.from !== r.to && ids.has(r.from) && ids.has(r.to));
  const adj = new Map<string, Set<string>>(nodes.map((n) => [n.id, new Set<string>()]));
  const deg = new Map<string, number>(nodes.map((n) => [n.id, 0]));
  for (const r of live) {
    adj.get(r.from)!.add(r.to);
    adj.get(r.to)!.add(r.from);
    deg.set(r.from, deg.get(r.from)! + 1);
    deg.set(r.to, deg.get(r.to)! + 1);
  }
  const byId = new Map(nodes.map((n) => [n.id, n]));
  // Most relationships first, then by name, so the order never depends on the order of the input.
  const rank = (a: MapNodeIn, b: MapNodeIn) => deg.get(b.id)! - deg.get(a.id)! || byName(a, b);
  const sorted = nodes.filter((n) => deg.get(n.id)! > 0).sort(rank);

  const seen = new Set<string>();
  const blocks: string[][][] = []; // block > layer > ids
  for (const root of sorted) {
    if (seen.has(root.id)) continue;
    seen.add(root.id);
    const layers: string[][] = [[root.id]];
    for (let cur = layers[0]; cur.length > 0; ) {
      const next: string[] = [];
      for (const id of cur) {
        for (const nb of [...adj.get(id)!].map((x) => byId.get(x)!).sort(rank)) {
          if (!seen.has(nb.id)) {
            seen.add(nb.id);
            next.push(nb.id);
          }
        }
      }
      if (next.length === 0) break;
      // Order the layer by where its relationships sit in the layer above, to keep lines from crossing.
      const above = new Map(cur.map((id, i) => [id, i]));
      const bary = (id: string) => {
        const xs = [...adj.get(id)!].filter((x) => above.has(x)).map((x) => above.get(x)!);
        return xs.reduce((s, v) => s + v, 0) / (xs.length || 1);
      };
      next.sort((a, b) => bary(a) - bary(b) || rank(byId.get(a)!, byId.get(b)!));
      layers.push(next);
      cur = next;
    }
    blocks.push(layers);
  }

  const boxes = new Map<string, MapBox>();
  const layerOf = new Map<string, number>();
  const rowWidth = (row: string[]) => row.reduce((s, id) => s + makeBox(byId.get(id)!, 0, 0).w, 0) + GAP_X * (row.length - 1);
  const W = Math.max(760, ...blocks.flat().map((r) => rowWidth(r) + 2 * MARGIN));
  let y = MARGIN;
  blocks.forEach((layers, bi) => {
    const hasArc = live.some((r) => layers.some((l) => l.includes(r.from) && l.includes(r.to)));
    y += hasArc ? 96 : 0;
    layers.forEach((row, li) => {
      let x = (W - rowWidth(row)) / 2;
      for (const id of row) {
        const b = makeBox(byId.get(id)!, 0, y + BOX_H / 2);
        b.x = x + b.w / 2;
        x += b.w + GAP_X;
        boxes.set(id, b);
        layerOf.set(id, bi * 1000 + li);
      }
      y += li === layers.length - 1 ? BOX_H : LAYER_GAP;
    });
    y += COMPONENT_GAP;
  });
  const H = y - COMPONENT_GAP + MARGIN;

  const all = [...boxes.values()];
  const placed: Rect[] = [];
  const pairs = new Map<string, number>();
  const edges = new Map<string, MapEdge>();
  for (const r of live) {
    const a = boxes.get(r.from);
    const b = boxes.get(r.to);
    if (!a || !b) continue;
    const key = [r.from, r.to].sort().join("|");
    const k = pairs.get(key) ?? 0;
    pairs.set(key, k + 1);
    const text = shortText(r.label, MAX_LABEL);
    const w = labelWidth(text);
    if (layerOf.get(r.from) === layerOf.get(r.to)) {
      // Same layer: an arc over the top of the row, high enough to clear the characters between.
      const p0 = { x: a.x, y: a.y - a.h / 2 };
      const p1 = { x: b.x, y: b.y - b.h / 2 };
      const lifts = [34, 58, 82];
      const spots = lifts.map((l) => ({ x: (p0.x + p1.x) / 2, y: p0.y - l }));
      const at = pickSpot(spots, w, placed, all);
      const lift = p0.y - at.y;
      placed.push({ x: at.x, y: at.y, w, h: LABEL_H });
      edges.set(r.id, { path: quad(p0, { x: at.x, y: p0.y - 2 * lift }, p1), label: { x: at.x, y: at.y, w, h: LABEL_H, text } });
    } else {
      // Next layer: from the bottom of the upper character to the top of the lower one, so a line never runs through a row.
      const [up, down] = a.y < b.y ? [a, b] : [b, a];
      const flip = up === b;
      const p0 = { x: up.x, y: up.y + up.h / 2 };
      const p1 = { x: down.x, y: down.y - down.h / 2 };
      const dx = p1.x - p0.x;
      const dy = p1.y - p0.y;
      const len = Math.hypot(dx, dy) || 1;
      const bend = k === 0 ? 0 : Math.ceil(k / 2) * 40 * (k % 2 ? 1 : -1);
      const c = { x: (p0.x + p1.x) / 2 + (-dy / len) * bend, y: (p0.y + p1.y) / 2 + (dx / len) * bend };
      const at = pickSpot(
        T_STEPS.map((t) => quadAt(p0, c, p1, t)),
        w,
        placed,
        all,
      );
      placed.push({ x: at.x, y: at.y, w, h: LABEL_H });
      // The arrow points at the character the relationship goes to, so a line drawn upwards is reversed.
      edges.set(r.id, { path: flip ? quad(p1, c, p0) : quad(p0, c, p1), label: { x: at.x, y: at.y, w, h: LABEL_H, text } });
    }
  }
  return { W, H, boxes, edges };
}

/** With a focus id: that character and their direct relationships only. Without: the whole film. */
export function layoutRelationshipMap(nodes: MapNodeIn[], rels: MapRelIn[], focus?: string | null): RelationshipMapLayout {
  return focus ? layoutFocus(nodes, rels, focus) : layoutLayers(nodes, rels);
}
