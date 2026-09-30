// Production Overview (FSD §108; UX §3.21, mock 102): "What do I need to do
// next to prepare the shoot?" Everything here is derived; every item opens
// its source workspace; no metric is a prerequisite.

import { useState, type ReactNode } from "react";
import { useOp } from "../../../ipc/query";
import type { ProductionOverview } from "../../../ipc/generated/ProductionOverview";
import type { ProductionSceneRef } from "../../../ipc/generated/ProductionSceneRef";
import { Button, Chip, PageHeader, Skeleton, Banner } from "../../../design-system";
import { PROD_TABLES, useCanEdit } from "../../../api/production";
import { useNav, type Route } from "../../../app/stores";
import { SelectSourceDialog, UpdateSourceDialog } from "../components/SourceDialogs";

export const tab = { id: "overview", label: "Overview", order: 0 };

function sceneList(refs: ProductionSceneRef[]): string {
  const nums = refs.map((r) => r.number).filter(Boolean) as string[];
  if (nums.length <= 1) return nums[0] ?? "";
  if (nums.length > 6) return `${nums.slice(0, 5).join(", ")} and ${nums.length - 5} more`;
  return `${nums.slice(0, -1).join(", ")} and ${nums[nums.length - 1]}`;
}

function Tile({ title, value, sub, onOpen, children }: { title: string; value: ReactNode; sub: string; onOpen: () => void; children?: ReactNode }) {
  return (
    <button type="button" className="card pad" style={{ display: "block", textAlign: "left" }} onClick={onOpen}>
      <div className="h4">{title}</div>
      <div style={{ fontSize: 22, fontWeight: 700 }}>{value}</div>
      <div className="xs muted">{sub}</div>
      {children}
    </button>
  );
}

export default function OverviewTab() {
  const go = useNav((s) => s.go);
  const canEdit = useCanEdit();
  const ov = useOp<ProductionOverview>("production.overview", {}, [...PROD_TABLES, "project_member"]);
  const [selectOpen, setSelectOpen] = useState(false);
  const [updateDraft, setUpdateDraft] = useState<string | null>(null);
  const open = (r: Route) => go(r);
  const o = ov.data;
  const src = o?.source ?? null;

  const attention: { key: string; title: string; detail: string; action: string; run: () => void }[] = [];
  if (o && src?.newer) {
    const newer = src.newer;
    attention.push({
      key: "newer",
      title: "Newer revision available",
      detail: `${newer.name} — production stays on ${src.draft.name} until you review the update.`,
      action: "Review",
      run: () => setUpdateDraft(newer.id),
    });
  }
  if (o && o.needsReview.length > 0) {
    attention.push({
      key: "review",
      title: `${o.needsReview.length} scene${o.needsReview.length === 1 ? " needs" : "s need"} breakdown review`,
      detail: `Script changed in Scene${o.needsReview.length === 1 ? "" : "s"} ${sceneList(o.needsReview)}.`,
      action: "Review",
      run: () => open({ workspace: "breakdown", params: { sceneId: o.needsReview[0].sceneId } }),
    });
  }
  if (o && o.needsBreakdown.length > 0) {
    attention.push({
      key: "breakdown",
      title: `${o.needsBreakdown.length} new scene${o.needsBreakdown.length === 1 ? " needs" : "s need"} breakdown`,
      detail: `Scene${o.needsBreakdown.length === 1 ? "" : "s"} ${sceneList(o.needsBreakdown)} arrived with a script update.`,
      action: "Open",
      run: () => open({ workspace: "breakdown", params: { sceneId: o.needsBreakdown[0].sceneId } }),
    });
  }
  if (o && o.historicalElements > 0) {
    attention.push({
      key: "hist",
      title: `${o.historicalElements} breakdown element${o.historicalElements === 1 ? "" : "s"} from removed scenes`,
      detail: "Kept from scenes that are no longer in the Production Source. Archive them when you no longer need them.",
      action: "Review",
      run: () => open({ workspace: "breakdown" }),
    });
  }
  if (o && o.unresolvedCharacters.length > 0) {
    attention.push({
      key: "cast",
      title: "Needs casting",
      detail: o.unresolvedCharacters.slice(0, 6).join(", ") + (o.unresolvedCharacters.length > 6 ? ` and ${o.unresolvedCharacters.length - 6} more` : ""),
      action: "Open",
      run: () => open({ workspace: "production", sub: "cast-crew" }),
    });
  }

  return (
    <div>
      <PageHeader
        title="Production"
        sub="What do I need to do next to prepare the shoot?"
        actions={
          canEdit && (
            <Button variant={src ? "default" : "primary"} onClick={() => setSelectOpen(true)}>
              {src ? "Change Production Source" : "Start Production Setup"}
            </Button>
          )
        }
      />
      {ov.isLoading && <Skeleton h={140} />}
      {ov.error && <Banner tone="err">{ov.error.message}</Banner>}
      {o && (
        <>
          {!src && (
            <div style={{ marginBottom: 12 }}>
              <Banner tone="info">
                Production starts from a named screenplay draft. Choose it with <b>Start Production Setup</b> — your screenplay is not changed, and you will not recreate any scene by hand.
              </Banner>
            </div>
          )}
          <div className="grid g3" style={{ marginBottom: 12 }}>
            <Tile
              title="Script status"
              value={src ? src.draft.name : "No source yet"}
              sub={src ? `${src.draft.status === "Locked" ? "Locked" : "Not locked"} · used as Production Source` : "Choose the draft production is based on"}
              onOpen={() => (src ? open({ workspace: "breakdown" }) : setSelectOpen(true))}
            >
              {src?.draft.status === "Locked" && <div style={{ marginTop: 6 }}><Chip tone="g">Locked</Chip></div>}
            </Tile>
            <Tile
              title="Breakdown progress"
              value={`${o.completeCount} / ${o.sceneCount}`}
              sub={`scenes marked complete · ${o.plannedCount} with confirmed elements${o.pendingSuggestions ? ` · ${o.pendingSuggestions} suggestions to review` : ""}`}
              onOpen={() => open({ workspace: "breakdown" })}
            >
              <div className="bar" style={{ marginTop: 6 }} role="progressbar" aria-valuemin={0} aria-valuemax={o.sceneCount} aria-valuenow={o.completeCount} aria-label="Scenes marked complete">
                <i style={{ width: `${o.sceneCount ? Math.round((o.completeCount / o.sceneCount) * 100) : 0}%` }} />
              </div>
            </Tile>
            <Tile
              title="Unresolved locations"
              value={o.unresolvedLocations}
              sub={o.locationCount ? `still Idea or Shortlisted (of ${o.locationCount})` : "No locations yet"}
              onOpen={() => open({ workspace: "production", sub: "locations" })}
            />
            <Tile
              title="Unresolved cast"
              value={o.unresolvedCharacters.length}
              sub={`character${o.unresolvedCharacters.length === 1 ? "" : "s"} without an actor (of ${o.characterCount})`}
              onOpen={() => open({ workspace: "production", sub: "cast-crew" })}
            />
            <Tile
              title="Catalog"
              value={o.catalogCount}
              sub="reusable production items"
              onOpen={() => open({ workspace: "production", sub: "catalog" })}
            />
          </div>
          <div className="h4">Needs attention</div>
          <div className="card">
            {attention.length === 0 ? (
              <div className="li sm muted">Nothing needs attention right now.</div>
            ) : (
              attention.map((a) => (
                <div key={a.key} className="li">
                  <span className="grow sm">
                    <b>{a.title}</b> — {a.detail}
                  </span>
                  <Button size="xs" onClick={a.run}>{a.action}</Button>
                </div>
              ))
            )}
          </div>
        </>
      )}
      <SelectSourceDialog open={selectOpen} onOpenChange={setSelectOpen} onReviewUpdate={setUpdateDraft} />
      <UpdateSourceDialog draftId={updateDraft} onClose={() => setUpdateDraft(null)} />
    </div>
  );
}
