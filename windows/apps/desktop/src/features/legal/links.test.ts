import { describe, expect, it } from "vitest";
import { LEGAL_PAGES, legalUrl } from "./links";

describe("legalUrl", () => {
  it("gives a secure straftstudio.com address with a trailing slash for every legal page", () => {
    expect(LEGAL_PAGES.map((p) => p.id)).toEqual(["privacy", "terms", "refunds"]);
    for (const { id } of LEGAL_PAGES) {
      expect(legalUrl(id)).toMatch(/^https:\/\/straftstudio\.com\/legal\/[a-z]+\/$/);
    }
  });
});
