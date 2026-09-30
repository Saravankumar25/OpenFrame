// Project Files (FSD §40, §112; UX §3.38; mock 154): "What files belong with this film?"
// A light file cabinet: add (copy into the project or link where it is), folders,
// rename, move (drag onto a folder or "Move to…"), notes, open externally,
// reveal, export a copy, relink unavailable links, delete to Recently Deleted,
// and drag files in from Explorer.

import { useEffect, useMemo, useState, type KeyboardEvent as ReactKeyboardEvent } from "react";
import { DndContext, PointerSensor, useDraggable, useDroppable, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { ChevronDown, ExternalLink, Folder, FolderPlus, Files, Link2, MoreHorizontal, Pencil, Plus, Search, Trash2, Upload } from "lucide-react";
import { call, inTauri } from "../../ipc/client";
import { reportError, useOp } from "../../ipc/query";
import type { FilesListing } from "../../ipc/generated/FilesListing";
import type { FolderDto } from "../../ipc/generated/FolderDto";
import type { ProjectFileDto } from "../../ipc/generated/ProjectFileDto";
import { Button, Chip, ConfirmDialog, ContextMenu, Dialog, EmptyState, Field, Menu, PageHeader, Segmented, Skeleton, TextInput, type MenuItemSpec } from "../../design-system";
import { useNav } from "../../app/stores";
import { toast } from "../../app/toast";
import { useRememberLocation } from "../../app/useRememberLocation";
import { dayWord } from "../../app/home/format";
import { FileDetails, folderPathLabel } from "./FileDetails";
import {
  FILE_TABLES,
  addFiles,
  deleteFile,
  exportCopy,
  fileBadge,
  fileKind,
  folderOf,
  moveFile,
  openFile,
  pickAndAddFiles,
  relinkFile,
  revealFile,
  whereOf,
} from "./fileUtils";

/** Folder filter: "all" = every file, "" = top level only, otherwise a folder id. */
type FolderSel = "all" | string;

export default function FilesWorkspace() {
  const listing = useOp<FilesListing>("files.list", {}, FILE_TABLES);
  const route = useNav((s) => s.route);
  const [folder, setFolder] = useState<FolderSel>("all");
  const [view, setView] = useState<"list" | "grid">("list");
  const [text, setText] = useState("");
  const [selected, setSelected] = useState<string | null>(route.params?.fileId ?? null);
  const [renameSignal, setRenameSignal] = useState(0);
  const [folderDialog, setFolderDialog] = useState<{ mode: "create" | "rename"; folder?: FolderDto } | null>(null);
  const [deletingFolder, setDeletingFolder] = useState<FolderDto | null>(null);
  const [dropping, setDropping] = useState(false);
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 6 } }));

  // Search results / Project Home can open a specific file.
  useEffect(() => {
    if (route.params?.fileId) setSelected(route.params.fileId);
  }, [route.params?.fileId]);

  const folders = useMemo(() => listing.data?.folders ?? [], [listing.data]);
  const files = useMemo(() => listing.data?.files ?? [], [listing.data]);
  const current = files.find((f) => f.id === selected) ?? null;
  useRememberLocation(current ? { workspace: "files", label: current.displayName, fileId: current.id } : null);
  const targetFolderId = folder === "all" || folder === "" ? null : folder;

  // FSD §40 / UX §3.38: drag files in from Explorer to add them (stored in the project).
  useEffect(() => {
    if (!inTauri()) return;
    let un: (() => void) | undefined;
    let cancelled = false;
    void import("@tauri-apps/api/webview").then(({ getCurrentWebview }) =>
      getCurrentWebview()
        .onDragDropEvent((ev) => {
          const p = ev.payload;
          if (p.type === "enter" || p.type === "over") setDropping(true);
          else if (p.type === "leave") setDropping(false);
          else if (p.type === "drop") {
            setDropping(false);
            void addFiles(p.paths, "copy", targetFolderId);
          }
        })
        .then((f) => {
          if (cancelled) f();
          else un = f;
        }),
    );
    return () => {
      cancelled = true;
      un?.();
    };
  }, [targetFolderId]);

  const q = text.trim().toLowerCase();
  const shown = files.filter((f) => {
    if (folder !== "all" && (f.folderId ?? "") !== folder) return false;
    if (!q) return true;
    return [f.displayName, f.asset.originalName, f.notes ?? "", fileKind(f.asset.mediaType, f.asset.originalName)].some((s) => s.toLowerCase().includes(q));
  });
  const folderName = (id: string | null) => (id ? `“${folderPathLabel(folders, id)}”` : "the top level");

  const menuFor = (f: ProjectFileDto): MenuItemSpec[] => {
    const where = whereOf(f);
    return [
      where === "unavailable" && f.asset.storageMode === "external"
        ? { label: "Relink…", icon: <Link2 size={14} />, onSelect: () => void relinkFile(f) }
        : { label: "Open", icon: <ExternalLink size={14} />, disabled: where === "unavailable", onSelect: () => void openFile(f) },
      { label: "Details", onSelect: () => setSelected(f.id) },
      { label: "Rename", shortcut: "F2", icon: <Pencil size={14} />, onSelect: () => { setSelected(f.id); setRenameSignal((n) => n + 1); } },
      { label: "Move to", header: true, separatorBefore: true },
      { label: "Top level", disabled: !f.folderId, onSelect: () => void moveFile(f, null, "the top level") },
      ...folders.map((d) => ({ label: folderPathLabel(folders, d.id), disabled: f.folderId === d.id, onSelect: () => void moveFile(f, d.id, folderName(d.id)) })),
      { label: "Reveal in File Manager", separatorBefore: true, disabled: where === "unavailable", onSelect: () => void revealFile(f) },
      { label: "Export Copy…", disabled: where === "unavailable", onSelect: () => void exportCopy(f) },
      ...(f.asset.storageMode === "external" && where !== "unavailable" ? [{ label: "Relink…", onSelect: () => void relinkFile(f) }] : []),
      { label: "Delete", icon: <Trash2 size={14} />, danger: true, separatorBefore: true, shortcut: "Del", onSelect: () => void deleteFile(f).then((ok) => ok && selected === f.id && setSelected(null)) },
    ];
  };

  const onRowKey = (e: ReactKeyboardEvent, f: ProjectFileDto) => {
    if (e.key === "Enter") {
      e.preventDefault();
      setSelected(f.id);
    } else if (e.key === "F2") {
      e.preventDefault();
      setSelected(f.id);
      setRenameSignal((n) => n + 1);
    } else if (e.key === "Delete") {
      e.preventDefault();
      void deleteFile(f).then((ok) => ok && selected === f.id && setSelected(null));
    }
  };

  const onDragEnd = (e: DragEndEvent) => {
    const f = files.find((x) => x.id === e.active.id);
    const target = e.over?.id;
    if (!f || typeof target !== "string") return;
    const folderId = target === "folder:top" ? null : target.replace(/^folder:/, "");
    void moveFile(f, folderId, folderName(folderId));
  };

  const addMenu: MenuItemSpec[] = [
    { label: "Copy into project…", icon: <Upload size={14} />, onSelect: () => void pickAndAddFiles("copy", targetFolderId) },
    { label: "Link to file…", icon: <Link2 size={14} />, onSelect: () => void pickAndAddFiles("link", targetFolderId) },
  ];

  return (
    <div className="content fill" style={{ position: "relative" }}>
      <PageHeader
        title="Project Files"
        sub="What files belong with this film?"
        actions={
          <>
            <Button icon={<FolderPlus size={15} />} onClick={() => setFolderDialog({ mode: "create" })}>New Folder</Button>
            <Menu
              align="end"
              items={addMenu}
              trigger={<Button variant="primary" icon={<Plus size={15} />}>Add File <ChevronDown size={13} aria-hidden /></Button>}
            />
          </>
        }
      />
      <div className="toolbar">
        <div className="input" style={{ width: 260, padding: "0 10px" }}>
          <Search size={15} aria-hidden style={{ color: "var(--muted)" }} />
          <input type="search" aria-label="Search files" placeholder="Search files" value={text} onChange={(e) => setText(e.target.value)} style={{ border: 0, outline: "none", flex: 1, background: "transparent", minHeight: 30 }} />
        </div>
        <Segmented ariaLabel="View" value={view} onChange={setView} options={[{ value: "list", label: "List" }, { value: "grid", label: "Grid" }]} />
        <span className="sp" />
        <span className="xs muted">Drag files here from Explorer to add them.</span>
      </div>
      {listing.isLoading ? (
        <div className="col"><Skeleton h={32} /><Skeleton h={32} /><Skeleton h={32} /></div>
      ) : (
        <DndContext sensors={sensors} onDragEnd={onDragEnd}>
          <div className="row" style={{ alignItems: "stretch", gap: 14, flex: 1, minHeight: 0 }}>
            <FolderPanel
              folders={folders}
              files={files}
              value={folder}
              onChange={setFolder}
              onRename={(f) => setFolderDialog({ mode: "rename", folder: f })}
              onDelete={setDeletingFolder}
            />
            <div className="grow" style={{ overflow: "auto", minHeight: 0 }}>
              {files.length === 0 ? (
                <div className="dz" style={{ height: 260 }}>
                  <EmptyState
                    icon={<Files size={34} />}
                    title="Keep project documents and attachments here."
                    actions={<Button variant="primary" icon={<Plus size={15} />} onClick={() => void pickAndAddFiles("copy", targetFolderId)}>Add File</Button>}
                  >
                    Contracts, permits, research, pitch decks — anything that belongs with this film. Drag files in, or add them.
                  </EmptyState>
                </div>
              ) : shown.length === 0 ? (
                <div className="card muted" style={{ padding: 20, textAlign: "center" }}>
                  {q ? `No files match “${text}”.` : "This folder is empty. Drag files onto it, or use Move to… on a file."}
                </div>
              ) : view === "list" ? (
                <div className="card" style={{ overflow: "hidden" }}>
                  <table className="tbl" aria-label="Files">
                    <thead>
                      <tr><th>Name</th><th>Type</th><th>Where</th><th>Modified</th><th><span className="sr-only">Actions</span></th></tr>
                    </thead>
                    <tbody>
                      {shown.map((f) => (
                        <FileRow key={f.id} file={f} selected={selected === f.id} menu={menuFor(f)} onSelect={() => setSelected(f.id)} onKeyDown={(e) => onRowKey(e, f)} folderLabel={folder === "all" && f.folderId ? folderPathLabel(folders, f.folderId) : null} />
                      ))}
                    </tbody>
                  </table>
                </div>
              ) : (
                <div className="grid g4">
                  {shown.map((f) => (
                    <FileCard key={f.id} file={f} selected={selected === f.id} menu={menuFor(f)} onSelect={() => setSelected(f.id)} onKeyDown={(e) => onRowKey(e, f)} />
                  ))}
                </div>
              )}
            </div>
          </div>
        </DndContext>
      )}
      {dropping && (
        <div className="dz" aria-live="polite" style={{ position: "absolute", inset: 12, zIndex: 30, background: "#faf9f6ee", borderColor: "var(--accent)" }}>
          <Upload size={28} aria-hidden />
          <div className="b" style={{ marginTop: 8 }}>Drop to add to {targetFolderId ? folderName(targetFolderId) : "this project"}</div>
          <div className="sm muted">A copy is stored inside the project.</div>
        </div>
      )}
      {current && <FileDetails key={current.id} file={current} folders={folders} onClose={() => setSelected(null)} renameSignal={renameSignal} />}
      {folderDialog && (
        <FolderDialog
          mode={folderDialog.mode}
          folder={folderDialog.folder}
          parentId={folderDialog.mode === "create" ? targetFolderId : null}
          parentLabel={folderDialog.mode === "create" && targetFolderId ? folderPathLabel(folders, targetFolderId) : null}
          onClose={() => setFolderDialog(null)}
          onCreated={(id) => setFolder(id)}
        />
      )}
      {deletingFolder && (
        <ConfirmDialog
          open
          onOpenChange={(v) => !v && setDeletingFolder(null)}
          title={`Delete folder “${deletingFolder.name}”?`}
          confirmLabel="Delete Folder"
          danger
          onConfirm={() => {
            const f = deletingFolder;
            setDeletingFolder(null);
            void call("files.delete_folder", { id: f.id })
              .then(() => {
                if (folder === f.id) setFolder("all");
                toast.undoable(`Deleted folder “${f.name}”`);
              })
              .catch(reportError);
          }}
        >
          <p style={{ marginTop: 0 }}>Files and folders inside it move to the top level — no files are deleted. The folder goes to Recently Deleted.</p>
        </ConfirmDialog>
      )}
    </div>
  );
}

function WhereCell({ file }: { file: ProjectFileDto }) {
  const where = whereOf(file);
  return (
    <span className="row gap4 wrap">
      {where === "stored" && <Chip tone="g">Stored in project</Chip>}
      {where === "linked" && <Chip tone="b">Linked (external)</Chip>}
      {where === "unavailable" && <Chip tone="r">Unavailable</Chip>}
      {file.asset.storageMode === "external" && <span className="xs muted" title={file.asset.path ?? undefined}>{folderOf(file.asset.path)}</span>}
    </span>
  );
}

function FileRow({ file, selected, menu, onSelect, onKeyDown, folderLabel }: {
  file: ProjectFileDto;
  selected: boolean;
  menu: MenuItemSpec[];
  onSelect: () => void;
  onKeyDown: (e: ReactKeyboardEvent) => void;
  folderLabel: string | null;
}) {
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({ id: file.id });
  const where = whereOf(file);
  return (
    <ContextMenu items={menu}>
      <tr
        ref={setNodeRef}
        className={selected ? "sel" : undefined}
        aria-selected={selected}
        aria-label={`${file.displayName}, ${fileKind(file.asset.mediaType, file.asset.originalName)}, ${where === "stored" ? "stored in project" : where === "linked" ? "linked" : "unavailable"}`}
        onClick={onSelect}
        onDoubleClick={() => where !== "unavailable" && void openFile(file)}
        onKeyDown={onKeyDown}
        style={{ cursor: "default", opacity: isDragging ? 0.5 : 1 }}
        {...listeners}
        {...attributes}
        role="row"
      >
        <td>
          <b>{file.displayName}</b>
          {folderLabel && <div className="xs muted"><Folder size={10} aria-hidden /> {folderLabel}</div>}
        </td>
        <td>{fileKind(file.asset.mediaType, file.asset.originalName)}</td>
        <td><WhereCell file={file} /></td>
        <td>{dayWord(file.updatedAt)}</td>
        <td style={{ textAlign: "right", whiteSpace: "nowrap" }} onClick={(e) => e.stopPropagation()}>
          {where === "unavailable" && file.asset.storageMode === "external" ? (
            <Button size="xs" icon={<Link2 size={12} />} onClick={() => void relinkFile(file)}>Relink</Button>
          ) : (
            <Button size="xs" disabled={where === "unavailable"} onClick={() => void openFile(file)}>Open</Button>
          )}{" "}
          <Menu align="end" items={menu} trigger={<button className="iconbtn" aria-label={`More actions for ${file.displayName}`} style={{ width: 24, height: 24 }}><MoreHorizontal size={14} /></button>} />
        </td>
      </tr>
    </ContextMenu>
  );
}

function FileCard({ file, selected, menu, onSelect, onKeyDown }: { file: ProjectFileDto; selected: boolean; menu: MenuItemSpec[]; onSelect: () => void; onKeyDown: (e: ReactKeyboardEvent) => void }) {
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({ id: file.id });
  const badge = fileBadge(file.asset.mediaType, file.asset.originalName);
  return (
    <ContextMenu items={menu}>
      <div
        ref={setNodeRef}
        className="card filecard"
        aria-selected={selected}
        aria-label={file.displayName}
        onClick={onSelect}
        onDoubleClick={() => whereOf(file) !== "unavailable" && void openFile(file)}
        onKeyDown={onKeyDown}
        style={{ outline: selected ? "2px solid var(--blue)" : undefined, opacity: isDragging ? 0.5 : 1, cursor: "default" }}
        {...listeners}
        {...attributes}
        role="button"
      >
        <span className={`fic ${badge.cls}`} aria-hidden>{badge.text}</span>
        <div className="grow sm" style={{ minWidth: 0 }}>
          <b style={{ display: "block", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{file.displayName}</b>
          <WhereCell file={file} />
        </div>
      </div>
    </ContextMenu>
  );
}

function FolderItem({ id, label, count, active, depth, onClick, menu }: { id: string; label: string; count: number; active: boolean; depth: number; onClick: () => void; menu?: MenuItemSpec[] }) {
  const { setNodeRef, isOver } = useDroppable({ id, disabled: id === "folder:all" });
  const btn = (
    <button
      ref={setNodeRef}
      className="row"
      aria-current={active ? "true" : undefined}
      onClick={onClick}
      style={{
        width: "100%",
        border: 0,
        borderRadius: 7,
        padding: `6px 8px 6px ${8 + depth * 14}px`,
        background: isOver ? "var(--blue-soft)" : active ? "var(--accent-soft)" : "transparent",
        outline: isOver ? "2px dashed var(--blue)" : undefined,
        textAlign: "left",
        fontWeight: active ? 700 : 500,
        fontSize: 13,
      }}
    >
      <Folder size={14} aria-hidden />
      <span className="grow" style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{label}</span>
      <span className="xs muted">{count}</span>
    </button>
  );
  return menu ? <ContextMenu items={menu}>{btn}</ContextMenu> : btn;
}

function FolderPanel({ folders, files, value, onChange, onRename, onDelete }: {
  folders: FolderDto[];
  files: ProjectFileDto[];
  value: FolderSel;
  onChange: (v: FolderSel) => void;
  onRename: (f: FolderDto) => void;
  onDelete: (f: FolderDto) => void;
}) {
  const count = (id: string | null) => files.filter((f) => (f.folderId ?? null) === id).length;
  const ordered: { folder: FolderDto; depth: number }[] = [];
  const walk = (parent: string | null, depth: number) => {
    for (const f of folders.filter((x) => (x.parentId ?? null) === parent)) {
      ordered.push({ folder: f, depth });
      if (depth < 8) walk(f.id, depth + 1);
    }
  };
  walk(null, 0);
  return (
    <nav className="card" aria-label="Folders" style={{ width: 210, flex: "none", padding: 6, overflow: "auto", alignSelf: "flex-start", maxHeight: "100%" }}>
      <FolderItem id="folder:all" label="All files" count={files.length} active={value === "all"} depth={0} onClick={() => onChange("all")} />
      <FolderItem id="folder:top" label="Top level" count={count(null)} active={value === ""} depth={0} onClick={() => onChange("")} />
      {ordered.map(({ folder, depth }) => (
        <FolderItem
          key={folder.id}
          id={`folder:${folder.id}`}
          label={folder.name}
          count={count(folder.id)}
          active={value === folder.id}
          depth={depth + 1}
          onClick={() => onChange(folder.id)}
          menu={[
            { label: "Rename Folder", icon: <Pencil size={14} />, onSelect: () => onRename(folder) },
            { label: "Delete Folder…", icon: <Trash2 size={14} />, danger: true, separatorBefore: true, onSelect: () => onDelete(folder) },
          ]}
        />
      ))}
      {value !== "all" && value !== "" && (
        <div className="row" style={{ padding: "8px 4px 2px", gap: 4 }}>
          <Button size="xs" variant="ghost" icon={<Pencil size={12} />} onClick={() => { const f = folders.find((x) => x.id === value); if (f) onRename(f); }}>Rename</Button>
          <Button size="xs" variant="ghost" icon={<Trash2 size={12} />} onClick={() => { const f = folders.find((x) => x.id === value); if (f) onDelete(f); }}>Delete</Button>
        </div>
      )}
    </nav>
  );
}

function FolderDialog({ mode, folder, parentId, parentLabel, onClose, onCreated }: {
  mode: "create" | "rename";
  folder?: FolderDto;
  parentId: string | null;
  parentLabel: string | null;
  onClose: () => void;
  onCreated: (id: string) => void;
}) {
  const [name, setName] = useState(folder?.name ?? "");
  const [busy, setBusy] = useState(false);
  const invalid = name.trim().length === 0;
  const save = async () => {
    if (invalid || busy) return;
    setBusy(true);
    try {
      if (mode === "create") {
        const f = await call<FolderDto>("files.create_folder", { name: name.trim(), parentId });
        toast.undoable(`Created folder “${f.name}”`);
        onCreated(f.id);
      } else if (folder) {
        await call("files.rename_folder", { id: folder.id, name: name.trim(), expectedRev: folder.rev });
        toast.undoable(`Renamed folder to “${name.trim()}”`);
      }
      onClose();
    } catch (e) {
      reportError(e);
      setBusy(false);
    }
  };
  return (
    <Dialog open onOpenChange={(v) => !v && onClose()} title={mode === "create" ? "New Folder" : "Rename Folder"} sub={mode === "create" && parentLabel ? `Inside “${parentLabel}”` : undefined} size="sm"
      footer={<><Button onClick={onClose}>Cancel</Button><Button variant="primary" disabled={invalid || busy} onClick={() => void save()}>{mode === "create" ? "Create" : "Rename"}</Button></>}>
      <form onSubmit={(e) => { e.preventDefault(); void save(); }}>
        <Field label="Folder name" required htmlFor="fo-name">
          <TextInput id="fo-name" autoFocus value={name} maxLength={120} onChange={(e) => setName(e.target.value)} placeholder="e.g. Contracts" />
        </Field>
      </form>
    </Dialog>
  );
}
