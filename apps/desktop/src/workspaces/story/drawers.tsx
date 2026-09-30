// Expanded detail drawers (FSD §11.4, §89.2; UX §3.8, mocks 053, 060, 071).
// Simple edits autosave; closing returns to exactly the same board place.

import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { FileText, Paperclip, X } from "lucide-react";
import { Button, Drawer, Field, TextArea, TextInput, cx } from "../../design-system";
import { openAsset } from "../../ipc/client";
import { reportError } from "../../ipc/query";
import { useNav } from "../../app/stores";
import { story, useCardDetail } from "../../api/story";
import type { StoryAttachmentDto } from "../../ipc/generated/StoryAttachmentDto";
import type { StoryBoardState } from "../../ipc/generated/StoryBoardState";
import { convertBeat, duplicateItems, parkItems, requestDelete, restoreToStory } from "./actions";
import { COLOR_NAMES, COLOR_VARS } from "./items";
import { containerLabel, locate, refKey, sequences } from "./model";
import { useStoryUi } from "./ui";
import { requestDeleteSequence } from "./dialogs";

type SaveState = "idle" | "saving" | "saved" | "error";

/** Text field that saves after a pause in typing and on blur (coalesced into one undo step by Rust). */
function AutoField({
  label,
  value,
  onSave,
  multiline,
  rows = 3,
  placeholder,
  hint,
  autoFocus,
  disabled,
  onState,
}: {
  label: string;
  value: string;
  onSave: (v: string) => Promise<void>;
  multiline?: boolean;
  rows?: number;
  placeholder?: string;
  hint?: string;
  autoFocus?: boolean;
  disabled?: boolean;
  onState: (s: SaveState) => void;
}) {
  const [v, setV] = useState(value);
  const focused = useRef(false);
  const last = useRef(value);
  const timer = useRef<number | undefined>(undefined);
  useEffect(() => {
    if (!focused.current) {
      setV(value);
      last.current = value;
    }
  }, [value]);
  useEffect(() => () => window.clearTimeout(timer.current), []);
  const flush = (next: string) => {
    window.clearTimeout(timer.current);
    if (next === last.current) return;
    last.current = next;
    onState("saving");
    onSave(next)
      .then(() => onState("saved"))
      .catch((e) => {
        onState("error");
        reportError(e);
      });
  };
  const change = (next: string) => {
    setV(next);
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => flush(next), 700);
  };
  const common = {
    value: v,
    placeholder,
    disabled,
    autoFocus,
    onFocus: () => (focused.current = true),
    onBlur: () => {
      focused.current = false;
      flush(v);
    },
  };
  const id = `f-${label.replace(/\W+/g, "-").toLowerCase()}`;
  return (
    <Field label={label} hint={hint} htmlFor={id}>
      {multiline ? (
        <TextArea id={id} rows={rows} {...common} onChange={(e) => change(e.target.value)} />
      ) : (
        <TextInput id={id} {...common} onChange={(e) => change(e.target.value)} />
      )}
    </Field>
  );
}

function SavedIndicator({ state }: { state: SaveState }) {
  const text = state === "saving" ? "Saving…" : state === "error" ? "Not saved" : state === "saved" ? "Saved" : "";
  return (
    <span className="muted" role="status" style={{ fontSize: 12, marginRight: "auto" }}>
      {text}
    </span>
  );
}

function Attachments({
  items,
  ownerType,
  ownerId,
  readOnly,
}: {
  items: StoryAttachmentDto[];
  ownerType: "scene_card" | "beat" | "sequence";
  ownerId: string;
  readOnly: boolean;
}) {
  const attach = async () => {
    try {
      const path = await open({ multiple: false, directory: false, title: "Attach a reference file" });
      if (typeof path === "string") await story.addAttachment(ownerType, ownerId, path);
    } catch (e) {
      reportError(e);
    }
  };
  return (
    <Field label="Attachments" hint="Reference material only — attachments never become production assets automatically.">
      <div className="row" style={{ flexWrap: "wrap", gap: 6 }}>
        {items.map((a) => (
          <span key={a.id} className="chip" title={a.asset.available ? a.asset.originalName : "The file is missing"}>
            <button
              type="button"
              className="linklike"
              onClick={() => void openAsset(a.asset.id).catch(reportError)}
              disabled={!a.asset.available}
            >
              <FileText size={12} /> {a.asset.originalName}
            </button>
            {!readOnly && (
              <button type="button" className="linklike" aria-label={`Remove ${a.asset.originalName}`} onClick={() => void story.removeAttachment(a.id).catch(reportError)}>
                <X size={12} />
              </button>
            )}
          </span>
        ))}
        {!readOnly && (
          <Button size="xs" icon={<Paperclip size={12} />} onClick={() => void attach()}>
            Attach
          </Button>
        )}
        {items.length === 0 && readOnly && <span className="muted">None</span>}
      </div>
    </Field>
  );
}

// ---------------------------------------------------------------- scene card

export function CardDrawer({ id, board, readOnly }: { id: string; board: StoryBoardState; readOnly: boolean }) {
  const detail = useCardDetail(id);
  const [state, setState] = useState<SaveState>("idle");
  const close = () => useStoryUi.getState().openDrawer(null);
  const go = useNav((s) => s.go);
  const d = detail.data;
  if (detail.isError) {
    return (
      <Drawer open onClose={close} title="Scene Card" typeLabel="Scene Card">
        <p className="muted">This Scene Card is no longer on the Story Board. It may have been deleted.</p>
      </Drawer>
    );
  }
  if (!d) return null;
  const c = d.card;
  const ref = { kind: "card" as const, id: c.id };
  const parked = c.parentType === "parking";
  const title = c.shortDescription.trim().split("\n")[0] || c.sceneHeading || "Untitled scene";
  return (
    <Drawer
      open
      onClose={close}
      title={<span className="truncate2">{title}</span>}
      typeLabel="Scene Card"
      footer={
        <>
          <SavedIndicator state={state} />
          <Button size="sm" disabled={readOnly} onClick={() => void duplicateItems(board, [ref])}>Duplicate</Button>
          {parked ? (
            <Button size="sm" disabled={readOnly} onClick={() => void restoreToStory(board, [ref])}>Restore to Story</Button>
          ) : (
            <Button size="sm" disabled={readOnly} onClick={() => void parkItems(board, [ref])}>Send to Parking Lot</Button>
          )}
          <Button size="sm" disabled={readOnly || parked} onClick={() => useStoryUi.getState().openDialog({ type: "build", preselect: [c.id] })}>
            Convert to Screenplay Scene
          </Button>
          <Button size="sm" variant="danger" disabled={readOnly} onClick={() => requestDelete(board, [ref])}>Delete</Button>
        </>
      }
    >
      <div className="muted" style={{ fontSize: 12, marginBottom: 10 }}>{d.location}</div>
      <AutoField
        label="Scene heading (optional now — required before it becomes a screenplay scene)"
        value={c.sceneHeading ?? ""}
        placeholder="EXT. OLD RAILWAY STATION — NIGHT"
        disabled={readOnly}
        onState={setState}
        onSave={(v) => story.updateCard({ id: c.id, sceneHeading: v })}
      />
      <AutoField
        label="Short description"
        value={c.shortDescription}
        multiline
        rows={4}
        autoFocus={!c.shortDescription}
        placeholder="What happens in this scene?"
        disabled={readOnly}
        onState={setState}
        onSave={(v) => story.updateCard({ id: c.id, shortDescription: v })}
      />
      <AutoField
        label="Notes"
        value={c.notes ?? ""}
        multiline
        rows={5}
        placeholder="Freeform scene notes"
        disabled={readOnly}
        onState={setState}
        onSave={(v) => story.updateCard({ id: c.id, notes: v })}
      />
      <Attachments items={c.attachments} ownerType="scene_card" ownerId={c.id} readOnly={readOnly} />
      {c.commentCount > 0 && (
        <div className="muted" style={{ fontSize: 12.5, marginBottom: 10 }}>
          Comments ({c.commentCount} open)
        </div>
      )}
      {d.linkedScene ? (
        <div className="banner info" role="status">
          <div>
            Used to create screenplay <b>Scene {d.linkedScene.number} — {d.linkedScene.heading}</b> in “{d.linkedScene.draftName}”. Changes here
            don't change the screenplay.
          </div>
          <span className="sp" />
          <Button
            size="xs"
            onClick={() =>
              go({ workspace: "screenplay", params: { sceneId: d.linkedScene!.sceneId, draftId: d.linkedScene!.draftId, screenplayId: d.linkedScene!.screenplayId } })
            }
          >
            Open Screenplay
          </Button>
        </div>
      ) : c.screenplaySceneId ? (
        <div className="muted" style={{ fontSize: 12.5 }}>The screenplay scene created from this card no longer exists.</div>
      ) : null}
    </Drawer>
  );
}

// --------------------------------------------------------------------- beat

export function BeatDrawer({ id, board, readOnly }: { id: string; board: StoryBoardState; readOnly: boolean }) {
  const [state, setState] = useState<SaveState>("idle");
  const close = () => useStoryUi.getState().openDrawer(null);
  const loc = locate(board, { kind: "beat", id });
  if (!loc || loc.item.kind !== "beat") {
    return (
      <Drawer open onClose={close} title="Beat" typeLabel="Beat Card" width="n">
        <p className="muted">This beat is no longer on the Story Board. It may have been deleted.</p>
      </Drawer>
    );
  }
  const b = loc.item;
  const ref = { kind: "beat" as const, id };
  const parked = loc.container.parentType === "parking";
  const converted = b.state === "converted";
  const setColor = (color: string) => {
    setState("saving");
    story
      .updateBeat({ id, color })
      .then(() => setState("saved"))
      .catch((e) => {
        setState("error");
        reportError(e);
      });
  };
  return (
    <Drawer
      open
      onClose={close}
      width="n"
      title={<span className="truncate2">{b.text.trim() || "Beat"}</span>}
      typeLabel="Beat Card"
      footer={
        <>
          <SavedIndicator state={state} />
          <Button size="sm" variant="primary" disabled={readOnly || converted} onClick={() => void convertBeat(id)}>Convert to Scene</Button>
          <Button size="sm" disabled={readOnly} onClick={() => void duplicateItems(board, [ref])}>Duplicate</Button>
          {parked ? (
            <Button size="sm" disabled={readOnly} onClick={() => void restoreToStory(board, [ref])}>Restore</Button>
          ) : (
            <Button size="sm" disabled={readOnly} onClick={() => void parkItems(board, [ref])}>Park</Button>
          )}
          <Button size="sm" variant="danger" disabled={readOnly} onClick={() => requestDelete(board, [ref])}>Delete</Button>
        </>
      }
    >
      <div className="muted" style={{ fontSize: 12, marginBottom: 10 }}>{containerLabel(board, loc.container)}</div>
      {converted && (
        <div className="banner ok" role="status" style={{ marginBottom: 10 }}>
          <div>Converted to a Scene Card. This beat stays as a reference.</div>
          <span className="sp" />
          {b.convertedSceneCardId && (
            <Button size="xs" onClick={() => useStoryUi.getState().openDrawer({ kind: "card", id: b.convertedSceneCardId! })}>Open card</Button>
          )}
        </div>
      )}
      <AutoField label="Beat text" value={b.text} multiline autoFocus={!b.text} disabled={readOnly} onState={setState} onSave={(v) => story.updateBeat({ id, text: v })} />
      <AutoField label="Note (optional)" value={b.note ?? ""} multiline placeholder="Add a note…" disabled={readOnly} onState={setState} onSave={(v) => story.updateBeat({ id, note: v })} />
      <Field label="Colour (optional)">
        <div className="row" style={{ gap: 6, flexWrap: "wrap" }} role="radiogroup" aria-label="Beat colour">
          <button type="button" role="radio" aria-checked={!b.color} className={cx("swatch", !b.color && "on")} disabled={readOnly} onClick={() => setColor("")}>
            None
          </button>
          {COLOR_NAMES.map((n) => (
            <button
              key={n}
              type="button"
              role="radio"
              aria-checked={b.color === n}
              aria-label={n}
              title={n}
              className={cx("swatch", b.color === n && "on")}
              style={{ background: COLOR_VARS[n] }}
              disabled={readOnly}
              onClick={() => setColor(n)}
            />
          ))}
        </div>
      </Field>
      <Attachments items={b.attachments} ownerType="beat" ownerId={id} readOnly={readOnly} />
      <p className="muted" style={{ fontSize: 12.5, lineHeight: 1.5 }}>
        Convert to Scene creates a new Scene Card from this text. The beat stays behind marked Converted so no thought is lost.
      </p>
    </Drawer>
  );
}

// ------------------------------------------------------------ sequence hub

export function SequenceDrawer({ id, board, readOnly }: { id: string; board: StoryBoardState; readOnly: boolean }) {
  const [state, setState] = useState<SaveState>("idle");
  const close = () => useStoryUi.getState().openDrawer(null);
  const seq = sequences(board).find((s) => s.id === id);
  if (!seq) {
    return (
      <Drawer open onClose={close} title="Sequence" typeLabel="Sequence" width="n">
        <p className="muted">This sequence is no longer on the Story Board. It may have been deleted.</p>
      </Drawer>
    );
  }
  const act = seq.actId ? board.acts.find((a) => a.id === seq.actId)?.title : "Unassigned";
  const cards = seq.items.filter((i) => i.kind === "card");
  return (
    <Drawer
      open
      onClose={close}
      width="n"
      title={seq.title}
      typeLabel="Sequence"
      footer={
        <>
          <SavedIndicator state={state} />
          <Button size="sm" variant="danger" disabled={readOnly} onClick={() => requestDeleteSequence(board, id)}>
            Delete
          </Button>
        </>
      }
    >
      <div className="muted" style={{ fontSize: 12, marginBottom: 10 }}>{act}</div>
      <AutoField label="Sequence name" value={seq.title} disabled={readOnly} onState={setState} onSave={(v) => (v.trim() ? story.updateSequence({ id, title: v.trim() }) : Promise.resolve())} />
      <Field label={`Scenes (${cards.length})`}>
        <div className="col" style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          {cards.map((c) =>
            c.kind === "card" ? (
              <button
                key={c.id}
                type="button"
                className="sc"
                style={{ textAlign: "left", width: "100%" }}
                onClick={() => {
                  useStoryUi.getState().select([refKey({ kind: "card", id: c.id })]);
                  useStoryUi.getState().openDrawer({ kind: "card", id: c.id });
                }}
              >
                {c.shortDescription || c.sceneHeading || "Blank scene card"}
              </button>
            ) : null,
          )}
          {cards.length === 0 && <span className="muted">Drop scenes here from the board.</span>}
        </div>
      </Field>
      <AutoField label="Note" value={seq.note ?? ""} multiline disabled={readOnly} onState={setState} onSave={(v) => story.updateSequence({ id, note: v })} />
      <Attachments items={seq.attachments} ownerType="sequence" ownerId={id} readOnly={readOnly} />
    </Drawer>
  );
}
