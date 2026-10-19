// Element progression rules (UX §4 "What Enter does", mock 077; FSD §16.2).
// Context rules only *suggest* the next element: the writer can always
// override with the element selector or Ctrl+1…7.

import type { ElementType } from "../../../ipc/generated/ElementType";
import { isElementType, ruleFor } from "./schema";

/** Enter at the end of a non-empty element: the element that follows, from the
 * shared element rule table (custom elements name their own next element). */
export function nextOnEnter(t: ElementType): ElementType {
  const next = ruleFor(t)?.nextOnEnter ?? "action";
  return isElementType(next) && next !== "custom" && next !== "page_break" ? next : "action";
}

/** Enter in an EMPTY element converts it instead of adding another blank line.
 * An empty action becomes a scene heading (Enter twice starts a new scene). */
export function onEnterEmpty(t: ElementType): ElementType | null {
  switch (t) {
    case "scene_heading":
    case "page_break":
      return null;
    case "action":
      return "scene_heading";
    case "parenthetical":
      return "dialogue";
    default:
      return "action";
  }
}

/** Tab cycles within the dialogue block. Page breaks and custom elements stay
 * as they are (null): the writer chose them on purpose. */
export function tabNext(t: ElementType): ElementType | null {
  switch (t) {
    case "page_break":
    case "custom":
      return null;
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

/** Shift+Tab goes back (null for page breaks and custom elements). */
export function tabPrev(t: ElementType): ElementType | null {
  switch (t) {
    case "page_break":
    case "custom":
      return null;
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
