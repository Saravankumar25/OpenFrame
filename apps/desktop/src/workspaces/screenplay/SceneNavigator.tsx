// Scene navigator (FSD §15.7, §16.3; UX §3.11): generated scene numbers and
// headings; click jumps within the same document; "+" adds a scene; drag or
// "Move up/down" reorders (numbers follow the new order).

import { useEffect, useRef } from "react";
import { Plus, GripVertical } from "lucide-react";
import { DndContext, KeyboardSensor, PointerSensor, closestCenter, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { SortableContext, sortableKeyboardCoordinates, useSortable, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { ContextMenu, type MenuItemSpec } from "../../design-system";
import type { OutlineScene } from "./editor/ScreenplayEditor";

function Row({ s, on, canEdit, onJump, menu }: {
  s: OutlineScene;
  on: boolean;
  canEdit: boolean;
  onJump: (id: string) => void;
  menu: MenuItemSpec[];
}) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: s.id, disabled: !canEdit });
  const ref = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    if (on) ref.current?.scrollIntoView({ block: "nearest" });
  }, [on]);
  return (
    <ContextMenu items={menu}>
      <div
        ref={(el) => {
          setNodeRef(el);
          ref.current = el;
        }}
        className={`sni spx-sni${on ? " on" : ""}${isDragging ? " dragging" : ""}`}
        style={{ transform: CSS.Transform.toString(transform), transition }}
      >
        <button type="button" className="spx-sni-main" onClick={() => onJump(s.id)} aria-current={on ? "location" : undefined}>
          <b>{s.number}</b>
          <span className={s.heading.trim() ? "" : "muted"}>{s.heading.trim() || "Untitled scene"}</span>
        </button>
        {canEdit && (
          <span className="spx-grip" {...attributes} {...listeners} aria-label={`Reorder scene ${s.number}`} title="Drag to reorder">
            <GripVertical size={12} />
          </span>
        )}
      </div>
    </ContextMenu>
  );
}

export function SceneNavigator({ scenes, currentSceneId, canEdit, onJump, onNew, onMove, menuFor }: {
  scenes: OutlineScene[];
  currentSceneId: string | null;
  canEdit: boolean;
  onJump: (id: string) => void;
  onNew: () => void;
  onMove: (id: string, index: number) => void;
  menuFor: (s: OutlineScene) => MenuItemSpec[];
}) {
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );
  const onDragEnd = (e: DragEndEvent) => {
    if (!e.over || e.active.id === e.over.id) return;
    const to = scenes.findIndex((s) => s.id === e.over!.id);
    if (to >= 0) onMove(String(e.active.id), to);
  };
  return (
    <nav className="sp-nav spx-nav" aria-label="Scenes">
      <div className="row" style={{ marginBottom: 8, padding: "0 4px" }}>
        <span className="h4" style={{ margin: 0 }}>Scenes</span>
        <span className="chip out">{scenes.length}</span>
        <span className="grow" />
        {canEdit && (
          <button type="button" className="iconbtn" style={{ width: 24, height: 24 }} aria-label="New scene" title="New scene after the current one" onClick={onNew}>
            <Plus size={14} />
          </button>
        )}
      </div>
      <div className="spx-scroll spx-nav-list">
        <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={onDragEnd}>
          <SortableContext items={scenes.map((s) => s.id)} strategy={verticalListSortingStrategy}>
            {scenes.map((s) => (
              <Row key={s.id} s={s} on={s.id === currentSceneId} canEdit={canEdit} onJump={onJump} menu={menuFor(s)} />
            ))}
          </SortableContext>
        </DndContext>
      </div>
    </nav>
  );
}
