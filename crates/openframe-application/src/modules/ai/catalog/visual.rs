//! Proposal tools for visual planning: moodboards, storyboards and shot lists.
//! Scenes are identified by number in the Production Source draft (or the
//! current draft before production starts), and planning follows scene
//! identity. Media the user must supply (image files, sketches, pasted images)
//! is never invented: panels are created as empty placeholders.

use openframe_domain::{AppError, AppResult, Capability};
use serde_json::{Value, json};

use super::super::queries::{self, SceneRow};
use super::super::toolbox::resolve::{self as rv, Found};
use super::super::toolbox::schema as sc;
use super::super::types::ChangeSetDraft;
use super::kit::{A, Draft, need_change, plural, set_text};
use super::workspace::{neutral_scope, spec};
use super::{PropCtx, ProposalSpec};

pub const SPECS: &[ProposalSpec] = &[
    spec!(
        "propose_mark_planning_reviewed",
        "Prepare marking a scene's visual planning as reviewed after a script change.",
        ["visual.mark_scene_reviewed"],
        [],
        Edit,
        "changing visual planning",
        "Visual Planning",
        s_scene,
        mark_reviewed
    ),
    spec!(
        "propose_moodboard",
        "Prepare a new moodboard (optionally for a scene).",
        ["moodboard.create"],
        [],
        Edit,
        "changing moodboards",
        "Visual Planning",
        s_moodboard_new,
        moodboard_new
    ),
    spec!(
        "propose_update_moodboard",
        "Prepare renaming a moodboard or editing its notes.",
        ["moodboard.rename", "moodboard.update_notes"],
        [],
        Edit,
        "changing moodboards",
        "Visual Planning",
        s_moodboard_update,
        moodboard_update
    ),
    spec!(
        "propose_delete_moodboard",
        "Prepare moving a moodboard to Recently Deleted.",
        ["moodboard.delete"],
        [],
        SoftDelete,
        "deleting moodboards",
        "Visual Planning",
        s_moodboard,
        moodboard_delete
    ),
    spec!(
        "propose_moodboard_vault_image",
        "Prepare placing an Idea Vault image on a moodboard.",
        ["moodboard.add_vault_image"],
        [],
        Edit,
        "changing moodboards",
        "Visual Planning",
        s_vault_image,
        moodboard_vault_image
    ),
    spec!(
        "propose_moodboard_note",
        "Prepare a text note on a moodboard.",
        ["moodboard.add_note"],
        [],
        Edit,
        "changing moodboards",
        "Visual Planning",
        s_moodboard_note,
        moodboard_note
    ),
    spec!(
        "propose_moodboard_link",
        "Prepare a web link on a moodboard.",
        ["moodboard.add_link"],
        [],
        Edit,
        "changing moodboards",
        "Visual Planning",
        s_moodboard_link,
        moodboard_link
    ),
    spec!(
        "propose_update_moodboard_item",
        "Prepare edits to a moodboard item's caption, note text or link.",
        ["moodboard.update_item"],
        [],
        Edit,
        "changing moodboards",
        "Visual Planning",
        s_moodboard_item_update,
        moodboard_item_update
    ),
    spec!(
        "propose_delete_moodboard_items",
        "Prepare moving moodboard items to Recently Deleted.",
        ["moodboard.delete_items"],
        [],
        SoftDelete,
        "changing moodboards",
        "Visual Planning",
        s_moodboard_items,
        moodboard_items_delete
    ),
    spec!(
        "propose_storyboard",
        "Prepare a new storyboard (for a scene, or standalone with a name).",
        ["storyboard.create"],
        [],
        Edit,
        "changing storyboards",
        "Visual Planning",
        s_storyboard_new,
        storyboard_new
    ),
    spec!(
        "propose_update_storyboard",
        "Prepare renaming a storyboard or changing its scene (or making it standalone).",
        ["storyboard.rename", "storyboard.set_scene"],
        [],
        Edit,
        "changing storyboards",
        "Visual Planning",
        s_storyboard_update,
        storyboard_update
    ),
    spec!(
        "propose_delete_storyboard",
        "Prepare moving a storyboard to Recently Deleted.",
        ["storyboard.delete"],
        [],
        SoftDelete,
        "deleting storyboards",
        "Visual Planning",
        s_storyboard,
        storyboard_delete
    ),
    spec!(
        "propose_storyboard_panel",
        "Prepare an empty storyboard panel with a description (images are added by the user).",
        ["storyboard.add_panel"],
        [],
        Edit,
        "changing storyboards",
        "Visual Planning",
        s_panel_new,
        panel_new
    ),
    spec!(
        "propose_update_storyboard_panel",
        "Prepare edits to a panel (description, framing, movement, angle, sound, duration, note).",
        ["storyboard.update_panel"],
        [],
        Edit,
        "changing storyboards",
        "Visual Planning",
        s_panel_update,
        panel_update
    ),
    spec!(
        "propose_move_storyboard_panel",
        "Prepare moving a panel to another position or another storyboard.",
        ["storyboard.reorder_panel", "storyboard.move_panel"],
        [],
        Edit,
        "changing storyboards",
        "Visual Planning",
        s_panel_move,
        panel_move
    ),
    spec!(
        "propose_delete_storyboard_panels",
        "Prepare moving storyboard panels to Recently Deleted.",
        ["storyboard.delete_panels"],
        [],
        SoftDelete,
        "changing storyboards",
        "Visual Planning",
        s_panels,
        panels_delete
    ),
    spec!(
        "propose_link_panel_shot",
        "Prepare linking a storyboard panel to a shot (or removing the link).",
        ["storyboard.link_shot"],
        [],
        Edit,
        "changing storyboards",
        "Visual Planning",
        s_panel_link,
        panel_link
    ),
    spec!(
        "propose_shot_from_panel",
        "Prepare a shot created from a storyboard panel.",
        ["storyboard.create_shot_from_panel"],
        [],
        Edit,
        "changing shot lists",
        "Visual Planning",
        s_panel_shot,
        shot_from_panel
    ),
    spec!(
        "propose_shot",
        "Prepare a new shot in a scene's shot list.",
        ["shot.create"],
        [],
        Edit,
        "changing shot lists",
        "Visual Planning",
        s_shot_new,
        shot_new
    ),
    spec!(
        "propose_update_shot",
        "Prepare edits to a shot (description, size, movement, angle, lens, camera notes, characters, sound).",
        ["shot.update"],
        [],
        Edit,
        "changing shot lists",
        "Visual Planning",
        s_shot_update,
        shot_update
    ),
    spec!(
        "propose_move_shot",
        "Prepare moving a shot to another position or another scene.",
        ["shot.reorder", "shot.move"],
        [],
        Edit,
        "changing shot lists",
        "Visual Planning",
        s_shot_move,
        shot_move
    ),
    spec!(
        "propose_duplicate_shot",
        "Prepare duplicating a shot.",
        ["shot.duplicate"],
        [],
        Edit,
        "changing shot lists",
        "Visual Planning",
        s_shot,
        shot_duplicate
    ),
    spec!(
        "propose_delete_shots",
        "Prepare moving shots of a scene to Recently Deleted.",
        ["shot.delete"],
        [],
        SoftDelete,
        "changing shot lists",
        "Visual Planning",
        s_shots,
        shots_delete
    ),
    spec!(
        "propose_panel_from_shot",
        "Prepare a storyboard panel for a shot.",
        ["shot.create_panel"],
        [],
        Edit,
        "changing storyboards",
        "Visual Planning",
        s_shot,
        panel_from_shot
    ),
    spec!(
        "propose_copy_planning",
        "Prepare copying shots and storyboards from one scene to another.",
        ["shot.copy_planning"],
        [],
        Edit,
        "changing shot lists",
        "Visual Planning",
        s_copy,
        copy_planning
    ),
];

// ------------------------------------------------------------------ schemas

fn s_scene() -> Value {
    sc::obj(&[("sceneNumber", sc::scene_number())], &["sceneNumber"])
}
fn s_moodboard_new() -> Value {
    sc::obj(
        &[("name", sc::s(200)), ("sceneNumber", sc::scene_number())],
        &["name"],
    )
}
fn s_moodboard_update() -> Value {
    sc::obj(
        &[
            ("moodboard", sc::reference()),
            ("name", sc::s(200)),
            ("notes", sc::s(4000)),
        ],
        &["moodboard"],
    )
}
fn s_moodboard() -> Value {
    sc::obj(&[("moodboard", sc::reference())], &["moodboard"])
}
fn s_vault_image() -> Value {
    sc::obj(
        &[
            ("moodboard", sc::reference()),
            ("vaultItem", sc::reference()),
        ],
        &["moodboard", "vaultItem"],
    )
}
fn s_moodboard_note() -> Value {
    sc::obj(
        &[
            ("moodboard", sc::reference()),
            ("text", sc::s(4000)),
            ("private", sc::boolean()),
        ],
        &["moodboard", "text"],
    )
}
fn s_moodboard_link() -> Value {
    sc::obj(
        &[
            ("moodboard", sc::reference()),
            ("url", sc::s(2000)),
            ("title", sc::s(200)),
        ],
        &["moodboard", "url"],
    )
}
fn s_moodboard_item_update() -> Value {
    sc::obj(
        &[
            ("moodboard", sc::reference()),
            ("item", sc::reference()),
            ("caption", sc::s(1000)),
            ("text", sc::s(4000)),
            ("url", sc::s(2000)),
            ("linkTitle", sc::s(200)),
        ],
        &["moodboard", "item"],
    )
}
fn s_moodboard_items() -> Value {
    sc::obj(
        &[
            ("moodboard", sc::reference()),
            ("items", sc::arr(sc::reference(), 50)),
        ],
        &["moodboard", "items"],
    )
}
fn s_storyboard_new() -> Value {
    sc::obj(
        &[("name", sc::s(200)), ("sceneNumber", sc::scene_number())],
        &[],
    )
}
fn s_storyboard_update() -> Value {
    sc::obj(
        &[
            ("storyboard", sc::reference()),
            ("name", sc::s(200)),
            ("sceneNumber", sc::scene_number()),
            ("standalone", sc::boolean()),
        ],
        &["storyboard"],
    )
}
fn s_storyboard() -> Value {
    sc::obj(&[("storyboard", sc::reference())], &["storyboard"])
}
fn s_panel_new() -> Value {
    sc::obj(
        &[
            ("storyboard", sc::reference()),
            ("description", sc::s(2000)),
            ("position", sc::int(1, 1000)),
        ],
        &["storyboard"],
    )
}
fn s_panel_update() -> Value {
    sc::obj(
        &[
            ("storyboard", sc::reference()),
            ("panel", sc::int(1, 1000)),
            ("description", sc::s(2000)),
            ("framing", sc::s(120)),
            ("movement", sc::s(120)),
            ("angle", sc::s(120)),
            ("sound", sc::s(500)),
            ("durationSeconds", sc::int(0, 3600)),
            ("note", sc::s(2000)),
        ],
        &["storyboard", "panel"],
    )
}
fn s_panel_move() -> Value {
    sc::obj(
        &[
            ("storyboard", sc::reference()),
            ("panel", sc::int(1, 1000)),
            ("toStoryboard", sc::reference()),
            ("position", sc::int(1, 1000)),
        ],
        &["storyboard", "panel"],
    )
}
fn s_panels() -> Value {
    sc::obj(
        &[
            ("storyboard", sc::reference()),
            ("panels", sc::arr(sc::int(1, 1000), 50)),
        ],
        &["storyboard", "panels"],
    )
}
fn s_panel_link() -> Value {
    sc::obj(
        &[
            ("storyboard", sc::reference()),
            ("panel", sc::int(1, 1000)),
            ("sceneNumber", sc::scene_number()),
            ("shot", sc::int(1, 1000)),
            ("unlink", sc::boolean()),
        ],
        &["storyboard", "panel"],
    )
}
fn s_panel_shot() -> Value {
    sc::obj(
        &[
            ("storyboard", sc::reference()),
            ("panel", sc::int(1, 1000)),
            ("sceneNumber", sc::scene_number()),
        ],
        &["storyboard", "panel"],
    )
}
fn shot_fields() -> Vec<(&'static str, Value)> {
    vec![
        ("description", sc::s(2000)),
        ("size", sc::s(60)),
        ("movement", sc::s(120)),
        ("angle", sc::s(120)),
        ("lens", sc::s(60)),
        ("cameraNotes", sc::s(2000)),
        ("characters", sc::arr(sc::s(120), 20)),
        ("sound", sc::s(500)),
    ]
}
fn s_shot_new() -> Value {
    let mut p = vec![
        ("sceneNumber", sc::scene_number()),
        ("position", sc::int(1, 1000)),
    ];
    p.extend(shot_fields());
    sc::obj(&p, &["sceneNumber", "description"])
}
fn s_shot_update() -> Value {
    let mut p = vec![
        ("sceneNumber", sc::scene_number()),
        ("shot", sc::int(1, 1000)),
    ];
    p.extend(shot_fields());
    sc::obj(&p, &["sceneNumber", "shot"])
}
fn s_shot_move() -> Value {
    sc::obj(
        &[
            ("sceneNumber", sc::scene_number()),
            ("shot", sc::int(1, 1000)),
            ("toPosition", sc::int(1, 1000)),
            ("toSceneNumber", sc::scene_number()),
        ],
        &["sceneNumber", "shot"],
    )
}
fn s_shot() -> Value {
    sc::obj(
        &[
            ("sceneNumber", sc::scene_number()),
            ("shot", sc::int(1, 1000)),
        ],
        &["sceneNumber", "shot"],
    )
}
fn s_shots() -> Value {
    sc::obj(
        &[
            ("sceneNumber", sc::scene_number()),
            ("shots", sc::arr(sc::int(1, 1000), 50)),
        ],
        &["sceneNumber", "shots"],
    )
}
fn s_copy() -> Value {
    sc::obj(
        &[
            ("fromSceneNumber", sc::scene_number()),
            ("toSceneNumber", sc::scene_number()),
        ],
        &["fromSceneNumber", "toSceneNumber"],
    )
}

// ------------------------------------------------------------------ helpers

fn scene(ctx: &PropCtx<'_>, n: u32, d: &mut Draft) -> AppResult<SceneRow> {
    let scope = ctx.scope.cloned().unwrap_or_else(neutral_scope);
    let draft = rv::production_draft(ctx.conn, &scope, None)?;
    let s = rv::scene(ctx.conn, &draft, n)?;
    d.base("screenplay_scene", &s.id);
    Ok(s)
}

fn scene_arg(
    ctx: &PropCtx<'_>,
    a: &A<'_>,
    key: &str,
    d: &mut Draft,
) -> AppResult<Option<SceneRow>> {
    match a.u(key) {
        Some(n) => scene(ctx, n, d).map(Some),
        None => Ok(None),
    }
}

fn board(ctx: &PropCtx<'_>, a: &A<'_>, d: &mut Draft) -> AppResult<Found> {
    let f = rv::find(
        ctx.conn,
        ctx.actor,
        &rv::MOODBOARD,
        &a.req("moodboard", "moodboard")?,
    )?;
    d.target("moodboard", &f);
    Ok(f)
}

fn storyboard(ctx: &PropCtx<'_>, a: &A<'_>, key: &str, d: &mut Draft) -> AppResult<Found> {
    let f = rv::find(
        ctx.conn,
        ctx.actor,
        &rv::STORYBOARD,
        &a.req(key, "storyboard")?,
    )?;
    d.target("storyboard", &f);
    Ok(f)
}

fn panel(ctx: &PropCtx<'_>, a: &A<'_>, sb: &Found, d: &mut Draft) -> AppResult<Found> {
    let p = rv::panel(ctx.conn, sb, a.u("panel").unwrap_or(1))?;
    d.target("storyboard_panel", &p);
    Ok(p)
}

fn shot(ctx: &PropCtx<'_>, s: &SceneRow, n: u32, d: &mut Draft) -> AppResult<Found> {
    let f = rv::shot(ctx.conn, s, n)?;
    d.target("shot", &f);
    Ok(f)
}

fn board_item(ctx: &PropCtx<'_>, board: &Found, reference: &str) -> AppResult<Found> {
    let mut stmt = ctx.conn.prepare(
        "SELECT id, COALESCE(NULLIF(trim(caption),''), NULLIF(substr(trim(body),1,80),''), NULLIF(trim(link_title),''), url, kind), rev
         FROM moodboard_item WHERE moodboard_id=?1 AND deleted_at IS NULL ORDER BY z, id",
    )?;
    let items: Vec<Found> = stmt
        .query_map([&board.id], |r| {
            Ok(Found {
                id: r.get(0)?,
                label: r.get(1)?,
                rev: r.get(2)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    match items.iter().find(|f| f.id == reference) {
        Some(f) => Ok(f.clone()),
        None => rv::pick("moodboard item", reference, items),
    }
}

// ------------------------------------------------------------------ scenes & moodboards

fn mark_reviewed(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed planning review");
    let s = scene(ctx, a.u("sceneNumber").unwrap_or(0), &mut d)?;
    d.row("Mark planning reviewed", s.label())
        .row("Covers", "Its shots, storyboards and moodboards");
    d.op(
        "visual.mark_scene_reviewed",
        json!({"sceneLineageId": s.lineage_id}),
        format!("Mark planning of {} reviewed", s.label()),
    );
    d.done()
}

fn moodboard_new(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let name = a.req("name", "moodboard name")?;
    let mut d = Draft::new(spec, args, "Proposed moodboard");
    let mut op = json!({"name": name});
    d.row("Moodboard", name.clone());
    if let Some(s) = scene_arg(ctx, &a, "sceneNumber", &mut d)? {
        op["sceneId"] = json!(s.id);
        d.row("For", s.label());
    }
    d.op("moodboard.create", op, format!("Create moodboard “{name}”"));
    d.done()
}

fn moodboard_update(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed moodboard change");
    let b = board(ctx, &a, &mut d)?;
    let mut changed = false;
    if let Some(n) = a.s("name")
        && n != b.label
    {
        d.change("Name", &b.label, &n);
        d.op(
            "moodboard.rename",
            json!({"id": b.id, "name": n, "expectedRev": b.rev}),
            format!("Rename moodboard “{}”", b.label),
        );
        changed = true;
    }
    if let Some(n) = a.raw("notes") {
        let old = rv::column(ctx.conn, "moodboard", "notes", &b.id)?;
        if old.trim() != n {
            d.change("Notes", &old, &n);
            d.op(
                "moodboard.update_notes",
                json!({"id": b.id, "notes": n}),
                format!("Edit notes of “{}”", b.label),
            );
            changed = true;
        }
    }
    need_change(changed, &format!("moodboard “{}”", b.label))?;
    d.done()
}

fn moodboard_delete(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed moodboard deletion");
    let b = board(ctx, &a, &mut d)?;
    let items = queries::count(
        ctx.conn,
        "SELECT count(*) FROM moodboard_item WHERE moodboard_id=?1 AND deleted_at IS NULL",
        [&b.id],
    )?;
    d.row("Delete moodboard", b.label.clone())
        .row("Items on it", items.to_string())
        .row("Recoverable", "Yes — from Recently Deleted");
    d.op(
        "moodboard.delete",
        json!({"id": b.id}),
        format!("Delete moodboard “{}”", b.label),
    );
    d.done()
}

fn moodboard_vault_image(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed moodboard image");
    let b = board(ctx, &a, &mut d)?;
    let item = rv::find(
        ctx.conn,
        ctx.actor,
        &rv::VAULT_ITEM,
        &a.req("vaultItem", "Idea Vault image")?,
    )?;
    let kind = rv::column(ctx.conn, "vault_item", "item_type", &item.id)?;
    if !matches!(kind.as_str(), "image" | "sketch" | "screenshot") {
        return Err(queries::ambiguous(format!(
            "“{}” isn't an image. Which Idea Vault image should I use?",
            item.label
        )));
    }
    d.target("vault_item", &item).module("Idea Vault");
    d.row("Place image", item.label.clone())
        .row("On", b.label.clone())
        .row("Idea Vault item", "Unchanged (the image is shared)");
    d.op(
        "moodboard.add_vault_image",
        json!({"moodboardId": b.id, "vaultItemId": item.id}),
        format!("Add “{}” to “{}”", item.label, b.label),
    );
    d.done()
}

fn moodboard_note(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed moodboard note");
    let b = board(ctx, &a, &mut d)?;
    let text = a.req("text", "note")?;
    let private = a.flag("private");
    d.row("Note", queries::truncate_chars(&text, 300))
        .row("On", b.label.clone());
    if private {
        d.row("Internal", "Left out of standard exports");
    }
    d.op(
        "moodboard.add_note",
        json!({"moodboardId": b.id, "text": text, "isPrivate": private}),
        format!("Add a note to “{}”", b.label),
    );
    d.done()
}

fn moodboard_link(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed moodboard link");
    let b = board(ctx, &a, &mut d)?;
    let url = a.req("url", "link")?;
    let mut op = json!({"moodboardId": b.id, "url": url});
    if let Some(t) = a.s("title") {
        op["title"] = json!(t);
        d.row("Title", t);
    }
    d.row("Link", url)
        .row("On", b.label.clone())
        .row("Network", "OpenFrame does not open or fetch the link");
    d.op(
        "moodboard.add_link",
        op,
        format!("Add a link to “{}”", b.label),
    );
    d.done()
}

fn moodboard_item_update(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed moodboard item edit");
    let b = board(ctx, &a, &mut d)?;
    let it = board_item(ctx, &b, &a.req("item", "moodboard item")?)?;
    d.target("moodboard_item", &it);
    let mut op = json!({"id": it.id, "expectedRev": it.rev});
    let mut changed = false;
    for (key, col, op_key, label, max) in [
        ("caption", "caption", "caption", "Caption", 1000usize),
        ("text", "body", "body", "Note text", 4000),
        ("url", "url", "url", "Link", 2000),
        ("linkTitle", "link_title", "linkTitle", "Link title", 200),
    ] {
        let old = rv::column(ctx.conn, "moodboard_item", col, &it.id)?;
        changed |= set_text(&mut d, &mut op, op_key, label, a.raw(key), &old, max)?;
    }
    need_change(changed, &format!("“{}”", it.label))?;
    d.op(
        "moodboard.update_item",
        op,
        format!("Edit moodboard item “{}”", it.label),
    );
    d.done()
}

fn moodboard_items_delete(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed moodboard item deletion");
    let b = board(ctx, &a, &mut d)?;
    let mut ids = Vec::new();
    for r in a.list("items") {
        let it = board_item(ctx, &b, &r)?;
        if !ids.contains(&it.id) {
            d.target("moodboard_item", &it)
                .row("Delete", it.label.clone());
            ids.push(it.id);
        }
    }
    d.row("Recoverable", "Yes — from Recently Deleted");
    d.op(
        "moodboard.delete_items",
        json!({"ids": ids}),
        format!(
            "Delete {} from “{}”",
            plural(ids.len(), "item", "items"),
            b.label
        ),
    );
    d.done()
}

// ------------------------------------------------------------------ storyboards

fn storyboard_new(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed storyboard");
    let mut op = json!({});
    let s = scene_arg(ctx, &a, "sceneNumber", &mut d)?;
    let name = a.s("name");
    if s.is_none() && name.is_none() {
        return Err(queries::ambiguous(
            "Which scene is the storyboard for, or what should it be called?",
        ));
    }
    if let Some(n) = &name {
        op["name"] = json!(n);
        d.row("Storyboard", n.clone());
    }
    match &s {
        Some(s) => {
            op["sceneId"] = json!(s.id);
            d.row("For", s.label());
        }
        None => {
            d.row("For", "Standalone (no scene)");
        }
    }
    d.op("storyboard.create", op, "Create storyboard");
    d.done()
}

fn storyboard_update(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed storyboard change");
    let sb = storyboard(ctx, &a, "storyboard", &mut d)?;
    let mut changed = false;
    if let Some(n) = a.s("name")
        && n != sb.label
    {
        d.change("Name", &sb.label, &n);
        d.op(
            "storyboard.rename",
            json!({"id": sb.id, "name": n, "expectedRev": sb.rev}),
            format!("Rename storyboard “{}”", sb.label),
        );
        changed = true;
    }
    if a.flag("standalone") {
        d.row("Scene", "None (standalone)");
        d.op(
            "storyboard.set_scene",
            json!({"id": sb.id, "sceneId": null}),
            format!("Make “{}” standalone", sb.label),
        );
        changed = true;
    } else if let Some(s) = scene_arg(ctx, &a, "sceneNumber", &mut d)? {
        d.row("Scene", s.label());
        d.op(
            "storyboard.set_scene",
            json!({"id": sb.id, "sceneId": s.id}),
            format!("Set scene of “{}”", sb.label),
        );
        changed = true;
    }
    need_change(changed, &format!("storyboard “{}”", sb.label))?;
    d.done()
}

fn storyboard_delete(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed storyboard deletion");
    let sb = storyboard(ctx, &a, "storyboard", &mut d)?;
    let panels = queries::count(
        ctx.conn,
        "SELECT count(*) FROM storyboard_panel WHERE storyboard_id=?1 AND deleted_at IS NULL",
        [&sb.id],
    )?;
    d.row("Delete storyboard", sb.label.clone())
        .row("Panels", panels.to_string())
        .row("Linked shots", "Kept (links are removed)")
        .row("Recoverable", "Yes — from Recently Deleted");
    d.op(
        "storyboard.delete",
        json!({"id": sb.id}),
        format!("Delete storyboard “{}”", sb.label),
    );
    d.done()
}

fn panel_new(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed storyboard panel");
    let sb = storyboard(ctx, &a, "storyboard", &mut d)?;
    let mut op = json!({"storyboardId": sb.id, "visual": "placeholder"});
    if let Some(t) = a.s("description") {
        d.row("Description", queries::truncate_chars(&t, 300));
        op["description"] = json!(t);
    }
    if let Some(p) = a.u("position") {
        op["index"] = json!(p.max(1) - 1);
        d.row("Position", p.to_string());
    }
    d.row(
        "Panel",
        format!("Empty panel on “{}” (add the image yourself)", sb.label),
    );
    d.op(
        "storyboard.add_panel",
        op,
        format!("Add a panel to “{}”", sb.label),
    );
    d.done()
}

fn panel_update(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed panel edit");
    let sb = storyboard(ctx, &a, "storyboard", &mut d)?;
    let p = panel(ctx, &a, &sb, &mut d)?;
    let mut op = json!({"id": p.id, "expectedRev": p.rev});
    let mut changed = false;
    for (key, col, op_key, label, max) in [
        (
            "description",
            "description",
            "description",
            "Description",
            2000usize,
        ),
        ("framing", "framing", "framing", "Framing", 120),
        ("movement", "movement", "movement", "Movement", 120),
        ("angle", "angle", "angle", "Angle", 120),
        ("sound", "sound_note", "soundNote", "Sound", 500),
        ("note", "note", "note", "Note", 2000),
    ] {
        let old = rv::column(ctx.conn, "storyboard_panel", col, &p.id)?;
        changed |= set_text(&mut d, &mut op, op_key, label, a.raw(key), &old, max)?;
    }
    if let Some(sec) = a.i("durationSeconds") {
        d.row(
            "Duration",
            if sec == 0 {
                "Cleared".to_string()
            } else {
                format!("{sec} s")
            },
        );
        op["durationMs"] = json!(sec * 1000);
        changed = true;
    }
    need_change(changed, &format!("{} of “{}”", p.label, sb.label))?;
    d.op(
        "storyboard.update_panel",
        op,
        format!("Edit {} of “{}”", p.label, sb.label),
    );
    d.done()
}

fn panel_move(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed panel move");
    let sb = storyboard(ctx, &a, "storyboard", &mut d)?;
    let p = panel(ctx, &a, &sb, &mut d)?;
    let position = a.u("position").map(|x| x.max(1) - 1);
    match a.s("toStoryboard") {
        Some(_) => {
            let to = storyboard(ctx, &a, "toStoryboard", &mut d)?;
            let mut op = json!({"id": p.id, "storyboardId": to.id});
            if let Some(i) = position {
                op["index"] = json!(i);
            }
            d.row("Move", format!("{} of “{}”", p.label, sb.label))
                .row("To", format!("“{}”", to.label));
            d.op(
                "storyboard.move_panel",
                op,
                format!("Move {} to “{}”", p.label, to.label),
            );
        }
        None => {
            let i = position.ok_or_else(|| {
                queries::ambiguous(
                    "Where should the panel go? Give a position or another storyboard.",
                )
            })?;
            d.row("Move", format!("{} of “{}”", p.label, sb.label))
                .row("To position", (i + 1).to_string());
            d.op(
                "storyboard.reorder_panel",
                json!({"id": p.id, "index": i}),
                format!("Move {} to position {}", p.label, i + 1),
            );
        }
    }
    d.done()
}

fn panels_delete(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed panel deletion");
    let sb = storyboard(ctx, &a, "storyboard", &mut d)?;
    let mut ids = Vec::new();
    let mut labels = Vec::new();
    for n in a.ints("panels") {
        let p = rv::panel(ctx.conn, &sb, n)?;
        if !ids.contains(&p.id) {
            d.target("storyboard_panel", &p);
            ids.push(p.id);
            labels.push(n.to_string());
        }
    }
    d.row(
        "Delete panels",
        format!("{} of “{}”", queries::join_and(&labels), sb.label),
    )
    .row("Recoverable", "Yes — from Recently Deleted");
    d.op(
        "storyboard.delete_panels",
        json!({"ids": ids}),
        format!(
            "Delete {} from “{}”",
            plural(labels.len(), "panel", "panels"),
            sb.label
        ),
    );
    d.done()
}

fn panel_link(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed panel link");
    let sb = storyboard(ctx, &a, "storyboard", &mut d)?;
    let p = panel(ctx, &a, &sb, &mut d)?;
    if a.flag("unlink") {
        d.row(
            "Unlink",
            format!("{} of “{}” from its shot", p.label, sb.label),
        );
        d.op(
            "storyboard.link_shot",
            json!({"panelId": p.id, "shotId": null}),
            format!("Unlink {} from its shot", p.label),
        );
        return d.done();
    }
    let n = a
        .u("sceneNumber")
        .ok_or_else(|| queries::ambiguous("Which scene's shot should the panel link to?"))?;
    let s = scene(ctx, n, &mut d)?;
    let sh = shot(ctx, &s, a.u("shot").unwrap_or(1), &mut d)?;
    d.row("Link", format!("{} of “{}”", p.label, sb.label)).row(
        "To",
        format!(
            "Shot {} of {} — {}",
            a.u("shot").unwrap_or(1),
            s.label(),
            queries::truncate_chars(&sh.label, 80)
        ),
    );
    d.op(
        "storyboard.link_shot",
        json!({"panelId": p.id, "shotId": sh.id}),
        format!("Link {} to a shot", p.label),
    );
    d.done()
}

fn shot_from_panel(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed shot from panel");
    let sb = storyboard(ctx, &a, "storyboard", &mut d)?;
    let p = panel(ctx, &a, &sb, &mut d)?;
    let mut op = json!({"panelId": p.id});
    let board_scene = rv::column(ctx.conn, "storyboard", "scene_id", &sb.id)?;
    if board_scene.is_empty() {
        let s = scene_arg(ctx, &a, "sceneNumber", &mut d)?.ok_or_else(|| {
            queries::ambiguous(
                "This storyboard has no scene. Which scene should the shot belong to?",
            )
        })?;
        op["sceneId"] = json!(s.id);
        d.row("Scene", s.label());
    }
    d.row("Create shot from", format!("{} of “{}”", p.label, sb.label));
    d.op(
        "storyboard.create_shot_from_panel",
        op,
        format!("Create a shot from {}", p.label),
    );
    d.done()
}

// ------------------------------------------------------------------ shots

fn shot_args(a: &A<'_>, d: &mut Draft, op: &mut Value) {
    for (key, op_key, label) in [
        ("size", "size", "Size"),
        ("movement", "movement", "Movement"),
        ("angle", "angle", "Angle"),
        ("lens", "lens", "Lens"),
        ("cameraNotes", "cameraNotes", "Camera notes"),
        ("sound", "soundNote", "Sound"),
    ] {
        if let Some(v) = a.s(key) {
            d.row(label, queries::truncate_chars(&v, 200));
            op[op_key] = json!(v);
        }
    }
    let chars = a.list("characters");
    if !chars.is_empty() {
        d.row("Characters", chars.join(", "));
        op["characters"] = json!(chars);
    }
}

fn shot_new(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed shot");
    let s = scene(ctx, a.u("sceneNumber").unwrap_or(0), &mut d)?;
    let desc = a.req("description", "shot description")?;
    let mut op = json!({"sceneId": s.id, "description": desc});
    d.row("Shot", queries::truncate_chars(&desc, 300))
        .row("Scene", s.label());
    if let Some(p) = a.u("position") {
        op["index"] = json!(p.max(1) - 1);
        d.row("Position", p.to_string());
    }
    shot_args(&a, &mut d, &mut op);
    d.op("shot.create", op, format!("Add a shot to {}", s.label()));
    d.done()
}

fn shot_update(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed shot edit");
    let s = scene(ctx, a.u("sceneNumber").unwrap_or(0), &mut d)?;
    let n = a.u("shot").unwrap_or(1);
    let sh = shot(ctx, &s, n, &mut d)?;
    let mut op = json!({"id": sh.id, "expectedRev": sh.rev});
    let mut changed = set_text(
        &mut d,
        &mut op,
        "description",
        "Description",
        a.s("description"),
        &sh.label,
        2000,
    )?;
    for (key, col, op_key, label, max) in [
        ("size", "size", "size", "Size", 60usize),
        ("movement", "movement", "movement", "Movement", 120),
        ("angle", "angle", "angle", "Angle", 120),
        ("lens", "lens", "lens", "Lens", 60),
        (
            "cameraNotes",
            "camera_notes",
            "cameraNotes",
            "Camera notes",
            2000,
        ),
        ("sound", "sound_note", "soundNote", "Sound", 500),
    ] {
        let old = rv::column(ctx.conn, "shot", col, &sh.id)?;
        changed |= set_text(&mut d, &mut op, op_key, label, a.raw(key), &old, max)?;
    }
    let chars = a.list("characters");
    if !chars.is_empty() {
        d.row("Characters", chars.join(", "));
        op["characters"] = json!(chars);
        changed = true;
    }
    need_change(changed, &format!("shot {n} of {}", s.label()))?;
    d.op("shot.update", op, format!("Edit shot {n} of {}", s.label()));
    d.done()
}

fn shot_move(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed shot move");
    let s = scene(ctx, a.u("sceneNumber").unwrap_or(0), &mut d)?;
    let n = a.u("shot").unwrap_or(1);
    let sh = shot(ctx, &s, n, &mut d)?;
    let position = a.u("toPosition").map(|p| p.max(1) - 1);
    match a.u("toSceneNumber") {
        Some(t) if t as usize != s.number => {
            let to = scene(ctx, t, &mut d)?;
            let mut op = json!({"id": sh.id, "sceneId": to.id});
            if let Some(i) = position {
                op["index"] = json!(i);
            }
            d.row("Move", format!("Shot {n} of {}", s.label()))
                .row("To", to.label());
            d.op("shot.move", op, format!("Move shot {n} to {}", to.label()));
        }
        _ => {
            let i = position.ok_or_else(|| {
                queries::ambiguous("Where should the shot go? Give a position or another scene.")
            })?;
            d.row("Move", format!("Shot {n} of {}", s.label()))
                .row("To position", (i + 1).to_string());
            d.op(
                "shot.reorder",
                json!({"id": sh.id, "index": i}),
                format!("Move shot {n} to position {}", i + 1),
            );
        }
    }
    d.done()
}

fn shot_duplicate(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed shot copy");
    let s = scene(ctx, a.u("sceneNumber").unwrap_or(0), &mut d)?;
    let n = a.u("shot").unwrap_or(1);
    let sh = shot(ctx, &s, n, &mut d)?;
    d.row(
        "Duplicate",
        format!(
            "Shot {n} of {} — {}",
            s.label(),
            queries::truncate_chars(&sh.label, 80)
        ),
    );
    d.op(
        "shot.duplicate",
        json!({"id": sh.id}),
        format!("Duplicate shot {n} of {}", s.label()),
    );
    d.done()
}

fn shots_delete(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed shot deletion");
    let s = scene(ctx, a.u("sceneNumber").unwrap_or(0), &mut d)?;
    let mut ids = Vec::new();
    let mut nums = Vec::new();
    for n in a.ints("shots") {
        let sh = shot(ctx, &s, n, &mut d)?;
        if !ids.contains(&sh.id) {
            ids.push(sh.id);
            nums.push(n.to_string());
        }
    }
    d.row(
        "Delete shots",
        format!("{} of {}", queries::join_and(&nums), s.label()),
    )
    .row("Recoverable", "Yes — from Recently Deleted");
    d.op(
        "shot.delete",
        json!({"ids": ids}),
        format!(
            "Delete {} of {}",
            plural(nums.len(), "shot", "shots"),
            s.label()
        ),
    );
    d.done()
}

fn panel_from_shot(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed panel for shot");
    let s = scene(ctx, a.u("sceneNumber").unwrap_or(0), &mut d)?;
    let n = a.u("shot").unwrap_or(1);
    let sh = shot(ctx, &s, n, &mut d)?;
    d.row("Create panel for", format!("Shot {n} of {}", s.label()))
        .row("Panel", "Empty (add the image yourself)");
    d.op(
        "shot.create_panel",
        json!({"id": sh.id}),
        format!("Create a panel for shot {n}"),
    );
    d.done()
}

fn copy_planning(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed planning copy");
    let from = scene(ctx, a.u("fromSceneNumber").unwrap_or(0), &mut d)?;
    let to = scene(ctx, a.u("toSceneNumber").unwrap_or(0), &mut d)?;
    if from.id == to.id {
        return Err(queries::ambiguous(
            "Those are the same scene. Which scene should receive the copy?",
        ));
    }
    let shots = queries::count(
        ctx.conn,
        "SELECT count(*) FROM shot WHERE scene_lineage_id=?1 AND deleted_at IS NULL",
        [&from.lineage_id],
    )?;
    if shots == 0 {
        return Err(AppError::ai(
            "not_found",
            format!("{} has no shots to copy.", from.label()),
        ));
    }
    d.row("Copy from", from.label())
        .row("To", to.label())
        .row("Shots", shots.to_string());
    d.op(
        "shot.copy_planning",
        json!({"fromSceneLineageId": from.lineage_id, "toSceneId": to.id}),
        format!("Copy planning from {} to {}", from.label(), to.label()),
    );
    d.done()
}
