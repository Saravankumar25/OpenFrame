import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { AiStatusDto } from "../ipc/generated/AiStatusDto";
import type { AiSetupInfoDto } from "../ipc/generated/AiSetupInfoDto";
import OfflineAiSetup, { INSTALL_STEPS, primaryLabel, stepStates } from "./OfflineAiSetup";

// Test fixtures only: the IPC boundary is replaced by an in-memory responder.
const calls: { op: string; args: Record<string, unknown> }[] = [];
let info: AiSetupInfoDto;

vi.mock("../ipc/client", async () => {
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
    call: vi.fn(async (op: string, args: Record<string, unknown> = {}) => {
      calls.push({ op, args });
      if (op === "ai.setup_info") return info;
      if (op === "ai.install") return { taskId: "t1" };
      return null;
    }),
  };
});

const base: AiStatusDto = { installed: false, mode: "Off", runtimeState: "NotInstalled", runtimeMessage: null, install: null, localOnly: true, updateAvailable: false, installedBytes: 0 };

function renderSetup(status: AiStatusDto) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <OfflineAiSetup status={status} />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  calls.length = 0;
  info = {
    supported: true, message: null, downloadBytes: 861_908_195, totalBytes: 861_908_195, downloadedBytes: 0, requiredFreeBytes: 1_417_000_000,
    freeDiskBytes: 200 * 1024 ** 3, enoughDisk: true, runsOn: "Processor", likelySlow: false, warnings: [], upToDate: false,
  };
});

describe("Offline AI setup", () => {
  it("offers one button with the exact size first, and no technical names", async () => {
    renderSetup(base);
    expect(screen.getByText("Offline AI is not installed.")).toBeTruthy();
    expect(screen.getByText(/AI runs on this computer and project data is not sent to OpenFrame or an AI cloud service/)).toBeTruthy();
    expect(await screen.findByText("822 MB")).toBeTruthy();
    expect(screen.getByText("Processor")).toBeTruthy();
    // No model picker: exactly one primary action.
    expect(screen.queryByRole("radiogroup")).toBeNull();
    expect(screen.getAllByRole("button").map((b) => b.textContent)).toEqual(["Download Offline AI"]);
    expect(document.body.textContent).not.toMatch(/gemma|gguf|q4_k|q8_0|bge|embedding|llama|sqlite-vec|127\.0\.0\.1/i);
    fireEvent.click(screen.getByRole("button", { name: "Download Offline AI" }));
    await waitFor(() => expect(calls.some((c) => c.op === "ai.install")).toBe(true));
    expect(calls.find((c) => c.op === "ai.install")!.args).toEqual({});
  });

  it("refuses to start when the drive is too full", async () => {
    info = { ...info, enoughDisk: false, freeDiskBytes: 100 * 1024 ** 2 };
    renderSetup(base);
    expect(await screen.findByText(/Nothing will be downloaded until then/)).toBeTruthy();
    expect((screen.getByRole("button", { name: "Download Offline AI" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("resumes a paused download or discards it", async () => {
    info = { ...info, downloadedBytes: 400 * 1024 ** 2, downloadBytes: 461_908_195 };
    renderSetup({ ...base, install: { taskId: "t", phase: "paused", bytesDone: 400 * 1024 ** 2, bytesTotal: 861_908_195, message: "Paused.", error: null, active: false } });
    expect(await screen.findByText("Already downloaded")).toBeTruthy();
    expect(screen.getByText(/Resuming continues where it stopped/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Discard download" }));
    await waitFor(() => expect(calls.some((c) => c.op === "ai.cancel_install" && c.args.discard === true)).toBe(true));
    fireEvent.click(screen.getByRole("button", { name: "Resume download" }));
    await waitFor(() => expect(calls.some((c) => c.op === "ai.install")).toBe(true));
  });

  it("explains a failure and offers a clear retry", async () => {
    renderSetup({
      ...base,
      install: {
        taskId: "t", phase: "failed", bytesDone: 0, bytesTotal: 0, message: "x", active: false,
        error: { code: "ai.integrity", message: "The download didn't pass OpenFrame's safety check, so it wasn't used.", retryable: true },
      },
    });
    expect(await screen.findByText(/didn't pass OpenFrame's safety check/)).toBeTruthy();
    expect(screen.getByRole("button", { name: "Retry" })).toBeTruthy();
  });

  it("shows one combined progress with the setup steps, pause and cancel", async () => {
    renderSetup({ ...base, install: { taskId: "t", phase: "verifying", bytesDone: 431_000_000, bytesTotal: 861_908_195, message: "Verifying…", error: null, active: true } });
    expect(screen.getByText("Verifying…")).toBeTruthy();
    expect(screen.getByText(/412 MB of 822 MB · 50%/)).toBeTruthy();
    const current = screen.getByRole("listitem", { current: "step" });
    expect(current.textContent).toContain("Verifying");
    expect(screen.getAllByRole("listitem").map((li) => li.textContent?.replace(/^\d/, ""))).toEqual(
      expect.arrayContaining(["Checking device", "Downloading", "Installing", "Starting", "Ready"]),
    );
    fireEvent.click(screen.getByRole("button", { name: "Pause" }));
    await waitFor(() => expect(calls.some((c) => c.op === "ai.cancel_install" && c.args.discard === false)).toBe(true));
  });
});

describe("setup steps", () => {
  it("marks earlier steps done and the current one current", () => {
    expect(INSTALL_STEPS.map((s) => s.label)).toEqual(["Checking device", "Downloading", "Verifying", "Installing", "Starting", "Ready"]);
    expect(stepStates("installing")).toEqual(["done", "done", "done", "current", "todo", "todo"]);
    expect(stepStates("ready").every((s) => s === "done")).toBe(true);
    expect(stepStates("paused").every((s) => s === "todo")).toBe(true);
  });

  it("labels the one button for the situation", () => {
    expect(primaryLabel(base, undefined)).toBe("Download Offline AI");
    expect(primaryLabel({ ...base, installed: true, updateAvailable: true }, undefined)).toBe("Update Offline AI");
    expect(primaryLabel({ ...base, install: { taskId: "t", phase: "failed", bytesDone: 0, bytesTotal: 0, message: "", error: null, active: false } }, undefined)).toBe("Retry");
  });
});
