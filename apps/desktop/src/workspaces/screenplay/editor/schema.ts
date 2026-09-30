// ProseMirror schema for the screenplay editor (FSD §15.5, UX §4).
//
// The document is a flat list of element blocks, exactly like a professional
// screenplay editor: every `scene_heading` block starts a new scene, and the
// blocks after it (until the next heading) are that scene's elements. The
// heading block's `id` is the scene id; every other block's `id` is the
// element id. The first block is always a scene heading.

import { Schema, type NodeSpec, type DOMOutputSpec } from "prosemirror-model";
import type { ElementType } from "../../../ipc/generated/ElementType";

export interface ElementMeta {
  label: string;
  /** Short label for the element indicator pill. */
  short: string;
  shortcut?: string;
  cls: string;
  /** Multi-line elements accept Shift+Enter line breaks. */
  multiline: boolean;
}

export const ELEMENT_META: Record<ElementType, ElementMeta> = {
  scene_heading: { label: "Scene Heading", short: "Scene Heading", shortcut: "Ctrl+1", cls: "sp-h", multiline: false },
  action: { label: "Action", short: "Action", shortcut: "Ctrl+2", cls: "sp-a", multiline: true },
  character: { label: "Character", short: "Character", shortcut: "Ctrl+3", cls: "sp-c", multiline: false },
  parenthetical: { label: "Parenthetical", short: "Parenthetical", shortcut: "Ctrl+4", cls: "sp-p", multiline: false },
  dialogue: { label: "Dialogue", short: "Dialogue", shortcut: "Ctrl+5", cls: "sp-d", multiline: true },
  transition: { label: "Transition", short: "Transition", shortcut: "Ctrl+6", cls: "sp-t", multiline: false },
  shot: { label: "Shot", short: "Shot", shortcut: "Ctrl+7", cls: "sp-sh", multiline: false },
  note: { label: "General note (not printed)", short: "Note (not printed)", cls: "sp-note", multiline: true },
};

/** Ctrl+1 … Ctrl+7 (mock 077 element menu). */
export const SHORTCUT_ORDER: ElementType[] = [
  "scene_heading",
  "action",
  "character",
  "parenthetical",
  "dialogue",
  "transition",
  "shot",
];

export const ELEMENT_TYPES: ElementType[] = [...SHORTCUT_ORDER, "note"];

export function isElementType(v: string): v is ElementType {
  return (ELEMENT_TYPES as string[]).includes(v);
}

function elementNode(type: ElementType): NodeSpec {
  const meta = ELEMENT_META[type];
  return {
    group: "block",
    content: meta.multiline ? "(text | hard_break)*" : "text*",
    marks: "",
    defining: true,
    attrs: { id: { default: null } },
    parseDOM: [
      {
        tag: `div.${meta.cls}`,
        getAttrs: (dom) => ({ id: (dom as HTMLElement).getAttribute("data-id") }),
      },
      ...(type === "action" ? [{ tag: "p" }, { tag: "div" }] : []),
    ],
    toDOM: (node): DOMOutputSpec => [
      "div",
      { class: `sp-el ${meta.cls}`, "data-id": node.attrs.id ?? "", "data-type": type },
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
  },
  marks: {},
});

export function newId(): string {
  return crypto.randomUUID();
}
