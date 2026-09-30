// Visual planning: pure logic (canvas geometry, labels, selection) and the
// Shot Lists tab rendered standalone against a mocked IPC boundary.

import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { canvasExtent, changedMoves, dragBoxes, resizeBox, type Box } from "./canvas";
import { looksLikeUrl, moveItem, parseSeconds, sceneLabel, shotLetters } from "../../../api/visual";
import type { VisualScenes } from "../../../ipc/generated/VisualScenes";
import type { ShotDto } from "../../../ipc/generated/ShotDto";
import type { StoryboardSummary } from "../../../ipc/generated/StoryboardSummary";
import { resolveSelection } from "../tabs/storyboards";
import ShotsTab from "../tabs/shots";
import { useVisualView } from "./shared";

// Rendering whole tabs is slow on a busy build machine.
vi.setConfig({ testTimeout: 30_000 });

const calls: { op: string; args: unknown }[] = [];
let responses: Record<string, unknown> = {};

vi.mock("../../../ipc/client", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../../ipc/client")>();
  return {
    ...actual,
    inTauri: () => false,
    onAppEvent: () => Promise.resolve(() => {}),
    call: vi.fn(async (op: string, args: unknown) => {
      calls.push({ op, args });
      if (!(op in responses)) throw new Error(`unexpected op ${op}`);
      const r = responses[op];
      return typeof r === "function" ? (r as (a: unknown) => unknown)(args) : r;
    }),
  };
});

// ------------------------------------------------------------------ pure logic

describe("moodboard canvas geometry", () => {
  const boxes: Box[] = [
    { id: "a", x: 10, y: 20, w: 100, h: 80 },
    { id: "b", x: 300, y: 40, w: 50, h: 50 },
    { id: "c", x: 0, y: 0, w: 60, h: 60 },
  ];

  it("moves a multi-selection together and never past the canvas edge", () => {
    const moved = dragBoxes(boxes, new Set(["a", "b"]), 25.4, -100);
    expect(moved.find((b) => b.id === "a")).toMatchObject({ x: 35, y: 0 });
    expect(moved.find((b) => b.id === "b")).toMatchObject({ x: 325, y: 20 }); // relative offset kept
    expect(moved.find((b) => b.id === "c")).toEqual(boxes[2]);
    expect(changedMoves(boxes, moved)).toEqual([
      { id: "a", x: 35, y: 0 },
      { id: "b", x: 325, y: 20 },
    ]);
  });

  it("resizes with a minimum size and optional aspect lock", () => {
    expect(resizeBox(boxes[0], -500, -500, false)).toMatchObject({ w: 40, h: 40 });
    const locked = resizeBox(boxes[0], 100, 0, true);
    expect(locked.w).toBe(200);
    expect(locked.h).toBe(160);
  });

  it("grows the scrollable area with the content", () => {
    expect(canvasExtent(boxes, 100)).toEqual({ width: 450, height: 228 });
  });
});

describe("visual helpers", () => {
  it("derives shot letters like Rust (A…Z, AA…)", () => {
    expect([0, 1, 25, 26, 27].map(shotLetters)).toEqual(["A", "B", "Z", "AA", "AB"]);
  });
  it("moves list items for drag previews", () => {
    expect(moveItem(["a", "b", "c"], 2, 0)).toEqual(["c", "a", "b"]);
  });
  it("parses durations in seconds", () => {
    expect(parseSeconds("3.5")).toBe(3500);
    expect(parseSeconds("2 s")).toBe(2000);
    expect(parseSeconds("")).toBe(0);
    expect(parseSeconds("-1")).toBeNull();
    expect(parseSeconds("abc")).toBeNull();
  });
  it("labels scenes from the screenplay", () => {
    expect(sceneLabel({ number: "12", heading: "INT. POLICE STATION - NIGHT", groupLabel: null })).toBe(
      "Scene 12 — INT. POLICE STATION - NIGHT",
    );
  });
  it("recognises pasted links", () => {
    expect(looksLikeUrl("imdb.com/title/tt0353969")).toBe(true);
    expect(looksLikeUrl("Cold, wet, blue-grey.")).toBe(false);
  });
});

describe("storyboard selection", () => {
  it("defaults to the first scene that has a storyboard and follows board links to their scene", () => {
    const scenes = scenesFixture();
    const boards = [board("b1", "L2"), board("b2", null)];
    expect(resolveSelection(null, {}, scenes, boards)).toMatchObject({ key: "scene:L2", boardId: "b1" });
    expect(resolveSelection("board:b1", {}, scenes, boards)).toMatchObject({ key: "scene:L2", boardId: "b1" });
    expect(resolveSelection("board:b2", {}, scenes, boards)).toMatchObject({ key: "board:b2", scene: null });
    expect(resolveSelection("scene:L1", {}, scenes, boards)).toMatchObject({ key: "scene:L1", boardId: null });
  });
});

// ------------------------------------------------------------------ Shot Lists tab

function scenesFixture(): VisualScenes {
  const scene = (n: number, lineage: string, heading: string) => ({
    sceneId: `S${n}`,
    lineageId: lineage,
    draftId: "D",
    number: String(n),
    heading,
    omitted: false,
    groupLabel: null,
    shotCount: 0,
    storyboardCount: 0,
    moodboardCount: 0,
    needsReview: false,
    changedAt: null,
  });
  return {
    sourceKind: "currentDraft",
    sourceLabel: "Current draft: First Draft",
    scenes: [scene(1, "L1", "EXT. PLATFORM - NIGHT"), { ...scene(2, "L2", "INT. POLICE STATION - NIGHT"), needsReview: true, changedAt: 1_700_000_000_000 }],
    removed: [],
  };
}

function board(id: string, lineage: string | null): StoryboardSummary {
  return {
    id,
    name: id,
    sceneId: lineage ? "S2" : null,
    sceneLineageId: lineage,
    sceneNumber: lineage ? "2" : null,
    sceneHeading: null,
    sceneRemoved: false,
    needsReview: false,
    panelCount: 0,
    cover: null,
    rev: 1,
  };
}

function shot(id: string, label: string, description: string, extra: Partial<ShotDto> = {}): ShotDto {
  return {
    id,
    sceneId: "S2",
    sceneLineageId: "L2",
    sceneNumber: "2",
    sceneHeading: "INT. POLICE STATION - NIGHT",
    sceneRemoved: false,
    label,
    order: 1,
    description,
    size: null,
    movement: null,
    angle: null,
    lens: null,
    cameraNotes: null,
    characters: [],
    soundNote: null,
    referenceAsset: null,
    storyboardPanelId: null,
    panels: [],
    needsReview: true,
    createdAt: 0,
    updatedAt: 0,
    rev: 1,
    ...extra,
  };
}

async function renderShots() {
  useVisualView.setState({ shotScene: "L2" });
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <ShotsTab />
    </QueryClientProvider>,
  );
}

describe("Shot Lists tab", () => {
  beforeEach(() => {
    calls.length = 0;
    responses = {
      "visual.scenes": scenesFixture(),
      "shot.list": [shot("a", "2A", "Arjun enters the station", { size: "Wide", movement: "Static" }), shot("b", "2B", "Meera looks up from the desk")],
      "shot.create": (a: unknown) => shot("c", "2C", (a as { description: string }).description),
    };
  });

  it("lists the scene's shots with generated labels and the review indicator", async () => {
    await renderShots();
    expect(await screen.findByText("Arjun enters the station")).toBeInTheDocument();
    expect(screen.getByText("2A")).toBeInTheDocument();
    expect(screen.getByText("2B")).toBeInTheDocument();
    expect(screen.getByText("Wide")).toBeInTheDocument();
    expect(screen.getByText("2 shots")).toBeInTheDocument();
    expect(screen.getByText(/Scene 2 changed since planning/)).toBeInTheDocument();
    expect(screen.getByText(/you never renumber by hand/)).toBeInTheDocument();
    // FSD §33.6: a real Shot List export (by scene, scenes or whole project).
    expect(screen.getByRole("button", { name: "Export" })).toBeInTheDocument();
    expect(calls.find((c) => c.op === "shot.list")?.args).toEqual({ sceneLineageId: "L2" });
  });

  it("adds a shot with only a description and validates empty input", async () => {
    await renderShots();
    const input = await screen.findByLabelText("New shot description");
    fireEvent.submit(input.closest("form") as HTMLFormElement);
    expect(await screen.findByRole("alert")).toHaveTextContent("Describe the shot to add it.");
    expect(calls.some((c) => c.op === "shot.create")).toBe(false);
    fireEvent.change(input, { target: { value: "Close on the red folder" } });
    fireEvent.submit(input.closest("form") as HTMLFormElement);
    await waitFor(() => expect(calls.find((c) => c.op === "shot.create")?.args).toEqual({ sceneId: "S2", description: "Close on the red folder" }));
  });

  it("shows the no-script state when there are no scenes", async () => {
    responses["visual.scenes"] = { sourceKind: "none", sourceLabel: "", scenes: [], removed: [] };
    await renderShots();
    expect(await screen.findByText("No screenplay scenes yet")).toBeInTheDocument();
  });
});
