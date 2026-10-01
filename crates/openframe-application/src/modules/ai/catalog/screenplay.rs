//! Proposal tools for the Screenplay: drafts, revisions, history points,
//! scene-level changes, screenplay lines, find/replace, review rounds,
//! character-cue links and the title page. Locked drafts are refused up front
//! (they only change through a revision), and earlier drafts are never
//! silently rewritten.

use openframe_domain::enums::ElementType;
use openframe_domain::{AppError, AppResult, Capability};
use rusqlite::OptionalExtension;
use serde_json::{Value, json};

use super::super::queries::{self, DraftRef, SceneRow};
use super::super::toolbox::resolve::{self as rv, Found};
use super::super::toolbox::schema as sc;
use super::super::types::ChangeSetDraft;
use super::kit::{A, Draft, need_change, plural, set_text};
use super::workspace::{neutral_scope, spec};
use super::{PropCtx, ProposalSpec};

const ELEMENT_TYPES: &[&str] = &[
    "scene_heading",
    "action",
    "character",
    "dialogue",
    "parenthetical",
    "transition",
    "shot",
    "note",
];

pub const SPECS: &[ProposalSpec] = &[
    spec!(
        "propose_screenplay",
        "Prepare a new screenplay document with its first draft.",
        ["screenplay.create"],
        [],
        Edit,
        "creating screenplays",
        "Screenplay",
        s_screenplay,
        screenplay
    ),
    spec!(
        "propose_new_draft",
        "Prepare a new draft copied from an existing draft (becomes current unless makeCurrent is false).",
        ["screenplay.new_draft"],
        [],
        Edit,
        "creating drafts",
        "Screenplay",
        s_new_draft,
        new_draft
    ),
    spec!(
        "propose_rename_draft",
        "Prepare renaming a draft or editing its note.",
        ["screenplay.rename_draft"],
        [],
        Edit,
        "editing drafts",
        "Screenplay",
        s_rename_draft,
        rename_draft
    ),
    spec!(
        "propose_current_draft",
        "Prepare making a draft the Current draft.",
        ["screenplay.set_current_draft"],
        [],
        Edit,
        "editing drafts",
        "Screenplay",
        s_draft_ref,
        current_draft
    ),
    spec!(
        "propose_restore_draft",
        "Prepare restoring an earlier draft as a NEW draft (the earlier draft is kept).",
        ["screenplay.restore_draft"],
        [],
        Edit,
        "restoring drafts",
        "Screenplay",
        s_draft_ref,
        restore_draft
    ),
    spec!(
        "propose_delete_draft",
        "Prepare moving a draft to Recently Deleted.",
        ["screenplay.delete_draft"],
        [],
        SoftDelete,
        "deleting drafts",
        "Screenplay",
        s_draft_ref,
        delete_draft
    ),
    spec!(
        "propose_lock_draft",
        "Prepare locking a draft as the shooting draft.",
        ["screenplay.lock_draft"],
        [],
        LockOrFinalize,
        "locking drafts",
        "Screenplay",
        s_draft_ref,
        lock_draft
    ),
    spec!(
        "propose_revision",
        "Prepare starting a revision of a locked draft (a new Revision draft with a label and colour).",
        ["screenplay.start_revision"],
        [],
        Edit,
        "starting revisions",
        "Screenplay",
        s_revision,
        revision
    ),
    spec!(
        "propose_update_revision",
        "Prepare changing a revision's label, colour or reason.",
        ["screenplay.update_revision"],
        [],
        Edit,
        "editing revisions",
        "Screenplay",
        s_update_revision,
        update_revision
    ),
    spec!(
        "propose_restore_history_point",
        "Prepare restoring an automatic history point of a draft as a NEW draft (1 = most recent point).",
        ["screenplay.restore_history_point"],
        [],
        Edit,
        "restoring history points",
        "Screenplay",
        s_history_point,
        restore_history_point
    ),
    spec!(
        "propose_screenplay_lines",
        "Prepare new screenplay lines (action, character, dialogue, …) in a scene, after a given line or at the end.",
        ["screenplay.insert_element"],
        [],
        Edit,
        "editing the screenplay",
        "Screenplay",
        s_lines,
        insert_lines
    ),
    spec!(
        "propose_edit_screenplay_line",
        "Prepare changing the text or type of one screenplay line (line numbers count from 1 within the scene).",
        ["screenplay.update_element"],
        [],
        Edit,
        "editing the screenplay",
        "Screenplay",
        s_edit_line,
        edit_line
    ),
    spec!(
        "propose_delete_screenplay_line",
        "Prepare deleting one screenplay line of a scene.",
        ["screenplay.delete_element"],
        [],
        Edit,
        "editing the screenplay",
        "Screenplay",
        s_line_ref,
        delete_line
    ),
    spec!(
        "propose_new_scene",
        "Prepare a new scene with a heading (at a position, or at the end of the draft).",
        ["screenplay.create_scene"],
        [],
        Edit,
        "editing the screenplay",
        "Screenplay",
        s_new_scene,
        new_scene
    ),
    spec!(
        "propose_update_scene",
        "Prepare changes to a scene's heading, synopsis, notes, Story Day or time note.",
        ["screenplay.update_scene"],
        [],
        Edit,
        "editing the screenplay",
        "Screenplay",
        s_update_scene,
        update_scene
    ),
    spec!(
        "propose_move_scene",
        "Prepare moving a scene to another position in its draft (scenes renumber).",
        ["screenplay.move_scene"],
        [],
        Edit,
        "editing the screenplay",
        "Screenplay",
        s_move_scene,
        move_scene
    ),
    spec!(
        "propose_delete_scene",
        "Prepare deleting a scene from a draft.",
        ["screenplay.delete_scene"],
        [],
        Edit,
        "editing the screenplay",
        "Screenplay",
        s_scene_ref,
        delete_scene
    ),
    spec!(
        "propose_replace_text",
        "Prepare a find-and-replace throughout a draft's text.",
        ["screenplay.replace_all"],
        [],
        Edit,
        "editing the screenplay",
        "Screenplay",
        s_replace,
        replace_text
    ),
    spec!(
        "propose_review_round",
        "Prepare starting a review round on a draft.",
        ["screenplay.start_review"],
        [],
        Edit,
        "starting reviews",
        "Screenplay",
        s_review,
        review_round
    ),
    spec!(
        "propose_update_review_round",
        "Prepare changing a review round's name, reviewers or deadline.",
        ["screenplay.update_review"],
        [],
        Edit,
        "editing reviews",
        "Screenplay",
        s_update_review,
        update_review_round
    ),
    spec!(
        "propose_complete_review_round",
        "Prepare completing a review round.",
        ["screenplay.complete_review"],
        [],
        ResolveComments,
        "completing reviews",
        "Screenplay",
        s_review_ref,
        complete_review_round
    ),
    spec!(
        "propose_link_character_cue",
        "Prepare linking a screenplay character cue (e.g. \"RAVI\") to a Character record, marking it not a character, or clearing the link.",
        [
            "screenplay.set_character_link",
            "screenplay.clear_character_link"
        ],
        [],
        Edit,
        "linking character cues",
        "Screenplay",
        s_cue_link,
        cue_link
    ),
    spec!(
        "propose_title_page",
        "Prepare changes to the screenplay title page.",
        ["screenplay.update_title_page"],
        [],
        Edit,
        "editing the title page",
        "Screenplay",
        s_title_page,
        title_page
    ),
];

// ------------------------------------------------------------------ schemas

fn s_screenplay() -> Value {
    sc::obj(
        &[
            ("title", sc::s(200)),
            ("episode", sc::reference()),
            ("draftName", sc::s(120)),
        ],
        &[],
    )
}
fn s_new_draft() -> Value {
    sc::obj(
        &[
            ("from", sc::s(120)),
            ("name", sc::s(120)),
            ("note", sc::s(1000)),
            ("makeCurrent", sc::boolean()),
        ],
        &["name"],
    )
}
fn s_rename_draft() -> Value {
    sc::obj(
        &[
            ("draft", sc::s(120)),
            ("name", sc::s(120)),
            ("note", sc::s(1000)),
        ],
        &["draft"],
    )
}
fn s_draft_ref() -> Value {
    sc::obj(&[("draft", sc::s(120))], &["draft"])
}
fn s_revision() -> Value {
    sc::obj(
        &[
            ("draft", sc::s(120)),
            ("label", sc::s(80)),
            ("color", sc::s(40)),
            ("reason", sc::s(1000)),
        ],
        &["draft", "label"],
    )
}
fn s_update_revision() -> Value {
    sc::obj(
        &[
            ("draft", sc::s(120)),
            ("label", sc::s(80)),
            ("color", sc::s(40)),
            ("reason", sc::s(1000)),
        ],
        &["draft"],
    )
}
fn s_history_point() -> Value {
    sc::obj(
        &[("draft", sc::s(120)), ("point", sc::int(1, 1000))],
        &["point"],
    )
}
fn line() -> Value {
    sc::obj(
        &[("type", sc::en(ELEMENT_TYPES)), ("text", sc::s(4000))],
        &["type", "text"],
    )
}
fn s_lines() -> Value {
    sc::obj(
        &[
            ("sceneNumber", sc::scene_number()),
            ("draft", sc::s(120)),
            ("lines", sc::arr(line(), 40)),
            ("afterLine", sc::int(0, 100_000)),
        ],
        &["sceneNumber", "lines"],
    )
}
fn s_edit_line() -> Value {
    sc::obj(
        &[
            ("sceneNumber", sc::scene_number()),
            ("draft", sc::s(120)),
            ("line", sc::int(1, 100_000)),
            ("text", sc::s(4000)),
            ("type", sc::en(ELEMENT_TYPES)),
        ],
        &["sceneNumber", "line"],
    )
}
fn s_line_ref() -> Value {
    sc::obj(
        &[
            ("sceneNumber", sc::scene_number()),
            ("draft", sc::s(120)),
            ("line", sc::int(1, 100_000)),
        ],
        &["sceneNumber", "line"],
    )
}
fn s_new_scene() -> Value {
    sc::obj(
        &[
            ("heading", sc::s(200)),
            ("position", sc::int(1, 100_000)),
            ("draft", sc::s(120)),
        ],
        &["heading"],
    )
}
fn s_update_scene() -> Value {
    sc::obj(
        &[
            ("sceneNumber", sc::scene_number()),
            ("draft", sc::s(120)),
            ("heading", sc::s(200)),
            ("synopsis", sc::s(2000)),
            ("notes", sc::s(4000)),
            ("storyDay", sc::s(60)),
            ("timeNote", sc::s(200)),
        ],
        &["sceneNumber"],
    )
}
fn s_move_scene() -> Value {
    sc::obj(
        &[
            ("sceneNumber", sc::scene_number()),
            ("toPosition", sc::int(1, 100_000)),
            ("draft", sc::s(120)),
        ],
        &["sceneNumber", "toPosition"],
    )
}
fn s_scene_ref() -> Value {
    sc::obj(
        &[("sceneNumber", sc::scene_number()), ("draft", sc::s(120))],
        &["sceneNumber"],
    )
}
fn s_replace() -> Value {
    sc::obj(
        &[
            ("find", sc::s(200)),
            ("replace", sc::s(200)),
            ("matchCase", sc::boolean()),
            ("wholeWord", sc::boolean()),
            ("draft", sc::s(120)),
        ],
        &["find", "replace"],
    )
}
fn s_review() -> Value {
    sc::obj(
        &[
            ("draft", sc::s(120)),
            ("name", sc::s(120)),
            ("reviewers", sc::arr(sc::s(120), 20)),
            ("deadline", sc::sd(10, "YYYY-MM-DD")),
        ],
        &["name", "reviewers"],
    )
}
fn s_update_review() -> Value {
    sc::obj(
        &[
            ("round", sc::reference()),
            ("name", sc::s(120)),
            ("reviewers", sc::arr(sc::s(120), 20)),
            ("deadline", sc::sd(10, "YYYY-MM-DD, or empty to clear")),
        ],
        &["round"],
    )
}
fn s_review_ref() -> Value {
    sc::obj(&[("round", sc::reference())], &["round"])
}
fn s_cue_link() -> Value {
    sc::obj(
        &[
            ("cue", sc::s(120)),
            ("character", sc::reference()),
            ("notACharacter", sc::boolean()),
            ("clear", sc::boolean()),
            ("draft", sc::s(120)),
        ],
        &["cue"],
    )
}
fn s_title_page() -> Value {
    sc::obj(
        &[
            ("draft", sc::s(120)),
            ("title", sc::s(200)),
            ("writtenBy", sc::s(300)),
            ("contact", sc::s(1000)),
            ("draftLine", sc::s(200)),
            ("notes", sc::s(2000)),
        ],
        &[],
    )
}

// ------------------------------------------------------------------ helpers

fn draft_of(ctx: &PropCtx<'_>, a: &A<'_>, d: &mut Draft) -> AppResult<DraftRef> {
    let scope = ctx.scope.cloned().unwrap_or_else(neutral_scope);
    let draft = rv::draft(ctx.conn, &scope, a.s("draft").as_deref())?;
    d.pin("draft", json!(draft.name));
    d.base("screenplay_draft", &draft.id);
    Ok(draft)
}

/// A draft whose text may change: locked drafts only change through a revision.
fn editable(draft: &DraftRef) -> AppResult<()> {
    if draft.locked() {
        return Err(AppError::ai(
            "locked",
            format!(
                "{} is locked, so its text can't change. Start a revision to make changes.",
                draft.label()
            ),
        ));
    }
    Ok(())
}

fn scene_of(ctx: &PropCtx<'_>, a: &A<'_>, d: &mut Draft, draft: &DraftRef) -> AppResult<SceneRow> {
    let n = a
        .u("sceneNumber")
        .ok_or_else(|| queries::ambiguous("Which scene? Tell me its number."))?;
    let s = rv::scene(ctx.conn, draft, n)?;
    d.target(
        "screenplay_scene",
        &Found {
            id: s.id.clone(),
            label: s.label(),
            rev: s.rev,
        },
    );
    Ok(s)
}

/// Elements of a scene in order: (id, type, text, rev).
fn lines(ctx: &PropCtx<'_>, scene_id: &str) -> AppResult<Vec<(String, String, String, i64)>> {
    let mut stmt = ctx.conn.prepare(
        "SELECT id, element_type, text, rev FROM screenplay_element WHERE scene_id=?1 ORDER BY position, id",
    )?;
    let rows = stmt
        .query_map([scene_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

fn line_at(
    ctx: &PropCtx<'_>,
    scene: &SceneRow,
    n: u32,
) -> AppResult<(String, String, String, i64)> {
    let all = lines(ctx, &scene.id)?;
    let count = all.len();
    all.into_iter()
        .nth((n as usize).saturating_sub(1))
        .filter(|_| n >= 1)
        .ok_or_else(|| {
            AppError::ai(
                "not_found",
                format!("{} has {count} lines; there is no line {n}.", scene.label()),
            )
        })
}

fn element(t: &str) -> AppResult<ElementType> {
    ElementType::parse(t).ok_or_else(|| {
        queries::ambiguous("Which kind of line is it (action, character, dialogue, …)?")
    })
}

// ------------------------------------------------------------------ drafts

fn screenplay(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed screenplay");
    let mut op = json!({});
    if let Some(t) = a.s("title") {
        op["title"] = json!(t);
        d.row("Title", t);
    }
    let episode = rv::find_opt(ctx.conn, ctx.actor, &rv::EPISODE, a.s("episode").as_deref())?;
    let exists = queries::count(
        ctx.conn,
        "SELECT count(*) FROM screenplay WHERE deleted_at IS NULL AND episode_id IS ?1",
        [episode.as_ref().map(|e| e.id.clone())],
    )?;
    if exists > 0 {
        return Err(queries::ambiguous(
            "A screenplay already exists here. Should I create a new draft of it instead?",
        ));
    }
    if let Some(e) = episode {
        d.base("episode", &e.id).row("Episode", e.label.clone());
        op["episodeId"] = json!(e.id);
    }
    if let Some(n) = a.s("draftName") {
        op["draftName"] = json!(n);
        d.row("First draft", n);
    }
    d.row("Impact", "A new screenplay with an empty first draft");
    d.op("screenplay.create", op, "Create screenplay");
    d.done()
}

fn new_draft(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let scope = ctx.scope.cloned().unwrap_or_else(neutral_scope);
    let src = rv::draft(ctx.conn, &scope, a.s("from").as_deref())?;
    let name = a.req("name", "new draft's name")?;
    let mut d = Draft::new(spec, args, "Proposed new draft");
    d.pin("from", json!(src.name))
        .base("screenplay_draft", &src.id);
    let mut op = json!({"sourceDraftId": src.id, "name": name});
    if let Some(n) = a.s("note") {
        op["note"] = json!(n);
    }
    let current = a.b("makeCurrent").unwrap_or(true);
    op["makeCurrent"] = json!(current);
    d.row("New draft", name.clone())
        .row("Copied from", src.label())
        .row("Becomes current", if current { "Yes" } else { "No" });
    d.op("screenplay.new_draft", op, format!("Create draft “{name}”"));
    d.done()
}

fn rename_draft(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed draft rename");
    let draft = draft_of(ctx, &a, &mut d)?;
    let mut op = json!({"draftId": draft.id, "name": draft.name});
    let mut changed = set_text(
        &mut d,
        &mut op,
        "name",
        "Draft name",
        a.s("name"),
        &draft.name,
        120,
    )?;
    let note = rv::column(ctx.conn, "screenplay_draft", "note", &draft.id)?;
    changed |= set_text(&mut d, &mut op, "note", "Note", a.raw("note"), &note, 1000)?;
    need_change(changed, &draft.label())?;
    d.op(
        "screenplay.rename_draft",
        op,
        format!("Rename draft “{}”", draft.label()),
    );
    d.done()
}

fn simple_draft_op(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
    title: &str,
    op: &str,
    row: &str,
    extra: Option<(&str, &str)>,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, title);
    let draft = draft_of(ctx, &a, &mut d)?;
    d.row(row, draft.label());
    if let Some((l, v)) = extra {
        d.row(l, v);
    }
    d.op(
        op,
        json!({"draftId": draft.id}),
        format!("{row} {}", draft.label()),
    );
    d.done()
}

fn current_draft(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    simple_draft_op(
        ctx,
        spec,
        args,
        "Proposed current draft",
        "screenplay.set_current_draft",
        "Make current",
        None,
    )
}

fn restore_draft(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    simple_draft_op(
        ctx,
        spec,
        args,
        "Proposed draft restore",
        "screenplay.restore_draft",
        "Restore as a new draft",
        Some(("Earlier draft", "Kept unchanged")),
    )
}

fn delete_draft(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    simple_draft_op(
        ctx,
        spec,
        args,
        "Proposed draft deletion",
        "screenplay.delete_draft",
        "Delete",
        Some(("Recoverable", "Yes — from Recently Deleted")),
    )
}

fn lock_draft(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed lock");
    let draft = draft_of(ctx, &a, &mut d)?;
    if draft.locked() {
        return Err(AppError::ai(
            "locked",
            format!("{} is already locked.", draft.label()),
        ));
    }
    let open = queries::count(
        ctx.conn,
        "SELECT count(*) FROM comment c JOIN screenplay_scene s ON s.id = c.scene_id
         WHERE s.draft_id=?1 AND c.deleted_at IS NULL AND c.status <> 'Resolved' AND c.parent_id IS NULL",
        [&draft.id],
    )?;
    d.row("Lock", draft.label())
        .row("After locking", "Text changes need a revision");
    if open > 0 {
        d.row(
            "Open comments",
            plural(open as usize, "unresolved comment", "unresolved comments"),
        );
    }
    d.op(
        "screenplay.lock_draft",
        json!({"draftId": draft.id}),
        format!("Lock {}", draft.label()),
    );
    d.done()
}

fn revision(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed revision");
    let draft = draft_of(ctx, &a, &mut d)?;
    if !draft.locked() {
        return Err(AppError::ai(
            "not_locked",
            format!(
                "{} isn't locked; revisions start from a locked draft. Edit it directly instead.",
                draft.label()
            ),
        ));
    }
    let label = a.req("label", "revision label")?;
    let mut op = json!({"draftId": draft.id, "label": label});
    for key in ["color", "reason"] {
        if let Some(v) = a.s(key) {
            op[key] = json!(v);
        }
    }
    d.row("Revision", label.clone())
        .row("Of", draft.label())
        .row("Creates", "A new Revision draft");
    d.op(
        "screenplay.start_revision",
        op,
        format!("Start revision “{label}”"),
    );
    d.done()
}

fn update_revision(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed revision change");
    let draft = draft_of(ctx, &a, &mut d)?;
    let mut op = json!({"draftId": draft.id});
    let mut changed = false;
    for (key, col, label, max) in [
        ("label", "revision_label", "Label", 80usize),
        ("color", "revision_color", "Colour", 40),
        ("reason", "revision_reason", "Reason", 1000),
    ] {
        let old = rv::column(ctx.conn, "screenplay_draft", col, &draft.id)?;
        changed |= set_text(&mut d, &mut op, key, label, a.raw(key), &old, max)?;
    }
    need_change(changed, &draft.label())?;
    d.op(
        "screenplay.update_revision",
        op,
        format!("Edit revision of {}", draft.label()),
    );
    d.done()
}

fn restore_history_point(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed history restore");
    let draft = draft_of(ctx, &a, &mut d)?;
    let n = a.u("point").unwrap_or(1).max(1);
    let point: Option<(String, i64, i64)> = ctx
        .conn
        .query_row(
            "SELECT id, created_at, scene_count FROM screenplay_history_point WHERE draft_id=?1
             ORDER BY created_at DESC, id DESC LIMIT 1 OFFSET ?2",
            rusqlite::params![draft.id, (n - 1) as i64],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let (id, at, scenes) = point.ok_or_else(|| {
        AppError::ai(
            "not_found",
            format!("{} doesn't have history point {n}.", draft.label()),
        )
    })?;
    let when = time::OffsetDateTime::from_unix_timestamp(at / 1000)
        .map(|t| format!("{} {:02}:{:02} UTC", t.date(), t.hour(), t.minute()))
        .unwrap_or_default();
    d.row(
        "Restore",
        format!(
            "History point {n} of {} ({when}, {scenes} scenes)",
            draft.label()
        ),
    )
    .row("Creates", "A new draft; the current text is kept");
    d.op(
        "screenplay.restore_history_point",
        json!({"historyPointId": id}),
        "Restore history point as a new draft",
    );
    d.done()
}

// ------------------------------------------------------------------ lines & scenes

fn insert_lines(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed screenplay lines");
    let draft = draft_of(ctx, &a, &mut d)?;
    editable(&draft)?;
    let scene = scene_of(ctx, &a, &mut d, &draft)?;
    let existing = lines(ctx, &scene.id)?.len();
    let after = a
        .u("afterLine")
        .map(|n| (n as usize).min(existing))
        .unwrap_or(existing);
    let items = args
        .get("lines")
        .and_then(|l| l.as_array())
        .cloned()
        .unwrap_or_default();
    if items.is_empty() {
        return Err(queries::ambiguous("Which lines should I add?"));
    }
    for (i, it) in items.iter().enumerate() {
        let t = element(it.get("type").and_then(|v| v.as_str()).unwrap_or(""))?;
        let text = it
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if text.is_empty() {
            return Err(queries::ambiguous(
                "One of the lines is empty. What should it say?",
            ));
        }
        d.row(
            format!("{} (new)", t.as_str().replace('_', " ")),
            queries::truncate_chars(&text, 200),
        );
        d.op(
            "screenplay.insert_element",
            json!({"sceneId": scene.id, "index": after + i, "elementType": t.as_str(), "text": text}),
            format!("Add a {} line to {}", t.as_str().replace('_', " "), scene.label()),
        );
    }
    d.row(
        "Where",
        if after == existing {
            format!("End of {}", scene.label())
        } else {
            format!("{} after line {after}", scene.label())
        },
    );
    d.done()
}

fn edit_line(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed screenplay edit");
    let draft = draft_of(ctx, &a, &mut d)?;
    editable(&draft)?;
    let scene = scene_of(ctx, &a, &mut d, &draft)?;
    let n = a.u("line").unwrap_or(1);
    let (id, kind, text, rev) = line_at(ctx, &scene, n)?;
    d.target(
        "screenplay_element",
        &Found {
            id: id.clone(),
            label: format!("{} line {n}", scene.label()),
            rev,
        },
    );
    let mut op = json!({"id": id});
    let mut changed = set_text(
        &mut d,
        &mut op,
        "text",
        &format!("Line {n} ({kind})"),
        a.s("text"),
        &text,
        4000,
    )?;
    if let Some(t) = a.s("type") {
        let t = element(&t)?;
        if t.as_str() != kind {
            d.change("Line type", &kind, t.as_str());
            op["elementType"] = json!(t.as_str());
            changed = true;
        }
    }
    need_change(changed, &format!("line {n}"))?;
    d.op(
        "screenplay.update_element",
        op,
        format!("Edit line {n} of {}", scene.label()),
    );
    d.done()
}

fn delete_line(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed line deletion");
    let draft = draft_of(ctx, &a, &mut d)?;
    editable(&draft)?;
    let scene = scene_of(ctx, &a, &mut d, &draft)?;
    let n = a.u("line").unwrap_or(1);
    let (id, kind, text, rev) = line_at(ctx, &scene, n)?;
    d.target(
        "screenplay_element",
        &Found {
            id: id.clone(),
            label: format!("{} line {n}", scene.label()),
            rev,
        },
    );
    d.row(
        format!("Delete line {n} ({kind})"),
        queries::truncate_chars(&text, 200),
    );
    d.op(
        "screenplay.delete_element",
        json!({"id": id}),
        format!("Delete line {n} of {}", scene.label()),
    );
    d.done()
}

fn new_scene(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed new scene");
    let draft = draft_of(ctx, &a, &mut d)?;
    editable(&draft)?;
    let heading = a.req("heading", "scene heading")?;
    let count = queries::scenes(ctx.conn, &draft.id)?.len();
    let mut op = json!({"draftId": draft.id, "heading": heading});
    let place = match a.u("position") {
        Some(p) if (p as usize) <= count => {
            op["index"] = json!(p - 1);
            format!("As Scene {p} (later scenes renumber)")
        }
        _ => format!("At the end, as Scene {}", count + 1),
    };
    d.row("Scene heading", heading.clone())
        .row("Place", place)
        .row("Draft", draft.label());
    d.op(
        "screenplay.create_scene",
        op,
        format!("Add scene “{heading}”"),
    );
    d.done()
}

fn update_scene(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed scene change");
    let draft = draft_of(ctx, &a, &mut d)?;
    let scene = scene_of(ctx, &a, &mut d, &draft)?;
    let mut op = json!({"sceneId": scene.id});
    let mut changed = false;
    if a.s("heading").is_some() {
        editable(&draft)?;
    }
    for (key, col, label, max) in [
        ("heading", "heading", "Heading", 200usize),
        ("synopsis", "synopsis", "Synopsis", 2000),
        ("notes", "notes", "Scene note", 4000),
        ("storyDay", "story_day", "Story Day", 60),
        ("timeNote", "time_note", "Time note", 200),
    ] {
        let old = rv::column(ctx.conn, "screenplay_scene", col, &scene.id)?;
        let new = if key == "heading" {
            a.s(key)
        } else {
            a.raw(key)
        };
        changed |= set_text(&mut d, &mut op, key, label, new, &old, max)?;
    }
    need_change(changed, &scene.label())?;
    d.op(
        "screenplay.update_scene",
        op,
        format!("Edit {}", scene.label()),
    );
    d.done()
}

fn move_scene(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed scene move");
    let draft = draft_of(ctx, &a, &mut d)?;
    editable(&draft)?;
    let scene = scene_of(ctx, &a, &mut d, &draft)?;
    let count = queries::scenes(ctx.conn, &draft.id)?.len();
    let to = a.u("toPosition").unwrap_or(1).clamp(1, count.max(1) as u32);
    if to as usize == scene.number {
        return Err(queries::ambiguous(format!(
            "{} is already at position {to}. Where should it go?",
            scene.label()
        )));
    }
    d.row("Move", scene.label())
        .row("To position", to.to_string())
        .row(
            "Scene numbers",
            "Scenes in between renumber (identity is kept)",
        );
    d.op(
        "screenplay.move_scene",
        json!({"sceneId": scene.id, "index": to - 1}),
        format!("Move {} to position {to}", scene.label()),
    );
    d.done()
}

fn delete_scene(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed scene deletion");
    let draft = draft_of(ctx, &a, &mut d)?;
    editable(&draft)?;
    let scene = scene_of(ctx, &a, &mut d, &draft)?;
    let n = lines(ctx, &scene.id)?.len();
    d.row("Delete", scene.label())
        .row("Its text", plural(n, "line", "lines"))
        .row("Other drafts", "Not changed");
    d.op(
        "screenplay.delete_scene",
        json!({"sceneId": scene.id}),
        format!("Delete {}", scene.label()),
    );
    d.done()
}

fn replace_text(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed find and replace");
    let draft = draft_of(ctx, &a, &mut d)?;
    editable(&draft)?;
    let find = a.req("find", "text to find")?;
    let replace = a.raw("replace").unwrap_or_default();
    let match_case = a.flag("matchCase");
    let whole = a.flag("wholeWord");
    let mut stmt = ctx.conn.prepare(
        "SELECT e.text FROM screenplay_element e JOIN screenplay_scene s ON s.id = e.scene_id
         WHERE s.draft_id=?1 AND s.deleted_at IS NULL AND e.element_type <> 'note'",
    )?;
    let texts: Vec<String> = stmt
        .query_map([&draft.id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let count: usize = texts
        .iter()
        .map(|t| occurrences(t, &find, match_case, whole))
        .sum();
    if count == 0 {
        return Err(AppError::ai(
            "not_found",
            format!("“{find}” doesn't appear in {}.", draft.label()),
        ));
    }
    d.row("Replace", format!("“{find}” → “{replace}”"))
        .row("In", draft.label())
        .row("Occurrences", count.to_string())
        .row("Other drafts", "Not changed");
    d.op(
        "screenplay.replace_all",
        json!({"draftId": draft.id, "query": find, "replacement": replace, "matchCase": match_case, "wholeWord": whole, "includeNotes": false}),
        format!("Replace “{find}” in {}", draft.label()),
    );
    d.done()
}

fn occurrences(text: &str, needle: &str, match_case: bool, whole: bool) -> usize {
    let (h, n) = if match_case {
        (text.to_string(), needle.to_string())
    } else {
        (text.to_lowercase(), needle.to_lowercase())
    };
    if n.is_empty() {
        return 0;
    }
    let mut count = 0;
    let mut start = 0;
    while let Some(pos) = h[start..].find(&n) {
        let abs = start + pos;
        let ok = !whole
            || (!h[..abs]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric())
                && !h[abs + n.len()..]
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_alphanumeric()));
        if ok {
            count += 1;
        }
        start = abs + n.len();
    }
    count
}

// ------------------------------------------------------------------ review & links

fn review_round(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed review round");
    let draft = draft_of(ctx, &a, &mut d)?;
    let name = a.req("name", "review round name")?;
    let reviewers = a.list("reviewers");
    if reviewers.is_empty() {
        return Err(queries::ambiguous("Who should review it?"));
    }
    let mut op = json!({"draftId": draft.id, "name": name, "reviewers": reviewers});
    if let Some(dl) = a.s("deadline") {
        op["deadline"] = json!(super::kit::date_text(&dl)?);
        d.row("Deadline", dl);
    }
    d.row("Review round", name.clone())
        .row("Draft", draft.label())
        .row("Reviewers", reviewers.join(", "));
    d.op(
        "screenplay.start_review",
        op,
        format!("Start review “{name}”"),
    );
    d.done()
}

fn round(ctx: &PropCtx<'_>, a: &A<'_>, d: &mut Draft) -> AppResult<Found> {
    let r = rv::find(
        ctx.conn,
        ctx.actor,
        &rv::REVIEW_ROUND,
        &a.req("round", "review round")?,
    )?;
    d.target("review_round", &r);
    Ok(r)
}

fn update_review_round(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed review round change");
    let r = round(ctx, &a, &mut d)?;
    let mut op = json!({"reviewRoundId": r.id});
    let mut changed = set_text(&mut d, &mut op, "name", "Name", a.s("name"), &r.label, 120)?;
    let reviewers = a.list("reviewers");
    if !reviewers.is_empty() {
        d.row("Reviewers", reviewers.join(", "));
        op["reviewers"] = json!(reviewers);
        changed = true;
    }
    if let Some(dl) = a.raw("deadline") {
        if !dl.is_empty() {
            super::kit::date_text(&dl)?;
        }
        let old = rv::column(ctx.conn, "review_round", "deadline", &r.id)?;
        changed |= set_text(&mut d, &mut op, "deadline", "Deadline", Some(dl), &old, 10)?;
    }
    need_change(changed, &format!("review round “{}”", r.label))?;
    d.op(
        "screenplay.update_review",
        op,
        format!("Edit review round “{}”", r.label),
    );
    d.done()
}

fn complete_review_round(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed review completion");
    let r = round(ctx, &a, &mut d)?;
    d.row("Complete review round", r.label.clone());
    d.op(
        "screenplay.complete_review",
        json!({"reviewRoundId": r.id}),
        format!("Complete review “{}”", r.label),
    );
    d.done()
}

fn cue_link(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed character cue link");
    let draft = draft_of(ctx, &a, &mut d)?;
    let screenplay_id: String = ctx.conn.query_row(
        "SELECT screenplay_id FROM screenplay_draft WHERE id=?1",
        [&draft.id],
        |r| r.get(0),
    )?;
    let cue = queries::normalize_cue(&a.req("cue", "character cue")?);
    let cues = queries::character_cues(ctx.conn, &draft.id)?;
    if !cues.contains_key(&cue) {
        return Err(AppError::ai(
            "not_found",
            format!("No character cue “{cue}” appears in {}.", draft.label()),
        ));
    }
    if a.flag("clear") {
        d.row("Clear link of", cue.clone());
        d.op(
            "screenplay.clear_character_link",
            json!({"screenplayId": screenplay_id, "cueName": cue}),
            format!("Clear link of {cue}"),
        );
    } else if a.flag("notACharacter") {
        d.row(cue.clone(), "Not a character (ignored in character lists)");
        d.op(
            "screenplay.set_character_link",
            json!({"screenplayId": screenplay_id, "cueName": cue, "characterId": null, "ignored": true}),
            format!("Mark {cue} as not a character"),
        );
    } else {
        let ch = rv::find(
            ctx.conn,
            ctx.actor,
            &rv::CHARACTER,
            &a.req("character", "Character record")?,
        )?;
        d.target("story_character", &ch);
        d.row(cue.clone(), format!("Linked to Character “{}”", ch.label));
        d.op(
            "screenplay.set_character_link",
            json!({"screenplayId": screenplay_id, "cueName": cue, "characterId": ch.id, "ignored": false}),
            format!("Link {cue} to {}", ch.label),
        );
    }
    d.done()
}

fn title_page(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed title page");
    let draft = draft_of(ctx, &a, &mut d)?;
    let (screenplay_id, title_page, rev): (String, String, i64) = ctx.conn.query_row(
        "SELECT s.id, s.title_page_json, s.rev FROM screenplay s JOIN screenplay_draft dr ON dr.screenplay_id = s.id WHERE dr.id=?1",
        [&draft.id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    d.target(
        "screenplay",
        &Found {
            id: screenplay_id.clone(),
            label: "Screenplay".into(),
            rev,
        },
    );
    let current: Value = serde_json::from_str(&title_page).unwrap_or_else(|_| json!({}));
    let mut page = json!({});
    let mut changed = false;
    for (key, label, max) in [
        ("title", "Title", 200usize),
        ("writtenBy", "Written by", 300),
        ("contact", "Contact", 1000),
        ("draftLine", "Draft line", 200),
        ("notes", "Notes", 2000),
    ] {
        let old = current
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        page[key] = json!(old.clone());
        changed |= set_text(&mut d, &mut page, key, label, a.raw(key), &old, max)?;
    }
    need_change(changed, "the title page")?;
    d.op(
        "screenplay.update_title_page",
        json!({"screenplayId": screenplay_id, "titlePage": page}),
        "Update the title page",
    );
    d.done()
}

#[cfg(test)]
mod tests {
    use super::occurrences;

    #[test]
    fn replace_counts_respect_case_and_words() {
        assert_eq!(
            occurrences("Ravi and RAVI and Ravindra", "ravi", false, true),
            2
        );
        assert_eq!(
            occurrences("Ravi and RAVI and Ravindra", "ravi", false, false),
            3
        );
        assert_eq!(occurrences("Ravi and RAVI", "Ravi", true, true), 1);
    }
}
