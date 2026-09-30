// Story Board — Board view (FSD §7.4, §89; UX §3.6; mocks 049–058, 066):
// act bands side by side (horizontal scroll), sequence groups, compact scene
// cards, dashed beats, the hatched Parking Lot and an Unassigned area.

import { Fragment, type CSSProperties } from "react";
import { DndContext, DragOverlay } from "@dnd-kit/core";
import { ChevronDown, ChevronRight, FileText, GripVertical, MessageSquare, MoreHorizontal, ParkingSquare, Plus } from "lucide-react";
import { ContextMenu, Menu, cx } from "../../design-system";
import type { StoryActDto } from "../../ipc/generated/StoryActDto";
import type { StoryBoardState } from "../../ipc/generated/StoryBoardState";
import type { StoryContainerRef } from "../../ipc/generated/StoryContainerRef";
import type { StoryItem } from "../../ipc/generated/StoryItem";
import { story } from "../../api/story";
import { reportError } from "../../ipc/query";
import { newBeat, newCard } from "./actions";
import { useActDnd, useContainerDrop, useItemDnd, useStoryDnd, type StoryDnd } from "./dnd";
import {
  COLOR_VARS,
  InlineText,
  commitItemText,
  itemMenu,
  onItemClick,
  renameAct,
  useItemKeys,
} from "./items";
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

export function BoardView({ board, episodeId, readOnly }: { board: StoryBoardState; episodeId: string | null; readOnly: boolean }) {
  const dnd = useStoryDnd(board, episodeId, "x");
  const zoom = useStoryUi((s) => s.zoom);
  const parkingOpen = useStoryUi((s) => s.parkingOpen);
  const addingAct = useStoryUi((s) => s.addingAct);
  const keys = useItemKeys(board, episodeId, readOnly);
  const ctx: Ctx = { board, episodeId, readOnly, dnd, collapsed: new Set(board.collapsedIds), keys };
  const style = { "--z": zoom / 100 } as CSSProperties;

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
      <div
        className={cx("story-canvas", zoom <= 70 && "zoom-low")}
        style={style}
        onClick={() => useStoryUi.getState().clearSelection()}
        role="region"
        aria-label="Story Board"
      >
        <div className="board">
          {board.acts.map((a) => (
            <Fragment key={a.id}>
              {dnd.actHint && dnd.actHint.before === a.id && <div className="ins-v" aria-hidden />}
              <ActBand act={a} ctx={ctx} />
            </Fragment>
          ))}
          {dnd.actHint && dnd.actHint.before === null && <div className="ins-v" aria-hidden />}
          {!readOnly && (addingAct ? <NewActBand episodeId={episodeId} /> : <AddActButton />)}
          {board.unassigned.length > 0 && <UnassignedColumn ctx={ctx} />}
          {parkingOpen && <ParkingColumn ctx={ctx} />}
        </div>
      </div>
      <DragOverlay dropAnimation={null}>
        {dnd.dragging && (
          <div className={dnd.dragging.actId ? "actband drag-overlay" : "sc drag"} style={{ width: 220 }}>
            {dnd.dragging.label}
          </div>
        )}
      </DragOverlay>
    </DndContext>
  );
}

function AddActButton() {
  return (
    <button
      type="button"
      className="story-add-act"
      onClick={(e) => {
        e.stopPropagation();
        useStoryUi.getState().setAddingAct(true);
      }}
    >
      <Plus size={14} /> Add Act
    </button>
  );
}

/** Inline title-only act creation (mock 051). */
export function NewActBand({ episodeId }: { episodeId: string | null }) {
  const done = () => useStoryUi.getState().setAddingAct(false);
  return (
    <div className="actband" onClick={(e) => e.stopPropagation()}>
      <InlineText
        value=""
        ariaLabel="Act title"
        placeholder="Act title, e.g. Act 1 — The Return"
        onCancel={done}
        onCommit={(v) => {
          done();
          if (v.trim()) void story.createAct({ episodeId, title: v.trim() }).catch(reportError);
        }}
      />
      <div className="muted" style={{ fontSize: 12, padding: "6px 2px" }}>
        Press Enter to add the Act. You can add a note later.
      </div>
    </div>
  );
}

function ActBand({ act, ctx }: { act: StoryActDto; ctx: Ctx }) {
  const { drag, drop } = useActDnd(act.id, ctx.readOnly);
  const body = useContainerDrop(ACT(act.id), false, ctx.readOnly);
  const renaming = useStoryUi((s) => s.renamingId === act.id);
  const addingSeq = useStoryUi((s) => s.addingSequenceIn === act.id);
  const collapsed = ctx.collapsed.has(act.id);
  const hinted = !!ctx.dnd.hint && sameContainer(ctx.dnd.hint.target, ACT(act.id));
  const ui = useStoryUi.getState();
  const menu = [
    { label: "Rename", disabled: ctx.readOnly, onSelect: () => ui.setRenaming(act.id) },
    { label: "Edit note…", disabled: ctx.readOnly, onSelect: () => ui.openDialog({ type: "actNote", actId: act.id }) },
    { label: "Add Sequence", disabled: ctx.readOnly, onSelect: () => ui.setAddingSequenceIn(act.id) },
    { label: "Add Scene", disabled: ctx.readOnly, onSelect: () => void newCard(ctx.episodeId, ACT(act.id)) },
    { label: "Add Beat", disabled: ctx.readOnly, onSelect: () => void newBeat(ctx.episodeId, ACT(act.id)) },
    ...moveActItems(ctx, act.id),
    { label: "Delete", danger: true, separatorBefore: true, disabled: ctx.readOnly, onSelect: () => requestDeleteAct(ctx.board, act.id) },
  ];
  return (
    <section
      ref={drop.setNodeRef}
      className={cx("actband", hinted && "drop-on", drag.isDragging && "drag-src")}
      aria-label={`Act ${act.title}`}
    >
      <div className="ah" ref={drag.setNodeRef} {...drag.listeners} {...drag.attributes} tabIndex={-1} title={act.note ?? undefined}>
        <button
          type="button"
          className="iconbtn sm"
          aria-label={collapsed ? `Expand ${act.title}` : `Collapse ${act.title}`}
          aria-expanded={!collapsed}
          onPointerDown={(e) => e.stopPropagation()}
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
          <span className="grow truncate" onDoubleClick={() => !ctx.readOnly && ui.setRenaming(act.id)}>
            {act.title}
          </span>
        )}
        <span className="cnt">{actCountLabel(act)}</span>
        <Menu
          align="end"
          items={menu}
          trigger={
            <button type="button" className="iconbtn sm" aria-label={`Act options for ${act.title}`} onPointerDown={(e) => e.stopPropagation()}>
              <MoreHorizontal size={14} />
            </button>
          }
        />
      </div>
      {!collapsed && (
        <div ref={body.setNodeRef} className="act-body">
          <ItemList container={ACT(act.id)} items={act.items} ctx={ctx} />
          {addingSeq && (
            <div className="seq">
              <InlineText
                value=""
                ariaLabel="Sequence name"
                placeholder="Sequence name, e.g. Hero Introduction"
                onCancel={() => ui.setAddingSequenceIn(null)}
                onCommit={(v) => {
                  ui.setAddingSequenceIn(null);
                  if (v.trim()) void story.createSequence(act.id, v.trim()).catch(reportError);
                }}
              />
            </div>
          )}
          {act.items.length === 0 && !addingSeq && <div className="story-empty-hint">Add a sequence or scene.</div>}
          {!ctx.readOnly && (
            <div className="story-add-row">
              <button type="button" className="story-add" onClick={(e) => { e.stopPropagation(); void newCard(ctx.episodeId, ACT(act.id)); }}>
                <Plus size={12} /> Scene
              </button>
              <button type="button" className="story-add" onClick={(e) => { e.stopPropagation(); ui.setAddingSequenceIn(act.id); }}>
                <Plus size={12} /> Sequence
              </button>
              <button type="button" className="story-add" onClick={(e) => { e.stopPropagation(); void newBeat(ctx.episodeId, ACT(act.id)); }}>
                <Plus size={12} /> Beat
              </button>
            </div>
          )}
        </div>
      )}
    </section>
  );
}

/** Keyboard-accessible alternative to dragging an act. */
function moveActItems(ctx: Ctx, actId: string) {
  const ids = ctx.board.acts.map((a) => a.id);
  const i = ids.indexOf(actId);
  const move = (before: string | null) => void story.moveAct(actId, before).catch(reportError);
  return [
    { label: "Move Left", disabled: ctx.readOnly || i <= 0, onSelect: () => move(ids[i - 1]) },
    { label: "Move Right", disabled: ctx.readOnly || i >= ids.length - 1, onSelect: () => move(ids[i + 2] ?? null) },
  ];
}

function ItemList({ container, items, ctx }: { container: StoryContainerRef; items: StoryItem[]; ctx: Ctx }) {
  const filter = useStoryUi((s) => s.filter);
  const hint = ctx.dnd.hint;
  const here = !!hint && sameContainer(hint.target, container);
  return (
    <>
      {items.map((it) => {
        const showIns = here && hint?.before && hint.before.kind === it.kind && hint.before.id === it.id;
        if (!matchesFilter(it, filter)) return showIns ? <div key={`ins-${it.id}`} className="ins" /> : null;
        return (
          <Fragment key={`${it.kind}:${it.id}`}>
            {showIns && <div className="ins" aria-hidden />}
            {it.kind === "sequence" ? <SequenceBlock seq={it} ctx={ctx} /> : <CardNode item={it} ctx={ctx} />}
          </Fragment>
        );
      })}
      {here && !hint?.before && <div className="ins" aria-hidden />}
    </>
  );
}

function SequenceBlock({ seq, ctx }: { seq: Extract<StoryItem, { kind: "sequence" }>; ctx: Ctx }) {
  const node = useItemDnd("sequence", seq.id, ctx.readOnly);
  const body = useContainerDrop(SEQ(seq.id), false, ctx.readOnly);
  const key = refKey({ kind: "sequence", id: seq.id });
  const selected = useStoryUi((s) => s.selected.includes(key));
  const renaming = useStoryUi((s) => s.renamingId === seq.id);
  const collapsed = ctx.collapsed.has(seq.id);
  const hinted = !!ctx.dnd.hint && sameContainer(ctx.dnd.hint.target, SEQ(seq.id));
  const ui = useStoryUi.getState();
  return (
    <ContextMenu items={itemMenu(ctx.board, seq, ctx.episodeId, ctx.readOnly)}>
      <div
        ref={node.setNodeRef}
        className={cx("seq", hinted && "drop-on", node.isDragging && "drag-src", selected && "sel")}
        onClick={(e) => onItemClick(e, key)}
      >
        <div
          className="sh"
          data-story-key={key}
          {...node.attributes}
          {...node.listeners}
          tabIndex={0}
          role="button"
          aria-label={`Sequence ${seq.title}, ${seq.cardCount} scenes`}
          onKeyDown={(e) => ctx.keys(e, seq)}
          onDoubleClick={() => ui.openDrawer({ kind: "sequence", id: seq.id })}
        >
          <GripVertical size={12} aria-hidden />
          <button
            type="button"
            className="iconbtn sm"
            aria-label={collapsed ? `Expand ${seq.title}` : `Collapse ${seq.title}`}
            aria-expanded={!collapsed}
            onPointerDown={(e) => e.stopPropagation()}
            onClick={(e) => {
              e.stopPropagation();
              void story.setCollapsed(seq.id, !collapsed).catch(reportError);
            }}
          >
            {collapsed ? <ChevronRight size={12} /> : <ChevronDown size={12} />}
          </button>
          {renaming ? (
            <InlineText
              value={seq.title}
              ariaLabel="Sequence name"
              onCancel={() => ui.setRenaming(null)}
              onCommit={(v) => {
                ui.setRenaming(null);
                void commitItemText(seq, v);
              }}
            />
          ) : (
            <span className="grow truncate" onDoubleClick={(e) => { e.stopPropagation(); if (!ctx.readOnly) ui.setRenaming(seq.id); }}>
              {seq.title}
            </span>
          )}
          <span className="cnt">{seq.cardCount}</span>
        </div>
        {!collapsed && (
          <div ref={body.setNodeRef} className="seq-body">
            <ItemList container={SEQ(seq.id)} items={seq.items} ctx={ctx} />
            {seq.items.length === 0 && <div className="story-empty-hint">Drop scenes here.</div>}
            {!ctx.readOnly && (
              <button type="button" className="story-add" onClick={(e) => { e.stopPropagation(); void newCard(ctx.episodeId, SEQ(seq.id)); }}>
                <Plus size={12} /> Add scene
              </button>
            )}
          </div>
        )}
      </div>
    </ContextMenu>
  );
}

function CardNode({ item, ctx }: { item: Extract<StoryItem, { kind: "card" | "beat" }>; ctx: Ctx }) {
  const node = useItemDnd(item.kind, item.id, ctx.readOnly);
  const key = refKey({ kind: item.kind, id: item.id });
  const selected = useStoryUi((s) => s.selected.includes(key));
  const editing = useStoryUi((s) => s.editingKey === key);
  const ui = useStoryUi.getState();
  const color = item.color ? COLOR_VARS[item.color] : undefined;
  const text = item.kind === "card" ? item.shortDescription : item.text;
  const isBeat = item.kind === "beat";
  const converted = isBeat && item.state === "converted";
  const style: CSSProperties = color ? { borderLeftColor: color } : {};
  return (
    <ContextMenu items={itemMenu(ctx.board, item, ctx.episodeId, ctx.readOnly)}>
      <div
        ref={node.setNodeRef}
        {...(editing ? {} : node.listeners)}
        {...node.attributes}
        data-story-key={key}
        tabIndex={0}
        role="button"
        aria-pressed={selected}
        aria-label={`${isBeat ? "Beat" : "Scene Card"}: ${text || "blank"}${converted ? " (converted)" : ""}`}
        className={cx(isBeat ? "beat" : "sc", selected && "sel", node.isDragging && "drag-src", editing && "editing", converted && "converted")}
        style={style}
        onClick={(e) => onItemClick(e, key)}
        onDoubleClick={(e) => {
          e.stopPropagation();
          ui.openDrawer({ kind: item.kind, id: item.id });
        }}
        onKeyDown={(e) => !editing && ctx.keys(e, item)}
      >
        {editing ? (
          <InlineText
            multiline
            value={text}
            ariaLabel={isBeat ? "Beat text" : "Short description"}
            placeholder={isBeat ? "Beat text" : "What happens in this scene?"}
            onCancel={() => ui.setEditing(null)}
            onCommit={(v) => {
              ui.setEditing(null);
              void commitItemText(item, v.trimEnd());
            }}
          />
        ) : (
          <>
            {text.trim() ? text : <span className="muted"><i>{isBeat ? "Blank beat" : "Blank scene card"}</i></span>}
            <span className="ind">
              {converted && <b title="Converted to a Scene Card">Converted</b>}
              {item.kind === "card" && item.sceneHeading && <b title="has a scene heading">H</b>}
              {item.kind === "card" && item.screenplaySceneId && (
                <span title="Used to create a screenplay scene" aria-label="Used to create a screenplay scene">
                  <FileText size={10} />
                </span>
              )}
              {item.commentCount > 0 && (
                <span title={`${item.commentCount} open comment${item.commentCount === 1 ? "" : "s"}`}>
                  <MessageSquare size={10} /> {item.commentCount}
                </span>
              )}
            </span>
          </>
        )}
      </div>
    </ContextMenu>
  );
}

function ParkingColumn({ ctx }: { ctx: Ctx }) {
  const drop = useContainerDrop(PARKING, false, ctx.readOnly);
  const hinted = !!ctx.dnd.hint && sameContainer(ctx.dnd.hint.target, PARKING);
  return (
    <section ref={drop.setNodeRef} className={cx("parking", hinted && "drop-on")} aria-label="Parking Lot">
      <div className="ah">
        <ParkingSquare size={14} aria-hidden /> Parking Lot
        <span className="chip out" style={{ marginLeft: "auto" }}>{ctx.board.parking.length}</span>
      </div>
      <ItemList container={PARKING} items={ctx.board.parking} ctx={ctx} />
      {ctx.board.parking.length === 0 && (
        <div className="story-empty-hint">Drag cards and beats here to keep them out of the story without deleting them.</div>
      )}
      {!ctx.readOnly && (
        <div className="story-add-row">
          <button type="button" className="story-add" onClick={(e) => { e.stopPropagation(); void newBeat(ctx.episodeId, PARKING); }}>
            <Plus size={12} /> Beat
          </button>
          <button type="button" className="story-add" onClick={(e) => { e.stopPropagation(); void newCard(ctx.episodeId, PARKING); }}>
            <Plus size={12} /> Scene
          </button>
        </div>
      )}
    </section>
  );
}

function UnassignedColumn({ ctx }: { ctx: Ctx }) {
  const drop = useContainerDrop(UNASSIGNED, false, true);
  return (
    <section ref={drop.setNodeRef} className="unassigned-col" aria-label="Unassigned">
      <div className="ah">
        Unassigned <span className="chip out" style={{ marginLeft: "auto" }}>{ctx.board.unassigned.length}</span>
      </div>
      <div className="story-empty-hint" style={{ marginBottom: 6 }}>
        Restored items whose Act or Sequence no longer exists. Drag them back into the story.
      </div>
      <ItemList container={UNASSIGNED} items={ctx.board.unassigned} ctx={ctx} />
    </section>
  );
}

