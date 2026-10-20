// Scene navigator (FSD §15.7, §16.3; UX §3.11): generated scene numbers and
// headings; click jumps within the same document; "+" adds a scene; drag or
// "Move up/down" reorders (numbers follow the new order). Optional columns the
// writer can show or hide (SCRIPT-017): Page, Notes, Revision, Production number.
// A column whose data does not exist in this draft is not offered.

import { memo, useCallback, useEffect, useMemo, useRef } from "react";
import { Plus, GripVertical, Columns3 } from "lucide-react";
import { DndContext, KeyboardSensor, PointerSensor, closestCenter, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { SortableContext, sortableKeyboardCoordinates, useSortable, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { ContextMenu, Menu, type MenuItemSpec } from "../../design-system";
import type { OutlineScene } from "./editor/ScreenplayEditor";
import type { NavigatorSceneColumns } from "../../ipc/generated/NavigatorSceneColumns";
import { NAV_COLUMNS, type NavColumn } from "./util";

export interface NavColumnsData {
  rows: Map<string, NavigatorSceneColumns>;
  hasRevisions: boolean;
  hasProductionNumbers: boolean;
}

const REVISION_LABEL = { Added: "New", Changed: "Changed", Moved: "Moved" } as const;

/** Columns that can show something for this draft. */
export function availableColumns(data: NavColumnsData | null): Set<NavColumn> {
  const out = new Set<NavColumn>(["page", "notes"]);
  if (data?.hasRevisions) out.add("revision");
  if (data?.hasProductionNumbers) out.add("production");
  return out;
}

/** One cell: a short visible value with a spoken label, or an empty cell that is not announced. */
function Cell({ label, value, cls = "" }: { label: string | null; value: string | number | null; cls?: string }) {
  if (value === null || value === "" || label === null) return <span className={`spx-col xs ${cls}`} aria-hidden="true" />;
  return (
    <span className={`spx-col xs ${cls}`} aria-label={label}>
      {value}
    </span>
  );
}

function Cells({ columns, c }: { columns: NavColumn[]; c: NavigatorSceneColumns | undefined }) {
  return (
    <>
      {columns.map((col) => {
        switch (col) {
          case "page":
            return <Cell key={col} cls="muted" label={c?.page ? `Page ${c.page}` : null} value={c?.page ?? null} />;
          case "notes":
            return <Cell key={col} cls="muted" label={c && c.notes > 0 ? `${c.notes} note${c.notes === 1 ? "" : "s"}` : null} value={c && c.notes > 0 ? c.notes : null} />;
          case "revision":
            return <Cell key={col} cls="spx-col-rev" label={c?.revision ? `Revision: ${REVISION_LABEL[c.revision.kind]}` : null} value={c?.revision ? REVISION_LABEL[c.revision.kind] : null} />;
          case "production":
            return <Cell key={col} label={c?.productionNumber ? `Production number ${c.productionNumber}` : null} value={c?.productionNumber ?? null} />;
        }
      })}
    </>
  );
}

// Rows are memoised and build their context menu only when it opens: typing in a
// 180-scene script must not re-render (or rebuild menus for) every row.
const Row = memo(function Row({ s, on, canEdit, onJump, menuFor, columns, cols }: {
  s: OutlineScene;
  on: boolean;
  canEdit: boolean;
  onJump: (id: string) => void;
  menuFor: (s: OutlineScene) => MenuItemSpec[];
  columns: NavColumn[];
  cols: NavigatorSceneColumns | undefined;
}) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: s.id, disabled: !canEdit });
  const ref = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    if (on) ref.current?.scrollIntoView({ block: "nearest" });
  }, [on]);
  return (
    <ContextMenu items={() => menuFor(s)}>
      <div
        ref={(el) => {
          setNodeRef(el);
          ref.current = el;
        }}
        className={`sni spx-sni${on ? " on" : ""}${isDragging ? " dragging" : ""}`}
        style={{ transform: CSS.Transform.toString(transform), transition }}
      >
        <button type="button" className="spx-sni-main" onClick={() => onJump(s.id)} aria-current={on ? "location" : undefined}>
          <b>{s.label}</b>
          <span className={`grow${s.heading.trim() ? "" : " muted"}`} title={s.heading.trim() || undefined}>{s.heading.trim() || "Untitled scene"}</span>
          {columns.length > 0 && <Cells columns={columns} c={cols} />}
        </button>
        {canEdit && (
          <span className="spx-grip" {...attributes} {...listeners} aria-label={`Reorder scene ${s.label}`} title="Drag to reorder">
            <GripVertical size={12} />
          </span>
        )}
      </div>
    </ContextMenu>
  );
});

export const SceneNavigator = memo(function SceneNavigator({ scenes, currentSceneId, canEdit, onJump, onNew, onMove, menuFor, columns = [], columnData = null, onToggleColumn }: {
  scenes: OutlineScene[];
  currentSceneId: string | null;
  canEdit: boolean;
  onJump: (id: string) => void;
  onNew: () => void;
  onMove: (id: string, index: number) => void;
  menuFor: (s: OutlineScene) => MenuItemSpec[];
  /** Columns the writer chose to show. */
  columns?: NavColumn[];
  columnData?: NavColumnsData | null;
  onToggleColumn?: (c: NavColumn) => void;
}) {
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );
  // Callers pass fresh closures on every render; rows see stable wrappers that
  // always call the latest one.
  const latest = useRef({ onJump, menuFor });
  latest.current = { onJump, menuFor };
  const jump = useCallback((id: string) => latest.current.onJump(id), []);
  const menu = useCallback((s: OutlineScene) => latest.current.menuFor(s), []);
  const ids = useMemo(() => scenes.map((s) => s.id), [scenes]);
  const available = availableColumns(columnData);
  const shown = columns.filter((c) => available.has(c));
  const columnMenu: MenuItemSpec[] = [
    { label: "Show columns", header: true },
    ...NAV_COLUMNS.map((c) => {
      // Until the column data has loaded (it loads once a column is shown) every column is offered.
      const missing = !!columnData && !available.has(c.id);
      return {
        label: `${columns.includes(c.id) ? "\u2713 " : ""}${c.label}${missing ? (c.id === "revision" ? " (revision drafts only)" : " (none yet)") : ""}`,
        disabled: missing && !columns.includes(c.id),
        onSelect: () => onToggleColumn?.(c.id),
      };
    }),
  ];
  const onDragEnd = (e: DragEndEvent) => {
    if (!e.over || e.active.id === e.over.id) return;
    const to = scenes.findIndex((s) => s.id === e.over!.id);
    if (to >= 0) onMove(String(e.active.id), to);
  };
  return (
    <nav className={`sp-nav spx-nav${shown.length > 1 ? " wide" : ""}`} aria-label="Scenes">
      <div className="row" style={{ marginBottom: 8, padding: "0 4px" }}>
        <span className="h4" style={{ margin: 0 }}>Scenes</span>
        <span className="chip out">{scenes.length}</span>
        <span className="grow" />
        {onToggleColumn && (
          <Menu
            trigger={
              <button type="button" className="iconbtn" style={{ width: 24, height: 24 }} aria-label="Navigator columns" title="Show or hide columns">
                <Columns3 size={14} />
              </button>
            }
            items={columnMenu}
            align="end"
          />
        )}
        {canEdit && (
          <button type="button" className="iconbtn" style={{ width: 24, height: 24 }} aria-label="New scene" title="New scene after the current one" onClick={onNew}>
            <Plus size={14} />
          </button>
        )}
      </div>
      {shown.length > 0 && (
        <div className="spx-nav-head xs muted" aria-hidden="true">
          <b>#</b>
          <span className="grow">Heading</span>
          {shown.map((c) => (
            <span key={c} className="spx-col">{c === "production" ? "Prod." : c === "revision" ? "Rev." : NAV_COLUMNS.find((x) => x.id === c)?.label}</span>
          ))}
        </div>
      )}
      <div className="spx-scroll spx-nav-list">
        <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={onDragEnd}>
          <SortableContext items={ids} strategy={verticalListSortingStrategy}>
            {scenes.map((s) => (
              <Row key={s.id} s={s} on={s.id === currentSceneId} canEdit={canEdit} onJump={jump} menuFor={menu} columns={shown} cols={columnData?.rows.get(s.id)} />
            ))}
          </SortableContext>
        </DndContext>
      </div>
    </nav>
  );
});
