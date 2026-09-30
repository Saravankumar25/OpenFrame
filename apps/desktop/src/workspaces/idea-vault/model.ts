// Pure presentation logic for the Idea Vault (no React, no IPC) — unit tested.

import type { VaultItemDto } from "../../ipc/generated/VaultItemDto";
import type { VaultItemType } from "../../ipc/generated/VaultItemType";
import type { VaultFolderDto } from "../../ipc/generated/VaultFolderDto";
import type { VaultCollectionDto } from "../../ipc/generated/VaultCollectionDto";
import type { VaultView } from "../../ipc/generated/VaultView";
import type { StoreSel } from "../../ipc/generated/StoreSel";

export type Scope = StoreSel;
export type DisplayMode = "grid" | "card" | "list" | "folder";

/** Tables every vault query depends on (drives invalidation from Rust events). */
export const VAULT_TABLES = ["vault_item", "vault_item_tag", "vault_item_collection", "vault_collection", "vault_folder", "asset"];

export const TYPE_LABEL: Record<VaultItemType, string> = {
  note: "Note",
  image: "Image",
  url: "URL",
  pdf: "PDF",
  document: "Document",
  audio: "Audio",
  voice: "Voice note",
  video: "Video",
  sketch: "Sketch",
  quote: "Quote",
  screenshot: "Screenshot",
  file: "File",
};

export const isTextType = (t: VaultItemType) => t === "note" || t === "quote";
export const isVisualType = (t: VaultItemType) => t === "image" || t === "screenshot" || t === "sketch";
export const isPlayableType = (t: VaultItemType) => t === "audio" || t === "voice" || t === "video";

export function scopeLabel(scope: Scope): string {
  return scope === "global" ? "Global Idea Vault" : "Project Idea Vault";
}

/** "0:42", "12:05", "1:02:03". */
export function formatDuration(ms: number | null | undefined): string | null {
  if (ms == null || !Number.isFinite(ms) || ms < 0) return null;
  const total = Math.round(ms / 1000);
  const h = Math.floor(total / 3600);
  const m = Math.floor(total / 60) % 60;
  const s = total % 60;
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}` : `${m}:${String(s).padStart(2, "0")}`;
}

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/** "today", "yesterday", "12 Sep", "12 Sep 2024" (mock 026 date style). */
export function relativeDay(ms: number, now: number = Date.now()): string {
  const d = new Date(ms);
  const n = new Date(now);
  const startOf = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const days = Math.round((startOf(n) - startOf(d)) / 86_400_000);
  if (days === 0) return "today";
  if (days === 1) return "yesterday";
  const month = MONTHS[d.getMonth()];
  return d.getFullYear() === n.getFullYear() ? `${d.getDate()} ${month}` : `${d.getDate()} ${month} ${d.getFullYear()}`;
}

export function formatBytes(n: number | null | undefined): string | null {
  if (n == null) return null;
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(0)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
  return `${(n / 1024 / 1024 / 1024).toFixed(1)} GB`;
}

/** Short extension label for file cards ("PDF", "DOCX"). */
export function extLabel(name: string | undefined): string {
  const m = /\.([a-z0-9]{1,5})$/i.exec(name ?? "");
  return m ? m[1].toUpperCase() : "FILE";
}

export function hostOf(url: string | null | undefined): string {
  if (!url) return "";
  const m = /^[a-z]+:\/\/([^/?#]+)(.*)$/i.exec(url);
  if (!m) return url;
  const rest = m[2] && m[2] !== "/" ? m[2] : "";
  return `${m[1]}${rest}`.slice(0, 80);
}

/**
 * Card meta line (mock 024): "Voice note · 0:42 · today", "Image · Visual References".
 * Type first, then a duration for playable items, then the first collection, first tag
 * or the day it was added.
 */
export function metaLine(item: VaultItemDto, collections: Pick<VaultCollectionDto, "id" | "name">[], now?: number): string {
  const parts: string[] = [TYPE_LABEL[item.itemType]];
  const dur = isPlayableType(item.itemType) ? formatDuration(item.asset?.durationMs) : null;
  if (dur) parts.push(dur);
  const coll = item.collectionIds.map((id) => collections.find((c) => c.id === id)?.name).find(Boolean);
  if (coll) parts.push(coll);
  else if (item.tags.length > 0) parts.push(item.tags[0]);
  else parts.push(relativeDay(item.createdAt, now));
  return parts.join(" · ");
}

export function isUnavailable(item: VaultItemDto): boolean {
  return !!item.asset && !item.asset.available;
}

export function isExternal(item: VaultItemDto): boolean {
  return item.asset?.storageMode === "external";
}

export function viewKey(v: VaultView): string {
  switch (v.kind) {
    case "folder":
      return `folder:${v.folderId ?? ""}`;
    case "collection":
      return `collection:${v.collectionId}`;
    case "tag":
      return `tag:${v.tag.toLowerCase()}`;
    default:
      return v.kind;
  }
}

export function sameView(a: VaultView, b: VaultView): boolean {
  return viewKey(a) === viewKey(b);
}

// ----------------------------------------------------------------- folders

/** Folder chain from the top level to `id` (inclusive), for breadcrumbs. */
export function folderPath(folders: VaultFolderDto[], id: string | null): VaultFolderDto[] {
  const byId = new Map(folders.map((f) => [f.id, f]));
  const out: VaultFolderDto[] = [];
  let cur = id ? byId.get(id) : undefined;
  const seen = new Set<string>();
  while (cur && !seen.has(cur.id)) {
    seen.add(cur.id);
    out.unshift(cur);
    cur = cur.parentId ? byId.get(cur.parentId) : undefined;
  }
  return out;
}

export function childFolders(folders: VaultFolderDto[], parentId: string | null): VaultFolderDto[] {
  return folders.filter((f) => (f.parentId ?? null) === parentId);
}

/** Depth-first flattened tree for indented pickers and the left panel. */
export function folderTree(folders: VaultFolderDto[]): { folder: VaultFolderDto; depth: number }[] {
  const ids = new Set(folders.map((f) => f.id));
  const out: { folder: VaultFolderDto; depth: number }[] = [];
  const walk = (parent: string | null, depth: number) => {
    for (const f of folders.filter((x) => (x.parentId && ids.has(x.parentId) ? x.parentId : null) === parent)) {
      out.push({ folder: f, depth });
      if (depth < 12) walk(f.id, depth + 1);
    }
  };
  walk(null, 0);
  return out;
}

// ------------------------------------------------------------- virtual rows

export type VaultRow =
  | { key: string; kind: "header"; label: string }
  | { key: string; kind: "items"; items: VaultItemDto[] }
  | { key: string; kind: "folders"; folders: VaultFolderDto[] };

function chunk<T>(xs: T[], n: number): T[][] {
  const size = Math.max(1, n);
  const out: T[][] = [];
  for (let i = 0; i < xs.length; i += size) out.push(xs.slice(i, i + size));
  return out;
}

/**
 * Rows for the virtualized Grid/Card/Folder canvases. Grid view splits Pinned from
 * "Everything else" (mock 024); Folder view starts with sub-folder tiles (mock 027).
 */
export function buildRows(
  items: VaultItemDto[],
  columns: number,
  opts: { splitPinned?: boolean; folders?: VaultFolderDto[]; itemsHeader?: string } = {},
): VaultRow[] {
  const rows: VaultRow[] = [];
  if (opts.folders && opts.folders.length > 0) {
    chunk(opts.folders, columns).forEach((fs, i) => rows.push({ key: `f${i}`, kind: "folders", folders: fs }));
    if (items.length > 0) rows.push({ key: "h-items", kind: "header", label: opts.itemsHeader ?? "Items in this folder" });
  } else if (opts.itemsHeader && items.length > 0) {
    rows.push({ key: "h-items", kind: "header", label: opts.itemsHeader });
  }
  const pinned = opts.splitPinned ? items.filter((i) => i.pinned) : [];
  if (pinned.length > 0) {
    const rest = items.filter((i) => !i.pinned);
    rows.push({ key: "h-pinned", kind: "header", label: "Pinned" });
    chunk(pinned, columns).forEach((xs, i) => rows.push({ key: `p${i}`, kind: "items", items: xs }));
    if (rest.length > 0) {
      rows.push({ key: "h-rest", kind: "header", label: "Everything else" });
      chunk(rest, columns).forEach((xs, i) => rows.push({ key: `r${i}`, kind: "items", items: xs }));
    }
  } else {
    chunk(items, columns).forEach((xs, i) => rows.push({ key: `a${i}`, kind: "items", items: xs }));
  }
  return rows;
}

// ---------------------------------------------------------------- selection

/** Click / Ctrl+click / Shift+click selection over an ordered list of ids. */
export function nextSelection(
  current: ReadonlySet<string>,
  ordered: string[],
  id: string,
  anchor: string | null,
  mods: { ctrl?: boolean; shift?: boolean },
): Set<string> {
  if (mods.shift && anchor && ordered.includes(anchor)) {
    const a = ordered.indexOf(anchor);
    const b = ordered.indexOf(id);
    const [lo, hi] = a < b ? [a, b] : [b, a];
    const range = new Set(mods.ctrl ? current : []);
    ordered.slice(lo, hi + 1).forEach((x) => range.add(x));
    return range;
  }
  if (mods.ctrl) {
    const s = new Set(current);
    if (s.has(id)) s.delete(id);
    else s.add(id);
    return s;
  }
  return new Set([id]);
}

// ------------------------------------------------------------------- bytes

/** Base64 of binary data, chunked to stay within argument limits. */
export function bytesToBase64(bytes: Uint8Array): string {
  let s = "";
  const step = 0x8000;
  for (let i = 0; i < bytes.length; i += step) {
    s += String.fromCharCode(...bytes.subarray(i, i + step));
  }
  return btoa(s);
}

/** Split a comma/newline separated tag string into clean tags. */
export function parseTags(text: string): string[] {
  const out: string[] = [];
  for (const raw of text.split(/[,\n]/)) {
    const t = raw.trim().replace(/^#+/, "").replace(/\s+/g, " ");
    if (t && !out.some((o) => o.toLowerCase() === t.toLowerCase())) out.push(t);
  }
  return out;
}

/** Summary text for batch toasts: “Old platform” or "3 items". */
export function itemsPhrase(items: Pick<VaultItemDto, "displayName">[]): string {
  return items.length === 1 ? `“${items[0].displayName}”` : `${items.length} items`;
}

export const FILE_FILTERS = {
  image: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "gif", "webp", "bmp", "heic", "svg"] }],
  video: [{ name: "Video", extensions: ["mp4", "mov", "m4v", "mkv", "avi", "webm"] }],
  audio: [{ name: "Audio", extensions: ["mp3", "wav", "m4a", "ogg", "oga", "flac", "webm", "aac"] }],
} as const;
