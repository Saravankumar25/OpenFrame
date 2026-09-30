// Production → Storyboards (FSD §32, §102; UX §45, §87; mocks 123–125).
// "What will the audience see?" — ordered panels for a scene (or a standalone
// board). Panel numbers come from order; a linked shot shows its generated
// label (e.g. 12A) and dragging panels re-sequences the linked shots in Rust.

import { useEffect, useMemo, useRef, useState, type ClipboardEvent } from "react";
import {
  DndContext,
  KeyboardSensor,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
  type DragEndEvent,
} from "@dnd-kit/core";
import { SortableContext, rectSortingStrategy, sortableKeyboardCoordinates, useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { ImagePlus, MoreHorizontal, PenLine, Plus, Square } from "lucide-react";
import {
  Button,
  Chip,
  ConfirmDialog,
  ContextMenu,
  Dialog,
  Drawer,
  EmptyState,
  Field,
  IconButton,
  Menu,
  PageHeader,
  Skeleton,
  TextArea,
  TextInput,
  cx,
  type MenuItemSpec,
} from "../../../design-system";
import { call } from "../../../ipc/client";
import { reportError, useCommand, useOp } from "../../../ipc/query";
import { useNav } from "../../../app/stores";
import { toast } from "../../../app/toast";
import {
  ANGLES,
  MOVEMENTS,
  SHOT_SIZES,
  SHOT_TABLES,
  STORYBOARD_TABLES,
  assetSrc,
  blobToBase64,
  formatSeconds,
  moveItem,
  parseSeconds,
  pickImages,
  sceneLabel,
  useVisualScenes,
} from "../../../api/visual";
import { RemovedSceneBanner, SceneReviewBanner, SuggestField, useVisualView } from "../visual/shared";
import { SketchDialog } from "../visual/SketchDialog";
import type { StoryboardSummary } from "../../../ipc/generated/StoryboardSummary";
import type { StoryboardDto } from "../../../ipc/generated/StoryboardDto";
import type { PanelDto } from "../../../ipc/generated/PanelDto";
import type { ShotDto } from "../../../ipc/generated/ShotDto";
import type { VisualScenes } from "../../../ipc/generated/VisualScenes";
import type { VisualScene } from "../../../ipc/generated/VisualScene";
import type { AddPanelArgs } from "../../../ipc/generated/AddPanelArgs";
import type { UpdatePanelArgs } from "../../../ipc/generated/UpdatePanelArgs";
import type { ReorderPanelArgs } from "../../../ipc/generated/ReorderPanelArgs";
import type { DeletePanelsArgs } from "../../../ipc/generated/DeletePanelsArgs";
import type { CreateStoryboardArgs } from "../../../ipc/generated/CreateStoryboardArgs";
import { StoryboardExportButton } from "../../../features/export/buttons";

export const tab = { id: "storyboards", label: "Storyboards", order: 50 };

type AddKind = "image" | "sketch" | "placeholder";

export default function StoryboardsTab() {
  const route = useNav((s) => s.route);
  const view = useVisualView();
  const scenesQ = useVisualScenes();
  const boardsQ = useOp<StoryboardSummary[]>("storyboard.list", {}, STORYBOARD_TABLES);
  const scenes = scenesQ.data;
  const boards = useMemo(() => boardsQ.data ?? [], [boardsQ.data]);

  const [selectedPanel, setSelectedPanel] = useState<string | null>(null);
  const [newOpen, setNewOpen] = useState(false);
  const [sketchOpen, setSketchOpen] = useState(false);

  // Deep links: { storyboardId, panelId } from search/shots, { sceneId } from the Scene Hub.
  const linkBoard = route.params?.storyboardId;
  const linkPanel = route.params?.panelId;
  const linkScene = route.params?.sceneId;
  const linkLineage = route.params?.sceneLineageId;
  const consumed = useRef<string | null>(null);
  useEffect(() => {
    const key = `${linkBoard ?? ""}|${linkPanel ?? ""}|${linkScene ?? ""}|${linkLineage ?? ""}`;
    if (consumed.current === key || !boardsQ.data || !scenes) return;
    consumed.current = key;
    if (linkBoard) {
      const b = boards.find((x) => x.id === linkBoard);
      if (b?.sceneLineageId && !b.sceneRemoved) {
        view.set({ boardKey: `scene:${b.sceneLineageId}`, sceneBoard: { ...view.sceneBoard, [b.sceneLineageId]: b.id } });
      } else if (b) {
        view.set({ boardKey: `board:${b.id}` });
      }
      if (linkPanel) setSelectedPanel(linkPanel);
    } else if (linkScene || linkLineage) {
      const sc =
        scenes.scenes.find((s) => s.sceneId === linkScene || s.lineageId === linkScene) ??
        scenes.scenes.find((s) => !!linkLineage && s.lineageId === linkLineage);
      if (sc) view.set({ boardKey: `scene:${sc.lineageId}` });
    }
  }, [linkBoard, linkPanel, linkScene, linkLineage, boardsQ.data, boards, scenes, view]);

  const sel = resolveSelection(view.boardKey, view.sceneBoard, scenes, boards);
  const boardQ = useOp<StoryboardDto>("storyboard.get", sel.boardId ? { id: sel.boardId } : undefined, STORYBOARD_TABLES, {
    enabled: !!sel.boardId,
  });
  const board = boardQ.data?.board.id === sel.boardId ? boardQ.data : undefined;
  const panels = board?.panels ?? [];
  const panel = panels.find((p) => p.id === selectedPanel);

  const create = useCommand<CreateStoryboardArgs, StoryboardSummary>("storyboard.create");
  const addPanel = useCommand<AddPanelArgs, PanelDto>("storyboard.add_panel", {
    onSuccess: (p) => {
      toast.undoable(`Added panel ${p.number}`);
      setSketchOpen(false);
    },
  });

  /** The board new panels go into; a scene's first panel creates its storyboard. */
  const ensureBoard = async (): Promise<string | null> => {
    if (sel.boardId) return sel.boardId;
    if (sel.scene) {
      const b = await create.mutateAsync({ sceneId: sel.scene.sceneId });
      view.set({ sceneBoard: { ...view.sceneBoard, [sel.scene.lineageId]: b.id } });
      return b.id;
    }
    setNewOpen(true);
    return null;
  };

  const add = async (kind: AddKind, extra: Partial<AddPanelArgs> = {}) => {
    try {
      if (kind === "sketch" && !extra.dataBase64) {
        setSketchOpen(true);
        return;
      }
      let path: string | undefined;
      if (kind === "image" && !extra.dataBase64) {
        const [p] = await pickImages(false);
        if (!p) return;
        path = p;
      }
      const boardId = await ensureBoard();
      if (!boardId) return;
      await addPanel.mutateAsync({ storyboardId: boardId, visual: kind, path, ...extra });
    } catch (e) {
      reportError(e);
    }
  };

  const onPaste = async (e: ClipboardEvent<HTMLDivElement>) => {
    const file = [...e.clipboardData.files].find((f) => f.type.startsWith("image/"));
    if (!file) return;
    e.preventDefault();
    await add("image", { dataBase64: await blobToBase64(file) });
  };

  const addMenu: MenuItemSpec[] = [
    { label: "Import Image", icon: <ImagePlus size={14} />, onSelect: () => void add("image") },
    { label: "Draw / Sketch", icon: <PenLine size={14} />, onSelect: () => void add("sketch") },
    { label: "Empty Panel", icon: <Square size={14} />, onSelect: () => void add("placeholder") },
  ];
  const canAdd = !!(sel.boardId || sel.scene);

  if (scenesQ.isLoading || boardsQ.isLoading) {
    return (
      <div>
        <PageHeader title="Storyboards" sub="What will the audience see?" />
        <Skeleton h={34} w={420} />
      </div>
    );
  }

  const noScenes = !scenes || scenes.scenes.length === 0;
  if (noScenes && boards.length === 0) {
    return (
      <div>
        <PageHeader title="Storyboards" sub="What will the audience see?" />
        <EmptyState
          title="No storyboards yet"
          actions={
            <Button variant="primary" onClick={() => setNewOpen(true)}>
              New Storyboard
            </Button>
          }
        >
          Storyboards usually belong to a screenplay scene, so its heading comes from the script. You can also start a standalone
          storyboard now — the screenplay never needs shot directions.
        </EmptyState>
        <NewStoryboardDialog open={newOpen} onOpenChange={setNewOpen} scenes={scenes} onCreated={(b) => view.set({ boardKey: `board:${b.id}` })} />
      </div>
    );
  }

  const sceneBoards = sel.scene ? boards.filter((b) => b.sceneLineageId === sel.scene?.lineageId) : [];

  return (
    <div style={{ position: "relative", minHeight: "100%" }} onPaste={(e) => void onPaste(e)}>
      <PageHeader
        title="Storyboards"
        sub="What will the audience see?"
        actions={
          <>
            <StoryboardExportButton
              current={board ? { id: board.board.id, name: board.board.name } : null}
              boards={boards.map((b) => ({ id: b.id, label: b.name }))}
              disabled={boards.length === 0}
            />
            <Button onClick={() => setNewOpen(true)}>New Storyboard</Button>
            <Menu
              align="end"
              items={addMenu}
              trigger={
                <Button variant="primary" icon={<Plus size={15} />} disabled={!canAdd}>
                  Add Panel
                </Button>
              }
            />
          </>
        }
      />
      <div className="toolbar">
        <select
          className="select"
          aria-label="Scene or storyboard"
          style={{ maxWidth: 440, width: "auto" }}
          value={sel.key ?? ""}
          onChange={(e) => {
            view.set({ boardKey: e.target.value });
            setSelectedPanel(null);
          }}
        >
          {scenes && scenes.scenes.length > 0 && (
            <optgroup label={scenes.sourceLabel || "Scenes"}>
              {scenes.scenes.map((s) => (
                <option key={s.lineageId} value={`scene:${s.lineageId}`}>
                  {sceneLabel(s)}
                  {s.needsReview ? " — review" : ""}
                </option>
              ))}
            </optgroup>
          )}
          {boards.some((b) => b.sceneRemoved) && (
            <optgroup label="Removed from script">
              {boards
                .filter((b) => b.sceneRemoved)
                .map((b) => (
                  <option key={b.id} value={`board:${b.id}`}>
                    {b.name}
                  </option>
                ))}
            </optgroup>
          )}
          {boards.some((b) => !b.sceneLineageId) && (
            <optgroup label="Other storyboards">
              {boards
                .filter((b) => !b.sceneLineageId)
                .map((b) => (
                  <option key={b.id} value={`board:${b.id}`}>
                    {b.name}
                  </option>
                ))}
            </optgroup>
          )}
        </select>
        {sceneBoards.length > 1 && (
          <select
            className="select"
            aria-label="Storyboard for this scene"
            style={{ width: "auto" }}
            value={sel.boardId ?? ""}
            onChange={(e) => sel.scene && view.set({ sceneBoard: { ...view.sceneBoard, [sel.scene.lineageId]: e.target.value } })}
          >
            {sceneBoards.map((b) => (
              <option key={b.id} value={b.id}>
                {b.name}
              </option>
            ))}
          </select>
        )}
        <Chip>
          {panels.length} {panels.length === 1 ? "panel" : "panels"}
        </Chip>
        <span className="sp" />
        {board && <BoardMenu board={board.board} scenes={scenes} onDeleted={() => view.set({ boardKey: null })} onNewForScene={() => setNewOpen(true)} />}
      </div>

      {sel.scene && <SceneReviewBanner scene={sel.scene} what="storyboard panels and shots" />}
      {board?.board.sceneRemoved && <RemovedSceneBanner heading={board.board.sceneHeading ?? ""} what="storyboard panels" />}

      {sel.boardId && boardQ.isLoading ? (
        <Skeleton h={160} />
      ) : panels.length === 0 ? (
        <EmptyState
          title="Create panels for this scene."
          actions={
            canAdd ? (
              <>
                <Button icon={<ImagePlus size={14} />} onClick={() => void add("image")}>
                  Import Image
                </Button>
                <Button icon={<PenLine size={14} />} onClick={() => void add("sketch")}>
                  Draw / Sketch
                </Button>
                <Button icon={<Square size={14} />} onClick={() => void add("placeholder")}>
                  Empty Panel
                </Button>
              </>
            ) : undefined
          }
        >
          A panel needs only a visual area and a short description. Link panels to shots whenever you like — neither is required.
        </EmptyState>
      ) : (
        board && (
          <PanelGrid
            board={board}
            selectedId={selectedPanel}
            onOpen={setSelectedPanel}
            addMenu={addMenu}
            boards={boards}
          />
        )
      )}

      {panel && board && (
        <PanelDrawer key={panel.id} panel={panel} board={board.board} scenes={scenes} onClose={() => setSelectedPanel(null)} />
      )}
      <NewStoryboardDialog
        open={newOpen}
        onOpenChange={setNewOpen}
        scenes={scenes}
        defaultSceneId={sel.scene?.sceneId}
        onCreated={(b) => {
          if (b.sceneLineageId) view.set({ boardKey: `scene:${b.sceneLineageId}`, sceneBoard: { ...view.sceneBoard, [b.sceneLineageId]: b.id } });
          else view.set({ boardKey: `board:${b.id}` });
        }}
      />
      <SketchDialog
        open={sketchOpen}
        onOpenChange={setSketchOpen}
        saving={addPanel.isPending}
        onSave={(png) => void add("sketch", { dataBase64: png })}
      />
    </div>
  );
}

interface Selection {
  key: string | null;
  boardId: string | null;
  scene: VisualScene | null;
}

export function resolveSelection(
  wanted: string | null,
  sceneBoard: Record<string, string>,
  scenes: VisualScenes | undefined,
  boards: StoryboardSummary[],
): Selection {
  const sceneFor = (lineage: string) => scenes?.scenes.find((s) => s.lineageId === lineage) ?? null;
  const pickSceneBoard = (lineage: string) => {
    const list = boards.filter((b) => b.sceneLineageId === lineage);
    return list.find((b) => b.id === sceneBoard[lineage])?.id ?? list[0]?.id ?? null;
  };
  if (wanted?.startsWith("scene:")) {
    const lineage = wanted.slice(6);
    const sc = sceneFor(lineage);
    if (sc) return { key: wanted, boardId: pickSceneBoard(lineage), scene: sc };
  }
  if (wanted?.startsWith("board:")) {
    const b = boards.find((x) => x.id === wanted.slice(6));
    if (b) {
      if (b.sceneLineageId && !b.sceneRemoved) {
        const sc = sceneFor(b.sceneLineageId);
        if (sc) return { key: `scene:${b.sceneLineageId}`, boardId: b.id, scene: sc };
      }
      return { key: wanted, boardId: b.id, scene: null };
    }
  }
  // Default: the first scene that has a storyboard, else the first scene, else the first board.
  const withBoard = scenes?.scenes.find((s) => boards.some((b) => b.sceneLineageId === s.lineageId));
  const first = withBoard ?? scenes?.scenes[0];
  if (first) return { key: `scene:${first.lineageId}`, boardId: pickSceneBoard(first.lineageId), scene: first };
  const b = boards[0];
  return b ? { key: `board:${b.id}`, boardId: b.id, scene: null } : { key: null, boardId: null, scene: null };
}

// ------------------------------------------------------------------ panels

function PanelGrid({
  board,
  selectedId,
  onOpen,
  addMenu,
  boards,
}: {
  board: StoryboardDto;
  selectedId: string | null;
  onOpen: (id: string) => void;
  addMenu: MenuItemSpec[];
  boards: StoryboardSummary[];
}) {
  const [order, setOrder] = useState<string[] | null>(null);
  const idsKey = board.panels.map((p) => p.id).join(",");
  useEffect(() => setOrder(null), [idsKey]);
  const shown = order ? order.map((id) => board.panels.find((p) => p.id === id)).filter((p): p is PanelDto => !!p) : board.panels;
  const reorder = useCommand<ReorderPanelArgs>("storyboard.reorder_panel", {
    onSuccess: () => toast.undoable(board.board.sceneLineageId ? "Moved panel · shot numbers follow the new order" : "Moved panel"),
    onError: () => setOrder(null),
  });
  const del = useCommand<DeletePanelsArgs>("storyboard.delete_panels", { onSuccess: () => toast.undoable("Deleted panel") });
  const [moveFor, setMoveFor] = useState<PanelDto | null>(null);
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
      keyboardCodes: { start: ["Space"], cancel: ["Escape"], end: ["Space"] },
    }),
  );
  const onDragEnd = (e: DragEndEvent) => {
    if (!e.over || e.active.id === e.over.id) return;
    const ids = shown.map((p) => p.id);
    const from = ids.indexOf(String(e.active.id));
    const to = ids.indexOf(String(e.over.id));
    if (from < 0 || to < 0) return;
    setOrder(moveItem(ids, from, to));
    reorder.mutate({ id: String(e.active.id), index: to });
  };
  return (
    <>
      <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={onDragEnd}>
        <SortableContext items={shown.map((p) => p.id)} strategy={rectSortingStrategy}>
          <div className="grid g3" role="group" aria-label="Storyboard panels" style={{ maxWidth: 980 }}>
            {shown.map((p, i) => (
              <PanelCard
                key={p.id}
                panel={p}
                number={i + 1}
                selected={p.id === selectedId}
                onOpen={() => onOpen(p.id)}
                menu={[
                  { label: "Open", onSelect: () => onOpen(p.id) },
                  { label: "Move to storyboard…", onSelect: () => setMoveFor(p), disabled: boards.length < 2 },
                  { label: "Link Shot…", onSelect: () => onOpen(p.id) },
                  { label: "Delete", danger: true, separatorBefore: true, onSelect: () => del.mutate({ ids: [p.id] }) },
                ]}
              />
            ))}
            <Menu
              items={addMenu}
              trigger={
                <button type="button" className="dz" style={{ minHeight: 140 }} aria-label="Add Panel">
                  <Plus size={18} />
                  <b className="sm">Add Panel</b>
                </button>
              }
            />
          </div>
        </SortableContext>
      </DndContext>
      <div className="hint" style={{ marginTop: 8 }}>
        Drag panels to change the order. Panel numbers{board.board.sceneLineageId ? " and linked shot numbers" : ""} follow the order
        automatically. Paste an image (Ctrl+V) to add it as a panel.
      </div>
      {moveFor && <MovePanelDialog panel={moveFor} boards={boards.filter((b) => b.id !== board.board.id)} onClose={() => setMoveFor(null)} />}
    </>
  );
}

function PanelCard({ panel, number, selected, onOpen, menu }: { panel: PanelDto; number: number; selected: boolean; onOpen: () => void; menu: MenuItemSpec[] }) {
  const go = useNav((s) => s.go);
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: panel.id });
  const src = assetSrc(panel.asset);
  const missing = panel.visualKind !== "placeholder" && !src;
  return (
    <ContextMenu items={menu}>
      <div
        ref={setNodeRef}
        className="pnl rel"
        style={{
          transform: CSS.Transform.toString(transform),
          transition,
          opacity: isDragging ? 0.6 : 1,
          outline: selected ? "2px solid var(--blue)" : undefined,
          cursor: "grab",
        }}
        {...attributes}
        {...listeners}
        aria-label={`Panel ${number}${panel.shotLabel ? `, shot ${panel.shotLabel}` : ""}${panel.description ? `: ${panel.description}` : ""}. Press Enter to open, Space to move.`}
        onClick={onOpen}
        onKeyDown={(e) => {
          listeners?.onKeyDown?.(e);
          if (e.key === "Enter") onOpen();
        }}
      >
        <div className={cx("pv", !src && "photo sketch")} style={{ position: "relative", height: 132, display: "grid", placeItems: "center" }}>
          {src ? (
            <img src={src} alt="" draggable={false} style={{ position: "absolute", inset: 0, width: "100%", height: "100%", objectFit: "cover" }} />
          ) : (
            <span className="xs muted">{missing ? "Image unavailable" : "Empty panel"}</span>
          )}
          <span className="badge">{number}</span>
          {panel.shotLabel && panel.shotId && (
            <button
              type="button"
              className="chip dark abs"
              style={{ right: 6, top: 6, border: 0 }}
              title={`Open shot ${panel.shotLabel}`}
              onPointerDown={(e) => e.stopPropagation()}
              onClick={(e) => {
                e.stopPropagation();
                go({ workspace: "production", sub: "shots", params: { shotId: panel.shotId as string } });
              }}
            >
              {panel.shotLabel}
            </button>
          )}
        </div>
        <div className="pt truncate2">{panel.description || <span className="muted">Add a short description</span>}</div>
      </div>
    </ContextMenu>
  );
}

function PanelDrawer({ panel, board, scenes, onClose }: { panel: PanelDto; board: StoryboardSummary; scenes: VisualScenes | undefined; onClose: () => void }) {
  const go = useNav((s) => s.go);
  const update = useCommand<UpdatePanelArgs, PanelDto>("storyboard.update_panel");
  const del = useCommand<DeletePanelsArgs>("storyboard.delete_panels", {
    onSuccess: () => {
      toast.undoable(`Deleted panel ${panel.number}`);
      onClose();
    },
  });
  const [desc, setDesc] = useState(panel.description);
  const [dur, setDur] = useState(formatSeconds(panel.durationMs));
  const [durError, setDurError] = useState<string | null>(null);
  const [note, setNote] = useState(panel.note ?? "");
  const [sketchOpen, setSketchOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const save = (patch: Omit<UpdatePanelArgs, "id">) => update.mutate({ id: panel.id, ...patch });

  const lineage = board.sceneLineageId && !board.sceneRemoved ? board.sceneLineageId : null;
  const shots = useOp<ShotDto[]>("shot.list", lineage ? { sceneLineageId: lineage } : undefined, SHOT_TABLES, { enabled: !!lineage });
  const [shotScene, setShotScene] = useState(scenes?.scenes[0]?.sceneId ?? "");

  const setVisual = async (visual: "image" | "sketch" | "placeholder", dataBase64?: string) => {
    setBusy(true);
    try {
      let path: string | undefined;
      if (visual === "image") {
        const [p] = await pickImages(false);
        if (!p) return;
        path = p;
      }
      await call("storyboard.set_panel_visual", { id: panel.id, visual, path, dataBase64 });
      toast.undoable(visual === "placeholder" ? "Cleared panel image" : "Changed panel image");
      setSketchOpen(false);
    } catch (e) {
      // The panel stays intact; another input method can be tried (UX §45).
      reportError(e);
    } finally {
      setBusy(false);
    }
  };
  const link = async (shotId: string | null) => {
    try {
      await call("storyboard.link_shot", { panelId: panel.id, shotId });
      toast.undoable(shotId ? "Linked panel to shot" : "Unlinked panel from shot");
    } catch (e) {
      reportError(e);
    }
  };
  const createShot = async () => {
    try {
      const s = await call<ShotDto>("storyboard.create_shot_from_panel", { panelId: panel.id, sceneId: lineage ? undefined : shotScene || undefined });
      toast.undoable(`Created shot ${s.label}`);
    } catch (e) {
      reportError(e);
    }
  };

  return (
    <Drawer
      open
      onClose={onClose}
      typeLabel="Storyboard panel"
      title={`Panel ${panel.number}${panel.shotLabel ? ` — Shot ${panel.shotLabel}` : ""}`}
      width="w"
      footer={
        <Button size="sm" variant="ghost" onClick={() => del.mutate({ ids: [panel.id] })}>
          Delete
        </Button>
      }
    >
      <Field label="Description" htmlFor="panel-desc" hint="A short description of what we see.">
        <TextArea id="panel-desc" rows={2} value={desc} onChange={(e) => setDesc(e.target.value)} onBlur={() => desc !== panel.description && save({ description: desc })} />
      </Field>
      <Field label="Visual">
        <div className="row wrap gap4">
          <Button size="xs" icon={<ImagePlus size={13} />} disabled={busy} onClick={() => void setVisual("image")}>
            Import Image…
          </Button>
          <Button size="xs" icon={<PenLine size={13} />} disabled={busy} onClick={() => setSketchOpen(true)}>
            Draw / Sketch…
          </Button>
          {panel.visualKind !== "placeholder" && (
            <Button size="xs" variant="ghost" disabled={busy} onClick={() => void setVisual("placeholder")}>
              Clear
            </Button>
          )}
        </div>
      </Field>
      <div className="grid g2" style={{ gap: 6 }}>
        <SuggestField label="Framing" value={panel.framing} options={SHOT_SIZES} onCommit={(v) => save({ framing: v })} />
        <SuggestField label="Camera movement" value={panel.movement} options={MOVEMENTS} onCommit={(v) => save({ movement: v })} />
        <SuggestField label="Angle" value={panel.angle} options={ANGLES} onCommit={(v) => save({ angle: v })} />
        <Field label="Duration (seconds)" htmlFor="panel-dur" error={durError}>
          <TextInput
            id="panel-dur"
            inputMode="decimal"
            value={dur}
            invalid={!!durError}
            placeholder="e.g. 3.5"
            onChange={(e) => setDur(e.target.value)}
            onBlur={() => {
              const ms = parseSeconds(dur);
              if (ms === null) {
                setDurError("Enter seconds between 0 and 3600.");
                return;
              }
              setDurError(null);
              if (ms !== (panel.durationMs ?? 0)) save({ durationMs: ms });
            }}
          />
        </Field>
      </div>
      <SuggestField label="Dialogue / sound note" value={panel.soundNote} placeholder="e.g. Phone rings off screen" onCommit={(v) => save({ soundNote: v })} />
      <Field label="Storyboard note" htmlFor="panel-note">
        <TextArea id="panel-note" rows={2} value={note} onChange={(e) => setNote(e.target.value)} onBlur={() => note !== (panel.note ?? "") && save({ note })} />
      </Field>

      <Field label="Shot">
        {panel.shotId && panel.shotLabel ? (
          <div className="row gap4">
            <button type="button" className="chip b" style={{ border: 0 }} onClick={() => go({ workspace: "production", sub: "shots", params: { shotId: panel.shotId as string } })}>
              Shot {panel.shotLabel}
            </button>
            <Button size="xs" variant="ghost" onClick={() => void link(null)}>
              Unlink
            </Button>
          </div>
        ) : lineage ? (
          <div className="row gap4">
            <select className="select" aria-label="Link Shot" value="" onChange={(e) => e.target.value && void link(e.target.value)} style={{ width: "auto", maxWidth: 260 }}>
              <option value="">Link Shot…</option>
              {(shots.data ?? []).map((s) => (
                <option key={s.id} value={s.id}>
                  {s.label} — {s.description}
                </option>
              ))}
            </select>
            <Button size="xs" onClick={() => void createShot()}>
              Create Shot
            </Button>
          </div>
        ) : scenes && scenes.scenes.length > 0 ? (
          <div className="row gap4">
            <select className="select" aria-label="Scene for the new shot" value={shotScene} onChange={(e) => setShotScene(e.target.value)} style={{ width: "auto", maxWidth: 260 }}>
              {scenes.scenes.map((s) => (
                <option key={s.lineageId} value={s.sceneId}>
                  {sceneLabel(s)}
                </option>
              ))}
            </select>
            <Button size="xs" onClick={() => void createShot()}>
              Create Shot
            </Button>
          </div>
        ) : (
          <span className="hint">Shots belong to screenplay scenes. Add a screenplay to plan shots.</span>
        )}
      </Field>
      <SketchDialog open={sketchOpen} onOpenChange={setSketchOpen} saving={busy} onSave={(png) => void setVisual("sketch", png)} />
    </Drawer>
  );
}

// ------------------------------------------------------------------ boards

function BoardMenu({
  board,
  scenes,
  onDeleted,
  onNewForScene,
}: {
  board: StoryboardSummary;
  scenes: VisualScenes | undefined;
  onDeleted: () => void;
  onNewForScene: () => void;
}) {
  const [renameOpen, setRenameOpen] = useState(false);
  const [sceneOpen, setSceneOpen] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const del = useCommand<{ id: string }>("storyboard.delete", {
    onSuccess: () => {
      toast.undoable(`Deleted storyboard “${board.name}”`);
      setConfirmOpen(false);
      onDeleted();
    },
  });
  const setScene = useCommand<{ id: string; sceneId: string | null }, StoryboardSummary>("storyboard.set_scene", {
    onSuccess: (b) => toast.undoable(b.sceneLineageId ? `Linked storyboard to Scene ${b.sceneNumber}` : "Storyboard is now standalone"),
  });
  return (
    <>
      <Menu
        align="end"
        trigger={
          <IconButton label="Storyboard actions">
            <MoreHorizontal size={16} />
          </IconButton>
        }
        items={[
          { label: "Rename…", onSelect: () => setRenameOpen(true) },
          { label: board.sceneLineageId ? "Change scene…" : "Link to a scene…", onSelect: () => setSceneOpen(true), disabled: !scenes?.scenes.length },
          ...(board.sceneLineageId ? [{ label: "Make standalone", onSelect: () => setScene.mutate({ id: board.id, sceneId: null }) }] : []),
          ...(board.sceneLineageId && !board.sceneRemoved ? [{ label: "Another storyboard for this scene…", onSelect: onNewForScene }] : []),
          { label: "Delete storyboard", danger: true, separatorBefore: true, onSelect: () => setConfirmOpen(true) },
        ]}
      />
      {renameOpen && <RenameDialog board={board} onClose={() => setRenameOpen(false)} />}
      {sceneOpen && scenes && (
        <PickSceneDialog
          title="Link storyboard to a scene"
          scenes={scenes}
          initial={board.sceneId ?? undefined}
          confirm="Link Scene"
          onClose={() => setSceneOpen(false)}
          onPick={(sceneId) => setScene.mutate({ id: board.id, sceneId }, { onSuccess: () => setSceneOpen(false) })}
        />
      )}
      <ConfirmDialog
        open={confirmOpen}
        onOpenChange={setConfirmOpen}
        title={`Delete storyboard “${board.name}”?`}
        confirmLabel="Delete Storyboard"
        danger
        busy={del.isPending}
        onConfirm={() => del.mutate({ id: board.id })}
      >
        Its {board.panelCount} panel{board.panelCount === 1 ? "" : "s"} go to Recently Deleted with it and can be restored. Shots are not
        affected.
      </ConfirmDialog>
    </>
  );
}

function RenameDialog({ board, onClose }: { board: StoryboardSummary; onClose: () => void }) {
  const [name, setName] = useState(board.name);
  const rename = useCommand<{ id: string; name: string; expectedRev?: number }>("storyboard.rename", {
    onSuccess: () => {
      toast.undoable("Renamed storyboard");
      onClose();
    },
  });
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Rename storyboard"
      size="sm"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!name.trim() || rename.isPending} onClick={() => rename.mutate({ id: board.id, name, expectedRev: board.rev })}>
            Rename
          </Button>
        </>
      }
    >
      <Field label="Name" required htmlFor="sb-rename">
        <TextInput id="sb-rename" autoFocus value={name} onChange={(e) => setName(e.target.value)} />
      </Field>
    </Dialog>
  );
}

function PickSceneDialog({
  title,
  scenes,
  initial,
  confirm,
  onClose,
  onPick,
}: {
  title: string;
  scenes: VisualScenes;
  initial?: string;
  confirm: string;
  onClose: () => void;
  onPick: (sceneId: string) => void;
}) {
  const [scene, setScene] = useState(initial && scenes.scenes.some((s) => s.sceneId === initial) ? initial : (scenes.scenes[0]?.sceneId ?? ""));
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title={title}
      size="md"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!scene} onClick={() => onPick(scene)}>
            {confirm}
          </Button>
        </>
      }
    >
      <Field label="Scene" htmlFor="pick-scene">
        <select id="pick-scene" className="select" value={scene} onChange={(e) => setScene(e.target.value)}>
          {scenes.scenes.map((s) => (
            <option key={s.lineageId} value={s.sceneId}>
              {sceneLabel(s)}
            </option>
          ))}
        </select>
      </Field>
    </Dialog>
  );
}

function NewStoryboardDialog({
  open,
  onOpenChange,
  scenes,
  defaultSceneId,
  onCreated,
}: {
  open: boolean;
  onOpenChange: (v: boolean) => void;
  scenes: VisualScenes | undefined;
  defaultSceneId?: string;
  onCreated: (b: StoryboardSummary) => void;
}) {
  const [name, setName] = useState("");
  const [sceneId, setSceneId] = useState<string>("");
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    if (open) {
      setName("");
      setError(null);
      setSceneId(defaultSceneId ?? "");
    }
  }, [open, defaultSceneId]);
  const create = useCommand<CreateStoryboardArgs, StoryboardSummary>("storyboard.create", {
    onSuccess: (b) => {
      toast.undoable(`Created storyboard “${b.name}”`);
      onOpenChange(false);
      onCreated(b);
    },
  });
  const submit = () => {
    if (!sceneId && !name.trim()) {
      setError("Give the storyboard a name, or choose a scene.");
      return;
    }
    create.mutate({ name: name.trim() || undefined, sceneId: sceneId || undefined });
  };
  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title="New Storyboard"
      size="md"
      footer={
        <>
          <Button onClick={() => onOpenChange(false)}>Cancel</Button>
          <Button variant="primary" disabled={create.isPending} onClick={submit}>
            Create Storyboard
          </Button>
        </>
      }
    >
      {scenes && scenes.scenes.length > 0 && (
        <Field label="Scene" htmlFor="new-sb-scene" hint="The scene heading and identity come from the screenplay.">
          <select id="new-sb-scene" className="select" value={sceneId} onChange={(e) => setSceneId(e.target.value)}>
            <option value="">No scene (standalone storyboard)</option>
            {scenes.scenes.map((s) => (
              <option key={s.lineageId} value={s.sceneId}>
                {sceneLabel(s)}
              </option>
            ))}
          </select>
        </Field>
      )}
      <Field label="Name" required={!sceneId} htmlFor="new-sb-name" error={error} hint={sceneId ? "Optional — defaults to the scene heading." : undefined}>
        <TextInput
          id="new-sb-name"
          autoFocus
          value={name}
          invalid={!!error}
          placeholder={sceneId ? "e.g. Alternative coverage" : "e.g. Title sequence"}
          onChange={(e) => {
            setName(e.target.value);
            setError(null);
          }}
          onKeyDown={(e) => e.key === "Enter" && submit()}
        />
      </Field>
    </Dialog>
  );
}

function MovePanelDialog({ panel, boards, onClose }: { panel: PanelDto; boards: StoryboardSummary[]; onClose: () => void }) {
  const [target, setTarget] = useState(boards[0]?.id ?? "");
  const move = useCommand<{ id: string; storyboardId: string }, PanelDto>("storyboard.move_panel", {
    onSuccess: () => {
      toast.undoable("Moved panel to another storyboard");
      onClose();
    },
  });
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title={`Move panel ${panel.number}`}
      size="md"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!target || move.isPending} onClick={() => move.mutate({ id: panel.id, storyboardId: target })}>
            Move Panel
          </Button>
        </>
      }
    >
      <Field label="Storyboard" htmlFor="move-panel-board" hint="The panel is added at the end. Its shot link is kept.">
        <select id="move-panel-board" className="select" value={target} onChange={(e) => setTarget(e.target.value)}>
          {boards.map((b) => (
            <option key={b.id} value={b.id}>
              {b.sceneNumber ? `Scene ${b.sceneNumber} — ` : ""}
              {b.name}
            </option>
          ))}
        </select>
      </Field>
    </Dialog>
  );
}
