// Project Home (FSD §6, §115; UX §3.4; mocks 011, 012, 015, 022).
// "What do I want to continue doing in this film?"
//
// * Brand-new project: exactly three starting actions and no fake metrics (FSD §6.3).
// * Otherwise: Continue (last meaningful location per workspace, FSD §6.2),
//   Quick Access, Recent (from activity), Production status (only when
//   production content exists, FSD §6.4) and the Project Files card.

import { useState, type ComponentType } from "react";
import {
  Archive,
  ArrowRight,
  Clapperboard,
  ClipboardList,
  Columns3,
  FileText,
  Folder,
  History,
  Lightbulb,
  ListChecks,
  ListTodo,
  Plus,
  Settings,
  Trash2,
} from "lucide-react";
import { useOp } from "../../ipc/query";
import type { ProjectHome } from "../../ipc/generated/ProjectHome";
import type { FilesListing } from "../../ipc/generated/FilesListing";
import type { TaskDto } from "../../ipc/generated/TaskDto";
import type { ActivityEntry } from "../../ipc/generated/ActivityEntry";
import { Banner, Button, Chip, EmptyState, PageHeader, Skeleton } from "../../design-system";
import { navigateTo } from "../../app/navigate";
import { useNav, type WorkspaceId } from "../../app/stores";
import { ago, dayAndTime, fileSize } from "../../app/home/format";
import { StatusMenu, setProjectArchived } from "./StatusMenu";
import { ProjectSettingsDialog } from "../settings/ProjectSettingsForm";
import { FILE_TABLES, fileBadge, pickAndAddFiles } from "../files/fileUtils";

type Icon = ComponentType<{ size?: number; "aria-hidden"?: boolean }>;

const CONTINUE_LABEL: Record<string, [string, Icon]> = {
  vault: ["Continue Idea Vault", Lightbulb],
  story: ["Continue Story Board", Columns3],
  screenplay: ["Continue screenplay", FileText],
  breakdown: ["Continue breakdown", ListChecks],
  production: ["Continue production", Clapperboard],
  callsheets: ["Continue call sheet", ClipboardList],
  files: ["Continue files", Folder],
  notes: ["Continue notes", ListTodo],
};

const QUICK: { label: string; workspace: WorkspaceId; icon: Icon }[] = [
  { label: "Idea Vault", workspace: "vault", icon: Lightbulb },
  { label: "Story Board", workspace: "story", icon: Columns3 },
  { label: "Screenplay", workspace: "screenplay", icon: FileText },
  { label: "Breakdown", workspace: "breakdown", icon: ListChecks },
  { label: "Production", workspace: "production", icon: Clapperboard },
  { label: "Call Sheets", workspace: "callsheets", icon: ClipboardList },
];

const PRODUCTION_KEYS = ["breakdownItems", "locations", "shots", "shootingDays", "callSheets"];

function continueTitle(loc: Record<string, unknown>): [string, Icon] {
  const ws = String(loc.workspace ?? "");
  if (ws === "production" && loc.sub === "schedule") return ["Continue schedule", Clapperboard];
  return CONTINUE_LABEL[ws] ?? ["Continue", ArrowRight];
}

export default function ProjectHomeWorkspace() {
  const home = useOp<ProjectHome>("project.home", {}, ["*"], { refetchOnMount: "always" });
  const [settingsOpen, setSettingsOpen] = useState(false);

  if (home.isLoading) {
    return (
      <div className="content" aria-busy="true">
        <Skeleton h={28} w={260} />
        <div style={{ height: 12 }} />
        <Skeleton h={120} />
      </div>
    );
  }
  if (!home.data) {
    return (
      <div className="content">
        <EmptyState title="Project Home couldn't be displayed." actions={<Button onClick={() => void home.refetch()}>Try Again</Button>}>
          Your project is safe. {home.error?.message}
        </EmptyState>
      </div>
    );
  }
  const h = home.data;
  const p = h.project;
  return (
    <div className="content" style={{ overflow: "auto" }}>
      {h.isEmpty ? <NewProjectHome home={h} onSettings={() => setSettingsOpen(true)} /> : <ActiveProjectHome home={h} onSettings={() => setSettingsOpen(true)} />}
      {p.archived && (
        <div style={{ marginTop: 14 }}>
          <Banner tone="info" actions={<Button size="sm" onClick={() => void setProjectArchived(p, false)}>Restore to Active</Button>}>
            This project is archived. Nothing is deleted — it is only hidden from your recent projects.
          </Banner>
        </div>
      )}
      {settingsOpen && <ProjectSettingsDialog onClose={() => setSettingsOpen(false)} />}
    </div>
  );
}

/** FSD §6.3 / mock 011: exactly three starting actions, no metrics. */
function NewProjectHome({ home, onSettings }: { home: ProjectHome; onSettings: () => void }) {
  const p = home.project;
  const go = useNav((s) => s.go);
  const actions: { label: string; text: string; icon: Icon; run: () => void; primary?: boolean }[] = [
    { label: "Open Idea Vault", text: "Collect thoughts, images, links and notes about the film — nothing needs to be organised yet.", icon: Lightbulb, run: () => go({ workspace: "vault" }), primary: true },
    { label: "Build Story", text: "Arrange acts, beats and scene cards on the Story Board until the story works.", icon: Columns3, run: () => go({ workspace: "story" }) },
    { label: "Write/Import Screenplay", text: "Start writing, or import a screenplay you already have.", icon: FileText, run: () => go({ workspace: "screenplay" }) },
  ];
  return (
    <>
      <PageHeader
        title={p.title}
        sub={
          <span className="row" style={{ gap: 6, flexWrap: "wrap" }}>
            {p.projectType} · Status: <StatusMenu project={p} /> · Created {ago(p.createdAt)}
          </span>
        }
        actions={<Button icon={<Settings size={15} />} onClick={onSettings}>Project settings</Button>}
      />
      <div className="card pad" style={{ padding: "18px 20px" }}>
        <div className="b" style={{ fontSize: 15 }}>Start with your ideas, build your story, or open a screenplay.</div>
        <div className="muted" style={{ marginBottom: 14 }}>Pick whichever fits where you are. You can move between them at any time.</div>
        <div className="grid g3" role="list" aria-label="Ways to begin">
          {actions.map((a) => (
            <button key={a.label} role="listitem" className="card pad clickable" style={{ textAlign: "left", display: "flex", flexDirection: "column", gap: 8, borderColor: a.primary ? "#f0d3ae" : undefined }} onClick={a.run}>
              <span className="iconbtn on" aria-hidden><a.icon size={16} aria-hidden /></span>
              <span className="b">{a.label}</span>
              <span className="muted sm">{a.text}</span>
            </button>
          ))}
        </div>
      </div>
    </>
  );
}

function ActiveProjectHome({ home, onSettings }: { home: ProjectHome; onSettings: () => void }) {
  const p = home.project;
  const go = useNav((s) => s.go);
  const sub = [p.projectType, p.language, p.genre, `Last opened ${dayAndTime(home.openedAt)}`].filter(Boolean).join(" · ");
  const production = home.counts.filter((c) => PRODUCTION_KEYS.includes(c.key));
  const writing = home.counts.filter((c) => !PRODUCTION_KEYS.includes(c.key));
  const continues = home.continueLocations.slice(0, 4);
  const recent = home.recentActivity.filter((a) => !a.action.startsWith("project.") && !a.action.startsWith("trash.")).slice(0, 6);
  return (
    <>
      <PageHeader
        title={p.title}
        sub={sub}
        actions={
          <>
            <StatusMenu project={p} />
            <Button icon={<Settings size={15} />} onClick={onSettings}>Project settings</Button>
          </>
        }
      />
      <div className="grid" style={{ gridTemplateColumns: "minmax(0,1.25fr) minmax(0,1fr)", gap: 16, alignItems: "start" }}>
        <div className="col" style={{ gap: 16 }}>
          {continues.length > 0 && (
            <section aria-label="Continue">
              <div className="h4">Continue</div>
              <div className="grid g2">
                {continues.map((loc, i) => {
                  const [title, LocIcon] = continueTitle(loc);
                  return (
                    <button key={i} className="card pad row clickable" style={{ textAlign: "left" }} onClick={() => navigateTo(loc)}>
                      <span className="iconbtn on" aria-hidden><LocIcon size={16} aria-hidden /></span>
                      <span className="grow">
                        <span className="b" style={{ display: "block" }}>{title}</span>
                        <span className="xs muted" style={{ display: "block", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{String(loc.label ?? "")}</span>
                      </span>
                      <ArrowRight size={15} aria-hidden />
                    </button>
                  );
                })}
              </div>
            </section>
          )}
          <section aria-label="Quick Access">
            <div className="h4">Quick Access</div>
            <div className="row wrap">
              {QUICK.map((q) => (
                <Button key={q.workspace} icon={<q.icon size={15} aria-hidden />} onClick={() => go({ workspace: q.workspace })}>{q.label}</Button>
              ))}
            </div>
            <div className="row wrap" style={{ marginTop: 6 }}>
              <Button variant="ghost" size="sm" icon={<ListTodo size={14} aria-hidden />} onClick={() => go({ workspace: "notes" })}>Notes & Tasks</Button>
              <Button variant="ghost" size="sm" icon={<History size={14} aria-hidden />} onClick={() => go({ workspace: "activity" })}>Activity</Button>
              <Button variant="ghost" size="sm" icon={<Trash2 size={14} aria-hidden />} onClick={() => go({ workspace: "trash" })}>Recently Deleted</Button>
              <Button variant="ghost" size="sm" icon={<Archive size={14} aria-hidden />} onClick={() => go({ workspace: "settings", sub: "templates" })}>Templates</Button>
            </div>
          </section>
          <section aria-label="Recent">
            <div className="h4">Recent</div>
            <div className="card">
              {recent.length === 0 ? (
                <div className="li muted sm">Recent changes to scenes, cards, drafts and documents will appear here.</div>
              ) : (
                recent.map((a) => <RecentRow key={a.id} entry={a} />)
              )}
            </div>
          </section>
        </div>
        <div className="col" style={{ gap: 16 }}>
          {production.length > 0 && (
            <section className="card pad" aria-label="Production status">
              <div className="h4">Production status</div>
              <div className="col" style={{ gap: 6 }}>
                {[...writing.filter((c) => c.key === "scenes" || c.key === "drafts"), ...production].map((c) => (
                  <div key={c.key} className="row sm">
                    <span className="grow">{c.label}</span>
                    <b>{c.count}</b>
                  </div>
                ))}
              </div>
              <div className="row" style={{ marginTop: 10 }}>
                <Button size="sm" icon={<Clapperboard size={14} aria-hidden />} onClick={() => go({ workspace: "production" })}>Open Production</Button>
              </div>
            </section>
          )}
          <FilesCard />
          <TasksCard />
        </div>
      </div>
    </>
  );
}

function RecentRow({ entry }: { entry: ActivityEntry }) {
  const meta = `${entry.actorName ? `${entry.actorName} · ` : ""}${ago(entry.at)}`;
  const body = (
    <>
      <span className="grow" style={{ minWidth: 0 }}>
        <b style={{ display: "block", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", fontWeight: 600 }}>{entry.summary}</b>
        <span className="xs muted">{meta}</span>
      </span>
      {entry.nav != null && <ArrowRight size={14} aria-hidden />}
    </>
  );
  return entry.nav != null ? (
    <button className="li clickable" style={{ width: "100%", border: 0, borderBottom: "1px solid var(--line)", background: "transparent", textAlign: "left" }} onClick={() => navigateTo(entry.nav)}>
      {body}
    </button>
  ) : (
    <div className="li">{body}</div>
  );
}

function FilesCard() {
  const files = useOp<FilesListing>("files.list", {}, FILE_TABLES);
  const go = useNav((s) => s.go);
  const list = files.data?.files ?? [];
  return (
    <section className="card pad" aria-label="Project Files">
      <div className="row" style={{ marginBottom: 6 }}>
        <div className="h4 grow" style={{ margin: 0 }}>Project Files</div>
        <Button size="sm" icon={<Plus size={14} aria-hidden />} onClick={() => void pickAndAddFiles("copy")}>Add File</Button>
      </div>
      {list.length === 0 ? (
        <div className="sm muted" style={{ padding: "6px 0" }}>Keep project documents and attachments here.</div>
      ) : (
        <>
          {list.slice(0, 5).map((f) => {
            const badge = fileBadge(f.asset.mediaType, f.asset.originalName);
            return (
              <button key={f.id} className="li clickable" style={{ width: "100%", border: 0, borderBottom: "1px solid var(--line)", background: "transparent", textAlign: "left", padding: "8px 2px" }} onClick={() => go({ workspace: "files", params: { fileId: f.id } })}>
                <span className={`fic ${badge.cls}`} aria-hidden>{badge.text}</span>
                <span className="grow sm" style={{ minWidth: 0 }}>
                  <b style={{ display: "block", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{f.displayName}</b>
                  <span className="xs muted">
                    {f.asset.available ? fileSize(f.asset.byteSize) || (f.asset.storageMode === "external" ? "Linked" : "") : "Unavailable"}
                  </span>
                </span>
                {!f.asset.available && <Chip tone="r">Unavailable</Chip>}
              </button>
            );
          })}
          <Button variant="ghost" size="sm" onClick={() => go({ workspace: "files" })}>
            {list.length > 5 ? `All ${list.length} files` : "Open Files"}
          </Button>
        </>
      )}
    </section>
  );
}

function TasksCard() {
  const tasks = useOp<TaskDto[]>("tasks.list", { status: "Open" }, ["task", "project_member"]);
  const go = useNav((s) => s.go);
  const open = tasks.data ?? [];
  if (open.length === 0) return null;
  return (
    <section className="card pad" aria-label="Open tasks">
      <div className="row" style={{ marginBottom: 6 }}>
        <div className="h4 grow" style={{ margin: 0 }}>Tasks</div>
        <Chip tone="out">{open.length} open</Chip>
      </div>
      {open.slice(0, 4).map((t) => (
        <div key={t.id} className="li sm" style={{ padding: "6px 2px" }}>
          <span className="grow">{t.title}</span>
          {t.dueAt != null && <span className="xs muted">due {new Date(t.dueAt).toLocaleDateString([], { day: "numeric", month: "short" })}</span>}
        </div>
      ))}
      <Button variant="ghost" size="sm" onClick={() => go({ workspace: "notes" })}>Open Notes & Tasks</Button>
    </section>
  );
}
