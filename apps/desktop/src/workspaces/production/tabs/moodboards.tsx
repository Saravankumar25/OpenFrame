// Production → Moodboards (FSD §31, §101; UX §44, §87.1; mocks 121–122).
// "What should this film feel like?" — named boards with a freeform canvas of
// image, note and link tiles. Tiles move (also as a multi-selection), resize and
// stack; the gesture is shown locally and saved in Rust as one undoable step.

import { useEffect, useMemo, useRef, useState, type ClipboardEvent, type KeyboardEvent, type PointerEvent as RPointerEvent } from "react";
import { ArrowDownToLine, ArrowUpToLine, ExternalLink, ImagePlus, Link2, Lock, MoreHorizontal, Plus, StickyNote, Trash2 } from "lucide-react";
import {
  Button,
  Checkbox,
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
  Segmented,
  Skeleton,
  TextArea,
  TextInput,
  cx,
} from "../../../design-system";
import { call, inTauri, openUrl } from "../../../ipc/client";
import { reportError, useCommand, useOp } from "../../../ipc/query";
import { useNav } from "../../../app/stores";
import { toast } from "../../../app/toast";
import { IMAGE_EXTENSIONS, MOODBOARD_TABLES, assetSrc, blobToBase64, looksLikeUrl, pickImages, sceneLabel, useVisualScenes } from "../../../api/visual";
import { useVisualView } from "../visual/shared";
import { canvasExtent, changedMoves, dragBoxes, resizeBox, type Box } from "../visual/canvas";
import type { MoodboardSummary } from "../../../ipc/generated/MoodboardSummary";
import type { MoodboardDto } from "../../../ipc/generated/MoodboardDto";
import type { MoodboardItemDto } from "../../../ipc/generated/MoodboardItemDto";
import type { VaultImageRef } from "../../../ipc/generated/VaultImageRef";
import type { VisualScenes } from "../../../ipc/generated/VisualScenes";
import type { StoreSel } from "../../../ipc/generated/StoreSel";
import { MoodboardExportButton } from "../../../features/export/buttons";

export const tab = { id: "moodboards", label: "Moodboards", order: 40 };

export default function MoodboardsTab() {
  const route = useNav((s) => s.route);
  const view = useVisualView();
  const boardsQ = useOp<MoodboardSummary[]>("moodboard.list", {}, MOODBOARD_TABLES);
  const boards = useMemo(() => boardsQ.data ?? [], [boardsQ.data]);
  const scenesQ = useVisualScenes();
  const [newOpen, setNewOpen] = useState(false);

  // Deep link from search: { moodboardId, itemId }.
  const linkBoard = route.params?.moodboardId;
  const linkItem = route.params?.itemId;
  const [focusItem, setFocusItem] = useState<string | null>(null);
  const consumed = useRef<string | null>(null);
  useEffect(() => {
    const key = `${linkBoard ?? ""}|${linkItem ?? ""}`;
    if (!linkBoard || consumed.current === key) return;
    consumed.current = key;
    view.set({ moodboardId: linkBoard });
    if (linkItem) setFocusItem(linkItem);
  }, [linkBoard, linkItem, view]);

  const current = boards.find((b) => b.id === view.moodboardId) ?? boards[0];

  if (boardsQ.isLoading) {
    return (
      <div>
        <Header onNew={() => setNewOpen(true)} />
        <Skeleton h={300} />
      </div>
    );
  }

  return (
    <div style={{ position: "relative", minHeight: "100%" }}>
      <Header onNew={() => setNewOpen(true)} current={current} />
      {boards.length === 0 ? (
        <EmptyState
          title="No moodboards yet"
          actions={
            <Button variant="primary" icon={<Plus size={15} />} onClick={() => setNewOpen(true)}>
              New Moodboard
            </Button>
          }
        >
          Collect images, notes and links that show how the film should look and feel. Name a board anything — for example Overall
          Look, Cinematography or Costume.
        </EmptyState>
      ) : (
        <div className="row" style={{ alignItems: "flex-start", gap: 12, height: "calc(100vh - 230px)", minHeight: 380 }}>
          <BoardList boards={boards} currentId={current?.id ?? null} onPick={(id) => view.set({ moodboardId: id })} />
          <div className="grow" style={{ height: "100%", minWidth: 0 }}>
            {current && <Board key={current.id} boardId={current.id} focusItem={focusItem} onFocused={() => setFocusItem(null)} />}
          </div>
        </div>
      )}
      <NewMoodboardDialog
        open={newOpen}
        onOpenChange={setNewOpen}
        scenes={scenesQ.data}
        onCreated={(b) => view.set({ moodboardId: b.id })}
      />
    </div>
  );
}

function Header({ onNew, current }: { onNew: () => void; current?: MoodboardSummary }) {
  return (
    <PageHeader
      title="Moodboards"
      sub="What should this film feel like?"
      actions={
        <>
          {current && <MoodboardExportButton board={current} />}
          <Button variant="primary" icon={<Plus size={15} />} onClick={onNew}>
            New Moodboard
          </Button>
        </>
      }
    />
  );
}

// ------------------------------------------------------------------ board list

function BoardList({ boards, currentId, onPick }: { boards: MoodboardSummary[]; currentId: string | null; onPick: (id: string) => void }) {
  const [renameFor, setRenameFor] = useState<MoodboardSummary | null>(null);
  const [notesFor, setNotesFor] = useState<MoodboardSummary | null>(null);
  const [deleteFor, setDeleteFor] = useState<MoodboardSummary | null>(null);
  const reorder = useCommand<{ id: string; index: number }>("moodboard.reorder", { onSuccess: () => toast.undoable("Reordered moodboards") });
  const del = useCommand<{ id: string }>("moodboard.delete", {
    onSuccess: () => {
      toast.undoable(`Deleted moodboard “${deleteFor?.name ?? ""}”`);
      setDeleteFor(null);
    },
  });
  return (
    <nav className="card" style={{ width: 190, flex: "none", padding: 0, maxHeight: "100%", overflow: "auto" }} aria-label="Moodboards">
      {boards.map((b, i) => (
        <ContextMenu
          key={b.id}
          items={[
            { label: "Rename…", onSelect: () => setRenameFor(b) },
            { label: "Board notes…", onSelect: () => setNotesFor(b) },
            { label: "Move up", disabled: i === 0, onSelect: () => reorder.mutate({ id: b.id, index: i - 1 }) },
            { label: "Move down", disabled: i === boards.length - 1, onSelect: () => reorder.mutate({ id: b.id, index: i + 1 }) },
            { label: "Delete", danger: true, separatorBefore: true, onSelect: () => setDeleteFor(b) },
          ]}
        >
          <button
            type="button"
            className={cx("li", b.id === currentId && "sel")}
            aria-current={b.id === currentId ? "true" : undefined}
            onClick={() => onPick(b.id)}
            style={{ width: "100%", border: 0, borderBottom: "1px solid var(--line)", textAlign: "left", background: b.id === currentId ? undefined : "transparent" }}
          >
            <span className="grow" style={{ minWidth: 0 }}>
              {b.id === currentId ? <b>{b.name}</b> : b.name}
              {b.sceneNumber && <div className="xs muted">Scene {b.sceneNumber} reference</div>}
            </span>
            <span className="xs muted">{b.itemCount}</span>
          </button>
        </ContextMenu>
      ))}
      {renameFor && <RenameBoardDialog board={renameFor} onClose={() => setRenameFor(null)} />}
      {notesFor && <BoardNotesDialog board={notesFor} onClose={() => setNotesFor(null)} />}
      <ConfirmDialog
        open={!!deleteFor}
        onOpenChange={(v) => !v && setDeleteFor(null)}
        title={`Delete moodboard “${deleteFor?.name ?? ""}”?`}
        confirmLabel="Delete Moodboard"
        danger
        busy={del.isPending}
        onConfirm={() => deleteFor && del.mutate({ id: deleteFor.id })}
      >
        The board and its {deleteFor?.itemCount ?? 0} item{deleteFor?.itemCount === 1 ? "" : "s"} go to Recently Deleted and can be restored.
      </ConfirmDialog>
    </nav>
  );
}

// ------------------------------------------------------------------ canvas

type Gesture =
  | { kind: "drag"; startX: number; startY: number; before: Box[]; moved: boolean }
  | { kind: "resize"; id: string; startX: number; startY: number; box: Box; keepAspect: boolean };

function Board({ boardId, focusItem, onFocused }: { boardId: string; focusItem: string | null; onFocused: () => void }) {
  const q = useOp<MoodboardDto>("moodboard.get", { id: boardId }, MOODBOARD_TABLES);
  const items = useMemo(() => q.data?.items ?? [], [q.data]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [preview, setPreview] = useState<Box[] | null>(null);
  const gesture = useRef<Gesture | null>(null);
  const wrapRef = useRef<HTMLDivElement | null>(null);
  const [inspect, setInspect] = useState<string | null>(null);
  const [noteOpen, setNoteOpen] = useState(false);
  const [linkOpen, setLinkOpen] = useState(false);
  const [vaultOpen, setVaultOpen] = useState(false);
  const [dropHover, setDropHover] = useState(false);

  const boxes: Box[] = useMemo(() => items.map((i) => ({ id: i.id, x: i.x, y: i.y, w: i.w, h: i.h })), [items]);
  const shown = preview ?? boxes;
  const boxOf = (id: string) => shown.find((b) => b.id === id);
  const itemsKey = items.map((i) => `${i.id}:${i.rev}`).join(",");
  useEffect(() => setPreview(null), [itemsKey]);
  useEffect(() => {
    setSelected((s) => new Set([...s].filter((id) => items.some((i) => i.id === id))));
  }, [items]);
  useEffect(() => {
    if (focusItem && items.some((i) => i.id === focusItem)) {
      setSelected(new Set([focusItem]));
      document.getElementById(`mb-tile-${focusItem}`)?.scrollIntoView({ block: "center", inline: "center" });
      onFocused();
    }
  }, [focusItem, items, onFocused]);

  const move = useCommand<{ moves: { id: string; x: number; y: number }[] }>("moodboard.move_items", { onError: () => setPreview(null) });
  const resize = useCommand<{ id: string; w: number; h: number }>("moodboard.resize_item", { onError: () => setPreview(null) });
  const arrange = useCommand<{ id: string; placement: "front" | "back" }>("moodboard.arrange_item");
  const del = useCommand<{ ids: string[] }>("moodboard.delete_items", {
    onSuccess: (_, a) => {
      toast.undoable(a.ids.length === 1 ? "Deleted moodboard item" : `Deleted ${a.ids.length} moodboard items`);
      setSelected(new Set());
      setInspect(null);
    },
  });

  // ---- adding
  const dropPoint = (): { x: number; y: number } => {
    const el = wrapRef.current;
    return el ? { x: Math.round(el.scrollLeft + 24), y: Math.round(el.scrollTop + 24) } : { x: 24, y: 24 };
  };
  const addImagesFromPaths = async (paths: string[], at?: { x: number; y: number }) => {
    if (paths.length === 0) return;
    try {
      await call("moodboard.add_images", { moodboardId: boardId, paths, ...(at ?? {}) });
      toast.undoable(paths.length === 1 ? "Added image" : `Added ${paths.length} images`);
    } catch (e) {
      reportError(e);
    }
  };
  const addFromFile = async () => {
    try {
      await addImagesFromPaths(await pickImages(true));
    } catch (e) {
      reportError(e);
    }
  };
  const onPaste = async (e: ClipboardEvent<HTMLDivElement>) => {
    if ((e.target as HTMLElement).closest("input, textarea")) return;
    const files = [...e.clipboardData.files].filter((f) => f.type.startsWith("image/"));
    const text = e.clipboardData.getData("text/plain");
    e.preventDefault();
    try {
      if (files.length > 0) {
        for (const f of files) {
          await call("moodboard.add_image_data", { moodboardId: boardId, dataBase64: await blobToBase64(f), fileName: f.name || "pasted-image", ...dropPoint() });
        }
        toast.undoable("Pasted image");
      } else if (text.trim()) {
        if (looksLikeUrl(text)) await call("moodboard.add_link", { moodboardId: boardId, url: text.trim(), ...dropPoint() });
        else await call("moodboard.add_note", { moodboardId: boardId, text: text.trim(), ...dropPoint() });
        toast.undoable(looksLikeUrl(text) ? "Pasted link" : "Pasted note");
      }
    } catch (err) {
      reportError(err);
    }
  };

  // Files dropped from Explorer arrive through the Tauri webview (paths, not bytes).
  useEffect(() => {
    if (!inTauri()) return;
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    void import("@tauri-apps/api/webview").then(({ getCurrentWebview }) =>
      getCurrentWebview()
        .onDragDropEvent((ev) => {
          const el = wrapRef.current;
          if (!el) return;
          const p = ev.payload;
          if (p.type === "leave") {
            setDropHover(false);
            return;
          }
          const r = el.getBoundingClientRect();
          const scale = window.devicePixelRatio || 1;
          const x = p.position.x / scale;
          const y = p.position.y / scale;
          const inside = x >= r.left && x <= r.right && y >= r.top && y <= r.bottom;
          if (p.type === "drop") {
            setDropHover(false);
            if (!inside) return;
            const images = p.paths.filter((f) => IMAGE_EXTENSIONS.includes(f.split(".").pop()?.toLowerCase() ?? ""));
            if (images.length < p.paths.length) toast.info("Only images can be dropped on a moodboard. Other files were skipped.");
            void addImagesFromPaths(images, { x: Math.round(x - r.left + el.scrollLeft), y: Math.round(y - r.top + el.scrollTop) });
          } else {
            setDropHover(inside);
          }
        })
        .then((u) => {
          if (cancelled) u();
          else unlisten = u;
        }),
    );
    return () => {
      cancelled = true;
      unlisten?.();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- re-subscribe per board only
  }, [boardId]);

  // ---- gestures
  const onTilePointerDown = (e: RPointerEvent<HTMLDivElement>, id: string) => {
    if (e.button !== 0) return;
    e.stopPropagation();
    const additive = e.shiftKey || e.ctrlKey || e.metaKey;
    let sel = selected;
    if (additive) {
      sel = new Set(selected);
      if (sel.has(id)) sel.delete(id);
      else sel.add(id);
    } else if (!selected.has(id)) {
      sel = new Set([id]);
    }
    setSelected(sel);
    if (!sel.has(id)) return;
    wrapRef.current?.setPointerCapture(e.pointerId);
    gesture.current = { kind: "drag", startX: e.clientX, startY: e.clientY, before: boxes, moved: false };
  };
  const onResizePointerDown = (e: RPointerEvent<HTMLDivElement>, item: MoodboardItemDto) => {
    e.stopPropagation();
    const b = boxOf(item.id);
    if (!b) return;
    wrapRef.current?.setPointerCapture(e.pointerId);
    gesture.current = { kind: "resize", id: item.id, startX: e.clientX, startY: e.clientY, box: b, keepAspect: item.kind === "image" && !e.shiftKey };
  };
  const onPointerMove = (e: RPointerEvent<HTMLDivElement>) => {
    const g = gesture.current;
    if (!g) return;
    const dx = e.clientX - g.startX;
    const dy = e.clientY - g.startY;
    if (g.kind === "drag") {
      if (!g.moved && Math.abs(dx) + Math.abs(dy) < 3) return;
      g.moved = true;
      setPreview(dragBoxes(g.before, selected, dx, dy));
    } else {
      const nb = resizeBox(g.box, dx, dy, g.keepAspect);
      setPreview(boxes.map((b) => (b.id === g.id ? nb : b)));
    }
  };
  const onPointerUp = (e: RPointerEvent<HTMLDivElement>) => {
    const g = gesture.current;
    gesture.current = null;
    if (wrapRef.current?.hasPointerCapture(e.pointerId)) wrapRef.current.releasePointerCapture(e.pointerId);
    if (!g || !preview) return;
    if (g.kind === "drag") {
      const moves = changedMoves(g.before, preview);
      if (moves.length > 0) move.mutate({ moves });
      else setPreview(null);
    } else {
      const nb = preview.find((b) => b.id === g.id);
      if (nb && (nb.w !== g.box.w || nb.h !== g.box.h)) resize.mutate({ id: g.id, w: nb.w, h: nb.h });
      else setPreview(null);
    }
  };
  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if ((e.target as HTMLElement).closest("input, textarea") || selected.size === 0) return;
    const step = e.shiftKey ? 32 : 8;
    const dirs: Record<string, [number, number]> = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step] };
    if (dirs[e.key]) {
      e.preventDefault();
      const [dx, dy] = dirs[e.key];
      const moves = changedMoves(boxes, dragBoxes(boxes, selected, dx, dy));
      if (moves.length) move.mutate({ moves });
    } else if (e.key === "Delete" || e.key === "Backspace") {
      e.preventDefault();
      del.mutate({ ids: [...selected] });
    } else if (e.key === "Escape") {
      setSelected(new Set());
    } else if (e.key === "Enter" && selected.size === 1) {
      e.preventDefault();
      setInspect([...selected][0]);
    }
  };

  if (q.isLoading || !q.data) return <Skeleton h={300} />;
  const extent = canvasExtent(shown);
  const single = selected.size === 1 ? items.find((i) => selected.has(i.id)) : undefined;

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", gap: 8 }}>
      <div className="toolbar" style={{ marginBottom: 0, minHeight: 32 }}>
        <Menu
          items={[
            { label: "From file…", icon: <ImagePlus size={14} />, onSelect: () => void addFromFile() },
            { label: "From Idea Vault…", icon: <ImagePlus size={14} />, onSelect: () => setVaultOpen(true) },
          ]}
          trigger={
            <Button size="sm" icon={<ImagePlus size={14} />}>
              Add image
            </Button>
          }
        />
        <Button size="sm" icon={<StickyNote size={14} />} onClick={() => setNoteOpen(true)}>
          Add note
        </Button>
        <Button size="sm" icon={<Link2 size={14} />} onClick={() => setLinkOpen(true)}>
          Add link
        </Button>
        <span className="sp" />
        {single ? (
          <SelectionBar item={single} onEdit={() => setInspect(single.id)} onArrange={(p) => arrange.mutate({ id: single.id, placement: p })} onDelete={() => del.mutate({ ids: [single.id] })} />
        ) : selected.size > 1 ? (
          <>
            <span className="sm muted">{selected.size} items selected</span>
            <Button size="sm" variant="ghost" icon={<Trash2 size={14} />} onClick={() => del.mutate({ ids: [...selected] })}>
              Delete
            </Button>
          </>
        ) : (
          <span className="hint">Drag to move · Shift-click to select several · paste images or links with Ctrl+V</span>
        )}
      </div>
      <div
        ref={wrapRef}
        className="canvas"
        role="application"
        aria-label={`Moodboard canvas — ${q.data.board.name}`}
        tabIndex={0}
        style={{ flex: 1, overflow: "auto", outline: dropHover ? "2px dashed var(--accent)" : undefined }}
        onPointerDown={(e) => {
          if (e.target === e.currentTarget || (e.target as HTMLElement).dataset.canvasBg) setSelected(new Set());
        }}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={onPointerUp}
        onKeyDown={onKeyDown}
        onPaste={(e) => void onPaste(e)}
      >
        <div data-canvas-bg="1" style={{ position: "relative", width: extent.width, height: extent.height, minWidth: "100%", minHeight: "100%" }}>
          {items.length === 0 && (
            <div data-canvas-bg="1" className="empty" style={{ position: "absolute", inset: 0 }}>
              <p data-canvas-bg="1">Drop images, notes and references here.</p>
            </div>
          )}
          {items.map((it) => {
            const b = boxOf(it.id) ?? it;
            return (
              <Tile
                key={it.id}
                item={it}
                box={b}
                selected={selected.has(it.id)}
                onPointerDown={(e) => onTilePointerDown(e, it.id)}
                onResizePointerDown={(e) => onResizePointerDown(e, it)}
                onOpen={() => setInspect(it.id)}
                onFocusSelect={() => !selected.has(it.id) && !gesture.current && setSelected(new Set([it.id]))}
                menu={[
                  { label: "Edit…", onSelect: () => setInspect(it.id) },
                  { label: "Bring to front", onSelect: () => arrange.mutate({ id: it.id, placement: "front" }) },
                  { label: "Send to back", onSelect: () => arrange.mutate({ id: it.id, placement: "back" }) },
                  ...(it.kind === "link" && it.url ? [{ label: "Open link", onSelect: () => void openUrl(it.url as string).catch(reportError) }] : []),
                  { label: selected.size > 1 && selected.has(it.id) ? `Delete ${selected.size} items` : "Delete", danger: true, separatorBefore: true, onSelect: () => del.mutate({ ids: selected.size > 1 && selected.has(it.id) ? [...selected] : [it.id] }) },
                ]}
              />
            );
          })}
        </div>
      </div>
      {inspect && items.some((i) => i.id === inspect) && (
        <ItemDrawer key={inspect} item={items.find((i) => i.id === inspect) as MoodboardItemDto} onClose={() => setInspect(null)} />
      )}
      {noteOpen && <NoteDialog boardId={boardId} at={dropPoint()} onClose={() => setNoteOpen(false)} />}
      {linkOpen && <LinkDialog boardId={boardId} at={dropPoint()} onClose={() => setLinkOpen(false)} />}
      {vaultOpen && <VaultDialog boardId={boardId} at={dropPoint()} onClose={() => setVaultOpen(false)} />}
    </div>
  );
}

function SelectionBar({ item, onEdit, onArrange, onDelete }: { item: MoodboardItemDto; onEdit: () => void; onArrange: (p: "front" | "back") => void; onDelete: () => void }) {
  const [caption, setCaption] = useState(item.caption ?? "");
  useEffect(() => setCaption(item.caption ?? ""), [item.caption, item.id]);
  const update = useCommand<{ id: string; caption: string }>("moodboard.update_item");
  const commit = () => caption !== (item.caption ?? "") && update.mutate({ id: item.id, caption });
  return (
    <div className="row gap4" style={{ minWidth: 0 }}>
      <TextInput
        aria-label="Caption"
        placeholder="Caption — why this matters"
        value={caption}
        maxLength={300}
        style={{ width: 240 }}
        onChange={(e) => setCaption(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => e.key === "Enter" && commit()}
      />
      <IconButton label="Bring to front" onClick={() => onArrange("front")}>
        <ArrowUpToLine size={15} />
      </IconButton>
      <IconButton label="Send to back" onClick={() => onArrange("back")}>
        <ArrowDownToLine size={15} />
      </IconButton>
      <IconButton label="Edit item" onClick={onEdit}>
        <MoreHorizontal size={15} />
      </IconButton>
      <IconButton label="Delete item" onClick={onDelete}>
        <Trash2 size={15} />
      </IconButton>
    </div>
  );
}

function Tile({
  item,
  box,
  selected,
  onPointerDown,
  onResizePointerDown,
  onOpen,
  onFocusSelect,
  menu,
}: {
  item: MoodboardItemDto;
  box: Box;
  selected: boolean;
  onFocusSelect: () => void;
  onPointerDown: (e: RPointerEvent<HTMLDivElement>) => void;
  onResizePointerDown: (e: RPointerEvent<HTMLDivElement>) => void;
  onOpen: () => void;
  menu: Parameters<typeof ContextMenu>[0]["items"];
}) {
  const src = assetSrc(item.asset);
  const label =
    item.kind === "image"
      ? `Image${item.caption ? `: ${item.caption}` : ""}`
      : item.kind === "note"
        ? `${item.isPrivate ? "Internal note" : "Note"}: ${item.body ?? ""}`
        : `Link: ${item.linkTitle ?? item.url ?? ""}`;
  return (
    <ContextMenu items={menu}>
      <div
        id={`mb-tile-${item.id}`}
        className={cx("tile", selected && "sel")}
        role="button"
        tabIndex={0}
        aria-label={label}
        aria-pressed={selected}
        onFocus={onFocusSelect}
        onPointerDown={onPointerDown}
        onDoubleClick={() => (item.kind === "link" && item.url ? void openUrl(item.url).catch(reportError) : onOpen())}
        style={{
          left: box.x,
          top: box.y,
          width: box.w,
          zIndex: item.z,
          cursor: "move",
          userSelect: "none",
          touchAction: "none",
          ...(item.kind === "note"
            ? { padding: 9, minHeight: box.h, background: item.isPrivate ? "#eef1f6" : "#fff8dc", fontSize: 12, whiteSpace: "pre-wrap" }
            : item.kind === "link"
              ? { padding: 8, minHeight: box.h, fontSize: 12 }
              : {}),
        }}
      >
        {item.kind === "image" &&
          (src ? (
            <img src={src} alt={item.caption ?? item.asset?.originalName ?? "Moodboard image"} draggable={false} style={{ display: "block", width: "100%", height: box.h, objectFit: "cover" }} />
          ) : (
            <div className="photo sketch" style={{ height: box.h, display: "grid", placeItems: "center", color: "var(--muted)", fontSize: 11, textAlign: "center", padding: 6 }}>
              Image unavailable{item.asset ? ` — ${item.asset.originalName}` : ""}
            </div>
          ))}
        {item.kind === "note" && (
          <>
            {item.isPrivate && (
              <div className="xs muted row gap4" style={{ marginBottom: 4 }}>
                <Lock size={11} /> Internal — left out of exports
              </div>
            )}
            {item.body}
          </>
        )}
        {item.kind === "link" && (
          <>
            <div className="row gap4">
              <Link2 size={13} />
              <b style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{item.linkTitle || item.url}</b>
            </div>
            <div className="xs muted">link · {item.urlHost}</div>
          </>
        )}
        {item.caption && item.kind !== "note" && <div className="cap">{item.caption}</div>}
        {item.caption && item.kind === "note" && <div className="xs muted" style={{ marginTop: 4 }}>{item.caption}</div>}
        {selected && (
          <div
            aria-hidden
            onPointerDown={onResizePointerDown}
            style={{ position: "absolute", right: -1, bottom: -1, width: 12, height: 12, background: "var(--blue)", borderRadius: 2, cursor: "nwse-resize" }}
          />
        )}
      </div>
    </ContextMenu>
  );
}

// ------------------------------------------------------------------ item editing

function ItemDrawer({ item, onClose }: { item: MoodboardItemDto; onClose: () => void }) {
  const update = useCommand<{ id: string; caption?: string; body?: string; url?: string; linkTitle?: string; isPrivate?: boolean }>("moodboard.update_item");
  const [caption, setCaption] = useState(item.caption ?? "");
  const [body, setBody] = useState(item.body ?? "");
  const [url, setUrl] = useState(item.url ?? "");
  const [title, setTitle] = useState(item.linkTitle ?? "");
  const [bodyError, setBodyError] = useState<string | null>(null);
  const typeLabel = item.kind === "image" ? "Image" : item.kind === "note" ? "Note" : "Link";
  return (
    <Drawer open onClose={onClose} typeLabel="Moodboard item" title={typeLabel} width="n">
      {item.kind === "image" && item.asset && <div className="hint" style={{ marginBottom: 8 }}>{item.asset.originalName}</div>}
      {item.kind === "note" && (
        <>
          <Field label="Note" required htmlFor="mb-note" error={bodyError}>
            <TextArea
              id="mb-note"
              rows={5}
              value={body}
              invalid={!!bodyError}
              onChange={(e) => setBody(e.target.value)}
              onBlur={() => {
                if (body === (item.body ?? "")) return;
                if (!body.trim()) {
                  setBodyError("A note needs some text.");
                  return;
                }
                setBodyError(null);
                update.mutate({ id: item.id, body });
              }}
            />
          </Field>
          <Checkbox
            checked={item.isPrivate}
            onChange={(v) => update.mutate({ id: item.id, isPrivate: v })}
            label="Internal note — left out of exports unless selected"
          />
        </>
      )}
      {item.kind === "link" && (
        <>
          <Field label="Web address" required htmlFor="mb-url">
            <TextInput id="mb-url" value={url} onChange={(e) => setUrl(e.target.value)} onBlur={() => url !== (item.url ?? "") && update.mutate({ id: item.id, url })} />
          </Field>
          <Field label="Title" htmlFor="mb-title">
            <TextInput id="mb-title" value={title} onChange={(e) => setTitle(e.target.value)} onBlur={() => title !== (item.linkTitle ?? "") && update.mutate({ id: item.id, linkTitle: title })} />
          </Field>
          {item.url && (
            <Button size="sm" icon={<ExternalLink size={13} />} onClick={() => void openUrl(item.url as string).catch(reportError)}>
              Open link
            </Button>
          )}
        </>
      )}
      <Field label="Caption" htmlFor="mb-caption" hint="Optional — a short note on why this matters.">
        <TextInput id="mb-caption" maxLength={300} value={caption} onChange={(e) => setCaption(e.target.value)} onBlur={() => caption !== (item.caption ?? "") && update.mutate({ id: item.id, caption })} />
      </Field>
      {item.sourceVaultItemId && <div className="hint">Copied from the Idea Vault. Changes here don't affect the vault.</div>}
    </Drawer>
  );
}

function NoteDialog({ boardId, at, onClose }: { boardId: string; at: { x: number; y: number }; onClose: () => void }) {
  const [text, setText] = useState("");
  const [priv, setPriv] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const add = useCommand<{ moodboardId: string; text: string; isPrivate: boolean; x: number; y: number }>("moodboard.add_note", {
    onSuccess: () => {
      toast.undoable("Added note");
      onClose();
    },
  });
  const submit = () => {
    if (!text.trim()) {
      setError("Write the note first.");
      return;
    }
    add.mutate({ moodboardId: boardId, text, isPrivate: priv, ...at });
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Add note"
      size="md"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={add.isPending} onClick={submit}>
            Add Note
          </Button>
        </>
      }
    >
      <Field label="Note" required htmlFor="new-note" error={error}>
        <TextArea id="new-note" rows={4} autoFocus value={text} invalid={!!error} placeholder="e.g. Cold, wet, blue-grey. Warm light only from windows." onChange={(e) => { setText(e.target.value); setError(null); }} />
      </Field>
      <Checkbox checked={priv} onChange={setPriv} label="Internal note — left out of exports unless selected" />
    </Dialog>
  );
}

function LinkDialog({ boardId, at, onClose }: { boardId: string; at: { x: number; y: number }; onClose: () => void }) {
  const [url, setUrl] = useState("");
  const [title, setTitle] = useState("");
  const add = useCommand<{ moodboardId: string; url: string; title?: string; x: number; y: number }>("moodboard.add_link", {
    onSuccess: () => {
      toast.undoable("Added link");
      onClose();
    },
  });
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Add link"
      size="md"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!url.trim() || add.isPending} onClick={() => add.mutate({ moodboardId: boardId, url, title: title || undefined, ...at })}>
            Add Link
          </Button>
        </>
      }
    >
      <Field label="Web address" required htmlFor="new-link">
        <TextInput id="new-link" autoFocus value={url} placeholder="https://…" onChange={(e) => setUrl(e.target.value)} />
      </Field>
      <Field label="Title" htmlFor="new-link-title" hint="Optional — e.g. Reference: Memories of Murder">
        <TextInput id="new-link-title" value={title} onChange={(e) => setTitle(e.target.value)} />
      </Field>
    </Dialog>
  );
}

function VaultDialog({ boardId, at, onClose }: { boardId: string; at: { x: number; y: number }; onClose: () => void }) {
  const [store, setStore] = useState<StoreSel>("project");
  const q = useOp<VaultImageRef[]>("visual.vault_images", { store }, ["vault_item", "asset"], { store: store === "global" ? "global" : "project" });
  const add = useCommand<{ moodboardId: string; vaultItemId: string; store: StoreSel; x: number; y: number }>("moodboard.add_vault_image", {
    onSuccess: () => {
      toast.undoable("Added Idea Vault image");
      onClose();
    },
  });
  return (
    <Dialog open onOpenChange={(v) => !v && onClose()} title="Add image from the Idea Vault" size="lg" footer={<Button onClick={onClose}>Cancel</Button>}>
      <div className="row" style={{ marginBottom: 10 }}>
        <Segmented
          ariaLabel="Idea Vault"
          value={store}
          onChange={setStore}
          options={[
            { value: "project", label: "Project Idea Vault" },
            { value: "global", label: "Global Idea Vault" },
          ]}
        />
        <span className="hint">The image is copied onto the board; the vault item is not changed.</span>
      </div>
      {q.isLoading ? (
        <Skeleton h={120} />
      ) : (q.data ?? []).length === 0 ? (
        <div className="muted" style={{ padding: 20, textAlign: "center" }}>
          There are no images in this Idea Vault yet.
        </div>
      ) : (
        <div className="grid g4">
          {(q.data ?? []).map((v) => {
            const src = assetSrc(v.asset);
            return (
              <button
                key={v.vaultItemId}
                type="button"
                className="pnl"
                disabled={add.isPending}
                style={{ padding: 0, textAlign: "left" }}
                onClick={() => add.mutate({ moodboardId: boardId, vaultItemId: v.vaultItemId, store, ...at })}
              >
                <div className={cx("pv", !src && "photo sketch")} style={{ height: 90 }}>
                  {src && <img src={src} alt="" style={{ width: "100%", height: "100%", objectFit: "cover" }} />}
                </div>
                <div className="pt truncate2">{v.title}</div>
              </button>
            );
          })}
        </div>
      )}
    </Dialog>
  );
}

// ------------------------------------------------------------------ boards

function NewMoodboardDialog({
  open,
  onOpenChange,
  scenes,
  onCreated,
}: {
  open: boolean;
  onOpenChange: (v: boolean) => void;
  scenes: VisualScenes | undefined;
  onCreated: (b: MoodboardSummary) => void;
}) {
  const [name, setName] = useState("");
  const [sceneId, setSceneId] = useState("");
  const [error, setError] = useState<string | null>(null);
  const sugg = useOp<string[]>("moodboard.suggestions", {}, ["moodboard"], { enabled: open });
  useEffect(() => {
    if (open) {
      setName("");
      setSceneId("");
      setError(null);
    }
  }, [open]);
  const create = useCommand<{ name: string; sceneId?: string }, MoodboardSummary>("moodboard.create", {
    onSuccess: (b) => {
      toast.undoable(`Created moodboard “${b.name}”`);
      onOpenChange(false);
      onCreated(b);
    },
  });
  const submit = () => {
    if (!name.trim()) {
      setError("Give the board a name.");
      return;
    }
    create.mutate({ name, sceneId: sceneId || undefined });
  };
  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title="New Moodboard"
      size="md"
      footer={
        <>
          <Button onClick={() => onOpenChange(false)}>Cancel</Button>
          <Button variant="primary" disabled={create.isPending} onClick={submit}>
            Create Moodboard
          </Button>
        </>
      }
    >
      <Field label="Board name" required htmlFor="mb-name" error={error}>
        <TextInput
          id="mb-name"
          autoFocus
          value={name}
          invalid={!!error}
          placeholder="e.g. Overall Look"
          onChange={(e) => {
            setName(e.target.value);
            setError(null);
          }}
          onKeyDown={(e) => e.key === "Enter" && submit()}
        />
      </Field>
      {(sugg.data ?? []).length > 0 && (
        <div style={{ marginBottom: 10 }}>
          <div className="hint" style={{ marginBottom: 4 }}>
            Suggestions — use any name you like
          </div>
          <div className="row wrap gap4">
            {(sugg.data ?? []).map((s) => (
              <button key={s} type="button" className={cx("chip", name === s ? "a" : "out")} style={{ cursor: "pointer" }} onClick={() => setName(s)}>
                {s}
              </button>
            ))}
          </div>
        </div>
      )}
      {scenes && scenes.scenes.length > 0 && (
        <Field label="Reference for a scene" htmlFor="mb-scene" hint="Optional.">
          <select id="mb-scene" className="select" value={sceneId} onChange={(e) => setSceneId(e.target.value)}>
            <option value="">No particular scene</option>
            {scenes.scenes.map((s) => (
              <option key={s.lineageId} value={s.sceneId}>
                {sceneLabel(s)}
              </option>
            ))}
          </select>
        </Field>
      )}
    </Dialog>
  );
}

function RenameBoardDialog({ board, onClose }: { board: MoodboardSummary; onClose: () => void }) {
  const [name, setName] = useState(board.name);
  const rename = useCommand<{ id: string; name: string; expectedRev?: number }>("moodboard.rename", {
    onSuccess: () => {
      toast.undoable("Renamed moodboard");
      onClose();
    },
  });
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Rename moodboard"
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
      <Field label="Board name" required htmlFor="mb-rename">
        <TextInput id="mb-rename" autoFocus value={name} onChange={(e) => setName(e.target.value)} />
      </Field>
    </Dialog>
  );
}

function BoardNotesDialog({ board, onClose }: { board: MoodboardSummary; onClose: () => void }) {
  const q = useOp<MoodboardDto>("moodboard.get", { id: board.id }, MOODBOARD_TABLES);
  const [notes, setNotes] = useState<string | null>(null);
  const value = notes ?? q.data?.notes ?? "";
  const save = useCommand<{ id: string; notes: string | null }>("moodboard.update_notes", {
    onSuccess: () => {
      toast.undoable("Saved board notes");
      onClose();
    },
  });
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title={`Board notes — ${board.name}`}
      size="md"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={save.isPending || notes === null} onClick={() => save.mutate({ id: board.id, notes: value })}>
            Save Notes
          </Button>
        </>
      }
    >
      <Field label="Internal notes" htmlFor="mb-notes" hint="For the team only — left out of standard exports.">
        <TextArea id="mb-notes" rows={6} value={value} onChange={(e) => setNotes(e.target.value)} />
      </Field>
      <Chip tone="p">
        <Lock size={11} /> Internal
      </Chip>
    </Dialog>
  );
}
