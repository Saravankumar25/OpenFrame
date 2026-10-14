// Filter bar for the stripboard and the list (spec section 27, SCHED-006).
// Filters are view state: they only decide which strips are shown or dimmed.

import { useMemo } from "react";
import { FilterX } from "lucide-react";
import { Button, Select } from "../../../design-system";
import type { ScheduleView } from "../../../api/schedule";
import { EMPTY_FILTER, filterOptions, isFilterActive, type ScheduleFilter } from "./scheduleFilter";

const LENGTHS = [
  { value: "", label: "Any length" },
  { value: "short", label: "Up to 1 page" },
  { value: "mid", label: "1 to 3 pages" },
  { value: "long", label: "Over 3 pages" },
] as const;

type LengthKey = (typeof LENGTHS)[number]["value"];

function lengthOf(f: ScheduleFilter): LengthKey {
  if (f.minEighths === null && f.maxEighths === 8) return "short";
  if (f.minEighths === 9 && f.maxEighths === 24) return "mid";
  if (f.minEighths === 25 && f.maxEighths === null) return "long";
  return "";
}

function withLength(f: ScheduleFilter, k: LengthKey): ScheduleFilter {
  switch (k) {
    case "short":
      return { ...f, minEighths: null, maxEighths: 8 };
    case "mid":
      return { ...f, minEighths: 9, maxEighths: 24 };
    case "long":
      return { ...f, minEighths: 25, maxEighths: null };
    default:
      return { ...f, minEighths: null, maxEighths: null };
  }
}

export function ScheduleFilterBar({
  view,
  filter,
  onChange,
  shown,
  total,
}: {
  view: ScheduleView;
  filter: ScheduleFilter;
  onChange: (f: ScheduleFilter) => void;
  shown: number;
  total: number;
}) {
  const opts = useMemo(() => filterOptions(view), [view]);
  const any = (label: string) => [{ value: "", label }];
  return (
    <div className="row wrap gap4 of-schedule-filters" role="group" aria-label="Filter scenes">
      <Select
        ariaLabel="Filter by location"
        value={filter.locations[0] ?? ""}
        onChange={(v) => onChange({ ...filter, locations: v ? [v] : [] })}
        options={[...any("All locations"), ...opts.locations]}
      />
      <Select
        ariaLabel="Filter by cast"
        value={filter.cast[0] ?? ""}
        onChange={(v) => onChange({ ...filter, cast: v ? [v] : [] })}
        options={[...any("All cast"), ...opts.cast]}
      />
      <Select
        ariaLabel="Filter by day or night"
        value={filter.dayNight}
        onChange={(v) => onChange({ ...filter, dayNight: v })}
        options={[
          { value: "", label: "Day and night" },
          { value: "D", label: "Day scenes" },
          { value: "N", label: "Night scenes" },
        ]}
      />
      <Select ariaLabel="Filter by scene length" value={lengthOf(filter)} onChange={(v) => onChange(withLength(filter, v))} options={LENGTHS} />
      <Select
        ariaLabel="Filter by scheduled state"
        value={filter.scheduled}
        onChange={(v) => onChange({ ...filter, scheduled: v })}
        options={[
          { value: "any", label: "Scheduled and unscheduled" },
          { value: "scheduled", label: "Scheduled" },
          { value: "unscheduled", label: "Unscheduled" },
        ]}
      />
      <Select
        ariaLabel="Filter by breakdown state"
        value={filter.breakdown}
        onChange={(v) => onChange({ ...filter, breakdown: v })}
        options={[
          { value: "any", label: "Any breakdown" },
          { value: "complete", label: "Breakdown complete" },
          { value: "incomplete", label: "Breakdown incomplete" },
        ]}
      />
      <Select
        ariaLabel="Filter by shooting day"
        value={filter.days[0] ?? ""}
        onChange={(v) => onChange({ ...filter, days: v ? [v] : [] })}
        options={[...any("All days"), ...opts.days]}
      />
      <Select
        ariaLabel="Filter by script revision"
        value={filter.revision}
        onChange={(v) => onChange({ ...filter, revision: v })}
        options={[
          { value: "any", label: "Any revision state" },
          { value: "changed", label: "Changed in the script" },
          { value: "current", label: "Unchanged" },
        ]}
      />
      {isFilterActive(filter) && (
        <>
          <span className="sm muted" aria-live="polite">
            Showing {shown} of {total} scenes
          </span>
          <Button size="xs" icon={<FilterX size={13} />} onClick={() => onChange(EMPTY_FILTER)}>
            Clear filters
          </Button>
        </>
      )}
    </div>
  );
}
