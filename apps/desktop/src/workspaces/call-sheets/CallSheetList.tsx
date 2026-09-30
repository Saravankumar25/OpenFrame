// Call sheet list and "Create Call Sheet" dialog (UX §3.31; mocks 141–142).

import { useMemo, useState } from "react";
import { ClipboardList, MoreHorizontal, Plus, Search } from "lucide-react";
import {
  Banner,
  Button,
  Checkbox,
  Chip,
  ContextMenu,
  Dialog,
  EmptyState,
  Field,
  IconButton,
  Menu,
  PageHeader,
  Segmented,
  Select,
  Skeleton,
  type MenuItemSpec,
} from "../../design-system";
import { call } from "../../ipc/client";
import { reportError, useOp } from "../../ipc/query";
import { toast } from "../../app/toast";
import { useNav } from "../../app/stores";
import type { CallSheetListRow, ScheduleView } from "../../api/schedule";
import { CALL_SHEET_TABLES, SCHEDULE_TABLES, callSheetStatusLabel, callSheetTone } from "../../api/schedule";
import { openCallSheetForDay } from "./shared/open-call-sheet";
import { CallSheetExportButton } from "../../features/export/buttons";

type Filter = "all" | "draft" | "refresh" | "final";

function matches(r: CallSheetListRow, f: Filter): boolean {
  switch (f) {
    case "draft":
      return r.status === "Draft" || r.status === "Ready";
    case "refresh":
      return r.status === "Needs Refresh";
    case "final":
      return r.status === "Final" || r.status === "Issued";
    default:
      return true;
  }
}

function CreateDialog({ view, onClose }: { view: ScheduleView | undefined; onClose: () => void }) {
  const candidates = (view?.days ?? []).filter((d) => !d.isOffDay && !d.callSheet);
  const [dayId, setDayId] = useState(candidates[0]?.id ?? "");
  const go = useNav((s) => s.go);
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Create Call Sheet"
      size="md"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button
            variant="primary"
            disabled={!dayId}
            onClick={async () => {
              await openCallSheetForDay(dayId);
              onClose();
            }}
          >
            Create Call Sheet
          </Button>
        </>
      }
    >
      {!view?.schedule ? (
        <Banner tone="info" actions={<Button size="xs" onClick={() => { onClose(); go({ workspace: "production", sub: "schedule" }); }}>Open Schedule</Button>}>
          Create a shooting schedule and a shooting day first.
        </Banner>
      ) : candidates.length === 0 ? (
        <Banner tone="info">Every shooting day already has a call sheet. Open it from the list to update it.</Banner>
      ) : (
        <>
          <Field label="Source shooting day" required>
            <Select ariaLabel="Source shooting day" value={dayId} onChange={setDayId} options={candidates.map((d) => ({ value: d.id, label: d.title }))} />
          </Field>
          <div className="h4" style={{ marginTop: 6 }}>Will be filled in automatically</div>
          <ul className="sm" style={{ margin: "0 0 8px", paddingLeft: 18, lineHeight: 1.7 }}>
            <li>Title, date, day number</li>
            <li>Scenes</li>
            <li>Cast</li>
            <li>Location + address</li>
            <li>Notes from the day</li>
          </ul>
          <div className="hint">You will only add call times and day-specific practical notes.</div>
        </>
      )}
    </Dialog>
  );
}

export function CallSheetList() {
  const q = useOp<CallSheetListRow[]>("callsheets.list", {}, CALL_SHEET_TABLES);
  const sched = useOp<ScheduleView>("schedule.get", {}, SCHEDULE_TABLES);
  const go = useNav((s) => s.go);
  const [filter, setFilter] = useState<Filter>("all");
  const [text, setText] = useState("");
  const [showOld, setShowOld] = useState(false);
  const [creating, setCreating] = useState(false);
  const rows = useMemo(() => {
    const t = text.trim().toLowerCase();
    return (q.data ?? []).filter(
      (r) =>
        (showOld || r.status !== "Superseded") &&
        matches(r, filter) &&
        (!t || `${r.title} ${r.dayLabel} ${r.dateLabel ?? ""} ${r.status}`.toLowerCase().includes(t)),
    );
  }, [q.data, filter, text, showOld]);

  const open = (r: CallSheetListRow, action?: string) => go({ workspace: "callsheets", params: action ? { callSheetId: r.id, action } : { callSheetId: r.id } });
  const menu = (r: CallSheetListRow): MenuItemSpec[] => [
    { label: "Open", onSelect: () => open(r) },
    { label: "Refresh", disabled: !r.sourceChanged || r.status === "Superseded" || !r.dayExists, onSelect: () => open(r, "refresh") },
    { label: "Finalize", disabled: !(r.status === "Draft" || r.status === "Ready" || r.status === "Needs Refresh"), onSelect: () => open(r, "finalize") },
    {
      label: "Delete",
      danger: true,
      separatorBefore: true,
      onSelect: async () => {
        try {
          await call("callsheets.delete", { id: r.id });
          toast.undoable(`Deleted ${r.title}`);
        } catch (e) {
          reportError(e);
        }
      },
    },
  ];

  if (q.isLoading) return <div className="content"><Skeleton h={28} w={240} /></div>;
  const empty = (q.data ?? []).length === 0;
  return (
    <div className="content">
      <PageHeader
        title="Call Sheets"
        sub="Which shooting day document do I need?"
        actions={
          <Button variant="primary" icon={<Plus size={15} />} onClick={() => setCreating(true)}>
            Create Call Sheet
          </Button>
        }
      />
      {q.error && <Banner tone="err">{q.error.message}</Banner>}
      {empty ? (
        <EmptyState icon={<ClipboardList size={40} />} title="Create a call sheet from a shooting day." actions={<Button variant="primary" onClick={() => setCreating(true)}>Create Call Sheet</Button>}>
          Scenes, cast and locations are filled in from the schedule. You add call times and practical notes.
        </EmptyState>
      ) : (
        <>
          <div className="toolbar">
            <label className="input" style={{ width: 260 }}>
              <Search size={14} aria-hidden />
              <input aria-label="Search call sheets" placeholder="Search" value={text} onChange={(e) => setText(e.target.value)} style={{ border: 0, outline: "none", flex: 1, background: "transparent" }} />
            </label>
            <Segmented
              ariaLabel="Filter call sheets"
              value={filter}
              onChange={setFilter}
              options={[
                { value: "all", label: "All" },
                { value: "draft", label: "Draft" },
                { value: "refresh", label: "Needs refresh" },
                { value: "final", label: "Finalized" },
              ]}
            />
            <Checkbox checked={showOld} onChange={setShowOld} label="Show superseded revisions" />
          </div>
          <table className="tbl card">
            <thead>
              <tr>
                <th>Shoot day</th>
                <th>Date</th>
                <th>Status</th>
                <th>Revision</th>
                <th aria-label="Actions" />
              </tr>
            </thead>
            <tbody>
              {rows.map((r) => (
                <ContextMenu key={r.id} items={menu(r)}>
                  <tr className="clickable" onDoubleClick={() => open(r)}>
                    <td className="b">
                      {r.dayLabel}
                      {!r.dayExists && <span className="xs muted"> (day deleted)</span>}
                    </td>
                    <td>{r.dateLabel ?? <span className="muted">No date</span>}</td>
                    <td>
                      <Chip tone={callSheetTone(r.status)}>{callSheetStatusLabel(r.status)}</Chip>
                    </td>
                    <td>v{r.revision}</td>
                    <td className="num" style={{ whiteSpace: "nowrap" }}>
                      <Button size="sm" onClick={() => open(r)}>
                        Open
                      </Button>
                      <CallSheetExportButton size="sm" label="Export" id={r.id} title={r.title} status={r.status} />
                      <Menu align="end" items={menu(r)} trigger={<IconButton label={`${r.title} actions`}><MoreHorizontal size={14} /></IconButton>} />
                    </td>
                  </tr>
                </ContextMenu>
              ))}
              {rows.length === 0 && (
                <tr>
                  <td colSpan={5} className="muted">
                    No matching call sheets.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </>
      )}
      {creating && <CreateDialog view={sched.data} onClose={() => setCreating(false)} />}
    </div>
  );
}
