// Production › Sides (FSD §58, §111, FSD-PROD-024; UX §3.32; mock 148).
// Sides contain only the chosen day's screenplay scenes from the production
// source. Saving creates a snapshot; the screenplay is never changed.

import { useEffect, useMemo, useState } from "react";
import { FileText, Trash2 } from "lucide-react";
import { Banner, Button, Checkbox, EmptyState, IconButton, PageHeader, Segmented, Select, Skeleton } from "../../../design-system";
import { call } from "../../../ipc/client";
import { reportError, useOp } from "../../../ipc/query";
import { toast } from "../../../app/toast";
import { useNav } from "../../../app/stores";
import type { ScheduleView, ScheduleSideDetail, ScheduleSideRow, ScheduleSidesContent } from "../../../api/schedule";
import { SCHEDULE_TABLES, SIDES_TABLES, formatDateTime } from "../../../api/schedule";
import "../../call-sheets/shared/schedule-ui.css";
import { SidesExportButton } from "../../../features/export/buttons";

export const tab = { id: "sides", label: "Sides", order: 90 };

const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;

const ELEMENT_CLASS: Record<string, string> = {
  action: "sp-a",
  character: "sp-c",
  dialogue: "sp-d",
  parenthetical: "sp-p",
  transition: "sp-t",
  shot: "sp-sh",
};

function SidesPaper({ content }: { content: ScheduleSidesContent }) {
  return (
    <div className="paper of-sides" aria-label="Sides preview">
      <div style={{ textAlign: "center", fontWeight: 700, marginBottom: 10 }}>{content.header}</div>
      {content.cover && (
        <div style={{ borderBottom: "1px solid #bbb" /* document colour: printed sides */, paddingBottom: 10, marginBottom: 10 }}>
          <div className="st">Call sheet cover</div>
          <div>
            {content.dayLabel}
            {content.dateLabel ? ` · ${content.dateLabel}` : ""}
            {content.cover.crewCall ? ` · CREW CALL ${content.cover.crewCall}` : ""}
          </div>
          {content.cover.locations.map((l) => (
            <div key={l}>{l}</div>
          ))}
          {content.cover.cast.length > 0 && (
            <table style={{ marginTop: 4 }}>
              <tbody>
                {content.cover.cast.map((c) => (
                  <tr key={c.character}>
                    <td>{c.actor ?? "Needs input"}</td>
                    <td>{c.character}</td>
                    <td>{c.callTime ?? ""}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
          {content.cover.notes && <div style={{ marginTop: 4, whiteSpace: "pre-wrap" }}>{content.cover.notes}</div>}
        </div>
      )}
      {content.scenes.map((s) => (
        <section key={s.sceneId} aria-label={`Scene ${s.number}`}>
          <div className="sp-h">
            {s.number}. {s.heading}
          </div>
          {s.elements
            .filter((e) => e.elementType !== "scene_heading")
            .map((e, i) => (
              <div key={i} className={ELEMENT_CLASS[e.elementType] ?? "sp-a"}>
                {e.text}
              </div>
            ))}
        </section>
      ))}
      {content.showDraftName && <div className="xs" style={{ marginTop: 14, textAlign: "center" }}>Source: {content.sourceLabel}</div>}
    </div>
  );
}

export default function SidesTab() {
  const route = useNav((s) => s.route);
  const go = useNav((s) => s.go);
  const sched = useOp<ScheduleView>("schedule.get", {}, SCHEDULE_TABLES);
  const saved = useOp<ScheduleSideRow[]>("sides.list", {}, ["side"]);
  const days = useMemo(() => (sched.data?.days ?? []).filter((d) => !d.isOffDay), [sched.data]);
  const [dayId, setDayId] = useState("");
  const [selected, setSelected] = useState<string[] | null>(null);
  const [forMode, setForMode] = useState<"all" | "selected">("all");
  const [castKeys, setCastKeys] = useState<string[]>([]);
  const [cover, setCover] = useState(false);
  const [draftName, setDraftName] = useState(true);
  const openId = route.params?.sideId;

  useEffect(() => {
    if (!dayId && days.length) setDayId(days[0].id);
  }, [days, dayId]);
  const day = days.find((d) => d.id === dayId);
  const strips = useMemo(() => (day ? day.items.flatMap((i) => (i.strip ? [i.strip] : [])) : []), [day]);
  const chosen = selected ?? strips.map((s) => s.id);
  // "Selected cast": the same rule the backend applies (a scene counts when one of its confirmed
  // breakdown cast keys is picked), worked out here so the checklist, the preview and the export agree.
  const filtering = forMode === "selected" && castKeys.length > 0;
  const hasCast = (s: (typeof strips)[number]) => s.cast.some((c) => castKeys.includes(c.key));
  const included = (s: (typeof strips)[number]) => !filtering || hasCast(s);
  const sceneCount = strips.filter(included).length;
  // Ticks the user made are kept for scenes left out, so "All cast" brings them back unchanged.
  const effective = strips.filter((s) => included(s) && chosen.includes(s.id)).map((s) => s.id);
  const args = {
    dayId: dayId || null,
    stripIds: effective,
    castKeys: forMode === "selected" ? castKeys : null,
    includeCover: cover,
    showDraftName: draftName,
  };
  const ready = !!day && effective.length > 0 && (forMode === "all" || castKeys.length > 0);
  const preview = useOp<ScheduleSidesContent>("sides.preview", args, SIDES_TABLES, { enabled: ready && !openId });
  const openSaved = useOp<ScheduleSideDetail>("sides.get", { id: openId ?? "" }, ["side"], { enabled: !!openId });

  if (sched.isLoading) return <div><Skeleton h={28} w={240} /></div>;
  if (!sched.data?.schedule) {
    return (
      <div className="of-tab">
        <EmptyState icon={<FileText size={40} />} title="Sides need a shooting schedule." actions={<Button onClick={() => go({ workspace: "production", sub: "schedule" })}>Open Schedule</Button>}>
          Create a shooting schedule and a shooting day first. Sides contain that day's screenplay scenes.
        </EmptyState>
      </div>
    );
  }
  const dayCast = day?.summary.cast ?? [];
  const save = async () => {
    try {
      const id = await call<string>("sides.create", args);
      toast.undoable("Saved sides");
      go({ workspace: "production", sub: "sides", params: { sideId: id } });
    } catch (e) {
      reportError(e);
    }
  };
  return (
    <div className="of-tab">
      <PageHeader title="Sides" sub="What script pages does this team need for this shooting day?" />
      <div className="row" style={{ alignItems: "stretch", gap: 14, flex: 1, minHeight: 0 }}>
        <div className="card pad of-scroll" style={{ width: 320, flex: "none" }}>
          <div className="h4">Setup</div>
          {days.length === 0 ? (
            <div className="sm muted">Create a shooting day first.</div>
          ) : (
            <>
              <label className="field">
                <span className="sm b">Shooting day</span>
                <Select
                  ariaLabel="Shooting day"
                  value={dayId}
                  onChange={(v) => {
                    setDayId(v);
                    setSelected(null);
                    setCastKeys([]);
                  }}
                  options={days.map((d) => ({ value: d.id, label: d.title }))}
                />
              </label>
              <div className="sm b" style={{ marginBottom: 4 }}>Scenes</div>
              {strips.length === 0 && <div className="sm muted">No scenes scheduled on this day.</div>}
              {strips.map((s) => {
                const left = !included(s);
                return (
                  <Checkbox
                    key={s.id}
                    checked={!left && chosen.includes(s.id)}
                    disabled={left}
                    onChange={(v) => setSelected(v ? [...chosen, s.id] : chosen.filter((x) => x !== s.id))}
                    label={
                      <>
                        {s.number ?? "Not numbered"}: {s.heading}
                        {left && <span className="xs muted"> · Left out: no selected cast in this scene</span>}
                      </>
                    }
                  />
                );
              })}
              {filtering && (
                <div className="hint" role="status" style={{ marginTop: 4 }}>
                  {sceneCount === 0
                    ? "No scene on this day has the selected cast. Choose other cast, or switch to All cast."
                    : `${sceneCount} of ${plural(strips.length, "scene")} have the selected cast. Only those are in the sides.`}
                </div>
              )}
              <div className="sm b" style={{ margin: "10px 0 4px" }}>For</div>
              <Segmented
                ariaLabel="Sides for"
                value={forMode}
                onChange={setForMode}
                options={[
                  { value: "all", label: "All cast" },
                  { value: "selected", label: "Selected cast" },
                ]}
              />
              {forMode === "selected" && (
                <div style={{ marginTop: 6 }}>
                  {dayCast.length === 0 && <div className="sm muted">No confirmed cast on this day.</div>}
                  {dayCast.map((c) => (
                    <Checkbox
                      key={c.key}
                      checked={castKeys.includes(c.key)}
                      onChange={(v) => setCastKeys(v ? [...castKeys, c.key] : castKeys.filter((k) => k !== c.key))}
                      label={`${c.actor ? `${c.actor} (${c.character})` : c.character} · ${plural(strips.filter((s) => s.cast.some((x) => x.key === c.key)).length, "scene")}`}
                    />
                  ))}
                </div>
              )}
              <div className="hr" />
              <Checkbox checked={cover} onChange={setCover} label="Include call-sheet cover page" />
              <Checkbox checked={draftName} onChange={setDraftName} label="Show draft / revision name" />
              <div className="row" style={{ marginTop: 10 }}>
                <Button onClick={() => { if (openId) go({ workspace: "production", sub: "sides" }); else void preview.refetch(); }} disabled={!ready}>
                  Preview
                </Button>
                <Button variant="primary" onClick={save} disabled={!ready}>
                  Save Sides
                </Button>
                <SidesExportButton args={args} documentName={`Sides - ${day?.title ?? "Selected scenes"}`} disabled={!ready} />
              </div>
              <div className="hint" style={{ marginTop: 6 }}>Saved sides are snapshots. Later script edits don't change them.</div>
            </>
          )}
          {saved.data && saved.data.length > 0 && (
            <>
              <div className="hr" />
              <div className="h4">Saved sides</div>
              {saved.data.map((s) => (
                <div key={s.id} className={`li${s.id === openId ? " sel" : ""}`} style={{ padding: "6px 4px" }}>
                  <button type="button" className="btn ghost sm grow" style={{ justifyContent: "flex-start", whiteSpace: "normal", textAlign: "left" }} onClick={() => go({ workspace: "production", sub: "sides", params: { sideId: s.id } })}>
                    <span>
                      {s.title}
                      <br />
                      <span className="xs muted">
                        {s.sceneCount} scenes · {formatDateTime(s.createdAt)}
                      </span>
                    </span>
                  </button>
                  <IconButton
                    label={`Delete ${s.title}`}
                    onClick={async () => {
                      try {
                        await call("sides.delete", { id: s.id });
                        toast.undoable("Deleted saved sides");
                        if (openId === s.id) go({ workspace: "production", sub: "sides" });
                      } catch (e) {
                        reportError(e);
                      }
                    }}
                  >
                    <Trash2 size={14} />
                  </IconButton>
                </div>
              ))}
            </>
          )}
        </div>
        <div className="grow of-scroll">
          {openId ? (
            openSaved.data ? (
              <>
                <Banner
                  tone="info"
                  actions={
                    <>
                      <SidesExportButton size="xs" args={{ sideId: openSaved.data.id }} documentName={openSaved.data.title} />
                      <Button size="xs" onClick={() => go({ workspace: "production", sub: "sides" })}>Back to live preview</Button>
                    </>
                  }
                >
                  Saved snapshot “{openSaved.data.title}” · {formatDateTime(openSaved.data.createdAt)}
                </Banner>
                <div style={{ marginTop: 8 }}>
                  <SidesPaper content={openSaved.data.content} />
                </div>
              </>
            ) : openSaved.error ? (
              <Banner tone="err">{openSaved.error.message}</Banner>
            ) : (
              <Skeleton h={200} />
            )
          ) : !ready ? (
            <div className="sm muted" style={{ padding: 20 }}>
              {filtering && sceneCount === 0
                ? "No scene on this day has the selected cast, so there are no sides to show. Choose other cast, or switch to All cast. Cast comes from each scene’s confirmed breakdown."
                : filtering && effective.length === 0
                  ? "All the scenes with the selected cast are unticked. Tick at least one scene to preview the sides."
                  : `Choose a shooting day and at least one scene${forMode === "selected" ? " and cast member" : ""} to preview the sides.`}
            </div>
          ) : preview.error ? (
            <Banner tone="warn">{preview.error.message}</Banner>
          ) : preview.data ? (
            <SidesPaper content={preview.data} />
          ) : (
            <Skeleton h={200} />
          )}
        </div>
      </div>
    </div>
  );
}
