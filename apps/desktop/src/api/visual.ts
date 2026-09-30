// Visual planning (Moodboards, Storyboards, Shot Lists): typed wrappers,
// invalidation table sets and small pure helpers shared by the three
// Production tabs. Project truth stays in Rust; nothing here caches it.

import { convertFileSrc } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { inTauri } from "../ipc/client";
import { useOp } from "../ipc/query";
import type { AssetInfo } from "../ipc/generated/AssetInfo";
import type { VisualScenes } from "../ipc/generated/VisualScenes";
import type { VisualScene } from "../ipc/generated/VisualScene";

/** Screenplay/production tables that decide which scenes are planned and their review state. */
export const SCENE_TABLES = [
  "screenplay",
  "screenplay_draft",
  "screenplay_scene",
  "screenplay_element",
  "production_source",
];
export const SHOT_TABLES = [...SCENE_TABLES, "shot", "storyboard", "storyboard_panel", "asset"];
export const STORYBOARD_TABLES = [...SCENE_TABLES, "storyboard", "storyboard_panel", "shot", "asset"];
export const MOODBOARD_TABLES = ["moodboard", "moodboard_item", "asset", ...SCENE_TABLES];

export function useVisualScenes() {
  return useOp<VisualScenes>("visual.scenes", {}, [...SCENE_TABLES, "shot", "storyboard", "moodboard"]);
}

/** "Scene 12 — INT. POLICE STATION — NIGHT" (episode title first when several screenplays are planned). */
export function sceneLabel(s: Pick<VisualScene, "number" | "heading" | "groupLabel">): string {
  const head = s.heading.trim() || "Untitled scene";
  return `${s.groupLabel ? `${s.groupLabel} · ` : ""}Scene ${s.number} — ${head}`;
}

/** URL for showing a project file in the webview (restricted asset protocol). */
export function assetSrc(asset: AssetInfo | null | undefined): string | null {
  if (!asset || !asset.available || !asset.path || !inTauri()) return null;
  try {
    return convertFileSrc(asset.path);
  } catch {
    return null;
  }
}

export const IMAGE_EXTENSIONS = ["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg"];

/** Ask the user for image files (the chosen paths are validated in Rust). */
export async function pickImages(multiple: boolean): Promise<string[]> {
  const chosen = await open({ multiple, directory: false, filters: [{ name: "Images", extensions: IMAGE_EXTENSIONS }] });
  if (!chosen) return [];
  return Array.isArray(chosen) ? chosen : [chosen];
}

/** Read a Blob (pasted image) as base64 without the data: prefix. */
export function blobToBase64(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const r = new FileReader();
    r.onerror = () => reject(r.error ?? new Error("read failed"));
    r.onload = () => {
      const s = String(r.result ?? "");
      const i = s.indexOf("base64,");
      resolve(i >= 0 ? s.slice(i + 7) : s);
    };
    r.readAsDataURL(blob);
  });
}

/** Move an element within a list (drag reorder preview). */
export function moveItem<T>(list: readonly T[], from: number, to: number): T[] {
  const out = list.slice();
  if (from < 0 || from >= out.length) return out;
  const [x] = out.splice(from, 1);
  out.splice(Math.max(0, Math.min(to, out.length)), 0, x);
  return out;
}

/** Shot letter for a zero-based order index (mirrors Rust; display only): A…Z, AA, AB… */
export function shotLetters(index: number): string {
  let n = index + 1;
  let out = "";
  while (n > 0) {
    const r = (n - 1) % 26;
    out = String.fromCharCode(65 + r) + out;
    n = Math.floor((n - 1) / 26);
  }
  return out;
}

/** Common values offered as suggestions (free text is always allowed; FSD §103 no taxonomy). */
export const SHOT_SIZES = ["Wide", "Full", "Medium", "Medium close", "Close", "Extreme close", "Two-shot", "OTS", "Insert", "POV", "Establishing"];
export const MOVEMENTS = ["Static", "Handheld", "Pan", "Tilt", "Dolly", "Tracking", "Crane", "Steadicam", "Zoom"];
export const ANGLES = ["Eye level", "Low", "High", "Overhead", "Dutch", "Ground level"];

/** "3 Oct, 14:02" for review indicators (FSD §125: say when). */
export function formatWhen(ms: number | null | undefined): string {
  if (!ms) return "";
  const d = new Date(ms);
  return d.toLocaleString(undefined, { day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" });
}

/** Seconds text → milliseconds (0 = clear). Returns null when the text is not a valid duration. */
export function parseSeconds(text: string): number | null {
  const t = text.trim().replace(",", ".").replace(/s(ec(onds?)?)?$/i, "").trim();
  if (t === "") return 0;
  const v = Number(t);
  if (!Number.isFinite(v) || v < 0 || v > 3600) return null;
  return Math.round(v * 1000);
}

export function formatSeconds(ms: number | null | undefined): string {
  if (!ms) return "";
  const s = ms / 1000;
  return Number.isInteger(s) ? String(s) : s.toFixed(1);
}

/** Crude but safe URL detection for pasted text. */
export function looksLikeUrl(text: string): boolean {
  const t = text.trim();
  return /^(https?:\/\/)?[\w-]+(\.[\w-]+)+(\/\S*)?$/i.test(t) && !/\s/.test(t);
}
