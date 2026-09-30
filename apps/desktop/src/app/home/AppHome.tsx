// Application Home (UX §3.2, mocks 003–010, 023): which project should I continue or open?
// Pinned + recent project cards, project search, archived table, card context
// menu (Open, Rename F2, Duplicate, Pin/Unpin, Reveal, Archive, Delete…),
// strong delete confirmation with "Archive Instead", unavailable-project dialog,
// empty state, Ctrl+N / Ctrl+O and the first-run name prompt.
// With no project open, Home is also the way into the Global Idea Vault
// (FSD §5.8): the idea-vault workspace in global scope, with global undo.

import { Suspense, useEffect, useMemo, useRef, useState, type RefObject } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { Archive, ArrowLeft, CircleHelp, Copy, FolderOpen, FolderSearch, Lightbulb, MoreHorizontal, Pencil, Pin, PinOff, Plus, RotateCcw, Search, Trash2 } from "lucide-react";
import { call, revealLocation } from "../../ipc/client";
import { reportError, useOp } from "../../ipc/query";
import type { AppInfo } from "../../ipc/generated/AppInfo";
import type { OpenProjectResult } from "../../ipc/generated/OpenProjectResult";
import type { RecentProject } from "../../ipc/generated/RecentProject";
import { Banner, Button, Chip, ContextMenu, Dialog, EmptyState, Field, Menu, PageHeader, Skeleton, TextInput, type MenuItemSpec } from "../../design-system";
import { useIntent, useNav, useUi } from "../stores";
import { workspaceDef } from "../routes";
import { ErrorBoundary } from "../shell/ErrorBoundary";
import { runUndo } from "../shell/Shell";
import { toast } from "../toast";
import { ago, shortDate } from "./format";
import { useHomeView } from "./homeState";
import { NewProjectDialog } from "./NewProjectDialog";
import { DisplayNameDialog } from "./DisplayNameDialog";

const PREVIEWS = ["night", "bus", "green", "warm", "cool"] as const;
function previewFor(id: string): string {
  let h = 0;
  for (let i = 0; i < id.length; i++) h = (h * 31 + id.charCodeAt(i)) >>> 0;
  return PREVIEWS[h % PREVIEWS.length];
}

function initials(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  return ((parts[0]?.[0] ?? "") + (parts.length > 1 ? parts[parts.length - 1][0] : "")).toUpperCase() || "?";
}

function lastModified(p: RecentProject): number {
  return p.modifiedAt ?? p.lastOpenedAt;
}

function isEditable(t: EventTarget | null): boolean {
  const el = t as HTMLElement | null;
  if (!el) return false;
  return el.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(el.tagName);
}

/** Show the Global Idea Vault from Home (no project open), optionally starting a Quick Capture. */
export function openGlobalVault(quickCapture = false): void {
  useUi.getState().setUndoScope("global");
  useNav.getState().go({ workspace: "vault", params: { scope: "global" } });
  if (quickCapture) useIntent.getState().fire("vault.new_note");
  useHomeView.getState().setView("vault");
}

function closeGlobalVault(): void {
  useUi.getState().setUndoScope("project");
  useNav.getState().reset();
  useHomeView.getState().setView("projects");
}

export function AppHome() {
  const view = useHomeView((s) => s.view);
  if (view === "vault") return <GlobalVaultHome />;
  return <ProjectsHome />;
}

/** Global Idea Vault with no project open: the vault workspace in global scope. */
function GlobalVaultHome() {
  const Vault = workspaceDef("vault").component;
  useEffect(() => {
    useUi.getState().setUndoScope("global");
    // The Global Idea Vault has its own undo history (Ctrl+Z / Ctrl+Y here are global-vault undo).
    const onKey = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey) || e.altKey || isEditable(e.target) || e.defaultPrevented) return;
      const k = e.key.toLowerCase();
      if (k === "z" && !e.shiftKey) {
        e.preventDefault();
        void runUndo("global");
      } else if (k === "y" || (k === "z" && e.shiftKey)) {
        e.preventDefault();
        void runUndo("global", true);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
  return (
    <div className="app-root">
      <header className="topbar">
        <div className="brand">
          <span className="mark" aria-hidden>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2"><rect x="3" y="5" width="18" height="14" rx="2" /><path d="M7 5v14M17 5v14" /></svg>
          </span>
          OpenFrame
        </div>
        <Button size="sm" variant="ghost" icon={<ArrowLeft size={14} />} onClick={closeGlobalVault}>Your projects</Button>
        <div className="grow" />
      </header>
      <div className="layout">
        <main className="main" aria-label="Global Idea Vault">
          <ErrorBoundary>
            <Suspense fallback={<div className="content"><Skeleton h={28} w={240} /></div>}>
              <Vault />
            </Suspense>
          </ErrorBoundary>
        </main>
      </div>
      <footer className="status">
        <span className="it"><span className="dot gray" aria-hidden /> No project open · Global Idea Vault</span>
        <div className="r"><span className="it">Offline · Your ideas stay on this computer</span></div>
      </footer>
    </div>
  );
}

function ProjectsHome() {
  const showArchived = useHomeView((s) => s.showArchived);
  const setShowArchived = useHomeView((s) => s.setShowArchived);
  const newRequested = useHomeView((s) => s.newProjectRequested);
  const requestNew = useHomeView((s) => s.requestNewProject);
  const [newOpen, setNewOpen] = useState(false);
  const [filter, setFilter] = useState("");
  const [unavailable, setUnavailable] = useState<RecentProject | null>(null);
  const [deleting, setDeleting] = useState<RecentProject | null>(null);
  const [renaming, setRenaming] = useState<RecentProject | null>(null);
  const [nameOpen, setNameOpen] = useState(false);
  const recents = useOp<RecentProject[]>("project.list_recent", { includeArchived: true }, ["*"], { refetchOnMount: "always" });
  const info = useOp<AppInfo>("app.info", {}, []);
  const reset = useNav((s) => s.reset);
  const searchRef = useRef<HTMLInputElement>(null);
  const dialogOpen = newOpen || !!unavailable || !!deleting || !!renaming || nameOpen;

  useEffect(() => {
    if (newRequested) {
      setNewOpen(true);
      requestNew(false);
    }
  }, [newRequested, requestNew]);

  const openProject = async (p: RecentProject) => {
    if (!p.available) return setUnavailable(p);
    try {
      await call<OpenProjectResult>("project.open", { path: p.path });
      reset();
    } catch (e) {
      reportError(e);
    }
  };

  const openExisting = async () => {
    const dir = await openDialog({ directory: true, title: "Open an OpenFrame project folder" });
    if (typeof dir !== "string") return;
    try {
      await call("project.open", { path: dir });
      reset();
    } catch (e) {
      reportError(e);
    }
  };

  // UX §3.2 shortcuts: Ctrl+N New Project, Ctrl+O Open Project.
  const openExistingRef = useRef(openExisting);
  openExistingRef.current = openExisting;
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey) || e.altKey || dialogOpen) return;
      const k = e.key.toLowerCase();
      if (e.shiftKey) {
        // Ctrl+Shift+N: Quick Capture into the Global Idea Vault (UX §6).
        if (k === "n") {
          e.preventDefault();
          openGlobalVault(true);
        }
        return;
      }
      if (k === "n") {
        e.preventDefault();
        setNewOpen(true);
      } else if (k === "o") {
        e.preventDefault();
        void openExistingRef.current();
      } else if (k === "f" || k === "k") {
        e.preventDefault();
        searchRef.current?.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [dialogOpen]);

  const all = useMemo(() => recents.data ?? [], [recents.data]);
  const q = filter.trim().toLowerCase();
  const active = all.filter((p) => !p.archived);
  const archived = all.filter((p) => p.archived);
  const matches = (p: RecentProject) => !q || p.title.toLowerCase().includes(q) || p.projectType.toLowerCase().includes(q) || p.status.toLowerCase().includes(q);
  const pinned = active.filter((p) => p.pinned && matches(p));
  const recent = active.filter((p) => !p.pinned && matches(p));
  const archivedShown = archived.filter(matches);

  const setArchived = async (p: RecentProject, value: boolean) => {
    try {
      await call("project.set_archived", { projectId: p.projectId, archived: value });
      toast.info(value ? `“${p.title}” was archived. It is not deleted — find it under Open Archived.` : `“${p.title}” is back in your projects.`);
    } catch (e) {
      reportError(e);
    }
  };

  const menuFor = (p: RecentProject): MenuItemSpec[] => [
    { label: "Open", icon: <FolderOpen size={14} />, onSelect: () => void openProject(p) },
    { label: "Rename", shortcut: "F2", icon: <Pencil size={14} />, disabled: !p.available, onSelect: () => setRenaming(p) },
    {
      label: "Duplicate Project",
      icon: <Copy size={14} />,
      disabled: !p.available,
      onSelect: () =>
        void call<RecentProject>("project.duplicate", { projectId: p.projectId })
          .then((c) => toast.success(`Created “${c.title}”. It is a separate project with its own identity.`))
          .catch(reportError),
    },
    {
      label: p.pinned ? "Unpin" : "Pin",
      icon: p.pinned ? <PinOff size={14} /> : <Pin size={14} />,
      onSelect: () => void call("project.set_pinned", { projectId: p.projectId, pinned: !p.pinned }).catch(reportError),
    },
    { label: "Reveal in File Manager", icon: <FolderSearch size={14} />, disabled: !p.available, onSelect: () => void revealLocation("project", p.projectId).catch(reportError) },
    p.archived
      ? { label: "Restore from Archive", icon: <RotateCcw size={14} />, separatorBefore: true, onSelect: () => void setArchived(p, false) }
      : { label: "Archive", icon: <Archive size={14} />, separatorBefore: true, onSelect: () => void setArchived(p, true) },
    { label: "Delete Project…", icon: <Trash2 size={14} />, danger: true, onSelect: () => setDeleting(p) },
  ];

  const noProjects = !recents.isLoading && all.length === 0;
  const profileName = info.data?.profile.displayName ?? "";

  return (
    <div className="app-root">
      <header className="topbar">
        <div className="brand">
          <span className="mark" aria-hidden>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2"><rect x="3" y="5" width="18" height="14" rx="2" /><path d="M7 5v14M17 5v14" /></svg>
          </span>
          OpenFrame
        </div>
        <div className="grow" />
        <div className="tb-actions row">
          <Button size="sm" variant="ghost" icon={<Lightbulb size={14} />} onClick={() => openGlobalVault()} title="Your ideas outside any project (Ctrl+Shift+N for Quick Capture)">
            Global Idea Vault
          </Button>
          <button
            className="iconbtn"
            aria-label="Help"
            title="Shortcuts: Ctrl+N new project · Ctrl+O open project · Ctrl+F search projects · F2 rename · Ctrl+Shift+N new idea"
            onClick={() => toast.info("Shortcuts: Ctrl+N new project · Ctrl+O open project · Ctrl+F search projects · F2 rename the selected project · Ctrl+Shift+N new idea in the Global Idea Vault")}
          >
            <CircleHelp size={16} />
          </button>
          <button className="avatar" style={{ border: 0 }} aria-label={`Your name: ${profileName}. Change`} title={`${profileName} — change your name`} onClick={() => setNameOpen(true)}>
            {initials(profileName)}
          </button>
        </div>
      </header>
      <main className="content" style={{ padding: "24px 32px", overflow: "auto" }} aria-label="Your projects">
        {recents.isLoading ? (
          <div className="col" aria-busy="true">
            <Skeleton h={28} w={220} />
            <div className="grid g4">{[0, 1, 2, 3].map((i) => <Skeleton key={i} h={150} />)}</div>
          </div>
        ) : recents.isError ? (
          <EmptyState title="Your project list couldn't be loaded." actions={<Button onClick={() => void recents.refetch()}>Try Again</Button>}>
            Your projects are safe on this computer. {recents.error?.message}
          </EmptyState>
        ) : noProjects ? (
          <EmptyState
            title="Create your first project."
            actions={
              <>
                <Button variant="primary" icon={<Plus size={15} />} onClick={() => setNewOpen(true)}>Create Project</Button>
                <Button icon={<FolderOpen size={15} />} onClick={() => void openExisting()}>Open Existing Project</Button>
              </>
            }
          >
            A project holds your ideas, story board, screenplay and production plan for one film. Everything is stored on this computer.
          </EmptyState>
        ) : showArchived ? (
          <>
            <PageHeader
              title="Archived projects"
              sub="Archived projects are not deleted. They stay on your computer and can be restored at any time."
              actions={<Button onClick={() => setShowArchived(false)}>Back to projects</Button>}
            />
            <ProjectSearch inputRef={searchRef} value={filter} onChange={setFilter} />
            <div className="card" style={{ overflow: "hidden" }}>
              {archivedShown.length === 0 ? (
                <div className="muted" style={{ padding: 20, textAlign: "center" }}>
                  {archived.length === 0 ? "No archived projects. Archive a project from its menu to tidy your list without deleting anything." : `No archived projects match “${filter}”.`}
                </div>
              ) : (
                <table className="tbl">
                  <thead>
                    <tr><th>Title</th><th>Type</th><th>Status</th><th>Last modified</th><th><span className="sr-only">Actions</span></th></tr>
                  </thead>
                  <tbody>
                    {archivedShown.map((p) => (
                      <ContextMenu key={p.projectId} items={menuFor(p)}>
                        <tr>
                          <td className="b">{p.title}{!p.available && <> <Chip tone="r">Unavailable</Chip></>}</td>
                          <td>{p.projectType}</td>
                          <td><Chip>Archived</Chip></td>
                          <td>{shortDate(lastModified(p))}</td>
                          <td style={{ textAlign: "right", whiteSpace: "nowrap" }}>
                            <Button size="sm" onClick={() => void setArchived(p, false)}>Restore</Button>{" "}
                            <Button size="sm" onClick={() => void openProject(p)}>Open</Button>
                          </td>
                        </tr>
                      </ContextMenu>
                    ))}
                  </tbody>
                </table>
              )}
            </div>
          </>
        ) : (
          <>
            <PageHeader
              title="Your projects"
              sub="Pick up where you left off, or start something new."
              actions={
                <>
                  <ProjectSearch inputRef={searchRef} value={filter} onChange={setFilter} inline />
                  <Button icon={<FolderOpen size={15} />} onClick={() => void openExisting()} title="Open Project (Ctrl+O)">Open Project</Button>
                  <Button variant="primary" icon={<Plus size={15} />} onClick={() => setNewOpen(true)} title="New Project (Ctrl+N)">New Project</Button>
                </>
              }
            />
            {q ? (
              <>
                <div className="h4">Search results</div>
                {pinned.length + recent.length === 0 ? (
                  <div className="muted" style={{ padding: "18px 0" }}>
                    No projects match “{filter}”.{" "}
                    {archivedShown.length > 0 && <Button variant="ghost" size="sm" onClick={() => setShowArchived(true)}>{archivedShown.length} archived match{archivedShown.length === 1 ? "es" : ""} — Open Archived</Button>}
                  </div>
                ) : (
                  <CardGrid projects={[...pinned, ...recent]} menuFor={menuFor} onOpen={openProject} onRename={setRenaming} />
                )}
              </>
            ) : (
              <>
                {pinned.length > 0 && (
                  <section aria-label="Pinned" style={{ marginBottom: 18 }}>
                    <div className="h4">Pinned</div>
                    <CardGrid projects={pinned} menuFor={menuFor} onOpen={openProject} onRename={setRenaming} />
                  </section>
                )}
                <section aria-label="Recent projects">
                  <div className="row" style={{ marginBottom: 6 }}>
                    <div className="h4" style={{ margin: 0 }}>Recent projects</div>
                    <span className="grow" />
                    <Button variant="ghost" size="sm" icon={<Archive size={14} />} onClick={() => setShowArchived(true)}>
                      Open Archived{archived.length > 0 ? ` (${archived.length})` : ""}
                    </Button>
                  </div>
                  {recent.length === 0 ? (
                    <div className="muted sm" style={{ padding: "12px 0" }}>
                      {active.length === 0 ? "All your projects are archived. Open Archived to restore one, or create a new project." : "Every project here is pinned above."}
                    </div>
                  ) : (
                    <CardGrid projects={recent} menuFor={menuFor} onOpen={openProject} onRename={setRenaming} />
                  )}
                </section>
              </>
            )}
          </>
        )}
      </main>
      <footer className="status">
        <span className="it"><span className="dot gray" aria-hidden /> No project open</span>
        <div className="r"><span className="it">Offline · Your projects stay on this computer</span></div>
      </footer>
      {newOpen && <NewProjectDialog onClose={() => setNewOpen(false)} />}
      {unavailable && <UnavailableDialog project={unavailable} onClose={() => setUnavailable(null)} />}
      {renaming && <RenameProjectDialog project={renaming} onClose={() => setRenaming(null)} />}
      {deleting && (
        <DeleteProjectDialog
          project={deleting}
          onClose={() => setDeleting(null)}
          onArchiveInstead={() => {
            void setArchived(deleting, true);
            setDeleting(null);
          }}
        />
      )}
      {(nameOpen || info.data?.displayNameSet === false) && info.data && (
        <DisplayNameDialog firstRun={!info.data.displayNameSet} current={info.data.profile.displayName} onClose={() => setNameOpen(false)} />
      )}
    </div>
  );
}

function ProjectSearch({ value, onChange, inputRef, inline }: { value: string; onChange: (v: string) => void; inputRef: RefObject<HTMLInputElement | null>; inline?: boolean }) {
  return (
    <div className="input" style={{ width: inline ? 220 : 320, marginBottom: inline ? 0 : 12, padding: "0 10px" }}>
      <Search size={15} aria-hidden style={{ color: "var(--muted)" }} />
      <input
        ref={inputRef}
        type="search"
        aria-label="Search projects"
        placeholder="Search projects"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => e.key === "Escape" && onChange("")}
        style={{ border: 0, outline: "none", flex: 1, background: "transparent", minHeight: 30 }}
      />
    </div>
  );
}

function CardGrid({ projects, menuFor, onOpen, onRename }: {
  projects: RecentProject[];
  menuFor: (p: RecentProject) => MenuItemSpec[];
  onOpen: (p: RecentProject) => void;
  onRename: (p: RecentProject) => void;
}) {
  return (
    <div className="grid g4" role="list">
      {projects.map((p) => (
        <ContextMenu key={p.projectId} items={menuFor(p)}>
          <div className="pcard" role="listitem" style={{ position: "relative", opacity: p.available ? 1 : 0.8 }}>
            <button
              className="clickable"
              style={{ display: "block", width: "100%", border: 0, padding: 0, background: "transparent", textAlign: "left" }}
              aria-label={`Open ${p.title}, ${p.projectType}, ${p.status}${p.pinned ? ", pinned" : ""}${p.available ? "" : ", unavailable"}`}
              onClick={() => onOpen(p)}
              onKeyDown={(e) => {
                if (e.key === "F2" && p.available) {
                  e.preventDefault();
                  onRename(p);
                }
              }}
            >
              <div className={`pv photo ${previewFor(p.projectId)}`} aria-hidden />
              <div className="pb">
                <div className="pt" style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{p.title}</div>
                <div className="row gap4 wrap" style={{ margin: "6px 0" }}>
                  <Chip>{p.projectType}</Chip>
                  <Chip tone="b">{p.status}</Chip>
                  {p.pinned && <Chip tone="a"><Pin size={10} aria-hidden /> Pinned</Chip>}
                  {!p.available && <Chip tone="r">Unavailable</Chip>}
                </div>
                <div className="xs muted">Last modified {ago(lastModified(p))}</div>
              </div>
            </button>
            <div style={{ position: "absolute", top: 6, right: 6 }}>
              <Menu
                align="end"
                items={menuFor(p)}
                trigger={
                  <button className="iconbtn" aria-label={`More actions for ${p.title}`} style={{ background: "#ffffffd9" }}>
                    <MoreHorizontal size={16} />
                  </button>
                }
              />
            </div>
          </div>
        </ContextMenu>
      ))}
    </div>
  );
}

function RenameProjectDialog({ project, onClose }: { project: RecentProject; onClose: () => void }) {
  const [title, setTitle] = useState(project.title);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const invalid = title.trim().length === 0;
  const save = async () => {
    if (invalid || busy) return;
    if (title.trim() === project.title) return onClose();
    setBusy(true);
    try {
      await call("project.rename", { projectId: project.projectId, title: title.trim() });
      toast.success(`Renamed to “${title.trim()}”.`);
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : "The project could not be renamed.");
    } finally {
      setBusy(false);
    }
  };
  return (
    <Dialog open onOpenChange={(v) => !v && onClose()} title="Rename Project" size="sm"
      footer={<><Button onClick={onClose}>Cancel</Button><Button variant="primary" disabled={invalid || busy} onClick={() => void save()}>{busy ? "Renaming…" : "Rename"}</Button></>}>
      <form onSubmit={(e) => { e.preventDefault(); void save(); }}>
        <Field label="Title" required htmlFor="rp-title" error={invalid ? "A title is required." : error}>
          <TextInput id="rp-title" autoFocus value={title} maxLength={200} invalid={invalid} onChange={(e) => { setTitle(e.target.value); setError(null); }} onFocus={(e) => e.currentTarget.select()} />
        </Field>
        <div className="hint">Renaming changes the name shown everywhere. The project folder and its contents stay where they are.</div>
      </form>
    </Dialog>
  );
}

/** Mock 023: the most destructive action — typed-name confirmation, Archive Instead offered. */
function DeleteProjectDialog({ project, onClose, onArchiveInstead }: { project: RecentProject; onClose: () => void; onArchiveInstead: () => void }) {
  const [typed, setTyped] = useState("");
  const [busy, setBusy] = useState(false);
  const matches = typed.trim() === project.title.trim();
  const remove = async () => {
    if (!matches || busy) return;
    setBusy(true);
    try {
      await call("project.delete", { projectId: project.projectId });
      toast.info(`“${project.title}” was moved to the Windows Recycle Bin.`);
      onClose();
    } catch (e) {
      reportError(e);
      setBusy(false);
    }
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title={`Delete “${project.title}”?`}
      size="md"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          {!project.archived && <Button onClick={onArchiveInstead}>Archive Instead</Button>}
          <Button variant="danger" disabled={!matches || busy} onClick={() => void remove()}>{busy ? "Deleting…" : "Delete Project"}</Button>
        </>
      }
    >
      <Banner tone="err"><b>This is the most destructive action in OpenFrame. The whole project will be removed from this computer.</b></Banner>
      <p className="sm" style={{ margin: "10px 0" }}>
        The project folder — ideas, story, screenplay, production plans and stored files — is moved to the Windows Recycle Bin. Files linked from other folders are not touched.
      </p>
      <Field label="Type the project name to confirm" htmlFor="dp-confirm" hint="Archiving instead keeps everything and simply hides the project from your recent list.">
        <TextInput id="dp-confirm" autoFocus autoComplete="off" placeholder={project.title} value={typed} onChange={(e) => setTyped(e.target.value)} onKeyDown={(e) => e.key === "Enter" && void remove()} />
      </Field>
    </Dialog>
  );
}

function UnavailableDialog({ project, onClose }: { project: RecentProject; onClose: () => void }) {
  const reset = useNav((s) => s.reset);
  const locate = async () => {
    const dir = await openDialog({ directory: true, title: `Locate “${project.title}”` });
    if (typeof dir !== "string") return;
    try {
      await call("project.locate", { projectId: project.projectId, path: dir });
      reset();
      onClose();
    } catch (e) {
      reportError(e);
    }
  };
  const retry = async () => {
    try {
      await call("project.open", { path: project.path });
      reset();
      onClose();
    } catch (e) {
      reportError(e);
    }
  };
  const forget = async () => {
    try {
      await call("project.remove_recent", { projectId: project.projectId });
      toast.info(`“${project.title}” was removed from the list. Nothing was deleted.`);
      onClose();
    } catch (e) {
      reportError(e);
    }
  };
  return (
    <Dialog open onOpenChange={(v) => !v && onClose()} title="This project could not be opened" size="sm"
      footerLeft={<Button variant="ghost" size="sm" onClick={() => void forget()}>Remove from List</Button>}
      footer={<><Button onClick={onClose}>Cancel</Button><Button icon={<FolderSearch size={14} />} onClick={() => void locate()}>Locate Project…</Button><Button variant="primary" icon={<RotateCcw size={14} />} onClick={() => void retry()}>Try Again</Button></>}>
      <Banner tone="err"><b>The project location is unavailable. It may be on an external drive that is not connected, or the folder was moved.</b></Banner>
      <p className="muted" style={{ margin: "10px 0 6px" }}>
        Project: <b>{project.title}</b>
        <br />
        Last known location: <b className="selectable-text" style={{ wordBreak: "break-all" }}>{project.path}</b>
      </p>
      <p style={{ margin: 0 }}>OpenFrame has not created a replacement project. Reconnect the drive and try again, or locate another copy of the project.</p>
    </Dialog>
  );
}
