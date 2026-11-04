// The logo reads "Straft Studio": "Straft" bold, "Studio" light, one colour, the mark to the left.

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { BrandLogo } from "./BrandLogo";

describe("BrandLogo", () => {
  it("has one accessible name and draws the mark before the wordmark", () => {
    render(<BrandLogo />);
    const logo = screen.getByRole("img", { name: "Straft Studio" });
    expect(logo.firstElementChild?.tagName.toLowerCase()).toBe("svg");
    const word = logo.querySelector(".wm");
    expect(word?.textContent).toBe("Straft Studio");
    expect(word?.querySelector("i")?.textContent).toBe("Studio");
  });

  it("draws the two words in two weights and one colour", () => {
    const css = readFileSync(join(__dirname, "..", "styles", "straft.css"), "utf8");
    expect(css).toMatch(/\.brandlogo \.wm\{font:700 [^}]*white-space:nowrap\}/);
    expect(css).toMatch(/\.brandlogo \.wm i\{font-style:normal;font-weight:300\}/);
  });
});
