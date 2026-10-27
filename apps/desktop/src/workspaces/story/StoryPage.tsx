// The Story page (Story > Story tab): one writing page for the writer's free text, shown as an A4
// sheet like the screenplay page. It is not linked to cards, scenes, the screenplay or the
// assistant. The text lives in the project (story.page.get / story.page.save) with its formatting:
// font size, bold, italic, underline and highlight for any stretch of text (owner, 2026-10-27).
// Export writes TXT (words only), PDF and Word (with the formatting, on A4).
//
// Nothing typed is lost: the page is saved 400 ms after the last change (like the screenplay), on
// leaving the editor, when the tab or workspace changes (unmount flush), when the project closes
// and when the window closes (all through the shared pending-save registry).

import { useCallback, useEffect, useRef, useState } from "react";
import { EditorState, type Transaction } from "prosemirror-state";
import { EditorView } from "prosemirror-view";
import { history, redo, undo } from "prosemirror-history";
import { keymap } from "prosemirror-keymap";
import { baseKeymap, toggleMark } from "prosemirror-commands";
import type { MarkType, Node as PMNode } from "prosemirror-model";
import { Bold, Highlighter, Italic, Moon, Underline, ZoomIn, ZoomOut } from "lucide-react";
import { EmptyState, PageHeader, Skeleton } from "../../design-system";
import { story } from "../../api/story";
import { OpError } from "../../ipc/client";
import { StoryPageExportButton } from "../../features/export/buttons";
import { useDebouncedSave } from "../notes/useDebouncedSave";
import { BODY_SIZE, SIZES, contentOf, docFrom, storySchema, wordCount } from "./storyDoc";

const SAVE_DELAY_MS = 400;

type Status = "saved" | "saving" | "failed";
const STATUS_LABEL: Record<Status, string> = { saved: "Saved", saving: "Saving…", failed: "Not saved yet" };

// Saves run one after another, also across a quick leave and return: the page that opens again
// waits for the save of the page that was just closed, so it never shows older text.
let lastSave: Promise<unknown> = Promise.resolve();
const queueSave = <T,>(job: () => Promise<T>): Promise<T> => {
  const run = lastSave.then(job, job);
  lastSave = run.catch(() => undefined);
  return run;
};
async function settledSaves(): Promise<void> {
  let seen: Promise<unknown>;
  do {
    seen = lastSave;
    await seen;
  } while (seen !== lastSave);
}

/** The editor behind each Story page element, for the tests that type into it. */
export const storyEditors = new WeakMap<Element, EditorView>();

// ------------------------------------------------------------------ zoom

const ZOOM_KEY = "openframe.story.zoom";
const ZOOM_MIN = 0.5;
const ZOOM_MAX = 2;

/** The page zoom is a per-user view preference, never project data. */
const DARK_KEY = "openframe.story.darkPage";

function readZoom(): number {
  try {
    const z = Number(window.localStorage.getItem(ZOOM_KEY));
    return Number.isFinite(z) && z >= ZOOM_MIN && z <= ZOOM_MAX ? z : 1;
  } catch {
    return 1;
  }
}

function writeZoom(z: number): number {
  const v = Math.round(Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, z)) * 100) / 100;
  try {
    window.localStorage.setItem(ZOOM_KEY, String(v));
  } catch {
    // The zoom still applies for this session.
  }
  return v;
}

// ------------------------------------------------------------------ formatting

type Tools = { bold: boolean; italic: boolean; underline: boolean; highlight: boolean; size: number | null };
const NO_TOOLS: Tools = { bold: false, italic: false, underline: false, highlight: false, size: BODY_SIZE };

function sizeOfMarks(marks: readonly { type: MarkType; attrs: Record<string, unknown> }[]): number {
  const m = marks.find((x) => x.type === storySchema.marks.size);
  return m ? (m.attrs.pt as number) : BODY_SIZE;
}

/** What the toolbar shows for the selection: a mark is on when any of the selected text has it
 *  (a click then takes it off, as in Word); the size is blank when the selection mixes sizes. */
function toolsOf(state: EditorState): Tools {
  const { from, to, empty, $from } = state.selection;
  const m = storySchema.marks;
  if (empty) {
    const marks = state.storedMarks ?? $from.marks();
    const has = (t: MarkType) => !!t.isInSet(marks);
    return { bold: has(m.bold), italic: has(m.italic), underline: has(m.underline), highlight: has(m.highlight), size: sizeOfMarks(marks) };
  }
  const sizes = new Set<number>();
  state.doc.nodesBetween(from, to, (n) => {
    if (n.isText) sizes.add(sizeOfMarks(n.marks));
  });
  const has = (t: MarkType) => state.doc.rangeHasMark(from, to, t);
  return {
    bold: has(m.bold),
    italic: has(m.italic),
    underline: has(m.underline),
    highlight: has(m.highlight),
    size: sizes.size === 1 ? [...sizes][0] : sizes.size === 0 ? BODY_SIZE : null,
  };
}

/** Set the size of the selection (or of what is typed next when nothing is selected). */
function setSize(state: EditorState, dispatch: (tr: Transaction) => void, pt: number): boolean {
  const t = storySchema.marks.size;
  const { from, to, empty } = state.selection;
  const tr = state.tr;
  if (empty) {
    tr.removeStoredMark(t);
    if (pt !== BODY_SIZE) tr.addStoredMark(t.create({ pt }));
  } else {
    tr.removeMark(from, to, t);
    if (pt !== BODY_SIZE) tr.addMark(from, to, t.create({ pt }));
  }
  dispatch(tr.scrollIntoView());
  return true;
}

/** One step up or down the toolbar sizes from the size at the cursor. */
function stepSize(state: EditorState, dispatch: (tr: Transaction) => void, dir: 1 | -1): boolean {
  const now = toolsOf(state).size ?? BODY_SIZE;
  const next = dir > 0 ? SIZES.find((s) => s > now) : [...SIZES].reverse().find((s) => s < now);
  return next === undefined ? false : setSize(state, dispatch, next);
}

const isEmptyDoc = (doc: PMNode) => doc.childCount === 1 && doc.firstChild!.content.size === 0;

// ------------------------------------------------------------------ page

export function StoryPage() {
  const [initial, setInitial] = useState<{ text: string; format: string | null } | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [status, setStatus] = useState<Status>("saved");
  const [words, setWords] = useState(0);
  const [tools, setTools] = useState<Tools>(NO_TOOLS);
  const rev = useRef(0);
  const latest = useRef<PMNode | null>(null);
  const viewRef = useRef<EditorView | null>(null);
  const hostRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let live = true;
    void settledSaves()
      .then(() => story.pageGet())
      .then(
        (p) => {
          if (!live) return;
          rev.current = p.rev;
          setWords(wordCount(p.text));
          setInitial({ text: p.text, format: p.format });
        },
        (e: unknown) => live && setLoadError(e instanceof Error ? e.message : "The page could not be read."),
      );
    return () => {
      live = false;
    };
  }, []);

  const persist = useCallback(
    (doc: PMNode) =>
      queueSave(async () => {
        const { text, format } = contentOf(doc);
        if (latest.current === doc) setWords(wordCount(text));
        try {
          let saved;
          try {
            saved = await story.pageSave(text, format, rev.current);
          } catch (e) {
            // Another window saved first. Take its revision and keep what the writer typed.
            if (!(e instanceof OpError) || !e.code.startsWith("conflict")) throw e;
            rev.current = (await story.pageGet()).rev;
            saved = await story.pageSave(text, format, rev.current);
          }
          rev.current = saved.rev;
          if (latest.current === doc) setStatus("saved");
        } catch (e) {
          setStatus("failed");
          throw e;
        }
      }),
    [],
  );
  const { schedule, flush } = useDebouncedSave(persist, SAVE_DELAY_MS);
  const flushRef = useRef(flush);
  useEffect(() => {
    flushRef.current = flush;
  }, [flush]);

  // The editor is made once, when the page has been read.
  useEffect(() => {
    const host = hostRef.current;
    if (!initial || !host) return;
    const mark = storySchema.marks;
    const view: EditorView = new EditorView(host, {
      state: EditorState.create({
        doc: docFrom(initial.text, initial.format),
        plugins: [
          history(),
          keymap({
            "Mod-z": undo,
            "Mod-y": redo,
            "Mod-Shift-z": redo,
            "Mod-b": toggleMark(mark.bold),
            "Mod-i": toggleMark(mark.italic),
            "Mod-u": toggleMark(mark.underline),
            "Mod-Shift-h": toggleMark(mark.highlight),
            "Mod-]": (s, d) => !!d && stepSize(s, d, 1),
            "Mod-[": (s, d) => !!d && stepSize(s, d, -1),
            "Shift-Enter": baseKeymap.Enter,
          }),
          keymap(baseKeymap),
        ],
      }),
      attributes: (state) => ({
        "aria-label": "Story text",
        "aria-multiline": "true",
        role: "textbox",
        spellcheck: "true",
        dir: "auto",
        class: "story-a4-text",
        "data-empty": isEmptyDoc(state.doc) ? "true" : "false",
        "data-placeholder": "Start writing your story here.",
      }),
      handleDOMEvents: {
        blur: () => {
          void flushRef.current();
          return false;
        },
      },
      dispatchTransaction(tr) {
        const next = view.state.apply(tr);
        view.updateState(next);
        setTools(toolsOf(next));
        if (tr.docChanged) {
          latest.current = next.doc;
          setStatus("saving");
          schedule(next.doc);
        }
      },
    });
    viewRef.current = view;
    storyEditors.set(view.dom, view);
    setTools(toolsOf(view.state));
    view.focus();
    return () => {
      storyEditors.delete(view.dom);
      view.destroy();
      viewRef.current = null;
    };
    // `schedule` is stable for the life of the page.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [initial]);

  // Toolbar commands act on the editor and give it the focus back.
  const run = (cmd: (s: EditorState, d: (tr: Transaction) => void) => boolean) => {
    const v = viewRef.current;
    if (!v) return;
    cmd(v.state, v.dispatch);
    v.focus();
  };

  // The writer's page zoom: Ctrl+wheel and touchpad pinch (which arrives as Ctrl+wheel) zoom the
  // page only, not the whole window; the buttons do the same.
  const [zoom, setZoomState] = useState(readZoom);
  // Dark page is a view preference for the screen only, like the screenplay's; exports stay white.
  const [darkPage, setDarkPageState] = useState(() => {
    try {
      return window.localStorage.getItem(DARK_KEY) === "1";
    } catch {
      return false;
    }
  });
  const setDarkPage = (on: boolean) => {
    setDarkPageState(on);
    try {
      window.localStorage.setItem(DARK_KEY, on ? "1" : "0");
    } catch {
      // Still applies for this session.
    }
  };
  const setZoom = useCallback((z: number) => setZoomState(writeZoom(z)), []);
  const deskRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const el = deskRef.current;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      if (!e.ctrlKey) return;
      e.preventDefault();
      setZoomState((z) => writeZoom(z * Math.exp(-e.deltaY / 400)));
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  });

  if (loadError) {
    return (
      <div className="content fill">
        <PageHeader title="Story" />
        <EmptyState title="The Story page could not be opened.">{loadError}</EmptyState>
      </div>
    );
  }

  const mark = storySchema.marks;
  return (
    <div className="content fill story-write">
      <PageHeader
        title="Story"
        sub="Write your story freely. This page is not linked to anything else."
        actions={<StoryPageExportButton beforeOpen={flush} />}
      />
      {initial === null ? (
        <Skeleton h={28} w={240} />
      ) : (
        <>
          <div className="story-tools" role="toolbar" aria-label="Formatting">
            <select
              className="story-size"
              aria-label="Font size"
              title="Font size (Ctrl+[ and Ctrl+])"
              value={tools.size === null ? "" : String(tools.size)}
              onChange={(e) => {
                const pt = Number(e.target.value);
                if (pt) run((s, d) => setSize(s, d, pt));
              }}
            >
              {tools.size === null && <option value="">Mixed</option>}
              {tools.size !== null && !SIZES.includes(tools.size) && <option value={String(tools.size)}>{tools.size}</option>}
              {SIZES.map((s) => (
                <option key={s} value={String(s)}>
                  {s}
                </option>
              ))}
            </select>
            <span className="story-tools-sep" aria-hidden />
            {(
              [
                ["bold", "Bold", "Ctrl+B", <Bold key="b" size={15} />],
                ["italic", "Italic", "Ctrl+I", <Italic key="i" size={15} />],
                ["underline", "Underline", "Ctrl+U", <Underline key="u" size={15} />],
                ["highlight", "Highlight", "Ctrl+Shift+H", <Highlighter key="h" size={15} />],
              ] as const
            ).map(([key, label, keys, icon]) => (
              <button
                key={key}
                type="button"
                className="iconbtn story-tool"
                aria-label={label}
                aria-pressed={tools[key]}
                title={`${label} (${keys})`}
                // Keep the selection in the editor while the button is pressed.
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => run(toggleMark(mark[key]))}
              >
                {icon}
              </button>
            ))}
            <span className="grow" />
            <button
              type="button"
              className="iconbtn story-tool"
              aria-label="Dark page"
              aria-pressed={darkPage}
              title={darkPage ? "Light page" : "Dark page"}
              onClick={() => setDarkPage(!darkPage)}
            >
              <Moon size={15} />
            </button>
            <div className="sp-zoom" role="group" aria-label="Page zoom">
              <button type="button" className="iconbtn" aria-label="Zoom out" title="Zoom out (Ctrl+wheel)" disabled={zoom <= ZOOM_MIN} onClick={() => setZoom(zoom - 0.1)}>
                <ZoomOut size={14} />
              </button>
              <button type="button" className="sp-zoom-val" title="Reset the page zoom" onClick={() => setZoom(1)}>
                {Math.round(zoom * 100)}%
              </button>
              <button type="button" className="iconbtn" aria-label="Zoom in" title="Zoom in (Ctrl+wheel)" disabled={zoom >= ZOOM_MAX} onClick={() => setZoom(zoom + 0.1)}>
                <ZoomIn size={14} />
              </button>
            </div>
          </div>
          <div className="story-desk" ref={deskRef}>
            <div className="story-a4" data-paper={darkPage ? "dark" : undefined} style={{ "--story-zoom": zoom } as React.CSSProperties} ref={hostRef} />
          </div>
          <div className="story-write-foot xs muted">
            <span>{words === 1 ? "1 word" : `${words.toLocaleString()} words`}</span>
            <span role="status" aria-live="polite">
              {STATUS_LABEL[status]}
            </span>
          </div>
        </>
      )}
    </div>
  );
}
