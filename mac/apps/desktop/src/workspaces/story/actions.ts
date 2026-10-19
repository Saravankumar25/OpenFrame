// Story Board actions shared by Board and Outline views: every one calls a
// Rust command and confirms with a human toast (with Undo where it applies).

import { story } from "../../api/story";
import { reportError } from "../../ipc/query";
import { toast } from "../../app/toast";
import type { StoryBoardState } from "../../ipc/generated/StoryBoardState";
import type { StoryContainerRef } from "../../ipc/generated/StoryContainerRef";
import { boardOrder, containerLabel, itemLabel, locate, refKey, type DropHint, type Ref, SEQ } from "./model";
import { useStoryUi } from "./ui";

const quote = (s: string) => `“${s.length > 60 ? `${s.slice(0, 57)}…` : s}”`;

function describe(board: StoryBoardState, refs: Ref[]): string {
  if (refs.length === 1) {
    const loc = locate(board, refs[0]);
    const kind = refs[0].kind === "card" ? "Scene Card" : refs[0].kind === "beat" ? "Beat" : "Sequence";
    return loc ? `${kind} ${quote(itemLabel(loc.item))}` : kind;
  }
  return refs.every((r) => r.kind === "card") ? `${refs.length} Scene Cards` : `${refs.length} items`;
}

export async function runMove(board: StoryBoardState, refs: Ref[], hint: DropHint, episodeId: string | null): Promise<void> {
  const ordered = boardOrder(board, refs);
  try {
    await story.moveItems(ordered, hint.target, hint.before, episodeId);
    const where = hint.target.parentType === "parking" ? "the Parking Lot" : `${hint.target.parentType === "act" ? "Act" : hint.target.parentType === "sequence" ? "Sequence" : ""} ${quote(containerLabel(board, hint.target))}`.trim();
    toast.undoable(`Moved ${describe(board, ordered)} to ${where}`);
  } catch (e) {
    reportError(e);
  }
}

export async function parkItems(board: StoryBoardState, refs: Ref[]): Promise<void> {
  const r = refs.filter((x) => x.kind !== "sequence");
  if (r.length === 0) return;
  try {
    await story.parkItems(r);
    toast.undoable(`Moved ${describe(board, r)} to the Parking Lot`);
  } catch (e) {
    reportError(e);
  }
}

export async function restoreToStory(board: StoryBoardState, refs: Ref[]): Promise<void> {
  try {
    const placed = await story.restoreFromParking(refs);
    if (placed.length === 1) toast.undoable(`Restored ${describe(board, refs)} to ${placed[0].label}`);
    else if (placed.length > 1) toast.undoable(`Restored ${placed.length} items to the story`);
  } catch (e) {
    reportError(e);
  }
}

export async function duplicateItems(board: StoryBoardState, refs: Ref[]): Promise<void> {
  const r = refs.filter((x) => x.kind !== "sequence");
  if (r.length === 0) return;
  try {
    const out = await story.duplicateItems(r);
    useStoryUi.getState().select(out.ids.map((id, i) => refKey({ kind: r[i]?.kind ?? "card", id })));
    toast.undoable(`Duplicated ${describe(board, r)}`);
  } catch (e) {
    reportError(e);
  }
}

/** Delete beats/cards; a card linked to a screenplay scene needs the warning dialog first (FSD §11.8). */
export function requestDelete(board: StoryBoardState, refs: Ref[]): void {
  const r = refs.filter((x) => x.kind !== "sequence");
  if (r.length === 0) return;
  const linked = r.some((x) => {
    const loc = locate(board, x);
    return loc?.item.kind === "card" && !!loc.item.screenplaySceneId;
  });
  if (linked) {
    useStoryUi.getState().openDialog({ type: "deleteLinked", refs: r });
    return;
  }
  void deleteNow(board, r);
}

export async function deleteNow(board: StoryBoardState, refs: Ref[]): Promise<void> {
  try {
    const label = describe(board, refs);
    await story.deleteItems(refs);
    const ui = useStoryUi.getState();
    ui.select(ui.selected.filter((k) => !refs.some((r) => refKey(r) === k)));
    if (ui.drawer && refs.some((r) => r.id === ui.drawer?.id)) ui.openDrawer(null);
    toast.undoable(`Deleted ${label}`);
  } catch (e) {
    reportError(e);
  }
}

export async function convertBeat(id: string): Promise<void> {
  try {
    const card = await story.convertBeat(id);
    useStoryUi.getState().openDrawer({ kind: "card", id: card.id });
    toast.undoable("Converted Beat to a Scene Card. The beat stays, marked Converted.");
  } catch (e) {
    reportError(e);
  }
}

export async function copyAsBeat(id: string): Promise<void> {
  try {
    const beat = await story.cardToBeat(id);
    useStoryUi.getState().select([refKey({ kind: "beat", id: beat.id })]);
    toast.undoable("Copied the Scene Card as a Beat. The card is unchanged.");
  } catch (e) {
    reportError(e);
  }
}

/** Where "Add Scene Card" puts a card: after the selected item, into a selected sequence, or the end of the story. */
export function defaultInsert(board: StoryBoardState, selected: string[]): { container: StoryContainerRef | null; index?: number } {
  const key = selected[selected.length - 1];
  if (key) {
    const [kind, ...rest] = key.split(":");
    const ref = { kind, id: rest.join(":") } as Ref;
    if (ref.kind === "sequence") return { container: SEQ(ref.id) };
    const loc = locate(board, ref);
    if (loc && loc.container.parentType !== "unassigned") return { container: loc.container, index: loc.index + 1 };
  }
  const lastAct = board.acts[board.acts.length - 1];
  if (lastAct) {
    const lastSeq = [...lastAct.items].reverse().find((i) => i.kind === "sequence");
    if (lastSeq && lastAct.items[lastAct.items.length - 1]?.id === lastSeq.id) return { container: SEQ(lastSeq.id) };
    return { container: { parentType: "act", parentId: lastAct.id } };
  }
  return { container: null };
}

export async function newCard(episodeId: string | null, container: StoryContainerRef | null, index?: number): Promise<void> {
  try {
    const r = await story.createCard({ parent: container, episodeId, shortDescription: "", index: index ?? null });
    const ui = useStoryUi.getState();
    ui.select([refKey({ kind: "card", id: r.id })]);
    ui.setEditing(refKey({ kind: "card", id: r.id }));
  } catch (e) {
    reportError(e);
  }
}

export async function newBeat(episodeId: string | null, container: StoryContainerRef | null, index?: number): Promise<void> {
  try {
    const r = await story.createBeat({ parent: container, episodeId, text: "", index: index ?? null });
    const ui = useStoryUi.getState();
    ui.select([refKey({ kind: "beat", id: r.id })]);
    ui.setEditing(refKey({ kind: "beat", id: r.id }));
  } catch (e) {
    reportError(e);
  }
}
