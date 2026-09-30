// Activity History (FSD §106, §157; UX §3.36; mock 152).
// "What important actions have happened in this project? Read-only."
// Entries come from the command pipeline (never per keystroke); "Open" appears
// only while the affected object still exists.

import { useState } from "react";
import { ArrowRight, History } from "lucide-react";
import { useOp } from "../../ipc/query";
import type { ActivityEntry } from "../../ipc/generated/ActivityEntry";
import { Button, EmptyState, PageHeader, Segmented, Skeleton } from "../../design-system";
import { navigateTo } from "../../app/navigate";
import { dayAndTime } from "../../app/home/format";

export type ActivityFilter = "all" | "drafts" | "story" | "production" | "exchange";

const FILTERS: Record<Exclude<ActivityFilter, "all">, RegExp> = {
  drafts: /^screenplay\./,
  story: /^(story|vault)\./,
  production: /^(production|breakdown|catalog|location|cast|crew|visual|moodboard|storyboard|shot|schedule|callsheet|call_sheet|budget|report)/,
  exchange: /^(exchange|package|import|export)/,
};

export function matchesFilter(entry: ActivityEntry, filter: ActivityFilter): boolean {
  return filter === "all" || FILTERS[filter].test(entry.action);
}

/** "Renamed file to “X”" → "renamed file to “X”" so it reads after the actor's name. */
export function sentence(entry: ActivityEntry): string {
  const s = entry.summary;
  return /^[A-Z][a-z]+ed\b/.test(s) ? s.charAt(0).toLowerCase() + s.slice(1) : s;
}

const PAGE = 200;

export default function ActivityHistory() {
  const [filter, setFilter] = useState<ActivityFilter>("all");
  const [limit, setLimit] = useState(PAGE);
  const activity = useOp<ActivityEntry[]>("history.activity", { limit }, ["*"], { placeholderData: (prev) => prev });
  const all = activity.data ?? [];
  const shown = all.filter((e) => matchesFilter(e, filter));
  return (
    <div className="content" style={{ overflow: "auto" }}>
      <PageHeader
        title="Activity History"
        sub="What important actions have happened in this project? Read-only."
        actions={
          <Segmented
            ariaLabel="Show activity"
            value={filter}
            onChange={setFilter}
            options={[
              { value: "all", label: "All" },
              { value: "drafts", label: "Drafts" },
              { value: "story", label: "Story" },
              { value: "production", label: "Production" },
              { value: "exchange", label: "Exchange" },
            ]}
          />
        }
      />
      {activity.isLoading ? (
        <div className="col"><Skeleton h={36} /><Skeleton h={36} /><Skeleton h={36} /></div>
      ) : shown.length === 0 ? (
        <div className="card" style={{ padding: 8 }}>
          <EmptyState icon={<History size={30} />} title="No major activity yet.">
            {filter === "all" ? "Important actions — drafts, moves, locks, production changes — are listed here as you work. Ordinary typing is never logged." : "Nothing of this kind has happened yet."}
          </EmptyState>
        </div>
      ) : (
        <ol className="card" aria-label="Activity" style={{ listStyle: "none", margin: 0, padding: 0 }}>
          {shown.map((e) => (
            <li key={e.id} className="li">
              <time className="xs muted" dateTime={new Date(e.at).toISOString()} style={{ width: 110, flex: "none" }}>{dayAndTime(e.at)}</time>
              <div className="grow sm" style={{ minWidth: 0 }}>
                {e.actorName ? <><b>{e.actorName}</b> {sentence(e)}</> : e.summary}
              </div>
              {e.nav != null && (
                <Button size="xs" icon={<ArrowRight size={12} />} onClick={() => navigateTo(e.nav)} aria-label={`Open: ${e.summary}`}>Open</Button>
              )}
            </li>
          ))}
        </ol>
      )}
      {all.length >= limit && (
        <div className="row" style={{ justifyContent: "center", marginTop: 12 }}>
          <Button size="sm" onClick={() => setLimit((l) => Math.min(l + PAGE, 1000))} disabled={limit >= 1000 || activity.isFetching}>Show older activity</Button>
        </div>
      )}
    </div>
  );
}
