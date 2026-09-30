// Typed wrappers over the Story module's application operations (story.*).
// React never owns story truth: every read is a query keyed by the tables it
// depends on, every change is a Rust command that goes through the pipeline.

import { call } from "../ipc/client";
import { queryClient, useOp } from "../ipc/query";
import type { StoryAttachmentDto } from "../ipc/generated/StoryAttachmentDto";
import type { StoryBoardState } from "../ipc/generated/StoryBoardState";
import type { StoryBuildArgs } from "../ipc/generated/StoryBuildArgs";
import type { StoryBuildPreview } from "../ipc/generated/StoryBuildPreview";
import type { StoryBuildResult } from "../ipc/generated/StoryBuildResult";
import type { StoryCardDetail } from "../ipc/generated/StoryCardDetail";
import type { StoryCharacterDetail } from "../ipc/generated/StoryCharacterDetail";
import type { StoryCharacterDto } from "../ipc/generated/StoryCharacterDto";
import type { StoryCharacterUsage } from "../ipc/generated/StoryCharacterUsage";
import type { StoryContainerRef } from "../ipc/generated/StoryContainerRef";
import type { StoryCreateBeatArgs } from "../ipc/generated/StoryCreateBeatArgs";
import type { StoryCreateCardArgs } from "../ipc/generated/StoryCreateCardArgs";
import type { StoryCreateCharacterArgs } from "../ipc/generated/StoryCreateCharacterArgs";
import type { StoryCreated } from "../ipc/generated/StoryCreated";
import type { StoryCreatedMany } from "../ipc/generated/StoryCreatedMany";
import type { StoryDeleteMode } from "../ipc/generated/StoryDeleteMode";
import type { StoryItemRef } from "../ipc/generated/StoryItemRef";
import type { StoryOrderPreview } from "../ipc/generated/StoryOrderPreview";
import type { StoryPlacement } from "../ipc/generated/StoryPlacement";
import type { StoryRelationshipDto } from "../ipc/generated/StoryRelationshipDto";
import type { StorySeriesDto } from "../ipc/generated/StorySeriesDto";
import type { StoryTimelineDto } from "../ipc/generated/StoryTimelineDto";
import type { StoryUpdateBeatArgs } from "../ipc/generated/StoryUpdateBeatArgs";
import type { StoryUpdateCardArgs } from "../ipc/generated/StoryUpdateCardArgs";
import type { StoryUpdateCharacterArgs } from "../ipc/generated/StoryUpdateCharacterArgs";
import type { StoryUpdateEpisodeArgs } from "../ipc/generated/StoryUpdateEpisodeArgs";
import type { StoryViewState } from "../ipc/generated/StoryViewState";

/** Tables read by the Story Board query (drives automatic invalidation). */
export const BOARD_TABLES = [
  "story_act",
  "story_sequence",
  "story_beat",
  "story_scene_card",
  "story_attachment",
  "story_character",
  "story_character_card_link",
  "comment",
  "asset",
];
export const SCREENPLAY_TABLES = ["screenplay", "screenplay_draft", "screenplay_scene", "screenplay_element"];
export const CHARACTER_TABLES = [
  "story_character",
  "story_character_relationship",
  "story_character_card_link",
  "story_scene_card",
  "asset",
  "cast_member",
  "catalog_item",
  ...SCREENPLAY_TABLES,
];
export const SERIES_TABLES = ["season", "episode", "story_scene_card", "project"];

type Scope = { episodeId: string | null };

// ------------------------------------------------------------------ queries

export const useStoryBoard = (episodeId: string | null) =>
  useOp<StoryBoardState>("story.board", { episodeId }, BOARD_TABLES);
export const useCardDetail = (id: string | null) =>
  useOp<StoryCardDetail>("story.card", { id }, [...BOARD_TABLES, ...SCREENPLAY_TABLES], { enabled: !!id });
export const useViewState = () => useOp<StoryViewState>("story.view_state", {}, ["project"]);
export const useSeries = () => useOp<StorySeriesDto>("story.series", {}, SERIES_TABLES);
export const useBuildPreview = (episodeId: string | null, enabled: boolean) =>
  useOp<StoryBuildPreview>("story.build_preview", { episodeId }, [...BOARD_TABLES, ...SCREENPLAY_TABLES], { enabled });
export const useOrderPreview = (episodeId: string | null, enabled: boolean) =>
  useOp<StoryOrderPreview>("story.order_preview", { episodeId }, [...BOARD_TABLES, ...SCREENPLAY_TABLES], { enabled });
export const useCharacters = (episodeId: string | null, includeArchived: boolean) =>
  useOp<StoryCharacterDto[]>("story.characters", { episodeId, includeArchived }, CHARACTER_TABLES);
export const useCharacter = (id: string | null, episodeId: string | null) =>
  useOp<StoryCharacterDetail>("story.character", { id, episodeId }, CHARACTER_TABLES, { enabled: !!id });
export const useRelationships = () =>
  useOp<StoryRelationshipDto[]>("story.relationships", {}, CHARACTER_TABLES);
export const useTimeline = (episodeId: string | null, screenplayId: string | null) =>
  useOp<StoryTimelineDto>("story.timeline", { episodeId, screenplayId }, [...SCREENPLAY_TABLES, "episode"]);

/** View-state writes don't emit data events (they are not story data), so refresh by hand. */
function refreshViewState() {
  void queryClient.invalidateQueries({ queryKey: ["story.view_state"] });
  void queryClient.invalidateQueries({ queryKey: ["story.board"] });
}

// ----------------------------------------------------------------- commands

export const story = {
  // view state
  setCollapsed: (id: string, collapsed: boolean) =>
    call<StoryViewState>("story.set_collapsed", { id, collapsed }).then((r) => {
      refreshViewState();
      return r;
    }),
  setCurrentEpisode: (episodeId: string | null) =>
    call<StoryViewState>("story.set_current_episode", { episodeId }).then((r) => {
      refreshViewState();
      return r;
    }),

  // acts
  createAct: (a: Scope & { title: string; beforeId?: string | null }) => call<StoryCreated>("story.create_act", a),
  updateAct: (a: { id: string; title?: string; note?: string }) => call<void>("story.update_act", a),
  moveAct: (id: string, beforeId: string | null) => call<void>("story.move_act", { id, beforeId }),
  deleteAct: (id: string, mode?: StoryDeleteMode, moveToActId?: string | null) =>
    call<void>("story.delete_act", { id, mode: mode ?? null, moveToActId: moveToActId ?? null }),

  // sequences
  createSequence: (actId: string, title: string, index?: number | null) =>
    call<StoryCreated>("story.create_sequence", { actId, title, index: index ?? null }),
  updateSequence: (a: { id: string; title?: string; note?: string }) => call<void>("story.update_sequence", a),
  deleteSequence: (id: string, mode?: StoryDeleteMode, moveTo?: StoryContainerRef | null) =>
    call<void>("story.delete_sequence", { id, mode: mode ?? null, moveTo: moveTo ?? null }),

  // beats & cards
  createBeat: (a: Partial<StoryCreateBeatArgs>) => call<StoryCreated>("story.create_beat", a),
  updateBeat: (a: Partial<StoryUpdateBeatArgs> & { id: string }) => call<void>("story.update_beat", a),
  convertBeat: (id: string) => call<StoryCreated>("story.convert_beat", { id }),
  createCard: (a: Partial<StoryCreateCardArgs>) => call<StoryCreated>("story.create_card", a),
  updateCard: (a: Partial<StoryUpdateCardArgs> & { id: string }) => call<void>("story.update_card", a),
  cardToBeat: (id: string) => call<StoryCreated>("story.card_to_beat", { id }),

  // multi-select / drag & drop
  moveItems: (items: StoryItemRef[], target: StoryContainerRef, before: StoryItemRef | null, episodeId: string | null) =>
    call<void>("story.move_items", { items, target, before, episodeId }),
  duplicateItems: (items: StoryItemRef[]) => call<StoryCreatedMany>("story.duplicate_items", { items }),
  parkItems: (items: StoryItemRef[]) => call<void>("story.park_items", { items }),
  restoreFromParking: (items: StoryItemRef[]) => call<StoryPlacement[]>("story.restore_from_parking", { items }),
  deleteItems: (items: StoryItemRef[]) => call<void>("story.delete_items", { items }),

  // attachments
  addAttachment: (ownerType: "scene_card" | "beat" | "sequence", ownerId: string, path: string) =>
    call<StoryAttachmentDto>("story.add_attachment", { ownerType, ownerId, path }),
  removeAttachment: (id: string) => call<void>("story.remove_attachment", { id }),

  // build screenplay
  buildScreenplay: (a: StoryBuildArgs) => call<StoryBuildResult>("story.build_screenplay", a),
  applyOrder: (episodeId: string | null, draftId: string | null, confirmed: boolean) =>
    call<StoryOrderPreview>("story.apply_order", { episodeId, draftId, confirmed }),

  // characters
  createCharacter: (a: Partial<StoryCreateCharacterArgs> & { name: string }) => call<StoryCreated>("story.create_character", a),
  updateCharacter: (a: Partial<StoryUpdateCharacterArgs> & { id: string }) => call<void>("story.update_character", a),
  setCharacterArchived: (id: string, archived: boolean) => call<void>("story.set_character_archived", { id, archived }),
  setCharacterImage: (id: string, path: string) => call<void>("story.set_character_image", { id, path }),
  clearCharacterImage: (id: string) => call<void>("story.clear_character_image", { id }),
  characterUsage: (id: string) => call<StoryCharacterUsage>("story.character_usage", { id }),
  deleteCharacter: (id: string) => call<void>("story.delete_character", { id }),
  createRelationship: (fromCharacterId: string, toCharacterId: string, relationshipType: string, note?: string) =>
    call<StoryCreated>("story.create_relationship", { fromCharacterId, toCharacterId, relationshipType, note: note ?? null }),
  updateRelationship: (id: string, relationshipType?: string, note?: string) =>
    call<void>("story.update_relationship", { id, relationshipType: relationshipType ?? null, note: note ?? null }),
  deleteRelationship: (id: string) => call<void>("story.delete_relationship", { id }),
  linkCard: (characterId: string, cardId: string) => call<void>("story.link_character_card", { characterId, cardId }),
  unlinkCard: (characterId: string, cardId: string) => call<void>("story.unlink_character_card", { characterId, cardId }),

  // timeline
  assignStoryDay: (sceneIds: string[], storyDay: string | null) => call<void>("story.assign_story_day", { sceneIds, storyDay }),
  setTimeNote: (sceneId: string, timeNote: string | null) => call<void>("story.set_time_note", { sceneId, timeNote }),

  // seasons & episodes
  createSeason: (title?: string) => call<StoryCreated>("story.create_season", { title: title ?? null }),
  updateSeason: (id: string, title?: string, note?: string) =>
    call<void>("story.update_season", { id, title: title ?? null, note: note ?? null }),
  moveSeason: (id: string, beforeId: string | null) => call<void>("story.move_season", { id, beforeId }),
  deleteSeason: (id: string) => call<void>("story.delete_season", { id }),
  createEpisode: (seasonId: string | null, title: string, summary?: string, status?: string) =>
    call<StoryCreated>("story.create_episode", { seasonId, title, summary: summary ?? null, status: status ?? null }),
  updateEpisode: (a: Partial<StoryUpdateEpisodeArgs> & { id: string }) => call<void>("story.update_episode", a),
  moveEpisode: (id: string, seasonId: string | null, beforeId: string | null) =>
    call<void>("story.move_episode", { id, seasonId, beforeId }),
  deleteEpisode: (id: string) => call<void>("story.delete_episode", { id }),
  duplicateEpisode: (id: string, copyStory: boolean) => call<StoryCreated>("story.duplicate_episode", { id, copyStory }),
};
