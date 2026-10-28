// Drag & drop for the Beat Board and Outline (FSD §11.6, §89.3–89.6, UX §3.6):
// @dnd-kit pointer dragging, a blue insertion marker for sibling insertion, a
// highlighted container for re-parenting, commit only on release over a valid
// target, and multi-select groups that keep their board order.

import { useCallback, useMemo, useRef, useState } from "react";
import {
  PointerSensor,
  pointerWithin,
  rectIntersection,
  useDraggable,
  useDroppable,
  useSensor,
  useSensors,
  type Active,
  type CollisionDetection,
  type DragEndEvent,
  type DragMoveEvent,
  type DragStartEvent,
  type Over,
} from "@dnd-kit/core";
import { story } from "../../api/story";
import { reportError } from "../../ipc/query";
import { toast } from "../../app/toast";
import type { StoryBoardState } from "../../ipc/generated/StoryBoardState";
import type { StoryContainerRef } from "../../ipc/generated/StoryContainerRef";
import { runMove } from "./actions";
import {
  canDrop,
  containerKey,
  dropBeside,
  dropInto,
  isNoop,
  itemLabel,
  locate,
  refKey,
  SEQ,
  type DropHint,
  type ItemKind,
  type Ref,
} from "./model";
import { keyToRef, useStoryUi } from "./ui";

export type DragData = { type: "item"; ref: Ref } | { type: "act"; actId: string };
export type DropData =
  | { type: "item"; ref: Ref }
  | { type: "container"; container: StoryContainerRef; atStart?: boolean }
  | { type: "act"; actId: string };

export interface Dragging {
  refs: Ref[];
  actId?: string;
  label: string;
}

const sameHint = (a: DropHint | null, b: DropHint | null) =>
  a === b ||
  (!!a &&
    !!b &&
    containerKey(a.target) === containerKey(b.target) &&
    (a.before ? refKey(a.before) : "") === (b.before ? refKey(b.before) : ""));

type Box = { left: number; top: number; width: number; height: number };

/** True when the dragged box's centre is in the first half of `over` along `axis` (left half for "x", top half for "y"). */
export function inFirstHalf(axis: "x" | "y", dragged: Box, over: Box): boolean {
  return axis === "x"
    ? dragged.left + dragged.width / 2 < over.left + over.width / 2
    : dragged.top + dragged.height / 2 < over.top + over.height / 2;
}

/**
 * `actAxis`: the direction acts are laid out in (the board's columns run along "x", its rows along "y").
 * `itemAxis`: the direction cards flow inside an act, which decides before or after a card.
 */
export function useStoryDnd(board: StoryBoardState | undefined, episodeId: string | null, actAxis: "x" | "y", itemAxis: "x" | "y" = "y") {
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 6 } }));
  const [dragging, setDragging] = useState<Dragging | null>(null);
  const [hint, setHint] = useState<DropHint | null>(null);
  const [actHint, setActHint] = useState<{ before: string | null } | null>(null);
  const moving = useRef<Set<string>>(new Set());
  const kinds = useRef<ItemKind[]>([]);
  const activeType = useRef<"item" | "act" | null>(null);

  const valid = useCallback(
    (d: DropData | undefined): boolean => {
      if (!d || !board) return false;
      if (activeType.current === "act") return d.type === "act";
      if (d.type === "act") return false;
      if (d.type === "container") return canDrop(kinds.current, d.container, moving.current);
      if (moving.current.has(refKey(d.ref))) return false;
      const loc = locate(board, d.ref);
      if (!loc) return false;
      if (kinds.current.includes("sequence")) return loc.container.parentType === "act";
      if (d.ref.kind === "sequence") return canDrop(kinds.current, SEQ(d.ref.id), moving.current);
      return canDrop(kinds.current, loc.container, moving.current);
    },
    [board],
  );

  const collision = useCallback<CollisionDetection>(
    (args) => {
      let hits = pointerWithin(args);
      if (hits.length === 0) hits = rectIntersection(args);
      const ok = hits.filter((h) => valid(args.droppableContainers.find((c) => c.id === h.id)?.data.current as DropData | undefined));
      const area = (id: string | number) => {
        const r = args.droppableRects.get(id);
        return r ? r.width * r.height : Number.MAX_SAFE_INTEGER;
      };
      ok.sort((a, b) => area(a.id) - area(b.id));
      return ok.slice(0, 1);
    },
    [valid],
  );

  const update = useCallback(
    (active: Active, over: Over | null) => {
      if (!board || !over) {
        setHint(null);
        setActHint(null);
        return;
      }
      const d = over.data.current as DropData | undefined;
      const r = active.rect.current.translated ?? active.rect.current.initial;
      if (!d || !r) return;
      if (d.type === "act") {
        const acts = board.acts.map((a) => a.id).filter((id) => id !== dragging?.actId);
        const firstHalf = inFirstHalf(actAxis, r, over.rect);
        const idx = acts.indexOf(d.actId);
        const before = firstHalf ? d.actId : (acts[idx + 1] ?? null);
        setActHint((prev) => (prev && prev.before === before ? prev : { before }));
        return;
      }
      let next: DropHint | null;
      if (d.type === "container") next = dropInto(board, d.container, !!d.atStart, moving.current);
      else if (d.ref.kind === "sequence" && !kinds.current.includes("sequence")) next = dropInto(board, SEQ(d.ref.id), true, moving.current);
      else next = dropBeside(board, d.ref, inFirstHalf(itemAxis, r, over.rect) ? "before" : "after", moving.current);
      setHint((prev) => (sameHint(prev, next) ? prev : next));
    },
    [board, actAxis, itemAxis, dragging?.actId],
  );

  const reset = () => {
    setDragging(null);
    setHint(null);
    setActHint(null);
    moving.current = new Set();
    kinds.current = [];
    activeType.current = null;
  };

  const onDragStart = (e: DragStartEvent) => {
    const data = e.active.data.current as DragData | undefined;
    if (!data || !board) return;
    if (data.type === "act") {
      activeType.current = "act";
      setDragging({ refs: [], actId: data.actId, label: board.acts.find((a) => a.id === data.actId)?.title ?? "Act" });
      return;
    }
    activeType.current = "item";
    const ui = useStoryUi.getState();
    const key = refKey(data.ref);
    let refs: Ref[] = [data.ref];
    if (data.ref.kind !== "sequence" && ui.selected.includes(key) && ui.selected.length > 1) {
      refs = ui.selected.map(keyToRef).filter((r) => r.kind !== "sequence" && !!locate(board, r));
    }
    moving.current = new Set(refs.map(refKey));
    kinds.current = [...new Set(refs.map((r) => r.kind))];
    const loc = locate(board, data.ref);
    const label =
      refs.length > 1
        ? `${refs.length} ${refs.every((r) => r.kind === "card") ? "cards" : "items"}`
        : loc
          ? itemLabel(loc.item)
          : "";
    setDragging({ refs, label });
  };

  const onDragMove = (e: DragMoveEvent) => update(e.active, e.over);

  const onDragEnd = (_e: DragEndEvent) => {
    const d = dragging;
    const h = hint;
    const ah = actHint;
    reset();
    if (!board || !d) return;
    if (d.actId) {
      if (!ah) return;
      const ids = board.acts.map((a) => a.id);
      const i = ids.indexOf(d.actId);
      if (ah.before === d.actId || (ah.before ?? null) === (ids[i + 1] ?? null)) return;
      void story
        .moveAct(d.actId, ah.before)
        .then(() => toast.undoable(`Moved Act “${d.label}”`))
        .catch(reportError);
      return;
    }
    // Invalid release: nothing happens, the card returns (no half-move state).
    if (!h || isNoop(board, d.refs, h)) return;
    void runMove(board, d.refs, h, episodeId);
  };

  return { sensors, collision, dragging, hint, actHint, onDragStart, onDragMove, onDragOver: onDragMove, onDragEnd, onDragCancel: reset };
}

export type StoryDnd = ReturnType<typeof useStoryDnd>;

/** A board item that can be dragged and dropped onto. */
export function useItemDnd(kind: ItemKind, id: string, disabled: boolean) {
  const key = `${kind}:${id}`;
  const data = useMemo(() => ({ type: "item", ref: { kind, id } }) as const, [kind, id]);
  const drag = useDraggable({ id: `drag:${key}`, data, disabled });
  const drop = useDroppable({ id: `over:${key}`, data });
  const { setNodeRef: setDrag } = drag;
  const { setNodeRef: setDrop } = drop;
  const setNodeRef = useCallback(
    (el: HTMLElement | null) => {
      setDrag(el);
      setDrop(el);
    },
    [setDrag, setDrop],
  );
  return { setNodeRef, listeners: drag.listeners, attributes: drag.attributes, isDragging: drag.isDragging };
}

/** A container body (act, sequence, parking lot, unassigned) that accepts drops. */
export function useContainerDrop(container: StoryContainerRef, atStart = false, disabled = false) {
  const { parentType, parentId } = container;
  const data = useMemo(
    () => ({ type: "container", container: { parentType, parentId }, atStart }) as const,
    [parentType, parentId, atStart],
  );
  return useDroppable({ id: `into:${containerKey(container)}${atStart ? ":start" : ""}`, data, disabled });
}

/** An act that can be dragged to reorder acts. */
export function useActDnd(actId: string, disabled: boolean) {
  const data = useMemo(() => ({ type: "act", actId }) as const, [actId]);
  const drag = useDraggable({ id: `drag:act:${actId}`, data, disabled });
  const drop = useDroppable({ id: `act:${actId}`, data });
  return { drag, drop };
}
