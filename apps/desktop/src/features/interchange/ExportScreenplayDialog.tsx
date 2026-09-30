// Export Screenplay (FSD §20, §117, §122–123, FSD-SCRIPT-045..050;
// Import/Export §5; mocks 095 export and 096 print preview).
// An export is a snapshot written to a location the user chooses; the
// project never changes and private notes are never included.

import { useCallback, useMemo, useState } from "react";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import { ChevronLeft, ChevronRight, Minus, Plus } from "lucide-react";
import { Banner, Button, Checkbox, Chip, Dialog, EmptyState, IconButton, Select, Skeleton } from "../../design-system";
import { OpError, revealLocation } from "../../ipc/client";
import { reportError } from "../../ipc/query";
import { toast } from "../../app/toast";
import type { InterchangeScreenplayExportOptions } from "../../ipc/generated/InterchangeScreenplayExportOptions";
import type { InterchangeScreenplayExportResult } from "../../ipc/generated/InterchangeScreenplayExportResult";
import type { InterchangeSources } from "../../ipc/generated/InterchangeSources";
import { interchange, useDraftScenes, useInterchangeSources, usePrintPreview } from "./api";
import { EXPORT_FORMATS, ZOOMS, defaultExportFileName, draftLabel, formatSpec, saveFilters, statusTone, type InterchangeExportFormat } from "./model";
import { PrintJob, ScaledPage, pageTitle } from "./PrintPreview";
import "./interchange.css";

export interface ExportScreenplayDialogProps {
  open: boolean;
  onClose: () => void;
  /** Draft to export; defaults to the current draft of the first screenplay. */
  draftId?: string;
}

interface DraftChoice {
  id: string;
  label: string;
  screenplayTitle: string;
  draftName: string;
  status: string;
  revision: string | null;
  sceneCount: number;
}

function draftChoices(src: InterchangeSources): DraftChoice[] {
  const many = src.screenplays.length > 1;
  return src.screenplays.flatMap((s) =>
    s.drafts.map((d) => ({
      id: d.id,
      label: `${draftLabel(s.title, d.name, many)}${d.isCurrent ? " (current)" : ""}`,
      screenplayTitle: s.title,
      draftName: d.name,
      status: d.status,
      revision: [d.revisionLabel, d.revisionColor && `${d.revisionColor} Revision`].filter(Boolean).join(" · ") || null,
      sceneCount: d.sceneCount,
    })),
  );
}

function defaultDraft(src: InterchangeSources, wanted?: string): string | null {
  const all = src.screenplays.flatMap((s) => s.drafts.map((d) => d.id));
  if (wanted && all.includes(wanted)) return wanted;
  const first = src.screenplays.find((s) => s.drafts.length > 0);
  return first?.currentDraftId && all.includes(first.currentDraftId) ? first.currentDraftId : (first?.drafts[0]?.id ?? null);
}

export function ExportScreenplayDialog({ open, onClose, draftId }: ExportScreenplayDialogProps) {
  return open ? <ExportFlow onClose={onClose} draftId={draftId} /> : null;
}

function ExportFlow({ onClose, draftId }: { onClose: () => void; draftId?: string }) {
  const sources = useInterchangeSources(true);
  if (sources.isError) {
    return (
      <Dialog open onOpenChange={(v) => !v && onClose()} size="lg" title="Export Screenplay">
        <Banner tone="err">The screenplay list could not be loaded. {sources.error?.message}</Banner>
      </Dialog>
    );
  }
  if (!sources.data) {
    return (
      <Dialog open onOpenChange={(v) => !v && onClose()} size="lg" title="Export Screenplay">
        <Skeleton h={160} />
      </Dialog>
    );
  }
  const initial = defaultDraft(sources.data, draftId);
  if (!initial) {
    return (
      <Dialog open onOpenChange={(v) => !v && onClose()} size="md" title="Export Screenplay" footer={<Button variant="primary" onClick={onClose}>Close</Button>}>
        <EmptyState title="No screenplay to export yet">Write or import a screenplay first. Exports are made from a screenplay draft.</EmptyState>
      </Dialog>
    );
  }
  return <ExportForm sources={sources.data} initialDraft={initial} onClose={onClose} />;
}

type Scope = "all" | "selected";

function ExportForm({ sources, initialDraft, onClose }: { sources: InterchangeSources; initialDraft: string; onClose: () => void }) {
  const choices = useMemo(() => draftChoices(sources), [sources]);
  const [draft, setDraft] = useState(initialDraft);
  const [format, setFormat] = useState<InterchangeExportFormat>("pdf");
  const [showMore, setShowMore] = useState(false);
  const [scope, setScope] = useState<Scope>("all");
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [revisionMarks, setRevisionMarks] = useState(false);
  const [titlePage, setTitlePage] = useState(true);
  const [sceneNumbers, setSceneNumbers] = useState(true);
  const [revisionInfo, setRevisionInfo] = useState(true);
  const [includeNotes, setIncludeNotes] = useState(false);
  const [pageNumbers, setPageNumbers] = useState(true);
  const [view, setView] = useState<"options" | "preview">("options");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<InterchangeScreenplayExportResult | null>(null);

  const choice = choices.find((c) => c.id === draft) ?? choices[0];
  const scenes = useDraftScenes(draft);
  const pickable = scenes.data ?? [];

  const options: InterchangeScreenplayExportOptions = {
    titlePage,
    sceneNumbers,
    includeNotes,
    revisionMarks,
    revisionInfo,
    sceneIds: scope === "selected" ? [...picked] : null,
  };
  const scopeOk = scope === "all" || picked.size > 0;

  const changeDraft = (id: string) => {
    setDraft(id);
    setPicked(new Set());
    setError(null);
  };

  const runExport = async (fmt: InterchangeExportFormat) => {
    setError(null);
    let path: string | null;
    try {
      path = await saveDialog({
        title: "Export Screenplay",
        defaultPath: defaultExportFileName(choice.screenplayTitle, choice.draftName, fmt),
        filters: saveFilters(fmt),
      });
    } catch (e) {
      reportError(e);
      return;
    }
    if (!path) return;
    setBusy(true);
    try {
      const r = await interchange.exportDraft({ draftId: draft, format: fmt, path, options });
      setResult(r);
      toast.success(`Exported ${r.fileName}`);
    } catch (e) {
      if (e instanceof OpError) setError(e.message);
      else reportError(e);
    } finally {
      setBusy(false);
    }
  };

  if (result) {
    return (
      <Dialog
        open
        onOpenChange={(v) => !v && onClose()}
        size="md"
        title="Export complete"
        footer={
          <>
            <Button onClick={() => void revealLocation("exported", undefined, result.path).catch(reportError)}>Show in Folder</Button>
            <Button variant="primary" onClick={onClose}>Close</Button>
          </>
        }
      >
        <Banner tone="ok">
          <b>{result.fileName}</b> was saved.
        </Banner>
        <div className="card" style={{ marginTop: 10 }}>
          <table className="tbl">
            <tbody>
              <tr><td className="muted">Source</td><td>{result.sourceLabel}</td></tr>
              <tr><td className="muted">Format</td><td>{formatSpec(result.format as InterchangeExportFormat).label}</td></tr>
              <tr><td className="muted">Scenes</td><td>{result.sceneCount}</td></tr>
              {result.pages !== null && <tr><td className="muted">Pages</td><td>{result.pages}</td></tr>}
              <tr><td className="muted">Location</td><td className="xs" style={{ wordBreak: "break-all" }}>{result.path}</td></tr>
            </tbody>
          </table>
        </div>
        {result.warnings.map((w, i) => (
          <div key={`${w.code}-${i}`} style={{ marginTop: 8 }}>
            <Banner tone={w.level === "attention" ? "warn" : "info"}>{w.message}</Banner>
          </div>
        ))}
        <div style={{ marginTop: 8 }}>
          <Banner tone="priv">
            <b>Private notes excluded.</b> Export creates a snapshot; your project does not change.
          </Banner>
        </div>
      </Dialog>
    );
  }

  if (view === "preview") {
    return (
      <PreviewView
        draft={draft}
        options={options}
        setTitlePage={setTitlePage}
        pageNumbers={pageNumbers}
        setPageNumbers={setPageNumbers}
        setRevisionInfo={setRevisionInfo}
        setIncludeNotes={setIncludeNotes}
        busy={busy}
        error={error}
        onBack={() => setView("options")}
        onClose={onClose}
        onExportPdf={() => {
          setFormat("pdf");
          void runExport("pdf");
        }}
      />
    );
  }

  const main = EXPORT_FORMATS.filter((f) => !f.additional);
  const more = EXPORT_FORMATS.filter((f) => f.additional);
  const formatsShown = showMore || more.some((f) => f.value === format) ? [...main, ...more] : main;

  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      size="lg"
      title="Export Screenplay"
      sub="What am I exporting, which version, in which format, and where will it be saved?"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button onClick={() => setView("preview")} disabled={!scopeOk}>Preview…</Button>
          <Button variant="primary" disabled={busy || !scopeOk} onClick={() => void runExport(format)}>
            {busy ? "Exporting…" : "Export…"}
          </Button>
        </>
      }
    >
      <div className="h4">Source</div>
      <div className="row wrap" style={{ marginBottom: 8 }}>
        {choices.length > 1 ? (
          <Select ariaLabel="Draft to export" value={draft} onChange={changeDraft} options={choices.map((c) => ({ value: c.id, label: c.label }))} />
        ) : (
          <b>{choice.label}</b>
        )}
        <Chip tone={statusTone(choice.status)}>{choice.status}</Chip>
        {choice.revision && <Chip tone="y">{choice.revision}</Chip>}
        <span className="xs muted">{choice.sceneCount} scene{choice.sceneCount === 1 ? "" : "s"}</span>
      </div>
      <div className="grid g2">
        <fieldset className="ix-fieldset">
          <legend className="h4">Format</legend>
          <div className="col" style={{ gap: 6 }}>
            {formatsShown.map((f) => (
              <label key={f.value} className="ix-radio">
                <input type="radio" name="ix-format" checked={format === f.value} onChange={() => setFormat(f.value)} />
                <b>{f.label}</b>
                {f.hint && <span className="xs muted">{f.hint}</span>}
              </label>
            ))}
          </div>
        </fieldset>
        <fieldset className="ix-fieldset">
          <legend className="h4">Scope</legend>
          <div className="col" style={{ gap: 6 }}>
            <label className="ix-radio">
              <input type="radio" name="ix-scope" checked={scope === "all"} onChange={() => setScope("all")} />
              <span>Entire draft</span>
            </label>
            <label className="ix-radio">
              <input type="radio" name="ix-scope" checked={scope === "selected"} onChange={() => setScope("selected")} />
              <span>Selected scenes</span>
            </label>
            {scope === "selected" && (
              <div className="ix-scene-pick" role="group" aria-label="Scenes to export">
                {scenes.isLoading && <Skeleton h={60} />}
                {pickable.map((s) => (
                  <Checkbox
                    key={s.id}
                    checked={picked.has(s.id)}
                    onChange={(v) =>
                      setPicked((prev) => {
                        const next = new Set(prev);
                        if (v) next.add(s.id);
                        else next.delete(s.id);
                        return next;
                      })
                    }
                    label={`${s.number}. ${s.omitted ? "OMITTED" : s.heading}`}
                  />
                ))}
                {!scenes.isLoading && pickable.length === 0 && <span className="xs muted">This draft has no scenes yet.</span>}
              </div>
            )}
            <div className="hr" style={{ margin: "4px 0" }} />
            <label className="ix-radio">
              <input type="radio" name="ix-marks" checked={!revisionMarks} onChange={() => setRevisionMarks(false)} />
              <span>Clean script</span>
            </label>
            <label className="ix-radio">
              <input type="radio" name="ix-marks" checked={revisionMarks} onChange={() => setRevisionMarks(true)} />
              <span>Revision-marked version</span>
            </label>
          </div>
        </fieldset>
      </div>
      <div className="hr" />
      <div className="row wrap" style={{ gap: 14 }}>
        <Checkbox checked={titlePage} onChange={setTitlePage} label="Title page" />
        <Checkbox checked={sceneNumbers} onChange={setSceneNumbers} label="Scene numbers" />
        <Checkbox checked={revisionInfo} onChange={setRevisionInfo} label="Revision info" disabled={!titlePage} />
        <Checkbox checked={includeNotes} onChange={setIncludeNotes} label="Include notes" />
      </div>
      {scope === "selected" && sceneNumbers && <div className="hint">Selected scenes keep their numbers from the full draft.</div>}
      {error && (
        <div style={{ marginTop: 8 }}>
          <Banner tone="err">{error}</Banner>
        </div>
      )}
      <div style={{ marginTop: 8 }}>
        <Banner tone="priv">
          <b>Private notes excluded.</b> Export creates a snapshot; your project does not change.
        </Banner>
      </div>
      {more.length > 0 && !formatsShown.some((f) => f.additional) && (
        <button type="button" className="ix-more" onClick={() => setShowMore(true)}>
          Additional formats…
        </button>
      )}
    </Dialog>
  );
}

function PreviewView({
  draft,
  options,
  setTitlePage,
  pageNumbers,
  setPageNumbers,
  setRevisionInfo,
  setIncludeNotes,
  busy,
  error,
  onBack,
  onClose,
  onExportPdf,
}: {
  draft: string;
  options: InterchangeScreenplayExportOptions;
  setTitlePage: (v: boolean) => void;
  pageNumbers: boolean;
  setPageNumbers: (v: boolean) => void;
  setRevisionInfo: (v: boolean) => void;
  setIncludeNotes: (v: boolean) => void;
  busy: boolean;
  error: string | null;
  onBack: () => void;
  onClose: () => void;
  onExportPdf: () => void;
}) {
  const preview = usePrintPreview(draft, options, pageNumbers, true);
  const [page, setPage] = useState(0);
  const [zoom, setZoom] = useState(1);
  const [printing, setPrinting] = useState(false);
  const donePrinting = useCallback(() => setPrinting(false), []);
  const data = preview.data;
  const total = data?.pages.length ?? 0;
  const current = Math.min(page, Math.max(0, total - 1));
  const zoomIndex = ZOOMS.findIndex((z) => z.scale === zoom);
  const blocking = data?.warnings.some((w) => w.code === "pdf_unsupported_characters") ?? false;

  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      size="xl"
      title="Print Preview"
      sub={data?.sourceLabel}
      footerLeft={<Button icon={<ChevronLeft size={14} />} onClick={onBack}>Back to Export Options</Button>}
      footer={<Button onClick={onClose}>Close</Button>}
    >
      <div className="ix-preview">
        <div className="ix-preview-pane" aria-live="polite">
          {data ? (
            total > 0 ? <ScaledPage preview={data} index={current} scale={zoom * 0.62} /> : <span className="muted">Nothing to show.</span>
          ) : preview.isError ? (
            <Banner tone="err">The preview could not be prepared. {preview.error?.message}</Banner>
          ) : (
            <Skeleton h={400} w={320} />
          )}
        </div>
        <div className="ix-preview-side">
          <div className="h4">Preview settings</div>
          <div className="col" style={{ gap: 6 }}>
            <Checkbox checked={options.titlePage} onChange={(v) => { setTitlePage(v); setPage(0); }} label="Title page" />
            <Checkbox checked={pageNumbers} onChange={setPageNumbers} label="Page numbers" />
            <Checkbox checked={options.revisionInfo} onChange={setRevisionInfo} label="Revision info" disabled={!options.titlePage} />
            <Checkbox checked={options.includeNotes} onChange={setIncludeNotes} label="Include notes" />
          </div>
          <div className="hr" />
          <div className="row">
            <IconButton label="Previous page" disabled={current <= 0} onClick={() => setPage(current - 1)}>
              <ChevronLeft size={15} />
            </IconButton>
            <Chip>{data ? `${pageTitle(data, current)} · ${current + 1} of ${total}` : "…"}</Chip>
            <IconButton label="Next page" disabled={current >= total - 1} onClick={() => setPage(current + 1)}>
              <ChevronRight size={15} />
            </IconButton>
          </div>
          <div className="row" style={{ marginTop: 6 }}>
            <IconButton label="Zoom out" disabled={zoomIndex <= 0} onClick={() => setZoom(ZOOMS[Math.max(0, zoomIndex - 1)].scale)}>
              <Minus size={15} />
            </IconButton>
            <Chip>Zoom {ZOOMS[zoomIndex]?.label ?? "100%"}</Chip>
            <IconButton label="Zoom in" disabled={zoomIndex >= ZOOMS.length - 1} onClick={() => setZoom(ZOOMS[Math.min(ZOOMS.length - 1, zoomIndex + 1)].scale)}>
              <Plus size={15} />
            </IconButton>
          </div>
          <div className="hr" />
          {data?.warnings.map((w, i) => (
            <div key={`${w.code}-${i}`} style={{ marginBottom: 6 }}>
              <Banner tone="warn">{w.message}</Banner>
            </div>
          ))}
          {error && (
            <div style={{ marginBottom: 6 }}>
              <Banner tone="err">{error}</Banner>
            </div>
          )}
          <div className="col" style={{ gap: 6 }}>
            <Button disabled={!data || total === 0 || printing} onClick={() => setPrinting(true)}>Print</Button>
            <Button variant="primary" disabled={!data || busy || blocking} onClick={onExportPdf}>
              {busy ? "Exporting…" : "Export PDF"}
            </Button>
          </div>
          <div className="hint" style={{ marginTop: 8 }}>
            Changing these settings affects only the outgoing document — never the screenplay itself.
          </div>
        </div>
      </div>
      {printing && data && <PrintJob preview={data} onDone={donePrinting} />}
    </Dialog>
  );
}
