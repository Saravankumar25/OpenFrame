// The AI Assistant panel docks beside the page (owner decision 2026-10-08): it is a column of the
// layout row after the page, never a layer inside it, so opening it narrows the page instead of
// covering controls. Its open state is a per-user layout preference that survives a restart, the
// shortcut (Ctrl+J by default) still opens and closes it, and keyboard focus reaches the page
// before the panel.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, fireEvent, screen, waitFor } from "@testing-library/react";
import { renderWithClient } from "../home/testUtils";
import { AI_PANEL_KEY, readAiPanelOpen, useUi } from "../stores";

vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => ({ setTitle: async () => undefined }) }));
vi.mock("../../ipc/client", async (orig) => {
  const real = await orig<typeof import("../../ipc/client")>();
  return {
    ...real,
    inTauri: () => false,
    call: vi.fn(async (op: string) => {
      if (op === "project.current") return { id: "p1", title: "Night Bus" };
      if (op === "history.info") return { canUndo: false, canRedo: false, undoLabel: null, redoLabel: null };
      if (op === "project.list_recent") return [];
      if (op === "app.info") return { version: "0.1.0", profile: { displayName: "Asha" } };
      return null;
    }),
    onAppEvent: vi.fn(() => Promise.resolve(() => undefined)),
  };
});
vi.mock("../useAppEvents", () => ({ useAppEvents: () => undefined }));
vi.mock("./RecoveryDialogs", () => ({
  OpenRecoveryDialog: () => null,
  RecoveryOffer: () => null,
  useRecoveryState: () => ({ data: null }),
}));
vi.mock("./SaveStatus", () => ({
  LeaveGuardDialog: () => null,
  LiveSaveErrorBanner: () => null,
  LiveSaveErrorDialog: () => null,
  guardedLeave: async () => undefined,
  retrySave: async () => undefined,
  useLiveSaveState: () => ({ status: "Saved", lastSavedAt: null, error: null, pendingOperations: 0 }),
}));
vi.mock("../../features/licensing/TrialCorner", () => ({ TrialCorner: () => null }));
vi.mock("../routes", () => {
  const Page = () => (
    <div className="content">
      <button type="button">Page action</button>
    </div>
  );
  const def = { id: "home", label: "Home", nav: true, component: Page };
  return { WORKSPACES: [def], workspaceDef: () => def };
});
vi.mock("../../ai/AiPanel.tsx", () => ({
  default: ({ onClose }: { onClose: () => void }) => (
    <aside className="ai" aria-label="AI Assistant">
      <button type="button" onClick={onClose}>
        Close AI Assistant
      </button>
    </aside>
  ),
}));

const { Shell } = await import("./Shell");

beforeEach(() => {
  window.localStorage.clear();
  useUi.setState({ aiOpen: false });
});
afterEach(() => {
  window.localStorage.clear();
});

const panel = () => screen.queryByRole("complementary", { name: "AI Assistant" });

describe("AI Assistant panel docked beside the page", () => {
  it("sits after the page in the layout row, not inside it, so it covers nothing", async () => {
    act(() => useUi.getState().setAiOpen(true));
    renderWithClient(<Shell />);
    const main = await screen.findByRole("main", { name: "Home" });
    const aside = panel()!;
    expect(aside).toBeTruthy();
    expect(main.contains(aside)).toBe(false);
    expect(aside.parentElement).toBe(main.parentElement);
    expect(aside.parentElement?.classList.contains("layout")).toBe(true);
    // Focus order follows the document: the page's controls come before the panel's.
    const pageAction = screen.getByRole("button", { name: "Page action" });
    expect(pageAction.compareDocumentPosition(aside) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it("opens and closes with the assistant shortcut and remembers the state", async () => {
    renderWithClient(<Shell />);
    await screen.findByRole("main", { name: "Home" });
    expect(panel()).toBeNull();
    fireEvent.keyDown(window, { key: "j", code: "KeyJ", ctrlKey: true });
    await waitFor(() => expect(panel()).toBeTruthy());
    expect(window.localStorage.getItem(AI_PANEL_KEY)).toBe("true");
    expect(readAiPanelOpen()).toBe(true);
    fireEvent.click(screen.getByRole("button", { name: "Close AI Assistant" }));
    await waitFor(() => expect(panel()).toBeNull());
    expect(readAiPanelOpen()).toBe(false);
  });

  it("the top bar button shows the panel state", async () => {
    renderWithClient(<Shell />);
    const button = await screen.findByRole("button", { name: "Assistant" });
    expect(button.getAttribute("aria-pressed")).toBe("false");
    fireEvent.click(button);
    await waitFor(() => expect(panel()).toBeTruthy());
    expect(button.getAttribute("aria-pressed")).toBe("true");
  });
});

describe("saved AI panel state", () => {
  it("is closed when nothing was saved or the value is unexpected", () => {
    expect(readAiPanelOpen()).toBe(false);
    window.localStorage.setItem(AI_PANEL_KEY, "{broken");
    expect(readAiPanelOpen()).toBe(false);
  });

  it("is closed when storage cannot be read", () => {
    const spy = vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("storage blocked");
    });
    try {
      expect(readAiPanelOpen()).toBe(false);
    } finally {
      spy.mockRestore();
    }
  });

  it("a failed save still opens the panel for this session", () => {
    const spy = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("storage full");
    });
    try {
      act(() => useUi.getState().setAiOpen(true));
      expect(useUi.getState().aiOpen).toBe(true);
    } finally {
      spy.mockRestore();
    }
  });
});
