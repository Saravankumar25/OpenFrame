// Project note drawer (FSD §106, §159; mock 151): title, body, pin, delete.
// Edits save on their own (coalesced into one undo step while typing).

import { useEffect, useRef, useState } from "react";
import { Pin, PinOff, Trash2 } from "lucide-react";
import { call } from "../../ipc/client";
import { reportError, useOp } from "../../ipc/query";
import type { NoteDto } from "../../ipc/generated/NoteDto";
import { Button, Drawer, Field, Skeleton, TextArea, TextInput } from "../../design-system";
import { toast } from "../../app/toast";
import { useRememberLocation } from "../../app/useRememberLocation";
import { dayAndTime } from "../../app/home/format";
import { useDebouncedSave } from "./useDebouncedSave";

export function noteLabel(n: Pick<NoteDto, "title" | "body">): string {
  if (n.title?.trim()) return n.title.trim();
  const first = n.body.split("\n").map((l) => l.trim()).find(Boolean) ?? "";
  if (!first) return "Untitled note";
  return first.length > 60 ? `${first.slice(0, 60)}…` : first;
}

export function NoteEditor({ noteId, onClose }: { noteId: string; onClose: () => void }) {
  const note = useOp<NoteDto>("notes.get", { id: noteId }, ["project_note", "project_member"]);
  if (note.isError) {
    return (
      <Drawer open onClose={onClose} typeLabel="Note" title="Note not available">
        <p className="muted">This note may have been deleted. Look in Recently Deleted to restore it.</p>
      </Drawer>
    );
  }
  if (!note.data) {
    return (
      <Drawer open onClose={onClose} typeLabel="Note" title="Loading…">
        <Skeleton h={32} />
      </Drawer>
    );
  }
  return <Editor note={note.data} onClose={onClose} />;
}

function Editor({ note, onClose }: { note: NoteDto; onClose: () => void }) {
  const [title, setTitle] = useState(note.title ?? "");
  const [body, setBody] = useState(note.body);
  const focused = useRef<"title" | "body" | null>(null);
  const { schedule, flush } = useDebouncedSave((v: { title: string; body: string }) => call("notes.update", { id: note.id, title: v.title, body: v.body }));
  useRememberLocation({ workspace: "notes", label: `Note “${noteLabel(note)}”`, noteId: note.id });

  // Reflect undo/redo and other changes, but never while that field is being edited.
  useEffect(() => {
    if (focused.current !== "title") setTitle(note.title ?? "");
  }, [note.title]);
  useEffect(() => {
    if (focused.current !== "body") setBody(note.body);
  }, [note.body]);

  const remove = async () => {
    flush();
    try {
      await call("notes.delete", { id: note.id });
      toast.undoable(`Moved note “${noteLabel(note)}” to Recently Deleted`);
      onClose();
    } catch (e) {
      reportError(e);
    }
  };
  const togglePin = async () => {
    try {
      await call("notes.set_pinned", { id: note.id, pinned: !note.pinned });
    } catch (e) {
      reportError(e);
    }
  };

  return (
    <Drawer
      open
      onClose={() => {
        flush();
        onClose();
      }}
      typeLabel="Project note"
      title={noteLabel({ title, body })}
      footer={
        <>
          <Button variant="ghost" icon={<Trash2 size={14} />} style={{ color: "var(--red)", marginRight: "auto" }} onClick={() => void remove()}>Delete</Button>
          <Button icon={note.pinned ? <PinOff size={14} /> : <Pin size={14} />} onClick={() => void togglePin()} aria-pressed={note.pinned}>
            {note.pinned ? "Unpin" : "Pin"}
          </Button>
        </>
      }
    >
      <Field label="Title" htmlFor="ne-title">
        <TextInput
          id="ne-title"
          value={title}
          maxLength={200}
          placeholder="Untitled note"
          onFocus={() => (focused.current = "title")}
          onBlur={() => {
            focused.current = null;
            flush();
          }}
          onChange={(e) => {
            setTitle(e.target.value);
            schedule({ title: e.target.value, body });
          }}
        />
      </Field>
      <Field label="Note" htmlFor="ne-body">
        <TextArea
          id="ne-body"
          autoFocus={!note.body}
          rows={14}
          value={body}
          placeholder="Write anything that doesn't belong in the Idea Vault or another workspace."
          onFocus={() => (focused.current = "body")}
          onBlur={() => {
            focused.current = null;
            flush();
          }}
          onChange={(e) => {
            setBody(e.target.value);
            schedule({ title, body: e.target.value });
          }}
        />
      </Field>
      <div className="xs muted">
        {note.createdByName ? `Added by ${note.createdByName} · ` : ""}Created {dayAndTime(note.createdAt)} · Changed {dayAndTime(note.updatedAt)}
      </div>
      <div className="hint" style={{ marginTop: 6 }}>Notes are kept out of formal documents such as screenplay and call-sheet exports.</div>
    </Drawer>
  );
}
