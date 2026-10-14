// A dialog footer holds a note on the left and the buttons on the right. With many buttons (the
// Import Exchange Package preview has six) the note was squeezed to a sliver one word wide and
// clipped at the dialog's edge. The footer now wraps and the note keeps a readable width, so it
// moves to its own line instead.

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const css = readFileSync(join(__dirname, "openframe.css"), "utf8");
const rule = (selector: string) => {
  const at = css.indexOf(`${selector}{`);
  expect(at, `no rule for ${selector}`).toBeGreaterThanOrEqual(0);
  return css.slice(at + selector.length + 1, css.indexOf("}", at));
};

describe("dialog footer", () => {
  it("wraps rather than crushing the note beside the buttons", () => {
    expect(rule(".dialog .df")).toContain("flex-wrap:wrap");
    expect(rule(".dialog .df .l")).toMatch(/min-width:min\(\d+px,100%\)/);
  });
});
