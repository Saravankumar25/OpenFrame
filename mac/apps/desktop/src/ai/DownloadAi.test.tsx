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
    constructor(_op: string, e: { message: string }) {
      super(e.message);
    }
    code = "x";
    is() {
      return false;
    }
  }
  return {
    OpError,
    inTauri: () => false,
    onAppEvent: () => Promise.resolve(() => undefined),
    call: vi.fn(async (op: string, args?: { path?: string }) => {
      calls.push(args?.path ? `${op} ${args.path}` : op);
      if (op === "ai.download.import") {
        if (imported instanceof Error) throw imported;
        return imported;
      }
      return op === "ai.download.status" ? dl : null;
    }),
  };
});

// The Open dialog: the file the person picks, or null when they cancel. What the import answers.
let picked: string | null = null;
let imported: { complete: boolean } | Error = { complete: true };
const toasts: string[] = [];
vi.mock("../app/toast", () => ({
  toast: { success: (m: string) => toasts.push(`ok ${m}`), error: (m: string) => toasts.push(`err ${m}`) },
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(async () => picked) }));

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
  picked = null;
  imported = { complete: true };
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

  it("removes a damaged set before downloading it again, but resumes a paused download of it", async () => {
    const { unmount } = show("An OpenFrame AI file is missing or damaged.");
    expect(await screen.findByText("An OpenFrame AI file is missing or damaged.")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Download again" }));
    await waitFor(() => expect(calls.filter((c) => c !== "ai.download.status")).toEqual(["ai.download.remove", "ai.download.start"]));
    unmount();
    calls.length = 0;
    dl = base({ phase: "Paused", doneBytes: 0.3 * GB, totalBytes: 1.1 * GB });
    show("An OpenFrame AI file is missing or damaged.");
    await userEvent.click(await screen.findByRole("button", { name: "Resume download" }));
    await waitFor(() => expect(calls).toContain("ai.download.start"));
    expect(calls).not.toContain("ai.download.remove");
  });

  it("announces the phase, not every byte, and says so plainly when the status cannot be read", async () => {
    dl = base({ phase: "Downloading", doneBytes: 0.55 * GB, totalBytes: 1.1 * GB });
    show();
    expect((await screen.findByRole("status")).textContent).toBe("Downloading OpenFrame AI.");
    expect(tooLittleSpace(base({ freeBytes: 1 * GB }), true)).toBe(false);
    expect(tooLittleSpace(base({ phase: "Failed", freeBytes: 1.2 * GB, doneBytes: 0.5 * GB }))).toBe(false);
  });

  it("says plainly when this copy cannot download", async () => {
    dl = base({ canDownload: false });
    show();
    expect(await screen.findByText(/cannot download OpenFrame AI/)).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Download OpenFrame AI" })).toBeNull();
    expect(screen.getByRole("button", { name: "Import a model file…" })).toBeTruthy();
    expect(downloadLine(base({ phase: "Checking" }))).toBe("Getting ready to download…");
  });

  it("imports a model file the person picks, and does nothing when they cancel", async () => {
    show();
    picked = null;
    await userEvent.click(await screen.findByRole("button", { name: "Import a model file…" }));
    expect(calls.some((c) => c.startsWith("ai.download.import"))).toBe(false);
    picked = "C:\\Models\\gemma.gguf";
    await userEvent.click(screen.getByRole("button", { name: "Import a model file…" }));
    await waitFor(() => expect(calls).toContain("ai.download.import C:\\Models\\gemma.gguf"));
    expect(calls).not.toContain("ai.download.start");
  });

  it("says when one more file is needed, and shows the plain reason when a file is refused", async () => {
    toasts.length = 0;
    show();
    picked = "C:\\Models\\bge.gguf";
    imported = { complete: false };
    await userEvent.click(await screen.findByRole("button", { name: "Import a model file…" }));
    await waitFor(() => expect(toasts).toContain("ok The file was added. Import the other model file to finish."));
    const { OpError } = await import("../ipc/client");
    imported = new OpError("ai.download.import", {
      code: "ai.not_a_model",
      message: "This file is not an OpenFrame AI model file, so it was not used. Nothing was changed.",
      retryable: false,
    });
    await userEvent.click(screen.getByRole("button", { name: "Import a model file…" }));
    await waitFor(() => expect(toasts).toContain("err This file is not an OpenFrame AI model file, so it was not used. Nothing was changed."));
    expect((screen.getByRole("button", { name: "Import a model file…" }) as HTMLButtonElement).disabled).toBe(false);
  });

  it("offers no import while a download runs", async () => {
    dl = base({ phase: "Downloading", doneBytes: 0.1 * GB, totalBytes: 1.1 * GB });
    show();
    expect(await screen.findByRole("button", { name: "Pause" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Import a model file…" })).toBeNull();
  });
});
