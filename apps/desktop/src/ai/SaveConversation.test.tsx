import { beforeEach, describe, expect, it, vi } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithClient } from "../app/home/testUtils";
import { useToasts } from "../app/toast";
import type { AiStatusDto } from "../ipc/generated/AiStatusDto";
import type { AiExchange } from "../ipc/generated/AiExchange";
import Assistant from "./Conversation";

// Test fixtures only: the IPC boundary and the system Save dialog are replaced.
const calls: { op: string; args: Record<string, unknown> }[] = [];
let exchanges: AiExchange[] = [];
const saveMock = vi.fn(async (_o: unknown) => "C:\\Reviews\\openframe-ai-conversation.json" as string | null);

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(async () => null), save: (o: unknown) => saveMock(o) }));

vi.mock("../ipc/client", async () => {
  class OpError extends Error {
    code = "x";
  }
  return {
    OpError,
    inTauri: () => false,
    onAppEvent: () => Promise.resolve(() => undefined),
    call: vi.fn(async (op: string, args: Record<string, unknown> = {}) => {
      calls.push({ op, args });
      if (op === "ai.history") return { conversationId: exchanges.length ? "conv1" : null, exchanges };
      if (op === "ai.export_conversation") return { fileName: "openframe-ai-conversation.json", rows: exchanges.length };
      return null;
    }),
  };
});

const status: AiStatusDto = { available: true, mode: "AI Ready", bundleState: "NotChecked", problem: null, runtimeState: "Installed", runtimeMessage: null, localOnly: true };

function exchange(id: string, text: string): AiExchange {
  return {
    requestId: id, conversationId: "conv1", requestText: text, scopeLabel: "Using: Whole Project", operationClass: "Read", modelReference: "p", createdAt: 1,
    result: {
      id: `r${id}`, kind: "Answer", status: "Informational", content: "Ten scenes.", details: [], confidence: null, provenance: [], items: [], itemsAreAnswer: false,
      nav: null, changeSet: null, errorCode: null, steps: [], taskId: null, createdAt: 1,
    },
  };
}

describe("Save conversation for review", () => {
  beforeEach(() => {
    calls.length = 0;
    useToasts.setState({ items: [] });
    exchanges = [];
    saveMock.mockClear();
  });

  it("is off while there is no conversation", async () => {
    renderWithClient(<Assistant status={status} hasProject />);
    const save = await screen.findByRole("button", { name: "Save conversation for review" });
    expect(save).toBeDisabled();
    expect(saveMock).not.toHaveBeenCalled();
  });

  it("asks where to save and writes only after the person chooses", async () => {
    exchanges = [exchange("a", "How many scenes?"), exchange("b", "And the drafts?")];
    renderWithClient(<Assistant status={status} hasProject />);
    await screen.findByText("How many scenes?");
    // Showing the conversation writes nothing.
    expect(calls.some((c) => c.op === "ai.export_conversation")).toBe(false);

    await userEvent.click(screen.getByRole("button", { name: "Save conversation for review" }));
    await waitFor(() => expect(calls.some((c) => c.op === "ai.export_conversation")).toBe(true));
    expect(calls.find((c) => c.op === "ai.export_conversation")?.args).toEqual({
      conversationId: "conv1",
      path: "C:\\Reviews\\openframe-ai-conversation.json",
    });
    await waitFor(() => expect(useToasts.getState().items.map((t) => t.text)).toContain("Saved openframe-ai-conversation.json (2 questions)."));

    // Cancelling the Save dialog writes nothing.
    calls.length = 0;
    saveMock.mockResolvedValueOnce(null);
    await userEvent.click(screen.getByRole("button", { name: "Save conversation for review" }));
    await waitFor(() => expect(saveMock).toHaveBeenCalledTimes(2));
    expect(calls.some((c) => c.op === "ai.export_conversation")).toBe(false);
  });
});
