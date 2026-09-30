// Shell extension: Share / Exchange, Backup and Portability entry points in the
// project menu, plus the overlay that hosts their dialogs. Loaded by Shell.tsx
// through `import.meta.glob("./extensions/*.tsx")`.

import type { MenuItemSpec } from "../../../design-system";
import type { ProjectSummary } from "../../../ipc/generated/ProjectSummary";
import { PackagesOverlay, packageMenuItems } from "../../../features/packages";

export function projectMenuItems(helpers: { project: ProjectSummary | null }): MenuItemSpec[] {
  return packageMenuItems({ projectOpen: !!helpers.project });
}

export const ShellOverlay = PackagesOverlay;
