//! Proposal tools for Production: the Production Source, breakdown, the
//! Production Catalog, locations, cast and crew. Scene numbers refer to the
//! Production Source draft (the draft production follows). Production never
//! rewrites screenplay text.

use openframe_domain::enums::BreakdownCategory;
use openframe_domain::{AppError, AppResult, Capability};
use rusqlite::params;
use serde_json::{Value, json};

use super::super::queries::{self, DraftRef, SceneRow};
use super::super::toolbox::resolve::{self as rv, Found};
use super::super::toolbox::schema as sc;
use super::super::types::ChangeSetDraft;
use super::kit::{A, Draft, need_change, plural, set_text};
use super::workspace::{neutral_scope, spec};
use super::{PropCtx, ProposalSpec};

const CATALOG_STATUS: &[&str] = &[
    "Required",
    "Searching",
    "Shortlisted",
    "Confirmed",
    "Not Required",
];
const LOCATION_STATUS: &[&str] = &["Idea", "Shortlisted", "Confirmed", "Rejected"];

fn categories() -> Vec<&'static str> {
    BreakdownCategory::ALL.iter().map(|c| c.as_str()).collect()
}

pub const SPECS: &[ProposalSpec] = &[
    spec!(
        "propose_production_source",
        "Prepare choosing the Production Source draft (the draft breakdown, schedule and call sheets follow).",
        ["production.set_source"],
        [],
        Edit,
        "choosing the Production Source",
        "Production",
        s_source,
        source
    ),
    spec!(
        "propose_production_update",
        "Prepare updating Production to a newer draft (breakdown carried over by scene identity).",
        ["production.apply_update"],
        [],
        Edit,
        "updating Production",
        "Production",
        s_update,
        production_update
    ),
    spec!(
        "propose_breakdown_suggestions",
        "Prepare running breakdown suggestions for a scene (adds Suggested elements to review).",
        ["breakdown.suggest"],
        [],
        Edit,
        "changing the breakdown",
        "Breakdown",
        s_scene,
        suggest
    ),
    spec!(
        "propose_confirm_breakdown",
        "Prepare confirming suggested breakdown elements of a scene (all of them unless named).",
        ["breakdown.accept_many"],
        [],
        Edit,
        "changing the breakdown",
        "Breakdown",
        s_elements_opt,
        confirm
    ),
    spec!(
        "propose_reject_breakdown",
        "Prepare rejecting suggested breakdown elements of a scene.",
        ["breakdown.reject"],
        [],
        Edit,
        "changing the breakdown",
        "Breakdown",
        s_elements,
        reject
    ),
    spec!(
        "propose_breakdown_element",
        "Prepare adding a breakdown element (prop, wardrobe, vehicle, cast, …) to a scene.",
        ["breakdown.add_element"],
        [],
        Edit,
        "changing the breakdown",
        "Breakdown",
        s_add_element,
        add_element
    ),
    spec!(
        "propose_breakdown_note",
        "Prepare notes for a breakdown element.",
        ["breakdown.update_element"],
        [],
        Edit,
        "changing the breakdown",
        "Breakdown",
        s_element_note,
        element_note
    ),
    spec!(
        "propose_remove_breakdown_element",
        "Prepare removing a breakdown element from a scene (recoverable).",
        ["breakdown.remove_element"],
        [],
        SoftDelete,
        "changing the breakdown",
        "Breakdown",
        s_element,
        remove_element
    ),
    spec!(
        "propose_archive_breakdown_element",
        "Prepare archiving (or unarchiving) a breakdown element.",
        ["breakdown.set_archived"],
        [],
        Edit,
        "changing the breakdown",
        "Breakdown",
        s_element_archive,
        archive_element
    ),
    spec!(
        "propose_breakdown_complete",
        "Prepare marking a scene's breakdown complete (or incomplete).",
        ["breakdown.set_complete"],
        [],
        Edit,
        "changing the breakdown",
        "Breakdown",
        s_complete,
        complete
    ),
    spec!(
        "propose_breakdown_reviewed",
        "Prepare marking a changed scene's breakdown as reviewed.",
        ["breakdown.mark_reviewed"],
        [],
        Edit,
        "changing the breakdown",
        "Breakdown",
        s_scene,
        reviewed
    ),
    spec!(
        "propose_refresh_breakdown_suggestions",
        "Prepare re-running suggestions for a changed scene (confirmed elements are kept).",
        ["breakdown.apply_suggested_update"],
        [],
        Edit,
        "changing the breakdown",
        "Breakdown",
        s_scene,
        refresh_suggestions
    ),
    spec!(
        "propose_catalog_item",
        "Prepare a new Production Catalog item.",
        ["catalog.create"],
        [],
        Edit,
        "changing the Production Catalog",
        "Production",
        s_catalog_new,
        catalog_new
    ),
    spec!(
        "propose_update_catalog_item",
        "Prepare edits to a catalog item (name, status, description, notes, contact).",
        ["catalog.update"],
        [],
        Edit,
        "changing the Production Catalog",
        "Production",
        s_catalog_update,
        catalog_update
    ),
    spec!(
        "propose_catalog_alias",
        "Prepare adding (or removing) another script name for a catalog item.",
        ["catalog.add_alias", "catalog.remove_alias"],
        [],
        Edit,
        "changing the Production Catalog",
        "Production",
        s_alias,
        catalog_alias
    ),
    spec!(
        "propose_archive_catalog_item",
        "Prepare archiving (or unarchiving) a catalog item.",
        ["catalog.set_archived"],
        [],
        Edit,
        "changing the Production Catalog",
        "Production",
        s_item_archive,
        catalog_archive
    ),
    spec!(
        "propose_delete_catalog_item",
        "Prepare moving a catalog item to Recently Deleted.",
        ["catalog.delete"],
        [],
        SoftDelete,
        "deleting catalog items",
        "Production",
        s_item,
        catalog_delete
    ),
    spec!(
        "propose_replace_catalog_item",
        "Prepare replacing one catalog item with another in scenes (the old item is kept).",
        ["catalog.replace_in_scenes"],
        [],
        Edit,
        "changing the Production Catalog",
        "Production",
        s_replace_item,
        catalog_replace
    ),
    spec!(
        "propose_location",
        "Prepare a new production location.",
        ["locations.create"],
        [],
        Edit,
        "changing locations",
        "Production",
        s_location_new,
        location_new
    ),
    spec!(
        "propose_update_location",
        "Prepare edits to a location (name, address, contact, practical notes).",
        ["locations.update"],
        [],
        Edit,
        "changing locations",
        "Production",
        s_location_update,
        location_update
    ),
    spec!(
        "propose_location_status",
        "Prepare changing a location's status (Idea, Shortlisted, Confirmed, Rejected).",
        ["locations.set_status"],
        [],
        Edit,
        "changing locations",
        "Production",
        s_location_status,
        location_status
    ),
    spec!(
        "propose_remove_location_photo",
        "Prepare removing a location photo (recoverable).",
        ["locations.remove_photo"],
        [],
        SoftDelete,
        "changing locations",
        "Production",
        s_location_photo,
        location_photo
    ),
    spec!(
        "propose_archive_location",
        "Prepare archiving (or unarchiving) a location.",
        ["locations.set_archived"],
        [],
        Edit,
        "changing locations",
        "Production",
        s_location_archive,
        location_archive
    ),
    spec!(
        "propose_delete_location",
        "Prepare moving a location to Recently Deleted.",
        ["locations.delete"],
        [],
        SoftDelete,
        "deleting locations",
        "Production",
        s_location,
        location_delete
    ),
    spec!(
        "propose_replace_location",
        "Prepare replacing a location with another in its scenes (e.g. a rejected location).",
        ["locations.replace"],
        [],
        Edit,
        "changing locations",
        "Production",
        s_location_replace,
        location_replace
    ),
    spec!(
        "propose_cast_member",
        "Prepare a new cast member (performer) for a character.",
        ["cast.create"],
        [],
        Edit,
        "changing cast and crew",
        "Cast & Crew",
        s_cast_new,
        cast_new
    ),
    spec!(
        "propose_update_cast_member",
        "Prepare edits to a cast member (name, contact, availability, notes).",
        ["cast.update"],
        [],
        Edit,
        "changing cast and crew",
        "Cast & Crew",
        s_cast_update,
        cast_update
    ),
    spec!(
        "propose_assign_cast",
        "Prepare assigning a cast member to a character (primary or alternate).",
        ["cast.assign"],
        [],
        Edit,
        "changing cast and crew",
        "Cast & Crew",
        s_cast_assign,
        cast_assign
    ),
    spec!(
        "propose_crew_member",
        "Prepare a new crew member.",
        ["crew.create"],
        [],
        Edit,
        "changing cast and crew",
        "Cast & Crew",
        s_crew_new,
        crew_new
    ),
    spec!(
        "propose_update_crew_member",
        "Prepare edits to a crew member (name, role, department, contact, notes).",
        ["crew.update"],
        [],
        Edit,
        "changing cast and crew",
        "Cast & Crew",
        s_crew_update,
        crew_update
    ),
    spec!(
        "propose_archive_person",
        "Prepare archiving (or unarchiving) a cast or crew member.",
        ["cast.set_archived", "crew.set_archived"],
        [],
        Edit,
        "changing cast and crew",
        "Cast & Crew",
        s_person_archive,
        person_archive
    ),
    spec!(
        "propose_remove_person",
        "Prepare removing a cast or crew member (recoverable).",
        ["cast.delete", "crew.delete"],
        [],
        SoftDelete,
        "changing cast and crew",
        "Cast & Crew",
        s_person,
        person_remove
    ),
];

// ------------------------------------------------------------------ schemas

fn s_source() -> Value {
    sc::obj(&[("draft", sc::s(120)), ("reason", sc::s(500))], &["draft"])
}
fn s_update() -> Value {
    sc::obj(&[("draft", sc::s(120))], &[])
}
fn scene_props() -> Vec<(&'static str, Value)> {
    vec![("sceneNumber", sc::scene_number()), ("draft", sc::s(120))]
}
fn with(mut p: Vec<(&'static str, Value)>, more: &[(&'static str, Value)], req: &[&str]) -> Value {
    p.extend(more.iter().cloned());
    sc::obj(&p, req)
}
fn s_scene() -> Value {
    with(scene_props(), &[], &["sceneNumber"])
}
fn s_elements_opt() -> Value {
    with(
        scene_props(),
        &[("elements", sc::arr(sc::s(200), 50))],
        &["sceneNumber"],
    )
}
fn s_elements() -> Value {
    with(
        scene_props(),
        &[("elements", sc::arr(sc::s(200), 50))],
        &["sceneNumber", "elements"],
    )
}
fn s_add_element() -> Value {
    with(
        scene_props(),
        &[
            ("category", sc::en(&categories())),
            ("name", sc::s(200)),
            ("notes", sc::s(2000)),
        ],
        &["sceneNumber", "category", "name"],
    )
}
fn s_element_note() -> Value {
    with(
        scene_props(),
        &[("element", sc::s(200)), ("notes", sc::s(2000))],
        &["sceneNumber", "element", "notes"],
    )
}
fn s_element() -> Value {
    with(
        scene_props(),
        &[("element", sc::s(200))],
        &["sceneNumber", "element"],
    )
}
fn s_element_archive() -> Value {
    with(
        scene_props(),
        &[("element", sc::s(200)), ("archived", sc::boolean())],
        &["sceneNumber", "element", "archived"],
    )
}
fn s_complete() -> Value {
    with(
        scene_props(),
        &[("complete", sc::boolean())],
        &["sceneNumber", "complete"],
    )
}
fn s_catalog_new() -> Value {
    sc::obj(
        &[
            ("category", sc::en(&categories())),
            ("name", sc::s(200)),
            ("status", sc::en(CATALOG_STATUS)),
            ("description", sc::s(4000)),
            ("notes", sc::s(4000)),
            ("contact", sc::s(500)),
        ],
        &["category", "name"],
    )
}
fn s_catalog_update() -> Value {
    sc::obj(
        &[
            ("item", sc::reference()),
            ("name", sc::s(200)),
            ("status", sc::en(CATALOG_STATUS)),
            ("description", sc::s(4000)),
            ("notes", sc::s(4000)),
            ("contact", sc::s(500)),
        ],
        &["item"],
    )
}
fn s_alias() -> Value {
    sc::obj(
        &[
            ("item", sc::reference()),
            ("alias", sc::s(200)),
            ("remove", sc::boolean()),
        ],
        &["item", "alias"],
    )
}
fn s_item_archive() -> Value {
    sc::obj(
        &[("item", sc::reference()), ("archived", sc::boolean())],
        &["item", "archived"],
    )
}
fn s_item() -> Value {
    sc::obj(&[("item", sc::reference())], &["item"])
}
fn s_replace_item() -> Value {
    sc::obj(
        &[
            ("from", sc::reference()),
            ("to", sc::reference()),
            ("sceneNumbers", sc::arr(sc::scene_number(), 200)),
            ("draft", sc::s(120)),
        ],
        &["from", "to"],
    )
}
fn s_location_new() -> Value {
    sc::obj(
        &[
            ("name", sc::s(200)),
            ("address", sc::s(500)),
            ("contact", sc::s(500)),
            ("status", sc::en(LOCATION_STATUS)),
        ],
        &["name"],
    )
}
const NOTE_KEYS: &[&str] = &[
    "access",
    "parking",
    "noise",
    "power",
    "permission",
    "toilets",
    "facilities",
    "travel",
    "general",
];
fn s_location_update() -> Value {
    let mut p = vec![
        ("location", sc::reference()),
        ("name", sc::s(200)),
        ("address", sc::s(500)),
        ("contact", sc::s(500)),
    ];
    for k in NOTE_KEYS {
        p.push((k, sc::s(4000)));
    }
    sc::obj(&p, &["location"])
}
fn s_location_status() -> Value {
    sc::obj(
        &[
            ("location", sc::reference()),
            ("status", sc::en(LOCATION_STATUS)),
        ],
        &["location", "status"],
    )
}
fn s_location_photo() -> Value {
    sc::obj(
        &[("location", sc::reference()), ("photo", sc::int(1, 1000))],
        &["location", "photo"],
    )
}
fn s_location_archive() -> Value {
    sc::obj(
        &[("location", sc::reference()), ("archived", sc::boolean())],
        &["location", "archived"],
    )
}
fn s_location() -> Value {
    sc::obj(&[("location", sc::reference())], &["location"])
}
fn s_location_replace() -> Value {
    sc::obj(
        &[
            ("location", sc::reference()),
            ("replacement", sc::reference()),
            ("sceneNumbers", sc::arr(sc::scene_number(), 200)),
            ("draft", sc::s(120)),
        ],
        &["location", "replacement"],
    )
}
fn s_cast_new() -> Value {
    sc::obj(
        &[
            ("person", sc::s(200)),
            ("character", sc::reference()),
            ("characterName", sc::s(120)),
            ("primary", sc::boolean()),
            ("contact", sc::s(500)),
            ("availability", sc::s(2000)),
            ("notes", sc::s(4000)),
        ],
        &["person"],
    )
}
fn s_cast_update() -> Value {
    sc::obj(
        &[
            ("castMember", sc::reference()),
            ("person", sc::s(200)),
            ("contact", sc::s(500)),
            ("availability", sc::s(2000)),
            ("notes", sc::s(4000)),
        ],
        &["castMember"],
    )
}
fn s_cast_assign() -> Value {
    sc::obj(
        &[
            ("castMember", sc::reference()),
            ("character", sc::reference()),
            ("characterName", sc::s(120)),
            ("primary", sc::boolean()),
        ],
        &["castMember"],
    )
}
fn s_crew_new() -> Value {
    sc::obj(
        &[
            ("person", sc::s(200)),
            ("role", sc::s(120)),
            ("department", sc::s(120)),
            ("contact", sc::s(500)),
            ("notes", sc::s(4000)),
        ],
        &["person", "role"],
    )
}
fn s_crew_update() -> Value {
    sc::obj(
        &[
            ("crewMember", sc::reference()),
            ("person", sc::s(200)),
            ("role", sc::s(120)),
            ("department", sc::s(120)),
            ("contact", sc::s(500)),
            ("notes", sc::s(4000)),
        ],
        &["crewMember"],
    )
}
fn s_person_archive() -> Value {
    sc::obj(
        &[
            ("kind", sc::en(&["cast", "crew"])),
            ("person", sc::reference()),
            ("archived", sc::boolean()),
        ],
        &["kind", "person", "archived"],
    )
}
fn s_person() -> Value {
    sc::obj(
        &[
            ("kind", sc::en(&["cast", "crew"])),
            ("person", sc::reference()),
        ],
        &["kind", "person"],
    )
}

// ------------------------------------------------------------------ helpers

fn active_source(ctx: &PropCtx<'_>) -> AppResult<(String, DraftRef)> {
    let (source, draft_id) = queries::production_source_draft(ctx.conn)?.ok_or_else(|| {
        AppError::ai(
            "not_found",
            "No Production Source is selected yet. Choose a Production Source draft in Production first.",
        )
    })?;
    let draft =
        queries::draft_by_id(ctx.conn, &draft_id)?.ok_or_else(|| AppError::not_found("draft"))?;
    Ok((source, draft))
}

/// Scene of the Production Source (or the named draft) by number.
fn prod_scene(ctx: &PropCtx<'_>, a: &A<'_>, d: &mut Draft) -> AppResult<(DraftRef, SceneRow)> {
    let scope = ctx.scope.cloned().unwrap_or_else(neutral_scope);
    let draft = rv::production_draft(ctx.conn, &scope, a.s("draft").as_deref())?;
    let n = a
        .u("sceneNumber")
        .ok_or_else(|| queries::ambiguous("Which scene? Tell me its number."))?;
    let s = rv::scene(ctx.conn, &draft, n)?;
    d.pin("draft", json!(draft.name));
    d.target(
        "screenplay_scene",
        &Found {
            id: s.id.clone(),
            label: s.label(),
            rev: s.rev,
        },
    );
    Ok((draft, s))
}

/// Breakdown elements of a scene: (id, name, category, state, rev).
fn elements(ctx: &PropCtx<'_>, scene: &SceneRow) -> AppResult<Vec<(Found, String, String)>> {
    let mut stmt = ctx.conn.prepare(
        "SELECT id, display_name, rev, category, confirmation_state FROM breakdown_element
         WHERE scene_id=?1 AND deleted_at IS NULL ORDER BY category, display_name",
    )?;
    let rows = stmt
        .query_map([&scene.id], |r| {
            Ok((
                Found {
                    id: r.get(0)?,
                    label: r.get(1)?,
                    rev: r.get(2)?,
                },
                r.get(3)?,
                r.get(4)?,
            ))
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

fn element_named(
    ctx: &PropCtx<'_>,
    scene: &SceneRow,
    name: &str,
    states: &[&str],
) -> AppResult<(Found, String, String)> {
    let all: Vec<(Found, String, String)> = elements(ctx, scene)?
        .into_iter()
        .filter(|(_, _, st)| states.is_empty() || states.contains(&st.as_str()))
        .collect();
    let f = match all.iter().find(|(f, _, _)| f.id == name) {
        Some((f, _, _)) => f.clone(),
        None => rv::pick(
            "breakdown element",
            name,
            all.iter().map(|(f, _, _)| f.clone()).collect(),
        )?,
    };
    Ok(all
        .into_iter()
        .find(|(x, _, _)| x.id == f.id)
        .expect("picked from the list"))
}

fn scene_simple(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
    title: &str,
    op: &str,
    row: &str,
    extra: Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, title);
    let (draft, s) = prod_scene(ctx, &a, &mut d)?;
    d.row(row, format!("{} ({})", s.label(), draft.label()));
    let mut o = json!({"sceneId": s.id});
    if let (Some(dst), Some(src)) = (o.as_object_mut(), extra.as_object()) {
        for (k, v) in src {
            dst.insert(k.clone(), v.clone());
        }
    }
    d.op(op, o, format!("{row} {}", s.label()));
    d.done()
}

// ------------------------------------------------------------------ source

fn source(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let scope = ctx.scope.cloned().unwrap_or_else(neutral_scope);
    let draft = rv::draft(ctx.conn, &scope, a.s("draft").as_deref())?;
    let current = queries::production_source_draft(ctx.conn)?;
    if current.as_ref().is_some_and(|(_, d)| *d == draft.id) {
        return Err(queries::ambiguous(format!(
            "{} is already the Production Source. Which draft should production follow?",
            draft.label()
        )));
    }
    if current.is_some() {
        // Product rule: once production has a source, moving to another draft goes
        // through the reviewed Production update (scene-identity carry-over).
        return Err(queries::ambiguous(format!(
            "Production already has a Production Source. Should I prepare a Production update to {} instead?",
            draft.label()
        )));
    }
    let mut d = Draft::new(spec, args, "Proposed Production Source");
    d.pin("draft", json!(draft.name))
        .base("screenplay_draft", &draft.id);
    let mut op = json!({"draftId": draft.id});
    if let Some(r) = a.s("reason") {
        op["reason"] = json!(r);
    }
    let from = match &current {
        Some((_, id)) => queries::draft_by_id(ctx.conn, id)?
            .map(|x| x.label())
            .unwrap_or_default(),
        None => "(none)".into(),
    };
    d.change("Production Source", &from, &draft.label());
    d.row(
        "Effect",
        "Breakdown, schedule and call sheets follow this draft",
    );
    d.op(
        "production.set_source",
        op,
        format!("Make {} the Production Source", draft.label()),
    );
    d.done()
}

fn production_update(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let (_, src) = active_source(ctx)?;
    let scope = ctx.scope.cloned().unwrap_or_else(neutral_scope);
    let target = match a.s("draft") {
        Some(r) => rv::draft(ctx.conn, &scope, Some(&r))?,
        None => queries::current_draft(ctx.conn)?.ok_or_else(|| AppError::not_found("draft"))?,
    };
    if target.id == src.id {
        return Err(queries::ambiguous(format!(
            "Production already follows {}. Which newer draft should it move to?",
            src.label()
        )));
    }
    let mut d = Draft::new(spec, args, "Proposed Production update");
    d.pin("draft", json!(target.name))
        .base("screenplay_draft", &target.id);
    d.change("Production follows", &src.label(), &target.label());
    d.row(
        "Breakdown",
        "Carried over by scene identity; changed scenes are flagged for review",
    );
    d.op(
        "production.apply_update",
        json!({"draftId": target.id}),
        format!("Update Production to {}", target.label()),
    );
    d.done()
}

// ------------------------------------------------------------------ breakdown

fn suggest(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    active_source(ctx)?;
    scene_simple(
        ctx,
        spec,
        args,
        "Proposed breakdown suggestions",
        "breakdown.suggest",
        "Suggest elements for",
        json!({}),
    )
}

fn confirm(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed breakdown confirmation");
    let (_, s) = prod_scene(ctx, &a, &mut d)?;
    let names = a.list("elements");
    let picked: Vec<(Found, String, String)> = if names.is_empty() {
        elements(ctx, &s)?
            .into_iter()
            .filter(|(_, _, st)| st == "Suggested")
            .collect()
    } else {
        names
            .iter()
            .map(|n| element_named(ctx, &s, n, &["Suggested"]))
            .collect::<AppResult<_>>()?
    };
    if picked.is_empty() {
        return Err(AppError::ai(
            "not_found",
            format!(
                "{} has no suggested breakdown elements to confirm.",
                s.label()
            ),
        ));
    }
    for (f, cat, _) in &picked {
        d.target("breakdown_element", f)
            .row(format!("Confirm ({cat})"), f.label.clone());
    }
    let ids: Vec<String> = picked.iter().map(|(f, _, _)| f.id.clone()).collect();
    d.op(
        "breakdown.accept_many",
        json!({"ids": ids}),
        format!(
            "Confirm {} in {}",
            plural(ids.len(), "element", "elements"),
            s.label()
        ),
    );
    d.done()
}

fn reject(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed breakdown rejection");
    let (_, s) = prod_scene(ctx, &a, &mut d)?;
    let mut ids = Vec::new();
    for n in a.list("elements") {
        let (f, cat, _) = element_named(ctx, &s, &n, &["Suggested"])?;
        d.target("breakdown_element", &f)
            .row(format!("Reject ({cat})"), f.label.clone());
        ids.push(f.id);
    }
    d.op(
        "breakdown.reject",
        json!({"ids": ids}),
        format!(
            "Reject {} in {}",
            plural(ids.len(), "suggestion", "suggestions"),
            s.label()
        ),
    );
    d.done()
}

fn add_element(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    active_source(ctx)?;
    let mut d = Draft::new(spec, args, "Proposed breakdown element");
    let (_, s) = prod_scene(ctx, &a, &mut d)?;
    let category = a.req("category", "breakdown category")?;
    let name = a.req("name", "element name")?;
    if elements(ctx, &s)?
        .iter()
        .any(|(f, c, st)| c == &category && f.label.eq_ignore_ascii_case(&name) && st != "Rejected")
    {
        return Err(queries::ambiguous(format!(
            "{} already has “{name}” under {category}. Should I change it instead?",
            s.label()
        )));
    }
    // Reuse the catalog item with this exact name (or alias) when there is exactly one.
    let mut stmt = ctx.conn.prepare(
        "SELECT DISTINCT i.id, i.name, i.rev FROM catalog_item i LEFT JOIN catalog_alias al ON al.catalog_item_id = i.id
         WHERE i.deleted_at IS NULL AND i.category=?1 AND (lower(i.name)=lower(?2) OR lower(al.alias)=lower(?2))",
    )?;
    let exact: Vec<Found> = stmt
        .query_map(params![category, name], |r| {
            Ok(Found {
                id: r.get(0)?,
                label: r.get(1)?,
                rev: r.get(2)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    let catalog = match exact.as_slice() {
        [one] => {
            d.base("catalog_item", &one.id)
                .row("Catalog item", format!("Existing: {}", one.label));
            json!({"mode": "existing", "catalogItemId": one.id})
        }
        [] => {
            d.row("Catalog item", format!("New: {name} ({category})"));
            json!({"mode": "new"})
        }
        _ => {
            return Err(queries::ambiguous(format!(
                "There are several {category} catalog items called “{name}”. Open the breakdown to choose one."
            )));
        }
    };
    let mut op = json!({"sceneId": s.id, "category": category, "name": name, "catalog": catalog});
    if let Some(n) = a.s("notes") {
        op["notes"] = json!(n);
    }
    d.row("Scene", s.label())
        .row(category.clone(), name.clone());
    d.op(
        "breakdown.add_element",
        op,
        format!("Add {category} “{name}” to {}", s.label()),
    );
    d.done()
}

fn element_note(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed breakdown note");
    let (_, s) = prod_scene(ctx, &a, &mut d)?;
    let (f, _, _) = element_named(ctx, &s, &a.req("element", "breakdown element")?, &[])?;
    d.target("breakdown_element", &f);
    let old = rv::column(ctx.conn, "breakdown_element", "notes", &f.id)?;
    let mut op = json!({"id": f.id});
    let changed = set_text(
        &mut d,
        &mut op,
        "notes",
        &format!("Notes on {}", f.label),
        a.raw("notes"),
        &old,
        2000,
    )?;
    need_change(changed, &f.label)?;
    d.op(
        "breakdown.update_element",
        op,
        format!("Edit notes of {}", f.label),
    );
    d.done()
}

fn remove_element(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed breakdown removal");
    let (_, s) = prod_scene(ctx, &a, &mut d)?;
    let (f, cat, _) = element_named(ctx, &s, &a.req("element", "breakdown element")?, &[])?;
    d.target("breakdown_element", &f)
        .row(format!("Remove ({cat})"), f.label.clone())
        .row("From", s.label())
        .row("Catalog item", "Kept");
    d.op(
        "breakdown.remove_element",
        json!({"id": f.id}),
        format!("Remove {} from {}", f.label, s.label()),
    );
    d.done()
}

fn archive_element(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed breakdown archive");
    let (_, s) = prod_scene(ctx, &a, &mut d)?;
    let (f, _, _) = element_named(ctx, &s, &a.req("element", "breakdown element")?, &[])?;
    let archived = a.flag("archived");
    d.target("breakdown_element", &f).row(
        if archived { "Archive" } else { "Unarchive" },
        f.label.clone(),
    );
    d.op(
        "breakdown.set_archived",
        json!({"id": f.id, "archived": archived}),
        format!(
            "{} {}",
            if archived { "Archive" } else { "Unarchive" },
            f.label
        ),
    );
    d.done()
}

fn complete(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let done = A(args).flag("complete");
    scene_simple(
        ctx,
        spec,
        args,
        "Proposed breakdown status",
        "breakdown.set_complete",
        if done {
            "Mark breakdown complete for"
        } else {
            "Mark breakdown incomplete for"
        },
        json!({"complete": done}),
    )
}

fn reviewed(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    scene_simple(
        ctx,
        spec,
        args,
        "Proposed breakdown review",
        "breakdown.mark_reviewed",
        "Mark breakdown reviewed for",
        json!({}),
    )
}

fn refresh_suggestions(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    scene_simple(
        ctx,
        spec,
        args,
        "Proposed suggestion refresh",
        "breakdown.apply_suggested_update",
        "Refresh suggestions for",
        json!({}),
    )
}

// ------------------------------------------------------------------ catalog

fn item(ctx: &PropCtx<'_>, a: &A<'_>, key: &str, d: &mut Draft) -> AppResult<Found> {
    let f = rv::find(
        ctx.conn,
        ctx.actor,
        &rv::CATALOG_ITEM,
        &a.req(key, "catalog item")?,
    )?;
    d.target("catalog_item", &f);
    Ok(f)
}

fn catalog_new(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let category = a.req("category", "category")?;
    let name = a.req("name", "item name")?;
    let dup = queries::count(
        ctx.conn,
        "SELECT count(*) FROM catalog_item WHERE deleted_at IS NULL AND category=?1 AND lower(name)=lower(?2)",
        params![category, name],
    )?;
    if dup > 0 {
        return Err(queries::ambiguous(format!(
            "The catalog already has a {category} item called “{name}”. Should I change that one instead?"
        )));
    }
    let mut d = Draft::new(spec, args, "Proposed catalog item");
    let mut op = json!({"category": category, "name": name});
    d.row("Catalog item", format!("{name} ({category})"));
    for (key, label) in [
        ("status", "Status"),
        ("description", "Description"),
        ("notes", "Notes"),
        ("contact", "Contact"),
    ] {
        if let Some(v) = a.s(key) {
            d.row(label, queries::truncate_chars(&v, 300));
            op[key] = json!(v);
        }
    }
    d.row("Impact", "1 new object");
    d.op(
        "catalog.create",
        op,
        format!("Create catalog item “{name}”"),
    );
    d.done()
}

fn catalog_update(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed catalog edit");
    let it = item(ctx, &a, "item", &mut d)?;
    let mut op = json!({"id": it.id, "expectedRev": it.rev});
    let mut changed = set_text(&mut d, &mut op, "name", "Name", a.s("name"), &it.label, 200)?;
    for (key, label, max) in [
        ("status", "Status", 40usize),
        ("description", "Description", 4000),
        ("notes", "Notes", 4000),
        ("contact", "Contact", 500),
    ] {
        let old = rv::column(ctx.conn, "catalog_item", key, &it.id)?;
        let new = if key == "status" {
            a.s(key)
        } else {
            a.raw(key)
        };
        changed |= set_text(&mut d, &mut op, key, label, new, &old, max)?;
    }
    need_change(changed, &it.label)?;
    if a.s("name").is_some() {
        d.row(
            "Breakdown references",
            "Follow automatically (same catalog item)",
        );
    }
    d.op(
        "catalog.update",
        op,
        format!("Edit catalog item “{}”", it.label),
    );
    d.done()
}

fn catalog_alias(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed catalog alias");
    let it = item(ctx, &a, "item", &mut d)?;
    let alias = a.req("alias", "alias")?;
    if a.flag("remove") {
        let al: Found = ctx
            .conn
            .query_row(
                "SELECT id, alias, rev FROM catalog_alias WHERE catalog_item_id=?1 AND lower(alias)=lower(?2)",
                params![it.id, alias],
                |r| Ok(Found { id: r.get(0)?, label: r.get(1)?, rev: r.get(2)? }),
            )
            .map_err(|_| AppError::ai("not_found", format!("“{}” has no alias “{alias}”.", it.label)))?;
        d.base("catalog_alias", &al.id)
            .row("Remove alias", format!("{} (from {})", al.label, it.label));
        d.op(
            "catalog.remove_alias",
            json!({"id": al.id}),
            format!("Remove alias “{}”", al.label),
        );
    } else {
        d.row("Add alias", format!("{alias} (for {})", it.label));
        d.op(
            "catalog.add_alias",
            json!({"id": it.id, "alias": alias}),
            format!("Add alias “{alias}”"),
        );
    }
    d.done()
}

fn catalog_archive(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
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
    let it = item(ctx, &a, "item", &mut d)?;
    d.row(
        if archived { "Archive" } else { "Unarchive" },
        it.label.clone(),
    );
    d.op(
        "catalog.set_archived",
        json!({"id": it.id, "archived": archived}),
        format!(
            "{} “{}”",
            if archived { "Archive" } else { "Unarchive" },
            it.label
        ),
    );
    d.done()
}

fn catalog_delete(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed catalog deletion");
    let it = item(ctx, &a, "item", &mut d)?;
    let used = queries::count(
        ctx.conn,
        "SELECT count(DISTINCT scene_id) FROM breakdown_element WHERE catalog_item_id=?1 AND deleted_at IS NULL",
        [&it.id],
    )?;
    d.row("Delete", it.label.clone())
        .row("Used in", plural(used as usize, "scene", "scenes"))
        .row("Recoverable", "Yes — from Recently Deleted");
    d.op(
        "catalog.delete",
        json!({"id": it.id}),
        format!("Delete catalog item “{}”", it.label),
    );
    d.done()
}

/// Scenes (ids, numbers) of the active source whose breakdown uses any of `items`,
/// optionally narrowed to `numbers`.
fn scenes_using(
    ctx: &PropCtx<'_>,
    a: &A<'_>,
    items: &[String],
    d: &mut Draft,
) -> AppResult<(Vec<String>, Vec<String>)> {
    let (source, draft) = active_source(ctx)?;
    let scenes = queries::scenes(ctx.conn, &draft.id)?;
    let marks = vec!["?"; items.len()].join(",");
    let sql = format!(
        "SELECT DISTINCT scene_id FROM breakdown_element WHERE deleted_at IS NULL AND source_id = ? AND catalog_item_id IN ({marks})"
    );
    let mut stmt = ctx.conn.prepare(&sql)?;
    let mut p: Vec<&dyn rusqlite::ToSql> = vec![&source];
    for i in items {
        p.push(i);
    }
    let used: Vec<String> = stmt
        .query_map(p.as_slice(), |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let wanted = a.ints("sceneNumbers");
    let mut ids = Vec::new();
    let mut nums = Vec::new();
    for s in scenes.iter().filter(|s| used.contains(&s.id)) {
        if wanted.is_empty() || wanted.contains(&(s.number as u32)) {
            d.target(
                "screenplay_scene",
                &Found {
                    id: s.id.clone(),
                    label: s.label(),
                    rev: s.rev,
                },
            );
            ids.push(s.id.clone());
            nums.push(s.number.to_string());
        }
    }
    if ids.is_empty() {
        return Err(AppError::ai(
            "not_found",
            "None of those scenes use it in the Production Source breakdown.",
        ));
    }
    Ok((ids, nums))
}

fn catalog_replace(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed catalog replacement");
    let from = item(ctx, &a, "from", &mut d)?;
    let to = item(ctx, &a, "to", &mut d)?;
    if from.id == to.id {
        return Err(queries::ambiguous(
            "Those are the same catalog item. What should replace it?",
        ));
    }
    let (ids, nums) = scenes_using(ctx, &a, std::slice::from_ref(&from.id), &mut d)?;
    d.row("Replace", format!("{} → {}", from.label, to.label))
        .row("In scenes", queries::join_and(&nums))
        .row("Old item", "Kept in the catalog");
    d.op(
        "catalog.replace_in_scenes",
        json!({"fromId": from.id, "toId": to.id, "sceneIds": ids}),
        format!("Replace “{}” with “{}”", from.label, to.label),
    );
    d.done()
}

// ------------------------------------------------------------------ locations

fn location(ctx: &PropCtx<'_>, a: &A<'_>, key: &str, d: &mut Draft) -> AppResult<Found> {
    let f = rv::find(ctx.conn, ctx.actor, &rv::LOCATION, &a.req(key, "location")?)?;
    d.target("location", &f);
    Ok(f)
}

fn location_new(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let name = a.req("name", "location name")?;
    let dup = queries::count(
        ctx.conn,
        "SELECT count(*) FROM location WHERE deleted_at IS NULL AND lower(name)=lower(?1)",
        [&name],
    )?;
    if dup > 0 {
        return Err(queries::ambiguous(format!(
            "There is already a location called “{name}”. Should I change that one instead?"
        )));
    }
    let mut d = Draft::new(spec, args, "Proposed location");
    let mut op = json!({"name": name});
    d.row("Location", name.clone());
    for (key, label) in [
        ("address", "Address"),
        ("contact", "Contact"),
        ("status", "Status"),
    ] {
        if let Some(v) = a.s(key) {
            d.row(label, v.clone());
            op[key] = json!(v);
        }
    }
    d.row("Impact", "1 new object");
    d.op("locations.create", op, format!("Create location “{name}”"));
    d.done()
}

fn location_update(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed location edit");
    let loc = location(ctx, &a, "location", &mut d)?;
    let mut op = json!({"id": loc.id, "expectedRev": loc.rev});
    let mut changed = set_text(
        &mut d,
        &mut op,
        "name",
        "Name",
        a.s("name"),
        &loc.label,
        200,
    )?;
    for (key, label) in [("address", "Address"), ("contact", "Contact")] {
        let old = rv::column(ctx.conn, "location", key, &loc.id)?;
        changed |= set_text(&mut d, &mut op, key, label, a.raw(key), &old, 500)?;
    }
    let notes_json = rv::column(ctx.conn, "location", "notes_json", &loc.id)?;
    let mut notes: Value = serde_json::from_str(&notes_json).unwrap_or_else(|_| json!({}));
    if !notes.is_object() {
        notes = json!({});
    }
    let mut notes_changed = false;
    for k in NOTE_KEYS {
        if let Some(v) = a.raw(k) {
            let old = notes
                .get(*k)
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            if old.trim() != v.trim() {
                d.change(&format!("{} notes", title_case(k)), &old, &v);
                notes[*k] = json!(v);
                notes_changed = true;
            }
        }
    }
    if notes_changed {
        op["notes"] = notes;
        changed = true;
    }
    need_change(changed, &loc.label)?;
    d.op(
        "locations.update",
        op,
        format!("Edit location “{}”", loc.label),
    );
    d.done()
}

fn title_case(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn location_status(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed location status");
    let loc = location(ctx, &a, "location", &mut d)?;
    let status = a.req("status", "status")?;
    let old = rv::column(ctx.conn, "location", "status", &loc.id)?;
    if old == status {
        return Err(queries::ambiguous(format!(
            "“{}” is already {status}.",
            loc.label
        )));
    }
    d.change(&loc.label, &old, &status);
    d.op(
        "locations.set_status",
        json!({"id": loc.id, "status": status}),
        format!("Set “{}” to {status}", loc.label),
    );
    d.done()
}

fn location_photo(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed photo removal");
    let loc = location(ctx, &a, "location", &mut d)?;
    let n = a.u("photo").unwrap_or(1);
    let photo: Found = ctx
        .conn
        .query_row(
            "SELECT id, COALESCE(caption, ''), rev FROM location_photo WHERE location_id=?1 AND deleted_at IS NULL
             ORDER BY position, id LIMIT 1 OFFSET ?2",
            params![loc.id, (n.max(1) - 1) as i64],
            |r| Ok(Found { id: r.get(0)?, label: r.get(1)?, rev: r.get(2)? }),
        )
        .map_err(|_| AppError::ai("not_found", format!("“{}” has no photo {n}.", loc.label)))?;
    d.base("location_photo", &photo.id)
        .row(
            "Remove photo",
            format!(
                "Photo {n} of {}{}",
                loc.label,
                if photo.label.is_empty() {
                    String::new()
                } else {
                    format!(" (“{}”)", photo.label)
                }
            ),
        )
        .row("Recoverable", "Yes — from Recently Deleted");
    d.op(
        "locations.remove_photo",
        json!({"id": photo.id}),
        format!("Remove photo {n} of “{}”", loc.label),
    );
    d.done()
}

fn location_archive(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
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
    let loc = location(ctx, &a, "location", &mut d)?;
    d.row(
        if archived { "Archive" } else { "Unarchive" },
        loc.label.clone(),
    );
    d.op(
        "locations.set_archived",
        json!({"id": loc.id, "archived": archived}),
        format!(
            "{} “{}”",
            if archived { "Archive" } else { "Unarchive" },
            loc.label
        ),
    );
    d.done()
}

fn location_delete(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed location deletion");
    let loc = location(ctx, &a, "location", &mut d)?;
    d.row("Delete", loc.label.clone())
        .row("Recoverable", "Yes — from Recently Deleted");
    d.op(
        "locations.delete",
        json!({"id": loc.id}),
        format!("Delete location “{}”", loc.label),
    );
    d.done()
}

fn location_replace(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed location replacement");
    let loc = location(ctx, &a, "location", &mut d)?;
    let rep = location(ctx, &a, "replacement", &mut d)?;
    if loc.id == rep.id {
        return Err(queries::ambiguous(
            "Those are the same location. Which location should replace it?",
        ));
    }
    let mut stmt = ctx
        .conn
        .prepare("SELECT id FROM catalog_item WHERE location_id=?1 AND deleted_at IS NULL")?;
    let items: Vec<String> = stmt
        .query_map([&loc.id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    if items.is_empty() {
        return Err(AppError::ai(
            "not_found",
            format!("“{}” isn't used by any scene's breakdown yet.", loc.label),
        ));
    }
    let (ids, nums) = scenes_using(ctx, &a, &items, &mut d)?;
    d.row("Replace", format!("{} → {}", loc.label, rep.label))
        .row("In scenes", queries::join_and(&nums));
    d.op(
        "locations.replace",
        json!({"locationId": loc.id, "replacementId": rep.id, "sceneIds": ids}),
        format!("Replace “{}” with “{}”", loc.label, rep.label),
    );
    d.done()
}

// ------------------------------------------------------------------ cast & crew

fn person(ctx: &PropCtx<'_>, a: &A<'_>, key: &str, cast: bool, d: &mut Draft) -> AppResult<Found> {
    let e = if cast { &rv::CAST } else { &rv::CREW };
    let f = rv::find(ctx.conn, ctx.actor, e, &a.req(key, e.what)?)?;
    d.target(e.table, &f);
    Ok(f)
}

fn character_choice(
    ctx: &PropCtx<'_>,
    a: &A<'_>,
    op: &mut Value,
    d: &mut Draft,
) -> AppResult<Option<String>> {
    if let Some(ch) = rv::find_opt(
        ctx.conn,
        ctx.actor,
        &rv::CHARACTER,
        a.s("character").as_deref(),
    )? {
        d.base("story_character", &ch.id);
        op["characterId"] = json!(ch.id);
        return Ok(Some(ch.label));
    }
    if let Some(n) = a.s("characterName") {
        op["characterName"] = json!(n);
        return Ok(Some(n));
    }
    Ok(None)
}

fn cast_new(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let name = a.req("person", "performer's name")?;
    let mut d = Draft::new(spec, args, "Proposed cast member");
    let mut op = json!({"personName": name});
    let role = character_choice(ctx, &a, &mut op, &mut d)?;
    d.row("Performer", name.clone());
    if let Some(r) = &role {
        d.row("Plays", r.clone());
    }
    if let Some(p) = a.b("primary") {
        op["primary"] = json!(p);
        d.row("Primary", if p { "Yes" } else { "Alternate" });
    }
    for (key, op_key, label) in [
        ("contact", "contact", "Contact"),
        ("availability", "availabilityNotes", "Availability"),
        ("notes", "notes", "Notes"),
    ] {
        if let Some(v) = a.s(key) {
            d.row(label, queries::truncate_chars(&v, 300));
            op[op_key] = json!(v);
        }
    }
    d.op("cast.create", op, format!("Add cast member “{name}”"));
    d.done()
}

fn cast_update(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed cast edit");
    let p = person(ctx, &a, "castMember", true, &mut d)?;
    let mut op = json!({"id": p.id, "expectedRev": p.rev});
    let mut changed = set_text(
        &mut d,
        &mut op,
        "personName",
        "Name",
        a.s("person"),
        &p.label,
        200,
    )?;
    for (key, col, op_key, label, max) in [
        ("contact", "contact", "contact", "Contact", 500usize),
        (
            "availability",
            "availability_notes",
            "availabilityNotes",
            "Availability",
            2000,
        ),
        ("notes", "notes", "notes", "Notes", 4000),
    ] {
        let old = rv::column(ctx.conn, "cast_member", col, &p.id)?;
        changed |= set_text(&mut d, &mut op, op_key, label, a.raw(key), &old, max)?;
    }
    need_change(changed, &p.label)?;
    d.op("cast.update", op, format!("Edit cast member “{}”", p.label));
    d.done()
}

fn cast_assign(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed casting");
    let p = person(ctx, &a, "castMember", true, &mut d)?;
    let primary = a.b("primary").unwrap_or(true);
    let mut op = json!({"id": p.id, "primary": primary});
    let role = character_choice(ctx, &a, &mut op, &mut d)?
        .ok_or_else(|| queries::ambiguous("Which character should they play?"))?;
    d.row("Performer", p.label.clone())
        .row("Plays", role.clone())
        .row(
            "As",
            if primary {
                "Primary performer (others become alternates)"
            } else {
                "Alternate"
            },
        );
    d.op("cast.assign", op, format!("Cast “{}” as {role}", p.label));
    d.done()
}

fn crew_new(_ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let name = a.req("person", "crew member's name")?;
    let role = a.req("role", "crew role")?;
    let mut d = Draft::new(spec, args, "Proposed crew member");
    let mut op = json!({"personName": name, "role": role});
    d.row("Crew member", format!("{name} — {role}"));
    for (key, label) in [
        ("department", "Department"),
        ("contact", "Contact"),
        ("notes", "Notes"),
    ] {
        if let Some(v) = a.s(key) {
            d.row(label, queries::truncate_chars(&v, 300));
            op[key] = json!(v);
        }
    }
    d.op("crew.create", op, format!("Add crew member “{name}”"));
    d.done()
}

fn crew_update(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed crew edit");
    let p = person(ctx, &a, "crewMember", false, &mut d)?;
    let mut op = json!({"id": p.id, "expectedRev": p.rev});
    let mut changed = set_text(
        &mut d,
        &mut op,
        "personName",
        "Name",
        a.s("person"),
        &p.label,
        200,
    )?;
    for (key, label, max) in [
        ("role", "Role", 120usize),
        ("department", "Department", 120),
        ("contact", "Contact", 500),
        ("notes", "Notes", 4000),
    ] {
        let old = rv::column(ctx.conn, "crew_member", key, &p.id)?;
        let new = if key == "role" { a.s(key) } else { a.raw(key) };
        changed |= set_text(&mut d, &mut op, key, label, new, &old, max)?;
    }
    need_change(changed, &p.label)?;
    d.op("crew.update", op, format!("Edit crew member “{}”", p.label));
    d.done()
}

fn person_archive(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let cast = a.s("kind").as_deref() == Some("cast");
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
    let p = person(ctx, &a, "person", cast, &mut d)?;
    d.row(
        if archived { "Archive" } else { "Unarchive" },
        p.label.clone(),
    );
    let op = if cast {
        "cast.set_archived"
    } else {
        "crew.set_archived"
    };
    d.op(
        op,
        json!({"id": p.id, "archived": archived}),
        format!(
            "{} “{}”",
            if archived { "Archive" } else { "Unarchive" },
            p.label
        ),
    );
    d.done()
}

fn person_remove(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let cast = a.s("kind").as_deref() == Some("cast");
    let mut d = Draft::new(spec, args, "Proposed removal");
    let p = person(ctx, &a, "person", cast, &mut d)?;
    d.row(
        if cast {
            "Remove from cast"
        } else {
            "Remove from crew"
        },
        p.label.clone(),
    )
    .row("Recoverable", "Yes — from Recently Deleted");
    let op = if cast { "cast.delete" } else { "crew.delete" };
    d.op(op, json!({"id": p.id}), format!("Remove “{}”", p.label));
    d.done()
}
