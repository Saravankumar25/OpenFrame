// Settings > Keyboard shortcuts (SCRIPT-019): remap, remove, several per command, conflicts,
// restore one, restore all. The IPC boundary is replaced by an in-memory store.

import { beforeEach, describe, expect, it, vi } from "vitest";
import { createRoot } from "react-dom/client";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ShortcutsSettings } from "./Shortcuts";
import { useShortcuts } from "../../app/shortcuts";

let stored: Record<string, string[]> = {};
const calls: { op: string; args: unknown }[] = [];

vi.mock("../../ipc/client", () => ({
  OpError: class extends Error {},
  inTauri: () => false,
  onAppEvent: () => Promise.resolve(() => undefined),
  call: vi.fn(async (op: string, args: { changes?: { commandId: string; keys: string[] | null }[] }) => {
    calls.push({ op, args });
    if (op === "app.set_shortcuts") {
      for (const c of args.changes ?? []) {
        if (c.keys === null) delete stored[c.commandId];
        else stored[c.commandId] = c.keys;
      }
    }
    if (op === "app.reset_shortcuts") stored = {};
    return { overrides: Object.entries(stored).map(([commandId, keys]) => ({ commandId, keys })) };
  }),
}));

beforeEach(() => {
  stored = {};
  calls.length = 0;
  useShortcuts.getState().set({});
});

const row = (name: string) => {
  const th = screen.getAllByRole("rowheader").find((h) => h.firstChild?.textContent === name);
  if (!th) throw new Error(`no row ${name}`);
  return th.closest("tr") as HTMLElement;
};

describe("Settings > Keyboard shortcuts", () => {
  it("lists every command with its default, current keys, conflicts and on-screen route", () => {
    render(<ShortcutsSettings />);
    for (const h of ["Command", "Default", "Current", "Conflict"]) expect(screen.getByRole("columnheader", { name: h })).toBeInTheDocument();
    const find = row("Find in script");
    expect(within(find).getByText("On screen: Screenplay toolbar: Find")).toBeInTheDocument();
    expect(within(find).getAllByText("Ctrl+F").length).toBeGreaterThan(0);
    expect(within(row("Redo")).getByText("Ctrl+Y, Ctrl+Shift+Z")).toBeInTheDocument();
  });

  it("adds a second shortcut by pressing the keys", async () => {
    const user = userEvent.setup();
    render(<ShortcutsSettings />);
    await user.click(screen.getByRole("button", { name: "Add a shortcut for Find in script" }));
    const rec = screen.getByRole("button", { name: /Press the new keys for Find in script/ });
    expect(rec).toHaveFocus();
    await user.keyboard("{F3}");
    expect(calls[0]).toEqual({ op: "app.set_shortcuts", args: { changes: [{ commandId: "screenplay.find", keys: ["Mod+F", "F3"] }] } });
    expect(useShortcuts.getState().overrides["screenplay.find"]).toEqual(["Mod+F", "F3"]);
    expect(within(row("Find in script")).getByText("(changed)")).toBeInTheDocument();
  });

  it("refuses a key that would block typing and says why", async () => {
    const user = userEvent.setup();
    render(<ShortcutsSettings />);
    await user.click(screen.getByRole("button", { name: "Add a shortcut for Search" }));
    await user.keyboard("k");
    expect(screen.getByRole("alert")).toHaveTextContent("Add Ctrl or Alt to that key");
    expect(calls).toEqual([]);
  });

  it("Escape cancels recording without changing anything", async () => {
    const user = userEvent.setup();
    render(<ShortcutsSettings />);
    await user.click(screen.getByRole("button", { name: "Add a shortcut for Search" }));
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("button", { name: /Press the new keys/ })).toBeNull();
    expect(calls).toEqual([]);
  });

  it("inside the Home dialog it drops its own heading and tells the dialog while it records keys", async () => {
    const user = userEvent.setup();
    const recording = vi.fn();
    render(<ShortcutsSettings embedded onRecordingChange={recording} />);
    expect(screen.queryByRole("heading", { name: "Keyboard shortcuts" })).toBeNull();
    expect(screen.getByRole("button", { name: "Restore all defaults" })).toBeInTheDocument();
    expect(recording).not.toHaveBeenCalledWith(true);
    await user.click(screen.getByRole("button", { name: "Add a shortcut for New Project" }));
    expect(recording).toHaveBeenLastCalledWith(true);
    await user.keyboard("{Escape}");
    expect(recording).toHaveBeenLastCalledWith(false);
    expect(calls).toEqual([]);
  });

  it("the recorder holds the keys from the moment it shows: focused, and the dialog told, before the next key can arrive", async () => {
    // In the app a key can arrive in the very next task after the click. The recorder used to take
    // focus, and to tell the Home dialog to keep Escape, only in effects that run later; an Escape
    // in between closed the whole dialog. Rendered without act() so nothing flushes those effects
    // early: after the click's own update (a microtask) the recorder must already hold the keys.
    const g = globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean };
    const before = g.IS_REACT_ACT_ENVIRONMENT;
    g.IS_REACT_ACT_ENVIRONMENT = false;
    const host = document.createElement("div");
    document.body.appendChild(host);
    const root = createRoot(host);
    const recording: boolean[] = [];
    try {
      root.render(<ShortcutsSettings embedded onRecordingChange={(r) => recording.push(r)} />);
      await new Promise((r) => setTimeout(r, 50));
      const add = host.querySelector<HTMLButtonElement>('button[aria-label="Add a shortcut for New Project"]')!;
      add.focus();
      add.click();
      await Promise.resolve();
      await Promise.resolve();
      const recorder = host.querySelector(".sc-record");
      expect(recorder).not.toBeNull();
      expect(document.activeElement).toBe(recorder);
      expect(recording.at(-1)).toBe(true);
    } finally {
      root.unmount();
      host.remove();
      g.IS_REACT_ACT_ENVIRONMENT = before;
    }
    // Closed while recording: the dialog is told it may close again.
    expect(recording.at(-1)).toBe(false);
  });

  it("asks before taking a key another command uses, then moves it in one step", async () => {
    const user = userEvent.setup();
    render(<ShortcutsSettings />);
    await user.click(screen.getByRole("button", { name: "Add a shortcut for Find in script" }));
    await user.keyboard("{Control>}k{/Control}");
    expect(await screen.findByText("Ctrl+K is already used")).toBeInTheDocument();
    expect(calls).toEqual([]);
    await user.click(screen.getByRole("button", { name: "Move it here" }));
    expect(calls[0].args).toEqual({
      changes: [
        { commandId: "screenplay.find", keys: ["Mod+F", "Mod+K"] },
        { commandId: "app.search", keys: [] },
      ],
    });
    expect(within(row("Search")).queryByRole("button", { name: /Remove Ctrl\+K/ })).toBeNull();
  });

  it("removes one key and restores the default for one command", async () => {
    const user = userEvent.setup();
    render(<ShortcutsSettings />);
    await user.click(screen.getByRole("button", { name: "Remove Ctrl+J from Open or close the assistant" }));
    expect(calls[0].args).toEqual({ changes: [{ commandId: "app.assistant", keys: [] }] });
    await user.click(screen.getByRole("button", { name: "Restore the default for Open or close the assistant" }));
    expect(calls[1].args).toEqual({ changes: [{ commandId: "app.assistant", keys: null }] });
    expect(useShortcuts.getState().overrides).toEqual({});
  });

  it("restoring a default that is now used elsewhere asks first", async () => {
    stored = { "app.assistant": [], "screenplay.find": ["Mod+F", "Mod+J"] };
    useShortcuts.getState().set({ ...stored });
    const user = userEvent.setup();
    render(<ShortcutsSettings />);
    await user.click(screen.getByRole("button", { name: "Restore the default for Open or close the assistant" }));
    expect(await screen.findByText("Restore the default?")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Restore and move" }));
    expect(calls[0].args).toEqual({
      changes: [
        { commandId: "app.assistant", keys: null },
        { commandId: "screenplay.find", keys: ["Mod+F"] },
      ],
    });
  });

  it("restores all defaults after confirmation", async () => {
    stored = { "app.search": ["F2"] };
    useShortcuts.getState().set({ ...stored });
    const user = userEvent.setup();
    render(<ShortcutsSettings />);
    await user.click(screen.getByRole("button", { name: "Restore all defaults" }));
    const dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByRole("button", { name: "Restore all defaults" }));
    expect(calls.map((c) => c.op)).toEqual(["app.reset_shortcuts"]);
    expect(useShortcuts.getState().overrides).toEqual({});
  });

  it("filters commands and shows a conflict left by an old saved set", async () => {
    useShortcuts.getState().set({ "screenplay.find": ["Mod+K"] });
    const user = userEvent.setup();
    render(<ShortcutsSettings />);
    expect(screen.getByText(/Some shortcuts are used by two commands/)).toBeInTheDocument();
    expect(within(row("Find in script")).getByText("Also used by Search")).toBeInTheDocument();
    await user.type(screen.getByRole("textbox", { name: "Filter commands" }), "scene heading");
    expect(screen.getByRole("rowheader", { name: /Scene Heading element/ })).toBeInTheDocument();
    expect(screen.queryByRole("rowheader", { name: /^Search/ })).toBeNull();
  });

  it("uses no em dashes in what it shows", () => {
    render(<ShortcutsSettings />);
    expect(document.body.textContent ?? "").not.toMatch(new RegExp(String.fromCharCode(0x2014)));
  });
});
