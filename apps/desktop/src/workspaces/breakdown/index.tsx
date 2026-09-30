// Breakdown workspace (FSD §26–27, §53–54, §96–97; UX §3.22, mocks 104–112).
// "What does this scene require to shoot?"
// Left: scenes of the Production Source · centre: script reading pane with
// tagging · right: the scene's breakdown with suggestions.

import { useCallback, useState } from "react";
import { FileText, Lock, ListChecks } from "lucide-react";
import { useOp } from "../../ipc/query";
import type { BreakdownScenesView } from "../../ipc/generated/BreakdownScenesView";
import type { BreakdownSceneDetail } from "../../ipc/generated/BreakdownSceneDetail";
import type { BreakdownElementDto } from "../../ipc/generated/BreakdownElementDto";
import { Banner, Button, EmptyState, Skeleton } from "../../design-system";
import { PROD_TABLES, useCanEdit } from "../../api/production";
import { useIntent, useNav } from "../../app/stores";
import { SelectSourceDialog, UpdateSourceDialog } from "../production/components/SourceDialogs";
import { SceneList, filterScenes, type SceneFilter } from "./SceneList";
import { ScriptPane } from "./ScriptPane";
import { ElementsPanel } from "./ElementsPanel";
import { HistoricalView, NeedsReviewView } from "./ReviewViews";
import { AddElementDialog, EditSuggestionDialog, ElementNotesDialog, SceneChangesDialog, SuggestionReviewDialog, type AddInitial } from "./dialogs";
import { suggestName, type TextSelection } from "./highlight";
import { BreakdownExportButton } from "../../features/export/buttons";
import "./breakdown.css";

type Mode = "scene" | "review" | "history";

export default function BreakdownWorkspace() {
  const route = useNav((s) => s.route);
  const go = useNav((s) => s.go);
  const fire = useIntent((s) => s.fire);
  const canEdit = useCanEdit();
  const view = useOp<BreakdownScenesView>("breakdown.scenes", {}, PROD_TABLES);

  // Links from other workspaces may name a scene of another draft (e.g. the
  // Screenplay's current draft); the same scene is found here by lineage.
  const rawScene = route.params?.sceneId ?? null;
  const rawLineage = route.params?.sceneLineageId ?? null;
  const sceneRows = view.data?.scenes;
  const paramScene =
    sceneRows && (rawScene || rawLineage)
      ? ((sceneRows.find((r) => r.sceneId === rawScene) ?? sceneRows.find((r) => !!rawLineage && r.lineageId === rawLineage))?.sceneId ?? rawScene)
      : rawScene;
  const [picked, setPicked] = useState<string | null>(paramScene);
  const [lastParam, setLastParam] = useState<string | null>(paramScene);
  const [mode, setMode] = useState<Mode>("scene");
  // Search / "Open Scenes" focus a scene through the route.
  if (paramScene !== lastParam) {
    setLastParam(paramScene);
    if (paramScene) {
      setPicked(paramScene);
      setMode("scene");
    }
  }

  const [filter, setFilter] = useState<SceneFilter>("all");
  const [selectOpen, setSelectOpen] = useState(false);
  const [updateDraft, setUpdateDraft] = useState<string | null>(null);
  const [dismissedNewer, setDismissedNewer] = useState<string | null>(null);
  const [addInitial, setAddInitial] = useState<AddInitial | null>(null);
  const [editing, setEditing] = useState<{ el: BreakdownElementDto; chooser: boolean } | null>(null);
  const [reviewOpen, setReviewOpen] = useState(false);
  const [changesScene, setChangesScene] = useState<string | null>(null);
  const [notesEl, setNotesEl] = useState<BreakdownElementDto | null>(null);
  const [selection, setSelection] = useState<TextSelection | null>(null);
  const onSelection = useCallback((s: TextSelection | null) => setSelection(s), []);

  const data = view.data;
  const rows = data?.scenes ?? [];
  const sceneId = picked ?? rows[0]?.sceneId ?? null;
  const detail = useOp<BreakdownSceneDetail>("breakdown.scene", { sceneId }, PROD_TABLES, { enabled: !!sceneId && !!data?.source });
  const d = detail.data;
  const source = data?.source ?? null;
  const index = rows.findIndex((r) => r.sceneId === sceneId);

  const dialogs = (
    <>
      <SelectSourceDialog open={selectOpen} onOpenChange={setSelectOpen} onReviewUpdate={setUpdateDraft} />
      <UpdateSourceDialog draftId={updateDraft} onClose={() => setUpdateDraft(null)} />
    </>
  );

  if (view.isLoading) {
    return (
      <div className="content">
        <Skeleton h={28} w={320} />
        <div style={{ height: 12 }} />
        <Skeleton h={200} />
      </div>
    );
  }
  if (view.error) {
    return (
      <div className="content">
        <Banner tone="err" actions={<Button size="sm" onClick={() => void view.refetch()}>Try Again</Button>}>{view.error.message}</Banner>
      </div>
    );
  }
  if (!source) {
    return (
      <div className="content">
        <EmptyState
          icon={<ListChecks size={40} />}
          title="Choose a screenplay source to begin the breakdown."
          actions={
            <>
              <Button variant="primary" disabled={!canEdit} onClick={() => setSelectOpen(true)}>Select Source</Button>
              <Button
                icon={<FileText size={15} />}
                onClick={() => {
                  fire("screenplay.import");
                  go({ workspace: "screenplay" });
                }}
              >
                Import Screenplay
              </Button>
            </>
          }
        >
          Open a screenplay to begin breaking down scenes. Choose the draft that production should be based on.
        </EmptyState>
        {dialogs}
      </div>
    );
  }

  const newer = source.newer && source.newer.id !== dismissedNewer ? source.newer : null;
  const reviewCount = rows.filter((r) => r.needsReview).length;
  const historicalCount = (data?.historical ?? []).reduce((n, h) => n + h.elements.filter((e) => !e.archived).length, 0);

  return (
    <div className="content flush">
      <div className="bdw">
        <div className="banner info srcbar" role="status">
          {source.draft.status === "Locked" ? <Lock size={13} aria-hidden /> : <FileText size={13} aria-hidden />}
          <span className="sp">
            <b>Production Source: {source.draft.name}</b>
            {source.draft.status === "Locked" ? " · Locked" : ` · ${source.draft.status}`} · {rows.length} scene{rows.length === 1 ? "" : "s"}
            {index >= 0 && ` · Scene ${rows[index].number} of ${rows.length}`}
          </span>
          {canEdit && <Button size="xs" onClick={() => setSelectOpen(true)}>Change source</Button>}
          <BreakdownExportButton size="xs" sceneId={sceneId} scenes={rows.map((r) => ({ id: r.sceneId, label: `${r.number} — ${r.heading}` }))} />
        </div>
        {newer && (
          <div className="banner warn srcbar" role="status">
            <span className="sp">
              <b>Newer revision available</b> — {newer.name}. Production stays on {source.draft.name} until you review the update.
            </span>
            {canEdit && <Button size="xs" onClick={() => setUpdateDraft(newer.id)}>Review Production Update</Button>}
            <Button size="xs" variant="ghost" onClick={() => setDismissedNewer(newer.id)}>Dismiss</Button>
          </div>
        )}
        <div className="sp-body">
          <SceneList
            rows={filterScenes(rows, filter)}
            total={rows.length}
            selected={mode === "scene" ? sceneId : null}
            filter={filter}
            onFilter={setFilter}
            onSelect={(id) => {
              setPicked(id);
              setMode("scene");
            }}
            reviewCount={reviewCount}
            historicalCount={historicalCount}
            onOpenReview={() => setMode("review")}
            onOpenHistory={() => setMode("history")}
          />
          {mode === "review" ? (
            <NeedsReviewView
              rows={rows}
              onBack={() => setMode("scene")}
              onReview={(id) => {
                setPicked(id);
                setMode("scene");
                setChangesScene(id);
              }}
            />
          ) : mode === "history" ? (
            <HistoricalView
              scenes={data?.historical ?? []}
              canEdit={canEdit}
              onBack={() => setMode("scene")}
              onOpen={(id) => {
                setPicked(id);
                setMode("scene");
              }}
            />
          ) : rows.length === 0 ? (
            <div className="queue">
              <EmptyState title="This draft has no scenes yet.">Scenes written in the Screenplay workspace appear here automatically.</EmptyState>
            </div>
          ) : !d ? (
            <div className="queue">{detail.error ? <Banner tone="err">{detail.error.message}</Banner> : <Skeleton h={240} />}</div>
          ) : (
            <>
              <ScriptPane detail={d} canEdit={canEdit} onTag={setAddInitial} onSelection={onSelection} />
              <ElementsPanel
                detail={d}
                canEdit={canEdit}
                hasSelection={!!selection && d.inSource}
                onAdd={() => setAddInitial({})}
                onTagSelection={() =>
                  selection &&
                  setAddInitial({ name: suggestName(selection.text), span: { elementId: selection.elementId, start: selection.start, end: selection.end } })
                }
                onReview={() => setReviewOpen(true)}
                onEdit={(el, chooser) => setEditing({ el, chooser })}
                onChanges={() => setChangesScene(d.sceneId)}
                onNotes={setNotesEl}
              />
            </>
          )}
        </div>
      </div>
      {d && addInitial && <AddElementDialog key={`${d.sceneId}-${addInitial.span?.start ?? "m"}-${addInitial.category ?? ""}`} scene={d} initial={addInitial} onClose={() => setAddInitial(null)} />}
      {d && editing && (
        <EditSuggestionDialog key={editing.el.id} element={editing.el} sceneNumber={d.number} chooser={editing.chooser} onClose={() => setEditing(null)} />
      )}
      {d && (
        <SuggestionReviewDialog
          key={d.sceneId}
          scene={d}
          open={reviewOpen}
          onClose={() => setReviewOpen(false)}
          onEdit={(el, chooser) => setEditing({ el, chooser })}
        />
      )}
      <SceneChangesDialog sceneId={changesScene} onClose={() => setChangesScene(null)} />
      {notesEl && <ElementNotesDialog key={notesEl.id} element={notesEl} onClose={() => setNotesEl(null)} />}
      {dialogs}
    </div>
  );
}
