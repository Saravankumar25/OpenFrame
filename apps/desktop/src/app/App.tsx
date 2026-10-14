import { useOp } from "../ipc/query";
import type { ProjectSummary } from "../ipc/generated/ProjectSummary";
import { Shell } from "./shell/Shell";
import { AppHome } from "./home/AppHome";
import { Toaster } from "./toast";
import { LicenceScreens } from "../features/licensing/LicenceScreens";
import { useShortcutSync } from "./shortcuts";
import { BrandLogo } from "../theme/BrandLogo";
import { useAppearance } from "../theme/useAppearance";
import { TeamGate } from "../features/team-gate/TeamGate";

/** Application root. Internal builds ask the team password first (ADR-0024). */
export function App() {
  useAppearance();
  return (
    <TeamGate>
      <AppContent />
    </TeamGate>
  );
}

/** Home (no project) or the project shell. */
function AppContent() {
  const current = useOp<ProjectSummary | null>("project.current", {}, ["project"]);
  useShortcutSync();
  // The splash covers the first answer only. Later the query can be pending again without data
  // (the store lock refuses project.current, and a component that reads it mounts and asks again):
  // showing the splash then unmounted Home, whose packages overlay asked again when it mounted
  // next, in a loop that never reached the lock screen.
  if (current.isPending && !current.isFetched) {
    return (
      <div className="app-root splash" role="status">
        <BrandLogo size="lg" />
        <div className="splash-sub">Opening your local projects…</div>
      </div>
    );
  }
  return (
    <>
      {current.data ? <Shell /> : <AppHome />}
      <LicenceScreens projectOpen={!!current.data} />
      <Toaster />
    </>
  );
}
