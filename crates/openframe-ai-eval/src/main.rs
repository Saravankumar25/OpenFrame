//! `ai-eval`: run the gold cases through the real assistant pipeline and write the report.
//!
//! ```text
//! node scripts/cargo-queued.mjs run --release -p openframe-ai-eval -- --gold crates/openframe-ai-eval/gold --out <folder> [--only <text>] [--limit N]
//! ```
//!
//! Needs the Offline AI engine staged by `node scripts/prepare-ai-assets.mjs` (OPENFRAME_AI_BUNDLE_DIR,
//! default `.build-cache/ai-bundle`) and the pinned models in the build inputs (OPENFRAME_AI_ASSETS_DIR,
//! default `.ai-assets`), which it installs in its own app data as a finished download would
//! (ADR-0026). Deterministic sampling (temperature 0, fixed seed). One app
//! core and one warm engine for the whole run; every case gets a fresh project with its sample
//! script, so no case sees another's changes. Writes `results.jsonl` (one line per case, numbers,
//! tools and answers), `summary.json` and `report.md` into the output folder. Development only.

use std::io::Write as _;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use openframe_ai_eval::{Case, Observation, StateCheck, load_cases, markdown, score, summarize};
use openframe_application::core::{AppConfig, AppCore};
use openframe_application::events::RecordingSink;
use openframe_application::modules::ai::{intelligence, orchestrator, retrieval, scope, toolbox};
use serde_json::{Value, json};

struct Args {
    gold: PathBuf,
    out: PathBuf,
    only: Option<String>,
    limit: Option<usize>,
}

fn parse_args() -> Result<Args, String> {
    let mut a = std::env::args().skip(1);
    let (mut gold, mut out, mut only, mut limit) = (None, None, None, None);
    while let Some(k) = a.next() {
        let mut v = || a.next().ok_or_else(|| format!("{k} needs a value"));
        match k.as_str() {
            "--gold" => gold = Some(PathBuf::from(v()?)),
            "--out" => out = Some(PathBuf::from(v()?)),
            "--only" => only = Some(v()?),
            "--limit" => limit = Some(v()?.parse().map_err(|_| "--limit takes a number")?),
            _ => return Err(format!("unknown argument {k}")),
        }
    }
    Ok(Args {
        gold: gold.ok_or("--gold <folder> is required")?,
        out: out.ok_or("--out <folder> is required")?,
        only,
        limit,
    })
}

fn main() {
    if let Err(e) = run() {
        eprintln!("ai-eval: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let mut cases = load_cases(&args.gold)?;
    if let Some(only) = &args.only {
        cases.retain(|c| c.id.contains(only.as_str()) || c.kind == *only || c.lang == *only);
    }
    if let Some(n) = args.limit {
        cases.truncate(n);
    }
    let bundle = std::env::var_os("OPENFRAME_AI_BUNDLE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.build-cache/ai-bundle")
        });
    if !bundle.join("runtimes").exists() {
        return Err(format!(
            "no Offline AI engine in {}; run node scripts/prepare-ai-assets.mjs",
            bundle.display()
        ));
    }
    // The models the compiled lock pins, installed in this run's app data as a finished download
    // leaves them (from OPENFRAME_AI_ASSETS_DIR or the checkout's .ai-assets).
    let assets = std::env::var_os("OPENFRAME_AI_ASSETS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.ai-assets"));
    std::fs::create_dir_all(&args.out).map_err(|e| e.to_string())?;
    let data = tempfile::tempdir().map_err(|e| e.to_string())?;
    let app_data = AppConfig::isolated(data.path()).app_data_dir;
    let lock = openframe_ai::AssetLock::embedded().map_err(|e| format!("{e:?}"))?;
    let source = openframe_ai::test_support::install_pinned_models(&app_data, &assets, &lock)
        .map_err(|e| format!("installing the models: {e:?}"))?;
    // The same place every run: the processor (a graphics card check would change the timings
    // between runs; ADR-0029 has its own benchmark).
    std::fs::create_dir_all(app_data.join("ai")).map_err(|e| e.to_string())?;
    std::fs::write(app_data.join("ai").join("graphics.json"), r#"{"off":true}"#)
        .map_err(|e| e.to_string())?;
    let config = AppConfig {
        ai_bundle_dir: Some(bundle.clone()),
        ai_model_source: Some(source),
        ai_deterministic: true,
        ..AppConfig::isolated(data.path())
    };
    let core =
        AppCore::new(config, Arc::new(RecordingSink::default())).map_err(|e| format!("{e:?}"))?;
    let mut results =
        std::fs::File::create(args.out.join("results.jsonl")).map_err(|e| e.to_string())?;
    let started = Instant::now();
    let mut scores = Vec::new();
    for (i, case) in cases.iter().enumerate() {
        let o = run_case(&core, case);
        let s = score(case, &o);
        eprintln!(
            "[{}/{}] {} {}ms{}",
            i + 1,
            cases.len(),
            case.id,
            o.elapsed_ms,
            o.setup_error
                .as_deref()
                .map(|e| format!(" SETUP ERROR: {e}"))
                .unwrap_or_default()
        );
        let line = json!({ "case": case.id, "kind": case.kind, "lang": case.lang, "score": s, "observation": {
            "offered": o.offered.len(), "steps": o.steps, "errorCode": o.error_code, "answer": o.answer,
            "proposed": o.proposed, "applied": o.applied, "endState": o.end_state, "rankedScenes": o.ranked_scenes,
            "elapsedMs": o.elapsed_ms, "engineMb": o.engine_mb, "setupError": o.setup_error } });
        writeln!(results, "{line}").map_err(|e| e.to_string())?;
        scores.push(s);
    }
    let summary = summarize(&scores);
    std::fs::write(
        args.out.join("summary.json"),
        serde_json::to_string_pretty(&summary).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    let meta = vec![
        ("Cases".to_string(), cases.len().to_string()),
        ("Gold folder".to_string(), args.gold.display().to_string()),
        (
            "Model files".to_string(),
            "the pinned Offline AI of packaging/ai/ai-assets.lock.json".to_string(),
        ),
        (
            "Build".to_string(),
            if cfg!(debug_assertions) {
                "debug".into()
            } else {
                "release".into()
            },
        ),
        (
            "Sampling".to_string(),
            "deterministic (temperature 0, fixed seed)".to_string(),
        ),
        (
            "Wall time".to_string(),
            format!("{:.1} min", started.elapsed().as_secs_f64() / 60.0),
        ),
    ];
    std::fs::write(
        args.out.join("report.md"),
        markdown("AI evaluation report", &summary, &meta),
    )
    .map_err(|e| e.to_string())?;
    core.shutdown();
    eprintln!(
        "ai-eval: {} cases, report in {}",
        cases.len(),
        args.out.display()
    );
    Ok(())
}

/// Call one operation as the local person does.
fn op(core: &AppCore, name: &str, args: Value) -> Result<Value, String> {
    core.dispatch(&core.local_actor(), name, args)
        .map_err(|e| format!("{name}: {} {}", e.code_str(), e.message))
}

struct Project {
    draft_id: String,
    scene_ids: Vec<String>,
    element_scene: std::collections::HashMap<String, usize>,
    text: String,
}

/// A fresh project holding the case's sample script, with its index caught up.
fn open_project(core: &AppCore, case: &Case) -> Result<Project, String> {
    if core.project_opt().is_some() {
        op(core, "project.close", json!({}))?;
    }
    op(
        core,
        "project.create",
        json!({ "title": case.id, "projectType": "Feature Film" }),
    )?;
    op(
        core,
        "screenplay.import_apply",
        json!({ "sample": case.project, "mode": "new_screenplay" }),
    )?;
    let ov = op(core, "screenplay.overview", json!({}))?;
    let draft_id = ov["drafts"]
        .as_array()
        .and_then(|d| d.iter().find(|d| d["isCurrent"] == true))
        .and_then(|d| d["id"].as_str())
        .ok_or("the imported screenplay has no current draft")?
        .to_string();
    let doc = op(core, "screenplay.document", json!({ "draftId": draft_id }))?;
    let mut scene_ids = Vec::new();
    let mut element_scene = std::collections::HashMap::new();
    let mut text = String::new();
    for (i, s) in doc["scenes"].as_array().into_iter().flatten().enumerate() {
        let id = s["id"].as_str().unwrap_or_default().to_string();
        text.push_str(s["heading"].as_str().unwrap_or_default());
        text.push('\n');
        for e in s["elements"].as_array().into_iter().flatten() {
            element_scene.insert(e["id"].as_str().unwrap_or_default().to_string(), i + 1);
            text.push_str(e["text"].as_str().unwrap_or_default());
            text.push('\n');
        }
        scene_ids.push(id);
    }
    intelligence::activate(core);
    intelligence::wait_idle(core, Duration::from_secs(180));
    Ok(Project {
        draft_id,
        scene_ids,
        element_scene,
        text,
    })
}

/// `$scene:N` and `$current_draft` in gold arguments become the ids of this project.
fn substitute(v: &Value, p: &Project) -> Value {
    match v {
        Value::String(s) if s == "$current_draft" => json!(p.draft_id),
        Value::String(s) if s.starts_with("$scene:") => {
            let n: usize = s[7..].parse().unwrap_or(0);
            json!(
                p.scene_ids
                    .get(n.wrapping_sub(1))
                    .cloned()
                    .unwrap_or_default()
            )
        }
        Value::Object(m) => Value::Object(
            m.iter()
                .map(|(k, x)| (k.clone(), substitute(x, p)))
                .collect(),
        ),
        Value::Array(a) => Value::Array(a.iter().map(|x| substitute(x, p)).collect()),
        _ => v.clone(),
    }
}

fn state_holds(core: &AppCore, check: &StateCheck, p: &Project) -> bool {
    let args = if check.args.is_null() {
        json!({})
    } else {
        substitute(&check.args, p)
    };
    let Ok(v) = op(core, &check.op, args) else {
        return false;
    };
    let text = v.to_string().to_lowercase();
    check
        .has
        .as_ref()
        .is_none_or(|h| text.contains(&h.to_lowercase()))
        && check
            .lacks
            .as_ref()
            .is_none_or(|l| !text.contains(&l.to_lowercase()))
}

fn scope_args(case: &Case, p: &Project) -> Value {
    match case.scope.as_ref().and_then(|s| s["kind"].as_str()) {
        Some("scene") => {
            let n = case
                .scope
                .as_ref()
                .and_then(|s| s["scene"].as_u64())
                .unwrap_or(1) as usize;
            json!({ "kind": "CurrentScene", "draftId": p.draft_id, "sceneId": p.scene_ids.get(n.saturating_sub(1)).cloned().unwrap_or_default() })
        }
        // There is no act scope in the panel: an act request is asked of the whole project.
        _ => json!({ "kind": "WholeProject" }),
    }
}

fn engine_mb() -> u64 {
    let mut sys = sysinfo::System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    sys.processes()
        .values()
        .filter(|p| {
            p.name()
                .to_string_lossy()
                .to_lowercase()
                .starts_with("llama-server")
        })
        .map(|p| p.memory() / 1_048_576)
        .sum()
}

fn run_case(core: &AppCore, case: &Case) -> Observation {
    let mut o = Observation::default();
    let p = match open_project(core, case) {
        Ok(p) => p,
        Err(e) => {
            o.setup_error = Some(e);
            return o;
        }
    };
    o.source_text = p.text.clone();
    for s in &case.setup {
        if let Err(e) = op(core, &s.op, substitute(&s.args, &p)) {
            o.setup_error = Some(format!("setup {e}"));
            return o;
        }
    }
    if case
        .setup
        .iter()
        .any(|s| s.op.starts_with("notes.") || s.op.starts_with("vault."))
    {
        intelligence::wait_idle(core, Duration::from_secs(60));
    }
    let actor = core.local_actor();
    if case.kind == "retrieval" {
        let t = Instant::now();
        let req = retrieval::RetrieveRequest {
            query: case.request.clone(),
            scope: None,
            limit: 10,
            route: None,
        };
        match retrieval::retrieve(core, &actor, &req) {
            Ok(packet) => {
                for src in &packet.sources {
                    let n = match src.entity_type.as_str() {
                        "screenplay_scene" => p
                            .scene_ids
                            .iter()
                            .position(|id| *id == src.entity_id)
                            .map(|i| i + 1),
                        _ => p.element_scene.get(&src.entity_id).copied(),
                    };
                    if let Some(n) = n.filter(|n| !o.ranked_scenes.contains(n)) {
                        o.ranked_scenes.push(n);
                    }
                }
            }
            Err(e) => o.error_code = Some(e.code_str().to_string()),
        }
        o.elapsed_ms = t.elapsed().as_millis() as u64;
        return o;
    }
    let scope_json = scope_args(case, &p);
    if let Ok(session) = core.project() {
        let parsed: Result<scope::AiScopeArgs, _> = serde_json::from_value(scope_json.clone());
        if let Ok(parsed) = parsed
            && let Ok(resolved) = session.store.read(|c| scope::resolve(c, &actor, &parsed))
        {
            o.offered = toolbox::tools_for_request(
                core,
                &actor,
                &resolved,
                &case.request,
                orchestrator::TOOL_CATALOG_CHARS,
            )
            .into_iter()
            .map(|t| t.name)
            .collect();
        }
    }
    let t = Instant::now();
    let asked = op(
        core,
        "ai.ask",
        json!({ "text": case.request, "scope": scope_json }),
    );
    o.elapsed_ms = t.elapsed().as_millis() as u64;
    o.engine_mb = engine_mb();
    let x = match asked {
        Ok(x) => x,
        Err(e) => {
            o.error_code = Some(e);
            return o;
        }
    };
    let r = &x["result"];
    o.error_code = r["errorCode"].as_str().map(str::to_string);
    let mut answer = r["content"].as_str().unwrap_or_default().to_string();
    for d in r["details"].as_array().into_iter().flatten() {
        answer.push('\n');
        answer.push_str(d.as_str().unwrap_or_default());
    }
    for it in r["items"].as_array().into_iter().flatten() {
        answer.push('\n');
        answer.push_str(it["label"].as_str().unwrap_or_default());
    }
    o.answer = answer;
    // The tools the assistant ran and their arguments, as recorded for this request.
    if let (Ok(session), Some(request_id)) = (core.project(), x["requestId"].as_str()) {
        o.steps = session
            .store
            .read(|c| {
                let mut st = c.prepare("SELECT tool_name, parameters_json FROM ai_tool_invocation WHERE request_id = ?1 ORDER BY step_index")?;
                let rows = st.query_map([request_id], |row| {
                    let tool: String = row.get(0)?;
                    let params: Option<String> = row.get(1)?;
                    Ok((tool, params.and_then(|p| serde_json::from_str(&p).ok()).unwrap_or(Value::Null)))
                })?;
                Ok(rows.filter_map(Result::ok).collect::<Vec<_>>())
            })
            .unwrap_or_default();
    }
    if o.steps.is_empty() {
        o.steps = r["steps"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|s| s["tool"].as_str().map(|t| (t.to_string(), Value::Null)))
            .collect();
    }
    let change_set = r["changeSet"]["id"].as_str().map(str::to_string);
    o.proposed = change_set.is_some();
    // Only action cases are applied, the way a person approves a proposal; safety cases never are.
    if case.kind == "action"
        && let Some(id) = &change_set
    {
        o.applied = op(
            core,
            "ai.change_set.accept",
            json!({ "id": id, "confirmDestructive": true }),
        )
        .is_ok();
    }
    o.end_state = case
        .expect
        .end_state
        .iter()
        .map(|c| state_holds(core, c, &p))
        .collect();
    o
}
