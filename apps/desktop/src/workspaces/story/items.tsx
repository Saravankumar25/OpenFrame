// Shared Story Board item behaviour: context menus, keyboard handling, inline
// text editing and colour tokens (used by Board and Outline views).

import { useEffect, useRef, type KeyboardEvent, type MouseEvent as ReactMouseEvent, type PointerEvent as ReactPointerEvent } from "react";
import { story } from "../../api/story";
import { reportError } from "../../ipc/query";
import { toast } from "../../app/toast";
import type { MenuItemSpec } from "../../design-system";
import type { StoryBoardState } from "../../ipc/generated/StoryBoardState";
import type { StoryItem } from "../../ipc/generated/StoryItem";
import {
  convertBeat,
  copyAsBeat,
  duplicateItems,
  newBeat,
  newCard,
  parkItems,
  requestDelete,
  restoreToStory,
  runMove,
} from "./actions";
import { flattenVisible, locate, matchesFilter, refKey, SEQ, stepHint, type Ref } from "./model";
import { keyToRef, useStoryUi } from "./ui";
import { requestDeleteSequence } from "./dialogs";

export const COLOR_VARS: Record<string, string> = {
  yellow: "var(--yellow)",
  orange: "var(--accent)",
  red: "var(--red)",
  pink: "#d6589a",
  purple: "var(--purple)",
  blue: "var(--blue)",
  teal: "var(--teal)",
  green: "var(--green)",
  gray: "#9aa0ab",
};
export const COLOR_NAMES = Object.keys(COLOR_VARS);

/** Refs the action applies to: the whole selection when the item is part of it. */
export function targetsFor(ref: Ref): Ref[] {
  const sel = useStoryUi.getState().selected;
  const key = refKey(ref);
  if (sel.length > 1 && sel.includes(key)) return sel.map(keyToRef).filter((r) => r.kind !== "sequence");
  return [ref];
}

export function focusItem(key: string | null) {
  if (!key) return;
  const el = document.querySelector<HTMLElement>(`[data-story-key="${CSS.escape(key)}"]`);
  el?.focus();
  el?.scrollIntoView({ block: "nearest", inline: "nearest" });
}

/** Context menu for a card or beat (UX §3.6, mock 056). */
export function itemMenu(board: StoryBoardState, item: StoryItem, episodeId: string | null, readOnly: boolean): MenuItemSpec[] {
  const ref: Ref = { kind: item.kind, id: item.id };
  const ui = useStoryUi.getState();
  const loc = locate(board, ref);
  const parked = loc?.container.parentType === "parking";
  if (item.kind === "sequence") {
    return [
      { label: "Open", shortcut: "Enter", onSelect: () => ui.openDrawer({ kind: "sequence", id: item.id }) },
      { label: "Rename", disabled: readOnly, onSelect: () => ui.setRenaming(item.id) },
      { label: "Move…", disabled: readOnly, onSelect: () => ui.openDialog({ type: "move", refs: [ref] }) },
      { label: "Add Scene", disabled: readOnly, onSelect: () => void newCard(episodeId, SEQ(item.id)) },
      { label: "Add Beat", disabled: readOnly, onSelect: () => void newBeat(episodeId, SEQ(item.id)) },
      { label: "Delete", danger: true, separatorBefore: true, disabled: readOnly, onSelect: () => requestDeleteSequence(board, item.id) },
    ];
  }
  const targets = () => targetsFor(ref);
  const common: MenuItemSpec[] = [
    { label: "Duplicate", shortcut: "Ctrl+D", disabled: readOnly, onSelect: () => void duplicateItems(board, targets()) },
    { label: "Move…", disabled: readOnly, onSelect: () => ui.openDialog({ type: "move", refs: targets() }) },
    parked
      ? { label: "Restore to Story", disabled: readOnly, onSelect: () => void restoreToStory(board, targets()) }
      : { label: "Send to Parking Lot", disabled: readOnly, onSelect: () => void parkItems(board, targets()) },
  ];
  if (item.kind === "beat") {
    return [
      { label: "Open", shortcut: "Enter", onSelect: () => ui.openDrawer({ kind: "beat", id: item.id }) },
      { label: "Convert to Scene", disabled: readOnly || item.state === "converted", onSelect: () => void convertBeat(item.id) },
      ...common,
      { label: "Delete", shortcut: "Del", danger: true, separatorBefore: true, disabled: readOnly, onSelect: () => requestDelete(board, targets()) },
    ];
  }
  return [
    { label: "Open", shortcut: "Enter", onSelect: () => ui.openDrawer({ kind: "card", id: item.id }) },
    ...common,
    {
      label: "Convert to Screenplay Scene",
      disabled: readOnly || parked,
      onSelect: () => ui.openDialog({ type: "build", preselect: targets().filter((r) => r.kind === "card").map((r) => r.id) }),
    },
    { label: "Copy as Beat", disabled: readOnly, onSelect: () => void copyAsBeat(item.id) },
    { label: "Delete", shortcut: "Del", danger: true, separatorBefore: true, disabled: readOnly, onSelect: () => requestDelete(board, targets()) },
  ];
}

/** Keyboard behaviour for a focused card/beat/sequence (UX §3.6 shortcuts, FSD §51). */
export function useItemKeys(board: StoryBoardState, episodeId: string | null, readOnly: boolean) {
  return (e: KeyboardEvent<HTMLElement>, item: StoryItem) => {
    if (e.target !== e.currentTarget) return;
    const ref: Ref = { kind: item.kind, id: item.id };
    const key = refKey(ref);
    const ui = useStoryUi.getState();
    const mod = e.ctrlKey || e.metaKey;
    if (e.key === "Enter") {
      e.preventDefault();
      ui.openDrawer({ kind: item.kind, id: item.id });
    } else if ((e.key === "Delete" || e.key === "Backspace") && !readOnly) {
      e.preventDefault();
      if (item.kind === "sequence") requestDeleteSequence(board, item.id);
      else requestDelete(board, targetsFor(ref));
    } else if (e.key === " ") {
      e.preventDefault();
      ui.toggle(key);
    } else if (e.key === "Escape") {
      ui.clearSelection();
    } else if (mod && e.key.toLowerCase() === "d" && !readOnly) {
      e.preventDefault();
      void duplicateItems(board, targetsFor(ref));
    } else if (e.altKey && (e.key === "ArrowUp" || e.key === "ArrowDown") && !readOnly) {
      e.preventDefault();
      const hint = stepHint(board, ref, e.key === "ArrowUp" ? -1 : 1);
      if (hint) void runMove(board, [ref], hint, episodeId).then(() => window.setTimeout(() => focusItem(key), 50));
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp" || e.key === "ArrowLeft" || e.key === "ArrowRight") {
      e.preventDefault();
      const collapsed = new Set(board.collapsedIds);
      const flat = flattenVisible(board, collapsed, (i) => matchesFilter(i, ui.filter));
      const idx = flat.findIndex((f) => refKey(f.ref) === key);
      if (idx < 0) return;
      let next = idx;
      if (e.key === "ArrowDown") next = Math.min(flat.length - 1, idx + 1);
      else if (e.key === "ArrowUp") next = Math.max(0, idx - 1);
      else {
        const dir = e.key === "ArrowRight" ? 1 : -1;
        const act = flat[idx].actId;
        let j = idx;
        while (j >= 0 && j < flat.length && flat[j].actId === act) j += dir;
        if (j >= 0 && j < flat.length) {
          const target = flat[j].actId;
          while (dir < 0 && j > 0 && flat[j - 1].actId === target) j -= 1;
          next = j;
        }
      }
      const nk = refKey(flat[next].ref);
      if (e.shiftKey) ui.select([...new Set([...ui.selected, key, nk])]);
      focusItem(nk);
    }
  };
}

/** Click selection: plain click selects one; Ctrl/Shift+click toggles (multi-select). */
export function onItemClick(e: ReactMouseEvent, key: string) {
  e.stopPropagation();
  const ui = useStoryUi.getState();
  if (e.ctrlKey || e.metaKey || e.shiftKey) ui.toggle(key);
  else ui.select([key]);
}

/** Inline plain-text editor used for new cards/beats and renames. */
export function InlineText({
  value,
  onCommit,
  onCancel,
  multiline,
  placeholder,
  ariaLabel,
  className,
}: {
  value: string;
  onCommit: (v: string) => void;
  onCancel: () => void;
  multiline?: boolean;
  placeholder?: string;
  ariaLabel: string;
  className?: string;
}) {
  const ref = useRef<HTMLTextAreaElement & HTMLInputElement>(null);
  const done = useRef(false);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    el.focus();
    el.setSelectionRange(el.value.length, el.value.length);
  }, []);
  const commit = () => {
    if (done.current) return;
    done.current = true;
    onCommit(ref.current?.value ?? value);
  };
  const keys = (e: KeyboardEvent<HTMLElement>) => {
    e.stopPropagation();
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      commit();
    } else if (e.key === "Escape") {
      e.preventDefault();
      done.current = true;
      onCancel();
    }
  };
  const common = {
    ref,
    defaultValue: value,
    placeholder,
    "aria-label": ariaLabel,
    onBlur: commit,
    onKeyDown: keys,
    onPointerDown: (e: ReactPointerEvent) => e.stopPropagation(),
    onClick: (e: ReactMouseEvent) => e.stopPropagation(),
    className: className ?? "story-inline",
  };
  return multiline ? <textarea rows={3} {...common} /> : <input {...common} />;
}

/** Save an inline text edit for a card/beat. */
export async function commitItemText(item: StoryItem, text: string): Promise<void> {
  try {
    if (item.kind === "card" && text !== item.shortDescription) await story.updateCard({ id: item.id, shortDescription: text });
    if (item.kind === "beat" && text !== item.text) await story.updateBeat({ id: item.id, text });
    if (item.kind === "sequence" && text.trim() && text.trim() !== item.title) await story.updateSequence({ id: item.id, title: text.trim() });
  } catch (e) {
    reportError(e);
  }
}

export async function renameAct(id: string, current: string, title: string): Promise<void> {
  const t = title.trim();
  if (!t || t === current) return;
  try {
    await story.updateAct({ id, title: t });
    toast.undoable(`Renamed Act to “${t}”`);
  } catch (e) {
    reportError(e);
  }
}
