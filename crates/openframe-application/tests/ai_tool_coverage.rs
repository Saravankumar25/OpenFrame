//! Operation inventory and AI tool coverage (agentic spec §6.1, §27, §28, §39, §49.3–4).
//!
//! Fails when any registered operation lacks explicit metadata, when metadata and
//! the toolbox disagree (an exposed operation no tool covers, a hidden operation a
//! tool uses, an assistant operation in a tool), or when a tool schema is not
//! strict and bounded. Writes `docs/engineering/ai-tool-coverage.md` from the live
//! registry so the published matrix can never drift from the code.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use openframe_application::OpKind;
use openframe_application::modules::ai::toolbox::{self, schema};
use openframe_application::registry::{self, AiExposure, OpClass, short_type_name};

fn cap(m: &registry::OperationMetadata) -> String {
    m.required_capability
        .map(|c| format!("{c:?}"))
        .unwrap_or_else(|| "— (app)".into())
}

#[test]
fn every_registered_operation_has_explicit_metadata() {
    let reg = registry::catalog();
    let mut missing = Vec::new();
    for name in reg.op_names() {
        let e = reg.get(name).unwrap();
        match &e.meta {
            None => missing.push(name),
            Some(m) => {
                assert!(!m.module.is_empty(), "{name}: module not set");
                assert!(
                    m.description.len() >= 10 && m.description.ends_with('.'),
                    "{name}: description must be a short sentence"
                );
                match e.kind {
                    OpKind::Query => assert_ne!(
                        m.operation_class,
                        OpClass::Mutate,
                        "{name}: queries never mutate"
                    ),
                    OpKind::Command => assert_eq!(
                        m.operation_class,
                        OpClass::Mutate,
                        "{name}: commands are Mutate"
                    ),
                }
                if m.irreversible {
                    assert!(m.destructive, "{name}: irreversible implies destructive");
                }
                if let AiExposure::Hidden(reason) = m.ai_exposure {
                    assert!(reason.len() > 20, "{name}: hidden without a real reason");
                }
            }
        }
    }
    assert!(
        missing.is_empty(),
        "operations without OperationMetadata: {missing:?}"
    );
}

#[test]
fn tool_coverage_matches_operation_metadata() {
    let reg = registry::catalog();
    let coverage = toolbox::coverage();
    let mut names = BTreeSet::new();
    let mut covered_queries = BTreeSet::new();
    let mut covered_commands = BTreeSet::new();
    for (tool, mutating, ops) in &coverage {
        assert!(names.insert(*tool), "duplicate tool name {tool}");
        assert!(
            !tool.contains("accept") && !tool.contains("change_set") && !tool.contains("apply"),
            "{tool}: no tool may name a review/apply action"
        );
        for op in ops {
            assert!(!op.starts_with("ai."), "{tool} covers assistant op {op}");
            let e = reg
                .get(op)
                .unwrap_or_else(|| panic!("{tool} covers unknown op {op}"));
            let m = e.meta.as_ref().unwrap();
            assert_eq!(
                m.ai_exposure,
                AiExposure::Tool,
                "{tool} covers {op}, which its metadata hides"
            );
            if *mutating {
                assert_eq!(e.kind, OpKind::Command, "{tool}: proposes query {op}");
                covered_commands.insert(*op);
            } else {
                assert_eq!(
                    e.kind,
                    OpKind::Query,
                    "{tool}: read tool covers command {op}"
                );
                covered_queries.insert(*op);
            }
        }
    }
    let mut uncovered = Vec::new();
    for name in reg.op_names() {
        let e = reg.get(name).unwrap();
        if e.meta.as_ref().unwrap().ai_exposure == AiExposure::Tool {
            let ok = match e.kind {
                OpKind::Query => covered_queries.contains(name),
                OpKind::Command => covered_commands.contains(name),
            };
            if !ok {
                uncovered.push(name);
            }
        }
    }
    assert!(
        uncovered.is_empty(),
        "operations exposed to the assistant but covered by no tool: {uncovered:?}"
    );
    // Every tool name the toolbox offers is in the matrix.
    for n in toolbox::all_tool_names() {
        assert!(names.contains(n), "{n} missing from coverage()");
    }
}

#[test]
fn every_tool_schema_is_strict_and_bounded() {
    use openframe_application::modules::ai::{catalog, tools};
    let mut schemas: Vec<(&str, serde_json::Value)> = Vec::new();
    for t in tools::TOOLS {
        schemas.push((t.name, (t.schema)()));
    }
    for r in toolbox::reads::READS {
        schemas.push((r.name, (r.schema)()));
    }
    for p in catalog::PROPOSALS.iter() {
        schemas.push((p.tool, (p.schema)()));
    }
    for (name, s) in schemas {
        assert_eq!(s["type"], "object", "{name}");
        assert_eq!(s["additionalProperties"], false, "{name}");
        assert_eq!(schema::unbounded_part(&s, name), None, "{name}");
    }
}

/// Render the inventory + coverage matrix from the live registry.
fn render() -> String {
    let reg = registry::catalog();
    let coverage = toolbox::coverage();
    let mut by_op: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (tool, _, ops) in &coverage {
        for op in ops {
            by_op.entry(op).or_default().push(tool);
        }
    }
    let mut modules: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut hidden: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let (mut n_q, mut n_c, mut n_exposed) = (0, 0, 0);
    for name in reg.op_names() {
        let e = reg.get(name).unwrap();
        let m = e.meta.as_ref().unwrap();
        modules.entry(m.module).or_default().push(name);
        match e.kind {
            OpKind::Query => n_q += 1,
            OpKind::Command => n_c += 1,
        }
        match m.ai_exposure {
            AiExposure::Tool => n_exposed += 1,
            AiExposure::Hidden(r) => hidden.entry(r).or_default().push(name),
        }
    }
    let read_tools = coverage.iter().filter(|(_, m, _)| !m).count();
    let proposal_tools = coverage.iter().filter(|(_, m, _)| *m).count();
    let mut s = String::new();
    s.push_str("# AI tool coverage — operation inventory\n\n");
    s.push_str("<!-- GENERATED by crates/openframe-application/tests/ai_tool_coverage.rs from the live Registry.\n     Do not edit by hand: run `cargo test -p openframe-application --test ai_tool_coverage`. -->\n\n");
    s.push_str("Every registered operation carries explicit `OperationMetadata` (agentic spec §39). The assistant\n");
    s.push_str("never names registry operations: it chooses **tools**. Read tools answer from canonical data with the\n");
    s.push_str("user's permissions; proposal tools only build a Change Set that a person reviews and applies\n");
    s.push_str("(spec §3, §7, §40). Operations whose UI asks for an extra destructive confirmation keep asking for it\n");
    s.push_str("in the Proposed Changes card.\n\n");
    s.push_str("How the toolbox (`modules/ai/toolbox.rs`, contract C3) enforces this:\n\n");
    s.push_str("- `tools_for` offers read tools to anyone with View, and a proposal tool only when the actor holds its\n  capability **and** ApplyChangeSet and every operation it uses is registered and exposed here. No tool\n  reviews, accepts or applies a Change Set, and no tool touches an `ai.*` operation.\n");
    s.push_str("- `run_tool` re-checks everything (the offered list is not the boundary): unknown tool → `ai.unknown_tool`;\n  arguments are validated against the tool's strict schema (closed objects, bounded strings/lists/numbers,\n  8 KB total) → `ai.tool_arguments`; missing permission → `permission.denied`.\n");
    s.push_str("- Proposal tools resolve references (names, scene/shot/panel numbers, \"Day 3\") to rows, never guess\n  between several matches (`ai.ambiguous` asks one question), record base revisions for stale detection,\n  and return a `ChangeSetDraft`. Every built draft is re-validated: only listed, exposed, registered commands.\n");
    s.push_str("- Read tools are bounded (pages of ≤ 80 items, clipped text) and read through the caller's connection.\n  Another user's private notes are never visible (owner filter on every lookup, search and retrieval).\n");
    s.push_str("- `tools_for_request` picks the core tools plus the request-relevant domain tools within a character\n  budget, so a small local model's prompt stays short. `retrieve_context` uses the hybrid retrieval\n  service when called through `run_tool_unlocked`, and keyword retrieval (FTS5, same privacy rule) otherwise.\n\n");
    let _ = writeln!(
        s,
        "- Registered operations: **{}** ({} queries, {} commands)",
        n_q + n_c,
        n_q,
        n_c
    );
    let _ = writeln!(
        s,
        "- Exposed to the assistant through tools: **{n_exposed}**; not AI-accessible: **{}**",
        n_q + n_c - n_exposed
    );
    let _ = writeln!(
        s,
        "- Tools: **{}** ({read_tools} read/compute/navigate, {proposal_tools} proposal)\n",
        read_tools + proposal_tools
    );
    s.push_str("Flags: **D** destructive · **I** irreversible · **C** extra UI confirmation · **L** long-running (task id) · file effect: R = reads a user-picked file, W = writes to a user-picked location, P = project storage on disk.\n\n");
    s.push_str("## Operations by module\n\n");
    for (module, ops) in &modules {
        let _ = writeln!(s, "### {module}\n");
        s.push_str(
            "| Operation | Kind | Class | Capability | Flags | Args → Result | AI | Covered by |\n",
        );
        s.push_str("|---|---|---|---|---|---|---|---|\n");
        for name in ops {
            let e = reg.get(name).unwrap();
            let m = e.meta.as_ref().unwrap();
            let mut flags = String::new();
            for (on, f) in [
                (m.destructive, "D"),
                (m.irreversible, "I"),
                (m.confirmation, "C"),
                (m.long_running, "L"),
            ] {
                if on {
                    flags.push_str(f);
                }
            }
            match m.filesystem_effect {
                registry::FsEffect::None => {}
                registry::FsEffect::ReadsUserFile => flags.push_str(" R"),
                registry::FsEffect::WritesUserFile => flags.push_str(" W"),
                registry::FsEffect::ProjectStorage => flags.push_str(" P"),
            }
            let (ai, tools) = match m.ai_exposure {
                AiExposure::Tool => (
                    "tool",
                    by_op
                        .get(name)
                        .map(|t| {
                            t.iter()
                                .map(|x| format!("`{x}`"))
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default(),
                ),
                AiExposure::Hidden(_) => ("hidden", "—".to_string()),
            };
            let _ = writeln!(
                s,
                "| `{name}` — {} | {} | {} | {} | {} | `{}` → `{}` | {ai} | {tools} |",
                m.description.replace('|', "/"),
                if e.kind == OpKind::Query {
                    "query"
                } else {
                    "command"
                },
                m.operation_class.as_str(),
                cap(m),
                flags.trim(),
                short_type_name(e.arg_type),
                short_type_name(e.result_type).replace('|', "/"),
            );
        }
        s.push('\n');
    }
    s.push_str("## Not AI-accessible, and why\n\n");
    for (reason, ops) in &hidden {
        let _ = writeln!(s, "- **{reason}**");
        let _ = writeln!(
            s,
            "  {}",
            ops.iter()
                .map(|o| format!("`{o}`"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    s.push_str("\n## Tools\n\n| Tool | Kind | Operations |\n|---|---|---|\n");
    for (tool, mutating, ops) in &coverage {
        let _ = writeln!(
            s,
            "| `{tool}` | {} | {} |",
            if *mutating {
                "proposal (Change Set)"
            } else {
                "read"
            },
            if ops.is_empty() {
                "— (deterministic computation, navigation or clarification)".to_string()
            } else {
                ops.iter()
                    .map(|o| format!("`{o}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        );
    }
    s
}

#[test]
fn coverage_matrix_document_is_generated_from_the_registry() {
    let doc = render();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/engineering/ai-tool-coverage.md");
    let current = std::fs::read_to_string(&path)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    if current != doc {
        std::fs::write(&path, doc.as_bytes()).expect("write ai-tool-coverage.md");
    }
    assert!(
        doc.contains("`ai.change_set.accept`"),
        "the inventory lists every op"
    );
}
