// Production › Day Out of Days (Screenplay and Production Depth spec section 28;
// feature DOOD-001). A derived report: cast members by shooting days. The user
// picks the cast and the day range, and can correct a single cell; corrections
// are the only stored data and never change the screenplay, breakdown or schedule.

import { useEffect, useMemo, useState } from "react";
import { CalendarRange } from "lucide-react";
import { Banner, Button, Chip, Dialog, EmptyState, Field, Menu, Select, Skeleton, TextArea, type MenuItemSpec } from "../../../design-system";
import { useOp } from "../../../ipc/query";
import { call } from "../../../ipc/client";
import { reportError } from "../../../ipc/query";
import { toast } from "../../../app/toast";
import { useNav } from "../../../app/stores";
import type { DoodCell, DoodRow, ScheduleDoodView } from "../../../api/schedule";
import { SCHEDULE_TABLES } from "../../../api/schedule";
import { DoodExportButton } from "../../../features/export/buttons";
import "../../call-sheets/shared/schedule-ui.css";

export const tab = { id: "dood", label: "Day Out of Days", order: 75 };

/** Range and cast selection (view state only). */
export interface DoodSelection {
  castKeys: string[] | null;
  fromDayId: string | null;
  toDayId: string | null;
}

export function doodArgs(sel: DoodSelection): Record<string, unknown> {
  const a: Record<string, unknown> = {};
  if (sel.castKeys) a.castKeys = sel.castKeys;
  if (sel.fromDayId) a.fromDayId = sel.fromDayId;
  if (sel.toDayId) a.toDayId = sel.toDayId;
  return a;
}

/** The cell whose correction note is being edited. */
interface NoteTarget {
  row: DoodRow;
  cell: DoodCell;
  dayTitle: string;
}

function cellMenu(view: ScheduleDoodView, row: DoodRow, cell: DoodCell, dayTitle: string, editNote: (t: NoteTarget) => void): MenuItemSpec[] {
  const set = (code: string | null) => async () => {
    try {
      // Changing the code of a correction keeps its note; clearing the correction removes both.
      const note = code && cell.overridden ? cell.note : null;
      await call("schedule.set_dood_override", { scheduleId: view.scheduleId, castKey: row.key, dayId: cell.dayId, code, note });
      toast.undoable(code ? `Set ${row.label} on ${dayTitle} to ${code}` : `Cleared the correction for ${row.label} on ${dayTitle}`);
    } catch (e) {
      reportError(e);
    }
  };
  const items: MenuItemSpec[] = [{ label: `${row.label}, ${dayTitle}`, header: true }];
  for (const c of view.legend) {
    items.push({ label: `${c.code}  ${c.meaning}`, disabled: c.code === cell.code, onSelect: set(c.code) });
  }
  items.push({
    label: cell.derivedCode ? `Use the schedule value (${cell.derivedCode})` : "Use the schedule value (blank)",
    separatorBefore: true,
    disabled: !cell.overridden,
    onSelect: set(null),
  });
  items.push({
    label: cell.note ? "Edit the note" : "Add a note",
    disabled: !cell.overridden,
    onSelect: () => editNote({ row, cell, dayTitle }),
  });
  return items;
}

/** A short note on a correction ("Flies in from Mumbai"). Shown as the cell's tooltip and found by search. */
export function DoodNoteDialog({ scheduleId, target, onClose }: { scheduleId: string; target: NoteTarget | null; onClose: () => void }) {
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    if (target) setText(target.cell.note ?? "");
  }, [target]);
  if (!target) return null;
  const save = async () => {
    setBusy(true);
    try {
      await call("schedule.set_dood_override", {
        scheduleId,
        castKey: target.row.key,
        dayId: target.cell.dayId,
        code: target.cell.code,
        note: text.trim() || null,
      });
      toast.undoable(`Changed the note for ${target.row.label} on ${target.dayTitle}`);
      onClose();
    } catch (e) {
      reportError(e);
    } finally {
      setBusy(false);
    }
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Correction note"
      sub={`${target.row.label}, ${target.dayTitle}: ${target.cell.code}`}
      size="sm"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" onClick={save} disabled={busy}>
            Save
          </Button>
        </>
      }
    >
      <Field label="Note" hint="Optional. Up to 500 characters." htmlFor="of-dood-note">
        <TextArea id="of-dood-note" value={text} onChange={(e) => setText(e.target.value)} maxLength={500} rows={3} />
      </Field>
    </Dialog>
  );
}

export default function DoodTab() {
  const [sel, setSel] = useState<DoodSelection>({ castKeys: null, fromDayId: null, toDayId: null });
  const [noteFor, setNoteFor] = useState<NoteTarget | null>(null);
  const args = useMemo(() => doodArgs(sel), [sel]);
  const q = useOp<ScheduleDoodView>("schedule.dood", args, SCHEDULE_TABLES);
  const go = useNav((s) => s.go);
  const view = q.data;

  if (q.isLoading || !view) {
    return <div className="of-tab">{q.error ? <Banner tone="err">{q.error.message}</Banner> : <Skeleton h={28} w={260} />}</div>;
  }
  if (!view.hasSchedule || view.allDays.length === 0) {
    return (
      <div className="of-tab">
        <EmptyState
          icon={<CalendarRange size={40} />}
          title="Day Out of Days comes from your shooting schedule."
          actions={<Button onClick={() => go({ workspace: "production", sub: "schedule" })}>Open Schedule</Button>}
        >
          Create shooting days and schedule scenes with confirmed cast. The report then shows who works on which day.
        </EmptyState>
      </div>
    );
  }
  const dayOptions = view.allDays.map((d) => ({ value: d.id, label: d.title }));
  const corrected = view.rows.reduce((n, r) => n + r.cells.filter((c) => c.overridden).length, 0);
  return (
    <div className="of-tab">
      <div className="toolbar">
        <Select
          ariaLabel="Cast to show"
          value={sel.castKeys?.[0] ?? ""}
          onChange={(v) => setSel({ ...sel, castKeys: v ? [v] : null })}
          options={[{ value: "", label: "All cast" }, ...view.allCast.map((c) => ({ value: c.key, label: c.label }))]}
        />
        <Select ariaLabel="First day" value={sel.fromDayId ?? ""} onChange={(v) => setSel({ ...sel, fromDayId: v || null })} options={[{ value: "", label: "From the first day" }, ...dayOptions]} />
        <Select ariaLabel="Last day" value={sel.toDayId ?? ""} onChange={(v) => setSel({ ...sel, toDayId: v || null })} options={[{ value: "", label: "To the last day" }, ...dayOptions]} />
        <Select
          ariaLabel="Code style"
          value={view.codeStyle}
          onChange={(v) => void changeStyle(v)}
          options={[
            { value: "industry", label: "Codes: SW, W, WF, H" },
            { value: "simple", label: "Codes: W and -" },
          ]}
        />
        <div className="sp" />
        {corrected > 0 && <Chip tone="y">{corrected} corrected by hand</Chip>}
        <DoodExportButton args={args} />
      </div>
      {view.staleCorrections > 0 && (
        <div style={{ marginBottom: 8 }}>
          <Banner tone="warn">
            {view.staleCorrections} correction{view.staleCorrections === 1 ? " is" : "s are"} kept for a cast member or day that is no longer in the schedule and not shown.
          </Banner>
        </div>
      )}
      {view.rows.length === 0 ? (
        <div className="empty" style={{ height: "auto", padding: 24 }}>
          No cast is assigned to scheduled scenes yet. Confirm cast in Breakdown and schedule the scenes.
        </div>
      ) : (
        <div className="of-scroll" style={{ flex: 1 }}>
          <table className="of-dood" aria-label="Day Out of Days">
            <thead>
              <tr>
                <th className="cast" scope="col">
                  Cast
                </th>
                {view.days.map((d) => (
                  <th key={d.id} scope="col" className={d.isOffDay ? "off" : undefined} title={d.title}>
                    {d.shortLabel}
                    {d.dateLabel && (
                      <div className="xs muted" style={{ fontWeight: 400 }}>
                        {d.dateLabel.split(" ").slice(0, 3).join(" ")}
                      </div>
                    )}
                  </th>
                ))}
                <th scope="col">Work</th>
                <th scope="col">Hold</th>
                <th scope="col">Travel</th>
                <th scope="col">Start</th>
                <th scope="col">Finish</th>
              </tr>
            </thead>
            <tbody>
              {view.rows.map((r) => (
                <tr key={r.key}>
                  <th className="cast" scope="row">
                    {r.number}. {r.label}
                  </th>
                  {r.cells.map((c, i) => {
                    const d = view.days[i];
                    const label = `${r.label}, ${d.title}: ${c.code || "blank"}${c.overridden ? `, corrected by hand (schedule says ${c.derivedCode || "blank"})` : ""}`;
                    return (
                      <td key={c.dayId} className={`code${d.isOffDay ? " off" : ""}${c.overridden ? " corrected" : ""}`}>
                        <Menu
                          items={() => cellMenu(view, r, c, d.title, setNoteFor)}
                          trigger={
                            <button type="button" aria-label={c.note ? `${label}, note: ${c.note}` : label} title={c.note ?? undefined}>
                              {c.code || " "}
                              {c.overridden ? "*" : ""}
                            </button>
                          }
                        />
                      </td>
                    );
                  })}
                  <td>{r.workDays}</td>
                  <td>{r.holdDays}</td>
                  <td>{r.travelDays}</td>
                  <td>{r.startLabel ?? ""}</td>
                  <td>{r.finishLabel ?? ""}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <div className="hint" style={{ marginTop: 8 }}>
            {view.legend.map((l) => `${l.code} ${l.meaning.toLowerCase()}`).join(" · ")}. * corrected by hand. Built from the cast confirmed in Breakdown and the scenes on each day; corrections never change the schedule.
          </div>
        </div>
      )}
      {view.scheduleId && <DoodNoteDialog scheduleId={view.scheduleId} target={noteFor} onClose={() => setNoteFor(null)} />}
    </div>
  );

  async function changeStyle(style: string) {
    if (!view || style === view.codeStyle) return;
    try {
      const s = await call<{ schedule: { id: string; name: string; strictValidation: boolean; dayDurationMinutes: number } | null }>("schedule.get", {});
      if (!s.schedule) return;
      await call("schedule.update_settings", {
        scheduleId: s.schedule.id,
        name: s.schedule.name,
        strictValidation: s.schedule.strictValidation,
        dayDurationMinutes: s.schedule.dayDurationMinutes,
        doodCodeStyle: style,
      });
      toast.undoable("Changed the Day Out of Days codes");
    } catch (e) {
      reportError(e);
    }
  }
}
