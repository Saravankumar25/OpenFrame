// While a request runs the panel shows what the assistant is doing (progress events of THIS
// conversation) with a Cancel button; a stopped request ends in an ordinary answer.

import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, fireEvent, screen, waitFor } from "@testing-library/react";
import { renderWithClient } from "../app/home/testUtils";
import type { AiStatusDto } from "../ipc/generated/AiStatusDto";
import type { AiExchange } from "../ipc/generated/AiExchange";
import type { AppEvent } from "../ipc/generated/AppEvent";
import Assistant, { ResultView, newConversationId } from "./Conversation";
import type { AiResultDto } from "../ipc/generated/AiResultDto";

const calls: { op: string; args: Record<string, unknown> }[] = [];
const handlers = new Set<(e: AppEvent) => void>();
let finishAsk: ((x: AiExchange) => void) | null = null;

vi.mock("../ipc/client", async () => {
  class OpError extends Error {
    code = "x";
    is() {
      return false;
    }
  }
  return {
    OpError,
    inTauri: () => false,
    onAppEvent: (h: (e: AppEvent) => void) => {
      handlers.add(h);
      return Promise.resolve(() => handlers.delete(h));
    },
    call: vi.fn(async (op: string, args: Record<string, unknown> = {}) => {
      calls.push({ op, args });
      switch (op) {
        case "ai.history":
          return { conversationId: null, exchanges: [] };
        case "ai.index_status":
          return "Current";
        case "ai.ask":
          return new Promise<AiExchange>((resolve) => {
            finishAsk = resolve;
          });
        case "ai.cancel":
          return { stopping: true };
        default:
          return null;
      }
    }),
  };
});

const status: AiStatusDto = {
  available: true,
  mode: "AI Ready",
  bundleState: "NotChecked",
  problem: null,
  runtimeState: "Ready",
  runtimeMessage: null,
  localOnly: true,
};

function emit(payload: Record<string, unknown>) {
  const ev = { type: "module", module: "ai", name: "progress", payload } as AppEvent;
  act(() => handlers.forEach((h) => h(ev)));
}

function result(over: Partial<AiResultDto>): AiResultDto {
  return {
    id: "r", kind: "Answer", status: "Informational", content: "", details: [], confidence: null, provenance: [], items: [], itemsAreAnswer: false,
    nav: null, changeSet: null, errorCode: null, steps: [], taskId: null, createdAt: 1, ...over,
  } as AiResultDto;
}

async function ask(text = "Walk me through the story") {
  renderWithClient(<Assistant status={status} hasProject />);
  const box = await screen.findByRole("textbox", { name: "Ask or command" });
  fireEvent.change(box, { target: { value: text } });
  fireEvent.keyDown(box, { key: "Enter" });
  await waitFor(() => expect(calls.some((c) => c.op === "ai.ask")).toBe(true));
  return calls.find((c) => c.op === "ai.ask")!.args.conversationId as string;
}

beforeEach(() => {
  calls.length = 0;
  handlers.clear();
  finishAsk = null;
});

describe("progress while a request runs", () => {
  it("names the conversation before the first answer, so Cancel can reach it", async () => {
    const id = await ask();
    expect(id).toMatch(/^[A-Za-z0-9-]{8,64}$/);
    expect(newConversationId()).toMatch(/^[A-Za-z0-9-]{8,64}$/);
  });

  it("shows the plain waiting line until a progress line arrives, then the latest label", async () => {
    const id = await ask();
    expect(await screen.findByText("Working on it on this computer…")).toBeTruthy();
    await waitFor(() => expect(handlers.size).toBeGreaterThan(0));
    emit({ conversationId: id, requestId: "r1", step: 1, total: 3, label: "Reading Scenes 5 and 6" });
    expect(await screen.findByText("Reading Scenes 5 and 6")).toBeTruthy();
    emit({ conversationId: id, requestId: "r1", step: 2, total: 3, label: "Writing the answer" });
    expect(await screen.findByText("Writing the answer")).toBeTruthy();
    expect(screen.queryByText("Reading Scenes 5 and 6")).toBeNull();
    // A polite live region, so a screen reader hears the change without being interrupted.
    expect(screen.getByText("Writing the answer").closest("[aria-live]")?.getAttribute("aria-live")).toBe("polite");
  });

  it("ignores progress lines of another conversation and of other events", async () => {
    const id = await ask();
    await waitFor(() => expect(handlers.size).toBeGreaterThan(0));
    emit({ conversationId: "someone-else", requestId: "r9", step: 1, total: 2, label: "Reading Scene 99" });
    act(() => handlers.forEach((h) => h({ type: "module", module: "other", name: "progress", payload: { conversationId: id, label: "Not mine" } } as AppEvent)));
    expect(screen.queryByText("Reading Scene 99")).toBeNull();
    expect(screen.queryByText("Not mine")).toBeNull();
    expect(screen.getByText("Working on it on this computer…")).toBeTruthy();
  });

  it("Cancel calls ai.cancel for this conversation once, says Stopping, and the answer then arrives as usual", async () => {
    const id = await ask();
    fireEvent.click(await screen.findByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(calls.filter((c) => c.op === "ai.cancel")).toHaveLength(1));
    expect(calls.find((c) => c.op === "ai.cancel")!.args).toEqual({ conversationId: id });
    expect(await screen.findByText("Stopping…")).toBeTruthy();
    expect((screen.getByRole("button", { name: "Cancel" }) as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(calls.filter((c) => c.op === "ai.cancel")).toHaveLength(1);
    await act(async () => {
      finishAsk?.({
        requestId: "1", conversationId: id, requestText: "Walk me through the story", scopeLabel: "Using: Whole Project", operationClass: "Read",
        modelReference: "p", createdAt: 1, result: result({ content: "Stopped. Nothing was changed." }),
      } as AiExchange);
    });
    await waitFor(() => expect(screen.queryByRole("button", { name: "Cancel" })).toBeNull());
  });
});

describe("answers", () => {
  it("show how much of the screenplay a whole-story answer used", () => {
    renderWithClient(<ResultView r={result({ content: "The story opens in a kitchen.", coverage: { used: 36, total: 36 } })} onChanged={() => undefined} />);
    expect(screen.getByText("Used 36 of 36 scenes")).toBeTruthy();
  });

  it("say nothing about coverage when the answer has none", () => {
    renderWithClient(<ResultView r={result({ content: "There are 10 scenes." })} onChanged={() => undefined} />);
    expect(screen.queryByText(/Used \d+ of/)).toBeNull();
  });

  it("draw lists and the Idea: label from the answer text", () => {
    const { container } = renderWithClient(
      <ResultView r={result({ content: "Two options.\n\n- Cut the kite\n- Keep the kite\n\nIdea: end on the station clock." })} onChanged={() => undefined} />,
    );
    expect(container.querySelectorAll("ul li")).toHaveLength(2);
    expect(container.querySelector("b")?.textContent).toBe("Idea:");
  });
});
