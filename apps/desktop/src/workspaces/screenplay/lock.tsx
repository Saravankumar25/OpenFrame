// Script lock and production revisions (FSD §24, §95; UX §3.17; mocks 088–090).

import { useState } from "react";
import { Lock } from "lucide-react";
import { Banner, Button, Chip, Dialog, Drawer, Field, Skeleton, TextArea, TextInput } from "../../design-system";
import { call } from "../../ipc/client";
import { reportError } from "../../ipc/query";
import type { ScreenplayDraftDto } from "../../ipc/generated/ScreenplayDraftDto";
import type { ScreenplayPermissions } from "../../ipc/generated/ScreenplayPermissions";
import { toast } from "../../app/toast";
import { useLockSummary, useRevisionChanges } from "./api";
import { formatDate, formatWhen } from "./util";

export const REVISION_COLOURS = ["Blue", "Pink", "Yellow", "Green"] as const;
const SWATCH: Record<string, string> = { Blue: "#8fb3ef", Pink: "#f2a7c3", Yellow: "#f3e27a", Green: "#9ed5a6" };

export function colourSwatch(c: string | null): string | undefined {
  if (!c) return undefined;
  return SWATCH[c] ?? (c.startsWith("#") ? c : undefined);
}

/** "Lock this draft as the Shooting Draft?" (mock 088). */
export function LockDialog({ draft, onClose }: { draft: ScreenplayDraftDto; onClose: () => void }) {
  const s = useLockSummary(draft.id);
  const [busy, setBusy] = useState(false);
  const lock = async () => {
    setBusy(true);
    try {
      await call("screenplay.lock_draft", { draftId: draft.id });
      toast.undoable(`Locked “${draft.name}” as the Shooting Draft`);
      onClose();
    } catch (e) {
      reportError(e);
    } finally {
      setBusy(false);
    }
  };
  const d = s.data;
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Lock this draft as the Shooting Draft?"
      sub="Confirm the details, then lock."
      size="md"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" icon={<Lock size={14} />} disabled={busy || !d} onClick={() => void lock()}>Lock as Shooting Draft</Button>
        </>
      }
    >
      {!d ? (
        <Skeleton h={120} />
      ) : (
        <table className="tbl" style={{ marginBottom: 10 }}>
          <tbody>
            <tr><td className="muted">Draft name</td><td className="b">{d.draftName}</td></tr>
            <tr><td className="muted">Current version</td><td>{d.isCurrent ? "Current draft" : "Not the current draft"} · {d.status}</td></tr>
            <tr>
              <td className="muted">Unresolved review notes</td>
              <td>
                {d.openComments === 0 ? "None" : <Chip tone="y">{d.openComments} open</Chip>}
                {d.openCommentSamples.length > 0 && (
                  <ul className="xs muted" style={{ margin: "4px 0 0", paddingLeft: 16 }}>
                    {d.openCommentSamples.map((x, i) => <li key={i}>{x}</li>)}
                  </ul>
                )}
              </td>
            </tr>
            <tr><td className="muted">Number of scenes</td><td>{d.sceneCount}</td></tr>
            <tr><td className="muted">Last modified</td><td>{formatWhen(d.lastModified)}</td></tr>
          </tbody>
        </table>
      )}
      <p className="sm">
        Locking is a safety state, not a destructive action. The draft stays readable and exportable. Future edits will start a post-lock revision so the production baseline cannot be changed by accident.
      </p>
    </Dialog>
  );
}

/** Typing into a locked draft (mock 089). */
export function LockedEditDialog({ draft, onClose, onStart }: { draft: ScreenplayDraftDto; onClose: () => void; onStart: () => void }) {
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="This is your locked shooting draft. Create a revision?"
      size="sm"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" onClick={onStart}>Start Revision</Button>
        </>
      }
    >
      <p>
        You started typing in <b>{draft.name}</b>. To protect the production baseline, your edits will go into a <b>new revision</b>; the locked draft stays exactly as it is.
      </p>
      <p className="sm muted">Nothing has been changed yet.</p>
    </Dialog>
  );
}

function nextRevisionLabel(drafts: ScreenplayDraftDto[]): string {
  const used = new Set(drafts.map((d) => (d.revisionLabel ?? "").toLowerCase()));
  for (let i = 0; i < 26; i++) {
    const l = `Revision ${String.fromCharCode(65 + i)}`;
    if (!used.has(l.toLowerCase())) return l;
  }
  return "Revision";
}

export function StartRevisionDialog({ source, drafts, onClose, onStarted }: {
  source: ScreenplayDraftDto;
  drafts: ScreenplayDraftDto[];
  onClose: () => void;
  onStarted: (d: ScreenplayDraftDto) => void;
}) {
  const [label, setLabel] = useState(() => nextRevisionLabel(drafts));
  const [colour, setColour] = useState<string>("Blue");
  const [custom, setCustom] = useState("#3b6fd4");
  const [useCustom, setUseCustom] = useState(false);
  const [reason, setReason] = useState("");
  const [busy, setBusy] = useState(false);
  const start = async () => {
    setBusy(true);
    try {
      const d = await call<ScreenplayDraftDto>("screenplay.start_revision", {
        draftId: source.id,
        label,
        color: useCustom ? custom : colour,
        reason: reason.trim() || undefined,
      });
      toast.undoable(`Started ${d.revisionLabel ?? d.name}. ${source.name} stays locked.`);
      onStarted(d);
    } catch (e) {
      reportError(e);
    } finally {
      setBusy(false);
    }
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Start Revision"
      sub={`Based on the locked ${source.name}. The locked draft never changes.`}
      size="md"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!label.trim() || busy} onClick={() => void start()}>Start Revision</Button>
        </>
      }
    >
      <Field label="Label" required htmlFor="rv-label">
        <TextInput id="rv-label" value={label} autoFocus onChange={(e) => setLabel(e.target.value)} maxLength={80} />
      </Field>
      <Field label="Colour">
        <div className="row wrap" role="radiogroup" aria-label="Revision colour">
          {REVISION_COLOURS.map((c) => (
            <button
              key={c}
              type="button"
              role="radio"
              aria-checked={!useCustom && colour === c}
              className={`btn sm${!useCustom && colour === c ? " on" : ""}`}
              onClick={() => { setUseCustom(false); setColour(c); }}
            >
              <span className="sw" style={{ background: SWATCH[c] }} aria-hidden />
              {c}
            </button>
          ))}
          <button type="button" role="radio" aria-checked={useCustom} className={`btn sm${useCustom ? " on" : ""}`} onClick={() => setUseCustom(true)}>
            Custom…
          </button>
          {useCustom && <input type="color" aria-label="Custom colour" value={custom} onChange={(e) => setCustom(e.target.value)} />}
        </div>
      </Field>
      <Field label="Reason (optional)" htmlFor="rv-reason">
        <TextArea id="rv-reason" value={reason} onChange={(e) => setReason(e.target.value)} rows={2} />
      </Field>
      <p className="xs muted">The colour is metadata and export styling only; it is separate from your personal card colours.</p>
    </Dialog>
  );
}

/** Revision history and changed scenes (mock 090). */
export function RevisionDrawer({ drafts, openDraft, permissions, onClose, onOpenDraft, onJumpScene, onStartRevision }: {
  drafts: ScreenplayDraftDto[];
  openDraft: ScreenplayDraftDto;
  permissions: ScreenplayPermissions;
  onClose: () => void;
  onOpenDraft: (d: ScreenplayDraftDto) => void;
  onJumpScene: (sceneId: string) => void;
  onStartRevision: (source: ScreenplayDraftDto) => void;
}) {
  const revisions = drafts.filter((d) => d.status === "Revision" || (d.revisionLabel && d.status !== "Locked")).sort((a, b) => b.createdAt - a.createdAt);
  const baselines = drafts.filter((d) => d.status === "Locked").sort((a, b) => b.createdAt - a.createdAt);
  const shown = openDraft.status === "Revision" || openDraft.revisionLabel ? openDraft : revisions[0] ?? null;
  const changes = useRevisionChanges(shown?.createdFromDraftId ? shown.id : null);
  const lockedSource = openDraft.status === "Locked" ? openDraft : baselines[0] ?? null;
  const unlock = async () => {
    try {
      await call("screenplay.unlock_draft", { draftId: openDraft.id });
      toast.undoable(`Unlocked “${openDraft.name}”`);
    } catch (e) {
      reportError(e);
    }
  };
  return (
    <Drawer open onClose={onClose} typeLabel="Screenplay" title="Revisions" width="w">
      <div className="h4">Revision history</div>
      {revisions.length === 0 && <p className="muted sm">No revisions yet.</p>}
      <div className="card">
        {revisions.map((r) => (
          <div key={r.id} className={`li${r.id === openDraft.id ? " sel" : ""}`}>
            <span className="sw" style={{ background: colourSwatch(r.revisionColor) ?? "#ddd", width: 12, height: 12 }} aria-hidden />
            <div className="grow">
              <b>{r.revisionLabel ?? r.name}{r.revisionColor ? ` — ${r.revisionColor}` : ""}</b>
              <div className="xs muted">
                {formatDate(r.createdAt)}
                {r.revisionReason ? ` · Reason: ${r.revisionReason}` : ""}
              </div>
            </div>
            {r.isCurrent && <Chip tone="a">Current revision</Chip>}
            {r.status === "Locked" && <Chip tone="g">Locked</Chip>}
            <Button size="sm" disabled={r.id === openDraft.id} onClick={() => onOpenDraft(r)}>Open</Button>
          </div>
        ))}
        {baselines.map((b) => (
          <div key={b.id} className={`li${b.id === openDraft.id ? " sel" : ""}`}>
            <Lock size={14} aria-hidden />
            <div className="grow">
              <b>{b.name}</b>
              <div className="xs muted">Locked {b.lockedAt ? formatDate(b.lockedAt) : ""} · baseline</div>
            </div>
            <Chip tone="g">Locked</Chip>
            <Button size="sm" disabled={b.id === openDraft.id} onClick={() => onOpenDraft(b)}>Open</Button>
          </div>
        ))}
      </div>
      {lockedSource && permissions.edit && (
        <div className="row" style={{ marginTop: 10 }}>
          <Button variant="primary" onClick={() => onStartRevision(lockedSource)}>Start Revision</Button>
          <span className="xs muted">From {lockedSource.name}. The locked draft stays exactly as it is.</span>
        </div>
      )}
      {openDraft.status === "Locked" && permissions.unlock && (
        <div style={{ marginTop: 10 }}>
          <Banner tone="info" actions={<Button size="sm" onClick={() => void unlock()}>Unlock draft</Button>}>
            Only the project owner can unlock a shooting draft.
          </Banner>
        </div>
      )}
      {shown && (
        <>
          <div className="h4" style={{ marginTop: 16 }}>Changed scenes in {shown.revisionLabel ?? shown.name}</div>
          {changes.isLoading ? (
            <Skeleton h={30} />
          ) : !changes.data || changes.data.scenes.length === 0 ? (
            <p className="muted sm">No scenes changed yet.</p>
          ) : (
            <>
              <div className="row wrap gap4">
                {changes.data.scenes.map((c, i) => {
                  const side = c.b ?? c.a!;
                  const suffix = c.kind === "added" ? " new" : c.kind === "removed" ? " removed" : c.kind === "moved" ? " moved" : "";
                  return (
                    <button
                      key={i}
                      type="button"
                      className={`chip${c.kind === "removed" ? " r" : c.kind === "added" ? " g" : " b"}`}
                      title={side.heading}
                      disabled={!c.b || shown.id !== openDraft.id}
                      onClick={() => c.b && onJumpScene(c.b.sceneId)}
                    >
                      Sc {side.number}{suffix}
                    </button>
                  );
                })}
              </div>
              <p className="xs muted" style={{ marginTop: 6 }}>
                Compared with {changes.data.draftA.name}: {changes.data.summary.changed} changed · {changes.data.summary.added} added · {changes.data.summary.removed} removed · {changes.data.summary.moved} moved.
              </p>
            </>
          )}
        </>
      )}
    </Drawer>
  );
}
