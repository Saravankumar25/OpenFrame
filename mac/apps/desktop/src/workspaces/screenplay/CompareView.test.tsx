// The compare bar (owner change list 2026-10-20, item 3): the draft pickers sit in a toolbar row of their own, the
// bar grows with its rows instead of keeping a fixed 46 px, and the pickers are not full width. Layout is measured in
// the real app by tests/e2e/specs/44-scene-hub-comments-compare.test.mjs; here the markup and the stylesheet rules
// that keep the bar from running over the changes list are checked.

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it, vi } from "vitest";
import { render, screen, within } from "@testing-library/react";
import type { ScreenplayDraftDto } from "../../ipc/generated/ScreenplayDraftDto";

vi.mock("./api", () => ({
  useCompare: () => ({
    isLoading: false,
    error: null,
    data: {
      draftA: { id: "d1", name: "First Draft" },
      draftB: { id: "d2", name: "Blue Revision" },
      summary: { added: 1, removed: 1, changed: 4, moved: 0, unchanged: 39, ambiguous: 0 },
      scenes: [],
    },
  }),
}));

const { CompareView } = await import("./CompareView");

const draft = (id: string, name: string, createdAt: number, isCurrent: boolean) => ({ id, name, createdAt, isCurrent }) as ScreenplayDraftDto;

describe("CompareView bar", () => {
  it("puts the draft pickers in one group and the view controls in another, above the content", () => {
    const { container } = render(
      <CompareView drafts={[draft("d1", "First Draft", 1, false), draft("d2", "Blue Revision", 2, true)]} initialA="d1" initialB="d2" onBack={() => undefined} />,
    );
    const bar = container.querySelector(".sp-top") as HTMLElement;
    const groups = bar.querySelectorAll(":scope > .spx-cmp-group");
    expect(groups).toHaveLength(2);
    const drafts = within(groups[0] as HTMLElement);
    expect(drafts.getByRole("button", { name: /Back to script/ })).toBeInTheDocument();
    expect(drafts.getByRole("combobox", { name: "Before" })).toBeInTheDocument();
    expect(drafts.getByRole("combobox", { name: "After" })).toBeInTheDocument();
    const tools = within(groups[1] as HTMLElement);
    expect(tools.getByRole("radiogroup", { name: "Comparison view" })).toBeInTheDocument();
    expect(tools.getByRole("button", { name: /Next change/ })).toBeInTheDocument();
    // The bar comes before the changes list and the summary in the page.
    const body = container.querySelector(".sp-body") as HTMLElement;
    expect(bar.compareDocumentPosition(body) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(screen.getByText(/Summary:/)).toBeInTheDocument();
  });

  it("the stylesheet lets the bar grow and keeps the pickers from filling a whole row", () => {
    const css = readFileSync(join(__dirname, "screenplay.css"), "utf8");
    const rule = (selector: string) => {
      const at = css.indexOf(`${selector} {`);
      expect(at, `no rule for ${selector}`).toBeGreaterThanOrEqual(0);
      return css.slice(at, css.indexOf("}", at));
    };
    const bar = rule(".spx-compare .sp-top");
    expect(bar).toContain("flex-wrap: wrap");
    expect(bar).toContain("height: auto");
    expect(bar).toContain("min-height: 46px");
    const picker = rule(".spx-cmp-group select.select");
    expect(picker).toContain("width: auto");
    expect(picker).toMatch(/max-width: \d+px/);
  });
});
