import { beforeEach, describe, expect, it, vi } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithClient } from "../../app/home/testUtils";
import { AssistantMemoryCard } from "./AssistantMemory";

// Test fixtures only: the IPC boundary is replaced by an in-memory responder.
const calls: { op: string; args: Record<string, unknown> }[] = [];
let entries: Record<string, unknown>[] = [];

vi.mock("../../ipc/client", async () => {
  class OpError extends Error {
    code = "x";
  }
  return {
    OpError,
    inTauri: () => false,
    onAppEvent: () => Promise.resolve(() => undefined),
    call: vi.fn(async (op: string, args: Record<string, unknown> = {}) => {
      calls.push({ op, args });
      if (op === "ai.memory_list") return { entries };
      if (op === "ai.memory_delete") {
        entries = entries.filter((e) => e.id !== args.id);
        return { deleted: true };
      }
      return null;
    }),
  };
});

const entry = (id: string, text: string, kind: "fact" | "preference") => ({
  id,
  text,
  kind,
  entityRefs: [],
  sourceRequestId: "r1",
  createdAt: Date.now(),
});

describe("Assistant memory card", () => {
  beforeEach(() => {
    calls.length = 0;
    entries = [];
  });

  it("explains what is kept when nothing is remembered yet", async () => {
    renderWithClient(<AssistantMemoryCard />);
    expect(await screen.findByText(/Nothing is remembered yet/)).toBeInTheDocument();
    expect(screen.getByText(/Saved on this computer, in this project, for you only/)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Delete/ })).toBeNull();
  });

  it("lists each entry with its kind and a labelled Delete button", async () => {
    entries = [entry("m1", "I'm the assistant director", "fact"), entry("m2", "Always answer in short sentences", "preference")];
    renderWithClient(<AssistantMemoryCard />);
    expect(await screen.findByText("I'm the assistant director")).toBeInTheDocument();
    expect(screen.getByText("Always answer in short sentences")).toBeInTheDocument();
    expect(screen.getByText(/^Preference, saved/)).toBeInTheDocument();
    expect(screen.getByRole("list", { name: "Remembered entries" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Delete: I'm the assistant director" })).toBeInTheDocument();
  });

  it("Delete forgets that entry only and the list updates", async () => {
    entries = [entry("m1", "I'm the assistant director", "fact"), entry("m2", "Call me Saravan", "preference")];
    renderWithClient(<AssistantMemoryCard />);
    await userEvent.click(await screen.findByRole("button", { name: "Delete: Call me Saravan" }));
    await waitFor(() => expect(calls.find((c) => c.op === "ai.memory_delete")?.args).toEqual({ id: "m2" }));
    await waitFor(() => expect(screen.queryByText("Call me Saravan")).toBeNull());
    expect(screen.getByText("I'm the assistant director")).toBeInTheDocument();
  });
});
