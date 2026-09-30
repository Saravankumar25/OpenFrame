// Manual project status chip menu (FSD §4.3, §115; mock 015). The status is a
// user-controlled label: changing it creates no tasks and disables nothing, and
// the app never changes it automatically. "Archived" is the separate archive
// flag (FSD §4.5) — archiving hides the project from the recent list without
// deleting anything.

import { Check, ChevronDown } from "lucide-react";
import { call } from "../../ipc/client";
import { reportError } from "../../ipc/query";
import type { ProjectStatus } from "../../ipc/generated/ProjectStatus";
import type { ProjectSummary } from "../../ipc/generated/ProjectSummary";
import { Menu, type MenuItemSpec } from "../../design-system";
import { toast } from "../../app/toast";

export const PROJECT_STATUSES: ProjectStatus[] = [
  "Idea",
  "Development",
  "Writing",
  "Rewrite",
  "Shooting Draft",
  "Pre-Production",
  "Shoot Preparation",
  "Shooting",
];

export async function setProjectStatus(status: ProjectStatus): Promise<void> {
  try {
    await call("project.set_status", { status });
    toast.undoable(`Changed project status to ${status}`);
  } catch (e) {
    reportError(e);
  }
}

export async function setProjectArchived(project: ProjectSummary, archived: boolean): Promise<void> {
  try {
    await call("project.set_archived", { projectId: project.id, archived });
    toast.info(archived ? "This project is archived. It stays on your computer and is hidden from the recent list." : "This project is back in your recent list.");
  } catch (e) {
    reportError(e);
  }
}

export function StatusMenu({ project, label }: { project: ProjectSummary; label?: string }) {
  const items: MenuItemSpec[] = [
    { label: "Project status", header: true },
    ...PROJECT_STATUSES.map((s) => ({
      label: s,
      icon: <span style={{ width: 14, display: "inline-flex" }}>{s === project.status && <Check size={14} aria-hidden />}</span>,
      onSelect: () => void (s !== project.status && setProjectStatus(s)),
    })),
    {
      label: project.archived ? "Archived — Restore to Active" : "Archived",
      separatorBefore: true,
      icon: <span style={{ width: 14, display: "inline-flex" }}>{project.archived && <Check size={14} aria-hidden />}</span>,
      onSelect: () => void setProjectArchived(project, !project.archived),
    },
  ];
  return (
    <Menu
      align="end"
      items={items}
      trigger={
        <button className="chip a" style={{ border: 0, cursor: "pointer", fontSize: 12, padding: "4px 10px" }} aria-label={`${label ?? "Status"}: ${project.status}${project.archived ? ", archived" : ""}. Change status`}>
          {project.archived ? `${project.status} · Archived` : project.status} <ChevronDown size={12} aria-hidden />
        </button>
      }
    />
  );
}
