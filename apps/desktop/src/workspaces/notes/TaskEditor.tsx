// Task drawer (FSD §106, §158): title (required), optional due date, owner and
// related object (Scene, Location, Character, Breakdown Item, Shooting Day,
// Call Sheet…), notes. Mark Done / Reopen never changes the related object.

import { useEffect, useMemo, useState } from "react";
import { CheckCircle2, Link2, RotateCcw, Search, Trash2, X } from "lucide-react";
import { call } from "../../ipc/client";
import { reportError, useOp } from "../../ipc/query";
import type { Member } from "../../ipc/generated/Member";
import type { RelatableType } from "../../ipc/generated/RelatableType";
import type { RelatedRef } from "../../ipc/generated/RelatedRef";
import type { SearchHit } from "../../ipc/generated/SearchHit";
import type { TaskDto } from "../../ipc/generated/TaskDto";
import { Button, Chip, Drawer, Field, Select, TextArea, TextInput } from "../../design-system";
import { navigateTo } from "../../app/navigate";
import { toast } from "../../app/toast";
import { fromDateInput, toDateInput } from "../../app/home/format";

interface RelatedChoice {
  ref: RelatedRef;
  title: string;
  typeLabel: string;
}

function RelatedPicker({ value, onChange }: { value: RelatedChoice | null; onChange: (v: RelatedChoice | null) => void }) {
  const types = useOp<RelatableType[]>("tasks.relatable_types", {}, []);
  const [text, setText] = useState("");
  const [debounced, setDebounced] = useState("");
  useEffect(() => {
    const t = window.setTimeout(() => setDebounced(text.trim()), 200);
    return () => window.clearTimeout(t);
  }, [text]);
  const labels = useMemo(() => new Map((types.data ?? []).map((t) => [t.targetType, t.label])), [types.data]);
  const hits = useOp<SearchHit[]>(
    "search.query",
    { text: debounced, entityTypes: (types.data ?? []).map((t) => t.targetType), limit: 8 },
    ["search_doc"],
    { enabled: debounced.length > 0 && !!types.data },
  );
  if (value) {
    return (
      <div className="row">
        <Chip>{value.typeLabel}</Chip>
        <span className="grow sm b" style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{value.title}</span>
        <Button size="xs" variant="ghost" icon={<X size={12} />} onClick={() => onChange(null)} aria-label="Remove related item">Remove</Button>
      </div>
    );
  }
  return (
    <div>
      <div className="input" style={{ padding: "0 10px" }}>
        <Search size={14} aria-hidden style={{ color: "var(--muted)" }} />
        <input
          type="search"
          aria-label="Find a scene, location, character, shooting day or call sheet"
          placeholder="Find a scene, location, character…"
          value={text}
          onChange={(e) => setText(e.target.value)}
          style={{ border: 0, outline: "none", flex: 1, background: "transparent", minHeight: 30 }}
        />
      </div>
      {debounced && (
        <div role="listbox" aria-label="Matching items" className="card" style={{ marginTop: 4, maxHeight: 200, overflow: "auto" }}>
          {(hits.data ?? []).length === 0 ? (
            <div className="li muted sm">{hits.isFetching ? "Searching…" : "Nothing matches. Tasks can relate to scenes, scene cards, characters, locations, breakdown items, cast, crew, shooting days and call sheets."}</div>
          ) : (
            (hits.data ?? []).map((h) => (
              <button
                key={h.entityId}
                role="option"
                aria-selected={false}
                className="li clickable"
                style={{ width: "100%", border: 0, borderBottom: "1px solid var(--line)", background: "transparent", textAlign: "left" }}
                onClick={() => {
                  onChange({ ref: { targetType: h.entityType, targetId: h.entityId }, title: h.title || "Untitled", typeLabel: labels.get(h.entityType) ?? "Item" });
                  setText("");
                }}
              >
                <Chip>{labels.get(h.entityType) ?? h.entityType}</Chip>
                <span className="grow sm">{h.title || "Untitled"}</span>
                <span className="xs muted">{h.context}</span>
              </button>
            ))
          )}
        </div>
      )}
    </div>
  );
}

export function TaskEditor({ task, onClose }: { task: TaskDto | null; onClose: () => void }) {
  const members = useOp<Member[]>("project.members", {}, ["project_member"]);
  const [title, setTitle] = useState(task?.title ?? "");
  const [due, setDue] = useState(toDateInput(task?.dueAt));
  const [owner, setOwner] = useState(task?.ownerUserId ?? "");
  const [notes, setNotes] = useState(task?.notes ?? "");
  const [related, setRelated] = useState<RelatedChoice | null>(
    task?.related ? { ref: { targetType: task.related.targetType, targetId: task.related.targetId }, title: task.related.title ?? "Unavailable item", typeLabel: task.related.typeLabel } : null,
  );
  const [touched, setTouched] = useState(false);
  const [busy, setBusy] = useState(false);
  const titleMissing = title.trim().length === 0;
  const initialRelated = task?.related ? `${task.related.targetType}:${task.related.targetId}` : "";
  const currentRelated = related ? `${related.ref.targetType}:${related.ref.targetId}` : "";
  const dirty =
    !task ||
    title !== task.title ||
    due !== toDateInput(task.dueAt) ||
    owner !== (task.ownerUserId ?? "") ||
    notes !== (task.notes ?? "") ||
    currentRelated !== initialRelated;

  const save = async () => {
    setTouched(true);
    if (titleMissing || busy) return;
    setBusy(true);
    const dueAt = fromDateInput(due);
    try {
      if (!task) {
        await call("tasks.create", { title: title.trim(), dueAt, ownerUserId: owner || null, notes: notes.trim() || null, related: related?.ref ?? null });
        toast.undoable(`Added task “${title.trim()}”`);
      } else {
        await call("tasks.update", {
          id: task.id,
          title: title.trim(),
          ...(dueAt == null ? { clearDue: true } : { dueAt }),
          ownerUserId: owner,
          notes,
          ...(related ? (currentRelated !== initialRelated ? { related: related.ref } : {}) : { clearRelated: true }),
          expectedRev: task.rev,
        });
        toast.undoable("Edited task");
      }
      onClose();
    } catch (e) {
      reportError(e);
      setBusy(false);
    }
  };

  const setDone = async (done: boolean) => {
    if (!task) return;
    try {
      await call("tasks.set_done", { id: task.id, done });
      toast.undoable(done ? `Completed task “${task.title}”` : `Reopened task “${task.title}”`);
    } catch (e) {
      reportError(e);
    }
  };
  const remove = async () => {
    if (!task) return;
    try {
      await call("tasks.delete", { id: task.id });
      toast.undoable(`Moved task “${task.title}” to Recently Deleted`);
      onClose();
    } catch (e) {
      reportError(e);
    }
  };

  const memberOptions = [{ value: "", label: "No owner" }, ...(members.data ?? []).map((m) => ({ value: m.userId, label: m.isYou ? `${m.displayName} (you)` : m.displayName }))];

  return (
    <Drawer
      open
      onClose={onClose}
      typeLabel="Task"
      title={task ? task.title : "New Task"}
      footer={
        <>
          {task && <Button variant="ghost" icon={<Trash2 size={14} />} style={{ color: "var(--red)", marginRight: "auto" }} onClick={() => void remove()}>Delete</Button>}
          {task && (task.status === "Done" ? (
            <Button icon={<RotateCcw size={14} />} onClick={() => void setDone(false)}>Reopen</Button>
          ) : (
            <Button icon={<CheckCircle2 size={14} />} onClick={() => void setDone(true)}>Mark Done</Button>
          ))}
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" type="submit" form="task-editor" disabled={busy || !dirty}>{task ? "Save" : "Add Task"}</Button>
        </>
      }
    >
      <form
        id="task-editor"
        onSubmit={(e) => {
          e.preventDefault();
          void save();
        }}
      >
        {task?.status === "Done" && <div style={{ marginBottom: 10 }}><Chip tone="g">Done</Chip></div>}
        <Field label="Task" required htmlFor="te-title" error={touched && titleMissing ? "Give the task a short title." : null}>
          <TextInput id="te-title" autoFocus={!task} value={title} maxLength={300} invalid={touched && titleMissing} placeholder="e.g. Confirm police station" onChange={(e) => setTitle(e.target.value)} onBlur={() => setTouched(true)} />
        </Field>
        <div className="grid g2" style={{ gap: "0 12px" }}>
          <Field label="Due date" htmlFor="te-due">
            <TextInput id="te-due" type="date" value={due} onChange={(e) => setDue(e.target.value)} />
          </Field>
          <Field label="Owner" htmlFor="te-owner">
            <Select id="te-owner" value={owner} onChange={setOwner} options={memberOptions} />
          </Field>
        </div>
        <Field label="Related to" hint="Optional. Completing the task never changes the related item.">
          <RelatedPicker value={related} onChange={setRelated} />
          {task?.related && !task.related.available && currentRelated === initialRelated && (
            <div className="xs muted" style={{ marginTop: 4 }}>The related {task.related.typeLabel.toLowerCase()} was deleted. The task keeps the reference.</div>
          )}
          {task?.related?.nav != null && currentRelated === initialRelated && (
            <Button size="xs" variant="ghost" icon={<Link2 size={12} />} style={{ marginTop: 4 }} onClick={() => navigateTo(task.related?.nav)}>Open {task.related.typeLabel}</Button>
          )}
        </Field>
        <Field label="Notes" htmlFor="te-notes">
          <TextArea id="te-notes" rows={4} maxLength={5000} value={notes} onChange={(e) => setNotes(e.target.value)} />
        </Field>
      </form>
    </Drawer>
  );
}
