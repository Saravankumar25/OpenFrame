//! Where every tool lives (ADR-0041 decision 7): the table the agent's tool list is filtered by when the
//! person picked sections. A tool is offered when one of its `any` sections is picked and all of its
//! `also` sections are. The few general tools live in every section, and what they read is held to the
//! picks by their own code. One table, hand kept; the coverage test fails when a tool has no row.

/// A tool that lives in every section.
const EVERY: &[&str] = &[];

/// One tool and its home.
pub struct ToolHome {
    pub tool: &'static str,
    /// One of these must be picked. Empty: every section.
    pub any: &'static [&'static str],
    /// All of these must be picked too.
    pub also: &'static [&'static str],
}

pub const TOOL_HOMES: &[ToolHome] = &[
    ToolHome {
        tool: "project_overview",
        any: &["production/overview"],
        also: &[],
    },
    ToolHome {
        tool: "list_drafts",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "count_scenes",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "scenes_with_characters",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "find_scenes_mentioning",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "compare_drafts",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "scene_fact",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "dialogue_fact",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "open_scene",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "count_characters",
        any: &["story/characters", "screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "list_characters",
        any: &["story/characters", "screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "location_statistics",
        any: &["production/locations", "screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "locations_in_act",
        any: &["story/beat-board", "story/outline"],
        also: &[],
    },
    ToolHome {
        tool: "unscheduled_scenes",
        any: &["production/schedule", "production/daily-view"],
        also: &[],
    },
    ToolHome {
        tool: "breakdown_status",
        any: &["breakdown/breakdown"],
        also: &[],
    },
    ToolHome {
        tool: "story_statistics",
        any: &["story/beat-board", "story/outline", "story/characters"],
        also: &[],
    },
    ToolHome {
        tool: "production_statistics",
        any: &["production/overview"],
        also: &[],
    },
    ToolHome {
        tool: "search_project",
        any: EVERY,
        also: &[],
    },
    ToolHome {
        tool: "open_workspace",
        any: EVERY,
        also: &[],
    },
    ToolHome {
        tool: "open_object",
        any: EVERY,
        also: &[],
    },
    ToolHome {
        tool: "retrieve_context",
        any: EVERY,
        also: &[],
    },
    ToolHome {
        tool: "suggest_breakdown",
        any: &["breakdown/breakdown"],
        also: &["screenplay/scenes"],
    },
    ToolHome {
        tool: "private_information",
        any: &["notes/notes", "screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "project_settings",
        any: &["help/settings"],
        also: &[],
    },
    ToolHome {
        tool: "app_appearance",
        any: &["help/settings"],
        also: &[],
    },
    ToolHome {
        tool: "app_settings",
        any: &["help/settings"],
        also: &[],
    },
    ToolHome {
        tool: "recently_deleted",
        any: &["help/settings"],
        also: &[],
    },
    ToolHome {
        tool: "recent_activity",
        any: &["help/settings"],
        also: &[],
    },
    ToolHome {
        tool: "templates",
        any: &["help/settings"],
        also: &[],
    },
    ToolHome {
        tool: "project_files",
        any: &["files/files"],
        also: &[],
    },
    ToolHome {
        tool: "package_history",
        any: &["files/files"],
        also: &[],
    },
    ToolHome {
        tool: "project_notes",
        any: &["notes/notes"],
        also: &[],
    },
    ToolHome {
        tool: "project_tasks",
        any: &["notes/tasks"],
        also: &[],
    },
    ToolHome {
        tool: "comments",
        any: &["notes/notes", "screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "idea_vault",
        any: &["vault/project", "vault/global"],
        also: &[],
    },
    ToolHome {
        tool: "story_outline",
        any: &["story/beat-board", "story/outline"],
        also: &[],
    },
    ToolHome {
        tool: "story_card",
        any: &["story/beat-board", "story/outline"],
        also: &[],
    },
    ToolHome {
        tool: "story_characters",
        any: &["story/characters"],
        also: &[],
    },
    ToolHome {
        tool: "story_series",
        any: &["story/episodes"],
        also: &[],
    },
    ToolHome {
        tool: "story_timeline",
        any: &["story/timeline"],
        also: &[],
    },
    ToolHome {
        tool: "screenplay_scenes",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "screenplay_scene",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "draft_status",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "screenplay_pages",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "character_cues",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "breakdown_scene",
        any: &["breakdown/breakdown"],
        also: &[],
    },
    ToolHome {
        tool: "production_catalog",
        any: &["production/catalog"],
        also: &[],
    },
    ToolHome {
        tool: "production_locations",
        any: &["production/locations"],
        also: &[],
    },
    ToolHome {
        tool: "cast_and_crew",
        any: &["production/cast-crew"],
        also: &[],
    },
    ToolHome {
        tool: "production_source",
        any: &["production/overview"],
        also: &[],
    },
    ToolHome {
        tool: "visual_planning",
        any: &[
            "production/moodboards",
            "production/storyboards",
            "production/shots",
        ],
        also: &[],
    },
    ToolHome {
        tool: "moodboards",
        any: &["production/moodboards"],
        also: &[],
    },
    ToolHome {
        tool: "storyboards",
        any: &["production/storyboards"],
        also: &[],
    },
    ToolHome {
        tool: "shot_list",
        any: &["production/shots"],
        also: &[],
    },
    ToolHome {
        tool: "schedule_overview",
        any: &["production/schedule", "production/daily-view"],
        also: &[],
    },
    ToolHome {
        tool: "schedule_day",
        any: &["production/schedule", "production/daily-view"],
        also: &[],
    },
    ToolHome {
        tool: "day_out_of_days",
        any: &["production/day-out-of-days"],
        also: &[],
    },
    ToolHome {
        tool: "call_sheets",
        any: &["callsheets/callsheets"],
        also: &[],
    },
    ToolHome {
        tool: "sides_and_reports",
        any: &["production/sides", "production/reports"],
        also: &[],
    },
    ToolHome {
        tool: "production_report",
        any: &["production/reports", "production/overview"],
        also: &[],
    },
    ToolHome {
        tool: "budget_summary",
        any: &["production/budget"],
        also: &[],
    },
    ToolHome {
        tool: "propose_scene_card",
        any: &["story/beat-board", "story/outline"],
        also: &[],
    },
    ToolHome {
        tool: "propose_act",
        any: &["story/beat-board", "story/outline"],
        also: &[],
    },
    ToolHome {
        tool: "propose_update_act",
        any: &["story/beat-board", "story/outline"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_act",
        any: &["story/beat-board", "story/outline"],
        also: &[],
    },
    ToolHome {
        tool: "propose_sequence",
        any: &["story/beat-board", "story/outline"],
        also: &[],
    },
    ToolHome {
        tool: "propose_update_sequence",
        any: &["story/beat-board", "story/outline"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_sequence",
        any: &["story/beat-board", "story/outline"],
        also: &[],
    },
    ToolHome {
        tool: "propose_beat",
        any: &["story/beat-board", "story/outline"],
        also: &[],
    },
    ToolHome {
        tool: "propose_update_beat",
        any: &["story/beat-board", "story/outline"],
        also: &[],
    },
    ToolHome {
        tool: "propose_convert_beat",
        any: &["story/beat-board", "story/outline"],
        also: &[],
    },
    ToolHome {
        tool: "propose_update_scene_card",
        any: &["story/beat-board", "story/outline"],
        also: &[],
    },
    ToolHome {
        tool: "propose_convert_card_to_beat",
        any: &["story/beat-board", "story/outline"],
        also: &[],
    },
    ToolHome {
        tool: "propose_move_story_items",
        any: &["story/beat-board", "story/outline"],
        also: &[],
    },
    ToolHome {
        tool: "propose_duplicate_story_items",
        any: &["story/beat-board", "story/outline"],
        also: &[],
    },
    ToolHome {
        tool: "propose_park_story_items",
        any: &["story/beat-board", "story/outline"],
        also: &[],
    },
    ToolHome {
        tool: "propose_unpark_story_items",
        any: &["story/beat-board", "story/outline"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_story_items",
        any: &["story/beat-board", "story/outline"],
        also: &[],
    },
    ToolHome {
        tool: "propose_remove_story_attachment",
        any: &["story/beat-board", "story/outline"],
        also: &[],
    },
    ToolHome {
        tool: "propose_build_screenplay",
        any: &["story/beat-board", "story/outline"],
        also: &[],
    },
    ToolHome {
        tool: "propose_story_order",
        any: &["story/beat-board", "story/outline"],
        also: &[],
    },
    ToolHome {
        tool: "propose_character",
        any: &["story/characters"],
        also: &[],
    },
    ToolHome {
        tool: "propose_update_character",
        any: &["story/characters"],
        also: &[],
    },
    ToolHome {
        tool: "propose_archive_character",
        any: &["story/characters"],
        also: &[],
    },
    ToolHome {
        tool: "propose_remove_character_image",
        any: &["story/characters"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_character",
        any: &["story/characters"],
        also: &[],
    },
    ToolHome {
        tool: "propose_relationship",
        any: &["story/characters"],
        also: &[],
    },
    ToolHome {
        tool: "propose_update_relationship",
        any: &["story/characters"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_relationship",
        any: &["story/characters"],
        also: &[],
    },
    ToolHome {
        tool: "propose_rename_character",
        any: &["story/characters"],
        also: &[],
    },
    ToolHome {
        tool: "propose_link_character_card",
        any: &["story/characters", "story/beat-board", "story/outline"],
        also: &[],
    },
    ToolHome {
        tool: "propose_story_day",
        any: &["story/timeline"],
        also: &[],
    },
    ToolHome {
        tool: "propose_time_note",
        any: &["story/timeline"],
        also: &[],
    },
    ToolHome {
        tool: "propose_season",
        any: &["story/episodes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_update_season",
        any: &["story/episodes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_season",
        any: &["story/episodes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_episode",
        any: &["story/episodes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_update_episode",
        any: &["story/episodes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_episode",
        any: &["story/episodes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_duplicate_episode",
        any: &["story/episodes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_screenplay",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_rename_screenplay",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_new_draft",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_rename_draft",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_current_draft",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_restore_draft",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_draft",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_lock_draft",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_revision",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_update_revision",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_restore_history_point",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_screenplay_lines",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_edit_screenplay_line",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_screenplay_line",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_new_scene",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_update_scene",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_move_scene",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_scene",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_replace_text",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_review_round",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_update_review_round",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_complete_review_round",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_title_page",
        any: &["screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_link_character_cue",
        any: &["screenplay/scenes", "story/characters"],
        also: &[],
    },
    ToolHome {
        tool: "propose_production_source",
        any: &["production/overview"],
        also: &[],
    },
    ToolHome {
        tool: "propose_production_update",
        any: &["production/overview"],
        also: &[],
    },
    ToolHome {
        tool: "propose_breakdown_suggestions",
        any: &["breakdown/breakdown"],
        also: &[],
    },
    ToolHome {
        tool: "propose_confirm_breakdown",
        any: &["breakdown/breakdown"],
        also: &[],
    },
    ToolHome {
        tool: "propose_reject_breakdown",
        any: &["breakdown/breakdown"],
        also: &[],
    },
    ToolHome {
        tool: "propose_breakdown_element",
        any: &["breakdown/breakdown"],
        also: &[],
    },
    ToolHome {
        tool: "propose_breakdown_note",
        any: &["breakdown/breakdown"],
        also: &[],
    },
    ToolHome {
        tool: "propose_remove_breakdown_element",
        any: &["breakdown/breakdown"],
        also: &[],
    },
    ToolHome {
        tool: "propose_archive_breakdown_element",
        any: &["breakdown/breakdown"],
        also: &[],
    },
    ToolHome {
        tool: "propose_breakdown_complete",
        any: &["breakdown/breakdown"],
        also: &[],
    },
    ToolHome {
        tool: "propose_breakdown_reviewed",
        any: &["breakdown/breakdown"],
        also: &[],
    },
    ToolHome {
        tool: "propose_refresh_breakdown_suggestions",
        any: &["breakdown/breakdown"],
        also: &[],
    },
    ToolHome {
        tool: "propose_catalog_item",
        any: &["production/catalog"],
        also: &[],
    },
    ToolHome {
        tool: "propose_update_catalog_item",
        any: &["production/catalog"],
        also: &[],
    },
    ToolHome {
        tool: "propose_catalog_alias",
        any: &["production/catalog"],
        also: &[],
    },
    ToolHome {
        tool: "propose_archive_catalog_item",
        any: &["production/catalog"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_catalog_item",
        any: &["production/catalog"],
        also: &[],
    },
    ToolHome {
        tool: "propose_replace_catalog_item",
        any: &["production/catalog"],
        also: &[],
    },
    ToolHome {
        tool: "propose_location",
        any: &["production/locations"],
        also: &[],
    },
    ToolHome {
        tool: "propose_update_location",
        any: &["production/locations"],
        also: &[],
    },
    ToolHome {
        tool: "propose_location_status",
        any: &["production/locations"],
        also: &[],
    },
    ToolHome {
        tool: "propose_remove_location_photo",
        any: &["production/locations"],
        also: &[],
    },
    ToolHome {
        tool: "propose_archive_location",
        any: &["production/locations"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_location",
        any: &["production/locations"],
        also: &[],
    },
    ToolHome {
        tool: "propose_replace_location",
        any: &["production/locations"],
        also: &[],
    },
    ToolHome {
        tool: "propose_cast_member",
        any: &["production/cast-crew"],
        also: &[],
    },
    ToolHome {
        tool: "propose_update_cast_member",
        any: &["production/cast-crew"],
        also: &[],
    },
    ToolHome {
        tool: "propose_assign_cast",
        any: &["production/cast-crew"],
        also: &[],
    },
    ToolHome {
        tool: "propose_crew_member",
        any: &["production/cast-crew"],
        also: &[],
    },
    ToolHome {
        tool: "propose_update_crew_member",
        any: &["production/cast-crew"],
        also: &[],
    },
    ToolHome {
        tool: "propose_archive_person",
        any: &["production/cast-crew"],
        also: &[],
    },
    ToolHome {
        tool: "propose_remove_person",
        any: &["production/cast-crew"],
        also: &[],
    },
    ToolHome {
        tool: "propose_schedule",
        any: &["production/schedule"],
        also: &[],
    },
    ToolHome {
        tool: "propose_schedule_settings",
        any: &["production/schedule"],
        also: &[],
    },
    ToolHome {
        tool: "propose_schedule_status",
        any: &["production/schedule"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_schedule",
        any: &["production/schedule"],
        also: &[],
    },
    ToolHome {
        tool: "propose_shooting_day",
        any: &["production/schedule"],
        also: &[],
    },
    ToolHome {
        tool: "propose_update_shooting_day",
        any: &["production/schedule"],
        also: &[],
    },
    ToolHome {
        tool: "propose_move_shooting_day",
        any: &["production/schedule"],
        also: &[],
    },
    ToolHome {
        tool: "propose_duplicate_shooting_day",
        any: &["production/schedule"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_shooting_day",
        any: &["production/schedule"],
        also: &[],
    },
    ToolHome {
        tool: "propose_schedule_scenes",
        any: &["production/schedule"],
        also: &[],
    },
    ToolHome {
        tool: "propose_scene_timing",
        any: &["production/schedule"],
        also: &[],
    },
    ToolHome {
        tool: "propose_schedule_break",
        any: &["production/schedule"],
        also: &[],
    },
    ToolHome {
        tool: "propose_update_schedule_break",
        any: &["production/schedule"],
        also: &[],
    },
    ToolHome {
        tool: "propose_move_schedule_break",
        any: &["production/schedule"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_schedule_break",
        any: &["production/schedule"],
        also: &[],
    },
    ToolHome {
        tool: "propose_reconcile_schedule",
        any: &["production/schedule"],
        also: &[],
    },
    ToolHome {
        tool: "propose_acknowledge_scene_change",
        any: &["production/schedule"],
        also: &[],
    },
    ToolHome {
        tool: "propose_confirm_scene_removal",
        any: &["production/schedule"],
        also: &[],
    },
    ToolHome {
        tool: "propose_call_sheet",
        any: &["callsheets/callsheets"],
        also: &[],
    },
    ToolHome {
        tool: "propose_update_call_sheet",
        any: &["callsheets/callsheets"],
        also: &[],
    },
    ToolHome {
        tool: "propose_call_sheet_section",
        any: &["callsheets/callsheets"],
        also: &[],
    },
    ToolHome {
        tool: "propose_refresh_call_sheet",
        any: &["callsheets/callsheets"],
        also: &[],
    },
    ToolHome {
        tool: "propose_finalize_call_sheet",
        any: &["callsheets/callsheets"],
        also: &[],
    },
    ToolHome {
        tool: "propose_call_sheet_revision",
        any: &["callsheets/callsheets"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_call_sheet",
        any: &["callsheets/callsheets"],
        also: &[],
    },
    ToolHome {
        tool: "propose_sides",
        any: &["production/sides"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_sides",
        any: &["production/sides"],
        also: &[],
    },
    ToolHome {
        tool: "propose_save_report",
        any: &["production/reports"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_report",
        any: &["production/reports"],
        also: &[],
    },
    ToolHome {
        tool: "propose_budget",
        any: &["production/budget"],
        also: &[],
    },
    ToolHome {
        tool: "propose_budget_settings",
        any: &["production/budget"],
        also: &[],
    },
    ToolHome {
        tool: "propose_budget_line",
        any: &["production/budget"],
        also: &[],
    },
    ToolHome {
        tool: "propose_update_budget_line",
        any: &["production/budget"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_budget_line",
        any: &["production/budget"],
        also: &[],
    },
    ToolHome {
        tool: "propose_budget_snapshot",
        any: &["production/budget"],
        also: &[],
    },
    ToolHome {
        tool: "propose_budget_reviewed",
        any: &["production/budget"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_budget_snapshot",
        any: &["production/budget"],
        also: &[],
    },
    ToolHome {
        tool: "propose_mark_planning_reviewed",
        any: &[
            "production/moodboards",
            "production/storyboards",
            "production/shots",
        ],
        also: &[],
    },
    ToolHome {
        tool: "propose_moodboard",
        any: &["production/moodboards"],
        also: &[],
    },
    ToolHome {
        tool: "propose_update_moodboard",
        any: &["production/moodboards"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_moodboard",
        any: &["production/moodboards"],
        also: &[],
    },
    ToolHome {
        tool: "propose_moodboard_vault_image",
        any: &["production/moodboards"],
        also: &[],
    },
    ToolHome {
        tool: "propose_moodboard_note",
        any: &["production/moodboards"],
        also: &[],
    },
    ToolHome {
        tool: "propose_moodboard_link",
        any: &["production/moodboards"],
        also: &[],
    },
    ToolHome {
        tool: "propose_update_moodboard_item",
        any: &["production/moodboards"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_moodboard_items",
        any: &["production/moodboards"],
        also: &[],
    },
    ToolHome {
        tool: "propose_storyboard",
        any: &["production/storyboards"],
        also: &[],
    },
    ToolHome {
        tool: "propose_update_storyboard",
        any: &["production/storyboards"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_storyboard",
        any: &["production/storyboards"],
        also: &[],
    },
    ToolHome {
        tool: "propose_storyboard_panel",
        any: &["production/storyboards"],
        also: &[],
    },
    ToolHome {
        tool: "propose_update_storyboard_panel",
        any: &["production/storyboards"],
        also: &[],
    },
    ToolHome {
        tool: "propose_move_storyboard_panel",
        any: &["production/storyboards"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_storyboard_panels",
        any: &["production/storyboards"],
        also: &[],
    },
    ToolHome {
        tool: "propose_link_panel_shot",
        any: &["production/storyboards", "production/shots"],
        also: &[],
    },
    ToolHome {
        tool: "propose_shot_from_panel",
        any: &["production/storyboards", "production/shots"],
        also: &[],
    },
    ToolHome {
        tool: "propose_panel_from_shot",
        any: &["production/storyboards", "production/shots"],
        also: &[],
    },
    ToolHome {
        tool: "propose_shot",
        any: &["production/shots"],
        also: &[],
    },
    ToolHome {
        tool: "propose_update_shot",
        any: &["production/shots"],
        also: &[],
    },
    ToolHome {
        tool: "propose_move_shot",
        any: &["production/shots"],
        also: &[],
    },
    ToolHome {
        tool: "propose_duplicate_shot",
        any: &["production/shots"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_shots",
        any: &["production/shots"],
        also: &[],
    },
    ToolHome {
        tool: "propose_copy_planning",
        any: &["production/shots"],
        also: &[],
    },
    ToolHome {
        tool: "propose_project_settings",
        any: &["help/settings"],
        also: &[],
    },
    ToolHome {
        tool: "propose_project_status",
        any: &["help/settings"],
        also: &[],
    },
    ToolHome {
        tool: "propose_restore_deleted",
        any: &["help/settings"],
        also: &[],
    },
    ToolHome {
        tool: "propose_project_template",
        any: &["help/settings"],
        also: &[],
    },
    ToolHome {
        tool: "propose_rename_template",
        any: &["help/settings"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_template",
        any: &["help/settings"],
        also: &[],
    },
    ToolHome {
        tool: "propose_folder",
        any: &["files/files"],
        also: &[],
    },
    ToolHome {
        tool: "propose_rename_file",
        any: &["files/files"],
        also: &[],
    },
    ToolHome {
        tool: "propose_move_file",
        any: &["files/files"],
        also: &[],
    },
    ToolHome {
        tool: "propose_file_notes",
        any: &["files/files"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_file",
        any: &["files/files"],
        also: &[],
    },
    ToolHome {
        tool: "propose_rename_folder",
        any: &["files/files"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_folder",
        any: &["files/files"],
        also: &[],
    },
    ToolHome {
        tool: "propose_project_note",
        any: &["notes/notes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_update_project_note",
        any: &["notes/notes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_pin_project_note",
        any: &["notes/notes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_project_note",
        any: &["notes/notes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_task",
        any: &["notes/tasks"],
        also: &[],
    },
    ToolHome {
        tool: "propose_update_task",
        any: &["notes/tasks"],
        also: &[],
    },
    ToolHome {
        tool: "propose_task_done",
        any: &["notes/tasks"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_task",
        any: &["notes/tasks"],
        also: &[],
    },
    ToolHome {
        tool: "propose_comment",
        any: &["notes/notes", "screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_comment_reply",
        any: &["notes/notes", "screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_update_comment",
        any: &["notes/notes", "screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_resolve_comment",
        any: &["notes/notes", "screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_comment",
        any: &["notes/notes", "screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_private_note",
        any: &["notes/notes", "screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_update_private_note",
        any: &["notes/notes", "screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_private_note",
        any: &["notes/notes", "screenplay/scenes"],
        also: &[],
    },
    ToolHome {
        tool: "propose_vault_item",
        any: &["vault/project"],
        also: &[],
    },
    ToolHome {
        tool: "propose_update_vault_item",
        any: &["vault/project"],
        also: &[],
    },
    ToolHome {
        tool: "propose_organize_vault_items",
        any: &["vault/project"],
        also: &[],
    },
    ToolHome {
        tool: "propose_delete_vault_items",
        any: &["vault/project"],
        also: &[],
    },
    ToolHome {
        tool: "propose_vault_folder",
        any: &["vault/project"],
        also: &[],
    },
    ToolHome {
        tool: "propose_vault_collection",
        any: &["vault/project"],
        also: &[],
    },
    ToolHome {
        tool: "propose_send_vault_to_story",
        any: &["vault/project"],
        also: &[],
    },
];

pub fn home(tool: &str) -> Option<&'static ToolHome> {
    TOOL_HOMES.iter().find(|h| h.tool == tool)
}

/// The section to add before this tool can run, when one is missing. An unknown tool has none: the
/// toolbox refuses it by itself.
pub fn missing_leaf(tool: &str, picks: &[&str]) -> Option<&'static str> {
    let h = home(tool)?;
    if !h.any.is_empty() && !h.any.iter().any(|l| picks.contains(l)) {
        return h.any.first().copied();
    }
    h.also.iter().find(|l| !picks.contains(l)).copied()
}

/// May this tool run with these picks? With no picks nothing is held back: that is the path the
/// evaluation measures the older sets on (`AppConfig::ai_eval_any_sections`).
pub fn offered(tool: &str, picks: &[&str]) -> bool {
    picks.is_empty() || (home(tool).is_some() && missing_leaf(tool, picks).is_none())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::modules::ai::sections::{LEAVES, leaf};
    use crate::modules::ai::toolbox::all_tool_names;

    #[test]
    fn every_tool_has_one_home_and_every_home_is_a_real_section() {
        let names: BTreeSet<&str> = all_tool_names().into_iter().collect();
        assert!(names.len() > 200, "{}", names.len());
        let mut seen = BTreeSet::new();
        for h in TOOL_HOMES {
            assert!(names.contains(h.tool), "{} is not a tool", h.tool);
            assert!(seen.insert(h.tool), "{} has two rows", h.tool);
            for l in h.any.iter().chain(h.also) {
                assert!(leaf(l).is_some(), "{}: no section {l}", h.tool);
            }
        }
        for n in &names {
            assert!(seen.contains(n), "{n} has no home");
        }
    }

    #[test]
    fn only_the_general_tools_live_everywhere() {
        let every: BTreeSet<&str> = TOOL_HOMES
            .iter()
            .filter(|h| h.any.is_empty())
            .map(|h| h.tool)
            .collect();
        assert_eq!(
            every,
            BTreeSet::from([
                "open_object",
                "open_workspace",
                "retrieve_context",
                "search_project"
            ])
        );
    }

    #[test]
    fn every_section_but_the_story_page_and_the_guide_has_a_tool_of_its_own() {
        for l in LEAVES {
            let own = TOOL_HOMES.iter().any(|h| h.any.contains(&l.id));
            let on_purpose = matches!(l.id, "story/page" | "help/guide");
            assert_eq!(own, !on_purpose, "{}", l.id);
        }
    }

    #[test]
    fn a_tool_is_offered_only_with_its_section_and_what_it_also_needs() {
        assert!(offered("propose_budget_line", &["production/budget"]));
        assert!(!offered("propose_budget_line", &["notes/tasks"]));
        assert!(offered("search_project", &["notes/tasks"]));
        assert_eq!(
            missing_leaf("propose_task", &["production/budget"]),
            Some("notes/tasks")
        );
        assert!(offered(
            "suggest_breakdown",
            &["breakdown/breakdown", "screenplay/scenes"]
        ));
        assert_eq!(
            missing_leaf("suggest_breakdown", &["breakdown/breakdown"]),
            Some("screenplay/scenes")
        );
        assert_eq!(
            missing_leaf("suggest_breakdown", &["screenplay/scenes"]),
            Some("breakdown/breakdown")
        );
        assert!(
            offered("propose_budget_line", &[]),
            "no picks: the older path"
        );
    }
}
