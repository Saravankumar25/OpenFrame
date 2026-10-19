// Find and replace scope (SCRIPT-004): "This scene" or "Whole script", in the editor highlights
// and in Replace All (one undoable step in Rust).

import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { computeMatches } from "./editor/find";
import { docFromSnapshot, type Snapshot } from "./editor/model";
import { FindBar } from "./misc";

const calls: { op: string; args: Record<string, unknown> }[] = [];

vi.mock("../../ipc/client", () => ({
  OpError: class extends Error {},
  inTauri: () => false,
  onAppEvent: () => Promise.resolve(() => undefined),
  call: vi.fn(async (op: string, args: Record<string, unknown>) => {
    calls.push({ op, args });
    return { count: 2, seq: 1 };
  }),
}));

beforeEach(() => {
  calls.length = 0;
});

const script: Snapshot = [
  { id: "s1", heading: "INT. DEPOT - NIGHT", elements: [{ id: "a", type: "action", text: "The bus waits. The bus leaves." }] },
  { id: "s2", heading: "EXT. ROAD - NIGHT", elements: [{ id: "b", type: "action", text: "Another bus." }, { id: "n", type: "note", text: "bus model?" }] },
];

describe("computeMatches", () => {
  const doc = docFromSnapshot(script);
  it("whole script finds every match; notes only while shown", () => {
    expect(computeMatches(doc, { query: "bus", matchCase: false, wholeWord: true }, true)).toHaveLength(4);
    expect(computeMatches(doc, { query: "bus", matchCase: false, wholeWord: true }, false)).toHaveLength(3);
  });
  it("this scene limits matches to that scene, heading included", () => {
    expect(computeMatches(doc, { query: "bus", matchCase: false, wholeWord: true, sceneId: "s1" }, true)).toHaveLength(2);
    expect(computeMatches(doc, { query: "NIGHT", matchCase: true, wholeWord: true, sceneId: "s2" }, true)).toHaveLength(1);
    expect(computeMatches(doc, { query: "bus", matchCase: false, wholeWord: true, sceneId: "missing" }, true)).toHaveLength(0);
  });
});

describe("Find bar scope", () => {
  function setup(currentScene: { id: string; number: number } | null = { id: "s2", number: 2 }) {
    const onSearch = vi.fn();
    render(
      <FindBar
        draftId="d1"
        canReplace
        showNotes={false}
        result={{ index: 0, count: 2 }}
        currentScene={currentScene}
        onSearch={onSearch}
        onStep={vi.fn()}
        onReplace={vi.fn()}
        onBeforeReplaceAll={() => Promise.resolve()}
        onClose={vi.fn()}
      />,
    );
    return onSearch;
  }

  it("searches the whole script by default", async () => {
    const user = userEvent.setup();
    const onSearch = setup();
    expect(screen.getByRole("radio", { name: "Whole script" })).toHaveAttribute("aria-checked", "true");
    await user.type(screen.getByRole("textbox", { name: "Find" }), "bus");
    expect(onSearch).toHaveBeenLastCalledWith({ query: "bus", matchCase: false, wholeWord: false, sceneId: null });
  });

  it("This scene limits the search and Replace All to the scene the writer is in", async () => {
    const user = userEvent.setup();
    const onSearch = setup();
    await user.type(screen.getByRole("textbox", { name: "Find" }), "bus");
    await user.click(screen.getByRole("radio", { name: "This scene" }));
    expect(screen.getByRole("radio", { name: "This scene (2)" })).toHaveAttribute("aria-checked", "true");
    expect(onSearch).toHaveBeenLastCalledWith({ query: "bus", matchCase: false, wholeWord: false, sceneId: "s2" });
    expect(screen.getByRole("textbox", { name: "Find" })).toHaveAttribute("placeholder", "Find in Scene 2");
    await user.type(screen.getByRole("textbox", { name: "Replace with" }), "van");
    await user.click(screen.getByRole("button", { name: "Replace All" }));
    expect(calls).toEqual([
      { op: "screenplay.replace_all", args: { draftId: "d1", query: "bus", replacement: "van", matchCase: false, wholeWord: false, includeNotes: false, sceneId: "s2" } },
    ]);
    // Back to the whole script: Replace All no longer names a scene.
    await user.click(screen.getByRole("radio", { name: "Whole script" }));
    await user.click(screen.getByRole("button", { name: "Replace All" }));
    expect(calls[1].args).not.toHaveProperty("sceneId");
  });

  it("saves pending typing before Replace All runs, and keeps the chosen scene", async () => {
    const user = userEvent.setup();
    const order: string[] = [];
    const onSearch = vi.fn();
    const { rerender } = render(
      <FindBar
        draftId="d1"
        canReplace
        showNotes={false}
        result={{ index: 0, count: 1 }}
        currentScene={{ id: "s2", number: 2 }}
        onSearch={onSearch}
        onStep={vi.fn()}
        onReplace={vi.fn()}
        onBeforeReplaceAll={async () => {
          order.push("flush");
        }}
        onClose={vi.fn()}
      />,
    );
    await user.type(screen.getByRole("textbox", { name: "Find" }), "bus");
    await user.click(screen.getByRole("radio", { name: "This scene" }));
    // The writer clicks into another scene while the bar is open: the chosen scene stays.
    rerender(
      <FindBar
        draftId="d1"
        canReplace
        showNotes={false}
        result={{ index: 0, count: 1 }}
        currentScene={{ id: "s1", number: 1 }}
        onSearch={onSearch}
        onStep={vi.fn()}
        onReplace={vi.fn()}
        onBeforeReplaceAll={async () => {
          order.push("flush");
        }}
        onClose={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Replace All" }));
    order.push(...calls.map((c) => c.op));
    expect(order).toEqual(["flush", "screenplay.replace_all"]);
    expect(calls[0].args.sceneId).toBe("s2");
  });

  it("explains instead of guessing when no scene is chosen yet", async () => {
    const user = userEvent.setup();
    setup(null);
    await user.click(screen.getByRole("radio", { name: "This scene" }));
    expect(screen.getByRole("radio", { name: "Whole script" })).toHaveAttribute("aria-checked", "true");
  });
});
