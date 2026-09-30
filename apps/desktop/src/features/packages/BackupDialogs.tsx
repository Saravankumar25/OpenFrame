// Create Backup (FSD §44.5–44.8, mockup 169) and Export Full Project
// (FSD §45.1–45.2, mockup 170). Both run as background tasks with progress and
// a real Cancel; the status bar shows "Saved · finishing export" meanwhile.

import { useState } from "react";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import { Lock } from "lucide-react";
import { useOp, reportError } from "../../ipc/query";
import type { BackupPreview } from "../../ipc/generated/BackupPreview";
import type { BackupReport } from "../../ipc/generated/BackupReport";
import { Banner, Button, Checkbox, Dialog, Field, Skeleton, TextInput } from "../../design-system";
import { backupFileName, formatBytes, joinPath, packageFilter, packagesApi } from "./api";
import { isFinished, usePackagesUi, useTask } from "./store";
import { DoneBanner, FileStatusTable, LinkedFiles, TaskProgressBar } from "./parts";

export function usePackageTask() {
  const taskId = usePackagesUi((s) => s.taskId);
  const setTaskId = usePackagesUi((s) => s.setTaskId);
  const task = useTask(taskId);
  return { task, taskId, setTaskId, running: !!taskId && !isFinished(task) };
}

export function BackupDialog({ onClose }: { onClose: () => void }) {
  const preview = useOp<BackupPreview>("packages.backup_preview", {}, ["asset", "project"]);
  const [label, setLabel] = useState("");
  const [dest, setDest] = useState<string | null>(null);
  const [portable, setPortable] = useState(false);
  const [starting, setStarting] = useState(false);
  const { task, taskId, setTaskId, running } = usePackageTask();
  const p = preview.data;
  const name = p ? backupFileName(p.baseName, label) : "";
  const report = task?.state === "completed" ? (task.result as unknown as BackupReport) : null;

  const browse = async () => {
    if (!p) return;
    const chosen = await saveDialog({
      title: "Save backup to",
      defaultPath: dest ?? joinPath(p.defaultDir, name),
      filters: packageFilter(["backup"], "OpenFrame backup"),
    });
    if (typeof chosen === "string") setDest(chosen);
  };
  const start = async () => {
    if (!dest) return;
    setStarting(true);
    try {
      const r = await packagesApi.createBackup(dest, label, portable);
      setTaskId(r.taskId);
    } catch (e) {
      reportError(e);
    } finally {
      setStarting(false);
    }
  };

  return (
    <Dialog
      open
      onOpenChange={(v) => !v && !running && onClose()}
      title="Create Backup"
      sub="A recoverable copy of supported project state."
      size="md"
      dismissible={!running}
      footer={
        report || task?.state === "cancelled" || task?.state === "failed" ? (
          <Button variant="primary" onClick={onClose}>Close</Button>
        ) : (
          <>
            <Button onClick={onClose} disabled={running}>Cancel</Button>
            <Button variant="primary" disabled={!dest || running || starting || !p} onClick={() => void start()}>
              Create Backup
            </Button>
          </>
        )
      }
    >
      {!p ? (
        <Skeleton h={120} />
      ) : report ? (
        <DoneBanner title="Backup created." path={report.path}>
          {report.summary}. {formatBytes(report.bytes)}.
          <LinkedFiles files={[...report.external, ...report.missing]} />
        </DoneBanner>
      ) : taskId ? (
        task?.state === "cancelled" ? (
          <Banner tone="info">Backup cancelled. No backup file was left behind.</Banner>
        ) : task?.state === "failed" ? (
          <Banner tone="err">{task.error?.message ?? "The backup could not be created."} Your project was not changed.</Banner>
        ) : (
          <TaskProgressBar task={task} onCancel={() => void packagesApi.cancelTask(taskId).catch(reportError)} />
        )
      ) : (
        <>
          <Field label="Backup name" hint="Project name, date and time, and an optional label.">
            <TextInput value={name} readOnly aria-readonly />
          </Field>
          <Field label="Label (optional)">
            <TextInput value={label} maxLength={80} placeholder="e.g. before-rewrite" onChange={(e) => setLabel(e.target.value)} />
          </Field>
          <Field label="Save to" hint="Choose another disk if you can. Project backups are separate from your active project.">
            <div className="row">
              <TextInput value={dest ?? ""} readOnly placeholder="Choose where to save the backup" aria-label="Backup location" />
              <Button onClick={() => void browse()}>Browse…</Button>
            </div>
          </Field>
          <Checkbox checked={portable} onChange={setPortable} label="Copy linked external files into the backup (portable backup)" />
          <div className="h4" style={{ marginTop: 12 }}>What this backup contains</div>
          <div className="sm muted" style={{ marginBottom: 6 }}>
            Idea Vault, Story Board, screenplay drafts, breakdown, catalog, production planning, comments
            {p.privateNoteCount > 0 ? ", your private notes" : ""}.
          </div>
          <FileStatusTable
            rows={[
              { label: `Files stored in the project · ${formatBytes(p.includedBytes)}`, status: "Included", count: p.includedFiles },
              { label: "Linked external files", status: portable ? "Copied" : "External", count: p.external.length },
              { label: "Missing linked files", status: "Missing", count: p.missing.length },
            ]}
          />
          <LinkedFiles files={[...p.external, ...p.missing]} />
        </>
      )}
    </Dialog>
  );
}

export function ExportProjectDialog({ onClose }: { onClose: () => void }) {
  const preview = useOp<BackupPreview>("packages.backup_preview", {}, ["asset", "project"]);
  const [portable, setPortable] = useState(false);
  const { task, taskId, setTaskId, running } = usePackageTask();
  const p = preview.data;
  const report = task?.state === "completed" ? (task.result as unknown as BackupReport) : null;

  const exportNow = async () => {
    if (!p) return;
    const chosen = await saveDialog({
      title: "Export Project Package",
      defaultPath: joinPath(p.defaultDir.replace(/[\\/]Backups$/, ""), `${p.projectTitle}.ofproject`),
      filters: packageFilter(["project"], "OpenFrame project package"),
    });
    if (typeof chosen !== "string") return;
    try {
      const r = await packagesApi.exportProject(chosen, portable);
      setTaskId(r.taskId);
    } catch (e) {
      reportError(e);
    }
  };

  return (
    <Dialog
      open
      onOpenChange={(v) => !v && !running && onClose()}
      title="Export Full Project"
      size="md"
      dismissible={!running}
      footer={
        report || task?.state === "cancelled" || task?.state === "failed" ? (
          <Button variant="primary" onClick={onClose}>Close</Button>
        ) : (
          <>
            <Button onClick={onClose} disabled={running}>Cancel</Button>
            <Button variant="primary" disabled={running || !p} onClick={() => void exportNow()}>
              Export Project Package…
            </Button>
          </>
        )
      }
    >
      <p className="sm" style={{ marginTop: 0 }}>
        A Full Project Package moves your whole working project to another PC. It is not a review package and not a PDF collection.
      </p>
      {!p ? (
        <Skeleton h={120} />
      ) : report ? (
        <DoneBanner title="Project package exported." path={report.path}>
          {report.summary}. {formatBytes(report.bytes)}. Open it on the other PC with Open Project Package….
          <LinkedFiles files={[...report.external, ...report.missing]} />
        </DoneBanner>
      ) : taskId ? (
        task?.state === "cancelled" ? (
          <Banner tone="info">Export cancelled. No package file was left behind.</Banner>
        ) : task?.state === "failed" ? (
          <Banner tone="err">{task.error?.message ?? "The package could not be exported."}</Banner>
        ) : (
          <TaskProgressBar task={task} onCancel={() => void packagesApi.cancelTask(taskId).catch(reportError)} />
        )
      ) : (
        <>
          <div className="h4">Include</div>
          <div className="col" style={{ gap: 4, marginBottom: 8 }}>
            {p.groups.map((g) => (
              <Checkbox key={g.label} checked disabled onChange={() => undefined} label={`${g.label}${g.count ? ` (${g.count})` : ""}`} />
            ))}
          </div>
          <Banner tone="priv">
            <Lock size={13} aria-hidden /> <b>Private project transfer.</b> Private notes are included because this is for you, not for outside reviewers.
          </Banner>
          <div style={{ marginTop: 8 }}>
            <Checkbox checked={portable} onChange={setPortable} label={`Copy linked external files into the package (${p.external.length})`} />
            {p.missing.length > 0 && (
              <div className="xs muted" style={{ marginTop: 4 }}>
                {p.missing.length} linked file{p.missing.length === 1 ? " is" : "s are"} missing and will be listed as missing — the project still opens.
              </div>
            )}
          </div>
        </>
      )}
    </Dialog>
  );
}
