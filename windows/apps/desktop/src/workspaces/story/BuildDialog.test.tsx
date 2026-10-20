// Build Screenplay preview (owner decision 2026-10-07): a card without a scene heading is never built
// silently and no heading is ever invented. The preview asks for each one: add a heading, or leave
// the card out.

import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { StoryBuildPreview } from "../../ipc/generated/StoryBuildPreview";

const buildScreenplay = vi.fn(async (..._args: unknown[]) => ({
  screenplayId: "sp",
  draftId: "d1",
  draftName: "Draft 1 from Beat Board",
  firstSceneId: "s1",
  sceneCount: 2,
  createdScreenplay: true,
}));

const preview: StoryBuildPreview = {
  rows: [
    { cardId: "c1", order: 1, shortDescription: "Bus stop", sceneHeading: "EXT. BUS STOP - NIGHT", headingValid: true, location: "Act 1", alreadyBuilt: false },
    { cardId: "c2", order: 2, shortDescription: "Morning at the old house", sceneHeading: null, headingValid: false, location: "Act 1", alreadyBuilt: false },
    { cardId: "c3", order: 3, shortDescription: "Police station", sceneHeading: null, headingValid: false, location: "Act 1", alreadyBuilt: false },
  ],
  parkedCount: 0,
  unassignedCount: 0,
  screenplays: [],
  defaultTitle: "Black Rain",
};

let shown: StoryBuildPreview = preview;

vi.mock("../../api/story", () => ({
  useBuildPreview: () => ({ data: shown, isError: false }),
  useOrderPreview: () => ({ data: undefined }),
  story: { buildScreenplay: (...a: unknown[]) => buildScreenplay(...a) },
}));
vi.mock("../../app/toast", () => ({ toast: { undoable: vi.fn(), info: vi.fn(), error: vi.fn() } }));

import { BuildDialog } from "./BuildDialog";

const buildButton = () => screen.getAllByRole("button", { name: "Build Screenplay" }).at(-1)!;

describe("Build Screenplay preview", () => {
  beforeEach(() => {
    buildScreenplay.mockClear();
    shown = preview;
  });

  it("asks for a heading or for the card to be left out, and blocks the build until then", () => {
    render(<BuildDialog episodeId={null} />);
    expect(screen.getByText(/2 cards have no valid scene heading/)).toBeInTheDocument();
    expect(screen.getByText(/OpenFrame never makes up a heading/)).toBeInTheDocument();
    expect(screen.getAllByText("Heading needed")).toHaveLength(2);
    expect(buildButton()).toBeDisabled();
    expect(screen.getByText("3 of 3 cards will be built · 2 need a heading")).toBeInTheDocument();
  });

  it("builds with the typed heading and without the card left out; nothing is invented", async () => {
    render(<BuildDialog episodeId={null} />);
    fireEvent.change(screen.getByLabelText("Heading for scene 2"), { target: { value: "INT. OLD HOUSE - MORNING" } });
    expect(buildButton()).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Leave out scene 3" }));
    expect(screen.queryByText(/no valid scene heading/)).toBeNull();
    expect(screen.getByText("2 of 3 cards will be built")).toBeInTheDocument();
    expect(buildButton()).toBeEnabled();
    fireEvent.click(buildButton());
    await waitFor(() => expect(buildScreenplay).toHaveBeenCalledTimes(1));
    const args = buildScreenplay.mock.calls[0][0] as { include: { cardId: string; heading: string | null }[] };
    expect(args.include).toEqual([
      { cardId: "c1", heading: null },
      { cardId: "c2", heading: "INT. OLD HOUSE - MORNING" },
    ]);
  });

  it("a description is not a heading", () => {
    render(<BuildDialog episodeId={null} />);
    fireEvent.change(screen.getByLabelText("Heading for scene 2"), { target: { value: "the old house in the morning" } });
    fireEvent.click(screen.getByRole("button", { name: "Leave out scene 3" }));
    expect(buildButton()).toBeDisabled();
    expect(screen.getByText(/1 card has no valid scene heading/)).toBeInTheDocument();
  });

  it("builds a second screenplay beside the existing one with a title of its own (ADR-0012, 2026-10-08)", async () => {
    shown = {
      ...preview,
      rows: [preview.rows[0]],
      defaultTitle: "Black Rain (2)",
      screenplays: [
        {
          id: "sp1",
          title: "Black Rain",
          draftCount: 2,
          currentDraftId: "d2",
          currentDraftName: "Draft 2",
          hasWrittenScenes: true,
          nextDraftName: "Draft 3 from Beat Board",
        },
      ],
    };
    render(<BuildDialog episodeId={null} />);
    const second = screen.getByRole("radio", { name: /Build a new screenplay document/ });
    expect(second).toBeEnabled();
    fireEvent.click(second);
    const title = screen.getByLabelText("Title of the new screenplay");
    expect(title).toHaveValue("Black Rain (2)");
    fireEvent.change(title, { target: { value: "black rain" } });
    expect(screen.getByRole("alert")).toHaveTextContent("Another screenplay already has this title");
    expect(buildButton()).toBeDisabled();
    fireEvent.change(title, { target: { value: "Black Rain (short)" } });
    expect(buildButton()).toBeEnabled();
    fireEvent.click(buildButton());
    await waitFor(() => expect(buildScreenplay).toHaveBeenCalledTimes(1));
    expect(buildScreenplay.mock.calls[0][0]).toMatchObject({ destination: "newScreenplay", screenplayId: null, title: "Black Rain (short)" });
  });
});
