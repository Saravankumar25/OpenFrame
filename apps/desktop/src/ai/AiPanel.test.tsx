import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { AiStatusDto } from "../ipc/generated/AiStatusDto";
import type { AiExchange } from "../ipc/generated/AiExchange";
import type { AiChangeSetDto } from "../ipc/generated/AiChangeSetDto";
import AiPanel from "./AiPanel";
import { useNav } from "../app/stores";

// Test fixtures only: the IPC boundary is replaced by an in-memory responder.
const calls: { op: string; args: Record<string, unknown> }[] = [];
let status: AiStatusDto;
let exchanges: AiExchange[] = [];
let askReply: AiExchange | null = null;

function changeSet(state: string, extra: Partial<AiChangeSetDto> = {}): AiChangeSetDto {
  return {
    id: "cs1", title: "Proposed Scene Card", summary: "I prepared a new Scene Card. Nothing has been added yet.", state, validationState: "Valid",
    rows: [{ label: "Description", value: "Ravi finds the ticket", tone: "normal" }, { label: "Place in", value: "Act One", tone: "normal" }],
    exclusions: [{ label: "Excluded: raw dialogue/action text", value: "2 mentions", tone: "excluded" }],
    affectedModules: ["Story"], targets: [], operationCount: 1, staleReason: null, errorMessage: null, createdAt: 1, approvedAt: null, appliedAt: null,
    appliedOperations: 0, partCount: 1, requiresConfirmation: false, ...extra,
  };
}

function exchange(id: string, text: string, result: Partial<AiExchange["result"]>): AiExchange {
  return {
    requestId: id, conversationId: "conv1", requestText: text, scopeLabel: "Using: Whole Project", operationClass: "Read", modelReference: "p",
    createdAt: 1,
    result: {
      id: `r${id}`, kind: "Answer", status: "Informational", content: "", details: [], confidence: null, provenance: [], items: [], nav: null,
      changeSet: null, errorCode: null, steps: [], taskId: null, createdAt: 1, ...result,
    },
  };
}

let indexState: unknown = "Current";

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
        case "ai.setup_info":
          return {
            supported: true, message: null, downloadBytes: 861_908_195, totalBytes: 861_908_195, downloadedBytes: 0, requiredFreeBytes: 1_417_000_000,
            freeDiskBytes: 100 * 1024 ** 3, enoughDisk: true, runsOn: "Processor", likelySlow: false, warnings: [], upToDate: false,
          };
        case "ai.install":
          return { taskId: "t1" };
        case "ai.history":
          return { conversationId: exchanges.length ? "conv1" : null, exchanges };
        case "ai.ask":
          return askReply;
        case "ai.change_set.accept":
          return changeSet("Applied", { appliedOperations: 1 });
        case "ai.change_set.reject":
          return changeSet("Rejected");
        case "ai.index_status":
          return indexState;
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
  indexState = "Current";
  useNav.getState().reset();
  status = { installed: false, mode: "Off", runtimeState: "NotInstalled", runtimeMessage: null, install: null, localOnly: true, updateAvailable: false, installedBytes: 0 };
});

describe("AI Assistant panel", () => {
  it("works without AI installed and offers the one-click Download Offline AI flow", async () => {
    renderPanel();
    expect(await screen.findByText("Offline AI is not installed.")).toBeTruthy();
    expect(await screen.findByText("822 MB")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Download Offline AI" }));
    await waitFor(() => expect(calls.some((c) => c.op === "ai.install")).toBe(true));
    expect(calls.find((c) => c.op === "ai.install")!.args).toEqual({});
  });

  it("shows download progress with pause and cancel", async () => {
    status = { ...status, install: { taskId: "t1", phase: "downloading", bytesDone: 1024 ** 3, bytesTotal: 2 * 1024 ** 3, message: "Downloading…", error: null, active: true } };
    renderPanel();
    expect(await screen.findByText("Downloading…")).toBeTruthy();
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
    expect(screen.getByText("Draft: Draft 1")).toBeTruthy();
    // Proposed Changes: summary, affected areas, exact preview, then an explicit choice.
    const card = screen.getByRole("region", { name: "Proposed Changes" });
    expect(card.textContent).toContain("I prepared a new Scene Card. Nothing has been added yet.");
    expect(card.textContent).toContain("Affected areas");
    expect(screen.getByText("Ravi finds the ticket")).toBeTruthy();
    expect(screen.getByText("Excluded: raw dialogue/action text")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Reject" })).toBeTruthy();
    expect(calls.some((c) => c.op === "ai.change_set.accept")).toBe(false);
    fireEvent.click(screen.getByRole("button", { name: "Apply Changes" }));
    await waitFor(() => expect(calls.some((c) => c.op === "ai.change_set.accept" && c.args.id === "cs1")).toBe(true));
    expect(calls.find((c) => c.op === "ai.change_set.accept")!.args.confirmDestructive).toBeUndefined();
  });

  it("rejecting leaves the project unchanged and never applies", async () => {
    status = { ...status, installed: true, mode: "Local model", runtimeState: "Ready" };
    exchanges = [exchange("2", "Create a scene card", { kind: "Proposal", status: "Pending Approval", content: "Prepared.", changeSet: changeSet("Pending") })];
    renderPanel();
    fireEvent.click(await screen.findByRole("button", { name: "Reject" }));
    await waitFor(() => expect(calls.some((c) => c.op === "ai.change_set.reject" && c.args.id === "cs1")).toBe(true));
    expect(calls.some((c) => c.op === "ai.change_set.accept")).toBe(false);
  });

  it("asks for the usual confirmation before applying destructive changes", async () => {
    status = { ...status, installed: true, mode: "Local model", runtimeState: "Ready" };
    exchanges = [
      exchange("2", "Tidy up", {
        kind: "Proposal", status: "Pending Approval", content: "Prepared.",
        changeSet: changeSet("Pending", { requiresConfirmation: true, rows: [{ label: "Moves to Recently Deleted", value: "2 items", tone: "destructive" }] }),
      }),
    ];
    renderPanel();
    fireEvent.click(await screen.findByRole("button", { name: "Apply Changes" }));
    expect(await screen.findByText("Apply changes that delete items?")).toBeTruthy();
    expect(calls.some((c) => c.op === "ai.change_set.accept")).toBe(false);
    const buttons = screen.getAllByRole("button", { name: "Apply Changes" });
    fireEvent.click(buttons[buttons.length - 1]);
    await waitFor(() => expect(calls.some((c) => c.op === "ai.change_set.accept" && c.args.confirmDestructive === true)).toBe(true));
  });

  it("shows one review card for a multi-step change, with its parts", async () => {
    status = { ...status, installed: true, mode: "Local model", runtimeState: "Ready" };
    exchanges = [
      exchange("4", "Plan the scout", {
        kind: "Proposal", status: "Pending Approval", content: "I prepared 2 changes for you to review together.",
        steps: [
          { index: 1, tool: "propose_task", label: "Propose task", status: "Succeeded" },
          { index: 2, tool: "propose_scene_card", label: "Propose scene card", status: "Succeeded" },
        ],
        changeSet: changeSet("Pending", {
          title: "2 changes: New task · New Scene Card", partCount: 2, operationCount: 2, affectedModules: ["Notes & Tasks", "Story"],
          rows: [
            { label: "New task", value: "Change 1 of 2", tone: "section" },
            { label: "Title", value: "Scout the station", tone: "normal" },
            { label: "New Scene Card", value: "Change 2 of 2", tone: "section" },
          ],
        }),
      }),
    ];
    renderPanel();
    expect(await screen.findAllByRole("region", { name: "Proposed Changes" })).toHaveLength(1);
    expect(screen.getByText("Change 2 of 2")).toBeTruthy();
    expect(screen.getByText("Notes & Tasks, Story")).toBeTruthy();
    expect(screen.getByText(/2 changes in 2 parts, applied together as one step you can undo/)).toBeTruthy();
    expect(screen.getByText("2 steps: Propose task · Propose scene card")).toBeTruthy();
  });

  it("opens a source from its provenance chip", async () => {
    status = { ...status, installed: true, mode: "Local model", runtimeState: "Ready" };
    exchanges = [
      exchange("5", "Who is Ravi?", {
        content: "Ravi is the lead.", confidence: "Inferred",
        provenance: [
          { kind: "Scope", label: "Whole Project" },
          { kind: "Character", label: "Ravi", nav: { workspace: "story", params: { characterId: "c1" } } },
        ],
      }),
    ];
    renderPanel();
    fireEvent.click(await screen.findByRole("button", { name: "Character: Ravi" }));
    await waitFor(() => expect(useNav.getState().route).toEqual({ workspace: "story", params: { characterId: "c1" } }));
    expect(screen.queryByRole("button", { name: "Whole Project" })).toBeNull();
    // Ordinary workflow UI never shows model or retrieval internals.
    expect(document.body.textContent).not.toMatch(/gemma|gguf|q4_k|embedding|sqlite-vec|rrf|context graph/i);
  });

  it("explains when project context is still being prepared", async () => {
    status = { ...status, installed: true, mode: "Local model", runtimeState: "Ready" };
    indexState = "Rebuilding";
    renderPanel();
    expect(await screen.findByText("Preparing project context…")).toBeTruthy();
    expect(screen.getByText(/AI can still answer some questions while project context is being prepared\./)).toBeTruthy();
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
    expect(await screen.findByText("Proposed Changes — out of date")).toBeTruthy();
    expect(screen.getByText(/It has not been applied\./)).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Apply Changes" })).toBeNull();
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
