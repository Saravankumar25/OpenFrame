// AI health card (ADR-0028 decision 8; Windows phase W5): figures from the local, numbers-only AI
// trace, and Export AI report, which writes them to a file the person chooses. Nothing is sent.

import { useState } from "react";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import { OpError, call } from "../../ipc/client";
import { reportError, useOp } from "../../ipc/query";
import { Button, Skeleton } from "../../design-system";
import { toast } from "../../app/toast";
import type { AiHealthDto } from "../../ipc/generated/AiHealthDto";
import type { AiReportResult } from "../../ipc/generated/AiReportResult";

const seconds = (ms: number | null) => (ms == null ? "none yet" : `${(ms / 1000).toFixed(1)} s`);

export function AiHealthCard() {
  const q = useOp<AiHealthDto>("ai.health", {}, [], { staleTime: 0 });
  const [busy, setBusy] = useState(false);
  const h = q.data;

  const exportReport = async () => {
    let path: string | null;
    try {
      path = await saveDialog({
        title: "Export AI report",
        defaultPath: "openframe-ai-report.json",
        filters: [{ name: "JSON file", extensions: ["json"] }],
      });
    } catch (e) {
      reportError(e);
      return;
    }
    if (!path) return;
    setBusy(true);
    try {
      const r = await call<AiReportResult>("ai.export_report", { path });
      toast.success(`Saved ${r.fileName} (${r.rows} requests).`);
    } catch (e) {
      if (e instanceof OpError) toast.error(e.message);
      else reportError(e);
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="card pad" style={{ width: 420, flex: "none" }} aria-label="AI health">
      <div className="h4" style={{ marginTop: 0 }}>
        AI health
      </div>
      {!h ? (
        <Skeleton h={80} />
      ) : h.requests === 0 ? (
        <p className="sm muted">No requests yet. The figures appear after you use the AI Assistant.</p>
      ) : (
        <table className="tbl sm" aria-label="AI health figures">
          <tbody>
            <tr>
              <th scope="row">Requests counted</th>
              <td>{h.requests}</td>
            </tr>
            <tr>
              <th scope="row">Typical answer time</th>
              <td>{seconds(h.medianTotalMs)}</td>
            </tr>
            <tr>
              <th scope="row">Proposals applied</th>
              <td>{h.applyRatePct == null ? `${h.applied} of ${h.proposals}` : `${h.applyRatePct}% (${h.applied} of ${h.applied + h.rejected + h.undone} decided)`}</td>
            </tr>
            <tr>
              <th scope="row">Checks per answer</th>
              <td>{h.averagePasses == null ? "none yet" : h.averagePasses.toFixed(1)}</td>
            </tr>
            <tr>
              <th scope="row">Errors</th>
              <td>{h.errors}</td>
            </tr>
            <tr>
              <th scope="row">Weakest tool</th>
              <td>{h.weakestTool ? `${h.weakestTool.name}: ${h.weakestTool.failurePct}% of ${h.weakestTool.uses} uses went wrong` : "none stands out"}</td>
            </tr>
          </tbody>
        </table>
      )}
      <p className="hint">
        Only numbers are kept on this computer: no questions, answers or project text. Nothing is sent. Export AI report saves them to a file you
        choose, to share if you want to.
      </p>
      <Button size="sm" disabled={busy} onClick={() => void exportReport()}>
        {busy ? "Saving…" : "Export AI report"}
      </Button>
    </section>
  );
}
