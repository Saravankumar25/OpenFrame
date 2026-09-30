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

export interface Layout {
  mode: Mode;
  /** Right-hand panels (Standard: at most one; Writing Room: at most two). */
  panels: PanelId[];
  showNotes: boolean;
  /** The draft the writer last had open (defaults to the Current draft). */
  draftId: string | null;
}

export const DEFAULT_LAYOUT: Layout = { mode: "standard", panels: [], showNotes: true, draftId: null };

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
  };
}

/** Toggle a panel respecting the mode's limit (oldest panel closes first). */
export function togglePanel(l: Layout, p: PanelId): Layout {
  if (l.panels.includes(p)) return { ...l, panels: l.panels.filter((x) => x !== p) };
  const max = l.mode === "writingRoom" ? 2 : 1;
  const mode = l.mode === "focus" ? "standard" : l.mode;
  return { ...l, mode, panels: [...l.panels, p].slice(-max) };
}

/** The writer's layout, remembered per project and user (never project content). */
export function useLayout(): [Layout, (f: (l: Layout) => Layout) => void, boolean] {
  const saved = useOp<unknown>("screenplay.view_state", {}, [], { staleTime: Infinity });
  const [layout, setLayout] = useState<Layout | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const current = layout ?? (saved.isSuccess ? normalizeLayout(saved.data) : DEFAULT_LAYOUT);
  const update = useCallback(
    (f: (l: Layout) => Layout) => {
      setLayout((prev) => {
        const next = f(prev ?? current);
        if (timer.current) clearTimeout(timer.current);
        timer.current = setTimeout(() => {
          void call("screenplay.set_view_state", { state: next }).catch(() => undefined);
        }, 500);
        return next;
      });
    },
    [current],
  );
  useEffect(() => () => {
    if (timer.current) clearTimeout(timer.current);
  }, []);
  return [current, update, saved.isSuccess || saved.isError];
}
