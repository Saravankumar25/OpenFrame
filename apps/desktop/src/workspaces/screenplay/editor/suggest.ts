// Smart suggestions while typing (SCRIPT-018; Screenplay and Production Depth Specification §7).
//
// Deterministic, no AI model: the list comes from fixed screenplay terms, the live script and
// project values (Beat Board characters, cast, production locations). It only ever offers;
// text changes only when the writer accepts an item with Enter or a click, and only the part
// being typed is replaced. The list appears only with the caret at the end of the element.

import type { Node as PMNode } from "prosemirror-model";
import type { ElementType } from "../../../ipc/generated/ElementType";
import type { WritingSuggestionSources } from "../../../ipc/generated/WritingSuggestionSources";
import { blockText } from "./model";

export type SuggestKind = "prefix" | "location" | "time" | "character" | "extension" | "parenthetical" | "transition";
export type SuggestSource = "Standard" | "Script" | "Beat Board" | "Cast" | "Locations";

export interface SuggestContext {
  kind: SuggestKind;
  /** Offsets in the element text of the part being typed (replaced on accept). */
  from: number;
  to: number;
  query: string;
}

export interface SuggestItem {
  label: string;
  /** Text that replaces [from, to). */
  insert: string;
  source: SuggestSource;
  /** "Create New": keeps the typed text and adds it to the project. */
  create?: "character" | "location";
}

export const HEADING_PREFIXES = ["INT.", "EXT.", "INT./EXT.", "I/E"];
export const TIMES = ["DAY", "NIGHT", "MORNING", "EVENING", "LATER", "CONTINUOUS", "SAME", "MOMENTS LATER"];
export const EXTENSIONS = ["V.O.", "O.S.", "O.C.", "CONT'D"];
export const TRANSITIONS = ["CUT TO:", "FADE OUT.", "FADE IN:", "DISSOLVE TO:", "SMASH CUT TO:", "MATCH CUT TO:"];
const MAX_ITEMS = 8;

/** Recognised prefixes, longest first so "INT./EXT." wins over "INT.". */
const PREFIX_PATTERN = /^(INT\.\/EXT\.|EXT\.\/INT\.|INT\/EXT|I\/E\.?|INT\.|EXT\.|EST\.)\s+/i;
const TIME_SEPARATOR = / [-\u2013\u2014] /g;

/** What the writer is typing at `offset` in an element of `type`, or null for no list. */
export function contextAt(type: ElementType, text: string, offset: number): SuggestContext | null {
  if (offset !== text.length || text.includes("\n")) return null;
  switch (type) {
    case "scene_heading": {
      const p = PREFIX_PATTERN.exec(text);
      if (!p) return { kind: "prefix", from: 0, to: offset, query: text.trimStart() };
      let sep: RegExpExecArray | null = null;
      let last: RegExpExecArray | null = null;
      TIME_SEPARATOR.lastIndex = 0;
      while ((sep = TIME_SEPARATOR.exec(text)) !== null) last = sep;
      if (last && last.index >= p[0].length) {
        const from = last.index + last[0].length;
        return { kind: "time", from, to: offset, query: text.slice(from) };
      }
      return { kind: "location", from: p[0].length, to: offset, query: text.slice(p[0].length) };
    }
    case "character": {
      const open = text.lastIndexOf("(");
      if (open >= 0 && !text.slice(open).includes(")")) {
        return { kind: "extension", from: open + 1, to: offset, query: text.slice(open + 1) };
      }
      if (open >= 0) return null;
      return { kind: "character", from: 0, to: offset, query: text };
    }
    case "parenthetical": {
      const start = text.startsWith("(") ? 1 : 0;
      return { kind: "parenthetical", from: start, to: offset, query: text.slice(start) };
    }
    case "transition":
      return { kind: "transition", from: 0, to: offset, query: text };
    default:
      return null;
  }
}

/** "MEERA (V.O.)" → "MEERA". */
export function cueName(raw: string): string {
  let s = raw.trim().replace(/\^$/, "").trim().toUpperCase();
  for (;;) {
    const m = /^(.*\S)\s*\([^()]*\)\s*$/.exec(s);
    if (!m) break;
    s = m[1];
  }
  return s.replace(/\s+/g, " ");
}

/** "INT. KITCHEN - NIGHT" → { prefix: "INT.", location: "KITCHEN", time: "NIGHT" }. */
export function headingParts(heading: string): { prefix: string; location: string; time: string } {
  const h = heading.trim();
  const p = PREFIX_PATTERN.exec(h + " ");
  const prefix = p ? p[1].toUpperCase() : "";
  const rest = p ? (h + " ").slice(p[0].length).trim() : h;
  let location = rest;
  let time = "";
  TIME_SEPARATOR.lastIndex = 0;
  let m: RegExpExecArray | null;
  let last: RegExpExecArray | null = null;
  while ((m = TIME_SEPARATOR.exec(rest)) !== null) last = m;
  if (last) {
    location = rest.slice(0, last.index);
    time = rest.slice(last.index + last[0].length);
  }
  return { prefix, location: location.trim().replace(/\s+/g, " ").toUpperCase(), time: time.trim().toUpperCase() };
}

export interface ScriptValues {
  characters: string[];
  locations: string[];
  times: string[];
  prefixes: string[];
  parentheticals: string[];
}

/** What one element adds to the script values, as (list, value) pairs. */
function contributions(type: string, text: string): [keyof ScriptValues, string][] {
  switch (type) {
    case "character":
      return [["characters", cueName(text)]];
    case "scene_heading": {
      const p = headingParts(text);
      return [
        ["prefixes", p.prefix],
        ["locations", p.location],
        ["times", p.time],
      ];
    }
    case "parenthetical":
      return [["parentheticals", text.trim().replace(/^\(/, "").replace(/\)$/, "").trim()]];
    default:
      return [];
  }
}

/** Most used first; equal counts keep the order they appear in (the sort is stable). */
function byUse(counts: Map<string, number>): string[] {
  return [...counts.keys()].sort((a, b) => (counts.get(b) ?? 0) - (counts.get(a) ?? 0));
}

type Counts = Record<keyof ScriptValues, Map<string, number>>;

const KEYS: (keyof ScriptValues)[] = ["characters", "locations", "times", "prefixes", "parentheticals"];
const countCache = new WeakMap<PMNode, Counts>();
const valuesCache = new WeakMap<PMNode, ScriptValues>();

/** How often each value is used in the whole script. Cached per document version, so moving the
 * caret between elements reads the script once, not once per element. */
function countsOf(doc: PMNode): Counts {
  const hit = countCache.get(doc);
  if (hit) return hit;
  const lists = Object.fromEntries(KEYS.map((k) => [k, new Map<string, number>()])) as Counts;
  doc.forEach((n) => {
    for (const [list, value] of contributions(n.type.name, blockText(n))) {
      if (value) lists[list].set(value, (lists[list].get(value) ?? 0) + 1);
    }
  });
  countCache.set(doc, lists);
  return lists;
}

function valuesFrom(lists: Counts): ScriptValues {
  return {
    characters: byUse(lists.characters),
    locations: byUse(lists.locations),
    times: byUse(lists.times),
    prefixes: byUse(lists.prefixes),
    parentheticals: byUse(lists.parentheticals).slice(0, 30),
  };
}

/** Values already in the script, most used first. Cached per document version. */
export function scriptValues(doc: PMNode): ScriptValues {
  const hit = valuesCache.get(doc);
  if (hit) return hit;
  const v = valuesFrom(countsOf(doc));
  valuesCache.set(doc, v);
  return v;
}

/** Values in the script leaving out the element at `skipIndex` (the one being typed). */
export function scriptValuesWithout(doc: PMNode, skipIndex: number): ScriptValues {
  const all = countsOf(doc);
  if (skipIndex < 0 || skipIndex >= doc.childCount) return scriptValues(doc);
  const skip = doc.child(skipIndex);
  const own = contributions(skip.type.name, blockText(skip)).filter(([, v]) => v);
  if (own.length === 0) return scriptValues(doc);
  const lists = { ...all };
  for (const [list, value] of own) {
    const m = new Map(lists[list]);
    const n = (m.get(value) ?? 0) - 1;
    if (n > 0) m.set(value, n);
    else m.delete(value);
    lists[list] = m;
  }
  return valuesFrom(lists);
}

function rank(label: string, q: string): number {
  const l = label.toLowerCase();
  if (!q) return 1;
  if (l.startsWith(q)) return 0;
  if (l.split(/[\s./-]+/).some((w) => w.startsWith(q))) return 1;
  return -1;
}

/** The list for a context: typing filters, the exact typed value is not offered again, and
 * "Create New" is added for a new character or location name. */
export function suggestions(
  ctx: SuggestContext,
  script: ScriptValues,
  project: WritingSuggestionSources | null,
  canCreate: boolean,
): SuggestItem[] {
  const q = ctx.query.trim().toLowerCase();
  const pool: { label: string; source: SuggestSource }[] = [];
  const add = (labels: string[], source: SuggestSource) => labels.forEach((label) => pool.push({ label, source }));
  switch (ctx.kind) {
    case "prefix":
      add(HEADING_PREFIXES, "Standard");
      add(script.prefixes, "Script");
      break;
    case "location":
      add(script.locations, "Script");
      for (const l of project?.locations ?? []) pool.push({ label: l.name, source: l.source === "Story" ? "Beat Board" : "Locations" });
      break;
    case "time":
      add(TIMES, "Standard");
      add(script.times, "Script");
      break;
    case "character":
      add(script.characters, "Script");
      for (const c of project?.characters ?? []) pool.push({ label: c.name, source: c.source === "Cast" ? "Cast" : "Beat Board" });
      break;
    case "extension":
      add(EXTENSIONS, "Standard");
      break;
    case "parenthetical":
      add(script.parentheticals, "Script");
      break;
    case "transition":
      add(TRANSITIONS, "Standard");
      break;
  }
  const seen = new Set<string>();
  const ranked: { item: SuggestItem; r: number; i: number }[] = [];
  let exact = false;
  pool.forEach((p, i) => {
    const key = p.label.toLowerCase();
    if (!p.label.trim() || seen.has(key)) return;
    seen.add(key);
    if (key === q) {
      exact = true;
      return;
    }
    const r = rank(p.label, q);
    if (r < 0) return;
    ranked.push({ item: { label: p.label, insert: insertFor(ctx, p.label), source: p.source }, r, i });
  });
  ranked.sort((a, b) => a.r - b.r || a.i - b.i);
  const items = ranked.slice(0, MAX_ITEMS).map((x) => x.item);
  const creatable = ctx.kind === "character" ? "character" : ctx.kind === "location" ? "location" : null;
  if (creatable && canCreate && q && !exact) {
    const name = ctx.query.trim().toUpperCase();
    items.push({ label: `Create New: ${name}`, insert: ctx.query, source: creatable === "character" ? "Beat Board" : "Locations", create: creatable });
  }
  return items;
}

/** What accepting an item writes: a prefix leaves room for the location, a location leads into
 * the time of day, an extension closes its bracket. */
export function insertFor(ctx: Pick<SuggestContext, "kind" | "from">, label: string): string {
  switch (ctx.kind) {
    case "prefix":
      return `${label} `;
    case "location":
      return `${label} - `;
    case "extension":
      return `${label})`;
    case "parenthetical":
      return ctx.from === 0 ? `(${label})` : `${label})`;
    default:
      return label;
  }
}

/** Apply an accepted item to the element text: only [from, to) changes. */
export function applySuggestion(text: string, ctx: SuggestContext, item: SuggestItem): string {
  return text.slice(0, ctx.from) + item.insert + text.slice(ctx.to);
}
