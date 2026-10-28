// In-script find (FSD §15.8, UX §3.13; scope per the Screenplay and Production Depth
// Specification §18: this scene or the whole script). Same semantics as Rust's
// `screenplay.replace_all`: literal text, optional case sensitivity, optional
// whole-word boundaries where letters, digits and "_" are word characters.

import type { Node as PMNode } from "prosemirror-model";
import { blockText } from "./model";

export interface FindOptions {
  query: string;
  matchCase: boolean;
  wholeWord: boolean;
  /** "This scene": only the scene with this id. Absent or null: the whole script. */
  sceneId?: string | null;
}

const WORD = /[\p{L}\p{N}_]/u;

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/** [start, end) offsets (UTF-16) of matches in `text`, non-overlapping, left to right. */
export function findInText(text: string, o: FindOptions): [number, number][] {
  if (!o.query) return [];
  const re = new RegExp(escapeRegExp(o.query), o.matchCase ? "gu" : "giu");
  const out: [number, number][] = [];
  let m: RegExpExecArray | null;
  while ((m = re.exec(text)) !== null) {
    const start = m.index;
    const end = start + m[0].length;
    if (m[0].length === 0) {
      re.lastIndex++;
      continue;
    }
    if (o.wholeWord) {
      const before = start > 0 ? Array.from(text.slice(0, start)).pop() : undefined;
      const after = end < text.length ? Array.from(text.slice(end))[0] : undefined;
      if ((before && WORD.test(before)) || (after && WORD.test(after))) continue;
    }
    out.push([start, end]);
  }
  return out;
}

/** Matches in the editor document as [from, to) positions, limited to one scene when the
 * scope is "This scene", and skipping notes while they are hidden. */
export function computeMatches(doc: PMNode, opts: FindOptions | null, showNotes: boolean): { from: number; to: number }[] {
  const out: { from: number; to: number }[] = [];
  if (!opts || !opts.query) return out;
  let scene: string | null = null;
  doc.forEach((node, offset) => {
    if (node.type.name === "scene_heading") scene = node.attrs.id as string;
    if (opts.sceneId && scene !== opts.sceneId) return;
    if (!showNotes && node.type.name === "note") return;
    for (const [s, e] of findInText(blockText(node), opts)) out.push({ from: offset + 1 + s, to: offset + 1 + e });
  });
  return out;
}

/** How far to scroll a box so a line at `rect` is comfortably visible: 0 when it already is. The line is centred
 * when it is outside the middle of the box. */
export function revealDelta(rect: { top: number; bottom: number }, box: { top: number; bottom: number }, margin = 56): number {
  const m = Math.min(margin, (box.bottom - box.top) / 4);
  if (rect.top >= box.top + m && rect.bottom <= box.bottom - m) return 0;
  return (rect.top + rect.bottom) / 2 - (box.top + box.bottom) / 2;
}

export interface RevealableView {
  dom: HTMLElement;
  coordsAtPos(pos: number): { top: number; bottom: number };
}

/** Scroll the page's own scroll area (`.spx-scroll`) so the match at `pos` is on screen. ProseMirror's built-in
 * scrollIntoView follows the browser selection, which sits in the Find box while that box has focus, so it
 * scrolled the wrong area and left the match off screen. Returns false when the editor has no scroll area. */
export function scrollMatchIntoView(view: RevealableView, pos: number): boolean {
  const scroller = view.dom.closest(".spx-scroll") as HTMLElement | null;
  if (!scroller) return false;
  const delta = revealDelta(view.coordsAtPos(pos), scroller.getBoundingClientRect());
  if (delta !== 0) scroller.scrollTop += delta;
  return true;
}

/** Index of the first match that starts at or after `pos`; the first match when none does (wraps round). */
export function matchIndexFrom(matches: { from: number }[], pos: number): number {
  const i = matches.findIndex((m) => m.from >= pos);
  return i < 0 ? 0 : i;
}
