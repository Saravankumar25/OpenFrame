import { beforeEach, describe, expect, it, vi } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { call } from "../../ipc/client";
import type { DeletedItemView } from "../../ipc/generated/DeletedItemView";
import { answerFrom, renderWithClient } from "../../app/home/testUtils";
import RecentlyDeleted, { restoredMessage } from "./index";

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
  wasIn: "Story Board · Parking Lot",
  parentMissing: false,
  deletedAt: Date.now(),
  deletedByName: "Nisha Verma",
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
    expect(screen.getByText("Story Board · Parking Lot")).toBeInTheDocument();

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
    expect(restoredMessage(item())).toBe("Restored “Maybe Scene 17” to Story Board · Parking Lot");
    // The backend's relocation note wins when present.
    expect(restoredMessage(item(), { restoredNote: "Its original place no longer exists, so it was restored to Unassigned." })).toBe(
      "Restored “Maybe Scene 17”. Its original place no longer exists, so it was restored to Unassigned.",
    );
  });

  it("explains the empty state", async () => {
    callMock.mockImplementation(answerFrom({ "project.deleted_items": [] }) as typeof call);
    renderWithClient(<RecentlyDeleted />);
    expect(await screen.findByText("Nothing has been deleted.")).toBeInTheDocument();
  });
});
