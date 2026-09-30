// Pure Story Board helpers (no React): locating items, computing drop targets,
// keyboard reordering, filtering and the heading rule used by Build Screenplay.
// The board data always comes from Rust; these helpers only interpret it.

import type { StoryActDto } from "../../ipc/generated/StoryActDto";
import type { StoryBoardState } from "../../ipc/generated/StoryBoardState";
import type { StoryContainerRef } from "../../ipc/generated/StoryContainerRef";
import type { StoryItem } from "../../ipc/generated/StoryItem";
import type { StorySequenceDto } from "../../ipc/generated/StorySequenceDto";

export type ItemKind = "sequence" | "beat" | "card";
export interface Ref {
  kind: ItemKind;
  id: string;
}

export const refKey = (r: Ref): string => `${r.kind}:${r.id}`;
export const toRef = (i: StoryItem): Ref => ({ kind: i.kind, id: i.id });
export const containerKey = (c: StoryContainerRef): string => `${c.parentType}:${c.parentId ?? ""}`;
export const sameContainer = (a: StoryContainerRef, b: StoryContainerRef): boolean =>
  a.parentType === b.parentType && (a.parentId ?? null) === (b.parentId ?? null);

export const ACT = (id: string): StoryContainerRef => ({ parentType: "act", parentId: id });
export const SEQ = (id: string): StoryContainerRef => ({ parentType: "sequence", parentId: id });
export const PARKING: StoryContainerRef = { parentType: "parking", parentId: null };
export const UNASSIGNED: StoryContainerRef = { parentType: "unassigned", parentId: null };

export function sequences(board: StoryBoardState): StorySequenceDto[] {
  const out: StorySequenceDto[] = [];
  const take = (items: StoryItem[]) => items.forEach((i) => i.kind === "sequence" && out.push(i));
  board.acts.forEach((a) => take(a.items));
  take(board.unassigned);
  return out;
}

export function childrenOf(board: StoryBoardState, c: StoryContainerRef): StoryItem[] {
  switch (c.parentType) {
    case "act":
      return board.acts.find((a) => a.id === c.parentId)?.items ?? [];
    case "sequence":
      return sequences(board).find((s) => s.id === c.parentId)?.items ?? [];
    case "parking":
      return board.parking;
    case "unassigned":
      return board.unassigned;
  }
}

/** Every container on the board with its children (acts, sequences, parking, unassigned). */
export function containers(board: StoryBoardState): { container: StoryContainerRef; items: StoryItem[] }[] {
  const out: { container: StoryContainerRef; items: StoryItem[] }[] = [];
  for (const a of board.acts) {
    out.push({ container: ACT(a.id), items: a.items });
  }
  for (const s of sequences(board)) out.push({ container: SEQ(s.id), items: s.items });
  out.push({ container: PARKING, items: board.parking });
  out.push({ container: UNASSIGNED, items: board.unassigned });
  return out;
}

export interface Located {
  item: StoryItem;
  container: StoryContainerRef;
  index: number;
  siblings: StoryItem[];
}

export function locate(board: StoryBoardState, ref: Ref): Located | null {
  for (const { container, items } of containers(board)) {
    const index = items.findIndex((i) => i.kind === ref.kind && i.id === ref.id);
    if (index >= 0) return { item: items[index], container, index, siblings: items };
  }
  return null;
}

export function itemLabel(i: StoryItem): string {
  const first = (s: string) => s.trim().split("\n")[0]?.trim() ?? "";
  if (i.kind === "sequence") return i.title;
  if (i.kind === "beat") return first(i.text) || "Untitled beat";
  return first(i.shortDescription) || i.sceneHeading || "Untitled scene";
}

export function containerLabel(board: StoryBoardState, c: StoryContainerRef): string {
  if (c.parentType === "parking") return "Parking Lot";
  if (c.parentType === "unassigned") return "Unassigned";
  if (c.parentType === "act") return board.acts.find((a) => a.id === c.parentId)?.title ?? "Act";
  return sequences(board).find((s) => s.id === c.parentId)?.title ?? "Sequence";
}

export function actCountLabel(a: StoryActDto): string {
  const scenes = a.cardCount;
  const items = a.items.length;
  if (scenes > 0) return `${scenes} scene${scenes === 1 ? "" : "s"}`;
  return `${items} item${items === 1 ? "" : "s"}`;
}

// ------------------------------------------------------------------ drops

export interface DropHint {
  target: StoryContainerRef;
  /** Insert before this sibling; null = at the end of the container. */
  before: Ref | null;
}

/** Can the dragged kinds be dropped into `target`? */
export function canDrop(kinds: ItemKind[], target: StoryContainerRef, moving: Set<string>): boolean {
  if (kinds.includes("sequence")) return target.parentType === "act" && kinds.every((k) => k === "sequence");
  if (target.parentType === "sequence" && moving.has(`sequence:${target.parentId}`)) return false;
  return true;
}

/** Drop next to an item (before/after it) in the item's container. */
export function dropBeside(board: StoryBoardState, over: Ref, placement: "before" | "after", moving: Set<string>): DropHint | null {
  const loc = locate(board, over);
  if (!loc) return null;
  const sibs = loc.siblings.filter((s) => !moving.has(refKey(toRef(s))));
  const idx = sibs.findIndex((s) => s.kind === over.kind && s.id === over.id);
  if (idx < 0) {
    // Hovering one of the moving items: resolve to the next non-moving sibling.
    const after = loc.siblings.slice(loc.index + 1).find((s) => !moving.has(refKey(toRef(s))));
    return { target: loc.container, before: after ? toRef(after) : null };
  }
  if (placement === "before") return { target: loc.container, before: over };
  const next = sibs[idx + 1];
  return { target: loc.container, before: next ? toRef(next) : null };
}

/** Drop into a container: at the start (e.g. over a sequence header) or the end. */
export function dropInto(board: StoryBoardState, c: StoryContainerRef, atStart: boolean, moving: Set<string>): DropHint {
  if (!atStart) return { target: c, before: null };
  const first = childrenOf(board, c).find((s) => !moving.has(refKey(toRef(s))));
  return { target: c, before: first ? toRef(first) : null };
}

/** True when dropping would leave a single item exactly where it is. */
export function isNoop(board: StoryBoardState, refs: Ref[], hint: DropHint): boolean {
  if (refs.length !== 1) return false;
  const loc = locate(board, refs[0]);
  if (!loc || !sameContainer(loc.container, hint.target)) return false;
  const next = loc.siblings[loc.index + 1];
  if (!hint.before) return !next;
  return !!next && next.kind === hint.before.kind && next.id === hint.before.id;
}

/** Keyboard reorder among siblings (Alt+↑ / Alt+↓). */
export function stepHint(board: StoryBoardState, ref: Ref, dir: -1 | 1): DropHint | null {
  const loc = locate(board, ref);
  if (!loc) return null;
  if (dir === -1) {
    if (loc.index === 0) return null;
    return { target: loc.container, before: toRef(loc.siblings[loc.index - 1]) };
  }
  if (loc.index >= loc.siblings.length - 1) return null;
  const afterNext = loc.siblings[loc.index + 2];
  return { target: loc.container, before: afterNext ? toRef(afterNext) : null };
}

/** Sort selected refs into board order (acts → sequences → parking → unassigned). */
export function boardOrder(board: StoryBoardState, refs: Ref[]): Ref[] {
  const order: string[] = [];
  const walk = (items: StoryItem[]) =>
    items.forEach((i) => {
      order.push(refKey(toRef(i)));
      if (i.kind === "sequence") walk(i.items);
    });
  board.acts.forEach((a) => walk(a.items));
  walk(board.parking);
  walk(board.unassigned);
  const rank = new Map(order.map((k, i) => [k, i]));
  return [...refs].sort((a, b) => (rank.get(refKey(a)) ?? 1e9) - (rank.get(refKey(b)) ?? 1e9));
}

// ----------------------------------------------------------- navigation

export interface FlatEntry {
  ref: Ref;
  actId: string | null;
}

/** Visible items in reading order, for arrow-key focus movement. */
export function flattenVisible(board: StoryBoardState, collapsed: Set<string>, visible: (i: StoryItem) => boolean): FlatEntry[] {
  const out: FlatEntry[] = [];
  const walk = (items: StoryItem[], actId: string | null) =>
    items.forEach((i) => {
      if (!visible(i)) return;
      out.push({ ref: toRef(i), actId });
      if (i.kind === "sequence" && !collapsed.has(i.id)) walk(i.items, actId);
    });
  board.acts.forEach((a) => !collapsed.has(a.id) && walk(a.items, a.id));
  walk(board.parking, "parking");
  walk(board.unassigned, "unassigned");
  return out;
}

// --------------------------------------------------------------- filtering

export function matchesFilter(i: StoryItem, q: string): boolean {
  const needle = q.trim().toLowerCase();
  if (!needle) return true;
  const has = (s: string | null | undefined) => (s ?? "").toLowerCase().includes(needle);
  switch (i.kind) {
    case "card":
      return has(i.shortDescription) || has(i.sceneHeading) || has(i.notes);
    case "beat":
      return has(i.text) || has(i.note);
    case "sequence":
      return has(i.title) || i.items.some((c) => matchesFilter(c, q));
  }
}

// ------------------------------------------------------------- build rules

/** Mirrors Rust `build::heading_is_valid`: a standard location prefix plus a place. */
export function headingIsValid(h: string | null | undefined): boolean {
  const u = (h ?? "").trim().toUpperCase();
  const prefixes = ["INT./EXT.", "EXT./INT.", "INT/EXT", "EXT/INT", "I/E", "INT.", "EXT.", "EST.", "INT ", "EXT ", "EST "];
  return prefixes.some((p) => u.startsWith(p) && /[\p{L}\p{N}]/u.test(u.slice(p.length).replace(/^[.\s]+/, "")));
}

/** Destination options for the "Move…" dialog. */
export function moveDestinations(board: StoryBoardState, kinds: ItemKind[]): { container: StoryContainerRef; label: string; depth: number }[] {
  const out: { container: StoryContainerRef; label: string; depth: number }[] = [];
  const onlySequences = kinds.length > 0 && kinds.every((k) => k === "sequence");
  for (const a of board.acts) {
    out.push({ container: ACT(a.id), label: a.title, depth: 0 });
    if (!onlySequences) {
      a.items.forEach((i) => i.kind === "sequence" && out.push({ container: SEQ(i.id), label: i.title, depth: 1 }));
    }
  }
  if (!kinds.includes("sequence")) out.push({ container: PARKING, label: "Parking Lot", depth: 0 });
  return out;
}
