// Typed wrappers for the screenplay interchange operations. Project truth
// stays in Rust; these hooks only cache it and are invalidated by the tables
// each query reads.

import { call } from "../../ipc/client";
import { useOp } from "../../ipc/query";
import type { InterchangeExportSceneDto } from "../../ipc/generated/InterchangeExportSceneDto";
import type { InterchangeImportApplyArgs } from "../../ipc/generated/InterchangeImportApplyArgs";
import type { InterchangeImportPreview } from "../../ipc/generated/InterchangeImportPreview";
import type { InterchangeImportPreviewArgs } from "../../ipc/generated/InterchangeImportPreviewArgs";
import type { InterchangeImportReport } from "../../ipc/generated/InterchangeImportReport";
import type { InterchangePrintPreview } from "../../ipc/generated/InterchangePrintPreview";
import type { InterchangeScreenplayExportArgs } from "../../ipc/generated/InterchangeScreenplayExportArgs";
import type { InterchangeScreenplayExportOptions } from "../../ipc/generated/InterchangeScreenplayExportOptions";
import type { InterchangeScreenplayExportResult } from "../../ipc/generated/InterchangeScreenplayExportResult";
import type { InterchangeSources } from "../../ipc/generated/InterchangeSources";

/** Every table the export dialog's queries read. */
export const SCREENPLAY_TABLES = ["screenplay", "screenplay_draft", "screenplay_scene", "screenplay_element"];

export const interchange = {
  /** Parse a file or pasted text; nothing is written. */
  preview: (args: InterchangeImportPreviewArgs) => call<InterchangeImportPreview>("screenplay.import_preview", args),
  /** Create a new screenplay or a new draft from a preview (one undoable step). */
  apply: (args: InterchangeImportApplyArgs) => call<InterchangeImportReport>("screenplay.import_apply", args),
  /** Write a snapshot file; the project never changes. */
  exportDraft: (args: InterchangeScreenplayExportArgs) => call<InterchangeScreenplayExportResult>("screenplay.export", args),
  /** Keep a file that could not be read as a screenplay as an ordinary project file. */
  addToProjectFiles: (path: string) => call<unknown>("files.add", { paths: [path], mode: "copy" }),
};

export function useInterchangeSources(enabled: boolean) {
  return useOp<InterchangeSources>("interchange.sources", {}, SCREENPLAY_TABLES, { enabled });
}

export function useDraftScenes(draftId: string | null) {
  return useOp<InterchangeExportSceneDto[]>("interchange.draft_scenes", { draftId: draftId ?? "" }, SCREENPLAY_TABLES, { enabled: !!draftId });
}

export function usePrintPreview(draftId: string | null, options: InterchangeScreenplayExportOptions, pageNumbers: boolean, enabled: boolean) {
  return useOp<InterchangePrintPreview>(
    "interchange.export_preview",
    { draftId: draftId ?? "", options, pageNumbers },
    SCREENPLAY_TABLES,
    { enabled: enabled && !!draftId },
  );
}
