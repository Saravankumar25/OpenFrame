// Scene breakdown panel (mock 104): categories with content, confirmed rows,
// visually distinct suggestion rows with Accept / Edit / Reject, and the
// Add Element / Suggest Elements actions.

import { useState } from "react";
import { Check, ChevronDown, ChevronRight, Eye, MoreHorizontal, Plus, Sparkles, Tag } from "lucide-react";
import { call } from "../../ipc/client";
import { reportError, useCommand } from "../../ipc/query";
import type { BreakdownSceneDetail } from "../../ipc/generated/BreakdownSceneDetail";
import type { BreakdownElementDto } from "../../ipc/generated/BreakdownElementDto";
import type { BreakdownSuggestResult } from "../../ipc/generated/BreakdownSuggestResult";
import { Banner, Button, Chip, IconButton, Menu } from "../../design-system";
import { catalogStatusTone } from "../../api/production";
import { useNav } from "../../app/stores";
import { toast } from "../../app/toast";

function ElementStatus({ e }: { e: BreakdownElementDto }) {
  if (e.catalogRemoved) return <Chip tone="r" title="The catalog item was deleted. Restore it from Recently Deleted or link another item.">Catalog item removed</Chip>;
  if (e.catalogArchived) return <Chip title="Archived in the Catalog — kept readable here.">Archived item</Chip>;
  if (e.catalogStatus) return <Chip tone={catalogStatusTone(e.catalogStatus)}>{e.catalogStatus}</Chip>;
  return null;
}

export function ElementsPanel({ detail, canEdit, hasSelection, onAdd, onTagSelection, onReview, onEdit, onChanges, onNotes }: {
  detail: BreakdownSceneDetail;
  canEdit: boolean;
  hasSelection: boolean;
  onAdd: () => void;
  onTagSelection: () => void;
  onReview: () => void;
  onEdit: (el: BreakdownElementDto, chooser: boolean) => void;
  onChanges: () => void;
  onNotes: (el: BreakdownElementDto) => void;
}) {
  const go = useNav((s) => s.go);
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  const n = detail.number ?? "";
  const complete = useCommand<{ sceneId: string; complete: boolean }>("breakdown.set_complete", {
    onSuccess: (_, a) => toast.undoable(a.complete ? `Scene ${n} marked complete` : `Scene ${n} reopened`),
  });
  const suggest = useCommand<{ sceneId: string }, BreakdownSuggestResult>("breakdown.suggest", {
    onSuccess: (r) =>
      r.added > 0
        ? toast.info(`${r.added} suggestion${r.added === 1 ? "" : "s"} to review. Nothing is added to production until you accept.`)
        : toast.info(r.pending > 0 ? "No new suggestions. Review the pending ones." : "No likely elements found. You can still tag text or add elements manually."),
  });

  const accept = async (s: BreakdownElementDto) => {
    if (s.matchKind === "ambiguous") return onEdit(s, true);
    try {
      const el = await call<BreakdownElementDto>("breakdown.accept", { id: s.id, catalog: { mode: "auto", catalogItemId: null } });
      toast.undoable(`Accepted “${el.name}”`);
    } catch (e) {
      if ((e as { code?: string }).code === "validation.ambiguous_match") onEdit(s, true);
      else reportError(e);
    }
  };
  const reject = (s: BreakdownElementDto) =>
    void call("breakdown.reject", { ids: [s.id] })
      .then(() => toast.undoable(`Rejected “${s.displayName}”. It won't be suggested again for this scene.`))
      .catch(reportError);
  const remove = (e: BreakdownElementDto) =>
    void call("breakdown.remove_element", { id: e.id })
      .then(() => toast.undoable(`Removed “${e.name}” from Scene ${n}. The catalog item is kept.`))
      .catch(reportError);

  const toggle = (c: string) => {
    const next = new Set(collapsed);
    if (next.has(c)) next.delete(c);
    else next.add(c);
    setCollapsed(next);
  };

  return (
    <aside className="sp-tools" aria-label={`Scene ${n} breakdown`}>
      <div className="row" style={{ marginBottom: 6 }}>
        <b style={{ fontSize: 13 }}>{detail.number ? `Scene ${n} breakdown` : "Removed scene"}</b>
        <span className="grow" />
        {detail.inSource && canEdit && (
          <Button size="xs" variant={detail.complete ? "on" : "default"} icon={<Check size={13} />} disabled={complete.isPending} onClick={() => complete.mutate({ sceneId: detail.sceneId, complete: !detail.complete })} title={detail.complete ? "Reopen this scene's breakdown" : "Mark this scene's breakdown complete"}>
            {detail.complete ? "Complete" : "Mark Complete"}
          </Button>
        )}
      </div>
      <div className="row wrap" style={{ marginBottom: 8 }}>
        {detail.facts.intExt && <Chip tone="out">{detail.facts.intExt}</Chip>}
        {detail.facts.timeOfDay && <Chip tone="out">{detail.facts.timeOfDay}</Chip>}
        {detail.facts.setName && <span className="xs muted">{detail.facts.setName}</span>}
      </div>
      {!detail.inSource && <div style={{ marginBottom: 8 }}><Banner tone="info">Scene removed from current source. Its breakdown is kept for reference.</Banner></div>}
      {detail.changeMessage && (
        <div style={{ marginBottom: 8 }}>
          <Banner tone="warn" actions={<Button size="xs" onClick={onChanges}>Review</Button>}>{detail.changeMessage}</Banner>
        </div>
      )}
      {detail.suggestionCount > 0 && (
        <div style={{ marginBottom: 8 }}>
          <Banner tone="warn" actions={<Button size="xs" icon={<Eye size={13} />} onClick={onReview}>Review…</Button>}>
            <Sparkles size={13} aria-hidden /> <b>Suggested elements — {detail.suggestionCount}</b>
          </Banner>
        </div>
      )}

      {detail.groups.length === 0 && (
        <div className="sm muted" style={{ padding: "10px 2px" }}>
          Nothing broken down yet. Select words in the script to tag them, add an element, or ask for suggestions.
        </div>
      )}
      {detail.groups.map((g) => {
        const isCollapsed = collapsed.has(g.category);
        return (
          <section key={g.category} className="bd-cat" aria-label={g.category}>
            <button type="button" className="bh" aria-expanded={!isCollapsed} onClick={() => toggle(g.category)}>
              {isCollapsed ? <ChevronRight size={13} aria-hidden /> : <ChevronDown size={13} aria-hidden />}
              {g.category}
              <span className="cn">{g.confirmed.filter((e) => !e.archived).length}</span>
            </button>
            {!isCollapsed && (
              <>
                {g.confirmed.map((e) => (
                  <div key={e.id} className={`bd-el${e.archived ? " hist" : ""}`}>
                    <span className="dot" aria-hidden />
                    <span className="nm">
                      <b>{e.name}</b>
                      {(e.notes || (e.name !== e.displayName && e.displayName)) && <small>{e.notes ?? `tagged as “${e.displayName}”`}</small>}
                    </span>
                    <ElementStatus e={e} />
                    <Menu
                      align="end"
                      trigger={
                        <IconButton label={`Actions for ${e.name}`}>
                          <MoreHorizontal size={15} />
                        </IconButton>
                      }
                      items={[
                        {
                          label: "Open in Catalog",
                          disabled: !e.catalogItemId,
                          onSelect: () => e.catalogItemId && go({ workspace: "production", sub: "catalog", params: { catalogItemId: e.catalogItemId } }),
                        },
                        { label: "Edit notes…", disabled: !canEdit, onSelect: () => onNotes(e) },
                        { label: "Remove from scene", danger: true, separatorBefore: true, disabled: !canEdit, onSelect: () => remove(e) },
                      ]}
                    />
                  </div>
                ))}
                {g.suggestions.map((s) => (
                  <div key={s.id} className="bd-el sug">
                    <Chip tone="y">Suggested</Chip>
                    <span className="nm">
                      <b>{s.displayName} — suggested</b>
                      <small title={s.evidence ?? undefined}>matched: “{s.evidence}”</small>
                    </span>
                    {canEdit && (
                      <>
                        <Button size="xs" aria-label={`Accept ${s.name}`} onClick={() => void accept(s)}>Accept</Button>
                        <Button size="xs" aria-label={`Edit ${s.name}`} onClick={() => onEdit(s, false)}>Edit</Button>
                        <Button size="xs" aria-label={`Reject ${s.name}`} onClick={() => reject(s)}>Reject</Button>
                      </>
                    )}
                  </div>
                ))}
              </>
            )}
          </section>
        );
      })}

      {detail.inSource && canEdit && (
        <div className="row wrap" style={{ marginTop: 6 }}>
          <Button size="sm" icon={<Plus size={14} />} onClick={onAdd}>Add Element</Button>
          <Button size="sm" icon={<Sparkles size={14} />} disabled={suggest.isPending} onClick={() => suggest.mutate({ sceneId: detail.sceneId })}>
            {suggest.isPending ? "Suggesting…" : "Suggest Elements"}
          </Button>
          {hasSelection && <Button size="sm" icon={<Tag size={14} />} onClick={onTagSelection}>Tag Selection</Button>}
        </div>
      )}
    </aside>
  );
}
