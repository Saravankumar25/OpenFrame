// Import Screenplay (FSD §19, §60, FSD-SCRIPT-038..044; Import/Export §4;
// mocks 091 source → 092 preview → 094 report, 093 failure).
// Nothing is created until the user confirms the preview; an existing
// screenplay is never overwritten (new screenplay or new named draft only).

import { useEffect, useRef, useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { FileText, FolderOpen } from "lucide-react";
import { Banner, Button, Checkbox, Chip, Dialog, Field, Segmented, Select, TextArea, TextInput } from "../../design-system";
import { OpError } from "../../ipc/client";
import { reportError } from "../../ipc/query";
import { toast } from "../../app/toast";
import { useNav } from "../../app/stores";
import type { InterchangeImportPreview } from "../../ipc/generated/InterchangeImportPreview";
import type { InterchangeImportReport } from "../../ipc/generated/InterchangeImportReport";
import type { InterchangeWarning } from "../../ipc/generated/InterchangeWarning";
import { interchange } from "./api";
import {
  IMPORT_FILTERS,
  type ImportMode,
  attentionFirst,
  canKeepAsProjectFile,
  confidenceLabel,
  failureStage,
  fileName,
  importModes,
  importReady,
  pagesLabel,
  previewBanner,
  sceneStatusChip,
  targetLabel,
} from "./model";
import "./interchange.css";

type Step = "source" | "preview" | "report" | "failed";
type Mode = ImportMode;

interface Failure {
  code: string;
  message: string;
  source: "file" | "pasted";
  path: string | null;
}

export interface ImportScreenplayDialogProps {
  open: boolean;
  onClose: () => void;
  /** Called after a successful import (e.g. to select the new draft). */
  onImported?: (report: InterchangeImportReport) => void;
}

const STEPS: readonly [Step[], string][] = [
  [["source", "failed"], "1  Select source"],
  [["preview"], "2  Preview interpretation"],
  [["report"], "3  Import"],
];

function Steps({ step }: { step: Step }) {
  return (
    <ol className="row ix-steps" aria-label="Import steps">
      {STEPS.map(([steps, label]) => (
        <li key={label} aria-current={steps.includes(step) ? "step" : undefined}>
          <Chip tone={steps.includes(step) ? "dark" : "default"}>{label}</Chip>
        </li>
      ))}
    </ol>
  );
}

export function ImportScreenplayDialog({ open, onClose, onImported }: ImportScreenplayDialogProps) {
  // Remount per opening so every import starts from a clean state.
  return open ? <ImportFlow onClose={onClose} onImported={onImported} /> : null;
}

function ImportFlow({ onClose, onImported }: { onClose: () => void; onImported?: (r: InterchangeImportReport) => void }) {
  const go = useNav((s) => s.go);
  const [step, setStep] = useState<Step>("source");
  const [path, setPath] = useState<string | null>(null);
  const [pasted, setPasted] = useState("");
  const [busy, setBusy] = useState(false);
  const [preview, setPreview] = useState<InterchangeImportPreview | null>(null);
  const [report, setReport] = useState<InterchangeImportReport | null>(null);
  const [failure, setFailure] = useState<Failure | null>(null);
  const [mode, setMode] = useState<Mode>("new_screenplay");
  const [screenplayId, setScreenplayId] = useState("");
  const [episodeId, setEpisodeId] = useState("");
  const [draftName, setDraftName] = useState("");
  const [keepSource, setKeepSource] = useState(true);
  const [showWarnings, setShowWarnings] = useState(false);
  const pasteRef = useRef<HTMLTextAreaElement>(null);

  const source: "file" | "pasted" = path ? "file" : "pasted";
  const canPreview = !busy && (!!path || pasted.trim().length > 0);

  const browse = async () => {
    try {
      const chosen = await openDialog({ multiple: false, directory: false, filters: IMPORT_FILTERS, title: "Choose a screenplay to import" });
      if (typeof chosen === "string") {
        setPath(chosen);
        setPasted("");
      }
    } catch (e) {
      reportError(e);
    }
  };

  const runPreview = async () => {
    setBusy(true);
    try {
      const p = await interchange.preview({ path, pastedText: path ? null : pasted, format: null });
      setPreview(p);
      setMode(p.defaultMode === "new_draft" ? "new_draft" : "new_screenplay");
      setScreenplayId(p.existingScreenplays[0]?.id ?? "");
      setEpisodeId(p.episodesWithoutScreenplay[0]?.id ?? "");
      setShowWarnings(false);
      setStep("preview");
    } catch (e) {
      if (e instanceof OpError) {
        setFailure({ code: e.code, message: e.message, source, path });
        setStep("failed");
      } else {
        reportError(e);
      }
    } finally {
      setBusy(false);
    }
  };

  const runImport = async () => {
    if (!preview) return;
    setBusy(true);
    try {
      const r = await interchange.apply({
        previewId: preview.previewId,
        path: null,
        pastedText: null,
        format: null,
        mode,
        screenplayId: mode === "new_draft" ? screenplayId || null : null,
        episodeId: mode === "new_screenplay" && preview.episodic ? episodeId || null : null,
        draftName: draftName.trim() || null,
        keepSourceFile: keepSource,
      });
      setReport(r);
      setShowWarnings(false);
      setStep("report");
      toast.undoable(mode === "new_screenplay" ? `Imported screenplay “${r.screenplayTitle}”` : `Imported ${r.draftLabel}`);
      onImported?.(r);
    } catch (e) {
      if (e instanceof OpError && e.code === "import.preview_expired") {
        setFailure({ code: e.code, message: e.message, source, path });
        setStep("failed");
      } else {
        reportError(e);
      }
    } finally {
      setBusy(false);
    }
  };

  const keepAsFile = async () => {
    if (!failure?.path) return;
    setBusy(true);
    try {
      await interchange.addToProjectFiles(failure.path);
      toast.undoable(`Added “${fileName(failure.path)}” to Project Files`);
      onClose();
    } catch (e) {
      reportError(e);
    } finally {
      setBusy(false);
    }
  };

  const pasteInstead = () => {
    setPath(null);
    setFailure(null);
    setStep("source");
  };

  useEffect(() => {
    if (step === "source" && !path) pasteRef.current?.focus();
  }, [step, path]);

  if (step === "failed" && failure) {
    return (
      <Dialog
        open
        onOpenChange={(v) => !v && onClose()}
        size="md"
        title="Import failed. Your current project was not changed."
        footer={
          <>
            <Button onClick={pasteInstead}>Paste Text Instead</Button>
            {failure.path && canKeepAsProjectFile(failure.code) && (
              <Button disabled={busy} onClick={() => void keepAsFile()}>Add to Project Files</Button>
            )}
            <Button variant="primary" onClick={onClose}>Close</Button>
          </>
        }
      >
        <div className="card pad">
          <table className="tbl">
            <tbody>
              <tr><td className="muted">{failure.path ? "File" : "Source"}</td><td><b>{failure.path ? fileName(failure.path) : "Pasted text"}</b></td></tr>
              <tr><td className="muted">Stage</td><td>{failureStage(failure.code, failure.source)}</td></tr>
              <tr><td className="muted">Problem</td><td>{failure.message.replace(/\s*Your (current )?project was not changed\.?$/, "")}</td></tr>
              <tr><td className="muted">Your project</td><td><Chip tone="g">Unchanged</Chip></td></tr>
              {failure.path && <tr><td className="muted">Source file</td><td><Chip tone="g">Untouched</Chip></td></tr>}
            </tbody>
          </table>
        </div>
        <p style={{ margin: "10px 0 0" }}>
          {failure.path && canKeepAsProjectFile(failure.code)
            ? "You can still import it as a project document, or paste the screenplay text into a new draft."
            : "You can paste the screenplay text instead, or choose another file."}
        </p>
      </Dialog>
    );
  }

  if (step === "report" && report) {
    return (
      <Dialog
        open
        onOpenChange={(v) => !v && onClose()}
        size="md"
        title="Import complete"
        footer={
          <>
            {report.warnings.length > 0 && (
              <Button onClick={() => setShowWarnings((v) => !v)} aria-expanded={showWarnings}>
                {showWarnings ? "Hide Warnings" : "Review Warnings"}
              </Button>
            )}
            <Button
              variant="primary"
              onClick={() => {
                go({ workspace: "screenplay", params: { screenplayId: report.screenplayId, draftId: report.draftId } });
                onClose();
              }}
            >
              Open Screenplay
            </Button>
          </>
        }
      >
        <Banner tone="ok">
          <b>
            {report.mode === "new_screenplay"
              ? `Imported as a new screenplay, “${report.screenplayTitle}” (${report.draftLabel}).`
              : `Imported as ${report.draftLabel}.`}
          </b>{" "}
          {report.mode === "new_draft" ? "Your other drafts were not changed." : "Nothing else in your project was changed."}
        </Banner>
        <div className="card" style={{ marginTop: 10 }}>
          <table className="tbl">
            <tbody>
              <tr><td className="muted">Source file</td><td>{report.sourceName}</td></tr>
              <tr><td className="muted">Imported scenes</td><td>{report.importedScenes}</td></tr>
              <tr><td className="muted">Detected characters</td><td>{report.detectedCharacters}</td></tr>
              <tr><td className="muted">Pages / length</td><td>{pagesLabel(report.approxPages)} pages</td></tr>
              <tr>
                <td className="muted">Warnings needing attention</td>
                <td><Chip tone={report.warningsNeedingAttention > 0 ? "y" : "g"}>{report.warningsNeedingAttention}</Chip></td>
              </tr>
              {report.keptFileId && <tr><td className="muted">Original file</td><td>Kept in Project Files</td></tr>}
            </tbody>
          </table>
        </div>
        {showWarnings && <WarningList warnings={report.warnings} />}
      </Dialog>
    );
  }

  if (step === "preview" && preview) {
    const banner = previewBanner(preview);
    const conf = confidenceLabel(preview.confidence);
    const existing = preview.existingScreenplays;
    const canImport = !busy && importReady(preview, mode, screenplayId, episodeId);
    return (
      <Dialog
        open
        onOpenChange={(v) => !v && onClose()}
        size="lg"
        title="Import Screenplay · Preview"
        sub="Check how the script was interpreted before anything is created."
        footer={
          <>
            <Button onClick={() => setStep("source")} disabled={busy}>Back</Button>
            <Button onClick={onClose}>Cancel</Button>
            <Button variant="primary" disabled={!canImport} onClick={() => void runImport()}>
              {busy ? "Importing…" : "Import"}
            </Button>
          </>
        }
      >
        <Steps step="preview" />
        <div className="grid ix-facts">
          <div className="card pad"><div className="xs muted">Source</div><b title={preview.sourceName}>{preview.sourceName}</b><div className="xs muted">{preview.formatLabel}</div></div>
          <div className="card pad"><div className="xs muted">Scenes</div><b>{preview.sceneCount}</b></div>
          <div className="card pad"><div className="xs muted">Characters</div><b>{preview.characterCount}</b></div>
          <div className="card pad"><div className="xs muted">Pages</div><b>{pagesLabel(preview.approxPages)}</b></div>
        </div>
        <div className="row wrap" style={{ margin: "0 0 8px", gap: 6 }}>
          <Chip tone={conf.tone} title={`Parsing confidence ${Math.round(preview.confidenceScore * 100)}%`}>{conf.label}</Chip>
          {preview.title && <Chip tone="out">Title: {preview.title}</Chip>}
          {preview.author && <Chip tone="out">Written by {preview.author}</Chip>}
          <Chip tone="out">{preview.dialogueCount} dialogue blocks</Chip>
        </div>
        <Banner
          tone={banner.tone}
          actions={
            preview.warnings.length > 0 ? (
              <Button size="sm" onClick={() => setShowWarnings((v) => !v)} aria-expanded={showWarnings}>
                {showWarnings ? "Hide warnings" : "Show warnings"}
              </Button>
            ) : undefined
          }
        >
          <b>{banner.title}</b> {banner.text}
        </Banner>
        {showWarnings && <WarningList warnings={preview.warnings} />}
        <div className="card ix-scenes" style={{ marginTop: 8 }}>
          <table className="tbl">
            <thead>
              <tr><th scope="col">#</th><th scope="col">Detected scene heading</th><th scope="col">Status</th></tr>
            </thead>
            <tbody>
              {preview.scenes.map((s, i) => {
                const chip = sceneStatusChip(s.status);
                const number = preview.scenes.slice(0, i + 1).filter((x) => x.heading.trim()).length;
                return (
                  <tr key={s.index}>
                    <td>{s.heading.trim() ? number : "—"}</td>
                    <td>{s.heading.trim() || <span className="muted">(material before the first scene)</span>}{s.sourceNumber && <span className="xs muted"> · numbered {s.sourceNumber} in the source</span>}</td>
                    <td><Chip tone={chip.tone}>{chip.label}</Chip></td>
                  </tr>
                );
              })}
              {preview.scenes.length === 0 && (
                <tr><td colSpan={3} className="muted">No scenes were detected.</td></tr>
              )}
            </tbody>
          </table>
        </div>
        <div className="row wrap" style={{ marginTop: 10, gap: 10 }}>
          <span className="h4" style={{ margin: 0 }}>Import as</span>
          <Segmented<Mode>
            ariaLabel="Import as"
            value={mode}
            onChange={setMode}
            options={importModes(preview)}
          />
          {mode === "new_draft" && existing.length > 1 && (
            <Select
              ariaLabel="Screenplay that receives the new draft"
              value={screenplayId}
              onChange={setScreenplayId}
              options={existing.map((s) => ({ value: s.id, label: targetLabel(s) }))}
            />
          )}
          {mode === "new_screenplay" && preview.episodic && (
            <Select
              ariaLabel="Episode for the new screenplay"
              value={episodeId}
              onChange={setEpisodeId}
              options={preview.episodesWithoutScreenplay.map((e) => ({ value: e.id, label: e.title }))}
            />
          )}
        </div>
        {!preview.canCreateScreenplay && (
          <div className="hint">
            {preview.episodic ? "Every episode already has a screenplay" : "This project already has a screenplay"}, so the script arrives as a new draft. The current draft is not changed.
          </div>
        )}
        <div className="grid g2" style={{ marginTop: 8 }}>
          <Field label="Draft name" hint={mode === "new_draft" ? "Leave empty to name it after the file. Existing drafts are not changed." : "Leave empty to name it after the file."}>
            <TextInput value={draftName} maxLength={120} placeholder={`${fileName(preview.sourceName).replace(/\.[^.]+$/, "")} (imported)`} onChange={(e) => setDraftName(e.target.value)} />
          </Field>
          <div style={{ alignSelf: "center" }}>
            <Checkbox
              checked={keepSource}
              onChange={setKeepSource}
              label={preview.sourceFormat === "pasted" ? "Keep the pasted text in Project Files" : "Keep the original file in Project Files"}
            />
          </div>
        </div>
      </Dialog>
    );
  }

  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      size="lg"
      title="Import Screenplay"
      sub="What existing script do you want to bring into OpenFrame?"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!canPreview} onClick={() => void runPreview()}>
            {busy ? "Reading…" : "Preview"}
          </Button>
        </>
      }
    >
      <Steps step="source" />
      <div className="grid g2">
        <div className="dz" style={{ minHeight: 170 }}>
          {path ? <FileText size={26} aria-hidden /> : <FolderOpen size={26} aria-hidden />}
          <b style={{ marginTop: 8 }}>{path ? fileName(path) : "Choose a file"}</b>
          <div className="xs muted">PDF · Final Draft (.fdx) · Fountain · TXT · DOCX</div>
          <div className="row" style={{ marginTop: 8, gap: 6 }}>
            <Button size="sm" onClick={() => void browse()}>Browse…</Button>
            {path && <Button size="sm" variant="ghost" onClick={() => setPath(null)}>Clear</Button>}
          </div>
        </div>
        <div>
          <label className="h4" htmlFor="ix-paste">…or paste screenplay text</label>
          <TextArea
            id="ix-paste"
            ref={pasteRef}
            className="ix-paste"
            value={pasted}
            disabled={!!path}
            spellCheck={false}
            placeholder={"INT. POLICE STATION — NIGHT\n\nArjun enters carrying a pistol.\n\n            MEERA\nYou said you would never come back."}
            onChange={(e) => setPasted(e.target.value)}
          />
        </div>
      </div>
      <div style={{ marginTop: 10 }}>
        <Banner tone="info">Importing never replaces your current screenplay. If one exists, the script arrives as a new screenplay or a new draft.</Banner>
      </div>
    </Dialog>
  );
}

function WarningList({ warnings }: { warnings: readonly InterchangeWarning[] }) {
  return (
    <ul className="ix-warnings" aria-label="Import warnings">
      {attentionFirst(warnings).map((w, i) => (
        <li key={`${w.code}-${i}`}>
          <Chip tone={w.level === "attention" ? "y" : "default"}>{w.level === "attention" ? "Needs attention" : "Info"}</Chip>
          <span>{w.message}</span>
        </li>
      ))}
    </ul>
  );
}
