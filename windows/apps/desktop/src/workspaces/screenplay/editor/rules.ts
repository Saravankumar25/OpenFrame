// Element progression rules (UX §4 "What Enter does", mock 077; FSD §16.2).
// Context rules only *suggest* the next element: the writer can always
// override with the element selector or Ctrl+1…7.

import type { ElementType } from "../../../ipc/generated/ElementType";
import { SHORTCUT_ORDER, isElementType, ruleFor } from "./schema";

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

/** Tab moves to the next built-in element in menu order and wraps around, so every element is one
 * or more presses away (owner change list item 7, 2026-10-19): Scene Heading, Action, Character,
 * Parenthetical, Dialogue, Transition, Shot, then Scene Heading again. A note becomes Action. Page
 * breaks and custom elements stay as they are (null): the writer chose them on purpose. */
export function tabNext(t: ElementType): ElementType | null {
  if (t === "page_break" || t === "custom") return null;
  const i = SHORTCUT_ORDER.indexOf(t);
  return i < 0 ? "action" : SHORTCUT_ORDER[(i + 1) % SHORTCUT_ORDER.length];
}

/** Shift+Tab goes the other way round the same cycle (null for page breaks and custom elements). */
export function tabPrev(t: ElementType): ElementType | null {
  if (t === "page_break" || t === "custom") return null;
  const i = SHORTCUT_ORDER.indexOf(t);
  return i < 0 ? "action" : SHORTCUT_ORDER[(i - 1 + SHORTCUT_ORDER.length) % SHORTCUT_ORDER.length];
}
