// ProseMirror schema for the screenplay editor (FSD §15.5, UX §4).
//
// The document is a flat list of element blocks, exactly like a professional
// screenplay editor: every `scene_heading` block starts a new scene, and the
// blocks after it (until the next heading) are that scene's elements. The
// heading block's `id` is the scene id; every other block's `id` is the
// element id. The first block is always a scene heading.
//
// Two blocks are special: `page_break` is an explicit page break (an atom with
// no text), and `custom` is a custom element whose `customId` names its
// definition (look, case and next element come from the document's custom
// elements). Labels, shortcuts and line-break rules come from the element rule
// table shared with the page layout (`elementRules.json`).

import { Schema, type NodeSpec, type DOMOutputSpec } from "prosemirror-model";
import type { ElementType } from "../../../ipc/generated/ElementType";
import RULES from "./elementRules.json";

export interface ElementMeta {
  label: string;
  /** Short label for the element indicator pill. */
  short: string;
  cls: string;
  /** Multi-line elements accept Shift+Enter line breaks. */
  multiline: boolean;
}

export interface ElementRule {
  kind: string;
  label: string;
  shortcut: string | null;
  multiline: boolean;
  nextOnEnter: string;
  printed: boolean;
}

/** The shared rule of a kind (stored element type or look). */
export function ruleFor(kind: string): ElementRule | undefined {
  return (RULES as ElementRule[]).find((r) => r.kind === kind);
}

function meta(kind: ElementType, cls: string, short?: string): ElementMeta {
  const r = ruleFor(kind);
  return {
    label: r?.label ?? kind,
    short: short ?? r?.label ?? kind,
    cls,
    multiline: r?.multiline ?? false,
  };
}

export const ELEMENT_META: Record<ElementType, ElementMeta> = {
  scene_heading: meta("scene_heading", "sp-h"),
  action: meta("action", "sp-a"),
  character: meta("character", "sp-c"),
  parenthetical: meta("parenthetical", "sp-p"),
  dialogue: meta("dialogue", "sp-d"),
  transition: meta("transition", "sp-t"),
  shot: meta("shot", "sp-sh"),
  note: meta("note", "sp-note", "Note (not printed)"),
  page_break: meta("page_break", "sp-pb"),
  custom: { label: "Custom element", short: "Custom", cls: "sp-cu", multiline: true },
};

/** The built-in elements in menu order (mock 077); their keys come from the shortcut
 * catalogue (app/shortcuts, `screenplay.element_*`), which the person can change. */
export const SHORTCUT_ORDER: ElementType[] = [
  "scene_heading",
  "action",
  "character",
  "parenthetical",
  "dialogue",
  "transition",
  "shot",
];

export const ELEMENT_TYPES: ElementType[] = [...SHORTCUT_ORDER, "note", "page_break", "custom"];

export function isElementType(v: string): v is ElementType {
  return (ELEMENT_TYPES as string[]).includes(v);
}

function elementNode(type: ElementType): NodeSpec {
  const m = ELEMENT_META[type];
  return {
    group: "block",
    content: m.multiline ? "(text | hard_break)*" : "text*",
    marks: "",
    defining: true,
    attrs: { id: { default: null } },
    parseDOM: [
      {
        tag: `div.${m.cls}`,
        getAttrs: (dom) => ({ id: (dom as HTMLElement).getAttribute("data-id") }),
      },
      ...(type === "action" ? [{ tag: "p" }, { tag: "div" }] : []),
    ],
    toDOM: (node): DOMOutputSpec => [
      "div",
      { class: `sp-el ${m.cls}`, "data-id": node.attrs.id ?? "", "data-type": type },
      0,
    ],
  };
}

export const schema = new Schema({
  nodes: {
    doc: { content: "scene_heading block*" },
    text: { group: "inline" },
    hard_break: {
      inline: true,
      group: "inline",
      selectable: false,
      leafText: () => "\n",
      parseDOM: [{ tag: "br" }],
      toDOM: () => ["br"],
    },
    scene_heading: elementNode("scene_heading"),
    action: elementNode("action"),
    character: elementNode("character"),
    parenthetical: elementNode("parenthetical"),
    dialogue: elementNode("dialogue"),
    transition: elementNode("transition"),
    shot: elementNode("shot"),
    note: elementNode("note"),
    // An explicit page break: an atom without text that the writer can select and delete.
    page_break: {
      group: "block",
      atom: true,
      selectable: true,
      attrs: { id: { default: null } },
      parseDOM: [{ tag: "div.sp-pb", getAttrs: (dom) => ({ id: (dom as HTMLElement).getAttribute("data-id") }) }],
      toDOM: (node): DOMOutputSpec => [
        "div",
        { class: "sp-el sp-pb", "data-id": node.attrs.id ?? "", "data-type": "page_break", role: "separator", "aria-label": "Page break" },
        ["span", { class: "sp-pb-label" }, "Page break"],
      ],
    },
    // A custom element; its look is applied as a class by the editor (see ScreenplayEditor).
    custom: {
      group: "block",
      content: "(text | hard_break)*",
      marks: "",
      defining: true,
      attrs: { id: { default: null }, customId: { default: null } },
      parseDOM: [
        {
          tag: "div.sp-cu",
          getAttrs: (dom) => ({
            id: (dom as HTMLElement).getAttribute("data-id"),
            customId: (dom as HTMLElement).getAttribute("data-custom"),
          }),
        },
      ],
      toDOM: (node): DOMOutputSpec => [
        "div",
        { class: "sp-el sp-cu", "data-id": node.attrs.id ?? "", "data-type": "custom", "data-custom": node.attrs.customId ?? "" },
        0,
      ],
    },
  },
  marks: {},
});

export function newId(): string {
  return crypto.randomUUID();
}
