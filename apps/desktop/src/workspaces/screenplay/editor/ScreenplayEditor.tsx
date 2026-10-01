// The structured screenplay editor (FSD §15–16, §92; UX §3.11, §4).
//
// ProseMirror handles cursor, selection, clipboard and typing like a normal
// writing app. Element structure, Enter/Tab progression and Ctrl+1…7 are
// screenplay rules layered on top. Undo/redo is Rust's (the editor flushes its
// pending edits first), so Ctrl+Z inside the page and the toolbar Undo are the
// same history.

import { forwardRef, useEffect, useImperativeHandle, useRef } from "react";
import { EditorState, Plugin, PluginKey, TextSelection, type Command, type Transaction } from "prosemirror-state";
import { EditorView, Decoration, DecorationSet } from "prosemirror-view";
import { keymap } from "prosemirror-keymap";
import { baseKeymap } from "prosemirror-commands";
import { Fragment, Slice, type Node as PMNode } from "prosemirror-model";
import type { ElementType } from "../../../ipc/generated/ElementType";
import type { ScreenplayDocument } from "../../../ipc/generated/ScreenplayDocument";
import type { ScreenplayEditResult } from "../../../ipc/generated/ScreenplayEditResult";
import type { CommentThread } from "../../../ipc/generated/CommentThread";
import { call, OpError } from "../../../ipc/client";
import { reportError } from "../../../ipc/query";
import { toast } from "../../../app/toast";
import { ELEMENT_META, SHORTCUT_ORDER, isElementType, newId, schema } from "./schema";
import { blockText, docFromSnapshot, mayAddBlocks, snapshotFromDoc, snapshotFromScenes } from "./model";
import { nextOnEnter, onEnterEmpty, tabNext, tabPrev } from "./rules";
import { findInText, type FindOptions } from "./find";
import { SyncController, type SyncStatus } from "./sync";

export interface OutlineScene {
  id: string;
  number: number;
  heading: string;
}

export interface CursorInfo {
  sceneId: string | null;
  elementId: string | null;
  elementType: ElementType;
}

export interface SelectionAnchor {
  /** Element id (body elements) — or the scene id when the selection is in a heading. */
  elementId: string;
  sceneId: string;
  isHeading: boolean;
  start: number;
  end: number;
  text: string;
}

export interface EditorHandle {
  flush(): Promise<void>;
  focus(): void;
  jumpToScene(sceneId: string): boolean;
  jumpToElement(elementId: string, start?: number, end?: number): boolean;
  setElementType(t: ElementType): void;
  setFind(o: FindOptions | null): { index: number; count: number };
  findStep(dir: 1 | -1): { index: number; count: number };
  replaceCurrent(replacement: string): { index: number; count: number };
  selectionAnchor(): SelectionAnchor | null;
  /** Plain text of one scene (for "Copy Scene"). */
  sceneText(sceneId: string): string;
}

interface Props {
  doc: ScreenplayDocument;
  canEdit: boolean;
  showNotes: boolean;
  comments: CommentThread[];
  onOutline: (scenes: OutlineScene[]) => void;
  onCursor: (c: CursorInfo) => void;
  onStatus: (s: SyncStatus) => void;
  /** The writer tried to change a locked draft (UX §3.17 "Create a revision?"). */
  onBlockedEdit: () => void;
  onFindShortcut: () => void;
  /** Non-empty text selection (for the nearby Comment affordance), or null. */
  onSelection: (s: { rect: DOMRect; anchor: SelectionAnchor } | null) => void;
  onFindResult?: (r: { index: number; count: number }) => void;
}

// ------------------------------------------------------------- plugins

interface FindState {
  opts: FindOptions | null;
  matches: { from: number; to: number }[];
  index: number;
  deco: DecorationSet;
}
const findKey = new PluginKey<FindState>("sp-find");
const commentKey = new PluginKey<DecorationSet>("sp-comments");

function computeMatches(doc: PMNode, opts: FindOptions | null, showNotes: boolean) {
  const out: { from: number; to: number }[] = [];
  if (!opts || !opts.query) return out;
  doc.forEach((node, offset) => {
    if (!showNotes && node.type.name === "note") return;
    for (const [s, e] of findInText(blockText(node), opts)) out.push({ from: offset + 1 + s, to: offset + 1 + e });
  });
  return out;
}

function findDeco(doc: PMNode, matches: FindState["matches"], index: number) {
  return DecorationSet.create(
    doc,
    matches.map((m, i) => Decoration.inline(m.from, m.to, { class: i === index ? "hlc" : "hl" })),
  );
}

function commentDeco(doc: PMNode, threads: CommentThread[]): DecorationSet {
  // No comments (the common case while writing): skip the walk over every block.
  if (threads.length === 0) return DecorationSet.empty;
  const pos = new Map<string, number>();
  doc.forEach((node, offset) => {
    if (node.attrs.id) pos.set(node.attrs.id as string, offset);
  });
  const decos: Decoration[] = [];
  for (const t of threads) {
    const a = t.comment.anchor;
    if (!a || t.comment.status === "Resolved" || t.comment.contextMoved) continue;
    const at = pos.get(a.elementId);
    if (at === undefined) continue;
    const node = doc.nodeAt(at);
    if (!node || a.end > node.content.size) continue;
    decos.push(Decoration.inline(at + 1 + a.start, at + 1 + a.end, { class: "sp-cmt", "data-comment": t.comment.id }));
  }
  return DecorationSet.create(doc, decos);
}

/** Give every block a unique id (split/paste create blocks without one). */
const idPlugin = new Plugin({
  appendTransaction(trs, _old, state) {
    // Walking all blocks of a 6,000-element script on every keystroke is wasted
    // work: only transactions that insert block nodes can add or duplicate ids.
    if (!trs.some((t) => t.docChanged && mayAddBlocks(t))) return null;
    const seen = new Set<string>();
    let tr: Transaction | null = null;
    state.doc.forEach((node, offset) => {
      const id = node.attrs.id as string | null;
      if (!id || seen.has(id)) {
        tr = (tr ?? state.tr).setNodeMarkup(offset, undefined, { ...node.attrs, id: newId() });
        tr.setMeta("ids", true);
      } else {
        seen.add(id);
      }
    });
    return tr;
  },
});

// ------------------------------------------------------------ commands

function topBlock(state: EditorState, pos: number): { node: PMNode; pos: number; index: number } {
  const $p = state.doc.resolve(pos);
  const index = $p.index(0);
  let start = 0;
  for (let i = 0; i < index; i++) start += state.doc.child(i).nodeSize;
  return { node: state.doc.child(Math.min(index, state.doc.childCount - 1)), pos: start, index };
}

function convertContent(node: PMNode, to: ElementType): Fragment {
  if (ELEMENT_META[to].multiline) return node.content;
  const nodes: PMNode[] = [];
  node.content.forEach((c) => nodes.push(c.type.name === "hard_break" ? schema.text(" ") : c));
  return Fragment.from(nodes);
}

/** Change the element type of the blocks in the selection. Changing between a
 * heading and a body element changes identity (it splits or merges scenes). */
function setType(to: ElementType, onFail: (m: string) => void): Command {
  return (state, dispatch) => {
    const { from, to: end } = state.selection;
    const first = topBlock(state, from);
    const last = topBlock(state, end);
    let tr = state.tr;
    let changed = false;
    for (let i = first.index; i <= last.index; i++) {
      let pos = 0;
      for (let k = 0; k < i; k++) pos += tr.doc.child(k).nodeSize;
      const node = tr.doc.child(i);
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
    if (!changed) return false;
    try {
      tr.doc.check();
    } catch {
      onFail("The first scene needs a scene heading.");
      return false;
    }
    if (dispatch) {
      const sel = TextSelection.create(tr.doc, Math.min(state.selection.from, tr.doc.content.size - 1));
      dispatch(tr.setSelection(sel).scrollIntoView());
    }
    return true;
  };
}

const enterAtCursor: Command = (state, dispatch) => {
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
    const nextType = nextOnEnter(type);
    const after = blockPos + node.nodeSize;
    const tr = state.tr.insert(after, schema.nodes[nextType].create({ id: newId() }));
    dispatch(tr.setSelection(TextSelection.create(tr.doc, after + 1)).scrollIntoView());
  } else if (atStart) {
    // Keep the element (and its identity) with its text; add a blank one above.
    const tr = state.tr.insert(blockPos, schema.nodes[type].create({ id: newId() }));
    dispatch(tr.setSelection(TextSelection.create(tr.doc, blockPos + 3)).scrollIntoView());
  } else {
    const afterType = type === "scene_heading" ? schema.nodes.action : node.type;
    const tr = state.tr.split($from.pos, 1, [{ type: afterType, attrs: { id: newId() } }]);
    dispatch(tr.scrollIntoView());
  }
  return true;
};

/** Enter: replaces a selection first (like any writing app), then applies the element rules. */
const enterCommand: Command = (state, dispatch) => {
  if (state.selection.empty) return enterAtCursor(state, dispatch);
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

const lineBreakCommand: Command = (state, dispatch) => {
  const { $from } = state.selection;
  if (!ELEMENT_META[$from.parent.type.name as ElementType]?.multiline) return enterCommand(state, dispatch);
  if (dispatch) dispatch(state.tr.replaceSelectionWith(schema.nodes.hard_break.create()).scrollIntoView());
  return true;
};

/** The scene heading at or before `pos` (walks back from the cursor's block, not the whole script). */
function sceneAt(doc: PMNode, pos: number): string | null {
  for (let i = Math.min(doc.resolve(pos).index(0), doc.childCount - 1); i >= 0; i--) {
    const n = doc.child(i);
    if (n.type.name === "scene_heading") return n.attrs.id as string;
  }
  return null;
}

function headingPositions(doc: PMNode): number[] {
  const out: number[] = [];
  doc.forEach((n, off) => {
    if (n.type.name === "scene_heading") out.push(off);
  });
  return out;
}

// ------------------------------------------------------------ component

export const ScreenplayEditor = forwardRef<EditorHandle, Props>(function ScreenplayEditor(props, ref) {
  const host = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const syncRef = useRef<SyncController | null>(null);
  const propsRef = useRef(props);
  propsRef.current = props;
  const findOpts = useRef<FindOptions | null>(null);
  const findIndex = useRef(0);
  const draftId = props.doc.draft.id;

  // Create the view once per draft (the parent keys this component by draft).
  useEffect(() => {
    const p = propsRef.current;
    const initial = snapshotFromScenes(p.doc.scenes);
    const outlineTimer = { t: 0 as unknown as ReturnType<typeof setTimeout> };

    const emitOutline = (doc: PMNode) => {
      clearTimeout(outlineTimer.t);
      outlineTimer.t = setTimeout(() => {
        const out: OutlineScene[] = [];
        doc.forEach((n) => {
          if (n.type.name === "scene_heading") out.push({ id: n.attrs.id as string, number: out.length + 1, heading: blockText(n) });
        });
        propsRef.current.onOutline(out);
      }, 120);
    };

    const emitCursor = (state: EditorState) => {
      const { $head } = state.selection;
      const node = $head.depth >= 1 ? $head.node(1) : state.doc.firstChild!;
      propsRef.current.onCursor({
        sceneId: sceneAt(state.doc, $head.pos),
        elementId: (node.attrs.id as string | null) ?? null,
        elementType: node.type.name as ElementType,
      });
    };

    const anchorOf = (state: EditorState): SelectionAnchor | null => {
      const { $from, $to, empty } = state.selection;
      if (empty || $from.depth < 1 || $to.depth < 1 || $from.before(1) !== $to.before(1)) return null;
      const node = $from.parent;
      const id = node.attrs.id as string | null;
      const sceneId = sceneAt(state.doc, $from.pos);
      if (!id || !sceneId) return null;
      const text = state.doc.textBetween($from.pos, $to.pos, undefined, "\n");
      if (!text.trim()) return null;
      return {
        elementId: id,
        sceneId,
        isHeading: node.type.name === "scene_heading",
        start: $from.parentOffset,
        end: $to.parentOffset,
        text,
      };
    };

    const undoRedo = (redo: boolean): Command => () => {
      void (async () => {
        try {
          await syncRef.current?.flush();
          const r = await call<{ label: string | null }>(redo ? "history.redo" : "history.undo", { store: "project" });
          if (r.label) toast.info(`${redo ? "Redid" : "Undid"}: ${r.label}`);
        } catch (e) {
          reportError(e);
        }
      })();
      return true;
    };

    const failMsg = (m: string) => toast.info(m);
    const typeKeys: Record<string, Command> = {};
    SHORTCUT_ORDER.forEach((t, i) => {
      typeKeys[`Mod-${i + 1}`] = setType(t, failMsg);
    });

    const moveScene = (dir: 1 | -1): Command => (state, dispatch) => {
      const heads = headingPositions(state.doc);
      const cur = state.selection.from;
      const target = dir === 1 ? heads.find((h) => h + 1 > cur) : [...heads].reverse().find((h) => h + 1 < cur);
      if (target === undefined) return true;
      if (dispatch) dispatch(state.tr.setSelection(TextSelection.create(state.doc, target + 1)).scrollIntoView());
      return true;
    };

    const guard = new Plugin({
      filterTransaction(tr) {
        if (!tr.docChanged || tr.getMeta("remote")) return true;
        if (propsRef.current.canEdit) return true;
        propsRef.current.onBlockedEdit();
        return false;
      },
    });

    const findPlugin = new Plugin<FindState>({
      key: findKey,
      state: {
        init: (_c, state) => {
          const matches = computeMatches(state.doc, findOpts.current, propsRef.current.showNotes);
          const index = Math.min(findIndex.current, Math.max(0, matches.length - 1));
          return { opts: findOpts.current, matches, index, deco: findDeco(state.doc, matches, index) };
        },
        apply: (tr, prev, _old, state) => {
          const meta = tr.getMeta(findKey) as { opts?: FindOptions | null; index?: number; refresh?: boolean } | undefined;
          if (!meta && !tr.docChanged) return prev;
          const opts = meta && "opts" in meta ? (meta.opts ?? null) : prev.opts;
          const matches = computeMatches(state.doc, opts, propsRef.current.showNotes);
          let index = meta?.index ?? prev.index;
          index = matches.length === 0 ? 0 : ((index % matches.length) + matches.length) % matches.length;
          return { opts, matches, index, deco: findDeco(state.doc, matches, index) };
        },
      },
      props: { decorations: (state) => findKey.getState(state)?.deco },
    });

    const commentPlugin = new Plugin<DecorationSet>({
      key: commentKey,
      state: {
        init: (_c, state) => commentDeco(state.doc, propsRef.current.comments),
        apply: (tr, prev, _old, state) => {
          if (tr.getMeta(commentKey) || tr.docChanged) return commentDeco(state.doc, propsRef.current.comments);
          return prev;
        },
      },
      props: { decorations: (state) => commentKey.getState(state) },
    });

    const uiDeco = new Plugin({
      props: {
        decorations(state) {
          const decos: Decoration[] = [];
          const { $head } = state.selection;
          if ($head.depth >= 1) {
            const start = $head.before(1);
            const node = $head.node(1);
            decos.push(Decoration.node(start, start + node.nodeSize, { class: "cur" }));
          }
          state.doc.forEach((node, offset) => {
            if (node.content.size !== 0) return;
            const isHead = node.type.name === "scene_heading";
            const isCur = $head.depth >= 1 && $head.before(1) === offset;
            if (!isHead && !isCur) return;
            const label = isHead ? "INT./EXT. LOCATION — TIME" : ELEMENT_META[node.type.name as ElementType]?.label ?? "";
            decos.push(
              Decoration.widget(
                offset + 1,
                () => {
                  const s = document.createElement("span");
                  s.className = "sp-ph";
                  s.textContent = label;
                  s.setAttribute("aria-hidden", "true");
                  return s;
                },
                { side: 1, key: `ph-${label}` },
              ),
            );
          });
          return DecorationSet.create(state.doc, decos);
        },
      },
    });

    const selectionReporter = new Plugin({
      view: () => ({
        update: (view, prev) => {
          if (prev && view.state.selection.eq(prev.selection) && view.state.doc.eq(prev.doc)) return;
          emitCursor(view.state);
          const anchor = anchorOf(view.state);
          if (!anchor || !view.hasFocus()) {
            propsRef.current.onSelection(null);
            return;
          }
          const a = view.coordsAtPos(view.state.selection.from);
          const b = view.coordsAtPos(view.state.selection.to);
          const rect = new DOMRect(Math.min(a.left, b.left), a.top, Math.abs(b.right - a.left), b.bottom - a.top);
          propsRef.current.onSelection({ rect, anchor });
        },
      }),
    });

    const state = EditorState.create({
      doc: docFromSnapshot(initial),
      plugins: [
        guard,
        keymap({
          Enter: enterCommand,
          "Shift-Enter": lineBreakCommand,
          Tab: (s, d) => setType(tabNext(s.selection.$head.node(1)?.type.name as ElementType ?? "action"), failMsg)(s, d) || true,
          "Shift-Tab": (s, d) => setType(tabPrev(s.selection.$head.node(1)?.type.name as ElementType ?? "action"), failMsg)(s, d) || true,
          ...typeKeys,
          "Mod-z": undoRedo(false),
          "Mod-y": undoRedo(true),
          "Shift-Mod-z": undoRedo(true),
          "Mod-f": () => {
            propsRef.current.onFindShortcut();
            return true;
          },
          "Alt-ArrowDown": moveScene(1),
          "Alt-ArrowUp": moveScene(-1),
          Escape: (_s, _d, view) => {
            view?.dom.blur();
            return true;
          },
        }),
        keymap(baseKeymap),
        idPlugin,
        findPlugin,
        commentPlugin,
        uiDeco,
        selectionReporter,
      ],
    });

    const view = new EditorView(host.current!, {
      state,
      attributes: {
        class: "spx-doc",
        role: "textbox",
        "aria-multiline": "true",
        "aria-label": "Screenplay",
        spellcheck: "true",
      },
      transformPasted: (slice) => {
        // Pasted blocks get fresh identities (the id plugin assigns them).
        const strip = (f: Fragment): Fragment => {
          const nodes: PMNode[] = [];
          f.forEach((n) => nodes.push(n.isTextblock ? n.type.create({ ...n.attrs, id: null }, n.content) : n));
          return Fragment.from(nodes);
        };
        return new Slice(strip(slice.content), slice.openStart, slice.openEnd);
      },
      dispatchTransaction(tr) {
        const next = view.state.apply(tr);
        view.updateState(next);
        if (tr.docChanged && !tr.getMeta("remote")) {
          syncRef.current?.markDirty();
          emitOutline(next.doc);
        }
      },
      handleDOMEvents: {
        blur: () => {
          void syncRef.current?.flush();
          return false;
        },
      },
    });
    viewRef.current = view;

    const applyRemote = (snap: ReturnType<typeof snapshotFromScenes>) => {
      const v = viewRef.current;
      if (!v) return;
      const { $head } = v.state.selection;
      const anchorId = $head.depth >= 1 ? ($head.node(1).attrs.id as string | null) : null;
      const offset = $head.parentOffset;
      const scroller = host.current?.closest(".spx-scroll") as HTMLElement | null;
      const top = scroller?.scrollTop ?? 0;
      const doc = docFromSnapshot(snap);
      let selPos = 1;
      doc.forEach((n, off) => {
        if (anchorId && n.attrs.id === anchorId) selPos = off + 1 + Math.min(offset, n.content.size);
      });
      const nextState = EditorState.create({
        doc,
        selection: TextSelection.create(doc, Math.min(selPos, doc.content.size)),
        plugins: v.state.plugins,
      });
      v.updateState(nextState);
      if (scroller) scroller.scrollTop = top;
      emitOutline(doc);
      emitCursor(nextState);
    };

    const sync = new SyncController(initial, p.doc.seq, {
      send: async (ops) => {
        const r = await call<ScreenplayEditResult>("screenplay.apply_edits", { draftId: p.doc.draft.id, ops });
        return { seq: r.seq };
      },
      read: () => snapshotFromDoc(viewRef.current!.state.doc),
      apply: applyRemote,
      onError: (e) => {
        if (e instanceof OpError && e.code === "permission.locked_draft") {
          propsRef.current.onBlockedEdit();
          return "revert";
        }
        if (e instanceof OpError && (e.retryable || e.is("storage"))) {
          reportError(e);
          return "retry";
        }
        reportError(e);
        return "revert";
      },
      onStatus: (s) => propsRef.current.onStatus(s),
    });
    syncRef.current = sync;
    emitOutline(view.state.doc);
    emitCursor(view.state);

    return () => {
      clearTimeout(outlineTimer.t);
      const s = syncRef.current;
      syncRef.current = null;
      if (s) void s.flush().finally(() => s.dispose());
      view.destroy();
      viewRef.current = null;
    };
  }, [draftId]);

  // Rust document updates (own saves, undo/redo, collaborators).
  useEffect(() => {
    syncRef.current?.receiveServer(props.doc.seq, snapshotFromScenes(props.doc.scenes));
  }, [props.doc]);

  // Comment highlights and note visibility refresh decorations.
  useEffect(() => {
    const v = viewRef.current;
    if (v) v.dispatch(v.state.tr.setMeta(commentKey, true).setMeta("remote", true));
  }, [props.comments]);
  useEffect(() => {
    const v = viewRef.current;
    if (v) v.dispatch(v.state.tr.setMeta(findKey, { refresh: true }).setMeta("remote", true));
  }, [props.showNotes]);

  useImperativeHandle(ref, (): EditorHandle => {
    const view = () => viewRef.current;
    const findResult = () => {
      const st = view() ? findKey.getState(view()!.state) : undefined;
      const r = { index: st?.index ?? 0, count: st?.matches.length ?? 0 };
      propsRef.current.onFindResult?.(r);
      return r;
    };
    const revealMatch = () => {
      const v = view();
      const st = v ? findKey.getState(v.state) : undefined;
      if (!v || !st || st.matches.length === 0) return;
      const m = st.matches[st.index];
      v.dispatch(v.state.tr.setSelection(TextSelection.create(v.state.doc, m.from, m.to)).scrollIntoView().setMeta("remote", true));
    };
    const scrollToPos = (pos: number) => {
      const v = view();
      if (!v) return;
      const dom = v.nodeDOM(pos);
      if (dom instanceof HTMLElement) dom.scrollIntoView({ block: "center" });
    };
    return {
      flush: async () => {
        await syncRef.current?.flush();
      },
      focus: () => view()?.focus(),
      jumpToScene: (sceneId) => {
        const v = view();
        if (!v) return false;
        let at = -1;
        v.state.doc.forEach((n, off) => {
          if (at < 0 && n.type.name === "scene_heading" && n.attrs.id === sceneId) at = off;
        });
        if (at < 0) return false;
        v.dispatch(v.state.tr.setSelection(TextSelection.create(v.state.doc, at + 1)).setMeta("remote", true));
        scrollToPos(at);
        v.focus();
        return true;
      },
      jumpToElement: (elementId, start, end) => {
        const v = view();
        if (!v) return false;
        let at = -1;
        let size = 0;
        v.state.doc.forEach((n, off) => {
          if (at < 0 && n.attrs.id === elementId) {
            at = off;
            size = n.content.size;
          }
        });
        if (at < 0) return false;
        const a = at + 1 + Math.min(start ?? 0, size);
        const b = at + 1 + Math.min(end ?? start ?? 0, size);
        v.dispatch(v.state.tr.setSelection(TextSelection.create(v.state.doc, a, b)).setMeta("remote", true));
        scrollToPos(at);
        v.focus();
        return true;
      },
      setElementType: (t) => {
        const v = view();
        if (!v) return;
        setType(t, (m) => toast.info(m))(v.state, v.dispatch);
        v.focus();
      },
      setFind: (o) => {
        const v = view();
        findOpts.current = o;
        findIndex.current = 0;
        if (!v) return { index: 0, count: 0 };
        // Start from the match at/after the cursor.
        const matches = computeMatches(v.state.doc, o, propsRef.current.showNotes);
        const from = v.state.selection.from;
        const idx = Math.max(0, matches.findIndex((m) => m.from >= from));
        v.dispatch(v.state.tr.setMeta(findKey, { opts: o, index: idx }).setMeta("remote", true));
        revealMatch();
        return findResult();
      },
      findStep: (dir) => {
        const v = view();
        const st = v ? findKey.getState(v.state) : undefined;
        if (!v || !st || st.matches.length === 0) return findResult();
        v.dispatch(v.state.tr.setMeta(findKey, { index: st.index + dir }).setMeta("remote", true));
        revealMatch();
        return findResult();
      },
      replaceCurrent: (replacement) => {
        const v = view();
        const st = v ? findKey.getState(v.state) : undefined;
        if (!v || !st || st.matches.length === 0) return findResult();
        if (!propsRef.current.canEdit) {
          propsRef.current.onBlockedEdit();
          return findResult();
        }
        const m = st.matches[st.index];
        const tr = v.state.tr;
        if (replacement) tr.replaceWith(m.from, m.to, schema.text(replacement));
        else tr.delete(m.from, m.to);
        v.dispatch(tr);
        revealMatch();
        return findResult();
      },
      selectionAnchor: () => {
        const v = view();
        if (!v) return null;
        const { $from, $to, empty } = v.state.selection;
        if (empty || $from.depth < 1 || $from.before(1) !== $to.before(1)) return null;
        const node = $from.parent;
        const id = node.attrs.id as string | null;
        const sceneId = sceneAt(v.state.doc, $from.pos);
        if (!id || !sceneId) return null;
        return {
          elementId: id,
          sceneId,
          isHeading: node.type.name === "scene_heading",
          start: $from.parentOffset,
          end: $to.parentOffset,
          text: v.state.doc.textBetween($from.pos, $to.pos, undefined, "\n"),
        };
      },
      sceneText: (sceneId) => {
        const v = view();
        if (!v) return "";
        const lines: string[] = [];
        let inScene = false;
        v.state.doc.forEach((n) => {
          if (n.type.name === "scene_heading") inScene = n.attrs.id === sceneId;
          if (inScene && n.type.name !== "note") {
            const t = blockText(n);
            const upper = ["scene_heading", "character", "transition", "shot"].includes(n.type.name);
            lines.push(upper ? t.toUpperCase() : t);
          }
        });
        return lines.join("\n\n");
      },
    };
  }, []);

  return <div ref={host} className={`spx-host${props.showNotes ? "" : " hide-notes"}${props.canEdit ? "" : " readonly"}`} />;
});
