// Typing budget for the writing comfort plugins (SCRIPT-018, SCRIPT-020; performance budget P3 in
// docs/engineering/21-performance-budgets.md). The suggestion list and the spelling underlines
// run on every keystroke, so their own share of the 16 ms frame must stay small on a long
// script. The reference size is the 6,000-element script used for P3. Timings run on the
// editor state only (no layout), and the bounds are generous so a slow test machine does not
// fail them; the structural checks prove the work does not grow with the script while typing.

import { describe, expect, it, vi } from "vitest";
import { EditorState, TextSelection, type Transaction } from "prosemirror-state";
import type { WritingSpellingCheck } from "../../../ipc/generated/WritingSpellingCheck";
import { docFromSnapshot, type Snapshot } from "./model";
import { schema } from "./schema";
import { createSuggestPlugin, suggestKey } from "./suggestPlugin";
import { createSpellPlugin, spellKey } from "./spelling";

/** About 6,000 elements: 300 scenes of 20 elements, with Latin, Telugu and Hindi names. */
function longScript(): Snapshot {
  const names = ["MEERA", "RAVI", "ANJALI", "సీత", "राहुल"];
  const snap: Snapshot = [];
  for (let s = 0; s < 300; s++) {
    const elements: Snapshot[number]["elements"] = [];
    for (let i = 0; i < 19; i++) {
      const id = `s${s}e${i}`;
      if (i % 3 === 0) elements.push({ id, type: "character", text: names[(s + i) % names.length] });
      else if (i % 3 === 1) elements.push({ id, type: "dialogue", text: "We should leave before the last bus goes, మీరా." });
      else elements.push({ id, type: "action", text: "The depot lights flicker. Rain on the tin roof. नमस्ते." });
    }
    snap.push({ id: `s${s}`, heading: `INT. LOCATION ${s % 40} - ${s % 2 ? "NIGHT" : "DAY"}`, elements });
  }
  return snap;
}

function setup() {
  let doc = docFromSnapshot(longScript());
  // The writer is typing a new character cue in the middle of the script (scene 150).
  let at = 0;
  let index = 0;
  doc.forEach((_n, pos, i) => {
    if (i === 3000) {
      at = pos;
      index = i;
    }
  });
  const cue = schema.nodes.character.create({ id: "typing" });
  doc = doc.copy(doc.content.replaceChild(index, cue));
  const check = vi.fn(async (): Promise<WritingSpellingCheck> => ({ enabled: true, results: [], missingLanguages: [] }));
  const plugins = [
    createSuggestPlugin({ sources: () => ({ characters: [], locations: [] }), canEdit: () => true, onCreate: () => {}, onChange: () => {} }),
    createSpellPlugin({ check, active: () => true, knownNames: () => [] }),
  ];
  const state = EditorState.create({ doc, plugins, selection: TextSelection.create(doc, at + 1) });
  return { state, check, elementCount: doc.childCount };
}

function type(state: EditorState, text: string): { state: EditorState; perKeyMs: number } {
  let s = state;
  const times: number[] = [];
  for (const ch of text) {
    const tr: Transaction = s.tr.insertText(ch);
    const t0 = performance.now();
    s = s.apply(tr);
    times.push(performance.now() - t0);
  }
  // The median, so one pause of a busy test machine does not decide the result.
  return { state: s, perKeyMs: times.sort((a, b) => a - b)[Math.floor(times.length / 2)] };
}

describe("typing budget on a 6,000-element script", () => {
  it("builds the reference script", () => {
    expect(setup().elementCount).toBeGreaterThanOrEqual(6000);
  });

  it("typing a character cue keeps the plugins' work per keystroke small", () => {
    const { state } = setup();
    type(state, "ME"); // warm up
    const { state: after, perKeyMs } = type(state, "MEERA ANJALI RAVI");
    expect(suggestKey.getState(after)?.ctx?.kind).toBe("character");
    // P3 allows 16 ms per keystroke for everything; these plugins get a small share of it.
    expect(perKeyMs).toBeLessThan(4);
  });

  it("Telugu and Hindi typing costs the same as Latin typing", () => {
    const { state } = setup();
    type(state, "సీ"); // warm up
    const { perKeyMs } = type(state, "సీత राहुल సీత");
    expect(perKeyMs).toBeLessThan(4);
  });

  it("typing in a block that was already checked does not copy the spelling record each keystroke", () => {
    const { state } = setup();
    // Mark every block as checked, as after the first idle check of a long script.
    let s = state;
    const results: { id: string; text: string; misses: never[] }[] = [];
    s.doc.forEach((n) => {
      const id = n.attrs.id as string | null;
      if (id) results.push({ id, text: n.textContent, misses: [] });
    });
    s = s.apply(s.tr.setMeta(spellKey, { results }));
    const first = type(s, "M").state;
    const checkedAfterFirst = spellKey.getState(first)!.checked;
    const second = type(first, "E").state;
    // The typed block left the record on the first keystroke; later keystrokes reuse it as is.
    expect(spellKey.getState(second)!.checked).toBe(checkedAfterFirst);
  });
});

describe("entering an element on a 6,000-element script", () => {
  it("reading the script values for a new element fits well inside one frame", () => {
    const { state } = setup();
    // Enter at the end of the cue makes a new element: the plugin reads the script once.
    let s = type(state, "MEERA").state;
    const times: number[] = [];
    for (let i = 0; i < 12; i++) {
      const { $head } = s.selection;
      const cue = schema.nodes.character.create({ id: `new${i}` });
      const tr = s.tr.insert($head.after(1), cue);
      const t0 = performance.now();
      s = s.apply(tr.setSelection(TextSelection.create(tr.doc, $head.after(1) + 1)));
      times.push(performance.now() - t0);
    }
    // The median, so one pause of a busy test machine does not decide the result.
    const perEnterMs = times.slice(2).sort((a, b) => a - b)[Math.floor((times.length - 2) / 2)];
    expect(suggestKey.getState(s)?.ctx?.kind).toBe("character");
    expect(perEnterMs).toBeLessThan(8);
  });
});

describe("moving the caret on a 6,000-element script", () => {
  it("moving between character cues reads the script once, not once per element", () => {
    const { state } = setup();
    const cues: number[] = [];
    state.doc.forEach((n, pos) => {
      if (n.type.name === "character" && n.textContent && cues.length < 200) cues.push(pos + n.nodeSize - 1);
    });
    let s = state.apply(state.tr.setSelection(TextSelection.create(state.doc, cues[0])));
    const t0 = performance.now();
    for (const pos of cues.slice(1)) s = s.apply(s.tr.setSelection(TextSelection.create(s.doc, pos)));
    const perMoveMs = (performance.now() - t0) / (cues.length - 1);
    // Every cue holds a full name, so no list opens, but each move still works out what is typed.
    expect(s.selection.head).toBe(cues[cues.length - 1]);
    // A full read of the script costs several milliseconds; a move must cost far less.
    expect(perMoveMs).toBeLessThan(1);
  });
});
