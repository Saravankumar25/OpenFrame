// The logo reads "Straft Studio". It is one picture of outlines, so it needs no font on the machine.

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { BrandLogo } from "./BrandLogo";

describe("BrandLogo", () => {
  it("has one accessible name and draws everything as outlines", () => {
    render(<BrandLogo />);
    const logo = screen.getByRole("img", { name: "Straft Studio" });
    expect(logo.firstElementChild?.tagName.toLowerCase()).toBe("svg");
    expect(logo.querySelector("text")).toBeNull();
    expect(logo.textContent).toBe("");
  });

  it("keeps the mark's gradient and lets only the word Straft follow the text colour", () => {
    render(<BrandLogo />);
    const paths = [...screen.getByRole("img", { name: "Straft Studio" }).querySelectorAll("path")];
    expect(paths.map((p) => p.getAttribute("fill")?.slice(0, 4))).toEqual(["url(", "curr", "#FF9"]);
  });
});
