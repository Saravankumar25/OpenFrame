// Stripboard (UX §3.29, mocks 130/133/134): Unscheduled pool + shooting days.
// Drag and drop via @dnd-kit with an insertion position; every strip and day
// also has a menu, the non-drag alternative required by FSD §127.2.

import { useMemo, useState, type ReactNode } from "react";
import {
  DndContext,
  DragOverlay,
  KeyboardSensor,
  PointerSensor,
  closestCorners,
  useDroppable,
  useSensor,
  useSensors,
  type DragEndEvent,
  type DragOverEvent,
  type DragStartEvent,
} from "@dnd-kit/core";
import { SortableContext, sortableKeyboardCoordinates, useSortable, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { CalendarDays, MoreHorizontal, TriangleAlert } from "lucide-react";
import { Chip, IconButton, Menu, type MenuItemSpec, type MenuItems } from "../../../design-system";
import { call } from "../../../ipc/client";
import { reportError } from "../../../ipc/query";
import { toast } from "../../../app/toast";
import type { ScheduleMarkerDto, ScheduleStripDto, ScheduleView, ScheduleShootDayDto } from "../../../api/schedule";
import { runScheduleMove } from "../../../api/schedule";
import { MarkerRow, StripRow } from "./Strip";

/** Actions the board delegates to its host (dialogs, navigation). */
export interface ScheduleUi {
  openStrip: (id: string) => void;
  openDay: (id: string) => void;
  editDate: (day: ScheduleShootDayDto) => void;
  addMarker: (dayId: string, type: "Meal" | "Travel" | "Company Move" | "Custom") => void;
  editMarker: (dayId: string, marker: ScheduleMarkerDto) => void;
  createDayAfter: (dayId: string) => void;
  openCallSheet: (day: ScheduleShootDayDto) => void;
}

async function command(op: string, args: object, undoText: string) {
  try {
    await call(op, args);
    toast.undoable(undoText);
  } catch (e) {
    reportError(e);
  }
}

export function dayMenuItems(day: ScheduleShootDayDto, ui: ScheduleUi, opts: { detail?: boolean } = {}): MenuItemSpec[] {
  const hasScenes = day.summary.sceneCount > 0;
  const items: MenuItemSpec[] = [];
  if (!opts.detail) items.push({ label: "Open day details", onSelect: () => ui.openDay(day.id) });
  items.push({ label: "Edit date", onSelect: () => ui.editDate(day) });
  items.push({ label: "Add to this day", header: true, separatorBefore: true });
  items.push({ label: "Meal Break", onSelect: () => ui.addMarker(day.id, "Meal") });
  items.push({ label: "Travel", onSelect: () => ui.addMarker(day.id, "Travel") });
  items.push({ label: "Company Move", onSelect: () => ui.addMarker(day.id, "Company Move") });
  items.push({ label: "Custom Note", onSelect: () => ui.addMarker(day.id, "Custom") });
  items.push({
    label: day.isOffDay ? "Set as Shooting Day" : "Set as Off Day",
    separatorBefore: true,
    disabled: !day.isOffDay && hasScenes,
    onSelect: () => command("schedule.set_off_day", { dayId: day.id, offDay: !day.isOffDay }, day.isOffDay ? "Changed the off day into a shooting day" : `Set ${day.label} as an off day`),
  });
  if (!opts.detail) items.push({ label: "Add Day Note", onSelect: () => ui.openDay(day.id) });
  items.push({ label: "Insert day after", onSelect: () => ui.createDayAfter(day.id) });
  items.push({ label: "Duplicate day (breaks and notes only)", onSelect: () => command("schedule.duplicate_day", { dayId: day.id }, `Duplicated ${day.label}`) });
  if (!day.isOffDay) {
    items.push({ label: day.callSheet ? "Open Call Sheet" : "Create Call Sheet", separatorBefore: true, onSelect: () => ui.openCallSheet(day) });
  }
  items.push({
    label: "Delete day",
    danger: true,
    separatorBefore: true,
    onSelect: () => command("schedule.delete_day", { dayId: day.id }, `Deleted ${day.label}; its scenes returned to Unscheduled`),
  });
  return items;
}

export function stripMenuItems(s: ScheduleStripDto, view: ScheduleView, ui: ScheduleUi): MenuItemSpec[] {
  const label = s.number !== null ? `Scene ${s.number}` : "Scene";
  const items: MenuItemSpec[] = [{ label: "Details…", onSelect: () => ui.openStrip(s.id) }, { label: "Move to", header: true, separatorBefore: true }];
  for (const d of view.days) {
    if (d.isOffDay) continue;
    items.push({
      label: d.title,
      disabled: d.id === s.dayId,
      onSelect: () => runScheduleMove("schedule.move_strip", { stripId: s.id, dayId: d.id }, `Moved ${label} to ${d.label}`),
    });
  }
  if (s.dayId) {
    const day = view.days.find((d) => d.id === s.dayId);
    const order = day ? day.items.map((i) => i.strip?.id ?? i.marker?.id) : [];
    const idx = order.indexOf(s.id);
    items.push({
      label: "Move up",
      separatorBefore: true,
      disabled: idx <= 0,
      onSelect: () => runScheduleMove("schedule.move_strip", { stripId: s.id, dayId: s.dayId, index: idx - 1 }, `Reordered ${day?.label ?? "the day"}`),
    });
    items.push({
      label: "Move down",
      disabled: idx < 0 || idx >= order.length - 1,
      onSelect: () => runScheduleMove("schedule.move_strip", { stripId: s.id, dayId: s.dayId, index: idx + 1 }, `Reordered ${day?.label ?? "the day"}`),
    });
    items.push({
      label: "Remove from Day",
      onSelect: () => runScheduleMove("schedule.move_strip", { stripId: s.id, dayId: null }, `Unscheduled ${label}`),
    });
  }
  return items;
}

function MenuButton({ label, items }: { label: string; items: MenuItems }) {
  return (
    <Menu
      align="end"
      items={items}
      trigger={
        <IconButton label={label}>
          <MoreHorizontal size={14} />
        </IconButton>
      }
    />
  );
}

// ------------------------------------------------------------------ sortable wrappers

type ItemInfo = { container: string; kind: "strip" | "marker"; strip?: ScheduleStripDto; marker?: ScheduleMarkerDto };

function SortableItem({ id, container, children }: { id: string; container: string; children: (handle: Record<string, unknown>, dragging: boolean) => ReactNode }) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id, data: { container } });
  // The strip keeps its own role/tabIndex; dnd-kit adds the drag description and
  // keyboard dragging (Space to pick up, arrows to move, Space to drop).
  const { role: _role, tabIndex: _tab, ...aria } = attributes;
  return (
    <div ref={setNodeRef} style={{ transform: CSS.Transform.toString(transform), transition }}>
      {children({ ...aria, ...listeners }, isDragging)}
    </div>
  );
}

function Container({ id, className, children }: { id: string; className: string; children: ReactNode }) {
  const { setNodeRef } = useDroppable({ id, data: { container: id } });
  return (
    <div ref={setNodeRef} className={className}>
      {children}
    </div>
  );
}

// ------------------------------------------------------------------ board

export function Board({ view, ui, warnStripIds }: { view: ScheduleView; ui: ScheduleUi; warnStripIds: Set<string> }) {
  const [activeId, setActiveId] = useState<string | null>(null);
  const [hot, setHot] = useState<string | null>(null);
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );

  const { containers, info } = useMemo(() => {
    const containers: Record<string, string[]> = { pool: view.unscheduled.map((s) => s.id) };
    const info = new Map<string, ItemInfo>();
    for (const s of view.unscheduled) info.set(s.id, { container: "pool", kind: "strip", strip: s });
    for (const d of view.days) {
      containers[d.id] = d.items.map((i) => (i.strip ? i.strip.id : i.marker!.id));
      for (const i of d.items) {
        if (i.strip) info.set(i.strip.id, { container: d.id, kind: "strip", strip: i.strip });
        else if (i.marker) info.set(i.marker.id, { container: d.id, kind: "marker", marker: i.marker });
      }
    }
    return { containers, info };
  }, [view]);

  const containerOf = (id: string | number | undefined, data?: { container?: string }): string | null => {
    if (id === undefined) return null;
    if (data?.container) return data.container;
    const k = String(id);
    return k in containers ? k : (info.get(k)?.container ?? null);
  };

  const onDragStart = (e: DragStartEvent) => setActiveId(String(e.active.id));
  const onDragOver = (e: DragOverEvent) => setHot(containerOf(e.over?.id, e.over?.data.current as { container?: string } | undefined));
  const onDragEnd = (e: DragEndEvent) => {
    setActiveId(null);
    setHot(null);
    const id = String(e.active.id);
    const it = info.get(id);
    const target = containerOf(e.over?.id, e.over?.data.current as { container?: string } | undefined);
    if (!it || !target || !e.over) return;
    const overId = String(e.over.id);
    const label = it.strip ? (it.strip.number !== null ? `Scene ${it.strip.number}` : "Scene") : "Break";
    if (target === "pool") {
      if (it.kind === "strip" && it.container !== "pool") void runScheduleMove("schedule.move_strip", { stripId: id, dayId: null }, `Unscheduled ${label}`);
      return;
    }
    const day = view.days.find((d) => d.id === target);
    if (!day) return;
    if (day.isOffDay && it.kind === "strip") {
      toast.error("This is an off day. Change it back to a shooting day before adding scenes.");
      return;
    }
    const list = containers[target] ?? [];
    const overIndex = list.indexOf(overId);
    const index = overIndex >= 0 ? overIndex : null;
    if (it.container === target && (index === null ? list.indexOf(id) === list.length - 1 : list.indexOf(id) === index)) return;
    if (it.kind === "strip") {
      const text = it.container === target ? `Reordered ${day.label}` : it.container === "pool" ? `Scheduled ${label} on ${day.label}` : `Moved ${label} to ${day.label}`;
      void runScheduleMove("schedule.move_strip", { stripId: id, dayId: target, index }, text);
    } else {
      void command("schedule.move_marker", { markerId: id, dayId: target, index }, `Moved a break on ${day.label}`);
    }
  };

  const active = activeId ? info.get(activeId) : undefined;

  return (
    <DndContext sensors={sensors} collisionDetection={closestCorners} onDragStart={onDragStart} onDragOver={onDragOver} onDragEnd={onDragEnd} onDragCancel={() => { setActiveId(null); setHot(null); }}>
      <div className="of-board">
        <Container id="pool" className={`pool${hot === "pool" ? " hot" : ""}`}>
          <div className="h4" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            Unscheduled <Chip tone="out">{view.unscheduled.length}</Chip>
          </div>
          <SortableContext id="pool" items={containers.pool} strategy={verticalListSortingStrategy}>
            {view.unscheduled.map((s) => (
              <SortableItem key={s.id} id={s.id} container="pool">
                {(handle, dragging) => (
                  <div className={dragging ? "of-dragging-wrap" : undefined} style={{ opacity: dragging ? 0.4 : 1 }}>
                    <StripRow strip={s} warn={warnStripIds.has(s.id)} onOpen={() => ui.openStrip(s.id)} dragHandleProps={handle} menu={<MenuButton label={`Scene ${s.number ?? ""} actions`} items={() => stripMenuItems(s, view, ui)} />} />
                  </div>
                )}
              </SortableItem>
            ))}
          </SortableContext>
          {view.unscheduled.length === 0 && <div className="sm muted" style={{ padding: 8 }}>Every scene is scheduled.</div>}
        </Container>

        <div className="of-days">
          {view.days.length === 0 && (
            <div className="daycol">
              <div className="of-empty">Create a shooting day, then drag scenes into it.</div>
            </div>
          )}
          {view.days.map((d) => (
            <Container key={d.id} id={d.id} className={`daycol${hot === d.id ? " hot" : ""}${d.isOffDay ? " off" : ""}`}>
              <div className="dch">
                <CalendarDays size={13} aria-hidden />
                <button type="button" className="of-dayname" onClick={() => ui.openDay(d.id)}>
                  {d.title}
                </button>
                {d.callSheet && <Chip tone={d.callSheet.status === "Needs Refresh" ? "y" : d.callSheet.status === "Final" || d.callSheet.status === "Issued" ? "g" : "default"}>Call sheet: {d.callSheet.status === "Final" ? "Finalized" : d.callSheet.status}</Chip>}
                {d.openWarnings > 0 && (
                  <Chip tone="r" title="Open warnings on this day">
                    <TriangleAlert size={11} aria-hidden /> {d.openWarnings}
                  </Chip>
                )}
                <span className="tot">{d.isOffDay ? "Off day" : d.summary.totalLabel}{d.summary.overTarget ? " · over target" : ""}</span>
                <MenuButton label={`${d.label} actions`} items={dayMenuItems(d, ui)} />
              </div>
              <SortableContext id={d.id} items={containers[d.id]} strategy={verticalListSortingStrategy}>
                {d.items.map((i) =>
                  i.strip ? (
                    <SortableItem key={i.strip.id} id={i.strip.id} container={d.id}>
                      {(handle, dragging) => (
                        <div style={{ opacity: dragging ? 0.4 : 1 }}>
                          <StripRow strip={i.strip!} warn={warnStripIds.has(i.strip!.id)} onOpen={() => ui.openStrip(i.strip!.id)} dragHandleProps={handle} menu={<MenuButton label={`Scene ${i.strip!.number ?? ""} actions`} items={() => stripMenuItems(i.strip!, view, ui)} />} />
                        </div>
                      )}
                    </SortableItem>
                  ) : (
                    <SortableItem key={i.marker!.id} id={i.marker!.id} container={d.id}>
                      {(handle, dragging) => (
                        <div style={{ opacity: dragging ? 0.4 : 1 }}>
                          <MarkerRow
                            marker={i.marker!}
                            dragHandleProps={handle}
                            onOpen={() => ui.editMarker(d.id, i.marker!)}
                          />
                        </div>
                      )}
                    </SortableItem>
                  ),
                )}
              </SortableContext>
              {d.items.length === 0 && (
                <div className="of-empty">{d.isOffDay ? (d.notes ?? "Off day — no scenes.") : "No scenes scheduled. Drag scenes here."}</div>
              )}
            </Container>
          ))}
        </div>
      </div>
      <DragOverlay>
        {active?.strip ? (
          <div className="of-overlay-wrap">
            <StripRow strip={active.strip} />
          </div>
        ) : active?.marker ? (
          <MarkerRow marker={active.marker} />
        ) : null}
      </DragOverlay>
    </DndContext>
  );
}
