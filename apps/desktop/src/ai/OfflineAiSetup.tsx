// One-click Offline AI installation (agentic AI spec §1, §24, §36; Local AI Runtime spec §1, §7).
//
// - One button, no model picker. The exact download size and the disk check are shown BEFORE
//   anything is downloaded (from `ai.setup_info`).
// - One combined progress for the whole package, with the steps the user sees:
//   Checking device → Downloading → Verifying → Installing → Starting → Ready.
// - Pause keeps what was downloaded; Cancel discards it; a failure offers Retry.
// - Ordinary UI never names the model, file format, quantization or runtime — those live in
//   Settings → Offline AI → Technical details.

import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { Check, Sparkles } from "lucide-react";
import { call } from "../ipc/client";
import { reportError, useOp } from "../ipc/query";
import { Banner, Button, ConfirmDialog, Skeleton } from "../design-system";
import type { AiStatusDto } from "../ipc/generated/AiStatusDto";
import type { AiSetupInfoDto } from "../ipc/generated/AiSetupInfoDto";
import type { AiInstallProgress } from "../ipc/generated/AiInstallProgress";
import type { AiInstallStarted } from "../ipc/generated/AiInstallStarted";
import { formatBytes, progressPercent } from "./model";
import "./OfflineAiSetup.css";

/** The privacy promise shown wherever Offline AI is offered (spec §1). */
export const OFFLINE_AI_PROMISE =
  "Download the AI components required for OpenFrame's AI features. AI runs on this computer and project data is not sent to OpenFrame or an AI cloud service.";

export const INSTALL_STEPS: { phase: string; label: string }[] = [
  { phase: "checking", label: "Checking device" },
  { phase: "downloading", label: "Downloading" },
  { phase: "verifying", label: "Verifying" },
  { phase: "installing", label: "Installing" },
  { phase: "starting", label: "Starting" },
  { phase: "ready", label: "Ready" },
];

export type StepState = "done" | "current" | "todo";

/** State of each step for the current install phase (unknown phases show nothing as current). */
export function stepStates(phase: string | null | undefined): StepState[] {
  const i = INSTALL_STEPS.findIndex((s) => s.phase === phase);
  return INSTALL_STEPS.map((_, k) => (i < 0 ? "todo" : k < i || phase === "ready" ? "done" : k === i ? "current" : "todo"));
}

/** Label of the primary button for the setup view. */
export function primaryLabel(status: AiStatusDto, info: AiSetupInfoDto | undefined): string {
  const phase = status.install?.phase;
  if (phase === "paused") return "Resume download";
  if (phase === "failed") return "Retry";
  if (status.installed && status.updateAvailable) return "Update Offline AI";
  if (info && !info.upToDate && info.downloadBytes === 0 && info.totalBytes > 0) return "Finish setup";
  return "Download Offline AI";
}

export default function OfflineAiSetup({ status, onCancel }: { status: AiStatusDto; onCancel?: () => void }) {
  if (status.install?.active) return <InstallProgressView p={status.install} />;
  return <SetupView status={status} onCancel={onCancel} />;
}

function SetupView({ status, onCancel }: { status: AiStatusDto; onCancel?: () => void }) {
  const qc = useQueryClient();
  const info = useOp<AiSetupInfoDto>("ai.setup_info", {}, [], { staleTime: 0 });
  const [busy, setBusy] = useState(false);
  const d = info.data;
  const install = status.install;
  const paused = install?.phase === "paused";
  const failed = install?.phase === "failed" ? install : null;
  const update = status.installed && status.updateAvailable;

  const start = () => {
    setBusy(true);
    call<AiInstallStarted>("ai.install", {})
      .then(() => qc.invalidateQueries({ queryKey: ["ai.status"] }))
      .catch(reportError)
      .finally(() => setBusy(false));
  };

  return (
    <div className="body ai-scroll">
      {!status.installed && (
        <div className="empty" style={{ height: "auto", padding: "12px 0 0" }}>
          <div className="ico" aria-hidden>
            <Sparkles size={26} />
          </div>
          <h2 style={{ fontSize: 15 }}>Offline AI is not installed.</h2>
        </div>
      )}
      <div className="h4" style={{ margin: 0 }}>
        {update ? "Update Offline AI" : "Enable Offline AI"}
      </div>
      <p className="sm" style={{ margin: 0 }}>
        {OFFLINE_AI_PROMISE}
      </p>
      {info.isLoading ? (
        <>
          <div className="sm muted" role="status">
            Checking device…
          </div>
          <Skeleton h={40} />
        </>
      ) : info.isError || !d ? (
        <Banner tone="err">{info.error?.message ?? "Offline AI isn't available right now."}</Banner>
      ) : !d.supported ? (
        <Banner tone="err">{d.message ?? "Offline AI isn't available for this kind of computer yet."}</Banner>
      ) : (
        <>
          <div className="cs" aria-label="Download details">
            <div className="csr">
              <span>Download size</span>
              <b>{formatBytes(d.downloadBytes)}</b>
            </div>
            {d.downloadedBytes > 0 && (
              <div className="csr">
                <span>Already downloaded</span>
                <b>{formatBytes(d.downloadedBytes)}</b>
              </div>
            )}
            <div className="csr">
              <span>Free space needed</span>
              <b>{formatBytes(d.requiredFreeBytes)}</b>
            </div>
            <div className="csr">
              <span>Free space on this drive</span>
              <b>{d.freeDiskBytes == null ? "Unknown" : formatBytes(d.freeDiskBytes)}</b>
            </div>
            <div className="csr">
              <span>Runs on</span>
              <b>{d.runsOn}</b>
            </div>
          </div>
          {d.warnings.map((w) => (
            <Banner key={w} tone="warn">
              {w}
            </Banner>
          ))}
          {!d.enoughDisk && (
            <Banner tone="err">
              Offline AI needs about {formatBytes(d.requiredFreeBytes)} of free space on this drive. Free up some space and try again. Nothing will be downloaded until then.
            </Banner>
          )}
        </>
      )}
      {failed && <Banner tone="err">{failed.error?.message ?? failed.message}</Banner>}
      {paused && (
        <Banner tone="info">
          Paused at {formatBytes(install!.bytesDone)} of {formatBytes(install!.bytesTotal)}. Resuming continues where it stopped.
        </Banner>
      )}
      {install?.phase === "cancelled" && <Banner tone="info">{install.message}</Banner>}
      <div className="row">
        <Button size="sm" variant="primary" disabled={busy || !d?.supported || !d?.enoughDisk} onClick={start}>
          {primaryLabel(status, d)}
        </Button>
        {paused && (
          <Button size="sm" variant="ghost" onClick={() => void call("ai.cancel_install", { discard: true }).catch(reportError)}>
            Discard download
          </Button>
        )}
        {onCancel && (
          <Button size="sm" variant="ghost" onClick={onCancel}>
            Not now
          </Button>
        )}
      </div>
      <p className="hint">OpenFrame works normally without Offline AI. {status.installed ? "Your current Offline AI keeps working until the update has passed its check." : ""}</p>
    </div>
  );
}

function InstallProgressView({ p }: { p: AiInstallProgress }) {
  const pct = progressPercent(p);
  const states = stepStates(p.phase);
  const [confirmCancel, setConfirmCancel] = useState(false);
  return (
    <div className="body">
      <div className="h4" style={{ margin: 0 }}>
        Setting up Offline AI
      </div>
      <ol className="ai-steps" aria-label="Setup steps">
        {INSTALL_STEPS.map((s, i) => (
          <li key={s.phase} className={states[i]} aria-current={states[i] === "current" ? "step" : undefined}>
            <span className="ai-step-mark" aria-hidden>
              {states[i] === "done" ? <Check size={12} /> : i + 1}
            </span>
            {s.label}
          </li>
        ))}
      </ol>
      <div className="sm" role="status" aria-live="polite">
        {p.message}
      </div>
      <div className="bar" role="progressbar" aria-label="Offline AI setup progress" aria-valuemin={0} aria-valuemax={100} aria-valuenow={pct ?? undefined}>
        <i style={{ width: `${pct ?? 0}%` }} />
      </div>
      {p.bytesTotal > 0 && (
        <div className="xs muted">
          {formatBytes(p.bytesDone)} of {formatBytes(p.bytesTotal)}
          {pct != null ? ` · ${pct}%` : ""}
        </div>
      )}
      <div className="row">
        <Button size="sm" onClick={() => void call("ai.cancel_install", { discard: false }).catch(reportError)}>
          Pause
        </Button>
        <Button size="sm" variant="ghost" onClick={() => setConfirmCancel(true)}>
          Cancel
        </Button>
      </div>
      <p className="hint">You can keep working while Offline AI downloads. If the connection drops, the download resumes where it stopped.</p>
      <ConfirmDialog
        open={confirmCancel}
        onOpenChange={setConfirmCancel}
        title="Cancel the download?"
        confirmLabel="Cancel download"
        cancelLabel="Keep downloading"
        danger
        onConfirm={() => {
          setConfirmCancel(false);
          void call("ai.cancel_install", { discard: true }).catch(reportError);
        }}
      >
        The partly downloaded files are deleted. Nothing is installed, and anything you already had stays as it is.
      </ConfirmDialog>
    </div>
  );
}
