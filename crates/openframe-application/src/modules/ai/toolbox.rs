//! Toolbox (contract C3): the only surface the agent loop uses to discover and
//! run tools (agentic spec §6, §26–§28, §39–§40).
//!
//! ```text
//! tools_for(actor, scope) ─► permission-filtered ToolDefs (never a Change Set review action)
//! run_tool(name, args)
//!   ├─ unknown name ─────────────────► ai.unknown_tool
//!   ├─ strict schema + size limits ──► ai.tool_arguments   (additionalProperties = false)
//!   ├─ permission as the real actor ─► permission.denied   (the offered list is not the boundary)
//!   ├─ read / compute / navigate ────► ToolStep::Output     (bounded, canonical data, own private notes only)
//!   └─ proposal tool ────────────────► ToolStep::Proposal   (a ChangeSetDraft; NOTHING is dispatched)
//! ```
//!
//! Every proposal is validated after it is built: each operation must be a
//! registered command whose explicit metadata exposes it to the assistant
//! (`registry::AiExposure::Tool`), listed by the proposal tool, and never an
//! `ai.*` operation. Operations whose product UI asks for an extra destructive
//! confirmation add a "destructive" preview row so the review card keeps asking
//! for it after the Change Set is approved (spec §7).
//!
//! Locking: `run_tool` receives a `ToolCtx` whose connection is the store's read
//! connection, so every tool here reads through `ctx.conn` and never re-enters
//! `Store::read`. `run_tool_unlocked` takes the lock itself and is the entry
//! point that can also reach the hybrid retrieval service (contract C2).

pub mod reads;
mod reads_production;
pub mod resolve;
pub mod schema;

use std::collections::BTreeSet;

use openframe_domain::{Actor, AppError, AppResult, Capability};
use serde_json::{Value, json};

use super::catalog::{self, PropCtx, ProposalSpec};
use super::queries;
use super::scope::ResolvedScope;
use super::tools::{self, ToolCtx, ToolOutput, ToolSpec};
use super::types::*;
use crate::core::AppCore;
use crate::registry::{AiExposure, OpKind, Registry};

/// A tool offered to the model.
#[derive(Debug, Clone)]
pub struct ToolDef {
    pub name: String,
    pub class: OperationClass,
    pub description: String,
    /// Strict JSON schema of the arguments (`additionalProperties: false`, bounded).
    pub schema: Value,
    /// The loop ends after this tool (navigation, clarification, final generation).
    pub terminal: bool,
    /// The tool prepares a Change Set proposal (never applies it).
    pub mutating: bool,
}

/// What one tool call produced.
#[derive(Debug, Clone)]
pub enum ToolStep {
    Output(ToolOutput),
    Proposal(ChangeSetDraft),
}

/// The retrieval tool (contract C2 consumer).
pub const RETRIEVE_TOOL: &str = "retrieve_context";

/// Tools that finish the request: a focused question, private-note handling and
/// model-written text over bounded context. Navigation tools are terminal too.
const TERMINAL_READS: &[&str] = &[
    "clarify",
    "private_information",
    "answer_product_question",
    "summarize_scope",
    "suggest_breakdown",
];

/// Registry queries whose facts the original deterministic tools expose.
const LEGACY_COVERS: &[(&str, &[&str])] = &[
    ("project_overview", &["project.home"]),
    ("list_drafts", &["screenplay.overview"]),
    (
        "compare_drafts",
        &["screenplay.compare", "screenplay.revision_changes"],
    ),
    ("breakdown_status", &["breakdown.scenes"]),
    ("production_statistics", &["production.overview"]),
    ("search_project", &["search.query"]),
    ("open_scene", &["screenplay.locate"]),
    ("private_information", &["private_note.list"]),
];

/// Upper bounds for one tool result (spec §5: bound tool output sizes).
const MAX_CONTENT_CHARS: usize = 4_000;
const MAX_DETAILS: usize = 24;
const MAX_DETAIL_CHARS: usize = 600;
const MAX_ITEMS: usize = 80;
const MAX_ITEM_CHARS: usize = 300;
const MAX_STRUCTURED_BYTES: usize = 16 * 1024;
/// Upper bound for the operations of one proposal.
pub const MAX_PROPOSAL_OPS: usize = 200;

#[derive(Clone, Copy)]
enum Tool {
    Legacy(&'static ToolSpec),
    Read(&'static reads::ReadSpec),
    Proposal(&'static ProposalSpec),
    Retrieve,
}

fn lookup(name: &str) -> Option<Tool> {
    if name == RETRIEVE_TOOL {
        return Some(Tool::Retrieve);
    }
    if let Some(p) = catalog::spec(name) {
        return Some(Tool::Proposal(p));
    }
    if let Some(r) = reads::spec(name) {
        return Some(Tool::Read(r));
    }
    tools::spec(name).map(Tool::Legacy)
}

fn unknown(name: &str) -> AppError {
    AppError::ai(
        "unknown_tool",
        "That isn't something the assistant can do yet.",
    )
    .with_detail(format!("tool {}", queries::truncate_chars(name, 64)))
}

fn retrieve_schema() -> Value {
    schema::obj(
        &[
            (
                "query",
                schema::sd(500, "what to look for, in the user's words"),
            ),
            ("limit", schema::int(1, 24)),
        ],
        &["query"],
    )
}

const RETRIEVE_DESCRIPTION: &str = "Find the project passages most relevant to a question (scenes, cards, characters, notes, production items) before answering.";

/// Every tool name the toolbox knows (read, compute, navigate, suggest, proposal).
pub fn all_tool_names() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = tools::TOOLS.iter().map(|t| t.name).collect();
    v.extend(reads::READS.iter().map(|r| r.name));
    v.push(RETRIEVE_TOOL);
    v.extend(catalog::PROPOSALS.iter().map(|p| p.tool));
    v
}

/// (tool, mutating, registry operations it reads or proposes) for the coverage matrix.
pub fn coverage() -> Vec<(&'static str, bool, Vec<&'static str>)> {
    let mut out = Vec::new();
    for t in tools::TOOLS {
        let ops = LEGACY_COVERS
            .iter()
            .find(|(n, _)| *n == t.name)
            .map(|(_, o)| o.to_vec())
            .unwrap_or_default();
        out.push((t.name, false, ops));
    }
    for r in reads::READS {
        out.push((r.name, false, r.covers.to_vec()));
    }
    out.push((RETRIEVE_TOOL, false, vec!["search.query"]));
    for p in catalog::PROPOSALS.iter() {
        let mut ops = p.required_ops.to_vec();
        ops.extend(p.optional_ops.iter().copied());
        out.push((p.tool, true, ops));
    }
    out
}

/// A command the assistant may place in a Change Set: registered, a command,
/// explicitly exposed by its metadata, and never an assistant operation.
pub fn op_exposed(reg: &Registry, op: &str) -> bool {
    !op.starts_with("ai.")
        && reg.get(op).is_some_and(|e| {
            e.kind == OpKind::Command && e.meta.is_some_and(|m| m.ai_exposure == AiExposure::Tool)
        })
}

fn proposal_available(reg: &Registry, p: &ProposalSpec) -> bool {
    p.required_ops.iter().all(|op| op_exposed(reg, op))
}

fn proposal_allowed(actor: &Actor, p: &ProposalSpec) -> bool {
    actor.can(p.cap) && actor.can(Capability::ApplyChangeSet)
}

fn def_legacy(t: &ToolSpec) -> ToolDef {
    ToolDef {
        name: t.name.to_string(),
        class: t.class,
        description: t.description.to_string(),
        schema: (t.schema)(),
        terminal: t.class == OperationClass::Navigate || TERMINAL_READS.contains(&t.name),
        mutating: false,
    }
}

fn def_read(r: &reads::ReadSpec) -> ToolDef {
    ToolDef {
        name: r.name.to_string(),
        class: r.class,
        description: r.description.to_string(),
        schema: (r.schema)(),
        terminal: r.class == OperationClass::Navigate,
        mutating: false,
    }
}

fn def_proposal(p: &ProposalSpec) -> ToolDef {
    ToolDef {
        name: p.tool.to_string(),
        class: OperationClass::Mutate,
        description: format!(
            "{} (prepares a proposal for the user to review)",
            p.description
        ),
        schema: (p.schema)(),
        terminal: false,
        mutating: true,
    }
}

fn def_retrieve() -> ToolDef {
    ToolDef {
        name: RETRIEVE_TOOL.into(),
        class: OperationClass::Search,
        description: RETRIEVE_DESCRIPTION.into(),
        schema: retrieve_schema(),
        terminal: false,
        mutating: false,
    }
}

/// The tools this actor may use in this scope: reads need View, proposals need
/// the proposal's capability plus ApplyChangeSet and every operation exposed.
/// Never contains a Change Set review action (accept / apply / reject / recheck).
pub fn tools_for(core: &AppCore, actor: &Actor, _scope: &ResolvedScope) -> Vec<ToolDef> {
    let mut out = Vec::new();
    if actor.can(Capability::View) {
        out.extend(tools::TOOLS.iter().map(def_legacy));
        out.push(def_retrieve());
        out.extend(reads::READS.iter().map(def_read));
    }
    for p in catalog::PROPOSALS.iter() {
        if proposal_available(&core.registry, p) && proposal_allowed(actor, p) {
            out.push(def_proposal(p));
        }
    }
    out
}

/// Words that signal the user wants something changed.
const MUTATION_WORDS: &[&str] = &[
    "add",
    "create",
    "new",
    "make",
    "rename",
    "change",
    "update",
    "edit",
    "set",
    "move",
    "schedule",
    "assign",
    "delete",
    "remove",
    "archive",
    "restore",
    "link",
    "unlink",
    "tag",
    "pin",
    "lock",
    "reorder",
    "replace",
    "write",
    "put",
    "mark",
    "prepare",
    "organize",
    "organise",
    "copy",
    "duplicate",
    "send",
    "park",
    "resolve",
    "reopen",
    "reply",
    "comment",
    "save",
    "finalize",
    "issue",
    "attach",
    "shift",
    "swap",
    "convert",
    "start",
    "complete",
    "confirm",
    "reject",
    "accept",
    "clear",
    "split",
    "unschedule",
    "note",
    "task",
];

const STOP_WORDS: &[&str] = &[
    "the", "a", "an", "of", "to", "in", "on", "for", "and", "or", "is", "are", "what", "which",
    "who", "how", "many", "much", "me", "my", "i", "you", "it", "this", "that", "with", "all",
    "please", "can", "could", "would", "do", "does", "there", "be", "at", "by", "from", "about",
];

fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .map(|w| w.to_lowercase())
        .filter(|w| w.len() >= 2 && !STOP_WORDS.contains(&w.as_str()))
        .map(|w| {
            if w.len() > 4 && w.ends_with('s') {
                w[..w.len() - 1].to_string()
            } else {
                w
            }
        })
        .collect()
}

/// Always offered (small, general): retrieval, search, navigation, clarification.
const CORE_TOOLS: &[&str] = &[
    RETRIEVE_TOOL,
    "search_project",
    "project_overview",
    "open_object",
    "open_workspace",
    "open_scene",
    "clarify",
    "answer_product_question",
    "private_information",
];

/// A deterministic, request-relevant subset of `tools_for` that fits a small
/// model's prompt: the core tools plus the domain tools whose names and
/// descriptions share the most words with the request (proposal tools only when
/// the request asks for a change). `max_chars` bounds the rendered catalogue.
pub fn tools_for_request(
    core: &AppCore,
    actor: &Actor,
    scope: &ResolvedScope,
    request: &str,
    max_chars: usize,
) -> Vec<ToolDef> {
    let all = tools_for(core, actor, scope);
    let want: Vec<String> = words(request);
    let wants_change = want.iter().any(|w| MUTATION_WORDS.contains(&w.as_str()));
    let scope_words = words(scope.kind.label());
    let mut scored: Vec<(i64, usize, &ToolDef)> = all
        .iter()
        .enumerate()
        .filter(|(_, t)| !CORE_TOOLS.contains(&t.name.as_str()))
        .filter(|(_, t)| wants_change || !t.mutating)
        .map(|(i, t)| {
            let name_words = words(&t.name.replace('_', " "));
            let desc_words = words(&t.description);
            let mut score = 0i64;
            for w in &want {
                if name_words.contains(w) {
                    score += 3;
                } else if desc_words.contains(w) {
                    score += 1;
                }
            }
            for w in &scope_words {
                if name_words.contains(w) || desc_words.contains(w) {
                    score += 1;
                }
            }
            (score, i, t)
        })
        .filter(|(s, _, _)| *s > 0)
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut out: Vec<ToolDef> = all
        .iter()
        .filter(|t| CORE_TOOLS.contains(&t.name.as_str()))
        .cloned()
        .collect();
    let mut used: usize = out.iter().map(rendered_len).sum();
    for (_, _, t) in scored {
        let len = rendered_len(t);
        if used + len > max_chars {
            continue;
        }
        used += len;
        out.push(t.clone());
    }
    out
}

fn rendered_len(t: &ToolDef) -> usize {
    t.name.len() + t.description.len() + t.schema.to_string().len() + 8
}

fn clip_output(mut o: ToolOutput) -> ToolOutput {
    o.content = queries::truncate_chars(&o.content, MAX_CONTENT_CHARS);
    o.details.truncate(MAX_DETAILS);
    for d in &mut o.details {
        *d = queries::truncate_chars(d, MAX_DETAIL_CHARS);
    }
    if o.items.len() > MAX_ITEMS {
        let more = o.items.len() - MAX_ITEMS;
        o.items.truncate(MAX_ITEMS);
        o.details.push(format!(
            "{more} more not shown; ask more specifically to narrow it down."
        ));
    }
    for it in &mut o.items {
        it.label = queries::truncate_chars(&it.label, MAX_ITEM_CHARS);
        if let Some(d) = it.detail.as_mut() {
            *d = queries::truncate_chars(d, MAX_ITEM_CHARS);
        }
    }
    if o.structured
        .as_ref()
        .is_some_and(|s| s.to_string().len() > MAX_STRUCTURED_BYTES)
    {
        o.structured = Some(json!({"truncated": true}));
    }
    o
}

fn denied(actor: &Actor, p: &ProposalSpec) -> AppError {
    AppError::new("permission.denied", catalog::denial_message(actor, p))
}

/// Run one tool as the requesting user. Reads run against canonical data through
/// `ctx.conn`; proposal tools only build a Change Set draft.
pub fn run_tool(
    core: &AppCore,
    ctx: &ToolCtx<'_>,
    name: &str,
    args: &Value,
) -> AppResult<ToolStep> {
    let tool = lookup(name).ok_or_else(|| unknown(name))?;
    let args = normalize(args)?;
    match tool {
        Tool::Proposal(p) => rebuild_proposal(core, ctx, p.tool, &args).map(ToolStep::Proposal),
        Tool::Legacy(t) => {
            schema::validate(t.name, &(t.schema)(), &args)?;
            ctx.actor.require(Capability::View, "view this project")?;
            tools::execute(ctx, name, &args).map(|o| ToolStep::Output(clip_output(o)))
        }
        Tool::Read(r) => {
            schema::validate(r.name, &(r.schema)(), &args)?;
            ctx.actor.require(Capability::View, "view this project")?;
            (r.run)(ctx, &args).map(|o| ToolStep::Output(clip_output(o)))
        }
        Tool::Retrieve => {
            schema::validate(RETRIEVE_TOOL, &retrieve_schema(), &args)?;
            ctx.actor.require(Capability::View, "view this project")?;
            reads::retrieve_lexical(ctx, &args).map(|o| ToolStep::Output(clip_output(o)))
        }
    }
}

/// Like `run_tool`, but takes the store's read lock itself. Retrieval runs outside
/// the lock so it can use the hybrid retrieval service (contract C2) when present.
pub fn run_tool_unlocked(
    core: &AppCore,
    actor: &Actor,
    scope: &ResolvedScope,
    request_text: &str,
    name: &str,
    args: &Value,
) -> AppResult<ToolStep> {
    if name == RETRIEVE_TOOL {
        let args = normalize(args)?;
        schema::validate(RETRIEVE_TOOL, &retrieve_schema(), &args)?;
        actor.require(Capability::View, "view this project")?;
        if let Some(out) = reads::retrieve_hybrid(core, actor, scope, &args)? {
            return Ok(ToolStep::Output(clip_output(out)));
        }
    }
    let s = core.project()?;
    s.store.read(|c| {
        let ctx = ToolCtx {
            conn: c,
            actor,
            scope,
            request_text,
        };
        run_tool(core, &ctx, name, args)
    })
}

/// `null` → `{}`; anything else must be an object.
fn normalize(args: &Value) -> AppResult<Value> {
    match args {
        Value::Null => Ok(json!({})),
        Value::Object(_) => Ok(args.clone()),
        _ => Err(AppError::ai(
            "tool_arguments",
            "I couldn't understand the details of that request.",
        )
        .with_detail("arguments must be an object")),
    }
}

/// Build (or rebuild, for "Re-check & Review") a proposal from validated
/// arguments, as the requesting user. Never applies anything.
pub fn rebuild_proposal(
    core: &AppCore,
    ctx: &ToolCtx<'_>,
    name: &str,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let p = catalog::spec(name).ok_or_else(|| unknown(name))?;
    let args = normalize(args)?;
    schema::validate(p.tool, &(p.schema)(), &args)?;
    if !proposal_available(&core.registry, p) {
        return Err(AppError::ai(
            "unsupported",
            "I can't prepare that change in this version of OpenFrame. Nothing was changed.",
        ));
    }
    if !proposal_allowed(ctx.actor, p) {
        return Err(denied(ctx.actor, p));
    }
    let pctx = PropCtx {
        conn: ctx.conn,
        actor: ctx.actor,
        registry: &core.registry,
        scope: Some(ctx.scope),
    };
    let draft = (p.build)(&pctx, p, &args)?;
    check_proposal(&core.registry, p, draft)
}

/// Defence in depth on every built proposal (spec §40): only exposed, listed,
/// registered commands; bounded; destructive confirmation carried into the preview.
fn check_proposal(
    reg: &Registry,
    p: &ProposalSpec,
    mut d: ChangeSetDraft,
) -> AppResult<ChangeSetDraft> {
    let refuse = |why: String| {
        AppError::ai(
            "unsupported",
            "I can't prepare that change in this version of OpenFrame. Nothing was changed.",
        )
        .with_detail(why)
    };
    if d.operations.is_empty() {
        return Err(refuse(format!("{}: empty proposal", p.tool)));
    }
    if d.operations.len() > MAX_PROPOSAL_OPS {
        return Err(AppError::ai(
            "too_large",
            format!(
                "That change touches more than {MAX_PROPOSAL_OPS} items. Please narrow it down. Nothing was changed."
            ),
        ));
    }
    let listed: BTreeSet<&str> = p
        .required_ops
        .iter()
        .chain(p.optional_ops.iter())
        .copied()
        .collect();
    let mut confirm = Vec::new();
    for op in &d.operations {
        if !listed.contains(op.op.as_str()) || !op_exposed(reg, &op.op) {
            return Err(refuse(format!(
                "{}: operation {} not allowed",
                p.tool, op.op
            )));
        }
        if !op.args.is_object() || op.args.to_string().len() > 64 * 1024 {
            return Err(refuse(format!("{}: operation arguments", p.tool)));
        }
        if let Some(m) = reg.metadata(&op.op)
            && m.confirmation
            && !confirm.contains(&m.description)
        {
            confirm.push(m.description);
        }
    }
    for what in confirm {
        d.preview.push(PreviewRow::destructive(
            "Needs your confirmation",
            what.trim_end_matches('.'),
        ));
    }
    if d.source_tool.is_empty() {
        d.source_tool = p.tool.to_string();
    }
    Ok(d)
}

/// Preview tone of a change the product UI confirms separately (destructive or
/// irreversible-by-intent). The review card requires that confirmation too.
pub const TONE_CONFIRM: &str = TONE_DESTRUCTIVE;

/// Does applying these operations need the product's extra destructive confirmation?
pub fn requires_confirmation(ops: &[OpCall]) -> bool {
    let reg = crate::registry::catalog();
    ops.iter()
        .any(|o| reg.metadata(&o.op).is_some_and(|m| m.confirmation))
}
