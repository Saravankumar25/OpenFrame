// Pure geometry for the moodboard canvas (drag, resize, extent). The canvas
// shows these values only while a gesture is in progress; the result is sent
// to Rust as one undoable command (moodboard.move_items / resize_item).

export interface Box {
  id: string;
  x: number;
  y: number;
  w: number;
  h: number;
}

export const TILE_MIN = 40;
export const TILE_MAX = 4000;
export const CANVAS_MAX = 20000;

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

/**
 * Offset every selected box by (dx, dy). The group moves together and stops at
 * the canvas edge as a unit, so relative positions never change.
 */
export function dragBoxes(boxes: Box[], selected: ReadonlySet<string>, dx: number, dy: number): Box[] {
  const moving = boxes.filter((b) => selected.has(b.id));
  if (moving.length === 0) return boxes;
  const minX = Math.min(...moving.map((b) => b.x));
  const minY = Math.min(...moving.map((b) => b.y));
  const maxX = Math.max(...moving.map((b) => b.x));
  const maxY = Math.max(...moving.map((b) => b.y));
  const ddx = clamp(Math.round(dx), -minX, CANVAS_MAX - maxX);
  const ddy = clamp(Math.round(dy), -minY, CANVAS_MAX - maxY);
  return boxes.map((b) => (selected.has(b.id) ? { ...b, x: b.x + ddx, y: b.y + ddy } : b));
}

/** New size while dragging the resize handle; `keepAspect` preserves the ratio (images). */
export function resizeBox(b: Box, dw: number, dh: number, keepAspect: boolean): Box {
  let w = clamp(Math.round(b.w + dw), TILE_MIN, TILE_MAX);
  let h = clamp(Math.round(b.h + dh), TILE_MIN, TILE_MAX);
  if (keepAspect && b.w > 0) {
    const ratio = b.h / b.w;
    h = clamp(Math.round(w * ratio), TILE_MIN, TILE_MAX);
    w = clamp(Math.round(h / ratio), TILE_MIN, TILE_MAX);
  }
  return { ...b, w, h };
}

/** Scrollable canvas size: the content plus room to keep arranging. */
export function canvasExtent(boxes: Box[], pad = 240): { width: number; height: number } {
  let width = 0;
  let height = 0;
  for (const b of boxes) {
    width = Math.max(width, b.x + b.w);
    height = Math.max(height, b.y + b.h + 28);
  }
  return { width: width + pad, height: height + pad };
}

/** The moves to persist after a drag: only boxes whose position changed. */
export function changedMoves(before: Box[], after: Box[]): { id: string; x: number; y: number }[] {
  const prev = new Map(before.map((b) => [b.id, b]));
  return after.filter((b) => {
    const p = prev.get(b.id);
    return p && (p.x !== b.x || p.y !== b.y);
  }).map((b) => ({ id: b.id, x: b.x, y: b.y }));
}
