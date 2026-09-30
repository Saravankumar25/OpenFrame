// Breakdown dialogs (UX §3.22, mocks 106–110): Add Element with catalog
// matching, Edit / choose a match before accepting a suggestion, the
// suggestion review list, and the "scene changed" review.

import { useEffect, useMemo, useState } from "react";
import { AlertTriangle } from "lucide-react";
import { reportError, useCommand, useOp } from "../../ipc/query";
import { call } from "../../ipc/client";
import type { BreakdownCategory } from "../../ipc/generated/BreakdownCategory";
import type { BreakdownCatalogChoice } from "../../ipc/generated/BreakdownCatalogChoice";
import type { BreakdownElementDto } from "../../ipc/generated/BreakdownElementDto";
import type { BreakdownAcceptManyResult } from "../../ipc/generated/BreakdownAcceptManyResult";
import type { BreakdownSceneChanges } from "../../ipc/generated/BreakdownSceneChanges";
import type { BreakdownSuggestResult } from "../../ipc/generated/BreakdownSuggestResult";
import type { BreakdownSceneDetail } from "../../ipc/generated/BreakdownSceneDetail";
import type { CatalogMatchDto } from "../../ipc/generated/CatalogMatchDto";
import { Banner, Button, Checkbox, Chip, Dialog, Field, Select, Skeleton, TextArea, TextInput } from "../../design-system";
import { CATEGORIES, CATEGORY_SHORT, PROD_TABLES } from "../../api/production";
import { toast } from "../../app/toast";
import type { BreakdownAcceptArgs } from "../../ipc/generated/BreakdownAcceptArgs";
import type { BreakdownAddArgs } from "../../ipc/generated/BreakdownAddArgs";

const CATEGORY_OPTIONS = CATEGORIES.map((c) => ({ value: c, label: c }));

function useDebounced<T>(value: T, ms = 150): T {
  const [v, setV] = useState(value);
  useEffect(() => {
    const t = window.setTimeout(() => setV(value), ms);
    return () => window.clearTimeout(t);
  }, [value, ms]);
  return v;
}

/** Existing catalog items for a name (FSD §28.5, §143). Renaming re-runs matching. */
export function useCatalogMatches(category: BreakdownCategory, name: string, enabled = true) {
  const q = useDebounced(name.trim());
  const res = useOp<CatalogMatchDto[]>("catalog.find_matches", { category, name: q }, PROD_TABLES, { enabled: enabled && q.length > 0 });
  const matches = q.length > 0 ? res.data ?? [] : [];
  const settled = q === name.trim() && !res.isFetching;
  // Obvious identity → offered first; nothing similar → create; otherwise the user must choose.
  const defaultChoice: BreakdownCatalogChoice | null = !settled || !q
    ? null
    : matches.length === 0
      ? { mode: "new", catalogItemId: null }
      : matches.length === 1 && matches[0].exact
        ? { mode: "existing", catalogItemId: matches[0].id }
        : null;
  return { matches, loading: res.isFetching || !settled, defaultChoice };
}

function sameChoice(a: BreakdownCatalogChoice | null, b: BreakdownCatalogChoice): boolean {
  return !!a && a.mode === b.mode && (a.mode !== "existing" || a.catalogItemId === b.catalogItemId);
}

/** Radio list: "Use existing: …" rows plus "Create new …" (mock 108/109). */
export function CatalogPicker({ name, matches, loading, value, onChange }: {
  name: string;
  matches: CatalogMatchDto[];
  loading: boolean;
  value: BreakdownCatalogChoice | null;
  onChange: (c: BreakdownCatalogChoice) => void;
}) {
  if (!name.trim()) return null;
  const createNew: BreakdownCatalogChoice = { mode: "new", catalogItemId: null };
  return (
    <div role="radiogroup" aria-label="Catalog item" className="card" style={{ marginTop: 4 }}>
      {matches.map((m) => {
        const c: BreakdownCatalogChoice = { mode: "existing", catalogItemId: m.id };
        const on = sameChoice(value, c);
        return (
          <button key={m.id} type="button" role="radio" aria-checked={on} className={`li${on ? " sel" : ""}`} style={{ width: "100%", textAlign: "left", background: on ? undefined : "transparent", border: 0 }} onClick={() => onChange(c)}>
            <span className="grow">
              <b>Use existing: {m.name}</b>
              <span className="xs muted" style={{ display: "block" }}>
                {CATEGORY_SHORT[m.category]} · {m.status} · {m.usedInLabel === "Not used yet" ? "not used yet" : `used in ${m.usedInLabel.replace(/^Scenes?/, (s) => s.toLowerCase())}`}
              </span>
            </span>
            <Chip tone={m.exact ? "g" : "y"}>{m.exact ? "Same item" : "Similar"}</Chip>
          </button>
        );
      })}
      <button type="button" role="radio" aria-checked={sameChoice(value, createNew)} className={`li${sameChoice(value, createNew) ? " sel" : ""}`} style={{ width: "100%", textAlign: "left", background: sameChoice(value, createNew) ? undefined : "transparent", border: 0 }} onClick={() => onChange(createNew)}>
        <span className="grow">
          <b>Create new item “{name.trim()}”</b>
          <span className="xs muted" style={{ display: "block" }}>Added to the production catalog</span>
        </span>
      </button>
      {loading && <div className="li xs muted">Looking for existing catalog items…</div>}
      {!loading && !value && matches.length > 0 && (
        <div className="li xs" style={{ color: "#8a6510" }}>
          <AlertTriangle size={13} aria-hidden /> Several catalog items look similar. Choose one, or create a new item.
        </div>
      )}
    </div>
  );
}

export interface AddInitial {
  category?: BreakdownCategory;
  name?: string;
  span?: { elementId: string; start: number; end: number };
}

/** "Add Element" (mock 109) — also used for tagging highlighted text. */
export function AddElementDialog({ scene, initial, onClose }: { scene: BreakdownSceneDetail; initial: AddInitial | null; onClose: () => void }) {
  const open = initial !== null;
  const [category, setCategory] = useState<BreakdownCategory>(initial?.category ?? "Props");
  const [name, setName] = useState(initial?.name ?? "");
  const [notes, setNotes] = useState("");
  const [choice, setChoice] = useState<BreakdownCatalogChoice | null>(null);
  const cm = useCatalogMatches(category, name, open);
  const effective = choice ?? cm.defaultChoice;
  const add = useCommand<BreakdownAddArgs, BreakdownElementDto>("breakdown.add_element", {
    onSuccess: (el) => {
      toast.undoable(`Added “${el.name}” to Scene ${scene.number ?? ""}`.trim());
      onClose();
    },
  });
  const submit = () => {
    if (!name.trim() || !effective) return;
    add.mutate({ sceneId: scene.sceneId, category, name: name.trim(), notes: notes.trim() || null, span: initial?.span ?? null, catalog: effective });
  };
  return (
    <Dialog
      open={open}
      onOpenChange={(v) => !v && onClose()}
      size="md"
      title={initial?.span ? `Tag “${initial.name ?? ""}”` : "Add Element"}
      sub={`What does Scene ${scene.number ?? ""} need? Pick an existing catalog item or create a new one.`}
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!name.trim() || !effective || add.isPending} onClick={submit}>
            Add to Scene {scene.number}
          </Button>
        </>
      }
    >
      <form onSubmit={(e) => { e.preventDefault(); submit(); }}>
        <Field label="Category" required htmlFor="add-cat">
          <Select id="add-cat" value={category} options={CATEGORY_OPTIONS} onChange={(v) => { setCategory(v); setChoice(null); }} />
        </Field>
        <Field label="Name" required htmlFor="add-name">
          <TextInput id="add-name" autoFocus value={name} maxLength={200} onChange={(e) => { setName(e.target.value); setChoice(null); }} placeholder="e.g. Red Folder" />
        </Field>
        <CatalogPicker name={name} matches={cm.matches} loading={cm.loading} value={effective} onChange={setChoice} />
        <Field label="Notes (optional)" htmlFor="add-notes">
          <TextArea id="add-notes" rows={2} value={notes} maxLength={4000} onChange={(e) => setNotes(e.target.value)} placeholder="e.g. must look worn" />
        </Field>
        {initial?.span && <div className="xs muted">The screenplay text is not changed; the tag marks the selected words in this scene.</div>}
      </form>
    </Dialog>
  );
}

/** Edit a suggestion before accepting (category, name, catalog item) — also the
 *  chooser for ambiguous matches (mock 108). */
export function EditSuggestionDialog({ element, sceneNumber, chooser, onClose }: {
  element: BreakdownElementDto | null;
  sceneNumber: string | null;
  /** Opened because the match is ambiguous. */
  chooser?: boolean;
  onClose: () => void;
}) {
  const open = element !== null;
  const [category, setCategory] = useState<BreakdownCategory>(element?.category ?? "Props");
  const [name, setName] = useState(element?.displayName ?? "");
  const [choice, setChoice] = useState<BreakdownCatalogChoice | null>(null);
  const cm = useCatalogMatches(category, name, open);
  const effective = choice ?? cm.defaultChoice;
  const accept = useCommand<BreakdownAcceptArgs, BreakdownElementDto>("breakdown.accept", {
    onSuccess: (el) => {
      toast.undoable(`Accepted “${el.name}”`);
      onClose();
    },
  });
  if (!element) return null;
  return (
    <Dialog
      open={open}
      onOpenChange={(v) => !v && onClose()}
      size="md"
      title={chooser ? `Possible existing item: ${element.displayName}` : `Edit suggestion — ${element.displayName}`}
      sub={
        chooser
          ? `You are adding “${element.displayName}” (${element.category}) to Scene ${sceneNumber ?? ""}. The catalog already has something similar.`
          : "Change the category, name or catalog item before accepting."
      }
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button
            variant="primary"
            disabled={!name.trim() || !effective || accept.isPending}
            onClick={() => effective && accept.mutate({ id: element.id, category, name: name.trim(), catalog: effective })}
          >
            {effective?.mode === "existing" ? "Use Existing" : "Accept"}
          </Button>
        </>
      }
    >
      {element.evidence && (
        <div className="sm muted" style={{ marginBottom: 8 }}>
          matched: “{element.evidence}”
        </div>
      )}
      <Field label="Category" required htmlFor="edit-cat">
        <Select id="edit-cat" value={category} options={CATEGORY_OPTIONS} onChange={(v) => { setCategory(v); setChoice(null); }} />
      </Field>
      <Field label="Name" required htmlFor="edit-name">
        <TextInput id="edit-name" value={name} maxLength={200} onChange={(e) => { setName(e.target.value); setChoice(null); }} />
      </Field>
      <CatalogPicker name={name} matches={cm.matches} loading={cm.loading} value={effective} onChange={setChoice} />
      <div className="xs muted" style={{ marginTop: 8 }}>If several items look similar you will see a list with distinguishing details and can choose one.</div>
    </Dialog>
  );
}

function matchNote(s: BreakdownElementDto): { text: string; tone: "g" | "y" | "default" } {
  if (s.matchKind === "exact") return { text: `Existing: ${s.matches[0]?.name ?? ""}`, tone: "g" };
  if (s.matchKind === "ambiguous") return { text: "Similar items — choose", tone: "y" };
  return { text: "New catalog item", tone: "default" };
}

/** "Suggested elements — Scene 12" (mock 107). */
export function SuggestionReviewDialog({ scene, open, onClose, onEdit }: {
  scene: BreakdownSceneDetail;
  open: boolean;
  onClose: () => void;
  onEdit: (el: BreakdownElementDto, chooser: boolean) => void;
}) {
  const pending = useMemo(() => scene.groups.flatMap((g) => g.suggestions), [scene]);
  const [deselected, setDeselected] = useState<Set<string>>(new Set());
  const selected = pending.filter((s) => !deselected.has(s.id));
  const safe = pending.filter((s) => s.matchKind !== "ambiguous");
  const [busy, setBusy] = useState(false);

  const summary = (list: BreakdownElementDto[]) => {
    const by = new Map<string, number>();
    for (const s of list) by.set(s.category, (by.get(s.category) ?? 0) + 1);
    return Array.from(by.entries()).map(([c, n]) => `${n} ${c}`).join(", ");
  };
  const acceptIds = async (list: BreakdownElementDto[]) => {
    if (list.length === 0) return;
    setBusy(true);
    try {
      const r = await call<BreakdownAcceptManyResult>("breakdown.accept_many", { ids: list.map((s) => s.id) });
      const extra = r.needsChoice.length ? ` ${r.needsChoice.length} need${r.needsChoice.length === 1 ? "s" : ""} a choice of catalog item.` : "";
      toast.undoable(`Accepted ${r.accepted} suggestion${r.accepted === 1 ? "" : "s"}.${extra}`);
      if (r.needsChoice.length === 0 && list.length === pending.length) onClose();
    } catch (e) {
      reportError(e);
    } finally {
      setBusy(false);
    }
  };
  const rejectIds = async (ids: string[]) => {
    if (ids.length === 0) return;
    try {
      await call("breakdown.reject", { ids });
      toast.undoable(ids.length === 1 ? "Suggestion rejected. It won't be suggested again for this scene." : `Dismissed ${ids.length} suggestions.`);
    } catch (e) {
      reportError(e);
    }
  };
  const acceptOne = async (s: BreakdownElementDto) => {
    if (s.matchKind === "ambiguous") return onEdit(s, true);
    try {
      const el = await call<BreakdownElementDto>("breakdown.accept", { id: s.id, catalog: { mode: "auto", catalogItemId: null } });
      toast.undoable(`Accepted “${el.name}”`);
    } catch (e) {
      if ((e as { code?: string }).code === "validation.ambiguous_match") onEdit(s, true);
      else reportError(e);
    }
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(v) => !v && onClose()}
      size="lg"
      title={`Suggested elements — Scene ${scene.number ?? ""}`}
      sub="Grouped by category, with the text that triggered each suggestion."
      footerLeft={<span className="sm muted">{pending.length} suggested · {selected.length} selected</span>}
      footer={
        <>
          <Button disabled={busy || selected.length === 0} onClick={() => void rejectIds(selected.map((s) => s.id))}>
            Dismiss Selected ({selected.length})
          </Button>
          <Button disabled={busy || selected.length === 0} onClick={() => void acceptIds(selected)} title={summary(selected)}>
            Accept Selected ({selected.length})
          </Button>
          <Button variant="primary" disabled={busy || safe.length === 0} onClick={() => void acceptIds(safe)} title={summary(safe)}>
            Accept All Safe ({safe.length})
          </Button>
        </>
      }
    >
      <Banner tone="warn">Suggestions are not production data yet. Accept, edit or reject each one.</Banner>
      {pending.length === 0 ? (
        <div className="sm muted" style={{ padding: "14px 2px" }}>
          No pending suggestions for this scene. You can still tag text in the script or use Add Element.
        </div>
      ) : (
        <div style={{ marginTop: 10 }}>
          {scene.groups
            .filter((g) => g.suggestions.length > 0)
            .map((g) => (
              <div key={g.category} className="bd-cat">
                <div className="bh">
                  {g.category}
                  <span className="cn">{g.suggestions.length}</span>
                </div>
                {g.suggestions.map((s) => {
                  const note = matchNote(s);
                  return (
                    <div key={s.id} className="bd-el sug">
                      <Checkbox
                        checked={!deselected.has(s.id)}
                        onChange={(v) => {
                          const next = new Set(deselected);
                          if (v) next.delete(s.id);
                          else next.add(s.id);
                          setDeselected(next);
                        }}
                        label={<span className="sr-only">Select {s.displayName}</span>}
                      />
                      <span className="grow">
                        <b>{s.displayName}</b>
                        <span className="xs muted" style={{ display: "block" }}>
                          matched: “{s.evidence}” · {s.confidence === "Likely" ? "Likely match" : "Possible match"}
                        </span>
                      </span>
                      <Chip tone={note.tone}>{note.text}</Chip>
                      <Button size="xs" aria-label={`Accept ${s.name}`} onClick={() => void acceptOne(s)}>Accept</Button>
                      <Button size="xs" aria-label={`Edit ${s.name}`} onClick={() => onEdit(s, false)}>Edit</Button>
                      <Button size="xs" aria-label={`Reject ${s.name}`} onClick={() => void rejectIds([s.id])}>Reject</Button>
                    </div>
                  );
                })}
              </div>
            ))}
          <div className="xs muted">Existing catalog matches are suggested where obvious; ambiguous matches open a chooser.</div>
        </div>
      )}
    </Dialog>
  );
}

/** "Scene 24 changed. Review breakdown differences." (mock 110) */
export function SceneChangesDialog({ sceneId, onClose }: { sceneId: string | null; onClose: () => void }) {
  const open = sceneId !== null;
  const ch = useOp<BreakdownSceneChanges>("breakdown.scene_changes", { sceneId }, PROD_TABLES, { enabled: open });
  const keep = useCommand<{ sceneId: string }>("breakdown.mark_reviewed", {
    onSuccess: () => {
      toast.undoable("Kept the production data. The scene is marked as reviewed.");
      onClose();
    },
  });
  const apply = useCommand<{ sceneId: string }, BreakdownSuggestResult>("breakdown.apply_suggested_update", {
    onSuccess: (r) => {
      toast.undoable(r.added ? `${r.added} new suggestion${r.added === 1 ? "" : "s"} added for review. Existing elements were kept.` : "Reviewed. Existing elements were kept.");
      onClose();
    },
  });
  const d = ch.data;
  return (
    <Dialog
      open={open}
      onOpenChange={(v) => !v && onClose()}
      size="md"
      title={d ? `Scene ${d.number ?? ""} changed. Review breakdown differences.` : "Scene changed"}
      sub="The screenplay changed after this scene was broken down. Your production data has not been changed."
      footer={
        <>
          <Button onClick={onClose}>Review Manually</Button>
          <Button disabled={!sceneId || keep.isPending} onClick={() => sceneId && keep.mutate({ sceneId })}>Keep Production Data</Button>
          <Button variant="primary" disabled={!sceneId || apply.isPending} onClick={() => sceneId && apply.mutate({ sceneId })}>
            Apply Suggested Update
          </Button>
        </>
      }
    >
      {ch.isLoading && <Skeleton h={80} />}
      {ch.error && <Banner tone="err">{ch.error.message}</Banner>}
      {d && (
        <>
          {d.headingChanged && (
            <Banner tone="warn">
              Heading changed from “{d.baselineHeading}” to “{d.heading}”. The location/time description changed; existing associations stay until you review them.
            </Banner>
          )}
          <div className="card" style={{ marginTop: 10 }}>
            {d.removedCandidates.map((c) => (
              <div key={`r-${c.category}-${c.name}`} className="li sm">
                <Chip tone="r">Removed candidate</Chip>
                <span className="grow"><b>{c.name}</b> <span className="muted">({CATEGORY_SHORT[c.category]}) — “{c.evidence}”</span></span>
              </div>
            ))}
            {d.addedCandidates.map((c) => (
              <div key={`a-${c.category}-${c.name}`} className="li sm">
                <Chip tone="g">Added candidate</Chip>
                <span className="grow"><b>{c.name}</b> <span className="muted">({CATEGORY_SHORT[c.category]}) — “{c.evidence}”</span></span>
              </div>
            ))}
            {d.removedCandidates.length + d.addedCandidates.length === 0 && (
              <div className="li sm muted">No new or removed likely elements were found in the changed text.</div>
            )}
          </div>
          <div className="h4" style={{ marginTop: 12 }}>Impact</div>
          <ul className="sm" style={{ margin: "0 0 0 18px", padding: 0, lineHeight: 1.7 }}>
            <li>New elements suggested: {d.newSuggestions}</li>
            <li>
              Existing elements requiring review: {d.elementsToReview.length}
              {d.elementsToReview.length > 0 && <span className="muted"> ({d.elementsToReview.map((e) => e.name).join(", ")})</span>}
            </li>
            {d.planningNotes.map((n) => (
              <li key={n}>{n}</li>
            ))}
          </ul>
        </>
      )}
    </Dialog>
  );
}

/** Notes editor for a confirmed element. */
export function ElementNotesDialog({ element, onClose }: { element: BreakdownElementDto | null; onClose: () => void }) {
  const [notes, setNotes] = useState(element?.notes ?? "");
  const save = useCommand<{ id: string; notes: string }>("breakdown.update_element", { onSuccess: () => onClose() });
  if (!element) return null;
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      size="md"
      title={`Notes — ${element.name}`}
      sub="Scene-specific notes for this element. The catalog item and the screenplay are not changed."
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={save.isPending} onClick={() => save.mutate({ id: element.id, notes })}>Save</Button>
        </>
      }
    >
      <Field label="Notes" htmlFor="el-notes">
        <TextArea id="el-notes" rows={4} autoFocus value={notes} maxLength={4000} onChange={(e) => setNotes(e.target.value)} placeholder="e.g. must look worn" />
      </Field>
    </Dialog>
  );
}

