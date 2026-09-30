// Pure helpers for the document Export dialog (FSD §20.7, §60, §122;
// Import/Export §3). No project truth lives here.

export type ExportFormatValue = "pdf" | "csv" | "xlsx" | "txt";

export interface ExportFormatOption {
  value: ExportFormatValue;
  label: string;
  hint?: string;
}

export const FORMAT_LABELS: Record<ExportFormatValue, string> = {
  pdf: "PDF",
  csv: "CSV",
  xlsx: "Excel (XLSX)",
  txt: "Plain text",
};

/** Commonly offered format sets. */
export const PDF_ONLY: ExportFormatOption[] = [{ value: "pdf", label: "PDF", hint: "printable snapshot" }];
export const TABLE_FORMATS: ExportFormatOption[] = [
  { value: "pdf", label: "PDF", hint: "printable" },
  { value: "csv", label: "CSV", hint: "spreadsheet-friendly text" },
  { value: "xlsx", label: "Excel (XLSX)", hint: "one sheet per table" },
];

/**
 * Extra option shown in the dialog; its value is sent as `args[arg ?? key]`.
 * `formats` limits the option to some formats (two fields may share an `arg`
 * when their formats differ, e.g. a CSV-only choice).
 */
export type ExportOptionField =
  | { kind: "checkbox"; key: string; arg?: string; label: string; default: boolean; formats?: ExportFormatValue[] }
  | { kind: "choice"; key: string; arg?: string; label: string; default: string; options: { value: string; label: string }[]; formats?: ExportFormatValue[] };

/** A scope choice ("Entire schedule", "Current day only", "Selected days"…). */
export interface ExportScope {
  value: string;
  label: string;
  /** Arguments this scope adds to the request. */
  args?: Record<string, unknown>;
  disabled?: boolean;
  /** Pick items from a list; the chosen ids are sent as `args[argKey]`. */
  picker?: { argKey: string; items: { id: string; label: string }[]; emptyText?: string };
}

export type OptionValues = Record<string, boolean | string>;

export function initialOptions(fields: readonly ExportOptionField[]): OptionValues {
  const out: OptionValues = {};
  for (const f of fields) out[f.key] = f.default;
  return out;
}

export function fieldApplies(f: ExportOptionField, format: ExportFormatValue): boolean {
  return !f.formats || f.formats.includes(format);
}

/** The request for an export op (without the destination path). */
export function buildExportArgs(
  base: Record<string, unknown> | undefined,
  format: ExportFormatValue,
  scope: ExportScope | undefined,
  picked: readonly string[],
  fields: readonly ExportOptionField[],
  values: OptionValues,
): Record<string, unknown> {
  const args: Record<string, unknown> = { ...(base ?? {}), ...(scope?.args ?? {}), format };
  if (scope?.picker) args[scope.picker.argKey] = [...picked];
  for (const f of fields) {
    if (fieldApplies(f, format) && values[f.key] !== undefined) args[f.arg ?? f.key] = values[f.key];
  }
  return args;
}

const WINDOWS_BAD = /[<>:"/\\|?*]/g;
const RESERVED = /^(con|prn|aux|nul|com[1-9]|lpt[1-9])$/i;

function clean(part: string): string {
  return part
    .replace(/[—–]/g, "-")
    .replace(WINDOWS_BAD, " ")
    .split("")
    .filter((c) => c.charCodeAt(0) >= 32)
    .join("")
    .replace(/\s+/g, " ")
    .trim();
}

/** "2026-09-30" in local time. */
export function isoDate(d: Date): string {
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}

/** Default file name: "<Project> - <Document> - <date>.<ext>", safe for Windows. */
export function exportFileName(project: string | null | undefined, document: string, format: ExportFormatValue, date: Date = new Date()): string {
  const parts = [clean(project ?? ""), clean(document), isoDate(date)].filter(Boolean);
  let base = parts.join(" - ").replace(/[. ]+$/, "");
  if (base.length > 150) base = base.slice(0, 150).trim();
  if (!base || RESERVED.test(base)) base = `Export ${base}`.trim();
  return `${base}.${format}`;
}

export function saveFilters(format: ExportFormatValue) {
  return [{ name: FORMAT_LABELS[format], extensions: [format] }];
}
