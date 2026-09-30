import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import type { InterchangeImportPreview } from "../../ipc/generated/InterchangeImportPreview";
import type { InterchangeImportReport } from "../../ipc/generated/InterchangeImportReport";
import type { InterchangeSources } from "../../ipc/generated/InterchangeSources";
import { ExportScreenplayDialog, ImportScreenplayDialog } from "./index";

// Test fixtures only: the IPC boundary and the system dialogs are replaced.
const calls: { op: string; args: Record<string, unknown> }[] = [];
let failPreview: { code: string; message: string } | null = null;

const preview: InterchangeImportPreview = {
  previewId: "p1",
  sourceName: "Pasted screenplay",
  sourceFormat: "pasted",
  formatLabel: "Pasted text",
  title: null,
  author: null,
  sceneCount: 3,
  characterCount: 2,
  characters: ["MEERA", "ARJUN"],
  approxPages: 1,
  elementCount: 9,
  dialogueCount: 2,
  scenes: [
    { index: 0, heading: "INT. POLICE STATION — NIGHT", sourceNumber: null, status: "ok", elementCount: 5 },
    { index: 1, heading: "STREET — DAY", sourceNumber: null, status: "uncertain_heading", elementCount: 2 },
    { index: 2, heading: "EXT. RAILWAY PLATFORM - DUSK", sourceNumber: null, status: "ok", elementCount: 1 },
  ],
  warnings: [{ code: "uncertain_heading", message: "“STREET — DAY” was read as a scene heading, but it has no INT./EXT. prefix.", level: "attention", sceneIndex: 1 }],
  attentionCount: 1,
  confidence: "medium",
  confidenceScore: 0.7,
  needsReview: true,
  existingScreenplays: [],
  episodic: false,
  canCreateScreenplay: true,
  episodesWithoutScreenplay: [],
  defaultMode: "new_screenplay",
  canKeepSource: true,
};

const report: InterchangeImportReport = {
  screenplayId: "s1",
  screenplayTitle: "Pasted screenplay",
  draftId: "d1",
  draftName: "Pasted screenplay (imported)",
  draftLabel: "Draft 1 — Pasted screenplay (imported)",
  mode: "new_screenplay",
  sourceName: "Pasted screenplay",
  sourceFormat: "pasted",
  importedScenes: 3,
  importedElements: 9,
  detectedCharacters: 2,
  approxPages: 1,
  warningsNeedingAttention: 1,
  warnings: preview.warnings,
  keptFileId: null,
  message: "Imported.",
};

const sources: InterchangeSources = {
  screenplays: [
    {
      id: "s1",
      title: "Black Rain",
      currentDraftId: "d2",
      drafts: [
        { id: "d1", name: "Draft 1", status: "Draft", revisionLabel: null, revisionColor: null, sceneCount: 4, isCurrent: false, createdAt: 1 },
        { id: "d2", name: "Draft 2 — Shooting Draft", status: "Locked", revisionLabel: null, revisionColor: null, sceneCount: 4, isCurrent: true, createdAt: 2 },
      ],
    },
  ],
};

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(async () => null),
  save: vi.fn(async () => "C:\\Exports\\Black Rain - Draft 2 - Shooting Draft.pdf"),
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
    revealLocation: vi.fn(async () => undefined),
    call: vi.fn(async (op: string, args: Record<string, unknown> = {}) => {
      calls.push({ op, args });
      switch (op) {
        case "screenplay.import_preview":
          if (failPreview) throw new OpError(failPreview.code, failPreview.message);
          return preview;
        case "screenplay.import_apply":
          return report;
        case "interchange.sources":
          return sources;
        case "interchange.draft_scenes":
          return [];
        case "screenplay.export":
          return {
            path: args.path,
            fileName: "Black Rain - Draft 2 - Shooting Draft.pdf",
            format: args.format,
            bytesWritten: 1234,
            pages: 3,
            sceneCount: 4,
            sourceLabel: "Black Rain — Draft 2 — Shooting Draft",
            warnings: [],
            privateNotesExcluded: true,
            summary: "Exported.",
          };
        default:
          return null;
      }
    }),
  };
});

function wrap(ui: ReactNode) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>);
}

beforeEach(() => {
  calls.length = 0;
  failPreview = null;
});

describe("ImportScreenplayDialog", () => {
  it("previews pasted text, shows uncertainty, and imports only after confirmation", async () => {
    const onImported = vi.fn();
    wrap(<ImportScreenplayDialog open onClose={() => undefined} onImported={onImported} />);
    expect(screen.getByText("Importing never replaces your current screenplay. If one exists, the script arrives as a new screenplay or a new draft.")).toBeInTheDocument();
    const preview = screen.getByRole("button", { name: "Preview" });
    expect(preview).toBeDisabled();
    fireEvent.change(screen.getByLabelText("…or paste screenplay text"), { target: { value: "INT. POLICE STATION — NIGHT\n\nArjun enters." } });
    fireEvent.click(preview);

    await screen.findByText("Import Screenplay · Preview");
    expect(calls.find((c) => c.op === "screenplay.import_preview")?.args).toMatchObject({ path: null, pastedText: "INT. POLICE STATION — NIGHT\n\nArjun enters." });
    expect(screen.getByText("Uncertain heading")).toBeInTheDocument();
    expect(screen.getByText("Medium confidence")).toBeInTheDocument();
    expect(screen.getByText(/interpreted with limited certainty/)).toBeInTheDocument();
    expect(calls.some((c) => c.op === "screenplay.import_apply")).toBe(false);

    fireEvent.click(screen.getByRole("button", { name: "Import" }));
    await screen.findByText("Import complete");
    expect(calls.find((c) => c.op === "screenplay.import_apply")?.args).toMatchObject({ previewId: "p1", mode: "new_screenplay", keepSourceFile: true });
    expect(onImported).toHaveBeenCalledWith(report);
    expect(screen.getByRole("button", { name: "Open Screenplay" })).toBeInTheDocument();
  });

  it("explains a failed import and that the project was not changed", async () => {
    failPreview = { code: "import.not_a_screenplay", message: "The pasted text could not be interpreted as a screenplay. Your current project was not changed." };
    wrap(<ImportScreenplayDialog open onClose={() => undefined} />);
    fireEvent.change(screen.getByLabelText("…or paste screenplay text"), { target: { value: "Minutes of the meeting." } });
    fireEvent.click(screen.getByRole("button", { name: "Preview" }));
    await screen.findByText("Import failed. Your current project was not changed.");
    expect(screen.getByText("Unchanged")).toBeInTheDocument();
    expect(screen.getByText("The pasted text could not be interpreted as a screenplay.")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Paste Text Instead" }));
    await screen.findByText("Import Screenplay");
  });
});

describe("ExportScreenplayDialog", () => {
  it("exports the chosen draft with the chosen options as a snapshot", async () => {
    wrap(<ExportScreenplayDialog open onClose={() => undefined} />);
    await screen.findByLabelText("Draft to export");
    expect(screen.getByText(/Private notes excluded\./)).toBeInTheDocument();
    // Current draft is preselected.
    expect((screen.getByLabelText("Draft to export") as HTMLSelectElement).value).toBe("d2");
    fireEvent.click(screen.getByLabelText("Include notes"));
    fireEvent.click(screen.getByRole("button", { name: "Export…" }));
    await screen.findByText("Export complete");
    const args = calls.find((c) => c.op === "screenplay.export")?.args;
    expect(args).toMatchObject({
      draftId: "d2",
      format: "pdf",
      path: "C:\\Exports\\Black Rain - Draft 2 - Shooting Draft.pdf",
      options: { titlePage: true, sceneNumbers: true, includeNotes: true, revisionMarks: false, sceneIds: null },
    });
    expect(screen.getByRole("button", { name: "Show in Folder" })).toBeInTheDocument();
  });

  it("requires at least one scene when exporting selected scenes", async () => {
    wrap(<ExportScreenplayDialog open onClose={() => undefined} draftId="d1" />);
    await screen.findByLabelText("Draft to export");
    expect((screen.getByLabelText("Draft to export") as HTMLSelectElement).value).toBe("d1");
    fireEvent.click(screen.getByLabelText("Selected scenes"));
    await waitFor(() => expect(screen.getByRole("button", { name: "Export…" })).toBeDisabled());
    expect(screen.getByText("This draft has no scenes yet.")).toBeInTheDocument();
  });
});
