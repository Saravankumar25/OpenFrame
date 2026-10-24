// "Dark page" must stay readable for every element style on the page (owner issue list 2026-10-24).
// The page colours are custom properties on .spx-page (light) and .spx-page[data-paper="dark"]; this
// reads them from the stylesheet and checks each text/background pair the page uses.

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const css = readFileSync(join(__dirname, "screenplay.css"), "utf8");

/** Every colour custom property declared in the rules with exactly this selector. */
function block(selector: string): Record<string, string> {
  const out: Record<string, string> = {};
  let found = false;
  for (let i = css.indexOf(selector + " {"); i >= 0; i = css.indexOf(selector + " {", i + 1)) {
    found = true;
    const body = css.slice(i + selector.length + 2, css.indexOf("}", i));
    for (const m of body.matchAll(/(--[a-z-]+):\s*(#[0-9a-fA-F]{6})/g)) out[m[1]] = m[2];
  }
  expect(found, selector).toBe(true);
  return out;
}

const light: Record<string, string> = { "--paper": "#ffffff", "--paper-ink": "#111111", ...block(".spx-page") };
const dark: Record<string, string> = { ...light, ...block('.spx-page[data-paper="dark"]') };

function lum(hex: string): number {
  const c = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255).map((v) => (v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4));
  return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
}
const contrast = (a: string, b: string) => {
  const [hi, lo] = [lum(a), lum(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
};

// [text colour, background, what it is]. The placeholder is a hint, so the readable minimum there is 3:1.
const PAIRS: [string, string, string, number][] = [
  ["--paper-ink", "--paper", "script text", 4.5],
  ["--paper-muted", "--paper", "scene numbers", 4.5],
  ["--paper-faint", "--paper", "page-break label", 4.5],
  ["--paper-note-ink", "--paper-note-bg", "note", 4.5],
  ["--paper-note-ink", "--paper-note-cur-bg", "note under the cursor", 4.5],
  ["--paper-ink", "--paper-cur-bg", "line under the cursor", 4.5],
  ["--paper-ink", "--paper-cmt-bg", "commented text", 4.5],
  ["--paper-hl-ink", "--paper-hl-bg", "find match", 4.5],
  ["--paper-hl-ink", "--paper-hlc-bg", "current find match", 4.5],
  ["--paper-sel-ink", "--paper-sel-bg", "selection", 4.5],
  ["--paper-hint", "--paper", "placeholder", 3],
];

describe("dark page colours", () => {
  it("is dark: the page is darker than the text", () => {
    expect(lum(dark["--paper"])).toBeLessThan(0.05);
    expect(lum(dark["--paper-ink"])).toBeGreaterThan(0.6);
  });

  for (const [fg, bg, what, min] of PAIRS) {
    it(`keeps ${what} readable on the dark page (${fg} on ${bg})`, () => {
      expect(contrast(dark[fg], dark[bg])).toBeGreaterThanOrEqual(min);
    });
  }

  it("leaves the light page exactly as it was", () => {
    expect(light["--paper-muted"]).toBe("#555555");
    expect(light["--paper-note-bg"]).toBe("#eee8fb");
    expect(light["--paper-hl-bg"]).toBe("#ffe27a");
    expect(light["--paper"]).toBe("#ffffff");
  });

  it("changes the screen only: nothing in the dark rules is a print or PDF rule", () => {
    expect(css).not.toMatch(/@media print[^{]*\{[^}]*data-paper/);
  });
});
