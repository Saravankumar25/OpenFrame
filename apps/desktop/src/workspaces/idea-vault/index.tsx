// Idea Vault workspace — Global and Project (FSD §5, §88; UX §3.5; mocks 024–047).
// "What have I collected for this film?" / "What do I have that might matter someday?"
// The primary action is Add, never classify. Views change presentation only.

import { useCallback, useEffect, useMemo, useRef, useState, type KeyboardEvent, type MouseEvent } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { FolderOpen, Lightbulb, Mic, Plus, Search, Trash2, X, Zap } from "lucide-react";
import { inTauri } from "../../ipc/client";
import { reportError, useOp } from "../../ipc/query";
import type { VaultAddFilesResult } from "../../ipc/generated/VaultAddFilesResult";
import type { ProjectSummary } from "../../ipc/generated/ProjectSummary";
import type { VaultCollectionDto } from "../../ipc/generated/VaultCollectionDto";
import type { VaultFolderDto } from "../../ipc/generated/VaultFolderDto";
import type { VaultItemDto } from "../../ipc/generated/VaultItemDto";
import type { VaultView } from "../../ipc/generated/VaultView";
import { Button, Chip, EmptyState, Menu, PageHeader, Segmented, Skeleton, type MenuItemSpec } from "../../design-system";
import { useIntent, useNav, useUi } from "../../app/stores";
import { toast } from "../../app/toast";
import { useVaultList, useVaultOverview, vault } from "./api";
import { DeletedView } from "./DeletedView";
import {
  AddResultDialog,
  AddToCollectionDialog,
  AddUrlDialog,
  COLLECTION_HINT,
  CopyDialog,
  MoveToFolderDialog,
  NameDialog,
  QuickCaptureDialog,
  RecordNoteDialog,
  SendToStoryDialog,
  SketchDialog,
  TagDialog,
} from "./dialogs";
import { ItemDrawer, type DrawerAction } from "./ItemDrawer";
import { FolderCrumbs, ItemCanvas } from "./ItemViews";
import { LeftPanel } from "./LeftPanel";
import { FILE_FILTERS, childFolders, folderPath, itemsPhrase, nextSelection, scopeLabel, type DisplayMode, type Scope } from "./model";
import "./vault.css";

type DialogState =
  | { kind: "url" }
  | { kind: "record" }
  | { kind: "sketch" }
  | { kind: "quick" }
  | { kind: "send"; item: VaultItemDto }
  | { kind: "copy"; items: VaultItemDto[] }
  | { kind: "move"; items: VaultItemDto[] }
  | { kind: "collection"; items: VaultItemDto[] }
  | { kind: "tag"; items: VaultItemDto[] }
  | { kind: "result"; result: VaultAddFilesResult }
  | { kind: "newCollection" }
  | { kind: "newFolder"; parentId: string | null }
  | { kind: "renameFolder"; folder: VaultFolderDto }
  | { kind: "renameCollection"; collection: VaultCollectionDto };

const MODE_KEY = "of.vault.mode";

function readMode(): DisplayMode {
  try {
    const v = localStorage.getItem(MODE_KEY);
    if (v === "grid" || v === "card" || v === "list" || v === "folder") return v;
  } catch {
    /* storage unavailable: default view */
  }
  return "grid";
}

function isEditable(t: EventTarget | null): boolean {
  const el = t as HTMLElement | null;
  if (!el) return false;
  return el.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(el.tagName);
}

export default function IdeaVaultWorkspace() {
  const route = useNav((s) => s.route);
  const go = useNav((s) => s.go);
  const project = useOp<ProjectSummary | null>("project.current", {}, ["project"]);
  const hasProject = !!project.data;
  const routeScope: Scope = route.params?.scope === "global" ? "global" : "project";
  const [scope, setScopeState] = useState<Scope>(routeScope);
  const effectiveScope: Scope = !hasProject && !project.isLoading ? "global" : scope;

  const [page, setPage] = useState<"vault" | "deleted">(route.sub === "deleted" ? "deleted" : "vault");
  const [view, setView] = useState<VaultView>({ kind: "all" });
  const [mode, setModeState] = useState<DisplayMode>(readMode);
  const [folderCursor, setFolderCursor] = useState<string | null>(null);
  const [searchText, setSearchText] = useState("");
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [anchor, setAnchor] = useState<string | null>(null);
  const [openId, setOpenId] = useState<string | null>(route.params?.itemId ?? null);
  const [focusTitle, setFocusTitle] = useState(false);
  const [dialog, setDialog] = useState<DialogState | null>(null);
  const [dropCount, setDropCount] = useState<number | null>(null);
  const searchRef = useRef<HTMLInputElement>(null);

  // Global undo/redo follows the vault being shown (the Global Vault has its own history).
  useEffect(() => {
    useUi.getState().setUndoScope(effectiveScope);
    return () => useUi.getState().setUndoScope("project");
  }, [effectiveScope]);

  // Navigation from search / Continue: { itemId, scope }.
  useEffect(() => {
    if (route.workspace !== "vault") return;
    if (route.params?.scope === "global" || route.params?.itemId) setScopeState(routeScope);
    if (route.params?.itemId) {
      setPage("vault");
      setOpenId(route.params.itemId);
    }
    if (route.sub === "deleted") setPage("deleted");
  }, [route, routeScope]);

  const setScope = (s: Scope) => {
    setScopeState(s);
    setView({ kind: "all" });
    setFolderCursor(null);
    setSelected(new Set());
    setOpenId(null);
  };
  const setMode = (m: DisplayMode) => {
    setModeState(m);
    try {
      localStorage.setItem(MODE_KEY, m);
    } catch {
      /* per-viewer convenience only */
    }
  };

  useEffect(() => {
    const t = window.setTimeout(() => setSearch(searchText), 200);
    return () => window.clearTimeout(t);
  }, [searchText]);

  const overview = useVaultOverview(effectiveScope);
  const listView: VaultView = useMemo(
    () => (mode === "folder" ? { kind: "folder", folderId: folderCursor } : view),
    [mode, folderCursor, view],
  );
  const list = useVaultList(effectiveScope, listView, search);
  const items = useMemo(() => list.data ?? [], [list.data]);
  const collections = useMemo(() => overview.data?.collections ?? [], [overview.data]);
  const folders = useMemo(() => overview.data?.folders ?? [], [overview.data]);
  const selectedItems = useMemo(() => items.filter((i) => selected.has(i.id)), [items, selected]);

  // Drop selections that are no longer visible.
  useEffect(() => {
    setSelected((s) => {
      if (s.size === 0) return s;
      const visible = new Set(items.map((i) => i.id));
      const next = new Set([...s].filter((id) => visible.has(id)));
      return next.size === s.size ? s : next;
    });
  }, [items]);

  // Where new items land: the folder/collection currently shown.
  const addContext = useMemo(() => {
    const v = listView;
    return {
      folderId: v.kind === "folder" ? v.folderId : null,
      collectionId: v.kind === "collection" ? v.collectionId : null,
    };
  }, [listView]);

  const copyLabel = effectiveScope === "global" ? (hasProject ? "Copy to Project" : null) : "Copy to Global Vault";
  const projectLabel = project.data ? `${project.data.title} (${project.data.projectType})` : "";

  // ------------------------------------------------------------------ adding

  const addFiles = useCallback(
    async (paths: string[]) => {
      if (paths.length === 0) return;
      try {
        const r = await vault.addFiles({ store: effectiveScope, paths, folderId: addContext.folderId, collectionId: addContext.collectionId });
        if (r.failed.length > 0) setDialog({ kind: "result", result: r });
        else toast.undoable(r.added.length === 1 ? `Added “${r.added[0].displayName}”` : `Added ${r.added.length} items`, effectiveScope);
      } catch (e) {
        reportError(e);
      }
    },
    [effectiveScope, addContext],
  );
  const addFilesRef = useRef(addFiles);
  useEffect(() => {
    addFilesRef.current = addFiles;
  }, [addFiles]);

  const pickFiles = async (kind: "image" | "video" | "audio" | "file") => {
    try {
      const picked = await openDialog({
        multiple: true,
        directory: false,
        title: "Add to the Idea Vault",
        filters: kind === "file" ? undefined : FILE_FILTERS[kind].map((f) => ({ name: f.name, extensions: [...f.extensions] })),
      });
      const paths = picked == null ? [] : Array.isArray(picked) ? picked : [picked];
      await addFiles(paths);
    } catch (e) {
      reportError(e);
    }
  };

  const createText = async (itemType: "note" | "quote") => {
    try {
      const item = await vault.create({ store: effectiveScope, itemType, folderId: addContext.folderId, collectionId: addContext.collectionId });
      setOpenId(item.id);
      setFocusTitle(false);
    } catch (e) {
      reportError(e);
    }
  };

  const onCreated = (item: VaultItemDto) => {
    setDialog(null);
    toast.undoable(`Added “${item.displayName}”`, effectiveScope);
  };

  // Drag files from Explorer into the visible vault (FSD §88.2, mock 034).
  useEffect(() => {
    if (!inTauri() || page !== "vault") return;
    let unlisten: (() => void) | undefined;
    let alive = true;
    getCurrentWebview()
      .onDragDropEvent((ev) => {
        const p = ev.payload;
        if (p.type === "enter") setDropCount(p.paths.length);
        else if (p.type === "leave") setDropCount(null);
        else if (p.type === "drop") {
          setDropCount(null);
          void addFilesRef.current(p.paths);
        }
      })
      .then((fn) => {
        if (alive) unlisten = fn;
        else fn();
      })
      .catch(() => undefined);
    return () => {
      alive = false;
      unlisten?.();
    };
  }, [page]);

  // Quick Capture from the shell's quick-actions menu, or Ctrl+Shift+N / Ctrl+F here.
  const intent = useIntent((s) => s.intent);
  useEffect(() => {
    if (intent === "vault.new_note" && useIntent.getState().consume("vault.new_note")) {
      setPage("vault");
      setDialog({ kind: "quick" });
    }
  }, [intent]);
  useEffect(() => {
    const onKey = (e: globalThis.KeyboardEvent) => {
      const mod = e.ctrlKey || e.metaKey;
      if (mod && e.shiftKey && e.key.toLowerCase() === "n") {
        e.preventDefault();
        setDialog({ kind: "quick" });
      } else if (mod && !e.shiftKey && e.key.toLowerCase() === "f") {
        e.preventDefault();
        setPage("vault");
        window.setTimeout(() => searchRef.current?.focus(), 0);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  // --------------------------------------------------------------- actions

  const deleteItems = async (xs: VaultItemDto[]) => {
    if (xs.length === 0) return;
    try {
      await vault.remove(effectiveScope, xs.map((i) => i.id));
      toast.undoable(`Deleted ${itemsPhrase(xs)}. You can restore from Recently Deleted.`, effectiveScope);
      setSelected(new Set());
      if (openId && xs.some((i) => i.id === openId)) setOpenId(null);
    } catch (e) {
      reportError(e);
    }
  };

  const setPinned = async (xs: VaultItemDto[], pinned: boolean) => {
    try {
      await vault.setPinned(effectiveScope, xs.map((i) => i.id), pinned);
      toast.undoable(`${pinned ? "Pinned" : "Unpinned"} ${itemsPhrase(xs)}`, effectiveScope);
    } catch (e) {
      reportError(e);
    }
  };

  const onDrawerAction = (action: DrawerAction, item: VaultItemDto) => {
    switch (action) {
      case "send":
        setDialog({ kind: "send", item });
        break;
      case "copy":
        setDialog({ kind: "copy", items: [item] });
        break;
      case "move":
        setDialog({ kind: "move", items: [item] });
        break;
      case "collection":
        setDialog({ kind: "collection", items: [item] });
        break;
      case "delete":
        void deleteItems([item]);
        break;
    }
  };

  const targetsFor = (item: VaultItemDto) => (selected.has(item.id) && selected.size > 1 ? selectedItems : [item]);

  const itemMenu = (item: VaultItemDto): MenuItemSpec[] => {
    const xs = targetsFor(item);
    const many = xs.length > 1;
    return [
      { label: "Open", shortcut: "Enter", onSelect: () => setOpenId(item.id) },
      { label: "Rename", shortcut: "F2", onSelect: () => { setOpenId(item.id); setFocusTitle(true); } },
      { label: many ? `Move ${xs.length} items…` : "Move…", onSelect: () => setDialog({ kind: "move", items: xs }) },
      { label: xs.every((i) => i.pinned) ? "Unpin" : "Pin", onSelect: () => void setPinned(xs, !xs.every((i) => i.pinned)) },
      ...(copyLabel ? [{ label: copyLabel, onSelect: () => setDialog({ kind: "copy", items: xs }) }] : []),
      ...(hasProject && !many ? [{ label: "Send to Story", onSelect: () => setDialog({ kind: "send", item }) }] : []),
      { label: many ? `Delete ${xs.length} items` : "Delete", shortcut: "Del", danger: true, separatorBefore: true, onSelect: () => void deleteItems(xs) },
    ];
  };

  const folderMenu = (f: VaultFolderDto): MenuItemSpec[] => [
    { label: "Open", onSelect: () => openFolder(f.id) },
    { label: "Rename…", onSelect: () => setDialog({ kind: "renameFolder", folder: f }) },
    { label: "New Folder Inside…", onSelect: () => setDialog({ kind: "newFolder", parentId: f.id }) },
    {
      label: "Delete Folder",
      danger: true,
      separatorBefore: true,
      onSelect: () =>
        void vault
          .deleteFolder(effectiveScope, f.id)
          .then(() => {
            toast.undoable(`Deleted folder “${f.name}”. Its items were kept and moved up a level.`, effectiveScope);
            if (folderCursor === f.id) setFolderCursor(f.parentId);
            if (view.kind === "folder" && view.folderId === f.id) setView({ kind: "all" });
          })
          .catch(reportError),
    },
  ];

  const collectionMenu = (c: VaultCollectionDto): MenuItemSpec[] => [
    { label: "Rename…", onSelect: () => setDialog({ kind: "renameCollection", collection: c }) },
    {
      label: "Delete Collection",
      danger: true,
      separatorBefore: true,
      onSelect: () =>
        void vault
          .deleteCollection(effectiveScope, c.id)
          .then(() => {
            toast.undoable(`Deleted collection “${c.name}”. Its items were kept.`, effectiveScope);
            if (view.kind === "collection" && view.collectionId === c.id) setView({ kind: "all" });
          })
          .catch(reportError),
    },
  ];

  const openFolder = (id: string | null) => {
    setSelected(new Set());
    if (mode === "folder") setFolderCursor(id);
    else setView(id ? { kind: "folder", folderId: id } : { kind: "all" });
  };

  const onPanelView = (v: VaultView) => {
    setSelected(new Set());
    setPage("vault");
    if (v.kind === "folder" && mode === "folder") setFolderCursor(v.folderId);
    else {
      if (mode === "folder") setMode("grid");
      setView(v);
    }
  };

  const ordered = useMemo(() => items.map((i) => i.id), [items]);
  const handlers = useMemo(
    () => ({
      onSelect: (item: VaultItemDto, e: MouseEvent | KeyboardEvent) => {
        const mods = { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey };
        setSelected((s) => nextSelection(s, ordered, item.id, anchor, mods));
        if (!mods.shift) setAnchor(item.id);
      },
      onToggle: (item: VaultItemDto) => {
        setSelected((s) => nextSelection(s, ordered, item.id, anchor, { ctrl: true }));
        setAnchor(item.id);
      },
      onOpen: (item: VaultItemDto) => {
        setOpenId(item.id);
        setFocusTitle(false);
      },
    }),
    [ordered, anchor],
  );

  const onCanvasKey = (e: KeyboardEvent<HTMLDivElement>) => {
    if (isEditable(e.target)) return;
    const mod = e.ctrlKey || e.metaKey;
    if (e.key === "Delete" && selectedItems.length > 0) {
      e.preventDefault();
      void deleteItems(selectedItems);
    } else if (e.key === "Escape" && selected.size > 0) {
      setSelected(new Set());
    } else if (mod && e.key.toLowerCase() === "a") {
      e.preventDefault();
      setSelected(new Set(ordered));
    } else if (e.key === "F2" && selectedItems.length > 0) {
      e.preventDefault();
      setOpenId(selectedItems[0].id);
      setFocusTitle(true);
    }
  };

  // ------------------------------------------------------------------ render

  if (page === "deleted") {
    return <DeletedView scope={effectiveScope} onBack={() => { setPage("vault"); if (route.sub === "deleted") go({ workspace: "vault" }); }} />;
  }

  const addMenu: MenuItemSpec[] = [
    { label: "Note", onSelect: () => void createText("note") },
    { label: "Image", onSelect: () => void pickFiles("image") },
    { label: "URL", onSelect: () => setDialog({ kind: "url" }) },
    { label: "File", onSelect: () => void pickFiles("file") },
    { label: "Audio / Voice Note", onSelect: () => setDialog({ kind: "record" }) },
    { label: "Audio file…", onSelect: () => void pickFiles("audio") },
    { label: "Video", onSelect: () => void pickFiles("video") },
    { label: "Quote", onSelect: () => void createText("quote") },
    { label: "Sketch", onSelect: () => setDialog({ kind: "sketch" }) },
    { label: "Quick Capture", shortcut: "Ctrl+Shift+N", separatorBefore: true, onSelect: () => setDialog({ kind: "quick" }) },
  ];

  const isGlobal = effectiveScope === "global";
  const total = overview.data?.total ?? 0;
  const searching = search.trim().length > 0;
  const path = folderPath(folders, folderCursor);

  const emptyNode = (() => {
    if (searching) {
      return <EmptyState icon={<Search size={28} />} title={`No results for “${search.trim()}”`}>Search looks at titles, note text, captions, filenames and tags.</EmptyState>;
    }
    if (total === 0) {
      return (
        <EmptyState
          icon={<Lightbulb size={30} />}
          title={isGlobal ? "Drop any film idea, reference or file here." : "Start collecting anything about this film."}
          actions={
            <>
              <Menu trigger={<Button variant="primary" icon={<Plus size={14} />}>Add</Button>} items={addMenu} />
              <Button icon={<Mic size={14} />} onClick={() => setDialog({ kind: "record" })}>Record Note</Button>
            </>
          }
        >
          This is where you throw anything about the film. Add a note, image, link, file, or voice note. You never have to decide what it is first.
        </EmptyState>
      );
    }
    const v = listView;
    const msg =
      v.kind === "pinned" ? "Nothing pinned yet. Pin items to keep them at the top."
      : v.kind === "folder" ? (mode === "folder" && childFolders(folders, folderCursor).length > 0 ? null : "This folder is empty. Move items here or add new ones while it's open.")
      : v.kind === "collection" ? "This collection is empty. Add items to it from their menu or the selection bar."
      : v.kind === "tag" ? "No items have this tag any more."
      : "Nothing here yet.";
    return msg ? <EmptyState title={msg} /> : null;
  })();

  return (
    <div className="content vv-root" onKeyDown={onCanvasKey}>
      <PageHeader
        title="Idea Vault"
        badge={<span style={{ verticalAlign: "middle" }}>{isGlobal ? <Chip tone="p">Global</Chip> : <Chip tone="a">{project.data?.title ?? "Project"}</Chip>}</span>}
        sub={
          isGlobal
            ? "Ideas across your whole filmmaking life — copy them into a film when they are ready."
            : "Everything collected for this film. Nothing here is classified or synced to the screenplay."
        }
        actions={
          <>
            <Button icon={<Zap size={14} />} onClick={() => setDialog({ kind: "quick" })}>Quick Capture</Button>
            <Button icon={<Trash2 size={14} />} onClick={() => setPage("deleted")}>Recently Deleted</Button>
          </>
        }
      />

      <div className="toolbar">
        {hasProject && (
          <Segmented<Scope>
            ariaLabel="Which Idea Vault"
            value={effectiveScope}
            onChange={setScope}
            options={[{ value: "global", label: "Global" }, { value: "project", label: "This Project" }]}
          />
        )}
        <Menu trigger={<Button variant="primary" icon={<Plus size={14} />}>Add</Button>} items={addMenu} />
        <Button icon={<Mic size={14} />} onClick={() => setDialog({ kind: "record" })}>Record Note</Button>
        <label className="input vv-search">
          <Search size={14} aria-hidden className="muted" />
          <input
            ref={searchRef}
            value={searchText}
            aria-label="Search ideas, tags, filenames"
            placeholder="Search ideas, tags, filenames…"
            onChange={(e) => setSearchText(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") setSearchText("");
            }}
          />
          {searchText && (
            <button type="button" className="vv-x" aria-label="Clear search" onClick={() => setSearchText("")}>
              <X size={12} />
            </button>
          )}
        </label>
        <div className="sp" />
        <Segmented<DisplayMode>
          ariaLabel="View"
          value={mode}
          onChange={(m) => {
            if (m === "folder") setFolderCursor(view.kind === "folder" ? view.folderId : null);
            setMode(m);
          }}
          options={[
            { value: "grid", label: "Grid" },
            { value: "card", label: "Card" },
            { value: "list", label: "List" },
            { value: "folder", label: "Folder" },
          ]}
        />
      </div>

      <div className="vv-body">
        <LeftPanel
          overview={overview.data}
          view={mode === "folder" ? { kind: "folder", folderId: folderCursor } : view}
          onView={onPanelView}
          onNewCollection={() => setDialog({ kind: "newCollection" })}
          onNewFolder={() => setDialog({ kind: "newFolder", parentId: mode === "folder" ? folderCursor : view.kind === "folder" ? view.folderId : null })}
          collectionMenu={collectionMenu}
          folderMenu={folderMenu}
        />
        <div className="vv-main rel">
          {mode === "folder" && <FolderCrumbs path={path} onGo={openFolder} />}
          {searching && list.data && (
            <div className="sm muted" style={{ marginBottom: 6 }} role="status">
              {items.length} {items.length === 1 ? "result" : "results"} for “{search.trim()}”
            </div>
          )}
          {list.isLoading && !list.data ? (
            <div className="col" style={{ gap: 8 }}>
              <Skeleton h={90} />
              <Skeleton h={90} />
            </div>
          ) : (
            <ItemCanvas
              mode={mode}
              items={items}
              selected={selected}
              collections={collections}
              handlers={handlers}
              itemMenu={itemMenu}
              folders={mode === "folder" && !searching ? childFolders(folders, folderCursor) : undefined}
              onOpenFolder={(f) => openFolder(f.id)}
              folderMenu={folderMenu}
              splitPinned={view.kind === "all" && !searching}
              empty={emptyNode}
            />
          )}
          {selected.size > 0 && (
            <div className="mbar" role="toolbar" aria-label="Selected items">
              <b>{selected.size} selected</b>
              <span style={{ opacity: 0.4 }}>|</span>
              <Button size="sm" onClick={() => setDialog({ kind: "move", items: selectedItems })}>Move to Folder</Button>
              <Button size="sm" onClick={() => setDialog({ kind: "collection", items: selectedItems })}>Add to Collection</Button>
              <Button size="sm" onClick={() => setDialog({ kind: "tag", items: selectedItems })}>Tag</Button>
              <Button size="sm" onClick={() => void setPinned(selectedItems, !selectedItems.every((i) => i.pinned))}>
                {selectedItems.every((i) => i.pinned) ? "Unpin" : "Pin"}
              </Button>
              {copyLabel && <Button size="sm" onClick={() => setDialog({ kind: "copy", items: selectedItems })}>{copyLabel}</Button>}
              <Button size="sm" variant="danger" onClick={() => void deleteItems(selectedItems)}>Delete</Button>
              <span className="grow" />
              <button type="button" className="vv-x" style={{ color: "#fff" }} aria-label="Clear selection" onClick={() => setSelected(new Set())}>
                <X size={15} />
              </button>
            </div>
          )}
          {dropCount !== null && (
            <div className="vv-drop" role="status">
              <FolderOpen size={30} aria-hidden />
              <div className="b" style={{ fontSize: 16 }}>
                Drop to add {dropCount} {dropCount === 1 ? "item" : "items"} to the {isGlobal ? "Global Vault" : "Project Vault"}
              </div>
              <div className="sm">No categories needed. They will be stored exactly as they are.</div>
            </div>
          )}
        </div>
      </div>

      {openId && (
        <ItemDrawer
          scope={effectiveScope}
          itemId={openId}
          collections={collections}
          folders={folders}
          copyLabel={copyLabel}
          canSendToStory={hasProject}
          focusTitle={focusTitle}
          onClose={() => {
            setOpenId(null);
            setFocusTitle(false);
          }}
          onAction={onDrawerAction}
        />
      )}

      {dialog?.kind === "url" && <AddUrlDialog scope={effectiveScope} onClose={() => setDialog(null)} onCreated={onCreated} />}
      {dialog?.kind === "record" && (
        <RecordNoteDialog
          scope={effectiveScope}
          onClose={() => setDialog(null)}
          onSaved={onCreated}
          onChooseFile={() => {
            setDialog(null);
            void pickFiles("audio");
          }}
        />
      )}
      {dialog?.kind === "sketch" && <SketchDialog scope={effectiveScope} onClose={() => setDialog(null)} onSaved={onCreated} />}
      {dialog?.kind === "quick" && (
        <QuickCaptureDialog
          scope={effectiveScope}
          context={isGlobal ? "Saving to your Global Idea Vault" : `Saving to ${project.data?.title ?? "this project"} · ${scopeLabel(effectiveScope)}`}
          onClose={() => setDialog(null)}
        />
      )}
      {dialog?.kind === "send" && <SendToStoryDialog scope={effectiveScope} item={dialog.item} onClose={() => setDialog(null)} />}
      {dialog?.kind === "copy" && (
        <CopyDialog from={effectiveScope} items={dialog.items} projectLabel={projectLabel} onClose={() => setDialog(null)} onDone={() => setSelected(new Set())} />
      )}
      {dialog?.kind === "move" && <MoveToFolderDialog scope={effectiveScope} items={dialog.items} folders={folders} onClose={() => setDialog(null)} />}
      {dialog?.kind === "collection" && (
        <AddToCollectionDialog scope={effectiveScope} items={dialog.items} collections={collections} onClose={() => setDialog(null)} />
      )}
      {dialog?.kind === "tag" && <TagDialog scope={effectiveScope} items={dialog.items} onClose={() => setDialog(null)} />}
      {dialog?.kind === "result" && (
        <AddResultDialog
          result={dialog.result}
          onClose={() => setDialog(null)}
          onRetry={(paths) => {
            setDialog(null);
            void addFiles(paths);
          }}
        />
      )}
      {dialog?.kind === "newCollection" && (
        <NameDialog
          title="New Collection"
          label="Name"
          hint={COLLECTION_HINT}
          confirmLabel="Create"
          onClose={() => setDialog(null)}
          onSubmit={async (name) => {
            const c = await vault.createCollection(effectiveScope, name);
            onPanelView({ kind: "collection", collectionId: c.id });
          }}
        />
      )}
      {dialog?.kind === "newFolder" && (
        <NameDialog
          title="New Folder"
          label="Folder name"
          confirmLabel="Create"
          onClose={() => setDialog(null)}
          onSubmit={(name) => vault.createFolder(effectiveScope, name, dialog.parentId)}
        />
      )}
      {dialog?.kind === "renameFolder" && (
        <NameDialog
          title="Rename Folder"
          label="Folder name"
          confirmLabel="Rename"
          initial={dialog.folder.name}
          onClose={() => setDialog(null)}
          onSubmit={(name) => vault.renameFolder(effectiveScope, dialog.folder.id, name)}
        />
      )}
      {dialog?.kind === "renameCollection" && (
        <NameDialog
          title="Rename Collection"
          label="Name"
          confirmLabel="Rename"
          initial={dialog.collection.name}
          onClose={() => setDialog(null)}
          onSubmit={(name) => vault.renameCollection(effectiveScope, dialog.collection.id, name)}
        />
      )}
    </div>
  );
}
