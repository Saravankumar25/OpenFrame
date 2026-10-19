// The suggestion list as a ProseMirror plugin (SCRIPT-018). The plugin tracks what the writer is
// typing, owns the keyboard while the list is open (arrows move, Enter accepts, Esc dismisses)
// and reports the list to the editor component, which draws it. Nothing is inserted unless the
// writer accepts an item. Enter on a character name, a known one or "Create New", fills or adds the
// name and moves on to Dialogue in the same key press, like Final Draft (owner decisions
// 2026-10-08). Tab never accepts an item; it keeps its element meaning.

import { Plugin, PluginKey, type EditorState } from "prosemirror-state";
import type { EditorView } from "prosemirror-view";
import type { ElementType } from "../../../ipc/generated/ElementType";
import type { WritingSuggestionSources } from "../../../ipc/generated/WritingSuggestionSources";
import { enterCommand } from "./commands";
import { blockText } from "./model";
import { contextAt, scriptValuesWithout, suggestions, type ScriptValues, type SuggestContext, type SuggestItem } from "./suggest";

export interface ActiveSuggest extends SuggestContext {
  blockPos: number;
  blockId: string | null;
}

export interface SuggestState {
  ctx: ActiveSuggest | null;
  items: SuggestItem[];
  /** Highlighted item; -1 = none (Enter then keeps its normal meaning). */
  index: number;
  dismissed: boolean;
}

type SuggestMeta = { dismiss: true } | { index: number };

export interface SuggestOptions {
  sources: () => WritingSuggestionSources | null;
  canEdit: () => boolean;
  /** "Create New" was accepted for a character or location name. `movesOn` is true when the same
   * Enter also moves the caret on to Dialogue (a new character name): the editor then keeps the
   * typed name, the new character and the Dialogue step apart in the undo history. It is called
   * before the Dialogue line is added. */
  onCreate: (kind: "character" | "location", name: string, movesOn: boolean) => void;
  /** Enter filled a known character name and is about to move on to Dialogue: the editor saves the
   * filled name first, so Undo takes back the Dialogue step before the name. */
  beforeMoveOn?: () => void;
  /** The list changed (null = hidden). `top` is below the caret line, `above` is its top. */
  onChange: (s: SuggestView | null) => void;
}

export interface SuggestView {
  items: SuggestItem[];
  index: number;
  left: number;
  top: number;
  above: number;
}

export const suggestKey = new PluginKey<SuggestState>("sp-suggest");
export const SUGGEST_LIST_ID = "sp-suggest-list";

const CLOSED: SuggestState = { ctx: null, items: [], index: -1, dismissed: false };

/** Script values without the element being typed (a half-typed name is never offered back as if
 * it already existed). Reading every element on each keystroke would slow typing in a long
 * script, so the values are kept while the writer stays in the same element and the number of
 * elements does not change; leaving the element, Enter, or a refresh reads them again. */
interface ValuesCache {
  blockId: string | null;
  childCount: number;
  values: ScriptValues;
}

function compute(state: EditorState, opts: SuggestOptions, cache: { current: ValuesCache | null }): Omit<SuggestState, "index" | "dismissed"> {
  const { $head, empty } = state.selection;
  if (!empty || $head.depth !== 1 || !opts.canEdit()) return { ctx: null, items: [] };
  const node = $head.parent;
  const text = blockText(node);
  const c = contextAt(node.type.name as ElementType, text, $head.parentOffset);
  if (!c) return { ctx: null, items: [] };
  const blockId = (node.attrs.id as string | null) ?? null;
  const hit = cache.current;
  let values: ScriptValues;
  if (hit && hit.blockId === blockId && hit.childCount === state.doc.childCount) {
    values = hit.values;
  } else {
    values = scriptValuesWithout(state.doc, $head.index(0));
    cache.current = { blockId, childCount: state.doc.childCount, values };
  }
  const items = suggestions(c, values, opts.sources(), true);
  return { ctx: { ...c, blockPos: $head.before(1), blockId }, items };
}

/** Key presses that belong to an input method composing text (Telugu, Hindi, ...). */
function composing(e: KeyboardEvent): boolean {
  return e.isComposing || e.keyCode === 229;
}

function sameContext(a: ActiveSuggest | null, b: ActiveSuggest | null): boolean {
  return !!a && !!b && a.blockId === b.blockId && a.kind === b.kind && a.query === b.query;
}

export function createSuggestPlugin(opts: SuggestOptions): Plugin<SuggestState> {
  const cache: { current: ValuesCache | null } = { current: null };
  return new Plugin<SuggestState>({
    key: suggestKey,
    state: {
      init: (_c, state) => {
        const r = compute(state, opts, cache);
        return { ...r, index: r.ctx?.query.trim() ? 0 : -1, dismissed: false };
      },
      apply: (tr, prev, _old, state) => {
        const meta = tr.getMeta(suggestKey) as SuggestMeta | undefined;
        if (meta && "dismiss" in meta) return { ...prev, dismissed: true };
        if (meta && "index" in meta) return { ...prev, index: meta.index };
        const refresh = !!tr.getMeta("suggestRefresh");
        if (!tr.docChanged && !tr.selectionSet && !refresh) return prev;
        if (refresh) cache.current = null;
        const r = compute(state, opts, cache);
        if (!r.ctx || r.items.length === 0) return { ...CLOSED, dismissed: tr.docChanged ? false : prev.dismissed && !!r.ctx };
        const same = sameContext(prev.ctx, r.ctx);
        // Typing brings a dismissed list back; moving the caret around the same word does not.
        const dismissed = tr.docChanged ? false : same ? prev.dismissed : false;
        let index = r.ctx.query.trim() ? 0 : -1;
        if (same && prev.index < r.items.length) index = prev.index;
        return { ...r, index, dismissed };
      },
    },
    props: {
      handleKeyDown(view, e) {
        const st = suggestKey.getState(view.state);
        if (!st || !st.ctx || st.dismissed || st.items.length === 0) return false;
        if (e.ctrlKey || e.metaKey || e.altKey || composing(e)) return false;
        const n = st.items.length;
        // Nothing highlighted yet (the writer has not typed): arrows keep moving the caret.
        if ((e.key === "ArrowDown" || e.key === "ArrowUp") && st.index < 0) return false;
        if (e.key === "ArrowDown") {
          view.dispatch(view.state.tr.setMeta(suggestKey, { index: (st.index + 1) % n }));
          return true;
        }
        if (e.key === "ArrowUp") {
          view.dispatch(view.state.tr.setMeta(suggestKey, { index: st.index <= 0 ? n - 1 : st.index - 1 }));
          return true;
        }
        if (e.key === "Escape") {
          view.dispatch(view.state.tr.setMeta(suggestKey, { dismiss: true }));
          return true;
        }
        if (e.key === "Enter" && !e.shiftKey && st.index >= 0) {
          acceptSuggestion(view, st.index, opts, { moveOn: true });
          return true;
        }
        return false;
      },
    },
    view: (view) => {
      const report = (v: EditorView) => {
        const st = suggestKey.getState(v.state);
        const open = !!st && !!st.ctx && !st.dismissed && st.items.length > 0 && v.hasFocus();
        if (open) v.dom.setAttribute("aria-controls", SUGGEST_LIST_ID);
        else v.dom.removeAttribute("aria-controls");
        if (open && st.index >= 0) v.dom.setAttribute("aria-activedescendant", `${SUGGEST_LIST_ID}-${st.index}`);
        else v.dom.removeAttribute("aria-activedescendant");
        if (!open || !st?.ctx) {
          opts.onChange(null);
          return;
        }
        let left = 0;
        let top = 0;
        let above = 0;
        try {
          const c = v.coordsAtPos(st.ctx.blockPos + 1 + st.ctx.from);
          left = c.left;
          top = c.bottom + 4;
          above = c.top - 4;
        } catch {
          // No layout (tests, hidden window): the list is still reported.
        }
        opts.onChange({ items: st.items, index: st.index, left, top, above });
      };
      view.dom.setAttribute("aria-autocomplete", "list");
      report(view);
      return {
        update: (v) => report(v),
        destroy: () => opts.onChange(null),
      };
    },
  });
}

/** Accept item `index` of the open list: replace only the typed part, or for "Create New" keep
 * the text as typed and add the name to the project. With `moveOn` (the Enter key), a character
 * name, known or new, also moves on to Dialogue, as Enter does after any character name. */
export function acceptSuggestion(
  view: EditorView,
  index: number,
  opts: Pick<SuggestOptions, "onCreate" | "beforeMoveOn">,
  how: { moveOn?: boolean } = {},
): boolean {
  const st = suggestKey.getState(view.state);
  const item = st?.items[index];
  if (!st?.ctx || !item) return false;
  const from = st.ctx.blockPos + 1 + st.ctx.from;
  const to = st.ctx.blockPos + 1 + st.ctx.to;
  if (item.create) {
    const movesOn = !!how.moveOn && item.create === "character";
    opts.onCreate(item.create, st.ctx.query.trim().toUpperCase(), movesOn);
    if (movesOn) enterCommand(view.state, view.dispatch);
    else view.dispatch(view.state.tr.setMeta(suggestKey, { dismiss: true }));
    return true;
  }
  if (view.state.doc.textBetween(from, to) !== item.insert) {
    view.dispatch(view.state.tr.insertText(item.insert, from, to).scrollIntoView());
  }
  if (how.moveOn && st.ctx.kind === "character") {
    opts.beforeMoveOn?.();
    enterCommand(view.state, view.dispatch);
  }
  return true;
}

/** Re-read the list (project values arrived or changed). */
export function refreshSuggestions(view: EditorView): void {
  view.dispatch(view.state.tr.setMeta("suggestRefresh", true).setMeta("remote", true));
}
