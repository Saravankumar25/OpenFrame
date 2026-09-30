// Export an exchange / review package (FSD §47.3–47.4, §50; UX §3.39 "Script
// Review Package", mockup 159). The dialog explains exactly what the package
// will contain before export; private notes are never allowed in it.

import { useEffect, useMemo, useState } from "react";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import { Lock } from "lucide-react";
import { reportError, useOp } from "../../ipc/query";
import type { PackageType } from "../../ipc/generated/PackageType";
import type { PackageExchangeSources } from "../../ipc/generated/PackageExchangeSources";
import type { PackageSceneOption } from "../../ipc/generated/PackageSceneOption";
import type { PackageExportSummary } from "../../ipc/generated/PackageExportSummary";
import type { PackageExportResult } from "../../ipc/generated/PackageExportResult";
import type { PackageExchangeArgs } from "../../ipc/generated/PackageExchangeArgs";
import { Banner, Button, Checkbox, Dialog, Field, Segmented, Select, Skeleton } from "../../design-system";
import { dayAndTime } from "../../app/home/format";
import { toast } from "../../app/toast";
import { PACKAGE_INFO, packageFilter, packagesApi } from "./api";
import { DoneBanner } from "./parts";

const TITLES: Partial<Record<PackageType, { title: string; sub: string }>> = {
  scriptReview: {
    title: "Script Review Package",
    sub: "How do I send this draft to someone for review and safely bring their notes back?",
  },
  story: { title: "Story Board Package", sub: "Which part of the Story Board do I share, and what comes with it?" },
  breakdown: { title: "Breakdown Package", sub: "Which scenes' breakdown do I share for review?" },
  shots: { title: "Shot List Package", sub: "Which shots do I share for review?" },
  schedule: { title: "Schedule Package", sub: "Share the shooting schedule for review. Returned changes never delete your days." },
  callReview: { title: "Call Sheet Review Package", sub: "Share a call sheet for feedback. The schedule is never changed by the reply." },
};

type Scope = "full" | "comments" | "scenes" | "act";

function scopeOptions(t: PackageType): { value: Scope; label: string }[] {
  switch (t) {
    case "scriptReview":
      return [
        { value: "full", label: "Full script" },
        { value: "comments", label: "Script + comments" },
        { value: "scenes", label: "Specific scenes only" },
      ];
    case "story":
      return [
        { value: "full", label: "Whole Story Board" },
        { value: "act", label: "One Act" },
      ];
    case "breakdown":
    case "shots":
      return [
        { value: "full", label: "All scenes" },
        { value: "scenes", label: "Specific scenes only" },
      ];
    default:
      return [];
  }
}

export function ExportExchangeDialog({ packageType, onClose }: { packageType: PackageType; onClose: () => void }) {
  const sources = useOp<PackageExchangeSources>("packages.exchange_sources", {}, ["*"]);
  const [draftId, setDraftId] = useState<string>("");
  const [scope, setScope] = useState<Scope>("full");
  const [sceneIds, setSceneIds] = useState<string[]>([]);
  const [actId, setActId] = useState<string>("");
  const [callSheetId, setCallSheetId] = useState<string>("");
  const [includeComments, setIncludeComments] = useState(true);
  const [includeAttachments, setIncludeAttachments] = useState(false);
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState<PackageExportResult | null>(null);
  const src = sources.data;

  // Sensible defaults once the sources are known.
  useEffect(() => {
    if (!src) return;
    if (!draftId && src.drafts.length) setDraftId((src.drafts.find((d) => d.isCurrent) ?? src.drafts[0]).id);
    if (!actId && src.acts.length) setActId(src.acts[0].id);
    if (!callSheetId && src.callSheets.length) setCallSheetId(src.callSheets[0].id);
  }, [src, draftId, actId, callSheetId]);

  const sceneDraft = packageType === "scriptReview" ? draftId || null : packageType === "breakdown" ? src?.productionSource?.id ?? null : null;
  const scenes = useOp<PackageSceneOption[]>("packages.exchange_scenes", { draftId: sceneDraft }, ["screenplay_scene"], {
    enabled: scope === "scenes",
  });

  const args: PackageExchangeArgs = useMemo(
    () => ({
      packageType,
      draftId: packageType === "scriptReview" ? draftId || null : null,
      scope: scopeOptions(packageType).length ? scope : null,
      sceneIds: scope === "scenes" ? sceneIds : [],
      actId: scope === "act" ? actId || null : null,
      callSheetId: packageType === "callReview" ? callSheetId || null : null,
      includeComments: includeComments || scope === "comments",
      includeAttachments,
      includePrivateNotes: false,
      path: null,
    }),
    [packageType, draftId, scope, sceneIds, actId, callSheetId, includeComments, includeAttachments],
  );
  const ready = !(scope === "scenes" && sceneIds.length === 0) && !(packageType === "scriptReview" && !draftId && (src?.drafts.length ?? 0) > 0);
  const preview = useOp<PackageExportSummary>("packages.exchange_preview", args, ["*"], { enabled: !!src && ready, retry: false });
  const summary = preview.data;
  const info = TITLES[packageType] ?? { title: PACKAGE_INFO[packageType].label, sub: "" };

  const exportNow = async () => {
    if (!summary) return;
    const chosen = await saveDialog({
      title: `Export ${PACKAGE_INFO[packageType].label}`,
      defaultPath: summary.suggestedFileName,
      filters: packageFilter([packageType], PACKAGE_INFO[packageType].label),
    });
    if (typeof chosen !== "string") return;
    setBusy(true);
    try {
      const r = await packagesApi.exportExchange({ ...args, path: chosen });
      setDone(r);
      toast.success(`Exported ${r.fileName}`);
    } catch (e) {
      reportError(e);
    } finally {
      setBusy(false);
    }
  };

  const toggleScene = (id: string, on: boolean) => setSceneIds((prev) => (on ? [...prev, id] : prev.filter((x) => x !== id)));

  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title={info.title}
      sub={info.sub}
      size="md"
      footer={
        done ? (
          <Button variant="primary" onClick={onClose}>Close</Button>
        ) : (
          <>
            <Button onClick={onClose}>Cancel</Button>
            <Button variant="primary" disabled={!summary || busy || !ready} onClick={() => void exportNow()}>
              {busy ? "Exporting…" : "Export Package…"}
            </Button>
          </>
        )
      }
    >
      {!src ? (
        <Skeleton h={160} />
      ) : done ? (
        <DoneBanner title="Package exported." path={done.path}>
          Send it by any normal file transfer. Nothing in the package changes your project, and later changes to your project don't change the package.
        </DoneBanner>
      ) : (
        <>
          {packageType === "scriptReview" && (
            <Field label="Source draft" htmlFor="pkg-draft">
              <Select
                id="pkg-draft"
                value={draftId}
                onChange={setDraftId}
                options={src.drafts.map((d) => ({ value: d.id, label: `${d.name}${d.isCurrent ? " (current)" : ""} · ${d.sceneCount} scenes` }))}
              />
            </Field>
          )}
          {packageType === "breakdown" && (
            <Field label="Production Source">
              <div className="sm">{src.productionSource?.label ?? "No Production Source has been chosen yet."}</div>
            </Field>
          )}
          {packageType === "schedule" && (
            <Field label="Schedule">
              <div className="sm">{src.schedule?.label ?? "This project has no shooting schedule yet."}</div>
            </Field>
          )}
          {packageType === "callReview" && (
            <Field label="Call sheet" htmlFor="pkg-cs">
              {src.callSheets.length ? (
                <Select id="pkg-cs" value={callSheetId} onChange={setCallSheetId} options={src.callSheets.map((c) => ({ value: c.id, label: c.label }))} />
              ) : (
                <div className="sm muted">This project has no call sheet yet.</div>
              )}
            </Field>
          )}
          {scopeOptions(packageType).length > 0 && (
            <Field label="Scope">
              <Segmented ariaLabel="Scope" value={scope} onChange={setScope} options={scopeOptions(packageType)} />
            </Field>
          )}
          {scope === "act" && (
            <Field label="Act" htmlFor="pkg-act">
              <Select id="pkg-act" value={actId} onChange={setActId} options={src.acts.map((a) => ({ value: a.id, label: a.label }))} />
            </Field>
          )}
          {scope === "scenes" && (
            <div className="card pad" style={{ maxHeight: 180, overflow: "auto", marginBottom: 8 }}>
              {scenes.isLoading ? (
                <Skeleton h={60} />
              ) : (
                (scenes.data ?? []).map((s) => (
                  <Checkbox key={s.id} checked={sceneIds.includes(s.id)} onChange={(v) => toggleScene(s.id, v)} label={`${s.number}  ${s.heading}`} />
                ))
              )}
            </div>
          )}
          <div className="col" style={{ gap: 4, margin: "6px 0 10px" }}>
            <Checkbox checked={includeComments || scope === "comments"} disabled={scope === "comments"} onChange={setIncludeComments} label="Include comments" />
            <Checkbox
              checked={includeAttachments && (summary?.attachmentsAvailable ?? 0) > 0}
              disabled={(summary?.attachmentsAvailable ?? 0) === 0}
              onChange={setIncludeAttachments}
              label={`Include attachments${summary ? ` (${summary.attachmentsAvailable})` : ""}`}
            />
            <Checkbox checked={false} disabled onChange={() => undefined} label={<>Include private notes <span className="xs muted">— Not allowed in review packages</span></>} />
          </div>
          {preview.error ? (
            <Banner tone="warn">{preview.error.message}</Banner>
          ) : !summary ? (
            ready ? <Skeleton h={70} /> : <Banner tone="info">Choose at least one scene.</Banner>
          ) : (
            <div className="card pad">
              <div className="h4">This package will contain</div>
              <div className="sm">
                Project: <b>{summary.projectTitle}</b> · Target: <b>{summary.target}</b> · Scope: {summary.scopeLabel} · Exported by: {summary.exportedBy} · Date: {dayAndTime(summary.date)}
              </div>
              <div className="sm" style={{ marginTop: 4 }}>
                {summary.counts.map((c) => `${c.count} ${c.label.toLowerCase()}`).join(" · ")}
              </div>
              <div className="sm" style={{ marginTop: 4 }}>
                <Lock size={12} aria-hidden /> <b>Private notes: excluded</b>
              </div>
              <div className="xs muted" style={{ marginTop: 6 }}>{summary.explanation}</div>
            </div>
          )}
        </>
      )}
    </Dialog>
  );
}
