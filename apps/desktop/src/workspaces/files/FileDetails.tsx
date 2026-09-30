// File detail drawer (FSD §40.2, §62 "Missing linked file"): rename the display
// name, move to a folder, notes, open externally, reveal, export a copy,
// relink, delete to Recently Deleted. Simple edits save on their own.

import { useEffect, useRef, useState } from "react";
import { Download, ExternalLink, FolderSearch, Link2, Trash2 } from "lucide-react";
import { call } from "../../ipc/client";
import { reportError } from "../../ipc/query";
import type { FolderDto } from "../../ipc/generated/FolderDto";
import type { ProjectFileDto } from "../../ipc/generated/ProjectFileDto";
import { Banner, Button, Chip, Drawer, Field, Select, TextArea, TextInput } from "../../design-system";
import { toast } from "../../app/toast";
import { dayAndTime, fileSize } from "../../app/home/format";
import { deleteFile, exportCopy, fileKind, moveFile, openFile, relinkFile, revealFile, whereOf } from "./fileUtils";

export function folderPathLabel(folders: FolderDto[], id: string | null): string {
  if (!id) return "Top level";
  const byId = new Map(folders.map((f) => [f.id, f]));
  const parts: string[] = [];
  let cur = byId.get(id);
  let guard = 0;
  while (cur && guard++ < 32) {
    parts.unshift(cur.name);
    cur = cur.parentId ? byId.get(cur.parentId) : undefined;
  }
  return parts.join(" / ") || "Top level";
}

export function FileDetails({ file, folders, onClose, renameSignal }: { file: ProjectFileDto; folders: FolderDto[]; onClose: () => void; renameSignal: number }) {
  const [name, setName] = useState(file.displayName);
  const [notes, setNotes] = useState(file.notes ?? "");
  const nameRef = useRef<HTMLInputElement>(null);
  const notesTimer = useRef<number | undefined>(undefined);
  const where = whereOf(file);

  const notesFocused = useRef(false);
  // Keep buffers in sync when the file changes elsewhere (undo, redo) — but never
  // overwrite text while the user is typing in that field.
  useEffect(() => {
    if (document.activeElement !== nameRef.current) setName(file.displayName);
  }, [file.displayName]);
  useEffect(() => {
    if (!notesFocused.current) setNotes(file.notes ?? "");
  }, [file.id, file.notes]);
  useEffect(() => {
    if (renameSignal > 0) {
      nameRef.current?.focus();
      nameRef.current?.select();
    }
  }, [renameSignal]);
  // Never lose the last keystrokes: flush a pending notes save when the drawer closes.
  const pending = useRef<{ id: string; notes: string } | null>(null);
  useEffect(
    () => () => {
      window.clearTimeout(notesTimer.current);
      const p = pending.current;
      if (p) void call("files.update_notes", { id: p.id, notes: p.notes.trim() ? p.notes : null }).catch(reportError);
    },
    [],
  );

  const commitName = async () => {
    const n = name.trim();
    if (!n) {
      setName(file.displayName);
      return;
    }
    if (n === file.displayName) return;
    try {
      await call("files.rename", { id: file.id, name: n, expectedRev: file.rev });
      toast.undoable(`Renamed file to “${n}”`);
    } catch (e) {
      reportError(e);
      setName(file.displayName);
    }
  };
  const saveNotes = (v: string) => {
    setNotes(v);
    pending.current = { id: file.id, notes: v };
    window.clearTimeout(notesTimer.current);
    notesTimer.current = window.setTimeout(() => {
      pending.current = null;
      void call("files.update_notes", { id: file.id, notes: v.trim() ? v : null }).catch(reportError);
    }, 600);
  };

  const folderOptions = [{ value: "", label: "Top level" }, ...folders.map((f) => ({ value: f.id, label: folderPathLabel(folders, f.id) }))].sort((a, b) =>
    a.value === "" ? -1 : b.value === "" ? 1 : a.label.localeCompare(b.label),
  );

  return (
    <Drawer
      open
      onClose={onClose}
      typeLabel="File"
      title={file.displayName}
      footer={
        <>
          <Button variant="ghost" icon={<Trash2 size={14} />} onClick={() => void deleteFile(file).then((ok) => ok && onClose())} style={{ color: "var(--red)", marginRight: "auto" }}>
            Delete
          </Button>
          {where === "unavailable" && file.asset.storageMode === "external" ? (
            <Button variant="primary" icon={<Link2 size={14} />} onClick={() => void relinkFile(file)}>Relink…</Button>
          ) : (
            <Button variant="primary" icon={<ExternalLink size={14} />} disabled={where === "unavailable"} onClick={() => void openFile(file)}>Open</Button>
          )}
        </>
      }
    >
      {where === "unavailable" && (
        <div style={{ marginBottom: 10 }}>
          <Banner tone="warn">
            {file.asset.storageMode === "external"
              ? "This linked file can't be found. It may be on a drive that isn't connected, or it was moved. Relink it — the reference is never deleted."
              : "This file is missing from the project folder. Restore the project folder from a backup, or delete this entry."}
          </Banner>
        </div>
      )}
      <Field label="Name" htmlFor="fd-name" hint="The name shown in OpenFrame. The file itself is not renamed.">
        <TextInput
          id="fd-name"
          ref={nameRef}
          value={name}
          maxLength={255}
          onChange={(e) => setName(e.target.value)}
          onBlur={() => void commitName()}
          onKeyDown={(e) => {
            if (e.key === "Enter") e.currentTarget.blur();
            if (e.key === "Escape") {
              setName(file.displayName);
              e.stopPropagation();
            }
          }}
        />
      </Field>
      <Field label="Folder" htmlFor="fd-folder">
        <Select
          id="fd-folder"
          value={file.folderId ?? ""}
          options={folderOptions}
          onChange={(v) => void moveFile(file, v || null, v ? `“${folderPathLabel(folders, v)}”` : "the top level")}
        />
      </Field>
      <div className="field">
        <label>Where</label>
        <div className="row wrap">
          {where === "stored" && <Chip tone="g">Stored in project</Chip>}
          {where === "linked" && <Chip tone="b">Linked (external)</Chip>}
          {where === "unavailable" && <Chip tone="r">Unavailable</Chip>}
          <span className="sm muted">{fileKind(file.asset.mediaType, file.asset.originalName)}{file.asset.byteSize != null ? ` · ${fileSize(file.asset.byteSize)}` : ""}</span>
        </div>
        {file.asset.storageMode === "external" && file.asset.path && <div className="xs muted selectable-text" style={{ wordBreak: "break-all", marginTop: 4 }}>{file.asset.path}</div>}
        <div className="hint">
          {file.asset.storageMode === "external"
            ? "Linked files stay where they are. Moving or renaming the original breaks the link until you relink it."
            : "A copy is stored inside the project folder and travels with the project."}
        </div>
      </div>
      <Field label="Notes" htmlFor="fd-notes">
        <TextArea id="fd-notes" rows={5} value={notes} maxLength={20000} placeholder="What is this file, and why does it matter?" onFocus={() => (notesFocused.current = true)} onBlur={() => (notesFocused.current = false)} onChange={(e) => saveNotes(e.target.value)} />
      </Field>
      <div className="xs muted" style={{ marginBottom: 12 }}>
        Original file name: <span className="selectable-text">{file.asset.originalName}</span>
        <br />
        Added {dayAndTime(file.createdAt)} · Changed {dayAndTime(file.updatedAt)}
      </div>
      <div className="row wrap">
        <Button size="sm" icon={<FolderSearch size={14} />} disabled={where === "unavailable"} onClick={() => void revealFile(file)}>Reveal in File Manager</Button>
        <Button size="sm" icon={<Download size={14} />} disabled={where === "unavailable"} onClick={() => void exportCopy(file)}>Export Copy…</Button>
        {file.asset.storageMode === "external" && where !== "unavailable" && (
          <Button size="sm" icon={<Link2 size={14} />} onClick={() => void relinkFile(file)}>Relink…</Button>
        )}
      </div>
    </Drawer>
  );
}
