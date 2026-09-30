import { render, screen, fireEvent } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import ProductionWorkspace, { PRODUCTION_TABS } from "./index";
import { useNav } from "../../app/stores";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(() => new Promise(() => {})), convertFileSrc: (p: string) => p }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));

describe("production workspace tabs", () => {
  it("discovers tab modules by convention, in mock order", () => {
    const ids = PRODUCTION_TABS.map((t) => t.tab.id);
    expect(ids.slice(0, 4)).toEqual(["overview", "catalog", "locations", "cast-crew"]);
    const orders = PRODUCTION_TABS.map((t) => t.tab.order);
    expect([...orders].sort((a, b) => a - b)).toEqual(orders);
  });

  it("switches tabs through the route (sub = tab id)", () => {
    useNav.getState().go({ workspace: "production" });
    const qc = new QueryClient();
    render(
      <QueryClientProvider client={qc}>
        <ProductionWorkspace />
      </QueryClientProvider>,
    );
    expect(screen.getByRole("button", { name: "Overview" })).toHaveAttribute("aria-current", "page");
    fireEvent.click(screen.getByRole("button", { name: "Locations" }));
    expect(useNav.getState().route).toMatchObject({ workspace: "production", sub: "locations" });
    expect(screen.getByRole("heading", { name: "Locations" })).toBeInTheDocument();
  });
});
