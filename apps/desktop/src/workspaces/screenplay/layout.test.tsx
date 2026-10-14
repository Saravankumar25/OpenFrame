// The writer's screenplay layout (open screenplay, open draft, panels, navigator columns) is
// remembered per project. A switch of screenplay or draft is saved at once; panel and column
// changes after a short pause; and a change still waiting when the screenplay closes is sent then.
// Before, closing the project (or leaving the screenplay) within half a second of a change lost it:
// the project reopened on the screenplay before the switch, and new navigator columns were gone.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";

const saved: unknown[] = [];
vi.mock("../../ipc/client", async (orig) => ({
  ...(await orig<typeof import("../../ipc/client")>()),
  call: vi.fn(async (op: string, args: { state?: unknown }) => {
    if (op === "screenplay.view_state") return null;
    if (op === "screenplay.set_view_state") {
      saved.push(args.state);
      return null;
    }
    throw new Error(`unexpected operation in test: ${op}`);
  }),
}));

const { useLayout } = await import("./util");

function setup() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, staleTime: Infinity } } });
  const wrapper = ({ children }: { children: ReactNode }) => <QueryClientProvider client={client}>{children}</QueryClientProvider>;
  return renderHook(() => useLayout(), { wrapper });
}

beforeEach(() => {
  saved.length = 0;
  vi.useFakeTimers({ shouldAdvanceTime: true });
});
afterEach(() => {
  vi.useRealTimers();
});

describe("useLayout", () => {
  it("saves a switch of screenplay or draft at once", async () => {
    const h = setup();
    await waitFor(() => expect(h.result.current[2]).toBe(true));
    act(() => h.result.current[1]((l) => ({ ...l, screenplayId: "sp-2", draftId: "d-2" })));
    expect(saved).toEqual([expect.objectContaining({ screenplayId: "sp-2", draftId: "d-2" })]);
  });

  it("saves panel and column changes after a pause, and once only", async () => {
    const h = setup();
    await waitFor(() => expect(h.result.current[2]).toBe(true));
    act(() => h.result.current[1]((l) => ({ ...l, navColumns: ["page"] })));
    act(() => h.result.current[1]((l) => ({ ...l, navColumns: ["page", "notes"] })));
    expect(saved).toEqual([]);
    await act(async () => vi.advanceTimersByTime(600));
    expect(saved).toEqual([expect.objectContaining({ navColumns: ["page", "notes"] })]);
    h.unmount();
    expect(saved).toHaveLength(1);
  });

  it("sends a change still waiting when the screenplay closes", async () => {
    const h = setup();
    await waitFor(() => expect(h.result.current[2]).toBe(true));
    act(() => h.result.current[1]((l) => ({ ...l, navColumns: ["page"] })));
    h.unmount();
    expect(saved).toEqual([expect.objectContaining({ navColumns: ["page"] })]);
  });
});
