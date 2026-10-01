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
    use crate::registry::{FsEffect as Fs, OperationMetadata as M, hidden as h};
    r.module("Visual Planning");
    // Scenes available for planning + review state.
    r.query("visual.scenes", scenes::list_scenes).meta(M::compute("Scenes available for visual planning with shot/storyboard/moodboard counts and review flags."));
    r.query("visual.scene_characters", scenes::scene_characters)
        .meta(M::read(
            "Characters who appear in a scene (for shot character lists).",
        ));
    r.command("visual.mark_scene_reviewed", scenes::mark_scene_reviewed)
        .meta(M::edit(
            "Mark a scene's visual planning as reviewed after a script change.",
        ));
    r.query("visual.vault_images", moodboard::vault_images)
        .meta(
        M::read(
            "Idea Vault images available to place on a moodboard (project or Global Idea Vault).",
        )
        .hidden(h::UI_FLOW),
    );

    // Moodboards.
    r.query("moodboard.list", moodboard::list)
        .meta(M::read("Moodboards with their scene and item counts."));
    r.query("moodboard.suggestions", moodboard::suggestions)
        .meta(M::read("Scenes suggested for new moodboards (dialog helper).").hidden(h::UI_FLOW));
    r.query("moodboard.get", moodboard::get)
        .meta(M::read("One moodboard with its images, notes and links."));
    r.command("moodboard.create", moodboard::create)
        .meta(M::edit("Create a moodboard (optionally for a scene)."));
    r.command("moodboard.rename", moodboard::rename)
        .meta(M::edit("Rename a moodboard."));
    r.command("moodboard.update_notes", moodboard::update_notes)
        .meta(M::edit("Edit a moodboard's notes."));
    r.command("moodboard.reorder", moodboard::reorder)
        .meta(M::edit("Change a moodboard's position in the list.").hidden(h::LAYOUT));
    r.command("moodboard.delete", moodboard::delete)
        .meta(M::soft_delete("Move a moodboard to Recently Deleted.").confirm());
    r.command("moodboard.add_images", moodboard::add_images)
        .meta(
            M::edit("Add image files the user picked to a moodboard.")
                .fs(Fs::ReadsUserFile)
                .hidden(h::MEDIA_INPUT),
        );
    r.command("moodboard.add_image_data", moodboard::add_image_data)
        .meta(M::edit("Add a pasted image to a moodboard.").hidden(h::MEDIA_INPUT));
    r.command("moodboard.add_vault_image", moodboard::add_vault_image)
        .meta(M::edit("Place an Idea Vault image on a moodboard."));
    r.command("moodboard.add_note", moodboard::add_note)
        .meta(M::edit("Add a text note to a moodboard."));
    r.command("moodboard.add_link", moodboard::add_link)
        .meta(M::edit("Add a web link to a moodboard."));
    r.command("moodboard.update_item", moodboard::update_item)
        .meta(M::edit(
            "Edit a moodboard item's caption, note text or link.",
        ));
    r.command("moodboard.move_items", moodboard::move_items)
        .meta(M::edit("Drag moodboard items to new canvas positions.").hidden(h::LAYOUT));
    r.command("moodboard.resize_item", moodboard::resize_item)
        .meta(M::edit("Resize a moodboard item on the canvas.").hidden(h::LAYOUT));
    r.command("moodboard.arrange_item", moodboard::arrange_item)
        .meta(M::edit("Bring a moodboard item to the front or send it back.").hidden(h::LAYOUT));
    r.command("moodboard.delete_items", moodboard::delete_items)
        .meta(M::soft_delete("Move moodboard items to Recently Deleted."));

    // Storyboards.
    r.query("storyboard.list", storyboard::list)
        .meta(M::read("Storyboards with their scene and panel counts."));
    r.query("storyboard.get", storyboard::get)
        .meta(M::read("One storyboard with its panels."));
    r.command("storyboard.create", storyboard::create)
        .meta(M::edit("Create a storyboard (for a scene or standalone)."));
    r.command("storyboard.rename", storyboard::rename)
        .meta(M::edit("Rename a storyboard."));
    r.command("storyboard.set_scene", storyboard::set_scene)
        .meta(M::edit(
            "Associate a storyboard with a scene, or make it standalone.",
        ));
    r.command("storyboard.delete", storyboard::delete)
        .meta(M::soft_delete("Move a storyboard to Recently Deleted.").confirm());
    r.command("storyboard.add_panel", storyboard::add_panel)
        .meta(
            M::edit("Add a storyboard panel (empty, imported image or sketch).")
                .fs(Fs::ReadsUserFile),
        );
    r.command("storyboard.update_panel", storyboard::update_panel)
        .meta(M::edit(
            "Edit a panel's description, framing, movement, angle, sound, duration or note.",
        ));
    r.command("storyboard.set_panel_visual", storyboard::set_panel_visual)
        .meta(
            M::edit("Replace or clear a panel's image or sketch.")
                .fs(Fs::ReadsUserFile)
                .hidden(h::MEDIA_INPUT),
        );
    r.command("storyboard.reorder_panel", storyboard::reorder_panel)
        .meta(M::edit(
            "Move a panel to another position in its storyboard.",
        ));
    r.command("storyboard.move_panel", storyboard::move_panel)
        .meta(M::edit("Move a panel to another storyboard."));
    r.command("storyboard.delete_panels", storyboard::delete_panels)
        .meta(M::soft_delete(
            "Move storyboard panels to Recently Deleted.",
        ));
    r.command("storyboard.link_shot", storyboard::link_shot)
        .meta(M::edit(
            "Link a storyboard panel to a shot, or remove the link.",
        ));
    r.command(
        "storyboard.create_shot_from_panel",
        storyboard::create_shot_from_panel,
    )
    .meta(M::edit("Create a shot from a storyboard panel."));

    // Shot lists.
    r.query("shot.list", shots::list)
        .meta(M::read("Shot list of a scene (or all scenes)."));
    r.query("shot.get", shots::get)
        .meta(M::read("One shot with its camera details."));
    r.command("shot.create", shots::create)
        .meta(M::edit("Add a shot to a scene's shot list."));
    r.command("shot.update", shots::update).meta(M::edit(
        "Edit a shot (description, size, movement, angle, lens, notes, characters, sound).",
    ));
    r.command("shot.reorder", shots::reorder)
        .meta(M::edit("Move a shot to another position in its scene."));
    r.command("shot.move", shots::move_to_scene)
        .meta(M::edit("Move a shot to another scene."));
    r.command("shot.duplicate", shots::duplicate)
        .meta(M::edit("Duplicate a shot."));
    r.command("shot.delete", shots::delete)
        .meta(M::soft_delete("Move shots to Recently Deleted."));
    r.command("shot.set_reference_image", shots::set_reference_image)
        .meta(
            M::edit("Set or remove a shot's reference image.")
                .fs(Fs::ReadsUserFile)
                .hidden(h::MEDIA_INPUT),
        );
    r.command("shot.attach_panel", shots::attach_panel)
        .meta(M::edit("Attach a storyboard panel to a shot.").hidden(h::DUPLICATE));
    r.command("shot.detach_panel", shots::detach_panel)
        .meta(M::edit("Detach a storyboard panel from a shot.").hidden(h::DUPLICATE));
    r.command("shot.create_panel", shots::create_panel)
        .meta(M::edit("Create a storyboard panel for a shot."));
    r.command("shot.copy_planning", shots::copy_planning)
        .meta(M::edit(
            "Copy shots and storyboards from one scene to another.",
        ));

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
