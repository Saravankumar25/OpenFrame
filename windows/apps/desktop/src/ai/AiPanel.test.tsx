import { beforeEach, describe, expect, it, vi } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { AiStatusDto } from "../ipc/generated/AiStatusDto";
import type { AiExchange } from "../ipc/generated/AiExchange";
import type { AiChangeSetDto } from "../ipc/generated/AiChangeSetDto";
import type { AiSectionTreeDto } from "../ipc/generated/AiSectionTreeDto";
import AiPanel from "./AiPanel";
import { useAiPicks } from "./sections";
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
    requestId: id, conversationId: "conv1", requestText: text, scopeLabel: "Using: Screenplay (Draft 1, all scenes)", operationClass: "Read", modelReference: "p",
    createdAt: 1,
    result: {
      id: `r${id}`, kind: "Answer", status: "Informational", content: "", details: [], confidence: null, provenance: [], items: [], itemsAreAnswer: false, nav: null,
      changeSet: null, errorCode: null, steps: [], taskId: null, createdAt: 1, ...result,
    },
  };
}

let indexState: unknown = "Current";

const leaf = (id: string, label: string, path: string, picker = "none") => ({ id, label, pathLabel: path, picker, note: null });
const oneScreenplay: AiSectionTreeDto = {
  branches: [
    { id: "screenplay", label: "Screenplay", leaves: [leaf("screenplay/scenes", "Scenes", "Screenplay > Scenes", "screenplay")] },
    {
      id: "production",
      label: "Production",
      leaves: [
        leaf("production/cast-crew", "Cast & Crew", "Production > Cast & Crew"),
        leaf("production/budget", "Budget", "Production > Budget"),
        leaf("production/shot-lists", "Shot Lists", "Production > Shot Lists", "scene"),
        leaf("production/schedule", "Schedule", "Production > Schedule", "shootingDay"),
        leaf("production/sides", "Sides", "Production > Sides", "dayAndScene"),
      ],
    },
    { id: "call-sheets", label: "Call Sheets", leaves: [leaf("call-sheets/call-sheets", "Call Sheets", "Call Sheets", "callSheet")] },
    { id: "files", label: "Files", leaves: [leaf("files/files", "Files", "Files")] },
  ],
  screenplays: [
    {
      id: "sp1",
      title: "BLACK RAIN",
      drafts: [
        { id: "d1", name: "Draft 1", current: false },
        { id: "d2", name: "Draft 2", current: true },
      ],
      scenesDraftId: "d2",
      scenes: [
        { id: "sc1", number: 1, heading: "INT. STATION - NIGHT" },
        { id: "sc2", number: 2, heading: "EXT. BUS STOP - DAY" },
      ],
    },
  ],
  days: [
    { id: "day1", label: "Shoot Day 1", date: "2027-03-01" },
    { id: "day2", label: "Shoot Day 2", date: "2027-03-02" },
  ],
  callSheets: [{ id: "cs1", title: "Call sheet, Day 1", dayId: "day1" }],
};
let tree: AiSectionTreeDto = oneScreenplay;
let sectionsFail = false;
let projectId = "p";
let askGate: Promise<AiExchange | null> | null = null;

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
          return { id: projectId, title: "BLACK RAIN", projectType: "Feature Film" };
        case "ai.status":
          return status;
        case "ai.history":
          return { conversationId: exchanges.length ? "conv1" : null, exchanges };
        case "ai.sections":
          if (sectionsFail) throw new Error("no sections");
          return tree;
        case "ai.ask":
          return askGate ?? askReply;
        case "ai.change_set.accept":
          return changeSet("Applied", { appliedOperations: 1 });
        case "ai.change_set.reject":
          return changeSet("Rejected");
        case "ai.index_status":
          return indexState;
        case "ai.download.status":
          return { canDownload: true, phase: "Idle", sizeBytes: 1_106_000_000, neededBytes: 1_643_000_000, freeBytes: 40_000_000_000, doneBytes: 0, totalBytes: 0, problem: null };
        default:
          return null;
      }
    }),
  };
});

function renderPanel(onClose: () => void = () => undefined) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <AiPanel onClose={onClose} />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  calls.length = 0;
  exchanges = [];
  askReply = null;
  indexState = "Current";
  tree = oneScreenplay;
  sectionsFail = false;
  projectId = "p";
  askGate = null;
  useAiPicks.setState({ picks: [], projectId: "p" });
  useNav.getState().reset();
  status = { available: true, mode: "Ready", bundleState: "NotChecked", problem: null, runtimeState: "Installed", runtimeMessage: null, localOnly: true };
});

/** Open the picker, tick the named sections and close it, as a person would. */
async function chooseSections(names: string[]) {
  const toggle = await screen.findByRole("button", { name: /^(Choose|Change) sections$/ });
  await userEvent.click(toggle);
  for (const n of names) await userEvent.click(screen.getByRole("checkbox", { name: n }));
  await userEvent.click(screen.getByRole("button", { name: "Done" }));
}

async function typeQuestion(text: string) {
  const box = await screen.findByRole("textbox", { name: "Ask or command" });
  fireEvent.change(box, { target: { value: text } });
  return box;
}

describe("Straft AI panel", () => {
  it("is ready once Straft AI is downloaded: no download button, no setup step", async () => {
    renderPanel();
    expect(await screen.findByRole("textbox", { name: "Ask or command" })).toBeTruthy();
    expect(screen.getByText("Ready")).toBeTruthy();
    for (const gone of [/download/i, /install straft ai/i, /remove straft ai/i]) {
      expect(screen.queryByRole("button", { name: gone })).toBeNull();
    }
    expect(calls.some((c) => c.op.startsWith("ai.download"))).toBe(false);
  });

  it("offers the download while Straft AI is not downloaded, and starts it only on the click", async () => {
    status = {
      ...status,
      available: false,
      mode: "Off",
      bundleState: "NotInstalled",
      runtimeState: "Unavailable",
      problem: { code: "ai.not_installed", message: "Straft AI is not downloaded yet.", retryable: false },
    };
    renderPanel();
    const start = await screen.findByRole("button", { name: "Download Straft AI" });
    expect(screen.getByText(/A one-time download of 1.0 GB/)).toBeTruthy();
    expect(screen.queryByRole("textbox", { name: "Ask or command" })).toBeNull();
    expect(calls.some((c) => c.op === "ai.download.start")).toBe(false);
    await userEvent.click(start);
    await waitFor(() => expect(calls.some((c) => c.op === "ai.download.start")).toBe(true));
  });

  it("stays open beside the Straft AI settings it opens (the panel is docked beside the page)", async () => {
    const onClose = vi.fn();
    renderPanel(onClose);
    await screen.findByRole("textbox", { name: "Ask or command" });
    await userEvent.click(screen.getByRole("button", { name: "Straft AI settings" }));
    await userEvent.click(await screen.findByRole("menuitem", { name: "Straft AI settings…" }));
    expect(useNav.getState().route).toMatchObject({ workspace: "settings", sub: "offline-ai" });
    expect(onClose).not.toHaveBeenCalled();
  });

  it("says plainly when the engine in the app is damaged and how to repair it, and never offers a download", async () => {
    status = {
      ...status,
      available: false,
      mode: "Off",
      bundleState: "Damaged",
      runtimeState: "Unavailable",
      problem: {
        code: "ai.assets_damaged",
        message: "Straft AI files in this installation were changed or damaged, so Straft did not start them. Reinstall Straft Studio to repair it. Your projects were not changed.",
        retryable: false,
      },
    };
    renderPanel();
    expect(await screen.findByText(/Reinstall Straft Studio to repair it/)).toBeTruthy();
    expect(screen.getByText("Everything else in Straft works normally.")).toBeTruthy();
    expect(screen.queryByRole("textbox", { name: "Ask or command" })).toBeNull();
    expect(document.body.textContent?.toLowerCase()).not.toContain("download");
  });

  it("previews a Change Set and applies it only on explicit Apply", async () => {
    status = { ...status, mode: "Local model", runtimeState: "Ready" };
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
    status = { ...status, mode: "Local model", runtimeState: "Ready" };
    exchanges = [exchange("2", "Create a scene card", { kind: "Proposal", status: "Pending Approval", content: "Prepared.", changeSet: changeSet("Pending") })];
    renderPanel();
    fireEvent.click(await screen.findByRole("button", { name: "Reject" }));
    await waitFor(() => expect(calls.some((c) => c.op === "ai.change_set.reject" && c.args.id === "cs1")).toBe(true));
    expect(calls.some((c) => c.op === "ai.change_set.accept")).toBe(false);
  });

  it("asks for the usual confirmation before applying destructive changes", async () => {
    status = { ...status, mode: "Local model", runtimeState: "Ready" };
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
    status = { ...status, mode: "Local model", runtimeState: "Ready" };
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
    status = { ...status, mode: "Local model", runtimeState: "Ready" };
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
    status = { ...status, mode: "Local model", runtimeState: "Ready" };
    indexState = "Rebuilding";
    renderPanel();
    expect(await screen.findByText("Preparing project context…")).toBeTruthy();
    expect(screen.getByText(/AI can still answer some questions while project context is being prepared\./)).toBeTruthy();
  });

  it("never offers Apply on a stale proposal", async () => {
    status = { ...status, mode: "Local model", runtimeState: "Ready" };
    exchanges = [
      exchange("1", "Apply the rename", {
        kind: "Proposal", status: "Stale", content: "I prepared a rename.",
        changeSet: changeSet("Stale", { staleReason: "The project changed after this proposal was prepared (1 item edited). It has not been applied." }),
      }),
    ];
    renderPanel();
    expect(await screen.findByText("Proposed Changes (out of date)")).toBeTruthy();
    expect(screen.getByText(/It has not been applied\./)).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Apply Changes" })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Re-check & Review" }));
    await waitFor(() => expect(calls.some((c) => c.op === "ai.change_set.recheck")).toBe(true));
  });

  it("sends the request with the picked sections and runs navigation directly", async () => {
    status = { ...status, mode: "Local model", runtimeState: "Ready" };
    askReply = exchange("3", "Open scene 2", { kind: "Navigate", content: "Opening Scene 2 in the Screenplay.", nav: { workspace: "screenplay", params: { sceneId: "s2" } } });
    renderPanel();
    const box = await screen.findByRole("textbox", { name: "Ask or command" });
    await chooseSections(["Scenes"]);
    fireEvent.change(box, { target: { value: "Open scene 2" } });
    fireEvent.keyDown(box, { key: "Enter" });
    await waitFor(() => expect(useNav.getState().route).toEqual({ workspace: "screenplay", params: { sceneId: "s2" } }));
    const ask = calls.find((c) => c.op === "ai.ask")!;
    expect(ask.args.text).toBe("Open scene 2");
    // The panel sends the sections and nothing about the screen it was opened on.
    expect(ask.args.sections).toEqual([{ leaf: "screenplay/scenes" }]);
    expect(ask.args).not.toHaveProperty("scope");
  });
});

describe("Section picker", () => {
  beforeEach(() => {
    status = { ...status, mode: "Local model", runtimeState: "Ready" };
  });

  it("starts with nothing picked: Send is off and says why", async () => {
    renderPanel();
    await typeQuestion("How many scenes?");
    const send = screen.getByRole("button", { name: "Send" });
    expect((send as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText("Pick a section")).toBeTruthy();
    expect(send.getAttribute("aria-describedby")).toBe("ai-send-hint");
    expect(screen.queryByRole("list", { name: "Chosen sections" })).toBeNull();
    // Nothing is sent from the keyboard either.
    fireEvent.keyDown(screen.getByRole("textbox", { name: "Ask or command" }), { key: "Enter" });
    expect(calls.some((c) => c.op === "ai.ask")).toBe(false);
  });

  it("turns Send on once a section is picked, and shows the pick as a chip", async () => {
    renderPanel();
    await typeQuestion("Who plays the lead?");
    await chooseSections(["Cast & Crew"]);
    const list = screen.getByRole("list", { name: "Chosen sections" });
    expect(list.textContent).toContain("Production > Cast & Crew");
    expect((screen.getByRole("button", { name: "Send" }) as HTMLButtonElement).disabled).toBe(false);
    expect(screen.queryByText("Pick a section")).toBeNull();
    expect(screen.getByText("1 of 3 chosen")).toBeTruthy();
  });

  it("takes three sections at most and says how to add another", async () => {
    renderPanel();
    await chooseSections(["Scenes", "Cast & Crew", "Budget"]);
    expect(screen.getByText("Three sections at most. Remove one to add another.")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Change sections" }));
    expect((screen.getByRole("checkbox", { name: "Files" }) as HTMLInputElement).disabled).toBe(true);
    // The ones already picked stay clickable, so they can be removed there too.
    expect((screen.getByRole("checkbox", { name: "Budget" }) as HTMLInputElement).disabled).toBe(false);
    await userEvent.click(screen.getByRole("button", { name: "Remove Production > Budget" }));
    expect(screen.queryByText("Three sections at most. Remove one to add another.")).toBeNull();
    expect((screen.getByRole("checkbox", { name: "Files" }) as HTMLInputElement).disabled).toBe(false);
    expect(screen.queryByRole("button", { name: "Remove Production > Budget" })).toBeNull();
  });

  it("asks which scene, day or call sheet under the sections that need one, and sends the ids", async () => {
    askReply = exchange("9", "q", { content: "ok" });
    renderPanel();
    await typeQuestion("What is shot?");
    await chooseSections(["Shot Lists", "Sides", "Call Sheets"]);
    await userEvent.selectOptions(screen.getAllByLabelText("Scene")[0], "sc2");
    await userEvent.selectOptions(screen.getAllByLabelText("Shooting day")[0], "day2");
    await userEvent.selectOptions(screen.getByLabelText("Call sheet"), "cs1");
    // Scenes are named by number and heading; "All scenes" is the first choice.
    expect(screen.getAllByRole("option", { name: "All scenes" }).length).toBe(2);
    expect(screen.getAllByRole("option", { name: "Scene 2: EXT. BUS STOP - DAY" }).length).toBe(2);
    await userEvent.click(screen.getByRole("button", { name: "Send" }));
    await waitFor(() => expect(calls.some((c) => c.op === "ai.ask")).toBe(true));
    expect(calls.find((c) => c.op === "ai.ask")!.args.sections).toEqual([
      { leaf: "production/shot-lists", sceneId: "sc2" },
      { leaf: "production/sides", shootingDayId: "day2" },
      { leaf: "call-sheets/call-sheets", callSheetId: "cs1" },
    ]);
  });

  it("offers the drafts of the screenplay and reads the scenes of the draft chosen", async () => {
    renderPanel();
    await chooseSections(["Scenes"]);
    const draft = screen.getByLabelText("Draft");
    expect((draft as HTMLSelectElement).value).toBe("");
    expect(screen.getByRole("option", { name: "Current draft" })).toBeTruthy();
    await userEvent.selectOptions(draft, "d1");
    await waitFor(() => expect(calls.some((c) => c.op === "ai.sections" && c.args.draftId === "d1")).toBe(true));
  });

  it("makes the person choose a screenplay when the project has several", async () => {
    tree = {
      ...oneScreenplay,
      screenplays: [
        ...oneScreenplay.screenplays,
        { id: "sp2", title: "THE TEASER", drafts: [{ id: "t1", name: "Draft 1", current: true }], scenesDraftId: "t1", scenes: [] },
      ],
    };
    renderPanel();
    await typeQuestion("How many scenes?");
    await chooseSections(["Scenes"]);
    expect(screen.getByText("Choose a screenplay", { selector: "div" })).toBeTruthy();
    expect((screen.getByRole("button", { name: "Send" }) as HTMLButtonElement).disabled).toBe(true);
    await userEvent.selectOptions(screen.getByLabelText("Screenplay"), "sp2");
    expect((screen.getByRole("button", { name: "Send" }) as HTMLButtonElement).disabled).toBe(false);
  });

  it("keeps the selection while the panel is closed and opened again, and New starts empty", async () => {
    const first = renderPanel();
    await chooseSections(["Scenes", "Budget"]);
    expect(screen.getByRole("list", { name: "Chosen sections" }).textContent).toContain("Production > Budget");
    first.unmount();
    renderPanel();
    expect((await screen.findByRole("list", { name: "Chosen sections" })).textContent).toContain("Production > Budget");
    await userEvent.click(screen.getByRole("button", { name: "New" }));
    await waitFor(() => expect(screen.queryByRole("list", { name: "Chosen sections" })).toBeNull());
    await typeQuestion("again");
    expect(screen.getByText("Pick a section")).toBeTruthy();
  });

  it("works from the keyboard and names every control", async () => {
    const user = userEvent.setup();
    renderPanel();
    const toggle = await screen.findByRole("button", { name: "Choose sections" });
    toggle.focus();
    await user.keyboard("{Enter}");
    expect(toggle.getAttribute("aria-expanded")).toBe("true");
    expect(screen.getByRole("group", { name: "Sections" })).toBeTruthy();
    expect(screen.getByRole("group", { name: "Production" })).toBeTruthy();
    const scenes = screen.getByRole("checkbox", { name: "Scenes" });
    scenes.focus();
    await user.keyboard(" ");
    expect((scenes as HTMLInputElement).checked).toBe(true);
    const remove = screen.getByRole("button", { name: "Remove Screenplay > Scenes" });
    remove.focus();
    await user.keyboard("{Enter}");
    expect(screen.queryByRole("button", { name: "Remove Screenplay > Scenes" })).toBeNull();
    expect((scenes as HTMLInputElement).checked).toBe(false);
  });

  it("shows which sections an answer was read from", async () => {
    exchanges = [exchange("1", "How many scenes?", { content: "Draft 1 contains 3 scenes." })];
    renderPanel();
    expect(await screen.findByText("Using: Screenplay (Draft 1, all scenes)")).toBeTruthy();
  });

  it("adds the section an answer points to and puts focus on the question box", async () => {
    exchanges = [
      exchange("1", "How much is the camera?", {
        content: "That is in Production > Budget. Add it to your selection and ask again.",
        pointer: { leaf: "production/budget", label: "Budget" },
      }),
    ];
    useAiPicks.getState().set([{ leaf: "screenplay/scenes" }]);
    renderPanel();
    await userEvent.click(await screen.findByRole("button", { name: "Add Budget" }));
    expect(useAiPicks.getState().picks.map((p) => p.leaf)).toEqual(["screenplay/scenes", "production/budget"]);
    expect(document.activeElement).toBe(screen.getByLabelText("Ask or command"));
    expect(screen.queryByRole("button", { name: "Add Budget" })).toBeNull();
  });

  it("offers no Add button when three sections are already picked", async () => {
    exchanges = [
      exchange("1", "How much is the camera?", {
        content: "That is in Production > Budget. Remove a section and add this one, then ask again.",
        pointer: { leaf: "production/budget", label: "Budget" },
      }),
    ];
    useAiPicks.getState().set([{ leaf: "screenplay/scenes" }, { leaf: "files/files" }, { leaf: "production/cast-crew" }]);
    renderPanel();
    expect(await screen.findByText(/Remove a section and add this one/)).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Add Budget" })).toBeNull();
  });
});

describe("Section picker review fixes", () => {
  beforeEach(() => {
    status = { ...status, mode: "Local model", runtimeState: "Ready" };
  });

  it("empties the selection when the panel finds another project open, and keeps it for the same one", async () => {
    useAiPicks.setState({ picks: [{ leaf: "production/budget" }], projectId: "p" });
    const same = renderPanel();
    expect((await screen.findByRole("list", { name: "Chosen sections" })).textContent).toContain("Production > Budget");
    same.unmount();
    projectId = "other";
    renderPanel();
    await waitFor(() => expect(useAiPicks.getState().projectId).toBe("other"));
    await screen.findByRole("textbox", { name: "Ask or command" });
    expect(screen.queryByRole("list", { name: "Chosen sections" })).toBeNull();
    expect(useAiPicks.getState().picks).toEqual([]);
  });

  it("disables New while a request runs, so the answer cannot bring the old conversation back", async () => {
    let release: (x: AiExchange) => void = () => undefined;
    askGate = new Promise<AiExchange | null>((r) => {
      release = r;
    });
    renderPanel();
    await typeQuestion("How many scenes?");
    await chooseSections(["Scenes"]);
    expect((screen.getByRole("button", { name: "New" }) as HTMLButtonElement).disabled).toBe(false);
    await userEvent.click(screen.getByRole("button", { name: "Send" }));
    await waitFor(() => expect((screen.getByRole("button", { name: "New" }) as HTMLButtonElement).disabled).toBe(true));
    release(exchange("1", "How many scenes?", { content: "Two." }));
    await waitFor(() => expect((screen.getByRole("button", { name: "New" }) as HTMLButtonElement).disabled).toBe(false));
  });

  it("says in plain words when the sections cannot be loaded, offers Retry, and keeps Send off", async () => {
    sectionsFail = true;
    useAiPicks.setState({ picks: [{ leaf: "screenplay/scenes" }], projectId: "p" });
    renderPanel();
    expect(await screen.findByText("The sections could not be loaded.")).toBeTruthy();
    expect(screen.queryByText("Loading the sections…")).toBeNull();
    await typeQuestion("How many scenes?");
    expect((screen.getByRole("button", { name: "Send" }) as HTMLButtonElement).disabled).toBe(true);
    sectionsFail = false;
    await userEvent.click(screen.getByRole("button", { name: "Retry" }));
    expect(await screen.findByRole("button", { name: "Change sections" })).toBeTruthy();
    expect((screen.getByRole("button", { name: "Send" }) as HTMLButtonElement).disabled).toBe(false);
  });

  it("moves focus to the sections button when a chip is removed", async () => {
    renderPanel();
    await chooseSections(["Budget"]);
    await userEvent.click(screen.getByRole("button", { name: "Remove Production > Budget" }));
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Choose sections" }));
  });

  it("scrolls inside the picker when many sections are chosen instead of being clipped", () => {
    const css = readFileSync(resolve(process.cwd(), "src/ai/ai.css"), "utf8");
    const rule = css.split("\n").find((l) => l.startsWith(".ai .ai-sections{")) ?? "";
    expect(rule).toMatch(/max-height:/);
    expect(rule).toMatch(/overflow-y:auto/);
  });
});
