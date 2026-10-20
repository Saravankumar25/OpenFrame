// Story page (Story > Story tab): a plain writing page saved in the project, with Export as its
// only extra. Typed text is stored after a short pause, when the page is left, and before an
// export; a save based on old text is retried on the newest revision without losing the text.

import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";

const calls: { op: string; args: Record<string, unknown> }[] = [];
let stored = { text: "In the beginning.", rev: 3, updatedAt: 1 };
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
        stored = { text: args.text as string, rev: stored.rev + 1, updatedAt: 2 };
        return { rev: stored.rev, updatedAt: 2 };
      }
      return {};
    }),
  };
});

import { StoryPage } from "./StoryPage";
import { flushPendingSaves } from "../../app/pendingSaves";

const wrap = (ui: ReactNode) =>
  render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>);
const saves = () => calls.filter((c) => c.op === "story.page.save");
const box = () => screen.findByLabelText("Story text") as Promise<HTMLTextAreaElement>;

beforeEach(async () => {
  await flushPendingSaves();
  calls.length = 0;
  stored = { text: "In the beginning.", rev: 3, updatedAt: 1 };
  saveFailures = [];
  readFails = false;
});

describe("Story page", () => {
  it("shows the heading, the saved text, a word count and nothing but Export beside it", async () => {
    wrap(<StoryPage />);
    const area = await box();
    expect(area.value).toBe("In the beginning.");
    expect(area).toHaveAttribute("placeholder", "Start writing your story here.");
    expect(area).toHaveAttribute("spellcheck", "true");
    expect(screen.getByRole("heading", { name: "Story" })).toBeInTheDocument();
    expect(screen.getByText("Write your story freely. This page is not linked to anything else.")).toBeInTheDocument();
    expect(screen.getByText("3 words")).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("Saved");
    expect(screen.getAllByRole("button").map((b) => b.textContent)).toEqual(["Export"]);
    expect(saves()).toHaveLength(0);
  });

  it("saves typed text after a short pause with the revision it was based on", async () => {
    wrap(<StoryPage />);
    const area = await box();
    fireEvent.change(area, { target: { value: "In the beginning.\n\nThen a bus." } });
    expect(screen.getByRole("status")).toHaveTextContent("Saving…");
    expect(saves()).toHaveLength(0);
    await waitFor(() => expect(saves()).toHaveLength(1));
    expect(saves()[0].args).toEqual({ text: "In the beginning.\n\nThen a bus.", expectedRev: 3 });
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("Saved"));
    expect(screen.getByText("6 words")).toBeInTheDocument();
    fireEvent.change(area, { target: { value: "Second pass." } });
    await waitFor(() => expect(saves()).toHaveLength(2));
    expect(saves()[1].args).toEqual({ text: "Second pass.", expectedRev: 4 });
  });

  it("stores the text when the box loses focus, when the page closes and when the app asks", async () => {
    const first = wrap(<StoryPage />);
    const area = await box();
    fireEvent.change(area, { target: { value: "Blur me" } });
    fireEvent.blur(area);
    await waitFor(() => expect(saves()).toHaveLength(1));

    fireEvent.change(area, { target: { value: "Leave me" } });
    first.unmount();
    await waitFor(() => expect(saves()).toHaveLength(2));
    expect(saves()[1].args.text).toBe("Leave me");

    // Window or project close: the shared registry flushes whatever is still waiting.
    wrap(<StoryPage />);
    const again = await box();
    expect(again.value).toBe("Leave me");
    fireEvent.change(again, { target: { value: "Close me" } });
    expect(await act(() => flushPendingSaves())).toBe(true);
    expect(saves().at(-1)?.args.text).toBe("Close me");
  });

  it("reopens with the newest text even when the last save is still running", async () => {
    const first = wrap(<StoryPage />);
    const area = await box();
    fireEvent.change(area, { target: { value: "Typed just before leaving" } });
    first.unmount();
    wrap(<StoryPage />);
    expect((await box()).value).toBe("Typed just before leaving");
  });

  it("keeps the writer's text and retries on the newest revision after a conflict", async () => {
    saveFailures = ["conflict"];
    wrap(<StoryPage />);
    fireEvent.change(await box(), { target: { value: "Mine" } });
    await waitFor(() => expect(saves()).toHaveLength(2));
    expect(saves()[1].args).toEqual({ text: "Mine", expectedRev: 4 });
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("Saved"));
    expect(stored.text).toBe("Mine");
  });

  it("says when a save failed and stores the text on the next try", async () => {
    saveFailures = ["disk"];
    wrap(<StoryPage />);
    const area = await box();
    fireEvent.change(area, { target: { value: "Keep this safe" } });
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("Not saved yet"));
    expect(area.value).toBe("Keep this safe");
    expect(await act(() => flushPendingSaves())).toBe(true);
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("Saved"));
    expect(stored.text).toBe("Keep this safe");
  });

  it("offers Plain text, PDF and Word, and stores the text before the dialog opens", async () => {
    wrap(<StoryPage />);
    fireEvent.change(await box(), { target: { value: "Export me" } });
    fireEvent.click(screen.getByRole("button", { name: "Export" }));
    expect(await screen.findByText("Export Story")).toBeInTheDocument();
    expect(saves().map((s) => s.args.text)).toEqual(["Export me"]);
    for (const label of ["Plain text (.txt)", "PDF", "Word (.docx)"]) {
      expect(screen.getByText(label)).toBeInTheDocument();
    }
    expect(await screen.findByText(/Night Bus - Story - \d{4}-\d{2}-\d{2}\.txt/)).toBeInTheDocument();
  });

  it("does not open the page for writing when it cannot be read", async () => {
    readFails = true;
    wrap(<StoryPage />);
    expect(await screen.findByText("The Story page could not be opened.")).toBeInTheDocument();
    expect(screen.queryByLabelText("Story text")).toBeNull();
  });
});
