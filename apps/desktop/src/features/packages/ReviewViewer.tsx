// Review package viewer (FSD §47.5–47.6; UX §3.39 "Reviewer view", mockup 160).
// The screenplay snapshot is read-only. A reviewer's comments are kept in a
// local review workspace — separate from any project — and go back to the
// author as a Response package. The same view shows a received package or a
// kept review record read-only.

import { useMemo, useRef, useState } from "react";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import { MessageSquarePlus, Quote, Trash2, X } from "lucide-react";
import { reportError, useOp } from "../../ipc/query";
import type { PackageReviewWorkspace } from "../../ipc/generated/PackageReviewWorkspace";
import type { PackageReviewElement } from "../../ipc/generated/PackageReviewElement";
import { Banner, Button, Chip, EmptyState, IconButton, Skeleton, TextArea } from "../../design-system";
import { dayAndTime } from "../../app/home/format";
import { toast } from "../../app/toast";
import { packageFilter, packagesApi } from "./api";
import { DoneBanner } from "./parts";

const OPS = {
  review: { op: "packages.review_get", arg: "packageId" },
  session: { op: "packages.session_view", arg: "sessionId" },
  record: { op: "packages.record_view", arg: "recordId" },
} as const;

const EL_CLASS: Record<string, string> = {
  scene_heading: "sp-h",
  action: "sp-a",
  character: "sp-c",
  dialogue: "sp-d",
  parenthetical: "sp-p",
  transition: "sp-t",
  shot: "sp-h",
  note: "sp-a",
};

function Element({ el }: { el: PackageReviewElement }) {
  return <div className={EL_CLASS[el.elementType] ?? "sp-a"}>{el.text || " "}</div>;
}

export function ReviewViewer({ source, reviewKey, onClose }: { source: "review" | "session" | "record"; reviewKey: string; onClose: () => void }) {
  const cfg = OPS[source];
  const query = useOp<PackageReviewWorkspace>(cfg.op, { [cfg.arg]: reviewKey }, ["exchange_review_record"]);
  const [override, setOverride] = useState<PackageReviewWorkspace | null>(null);
  const ws = override ?? query.data ?? null;
  const [sceneId, setSceneId] = useState<string | null>(null);
  const [body, setBody] = useState("");
  const [quote, setQuote] = useState("");
  const [busy, setBusy] = useState(false);
  const [exported, setExported] = useState<string | null>(null);
  const paperRef = useRef<HTMLDivElement>(null);

  const scene = useMemo(() => ws?.scenes.find((s) => s.id === sceneId) ?? ws?.scenes[0] ?? null, [ws, sceneId]);
  const comments = ws?.comments.filter((c) => c.sceneId === scene?.id) ?? [];
  const counts = useMemo(() => {
    const m = new Map<string, number>();
    for (const c of ws?.comments ?? []) if (c.sceneId) m.set(c.sceneId, (m.get(c.sceneId) ?? 0) + 1);
    return m;
  }, [ws]);
  const myCount = ws?.comments.filter((c) => c.mine).length ?? 0;

  const quoteSelection = () => {
    const sel = window.getSelection();
    const text = sel?.toString().trim() ?? "";
    if (text && paperRef.current && sel?.anchorNode && paperRef.current.contains(sel.anchorNode)) setQuote(text.slice(0, 500));
    else toast.info("Select some text in the scene first.");
  };

  const add = async () => {
    if (!ws || !scene || !body.trim()) return;
    setBusy(true);
    try {
      setOverride(await packagesApi.reviewComment(ws.key, scene.id, body, quote.trim() || null));
      setBody("");
      setQuote("");
    } catch (e) {
      reportError(e);
    } finally {
      setBusy(false);
    }
  };

  const remove = async (id: string) => {
    if (!ws) return;
    try {
      setOverride(await packagesApi.reviewDeleteComment(ws.key, id));
    } catch (e) {
      reportError(e);
    }
  };

  const exportResponse = async () => {
    if (!ws) return;
    const chosen = await saveDialog({
      title: "Export Response Package",
      defaultPath: `${ws.draftName} - review response.ofresponse`,
      filters: packageFilter(["response"], "OpenFrame response package"),
    });
    if (typeof chosen !== "string") return;
    try {
      const r = await packagesApi.reviewExportResponse(ws.key, chosen);
      setExported(r.path);
      toast.success(`Response package exported with ${r.commentCount} comment${r.commentCount === 1 ? "" : "s"}.`);
    } catch (e) {
      reportError(e);
    }
  };

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Review package"
      style={{ position: "fixed", inset: 0, zIndex: 45, background: "var(--bg)", display: "flex", flexDirection: "column" }}
      onKeyDown={(e) => e.key === "Escape" && onClose()}
    >
      <Banner
        tone="info"
        actions={
          <div className="row">
            {ws?.canComment && (
              <Button variant="primary" size="sm" disabled={myCount === 0} title={myCount === 0 ? "Add a comment first" : undefined} onClick={() => void exportResponse()}>
                Export Response Package
              </Button>
            )}
            <IconButton label="Close review" onClick={onClose}>
              <X size={16} />
            </IconButton>
          </div>
        }
      >
        <b>{source === "record" ? "Review record — kept separately from your working content." : "Review package — comments are separate from the original project."}</b>{" "}
        {ws && (
          <span className="sm muted">
            {ws.draftName || ws.typeLabel} · exported by {ws.exportedBy} · {dayAndTime(ws.exportedAt)}
          </span>
        )}
      </Banner>
      {exported && <DoneBanner title="Response package exported. Send it back to the author." path={exported} />}
      {!ws ? (
        query.error ? (
          <div className="content"><Banner tone="err">{query.error.message}</Banner></div>
        ) : (
          <div className="content"><Skeleton h={300} /></div>
        )
      ) : ws.scenes.length === 0 ? (
        <div className="content">
          <EmptyState title={ws.typeLabel}>
            {ws.summary.map((c) => `${c.count} ${c.label.toLowerCase()}`).join(" · ")} — from “{ws.projectTitle}”.
          </EmptyState>
        </div>
      ) : (
        <div className="row" style={{ flex: 1, minHeight: 0, alignItems: "stretch", gap: 0 }}>
          <nav aria-label="Scenes" style={{ width: 250, borderRight: "1px solid var(--line)", overflow: "auto", padding: 8 }}>
            <div className="row" style={{ marginBottom: 6 }}>
              <b className="sm">{ws.draftName}</b>
              <Chip tone="b">Review package</Chip>
            </div>
            {ws.scenes.map((s) => (
              <button
                key={s.id}
                className={`navitem${scene?.id === s.id ? " active" : ""}`}
                style={{ width: "100%", textAlign: "left" }}
                aria-current={scene?.id === s.id ? "true" : undefined}
                onClick={() => setSceneId(s.id)}
              >
                <span className="muted xs" style={{ width: 24 }}>{s.number}</span>
                <span className="grow" style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{s.heading || "(untitled scene)"}</span>
                {counts.get(s.id) ? <Chip>{counts.get(s.id)}</Chip> : null}
              </button>
            ))}
          </nav>
          <div style={{ flex: 1, overflow: "auto", padding: 20, background: "var(--bg2, #eceae4)" }}>
            {scene && (
              <div ref={paperRef} className="sp-page" style={{ margin: "0 auto", height: "auto", minHeight: 400, paddingBottom: 40 }}>
                <div className="sp-h">
                  <span className="sn">{scene.number}</span>
                  <span className="sn r">{scene.number}</span>
                  {scene.heading}
                </div>
                {scene.elements
                  .filter((e) => e.elementType !== "scene_heading")
                  .map((e) => (
                    <Element key={e.id} el={e} />
                  ))}
              </div>
            )}
          </div>
          <aside aria-label="Comments" style={{ width: 320, borderLeft: "1px solid var(--line)", overflow: "auto", padding: 12, background: "#fff" }}>
            <div className="h4">Comments on Scene {scene?.number}</div>
            {comments.length === 0 && <div className="sm muted">No comments on this scene yet.</div>}
            {comments.map((c) => (
              <div key={c.id} className="cmt card pad" style={{ marginBottom: 8 }}>
                <div className="row xs muted">
                  <b style={{ color: "var(--ink)" }}>{c.authorName}</b>
                  <span>{dayAndTime(c.createdAt)}</span>
                  <span className="grow" />
                  {c.mine && <Chip tone="g">Yours</Chip>}
                  {c.mine && ws.canComment && (
                    <IconButton label="Delete this comment" onClick={() => void remove(c.id)}>
                      <Trash2 size={13} />
                    </IconButton>
                  )}
                </div>
                {c.quotedText && <div className="xs muted" style={{ fontStyle: "italic" }}>“{c.quotedText}”</div>}
                <div className="sm" style={{ whiteSpace: "pre-wrap" }}>{c.body}</div>
              </div>
            ))}
            {ws.canComment && scene && (
              <div className="card pad" style={{ marginTop: 8 }}>
                <div className="row" style={{ marginBottom: 6 }}>
                  <b className="sm">Add a comment</b>
                  <span className="grow" />
                  <Button size="xs" icon={<Quote size={12} />} onClick={quoteSelection}>Quote selection</Button>
                </div>
                {quote && (
                  <div className="xs muted row" style={{ marginBottom: 4 }}>
                    <span className="grow" style={{ fontStyle: "italic" }}>“{quote}”</span>
                    <IconButton label="Remove quote" onClick={() => setQuote("")}><X size={12} /></IconButton>
                  </div>
                )}
                <TextArea
                  value={body}
                  onChange={(e) => setBody(e.target.value)}
                  rows={4}
                  aria-label="Comment"
                  placeholder="Your note for the author…"
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) void add();
                  }}
                />
                <div className="row" style={{ marginTop: 6 }}>
                  <span className="xs muted">Ctrl+Enter to add</span>
                  <span className="grow" />
                  <Button size="sm" variant="primary" icon={<MessageSquarePlus size={13} />} disabled={busy || !body.trim()} onClick={() => void add()}>
                    Comment
                  </Button>
                </div>
              </div>
            )}
            {!ws.canComment && <div className="xs muted" style={{ marginTop: 8 }}>Read-only. Nothing here changes your project.</div>}
          </aside>
        </div>
      )}
    </div>
  );
}
