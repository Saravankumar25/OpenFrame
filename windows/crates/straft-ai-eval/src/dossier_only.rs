//! `ai-eval --dossier-only`: run the fetch stage (spec slice S5) alone, with no model writing.
//!
//! Every conversation case on the Low Tide fixtures (the 251 `conv-lt-` cases and the section-scoped
//! `sec-` cases) is read the way the assistant reads it (the `before` messages first, then index,
//! resolve, answer type) and the dossier is built for the last message from the fixture project,
//! under the sections the case picks (a legacy case: the leaves `legacy::leaves_for` maps it to). Each entry the case expects of the answer is then looked for in
//! the rendered dossier and in the statements about what is missing: every `required_facts` entry,
//! every `answer_has` group (any alternative), and for an `abstain` case a statement naming the
//! thing. The match ignores case and the digit grouping of amounts (`1,64,60,000` is `16460000`).
//! This measures the fetch plans, not the writer: a fact that is in the dossier can still be
//! written wrongly, and S7 and the checker measure that.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use straft_ai::InferenceProvider;
use straft_ai_eval::Case;
use straft_ai_eval::turn::{from_cases, index_of, read_conv_resolved};
use straft_ai_eval::{legacy, picks};
use straft_application::core::AppCore;
use straft_application::modules::ai::sections::pointer::{self, Decision};
use straft_application::modules::ai::{dossier, intelligence, turn};

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
    domains: String,
    /// The leaves the case was asked under, comma separated.
    leaves: String,
    /// The leaf when the case picks one leaf alone, else its family (pair, triple, neg ...).
    leaf: String,
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
    /// Round R4: the leaf the case expects a pointer to, and the one the pointer stage chose.
    pointer_expected: Option<String>,
    /// Round R5: whether the unsupported-relationship and unstated-mood checks flag the expected answer.
    flags: Option<(bool, bool)>,
    pointer: Option<String>,
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
            // The scene the case has selected on screen, as the app's scope would carry it.
            let selected = case
                .scope
                .as_ref()
                .filter(|s| s.get("kind").and_then(Value::as_str) == Some("scene"))
                .and_then(|s| s.get("scene").and_then(Value::as_u64))
                .and_then(|n| {
                    let sp = ix.screenplays.iter().max_by_key(|s| s.scene_count)?;
                    ix.scene(&sp.id, n as u32).map(|e| e.id.clone())
                });
            let message = &conv.turns[m.turn];
            // The feature's scenes by ordinal, for a scene pick that names no screenplay.
            let feature: Vec<String> = ix
                .screenplays
                .iter()
                .max_by_key(|s| s.scene_count)
                .map(|sp| {
                    (1..=sp.scene_count)
                        .filter_map(|n| ix.scene(&sp.id, n).map(|e| e.id.clone()))
                        .collect()
                })
                .unwrap_or_default();
            let sections: Vec<serde_json::Value> = if case.sections.is_empty() {
                legacy::leaves_for(&case.kind, &case.id, &read.domains)
                    .into_iter()
                    .map(|l| json!({ "leaf": l }))
                    .collect()
            } else {
                picks::resolve(core, &case.sections, &feature)
                    .map_err(|e| format!("{}: {e}", case.id))?
            };
            let picked: Vec<straft_application::modules::ai::sections::SectionPick> =
                serde_json::from_value(Value::Array(sections.clone()))
                    .map_err(|e| format!("{}: {e}", case.id))?;
            let leaves = picked
                .iter()
                .map(|p| p.leaf.as_str())
                .collect::<Vec<_>>()
                .join(",");
            let leaf = if picked.len() == 1 {
                picked[0].leaf.clone()
            } else {
                m.family.clone()
            };
            let started = Instant::now();
            let d = session
                .store
                .read(|c| {
                    Ok(dossier::build(&dossier::Input {
                        conn: c,
                        actor: &actor,
                        index: &ix,
                        core: Some(core),
                        message,
                        read,
                        resolved,
                        selected_scene: selected.as_deref(),
                        focus: Some(focus),
                        previous_message: m.turn.checked_sub(1).map(|i| conv.turns[i].as_str()),
                        sections: &picked,
                    }))
                })
                .map_err(|e| e.message)?;
            let build_ms = started.elapsed().as_secs_f64() * 1000.0;
            // Round R4: the pointer is code only, so it is measured here without the model. The leaf
            // shown is the one the pointer chose; None means the answer is written from the picked
            // leaves (a not-found for a leaf the person may not read counts as none).
            let pointer = if pointer::applies(read.group) {
                session
                    .store
                    .read(|c| {
                        Ok(pointer::decide(&pointer::Input {
                            conn: c,
                            user_id: &actor.user_id,
                            message,
                            read,
                            dossier: &d,
                            picks: &picked,
                            may: &|l| actor.can(l.min_capability),
                            vault: Some((core, &actor)),
                        }))
                    })
                    .map_err(|e| e.message)?
                    .and_then(|x| match x {
                        Decision::Point(p) => Some(p.leaf.to_string()),
                        Decision::NotFound(_) => None,
                    })
            } else {
                None
            };
            let text = d.render();
            let hay = norm(&format!("{text}\n{}", d.missing.join("\n")));
            let wanted = entries(case);
            // Round R5: the expected answer of the case, put together from what it expects, run
            // through the two checks that are off in the app. A flag here is a false positive.
            let expected_answer = wanted
                .iter()
                .filter_map(|e| e.alternatives.first().cloned())
                .collect::<Vec<_>>()
                .join(". ");
            let flags = (!expected_answer.is_empty()).then(|| {
                straft_application::modules::ai::answer::narrative_flags(
                    &expected_answer,
                    &[&d.render_plain(), &d.missing.join("\n"), message],
                )
            });
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
                domains: read.domain_codes(),
                leaves,
                leaf,
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
                pointer_expected: case.expect.pointer.clone(),
                flags,
                pointer,
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
                "domains": r.domains, "leaves": r.leaves, "facts": r.facts, "tokens": r.tokens, "required": r.required,
                "missing": r.missing, "wanted": r.wanted, "found": r.found, "misses": r.misses,
                "abstain": r.abstain, "pointer": r.pointer, "pointerExpected": r.pointer_expected, "buildMs": r.build_ms, "guardNotes": r.guard_notes,
                "dossier": r.text })
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
    let refs: Vec<&Row> = rows.iter().collect();
    t.push_str("| Family | Cases | Recall | Cases fully covered | Abstain | Mean facts | Mean tokens | p95 tokens |\n|---|---|---|---|---|---|---|---|\n");
    fn line(t: &mut String, name: &str, v: &Tally, with: usize) {
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
    }
    for (name, v) in &fam {
        let with = rows
            .iter()
            .filter(|r| r.family == *name && r.wanted > 0)
            .count();
        line(&mut t, name, v, with);
    }
    line(&mut t, "all", &all, all.with_entries(&refs));
    // Section-scoped cases: recall by the leaf a case picks alone, and by family for the rest.
    if rows.iter().any(|r| r.id.starts_with("sec-")) {
        let mut by_leaf: BTreeMap<&str, Tally> = BTreeMap::new();
        for r in rows.iter().filter(|r| r.id.starts_with("sec-")) {
            by_leaf.entry(r.leaf.as_str()).or_default().add(r);
        }
        t.push_str("\n## Required-fact recall per leaf (section-scoped cases)\n\n| Leaf or family | Cases | Recall | Cases fully covered | Abstain | Mean facts | Mean tokens | p95 tokens |\n|---|---|---|---|---|---|---|---|\n");
        for (name, v) in &by_leaf {
            let with = rows
                .iter()
                .filter(|r| r.id.starts_with("sec-") && r.leaf == *name && r.wanted > 0)
                .count();
            line(&mut t, name, v, with);
        }
        let mut all_sec = Tally::default();
        for r in rows.iter().filter(|r| r.id.starts_with("sec-")) {
            all_sec.add(r);
        }
        let with = rows
            .iter()
            .filter(|r| r.id.starts_with("sec-") && r.wanted > 0)
            .count();
        line(&mut t, "all sec- cases", &all_sec, with);
        // The pointer (spec 12.4), by family: code only, so it is fully measured by this run.
        t.push_str("\n## Pointer (section-scoped cases)\n\nPrecision: pointers that went to the expected leaf. Recall: expected pointers that fired right. False pointers: cases that expect none and got one. Bars: 90 percent, 90 percent, at most 5 percent.\n\n| Family | Expect a pointer | Pointed | Precision | Recall | False pointers |\n|---|---|---|---|---|---|\n");
        let mut fams: BTreeMap<&str, Vec<&Row>> = BTreeMap::new();
        for r in rows.iter().filter(|r| r.id.starts_with("sec-")) {
            fams.entry(r.family.as_str()).or_default().push(r);
        }
        let all_sec: Vec<&Row> = rows.iter().filter(|r| r.id.starts_with("sec-")).collect();
        let mut lines: Vec<(&str, &Vec<&Row>)> = fams.iter().map(|(k, v)| (*k, v)).collect();
        lines.push(("all sec- cases", &all_sec));
        for (name, rs) in lines {
            let expected = rs.iter().filter(|r| r.pointer_expected.is_some()).count();
            let fired = rs.iter().filter(|r| r.pointer.is_some()).count();
            let right = rs
                .iter()
                .filter(|r| r.pointer.is_some() && r.pointer == r.pointer_expected)
                .count();
            let none = rs.len() - expected;
            let wrong = rs
                .iter()
                .filter(|r| r.pointer_expected.is_none() && r.pointer.is_some())
                .count();
            let _ = writeln!(
                t,
                "| {name} | {expected} | {fired} | {} | {} | {} |",
                pc(right, fired),
                pc(right, expected),
                pc(wrong, none)
            );
        }
        // The two narrative checks that are off in the app (spec 3.7): turned on only when this is under 5 percent.
        let tested: Vec<&Row> = rows
            .iter()
            .filter(|r| r.id.starts_with("sec-") && r.flags.is_some())
            .collect();
        let rel = tested
            .iter()
            .filter(|r| r.flags.is_some_and(|f| f.0))
            .count();
        let mood = tested
            .iter()
            .filter(|r| r.flags.is_some_and(|f| f.1))
            .count();
        let _ = writeln!(
            t,
            "\n## Narrative checks that are off (false positives)\n\nThe expected answer of each section-scoped case (its required facts and first alternatives) run through the checks. A flag means the check would refuse a correct answer. Turn a check on only when its rate is under 5 percent.\n\n| Check | Cases tested | Flagged | False positives |\n|---|---|---|---|\n| Unsupported relationship word | {} | {rel} | {} |\n| Unstated mood | {} | {mood} | {} |",
            tested.len(),
            pc(rel, tested.len()),
            tested.len(),
            pc(mood, tested.len())
        );
        t.push_str("\n### Pointer misses and false pointers\n\n| Id | Leaves | Message | Expected | Chose |\n|---|---|---|---|---|\n");
        for r in rows
            .iter()
            .filter(|r| r.id.starts_with("sec-") && r.pointer_expected != r.pointer)
        {
            let _ = writeln!(
                t,
                "| {} | {} | {} | {} | {} |",
                r.id,
                r.leaves,
                r.message.replace('|', "/").replace('\n', " "),
                r.pointer_expected.as_deref().unwrap_or("none"),
                r.pointer.as_deref().unwrap_or("none")
            );
        }
    }
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

    t.push_str("\n## Entries not found\n\n| Id | Type | Leaves | Message | Not found | Facts |\n|---|---|---|---|---|---|\n");
    for r in rows.iter().filter(|r| !r.misses.is_empty()) {
        let _ = writeln!(
            t,
            "| {} | {} | {} | {} | {} | {} |",
            r.id,
            r.answer_type,
            r.leaves,
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
