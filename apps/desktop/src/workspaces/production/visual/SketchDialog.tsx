// "Draw / Sketch panel" (UX §3.27, mock 125): basic sketching only — pen,
// eraser, three ink colours, undo. The drawing is saved as a PNG through Rust
// (storyboard.add_panel / set_panel_visual with `dataBase64`).

import { useEffect, useRef, useState, type PointerEvent as RPointerEvent } from "react";
import { Eraser, PenLine } from "lucide-react";
import { Button, Dialog, Segmented } from "../../../design-system";

export interface Stroke {
  color: string;
  width: number;
  erase: boolean;
  points: [number, number][];
}

const W = 960;
const H = 540;
const INKS = [
  { color: "#111111", label: "Black" },
  { color: "#c8443a", label: "Red" },
  { color: "#3b6fd4", label: "Blue" },
];

/** Paint strokes onto a 2D context (white paper; the eraser paints paper). */
export function paint(ctx: CanvasRenderingContext2D, strokes: Stroke[]): void {
  ctx.fillStyle = "#ffffff";
  ctx.fillRect(0, 0, W, H);
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  for (const s of strokes) {
    ctx.strokeStyle = s.erase ? "#ffffff" : s.color;
    ctx.lineWidth = s.width;
    ctx.beginPath();
    s.points.forEach(([x, y], i) => (i === 0 ? ctx.moveTo(x, y) : ctx.lineTo(x, y)));
    if (s.points.length === 1) ctx.lineTo(s.points[0][0] + 0.1, s.points[0][1]);
    ctx.stroke();
  }
}

export function SketchDialog({
  open,
  onOpenChange,
  onSave,
  saving,
}: {
  open: boolean;
  onOpenChange: (v: boolean) => void;
  /** Receives the PNG as base64 (no data: prefix). */
  onSave: (pngBase64: string) => void;
  saving?: boolean;
}) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const [strokes, setStrokes] = useState<Stroke[]>([]);
  const drawing = useRef<Stroke | null>(null);
  const [tool, setTool] = useState<"pen" | "eraser">("pen");
  const [ink, setInk] = useState(INKS[0].color);

  useEffect(() => {
    if (!open) {
      setStrokes([]);
      drawing.current = null;
    }
  }, [open]);

  useEffect(() => {
    const ctx = canvasRef.current?.getContext("2d");
    if (ctx) paint(ctx, drawing.current ? [...strokes, drawing.current] : strokes);
  }, [strokes, open]);

  const at = (e: RPointerEvent<HTMLCanvasElement>): [number, number] => {
    const r = e.currentTarget.getBoundingClientRect();
    return [((e.clientX - r.left) / r.width) * W, ((e.clientY - r.top) / r.height) * H];
  };
  const down = (e: RPointerEvent<HTMLCanvasElement>) => {
    e.currentTarget.setPointerCapture(e.pointerId);
    drawing.current = { color: ink, width: tool === "eraser" ? 28 : 4, erase: tool === "eraser", points: [at(e)] };
    const ctx = e.currentTarget.getContext("2d");
    if (ctx) paint(ctx, [...strokes, drawing.current]);
  };
  const move = (e: RPointerEvent<HTMLCanvasElement>) => {
    if (!drawing.current) return;
    drawing.current.points.push(at(e));
    const ctx = e.currentTarget.getContext("2d");
    if (ctx) paint(ctx, [...strokes, drawing.current]);
  };
  const up = () => {
    if (!drawing.current) return;
    const s = drawing.current;
    drawing.current = null;
    setStrokes((prev) => [...prev, s]);
  };

  const save = () => {
    const c = canvasRef.current;
    if (!c) return;
    const ctx = c.getContext("2d");
    if (ctx) paint(ctx, strokes);
    const url = c.toDataURL("image/png");
    onSave(url.slice(url.indexOf("base64,") + 7));
  };

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title="Draw / Sketch panel"
      size="md"
      footer={
        <>
          <Button onClick={() => onOpenChange(false)}>Cancel</Button>
          <Button variant="primary" disabled={strokes.length === 0 || saving} onClick={save}>
            Save Panel
          </Button>
        </>
      }
    >
      <div className="row" style={{ marginBottom: 8 }}>
        <Segmented
          ariaLabel="Drawing tool"
          value={tool}
          onChange={setTool}
          options={[
            { value: "pen", label: <><PenLine size={13} /> Pen</> },
            { value: "eraser", label: <><Eraser size={13} /> Eraser</> },
          ]}
        />
        <div className="row gap4" role="radiogroup" aria-label="Ink colour">
          {INKS.map((i) => (
            <button
              key={i.color}
              type="button"
              role="radio"
              aria-checked={ink === i.color}
              aria-label={i.label}
              className="sw"
              onClick={() => {
                setInk(i.color);
                setTool("pen");
              }}
              style={{
                background: i.color,
                width: 18,
                height: 18,
                border: 0,
                outline: ink === i.color ? "2px solid var(--blue)" : "none",
                outlineOffset: 1,
              }}
            />
          ))}
        </div>
        <Button size="xs" disabled={strokes.length === 0} onClick={() => setStrokes((s) => s.slice(0, -1))}>
          Undo
        </Button>
        <Button size="xs" variant="ghost" disabled={strokes.length === 0} onClick={() => setStrokes([])}>
          Clear
        </Button>
      </div>
      <canvas
        ref={canvasRef}
        width={W}
        height={H}
        aria-label="Sketch area"
        role="img"
        onPointerDown={down}
        onPointerMove={move}
        onPointerUp={up}
        onPointerCancel={up}
        style={{
          width: "100%",
          aspectRatio: "16 / 9",
          borderRadius: 8,
          border: "1px solid var(--line2)",
          touchAction: "none",
          cursor: tool === "eraser" ? "cell" : "crosshair",
          background: "#fff",
          display: "block",
        }}
      />
      <div className="hint" style={{ marginTop: 6 }}>
        Basic sketching only. For polished art, import an image made elsewhere.
      </div>
    </Dialog>
  );
}
