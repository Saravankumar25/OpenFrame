// The structured screenplay editor (FSD §15–16, §92; UX §3.11, §4).
//
// ProseMirror handles cursor, selection, clipboard and typing like a normal
// writing app. Element structure, Enter/Tab progression and Ctrl+1…7 are
// screenplay rules layered on top. Undo/redo is Rust's (the editor flushes its
// pending edits first), so Ctrl+Z inside the page and the toolbar Undo are the
// same history. Keyboard shortcuts follow the person's settings (app/shortcuts);
// smart suggestions and spelling underlines are plugins (suggestPlugin, spelling).

import { forwardRef, useEffect, useImperativeHandle, useRef, useState, type KeyboardEvent as ReactKeyboardEvent } from "react";
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
import type { WritingSuggestionSources } from "../../../ipc/generated/WritingSuggestionSources";
import type { WritingSpellingCheck } from "../../../ipc/generated/WritingSpellingCheck";
import type { WritingSpellingSuggestions } from "../../../ipc/generated/WritingSpellingSuggestions";
import { ELEMENT_META, SHORTCUT_ORDER, newId, schema } from "./schema";
import { blockText, docFromSnapshot, mayAddBlocks, snapshotFromDoc, snapshotFromScenes } from "./model";
import { tabNext, tabPrev } from "./rules";
import {
  customDef,
  customDefByShortcut,
  enterCommand,
  headingPositions,
  insertPageBreak,
  lineBreakCommand,
  runShortcut,
  sceneAt,
  setCustom,
  setCustomDefs,
  setType,
  tabCommand,
} from "./commands";
import { computeMatches, type FindOptions } from "./find";
import { SyncController, type SyncStatus } from "./sync";
import { usePendingSave } from "../../../app/pendingSaves";
import { SUGGEST_LIST_ID, acceptSuggestion, createSuggestPlugin, refreshSuggestions, type SuggestView } from "./suggestPlugin";
import { createSpellPlugin, ignoreWord, missAt, nextMiss, resetSpelling } from "./spelling";

export interface OutlineScene {
  id: string;
  number: number;
  /** Scene number as shown (production label such as "12A" once numbers are frozen). */
  label: string;
  heading: string;
}

export interface CursorInfo {
  sceneId: string | null;
  elementId: string | null;
  elementType: ElementType;
}

export interface SelectionAnchor {
  /** Element id (body elements), or the scene id when the selection is in a heading. */
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
  /** Turn the current block into a custom element. */
  setCustomElement(customElementId: string): void;
  /** Insert an explicit page break after the current block (Insert page break shortcut). */
  insertPageBreak(): void;
  setFind(o: FindOptions | null): { index: number; count: number };
  findStep(dir: 1 | -1): { index: number; count: number };
  replaceCurrent(replacement: string): { index: number; count: number };
  selectionAnchor(): SelectionAnchor | null;
  /** Plain text of one scene (for "Copy Scene"). */
  sceneText(sceneId: string): string;
  /** Spelling settings or the personal dictionary changed: check everything again. */
  recheckSpelling(): void;
}

/** How the page is proofed: by OpenFrame (operating system dictionaries), by the web view's
 * own spell checker (systems without an OpenFrame adapter), or not at all. */
export interface ProofingMode {
  mode: "openframe" | "native" | "off";
  /** Main language tag, set as the page language. */
  lang: string;
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
  /** Project values for the suggestion list (Story Board characters, cast, locations). */
  suggestionSources?: WritingSuggestionSources | null;
  /** "Create New" in the suggestion list. `movedOn` is true when the same Enter moved on to
   * Dialogue. The returned promise settles once the value is recorded. */
  onCreateValue?: (kind: "character" | "location", name: string, movedOn: boolean) => Promise<void> | void;
  proofing?: ProofingMode;
}

/** "Create New" from the suggestion list. When the same Enter moved on to Dialogue, the new value
 * is recorded between the typed name and the Dialogue line (SyncController.saveThen), so Undo
 * takes back the Dialogue step, then the new character, then the typed name. */
function createInOrder(
  sync: SyncController | null,
  create: Props["onCreateValue"],
  kind: "character" | "location",
  name: string,
  movesOn: boolean,
): void {
  if (movesOn && sync) sync.saveThen(() => create?.(kind, name, true));
  else void create?.(kind, name, false);
}

interface SpellMenuState {
  from: number;
  to: number;
  word: string;
  left: number;
  top: number;
  suggestions: string[] | null;
}

// ------------------------------------------------------------- plugins

interface FindState {
  opts: FindOptions | null;
  matches: { from: number; to: number }[];
  index: number;
  deco: DecorationSet;
}
const findKey = new PluginKey<FindState>("sp-find");
const lookKey = new PluginKey<DecorationSet>("sp-custom-look");

/** Scene number labels of the open document (production numbers once frozen). */
let sceneLabels = new Map<string, string>();

/** The look of custom elements as node classes (the shared rule table decides positions in print). */
function lookDeco(doc: PMNode): DecorationSet {
  const decos: Decoration[] = [];
  doc.forEach((node, offset) => {
    if (node.type.name === "scene_heading") {
      const label = sceneLabels.get(node.attrs.id as string);
      if (label) decos.push(Decoration.node(offset, offset + node.nodeSize, { "data-number": label }));
      return;
    }
    if (node.type.name !== "custom") return;
    const d = customDef(node.attrs.customId);
    const cls = [`look-${d?.look ?? "action"}`, d?.uppercase ? "upper" : "", d?.italic ? "italic" : ""].filter(Boolean).join(" ");
    decos.push(Decoration.node(offset, offset + node.nodeSize, { class: cls, "data-label": d?.name ?? "Custom element" }));
  });
  return DecorationSet.create(doc, decos);
}

const commentKey = new PluginKey<DecorationSet>("sp-comments");

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

// ------------------------------------------------------------ component

export const ScreenplayEditor = forwardRef<EditorHandle, Props>(function ScreenplayEditor(props, ref) {
  const host = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const syncRef = useRef<SyncController | null>(null);
  // Window close, workspace switch and project switch wait for the text typed last (W2).
  usePendingSave(async () => {
    const s = syncRef.current;
    if (!s) return true;
    await s.flush();
    return !s.isDirty;
  });
  const propsRef = useRef(props);
  propsRef.current = props;
  const findOpts = useRef<FindOptions | null>(null);
  const findIndex = useRef(0);
  const draftId = props.doc.draft.id;
  const [suggestView, setSuggestView] = useState<SuggestView | null>(null);
  const [spellMenu, setSpellMenu] = useState<SpellMenuState | null>(null);

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
          if (n.type.name === "scene_heading") {
            const id = n.attrs.id as string;
            out.push({ id, number: out.length + 1, label: sceneLabels.get(id) ?? String(out.length + 1), heading: blockText(n) });
          }
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
    // Ctrl+Alt+1 to Ctrl+Alt+9: the custom elements that carry those shortcuts.
    const typeKeys: Record<string, Command> = {};
    for (let d = 1; d <= 9; d++) {
      typeKeys[`Mod-Alt-${d}`] = (st, dispatch) => {
        const def = customDefByShortcut(`Ctrl+Alt+${d}`);
        if (!def) return false;
        return setCustom(def.id, failMsg)(st, dispatch);
      };
    }

    const moveScene = (dir: 1 | -1): Command => (state, dispatch) => {
      const heads = headingPositions(state.doc);
      const cur = state.selection.from;
      const target = dir === 1 ? heads.find((h) => h + 1 > cur) : [...heads].reverse().find((h) => h + 1 < cur);
      if (target === undefined) return true;
      if (dispatch) dispatch(state.tr.setSelection(TextSelection.create(state.doc, target + 1)).scrollIntoView());
      return true;
    };

    // Open the spelling menu for the underlined word at `from` (right-click or F7).
    const openSpellMenu = (view: EditorView, miss: { from: number; to: number; word: string }, at?: { left: number; top: number }) => {
      let pos = at;
      if (!pos) {
        try {
          const c = view.coordsAtPos(miss.from);
          pos = { left: c.left, top: c.bottom + 4 };
        } catch {
          pos = { left: 0, top: 0 };
        }
      }
      setSpellMenu({ ...miss, left: pos.left, top: pos.top, suggestions: null });
      void call<WritingSpellingSuggestions>("writing.spelling_suggestions", { word: miss.word })
        .then((r) => setSpellMenu((m) => (m && m.from === miss.from && m.word === miss.word ? { ...m, suggestions: r.suggestions } : m)))
        .catch(() => setSpellMenu((m) => (m && m.from === miss.from ? { ...m, suggestions: [] } : m)));
    };

    // Shortcuts follow the person's settings (Settings > Keyboard shortcuts).
    const shortcutCommands: [string, Command][] = [
      ...[...SHORTCUT_ORDER, "note" as ElementType].map((t): [string, Command] => [`screenplay.element_${t}`, setType(t, failMsg)]),
      ["app.undo", undoRedo(false)],
      ["app.redo", undoRedo(true)],
      [
        "screenplay.find",
        () => {
          propsRef.current.onFindShortcut();
          return true;
        },
      ],
      ["screenplay.page_break", insertPageBreak],
      ["screenplay.next_scene", moveScene(1)],
      ["screenplay.previous_scene", moveScene(-1)],
      [
        "screenplay.next_misspelling",
        (state, _d, view) => {
          const miss = nextMiss(state, state.selection.from);
          if (!miss || !view) {
            toast.info("No spelling mistakes found.");
            return true;
          }
          view.dispatch(state.tr.setSelection(TextSelection.create(state.doc, miss.from, miss.to)).scrollIntoView().setMeta("remote", true));
          openSpellMenu(view, miss);
          return true;
        },
      ],
    ];
    const shortcutPlugin = new Plugin({
      props: {
        handleKeyDown(view, e) {
          return runShortcut(e, shortcutCommands, view.state, view.dispatch, view);
        },
      },
    });

    const suggestPlugin = createSuggestPlugin({
      sources: () => propsRef.current.suggestionSources ?? null,
      canEdit: () => propsRef.current.canEdit,
      onCreate: (kind, name, movesOn) => createInOrder(syncRef.current, propsRef.current.onCreateValue, kind, name, movesOn),
      // A known name filled by Enter is saved on its own before the Dialogue line (see createInOrder).
      beforeMoveOn: () => void syncRef.current?.flush(),
      onChange: setSuggestView,
    });

    const spellPlugin = createSpellPlugin({
      check: (items) => call<WritingSpellingCheck>("writing.check_spelling", { items }),
      active: () => propsRef.current.proofing?.mode === "openframe",
      knownNames: () => {
        const src = propsRef.current.suggestionSources;
        return src ? [...src.characters, ...src.locations].map((v) => v.name) : [];
      },
      onError: (e) => {
        // Spelling is a comfort feature: a failure never interrupts writing.
        if (e instanceof OpError) console.warn(`[spelling] ${e.code}`, e.detail ?? "");
      },
    });

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

    const lookPlugin = new Plugin<DecorationSet>({
      key: lookKey,
      state: {
        init: (_c, state) => lookDeco(state.doc),
        apply: (tr, prev, _old, state) => (tr.getMeta(lookKey) || tr.docChanged ? lookDeco(state.doc) : prev),
      },
      props: { decorations: (state) => lookKey.getState(state) },
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
            const label = isHead
              ? "INT./EXT. LOCATION - TIME"
              : node.type.name === "custom"
                ? (customDef(node.attrs.customId)?.name ?? ELEMENT_META.custom.label)
                : (ELEMENT_META[node.type.name as ElementType]?.label ?? "");
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
        suggestPlugin,
        shortcutPlugin,
        keymap({
          Enter: enterCommand,
          "Shift-Enter": lineBreakCommand,
          Tab: tabCommand(tabNext, failMsg),
          "Shift-Tab": tabCommand(tabPrev, failMsg),
          ...typeKeys,
          Escape: (_s, _d, view) => {
            view?.dom.blur();
            return true;
          },
        }),
        keymap(baseKeymap),
        idPlugin,
        findPlugin,
        lookPlugin,
        commentPlugin,
        uiDeco,
        selectionReporter,
        spellPlugin,
      ],
    });

    const view = new EditorView(host.current!, {
      state,
      attributes: {
        class: "spx-doc",
        role: "textbox",
        "aria-multiline": "true",
        "aria-label": "Screenplay",
        // OpenFrame draws its own underlines; the web view's checker is used only where
        // OpenFrame has no dictionary adapter.
        spellcheck: p.proofing?.mode === "native" ? "true" : "false",
        lang: p.proofing?.lang ?? "en-US",
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
        blur: (v) => {
          void syncRef.current?.flush();
          // The suggestion list belongs to the caret: hide it while the page has no focus.
          setSuggestView(null);
          v.dom.removeAttribute("aria-controls");
          v.dom.removeAttribute("aria-activedescendant");
          return false;
        },
        focus: (v) => {
          refreshSuggestions(v);
          return false;
        },
        contextmenu: (v, e) => {
          const hit = v.posAtCoords({ left: e.clientX, top: e.clientY });
          const miss = hit ? missAt(v.state, hit.pos) : null;
          if (!miss) return false;
          e.preventDefault();
          openSpellMenu(v, miss, { left: e.clientX, top: e.clientY });
          return true;
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

  // Custom element definitions (looks, names, shortcuts) of this screenplay.
  setCustomDefs(props.doc.customElements);
  sceneLabels = new Map(props.doc.scenes.map((sc) => [sc.id, sc.label]));
  useEffect(() => {
    const v = viewRef.current;
    if (v) v.dispatch(v.state.tr.setMeta(lookKey, true).setMeta("remote", true));
  }, [props.doc.customElements, props.doc.scenes]);

  // Rust document updates (own saves, undo/redo, collaborators).
  useEffect(() => {
    syncRef.current?.receiveServer(props.doc.seq, snapshotFromScenes(props.doc.scenes));
  }, [props.doc]);

  // Scrolling the page moves the suggestion list with the caret and closes the spelling menu.
  useEffect(() => {
    const scroller = host.current?.closest(".spx-scroll");
    if (!scroller) return;
    let frame = 0;
    const onScroll = () => {
      setSpellMenu(null);
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        // An empty transaction redraws the plugin views, which place the list at the caret again.
        const v = viewRef.current;
        if (v && v.hasFocus()) v.dispatch(v.state.tr.setMeta("remote", true));
      });
    };
    scroller.addEventListener("scroll", onScroll, { passive: true });
    return () => {
      cancelAnimationFrame(frame);
      scroller.removeEventListener("scroll", onScroll);
    };
  }, [draftId]);

  // Project values for the suggestion list arrived or changed.
  useEffect(() => {
    const v = viewRef.current;
    if (v) refreshSuggestions(v);
  }, [props.suggestionSources]);

  // Spelling mode or language changed: update the page and check again.
  const proofMode = props.proofing?.mode ?? "off";
  const proofLang = props.proofing?.lang ?? "en-US";
  useEffect(() => {
    const v = viewRef.current;
    if (!v) return;
    v.dom.setAttribute("spellcheck", proofMode === "native" ? "true" : "false");
    v.dom.setAttribute("lang", proofLang);
    resetSpelling(v);
  }, [proofMode, proofLang]);

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
      setCustomElement: (id) => {
        const v = view();
        if (!v) return;
        setCustom(id, (m) => toast.info(m))(v.state, v.dispatch);
        v.focus();
      },
      insertPageBreak: () => {
        const v = view();
        if (!v) return;
        insertPageBreak(v.state, v.dispatch);
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
          if (inScene && n.type.name !== "note" && n.type.name !== "page_break") {
            const t = blockText(n);
            const upper = ["scene_heading", "character", "transition", "shot"].includes(n.type.name);
            lines.push(upper ? t.toUpperCase() : t);
          }
        });
        return lines.join("\n\n");
      },
      recheckSpelling: () => {
        const v = view();
        if (v) resetSpelling(v);
      },
    };
  }, []);

  const accept = (index: number) => {
    const v = viewRef.current;
    if (!v) return;
    acceptSuggestion(v, index, { onCreate: (kind, name, movesOn) => createInOrder(syncRef.current, propsRef.current.onCreateValue, kind, name, movesOn) });
    v.focus();
  };

  const closeSpellMenu = () => {
    setSpellMenu(null);
    viewRef.current?.focus();
  };

  const spellAction = (m: SpellMenuState, action: { replace: string } | "add" | "ignore") => {
    const v = viewRef.current;
    setSpellMenu(null);
    if (!v) return;
    if (action === "ignore") {
      ignoreWord(v, m.word);
    } else if (action === "add") {
      // The dictionaryChanged event checks the page again (this window and any other).
      void call("writing.dictionary_add", { word: m.word })
        .then(() => toast.success(`Added “${m.word}” to your dictionary`))
        .catch(reportError);
    } else if (v.state.doc.textBetween(m.from, m.to) === m.word) {
      if (!propsRef.current.canEdit) {
        propsRef.current.onBlockedEdit();
      } else {
        v.dispatch(v.state.tr.insertText(action.replace, m.from, m.to));
      }
    }
    v.focus();
  };

  return (
    <>
      <div ref={host} className={`spx-host${props.showNotes ? "" : " hide-notes"}${props.canEdit ? "" : " readonly"}`} />
      {suggestView && (
        <div className="sp-suggest card" style={popupPosition(suggestView.left, suggestView.top, suggestView.above, 300, 380)}>
          <ul id={SUGGEST_LIST_ID} role="listbox" aria-label="Suggestions" className="sp-suggest-list">
          {suggestView.items.map((it, i) => (
            <li
              key={`${it.label}-${i}`}
              id={`${SUGGEST_LIST_ID}-${i}`}
              role="option"
              aria-selected={i === suggestView.index}
              className={`sp-suggest-item${i === suggestView.index ? " on" : ""}${it.create ? " create" : ""}`}
              onMouseDown={(e) => e.preventDefault()}
              onClick={() => accept(i)}
            >
              <span className="grow">{it.label}</span>
              <span className="xs muted">{it.source}</span>
            </li>
          ))}
          </ul>
          <div className="sp-suggest-hint xs muted" aria-hidden="true">Enter accepts, Esc keeps your text</div>
        </div>
      )}
      <span className="sr-only" aria-live="polite">
        {suggestView ? `${suggestView.items.length} suggestion${suggestView.items.length === 1 ? "" : "s"}` : ""}
      </span>
      {spellMenu && <SpellMenu menu={spellMenu} canEdit={props.canEdit} onAction={(a) => spellAction(spellMenu, a)} onClose={closeSpellMenu} />}
    </>
  );
});

/** The menu for an underlined word: suggestions, add to dictionary, ignore. Keyboard: arrows
 * move, Enter chooses, Esc closes and returns to the page. */
function SpellMenu({ menu, canEdit, onAction, onClose }: {
  menu: SpellMenuState;
  canEdit: boolean;
  onAction: (a: { replace: string } | "add" | "ignore") => void;
  onClose: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const items: { label: string; action: { replace: string } | "add" | "ignore" }[] = [
    ...(canEdit ? (menu.suggestions ?? []).map((w) => ({ label: w, action: { replace: w } })) : []),
    { label: "Add to dictionary", action: "add" as const },
    { label: "Ignore", action: "ignore" as const },
  ];
  const count = items.length;
  const loading = menu.suggestions === null;
  useEffect(() => {
    // Focus the first item once the menu opens and once suggestions arrive, unless the person
    // has already moved within the menu.
    if (!ref.current?.contains(document.activeElement)) ref.current?.querySelector<HTMLButtonElement>("[role=menuitem]")?.focus();
  }, [loading, count]);
  useEffect(() => {
    const onDown = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    window.addEventListener("mousedown", onDown);
    return () => window.removeEventListener("mousedown", onDown);
  }, [onClose]);
  const move = (e: ReactKeyboardEvent) => {
    const buttons = Array.from(ref.current?.querySelectorAll<HTMLButtonElement>("[role=menuitem]") ?? []);
    const i = buttons.indexOf(document.activeElement as HTMLButtonElement);
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const n = buttons.length;
      buttons[(i + (e.key === "ArrowDown" ? 1 : n - 1)) % n]?.focus();
    } else if (e.key === "Escape" || e.key === "Tab") {
      e.preventDefault();
      onClose();
    }
  };
  return (
    <div ref={ref} role="menu" aria-label={`Spelling: ${menu.word}`} className="sp-spellmenu card" style={popupPosition(menu.left, menu.top, menu.top - 24, 240, 260)} onKeyDown={move}>
      {loading && canEdit && <div className="xs muted sp-spellmenu-note">Looking for suggestions…</div>}
      {!loading && canEdit && menu.suggestions?.length === 0 && <div className="xs muted sp-spellmenu-note">No suggestions</div>}
      {items.map((it) => (
        <button key={it.label} type="button" role="menuitem" className={`sp-spellmenu-item${typeof it.action === "object" ? " b" : ""}`} onClick={() => onAction(it.action)}>
          {it.label}
        </button>
      ))}
    </div>
  );
}

/** Keep a popup inside the window: shift it left near the right edge and open it above the line
 * when there is no room below. */
function popupPosition(left: number, top: number, above: number, height: number, width: number): { left: number; top?: number; bottom?: number } {
  const w = typeof window === "undefined" ? 1280 : window.innerWidth;
  const h = typeof window === "undefined" ? 800 : window.innerHeight;
  const x = Math.max(8, Math.min(left, w - width - 8));
  if (top + height > h && above - height > 0) return { left: x, bottom: h - above };
  return { left: x, top };
}
