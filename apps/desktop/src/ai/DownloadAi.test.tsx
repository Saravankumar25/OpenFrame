import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { AiDownloadDto } from "../ipc/generated/AiDownloadDto";
import DownloadAi, { downloadLine, tooLittleSpace } from "./DownloadAi";

// Test fixtures only: the IPC boundary is replaced by an in-memory responder.
const calls: string[] = [];
let dl: AiDownloadDto;

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
    call: vi.fn(async (op: string) => {
      calls.push(op);
      return op === "ai.download.status" ? dl : null;
    }),
  };
});

const GB = 1024 ** 3;
const base = (p: Partial<AiDownloadDto> = {}): AiDownloadDto => ({
  canDownload: true,
  phase: "Idle",
  sizeBytes: 1.1 * GB,
  neededBytes: 1.6 * GB,
  freeBytes: 40 * GB,
  doneBytes: 0,
  totalBytes: 0,
  problem: null,
  ...p,
});

function show(damaged?: string) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <DownloadAi damaged={damaged} />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  calls.length = 0;
  dl = base();
});

describe("Download OpenFrame AI", () => {
  it("shows the size, the space needed and the internet use before any click, and starts only on the click", async () => {
    show();
    expect(await screen.findByText(/A one-time download of 1.1 GB\. It needs 1.6 GB of free space \(40 GB free on this drive\)\./)).toBeTruthy();
    expect(screen.getByText(/uses the internet once/)).toBeTruthy();
    expect(calls).not.toContain("ai.download.start");
    await userEvent.click(screen.getByRole("button", { name: "Download OpenFrame AI" }));
    await waitFor(() => expect(calls).toContain("ai.download.start"));
  });

  it("shows progress with Pause while it downloads", async () => {
    dl = base({ phase: "Downloading", doneBytes: 0.55 * GB, totalBytes: 1.1 * GB });
    show();
    const bar = await screen.findByRole("progressbar", { name: "OpenFrame AI download" });
    expect(bar.getAttribute("aria-valuenow")).toBe("50");
    expect(screen.getByText("Downloading: 564 MB of 1.1 GB")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Pause" }));
    await waitFor(() => expect(calls).toContain("ai.download.pause"));
  });

  it("offers Resume after a pause and Try again with the plain reason after a failure", async () => {
    dl = base({ phase: "Paused", doneBytes: 0.2 * GB, totalBytes: 1.1 * GB });
    const { unmount } = show();
    expect(await screen.findByRole("button", { name: "Resume download" })).toBeTruthy();
    unmount();
    dl = base({ phase: "Failed", problem: { code: "ai.file_damaged", message: "A downloaded OpenFrame AI file was damaged and has been set aside.", retryable: false } });
    show();
    expect(await screen.findByText("A downloaded OpenFrame AI file was damaged and has been set aside.")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Try again" })).toBeTruthy();
  });

  it("refuses to start without room, and says so", async () => {
    dl = base({ freeBytes: 1 * GB });
    show();
    expect(await screen.findByText(/not enough free space/)).toBeTruthy();
    expect((screen.getByRole("button", { name: "Download OpenFrame AI" }) as HTMLButtonElement).disabled).toBe(true);
    expect(tooLittleSpace(base({ phase: "Paused", freeBytes: 1 * GB, doneBytes: 0.7 * GB }))).toBe(false);
  });

  it("removes a damaged set before downloading it again", async () => {
    show("An OpenFrame AI file is missing or damaged.");
    expect(await screen.findByText("An OpenFrame AI file is missing or damaged.")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Download again" }));
    await waitFor(() => expect(calls.filter((c) => c !== "ai.download.status")).toEqual(["ai.download.remove", "ai.download.start"]));
  });

  it("says plainly when this copy cannot download", async () => {
    dl = base({ canDownload: false });
    show();
    expect(await screen.findByText(/cannot download OpenFrame AI/)).toBeTruthy();
    expect(screen.queryByRole("button")).toBeNull();
    expect(downloadLine(base({ phase: "Checking" }))).toBe("Getting the download list…");
  });
});
