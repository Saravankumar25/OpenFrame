//! Proposal tools for Story: acts, sequences, beats, Scene Cards, the Parking
//! Lot, characters and relationships, Story Days, seasons and episodes, and
//! Build Screenplay / Apply Order (which only ever create or reorder through the
//! Story module's own rules).

use openframe_domain::{AppError, AppResult, Capability};
use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};

use super::super::queries;
use super::super::toolbox::resolve::{self as rv, Found};
use super::super::toolbox::schema as sc;
use super::super::types::ChangeSetDraft;
use super::kit::{A, Draft, need_change, plural, set_text};
use super::workspace::{neutral_scope, spec};
use super::{PropCtx, ProposalSpec};

const COLORS: &[&str] = &[
    "yellow", "orange", "red", "pink", "purple", "blue", "teal", "green", "gray",
];

pub const SPECS: &[ProposalSpec] = &[
    spec!(
        "propose_act",
        "Prepare a new act on the Story Board (optionally before another act).",
        ["story.create_act"],
        [],
        Edit,
        "changing the Story Board",
        "Story",
        s_act,
        act
    ),
    spec!(
        "propose_update_act",
        "Prepare renaming an act, editing its note, or moving it before another act.",
        ["story.update_act", "story.move_act"],
        [],
        Edit,
        "changing the Story Board",
        "Story",
        s_update_act,
        update_act
    ),
    spec!(
        "propose_delete_act",
        "Prepare deleting an act (by default its contents move to Unassigned; recoverable).",
        ["story.delete_act"],
        [],
        SoftDelete,
        "deleting Story Board items",
        "Story",
        s_delete_act,
        delete_act
    ),
    spec!(
        "propose_sequence",
        "Prepare a new sequence inside an act.",
        ["story.create_sequence"],
        [],
        Edit,
        "changing the Story Board",
        "Story",
        s_sequence,
        sequence
    ),
    spec!(
        "propose_update_sequence",
        "Prepare renaming a sequence or editing its note.",
        ["story.update_sequence"],
        [],
        Edit,
        "changing the Story Board",
        "Story",
        s_update_sequence,
        update_sequence
    ),
    spec!(
        "propose_delete_sequence",
        "Prepare deleting a sequence (by default its contents stay in the act; recoverable).",
        ["story.delete_sequence"],
        [],
        SoftDelete,
        "deleting Story Board items",
        "Story",
        s_delete_sequence,
        delete_sequence
    ),
    spec!(
        "propose_beat",
        "Prepare a new story beat (in an act or sequence; otherwise in the Parking Lot).",
        ["story.create_beat"],
        [],
        Edit,
        "changing the Story Board",
        "Story",
        s_beat,
        beat
    ),
    spec!(
        "propose_update_beat",
        "Prepare edits to a beat's text, note or colour.",
        ["story.update_beat"],
        [],
        Edit,
        "changing the Story Board",
        "Story",
        s_update_beat,
        update_beat
    ),
    spec!(
        "propose_convert_beat",
        "Prepare converting a beat into a Scene Card.",
        ["story.convert_beat"],
        [],
        Edit,
        "changing the Story Board",
        "Story",
        s_beat_ref,
        convert_beat
    ),
    spec!(
        "propose_update_scene_card",
        "Prepare edits to a Scene Card's description, scene heading, notes or colour.",
        ["story.update_card"],
        [],
        Edit,
        "changing the Story Board",
        "Story",
        s_update_card,
        update_card
    ),
    spec!(
        "propose_convert_card_to_beat",
        "Prepare converting a Scene Card back into a beat.",
        ["story.card_to_beat"],
        [],
        Edit,
        "changing the Story Board",
        "Story",
        s_card_ref,
        card_to_beat
    ),
    spec!(
        "propose_move_story_items",
        "Prepare moving Scene Cards, beats or sequences to an act, a sequence, the Parking Lot or Unassigned.",
        ["story.move_items"],
        [],
        Edit,
        "changing the Story Board",
        "Story",
        s_move_items,
        move_items
    ),
    spec!(
        "propose_duplicate_story_items",
        "Prepare duplicating Scene Cards, beats or sequences.",
        ["story.duplicate_items"],
        [],
        Edit,
        "changing the Story Board",
        "Story",
        s_items,
        duplicate_items
    ),
    spec!(
        "propose_park_story_items",
        "Prepare moving Scene Cards or beats to the Parking Lot.",
        ["story.park_items"],
        [],
        Edit,
        "changing the Story Board",
        "Story",
        s_items,
        park_items
    ),
    spec!(
        "propose_unpark_story_items",
        "Prepare returning parked Scene Cards or beats to where they came from.",
        ["story.restore_from_parking"],
        [],
        Edit,
        "changing the Story Board",
        "Story",
        s_items,
        unpark_items
    ),
    spec!(
        "propose_delete_story_items",
        "Prepare moving Scene Cards, beats or sequences to Recently Deleted.",
        ["story.delete_items"],
        [],
        SoftDelete,
        "deleting Story Board items",
        "Story",
        s_items,
        delete_items
    ),
    spec!(
        "propose_remove_story_attachment",
        "Prepare removing an attachment from a Scene Card.",
        ["story.remove_attachment"],
        [],
        Edit,
        "changing the Story Board",
        "Story",
        s_attachment,
        remove_attachment
    ),
    spec!(
        "propose_build_screenplay",
        "Prepare building a NEW screenplay or a NEW draft from the Scene Cards in Story Board order (never overwrites).",
        ["story.build_screenplay"],
        [],
        Edit,
        "building screenplays",
        "Story",
        s_build,
        build_screenplay
    ),
    spec!(
        "propose_story_order",
        "Prepare reordering a screenplay draft's scenes to match the Story Board order.",
        ["story.apply_order"],
        [],
        Edit,
        "reordering the screenplay",
        "Story",
        s_order,
        apply_order
    ),
    spec!(
        "propose_character",
        "Prepare a new Character record.",
        ["story.create_character"],
        [],
        Edit,
        "creating characters",
        "Story",
        s_character,
        character
    ),
    spec!(
        "propose_update_character",
        "Prepare edits to a character's role, description or notes (use propose_rename_character to rename).",
        ["story.update_character"],
        [],
        Edit,
        "editing characters",
        "Story",
        s_update_character,
        update_character
    ),
    spec!(
        "propose_archive_character",
        "Prepare archiving (or unarchiving) a character.",
        ["story.set_character_archived"],
        [],
        Edit,
        "editing characters",
        "Story",
        s_archive_character,
        archive_character
    ),
    spec!(
        "propose_remove_character_image",
        "Prepare removing a character's image.",
        ["story.clear_character_image"],
        [],
        Edit,
        "editing characters",
        "Story",
        s_character_ref,
        remove_character_image
    ),
    spec!(
        "propose_delete_character",
        "Prepare moving a character to Recently Deleted.",
        ["story.delete_character"],
        [],
        SoftDelete,
        "deleting characters",
        "Story",
        s_character_ref,
        delete_character
    ),
    spec!(
        "propose_relationship",
        "Prepare a relationship between two characters.",
        ["story.create_relationship"],
        [],
        Edit,
        "editing characters",
        "Story",
        s_relationship,
        relationship
    ),
    spec!(
        "propose_update_relationship",
        "Prepare changing the relationship between two characters.",
        ["story.update_relationship"],
        [],
        Edit,
        "editing characters",
        "Story",
        s_update_relationship,
        update_relationship
    ),
    spec!(
        "propose_delete_relationship",
        "Prepare removing the relationship between two characters.",
        ["story.delete_relationship"],
        [],
        Edit,
        "editing characters",
        "Story",
        s_pair,
        delete_relationship
    ),
    spec!(
        "propose_link_character_card",
        "Prepare linking a character to a Scene Card (or unlinking).",
        ["story.link_character_card", "story.unlink_character_card"],
        [],
        Edit,
        "editing characters",
        "Story",
        s_link_card,
        link_card
    ),
    spec!(
        "propose_story_day",
        "Prepare assigning screenplay scenes to a Story Day (empty storyDay clears it).",
        ["story.assign_story_day"],
        [],
        Edit,
        "changing the story timeline",
        "Story",
        s_story_day,
        story_day
    ),
    spec!(
        "propose_time_note",
        "Prepare a time note for a screenplay scene (e.g. \"three weeks later\").",
        ["story.set_time_note"],
        [],
        Edit,
        "changing the story timeline",
        "Story",
        s_time_note,
        time_note
    ),
    spec!(
        "propose_season",
        "Prepare a new season.",
        ["story.create_season"],
        [],
        Edit,
        "changing seasons and episodes",
        "Story",
        s_season,
        season
    ),
    spec!(
        "propose_update_season",
        "Prepare renaming a season, editing its note or moving it before another season.",
        ["story.update_season", "story.move_season"],
        [],
        Edit,
        "changing seasons and episodes",
        "Story",
        s_update_season,
        update_season
    ),
    spec!(
        "propose_delete_season",
        "Prepare moving a season to Recently Deleted.",
        ["story.delete_season"],
        [],
        SoftDelete,
        "deleting seasons",
        "Story",
        s_season_ref,
        delete_season
    ),
    spec!(
        "propose_episode",
        "Prepare a new episode (optionally in a season).",
        ["story.create_episode"],
        [],
        Edit,
        "changing seasons and episodes",
        "Story",
        s_episode,
        episode
    ),
    spec!(
        "propose_update_episode",
        "Prepare edits to an episode (title, summary, status) or moving it to another season.",
        ["story.update_episode", "story.move_episode"],
        [],
        Edit,
        "changing seasons and episodes",
        "Story",
        s_update_episode,
        update_episode
    ),
    spec!(
        "propose_delete_episode",
        "Prepare moving an episode to Recently Deleted.",
        ["story.delete_episode"],
        [],
        SoftDelete,
        "deleting episodes",
        "Story",
        s_episode_ref,
        delete_episode
    ),
    spec!(
        "propose_duplicate_episode",
        "Prepare duplicating an episode (optionally with its Story Board).",
        ["story.duplicate_episode"],
        [],
        Edit,
        "changing seasons and episodes",
        "Story",
        s_duplicate_episode,
        duplicate_episode
    ),
];

// ------------------------------------------------------------------ schemas

fn s_act() -> Value {
    sc::obj(
        &[
            ("title", sc::s(200)),
            ("note", sc::s(4000)),
            ("before", sc::reference()),
        ],
        &["title"],
    )
}
fn s_update_act() -> Value {
    sc::obj(
        &[
            ("act", sc::reference()),
            ("title", sc::s(200)),
            ("note", sc::s(4000)),
            ("moveBefore", sc::reference()),
            ("moveToEnd", sc::boolean()),
        ],
        &["act"],
    )
}
fn s_delete_act() -> Value {
    sc::obj(
        &[
            ("act", sc::reference()),
            ("deleteContents", sc::boolean()),
            ("moveContentsTo", sc::reference()),
        ],
        &["act"],
    )
}
fn s_sequence() -> Value {
    sc::obj(
        &[("act", sc::reference()), ("title", sc::s(200))],
        &["act", "title"],
    )
}
fn s_update_sequence() -> Value {
    sc::obj(
        &[
            ("sequence", sc::reference()),
            ("title", sc::s(200)),
            ("note", sc::s(4000)),
        ],
        &["sequence"],
    )
}
fn s_delete_sequence() -> Value {
    sc::obj(
        &[
            ("sequence", sc::reference()),
            ("deleteContents", sc::boolean()),
        ],
        &["sequence"],
    )
}
fn s_beat() -> Value {
    sc::obj(
        &[
            ("text", sc::s(2000)),
            ("act", sc::reference()),
            ("sequence", sc::reference()),
            ("color", sc::en(COLORS)),
        ],
        &["text"],
    )
}
fn s_update_beat() -> Value {
    sc::obj(
        &[
            ("beat", sc::reference()),
            ("text", sc::s(2000)),
            ("note", sc::s(4000)),
            ("color", sc::en(COLORS)),
        ],
        &["beat"],
    )
}
fn s_beat_ref() -> Value {
    sc::obj(&[("beat", sc::reference())], &["beat"])
}
fn s_update_card() -> Value {
    sc::obj(
        &[
            ("card", sc::reference()),
            ("description", sc::s(600)),
            ("heading", sc::s(200)),
            ("notes", sc::s(8000)),
            ("color", sc::en(COLORS)),
        ],
        &["card"],
    )
}
fn s_card_ref() -> Value {
    sc::obj(&[("card", sc::reference())], &["card"])
}
fn item_lists() -> Vec<(&'static str, Value)> {
    vec![
        ("cards", sc::arr(sc::reference(), 50)),
        ("beats", sc::arr(sc::reference(), 50)),
        ("sequences", sc::arr(sc::reference(), 20)),
    ]
}
fn s_items() -> Value {
    sc::obj(&item_lists(), &[])
}
fn s_move_items() -> Value {
    let mut p = item_lists();
    p.push((
        "to",
        sc::en(&["act", "sequence", "parking_lot", "unassigned"]),
    ));
    p.push(("act", sc::reference()));
    p.push(("sequence", sc::reference()));
    p.push(("beforeCard", sc::reference()));
    sc::obj(&p, &["to"])
}
fn s_attachment() -> Value {
    sc::obj(
        &[("card", sc::reference()), ("attachment", sc::s(200))],
        &["card", "attachment"],
    )
}
fn s_build() -> Value {
    sc::obj(
        &[
            ("destination", sc::en(&["new_screenplay", "new_draft"])),
            ("title", sc::s(200)),
            ("draftName", sc::s(120)),
            ("descriptionAs", sc::en(&["planning_note", "action_text"])),
            ("episode", sc::reference()),
        ],
        &["destination"],
    )
}
fn s_order() -> Value {
    sc::obj(&[("draft", sc::s(120)), ("episode", sc::reference())], &[])
}
fn s_character() -> Value {
    sc::obj(
        &[
            ("name", sc::s(120)),
            ("role", sc::s(120)),
            ("description", sc::s(4000)),
            ("notes", sc::s(8000)),
        ],
        &["name"],
    )
}
fn s_update_character() -> Value {
    sc::obj(
        &[
            ("character", sc::reference()),
            ("role", sc::s(120)),
            ("description", sc::s(4000)),
            ("notes", sc::s(8000)),
        ],
        &["character"],
    )
}
fn s_archive_character() -> Value {
    sc::obj(
        &[("character", sc::reference()), ("archived", sc::boolean())],
        &["character", "archived"],
    )
}
fn s_character_ref() -> Value {
    sc::obj(&[("character", sc::reference())], &["character"])
}
fn s_relationship() -> Value {
    sc::obj(
        &[
            ("from", sc::reference()),
            ("to", sc::reference()),
            ("type", sc::s(80)),
            ("note", sc::s(1000)),
        ],
        &["from", "to", "type"],
    )
}
fn s_update_relationship() -> Value {
    sc::obj(
        &[
            ("from", sc::reference()),
            ("to", sc::reference()),
            ("type", sc::s(80)),
            ("note", sc::s(1000)),
        ],
        &["from", "to"],
    )
}
fn s_pair() -> Value {
    sc::obj(
        &[("from", sc::reference()), ("to", sc::reference())],
        &["from", "to"],
    )
}
fn s_link_card() -> Value {
    sc::obj(
        &[
            ("character", sc::reference()),
            ("card", sc::reference()),
            ("unlink", sc::boolean()),
        ],
        &["character", "card"],
    )
}
fn s_story_day() -> Value {
    sc::obj(
        &[
            ("sceneNumbers", sc::arr(sc::scene_number(), 100)),
            ("storyDay", sc::s(60)),
            ("draft", sc::s(120)),
        ],
        &["sceneNumbers", "storyDay"],
    )
}
fn s_time_note() -> Value {
    sc::obj(
        &[
            ("sceneNumber", sc::scene_number()),
            ("timeNote", sc::s(200)),
            ("draft", sc::s(120)),
        ],
        &["sceneNumber", "timeNote"],
    )
}
fn s_season() -> Value {
    sc::obj(&[("title", sc::s(200))], &[])
}
fn s_update_season() -> Value {
    sc::obj(
        &[
            ("season", sc::reference()),
            ("title", sc::s(200)),
            ("note", sc::s(4000)),
            ("moveBefore", sc::reference()),
        ],
        &["season"],
    )
}
fn s_season_ref() -> Value {
    sc::obj(&[("season", sc::reference())], &["season"])
}
fn s_episode() -> Value {
    sc::obj(
        &[
            ("title", sc::s(200)),
            ("season", sc::reference()),
            ("summary", sc::s(4000)),
            ("status", sc::s(60)),
        ],
        &["title"],
    )
}
fn s_update_episode() -> Value {
    sc::obj(
        &[
            ("episode", sc::reference()),
            ("title", sc::s(200)),
            ("summary", sc::s(4000)),
            ("status", sc::s(60)),
            ("season", sc::reference()),
        ],
        &["episode"],
    )
}
fn s_episode_ref() -> Value {
    sc::obj(&[("episode", sc::reference())], &["episode"])
}
fn s_duplicate_episode() -> Value {
    sc::obj(
        &[("episode", sc::reference()), ("copyStory", sc::boolean())],
        &["episode"],
    )
}

// ------------------------------------------------------------------ helpers

fn find(ctx: &PropCtx<'_>, a: &A<'_>, key: &str, e: &rv::Entity) -> AppResult<Found> {
    rv::find(ctx.conn, ctx.actor, e, &a.req(key, e.what)?)
}

/// Where an act is used: sequences and cards inside it.
fn act_contents(ctx: &PropCtx<'_>, act_id: &str) -> AppResult<(i64, i64)> {
    let seqs = queries::count(
        ctx.conn,
        "SELECT count(*) FROM story_sequence WHERE act_id=?1 AND deleted_at IS NULL",
        [act_id],
    )?;
    let cards = queries::count(
        ctx.conn,
        "SELECT count(*) FROM story_scene_card WHERE deleted_at IS NULL AND ((parent_type='act' AND parent_id=?1)
           OR (parent_type='sequence' AND parent_id IN (SELECT id FROM story_sequence WHERE act_id=?1 AND deleted_at IS NULL)))",
        [act_id],
    )?;
    Ok((seqs, cards))
}

/// A Scene Card placed in a sequence (used by `propose_scene_card`).
pub(super) fn card_in_sequence(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
    sequence: &str,
    description: &str,
    heading: Option<&str>,
) -> AppResult<ChangeSetDraft> {
    let seq = rv::find(ctx.conn, ctx.actor, &rv::SEQUENCE, sequence)?;
    let description = description.trim();
    if description.is_empty() {
        return Err(queries::ambiguous(
            "What should the Scene Card description be?",
        ));
    }
    let mut d = Draft::new(spec, args, "Proposed Scene Card");
    d.summary("I prepared a suggestion. Nothing has been added yet.");
    d.target("story_sequence", &seq);
    let mut op = json!({"parent": {"parentType": "sequence", "parentId": seq.id}, "shortDescription": description});
    d.row("Description", description);
    if let Some(h) = heading.map(str::trim).filter(|h| !h.is_empty()) {
        op["sceneHeading"] = json!(h);
        d.row("Scene heading", h);
    }
    d.row("Place in", format!("Sequence “{}”", seq.label))
        .row("Impact", "1 new object");
    d.op(
        "story.create_card",
        op,
        format!(
            "Create Scene Card “{}”",
            queries::truncate_chars(description, 60)
        ),
    );
    d.done()
}

/// Items named in `cards` / `beats` / `sequences` as `{kind, id}` refs.
fn story_items(
    ctx: &PropCtx<'_>,
    a: &A<'_>,
    d: &mut Draft,
    allow_sequences: bool,
) -> AppResult<(Vec<Value>, Vec<String>)> {
    let mut refs = Vec::new();
    let mut labels = Vec::new();
    for (key, kind, e) in [
        ("cards", "card", &rv::CARD),
        ("beats", "beat", &rv::BEAT),
        ("sequences", "sequence", &rv::SEQUENCE),
    ] {
        let list = a.list(key);
        if kind == "sequence" && !allow_sequences && !list.is_empty() {
            return Err(queries::ambiguous(
                "Only Scene Cards and beats can be parked. Which cards or beats do you mean?",
            ));
        }
        for r in list {
            let f = rv::find(ctx.conn, ctx.actor, e, &r)?;
            if refs.iter().any(|x: &Value| x["id"] == json!(f.id)) {
                continue;
            }
            d.target(e.table, &f);
            refs.push(json!({"kind": kind, "id": f.id}));
            labels.push(format!(
                "{} “{}”",
                e.what,
                queries::truncate_chars(&f.label, 60)
            ));
        }
    }
    if refs.is_empty() {
        return Err(queries::ambiguous(
            "Which Scene Cards or beats do you mean?",
        ));
    }
    Ok((refs, labels))
}

fn listing(labels: &[String]) -> String {
    let shown: Vec<String> = labels.iter().take(8).cloned().collect();
    let more = labels.len().saturating_sub(8);
    if more > 0 {
        format!("{} and {more} more", shown.join(", "))
    } else {
        queries::join_and(&shown)
    }
}

// ------------------------------------------------------------------ acts & sequences

fn act(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let title = a.req("title", "act title")?;
    let mut d = Draft::new(spec, args, "Proposed act");
    let mut op = json!({"title": title});
    if let Some(n) = a.s("note") {
        op["note"] = json!(n);
        d.row("Note", n);
    }
    let place = match rv::find_opt(ctx.conn, ctx.actor, &rv::ACT, a.s("before").as_deref())? {
        Some(b) => {
            d.base("story_act", &b.id);
            op["beforeId"] = json!(b.id);
            format!("Before “{}”", b.label)
        }
        None => "At the end of the Story Board".into(),
    };
    d.row("Act", title.clone())
        .row("Place", place)
        .row("Impact", "1 new object");
    d.op("story.create_act", op, format!("Create act “{title}”"));
    d.done()
}

fn update_act(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let act = find(ctx, &a, "act", &rv::ACT)?;
    let mut d = Draft::new(spec, args, "Proposed act change");
    d.target("story_act", &act);
    let mut op = json!({"id": act.id, "expectedRev": act.rev});
    let mut changed = false;
    changed |= set_text(
        &mut d,
        &mut op,
        "title",
        "Title",
        a.s("title"),
        &act.label,
        200,
    )?;
    let note = rv::column(ctx.conn, "story_act", "note", &act.id)?;
    changed |= set_text(&mut d, &mut op, "note", "Note", a.raw("note"), &note, 4000)?;
    if changed {
        d.op("story.update_act", op, format!("Edit act “{}”", act.label));
    }
    if let Some(before) = rv::find_opt(ctx.conn, ctx.actor, &rv::ACT, a.s("moveBefore").as_deref())?
    {
        if before.id == act.id {
            return Err(queries::ambiguous(
                "An act can't be moved before itself. Where should it go?",
            ));
        }
        d.base("story_act", &before.id)
            .row("Move", format!("Before “{}”", before.label));
        d.op(
            "story.move_act",
            json!({"id": act.id, "beforeId": before.id}),
            format!("Move act “{}”", act.label),
        );
        changed = true;
    } else if a.flag("moveToEnd") {
        d.row("Move", "To the end of the Story Board");
        d.op(
            "story.move_act",
            json!({"id": act.id, "beforeId": null}),
            format!("Move act “{}”", act.label),
        );
        changed = true;
    }
    need_change(changed, &format!("act “{}”", act.label))?;
    d.done()
}

fn delete_act(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let act = find(ctx, &a, "act", &rv::ACT)?;
    let (seqs, cards) = act_contents(ctx, &act.id)?;
    let mut d = Draft::new(spec, args, "Proposed act deletion");
    d.target("story_act", &act);
    let contents = format!(
        "{} and {}",
        plural(seqs as usize, "sequence", "sequences"),
        plural(cards as usize, "Scene Card", "Scene Cards")
    );
    let mut op = json!({"id": act.id});
    if a.flag("deleteContents") {
        op["mode"] = json!("deleteAll");
        d.row("Delete act", act.label.clone())
            .row("Also delete", contents);
    } else {
        op["mode"] = json!("moveContents");
        let dest = match rv::find_opt(
            ctx.conn,
            ctx.actor,
            &rv::ACT,
            a.s("moveContentsTo").as_deref(),
        )? {
            Some(to) if to.id != act.id => {
                d.base("story_act", &to.id);
                op["moveToActId"] = json!(to.id);
                format!("Act “{}”", to.label)
            }
            Some(_) => return Err(queries::ambiguous("Where should the act's contents go?")),
            None => "Unassigned".into(),
        };
        d.row("Delete act", act.label.clone())
            .row("Its contents", format!("{contents} move to {dest}"));
    }
    d.row("Recoverable", "Yes — from Recently Deleted");
    d.op(
        "story.delete_act",
        op,
        format!("Delete act “{}”", act.label),
    );
    d.done()
}

fn sequence(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let act = find(ctx, &a, "act", &rv::ACT)?;
    let title = a.req("title", "sequence title")?;
    let mut d = Draft::new(spec, args, "Proposed sequence");
    d.target("story_act", &act);
    d.row("Sequence", title.clone())
        .row("In act", act.label.clone())
        .row("Impact", "1 new object");
    d.op(
        "story.create_sequence",
        json!({"actId": act.id, "title": title}),
        format!("Create sequence “{title}”"),
    );
    d.done()
}

fn update_sequence(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let seq = find(ctx, &a, "sequence", &rv::SEQUENCE)?;
    let mut d = Draft::new(spec, args, "Proposed sequence change");
    d.target("story_sequence", &seq);
    let mut op = json!({"id": seq.id, "expectedRev": seq.rev});
    let mut changed = set_text(
        &mut d,
        &mut op,
        "title",
        "Title",
        a.s("title"),
        &seq.label,
        200,
    )?;
    let note = rv::column(ctx.conn, "story_sequence", "note", &seq.id)?;
    changed |= set_text(&mut d, &mut op, "note", "Note", a.raw("note"), &note, 4000)?;
    need_change(changed, &format!("sequence “{}”", seq.label))?;
    d.op(
        "story.update_sequence",
        op,
        format!("Edit sequence “{}”", seq.label),
    );
    d.done()
}

fn delete_sequence(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let seq = find(ctx, &a, "sequence", &rv::SEQUENCE)?;
    let cards = queries::count(
        ctx.conn,
        "SELECT count(*) FROM story_scene_card WHERE parent_type='sequence' AND parent_id=?1 AND deleted_at IS NULL",
        [&seq.id],
    )?;
    let mut d = Draft::new(spec, args, "Proposed sequence deletion");
    d.target("story_sequence", &seq);
    let what = plural(cards as usize, "Scene Card", "Scene Cards");
    let mode = if a.flag("deleteContents") {
        d.row("Delete sequence", seq.label.clone())
            .row("Also delete", what);
        "deleteAll"
    } else {
        d.row("Delete sequence", seq.label.clone()).row(
            "Its contents",
            format!("{what} stay in the act, in its place"),
        );
        "moveContents"
    };
    d.row("Recoverable", "Yes — from Recently Deleted");
    d.op(
        "story.delete_sequence",
        json!({"id": seq.id, "mode": mode}),
        format!("Delete sequence “{}”", seq.label),
    );
    d.done()
}

// ------------------------------------------------------------------ beats & cards

fn container(ctx: &PropCtx<'_>, a: &A<'_>, d: &mut Draft) -> AppResult<(Value, String)> {
    if let Some(seq) = rv::find_opt(
        ctx.conn,
        ctx.actor,
        &rv::SEQUENCE,
        a.s("sequence").as_deref(),
    )? {
        d.base("story_sequence", &seq.id);
        return Ok((
            json!({"parentType": "sequence", "parentId": seq.id}),
            format!("Sequence “{}”", seq.label),
        ));
    }
    if let Some(act) = rv::find_opt(ctx.conn, ctx.actor, &rv::ACT, a.s("act").as_deref())? {
        d.base("story_act", &act.id);
        return Ok((
            json!({"parentType": "act", "parentId": act.id}),
            format!("Act “{}”", act.label),
        ));
    }
    Ok((json!({"parentType": "parking"}), "Parking Lot".into()))
}

fn beat(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let text = a.req("text", "beat")?;
    let mut d = Draft::new(spec, args, "Proposed beat");
    let (parent, place) = container(ctx, &a, &mut d)?;
    let mut op = json!({"parent": parent, "text": text});
    if let Some(c) = a.s("color") {
        op["color"] = json!(c);
    }
    d.row("Beat", text.clone())
        .row("Place in", place)
        .row("Impact", "1 new object");
    d.op(
        "story.create_beat",
        op,
        format!("Create beat “{}”", queries::truncate_chars(&text, 60)),
    );
    d.done()
}

fn update_beat(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let b = find(ctx, &a, "beat", &rv::BEAT)?;
    let mut d = Draft::new(spec, args, "Proposed beat edit");
    d.target("story_beat", &b);
    let mut op = json!({"id": b.id, "expectedRev": b.rev});
    let mut changed = set_text(&mut d, &mut op, "text", "Beat", a.s("text"), &b.label, 2000)?;
    let note = rv::column(ctx.conn, "story_beat", "note", &b.id)?;
    changed |= set_text(&mut d, &mut op, "note", "Note", a.raw("note"), &note, 4000)?;
    let color = rv::column(ctx.conn, "story_beat", "color", &b.id)?;
    changed |= set_text(&mut d, &mut op, "color", "Colour", a.s("color"), &color, 20)?;
    need_change(changed, "the beat")?;
    d.op("story.update_beat", op, "Edit beat");
    d.done()
}

fn convert_beat(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let b = find(ctx, &a, "beat", &rv::BEAT)?;
    let mut d = Draft::new(spec, args, "Proposed conversion");
    d.target("story_beat", &b);
    d.row("Beat", b.label.clone())
        .row("Becomes", "A Scene Card in the same place");
    d.op(
        "story.convert_beat",
        json!({"id": b.id}),
        "Convert beat to Scene Card",
    );
    d.done()
}

fn update_card(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let card = find(ctx, &a, "card", &rv::CARD)?;
    let mut d = Draft::new(spec, args, "Proposed Scene Card edit");
    d.target("story_scene_card", &card);
    let mut op = json!({"id": card.id, "expectedRev": card.rev});
    let mut changed = false;
    for (key, col, op_key, label, max) in [
        (
            "description",
            "short_description",
            "shortDescription",
            "Description",
            600usize,
        ),
        (
            "heading",
            "scene_heading",
            "sceneHeading",
            "Scene heading",
            200,
        ),
        ("notes", "notes", "notes", "Notes", 8000),
        ("color", "color", "color", "Colour", 20),
    ] {
        let old = rv::column(ctx.conn, "story_scene_card", col, &card.id)?;
        let new = if key == "description" || key == "color" {
            a.s(key)
        } else {
            a.raw(key)
        };
        changed |= set_text(&mut d, &mut op, op_key, label, new, &old, max)?;
    }
    need_change(changed, &format!("Scene Card “{}”", card.label))?;
    d.op(
        "story.update_card",
        op,
        format!(
            "Edit Scene Card “{}”",
            queries::truncate_chars(&card.label, 60)
        ),
    );
    d.done()
}

fn card_to_beat(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let card = find(ctx, &a, "card", &rv::CARD)?;
    let linked: Option<String> = ctx.conn.query_row(
        "SELECT screenplay_scene_id FROM story_scene_card WHERE id=?1",
        [&card.id],
        |r| r.get(0),
    )?;
    let mut d = Draft::new(spec, args, "Proposed conversion");
    d.target("story_scene_card", &card);
    d.row("Scene Card", card.label.clone())
        .row("Becomes", "A beat in the same place");
    if linked.is_some() {
        d.row(
            "Note",
            "The screenplay scene built from this card is not changed",
        );
    }
    d.op(
        "story.card_to_beat",
        json!({"id": card.id}),
        "Convert Scene Card to beat",
    );
    d.done()
}

fn move_items(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed Story Board move");
    let (items, labels) = story_items(ctx, &a, &mut d, true)?;
    let (target, place) = match a.req("to", "destination")?.as_str() {
        "act" => {
            let act = find(ctx, &a, "act", &rv::ACT)?;
            d.base("story_act", &act.id);
            (
                json!({"parentType": "act", "parentId": act.id}),
                format!("Act “{}”", act.label),
            )
        }
        "sequence" => {
            let seq = find(ctx, &a, "sequence", &rv::SEQUENCE)?;
            d.base("story_sequence", &seq.id);
            (
                json!({"parentType": "sequence", "parentId": seq.id}),
                format!("Sequence “{}”", seq.label),
            )
        }
        "parking_lot" => (json!({"parentType": "parking"}), "Parking Lot".to_string()),
        _ => (
            json!({"parentType": "unassigned"}),
            "Unassigned".to_string(),
        ),
    };
    let mut op = json!({"items": items, "target": target});
    if let Some(before) =
        rv::find_opt(ctx.conn, ctx.actor, &rv::CARD, a.s("beforeCard").as_deref())?
    {
        d.base("story_scene_card", &before.id);
        op["before"] = json!({"kind": "card", "id": before.id});
    }
    d.row("Move", listing(&labels)).row("To", place.clone());
    d.op(
        "story.move_items",
        op,
        format!("Move {} to {place}", plural(labels.len(), "item", "items")),
    );
    d.done()
}

fn duplicate_items(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed duplicates");
    let (items, labels) = story_items(ctx, &a, &mut d, true)?;
    d.row("Duplicate", listing(&labels))
        .row("Impact", plural(labels.len(), "new object", "new objects"));
    d.op(
        "story.duplicate_items",
        json!({"items": items}),
        format!("Duplicate {}", plural(labels.len(), "item", "items")),
    );
    d.done()
}

fn park_items(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed parking");
    let (items, labels) = story_items(ctx, &a, &mut d, false)?;
    d.row("Park", listing(&labels))
        .row("To", "Parking Lot (they remember where they came from)");
    d.op(
        "story.park_items",
        json!({"items": items}),
        format!("Park {}", plural(labels.len(), "item", "items")),
    );
    d.done()
}

fn unpark_items(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed return from parking");
    let (items, labels) = story_items(ctx, &a, &mut d, false)?;
    d.row("Return", listing(&labels))
        .row("To", "Where each item was parked from (or Unassigned)");
    d.op(
        "story.restore_from_parking",
        json!({"items": items}),
        format!(
            "Return {} from the Parking Lot",
            plural(labels.len(), "item", "items")
        ),
    );
    d.done()
}

fn delete_items(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed deletion");
    let (items, labels) = story_items(ctx, &a, &mut d, true)?;
    d.row("Delete", listing(&labels))
        .row("Recoverable", "Yes — from Recently Deleted");
    d.op(
        "story.delete_items",
        json!({"items": items}),
        format!(
            "Delete {}",
            plural(labels.len(), "Story item", "Story items")
        ),
    );
    d.done()
}

fn remove_attachment(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let card = find(ctx, &a, "card", &rv::CARD)?;
    let mut stmt = ctx.conn.prepare(
        "SELECT sa.id, a.original_name, sa.rev FROM story_attachment sa JOIN asset a ON a.id = sa.asset_id
         WHERE sa.owner_type='scene_card' AND sa.owner_id=?1 ORDER BY sa.position",
    )?;
    let found: Vec<Found> = stmt
        .query_map([&card.id], |r| {
            Ok(Found {
                id: r.get(0)?,
                label: r.get(1)?,
                rev: r.get(2)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    let reference = a.req("attachment", "attachment")?;
    let att = match found.iter().find(|f| f.id == reference) {
        Some(f) => f.clone(),
        None => rv::pick("attachment", &reference, found)?,
    };
    let mut d = Draft::new(spec, args, "Proposed attachment removal");
    d.target("story_scene_card", &card)
        .target("story_attachment", &att);
    d.row("Remove attachment", att.label.clone())
        .row("From", card.label.clone());
    d.op(
        "story.remove_attachment",
        json!({"id": att.id}),
        format!("Remove attachment “{}”", att.label),
    );
    d.done()
}

fn build_screenplay(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let episode = rv::find_opt(ctx.conn, ctx.actor, &rv::EPISODE, a.s("episode").as_deref())?;
    let ids = crate::modules::story::build::active_cards_in_order(
        ctx.conn,
        episode.as_ref().map(|e| e.id.as_str()),
    )?;
    let mut include = Vec::new();
    let mut missing = 0usize;
    for id in &ids {
        let heading: Option<String> = ctx.conn.query_row(
            "SELECT scene_heading FROM story_scene_card WHERE id=?1",
            [id],
            |r| r.get(0),
        )?;
        match heading {
            Some(h) if crate::modules::story::build::heading_is_valid(&h) => {
                include.push(json!({"cardId": id}))
            }
            _ => missing += 1,
        }
    }
    if include.is_empty() {
        return Err(AppError::ai(
            "not_found",
            "No Scene Card on the Story Board has a valid scene heading yet (e.g. “INT. STATION — NIGHT”), so there is nothing to build.",
        ));
    }
    let mut d = Draft::new(spec, args, "Proposed Build Screenplay");
    d.module("Screenplay");
    let dest = a.req("destination", "destination")?;
    let mut op = json!({
        "include": include,
        "descriptionMode": if a.s("descriptionAs").as_deref() == Some("action_text") { "actionText" } else { "planningNote" },
    });
    if let Some(e) = &episode {
        d.base("episode", &e.id);
        op["episodeId"] = json!(e.id);
    }
    if dest == "new_draft" {
        let (sp_id, sp_title): (String, String) = ctx
            .conn
            .query_row(
                "SELECT id, title FROM screenplay WHERE deleted_at IS NULL AND episode_id IS ?1 ORDER BY updated_at DESC LIMIT 1",
                params![episode.as_ref().map(|e| e.id.clone())],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| AppError::ai("not_found", "There is no screenplay yet to add a draft to. Build a new screenplay instead."))?;
        d.base("screenplay", &sp_id);
        op["destination"] = json!("newDraft");
        op["screenplayId"] = json!(sp_id);
        d.row("Creates", format!("A new draft of “{sp_title}”"));
    } else {
        op["destination"] = json!("newScreenplay");
        if let Some(t) = a.s("title") {
            op["title"] = json!(t);
        }
        d.row("Creates", "A new screenplay");
    }
    if let Some(n) = a.s("draftName") {
        op["draftName"] = json!(n);
        d.row("Draft name", n);
    }
    d.row(
        "Scenes",
        plural(
            include.len(),
            "Scene Card becomes a scene",
            "Scene Cards become scenes",
        ),
    );
    if missing > 0 {
        d.exclude(
            "Scene Cards without a valid scene heading (not included)",
            missing.to_string(),
        );
    }
    d.row("Existing screenplays", "Never overwritten");
    d.op(
        "story.build_screenplay",
        op,
        "Build Screenplay from the Story Board",
    );
    d.done()
}

fn apply_order(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let scope = ctx.scope.cloned().unwrap_or_else(neutral_scope);
    let draft = rv::draft(ctx.conn, &scope, a.s("draft").as_deref())?;
    if draft.locked() {
        return Err(AppError::ai(
            "locked",
            format!(
                "{} is locked; its scene order can only change through a revision.",
                draft.label()
            ),
        ));
    }
    let episode = rv::find_opt(ctx.conn, ctx.actor, &rv::EPISODE, a.s("episode").as_deref())?;
    let mut d = Draft::new(spec, args, "Proposed scene order");
    d.pin("draft", json!(draft.name)).module("Screenplay");
    d.base("screenplay_draft", &draft.id);
    let mut op = json!({"draftId": draft.id, "confirmed": true});
    if let Some(e) = &episode {
        op["episodeId"] = json!(e.id);
    }
    d.row("Draft", draft.label())
        .row("Change", "Scenes built from Scene Cards are reordered to match the Story Board; scene numbers follow the new order")
        .row("Text", "Scene text is not changed");
    d.op(
        "story.apply_order",
        op,
        format!("Apply Story Board order to {}", draft.label()),
    );
    d.done()
}

// ------------------------------------------------------------------ characters

fn character(_ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let name = a.req("name", "character's name")?;
    let mut d = Draft::new(spec, args, "Proposed character");
    let mut op = json!({"name": name});
    d.row("Character", name.clone());
    for (key, op_key, label) in [
        ("role", "roleLabel", "Role"),
        ("description", "description", "Description"),
        ("notes", "notes", "Notes"),
    ] {
        if let Some(v) = a.s(key) {
            d.row(label, queries::truncate_chars(&v, 300));
            op[op_key] = json!(v);
        }
    }
    d.row("Impact", "1 new object");
    d.op(
        "story.create_character",
        op,
        format!("Create character “{name}”"),
    );
    d.done()
}

fn update_character(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let ch = find(ctx, &a, "character", &rv::CHARACTER)?;
    let mut d = Draft::new(spec, args, "Proposed character edit");
    d.target("story_character", &ch);
    let mut op = json!({"id": ch.id, "expectedRev": ch.rev});
    let mut changed = false;
    for (key, col, op_key, label, max) in [
        ("role", "role_label", "roleLabel", "Role", 120usize),
        (
            "description",
            "description",
            "description",
            "Description",
            4000,
        ),
        ("notes", "notes", "notes", "Notes", 8000),
    ] {
        let old = rv::column(ctx.conn, "story_character", col, &ch.id)?;
        changed |= set_text(&mut d, &mut op, op_key, label, a.raw(key), &old, max)?;
    }
    need_change(changed, &ch.label)?;
    d.op(
        "story.update_character",
        op,
        format!("Edit character “{}”", ch.label),
    );
    d.done()
}

fn archive_character(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let ch = find(ctx, &a, "character", &rv::CHARACTER)?;
    let archived = a.flag("archived");
    let mut d = Draft::new(
        spec,
        args,
        if archived {
            "Proposed archive"
        } else {
            "Proposed unarchive"
        },
    );
    d.target("story_character", &ch);
    d.row(
        if archived { "Archive" } else { "Unarchive" },
        ch.label.clone(),
    );
    d.op(
        "story.set_character_archived",
        json!({"id": ch.id, "archived": archived}),
        format!(
            "{} character “{}”",
            if archived { "Archive" } else { "Unarchive" },
            ch.label
        ),
    );
    d.done()
}

fn remove_character_image(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let ch = find(ctx, &a, "character", &rv::CHARACTER)?;
    let mut d = Draft::new(spec, args, "Proposed image removal");
    d.target("story_character", &ch);
    d.row("Remove image of", ch.label.clone());
    d.op(
        "story.clear_character_image",
        json!({"id": ch.id}),
        format!("Remove image of “{}”", ch.label),
    );
    d.done()
}

fn delete_character(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let ch = find(ctx, &a, "character", &rv::CHARACTER)?;
    let mut d = Draft::new(spec, args, "Proposed character deletion");
    d.target("story_character", &ch);
    d.row("Delete character", ch.label.clone())
        .row("Screenplay text", "Not changed")
        .row("Recoverable", "Yes — from Recently Deleted");
    d.op(
        "story.delete_character",
        json!({"id": ch.id}),
        format!("Delete character “{}”", ch.label),
    );
    d.done()
}

fn pair(ctx: &PropCtx<'_>, a: &A<'_>) -> AppResult<(Found, Found)> {
    let from = find(ctx, a, "from", &rv::CHARACTER)?;
    let to = find(ctx, a, "to", &rv::CHARACTER)?;
    if from.id == to.id {
        return Err(queries::ambiguous(
            "A relationship needs two different characters. Who is the other one?",
        ));
    }
    Ok((from, to))
}

fn relationship_row(ctx: &PropCtx<'_>, from: &Found, to: &Found) -> AppResult<Option<Found>> {
    Ok(ctx
        .conn
        .query_row(
            "SELECT id, relationship_type, rev FROM story_character_relationship
             WHERE (from_character_id=?1 AND to_character_id=?2) OR (from_character_id=?2 AND to_character_id=?1) LIMIT 1",
            params![from.id, to.id],
            |r| Ok(Found { id: r.get(0)?, label: r.get(1)?, rev: r.get(2)? }),
        )
        .optional()?)
}

fn relationship(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let (from, to) = pair(ctx, &a)?;
    if relationship_row(ctx, &from, &to)?.is_some() {
        return Err(queries::ambiguous(format!(
            "{} and {} already have a relationship. Should I change it instead?",
            from.label, to.label
        )));
    }
    let kind = a.req("type", "relationship")?;
    let mut d = Draft::new(spec, args, "Proposed relationship");
    d.target("story_character", &from)
        .target("story_character", &to);
    let mut op =
        json!({"fromCharacterId": from.id, "toCharacterId": to.id, "relationshipType": kind});
    if let Some(n) = a.s("note") {
        op["note"] = json!(n);
    }
    d.row(
        "Relationship",
        format!("{} — {kind} — {}", from.label, to.label),
    );
    d.op(
        "story.create_relationship",
        op,
        format!("Relate {} and {}", from.label, to.label),
    );
    d.done()
}

fn update_relationship(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let (from, to) = pair(ctx, &a)?;
    let rel = relationship_row(ctx, &from, &to)?.ok_or_else(|| {
        AppError::ai(
            "not_found",
            format!(
                "{} and {} don't have a relationship yet.",
                from.label, to.label
            ),
        )
    })?;
    let mut d = Draft::new(spec, args, "Proposed relationship change");
    d.target("story_character_relationship", &rel);
    let mut op = json!({"id": rel.id});
    let mut changed = set_text(
        &mut d,
        &mut op,
        "relationshipType",
        "Relationship",
        a.s("type"),
        &rel.label,
        80,
    )?;
    let note = rv::column(ctx.conn, "story_character_relationship", "note", &rel.id)?;
    changed |= set_text(&mut d, &mut op, "note", "Note", a.raw("note"), &note, 1000)?;
    need_change(changed, "the relationship")?;
    d.op(
        "story.update_relationship",
        op,
        format!("Change relationship of {} and {}", from.label, to.label),
    );
    d.done()
}

fn delete_relationship(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let (from, to) = pair(ctx, &a)?;
    let rel = relationship_row(ctx, &from, &to)?.ok_or_else(|| {
        AppError::ai(
            "not_found",
            format!("{} and {} don't have a relationship.", from.label, to.label),
        )
    })?;
    let mut d = Draft::new(spec, args, "Proposed relationship removal");
    d.target("story_character_relationship", &rel);
    d.row(
        "Remove relationship",
        format!("{} — {} — {}", from.label, rel.label, to.label),
    );
    d.op(
        "story.delete_relationship",
        json!({"id": rel.id}),
        format!("Remove relationship of {} and {}", from.label, to.label),
    );
    d.done()
}

fn link_card(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let ch = find(ctx, &a, "character", &rv::CHARACTER)?;
    let card = find(ctx, &a, "card", &rv::CARD)?;
    let unlink = a.flag("unlink");
    let mut d = Draft::new(
        spec,
        args,
        if unlink {
            "Proposed unlink"
        } else {
            "Proposed link"
        },
    );
    d.target("story_character", &ch)
        .target("story_scene_card", &card);
    d.row(
        if unlink { "Unlink" } else { "Link" },
        format!("{} ↔ Scene Card “{}”", ch.label, card.label),
    );
    let op = if unlink {
        "story.unlink_character_card"
    } else {
        "story.link_character_card"
    };
    d.op(
        op,
        json!({"characterId": ch.id, "cardId": card.id}),
        format!(
            "{} {} and a Scene Card",
            if unlink { "Unlink" } else { "Link" },
            ch.label
        ),
    );
    d.done()
}

// ------------------------------------------------------------------ timeline

fn story_day(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let scope = ctx.scope.cloned().unwrap_or_else(neutral_scope);
    let draft = rv::draft(ctx.conn, &scope, a.s("draft").as_deref())?;
    let mut d = Draft::new(spec, args, "Proposed Story Day");
    d.pin("draft", json!(draft.name)).module("Screenplay");
    let mut ids = Vec::new();
    let mut labels = Vec::new();
    for n in a.ints("sceneNumbers") {
        let s = rv::scene(ctx.conn, &draft, n)?;
        d.target(
            "screenplay_scene",
            &Found {
                id: s.id.clone(),
                label: s.label(),
                rev: s.rev,
            },
        );
        if !ids.contains(&s.id) {
            ids.push(s.id.clone());
            labels.push(s.number.to_string());
        }
    }
    let day = a.raw("storyDay").unwrap_or_default();
    d.row(
        "Scenes",
        format!("{} ({})", queries::join_and(&labels), draft.label()),
    )
    .row(
        "Story Day",
        if day.is_empty() {
            "Unassigned".to_string()
        } else {
            day.clone()
        },
    )
    .row("Scene order", "Not changed");
    d.op(
        "story.assign_story_day",
        json!({"sceneIds": ids, "storyDay": if day.is_empty() { Value::Null } else { json!(day) }}),
        "Assign Story Day",
    );
    d.done()
}

fn time_note(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let scope = ctx.scope.cloned().unwrap_or_else(neutral_scope);
    let draft = rv::draft(ctx.conn, &scope, a.s("draft").as_deref())?;
    let n = a
        .u("sceneNumber")
        .ok_or_else(|| queries::ambiguous("Which scene?"))?;
    let s = rv::scene(ctx.conn, &draft, n)?;
    let mut d = Draft::new(spec, args, "Proposed time note");
    d.pin("draft", json!(draft.name)).module("Screenplay");
    d.target(
        "screenplay_scene",
        &Found {
            id: s.id.clone(),
            label: s.label(),
            rev: s.rev,
        },
    );
    let old = rv::column(ctx.conn, "screenplay_scene", "time_note", &s.id)?;
    let new = a.raw("timeNote").unwrap_or_default();
    d.change(&s.label(), &old, &new);
    d.op(
        "story.set_time_note",
        json!({"sceneId": s.id, "timeNote": new}),
        format!("Set time note of {}", s.label()),
    );
    d.done()
}

// ------------------------------------------------------------------ seasons & episodes

fn season(_ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed season");
    let mut op = json!({});
    match a.s("title") {
        Some(t) => {
            op["title"] = json!(t);
            d.row("Season", t);
        }
        None => {
            d.row("Season", "Next season (numbered automatically)");
        }
    }
    d.row("Impact", "1 new object");
    d.op("story.create_season", op, "Create season");
    d.done()
}

fn update_season(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let s = find(ctx, &a, "season", &rv::SEASON)?;
    let mut d = Draft::new(spec, args, "Proposed season change");
    d.target("season", &s);
    let mut op = json!({"id": s.id});
    let mut changed = set_text(
        &mut d,
        &mut op,
        "title",
        "Title",
        a.s("title"),
        &s.label,
        200,
    )?;
    let note = rv::column(ctx.conn, "season", "note", &s.id)?;
    changed |= set_text(&mut d, &mut op, "note", "Note", a.raw("note"), &note, 4000)?;
    if changed {
        d.op(
            "story.update_season",
            op,
            format!("Edit season “{}”", s.label),
        );
    }
    if let Some(before) = rv::find_opt(
        ctx.conn,
        ctx.actor,
        &rv::SEASON,
        a.s("moveBefore").as_deref(),
    )? {
        d.base("season", &before.id)
            .row("Move", format!("Before “{}”", before.label));
        d.op(
            "story.move_season",
            json!({"id": s.id, "beforeId": before.id}),
            format!("Move season “{}”", s.label),
        );
        changed = true;
    }
    need_change(changed, &format!("season “{}”", s.label))?;
    d.done()
}

fn delete_season(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let s = find(ctx, &a, "season", &rv::SEASON)?;
    let episodes = queries::count(
        ctx.conn,
        "SELECT count(*) FROM episode WHERE season_id=?1 AND deleted_at IS NULL",
        [&s.id],
    )?;
    let mut d = Draft::new(spec, args, "Proposed season deletion");
    d.target("season", &s);
    d.row("Delete season", s.label.clone())
        .row("Episodes in it", episodes.to_string())
        .row("Recoverable", "Yes — from Recently Deleted");
    d.op(
        "story.delete_season",
        json!({"id": s.id}),
        format!("Delete season “{}”", s.label),
    );
    d.done()
}

fn episode(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let title = a.req("title", "episode title")?;
    let mut d = Draft::new(spec, args, "Proposed episode");
    let mut op = json!({"title": title});
    d.row("Episode", title.clone());
    if let Some(s) = rv::find_opt(ctx.conn, ctx.actor, &rv::SEASON, a.s("season").as_deref())? {
        d.base("season", &s.id).row("Season", s.label.clone());
        op["seasonId"] = json!(s.id);
    }
    for (key, label) in [("summary", "Summary"), ("status", "Status")] {
        if let Some(v) = a.s(key) {
            d.row(label, queries::truncate_chars(&v, 300));
            op[key] = json!(v);
        }
    }
    d.row("Impact", "1 new object");
    d.op(
        "story.create_episode",
        op,
        format!("Create episode “{title}”"),
    );
    d.done()
}

fn update_episode(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let e = find(ctx, &a, "episode", &rv::EPISODE)?;
    let mut d = Draft::new(spec, args, "Proposed episode change");
    d.target("episode", &e);
    let mut op = json!({"id": e.id, "expectedRev": e.rev});
    let mut changed = set_text(
        &mut d,
        &mut op,
        "title",
        "Title",
        a.s("title"),
        &e.label,
        200,
    )?;
    for (key, label, max) in [("summary", "Summary", 4000usize), ("status", "Status", 60)] {
        let old = rv::column(
            ctx.conn,
            "episode",
            if key == "summary" {
                "summary"
            } else {
                "status"
            },
            &e.id,
        )?;
        changed |= set_text(&mut d, &mut op, key, label, a.raw(key), &old, max)?;
    }
    if changed {
        d.op(
            "story.update_episode",
            op,
            format!("Edit episode “{}”", e.label),
        );
    }
    if let Some(s) = rv::find_opt(ctx.conn, ctx.actor, &rv::SEASON, a.s("season").as_deref())? {
        d.base("season", &s.id)
            .row("Move to season", s.label.clone());
        d.op(
            "story.move_episode",
            json!({"id": e.id, "seasonId": s.id}),
            format!("Move episode “{}”", e.label),
        );
        changed = true;
    }
    need_change(changed, &format!("episode “{}”", e.label))?;
    d.done()
}

fn delete_episode(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let e = find(ctx, &a, "episode", &rv::EPISODE)?;
    let mut d = Draft::new(spec, args, "Proposed episode deletion");
    d.target("episode", &e);
    d.row("Delete episode", e.label.clone())
        .row("Recoverable", "Yes — from Recently Deleted");
    d.op(
        "story.delete_episode",
        json!({"id": e.id}),
        format!("Delete episode “{}”", e.label),
    );
    d.done()
}

fn duplicate_episode(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let e = find(ctx, &a, "episode", &rv::EPISODE)?;
    let copy_story = a.flag("copyStory");
    let mut d = Draft::new(spec, args, "Proposed episode copy");
    d.target("episode", &e);
    d.row("Duplicate episode", e.label.clone()).row(
        "Story Board",
        if copy_story {
            "Copied (new identities, no screenplay links)"
        } else {
            "Not copied"
        },
    );
    d.op(
        "story.duplicate_episode",
        json!({"id": e.id, "copyStory": copy_story}),
        format!("Duplicate episode “{}”", e.label),
    );
    d.done()
}
