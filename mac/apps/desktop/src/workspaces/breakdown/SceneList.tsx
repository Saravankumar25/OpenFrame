// Scene list (mock 104 left pane): derived numbers, headings and concise
// status chips. Filters run in Rust (`breakdown.scenes`), so they stay fast on
// large projects (performance budget P15); counts cover the whole source.

import { useRef, type KeyboardEvent } from "react";
import { Check, History, ListFilter, TriangleAlert } from "lucide-react";
import type { BreakdownSceneRow } from "../../ipc/generated/BreakdownSceneRow";
import type { BreakdownSceneFilter } from "../../ipc/generated/BreakdownSceneFilter";
import type { BreakdownFilterCounts } from "../../ipc/generated/BreakdownFilterCounts";
import type { BreakdownCategory } from "../../ipc/generated/BreakdownCategory";
import { Chip, Menu } from "../../design-system";
import { CATEGORIES } from "../../api/production";
import { CategoryMark, useCategoryLook } from "./categoryStyle";

export const FILTER_LABELS: Record<BreakdownSceneFilter, string> = {
  all: "All scenes",
  unreviewed: "Only unreviewed",
  suggested: "Only suggested",
  missing: "Only missing a breakdown",
  needsReview: "Script changed",
  complete: "Complete",
  incomplete: "Not complete",
};

const COUNT_KEY: Record<BreakdownSceneFilter, keyof BreakdownFilterCounts> = {
  all: "all",
  unreviewed: "unreviewed",
  suggested: "suggested",
  missing: "missing",
  needsReview: "needsReview",
  complete: "complete",
  incomplete: "incomplete",
};

export function SceneList({ rows, total, counts, selected, filter, category, onFilter, onCategory, onSelect, reviewCount, historicalCount, onOpenReview, onOpenHistory }: {
  rows: BreakdownSceneRow[];
  total: number;
  counts: BreakdownFilterCounts | null;
  selected: string | null;
  filter: BreakdownSceneFilter;
  category: BreakdownCategory | null;
  onFilter: (f: BreakdownSceneFilter) => void;
  onCategory: (c: BreakdownCategory | null) => void;
  onSelect: (sceneId: string) => void;
  reviewCount: number;
  historicalCount: number;
  onOpenReview: () => void;
  onOpenHistory: () => void;
}) {
  const look = useCategoryLook();
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
        <Chip tone="out">{filter === "all" && !category ? total : `${rows.length} of ${total}`}</Chip>
        <span className="grow" />
        <Menu
          align="end"
          trigger={
            <button type="button" className="chip" aria-label={`Filter scenes: ${FILTER_LABELS[filter]}${category ? `, ${category}` : ""}`} style={{ border: 0, cursor: "pointer" }}>
              <ListFilter size={12} aria-hidden /> {filter === "all" && !category ? "Filter" : [filter === "all" ? null : FILTER_LABELS[filter], category].filter(Boolean).join(" · ")}
            </button>
          }
          items={[
            ...(Object.keys(FILTER_LABELS) as BreakdownSceneFilter[]).map((f) => ({
              label: `${f === filter ? "✓ " : ""}${FILTER_LABELS[f]}${counts ? ` (${counts[COUNT_KEY[f]]})` : ""}`,
              onSelect: () => onFilter(f),
            })),
            { label: `${category === null ? "✓ " : ""}Any category`, separatorBefore: true, onSelect: () => onCategory(null) },
            ...CATEGORIES.map((c) => ({ label: `${c === category ? "✓ " : ""}${c}`, onSelect: () => onCategory(c) })),
          ]}
        />
      </div>
      {counts && (
        <div className="xs muted" style={{ margin: "0 0 6px 2px" }} aria-live="polite">
          {counts.complete} of {counts.all} complete
        </div>
      )}
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
            <span style={{ flex: 1 }}>{r.omitted ? `OMITTED: ${r.heading}` : r.heading || "(no heading)"}</span>
            {(r.complete || r.suggestedCount > 0 || r.needsReview || r.needsBreakdown || r.categories.length > 0) && (
              <span className="chips">
                {r.categories.map((c) => (
                  <CategoryMark key={c} look={look(c)} label={false} size={11} />
                ))}
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
