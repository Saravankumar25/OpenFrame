import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { VaultItemDto } from "../../ipc/generated/VaultItemDto";
import type { VaultOverview } from "../../ipc/generated/VaultOverview";
import IdeaVaultWorkspace from "./index";
import { useIntent, useUi } from "../../app/stores";

// Test fixtures only: the IPC boundary is replaced by an in-memory responder.
const calls: { op: string; args: Record<string, unknown> }[] = [];
let items: VaultItemDto[] = [];

function overview(): VaultOverview {
  return { store: "project", total: items.length, pinned: items.filter((i) => i.pinned).length, unfiled: items.length, folders: [], collections: [], tags: [] };
}

vi.mock("../../ipc/client", async () => {
  class OpError extends Error {
    code = "x";
    is() {
      return false;
    }
  }
  return {
    OpError,
    inTauri: () => false,
    onAppEvent: () => Promise.resolve(() => undefined),
    openAsset: vi.fn(),
    openUrl: vi.fn(),
    revealLocation: vi.fn(),
    call: vi.fn(async (op: string, args: Record<string, unknown> = {}) => {
      calls.push({ op, args });
      switch (op) {
        case "project.current":
          return { id: "p", title: "BLACK RAIN", projectType: "Feature Film" };
        case "vault.overview":
          return overview();
        case "vault.list":
          return items;
        case "vault.get":
          return items.find((i) => i.id === args.id);
        case "vault.create": {
          const it = mk({ id: `n${items.length + 1}`, itemType: args.itemType as VaultItemDto["itemType"] });
          items = [it, ...items];
          return it;
        }
        default:
          return null;
      }
    }),
  };
});

function mk(p: Partial<VaultItemDto> & { id: string }): VaultItemDto {
  return {
    store: "project",
    itemType: "note",
    title: null,
    displayName: "Untitled note",
    body: null,
    caption: null,
    url: null,
    sourceText: null,
    asset: null,
    folderId: null,
    collectionIds: [],
    tags: [],
    pinned: false,
    sourceGlobalItemId: null,
    createdAt: Date.now(),
    updatedAt: Date.now(),
    rev: 1,
    matchReason: null,
    ...p,
  };
}

beforeAll(() => {
  // jsdom has no layout engine.
  globalThis.ResizeObserver ??= class {
    observe() {}
    unobserve() {}
    disconnect() {}
  } as unknown as typeof ResizeObserver;
});

beforeEach(() => {
  calls.length = 0;
  items = [];
  localStorage.clear();
});

async function renderVault() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <IdeaVaultWorkspace />
    </QueryClientProvider>,
  );
}

// Rendering the full workspace in jsdom is slow on a busy machine.
describe("Idea Vault workspace", { timeout: 30_000 }, () => {
  it("shows the project header, toolbar and the empty state", async () => {
    await renderVault();
    expect(await screen.findByText("Start collecting anything about this film.")).toBeInTheDocument();
    expect(screen.getByText("Everything collected for this film. Nothing here is classified or synced to the screenplay.")).toBeInTheDocument();
    expect(screen.getByText("BLACK RAIN")).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "This Project" })).toHaveAttribute("aria-checked", "true");
    expect(screen.getByPlaceholderText("Search ideas, tags, filenames…")).toBeInTheDocument();
    for (const v of ["Grid", "Card", "List", "Folder"]) expect(screen.getByRole("radio", { name: v })).toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: /Record Note/ }).length).toBeGreaterThan(0);
  });

  it("switches to the Global vault with its own copy and undo scope", async () => {
    await renderVault();
    fireEvent.click(await screen.findByRole("radio", { name: "Global" }));
    expect(await screen.findByText("Ideas across your whole filmmaking life — copy them into a film when they are ready.")).toBeInTheDocument();
    expect(screen.getByText("Drop any film idea, reference or file here.")).toBeInTheDocument();
    await waitFor(() => expect(useUi.getState().undoScope).toBe("global"));
    await waitFor(() => expect(calls.some((c) => c.op === "vault.list" && c.args.store === "global")).toBe(true));
  });

  it("lists items in List view with the mock's columns", async () => {
    items = [mk({ id: "a", displayName: "Rain against a train window", itemType: "image", tags: ["rain"] }), mk({ id: "b", displayName: "Final scene before sunrise", body: "Final scene before sunrise" })];
    await renderVault();
    fireEvent.click(await screen.findByRole("radio", { name: "List" }));
    for (const h of ["Name", "Type", "Collection", "Tags", "Added", "Modified"]) expect(await screen.findByRole("columnheader", { name: h })).toBeInTheDocument();
    expect(screen.getByText("2")).toBeInTheDocument(); // All items count
  });

  it("opens Quick Capture from the shell intent", async () => {
    useIntent.getState().fire("vault.new_note");
    await renderVault();
    expect(await screen.findByRole("heading", { name: "Quick Capture" })).toBeInTheDocument();
    expect(screen.getByText("Saving to BLACK RAIN · Project Idea Vault")).toBeInTheDocument();
    expect(useIntent.getState().intent).toBeNull();
  });
});
