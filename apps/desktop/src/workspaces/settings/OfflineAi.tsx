// Settings → Offline AI (agentic AI spec §1, §36, §37; ADR-0025). Offline AI ships inside the app,
// so there is nothing to download, update or remove here: the status, a way to free memory, and
// the technical details ordinary UI never shows (component names, versions, licences, sizes and
// SHA-256 hashes, where the bundle is installed, this computer).

import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { call, onAppEvent } from "../../ipc/client";
import { reportError, useOp } from "../../ipc/query";
import { Banner, Button, Chip, PageHeader, Skeleton } from "../../design-system";
import { toast } from "../../app/toast";
import type { AiStatusDto } from "../../ipc/generated/AiStatusDto";
import type { AiDiagnosticsDto } from "../../ipc/generated/AiDiagnosticsDto";
import { formatBytes, offReason, runtimeLabel } from "../../ai/model";
import "../../ai/ai.css";

/** The privacy promise shown wherever Offline AI is described (spec §1). */
export const OFFLINE_AI_PROMISE =
  "Offline AI is installed with OpenFrame and runs on this computer. Your project data is not sent to OpenFrame or to an AI cloud service.";

/** Plain-language status line for the settings card. */
export function offlineAiStateText(s: AiStatusDto): string {
  if (!s.available) return s.bundleState === "Unsupported" ? "Not available on this computer" : "Needs a reinstall";
  if (s.bundleState === "Checking") return "Checking its files…";
  if (s.runtimeState === "Failed") return "Stopped after a problem. It restarts when you ask something";
  return runtimeLabel(s.runtimeState) ?? "AI Ready";
}

const GEMMA_NOTICE = "Gemma is provided under and subject to the Gemma Terms of Use found at ai.google.dev/gemma/terms.";

export default function OfflineAiSettings() {
  const qc = useQueryClient();
  const status = useOp<AiStatusDto>("ai.status", {}, [], { staleTime: 0 });

  useEffect(() => {
    const un = onAppEvent((ev) => {
      if (ev.type === "module" && ev.module === "ai") {
        void qc.invalidateQueries({ queryKey: ["ai.status"] });
        void qc.invalidateQueries({ queryKey: ["ai.diagnostics"] });
      }
    });
    return () => {
      void un.then((f) => f());
    };
  }, [qc]);

  const s = status.data;

  return (
    <div className="content" style={{ overflow: "auto" }}>
      <PageHeader title="Offline AI" sub="AI runs on this computer." />
      <div className="row" style={{ alignItems: "flex-start", gap: 16, flexWrap: "wrap" }}>
        <section className="card pad grow" style={{ maxWidth: 720 }} aria-label="Offline AI">
          {!s ? (
            <Skeleton h={80} />
          ) : (
            <>
              <div className="row" style={{ marginBottom: 8 }}>
                <div className="h4" style={{ margin: 0 }}>
                  Offline AI
                </div>
                <Chip tone={s.available ? "g" : "default"}>{offlineAiStateText(s)}</Chip>
              </div>
              <p className="sm">{OFFLINE_AI_PROMISE}</p>
              {!s.available && <Banner tone="err">{offReason(s)}</Banner>}
              {s.available && (
                <div className="row">
                  <Button
                    size="sm"
                    onClick={() => void call("ai.stop_runtime").then(() => toast.info("Offline AI stopped. It starts again when you ask something.")).catch(reportError)}
                  >
                    Stop Offline AI (free memory)
                  </Button>
                </div>
              )}
              <p className="hint">OpenFrame works normally without Offline AI. Nothing is downloaded: Offline AI is updated together with OpenFrame.</p>
            </>
          )}
        </section>
        <TechnicalDetails />
      </div>
    </div>
  );
}

/** Diagnostics / About: the only place model and runtime names appear (spec §1, §36). */
function TechnicalDetails() {
  const d = useOp<AiDiagnosticsDto>("ai.diagnostics", {}, [], { staleTime: 10_000 });
  const x = d.data;
  return (
    <section className="card pad" style={{ width: 420, flex: "none" }} aria-label="Technical details">
      <details>
        <summary className="h4" style={{ cursor: "pointer" }}>
          Technical details
        </summary>
        {d.isLoading ? (
          <Skeleton h={60} />
        ) : !x ? (
          <div className="xs muted">{d.error?.message ?? "Not available."}</div>
        ) : (
          <div className="ai-tech">
            <table className="tbl" aria-label="Offline AI components">
              <thead>
                <tr>
                  <th>Component</th>
                  <th>Licence</th>
                  <th>Size</th>
                </tr>
              </thead>
              <tbody>
                {x.components.map((c) => (
                  <tr key={c.id}>
                    <td>
                      <div>
                        <b>{c.role}</b>: {c.name}
                      </div>
                      <div className="muted">Version {c.version}</div>
                      <div className="hash selectable-text" title="SHA-256">
                        {c.sha256}
                      </div>
                    </td>
                    <td>
                      <div>{c.licenseId}</div>
                      {c.licenseUrl && <div className="muted selectable-text">{c.licenseUrl}</div>}
                    </td>
                    <td>{formatBytes(c.bytes)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
            {x.components.some((c) => c.licenseId === "Gemma-Terms-of-Use") && <p className="xs muted">{GEMMA_NOTICE}</p>}
            <div className="hr" />
            <div className="csr">
              Package: {x.bundleId} v{x.bundleVersion} · files: {fileCheckText(x.bundleState)}
            </div>
            {x.problem && <div className="csr">{x.problem}</div>}
            <div className="csr">
              Runs on: processor · language model: {x.chatState} · search model: {x.searchState}
            </div>
            <div className="csr">
              This computer: {formatBytes(x.memoryBytes)} memory · {x.processor} ({x.processorThreads} threads)
            </div>
            {x.warnings.map((w) => (
              <div className="csr" key={w}>
                {w}
              </div>
            ))}
            <div className="csr selectable-text" style={{ wordBreak: "break-all" }}>
              Installed in: {x.bundleDir || "unknown"} ({formatBytes(x.bundleBytes)}, read-only)
            </div>
            <div className="csr selectable-text" style={{ wordBreak: "break-all" }}>
              Log: {x.logFile}
            </div>
            <p className="xs muted">Everything runs on this computer. Prompts, project text and answers are never sent anywhere, and Offline AI never uses the network.</p>
          </div>
        )}
      </details>
    </section>
  );
}

function fileCheckText(state: string): string {
  switch (state) {
    case "Verified":
      return "checked";
    case "NotChecked":
      return "checked before the first answer";
    case "Checking":
      return "checking…";
    case "Missing":
      return "missing, reinstall OpenFrame Studio";
    case "Damaged":
      return "changed or damaged, reinstall OpenFrame Studio";
    default:
      return "not available on this computer";
  }
}
