// Episodic / Series home and Season Board (FSD §25, UX §3.20, mocks 073–074).
// Each episode has its own Story Board; opening an episode scopes the board.

import { useState } from "react";
import { DndContext, PointerSensor, closestCenter, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { SortableContext, arrayMove, rectSortingStrategy, useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { MoreHorizontal, Plus, Tv } from "lucide-react";
import { Button, Checkbox, Chip, ConfirmDialog, Dialog, EmptyState, Field, Menu, PageHeader, Segmented, Select, Skeleton, TextArea, TextInput } from "../../design-system";
import { reportError, useOp } from "../../ipc/query";
import { toast } from "../../app/toast";
import { story, useCharacters, useSeries } from "../../api/story";
import type { ProjectSummary } from "../../ipc/generated/ProjectSummary";
import type { StoryEpisodeDto } from "../../ipc/generated/StoryEpisodeDto";
import type { StorySeriesDto } from "../../ipc/generated/StorySeriesDto";

const NO_SEASON = "__none__";
const code = (e: StoryEpisodeDto) => `E${String(e.number).padStart(2, "0")}`;
const statusTone = (s: string | null) =>
  s === "Locked" ? "dark" : s === "Writing" || s === "Rewrite" ? "b" : s === "Shooting" || s === "Complete" ? "g" : s === "Development" ? "y" : "default";

type EpisodeDialogState = { mode: "new"; seasonId: string | null } | { mode: "edit"; episode: StoryEpisodeDto } | null;

export function EpisodesView({ readOnly, onOpenEpisode }: { readOnly: boolean; onOpenEpisode: (id: string) => void }) {
  const series = useSeries();
  const project = useOp<ProjectSummary | null>("project.current", {}, ["project"]);
  const characters = useCharacters(null, false);
  const [seasonTab, setSeasonTab] = useState<string | null>(null);
  const [view, setView] = useState<"list" | "board">("list");
  const [dialog, setDialog] = useState<EpisodeDialogState>(null);
  const [dup, setDup] = useState<StoryEpisodeDto | null>(null);
  const [del, setDel] = useState<StoryEpisodeDto | null>(null);
  const [renameSeason, setRenameSeason] = useState<string | null>(null);
  const data = series.data;
  if (!data) return <div className="content"><Skeleton h={160} /></div>;

  const tabs = [...data.seasons.map((s) => ({ value: s.id, label: s.title })), ...(data.unseasoned.length ? [{ value: NO_SEASON, label: "No season" }] : [])];
  const active = seasonTab && tabs.some((t) => t.value === seasonTab) ? seasonTab : (tabs[0]?.value ?? NO_SEASON);
  const season = data.seasons.find((s) => s.id === active) ?? null;
  const episodes = season ? season.episodes : data.unseasoned;
  const seasonId = season?.id ?? null;
  const total = data.seasons.reduce((n, s) => n + s.episodes.length, data.unseasoned.length);

  const addSeason = () =>
    void story
      .createSeason()
      .then((r) => {
        setSeasonTab(r.id);
        toast.undoable("Added a season");
      })
      .catch(reportError);

  return (
    <div className="content" style={{ overflow: "auto" }}>
      <PageHeader
        title={`Series: ${project.data?.title ?? ""}`}
        sub="Which episode am I working on?"
        actions={
          !readOnly && (
            <>
              <Button icon={<Plus size={14} />} onClick={addSeason}>Add Season</Button>
              <Button variant="primary" icon={<Plus size={14} />} onClick={() => setDialog({ mode: "new", seasonId })}>Add Episode</Button>
            </>
          )
        }
      />
      {!data.isEpisodic && (
        <p className="muted" style={{ fontSize: 12.5 }}>
          This project is a {data.projectType}. Episodes are optional; the Story Board works without them.
        </p>
      )}
      {total === 0 && data.seasons.length === 0 ? (
        <EmptyState icon={<Tv size={28} />} title="Create your first episode." actions={!readOnly ? <Button variant="primary" onClick={() => setDialog({ mode: "new", seasonId: null })}>Add Episode</Button> : undefined}>
          Each episode has its own Story Board, screenplay drafts and production planning.
        </EmptyState>
      ) : (
        <div className="row" style={{ alignItems: "flex-start", gap: 16 }}>
          <div className="grow" style={{ minWidth: 0 }}>
            <div className="toolbar">
              {tabs.length > 0 && <Segmented ariaLabel="Season" value={active} onChange={setSeasonTab} options={tabs} />}
              {season && !readOnly && (
                <Menu
                  trigger={
                    <button type="button" className="iconbtn" aria-label={`Options for ${season.title}`}>
                      <MoreHorizontal size={16} />
                    </button>
                  }
                  items={[
                    { label: "Rename season…", onSelect: () => setRenameSeason(season.id) },
                    { label: "Move left", disabled: data.seasons[0]?.id === season.id, onSelect: () => void story.moveSeason(season.id, data.seasons[data.seasons.findIndex((s) => s.id === season.id) - 1]?.id ?? null).catch(reportError) },
                    { label: "Delete season", danger: true, separatorBefore: true, onSelect: () => void story.deleteSeason(season.id).then(() => toast.undoable(`Deleted ${season.title}. Its episodes were kept.`)).catch(reportError) },
                  ]}
                />
              )}
              <span className="sp" />
              <Segmented
                ariaLabel="View"
                value={view}
                onChange={setView}
                options={[
                  { value: "list", label: "List" },
                  { value: "board", label: "Season Board" },
                ]}
              />
            </div>
            {view === "list" ? (
              <div className="card">
                {episodes.length === 0 && <div className="muted" style={{ padding: 16 }}>No episodes in this season yet.</div>}
                {episodes.map((e) => (
                  <div key={e.id} className="li row" style={{ gap: 10, padding: "10px 12px" }}>
                    <button type="button" className="grow" style={{ textAlign: "left", background: "transparent", border: 0, padding: 0 }} onClick={() => onOpenEpisode(e.id)}>
                      <b>{code(e)} {e.title}</b>
                      {e.summary && <span className="muted" style={{ display: "block", fontSize: 12.5 }}>{e.summary}</span>}
                    </button>
                    <span className="muted" style={{ fontSize: 12 }}>{e.cardCount} card{e.cardCount === 1 ? "" : "s"}</span>
                    {e.status && <Chip tone={statusTone(e.status)}>{e.status}</Chip>}
                    <EpisodeMenu e={e} readOnly={readOnly} onEdit={() => setDialog({ mode: "edit", episode: e })} onDuplicate={() => setDup(e)} onDelete={() => setDel(e)} onOpen={() => onOpenEpisode(e.id)} />
                  </div>
                ))}
              </div>
            ) : (
              <SeasonBoard title={season?.title ?? "No season"} seasonId={seasonId} episodes={episodes} readOnly={readOnly} onOpen={onOpenEpisode} onBack={() => setView("list")} />
            )}
          </div>
          <aside className="card pad" style={{ width: 260, flex: "none" }} aria-label="Series references">
            <div className="h4">Series references</div>
            <div className="row" style={{ justifyContent: "space-between" }}>
              <span>Recurring characters</span>
              <b>{characters.data?.filter((c) => !c.episodeId).length ?? 0}</b>
            </div>
            <p className="muted" style={{ fontSize: 12, lineHeight: 1.5 }}>
              Episodes can reference these lightweight lists. OpenFrame does not require you to manage cross-episode continuity.
            </p>
          </aside>
        </div>
      )}
      {dialog && <EpisodeDialog state={dialog} data={data} onClose={() => setDialog(null)} />}
      {dup && <DuplicateDialog e={dup} onClose={() => setDup(null)} />}
      {del && (
        <ConfirmDialog
          open
          onOpenChange={(v) => !v && setDel(null)}
          title={`Delete “${del.title}”?`}
          confirmLabel="Delete Episode"
          danger
          onConfirm={() => {
            const e = del;
            setDel(null);
            void story.deleteEpisode(e.id).then(() => toast.undoable(`Deleted episode “${e.title}”`)).catch(reportError);
          }}
        >
          <p style={{ marginTop: 0 }}>The episode and its Story Board move to Recently Deleted together. You can restore them or press Ctrl+Z.</p>
        </ConfirmDialog>
      )}
      {renameSeason && <RenameSeasonDialog id={renameSeason} current={data.seasons.find((s) => s.id === renameSeason)?.title ?? ""} onClose={() => setRenameSeason(null)} />}
    </div>
  );
}

function EpisodeMenu({ e, readOnly, onEdit, onDuplicate, onDelete, onOpen }: { e: StoryEpisodeDto; readOnly: boolean; onEdit: () => void; onDuplicate: () => void; onDelete: () => void; onOpen: () => void }) {
  return (
    <Menu
      align="end"
      trigger={
        <button type="button" className="iconbtn" aria-label={`Options for ${e.title}`} onPointerDown={(ev) => ev.stopPropagation()}>
          <MoreHorizontal size={16} />
        </button>
      }
      items={[
        { label: "Open Story Board", onSelect: onOpen },
        { label: "Edit…", disabled: readOnly, onSelect: onEdit },
        { label: "Duplicate…", disabled: readOnly, onSelect: onDuplicate },
        { label: "Delete", danger: true, separatorBefore: true, disabled: readOnly, onSelect: onDelete },
      ]}
    />
  );
}

function SeasonBoard({ title, seasonId, episodes, readOnly, onOpen, onBack }: { title: string; seasonId: string | null; episodes: StoryEpisodeDto[]; readOnly: boolean; onOpen: (id: string) => void; onBack: () => void }) {
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 6 } }));
  const ids = episodes.map((e) => e.id);
  const end = (ev: DragEndEvent) => {
    if (!ev.over || ev.active.id === ev.over.id) return;
    const from = ids.indexOf(String(ev.active.id));
    const to = ids.indexOf(String(ev.over.id));
    const next = arrayMove(ids, from, to);
    const before = next[to + 1] ?? null;
    void story
      .moveEpisode(String(ev.active.id), seasonId, before)
      .then(() => toast.undoable("Reordered episodes"))
      .catch(reportError);
  };
  return (
    <section aria-label={`${title} board`}>
      <div className="row" style={{ marginBottom: 8 }}>
        <div>
          <b>{title} — Board</b>
          <div className="muted" style={{ fontSize: 12.5 }}>Plan the season at a glance. Drag episode cards to reorder.</div>
        </div>
        <span className="grow" />
        <Button size="sm" onClick={onBack}>Back to list</Button>
      </div>
      <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={end}>
        <SortableContext items={ids} strategy={rectSortingStrategy}>
          <div className="story-season-grid">
            {episodes.map((e) => (
              <EpisodeCard key={e.id} e={e} readOnly={readOnly} onOpen={() => onOpen(e.id)} />
            ))}
          </div>
        </SortableContext>
      </DndContext>
    </section>
  );
}

function EpisodeCard({ e, readOnly, onOpen }: { e: StoryEpisodeDto; readOnly: boolean; onOpen: () => void }) {
  const s = useSortable({ id: e.id, disabled: readOnly });
  return (
    <div
      ref={s.setNodeRef}
      {...s.attributes}
      {...s.listeners}
      className="card pad story-episode-card"
      style={{ transform: CSS.Transform.toString(s.transform), transition: s.transition, opacity: s.isDragging ? 0.6 : 1 }}
      onDoubleClick={onOpen}
      onKeyDown={(ev) => ev.key === "Enter" && onOpen()}
      aria-label={`Episode ${e.number}: ${e.title}`}
    >
      <div className="muted" style={{ fontSize: 11, fontWeight: 700, textTransform: "uppercase" }}>Episode {e.number}</div>
      <b>{e.title}</b>
      {e.summary && <div className="muted truncate2" style={{ fontSize: 12.5 }}>{e.summary}</div>}
      {e.status && <Chip tone={statusTone(e.status)}>{e.status}</Chip>}
    </div>
  );
}

function EpisodeDialog({ state, data, onClose }: { state: NonNullable<EpisodeDialogState>; data: StorySeriesDto; onClose: () => void }) {
  const ep = state.mode === "edit" ? state.episode : null;
  const [title, setTitle] = useState(ep?.title ?? "");
  const [summary, setSummary] = useState(ep?.summary ?? "");
  const [status, setStatus] = useState(ep?.status ?? "Idea");
  const [season, setSeason] = useState<string>((state.mode === "new" ? state.seasonId : ep?.seasonId) ?? NO_SEASON);
  const save = async () => {
    try {
      const seasonId = season === NO_SEASON ? null : season;
      if (ep) {
        await story.updateEpisode({ id: ep.id, title, summary, status });
        if (seasonId !== ep.seasonId) await story.moveEpisode(ep.id, seasonId, null);
      } else {
        await story.createEpisode(seasonId, title, summary || undefined, status);
        toast.undoable(`Added episode “${title.trim()}”`);
      }
      onClose();
    } catch (e) {
      reportError(e);
    }
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      size="md"
      title={ep ? `Edit “${ep.title}”` : "New Episode"}
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!title.trim()} onClick={() => void save()}>{ep ? "Save" : "Create Episode"}</Button>
        </>
      }
    >
      <Field label="Title" required htmlFor="ep-title">
        <TextInput id="ep-title" value={title} onChange={(e) => setTitle(e.target.value)} autoFocus placeholder="Pilot — First Rain" />
      </Field>
      <Field label="One-line summary" htmlFor="ep-sum">
        <TextArea id="ep-sum" rows={2} value={summary} onChange={(e) => setSummary(e.target.value)} />
      </Field>
      <div className="grid g2" style={{ gap: 12 }}>
        <Field label="Status">
          <Select value={status} onChange={setStatus} ariaLabel="Status" options={data.statuses.map((s) => ({ value: s, label: s }))} />
        </Field>
        <Field label="Season">
          <Select
            value={season}
            onChange={setSeason}
            ariaLabel="Season"
            options={[...data.seasons.map((s) => ({ value: s.id, label: s.title })), { value: NO_SEASON, label: "No season" }]}
          />
        </Field>
      </div>
      <p className="muted" style={{ fontSize: 12 }}>The episode number comes from its order in the season.</p>
    </Dialog>
  );
}

function DuplicateDialog({ e, onClose }: { e: StoryEpisodeDto; onClose: () => void }) {
  const [copyStory, setCopyStory] = useState(true);
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      size="sm"
      title={`Duplicate “${e.title}”`}
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button
            variant="primary"
            onClick={() => {
              onClose();
              void story.duplicateEpisode(e.id, copyStory).then(() => toast.undoable(`Duplicated “${e.title}”`)).catch(reportError);
            }}
          >
            Duplicate
          </Button>
        </>
      }
    >
      <Checkbox checked={copyStory} onChange={setCopyStory} label="Also copy its Story Board (new, independent cards)" />
      <p className="muted" style={{ fontSize: 12.5 }}>The copy is a new episode. Screenplay links are not copied.</p>
    </Dialog>
  );
}

function RenameSeasonDialog({ id, current, onClose }: { id: string; current: string; onClose: () => void }) {
  const [title, setTitle] = useState(current);
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      size="sm"
      title="Rename season"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button
            variant="primary"
            disabled={!title.trim()}
            onClick={() => {
              onClose();
              void story.updateSeason(id, title).catch(reportError);
            }}
          >
            Save
          </Button>
        </>
      }
    >
      <Field label="Season title" htmlFor="season-title">
        <TextInput id="season-title" value={title} onChange={(e) => setTitle(e.target.value)} autoFocus />
      </Field>
    </Dialog>
  );
}
