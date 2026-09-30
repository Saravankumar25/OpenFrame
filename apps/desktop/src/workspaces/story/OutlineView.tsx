// Story Board — Outline view (FSD §7.5, UX §3.7, mock 052): the same objects as
// the Board view shown as an indented hierarchy. Reorder by drag (grip) or
// keyboard (Alt+↑/↓); nothing here adds metadata or touches the screenplay.

import { Fragment } from "react";
import { DndContext, DragOverlay } from "@dnd-kit/core";
import { ChevronDown, ChevronRight, GripVertical, MoreHorizontal, ParkingSquare } from "lucide-react";
import { ContextMenu, Menu, cx } from "../../design-system";
import type { StoryActDto } from "../../ipc/generated/StoryActDto";
import type { StoryBoardState } from "../../ipc/generated/StoryBoardState";
import type { StoryContainerRef } from "../../ipc/generated/StoryContainerRef";
import type { StoryItem } from "../../ipc/generated/StoryItem";
import { story } from "../../api/story";
import { reportError } from "../../ipc/query";
import { newBeat, newCard } from "./actions";
import { useActDnd, useContainerDrop, useItemDnd, useStoryDnd, type StoryDnd } from "./dnd";
import { COLOR_VARS, InlineText, commitItemText, itemMenu, onItemClick, renameAct, useItemKeys } from "./items";
import { ACT, PARKING, SEQ, UNASSIGNED, actCountLabel, matchesFilter, refKey, sameContainer } from "./model";
import { useStoryUi } from "./ui";
import { requestDeleteAct } from "./dialogs";

interface Ctx {
  board: StoryBoardState;
  episodeId: string | null;
  readOnly: boolean;
  dnd: StoryDnd;
  collapsed: Set<string>;
  keys: ReturnType<typeof useItemKeys>;
}

const INDENT = [0, 22, 46];

export function OutlineView({ board, episodeId, readOnly }: { board: StoryBoardState; episodeId: string | null; readOnly: boolean }) {
  const dnd = useStoryDnd(board, episodeId, "y");
  const keys = useItemKeys(board, episodeId, readOnly);
  const ctx: Ctx = { board, episodeId, readOnly, dnd, collapsed: new Set(board.collapsedIds), keys };
  return (
    <DndContext
      sensors={dnd.sensors}
      collisionDetection={dnd.collision}
      onDragStart={dnd.onDragStart}
      onDragMove={dnd.onDragMove}
      onDragOver={dnd.onDragOver}
      onDragEnd={dnd.onDragEnd}
      onDragCancel={dnd.onDragCancel}
    >
      <div className="outline story-outline" role="tree" aria-label="Story outline" onClick={() => useStoryUi.getState().clearSelection()}>
        {board.acts.map((a) => (
          <Fragment key={a.id}>
            {dnd.actHint && dnd.actHint.before === a.id && <div className="ins" aria-hidden />}
            <ActRows act={a} ctx={ctx} />
          </Fragment>
        ))}
        {dnd.actHint && dnd.actHint.before === null && <div className="ins" aria-hidden />}
        {board.unassigned.length > 0 && (
          <>
            <HeaderRow container={UNASSIGNED} label={`Unassigned (${board.unassigned.length})`} ctx={ctx} droppable={false} />
            <div className="story-empty-hint" style={{ marginLeft: 22 }}>
              Restored items whose Act or Sequence no longer exists. Drag them back into the story.
            </div>
            <Rows container={UNASSIGNED} items={board.unassigned} depth={1} ctx={ctx} />
          </>
        )}
        <HeaderRow container={PARKING} label={`Parking Lot (${board.parking.length})`} ctx={ctx} droppable={!readOnly} />
        <Rows container={PARKING} items={board.parking} depth={1} ctx={ctx} />
      </div>
      <DragOverlay dropAnimation={null}>
        {dnd.dragging && <div className="ol story-row-overlay">{dnd.dragging.label}</div>}
      </DragOverlay>
    </DndContext>
  );
}

function HeaderRow({ container, label, ctx, droppable }: { container: StoryContainerRef; label: string; ctx: Ctx; droppable: boolean }) {
  const drop = useContainerDrop(container, true, !droppable);
  const hinted = !!ctx.dnd.hint && sameContainer(ctx.dnd.hint.target, container);
  return (
    <div ref={drop.setNodeRef} className={cx("ol act1 story-lot", hinted && "drop-on")} role="treeitem" aria-level={1}>
      {container.parentType === "parking" && <ParkingSquare size={14} aria-hidden />}
      {label}
    </div>
  );
}

function ActRows({ act, ctx }: { act: StoryActDto; ctx: Ctx }) {
  const { drag, drop } = useActDnd(act.id, ctx.readOnly);
  const into = useContainerDrop(ACT(act.id), true, ctx.readOnly);
  const renaming = useStoryUi((s) => s.renamingId === act.id);
  const collapsed = ctx.collapsed.has(act.id);
  const hinted = !!ctx.dnd.hint && sameContainer(ctx.dnd.hint.target, ACT(act.id));
  const ui = useStoryUi.getState();
  const setRefs = (el: HTMLElement | null) => {
    drop.setNodeRef(el);
    into.setNodeRef(el);
  };
  return (
    <>
      <div
        ref={setRefs}
        className={cx("ol act1", hinted && "drop-on", drag.isDragging && "drag-src")}
        role="treeitem"
        aria-level={1}
        aria-expanded={!collapsed}
      >
        <span ref={drag.setNodeRef} {...drag.listeners} {...drag.attributes} className="grip" aria-label={`Drag ${act.title}`}>
          <GripVertical size={14} />
        </span>
        <button
          type="button"
          className="iconbtn sm"
          aria-label={collapsed ? `Expand ${act.title}` : `Collapse ${act.title}`}
          onClick={(e) => {
            e.stopPropagation();
            void story.setCollapsed(act.id, !collapsed).catch(reportError);
          }}
        >
          {collapsed ? <ChevronRight size={14} /> : <ChevronDown size={14} />}
        </button>
        {renaming ? (
          <InlineText
            value={act.title}
            ariaLabel="Act title"
            onCancel={() => ui.setRenaming(null)}
            onCommit={(v) => {
              ui.setRenaming(null);
              void renameAct(act.id, act.title, v);
            }}
          />
        ) : (
          <span className="grow" onDoubleClick={() => !ctx.readOnly && ui.setRenaming(act.id)}>
            {act.title}
          </span>
        )}
        <span className="muted" style={{ fontWeight: 600, textTransform: "none" }}>{actCountLabel(act)}</span>
        <Menu
          align="end"
          items={[
            { label: "Rename", disabled: ctx.readOnly, onSelect: () => ui.setRenaming(act.id) },
            { label: "Edit note…", disabled: ctx.readOnly, onSelect: () => ui.openDialog({ type: "actNote", actId: act.id }) },
            { label: "Add Sequence", disabled: ctx.readOnly, onSelect: () => ui.setAddingSequenceIn(act.id) },
            { label: "Add Scene", disabled: ctx.readOnly, onSelect: () => void newCard(ctx.episodeId, ACT(act.id)) },
            { label: "Add Beat", disabled: ctx.readOnly, onSelect: () => void newBeat(ctx.episodeId, ACT(act.id)) },
            {
              label: "Move Up",
              disabled: ctx.readOnly || ctx.board.acts[0]?.id === act.id,
              onSelect: () => {
                const ids = ctx.board.acts.map((a) => a.id);
                void story.moveAct(act.id, ids[ids.indexOf(act.id) - 1]).catch(reportError);
              },
            },
            {
              label: "Move Down",
              disabled: ctx.readOnly || ctx.board.acts[ctx.board.acts.length - 1]?.id === act.id,
              onSelect: () => {
                const ids = ctx.board.acts.map((a) => a.id);
                void story.moveAct(act.id, ids[ids.indexOf(act.id) + 2] ?? null).catch(reportError);
              },
            },
            { label: "Delete", danger: true, separatorBefore: true, disabled: ctx.readOnly, onSelect: () => requestDeleteAct(ctx.board, act.id) },
          ]}
          trigger={
            <button type="button" className="iconbtn sm" aria-label={`Act options for ${act.title}`}>
              <MoreHorizontal size={14} />
            </button>
          }
        />
      </div>
      {!collapsed && <Rows container={ACT(act.id)} items={act.items} depth={1} ctx={ctx} />}
      {!collapsed && act.items.length === 0 && (
        <div className="story-empty-hint" style={{ marginLeft: 22 }}>Add a sequence or scene.</div>
      )}
      <NewSequenceRow actId={act.id} />
    </>
  );
}

function NewSequenceRow({ actId }: { actId: string }) {
  const adding = useStoryUi((s) => s.addingSequenceIn === actId);
  if (!adding) return null;
  const ui = useStoryUi.getState();
  return (
    <div className="ol seq1">
      <InlineText
        value=""
        ariaLabel="Sequence name"
        placeholder="Sequence name"
        onCancel={() => ui.setAddingSequenceIn(null)}
        onCommit={(v) => {
          ui.setAddingSequenceIn(null);
          if (v.trim()) void story.createSequence(actId, v.trim()).catch(reportError);
        }}
      />
    </div>
  );
}

function Rows({ container, items, depth, ctx }: { container: StoryContainerRef; items: StoryItem[]; depth: number; ctx: Ctx }) {
  const filter = useStoryUi((s) => s.filter);
  const hint = ctx.dnd.hint;
  const here = !!hint && sameContainer(hint.target, container);
  const ins = <div className="ins" style={{ marginLeft: INDENT[depth] }} aria-hidden />;
  return (
    <>
      {items.map((it) => {
        const showIns = here && hint?.before && hint.before.kind === it.kind && hint.before.id === it.id;
        return (
          <Fragment key={`${it.kind}:${it.id}`}>
            {showIns && ins}
            {matchesFilter(it, filter) && <Row item={it} depth={depth} ctx={ctx} />}
            {it.kind === "sequence" && !ctx.collapsed.has(it.id) && matchesFilter(it, filter) && (
              <>
                <Rows container={SEQ(it.id)} items={it.items} depth={Math.min(depth + 1, 2)} ctx={ctx} />
                {it.items.length === 0 && (
                  <div className="story-empty-hint" style={{ marginLeft: INDENT[Math.min(depth + 1, 2)] }}>Drop scenes here.</div>
                )}
              </>
            )}
          </Fragment>
        );
      })}
      {here && !hint?.before && ins}
    </>
  );
}

function Row({ item, depth, ctx }: { item: StoryItem; depth: number; ctx: Ctx }) {
  const node = useItemDnd(item.kind, item.id, ctx.readOnly);
  const key = refKey({ kind: item.kind, id: item.id });
  const selected = useStoryUi((s) => s.selected.includes(key));
  const editing = useStoryUi((s) => s.editingKey === key || (item.kind === "sequence" && s.renamingId === item.id));
  const ui = useStoryUi.getState();
  const collapsed = ctx.collapsed.has(item.id);
  const hinted = item.kind === "sequence" && !!ctx.dnd.hint && sameContainer(ctx.dnd.hint.target, SEQ(item.id));
  const prefix = item.kind === "sequence" ? "Sequence — " : item.kind === "beat" ? "Beat — " : "Scene — ";
  const text = item.kind === "sequence" ? item.title : item.kind === "beat" ? item.text : item.shortDescription || item.sceneHeading || "";
  const color = item.kind !== "sequence" && item.color ? COLOR_VARS[item.color] : undefined;
  return (
    <ContextMenu items={itemMenu(ctx.board, item, ctx.episodeId, ctx.readOnly)}>
      <div
        ref={node.setNodeRef}
        data-story-key={key}
        tabIndex={0}
        role="treeitem"
        aria-level={depth + 1}
        aria-selected={selected}
        className={cx("ol", depth === 1 ? "seq1" : "it", selected && "sel", hinted && "drop-on", node.isDragging && "drag-src")}
        style={{ fontWeight: item.kind === "sequence" ? 700 : 400 }}
        onClick={(e) => onItemClick(e, key)}
        onDoubleClick={(e) => {
          e.stopPropagation();
          ui.openDrawer({ kind: item.kind, id: item.id });
        }}
        onKeyDown={(e) => !editing && ctx.keys(e, item)}
      >
        <span {...node.listeners} {...node.attributes} className="grip" aria-label={`Drag ${prefix.trim()}`} tabIndex={-1}>
          <GripVertical size={14} />
        </span>
        {item.kind === "sequence" && (
          <button
            type="button"
            className="iconbtn sm"
            aria-label={collapsed ? `Expand ${item.title}` : `Collapse ${item.title}`}
            onClick={(e) => {
              e.stopPropagation();
              void story.setCollapsed(item.id, !collapsed).catch(reportError);
            }}
          >
            {collapsed ? <ChevronRight size={12} /> : <ChevronDown size={12} />}
          </button>
        )}
        {color && <span className="dot" style={{ background: color }} aria-hidden />}
        {editing ? (
          <InlineText
            value={text}
            ariaLabel={item.kind === "sequence" ? "Sequence name" : item.kind === "beat" ? "Beat text" : "Short description"}
            onCancel={() => {
              ui.setEditing(null);
              ui.setRenaming(null);
            }}
            onCommit={(v) => {
              ui.setEditing(null);
              ui.setRenaming(null);
              void commitItemText(item, v);
            }}
          />
        ) : (
          <span className={cx("grow truncate", item.kind === "beat" && item.state === "converted" && "muted")}>
            <span className="muted">{prefix}</span>
            {text || <i className="muted">blank</i>}
            {item.kind === "beat" && item.state === "converted" && <span className="chip" style={{ marginLeft: 6 }}>Converted</span>}
          </span>
        )}
        {item.kind === "card" && item.sceneHeading && <b className="muted" title="has a scene heading">H</b>}
        {item.kind === "sequence" && <span className="muted">{item.cardCount}</span>}
      </div>
    </ContextMenu>
  );
}
