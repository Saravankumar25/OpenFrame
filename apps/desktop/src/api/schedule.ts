// Typed wrappers and pure helpers for the Shooting Schedule, Daily View,
// Call Sheets, Sides, Reports and Budget operations (Rust: modules/schedule).

import { create } from "zustand";
import { call, OpError } from "../ipc/client";
import { reportError } from "../ipc/query";
import { toast } from "../app/toast";

export type { ScheduleView } from "../ipc/generated/ScheduleView";
export type { ScheduleShootDayDto } from "../ipc/generated/ScheduleShootDayDto";
export type { ScheduleStripDto } from "../ipc/generated/ScheduleStripDto";
export type { ScheduleMarkerDto } from "../ipc/generated/ScheduleMarkerDto";
export type { ScheduleWarningDto } from "../ipc/generated/ScheduleWarningDto";
export type { ScheduleGroupingSuggestions } from "../ipc/generated/ScheduleGroupingSuggestions";
export type { ScheduleGroupingSuggestion } from "../ipc/generated/ScheduleGroupingSuggestion";
export type { ScheduleDailyView } from "../ipc/generated/ScheduleDailyView";
export type { CallSheetListRow } from "../ipc/generated/CallSheetListRow";
export type { CallSheetDetail } from "../ipc/generated/CallSheetDetail";
export type { CallSheetDocument } from "../ipc/generated/CallSheetDocument";
export type { CallSheetRefreshPreview } from "../ipc/generated/CallSheetRefreshPreview";
export type { ScheduleSidesContent } from "../ipc/generated/ScheduleSidesContent";
export type { ScheduleSideRow } from "../ipc/generated/ScheduleSideRow";
export type { ScheduleSideDetail } from "../ipc/generated/ScheduleSideDetail";
export type { ReportData } from "../ipc/generated/ReportData";
export type { ReportSavedRow } from "../ipc/generated/ReportSavedRow";
export type { ReportSaved } from "../ipc/generated/ReportSaved";
export type { BudgetView } from "../ipc/generated/BudgetView";
export type { BudgetDto } from "../ipc/generated/BudgetDto";
export type { BudgetLineDto } from "../ipc/generated/BudgetLineDto";

/** Every table the schedule projections read (drives query invalidation). */
export const SCHEDULE_TABLES = [
  "shooting_schedule",
  "shooting_day",
  "schedule_strip",
  "schedule_marker",
  "schedule_warning_ack",
  "call_sheet",
  "production_source",
  "screenplay",
  "screenplay_draft",
  "screenplay_scene",
  "screenplay_element",
  "breakdown_element",
  "catalog_item",
  "location",
  "cast_member",
  "story_character",
  "project",
];
export const CALL_SHEET_TABLES = [...SCHEDULE_TABLES, "asset"];
export const SIDES_TABLES = [...SCHEDULE_TABLES, "side"];
export const REPORT_TABLES = [...SCHEDULE_TABLES, "production_report"];
// production_source + draft tables drive the manual "Review budget" reminder only.
export const BUDGET_TABLES = ["budget_snapshot", "budget_line", "production_source", "screenplay_draft", "screenplay"];

// ------------------------------------------------------------------ formatting

/** "5h 30m", "45m", "10h". */
export function formatMinutes(m: number): string {
  const h = Math.floor(m / 60);
  const r = m % 60;
  if (h === 0) return `${r}m`;
  if (r === 0) return `${h}h`;
  return `${h}h ${String(r).padStart(2, "0")}m`;
}

/**
 * Parse a duration typed by a person: "90", "1:30", "1h 30m", "1.5h", "45m".
 * Returns minutes, null for empty input, or NaN when it can't be understood.
 */
export function parseDuration(input: string): number | null {
  const s = input.trim().toLowerCase();
  if (!s) return null;
  if (/^\d+$/.test(s)) return Number(s);
  const colon = /^(\d+):(\d{1,2})$/.exec(s);
  if (colon) return Number(colon[1]) * 60 + Number(colon[2]);
  const hm = /^(?:(\d+(?:\.\d+)?)\s*h)?\s*(?:(\d+)\s*m(?:in)?)?$/.exec(s);
  if (hm && (hm[1] || hm[2])) return Math.round(Number(hm[1] ?? 0) * 60) + Number(hm[2] ?? 0);
  return NaN;
}

/** "1 2/8", "3/8", "2" from eighths of a page. */
export function formatPages(eighths: number): string {
  const whole = Math.floor(eighths / 8);
  const rest = eighths % 8;
  if (whole === 0) return `${rest}/8`;
  if (rest === 0) return `${whole}`;
  return `${whole} ${rest}/8`;
}

/** Parse "1 2/8", "3/8", "2" or "1.25" into eighths; null for empty, NaN if invalid. */
export function parsePages(input: string): number | null {
  const s = input.trim();
  if (!s) return null;
  const m = /^(?:(\d+)\s+)?(\d+)\/8$/.exec(s);
  if (m) return Number(m[1] ?? 0) * 8 + Number(m[2]);
  if (/^\d+(?:\.\d+)?$/.test(s)) return Math.round(Number(s) * 8);
  return NaN;
}

const CURRENCY_DIGITS: Record<string, number> = { JPY: 0, KRW: 0 };

export function currencyDigits(currency: string): number {
  return CURRENCY_DIGITS[currency] ?? 2;
}

/** Format integer minor units (cents/paise) as money. */
export function formatMoney(minor: number, currency: string): string {
  const digits = currencyDigits(currency);
  const value = minor / 10 ** digits;
  try {
    return new Intl.NumberFormat(currency === "INR" ? "en-IN" : undefined, {
      style: "currency",
      currency,
      maximumFractionDigits: digits,
      minimumFractionDigits: 0,
    }).format(value);
  } catch {
    return `${currency} ${value.toFixed(digits)}`;
  }
}

/**
 * Parse a typed amount ("12,00,000", "1500.50", "₹ 2,000") into minor units.
 * Returns null for empty input and NaN for anything negative or malformed.
 */
export function parseMoney(input: string, currency: string): number | null {
  const s = input.replace(/[\s,]/g, "").replace(/^[^\d.-]+/, "");
  if (!s) return null;
  if (!/^\d+(?:\.\d+)?$/.test(s)) return NaN;
  const digits = currencyDigits(currency);
  const [whole, frac = ""] = s.split(".");
  if (frac.length > digits) return NaN;
  return Number(whole) * 10 ** digits + Number((frac + "0".repeat(digits)).slice(0, digits) || "0");
}

export function minorToInput(minor: number | null | undefined, currency: string): string {
  if (minor === null || minor === undefined) return "";
  const digits = currencyDigits(currency);
  return digits === 0 ? String(minor) : (minor / 10 ** digits).toFixed(digits).replace(/\.0+$/, "");
}

export function formatDateTime(ms: number): string {
  return new Date(ms).toLocaleString(undefined, { day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" });
}

// ------------------------------------------------------------------ moves with Keep Anyway

interface KeepAnywayState {
  pending: { message: string; retry: () => Promise<void> } | null;
  set: (p: KeepAnywayState["pending"]) => void;
}

/** A move refused by strict validation waits here for "Keep Anyway" (FSD §36.6). */
export const useKeepAnyway = create<KeepAnywayState>((set) => ({
  pending: null,
  set: (pending) => set({ pending }),
}));

/** Run a schedule move; strict-validation refusals offer "Keep Anyway". */
export async function runScheduleMove(op: "schedule.move_strip" | "schedule.move_strips", args: Record<string, unknown>, doneText: string): Promise<void> {
  try {
    await call(op, args);
    toast.undoable(doneText);
  } catch (e) {
    if (e instanceof OpError && e.code === "validation.schedule_warning") {
      useKeepAnyway.getState().set({
        message: e.message,
        retry: async () => {
          try {
            await call(op, { ...args, keepAnyway: true });
            toast.undoable(doneText);
          } catch (err) {
            reportError(err);
          }
        },
      });
      return;
    }
    reportError(e);
  }
}

/** Status chip tone for call sheets. */
export function callSheetTone(status: string): "default" | "g" | "b" | "y" | "out" {
  switch (status) {
    case "Final":
    case "Issued":
      return "g";
    case "Ready":
      return "b";
    case "Needs Refresh":
      return "y";
    case "Superseded":
      return "out";
    default:
      return "default";
  }
}

/** Human status label (UX §3.31). */
export function callSheetStatusLabel(status: string): string {
  switch (status) {
    case "Final":
      return "Finalized";
    case "Needs Refresh":
      return "Source changed — needs refresh";
    default:
      return status;
  }
}
