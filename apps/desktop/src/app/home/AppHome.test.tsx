import { beforeEach, describe, expect, it, vi } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { act } from "react";
import { call, onAppEvent } from "../../ipc/client";
import { installInvalidation } from "../../ipc/query";
import type { AppEvent } from "../../ipc/generated/AppEvent";
import { AppHome } from "./AppHome";
import { answerFrom, renderWithClient, type OpTable } from "./testUtils";
import type { RecentProject } from "../../ipc/generated/RecentProject";
import { useIntent, useNav, useUi } from "../stores";
import { useHomeView } from "./homeState";
import { usePackagesUi } from "../../features/packages";

vi.mock("../../ipc/client", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../ipc/client")>()),
  call: vi.fn(),
  onAppEvent: vi.fn(() => Promise.resolve(() => undefined)),
}));

// The real Idea Vault workspace is tested on its own; here a stub stands in for it.
vi.mock("../routes", () => ({
  WORKSPACES: [],
  workspaceDef: () => ({ id: "vault", label: "Idea Vault", nav: true, component: () => <div>Vault workspace</div> }),
}));

const callMock = vi.mocked(call);
const onAppEventMock = vi.mocked(onAppEvent);
// Interaction tests (dialogs, typing) can be slow on a busy machine.
vi.setConfig({ testTimeout: 20_000 });

const info = (displayNameSet = true) => ({
  appVersion: "0.1.0",
  projectsDir: "C:\\Users\\me\\Documents\\OpenFrame\\Projects",
  globalVaultDir: "C:\\vault",
  profile: { userId: "u1", displayName: "Nisha Verma" },
  platform: "windows",
  schemaVersion: 9,
  displayNameSet,
});

function project(p: Partial<RecentProject>): RecentProject {
  return {
    projectId: "p1",
    path: "C:\\Films\\BLACK RAIN",
    title: "BLACK RAIN",
    projectType: "Feature Film",
    status: "Writing",
    archived: false,
    pinned: false,
    lastOpenedAt: Date.now() - 3_600_000,
    modifiedAt: Date.now() - 7_200_000,
    available: true,
    ...p,
  };
}

function setup(table: OpTable) {
  callMock.mockImplementation(answerFrom(table) as typeof call);
  return renderWithClient(<AppHome />);
}

describe("Application Home", () => {
  beforeEach(() => {
    callMock.mockReset();
    useHomeView.setState({ view: "projects", showArchived: false });
    useUi.setState({ undoScope: "project" });
    useIntent.setState({ intent: null });
    useNav.getState().reset();
    usePackagesUi.setState({ dialog: null, taskId: null });
  });

  it("shows the empty state with the two first actions when there are no projects", async () => {
    setup({ "project.list_recent": [], "app.info": info() });
    expect(await screen.findByText("Create your first project.")).toBeInTheDocument();
    expect(screen.getByText(/A project holds your ideas, beat board, screenplay and production plan/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Create Project" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Open Existing Project" })).toBeInTheDocument();
    // No project grid, no production metadata on the empty Home.
    expect(screen.queryByText("Recent projects")).not.toBeInTheDocument();
    expect(screen.getByText("No project open")).toBeInTheDocument();
    // One Settings button replaces the old shortcuts and appearance buttons (owner item 6).
    expect(screen.getByRole("button", { name: "Settings" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Help" })).not.toBeInTheDocument();
  });

  it("offers Restore Backup or Package on an empty Home (a new computer) and on the project list", async () => {
    const user = userEvent.setup();
    setup({ "project.list_recent": [], "app.info": info(), "project.current": null });
    await user.click(await screen.findByRole("button", { name: "Restore Backup or Package…" }));
    expect(await screen.findByRole("dialog", { name: /Open Project Package/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Choose Package…" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });

  it("keeps Restore Backup or Package next to Open Project when projects exist", async () => {
    setup({ "project.list_recent": [project({})], "app.info": info(), "project.current": null });
    expect(await screen.findByRole("button", { name: "Restore Backup or Package…" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Open Project" })).toBeInTheDocument();
  });

  it("opens the New Project dialog from the empty state and with Ctrl+N", async () => {
    const user = userEvent.setup();
    setup({ "project.list_recent": [], "app.info": info() });
    await user.click(await screen.findByRole("button", { name: "Create Project" }));
    expect(await screen.findByRole("heading", { name: "New Project" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(screen.queryByRole("heading", { name: "New Project" })).not.toBeInTheDocument());
    await user.keyboard("{Control>}n{/Control}");
    expect(await screen.findByRole("heading", { name: "New Project" })).toBeInTheDocument();
  });

  it("lists pinned projects before recent ones and hides archived ones", async () => {
    setup({
      "project.list_recent": [
        project({ projectId: "a", title: "The Last Bus", pinned: false }),
        project({ projectId: "b", title: "BLACK RAIN", pinned: true }),
        project({ projectId: "c", title: "Old Town Blues", archived: true }),
      ],
      "app.info": info(),
    });
    expect(await screen.findByRole("region", { name: "Pinned" })).toBeInTheDocument();
    const cards = screen.getAllByRole("button", { name: /^Open .+, Feature Film, / });
    expect(cards[0]).toHaveAccessibleName(/BLACK RAIN/);
    expect(cards[1]).toHaveAccessibleName(/The Last Bus/);
    expect(screen.queryByText("Old Town Blues")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Open Archived \(1\)/ })).toBeInTheDocument();
  });

  it("asks for the user's name on first run and stores it", async () => {
    const user = userEvent.setup();
    const setName = vi.fn(() => ({ userId: "u1", displayName: "Rohan Das" }));
    setup({ "project.list_recent": [], "app.info": info(false), "app.set_display_name": setName });
    expect(await screen.findByRole("heading", { name: "Welcome to OpenFrame Studio" })).toBeInTheDocument();
    const field = screen.getByRole("textbox", { name: /Your name/ });
    await user.clear(field);
    await user.type(field, "Rohan Das");
    await user.click(screen.getByRole("button", { name: "Continue" }));
    await waitFor(() => expect(setName).toHaveBeenCalledWith({ displayName: "Rohan Das" }));
  });

  it("opens the Global Idea Vault with no project open, with global undo, and returns", async () => {
    const user = userEvent.setup();
    setup({ "project.list_recent": [project({})], "app.info": info() });
    await user.click(await screen.findByRole("button", { name: "Global Idea Vault" }));
    expect(await screen.findByText("Vault workspace")).toBeInTheDocument();
    expect(useUi.getState().undoScope).toBe("global");
    expect(useNav.getState().route).toMatchObject({ workspace: "vault", params: { scope: "global" } });
    await user.click(screen.getByRole("button", { name: "Your projects" }));
    expect(await screen.findByRole("heading", { name: "Your projects" })).toBeInTheDocument();
    expect(useUi.getState().undoScope).toBe("project");
  });

  it("updates the card in place when Rename emits the project-list change event", async () => {
    const user = userEvent.setup();
    let emit: (e: AppEvent) => void = () => undefined;
    onAppEventMock.mockImplementation((handler) => {
      emit = handler;
      return Promise.resolve(() => undefined);
    });
    let title = "Harbour Lights";
    const rename = vi.fn((a: Record<string, unknown>) => {
      title = String(a.title);
      return project({ projectId: "p1", title });
    });
    const { client } = setup({
      "project.list_recent": () => [project({ projectId: "p1", title })],
      "project.rename": rename,
      "app.info": info(),
    });
    installInvalidation(client);
    await user.click(await screen.findByRole("button", { name: "More actions for Harbour Lights" }));
    await user.click(await screen.findByRole("menuitem", { name: /Rename/ }));
    const field = await screen.findByRole("textbox", { name: /Title/ });
    await user.clear(field);
    await user.type(field, "Harbour Lights Redux");
    await user.click(screen.getByRole("button", { name: "Rename" }));
    await waitFor(() => expect(rename).toHaveBeenCalledWith({ projectId: "p1", title: "Harbour Lights Redux" }));
    // What the Rust core emits after a rename (project.rs `recent_list_changed`).
    act(() => emit({ type: "dataChanged", store: "project", tables: ["sys_recent_project"], ids: ["p1"], origin: "local" }));
    expect(await screen.findByRole("button", { name: "Open Harbour Lights Redux, Feature Film, Writing" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /^Open Harbour Lights, / })).not.toBeInTheDocument();
  });

  it("starts a Quick Capture in the Global Idea Vault with Ctrl+Shift+N", async () => {
    const user = userEvent.setup();
    setup({ "project.list_recent": [], "app.info": info() });
    await screen.findByText("Create your first project.");
    await user.keyboard("{Control>}{Shift>}N{/Shift}{/Control}");
    expect(await screen.findByText("Vault workspace")).toBeInTheDocument();
    expect(useIntent.getState().intent).toBe("vault.new_note");
    expect(useUi.getState().undoScope).toBe("global");
  });
});
