import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { StoryboardExportButton } from "./buttons";
import { buildExportArgs, initialOptions, type ExportOptionField } from "./model";

// Test fixtures only: the IPC boundary and the system Save dialog are replaced.
const calls: { op: string; args: Record<string, unknown> }[] = [];

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(async () => null),
  save: vi.fn(async () => "C:\\Exports\\Storyboards.pdf"),
}));

vi.mock("../../ipc/client", async () => ({
  OpError: class extends Error {},
  inTauri: () => false,
  onAppEvent: () => Promise.resolve(() => undefined),
  revealLocation: vi.fn(async () => undefined),
  call: vi.fn(async (op: string, args: Record<string, unknown> = {}) => {
    calls.push({ op, args });
    if (op === "project.current") return { title: "Black Rain" };
    return {
      path: args.path,
      fileName: "Storyboards.pdf",
      format: args.format,
      bytesWritten: 2048,
      pages: 2,
      document: "Storyboards",
      sourceLabel: "Storyboards",
      scopeLabel: "All storyboards",
      contentsLabel: "12 panels",
      warnings: [],
      privateNotesExcluded: true,
      summary: "Exported.",
    };
  }),
}));

beforeEach(() => {
  calls.length = 0;
});

describe("Storyboard export: panels per row", () => {
  it("sends a numeric choice as a number", () => {
    const fields: ExportOptionField[] = [
      { kind: "choice", key: "columns", label: "Panels per row", default: "2", numeric: true, options: [] },
      { kind: "choice", key: "layout", label: "Layout", default: "board", options: [] },
    ];
    const values = { ...initialOptions(fields), columns: "3" };
    expect(buildExportArgs(undefined, "pdf", undefined, [], fields, values)).toEqual({ format: "pdf", columns: 3, layout: "board" });
  });

  it("offers 2 (recommended) and 3, defaults to 2 and exports with columns: 2", async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(
      <QueryClientProvider client={client}>
        <StoryboardExportButton boards={[{ id: "b1", label: "Scene 1" }]} />
      </QueryClientProvider>,
    );
    fireEvent.click(screen.getByRole("button", { name: /Export/ }));
    expect(screen.getByText("Panels per row")).toBeTruthy();
    expect(screen.getByText("2 (recommended)")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Export…" }));
    await waitFor(() => expect(calls.some((c) => c.op === "storyboard.export_sheet")).toBe(true));
    const sent = calls.find((c) => c.op === "storyboard.export_sheet")!.args;
    expect(sent).toMatchObject({ format: "pdf", columns: 2, includeNotes: false });
    expect(typeof sent.columns).toBe("number");
  });
});
