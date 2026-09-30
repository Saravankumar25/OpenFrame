// Typed reads for the screenplay workspace. Each query declares every table it
// reads so Rust `dataChanged` events refresh exactly what changed.

import { useOp } from "../../ipc/query";
import type { ScreenplayOverview } from "../../ipc/generated/ScreenplayOverview";
import type { ScreenplayDocument } from "../../ipc/generated/ScreenplayDocument";
import type { CommentThread } from "../../ipc/generated/CommentThread";
import type { PrivateNoteDto } from "../../ipc/generated/PrivateNoteDto";
import type { ScreenplayHistoryPoint } from "../../ipc/generated/ScreenplayHistoryPoint";
import type { ScreenplayReviewRoundDto } from "../../ipc/generated/ScreenplayReviewRoundDto";
import type { ScreenplayCharacterReport } from "../../ipc/generated/ScreenplayCharacterReport";
import type { ScreenplayLockSummary } from "../../ipc/generated/ScreenplayLockSummary";
import type { ScreenplayCompareResult } from "../../ipc/generated/ScreenplayCompareResult";
import type { ScreenplaySceneHub } from "../../ipc/generated/ScreenplaySceneHub";
import type { ScreenplayStoryReference } from "../../ipc/generated/ScreenplayStoryReference";

const CONTENT = ["screenplay", "screenplay_draft", "screenplay_scene", "screenplay_element"];

export function useOverview(episodeId: string | null) {
  return useOp<ScreenplayOverview>(
    "screenplay.overview",
    episodeId ? { episodeId } : {},
    [...CONTENT, "comment", "episode", "season", "project"],
  );
}

export function useDocument(draftId: string | null) {
  return useOp<ScreenplayDocument>("screenplay.document", { draftId }, [...CONTENT, "comment"], { enabled: !!draftId });
}

export function useDraftComments(draftId: string | null) {
  return useOp<CommentThread[]>("comment.list", { draftId }, ["comment", "screenplay_scene", "screenplay_element", "review_round"], {
    enabled: !!draftId,
  });
}

export function usePrivateNotes(targetType: string | null, targetId: string | null) {
  return useOp<PrivateNoteDto[]>(
    "private_note.list",
    targetType && targetId ? { targetType, targetId } : {},
    ["private_note"],
    { enabled: !!targetId },
  );
}

export function useHistoryPoints(draftId: string | null) {
  return useOp<ScreenplayHistoryPoint[]>("screenplay.history_points", { draftId }, ["screenplay_history_point", "screenplay_draft"], {
    enabled: !!draftId,
  });
}

export function useReviewRounds(screenplayId: string | null) {
  return useOp<ScreenplayReviewRoundDto[]>("screenplay.review_rounds", { screenplayId }, ["review_round", "comment", "screenplay_draft"], {
    enabled: !!screenplayId,
  });
}

export function useCharacters(draftId: string | null) {
  return useOp<ScreenplayCharacterReport>(
    "screenplay.characters",
    { draftId },
    ["screenplay_element", "screenplay_scene", "story_character", "screenplay_character_link"],
    { enabled: !!draftId },
  );
}

export function useLockSummary(draftId: string | null) {
  return useOp<ScreenplayLockSummary>("screenplay.lock_summary", { draftId }, [...CONTENT, "comment"], { enabled: !!draftId });
}

export function useCompare(draftA: string | null, draftB: string | null) {
  return useOp<ScreenplayCompareResult>("screenplay.compare", { draftA, draftB }, CONTENT, {
    enabled: !!draftA && !!draftB && draftA !== draftB,
  });
}

export function useRevisionChanges(draftId: string | null) {
  return useOp<ScreenplayCompareResult>("screenplay.revision_changes", { draftId }, CONTENT, { enabled: !!draftId });
}

export function useSceneHub(sceneId: string | null) {
  return useOp<ScreenplaySceneHub>("screenplay.scene_hub", { sceneId }, [...CONTENT, "comment", "breakdown_element", "story_scene_card", "*"], {
    enabled: !!sceneId,
  });
}

export function useStoryReference(sceneId: string | null) {
  return useOp<ScreenplayStoryReference>(
    "screenplay.story_reference",
    { sceneId },
    ["story_scene_card", "story_act", "story_sequence", "screenplay_scene"],
    { enabled: !!sceneId },
  );
}
