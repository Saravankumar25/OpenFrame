// AI Assistant side panel (FSD §42; UX §3.40; mockups 174–181; Local AI Runtime spec §1).
//
// - OpenFrame works fully without it: when Offline AI isn't installed the panel
//   shows the one-click setup (./OfflineAiSetup) and nothing else depends on it.
// - Everything runs on this computer; the UI never shows model files, formats or ports.
// - The conversation, sources and "Proposed Changes" review live in ./Conversation
//   and ./ProposedChanges: project changes are only ever proposed for the user
//   to review and apply; stale proposals must be re-checked first.

import { useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { MoreHorizontal, Sparkles, X } from "lucide-react";
import { call, onAppEvent } from "../ipc/client";
import { reportError, useOp } from "../ipc/query";
import { Chip, ConfirmDialog, IconButton, Menu, Skeleton } from "../design-system";
import { useNav } from "../app/stores";
import { toast } from "../app/toast";
import type { ProjectSummary } from "../ipc/generated/ProjectSummary";
import type { AiStatusDto } from "../ipc/generated/AiStatusDto";
import type { AiUninstallResult } from "../ipc/generated/AiUninstallResult";
import { UNAVAILABLE_TEXT, formatBytes, panelMode } from "./model";
import Assistant from "./Conversation";
import OfflineAiSetup from "./OfflineAiSetup";
import "./ai.css";

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
        if (ev.type === "task") void qc.invalidateQueries({ queryKey: ["ai.setup_info"] });
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

  const go = useNav((st) => st.go);
  const settings = s?.installed
    ? [
        { label: "Offline AI", header: true },
        ...(s.updateAvailable ? [{ label: "Update Offline AI…", onSelect: () => setSetup(true) }] : []),
        ...(project.data ? [{ label: "Offline AI settings…", onSelect: () => go({ workspace: "settings", sub: "offline-ai" }) }] : []),
        { label: "Stop Offline AI (free memory)", onSelect: () => void call("ai.stop_runtime").then(() => toast.info("Offline AI stopped. It starts again when you ask something.")).catch(reportError) },
        { label: "Remove Offline AI…", danger: true, separatorBefore: true, onSelect: () => setRemoveOpen(true) },
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
        <Unavailable />
      ) : mode === "setup" || mode === "installing" ? (
        <OfflineAiSetup status={s!} onCancel={setup ? () => setSetup(false) : undefined} />
      ) : (
        <Assistant status={s!} hasProject={!!project.data} />
      )}
      <ConfirmDialog
        open={removeOpen}
        onOpenChange={setRemoveOpen}
        title="Remove Offline AI?"
        confirmLabel="Remove"
        danger
        onConfirm={() => {
          setRemoveOpen(false);
          call<AiUninstallResult>("ai.uninstall")
            .then((r) => {
              toast.info(`Offline AI was removed (${r.freedLabel} freed). Everything else in OpenFrame works normally.`);
              void qc.invalidateQueries({ queryKey: ["ai.status"] });
            })
            .catch(reportError);
        }}
      >
        The downloaded AI components ({formatBytes(s?.installedBytes ?? 0)}) are deleted from this computer. Your projects and your assistant history are not affected. You can download Offline AI again later.
      </ConfirmDialog>
    </aside>
  );
}

// ----------------------------------------------------------------- unavailable

/** The AI subsystem itself couldn't be reached ("not installed" is the setup view). */
function Unavailable() {
  return (
    <div className="body">
      <div className="empty" style={{ height: "auto", padding: 20 }}>
        <div className="ico" aria-hidden>
          <Sparkles size={26} />
        </div>
        <h2 style={{ fontSize: 15 }}>AI is not available right now</h2>
        <p className="sm">{UNAVAILABLE_TEXT}</p>
      </div>
    </div>
  );
}
