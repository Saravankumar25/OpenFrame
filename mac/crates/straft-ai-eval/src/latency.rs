//! The latency report (spec `ai-conversational-harness`, section 7.5 and slice S8): how long the
//! assistant takes to answer, and where the time goes. It is a section of every normal run, written
//! from numbers the run already records, so a number is either measured or absent, never guessed.
//!
//! Per case: the wall time of `ai.ask`, the stage times of the trace (`turn_ms`, `dossier_ms`), the
//! engine's own figures (prompt, cached and completion tokens, prompt and generation time), and any
//! stage time a later slice records in the request's `scope_json` as a `<stage>_ms` number (for
//! example `memory_ms`, `write_ms`, `check_ms`, at the top or under `timings`). A stage nobody
//! records is not reported.
//!
//! Report: median and 95th percentile against the bars, the same per length class, the first
//! question after the project opened, the share of prompt tokens the engine served from its stored
//! work, and the mean of each stage. `ai-eval --latency-from <run folder>` rebuilds it from a
//! finished run's `results.jsonl`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Section 7.5: median at most this many milliseconds on the reference machine.
pub const MEDIAN_BAR_MS: u64 = 15_000;
/// Section 7.5: 95th percentile at most this many milliseconds.
pub const P95_BAR_MS: u64 = 30_000;

/// The times and engine figures of one case.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CaseTiming {
    pub id: String,
    pub length_class: String,
    pub elapsed_ms: u64,
    pub turn_ms: u64,
    pub dossier_ms: u64,
    pub passes: u32,
    pub calls: u32,
    pub prompt_tokens: u32,
    pub cached_tokens: u32,
    pub completion_tokens: u32,
    pub prompt_ms: u32,
    pub generation_ms: u32,
    /// Stage times recorded with the request (`resolve`, `memory`, `write`, `check`, ...).
    pub stages: BTreeMap<String, u64>,
}

/// What the trace row of a request holds.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TraceTimes {
    pub turn_ms: u64,
    pub dossier_ms: u64,
    pub passes: u32,
    pub length_class: String,
}

pub fn times_from_trace(c: &rusqlite::Connection, request_id: &str) -> Option<TraceTimes> {
    c.query_row(
        "SELECT turn_ms, dossier_ms, passes, length_class FROM sys_ai_trace WHERE request_id = ?1",
        [request_id],
        |r| {
            Ok(TraceTimes {
                turn_ms: r.get::<_, i64>(0)?.max(0) as u64,
                dossier_ms: r.get::<_, i64>(1)?.max(0) as u64,
                passes: r.get::<_, i64>(2)?.max(0) as u32,
                length_class: r.get(3)?,
            })
        },
    )
    .ok()
}

/// The `<stage>_ms` numbers a request recorded in its `scope_json`, at the top or under `timings`.
/// Anything else, and a value that is not a number, is ignored.
pub fn stages_from_scope(scope_json: &Value) -> BTreeMap<String, u64> {
    let mut out = BTreeMap::new();
    let mut take = |o: &serde_json::Map<String, Value>| {
        for (k, v) in o {
            if let (Some(stage), Some(ms)) = (k.strip_suffix("_ms"), v.as_f64())
                && ms.is_finite()
                && ms >= 0.0
            {
                out.insert(stage.to_string(), ms as u64);
            }
        }
    };
    if let Some(o) = scope_json.as_object() {
        take(o);
        if let Some(t) = o.get("timings").and_then(Value::as_object) {
            take(t);
        }
    }
    out
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Spread {
    pub n: usize,
    pub median_ms: u64,
    pub p95_ms: u64,
}

/// Median and 95th percentile (nearest rank) of the values.
pub fn spread(values: &[u64]) -> Spread {
    if values.is_empty() {
        return Spread::default();
    }
    let mut v = values.to_vec();
    v.sort_unstable();
    let n = v.len();
    let median = if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2
    };
    let rank = (n * 95).div_ceil(100).max(1);
    Spread {
        n,
        median_ms: median,
        p95_ms: v[rank - 1],
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LatencyReport {
    pub cases: usize,
    /// Every case that ran a question through `ai.ask`.
    pub all: Spread,
    /// Only the cases a model wrote or checked (at least one engine call).
    pub model: Spread,
    pub by_length_class: BTreeMap<String, Spread>,
    /// `ai.ask` of the first case after the project opened, and of the first case that used the engine.
    pub first_question_ms: Option<u64>,
    pub first_model_question_ms: Option<u64>,
    pub prompt_tokens: u64,
    pub cached_tokens: u64,
    pub completion_tokens: u64,
    pub calls: u64,
    /// Cached prompt tokens over prompt tokens, 0 to 1.
    pub cached_share: f64,
    /// Mean milliseconds of each stage, over the cases that report it.
    pub stage_mean_ms: BTreeMap<String, u64>,
    pub median_bar_met: bool,
    pub p95_bar_met: bool,
}

pub fn report(rows: &[CaseTiming]) -> LatencyReport {
    let asked: Vec<&CaseTiming> = rows.iter().filter(|r| r.elapsed_ms > 0).collect();
    let all: Vec<u64> = asked.iter().map(|r| r.elapsed_ms).collect();
    let model_rows: Vec<&&CaseTiming> = asked.iter().filter(|r| r.calls > 0).collect();
    let model: Vec<u64> = model_rows.iter().map(|r| r.elapsed_ms).collect();
    let mut classes: BTreeMap<String, Vec<u64>> = BTreeMap::new();
    for r in &asked {
        if !r.length_class.is_empty() {
            classes
                .entry(r.length_class.clone())
                .or_default()
                .push(r.elapsed_ms);
        }
    }
    let sum = |f: fn(&CaseTiming) -> u64| rows.iter().map(f).sum::<u64>();
    let (prompt, cached) = (
        sum(|r| u64::from(r.prompt_tokens)),
        sum(|r| u64::from(r.cached_tokens)),
    );
    let mut stages: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    let mut add = |name: &str, ms: u64| {
        let e = stages.entry(name.to_string()).or_default();
        e.0 += ms;
        e.1 += 1;
    };
    for r in &asked {
        if r.turn_ms > 0 || r.dossier_ms > 0 {
            add("turn", r.turn_ms);
            add("dossier", r.dossier_ms);
        }
        if r.calls > 0 {
            add("engine_prompt", u64::from(r.prompt_ms));
            add("engine_generation", u64::from(r.generation_ms));
        }
        for (k, v) in &r.stages {
            add(k, *v);
        }
    }
    let overall = spread(&all);
    LatencyReport {
        cases: rows.len(),
        median_bar_met: !all.is_empty() && overall.median_ms <= MEDIAN_BAR_MS,
        p95_bar_met: !all.is_empty() && overall.p95_ms <= P95_BAR_MS,
        all: overall,
        model: spread(&model),
        by_length_class: classes.into_iter().map(|(k, v)| (k, spread(&v))).collect(),
        first_question_ms: asked.first().map(|r| r.elapsed_ms),
        first_model_question_ms: model_rows.first().map(|r| r.elapsed_ms),
        prompt_tokens: prompt,
        cached_tokens: cached,
        completion_tokens: sum(|r| u64::from(r.completion_tokens)),
        calls: sum(|r| u64::from(r.calls)),
        cached_share: if prompt == 0 {
            0.0
        } else {
            cached as f64 / prompt as f64
        },
        stage_mean_ms: stages
            .into_iter()
            .map(|(k, (t, n))| (k, t / n.max(1)))
            .collect(),
    }
}

fn secs(ms: u64) -> String {
    format!("{:.1} s", ms as f64 / 1000.0)
}

/// The report as a section of `report.md`.
pub fn markdown(r: &LatencyReport) -> String {
    let mut s = String::from("\n## Latency (spec section 7.5)\n\n");
    let yes = |b: bool| if b { "met" } else { "not met" };
    s.push_str("| Measure | Value | Bar |\n|---|---|---|\n");
    s.push_str(&format!(
        "| Median, all questions ({} asked) | {} | {} or lower: {} |\n",
        r.all.n,
        secs(r.all.median_ms),
        secs(MEDIAN_BAR_MS),
        yes(r.median_bar_met)
    ));
    s.push_str(&format!(
        "| 95th percentile, all questions | {} | {} or lower: {} |\n",
        secs(r.all.p95_ms),
        secs(P95_BAR_MS),
        yes(r.p95_bar_met)
    ));
    s.push_str(&format!(
        "| Median, questions that used the engine ({}) | {} | |\n| 95th percentile, same | {} | |\n",
        r.model.n,
        secs(r.model.median_ms),
        secs(r.model.p95_ms)
    ));
    let first = |v: Option<u64>| v.map_or("not run".to_string(), secs);
    s.push_str(&format!(
        "| First question after the project opened | {} | |\n| First question that used the engine | {} | |\n",
        first(r.first_question_ms),
        first(r.first_model_question_ms)
    ));
    s.push_str(&format!(
        "| Prompt tokens served from stored work | {:.1}% ({} of {}) | |\n| Completion tokens, engine calls | {}, {} | |\n",
        r.cached_share * 100.0,
        r.cached_tokens,
        r.prompt_tokens,
        r.completion_tokens,
        r.calls
    ));
    if !r.by_length_class.is_empty() {
        s.push_str(
            "\n| Length class | Questions | Median | 95th percentile |\n|---|---|---|---|\n",
        );
        for (k, v) in &r.by_length_class {
            s.push_str(&format!(
                "| {k} | {} | {} | {} |\n",
                v.n,
                secs(v.median_ms),
                secs(v.p95_ms)
            ));
        }
    }
    if !r.stage_mean_ms.is_empty() {
        s.push_str("\n| Stage | Mean per question |\n|---|---|\n");
        for (k, v) in &r.stage_mean_ms {
            s.push_str(&format!("| {k} | {v} ms |\n"));
        }
    }
    s.push_str(
        "\nStage times come from the AI trace and from what the request recorded; a stage nobody \
         records is not listed. Engine prompt and generation are the engine's own figures and \
         include the earlier turns a conversation case asks first, so they add up to more than \
         the time of the last question.\n",
    );
    s
}

/// The rows of a finished run, from its `results.jsonl` (lines written by `ai-eval`). Fields a
/// run did not write read as absent.
pub fn rows_from_results(jsonl: &str) -> Vec<CaseTiming> {
    jsonl
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .map(|v| {
            let n = |p: &str| v.pointer(p).and_then(Value::as_u64).unwrap_or(0);
            CaseTiming {
                id: v["case"].as_str().unwrap_or_default().to_string(),
                length_class: v
                    .pointer("/observation/timing/lengthClass")
                    .or_else(|| v.pointer("/score/turn/length_class"))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                elapsed_ms: n("/observation/elapsedMs"),
                turn_ms: n("/observation/timing/turnMs"),
                dossier_ms: n("/observation/timing/dossierMs"),
                passes: n("/observation/timing/passes") as u32,
                calls: n("/engine/calls") as u32,
                prompt_tokens: n("/engine/promptTokens") as u32,
                cached_tokens: n("/engine/cachedTokens") as u32,
                completion_tokens: n("/engine/completionTokens") as u32,
                prompt_ms: n("/engine/promptMs") as u32,
                generation_ms: n("/engine/generationMs") as u32,
                stages: v
                    .pointer("/observation/timing/stages")
                    .map(stages_from_scope)
                    .unwrap_or_default(),
            }
        })
        .collect()
}

/// The `observation.timing` object a run writes for one case.
pub fn timing_json(t: &CaseTiming) -> Value {
    json!({ "lengthClass": t.length_class, "turnMs": t.turn_ms, "dossierMs": t.dossier_ms,
        "passes": t.passes, "stages": t.stages.iter().map(|(k, v)| (format!("{k}_ms"), json!(v))).collect::<serde_json::Map<_, _>>() })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, class: &str, ms: u64, calls: u32) -> CaseTiming {
        CaseTiming {
            id: id.into(),
            length_class: class.into(),
            elapsed_ms: ms,
            calls,
            prompt_tokens: if calls > 0 { 1000 } else { 0 },
            cached_tokens: if calls > 0 { 600 } else { 0 },
            prompt_ms: if calls > 0 { 400 } else { 0 },
            generation_ms: if calls > 0 { 2000 } else { 0 },
            turn_ms: 30,
            dossier_ms: 10,
            ..CaseTiming::default()
        }
    }

    #[test]
    fn median_and_p95_use_the_middle_and_the_nearest_rank() {
        assert_eq!(spread(&[]), Spread::default());
        assert_eq!(spread(&[7]).median_ms, 7);
        let s = spread(&[4000, 1000, 3000, 2000]);
        assert_eq!((s.n, s.median_ms, s.p95_ms), (4, 2500, 4000));
        let twenty: Vec<u64> = (1..=20).map(|i| i * 100).collect();
        assert_eq!(spread(&twenty).p95_ms, 1900);
    }

    #[test]
    fn the_report_has_the_shape_the_spec_asks_for() {
        let mut rows = vec![
            row("a", "paragraph", 20_000, 2),
            row("b", "one_sentence", 3_000, 1),
            row("c", "one_sentence", 100, 0),
            row("d", "short", 9_000, 1),
        ];
        rows[1].stages.insert("write".into(), 2500);
        let r = report(&rows);
        assert_eq!(r.cases, 4);
        assert_eq!(r.all.n, 4);
        assert_eq!(r.model.n, 3);
        assert_eq!(r.first_question_ms, Some(20_000));
        assert_eq!(r.first_model_question_ms, Some(20_000));
        assert_eq!(r.by_length_class["one_sentence"].n, 2);
        assert_eq!(r.by_length_class["paragraph"].median_ms, 20_000);
        assert!((r.cached_share - 0.6).abs() < 1e-9);
        assert_eq!(r.stage_mean_ms["write"], 2500);
        assert_eq!(r.stage_mean_ms["turn"], 30);
        assert_eq!(r.stage_mean_ms["engine_generation"], 2000);
        assert!(r.median_bar_met && r.p95_bar_met);
        // One question over the bar, among few, is the 95th percentile.
        rows.push(row("e", "short", 31_000, 1));
        assert!(!report(&rows).p95_bar_met);
        let slow: Vec<CaseTiming> = (0..5)
            .map(|i| row(&i.to_string(), "short", 16_000, 1))
            .collect();
        assert!(!report(&slow).median_bar_met);
        let text = markdown(&r);
        for needle in [
            "Median, all questions",
            "95th percentile, all questions",
            "First question after the project opened",
            "Prompt tokens served from stored work",
            "| one_sentence |",
            "| write |",
        ] {
            assert!(text.contains(needle), "{needle}");
        }
        assert!(!text.contains('\u{2014}'));
    }

    #[test]
    fn an_empty_run_reports_no_bars_met_and_does_not_divide_by_zero() {
        let r = report(&[]);
        assert!(!r.median_bar_met && !r.p95_bar_met);
        assert_eq!((r.cached_share, r.first_question_ms), (0.0, None));
        assert!(markdown(&r).contains("not run"));
    }

    #[test]
    fn stage_times_are_read_defensively_from_the_scope() {
        let v = json!({
            "memory": { "recent": 2 },
            "memory_ms": 12,
            "timings": { "write_ms": 3400.7, "check_ms": "slow", "note": 5, "neg_ms": -4 },
            "write_ms": "x"
        });
        let s = stages_from_scope(&v);
        assert_eq!(s.get("memory"), Some(&12));
        assert_eq!(s.get("write"), Some(&3400));
        assert_eq!(s.len(), 2, "{s:?}");
        assert!(stages_from_scope(&json!(null)).is_empty());
        assert!(stages_from_scope(&json!([1, 2])).is_empty());
    }

    #[test]
    fn rows_are_rebuilt_from_a_results_file_and_a_field_that_is_missing_reads_as_absent() {
        let line = json!({
            "case": "x1",
            "engine": { "calls": 2, "promptTokens": 900, "cachedTokens": 450, "completionTokens": 40,
                        "promptMs": 300, "generationMs": 1800 },
            "observation": { "elapsedMs": 5200,
                "timing": { "lengthClass": "short", "turnMs": 20, "dossierMs": 5, "passes": 2,
                            "stages": { "check_ms": 90 } } }
        });
        let old = json!({ "case": "old", "observation": { "elapsedMs": 800 } });
        let rows = rows_from_results(&format!("{line}\nnot json\n{old}\n"));
        assert_eq!(rows.len(), 2);
        assert_eq!(
            (
                rows[0].id.as_str(),
                rows[0].length_class.as_str(),
                rows[0].calls,
                rows[0].passes
            ),
            ("x1", "short", 2, 2)
        );
        assert_eq!(rows[0].stages.get("check"), Some(&90));
        assert_eq!(
            (rows[1].calls, rows[1].cached_tokens, rows[1].elapsed_ms),
            (0, 0, 800)
        );
        // What a run writes is what the rebuild reads.
        let again = rows_from_results(&json!({ "case": "x1", "observation": { "elapsedMs": 5200, "timing": timing_json(&rows[0]) } }).to_string());
        assert_eq!(again[0].stages, rows[0].stages);
        assert_eq!(again[0].length_class, "short");
    }
}
