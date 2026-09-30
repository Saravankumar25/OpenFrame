import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { AiStatusDto } from "../ipc/generated/AiStatusDto";
import type { AiExchange } from "../ipc/generated/AiExchange";
import type { AiChangeSetDto } from "../ipc/generated/AiChangeSetDto";
import type { AiProfileDto } from "../ipc/generated/AiProfileDto";
import AiPanel from "./AiPanel";
import { useNav } from "../app/stores";

// Test fixtures only: the IPC boundary is replaced by an in-memory responder.
const calls: { op: string; args: Record<string, unknown> }[] = [];
let status: AiStatusDto;
let exchanges: AiExchange[] = [];
let askReply: AiExchange | null = null;

const profile = (tier: string, id: string, recommended = false): AiProfileDto => ({
  profileId: id, tier, name: tier, sizeBytes: 2_497_280_256, sizeLabel: "2.3 GB", installed: false, active: false, recommended,
  suitable: true, likelySlow: false, note: null, downloadedBytes: 0, startFailed: false,
});

function changeSet(state: string, extra: Partial<AiChangeSetDto> = {}): AiChangeSetDto {
  return {
    id: "cs1", title: "Proposed Scene Card", summary: "", state, validationState: "Valid",
    rows: [{ label: "Description", value: "Ravi finds the ticket", tone: "normal" }, { label: "Place in", value: "Act One", tone: "normal" }],
    exclusions: [{ label: "Excluded: raw dialogue/action text", value: "2 mentions", tone: "excluded" }],
    affectedModules: ["Story"], targets: [], operationCount: 1, staleReason: null, errorMessage: null, createdAt: 1, approvedAt: null, appliedAt: null,
    appliedOperations: 0, ...extra,
  };
}

function exchange(id: string, text: string, result: Partial<AiExchange["result"]>): AiExchange {
  return {
    requestId: id, conversationId: "conv1", requestText: text, scopeLabel: "Using: Whole Project", operationClass: "Read", modelReference: "p",
    createdAt: 1,
    result: {
      id: `r${id}`, kind: "Answer", status: "Informational", content: "", details: [], confidence: null, provenance: [], items: [], nav: null,
      changeSet: null, errorCode: null, createdAt: 1, ...result,
    },
  };
}

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
    onAppEvent: () => Promise.resolve(() => undefined),
    call: vi.fn(async (op: string, args: Record<string, unknown> = {}) => {
      calls.push({ op, args });
      switch (op) {
        case "project.current":
          return { id: "p", title: "BLACK RAIN", projectType: "Feature Film" };
        case "ai.status":
          return status;
        case "ai.hardware":
          return {
            memoryBytes: 16 * 1024 ** 3, availableMemoryBytes: 8 * 1024 ** 3, processor: "CPU", processorThreads: 8, graphics: [], freeDiskBytes: 100 * 1024 ** 3,
            recommendation: {
              profileId: "rec", tier: "Recommended", name: "Recommended", sizeBytes: 2_497_280_256, downloadBytes: 2_516_445_059,
              requiredFreeBytes: 3_100_000_000, runsOn: "Processor", likelySlow: false, warnings: [], enoughDisk: true,
            },
            profiles: status.profiles,
          };
        case "ai.install":
          return { taskId: "t1" };
        case "ai.history":
          return { conversationId: exchanges.length ? "conv1" : null, exchanges };
        case "ai.ask":
          return askReply;
        case "ai.change_set.accept":
          return changeSet("Applied", { appliedOperations: 1 });
        default:
          return null;
      }
    }),
  };
});

function renderPanel() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <AiPanel onClose={() => undefined} />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  calls.length = 0;
  exchanges = [];
  askReply = null;
  useNav.getState().reset();
  status = {
    installed: false, mode: "Off", runtimeState: "NotInstalled", runtimeMessage: null, activeProfile: null,
    profiles: [profile("Lightweight", "light"), profile("Recommended", "rec", true), profile("High Quality", "hq")], install: null, localOnly: true,
  };
});

describe("AI Assistant panel", () => {
  it("works without AI installed and offers the one-click Download Offline AI flow", async () => {
    renderPanel();
    expect(await screen.findByText("AI is not available right now")).toBeTruthy();
    expect(screen.getByText(/Everything else in OpenFrame works normally/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Download Offline AI" }));
    expect(await screen.findByText(/OpenFrame checked this computer and recommends/)).toBeTruthy();
    // Simple names only — no model files, quantization codes or ports.
    expect(document.body.textContent).not.toMatch(/gguf|q4_k|127\.0\.0\.1/i);
    fireEvent.click(screen.getByRole("button", { name: /^Download Offline AI \(/ }));
    await waitFor(() => expect(calls.some((c) => c.op === "ai.install" && c.args.profileId === "rec")).toBe(true));
  });

  it("shows download progress with pause and cancel", async () => {
    status = { ...status, install: { taskId: "t1", profileId: "rec", phase: "downloadingModel", bytesDone: 1024 ** 3, bytesTotal: 2 * 1024 ** 3, message: "Downloading the AI model…", error: null, active: true } };
    renderPanel();
    expect(await screen.findByText("Downloading the AI model…")).toBeTruthy();
    expect(screen.getByText(/1.0 GB of 2.0 GB · 50%/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Pause" }));
    await waitFor(() => expect(calls.some((c) => c.op === "ai.cancel_install" && c.args.discard === false)).toBe(true));
  });

  it("previews a Change Set and applies it only on explicit Apply", async () => {
    status = { ...status, installed: true, mode: "Local model", runtimeState: "Ready" };
    exchanges = [
      exchange("1", "How many scenes?", { content: "Draft 1 contains 3 scenes.", confidence: "Exact", provenance: [{ kind: "Draft", label: "Draft 1" }] }),
      exchange("2", "Create a scene card", { kind: "Proposal", status: "Pending Approval", content: "I prepared a suggestion. Nothing has been added yet.", changeSet: changeSet("Pending") }),
    ];
    renderPanel();
    expect(await screen.findByText("Draft 1 contains 3 scenes.")).toBeTruthy();
    expect(screen.getByText("Exact · from project data")).toBeTruthy();
    expect(screen.getByText("Based on: Draft: Draft 1")).toBeTruthy();
    expect(screen.getByText("Ravi finds the ticket")).toBeTruthy();
    expect(screen.getByText("Excluded: raw dialogue/action text")).toBeTruthy();
    expect(calls.some((c) => c.op === "ai.change_set.accept")).toBe(false);
    fireEvent.click(screen.getByRole("button", { name: "Apply" }));
    await waitFor(() => expect(calls.some((c) => c.op === "ai.change_set.accept" && c.args.id === "cs1")).toBe(true));
  });

  it("never offers Apply on a stale proposal", async () => {
    status = { ...status, installed: true, mode: "Local model", runtimeState: "Ready" };
    exchanges = [
      exchange("1", "Apply the rename", {
        kind: "Proposal", status: "Stale", content: "I prepared a rename.",
        changeSet: changeSet("Stale", { staleReason: "The project changed after this proposal was prepared (1 item edited). It has not been applied." }),
      }),
    ];
    renderPanel();
    expect(await screen.findByText("This proposal is out of date")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Apply" })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Re-check & Review" }));
    await waitFor(() => expect(calls.some((c) => c.op === "ai.change_set.recheck")).toBe(true));
  });

  it("sends the request with the chosen scope and runs navigation directly", async () => {
    status = { ...status, installed: true, mode: "Local model", runtimeState: "Ready" };
    askReply = exchange("3", "Open scene 2", { kind: "Navigate", content: "Opening Scene 2 in the Screenplay.", nav: { workspace: "screenplay", params: { sceneId: "s2" } } });
    renderPanel();
    const box = await screen.findByRole("textbox", { name: "Ask or command" });
    fireEvent.change(box, { target: { value: "Open scene 2" } });
    fireEvent.keyDown(box, { key: "Enter" });
    await waitFor(() => expect(useNav.getState().route).toEqual({ workspace: "screenplay", params: { sceneId: "s2" } }));
    const ask = calls.find((c) => c.op === "ai.ask")!;
    expect(ask.args.text).toBe("Open scene 2");
    expect((ask.args.scope as { kind: string }).kind).toBe("WholeProject");
  });
});
