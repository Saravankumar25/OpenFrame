// Share / Exchange, Backup and Portability — entry-point registry.
//
// Other surfaces (the shell's project menu today; a workspace toolbar later)
// list `packageActions` or call `packageMenuItems()`; every action opens its
// dialog in the single `PackagesOverlay`.

import { open as openDialog } from "@tauri-apps/plugin-dialog";
import type { MenuItemSpec } from "../../design-system";
import { reportError } from "../../ipc/query";
import { EXCHANGE_EXPORT_TYPES, PACKAGE_INFO, REVIEW_OPEN_FILTER, packagesApi } from "./api";
import { usePackagesUi, type PackagesDialog } from "./store";

export { PackagesOverlay } from "./PackagesOverlay";
export { usePackagesUi } from "./store";
export type { PackagesDialog } from "./store";

export interface PackageAction {
  id: string;
  label: string;
  section: "exchange" | "import" | "portability";
  run: () => void;
}

const show = (d: PackagesDialog) => () => usePackagesUi.getState().open(d);

/** "Open Review Package…": validate a review package and open it read-only for commenting. */
export async function openReviewPackage(): Promise<void> {
  const chosen = await openDialog({ title: "Open Review Package", multiple: false, filters: REVIEW_OPEN_FILTER });
  if (typeof chosen !== "string") return;
  try {
    const ws = await packagesApi.reviewOpen(chosen);
    usePackagesUi.getState().open({ kind: "viewer", source: "review", key: ws.key });
  } catch (e) {
    reportError(e);
  }
}

export const packageActions: PackageAction[] = [
  ...EXCHANGE_EXPORT_TYPES.map<PackageAction>((t) => ({
    id: `packages.export.${t}`,
    label: PACKAGE_INFO[t].menuLabel,
    section: "exchange",
    run: show({ kind: "exportExchange", packageType: t }),
  })),
  { id: "packages.import", label: "Import Exchange Package…", section: "import", run: show({ kind: "importExchange" }) },
  { id: "packages.review_open", label: "Open Review Package…", section: "import", run: () => void openReviewPackage() },
  { id: "packages.queue", label: "Review Queue", section: "import", run: show({ kind: "queue" }) },
  { id: "packages.history", label: "Import & Export History", section: "import", run: show({ kind: "history" }) },
  { id: "packages.backup", label: "Create Backup…", section: "portability", run: show({ kind: "backup" }) },
  { id: "packages.export_project", label: "Export Full Project…", section: "portability", run: show({ kind: "exportProject" }) },
  { id: "packages.open_project", label: "Open Project Package…", section: "portability", run: show({ kind: "openProject" }) },
];

export interface PackageMenuHelpers {
  /** False when no project is open (project-only actions are hidden). */
  projectOpen: boolean;
}

/** Items for a menu: "Share / Exchange" and "Backup & Portability" sections (UX §3.39). */
export function packageMenuItems(h: PackageMenuHelpers): MenuItemSpec[] {
  if (!h.projectOpen) return [];
  const item = (a: PackageAction, separatorBefore = false): MenuItemSpec => ({ label: a.label, onSelect: a.run, separatorBefore });
  const exchange = packageActions.filter((a) => a.section === "exchange");
  const imports = packageActions.filter((a) => a.section === "import");
  const portability = packageActions.filter((a) => a.section === "portability");
  return [
    { label: "Share / Exchange", header: true, separatorBefore: true },
    ...exchange.map((a) => item(a)),
    ...imports.map((a, i) => item(a, i === 0)),
    { label: "Backup & Portability", header: true, separatorBefore: true },
    ...portability.map((a) => item(a)),
  ];
}
