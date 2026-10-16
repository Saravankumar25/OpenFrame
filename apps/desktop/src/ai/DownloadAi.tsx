// Download OpenFrame AI (ADR-0026 decisions 3, 5 and 7; ADR-0032). The models are not in the
// package: the person downloads them once, here, by clicking, from Hugging Face at the revisions
// the app pins. A person who already has the files imports them instead (copied, used only when
// they match the pins). Before the click the screen shows the size, the
// free space needed and that the download uses the internet; nothing touches the network until the
// click. Then: progress with Pause, Resume where it stopped, a plain reason and Try again when it
// fails. Shown in Settings > Offline AI and in the AI panel while OpenFrame AI is not installed.

import { useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { OpError, call } from "../ipc/client";
import { reportError, useOp } from "../ipc/query";
import { Banner, Button } from "../design-system";
import { toast } from "../app/toast";
import type { AiDownloadDto } from "../ipc/generated/AiDownloadDto";
import type { AiImportResult } from "../ipc/generated/AiImportResult";
import { formatBytes } from "./model";

const RUNNING = new Set(["Checking", "Downloading", "Installing"]);

/** The line under the title, for each phase. */
export function downloadLine(d: AiDownloadDto): string {
  const progress = `${formatBytes(d.doneBytes)} of ${formatBytes(d.totalBytes || d.sizeBytes)}`;
  switch (d.phase) {
    case "Checking":
      return "Getting ready to download…";
    case "Downloading":
      return `Downloading: ${progress}`;
    case "Installing":
      return "Checking the files and getting OpenFrame AI ready…";
    case "Paused":
      return `Paused at ${progress}. It continues from there.`;
    default:
      return `A one-time download of ${formatBytes(d.sizeBytes)}. It needs ${formatBytes(d.neededBytes)} of free space${
        d.freeBytes == null ? "" : ` (${formatBytes(d.freeBytes)} free on this drive)`
      }.`;
  }
}

/** Not enough room on the drive for the rest of the download. A damaged set frees its own space
 * when it is removed first, so the app checks that case itself (it refuses with a plain message). */
export function tooLittleSpace(d: AiDownloadDto, damaged = false): boolean {
  const done = d.phase === "Paused" || d.phase === "Failed" ? d.doneBytes : 0;
  return !damaged && d.freeBytes != null && d.freeBytes < d.neededBytes - done;
}

/** What a screen reader hears: the phase, not every byte count. */
export function phaseAnnouncement(phase: string): string {
  switch (phase) {
    case "Checking":
      return "Getting ready to download.";
    case "Downloading":
      return "Downloading OpenFrame AI.";
    case "Installing":
      return "Checking the files.";
    case "Paused":
      return "Download paused.";
    case "Failed":
      return "The download stopped.";
    case "Done":
      return "OpenFrame AI is ready.";
    default:
      return "";
  }
}

export default function DownloadAi({ damaged }: { damaged?: string }) {
  const qc = useQueryClient();
  const [busy, setBusy] = useState(false);
  const q = useOp<AiDownloadDto>("ai.download.status", {}, [], {
    staleTime: 0,
    refetchInterval: (query) => (RUNNING.has(query.state.data?.phase ?? "") ? 500 : false),
  });
  const d = q.data;
  const phase = d?.phase;

  // A finished download changes what every AI surface shows.
  useEffect(() => {
    if (phase === "Done") {
      void qc.invalidateQueries({ queryKey: ["ai.status"] });
      void qc.invalidateQueries({ queryKey: ["ai.diagnostics"] });
    }
  }, [phase, qc]);

  if (!d)
    return q.isError ? (
      <section className="card pad" aria-label="Download OpenFrame AI">
        <Banner tone="err" actions={<Button size="sm" onClick={() => void q.refetch()}>Try again</Button>}>
          OpenFrame could not show the download right now.
        </Banner>
      </section>
    ) : null;
  const run = (op: string) => {
    setBusy(true);
    void call(op)
      .then(() => qc.invalidateQueries({ queryKey: ["ai.download.status"] }))
      .catch(reportError)
      .finally(() => setBusy(false));
  };
  // A damaged set is removed first, then downloaded again. Only before a download of it began:
  // a paused or stopped download keeps what it already has and resumes.
  const removeFirst = !!damaged && (d.phase === "Idle" || d.problem?.code === "ai.remove_first");
  const start = () => {
    setBusy(true);
    void (removeFirst ? call("ai.download.remove") : Promise.resolve())
      .then(() => call("ai.download.start"))
      .then(() => {
        void qc.invalidateQueries({ queryKey: ["ai.status"] });
        return qc.invalidateQueries({ queryKey: ["ai.download.status"] });
      })
      .catch(reportError)
      .finally(() => setBusy(false));
  };
  // A model file the person already has: copied and checked against the pins, never moved.
  const importFile = async () => {
    // Busy from the first click: a second click while the dialog is open does nothing.
    setBusy(true);
    try {
      const path = await openDialog({
        title: "Import an OpenFrame AI model file",
        multiple: false,
        filters: [{ name: "Model file", extensions: ["gguf"] }],
      });
      if (typeof path !== "string") return;
      const r = await call<AiImportResult>("ai.download.import", { path });
      toast.success(r.complete ? "OpenFrame AI is ready." : "The file was added. Import the other model file to finish.");
      void qc.invalidateQueries({ queryKey: ["ai.status"] });
      void qc.invalidateQueries({ queryKey: ["ai.diagnostics"] });
      await qc.invalidateQueries({ queryKey: ["ai.download.status"] });
    } catch (e) {
      if (e instanceof OpError) toast.error(e.message);
      else reportError(e);
    } finally {
      setBusy(false);
    }
  };
  const importButton = (
    <Button size="sm" disabled={busy} onClick={() => void importFile()}>
      Import a model file…
    </Button>
  );
  const running = RUNNING.has(d.phase);
  const pct = d.totalBytes ? Math.round((d.doneBytes / d.totalBytes) * 100) : 0;

  return (
    <section className="card pad" aria-label="Download OpenFrame AI">
      <div className="h4" style={{ marginTop: 0 }}>
        {damaged ? "Download OpenFrame AI again" : "Download OpenFrame AI"}
      </div>
      {damaged && <Banner tone="err">{damaged}</Banner>}
      {!d.canDownload ? (
        <>
          <p className="sm">
            This copy of OpenFrame Studio cannot download OpenFrame AI. You can import model files you already have. Everything else works normally.
          </p>
          <div className="row">{importButton}</div>
        </>
      ) : (
        <>
          <p className="sm">{downloadLine(d)}</p>
          <span className="sr-only" role="status" aria-live="polite">
            {phaseAnnouncement(d.phase)}
          </span>
          {(running || d.phase === "Paused") && (
            <div
              className="bar"
              role="progressbar"
              aria-label="OpenFrame AI download"
              aria-valuemin={0}
              aria-valuemax={100}
              // Unknown size (getting the list, checking the files): no value, so it reads as busy.
              aria-valuenow={d.phase === "Downloading" || d.phase === "Paused" ? pct : undefined}
              aria-valuetext={d.totalBytes ? downloadLine(d) : phaseAnnouncement(d.phase)}
              style={{ marginBottom: 10 }}
            >
              <i style={{ width: `${pct}%` }} />
            </div>
          )}
          {d.phase === "Failed" && d.problem && <Banner tone="err">{d.problem.message}</Banner>}
          {!running && tooLittleSpace(d, !!damaged) && (
            <Banner tone="warn">There is not enough free space on this drive. Free some space, then download.</Banner>
          )}
          {!running && (
            <p className="xs muted">
              The download uses the internet once. After it, OpenFrame AI works on this computer without the internet, and your projects are never sent
              anywhere.
            </p>
          )}
          <div className="row">
            {d.phase === "Downloading" ? (
              <Button size="sm" disabled={busy} onClick={() => run("ai.download.pause")}>
                Pause
              </Button>
            ) : running ? null : (
              <Button size="sm" variant="primary" disabled={busy || tooLittleSpace(d, !!damaged)} onClick={start}>
                {d.phase === "Paused" ? "Resume download" : d.phase === "Failed" ? "Try again" : damaged ? "Download again" : "Download OpenFrame AI"}
              </Button>
            )}
            {!running && importButton}
          </div>
        </>
      )}
    </section>
  );
}
