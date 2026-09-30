// Security review 2026-09-30 (WEB-03): a revision colour comes from project data and is
// used as a CSS value; only named swatches or plain hex colours may pass.
import { describe, expect, it } from "vitest";
import { colourSwatch } from "./lock";

describe("colourSwatch", () => {
  it("maps named revision colours and plain hex values", () => {
    expect(colourSwatch("Blue")).toBe("#8fb3ef");
    expect(colourSwatch("#abc")).toBe("#abc");
    expect(colourSwatch("#A1B2C3D4")).toBe("#A1B2C3D4");
    expect(colourSwatch(null)).toBeUndefined();
  });

  it("rejects anything that could smuggle CSS", () => {
    for (const bad of ["#000 url(https://x.example/a.png)", "#fff;position:fixed", "#12345g", "#", "red", "#0000000000"]) {
      expect(colourSwatch(bad)).toBeUndefined();
    }
  });
});
