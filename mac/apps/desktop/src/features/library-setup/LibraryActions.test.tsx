// Settings > Storage, "Your OpenFrame library": Move Library confirms with both paths and runs as
// a task; Locate Existing Library previews what it found before connecting.

import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, screen } from "@testing-library/react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { call } from "../../ipc/client";
import { answerFrom, renderWithClient, type OpTable } from "../../app/home/testUtils";
import type { StorageInfo } from "../../ipc/generated/StorageInfo";
import { LibraryActions } from "./LibraryActions";

vi.mock("../../ipc/client", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../ipc/client")>()),
  call: vi.fn(),
  onAppEvent: vi.fn(() => Promise.resolve(() => undefined)),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("../../app/toast", () => ({ toast: { success: vi.fn(), error: vi.fn(), info: vi.fn() } }));

const callMock = vi.mocked(call);
const calls: { op: string; args: Record<string, unknown> }[] = [];
const LIB = "C:\\Users\\me\\OpenFrame";
const INFO: StorageInfo = {
  appDataDir: "C:\\Users\\me\\AppData\\Local\\OpenFrame",
  libraryDir: LIB,
  libraryFile: `${LIB}\\library.json`,
  projectsDir: `${LIB}\\Projects`,
  projectsDirIsDefault: true,
  vaultDir: `${LIB}\\Global Idea Vault`,
  vaultDirIsDefault: true,
  vaultExists: true,
  projectDir: null,
};

function answer(table: OpTable) {
  const inner = answerFrom(table);
  callMock.mockImplementation((async (op: string, args: Record<string, unknown> = {}) => {
    calls.push({ op, args });
    return inner(op, args);
  }) as typeof call);
}

beforeEach(() => {
  calls.length = 0;
  vi.mocked(openDialog).mockReset();
});

describe("Your OpenFrame library actions", () => {
  it("moves the library only after a confirmation that names both places", async () => {
    vi.mocked(openDialog).mockResolvedValueOnce("D:\\");
    answer({ "library.move": { taskId: "t1" } });
    renderWithClient(<LibraryActions info={INFO} />);
    fireEvent.click(screen.getByRole("button", { name: "Move Library…" }));
    const dialog = await screen.findByRole("dialog", { name: "Move Your OpenFrame Library" });
    expect(dialog).toHaveTextContent(`From ${LIB}`);
    expect(dialog).toHaveTextContent("To D:\\OpenFrame");
    expect(dialog).toHaveTextContent(/the Global Idea Vault/);
    expect(calls).toEqual([]);
    fireEvent.click(screen.getByRole("button", { name: "Move Library" }));
    await screen.findByRole("dialog", { name: "Move Your OpenFrame Library" });
    expect(calls.find((c) => c.op === "library.move")?.args).toEqual({ destParent: "D:\\" });
  });

  it("previews an existing library before connecting it", async () => {
    vi.mocked(openDialog).mockResolvedValueOnce("E:\\Backup\\OpenFrame");
    answer({
      "library.inspect": {
        path: "E:\\Backup\\OpenFrame",
        kind: "libraryRoot",
        libraryRoot: "E:\\Backup\\OpenFrame",
        hasLibraryFile: true,
        ownerName: "Meera",
        projectCount: 2,
        vaultFound: true,
        ideaCount: 5,
        cloudService: null,
        message: "This is an OpenFrame library with 2 projects and a Global Idea Vault with 5 ideas.",
      },
      "library.connect": { state: "ready", root: "E:\\Backup\\OpenFrame", suggested: LIB, cloudService: null, found: [] },
    });
    renderWithClient(<LibraryActions info={INFO} />);
    fireEvent.click(screen.getByRole("button", { name: "Locate Existing Library…" }));
    const dialog = await screen.findByRole("dialog", { name: "Connect This Library?" });
    expect(dialog).toHaveTextContent(/2 projects and a Global Idea Vault with 5 ideas/);
    expect(dialog).toHaveTextContent(/is not moved or deleted/);
    expect(calls.map((c) => c.op)).toEqual(["library.inspect"]);
    fireEvent.click(screen.getByRole("button", { name: "Connect Library" }));
    await vi.waitFor(() => expect(calls.find((c) => c.op === "library.connect")?.args).toEqual({ dir: "E:\\Backup\\OpenFrame" }));
  });
});
