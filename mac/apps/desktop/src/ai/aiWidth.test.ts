// The docked AI panel width stays within bounds on any window (UX brief 4.2).
import { describe, expect, it } from "vitest";
import { clampAiWidth } from "./AiPanel";

describe("AI panel width", () => {
  it("never goes below 300 or above 45 percent of the window (640 at most)", () => {
    expect(clampAiWidth(100, 1920)).toBe(300);
    expect(clampAiWidth(900, 1920)).toBe(640);
    expect(clampAiWidth(600, 1280)).toBe(576);
    expect(clampAiWidth(500, 1024)).toBe(461);
    expect(clampAiWidth(Number.NaN, 1920)).toBe(360);
  });
});
