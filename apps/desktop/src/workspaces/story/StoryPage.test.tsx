// Story page (Story > Story tab): a writing page on an A4 sheet, saved in the project with its
// formatting (font size, bold, italic, underline, highlight), zoomable, with Export beside it.
// Typed text is stored after a short pause, when the page is left, and before an export; a save
// based on old text is retried on the newest revision without losing the text.

import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { TextSelection } from "prosemirror-state";
import type { EditorView } from "prosemirror-view";

const calls: { op: string; args: Record<string, unknown> }[] = [];
let stored: { text: string; format: string | null; rev: number; updatedAt: number } = { text: "In the beginning.", format: null, rev: 3, updatedAt: 1 };
let saveFailures: ("conflict" | "disk")[] = [];
let readFails = false;

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(async () => null), save: vi.fn(async () => null) }));
vi.mock("../../app/toast", () => ({ toast: { success: vi.fn(), error: vi.fn(), info: vi.fn(), undoable: vi.fn() } }));

vi.mock("../../ipc/client", async () => {
  class OpError extends Error {
    code: string;
    retryable = false;
    constructor(code: string, message: string) {
      super(message);
      this.code = code;
    }
    is(p: string) {
      return this.code === p || this.code.startsWith(p + ".");
    }
  }
  return {
    OpError,
    inTauri: () => false,
    onAppEvent: () => Promise.resolve(() => undefined),
    revealLocation: vi.fn(async () => undefined),
    call: vi.fn(async (op: string, args: Record<string, unknown> = {}) => {
      calls.push({ op, args });
      if (op === "project.current") return { title: "Night Bus" };
      if (op === "story.page.get") {
        if (readFails) throw new OpError("storage.read_failed", "The page could not be read.");
        return { ...stored };
      }
      if (op === "story.page.save") {
        const fail = saveFailures.shift();
        if (fail === "conflict") {
          stored = { ...stored, rev: stored.rev + 1 };
          throw new OpError("conflict.stale", "This Story page was changed by someone else.");
        }
        if (fail === "disk") throw new OpError("storage.write_failed", "OpenFrame could not save this change yet.");
        if (args.expectedRev !== stored.rev) throw new OpError("conflict.stale", "stale");
        stored = { text: args.text as string, format: (args.format as string | null) ?? null, rev: stored.rev + 1, updatedAt: 2 };
        return { rev: stored.rev, updatedAt: 2 };
      }
      return {};
    }),
  };
});

import { StoryPage, storyEditors } from "./StoryPage";
import { contentOf, docFrom } from "./storyDoc";
import { flushPendingSaves } from "../../app/pendingSaves";

const wrap = (ui: ReactNode) =>
  render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>);
const saves = () => calls.filter((c) => c.op === "story.page.save");
const editor = async (): Promise<EditorView> => {
  const el = await screen.findByLabelText("Story text");
  const v = storyEditors.get(el);
  if (!v) throw new Error("no editor");
  return v;
};
const textOf = (v: EditorView) => contentOf(v.state.doc).text;
/** Replace the whole page with plain text, as typing would. */
const type = (v: EditorView, text: string) =>
  act(() => {
    v.dispatch(v.state.tr.replaceWith(0, v.state.doc.content.size, docFrom(text, null).content));
  });
/** Select from `from` to `to` (document positions). */
const select = (v: EditorView, from: number, to: number) =>
  act(() => {
    v.dispatch(v.state.tr.setSelection(TextSelection.create(v.state.doc, from, to)));
  });

// jsdom has no layout: ProseMirror asks ranges for their rectangles when it scrolls or focuses.
const noRects = () => Object.assign([], { item: () => null }) as unknown as DOMRectList;
Range.prototype.getClientRects = noRects;
Range.prototype.getBoundingClientRect = () => new DOMRect();
Element.prototype.getClientRects ??= noRects;

beforeEach(async () => {
  await flushPendingSaves();
  calls.length = 0;
  stored = { text: "In the beginning.", format: null, rev: 3, updatedAt: 1 };
  saveFailures = [];
  readFails = false;
  window.localStorage.removeItem("openframe.story.zoom");
});

describe("Story page", () => {
  it("shows the heading, the saved text on an A4 sheet, the toolbar, a word count and Export", async () => {
    wrap(<StoryPage />);
    const v = await editor();
    expect(textOf(v)).toBe("In the beginning.");
    expect(v.dom).toHaveAttribute("data-placeholder", "Start writing your story here.");
    expect(v.dom).toHaveAttribute("spellcheck", "true");
    expect(v.dom.closest(".story-a4")).not.toBeNull();
    expect(screen.getByRole("heading", { name: "Story" })).toBeInTheDocument();
    expect(screen.getByText("Write your story freely. This page is not linked to anything else.")).toBeInTheDocument();
    expect(screen.getByText("3 words")).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("Saved");
    expect(screen.getByRole("combobox", { name: "Font size" })).toHaveValue("12");
    for (const name of ["Bold", "Italic", "Underline", "Highlight"]) {
      expect(screen.getByRole("button", { name })).toHaveAttribute("aria-pressed", "false");
    }
    expect(screen.getByRole("button", { name: "Export" })).toBeInTheDocument();
    expect(screen.getByRole("group", { name: "Page zoom" })).toHaveTextContent("100%");
    expect(saves()).toHaveLength(0);
  });

  it("saves typed text after a short pause with the revision it was based on", async () => {
    wrap(<StoryPage />);
    const v = await editor();
    await type(v, "In the beginning.\n\nThen a bus.");
    expect(screen.getByRole("status")).toHaveTextContent("Saving…");
    expect(saves()).toHaveLength(0);
    await waitFor(() => expect(saves()).toHaveLength(1));
    expect(saves()[0].args).toEqual({ text: "In the beginning.\n\nThen a bus.", format: null, expectedRev: 3 });
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("Saved"));
    expect(screen.getByText("6 words")).toBeInTheDocument();
    await type(v, "Second pass.");
    await waitFor(() => expect(saves()).toHaveLength(2));
    expect(saves()[1].args).toEqual({ text: "Second pass.", format: null, expectedRev: 4 });
  });

  it("makes selected text bold, italic, underlined, highlighted and bigger, and saves the formatting", async () => {
    wrap(<StoryPage />);
    const v = await editor();
    // "In the" is positions 1 to 7 of the first paragraph.
    await select(v, 1, 7);
    for (const name of ["Bold", "Italic", "Underline", "Highlight"]) {
      fireEvent.click(screen.getByRole("button", { name }));
      expect(screen.getByRole("button", { name })).toHaveAttribute("aria-pressed", "true");
    }
    fireEvent.change(screen.getByRole("combobox", { name: "Font size" }), { target: { value: "24" } });
    expect(screen.getByRole("combobox", { name: "Font size" })).toHaveValue("24");
    await waitFor(() => expect(saves().length).toBeGreaterThan(0));
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("Saved"));
    const last = saves().at(-1)!.args;
    expect(last.text).toBe("In the beginning.");
    expect(JSON.parse(last.format as string)).toEqual([[{ t: "In the", b: true, i: true, u: true, h: true, s: 24 }, { t: " beginning." }]]);
    expect(v.dom.querySelector("strong")).not.toBeNull();
    expect(v.dom.querySelector("em")).not.toBeNull();
    expect(v.dom.querySelector("u")).not.toBeNull();
    expect(v.dom.querySelector("mark")).not.toBeNull();
    // A second click takes the style off again.
    fireEvent.click(screen.getByRole("button", { name: "Bold" }));
    expect(screen.getByRole("button", { name: "Bold" })).toHaveAttribute("aria-pressed", "false");
  });

  it("shows a saved formatting and a mixed size as Mixed", async () => {
    stored = {
      text: "Big and small",
      format: JSON.stringify([[{ t: "Big", b: true, s: 30 }, { t: " and small" }]]),
      rev: 1,
      updatedAt: 1,
    };
    wrap(<StoryPage />);
    const v = await editor();
    expect(v.dom.querySelector("strong")?.textContent).toBe("Big");
    expect(v.dom.querySelector('span[style*="30pt"]')).not.toBeNull();
    await select(v, 1, 9);
    expect(screen.getByRole("combobox", { name: "Font size" })).toHaveValue("");
    expect(screen.getByRole("option", { name: "Mixed" })).toBeInTheDocument();
  });

  it("zooms the page with the buttons and with Ctrl and the mouse wheel, and remembers it", async () => {
    wrap(<StoryPage />);
    const v = await editor();
    const sheet = v.dom.closest(".story-a4") as HTMLElement;
    fireEvent.click(screen.getByRole("button", { name: "Zoom in" }));
    expect(screen.getByRole("group", { name: "Page zoom" })).toHaveTextContent("110%");
    expect(sheet.style.getPropertyValue("--story-zoom")).toBe("1.1");
    const desk = sheet.parentElement!;
    fireEvent.wheel(desk, { deltaY: -400, ctrlKey: true });
    expect(Number(window.localStorage.getItem("openframe.story.zoom"))).toBeGreaterThan(1.1);
    const zoomed = screen.getByRole("group", { name: "Page zoom" }).textContent;
    // A wheel without Ctrl scrolls and leaves the zoom alone.
    fireEvent.wheel(desk, { deltaY: 400 });
    expect(screen.getByRole("group", { name: "Page zoom" }).textContent).toBe(zoomed);
    fireEvent.click(screen.getByTitle("Reset the page zoom"));
    expect(screen.getByRole("group", { name: "Page zoom" })).toHaveTextContent("100%");
  });

  it("stores the text when the editor loses focus, when the page closes and when the app asks", async () => {
    const first = wrap(<StoryPage />);
    let v = await editor();
    await type(v, "Blur me");
    fireEvent.blur(v.dom);
    await waitFor(() => expect(saves()).toHaveLength(1));

    await type(v, "Leave me");
    first.unmount();
    await waitFor(() => expect(saves()).toHaveLength(2));
    expect(saves()[1].args.text).toBe("Leave me");

    // Window or project close: the shared registry flushes whatever is still waiting.
    wrap(<StoryPage />);
    v = await editor();
    expect(textOf(v)).toBe("Leave me");
    await type(v, "Close me");
    expect(await act(() => flushPendingSaves())).toBe(true);
    expect(saves().at(-1)?.args.text).toBe("Close me");
  });

  it("reopens with the newest text even when the last save is still running", async () => {
    const first = wrap(<StoryPage />);
    await type(await editor(), "Typed just before leaving");
    first.unmount();
    wrap(<StoryPage />);
    expect(textOf(await editor())).toBe("Typed just before leaving");
  });

  it("keeps the writer's text and retries on the newest revision after a conflict", async () => {
    saveFailures = ["conflict"];
    wrap(<StoryPage />);
    await type(await editor(), "Mine");
    await waitFor(() => expect(saves()).toHaveLength(2));
    expect(saves()[1].args).toEqual({ text: "Mine", format: null, expectedRev: 4 });
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("Saved"));
    expect(stored.text).toBe("Mine");
  });

  it("says when a save failed and stores the text on the next try", async () => {
    saveFailures = ["disk"];
    wrap(<StoryPage />);
    const v = await editor();
    await type(v, "Keep this safe");
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("Not saved yet"));
    expect(textOf(v)).toBe("Keep this safe");
    expect(await act(() => flushPendingSaves())).toBe(true);
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("Saved"));
    expect(stored.text).toBe("Keep this safe");
  });

  it("offers Plain text, PDF and Word, and stores the text before the dialog opens", async () => {
    wrap(<StoryPage />);
    await type(await editor(), "Export me");
    fireEvent.click(screen.getByRole("button", { name: "Export" }));
    expect(await screen.findByText("Export Story")).toBeInTheDocument();
    expect(saves().map((s) => s.args.text)).toEqual(["Export me"]);
    for (const label of ["Plain text (.txt)", "PDF", "Word (.docx)"]) {
      expect(screen.getByText(label)).toBeInTheDocument();
    }
    expect(await screen.findByText(/Night Bus - Story - \d{4}-\d{2}-\d{2}\.txt/)).toBeInTheDocument();
  });

  it("turns the sheet dark and back, and remembers the choice", async () => {
    window.localStorage.removeItem("openframe.story.darkPage");
    const first = wrap(<StoryPage />);
    const v = await editor();
    const sheet = v.dom.closest(".story-a4") as HTMLElement;
    const toggle = screen.getByRole("button", { name: "Dark page" });
    expect(toggle).toHaveAttribute("aria-pressed", "false");
    expect(sheet).not.toHaveAttribute("data-paper");
    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-pressed", "true");
    expect(sheet).toHaveAttribute("data-paper", "dark");
    first.unmount();
    wrap(<StoryPage />);
    expect((await editor()).dom.closest(".story-a4")).toHaveAttribute("data-paper", "dark");
    fireEvent.click(screen.getByRole("button", { name: "Dark page" }));
    expect(window.localStorage.getItem("openframe.story.darkPage")).toBe("0");
  });

  it("does not open the page for writing when it cannot be read", async () => {
    readFails = true;
    wrap(<StoryPage />);
    expect(await screen.findByText("The Story page could not be opened.")).toBeInTheDocument();
    expect(screen.queryByLabelText("Story text")).toBeNull();
  });
});
