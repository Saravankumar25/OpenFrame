// The screenplay workspace in Standard, Writing Room and Focus modes
// (FSD §15.3, §17; UX §3.11–3.17; mocks 076–090, 097–101).

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  ChevronDown,
  Columns2,
  Download,
  FileText,
  GitCompare,
  Layers,
  Lock,
  MessageSquare,
  NotebookPen,
  Search,
  Users,
} from "lucide-react";
import { Banner, Button, Chip, Menu, Skeleton, type MenuItemSpec } from "../../design-system";
import { call } from "../../ipc/client";
import { reportError } from "../../ipc/query";
import type { ScreenplayOverview } from "../../ipc/generated/ScreenplayOverview";
import type { ScreenplayDraftDto } from "../../ipc/generated/ScreenplayDraftDto";
import type { ScreenplayEditResult } from "../../ipc/generated/ScreenplayEditResult";
import type { ScreenplaySceneHub } from "../../ipc/generated/ScreenplaySceneHub";
import type { CommentDto } from "../../ipc/generated/CommentDto";
import type { ElementType } from "../../ipc/generated/ElementType";
import { useNav } from "../../app/stores";
import { navigateTo } from "../../app/navigate";
import { toast } from "../../app/toast";
import { useDocument, useDraftComments } from "./api";
import {
  ScreenplayEditor,
  type CursorInfo,
  type EditorHandle,
  type OutlineScene,
  type SelectionAnchor,
} from "./editor/ScreenplayEditor";
import { ELEMENT_META, SHORTCUT_ORDER } from "./editor/schema";
import { nextOnEnter } from "./editor/rules";
import type { SyncStatus } from "./editor/sync";
import type { FindOptions } from "./editor/find";
import { SceneNavigator } from "./SceneNavigator";
import { RightPanels, type CommentTarget, type PanelContext } from "./panels";
import { DeleteDraftDialog, DraftsDrawer, NewDraftDialog, RenameDraftDialog, statusChip, type DraftActions } from "./drafts";
import { LockDialog, LockedEditDialog, RevisionDrawer, StartRevisionDialog, colourSwatch } from "./lock";
import { FindBar, ReorderWarningDialog, SceneHubDrawer, TitlePageDialog, type FindState } from "./misc";
import { NewReviewDialog } from "./ReviewView";
import { ExportScreenplayDialog } from "../../features/interchange";
import { togglePanel, type Layout, type Mode, type PanelId } from "./util";

type DialogState =
  | { kind: "drafts" }
  | { kind: "newDraft" }
  | { kind: "rename"; draft: ScreenplayDraftDto }
  | { kind: "delete"; draft: ScreenplayDraftDto }
  | { kind: "lock"; draft: ScreenplayDraftDto }
  | { kind: "lockedEdit" }
  | { kind: "startRevision"; source: ScreenplayDraftDto }
  | { kind: "revisions" }
  | { kind: "titlePage" }
  | { kind: "hub"; sceneId: string }
  | { kind: "reorder"; hub: ScreenplaySceneHub; sceneId: string; index: number }
  | { kind: "newReview"; draftId: string }
  | { kind: "export" };

const SAVE_LABEL: Record<SyncStatus, string> = {
  saved: "Saved",
  pending: "Saving…",
  saving: "Saving…",
  error: "Not saved yet — retrying",
};

export function ScreenplayView({ overview, layout, setLayout, onCompare, onReview, onImport }: {
  overview: ScreenplayOverview;
  layout: Layout;
  setLayout: (f: (l: Layout) => Layout) => void;
  onCompare: (a: string | null, b: string | null) => void;
  onReview: () => void;
  /** Opens the workspace's Import Screenplay dialog (a successful import opens the new draft). */
  onImport: () => void;
}) {
  const screenplay = overview.screenplay!;
  const drafts = overview.drafts;
  const permissions = overview.permissions;
  const current = drafts.find((d) => d.isCurrent) ?? drafts[0];
  const openId = layout.draftId && drafts.some((d) => d.id === layout.draftId) ? layout.draftId : current.id;
  const draft = drafts.find((d) => d.id === openId) ?? current;
  const doc = useDocument(openId);
  const comments = useDraftComments(openId);
  const editorRef = useRef<EditorHandle>(null);
  const route = useNav((s) => s.route);

  const [outline, setOutline] = useState<OutlineScene[]>([]);
  const [cursor, setCursor] = useState<CursorInfo>({ sceneId: null, elementId: null, elementType: "action" });
  const [sync, setSync] = useState<SyncStatus>("saved");
  const [selection, setSelection] = useState<{ rect: DOMRect; anchor: SelectionAnchor } | null>(null);
  const [findOpen, setFindOpen] = useState(false);
  const [findResult, setFindResult] = useState<FindState>({ index: 0, count: 0 });
  const [pendingComment, setPendingComment] = useState<CommentTarget | null>(null);
  const [focusCommentId, setFocusCommentId] = useState<string | null>(null);
  const [dialog, setDialog] = useState<DialogState | null>(null);
  const pendingJump = useRef<string | null>(null);
  const blockedAt = useRef(0);

  const flush = useCallback(async () => {
    await editorRef.current?.flush();
  }, []);

  const openDraft = useCallback(
    (d: ScreenplayDraftDto | string) => {
      const id = typeof d === "string" ? d : d.id;
      void flush().then(() => setLayout((l) => ({ ...l, draftId: id })));
    },
    [flush, setLayout],
  );

  const openPanel = useCallback(
    (p: PanelId) => setLayout((l) => (l.panels.includes(p) && l.mode !== "focus" ? l : togglePanel({ ...l, panels: l.panels.filter((x) => x !== p) }, p))),
    [setLayout],
  );

  const setMode = (m: Mode) =>
    setLayout((l) => ({
      ...l,
      mode: m,
      panels: m === "writingRoom" ? (l.panels.length ? l.panels : ["story"]) : m === "standard" ? l.panels.slice(0, 1) : l.panels,
    }));

  // ------------------------------------------------ navigation targets

  const jumpToScene = useCallback((id: string) => {
    editorRef.current?.jumpToScene(id);
  }, []);

  const jumpToComment = useCallback(
    (c: CommentDto) => {
      setFocusCommentId(c.id);
      const e = editorRef.current;
      if (!e) return;
      if (c.anchor && !c.contextMoved && e.jumpToElement(c.anchor.elementId, c.anchor.start, c.anchor.end)) return;
      const scene = c.contextMoved || c.targetDeleted ? c.nearestSceneId : c.sceneId;
      if (scene) e.jumpToScene(scene);
    },
    [],
  );

  // Search results, Scene Hub links, "Continue": { sceneId, draftId, commentId, panel }.
  const handledParams = useRef("");
  useEffect(() => {
    const p = route.params ?? {};
    const key = JSON.stringify(p);
    if (!doc.data || handledParams.current === key) return;
    if (p.draftId && p.draftId !== openId && drafts.some((d) => d.id === p.draftId)) {
      openDraft(p.draftId);
      return;
    }
    handledParams.current = key;
    if (p.commentId || p.panel === "comments") {
      openPanel("comments");
      if (p.commentId) setFocusCommentId(p.commentId);
    }
    if (p.sceneId) {
      const id = p.sceneId;
      setTimeout(() => editorRef.current?.jumpToScene(id), 0);
    }
  }, [route.params, doc.data, openId, drafts, openDraft, openPanel]);

  // Jump to a scene created from the navigator once it arrives.
  useEffect(() => {
    if (pendingJump.current && editorRef.current?.jumpToScene(pendingJump.current)) pendingJump.current = null;
  }, [doc.data]);

  // Remember where the writer is (Project Home "Continue", FSD §6.2).
  useEffect(() => {
    if (!cursor.sceneId) return;
    const t = setTimeout(() => {
      void call("project.set_last_location", { location: { workspace: "screenplay", sceneId: cursor.sceneId, draftId: openId, ...(screenplay.episodeId ? { episodeId: screenplay.episodeId } : {}) } }).catch(() => undefined);
    }, 2000);
    return () => clearTimeout(t);
  }, [cursor.sceneId, openId, screenplay.episodeId]);

  // Ctrl+F outside the page opens find too (inside the page the editor handles it).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f" && !e.defaultPrevented) {
        e.preventDefault();
        setFindOpen(true);
      }
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") void flush();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [flush]);

  // ------------------------------------------------------- actions

  const onBlockedEdit = useCallback(() => {
    const now = Date.now();
    if (now - blockedAt.current < 1500) return;
    blockedAt.current = now;
    if (draft.status === "Locked" && permissions.edit) setDialog({ kind: "lockedEdit" });
    else if (!permissions.edit) toast.info("You can read this screenplay, but your role doesn't allow editing it.");
  }, [draft.status, permissions.edit]);

  const createScene = async () => {
    try {
      await flush();
      const idx = outline.findIndex((s) => s.id === cursor.sceneId);
      const r = await call<ScreenplayEditResult>("screenplay.create_scene", {
        draftId: openId,
        index: idx >= 0 ? idx + 1 : outline.length,
        heading: "",
      });
      pendingJump.current = r.createdIds[0] ?? null;
      toast.undoable("Added a scene");
    } catch (e) {
      reportError(e);
    }
  };

  const doMove = async (sceneId: string, index: number) => {
    try {
      await call<ScreenplayEditResult>("screenplay.move_scene", { sceneId, index });
      toast.undoable(`Moved the scene to position ${index + 1}`);
    } catch (e) {
      reportError(e);
    }
  };

  const requestMove = async (sceneId: string, index: number) => {
    if (index < 0 || index >= outline.length) return;
    try {
      await flush();
      const hub = await call<ScreenplaySceneHub>("screenplay.scene_hub", { sceneId });
      if (hub.usedInProduction) setDialog({ kind: "reorder", hub, sceneId, index });
      else await doMove(sceneId, index);
    } catch (e) {
      reportError(e);
    }
  };

  const deleteScene = async (s: OutlineScene) => {
    try {
      await flush();
      await call("screenplay.delete_scene", { sceneId: s.id });
      toast.undoable(`Moved Scene ${s.number} to Recently Deleted`);
    } catch (e) {
      reportError(e);
    }
  };

  const copyScene = async (s: OutlineScene) => {
    const text = editorRef.current?.sceneText(s.id) ?? "";
    try {
      await navigator.clipboard.writeText(text);
      toast.success(`Copied Scene ${s.number}`);
    } catch {
      toast.error("The scene could not be copied to the clipboard.");
    }
  };

  const sceneMenu = (s: OutlineScene): MenuItemSpec[] => {
    const scene = doc.data?.scenes.find((x) => x.id === s.id);
    const lineage = scene?.lineageId;
    const index = outline.findIndex((x) => x.id === s.id);
    return [
      { label: "Open in Story Board", onSelect: () => navigateTo(scene?.sourceSceneCardId ? { workspace: "story", cardId: scene.sourceSceneCardId } : { workspace: "story", screenplaySceneId: s.id }) },
      { label: "Break down", onSelect: () => navigateTo({ workspace: "breakdown", sceneId: s.id, sceneLineageId: lineage ?? "" }) },
      { label: "Create Shot List", onSelect: () => navigateTo({ workspace: "production", sub: "shots", sceneId: s.id, sceneLineageId: lineage ?? "" }) },
      { label: "Create Storyboard", onSelect: () => navigateTo({ workspace: "production", sub: "storyboards", sceneId: s.id, sceneLineageId: lineage ?? "" }) },
      { label: "Open in Schedule", onSelect: () => navigateTo({ workspace: "production", sub: "schedule", sceneId: s.id, sceneLineageId: lineage ?? "" }) },
      { label: "Scene Hub", separatorBefore: true, onSelect: () => setDialog({ kind: "hub", sceneId: s.id }) },
      ...(permissions.comment
        ? [{ label: "Add Comment", onSelect: () => { setPendingComment({ kind: "scene", sceneId: s.id }); openPanel("comments"); } }]
        : []),
      { label: "Copy Scene", onSelect: () => void copyScene(s) },
      { label: "Export Scene", onSelect: () => void flush().then(() => setDialog({ kind: "export" })) },
      ...(doc.data?.canEdit
        ? [
            { label: "Move up", separatorBefore: true, disabled: index <= 0, onSelect: () => void requestMove(s.id, index - 1) },
            { label: "Move down", disabled: index >= outline.length - 1, onSelect: () => void requestMove(s.id, index + 1) },
            { label: "Delete scene", danger: true, disabled: outline.length <= 1, onSelect: () => void deleteScene(s) },
          ]
        : []),
    ];
  };

  const draftActions: DraftActions = {
    open: (d) => {
      openDraft(d);
      setDialog(null);
    },
    rename: (d) => setDialog({ kind: "rename", draft: d }),
    makeCurrent: (d) =>
      void call("screenplay.set_current_draft", { draftId: d.id })
        .then(() => toast.undoable(`“${d.name}” is now the current draft`))
        .catch(reportError),
    restore: (d) =>
      void flush()
        .then(() => call<ScreenplayDraftDto>("screenplay.restore_draft", { draftId: d.id }))
        .then((nd) => {
          toast.undoable(`Restored as a new draft: ${nd.name}`);
          openDraft(nd);
        })
        .catch(reportError),
    remove: (d) => setDialog({ kind: "delete", draft: d }),
    lock: (d) => void flush().then(() => setDialog({ kind: "lock", draft: d })),
    review: (d) => setDialog({ kind: "newReview", draftId: d.id }),
    compare: (a, b) => {
      setDialog(null);
      void flush().then(() => onCompare(a?.id ?? null, b?.id ?? null));
    },
    newDraft: () => void flush().then(() => setDialog({ kind: "newDraft" })),
  };

  const findActions = {
    search: (o: FindOptions | null) => setFindResult(editorRef.current?.setFind(o) ?? { index: 0, count: 0 }),
    step: (dir: 1 | -1) => setFindResult(editorRef.current?.findStep(dir) ?? { index: 0, count: 0 }),
    replace: (r: string) => setFindResult(editorRef.current?.replaceCurrent(r) ?? { index: 0, count: 0 }),
  };

  const ctx: PanelContext | null = doc.data
    ? {
        doc: doc.data,
        permissions,
        currentSceneId: cursor.sceneId,
        threads: comments.data ?? [],
        commentsLoading: comments.isLoading,
        pendingComment,
        setPendingComment,
        focusCommentId,
        showNotes: layout.showNotes,
        setShowNotes: (v) => setLayout((l) => ({ ...l, showNotes: v })),
        jumpToScene,
        jumpToComment,
        flush,
      }
    : null;

  const threads = useMemo(() => comments.data ?? [], [comments.data]);
  const type = cursor.elementType;
  const locked = draft.status === "Locked";
  const focus = layout.mode === "focus";

  const elementMenu: MenuItemSpec[] = [
    { label: "Element type", header: true },
    ...SHORTCUT_ORDER.map((t) => ({
      label: ELEMENT_META[t].label,
      shortcut: ELEMENT_META[t].shortcut,
      disabled: !doc.data?.canEdit,
      onSelect: () => editorRef.current?.setElementType(t),
    })),
    { label: ELEMENT_META.note.label, separatorBefore: true, disabled: !doc.data?.canEdit, onSelect: () => editorRef.current?.setElementType("note" as ElementType) },
  ];

  const viewMenu: MenuItemSpec[] = [
    { label: "Layout", header: true },
    { label: `${layout.mode === "standard" ? "✓ " : ""}Standard Mode`, onSelect: () => setMode("standard") },
    { label: `${layout.mode === "writingRoom" ? "✓ " : ""}Writing Room Mode`, onSelect: () => setMode("writingRoom") },
    { label: "Focus Mode", onSelect: () => setMode("focus") },
    { label: `${layout.showNotes ? "Hide" : "Show"} notes in the script`, separatorBefore: true, onSelect: () => setLayout((l) => ({ ...l, showNotes: !l.showNotes })) },
    { label: "Title page…", onSelect: () => setDialog({ kind: "titlePage" }), disabled: !permissions.edit },
    { label: "Scene Hub…", disabled: !cursor.sceneId, onSelect: () => cursor.sceneId && setDialog({ kind: "hub", sceneId: cursor.sceneId }) },
    { label: "Characters", onSelect: () => openPanel("characters") },
    { label: "Draft information", onSelect: () => openPanel("draft") },
    ...(permissions.edit ? [{ label: "Import screenplay…", separatorBefore: true, onSelect: () => void flush().then(onImport) }] : []),
  ];

  const draftMenu: MenuItemSpec[] = [
    { label: "Open draft", header: true },
    ...drafts
      .slice()
      .sort((a, b) => b.createdAt - a.createdAt)
      .map((d) => ({
        label: `${d.name}${d.isCurrent ? " · Current" : ""}${d.status === "Locked" ? " · Locked" : ""}`,
        disabled: d.id === openId,
        onSelect: () => openDraft(d),
      })),
    { label: "Drafts…", separatorBefore: true, onSelect: () => setDialog({ kind: "drafts" }) },
    ...(permissions.edit ? [{ label: "New Draft…", onSelect: draftActions.newDraft }] : []),
  ];

  const page = doc.error ? (
    <div className="content"><Banner tone="err">{doc.error.message}</Banner></div>
  ) : !doc.data ? (
    <div className="spx-wrap"><div className="spx-page"><Skeleton h={18} w={260} /><div style={{ height: 12 }} /><Skeleton h={220} /></div></div>
  ) : (
    <div className="spx-wrap">
      <div className="spx-scroll">
        {locked && !focus && (
          <div className="spx-lockbar" role="status">
            <Lock size={13} aria-hidden /> Locked shooting draft — read and export freely. Typing offers to start a revision.
          </div>
        )}
        <div className="spx-page" style={draft.revisionColor ? { borderTop: `4px solid ${colourSwatch(draft.revisionColor) ?? "transparent"}` } : undefined}>
          <ScreenplayEditor
            key={openId}
            ref={editorRef}
            doc={doc.data}
            canEdit={doc.data.canEdit}
            showNotes={layout.showNotes}
            comments={threads}
            onOutline={setOutline}
            onCursor={setCursor}
            onStatus={setSync}
            onBlockedEdit={onBlockedEdit}
            onFindShortcut={() => setFindOpen(true)}
            onSelection={setSelection}
            onFindResult={setFindResult}
          />
        </div>
      </div>
      {!focus && (
        <Menu
          trigger={
            <button type="button" className="elind spx-elind" aria-label={`Element type: ${ELEMENT_META[type]?.label ?? type}. Change element type`}>
              <FileText size={12} aria-hidden />
              <b>{ELEMENT_META[type]?.short ?? type}</b>
              {doc.data.canEdit ? <span style={{ opacity: 0.6 }}>· Enter → {ELEMENT_META[nextOnEnter(type)].short}</span> : <span style={{ opacity: 0.6 }}>· Read only</span>}
              <ChevronDown size={11} aria-hidden />
            </button>
          }
          items={elementMenu}
        />
      )}
      {findOpen && (
        <FindBar
          draftId={openId}
          canReplace={doc.data.canEdit}
          showNotes={layout.showNotes}
          result={findResult}
          onSearch={findActions.search}
          onStep={findActions.step}
          onReplace={findActions.replace}
          onBeforeReplaceAll={flush}
          onClose={() => {
            setFindOpen(false);
            findActions.search(null);
            editorRef.current?.focus();
          }}
        />
      )}
      {selection && permissions.comment && !focus && (
        <button
          type="button"
          className="btn sm spx-cmt-btn"
          style={{ left: Math.min(selection.rect.right + 8, window.innerWidth - 110), top: selection.rect.bottom + 6 }}
          onMouseDown={(e) => e.preventDefault()}
          onClick={() => {
            setPendingComment({ kind: "text", anchor: selection.anchor });
            openPanel("comments");
            setSelection(null);
          }}
        >
          <MessageSquare size={13} /> Comment
        </button>
      )}
    </div>
  );

  return (
    <div className={`spx mode-${layout.mode}`}>
      {focus ? (
        <div className="spx-focusbar" role="toolbar" aria-label="Focus Mode">
          <span className="b">{draft.name}</span>
          <Button size="sm" onClick={() => setMode("standard")}>Exit Focus</Button>
          <span className="xs muted" aria-live="polite">{SAVE_LABEL[sync]}</span>
        </div>
      ) : (
        <div className="sp-top" role="toolbar" aria-label="Screenplay">
          <Menu
            trigger={
              <button type="button" className="btn" style={{ padding: "5px 10px" }} aria-label={`Draft: ${draft.name}. Switch draft`}>
                <FileText size={13} aria-hidden /> <b>{draft.name}</b> <ChevronDown size={13} aria-hidden />
              </button>
            }
            items={draftMenu}
          />
          {locked ? (
            <Chip tone="g"><Lock size={11} aria-hidden /> Locked</Chip>
          ) : (
            statusChip(draft)
          )}
          {!permissions.edit && <Chip tone="out">Read only</Chip>}
          <span className="xs muted" aria-live="polite" title="Every change is saved on this computer as you type.">{SAVE_LABEL[sync]}</span>
          <span className="grow" />
          <Button size="sm" icon={<Search size={14} />} onClick={() => setFindOpen(true)} title="Find (Ctrl+F)">Find</Button>
          <Button size="sm" variant={layout.panels.includes("notes") ? "on" : "default"} icon={<NotebookPen size={14} />} onClick={() => setLayout((l) => togglePanel(l, "notes"))}>Notes</Button>
          <Button size="sm" variant={layout.panels.includes("comments") ? "on" : "default"} icon={<MessageSquare size={14} />} onClick={() => setLayout((l) => togglePanel(l, "comments"))}>
            Comments{draft.openComments > 0 ? ` (${draft.openComments})` : ""}
          </Button>
          <Button size="sm" icon={<Layers size={14} />} onClick={() => setDialog({ kind: "drafts" })}>Drafts</Button>
          <Button size="sm" icon={<GitCompare size={14} />} disabled={drafts.length < 2} onClick={() => draftActions.compare(null, null)}>Compare</Button>
          <Button size="sm" icon={<Users size={14} />} onClick={() => void flush().then(onReview)}>Review</Button>
          <Button size="sm" icon={<Download size={14} />} onClick={() => void flush().then(() => setDialog({ kind: "export" }))}>Export</Button>
          {locked || draft.status === "Revision" ? (
            <Button size="sm" icon={<Lock size={14} />} onClick={() => setDialog({ kind: "revisions" })}>Revisions</Button>
          ) : (
            permissions.lock && <Button size="sm" icon={<Lock size={14} />} onClick={() => draftActions.lock(draft)}>Lock</Button>
          )}
          <Menu trigger={<button type="button" className="iconbtn" aria-label="View and more"><Columns2 size={16} /></button>} items={viewMenu} align="end" />
        </div>
      )}
      <div className="sp-body">
        {!focus && (
          <SceneNavigator
            scenes={outline}
            currentSceneId={cursor.sceneId}
            canEdit={!!doc.data?.canEdit}
            onJump={jumpToScene}
            onNew={() => void createScene()}
            onMove={(id, index) => void requestMove(id, index)}
            menuFor={sceneMenu}
          />
        )}
        {page}
        {!focus && ctx && (
          <RightPanels
            layout={layout}
            ctx={ctx}
            onToggle={(p) => setLayout((l) => togglePanel(l, p))}
            onClose={(p) => setLayout((l) => ({ ...l, panels: l.panels.filter((x) => x !== p) }))}
          />
        )}
      </div>

      {dialog?.kind === "drafts" && (
        <DraftsDrawer drafts={drafts} openDraftId={openId} permissions={permissions} actions={draftActions} onClose={() => setDialog(null)} />
      )}
      {dialog?.kind === "newDraft" && (
        <NewDraftDialog
          drafts={drafts}
          current={draft}
          onClose={() => setDialog(null)}
          onCreated={(d) => {
            setDialog(null);
            openDraft(d);
          }}
        />
      )}
      {dialog?.kind === "rename" && <RenameDraftDialog draft={dialog.draft} onClose={() => setDialog(null)} />}
      {dialog?.kind === "delete" && <DeleteDraftDialog draft={dialog.draft} onClose={() => setDialog(null)} />}
      {dialog?.kind === "lock" && <LockDialog draft={dialog.draft} onClose={() => setDialog(null)} />}
      {dialog?.kind === "lockedEdit" && (
        <LockedEditDialog draft={draft} onClose={() => setDialog(null)} onStart={() => setDialog({ kind: "startRevision", source: draft })} />
      )}
      {dialog?.kind === "startRevision" && (
        <StartRevisionDialog
          source={dialog.source}
          drafts={drafts}
          onClose={() => setDialog(null)}
          onStarted={(d) => {
            setDialog(null);
            openDraft(d);
          }}
        />
      )}
      {dialog?.kind === "revisions" && (
        <RevisionDrawer
          drafts={drafts}
          openDraft={draft}
          permissions={permissions}
          onClose={() => setDialog(null)}
          onOpenDraft={(d) => openDraft(d)}
          onJumpScene={(id) => {
            setDialog(null);
            jumpToScene(id);
          }}
          onStartRevision={(source) => setDialog({ kind: "startRevision", source })}
        />
      )}
      {dialog?.kind === "titlePage" && <TitlePageDialog screenplay={screenplay} onClose={() => setDialog(null)} />}
      {dialog?.kind === "hub" && (
        <SceneHubDrawer
          sceneId={dialog.sceneId}
          onClose={() => setDialog(null)}
          onOpenComments={() => {
            setPendingComment(null);
            openPanel("comments");
          }}
        />
      )}
      {dialog?.kind === "reorder" && (
        <ReorderWarningDialog
          hub={dialog.hub}
          newNumber={dialog.index + 1}
          onCancel={() => setDialog(null)}
          onContinue={() => {
            const { sceneId, index } = dialog;
            setDialog(null);
            void doMove(sceneId, index);
          }}
        />
      )}
      {dialog?.kind === "newReview" && (
        <NewReviewDialog
          drafts={drafts}
          initialDraftId={dialog.draftId}
          onClose={() => setDialog(null)}
          onStarted={() => {
            setDialog(null);
            onReview();
          }}
        />
      )}
      <ExportScreenplayDialog open={dialog?.kind === "export"} onClose={() => setDialog(null)} draftId={openId} />
      {!doc.data && !doc.error && <span className="sr-only">Loading the screenplay…</span>}
    </div>
  );
}
