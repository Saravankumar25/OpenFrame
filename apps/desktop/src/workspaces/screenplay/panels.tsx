// Right-hand panels of the Standard and Writing Room layouts (FSD §17.2,
// UX §3.12, §3.14; mocks 080–082). Panels are views: opening one never edits
// the screenplay (FSD §17.5). There is no Idea Vault panel (FSD §17.3).

import { useEffect, useMemo, useState, type ReactNode } from "react";
import { Lock, MessageSquare, MoreHorizontal, X } from "lucide-react";
import { Button, Chip, Checkbox, Menu, Segmented, Skeleton, TextArea, type MenuItemSpec } from "../../design-system";
import { call } from "../../ipc/client";
import { reportError } from "../../ipc/query";
import type { CommentThread } from "../../ipc/generated/CommentThread";
import type { CommentDto } from "../../ipc/generated/CommentDto";
import type { ScreenplayDocument } from "../../ipc/generated/ScreenplayDocument";
import type { ScreenplayPermissions } from "../../ipc/generated/ScreenplayPermissions";
import type { PrivateNoteDto } from "../../ipc/generated/PrivateNoteDto";
import { useCharacters, usePrivateNotes, useStoryReference } from "./api";
import { formatAgo, formatDate, formatWhen, initials, type Layout, type PanelId } from "./util";
import type { SelectionAnchor } from "./editor/ScreenplayEditor";

export const PANEL_LABELS: Record<PanelId, string> = {
  story: "Story Board",
  notes: "Scene Notes",
  characters: "Characters",
  comments: "Comments",
  draft: "Draft information",
};

/** What a new comment will attach to. */
export type CommentTarget =
  | { kind: "text"; anchor: SelectionAnchor }
  | { kind: "scene"; sceneId: string }
  | { kind: "draft" };

export interface PanelContext {
  doc: ScreenplayDocument;
  permissions: ScreenplayPermissions;
  currentSceneId: string | null;
  threads: CommentThread[];
  commentsLoading: boolean;
  pendingComment: CommentTarget | null;
  setPendingComment: (t: CommentTarget | null) => void;
  focusCommentId: string | null;
  showNotes: boolean;
  setShowNotes: (v: boolean) => void;
  jumpToScene: (sceneId: string) => void;
  jumpToComment: (c: CommentDto) => void;
  /** Flush pending editor edits before a content-changing action. */
  flush: () => Promise<void>;
}

export function RightPanels({ layout, ctx, onToggle, onClose }: {
  layout: Layout;
  ctx: PanelContext;
  onToggle: (p: PanelId) => void;
  onClose: (p: PanelId) => void;
}) {
  const writingRoom = layout.mode === "writingRoom";
  if (!writingRoom && layout.panels.length === 0) return null;
  return (
    <aside className="spx-tools" aria-label="Screenplay panels">
      {writingRoom && (
        <div className="spx-tabs" role="toolbar" aria-label="Writing Room panels">
          {(Object.keys(PANEL_LABELS) as PanelId[]).map((p) => (
            <button
              key={p}
              type="button"
              className={layout.panels.includes(p) ? "on" : ""}
              aria-pressed={layout.panels.includes(p)}
              onClick={() => onToggle(p)}
            >
              {PANEL_LABELS[p]}
            </button>
          ))}
        </div>
      )}
      {layout.panels.length === 0 && (
        <div className="spx-panel-empty muted sm">Choose a panel above. With no panel open, the page stays centred.</div>
      )}
      {layout.panels.map((p) => (
        <section key={p} className="spx-panel" aria-label={PANEL_LABELS[p]}>
          <PanelBody id={p} ctx={ctx} onClose={() => onClose(p)} />
        </section>
      ))}
    </aside>
  );
}

function PanelHead({ title, onClose, right }: { title: ReactNode; onClose: () => void; right?: ReactNode }) {
  return (
    <div className="spx-ph">
      <h3>{title}</h3>
      <span className="grow" />
      {right}
      <button type="button" className="iconbtn" aria-label="Close panel" title="Close panel" onClick={onClose}>
        <X size={14} />
      </button>
    </div>
  );
}

function PanelBody({ id, ctx, onClose }: { id: PanelId; ctx: PanelContext; onClose: () => void }) {
  switch (id) {
    case "story":
      return <StoryPanel ctx={ctx} onClose={onClose} />;
    case "notes":
      return <NotesPanel ctx={ctx} onClose={onClose} />;
    case "characters":
      return <CharactersPanel ctx={ctx} onClose={onClose} />;
    case "comments":
      return <CommentsPanel ctx={ctx} onClose={onClose} />;
    case "draft":
      return <DraftInfoPanel ctx={ctx} onClose={onClose} />;
  }
}

function sceneLabel(ctx: PanelContext, sceneId: string | null): string {
  if (!sceneId) return "";
  const s = ctx.doc.scenes.find((x) => x.id === sceneId);
  return s ? `Scene ${s.number}` : "";
}

// ------------------------------------------------------------ Story Board

function StoryPanel({ ctx, onClose }: { ctx: PanelContext; onClose: () => void }) {
  const ref = useStoryReference(ctx.currentSceneId);
  const [showAll, setShowAll] = useState(false);
  const linked = ref.data?.linked;
  return (
    <>
      <PanelHead title="Story Board (reference only)" onClose={onClose} />
      <div className="spx-pb">
        {!ctx.currentSceneId ? (
          <p className="muted sm">Place the cursor in a scene to see its Story Board card.</p>
        ) : ref.isLoading ? (
          <Skeleton h={60} />
        ) : linked ? (
          <div className="card pad">
            <div className="b" style={{ marginBottom: 4 }}>{linked.sceneHeading || "Scene card"}</div>
            <div className="sm selectable-text" style={{ whiteSpace: "pre-wrap" }}>{linked.shortDescription || "No description yet."}</div>
            {linked.notes && <div className="xs muted selectable-text" style={{ marginTop: 6, whiteSpace: "pre-wrap" }}>{linked.notes}</div>}
          </div>
        ) : (
          <p className="muted sm">No Story Board card is linked to {sceneLabel(ctx, ctx.currentSceneId) || "this scene"}.</p>
        )}
        {ref.data && ref.data.cards.length > 0 && (
          <div style={{ marginTop: 10 }}>
            <button type="button" className="btn ghost sm" aria-expanded={showAll} onClick={() => setShowAll(!showAll)}>
              {showAll ? "Hide" : "Show"} all Story Board cards ({ref.data.cards.length})
            </button>
            {showAll && (
              <ol className="spx-cards">
                {ref.data.cards.map((c) => (
                  <li key={c.id} className={c.id === linked?.id ? "on" : ""}>
                    <b>{c.sceneHeading || "—"}</b>
                    <span className="muted">{c.shortDescription}</span>
                  </li>
                ))}
              </ol>
            )}
          </div>
        )}
      </div>
      <div className="spx-pf xs muted">Viewing a card here never edits the screenplay or locks anything.</div>
    </>
  );
}

// ------------------------------------------------------------- Scene notes

function NotesPanel({ ctx, onClose }: { ctx: PanelContext; onClose: () => void }) {
  const scene = ctx.doc.scenes.find((s) => s.id === ctx.currentSceneId) ?? null;
  const notes = usePrivateNotes("screenplay_scene", scene?.id ?? null);
  const [sceneNote, setSceneNote] = useState(scene?.notes ?? "");
  const [editingNote, setEditingNote] = useState(false);
  const [newPrivate, setNewPrivate] = useState<string | null>(null);
  useEffect(() => {
    setSceneNote(scene?.notes ?? "");
    setEditingNote(false);
  }, [scene?.id, scene?.notes]);
  const inline = scene?.elements.filter((e) => e.elementType === "note") ?? [];

  const saveSceneNote = async () => {
    if (!scene || (scene.notes ?? "") === sceneNote) {
      setEditingNote(false);
      return;
    }
    try {
      await ctx.flush();
      await call("screenplay.update_scene", { sceneId: scene.id, notes: sceneNote });
      setEditingNote(false);
    } catch (e) {
      reportError(e);
    }
  };
  const addPrivate = async () => {
    if (!scene || !newPrivate?.trim()) return;
    try {
      await call("private_note.create", { targetType: "screenplay_scene", targetId: scene.id, body: newPrivate });
      setNewPrivate(null);
    } catch (e) {
      reportError(e);
    }
  };

  return (
    <>
      <PanelHead title={`Scene notes${scene ? ` — Scene ${scene.number}` : ""}`} onClose={onClose} />
      <div className="spx-pb">
        {!scene ? (
          <p className="muted sm">Place the cursor in a scene to see its notes.</p>
        ) : (
          <>
            {(notes.data ?? []).map((n) => (
              <PrivateNoteCard key={n.id} note={n} />
            ))}
            {newPrivate !== null && (
              <div className="cmt priv">
                <div className="ch">
                  <b>Private note</b>
                  <Chip tone="p">Only you</Chip>
                </div>
                <TextArea aria-label="Private note" value={newPrivate} autoFocus onChange={(e) => setNewPrivate(e.target.value)} rows={3} />
                <div className="row" style={{ marginTop: 6, justifyContent: "flex-end" }}>
                  <Button size="sm" onClick={() => setNewPrivate(null)}>Cancel</Button>
                  <Button size="sm" variant="primary" disabled={!newPrivate.trim()} onClick={() => void addPrivate()}>Save</Button>
                </div>
              </div>
            )}
            <div className="cmt">
              <div className="ch">
                <b>Scene note</b>
                <span className="grow" />
                {!ctx.permissions.edit || !ctx.doc.canEdit ? null : !editingNote ? (
                  <Button size="xs" variant="ghost" onClick={() => setEditingNote(true)}>{scene.notes ? "Edit" : "Add"}</Button>
                ) : null}
              </div>
              {editingNote ? (
                <>
                  <TextArea aria-label="Scene note" value={sceneNote} autoFocus rows={4} onChange={(e) => setSceneNote(e.target.value)} />
                  <div className="row" style={{ marginTop: 6, justifyContent: "flex-end" }}>
                    <Button size="sm" onClick={() => { setSceneNote(scene.notes ?? ""); setEditingNote(false); }}>Cancel</Button>
                    <Button size="sm" variant="primary" onClick={() => void saveSceneNote()}>Save</Button>
                  </div>
                </>
              ) : (
                <div className="sm selectable-text" style={{ whiteSpace: "pre-wrap" }}>{scene.notes || <span className="muted">No scene note yet.</span>}</div>
              )}
            </div>
            {inline.length > 0 && (
              <div style={{ marginTop: 8 }}>
                <div className="h4">In-script notes</div>
                {inline.map((n) => (
                  <div key={n.id} className="cmt sm" style={{ whiteSpace: "pre-wrap" }}>{n.text || <span className="muted">Empty note</span>}</div>
                ))}
              </div>
            )}
            <div className="row wrap" style={{ marginTop: 8 }}>
              {ctx.permissions.edit && ctx.doc.canEdit && (
                <Button size="sm" onClick={() => setEditingNote(true)}>Add note</Button>
              )}
              <Button size="sm" onClick={() => setNewPrivate("")}>Add private note</Button>
            </div>
            <div style={{ marginTop: 10 }}>
              <Checkbox checked={ctx.showNotes} onChange={ctx.setShowNotes} label="Show notes in the script" />
            </div>
          </>
        )}
      </div>
      <div className="spx-pf xs muted">
        Notes are not printed and are excluded from PDF/FDX/Fountain/DOCX exports and review packages by default.
      </div>
    </>
  );
}

function PrivateNoteCard({ note }: { note: PrivateNoteDto }) {
  const [editing, setEditing] = useState(false);
  const [body, setBody] = useState(note.body);
  const save = async () => {
    try {
      await call("private_note.update", { id: note.id, body });
      setEditing(false);
    } catch (e) {
      reportError(e);
    }
  };
  const items: MenuItemSpec[] = [
    { label: "Edit", onSelect: () => { setBody(note.body); setEditing(true); } },
    { label: "Delete", danger: true, onSelect: () => void call("private_note.delete", { id: note.id }).catch(reportError) },
  ];
  return (
    <div className="cmt priv">
      <div className="ch">
        <Lock size={12} aria-hidden />
        <b>Private note</b>
        <Chip tone="p">Only you</Chip>
        <span className="grow" />
        <Menu trigger={<button className="iconbtn" aria-label="Private note actions"><MoreHorizontal size={14} /></button>} items={items} align="end" />
      </div>
      {editing ? (
        <>
          <TextArea aria-label="Private note" value={body} rows={3} onChange={(e) => setBody(e.target.value)} />
          <div className="row" style={{ marginTop: 6, justifyContent: "flex-end" }}>
            <Button size="sm" onClick={() => setEditing(false)}>Cancel</Button>
            <Button size="sm" variant="primary" disabled={!body.trim()} onClick={() => void save()}>Save</Button>
          </div>
        </>
      ) : (
        <div className="selectable-text" style={{ whiteSpace: "pre-wrap" }}>{note.body}</div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------- Comments

export function targetLabel(c: CommentDto): string {
  const scene = c.sceneNumber ? `Scene ${c.sceneNumber}` : c.sceneId ? "Deleted scene" : "";
  switch (c.targetType) {
    case "screenplay_element":
      return `${scene} · text comment`;
    case "screenplay_scene":
      return `${scene} · scene comment`;
    case "screenplay_draft":
      return "Draft comment";
    default:
      return c.targetType.replace(/_/g, " ");
  }
}

function statusTone(s: string): "default" | "y" | "g" | "b" {
  return s === "Resolved" ? "g" : s === "In Discussion" ? "b" : "y";
}

export function CommentsPanel({ ctx, onClose }: { ctx: PanelContext; onClose?: () => void }) {
  const [tab, setTab] = useState<"open" | "resolved">("open");
  const open = ctx.threads.filter((t) => t.comment.status !== "Resolved");
  const resolved = ctx.threads.filter((t) => t.comment.status === "Resolved");
  const shown = (tab === "open" ? open : resolved).slice().sort((a, b) => (a.comment.sceneNumber ?? 0) - (b.comment.sceneNumber ?? 0));
  useEffect(() => {
    if (ctx.focusCommentId && resolved.some((t) => t.comment.id === ctx.focusCommentId)) setTab("resolved");
  }, [ctx.focusCommentId, resolved]);
  return (
    <>
      {onClose && <PanelHead title="Comments" onClose={onClose} />}
      <div className="spx-pb">
        <div className="row" style={{ marginBottom: 8 }}>
          <Segmented
            ariaLabel="Comment status"
            value={tab}
            onChange={setTab}
            options={[
              { value: "open", label: `Open (${open.length})` },
              { value: "resolved", label: `Resolved (${resolved.length})` },
            ]}
          />
        </div>
        {ctx.pendingComment ? (
          <CommentComposer ctx={ctx} />
        ) : (
          ctx.permissions.comment && (
            <div className="row wrap" style={{ marginBottom: 8 }}>
              <Button size="sm" icon={<MessageSquare size={13} />} disabled={!ctx.currentSceneId} onClick={() => ctx.currentSceneId && ctx.setPendingComment({ kind: "scene", sceneId: ctx.currentSceneId })}>
                Comment on {sceneLabel(ctx, ctx.currentSceneId) || "scene"}
              </Button>
              <Button size="sm" variant="ghost" onClick={() => ctx.setPendingComment({ kind: "draft" })}>Comment on draft</Button>
            </div>
          )
        )}
        {ctx.commentsLoading ? (
          <Skeleton h={50} />
        ) : shown.length === 0 ? (
          <p className="muted sm">{tab === "open" ? "No comments yet." : "No resolved comments."}</p>
        ) : (
          shown.map((t) => <ThreadCard key={t.comment.id} t={t} ctx={ctx} focused={ctx.focusCommentId === t.comment.id} />)
        )}
      </div>
    </>
  );
}

function CommentComposer({ ctx }: { ctx: PanelContext }) {
  const [body, setBody] = useState("");
  const [busy, setBusy] = useState(false);
  const t = ctx.pendingComment!;
  const label =
    t.kind === "text"
      ? t.anchor.isHeading
        ? `${sceneLabel(ctx, t.anchor.sceneId)} · scene comment`
        : `${sceneLabel(ctx, t.anchor.sceneId)} · text comment`
      : t.kind === "scene"
        ? `${sceneLabel(ctx, t.sceneId)} · scene comment`
        : "Draft comment";
  const post = async () => {
    setBusy(true);
    try {
      await ctx.flush();
      let args: Record<string, unknown>;
      if (t.kind === "text" && !t.anchor.isHeading) {
        args = { targetType: "screenplay_element", targetId: t.anchor.elementId, anchor: { start: t.anchor.start, end: t.anchor.end }, body };
      } else if (t.kind === "text") {
        args = { targetType: "screenplay_scene", targetId: t.anchor.sceneId, body };
      } else if (t.kind === "scene") {
        args = { targetType: "screenplay_scene", targetId: t.sceneId, body };
      } else {
        args = { targetType: "screenplay_draft", targetId: ctx.doc.draft.id, body };
      }
      await call("comment.create", args);
      ctx.setPendingComment(null);
    } catch (e) {
      reportError(e);
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="cmt" style={{ borderColor: "var(--accent)" }}>
      <div className="ch xs muted">{label}</div>
      {t.kind === "text" && <div className="spx-quote">“{t.anchor.text}”</div>}
      <TextArea
        aria-label="Comment"
        placeholder="Write a comment…"
        value={body}
        rows={3}
        autoFocus
        onChange={(e) => setBody(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && (e.ctrlKey || e.metaKey) && body.trim()) void post();
          if (e.key === "Escape") ctx.setPendingComment(null);
        }}
      />
      <div className="row" style={{ marginTop: 6, justifyContent: "flex-end" }}>
        <Button size="sm" onClick={() => ctx.setPendingComment(null)}>Cancel</Button>
        <Button size="sm" variant="primary" disabled={!body.trim() || busy} onClick={() => void post()}>Comment</Button>
      </div>
    </div>
  );
}

export function ThreadCard({ t, ctx, focused }: { t: CommentThread; ctx: PanelContext; focused?: boolean }) {
  const c = t.comment;
  const [reply, setReply] = useState<string | null>(null);
  const [editing, setEditing] = useState<string | null>(null);
  const run = (op: string, args: object) => void call(op, args).catch(reportError);
  const items: MenuItemSpec[] = [
    ...(c.isMine ? [{ label: "Edit", onSelect: () => setEditing(c.body) }] : []),
    ...(c.status === "Open" && ctx.permissions.comment ? [{ label: "Mark In Discussion", onSelect: () => run("comment.update", { id: c.id, status: "In Discussion" }) }] : []),
    ...(c.status === "In Discussion" && ctx.permissions.comment ? [{ label: "Mark Open", onSelect: () => run("comment.update", { id: c.id, status: "Open" }) }] : []),
    ...(c.isMine || ctx.permissions.delete ? [{ label: "Delete", danger: true, separatorBefore: true, onSelect: () => run("comment.delete", { id: c.id }) }] : []),
  ];
  const sendReply = async () => {
    if (!reply?.trim()) return;
    try {
      await call("comment.reply", { parentId: c.id, body: reply });
      setReply(null);
    } catch (e) {
      reportError(e);
    }
  };
  const saveEdit = async () => {
    if (!editing?.trim()) return;
    try {
      await call("comment.update", { id: c.id, body: editing });
      setEditing(null);
    } catch (e) {
      reportError(e);
    }
  };
  return (
    <div className={`cmt${c.status === "Resolved" ? " res" : ""}${focused ? " spx-focus" : ""}`} data-comment-id={c.id}>
      <div className="ch">
        <span className="avatar" aria-hidden style={{ width: 22, height: 22, fontSize: 10 }}>{initials(c.authorName)}</span>
        <b>{c.authorName}</b>
        <button type="button" className="spx-link xs muted" onClick={() => ctx.jumpToComment(c)} title="Show in the script">
          {targetLabel(c)}
        </button>
        <span className="grow" />
        {items.length > 0 && (
          <Menu trigger={<button className="iconbtn" aria-label="Comment actions"><MoreHorizontal size={14} /></button>} items={items} align="end" />
        )}
      </div>
      <div className="row wrap gap4" style={{ marginBottom: 4 }}>
        {c.status !== "Open" && <Chip tone={statusTone(c.status)}>{c.status}</Chip>}
        {c.contextMoved && <Chip tone="r" title="The commented text no longer exists; the comment points to the nearest scene.">Context moved</Chip>}
        {c.targetDeleted && !c.contextMoved && <Chip tone="r">Target deleted</Chip>}
      </div>
      {c.quotedText && (
        <button type="button" className="spx-quote" onClick={() => ctx.jumpToComment(c)}>“{c.quotedText}”</button>
      )}
      {editing !== null ? (
        <>
          <TextArea aria-label="Edit comment" value={editing} rows={3} onChange={(e) => setEditing(e.target.value)} />
          <div className="row" style={{ marginTop: 6, justifyContent: "flex-end" }}>
            <Button size="sm" onClick={() => setEditing(null)}>Cancel</Button>
            <Button size="sm" variant="primary" disabled={!editing.trim()} onClick={() => void saveEdit()}>Save</Button>
          </div>
        </>
      ) : (
        <div className="selectable-text" style={{ whiteSpace: "pre-wrap" }}>{c.body}</div>
      )}
      <div className="xs muted" style={{ marginTop: 2 }}>{formatAgo(c.createdAt)}{c.resolvedBy ? ` · Resolved by ${c.resolvedBy}` : ""}</div>
      {t.replies.map((r) => (
        <div key={r.id} className="reply">
          <div className="ch">
            <span className="avatar b" aria-hidden style={{ width: 20, height: 20, fontSize: 9 }}>{initials(r.authorName)}</span>
            <b className="sm">{r.authorName}</b>
            <span className="xs muted">{formatAgo(r.createdAt)}</span>
            <span className="grow" />
            {(r.isMine || ctx.permissions.delete) && (
              <button type="button" className="iconbtn" aria-label="Delete reply" title="Delete reply" onClick={() => run("comment.delete", { id: r.id })}>
                <X size={12} />
              </button>
            )}
          </div>
          <div className="selectable-text" style={{ whiteSpace: "pre-wrap" }}>{r.body}</div>
        </div>
      ))}
      {reply !== null && (
        <div style={{ marginTop: 6 }}>
          <TextArea
            aria-label="Reply"
            placeholder="Reply…"
            value={reply}
            rows={2}
            autoFocus
            onChange={(e) => setReply(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) void sendReply();
              if (e.key === "Escape") setReply(null);
            }}
          />
          <div className="row" style={{ marginTop: 6, justifyContent: "flex-end" }}>
            <Button size="sm" onClick={() => setReply(null)}>Cancel</Button>
            <Button size="sm" variant="primary" disabled={!reply.trim()} onClick={() => void sendReply()}>Reply</Button>
          </div>
        </div>
      )}
      <div className="row" style={{ marginTop: 6 }}>
        {ctx.permissions.comment && reply === null && <Button size="xs" onClick={() => setReply("")}>Reply</Button>}
        {ctx.permissions.resolveComments &&
          (c.status === "Resolved" ? (
            <Button size="xs" onClick={() => run("comment.reopen", { id: c.id })}>Reopen</Button>
          ) : (
            <Button size="xs" onClick={() => run("comment.resolve", { id: c.id })}>Resolve</Button>
          ))}
      </div>
    </div>
  );
}

// -------------------------------------------------------------- Characters

function CharactersPanel({ ctx, onClose }: { ctx: PanelContext; onClose: () => void }) {
  const rep = useCharacters(ctx.doc.draft.id);
  const spId = ctx.doc.screenplay.id;
  const link = (cueName: string, characterId: string | null, ignored: boolean) =>
    void call("screenplay.set_character_link", { screenplayId: spId, cueName, characterId: characterId ?? undefined, ignored }).catch(reportError);
  const reset = (cueName: string) => void call("screenplay.clear_character_link", { screenplayId: spId, cueName }).catch(reportError);
  const chars = useMemo(() => rep.data?.characters ?? [], [rep.data]);
  const unused = useMemo(() => {
    const used = new Set((rep.data?.cues ?? []).map((c) => c.characterId).filter(Boolean));
    return chars.filter((c) => !used.has(c.id));
  }, [rep.data, chars]);
  return (
    <>
      <PanelHead title="Characters" onClose={onClose} />
      <div className="spx-pb">
        {rep.isLoading ? (
          <Skeleton h={60} />
        ) : !rep.data || rep.data.cues.length === 0 ? (
          <p className="muted sm">No character cues in this draft yet. Characters appear here as soon as someone speaks.</p>
        ) : (
          rep.data.cues.map((c) => {
            const items: MenuItemSpec[] = [
              { label: "Link to character", header: true },
              ...chars.map((ch) => ({ label: ch.name, onSelect: () => link(c.cueName, ch.id, false) })),
              { label: "Not a character", separatorBefore: true, onSelect: () => link(c.cueName, null, true) },
              ...(c.matchKind === "manual" || c.matchKind === "ignored" ? [{ label: "Reset match", onSelect: () => reset(c.cueName) }] : []),
            ];
            return (
              <div key={c.cueName} className="cmt">
                <div className="ch">
                  <b>{c.cueName}</b>
                  <span className="xs muted">{c.speeches} {c.speeches === 1 ? "speech" : "speeches"}</span>
                  <span className="grow" />
                  {ctx.permissions.edit && (
                    <Menu trigger={<button className="iconbtn" aria-label={`Correct match for ${c.cueName}`}><MoreHorizontal size={14} /></button>} items={items} align="end" />
                  )}
                </div>
                <div className="xs" style={{ marginBottom: 4 }}>
                  {c.matchKind === "name" && <Chip tone="g">Story character: {c.characterName}</Chip>}
                  {c.matchKind === "manual" && <Chip tone="g">Linked: {c.characterName}</Chip>}
                  {c.matchKind === "first_name" && <Chip tone="y">Suggested: {c.characterName}</Chip>}
                  {c.matchKind === "none" && <Chip>Not in the character list</Chip>}
                  {c.matchKind === "ignored" && <Chip>Not a character</Chip>}
                </div>
                <div className="row wrap gap4">
                  {c.scenes.map((s) => (
                    <button key={s.sceneId} type="button" className="chip out" title={s.heading} onClick={() => ctx.jumpToScene(s.sceneId)}>
                      Sc {s.number}
                    </button>
                  ))}
                </div>
              </div>
            );
          })
        )}
        {unused.length > 0 && (
          <div style={{ marginTop: 8 }}>
            <div className="h4">Not speaking in this draft</div>
            <div className="row wrap gap4">
              {unused.map((c) => <Chip key={c.id}>{c.name}</Chip>)}
            </div>
          </div>
        )}
      </div>
    </>
  );
}

// -------------------------------------------------------- Draft information

function DraftInfoPanel({ ctx, onClose }: { ctx: PanelContext; onClose: () => void }) {
  const d = ctx.doc.draft;
  const rows: [string, ReactNode][] = [
    ["Draft", d.name],
    ["Status", d.status + (d.isCurrent ? " · Current" : "")],
    ["Lineage", d.lineage.map((l) => l.name).join(" → ")],
    ["Scenes", String(d.sceneCount)],
    ["Open comments", String(d.openComments)],
    ["Created", formatDate(d.createdAt)],
    ["Last modified", formatWhen(d.lastModified)],
  ];
  if (d.note) rows.push(["Note", d.note]);
  if (d.lockedAt) rows.push(["Locked", `${formatWhen(d.lockedAt)}${d.lockedByName ? ` by ${d.lockedByName}` : ""}`]);
  if (d.revisionLabel) rows.push(["Revision", `${d.revisionLabel}${d.revisionColor ? ` — ${d.revisionColor}` : ""}`]);
  if (d.revisionReason) rows.push(["Reason", d.revisionReason]);
  return (
    <>
      <PanelHead title="Draft information" onClose={onClose} />
      <div className="spx-pb">
        <table className="tbl">
          <tbody>
            {rows.map(([k, v]) => (
              <tr key={k}>
                <td className="muted" style={{ width: 110 }}>{k}</td>
                <td className="selectable-text">{v}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </>
  );
}
