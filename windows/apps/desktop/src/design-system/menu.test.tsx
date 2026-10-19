import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { ContextMenu } from "./index";

describe("ContextMenu", () => {
  it("builds lazy items only when the menu opens", async () => {
    const items = vi.fn(() => [{ label: "Open", onSelect: () => {} }]);
    render(
      <ContextMenu items={items}>
        <div>Scene 12</div>
      </ContextMenu>,
    );
    expect(items).not.toHaveBeenCalled();
    fireEvent.contextMenu(screen.getByText("Scene 12"));
    expect(await screen.findByText("Open")).toBeTruthy();
    expect(items).toHaveBeenCalled();
  });

  it("still accepts a plain item list", async () => {
    render(
      <ContextMenu items={[{ label: "Delete", onSelect: () => {} }]}>
        <div>Card</div>
      </ContextMenu>,
    );
    fireEvent.contextMenu(screen.getByText("Card"));
    expect(await screen.findByText("Delete")).toBeTruthy();
  });
});
