// Small helpers for the screenplay workspace: dates, initials and the
// remembered layout (FSD §17.4).

import { useCallback, useEffect, useRef, useState } from "react";
import { call } from "../../ipc/client";
import { useOp } from "../../ipc/query";

// ------------------------------------------------------------------ dates

const DAY = 86_400_000;

export function formatTime(ms: number): string {
  return new Date(ms).toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
}

export function formatDate(ms: number): string {
  return new Date(ms).toLocaleDateString(undefined, { day: "numeric", month: "short", year: "numeric" });
}

/** "Today 10:39", "Yesterday 21:12", "26 Sep 2026, 10:39". */
export function formatWhen(ms: number, now = Date.now()): string {
  const d = new Date(ms);
  const today = new Date(now);
  const startToday = new Date(today.getFullYear(), today.getMonth(), today.getDate()).getTime();
  if (ms >= startToday) return `Today ${formatTime(ms)}`;
  if (ms >= startToday - DAY) return `Yesterday ${formatTime(ms)}`;
  return `${d.toLocaleDateString(undefined, { day: "numeric", month: "short", year: "numeric" })}, ${formatTime(ms)}`;
}

/** "2 days ago", "last week", "3 weeks ago" (mock 083). */
export function formatAgo(ms: number, now = Date.now()): string {
  const diff = Math.max(0, now - ms);
  if (diff < 60_000) return "just now";
  if (diff < 3_600_000) return `${Math.round(diff / 60_000)} min ago`;
  if (diff < DAY) return `${Math.round(diff / 3_600_000)} h ago`;
  const days = Math.round(diff / DAY);
  if (days === 1) return "yesterday";
  if (days < 7) return `${days} days ago`;
  if (days < 14) return "last week";
  if (days < 60) return `${Math.round(days / 7)} weeks ago`;
  return `${Math.round(days / 30)} months ago`;
}

export function initials(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  return ((parts[0]?.[0] ?? "?") + (parts.length > 1 ? parts[parts.length - 1][0] : "")).toUpperCase();
}

// ----------------------------------------------------------------- layout

export type PanelId = "story" | "notes" | "characters" | "comments" | "draft";
export type Mode = "standard" | "focus" | "writingRoom";
/** Optional scene navigator columns (SCRIPT-017), in display order. */
export type NavColumn = "page" | "notes" | "revision" | "production";
export const NAV_COLUMNS: { id: NavColumn; label: string }[] = [
  { id: "page", label: "Page" },
  { id: "notes", label: "Notes" },
  { id: "revision", label: "Revision" },
  { id: "production", label: "Production number" },
];

export interface Layout {
  mode: Mode;
  /** Right-hand panels (Standard: at most one; Writing Room: at most two). */
  panels: PanelId[];
  showNotes: boolean;
  /** The draft the writer last had open (defaults to the Current draft). */
  draftId: string | null;
  /** The screenplay the writer last had open; a project may hold several (defaults to the oldest). */
  screenplayId: string | null;
  /** Scene navigator columns shown next to the number and heading. */
  navColumns: NavColumn[];
  /** The scene list is hidden so the page gets the room (owner request 2026-10-19). */
  navHidden: boolean;
}

export const DEFAULT_LAYOUT: Layout = { mode: "standard", panels: [], showNotes: true, draftId: null, screenplayId: null, navColumns: [], navHidden: false };

const PANELS: PanelId[] = ["story", "notes", "characters", "comments", "draft"];

export function normalizeLayout(v: unknown): Layout {
  if (!v || typeof v !== "object") return DEFAULT_LAYOUT;
  const o = v as Record<string, unknown>;
  const mode: Mode = o.mode === "focus" || o.mode === "writingRoom" ? o.mode : "standard";
  const panels = Array.isArray(o.panels) ? (o.panels.filter((p) => PANELS.includes(p as PanelId)) as PanelId[]).slice(0, 2) : [];
  return {
    mode,
    panels: mode === "standard" ? panels.slice(0, 1) : panels,
    showNotes: o.showNotes !== false,
    draftId: typeof o.draftId === "string" ? o.draftId : null,
    screenplayId: typeof o.screenplayId === "string" ? o.screenplayId : null,
    navColumns: normalizeColumns(o.navColumns),
    navHidden: o.navHidden === true,
  };
}

/** Known columns only, each once, in display order. */
export function normalizeColumns(v: unknown): NavColumn[] {
  if (!Array.isArray(v)) return [];
  return NAV_COLUMNS.map((c) => c.id).filter((id) => v.includes(id));
}

/** Show or hide one navigator column. */
export function toggleColumn(l: Layout, c: NavColumn): Layout {
  const has = l.navColumns.includes(c);
  return { ...l, navColumns: normalizeColumns(has ? l.navColumns.filter((x) => x !== c) : [...l.navColumns, c]) };
}

/** Toggle a panel respecting the mode's limit (oldest panel closes first). */
export function togglePanel(l: Layout, p: PanelId): Layout {
  if (l.panels.includes(p)) return { ...l, panels: l.panels.filter((x) => x !== p) };
  const max = l.mode === "writingRoom" ? 2 : 1;
  const mode = l.mode === "focus" ? "standard" : l.mode;
  return { ...l, mode, panels: [...l.panels, p].slice(-max) };
}

/** The writer's layout, remembered per project and user (never project content). Panel and
 * column changes are saved 0.5 s after the last one; a switch to another screenplay or draft is
 * saved at once, and a change still waiting when the workspace closes is sent then, so closing
 * the project right after a switch still reopens where the writer was. */
export function useLayout(): [Layout, (f: (l: Layout) => Layout) => void, boolean] {
  const saved = useOp<unknown>("screenplay.view_state", {}, [], { staleTime: Infinity });
  const [layout, setLayout] = useState<Layout | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pending = useRef<Layout | null>(null);
  const current = layout ?? (saved.isSuccess ? normalizeLayout(saved.data) : DEFAULT_LAYOUT);
  const update = useCallback(
    (f: (l: Layout) => Layout) => {
      setLayout((prev) => {
        const before = prev ?? current;
        const next = f(before);
        if (timer.current) clearTimeout(timer.current);
        timer.current = null;
        pending.current = next;
        if (next.screenplayId !== before.screenplayId || next.draftId !== before.draftId) {
          sendLayout(pending);
        } else {
          timer.current = setTimeout(() => sendLayout(pending), 500);
        }
        return next;
      });
    },
    [current],
  );
  useEffect(
    () => () => {
      if (timer.current) clearTimeout(timer.current);
      sendLayout(pending);
    },
    [],
  );
  return [current, update, saved.isSuccess || saved.isError];
}

/** Save the waiting layout, if any (view state only: a failure never interrupts the writer). */
function sendLayout(pending: { current: Layout | null }): void {
  const state = pending.current;
  pending.current = null;
  if (state) void call("screenplay.set_view_state", { state }).catch(() => undefined);
}
