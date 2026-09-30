// Dialogs and drawers of the Shooting Schedule (UX §3.29; mocks 129, 134–138).

import { useEffect, useState } from "react";
import { ArrowRight, Check, Sparkles } from "lucide-react";
import {
  Banner,
  Button,
  Checkbox,
  Chip,
  Dialog,
  Drawer,
  Field,
  Select,
  TextArea,
  TextInput,
} from "../../../design-system";
import { call } from "../../../ipc/client";
import { reportError, useOp } from "../../../ipc/query";
import { toast } from "../../../app/toast";
import { useNav } from "../../../app/stores";
import type { ScheduleSourceInfo } from "../../../ipc/generated/ScheduleSourceInfo";
import type { ScheduleDto } from "../../../ipc/generated/ScheduleDto";
import type { ScheduleScriptChanges } from "../../../ipc/generated/ScheduleScriptChanges";
import type {
  ScheduleGroupingSuggestion,
  ScheduleGroupingSuggestions,
  ScheduleMarkerDto,
  ScheduleStripDto,
  ScheduleView,
  ScheduleWarningDto,
  ScheduleShootDayDto,
} from "../../../api/schedule";
import {
  SCHEDULE_TABLES,
  formatMinutes,
  formatPages,
  parseDuration,
  parsePages,
  runScheduleMove,
} from "../../../api/schedule";

async function run(op: string, args: object, undoText?: string): Promise<boolean> {
  try {
    await call(op, args);
    if (undoText) toast.undoable(undoText);
    return true;
  } catch (e) {
    reportError(e);
    return false;
  }
}

// ------------------------------------------------------------------ create schedule

export function CreateScheduleDialog({ open, onOpenChange, source }: {
  open: boolean;
  onOpenChange: (v: boolean) => void;
  source: ScheduleSourceInfo;
}) {
  const [busy, setBusy] = useState(false);
  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title="Create Shooting Schedule"
      size="md"
      footer={
        <>
          <Button onClick={() => onOpenChange(false)}>Cancel</Button>
          <Button
            variant="primary"
            disabled={busy}
            onClick={async () => {
              setBusy(true);
              const ok = await run("schedule.create", {}, "Created the shooting schedule");
              setBusy(false);
              if (ok) onOpenChange(false);
            }}
          >
            Create Schedule
          </Button>
        </>
      }
    >
      <Field label="Screenplay source">
        <div className="input" aria-readonly>
          {source.screenplayTitle} — {source.label} · {source.sceneCount} scenes
        </div>
      </Field>
      <Banner tone="info">
        All {source.sceneCount} scenes will start in the Unscheduled pool. Scenes that need a location or cast will show it from your
        Breakdown once available.
      </Banner>
    </Dialog>
  );
}

// ------------------------------------------------------------------ days

export type DayDialogMode = { kind: "create"; offDay: boolean; afterDayId?: string } | { kind: "date"; day: ScheduleShootDayDto };

export function DayDialog({ mode, scheduleId, onClose }: { mode: DayDialogMode | null; scheduleId: string; onClose: () => void }) {
  const [date, setDate] = useState("");
  const [notes, setNotes] = useState("");
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    setDate(mode?.kind === "date" ? (mode.day.date ?? "") : "");
    setNotes("");
  }, [mode]);
  if (!mode) return null;
  const creating = mode.kind === "create";
  const title = creating ? (mode.offDay ? "Add Off Day" : "Create Shooting Day") : `Edit date — ${mode.day.label}`;
  const save = async () => {
    setBusy(true);
    const ok = creating
      ? await run(
          "schedule.create_day",
          { scheduleId, date: date || null, notes: notes || null, offDay: mode.offDay, afterDayId: mode.afterDayId ?? null },
          mode.offDay ? "Added an off day" : "Created a shooting day",
        )
      : await run("schedule.set_day_date", { dayId: mode.day.id, date: date || null }, `Rescheduled ${mode.day.label}`);
    setBusy(false);
    if (ok) onClose();
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title={title}
      size="sm"
      footerLeft={!creating && mode.day.date ? <Button variant="ghost" onClick={() => setDate("")}>Clear date</Button> : undefined}
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={busy} onClick={save}>
            {creating ? (mode.offDay ? "Add Off Day" : "Create Day") : "Save date"}
          </Button>
        </>
      }
    >
      <Field label="Shooting date" hint="Optional. A day can exist without a date." htmlFor="of-day-date">
        <TextInput id="of-day-date" type="date" value={date} onChange={(e) => setDate(e.target.value)} />
      </Field>
      {!creating && mode.day.callSheet && (
        <div className="hint">Changing the date marks this day's call sheet as needing a refresh. The call sheet itself is not rewritten.</div>
      )}
      {creating && (
        <Field label={mode.offDay ? "Note" : "Day notes"} htmlFor="of-day-notes">
          <TextArea id="of-day-notes" value={notes} onChange={(e) => setNotes(e.target.value)} rows={3} />
        </Field>
      )}
    </Dialog>
  );
}

// ------------------------------------------------------------------ markers

const MARKER_TYPES = [
  { value: "Meal", label: "Meal Break" },
  { value: "Travel", label: "Travel" },
  { value: "Company Move", label: "Company Move" },
  { value: "Custom", label: "Custom Note" },
] as const;
type MarkerType = (typeof MARKER_TYPES)[number]["value"];

export type MarkerDialogMode = { dayId: string; type: MarkerType; marker?: undefined } | { dayId: string; marker: ScheduleMarkerDto; type?: undefined };

export function MarkerDialog({ mode, onClose }: { mode: MarkerDialogMode | null; onClose: () => void }) {
  const [type, setType] = useState<MarkerType>("Meal");
  const [label, setLabel] = useState("");
  const [time, setTime] = useState("");
  const [duration, setDuration] = useState("");
  const [notes, setNotes] = useState("");
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    if (!mode) return;
    const m = mode.marker;
    setType((m?.markerType as MarkerType) ?? mode.type ?? "Meal");
    setLabel(m?.label ?? "");
    setTime(m?.atTime ?? "");
    setDuration(m?.durationMinutes ? formatMinutes(m.durationMinutes) : "");
    setNotes(m?.notes ?? "");
    setError(null);
  }, [mode]);
  if (!mode) return null;
  const save = async () => {
    const mins = parseDuration(duration);
    if (Number.isNaN(mins)) {
      setError("Enter a duration such as 45m or 1h 30m.");
      return;
    }
    if (type === "Custom" && !label.trim()) {
      setError("Write the note for this marker.");
      return;
    }
    const base = { markerType: type, label: label.trim() || null, atTime: time || null, durationMinutes: mins, notes: notes.trim() || null };
    const ok = mode.marker
      ? await run("schedule.update_marker", { markerId: mode.marker.id, ...base }, "Edited a break")
      : await run("schedule.add_marker", { dayId: mode.dayId, ...base }, "Added a break");
    if (ok) onClose();
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title={mode.marker ? "Edit break" : "Add to this day"}
      size="sm"
      footerLeft={
        mode.marker ? (
          <Button
            variant="ghost"
            onClick={async () => {
              if (await run("schedule.delete_marker", { markerId: mode.marker!.id }, "Removed a break")) onClose();
            }}
          >
            Delete
          </Button>
        ) : undefined
      }
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" onClick={save}>
            {mode.marker ? "Save" : "Add"}
          </Button>
        </>
      }
    >
      <Field label="Type" htmlFor="of-mk-type">
        <Select id="of-mk-type" value={type} onChange={setType} options={MARKER_TYPES} />
      </Field>
      <Field label={type === "Custom" ? "Note" : "Label"} required={type === "Custom"} hint={type === "Custom" ? undefined : "Optional — defaults to the type."} htmlFor="of-mk-label">
        <TextInput id="of-mk-label" value={label} onChange={(e) => setLabel(e.target.value)} maxLength={120} />
      </Field>
      <div className="row" style={{ alignItems: "flex-start" }}>
        <div className="grow">
          <Field label="Time" hint="Optional, e.g. 20:30" htmlFor="of-mk-time">
            <TextInput id="of-mk-time" type="time" value={time} onChange={(e) => setTime(e.target.value)} />
          </Field>
        </div>
        <div className="grow">
          <Field label="Duration" hint="Counts toward the day estimate" htmlFor="of-mk-dur">
            <TextInput id="of-mk-dur" value={duration} placeholder="45m" onChange={(e) => setDuration(e.target.value)} />
          </Field>
        </div>
      </div>
      <Field label="Notes" htmlFor="of-mk-notes" error={error}>
        <TextArea id="of-mk-notes" value={notes} onChange={(e) => setNotes(e.target.value)} rows={2} />
      </Field>
    </Dialog>
  );
}

// ------------------------------------------------------------------ settings

export function SettingsDialog({ schedule, open, onOpenChange }: { schedule: ScheduleDto; open: boolean; onOpenChange: (v: boolean) => void }) {
  const [name, setName] = useState(schedule.name);
  const [length, setLength] = useState(formatMinutes(schedule.dayDurationMinutes));
  const [strict, setStrict] = useState(schedule.strictValidation);
  const [status, setStatus] = useState(schedule.status);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    if (!open) return;
    setName(schedule.name);
    setLength(formatMinutes(schedule.dayDurationMinutes));
    setStrict(schedule.strictValidation);
    setStatus(schedule.status);
    setError(null);
  }, [open, schedule]);
  const save = async () => {
    const mins = parseDuration(length);
    if (mins === null || Number.isNaN(mins)) {
      setError("Enter the target day length, e.g. 10h or 11h 30m.");
      return;
    }
    const ok = await run(
      "schedule.update_settings",
      { scheduleId: schedule.id, name, strictValidation: strict, dayDurationMinutes: mins, expectedRev: schedule.rev },
      "Changed schedule settings",
    );
    if (ok && status !== schedule.status) {
      await run("schedule.set_status", { scheduleId: schedule.id, status }, `Marked the schedule ${status}`);
    }
    if (ok) onOpenChange(false);
  };
  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title="Schedule settings"
      size="md"
      footer={
        <>
          <Button onClick={() => onOpenChange(false)}>Cancel</Button>
          <Button variant="primary" onClick={save}>
            Save
          </Button>
        </>
      }
    >
      <Field label="Name" required htmlFor="of-sch-name">
        <TextInput id="of-sch-name" value={name} onChange={(e) => setName(e.target.value)} maxLength={120} />
      </Field>
      <Field label="Target day length" hint="Days whose estimates exceed this show a warning. Each day can override it." error={error} htmlFor="of-sch-len">
        <TextInput id="of-sch-len" value={length} onChange={(e) => setLength(e.target.value)} />
      </Field>
      <Field label="Status">
        <Select
          ariaLabel="Schedule status"
          value={status}
          onChange={setStatus}
          options={[
            { value: "Draft", label: "Draft" },
            { value: "Active", label: "Active" },
            { value: "Finalized", label: "Finalized (no further changes until reopened)" },
          ]}
        />
      </Field>
      <Checkbox checked={strict} onChange={setStrict} label="Strict validation: ask before keeping a change that creates a warning" />
      <div className="hint" style={{ marginTop: 6 }}>
        Warnings never block you by default. With strict validation, a change that creates a new warning needs an explicit “Keep Anyway”.
      </div>
    </Dialog>
  );
}

// ------------------------------------------------------------------ script changes

export function ScriptChangesDialog({ changes, scheduleId, open, onOpenChange }: {
  changes: ScheduleScriptChanges;
  scheduleId: string;
  open: boolean;
  onOpenChange: (v: boolean) => void;
}) {
  const [busy, setBusy] = useState(false);
  const affected = changes.changed.length + changes.removed.length;
  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={affected ? `Script changed in ${affected} scene${affected === 1 ? "" : "s"}` : "The production source changed"}
      sub="The production source script changed. Your schedule has not been changed."
      size="lg"
      footer={
        <>
          <Button onClick={() => onOpenChange(false)}>Close</Button>
          <Button
            variant="primary"
            disabled={busy}
            onClick={async () => {
              setBusy(true);
              const ok = await run("schedule.reconcile", { scheduleId }, "Updated the schedule from the script changes");
              setBusy(false);
              if (ok) onOpenChange(false);
            }}
          >
            Update Schedule
          </Button>
        </>
      }
    >
      {changes.sourceSwitched && changes.newSource && (
        <Banner tone="info">
          New production source: <b>{changes.newSource.label}</b>. Scheduled scenes keep their days.
        </Banner>
      )}
      <table className="tbl" style={{ marginTop: 8 }}>
        <thead>
          <tr>
            <th>Sc</th>
            <th>Heading</th>
            <th>Change</th>
            <th>Scheduled</th>
          </tr>
        </thead>
        <tbody>
          {[...changes.changed, ...changes.removed].map((r) => (
            <tr key={r.stripId}>
              <td>{r.number ?? "—"}</td>
              <td>{r.heading}</td>
              <td>
                <div className="row wrap gap4">
                  {r.kinds.map((k) => (
                    <Chip key={k} tone={k.startsWith("Removed") ? "r" : "y"}>
                      {k}
                    </Chip>
                  ))}
                </div>
              </td>
              <td>{r.dayLabel ?? "Unscheduled"}</td>
            </tr>
          ))}
          {changes.newScenes > 0 && (
            <tr>
              <td>—</td>
              <td>
                {changes.newScenes} new scene{changes.newScenes === 1 ? "" : "s"}
              </td>
              <td>
                <Chip tone="b">New scene</Chip>
              </td>
              <td>Unscheduled</td>
            </tr>
          )}
        </tbody>
      </table>
      <div className="hint" style={{ marginTop: 8 }}>
        New scenes appear in Unscheduled. Removed scenes stay marked in the schedule history until you confirm removal. “Update Schedule”
        only flags changes — it never moves or deletes a scheduled scene.
      </div>
    </Dialog>
  );
}

// ------------------------------------------------------------------ strip details

export function StripDrawer({ strip, view, onClose }: { strip: ScheduleStripDto; view: ScheduleView; onClose: () => void }) {
  const go = useNav((s) => s.go);
  const [est, setEst] = useState("");
  const [pages, setPages] = useState("");
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    setEst(strip.estimatedMinutes !== null ? formatMinutes(strip.estimatedMinutes) : "");
    setPages(strip.pagesOverridden ? strip.pagesLabel : "");
    setError(null);
  }, [strip]);
  const day = view.days.find((d) => d.id === strip.dayId);
  const saveEst = async () => {
    const m = parseDuration(est);
    if (Number.isNaN(m)) return setError("Enter the estimate as minutes or e.g. 1h 30m.");
    setError(null);
    if (m !== strip.estimatedMinutes) await run("schedule.set_strip_estimate", { stripId: strip.id, minutes: m }, "Changed a time estimate");
  };
  const savePages = async () => {
    const e = parsePages(pages);
    if (Number.isNaN(e) || e === 0) return setError("Enter pages like 1 2/8 or 3/8.");
    setError(null);
    const current = strip.pagesOverridden ? strip.pageEighths : null;
    if (e !== current) await run("schedule.set_strip_pages", { stripId: strip.id, eighths: e }, "Changed a page count");
  };
  const title = strip.number !== null ? `Scene ${strip.number}` : "Scene";
  return (
    <Drawer open onClose={onClose} title={title} typeLabel="Scheduled scene" width="n">
      <div className="b" style={{ marginBottom: 4 }}>
        {strip.heading}
      </div>
      {strip.synopsis && <div className="sm muted" style={{ marginBottom: 8 }}>{strip.synopsis}</div>}
      {strip.sourceState !== "Current" && (
        <Banner
          tone={strip.sourceState === "Removed" ? "err" : "warn"}
          actions={
            strip.sourceState === "Removed" ? (
              <Button size="xs" onClick={() => run("schedule.confirm_removal", { stripId: strip.id }, "Removed the scene from the schedule").then((ok) => ok && onClose())}>
                Confirm removal
              </Button>
            ) : (
              <Button size="xs" onClick={() => run("schedule.acknowledge_change", { stripId: strip.id }, "Marked the change reviewed")}>
                Mark reviewed
              </Button>
            )
          }
        >
          {strip.changeKinds.join(" · ") || strip.sourceState}
        </Banner>
      )}
      <div className="hr" />
      <div className="sm">
        <b>Scheduled:</b> {day ? day.title : "Unscheduled"}
      </div>
      <div className="sm" style={{ marginTop: 6 }}>
        <b>Location:</b>{" "}
        {strip.locations.length === 0 ? (
          <span className="need">Missing</span>
        ) : (
          strip.locations.map((l) => `${l.name}${l.status ? ` (${l.status})` : l.fromBreakdown ? "" : " — from heading"}`).join(", ")
        )}
      </div>
      <div className="sm" style={{ marginTop: 6 }}>
        <b>Cast:</b> {strip.cast.length ? strip.cast.map((c) => (c.actor ? `${c.actor} (${c.character})` : c.character)).join(", ") : "None confirmed"}
      </div>
      <div className="hr" />
      <Field label="Estimated shooting time" hint="Entered by you. Never guessed." htmlFor="of-strip-est">
        <TextInput id="of-strip-est" value={est} placeholder="Missing" onChange={(e) => setEst(e.target.value)} onBlur={saveEst} onKeyDown={(e) => e.key === "Enter" && saveEst()} />
      </Field>
      <Field label="Page count override" hint={`From the screenplay: ${strip.pagesOverridden ? "overridden" : formatPages(strip.pageEighths)}. Leave empty to use it.`} htmlFor="of-strip-pages" error={error}>
        <TextInput id="of-strip-pages" value={pages} placeholder={formatPages(strip.pageEighths)} onChange={(e) => setPages(e.target.value)} onBlur={savePages} onKeyDown={(e) => e.key === "Enter" && savePages()} />
      </Field>
      <div className="col" style={{ gap: 6 }}>
        {strip.dayId && (
          <Button onClick={() => runScheduleMove("schedule.move_strip", { stripId: strip.id, dayId: null }, `Unscheduled ${title}`)}>Remove from Day</Button>
        )}
        <Button variant="ghost" icon={<ArrowRight size={14} />} onClick={() => go({ workspace: "screenplay", params: { sceneId: strip.sceneId } })}>
          Open in Screenplay
        </Button>
        <Button variant="ghost" icon={<ArrowRight size={14} />} onClick={() => go({ workspace: "breakdown", params: { sceneId: strip.sceneId } })}>
          Open in Breakdown
        </Button>
      </div>
      <div className="hint" style={{ marginTop: 10 }}>
        The heading and scene number come from the screenplay source and are edited there. Scheduling never changes the script.
      </div>
    </Drawer>
  );
}

// ------------------------------------------------------------------ warnings

export function WarningsDrawer({ view, onClose, onOpenDay }: { view: ScheduleView; onClose: () => void; onOpenDay: (id: string) => void }) {
  const scheduleId = view.schedule!.id;
  const decide = (w: ScheduleWarningDto, decision: string | null) =>
    run("schedule.decide_warning", { scheduleId, key: w.key, decision }, decision === "Kept" ? "Kept the schedule as it is" : undefined);
  const open = view.warnings.filter((w) => !w.decision);
  const decided = view.warnings.filter((w) => w.decision);
  return (
    <Drawer
      open
      onClose={onClose}
      title="Schedule assistance"
      typeLabel="Practical observations"
      width="n"
      footer={<div className="hint">Warnings never block you and nothing is fixed automatically.</div>}
    >
      {view.warnings.length === 0 && <div className="sm muted">No warnings. The schedule looks consistent.</div>}
      {open.map((w) => (
        <WarningCard key={w.key} w={w} onOpenDay={onOpenDay} decide={decide} />
      ))}
      {decided.length > 0 && (
        <>
          <div className="h4" style={{ marginTop: 12 }}>
            Kept or dismissed
          </div>
          {decided.map((w) => (
            <WarningCard key={w.key} w={w} onOpenDay={onOpenDay} decide={decide} />
          ))}
        </>
      )}
    </Drawer>
  );
}

function WarningCard({ w, onOpenDay, decide }: {
  w: ScheduleWarningDto;
  onOpenDay: (id: string) => void;
  decide: (w: ScheduleWarningDto, d: string | null) => Promise<boolean>;
}) {
  return (
    <div className={`of-warn-card${w.decision ? " decided" : ""}`}>
      <div>{w.message}</div>
      {w.detail && <div className="xs muted" style={{ marginTop: 3 }}>{w.detail}</div>}
      <div className="row gap4">
        {w.dayId && (
          <Button size="xs" onClick={() => onOpenDay(w.dayId!)}>
            Open affected item
          </Button>
        )}
        {w.dayId && w.kind !== "call_sheet_stale" && (
          <Button size="xs" onClick={() => onOpenDay(w.dayId!)}>
            Move scene (you decide)
          </Button>
        )}
        {w.decision ? (
          <>
            <Chip tone={w.decision === "Kept" ? "g" : "default"}>
              <Check size={11} /> {w.decision === "Kept" ? "Kept anyway" : "Dismissed"}
            </Chip>
            <Button size="xs" variant="ghost" onClick={() => decide(w, null)}>
              Show again
            </Button>
          </>
        ) : (
          <>
            <Button size="xs" onClick={() => decide(w, "Kept")}>
              Keep anyway
            </Button>
            <Button size="xs" variant="ghost" onClick={() => decide(w, "Dismissed")}>
              Dismiss
            </Button>
          </>
        )}
      </div>
    </div>
  );
}

// ------------------------------------------------------------------ grouping suggestions

export function SuggestionsDrawer({ view, onClose }: { view: ScheduleView; onClose: () => void }) {
  const q = useOp<ScheduleGroupingSuggestions>("schedule.suggestions", {}, SCHEDULE_TABLES);
  const [dismissed, setDismissed] = useState<string[]>([]);
  const [applying, setApplying] = useState<ScheduleGroupingSuggestion | null>(null);
  const [target, setTarget] = useState("");
  const days = view.days.filter((d) => !d.isOffDay);
  const list = (q.data?.suggestions ?? []).filter((s) => !dismissed.includes(s.key));
  return (
    <Drawer
      open
      onClose={onClose}
      title="Grouping suggestions"
      typeLabel="Schedule assistance"
      width="n"
      footer={
        <div className="hint">
          Suggestions are advice. OpenFrame never rearranges your schedule silently.
        </div>
      }
    >
      <Button size="sm" icon={<Sparkles size={14} />} onClick={() => q.refetch()} disabled={q.isFetching}>
        Ask for suggestion
      </Button>
      <div style={{ marginTop: 10 }}>
        {q.data && list.length === 0 && <div className="sm muted">{q.data.message ?? "No obvious grouping improvement found."}</div>}
        {list.map((s) => (
          <div key={s.key} className="of-warn-card">
            <div className="b">{s.title}</div>
            <div className="row gap4" style={{ marginTop: 4 }}>
              <Chip tone={s.kind === "location" ? "t" : "p"}>{s.subject}</Chip>
            </div>
            <div className="sm" style={{ marginTop: 4 }}>{s.advice}</div>
            {s.suggestedDayLabel && <div className="xs muted" style={{ marginTop: 3 }}>Most of them are already on {s.suggestedDayLabel}.</div>}
            <div className="row gap4">
              <Button
                size="xs"
                variant="primary"
                disabled={days.length === 0}
                onClick={() => {
                  setApplying(s);
                  setTarget(s.suggestedDayId ?? days[0]?.id ?? "");
                }}
              >
                Apply manually
              </Button>
              <Button size="xs" variant="ghost" onClick={() => setDismissed((d) => [...d, s.key])}>
                Dismiss
              </Button>
            </div>
          </div>
        ))}
      </div>
      {applying && (
        <Dialog
          open
          onOpenChange={(v) => !v && setApplying(null)}
          title="Group these scenes?"
          size="sm"
          footer={
            <>
              <Button onClick={() => setApplying(null)}>Cancel</Button>
              <Button
                variant="primary"
                disabled={!target}
                onClick={async () => {
                  const ids = applying.stripIds.filter((id) => {
                    const onTarget = days.find((d) => d.id === target)?.items.some((i) => i.strip?.id === id);
                    return !onTarget;
                  });
                  const label = days.find((d) => d.id === target)?.label ?? "the day";
                  if (ids.length) await runScheduleMove("schedule.move_strips", { stripIds: ids, dayId: target }, `Grouped scenes on ${label}`);
                  setApplying(null);
                }}
              >
                Move scenes
              </Button>
            </>
          }
        >
          <p style={{ marginTop: 0 }}>
            Scenes {applying.sceneNumbers.join(", ")} will be moved to the chosen day, after its current scenes. Nothing else changes and you can
            undo it.
          </p>
          <Field label="Shooting day">
            <Select ariaLabel="Shooting day" value={target} onChange={setTarget} options={days.map((d) => ({ value: d.id, label: d.title }))} />
          </Field>
        </Dialog>
      )}
    </Drawer>
  );
}
