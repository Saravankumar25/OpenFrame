// Stripboard strip and break-marker rows (UX §3.29, mock 130). Colour follows
// INT/EXT × DAY/NIGHT (.xd .xn .id .in); state is never colour-only: flags and
// missing values also carry text.

import type { KeyboardEvent, MouseEvent, ReactNode } from "react";
import type { ScheduleMarkerDto, ScheduleStripDto } from "../../../api/schedule";
import { formatMinutes } from "../../../api/schedule";
import "./schedule-ui.css";

export function stripAriaLabel(s: ScheduleStripDto): string {
  const parts = [
    s.number !== null ? `Scene ${s.number}` : "Scene (not in script)",
    s.ieLabel,
    s.locationName ?? "location missing",
    s.synopsis || s.heading,
    `${s.pagesLabel} pages`,
  ];
  if (s.estimatedMinutes !== null) parts.push(`estimated ${formatMinutes(s.estimatedMinutes)}`);
  if (s.sourceState !== "Current") parts.push(`script: ${s.changeKinds.join(", ") || s.sourceState}`);
  return parts.join(", ");
}

export function StripRow({
  strip: s,
  warn,
  selected,
  onOpen,
  menu,
  dragHandleProps,
  compact,
}: {
  strip: ScheduleStripDto;
  warn?: boolean;
  selected?: boolean;
  onOpen?: () => void;
  /** Trailing menu trigger (non-drag alternative for moves). */
  menu?: ReactNode;
  dragHandleProps?: Record<string, unknown>;
  compact?: boolean;
}) {
  const onKey = (e: KeyboardEvent) => {
    if ((e.key === "Enter" || e.key === " ") && onOpen && e.target === e.currentTarget) {
      e.preventDefault();
      onOpen();
    }
  };
  const flagged = s.sourceState !== "Current";
  return (
    <div
      className={`strip ${s.stripClass}${warn ? " warn" : ""}${selected ? " of-sel" : ""}${s.sourceState === "Removed" ? " of-removed" : ""}`}
      role="button"
      tabIndex={0}
      aria-label={stripAriaLabel(s)}
      onClick={(e: MouseEvent) => {
        // React bubbles clicks from portalled content (the strip's own menu)
        // through this element: only clicks physically inside the strip open it.
        if (!e.currentTarget.contains(e.target as Node) || (e.target as HTMLElement).closest("[data-no-open]")) return;
        onOpen?.();
      }}
      onKeyDown={onKey}
      {...dragHandleProps}
    >
      <span className="no">{s.number ?? "—"}</span>
      <span className="ie">{s.ieLabel}</span>
      <span className="lc">{s.locationName ?? <span className="need">No location</span>}</span>
      {!compact && <span className="syn">{s.synopsis || s.heading}</span>}
      {flagged && (
        <span className={`chip ${s.sourceState === "Removed" ? "r" : s.sourceState === "New" ? "b" : "y"}`} title={s.changeKinds.join(", ")}>
          {s.sourceState === "Removed" ? "Removed from script" : s.sourceState === "New" ? "New scene" : "Script changed"}
        </span>
      )}
      <span className="cast" aria-hidden>
        {s.cast.slice(0, 4).map((c) => (
          <i key={c.key} title={c.actor ? `${c.actor} (${c.character})` : c.character}>
            {c.initials}
          </i>
        ))}
        {s.cast.length > 4 && <i>+{s.cast.length - 4}</i>}
      </span>
      <span className="est" title="Estimated shooting time">
        {s.estimatedMinutes !== null ? formatMinutes(s.estimatedMinutes) : ""}
      </span>
      <span className="pg" title={s.pagesOverridden ? "Page count entered manually" : "Page count from the screenplay"}>
        {s.pagesLabel}
        {s.pagesOverridden ? "*" : ""}
      </span>
      {menu && (
        <span data-no-open className="of-strip-menu">
          {menu}
        </span>
      )}
    </div>
  );
}

export function markerText(m: ScheduleMarkerDto): string {
  const base = m.label.toUpperCase();
  const time = m.atTime ? ` — ${m.atTime}` : "";
  const dur = m.durationMinutes ? ` (${formatMinutes(m.durationMinutes)})` : "";
  return `${base}${time}${dur}`;
}

export function MarkerRow({ marker, menu, dragHandleProps, onOpen }: {
  marker: ScheduleMarkerDto;
  menu?: ReactNode;
  dragHandleProps?: Record<string, unknown>;
  onOpen?: () => void;
}) {
  return (
    <div
      className="strip mk"
      role="button"
      tabIndex={0}
      aria-label={`${marker.markerType === "Custom" ? "Note" : "Break"}: ${markerText(marker)}`}
      onClick={(e) => {
        if (!e.currentTarget.contains(e.target as Node) || (e.target as HTMLElement).closest("[data-no-open]")) return;
        onOpen?.();
      }}
      onKeyDown={(e) => {
        if ((e.key === "Enter" || e.key === " ") && e.target === e.currentTarget) {
          e.preventDefault();
          onOpen?.();
        }
      }}
      {...dragHandleProps}
    >
      <span className="grow" style={{ textAlign: "center" }}>
        {markerText(marker)}
      </span>
      {menu && (
        <span data-no-open className="of-strip-menu">
          {menu}
        </span>
      )}
    </div>
  );
}
