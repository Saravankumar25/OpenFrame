// Element progression rules (UX §4 "What Enter does", mock 077; FSD §16.2).
// Context rules only *suggest* the next element: the writer can always
// override with the element selector or Ctrl+1…7.

import type { ElementType } from "../../../ipc/generated/ElementType";

/** Enter at the end of a non-empty element: the element that follows. */
export function nextOnEnter(t: ElementType): ElementType {
  switch (t) {
    case "scene_heading":
      return "action";
    case "action":
      return "character";
    case "character":
      return "dialogue";
    case "parenthetical":
      return "dialogue";
    case "dialogue":
      return "action";
    case "transition":
      return "scene_heading";
    case "shot":
      return "action";
    case "note":
      return "action";
  }
}

/** Enter in an EMPTY element converts it instead of adding another blank line.
 * An empty action becomes a scene heading (Enter twice starts a new scene). */
export function onEnterEmpty(t: ElementType): ElementType | null {
  switch (t) {
    case "scene_heading":
      return null;
    case "action":
      return "scene_heading";
    case "parenthetical":
      return "dialogue";
    default:
      return "action";
  }
}

/** Tab cycles within the dialogue block. */
export function tabNext(t: ElementType): ElementType {
  switch (t) {
    case "action":
      return "character";
    case "character":
      return "parenthetical";
    case "parenthetical":
      return "dialogue";
    case "dialogue":
      return "parenthetical";
    case "scene_heading":
      return "action";
    default:
      return "action";
  }
}

/** Shift+Tab goes back. */
export function tabPrev(t: ElementType): ElementType {
  switch (t) {
    case "character":
      return "action";
    case "parenthetical":
      return "character";
    case "dialogue":
      return "character";
    case "action":
      return "scene_heading";
    default:
      return "action";
  }
}
