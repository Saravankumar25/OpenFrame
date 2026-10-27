// Settings > Storage, Global Idea Vault: Locate reads the chosen folder first and says what it found,
// refuses projects and empty folders, and a line offers to add back files left in the vault folder.

import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { call } from "../../ipc/client";
import { toast } from "../../app/toast";
import { answerFrom, renderWithClient, type OpTable } from "../../app/home/testUtils";
import type { StorageInfo } from "../../ipc/generated/StorageInfo";
import type { VaultFolderInspection } from "../../ipc/generated/VaultFolderInspection";
import { VaultActions } from "./VaultActions";

vi.mock("../../ipc/client", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../ipc/client")>()),
  call: vi.fn(),
  revealLocation: vi.fn(async () => undefined),
  onAppEvent: vi.fn(() => Promise.resolve(() => undefined)),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn() }));
vi.mock("../../app/toast", () => ({ toast: { success: vi.fn(), error: vi.fn(), info: vi.fn() } }));

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
  projectDir: null,
};

const FOUND: VaultFolderInspection = {
  kind: "parentOfVault",
  vaultDir: "D:\\OneDrive\\Documents\\OpenFrame\\Global Idea Vault",
  ideas: 42,
  filesPresent: 17,
  filesMissing: 0,
  active: false,
};

function answer(table: OpTable) {
  const inner = answerFrom({ "vault.orphan_files": { count: 0, bytes: 0, samples: [] }, ...table });
  vi.mocked(call).mockImplementation((async (op: string, args: Record<string, unknown> = {}) => {
    calls.push({ op, args });
    return inner(op, args);
  }) as typeof call);
}

async function pickFolder(found: VaultFolderInspection) {
  answer({ "vault.inspect_folder": found, "vault.locate": INFO, "storage.info": INFO });
  vi.mocked(openDialog).mockResolvedValue("D:\\OneDrive\\Documents\\OpenFrame");
  renderWithClient(<VaultActions info={INFO} />);
  fireEvent.click(screen.getByRole("button", { name: "Locate Existing Vault…" }));
  await waitFor(() => expect(calls.some((c) => c.op === "vault.inspect_folder")).toBe(true));
}

beforeEach(() => {
  calls.length = 0;
  vi.mocked(openDialog).mockReset();
  vi.mocked(toast.error).mockReset();
  vi.mocked(toast.info).mockReset();
});

describe("Global Idea Vault actions", () => {
  it("says what Locate found and connects the vault inside the chosen folder", async () => {
    await pickFolder(FOUND);
    expect(await screen.findByText(/Found a vault at D:\\OneDrive\\Documents\\OpenFrame\\Global Idea Vault with 42 ideas and 17 files\./)).toBeInTheDocument();
    expect(calls.some((c) => c.op === "vault.locate")).toBe(false);
    fireEvent.click(screen.getByRole("button", { name: "Connect Vault" }));
    await waitFor(() => expect(calls.find((c) => c.op === "vault.locate")?.args).toEqual({ dir: FOUND.vaultDir }));
  });

  it("refuses a project folder and an empty folder in plain words", async () => {
    await pickFolder({ ...FOUND, kind: "project", vaultDir: null, ideas: 0, filesPresent: 0 });
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith(expect.stringContaining("That folder is a project, not a vault.")));
    calls.length = 0;
    fireEvent.click(screen.getByRole("button", { name: "Locate Existing Vault…" }));
    answer({ "vault.inspect_folder": { ...FOUND, kind: "nothing", vaultDir: null, ideas: 0, filesPresent: 0 } });
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith(expect.stringContaining("There is no Global Idea Vault in that folder.")));
    expect(calls.some((c) => c.op === "vault.locate")).toBe(false);
  });

  it("says when the chosen vault is already the one in use", async () => {
    await pickFolder({ ...FOUND, active: true });
    await waitFor(() => expect(toast.info).toHaveBeenCalledWith("This vault is already the one in use. Nothing was changed."));
    expect(screen.queryByRole("button", { name: "Connect Vault" })).not.toBeInTheDocument();
  });

  it("asks a second time before connecting a vault with no ideas", async () => {
    await pickFolder({ ...FOUND, ideas: 0, filesPresent: 0 });
    const connect = await screen.findByRole("button", { name: "Connect Empty Vault" });
    expect(connect).toBeDisabled();
    fireEvent.click(screen.getByRole("checkbox", { name: "This vault has no ideas. Connect it anyway." }));
    expect(connect).toBeEnabled();
    fireEvent.click(connect);
    await waitFor(() => expect(calls.some((c) => c.op === "vault.locate")).toBe(true));
  });

  it("offers to add back files that are in the vault folder but not in the vault", async () => {
    let count = 3;
    answer({
      "vault.orphan_files": () => ({ count, bytes: 3072, samples: [] }),
      "vault.recover_files": () => {
        const added = count;
        count = 0;
        return { added, failed: 0 };
      },
    });
    renderWithClient(<VaultActions info={INFO} />);
    expect(await screen.findByText(/3 files in the vault folder are not in the vault/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Add Them Back" }));
    await waitFor(() => expect(calls.find((c) => c.op === "vault.recover_files")?.args).toEqual({ store: "global" }));
    await waitFor(() => expect(screen.queryByRole("button", { name: "Add Them Back" })).not.toBeInTheDocument());
  });
});
