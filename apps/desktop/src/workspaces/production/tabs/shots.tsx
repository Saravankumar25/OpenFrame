// Production → Shot Lists (FSD §33, §103; UX §46, §88; mocks 126–127).
// "How will I capture this scene?" — shots are planned scene by scene, ordered
// by drag and drop; labels (12A, 12B…) are generated from order in Rust and are
// never typed or renumbered by hand. Only a description is required.

import { useEffect, useMemo, useRef, useState, type FormEvent, type ReactNode } from "react";
import {
  DndContext,
  KeyboardSensor,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
  type DragEndEvent,
} from "@dnd-kit/core";
import {
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { Clapperboard, GripVertical, ImageIcon, MoreHorizontal, Plus, X } from "lucide-react";
import {
  Button,
  Checkbox,
  Chip,
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
  type MenuItems,
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
  moveItem,
  pickImages,
  sceneLabel,
  useVisualScenes,
} from "../../../api/visual";
import { NoScriptState, RemovedSceneBanner, SceneReviewBanner, SuggestField, useVisualView } from "../visual/shared";
import type { ShotDto } from "../../../ipc/generated/ShotDto";
import type { VisualScenes } from "../../../ipc/generated/VisualScenes";
import type { StoryboardSummary } from "../../../ipc/generated/StoryboardSummary";
import type { StoryboardDto } from "../../../ipc/generated/StoryboardDto";
import type { CreateShotArgs } from "../../../ipc/generated/CreateShotArgs";
import type { UpdateShotArgs } from "../../../ipc/generated/UpdateShotArgs";
import type { ReorderShotArgs } from "../../../ipc/generated/ReorderShotArgs";
import type { DeleteShotsArgs } from "../../../ipc/generated/DeleteShotsArgs";
import type { ShotIdArgs } from "../../../ipc/generated/ShotIdArgs";
import type { PanelDto } from "../../../ipc/generated/PanelDto";
import { ShotListExportButton } from "../../../features/export/buttons";

export const tab = { id: "shots", label: "Shot Lists", order: 60 };

const HINT = "Add only the shots you need. Camera details are optional. Shot letters are generated from order — you never renumber by hand.";

export default function ShotsTab() {
  const route = useNav((s) => s.route);
  const view = useVisualView();
  const scenesQ = useVisualScenes();
  const scenes = scenesQ.data;

  // Deep links from search / Scene Hub: { shotId } or { sceneId }.
  const focusShotId = route.params?.shotId;
  const focusSceneId = route.params?.sceneId;
  const focusLineage = route.params?.sceneLineageId;
  const focusShot = useOp<ShotDto>("shot.get", focusShotId ? { id: focusShotId } : undefined, SHOT_TABLES, {
    enabled: !!focusShotId,
  });
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const consumed = useRef<string | null>(null);
  useEffect(() => {
    const key = `${focusShotId ?? ""}|${focusSceneId ?? ""}|${focusLineage ?? ""}`;
    if (consumed.current === key) return;
    if (focusShotId && focusShot.data) {
      consumed.current = key;
      if (focusShot.data.sceneRemoved) view.set({ showRemoved: true });
      view.set({ shotScene: focusShot.data.sceneLineageId });
      setSelectedId(focusShot.data.id);
    } else if (!focusShotId && (focusSceneId || focusLineage) && scenes) {
      consumed.current = key;
      const sc =
        scenes.scenes.find((s) => s.sceneId === focusSceneId || s.lineageId === focusSceneId) ??
        scenes.scenes.find((s) => !!focusLineage && s.lineageId === focusLineage);
      if (sc) view.set({ shotScene: sc.lineageId });
    }
  }, [focusShotId, focusSceneId, focusLineage, focusShot.data, scenes, view]);

  const sceneKey = resolveSceneKey(view.shotScene, scenes, view.showRemoved);
  const shotsQ = useOp<ShotDto[]>(
    "shot.list",
    sceneKey === "all" ? { includeRemoved: view.showRemoved } : sceneKey ? { sceneLineageId: sceneKey } : undefined,
    SHOT_TABLES,
    { enabled: !!sceneKey },
  );
  const shots = useMemo(() => shotsQ.data ?? [], [shotsQ.data]);
  const scene = scenes?.scenes.find((s) => s.lineageId === sceneKey);
  const removed = scenes?.removed.find((r) => r.lineageId === sceneKey);
  const selected = shots.find((s) => s.id === selectedId) ?? (focusShot.data?.id === selectedId ? focusShot.data : undefined);

  const [picked, setPicked] = useState<Set<string>>(new Set());
  const composerRef = useRef<HTMLInputElement>(null);
  const [copyOpen, setCopyOpen] = useState(false);

  const del = useCommand<DeleteShotsArgs>("shot.delete", {
    onSuccess: (_, a) => {
      toast.undoable(a.ids.length === 1 ? "Deleted shot" : `Deleted ${a.ids.length} shots`);
      setPicked(new Set());
      if (selectedId && a.ids.includes(selectedId)) setSelectedId(null);
    },
  });

  if (scenesQ.isLoading) {
    return (
      <div>
        <Header onAdd={undefined} />
        <Skeleton h={34} w={420} />
      </div>
    );
  }
  if (!scenes || (scenes.sourceKind === "none" && scenes.removed.length === 0)) {
    return (
      <div>
        <Header onAdd={undefined} />
        <NoScriptState what="Shots" />
      </div>
    );
  }

  const canAdd = !!scene;
  const groups = groupByScene(shots);

  return (
    <div style={{ position: "relative", minHeight: "100%" }}>
      <Header
        exportButton={
          <ShotListExportButton
            scene={scene ? { id: scene.sceneId, label: `${scene.number} — ${scene.heading}` } : null}
            scenes={scenes.scenes.map((s) => ({ id: s.sceneId, label: `${s.number} — ${s.heading}` }))}
          />
        }
        onAdd={
          canAdd
            ? () => {
                // Focus the composer; if a description is already typed, add it.
                const input = composerRef.current;
                input?.focus();
                if (input?.value.trim()) input.form?.requestSubmit();
              }
            : undefined
        }
      />
      {scenes.sourceLabel && <div className="hint" style={{ marginTop: -6, marginBottom: 8 }}>Scenes from {scenes.sourceLabel}</div>}
      <div className="toolbar">
        <select
          className="select"
          aria-label="Scene"
          style={{ maxWidth: 440, width: "auto" }}
          value={sceneKey ?? ""}
          onChange={(e) => {
            view.set({ shotScene: e.target.value });
            setSelectedId(null);
            setPicked(new Set());
          }}
        >
          <option value="all">All scenes</option>
          <optgroup label={scenes.sourceLabel || "Scenes"}>
            {scenes.scenes.map((s) => (
              <option key={s.lineageId} value={s.lineageId}>
                {sceneLabel(s)}
                {s.needsReview ? " — review" : ""}
              </option>
            ))}
          </optgroup>
          {view.showRemoved && scenes.removed.length > 0 && (
            <optgroup label="Removed from script">
              {scenes.removed.map((r) => (
                <option key={r.lineageId} value={r.lineageId}>
                  {r.heading || "Removed scene"} ({r.shotCount})
                </option>
              ))}
            </optgroup>
          )}
        </select>
        <Chip>
          {shots.length} {shots.length === 1 ? "shot" : "shots"}
        </Chip>
        <span className="sp" />
        {picked.size > 1 && (
          <Button size="sm" variant="danger" onClick={() => del.mutate({ ids: [...picked] })}>
            Delete {picked.size} shots
          </Button>
        )}
        {scenes.removed.length > 0 && (
          <Checkbox
            checked={view.showRemoved}
            onChange={(v) => view.set({ showRemoved: v, shotScene: !v && removed ? null : view.shotScene })}
            label="Show removed scenes"
          />
        )}
        {scene && shots.length > 0 && (
          <Menu
            align="end"
            trigger={
              <IconButton label="More scene actions">
                <MoreHorizontal size={16} />
              </IconButton>
            }
            items={[{ label: "Copy planning to another scene…", onSelect: () => setCopyOpen(true) }]}
          />
        )}
      </div>

      {scene && <SceneReviewBanner scene={scene} what="shots and storyboards" />}
      {removed && <RemovedSceneBanner heading={removed.heading} what="shots" />}

      <div style={{ maxWidth: 780 }}>
        {shotsQ.isLoading ? (
          <Skeleton h={120} />
        ) : sceneKey === "all" ? (
          groups.length === 0 ? (
            <EmptyState title="No shots yet">Choose a scene to start its shot list.</EmptyState>
          ) : (
            groups.map((g) => (
              <section key={g.lineage} aria-label={g.title} style={{ marginBottom: 14 }}>
                <div className="row" style={{ marginBottom: 6 }}>
                  <b className="grow">{g.title}</b>
                  {g.needsReview && <Chip tone="y">Scene changed since planning</Chip>}
                  <Button size="xs" onClick={() => view.set({ shotScene: g.lineage })}>
                    Open scene
                  </Button>
                </div>
                <ShotList
                  shots={g.shots}
                  scenes={scenes}
                  selectedId={selectedId}
                  picked={picked}
                  setPicked={setPicked}
                  onOpen={setSelectedId}
                  onDelete={(ids) => del.mutate({ ids })}
                />
              </section>
            ))
          )
        ) : (
          <>
            {shots.length === 0 && !scene ? null : shots.length === 0 ? (
              <EmptyState title="No shots in this scene yet">Add only the shots you need. Camera details are optional.</EmptyState>
            ) : (
              <ShotList
                shots={shots}
                scenes={scenes}
                selectedId={selectedId}
                picked={picked}
                setPicked={setPicked}
                onOpen={setSelectedId}
                onDelete={(ids) => del.mutate({ ids })}
              />
            )}
            {scene && <Composer inputRef={composerRef} sceneId={scene.sceneId} />}
          </>
        )}
        <div className="hint" style={{ marginTop: 6 }}>
          {HINT}
        </div>
      </div>

      {selected && (
        <ShotDrawer key={selected.id} shot={selected} scenes={scenes} onClose={() => setSelectedId(null)} onDeleted={() => setSelectedId(null)} />
      )}
      {scene && (
        <CopyPlanningDialog open={copyOpen} onOpenChange={setCopyOpen} fromLineage={scene.lineageId} scenes={scenes} />
      )}
    </div>
  );
}

function Header({ onAdd, exportButton }: { onAdd: (() => void) | undefined; exportButton?: ReactNode }) {
  return (
    <PageHeader
      title="Shot Lists"
      sub="How will I capture this scene?"
      actions={
        <>
          {exportButton}
          <Button variant="primary" icon={<Plus size={15} />} disabled={!onAdd} title={onAdd ? undefined : "Choose a scene first"} onClick={onAdd}>
            Add Shot
          </Button>
        </>
      }
    />
  );
}

function resolveSceneKey(wanted: string | null, scenes: VisualScenes | undefined, showRemoved: boolean): string | null {
  if (!scenes) return null;
  if (wanted === "all") return "all";
  if (wanted && scenes.scenes.some((s) => s.lineageId === wanted)) return wanted;
  if (wanted && showRemoved && scenes.removed.some((r) => r.lineageId === wanted)) return wanted;
  return scenes.scenes[0]?.lineageId ?? (scenes.removed.length > 0 ? "all" : null);
}

interface Group {
  lineage: string;
  title: string;
  needsReview: boolean;
  shots: ShotDto[];
}

function groupByScene(shots: ShotDto[]): Group[] {
  const out: Group[] = [];
  const byLineage = new Map<string, Group>();
  for (const s of shots) {
    let g = byLineage.get(s.sceneLineageId);
    if (!g) {
      const title = s.sceneRemoved
        ? `Removed scene — ${s.sceneHeading || "untitled"}`
        : `Scene ${s.sceneNumber ?? ""} — ${s.sceneHeading || "Untitled scene"}`;
      g = { lineage: s.sceneLineageId, title, needsReview: false, shots: [] };
      out.push(g);
      byLineage.set(s.sceneLineageId, g);
    }
    g.needsReview ||= s.needsReview;
    g.shots.push(s);
  }
  return out;
}

// ------------------------------------------------------------------ list

function ShotList({
  shots,
  scenes,
  selectedId,
  picked,
  setPicked,
  onOpen,
  onDelete,
}: {
  shots: ShotDto[];
  scenes: VisualScenes;
  selectedId: string | null;
  picked: Set<string>;
  setPicked: (s: Set<string>) => void;
  onOpen: (id: string) => void;
  onDelete: (ids: string[]) => void;
}) {
  const [order, setOrder] = useState<string[] | null>(null);
  const ids = shots.map((s) => s.id);
  // Drop the optimistic order once Rust reports the new order.
  const idsKey = ids.join(",");
  useEffect(() => setOrder(null), [idsKey]);
  const shown = order ? order.map((id) => shots.find((s) => s.id === id)).filter((s): s is ShotDto => !!s) : shots;
  const reorder = useCommand<ReorderShotArgs>("shot.reorder", {
    onSuccess: () => toast.undoable("Reordered shots"),
    onError: () => setOrder(null),
  });
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 4 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );
  const [moveFor, setMoveFor] = useState<ShotDto | null>(null);
  const [attachFor, setAttachFor] = useState<ShotDto | null>(null);
  const duplicate = useCommand<ShotIdArgs, ShotDto>("shot.duplicate", { onSuccess: (s) => toast.undoable(`Duplicated as ${s.label}`) });
  const makePanel = useCommand<ShotIdArgs, PanelDto>("shot.create_panel", {
    onSuccess: () => toast.undoable("Created storyboard panel from shot"),
  });

  const onDragEnd = (e: DragEndEvent) => {
    if (!e.over || e.active.id === e.over.id) return;
    const current = shown.map((s) => s.id);
    const from = current.indexOf(String(e.active.id));
    const to = current.indexOf(String(e.over.id));
    if (from < 0 || to < 0) return;
    setOrder(moveItem(current, from, to));
    reorder.mutate({ id: String(e.active.id), index: to });
  };

  const click = (s: ShotDto, ev: React.MouseEvent) => {
    if (ev.ctrlKey || ev.metaKey) {
      const n = new Set(picked);
      if (n.has(s.id)) n.delete(s.id);
      else n.add(s.id);
      setPicked(n);
      return;
    }
    if (ev.shiftKey && picked.size > 0) {
      const all = shown.map((x) => x.id);
      const anchor = all.findIndex((id) => picked.has(id));
      const at = all.indexOf(s.id);
      const [a, b] = anchor < at ? [anchor, at] : [at, anchor];
      setPicked(new Set(all.slice(a, b + 1)));
      return;
    }
    setPicked(new Set([s.id]));
    onOpen(s.id);
  };

  const menu = (s: ShotDto): MenuItemSpec[] => [
    { label: "Open", onSelect: () => onOpen(s.id) },
    { label: "Duplicate", onSelect: () => duplicate.mutate({ id: s.id }) },
    { label: "Move to scene…", onSelect: () => setMoveFor(s), disabled: scenes.scenes.length < 2 },
    { label: "Attach Storyboard…", onSelect: () => setAttachFor(s), separatorBefore: true },
    { label: "Create storyboard panel", onSelect: () => makePanel.mutate({ id: s.id }), disabled: s.sceneRemoved && s.panels.length === 0 },
    {
      label: picked.size > 1 && picked.has(s.id) ? `Delete ${picked.size} shots` : "Delete",
      danger: true,
      separatorBefore: true,
      onSelect: () => onDelete(picked.size > 1 && picked.has(s.id) ? [...picked] : [s.id]),
    },
  ];

  return (
    <>
      <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={onDragEnd}>
        <SortableContext items={shown.map((s) => s.id)} strategy={verticalListSortingStrategy}>
          <div
            role="list"
            aria-label="Shots"
            onKeyDown={(e) => {
              if ((e.key === "Delete" || e.key === "Backspace") && picked.size > 0 && (e.target as HTMLElement).tagName !== "INPUT") {
                e.preventDefault();
                onDelete([...picked]);
              }
            }}
          >
            {shown.map((s) => (
              <ShotRow
                key={s.id}
                shot={s}
                selected={s.id === selectedId || picked.has(s.id)}
                onClick={(ev) => click(s, ev)}
                menu={() => menu(s)}
              />
            ))}
          </div>
        </SortableContext>
      </DndContext>
      {moveFor && <MoveShotDialog shot={moveFor} scenes={scenes} onClose={() => setMoveFor(null)} />}
      {attachFor && <AttachPanelDialog shot={attachFor} onClose={() => setAttachFor(null)} />}
    </>
  );
}

function ShotRow({ shot, selected, onClick, menu }: { shot: ShotDto; selected: boolean; onClick: (e: React.MouseEvent) => void; menu: MenuItems }) {
  const { attributes, listeners, setNodeRef, setActivatorNodeRef, transform, transition, isDragging } = useSortable({ id: shot.id });
  return (
    <ContextMenu items={menu}>
      <div
        ref={setNodeRef}
        role="listitem"
        className={cx("shot", selected && "sel")}
        style={{ transform: CSS.Transform.toString(transform), transition, opacity: isDragging ? 0.65 : 1, cursor: "default" }}
        onClick={onClick}
      >
        <button
          ref={setActivatorNodeRef}
          type="button"
          className="iconbtn"
          aria-label={`Reorder shot ${shot.label}`}
          style={{ cursor: "grab", width: 22, height: 22 }}
          onClick={(e) => e.stopPropagation()}
          {...attributes}
          {...listeners}
        >
          <GripVertical size={15} />
        </button>
        <span className="lab" style={{ width: "auto", minWidth: 34 }}>
          {shot.label}
        </span>
        {shot.size && <Chip>{shot.size}</Chip>}
        <button
          type="button"
          className="grow"
          style={{ fontWeight: 600, textAlign: "left", background: "transparent", border: 0, padding: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}
          aria-label={`Open shot ${shot.label}: ${shot.description}`}
        >
          {shot.description}
        </button>
        {shot.movement && <Chip tone="out">{shot.movement}</Chip>}
        {shot.angle && <Chip tone="out">{shot.angle}</Chip>}
        {shot.lens && <Chip tone="out">{shot.lens}</Chip>}
        {shot.panels.length > 0 && (
          <span title={`${shot.panels.length} storyboard panel${shot.panels.length === 1 ? "" : "s"}`} className="muted" aria-label="Has storyboard panel">
            <Clapperboard size={14} />
          </span>
        )}
        {shot.referenceAsset && (
          <span title="Has reference image" className="muted" aria-label="Has reference image">
            <ImageIcon size={14} />
          </span>
        )}
      </div>
    </ContextMenu>
  );
}

function Composer({ inputRef, sceneId }: { inputRef: React.RefObject<HTMLInputElement | null>; sceneId: string }) {
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const create = useCommand<CreateShotArgs, ShotDto>("shot.create", {
    onSuccess: () => {
      setText("");
      setError(null);
      inputRef.current?.focus();
    },
  });
  const submit = (e: FormEvent) => {
    e.preventDefault();
    if (!text.trim()) {
      setError("Describe the shot to add it.");
      inputRef.current?.focus();
      return;
    }
    create.mutate({ sceneId, description: text });
  };
  return (
    <form onSubmit={submit} className="row" style={{ marginTop: 6, alignItems: "flex-start" }}>
      <div className="grow">
        <TextInput
          ref={inputRef}
          aria-label="New shot description"
          placeholder="Describe the next shot — e.g. Wide: Arjun enters the station"
          value={text}
          invalid={!!error}
          onChange={(e) => {
            setText(e.target.value);
            if (error) setError(null);
          }}
        />
        {error && (
          <div className="errtxt" role="alert">
            {error}
          </div>
        )}
      </div>
      <Button type="submit" icon={<Plus size={14} />} disabled={create.isPending}>
        Add Shot
      </Button>
    </form>
  );
}

// ------------------------------------------------------------------ drawer

function ShotDrawer({ shot, scenes, onClose, onDeleted }: { shot: ShotDto; scenes: VisualScenes; onClose: () => void; onDeleted: () => void }) {
  const go = useNav((s) => s.go);
  const update = useCommand<UpdateShotArgs, ShotDto>("shot.update");
  const del = useCommand<DeleteShotsArgs>("shot.delete", {
    onSuccess: () => {
      toast.undoable(`Deleted shot ${shot.label}`);
      onDeleted();
    },
  });
  const duplicate = useCommand<ShotIdArgs, ShotDto>("shot.duplicate", { onSuccess: (s) => toast.undoable(`Duplicated as ${s.label}`) });
  const makePanel = useCommand<ShotIdArgs, PanelDto>("shot.create_panel", { onSuccess: () => toast.undoable("Created storyboard panel from shot") });
  const detach = useCommand<{ shotId: string; panelId: string }>("shot.detach_panel");
  const characters = useOp<string[]>("visual.scene_characters", { sceneId: shot.sceneId }, ["screenplay_element", "screenplay_scene"], {
    enabled: !shot.sceneRemoved,
  });

  const [desc, setDesc] = useState(shot.description);
  const [descError, setDescError] = useState<string | null>(null);
  const hasTech = !!(shot.size || shot.movement || shot.angle || shot.lens || shot.cameraNotes || shot.characters.length || shot.soundNote);
  const [techOpen, setTechOpen] = useState(hasTech);
  const [attachOpen, setAttachOpen] = useState(false);

  const save = (patch: Omit<UpdateShotArgs, "id">) => update.mutate({ id: shot.id, ...patch });
  const commitDesc = () => {
    if (desc === shot.description) return;
    if (!desc.trim()) {
      setDescError("Description is required.");
      return;
    }
    setDescError(null);
    save({ description: desc });
  };

  const title = [shot.label, shot.size, shot.description].filter(Boolean).join(" — ");
  const refSrc = assetSrc(shot.referenceAsset);

  const chooseReference = async () => {
    try {
      const [path] = await pickImages(false);
      if (!path) return;
      await call("shot.set_reference_image", { id: shot.id, path });
      toast.undoable("Set reference image");
    } catch (e) {
      reportError(e);
    }
  };
  const removeReference = async () => {
    try {
      await call("shot.set_reference_image", { id: shot.id });
      toast.undoable("Removed reference image");
    } catch (e) {
      reportError(e);
    }
  };

  return (
    <Drawer
      open
      onClose={onClose}
      typeLabel="Shot"
      title={title}
      width="w"
      footer={
        <>
          <Button size="sm" variant="ghost" onClick={() => del.mutate({ ids: [shot.id] })}>
            Delete
          </Button>
          <Button size="sm" onClick={() => duplicate.mutate({ id: shot.id })}>
            Duplicate
          </Button>
        </>
      }
    >
      {shot.needsReview && !shot.sceneRemoved && (
        <div className="hint" style={{ color: "#8a6510", marginBottom: 8 }}>
          Scene {shot.sceneNumber} changed since this shot was planned. Review it against the script.
        </div>
      )}
      <Field label="Description" required error={descError} htmlFor="shot-desc">
        <TextArea
          id="shot-desc"
          rows={2}
          value={desc}
          invalid={!!descError}
          onChange={(e) => setDesc(e.target.value)}
          onBlur={commitDesc}
        />
      </Field>

      <div className="row" style={{ marginBottom: 6 }}>
        <button type="button" className="chip a" aria-expanded={techOpen} onClick={() => setTechOpen(!techOpen)} style={{ border: 0 }}>
          Technical details
        </button>
        <span className="hint">optional — hidden until you need them</span>
      </div>
      {techOpen && (
        <>
          <div className="grid g2" style={{ gap: 6 }}>
            <SuggestField label="Shot size" value={shot.size} options={SHOT_SIZES} onCommit={(v) => save({ size: v })} />
            <SuggestField label="Movement" value={shot.movement} options={MOVEMENTS} onCommit={(v) => save({ movement: v })} />
            <SuggestField label="Angle" value={shot.angle} options={ANGLES} onCommit={(v) => save({ angle: v })} />
            <SuggestField label="Lens" value={shot.lens} placeholder="e.g. 35mm" onCommit={(v) => save({ lens: v })} />
          </div>
          <SuggestField
            label="Camera notes"
            value={shot.cameraNotes}
            placeholder="e.g. follow focus to the folder"
            onCommit={(v) => save({ cameraNotes: v })}
          />
          <CharactersField value={shot.characters} suggestions={characters.data ?? []} onChange={(v) => save({ characters: v })} />
          <SuggestField label="Sound note" value={shot.soundNote} placeholder="e.g. rain on window" onCommit={(v) => save({ soundNote: v })} />
        </>
      )}

      <Field label="Reference image" hint="A visual aid for this shot. It is copied into the project.">
        {shot.referenceAsset ? (
          <div className="row" style={{ alignItems: "flex-start" }}>
            {refSrc ? (
              <img src={refSrc} alt="Shot reference" style={{ width: 120, height: 72, objectFit: "cover", borderRadius: 6, border: "1px solid var(--line2)" }} />
            ) : (
              <div className="photo sketch" style={{ width: 120, height: 72, borderRadius: 6, display: "grid", placeItems: "center", fontSize: 11 }}>
                Image unavailable
              </div>
            )}
            <div className="row wrap gap4">
              <Button size="xs" onClick={() => void chooseReference()}>
                Replace…
              </Button>
              <Button size="xs" variant="ghost" onClick={() => void removeReference()}>
                Remove
              </Button>
            </div>
          </div>
        ) : (
          <Button size="xs" icon={<ImageIcon size={13} />} onClick={() => void chooseReference()}>
            Add reference image…
          </Button>
        )}
      </Field>

      <Field label="Storyboard">
        <div className="row wrap" style={{ gap: 6 }}>
          {shot.panels.map((p) => (
            <span key={p.panelId} className="chip b" title={p.storyboardName}>
              <button
                type="button"
                style={{ background: "transparent", border: 0, padding: 0, color: "inherit", font: "inherit" }}
                onClick={() => go({ workspace: "production", sub: "storyboards", params: { storyboardId: p.storyboardId, panelId: p.panelId } })}
              >
                Storyboard panel {p.number}
              </button>
              <button
                type="button"
                aria-label={`Detach storyboard panel ${p.number}`}
                style={{ background: "transparent", border: 0, padding: 0, color: "inherit", display: "inline-flex" }}
                onClick={() => detach.mutate({ shotId: shot.id, panelId: p.panelId }, { onSuccess: () => toast.undoable("Detached storyboard panel") })}
              >
                <X size={12} />
              </button>
            </span>
          ))}
          <Button size="xs" onClick={() => setAttachOpen(true)}>
            Attach…
          </Button>
          <Button size="xs" onClick={() => makePanel.mutate({ id: shot.id })} disabled={makePanel.isPending || (shot.sceneRemoved && shot.panels.length === 0)}>
            Create panel
          </Button>
        </div>
      </Field>
      {scenes.sourceLabel && <div className="hint">Scene {shot.sceneNumber ?? "—"} · {shot.sceneHeading}</div>}
      {attachOpen && <AttachPanelDialog shot={shot} onClose={() => setAttachOpen(false)} />}
    </Drawer>
  );
}

function CharactersField({ value, suggestions, onChange }: { value: string[]; suggestions: string[]; onChange: (v: string[]) => void }) {
  const [text, setText] = useState("");
  const add = () => {
    const t = text.trim();
    if (!t) return;
    if (!value.some((v) => v.toLowerCase() === t.toLowerCase())) onChange([...value, t]);
    setText("");
  };
  const unused = suggestions.filter((s) => !value.some((v) => v.toLowerCase() === s.toLowerCase()));
  return (
    <Field label="Characters" htmlFor="shot-chars" hint={unused.length ? `In this scene: ${unused.join(", ")}` : undefined}>
      <div className="row wrap" style={{ gap: 4, marginBottom: value.length ? 4 : 0 }}>
        {value.map((c) => (
          <span key={c} className="chip">
            {c}
            <button
              type="button"
              aria-label={`Remove ${c}`}
              style={{ background: "transparent", border: 0, padding: 0, display: "inline-flex" }}
              onClick={() => onChange(value.filter((x) => x !== c))}
            >
              <X size={11} />
            </button>
          </span>
        ))}
      </div>
      <TextInput
        id="shot-chars"
        list="shot-chars-list"
        placeholder="Add a character and press Enter"
        value={text}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            add();
          }
        }}
        onBlur={add}
      />
      <datalist id="shot-chars-list">
        {unused.map((s) => (
          <option key={s} value={s} />
        ))}
      </datalist>
    </Field>
  );
}

// ------------------------------------------------------------------ dialogs

function MoveShotDialog({ shot, scenes, onClose }: { shot: ShotDto; scenes: VisualScenes; onClose: () => void }) {
  const others = scenes.scenes.filter((s) => s.lineageId !== shot.sceneLineageId);
  const [target, setTarget] = useState(others[0]?.sceneId ?? "");
  const move = useCommand<{ id: string; sceneId: string }, ShotDto>("shot.move", {
    onSuccess: (s) => {
      toast.undoable(`Moved shot to Scene ${s.sceneNumber} as ${s.label}`);
      onClose();
    },
  });
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title={`Move shot ${shot.label}`}
      size="md"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!target || move.isPending} onClick={() => move.mutate({ id: shot.id, sceneId: target })}>
            Move Shot
          </Button>
        </>
      }
    >
      <Field label="Scene" htmlFor="move-scene" hint="The shot is added at the end of that scene's shot list.">
        <select id="move-scene" className="select" value={target} onChange={(e) => setTarget(e.target.value)}>
          {others.map((s) => (
            <option key={s.lineageId} value={s.sceneId}>
              {sceneLabel(s)}
            </option>
          ))}
        </select>
      </Field>
    </Dialog>
  );
}

function AttachPanelDialog({ shot, onClose }: { shot: ShotDto; onClose: () => void }) {
  const boards = useOp<StoryboardSummary[]>("storyboard.list", {}, STORYBOARD_TABLES);
  const sorted = useMemo(() => {
    const all = boards.data ?? [];
    return [...all.filter((b) => b.sceneLineageId === shot.sceneLineageId), ...all.filter((b) => b.sceneLineageId !== shot.sceneLineageId)];
  }, [boards.data, shot.sceneLineageId]);
  const [boardId, setBoardId] = useState<string | null>(null);
  const current = boardId ?? sorted[0]?.id ?? null;
  const board = useOp<StoryboardDto>("storyboard.get", current ? { id: current } : undefined, STORYBOARD_TABLES, { enabled: !!current });
  const attach = useCommand<{ shotId: string; panelId: string }, ShotDto>("shot.attach_panel", {
    onSuccess: () => {
      toast.undoable(`Attached storyboard panel to ${shot.label}`);
      onClose();
    },
  });
  return (
    <Dialog open onOpenChange={(v) => !v && onClose()} title={`Attach storyboard panel to ${shot.label}`} size="lg" footer={<Button onClick={onClose}>Cancel</Button>}>
      {boards.isLoading ? (
        <Skeleton h={80} />
      ) : sorted.length === 0 ? (
        <div className="muted">There are no storyboards yet. Use “Create panel” to start one from this shot.</div>
      ) : (
        <>
          <Field label="Storyboard" htmlFor="attach-board">
            <select id="attach-board" className="select" value={current ?? ""} onChange={(e) => setBoardId(e.target.value)}>
              {sorted.map((b) => (
                <option key={b.id} value={b.id}>
                  {b.sceneNumber ? `Scene ${b.sceneNumber} — ` : ""}
                  {b.name} ({b.panelCount})
                </option>
              ))}
            </select>
          </Field>
          <div className="grid g4" role="list" aria-label="Panels">
            {(board.data?.panels ?? []).map((p) => {
              const src = assetSrc(p.asset);
              const already = p.shotId === shot.id;
              return (
                <button
                  key={p.id}
                  type="button"
                  role="listitem"
                  className="pnl rel"
                  disabled={already || attach.isPending}
                  style={{ textAlign: "left", padding: 0, opacity: already ? 0.5 : 1 }}
                  onClick={() => attach.mutate({ shotId: shot.id, panelId: p.id })}
                  aria-label={`Panel ${p.number}${p.description ? `: ${p.description}` : ""}${already ? " (already attached)" : ""}`}
                >
                  <div className={cx("pv", !src && "photo sketch")} style={{ position: "relative", height: 70 }}>
                    {src && <img src={src} alt="" style={{ width: "100%", height: "100%", objectFit: "cover" }} />}
                    <span className="badge">{p.number}</span>
                    {p.shotLabel && (
                      <span className="chip dark abs" style={{ right: 6, top: 6 }}>
                        {p.shotLabel}
                      </span>
                    )}
                  </div>
                  <div className="pt truncate2">{p.description || <span className="muted">No description</span>}</div>
                </button>
              );
            })}
          </div>
        </>
      )}
    </Dialog>
  );
}

function CopyPlanningDialog({
  open,
  onOpenChange,
  fromLineage,
  scenes,
}: {
  open: boolean;
  onOpenChange: (v: boolean) => void;
  fromLineage: string;
  scenes: VisualScenes;
}) {
  const others = scenes.scenes.filter((s) => s.lineageId !== fromLineage);
  const [target, setTarget] = useState("");
  const chosen = target || others[0]?.sceneId || "";
  const copy = useCommand<{ fromSceneLineageId: string; toSceneId: string }, { shots: number; storyboards: number }>("shot.copy_planning", {
    onSuccess: (r) => {
      toast.undoable(`Copied ${r.shots} shot${r.shots === 1 ? "" : "s"} and ${r.storyboards} storyboard${r.storyboards === 1 ? "" : "s"}`);
      onOpenChange(false);
    },
  });
  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title="Copy planning to another scene"
      size="md"
      footer={
        <>
          <Button onClick={() => onOpenChange(false)}>Cancel</Button>
          <Button variant="primary" disabled={!chosen || copy.isPending} onClick={() => copy.mutate({ fromSceneLineageId: fromLineage, toSceneId: chosen })}>
            Copy Planning
          </Button>
        </>
      }
    >
      <p style={{ marginTop: 0 }}>
        Copies this scene's shots and scene storyboards as new, independent items — useful when a scene was duplicated in the script.
      </p>
      <Field label="Copy to" htmlFor="copy-target">
        <select id="copy-target" className="select" value={chosen} onChange={(e) => setTarget(e.target.value)}>
          {others.map((s) => (
            <option key={s.lineageId} value={s.sceneId}>
              {sceneLabel(s)}
            </option>
          ))}
        </select>
      </Field>
    </Dialog>
  );
}
