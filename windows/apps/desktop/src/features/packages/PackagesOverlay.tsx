// Mounts whichever package dialog is open. Rendered once by the shell (via
// app/shell/extensions/packages.tsx), so every entry point shares one place.

import { useEffect } from "react";
import { useOp } from "../../ipc/query";
import type { ProjectSummary } from "../../ipc/generated/ProjectSummary";
import { BackupDialog, ExportProjectDialog } from "./BackupDialogs";
import { OpenProjectDialog } from "./OpenProjectDialog";
import { ExportExchangeDialog } from "./ExportExchangeDialog";
import { ImportExchangeDialog } from "./ImportExchangeDialog";
import { ReviewViewer } from "./ReviewViewer";
import { PackageHistoryDialog, ReviewQueueDialog } from "./QueueAndHistory";
import { ensureTaskListener, usePackagesUi } from "./store";

export function PackagesOverlay() {
  const dialog = usePackagesUi((s) => s.dialog);
  const close = usePackagesUi((s) => s.close);
  const open = usePackagesUi((s) => s.open);
  const project = useOp<ProjectSummary | null>("project.current", {}, ["project"]);
  // Listen for background task events before any package task can start.
  useEffect(() => ensureTaskListener(), []);
  if (!dialog) return null;
  switch (dialog.kind) {
    case "backup":
      return <BackupDialog onClose={close} />;
    case "exportProject":
      return <ExportProjectDialog onClose={close} />;
    case "openProject":
      return <OpenProjectDialog onClose={close} currentTitle={project.data?.title ?? null} />;
    case "exportExchange":
      return <ExportExchangeDialog packageType={dialog.packageType} onClose={close} />;
    case "importExchange":
      return <ImportExchangeDialog key={dialog.sessionId ?? "new"} sessionId={dialog.sessionId} onClose={close} />;
    case "viewer":
      return (
        <ReviewViewer
          source={dialog.source}
          reviewKey={dialog.key}
          onClose={() => (dialog.back ? open(dialog.back) : close())}
        />
      );
    case "queue":
      return <ReviewQueueDialog onClose={close} />;
    case "history":
      return <PackageHistoryDialog onClose={close} />;
  }
}
