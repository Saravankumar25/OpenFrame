// Stripboard strips keep the fixed day and night paper colours in every theme (white, yellow,
// blue). Their text must therefore be a fixed dark colour too: inheriting the theme's text colour
// drew the scene number, INT/EXT and location in light text on a light strip in dark themes.

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const css = readFileSync(join(__dirname, "openframe.css"), "utf8");

/** The declarations of the first rule whose selector list is exactly `selector`. */
function rule(selector: string): string {
  const at = css.indexOf(`${selector}{`);
  expect(at, `no rule for ${selector}`).toBeGreaterThanOrEqual(0);
  return css.slice(at + selector.length + 1, css.indexOf("}", at));
}

/** Relative luminance of a #rgb or #rrggbb colour (WCAG 2). */
function luminance(hex: string): number {
  const h = hex.length === 4 ? [...hex.slice(1)].map((c) => c + c).join("") : hex.slice(1, 7);
  const [r, g, b] = [0, 2, 4].map((i) => {
    const c = parseInt(h.slice(i, i + 2), 16) / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

describe("stripboard strip colours", () => {
  it("set their own text colour, dark enough on every strip paper", () => {
    const base = rule(".strip");
    const text = /(?:^|;)color:(#[0-9a-fA-F]{3,6})/.exec(base)?.[1];
    expect(text, ".strip must set a fixed text colour").toBeTruthy();
    const papers = [...css.matchAll(/\.strip\.(?:ied2?|ien|xd|xn|id|in)\{background:(#[0-9a-fA-F]{6})/g)].map((m) => m[1]);
    expect(papers.length).toBeGreaterThanOrEqual(6);
    for (const paper of ["#ffffff", ...papers]) {
      const [hi, lo] = [luminance(paper), luminance(text!)].sort((a, b) => b - a);
      expect((hi + 0.05) / (lo + 0.05), `contrast of ${text} on ${paper}`).toBeGreaterThanOrEqual(4.5);
    }
  });
});
