import { useOp } from "../ipc/query";
import type { ProjectSummary } from "../ipc/generated/ProjectSummary";
import { Shell } from "./shell/Shell";
import { AppHome } from "./home/AppHome";
import { Toaster } from "./toast";

/** Application root: Home (no project) or the project shell. */
export function App() {
  const current = useOp<ProjectSummary | null>("project.current", {}, ["project"]);
  if (current.isLoading) {
    return (
      <div className="app-root" style={{ alignItems: "center", justifyContent: "center", background: "radial-gradient(circle at 50% 40%,#2c3350,#12141c)", color: "#fff" }}>
        <div style={{ fontSize: 22, fontWeight: 700 }}>OpenFrame Studio</div>
        <div style={{ opacity: 0.7, marginTop: 6 }}>Opening your local projects…</div>
      </div>
    );
  }
  return (
    <>
      {current.data ? <Shell /> : <AppHome />}
      <Toaster />
    </>
  );
}
