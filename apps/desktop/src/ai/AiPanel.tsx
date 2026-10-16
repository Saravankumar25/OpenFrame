// AI Assistant side panel (FSD §42; UX §3.40; mockups 174 to 181; Local AI Runtime spec §1; ADR-0026).
//
// - The panel docks beside the page (owner decision 2026-10-08): the Shell places it after the
//   page in the layout row, so opening it narrows the page and never covers a control.
// - OpenFrame AI is a one-time download after install, started by the person: until it is
//   downloaded (or when a downloaded file is damaged) the panel offers the download; when the
//   engine in the app is missing or damaged it says to reinstall. Nothing else in OpenFrame
//   depends on it.
// - Everything runs on this computer; the UI never shows model files, formats or ports.
// - The conversation, sources and "Proposed Changes" review live in ./Conversation
//   and ./ProposedChanges: project changes are only ever proposed for the user
//   to review and apply; stale proposals must be re-checked first.

import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { MoreHorizontal, Sparkles, X } from "lucide-react";
import { call, onAppEvent } from "../ipc/client";
import { reportError, useOp } from "../ipc/query";
import { Chip, IconButton, Menu, Skeleton } from "../design-system";
import { useNav } from "../app/stores";
import { toast } from "../app/toast";
import type { ProjectSummary } from "../ipc/generated/ProjectSummary";
import type { AiStatusDto } from "../ipc/generated/AiStatusDto";
import { UNAVAILABLE_TEXT, damagedModels, offReason, panelMode } from "./model";
import DownloadAi from "./DownloadAi";
import Assistant from "./Conversation";
import "./ai.css";

export default function AiPanel({ onClose }: { onClose: () => void }) {
  const qc = useQueryClient();
  const project = useOp<ProjectSummary | null>("project.current", {}, ["project"]);
  const status = useOp<AiStatusDto>("ai.status", {}, [], { staleTime: 0 });

  // Engine state changes arrive as module events from the Rust core.
  useEffect(() => {
    const un = onAppEvent((ev) => {
      if (ev.type === "module" && ev.module === "ai") void qc.invalidateQueries({ queryKey: ["ai.status"] });
    });
    return () => {
      void un.then((f) => f());
    };
  }, [qc]);

  const s = status.data;
  const mode = panelMode(s);
  const go = useNav((st) => st.go);
  const settings = [
    { label: "Offline AI", header: true },
    // The panel is docked beside the page, so it stays open next to the settings it opens.
    ...(project.data
      ? [
          {
            label: "Offline AI settings…",
            onSelect: () => go({ workspace: "settings", sub: "offline-ai" }),
          },
        ]
      : []),
    ...(s?.available
      ? [
          {
            label: "Stop Offline AI (free memory)",
            onSelect: () => void call("ai.stop_runtime").then(() => toast.info("Offline AI stopped. It starts again when you ask something.")).catch(reportError),
          },
        ]
      : []),
  ];

  return (
    <aside className="ai" aria-label="AI Assistant">
      <div className="ahd">
        <Sparkles size={16} aria-hidden />
        AI Assistant
        <span className="grow" />
        <Chip tone={s?.available ? "g" : "default"} title={s?.available ? "Runs on this computer" : undefined}>
          {s?.mode ?? "Off"}
        </Chip>
        {settings.length > 1 && (
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
        <Unavailable text={UNAVAILABLE_TEXT} />
      ) : mode === "download" ? (
        <div className="body">
          <DownloadAi damaged={damagedModels(s!)} />
        </div>
      ) : mode === "off" ? (
        <Unavailable text={offReason(s!)} />
      ) : (
        <Assistant status={s!} hasProject={!!project.data} />
      )}
    </aside>
  );
}

// ----------------------------------------------------------------- unavailable

/** Offline AI cannot run: the subsystem is unreachable, or the engine in the app is missing or damaged. */
function Unavailable({ text }: { text: string }) {
  return (
    <div className="body">
      <div className="empty" style={{ height: "auto", padding: 20 }} role="status">
        <div className="ico" aria-hidden>
          <Sparkles size={26} />
        </div>
        <h2 style={{ fontSize: 15 }}>AI is not available right now</h2>
        <p className="sm">{text}</p>
        <p className="xs muted">Everything else in OpenFrame works normally.</p>
      </div>
    </div>
  );
}
