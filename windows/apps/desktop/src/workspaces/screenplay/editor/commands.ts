// Editing commands of the screenplay editor: element type changes, custom
// elements, explicit page breaks and Enter, Tab and Shift+Enter (UX §4 "What
// Enter does", mock 077; Screenplay and Production Depth Specification §8, §10).
//
// Page breaks are atoms without text: typing commands never turn one into a
// text element, and Tab leaves it alone. Custom elements keep their definition
// (`customId`) when split, and Enter at their end creates their own next element.

import { NodeSelection, TextSelection, type Command, type EditorState, type Transaction } from "prosemirror-state";
import { Fragment, type Node as PMNode } from "prosemirror-model";
import type { ElementType } from "../../../ipc/generated/ElementType";
import type { ScreenplayCustomElementDto } from "../../../ipc/generated/ScreenplayCustomElementDto";
import type { EditorView } from "prosemirror-view";
import { matches } from "../../../app/shortcuts";
import { ELEMENT_META, isElementType, newId, schema } from "./schema";
import { nextOnEnter, onEnterEmpty, tabNext, tabPrev } from "./rules";

/** Custom element definitions of the open document (one editor at a time). */
let customDefs: ScreenplayCustomElementDto[] = [];

export function setCustomDefs(defs: ScreenplayCustomElementDto[]): void {
  customDefs = defs;
}

export function customDef(id: unknown): ScreenplayCustomElementDto | undefined {
  return customDefs.find((d) => d.id === id);
}

export function customDefByShortcut(shortcut: string): ScreenplayCustomElementDto | undefined {
  return customDefs.find((d) => d.shortcut === shortcut);
}

/** Element types a writer can type text into (not a page break, not a bare custom element). */
type TextElementType = Exclude<ElementType, "page_break" | "custom">;

function isTextType(t: ElementType): t is TextElementType {
  return t !== "page_break" && t !== "custom";
}

/** The block Enter creates after a custom element (its own next element). */
export function nextAfterCustom(node: PMNode): PMNode {
  const next = customDef(node.attrs.customId)?.nextElement ?? "action";
  if (next.startsWith("custom:")) {
    const id = next.slice("custom:".length);
    if (customDef(id)) return schema.nodes.custom.create({ id: newId(), customId: id });
    return schema.nodes.action.create({ id: newId() });
  }
  const t = isElementType(next) && isTextType(next) ? next : "action";
  return schema.nodes[t].create({ id: newId() });
}

export function topBlock(state: EditorState, pos: number): { node: PMNode; pos: number; index: number } {
  const $p = state.doc.resolve(pos);
  const index = Math.min($p.index(0), state.doc.childCount - 1);
  let start = 0;
  for (let i = 0; i < index; i++) start += state.doc.child(i).nodeSize;
  return { node: state.doc.child(index), pos: start, index };
}

function convertContent(node: PMNode, to: ElementType): Fragment {
  if (ELEMENT_META[to].multiline) return node.content;
  const nodes: PMNode[] = [];
  node.content.forEach((c) => nodes.push(c.type.name === "hard_break" ? schema.text(" ") : c));
  return Fragment.from(nodes);
}

/** A cursor near `from` after a change (inside a text block, never inside a page break). */
function selectionAfter(tr: Transaction, from: number): TextSelection {
  const pos = Math.max(1, Math.min(from, tr.doc.content.size - 1));
  const $pos = tr.doc.resolve(pos);
  if ($pos.parent.inlineContent) return TextSelection.create(tr.doc, pos);
  const near = TextSelection.findFrom($pos, 1, true) ?? TextSelection.findFrom($pos, -1, true);
  return (near as TextSelection | null) ?? TextSelection.create(tr.doc, 1);
}

/** Change the element type of the blocks in the selection. Changing between a
 * heading and a body element changes identity (it splits or merges scenes).
 * Page breaks in the selection stay page breaks; custom elements have their
 * own command ({@link setCustom}). */
export function setType(to: ElementType, onFail: (m: string) => void): Command {
  return (state, dispatch) => {
    if (!isTextType(to)) return false;
    const { from, to: end } = state.selection;
    const first = topBlock(state, from);
    // The selection end is exclusive: a selected page break ends where the next block starts.
    const last = topBlock(state, end > from ? end - 1 : end);
    let tr = state.tr;
    let changed = false;
    let onlyBreaks = true;
    for (let i = first.index; i <= last.index; i++) {
      let pos = 0;
      for (let k = 0; k < i; k++) pos += tr.doc.child(k).nodeSize;
      const node = tr.doc.child(i);
      if (node.type.name === "page_break") continue;
      onlyBreaks = false;
      if (node.type.name === to) continue;
      if (i === 0 && to !== "scene_heading") {
        onFail("The first scene needs a scene heading.");
        return false;
      }
      const crosses = (node.type.name === "scene_heading") !== (to === "scene_heading");
      const replacement = schema.nodes[to].create({ id: crosses ? newId() : node.attrs.id }, convertContent(node, to));
      try {
        tr = tr.replaceWith(pos, pos + node.nodeSize, replacement);
        changed = true;
      } catch {
        onFail("The first scene needs a scene heading.");
        return false;
      }
    }
    if (onlyBreaks) {
      onFail("A page break has no text to change.");
      return false;
    }
    if (!changed) return false;
    try {
      tr.doc.check();
    } catch {
      onFail("The first scene needs a scene heading.");
      return false;
    }
    if (dispatch) dispatch(tr.setSelection(selectionAfter(tr, state.selection.from)).scrollIntoView());
    return true;
  };
}

/** Turn the selected blocks into a custom element (never the first heading or a page break). */
export function setCustom(customId: string, onFail: (m: string) => void): Command {
  return (state, dispatch) => {
    if (!customDef(customId)) return false;
    const { from, to: end } = state.selection;
    const first = topBlock(state, from);
    // The selection end is exclusive: a selected page break ends where the next block starts.
    const last = topBlock(state, end > from ? end - 1 : end);
    let tr = state.tr;
    for (let i = first.index; i <= last.index; i++) {
      let pos = 0;
      for (let k = 0; k < i; k++) pos += tr.doc.child(k).nodeSize;
      const node = tr.doc.child(i);
      if (node.type.name === "custom" && node.attrs.customId === customId) continue;
      if (i === 0 || node.type.name === "page_break") {
        onFail(i === 0 ? "The first scene needs a scene heading." : "A page break has no text to change.");
        return false;
      }
      const crosses = node.type.name === "scene_heading";
      const replacement = schema.nodes.custom.create({ id: crosses ? newId() : node.attrs.id, customId }, convertContent(node, "custom"));
      tr = tr.replaceWith(pos, pos + node.nodeSize, replacement);
    }
    if (!tr.docChanged) return false;
    if (dispatch) dispatch(tr.setSelection(selectionAfter(tr, state.selection.from)).scrollIntoView());
    return true;
  };
}

/** Tab and Shift+Tab: the next element of the dialogue cycle. Page breaks and
 * custom elements are left as they are (the writer chose them on purpose);
 * the element menu and shortcuts still change them. */
export function tabCommand(
  step: 1 | -1 | ((t: ElementType) => ElementType | null),
  onFail: (m: string) => void,
): Command {
  const next = typeof step === "function" ? step : step === 1 ? tabNext : tabPrev;
  return (state, dispatch) => {
    const sel = state.selection;
    const block = topBlock(state, sel instanceof NodeSelection ? sel.from : sel.head);
    const to = next(block.node.type.name as ElementType);
    if (to) setType(to, onFail)(state, dispatch);
    return true; // Tab never moves focus out of the page.
  };
}

/** Run the first command whose shortcut (from Settings > Keyboard shortcuts, or its default)
 * this key press is. Returns whether a command handled it. */
export function runShortcut(
  e: KeyboardEvent,
  commands: [string, Command][],
  state: EditorState,
  dispatch: ((tr: Transaction) => void) | undefined,
  view?: EditorView,
): boolean {
  for (const [id, cmd] of commands) {
    if (matches(e, id)) return cmd(state, dispatch, view);
  }
  return false;
}

/** Insert page break (default Ctrl+Enter, configurable as `screenplay.page_break`): an explicit
 * page break after the current block, then a new block of the same kind. */
export const insertPageBreak: Command = (state, dispatch) => {
  const block = topBlock(state, state.selection.from);
  const after = block.pos + block.node.nodeSize;
  if (!dispatch) return true;
  const follow =
    block.node.type.name === "scene_heading" || block.node.type.name === "page_break"
      ? schema.nodes.action.create({ id: newId() })
      : block.node.type.create({ ...block.node.attrs, id: newId() });
  const tr = state.tr.insert(after, [schema.nodes.page_break.create({ id: newId() }), follow]);
  const target = after + tr.doc.child(block.index + 1).nodeSize + 1;
  dispatch(tr.setSelection(TextSelection.create(tr.doc, Math.min(target, tr.doc.content.size - 1))).scrollIntoView());
  return true;
};

export const enterAtCursor: Command = (state, dispatch) => {
  const { $from, empty } = state.selection;
  if (!empty) return false;
  const node = $from.parent;
  const type = node.type.name as ElementType;
  if (!isElementType(type) || $from.depth !== 1) return false;
  const blockPos = $from.before(1);
  if (node.content.size === 0) {
    const to = onEnterEmpty(type);
    if (!to) return true;
    if (dispatch) {
      const crosses = (type === "scene_heading") !== (to === "scene_heading");
      const tr = state.tr.setNodeMarkup(blockPos, schema.nodes[to], { id: crosses ? newId() : node.attrs.id });
      dispatch(tr.scrollIntoView());
    }
    return true;
  }
  if (!dispatch) return true;
  const atEnd = $from.parentOffset === node.content.size;
  const atStart = $from.parentOffset === 0;
  if (atEnd) {
    const after = blockPos + node.nodeSize;
    // The element that follows is already the empty one Enter would create (a new screenplay's
    // first action, or a line the writer left blank): go into it instead of stacking another
    // empty element in front of it.
    const following = state.doc.childAfter(after).node;
    if (type !== "custom" && following && following.content.size === 0 && following.type.name === nextOnEnter(type)) {
      dispatch(state.tr.setSelection(TextSelection.create(state.doc, after + 1)).scrollIntoView());
      return true;
    }
    const next = type === "custom" ? nextAfterCustom(node) : schema.nodes[nextOnEnter(type)].create({ id: newId() });
    const tr = state.tr.insert(after, next);
    dispatch(tr.setSelection(TextSelection.create(tr.doc, after + 1)).scrollIntoView());
  } else if (atStart) {
    // Keep the element (and its identity) with its text; add a blank one above.
    const tr = state.tr.insert(blockPos, node.type.create({ ...node.attrs, id: newId() }));
    dispatch(tr.setSelection(TextSelection.create(tr.doc, blockPos + 3)).scrollIntoView());
  } else {
    const afterType = type === "scene_heading" ? schema.nodes.action : node.type;
    const attrs = type === "custom" ? { id: newId(), customId: node.attrs.customId } : { id: newId() };
    const tr = state.tr.split($from.pos, 1, [{ type: afterType, attrs }]);
    dispatch(tr.scrollIntoView());
  }
  return true;
};

/** Enter: replaces a selection first (like any writing app), then applies the
 * element rules. Enter on a selected page break adds an action after it. */
export const enterCommand: Command = (state, dispatch) => {
  if (state.selection.empty) return enterAtCursor(state, dispatch);
  const sel = state.selection;
  if (sel instanceof NodeSelection && sel.node.type.name === "page_break") {
    if (dispatch) {
      const tr = state.tr.insert(sel.to, schema.nodes.action.create({ id: newId() }));
      dispatch(tr.setSelection(TextSelection.create(tr.doc, sel.to + 1)).scrollIntoView());
    }
    return true;
  }
  if (!dispatch) return true;
  const tr = state.tr.deleteSelection();
  const after = state.apply(tr);
  let follow: Transaction | null = null;
  enterAtCursor(after, (t) => {
    follow = t;
  });
  const extra = follow as Transaction | null;
  if (extra) {
    extra.steps.forEach((step) => tr.step(step));
    tr.setSelection(TextSelection.create(tr.doc, extra.selection.from));
  }
  dispatch(tr.scrollIntoView());
  return true;
};

export const lineBreakCommand: Command = (state, dispatch) => {
  const { $from } = state.selection;
  if (!ELEMENT_META[$from.parent.type.name as ElementType]?.multiline) return enterCommand(state, dispatch);
  if (dispatch) dispatch(state.tr.replaceSelectionWith(schema.nodes.hard_break.create()).scrollIntoView());
  return true;
};

/** The scene heading at or before `pos` (walks back from the cursor's block, not the whole script). */
export function sceneAt(doc: PMNode, pos: number): string | null {
  for (let i = Math.min(doc.resolve(pos).index(0), doc.childCount - 1); i >= 0; i--) {
    const n = doc.child(i);
    if (n.type.name === "scene_heading") return n.attrs.id as string;
  }
  return null;
}

export function headingPositions(doc: PMNode): number[] {
  const out: number[] = [];
  doc.forEach((n, off) => {
    if (n.type.name === "scene_heading") out.push(off);
  });
  return out;
}
