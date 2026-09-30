// Drafts drawer, New Draft, Rename, Delete (FSD §21; UX §3.15; mocks 083, 084, 101).

import { useState } from "react";
import { MoreHorizontal } from "lucide-react";
import { Banner, Button, Chip, Dialog, Drawer, Field, Menu, Segmented, Select, Skeleton, TextArea, TextInput, type MenuItemSpec } from "../../design-system";
import { call } from "../../ipc/client";
import { reportError } from "../../ipc/query";
import type { ScreenplayDraftDto } from "../../ipc/generated/ScreenplayDraftDto";
import type { ScreenplayPermissions } from "../../ipc/generated/ScreenplayPermissions";
import { toast } from "../../app/toast";
import { useHistoryPoints } from "./api";
import { formatAgo, formatWhen } from "./util";

export function statusChip(d: ScreenplayDraftDto) {
  switch (d.status) {
    case "Locked":
      return <Chip tone="g">Locked</Chip>;
    case "Review":
      return <Chip tone="b">In review</Chip>;
    case "Revision":
      return <Chip tone="p">{d.revisionLabel ? `${d.revisionLabel}${d.revisionColor ? ` · ${d.revisionColor}` : ""}` : "Revision"}</Chip>;
    default:
      return <Chip>Draft</Chip>;
  }
}

export interface DraftActions {
  open: (d: ScreenplayDraftDto) => void;
  rename: (d: ScreenplayDraftDto) => void;
  makeCurrent: (d: ScreenplayDraftDto) => void;
  restore: (d: ScreenplayDraftDto) => void;
  remove: (d: ScreenplayDraftDto) => void;
  lock: (d: ScreenplayDraftDto) => void;
  review: (d: ScreenplayDraftDto) => void;
  compare: (a: ScreenplayDraftDto | null, b: ScreenplayDraftDto | null) => void;
  newDraft: () => void;
}

export function DraftsDrawer({ drafts, openDraftId, permissions, actions, onClose }: {
  drafts: ScreenplayDraftDto[];
  openDraftId: string;
  permissions: ScreenplayPermissions;
  actions: DraftActions;
  onClose: () => void;
}) {
  const current = drafts.find((d) => d.isCurrent) ?? null;
  const openDraft = drafts.find((d) => d.id === openDraftId) ?? current;
  const history = useHistoryPoints(openDraft?.id ?? null);
  const sorted = drafts.slice().sort((a, b) => b.createdAt - a.createdAt);
  const restorePoint = async (id: string) => {
    try {
      const d = await call<ScreenplayDraftDto>("screenplay.restore_history_point", { historyPointId: id });
      toast.undoable(`Restored as a new draft: ${d.name}`);
      actions.open(d);
    } catch (e) {
      reportError(e);
    }
  };
  return (
    <Drawer
      open
      onClose={onClose}
      typeLabel="Screenplay"
      title="Drafts"
      width="w"
      footer={
        <>
          <Button onClick={() => actions.compare(null, null)} disabled={drafts.length < 2}>Compare…</Button>
          {permissions.edit && <Button variant="primary" onClick={actions.newDraft}>New Draft</Button>}
        </>
      }
    >
      {current && (
        <div className="spx-lineage sm" aria-label="Draft lineage">
          {current.lineage.map((l, i) => (
            <span key={l.id}>
              {i > 0 && <span className="muted"> → </span>}
              <span className={l.id === current.id ? "b" : ""}>{l.name}</span>
            </span>
          ))}
        </div>
      )}
      <div className="card" style={{ marginTop: 10 }}>
        {sorted.map((d) => {
          const items: MenuItemSpec[] = [
            { label: "Open", onSelect: () => actions.open(d) },
            ...(permissions.edit ? [{ label: "Rename…", onSelect: () => actions.rename(d) }] : []),
            ...(permissions.edit && !d.isCurrent ? [{ label: "Mark current", onSelect: () => actions.makeCurrent(d) }] : []),
            ...(permissions.edit ? [{ label: "Restore as new draft", onSelect: () => actions.restore(d) }] : []),
            ...(!d.isCurrent && current ? [{ label: "Compare with current", onSelect: () => actions.compare(d, current) }] : []),
            ...(permissions.edit ? [{ label: "Start Review…", onSelect: () => actions.review(d) }] : []),
            ...(permissions.lock && d.status !== "Locked" ? [{ label: "Lock as Shooting Draft…", onSelect: () => actions.lock(d) }] : []),
            ...(permissions.delete ? [{ label: "Delete…", danger: true, separatorBefore: true, onSelect: () => actions.remove(d) }] : []),
          ];
          return (
            <div key={d.id} className={`li${d.id === openDraftId ? " sel" : ""}`}>
              <div className="grow">
                <div className="row wrap gap4">
                  <b>{d.name}</b>
                  {d.isCurrent && <Chip tone="a">Current</Chip>}
                  {statusChip(d)}
                </div>
                <div className="xs muted">
                  {formatAgo(d.createdAt)}
                  {d.note ? ` · ${d.note}` : ""}
                  {d.createdFromName ? ` · from ${d.createdFromName}` : ""}
                </div>
              </div>
              <Button size="sm" onClick={() => actions.open(d)} disabled={d.id === openDraftId}>Open</Button>
              <Menu trigger={<button className="iconbtn" aria-label={`Actions for ${d.name}`}><MoreHorizontal size={15} /></button>} items={items} align="end" />
            </div>
          );
        })}
      </div>
      <div className="h4" style={{ marginTop: 16 }}>Recovery History{openDraft ? ` — ${openDraft.name}` : ""}</div>
      {history.isLoading ? (
        <Skeleton h={40} />
      ) : (history.data ?? []).length === 0 ? (
        <p className="muted sm">No automatic points yet.</p>
      ) : (
        <div className="card">
          {(history.data ?? []).map((h) => (
            <div key={h.id} className="li">
              <div className="grow">
                {formatWhen(h.createdAt)} — {h.reason === "draft_created" ? "draft created" : "automatic point"}
                <div className="xs muted">{h.sceneCount} scenes</div>
              </div>
              {permissions.edit && <Button size="sm" onClick={() => void restorePoint(h.id)}>Restore</Button>}
            </div>
          ))}
        </div>
      )}
      <p className="xs muted" style={{ marginTop: 8 }}>
        Automatic history is for recovery. It is not a deliverable draft. Restoring creates a new draft; nothing is replaced.
      </p>
    </Drawer>
  );
}

export function NewDraftDialog({ drafts, current, onClose, onCreated }: {
  drafts: ScreenplayDraftDto[];
  current: ScreenplayDraftDto;
  onClose: () => void;
  onCreated: (d: ScreenplayDraftDto) => void;
}) {
  const [from, setFrom] = useState<"current" | "previous">("current");
  const others = drafts.filter((d) => d.id !== current.id);
  const [prevId, setPrevId] = useState(others[0]?.id ?? current.id);
  const [name, setName] = useState(() => {
    for (let n = drafts.length + 1; n < drafts.length + 100; n++) {
      const candidate = `Draft ${n}`;
      if (!drafts.some((d) => d.name.toLowerCase() === candidate.toLowerCase())) return candidate;
    }
    return "";
  });
  const [note, setNote] = useState("");
  const [err, setErr] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const create = async () => {
    setBusy(true);
    setErr(null);
    try {
      const d = await call<ScreenplayDraftDto>("screenplay.new_draft", {
        sourceDraftId: from === "current" ? current.id : prevId,
        name,
        note: note.trim() || undefined,
      });
      toast.undoable(`Created draft “${d.name}”`);
      onCreated(d);
    } catch (e) {
      const m = (e as { message?: string }).message;
      setErr(m ?? "The draft could not be created.");
      if (!(e as { code?: string }).code?.startsWith("validation")) reportError(e);
    } finally {
      setBusy(false);
    }
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="New Draft"
      size="md"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!name.trim() || busy} onClick={() => void create()}>Create Draft</Button>
        </>
      }
    >
      <Field label="Create from">
        <Segmented
          ariaLabel="Create from"
          value={from}
          onChange={setFrom}
          options={[
            { value: "current", label: `Current draft (${current.name})` },
            ...(others.length > 0 ? [{ value: "previous" as const, label: "Previous named draft…" }] : []),
          ]}
        />
      </Field>
      {from === "previous" && others.length > 0 && (
        <Field label="Source draft" htmlFor="nd-src">
          <Select id="nd-src" value={prevId} onChange={setPrevId} options={others.map((d) => ({ value: d.id, label: d.name }))} />
        </Field>
      )}
      <Field label="Draft name" required htmlFor="nd-name" error={err}>
        <TextInput id="nd-name" value={name} autoFocus onChange={(e) => setName(e.target.value)} invalid={!!err} maxLength={120} />
      </Field>
      <Field label="Note (optional)" htmlFor="nd-note">
        <TextArea id="nd-note" value={note} onChange={(e) => setNote(e.target.value)} rows={2} />
      </Field>
      <p className="sm muted">The source draft stays exactly as it is. The new draft becomes a separate version.</p>
    </Dialog>
  );
}

export function RenameDraftDialog({ draft, onClose }: { draft: ScreenplayDraftDto; onClose: () => void }) {
  const [name, setName] = useState(draft.name);
  const [note, setNote] = useState(draft.note ?? "");
  const [err, setErr] = useState<string | null>(null);
  const save = async () => {
    try {
      await call("screenplay.rename_draft", { draftId: draft.id, name, note });
      onClose();
    } catch (e) {
      setErr((e as { message?: string }).message ?? "The draft could not be renamed.");
    }
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Rename draft"
      sub="Renaming changes only the label, never the text."
      size="md"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!name.trim()} onClick={() => void save()}>Save</Button>
        </>
      }
    >
      <Field label="Draft name" required htmlFor="rd-name" error={err}>
        <TextInput id="rd-name" value={name} autoFocus onChange={(e) => setName(e.target.value)} maxLength={120} />
      </Field>
      <Field label="Note (optional)" htmlFor="rd-note">
        <TextArea id="rd-note" value={note} onChange={(e) => setNote(e.target.value)} rows={2} />
      </Field>
    </Dialog>
  );
}

export function DeleteDraftDialog({ draft, onClose }: { draft: ScreenplayDraftDto; onClose: () => void }) {
  const [busy, setBusy] = useState(false);
  const blocked = draft.isCurrent;
  const remove = async () => {
    setBusy(true);
    try {
      await call("screenplay.delete_draft", { draftId: draft.id });
      toast.undoable(`Moved “${draft.name}” to Recently Deleted`);
      onClose();
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
      title={`Delete “${draft.name}”?`}
      size="md"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="danger" disabled={busy || blocked} onClick={() => void remove()}>Delete Draft</Button>
        </>
      }
    >
      <p>The draft moves to Recently Deleted and can be restored. Your other drafts are not affected.</p>
      {blocked ? (
        <Banner tone="warn">This is the current draft. Make another draft current before deleting it.</Banner>
      ) : (
        <p className="sm muted">
          The current draft cannot be deleted until another draft is made current. A locked draft cannot be deleted while it is the only production baseline.
        </p>
      )}
    </Dialog>
  );
}
