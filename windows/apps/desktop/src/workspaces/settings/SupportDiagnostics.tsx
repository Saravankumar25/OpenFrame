// Settings > Project settings > Support: "Export Diagnostics…" (build plan item export diagnostics, CORE-015,
// ADR-0021, docs/engineering/26-observability.md §3). The UX specification and mock-ups have
// no support or help location, so the action lives here under a "Support" heading.
// Lifecycle: open → Rust collects the bundle (preview) → the user reads every file → Save
// dialog → Rust writes the ZIP → result with "Reveal in File Manager". Nothing is sent.

import { useEffect, useId, useState } from "react";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import { FolderSearch, LifeBuoy } from "lucide-react";
import { Banner, Button, Dialog, Skeleton } from "../../design-system";
import { call, OpError, revealLocation } from "../../ipc/client";
import { reportError } from "../../ipc/query";
import { toast } from "../../app/toast";
import type { DiagnosticsExportResult } from "../../ipc/generated/DiagnosticsExportResult";
import type { DiagnosticsFile } from "../../ipc/generated/DiagnosticsFile";
import type { DiagnosticsPreview } from "../../ipc/generated/DiagnosticsPreview";

const TITLE = "Export Diagnostics";

export function SupportSection() {
  const [open, setOpen] = useState(false);
  return (
    <section className="card pad" style={{ maxWidth: 720, marginTop: 16 }} aria-labelledby="support-heading">
      <h2 id="support-heading" className="h4">Support</h2>
      <p className="sm muted" style={{ margin: "0 0 10px", lineHeight: 1.5 }}>
        If you contact support, you can attach a diagnostics bundle. You can read everything in it before you save it.
        Nothing is sent automatically.
      </p>
      <Button icon={<LifeBuoy size={14} />} onClick={() => setOpen(true)}>
        Export Diagnostics…
      </Button>
      {open && <ExportDiagnosticsDialog onClose={() => setOpen(false)} />}
    </section>
  );
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} bytes`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function ExportDiagnosticsDialog({ onClose }: { onClose: () => void }) {
  const [preview, setPreview] = useState<DiagnosticsPreview | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<DiagnosticsExportResult | null>(null);

  useEffect(() => {
    let live = true;
    call<DiagnosticsPreview>("diagnostics.preview", {})
      .then((p) => live && setPreview(p))
      .catch((e: unknown) => {
        if (!live) return;
        if (e instanceof OpError) setLoadError(e.message);
        else {
          reportError(e);
          setLoadError("The diagnostics could not be collected.");
        }
      });
    return () => {
      live = false;
    };
  }, []);

  const run = async () => {
    if (!preview) return;
    setError(null);
    let path: string | null;
    try {
      path = await saveDialog({
        title: TITLE,
        defaultPath: preview.suggestedFileName,
        filters: [{ name: "ZIP archive", extensions: ["zip"] }],
      });
    } catch (e) {
      reportError(e);
      return;
    }
    if (!path) return;
    setBusy(true);
    try {
      const r = await call<DiagnosticsExportResult>("diagnostics.export_bundle", { path });
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
            <Button
              icon={<FolderSearch size={14} />}
              onClick={() => void revealLocation("exported", undefined, result.path).catch(reportError)}
            >
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
              <tr><td className="muted">Files</td><td>{result.files.length}</td></tr>
              <tr><td className="muted">Size</td><td>{formatBytes(result.bytesWritten)}</td></tr>
              <tr><td className="muted">Location</td><td className="xs" style={{ wordBreak: "break-all" }}>{result.path}</td></tr>
            </tbody>
          </table>
        </div>
        <div style={{ marginTop: 8 }}>
          <Banner tone="priv">
            <b>Nothing was sent.</b> Review the file, then attach it to your support request if you want to.
          </Banner>
        </div>
      </Dialog>
    );
  }

  return (
    <Dialog
      open
      onOpenChange={(v) => !v && !busy && onClose()}
      size="lg"
      title={TITLE}
      sub="Review what the bundle contains before you save it."
      dismissible={!busy}
      footer={
        <>
          <Button onClick={onClose} disabled={busy}>Cancel</Button>
          <Button variant="primary" disabled={busy || !preview} onClick={() => void run()}>
            {busy ? "Exporting…" : "Export…"}
          </Button>
        </>
      }
    >
      <Banner tone="priv">
        <b>Nothing is sent automatically.</b> The bundle is saved on this computer, and you decide whether to share it.
      </Banner>
      {loadError && (
        <div style={{ marginTop: 10 }}>
          <Banner tone="err">{loadError}</Banner>
        </div>
      )}
      {error && (
        <div style={{ marginTop: 10 }}>
          <Banner tone="err">{error}</Banner>
        </div>
      )}
      {!preview && !loadError && (
        <div role="status" aria-label="Collecting diagnostics" style={{ marginTop: 12 }}>
          <Skeleton h={14} />
          <div style={{ height: 8 }} />
          <Skeleton h={14} w="70%" />
        </div>
      )}
      {preview && (
        <>
          <h3 className="h4" style={{ marginTop: 14 }}>Files in the bundle</h3>
          <ul aria-label="Files in the bundle" style={{ listStyle: "none", margin: 0, padding: 0, display: "grid", gap: 8 }}>
            {preview.files.map((f) => (
              <BundleFileRow key={f.name} file={f} />
            ))}
          </ul>
          <h3 className="h4" style={{ marginTop: 14 }}>Never included</h3>
          <ul className="sm" style={{ margin: 0, paddingLeft: 18, lineHeight: 1.6 }}>
            {preview.neverIncluded.map((n) => (
              <li key={n}>{n}</li>
            ))}
          </ul>
        </>
      )}
    </Dialog>
  );
}

function BundleFileRow({ file }: { file: DiagnosticsFile }) {
  const [shown, setShown] = useState(false);
  const id = useId();
  return (
    <li className="card" style={{ padding: "8px 10px" }}>
      <div className="row" style={{ alignItems: "flex-start", gap: 10 }}>
        <div className="grow">
          <div className="sm b">{file.name}</div>
          <div className="xs muted" style={{ lineHeight: 1.5 }}>{file.description}</div>
        </div>
        <span className="xs muted" style={{ whiteSpace: "nowrap" }}>{formatBytes(file.bytes)}</span>
        <Button
          size="sm"
          aria-expanded={shown}
          aria-controls={id}
          aria-label={`${shown ? "Hide" : "Show"} contents of ${file.name}`}
          onClick={() => setShown((v) => !v)}
        >
          {shown ? "Hide contents" : "Show contents"}
        </Button>
      </div>
      {shown && (
        <pre
          id={id}
          tabIndex={0}
          aria-label={`Contents of ${file.name}`}
          className="xs selectable-text"
          style={{
            margin: "8px 0 0",
            maxHeight: 240,
            overflow: "auto",
            padding: 8,
            border: "1px solid var(--line)",
            borderRadius: 6,
            background: "var(--bg)",
            fontFamily: "Consolas, \"Cascadia Mono\", monospace",
            whiteSpace: "pre-wrap",
            wordBreak: "break-word",
          }}
        >
          {file.content}
        </pre>
      )}
    </li>
  );
}
