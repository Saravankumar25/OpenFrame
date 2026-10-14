import { beforeEach, describe, expect, it, vi } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { call, OpError } from "../../ipc/client";
import { NewProjectDialog, titleError, toProjectType } from "./NewProjectDialog";
import { answerFrom, renderWithClient } from "./testUtils";

vi.mock("../../ipc/client", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../ipc/client")>()),
  call: vi.fn(),
  onAppEvent: vi.fn(() => Promise.resolve(() => undefined)),
}));

const callMock = vi.mocked(call);
// Interaction tests (dialogs, typing) can be slow on a busy machine.
vi.setConfig({ testTimeout: 20_000 });
const info = { profile: { userId: "u1", displayName: "Me" }, projectsDir: "C:\\Projects", displayNameSet: true };

describe("New Project validation", () => {
  beforeEach(() => {
    callMock.mockReset();
  });

  it("maps the type choices to the four project types", () => {
    expect(toProjectType("Feature Film", "Episodic")).toBe("Feature Film");
    expect(toProjectType("Short Film", "Series")).toBe("Short Film");
    expect(toProjectType("Episodic / Series", "Episodic")).toBe("Episodic");
    expect(toProjectType("Episodic / Series", "Series")).toBe("Series");
  });

  it("requires a title", () => {
    expect(titleError("")).toBe("A title is required.");
    expect(titleError("   ")).toBe("A title is required.");
    expect(titleError("BLACK RAIN")).toBeNull();
    expect(titleError("x".repeat(201))).toMatch(/too long/);
  });

  it("keeps Create disabled until a title is entered and explains why", async () => {
    const user = userEvent.setup();
    callMock.mockImplementation(answerFrom({ "app.info": info }) as typeof call);
    renderWithClient(<NewProjectDialog onClose={() => undefined} />);
    const create = screen.getByRole("button", { name: "Create" });
    expect(create).toBeDisabled();
    const title = screen.getByLabelText(/Title/);
    await user.type(title, "   ");
    await user.tab();
    expect(await screen.findByText("A title is required.")).toBeInTheDocument();
    expect(title).toHaveAttribute("aria-invalid", "true");
    expect(create).toBeDisabled();
    await user.clear(title);
    await user.type(title, "BLACK RAIN");
    expect(screen.queryByText("A title is required.")).not.toBeInTheDocument();
    expect(create).toBeEnabled();
  });

  it("creates a Series project with only the required fields", async () => {
    const user = userEvent.setup();
    const created = vi.fn(() => ({ project: { id: "p", title: "Monsoon Diaries" }, recovery: null, migratedFrom: null }));
    callMock.mockImplementation(answerFrom({ "app.info": info, "project.create": created }) as typeof call);
    const onClose = vi.fn();
    renderWithClient(<NewProjectDialog onClose={onClose} />);
    await user.type(screen.getByLabelText(/Title/), "  Monsoon Diaries ");
    await user.click(screen.getByRole("radio", { name: "Episodic / Series" }));
    await user.click(screen.getByLabelText(/^Series/));
    await user.click(screen.getByRole("button", { name: "Create" }));
    await waitFor(() => expect(created).toHaveBeenCalled());
    expect(created).toHaveBeenCalledWith(
      expect.objectContaining({ title: "Monsoon Diaries", projectType: "Series", language: null, genre: null, creator: null }),
    );
    await waitFor(() => expect(onClose).toHaveBeenCalled());
  });

  it("keeps the entries and offers Retry when creation fails", async () => {
    const user = userEvent.setup();
    callMock.mockImplementation(async (op: string) => {
      if (op === "app.info") return info;
      throw new OpError("project.create", { code: "storage.disk_full", message: "The disk is full.", retryable: true });
    });
    renderWithClient(<NewProjectDialog onClose={() => undefined} />);
    await user.type(screen.getByLabelText(/Title/), "BLACK RAIN");
    await user.click(screen.getByRole("button", { name: "Create" }));
    expect(await screen.findByText(/The project could not be created. Nothing was saved and no partial project was made/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Retry" })).toBeEnabled();
    expect(screen.getByLabelText(/Title/)).toHaveValue("BLACK RAIN");
  });
});
