// Production › Daily View (FSD §57, FSD-PROD-022; UX §3.30; mock 140).
// A derived, mostly read-only view of one shooting day. Every item links to
// its source workspace; nothing here edits scenes, cast or locations.

import { CalendarDays } from "lucide-react";
import { Banner, Button, Chip, EmptyState, PageHeader, Select, Skeleton } from "../../../design-system";
import { useOp } from "../../../ipc/query";
import { useNav } from "../../../app/stores";
import type { ScheduleDailyView } from "../../../api/schedule";
import { SCHEDULE_TABLES, callSheetStatusLabel, callSheetTone, formatMinutes } from "../../../api/schedule";
import { openCallSheetForDay } from "../../call-sheets/shared/open-call-sheet";
import { markerText } from "../../call-sheets/shared/Strip";

export const tab = { id: "daily", label: "Daily View", order: 80 };

export default function DailyTab() {
  const route = useNav((s) => s.route);
  const go = useNav((s) => s.go);
  const dayId = route.params?.dayId;
  const q = useOp<ScheduleDailyView>("schedule.daily", { dayId: dayId ?? null }, SCHEDULE_TABLES);
  const pick = (id: string) => go({ workspace: "production", sub: "daily", params: { dayId: id } });

  if (q.isLoading || !q.data) {
    return <div>{q.error ? <Banner tone="err">{q.error.message}</Banner> : <Skeleton h={28} w={260} />}</div>;
  }
  const v = q.data;
  const d = v.day;
  if (!v.hasSchedule || !d) {
    return (
      <div className="of-tab">
        <EmptyState
          icon={<CalendarDays size={40} />}
          title="Select or create a shooting day."
          actions={<Button onClick={() => go({ workspace: "production", sub: "schedule" })}>Open Schedule</Button>}
        >
          The daily view shows everything you need to know for one shooting day, derived from the schedule.
        </EmptyState>
      </div>
    );
  }
  const s = d.summary;
  const openIssues = d.warnings.filter((w) => !w.decision);
  const calls = d.cast.filter((c) => c.callTime).map((c) => c.callTime);
  return (
    <div>
      <PageHeader
        title={d.isToday ? `Today: ${d.label}` : d.label}
        sub={`${d.dateLabel ?? "No date yet"} · What do I need to know for this day?`}
        badge={d.isOffDay ? <Chip>Off day</Chip> : undefined}
        actions={
          <Select
            ariaLabel="Shooting day"
            value={d.id}
            onChange={pick}
            options={v.days.map((x) => ({ value: x.id, label: x.title }))}
          />
        }
      />
      {openIssues.length > 0 && (
        <div style={{ marginBottom: 10 }}>
          <Banner tone="warn" actions={<Button size="xs" onClick={() => go({ workspace: "production", sub: "schedule", params: { dayId: d.id } })}>Open day</Button>}>
            {openIssues.length === 1 ? openIssues[0].message : `${openIssues.length} things to check on this day: ${openIssues.map((w) => w.message).join(" ")}`}
          </Banner>
        </div>
      )}
      <div className="grid g3" style={{ marginBottom: 10 }}>
        <div className="card pad">
          <div className="h4">Scenes</div>
          {d.scenes.length ? (
            <>
              <div className="row wrap gap4">
                {d.scenes.map((sc) => (
                  <button
                    key={sc.id}
                    type="button"
                    className="btn sm"
                    title={sc.heading}
                    onClick={() => go({ workspace: "breakdown", params: { sceneId: sc.sceneId } })}
                  >
                    {sc.number ?? "—"}
                  </button>
                ))}
              </div>
              <div className="xs muted" style={{ marginTop: 4 }}>
                {s.sceneCount} scene{s.sceneCount === 1 ? "" : "s"}
                {s.estimatedMinutes ? ` · ${formatMinutes(s.estimatedMinutes)} estimated` : ""}
                {s.missingEstimates ? ` · ${s.missingEstimates} without estimate` : ""}
              </div>
            </>
          ) : (
            <div className="sm muted">No scenes scheduled.</div>
          )}
        </div>
        <div className="card pad">
          <div className="h4">Location</div>
          {d.locations.length ? (
            d.locations.map((l) => (
              <div key={l.key} style={{ marginBottom: 4 }}>
                {l.locationId ? (
                  <button type="button" className="btn ghost sm" style={{ padding: 0, fontWeight: 700, fontSize: 15 }} onClick={() => go({ workspace: "production", sub: "locations", params: { locationId: l.locationId! } })}>
                    {l.name}
                  </button>
                ) : (
                  <b style={{ fontSize: 15 }}>{l.name}</b>
                )}
                <div className="xs muted">
                  {[l.status, l.address ?? "Address missing"].filter(Boolean).join(" · ")}
                </div>
              </div>
            ))
          ) : (
            <span className="need">Missing</span>
          )}
        </div>
        <div className="card pad">
          <div className="h4">Cast</div>
          {d.cast.length ? (
            <>
              <button type="button" className="btn ghost sm" style={{ padding: 0, fontWeight: 700, fontSize: 15, whiteSpace: "normal", textAlign: "left" }} onClick={() => go({ workspace: "production", sub: "cast" })}>
                {d.cast.map((c) => c.actor ?? c.character).join(" · ")}
              </button>
              <div className="xs muted">
                {calls.length ? `Calls ${calls.join(" / ")}` : "Call times not entered yet"}
                {d.crewCall ? ` · Crew call ${d.crewCall}` : ""}
              </div>
            </>
          ) : (
            <div className="sm muted">No cast confirmed for these scenes.</div>
          )}
        </div>
      </div>
      <div className="grid g2">
        <div className="card pad">
          <div className="h4">Call sheet</div>
          {d.callSheet ? (
            <>
              <Chip tone={callSheetTone(d.callSheet.status)}>{callSheetStatusLabel(d.callSheet.status)}</Chip>
              <div className="sm" style={{ margin: "6px 0" }}>
                {d.callSheet.sourceChanged
                  ? "Available — the schedule changed after it was created."
                  : d.callSheet.status === "Final" || d.callSheet.status === "Issued"
                    ? "Finalized and matches the schedule."
                    : "Available and up to date with the schedule."}
              </div>
              <Button size="sm" variant="primary" onClick={() => openCallSheetForDay(d.id, d.callSheet!.id)}>
                Open Call Sheet
              </Button>
            </>
          ) : d.isOffDay ? (
            <div className="sm muted">Off days have no call sheet.</div>
          ) : (
            <>
              <div className="sm" style={{ marginBottom: 6 }}>No call sheet yet for this day.</div>
              <Button size="sm" variant="primary" onClick={() => openCallSheetForDay(d.id)}>
                Create Call Sheet
              </Button>
            </>
          )}
        </div>
        <div className="card pad">
          <div className="h4">Important notes</div>
          {d.notes ? <div className="sm" style={{ whiteSpace: "pre-wrap" }}>{d.notes}</div> : <div className="sm muted">No day notes.</div>}
          {d.markers.length > 0 && (
            <div className="row wrap gap4" style={{ marginTop: 6 }}>
              {d.markers.map((m) => (
                <Chip key={m.id} tone="out">
                  {markerText(m)}
                </Chip>
              ))}
            </div>
          )}
        </div>
        <div className="card pad">
          <div className="h4">Important breakdown items</div>
          {d.breakdownItems.length ? (
            <div className="row wrap gap4">
              {d.breakdownItems.map((it) => (
                <Chip key={`${it.category}:${it.name}`} title={it.category}>
                  {it.name}
                </Chip>
              ))}
            </div>
          ) : (
            <div className="sm muted">No confirmed props, wardrobe or other items for these scenes.</div>
          )}
        </div>
        <div className="card pad">
          <div className="h4">Scene order</div>
          {d.scenes.length ? (
            <ol className="sm" style={{ margin: 0, paddingLeft: 18 }}>
              {d.scenes.map((sc) => (
                <li key={sc.id}>
                  {sc.number ?? "—"} — {sc.heading}
                  {sc.estimatedMinutes !== null ? ` (${formatMinutes(sc.estimatedMinutes)})` : ""}
                </li>
              ))}
            </ol>
          ) : (
            <div className="sm muted">Nothing scheduled yet.</div>
          )}
        </div>
      </div>
    </div>
  );
}
