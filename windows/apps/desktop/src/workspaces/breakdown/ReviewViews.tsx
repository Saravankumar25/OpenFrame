// "Needs Review" queue (mock 112) and the historical breakdown of scenes that
// are no longer in the Production Source (FSD §54: kept, never deleted).

import { useState } from "react";
import { ArrowLeft } from "lucide-react";
import { call } from "../../ipc/client";
import { reportError } from "../../ipc/query";
import type { BreakdownSceneRow } from "../../ipc/generated/BreakdownSceneRow";
import type { BreakdownHistoricalScene } from "../../ipc/generated/BreakdownHistoricalScene";
import { Button, Checkbox, Chip, EmptyState, PageHeader } from "../../design-system";
import { toast } from "../../app/toast";

function when(ms: number | null): string {
  return ms ? new Date(ms).toLocaleString(undefined, { day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" }) : "";
}

export function NeedsReviewView({ rows, onBack, onReview }: {
  rows: BreakdownSceneRow[];
  onBack: () => void;
  onReview: (sceneId: string) => void;
}) {
  const list = rows.filter((r) => r.needsReview);
  return (
    <div className="queue">
      <PageHeader
        title="Needs Review"
        sub="Scenes whose script changed after production planning began."
        actions={<Button icon={<ArrowLeft size={14} />} onClick={onBack}>Back to Breakdown</Button>}
      />
      {list.length === 0 ? (
        <EmptyState title="Nothing to review.">Every planned scene matches the Production Source text.</EmptyState>
      ) : (
        <div className="card">
          {list.map((r) => (
            <div key={r.sceneId} className="li">
              <b style={{ width: 34, color: "var(--muted)" }}>{r.number}</b>
              <span className="grow sm">
                <b>{r.heading}</b>
                <span className="muted" style={{ display: "block" }}>
                  {r.changeMessage} {r.changedAt ? `Changed ${when(r.changedAt)}.` : ""} {r.confirmedCount} element{r.confirmedCount === 1 ? "" : "s"} kept.
                </span>
              </span>
              {r.headingChanged && <Chip tone="y">Heading changed</Chip>}
              {r.textChanged && <Chip tone="y">Text changed</Chip>}
              <Button size="xs" onClick={() => onReview(r.sceneId)}>Review</Button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export function HistoricalView({ scenes, canEdit, onBack, onOpen }: {
  scenes: BreakdownHistoricalScene[];
  canEdit: boolean;
  onBack: () => void;
  onOpen: (sceneId: string) => void;
}) {
  const [showArchived, setShowArchived] = useState(false);
  const setArchived = (id: string, archived: boolean, name: string) =>
    void call("breakdown.set_archived", { id, archived })
      .then(() => toast.undoable(archived ? `Archived “${name}”` : `Unarchived “${name}”`))
      .catch(reportError);
  const visible = scenes
    .map((s) => ({ ...s, elements: s.elements.filter((e) => showArchived || !e.archived) }))
    .filter((s) => s.elements.length > 0);
  return (
    <div className="queue">
      <PageHeader
        title="Removed scenes"
        sub="Scene removed from current source. Production data is kept here so nothing is lost; archive what you no longer need."
        actions={<Button icon={<ArrowLeft size={14} />} onClick={onBack}>Back to Breakdown</Button>}
      />
      <div style={{ marginBottom: 10 }}>
        <Checkbox checked={showArchived} onChange={setShowArchived} label="Show archived" />
      </div>
      {visible.length === 0 ? (
        <EmptyState title="Nothing kept from removed scenes.">When a script update removes a scene, its breakdown stays here.</EmptyState>
      ) : (
        visible.map((s) => (
          <section key={s.sceneId} className="bd-cat" aria-label={s.heading}>
            <div className="bh">
              {s.heading || "(no heading)"}
              <span className="cn">{s.draftName ? `from ${s.draftName}` : ""}</span>
              <Button size="xs" onClick={() => onOpen(s.sceneId)}>Open</Button>
            </div>
            {s.elements.map((e) => (
              <div key={e.id} className={`bd-el${e.archived ? " hist" : ""}`}>
                <span className="dot gray" aria-hidden />
                <b className="grow">{e.name}</b>
                <span className="xs muted">{e.category}</span>
                {e.archived && <Chip>Archived</Chip>}
                {canEdit && (
                  <Button size="xs" onClick={() => setArchived(e.id, !e.archived, e.name)}>{e.archived ? "Unarchive" : "Archive"}</Button>
                )}
              </div>
            ))}
          </section>
        ))
      )}
    </div>
  );
}
