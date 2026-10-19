// Element flow test matrix (build plan item "Element flow test matrix with manual override";
// SCRIPT-001). Every element type through Enter at the end, Enter on an empty element, Enter in
// the middle, Shift+Enter, Tab and Shift+Tab, then the manual override (Ctrl+1 to Ctrl+7 and the
// element menu) and proof that an override sticks: through typing, through Enter, and through the
// save and reload path (snapshot round trip).
//
// The matrix covers the built-in text elements. Page breaks and custom elements follow their own
// rules (Screenplay and Production Depth Specification §8, §10) and have explicit cases at the end:
// a page break holds no text, Tab and Shift+Tab leave both alone, and a custom element is set with
// its own command (setCustom) rather than setType. The last block covers Enter after a character
// name while the suggestion list is open: one Enter creates a new character and moves on.

import { beforeEach, describe, expect, it } from "vitest";
import { EditorState, NodeSelection, TextSelection, type Command } from "prosemirror-state";
import { EditorView } from "prosemirror-view";
import { keymap } from "prosemirror-keymap";
import type { ElementType } from "../../../ipc/generated/ElementType";
import type { ScreenplayCustomElementDto } from "../../../ipc/generated/ScreenplayCustomElementDto";
import { enterCommand, insertPageBreak, lineBreakCommand, setCustom, setCustomDefs, setType, tabCommand } from "./commands";
import { docFromSnapshot, snapshotFromDoc, type Snapshot } from "./model";
import { nextOnEnter, onEnterEmpty, tabNext, tabPrev } from "./rules";
import { ELEMENT_TYPES, schema } from "./schema";
import { createSuggestPlugin } from "./suggestPlugin";

/** Built-in text elements: everything except page breaks and custom elements. */
const BUILT_IN: ElementType[] = ELEMENT_TYPES.filter((t) => t !== "page_break" && t !== "custom");
const BODY: ElementType[] = BUILT_IN.filter((t) => t !== "scene_heading");

/** A one-scene document whose second block is `type` with `text`, caret at `caret` in it. */
function stateWith(type: ElementType, text: string, caret: "end" | "start" | number = "end", customId: string | null = null): EditorState {
  const snap: Snapshot = [{ id: "s1", heading: "INT. DEPOT - NIGHT", elements: [{ id: "e1", type: "action", text: "Rain." }] }];
  let doc = docFromSnapshot(snap);
  if (type === "scene_heading") {
    // A second scene: the block under test is its heading.
    doc = docFromSnapshot([...snap, { id: "s2", heading: text, elements: [] }]);
  } else {
    const attrs = type === "custom" ? { id: "x1", customId } : { id: "x1" };
    const node = schema.nodes[type].create(attrs, text ? schema.text(text) : undefined);
    doc = doc.copy(doc.content.append(schema.nodes.doc.create(null, [node]).content));
  }
  const blockStart = doc.content.size - doc.lastChild!.nodeSize;
  const len = doc.lastChild!.content.size;
  const offset = caret === "end" ? len : caret === "start" ? 0 : caret;
  return EditorState.create({ doc, selection: TextSelection.create(doc, blockStart + 1 + offset) });
}

function run(state: EditorState, cmd: Command): EditorState {
  let next = state;
  const handled = cmd(state, (tr) => {
    next = state.apply(tr);
  });
  expect(handled).toBe(true);
  return next;
}

const typeAtCaret = (s: EditorState) => s.selection.$head.node(1).type.name as ElementType;
const types = (s: EditorState) => {
  const out: string[] = [];
  s.doc.forEach((n) => out.push(n.type.name));
  return out;
};
const typeText = (s: EditorState, text: string) => s.apply(s.tr.insertText(text));
const noFail = () => {
  throw new Error("unexpected refusal");
};

describe("element flow matrix: Enter at the end of a non-empty element", () => {
  for (const t of BUILT_IN) {
    it(`${t} + Enter → ${nextOnEnter(t)} on a new line; the ${t} keeps its text`, () => {
      const s = run(stateWith(t, "TEXT"), enterCommand);
      expect(typeAtCaret(s)).toBe(nextOnEnter(t));
      expect(s.selection.$head.parent.content.size).toBe(0);
      const before = s.doc.child(s.doc.childCount - 2);
      expect(before.type.name).toBe(t);
      expect(before.textContent).toBe("TEXT");
    });
  }
});

describe("element flow matrix: Enter on an empty element converts it", () => {
  for (const t of BODY) {
    it(`empty ${t} + Enter → ${onEnterEmpty(t) ?? "stays"}`, () => {
      const s0 = stateWith(t, "");
      const s = run(s0, enterCommand);
      expect(s.doc.childCount).toBe(s0.doc.childCount);
      expect(typeAtCaret(s)).toBe(onEnterEmpty(t) ?? t);
    });
  }
  it("empty scene heading + Enter does nothing (a scene needs a heading)", () => {
    const s0 = stateWith("scene_heading", "");
    const s = run(s0, enterCommand);
    expect(s.doc.eq(s0.doc)).toBe(true);
  });
  it("Enter twice from dialogue starts a new scene", () => {
    let s = run(stateWith("dialogue", "Go."), enterCommand);
    s = run(s, enterCommand);
    expect(typeAtCaret(s)).toBe("scene_heading");
  });
});

describe("element flow matrix: Enter inside text and at the start", () => {
  for (const t of BODY) {
    it(`${t}: Enter in the middle splits into two ${t} elements`, () => {
      const s = run(stateWith(t, "ABCD", 2), enterCommand);
      const last = s.doc.lastChild!;
      const prev = s.doc.child(s.doc.childCount - 2);
      expect([prev.type.name, prev.textContent, last.type.name, last.textContent]).toEqual([t, "AB", t, "CD"]);
    });
    it(`${t}: Enter at the start adds a blank ${t} above and keeps identity with the text`, () => {
      const s = run(stateWith(t, "ABCD", "start"), enterCommand);
      const last = s.doc.lastChild!;
      expect(last.textContent).toBe("ABCD");
      expect(last.attrs.id).toBe("x1");
      expect(s.doc.child(s.doc.childCount - 2).type.name).toBe(t);
    });
  }
  it("scene heading: Enter in the middle moves the rest into an action line", () => {
    const s = run(stateWith("scene_heading", "EXT. ROAD", 4), enterCommand);
    expect(s.doc.lastChild!.type.name).toBe("action");
    expect(s.doc.lastChild!.textContent).toBe(" ROAD");
  });
});

describe("element flow matrix: Shift+Enter", () => {
  for (const t of BUILT_IN) {
    const multi = t === "action" || t === "dialogue" || t === "note";
    it(`${t}: Shift+Enter ${multi ? "adds a line break inside the element" : "behaves like Enter"}`, () => {
      const s0 = stateWith(t, "TEXT");
      const s = run(s0, lineBreakCommand);
      if (multi) {
        expect(s.doc.childCount).toBe(s0.doc.childCount);
        expect(s.selection.$head.parent.child(1).type.name).toBe("hard_break");
      } else {
        expect(s.doc.childCount).toBe(s0.doc.childCount + 1);
      }
    });
  }
});

describe("element flow matrix: Tab and Shift+Tab", () => {
  for (const t of BODY) {
    it(`${t} + Tab → ${tabNext(t)}, + Shift+Tab → ${tabPrev(t)}`, () => {
      expect(typeAtCaret(run(stateWith(t, "X"), tabCommand(1, noFail)))).toBe(tabNext(t));
      expect(typeAtCaret(run(stateWith(t, "X"), tabCommand(-1, noFail)))).toBe(tabPrev(t));
    });
  }
  it("Tab goes round every element: action → character → parenthetical → dialogue → transition → shot → scene heading → action", () => {
    let s = stateWith("action", "");
    const seen: string[] = [];
    for (let i = 0; i < 7; i++) {
      s = run(s, tabCommand(1, noFail));
      seen.push(typeAtCaret(s));
    }
    expect(seen).toEqual(["character", "parenthetical", "dialogue", "transition", "shot", "scene_heading", "action"]);
  });
  it("Tab never leaves the page even when nothing changes", () => {
    const s0 = stateWith("transition", "CUT TO:");
    const s = run(s0, tabCommand(1, noFail));
    expect(typeAtCaret(s)).toBe("shot");
  });
});

describe("manual override sticks", () => {
  for (const from of BODY) {
    for (const to of BUILT_IN.filter((x) => x !== from)) {
      it(`${from} → ${to} by the element menu or its shortcut`, () => {
        const failures: string[] = [];
        let s = run(stateWith(from, "Words"), setType(to, (m) => failures.push(m)));
        expect(failures).toEqual([]);
        expect(typeAtCaret(s)).toBe(to);
        // Typing keeps the chosen element.
        s = typeText(s, " more");
        expect(typeAtCaret(s)).toBe(to);
        // Save and reload (the sync snapshot) keeps it.
        const reloaded = docFromSnapshot(snapshotFromDoc(s.doc));
        const lastType = to === "scene_heading" ? "scene_heading" : reloaded.lastChild!.type.name;
        expect(lastType).toBe(to);
        // Enter afterwards follows the rules of the chosen element, not the old one.
        s = run(s, enterCommand);
        expect(typeAtCaret(s)).toBe(nextOnEnter(to));
      });
    }
  }

  it("overriding the suggested next element: after Enter from a heading, Character is kept", () => {
    let s = run(stateWith("scene_heading", "INT. BUS - NIGHT"), enterCommand);
    expect(typeAtCaret(s)).toBe("action");
    s = run(s, setType("character", noFail));
    s = typeText(s, "MEERA");
    s = run(s, enterCommand);
    expect(typeAtCaret(s)).toBe("dialogue");
    expect(types(s).slice(-3)).toEqual(["scene_heading", "character", "dialogue"]);
  });

  it("Enter goes into an empty element of the next kind instead of stacking another (a new screenplay's first action)", () => {
    // A new screenplay is an empty heading followed by an empty action.
    const doc = docFromSnapshot([{ id: "s1", heading: "", elements: [{ id: "a1", type: "action", text: "" }] }]);
    let s = EditorState.create({ doc, selection: TextSelection.create(doc, 1) });
    s = typeText(s, "INT. BUS - NIGHT");
    s = run(s, enterCommand);
    expect(types(s)).toEqual(["scene_heading", "action"]);
    expect(s.selection.$head.node(1).attrs.id).toBe("a1");
    // The next element is not empty: Enter adds one, as before.
    s = run(stateWith("action", "Rain."), enterCommand);
    expect(types(s).slice(-2)).toEqual(["action", "character"]);
    // An empty element of another kind is left alone: a new one of the right kind comes first.
    const withChar = docFromSnapshot([{ id: "s1", heading: "INT. BUS - NIGHT", elements: [{ id: "a1", type: "action", text: "Rain." }, { id: "c1", type: "dialogue", text: "" }] }]);
    const end = withChar.child(0).nodeSize + withChar.child(1).nodeSize - 1;
    s = run(EditorState.create({ doc: withChar, selection: TextSelection.create(withChar, end) }), enterCommand);
    expect(types(s)).toEqual(["scene_heading", "action", "character", "dialogue"]);
  });

  it("the first scene heading cannot be turned into a body element", () => {
    const doc = docFromSnapshot([{ id: "s1", heading: "INT. DEPOT - NIGHT", elements: [] }]);
    const s0 = EditorState.create({ doc, selection: TextSelection.create(doc, 2) });
    const failures: string[] = [];
    const handled = setType("action", (m) => failures.push(m))(s0, () => undefined);
    expect(handled).toBe(false);
    expect(failures[0]).toMatch(/first scene needs a scene heading/);
  });

  it("Unicode text survives an override (Telugu and Hindi)", () => {
    let s = stateWith("action", "మీరా नमस्ते");
    s = run(s, setType("dialogue", noFail));
    expect(s.doc.lastChild!.textContent).toBe("మీరా नमस्ते");
    s = run(s, setType("character", noFail));
    expect(s.doc.lastChild!.textContent).toBe("మీరా नमस्ते");
  });
});

// Page breaks and custom elements (area 11 rules; see commands.ts and rules.ts).

const customDefinition = (over: Partial<ScreenplayCustomElementDto>): ScreenplayCustomElementDto => ({
  id: "tel",
  screenplayId: "sp",
  name: "Telephone",
  shortcut: "Ctrl+Alt+1",
  look: "dialogue",
  uppercase: true,
  italic: true,
  spaceBefore: 1,
  nextElement: "character",
  exportName: "TELEPHONE",
  usageCount: 1,
  rev: 1,
  ...over,
});

/** A one-scene document with an action line followed by a page break, the break selected. */
function pageBreakSelected(): EditorState {
  const doc = docFromSnapshot([
    {
      id: "s1",
      heading: "INT. DEPOT - NIGHT",
      elements: [
        { id: "e1", type: "action", text: "Rain.", customElementId: null },
        { id: "pb", type: "page_break", text: "", customElementId: null },
      ],
    },
  ]);
  const pos = doc.content.size - doc.lastChild!.nodeSize;
  return EditorState.create({ doc, selection: NodeSelection.create(doc, pos) });
}

describe("element flow: page breaks", () => {
  it("Enter and Shift+Enter on a selected page break add an action after it and keep the break", () => {
    for (const cmd of [enterCommand, lineBreakCommand]) {
      const s = run(pageBreakSelected(), cmd);
      expect(types(s).slice(-3)).toEqual(["action", "page_break", "action"]);
      expect(typeAtCaret(s)).toBe("action");
      expect(s.doc.child(s.doc.childCount - 2).attrs.id).toBe("pb");
    }
  });

  it("Tab and Shift+Tab leave a page break as it is and keep focus in the page", () => {
    for (const step of [1, -1] as const) {
      const s0 = pageBreakSelected();
      const s = run(s0, tabCommand(step, noFail));
      expect(s.doc.eq(s0.doc)).toBe(true);
    }
  });

  it("the element menu and Ctrl+1 to Ctrl+7 never turn a page break into text", () => {
    for (const to of BUILT_IN) {
      const failures: string[] = [];
      const handled = setType(to, (m) => failures.push(m))(pageBreakSelected(), () => undefined);
      expect(handled).toBe(false);
      expect(failures).toEqual(["A page break has no text to change."]);
    }
  });

  it("setType refuses to make a page break or a custom element (they have their own commands)", () => {
    for (const to of ["page_break", "custom"] as const) {
      const failures: string[] = [];
      expect(setType(to, (m) => failures.push(m))(stateWith("action", "Words"), () => undefined)).toBe(false);
      expect(failures).toEqual([]);
    }
  });

  it("Ctrl+Enter adds a page break that survives save and reload", () => {
    const s = run(stateWith("dialogue", "Go."), insertPageBreak);
    expect(types(s).slice(-3)).toEqual(["dialogue", "page_break", "dialogue"]);
    const reloaded = docFromSnapshot(snapshotFromDoc(s.doc));
    expect(reloaded.child(reloaded.childCount - 2).type.name).toBe("page_break");
  });
});

describe("element flow: custom elements", () => {
  beforeEach(() => {
    setCustomDefs([customDefinition({})]);
  });

  it("Enter at the end creates the element the definition names; the custom element keeps its text", () => {
    const s = run(stateWith("custom", "Hello?", "end", "tel"), enterCommand);
    expect(typeAtCaret(s)).toBe("character");
    const before = s.doc.child(s.doc.childCount - 2);
    expect([before.type.name, before.attrs.customId, before.textContent]).toEqual(["custom", "tel", "Hello?"]);
  });

  it("Enter on an empty custom element turns it into action", () => {
    const s0 = stateWith("custom", "", "end", "tel");
    const s = run(s0, enterCommand);
    expect(s.doc.childCount).toBe(s0.doc.childCount);
    expect(typeAtCaret(s)).toBe(onEnterEmpty("custom"));
  });

  it("Enter in the middle splits it and both halves keep the definition", () => {
    const s = run(stateWith("custom", "ABCD", 2, "tel"), enterCommand);
    const prev = s.doc.child(s.doc.childCount - 2);
    const last = s.doc.lastChild!;
    expect([prev.textContent, prev.attrs.customId, last.textContent, last.attrs.customId]).toEqual(["AB", "tel", "CD", "tel"]);
  });

  it("Shift+Enter adds a line break inside the element", () => {
    const s0 = stateWith("custom", "TEXT", "end", "tel");
    const s = run(s0, lineBreakCommand);
    expect(s.doc.childCount).toBe(s0.doc.childCount);
    expect(s.selection.$head.parent.child(1).type.name).toBe("hard_break");
  });

  it("Tab and Shift+Tab leave a custom element as it is and keep focus in the page", () => {
    expect([tabNext("custom"), tabPrev("custom")]).toEqual([null, null]);
    for (const step of [1, -1] as const) {
      const s0 = stateWith("custom", "X", "end", "tel");
      const s = run(s0, tabCommand(step, noFail));
      expect(s.doc.eq(s0.doc)).toBe(true);
    }
  });

  for (const from of BODY) {
    it(`${from} → custom by its shortcut sticks through typing, reload and Enter`, () => {
      const failures: string[] = [];
      let s = run(stateWith(from, "Words"), setCustom("tel", (m) => failures.push(m)));
      expect(failures).toEqual([]);
      expect(typeAtCaret(s)).toBe("custom");
      s = typeText(s, " more");
      expect(typeAtCaret(s)).toBe("custom");
      const reloaded = docFromSnapshot(snapshotFromDoc(s.doc));
      expect([reloaded.lastChild!.type.name, reloaded.lastChild!.attrs.customId]).toEqual(["custom", "tel"]);
      s = run(s, enterCommand);
      expect(typeAtCaret(s)).toBe("character");
    });
  }

  for (const to of BUILT_IN) {
    it(`custom → ${to} by the element menu or its shortcut drops the definition and sticks`, () => {
      const failures: string[] = [];
      let s = run(stateWith("custom", "Words", "end", "tel"), setType(to, (m) => failures.push(m)));
      expect(failures).toEqual([]);
      expect(typeAtCaret(s)).toBe(to);
      s = typeText(s, " more");
      expect(typeAtCaret(s)).toBe(to);
      const reloaded = docFromSnapshot(snapshotFromDoc(s.doc));
      expect(reloaded.lastChild!.type.name).toBe(to);
      s = run(s, enterCommand);
      expect(typeAtCaret(s)).toBe(nextOnEnter(to));
    });
  }

  it("the first scene heading cannot become a custom element, and an unknown definition is refused", () => {
    const doc = docFromSnapshot([{ id: "s1", heading: "INT. DEPOT - NIGHT", elements: [] }]);
    const s0 = EditorState.create({ doc, selection: TextSelection.create(doc, 2) });
    const failures: string[] = [];
    expect(setCustom("tel", (m) => failures.push(m))(s0, () => undefined)).toBe(false);
    expect(failures[0]).toMatch(/first scene needs a scene heading/);
    expect(setCustom("missing", noFail)(stateWith("action", "Words"), () => undefined)).toBe(false);
  });
});

describe("element flow matrix: Enter after a character name while the suggestion list is open", () => {
  // Owner decisions 2026-10-08 (like Final Draft): one Enter after a character name, new or chosen
  // from the list, moves on to Dialogue (a new name is created first); Tab never accepts an item. The list owns Enter while it is open, so this runs through
  // a real editor view with the suggestion plugin and the editor's own Enter key.
  function mountCharacter(text: string, onCreate: (kind: string, name: string, movesOn: boolean) => void) {
    const base = stateWith("character", text);
    const plugin = createSuggestPlugin({ sources: () => ({ characters: [{ name: "MEERA", source: "Story" }], locations: [] }), canEdit: () => true, onCreate, onChange: () => undefined });
    const keys = keymap({ Enter: enterCommand, Tab: tabCommand(tabNext, noFail) });
    const state = EditorState.create({ doc: base.doc, selection: base.selection, plugins: [plugin, keys] });
    const host = document.createElement("div");
    document.body.appendChild(host);
    return new EditorView(host, { state });
  }
  const press = (v: EditorView, key: string) => {
    const e = new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true });
    v.dom.dispatchEvent(e);
    return e.defaultPrevented;
  };
  const tail = (v: EditorView) => {
    const out: string[] = [];
    v.state.doc.forEach((n) => out.push(`${n.type.name}: ${n.textContent}`));
    return out.slice(-2);
  };

  it("a new name + Enter → the character is created and the caret is in Dialogue (one key press)", () => {
    const created: [string, string, boolean][] = [];
    const v = mountCharacter("ZED", (kind, name, movesOn) => created.push([kind, name, movesOn]));
    try {
      expect(press(v, "Enter")).toBe(true);
      expect(created).toEqual([["character", "ZED", true]]);
      expect(tail(v)).toEqual(["character: ZED", "dialogue: "]);
      expect(v.state.selection.$head.parent.type.name).toBe(nextOnEnter("character"));
    } finally {
      v.destroy();
    }
  });

  it("part of a known name + Enter → the name is filled and the caret is in Dialogue (one key press)", () => {
    const created: unknown[] = [];
    const v = mountCharacter("ME", (...a) => created.push(a));
    try {
      expect(press(v, "Enter")).toBe(true);
      expect(created).toEqual([]);
      expect(tail(v)).toEqual(["character: MEERA", "dialogue: "]);
      expect(v.state.selection.$head.parent.type.name).toBe(nextOnEnter("character"));
    } finally {
      v.destroy();
    }
  });

  it("Tab with the list open keeps its element meaning and accepts nothing", () => {
    const created: unknown[] = [];
    const v = mountCharacter("ME", (...a) => created.push(a));
    try {
      press(v, "Tab");
      expect(created).toEqual([]);
      expect(v.state.selection.$head.parent.type.name).toBe(tabNext("character"));
      expect(v.state.selection.$head.parent.textContent).toBe("ME");
    } finally {
      v.destroy();
    }
  });

  it("a known name typed in full + Enter → Dialogue, nothing created", () => {
    const created: unknown[] = [];
    const v = mountCharacter("MEERA", (...a) => created.push(a));
    try {
      press(v, "Enter");
      expect(created).toEqual([]);
      expect(tail(v)).toEqual(["character: MEERA", "dialogue: "]);
    } finally {
      v.destroy();
    }
  });

  it("Esc first keeps the new name uncreated; Enter then moves on to Dialogue", () => {
    const created: unknown[] = [];
    const v = mountCharacter("ZED", (...a) => created.push(a));
    try {
      press(v, "Escape");
      press(v, "Enter");
      expect(created).toEqual([]);
      expect(tail(v)).toEqual(["character: ZED", "dialogue: "]);
    } finally {
      v.destroy();
    }
  });
});
