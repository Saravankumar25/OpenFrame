// Project settings workspace (UX §3.4 "Project settings", mock 022; §3.37 Templates,
// mock 153). Sub-pages: Project settings | Templates.

import { useState } from "react";
import { FolderOpen } from "lucide-react";
import { revealLocation } from "../../ipc/client";
import { reportError, useOp } from "../../ipc/query";
import type { ProjectSummary } from "../../ipc/generated/ProjectSummary";
import { Button, PageHeader } from "../../design-system";
import { useNav } from "../../app/stores";
import { dayAndTime } from "../../app/home/format";
import { ProjectSettingsForm } from "./ProjectSettingsForm";
import { Templates } from "./Templates";
import OfflineAiSettings from "./OfflineAi";

const TABS = [
  { id: "general", label: "Project settings" },
  { id: "templates", label: "Templates" },
  { id: "offline-ai", label: "Offline AI" },
] as const;

export default function SettingsWorkspace() {
  const route = useNav((s) => s.route);
  const go = useNav((s) => s.go);
  const tab = route.sub === "templates" || route.sub === "offline-ai" ? route.sub : "general";
  return (
    <>
      <div className="subnav" role="tablist" aria-label="Settings">
        {TABS.map((t) => (
          <button
            key={t.id}
            role="tab"
            aria-selected={tab === t.id}
            className={tab === t.id ? "active" : undefined}
            onClick={() => go({ workspace: "settings", sub: t.id === "general" ? undefined : t.id })}
          >
            {t.label}
          </button>
        ))}
      </div>
      {tab === "templates" ? <Templates /> : tab === "offline-ai" ? <OfflineAiSettings /> : <General />}
    </>
  );
}

function General() {
  const project = useOp<ProjectSummary | null>("project.current", {}, ["project"]);
  const [dirty, setDirty] = useState(false);
  const p = project.data;
  return (
    <div className="content" style={{ overflow: "auto" }}>
      <PageHeader
        title="Project settings"
        sub="Project identity fields."
        actions={<Button variant="primary" type="submit" form="project-settings-page" disabled={!dirty}>Save</Button>}
      />
      <div className="row" style={{ alignItems: "flex-start", gap: 16 }}>
        <section className="card pad grow" style={{ maxWidth: 720 }} aria-label="Identity">
          <ProjectSettingsForm formId="project-settings-page" onDirtyChange={setDirty} />
        </section>
        {p && (
          <aside className="card pad" style={{ width: 300, flex: "none" }} aria-label="Where this project is stored">
            <div className="h4">Stored on this computer</div>
            <div className="sm selectable-text" style={{ wordBreak: "break-all", marginBottom: 8 }}>{p.path}</div>
            <Button size="sm" icon={<FolderOpen size={14} />} onClick={() => void revealLocation("project").catch(reportError)}>Open project location</Button>
            <div className="hr" />
            <div className="xs muted">Created {dayAndTime(p.createdAt)}</div>
            <div className="xs muted">Last changed {dayAndTime(p.updatedAt)}</div>
            <div className="xs muted" style={{ marginTop: 6 }}>The project works fully offline. Everything stays in this folder.</div>
          </aside>
        )}
      </div>
    </div>
  );
}
