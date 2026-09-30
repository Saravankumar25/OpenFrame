// Reusable document Export dialog (FSD §20.7, §60, §122–123; Import/Export §3;
// UX §3.19/§3.26–3.34; mocks 072, 122, 139, 147–149).
// Lifecycle: source → scope → format/options → Save dialog → export → result.
// The export is a snapshot written by Rust to the chosen path; the project is
// never changed and private notes are never included.

import { useMemo, useState, type ReactNode } from "react";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import { FolderSearch } from "lucide-react";
import { Banner, Button, Checkbox, Dialog, Select } from "../../design-system";
import { call, OpError, revealLocation } from "../../ipc/client";
import { reportError, useOp } from "../../ipc/query";
import { toast } from "../../app/toast";
import type { ExportResult } from "../../ipc/generated/ExportResult";
import type { ProjectSummary } from "../../ipc/generated/ProjectSummary";
import {
  FORMAT_LABELS,
  buildExportArgs,
  exportFileName,
  fieldApplies,
  initialOptions,
  saveFilters,
  type ExportFormatOption,
  type ExportFormatValue,
  type ExportOptionField,
  type ExportScope,
  type OptionValues,
} from "./model";
import "./export.css";

export interface ExportDialogProps {
  open: boolean;
  onClose: () => void;
  /** "Export Shooting Schedule". */
  title: string;
  sub?: string;
  /** What is being exported, shown before confirming ("Source: …"). */
  source?: ReactNode;
  /** Document name used in the suggested file name. */
  documentName: string;
  /** Rust operation, e.g. `schedule.export_schedule`. */
  op: string;
  /** Arguments every request carries (ids of the current item…). */
  baseArgs?: Record<string, unknown>;
  formats: readonly ExportFormatOption[];
  scopes?: readonly ExportScope[];
  options?: readonly ExportOptionField[];
  /** Replaces the default privacy/snapshot note. */
  note?: ReactNode;
  /** When set, exporting is not possible and this explains why. */
  unavailable?: string | null;
}

export function ExportDialog(props: ExportDialogProps) {
  return props.open ? <ExportFlow {...props} /> : null;
}

const DEFAULT_NOTE = "Private notes excluded. Export creates a snapshot and does not change your project.";

function ExportFlow({ onClose, title, sub, source, documentName, op, baseArgs, formats, scopes = [], options = [], note, unavailable }: ExportDialogProps) {
  const project = useOp<ProjectSummary | null>("project.current", {}, ["project"]);
  const [format, setFormat] = useState<ExportFormatValue>(formats[0]?.value ?? "pdf");
  const firstScope = scopes.find((s) => !s.disabled) ?? scopes[0];
  const [scopeValue, setScopeValue] = useState<string | undefined>(firstScope?.value);
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [values, setValues] = useState<OptionValues>(() => initialOptions(options));
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<ExportResult | null>(null);

  const scope = scopes.find((s) => s.value === scopeValue);
  const scopeOk = !scope?.picker || picked.size > 0;
  const shownFields = useMemo(() => options.filter((f) => fieldApplies(f, format)), [options, format]);

  const run = async () => {
    setError(null);
    let path: string | null;
    try {
      path = await saveDialog({
        title,
        defaultPath: exportFileName(project.data?.title, documentName, format),
        filters: saveFilters(format),
      });
    } catch (e) {
      reportError(e);
      return;
    }
    if (!path) return;
    setBusy(true);
    try {
      const args = buildExportArgs(baseArgs, format, scope, [...picked], options, values);
      const r = await call<ExportResult>(op, { ...args, path });
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
            <Button icon={<FolderSearch size={14} />} onClick={() => void revealLocation("exported", undefined, result.path).catch(reportError)}>
              Reveal in File Manager
            </Button>
            <Button variant="primary" onClick={onClose}>
              Close
            </Button>
          </>
        }
      >
        <Banner tone="ok">
          <b>{result.fileName}</b> was saved.
        </Banner>
        <div className="card" style={{ marginTop: 10 }}>
          <table className="tbl">
            <tbody>
              <tr><td className="muted">Document</td><td>{result.document}</td></tr>
              <tr><td className="muted">Source</td><td>{result.sourceLabel}</td></tr>
              <tr><td className="muted">Scope</td><td>{result.scopeLabel}</td></tr>
              {result.contentsLabel && <tr><td className="muted">Contents</td><td>{result.contentsLabel}</td></tr>}
              <tr><td className="muted">Format</td><td>{FORMAT_LABELS[result.format as ExportFormatValue] ?? result.format}</td></tr>
              {result.pages !== null && <tr><td className="muted">Pages</td><td>{result.pages}</td></tr>}
              <tr><td className="muted">Location</td><td className="xs exp-path">{result.path}</td></tr>
            </tbody>
          </table>
        </div>
        {result.warnings.map((w, i) => (
          <div key={i} style={{ marginTop: 8 }}>
            <Banner tone="warn">{w}</Banner>
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

  return (
    <Dialog
      open
      onOpenChange={(v) => !v && !busy && onClose()}
      size="md"
      title={title}
      sub={sub}
      dismissible={!busy}
      footer={
        <>
          <Button onClick={onClose} disabled={busy}>Cancel</Button>
          <Button variant="primary" disabled={busy || !scopeOk || !!unavailable} onClick={() => void run()}>
            {busy ? "Exporting…" : "Export…"}
          </Button>
        </>
      }
    >
      {source && <div className="exp-source">{source}</div>}
      {unavailable && (
        <div style={{ marginBottom: 8 }}>
          <Banner tone="info">{unavailable}</Banner>
        </div>
      )}
      {scopes.length > 0 && (
        <fieldset className="exp-fieldset">
          <legend className="h4">What are you exporting?</legend>
          <div className="col" style={{ gap: 6 }}>
            {scopes.map((s) => (
              <label key={s.value} className={`exp-radio${s.disabled ? " disabled" : ""}`}>
                <input
                  type="radio"
                  name="exp-scope"
                  checked={scopeValue === s.value}
                  disabled={s.disabled || busy}
                  onChange={() => {
                    setScopeValue(s.value);
                    setPicked(new Set());
                  }}
                />
                <span>{s.label}</span>
              </label>
            ))}
            {scope?.picker && (
              <div className="exp-pick" role="group" aria-label={scope.label}>
                {scope.picker.items.length === 0 && <span className="xs muted">{scope.picker.emptyText ?? "Nothing to choose from yet."}</span>}
                {scope.picker.items.map((it) => (
                  <Checkbox
                    key={it.id}
                    checked={picked.has(it.id)}
                    disabled={busy}
                    label={it.label}
                    onChange={(v) =>
                      setPicked((prev) => {
                        const next = new Set(prev);
                        if (v) next.add(it.id);
                        else next.delete(it.id);
                        return next;
                      })
                    }
                  />
                ))}
              </div>
            )}
          </div>
        </fieldset>
      )}
      <fieldset className="exp-fieldset">
        <legend className="h4">Format</legend>
        <div className="row wrap" style={{ gap: 14 }}>
          {formats.map((f) => (
            <label key={f.value} className="exp-radio">
              <input type="radio" name="exp-format" checked={format === f.value} disabled={busy} onChange={() => setFormat(f.value)} />
              <b>{f.label}</b>
              {f.hint && <span className="xs muted">{f.hint}</span>}
            </label>
          ))}
        </div>
      </fieldset>
      <div className="xs muted exp-name">
        Suggested file name: <span>{exportFileName(project.data?.title, documentName, format)}</span>
      </div>
      {shownFields.length > 0 && (
        <fieldset className="exp-fieldset">
          <legend className="h4">Options</legend>
          <div className="col" style={{ gap: 6 }}>
            {shownFields.map((f) =>
              f.kind === "checkbox" ? (
                <Checkbox
                  key={f.key}
                  checked={values[f.key] === true}
                  disabled={busy}
                  label={f.label}
                  onChange={(v) => setValues((prev) => ({ ...prev, [f.key]: v }))}
                />
              ) : (
                <label key={f.key} className="row" style={{ gap: 8 }}>
                  <span className="xs muted">{f.label}</span>
                  <Select
                    ariaLabel={f.label}
                    value={String(values[f.key])}
                    options={f.options}
                    onChange={(v) => setValues((prev) => ({ ...prev, [f.key]: v }))}
                  />
                </label>
              ),
            )}
          </div>
        </fieldset>
      )}
      {busy && (
        <div className="exp-progress" role="progressbar" aria-label="Exporting" aria-busy="true">
          <div className="exp-progress-bar" />
          <span className="xs muted">Creating the document… documents with images can take a moment.</span>
        </div>
      )}
      {error && (
        <div style={{ marginTop: 8 }}>
          <Banner tone="err">{error}</Banner>
        </div>
      )}
      <div style={{ marginTop: 10 }}>
        <Banner tone="priv">{note ?? DEFAULT_NOTE}</Banner>
      </div>
    </Dialog>
  );
}
