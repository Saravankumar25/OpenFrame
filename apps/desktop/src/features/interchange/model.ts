// Pure helpers for the screenplay import/export dialogs (FSD §19–20, §60,
// §117, §122–123; mocks 091–096). No project truth lives here.

import type { ChipTone } from "../../design-system";
import type { InterchangeImportPreview } from "../../ipc/generated/InterchangeImportPreview";
import type { InterchangeImportTargetScreenplay } from "../../ipc/generated/InterchangeImportTargetScreenplay";
import type { InterchangeWarning } from "../../ipc/generated/InterchangeWarning";

export type InterchangeExportFormat = "pdf" | "fdx" | "fountain" | "docx" | "txt";

export interface ExportFormatSpec {
  value: InterchangeExportFormat;
  label: string;
  hint?: string;
  extension: string;
  /** Shown under "Additional formats" rather than in the main list. */
  additional?: boolean;
}

/** Export formats in the order of mock 095. */
export const EXPORT_FORMATS: readonly ExportFormatSpec[] = [
  { value: "pdf", label: "PDF", hint: "professional, printable", extension: "pdf" },
  { value: "fdx", label: "Final Draft (.fdx)", extension: "fdx" },
  { value: "fountain", label: "Fountain", extension: "fountain" },
  { value: "docx", label: "DOCX", hint: "editable", extension: "docx" },
  { value: "txt", label: "Plain text (.txt)", hint: "screenplay layout in plain characters", extension: "txt", additional: true },
];

export function formatSpec(f: InterchangeExportFormat): ExportFormatSpec {
  return EXPORT_FORMATS.find((x) => x.value === f) ?? EXPORT_FORMATS[0];
}

/** File-type filter for the system Open dialog. */
export const IMPORT_FILTERS = [
  { name: "Screenplays (PDF, Final Draft, Fountain, TXT, DOCX)", extensions: ["pdf", "fdx", "fountain", "spmd", "txt", "docx"] },
  { name: "All files", extensions: ["*"] },
];

export function saveFilters(f: InterchangeExportFormat) {
  const spec = formatSpec(f);
  return [{ name: spec.label, extensions: [spec.extension] }];
}

const WINDOWS_BAD = /[<>:"/\\|?*]/g;

/** Suggested export file name: "Black Rain - Draft 6 - Shooting Draft.pdf". */
export function defaultExportFileName(screenplayTitle: string, draftName: string, f: InterchangeExportFormat): string {
  const parts = [screenplayTitle.trim(), draftName.trim()].filter(Boolean);
  let base = parts
    .join(" - ")
    .replace(/[—–]/g, "-")
    .replace(WINDOWS_BAD, " ")
    .split("")
    .filter((c) => c.charCodeAt(0) >= 32)
    .join("")
    .replace(/\s+/g, " ")
    .trim()
    .replace(/[. ]+$/, "");
  if (!base) base = "Screenplay";
  if (base.length > 100) base = base.slice(0, 100).trim();
  return `${base}.${formatSpec(f).extension}`;
}

/** Last path component of a Windows or POSIX path. */
export function fileName(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] || path;
}

/** Status chip for a detected scene in the import preview (mock 092). */
export function sceneStatusChip(status: string): { tone: ChipTone; label: string } {
  switch (status) {
    case "ok":
      return { tone: "g", label: "OK" };
    case "uncertain_heading":
      return { tone: "y", label: "Uncertain heading" };
    case "potentially_empty":
      return { tone: "y", label: "Potentially empty" };
    case "no_heading":
      return { tone: "default", label: "Before first scene" };
    default:
      return { tone: "default", label: status };
  }
}

export function confidenceLabel(c: string): { tone: ChipTone; label: string } {
  switch (c) {
    case "high":
      return { tone: "g", label: "High confidence" };
    case "medium":
      return { tone: "y", label: "Medium confidence" };
    default:
      return { tone: "r", label: "Low confidence" };
  }
}

const SOURCE_NOUN: Record<string, string> = {
  pdf: "PDF",
  fdx: "Final Draft file",
  fountain: "Fountain file",
  txt: "text file",
  docx: "Word document",
  pasted: "pasted text",
};

/** Headline for the preview banner. Never presents uncertain parsing as certain (IEX-004). */
export function previewBanner(p: Pick<InterchangeImportPreview, "warnings" | "confidence" | "needsReview" | "sourceFormat" | "attentionCount">): {
  tone: "ok" | "warn" | "info";
  title: string;
  text: string;
} {
  const n = p.warnings.length;
  const noun = SOURCE_NOUN[p.sourceFormat] ?? "file";
  const count = n === 0 ? "No warnings." : `${n} warning${n === 1 ? "" : "s"}.`;
  if (p.confidence === "low" || p.needsReview) {
    return {
      tone: "warn",
      title: count,
      text: `This ${noun} was interpreted with limited certainty. Review the items below before importing. You may continue with warnings.`,
    };
  }
  if (n > 0) {
    return { tone: "info", title: count, text: "The screenplay structure was recognised. Some details were adjusted on the way in — see below." };
  }
  return { tone: "ok", title: count, text: "The screenplay structure was recognised." };
}

export function attentionFirst(ws: readonly InterchangeWarning[]): InterchangeWarning[] {
  return [...ws].sort((a, b) => (a.level === b.level ? 0 : a.level === "attention" ? -1 : 1));
}

export type ImportMode = "new_screenplay" | "new_draft";

type ImportTargets = Pick<InterchangeImportPreview, "canCreateScreenplay" | "existingScreenplays" | "episodic" | "episodesWithoutScreenplay">;

/** Destinations offered in the preview (mock 092). A project (or episode) has
 *  one screenplay, so New Screenplay is offered only where none exists yet;
 *  the current draft is never replaced (FSD-SCRIPT-044). */
export function importModes(p: ImportTargets): { value: ImportMode; label: string }[] {
  const out: { value: ImportMode; label: string }[] = [];
  if (p.canCreateScreenplay) out.push({ value: "new_screenplay", label: p.episodic ? "New Screenplay for episode" : "New Screenplay" });
  const n = p.existingScreenplays.length;
  if (n > 0) out.push({ value: "new_draft", label: n === 1 ? "New Draft in current screenplay" : "New Draft in…" });
  return out;
}

/** The chosen destination is complete. */
export function importReady(p: ImportTargets, mode: ImportMode, screenplayId: string, episodeId: string): boolean {
  if (mode === "new_screenplay") return p.canCreateScreenplay && (!p.episodic || p.episodesWithoutScreenplay.some((e) => e.id === episodeId));
  return p.existingScreenplays.length === 1 || p.existingScreenplays.some((s) => s.id === screenplayId);
}

export function targetLabel(s: Pick<InterchangeImportTargetScreenplay, "title" | "episodeTitle">): string {
  return s.episodeTitle ? `${s.episodeTitle} — ${s.title}` : s.title;
}

/** "~96" pages. */
export function pagesLabel(n: number): string {
  return `~${n}`;
}

/** Stage of an import failure for the "Import failed" dialog (mock 093), from the error code. */
export function failureStage(code: string, source: "file" | "pasted"): string {
  if (code.startsWith("import.pdf")) return "Reading text from the PDF";
  if (code === "import.unsupported_format") return "Checking the file type";
  if (code === "import.unsafe_archive" || code.startsWith("import.docx")) return "Opening the document";
  if (code.startsWith("import.fdx")) return "Reading the Final Draft file";
  if (code === "import.empty_source" || code === "import.too_large") return source === "pasted" ? "Reading the pasted text" : "Reading the file";
  if (code.startsWith("validation") || code.startsWith("not_found")) return "Reading the file";
  if (code.startsWith("storage")) return "Saving the imported screenplay";
  return "Interpreting the screenplay";
}

/** Failures after which the file can still be kept as an ordinary project file (Import/Export §4.2). */
export function canKeepAsProjectFile(code: string): boolean {
  return code.startsWith("import.") && code !== "import.too_large" && code !== "import.empty_source";
}

/** Display label for a draft in the export source picker (mock 095). */
export function draftLabel(screenplayTitle: string, draftName: string, multipleScreenplays: boolean): string {
  return multipleScreenplays ? `${screenplayTitle} — ${draftName}` : draftName;
}

export function statusTone(status: string): ChipTone {
  switch (status) {
    case "Locked":
      return "g";
    case "Review":
      return "b";
    case "Revision":
      return "y";
    default:
      return "default";
  }
}

export interface PreviewZoom {
  scale: number;
  label: string;
}

export const ZOOMS: readonly PreviewZoom[] = [
  { scale: 0.5, label: "50%" },
  { scale: 0.75, label: "75%" },
  { scale: 1, label: "100%" },
  { scale: 1.25, label: "125%" },
];
