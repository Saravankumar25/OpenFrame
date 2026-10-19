// Spelling underlines in the screenplay editor (SCRIPT-020).
//
// Rust checks the text against the operating system's dictionaries (`writing.check_spelling`),
// skipping personal dictionary words and any writing system without an installed dictionary.
// This plugin decides what to send, draws the underlines and keeps them in place while the
// writer types. Character cues are names and are never checked; a word that is a character
// name used in the script is never underlined either. Checking never changes the text.

import { Plugin, PluginKey, type EditorState, type Transaction } from "prosemirror-state";
import { Decoration, DecorationSet, type EditorView } from "prosemirror-view";
import type { Node as PMNode } from "prosemirror-model";
import type { SpellingItem } from "../../../ipc/generated/SpellingItem";
import type { WritingSpellingCheck } from "../../../ipc/generated/WritingSpellingCheck";
import { blockText } from "./model";
import { scriptValues } from "./suggest";

export const MAX_BATCH_ITEMS = 400;
export const MAX_BATCH_BYTES = 400 * 1024;
const DEBOUNCE_MS = 600;

export interface SpellResult {
  id: string;
  text: string;
  misses: { start: number; end: number }[];
}

export interface SpellOptions {
  /** Ask Rust to check these blocks. */
  check: (items: SpellingItem[]) => Promise<WritingSpellingCheck>;
  /** False while spell checking is off or no dictionary is available. */
  active: () => boolean;
  /** Extra names never to underline (project characters and locations). */
  knownNames: () => string[];
  onError?: (e: unknown) => void;
}

interface SpellState {
  deco: DecorationSet;
  /** Text each block had when it was last checked. */
  checked: Map<string, string>;
  ignored: Set<string>;
  /** Bumped by a reset so the checker starts again. */
  epoch: number;
}

type SpellMeta = { results: SpellResult[] } | { reset: true } | { ignore: string };

export const spellKey = new PluginKey<SpellState>("sp-spell");

/** Element types whose text is checked (character cues are names). */
export function checksType(type: string): boolean {
  return type !== "character";
}

/** Split blocks into calls that stay inside the Rust limits. */
export function batches(items: SpellingItem[], maxItems = MAX_BATCH_ITEMS, maxBytes = MAX_BATCH_BYTES): SpellingItem[][] {
  const out: SpellingItem[][] = [];
  let cur: SpellingItem[] = [];
  let bytes = 0;
  const enc = new TextEncoder();
  for (const it of items) {
    const size = enc.encode(it.text).length;
    if (cur.length && (cur.length >= maxItems || bytes + size > maxBytes)) {
      out.push(cur);
      cur = [];
      bytes = 0;
    }
    cur.push(it);
    bytes += size;
  }
  if (cur.length) out.push(cur);
  return out;
}

/** Drop misses that are known names (case-insensitive) or ignored for this session. */
export function filterMisses(text: string, misses: SpellResult["misses"], known: Set<string>, ignored: Set<string>): SpellResult["misses"] {
  return misses.filter((m) => {
    const w = text.slice(m.start, m.end).toLowerCase();
    return w.length > 0 && !known.has(w) && !ignored.has(w);
  });
}

function blockPositions(doc: PMNode): Map<string, { pos: number; node: PMNode }> {
  const out = new Map<string, { pos: number; node: PMNode }>();
  doc.forEach((node, pos) => {
    const id = node.attrs.id as string | null;
    if (id) out.set(id, { pos, node });
  });
  return out;
}

function knownWords(doc: PMNode, extra: string[]): Set<string> {
  const s = new Set<string>();
  const add = (name: string) => name.toLowerCase().split(/\s+/).forEach((w) => w && s.add(w));
  scriptValues(doc).characters.forEach(add);
  extra.forEach(add);
  return s;
}

/** Positions inside blocks touched by a transaction (whole blocks, so a changed word loses its
 * stale underline until it is checked again). */
function touchedBlocks(tr: Transaction): [number, number][] {
  const ranges: [number, number][] = [];
  tr.mapping.maps.forEach((map, i) => {
    map.forEach((_os, _oe, ns, ne) => {
      // Map the range to the final document.
      const after = tr.mapping.slice(i + 1);
      ranges.push([after.map(ns, -1), after.map(ne, 1)]);
    });
  });
  const doc = tr.doc;
  return ranges.map(([a, b]) => {
    const $a = doc.resolve(Math.max(0, Math.min(a, doc.content.size)));
    const $b = doc.resolve(Math.max(0, Math.min(b, doc.content.size)));
    const start = $a.depth >= 1 ? $a.before(1) : a;
    const end = $b.depth >= 1 ? $b.after(1) : b;
    return [start, end];
  });
}

export function createSpellPlugin(opts: SpellOptions): Plugin<SpellState> {
  // Words ignored in this session. Kept with the plugin, not the state, so they stay ignored when
  // the editor rebuilds its state (undo, redo or a change from another window).
  const ignoredWords = new Set<string>();
  return new Plugin<SpellState>({
    key: spellKey,
    state: {
      init: () => ({ deco: DecorationSet.empty, checked: new Map(), ignored: ignoredWords, epoch: 0 }),
      apply: (tr, prev, _old, state) => {
        const meta = tr.getMeta(spellKey) as SpellMeta | undefined;
        let deco = prev.deco;
        let checked = prev.checked;
        let ignored = prev.ignored;
        if (tr.docChanged) {
          deco = deco.map(tr.mapping, tr.doc);
          const touched = touchedBlocks(tr);
          if (touched.length) {
            const stale: Decoration[] = [];
            for (const [a, b] of touched) stale.push(...deco.find(a, b));
            if (stale.length) deco = deco.remove(stale);
            // A touched block is checked again even if its text ends up as it was (type a
            // letter and delete it, or undo): its underlines were just removed. The record is
            // copied only when it holds a touched block, so typing on in the same block does not
            // copy a record of thousands of blocks on every keystroke.
            const ids: string[] = [];
            for (const [a, b] of touched) {
              tr.doc.nodesBetween(a, Math.min(b, tr.doc.content.size), (node) => {
                const id = node.attrs.id as string | null;
                if (id && checked.has(id)) ids.push(id);
                return false;
              });
            }
            if (ids.length) {
              checked = new Map(checked);
              for (const id of ids) checked.delete(id);
            }
          }
        }
        if (meta && "reset" in meta) {
          return { deco: DecorationSet.empty, checked: new Map(), ignored, epoch: prev.epoch + 1 };
        }
        if (meta && "ignore" in meta) {
          ignoredWords.add(meta.ignore.toLowerCase());
          ignored = ignoredWords;
          const doc = state.doc;
          const drop = deco.find().filter((d) => doc.textBetween(d.from, d.to).toLowerCase() === meta.ignore.toLowerCase());
          return { deco: deco.remove(drop), checked, ignored, epoch: prev.epoch };
        }
        if (meta && "results" in meta) {
          checked = new Map(checked);
          const pos = blockPositions(state.doc);
          const known = knownWords(state.doc, opts.knownNames());
          for (const r of meta.results) {
            const b = pos.get(r.id);
            // The block changed again while it was being checked: check it next round.
            if (!b || blockText(b.node) !== r.text) continue;
            checked.set(r.id, r.text);
            const start = b.pos + 1;
            deco = deco.remove(deco.find(b.pos, b.pos + b.node.nodeSize));
            const add = filterMisses(r.text, r.misses, known, ignored).map((m) =>
              Decoration.inline(start + m.start, start + m.end, { class: "sp-miss", "data-spell": "1" }),
            );
            if (add.length) deco = deco.add(state.doc, add);
          }
        }
        return { deco, checked, ignored, epoch: prev.epoch };
      },
    },
    props: {
      decorations: (state) => spellKey.getState(state)?.deco,
    },
    view: (view) => {
      let timer: ReturnType<typeof setTimeout> | null = null;
      let running = false;
      let again = false;
      let destroyed = false;
      const run = async () => {
        if (running) {
          again = true;
          return;
        }
        if (!opts.active()) return;
        running = true;
        try {
          const st = spellKey.getState(view.state);
          const items: SpellingItem[] = [];
          view.state.doc.forEach((node) => {
            const id = node.attrs.id as string | null;
            if (!id || !checksType(node.type.name)) return;
            const text = blockText(node);
            if (!text.trim() || st?.checked.get(id) === text) return;
            items.push({ key: id, text });
          });
          for (const batch of batches(items)) {
            const r = await opts.check(batch);
            if (destroyed || !r.enabled) break;
            const texts = new Map(batch.map((b) => [b.key, b.text]));
            const results: SpellResult[] = r.results.map((x) => ({ id: x.key, text: texts.get(x.key) ?? "", misses: x.misses }));
            view.dispatch(view.state.tr.setMeta(spellKey, { results } satisfies SpellMeta).setMeta("remote", true).setMeta("addToHistory", false));
          }
        } catch (e) {
          opts.onError?.(e);
        } finally {
          running = false;
          if (again && !destroyed) {
            again = false;
            schedule();
          }
        }
      };
      const schedule = () => {
        if (timer) clearTimeout(timer);
        timer = setTimeout(() => void run(), DEBOUNCE_MS);
      };
      schedule();
      return {
        update: (v, prev) => {
          // Identity, not a deep compare: a deep compare walks every block on each keystroke.
          if (v.state.doc !== prev.doc) schedule();
          if (spellKey.getState(v.state)?.epoch !== spellKey.getState(prev)?.epoch) schedule();
        },
        destroy: () => {
          destroyed = true;
          if (timer) clearTimeout(timer);
        },
      };
    },
  });
}

/** Clear all underlines and check everything again (settings or dictionary changed). */
export function resetSpelling(view: EditorView): void {
  view.dispatch(view.state.tr.setMeta(spellKey, { reset: true } satisfies SpellMeta).setMeta("remote", true));
}

/** Stop underlining a word for the rest of this session. */
export function ignoreWord(view: EditorView, word: string): void {
  view.dispatch(view.state.tr.setMeta(spellKey, { ignore: word } satisfies SpellMeta).setMeta("remote", true));
}

/** The underlined word at `pos`, if any. */
export function missAt(state: EditorState, pos: number): { from: number; to: number; word: string } | null {
  const d = spellKey.getState(state)?.deco.find(pos, pos)[0];
  if (!d) return null;
  return { from: d.from, to: d.to, word: state.doc.textBetween(d.from, d.to) };
}

/** The next underlined word after `pos`, wrapping to the start. */
export function nextMiss(state: EditorState, pos: number): { from: number; to: number; word: string } | null {
  const all = spellKey.getState(state)?.deco.find() ?? [];
  if (!all.length) return null;
  const sorted = [...all].sort((a, b) => a.from - b.from);
  const d = sorted.find((x) => x.from > pos) ?? sorted[0];
  return { from: d.from, to: d.to, word: state.doc.textBetween(d.from, d.to) };
}
