import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { act, render, screen, fireEvent, waitFor, within } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { VaultItemDto } from "../../ipc/generated/VaultItemDto";
import type { VaultOverview } from "../../ipc/generated/VaultOverview";
import IdeaVaultWorkspace from "./index";
import { useIntent, useUi } from "../../app/stores";

// Test fixtures only: the IPC boundary is replaced by an in-memory responder.
const calls: { op: string; args: Record<string, unknown> }[] = [];
let items: VaultItemDto[] = [];
let tauri = false;
let orphanCount = 0;
type DropEvent = { payload: { type: "enter" | "over" | "drop" | "leave"; paths: string[] } };
let dropHandler: ((ev: DropEvent) => void) | null = null;

// jsdom has no layout, so the virtual lists render every row.
vi.mock("@tanstack/react-virtual", () => ({
  useVirtualizer: (o: { count: number; estimateSize: (i: number) => number }) => ({
    getTotalSize: () => o.count * 100,
    getVirtualItems: () => Array.from({ length: o.count }, (_, index) => ({ key: index, index, size: 100, start: index * 100 })),
    measureElement: () => undefined,
  }),
}));

// The native drag-and-drop source (Windows Explorer) is replaced by a captured handler.
vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({
    onDragDropEvent: (h: (ev: DropEvent) => void) => {
      dropHandler = h;
      return Promise.resolve(() => {
        dropHandler = null;
      });
    },
  }),
}));

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
    inTauri: () => tauri,
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
        case "vault.orphan_files":
          return { count: orphanCount, bytes: orphanCount * 1000, samples: [] };
        case "vault.recover_files": {
          const added = orphanCount;
          orphanCount = 0;
          return { added, failed: 0 };
        }
        case "vault.get":
          return items.find((i) => i.id === args.id);
        case "vault.add_files": {
          const paths = args.paths as string[];
          const added = paths.filter((p) => !p.endsWith(".broken")).map((p, i) => mk({ id: `f${i}`, itemType: "image", displayName: p.split("\\").pop() ?? p }));
          items = [...added, ...items];
          return { added, failed: paths.filter((p) => p.endsWith(".broken")).map((p) => ({ path: p, name: p.split("\\").pop(), reason: "The file could not be read." })) };
        }
        case "vault.set_research_flag": {
          for (const id of args.ids as string[]) {
            const it = items.find((i) => i.id === id);
            if (it) it.researchFlag = (args.flag as VaultItemDto["researchFlag"]) ?? null;
          }
          return null;
        }
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
    researchFlag: null,
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
  tauri = false;
  orphanCount = 0;
  dropHandler = null;
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
    expect(await screen.findByText("Ideas across your whole filmmaking life. Copy them into a film when they are ready.")).toBeInTheDocument();
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

  it("shows research flags as words on cards and list rows (mock 045)", async () => {
    items = [
      mk({ id: "r1", displayName: "Police procedure research.pdf", itemType: "pdf", researchFlag: "needsExpertCheck" }),
      mk({ id: "r2", displayName: "Interview with a retired inspector", itemType: "audio", researchFlag: "checked" }),
      mk({ id: "r3", displayName: "Not research" }),
    ];
    await renderVault();
    const card = await screen.findByRole("button", { name: /^Police procedure research/ });
    expect(within(card).getByText("Needs Expert Check")).toBeInTheDocument();
    expect(within(screen.getByRole("button", { name: /^Interview with a retired inspector/ })).getByText("Checked")).toBeInTheDocument();
    expect(within(screen.getByRole("button", { name: /^Not research/ })).queryByTitle(/Research flag/)).toBeNull();
    fireEvent.click(screen.getByRole("radio", { name: "List" }));
    expect(await screen.findAllByText("Needs Expert Check")).not.toHaveLength(0);
  });

  it("sets a research flag from the item's context menu", async () => {
    items = [mk({ id: "r1", displayName: "FIR format, state police site", itemType: "url" })];
    await renderVault();
    const card = await screen.findByRole("button", { name: /^FIR format/ });
    fireEvent.contextMenu(card);
    expect(await screen.findByText("Research flag")).toBeInTheDocument();
    fireEvent.click(await screen.findByRole("menuitem", { name: "Needs Expert Check" }));
    await waitFor(() =>
      expect(calls.find((c) => c.op === "vault.set_research_flag")?.args).toEqual({ store: "project", ids: ["r1"], flag: "needsExpertCheck" }),
    );
  });

  it("drag-in: dropping files from Explorer adds them, and a partial failure shows what was not added", async () => {
    tauri = true;
    await renderVault();
    await waitFor(() => expect(dropHandler).not.toBeNull());
    act(() => dropHandler!({ payload: { type: "enter", paths: ["C:\\refs\\station.jpg", "C:\\refs\\clip.broken"] } }));
    expect(await screen.findByText("Drop to add 2 items to the Project Vault")).toBeInTheDocument();
    act(() => dropHandler!({ payload: { type: "drop", paths: ["C:\\refs\\station.jpg", "C:\\refs\\clip.broken"] } }));
    await waitFor(() => expect(calls.find((c) => c.op === "vault.add_files")?.args.paths).toEqual(["C:\\refs\\station.jpg", "C:\\refs\\clip.broken"]));
    expect(screen.queryByText("Drop to add 2 items to the Project Vault")).toBeNull();
    expect(await screen.findByText(/clip\.broken/)).toBeInTheDocument();
  });

  it("opens Quick Capture from the shell intent", async () => {
    useIntent.getState().fire("vault.new_note");
    await renderVault();
    expect(await screen.findByRole("heading", { name: "Quick Capture" })).toBeInTheDocument();
    expect(screen.getByText("Saving to BLACK RAIN · Project Idea Vault")).toBeInTheDocument();
    expect(useIntent.getState().intent).toBeNull();
  });

  it("offers to add back files that are in the vault folder but not in the vault", async () => {
    orphanCount = 3;
    await renderVault();
    expect(await screen.findByText("3 files in the vault folder are not in the vault.")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Add Them Back" }));
    await waitFor(() => expect(calls.find((c) => c.op === "vault.recover_files")?.args).toEqual({ store: "project" }));
    await waitFor(() => expect(screen.queryByText("3 files in the vault folder are not in the vault.")).not.toBeInTheDocument());
  });

  it("shows no recovery banner when every file is in the vault", async () => {
    await renderVault();
    await waitFor(() => expect(calls.some((c) => c.op === "vault.orphan_files")).toBe(true));
    expect(screen.queryByRole("button", { name: "Add Them Back" })).not.toBeInTheDocument();
  });
});
