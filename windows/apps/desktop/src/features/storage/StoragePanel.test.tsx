// Settings > Storage: every place is shown with plain words, changing the folder for new projects
// says that existing projects stay put, and vault actions confirm before anything changes.

import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { call } from "../../ipc/client";
import { answerFrom, renderWithClient, type OpTable } from "../../app/home/testUtils";
import type { StorageInfo } from "../../ipc/generated/StorageInfo";
import { StoragePanel } from "./StoragePanel";
import { FoundProjects } from "../../app/home/FoundProjects";

vi.mock("../../ipc/client", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../ipc/client")>()),
  call: vi.fn(),
  revealLocation: vi.fn(async () => undefined),
  onAppEvent: vi.fn(() => Promise.resolve(() => undefined)),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn() }));
vi.mock("../../app/toast", () => ({ toast: { success: vi.fn(), error: vi.fn(), info: vi.fn() } }));

const callMock = vi.mocked(call);
const calls: { op: string; args: Record<string, unknown> }[] = [];

const INFO: StorageInfo = {
  appDataDir: "C:\\Users\\me\\AppData\\Local\\OpenFrame",
  libraryDir: "C:\\Users\\me\\Documents\\OpenFrame",
  libraryFile: "C:\\Users\\me\\Documents\\OpenFrame\\library.json",
  projectsDir: "C:\\Users\\me\\Documents\\OpenFrame\\Projects",
  projectsDirIsDefault: true,
  vaultDir: "C:\\Users\\me\\Documents\\OpenFrame\\Global Idea Vault",
  vaultDirIsDefault: true,
  vaultExists: true,
  projectDir: "D:\\Films\\Night Bus.openframe",
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
  vi.mocked(saveDialog).mockReset();
});

describe("Settings > Storage", () => {
  it("shows the library, the folder for new projects, the vault, this project and the app data", async () => {
    answer({ "storage.info": INFO, "project.orphan_notes": [] });
    renderWithClient(<StoragePanel />);
    for (const title of ["Your OpenFrame library", "Folder for new projects", "Global Idea Vault", "This project", "App data"]) {
      expect(await screen.findByRole("region", { name: title })).toBeInTheDocument();
    }
    expect(screen.getByText(/Changing it does not move the projects you already have/)).toBeInTheDocument();
    for (const name of ["Back Up…", "Export Package…", "Restore…", "Move…", "Locate Existing Vault…", "Move Project…"]) {
      expect(screen.getByRole("button", { name })).toBeEnabled();
    }
  });

  it("changes the folder for new projects only", async () => {
    answer({ "storage.info": INFO, "project.orphan_notes": [], "storage.set_projects_dir": INFO });
    vi.mocked(openDialog).mockResolvedValue("E:\\New Films");
    renderWithClient(<StoragePanel />);
    fireEvent.click(await screen.findByRole("button", { name: "Change Folder…" }));
    await waitFor(() => expect(calls.some((c) => c.op === "storage.set_projects_dir")).toBe(true));
    expect(calls.find((c) => c.op === "storage.set_projects_dir")?.args).toEqual({ path: "E:\\New Films" });
    expect(calls.some((c) => c.op.includes("move"))).toBe(false);
  });

  it("reads a vault file and asks before restoring it", async () => {
    answer({
      "storage.info": INFO,
      "project.orphan_notes": [],
      "vault.inspect": { fileName: "old.ofvault", kind: "backup", createdAt: 1, appVersion: "0.1.0", ideaCount: 12, assetCount: 3, bytes: 2048 },
    });
    vi.mocked(openDialog).mockResolvedValue("D:\\old.ofvault");
    renderWithClient(<StoragePanel />);
    fireEvent.click(await screen.findByRole("button", { name: "Restore…" }));
    expect(await screen.findByText(/12 items and 3 files/)).toBeInTheDocument();
    expect(screen.getByText(/kept beside it/)).toBeInTheDocument();
    expect(calls.some((c) => c.op === "vault.restore_backup")).toBe(false);
    expect(screen.getByRole("button", { name: "Replace and Restore" })).toBeInTheDocument();
  });

  it("offers a vault that does not exist yet only Restore and Locate", async () => {
    answer({ "storage.info": { ...INFO, vaultExists: false, projectDir: null } });
    renderWithClient(<StoragePanel />);
    expect(await screen.findByText(/Not created yet/)).toBeInTheDocument();
    for (const name of ["Back Up…", "Export Package…", "Move…"]) expect(screen.getByRole("button", { name })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Restore…" })).toBeEnabled();
    expect(screen.queryByRole("region", { name: "This project" })).toBeNull();
  });

  it("lists the private notes of a former owner and asks before taking them over", async () => {
    answer({ "storage.info": INFO, "project.orphan_notes": [{ ownerUserId: "old-me", noteCount: 4 }], "project.adopt_orphan_notes": { adopted: 4 } });
    renderWithClient(<StoragePanel />);
    expect(await screen.findByText("4 private notes")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Make Them Mine…" }));
    fireEvent.click(await screen.findByRole("button", { name: "Make Them Mine" }));
    await waitFor(() => expect(calls.find((c) => c.op === "project.adopt_orphan_notes")?.args).toEqual({ ownerUserId: "old-me" }));
  });
});

describe("Home: found on this PC", () => {
  it("lists projects in the projects folder that are not in the list, and opens one", async () => {
    const opened = vi.fn();
    answer({
      "project.discover": [{ path: "C:\\P\\Lost.openframe", title: "Lost Film", projectId: "p9", modifiedAt: 1_700_000_000_000 }],
      "project.open": {},
    });
    renderWithClient(<FoundProjects onOpened={opened} />);
    expect(await screen.findByText("Lost Film")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Open Lost Film" }));
    await waitFor(() => expect(opened).toHaveBeenCalled());
    expect(calls.find((c) => c.op === "project.open")?.args).toEqual({ path: "C:\\P\\Lost.openframe" });
  });

  it("shows nothing when every project is already listed", async () => {
    answer({ "project.discover": [] });
    const { container } = renderWithClient(<FoundProjects onOpened={() => undefined} />);
    await waitFor(() => expect(calls.length).toBe(1));
    expect(container.textContent).toBe("");
  });
});
