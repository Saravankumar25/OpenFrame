// Theme engine (ADR-0014; features THEME-001, THEME-003, THEME-004, THEME-008).

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  BUILT_IN_THEMES,
  DEFAULT_THEME_ID,
  ENGINE,
  PREVIEW_ID,
  applyAppearance,
  applyStartupHint,
  buildStyleLayer,
  followSystem,
  safeColour,
  sanitize,
  setPreviewTheme,
  type ThemeData,
} from "./engine";

const SRC = join(__dirname, "..");

function walk(dir: string, out: string[] = []): string[] {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) {
      if (name !== "generated" && name !== "node_modules") walk(p, out);
    } else if (/\.(css|tsx?)$/.test(name) && !/\.test\.tsx?$/.test(name)) out.push(p);
  }
  return out;
}

/** A fake prefers-color-scheme query whose answer the test controls. */
function fakeSystem(dark: boolean) {
  const listeners = new Set<() => void>();
  const q = {
    matches: dark,
    addEventListener: (_: string, cb: () => void) => listeners.add(cb),
    removeEventListener: (_: string, cb: () => void) => listeners.delete(cb),
  };
  vi.stubGlobal("matchMedia", () => q);
  window.matchMedia = (() => q) as unknown as typeof window.matchMedia;
  return {
    set(next: boolean) {
      q.matches = next;
      listeners.forEach((l) => l());
    },
  };
}

const hostile = (over: Partial<ThemeData>): ThemeData => ({ ...BUILT_IN_THEMES[3], id: "mine-test", builtIn: false, light: {}, dark: {}, ...over });

beforeEach(() => {
  localStorage.clear();
  document.head.querySelector("#of-theme-layer")?.remove();
});
afterEach(() => vi.unstubAllGlobals());

describe("built-in themes come from one list", () => {
  it("has OpenFrame Default first and the fifteen themes from the reference mocks", () => {
    expect(BUILT_IN_THEMES[0].id).toBe(DEFAULT_THEME_ID);
    // Black and White (UX brief 2026-10-20) sits right after the default so it is easy to find.
    expect(BUILT_IN_THEMES[1].id).toBe("black-and-white");
    expect(BUILT_IN_THEMES.map((t) => t.id).slice(2)).toEqual([
      "studio-amber", "slate-noir", "midnight-reel", "teal-orange", "courier-cream", "cutting-room", "velvet-curtain",
      "sunset-boulevard", "arctic-frame", "neon-reel", "location-scout", "desert-dune", "colorist-neutral", "silver-screen", "high-contrast",
    ]);
    expect(new Set(BUILT_IN_THEMES.map((t) => t.id)).size).toBe(17);
    for (const t of BUILT_IN_THEMES) expect(sanitize(t), t.id).not.toBeNull();
  });

  it("adding a theme is one list entry: the layer gets its block without any component change", () => {
    const extra: ThemeData = { ...BUILT_IN_THEMES.find((t) => t.id === "slate-noir")!, id: "added-later", name: "Added Later", builtIn: true };
    const css = buildStyleLayer([...BUILT_IN_THEMES, extra]);
    expect(css).toContain('[data-theme="added-later"]{--h:265;');
    for (const t of BUILT_IN_THEMES) expect(css).toContain(`[data-theme="${t.id}"]{`);
  });
});

describe("OpenFrame Default stays today's look", () => {
  // The palette openframe.css held before the theme system (git: apps/desktop/src/styles/openframe.css at session-2),
  // with the three colours raised to 4.5 to 1 by the owner decision of 2026-10-07 (ADR-0014 amendment):
  // muted #7a8291 to #666e7c, accent-d #b96a1a to #a15c17, button text on the accent #ffffff to #1e2330.
  const TODAY: Record<string, string> = {
    bg: "#f3f1ec", panel: "#ffffff", ink: "#1e2330", ink2: "#48505f", muted: "#666e7c", line: "#e1ddd4", line2: "#cfc9bd",
    side: "#1a1d26", side2: "#252936", "side-ink": "#c5cad6", "side-mut": "#7f8799", accent: "#d9822b", "accent-d": "#a15c17",
    "accent-soft": "#fcefdc", "accent-ink": "#1e2330", blue: "#3b6fd4", "blue-soft": "#e6eefc", green: "#2f8f5b", "green-soft": "#e0f3e8",
    red: "#c8443a", "red-soft": "#fbe6e3", purple: "#7a5ac8", "purple-soft": "#eee8fb", yellow: "#d9a520", "yellow-soft": "#fcf2d2",
    teal: "#1f8a8a", "teal-soft": "#dcf1f1", backdrop: "#22252d",
  };
  it("sets every one of today's palette colours exactly in Light mode", () => {
    const light = BUILT_IN_THEMES[0].light;
    for (const [token, value] of Object.entries(TODAY)) expect(light[token], token).toBe(value);
  });

  it("defines every token and role the screens use, so nothing falls back to a browser default", () => {
    const css = buildStyleLayer(BUILT_IN_THEMES);
    const defined = new Set<string>(["paper", "paper-ink", "shadow", "shadow-s"]);
    for (const m of css.matchAll(/--([a-z0-9-]+):/g)) defined.add(m[1]);
    // Local helper properties a stylesheet declares for itself (for example --ap-ring on the theme grid).
    for (const file of walk(SRC).filter((f) => f.endsWith(".css"))) {
      for (const m of readFileSync(file, "utf8").matchAll(/(?:^|[{;\s])--([a-z0-9-]+)\s*:/g)) defined.add(m[1]);
    }
    const used = new Map<string, string>();
    // Layout variables, not colours: the Beat Board zoom factor and the screenplay and Breakdown page zoom.
    const layout = new Set(["z", "sp-zoom", "bd-zoom"]);
    for (const file of walk(SRC)) {
      if (file.endsWith(join("theme", "engine.ts"))) continue; // documents var(--token) in prose
      const text = readFileSync(file, "utf8").replace(/\/\*[\s\S]*?\*\//g, "").replace(/^\s*\/\/.*$/gm, "");
      for (const m of text.matchAll(/var\(--([a-z0-9-]+)/g)) {
        if (layout.has(m[1]) || m[1].startsWith("radix-") || m[1].startsWith("tw-") || m[1].startsWith("color-") || m[1].startsWith("spacing")) continue;
        used.set(m[1], relative(SRC, file));
      }
    }
    const missing = [...used].filter(([t]) => !defined.has(t)).map(([t, f]) => `--${t} (${f})`);
    expect(missing).toEqual([]);
    // Every role has an exact OpenFrame Default value.
    for (const role of Object.keys(ENGINE.roles)) expect(BUILT_IN_THEMES[0].light[role], role).toBeTruthy();
  });
});

describe("the style layer only ever contains checked values", () => {
  it("accepts only #hex and bounded oklch()", () => {
    for (const ok of ["#fff", "#a0b1c2", "#d9822b33", "oklch(62% 0.15 62)", "oklch(55.5% .15 238)"]) expect(safeColour(ok), ok).toBe(true);
    for (const bad of ["red", "url(x)", "#fff;", "#fff}body{", "oklch(62% 0.15 62);x", "oklch(120% 0.1 10)", "oklch(50% 0.9 10)", "var(--ink)", "expression(1)", "", "#ggg"]) {
      expect(safeColour(bad), bad).toBe(false);
    }
  });

  it("drops a theme with a hostile id, value, token name or seed", () => {
    const cases: ThemeData[] = [
      hostile({ id: 'x"]{}body{display:none}[x="' }),
      hostile({ id: "UPPER" }),
      hostile({ light: { bg: "url(https://example.com/a.png)" } }),
      hostile({ light: { bg: "#fff;color:red" } }),
      hostile({ dark: { "bg}body{": "#ffffff" } }),
      hostile({ seeds: { ...BUILT_IN_THEMES[3].seeds, neutralHue: Number.NaN } }),
      hostile({ seeds: { ...BUILT_IN_THEMES[3].seeds, accentChroma: 9 } }),
      hostile({ seeds: { ...BUILT_IN_THEMES[3].seeds, neutralTint: "0.01" as unknown as number } }),
      hostile({ light: { h: "#ffffff" } }),
      hostile({ dark: { "beat-ink": "#ffffff" } }),
      hostile({ light: { bg: "#ffffff80" } }),
    ];
    for (const t of cases) {
      expect(sanitize(t)).toBeNull();
      const css = buildStyleLayer([t]);
      expect(css).not.toMatch(/url\(|display:none|color:red|NaN/);
    }
  });

  it("writes the layer as text content, never as markup", () => {
    applyAppearance("light", DEFAULT_THEME_ID, [hostile({ id: "evil-one", name: "<img src=x onerror=alert(1)>" })]);
    const el = document.getElementById("of-theme-layer")!;
    expect(el.tagName).toBe("STYLE");
    expect(el.children.length).toBe(0);
    expect(el.textContent).not.toContain("<img");
  });

  it("shows an import preview only under its own id", () => {
    setPreviewTheme(hostile({ id: "anything", light: { accent: "oklch(60% 0.2 140)" } }));
    const css = document.getElementById("of-theme-layer")!.textContent ?? "";
    expect(css).toContain(`[data-theme="${PREVIEW_ID}"][data-mode="light"]{--accent:oklch(60% 0.2 140)}`);
    setPreviewTheme(null);
    expect(document.getElementById("of-theme-layer")!.textContent).not.toContain(PREVIEW_ID);
  });
});

describe("Light, Dark and System", () => {
  it("applies mode and theme as two attributes; an unknown theme shows OpenFrame Default", () => {
    fakeSystem(false);
    applyAppearance("dark", "midnight-reel");
    const root = document.documentElement;
    expect(root.dataset.theme).toBe("midnight-reel");
    expect(root.dataset.mode).toBe("dark");
    applyAppearance("light", "no-such-theme");
    expect(root.dataset.theme).toBe(DEFAULT_THEME_ID);
    expect(root.dataset.mode).toBe("light");
  });

  it("System follows the operating system live, without a restart", () => {
    const os = fakeSystem(false);
    const stop = followSystem();
    applyAppearance("system", "slate-noir");
    const root = document.documentElement;
    expect(root.dataset.mode).toBe("light");
    os.set(true);
    expect(root.dataset.mode).toBe("dark");
    os.set(false);
    expect(root.dataset.mode).toBe("light");
    // A fixed mode ignores the operating system.
    applyAppearance("dark", "slate-noir");
    os.set(false);
    expect(root.dataset.mode).toBe("dark");
    stop();
  });

  it("the start-up hint restores a built-in choice before Rust answers and ignores a damaged one", () => {
    fakeSystem(false);
    applyAppearance("dark", "cutting-room");
    document.documentElement.removeAttribute("data-theme");
    applyStartupHint();
    expect(document.documentElement.dataset.theme).toBe("cutting-room");
    expect(document.documentElement.dataset.mode).toBe("dark");
    localStorage.setItem("openframe.appearance.hint", '{"pref":"sepia","themeId":"x\\"]{"}');
    applyStartupHint();
    expect(document.documentElement.dataset.theme).toBe(DEFAULT_THEME_ID);
    expect(document.documentElement.dataset.pref).toBe("system");
    localStorage.setItem("openframe.appearance.hint", "not json");
    expect(() => applyStartupHint()).not.toThrow();
  });
});

describe("paper never follows the theme", () => {
  it("keeps the screenplay page and call-sheet paper on fixed document colours", () => {
    const css = readFileSync(join(SRC, "styles", "openframe.css"), "utf8");
    expect(css).toMatch(/--paper:#ffffff; --paper-ink:#111111;/);
    // The theme layer never sets the paper tokens, in any theme or mode.
    const layer = buildStyleLayer(BUILT_IN_THEMES);
    expect(layer).not.toMatch(/--paper(-ink)?:/);
    const paper = css.split("\n").find((l) => l.startsWith(".paper{"))!;
    expect(paper).toContain("background:#fff");
    expect(paper).toContain("color:#111");
    expect(paper).not.toMatch(/var\(--(bg|panel|ink|surface)/);
    const sp = readFileSync(join(SRC, "workspaces", "screenplay", "screenplay.css"), "utf8");
    expect(sp).toMatch(/background: var\(--paper\); box-shadow/);
    expect(sp).toMatch(/color: var\(--paper-ink\);/);
  });
});
