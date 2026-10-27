// Home: projects found in the projects folder that are not in Recent Projects (after a reinstall,
// an app data reset, or a project copied in by hand). Nothing is added or opened until the person
// chooses Open. See docs/engineering/40-storage-locations-and-recovery.md.

import { FolderOpen } from "lucide-react";
import { call } from "../../ipc/client";
import { reportError, useOp } from "../../ipc/query";
import type { DiscoveredProject } from "../../ipc/generated/DiscoveredProject";
import { Button } from "../../design-system";
import { shortDate } from "./format";

export function FoundProjects({ onOpened }: { onOpened: () => void }) {
  const found = useOp<DiscoveredProject[]>("project.discover", {}, ["sys_recent_project"], { refetchOnMount: "always" });
  const list = Array.isArray(found.data) ? found.data : [];
  if (list.length === 0) return null;
  const open = async (p: DiscoveredProject) => {
    try {
      await call("project.open", { path: p.path });
      onOpened();
    } catch (e) {
      reportError(e);
    }
  };
  return (
    <section aria-label="Found on this PC" style={{ marginTop: 24 }}>
      <div className="h4" style={{ margin: "0 0 2px" }}>Found on this PC</div>
      <div className="xs muted" style={{ marginBottom: 8 }}>
        These projects are in your projects folder but not in your list. Open one to add it back.
      </div>
      <div className="col" style={{ gap: 6 }}>
        {list.map((p) => (
          <div key={p.path} className="card row" style={{ padding: "8px 12px", gap: 12, alignItems: "center" }}>
            <div className="grow" style={{ minWidth: 0 }}>
              <div className="sm" style={{ fontWeight: 600 }}>{p.title}</div>
              <div className="xs muted selectable-text" style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={p.path}>
                {p.path}
                {p.modifiedAt ? ` · changed ${shortDate(p.modifiedAt)}` : ""}
              </div>
            </div>
            <Button size="sm" icon={<FolderOpen size={14} />} onClick={() => void open(p)} aria-label={`Open ${p.title}`}>
              Open
            </Button>
          </div>
        ))}
      </div>
    </section>
  );
}
