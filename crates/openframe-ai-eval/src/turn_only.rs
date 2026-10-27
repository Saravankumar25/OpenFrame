//! `ai-eval --turn-only`: run the answer-type stage (spec slice S4) alone, with no model writing.
//!
//! Every conversation case and tool case is read the way the assistant reads it: the `before`
//! messages first, in order, each read with the previous message and the previous type, the
//! resolver working on the case's fixture project. The 40 export messages (one conversation, in
//! order) and the 30 held-out messages of `turn-sets/` are read the same way. Three passes show what
//! each step adds: the rules alone, rules plus nearest neighbours (the app's search model), and
//! plus the one short model call for weak votes. Exact and group accuracy are printed overall, per
//! set, per family and per type, with every misread of anger or small talk and every failure.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use openframe_ai::InferenceProvider;
use openframe_ai_eval::Case;
use openframe_ai_eval::turn::{Conv, from_cases, from_sets, index_of, read_conv};
use openframe_application::core::AppCore;
use openframe_application::modules::ai::{intelligence, turn};
use serde_json::{Value, json};

/// How long one message may wait for the embedding model in this run (the first one starts it).
const WAIT: Duration = Duration::from_secs(300);

#[derive(Default, Clone, Copy)]
struct Tally {
    n: usize,
    exact: usize,
    group: usize,
}

impl Tally {
    fn add(&mut self, want: turn::AnswerType, got: turn::AnswerType) {
        self.n += 1;
        self.exact += usize::from(want == got);
        self.group += usize::from(want.group() == got.group());
    }
}

fn pc(a: usize, b: usize) -> String {
    if b == 0 {
        "none".into()
    } else {
        format!("{:.1}% ({a}/{b})", 100.0 * a as f64 / b as f64)
    }
}

struct Row {
    set: String,
    family: String,
    id: String,
    message: String,
    want: Option<turn::AnswerType>,
    /// One read per pass.
    reads: Vec<turn::TurnRead>,
}

pub fn run(
    core: &AppCore,
    cases: &[Case],
    sets_dir: &Path,
    out: &Path,
    model: Option<Arc<dyn InferenceProvider>>,
) -> Result<(), String> {
    let mut convs = from_cases(cases);
    convs.extend(from_sets(sets_dir)?);
    let embedder = intelligence::service(core).embedder(core);
    let mut passes: Vec<&str> = vec!["rules"];
    if embedder.is_some() {
        passes.push("rules + embeddings");
        if model.is_some() {
            passes.push("rules + embeddings + model");
        }
    }
    eprintln!(
        "ai-eval turn-only: {} conversations, passes: {}",
        convs.len(),
        passes.join(", ")
    );
    // The model needs no cancellation here: a run is a measurement, not a person waiting.
    let aids_for = |pass: usize| turn::Aids {
        embedder: if pass >= 1 { embedder.clone() } else { None },
        model: if pass >= 2 { model.as_deref() } else { None },
        stopped: &|| false,
        wait: WAIT,
    };
    let mut rows: Vec<Row> = Vec::new();
    let mut elapsed = vec![Duration::ZERO; passes.len()];
    let mut reads_done = vec![0usize; passes.len()];
    let mut by_project: BTreeMap<&str, Vec<&Conv>> = BTreeMap::new();
    for c in &convs {
        by_project.entry(c.project.as_str()).or_default().push(c);
    }
    for (project, list) in by_project {
        let ix = index_of(core, project)?;
        eprintln!("ai-eval turn-only: {project}: {} conversations", list.len());
        let first = rows.len();
        for c in &list {
            for m in &c.measured {
                rows.push(Row {
                    set: m.set.clone(),
                    family: m.family.clone(),
                    id: m.id.clone(),
                    message: c.turns[m.turn].clone(),
                    want: m.want,
                    reads: Vec::new(),
                });
            }
        }
        for pass in 0..passes.len() {
            let aids = aids_for(pass);
            let mut next = first;
            for c in &list {
                let t = Instant::now();
                let reads = read_conv(&ix, c, &aids);
                elapsed[pass] += t.elapsed();
                reads_done[pass] += reads.len();
                for m in &c.measured {
                    rows[next].reads.push(reads[m.turn].clone());
                    next += 1;
                }
            }
        }
    }
    let loo = embedder.as_deref().and_then(turn::leave_one_out);
    let report = report(&passes, &rows, &elapsed, &reads_done, loo.as_deref());
    std::fs::write(out.join("turn-report.md"), &report).map_err(|e| e.to_string())?;
    let mut lines = String::new();
    for r in &rows {
        let reads: Vec<Value> = r
            .reads
            .iter()
            .map(|t| json!({ "type": t.answer_type.code(), "how": t.how.code(), "vote": t.vote }))
            .collect();
        let _ = writeln!(
            lines,
            "{}",
            json!({ "set": r.set, "family": r.family, "id": r.id, "message": r.message,
                "want": r.want.map(|w| w.code()), "reads": reads,
                "domains": r.reads.last().map(|t| t.domain_codes()),
                "lengthClass": r.reads.last().map(|t| t.length_class.code()) })
        );
    }
    std::fs::write(out.join("turn-results.jsonl"), lines).map_err(|e| e.to_string())?;
    println!("{report}");
    Ok(())
}

fn report(
    passes: &[&str],
    rows: &[Row],
    elapsed: &[Duration],
    reads_done: &[usize],
    loo: Option<&[turn::LooRead]>,
) -> String {
    let mut t = String::from("# Answer type read (spec 5.3, slice S4)\n\n");
    let labelled: Vec<&Row> = rows.iter().filter(|r| r.want.is_some()).collect();
    let _ = writeln!(
        t,
        "{} measured messages, {} with a label. Exact: the type is right. Group: the coarse group is right (facts, narrative, brainstorm, navigate, change, help, chat).\n",
        rows.len(),
        labelled.len()
    );
    t.push_str("| Pass | Reads | Mean ms per read |\n|---|---|---|\n");
    for (i, p) in passes.iter().enumerate() {
        let mean = elapsed[i].as_secs_f64() * 1000.0 / reads_done[i].max(1) as f64;
        let _ = writeln!(t, "| {p} | {} | {mean:.1} |", reads_done[i]);
    }
    let table = |t: &mut String, title: &str, key: &dyn Fn(&Row) -> String| {
        let mut groups: BTreeMap<String, Vec<Tally>> = BTreeMap::new();
        for r in &labelled {
            let tallies = groups
                .entry(key(r))
                .or_insert_with(|| vec![Tally::default(); passes.len()]);
            for (i, read) in r.reads.iter().enumerate() {
                tallies[i].add(r.want.expect("labelled"), read.answer_type);
            }
        }
        let _ = write!(t, "\n## {title}\n\n| Group | Messages |");
        for p in passes {
            let _ = write!(t, " {p}: exact | group |");
        }
        t.push_str("\n|---|---|");
        for _ in passes {
            t.push_str("---|---|");
        }
        t.push('\n');
        let all: Vec<Tally> = (0..passes.len())
            .map(|i| {
                groups.values().fold(Tally::default(), |a, v| Tally {
                    n: a.n + v[i].n,
                    exact: a.exact + v[i].exact,
                    group: a.group + v[i].group,
                })
            })
            .collect();
        let mut line = |name: &str, v: &[Tally]| {
            let _ = write!(t, "| {name} | {} |", v[0].n);
            for x in v {
                let _ = write!(t, " {} | {} |", pc(x.exact, x.n), pc(x.group, x.n));
            }
            t.push('\n');
        };
        for (k, v) in &groups {
            line(k, v);
        }
        line("all", &all);
    };
    table(&mut t, "By set", &|r| r.set.clone());
    table(&mut t, "By family", &|r| {
        format!("{} / {}", r.set, r.family)
    });
    table(&mut t, "By expected type", &|r| {
        r.want.map_or(String::new(), |w| w.code().to_string())
    });

    let last = passes.len() - 1;
    t.push_str("\n## What decided each read (last pass)\n\n| Set | rule | repair | neighbour | model | fallback |\n|---|---|---|---|---|---|\n");
    let mut how: BTreeMap<&str, [usize; 5]> = BTreeMap::new();
    for r in rows {
        let slot = match r.reads[last].how {
            turn::ReadHow::Rule => 0,
            turn::ReadHow::Repair => 1,
            turn::ReadHow::Neighbour => 2,
            turn::ReadHow::Model => 3,
            turn::ReadHow::Fallback => 4,
        };
        how.entry(r.set.as_str()).or_default()[slot] += 1;
    }
    for (set, c) in &how {
        let _ = writeln!(
            t,
            "| {set} | {} | {} | {} | {} | {} |",
            c[0], c[1], c[2], c[3], c[4]
        );
    }

    t.push_str("\n## Anger and small talk (labelled chat)\n\nMisread as change, count or a fact fetch (lookup, list, compare) is the bar: none allowed.\n\n| Pass | Chat messages | Read as chat | Read as change, count or a fetch | Read as something else |\n|---|---|---|---|---|\n");
    let chat: Vec<&&Row> = labelled
        .iter()
        .filter(|r| r.want == Some(turn::AnswerType::Chat))
        .collect();
    for (i, p) in passes.iter().enumerate() {
        use turn::AnswerType::*;
        let (mut ok, mut bad, mut other) = (0, 0, 0);
        for r in &chat {
            match r.reads[i].answer_type {
                Chat => ok += 1,
                Change | Count | Lookup | List | Compare => bad += 1,
                _ => other += 1,
            }
        }
        let _ = writeln!(t, "| {p} | {} | {ok} | {bad} | {other} |", chat.len());
    }

    let _ = write!(t, "\n## Misreads ({})\n\n", passes[last]);
    t.push_str("| Set | Id | Wanted | Read | By | Message |\n|---|---|---|---|---|---|\n");
    for r in &labelled {
        let (want, got) = (r.want.expect("labelled"), &r.reads[last]);
        if want != got.answer_type {
            let _ = writeln!(
                t,
                "| {} | {} | {} | {} | {} | {} |",
                r.set,
                r.id,
                want.code(),
                got.answer_type.code(),
                got.how.code(),
                r.message.replace('|', "/").replace('\n', " ")
            );
        }
    }

    if let Some(loo) = loo {
        let n = loo.len();
        let right = loo.iter().filter(|r| r.expected == r.got).count();
        let clear: Vec<_> = loo.iter().filter(|r| r.clear).collect();
        let clear_right = clear.iter().filter(|r| r.expected == r.got).count();
        let _ = write!(
            t,
            "\n## Leave-one-out on the example file\n\nEach example read by the others: {} right of {n}. Clear votes: {} ({}), right {}. Weak votes go to the model.\n",
            pc(right, n),
            clear.len(),
            pc(clear.len(), n),
            pc(clear_right, clear.len()),
        );
        let mut by_lang: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
        for r in loo {
            let e = by_lang.entry(r.lang.as_str()).or_default();
            e.0 += 1;
            e.1 += usize::from(r.expected == r.got);
        }
        let langs: Vec<String> = by_lang
            .iter()
            .map(|(l, (n, ok))| format!("{l} {}", pc(*ok, *n)))
            .collect();
        let _ = writeln!(t, "\nBy language: {}.", langs.join(", "));
    }
    t
}
