// List, calendar and shooting-day detail views of the schedule
// (UX §3.29; mocks 131, 132, 135).

import { useEffect, useMemo, useState } from "react";
import { ArrowLeft, ChevronLeft, ChevronRight, MoreHorizontal } from "lucide-react";
import { Button, Chip, IconButton, Menu, PageHeader, Segmented, TextArea } from "../../../design-system";
import { call } from "../../../ipc/client";
import { reportError } from "../../../ipc/query";
import { toast } from "../../../app/toast";
import type { ScheduleStripDto, ScheduleView, ScheduleShootDayDto } from "../../../api/schedule";
import { callSheetStatusLabel, formatMinutes, parseDuration } from "../../../api/schedule";
import { dayMenuItems, stripMenuItems, type ScheduleUi } from "./Board";
import { MarkerRow, StripRow } from "./Strip";
import { monthGrid, monthLabel, shiftMonth } from "./calendar";
import { ScheduleExportButton } from "../../../features/export/buttons";

// ------------------------------------------------------------------ list

type ListFilter = "all" | "unscheduled" | "scheduled";

export function ScheduleList({ view, ui }: { view: ScheduleView; ui: ScheduleUi }) {
  const [filter, setFilter] = useState<ListFilter>("all");
  const rows = useMemo(() => {
    const out: { s: ScheduleStripDto; day?: ScheduleShootDayDto }[] = [];
    for (const d of view.days) for (const i of d.items) if (i.strip) out.push({ s: i.strip, day: d });
    for (const s of view.unscheduled) out.push({ s });
    out.sort((a, b) => (a.s.number ?? 1e9) - (b.s.number ?? 1e9));
    return out.filter((r) => filter === "all" || (filter === "unscheduled" ? !r.day : !!r.day));
  }, [view, filter]);
  return (
    <div className="of-scroll" style={{ flex: 1 }}>
      <div className="row" style={{ marginBottom: 8 }}>
        <Segmented
          ariaLabel="Show scenes"
          value={filter}
          onChange={setFilter}
          options={[
            { value: "all", label: "All scenes" },
            { value: "unscheduled", label: "Only unscheduled" },
            { value: "scheduled", label: "Scheduled" },
          ]}
        />
      </div>
      <table className="tbl card">
        <thead>
          <tr>
            <th>Sc</th>
            <th>I/E · D/N</th>
            <th>Location</th>
            <th>Day</th>
            <th className="num">Pages</th>
            <th className="num">Est. time</th>
            <th>Cast</th>
            <th aria-label="Actions" />
          </tr>
        </thead>
        <tbody>
          {rows.map(({ s, day }) => (
            <tr key={s.id} className="clickable" onClick={() => ui.openStrip(s.id)}>
              <td className="b">{s.number ?? "—"}</td>
              <td>{s.ieLabel}</td>
              <td>{s.locationName ?? <span className="need">Missing</span>}</td>
              <td>{day ? day.title : <span className="muted">Unscheduled</span>}</td>
              <td className="num">
                {s.pagesLabel}
                {s.pagesOverridden ? "*" : ""}
              </td>
              <td className="num">{s.estimatedMinutes !== null ? formatMinutes(s.estimatedMinutes) : <span className="muted">Missing</span>}</td>
              <td>{s.cast.map((c) => c.character).join(", ")}</td>
              <td onClick={(e) => e.stopPropagation()}>
                <Menu
                  align="end"
                  items={stripMenuItems(s, view, ui)}
                  trigger={
                    <IconButton label={`Scene ${s.number ?? ""} actions`}>
                      <MoreHorizontal size={14} />
                    </IconButton>
                  }
                />
              </td>
            </tr>
          ))}
          {rows.length === 0 && (
            <tr>
              <td colSpan={8} className="muted">
                No matching scenes.
              </td>
            </tr>
          )}
        </tbody>
      </table>
    </div>
  );
}

// ------------------------------------------------------------------ calendar

const WEEKDAYS = ["MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"];

export function ScheduleCalendar({ view, ui }: { view: ScheduleView; ui: ScheduleUi }) {
  const firstDated = view.days.find((d) => d.date)?.date;
  const initial = useMemo<[number, number]>(() => {
    const src = firstDated ? new Date(`${firstDated}T00:00:00`) : new Date();
    return [src.getFullYear(), src.getMonth()];
  }, [firstDated]);
  const [ym, setYm] = useState<[number, number]>(initial);
  useEffect(() => setYm(initial), [initial]);
  const byDate = useMemo(() => {
    const m = new Map<string, ScheduleShootDayDto[]>();
    for (const d of view.days) if (d.date) m.set(d.date, [...(m.get(d.date) ?? []), d]);
    return m;
  }, [view.days]);
  const undated = view.days.filter((d) => !d.date);
  const cells = monthGrid(ym[0], ym[1]);
  return (
    <div className="of-scroll" style={{ flex: 1 }}>
      <div className="row" style={{ marginBottom: 8 }}>
        <IconButton label="Previous month" onClick={() => setYm(shiftMonth(ym[0], ym[1], -1))}>
          <ChevronLeft size={16} />
        </IconButton>
        <b style={{ minWidth: 150, textAlign: "center" }}>{monthLabel(ym[0], ym[1])}</b>
        <IconButton label="Next month" onClick={() => setYm(shiftMonth(ym[0], ym[1], 1))}>
          <ChevronRight size={16} />
        </IconButton>
      </div>
      <div className="calgrid" role="grid" aria-label={`Shooting calendar, ${monthLabel(ym[0], ym[1])}`}>
        {WEEKDAYS.map((w) => (
          <div key={w} className="of-calhead" role="columnheader">
            {w}
          </div>
        ))}
        {cells.map((c) => {
          const days = byDate.get(c.iso) ?? [];
          const off = days.length > 0 && days.every((d) => d.isOffDay);
          const shoot = days.some((d) => !d.isOffDay);
          return (
            <div key={c.iso} role="gridcell" className={`calc${shoot ? " sd" : ""}${off ? " off" : ""}${c.inMonth ? "" : " of-outside"}`}>
              <div className="d">{c.day}</div>
              {days.map((d) => (
                <button key={d.id} type="button" className="of-calday" onClick={() => ui.openDay(d.id)}>
                  {d.isOffDay ? (
                    <b>OFF</b>
                  ) : (
                    <>
                      <b>{d.label}</b>
                      <div className="xs">
                        {d.summary.sceneCount} scene{d.summary.sceneCount === 1 ? "" : "s"}
                      </div>
                      {d.callSheet && <div className="xs muted">Call sheet: {callSheetStatusLabel(d.callSheet.status)}</div>}
                    </>
                  )}
                </button>
              ))}
            </div>
          );
        })}
      </div>
      {undated.length > 0 && (
        <div className="card pad" style={{ marginTop: 10 }}>
          <div className="h4">Not yet dated</div>
          <div className="row wrap gap4">
            {undated.map((d) => (
              <Button key={d.id} size="sm" onClick={() => ui.editDate(d)}>
                {d.label} — set date
              </Button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

// ------------------------------------------------------------------ day detail

export function DayDetail({ view, day, ui, onBack }: { view: ScheduleView; day: ScheduleShootDayDto; ui: ScheduleUi; onBack: () => void }) {
  const [notes, setNotes] = useState(day.notes ?? "");
  const [target, setTarget] = useState(day.plannedMinutes !== null ? formatMinutes(day.plannedMinutes) : "");
  useEffect(() => setNotes(day.notes ?? ""), [day.notes]);
  useEffect(() => setTarget(day.plannedMinutes !== null ? formatMinutes(day.plannedMinutes) : ""), [day.plannedMinutes]);
  const saveNotes = async () => {
    if ((day.notes ?? "") === notes) return;
    try {
      await call("schedule.set_day_notes", { dayId: day.id, notes: notes || null });
    } catch (e) {
      reportError(e);
    }
  };
  const saveTarget = async () => {
    const m = parseDuration(target);
    if (Number.isNaN(m)) {
      toast.error("Enter the target day length, e.g. 10h.");
      return;
    }
    if (m === day.plannedMinutes) return;
    try {
      await call("schedule.set_day_target", { dayId: day.id, minutes: m });
      toast.undoable(`Changed the target length of ${day.label}`);
    } catch (e) {
      reportError(e);
    }
  };
  const s = day.summary;
  const warnings = view.warnings.filter((w) => w.dayId === day.id && !w.decision);
  return (
    <div className="of-tab">
      <PageHeader
        title={day.title}
        sub={day.isOffDay ? "Off day — no scenes, notes allowed." : "What exactly are we doing on this day?"}
        actions={
          <>
            <Button size="sm" icon={<ArrowLeft size={14} />} onClick={onBack}>
              Back to schedule
            </Button>
            <Button size="sm" onClick={() => ui.editDate(day)}>
              Edit date
            </Button>
            <Menu
              align="end"
              items={[
                { label: "Meal Break", onSelect: () => ui.addMarker(day.id, "Meal") },
                { label: "Travel", onSelect: () => ui.addMarker(day.id, "Travel") },
                { label: "Company Move", onSelect: () => ui.addMarker(day.id, "Company Move") },
                { label: "Custom Note", onSelect: () => ui.addMarker(day.id, "Custom") },
              ]}
              trigger={<Button size="sm">Add break</Button>}
            />
            <Menu align="end" items={dayMenuItems(day, ui, { detail: true })} trigger={<IconButton label="More day actions"><MoreHorizontal size={16} /></IconButton>} />
          </>
        }
      />
      <div className="row" style={{ alignItems: "flex-start", gap: 14, flex: 1, minHeight: 0 }}>
        <div className="grow of-scroll" style={{ maxHeight: "100%" }}>
          {warnings.map((w) => (
            <div key={w.key} className="banner warn" style={{ marginBottom: 6 }}>
              <span className="sp">{w.message}</span>
            </div>
          ))}
          {day.items.length === 0 && (
            <div className="empty" style={{ height: "auto", padding: 24 }}>
              <div>{day.isOffDay ? "Off day." : "No scenes scheduled."}</div>
              {!day.isOffDay && (
                <div className="row">
                  <Button size="sm" onClick={onBack}>
                    Add Scene
                  </Button>
                  <Button size="sm" onClick={() => call("schedule.set_off_day", { dayId: day.id, offDay: true }).then(() => toast.undoable(`Set ${day.label} as an off day`), reportError)}>
                    Off Day
                  </Button>
                </div>
              )}
            </div>
          )}
          {day.items.map((i) =>
            i.strip ? (
              <StripRow
                key={i.strip.id}
                strip={i.strip}
                onOpen={() => ui.openStrip(i.strip!.id)}
                menu={
                  <Menu
                    align="end"
                    items={stripMenuItems(i.strip, view, ui)}
                    trigger={
                      <IconButton label={`Scene ${i.strip.number ?? ""} actions`}>
                        <MoreHorizontal size={14} />
                      </IconButton>
                    }
                  />
                }
              />
            ) : (
              <MarkerRow key={i.marker!.id} marker={i.marker!} onOpen={() => ui.editMarker(day.id, i.marker!)} />
            ),
          )}
          <div className="row" style={{ marginTop: 10 }}>
            <Chip>Total estimate: {s.estimatedMinutes ? formatMinutes(s.estimatedMinutes) : "Missing"}</Chip>
            {s.missingEstimates > 0 && <Chip tone="y">{s.missingEstimates} without estimate</Chip>}
            <Chip tone="out">{s.pagesLabel} pages</Chip>
            <span className="grow" />
            <ScheduleExportButton
              days={view.days.map((d) => ({ id: d.id, label: d.title }))}
              currentDayId={day.id}
              sourceLabel={view.schedule?.source?.label}
            />
            {!day.isOffDay && (
              <Button variant="primary" onClick={() => ui.openCallSheet(day)}>
                {day.callSheet ? "Open Call Sheet" : "Create Call Sheet"}
              </Button>
            )}
          </div>
        </div>
        <div className="card pad" style={{ width: 300, flex: "none" }}>
          <div className="h4">Derived from the scenes</div>
          <div className="sm">
            <b>Locations</b>
            <br />
            {s.locations.length ? s.locations.map((l) => `${l.name}${l.status ? ` (${l.status})` : ""}`).join(" · ") : <span className="need">Missing</span>}
          </div>
          <div className="sm" style={{ marginTop: 8 }}>
            <b>Cast</b>
            <br />
            {s.cast.length ? s.cast.map((c) => c.actor ?? c.character).join(" · ") : "—"}
          </div>
          <div className="sm" style={{ marginTop: 8 }}>
            <b>Estimated duration</b>
            <br />
            {formatMinutes(s.estimatedMinutes)} of {formatMinutes(s.targetMinutes)} target
            {s.overTarget && <Chip tone="r">Over target</Chip>}
          </div>
          <label className="sm" style={{ display: "block", marginTop: 8 }}>
            <b>Day target</b> <span className="muted">(empty = schedule default)</span>
            <input className="input" value={target} placeholder={formatMinutes(view.schedule!.dayDurationMinutes)} onChange={(e) => setTarget(e.target.value)} onBlur={saveTarget} aria-label="Day target length" />
          </label>
          {day.callSheet && (
            <div className="sm" style={{ marginTop: 8 }}>
              <b>Call sheet</b>
              <br />
              {callSheetStatusLabel(day.callSheet.status)}
            </div>
          )}
          <div className="hr" />
          <div className="h4">Day notes</div>
          <TextArea aria-label="Day notes" value={notes} onChange={(e) => setNotes(e.target.value)} onBlur={saveNotes} rows={4} />
          <div className="hint" style={{ marginTop: 6 }}>
            Derived values are viewed here, not edited. Change scenes in the schedule.
          </div>
        </div>
      </div>
    </div>
  );
}
