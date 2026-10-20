// Settings > Offline AI > Technical details (owner change list 2026-10-20, item 4): the card takes a row of its own
// at the full width, the long hashes and addresses carry their full text in a tooltip, and the table scrolls inside
// its card instead of running under the AI health card. Real widths are measured in the app by
// tests/e2e/specs/98-layout-geometry.test.mjs (Settings, Offline AI at 1200 to 1920).

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it, vi } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

const HASH = "a671fe6f24b83a01e46b61b2845edc0a6c05591288d7110abb8e35bfdda2fccd";
const LICENCE_URL = "https://huggingface.co/BAAI/bge-small-en-v1.5";

// Test fixtures only: the IPC boundary is replaced by an in-memory responder.
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
    call: vi.fn(async (op: string) => {
      if (op !== "ai.diagnostics") return null;
      return {
        bundleId: "openframe-ai",
        bundleVersion: "1",
        bundleState: "Verified",
        bundleDir: "C:/Program Files/OpenFrame/ai",
        bundleBytes: 1_000_000,
        problem: null,
        components: [
          { id: "search", role: "Search model", name: "BGE small English v1.5 (Q8_0)", version: "f2068ed", licenseId: "MIT", licenseUrl: LICENCE_URL, bytes: 35_000_000, sha256: HASH },
        ],
        chatState: "Ready",
        searchState: "Ready",
        memoryBytes: 16_000_000_000,
        processor: "Test CPU",
        processorThreads: 8,
        warnings: [],
        logFile: "C:/logs/ai.log",
        placement: "Runs on the processor.",
        graphicsOff: false,
      };
    }),
  };
});

const { TechnicalDetails } = await import("./OfflineAi");

describe("Technical details card", () => {
  it("spans the full width, scrolls its table inside the card and shows long values in full on hover", async () => {
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(
      <QueryClientProvider client={qc}>
        <TechnicalDetails />
      </QueryClientProvider>,
    );
    const card = screen.getByRole("region", { name: "Technical details" });
    expect(card.className).toContain("ai-tech-card");
    await userEvent.click(within(card).getByText("Technical details"));
    const table = await within(card).findByRole("table", { name: "Offline AI components" });
    expect(table.parentElement?.className).toBe("ai-tech-scroll");
    expect(within(card).getByTitle(`SHA-256 ${HASH}`)).toBeInTheDocument();
    expect(within(card).getByTitle(LICENCE_URL)).toBeInTheDocument();
  });

  it("the stylesheet gives the card a row of its own and wraps long values inside their cells", () => {
    const css = readFileSync(join(__dirname, "..", "..", "ai", "ai.css"), "utf8");
    expect(css).toMatch(/\.ai-tech-card\{flex:1 1 100%;/);
    expect(css).toContain(".ai-tech-scroll{overflow-x:auto");
    expect(css).toContain(".ai-tech td{overflow-wrap:anywhere}");
    expect(css).toMatch(/\.ai-tech \.hash\{[^}]*word-break:break-all/);
  });
});
