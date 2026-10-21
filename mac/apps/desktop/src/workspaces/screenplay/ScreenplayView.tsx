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
  PanelLeftClose,
  PanelLeftOpen,
  Search,
  ZoomIn,
  ZoomOut,
} from "lucide-react";
import { Banner, Button, Chip, Menu, Skeleton, type MenuItemSpec } from "../../design-system";
import { call, onAppEvent } from "../../ipc/client";
import { reportError } from "../../ipc/query";
import type { ScreenplayOverview } from "../../ipc/generated/ScreenplayOverview";
import type { ScreenplayDraftDto } from "../../ipc/generated/ScreenplayDraftDto";
import type { ScreenplayEditResult } from "../../ipc/generated/ScreenplayEditResult";
import type { ScreenplaySceneHub } from "../../ipc/generated/ScreenplaySceneHub";
import type { CommentDto } from "../../ipc/generated/CommentDto";
import type { ElementType } from "../../ipc/generated/ElementType";
import { useNav, useOpenDraft, useUi } from "../../app/stores";
import { navigateTo } from "../../app/navigate";
import { toast } from "../../app/toast";
import { useDocument, useDraftComments, useNavigatorColumns, usePagination, useProofing, useSuggestionSources } from "./api";
import { CustomElementsDialog, PageSetupDialog, PageViewDrawer, SceneNumberDialog } from "./pagesetup";
import {
  ScreenplayEditor,
  type CursorInfo,
  type EditorHandle,
  type OutlineScene,
  type ProofingMode,
  type SelectionAnchor,
} from "./editor/ScreenplayEditor";
import { ELEMENT_META, SHORTCUT_ORDER } from "./editor/schema";
import { matches, shortcutLabel, useShortcuts } from "../../app/shortcuts";
import { nextOnEnter } from "./editor/rules";
import type { SyncStatus } from "./editor/sync";
import type { FindOptions } from "./editor/find";
import { SceneNavigator, type NavColumnsData } from "./SceneNavigator";
import { RightPanels, type CommentTarget, type PanelContext } from "./panels";
import { DraftsDrawer, NewDraftDialog, statusChip, type DraftActions } from "./drafts";
import { LockDialog, LockedEditDialog, RevisionDrawer, StartRevisionDialog, colourSwatch } from "./lock";
import { FindBar, ReorderWarningDialog, SceneHubDrawer, TitlePageDialog, type FindState } from "./misc";
import { ExportScreenplayDialog } from "../../features/interchange";
import { toggleColumn, togglePanel, type Layout, type Mode, type PanelId } from "./util";
import { DeleteScreenplayDialog, NewScreenplayDialog, RenameScreenplayDialog, ScreenplayPicker } from "./picker";

type DialogState =
  | { kind: "drafts" }
  | { kind: "newDraft" }
  | { kind: "lock"; draft: ScreenplayDraftDto }
  | { kind: "lockedEdit" }
  | { kind: "startRevision"; source: ScreenplayDraftDto }
  | { kind: "revisions" }
  | { kind: "titlePage" }
  | { kind: "hub"; sceneId: string }
  | { kind: "reorder"; hub: ScreenplaySceneHub; sceneId: string; index: number }
  | { kind: "export" }
  | { kind: "pageSetup" }
  | { kind: "customElements" }
  | { kind: "sceneNumber"; sceneId: string; label: string }
  | { kind: "pageView" }
  | { kind: "newScreenplay" }
  | { kind: "renameScreenplay" }
  | { kind: "deleteScreenplay" };

const SAVE_LABEL: Record<SyncStatus, string> = {
  saved: "Saved",
  pending: "Saving…",
  saving: "Saving…",
  error: "Not saved yet. Retrying",
};

const ZOOM_KEY = "openframe.screenplay.zoom";
const ZOOM_MIN = 0.5;
const ZOOM_MAX = 2;

/** The page zoom is a per-user view preference, never project data. */
function readZoom(): number {
  try {
    const z = Number(window.localStorage.getItem(ZOOM_KEY));
    return Number.isFinite(z) && z >= ZOOM_MIN && z <= ZOOM_MAX ? z : 1;
  } catch {
    return 1;
  }
}

function writeZoom(z: number): number {
  const v = Math.round(Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, z)) * 100) / 100;
  try {
    window.localStorage.setItem(ZOOM_KEY, String(v));
  } catch {
    // View preference only.
  }
  return v;
}

export function ScreenplayView({ overview, layout, setLayout, onCompare, onImport, onSwitchScreenplay }: {
  overview: ScreenplayOverview;
  layout: Layout;
  setLayout: (f: (l: Layout) => Layout) => void;
  onCompare: (a: string | null, b: string | null) => void;
  /** Opens the workspace's Import Screenplay dialog (a successful import opens the new draft). */
  onImport: () => void;
  /** Opens another screenplay of the project (at the given draft, else its current draft). */
  onSwitchScreenplay: (screenplayId: string, draftId?: string | null) => void;
}) {
  const screenplay = overview.screenplay!;
  const drafts = overview.drafts;
  const permissions = overview.permissions;
  const current = drafts.find((d) => d.isCurrent) ?? drafts[0];
  const openId = layout.draftId && drafts.some((d) => d.id === layout.draftId) ? layout.draftId : current.id;
  const draft = drafts.find((d) => d.id === openId) ?? current;
  const doc = useDocument(openId);
  // The assistant's "Current Screenplay" scope uses the draft open here.
  const setOpenDraft = useOpenDraft((s) => s.setDraftId);
  useEffect(() => {
    setOpenDraft(openId);
    return () => setOpenDraft(null);
  }, [openId, setOpenDraft]);
  const comments = useDraftComments(openId);
  const pagination = usePagination(openId);
  const editorRef = useRef<EditorHandle>(null);
  const route = useNav((s) => s.route);
  const sources = useSuggestionSources();
  const proofingState = useProofing();
  const navColumns = useNavigatorColumns(openId, layout.navColumns.length > 0);
  // Re-render menus and tooltips when the person changes a shortcut.
  useShortcuts((s) => s.overrides);

  const [outline, setOutline] = useState<OutlineScene[]>([]);
  const [cursor, setCursor] = useState<CursorInfo>({ sceneId: null, elementId: null, elementType: "action" });
  const [sync, setSync] = useState<SyncStatus>("saved");
  const [selection, setSelection] = useState<{ rect: DOMRect; anchor: SelectionAnchor } | null>(null);
  const [findOpen, setFindOpen] = useState(false);
  const [findResult, setFindResult] = useState<FindState>({ index: 0, count: 0 });
  const [pendingComment, setPendingComment] = useState<CommentTarget | null>(null);
  const [focusCommentId, setFocusCommentId] = useState<string | null>(null);
  const [dialog, setDialog] = useState<DialogState | null>(null);
  // The editor reports the cursor on every keystroke and the outline after every
  // typing pause; only real changes may re-render this view (and its panels).
  const onCursor = useCallback((c: CursorInfo) => {
    setCursor((prev) => (prev.sceneId === c.sceneId && prev.elementId === c.elementId && prev.elementType === c.elementType ? prev : c));
  }, []);
  const onOutline = useCallback((next: OutlineScene[]) => {
    setOutline((prev) => (sameOutline(prev, next) ? prev : next));
  }, []);
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

  // Find outside the page opens find too (inside the page the editor handles it).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!e.defaultPrevented && matches(e, "screenplay.find")) {
        e.preventDefault();
        setFindOpen(true);
      }
      if (matches(e, "app.save")) void flush();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [flush]);

  // The personal dictionary changed (here or in Settings): check the page again.
  useEffect(() => {
    const un = onAppEvent((ev) => {
      if (ev.type === "module" && ev.module === "writing" && ev.name === "dictionaryChanged") editorRef.current?.recheckSpelling();
    });
    return () => {
      void un.then((f) => f());
    };
  }, []);

  const proofing = useMemo((): ProofingMode => {
    const p = proofingState.data;
    if (!p || !p.settings.enabled) return { mode: "off", lang: p?.settings.mainLanguage ?? "en-US" };
    return { mode: p.adapterAvailable ? "openframe" : "native", lang: p.settings.mainLanguage };
  }, [proofingState.data]);

  const setSpelling = (enabled: boolean) =>
    void call("writing.set_proofing", { enabled })
      .then(() => toast.undoable(enabled ? "Turned spell checking on" : "Turned spell checking off"))
      .catch(reportError);

  const createValue = useCallback((kind: "character" | "location", name: string, movedOn: boolean) => {
    const op = kind === "character" ? call("story.create_character", { name }) : call("locations.create", { name });
    const text = kind === "character" ? `Added ${name} to Characters` : `Added ${name} to Locations`;
    // After Enter moved on to Dialogue, the latest undo step is soon the Dialogue line, so the toast
    // offers no Undo button that would take back something else; Ctrl+Z undoes one step at a time.
    return op
      .then(() => {
        if (movedOn) toast.info(text);
        else toast.undoable(text);
      })
      .catch(reportError);
  }, []);

  const columnData = useMemo((): NavColumnsData | null => {
    const d = navColumns.data;
    if (!d) return null;
    return { rows: new Map(d.rows.map((r) => [r.sceneId, r])), hasRevisions: d.hasRevisions, hasProductionNumbers: d.hasProductionNumbers };
  }, [navColumns.data]);

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
      toast.undoable(doc.data?.numberingFrozen && draft.status === "Revision" ? `Scene ${s.label} is now OMITTED` : `Moved Scene ${s.label} to Recently Deleted`);
    } catch (e) {
      reportError(e);
    }
  };

  const copyScene = async (s: OutlineScene) => {
    const text = editorRef.current?.sceneText(s.id) ?? "";
    try {
      await navigator.clipboard.writeText(text);
      toast.success(`Copied Scene ${s.label}`);
    } catch {
      toast.error("The scene could not be copied to the clipboard.");
    }
  };

  const sceneMenu = (s: OutlineScene): MenuItemSpec[] => {
    const scene = doc.data?.scenes.find((x) => x.id === s.id);
    const lineage = scene?.lineageId;
    const index = outline.findIndex((x) => x.id === s.id);
    return [
      { label: "Open in Beat Board", onSelect: () => navigateTo(scene?.sourceSceneCardId ? { workspace: "story", cardId: scene.sourceSceneCardId } : { workspace: "story", screenplaySceneId: s.id }) },
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
            { label: "Scene number…", separatorBefore: true, onSelect: () => void flush().then(() => setDialog({ kind: "sceneNumber", sceneId: s.id, label: s.label })) },
            { label: "Move up", disabled: index <= 0, onSelect: () => void requestMove(s.id, index - 1) },
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
    lock: (d) => void flush().then(() => setDialog({ kind: "lock", draft: d })),
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
  const currentScene = useMemo(() => {
    const s = outline.find((x) => x.id === cursor.sceneId);
    return s ? { id: s.id, number: s.number } : null;
  }, [outline, cursor.sceneId]);
  const type = cursor.elementType;
  const locked = draft.status === "Locked";
  const focus = layout.mode === "focus";
  // Focus mode hides the whole application frame, not only the screenplay panels (owner item 13).
  const setFocusMode = useUi((s) => s.setFocusMode);
  useEffect(() => {
    setFocusMode(focus);
    return () => setFocusMode(false);
  }, [focus, setFocusMode]);
  useEffect(() => {
    if (!focus) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !document.querySelector(".scrim")) setMode("standard");
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });
  // The writer's page zoom (owner items 10 and 14): Ctrl+wheel and touchpad pinch (which arrives as
  // Ctrl+wheel) zoom the page only, not the whole window.
  const [zoom, setZoomState] = useState(readZoom);
  const setZoom = useCallback((z: number) => setZoomState(writeZoom(z)), []);
  const scrollRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      if (!e.ctrlKey) return;
      e.preventDefault();
      setZoomState((z) => writeZoom(z * Math.exp(-e.deltaY / 400)));
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  });

  const elementMenu: MenuItemSpec[] = [
    { label: "Element type", header: true },
    ...SHORTCUT_ORDER.map((t) => ({
      label: ELEMENT_META[t].label,
      shortcut: shortcutLabel(`screenplay.element_${t}`) || undefined,
      disabled: !doc.data?.canEdit,
      onSelect: () => editorRef.current?.setElementType(t),
    })),
    { label: ELEMENT_META.note.label, shortcut: shortcutLabel("screenplay.element_note") || undefined, separatorBefore: true, disabled: !doc.data?.canEdit, onSelect: () => editorRef.current?.setElementType("note" as ElementType) },
    ...(doc.data?.customElements.length ? [{ label: "Custom elements", header: true } as MenuItemSpec] : []),
    ...(doc.data?.customElements ?? []).map((c) => ({
      label: c.name,
      shortcut: c.shortcut ?? undefined,
      disabled: !doc.data?.canEdit,
      onSelect: () => editorRef.current?.setCustomElement(c.id),
    })),
    { label: "Insert page break", shortcut: shortcutLabel("screenplay.page_break") || undefined, separatorBefore: true, disabled: !doc.data?.canEdit, onSelect: () => editorRef.current?.insertPageBreak() },
  ];

  const viewMenu: MenuItemSpec[] = [
    { label: "Layout", header: true },
    { label: `${layout.mode === "standard" ? "✓ " : ""}Standard Mode`, onSelect: () => setMode("standard") },
    { label: `${layout.mode === "writingRoom" ? "✓ " : ""}Writing Room Mode`, onSelect: () => setMode("writingRoom") },
    { label: "Focus Mode", onSelect: () => setMode("focus") },
    { label: `${layout.showNotes ? "Hide" : "Show"} notes in the script`, separatorBefore: true, onSelect: () => setLayout((l) => ({ ...l, showNotes: !l.showNotes })) },
    { label: "Page view", onSelect: () => void flush().then(() => setDialog({ kind: "pageView" })) },
    {
      label: `${proofing.mode === "off" ? "" : "\u2713 "}Check spelling`,
      disabled: !proofingState.data?.canEdit,
      onSelect: () => setSpelling(proofing.mode === "off"),
    },
    { label: "Spelling and languages…", onSelect: () => navigateTo({ workspace: "settings", sub: "writing" }) },
    { label: "Title page…", onSelect: () => setDialog({ kind: "titlePage" }), disabled: !permissions.edit },
    { label: "Advanced", header: true },
    { label: "Page setup…", onSelect: () => setDialog({ kind: "pageSetup" }) },
    { label: "Custom elements…", onSelect: () => setDialog({ kind: "customElements" }) },
    ...(doc.data?.canEdit && doc.data.omittedScenes.length > 0
      ? doc.data.omittedScenes.map((o) => ({
          label: `Restore omitted scene ${o.label}`,
          onSelect: () =>
            void call("screenplay.restore_omitted_scene", { sceneId: o.id })
              .then(() => toast.undoable(`Restored scene ${o.label}`))
              .catch(reportError),
        }))
      : []),
    ...(doc.data?.canEdit && doc.data.numberingFrozen && draft.status !== "Revision"
      ? [{ label: "Renumber scenes from their order", onSelect: () => void flush().then(() => call("screenplay.renumber_scenes", { draftId: openId })).then(() => toast.undoable("Renumbered the scenes")).catch(reportError) }]
      : []),
    { label: "Scenes", header: true },
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
      <div className="spx-scroll" ref={scrollRef} style={{ ["--sp-zoom" as string]: zoom }}>
        {locked && !focus && (
          <div className="spx-lockbar" role="status">
            <Lock size={13} aria-hidden /> Locked shooting draft. You can read and export it freely. Typing offers to start a revision.
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
            onOutline={onOutline}
            onCursor={onCursor}
            onStatus={setSync}
            onBlockedEdit={onBlockedEdit}
            onFindShortcut={() => setFindOpen(true)}
            onSelection={setSelection}
            onFindResult={setFindResult}
            suggestionSources={sources.data ?? null}
            onCreateValue={createValue}
            proofing={proofing}
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
          currentScene={currentScene}
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
          <ScreenplayPicker
            screenplays={overview.screenplays}
            openId={screenplay.id}
            canEdit={permissions.edit}
            onSwitch={(id) => void flush().then(() => onSwitchScreenplay(id))}
            onNew={() => setDialog({ kind: "newScreenplay" })}
            onRename={() => setDialog({ kind: "renameScreenplay" })}
            canDelete={permissions.delete}
            onDelete={() => void flush().then(() => setDialog({ kind: "deleteScreenplay" }))}
          />
          <Menu
            trigger={
              <button type="button" className="btn" style={{ padding: "5px 10px" }} aria-label={`Draft: ${draft.name}. Switch draft`}>
                <FileText size={13} aria-hidden /> <b className="sp-name" title={draft.name}>{draft.name}</b> <ChevronDown size={13} aria-hidden />
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
          {pagination.data && (
            <span className="xs muted" title={`${pagination.data.paper}, ${pagination.data.linesPerPage} lines per page`}>
              {pagination.data.pageCount} page{pagination.data.pageCount === 1 ? "" : "s"}
            </span>
          )}
          <span className="grow" />
          <Button
            size="sm"
            variant={layout.navHidden ? "default" : "on"}
            icon={layout.navHidden ? <PanelLeftOpen size={14} /> : <PanelLeftClose size={14} />}
            aria-pressed={!layout.navHidden}
            title={layout.navHidden ? "Show the scene list" : "Hide the scene list"}
            onClick={() => setLayout((l) => ({ ...l, navHidden: !l.navHidden }))}
          >
            Scenes
          </Button>
          <div className="sp-zoom" role="group" aria-label="Page zoom">
            <button type="button" className="iconbtn" aria-label="Zoom out" title="Zoom out (Ctrl+wheel)" disabled={zoom <= ZOOM_MIN} onClick={() => setZoom(zoom - 0.1)}><ZoomOut size={14} /></button>
            <button type="button" className="sp-zoom-val" title="Reset the page zoom" onClick={() => setZoom(1)}>{Math.round(zoom * 100)}%</button>
            <button type="button" className="iconbtn" aria-label="Zoom in" title="Zoom in (Ctrl+wheel)" disabled={zoom >= ZOOM_MAX} onClick={() => setZoom(zoom + 0.1)}><ZoomIn size={14} /></button>
          </div>
          <Button size="sm" icon={<Search size={14} />} onClick={() => setFindOpen(true)} title={shortcutLabel("screenplay.find") ? `Find (${shortcutLabel("screenplay.find")})` : "Find"}>Find</Button>
          <Button size="sm" variant={layout.panels.includes("notes") ? "on" : "default"} icon={<NotebookPen size={14} />} onClick={() => setLayout((l) => togglePanel(l, "notes"))}>Notes</Button>
          <Button size="sm" variant={layout.panels.includes("comments") ? "on" : "default"} icon={<MessageSquare size={14} />} onClick={() => setLayout((l) => togglePanel(l, "comments"))}>
            Comments{draft.openComments > 0 ? ` (${draft.openComments})` : ""}
          </Button>
          <Button size="sm" icon={<Layers size={14} />} onClick={() => setDialog({ kind: "drafts" })}>Drafts</Button>
          <Button size="sm" icon={<GitCompare size={14} />} disabled={drafts.length < 2} onClick={() => draftActions.compare(null, null)}>Compare</Button>
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
        {!focus && !layout.navHidden && (
          <SceneNavigator
            scenes={outline}
            currentSceneId={cursor.sceneId}
            canEdit={!!doc.data?.canEdit}
            onJump={jumpToScene}
            onNew={() => void createScene()}
            onMove={(id, index) => void requestMove(id, index)}
            menuFor={sceneMenu}
            columns={layout.navColumns}
            columnData={columnData}
            onToggleColumn={(c) => setLayout((l) => toggleColumn(l, c))}
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

      {dialog?.kind === "newScreenplay" && (
        <NewScreenplayDialog
          episodeId={overview.episodeId}
          screenplays={overview.screenplays}
          onClose={() => setDialog(null)}
          onCreated={(r) => {
            setDialog(null);
            void flush().then(() => onSwitchScreenplay(r.screenplay.id, r.draft.id));
          }}
        />
      )}
      {dialog?.kind === "renameScreenplay" && (
        <RenameScreenplayDialog screenplay={screenplay} screenplays={overview.screenplays} onClose={() => setDialog(null)} />
      )}
      {dialog?.kind === "deleteScreenplay" && (
        <DeleteScreenplayDialog
          screenplay={screenplay}
          screenplays={overview.screenplays}
          onClose={() => setDialog(null)}
          onDeleted={(next) => {
            setDialog(null);
            // With no screenplay left the overview comes back empty and the workspace shows its
            // empty state, as in a new project.
            if (next) onSwitchScreenplay(next);
          }}
        />
      )}
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
      {dialog?.kind === "titlePage" && <TitlePageDialog screenplay={screenplay} draftId={openId} onClose={() => setDialog(null)} />}
      {dialog?.kind === "pageSetup" && <PageSetupDialog screenplayId={screenplay.id} canEdit={permissions.edit} onClose={() => setDialog(null)} />}
      {dialog?.kind === "customElements" && (
        <CustomElementsDialog screenplayId={screenplay.id} elements={doc.data?.customElements ?? []} canEdit={permissions.edit} onClose={() => setDialog(null)} />
      )}
      {dialog?.kind === "sceneNumber" && <SceneNumberDialog sceneId={dialog.sceneId} label={dialog.label} onClose={() => setDialog(null)} />}
      {dialog?.kind === "pageView" && <PageViewDrawer draftId={openId} onClose={() => setDialog(null)} />}
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
      <ExportScreenplayDialog
        open={dialog?.kind === "export"}
        onClose={() => setDialog(null)}
        draftId={openId}
        defaults={{
          titlePage: screenplay.pageSetup.includeTitlePage,
          // Scene numbers start on in the export dialog, as before page setup existed.
          sceneNumbers: true,
          revisionMarks: screenplay.pageSetup.revisionMarks,
        }}
      />
      {!doc.data && !doc.error && <span className="sr-only">Loading the screenplay…</span>}
    </div>
  );
}

function sameOutline(a: OutlineScene[], b: OutlineScene[]): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    if (a[i].id !== b[i].id || a[i].number !== b[i].number || a[i].label !== b[i].label || a[i].heading !== b[i].heading) return false;
  }
  return true;
}
