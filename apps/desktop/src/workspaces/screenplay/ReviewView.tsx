// Review rounds (FSD §23; UX §3.16; mocks 086–087). Completing a review never
// locks the script (FSD §23.7).

import { useEffect, useMemo, useState } from "react";
import { ArrowLeft, Plus, X } from "lucide-react";
import { Banner, Button, Chip, Dialog, EmptyState, Field, Select, Skeleton, TextInput } from "../../design-system";
import { call } from "../../ipc/client";
import { reportError, useOp } from "../../ipc/query";
import type { ScreenplayDraftDto } from "../../ipc/generated/ScreenplayDraftDto";
import type { ScreenplayDocument } from "../../ipc/generated/ScreenplayDocument";
import type { ScreenplayPermissions } from "../../ipc/generated/ScreenplayPermissions";
import type { ScreenplayReviewRoundDto } from "../../ipc/generated/ScreenplayReviewRoundDto";
import type { CommentThread } from "../../ipc/generated/CommentThread";
import type { CommentDto } from "../../ipc/generated/CommentDto";
import { toast } from "../../app/toast";
import { useReviewRounds } from "./api";
import { ThreadCard, type PanelContext } from "./panels";

function formatDeadline(d: string): string {
  const [y, m, day] = d.split("-").map(Number);
  return new Date(y, m - 1, day).toLocaleDateString(undefined, { day: "numeric", month: "short" });
}

export function NewReviewDialog({ drafts, initialDraftId, onClose, onStarted }: {
  drafts: ScreenplayDraftDto[];
  initialDraftId: string;
  onClose: () => void;
  onStarted: (r: ScreenplayReviewRoundDto) => void;
}) {
  const [draftId, setDraftId] = useState(initialDraftId);
  const draft = drafts.find((d) => d.id === draftId);
  const [name, setName] = useState(() => `${draft?.name ?? "Draft"} — Review`);
  const [reviewers, setReviewers] = useState<string[]>([]);
  const [who, setWho] = useState("");
  const [deadline, setDeadline] = useState("");
  const [err, setErr] = useState<string | null>(null);
  const add = () => {
    const w = who.trim();
    if (w && !reviewers.some((r) => r.toLowerCase() === w.toLowerCase())) setReviewers([...reviewers, w]);
    setWho("");
  };
  const start = async () => {
    try {
      const r = await call<ScreenplayReviewRoundDto>("screenplay.start_review", {
        draftId,
        name,
        reviewers: who.trim() ? [...reviewers, who.trim()] : reviewers,
        deadline: deadline || undefined,
      });
      toast.undoable(`Started review “${r.name}”`);
      onStarted(r);
    } catch (e) {
      setErr((e as { message?: string }).message ?? "The review could not be started.");
    }
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="New Review"
      size="md"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!name.trim()} onClick={() => void start()}>Start Review</Button>
        </>
      }
    >
      <Field label="Review name" required htmlFor="rv-name" error={err}>
        <TextInput id="rv-name" value={name} autoFocus onChange={(e) => setName(e.target.value)} maxLength={120} />
      </Field>
      <Field label="Source draft" required htmlFor="rv-draft">
        <Select id="rv-draft" value={draftId} onChange={setDraftId} options={drafts.map((d) => ({ value: d.id, label: d.name }))} />
      </Field>
      <Field label="Reviewers" htmlFor="rv-who">
        <div className="row wrap gap4" style={{ marginBottom: 4 }}>
          {reviewers.map((r) => (
            <Chip key={r}>
              {r}
              <button type="button" className="spx-x" aria-label={`Remove ${r}`} onClick={() => setReviewers(reviewers.filter((x) => x !== r))}>
                <X size={10} />
              </button>
            </Chip>
          ))}
        </div>
        <TextInput
          id="rv-who"
          placeholder="Add reviewer…"
          value={who}
          onChange={(e) => setWho(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              add();
            }
          }}
          onBlur={add}
        />
      </Field>
      <Field label="Deadline (optional)" htmlFor="rv-deadline">
        <TextInput id="rv-deadline" type="date" value={deadline} onChange={(e) => setDeadline(e.target.value)} />
      </Field>
    </Dialog>
  );
}

function AddReviewerDialog({ round, onClose }: { round: ScreenplayReviewRoundDto; onClose: () => void }) {
  const [who, setWho] = useState("");
  const save = async () => {
    try {
      await call("screenplay.update_review", { reviewRoundId: round.id, reviewers: [...round.reviewers, who.trim()] });
      onClose();
    } catch (e) {
      reportError(e);
    }
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Add reviewer"
      size="sm"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!who.trim()} onClick={() => void save()}>Add</Button>
        </>
      }
    >
      <Field label="Reviewer name" required htmlFor="ar-who">
        <TextInput id="ar-who" value={who} autoFocus onChange={(e) => setWho(e.target.value)} onKeyDown={(e) => e.key === "Enter" && who.trim() && void save()} />
      </Field>
    </Dialog>
  );
}

export function ReviewView({ screenplayId, drafts, doc, permissions, initialRoundId, onBack, onJumpToComment }: {
  screenplayId: string;
  drafts: ScreenplayDraftDto[];
  doc: ScreenplayDocument | null;
  permissions: ScreenplayPermissions;
  initialRoundId: string | null;
  onBack: () => void;
  onJumpToComment: (c: CommentDto) => void;
}) {
  const rounds = useReviewRounds(screenplayId);
  const [roundId, setRoundId] = useState<string | null>(initialRoundId);
  const [newOpen, setNewOpen] = useState(false);
  const [addOpen, setAddOpen] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  useEffect(() => {
    if (!roundId && rounds.data && rounds.data.length > 0) setRoundId(rounds.data[0].id);
  }, [rounds.data, roundId]);
  const round = rounds.data?.find((r) => r.id === roundId) ?? null;
  const threads = useOp<CommentThread[]>("comment.list", { reviewRoundId: roundId }, ["comment", "screenplay_scene", "screenplay_element"], { enabled: !!roundId });
  const groups = useMemo(() => {
    const m = new Map<string, { label: string; order: number; items: CommentThread[] }>();
    for (const t of threads.data ?? []) {
      const key = t.comment.sceneId ?? "draft";
      const label = t.comment.sceneNumber
        ? `Scene ${t.comment.sceneNumber} — ${t.comment.sceneHeading || "Untitled scene"}`
        : t.comment.targetType === "screenplay_draft"
          ? "Whole draft"
          : "Deleted or moved scene";
      if (!m.has(key)) m.set(key, { label, order: t.comment.sceneNumber ?? 0, items: [] });
      m.get(key)!.items.push(t);
    }
    return [...m.values()].sort((a, b) => a.order - b.order);
  }, [threads.data]);
  const sel = (threads.data ?? []).find((t) => t.comment.id === selected) ?? null;
  const complete = async () => {
    if (!round) return;
    try {
      await call("screenplay.complete_review", { reviewRoundId: round.id });
      toast.undoable(`Completed “${round.name}”. The script was not locked.`);
    } catch (e) {
      reportError(e);
    }
  };
  const ctx: PanelContext | null = doc
    ? {
        doc,
        permissions,
        currentSceneId: null,
        threads: threads.data ?? [],
        commentsLoading: threads.isLoading,
        pendingComment: null,
        setPendingComment: () => undefined,
        focusCommentId: selected,
        showNotes: true,
        setShowNotes: () => undefined,
        jumpToScene: () => undefined,
        jumpToComment: onJumpToComment,
        flush: async () => undefined,
      }
    : null;

  return (
    <div className="spx-review">
      <div className="sp-top">
        <Button size="sm" variant="ghost" icon={<ArrowLeft size={14} />} onClick={onBack}>Back to script</Button>
        {rounds.data && rounds.data.length > 1 && (
          <Select ariaLabel="Review round" value={roundId ?? ""} onChange={(v) => { setRoundId(v); setSelected(null); }} options={rounds.data.map((r) => ({ value: r.id, label: `${r.name}${r.status === "Complete" ? " (complete)" : ""}` }))} />
        )}
        <span className="grow" />
        {permissions.edit && <Button size="sm" icon={<Plus size={14} />} onClick={() => setNewOpen(true)}>New Review</Button>}
        {round && permissions.edit && round.status === "Open" && <Button size="sm" onClick={() => setAddOpen(true)}>Add reviewer</Button>}
        {round && permissions.resolveComments && round.status === "Open" && <Button size="sm" variant="primary" onClick={() => void complete()}>Complete Review</Button>}
      </div>
      {rounds.isLoading ? (
        <div className="content"><Skeleton h={120} /></div>
      ) : !round ? (
        <EmptyState
          title="No review rounds yet"
          actions={permissions.edit ? <Button variant="primary" onClick={() => setNewOpen(true)}>New Review</Button> : undefined}
        >
          Start a review on a draft, add reviewers, and collect their notes as comments on the script. Reviewers add notes by selecting text in the script and choosing Comment.
        </EmptyState>
      ) : (
        <div className="sp-body">
          <div className="grow spx-scroll" style={{ padding: "14px 18px" }}>
            <h2 style={{ margin: 0, fontSize: 18 }}>{round.name}</h2>
            <div className="sm muted" style={{ marginBottom: 10 }}>
              Source draft: {round.draftName}
              {round.reviewers.length > 0 ? ` · Reviewers: ${round.reviewers.join(", ")}` : ""}
              {round.deadline ? ` · Deadline ${formatDeadline(round.deadline)}` : ""}
            </div>
            <div className="row wrap" style={{ marginBottom: 12 }}>
              <Chip tone="y">Open notes: {round.openCount + round.discussionCount}</Chip>
              <Chip tone="g">Resolved: {round.resolvedCount}</Chip>
              <Chip tone={round.status === "Open" ? "b" : "default"}>{round.status === "Open" ? "In progress" : "Complete"}</Chip>
            </div>
            {round.status === "Complete" && <Banner tone="ok">This review is complete. Its comments stay in review history.</Banner>}
            {threads.isLoading ? (
              <Skeleton h={80} />
            ) : groups.length === 0 ? (
              <p className="muted">No notes yet. Open the script, select text, and choose Comment to add the first note to this review.</p>
            ) : (
              groups.map((g) => (
                <div key={g.label} style={{ marginBottom: 14 }}>
                  <div className="h4">{g.label}</div>
                  <div className="card">
                    {g.items.map((t) => (
                      <button
                        key={t.comment.id}
                        type="button"
                        className={`li spx-li-btn${selected === t.comment.id ? " sel" : ""}`}
                        onClick={() => setSelected(t.comment.id)}
                      >
                        <Chip tone={t.comment.status === "Resolved" ? "g" : t.comment.status === "In Discussion" ? "b" : "y"}>{t.comment.status}</Chip>
                        <span className="grow" style={{ textAlign: "left" }}>{t.comment.body}</span>
                        <span className="xs muted">{t.comment.authorName}</span>
                      </button>
                    ))}
                  </div>
                </div>
              ))
            )}
          </div>
          <aside className="spx-tools" aria-label="Selected thread">
            <div className="spx-ph"><h3>Selected thread</h3></div>
            <div className="spx-pb">
              {sel && ctx ? <ThreadCard t={sel} ctx={ctx} focused /> : <p className="muted sm">Select a note to read and answer it.</p>}
            </div>
          </aside>
        </div>
      )}
      {newOpen && (
        <NewReviewDialog
          drafts={drafts}
          initialDraftId={round?.draftId ?? drafts.find((d) => d.isCurrent)?.id ?? drafts[0].id}
          onClose={() => setNewOpen(false)}
          onStarted={(r) => {
            setNewOpen(false);
            setRoundId(r.id);
          }}
        />
      )}
      {addOpen && round && <AddReviewerDialog round={round} onClose={() => setAddOpen(false)} />}
    </div>
  );
}
