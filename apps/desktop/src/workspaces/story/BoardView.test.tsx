import { describe, expect, it, beforeEach } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/react";
import type { StoryBoardState } from "../../ipc/generated/StoryBoardState";
import { BoardView } from "./BoardView";
import { OutlineView } from "./OutlineView";
import { useStoryUi } from "./ui";

const board: StoryBoardState = {
  episodeId: null,
  acts: [
    {
      id: "a1",
      episodeId: null,
      title: "Act 1 — The Return",
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
              sceneHeading: "EXT. BUS STAND — NIGHT",
              notes: null,
              color: null,
              screenplaySceneId: null,
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
              sceneHeading: null,
              notes: null,
              color: null,
              screenplaySceneId: null,
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
    expect(screen.getByText("Act 1 — The Return")).toBeInTheDocument();
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

  it("outline shows the same hierarchy in the same order", () => {
    render(<OutlineView board={board} episodeId={null} readOnly={false} />);
    const rows = screen.getAllByRole("treeitem").map((r) => r.textContent ?? "");
    const idx = (s: string) => rows.findIndex((r) => r.includes(s));
    expect(idx("Act 1 — The Return")).toBeLessThan(idx("Hero Introduction"));
    expect(idx("Hero Introduction")).toBeLessThan(idx("Arjun steps off"));
    expect(idx("Arjun steps off")).toBeLessThan(idx("Morning at the old house"));
    expect(idx("Parking Lot")).toBeLessThan(idx("Ravi's secret radio"));
  });
});
