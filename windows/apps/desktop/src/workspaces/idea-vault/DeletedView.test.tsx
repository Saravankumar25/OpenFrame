import { beforeEach, describe, expect, it, vi } from "vitest";
import { screen } from "@testing-library/react";
import { call } from "../../ipc/client";
import type { VaultTrashRow } from "../../ipc/generated/VaultTrashRow";
import { answerFrom, renderWithClient } from "../../app/home/testUtils";
import { DeletedView } from "./DeletedView";

vi.mock("../../ipc/client", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../ipc/client")>()),
  call: vi.fn(),
  onAppEvent: vi.fn(() => Promise.resolve(() => undefined)),
}));

const callMock = vi.mocked(call);
vi.setConfig({ testTimeout: 20_000 });

const row = (id: string, title: string, daysLeft: number): VaultTrashRow => ({
  deleted: {
    id,
    objectType: "vault_item",
    objectId: `o-${id}`,
    tableName: "vault_item",
    title,
    parentType: null,
    parentId: null,
    position: null,
    deletedAt: Date.now(),
    deletedBy: null,
  },
  typeLabel: "Note",
  originalPlace: "Idea Vault",
  daysLeft,
});

describe("Idea Vault Recently Deleted", () => {
  // Braces: a function returned from beforeEach is run as a clean-up hook after each test.
  beforeEach(() => {
    callMock.mockReset();
  });

  it("says how many days each item has left and that items go after 30 days", async () => {
    callMock.mockImplementation(
      answerFrom({ "vault.trash_list": [row("d1", "Harbour at dawn", 30), row("d2", "రైలు స్టేషన్", 1)] }) as typeof call,
    );
    renderWithClient(<DeletedView scope="global" onBack={() => undefined} />);
    expect(await screen.findByText("Harbour at dawn")).toBeInTheDocument();
    expect(screen.getByText("30 days left")).toBeInTheDocument();
    expect(screen.getByText("1 day left")).toBeInTheDocument();
    expect(screen.getByText(/stay here for 30 days/)).toBeInTheDocument();
  });
});
