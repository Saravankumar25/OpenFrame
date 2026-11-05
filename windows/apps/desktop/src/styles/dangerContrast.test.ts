// Danger buttons draw their label on the theme's red. In dark mode that red is light, so white text
// read 2.98:1 (axe-core audit, 2026-10-31); straft.css switches the label to a dark colour there.

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const css = readFileSync(join(__dirname, "straft.css"), "utf8");
const themes = JSON.parse(
  readFileSync(join(__dirname, "../../../../crates/straft-application/src/modules/appearance/themes.json"), "utf8"),
) as { themes: { id: string; overrides: { dark: Record<string, string> } }[] };

/** Relative luminance of a #rrggbb colour (WCAG 2). */
function luminance(hex: string): number {
  const [r, g, b] = [1, 3, 5].map((i) => {
    const c = parseInt(hex.slice(i, i + 2), 16) / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

describe("danger button text in dark mode", () => {
  it("is dark and reads at least 4.5:1 on the default dark red and its hover red", () => {
    const text = /:root\[data-mode="dark"\] \.btn\.dan,[^{]*\{color:(#[0-9a-f]{6})\}/.exec(css)?.[1];
    expect(text, "dark-mode .btn.dan must set its own text colour").toBeTruthy();
    const dark = themes.themes.find((t) => t.id === "straft-default")!.overrides.dark;
    for (const bg of [dark.red, dark["danger-hover"]]) {
      const [hi, lo] = [luminance(bg), luminance(text!)].sort((a, b) => b - a);
      expect((hi + 0.05) / (lo + 0.05), `${text} on ${bg}`).toBeGreaterThanOrEqual(4.5);
    }
  });
});
