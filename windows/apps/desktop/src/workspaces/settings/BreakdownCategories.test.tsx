// Settings > Advanced: breakdown category colours and icons (depth spec section 23).

import { render, screen, fireEvent, waitFor, within } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { BreakdownCategory } from "../../ipc/generated/BreakdownCategory";
import type { BreakdownCategoryStyles } from "../../ipc/generated/BreakdownCategoryStyles";
import { BreakdownCategorySettings } from "./BreakdownCategories";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...a: unknown[]) => invoke(...a), convertFileSrc: (p: string) => p }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));

const CATS: BreakdownCategory[] = [
  "Cast", "Extras / Background", "Location / Set", "Props", "Wardrobe", "Vehicles",
  "Hair / Makeup", "Special Effects", "VFX", "Sound", "Animals",
];

function styles(customized: BreakdownCategory[] = []): BreakdownCategoryStyles {
  return {
    categories: CATS.map((c) => ({
      category: c,
      label: c,
      tooltip: `${c}: what this category covers.`,
      color: customized.includes(c) ? "blue" : "red",
      colorHex: customized.includes(c) ? "#1864ab" : "#c92a2a",
      icon: "tag",
      customized: customized.includes(c),
    })),
    palette: [
      { token: "red", label: "Red", hex: "#c92a2a" },
      { token: "blue", label: "Blue", hex: "#1864ab" },
    ],
    icons: ["tag", "box"],
  };
}

function backend(s: BreakdownCategoryStyles, role = "Owner") {
  invoke.mockImplementation(async (_cmd: string, a?: { op: string }) => {
    switch (a?.op) {
      case "breakdown.categories":
        return s;
      case "project.members":
        return [{ userId: "u", displayName: "Me", role, isYou: true }];
      default:
        return s;
    }
  });
}

function wrap() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <BreakdownCategorySettings />
    </QueryClientProvider>,
  );
}

describe("breakdown category settings", () => {
  beforeEach(() => invoke.mockReset());

  it("shows all eleven categories with their name, description, colour choices and icon choices", async () => {
    backend(styles());
    wrap();
    for (const c of CATS) {
      const section = await screen.findByRole("region", { name: `${c} look` });
      expect(within(section).getByText(`${c}: what this category covers.`)).toBeInTheDocument();
      expect(within(section).getByRole("group", { name: `${c} colour` })).toBeInTheDocument();
      expect(within(section).getByRole("group", { name: `${c} icon` })).toBeInTheDocument();
    }
  });

  it("choosing a colour stores the palette token for that category only", async () => {
    backend(styles());
    wrap();
    const section = await screen.findByRole("region", { name: "Props look" });
    const blue = within(within(section).getByRole("group", { name: "Props colour" })).getByRole("button", { name: "Blue" });
    expect(blue).toHaveAttribute("aria-pressed", "false");
    fireEvent.click(blue);
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("of_invoke", {
        op: "breakdown.set_category_style",
        args: { category: "Props", color: "blue", icon: null },
      }),
    );
  });

  it("choosing an icon stores the icon key", async () => {
    backend(styles());
    wrap();
    const section = await screen.findByRole("region", { name: "Sound look" });
    fireEvent.click(within(within(section).getByRole("group", { name: "Sound icon" })).getByRole("button", { name: "Icon box" }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("of_invoke", {
        op: "breakdown.set_category_style",
        args: { category: "Sound", color: null, icon: "box" },
      }),
    );
  });

  it("Restore Defaults is offered only when something changed", async () => {
    backend(styles());
    const first = wrap();
    await screen.findByRole("region", { name: "Cast look" });
    expect(await screen.findByRole("button", { name: "Restore Defaults" })).toBeDisabled();
    first.unmount();

    backend(styles(["Vehicles"]));
    wrap();
    await screen.findByText("Changed");
    const restore = screen.getByRole("button", { name: "Restore Defaults" });
    await waitFor(() => expect(restore).toBeEnabled());
    fireEvent.click(restore);
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("of_invoke", { op: "breakdown.reset_category_styles", args: {} }));
  });

  it("read-only roles see the looks but cannot change them", async () => {
    backend(styles(), "Viewer");
    wrap();
    const section = await screen.findByRole("region", { name: "Cast look" });
    await waitFor(() => expect(screen.queryByRole("button", { name: "Restore Defaults" })).toBeNull());
    for (const b of within(section).getAllByRole("button")) expect(b).toBeDisabled();
  });
});
