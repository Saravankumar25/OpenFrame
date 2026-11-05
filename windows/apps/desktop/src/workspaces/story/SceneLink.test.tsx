import { describe, expect, it, beforeEach, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { SceneLink } from "./SceneLink";

const linkScene = vi.fn(() => Promise.resolve());
let scenes: { sceneId: string; screenplayId: string; screenplayTitle: string; draftId: string; draftName: string; label: string; heading: string }[] = [];
vi.mock("../../api/story", () => ({
  story: { linkScene: (...a: unknown[]) => (linkScene as (...x: unknown[]) => Promise<void>)(...a) },
  useLinkableScenes: () => ({ data: scenes, isLoading: false }),
}));

const imported = {
  sceneId: "s2",
  screenplayId: "p1",
  screenplayTitle: "Black Rain",
  draftId: "d1",
  draftName: "Draft 1",
  label: "2",
  heading: "INT. POLICE STATION - DAY",
};

describe("SceneLink", () => {
  beforeEach(() => {
    linkScene.mockClear();
    scenes = [{ ...imported, sceneId: "s1", label: "1", heading: "EXT. BUS STOP - NIGHT" }, imported];
  });

  it("offers imported scenes with their number and links the chosen one", async () => {
    render(<SceneLink cardId="c1" linked={null} readOnly={false} />);
    fireEvent.change(screen.getByLabelText("Screenplay scene"), { target: { value: "s2" } });
    expect(screen.getByRole("option", { name: "Black Rain · Scene 2: INT. POLICE STATION - DAY" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Link" }));
    await waitFor(() => expect(linkScene).toHaveBeenCalledWith("c1", "s2"));
  });

  it("shows the linked scene number and heading, and unlinks", async () => {
    render(<SceneLink cardId="c1" linked={{ ...imported, number: "2" }} readOnly={false} />);
    expect(screen.getByText(/Scene 2: INT. POLICE STATION - DAY/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Unlink" }));
    await waitFor(() => expect(linkScene).toHaveBeenCalledWith("c1", null));
  });

  it("does not let a viewer change the link", () => {
    render(<SceneLink cardId="c1" linked={{ ...imported, number: "2" }} readOnly />);
    expect((screen.getByRole("button", { name: "Unlink" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("says so when there is no screenplay scene to link", () => {
    scenes = [];
    render(<SceneLink cardId="c1" linked={null} readOnly={false} />);
    expect(screen.getByText(/No screenplay scenes to link yet/)).toBeTruthy();
  });
});
