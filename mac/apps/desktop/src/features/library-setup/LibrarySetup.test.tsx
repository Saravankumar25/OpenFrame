// The library setup screen: shown before Home until a choice works, three choices, a preview
// before connecting, a failure that stays on the screen, and libraries found on this computer.

import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { call } from "../../ipc/client";
import { answerFrom, renderWithClient, type OpTable } from "../../app/home/testUtils";
import type { LibraryInspection } from "../../ipc/generated/LibraryInspection";
import type { LibraryStatus } from "../../ipc/generated/LibraryStatus";
import { LibrarySetupGate } from "./LibrarySetup";

vi.mock("../../ipc/client", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../ipc/client")>()),
  call: vi.fn(),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

const callMock = vi.mocked(call);
const calls: { op: string; args: Record<string, unknown> }[] = [];

const HOME = "C:\\Users\\me\\OpenFrame";
const NEEDS: LibraryStatus = { state: "needsSetup", root: null, suggested: HOME, cloudService: null, found: [] };
const READY: LibraryStatus = { state: "ready", root: HOME, suggested: HOME, cloudService: null, found: [] };
const DOCS: LibraryInspection = {
  path: "C:\\Users\\me\\Documents\\OpenFrame",
  kind: "libraryRoot",
  libraryRoot: "C:\\Users\\me\\Documents\\OpenFrame",
  hasLibraryFile: true,
  ownerName: "Meera",
  projectCount: 3,
  vaultFound: true,
  ideaCount: 12,
  cloudService: "OneDrive",
  message: "This is an OpenFrame library with 3 projects and a Global Idea Vault with 12 ideas.",
};

function answer(table: OpTable) {
  const inner = answerFrom(table);
  callMock.mockImplementation((async (op: string, args: Record<string, unknown> = {}) => {
    calls.push({ op, args });
    return inner(op, args);
  }) as typeof call);
}

const show = () => renderWithClient(<LibrarySetupGate><div>Home screen</div></LibrarySetupGate>);

beforeEach(() => {
  calls.length = 0;
  vi.mocked(openDialog).mockReset();
});

describe("Set up your OpenFrame library", () => {
  it("opens straight away when the library is ready", async () => {
    answer({ "library.status": READY });
    show();
    expect(await screen.findByText("Home screen")).toBeInTheDocument();
  });

  it("offers three choices and creates the default only on confirm", async () => {
    answer({ "library.status": NEEDS, "library.use_default": READY });
    show();
    expect(await screen.findByRole("heading", { name: "Set up your OpenFrame library" })).toBeInTheDocument();
    for (const name of ["Use the default location", "Choose a new location", "Locate existing OpenFrame data"]) {
      expect(screen.getByRole("radio", { name })).toBeInTheDocument();
    }
    expect(screen.getByText(HOME)).toBeInTheDocument();
    expect(screen.getByText(/outside Documents/)).toBeInTheDocument();
    expect(screen.queryByText("Home screen")).not.toBeInTheDocument();
    expect(calls.map((c) => c.op)).toEqual(["library.status"]);
    fireEvent.click(screen.getByRole("button", { name: "Create Library Here" }));
    expect(await screen.findByText("Home screen")).toBeInTheDocument();
    expect(calls.map((c) => c.op)).toContain("library.use_default");
  });

  it("shows the final path of a new location before creating it", async () => {
    vi.mocked(openDialog).mockResolvedValueOnce("D:\\Work");
    answer({
      "library.status": NEEDS,
      "library.inspect": { ...DOCS, kind: "empty", libraryRoot: null, cloudService: null, message: "This folder is empty." },
      "library.create_at": READY,
    });
    show();
    fireEvent.click(await screen.findByRole("radio", { name: "Choose a new location" }));
    fireEvent.click(screen.getByRole("button", { name: "Choose Folder…" }));
    expect(await screen.findByText("D:\\Work\\OpenFrame")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Create Library" }));
    expect(await screen.findByText("Home screen")).toBeInTheDocument();
    expect(calls.find((c) => c.op === "library.create_at")?.args).toEqual({ parent: "D:\\Work" });
  });

  it("previews what Locate found, and a failed connect stays on the screen", async () => {
    vi.mocked(openDialog).mockResolvedValueOnce("C:\\Users\\me\\Documents");
    answer({
      "library.status": NEEDS,
      "library.inspect": { ...DOCS, kind: "libraryParent" },
      "library.connect": () => {
        throw Object.assign(new Error("That folder can't be found."), { code: "not_found.folder" });
      },
    });
    show();
    fireEvent.click(await screen.findByRole("radio", { name: "Locate existing OpenFrame data" }));
    fireEvent.click(screen.getByRole("button", { name: "Choose Folder…" }));
    expect(await screen.findByText(/3 projects and a Global Idea Vault with 12 ideas/)).toBeInTheDocument();
    expect(screen.getByText(/It belongs to Meera/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Connect Library" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("That folder can't be found.");
    expect(screen.queryByText("Home screen")).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Set up your OpenFrame library" })).toBeInTheDocument();
    expect(calls.find((c) => c.op === "library.connect")?.args).toEqual({ dir: DOCS.libraryRoot });
  });

  it("explains a folder with no library and offers nothing to connect", async () => {
    vi.mocked(openDialog).mockResolvedValueOnce("C:\\Temp");
    answer({
      "library.status": NEEDS,
      "library.inspect": { ...DOCS, kind: "project", libraryRoot: null, message: "This folder is one project, not a library." },
    });
    show();
    fireEvent.click(await screen.findByRole("radio", { name: "Locate existing OpenFrame data" }));
    fireEvent.click(screen.getByRole("button", { name: "Choose Folder…" }));
    expect(await screen.findByText(/one project, not a library/)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Connect Library" })).not.toBeInTheDocument();
  });

  it("lists libraries found on this computer first", async () => {
    answer({ "library.status": { ...NEEDS, found: [DOCS] }, "library.connect": READY });
    show();
    const region = await screen.findByRole("region", { name: "OpenFrame data found on this computer" });
    expect(region).toHaveTextContent(DOCS.path);
    expect(region).toHaveTextContent(/synced by OneDrive/);
    fireEvent.click(screen.getByRole("button", { name: "Use This Library" }));
    expect(await screen.findByText("Home screen")).toBeInTheDocument();
    expect(calls.find((c) => c.op === "library.connect")?.args).toEqual({ dir: DOCS.libraryRoot });
  });

  it("says where a missing library was expected and can try again", async () => {
    const expected = "E:\\OpenFrame";
    answer({ "library.status": { ...NEEDS, state: "missing", root: expected }, "library.connect": READY });
    show();
    expect(await screen.findByText(/Your library was not found at E:\\OpenFrame/)).toBeInTheDocument();
    for (const name of ["Use the default location", "Choose a new location", "Locate existing OpenFrame data"]) {
      expect(screen.getByRole("radio", { name })).toBeInTheDocument();
    }
    fireEvent.click(screen.getByRole("button", { name: "Try Again" }));
    await waitFor(() => expect(screen.getByText("Home screen")).toBeInTheDocument());
    expect(calls.find((c) => c.op === "library.connect")?.args).toEqual({ dir: expected });
  });
});
