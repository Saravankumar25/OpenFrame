// Scene navigator columns the writer can show or hide (SCRIPT-017).

import { beforeAll, describe, expect, it, vi } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SceneNavigator, availableColumns, type NavColumnsData } from "./SceneNavigator";
import { DEFAULT_LAYOUT, normalizeColumns, normalizeLayout, toggleColumn } from "./util";

beforeAll(() => {
  // jsdom has no layout; the navigator scrolls the current scene into view.
  Element.prototype.scrollIntoView = vi.fn();
});

const scenes = [
  { id: "s1", number: 1, label: "1", heading: "INT. DEPOT - NIGHT" },
  { id: "s2", number: 2, label: "2", heading: "EXT. ROAD - NIGHT" },
];

function data(extra: Partial<NavColumnsData> = {}): NavColumnsData {
  return {
    rows: new Map([
      ["s1", { sceneId: "s1", page: 1, notes: 0, revision: null, productionNumber: null }],
      ["s2", { sceneId: "s2", page: 3, notes: 2, revision: { kind: "Changed", label: "Blue Revision", color: "Blue" }, productionNumber: "12A" }],
    ]),
    hasRevisions: true,
    hasProductionNumbers: true,
    ...extra,
  };
}

function nav(columns: Parameters<typeof SceneNavigator>[0]["columns"], columnData: NavColumnsData | null, onToggleColumn = vi.fn()) {
  render(
    <SceneNavigator
      scenes={scenes}
      currentSceneId="s1"
      canEdit
      onJump={vi.fn()}
      onNew={vi.fn()}
      onMove={vi.fn()}
      menuFor={() => []}
      columns={columns}
      columnData={columnData}
      onToggleColumn={onToggleColumn}
    />,
  );
  return onToggleColumn;
}

describe("layout memory", () => {
  it("keeps known columns once, in display order, and survives old saved layouts", () => {
    expect(normalizeColumns(["production", "page", "page", "bogus"])).toEqual(["page", "production"]);
    expect(normalizeColumns("page")).toEqual([]);
    expect(normalizeLayout({ mode: "standard" }).navColumns).toEqual([]);
    expect(normalizeLayout({ navColumns: ["notes"] }).navColumns).toEqual(["notes"]);
    const l = toggleColumn(toggleColumn(DEFAULT_LAYOUT, "revision"), "page");
    expect(l.navColumns).toEqual(["page", "revision"]);
    expect(toggleColumn(l, "page").navColumns).toEqual(["revision"]);
  });
  it("offers revision and production only when the draft has them", () => {
    expect([...availableColumns(null)]).toEqual(["page", "notes"]);
    expect(availableColumns(data()).has("production")).toBe(true);
    expect(availableColumns(data({ hasProductionNumbers: false, hasRevisions: false })).has("production")).toBe(false);
  });
});

describe("navigator", () => {
  it("shows only number and heading by default", () => {
    nav([], null);
    const row = screen.getByRole("button", { name: /EXT\. ROAD/ });
    expect(row).toHaveTextContent("2EXT. ROAD - NIGHT");
    expect(screen.queryByText("Page")).toBeNull();
  });

  it("shows the chosen columns with words, not colour alone", () => {
    nav(["page", "notes", "revision", "production"], data());
    const row = screen.getByRole("button", { name: /EXT\. ROAD/ });
    expect(within(row).getByLabelText("Page 3")).toHaveTextContent("3");
    expect(within(row).getByLabelText("2 notes")).toHaveTextContent("2");
    // Empty cells are not announced on every row.
    expect(within(row).getByLabelText("Revision: Changed")).toHaveTextContent("Changed");
    expect(within(row).getByLabelText("Production number 12A")).toHaveTextContent("12A");
    const first = screen.getByRole("button", { name: /INT\. DEPOT/ });
    expect(within(first).queryByLabelText(/Revision/)).toBeNull();
    expect(within(first).queryByLabelText(/notes/)).toBeNull();
  });

  it("hides a chosen column gracefully when this draft has no data for it", () => {
    nav(["page", "production"], data({ hasProductionNumbers: false }));
    const row = screen.getByRole("button", { name: /EXT\. ROAD/ });
    expect(within(row).getByLabelText("Page 3")).toBeInTheDocument();
    expect(within(row).queryByLabelText(/Production number/)).toBeNull();
  });

  it("the Columns menu toggles columns from the keyboard", async () => {
    const user = userEvent.setup();
    const onToggle = nav(["page"], data({ hasProductionNumbers: false }));
    screen.getByRole("button", { name: "Navigator columns" }).focus();
    await user.keyboard("{Enter}");
    const notes = await screen.findByRole("menuitem", { name: "Notes" });
    expect(screen.getByRole("menuitem", { name: "✓ Page" })).toBeInTheDocument();
    expect(screen.getByRole("menuitem", { name: "Production number (none yet)" })).toHaveAttribute("aria-disabled", "true");
    await user.click(notes);
    expect(onToggle).toHaveBeenCalledWith("notes");
  });
});
