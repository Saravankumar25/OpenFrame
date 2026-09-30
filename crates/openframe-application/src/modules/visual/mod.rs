//! Visual planning: Moodboards, Storyboards and Shot Lists
//! (FSD §31–34, §55, §101–103; FSD-SHOT-001, FSD-STB-001, FSD-PROD-011..020).
//!
//! The three tools are independent (FSD §34): a filmmaker can go
//! script → shots → storyboard, script → storyboard → shots, or shots only.
//! Shots and scene storyboards follow screenplay scene IDENTITY (lineage) in the
//! active Production Source draft (or the current draft before production
//! starts); script changes flag planning for review and never delete it (§55).
//! Exports are provided by the shared report engine (not in this module).

pub mod common;
pub mod moodboard;
pub mod scenes;
pub mod shots;
pub mod storyboard;

use crate::registry::{Registry, TrashHandler};

pub use scenes::flag_scene_for_review;

pub fn register(r: &mut Registry) {
    // Scenes available for planning + review state.
    r.query("visual.scenes", scenes::list_scenes);
    r.query("visual.scene_characters", scenes::scene_characters);
    r.command("visual.mark_scene_reviewed", scenes::mark_scene_reviewed);
    r.query("visual.vault_images", moodboard::vault_images);

    // Moodboards.
    r.query("moodboard.list", moodboard::list);
    r.query("moodboard.suggestions", moodboard::suggestions);
    r.query("moodboard.get", moodboard::get);
    r.command("moodboard.create", moodboard::create);
    r.command("moodboard.rename", moodboard::rename);
    r.command("moodboard.update_notes", moodboard::update_notes);
    r.command("moodboard.reorder", moodboard::reorder);
    r.command("moodboard.delete", moodboard::delete);
    r.command("moodboard.add_images", moodboard::add_images);
    r.command("moodboard.add_image_data", moodboard::add_image_data);
    r.command("moodboard.add_vault_image", moodboard::add_vault_image);
    r.command("moodboard.add_note", moodboard::add_note);
    r.command("moodboard.add_link", moodboard::add_link);
    r.command("moodboard.update_item", moodboard::update_item);
    r.command("moodboard.move_items", moodboard::move_items);
    r.command("moodboard.resize_item", moodboard::resize_item);
    r.command("moodboard.arrange_item", moodboard::arrange_item);
    r.command("moodboard.delete_items", moodboard::delete_items);

    // Storyboards.
    r.query("storyboard.list", storyboard::list);
    r.query("storyboard.get", storyboard::get);
    r.command("storyboard.create", storyboard::create);
    r.command("storyboard.rename", storyboard::rename);
    r.command("storyboard.set_scene", storyboard::set_scene);
    r.command("storyboard.delete", storyboard::delete);
    r.command("storyboard.add_panel", storyboard::add_panel);
    r.command("storyboard.update_panel", storyboard::update_panel);
    r.command("storyboard.set_panel_visual", storyboard::set_panel_visual);
    r.command("storyboard.reorder_panel", storyboard::reorder_panel);
    r.command("storyboard.move_panel", storyboard::move_panel);
    r.command("storyboard.delete_panels", storyboard::delete_panels);
    r.command("storyboard.link_shot", storyboard::link_shot);
    r.command(
        "storyboard.create_shot_from_panel",
        storyboard::create_shot_from_panel,
    );

    // Shot lists.
    r.query("shot.list", shots::list);
    r.query("shot.get", shots::get);
    r.command("shot.create", shots::create);
    r.command("shot.update", shots::update);
    r.command("shot.reorder", shots::reorder);
    r.command("shot.move", shots::move_to_scene);
    r.command("shot.duplicate", shots::duplicate);
    r.command("shot.delete", shots::delete);
    r.command("shot.set_reference_image", shots::set_reference_image);
    r.command("shot.attach_panel", shots::attach_panel);
    r.command("shot.detach_panel", shots::detach_panel);
    r.command("shot.create_panel", shots::create_panel);
    r.command("shot.copy_planning", shots::copy_planning);

    // Search.
    r.indexer("moodboard", moodboard::index_moodboard);
    r.indexer("moodboard_item", moodboard::index_item);
    r.indexer("storyboard", storyboard::index_storyboard);
    r.indexer("storyboard_panel", storyboard::index_panel);
    r.indexer("shot", shots::index_shot);

    // Recently Deleted.
    r.trash_handler(TrashHandler {
        object_type: "moodboard",
        table: "moodboard",
        label: "Moodboard",
        restore: Some(moodboard::restore_moodboard),
        purge: moodboard::purge_moodboard,
    });
    r.trash_handler(TrashHandler {
        object_type: "moodboard_item",
        table: "moodboard_item",
        label: "Moodboard item",
        restore: Some(moodboard::restore_item),
        purge: moodboard::purge_item,
    });
    r.trash_handler(TrashHandler {
        object_type: "storyboard",
        table: "storyboard",
        label: "Storyboard",
        restore: Some(storyboard::restore_storyboard),
        purge: storyboard::purge_storyboard,
    });
    r.trash_handler(TrashHandler {
        object_type: "storyboard_panel",
        table: "storyboard_panel",
        label: "Storyboard panel",
        restore: Some(storyboard::restore_panel),
        purge: storyboard::purge_panel,
    });
    r.trash_handler(TrashHandler {
        object_type: "shot",
        table: "shot",
        label: "Shot",
        restore: Some(shots::restore_shot),
        purge: shots::purge_shot,
    });
}
