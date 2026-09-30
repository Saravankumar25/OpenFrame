// Scene list (mock 104 left pane): derived numbers, headings and concise
// status chips; filterable (FSD-BREAKDOWN-015).

import { useRef, type KeyboardEvent } from "react";
import { Check, History, ListFilter, TriangleAlert } from "lucide-react";
import type { BreakdownSceneRow } from "../../ipc/generated/BreakdownSceneRow";
import { Chip, Menu } from "../../design-system";

export type SceneFilter = "all" | "needsReview" | "needsBreakdown" | "suggested" | "complete" | "incomplete";

export const FILTER_LABELS: Record<SceneFilter, string> = {
  all: "All scenes",
  needsReview: "Needs review",
  needsBreakdown: "Needs breakdown",
  suggested: "Has suggestions",
  complete: "Complete",
  incomplete: "Not complete",
};

export function filterScenes(rows: BreakdownSceneRow[], f: SceneFilter): BreakdownSceneRow[] {
  switch (f) {
    case "needsReview":
      return rows.filter((r) => r.needsReview);
    case "needsBreakdown":
      return rows.filter((r) => r.needsBreakdown);
    case "suggested":
      return rows.filter((r) => r.suggestedCount > 0);
    case "complete":
      return rows.filter((r) => r.complete);
    case "incomplete":
      return rows.filter((r) => !r.complete);
    default:
      return rows;
  }
}

export function SceneList({ rows, total, selected, filter, onFilter, onSelect, reviewCount, historicalCount, onOpenReview, onOpenHistory }: {
  rows: BreakdownSceneRow[];
  total: number;
  selected: string | null;
  filter: SceneFilter;
  onFilter: (f: SceneFilter) => void;
  onSelect: (sceneId: string) => void;
  reviewCount: number;
  historicalCount: number;
  onOpenReview: () => void;
  onOpenHistory: () => void;
}) {
  const listRef = useRef<HTMLDivElement>(null);
  const onKey = (e: KeyboardEvent) => {
    if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
    e.preventDefault();
    const i = rows.findIndex((r) => r.sceneId === selected);
    const next = rows[Math.max(0, Math.min(rows.length - 1, i + (e.key === "ArrowDown" ? 1 : -1)))];
    if (next) {
      onSelect(next.sceneId);
      listRef.current?.querySelector<HTMLButtonElement>(`[data-scene="${next.sceneId}"]`)?.focus();
    }
  };
  return (
    <nav className="sp-nav" aria-label="Scenes">
      <div className="row" style={{ marginBottom: 6 }}>
        <span className="h4" style={{ margin: 0 }}>Scenes</span>
        <Chip tone="out">{total}</Chip>
        <span className="grow" />
        <Menu
          align="end"
          trigger={
            <button type="button" className="chip" aria-label={`Filter scenes: ${FILTER_LABELS[filter]}`} style={{ border: 0, cursor: "pointer" }}>
              <ListFilter size={12} aria-hidden /> {filter === "all" ? "Filter" : FILTER_LABELS[filter]}
            </button>
          }
          items={(Object.keys(FILTER_LABELS) as SceneFilter[]).map((f) => ({ label: `${f === filter ? "✓ " : ""}${FILTER_LABELS[f]}`, onSelect: () => onFilter(f) }))}
        />
      </div>
      <div ref={listRef} role="listbox" aria-label="Scenes in the Production Source" onKeyDown={onKey}>
        {rows.map((r) => (
          <button
            key={r.sceneId}
            type="button"
            role="option"
            aria-selected={r.sceneId === selected}
            data-scene={r.sceneId}
            tabIndex={r.sceneId === selected || (!selected && r === rows[0]) ? 0 : -1}
            className={`sni${r.sceneId === selected ? " on" : ""}${r.omitted ? " omitted" : ""}`}
            onClick={() => onSelect(r.sceneId)}
          >
            <b>{r.number}</b>
            <span style={{ flex: 1 }}>{r.omitted ? `OMITTED — ${r.heading}` : r.heading || "(no heading)"}</span>
            {(r.complete || r.suggestedCount > 0 || r.needsReview || r.needsBreakdown) && (
              <span className="chips">
                {r.complete && (
                  <Chip tone="g" title="Breakdown complete">
                    <Check size={11} aria-hidden /> <span className="sr-only">Complete</span>
                  </Chip>
                )}
                {r.suggestedCount > 0 && <Chip tone="a">{r.suggestedCount} suggested</Chip>}
                {r.needsReview && <Chip tone="y">Needs review</Chip>}
                {r.needsBreakdown && <Chip tone="r">Needs breakdown</Chip>}
              </span>
            )}
          </button>
        ))}
        {rows.length === 0 && <div className="sm muted" style={{ padding: 8 }}>No scenes match this filter.</div>}
      </div>
      {(reviewCount > 0 || historicalCount > 0) && (
        <div className="navfoot">
          {reviewCount > 0 && (
            <button type="button" className="sni" onClick={onOpenReview}>
              <TriangleAlert size={13} aria-hidden /> Needs Review ({reviewCount})
            </button>
          )}
          {historicalCount > 0 && (
            <button type="button" className="sni" onClick={onOpenHistory}>
              <History size={13} aria-hidden /> Removed scenes ({historicalCount})
            </button>
          )}
        </div>
      )}
    </nav>
  );
}
