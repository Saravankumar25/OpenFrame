// Production Source dialogs (FSD §53, §160–161; UX §3.21 mock 103, §3.22 mock 111).
// - Select Source: choose the named draft production is based on.
// - Update Production Source: reconciliation preview by stable scene identity;
//   nothing changes until the user confirms, and nothing is deleted.

import { useState } from "react";
import { Check, FileText, Lock } from "lucide-react";
import { useCommand, useOp } from "../../../ipc/query";
import type { ProductionSourceOptions } from "../../../ipc/generated/ProductionSourceOptions";
import type { ProductionSourceInfo } from "../../../ipc/generated/ProductionSourceInfo";
import type { ProductionUpdatePreview } from "../../../ipc/generated/ProductionUpdatePreview";
import type { ProductionUpdateResult } from "../../../ipc/generated/ProductionUpdateResult";
import type { ProductionSceneChange } from "../../../ipc/generated/ProductionSceneChange";
import type { ProductionDraftInfo } from "../../../ipc/generated/ProductionDraftInfo";
import { Banner, Button, Chip, Dialog, Field, Skeleton, TextInput, type ChipTone } from "../../../design-system";
import { PROD_TABLES } from "../../../api/production";
import { toast } from "../../../app/toast";

function fmtDate(ms: number | null): string {
  return ms ? new Date(ms).toLocaleDateString(undefined, { day: "numeric", month: "short", year: "numeric" }) : "";
}

export function draftSummary(d: ProductionDraftInfo): string {
  const scenes = `${d.sceneCount} scene${d.sceneCount === 1 ? "" : "s"}`;
  return d.status === "Locked" ? `${scenes} · locked ${fmtDate(d.lockedAt)}` : `Not locked · ${scenes}`;
}

/** Choose the draft production is based on. When a source exists, choosing
 *  another draft opens the update review instead of replacing it. */
export function SelectSourceDialog({ open, onOpenChange, onReviewUpdate }: {
  open: boolean;
  onOpenChange: (v: boolean) => void;
  onReviewUpdate: (draftId: string) => void;
}) {
  const opts = useOp<ProductionSourceOptions>("production.drafts", {}, PROD_TABLES, { enabled: open });
  const [picked, setPicked] = useState<string | null>(null);
  const [reason, setReason] = useState("");
  const setSource = useCommand<{ draftId: string; reason: string | null }, ProductionSourceInfo>("production.set_source", {
    onSuccess: (s) => {
      toast.undoable(`Production Source set to “${s.draft.name}”. Your screenplay is not changed.`);
      onOpenChange(false);
    },
  });
  const drafts = opts.data?.drafts ?? [];
  const active = opts.data?.activeDraftId ?? null;
  const choice = picked ?? active ?? drafts.find((d) => d.status === "Locked")?.id ?? drafts[0]?.id ?? null;
  const multipleScreenplays = new Set(drafts.map((d) => d.screenplayId)).size > 1;

  const confirm = () => {
    if (!choice) return;
    if (active && choice !== active) {
      onOpenChange(false);
      onReviewUpdate(choice);
      return;
    }
    setSource.mutate({ draftId: choice, reason: reason.trim() || null });
  };

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      size="md"
      title="Handing the script over to production."
      sub="Which screenplay draft should become the production source?"
      footer={
        <>
          <Button onClick={() => onOpenChange(false)}>Cancel</Button>
          <Button variant="primary" disabled={!choice || choice === active || setSource.isPending} onClick={confirm}>
            {active ? "Review Production Update" : "Set as Production Source"}
          </Button>
        </>
      }
    >
      {opts.isLoading ? (
        <Skeleton h={60} />
      ) : drafts.length === 0 ? (
        <Banner tone="info">There are no screenplay drafts yet. Write or import a screenplay first — production is always based on a named draft.</Banner>
      ) : (
        <div role="radiogroup" aria-label="Screenplay drafts" style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {drafts.map((d) => {
            const on = d.id === choice;
            return (
              <button
                key={d.id}
                type="button"
                role="radio"
                aria-checked={on}
                className="card pad"
                onClick={() => setPicked(d.id)}
                style={{ textAlign: "left", display: "flex", gap: 10, alignItems: "center", outline: on ? "2px solid var(--accent)" : undefined }}
              >
                <FileText size={18} aria-hidden />
                <span className="grow">
                  <span className="b">{d.name}</span>
                  {multipleScreenplays && <span className="muted"> · {d.screenplayTitle}</span>}
                  <span className="xs muted" style={{ display: "block" }}>{draftSummary(d)}</span>
                </span>
                {d.status === "Locked" && <Chip tone="g"><Lock size={11} aria-hidden /> Locked</Chip>}
                {d.id === active && <Chip tone="a">Current source</Chip>}
                {on && <Check size={16} aria-label="Selected" />}
              </button>
            );
          })}
        </div>
      )}
      {!active && drafts.length > 0 && (
        <>
          <div className="h4" style={{ marginTop: 14 }}>OpenFrame will create or initialise</div>
          <ul className="sm" style={{ margin: "0 0 8px 18px", padding: 0, lineHeight: 1.6 }}>
            <li>Breakdown scenes — one per screenplay scene, numbered automatically</li>
            <li>Production catalog</li>
            <li>The scene list used by shot lists, storyboards and the schedule</li>
          </ul>
          <Field label="Why this draft? (optional)" htmlFor="src-reason">
            <TextInput id="src-reason" value={reason} onChange={(e) => setReason(e.target.value)} placeholder="e.g. Locked shooting draft" maxLength={1000} />
          </Field>
        </>
      )}
      <div className="xs muted" style={{ marginTop: 8 }}>You will not recreate any scene by hand. Your screenplay is not changed.</div>
    </Dialog>
  );
}

function changeTag(c: ProductionSceneChange): { label: string; tone: ChipTone } {
  switch (c.status) {
    case "added":
      return { label: "Needs Breakdown", tone: "r" };
    case "removed":
      return { label: "Historical", tone: "default" };
    default:
      return { label: "Review", tone: "y" };
  }
}

function changeText(c: ProductionSceneChange): string {
  switch (c.status) {
    case "added":
      return "New scene";
    case "removed":
      return c.elementCount > 0 ? `Removed — its breakdown (${c.elementCount}) is kept as history` : "Removed from the script";
    case "ambiguous":
      return "Scene identity is ambiguous (duplicated) — breakdown stays on the old scene for review";
    case "moved":
      return `Moved (was Scene ${c.oldNumber})`;
    default: {
      const parts = [];
      if (c.headingChanged) parts.push(`heading changed (was “${c.oldHeading ?? ""}”)`);
      if (c.textChanged) parts.push("text changed");
      if (c.moved) parts.push(`moved from Scene ${c.oldNumber}`);
      return parts.join(" · ").replace(/^./, (x) => x.toUpperCase());
    }
  }
}

/** "Update Production from Draft 7" (mock 111). */
export function UpdateSourceDialog({ draftId, onClose }: { draftId: string | null; onClose: () => void }) {
  const open = draftId !== null;
  const preview = useOp<ProductionUpdatePreview>("production.preview_update", { draftId }, PROD_TABLES, { enabled: open });
  const [showAll, setShowAll] = useState(false);
  const apply = useCommand<{ draftId: string }, ProductionUpdateResult>("production.apply_update", {
    onSuccess: (r) => {
      toast.undoable(
        `Production now uses “${r.source.draft.name}”. ${r.addedScenes} new scene${r.addedScenes === 1 ? "" : "s"} need breakdown; nothing was deleted.`,
      );
      onClose();
    },
  });
  const p = preview.data;
  const tiles: [string, number][] = p
    ? [
        ["Scenes added", p.added],
        ["Scenes removed", p.removed],
        ["Text changed", p.textChanged],
        ["Heading changed", p.headingChanged],
      ]
    : [];
  const rows = p ? (showAll ? p.changes : p.changes.slice(0, 8)) : [];
  return (
    <Dialog
      open={open}
      onOpenChange={(v) => !v && onClose()}
      size="lg"
      title={p ? `Update Production from ${p.to.name}` : "Update Production Source"}
      sub={
        p
          ? `Production is currently based on ${p.from.name}. ${p.to.createdAt > p.from.createdAt ? "A newer revision is available." : "You chose a different draft."} Nothing changes until you confirm.`
          : undefined
      }
      footerLeft={<Button variant="ghost" onClick={onClose}>Dismiss</Button>}
      footer={
        <>
          <Button onClick={onClose}>{p ? `Keep Production on ${p.from.name}` : "Cancel"}</Button>
          {p && p.changes.length > 8 && !showAll && <Button onClick={() => setShowAll(true)}>Review Changes First</Button>}
          <Button variant="primary" disabled={!p || apply.isPending} onClick={() => draftId && apply.mutate({ draftId })}>
            Update Production Baseline
          </Button>
        </>
      }
    >
      {preview.isLoading && <Skeleton h={120} />}
      {preview.error && <Banner tone="err">{preview.error.message}</Banner>}
      {p && (
        <>
          <div className="grid g4" style={{ marginBottom: 12 }}>
            {tiles.map(([label, n]) => (
              <div key={label} className="card pad">
                <div className="h4">{label}</div>
                <div style={{ fontSize: 22, fontWeight: 700 }}>{n}</div>
              </div>
            ))}
          </div>
          {(p.moved > 0 || p.ambiguous > 0) && (
            <div className="sm muted" style={{ marginBottom: 8 }}>
              {p.moved > 0 && `${p.moved} moved · `}
              {p.ambiguous > 0 && `${p.ambiguous} need a manual match · `}
              {p.unchanged} unchanged
            </div>
          )}
          {p.changes.length === 0 ? (
            <Banner tone="ok">No scene changes. Breakdown data carries over as it is.</Banner>
          ) : (
            <div className="card" style={{ maxHeight: 300, overflow: "auto" }}>
              {rows.map((c) => {
                const t = changeTag(c);
                return (
                  <div key={`${c.lineageId}-${c.newSceneId ?? c.oldSceneId}`} className="li">
                    <b style={{ width: 34, color: "var(--muted)" }}>{c.newNumber ?? c.oldNumber}</b>
                    <span className="grow sm">
                      <b>{c.heading || "(no heading)"}</b>
                      <span className="muted"> — {changeText(c)}</span>
                    </span>
                    <Chip tone={t.tone}>{t.label}</Chip>
                  </div>
                );
              })}
              {!showAll && p.changes.length > rows.length && (
                <div className="li sm muted">…and {p.changes.length - rows.length} more.</div>
              )}
            </div>
          )}
          <div className="sm muted" style={{ marginTop: 10 }}>
            Nothing is deleted: all {p.elementsKept} breakdown elements are kept. Removed scenes keep their breakdown as history; changed scenes are
            flagged for review.
          </div>
        </>
      )}
    </Dialog>
  );
}
