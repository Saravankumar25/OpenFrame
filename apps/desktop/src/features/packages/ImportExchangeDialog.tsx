// Import Exchange / Review / Response package (FSD §47.5–47.9, §121; UX §3.39
// "Import Exchange Package" + "Stale package", mockups 161–162).
//
// Open Package → Validation → Mapping Preview → Changes by Object →
// Ambiguities/Unmatched → Select → Apply. Nothing changes before the user
// chooses; a failed apply is rolled back completely.

import { Fragment, useCallback, useEffect, useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { ChevronDown, ChevronRight, FileInput } from "lucide-react";
import { call } from "../../ipc/client";
import { reportError } from "../../ipc/query";
import type { PackageImportSession } from "../../ipc/generated/PackageImportSession";
import type { PackageApplyMode } from "../../ipc/generated/PackageApplyMode";
import { Banner, Button, Checkbox, Chip, Dialog, EmptyState } from "../../design-system";
import { dayAndTime } from "../../app/home/format";
import {
  EXCHANGE_OPEN_FILTER,
  compatibilityTone,
  defaultSelection,
  groupSelection,
  packagesApi,
  resultLabel,
  staleDefault,
  stateChip,
  toggleGroup,
} from "./api";
import { usePackagesUi } from "./store";

type StaleChoice = "view" | "comments" | "record" | "compare";

export function ImportExchangeDialog({ sessionId, onClose }: { sessionId?: string; onClose: () => void }) {
  const openUi = usePackagesUi((s) => s.open);
  const [session, setSession] = useState<PackageImportSession | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [backup, setBackup] = useState(false);
  const [busy, setBusy] = useState(false);
  const [staleStep, setStaleStep] = useState(false);
  const [staleChoice, setStaleChoice] = useState<StaleChoice>("comments");
  const [onlyText, setOnlyText] = useState(false);

  const show = useCallback((s: PackageImportSession) => {
    setSession(s);
    setSelected(defaultSelection(s.changes));
    setStaleStep(s.stale && s.status === "Previewed");
    setStaleChoice(staleDefault(s) === "commentsOnly" ? "comments" : "record");
  }, []);

  // Resume an existing Import Session (e.g. after viewing the package).
  useEffect(() => {
    if (!sessionId) return;
    call<PackageImportSession>("packages.session", { sessionId }).then(show, reportError);
  }, [sessionId, show]);

  const viewPackage = (s: PackageImportSession) =>
    openUi({ kind: "viewer", source: "session", key: s.id, back: { kind: "importExchange", sessionId: s.id } });

  const choose = async () => {
    const chosen = await openDialog({ title: "Import Exchange Package", multiple: false, filters: EXCHANGE_OPEN_FILTER });
    if (typeof chosen !== "string") return;
    setBusy(true);
    try {
      show(await packagesApi.openExchange(chosen));
    } catch (e) {
      reportError(e);
    } finally {
      setBusy(false);
    }
  };

  const apply = async (mode: PackageApplyMode) => {
    if (!session) return;
    setBusy(true);
    try {
      const s = await packagesApi.apply(session.id, mode, [...selected], backup);
      setSession(s);
      setStaleStep(false);
    } catch (e) {
      reportError(e);
    } finally {
      setBusy(false);
    }
  };

  const cancel = async () => {
    if (session && session.status === "Previewed") {
      try {
        await packagesApi.cancel(session.id);
      } catch (e) {
        reportError(e);
      }
    }
    onClose();
  };

  const undo = async () => {
    if (!session) return;
    setBusy(true);
    try {
      setSession(await packagesApi.undo(session.id));
    } catch (e) {
      reportError(e);
    } finally {
      setBusy(false);
    }
  };

  const continueStale = () => {
    if (!session) return;
    if (staleChoice === "view") viewPackage(session);
    else if (staleChoice === "comments") void apply("commentsOnly");
    else if (staleChoice === "record") void apply("record");
    else {
      setOnlyText(true);
      setStaleStep(false);
    }
  };

  // ---------------------------------------------------------------- views
  if (!session) {
    return (
      <Dialog open onOpenChange={(v) => !v && onClose()} title="Import Exchange Package" sub="What exactly will this imported package change?" size="md">
        <EmptyState
          icon={<FileInput size={28} />}
          title="Choose a package"
          actions={<Button variant="primary" disabled={busy} onClick={() => void choose()}>{busy ? "Validating…" : "Choose Package…"}</Button>}
        >
          Review, response, Story Board, Breakdown, Shot List, Schedule or Call Sheet review packages. The package is validated first; nothing changes until you choose what to import.
        </EmptyState>
      </Dialog>
    );
  }

  const finished = session.status !== "Previewed";
  const result = session.result;

  if (staleStep) {
    const options: { value: StaleChoice; title: string; hint: string; show: boolean; isDefault?: boolean }[] = [
      { value: "view", title: "View the review without importing", hint: "Open the package read-only. Nothing is imported.", show: session.canView },
      {
        value: "comments",
        title: "Import comments only",
        hint: `Attach comments to matching text; your ${session.hostVersion} is not touched.`,
        show: session.changes.some((c) => c.kind === "comment"),
        isDefault: true,
      },
      { value: "record", title: "Create a separate review record", hint: "Keep the package and its notes for reference without changing your work.", show: session.canCreateRecord },
      { value: "compare", title: "Compare versions", hint: `${session.sourceVersion} (package) vs ${session.hostVersion} (yours)`, show: true },
    ];
    return (
      <Dialog
        open
        onOpenChange={(v) => !v && void cancel()}
        title={session.staleTitle ?? "This package was created from an older project state."}
        sub="Staleness does not mean rejection — it means the package needs review."
        size="md"
        footer={
          <>
            <Button onClick={() => void cancel()}>Cancel</Button>
            <Button variant="primary" disabled={busy} onClick={continueStale}>Continue</Button>
          </>
        }
      >
        <div className="sm muted" style={{ marginBottom: 8 }}>This package was created from an older project state.</div>
        <div className="col" style={{ gap: 8 }} role="radiogroup" aria-label="What to do with this package">
          {options.filter((o) => o.show).map((o) => (
            <label key={o.value} className="card pad row" style={{ cursor: "pointer", alignItems: "flex-start" }}>
              <input type="radio" name="stale-choice" checked={staleChoice === o.value} onChange={() => setStaleChoice(o.value)} />
              <span>
                <b>{o.title}</b> {o.isDefault && <Chip tone="g">Default</Chip>}
                <div className="sm muted">{o.hint}</div>
              </span>
            </label>
          ))}
        </div>
      </Dialog>
    );
  }

  const visible = session.changes.filter((c) => !c.copyOnly && (!onlyText || c.group === "text"));
  const groups = session.groups.filter((g) => g.key !== "copy" && (!onlyText || g.key === "text"));
  const commentsReceived = session.changes.filter((c) => c.kind === "comment" && c.applicable).length;
  const nSelected = [...selected].length;

  return (
    <Dialog
      open
      onOpenChange={(v) => !v && (finished ? onClose() : void cancel())}
      title="Import Exchange Package"
      sub="What exactly will this imported package change?"
      size="lg"
      footerLeft={<span className="muted xs">Validation happened before anything was changed.</span>}
      footer={
        finished ? (
          <>
            {result?.canUndo && session.status !== "Undone" && (
              <Button disabled={busy} onClick={() => void undo()}>Undo Import</Button>
            )}
            {(result?.queued ?? 0) > 0 && <Button onClick={() => openUi({ kind: "queue" })}>Open Review Queue</Button>}
            <Button variant="primary" onClick={onClose}>Close</Button>
          </>
        ) : (
          <>
            <Button onClick={() => void cancel()}>Cancel</Button>
            {session.compatibility !== "Rejected" && (
              <>
                {session.canView && <Button onClick={() => viewPackage(session)}>View Package</Button>}
                {session.canCreateRecord && <Button disabled={busy} onClick={() => void apply("record")}>Keep as Review Record</Button>}
                {session.canCopyAsNew && <Button disabled={busy} onClick={() => void apply("copyAsNew")}>Copy as New</Button>}
                <Button disabled={busy} onClick={() => void apply("commentsOnly")}>Comments Only</Button>
                <Button disabled={busy} onClick={() => void apply("allSafe")}>Accept All Safe</Button>
                <Button variant="primary" disabled={busy || nSelected === 0} onClick={() => void apply("selected")}>Import Selected</Button>
              </>
            )}
          </>
        )
      }
    >
      <div className="grid g4" style={{ gap: 8, marginBottom: 8 }}>
        <div className="card pad"><div className="xs muted">Package type</div><b>{session.typeLabel}</b></div>
        <div className="card pad"><div className="xs muted">Source project</div><b>{session.sourceProjectTitle}</b></div>
        <div className="card pad">
          <div className="xs muted">Source version</div>
          <b>{session.sourceVersion}</b>
          <div className="xs muted">Yours: {session.hostVersion}</div>
        </div>
        <div className="card pad"><div className="xs muted">Compatibility</div><Chip tone={compatibilityTone(session.compatibility)}>{session.compatibility}</Chip></div>
      </div>
      <div className="xs muted" style={{ marginBottom: 6 }}>
        Exported by {session.exportedBy} · {dayAndTime(session.exportedAt)} · {session.fileName}
      </div>
      {result ? (
        <Banner tone={result.status === "Failed" ? "err" : result.status === "Applied" || result.status === "Undone" ? "ok" : "info"}>
          <b>{resultLabel(result.status)}.</b> {result.message}
          {result.safetyBackup && <div className="xs muted selectable-text">Backup before import: {result.safetyBackup}</div>}
        </Banner>
      ) : session.rejection ? (
        <Banner tone="err">{session.rejection} Your project was not changed.</Banner>
      ) : session.stale ? (
        <Banner tone="warn">{session.staleTitle} Staleness does not mean rejection — review what you import.</Banner>
      ) : commentsReceived > 0 ? (
        <Banner tone="ok"><b>{commentsReceived} new comment{commentsReceived === 1 ? "" : "s"} received.</b> Comments are attached to the matching scenes and text where safe.</Banner>
      ) : null}
      {session.warnings.map((w) => (
        <Banner key={w} tone="info">{w}</Banner>
      ))}
      {onlyText && (
        <Banner tone="info" actions={<Button size="sm" onClick={() => setOnlyText(false)}>Show all changes</Button>}>
          Comparing {session.sourceVersion} (package) with {session.hostVersion} (yours). Text differences are shown for review only — import never changes screenplay text.
        </Banner>
      )}
      {groups.length > 0 && (
        <div className="card" style={{ marginTop: 8, maxHeight: 360, overflow: "auto" }}>
          <table className="tbl">
            <thead>
              <tr>
                <th>Changes by object</th>
                <th style={{ width: 60 }}>Count</th>
                <th style={{ width: 150 }}>State</th>
                <th style={{ width: 60 }}>Apply</th>
              </tr>
            </thead>
            <tbody>
              {groups.map((g) => {
                const chip = stateChip(g.state);
                const sel = groupSelection(selected, session.changes, g.key);
                const open = expanded.has(g.key);
                const items = visible.filter((c) => c.group === g.key);
                return (
                  <Fragment key={g.key}>
                    <tr>
                      <td>
                        <button
                          className="iconbtn"
                          style={{ width: 22, height: 22 }}
                          aria-expanded={open}
                          aria-label={`${open ? "Hide" : "Show"} ${g.label}`}
                          onClick={() => setExpanded((prev) => {
                            const next = new Set(prev);
                            if (open) next.delete(g.key);
                            else next.add(g.key);
                            return next;
                          })}
                        >
                          {open ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                        </button>{" "}
                        <b>{g.label}</b>
                      </td>
                      <td style={{ textAlign: "center" }}>{g.count}</td>
                      <td><Chip tone={chip.tone}>{chip.label}</Chip></td>
                      <td>
                        {g.applicable > 0 && !finished ? (
                          <input
                            type="checkbox"
                            aria-label={`Apply ${g.label}`}
                            checked={sel === "all"}
                            ref={(el) => {
                              if (el) el.indeterminate = sel === "some";
                            }}
                            onChange={(e) => setSelected(toggleGroup(selected, session.changes, g.key, e.target.checked))}
                          />
                        ) : (
                          <span className="xs muted">—</span>
                        )}
                      </td>
                    </tr>
                    {open &&
                      items.map((c) => {
                        const ch = stateChip(c.state);
                        return (
                          <tr key={c.id} className="sm">
                            <td style={{ paddingLeft: 36 }}>
                              {c.label}
                              {c.detail && <div className="xs muted">{c.detail}</div>}
                              {c.candidates.length > 0 && (
                                <div className="xs muted">Possible places: {c.candidates.map((x) => x.label).join(" · ")}</div>
                              )}
                            </td>
                            <td />
                            <td><Chip tone={ch.tone}>{ch.label}</Chip></td>
                            <td>
                              {c.applicable && !finished ? (
                                <Checkbox
                                  checked={selected.has(c.id)}
                                  onChange={(v) => setSelected((prev) => {
                                    const next = new Set(prev);
                                    if (v) next.add(c.id);
                                    else next.delete(c.id);
                                    return next;
                                  })}
                                  label={<span className="sr-only">Apply {c.label}</span>}
                                />
                              ) : null}
                            </td>
                          </tr>
                        );
                      })}
                  </Fragment>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
      {!finished && session.compatibility !== "Rejected" && (
        <div style={{ marginTop: 8 }}>
          <Checkbox checked={backup} onChange={setBackup} label="Create Backup Before Import" />
          <div className="xs muted">
            Ambiguous and unmatched notes go to the Review Queue — nothing is discarded. Conflicts are never applied unless you select them.
          </div>
        </div>
      )}
    </Dialog>
  );
}
