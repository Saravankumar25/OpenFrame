// Review Queue (Import/Export §16–§17, UX §3.39, mockup 163) and the import /
// export history: import sessions with their results, kept review records,
// open reviews and the package log.

import { useState } from "react";
import { useCommand, useOp, reportError } from "../../ipc/query";
import type { PackageQueueItem } from "../../ipc/generated/PackageQueueItem";
import type { PackageSceneOption } from "../../ipc/generated/PackageSceneOption";
import type { PackageSessionSummary } from "../../ipc/generated/PackageSessionSummary";
import type { PackageRecordSummary } from "../../ipc/generated/PackageRecordSummary";
import type { PackageLogEntry } from "../../ipc/generated/PackageLogEntry";
import type { PackageReviewSummary } from "../../ipc/generated/PackageReviewSummary";
import { Banner, Button, Chip, ConfirmDialog, Dialog, EmptyState, Segmented, Select, Skeleton } from "../../design-system";
import { dayAndTime } from "../../app/home/format";
import { toast } from "../../app/toast";
import { packagesApi, resultLabel } from "./api";
import { usePackagesUi } from "./store";

function QueueRow({ item, scenes }: { item: PackageQueueItem; scenes: PackageSceneOption[] }) {
  const [choosing, setChoosing] = useState(false);
  const [target, setTarget] = useState<string>(scenes[0]?.id ?? "");
  const [busy, setBusy] = useState(false);
  const attach = async (sceneId: string) => {
    setBusy(true);
    try {
      const r = await packagesApi.queueAttach(item.id, sceneId);
      toast.success(`Attached to ${r.attachedLabel ?? "the scene"}.`);
    } catch (e) {
      reportError(e);
    } finally {
      setBusy(false);
    }
  };
  const attached = item.status === "Attached";
  return (
    <div className="card pad" style={{ marginBottom: 8, opacity: attached ? 0.65 : 1 }}>
      <div className="sm">
        <b>“{item.body}”</b> <span className="muted">— {item.authorName}</span>
      </div>
      {item.quotedText && <div className="xs muted" style={{ fontStyle: "italic" }}>on “{item.quotedText}”</div>}
      <div className="xs muted">{[item.sourceLabel, item.reason].filter(Boolean).join(" · ")}</div>
      {attached ? (
        <div className="xs" style={{ marginTop: 4 }}><Chip tone="g">Attached</Chip> {item.attachedLabel}</div>
      ) : (
        <div className="row wrap" style={{ marginTop: 6 }}>
          {item.candidates.map((c) => (
            <Button key={c.sceneId} size="sm" disabled={busy} onClick={() => void attach(c.sceneId)}>
              Attach to {c.label.split(" · ")[0]}
            </Button>
          ))}
          {!choosing ? (
            <Button size="sm" disabled={busy || scenes.length === 0} onClick={() => setChoosing(true)}>
              {item.kind === "ambiguous" ? "Choose…" : "Attach manually…"}
            </Button>
          ) : (
            <>
              <Select ariaLabel="Scene" value={target} onChange={setTarget} options={scenes.map((s) => ({ value: s.id, label: `${s.number} · ${s.heading}` }))} />
              <Button size="sm" variant="primary" disabled={busy || !target} onClick={() => void attach(target)}>Attach</Button>
              <Button size="sm" variant="ghost" onClick={() => setChoosing(false)}>Cancel</Button>
            </>
          )}
        </div>
      )}
    </div>
  );
}

export function ReviewQueueDialog({ onClose }: { onClose: () => void }) {
  const queue = useOp<PackageQueueItem[]>("packages.review_queue", {}, ["review_queue_item"]);
  const scenes = useOp<PackageSceneOption[]>("packages.exchange_scenes", {}, ["screenplay_scene", "screenplay"]);
  const items = queue.data ?? [];
  const pending = items.filter((i) => i.status === "Pending");
  const ambiguous = pending.filter((i) => i.kind === "ambiguous");
  const unmatched = pending.filter((i) => i.kind === "unmapped");
  const attached = items.filter((i) => i.status === "Attached");
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Review Queue"
      sub="Imported comments that need a decision. No comment is ever silently discarded."
      size="lg"
      footer={<Button variant="primary" onClick={onClose}>Close</Button>}
    >
      {queue.isLoading ? (
        <Skeleton h={120} />
      ) : items.length === 0 ? (
        <EmptyState title="Nothing waiting">When an imported comment can't be matched safely, it waits here for you.</EmptyState>
      ) : (
        <>
          <div className="h4">Ambiguous mapping ({ambiguous.length})</div>
          {ambiguous.map((i) => <QueueRow key={i.id} item={i} scenes={scenes.data ?? []} />)}
          <div className="h4" style={{ marginTop: 10 }}>Unmatched Review Notes ({unmatched.length})</div>
          {unmatched.map((i) => <QueueRow key={i.id} item={i} scenes={scenes.data ?? []} />)}
          {attached.length > 0 && (
            <>
              <div className="h4" style={{ marginTop: 10 }}>Attached ({attached.length})</div>
              {attached.map((i) => <QueueRow key={i.id} item={i} scenes={[]} />)}
            </>
          )}
        </>
      )}
    </Dialog>
  );
}

type Tab = "imports" | "records" | "reviews" | "log";

export function PackageHistoryDialog({ onClose }: { onClose: () => void }) {
  const [tab, setTab] = useState<Tab>("imports");
  const openUi = usePackagesUi((s) => s.open);
  const sessions = useOp<PackageSessionSummary[]>("packages.sessions", {}, ["*"], { enabled: tab === "imports" });
  const records = useOp<PackageRecordSummary[]>("packages.records", {}, ["exchange_review_record"], { enabled: tab === "records" });
  const reviews = useOp<PackageReviewSummary[]>("packages.review_list", {}, [], { enabled: tab === "reviews" });
  const log = useOp<PackageLogEntry[]>("packages.log", {}, ["*"], { enabled: tab === "log" });
  const [deleting, setDeleting] = useState<PackageRecordSummary | null>(null);
  const del = useCommand<{ recordId: string }>("packages.record_delete", {
    onSuccess: () => {
      toast.undoable("Deleted review record");
      setDeleting(null);
    },
  });
  const undo = async (s: PackageSessionSummary) => {
    try {
      const r = await packagesApi.undo(s.id);
      toast.info(r.result?.message ?? "Import undone.");
      void sessions.refetch();
    } catch (e) {
      reportError(e);
    }
  };

  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Import & Export History"
      sub="Every package this project exported or imported, and what happened."
      size="lg"
      footer={<Button variant="primary" onClick={onClose}>Close</Button>}
    >
      <Segmented
        ariaLabel="History"
        value={tab}
        onChange={setTab}
        options={[
          { value: "imports", label: "Imports" },
          { value: "records", label: "Review records" },
          { value: "reviews", label: "My reviews" },
          { value: "log", label: "Package log" },
        ]}
      />
      <div style={{ marginTop: 10, maxHeight: 420, overflow: "auto" }}>
        {tab === "imports" &&
          (sessions.isLoading ? (
            <Skeleton h={100} />
          ) : (sessions.data ?? []).length === 0 ? (
            <EmptyState title="No imports yet" />
          ) : (
            <table className="tbl">
              <thead>
                <tr><th>Package</th><th style={{ width: 140 }}>Result</th><th style={{ width: 120 }}>When</th><th style={{ width: 90 }} /></tr>
              </thead>
              <tbody>
                {(sessions.data ?? []).map((s) => (
                  <tr key={s.id}>
                    <td>
                      <b className="sm">{s.fileName}</b>
                      <div className="xs muted">{s.typeLabel} · from “{s.sourceProjectTitle}”</div>
                      {s.message && <div className="xs">{s.message}</div>}
                    </td>
                    <td><Chip tone={s.status === "Failed" || s.status === "Rejected" ? "r" : s.status === "Applied" ? "g" : "y"}>{resultLabel(s.status)}</Chip></td>
                    <td className="xs muted">{dayAndTime(s.createdAt)}</td>
                    <td>
                      {s.canUndo && <Button size="sm" onClick={() => void undo(s)}>Undo</Button>}
                      {s.status === "Previewed" && (
                        <Button size="sm" onClick={() => openUi({ kind: "importExchange", sessionId: s.id })}>Review</Button>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          ))}
        {tab === "records" &&
          ((records.data ?? []).length === 0 ? (
            <EmptyState title="No review records">A stale package can be kept as a separate review record instead of being imported.</EmptyState>
          ) : (
            (records.data ?? []).map((r) => (
              <div key={r.id} className="card pad row" style={{ marginBottom: 6 }}>
                <div className="grow">
                  <b className="sm">{r.title}</b>
                  <div className="xs muted">from “{r.sourceProjectTitle}” · {r.commentCount} comments · kept {dayAndTime(r.createdAt)}</div>
                </div>
                <Button size="sm" onClick={() => openUi({ kind: "viewer", source: "record", key: r.id, back: { kind: "history" } })}>View</Button>
                <Button size="sm" variant="ghost" onClick={() => setDeleting(r)}>Delete</Button>
              </div>
            ))
          ))}
        {tab === "reviews" &&
          ((reviews.data ?? []).length === 0 ? (
            <EmptyState title="No open reviews">Review packages you open with Open Review Package… are listed here.</EmptyState>
          ) : (
            (reviews.data ?? []).map((r) => (
              <div key={r.packageId} className="card pad row" style={{ marginBottom: 6 }}>
                <div className="grow">
                  <b className="sm">{r.projectTitle} — {r.draftName}</b>
                  <div className="xs muted">
                    from {r.exportedBy} · {r.myCommentCount} of your comments
                    {r.responseExportedAt ? ` · response exported ${dayAndTime(r.responseExportedAt)}` : ""}
                  </div>
                </div>
                <Button size="sm" onClick={() => openUi({ kind: "viewer", source: "review", key: r.packageId, back: { kind: "history" } })}>Open</Button>
              </div>
            ))
          ))}
        {tab === "log" &&
          ((log.data ?? []).length === 0 ? (
            <EmptyState title="No packages yet" />
          ) : (
            <table className="tbl">
              <tbody>
                {(log.data ?? []).map((l) => (
                  <tr key={l.id}>
                    <td style={{ width: 70 }}><Chip tone={l.direction === "export" ? "b" : "p"}>{l.direction === "export" ? "Export" : "Import"}</Chip></td>
                    <td className="sm">{l.summary}</td>
                    <td className="xs muted" style={{ width: 130 }}>{dayAndTime(l.at)}{l.actorName ? ` · ${l.actorName}` : ""}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          ))}
      </div>
      {tab === "imports" && <Banner tone="info">Undo removes everything one import added, as long as it wasn't changed afterwards.</Banner>}
      <ConfirmDialog
        open={!!deleting}
        onOpenChange={(v) => !v && setDeleting(null)}
        title="Delete this review record?"
        confirmLabel="Delete"
        danger
        busy={del.isPending}
        onConfirm={() => deleting && del.mutate({ recordId: deleting.id })}
      >
        <p className="sm">It moves to Recently Deleted and can be restored from there.</p>
      </ConfirmDialog>
    </Dialog>
  );
}
