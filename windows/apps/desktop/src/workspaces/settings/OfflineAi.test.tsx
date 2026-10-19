import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { GraphicsChoice } from "./OfflineAi";

// Test fixtures only: the IPC boundary is replaced by an in-memory responder.
const calls: { op: string; args: unknown }[] = [];
let graphicsOff = false;

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
    call: vi.fn(async (op: string, args: unknown) => {
      calls.push({ op, args });
      if (op === "ai.graphics.set") graphicsOff = (args as { off: boolean }).off;
      if (op === "ai.diagnostics")
        return {
          graphicsOff,
          placement: graphicsOff
            ? "Runs on the processor. The graphics card is switched off in Settings."
            : "Runs on the graphics card (Intel(R) Arc(TM) Graphics), about 30 words a second.",
        };
      return null;
    }),
  };
});

beforeEach(() => {
  calls.length = 0;
  graphicsOff = false;
});

describe("Graphics card setting", () => {
  it("shows where the AI runs and switches the card off and on", async () => {
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(
      <QueryClientProvider client={qc}>
        <GraphicsChoice />
      </QueryClientProvider>,
    );
    expect(await screen.findByText(/Runs on the graphics card \(Intel\(R\) Arc\(TM\) Graphics\)/)).toBeTruthy();
    expect(screen.getByRole("radio", { name: "Automatic" }).getAttribute("aria-checked")).toBe("true");
    await userEvent.click(screen.getByRole("radio", { name: "Off" }));
    await waitFor(() => expect(calls.some((c) => c.op === "ai.graphics.set" && (c.args as { off: boolean }).off)).toBe(true));
    expect(await screen.findByText(/switched off in Settings/)).toBeTruthy();
    expect(screen.getByRole("radio", { name: "Off" }).getAttribute("aria-checked")).toBe("true");
  });
});
