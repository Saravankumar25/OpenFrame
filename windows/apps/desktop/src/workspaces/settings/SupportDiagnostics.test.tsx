import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SupportSection, formatBytes } from "./SupportDiagnostics";

// Test fixtures only: the IPC boundary and the system Save dialog are replaced.
const calls: { op: string; args: Record<string, unknown> }[] = [];
let previewFail: { code: string; message: string } | null = null;
let exportFail: { code: string; message: string } | null = null;
const saveMock = vi.fn(async (_o: { defaultPath?: string; filters?: unknown }) => "C:\\Support\\OpenFrame Diagnostics 2026-10-05.zip" as string | null);
const reveal = vi.fn(async (..._a: unknown[]) => undefined);

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(async () => null),
  save: (o: { defaultPath?: string }) => saveMock(o),
}));

vi.mock("../../ipc/client", async () => {
  class OpError extends Error {
    code: string;
    retryable = false;
    constructor(code: string, message: string) {
      super(message);
      this.code = code;
    }
    is(p: string) {
      return this.code === p || this.code.startsWith(p + ".");
    }
  }
  return {
    OpError,
    inTauri: () => false,
    onAppEvent: () => Promise.resolve(() => undefined),
    revealLocation: (...a: unknown[]) => reveal(...a),
    call: vi.fn(async (op: string, args: Record<string, unknown> = {}) => {
      calls.push({ op, args });
      if (op === "diagnostics.preview") {
        if (previewFail) throw new OpError(previewFail.code, previewFail.message);
        return {
          suggestedFileName: "OpenFrame Diagnostics 2026-10-05.zip",
          files: [
            { name: "manifest.json", description: "List of every file in this bundle.", bytes: 812, content: '{ "format": "openframe-diagnostics" }' },
            { name: "log-tail.txt", description: "The most recent log entries.", bytes: 2048, content: "INFO operation failed op=notes.create code=validation.invalid_input" },
          ],
          neverIncluded: ["Screenplay text, notes, comments and any other project content", "Passwords, keys and tokens"],
        };
      }
      if (op === "diagnostics.export_bundle") {
        if (exportFail) throw new OpError(exportFail.code, exportFail.message);
        return {
          path: args.path,
          fileName: "OpenFrame Diagnostics 2026-10-05.zip",
          bytesWritten: 4096,
          files: ["manifest.json", "log-tail.txt"],
          summary: "Saved.",
        };
      }
      return null;
    }),
  };
});

beforeEach(() => {
  calls.length = 0;
  previewFail = null;
  exportFail = null;
  saveMock.mockClear();
  reveal.mockClear();
});

async function openDialog() {
  const user = userEvent.setup();
  render(<SupportSection />);
  // Keyboard route: Tab to the action and press Enter.
  await user.tab();
  const button = screen.getByRole("button", { name: "Export Diagnostics…" });
  expect(button).toHaveFocus();
  await user.keyboard("{Enter}");
  const dialog = await screen.findByRole("dialog", { name: "Export Diagnostics" });
  return { user, dialog };
}

describe("Support > Export Diagnostics", () => {
  it("sits under a Support heading and shows every file before anything is saved", async () => {
    const { user, dialog } = await openDialog();
    // The open dialog is modal, so the page behind it is hidden from assistive technology; query with hidden.
    expect(screen.getByRole("heading", { name: "Support", hidden: true })).toBeInTheDocument();
    expect(within(dialog).getByText(/Nothing is sent automatically/)).toBeInTheDocument();
    await within(dialog).findByText("manifest.json");
    expect(within(dialog).getByText("log-tail.txt")).toBeInTheDocument();
    expect(within(dialog).getByText("2.0 KB")).toBeInTheDocument();
    expect(within(dialog).getByText("Passwords, keys and tokens")).toBeInTheDocument();
    expect(calls.map((c) => c.op)).toEqual(["diagnostics.preview"]);
    expect(saveMock).not.toHaveBeenCalled();

    // Inspect: each file's exact contents can be shown and hidden.
    const show = within(dialog).getByRole("button", { name: "Show contents of log-tail.txt" });
    expect(show).toHaveAttribute("aria-expanded", "false");
    await user.click(show);
    const pre = within(dialog).getByLabelText("Contents of log-tail.txt");
    expect(pre).toHaveTextContent("op=notes.create code=validation.invalid_input");
    expect(pre).toHaveAttribute("tabindex", "0");
    const hide = within(dialog).getByRole("button", { name: "Hide contents of log-tail.txt" });
    expect(hide).toHaveAttribute("aria-expanded", "true");
    expect(hide).toHaveAttribute("aria-controls", pre.id);
    await user.click(hide);
    expect(within(dialog).queryByLabelText("Contents of log-tail.txt")).toBeNull();
  });

  it("saves to the picked location, then offers Reveal in File Manager", async () => {
    const { user, dialog } = await openDialog();
    await within(dialog).findByText("manifest.json");
    await user.click(within(dialog).getByRole("button", { name: "Export…" }));
    await screen.findByText("Export complete");
    expect(saveMock).toHaveBeenCalledWith(
      expect.objectContaining({
        defaultPath: "OpenFrame Diagnostics 2026-10-05.zip",
        filters: [{ name: "ZIP archive", extensions: ["zip"] }],
      }),
    );
    expect(calls.at(-1)).toEqual({
      op: "diagnostics.export_bundle",
      args: { path: "C:\\Support\\OpenFrame Diagnostics 2026-10-05.zip" },
    });
    expect(screen.getByText(/Nothing was sent\./)).toBeInTheDocument();
    expect(screen.getByText("4.0 KB")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Reveal in File Manager" }));
    expect(reveal).toHaveBeenCalledWith("exported", undefined, "C:\\Support\\OpenFrame Diagnostics 2026-10-05.zip");
  });

  it("does nothing when the Save dialog is cancelled", async () => {
    saveMock.mockResolvedValueOnce(null);
    const { user, dialog } = await openDialog();
    await within(dialog).findByText("manifest.json");
    await user.click(within(dialog).getByRole("button", { name: "Export…" }));
    await waitFor(() => expect(saveMock).toHaveBeenCalled());
    expect(calls.some((c) => c.op === "diagnostics.export_bundle")).toBe(false);
    expect(screen.getByRole("dialog", { name: "Export Diagnostics" })).toBeInTheDocument();
  });

  it("shows a refused destination in plain words and keeps the dialog open", async () => {
    exportFail = {
      code: "export.inside_project",
      message: "The diagnostics bundle can't be saved inside the project's own folder. Choose another location, such as Documents.",
    };
    const { user, dialog } = await openDialog();
    await within(dialog).findByText("manifest.json");
    await user.click(within(dialog).getByRole("button", { name: "Export…" }));
    const alert = await within(dialog).findByRole("alert");
    expect(alert).toHaveTextContent("inside the project's own folder");
    expect(within(dialog).getByRole("button", { name: "Export…" })).toBeEnabled();
  });

  it("explains a permission refusal and offers no export", async () => {
    previewFail = { code: "permission.denied", message: "You don't have permission to export from this project." };
    const { dialog } = await openDialog();
    expect(await within(dialog).findByRole("alert")).toHaveTextContent("You don't have permission");
    expect(within(dialog).getByRole("button", { name: "Export…" })).toBeDisabled();
  });

  it("uses no em dashes in what it shows", async () => {
    const { dialog } = await openDialog();
    await within(dialog).findByText("manifest.json");
    expect(document.body.textContent ?? "").not.toMatch(/\u2014/);
  });
});

describe("formatBytes", () => {
  it("reads naturally", () => {
    expect(formatBytes(512)).toBe("512 bytes");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(3 * 1024 * 1024)).toBe("3.0 MB");
  });
});
