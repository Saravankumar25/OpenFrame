// Smart suggestions while typing (SCRIPT-018): the deterministic list and the keyboard rules.

import { afterEach, describe, expect, it, vi } from "vitest";
import { EditorState, TextSelection } from "prosemirror-state";
import { EditorView } from "prosemirror-view";
import type { WritingSuggestionSources } from "../../../ipc/generated/WritingSuggestionSources";
import { docFromSnapshot, type Snapshot } from "./model";
import { schema } from "./schema";
import {
  HEADING_PREFIXES,
  TIMES,
  contextAt,
  cueName,
  headingParts,
  scriptValues,
  suggestions,
  type SuggestContext,
  type SuggestItem,
} from "./suggest";
import { acceptSuggestion, createSuggestPlugin, suggestKey } from "./suggestPlugin";

const script: Snapshot = [
  {
    id: "s1",
    heading: "INT. BUS DEPOT - NIGHT",
    elements: [
      { id: "e1", type: "character", text: "MEERA" },
      { id: "e2", type: "parenthetical", text: "(whispering)" },
      { id: "e3", type: "dialogue", text: "Is it late?" },
      { id: "e4", type: "character", text: "RAVI (V.O.)" },
      { id: "e5", type: "character", text: "Meera" },
    ],
  },
  { id: "s2", heading: "EXT. ROAD - MOMENTS LATER", elements: [] },
];
const project: WritingSuggestionSources = {
  characters: [
    { name: "ANJALI", source: "Story" },
    { name: "MEERA", source: "Story" },
    { name: "CONDUCTOR", source: "Cast" },
  ],
  locations: [{ name: "OLD RAILWAY STATION", source: "Production" }],
};
const values = () => scriptValues(docFromSnapshot(script));
const labels = (items: { label: string }[]) => items.map((i) => i.label);

describe("what the writer is typing", () => {
  it("knows the part of a scene heading: prefix, location, time", () => {
    expect(contextAt("scene_heading", "", 0)).toEqual({ kind: "prefix", from: 0, to: 0, query: "" });
    expect(contextAt("scene_heading", "IN", 2)).toMatchObject({ kind: "prefix", query: "IN" });
    expect(contextAt("scene_heading", "INT. BU", 7)).toEqual({ kind: "location", from: 5, to: 7, query: "BU" });
    expect(contextAt("scene_heading", "INT./EXT. CAR - NI", 18)).toEqual({ kind: "time", from: 16, to: 18, query: "NI" });
    expect(contextAt("scene_heading", "I/E CAR \u2013 D", 11)).toMatchObject({ kind: "time", query: "D" });
  });
  it("offers nothing unless the caret is at the end, and never in action or dialogue", () => {
    expect(contextAt("scene_heading", "INT. BUS", 3)).toBeNull();
    expect(contextAt("action", "Meera", 5)).toBeNull();
    expect(contextAt("dialogue", "Hi", 2)).toBeNull();
  });
  it("character names, extensions, parentheticals and transitions", () => {
    expect(contextAt("character", "ME", 2)).toEqual({ kind: "character", from: 0, to: 2, query: "ME" });
    expect(contextAt("character", "MEERA (V", 8)).toEqual({ kind: "extension", from: 7, to: 8, query: "V" });
    expect(contextAt("character", "MEERA (V.O.)", 12)).toBeNull();
    expect(contextAt("parenthetical", "(wh", 3)).toEqual({ kind: "parenthetical", from: 1, to: 3, query: "wh" });
    expect(contextAt("transition", "CU", 2)).toMatchObject({ kind: "transition" });
  });
  it("reads cue names and heading parts like Rust does", () => {
    expect(cueName("ravi (V.O.) (CONT'D)")).toBe("RAVI");
    expect(headingParts("int. old  depot - night")).toEqual({ prefix: "INT.", location: "OLD DEPOT", time: "NIGHT" });
    expect(headingParts("EXT. రైల్వే స్టేషన్ - DAY").location).toBe("రైల్వే స్టేషన్");
  });
});

describe("the list", () => {
  it("collects script values, most used first, without duplicates", () => {
    const v = values();
    expect(v.characters).toEqual(["MEERA", "RAVI"]);
    expect(v.locations).toEqual(["BUS DEPOT", "ROAD"]);
    expect(v.times).toEqual(["NIGHT", "MOMENTS LATER"]);
    expect(v.parentheticals).toEqual(["whispering"]);
  });
  it("prefixes and times include the standard terms", () => {
    expect(labels(suggestions({ kind: "prefix", from: 0, to: 0, query: "" }, values(), null, true))).toEqual(HEADING_PREFIXES);
    const times = labels(suggestions({ kind: "time", from: 0, to: 0, query: "" }, values(), null, true));
    for (const t of ["DAY", "NIGHT", "LATER", "CONTINUOUS", "SAME", "MOMENTS LATER"]) expect(times).toContain(t);
    expect(TIMES).toContain("CONTINUOUS");
  });
  it("typing filters; word starts count; the exact typed value is not offered again", () => {
    const ctx = { kind: "time" as const, from: 0, to: 2, query: "la" };
    expect(labels(suggestions(ctx, values(), null, true))).toEqual(["LATER", "MOMENTS LATER"]);
    expect(labels(suggestions({ kind: "time", from: 0, to: 5, query: "night" }, values(), null, true))).toEqual([]);
  });
  it("characters come from the script, the Beat Board and cast, with their source", () => {
    const items = suggestions({ kind: "character", from: 0, to: 0, query: "" }, values(), project, true);
    expect(items.map((i) => [i.label, i.source])).toEqual([
      ["MEERA", "Script"],
      ["RAVI", "Script"],
      ["ANJALI", "Beat Board"],
      ["CONDUCTOR", "Cast"],
    ]);
  });
  it("offers Create New for a new character or location, never for a known one", () => {
    const fresh = suggestions({ kind: "character", from: 0, to: 3, query: "Zar" }, values(), project, true);
    expect(fresh.at(-1)).toMatchObject({ label: "Create New: ZAR", create: "character", insert: "Zar" });
    const known = suggestions({ kind: "character", from: 0, to: 5, query: "meera" }, values(), project, true);
    expect(known.some((i) => i.create)).toBe(false);
    const loc = suggestions({ kind: "location", from: 5, to: 9, query: "ROOF" }, values(), project, true);
    expect(loc.at(-1)).toMatchObject({ create: "location" });
    expect(suggestions({ kind: "location", from: 5, to: 9, query: "ROOF" }, values(), project, false).some((i) => i.create)).toBe(false);
  });
  // What suggestPlugin does on accept (`tr.insertText(item.insert, from, to)`): only [from, to) changes.
  const accept = (text: string, ctx: SuggestContext, item: SuggestItem) => text.slice(0, ctx.from) + item.insert + text.slice(ctx.to);
  it("accepting replaces only the typed part", () => {
    const ctx = contextAt("scene_heading", "INT. BU", 7)!;
    const item = suggestions(ctx, values(), project, true)[0];
    expect(item.label).toBe("BUS DEPOT");
    expect(accept("INT. BU", ctx, item)).toBe("INT. BUS DEPOT - ");
    const p = contextAt("parenthetical", "wh", 2)!;
    expect(accept("wh", p, suggestions(p, values(), null, true)[0])).toBe("(whispering)");
    const create = suggestions({ kind: "character", from: 0, to: 3, query: "Zar" }, values(), project, true).at(-1)!;
    expect(accept("Zar", { kind: "character", from: 0, to: 3, query: "Zar" }, create)).toBe("Zar");
  });
});

describe("keyboard and mouse in the editor", () => {
  let view: EditorView | null = null;
  afterEach(() => {
    view?.destroy();
    view = null;
  });

  function mount(type: "scene_heading" | "character", text: string, onCreate = vi.fn(), beforeMoveOn?: () => void) {
    let doc = docFromSnapshot(script);
    const node = schema.nodes[type].create({ id: "new" }, text ? schema.text(text) : undefined);
    doc = doc.copy(doc.content.append(schema.nodes.doc.create(null, [node]).content));
    const changes: unknown[] = [];
    const plugin = createSuggestPlugin({ sources: () => project, canEdit: () => true, onCreate, beforeMoveOn, onChange: (s) => changes.push(s) });
    const state = EditorState.create({ doc, plugins: [plugin], selection: TextSelection.create(doc, doc.content.size - 1) });
    const host = document.createElement("div");
    document.body.appendChild(host);
    view = new EditorView(host, { state });
    return { view, changes };
  }
  const key = (v: EditorView, k: string, extra: KeyboardEventInit = {}) => {
    const e = new KeyboardEvent("keydown", { key: k, ...extra });
    return v.someProp("handleKeyDown", (f) => f(v, e)) ?? false;
  };
  const lastText = (v: EditorView) => v.state.doc.lastChild!.textContent;
  const st = (v: EditorView) => suggestKey.getState(v.state)!;

  it("typing filters, arrows move, Enter accepts the highlighted name and moves on to Dialogue", () => {
    const { view } = mount("character", "M");
    expect(st(view).items[0].label).toBe("MEERA");
    expect(st(view).index).toBe(0);
    expect(key(view, "ArrowDown")).toBe(true);
    expect(key(view, "ArrowUp")).toBe(true);
    expect(st(view).index).toBe(0);
    expect(key(view, "Enter")).toBe(true);
    const out: string[] = [];
    view.state.doc.forEach((n) => out.push(`${n.type.name}: ${n.textContent}`));
    expect(out.slice(-2)).toEqual(["character: MEERA", "dialogue: "]);
    // The list is gone in the Dialogue line: the next Enter is the editor's own.
    expect(st(view).ctx).toBeNull();
    expect(key(view, "Enter")).toBe(false);
  });

  it("an empty element shows the list with nothing chosen, so Enter keeps its meaning", () => {
    const { view } = mount("character", "");
    expect(st(view).items.length).toBeGreaterThan(0);
    expect(st(view).index).toBe(-1);
    expect(key(view, "Enter")).toBe(false);
    expect(lastText(view)).toBe("");
  });

  it("Esc dismisses without changing the text; typing brings the list back", () => {
    const { view } = mount("scene_heading", "IN");
    expect(key(view, "Escape")).toBe(true);
    expect(st(view).dismissed).toBe(true);
    expect(lastText(view)).toBe("IN");
    expect(key(view, "Enter")).toBe(false);
    view.dispatch(view.state.tr.insertText("T"));
    expect(st(view).dismissed).toBe(false);
  });

  it("a heading is built prefix, location, time with three accepts", () => {
    const { view } = mount("scene_heading", "IN");
    key(view, "Enter");
    expect(lastText(view)).toBe("INT. ");
    // Nothing is highlighted before typing, so arrows still move the caret.
    expect(key(view, "ArrowDown")).toBe(false);
    view.dispatch(view.state.tr.insertText("B"));
    key(view, "Enter");
    expect(lastText(view)).toBe("INT. BUS DEPOT - ");
    view.dispatch(view.state.tr.insertText("CON"));
    key(view, "Enter");
    expect(lastText(view)).toBe("INT. BUS DEPOT - CONTINUOUS");
  });

  const blocks = (v: EditorView) => {
    const out: string[] = [];
    v.state.doc.forEach((n) => out.push(`${n.type.name}: ${n.textContent}`));
    return out;
  };
  const caretType = (v: EditorView) => v.state.selection.$head.parent.type.name;

  it("Create New keeps the typed text, asks to add the name and moves on to Dialogue", () => {
    const onCreate = vi.fn();
    const { view } = mount("character", "Zara", onCreate);
    const i = st(view).items.findIndex((x) => x.create);
    for (let k = 0; k < i; k++) key(view, "ArrowDown");
    expect(key(view, "Enter")).toBe(true);
    expect(onCreate).toHaveBeenCalledWith("character", "ZARA", true);
    expect(blocks(view).slice(-2)).toEqual(["character: Zara", "dialogue: "]);
    expect(caretType(view)).toBe("dialogue");
  });

  it("one Enter after a new character name creates it and moves on to Dialogue (like Final Draft)", () => {
    // The name is recorded before the Dialogue line exists, so the editor can save the typed name,
    // then the character, then the Dialogue step, and Undo takes them back one at a time.
    let blocksAtCreate: string[] = [];
    const onCreate = vi.fn(() => {
      blocksAtCreate = blocks(view!);
    });
    const { view: v } = mount("character", "Quill", onCreate);
    expect(st(v).items.map((x) => x.label)).toEqual(["Create New: QUILL"]);
    expect(st(v).index).toBe(0);
    expect(key(v, "Enter")).toBe(true);
    expect(onCreate).toHaveBeenCalledTimes(1);
    expect(onCreate).toHaveBeenCalledWith("character", "QUILL", true);
    expect(blocksAtCreate.at(-1)).toBe("character: Quill");
    expect(blocks(v).slice(-2)).toEqual(["character: Quill", "dialogue: "]);
    expect(caretType(v)).toBe("dialogue");
    // The list is gone in the Dialogue line: the next Enter is the editor's own.
    expect(st(v).ctx).toBeNull();
    expect(key(v, "Enter")).toBe(false);
  });

  it("one Enter on a known name fills it and moves on to Dialogue; the name is saved first", () => {
    // Owner decision 2026-10-08: same as a new name. beforeMoveOn lets the editor save the filled
    // name before the Dialogue line exists, so Undo takes back the Dialogue step first.
    const onCreate = vi.fn();
    let blocksBeforeMove: string[] = [];
    const { view: v } = mount("character", "M", onCreate, () => {
      blocksBeforeMove = blocks(view!);
    });
    expect(st(v).items.map((x) => x.label)).toEqual(["MEERA", "Create New: M"]);
    key(v, "ArrowDown");
    key(v, "ArrowUp");
    expect(key(v, "Enter")).toBe(true);
    expect(onCreate).not.toHaveBeenCalled();
    expect(blocksBeforeMove.at(-1)).toBe("character: MEERA");
    expect(blocks(v).slice(-2)).toEqual(["character: MEERA", "dialogue: "]);
    expect(caretType(v)).toBe("dialogue");
  });

  it("a known name with an extension still fills only the bracket", () => {
    const { view } = mount("character", "MEERA (V");
    expect(key(view, "Enter")).toBe(true);
    expect(blocks(view).at(-1)).toBe("character: MEERA (V.O.)");
    expect(caretType(view)).toBe("character");
  });

  it("a click on a known name fills it and leaves the caret in the character line", () => {
    const { view } = mount("character", "M");
    expect(acceptSuggestion(view, 0, { onCreate: vi.fn() })).toBe(true);
    expect(blocks(view).at(-1)).toBe("character: MEERA");
    expect(caretType(view)).toBe("character");
  });

  it("a scene heading accept never moves on (prefix, location, time stay in the heading)", () => {
    const { view } = mount("scene_heading", "IN");
    key(view, "Enter");
    expect(caretType(view)).toBe("scene_heading");
  });

  it("Esc keeps a new name without creating it; the next Enter moves on as usual", () => {
    const onCreate = vi.fn();
    const { view } = mount("character", "Quill", onCreate);
    expect(key(view, "Escape")).toBe(true);
    expect(key(view, "Enter")).toBe(false);
    expect(onCreate).not.toHaveBeenCalled();
    expect(lastText(view)).toBe("Quill");
  });

  it("a click on Create New adds the name and leaves the caret where it is", () => {
    const onCreate = vi.fn();
    const { view } = mount("character", "Quill", onCreate);
    expect(acceptSuggestion(view, 0, { onCreate })).toBe(true);
    expect(onCreate).toHaveBeenCalledWith("character", "QUILL", false);
    expect(blocks(view).at(-1)).toBe("character: Quill");
    expect(st(view).dismissed).toBe(true);
  });

  it("Enter on Create New for a location adds it and stays in the heading for the time of day", () => {
    const onCreate = vi.fn();
    const { view } = mount("scene_heading", "INT. HARBOUR", onCreate);
    const i = st(view).items.findIndex((x) => x.create === "location");
    for (let k = 0; k < i; k++) key(view, "ArrowDown");
    expect(key(view, "Enter")).toBe(true);
    expect(onCreate).toHaveBeenCalledWith("location", "HARBOUR", false);
    expect(blocks(view).at(-1)).toBe("scene_heading: INT. HARBOUR");
    expect(caretType(view)).toBe("scene_heading");
  });

  it("modifier keys and Tab pass through to the editor", () => {
    const { view } = mount("character", "M");
    expect(key(view, "Enter", { ctrlKey: true })).toBe(false);
    expect(key(view, "Tab")).toBe(false);
  });

  it("an input method composing text (Telugu, Hindi) keeps its Enter", () => {
    const { view } = mount("character", "M");
    expect(key(view, "Enter", { isComposing: true })).toBe(false);
    expect(key(view, "Enter", { keyCode: 229 })).toBe(false);
    expect(lastText(view)).toBe("M");
  });

  it("offers Telugu names from the script by their first letters", () => {
    const tel: Snapshot = [{ id: "t1", heading: "INT. ఇల్లు - DAY", elements: [{ id: "c1", type: "character", text: "\u0C2E\u0C40\u0C30\u0C3E" }] }];
    const items = suggestions({ kind: "character", from: 0, to: 2, query: "\u0C2E\u0C40" }, scriptValues(docFromSnapshot(tel)), null, true);
    expect(items[0].label).toBe("\u0C2E\u0C40\u0C30\u0C3E");
  });

  it("read-only drafts never show the list", () => {
    let doc = docFromSnapshot(script);
    doc = doc.copy(doc.content.append(schema.nodes.doc.create(null, [schema.nodes.character.create({ id: "n" }, schema.text("M"))]).content));
    const plugin = createSuggestPlugin({ sources: () => project, canEdit: () => false, onCreate: vi.fn(), onChange: vi.fn() });
    const state = EditorState.create({ doc, plugins: [plugin], selection: TextSelection.create(doc, doc.content.size - 1) });
    expect(suggestKey.getState(state)!.ctx).toBeNull();
  });
});
