// Script reading pane (FSD §96: "A script reading pane is available while
// tagging"). Read-only: the screenplay is never edited from Breakdown.
// Selecting words in one line offers "Tag “…” as…" with the 11 categories.

import { useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import type { BreakdownSceneDetail } from "../../ipc/generated/BreakdownSceneDetail";
import type { BreakdownCategory } from "../../ipc/generated/BreakdownCategory";
import type { BreakdownSourceAnchor } from "../../ipc/generated/BreakdownSourceAnchor";
import { CategoryMark, useCategoryLook } from "./categoryStyle";
import { CATEGORIES } from "../../api/production";
import { segmentText, selectionInLine, suggestName, type Mark, type TextSelection } from "./highlight";
import type { AddInitial } from "./dialogs";
import { readZoom, writeZoom } from "./filterPref";

const LINE_CLASS: Record<string, string> = {
  scene_heading: "sp-h",
  action: "sp-a",
  character: "sp-c",
  dialogue: "sp-d",
  parenthetical: "sp-p",
  transition: "sp-t",
  shot: "sp-sh",
  note: "note",
};

export function ScriptPane({ detail, canEdit, onTag, onSelection, jump = null }: {
  detail: BreakdownSceneDetail;
  canEdit: boolean;
  onTag: (initial: AddInitial) => void;
  /** Current taggable selection (for the keyboard-reachable "Tag Selection" action). */
  onSelection: (sel: TextSelection | null) => void;
  /** Source phrase to show ("Show in script"), already resolved against the current text. */
  jump?: BreakdownSourceAnchor | null;
}) {
  const wrapRef = useRef<HTMLDivElement>(null);
  // Ctrl+wheel and touchpad pinch zoom the script page only (owner item 14).
  const [zoom, setZoom] = useState(readZoom);
  useEffect(() => {
    const el = wrapRef.current;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      if (!e.ctrlKey) return;
      e.preventDefault();
      setZoom((z) => writeZoom(z * Math.exp(-e.deltaY / 400)));
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  }, []);
  const look = useCategoryLook();
  const [menu, setMenu] = useState<{ sel: TextSelection; x: number; y: number } | null>(null);

  const marksByLine = useMemo(() => {
    const m = new Map<string, Mark[]>();
    for (const g of detail.groups) {
      for (const [list, kind] of [[g.confirmed, "tag"], [g.suggestions, "suggestion"]] as const) {
        for (const e of list) {
          if (!e.spanElementId || e.spanStart === null || e.spanEnd === null || e.archived) continue;
          const arr = m.get(e.spanElementId) ?? [];
          arr.push({ start: e.spanStart, end: e.spanEnd, kind, category: e.category, label: `${e.category}: ${e.name}${kind === "suggestion" ? " (suggested)" : ""}` });
          m.set(e.spanElementId, arr);
        }
      }
    }
    if (jump?.elementId && jump.start !== null && jump.end !== null) {
      const arr = m.get(jump.elementId) ?? [];
      arr.push({ start: jump.start, end: jump.end, kind: "jump", label: `Source phrase: ${jump.phrase ?? ""}` });
      m.set(jump.elementId, arr);
    }
    return m;
  }, [detail, jump]);

  // Bring the source phrase into view when "Show in script" is used.
  useEffect(() => {
    if (!jump?.elementId) return;
    const id = jump.elementId;
    const line = Array.from(wrapRef.current?.querySelectorAll<HTMLElement>("[data-element-id]") ?? []).find((n) => n.dataset.elementId === id);
    const el = line?.querySelector<HTMLElement>("mark.jump");
    el?.scrollIntoView?.({ block: "center", behavior: "smooth" });
  }, [jump]);

  useEffect(() => {
    setMenu(null);
    onSelection(null);
  }, [detail.sceneId, onSelection]);

  useEffect(() => {
    if (!menu) return;
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && setMenu(null);
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [menu]);

  const onMouseUp = () => {
    const sel = selectionInLine(window.getSelection());
    onSelection(sel);
    if (!sel || !canEdit || !detail.inSource || !wrapRef.current) {
      setMenu(null);
      return;
    }
    const range = window.getSelection()!.getRangeAt(0).getBoundingClientRect();
    const box = wrapRef.current.getBoundingClientRect();
    setMenu({ sel, x: range.left - box.left + wrapRef.current.scrollLeft, y: range.bottom - box.top + wrapRef.current.scrollTop + 6 });
  };

  const tagAs = (category: BreakdownCategory) => {
    if (!menu) return;
    const { sel } = menu;
    setMenu(null);
    window.getSelection()?.removeAllRanges();
    onSelection(null);
    onTag({ category, name: suggestName(sel.text), span: { elementId: sel.elementId, start: sel.start, end: sel.end } });
  };

  const lines = detail.script.filter((l, i) => !(i === 0 && l.elementType === "scene_heading" && l.text.trim() === detail.heading.trim()));

  return (
    <div className="sp-wrap" ref={wrapRef} style={{ ["--bd-zoom" as string]: zoom } as CSSProperties} onMouseDown={(e) => menu && !(e.target as HTMLElement).closest(".tagmenu") && setMenu(null)}>
      <div className="sp-page" onMouseUp={onMouseUp} aria-label={`Script, Scene ${detail.number ?? ""}`}>
        {detail.number && <div className="pn">{detail.number}.</div>}
        <div className="sp-h">
          {detail.number && <span className="sn">{detail.number}</span>}
          {detail.heading || "(no heading)"}
          {detail.number && <span className="sn r">{detail.number}</span>}
        </div>
        {lines.map((l) => (
          <div key={l.elementId} data-element-id={l.elementId} className={LINE_CLASS[l.elementType] ?? "sp-a"}>
            {segmentText(l.text, marksByLine.get(l.elementId) ?? []).map((s, i) =>
              s.mark ? (
                <mark
                  key={i}
                  className={s.mark.kind === "tag" ? "hl" : s.mark.kind === "jump" ? "hl jump" : "sugmark"}
                  title={s.mark.label}
                  data-category={s.mark.category}
                  style={s.mark.category ? ({ "--cat": look(s.mark.category as BreakdownCategory).hex } as CSSProperties) : undefined}
                >
                  {s.text}
                </mark>
              ) : (
                <span key={i}>{s.text}</span>
              ),
            )}
          </div>
        ))}
        {lines.length === 0 && <div className="sp-a muted">This scene has no text yet.</div>}
      </div>
      {menu && (
        <div className="tagmenu" role="menu" aria-label={`Tag “${menu.sel.text}” as`} style={{ left: menu.x, top: menu.y }}>
          <div className="mh">Tag “{menu.sel.text.length > 28 ? `${menu.sel.text.slice(0, 28)}…` : menu.sel.text}” as…</div>
          {CATEGORIES.map((c, i) => (
            <button key={c} type="button" role="menuitem" autoFocus={i === 0} onClick={() => tagAs(c)}>
              <CategoryMark look={look(c)} />
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
