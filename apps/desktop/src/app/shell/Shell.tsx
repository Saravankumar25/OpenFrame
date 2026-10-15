// Application shell (UX §2): top bar, left navigation, active workspace, status bar.
// The shell tells the user which project is open, gives stable navigation and
// exposes universal actions; it is not a dashboard.

import { Suspense, useEffect, useMemo, useRef, useState, type ComponentType } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  Check,
  ChevronDown,
  CircleHelp,
  FolderOpen,
  HardDrive,
  History,
  Plus,
  Redo2,
  RotateCcw,
  Save,
  Search,
  Sparkles,
  Undo2,
} from "lucide-react";
import { call, inTauri, revealLocation } from "../../ipc/client";
import { reportError, useOp } from "../../ipc/query";
import type { ProjectSummary } from "../../ipc/generated/ProjectSummary";
import type { UndoInfo } from "../../ipc/generated/UndoInfo";
import type { RecentProject } from "../../ipc/generated/RecentProject";
import type { AppInfo } from "../../ipc/generated/AppInfo";
import { Button, Dot, Menu, Skeleton, type MenuItemSpec } from "../../design-system";
import { useIntent, useNav, useUi } from "../stores";
import { WORKSPACES, workspaceDef } from "../routes";
import { toast } from "../toast";
import { flushPendingSaves } from "../pendingSaves";
import { SearchOverlay } from "./SearchOverlay";
import { ErrorBoundary } from "./ErrorBoundary";
import { useAppEvents } from "../useAppEvents";
import { TrialCorner } from "../../features/licensing/TrialCorner";
import { matches, useShortcutLabel } from "../shortcuts";
import { useHomeView } from "../home/homeState";
import { DisplayNameDialog } from "../home/DisplayNameDialog";
import { dayAndTime } from "../home/format";
import { OpenRecoveryDialog, RecoveryOffer, useRecoveryState } from "./RecoveryDialogs";
import { LiveSaveErrorBanner, LiveSaveErrorDialog, guardedLeave, retrySave, useLiveSaveState } from "./SaveStatus";
import { BrandLogo } from "../../theme/BrandLogo";

const aiPanelModules = import.meta.glob<{ default: ComponentType<{ onClose: () => void }> }>("../../ai/AiPanel.tsx", { eager: true });
const AiPanel = Object.values(aiPanelModules)[0]?.default;

/** Optional shell extensions (./extensions/*.tsx): extra project-menu items and an overlay host. */
interface ShellExtension {
  projectMenuItems?: (helpers: { project: ProjectSummary | null }) => MenuItemSpec[];
  ShellOverlay?: ComponentType;
}
const shellExtensions = Object.values(import.meta.glob<ShellExtension>("./extensions/*.tsx", { eager: true }));

function isEditable(t: EventTarget | null): boolean {
  const el = t as HTMLElement | null;
  if (!el) return false;
  return el.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(el.tagName);
}

export async function runUndo(store: "project" | "global", redo = false) {
  try {
    const r = await call<{ label: string | null }>(redo ? "history.redo" : "history.undo", { store });
    if (r.label) toast.info(`${redo ? "Redid" : "Undid"}: ${r.label}`);
  } catch (e) {
    reportError(e);
  }
}

export async function saveNow() {
  await retrySave();
}

/** Close the open project and show Application Home (optionally the archived list or New Project). */
function leaveToHome(project: ProjectSummary | null, opts: { archived?: boolean; newProject?: boolean } = {}) {
  void guardedLeave(project?.title ?? "this project", async () => {
    useHomeView.getState().setShowArchived(!!opts.archived);
    if (opts.newProject) useHomeView.getState().requestNewProject(true);
    await call("project.close");
  });
}

function initials(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  return ((parts[0]?.[0] ?? "") + (parts.length > 1 ? parts[parts.length - 1][0] : "")).toUpperCase() || "?";
}

export function Shell() {
  const project = useOp<ProjectSummary | null>("project.current", {}, ["project"]);
  const route = useNav((s) => s.route);
  const go = useNav((s) => s.go);
  const { searchOpen, setSearchOpen, aiOpen, setAiOpen, undoScope } = useUi();
  const def = workspaceDef(route.workspace);
  const Workspace = def.component;
  // Save state is read by the status bar, banner and dialog themselves: the Shell
  // (and with it the active workspace) must not re-render on every save event.
  const [recoveryOpen, setRecoveryOpen] = useState(false);
  const recovery = useRecoveryState(!!project.data);
  const offerPending = !!recovery.data?.offer;
  useAppEvents();

  // Window title mirrors the open project (UX §2.1).
  useEffect(() => {
    const title = project.data ? `${project.data.title} - OpenFrame Studio` : "OpenFrame Studio";
    // The document title too: assistive technology announces it.
    document.title = title;
    if (!inTauri()) return;
    void getCurrentWindow().setTitle(title);
  }, [project.data]);

  // Global keyboard shortcuts (UX §6), as set in Settings > Keyboard shortcuts.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      // The browser's own Ctrl+N (new window) is always suppressed.
      if ((e.ctrlKey || e.metaKey) && !e.altKey && (e.key.toLowerCase() === "n" || e.code === "KeyN")) e.preventDefault();
      if (matches(e, "app.quick_capture")) {
        // Quick Capture / New Idea from anywhere (UX §6). The Idea Vault handles it
        // itself while it is showing.
        e.preventDefault();
        if (useNav.getState().route.workspace !== "vault") {
          useIntent.getState().fire("vault.new_note");
          useNav.getState().go({ workspace: "vault" });
        }
      } else if (matches(e, "app.new_scene_card")) {
        // New Scene Card (quick actions menu, UX §6). Never inside a text field.
        e.preventDefault();
        if (!isEditable(e.target) && !document.querySelector(".scrim")) {
          useIntent.getState().fire("story.new_scene");
          // The Story workspace places the card (switching to the Board if needed).
          if (useNav.getState().route.workspace !== "story") useNav.getState().go({ workspace: "story" });
        }
      } else if (matches(e, "app.save")) {
        e.preventDefault();
        void saveNow();
      } else if (matches(e, "app.search")) {
        e.preventDefault();
        setSearchOpen(true);
      } else if (matches(e, "app.assistant")) {
        e.preventDefault();
        setAiOpen(!useUi.getState().aiOpen);
      } else if (!isEditable(e.target) && !e.defaultPrevented) {
        if (matches(e, "app.undo")) {
          e.preventDefault();
          void runUndo(useUi.getState().undoScope);
        } else if (matches(e, "app.redo")) {
          e.preventDefault();
          void runUndo(useUi.getState().undoScope, true);
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [setSearchOpen, setAiOpen]);

  return (
    <div className="app-root">
      <TopBar project={project.data ?? null} undoScope={undoScope} />
      <div className="layout">
        <nav className="nav" aria-label="Workspaces">
          {WORKSPACES.filter((w) => w.nav).map((w) => {
            const Icon = w.icon;
            return (
              <button
                key={w.id}
                className={`navitem${route.workspace === w.id ? " active" : ""}`}
                aria-current={route.workspace === w.id ? "page" : undefined}
                onClick={() => void flushPendingSaves().then(() => go({ workspace: w.id }))}
              >
                {Icon && <Icon size={16} />}
                {w.label}
              </button>
            );
          })}
          <div className="foot">
            Project stored locally
            <br />
            on this PC
          </div>
        </nav>
        <main className="main" aria-label={def.label}>
          <LiveSaveErrorBanner onOpenRecovery={() => setRecoveryOpen(true)} />
          <ErrorBoundary key={route.workspace}>
            <Suspense fallback={<div className="content"><Skeleton h={28} w={240} /></div>}>
              <Workspace />
            </Suspense>
          </ErrorBoundary>
        </main>
        {/* The AI Assistant docks beside the page (owner decision 2026-10-08): opening it narrows
            the page instead of covering it, and closing gives the space back. It follows the page
            in the keyboard focus order. */}
        {aiOpen && AiPanel && <AiPanel onClose={() => setAiOpen(false)} />}
      </div>
      <StatusBar project={project.data ?? null} onOpenRecovery={() => setRecoveryOpen(true)} />
      {searchOpen && <SearchOverlay onClose={() => setSearchOpen(false)} />}
      <RecoveryOffer project={project.data ?? null} state={recovery.data} />
      {/* One modal at a time (UX §2.10): the recovery decision comes first. */}
      {!offerPending && <LiveSaveErrorDialog />}
      {recoveryOpen && !offerPending && <OpenRecoveryDialog onClose={() => setRecoveryOpen(false)} />}
      {shellExtensions.map((e, i) => (e.ShellOverlay ? <e.ShellOverlay key={i} /> : null))}
    </div>
  );
}

function TopBar({ project, undoScope }: { project: ProjectSummary | null; undoScope: "project" | "global" }) {
  const keys = {
    search: useShortcutLabel("app.search"),
    save: useShortcutLabel("app.save"),
    undo: useShortcutLabel("app.undo"),
    redo: useShortcutLabel("app.redo"),
    assistant: useShortcutLabel("app.assistant"),
    capture: useShortcutLabel("app.quick_capture"),
    newCard: useShortcutLabel("app.new_scene_card"),
  };
  const withKey = (label: string, key: string) => (key ? `${label} (${key})` : label);
  const shortcutSummary = [
    keys.search && `${keys.search} search`,
    keys.save && `${keys.save} save`,
    keys.undo && `${keys.undo} undo`,
    keys.redo && `${keys.redo} redo`,
    keys.assistant && `${keys.assistant} assistant`,
  ]
    .filter(Boolean)
    .join(" · ");
  const go = useNav((s) => s.go);
  const reset = useNav((s) => s.reset);
  const fire = useIntent((s) => s.fire);
  const { setSearchOpen, aiOpen, setAiOpen } = useUi();
  const undo = useOp<UndoInfo>("history.info", { store: undoScope }, ["*"], { store: undoScope });
  const recents = useOp<RecentProject[]>("project.list_recent", {}, ["project"], { staleTime: 5_000 });
  const info = useOp<AppInfo>("app.info", {}, []);
  const [nameOpen, setNameOpen] = useState(false);

  const quick: MenuItemSpec[] = [
    { label: "Create", header: true },
    { label: "New Idea", shortcut: keys.capture || undefined, onSelect: () => { fire("vault.new_note"); go({ workspace: "vault" }); } },
    { label: "New Act", onSelect: () => { fire("story.new_act"); go({ workspace: "story" }); } },
    { label: "New Sequence", onSelect: () => { fire("story.new_sequence"); go({ workspace: "story" }); } },
    { label: "New Beat", onSelect: () => { fire("story.new_beat"); go({ workspace: "story" }); } },
    { label: "New Scene Card", shortcut: keys.newCard || undefined, onSelect: () => { fire("story.new_scene"); go({ workspace: "story" }); } },
    { label: "New Character", onSelect: () => { fire("story.new_character"); go({ workspace: "story", sub: "characters" }); } },
    { label: "New Location", onSelect: () => { fire("production.new_location"); go({ workspace: "production", sub: "locations" }); } },
    { label: "New Shot List", onSelect: () => { fire("production.new_shot"); go({ workspace: "production", sub: "shots" }); } },
    { label: "New Moodboard", onSelect: () => { fire("production.new_moodboard"); go({ workspace: "production", sub: "moodboards" }); } },
    { label: "Import Screenplay…", separatorBefore: true, onSelect: () => { fire("screenplay.import"); go({ workspace: "screenplay" }); } },
    { label: "New Note", separatorBefore: true, onSelect: () => { fire("notes.new_note"); go({ workspace: "notes" }); } },
    { label: "New Task", onSelect: () => { fire("notes.new_task"); go({ workspace: "notes" }); } },
    { label: "New Project…", separatorBefore: true, onSelect: () => leaveToHome(project, { newProject: true }) },
  ];

  const switchTo = (r: RecentProject) =>
    void guardedLeave(project?.title ?? "this project", async () => {
      await call("project.open", { path: r.path });
      reset();
    });

  const others = (recents.data ?? []).filter((r) => r.projectId !== project?.id);
  const recentRows = others.filter((r) => !r.pinned).slice(0, 5);
  const pinnedRows = others.filter((r) => r.pinned).slice(0, 5);
  const row = (r: RecentProject): MenuItemSpec => ({
    label: r.title,
    shortcut: r.available ? `${r.projectType} · ${r.status}` : "Unavailable",
    disabled: !r.available,
    onSelect: () => switchTo(r),
  });
  const projectMenu: MenuItemSpec[] = [
    { label: "Recent projects", header: true },
    ...(project ? [{ label: project.title, shortcut: "Open", icon: <Check size={14} />, onSelect: () => go({ workspace: "home" }) }] : []),
    ...recentRows.map(row),
    ...(pinnedRows.length ? [{ label: "Pinned", header: true, separatorBefore: true } as MenuItemSpec, ...pinnedRows.map(row)] : []),
    { label: "Project settings", separatorBefore: true, onSelect: () => go({ workspace: "settings" }) },
    { label: "Notes & Tasks", onSelect: () => go({ workspace: "notes" }) },
    { label: "Activity", onSelect: () => go({ workspace: "activity" }) },
    { label: "Recently Deleted", onSelect: () => go({ workspace: "trash" }) },
    { label: "Templates", onSelect: () => go({ workspace: "settings", sub: "templates" }) },
    { label: "Appearance", onSelect: () => go({ workspace: "settings", sub: "appearance" }) },
    ...shellExtensions.flatMap((e) => e.projectMenuItems?.({ project }) ?? []),
    { label: "All projects…", separatorBefore: true, onSelect: () => leaveToHome(project) },
    { label: "Archived…", onSelect: () => leaveToHome(project, { archived: true }) },
    { label: "Close project", onSelect: () => leaveToHome(project) },
  ];

  const name = info.data?.profile.displayName ?? "";
  return (
    <header className="topbar">
      <div className="brand">
        <BrandLogo />
      </div>
      <Menu
        trigger={
          <button className="projsel" aria-label={`Project: ${project?.title ?? ""}. Switch or manage projects`}>
            <span className="col" style={{ gap: 0, alignItems: "flex-start", minWidth: 0 }}>
              <span style={{ maxWidth: 220, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{project?.title ?? "…"}</span>
              <small>{project?.projectType}</small>
            </span>
            <span className="grow" />
            <ChevronDown size={14} />
          </button>
        }
        items={projectMenu}
      />
      <button className="search" onClick={() => setSearchOpen(true)} aria-label="Search this project" style={{ cursor: "text" }}>
        <Search size={15} />
        <span className="grow" style={{ textAlign: "left" }}>Search this project…</span>
        {keys.search && <kbd>{keys.search}</kbd>}
      </button>
      <div className="tb-actions row">
        <TrialCorner />
        <Menu trigger={<button className="iconbtn" aria-label="Create" title="Create"><Plus size={17} /></button>} items={quick} align="end" />
        <button className="iconbtn" aria-label={undo.data?.undoLabel ? `Undo ${undo.data.undoLabel}` : "Undo"} title={undo.data?.undoLabel ? withKey(`Undo: ${undo.data.undoLabel}`, keys.undo) : "Nothing to undo"} disabled={!undo.data?.canUndo} onClick={() => void runUndo(undoScope)}>
          <Undo2 size={16} />
        </button>
        <button className="iconbtn" aria-label={undo.data?.redoLabel ? `Redo ${undo.data.redoLabel}` : "Redo"} title={undo.data?.redoLabel ? withKey(`Redo: ${undo.data.redoLabel}`, keys.redo) : "Nothing to redo"} disabled={!undo.data?.canRedo} onClick={() => void runUndo(undoScope, true)}>
          <Redo2 size={16} />
        </button>
        <button className={`iconbtn${aiOpen ? " on" : ""}`} aria-label="Assistant" aria-pressed={aiOpen} title={withKey("Assistant", keys.assistant)} onClick={() => setAiOpen(!aiOpen)}>
          <Sparkles size={16} />
        </button>
        <button
          className="iconbtn"
          aria-label="Help"
          title={`Keyboard shortcuts: ${shortcutSummary}`}
          onClick={() =>
            toast.info(`Shortcuts: ${shortcutSummary}`, {
              label: "Change shortcuts",
              run: () => useNav.getState().go({ workspace: "settings", sub: "shortcuts" }),
            })
          }
        >
          <CircleHelp size={16} />
        </button>
        <button className="avatar" style={{ border: 0 }} aria-label={`Your name: ${name}. Change`} title={`${name}: change your name`} onClick={() => setNameOpen(true)}>
          {initials(name)}
        </button>
      </div>
      {nameOpen && info.data && <DisplayNameDialog firstRun={false} current={info.data.profile.displayName} onClose={() => setNameOpen(false)} />}
    </header>
  );
}

function StatusBar({ project, onOpenRecovery }: { project: ProjectSummary | null; onOpenRecovery: () => void }) {
  const save = useLiveSaveState();
  const route = useNav((s) => s.route);
  const [open, setOpen] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const def = workspaceDef(route.workspace);
  const label = useMemo(() => {
    switch (save?.status) {
      case "Saving": return "Saving";
      case "Error": return "Save error";
      case "SavedPendingExternal": return "Saved · finishing export";
      default: return "Saved";
    }
  }, [save?.status]);
  const tone = save?.status === "Error" ? "red" : save?.status === "Saving" || save?.status === "SavedPendingExternal" ? "amber" : "green";
  const crumb =
    route.workspace === "home" && project
      ? [project.title, project.projectType, project.status].join(" · ")
      : def.label + (route.sub ? ` · ${route.sub.charAt(0).toUpperCase()}${route.sub.slice(1)}` : "");
  const right = save?.status === "Error" ? "Offline · Local save failed" : "Offline · Local Project Saved";

  // Escape or a click elsewhere closes the panel and returns focus to its button.
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setOpen(false);
        buttonRef.current?.focus();
      }
    };
    const onDown = (e: MouseEvent) => {
      if (!panelRef.current?.contains(e.target as Node) && !buttonRef.current?.contains(e.target as Node)) setOpen(false);
    };
    window.addEventListener("keydown", onKey);
    window.addEventListener("mousedown", onDown);
    panelRef.current?.querySelector<HTMLButtonElement>("button")?.focus();
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("mousedown", onDown);
    };
  }, [open]);

  return (
    <footer className="status" style={{ position: "relative" }}>
      <button ref={buttonRef} className="it" style={{ background: "transparent", border: 0, padding: 0 }} onClick={() => setOpen(!open)} aria-expanded={open} aria-haspopup="dialog" aria-label={`Save status: ${label}. Show project status`}>
        <Dot tone={tone} />
        <span aria-live="polite">{label}</span>
      </button>
      <span>{crumb}</span>
      <div className="r">
        <span className="it"><HardDrive size={13} aria-hidden /> {right}</span>
      </div>
      {open && project && (
        <div ref={panelRef} className="menu-pop" style={{ position: "absolute", bottom: 30, left: 8, width: 360, padding: 12 }} role="dialog" aria-label="Project status">
          <div className="h4">Project status</div>
          <div className="row" style={{ marginBottom: 6 }}>
            <Dot tone={tone} /> <b>{label}</b>
            <span className="muted xs">{save?.lastSavedAt ? dayAndTime(save.lastSavedAt) : "Just now"}</span>
          </div>
          {save?.status === "Error" && save.error && <div className="errtxt" style={{ marginBottom: 6 }}>{save.error.message}</div>}
          <div className="row sm" style={{ marginBottom: 4 }}><HardDrive size={14} aria-hidden /> <span>Offline: your project is stored locally.</span></div>
          <div className="row sm muted" style={{ marginBottom: 10, alignItems: "flex-start" }}>
            <FolderOpen size={14} aria-hidden style={{ flex: "none", marginTop: 2 }} />
            <span className="selectable-text" style={{ wordBreak: "break-all" }}>{project.path}</span>
          </div>
          <div className="ms" style={{ height: 1, background: "var(--line)", margin: "0 0 10px" }} />
          <div className="row wrap">
            <Button size="sm" variant={save?.status === "Error" ? "primary" : "default"} icon={<Save size={13} />} onClick={() => void saveNow()}>
              {save?.status === "Error" ? "Retry Save" : "Save now"}
            </Button>
            <Button size="sm" icon={<RotateCcw size={13} />} onClick={() => { setOpen(false); onOpenRecovery(); }}>Open recovery</Button>
            <Button size="sm" icon={<FolderOpen size={13} />} onClick={() => void revealLocation("project").catch(reportError)}>Open project location</Button>
            <Button size="sm" variant="ghost" icon={<History size={13} />} onClick={() => { setOpen(false); useNav.getState().go({ workspace: "activity" }); }}>Activity</Button>
          </div>
        </div>
      )}
    </footer>
  );
}
