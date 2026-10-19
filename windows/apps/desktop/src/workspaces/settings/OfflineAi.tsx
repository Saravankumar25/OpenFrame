// Settings → Offline AI (agentic AI spec §1, §36, §37; ADR-0026). The engine ships inside the app;
// the models are a one-time download the person starts here (or in the AI panel). This page shows
// the status, the download while OpenFrame AI is not installed, a way to free memory, Remove, and
// the technical details ordinary UI never shows (component names, versions, licences, sizes and
// SHA-256 hashes, where the engine is installed, this computer).

import { useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { call, onAppEvent } from "../../ipc/client";
import { reportError, useOp } from "../../ipc/query";
import { Banner, Button, Chip, ConfirmDialog, PageHeader, Segmented, Skeleton } from "../../design-system";
import { toast } from "../../app/toast";
import type { AiStatusDto } from "../../ipc/generated/AiStatusDto";
import type { AiDiagnosticsDto } from "../../ipc/generated/AiDiagnosticsDto";
import { damagedModels, formatBytes, offReason, panelMode, runtimeLabel } from "../../ai/model";
import DownloadAi from "../../ai/DownloadAi";
import "../../ai/ai.css";
import { AiHealthCard } from "./AiHealth";

/** The privacy promise shown wherever Offline AI is described (spec §1). */
export const OFFLINE_AI_PROMISE =
  "OpenFrame AI runs on this computer after a one-time download. Your project data is not sent to OpenFrame or to an AI cloud service.";

/** Plain-language status line for the settings card. */
export function offlineAiStateText(s: AiStatusDto): string {
  if (s.bundleState === "NotInstalled") return "Not downloaded";
  if (s.problem?.code === "ai.models_damaged") return "Needs a new download";
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
  const [removing, setRemoving] = useState(false);
  const [removeBusy, setRemoveBusy] = useState(false);
  const remove = () => {
    if (removeBusy) return;
    setRemoveBusy(true);
    void call("ai.download.remove")
      .then(() => toast.info("OpenFrame AI was removed. You can download it again here."))
      .catch(reportError)
      .finally(() => {
        // Also after a partial failure: the screens show what is really on the disk now.
        void qc.invalidateQueries({ queryKey: ["ai.status"] });
        void qc.invalidateQueries({ queryKey: ["ai.diagnostics"] });
        void qc.invalidateQueries({ queryKey: ["ai.download.status"] });
        setRemoveBusy(false);
        setRemoving(false);
      });
  };

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
              {panelMode(s) === "off" && <Banner tone="err">{offReason(s)}</Banner>}
              {s.available && (
                <div className="row">
                  <Button
                    size="sm"
                    onClick={() => void call("ai.stop_runtime").then(() => toast.info("Offline AI stopped. It starts again when you ask something.")).catch(reportError)}
                  >
                    Stop Offline AI (free memory)
                  </Button>
                  <Button size="sm" variant="ghost" onClick={() => setRemoving(true)}>
                    Remove OpenFrame AI…
                  </Button>
                </div>
              )}
              {s.available && <GraphicsChoice />}
              <p className="hint">OpenFrame works normally without OpenFrame AI.</p>
              <ConfirmDialog
                open={removing}
                onOpenChange={setRemoving}
                title="Remove OpenFrame AI?"
                confirmLabel="Remove"
                danger
                busy={removeBusy}
                onConfirm={remove}
              >
                <p className="sm">This frees the space the AI models use on this computer. AI stays off until you download it again. Your projects are not changed.</p>
              </ConfirmDialog>
            </>
          )}
        </section>
        {s && panelMode(s) === "download" && (
          <div className="grow" style={{ maxWidth: 720 }}>
            <DownloadAi damaged={damagedModels(s)} />
          </div>
        )}
        <TechnicalDetails />
        <AiHealthCard />
      </div>
    </div>
  );
}

/** Graphics card: Automatic or Off (ADR-0029 decision 6), with where the AI runs now. */
export function GraphicsChoice() {
  const qc = useQueryClient();
  const d = useOp<AiDiagnosticsDto>("ai.diagnostics", {}, [], { staleTime: 10_000 });
  if (!d.data) return null;
  const set = (v: "automatic" | "off") =>
    void call("ai.graphics.set", { off: v === "off" })
      .then(() => {
        void qc.invalidateQueries({ queryKey: ["ai.diagnostics"] });
        void qc.invalidateQueries({ queryKey: ["ai.health"] });
        toast.info(v === "off" ? "OpenFrame AI will run on the processor." : "OpenFrame AI will use a checked graphics card when there is one.");
      })
      .catch(reportError);
  return (
    <div style={{ margin: "12px 0" }}>
      <div className="row" style={{ gap: 12 }}>
        <span className="sm">
          <b>Graphics card</b>
        </span>
        <Segmented
          ariaLabel="Graphics card"
          value={d.data.graphicsOff ? "off" : "automatic"}
          onChange={set}
          options={[
            { value: "automatic", label: "Automatic" },
            { value: "off", label: "Off" },
          ]}
        />
      </div>
      <p className="hint" role="status">
        {d.data.placement} Automatic uses a graphics card only after it passes a short check; the processor is always the fallback.
      </p>
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
              Language model: {x.chatState} · search model: {x.searchState} (search always on the processor)
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
              Engine in: {x.bundleDir || "unknown"} (read-only) · engine and models: {formatBytes(x.bundleBytes)}
            </div>
            <div className="csr selectable-text" style={{ wordBreak: "break-all" }}>
              Log: {x.logFile}
            </div>
            <p className="xs muted">Everything runs on this computer. Prompts, project text and answers are never sent anywhere. The only network use is the model download you start.</p>
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
    case "NotInstalled":
      return "models not downloaded";
    case "Missing":
      return "engine missing, reinstall OpenFrame Studio";
    case "Damaged":
      return "changed or damaged";
    default:
      return "not available on this computer";
  }
}
