// Workspace registry. Each module owns its workspace directory under
// src/workspaces/<dir>/ and default-exports its root component from index.tsx.
// Discovery is by file convention, so modules never edit this file.

import { lazy, type ComponentType, type LazyExoticComponent } from "react";
import {
  Clapperboard,
  ClipboardList,
  Columns3,
  FileText,
  Folder,
  House,
  Lightbulb,
  ListChecks,
} from "lucide-react";
import type { WorkspaceId } from "./stores";
import { WorkspaceUnavailable } from "./shell/WorkspaceUnavailable";

const modules = import.meta.glob<{ default: ComponentType }>("../workspaces/*/index.tsx");

function load(dir: string): LazyExoticComponent<ComponentType> {
  const loader = modules[`../workspaces/${dir}/index.tsx`];
  return lazy(loader ?? (async () => ({ default: WorkspaceUnavailable })));
}

export interface WorkspaceDef {
  id: WorkspaceId;
  label: string;
  icon?: ComponentType<{ size?: number }>;
  /** Shown in the left navigation (FSD §3.1 exact order). */
  nav: boolean;
  component: LazyExoticComponent<ComponentType>;
}

export const WORKSPACES: WorkspaceDef[] = [
  { id: "home", label: "Home", icon: House, nav: true, component: load("project-home") },
  { id: "vault", label: "Idea Vault", icon: Lightbulb, nav: true, component: load("idea-vault") },
  { id: "story", label: "Story", icon: Columns3, nav: true, component: load("story") },
  { id: "screenplay", label: "Screenplay", icon: FileText, nav: true, component: load("screenplay") },
  { id: "breakdown", label: "Breakdown", icon: ListChecks, nav: true, component: load("breakdown") },
  { id: "production", label: "Production", icon: Clapperboard, nav: true, component: load("production") },
  { id: "callsheets", label: "Call Sheets", icon: ClipboardList, nav: true, component: load("call-sheets") },
  { id: "files", label: "Files", icon: Folder, nav: true, component: load("files") },
  { id: "trash", label: "Recently Deleted", nav: false, component: load("trash") },
  { id: "activity", label: "Activity", nav: false, component: load("activity") },
  { id: "notes", label: "Notes & Tasks", nav: false, component: load("notes") },
  { id: "settings", label: "Project settings", nav: false, component: load("settings") },
];

export function workspaceDef(id: WorkspaceId): WorkspaceDef {
  return WORKSPACES.find((w) => w.id === id) ?? WORKSPACES[0];
}
