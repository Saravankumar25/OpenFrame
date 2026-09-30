// Story workspace root (FSD §7–14, §18, §25; UX §3.6–3.10, §3.20).
// Tabs: Board | Outline | Characters | Timeline (+ Episodes for series).
// Board and Outline render the SAME Rust-owned structure; switching views
// never changes data.

import { useCallback, useEffect, useMemo, useState } from "react";
import { Columns3, FileText, Info, LayoutList, Minus, MoreHorizontal, ParkingSquare, Plus, Search, X } from "lucide-react";
import { Button, Chip, EmptyState, Menu, PageHeader, Segmented, Select, Skeleton, cx } from "../../design-system";
import { useIntent, useNav, type Route } from "../../app/stores";
import { reportError } from "../../ipc/query";
import { story, useSeries, useStoryBoard, useViewState } from "../../api/story";
import type { StoryBoardState } from "../../ipc/generated/StoryBoardState";
import type { StorySeriesDto } from "../../ipc/generated/StorySeriesDto";
import { defaultInsert, duplicateItems, newBeat, newCard, parkItems, requestDelete } from "./actions";
import { BoardView, NewActBand } from "./BoardView";
import { ApplyOrderDialog, BuildDialog } from "./BuildDialog";
import { CharactersView } from "./Characters";
import { ActNoteDialog, DeleteActDialog, DeleteLinkedDialog, DeleteSequenceDialog, MoveDialog } from "./dialogs";
import { BeatDrawer, CardDrawer, SequenceDrawer } from "./drawers";
import { EpisodesView } from "./Episodes";
import { focusItem } from "./items";
import { PARKING, containers, refKey } from "./model";
import { OutlineView } from "./OutlineView";
import { TimelineView } from "./Timeline";
import { ZOOM_LEVELS, keyToRef, useStoryUi } from "./ui";
import { StoryExportButton } from "../../features/export/buttons";
import "./story.css";

type Tab = "board" | "outline" | "characters" | "timeline" | "episodes";
const TABS: { id: Tab; label: string }[] = [
  { id: "board", label: "Board" },
  { id: "outline", label: "Outline" },
  { id: "characters", label: "Characters" },
  { id: "timeline", label: "Timeline" },
];

// Local projects are edited by their owner; collaborator roles are enforced by
// Rust on every command (errors are shown as human messages).
const READ_ONLY = false;

function allEpisodes(s: StorySeriesDto | undefined) {
  if (!s) return [];
  return [...s.seasons.flatMap((x) => x.episodes.map((e) => ({ ...e, seasonTitle: x.title }))), ...s.unseasoned.map((e) => ({ ...e, seasonTitle: "" }))];
}

const isEditable = (t: EventTarget | null) =>
  t instanceof HTMLElement && (t.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(t.tagName));

export default function StoryWorkspace() {
  const route = useNav((s) => s.route);
  const go = useNav((s) => s.go);
  const series = useSeries();
  const viewState = useViewState();
  const episodes = useMemo(() => allEpisodes(series.data), [series.data]);
  const episodic = series.data?.isEpisodic ?? false;
  const showEpisodes = episodic || episodes.length > 0;
  const tab: Tab = (["board", "outline", "characters", "timeline", "episodes"] as Tab[]).includes(route.sub as Tab) ? (route.sub as Tab) : "board";

  // Episode scope: explicit route param > the user's last choice > first episode.
  const paramEpisode = route.params?.episodeId;
  const vs = viewState.data;
  const episodeId: string | null = useMemo(() => {
    const exists = (id: string | null | undefined) => !!id && episodes.some((e) => e.id === id);
    if (exists(paramEpisode)) return paramEpisode!;
    if (vs?.episodeChosen && (vs.episodeId === null || exists(vs.episodeId))) return vs.episodeId;
    if (episodic && episodes[0]) return episodes[0].id;
    return null;
  }, [paramEpisode, vs, episodes, episodic]);

  useEffect(() => {
    const ui = useStoryUi.getState();
    ui.clearSelection();
    ui.openDrawer(null);
  }, [episodeId]);

  const setTab = (t: Tab) => go({ workspace: "story", sub: t === "board" ? undefined : t, params: episodeId ? { episodeId } : undefined });
  const chooseEpisode = (id: string | null) => {
    void story.setCurrentEpisode(id).catch(reportError);
    go({ workspace: "story", sub: route.sub, params: id ? { episodeId: id } : undefined });
  };

  // Quick-action intents from the shell "+" menu (UX §2.7).
  const intent = useIntent((s) => s.intent);
  const [wantCharacter, setWantCharacter] = useState(false);
  const board = useStoryBoard(episodeId);
  useEffect(() => {
    if (!intent) return;
    const consume = useIntent.getState().consume;
    const ui = useStoryUi.getState();
    const b = board.data;
    if (intent === "story.new_character" && consume(intent)) {
      setWantCharacter(true);
      if (tab !== "characters") go({ workspace: "story", sub: "characters" });
      return;
    }
    if (!b) return; // wait for the board before placing new objects
    if (consume("story.new_act")) {
      if (tab !== "board" && tab !== "outline") setTab("board");
      ui.setAddingAct(true);
    } else if (consume("story.new_sequence")) {
      if (tab !== "board" && tab !== "outline") setTab("board");
      const last = b.acts[b.acts.length - 1];
      if (last) ui.setAddingSequenceIn(last.id);
      else
        void story
          .createAct({ episodeId, title: "Act 1" })
          .then((r) => ui.setAddingSequenceIn(r.id))
          .catch(reportError);
    } else if (consume("story.new_beat")) {
      if (tab !== "board" && tab !== "outline") setTab("board");
      const t = defaultInsert(b, ui.selected);
      void newBeat(episodeId, t.container ?? PARKING, t.index);
    } else if (consume("story.new_scene")) {
      if (tab !== "board" && tab !== "outline") setTab("board");
      const t = defaultInsert(b, ui.selected);
      void newCard(episodeId, t.container, t.index);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [intent, board.data]);

  const onNewCharacterHandled = useCallback(() => setWantCharacter(false), []);

  return (
    <>
      <div className="subnav" role="tablist" aria-label="Story views">
        {TABS.map((t) => (
          <button key={t.id} role="tab" aria-selected={tab === t.id} className={cx(tab === t.id && "active")} onClick={() => setTab(t.id)}>
            {t.label}
          </button>
        ))}
        {showEpisodes && (
          <button role="tab" aria-selected={tab === "episodes"} className={cx(tab === "episodes" && "active")} onClick={() => setTab("episodes")}>
            Episodes
          </button>
        )}
        <span style={{ flex: 1 }} />
        {showEpisodes && tab !== "episodes" && (
          <div className="row" style={{ gap: 6, padding: "4px 0" }}>
            <span className="muted" style={{ fontSize: 12 }}>Episode</span>
            <Select
              ariaLabel="Episode"
              value={episodeId ?? ""}
              onChange={(v) => chooseEpisode(v || null)}
              options={[
                ...episodes.map((e) => ({ value: e.id, label: `${e.seasonTitle ? `${e.seasonTitle} · ` : ""}E${String(e.number).padStart(2, "0")} ${e.title}` })),
                { value: "", label: "Series level (no episode)" },
              ]}
            />
          </div>
        )}
      </div>
      {tab === "characters" ? (
        <div className="content fill">
          <PageHeader title="Characters" sub="A lightweight directory. Its most useful feature is the list of scenes each character appears in." />
          <CharactersView episodeId={episodeId} readOnly={READ_ONLY} focusId={route.params?.characterId} wantNew={wantCharacter} onNewHandled={onNewCharacterHandled} />
        </div>
      ) : tab === "timeline" ? (
        <TimelineView episodeId={episodeId} readOnly={READ_ONLY} />
      ) : tab === "episodes" ? (
        <EpisodesView
          readOnly={READ_ONLY}
          onOpenEpisode={(id) => {
            void story.setCurrentEpisode(id).catch(reportError);
            go({ workspace: "story", params: { episodeId: id } });
          }}
        />
      ) : (
        <BoardPage tab={tab} board={board.data} loading={board.isLoading} error={board.error?.message} episodeId={episodeId} route={route} onTab={setTab} />
      )}
    </>
  );
}

function BoardPage({
  tab,
  board,
  loading,
  error,
  episodeId,
  route,
  onTab,
}: {
  tab: "board" | "outline";
  board: StoryBoardState | undefined;
  loading: boolean;
  error?: string;
  episodeId: string | null;
  route: Route;
  onTab: (t: Tab) => void;
}) {
  const ui = useStoryUi();
  const readOnly = READ_ONLY;

  // Navigation targets from search / Continue: open and reveal the object.
  const params = route.params;
  useEffect(() => {
    if (!board || !params) return;
    const s = useStoryUi.getState();
    // `sceneCardId` (older links) and `screenplaySceneId` (Screenplay "Open in
    // Story Board" for a scene without a source card) resolve to a card too.
    const linkedCard = params.screenplaySceneId
      ? containers(board)
          .flatMap((x) => x.items)
          .find((i) => i.kind === "card" && i.screenplaySceneId === params.screenplaySceneId)?.id
      : undefined;
    const cardId = params.cardId ?? params.sceneCardId ?? linkedCard;
    const target = cardId
      ? ({ kind: "card", id: cardId } as const)
      : params.beatId
        ? ({ kind: "beat", id: params.beatId } as const)
        : params.sequenceId
          ? ({ kind: "sequence", id: params.sequenceId } as const)
          : null;
    if (target) {
      s.select([refKey(target)]);
      s.openDrawer({ kind: target.kind, id: target.id });
      window.setTimeout(() => focusItem(refKey(target)), 80);
    } else if (params.actId) {
      document.querySelector(`[aria-label="Act ${CSS.escape(board.acts.find((a) => a.id === params.actId)?.title ?? "")}"]`)?.scrollIntoView({ inline: "center" });
    }
    // Only when the navigation target changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [params, !!board]);

  // Workspace shortcuts: Escape closes the drawer / clears selection. Ctrl+N
  // (New Scene Card) is bound by the shell, which fires `story.new_scene`
  // (handled by StoryWorkspace) so it works from every Story tab and workspace.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (isEditable(e.target) || !board) return;
      const s = useStoryUi.getState();
      if (e.key === "Escape" && !s.dialog) {
        if (s.drawer) s.openDrawer(null);
        else s.clearSelection();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [board]);

  if (loading || !board) {
    return (
      <div className="content">
        {error ? <EmptyState title="The Story Board could not be loaded.">{error}</EmptyState> : <Skeleton h={28} w={240} />}
      </div>
    );
  }

  const empty = board.acts.length === 0 && board.parking.length === 0 && board.unassigned.length === 0;
  const selectedRefs = ui.selected.map(keyToRef).filter((r) => r.kind !== "sequence");
  const zoomIdx = ZOOM_LEVELS.indexOf(ui.zoom as (typeof ZOOM_LEVELS)[number]);
  const lastAct = board.acts[board.acts.length - 1];

  const addMenu = [
    { label: "Act", onSelect: () => ui.setAddingAct(true) },
    {
      label: "Sequence",
      disabled: !lastAct,
      onSelect: () => lastAct && ui.setAddingSequenceIn(ui.selected.length && board.acts.some((a) => a.id === ui.selected[0]) ? ui.selected[0] : lastAct.id),
    },
    {
      label: "Beat",
      onSelect: () => {
        const t = defaultInsert(board, ui.selected);
        void newBeat(episodeId, t.container ?? PARKING, t.index);
      },
    },
    {
      label: "Scene Card",
      shortcut: "Ctrl+N",
      onSelect: () => {
        const t = defaultInsert(board, ui.selected);
        void newCard(episodeId, t.container, t.index);
      },
    },
    { label: "From Idea Vault…", separatorBefore: true, onSelect: () => useNav.getState().go({ workspace: "vault" }) },
  ];

  return (
    <div className="content fill story-page">
      <PageHeader
        title="Story Board"
        sub="How does my story fit together? · Drag cards until the story works."
        actions={
          <>
            <Chip>
              <Info size={12} /> No scene numbers — they come from the screenplay
            </Chip>
            <StoryExportButton episodeId={episodeId} selectedIds={ui.selected.map((k) => keyToRef(k).id)} />
          </>
        }
      />
      <div className="toolbar">
        <Segmented
          ariaLabel="Story Board view"
          value={tab}
          onChange={(v) => onTab(v)}
          options={[
            { value: "board", label: <><Columns3 size={13} /> Board</> },
            { value: "outline", label: <><LayoutList size={13} /> Outline</> },
          ]}
        />
        {!readOnly && (
          <Menu
            items={addMenu}
            trigger={
              <Button variant="primary" icon={<Plus size={14} />}>
                Add
              </Button>
            }
          />
        )}
        <div className="input story-filter" style={{ flex: "0 0 190px", display: "flex", alignItems: "center", gap: 6 }}>
          <Search size={13} aria-hidden />
          <input
            aria-label="Search / filter cards"
            placeholder="Search / filter cards"
            value={ui.filter}
            onChange={(e) => ui.setFilter(e.target.value)}
            style={{ border: 0, outline: "none", background: "transparent", flex: 1, minWidth: 0 }}
          />
          {ui.filter && (
            <button type="button" className="iconbtn sm" aria-label="Clear filter" onClick={() => ui.setFilter("")}>
              <X size={12} />
            </button>
          )}
        </div>
        {tab === "board" && (
          <div className="row" style={{ gap: 4 }} role="group" aria-label="Zoom">
            <button type="button" className="iconbtn" aria-label="Zoom out" disabled={zoomIdx <= 0} onClick={() => ui.setZoom(ZOOM_LEVELS[Math.max(0, zoomIdx - 1)])}>
              <Minus size={14} />
            </button>
            <span style={{ width: 42, textAlign: "center", fontSize: 12.5, fontWeight: 600 }} aria-live="polite">{ui.zoom}%</span>
            <button
              type="button"
              className="iconbtn"
              aria-label="Zoom in"
              disabled={zoomIdx >= ZOOM_LEVELS.length - 1}
              onClick={() => ui.setZoom(ZOOM_LEVELS[Math.min(ZOOM_LEVELS.length - 1, zoomIdx + 1)])}
            >
              <Plus size={14} />
            </button>
          </div>
        )}
        <div className="sp" />
        <Button variant={ui.parkingOpen ? "on" : "default"} icon={<ParkingSquare size={14} />} aria-pressed={ui.parkingOpen} onClick={() => ui.setParkingOpen(!ui.parkingOpen)}>
          Parking Lot
        </Button>
        {!readOnly && (
          <>
            <Button variant="primary" icon={<FileText size={14} />} onClick={() => ui.openDialog({ type: "build" })}>
              Build Screenplay
            </Button>
            <Menu
              align="end"
              items={[{ label: "Apply Board Order to Screenplay…", onSelect: () => ui.openDialog({ type: "applyOrder" }) }]}
              trigger={
                <button type="button" className="iconbtn" aria-label="More screenplay actions">
                  <MoreHorizontal size={16} />
                </button>
              }
            />
          </>
        )}
      </div>

      {selectedRefs.length > 1 && (
        <div className="mbar" role="toolbar" aria-label="Selection actions">
          <b>
            {selectedRefs.length} {selectedRefs.every((r) => r.kind === "card") ? "cards" : "items"} selected
          </b>
          <span style={{ flex: 1 }} />
          <button type="button" className="btn sm" onClick={() => ui.openDialog({ type: "move", refs: selectedRefs })}>Move…</button>
          <button type="button" className="btn sm" onClick={() => void duplicateItems(board, selectedRefs)}>Duplicate</button>
          <button type="button" className="btn sm" onClick={() => void parkItems(board, selectedRefs)}>Park</button>
          <button type="button" className="btn sm dan" onClick={() => requestDelete(board, selectedRefs)}>Delete</button>
          <button type="button" className="iconbtn" aria-label="Clear selection" style={{ color: "#fff" }} onClick={() => ui.clearSelection()}>
            <X size={14} />
          </button>
        </div>
      )}

      {empty ? (
        ui.addingAct ? (
          <div className="story-canvas"><div className="board"><NewActBand episodeId={episodeId} /></div></div>
        ) : (
          <EmptyState
            icon={<Columns3 size={30} />}
            title="Your Story Board is empty."
            actions={
              !readOnly && (
                <>
                  <Button variant="primary" icon={<Plus size={14} />} onClick={() => void newCard(episodeId, null)}>Add Scene</Button>
                  <Button icon={<Plus size={14} />} onClick={() => ui.setAddingAct(true)}>Add Act</Button>
                </>
              )
            }
          >
            <p style={{ margin: "0 0 6px" }}>Scene Cards are short reminders of what happens in each scene. Drag them until the story works.</p>
            Start shaping the story. Add an Act, Sequence, Beat, or Scene Card — or drag an idea from your Idea Vault.
          </EmptyState>
        )
      ) : tab === "board" ? (
        <BoardView board={board} episodeId={episodeId} readOnly={readOnly} />
      ) : (
        <div className="story-canvas" style={{ padding: "4px 2px" }}>
          <OutlineView board={board} episodeId={episodeId} readOnly={readOnly} />
          {ui.addingAct && <NewActBand episodeId={episodeId} />}
          {!readOnly && !ui.addingAct && (
            <button type="button" className="story-add" style={{ marginTop: 8, width: 200 }} onClick={() => ui.setAddingAct(true)}>
              <Plus size={12} /> Add Act
            </button>
          )}
        </div>
      )}

      {ui.drawer?.kind === "card" && <CardDrawer key={ui.drawer.id} id={ui.drawer.id} board={board} readOnly={readOnly} />}
      {ui.drawer?.kind === "beat" && <BeatDrawer key={ui.drawer.id} id={ui.drawer.id} board={board} readOnly={readOnly} />}
      {ui.drawer?.kind === "sequence" && <SequenceDrawer key={ui.drawer.id} id={ui.drawer.id} board={board} readOnly={readOnly} />}

      {ui.dialog?.type === "deleteAct" && <DeleteActDialog board={board} actId={ui.dialog.actId} />}
      {ui.dialog?.type === "deleteSequence" && <DeleteSequenceDialog board={board} sequenceId={ui.dialog.sequenceId} />}
      {ui.dialog?.type === "deleteLinked" && <DeleteLinkedDialog board={board} refs={ui.dialog.refs} />}
      {ui.dialog?.type === "move" && <MoveDialog board={board} refs={ui.dialog.refs} episodeId={episodeId} />}
      {ui.dialog?.type === "actNote" && <ActNoteDialog board={board} actId={ui.dialog.actId} />}
      {ui.dialog?.type === "build" && <BuildDialog episodeId={episodeId} preselect={ui.dialog.preselect} />}
      {ui.dialog?.type === "applyOrder" && <ApplyOrderDialog episodeId={episodeId} />}
    </div>
  );
}
