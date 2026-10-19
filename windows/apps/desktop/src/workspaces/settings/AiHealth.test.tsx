import { beforeEach, describe, expect, it, vi } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithClient } from "../../app/home/testUtils";
import { AiHealthCard } from "./AiHealth";

// Test fixtures only: the IPC boundary and the system Save dialog are replaced.
const calls: { op: string; args: Record<string, unknown> }[] = [];
let health: Record<string, unknown> = {};
const saveMock = vi.fn(async (_o: unknown) => "C:\\Reports\\openframe-ai-report.json" as string | null);

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(async () => null), save: (o: unknown) => saveMock(o) }));

vi.mock("../../ipc/client", async () => {
  class OpError extends Error {
    code = "x";
  }
  return {
    OpError,
    inTauri: () => false,
    onAppEvent: () => Promise.resolve(() => undefined),
    call: vi.fn(async (op: string, args: Record<string, unknown> = {}) => {
      calls.push({ op, args });
      if (op === "ai.health") return health;
      if (op === "ai.export_report") return { fileName: "openframe-ai-report.json", rows: 12 };
      return null;
    }),
  };
});

const empty = { requests: 0, medianTotalMs: null, medianTokensIn: null, medianContextFillPct: null, proposals: 0, applied: 0, rejected: 0, undone: 0, applyRatePct: null, averagePasses: null, errors: 0, weakestTool: null };

describe("AI health card", () => {
  beforeEach(() => {
    calls.length = 0;
    saveMock.mockClear();
  });

  it("shows the figures from the local trace, including the weakest tool", async () => {
    health = { ...empty, requests: 12, medianTotalMs: 21500, proposals: 5, applied: 3, rejected: 1, undone: 0, applyRatePct: 75, averagePasses: 1, errors: 1, weakestTool: { name: "propose_task", uses: 4, failurePct: 50 } };
    renderWithClient(<AiHealthCard />);
    expect(await screen.findByText("21.5 s")).toBeInTheDocument();
    expect(screen.getByText("75% (3 of 4 decided)")).toBeInTheDocument();
    expect(screen.getByText("propose_task: 50% of 4 uses went wrong")).toBeInTheDocument();
    expect(screen.getByText(/no questions, answers or project text/)).toBeInTheDocument();
  });

  it("says plainly when there is nothing yet", async () => {
    health = empty;
    renderWithClient(<AiHealthCard />);
    expect(await screen.findByText(/No requests yet/)).toBeInTheDocument();
  });

  it("Export AI report asks where to save and writes only after the person chooses", async () => {
    health = empty;
    renderWithClient(<AiHealthCard />);
    await userEvent.click(await screen.findByRole("button", { name: "Export AI report" }));
    await waitFor(() => expect(calls.some((c) => c.op === "ai.export_report")).toBe(true));
    expect(calls.find((c) => c.op === "ai.export_report")?.args).toEqual({ path: "C:\\Reports\\openframe-ai-report.json" });

    calls.length = 0;
    saveMock.mockResolvedValueOnce(null);
    await userEvent.click(screen.getByRole("button", { name: "Export AI report" }));
    await waitFor(() => expect(saveMock).toHaveBeenCalledTimes(2));
    expect(calls.some((c) => c.op === "ai.export_report")).toBe(false);
  });
});
