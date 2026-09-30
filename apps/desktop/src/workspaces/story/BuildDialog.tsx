// Build Screenplay from the Story Board (FSD §18, §91; UX §3.6 mocks 063–065).
// Preview first; nothing is created until the user confirms. An existing
// screenplay is never overwritten: the build adds a new draft or a new document.

import { useMemo, useState, type ReactNode } from "react";
import { Banner, Button, Checkbox, Chip, Dialog, Select, Skeleton, TextInput } from "../../design-system";
import { reportError } from "../../ipc/query";
import { toast } from "../../app/toast";
import { useNav } from "../../app/stores";
import { story, useBuildPreview, useOrderPreview } from "../../api/story";
import type { StoryBuildPreview } from "../../ipc/generated/StoryBuildPreview";
import type { StoryDescriptionMode } from "../../ipc/generated/StoryDescriptionMode";
import { headingIsValid } from "./model";
import { useStoryUi } from "./ui";

const close = () => useStoryUi.getState().openDialog(null);

function Choice({ name, checked, onChange, title, children, disabled }: { name: string; checked: boolean; onChange: () => void; title: string; children?: ReactNode; disabled?: boolean }) {
  return (
    <label className="story-radio" aria-disabled={disabled || undefined} style={disabled ? { opacity: 0.6 } : undefined}>
      <input type="radio" name={name} checked={checked} onChange={onChange} disabled={disabled} />
      <span>
        <b>{title}</b>
        {children && <span className="muted" style={{ display: "block", fontSize: 12.5 }}>{children}</span>}
      </span>
    </label>
  );
}

export function BuildDialog({ episodeId, preselect }: { episodeId: string | null; preselect?: string[] }) {
  const preview = useBuildPreview(episodeId, true);
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && close()}
      size="lg"
      title="Build Screenplay from Story Board"
      sub="Preview the order before anything is created."
    >
      {preview.data ? (
        <BuildForm data={preview.data} episodeId={episodeId} preselect={preselect} />
      ) : preview.isError ? (
        <Banner tone="err">The preview could not be prepared. {preview.error?.message}</Banner>
      ) : (
        <Skeleton h={120} />
      )}
    </Dialog>
  );
}

function BuildForm({ data, episodeId, preselect }: { data: StoryBuildPreview; episodeId: string | null; preselect?: string[] }) {
  const go = useNav((s) => s.go);
  const [included, setIncluded] = useState<Set<string>>(
    () => new Set(preselect && preselect.length ? preselect : data.rows.map((r) => r.cardId)),
  );
  const [headings, setHeadings] = useState<Record<string, string>>({});
  const [mode, setMode] = useState<StoryDescriptionMode>("planningNote");
  const existing = data.screenplays;
  const [destination, setDestination] = useState<"newDraft" | "newScreenplay">(existing.length ? "newDraft" : "newScreenplay");
  const [screenplayId, setScreenplayId] = useState<string>(existing[0]?.id ?? "");
  const [busy, setBusy] = useState(false);
  const target = existing.find((s) => s.id === screenplayId);

  const headingFor = (id: string, current: string | null) => headings[id] ?? current ?? "";
  const rows = data.rows;
  const chosen = rows.filter((r) => included.has(r.cardId));
  const missing = chosen.filter((r) => !headingIsValid(headingFor(r.cardId, r.sceneHeading))).length;
  const canBuild = chosen.length > 0 && missing === 0 && !busy && (destination === "newScreenplay" || !!target);
  const summary = useMemo(
    () =>
      `${chosen.length} of ${rows.length} card${rows.length === 1 ? "" : "s"} will be built${missing ? ` · ${missing} need${missing === 1 ? "s" : ""} a heading` : ""}`,
    [chosen.length, rows.length, missing],
  );

  const build = async () => {
    setBusy(true);
    try {
      const r = await story.buildScreenplay({
        episodeId,
        include: chosen.map((row) => ({ cardId: row.cardId, heading: headings[row.cardId]?.trim() ? headings[row.cardId] : null })),
        descriptionMode: mode,
        destination,
        screenplayId: destination === "newDraft" ? screenplayId : null,
        title: null,
        draftName: null,
      });
      close();
      toast.undoable(`Built “${r.draftName}” with ${r.sceneCount} scene${r.sceneCount === 1 ? "" : "s"}`);
      // In a series the Screenplay workspace needs the episode to show its screenplay.
      go({ workspace: "screenplay", params: { ...(episodeId ? { episodeId } : {}), screenplayId: r.screenplayId, draftId: r.draftId, sceneId: r.firstSceneId } });
    } catch (e) {
      reportError(e);
    } finally {
      setBusy(false);
    }
  };

  if (rows.length === 0) {
    return (
      <>
        <Banner tone="info">
          There are no Scene Cards in the active Story Board yet{data.parkedCount ? ` (Parking Lot cards are excluded)` : ""}. Add Scene Cards to
          Acts or Sequences first.
        </Banner>
        <div className="df" style={{ padding: "12px 0 0", border: 0, background: "transparent" }}>
          <Button onClick={close}>Close</Button>
        </div>
      </>
    );
  }

  return (
    <>
      <Banner tone="info">
        Cards are used in Story Board order. Parking Lot cards are excluded. Nothing is created until you confirm.
      </Banner>
      <div style={{ maxHeight: 300, overflow: "auto", margin: "10px 0", border: "1px solid var(--line)", borderRadius: 8 }}>
        <table className="tbl">
          <thead>
            <tr>
              <th style={{ width: 36 }}>#</th>
              <th>Story Board scene</th>
              <th style={{ width: 300 }}>Heading</th>
              <th style={{ width: 70 }}>Include</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((r) => {
              const inc = included.has(r.cardId);
              const h = headingFor(r.cardId, r.sceneHeading);
              const ok = headingIsValid(h);
              return (
                <tr key={r.cardId} style={{ opacity: inc ? 1 : 0.5 }}>
                  <td>{r.order}</td>
                  <td>
                    <div className="truncate2">{r.shortDescription || <i className="muted">Blank scene card</i>}</div>
                    <div className="muted" style={{ fontSize: 11.5 }}>
                      {r.location}
                      {r.alreadyBuilt && " · used in a screenplay before"}
                    </div>
                  </td>
                  <td>
                    {r.headingValid && headings[r.cardId] === undefined ? (
                      <span style={{ fontFamily: "Courier New, monospace", fontSize: 12.5 }}>{r.sceneHeading}</span>
                    ) : (
                      <div className="row" style={{ gap: 6 }}>
                        <TextInput
                          aria-label={`Heading for scene ${r.order}`}
                          placeholder="Add a heading…"
                          value={h}
                          invalid={inc && !ok}
                          onChange={(e) => setHeadings((s) => ({ ...s, [r.cardId]: e.target.value }))}
                          style={{ fontFamily: "Courier New, monospace", fontSize: 12.5 }}
                        />
                        {!ok && <Chip tone="y">Heading needed</Chip>}
                      </div>
                    )}
                  </td>
                  <td>
                    <Checkbox
                      checked={inc}
                      label={<span className="sr-only">Include scene {r.order}</span>}
                      onChange={(v) =>
                        setIncluded((s) => {
                          const n = new Set(s);
                          if (v) n.add(r.cardId);
                          else n.delete(r.cardId);
                          return n;
                        })
                      }
                    />
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
      <div className="grid g2" style={{ gap: 16 }}>
        <div>
          <div className="h4">How to use the card description</div>
          <Choice name="desc" checked={mode === "planningNote"} onChange={() => setMode("planningNote")} title="Use as a scene planning note (default)">
            The description is kept as a note outside the printed script.
          </Choice>
          <Choice name="desc" checked={mode === "actionText"} onChange={() => setMode("actionText")} title="Insert as temporary action text">
            Editable draft text you are expected to rewrite.
          </Choice>
        </div>
        <div>
          <div className="h4">Destination</div>
          {existing.length > 0 && (
            <p className="muted" style={{ fontSize: 12.5, marginTop: 0 }}>
              {existing.length === 1 ? `“${existing[0].title}” already has a screenplay` : "This project already has screenplays"}
              {existing.some((s) => s.hasWrittenScenes) ? " with written scenes" : ""}. Building again will not overwrite it.
            </p>
          )}
          <Choice
            name="dest"
            checked={destination === "newDraft"}
            disabled={existing.length === 0}
            onChange={() => existing.length && setDestination("newDraft")}
            title="New Draft in the existing screenplay"
          >
            {target
              ? `“${target.nextDraftName}” is added; ${target.currentDraftName ? `“${target.currentDraftName}”` : "the current draft"} is untouched.`
              : "Available once a screenplay exists."}
          </Choice>
          {destination === "newDraft" && existing.length > 1 && (
            <div style={{ margin: "0 0 8px 26px" }}>
              <Select
                value={screenplayId}
                onChange={setScreenplayId}
                ariaLabel="Screenplay"
                options={existing.map((s) => ({ value: s.id, label: s.title }))}
              />
            </div>
          )}
          <Choice
            name="dest"
            checked={destination === "newScreenplay"}
            disabled={existing.length > 0}
            onChange={() => setDestination("newScreenplay")}
            title="New Screenplay"
          >
            {existing.length
              ? "Not available: a project has one screenplay. Build a new draft of it instead."
              : `A new screenplay “${data.defaultTitle}” with its first draft.`}
          </Choice>
        </div>
      </div>
      <div className="df" style={{ padding: "12px 0 0", margin: 0, border: 0, background: "transparent" }}>
        <div className="l muted" style={{ fontSize: 12.5 }}>{summary}</div>
        <Button onClick={close}>Cancel</Button>
        <Button variant="primary" disabled={!canBuild} onClick={() => void build()}>
          Build Screenplay
        </Button>
      </div>
    </>
  );
}

/** Explicitly apply the board order to a written screenplay (FSD-STORY-025, mock 065). */
export function ApplyOrderDialog({ episodeId }: { episodeId: string | null }) {
  const preview = useOrderPreview(episodeId, true);
  const [busy, setBusy] = useState(false);
  const p = preview.data;
  const apply = async () => {
    if (!p) return;
    setBusy(true);
    try {
      await story.applyOrder(episodeId, p.draftId, true);
      close();
      toast.undoable(`Applied the Story Board order to “${p.draftName ?? "the screenplay"}”`);
    } catch (e) {
      reportError(e);
    } finally {
      setBusy(false);
    }
  };
  const moves = p?.moves ?? [];
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && close()}
      size="md"
      title="Apply this order to the written screenplay?"
      sub={
        p?.hasWrittenScenes
          ? "Reordering these cards will change the screenplay scene order. Some of these scenes have already been written."
          : "Reordering these cards will change the screenplay scene order."
      }
      footer={
        <>
          <Button onClick={close}>Cancel</Button>
          <Button variant="primary" disabled={busy || !p?.draftId || moves.length === 0 || p.locked} onClick={() => void apply()}>
            Apply
          </Button>
        </>
      }
    >
      {!p ? (
        <Skeleton h={80} />
      ) : !p.draftId ? (
        <Banner tone="info">There is no screenplay yet. Use Build Screenplay to create one from the Story Board.</Banner>
      ) : p.locked ? (
        <Banner tone="warn">“{p.draftName}” is locked as the shooting draft. Start a revision in the Screenplay workspace to change scene order.</Banner>
      ) : moves.length === 0 ? (
        <Banner tone="ok">“{p.draftName}” already follows the Story Board order{p.linkedCount === 0 ? " (no cards are linked to its scenes)" : ""}.</Banner>
      ) : (
        <>
          <p style={{ marginTop: 0 }}>
            In “{p.draftName}”{p.screenplayTitle ? ` of ${p.screenplayTitle}` : ""}:
          </p>
          <ul style={{ maxHeight: 220, overflow: "auto", paddingLeft: 18, margin: 0 }}>
            {moves.map((m) => (
              <li key={m.sceneId} style={{ marginBottom: 4 }}>
                Screenplay Scene {m.fromNumber} would become Scene {m.toNumber} — <span className="muted">{m.heading}</span>
              </li>
            ))}
          </ul>
          <p className="muted" style={{ fontSize: 12.5 }}>Production planning, if any, is not reordered automatically.</p>
        </>
      )}
    </Dialog>
  );
}
