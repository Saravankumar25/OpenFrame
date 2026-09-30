// Open / Import Project package and Restore Backup (FSD §45.3–45.5, §44 restore;
// Import/Export §8.6: Open Package → Validate → Project Identity → Collision
// Choice → File/Reference Report → Import; mockup 171).

import { useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { Check, FileArchive } from "lucide-react";
import { reportError } from "../../ipc/query";
import type { PackageInspection } from "../../ipc/generated/PackageInspection";
import type { PackageOpenMode } from "../../ipc/generated/PackageOpenMode";
import type { PackageProjectImportReport } from "../../ipc/generated/PackageProjectImportReport";
import { Banner, Button, Chip, ConfirmDialog, Dialog, EmptyState } from "../../design-system";
import { dayAndTime } from "../../app/home/format";
import { guardedLeave } from "../../app/shell/SaveStatus";
import { useNav } from "../../app/stores";
import { PROJECT_OPEN_FILTER, formatBytes, packagesApi } from "./api";
import { DoneBanner, LinkedFiles, TaskProgressBar } from "./parts";
import { usePackageTask } from "./BackupDialogs";

export function OpenProjectDialog({ onClose, currentTitle }: { onClose: () => void; currentTitle: string | null }) {
  const [info, setInfo] = useState<PackageInspection | null>(null);
  const [path, setPath] = useState<string | null>(null);
  const [mode, setMode] = useState<PackageOpenMode>("copy");
  const [confirmReplace, setConfirmReplace] = useState(false);
  const [checking, setChecking] = useState(false);
  const { task, taskId, setTaskId, running } = usePackageTask();
  const report = task?.state === "completed" ? (task.result as unknown as PackageProjectImportReport) : null;

  const choose = async () => {
    const chosen = await openDialog({ title: "Open Project Package", multiple: false, filters: PROJECT_OPEN_FILTER });
    if (typeof chosen !== "string") return;
    setChecking(true);
    setInfo(null);
    try {
      const r = await packagesApi.inspect(chosen);
      setPath(chosen);
      setInfo(r);
      setMode(r.defaultMode ?? "copy");
    } catch (e) {
      reportError(e);
    } finally {
      setChecking(false);
    }
  };

  const start = (m: PackageOpenMode) => {
    if (!path) return;
    // Opening another project closes this one; the save guard runs first.
    void guardedLeave(currentTitle ?? "this project", async () => {
      const r = await packagesApi.importProject(path, m);
      setTaskId(r.taskId);
      useNav.getState().reset();
    });
  };

  const isBackup = info?.manifest.packageType === "backup";
  const title = isBackup ? "Restore Backup" : "Open Project Package";
  const collision = info?.collision ?? null;
  const missing = info?.external.filter((f) => f.status === "Missing") ?? [];

  return (
    <Dialog
      open
      onOpenChange={(v) => !v && !running && onClose()}
      title={collision && !taskId ? "This project already exists on this computer" : title}
      sub={!info ? "Open a Full Project Package or restore a Backup Package from another disk or PC." : undefined}
      size="md"
      dismissible={!running}
      footer={
        report || task?.state === "failed" || task?.state === "cancelled" ? (
          <Button variant="primary" onClick={onClose}>Close</Button>
        ) : (
          <>
            <Button onClick={onClose} disabled={running}>Cancel</Button>
            {info && !taskId && (
              <Button
                variant="primary"
                onClick={() => (mode === "replace" ? setConfirmReplace(true) : start(mode))}
              >
                {mode === "replace" ? "Replace Existing After Backup" : mode === "copy" ? "Open as Copy" : isBackup ? "Restore" : "Open Project"}
              </Button>
            )}
          </>
        )
      }
    >
      {report ? (
        <DoneBanner title={report.identityKept ? "Project opened." : "Opened as a copy."}>
          <div className="row sm"><Check size={13} aria-hidden /> Validated · {report.message}</div>
          {report.safetyBackup && <div className="xs muted selectable-text">Safety backup: {report.safetyBackup}</div>}
          {report.migratedFrom != null && <div className="xs muted">The project was upgraded from an older OpenFrame version; a safety copy was kept.</div>}
          <LinkedFiles files={report.missingExternal} />
        </DoneBanner>
      ) : taskId ? (
        task?.state === "failed" ? (
          <Banner tone="err">{task.error?.message ?? "The package could not be opened."} Nothing was changed.</Banner>
        ) : task?.state === "cancelled" ? (
          <Banner tone="info">Cancelled. Nothing was changed.</Banner>
        ) : (
          <TaskProgressBar task={task} onCancel={() => void packagesApi.cancelTask(taskId).catch(reportError)} />
        )
      ) : !info ? (
        <EmptyState
          icon={<FileArchive size={28} />}
          title="Choose a package"
          actions={<Button variant="primary" disabled={checking} onClick={() => void choose()}>{checking ? "Checking…" : "Choose Package…"}</Button>}
        >
          .ofproject (Full Project Package) or .ofbackup (Backup Package). The package is validated before anything is created.
        </EmptyState>
      ) : (
        <>
          <div className="grid g2" style={{ gap: 8, marginBottom: 8 }}>
            <div className="card pad"><div className="xs muted">Package</div><b>{info.typeLabel}</b></div>
            <div className="card pad"><div className="xs muted">Project</div><b>{info.manifest.sourceProjectTitle}</b></div>
            <div className="card pad"><div className="xs muted">Created</div><b>{dayAndTime(info.manifest.exportedAt)}</b>{info.manifest.label ? <span className="xs muted"> · {info.manifest.label}</span> : null}</div>
            <div className="card pad"><div className="xs muted">Size</div><b>{formatBytes(info.sizeBytes)}</b> <Chip tone="g">Valid</Chip></div>
          </div>
          {collision ? (
            <div className="col" style={{ gap: 8 }} role="radiogroup" aria-label="How to open it">
              <label className={`card pad row${mode === "copy" ? " on" : ""}`} style={{ cursor: "pointer", alignItems: "flex-start" }}>
                <input type="radio" name="open-mode" checked={mode === "copy"} onChange={() => setMode("copy")} />
                <span>
                  <b>Open as Copy</b> <Chip tone="g">Default</Chip>
                  <div className="sm muted">Creates “{info.manifest.sourceProjectTitle} (copy)” with a new identity. Your existing project is not touched.</div>
                </span>
              </label>
              <label className={`card pad row${mode === "replace" ? " on" : ""}`} style={{ cursor: "pointer", alignItems: "flex-start" }}>
                <input type="radio" name="open-mode" checked={mode === "replace"} onChange={() => setMode("replace")} />
                <span>
                  <b>Replace Existing After Backup</b> <Chip tone="r">High impact</Chip>
                  <div className="sm muted">
                    Saves a backup of “{collision.title}” first, then replaces it with the package{collision.isOpen ? " (it will be closed)" : ""}.
                  </div>
                </span>
              </label>
            </div>
          ) : (
            <Banner tone="info">No project with this identity exists here. It will open as the same project, keeping its identity.</Banner>
          )}
          <div className="h4" style={{ marginTop: 10 }}>Import report</div>
          <div className="sm">
            <Check size={13} aria-hidden /> Validated
            {missing.length > 0
              ? ` · ${missing.length} linked external file${missing.length === 1 ? " is" : "s are"} missing on this PC (the project still opens).`
              : " · all files are included or available."}
          </div>
          <LinkedFiles files={info.external} />
          <div className="xs muted" style={{ marginTop: 8 }}>Importing never silently replaces the current project.</div>
        </>
      )}
      <ConfirmDialog
        open={confirmReplace}
        onOpenChange={setConfirmReplace}
        title="Replace the existing project?"
        confirmLabel="Back Up and Replace"
        danger
        onConfirm={() => {
          setConfirmReplace(false);
          start("replace");
        }}
      >
        <p className="sm">
          “{collision?.title}” will be backed up to the Backups folder and then replaced by the package. Work saved after the package was created will only be in that backup.
        </p>
      </ConfirmDialog>
    </Dialog>
  );
}
