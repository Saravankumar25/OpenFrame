// Story Timeline (FSD §14, UX §3.10, mock 070): optional Story Day and
// time-of-day notes on screenplay scenes. Grouping only — OpenFrame never
// reorders scenes for you.

import { useEffect, useRef, useState } from "react";
import { CalendarDays } from "lucide-react";
import { Button, Checkbox, Dialog, EmptyState, Field, PageHeader, Select, Skeleton, TextInput } from "../../design-system";
import { reportError } from "../../ipc/query";
import { toast } from "../../app/toast";
import { useNav } from "../../app/stores";
import { story, useTimeline } from "../../api/story";
import type { StoryTimelineScene } from "../../ipc/generated/StoryTimelineScene";
import { useStoryUi } from "./ui";

export function TimelineView({ episodeId, readOnly }: { episodeId: string | null; readOnly: boolean }) {
  const [screenplayId, setScreenplayId] = useState<string | null>(null);
  const t = useTimeline(episodeId, screenplayId);
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [assign, setAssign] = useState(false);
  const go = useNav((s) => s.go);
  const data = t.data;

  const header = (
    <PageHeader
      title="Story Timeline"
      sub="Optional. Useful only when time continuity matters."
      actions={
        <>
          {data && data.screenplays.length > 1 && (
            <Select
              ariaLabel="Screenplay"
              value={data.screenplayId ?? ""}
              onChange={(v) => setScreenplayId(v)}
              options={data.screenplays.map((s) => ({ value: s.id, label: s.title }))}
            />
          )}
          {!readOnly && (
            <Button variant="primary" icon={<CalendarDays size={14} />} disabled={picked.size === 0} onClick={() => setAssign(true)}>
              Assign Story Day{picked.size ? ` (${picked.size})` : ""}
            </Button>
          )}
        </>
      }
    />
  );

  if (!data) return <div className="content">{header}<Skeleton h={160} /></div>;
  if (!data.draftId) {
    return (
      <div className="content">
        {header}
        <EmptyState
          icon={<CalendarDays size={28} />}
          title="No screenplay scenes yet."
          actions={
            !readOnly ? (
              <Button variant="primary" onClick={() => {
                go({ workspace: "story" });
                useStoryUi.getState().openDialog({ type: "build" });
              }}>
                Build Screenplay
              </Button>
            ) : undefined
          }
        >
          Story days belong to screenplay scenes. Build a screenplay from the Story Board or write one first. Story Board cards never need a story day.
        </EmptyState>
      </div>
    );
  }

  const toggle = (id: string, on: boolean) =>
    setPicked((s) => {
      const n = new Set(s);
      if (on) n.add(id);
      else n.delete(id);
      return n;
    });

  return (
    <div className="content" style={{ overflow: "auto" }}>
      {header}
      <div className="muted" style={{ fontSize: 12.5, marginBottom: 10 }}>
        {data.draftName} · {data.sceneCount} scene{data.sceneCount === 1 ? "" : "s"}. Scene numbers come from the screenplay order.
      </div>
      {data.groups.map((g) => (
        <section key={g.storyDay ?? "unassigned"} className="card" style={{ marginBottom: 12 }} aria-label={g.storyDay ? `Day ${g.storyDay}` : "Unassigned"}>
          <div className="row" style={{ padding: "8px 12px", borderBottom: "1px solid var(--line)", background: "#faf9f6" }}>
            <b style={{ textTransform: "uppercase" }}>{g.storyDay ? dayLabel(g.storyDay) : "Unassigned"}</b>
            <span className="chip">{g.scenes.length} scene{g.scenes.length === 1 ? "" : "s"}</span>
            {!g.storyDay && (
              <span className="muted" style={{ fontSize: 12 }}>Scenes you have not given a story day yet. This is fine — story days are optional.</span>
            )}
          </div>
          {g.scenes.map((s) => (
            <SceneRow key={s.sceneId} s={s} readOnly={readOnly} checked={picked.has(s.sceneId)} onCheck={(v) => toggle(s.sceneId, v)} />
          ))}
        </section>
      ))}
      <p className="muted" style={{ fontSize: 12 }}>This is a grouping view only. OpenFrame never reorders scenes for you.</p>
      {assign && (
        <AssignDayDialog
          count={picked.size}
          onClose={() => setAssign(false)}
          onAssign={(day) => {
            setAssign(false);
            void story
              .assignStoryDay([...picked], day)
              .then(() => {
                toast.undoable(day ? `Assigned Story Day ${day}` : "Cleared Story Day");
                setPicked(new Set());
              })
              .catch(reportError);
          }}
        />
      )}
    </div>
  );
}

const dayLabel = (d: string) => (/^\d+$/.test(d.trim()) ? `Day ${d.trim()}` : d);

function SceneRow({ s, readOnly, checked, onCheck }: { s: StoryTimelineScene; readOnly: boolean; checked: boolean; onCheck: (v: boolean) => void }) {
  const [note, setNote] = useState(s.timeNote ?? "");
  const last = useRef(s.timeNote ?? "");
  useEffect(() => {
    setNote(s.timeNote ?? "");
    last.current = s.timeNote ?? "";
  }, [s.timeNote]);
  return (
    <div className="row" style={{ padding: "6px 12px", borderBottom: "1px solid var(--line)", gap: 10 }}>
      {!readOnly && <Checkbox checked={checked} onChange={onCheck} label={<span className="sr-only">Select scene {s.number}</span>} />}
      <b style={{ width: 48 }}>Sc {s.number}</b>
      <span className="grow truncate" style={{ fontFamily: "Courier New, monospace", fontSize: 12.5 }}>{s.heading || "(no heading)"}</span>
      <TextInput
        aria-label={`Time of day note for scene ${s.number}`}
        placeholder="time note"
        value={note}
        disabled={readOnly}
        style={{ width: 150 }}
        onChange={(e) => setNote(e.target.value)}
        onBlur={() => {
          if (note !== last.current) {
            last.current = note;
            void story.setTimeNote(s.sceneId, note || null).catch(reportError);
          }
        }}
      />
    </div>
  );
}

function AssignDayDialog({ count, onClose, onAssign }: { count: number; onClose: () => void; onAssign: (day: string | null) => void }) {
  const [day, setDay] = useState("");
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      size="sm"
      title="Assign Story Day"
      sub={`${count} scene${count === 1 ? "" : "s"} selected`}
      footer={
        <>
          <Button onClick={() => onAssign(null)}>Clear Story Day</Button>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!day.trim()} onClick={() => onAssign(day.trim())}>Assign</Button>
        </>
      }
    >
      <Field label="Story Day" htmlFor="story-day" hint="For example 1, 2, 3 — or a label such as Flashback.">
        <TextInput id="story-day" value={day} onChange={(e) => setDay(e.target.value)} autoFocus onKeyDown={(e) => e.key === "Enter" && day.trim() && onAssign(day.trim())} />
      </Field>
    </Dialog>
  );
}
