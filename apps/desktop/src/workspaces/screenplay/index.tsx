// Screenplay workspace root (FSD §15.2 entry paths; UX §3.11 empty state, mock 075).

import { useCallback, useEffect, useState } from "react";
import { Clapperboard, FileText } from "lucide-react";
import { Banner, Button, EmptyState, Skeleton } from "../../design-system";
import { call } from "../../ipc/client";
import { reportError } from "../../ipc/query";
import type { CreatedScreenplay } from "../../ipc/generated/CreatedScreenplay";
import type { CommentDto } from "../../ipc/generated/CommentDto";
import type { InterchangeImportReport } from "../../ipc/generated/InterchangeImportReport";
import type { ScreenplayLocation } from "../../ipc/generated/ScreenplayLocation";
import { useIntent, useNav } from "../../app/stores";
import { toast } from "../../app/toast";
import { useDocument, useOverview } from "./api";
import { ScreenplayView } from "./ScreenplayView";
import { CompareView } from "./CompareView";
import { ReviewView } from "./ReviewView";
import { ImportScreenplayDialog } from "../../features/interchange";
import { useLayout } from "./util";
import "./screenplay.css";

export default function ScreenplayWorkspace() {
  const route = useNav((s) => s.route);
  const go = useNav((s) => s.go);
  const fire = useIntent((s) => s.fire);
  const consume = useIntent((s) => s.consume);
  const intent = useIntent((s) => s.intent);
  const [episodeId, setEpisodeId] = useState<string | null>(route.params?.episodeId ?? null);
  const overview = useOverview(episodeId);
  const [layout, setLayout, layoutReady] = useLayout();
  const [importOpen, setImportOpen] = useState(false);
  const [creating, setCreating] = useState(false);
  const [compareIds, setCompareIds] = useState<{ a: string | null; b: string | null }>({ a: null, b: null });

  useEffect(() => {
    if (route.params?.episodeId && route.params.episodeId !== episodeId) setEpisodeId(route.params.episodeId);
  }, [route.params, episodeId]);

  // Quick action "Import Screenplay…" (shell `+` menu, Breakdown empty state).
  // Watch the intent itself so it also works while this workspace is already open.
  useEffect(() => {
    if (intent === "screenplay.import" && consume("screenplay.import")) setImportOpen(true);
  }, [intent, consume]);

  // Links from other workspaces may name a scene without its draft (schedule
  // strips, characters, breakdown) or a draft without its episode (series):
  // resolve them once so the right draft/episode opens at that scene.
  const episodic = overview.data?.episodic;
  useEffect(() => {
    const p = route.params ?? {};
    const needsDraft = !!p.sceneId && !p.draftId;
    const needsEpisode = !!p.draftId && !p.episodeId && episodic === true;
    if (!needsDraft && !needsEpisode) return;
    let alive = true;
    call<ScreenplayLocation>("screenplay.locate", needsDraft ? { sceneId: p.sceneId } : { draftId: p.draftId })
      .then((loc) => {
        if (alive) go({ workspace: "screenplay", sub: route.sub, params: { ...p, draftId: loc.draftId, ...(loc.episodeId ? { episodeId: loc.episodeId } : {}) } });
      })
      // A scene that no longer exists: the workspace simply shows the current draft.
      .catch(() => undefined);
    return () => {
      alive = false;
    };
  }, [route.params, route.sub, episodic, go]);

  // After a successful import, open the new draft (in its episode for series).
  const openImported = useCallback(
    async (r: InterchangeImportReport) => {
      let episode: string | null = null;
      try {
        episode = (await call<ScreenplayLocation>("screenplay.locate", { draftId: r.draftId })).episodeId;
      } catch {
        // The draft is there; fall back to the current scope.
      }
      if (episode) setEpisodeId(episode);
      setLayout((l) => ({ ...l, draftId: r.draftId }));
      go({ workspace: "screenplay", params: { ...(episode ? { episodeId: episode } : {}), draftId: r.draftId } });
    },
    [go, setLayout],
  );

  const o = overview.data;
  const drafts = o?.drafts ?? [];
  const current = drafts.find((d) => d.isCurrent) ?? drafts[0];
  const openId = layout.draftId && drafts.some((d) => d.id === layout.draftId) ? layout.draftId : current?.id ?? null;
  // Review needs the open document for thread actions (it is cached with the editor's query).
  const reviewDoc = useDocument(route.sub === "review" ? openId : null);

  // Rendered at a stable position (see the return below) so the dialog keeps
  // its report step when the page behind it changes after an import.
  const importDialog = (
    <ImportScreenplayDialog open={importOpen} onClose={() => setImportOpen(false)} onImported={(r) => void openImported(r)} />
  );

  const renderBody = () => {
    if (overview.isLoading || !layoutReady) {
      return (
        <div className="content">
          <Skeleton h={28} w={260} />
          <div style={{ height: 14 }} />
          <Skeleton h={320} />
        </div>
      );
    }
    if (overview.error || !o) {
      return (
        <div className="content">
          <Banner tone="err">{overview.error?.message ?? "The screenplay could not be loaded."}</Banner>
        </div>
      );
    }

    // Episodic projects: one screenplay per episode (FSD §25.3).
    if (o.episodic && !o.episodeId) {
      return (
        <div className="content">
          <div className="ph">
            <div>
              <h1>Screenplay</h1>
              <div className="sub">Each episode has its own screenplay and drafts. Choose an episode.</div>
            </div>
          </div>
          {o.episodes.length === 0 ? (
            <EmptyState icon={<Clapperboard size={28} />} title="No episodes yet">
              Screenplays in a series live inside episodes. Add the first episode to this series, then come back to write its screenplay.
            </EmptyState>
          ) : (
            <div className="card" style={{ maxWidth: 560 }}>
              {o.episodes.map((e, i) => (
                <button key={e.id} type="button" className="li spx-li-btn" onClick={() => setEpisodeId(e.id)}>
                  <span className="b" style={{ width: 28 }}>{i + 1}</span>
                  <span className="grow" style={{ textAlign: "left" }}>{e.title}</span>
                  {e.seasonTitle && <span className="xs muted">{e.seasonTitle}</span>}
                </button>
              ))}
            </div>
          )}
        </div>
      );
    }

    if (!o.screenplay) {
      const create = async () => {
        setCreating(true);
        try {
          const r = await call<CreatedScreenplay>("screenplay.create", o.episodeId ? { episodeId: o.episodeId } : {});
          toast.undoable(`Created ${r.screenplay.title}`);
        } catch (e) {
          reportError(e);
        } finally {
          setCreating(false);
        }
      };
      return (
        <div className="content">
          {o.episodic && (
            <Button size="sm" variant="ghost" onClick={() => setEpisodeId(null)}>← All episodes</Button>
          )}
          <EmptyState
            icon={<FileText size={28} />}
            title="Write your screenplay or import one you already have."
            actions={
              o.permissions.edit ? (
                <>
                  <Button variant="primary" disabled={creating} onClick={() => void create()}>New Screenplay</Button>
                  <Button
                    onClick={() => {
                      fire("story.build_screenplay");
                      go({ workspace: "story" });
                    }}
                  >
                    Build from Story Board
                  </Button>
                  <Button onClick={() => setImportOpen(true)}>Import Screenplay</Button>
                </>
              ) : undefined
            }
          >
            Write from scratch, or build your first screenplay scenes from the Story Board. Start writing or use Build Screenplay.
            {!o.permissions.edit && <div className="sm" style={{ marginTop: 8 }}>Your role can view the screenplay once someone creates it.</div>}
          </EmptyState>
        </div>
      );
    }

    const backToScript = () => go({ workspace: "screenplay", params: o.episodeId ? { episodeId: o.episodeId } : undefined });
    const jumpFromReview = (c: CommentDto) =>
      go({
        workspace: "screenplay",
        params: {
          ...(o.episodeId ? { episodeId: o.episodeId } : {}),
          ...(c.sceneId ? { sceneId: (c.contextMoved || c.targetDeleted ? c.nearestSceneId : c.sceneId) ?? c.sceneId } : {}),
          commentId: c.id,
        },
      });

    if (route.sub === "compare") {
      return <CompareView drafts={drafts} initialA={compareIds.a} initialB={compareIds.b} onBack={backToScript} />;
    }
    if (route.sub === "review") {
      return (
        <ReviewView
          screenplayId={o.screenplay.id}
          drafts={drafts}
          doc={reviewDoc.data ?? null}
          permissions={o.permissions}
          initialRoundId={route.params?.reviewRoundId ?? null}
          onBack={backToScript}
          onJumpToComment={jumpFromReview}
        />
      );
    }

    return (
        <ScreenplayView
          overview={o}
          layout={layout}
          setLayout={setLayout}
          onCompare={(a, b) => {
            setCompareIds({ a, b });
            go({ workspace: "screenplay", sub: "compare", params: o.episodeId ? { episodeId: o.episodeId } : undefined });
          }}
          onReview={() => go({ workspace: "screenplay", sub: "review", params: o.episodeId ? { episodeId: o.episodeId } : undefined })}
          onImport={() => setImportOpen(true)}
        />
    );
  };

  return (
    <>
      {renderBody()}
      {importDialog}
    </>
  );
}
