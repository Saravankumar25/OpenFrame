// Theme engine (ADR-0014; docs/engineering/30-appearance-themes.md section 3).
//
// One list: the engine constants and the built-in themes come from the same themes.json the Rust
// validator reads. This module turns validated theme data into ONE generated style layer
// (<style id="of-theme-layer">) and sets two attributes on <html>:
//   data-theme = theme id    data-mode = light | dark (System is resolved here, live)
// Components only use var(--token). Adding a built-in theme is one entry in themes.json.
//
// Imported themes arrive from Rust already validated; every value is checked again here against a
// strict pattern before it is written, so nothing but numbers, ids and `#hex` / `oklch()` colours
// can reach the style layer.

import themesFile from "../../../../crates/openframe-application/src/modules/appearance/themes.json";
import type { ThemeDto } from "../ipc/generated/ThemeDto";
import type { AppearanceMode } from "../ipc/generated/AppearanceMode";

export type ResolvedMode = "light" | "dark";
export const DEFAULT_THEME_ID = "openframe-default";
const LAYER_ID = "of-theme-layer";

interface Engine {
  ladder: Record<ResolvedMode, Record<string, [number, number]>>;
  accentLightness: Record<ResolvedMode, number>;
  statusLightness: Record<ResolvedMode, number>;
  softMix: Record<ResolvedMode, number>;
  darkAccentInk: [number, number];
  lightAccentInk: string;
  accentStrongMix: number;
  accentLineMix: number;
  accentRingMix: number;
  statusTextMix: number;
  warningTextMix: number;
  statusLineMix: number;
  warningLineMix: number;
  status: Record<string, [number, number, number]>;
  roles: Record<string, string>;
  logo: Record<string, string>;
  /** Tokens an imported theme may set (the same list the Rust validator uses). */
  importableTokens: string[];
}

/** A theme as the engine needs it (built-in entries of themes.json and ThemeDto share this shape). */
export interface ThemeData {
  id: string;
  name: string;
  description: string;
  author: string;
  builtIn: boolean;
  seeds: ThemeDto["seeds"];
  light: Record<string, string>;
  dark: Record<string, string>;
}

interface FileTheme {
  id: string;
  name: string;
  description: string;
  author: string;
  seeds: ThemeDto["seeds"];
  overrides?: { light?: Record<string, string>; dark?: Record<string, string> };
}

const file = themesFile as unknown as { engine: Engine; themes: FileTheme[] };
export const ENGINE: Engine = file.engine;

/** OpenFrame Default first, then the fifteen built-in themes, in picker order. */
export const BUILT_IN_THEMES: ThemeData[] = file.themes.map((t) => ({
  id: t.id,
  name: t.name,
  description: t.description,
  author: t.author,
  builtIn: true,
  seeds: t.seeds,
  light: t.overrides?.light ?? {},
  dark: t.overrides?.dark ?? {},
}));

// ------------------------------------------------------------------ value guards

const ID_RE = /^[a-z0-9-]{3,40}$/;
const TOKEN_RE = /^[a-z0-9-]{1,40}$/;
const HEX_RE = /^#(?:[0-9a-f]{3}|[0-9a-f]{4}|[0-9a-f]{6}|[0-9a-f]{8})$/i;
const OKLCH_RE = /^oklch\((\d{1,3}(?:\.\d{1,4})?)% (\d?\.?\d{1,4}) (\d{1,3}(?:\.\d{1,4})?)\)$/;

/** True for a colour the style layer may contain. */
export function safeColour(v: unknown): v is string {
  if (typeof v !== "string") return false;
  if (HEX_RE.test(v)) return true;
  const m = OKLCH_RE.exec(v);
  if (!m) return false;
  const [l, c, h] = [Number(m[1]), Number(m[2]), Number(m[3])];
  return l >= 0 && l <= 100 && c >= 0 && c <= 0.4 && h >= 0 && h <= 360;
}

function num(v: unknown, min: number, max: number): number | null {
  return typeof v === "number" && Number.isFinite(v) && v >= min && v <= max ? v : null;
}

/** The theme with every value checked, or null if anything is out of bounds. */
export function sanitize(t: ThemeData): ThemeData | null {
  if (!ID_RE.test(t.id)) return null;
  const s = t.seeds;
  const seeds = {
    neutralHue: num(s?.neutralHue, 0, 360),
    neutralTint: num(s?.neutralTint, 0, 0.04),
    accentHue: num(s?.accentHue, 0, 360),
    accentChroma: num(s?.accentChroma, 0, 0.25),
    accentLightnessNudge: num(s?.accentLightnessNudge, -12, 16),
  };
  if (Object.values(seeds).some((v) => v === null)) return null;
  const side = (m: Record<string, string> | undefined) => {
    const out: Record<string, string> = {};
    for (const [k, v] of Object.entries(m ?? {})) {
      if (!TOKEN_RE.test(k) || !safeColour(v)) return null;
      // Imported themes: only the allowed tokens and only #rgb, #rrggbb or oklch() (second check after Rust).
      if (!t.builtIn && (!ENGINE.importableTokens.includes(k) || /^#(?:[0-9a-f]{4}|[0-9a-f]{8})$/i.test(v))) return null;
      out[k] = v;
    }
    return out;
  };
  const light = side(t.light);
  const dark = side(t.dark);
  if (!light || !dark) return null;
  return { ...t, seeds: seeds as ThemeData["seeds"], light, dark };
}

// ------------------------------------------------------------------ style layer

const fmt = (n: number) => String(Math.round(n * 10000) / 10000);

function ladderBlock(mode: ResolvedMode): string {
  const e = ENGINE;
  const lines = [`color-scheme:${mode}`];
  for (const [name, [l, k]] of Object.entries(e.ladder[mode])) {
    lines.push(`--${name}:oklch(${fmt(l)}% calc(var(--c) * ${fmt(k)}) var(--h))`);
  }
  lines.push(`--accent:oklch(calc(${fmt(e.accentLightness[mode])}% + var(--ak)) var(--ac) var(--ah))`);
  lines.push(
    mode === "light"
      ? `--accent-ink:${e.lightAccentInk}`
      : `--accent-ink:oklch(${fmt(e.darkAccentInk[0])}% ${fmt(e.darkAccentInk[1])} var(--ah))`,
  );
  lines.push(`--mix:${fmt(e.softMix[mode])}%`);
  lines.push(`--sl:${fmt(e.statusLightness[mode])}%`);
  for (const [name, [dl, c, h]] of Object.entries(e.status)) {
    lines.push(`--${name}:oklch(${fmt(e.statusLightness[mode] + dl)}% ${fmt(c)} ${fmt(h)})`);
  }
  return `[data-mode="${mode}"]{${lines.join(";")}}`;
}

function derivedBlock(): string {
  const e = ENGINE;
  const lines = [
    `--accent-d:color-mix(in oklab,var(--accent) ${fmt(e.accentStrongMix)}%,var(--ink))`,
    `--accent-soft:color-mix(in oklab,var(--accent) var(--mix),var(--panel))`,
    `--accent-line:color-mix(in oklab,var(--accent) ${fmt(e.accentLineMix)}%,var(--panel))`,
    `--accent-ring:color-mix(in oklab,var(--accent) ${fmt(e.accentRingMix)}%,transparent)`,
  ];
  for (const name of Object.keys(e.status)) {
    const text = name === "yellow" ? e.warningTextMix : e.statusTextMix;
    const line = name === "yellow" ? e.warningLineMix : e.statusLineMix;
    lines.push(`--${name}-soft:color-mix(in oklab,var(--${name}) var(--mix),var(--panel))`);
    lines.push(`--${name}-t:color-mix(in oklab,var(--${name}) ${fmt(text)}%,var(--ink))`);
    lines.push(`--${name}-line:color-mix(in oklab,var(--${name}) ${fmt(line)}%,var(--panel))`);
  }
  for (const [name, formula] of Object.entries({ ...e.roles, ...e.logo })) lines.push(`--${name}:${formula}`);
  return `[data-mode]{${lines.join(";")}}`;
}

function themeBlocks(t: ThemeData): string {
  const s = t.seeds;
  const sel = `[data-theme="${t.id}"]`;
  const seeds = `${sel}{--h:${fmt(s.neutralHue)};--c:${fmt(s.neutralTint)};--ah:${fmt(s.accentHue)};--ac:${fmt(s.accentChroma)};--ak:${fmt(s.accentLightnessNudge)}%}`;
  const over = (mode: ResolvedMode, m: Record<string, string>) => {
    const entries = Object.entries(m);
    return entries.length ? `${sel}[data-mode="${mode}"]{${entries.map(([k, v]) => `--${k}:${v}`).join(";")}}` : "";
  };
  return seeds + over("light", t.light) + over("dark", t.dark);
}

/** The whole generated style layer for these themes (built-in and validated imported ones). */
export function buildStyleLayer(themes: ThemeData[], preview: ThemeData | null = null): string {
  const blocks = [ladderBlock("light"), ladderBlock("dark"), derivedBlock()];
  for (const t of themes) {
    const clean = sanitize(t);
    if (clean) blocks.push(themeBlocks(clean));
  }
  // An imported theme being checked: shown only inside preview cards, under an id no theme can have.
  const p = preview && sanitize({ ...preview, id: "preview-theme" });
  if (p) blocks.push(themeBlocks({ ...p, id: PREVIEW_ID }));
  return blocks.join("\n");
}

/** data-theme value of the import preview (outside the id rule, so it never meets a real theme). */
export const PREVIEW_ID = "__preview";

// ------------------------------------------------------------------ applying

const darkQuery = (): MediaQueryList | null =>
  typeof window !== "undefined" && typeof window.matchMedia === "function"
    ? window.matchMedia("(prefers-color-scheme: dark)")
    : null;

export function resolveMode(pref: AppearanceMode): ResolvedMode {
  if (pref === "light" || pref === "dark") return pref;
  return darkQuery()?.matches ? "dark" : "light";
}

let current: { pref: AppearanceMode; themeId: string } = { pref: "system", themeId: DEFAULT_THEME_ID };
let layerKey = "";

/** Write the style layer (only when the theme list changed) and the two attributes. */
let mineNow: ThemeData[] = [];
let previewNow: ThemeData | null = null;

function writeLayer(): ThemeData[] {
  const all = [...BUILT_IN_THEMES, ...mineNow.filter((t) => !t.builtIn && !BUILT_IN_THEMES.some((b) => b.id === t.id))];
  const key = JSON.stringify([all.map((t) => [t.id, t.seeds, t.light, t.dark]), previewNow && [previewNow.seeds, previewNow.light, previewNow.dark]]);
  if (key !== layerKey) {
    let el = document.getElementById(LAYER_ID) as HTMLStyleElement | null;
    if (!el) {
      el = document.createElement("style");
      el.id = LAYER_ID;
      document.head.appendChild(el);
    }
    // textContent (never innerHTML): the layer is plain CSS text built from checked values.
    el.textContent = buildStyleLayer(all, previewNow);
    layerKey = key;
  }
  return all;
}

/** Show (or with null, remove) an imported theme in the preview cards only. */
export function setPreviewTheme(t: ThemeData | null): void {
  previewNow = t;
  writeLayer();
}

export function applyAppearance(pref: AppearanceMode, themeId: string, mine: ThemeData[] = mineNow): void {
  mineNow = mine;
  const all = writeLayer();
  const usable = all.some((t) => t.id === themeId && sanitize(t));
  const id = usable ? themeId : DEFAULT_THEME_ID;
  current = { pref, themeId: id };
  const root = document.documentElement;
  root.setAttribute("data-theme", id);
  root.setAttribute("data-pref", pref);
  root.setAttribute("data-mode", resolveMode(pref));
  rememberHint(pref, id);
}

/** Follow the operating system live while the preference is System. Returns the unsubscribe. */
export function followSystem(): () => void {
  const q = darkQuery();
  if (!q) return () => undefined;
  const onChange = () => {
    if (current.pref === "system") document.documentElement.setAttribute("data-mode", q.matches ? "dark" : "light");
  };
  q.addEventListener("change", onChange);
  return () => q.removeEventListener("change", onChange);
}

// ------------------------------------------------------------------ startup hint

// The app database is the source of truth. A copy of the last built-in choice is kept in
// localStorage only so the first frame already has the right look before Rust answers; it is
// re-checked like any other input and replaced as soon as the real preference arrives.
const HINT_KEY = "openframe.appearance.hint";

function rememberHint(pref: AppearanceMode, themeId: string): void {
  try {
    localStorage.setItem(HINT_KEY, JSON.stringify({ pref, themeId }));
  } catch {
    // Storage can be unavailable; the hint is only a convenience.
  }
}

/** Apply the remembered choice before the first render (built-in themes only). */
export function applyStartupHint(): void {
  let pref: AppearanceMode = "system";
  let themeId = DEFAULT_THEME_ID;
  try {
    const raw = JSON.parse(localStorage.getItem(HINT_KEY) ?? "null") as { pref?: unknown; themeId?: unknown } | null;
    if (raw && (raw.pref === "light" || raw.pref === "dark" || raw.pref === "system")) pref = raw.pref;
    if (raw && typeof raw.themeId === "string" && BUILT_IN_THEMES.some((t) => t.id === raw.themeId)) themeId = raw.themeId;
  } catch {
    // Ignore a damaged hint.
  }
  applyAppearance(pref, themeId);
}
