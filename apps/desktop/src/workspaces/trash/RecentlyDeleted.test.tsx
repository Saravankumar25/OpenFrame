import { beforeEach, describe, expect, it, vi } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { call } from "../../ipc/client";
import type { DeletedItemView } from "../../ipc/generated/DeletedItemView";
import { answerFrom, renderWithClient } from "../../app/home/testUtils";
import RecentlyDeleted, { parentMissingNote, restoredMessage } from "./index";

vi.mock("../../ipc/client", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../ipc/client")>()),
  call: vi.fn(),
  onAppEvent: vi.fn(() => Promise.resolve(() => undefined)),
}));

const callMock = vi.mocked(call);
// Interaction tests (dialogs, typing) can be slow on a busy machine.
vi.setConfig({ testTimeout: 20_000 });

const item = (p: Partial<DeletedItemView> = {}): DeletedItemView => ({
  id: "d1",
  objectType: "story_scene_card",
  objectId: "o1",
  title: "Maybe Scene 17",
  typeLabel: "Scene Card",
  wasIn: "Beat Board · Parking Lot",
  parentMissing: false,
  deletedAt: Date.now(),
  deletedByName: "Nisha Verma",
  daysLeft: 30,
  ...p,
});

describe("Recently Deleted", () => {
  beforeEach(() => {
    callMock.mockReset();
  });

  it("asks for a second, deliberate confirmation before deleting permanently", async () => {
    const user = userEvent.setup();
    const purge = vi.fn(() => item());
    callMock.mockImplementation(answerFrom({ "project.deleted_items": [item()], "trash.purge": purge }) as typeof call);
    renderWithClient(<RecentlyDeleted />);
    expect(await screen.findByText("Maybe Scene 17")).toBeInTheDocument();
    expect(screen.getByText("Beat Board · Parking Lot")).toBeInTheDocument();

    // First click only opens the confirmation.
    await user.click(screen.getByRole("button", { name: "Delete Maybe Scene 17 permanently" }));
    expect(await screen.findByRole("heading", { name: "Delete “Maybe Scene 17” permanently?" })).toBeInTheDocument();
    expect(screen.getByText(/This can't be undone/)).toBeInTheDocument();
    expect(purge).not.toHaveBeenCalled();

    // Backing out keeps the item.
    await user.click(screen.getByRole("button", { name: "Keep in Recently Deleted" }));
    await waitFor(() => expect(screen.queryByRole("heading", { name: /permanently\?/ })).not.toBeInTheDocument());
    expect(purge).not.toHaveBeenCalled();

    // The deliberate second action deletes it.
    await user.click(screen.getByRole("button", { name: "Delete Maybe Scene 17 permanently" }));
    await user.click(await screen.findByRole("button", { name: "Delete Permanently" }));
    await waitFor(() => expect(purge).toHaveBeenCalledWith({ store: "project", id: "d1" }));
  });

  it("restores and says where the item went, including Unassigned", async () => {
    const user = userEvent.setup();
    const restore = vi.fn(() => item());
    callMock.mockImplementation(
      answerFrom({ "project.deleted_items": [item({ parentMissing: true })], "trash.restore": restore }) as typeof call,
    );
    renderWithClient(<RecentlyDeleted />);
    expect(await screen.findByText(/restores to Unassigned/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Restore Maybe Scene 17" }));
    await waitFor(() => expect(restore).toHaveBeenCalledWith({ store: "project", id: "d1" }));
    expect(restoredMessage(item({ parentMissing: true }))).toMatch(/put in Unassigned/);
    expect(restoredMessage(item())).toBe("Restored “Maybe Scene 17” to Beat Board · Parking Lot");
    // The backend's relocation note wins when present.
    expect(restoredMessage(item(), { restoredNote: "Its original place no longer exists, so it was restored to Unassigned." })).toBe(
      "Restored “Maybe Scene 17”. Its original place no longer exists, so it was restored to Unassigned.",
    );
  });

  it("says how many days each item has left before it is removed", async () => {
    callMock.mockImplementation(
      answerFrom({ "project.deleted_items": [item(), item({ id: "d2", objectId: "o2", title: "Old note", daysLeft: 1 })] }) as typeof call,
    );
    renderWithClient(<RecentlyDeleted />);
    expect(await screen.findByText("30 days left")).toBeInTheDocument();
    expect(screen.getByText("1 day left")).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: "Removed in" })).toBeInTheDocument();
    expect(screen.getByText(/stay here for 30 days/)).toBeInTheDocument();
  });

  it("lists a deleted screenplay and restores it, saying when its title had to change", async () => {
    const user = userEvent.setup();
    const screenplay = item({ id: "d7", objectType: "screenplay", objectId: "s2", title: "The Short", typeLabel: "Screenplay", wasIn: "Screenplay" });
    const draft = item({ id: "d8", objectType: "screenplay_draft", objectId: "dr3", title: "Draft 3", typeLabel: "Screenplay draft", wasIn: "Screenplay “The Short”", parentMissing: true });
    const restore = vi.fn(() => ({ restoredNote: "Another screenplay is now called “The Short”, so this one was restored as “The Short (2)”." }));
    callMock.mockImplementation(answerFrom({ "project.deleted_items": [screenplay, draft], "trash.restore": restore }) as typeof call);
    renderWithClient(<RecentlyDeleted />);
    expect(await screen.findByText("The Short")).toBeInTheDocument();
    expect(screen.getByText("Its screenplay is in Recently Deleted too. Restore the screenplay first.")).toBeInTheDocument();
    const button = screen.getByRole("button", { name: "Restore The Short" });
    button.focus();
    await user.keyboard("{Enter}");
    await waitFor(() => expect(restore).toHaveBeenCalledWith({ store: "project", id: "d7" }));
    expect(restoredMessage(screenplay, { restoredNote: "Another screenplay is now called “The Short”, so this one was restored as “The Short (2)”." })).toBe(
      "Restored “The Short”. Another screenplay is now called “The Short”, so this one was restored as “The Short (2)”.",
    );
    expect(restoredMessage(screenplay)).toBe("Restored “The Short” to Screenplay");
    expect(parentMissingNote({ objectType: "screenplay" })).toMatch(/Restore the episode first/);
    expect(parentMissingNote({ objectType: "story_scene_card" })).toMatch(/Unassigned/);
  });

  it("explains the empty state", async () => {
    callMock.mockImplementation(answerFrom({ "project.deleted_items": [] }) as typeof call);
    renderWithClient(<RecentlyDeleted />);
    expect(await screen.findByText("Nothing has been deleted.")).toBeInTheDocument();
  });
});
