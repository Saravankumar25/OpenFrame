// Production › Schedule tab (FSD §35–37, §56, §104; UX §3.29; mocks 128–138).
// Discovered by the Production workspace via `export const tab`.

import { useEffect, useMemo, useRef, useState } from "react";
import { CalendarDays, Columns3, List, Moon, Plus, Settings2, Sparkles, TriangleAlert } from "lucide-react";
import { Banner, Button, Chip, EmptyState, IconButton, Segmented, Skeleton } from "../../../design-system";
import { useOp } from "../../../ipc/query";
import { useNav } from "../../../app/stores";
import type { ScheduleMarkerDto, ScheduleView } from "../../../api/schedule";
import { SCHEDULE_TABLES } from "../../../api/schedule";
import { Board, type ScheduleUi } from "../../call-sheets/shared/Board";
import { DayDetail, ScheduleCalendar, ScheduleList } from "../../call-sheets/shared/ScheduleViews";
import {
  CreateScheduleDialog,
  DayDialog,
  MarkerDialog,
  ScriptChangesDialog,
  SettingsDialog,
  StripDrawer,
  SuggestionsDrawer,
  WarningsDrawer,
  type DayDialogMode,
  type MarkerDialogMode,
} from "../../call-sheets/shared/ScheduleDialogs";
import { KeepAnywayDialog } from "../../call-sheets/shared/KeepAnywayDialog";
import { openCallSheetForDay } from "../../call-sheets/shared/open-call-sheet";
import { ScheduleExportButton } from "../../../features/export/buttons";

export const tab = { id: "schedule", label: "Schedule", order: 70 };

type Mode = "board" | "list" | "calendar";

export default function ScheduleTab() {
  const q = useOp<ScheduleView>("schedule.get", {}, SCHEDULE_TABLES);
  const route = useNav((s) => s.route);
  const go = useNav((s) => s.go);
  const [mode, setMode] = useState<Mode>("board");
  const [creating, setCreating] = useState(false);
  const [dayDialog, setDayDialog] = useState<DayDialogMode | null>(null);
  const [markerDialog, setMarkerDialog] = useState<MarkerDialogMode | null>(null);
  const [stripId, setStripId] = useState<string | null>(null);
  const [drawer, setDrawer] = useState<"warnings" | "suggest" | null>(null);
  const [settings, setSettings] = useState(false);
  const [changesOpen, setChangesOpen] = useState(false);

  const view = q.data;
  const dayId = route.params?.dayId;

  // Deep link from the Screenplay / Scene Hub ("Open in Schedule"): open the
  // scene's strip. The link may name another draft's scene, so lineage matches too.
  const linkScene = route.params?.sceneId;
  const linkLineage = route.params?.sceneLineageId;
  const linked = useRef<string | null>(null);
  useEffect(() => {
    const key = `${linkScene ?? ""}|${linkLineage ?? ""}`;
    if (!view || (!linkScene && !linkLineage) || linked.current === key) return;
    linked.current = key;
    const strips = [...view.unscheduled, ...view.days.flatMap((d) => d.items.map((i) => i.strip)).filter((s): s is NonNullable<typeof s> => !!s)];
    const s = strips.find((x) => x.sceneId === linkScene) ?? strips.find((x) => x.lineageId === (linkLineage ?? linkScene));
    if (s) setStripId(s.id);
  }, [view, linkScene, linkLineage]);
  const openDay = (id: string | null) =>
    go({ workspace: "production", sub: "schedule", params: id ? { dayId: id } : {} });

  const ui: ScheduleUi = useMemo(
    () => ({
      openStrip: (id) => setStripId(id),
      openDay: (id) => openDay(id),
      editDate: (day) => setDayDialog({ kind: "date", day }),
      addMarker: (dayId, type) => setMarkerDialog({ dayId, type }),
      editMarker: (dayId: string, marker: ScheduleMarkerDto) => setMarkerDialog({ dayId, marker }),
      createDayAfter: (afterDayId) => setDayDialog({ kind: "create", offDay: false, afterDayId }),
      openCallSheet: (day) => void openCallSheetForDay(day.id, day.callSheet?.id),
    }),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  );

  const warnStripIds = useMemo(() => {
    const set = new Set<string>();
    for (const w of view?.warnings ?? []) if (!w.decision && w.kind !== "call_sheet_stale") for (const id of w.stripIds) set.add(id);
    return set;
  }, [view?.warnings]);

  if (q.isLoading || !view) {
    return (
      <div>
        {q.error ? <Banner tone="err">{q.error.message}</Banner> : <Skeleton h={28} w={260} />}
      </div>
    );
  }

  if (!view.schedule) {
    return (
      <div className="of-tab">
        {view.activeSource ? (
          <EmptyState
            icon={<CalendarDays size={40} />}
            title="Create a shooting schedule from your screenplay."
            actions={
              <Button variant="primary" onClick={() => setCreating(true)}>
                Create Shooting Schedule
              </Button>
            }
          >
            Break down your script first, then move scenes into shooting days. Production source: <b>{view.activeSource.label}</b> ·{" "}
            {view.activeSource.sceneCount} scenes.
          </EmptyState>
        ) : (
          <EmptyState
            icon={<CalendarDays size={40} />}
            title="Create a shooting schedule from your screenplay."
            actions={<Button onClick={() => go({ workspace: "breakdown" })}>Open Breakdown</Button>}
          >
            Choose a Production Source in Breakdown first. The schedule is built from the scenes of that screenplay draft.
          </EmptyState>
        )}
        {view.activeSource && <CreateScheduleDialog open={creating} onOpenChange={setCreating} source={view.activeSource} />}
      </div>
    );
  }

  const schedule = view.schedule;
  const day = dayId ? view.days.find((d) => d.id === dayId) : undefined;
  const strip = stripId
    ? (view.unscheduled.find((s) => s.id === stripId) ??
      view.days.flatMap((d) => d.items.map((i) => i.strip)).find((s) => s?.id === stripId) ??
      view.history.find((s) => s.id === stripId))
    : undefined;
  const changes = view.scriptChanges;

  const overlays = (
    <>
      <DayDialog mode={dayDialog} scheduleId={schedule.id} onClose={() => setDayDialog(null)} />
      <MarkerDialog mode={markerDialog} onClose={() => setMarkerDialog(null)} />
      {strip && <StripDrawer strip={strip} view={view} onClose={() => setStripId(null)} />}
      {drawer === "warnings" && <WarningsDrawer view={view} onClose={() => setDrawer(null)} onOpenDay={(id) => { setDrawer(null); openDay(id); }} />}
      {drawer === "suggest" && <SuggestionsDrawer view={view} onClose={() => setDrawer(null)} />}
      <SettingsDialog schedule={schedule} open={settings} onOpenChange={setSettings} />
      {changes && <ScriptChangesDialog changes={changes} scheduleId={schedule.id} open={changesOpen} onOpenChange={setChangesOpen} />}
      <KeepAnywayDialog />
    </>
  );

  if (day) {
    return (
      <div className="of-tab">
        <DayDetail view={view} day={day} ui={ui} onBack={() => openDay(null)} />
        {overlays}
      </div>
    );
  }

  const affected = changes ? changes.changed.length + changes.removed.length : 0;
  return (
    <div className="of-tab">
      <div className="banner info" style={{ marginBottom: 8 }}>
        <span className="sp">
          <b>Production Source: {schedule.source?.label ?? "missing"}</b> · {schedule.name} · {view.counts.total} scenes ({view.counts.scheduled} scheduled,{" "}
          {view.counts.unscheduled} unscheduled){schedule.status === "Finalized" ? " · Finalized" : ""}
        </span>
      </div>
      {changes && (
        <div style={{ marginBottom: 8 }}>
          <Banner
            tone="warn"
            actions={
              <Button size="xs" variant="primary" onClick={() => setChangesOpen(true)}>
                Review changes
              </Button>
            }
          >
            <b>{affected ? `Script changed in ${affected} scene${affected === 1 ? "" : "s"}` : changes.newScenes ? `${changes.newScenes} new scene${changes.newScenes === 1 ? "" : "s"} in the script` : "The production source changed"}.</b>{" "}
            Your schedule has not been changed.
          </Banner>
        </div>
      )}
      {schedule.status === "Finalized" && (
        <div style={{ marginBottom: 8 }}>
          <Banner tone="ok" actions={<Button size="xs" onClick={() => setSettings(true)}>Reopen…</Button>}>
            This schedule is finalized. Reopen it in settings to make changes.
          </Banner>
        </div>
      )}
      <div className="toolbar">
        <Segmented
          ariaLabel="Schedule view"
          value={mode}
          onChange={setMode}
          options={[
            { value: "board", label: <><Columns3 size={13} /> Board</> },
            { value: "list", label: <><List size={13} /> List</> },
            { value: "calendar", label: <><CalendarDays size={13} /> Calendar</> },
          ]}
        />
        <Button variant="primary" icon={<Plus size={15} />} onClick={() => setDayDialog({ kind: "create", offDay: false })}>
          Create Shooting Day
        </Button>
        <Button icon={<Moon size={15} />} onClick={() => setDayDialog({ kind: "create", offDay: true })}>
          Off Day
        </Button>
        <Button icon={<Sparkles size={15} />} onClick={() => setDrawer("suggest")}>
          Suggest Grouping
        </Button>
        <div className="sp" />
        <button type="button" className={`chip ${view.openWarningCount ? "r" : "g"}`} style={{ border: 0, cursor: "pointer" }} onClick={() => setDrawer("warnings")}>
          <TriangleAlert size={12} aria-hidden /> {view.openWarningCount} warning{view.openWarningCount === 1 ? "" : "s"}
        </button>
        {schedule.strictValidation && <Chip tone="p">Strict validation</Chip>}
        <IconButton label="Schedule settings" onClick={() => setSettings(true)}>
          <Settings2 size={16} />
        </IconButton>
        <ScheduleExportButton days={view.days.map((d) => ({ id: d.id, label: d.title }))} sourceLabel={schedule.source?.label} />
      </div>
      {mode === "board" && <Board view={view} ui={ui} warnStripIds={warnStripIds} />}
      {mode === "list" && <ScheduleList view={view} ui={ui} />}
      {mode === "calendar" && <ScheduleCalendar view={view} ui={ui} />}
      {view.history.length > 0 && mode === "list" && (
        <div className="hint" style={{ marginTop: 6 }}>
          {view.history.length} scene{view.history.length === 1 ? " was" : "s were"} removed from the schedule after script changes (kept in schedule history).
        </div>
      )}
      {overlays}
    </div>
  );
}
