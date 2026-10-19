import { describe, expect, it } from "vitest";
import {
  SHORTCUT_COMMANDS,
  bindingsFor,
  checkChord,
  chordFromEvent,
  displayChord,
  findConflicts,
  matches,
  toOverrides,
  useShortcuts,
  whoUses,
} from "./shortcuts";

const key = (code: string, k: string, mods: { ctrl?: boolean; meta?: boolean; alt?: boolean; shift?: boolean } = {}) => ({
  code,
  key: k,
  ctrlKey: !!mods.ctrl,
  metaKey: !!mods.meta,
  altKey: !!mods.alt,
  shiftKey: !!mods.shift,
});

describe("keyboard shortcut catalogue (SCRIPT-019)", () => {
  it("has unique ids, an on-screen route for every command and no conflicts by default", () => {
    const ids = SHORTCUT_COMMANDS.map((c) => c.id);
    expect(new Set(ids).size).toBe(ids.length);
    for (const c of SHORTCUT_COMMANDS) {
      expect(c.route.length, c.id).toBeGreaterThan(5);
      for (const d of c.defaults) expect(checkChord(d), `${c.id} ${d}`).toEqual({ chord: d });
    }
    expect(findConflicts({})).toEqual([]);
  });

  it("reads chords from key presses using the physical key", () => {
    expect(chordFromEvent(key("Digit1", "!", { ctrl: true, shift: true }))).toBe("Mod+Shift+1");
    expect(chordFromEvent(key("KeyF", "f", { meta: true }))).toBe("Mod+F");
    expect(chordFromEvent(key("ArrowDown", "ArrowDown", { alt: true }))).toBe("Alt+ArrowDown");
    expect(chordFromEvent(key("F7", "F7"))).toBe("F7");
    expect(chordFromEvent(key("Slash", "/", { ctrl: true }))).toBe("Mod+/");
    expect(chordFromEvent(key("ControlLeft", "Control", { ctrl: true }))).toBeNull();
  });

  it("follows the typing layout for letters and ignores composing text and AltGr", () => {
    // German QWERTZ: the key labelled Z sits where QWERTY has Y; Ctrl+Z must stay Undo.
    expect(chordFromEvent(key("KeyY", "z", { ctrl: true }))).toBe("Mod+Z");
    // A Telugu layout gives a Telugu letter: the physical key is used instead.
    expect(chordFromEvent(key("KeyK", "\u0C15", { ctrl: true }))).toBe("Mod+K");
    expect(chordFromEvent({ ...key("KeyK", "k", { ctrl: true }), isComposing: true })).toBeNull();
    expect(chordFromEvent({ ...key("KeyK", "k", { ctrl: true }), keyCode: 229 })).toBeNull();
    expect(chordFromEvent({ ...key("Digit7", "{", { ctrl: true, alt: true }), getModifierState: (k: string) => k === "AltGraph" })).toBeNull();
  });

  it("refuses chords that would block typing or the clipboard, like Rust does", () => {
    expect(checkChord("shift+Mod+n")).toEqual({ chord: "Mod+Shift+N" });
    for (const bad of ["K", "Shift+K", "Enter", "Mod+C", "Mod+V", "Mod+X", "Mod+A", "Ctrl+K", "Mod+Mod+K", "Mod+F99"]) {
      expect("error" in checkChord(bad), bad).toBe(true);
    }
  });

  it("shows chords the way each system does", () => {
    expect(displayChord("Mod+Shift+N", false)).toBe("Ctrl+Shift+N");
    expect(displayChord("Alt+ArrowDown", false)).toBe("Alt+Down");
    expect(displayChord("Mod+Shift+N", true)).toBe("⌘⇧N");
  });

  it("merges saved changes with defaults: remap, remove, several per command, restore", () => {
    expect(bindingsFor("screenplay.find", {})).toEqual(["Mod+F"]);
    expect(bindingsFor("screenplay.find", { "screenplay.find": ["Mod+Shift+F", "F3"] })).toEqual(["Mod+Shift+F", "F3"]);
    expect(bindingsFor("app.assistant", { "app.assistant": [] })).toEqual([]);
    // Unknown commands from an older version are ignored.
    expect(toOverrides({ overrides: [{ commandId: "gone.command", keys: ["F2"] }, { commandId: "app.search", keys: ["F2"] }] })).toEqual({ "app.search": ["F2"] });
  });

  it("detects conflicts only where both commands are active", () => {
    // Ctrl+N is New Scene Card in a project and New Project on Home: no conflict.
    expect(whoUses("Mod+N", "home", {}, "home.new_project")).toEqual([]);
    // Ctrl+K in the screenplay is already Search (project-wide).
    expect(whoUses("Mod+K", "screenplay", {})).toEqual(["app.search"]);
    const clash = { "screenplay.find": ["Mod+K"] };
    const found = findConflicts(clash);
    expect(found).toContainEqual({ chord: "Mod+K", commandId: "screenplay.find", otherId: "app.search" });
    expect(found).toContainEqual({ chord: "Mod+K", commandId: "app.search", otherId: "screenplay.find" });
  });

  it("matches key presses against the live bindings", () => {
    useShortcuts.getState().set({ "screenplay.find": ["F3"] });
    expect(matches(new KeyboardEvent("keydown", { key: "F3", code: "F3" }), "screenplay.find")).toBe(true);
    expect(matches(new KeyboardEvent("keydown", { key: "f", code: "KeyF", ctrlKey: true }), "screenplay.find")).toBe(false);
    useShortcuts.getState().set({});
    expect(matches(new KeyboardEvent("keydown", { key: "f", code: "KeyF", ctrlKey: true }), "screenplay.find")).toBe(true);
  });

  it("the Idea Vault shares project keys but not the screenplay's or Home's", () => {
    // Ctrl+F is Find in the script and Search ideas in the vault: they never show together.
    expect(whoUses("Mod+F", "vault", {}, "vault.search")).toEqual([]);
    // A vault key that is already a project key is a conflict.
    expect(whoUses("Mod+K", "vault", {})).toEqual(["app.search"]);
    expect(findConflicts({ "vault.search": ["Mod+K"] })).toContainEqual({ chord: "Mod+K", commandId: "vault.search", otherId: "app.search" });
  });
});

describe("keys owned by the screenplay", () => {
  it("Ctrl+Alt+1 to 9 belong to custom elements: binding one is refused with a conflict message", () => {
    for (let d = 1; d <= 9; d++) {
      const r = checkChord(`Mod+Alt+${d}`);
      expect("error" in r && r.error, `Mod+Alt+${d}`).toContain("custom elements");
    }
    expect(checkChord("Mod+Alt+0")).toEqual({ chord: "Mod+Alt+0" });
  });

  it("the page break is an editable command with Ctrl+Enter by default", () => {
    expect(bindingsFor("screenplay.page_break", {})).toEqual(["Mod+Enter"]);
    // Taking Ctrl+Enter for another screenplay command shows the conflict.
    expect(whoUses("Mod+Enter", "screenplay", {}, "screenplay.find")).toEqual(["screenplay.page_break"]);
    useShortcuts.getState().set({ "screenplay.page_break": ["Mod+Shift+Enter"] });
    expect(matches(new KeyboardEvent("keydown", { key: "Enter", code: "Enter", ctrlKey: true, shiftKey: true }), "screenplay.page_break")).toBe(true);
    expect(matches(new KeyboardEvent("keydown", { key: "Enter", code: "Enter", ctrlKey: true }), "screenplay.page_break")).toBe(false);
    useShortcuts.getState().set({});
  });
});
