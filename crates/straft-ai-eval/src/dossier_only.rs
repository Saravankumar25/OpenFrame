//! `ai-eval --dossier-only`: run the fetch stage (spec slice S5) alone, with no model writing.
//!
//! Every conversation case on the Low Tide fixtures is read the way the assistant reads it (the
//! `before` messages first, then index, resolve, answer type) and the dossier is built for the last
//! message from the fixture project. Each entry the case expects of the answer is then looked for in
//! the rendered dossier and in the statements about what is missing: every `required_facts` entry,
//! every `answer_has` group (any alternative), and for an `abstain` case a statement naming the
//! thing. The match ignores case and the digit grouping of amounts (`1,64,60,000` is `16460000`).
//! This measures the fetch plans, not the writer: a fact that is in the dossier can still be
//! written wrongly, and S7 and the checker measure that.
//!
//! A case with `sections` (ADR-0041) reads only those sections, through the same picks and leaf-scoped
//! text search a request uses. The out-of-section pointer is code, so its precision, recall and
//! false-pointer rate are measured here with no model at all.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use straft_ai::InferenceProvider;
use straft_ai_eval::turn::{from_cases, index_of, picks_of, read_conv_resolved};
use straft_ai_eval::{Case, PointerCheck, PointerMetrics};
use straft_application::core::AppCore;
use straft_application::modules::ai::sections;
use straft_application::modules::ai::{dossier, intelligence, orchestrator, pointer, turn};
use serde_json::{Value, json};

/// How long one message may wait for the embedding model (the first one starts it).
const WAIT: Duration = Duration::from_secs(300);

/// Lower case, with the commas inside numbers taken out.
fn norm(s: &str) -> String {
    let chars: Vec<char> = s.to_lowercase().chars().collect();
    let mut out = String::with_capacity(chars.len());
    for (i, c) in chars.iter().enumerate() {
        let between_digits = *c == ','
            && i > 0
            && chars[i - 1].is_ascii_digit()
            && chars.get(i + 1).is_some_and(char::is_ascii_digit);
        if !between_digits {
            out.push(*c);
        }
    }
    out
}

struct Entry {
    /// What the case asked for, for the report.
    label: String,
    alternatives: Vec<String>,
}

fn entries(c: &Case) -> Vec<Entry> {
    let mut out = Vec::new();
    for f in &c.expect.required_facts {
        out.push(Entry {
            label: f.clone(),
            alternatives: vec![f.clone()],
        });
    }
    for g in &c.expect.answer_has {
        let alts: Vec<String> = match g {
            Value::String(s) => vec![s.clone()],
            Value::Array(a) => a
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect(),
            _ => Vec::new(),
        };
        if let Some(first) = alts.first() {
            out.push(Entry {
                label: first.clone(),
                alternatives: alts,
            });
        }
    }
    out
}

struct Row {
    family: String,
    id: String,
    message: String,
    answer_type: String,
    facts: usize,
    tokens: usize,
    required: usize,
    missing: Vec<String>,
    /// Entries looked for, and the ones not found.
    wanted: usize,
    found: usize,
    misses: Vec<String>,
    abstain: Option<bool>,
    build_ms: f64,
    guard_notes: usize,
    text: String,
    /// What the pointer named against what the case wants, on a case with sections.
    pointer: Option<PointerCheck>,
    /// The reply code chose instead of the writer, on a case with sections.
    verdict: String,
}

pub fn run(
    core: &AppCore,
    cases: &[Case],
    out: &Path,
    model: Option<Arc<dyn InferenceProvider>>,
) -> Result<(), String> {
    let cases: Vec<&Case> = cases
        .iter()
        .filter(|c| c.id.starts_with("conv-lt-") || c.id.starts_with("sec-"))
        .collect();
    let owned: Vec<Case> = cases.iter().map(|c| (*c).clone()).collect();
    let convs = from_cases(&owned);
    let embedder = intelligence::service(core).embedder(core);
    eprintln!(
        "ai-eval dossier-only: {} conversation cases, embeddings: {}, model: {}",
        convs.len(),
        if embedder.is_some() {
            "yes"
        } else {
            "no (rules only)"
        },
        if model.is_some() { "yes" } else { "no" }
    );
    let aids = turn::Aids {
        embedder: embedder.clone(),
        model: model.as_deref(),
        stopped: &|| false,
        wait: WAIT,
    };
    let mut by_project: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (i, c) in convs.iter().enumerate() {
        by_project.entry(c.project.as_str()).or_default().push(i);
    }
    let mut rows: Vec<Row> = Vec::new();
    for (project, list) in by_project {
        let ix = index_of(core, project)?;
        let session = core.project().map_err(|e| e.message)?;
        let actor = core.local_actor();
        eprintln!("ai-eval dossier-only: {project}: {} cases", list.len());
        for i in list {
            let conv = &convs[i];
            let m = &conv.measured[0];
            let case = owned
                .iter()
                .find(|c| c.id == m.id)
                .ok_or_else(|| format!("case {} not found", m.id))?;
            let reads = read_conv_resolved(&ix, conv, &aids);
            let (resolved, read, focus) = &reads[m.turn];
            let read = &orchestrator::hinted(read.clone(), &conv.turns[m.turn]);
            let message = &conv.turns[m.turn];
            let selection = picks_of(core, case, &ix)?;
            // The same text search a request runs: it opens the store itself, so it goes first.
            let retrieved = sections::retrieve(core, &session, &actor, &selection, message);
            let leaf_read = dossier::LeafRead {
                selection: &selection,
                passages: &retrieved.passages,
            };
            let started = Instant::now();
            let (d, verdict) = session
                .store
                .read(|c| {
                    let input = dossier::Input {
                        conn: c,
                        actor: &actor,
                        index: &ix,
                        core: Some(core),
                        message,
                        read,
                        resolved,
                        focus: Some(focus),
                        previous_message: m.turn.checked_sub(1).map(|i| conv.turns[i].as_str()),
                        sections: &leaf_read,
                    };
                    let d = dossier::build(&input);
                    let verdict = pointer::decide(&input, &selection, &d);
                    Ok((d, verdict))
                })
                .map_err(|e| e.message)?;
            let build_ms = started.elapsed().as_secs_f64() * 1000.0;
            let text = d.render();
            let hay = norm(&format!("{text}\n{}", d.missing.join("\n")));
            let wanted = entries(case);
            let mut misses = Vec::new();
            let mut found = 0;
            for e in &wanted {
                if e.alternatives.iter().any(|a| hay.contains(&norm(a))) {
                    found += 1;
                } else {
                    misses.push(e.label.clone());
                }
            }
            let abstain = case.expect.abstain.then(|| {
                let missing = norm(&d.missing.join("\n"));
                if case.expect.abstain_names.is_empty() {
                    !d.missing.is_empty()
                } else {
                    case.expect
                        .abstain_names
                        .iter()
                        .any(|n| missing.contains(&norm(n)))
                }
            });
            rows.push(Row {
                family: m.family.clone(),
                id: m.id.clone(),
                message: message.clone(),
                answer_type: read.answer_type.code().to_string(),
                facts: d.facts.len(),
                tokens: d.tokens_estimate,
                required: d.required.len(),
                missing: d.missing.clone(),
                wanted: wanted.len(),
                found,
                misses,
                abstain,
                build_ms,
                guard_notes: d.notes_on_guard,
                text,
                // Every section read leaves nothing to point at: only a case with picks counts.
                pointer: case.sections.as_ref().map(|_| PointerCheck {
                    want: case.expect.pointer.clone(),
                    got: verdict.pointer().map(|p| p.leaf.id.to_string()),
                }),
                verdict: format!("{verdict:?}"),
            });
        }
    }
    let report = report(&rows);
    std::fs::write(out.join("dossier-report.md"), &report).map_err(|e| e.to_string())?;
    let mut lines = String::new();
    for r in &rows {
        let _ = writeln!(
            lines,
            "{}",
            json!({ "id": r.id, "family": r.family, "message": r.message, "type": r.answer_type,
                "facts": r.facts, "tokens": r.tokens, "required": r.required,
                "missing": r.missing, "wanted": r.wanted, "found": r.found, "misses": r.misses,
                "abstain": r.abstain, "buildMs": r.build_ms, "guardNotes": r.guard_notes,
                "pointerWant": r.pointer.as_ref().and_then(|p| p.want.clone()),
                "pointerGot": r.pointer.as_ref().and_then(|p| p.got.clone()),
                "verdict": r.verdict, "dossier": r.text })
        );
    }
    std::fs::write(out.join("dossier-results.jsonl"), lines).map_err(|e| e.to_string())?;
    println!("{report}");
    Ok(())
}

fn pc(a: usize, b: usize) -> String {
    if b == 0 {
        "none".into()
    } else {
        format!("{:.1}% ({a}/{b})", 100.0 * a as f64 / b as f64)
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let i = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[i.min(sorted.len() - 1)]
}

#[derive(Default)]
struct Tally {
    cases: usize,
    wanted: usize,
    found: usize,
    whole: usize,
    abstain: usize,
    abstain_ok: usize,
    facts: usize,
    tokens: Vec<f64>,
    ms: Vec<f64>,
}

impl Tally {
    fn add(&mut self, r: &Row) {
        self.cases += 1;
        self.wanted += r.wanted;
        self.found += r.found;
        self.whole += usize::from(r.wanted > 0 && r.found == r.wanted);
        if let Some(ok) = r.abstain {
            self.abstain += 1;
            self.abstain_ok += usize::from(ok);
        }
        self.facts += r.facts;
        self.tokens.push(r.tokens as f64);
        self.ms.push(r.build_ms);
    }

    fn with_entries(&self, rows: &[&Row]) -> usize {
        rows.iter().filter(|r| r.wanted > 0).count()
    }
}

fn report(rows: &[Row]) -> String {
    let mut t = String::from("# Dossier fetch (spec 5.4, slice S5)\n\n");
    let _ = writeln!(
        t,
        "{} conversation cases on the Low Tide fixtures. Recall: how many of the entries a case expects of the answer (each required fact, each answer_has group with any alternative) are in the dossier or in the statements about what is missing. Abstain: the statements name the thing. This measures the fetch, not the writing.\n",
        rows.len()
    );
    let mut all = Tally::default();
    let mut fam: BTreeMap<&str, Tally> = BTreeMap::new();
    for r in rows {
        all.add(r);
        fam.entry(r.family.as_str()).or_default().add(r);
    }
    let mut metrics = PointerMetrics::default();
    for p in rows.iter().filter_map(|r| r.pointer.as_ref()) {
        metrics.add(p);
    }
    if metrics.expected + metrics.quiet_cases > 0 {
        t.push_str("## Out-of-section pointer (ADR-0041), code only\n\n");
        t.push_str(&metrics.table());
        t.push('\n');
    }
    let refs: Vec<&Row> = rows.iter().collect();
    t.push_str("| Family | Cases | Recall | Cases fully covered | Abstain | Mean facts | Mean tokens | p95 tokens |\n|---|---|---|---|---|---|---|---|\n");
    let mut line = |name: &str, v: &Tally, with: usize| {
        let mut tok = v.tokens.clone();
        tok.sort_by(f64::total_cmp);
        let _ = writeln!(
            t,
            "| {name} | {} | {} | {} | {} | {:.1} | {:.0} | {:.0} |",
            v.cases,
            pc(v.found, v.wanted),
            pc(v.whole, with),
            pc(v.abstain_ok, v.abstain),
            v.facts as f64 / v.cases.max(1) as f64,
            tok.iter().sum::<f64>() / tok.len().max(1) as f64,
            percentile(&tok, 0.95),
        );
    };
    for (name, v) in &fam {
        let with = rows
            .iter()
            .filter(|r| r.family == *name && r.wanted > 0)
            .count();
        line(name, v, with);
    }
    line("all", &all, all.with_entries(&refs));
    let mut ms = all.ms.clone();
    ms.sort_by(f64::total_cmp);
    let _ = writeln!(
        t,
        "\nBuild time per dossier: mean {:.1} ms, p95 {:.1} ms, max {:.1} ms.",
        ms.iter().sum::<f64>() / ms.len().max(1) as f64,
        percentile(&ms, 0.95),
        ms.last().copied().unwrap_or(0.0)
    );
    let chat_with_facts = rows
        .iter()
        .filter(|r| matches!(r.answer_type.as_str(), "chat" | "change") && r.facts > 0)
        .count();
    let _ = writeln!(
        t,
        "Chat or change turns that were given facts: {chat_with_facts} (none wanted)."
    );
    let guard: usize = rows.iter().map(|r| r.guard_notes).sum();
    let _ = writeln!(t, "Guard actions across the run: {guard}.");
    // The fixture holds a private note of a second user ("PRIVATE-OTHER-n"); no dossier may carry it.
    let leaked = rows
        .iter()
        .filter(|r| {
            r.text.contains("PRIVATE-OTHER")
                || r.missing.iter().any(|m| m.contains("PRIVATE-OTHER"))
        })
        .count();
    let _ = writeln!(
        t,
        "Dossiers that carry another user's private note: {leaked} (none allowed)."
    );
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    for r in rows {
        for line in r.text.lines() {
            let kind = line.split(" | ").next().unwrap_or("");
            // A relationship line starts with its sentence; count it under its own name.
            let kind = if line.contains(" | id: ") && kind.ends_with('.') {
                "Relationship"
            } else {
                kind
            };
            *kinds.entry(kind.to_string()).or_default() += 1;
        }
    }
    let _ = writeln!(
        t,
        "Facts by kind over the run: {}.",
        kinds
            .iter()
            .map(|(k, n)| format!("{k} {n}"))
            .collect::<Vec<_>>()
            .join(", ")
    );

    t.push_str("\n## Entries not found\n\n| Id | Type | Message | Not found | Facts |\n|---|---|---|---|---|\n");
    for r in rows.iter().filter(|r| !r.misses.is_empty()) {
        let _ = writeln!(
            t,
            "| {} | {} | {} | {} | {} |",
            r.id,
            r.answer_type,
            r.message.replace('|', "/").replace('\n', " "),
            r.misses.join("; ").replace('|', "/"),
            r.facts
        );
    }
    t.push_str("\n## Abstain cases not covered\n\n| Id | Message | Statements |\n|---|---|---|\n");
    for r in rows.iter().filter(|r| r.abstain == Some(false)) {
        let _ = writeln!(
            t,
            "| {} | {} | {} |",
            r.id,
            r.message.replace('|', "/"),
            r.missing.join("; ").replace('|', "/")
        );
    }
    t
}
