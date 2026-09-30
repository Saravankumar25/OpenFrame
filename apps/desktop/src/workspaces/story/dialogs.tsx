// Story Board decision dialogs (UX §3.6; mocks 061, 062): deleting containers
// recommends moving creative material out; deleting a linked card explains
// that the screenplay scene remains. Plus Move… and the act note editor.

import { useState, type ReactNode } from "react";
import { Button, ConfirmDialog, Dialog, Field, Select, TextArea } from "../../design-system";
import { reportError } from "../../ipc/query";
import { toast } from "../../app/toast";
import { story, useCardDetail } from "../../api/story";
import type { StoryBoardState } from "../../ipc/generated/StoryBoardState";
import type { StoryContainerRef } from "../../ipc/generated/StoryContainerRef";
import { deleteNow, runMove } from "./actions";
import { containerKey, locate, moveDestinations, sequences, type Ref } from "./model";
import { useStoryUi } from "./ui";

const close = () => useStoryUi.getState().openDialog(null);

function Radio({ name, checked, onChange, title, children, tag }: { name: string; checked: boolean; onChange: () => void; title: string; children?: ReactNode; tag?: string }) {
  return (
    <label className="story-radio">
      <input type="radio" name={name} checked={checked} onChange={onChange} />
      <span>
        <b>{title}</b> {tag && <span className="chip g">{tag}</span>}
        {children && <span className="muted" style={{ display: "block", fontSize: 12.5 }}>{children}</span>}
      </span>
    </label>
  );
}

const plural = (n: number, w: string) => `${n} ${w}${n === 1 ? "" : "s"}`;

/** Delete an Act: empty acts go straight to Recently Deleted; others ask first. */
export function requestDeleteAct(board: StoryBoardState, actId: string) {
  const act = board.acts.find((a) => a.id === actId);
  if (!act) return;
  if (act.items.length === 0) {
    void story
      .deleteAct(actId)
      .then(() => toast.undoable(`Deleted Act “${act.title}”`))
      .catch(reportError);
    return;
  }
  useStoryUi.getState().openDialog({ type: "deleteAct", actId });
}

export function requestDeleteSequence(board: StoryBoardState, sequenceId: string) {
  const seq = sequences(board).find((s) => s.id === sequenceId);
  if (!seq) return;
  if (seq.items.length === 0) {
    void story
      .deleteSequence(sequenceId)
      .then(() => toast.undoable(`Deleted Sequence “${seq.title}”`))
      .catch(reportError);
    return;
  }
  useStoryUi.getState().openDialog({ type: "deleteSequence", sequenceId });
}

export function DeleteActDialog({ board, actId }: { board: StoryBoardState; actId: string }) {
  const act = board.acts.find((a) => a.id === actId);
  const others = board.acts.filter((a) => a.id !== actId);
  const [mode, setMode] = useState<"moveContents" | "deleteAll">("moveContents");
  const [dest, setDest] = useState<string>(others[0]?.id ?? "unassigned");
  const [busy, setBusy] = useState(false);
  if (!act) return null;
  const seqs = act.items.filter((i) => i.kind === "sequence").length;
  const beats = act.items.reduce(
    (n, i) => n + (i.kind === "beat" ? 1 : i.kind === "sequence" ? i.items.filter((c) => c.kind === "beat").length : 0),
    0,
  );
  const run = async () => {
    setBusy(true);
    try {
      if (mode === "moveContents") await story.deleteAct(actId, "moveContents", dest === "unassigned" ? null : dest);
      else await story.deleteAct(actId, "deleteAll");
      toast.undoable(`Deleted Act “${act.title}”`);
      close();
    } catch (e) {
      reportError(e);
    } finally {
      setBusy(false);
    }
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && close()}
      size="md"
      title={`Delete “${act.title}”?`}
      footer={
        <>
          <Button onClick={close}>Cancel</Button>
          <Button variant={mode === "deleteAll" ? "danger" : "primary"} disabled={busy} onClick={() => void run()}>
            {mode === "moveContents" ? "Move Contents and Delete Act" : "Delete Act and Contents"}
          </Button>
        </>
      }
    >
      <p style={{ marginTop: 0 }}>
        This Act contains {plural(seqs, "Sequence")} and {plural(act.cardCount, "card")}
        {beats > 0 ? ` and ${plural(beats, "beat")}` : ""}. Your creative material is not deleted unless you
        choose that.
      </p>
      <Radio name="del-act" checked={mode === "moveContents"} onChange={() => setMode("moveContents")} title="Move its contents to another Act" tag="Recommended">
        Then delete only the empty Act.
      </Radio>
      {mode === "moveContents" && (
        <div style={{ margin: "4px 0 10px 26px" }}>
          <Field label="Move contents to">
            <Select
              value={dest}
              onChange={setDest}
              ariaLabel="Move contents to"
              options={[...others.map((a) => ({ value: a.id, label: a.title })), { value: "unassigned", label: "Unassigned (keep aside)" }]}
            />
          </Field>
        </div>
      )}
      <Radio name="del-act" checked={mode === "deleteAll"} onChange={() => setMode("deleteAll")} title="Delete the Act and everything inside it">
        The cards go to Recently Deleted and can be restored.
      </Radio>
    </Dialog>
  );
}

export function DeleteSequenceDialog({ board, sequenceId }: { board: StoryBoardState; sequenceId: string }) {
  const seq = sequences(board).find((s) => s.id === sequenceId);
  const [mode, setMode] = useState<"moveContents" | "deleteAll">("moveContents");
  const [dest, setDest] = useState<string>("here");
  const [busy, setBusy] = useState(false);
  if (!seq) return null;
  const actTitle = seq.actId ? board.acts.find((a) => a.id === seq.actId)?.title ?? "its Act" : "Unassigned";
  const options = [
    { value: "here", label: `${actTitle} (where the sequence is now)` },
    ...board.acts.filter((a) => a.id !== seq.actId).map((a) => ({ value: `act:${a.id}`, label: a.title })),
    ...sequences(board)
      .filter((s) => s.id !== sequenceId)
      .map((s) => ({ value: `sequence:${s.id}`, label: `Sequence — ${s.title}` })),
  ];
  const run = async () => {
    setBusy(true);
    try {
      if (mode === "deleteAll") await story.deleteSequence(sequenceId, "deleteAll");
      else {
        const moveTo: StoryContainerRef | null =
          dest === "here" ? null : { parentType: dest.startsWith("act:") ? "act" : "sequence", parentId: dest.slice(dest.indexOf(":") + 1) };
        await story.deleteSequence(sequenceId, "moveContents", moveTo);
      }
      toast.undoable(`Deleted Sequence “${seq.title}”`);
      close();
    } catch (e) {
      reportError(e);
    } finally {
      setBusy(false);
    }
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && close()}
      size="md"
      title={`Delete “${seq.title}”?`}
      footer={
        <>
          <Button onClick={close}>Cancel</Button>
          <Button variant={mode === "deleteAll" ? "danger" : "primary"} disabled={busy} onClick={() => void run()}>
            {mode === "moveContents" ? "Move Cards and Delete Sequence" : "Delete Sequence and Cards"}
          </Button>
        </>
      }
    >
      <p style={{ marginTop: 0 }}>
        This Sequence contains {plural(seq.items.length, "item")}. Your creative material is not deleted unless you choose that.
      </p>
      <Radio name="del-seq" checked={mode === "moveContents"} onChange={() => setMode("moveContents")} title="Move its cards out first" tag="Recommended">
        Then delete only the empty Sequence.
      </Radio>
      {mode === "moveContents" && (
        <div style={{ margin: "4px 0 10px 26px" }}>
          <Field label="Move cards to">
            <Select value={dest} onChange={setDest} ariaLabel="Move cards to" options={options} />
          </Field>
        </div>
      )}
      <Radio name="del-seq" checked={mode === "deleteAll"} onChange={() => setMode("deleteAll")} title="Delete the Sequence and everything inside it">
        The cards go to Recently Deleted and can be restored.
      </Radio>
    </Dialog>
  );
}

/** Deleting cards linked to screenplay scenes (mock 062). */
export function DeleteLinkedDialog({ board, refs }: { board: StoryBoardState; refs: Ref[] }) {
  const firstLinked = refs.find((r) => {
    const loc = locate(board, r);
    return loc?.item.kind === "card" && !!loc.item.screenplaySceneId;
  });
  const detail = useCardDetail(firstLinked?.id ?? null);
  const scene = detail.data?.linkedScene;
  const single = refs.length === 1;
  return (
    <ConfirmDialog
      open
      onOpenChange={(v) => !v && close()}
      title={single ? "Delete this Scene Card?" : `Delete ${refs.length} items?`}
      confirmLabel={single ? "Delete Card" : "Delete"}
      danger
      onConfirm={() => {
        close();
        void deleteNow(board, refs);
      }}
    >
      <p style={{ marginTop: 0 }}>
        {single ? "Its linked screenplay scene" : "Linked screenplay scenes"}
        {scene ? ` (Scene ${scene.number} — ${scene.heading})` : ""} will remain. Screenplay scenes are never deleted from the Story Board.
      </p>
      <p className="muted">
        {single ? "The card moves" : "They move"} to Recently Deleted. You can restore {single ? "it" : "them"} or press Ctrl+Z.
      </p>
    </ConfirmDialog>
  );
}

export function MoveDialog({ board, refs, episodeId }: { board: StoryBoardState; refs: Ref[]; episodeId: string | null }) {
  const kinds = [...new Set(refs.map((r) => r.kind))];
  const dests = moveDestinations(board, kinds);
  const [dest, setDest] = useState(dests[0] ? containerKey(dests[0].container) : "");
  const chosen = dests.find((d) => containerKey(d.container) === dest);
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && close()}
      size="sm"
      title={refs.length === 1 ? "Move to…" : `Move ${refs.length} items to…`}
      footer={
        <>
          <Button onClick={close}>Cancel</Button>
          <Button
            variant="primary"
            disabled={!chosen}
            onClick={() => {
              close();
              if (chosen) void runMove(board, refs, { target: chosen.container, before: null }, episodeId);
            }}
          >
            Move
          </Button>
        </>
      }
    >
      {dests.length === 0 ? (
        <p className="muted">Add an Act first.</p>
      ) : (
        <Field label="Destination" hint="Items are added at the end and keep their order.">
          <Select
            value={dest}
            onChange={setDest}
            ariaLabel="Destination"
            options={dests.map((d) => ({ value: containerKey(d.container), label: `${d.depth ? "    › " : ""}${d.label}` }))}
          />
        </Field>
      )}
    </Dialog>
  );
}

export function ActNoteDialog({ board, actId }: { board: StoryBoardState; actId: string }) {
  const act = board.acts.find((a) => a.id === actId);
  const [note, setNote] = useState(act?.note ?? "");
  if (!act) return null;
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && close()}
      size="sm"
      title={`Note for “${act.title}”`}
      footer={
        <>
          <Button onClick={close}>Cancel</Button>
          <Button
            variant="primary"
            onClick={() => {
              close();
              void story.updateAct({ id: actId, note }).catch(reportError);
            }}
          >
            Save Note
          </Button>
        </>
      }
    >
      <Field label="Note" htmlFor="act-note">
        <TextArea id="act-note" rows={5} value={note} onChange={(e) => setNote(e.target.value)} autoFocus placeholder="Optional note about this Act" />
      </Field>
    </Dialog>
  );
}

