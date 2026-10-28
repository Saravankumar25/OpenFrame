// The page appearance (light or dark paper) is a view preference kept with the writer's layout, per
// project and user, in the same place as the other layout choices.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { DEFAULT_LAYOUT, normalizeLayout, useLayout } from "./util";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...a: unknown[]) => invoke(...a), convertFileSrc: (p: string) => p }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));

let saved: unknown = null;
const wrapper = ({ children }: { children: ReactNode }) => (
  <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{children}</QueryClientProvider>
);
const sent = () => invoke.mock.calls.filter((c) => (c[1] as { op?: string })?.op === "screenplay.set_view_state").map((c) => (c[1] as { args: { state: Record<string, unknown> } }).args.state);

beforeEach(() => {
  invoke.mockReset();
  invoke.mockImplementation(async (_cmd: string, a?: { op?: string }) => (a?.op === "screenplay.view_state" ? saved : null));
});
afterEach(() => vi.useRealTimers());

describe("dark page preference", () => {
  it("is light paper by default and for layouts saved before the option existed", () => {
    expect(DEFAULT_LAYOUT.darkPaper).toBe(false);
    expect(normalizeLayout({ mode: "standard", panels: [] }).darkPaper).toBe(false);
    expect(normalizeLayout({ darkPaper: "yes" }).darkPaper).toBe(false);
    expect(normalizeLayout({ darkPaper: true }).darkPaper).toBe(true);
  });

  it("comes back as the writer left it", async () => {
    saved = { mode: "standard", panels: [], darkPaper: true };
    const { result } = renderHook(() => useLayout(), { wrapper });
    await waitFor(() => expect(result.current[2]).toBe(true));
    expect(result.current[0].darkPaper).toBe(true);
  });

  it("is saved with the layout when toggled, and toggles back", async () => {
    saved = null;
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const { result } = renderHook(() => useLayout(), { wrapper });
    await waitFor(() => expect(result.current[2]).toBe(true));
    act(() => result.current[1]((l) => ({ ...l, darkPaper: !l.darkPaper })));
    expect(result.current[0].darkPaper).toBe(true);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(600);
    });
    expect(sent().at(-1)).toMatchObject({ darkPaper: true });
    act(() => result.current[1]((l) => ({ ...l, darkPaper: !l.darkPaper })));
    expect(result.current[0].darkPaper).toBe(false);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(600);
    });
    expect(sent().at(-1)).toMatchObject({ darkPaper: false });
  });
});
