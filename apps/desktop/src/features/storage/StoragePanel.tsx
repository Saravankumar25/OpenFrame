// Settings > Storage: where OpenFrame keeps each kind of data on this computer, and the actions
// that change a place (docs/engineering/40-storage-locations-and-recovery.md, ADR-0038).
// Plain words keep the kinds apart: the library folder (the one to back up), the folder for new
// projects, the Global Idea Vault, the open project and the app's own technical data.

import type { ReactNode } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { FolderOpen } from "lucide-react";
import { call, revealLocation } from "../../ipc/client";
import { reportError, useOp } from "../../ipc/query";
import { refreshStorage } from "./refresh";
import type { StorageInfo } from "../../ipc/generated/StorageInfo";
import { Button, Skeleton } from "../../design-system";
import { toast } from "../../app/toast";
import { VaultActions } from "./VaultActions";
import { ProjectStorage } from "./ProjectStorage";
import { LibraryActions } from "../library-setup/LibraryActions";



function Place({ title, path, note, children }: { title: string; path: string; note: ReactNode; children?: ReactNode }) {
  return (
    <section className="card pad" aria-label={title} style={{ marginBottom: 12 }}>
      <div className="h4" style={{ margin: "0 0 4px" }}>{title}</div>
      <div className="sm selectable-text" style={{ wordBreak: "break-all", marginBottom: 4 }}>{path}</div>
      <div className="xs muted" style={{ marginBottom: 8 }}>{note}</div>
      <div className="row" style={{ gap: 6, flexWrap: "wrap" }}>{children}</div>
    </section>
  );
}

export function StoragePanel() {
  const info = useOp<StorageInfo>("storage.info", {}, ["*"], { refetchOnMount: "always" });
  const s = info.data;
  if (!s) return info.isError ? <div className="muted sm">{info.error?.message}</div> : <Skeleton h={200} />;

  const changeProjectsDir = async () => {
    const dir = await openDialog({ directory: true, title: "Choose the folder for new projects", defaultPath: s.projectsDir });
    if (typeof dir !== "string") return;
    try {
      await call<StorageInfo>("storage.set_projects_dir", { path: dir });
      refreshStorage();
      toast.success("New projects will be created in that folder. Projects you already have stay where they are.");
    } catch (e) {
      reportError(e);
    }
  };

  return (
    <div style={{ maxWidth: 820 }}>
      <Place
        title="Your OpenFrame library"
        path={s.libraryDir}
        note="Back up this folder. It holds library.json, which remembers who you are on this computer and where your work is, so a reinstall finds everything again. By default it also holds your projects and the Global Idea Vault."
      >
        <Button size="sm" icon={<FolderOpen size={14} />} onClick={() => void revealLocation("dataRoot").catch(reportError)}>Open Folder</Button>
        <LibraryActions info={s} />
      </Place>
      <Place
        title="Folder for new projects"
        path={s.projectsDir + (s.projectsDirIsDefault ? " (default)" : "")}
        note="New projects are created here unless you choose another folder when you create one. Changing it does not move the projects you already have."
      >
        <Button size="sm" onClick={() => void changeProjectsDir()}>Change Folder…</Button>
      </Place>
      <Place
        title="Global Idea Vault"
        path={s.vaultDir + (s.vaultDirIsDefault ? " (default)" : "")}
        note={s.vaultExists ? "Your ideas, research and their pictures, shared by every project." : "Not created yet. It is created the first time you open the Idea Vault, or you can connect an existing vault."}
      >
        <VaultActions info={s} />
      </Place>
      {s.projectDir && <ProjectStorage path={s.projectDir} />}
      <Place
        title="App data"
        path={s.appDataDir}
        note="Settings, the recent project list, the assistant's files and logs. Not your work: uninstalling may remove it, and your library stays."
      >
        <Button size="sm" icon={<FolderOpen size={14} />} onClick={() => void revealLocation("appData").catch(reportError)}>Open Folder</Button>
      </Place>
    </div>
  );
}
