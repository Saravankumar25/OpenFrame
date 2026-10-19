import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { ExportDialog } from "./ExportDialog";
import { ScheduleExportButton } from "./buttons";
import { buildExportArgs, exportFileName, initialOptions, type ExportOptionField } from "./model";

// Test fixtures only: the IPC boundary and the system Save dialog are replaced.
const calls: { op: string; args: Record<string, unknown> }[] = [];
let fail: { code: string; message: string } | null = null;
const saveMock = vi.fn(async (_o: { defaultPath?: string }) => "C:\\Exports\\Black Rain - Shooting Schedule - 2026-09-30.pdf" as string | null);

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(async () => null),
  save: (o: { defaultPath?: string }) => saveMock(o),
}));

const reveal = vi.fn(async () => undefined);

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
    revealLocation: (...a: unknown[]) => reveal(...(a as [])),
    call: vi.fn(async (op: string, args: Record<string, unknown> = {}) => {
      calls.push({ op, args });
      if (op === "project.current") return { title: "Black Rain" };
      if (fail) throw new OpError(fail.code, fail.message);
      return {
        path: args.path,
        fileName: "Black Rain - Shooting Schedule - 2026-09-30.pdf",
        format: args.format,
        bytesWritten: 2048,
        pages: 2,
        document: "Shooting Schedule",
        sourceLabel: "Main schedule · Shooting Draft 6 (Locked)",
        scopeLabel: "Entire schedule",
        contentsLabel: "3 days · 12 scenes",
        warnings: ["1 panel image could not be found and is shown as a placeholder."],
        privateNotesExcluded: true,
        summary: "Exported.",
      };
    }),
  };
});

function wrap(ui: ReactNode) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>);
}

beforeEach(() => {
  calls.length = 0;
  fail = null;
  saveMock.mockClear();
  reveal.mockClear();
});

describe("export file names", () => {
  it("follows <Project> - <Document> - <date>.<ext> and is safe for Windows", () => {
    const d = new Date(2026, 8, 30);
    expect(exportFileName("Black Rain", "Shooting Schedule", "pdf", d)).toBe("Black Rain - Shooting Schedule - 2026-09-30.pdf");
    expect(exportFileName("A/B: “C”?", "Moodboard — Overall Look", "pdf", d)).toBe("A B “C” - Moodboard - Overall Look - 2026-09-30.pdf");
    expect(exportFileName(null, "Budget", "xlsx", d)).toBe("Budget - 2026-09-30.xlsx");
    expect(exportFileName("x".repeat(300), "Doc", "csv", d).length).toBeLessThanOrEqual(154);
  });
});

describe("export arguments", () => {
  const fields: ExportOptionField[] = [
    { kind: "choice", key: "content", label: "Contents", default: "both", formats: ["pdf", "xlsx"], options: [] },
    { kind: "choice", key: "csvContent", arg: "content", label: "Table", default: "scenes", formats: ["csv"], options: [] },
    { kind: "checkbox", key: "includeNotes", label: "Notes", default: false },
  ];
  it("merges base, scope, picked items and the options that apply to the format", () => {
    const values = initialOptions(fields);
    const scope = { value: "selected", label: "Selected", picker: { argKey: "sceneIds", items: [] } };
    expect(buildExportArgs({ id: "x" }, "csv", scope, ["s1", "s2"], fields, values)).toEqual({
      id: "x",
      format: "csv",
      sceneIds: ["s1", "s2"],
      content: "scenes",
      includeNotes: false,
    });
    expect(buildExportArgs(undefined, "pdf", { value: "day", label: "Day", args: { dayIds: ["d1"] } }, [], fields, values)).toEqual({
      format: "pdf",
      dayIds: ["d1"],
      content: "both",
      includeNotes: false,
    });
  });
});

describe("ExportDialog", () => {
  it("asks where to save, exports the snapshot and offers Reveal in File Manager", async () => {
    const onClose = vi.fn();
    wrap(
      <ExportDialog
        open
        onClose={onClose}
        title="Export Shooting Schedule"
        documentName="Shooting Schedule"
        op="schedule.export_schedule"
        formats={[{ value: "pdf", label: "PDF" }, { value: "csv", label: "CSV" }]}
        scopes={[
          { value: "all", label: "Entire schedule" },
          { value: "days", label: "Selected days", picker: { argKey: "dayIds", items: [{ id: "d1", label: "Shoot Day 1" }] } },
        ]}
        options={[{ kind: "checkbox", key: "includeUnscheduled", label: "Include unscheduled scenes", default: true }]}
      />,
    );
    expect(screen.getByText(/Private notes excluded/)).toBeTruthy();
    // Selected days need at least one day.
    fireEvent.click(screen.getByLabelText("Selected days"));
    const exportBtn = screen.getByRole("button", { name: "Export…" }) as HTMLButtonElement;
    expect(exportBtn.disabled).toBe(true);
    fireEvent.click(screen.getByLabelText("Shoot Day 1"));
    expect(exportBtn.disabled).toBe(false);
    fireEvent.click(screen.getByLabelText("CSV"));
    // The file name follows "<Project> - <Document> - <date>.<ext>".
    expect(await screen.findByText(/^Black Rain - Shooting Schedule - \d{4}-\d{2}-\d{2}\.csv$/)).toBeTruthy();
    fireEvent.click(exportBtn);
    await waitFor(() => expect(screen.getByText("Export complete")).toBeTruthy());
    expect(saveMock.mock.calls[0][0].defaultPath).toMatch(/^Black Rain - Shooting Schedule - \d{4}-\d{2}-\d{2}\.csv$/);
    const exp = calls.find((c) => c.op === "schedule.export_schedule");
    expect(exp?.args).toMatchObject({ format: "csv", dayIds: ["d1"], includeUnscheduled: true, path: "C:\\Exports\\Black Rain - Shooting Schedule - 2026-09-30.pdf" });
    expect(screen.getByText("3 days · 12 scenes")).toBeTruthy();
    expect(screen.getByText(/could not be found/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /Reveal in File Manager/ }));
    expect(reveal).toHaveBeenCalledWith("exported", undefined, "C:\\Exports\\Black Rain - Shooting Schedule - 2026-09-30.pdf");
  });

  it("shows a human error and keeps the dialog open when the export fails", async () => {
    fail = { code: "export.pdf_unsupported_characters", message: "This document contains characters the standard PDF font can't print." };
    wrap(<ScheduleExportButton days={[]} />);
    fireEvent.click(screen.getByRole("button", { name: /Export/ }));
    fireEvent.click(screen.getByRole("button", { name: "Export…" }));
    await waitFor(() => expect(screen.getByText(/can't print/)).toBeTruthy());
    expect(screen.queryByText("Export complete")).toBeNull();
  });

  it("does nothing when the Save dialog is cancelled", async () => {
    saveMock.mockResolvedValueOnce(null);
    wrap(<ScheduleExportButton days={[]} />);
    fireEvent.click(screen.getByRole("button", { name: /Export/ }));
    fireEvent.click(screen.getByRole("button", { name: "Export…" }));
    await waitFor(() => expect(saveMock).toHaveBeenCalled());
    expect(calls.some((c) => c.op === "schedule.export_schedule")).toBe(false);
  });
});
