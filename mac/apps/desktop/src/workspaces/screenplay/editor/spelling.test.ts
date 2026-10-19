// Spelling underlines (SCRIPT-020): what is sent, what is drawn, and that underlines follow
// typing without ever changing the text.

import { afterEach, describe, expect, it, vi } from "vitest";
import { EditorState, TextSelection } from "prosemirror-state";
import { EditorView } from "prosemirror-view";
import type { SpellingItem } from "../../../ipc/generated/SpellingItem";
import type { WritingSpellingCheck } from "../../../ipc/generated/WritingSpellingCheck";
import { docFromSnapshot, type Snapshot } from "./model";
import { MAX_BATCH_ITEMS, batches, checksType, createSpellPlugin, filterMisses, ignoreWord, missAt, nextMiss, resetSpelling, spellKey } from "./spelling";

const script: Snapshot = [
  {
    id: "s1",
    heading: "INT. DEPOT - NIGHT",
    elements: [
      { id: "a1", type: "action", text: "Meera walks into teh depot." },
      { id: "c1", type: "character", text: "MEERA" },
      { id: "d1", type: "dialogue", text: "మీరా వచ్చింది. Teh bus is late." },
    ],
  },
];

/** A fake Rust check: every "teh" (any case) is unknown, plus "Meera" so the name filter shows. */
function fakeCheck(items: SpellingItem[]): Promise<WritingSpellingCheck> {
  return Promise.resolve({
    enabled: true,
    missingLanguages: [],
    results: items.map((it) => {
      const misses: { start: number; end: number }[] = [];
      for (const w of ["teh", "Teh", "Meera"]) {
        let i = it.text.indexOf(w);
        while (i >= 0) {
          misses.push({ start: i, end: i + w.length });
          i = it.text.indexOf(w, i + 1);
        }
      }
      return { key: it.key, misses };
    }),
  });
}

describe("what is checked", () => {
  it("never checks character cues", () => {
    expect(checksType("character")).toBe(false);
    for (const t of ["action", "dialogue", "scene_heading", "parenthetical", "transition", "shot", "note"]) expect(checksType(t)).toBe(true);
  });
  it("splits large scripts into calls inside the Rust limits", () => {
    const items = Array.from({ length: MAX_BATCH_ITEMS * 2 + 5 }, (_, i) => ({ key: `k${i}`, text: "word" }));
    expect(batches(items).map((b) => b.length)).toEqual([MAX_BATCH_ITEMS, MAX_BATCH_ITEMS, 5]);
    const big = [{ key: "a", text: "x".repeat(300) }, { key: "b", text: "y".repeat(300) }];
    expect(batches(big, 400, 400).map((b) => b.length)).toEqual([1, 1]);
    // Bytes, not characters: Telugu takes three bytes per character.
    expect(batches([{ key: "t", text: "మ".repeat(100) }, { key: "u", text: "a" }], 400, 300).length).toBe(2);
  });
  it("drops names and ignored words, case-insensitively", () => {
    const text = "Meera walks into teh depot.";
    const misses = [{ start: 0, end: 5 }, { start: 17, end: 20 }];
    expect(filterMisses(text, misses, new Set(["meera"]), new Set())).toEqual([{ start: 17, end: 20 }]);
    expect(filterMisses(text, misses, new Set(), new Set(["teh"]))).toEqual([{ start: 0, end: 5 }]);
  });
});

describe("underlines in the editor", () => {
  let view: EditorView | null = null;
  afterEach(() => {
    view?.destroy();
    view = null;
    vi.useRealTimers();
  });

  async function mount(active = true) {
    vi.useFakeTimers();
    const check = vi.fn(fakeCheck);
    const doc = docFromSnapshot(script);
    const plugin = createSpellPlugin({ check, active: () => active, knownNames: () => [] });
    const state = EditorState.create({ doc, plugins: [plugin], selection: TextSelection.create(doc, 1) });
    const host = document.createElement("div");
    document.body.appendChild(host);
    view = new EditorView(host, { state });
    await vi.advanceTimersByTimeAsync(700);
    return { view, check };
  }
  const words = (v: EditorView) => (spellKey.getState(v.state)?.deco.find() ?? []).map((d) => v.state.doc.textBetween(d.from, d.to));

  it("underlines unknown words but not character names used in the script", async () => {
    const { view, check } = await mount();
    expect(check).toHaveBeenCalledTimes(1);
    const sent = check.mock.calls[0][0].map((i) => i.key);
    expect(sent).not.toContain("c1");
    expect(words(view).sort()).toEqual(["Teh", "teh"]);
    expect(view.dom.querySelectorAll(".sp-miss").length).toBe(2);
  });

  it("only changed blocks are checked again; a changed word loses its underline at once", async () => {
    const { view, check } = await mount();
    const at = view.state.doc.textContent.indexOf("teh");
    expect(at).toBeGreaterThan(0);
    // Fix the typo in the action line.
    const miss = nextMiss(view.state, 0)!;
    expect(miss.word.toLowerCase()).toBe("teh");
    view.dispatch(view.state.tr.insertText("the", miss.from, miss.to));
    expect(words(view)).toEqual(["Teh"]);
    await vi.advanceTimersByTimeAsync(700);
    expect(check).toHaveBeenCalledTimes(2);
    expect(check.mock.calls[1][0].map((i) => i.key)).toEqual(["a1"]);
    expect(words(view)).toEqual(["Teh"]);
  });

  it("typing a letter and deleting it checks the block again (the underline comes back)", async () => {
    const { view, check } = await mount();
    const miss = nextMiss(view.state, 0)!;
    view.dispatch(view.state.tr.insertText("x", miss.to));
    view.dispatch(view.state.tr.delete(miss.to, miss.to + 1));
    expect(view.state.doc.textBetween(miss.from, miss.to)).toBe(miss.word);
    await vi.advanceTimersByTimeAsync(700);
    expect(check.mock.calls.at(-1)![0].map((i) => i.key)).toEqual(["a1"]);
    expect(words(view)).toContain(miss.word);
  });

  it("a result for text that changed while it was being checked is not drawn", async () => {
    vi.useFakeTimers();
    let answer: (r: WritingSpellingCheck) => void = () => undefined;
    const check = vi.fn(
      () =>
        new Promise<WritingSpellingCheck>((res) => {
          answer = res;
        }),
    );
    const doc = docFromSnapshot(script);
    const plugin = createSpellPlugin({ check, active: () => true, knownNames: () => [] });
    view = new EditorView(document.body.appendChild(document.createElement("div")), { state: EditorState.create({ doc, plugins: [plugin] }) });
    await vi.advanceTimersByTimeAsync(700);
    expect(check).toHaveBeenCalledTimes(1);
    // The writer changes the action line before the answer comes back.
    let a1Pos = -1;
    view.state.doc.forEach((n, pos) => {
      if (n.attrs.id === "a1") a1Pos = pos;
    });
    view.dispatch(view.state.tr.insertText("Rain. ", a1Pos + 1));
    answer({ enabled: true, missingLanguages: [], results: [{ key: "a1", misses: [{ start: 17, end: 20 }] }] });
    await vi.advanceTimersByTimeAsync(0);
    // Nothing is drawn at the old offsets; the line is checked again in the next round.
    expect(words(view)).toEqual([]);
  });

  it("Ignore hides a word for the session; reset checks everything again", async () => {
    const { view, check } = await mount();
    ignoreWord(view, "teh");
    expect(words(view)).toEqual([]);
    resetSpelling(view);
    await vi.advanceTimersByTimeAsync(700);
    expect(check).toHaveBeenCalledTimes(2);
    // Ignored words stay hidden after a recheck.
    expect(words(view)).toEqual([]);
  });

  it("finds the word under the caret and the next one, wrapping", async () => {
    const { view } = await mount();
    const first = nextMiss(view.state, 0)!;
    expect(missAt(view.state, first.from + 1)?.word).toBe(first.word);
    const second = nextMiss(view.state, first.from)!;
    expect(second.from).toBeGreaterThan(first.from);
    expect(nextMiss(view.state, second.from)!.from).toBe(first.from);
  });

  it("does nothing while spell checking is off and never changes the text", async () => {
    const { view, check } = await mount(false);
    const before = view.state.doc.toJSON();
    expect(check).not.toHaveBeenCalled();
    expect(words(view)).toEqual([]);
    expect(view.state.doc.toJSON()).toEqual(before);
  });

  it("a failed check is reported once and leaves the page as it was", async () => {
    vi.useFakeTimers();
    const onError = vi.fn();
    const doc = docFromSnapshot(script);
    const plugin = createSpellPlugin({ check: () => Promise.reject(new Error("offline service")), active: () => true, knownNames: () => [], onError });
    view = new EditorView(document.body.appendChild(document.createElement("div")), { state: EditorState.create({ doc, plugins: [plugin] }) });
    await vi.advanceTimersByTimeAsync(700);
    expect(onError).toHaveBeenCalledTimes(1);
    expect(words(view)).toEqual([]);
  });
});
