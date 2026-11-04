//! Which leaf each agent tool is homed in. The read tools are listed on their leaf in the registry
//! (`Leaf::tools`); the proposal tools, about two hundred, are listed here by what they change, with
//! the `propose_` left off. Round R6 offers the agent only the tools homed in the picked leaves.
//!
//! A tool belongs to the screen where a person would do the same thing by hand.

use super::{CROSS_TOOLS, LEAVES, LOOP_TOOLS, Leaf, SectionPick, UNSECTIONED_TOOLS};

/// `(leaf, proposals)`; every name is `propose_<name>`.
pub const PROPOSALS: &[(&str, &[&str])] = &[
    (
        "idea-vault/this-project",
        &[
            "vault_item",
            "update_vault_item",
            "delete_vault_items",
            "vault_folder",
            "vault_collection",
            "organize_vault_items",
            "send_vault_to_story",
        ],
    ),
    (
        "story/beat-board",
        &[
            "scene_card",
            "update_scene_card",
            "act",
            "update_act",
            "delete_act",
            "beat",
            "update_beat",
            "convert_beat",
            "convert_card_to_beat",
            "sequence",
            "update_sequence",
            "delete_sequence",
            "story_order",
            "delete_story_items",
            "duplicate_story_items",
            "move_story_items",
            "park_story_items",
            "unpark_story_items",
            "remove_story_attachment",
        ],
    ),
    (
        "story/characters",
        &[
            "character",
            "update_character",
            "archive_character",
            "delete_character",
            "rename_character",
            "relationship",
            "update_relationship",
            "delete_relationship",
            "link_character_card",
            "link_character_cue",
            "remove_character_image",
        ],
    ),
    ("story/timeline", &["story_day", "time_note"]),
    (
        "story/episodes",
        &[
            "episode",
            "update_episode",
            "delete_episode",
            "duplicate_episode",
            "season",
            "update_season",
            "delete_season",
        ],
    ),
    (
        "screenplay/scenes",
        &[
            "screenplay",
            "rename_screenplay",
            "build_screenplay",
            "new_scene",
            "update_scene",
            "delete_scene",
            "move_scene",
            "scene_timing",
            "replace_text",
            "edit_screenplay_line",
            "screenplay_lines",
            "delete_screenplay_line",
            "new_draft",
            "rename_draft",
            "delete_draft",
            "current_draft",
            "lock_draft",
            "restore_draft",
            "restore_history_point",
            "title_page",
            "revision",
            "update_revision",
            "comment",
            "comment_reply",
            "resolve_comment",
            "update_comment",
            "delete_comment",
            "review_round",
            "update_review_round",
            "complete_review_round",
            "acknowledge_scene_change",
            "confirm_scene_removal",
        ],
    ),
    (
        "breakdown/breakdown",
        &[
            "breakdown_element",
            "remove_breakdown_element",
            "archive_breakdown_element",
            "breakdown_note",
            "breakdown_reviewed",
            "breakdown_complete",
            "breakdown_suggestions",
            "refresh_breakdown_suggestions",
            "confirm_breakdown",
            "reject_breakdown",
            "copy_planning",
            "mark_planning_reviewed",
        ],
    ),
    (
        "production/overview",
        &["production_source", "production_update"],
    ),
    (
        "production/catalog",
        &[
            "catalog_item",
            "update_catalog_item",
            "archive_catalog_item",
            "delete_catalog_item",
            "replace_catalog_item",
            "catalog_alias",
        ],
    ),
    (
        "production/locations",
        &[
            "location",
            "update_location",
            "archive_location",
            "delete_location",
            "replace_location",
            "location_status",
            "remove_location_photo",
        ],
    ),
    (
        "production/cast-crew",
        &[
            "cast_member",
            "update_cast_member",
            "crew_member",
            "update_crew_member",
            "assign_cast",
            "archive_person",
            "remove_person",
        ],
    ),
    (
        "production/moodboards",
        &[
            "moodboard",
            "update_moodboard",
            "delete_moodboard",
            "moodboard_link",
            "moodboard_note",
            "moodboard_vault_image",
            "update_moodboard_item",
            "delete_moodboard_items",
        ],
    ),
    (
        "production/storyboards",
        &[
            "storyboard",
            "update_storyboard",
            "delete_storyboard",
            "storyboard_panel",
            "update_storyboard_panel",
            "delete_storyboard_panels",
            "move_storyboard_panel",
            "panel_from_shot",
            "link_panel_shot",
        ],
    ),
    (
        "production/shot-lists",
        &[
            "shot",
            "update_shot",
            "delete_shots",
            "duplicate_shot",
            "move_shot",
            "shot_from_panel",
        ],
    ),
    (
        "production/schedule",
        &[
            "schedule",
            "delete_schedule",
            "schedule_scenes",
            "schedule_settings",
            "schedule_status",
            "shooting_day",
            "update_shooting_day",
            "delete_shooting_day",
            "move_shooting_day",
            "duplicate_shooting_day",
            "schedule_break",
            "update_schedule_break",
            "delete_schedule_break",
            "move_schedule_break",
            "reconcile_schedule",
        ],
    ),
    ("production/sides", &["sides", "delete_sides"]),
    ("production/reports", &["save_report", "delete_report"]),
    (
        "production/budget",
        &[
            "budget",
            "budget_line",
            "update_budget_line",
            "delete_budget_line",
            "budget_reviewed",
            "budget_settings",
            "budget_snapshot",
            "delete_budget_snapshot",
        ],
    ),
    (
        "call-sheets/call-sheets",
        &[
            "call_sheet",
            "update_call_sheet",
            "delete_call_sheet",
            "call_sheet_revision",
            "call_sheet_section",
            "finalize_call_sheet",
            "refresh_call_sheet",
        ],
    ),
    (
        "files/files",
        &[
            "folder",
            "rename_folder",
            "delete_folder",
            "rename_file",
            "move_file",
            "delete_file",
            "file_notes",
        ],
    ),
    (
        "notes-tasks/notes",
        &[
            "project_note",
            "update_project_note",
            "delete_project_note",
            "pin_project_note",
            "private_note",
            "update_private_note",
            "delete_private_note",
        ],
    ),
    (
        "notes-tasks/tasks",
        &["task", "update_task", "delete_task", "task_done"],
    ),
    (
        "help/settings",
        &[
            "project_settings",
            "project_status",
            "project_template",
            "rename_template",
            "delete_template",
        ],
    ),
];

/// A proposal that belongs to a page which is no section (Recently Deleted).
pub const UNSECTIONED_PROPOSALS: &[&str] = &["propose_restore_deleted"];

/// The leaf a proposal tool (`propose_...`) is homed in.
fn proposal_home(tool: &str) -> Option<&'static Leaf> {
    let name = tool.strip_prefix("propose_")?;
    let (leaf, _) = PROPOSALS.iter().find(|(_, names)| names.contains(&name))?;
    super::leaf(leaf)
}

/// The leaf a tool is homed in: the read tools by the leaf's own list, the proposals by this table.
pub fn leaf_of_tool(tool: &str) -> Option<&'static Leaf> {
    LEAVES
        .iter()
        .find(|l| l.tools.contains(&tool))
        .or_else(|| proposal_home(tool))
}

/// The leaf a tool is homed in by explicit listing, counted over both tables. A tool listed twice
/// has more than one home, and the coverage test refuses it.
pub fn homes_of_tool(tool: &str) -> usize {
    let reads = LEAVES.iter().filter(|l| l.tools.contains(&tool)).count();
    let proposals = tool.strip_prefix("propose_").map_or(0, |name| {
        PROPOSALS.iter().filter(|(_, n)| n.contains(&name)).count()
    });
    reads + proposals
}

/// Every tool homed in a leaf: its own list and its proposals.
pub fn tools_of(leaf: &Leaf) -> Vec<String> {
    let mut out: Vec<String> = leaf.tools.iter().map(|t| (*t).to_string()).collect();
    for (id, names) in PROPOSALS {
        if *id == leaf.id {
            out.extend(names.iter().map(|n| format!("propose_{n}")));
        }
    }
    out
}

// ------------------------------------------------------------------ what the agent may run (R6)

/// Tools that need more than their own leaf. A breakdown suggestion reads the scene text, which
/// lives in the Screenplay (spec 3.9).
pub const NEEDS: &[(&str, &[&str])] = &[(
    "suggest_breakdown",
    &["breakdown/breakdown", "screenplay/scenes"],
)];

/// Whether the agent may offer or run a tool under the sections the person picked.
#[derive(Clone, Copy)]
pub enum Gate {
    /// The tool is homed in a picked leaf (or works across the picks).
    Open,
    /// The tool, or what it needs, lives in this leaf, which was not picked: the pointer names it.
    Needs(&'static Leaf),
    /// No leaf to name: a page that is no section (Activity, Recently Deleted), or a tool nobody homes.
    Closed,
}

/// The gate of one tool. A scope built without picks (an internal call) leaves every tool open.
pub fn gate_tool(picks: &[SectionPick], tool: &str) -> Gate {
    if picks.is_empty() || LOOP_TOOLS.contains(&tool) || CROSS_TOOLS.contains(&tool) {
        return Gate::Open;
    }
    let picked = |id: &str| picks.iter().any(|p| p.leaf == id);
    if let Some((_, needs)) = NEEDS.iter().find(|(t, _)| *t == tool)
        && let Some(missing) = needs.iter().find(|n| !picked(n))
    {
        return super::leaf(missing).map_or(Gate::Closed, Gate::Needs);
    }
    if UNSECTIONED_TOOLS.contains(&tool) {
        return Gate::Closed;
    }
    match leaf_of_tool(tool) {
        Some(l) if picked(l.id) => Gate::Open,
        Some(l) => Gate::Needs(l),
        None => Gate::Closed,
    }
}

/// The search entity types the picked leaves home, for the tools that search across the picks
/// (`search_project`, `retrieve_context`). `None` when nothing was picked: nothing narrows.
pub fn picked_entity_types(picks: &[SectionPick]) -> Option<Vec<&'static str>> {
    if picks.is_empty() {
        return None;
    }
    let mut types: Vec<&'static str> = LEAVES
        .iter()
        .filter(|l| picks.iter().any(|p| p.leaf == l.id))
        .flat_map(|l| l.entity_types.iter().copied())
        .collect();
    types.sort_unstable();
    types.dedup();
    Some(types)
}
