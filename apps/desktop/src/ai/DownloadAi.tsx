// Download OpenFrame AI (ADR-0026 decisions 3, 5 and 7). The models are not in the package: the
// person downloads them once, here, by clicking. Before the click the screen shows the size, the
// free space needed and that the download uses the internet; nothing touches the network until the
// click. Then: progress with Pause, Resume where it stopped, a plain reason and Try again when it
// fails. Shown in Settings > Offline AI and in the AI panel while OpenFrame AI is not installed.

import { useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { call } from "../ipc/client";
import { reportError, useOp } from "../ipc/query";
import { Banner, Button } from "../design-system";
import type { AiDownloadDto } from "../ipc/generated/AiDownloadDto";
import { formatBytes } from "./model";

const RUNNING = new Set(["Checking", "Downloading", "Installing"]);

/** The line under the title, for each phase. */
export function downloadLine(d: AiDownloadDto): string {
  const progress = `${formatBytes(d.doneBytes)} of ${formatBytes(d.totalBytes || d.sizeBytes)}`;
  switch (d.phase) {
    case "Checking":
      return "Getting the download list…";
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

/** Not enough room on the drive for the rest of the download. */
export function tooLittleSpace(d: AiDownloadDto): boolean {
  return d.freeBytes != null && d.freeBytes < d.neededBytes - (d.phase === "Paused" ? d.doneBytes : 0);
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

  if (!d) return null;
  const run = (op: string) => {
    setBusy(true);
    void call(op)
      .then(() => qc.invalidateQueries({ queryKey: ["ai.download.status"] }))
      .catch(reportError)
      .finally(() => setBusy(false));
  };
  // A damaged set is removed first, then downloaded again.
  const start = () => {
    setBusy(true);
    void (damaged ? call("ai.download.remove") : Promise.resolve())
      .then(() => call("ai.download.start"))
      .then(() => qc.invalidateQueries({ queryKey: ["ai.download.status"] }))
      .catch(reportError)
      .finally(() => setBusy(false));
  };
  const running = RUNNING.has(d.phase);
  const pct = d.totalBytes ? Math.round((d.doneBytes / d.totalBytes) * 100) : 0;

  return (
    <section className="card pad" aria-label="Download OpenFrame AI">
      <div className="h4" style={{ marginTop: 0 }}>
        {damaged ? "Download OpenFrame AI again" : "Download OpenFrame AI"}
      </div>
      {damaged && <Banner tone="err">{damaged}</Banner>}
      {!d.canDownload ? (
        <p className="sm">This copy of OpenFrame Studio cannot download OpenFrame AI. Everything else works normally.</p>
      ) : (
        <>
          <p className="sm" role="status" aria-live="polite">
            {downloadLine(d)}
          </p>
          {(running || d.phase === "Paused") && (
            <div
              className="bar"
              role="progressbar"
              aria-label="OpenFrame AI download"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={pct}
              style={{ marginBottom: 10 }}
            >
              <i style={{ width: `${pct}%` }} />
            </div>
          )}
          {d.phase === "Failed" && d.problem && <Banner tone="err">{d.problem.message}</Banner>}
          {!running && tooLittleSpace(d) && (
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
              <Button size="sm" variant="primary" disabled={busy || tooLittleSpace(d)} onClick={start}>
                {d.phase === "Paused" ? "Resume download" : d.phase === "Failed" ? "Try again" : damaged ? "Download again" : "Download OpenFrame AI"}
              </Button>
            )}
          </div>
        </>
      )}
    </section>
  );
}
