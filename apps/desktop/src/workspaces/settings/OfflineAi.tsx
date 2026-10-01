// Settings → Offline AI (agentic AI spec §1, §24, §36, §37). Status, update, stop and remove,
// plus the technical details ordinary UI never shows: component names, versions, licences,
// sizes and SHA-256 hashes, how it runs on this computer, and where it is stored.

import { useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { call, onAppEvent } from "../../ipc/client";
import { reportError, useOp } from "../../ipc/query";
import { Banner, Button, Chip, ConfirmDialog, PageHeader, Skeleton } from "../../design-system";
import { toast } from "../../app/toast";
import type { AiStatusDto } from "../../ipc/generated/AiStatusDto";
import type { AiDiagnosticsDto } from "../../ipc/generated/AiDiagnosticsDto";
import type { AiUninstallResult } from "../../ipc/generated/AiUninstallResult";
import OfflineAiSetup, { OFFLINE_AI_PROMISE } from "../../ai/OfflineAiSetup";
import { formatBytes, runtimeLabel } from "../../ai/model";

/** Plain-language status line for the settings card. */
export function offlineAiStateText(s: AiStatusDto): string {
  if (s.install?.active) return "Setting up…";
  if (!s.installed) return "Not installed";
  if (s.runtimeState === "Failed") return "Stopped after a problem — it restarts when you ask something";
  return runtimeLabel(s.runtimeState) ?? "AI Ready";
}

const GEMMA_NOTICE = "Gemma is provided under and subject to the Gemma Terms of Use found at ai.google.dev/gemma/terms.";

export default function OfflineAiSettings() {
  const qc = useQueryClient();
  const status = useOp<AiStatusDto>("ai.status", {}, [], { staleTime: 0 });
  const [setup, setSetup] = useState(false);
  const [removeOpen, setRemoveOpen] = useState(false);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    const un = onAppEvent((ev) => {
      if ((ev.type === "module" && ev.module === "ai") || (ev.type === "task" && ev.kind === "ai.install")) {
        void qc.invalidateQueries({ queryKey: ["ai.status"] });
        void qc.invalidateQueries({ queryKey: ["ai.diagnostics"] });
        void qc.invalidateQueries({ queryKey: ["ai.setup_info"] });
      }
    });
    return () => {
      void un.then((f) => f());
    };
  }, [qc]);

  const s = status.data;
  const showSetup = !!s && (!s.installed || !!s.install?.active || setup);

  const remove = () => {
    setRemoveOpen(false);
    setBusy(true);
    call<AiUninstallResult>("ai.uninstall")
      .then((r) => {
        toast.info(`Offline AI was removed (${r.freedLabel} freed). Everything else in OpenFrame works normally.`);
        void qc.invalidateQueries({ queryKey: ["ai.status"] });
        void qc.invalidateQueries({ queryKey: ["ai.diagnostics"] });
      })
      .catch(reportError)
      .finally(() => setBusy(false));
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
                <Chip tone={s.installed ? "g" : "default"}>{offlineAiStateText(s)}</Chip>
              </div>
              {showSetup ? (
                <OfflineAiSetup status={s} onCancel={setup ? () => setSetup(false) : undefined} />
              ) : (
                <>
                  <p className="sm">{OFFLINE_AI_PROMISE}</p>
                  <div className="xs muted" style={{ marginBottom: 10 }}>
                    Uses {formatBytes(s.installedBytes)} on this computer, shared by all projects.
                  </div>
                  {s.updateAvailable && <Banner tone="info">A newer version of Offline AI is available.</Banner>}
                  <div className="row">
                    {s.updateAvailable && (
                      <Button size="sm" variant="primary" onClick={() => setSetup(true)}>
                        Update Offline AI
                      </Button>
                    )}
                    <Button
                      size="sm"
                      disabled={busy}
                      onClick={() => void call("ai.stop_runtime").then(() => toast.info("Offline AI stopped. It starts again when you ask something.")).catch(reportError)}
                    >
                      Stop Offline AI (free memory)
                    </Button>
                    <Button size="sm" variant="ghost" disabled={busy} onClick={() => setRemoveOpen(true)}>
                      Remove Offline AI…
                    </Button>
                  </div>
                </>
              )}
            </>
          )}
        </section>
        <TechnicalDetails />
      </div>
      <ConfirmDialog open={removeOpen} onOpenChange={setRemoveOpen} title="Remove Offline AI?" confirmLabel="Remove" danger onConfirm={remove}>
        The downloaded AI components ({formatBytes(s?.installedBytes ?? 0)}) are deleted from this computer. Your projects and your assistant history are not affected. You can download Offline AI again later.
      </ConfirmDialog>
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
                      <div className="muted">
                        Version {c.version}
                        {c.installed ? " · installed" : " · not installed"}
                      </div>
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
              Package: {x.profileId} {x.installedVersion ? `v${x.installedVersion}` : "(not installed)"}
              {x.availableVersion && x.availableVersion !== x.installedVersion ? ` · available v${x.availableVersion}` : ""}
            </div>
            <div className="csr">
              Runs on: {x.runsOn ?? "—"} · language model: {x.chatState} · search model: {x.searchState}
            </div>
            <div className="csr">
              This computer: {formatBytes(x.memoryBytes)} memory · {x.processor} ({x.processorThreads} threads)
              {x.graphics.length > 0 ? ` · ${x.graphics.map((g) => `${g.name} (${formatBytes(g.memoryBytes)})`).join(", ")}` : ""}
              {x.vulkanAvailable ? " · Vulkan driver present" : ""}
            </div>
            <div className="csr">
              Download information: {x.manifestChannel ?? "—"} channel, #{x.manifestSequence ?? "—"}
              {x.developmentKey ? " · development signing key (not for public release)" : ""}
            </div>
            <div className="csr selectable-text" style={{ wordBreak: "break-all" }}>
              Stored in: {x.storeDir}
            </div>
            <div className="csr selectable-text" style={{ wordBreak: "break-all" }}>
              Log: {x.logFile}
            </div>
            <p className="xs muted">Everything runs on this computer. Prompts, project text and answers are never sent anywhere; the network is used only to download these components.</p>
          </div>
        )}
      </details>
    </section>
  );
}
