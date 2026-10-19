import { describe, expect, it, beforeEach } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/react";
import type { StoryBoardState } from "../../ipc/generated/StoryBoardState";
import { BoardView } from "./BoardView";
import { OutlineView } from "./OutlineView";
import { useStoryUi } from "./ui";
import { MINIMAL } from "./CardDisplay";
import type { StoryCardToggles } from "../../ipc/generated/StoryCardToggles";

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

describe("Story Board views", () => {
  beforeEach(() => {
    useStoryUi.setState({ selected: [], drawer: null, dialog: null, editingKey: null, filter: "", parkingOpen: true, zoom: 100 });
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
});
