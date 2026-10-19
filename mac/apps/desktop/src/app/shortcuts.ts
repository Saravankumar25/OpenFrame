// Keyboard shortcuts (SCRIPT-019; Screenplay and Production Depth Specification §20).
//
// One catalogue of every command that has a keyboard shortcut, with its default keys and the
// on-screen route that does the same thing without a keyboard. The person's changes are stored
// by Rust (`app.shortcuts`, application database); this module merges them with the defaults,
// matches key presses and finds conflicts. Keys are logical chords such as "Mod+Shift+N":
// "Mod" is Ctrl on Windows and Command on macOS.

import { useEffect } from "react";
import { create } from "zustand";
import { call, onAppEvent } from "../ipc/client";
import type { ShortcutOverrides } from "../ipc/generated/ShortcutOverrides";

/** Where a shortcut works. "project" is the window with a project open; "screenplay" is the
 * script editor inside it; "vault" is the Idea Vault inside it; "home" is the project list with
 * no project open. */
export type ShortcutScope = "project" | "screenplay" | "vault" | "home";

export interface ShortcutCommand {
  id: string;
  label: string;
  group: string;
  scope: ShortcutScope;
  defaults: string[];
  /** The on-screen way to run the same command. */
  route: string;
}

const el = (id: string, label: string, key: string | null): ShortcutCommand => ({
  id: `screenplay.element_${id}`,
  label: `${label} element`,
  group: "Screenplay elements",
  scope: "screenplay",
  defaults: key ? [key] : [],
  route: "Element menu at the bottom of the page",
});

export const SHORTCUT_COMMANDS: ShortcutCommand[] = [
  { id: "app.search", label: "Search", group: "General", scope: "project", defaults: ["Mod+K"], route: "Search box at the top of the window" },
  { id: "app.assistant", label: "Open or close the assistant", group: "General", scope: "project", defaults: ["Mod+J"], route: "Assistant button at the top right" },
  { id: "app.save", label: "Save now", group: "General", scope: "project", defaults: ["Mod+S"], route: "Project menu: Save now" },
  { id: "app.undo", label: "Undo", group: "General", scope: "project", defaults: ["Mod+Z"], route: "Undo button at the top" },
  { id: "app.redo", label: "Redo", group: "General", scope: "project", defaults: ["Mod+Y", "Mod+Shift+Z"], route: "Redo button at the top" },
  { id: "app.new_scene_card", label: "New Scene Card", group: "General", scope: "project", defaults: ["Mod+N"], route: "Story Board: Add" },
  { id: "app.quick_capture", label: "Quick Capture (new idea)", group: "General", scope: "project", defaults: ["Mod+Shift+N"], route: "Idea Vault: New" },
  { id: "home.new_project", label: "New Project", group: "Project list", scope: "home", defaults: ["Mod+N"], route: "Home: New Project button" },
  { id: "home.open_project", label: "Open Project", group: "Project list", scope: "home", defaults: ["Mod+O"], route: "Home: Open button" },
  { id: "home.search", label: "Search projects", group: "Project list", scope: "home", defaults: ["Mod+F", "Mod+K"], route: "Home: search box" },
  { id: "home.quick_capture", label: "Quick Capture into the Global Idea Vault", group: "Project list", scope: "home", defaults: ["Mod+Shift+N"], route: "Home: Global Idea Vault" },
  { id: "vault.search", label: "Search ideas", group: "Idea Vault", scope: "vault", defaults: ["Mod+F"], route: "Idea Vault: search box" },
  { id: "screenplay.find", label: "Find in script", group: "Screenplay", scope: "screenplay", defaults: ["Mod+F"], route: "Screenplay toolbar: Find" },
  { id: "screenplay.next_scene", label: "Next scene", group: "Screenplay", scope: "screenplay", defaults: ["Alt+ArrowDown"], route: "Scene navigator on the left" },
  { id: "screenplay.previous_scene", label: "Previous scene", group: "Screenplay", scope: "screenplay", defaults: ["Alt+ArrowUp"], route: "Scene navigator on the left" },
  { id: "screenplay.next_misspelling", label: "Next spelling mistake", group: "Screenplay", scope: "screenplay", defaults: ["F7"], route: "Right-click an underlined word" },
  { id: "screenplay.page_break", label: "Insert page break", group: "Screenplay", scope: "screenplay", defaults: ["Mod+Enter"], route: "Element menu: Insert page break" },
  el("scene_heading", "Scene Heading", "Mod+1"),
  el("action", "Action", "Mod+2"),
  el("character", "Character", "Mod+3"),
  el("parenthetical", "Parenthetical", "Mod+4"),
  el("dialogue", "Dialogue", "Mod+5"),
  el("transition", "Transition", "Mod+6"),
  el("shot", "Shot", "Mod+7"),
  el("note", "General note (not printed)", null),
];

const BY_ID = new Map(SHORTCUT_COMMANDS.map((c) => [c.id, c]));
export const MAX_KEYS_PER_COMMAND = 4;

// ---------------------------------------------------------------- chords

const MODS = ["Mod", "Alt", "Shift"] as const;
const NAMED = new Set([
  "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Enter", "Escape", "Tab", "Backspace",
  "Delete", "Home", "End", "PageUp", "PageDown", "Space", "Insert",
]);
const PUNCT_BY_CODE: Record<string, string> = {
  BracketLeft: "[", BracketRight: "]", Semicolon: ";", Quote: "'", Comma: ",", Period: ".",
  Slash: "/", Backslash: "\\", Minus: "-", Equal: "=", Backquote: "`",
};
const PUNCT = new Set(Object.values(PUNCT_BY_CODE));
const RESERVED = new Set(["Mod+A", "Mod+C", "Mod+V", "Mod+X"]);
/** Owned by the screenplay's custom elements (View, Custom elements). */
const CUSTOM_ELEMENT_KEYS = new Set(Array.from({ length: 9 }, (_, i) => `Mod+Alt+${i + 1}`));

const isFunctionKey = (k: string) => /^F([1-9]|1[0-9]|2[0-4])$/.test(k);

type KeyLike = Pick<KeyboardEvent, "key" | "code" | "ctrlKey" | "metaKey" | "altKey" | "shiftKey"> &
  Partial<Pick<KeyboardEvent, "isComposing" | "keyCode" | "getModifierState">>;

/** The chord a key press makes, or null when it is not a shortcut: a lone modifier, a key press
 * that is part of composing text (Telugu, Hindi and other input methods), or AltGr (which types
 * characters on many layouts, Indian Inscript among them).
 *
 * Letters follow the layout the person types with (Ctrl+Z is Undo on QWERTZ and AZERTY too);
 * on a non-Latin layout, where the key gives a letter of another script, the physical key is
 * used. Digits and punctuation use the physical key so Shift+1 is "Shift+1" and not "!". */
export function chordFromEvent(e: KeyLike): string | null {
  if (e.isComposing || e.keyCode === 229) return null;
  if (e.getModifierState?.("AltGraph")) return null;
  let key: string | null = null;
  if (/^[a-z]$/i.test(e.key) && /^Key[A-Z]$/.test(e.code)) key = e.key.toUpperCase();
  else if (/^Key[A-Z]$/.test(e.code)) key = e.code.slice(3);
  else if (/^Digit[0-9]$/.test(e.code)) key = e.code.slice(5);
  else if (/^Numpad[0-9]$/.test(e.code)) key = e.code.slice(6);
  else if (PUNCT_BY_CODE[e.code]) key = PUNCT_BY_CODE[e.code];
  else if (e.key === " ") key = "Space";
  else if (NAMED.has(e.key) || isFunctionKey(e.key)) key = e.key;
  else if (e.key.length === 1) key = e.key.toUpperCase();
  if (!key || ["Control", "Meta", "Alt", "Shift"].includes(e.key)) return null;
  const parts: string[] = [];
  if (e.ctrlKey || e.metaKey) parts.push("Mod");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  parts.push(key);
  return parts.join("+");
}

/** Canonical form, or an error sentence (the same rules Rust applies before saving). */
export function checkChord(raw: string): { chord: string } | { error: string } {
  const parts = raw.split("+").map((p) => p.trim());
  const keyRaw = parts.pop() ?? "";
  const key = keyRaw.length === 1 ? keyRaw.toUpperCase() : keyRaw;
  const mods = new Set<string>();
  for (const raw of parts) {
    // Modifier names are accepted in any case ("shift" is "Shift").
    const m = MODS.find((x) => x.toLowerCase() === raw.toLowerCase());
    if (!m || mods.has(m)) return { error: "That key combination can't be used." };
    mods.add(m);
  }
  const known = /^[A-Z0-9]$/.test(key) || PUNCT.has(key) || NAMED.has(key) || isFunctionKey(key);
  if (!known) return { error: "That key combination can't be used." };
  if (!mods.has("Mod") && !mods.has("Alt") && !isFunctionKey(key)) {
    return { error: "Add Ctrl or Alt to that key, otherwise it would stop you typing." };
  }
  const chord = [...MODS.filter((m) => mods.has(m)), key].join("+");
  if (RESERVED.has(chord)) return { error: "That shortcut is kept for copy, cut, paste and select all." };
  if (CUSTOM_ELEMENT_KEYS.has(chord)) {
    return { error: "Ctrl+Alt+1 to Ctrl+Alt+9 belong to the screenplay's custom elements. Choose another shortcut." };
  }
  return { chord };
}

const isMac = () => typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);

const KEY_NAMES: Record<string, string> = {
  ArrowUp: "Up", ArrowDown: "Down", ArrowLeft: "Left", ArrowRight: "Right", Escape: "Esc", PageUp: "Page Up", PageDown: "Page Down",
};

/** How a chord is shown: "Ctrl+Shift+N" on Windows, "⌘⇧N" on macOS. */
export function displayChord(chord: string, mac = isMac()): string {
  const parts = chord.split("+").map((p) => {
    if (p === "Mod") return mac ? "⌘" : "Ctrl";
    if (p === "Alt") return mac ? "⌥" : "Alt";
    if (p === "Shift") return mac ? "⇧" : "Shift";
    return KEY_NAMES[p] ?? p;
  });
  return parts.join(mac ? "" : "+");
}

// ---------------------------------------------------------------- bindings

export type Overrides = Record<string, string[]>;

export function bindingsFor(id: string, overrides: Overrides): string[] {
  return overrides[id] ?? BY_ID.get(id)?.defaults ?? [];
}

/** The screenplay editor and the Idea Vault are parts of the project window, so a project
 * shortcut works in both; they never show at the same time. */
function overlaps(a: ShortcutScope, b: ShortcutScope): boolean {
  if (a === b) return true;
  const pair = new Set([a, b]);
  return pair.has("project") && (pair.has("screenplay") || pair.has("vault"));
}

export interface Conflict {
  chord: string;
  commandId: string;
  otherId: string;
}

/** Commands (other than `exceptId`) that already use `chord` where `scope` is active. */
export function whoUses(chord: string, scope: ShortcutScope, overrides: Overrides, exceptId?: string): string[] {
  return SHORTCUT_COMMANDS.filter(
    (c) => c.id !== exceptId && overlaps(c.scope, scope) && bindingsFor(c.id, overrides).includes(chord),
  ).map((c) => c.id);
}

/** Every pair of commands sharing a chord in overlapping scopes (a damaged or old saved set). */
export function findConflicts(overrides: Overrides): Conflict[] {
  const out: Conflict[] = [];
  for (const c of SHORTCUT_COMMANDS) {
    for (const chord of bindingsFor(c.id, overrides)) {
      for (const other of whoUses(chord, c.scope, overrides, c.id)) out.push({ chord, commandId: c.id, otherId: other });
    }
  }
  return out;
}

export function commandById(id: string): ShortcutCommand | undefined {
  return BY_ID.get(id);
}

// ---------------------------------------------------------------- live store

interface ShortcutState {
  overrides: Overrides;
  loaded: boolean;
  set: (o: Overrides) => void;
}

export const useShortcuts = create<ShortcutState>((set) => ({
  overrides: {},
  loaded: false,
  set: (overrides) => set({ overrides, loaded: true }),
}));

export function toOverrides(r: ShortcutOverrides): Overrides {
  const o: Overrides = {};
  for (const x of r.overrides) if (BY_ID.has(x.commandId)) o[x.commandId] = x.keys;
  return o;
}

let reloadSeq = 0;

export async function reloadShortcuts(): Promise<void> {
  const mine = ++reloadSeq;
  try {
    const r = await call<ShortcutOverrides>("app.shortcuts", {});
    // An older reload that answers late never replaces a newer answer.
    if (mine === reloadSeq) useShortcuts.getState().set(toOverrides(r));
  } catch {
    // Keep whatever is already applied (the defaults on first load): a passing failure must not
    // silently switch the person's shortcuts back to the defaults.
    if (mine === reloadSeq) useShortcuts.setState({ loaded: true });
  }
}

/** Load the saved shortcuts once and follow changes made in Settings (any window). */
export function useShortcutSync(): void {
  useEffect(() => {
    void reloadShortcuts();
    const un = onAppEvent((ev) => {
      if (ev.type === "module" && ev.module === "writing" && ev.name === "shortcutsChanged") void reloadShortcuts();
    });
    return () => {
      void un.then((f) => f());
    };
  }, []);
}

/** Does this key press run `id` with the current bindings? */
export function matches(e: KeyboardEvent, id: string): boolean {
  const chord = chordFromEvent(e);
  return chord !== null && bindingsFor(id, useShortcuts.getState().overrides).includes(chord);
}

/** The first chord of a command, ready to show in a tooltip or menu ("" when none). */
export function shortcutLabel(id: string): string {
  const first = bindingsFor(id, useShortcuts.getState().overrides)[0];
  return first ? displayChord(first) : "";
}

/** React hook form of `shortcutLabel` that re-renders when shortcuts change. */
export function useShortcutLabel(id: string): string {
  const overrides = useShortcuts((s) => s.overrides);
  const first = bindingsFor(id, overrides)[0];
  return first ? displayChord(first) : "";
}
