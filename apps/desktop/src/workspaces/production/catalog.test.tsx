// Catalog tab (FSD §28, §98; UX §3.23): derived "Used in", statuses, archive
// instead of destructive delete, duplicate warning when adding. Lives outside
// ./tabs so the tab glob never loads it.

import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { CatalogItemDto } from "../../ipc/generated/CatalogItemDto";
import CatalogTab from "./tabs/catalog";
import { useNav } from "../../app/stores";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...a: unknown[]) => invoke(...a), convertFileSrc: (p: string) => p }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));

function item(over: Partial<CatalogItemDto> = {}): CatalogItemDto {
  return {
    id: "c1",
    category: "Props",
    name: "Red Folder",
    description: null,
    notes: null,
    contact: null,
    status: "Searching",
    image: null,
    archived: false,
    characterId: null,
    locationId: null,
    aliases: [],
    usedIn: [
      { sceneId: "s1", number: "1", heading: "INT. POLICE STATION — NIGHT", inSource: true },
      { sceneId: "s2", number: "2", heading: "EXT. OLD RAILWAY STATION — NIGHT", inSource: true },
    ],
    usedInLabel: "Scenes 1, 2",
    rev: 1,
    updatedAt: 0,
    ...over,
  };
}

function backend(items: CatalogItemDto[], matches: unknown[] = []) {
  invoke.mockImplementation(async (_cmd: string, { op }: { op: string }) => {
    switch (op) {
      case "catalog.list":
        return items;
      case "catalog.find_matches":
        return matches;
      case "project.members":
        return [];
      default:
        return null;
    }
  });
}

function wrap() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <CatalogTab />
    </QueryClientProvider>,
  );
}

describe("catalog tab", () => {
  beforeEach(() => {
    invoke.mockReset();
    useNav.getState().go({ workspace: "production", sub: "catalog" });
  });

  it("lists items with category, status and derived scene usage", async () => {
    backend([item(), item({ id: "c2", name: "Jeep", category: "Vehicles", status: "Confirmed", usedIn: [], usedInLabel: "Not used yet" })]);
    wrap();
    expect(await screen.findByText("Red Folder")).toBeInTheDocument();
    expect(screen.getByText("Scenes 1, 2")).toBeInTheDocument();
    expect(screen.getByText("Prop")).toBeInTheDocument();
    expect(screen.getByText("Vehicle")).toBeInTheDocument();
    expect(screen.getByText("Searching")).toBeInTheDocument();
    expect(screen.getByText("Not used yet")).toBeInTheDocument();
  });

  it("filters by category through the backend query", async () => {
    backend([item()]);
    wrap();
    await screen.findByText("Red Folder");
    fireEvent.click(screen.getByRole("radio", { name: "Wardrobe" }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("of_invoke", { op: "catalog.list", args: { category: "Wardrobe", search: null, includeArchived: false } }),
    );
  });

  it("opens the item drawer with its scenes and offers Archive, not only Delete", async () => {
    backend([item()]);
    wrap();
    fireEvent.click(await screen.findByText("Red Folder"));
    expect(await screen.findByText("Catalog item")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Archive" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Open Scenes" })).toBeEnabled();
  });

  it("warns about an existing item with the same name when adding", async () => {
    backend([item()], [{ id: "c1", name: "Red Folder", category: "Props", status: "Searching", exact: true }]);
    wrap();
    fireEvent.click((await screen.findAllByRole("button", { name: "Add Item" }))[0]);
    fireEvent.change(screen.getByLabelText(/^Name/), { target: { value: "red folder" } });
    expect(await screen.findByText(/The catalog already has “Red Folder”/)).toBeInTheDocument();
  });
});
