import { EmptyState } from "../../design-system";

/** Rendered only if a workspace bundle is missing from the build (a packaging defect). */
export function WorkspaceUnavailable() {
  return (
    <EmptyState title="This workspace isn't available in this build.">
      Reinstall OpenFrame Studio. Your project is safe and unchanged.
    </EmptyState>
  );
}
