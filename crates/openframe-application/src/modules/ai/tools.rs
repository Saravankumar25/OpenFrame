//! Deterministic AI tool registry (AI spec §10, §27–§28; FSD §42.5–§42.6).
//!
//! The model only *chooses* a tool and fills typed arguments. Every argument
//! struct denies unknown fields; every tool runs against canonical data with
//! the requesting user's permissions. Read/compute tools return exact values
//! computed here (the model is never the calculator); navigation tools return
//! a UI target; suggestion tools gather minimal context for generation;
//! mutation tools only ever produce a Change Set proposal (see `catalog`).

use std::collections::{BTreeMap, BTreeSet};

use openframe_domain::{Actor, AppError, AppResult, Capability};
use rusqlite::{Connection, params};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use super::queries::{self, DraftRef, SceneRow, join_and};
use super::scope::ResolvedScope;
use super::types::*;

/// Tool specification offered to the model (name, class, description, argument schema).
pub struct ToolSpec {
    pub name: &'static str,
    pub class: OperationClass,
    pub description: &'static str,
    pub schema: fn() -> Value,
}

// Strict, bounded argument schemas (agentic spec §39).
use super::toolbox::schema as sc;

fn no_args() -> Value {
    sc::none()
}
fn draft_arg() -> Value {
    sc::obj(&[("draft", sc::s(120))], &[])
}
fn characters_args() -> Value {
    sc::obj(
        &[
            ("characters", sc::arr(sc::s(120), 6)),
            ("draft", sc::s(120)),
        ],
        &["characters"],
    )
}
fn text_draft_args() -> Value {
    sc::obj(&[("text", sc::s(200)), ("draft", sc::s(120))], &["text"])
}
fn act_args() -> Value {
    sc::obj(&[("act", sc::s(200))], &["act"])
}
fn compare_args() -> Value {
    sc::obj(
        &[("fromDraft", sc::s(120)), ("toDraft", sc::s(120))],
        &["fromDraft", "toDraft"],
    )
}
fn text_args() -> Value {
    sc::obj(&[("text", sc::s(200))], &["text"])
}
fn open_scene_args() -> Value {
    sc::obj(
        &[("sceneNumber", sc::scene_number()), ("draft", sc::s(120))],
        &["sceneNumber"],
    )
}
fn workspace_args() -> Value {
    let names: Vec<&str> = WORKSPACES.iter().map(|(id, _)| *id).collect();
    sc::obj(&[("workspace", sc::en(&names))], &["workspace"])
}
fn scene_fact_args() -> Value {
    sc::obj(
        &[
            ("scene", sc::scene_number()),
            (
                "fact",
                sc::en(&[
                    "speakers",
                    "speaker_count",
                    "setting",
                    "time",
                    "first_line",
                    "last_line",
                    "first_dialogue",
                    "mentions",
                ]),
            ),
            ("text", sc::s(120)),
            ("draft", sc::s(120)),
        ],
        &["scene", "fact"],
    )
}
fn dialogue_fact_args() -> Value {
    sc::obj(
        &[
            ("character", sc::s(80)),
            ("scene", sc::scene_number()),
            (
                "fact",
                sc::en(&[
                    "blocks",
                    "words",
                    "scenes",
                    "first_scene",
                    "most",
                    "first_speaker",
                ]),
            ),
            ("draft", sc::s(120)),
        ],
        &["fact"],
    )
}
fn private_args() -> Value {
    sc::obj(&[("whose", sc::en(&["mine", "someone_else"]))], &["whose"])
}
fn scene_opt_args() -> Value {
    sc::obj(&[("sceneNumber", sc::scene_number())], &[])
}

/// Read / compute / navigate / suggest tools. Mutation proposal tools live in `catalog`.
pub const TOOLS: &[ToolSpec] = &[
    ToolSpec {
        name: "project_overview",
        class: OperationClass::Compute,
        description: "Project facts: title, type, status and headline counts across modules.",
        schema: no_args,
    },
    ToolSpec {
        name: "list_drafts",
        class: OperationClass::Read,
        description: "List screenplay drafts and which one is current.",
        schema: no_args,
    },
    ToolSpec {
        name: "count_scenes",
        class: OperationClass::Compute,
        description: "Exact number of scenes in a draft (default: the draft in scope).",
        schema: draft_arg,
    },
    ToolSpec {
        name: "count_characters",
        class: OperationClass::Compute,
        description: "Exact number of distinct character cues in a draft, plus Character records.",
        schema: draft_arg,
    },
    ToolSpec {
        name: "list_characters",
        class: OperationClass::Read,
        description: "List the characters who speak in a draft with their scene counts.",
        schema: draft_arg,
    },
    ToolSpec {
        name: "scenes_with_characters",
        class: OperationClass::Compute,
        description: "Scenes where ALL the named characters speak.",
        schema: characters_args,
    },
    ToolSpec {
        name: "find_scenes_mentioning",
        class: OperationClass::Read,
        description: "Scenes whose heading or text mentions a place, prop or phrase.",
        schema: text_draft_args,
    },
    ToolSpec {
        name: "location_statistics",
        class: OperationClass::Compute,
        description: "Distinct screenplay locations, interior/exterior and day/night counts, plus Location records.",
        schema: draft_arg,
    },
    ToolSpec {
        name: "locations_in_act",
        class: OperationClass::Compute,
        description: "Locations used by the Scene Cards of a Beat Board act.",
        schema: act_args,
    },
    ToolSpec {
        name: "unscheduled_scenes",
        class: OperationClass::Compute,
        description: "Scenes of the Production Source draft not yet placed on the shooting schedule.",
        schema: no_args,
    },
    ToolSpec {
        name: "compare_drafts",
        class: OperationClass::Compute,
        description: "Scenes added, removed, changed or moved between two drafts.",
        schema: compare_args,
    },
    ToolSpec {
        name: "breakdown_status",
        class: OperationClass::Compute,
        description: "Breakdown progress: confirmed vs suggested elements and scenes without a breakdown.",
        schema: no_args,
    },
    ToolSpec {
        name: "story_statistics",
        class: OperationClass::Compute,
        description: "Beat Board counts: acts, sequences, beats, Scene Cards, parked cards, characters.",
        schema: no_args,
    },
    ToolSpec {
        name: "production_statistics",
        class: OperationClass::Compute,
        description: "Production counts: catalog items by category, locations, cast, crew.",
        schema: no_args,
    },
    ToolSpec {
        name: "scene_fact",
        class: OperationClass::Compute,
        description: "One fact about a numbered screenplay scene, read from its full text: summary, who speaks, how many speak, where, day or night, first or last line, first dialogue, or whether it mentions something.",
        schema: scene_fact_args,
    },
    ToolSpec {
        name: "dialogue_fact",
        class: OperationClass::Compute,
        description: "Dialogue facts from the screenplay: how many dialogue blocks or words a character has (in the script or one scene), which scenes they speak in, their first speaking scene, who has the most dialogue, who speaks first.",
        schema: dialogue_fact_args,
    },
    ToolSpec {
        name: "search_project",
        class: OperationClass::Read,
        description: "Find every place in the project that mentions some words.",
        schema: text_args,
    },
    ToolSpec {
        name: "open_scene",
        class: OperationClass::Navigate,
        description: "Open a screenplay scene by number.",
        schema: open_scene_args,
    },
    ToolSpec {
        name: "open_workspace",
        class: OperationClass::Navigate,
        description: "Open an OpenFrame workspace.",
        schema: workspace_args,
    },
    ToolSpec {
        name: "open_object",
        class: OperationClass::Navigate,
        description: "Open the best match for a named object (character, location, file, idea…).",
        schema: text_args,
    },
    ToolSpec {
        name: "suggest_breakdown",
        class: OperationClass::Suggest,
        description: "Suggest likely breakdown elements (props, wardrobe, vehicles…) for a scene.",
        schema: scene_opt_args,
    },
    ToolSpec {
        name: "private_information",
        class: OperationClass::Read,
        description: "The user asks for private notes (their own, or someone else's).",
        schema: private_args,
    },
];

pub const WORKSPACES: &[(&str, &str)] = &[
    ("home", "Home"),
    ("vault", "Idea Vault"),
    ("story", "Story"),
    ("screenplay", "Screenplay"),
    ("breakdown", "Breakdown"),
    ("production", "Production"),
    ("callsheets", "Call Sheets"),
    ("files", "Files"),
    ("notes", "Notes & Tasks"),
    ("activity", "Activity"),
    ("trash", "Recently Deleted"),
    ("settings", "Project settings"),
];

pub fn spec(name: &str) -> Option<&'static ToolSpec> {
    TOOLS.iter().find(|t| t.name == name)
}

/// Deserialize tool arguments strictly (unknown fields rejected).
pub fn parse_args<T: DeserializeOwned>(tool: &str, args: &Value) -> AppResult<T> {
    let v = if args.is_null() {
        json!({})
    } else {
        args.clone()
    };
    serde_json::from_value(v).map_err(|e| {
        AppError::ai(
            "tool_arguments",
            "I couldn't understand the details of that request.",
        )
        .with_detail(format!("{tool}: {e}"))
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NoArgs {}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DraftArg {
    #[serde(default)]
    draft: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SceneFactArgs {
    scene: u32,
    fact: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    draft: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DialogueFactArgs {
    #[serde(default)]
    character: Option<String>,
    #[serde(default)]
    scene: Option<u32>,
    fact: String,
    #[serde(default)]
    draft: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CharactersArgs {
    characters: Vec<String>,
    #[serde(default)]
    draft: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TextDraftArgs {
    text: String,
    #[serde(default)]
    draft: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ActArgs {
    act: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CompareArgs {
    from_draft: String,
    to_draft: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextArgs {
    pub text: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OpenSceneArgs {
    scene_number: u32,
    #[serde(default)]
    draft: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WorkspaceArgs {
    workspace: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PrivateArgs {
    whose: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SceneOptArgs {
    #[serde(default)]
    pub scene_number: Option<u32>,
}

/// Work the action path must finish with the model: the breakdown suggestions of one scene (a JSON list
/// the person confirms in Breakdown; nothing is added to the project).
#[derive(Debug, Clone)]
pub enum Generation {
    Breakdown {
        scene: Box<SceneRow>,
        draft: DraftRef,
        text: String,
    },
}

/// The outcome of one tool call.
#[derive(Debug, Clone)]
pub struct ToolOutput {
    pub kind: AiResultKind,
    pub content: String,
    pub details: Vec<String>,
    pub provenance: Vec<Provenance>,
    pub confidence: Option<Confidence>,
    pub items: Vec<ResultItem>,
    pub nav: Option<NavTarget>,
    pub structured: Option<Value>,
    pub targets: Vec<ObjRef>,
    pub generation: Option<Generation>,
    /// True when `items` ARE the answer (the person asked for a list). Otherwise they are the
    /// evidence behind the text and the screen keeps them under "See sources".
    pub items_are_answer: bool,
}

impl ToolOutput {
    pub fn exact(content: impl Into<String>) -> Self {
        Self {
            kind: AiResultKind::Answer,
            content: content.into(),
            details: Vec::new(),
            provenance: Vec::new(),
            confidence: Some(Confidence::Exact),
            items: Vec::new(),
            nav: None,
            structured: None,
            targets: Vec::new(),
            generation: None,
            items_are_answer: false,
        }
    }
    pub fn of_kind(kind: AiResultKind, content: impl Into<String>) -> Self {
        let mut o = Self::exact(content);
        o.kind = kind;
        o.confidence = None;
        o
    }
    pub(crate) fn detail(mut self, d: impl Into<String>) -> Self {
        self.details.push(d.into());
        self
    }
    pub(crate) fn prov(mut self, kind: &str, label: impl Into<String>) -> Self {
        self.provenance.push(Provenance::new(kind, label));
        self
    }
}

pub struct ToolCtx<'a> {
    /// The application core, for the few reads that answer from app-level data (appearance, shortcuts).
    pub core: &'a crate::core::AppCore,
    pub conn: &'a Connection,
    pub actor: &'a Actor,
    pub scope: &'a ResolvedScope,
    /// The user's words (used by tools that need the original phrasing).
    pub request_text: &'a str,
}

impl ToolCtx<'_> {
    /// The draft a tool works on. A draft name the model supplied that matches no draft and that
    /// the person never wrote is the small model filling an optional argument from context (for
    /// example "scene 1" from the scene in view); the tool then uses the draft in scope, which
    /// the answer names in its sources. A draft the person named that does not exist is still
    /// reported as not found.
    fn draft(&self, reference: Option<&str>) -> AppResult<DraftRef> {
        match queries::resolve_draft(self.conn, reference, self.scope.draft.as_ref()) {
            Err(e)
                if e.code_str() == "ai.not_found"
                    && reference.is_some_and(|r| !mentioned_in(self.request_text, r)) =>
            {
                queries::resolve_draft(self.conn, None, self.scope.draft.as_ref())
            }
            other => other,
        }
    }
}

/// Did the person's request contain `reference` (ignoring case, quotes and spacing)?
fn mentioned_in(request: &str, reference: &str) -> bool {
    let norm = |t: &str| {
        t.to_lowercase()
            .chars()
            .filter(|c| !matches!(c, '"' | '\'' | '“' | '”' | '‘' | '’'))
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    let r = norm(reference);
    !r.is_empty() && norm(request).contains(&r)
}

pub(crate) fn plural(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {many}")
    }
}

pub(crate) fn scene_nav(draft: &DraftRef, s: &SceneRow) -> NavTarget {
    let nav = NavTarget::to("screenplay")
        .with("draftId", &draft.id)
        .with("sceneId", &s.id);
    match &draft.episode_id {
        Some(e) => nav.with("episodeId", e),
        None => nav,
    }
}

pub(crate) fn scene_items(draft: &DraftRef, scenes: &[&SceneRow]) -> Vec<ResultItem> {
    scenes
        .iter()
        .map(|s| ResultItem {
            label: s.label(),
            detail: None,
            nav: Some(scene_nav(draft, s)),
        })
        .collect()
}

/// Run a read/compute/navigate/suggest tool.
pub fn execute(ctx: &ToolCtx<'_>, name: &str, args: &Value) -> AppResult<ToolOutput> {
    ctx.actor.require(Capability::View, "view this project")?;
    match name {
        "project_overview" => {
            let _: NoArgs = parse_args(name, args)?;
            project_overview(ctx)
        }
        "list_drafts" => {
            let _: NoArgs = parse_args(name, args)?;
            list_drafts(ctx)
        }
        "count_scenes" => {
            let a: DraftArg = parse_args(name, args)?;
            count_scenes(ctx, a.draft.as_deref())
        }
        "count_characters" => {
            let a: DraftArg = parse_args(name, args)?;
            count_characters(ctx, a.draft.as_deref())
        }
        "list_characters" => {
            let a: DraftArg = parse_args(name, args)?;
            list_characters(ctx, a.draft.as_deref())
        }
        "scenes_with_characters" => {
            let a: CharactersArgs = parse_args(name, args)?;
            scenes_with_characters(ctx, &a.characters, a.draft.as_deref())
        }
        "find_scenes_mentioning" => {
            let a: TextDraftArgs = parse_args(name, args)?;
            find_scenes_mentioning(ctx, &a.text, a.draft.as_deref())
        }
        "location_statistics" => {
            let a: DraftArg = parse_args(name, args)?;
            location_statistics(ctx, a.draft.as_deref())
        }
        "locations_in_act" => {
            let a: ActArgs = parse_args(name, args)?;
            locations_in_act(ctx, &a.act)
        }
        "unscheduled_scenes" => {
            let _: NoArgs = parse_args(name, args)?;
            unscheduled_scenes(ctx)
        }
        "compare_drafts" => {
            let a: CompareArgs = parse_args(name, args)?;
            compare_drafts(ctx, &a.from_draft, &a.to_draft)
        }
        "breakdown_status" => {
            let _: NoArgs = parse_args(name, args)?;
            breakdown_status(ctx)
        }
        "story_statistics" => {
            let _: NoArgs = parse_args(name, args)?;
            story_statistics(ctx)
        }
        "production_statistics" => {
            let _: NoArgs = parse_args(name, args)?;
            production_statistics(ctx)
        }
        "scene_fact" => {
            let a: SceneFactArgs = parse_args(name, args)?;
            scene_fact(ctx, &a)
        }
        "dialogue_fact" => {
            let a: DialogueFactArgs = parse_args(name, args)?;
            dialogue_fact(ctx, &a)
        }
        "search_project" => {
            let a: TextArgs = parse_args(name, args)?;
            search_project(ctx, &a.text)
        }
        "open_scene" => {
            let a: OpenSceneArgs = parse_args(name, args)?;
            open_scene(ctx, a.scene_number, a.draft.as_deref())
        }
        "open_workspace" => {
            let a: WorkspaceArgs = parse_args(name, args)?;
            open_workspace(&a.workspace)
        }
        "open_object" => {
            let a: TextArgs = parse_args(name, args)?;
            open_object(ctx, &a.text)
        }
        "suggest_breakdown" => {
            let a: SceneOptArgs = parse_args(name, args)?;
            suggest_breakdown(ctx, a.scene_number)
        }
        "private_information" => {
            let a: PrivateArgs = parse_args(name, args)?;
            private_information(ctx, &a.whose)
        }
        other => Err(AppError::ai(
            "unknown_tool",
            "That isn't something the assistant can do yet.",
        )
        .with_detail(format!("tool {other}"))),
    }
}

// --------------------------------------------------------------------- reads

fn project_overview(ctx: &ToolCtx<'_>) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    let (title, ptype, status): (String, String, String) = c.query_row(
        "SELECT title, project_type, status FROM project LIMIT 1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    let drafts = queries::list_drafts(c)?;
    let currents = queries::current_drafts(c)?;
    let mut o = ToolOutput::exact(format!("{title} is a {ptype}. Its status is {status}."));
    if currents.len() > 1 {
        o = o.detail(format!("Screenplays: {}.", currents.len()));
    }
    // One line per screenplay: a project may hold several (ADR-0012).
    for d in &currents {
        let n = queries::scenes(c, &d.id)?.len();
        o = o.detail(format!(
            "Current draft: {} with {}.",
            d.label(),
            plural(n, "scene", "scenes")
        ));
    }
    o = o.detail(format!("Screenplay drafts: {}.", drafts.len()));
    let cards = queries::count(
        c,
        "SELECT count(*) FROM story_scene_card WHERE deleted_at IS NULL",
        [],
    )?;
    let chars = queries::character_records(c)?.len();
    let locs = queries::count(
        c,
        "SELECT count(*) FROM location WHERE deleted_at IS NULL AND archived=0",
        [],
    )?;
    let open_tasks = queries::count(
        c,
        "SELECT count(*) FROM task WHERE deleted_at IS NULL AND status='Open'",
        [],
    )?;
    o = o
        .detail(format!(
            "Beat Board: {}.",
            plural(cards as usize, "Scene Card", "Scene Cards")
        ))
        .detail(format!(
            "Character directory: {}.",
            plural(chars, "Character record", "Character records")
        ))
        .detail(format!(
            "Production: {}.",
            plural(locs as usize, "Location record", "Location records")
        ))
        .detail(format!("Open tasks: {open_tasks}."))
        .prov("Scope", "Whole Project");
    Ok(o)
}

fn list_drafts(ctx: &ToolCtx<'_>) -> AppResult<ToolOutput> {
    let drafts = queries::list_drafts(ctx.conn)?;
    if drafts.is_empty() {
        return Ok(
            ToolOutput::exact("This project doesn't have a screenplay draft yet.")
                .prov("Scope", "Screenplay"),
        );
    }
    let mut o = ToolOutput::exact(format!(
        "The screenplay has {}.",
        plural(drafts.len(), "draft", "drafts")
    ));
    for d in &drafts {
        let mut detail = d.status.clone();
        if d.is_current {
            detail.push_str(" · current");
        }
        o.items.push(ResultItem {
            label: d.label(),
            detail: Some(detail),
            nav: Some(NavTarget::to("screenplay").with("draftId", &d.id)),
        });
    }
    Ok(o.prov("Scope", "Screenplay drafts"))
}

fn count_scenes(ctx: &ToolCtx<'_>, draft: Option<&str>) -> AppResult<ToolOutput> {
    let d = ctx.draft(draft)?;
    let scenes = queries::scenes(ctx.conn, &d.id)?;
    let omitted = scenes.iter().filter(|s| s.omitted).count();
    // Omitted scenes are part of the answer, not a hidden detail (AI context brief, question 2).
    let o = ToolOutput::exact(if omitted > 0 {
        format!(
            "{} contains {}; {} marked omitted.",
            d.label(),
            plural(scenes.len(), "scene", "scenes"),
            plural(omitted, "is", "are")
        )
    } else {
        format!(
            "{} contains {}.",
            d.label(),
            plural(scenes.len(), "scene", "scenes")
        )
    });
    let mut o = o;
    o.structured = Some(json!({"draftId": d.id, "sceneCount": scenes.len(), "omitted": omitted}));
    Ok(o.prov("Draft", d.label())
        .prov("Basis", "screenplay scenes in the draft"))
}

fn count_characters(ctx: &ToolCtx<'_>, draft: Option<&str>) -> AppResult<ToolOutput> {
    let d = ctx.draft(draft)?;
    let cues = queries::character_cues(ctx.conn, &d.id)?;
    let records = queries::character_records(ctx.conn)?;
    let mut o = ToolOutput::exact(format!(
        "In {} there are {}.",
        d.label(),
        plural(
            cues.len(),
            "distinct character cue",
            "distinct character cues"
        )
    ))
    .detail(format!(
        "The Character directory contains {}.",
        plural(records.len(), "Character record", "Character records")
    ));
    o.structured = Some(
        json!({"draftId": d.id, "distinctCues": cues.len(), "characterRecords": records.len()}),
    );
    Ok(o.prov("Draft", d.label())
        .prov("Basis", "screenplay Character elements"))
}

fn list_characters(ctx: &ToolCtx<'_>, draft: Option<&str>) -> AppResult<ToolOutput> {
    let d = ctx.draft(draft)?;
    let cues = queries::character_cues(ctx.conn, &d.id)?;
    let mut sorted: Vec<(&String, &Vec<String>)> = cues.iter().collect();
    sorted.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(b.0)));
    let mut o = ToolOutput::exact(format!(
        "{} characters speak in {}.",
        sorted.len(),
        d.label()
    ));
    o.items = sorted
        .iter()
        .map(|(name, scenes)| ResultItem {
            label: (*name).clone(),
            detail: Some(plural(scenes.len(), "scene", "scenes")),
            nav: None,
        })
        .collect();
    Ok(o.prov("Draft", d.label())
        .prov("Basis", "screenplay Character elements"))
}

/// The cue that names `wanted`: the exact name, else the one cue that starts with it ("KARTHIK"
/// for "Karthik"), never a guess between several.
fn match_cue<'a>(cues: &'a BTreeMap<String, Vec<String>>, wanted: &str) -> Option<&'a str> {
    let w = queries::normalize_cue(wanted);
    if w.is_empty() {
        return None;
    }
    if let Some((k, _)) = cues.get_key_value(&w) {
        return Some(k.as_str());
    }
    let starts: Vec<&String> = cues
        .keys()
        .filter(|k| k.split_whitespace().next() == Some(w.as_str()))
        .collect();
    match starts.as_slice() {
        [one] => Some(one.as_str()),
        _ => {
            // "KARTHIK" over "KARTHIK'S FATHER": the shortest cue that starts with the name.
            starts
                .iter()
                .min_by_key(|k| k.len())
                .map(|k| k.as_str())
                .filter(|k| k.starts_with(&w))
        }
    }
}

/// A screenplay line in quotes that ends the sentence: “Here.” keeps its own full stop and gets
/// no second one; “Wait” gets one after the closing quote.
fn quoted_end(t: &str) -> String {
    let t = t.trim();
    if t.ends_with(['.', '!', '?', '\u{2026}']) {
        format!("\u{201c}{t}\u{201d}")
    } else {
        format!("\u{201c}{t}\u{201d}.")
    }
}

/// The live, ordered elements of one scene: (element type, text). Notes are the writer's own and
/// never part of the scene's story text.
fn scene_elements(c: &Connection, scene_id: &str) -> AppResult<Vec<(String, String)>> {
    let mut stmt = c.prepare(
        "SELECT element_type, text FROM screenplay_element WHERE scene_id=?1 AND element_type<>'note' ORDER BY position, id",
    )?;
    let rows: Vec<(String, String)> = stmt
        .query_map([scene_id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    Ok(rows
        .into_iter()
        .filter(|(_, t)| !t.trim().is_empty())
        .collect())
}

/// Dialogue blocks in order: (speaker cue, dialogue text). A block is one character cue followed by
/// its dialogue; parentheticals belong to the block and are not counted as dialogue.
pub(super) fn dialogue_blocks(rows: &[(String, String)]) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut speaker: Option<String> = None;
    let mut open = false;
    for (t, text) in rows {
        match t.as_str() {
            "character" => {
                speaker = Some(queries::normalize_cue(text));
                open = false;
            }
            "dialogue" => {
                if let Some(s) = &speaker {
                    if open {
                        if let Some(last) = out.last_mut() {
                            last.1.push(' ');
                            last.1.push_str(text.trim());
                        }
                    } else {
                        out.push((s.clone(), text.trim().to_string()));
                        open = true;
                    }
                }
            }
            "parenthetical" => {}
            _ => {
                speaker = None;
                open = false;
            }
        }
    }
    out
}

fn scene_by_number(
    ctx: &ToolCtx<'_>,
    n: u32,
    draft: Option<&str>,
) -> AppResult<(DraftRef, SceneRow)> {
    let d = ctx.draft(draft)?;
    let s = queries::scenes(ctx.conn, &d.id)?
        .into_iter()
        .find(|s| s.number == n as usize)
        .ok_or_else(|| AppError::ai("not_found", format!("{} has no Scene {n}.", d.label())))?;
    Ok((d, s))
}

/// One fact about a numbered scene, always read from that scene's own rows in the draft in scope.
fn scene_fact(ctx: &ToolCtx<'_>, a: &SceneFactArgs) -> AppResult<ToolOutput> {
    let (d, s) = scene_by_number(ctx, a.scene, a.draft.as_deref())?;
    let rows = scene_elements(ctx.conn, &s.id)?;
    let name = format!("Scene {}", s.number);
    let blocks = dialogue_blocks(&rows);
    let mut speakers: Vec<String> = Vec::new();
    for (who, _) in &blocks {
        if !speakers.contains(who) {
            speakers.push(who.clone());
        }
    }
    let title = |w: &str| {
        let mut c = w.to_lowercase().chars().collect::<Vec<_>>();
        if let Some(f) = c.first_mut() {
            *f = f.to_ascii_uppercase();
        }
        c.into_iter().collect::<String>()
    };
    let heading = queries::parse_heading(&s.heading);
    let content = match a.fact.as_str() {
        "speakers" => match speakers.len() {
            0 => format!("Nobody speaks in {name}."),
            1 => format!("{} speaks in {name}.", title(&speakers[0])),
            _ => format!(
                "{} speak in {name}.",
                join_and(&speakers.iter().map(|w| title(w)).collect::<Vec<_>>())
            ),
        },
        "speaker_count" => format!(
            "{} in {name}.",
            match speakers.len() {
                0 => "No characters speak".to_string(),
                1 => "1 character speaks".to_string(),
                n => format!("{n} characters speak"),
            }
        ),
        "setting" => {
            if heading.location.trim().is_empty() {
                format!("{name} has no location in its heading.")
            } else {
                format!("{name} takes place at {}.", heading.location.trim())
            }
        }
        "time" => match heading
            .time
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty())
        {
            Some(t) => format!("{name} is set at {}.", t.to_lowercase()),
            None => format!("{name}'s heading does not say the time of day."),
        },
        "first_line" | "last_line" => {
            let body: Vec<&(String, String)> =
                rows.iter().filter(|(t, _)| t != "scene_heading").collect();
            let pick = if a.fact == "first_line" {
                body.first()
            } else {
                body.last()
            };
            match pick {
                Some((_, t)) => format!(
                    "The {} line of {name} is {}",
                    if a.fact == "first_line" {
                        "first"
                    } else {
                        "last"
                    },
                    quoted_end(t)
                ),
                None => format!("{name} has no lines yet."),
            }
        }
        "first_dialogue" => match blocks.first() {
            Some((who, t)) => format!("{} first says {}", title(who), quoted_end(t)),
            None => format!("Nobody speaks in {name}."),
        },
        "mentions" => {
            let what = a.text.as_deref().unwrap_or("").trim();
            if what.is_empty() {
                return Err(queries::ambiguous("What should I look for in the scene?"));
            }
            let w = what.to_lowercase();
            let hit = rows.iter().find(|(_, t)| t.to_lowercase().contains(&w));
            match hit {
                Some((_, t)) => format!(
                    "Yes, {name} mentions {what}: \u{201c}{}\u{201d}",
                    queries::truncate_chars(t.trim(), 200)
                ),
                None => format!("No, {name} does not mention {what}."),
            }
        }
        other => {
            return Err(AppError::ai(
                "unknown_tool",
                "That isn't something the assistant can do yet.",
            )
            .with_detail(format!("scene fact {other}")));
        }
    };
    let mut o = ToolOutput::exact(content);
    o.nav = Some(scene_nav(&d, &s));
    Ok(o.prov("Draft", d.label()).prov("Scene", s.label()))
}

/// Dialogue facts over the whole draft or one scene, computed from the screenplay rows.
fn dialogue_fact(ctx: &ToolCtx<'_>, a: &DialogueFactArgs) -> AppResult<ToolOutput> {
    let d = ctx.draft(a.draft.as_deref())?;
    let scenes = queries::scenes(ctx.conn, &d.id)?;
    let in_scope: Vec<&SceneRow> = match a.scene {
        Some(n) => {
            let s = scenes
                .iter()
                .find(|s| s.number == n as usize)
                .ok_or_else(|| {
                    AppError::ai("not_found", format!("{} has no Scene {n}.", d.label()))
                })?;
            vec![s]
        }
        None => scenes.iter().filter(|s| !s.omitted).collect(),
    };
    let mut per_scene: Vec<(&SceneRow, Vec<(String, String)>)> = Vec::new();
    for s in &in_scope {
        per_scene.push((s, dialogue_blocks(&scene_elements(ctx.conn, &s.id)?)));
    }
    let where_ = match a.scene {
        Some(n) => format!("Scene {n}"),
        None => d.label(),
    };
    let words = |t: &str| {
        t.split_whitespace()
            .filter(|w| w.chars().any(char::is_alphanumeric))
            .count()
    };
    let title = |w: &str| {
        w.split_whitespace()
            .map(|p| {
                let mut c = p.to_lowercase().chars().collect::<Vec<_>>();
                if let Some(f) = c.first_mut() {
                    *f = f.to_ascii_uppercase();
                }
                c.into_iter().collect::<String>()
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    let content = match a.fact.as_str() {
        "most" => {
            let mut tally: BTreeMap<String, usize> = BTreeMap::new();
            for (_, b) in &per_scene {
                for (w, _) in b {
                    *tally.entry(w.clone()).or_default() += 1;
                }
            }
            match tally.iter().max_by(|x, y| x.1.cmp(y.1).then(y.0.cmp(x.0))) {
                Some((w, n)) => format!(
                    "{} has the most dialogue in {where_}: {}.",
                    title(w),
                    plural(*n, "dialogue block", "dialogue blocks")
                ),
                None => format!("Nobody speaks in {where_}."),
            }
        }
        "first_speaker" => match per_scene
            .iter()
            .find_map(|(s, b)| b.first().map(|x| (s, x)))
        {
            Some((s, (w, t))) => format!(
                "{} speaks first, in Scene {}: {}",
                title(w),
                s.number,
                quoted_end(&queries::truncate_chars(t, 160))
            ),
            None => format!("Nobody speaks in {where_}."),
        },
        fact => {
            let raw = a.character.as_deref().unwrap_or("").trim();
            if raw.is_empty() {
                return Err(queries::ambiguous("Which character do you mean?"));
            }
            let cues = queries::character_cues(ctx.conn, &d.id)?;
            let Some(cue) = match_cue(&cues, raw).map(str::to_string) else {
                return Err(AppError::ai(
                    "not_found",
                    format!("I couldn't find a character called {raw} in {}.", d.label()),
                ));
            };
            let mine: Vec<(u32, &String)> = per_scene
                .iter()
                .flat_map(|(s, b)| {
                    b.iter()
                        .filter(|(w, _)| *w == cue)
                        .map(move |(_, t)| (s.number as u32, t))
                })
                .collect();
            let name = title(&cue);
            match fact {
                "blocks" => format!(
                    "{name} has {} in {where_}.",
                    plural(mine.len(), "dialogue block", "dialogue blocks")
                ),
                "words" => format!(
                    "{name} speaks {} in {where_}.",
                    plural(mine.iter().map(|(_, t)| words(t)).sum(), "word", "words")
                ),
                "scenes" | "first_scene" => {
                    let mut nums: Vec<u32> = mine.iter().map(|(n, _)| *n).collect();
                    nums.dedup();
                    if nums.is_empty() {
                        format!("{name} has no dialogue in {where_}.")
                    } else if fact == "first_scene" {
                        format!("{name} first speaks in Scene {}.", nums[0])
                    } else {
                        let list: Vec<String> = nums.iter().map(u32::to_string).collect();
                        format!(
                            "{name} speaks in {} {}.",
                            if nums.len() == 1 { "Scene" } else { "Scenes" },
                            join_and(&list)
                        )
                    }
                }
                other => {
                    return Err(AppError::ai(
                        "unknown_tool",
                        "That isn't something the assistant can do yet.",
                    )
                    .with_detail(format!("dialogue fact {other}")));
                }
            }
        }
    };
    Ok(ToolOutput::exact(content).prov("Draft", d.label()).prov(
        "Basis",
        "screenplay dialogue blocks (a character cue and its dialogue)",
    ))
}

fn scenes_with_characters(
    ctx: &ToolCtx<'_>,
    names: &[String],
    draft: Option<&str>,
) -> AppResult<ToolOutput> {
    let d = ctx.draft(draft)?;
    let cues = queries::character_cues(ctx.conn, &d.id)?;
    let wanted: Vec<String> = names
        .iter()
        .map(|n| queries::normalize_cue(n))
        .filter(|n| !n.is_empty())
        .collect();
    if wanted.is_empty() {
        return Err(queries::ambiguous("Which characters should I look for?"));
    }
    for w in &wanted {
        if !cues.contains_key(w) {
            // Offer close matches instead of guessing (AI spec §6.3).
            let close: Vec<String> = cues
                .keys()
                .filter(|k| k.contains(w.as_str()) || w.contains(k.as_str()))
                .cloned()
                .collect();
            if close.len() > 1 {
                return Err(queries::ambiguous(format!(
                    "There are {} characters whose names match “{w}”: {}. Which one should I use?",
                    close.len(),
                    queries::join_and(&close)
                )));
            }
        }
    }
    let resolve = |w: &String| -> Option<&Vec<String>> {
        cues.get(w).or_else(|| {
            let close: Vec<&String> = cues.keys().filter(|k| k.contains(w.as_str())).collect();
            (close.len() == 1).then(|| &cues[close[0]])
        })
    };
    let mut common: Option<BTreeSet<&String>> = None;
    let mut missing = Vec::new();
    for w in &wanted {
        match resolve(w) {
            Some(scenes) => {
                let set: BTreeSet<&String> = scenes.iter().collect();
                common = Some(match common {
                    None => set,
                    Some(prev) => prev.intersection(&set).copied().collect(),
                });
            }
            None => missing.push(w.clone()),
        }
    }
    let scenes = queries::scenes(ctx.conn, &d.id)?;
    let hits: Vec<&SceneRow> = match (&common, missing.is_empty()) {
        (Some(set), true) => scenes.iter().filter(|s| set.contains(&s.id)).collect(),
        _ => Vec::new(),
    };
    let who = join_and(&wanted.iter().map(|w| title_case(w)).collect::<Vec<_>>());
    let content = if !missing.is_empty() {
        format!(
            "{} doesn't speak in {}, so no scenes contain {}.",
            join_and(&missing.iter().map(|w| title_case(w)).collect::<Vec<_>>()),
            d.label(),
            if wanted.len() > 1 {
                "all of them"
            } else {
                "that character"
            }
        )
    } else if hits.is_empty() {
        format!("No scene in {} contains {who} together.", d.label())
    } else {
        let nums: Vec<String> = hits.iter().map(|s| s.number.to_string()).collect();
        let what = match wanted.len() {
            1 => title_case(&wanted[0]),
            2 => "both".to_string(),
            _ => "all of them".to_string(),
        };
        let (noun, verb) = if hits.len() == 1 {
            ("Scene", "contains")
        } else {
            ("Scenes", "contain")
        };
        format!("{noun} {} {verb} {what}.", join_and(&nums))
    };
    let mut o = ToolOutput::exact(content);
    o.items = scene_items(&d, &hits);
    o.structured = Some(
        json!({"draftId": d.id, "sceneIds": hits.iter().map(|s| s.id.clone()).collect::<Vec<_>>()}),
    );
    Ok(o.prov("Draft", d.label()).prov(
        "Basis",
        "characters with dialogue (Character cues) in each scene",
    ))
}

pub(crate) fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn like_pattern(text: &str) -> String {
    let escaped = text
        .trim()
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{escaped}%")
}

fn find_scenes_mentioning(
    ctx: &ToolCtx<'_>,
    text: &str,
    draft: Option<&str>,
) -> AppResult<ToolOutput> {
    let text = text.trim();
    if text.chars().count() < 2 {
        return Err(queries::ambiguous("What should I look for?"));
    }
    let d = ctx.draft(draft)?;
    let scenes = queries::scenes(ctx.conn, &d.id)?;
    let pat = like_pattern(text);
    let mut in_heading = BTreeSet::new();
    let mut in_text = BTreeSet::new();
    for s in &scenes {
        if s.heading.to_lowercase().contains(&text.to_lowercase()) {
            in_heading.insert(s.id.clone());
        }
    }
    let mut stmt = ctx.conn.prepare(
        "SELECT DISTINCT e.scene_id FROM screenplay_element e JOIN screenplay_scene s ON s.id=e.scene_id
         WHERE s.draft_id=?1 AND s.deleted_at IS NULL AND e.element_type <> 'note' AND e.text LIKE ?2 ESCAPE '\\'",
    )?;
    for id in stmt.query_map(params![d.id, pat], |r| r.get::<_, String>(0))? {
        in_text.insert(id?);
    }
    let hits: Vec<&SceneRow> = scenes
        .iter()
        .filter(|s| in_heading.contains(&s.id) || in_text.contains(&s.id))
        .collect();
    let nums = |ids: &BTreeSet<String>| -> Vec<String> {
        scenes
            .iter()
            .filter(|s| ids.contains(&s.id))
            .map(|s| s.number.to_string())
            .collect()
    };
    let mut o = if hits.is_empty() {
        ToolOutput::exact(format!("No scene in {} mentions “{text}”.", d.label()))
    } else {
        let all: Vec<String> = hits.iter().map(|s| s.number.to_string()).collect();
        ToolOutput::exact(format!(
            "{} {} “{text}”: {} {}.",
            plural(hits.len(), "scene", "scenes"),
            if hits.len() == 1 {
                "mentions"
            } else {
                "mention"
            },
            if hits.len() == 1 { "Scene" } else { "Scenes" },
            join_and(&all)
        ))
    };
    let heading_only: BTreeSet<String> = in_heading.clone();
    if !heading_only.is_empty() {
        o = o.detail(format!(
            "In the scene heading (location): {}.",
            join_and(&nums(&heading_only))
        ));
    }
    let text_only: BTreeSet<String> = in_text.difference(&in_heading).cloned().collect();
    if !text_only.is_empty() {
        o = o.detail(format!(
            "Only in action/dialogue text: {}.",
            join_and(&nums(&text_only))
        ));
    }
    o.items = scene_items(&d, &hits);
    Ok(o.prov("Draft", d.label())
        .prov("Basis", "scene headings and screenplay text"))
}

fn location_statistics(ctx: &ToolCtx<'_>, draft: Option<&str>) -> AppResult<ToolOutput> {
    let d = ctx.draft(draft)?;
    let scenes = queries::scenes(ctx.conn, &d.id)?;
    let mut locs: BTreeMap<String, usize> = BTreeMap::new();
    let (mut int, mut ext, mut both, mut day, mut night) = (0, 0, 0, 0, 0);
    for s in &scenes {
        let h = queries::parse_heading(&s.heading);
        if !h.location.is_empty() {
            *locs.entry(h.location.clone()).or_default() += 1;
        }
        match h.int_ext.as_deref() {
            Some("INT.") => int += 1,
            Some("EXT.") => ext += 1,
            Some(_) => both += 1,
            None => {}
        }
        match h.time.as_deref() {
            Some(t) if t.contains("NIGHT") || t.contains("EVENING") || t.contains("DUSK") => {
                night += 1
            }
            Some(t)
                if t.contains("DAY")
                    || t.contains("MORNING")
                    || t.contains("DAWN")
                    || t.contains("AFTERNOON") =>
            {
                day += 1
            }
            _ => {}
        }
    }
    let records = queries::count(
        ctx.conn,
        "SELECT count(*) FROM location WHERE deleted_at IS NULL AND archived=0",
        [],
    )?;
    let mut sorted: Vec<(&String, &usize)> = locs.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    // The answer names the places, most used first; the per-location scene counts are its sources.
    let names: Vec<String> = sorted.iter().take(8).map(|(l, _)| title_case(l)).collect();
    let named = match locs.len() {
        0 => String::new(),
        n if n <= 8 => format!(": {}", join_and(&names)),
        _ => format!(". The most used are {}", join_and(&names[..5])),
    };
    let mut o = ToolOutput::exact(format!(
        "{} uses {}{named}.",
        d.label(),
        plural(locs.len(), "location", "locations"),
    ))
    .detail("Counted from the scene headings.")
    .detail(format!(
        "The Production Catalog currently contains {}.",
        plural(records as usize, "Location record", "Location records")
    ))
    .detail(format!(
        "Interior {int} · Exterior {ext}{} · Day {day} · Night {night}.",
        if both > 0 {
            format!(" · Int./Ext. {both}")
        } else {
            String::new()
        }
    ));
    o.items = sorted
        .iter()
        .map(|(l, n)| ResultItem {
            label: title_case(l),
            detail: Some(plural(**n, "scene", "scenes")),
            nav: None,
        })
        .collect();
    Ok(o.prov("Draft", d.label())
        .prov("Basis", "parsed screenplay scene headings"))
}

fn find_act(c: &Connection, act: &str) -> AppResult<(String, String, usize)> {
    let mut stmt =
        c.prepare("SELECT id, title FROM story_act WHERE deleted_at IS NULL ORDER BY position")?;
    let acts: Vec<(String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    if acts.is_empty() {
        return Err(AppError::ai(
            "not_found",
            "The Beat Board doesn't have any acts yet.",
        ));
    }
    let a = act.trim().to_lowercase();
    if let Some((i, (id, t))) = acts
        .iter()
        .enumerate()
        .find(|(_, (_, t))| t.to_lowercase() == a)
    {
        return Ok((id.clone(), t.clone(), i + 1));
    }
    let digits: String = a
        .trim_start_matches("act")
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    let roman = match a.trim_start_matches("act").trim() {
        "i" | "one" | "first" => Some(1),
        "ii" | "two" | "second" => Some(2),
        "iii" | "three" | "third" => Some(3),
        "iv" | "four" | "fourth" => Some(4),
        "v" | "five" | "fifth" => Some(5),
        _ => None,
    };
    if let Some(n) = digits.parse::<usize>().ok().or(roman)
        && let Some((id, t)) = acts.get(n.wrapping_sub(1))
    {
        return Ok((id.clone(), t.clone(), n));
    }
    let partial: Vec<(usize, &(String, String))> = acts
        .iter()
        .enumerate()
        .filter(|(_, (_, t))| t.to_lowercase().contains(&a))
        .collect();
    match partial.len() {
        1 => Ok((
            partial[0].1.0.clone(),
            partial[0].1.1.clone(),
            partial[0].0 + 1,
        )),
        0 => Err(AppError::ai(
            "not_found",
            format!("I couldn't find an act called “{}”.", act.trim()),
        )),
        _ => Err(queries::ambiguous(format!(
            "Which act do you mean: {}?",
            queries::join_or(
                &partial
                    .iter()
                    .map(|(_, (_, t))| t.clone())
                    .collect::<Vec<_>>()
            )
        ))),
    }
}

fn locations_in_act(ctx: &ToolCtx<'_>, act: &str) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    let (act_id, act_title, _) = find_act(c, act)?;
    let mut stmt = c.prepare(
        "SELECT sc.id, sc.scene_heading, sc.screenplay_scene_id FROM story_scene_card sc
         WHERE sc.deleted_at IS NULL AND ((sc.parent_type='act' AND sc.parent_id=?1)
            OR (sc.parent_type='sequence' AND sc.parent_id IN (SELECT id FROM story_sequence WHERE act_id=?1 AND deleted_at IS NULL)))
         ORDER BY sc.position",
    )?;
    let cards: Vec<(String, Option<String>, Option<String>)> = stmt
        .query_map([&act_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    let mut locs: BTreeMap<String, usize> = BTreeMap::new();
    let mut without = 0usize;
    for (_, heading, sp_scene) in &cards {
        let mut h = heading.clone().unwrap_or_default();
        if h.trim().is_empty()
            && let Some(sid) = sp_scene
        {
            h = c
                .query_row(
                    "SELECT heading FROM screenplay_scene WHERE id=?1 AND deleted_at IS NULL",
                    [sid],
                    |r| r.get(0),
                )
                .unwrap_or_default();
        }
        let parsed = queries::parse_heading(&h);
        if parsed.location.is_empty() {
            without += 1;
        } else {
            *locs.entry(parsed.location).or_default() += 1;
        }
    }
    let names: Vec<String> = locs.keys().map(|l| title_case(l)).collect();
    let mut o = if locs.is_empty() {
        ToolOutput::exact(format!(
            "None of the {} in {act_title} has a scene heading with a location yet.",
            plural(cards.len(), "Scene Card", "Scene Cards")
        ))
    } else {
        ToolOutput::exact(format!(
            "{act_title} uses {}: {}.",
            plural(locs.len(), "location", "locations"),
            join_and(&names)
        ))
    };
    if without > 0 && !locs.is_empty() {
        o = o.detail(format!(
            "{} in this act {} no location in its heading.",
            plural(without, "Scene Card", "Scene Cards"),
            if without == 1 { "has" } else { "have" }
        ));
    }
    o.items = locs
        .iter()
        .map(|(l, n)| ResultItem {
            label: title_case(l),
            detail: Some(plural(*n, "Scene Card", "Scene Cards")),
            nav: None,
        })
        .collect();
    Ok(o.prov("Beat Board", act_title).prov(
        "Basis",
        "Scene Card headings (or the linked screenplay scene heading)",
    ))
}

/// Scene lineages placed on a shooting day of the Production Source's schedule
/// (Schedule module contract: a strip with `day_id` NULL is in the Unscheduled pool;
/// archived strips have left the active schedule).
fn scheduled_lineages(c: &Connection, source_id: &str) -> AppResult<BTreeSet<String>> {
    let mut stmt = c.prepare(
        "SELECT DISTINCT st.scene_lineage_id FROM schedule_strip st
         JOIN shooting_day d ON d.id = st.day_id
         JOIN shooting_schedule sc ON sc.id = st.schedule_id
         WHERE st.deleted_at IS NULL AND st.archived = 0 AND d.deleted_at IS NULL AND d.is_off_day = 0
           AND sc.deleted_at IS NULL AND sc.source_id = ?1",
    )?;
    let rows = stmt
        .query_map([source_id], |r| r.get::<_, String>(0))?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

fn unscheduled_scenes(ctx: &ToolCtx<'_>) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    let Some((source_id, draft_id)) = queries::production_source_draft(c)? else {
        return Ok(ToolOutput::exact(
            "No Production Source is selected yet, so there are no scenes to schedule. Choose a Production Source draft in Production first.",
        )
        .prov("Scope", "Production"));
    };
    let draft = queries::draft_by_id(c, &draft_id)?.ok_or_else(|| AppError::not_found("draft"))?;
    let placed = scheduled_lineages(c, &source_id)?;
    let scenes = queries::scenes(c, &draft.id)?;
    let eligible: Vec<&SceneRow> = scenes.iter().filter(|s| !s.omitted).collect();
    let unscheduled: Vec<&SceneRow> = eligible
        .iter()
        .copied()
        .filter(|s| !placed.contains(&s.lineage_id))
        .collect();
    let mut o = ToolOutput::exact(format!(
        "{} of the {} in the Production Source ({}) {} not scheduled yet.",
        unscheduled.len(),
        plural(eligible.len(), "scene", "scenes"),
        draft.label(),
        if unscheduled.len() == 1 { "is" } else { "are" }
    ));
    o.items = scene_items(&draft, &unscheduled);
    o.structured = Some(
        json!({"draftId": draft.id, "eligible": eligible.len(), "unscheduled": unscheduled.len()}),
    );
    Ok(o.prov("Production Source", draft.label()).prov(
        "Basis",
        "eligible (not omitted) scenes minus scenes on a shooting day",
    ))
}

fn compare_drafts(ctx: &ToolCtx<'_>, from: &str, to: &str) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    let a = queries::resolve_draft(c, Some(from), None)?;
    let b = queries::resolve_draft(c, Some(to), None)?;
    if a.id == b.id {
        return Err(queries::ambiguous(
            "Those are the same draft. Which two drafts should I compare?",
        ));
    }
    let sa = queries::scenes(c, &a.id)?;
    let sb = queries::scenes(c, &b.id)?;
    let map_a: BTreeMap<&String, &SceneRow> = sa.iter().map(|s| (&s.lineage_id, s)).collect();
    let map_b: BTreeMap<&String, &SceneRow> = sb.iter().map(|s| (&s.lineage_id, s)).collect();
    let added: Vec<&SceneRow> = sb
        .iter()
        .filter(|s| !map_a.contains_key(&s.lineage_id))
        .collect();
    let removed: Vec<&SceneRow> = sa
        .iter()
        .filter(|s| !map_b.contains_key(&s.lineage_id))
        .collect();
    let mut changed = Vec::new();
    let mut moved = Vec::new();
    // Relative order of shared scenes decides "moved" (renumbering alone is not a move).
    let shared_a: Vec<&String> = sa
        .iter()
        .filter(|s| map_b.contains_key(&s.lineage_id))
        .map(|s| &s.lineage_id)
        .collect();
    let shared_b: Vec<&String> = sb
        .iter()
        .filter(|s| map_a.contains_key(&s.lineage_id))
        .map(|s| &s.lineage_id)
        .collect();
    for s in &sb {
        if let Some(old) = map_a.get(&s.lineage_id) {
            if queries::scene_fingerprint(c, &old.id)? != queries::scene_fingerprint(c, &s.id)? {
                changed.push(s);
            }
            let ia = shared_a.iter().position(|l| *l == &s.lineage_id);
            let ib = shared_b.iter().position(|l| *l == &s.lineage_id);
            if ia != ib {
                moved.push(s);
            }
        }
    }
    let nums =
        |v: &[&SceneRow]| join_and(&v.iter().map(|s| s.number.to_string()).collect::<Vec<_>>());
    let mut o = ToolOutput::exact(format!(
        "From {} to {}: {} added, {} removed, {} changed, {} moved.",
        a.label(),
        b.label(),
        added.len(),
        removed.len(),
        changed.len(),
        moved.len()
    ));
    if !added.is_empty() {
        o = o.detail(format!("Added in {}: {}.", b.label(), nums(&added)));
    }
    if !removed.is_empty() {
        o = o.detail(format!(
            "Removed (numbers in {}): {}.",
            a.label(),
            nums(&removed)
        ));
    }
    if !changed.is_empty() {
        o = o.detail(format!("Changed: {}.", nums(&changed)));
    }
    if !moved.is_empty() {
        o = o.detail(format!("Moved: {}.", nums(&moved)));
    }
    let mut list: Vec<&SceneRow> = added.iter().chain(changed.iter()).copied().collect();
    list.sort_by_key(|s| s.number);
    list.dedup_by_key(|s| s.id.clone());
    o.items = scene_items(&b, &list);
    Ok(o.prov("Drafts", format!("{} → {}", a.label(), b.label()))
        .prov("Basis", "OpenFrame draft comparison by scene identity"))
}

fn breakdown_status(ctx: &ToolCtx<'_>) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    let Some((source_id, draft_id)) = queries::production_source_draft(c)? else {
        return Ok(ToolOutput::exact(
            "No Production Source is selected yet, so there is no breakdown to report.",
        )
        .prov("Scope", "Production"));
    };
    let draft = queries::draft_by_id(c, &draft_id)?.ok_or_else(|| AppError::not_found("draft"))?;
    let confirmed = queries::count(
        c,
        "SELECT count(*) FROM breakdown_element WHERE source_id=?1 AND deleted_at IS NULL AND confirmation_state IN ('Confirmed','Manual')",
        [&source_id],
    )?;
    let suggested = queries::count(
        c,
        "SELECT count(*) FROM breakdown_element WHERE source_id=?1 AND deleted_at IS NULL AND confirmation_state='Suggested'",
        [&source_id],
    )?;
    let scenes = queries::scenes(c, &draft.id)?;
    let mut stmt = c.prepare(
        "SELECT DISTINCT scene_id FROM breakdown_element WHERE source_id=?1 AND deleted_at IS NULL AND confirmation_state IN ('Confirmed','Manual')",
    )?;
    let with: BTreeSet<String> = stmt
        .query_map([&source_id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let without: Vec<&SceneRow> = scenes
        .iter()
        .filter(|s| !s.omitted && !with.contains(&s.id))
        .collect();
    let mut o = ToolOutput::exact(format!(
        "The breakdown for {} has {} and {} awaiting review.",
        draft.label(),
        plural(
            confirmed as usize,
            "confirmed element",
            "confirmed elements"
        ),
        plural(
            suggested as usize,
            "suggested element",
            "suggested elements"
        )
    ))
    .detail(format!(
        "{} no confirmed breakdown elements yet.",
        plural(without.len(), "scene has", "scenes have")
    ));
    o.items = scene_items(&draft, &without);
    Ok(o.prov("Production Source", draft.label())
        .prov("Basis", "breakdown confirmation states"))
}

fn story_statistics(ctx: &ToolCtx<'_>) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    let acts = queries::count(
        c,
        "SELECT count(*) FROM story_act WHERE deleted_at IS NULL",
        [],
    )?;
    let seqs = queries::count(
        c,
        "SELECT count(*) FROM story_sequence WHERE deleted_at IS NULL",
        [],
    )?;
    let beats = queries::count(
        c,
        "SELECT count(*) FROM story_beat WHERE deleted_at IS NULL AND state='active'",
        [],
    )?;
    let cards = queries::count(
        c,
        "SELECT count(*) FROM story_scene_card WHERE deleted_at IS NULL",
        [],
    )?;
    let parked = queries::count(
        c,
        "SELECT count(*) FROM story_scene_card WHERE deleted_at IS NULL AND parent_type='parking'",
        [],
    )?;
    let chars = queries::character_records(c)?.len();
    let o = ToolOutput::exact(format!(
        "The Beat Board has {}, {}, {} and {}.",
        plural(acts as usize, "act", "acts"),
        plural(seqs as usize, "sequence", "sequences"),
        plural(beats as usize, "beat", "beats"),
        plural(cards as usize, "Scene Card", "Scene Cards")
    ))
    .detail(format!(
        "{} in the Parking Lot.",
        plural(parked as usize, "Scene Card is", "Scene Cards are")
    ))
    .detail(format!(
        "The Character directory contains {}.",
        plural(chars, "Character record", "Character records")
    ));
    Ok(o.prov("Scope", "Beat Board"))
}

fn production_statistics(ctx: &ToolCtx<'_>) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    let mut stmt = c.prepare(
        "SELECT category, count(*) FROM catalog_item WHERE deleted_at IS NULL AND archived=0 GROUP BY category ORDER BY count(*) DESC, category",
    )?;
    let cats: Vec<(String, i64)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let total: i64 = cats.iter().map(|(_, n)| n).sum();
    let locs = queries::count(
        c,
        "SELECT count(*) FROM location WHERE deleted_at IS NULL AND archived=0",
        [],
    )?;
    let cast = queries::count(
        c,
        "SELECT count(*) FROM cast_member WHERE deleted_at IS NULL AND archived=0",
        [],
    )?;
    let crew = queries::count(
        c,
        "SELECT count(*) FROM crew_member WHERE deleted_at IS NULL AND archived=0",
        [],
    )?;
    let mut o = ToolOutput::exact(format!(
        "The Production Catalog contains {}. There are {}, {} and {}.",
        plural(total as usize, "item", "items"),
        plural(locs as usize, "Location record", "Location records"),
        plural(cast as usize, "cast member", "cast members"),
        plural(crew as usize, "crew member", "crew members")
    ));
    o.items = cats
        .iter()
        .map(|(cat, n)| ResultItem {
            label: cat.clone(),
            detail: Some(n.to_string()),
            nav: None,
        })
        .collect();
    if let Some((_, d)) = queries::production_source_draft(c)?
        && let Some(draft) = queries::draft_by_id(c, &d)?
    {
        o = o.detail(format!("The Production Source is {}.", draft.label()));
    }
    Ok(o.prov("Scope", "Production"))
}

fn fts_query(text: &str) -> Option<String> {
    let terms: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.chars().count() >= 2)
        .take(8)
        .map(|t| format!("\"{}\"*", t.replace('"', "")))
        .collect();
    (!terms.is_empty()).then(|| terms.join(" AND "))
}

/// Search the project index with the Global Search privacy rule (own private notes only).
pub fn search_hits(
    c: &Connection,
    actor: &Actor,
    text: &str,
    limit: usize,
) -> AppResult<Vec<(String, String, String, Value)>> {
    let Some(q) = fts_query(text) else {
        return Ok(Vec::new());
    };
    let mut stmt = c.prepare(
        "SELECT d.entity_type, d.title, d.context, d.nav_json FROM search_fts f JOIN search_doc d ON d.rowid=f.rowid
         WHERE search_fts MATCH ?1 AND (d.owner_user_id IS NULL OR d.owner_user_id=?2)
         ORDER BY bm25(search_fts) LIMIT ?3",
    )?;
    let rows = stmt
        .query_map(params![q, actor.user_id, limit as i64], |r| {
            let nav: String = r.get(3)?;
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                serde_json::from_str(&nav).unwrap_or(Value::Null),
            ))
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

pub(crate) fn nav_from_json(v: &Value) -> Option<NavTarget> {
    let workspace = v.get("workspace")?.as_str()?.to_string();
    let sub = v.get("sub").and_then(|s| s.as_str()).map(|s| s.to_string());
    let mut params = BTreeMap::new();
    if let Some(obj) = v.as_object() {
        for (k, val) in obj {
            if k != "workspace"
                && k != "sub"
                && let Some(s) = val.as_str()
            {
                params.insert(k.clone(), s.to_string());
            }
        }
    }
    Some(NavTarget {
        workspace,
        sub,
        params,
    })
}

/// Search hits named in the answer (and offered as sources) when there are this few.
const NAMED_SEARCH_HITS: usize = 3;

/// The source kind shown for a search hit.
fn search_kind(entity_type: &str) -> &'static str {
    match entity_type {
        "screenplay_scene" => "Scene",
        "story_scene_card" => "Scene Card",
        "story_character" => "Character",
        "vault_item" => "Idea",
        "project_note" | "private_note" => "Note",
        "task" => "Task",
        "project_file" => "File",
        "location" => "Location",
        "catalog_item" => "Catalog item",
        "cast_member" | "crew_member" => "Person",
        "shooting_day" => "Shooting Day",
        "call_sheet" => "Call Sheet",
        "comment" => "Comment",
        _ => "Result",
    }
}

/// What a search for a bare category word really asks. "scenes", "characters" or "locations" are
/// never written in scene text, so a text search finds nothing and the reply used to be "Nothing in
/// this project mentions scenes" for a 130-scene screenplay (OpenFrameAI.docx, exhibits E01 and E06).
fn category_query(text: &str) -> Option<&'static str> {
    let words: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| w.to_lowercase())
        .filter(|w| {
            !matches!(
                w.as_str(),
                "all" | "the" | "of" | "in" | "my" | "this" | "total" | "number" | "count"
            )
        })
        .collect();
    if words.is_empty() || words.len() > 2 {
        return None;
    }
    let one = |set: &[&str]| words.iter().all(|w| set.contains(&w.as_str()));
    if one(&["scene", "scenes", "screenplay", "script"])
        && words.iter().any(|w| w.starts_with("scene"))
    {
        Some("scenes")
    } else if one(&["character", "characters", "cast", "speaking", "people"]) {
        Some("characters")
    } else if one(&[
        "location",
        "locations",
        "place",
        "places",
        "setting",
        "settings",
        "set",
        "sets",
    ]) {
        Some("locations")
    } else {
        None
    }
}

fn search_project(ctx: &ToolCtx<'_>, text: &str) -> AppResult<ToolOutput> {
    let by_category = match category_query(text) {
        Some("scenes") => count_scenes(ctx, None).ok(),
        Some("characters") => count_characters(ctx, None).ok(),
        Some("locations") => location_statistics(ctx, None).ok(),
        _ => None,
    };
    if let Some(o) = by_category {
        return Ok(o);
    }
    let hits = search_hits(ctx.conn, ctx.actor, text, 50)?;
    if hits.is_empty() {
        return Ok(ToolOutput::exact(format!(
            "Nothing in this project mentions “{}”.",
            text.trim()
        ))
        .prov("Scope", "Whole Project"));
    }
    let mut groups: BTreeMap<String, usize> = BTreeMap::new();
    for (_, _, context, _) in &hits {
        *groups
            .entry(if context.is_empty() {
                "Other".into()
            } else {
                context.clone()
            })
            .or_default() += 1;
    }
    // A few hits are named, so "Which scene...?" is answered by the scene itself; many are
    // counted per place.
    let summary: Vec<String> = if hits.len() <= NAMED_SEARCH_HITS {
        hits.iter()
            .map(|(etype, title, context, _)| {
                let name = if title.is_empty() { etype } else { title };
                if context.is_empty() {
                    name.clone()
                } else {
                    format!("{name} ({context})")
                }
            })
            .collect()
    } else {
        groups.iter().map(|(g, n)| format!("{n} {g}")).collect()
    };
    let mut o = ToolOutput::exact(format!(
        "{}{}: {}.",
        plural(hits.len(), "result", "results"),
        if hits.len() == 50 {
            " (showing the first 50)"
        } else {
            ""
        },
        summary.join(if hits.len() <= NAMED_SEARCH_HITS {
            "; "
        } else {
            " · "
        })
    ));
    // The named hits are sources the person can open.
    for (etype, title, _, nav) in hits.iter().take(NAMED_SEARCH_HITS) {
        if let Some(target) = nav_from_json(nav)
            && !title.is_empty()
        {
            o.provenance.push(Provenance::linked(
                search_kind(etype),
                title.clone(),
                target,
            ));
        }
    }
    o.items = hits
        .iter()
        .map(|(etype, title, context, nav)| ResultItem {
            label: if title.is_empty() {
                etype.clone()
            } else {
                title.clone()
            },
            detail: Some(context.clone()),
            nav: nav_from_json(nav),
        })
        .collect();
    o.nav = Some(NavTarget::to("home").with("search", text.trim()));
    Ok(o.prov("Scope", "Whole Project")
        .prov("Basis", "project search index"))
}

fn open_scene(ctx: &ToolCtx<'_>, number: u32, draft: Option<&str>) -> AppResult<ToolOutput> {
    let d = ctx.draft(draft)?;
    let scenes = queries::scenes(ctx.conn, &d.id)?;
    let s = scenes
        .iter()
        .find(|s| s.number == number as usize)
        .ok_or_else(|| {
            AppError::ai(
                "not_found",
                format!(
                    "{} has {}; there is no Scene {number}.",
                    d.label(),
                    plural(scenes.len(), "scene", "scenes")
                ),
            )
        })?;
    let mut o = ToolOutput::of_kind(
        AiResultKind::Navigate,
        format!("Opening {} in the Screenplay.", s.label()),
    );
    o.details
        .push("Navigation commands run directly; nothing is changed.".into());
    o.nav = Some(scene_nav(&d, s));
    o.confidence = Some(Confidence::Exact);
    o.targets.push(ObjRef {
        table: "screenplay_scene".into(),
        id: s.id.clone(),
        label: s.label(),
    });
    Ok(o.prov("Draft", d.label()))
}

fn open_workspace(ws: &str) -> AppResult<ToolOutput> {
    let (id, label) = WORKSPACES
        .iter()
        .find(|(id, label)| *id == ws || label.eq_ignore_ascii_case(ws))
        .ok_or_else(|| AppError::ai("not_found", "I don't know that part of OpenFrame."))?;
    let mut o = ToolOutput::of_kind(AiResultKind::Navigate, format!("Opening {label}."));
    o.details
        .push("Navigation commands run directly; nothing is changed.".into());
    o.nav = Some(NavTarget::to(id));
    Ok(o)
}

fn open_object(ctx: &ToolCtx<'_>, text: &str) -> AppResult<ToolOutput> {
    let hits = search_hits(ctx.conn, ctx.actor, text, 5)?;
    let Some((etype, title, context, nav)) = hits.first() else {
        return Ok(ToolOutput::exact(format!(
            "I couldn't find anything called “{}” in this project.",
            text.trim()
        )));
    };
    let target = nav_from_json(nav).ok_or_else(|| {
        AppError::ai("not_found", "I found it, but it can't be opened from here.")
    })?;
    let label = if title.is_empty() {
        etype.clone()
    } else {
        title.clone()
    };
    let mut o = ToolOutput::of_kind(
        AiResultKind::Navigate,
        format!("Opening {label} ({context})."),
    );
    o.details
        .push("Navigation commands run directly; nothing is changed.".into());
    if hits.len() > 1 {
        o.details.push(format!(
            "{} other matches are listed below.",
            hits.len() - 1
        ));
        o.items = hits[1..]
            .iter()
            .map(|(e, t, cx, n)| ResultItem {
                label: if t.is_empty() { e.clone() } else { t.clone() },
                detail: Some(cx.clone()),
                nav: nav_from_json(n),
            })
            .collect();
    }
    o.nav = Some(target);
    Ok(o)
}

fn suggest_breakdown(ctx: &ToolCtx<'_>, number: Option<u32>) -> AppResult<ToolOutput> {
    let (draft, scene) = match (number, &ctx.scope.scene, &ctx.scope.draft) {
        (None, Some(s), Some(d)) => (d.clone(), s.clone()),
        (Some(n), _, _) => {
            let d = ctx.draft(None)?;
            let s = queries::scenes(ctx.conn, &d.id)?
                .into_iter()
                .find(|s| s.number == n as usize)
                .ok_or_else(|| {
                    AppError::ai("not_found", format!("{} has no Scene {n}.", d.label()))
                })?;
            (d, s)
        }
        _ => {
            return Err(queries::ambiguous(
                "Which scene should I break down? Open it or tell me its number.",
            ));
        }
    };
    let text = queries::scene_text(ctx.conn, &scene.id, 3_000)?;
    let mut o = ToolOutput::of_kind(AiResultKind::Suggestion, String::new());
    o.confidence = Some(Confidence::Inferred);
    o.provenance.push(Provenance::new("Draft", draft.label()));
    o.provenance.push(Provenance::new("Scene", scene.label()));
    o.targets.push(ObjRef {
        table: "screenplay_scene".into(),
        id: scene.id.clone(),
        label: scene.label(),
    });
    o.generation = Some(Generation::Breakdown {
        scene: Box::new(scene),
        draft,
        text,
    });
    Ok(o)
}

fn private_information(ctx: &ToolCtx<'_>, whose: &str) -> AppResult<ToolOutput> {
    if whose != "mine" {
        // Never reveal existence, count or content of another user's private notes (Security §8.5).
        return Ok(ToolOutput::of_kind(
            AiResultKind::Private,
            "The requested information is private and cannot be accessed in this context.",
        ));
    }
    let mut stmt = ctx.conn.prepare(
        "SELECT body FROM private_note WHERE owner_user_id=?1 AND deleted_at IS NULL ORDER BY updated_at DESC LIMIT 20",
    )?;
    let notes: Vec<String> = stmt
        .query_map([&ctx.actor.user_id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let mut o = ToolOutput::exact(format!(
        "You have {} in this project.",
        plural(notes.len(), "private note", "private notes")
    ));
    o.items = notes
        .iter()
        .map(|b| ResultItem {
            label: queries::truncate_chars(b.lines().next().unwrap_or(""), 120),
            detail: None,
            nav: None,
        })
        .collect();
    Ok(o.prov("Scope", "Your private notes (visible only to you)"))
}
