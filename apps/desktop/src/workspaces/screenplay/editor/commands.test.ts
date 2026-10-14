// Keyboard and element commands for page breaks and custom elements
// (Screenplay and Production Depth Specification §8, §10; contract doc 32 §2.1).

import { describe, expect, it, beforeEach } from "vitest";
import { EditorState, NodeSelection, TextSelection, type Command } from "prosemirror-state";
import type { ScreenplayCustomElementDto } from "../../../ipc/generated/ScreenplayCustomElementDto";
import { docFromSnapshot, snapshotFromDoc, type Snapshot } from "./model";
import { enterCommand, insertPageBreak, lineBreakCommand, runShortcut, setCustom, setCustomDefs, setType, tabCommand } from "./commands";
import { useShortcuts } from "../../../app/shortcuts";
import { tabNext, tabPrev, onEnterEmpty } from "./rules";

const def = (over: Partial<ScreenplayCustomElementDto>): ScreenplayCustomElementDto => ({
  id: "tel",
  screenplayId: "sp",
  name: "Telephone",
  shortcut: "Ctrl+Alt+1",
  look: "dialogue",
  uppercase: true,
  italic: true,
  spaceBefore: 1,
  nextElement: "action",
  exportName: "TELEPHONE",
  usageCount: 1,
  rev: 1,
  ...over,
});

const script: Snapshot = [
  {
    id: "s1",
    heading: "INT. SIGNAL BOX - NIGHT",
    elements: [
      { id: "e1", type: "custom", text: "Hello?", customElementId: "tel" },
      { id: "e2", type: "page_break", text: "", customElementId: null },
      { id: "e3", type: "action", text: "Silence.", customElementId: null },
    ],
  },
];

/** Start of the text of the block at `index`. */
function textStart(state: EditorState, index: number): number {
  let pos = 0;
  for (let i = 0; i < index; i++) pos += state.doc.child(i).nodeSize;
  return pos + 1;
}

function at(index: number, offset: "start" | "end" | number, snap: Snapshot = script): EditorState {
  const doc = docFromSnapshot(snap);
  const state = EditorState.create({ doc });
  const node = doc.child(index);
  if (node.type.name === "page_break") {
    let pos = 0;
    for (let i = 0; i < index; i++) pos += doc.child(i).nodeSize;
    return state.apply(state.tr.setSelection(NodeSelection.create(doc, pos)));
  }
  const start = textStart(state, index);
  const off = offset === "start" ? 0 : offset === "end" ? node.content.size : offset;
  return state.apply(state.tr.setSelection(TextSelection.create(doc, start + off)));
}

function run(state: EditorState, cmd: Command): { ok: boolean; state: EditorState } {
  let next = state;
  const ok = cmd(state, (tr) => {
    next = state.apply(tr);
  });
  return { ok, state: next };
}

const types = (s: EditorState) => {
  const out: string[] = [];
  s.doc.forEach((n) => out.push(n.type.name));
  return out;
};

const failures: string[] = [];
const fail = (m: string) => failures.push(m);

beforeEach(() => {
  failures.length = 0;
  setCustomDefs([def({}), def({ id: "radio", name: "Radio", shortcut: null, look: "centered", nextElement: "custom:tel", exportName: "RADIO" })]);
});

describe("rules for page breaks and custom elements", () => {
  it("Tab and Shift+Tab leave them alone, Enter in an empty one never deletes a break", () => {
    expect(tabNext("page_break")).toBeNull();
    expect(tabPrev("page_break")).toBeNull();
    expect(tabNext("custom")).toBeNull();
    expect(tabPrev("custom")).toBeNull();
    expect(onEnterEmpty("page_break")).toBeNull();
    expect(onEnterEmpty("custom")).toBe("action");
  });
});

describe("Tab", () => {
  it("does not change a custom element and still keeps focus in the page", () => {
    const r = run(at(1, "end"), tabCommand(tabNext, fail));
    expect(r.ok).toBe(true);
    expect(types(r.state)).toEqual(["scene_heading", "custom", "page_break", "action"]);
    expect(r.state.doc.child(1).attrs.customId).toBe("tel");
  });

  it("does not turn a selected page break into text", () => {
    const r = run(at(2, "start"), tabCommand(tabNext, fail));
    expect(r.ok).toBe(true);
    expect(types(r.state)).toEqual(["scene_heading", "custom", "page_break", "action"]);
  });

  it("still cycles built-in elements", () => {
    const r = run(at(3, "end"), tabCommand(tabNext, fail));
    expect(types(r.state)[3]).toBe("character");
  });
});

describe("Enter", () => {
  it("at the end of a custom element creates its own next element", () => {
    const r = run(at(1, "end"), enterCommand);
    expect(types(r.state)).toEqual(["scene_heading", "custom", "action", "page_break", "action"]);
  });

  it("after a custom element whose next element is a custom element creates that custom element", () => {
    const snap: Snapshot = [{ ...script[0], elements: [{ id: "r1", type: "custom", text: "Over.", customElementId: "radio" }] }];
    const r = run(at(1, "end", snap), enterCommand);
    expect(types(r.state)).toEqual(["scene_heading", "custom", "custom"]);
    expect(r.state.doc.child(2).attrs.customId).toBe("tel");
  });

  it("in the middle of a custom element splits it and both halves keep the definition", () => {
    const r = run(at(1, 3), enterCommand);
    expect(types(r.state)).toEqual(["scene_heading", "custom", "custom", "page_break", "action"]);
    expect(r.state.doc.child(1).attrs.customId).toBe("tel");
    expect(r.state.doc.child(2).attrs.customId).toBe("tel");
    expect(r.state.doc.child(1).attrs.id).toBe("e1");
    expect(r.state.doc.child(2).attrs.id).not.toBe("e1");
  });

  it("on a selected page break adds an action after it and keeps the break", () => {
    const r = run(at(2, "start"), enterCommand);
    expect(types(r.state)).toEqual(["scene_heading", "custom", "page_break", "action", "action"]);
    expect(r.state.selection.$from.parent.type.name).toBe("action");
  });

  it("Shift+Enter is a line break inside a custom element", () => {
    const r = run(at(1, "end"), lineBreakCommand);
    expect(types(r.state)).toEqual(["scene_heading", "custom", "page_break", "action"]);
    expect(snapshotFromDoc(r.state.doc)[0].elements[0].text).toBe("Hello?\n");
  });
});

describe("element changes", () => {
  it("Ctrl+1 to Ctrl+7 never turn a page break into text", () => {
    const r = run(at(2, "start"), setType("action", fail));
    expect(r.ok).toBe(false);
    expect(failures).toEqual(["A page break has no text to change."]);
  });

  it("changing a range that holds a page break keeps the break", () => {
    let s = at(1, "start");
    s = s.apply(s.tr.setSelection(TextSelection.create(s.doc, textStart(s, 1), textStart(s, 3) + 2)));
    const r = run(s, setType("action", fail));
    expect(r.ok).toBe(true);
    expect(types(r.state)).toEqual(["scene_heading", "action", "page_break", "action"]);
  });

  it("setType refuses the special types that have their own commands", () => {
    expect(run(at(3, "end"), setType("page_break", fail)).ok).toBe(false);
    expect(run(at(3, "end"), setType("custom", fail)).ok).toBe(false);
  });

  it("a custom element shortcut changes an action and refuses a page break or unknown definition", () => {
    const r = run(at(3, "end"), setCustom("tel", fail));
    expect(r.ok).toBe(true);
    expect(r.state.doc.child(3).type.name).toBe("custom");
    expect(r.state.doc.child(3).attrs.id).toBe("e3");
    expect(run(at(2, "start"), setCustom("tel", fail)).ok).toBe(false);
    expect(run(at(3, "end"), setCustom("missing", fail)).ok).toBe(false);
  });

  it("changing a custom element to a built-in drops the definition", () => {
    const r = run(at(1, "end"), setType("character", fail));
    const el = snapshotFromDoc(r.state.doc)[0].elements[0];
    expect(el.type).toBe("character");
    expect(el.customElementId ?? null).toBeNull();
  });
});

describe("Ctrl+Enter", () => {
  it("inserts a page break and continues with the same kind of element", () => {
    const r = run(at(1, "end"), insertPageBreak);
    expect(types(r.state)).toEqual(["scene_heading", "custom", "page_break", "custom", "page_break", "action"]);
    expect(r.state.doc.child(3).attrs.customId).toBe("tel");
    expect(r.state.selection.$from.parent.type.name).toBe("custom");
  });

  it("after a heading continues with action", () => {
    const r = run(at(0, "end"), insertPageBreak);
    expect(types(r.state).slice(0, 3)).toEqual(["scene_heading", "page_break", "action"]);
  });
});

describe("configurable page break shortcut", () => {
  const press = (init: KeyboardEventInit) => new KeyboardEvent("keydown", init);
  const ctrlEnter = { key: "Enter", code: "Enter", ctrlKey: true };
  const ctrlShiftEnter = { key: "Enter", code: "Enter", ctrlKey: true, shiftKey: true };

  it("inserts a page break with the default Ctrl+Enter", () => {
    useShortcuts.getState().set({});
    const r = run(at(3, "end"), (st, d) => runShortcut(press(ctrlEnter), [["screenplay.page_break", insertPageBreak]], st, d));
    expect(r.ok).toBe(true);
    expect(types(r.state).filter((t) => t === "page_break")).toHaveLength(2);
  });

  it("follows a remapped key and no longer answers the old one", () => {
    useShortcuts.getState().set({ "screenplay.page_break": ["Mod+Shift+Enter"] });
    const cmds: [string, Command][] = [["screenplay.page_break", insertPageBreak]];
    const old = run(at(3, "end"), (st, d) => runShortcut(press(ctrlEnter), cmds, st, d));
    expect(old.ok).toBe(false);
    expect(types(old.state).filter((t) => t === "page_break")).toHaveLength(1);
    const now = run(at(3, "end"), (st, d) => runShortcut(press(ctrlShiftEnter), cmds, st, d));
    expect(now.ok).toBe(true);
    expect(types(now.state).filter((t) => t === "page_break")).toHaveLength(2);
    useShortcuts.getState().set({});
  });
});
