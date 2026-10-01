//! Proposal tools for project-wide workspaces: project settings, Recently
//! Deleted, Project Files, Notes & Tasks, project templates, comments, the
//! user's own Private Notes and the project Idea Vault.

use openframe_domain::enums::VaultItemType;
use openframe_domain::{AppError, AppResult, Capability};
use serde_json::{Value, json};

use super::super::queries;
use super::super::toolbox::resolve::{self as rv, Found};
use super::super::toolbox::schema as sc;
use super::super::types::ChangeSetDraft;
use super::kit::{A, Draft, need_change, plural, set_text};
use super::{PropCtx, ProposalSpec};

macro_rules! spec {
    ($tool:literal, $desc:literal, [$($req:literal),*], [$($opt:literal),*], $cap:ident, $phrase:literal, $module:literal, $schema:ident, $build:ident) => {
        ProposalSpec {
            tool: $tool,
            description: $desc,
            required_ops: &[$($req),*],
            optional_ops: &[$($opt),*],
            cap: Capability::$cap,
            action_phrase: $phrase,
            module: $module,
            schema: $schema,
            build: $build,
        }
    };
}
pub(super) use spec;

pub const SPECS: &[ProposalSpec] = &[
    spec!(
        "propose_project_settings",
        "Prepare changes to the project's title, logline, genre, language or creator.",
        ["project.update_settings"],
        [],
        ManageProject,
        "changing project settings",
        "Project",
        s_settings,
        settings
    ),
    spec!(
        "propose_restore_deleted",
        "Prepare restoring an item from Recently Deleted.",
        ["trash.restore"],
        [],
        SoftDelete,
        "restoring deleted items",
        "Recently Deleted",
        s_restore,
        restore
    ),
    spec!(
        "propose_move_file",
        "Prepare moving a project file into a folder (or to the top level).",
        ["files.move"],
        [],
        Edit,
        "changing Project Files",
        "Files",
        s_move_file,
        move_file
    ),
    spec!(
        "propose_file_notes",
        "Prepare new notes for a project file.",
        ["files.update_notes"],
        [],
        Edit,
        "changing Project Files",
        "Files",
        s_file_notes,
        file_notes
    ),
    spec!(
        "propose_delete_file",
        "Prepare moving a project file to Recently Deleted.",
        ["files.delete"],
        [],
        SoftDelete,
        "deleting files",
        "Files",
        s_file,
        delete_file
    ),
    spec!(
        "propose_rename_folder",
        "Prepare renaming a Project Files folder.",
        ["files.rename_folder"],
        [],
        Edit,
        "changing Project Files",
        "Files",
        s_rename_folder,
        rename_folder
    ),
    spec!(
        "propose_delete_folder",
        "Prepare deleting a Project Files folder (its files move to the top level).",
        ["files.delete_folder"],
        [],
        SoftDelete,
        "deleting folders",
        "Files",
        s_folder,
        delete_folder
    ),
    spec!(
        "propose_update_project_note",
        "Prepare edits to a Project Note's title or text.",
        ["notes.update"],
        [],
        Edit,
        "editing Project Notes",
        "Notes & Tasks",
        s_update_note,
        update_note
    ),
    spec!(
        "propose_pin_project_note",
        "Prepare pinning or unpinning a Project Note.",
        ["notes.set_pinned"],
        [],
        Edit,
        "editing Project Notes",
        "Notes & Tasks",
        s_pin_note,
        pin_note
    ),
    spec!(
        "propose_delete_project_note",
        "Prepare moving a Project Note to Recently Deleted.",
        ["notes.delete"],
        [],
        SoftDelete,
        "deleting Project Notes",
        "Notes & Tasks",
        s_note,
        delete_note
    ),
    spec!(
        "propose_update_task",
        "Prepare edits to a task (title, notes, due date).",
        ["tasks.update"],
        [],
        Edit,
        "editing tasks",
        "Notes & Tasks",
        s_update_task,
        update_task
    ),
    spec!(
        "propose_task_done",
        "Prepare marking a task done (or open again).",
        ["tasks.set_done"],
        [],
        Edit,
        "editing tasks",
        "Notes & Tasks",
        s_task_done,
        task_done
    ),
    spec!(
        "propose_delete_task",
        "Prepare moving a task to Recently Deleted.",
        ["tasks.delete"],
        [],
        SoftDelete,
        "deleting tasks",
        "Notes & Tasks",
        s_task,
        delete_task
    ),
    spec!(
        "propose_project_template",
        "Prepare a new, empty project template of a given type.",
        ["templates.create"],
        [],
        Edit,
        "creating templates",
        "Templates",
        s_template_new,
        template_new
    ),
    spec!(
        "propose_rename_template",
        "Prepare renaming a project template.",
        ["templates.update"],
        [],
        Edit,
        "editing templates",
        "Templates",
        s_template_rename,
        template_rename
    ),
    spec!(
        "propose_delete_template",
        "Prepare deleting a project template.",
        ["templates.delete"],
        [],
        SoftDelete,
        "deleting templates",
        "Templates",
        s_template,
        template_delete
    ),
    spec!(
        "propose_comment",
        "Prepare a comment on a screenplay scene, Scene Card, character, location or catalog item.",
        ["comment.create"],
        [],
        Comment,
        "adding comments",
        "Comments",
        s_comment,
        comment
    ),
    spec!(
        "propose_comment_reply",
        "Prepare a reply to a comment.",
        ["comment.reply"],
        [],
        Comment,
        "adding comments",
        "Comments",
        s_reply,
        reply
    ),
    spec!(
        "propose_update_comment",
        "Prepare edits to a comment's text or discussion status.",
        ["comment.update"],
        [],
        Comment,
        "editing comments",
        "Comments",
        s_update_comment,
        update_comment
    ),
    spec!(
        "propose_resolve_comment",
        "Prepare resolving a comment thread (or reopening it).",
        ["comment.resolve", "comment.reopen"],
        [],
        ResolveComments,
        "resolving comments",
        "Comments",
        s_resolve_comment,
        resolve_comment
    ),
    spec!(
        "propose_delete_comment",
        "Prepare deleting a comment.",
        ["comment.delete"],
        [],
        Comment,
        "deleting comments",
        "Comments",
        s_comment_ref,
        delete_comment
    ),
    spec!(
        "propose_private_note",
        "Prepare one of your own Private Notes (only you can see it), optionally about a scene.",
        ["private_note.create"],
        [],
        View,
        "creating Private Notes",
        "Private Notes",
        s_private_note,
        private_note
    ),
    spec!(
        "propose_update_private_note",
        "Prepare new text for one of your own Private Notes.",
        ["private_note.update"],
        [],
        View,
        "editing Private Notes",
        "Private Notes",
        s_private_update,
        private_update
    ),
    spec!(
        "propose_delete_private_note",
        "Prepare deleting one of your own Private Notes (recoverable).",
        ["private_note.delete"],
        [],
        View,
        "deleting Private Notes",
        "Private Notes",
        s_private_ref,
        private_delete
    ),
    spec!(
        "propose_vault_item",
        "Prepare a new Idea Vault note, quote or link (optionally tagged, in a folder or collection).",
        ["vault.create"],
        [],
        Edit,
        "adding to the Idea Vault",
        "Idea Vault",
        s_vault_item,
        vault_item
    ),
    spec!(
        "propose_update_vault_item",
        "Prepare edits to an Idea Vault item (title, text, caption, link, source).",
        ["vault.update"],
        [],
        Edit,
        "editing the Idea Vault",
        "Idea Vault",
        s_vault_update,
        vault_update
    ),
    spec!(
        "propose_organize_vault_items",
        "Prepare organizing Idea Vault items: pin/unpin, move to a folder, add to/remove from a collection, add/remove tags.",
        [
            "vault.set_pinned",
            "vault.move_to_folder",
            "vault.add_to_collection",
            "vault.remove_from_collection",
            "vault.add_tags",
            "vault.remove_tag"
        ],
        [],
        Edit,
        "organizing the Idea Vault",
        "Idea Vault",
        s_vault_organize,
        vault_organize
    ),
    spec!(
        "propose_delete_vault_items",
        "Prepare moving Idea Vault items to Recently Deleted.",
        ["vault.delete"],
        [],
        SoftDelete,
        "deleting Idea Vault items",
        "Idea Vault",
        s_vault_items,
        vault_delete
    ),
    spec!(
        "propose_vault_folder",
        "Prepare creating, renaming or deleting an Idea Vault folder.",
        [
            "vault.create_folder",
            "vault.rename_folder",
            "vault.delete_folder"
        ],
        [],
        Edit,
        "organizing the Idea Vault",
        "Idea Vault",
        s_vault_folder,
        vault_folder
    ),
    spec!(
        "propose_vault_collection",
        "Prepare creating, renaming or deleting an Idea Vault collection.",
        [
            "vault.create_collection",
            "vault.rename_collection",
            "vault.delete_collection"
        ],
        [],
        Edit,
        "organizing the Idea Vault",
        "Idea Vault",
        s_vault_collection,
        vault_collection
    ),
    spec!(
        "propose_send_vault_to_story",
        "Prepare sending an Idea Vault item to Story as a beat, Scene Card, sequence or character (a copy).",
        ["vault.send_to_story"],
        [],
        Edit,
        "sending ideas to Story",
        "Idea Vault",
        s_vault_send,
        vault_send
    ),
];

// ------------------------------------------------------------------ schemas

fn s_settings() -> Value {
    sc::obj(
        &[
            ("title", sc::s(200)),
            ("logline", sc::s(1000)),
            ("genre", sc::s(80)),
            ("language", sc::s(80)),
            ("creator", sc::s(200)),
        ],
        &[],
    )
}
fn s_restore() -> Value {
    sc::obj(&[("item", sc::reference())], &["item"])
}
fn s_move_file() -> Value {
    sc::obj(
        &[
            ("file", sc::reference()),
            ("folder", sc::sd(200, "folder name; omit for the top level")),
        ],
        &["file"],
    )
}
fn s_file_notes() -> Value {
    sc::obj(
        &[("file", sc::reference()), ("notes", sc::s(4000))],
        &["file", "notes"],
    )
}
fn s_file() -> Value {
    sc::obj(&[("file", sc::reference())], &["file"])
}
fn s_rename_folder() -> Value {
    sc::obj(
        &[("folder", sc::reference()), ("name", sc::s(120))],
        &["folder", "name"],
    )
}
fn s_folder() -> Value {
    sc::obj(&[("folder", sc::reference())], &["folder"])
}
fn s_update_note() -> Value {
    sc::obj(
        &[
            ("note", sc::reference()),
            ("title", sc::s(200)),
            ("body", sc::s(20_000)),
        ],
        &["note"],
    )
}
fn s_pin_note() -> Value {
    sc::obj(
        &[("note", sc::reference()), ("pinned", sc::boolean())],
        &["note", "pinned"],
    )
}
fn s_note() -> Value {
    sc::obj(&[("note", sc::reference())], &["note"])
}
fn s_update_task() -> Value {
    sc::obj(
        &[
            ("task", sc::reference()),
            ("title", sc::s(200)),
            ("notes", sc::s(2000)),
            ("dueDate", sc::sd(10, "YYYY-MM-DD")),
            ("clearDueDate", sc::boolean()),
        ],
        &["task"],
    )
}
fn s_task_done() -> Value {
    sc::obj(
        &[("task", sc::reference()), ("done", sc::boolean())],
        &["task", "done"],
    )
}
fn s_task() -> Value {
    sc::obj(&[("task", sc::reference())], &["task"])
}
fn template_types() -> Vec<&'static str> {
    crate::modules::notes::templates::TEMPLATE_TYPES
        .iter()
        .map(|(k, _)| *k)
        .collect()
}
fn s_template_new() -> Value {
    sc::obj(
        &[("type", sc::en(&template_types())), ("name", sc::s(120))],
        &["type", "name"],
    )
}
fn s_template_rename() -> Value {
    sc::obj(
        &[("template", sc::reference()), ("name", sc::s(120))],
        &["template", "name"],
    )
}
fn s_template() -> Value {
    sc::obj(&[("template", sc::reference())], &["template"])
}
fn target_props() -> Vec<(&'static str, Value)> {
    vec![
        ("sceneNumber", sc::scene_number()),
        ("draft", sc::s(120)),
        ("card", sc::reference()),
        ("character", sc::reference()),
        ("location", sc::reference()),
        ("catalogItem", sc::reference()),
    ]
}
fn s_comment() -> Value {
    let mut p = vec![("text", sc::s(4000))];
    p.extend(target_props());
    sc::obj(&p, &["text"])
}
fn s_reply() -> Value {
    sc::obj(
        &[("comment", sc::reference()), ("text", sc::s(4000))],
        &["comment", "text"],
    )
}
fn s_update_comment() -> Value {
    sc::obj(
        &[
            ("comment", sc::reference()),
            ("text", sc::s(4000)),
            ("status", sc::en(&["Open", "In Discussion"])),
        ],
        &["comment"],
    )
}
fn s_resolve_comment() -> Value {
    sc::obj(
        &[("comment", sc::reference()), ("reopen", sc::boolean())],
        &["comment"],
    )
}
fn s_comment_ref() -> Value {
    sc::obj(&[("comment", sc::reference())], &["comment"])
}
fn s_private_note() -> Value {
    sc::obj(
        &[
            ("text", sc::s(8000)),
            ("sceneNumber", sc::scene_number()),
            ("draft", sc::s(120)),
        ],
        &["text"],
    )
}
fn s_private_update() -> Value {
    sc::obj(
        &[("note", sc::reference()), ("text", sc::s(8000))],
        &["note", "text"],
    )
}
fn s_private_ref() -> Value {
    sc::obj(&[("note", sc::reference())], &["note"])
}
fn s_vault_item() -> Value {
    sc::obj(
        &[
            ("type", sc::en(&["note", "quote", "url"])),
            ("title", sc::s(200)),
            ("text", sc::s(20_000)),
            ("url", sc::s(2000)),
            ("source", sc::s(500)),
            ("tags", sc::arr(sc::s(40), 12)),
            ("folder", sc::reference()),
            ("collection", sc::reference()),
        ],
        &["type"],
    )
}
fn s_vault_update() -> Value {
    sc::obj(
        &[
            ("item", sc::reference()),
            ("title", sc::s(200)),
            ("text", sc::s(20_000)),
            ("caption", sc::s(1000)),
            ("url", sc::s(2000)),
            ("source", sc::s(500)),
        ],
        &["item"],
    )
}
fn s_vault_organize() -> Value {
    sc::obj(
        &[
            ("items", sc::arr(sc::reference(), 50)),
            ("pin", sc::boolean()),
            ("folder", sc::reference()),
            ("topLevel", sc::boolean()),
            ("addToCollection", sc::reference()),
            ("removeFromCollection", sc::reference()),
            ("addTags", sc::arr(sc::s(40), 12)),
            ("removeTag", sc::s(40)),
        ],
        &["items"],
    )
}
fn s_vault_items() -> Value {
    sc::obj(&[("items", sc::arr(sc::reference(), 50))], &["items"])
}
fn s_vault_folder() -> Value {
    sc::obj(
        &[
            ("action", sc::en(&["create", "rename", "delete"])),
            ("folder", sc::reference()),
            ("name", sc::s(120)),
        ],
        &["action"],
    )
}
fn s_vault_collection() -> Value {
    sc::obj(
        &[
            ("action", sc::en(&["create", "rename", "delete"])),
            ("collection", sc::reference()),
            ("name", sc::s(120)),
            ("note", sc::s(1000)),
        ],
        &["action"],
    )
}
fn s_vault_send() -> Value {
    sc::obj(
        &[
            ("item", sc::reference()),
            (
                "as",
                sc::en(&["beat", "scene_card", "sequence", "character"]),
            ),
            ("act", sc::reference()),
            ("sequence", sc::reference()),
        ],
        &["item", "as"],
    )
}

// ------------------------------------------------------------------ project

fn settings(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let c = ctx.conn;
    let (id, title, logline, genre, language, creator, rev): (
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        i64,
    ) = c.query_row(
        "SELECT id, title, logline, genre, language, creator, rev FROM project LIMIT 1",
        [],
        |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
            ))
        },
    )?;
    let mut d = Draft::new(spec, args, "Proposed project settings");
    d.target(
        "project",
        &Found {
            id,
            label: "Project".into(),
            rev,
        },
    );
    let mut op = json!({});
    let mut changed = false;
    changed |= set_text(&mut d, &mut op, "title", "Title", a.s("title"), &title, 200)?;
    changed |= set_text(
        &mut d,
        &mut op,
        "logline",
        "Logline",
        a.raw("logline"),
        logline.as_deref().unwrap_or(""),
        1000,
    )?;
    changed |= set_text(
        &mut d,
        &mut op,
        "genre",
        "Genre",
        a.raw("genre"),
        genre.as_deref().unwrap_or(""),
        80,
    )?;
    changed |= set_text(
        &mut d,
        &mut op,
        "language",
        "Language",
        a.raw("language"),
        language.as_deref().unwrap_or(""),
        80,
    )?;
    changed |= set_text(
        &mut d,
        &mut op,
        "creator",
        "Creator",
        a.raw("creator"),
        creator.as_deref().unwrap_or(""),
        200,
    )?;
    need_change(changed, "the project settings")?;
    d.op("project.update_settings", op, "Update project settings");
    d.done()
}

fn restore(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let reference = a.req("item", "item to restore")?;
    // Another user's private notes never appear (their existence is not revealed).
    let mut stmt = ctx.conn.prepare(
        "SELECT d.id, COALESCE(d.title, d.object_type), d.rev, d.object_type FROM deleted_item d
         WHERE d.table_name <> 'private_note'
            OR d.object_id IN (SELECT id FROM private_note WHERE owner_user_id = ?1)
         ORDER BY d.deleted_at DESC LIMIT 2000",
    )?;
    let rows: Vec<(Found, String)> = stmt
        .query_map([&ctx.actor.user_id], |r| {
            Ok((
                Found {
                    id: r.get(0)?,
                    label: r.get(1)?,
                    rev: r.get(2)?,
                },
                r.get(3)?,
            ))
        })?
        .collect::<Result<_, _>>()?;
    let found = match rows.iter().find(|(f, _)| f.id == reference) {
        Some((f, _)) => f.clone(),
        None => rv::pick(
            "deleted item",
            &reference,
            rows.iter().map(|(f, _)| f.clone()).collect(),
        )?,
    };
    let kind = rows
        .iter()
        .find(|(f, _)| f.id == found.id)
        .map(|(_, t)| t.replace('_', " "))
        .unwrap_or_default();
    let mut d = Draft::new(spec, args, "Proposed restore");
    d.target("deleted_item", &found);
    d.row("Restore", format!("{} ({kind})", found.label));
    d.row(
        "Where",
        "Back to its previous place (or Unassigned if that is gone)",
    );
    d.op(
        "trash.restore",
        json!({"id": found.id}),
        format!("Restore “{}”", found.label),
    );
    d.done()
}

// ------------------------------------------------------------------ files

fn move_file(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let file = rv::find(ctx.conn, ctx.actor, &rv::FILE, &a.req("file", "file")?)?;
    let folder = rv::find_opt(ctx.conn, ctx.actor, &rv::FOLDER, a.s("folder").as_deref())?;
    let mut d = Draft::new(spec, args, "Proposed file move");
    d.target("project_file", &file);
    let dest = match &folder {
        Some(f) => {
            d.base("project_file_folder", &f.id);
            f.label.clone()
        }
        None => "Top level of Project Files".into(),
    };
    d.row("File", file.label.clone())
        .row("Move to", dest.clone());
    d.op(
        "files.move",
        json!({"id": file.id, "folderId": folder.as_ref().map(|f| f.id.clone())}),
        format!("Move “{}” to {dest}", file.label),
    );
    d.done()
}

fn file_notes(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let file = rv::find(ctx.conn, ctx.actor, &rv::FILE, &a.req("file", "file")?)?;
    let old = rv::column(ctx.conn, "project_file", "notes", &file.id)?;
    let mut d = Draft::new(spec, args, "Proposed file notes");
    d.target("project_file", &file);
    let mut op = json!({"id": file.id});
    let changed = set_text(
        &mut d,
        &mut op,
        "notes",
        "Notes",
        a.raw("notes"),
        &old,
        4000,
    )?;
    need_change(changed, &format!("“{}”", file.label))?;
    d.op(
        "files.update_notes",
        op,
        format!("Edit notes of “{}”", file.label),
    );
    d.done()
}

fn delete_file(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let file = rv::find(ctx.conn, ctx.actor, &rv::FILE, &a.req("file", "file")?)?;
    let mut d = Draft::new(spec, args, "Proposed deletion");
    d.target("project_file", &file);
    d.row("Delete", file.label.clone())
        .row("Recoverable", "Yes — from Recently Deleted");
    d.op(
        "files.delete",
        json!({"id": file.id}),
        format!("Delete file “{}”", file.label),
    );
    d.done()
}

fn rename_folder(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let folder = rv::find(
        ctx.conn,
        ctx.actor,
        &rv::FOLDER,
        &a.req("folder", "folder")?,
    )?;
    let name = a.req("name", "new folder name")?;
    let mut d = Draft::new(spec, args, "Proposed folder rename");
    d.target("project_file_folder", &folder);
    d.change("Folder", &folder.label, &name);
    d.op(
        "files.rename_folder",
        json!({"id": folder.id, "name": name, "expectedRev": folder.rev}),
        format!("Rename folder “{}” to “{name}”", folder.label),
    );
    d.done()
}

fn delete_folder(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let folder = rv::find(
        ctx.conn,
        ctx.actor,
        &rv::FOLDER,
        &a.req("folder", "folder")?,
    )?;
    let files = queries::count(
        ctx.conn,
        "SELECT count(*) FROM project_file WHERE folder_id=?1 AND deleted_at IS NULL",
        [&folder.id],
    )?;
    let mut d = Draft::new(spec, args, "Proposed folder deletion");
    d.target("project_file_folder", &folder);
    d.row("Delete folder", folder.label.clone()).row(
        "Its files",
        format!(
            "{} move to the top level (not deleted)",
            plural(files as usize, "file", "files")
        ),
    );
    d.op(
        "files.delete_folder",
        json!({"id": folder.id}),
        format!("Delete folder “{}”", folder.label),
    );
    d.done()
}

// ------------------------------------------------------------------ notes & tasks

fn update_note(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let note = rv::find(ctx.conn, ctx.actor, &rv::NOTE, &a.req("note", "note")?)?;
    let mut d = Draft::new(spec, args, "Proposed note edit");
    d.target("project_note", &note);
    let mut op = json!({"id": note.id, "expectedRev": note.rev});
    let mut changed = false;
    let old_title = rv::column(ctx.conn, "project_note", "title", &note.id)?;
    let old_body = rv::column(ctx.conn, "project_note", "body", &note.id)?;
    changed |= set_text(
        &mut d,
        &mut op,
        "title",
        "Title",
        a.raw("title"),
        &old_title,
        200,
    )?;
    changed |= set_text(
        &mut d,
        &mut op,
        "body",
        "Text",
        a.raw("body"),
        &old_body,
        20_000,
    )?;
    need_change(changed, &format!("the note “{}”", note.label))?;
    d.op(
        "notes.update",
        op,
        format!("Edit Project Note “{}”", note.label),
    );
    d.done()
}

fn pin_note(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let note = rv::find(ctx.conn, ctx.actor, &rv::NOTE, &a.req("note", "note")?)?;
    let pinned = a.flag("pinned");
    let mut d = Draft::new(
        spec,
        args,
        if pinned {
            "Proposed pin"
        } else {
            "Proposed unpin"
        },
    );
    d.target("project_note", &note);
    d.row(if pinned { "Pin" } else { "Unpin" }, note.label.clone());
    d.op(
        "notes.set_pinned",
        json!({"id": note.id, "pinned": pinned}),
        format!(
            "{} Project Note “{}”",
            if pinned { "Pin" } else { "Unpin" },
            note.label
        ),
    );
    d.done()
}

fn delete_note(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let note = rv::find(ctx.conn, ctx.actor, &rv::NOTE, &a.req("note", "note")?)?;
    let mut d = Draft::new(spec, args, "Proposed deletion");
    d.target("project_note", &note);
    d.row("Delete", note.label.clone())
        .row("Recoverable", "Yes — from Recently Deleted");
    d.op(
        "notes.delete",
        json!({"id": note.id}),
        format!("Delete Project Note “{}”", note.label),
    );
    d.done()
}

fn update_task(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let task = rv::find(ctx.conn, ctx.actor, &rv::TASK, &a.req("task", "task")?)?;
    let mut d = Draft::new(spec, args, "Proposed task edit");
    d.target("task", &task);
    let mut op = json!({"id": task.id});
    let mut changed = false;
    changed |= set_text(
        &mut d,
        &mut op,
        "title",
        "Title",
        a.s("title"),
        &task.label,
        200,
    )?;
    let old_notes = rv::column(ctx.conn, "task", "notes", &task.id)?;
    changed |= set_text(
        &mut d,
        &mut op,
        "notes",
        "Notes",
        a.raw("notes"),
        &old_notes,
        2000,
    )?;
    if let Some(due) = a.s("dueDate") {
        op["dueAt"] = json!(super::kit::date_ms(&due)?);
        d.row("Due", due);
        changed = true;
    } else if a.flag("clearDueDate") {
        op["clearDue"] = json!(true);
        d.row("Due", "No due date");
        changed = true;
    }
    need_change(changed, &format!("the task “{}”", task.label))?;
    d.op("tasks.update", op, format!("Edit task “{}”", task.label));
    d.done()
}

fn task_done(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let task = rv::find(ctx.conn, ctx.actor, &rv::TASK, &a.req("task", "task")?)?;
    let done = a.flag("done");
    let mut d = Draft::new(spec, args, "Proposed task status");
    d.target("task", &task);
    let old = rv::column(ctx.conn, "task", "status", &task.id)?;
    d.change(&task.label, &old, if done { "Done" } else { "Open" });
    d.op(
        "tasks.set_done",
        json!({"id": task.id, "done": done}),
        format!(
            "Mark task “{}” {}",
            task.label,
            if done { "done" } else { "open" }
        ),
    );
    d.done()
}

fn delete_task(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let task = rv::find(ctx.conn, ctx.actor, &rv::TASK, &a.req("task", "task")?)?;
    let mut d = Draft::new(spec, args, "Proposed deletion");
    d.target("task", &task);
    d.row("Delete task", task.label.clone())
        .row("Recoverable", "Yes — from Recently Deleted");
    d.op(
        "tasks.delete",
        json!({"id": task.id}),
        format!("Delete task “{}”", task.label),
    );
    d.done()
}

// ------------------------------------------------------------------ templates

fn template_new(
    _ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let t = a.req("type", "template type")?;
    let name = a.req("name", "template name")?;
    let mut d = Draft::new(spec, args, "Proposed project template");
    d.row("Template", name.clone())
        .row("Type", t.replace('_', " "))
        .row("Scope", "This project");
    d.op(
        "templates.create",
        json!({"scope": "project", "templateType": t, "name": name}),
        format!("Create project template “{name}”"),
    );
    d.done()
}

fn template_rename(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let t = rv::find(
        ctx.conn,
        ctx.actor,
        &rv::TEMPLATE,
        &a.req("template", "template")?,
    )?;
    let name = a.req("name", "new template name")?;
    let mut d = Draft::new(spec, args, "Proposed template rename");
    d.target("template", &t);
    d.change("Template", &t.label, &name);
    d.op(
        "templates.update",
        json!({"scope": "project", "id": t.id, "name": name}),
        format!("Rename template “{}” to “{name}”", t.label),
    );
    d.done()
}

fn template_delete(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let t = rv::find(
        ctx.conn,
        ctx.actor,
        &rv::TEMPLATE,
        &a.req("template", "template")?,
    )?;
    let mut d = Draft::new(spec, args, "Proposed template deletion");
    d.target("template", &t);
    d.row("Delete template", t.label.clone());
    d.op(
        "templates.delete",
        json!({"scope": "project", "id": t.id}),
        format!("Delete template “{}”", t.label),
    );
    d.done()
}

// ------------------------------------------------------------------ comments

/// The object a comment or private note is about: (table, id, label, scene id).
fn comment_target(
    ctx: &PropCtx<'_>,
    a: &A<'_>,
    d: &mut Draft,
) -> AppResult<Option<(String, String, String, Option<String>)>> {
    let mut picked = Vec::new();
    if let Some(n) = a.u("sceneNumber") {
        let scope = ctx.scope.cloned().unwrap_or_else(neutral_scope);
        let draft = rv::draft(ctx.conn, &scope, a.s("draft").as_deref())?;
        let s = rv::scene(ctx.conn, &draft, n)?;
        d.pin("draft", json!(draft.name));
        picked.push((
            "screenplay_scene".to_string(),
            s.id.clone(),
            s.label(),
            Some(s.id.clone()),
        ));
    }
    for (key, e) in [
        ("card", &rv::CARD),
        ("character", &rv::CHARACTER),
        ("location", &rv::LOCATION),
        ("catalogItem", &rv::CATALOG_ITEM),
    ] {
        if let Some(r) = a.s(key) {
            let f = rv::find(ctx.conn, ctx.actor, e, &r)?;
            picked.push((e.table.to_string(), f.id, f.label, None));
        }
    }
    if picked.len() > 1 {
        return Err(queries::ambiguous(
            "Which one item should the comment be attached to?",
        ));
    }
    Ok(picked.pop())
}

/// A scope with no selection (current draft only), for rebuilds without context.
pub(super) fn neutral_scope() -> super::super::scope::ResolvedScope {
    super::super::scope::ResolvedScope {
        kind: super::super::scope::AiScopeKind::WholeProject,
        label: "Using: Whole Project".into(),
        draft: None,
        scene: None,
        selection: Vec::new(),
        context: Vec::new(),
    }
}

fn comment(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let text = a.req("text", "comment")?;
    let mut d = Draft::new(spec, args, "Proposed comment");
    let (table, id, label, scene_id) = comment_target(ctx, &a, &mut d)?.ok_or_else(|| {
        queries::ambiguous("What should the comment be about? Name a scene, Scene Card, character, location or catalog item.")
    })?;
    d.target(
        &table,
        &Found {
            id: id.clone(),
            label: label.clone(),
            rev: 0,
        },
    );
    d.row("On", label.clone())
        .row("Comment", queries::truncate_chars(&text, 300));
    let mut op = json!({"targetType": table, "targetId": id, "body": text});
    if let Some(s) = scene_id {
        op["sceneId"] = json!(s);
    }
    d.op("comment.create", op, format!("Comment on {label}"));
    d.done()
}

fn visible_comment(ctx: &PropCtx<'_>, reference: &str) -> AppResult<Found> {
    rv::find(ctx.conn, ctx.actor, &rv::COMMENT, reference)
}

fn reply(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let parent = visible_comment(ctx, &a.req("comment", "comment to reply to")?)?;
    let text = a.req("text", "reply")?;
    let mut d = Draft::new(spec, args, "Proposed reply");
    d.target("comment", &parent);
    d.row("Reply to", parent.label.clone())
        .row("Reply", queries::truncate_chars(&text, 300));
    d.op(
        "comment.reply",
        json!({"parentId": parent.id, "body": text}),
        "Reply to comment",
    );
    d.done()
}

fn update_comment(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let cmt = visible_comment(ctx, &a.req("comment", "comment")?)?;
    let mut d = Draft::new(spec, args, "Proposed comment edit");
    d.target("comment", &cmt);
    let mut op = json!({"id": cmt.id, "expectedRev": cmt.rev});
    let mut changed = false;
    let old_body = rv::column(ctx.conn, "comment", "body", &cmt.id)?;
    changed |= set_text(
        &mut d,
        &mut op,
        "body",
        "Comment",
        a.s("text"),
        &old_body,
        4000,
    )?;
    let old_status = rv::column(ctx.conn, "comment", "status", &cmt.id)?;
    changed |= set_text(
        &mut d,
        &mut op,
        "status",
        "Status",
        a.s("status"),
        &old_status,
        20,
    )?;
    need_change(changed, "the comment")?;
    d.op("comment.update", op, "Edit comment");
    d.done()
}

fn resolve_comment(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let cmt = visible_comment(ctx, &a.req("comment", "comment")?)?;
    let reopen = a.flag("reopen");
    let mut d = Draft::new(
        spec,
        args,
        if reopen {
            "Proposed reopen"
        } else {
            "Proposed resolve"
        },
    );
    d.target("comment", &cmt);
    d.row(if reopen { "Reopen" } else { "Resolve" }, cmt.label.clone());
    let op = if reopen {
        "comment.reopen"
    } else {
        "comment.resolve"
    };
    d.op(
        op,
        json!({"id": cmt.id}),
        if reopen {
            "Reopen comment"
        } else {
            "Resolve comment"
        },
    );
    d.done()
}

fn delete_comment(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let cmt = visible_comment(ctx, &a.req("comment", "comment")?)?;
    let author: String = ctx.conn.query_row(
        "SELECT author_user_id FROM comment WHERE id=?1",
        [&cmt.id],
        |r| r.get(0),
    )?;
    if author != ctx.actor.user_id && !ctx.actor.can(Capability::SoftDelete) {
        return Err(AppError::permission_denied(
            "delete other people's comments",
        ));
    }
    let mut d = Draft::new(spec, args, "Proposed comment deletion");
    d.target("comment", &cmt);
    d.row("Delete comment", cmt.label.clone());
    d.op("comment.delete", json!({"id": cmt.id}), "Delete comment");
    d.done()
}

// ------------------------------------------------------------------ private notes

fn private_note(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let text = a.req("text", "private note")?;
    let mut d = Draft::new(spec, args, "Proposed Private Note");
    d.summary("I prepared a Private Note that only you can see. Nothing has been added yet.");
    let mut op = json!({"body": text});
    if let Some((table, id, label, _)) = comment_target(ctx, &a, &mut d)? {
        op["targetType"] = json!(table);
        op["targetId"] = json!(id);
        d.row("About", label);
    }
    d.row(
        "Private Note (only you)",
        queries::truncate_chars(&text, 300),
    );
    d.op("private_note.create", op, "Create Private Note");
    d.done()
}

fn private_update(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let note = rv::find(
        ctx.conn,
        ctx.actor,
        &rv::PRIVATE_NOTE,
        &a.req("note", "private note")?,
    )?;
    let text = a.req("text", "note text")?;
    let mut d = Draft::new(spec, args, "Proposed Private Note edit");
    d.target("private_note", &note);
    let old = rv::column(ctx.conn, "private_note", "body", &note.id)?;
    d.change("Private Note (only you)", &old, &text);
    d.op(
        "private_note.update",
        json!({"id": note.id, "body": text}),
        "Edit Private Note",
    );
    d.done()
}

fn private_delete(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let note = rv::find(
        ctx.conn,
        ctx.actor,
        &rv::PRIVATE_NOTE,
        &a.req("note", "private note")?,
    )?;
    let mut d = Draft::new(spec, args, "Proposed Private Note deletion");
    d.target("private_note", &note);
    d.row("Delete Private Note", note.label.clone())
        .row("Recoverable", "Yes — from Recently Deleted");
    d.op(
        "private_note.delete",
        json!({"id": note.id}),
        "Delete Private Note",
    );
    d.done()
}

// ------------------------------------------------------------------ Idea Vault

fn vault_item(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let t = a.req("type", "item type")?;
    let kind = VaultItemType::parse(&t)
        .ok_or_else(|| queries::ambiguous("Should it be a note, a quote or a link?"))?;
    let mut d = Draft::new(spec, args, "Proposed Idea Vault item");
    let mut op = json!({"itemType": kind.as_str()});
    d.row(
        "Type",
        match kind {
            VaultItemType::Url => "Link",
            VaultItemType::Quote => "Quote",
            _ => "Note",
        },
    );
    for (key, op_key, label) in [
        ("title", "title", "Title"),
        ("text", "body", "Text"),
        ("url", "url", "Link"),
        ("source", "sourceText", "Source"),
    ] {
        if let Some(v) = a.s(key) {
            d.row(label, queries::truncate_chars(&v, 300));
            op[op_key] = json!(v);
        }
    }
    if kind == VaultItemType::Url && a.s("url").is_none() {
        return Err(queries::ambiguous("What is the link (URL) to save?"));
    }
    if kind != VaultItemType::Url && a.s("text").is_none() && a.s("title").is_none() {
        return Err(queries::ambiguous("What should the idea say?"));
    }
    let tags = a.list("tags");
    if !tags.is_empty() {
        d.row("Tags", tags.join(", "));
        op["tags"] = json!(tags);
    }
    if let Some(f) = rv::find_opt(
        ctx.conn,
        ctx.actor,
        &rv::VAULT_FOLDER,
        a.s("folder").as_deref(),
    )? {
        d.base("vault_folder", &f.id).row("Folder", f.label.clone());
        op["folderId"] = json!(f.id);
    }
    if let Some(col) = rv::find_opt(
        ctx.conn,
        ctx.actor,
        &rv::VAULT_COLLECTION,
        a.s("collection").as_deref(),
    )? {
        d.base("vault_collection", &col.id)
            .row("Collection", col.label.clone());
        op["collectionId"] = json!(col.id);
    }
    d.row("Impact", "1 new object");
    d.op("vault.create", op, "Add to the Idea Vault");
    d.done()
}

fn vault_update(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let item = rv::find(
        ctx.conn,
        ctx.actor,
        &rv::VAULT_ITEM,
        &a.req("item", "Idea Vault item")?,
    )?;
    let mut d = Draft::new(spec, args, "Proposed Idea Vault edit");
    d.target("vault_item", &item);
    let mut op = json!({"id": item.id, "expectedRev": item.rev});
    let mut changed = false;
    for (key, col, op_key, label, max) in [
        ("title", "title", "title", "Title", 200usize),
        ("text", "body", "body", "Text", 20_000),
        ("caption", "caption", "caption", "Caption", 1000),
        ("url", "url", "url", "Link", 2000),
        ("source", "source_text", "sourceText", "Source", 500),
    ] {
        let old = rv::column(ctx.conn, "vault_item", col, &item.id)?;
        changed |= set_text(&mut d, &mut op, op_key, label, a.raw(key), &old, max)?;
    }
    need_change(changed, &format!("“{}”", item.label))?;
    d.op(
        "vault.update",
        op,
        format!("Edit Idea Vault item “{}”", item.label),
    );
    d.done()
}

fn vault_refs(ctx: &PropCtx<'_>, a: &A<'_>, d: &mut Draft) -> AppResult<Vec<Found>> {
    let refs = a.list("items");
    if refs.is_empty() {
        return Err(queries::ambiguous("Which Idea Vault items?"));
    }
    let mut out: Vec<Found> = Vec::new();
    for r in refs {
        let f = rv::find(ctx.conn, ctx.actor, &rv::VAULT_ITEM, &r)?;
        if !out.iter().any(|x| x.id == f.id) {
            d.target("vault_item", &f);
            out.push(f);
        }
    }
    Ok(out)
}

fn names(items: &[Found]) -> String {
    let v: Vec<String> = items
        .iter()
        .take(6)
        .map(|f| format!("“{}”", f.label))
        .collect();
    let more = items.len().saturating_sub(6);
    if more > 0 {
        format!("{} and {more} more", v.join(", "))
    } else {
        queries::join_and(&v)
    }
}

fn vault_organize(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed Idea Vault organization");
    let items = vault_refs(ctx, &a, &mut d)?;
    let ids: Vec<String> = items.iter().map(|f| f.id.clone()).collect();
    d.row("Items", names(&items));
    if let Some(pin) = a.b("pin") {
        d.row(
            if pin { "Pin" } else { "Unpin" },
            plural(ids.len(), "item", "items"),
        );
        d.op(
            "vault.set_pinned",
            json!({"ids": ids, "pinned": pin}),
            if pin { "Pin items" } else { "Unpin items" },
        );
    }
    if a.flag("topLevel") {
        d.row("Move to", "Top level");
        d.op(
            "vault.move_to_folder",
            json!({"ids": ids, "folderId": null}),
            "Move items to the top level",
        );
    } else if let Some(f) = rv::find_opt(
        ctx.conn,
        ctx.actor,
        &rv::VAULT_FOLDER,
        a.s("folder").as_deref(),
    )? {
        d.base("vault_folder", &f.id)
            .row("Move to folder", f.label.clone());
        d.op(
            "vault.move_to_folder",
            json!({"ids": ids, "folderId": f.id}),
            format!("Move items to “{}”", f.label),
        );
    }
    if let Some(col) = rv::find_opt(
        ctx.conn,
        ctx.actor,
        &rv::VAULT_COLLECTION,
        a.s("addToCollection").as_deref(),
    )? {
        d.base("vault_collection", &col.id)
            .row("Add to collection", col.label.clone());
        d.op(
            "vault.add_to_collection",
            json!({"ids": ids, "collectionId": col.id}),
            format!("Add items to “{}”", col.label),
        );
    }
    if let Some(col) = rv::find_opt(
        ctx.conn,
        ctx.actor,
        &rv::VAULT_COLLECTION,
        a.s("removeFromCollection").as_deref(),
    )? {
        d.base("vault_collection", &col.id)
            .row("Remove from collection", col.label.clone());
        d.op(
            "vault.remove_from_collection",
            json!({"ids": ids, "collectionId": col.id}),
            format!("Remove items from “{}”", col.label),
        );
    }
    let tags = a.list("addTags");
    if !tags.is_empty() {
        d.row("Add tags", tags.join(", "));
        d.op(
            "vault.add_tags",
            json!({"ids": ids, "tags": tags}),
            "Tag items",
        );
    }
    if let Some(tag) = a.s("removeTag") {
        d.row("Remove tag", tag.clone());
        d.op(
            "vault.remove_tag",
            json!({"ids": ids, "tag": tag}),
            "Remove tag",
        );
    }
    d.done()
}

fn vault_delete(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed deletion");
    let items = vault_refs(ctx, &a, &mut d)?;
    d.row("Delete", names(&items))
        .row("Recoverable", "Yes — from Recently Deleted");
    let ids: Vec<String> = items.iter().map(|f| f.id.clone()).collect();
    d.op(
        "vault.delete",
        json!({"ids": ids}),
        format!(
            "Delete {}",
            plural(items.len(), "Idea Vault item", "Idea Vault items")
        ),
    );
    d.done()
}

fn vault_folder(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed Idea Vault folder change");
    match a.req("action", "action")?.as_str() {
        "create" => {
            let name = a.req("name", "folder name")?;
            d.row("New folder", name.clone());
            d.op(
                "vault.create_folder",
                json!({"name": name}),
                format!("Create Idea Vault folder “{name}”"),
            );
        }
        "rename" => {
            let f = rv::find(
                ctx.conn,
                ctx.actor,
                &rv::VAULT_FOLDER,
                &a.req("folder", "folder")?,
            )?;
            let name = a.req("name", "new folder name")?;
            d.target("vault_folder", &f)
                .change("Folder", &f.label, &name);
            d.op(
                "vault.rename_folder",
                json!({"id": f.id, "name": name, "expectedRev": f.rev}),
                format!("Rename folder “{}”", f.label),
            );
        }
        _ => {
            let f = rv::find(
                ctx.conn,
                ctx.actor,
                &rv::VAULT_FOLDER,
                &a.req("folder", "folder")?,
            )?;
            d.target("vault_folder", &f)
                .row("Delete folder", f.label.clone())
                .row("Its items", "Move to the top level (not deleted)");
            d.op(
                "vault.delete_folder",
                json!({"id": f.id}),
                format!("Delete folder “{}”", f.label),
            );
        }
    }
    d.done()
}

fn vault_collection(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed Idea Vault collection change");
    match a.req("action", "action")?.as_str() {
        "create" => {
            let name = a.req("name", "collection name")?;
            let mut op = json!({"name": name});
            if let Some(n) = a.s("note") {
                op["note"] = json!(n);
            }
            d.row("New collection", name.clone());
            d.op(
                "vault.create_collection",
                op,
                format!("Create collection “{name}”"),
            );
        }
        "rename" => {
            let col = rv::find(
                ctx.conn,
                ctx.actor,
                &rv::VAULT_COLLECTION,
                &a.req("collection", "collection")?,
            )?;
            let name = a.s("name").unwrap_or_else(|| col.label.clone());
            d.target("vault_collection", &col)
                .change("Collection", &col.label, &name);
            let mut op = json!({"id": col.id, "name": name, "expectedRev": col.rev});
            if let Some(n) = a.raw("note") {
                op["note"] = json!(n);
                d.row("Note", n);
            }
            d.op(
                "vault.rename_collection",
                op,
                format!("Edit collection “{}”", col.label),
            );
        }
        _ => {
            let col = rv::find(
                ctx.conn,
                ctx.actor,
                &rv::VAULT_COLLECTION,
                &a.req("collection", "collection")?,
            )?;
            d.target("vault_collection", &col)
                .row("Delete collection", col.label.clone())
                .row("Its items", "Kept in the Idea Vault");
            d.op(
                "vault.delete_collection",
                json!({"id": col.id}),
                format!("Delete collection “{}”", col.label),
            );
        }
    }
    d.done()
}

fn vault_send(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let item = rv::find(
        ctx.conn,
        ctx.actor,
        &rv::VAULT_ITEM,
        &a.req("item", "Idea Vault item")?,
    )?;
    let target = a.req("as", "kind of Story object")?;
    let mut d = Draft::new(spec, args, "Proposed Story copy");
    d.target("vault_item", &item).module("Story");
    let mut op = json!({"id": item.id, "target": match target.as_str() {
        "beat" => "beat", "scene_card" => "sceneCard", "sequence" => "sequence", _ => "character",
    }});
    let mut place = "Story (default place)".to_string();
    if let Some(seq) = rv::find_opt(
        ctx.conn,
        ctx.actor,
        &rv::SEQUENCE,
        a.s("sequence").as_deref(),
    )? {
        d.base("story_sequence", &seq.id);
        op["parentType"] = json!("sequence");
        op["parentId"] = json!(seq.id);
        place = format!("Sequence “{}”", seq.label);
    } else if let Some(act) = rv::find_opt(ctx.conn, ctx.actor, &rv::ACT, a.s("act").as_deref())? {
        d.base("story_act", &act.id);
        op["parentType"] = json!("act");
        op["parentId"] = json!(act.id);
        place = format!("Act “{}”", act.label);
    }
    d.row("Idea", item.label.clone())
        .row("Becomes", target.replace('_', " "))
        .row("Place in", place)
        .row(
            "Source",
            "The Idea Vault item stays unchanged (a copy is made)",
        );
    d.op(
        "vault.send_to_story",
        op,
        format!("Send “{}” to Story", item.label),
    );
    d.done()
}
