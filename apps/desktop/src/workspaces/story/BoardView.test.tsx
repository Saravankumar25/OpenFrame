import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it, beforeEach, vi } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/react";
import type { StoryBoardState } from "../../ipc/generated/StoryBoardState";
import { BoardView } from "./BoardView";
import { OutlineView } from "./OutlineView";
import { useStoryUi } from "./ui";
import { MINIMAL } from "./CardDisplay";
import { inFirstHalf } from "./dnd";
import StoryWorkspace from "./index";
import { useNav } from "../../app/stores";
import type { StoryCardToggles } from "../../ipc/generated/StoryCardToggles";

// Only the Beat Board page header test mounts the whole Story workspace; it reads the board from here.
let mockBoard: StoryBoardState | undefined;
vi.mock("../../api/story", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api/story")>()),
  useSeries: () => ({ data: undefined }),
  useViewState: () => ({ data: undefined }),
  useCardDisplay: () => ({ data: undefined }),
  useStoryBoard: () => ({ data: mockBoard, isLoading: false, error: null }),
}));
vi.mock("../../features/export/buttons", () => ({ StoryExportButton: () => null }));

const board: StoryBoardState = {
  episodeId: null,
  acts: [
    {
      id: "a1",
      episodeId: null,
      title: "Act 1: The Return",
      note: null,
      cardCount: 2,
      rev: 1,
      items: [
        {
          kind: "sequence",
          id: "s1",
          actId: "a1",
          episodeId: null,
          title: "Hero Introduction",
          note: null,
          cardCount: 2,
          attachments: [],
          commentCount: 0,
          rev: 1,
          items: [
            {
              kind: "card",
              id: "c1",
              episodeId: null,
              parentType: "sequence",
              parentId: "s1",
              shortDescription: "Arjun steps off the night bus into the empty town.",
              longDescription: "The bus stand is empty. A single tube light hums over the timetable board.",
              sceneHeading: "EXT. BUS STAND - NIGHT",
              notes: "Shoot at the real stand if the council agrees.",
              color: null,
              screenplaySceneId: "sc1",
              sceneStatus: { sceneId: "sc1", draftId: "d1", draftName: "Draft 1", revised: true, breakdownItems: 3, breakdownComplete: false, shots: 2, storyboards: 1 },
              setName: "Bus Stand",
              timeOfDay: "NIGHT",
              sourceBeatId: null,
              parkedFromType: null,
              parkedFromId: null,
              commentCount: 2,
              attachments: [],
              characterIds: [],
              rev: 1,
              createdAt: 0,
              updatedAt: 0,
            },
            {
              kind: "card",
              id: "c2",
              episodeId: null,
              parentType: "sequence",
              parentId: "s1",
              shortDescription: "Morning at the old house.",
              longDescription: null,
              sceneHeading: null,
              notes: null,
              color: null,
              screenplaySceneId: null,
              sceneStatus: null,
              setName: null,
              timeOfDay: null,
              sourceBeatId: null,
              parkedFromType: null,
              parkedFromId: null,
              commentCount: 0,
              attachments: [],
              characterIds: [],
              rev: 1,
              createdAt: 0,
              updatedAt: 0,
            },
          ],
        },
      ],
    },
  ],
  parking: [
    {
      kind: "beat",
      id: "b1",
      episodeId: null,
      parentType: "parking",
      parentId: null,
      text: "Ravi's secret radio",
      note: null,
      color: null,
      state: "active",
      convertedSceneCardId: null,
      parkedFromType: null,
      parkedFromId: null,
      commentCount: 0,
      attachments: [],
      rev: 1,
    },
  ],
  unassigned: [],
  collapsedIds: [],
  cardCount: 2,
  beatCount: 1,
  sequenceCount: 1,
};

describe("Beat Board views", () => {
  beforeEach(() => {
    useStoryUi.setState({ selected: [], drawer: null, dialog: null, editingKey: null, filter: "", parkingOpen: true, zoom: 100, boardLayout: "vertical" });
  });

  it("shows acts, sequences, compact cards without scene numbers, and the Parking Lot", () => {
    render(<BoardView board={board} episodeId={null} readOnly={false} />);
    expect(screen.getByText("Act 1: The Return")).toBeInTheDocument();
    expect(screen.getByText("2 scenes")).toBeInTheDocument();
    expect(screen.getByText("Hero Introduction")).toBeInTheDocument();
    const c1 = screen.getByRole("button", { name: /Scene Card: Arjun steps off/ });
    expect(within(c1).getByTitle("has a scene heading")).toHaveTextContent("H");
    expect(within(c1).getByTitle("2 open comments")).toBeInTheDocument();
    expect(screen.queryByText(/Scene 1\b/)).toBeNull();
    const lot = screen.getByRole("region", { name: "Parking Lot" });
    expect(within(lot).getByText("Ravi's secret radio")).toBeInTheDocument();
  });

  it("selects on click, multi-selects with Ctrl+click and opens the drawer on double-click", () => {
    render(<BoardView board={board} episodeId={null} readOnly={false} />);
    const c1 = screen.getByRole("button", { name: /Arjun steps off/ });
    const c2 = screen.getByRole("button", { name: /Morning at the old house/ });
    fireEvent.click(c1);
    expect(useStoryUi.getState().selected).toEqual(["card:c1"]);
    fireEvent.click(c2, { ctrlKey: true });
    expect(useStoryUi.getState().selected).toEqual(["card:c1", "card:c2"]);
    fireEvent.doubleClick(c2);
    expect(useStoryUi.getState().drawer).toEqual({ kind: "card", id: "c2" });
  });

  it("filters cards visually without touching data", () => {
    useStoryUi.setState({ filter: "morning" });
    render(<BoardView board={board} episodeId={null} readOnly={false} />);
    expect(screen.queryByText(/Arjun steps off/)).toBeNull();
    expect(screen.getByText("Morning at the old house.")).toBeInTheDocument();
  });

  it("Minimal display keeps the compact card: no heading line, no long description preview", () => {
    const { container } = render(<BoardView board={board} episodeId={null} readOnly={false} display={MINIMAL} />);
    expect(container.querySelector(".sc-rich")).toBeNull();
    expect(screen.queryByText(/single tube light/)).toBeNull();
  });

  it("Story and Production toggles show more on the card, in words, without changing the board data", () => {
    const before = JSON.stringify(board);
    const story: StoryCardToggles = { ...MINIMAL, sceneHeading: true, longDescriptionPreview: true, notes: true };
    const { rerender } = render(<BoardView board={board} episodeId={null} readOnly={false} display={story} />);
    const c1 = screen.getByRole("button", { name: /Scene Card: Arjun steps off/ });
    expect(within(c1).getByText("EXT. BUS STAND - NIGHT")).toBeInTheDocument();
    expect(within(c1).getByText(/single tube light/)).toBeInTheDocument();
    expect(within(c1).getByText(/Note: Shoot at the real stand/)).toBeInTheDocument();
    expect(within(c1).queryByTitle("has a scene heading")).toBeNull();
    const production: StoryCardToggles = { ...MINIMAL, location: true, dayNight: true, revisionChange: true, breakdownStatus: true, shotStatus: true };
    rerender(<BoardView board={board} episodeId={null} readOnly={false} display={production} />);
    const p1 = screen.getByRole("button", { name: /Scene Card: Arjun steps off/ });
    expect(within(p1).getByText("Bus Stand · NIGHT · Revised · Breakdown: 3 items · 2 shots · 1 storyboard")).toBeInTheDocument();
    // Status is never colour alone, and a card without a linked scene shows no production line.
    const c2 = screen.getByRole("button", { name: /Scene Card: Morning at the old house/ });
    expect(c2.querySelector(".sc-facts")).toBeNull();
    // Only short description and nothing else: the card still names itself for screen readers.
    rerender(<BoardView board={board} episodeId={null} readOnly={false} display={{ ...MINIMAL, shortDescription: false, comments: false, attachments: true }} />);
    expect(screen.getByRole("button", { name: /Scene Card: Arjun steps off/ })).toBeInTheDocument();
    expect(JSON.stringify(board)).toBe(before);
  });

  it("outline shows the same hierarchy in the same order", () => {
    render(<OutlineView board={board} episodeId={null} readOnly={false} />);
    const rows = screen.getAllByRole("treeitem").map((r) => r.textContent ?? "");
    const idx = (s: string) => rows.findIndex((r) => r.includes(s));
    expect(idx("Act 1: The Return")).toBeLessThan(idx("Hero Introduction"));
    expect(idx("Hero Introduction")).toBeLessThan(idx("Arjun steps off"));
    expect(idx("Arjun steps off")).toBeLessThan(idx("Morning at the old house"));
    expect(idx("Parking Lot")).toBeLessThan(idx("Ravi's secret radio"));
  });

  it("outline is its own keyboard-focusable scroll area that also holds the Add Act button", () => {
    render(
      <OutlineView board={board} episodeId={null} readOnly={false}>
        <button type="button">Add Act</button>
      </OutlineView>,
    );
    const area = screen.getByRole("region", { name: /Story outline, scrollable/ });
    expect(area.className).toContain("story-canvas-outline");
    // Focusable so the arrow and Page keys scroll it; the tree and the Add Act button are inside it.
    expect(area.getAttribute("tabindex")).toBe("0");
    expect(within(area).getByRole("tree", { name: "Story outline" })).toBeInTheDocument();
    expect(within(area).getByRole("button", { name: "Add Act" })).toBeInTheDocument();
  });

  it("the Beat Board canvas rule that stops vertical scroll does not apply to the outline", () => {
    const css = readFileSync(join(__dirname, "story.css"), "utf8");
    // The board holds the page still (columns scroll); the outline has no columns, so it must scroll itself.
    expect(css).toMatch(/\.story-canvas \{ overflow-x: auto; overflow-y: hidden; \}/);
    const at = css.indexOf(".story-canvas.story-canvas-outline {");
    expect(at, "no rule for the outline scroll area").toBeGreaterThanOrEqual(0);
    const rule = css.slice(at, css.indexOf("}", at));
    expect(rule).toContain("overflow-y: auto");
    expect(rule).toContain("overflow-x: hidden");
    // It must come after the board rule, or the board rule would still win.
    expect(at).toBeGreaterThan(css.indexOf(".story-canvas { overflow-x: auto; overflow-y: hidden; }"));
  });
});

describe("Beat Board layout: Vertical columns or Horizontal rows", () => {
  beforeEach(() => {
    window.localStorage.clear();
    Element.prototype.scrollIntoView ??= () => undefined;
    useStoryUi.setState({ selected: [], drawer: null, dialog: null, editingKey: null, filter: "", parkingOpen: true, zoom: 100, boardLayout: "vertical" });
  });
  const canvas = () => screen.getByRole("region", { name: "Beat Board" });

  it("a fresh store starts Vertical, and an unknown stored value falls back to Vertical", async () => {
    vi.resetModules();
    expect((await import("./ui")).useStoryUi.getState().boardLayout).toBe("vertical");
    window.localStorage.setItem("of.story.boardLayout", JSON.stringify("diagonal"));
    vi.resetModules();
    expect((await import("./ui")).useStoryUi.getState().boardLayout).toBe("vertical");
  });

  it("the header toggle switches to Horizontal rows and the choice is remembered after a remount", async () => {
    mockBoard = board;
    useNav.setState({ route: { workspace: "story", sub: "board" } });
    const { unmount } = render(<StoryWorkspace />);
    const group = screen.getByRole("radiogroup", { name: "Beat Board layout" });
    const vertical = within(group).getByRole("radio", { name: "Vertical" });
    const horizontal = within(group).getByRole("radio", { name: "Horizontal" });
    expect(vertical).toHaveAttribute("aria-checked", "true");
    expect(canvas()).toHaveAttribute("data-layout", "vertical");

    fireEvent.click(horizontal);
    expect(horizontal).toHaveAttribute("aria-checked", "true");
    expect(canvas()).toHaveAttribute("data-layout", "horizontal");
    // A view preference in this browser profile only, never project data.
    expect(window.localStorage.getItem("of.story.boardLayout")).toBe(JSON.stringify("horizontal"));
    unmount();

    // A new store (the app restarted) reads the stored choice back.
    vi.resetModules();
    expect((await import("./ui")).useStoryUi.getState().boardLayout).toBe("horizontal");
    render(<StoryWorkspace />);
    expect(canvas()).toHaveAttribute("data-layout", "horizontal");
    fireEvent.click(screen.getByRole("radio", { name: "Vertical" }));
    expect(canvas()).toHaveAttribute("data-layout", "vertical");
    expect(window.localStorage.getItem("of.story.boardLayout")).toBe(JSON.stringify("vertical"));
  });

  it("arrow keys follow the layout: across an act in rows, down an act in columns", () => {
    const focused = () => document.activeElement?.getAttribute("data-story-key");
    const press = (key: string, k: string) => fireEvent.keyDown(document.querySelector(`[data-story-key="${key}"]`)!, { key: k });
    const { unmount } = render(<BoardView board={board} episodeId={null} readOnly={false} />);
    // Columns (today): Down is the next card in the act, Right jumps to the next column (the Parking Lot).
    press("card:c1", "ArrowDown");
    expect(focused()).toBe("card:c2");
    press("card:c1", "ArrowRight");
    expect(focused()).toBe("beat:b1");
    unmount();

    useStoryUi.setState({ boardLayout: "horizontal" });
    render(<BoardView board={board} episodeId={null} readOnly={false} />);
    expect(canvas()).toHaveAttribute("data-layout", "horizontal");
    // Rows: Right is the next card in the same act, Down jumps to the next row (the Parking Lot).
    press("card:c1", "ArrowRight");
    expect(focused()).toBe("card:c2");
    press("card:c2", "ArrowLeft");
    expect(focused()).toBe("card:c1");
    press("card:c1", "ArrowDown");
    expect(focused()).toBe("beat:b1");
    // Up from the Parking Lot row lands on the first item of the act row above.
    press("beat:b1", "ArrowUp");
    expect(focused()).toBe("sequence:s1");
  });

  it("drop position math follows the layout (before or after a card, before or after an act)", () => {
    const over = { left: 100, top: 100, width: 200, height: 80 }; // centre (200, 140)
    // The dragged card's centre is left of the target's centre but lower than it.
    const leftAndLow = { left: 80, top: 150, width: 200, height: 80 }; // centre (180, 190)
    // Columns: cards stack top to bottom, so a lower centre means after.
    expect(inFirstHalf("y", leftAndLow, over)).toBe(false);
    // Rows: cards flow left to right, so a centre to the left means before.
    expect(inFirstHalf("x", leftAndLow, over)).toBe(true);
    const rightAndHigh = { left: 130, top: 20, width: 200, height: 80 }; // centre (230, 60)
    expect(inFirstHalf("x", rightAndHigh, over)).toBe(false);
    expect(inFirstHalf("y", rightAndHigh, over)).toBe(true);
    // Acts: columns compare along x, rows along y (the same helper, the other axis).
    const actRow = { left: 0, top: 0, width: 900, height: 160 }; // centre y 80
    const draggedAct = { left: 600, top: 10, width: 220, height: 40 }; // centre (710, 30)
    expect(inFirstHalf("y", draggedAct, actRow)).toBe(true);
    expect(inFirstHalf("x", draggedAct, actRow)).toBe(false);
  });

  it("the horizontal rules stack the bands as rows that wrap their cards", () => {
    const css = readFileSync(join(__dirname, "story.css"), "utf8");
    expect(css).toMatch(/\.story-canvas\[data-layout="horizontal"\] \.board \{ flex-direction: column;/);
    expect(css).toMatch(/\.story-canvas\[data-layout="horizontal"\] \{ overflow-x: hidden; overflow-y: auto;/);
    expect(css).toMatch(/\.story-canvas\[data-layout="horizontal"\] \.unassigned-col \{ display: flex; flex-direction: row; flex-wrap: wrap;/);
  });
});
