// AI Assistant side panel (FSD §42; UX §3.40; mockups 174–181; Local AI Runtime spec §1).
//
// - OpenFrame works fully without it: when Offline AI isn't installed the panel
//   offers "Download Offline AI" and nothing else depends on it.
// - Everything runs on this computer; the UI never shows model files, formats or ports.
// - Answers show scope ("Using: …"), provenance and Exact/Inferred markers.
// - Project changes arrive only as Change Set previews that the user explicitly
//   applies or cancels; stale proposals must be re-checked first.

import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { Cpu, HardDrive, MemoryStick, MoreHorizontal, Sparkles, X } from "lucide-react";
import { call, onAppEvent } from "../ipc/client";
import { reportError, useOp } from "../ipc/query";
import { Banner, Button, Chip, ConfirmDialog, IconButton, Menu, Segmented, Select, Skeleton, TextArea } from "../design-system";
import { useNav } from "../app/stores";
import { toast } from "../app/toast";
import type { ProjectSummary } from "../ipc/generated/ProjectSummary";
import type { AiStatusDto } from "../ipc/generated/AiStatusDto";
import type { AiHardwareDto } from "../ipc/generated/AiHardwareDto";
import type { AiHistoryDto } from "../ipc/generated/AiHistoryDto";
import type { AiExchange } from "../ipc/generated/AiExchange";
import type { AiResultDto } from "../ipc/generated/AiResultDto";
import type { AiChangeSetDto } from "../ipc/generated/AiChangeSetDto";
import type { AiInstallStarted } from "../ipc/generated/AiInstallStarted";
import type { AiScopeKind } from "../ipc/generated/AiScopeKind";
import {
  SCOPE_LABELS,
  STALE_HINT,
  UNAVAILABLE_TEXT,
  changeSetActions,
  changeSetStateText,
  confidenceLabel,
  defaultScope,
  formatBytes,
  panelMode,
  progressPercent,
  runtimeLabel,
  scopeArgs,
  scopeOptions,
  toRoute,
} from "./model";
import "./ai.css";

const HISTORY_TABLES = ["ai_request", "ai_result", "change_set"];

export default function AiPanel({ onClose }: { onClose: () => void }) {
  const qc = useQueryClient();
  const project = useOp<ProjectSummary | null>("project.current", {}, ["project"]);
  const status = useOp<AiStatusDto>("ai.status", {}, [], { staleTime: 0 });
  const [setup, setSetup] = useState(false);
  const [removeOpen, setRemoveOpen] = useState(false);

  // Runtime / install progress arrive as module events from the Rust core.
  useEffect(() => {
    const un = onAppEvent((ev) => {
      if ((ev.type === "module" && ev.module === "ai") || (ev.type === "task" && ev.kind === "ai.install")) {
        void qc.invalidateQueries({ queryKey: ["ai.status"] });
        if (ev.type === "task") void qc.invalidateQueries({ queryKey: ["ai.hardware"] });
      }
    });
    return () => {
      void un.then((f) => f());
    };
  }, [qc]);

  const s = status.data;
  const mode = panelMode(s, setup);
  useEffect(() => {
    if (s?.installed && s.install?.phase === "ready") setSetup(false);
  }, [s?.installed, s?.install?.phase]);

  const active = s?.activeProfile ?? null;
  const settings = s?.installed
    ? [
        { label: "Offline AI", header: true },
        { label: "Change AI quality…", onSelect: () => setSetup(true) },
        { label: "Stop Offline AI (free memory)", onSelect: () => void call("ai.stop_runtime").then(() => toast.info("Offline AI stopped. It starts again when you ask something.")).catch(reportError) },
        { label: "Remove downloaded AI…", danger: true, separatorBefore: true, onSelect: () => setRemoveOpen(true) },
      ]
    : [];

  return (
    <aside className="ai" aria-label="AI Assistant">
      <div className="ahd">
        <Sparkles size={16} aria-hidden />
        AI Assistant
        <span className="grow" />
        <Chip tone={s?.installed ? "g" : "default"} title={s?.installed ? "Runs on this computer" : undefined}>
          {s?.mode ?? "Off"}
        </Chip>
        {settings.length > 0 && (
          <Menu
            align="end"
            items={settings}
            trigger={
              <button type="button" className="iconbtn" aria-label="Offline AI settings" title="Offline AI settings">
                <MoreHorizontal size={16} />
              </button>
            }
          />
        )}
        <IconButton label="Close AI Assistant" onClick={onClose}>
          <X size={16} />
        </IconButton>
      </div>
      {status.isLoading ? (
        <div className="body">
          <Skeleton h={28} />
          <Skeleton h={60} />
        </div>
      ) : mode === "unavailable" ? (
        <Unavailable error={status.isError} onSetup={() => setSetup(true)} />
      ) : mode === "setup" || (mode === "ready" && setup) ? (
        <SetupView status={s!} onCancel={() => setSetup(false)} />
      ) : mode === "installing" ? (
        <InstallProgressView status={s!} />
      ) : (
        <Assistant status={s!} hasProject={!!project.data} />
      )}
      <ConfirmDialog
        open={removeOpen}
        onOpenChange={setRemoveOpen}
        title="Remove downloaded AI?"
        confirmLabel="Remove"
        danger
        onConfirm={() => {
          setRemoveOpen(false);
          if (!active) return;
          call("ai.remove_model", { profileId: active.profileId })
            .then(() => {
              toast.info("Offline AI was removed. Everything else in OpenFrame works normally.");
              void qc.invalidateQueries({ queryKey: ["ai.status"] });
            })
            .catch(reportError);
        }}
      >
        The downloaded AI ({active ? formatBytes(active.sizeBytes) : "model"}) is deleted from this computer. Your projects and your assistant history are not affected. You can download it again later.
      </ConfirmDialog>
    </aside>
  );
}

// ----------------------------------------------------------------- not installed

function Unavailable({ error, onSetup }: { error: boolean; onSetup: () => void }) {
  return (
    <div className="body">
      <div className="xs muted">Using: —</div>
      <div className="empty" style={{ height: "auto", padding: 20 }}>
        <div className="ico" aria-hidden>
          <Sparkles size={26} />
        </div>
        <h2 style={{ fontSize: 15 }}>AI is not available right now</h2>
        {error ? (
          <p className="sm">{UNAVAILABLE_TEXT}</p>
        ) : (
          <p className="sm">
            Offline AI isn't installed on this computer. <b>Everything else in OpenFrame works normally.</b>
          </p>
        )}
        {!error && (
          <div className="row">
            <Button size="sm" variant="primary" onClick={onSetup}>
              Download Offline AI
            </Button>
          </div>
        )}
        <p className="hint">Offline AI runs entirely on this computer. No account is needed and your project is never sent anywhere.</p>
      </div>
    </div>
  );
}

function SetupView({ status, onCancel }: { status: AiStatusDto; onCancel: () => void }) {
  const qc = useQueryClient();
  const hw = useOp<AiHardwareDto>("ai.hardware", {}, [], { staleTime: 30_000 });
  const [choice, setChoice] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const rec = hw.data?.recommendation;
  const profiles = hw.data?.profiles ?? status.profiles;
  const selected = choice ?? status.install?.profileId ?? rec?.profileId ?? null;
  const prof = profiles.find((p) => p.profileId === selected);
  const paused = status.install?.phase === "paused";
  const failed = status.install?.phase === "failed" ? status.install : null;
  const isRecommended = selected === rec?.profileId;

  const start = () => {
    setBusy(true);
    call<AiInstallStarted>("ai.install", { profileId: selected })
      .then(() => qc.invalidateQueries({ queryKey: ["ai.status"] }))
      .catch(reportError)
      .finally(() => setBusy(false));
  };

  if (hw.isLoading) {
    return (
      <div className="body">
        <div className="sm">Checking this computer…</div>
        <Skeleton h={80} />
      </div>
    );
  }
  if (hw.isError || !rec) {
    return (
      <div className="body">
        <Banner tone="err">{hw.error?.message ?? "Offline AI isn't available for this kind of computer yet."}</Banner>
        <div className="row">
          <Button size="sm" variant="ghost" onClick={onCancel}>
            Close
          </Button>
        </div>
      </div>
    );
  }
  const remaining = prof ? Math.max(0, prof.sizeBytes - prof.downloadedBytes) : rec.downloadBytes;
  return (
    <div className="body ai-scroll">
      <div className="h4" style={{ margin: 0 }}>
        Download Offline AI
      </div>
      <p className="sm" style={{ margin: 0 }}>
        OpenFrame checked this computer and recommends <b>{rec.name}</b>. After the download, AI works without internet.
      </p>
      <div className="cs">
        <div className="csh">This computer</div>
        <div className="csr">
          <span>
            <MemoryStick size={13} aria-hidden /> Memory
          </span>
          <b>{formatBytes(hw.data!.memoryBytes)}</b>
        </div>
        <div className="csr">
          <span>
            <Cpu size={13} aria-hidden /> Runs on
          </span>
          <b>{rec.runsOn}</b>
        </div>
        <div className="csr">
          <span>
            <HardDrive size={13} aria-hidden /> Free space
          </span>
          <b>{hw.data!.freeDiskBytes == null ? "Unknown" : formatBytes(hw.data!.freeDiskBytes)}</b>
        </div>
      </div>
      <div>
        <div className="xs muted" style={{ marginBottom: 4 }}>
          AI quality
        </div>
        <Segmented
          ariaLabel="AI quality"
          value={selected ?? rec.profileId}
          onChange={(v) => setChoice(v)}
          options={profiles.map((p) => ({ value: p.profileId, label: p.recommended ? `${p.name} ★` : p.name }))}
        />
        {prof && (
          <div className="xs muted" style={{ marginTop: 6 }}>
            {prof.recommended ? "Recommended for this computer. " : ""}
            Size {formatBytes(prof.sizeBytes)}.{prof.note ? ` ${prof.note}` : ""}
            {prof.installed ? " Already downloaded." : ""}
            {prof.startFailed ? " It couldn't start on this computer last time." : ""}
          </div>
        )}
      </div>
      {isRecommended && rec.warnings.map((w) => <Banner key={w} tone="warn">{w}</Banner>)}
      {!rec.enoughDisk && (
        <Banner tone="err">
          Offline AI needs about {formatBytes(rec.requiredFreeBytes)} of free space on this drive. Free up some space and try again. Nothing will be downloaded until then.
        </Banner>
      )}
      {failed && (
        <Banner tone="err">
          {failed.error?.message ?? failed.message}
        </Banner>
      )}
      {paused && (
        <Banner tone="info">
          Paused at {formatBytes(status.install!.bytesDone)}. Resuming continues where it stopped.
        </Banner>
      )}
      <div className="row">
        <Button size="sm" variant="primary" disabled={busy || !rec.enoughDisk || !selected} onClick={start}>
          {paused ? "Resume download" : failed ? "Retry" : prof?.installed ? "Use this AI" : `Download Offline AI (${formatBytes(remaining)})`}
        </Button>
        <Button size="sm" variant="ghost" onClick={onCancel}>
          Not now
        </Button>
      </div>
      <p className="hint">
        Everything runs on this computer. Nothing about your project is sent anywhere, and your current AI (if any) keeps working until the new one has passed its check.
      </p>
    </div>
  );
}

function InstallProgressView({ status }: { status: AiStatusDto }) {
  const p = status.install!;
  const pct = progressPercent(p);
  const [confirmCancel, setConfirmCancel] = useState(false);
  const downloading = p.phase === "downloadingModel" || p.phase === "downloadingRuntime";
  return (
    <div className="body">
      <div className="h4" style={{ margin: 0 }}>
        Setting up Offline AI
      </div>
      <div className="sm" role="status" aria-live="polite">
        {p.message}
      </div>
      <div className="bar" role="progressbar" aria-label="Download progress" aria-valuemin={0} aria-valuemax={100} aria-valuenow={pct ?? undefined}>
        <i style={{ width: `${pct ?? (downloading ? 0 : 100)}%` }} />
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
        The partly downloaded files are deleted. Nothing is installed, and any AI you already had stays as it is.
      </ConfirmDialog>
    </div>
  );
}

// -------------------------------------------------------------------- assistant

function Assistant({ status, hasProject }: { status: AiStatusDto; hasProject: boolean }) {
  const qc = useQueryClient();
  const route = useNav((s) => s.route);
  const go = useNav((s) => s.go);
  const options = useMemo(() => scopeOptions(route), [route]);
  const [scope, setScope] = useState<AiScopeKind>(() => defaultScope(route));
  const [conversation, setConversation] = useState<string | null | undefined>(undefined);
  const [text, setText] = useState("");
  const [pending, setPending] = useState<string | null>(null);
  const endRef = useRef<HTMLDivElement>(null);
  const effectiveScope = options.includes(scope) ? scope : defaultScope(route);

  const history = useOp<AiHistoryDto>("ai.history", { conversationId: conversation ?? null, limit: 50 }, HISTORY_TABLES, {
    enabled: hasProject && conversation !== null,
  });
  const exchanges = conversation === null ? [] : history.data?.exchanges ?? [];
  const last = exchanges[exchanges.length - 1];

  useEffect(() => {
    endRef.current?.scrollIntoView?.({ block: "end" });
  }, [exchanges.length, pending]);

  const refresh = () => qc.invalidateQueries({ queryKey: ["ai.history"] });

  const send = () => {
    const q = text.trim();
    if (!q || pending) return;
    setPending(q);
    setText("");
    call<AiExchange>("ai.ask", {
      text: q,
      scope: scopeArgs(effectiveScope, route),
      conversationId: conversation === null ? null : (conversation ?? history.data?.conversationId ?? null),
    })
      .then((x) => {
        setConversation(x.conversationId);
        void refresh();
        // Navigation is application control, not a project change: it runs directly (AI spec §5.3).
        if (x.result.kind === "Navigate") {
          const r = toRoute(x.result.nav);
          if (r) go(r);
        }
      })
      .catch((e) => {
        setText(q);
        reportError(e);
      })
      .finally(() => setPending(null));
  };

  const onKey = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  };

  const rl = runtimeLabel(status.runtimeState);
  const usingLabel = last && last.scopeLabel && scopeOfLast(last) === effectiveScope ? last.scopeLabel : `Using: ${SCOPE_LABELS[effectiveScope]}`;

  if (!hasProject) {
    return (
      <div className="body">
        <div className="empty" style={{ height: "auto", padding: 20 }}>
          <h2 style={{ fontSize: 15 }}>Open a project to ask about it</h2>
          <p className="sm">The assistant answers from the project that is open, on this computer.</p>
        </div>
      </div>
    );
  }

  return (
    <>
      <div className="body" style={{ paddingBottom: 0 }}>
        <div className="row">
          <label className="xs muted" htmlFor="ai-scope">
            Scope
          </label>
          <div style={{ flex: 1 }}>
            <Select<AiScopeKind>
              id="ai-scope"
              value={effectiveScope}
              onChange={setScope}
              options={options.map((k) => ({ value: k, label: SCOPE_LABELS[k] }))}
            />
          </div>
          <Button size="xs" variant="ghost" onClick={() => setConversation(null)} title="Start a new conversation">
            New
          </Button>
        </div>
        <div className="xs muted">{usingLabel}</div>
        {status.runtimeState === "Failed" && (
          <Banner tone="warn">{status.runtimeMessage ?? UNAVAILABLE_TEXT} Ask again to restart it.</Banner>
        )}
        {rl && <div className="xs muted" role="status">{rl}</div>}
        <div className="ai-conv" aria-live="polite">
          {exchanges.length === 0 && !pending && (
            <div className="xs muted" style={{ padding: "8px 0" }}>
              Ask about your project — “How many scenes are in the script?”, “Open Scene 12.”, “Which scenes contain Ravi and Arjun?”. Changes are
              only ever prepared for you to review.
            </div>
          )}
          {exchanges.map((x) => (
            <ExchangeView key={x.requestId} x={x} onChanged={refresh} />
          ))}
          {pending && (
            <>
              <div className="msg u">{pending}</div>
              <div className="msg a muted" role="status">
                Working on it on this computer…
              </div>
            </>
          )}
          <div ref={endRef} />
        </div>
      </div>
      <div className="ai-input">
        <TextArea
          aria-label="Ask or command"
          placeholder="Ask or command… e.g. “Open Scene 42.”"
          rows={2}
          maxLength={2000}
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={onKey}
          disabled={!!pending}
        />
        <Button size="sm" variant="primary" onClick={send} disabled={!text.trim() || !!pending}>
          Send
        </Button>
      </div>
    </>
  );
}

function scopeOfLast(x: AiExchange): AiScopeKind | null {
  for (const [k, label] of Object.entries(SCOPE_LABELS) as [AiScopeKind, string][]) {
    if (x.scopeLabel === `Using: ${label}`) return k;
  }
  return null;
}

function ExchangeView({ x, onChanged }: { x: AiExchange; onChanged: () => void }) {
  return (
    <>
      <div className="msg u">{x.requestText}</div>
      <ResultView r={x.result} onChanged={onChanged} />
    </>
  );
}

function ResultView({ r, onChanged }: { r: AiResultDto; onChanged: () => void }) {
  const go = useNav((s) => s.go);
  const conf = confidenceLabel(r.confidence);
  const navRoute = toRoute(r.nav);
  const denied = r.kind === "Denied";
  return (
    <>
      <div className={`msg a${r.kind === "Failed" || r.kind === "Unavailable" ? " ai-err" : ""}`} data-kind={r.kind}>
        {denied || r.kind === "Answer" || r.kind === "Proposal" || r.kind === "Suggestion" ? <b>{r.content}</b> : r.content}
        {r.details.length > 0 && (
          <div className="xs muted" style={{ marginTop: 4 }}>
            {r.details.map((d, i) => (
              <div key={i}>{denied && !d.startsWith("Nothing") ? `• ${d}` : d}</div>
            ))}
          </div>
        )}
        {r.items.length > 0 && (
          <ul className="ai-items">
            {r.items.slice(0, 25).map((it, i) => {
              const to = toRoute(it.nav);
              return (
                <li key={i}>
                  {r.kind === "Suggestion" && <Chip tone="y">Suggested</Chip>}
                  <span className="grow">
                    {it.label}
                    {it.detail && <span className="muted"> · {it.detail}</span>}
                  </span>
                  {to && (
                    <Button size="xs" onClick={() => go(to)}>
                      Show
                    </Button>
                  )}
                </li>
              );
            })}
            {r.items.length > 25 && <li className="muted">…and {r.items.length - 25} more</li>}
          </ul>
        )}
        {(conf || r.provenance.length > 0 || navRoute) && (
          <div className="row" style={{ marginTop: 6, flexWrap: "wrap", gap: 6 }}>
            {conf && <Chip tone={conf.tone}>{conf.text}</Chip>}
            {navRoute && r.kind !== "Navigate" && (
              <Button size="xs" onClick={() => go(navRoute)}>
                Open
              </Button>
            )}
            {navRoute && r.kind === "Navigate" && (
              <Button size="xs" onClick={() => go(navRoute)}>
                Open again
              </Button>
            )}
          </div>
        )}
        {r.provenance.length > 0 && (
          <div className="xs muted" style={{ marginTop: 4 }}>
            Based on: {r.provenance.map((p) => `${p.kind}: ${p.label}`).join(" · ")}
          </div>
        )}
      </div>
      {r.changeSet && <ChangeSetCard cs={r.changeSet} onChanged={onChanged} />}
    </>
  );
}

export function ChangeSetCard({ cs, onChanged }: { cs: AiChangeSetDto; onChanged: () => void }) {
  const [review, setReview] = useState(false);
  const [busy, setBusy] = useState(false);
  const actions = changeSetActions(cs);
  const stateText = changeSetStateText(cs);
  const stale = cs.state === "Stale" || cs.state === "Conflict";

  const run = (op: string, done?: (r: AiChangeSetDto) => void) => {
    setBusy(true);
    call<AiChangeSetDto>(op, { id: cs.id })
      .then((r) => {
        done?.(r);
        onChanged();
      })
      .catch(reportError)
      .finally(() => setBusy(false));
  };

  return (
    <>
      {stale ? (
        <div className="cs" style={{ borderColor: "var(--yellow)" }} role="alert">
          <div className="csh" style={{ background: "var(--yellow-soft)" }}>
            This proposal is out of date
          </div>
          <div className="csr" style={{ display: "block" }}>
            {cs.staleReason ?? "The project changed after this suggestion was prepared. Review is required before applying it."}
          </div>
        </div>
      ) : (
        <div className="cs" aria-label="Proposed changes">
          <div className="csh">{cs.title}</div>
          {cs.rows.map((row, i) => (
            <div key={`r${i}`} className="csr">
              <span>{row.label}</span>
              <b>{row.value}</b>
            </div>
          ))}
          {cs.exclusions.map((row, i) => (
            <div key={`x${i}`} className={`csr ${row.tone === "locked" ? "ai-locked" : "ai-excluded"}`}>
              <span>{row.label}</span>
              <b>{row.value}</b>
            </div>
          ))}
          {review && (
            <div className="csr" style={{ display: "block" }}>
              <div className="xs muted">
                {cs.operationCount === 1 ? "1 change" : `${cs.operationCount} changes`} · Affects: {cs.affectedModules.join(", ") || "—"}
              </div>
              {cs.targets.length > 0 && <div className="xs">Objects: {cs.targets.map((t) => t.label).join(", ")}</div>}
              <div className="xs muted">Applied together as one step you can undo. If any part fails, nothing is changed.</div>
            </div>
          )}
        </div>
      )}
      {stateText && (
        <div className={`xs ${cs.state === "Failed" ? "" : "muted"}`} role={cs.state === "Failed" ? "alert" : undefined}>
          {stateText}
        </div>
      )}
      {(actions.apply || actions.reject || actions.recheck) && (
        <div className="row">
          {actions.apply && (
            <>
              <Button size="sm" onClick={() => setReview((v) => !v)} aria-expanded={review}>
                Review Changes
              </Button>
              <Button
                size="sm"
                variant="primary"
                disabled={busy}
                onClick={() =>
                  run("ai.change_set.accept", (r) => {
                    if (r.state === "Applied") toast.undoable(`Applied: ${r.title}`);
                  })
                }
              >
                Apply
              </Button>
            </>
          )}
          {actions.recheck && (
            <Button size="sm" variant="primary" disabled={busy} onClick={() => run("ai.change_set.recheck")}>
              Re-check &amp; Review
            </Button>
          )}
          {actions.reject && (
            <Button size="sm" variant="ghost" disabled={busy} onClick={() => run("ai.change_set.reject")}>
              Cancel
            </Button>
          )}
        </div>
      )}
      {stale && <div className="hint">{STALE_HINT}</div>}
    </>
  );
}
