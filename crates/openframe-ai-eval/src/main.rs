//! `ai-eval`: run the gold cases through the real assistant pipeline and write the report.
//!
//! ```text
//! node scripts/cargo-queued.mjs run --release -p openframe-ai-eval --bin ai-eval -- --gold crates/openframe-ai-eval/gold --out <folder> [--only <text>] [--limit N]
//! ```
//!
//! `--only` keeps the cases whose id contains the text (or whose kind or language equals it), so
//! `--only poison-` runs just `gold/poisoned.jsonl`.
//!
//! `--turn-only` runs just the answer-type stage (spec slice S4) on every non-retrieval case and on
//! the sets of `turn-sets/` (`--turn-sets <folder>` to change it), with no model writing: see
//! `turn_only.rs`. It still needs the models, for the embeddings and the short model call.
//!
//! `--dossier-only` runs the stage after it, the fetch of the facts an answer is written from (slice
//! S5), on every Low Tide conversation case and scores the dossier against what the case expects:
//! see `dossier_only.rs`. It reads the answer type the way `--turn-only` does, so it needs the same
//! models.
//!
//! Needs the Offline AI engine staged by `node scripts/prepare-ai-assets.mjs` (OPENFRAME_AI_BUNDLE_DIR,
//! default `.build-cache/ai-bundle`) and the pinned models in the build inputs (OPENFRAME_AI_ASSETS_DIR,
//! default `.ai-assets`), which it installs in its own app data as a finished download would
//! (ADR-0026). Deterministic sampling (temperature 0, fixed seed). One app
//! core and one warm engine for the whole run; every case gets a fresh project with its sample
//! script, so no case sees another's changes. Writes `results.jsonl` (one line per case, numbers,
//! tools and answers), `summary.json` and `report.md` into the output folder. Development only.

mod dossier_only;
mod turn_only;

use std::io::Write as _;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use openframe_ai::client::build_body;
use openframe_ai::{ChatRequest, InferenceProvider, ProviderCapabilities, Usage};
use openframe_ai_eval::{
    Case, Observation, StateCheck, latency, load_cases, markdown, score, summarize,
};
use openframe_application::core::{AppConfig, AppCore};
use openframe_application::events::RecordingSink;
use openframe_application::modules::ai::{intelligence, orchestrator, retrieval, scope, toolbox};
use openframe_domain::AppResult;
use serde_json::{Value, json};

struct Args {
    gold: PathBuf,
    out: PathBuf,
    only: Option<String>,
    limit: Option<usize>,
    /// `english` or `multilingual`: the search model of the run (default: the app's automatic choice).
    search: Option<String>,
    /// `auto`: the app's own engine placement (the graphics card when its check passes); default
    /// `off`: always the processor, so timings compare between runs.
    graphics: Option<String>,
    /// `on` (default) or `off`: the warm-up the app does when a project opens (index, example
    /// vectors, the engine and the start of each fixed prompt). `off` shows what it saves.
    warmup: Option<String>,
    /// Rebuild the latency report of a finished run folder and print it; runs nothing.
    latency_from: Option<PathBuf>,
    /// Run only the answer-type stage.
    turn_only: bool,
    /// Run the answer-type stage and the fetch stage, and score the fetched facts.
    dossier_only: bool,
    turn_sets: PathBuf,
}

fn parse_args() -> Result<Args, String> {
    let mut a = std::env::args().skip(1);
    let (mut gold, mut out, mut only, mut limit, mut search, mut graphics) =
        (None, None, None, None, None, None);
    let (mut warmup, mut latency_from) = (None, None);
    let mut turn_only = false;
    let mut dossier_only = false;
    let mut turn_sets = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("turn-sets");
    while let Some(k) = a.next() {
        let mut v = || a.next().ok_or_else(|| format!("{k} needs a value"));
        match k.as_str() {
            "--gold" => gold = Some(PathBuf::from(v()?)),
            "--out" => out = Some(PathBuf::from(v()?)),
            "--only" => only = Some(v()?),
            "--limit" => limit = Some(v()?.parse().map_err(|_| "--limit takes a number")?),
            "--search" => search = Some(v()?),
            "--graphics" => graphics = Some(v()?),
            "--warmup" => warmup = Some(v()?),
            "--latency-from" => latency_from = Some(PathBuf::from(v()?)),
            "--turn-only" => turn_only = true,
            "--dossier-only" => dossier_only = true,
            "--turn-sets" => turn_sets = PathBuf::from(v()?),
            _ => return Err(format!("unknown argument {k}")),
        }
    }
    if let Some(dir) = latency_from {
        return Ok(Args {
            gold: PathBuf::new(),
            out: PathBuf::new(),
            only: None,
            limit: None,
            search: None,
            graphics: None,
            warmup: None,
            latency_from: Some(dir),
            turn_only: false,
            dossier_only: false,
            turn_sets,
        });
    }
    Ok(Args {
        gold: gold.ok_or("--gold <folder> is required")?,
        out: out.ok_or("--out <folder> is required")?,
        warmup,
        latency_from: None,
        only,
        limit,
        search,
        graphics,
        turn_only,
        dossier_only,
        turn_sets,
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
    if let Some(dir) = &args.latency_from {
        let text = std::fs::read_to_string(dir.join("results.jsonl")).map_err(|e| e.to_string())?;
        let report = latency::report(&latency::rows_from_results(&text));
        print!("{}", latency::markdown(&report));
        return Ok(());
    }
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
    if args.graphics.as_deref() != Some("auto") {
        std::fs::create_dir_all(app_data.join("ai")).map_err(|e| e.to_string())?;
        std::fs::write(app_data.join("ai").join("graphics.json"), r#"{"off":true}"#)
            .map_err(|e| e.to_string())?;
    }
    let config = AppConfig {
        ai_bundle_dir: Some(bundle.clone()),
        ai_model_source: Some(source),
        ai_deterministic: true,
        ..AppConfig::isolated(data.path())
    };
    let core =
        AppCore::new(config, Arc::new(RecordingSink::default())).map_err(|e| format!("{e:?}"))?;
    let choice = match args.search.as_deref() {
        None | Some("auto") => intelligence::SearchChoice::Automatic,
        Some("english") => intelligence::SearchChoice::English,
        Some("multilingual") => intelligence::SearchChoice::Multilingual,
        Some(other) => {
            return Err(format!(
                "--search takes english, multilingual or auto, not {other}"
            ));
        }
    };
    intelligence::service(&core).set_search_choice(choice);
    let svc = openframe_application::modules::ai::service(&core);
    if args.turn_only {
        turn_only::run(&core, &cases, &args.turn_sets, &args.out, svc.model())?;
        core.shutdown();
        return Ok(());
    }
    if args.dossier_only {
        dossier_only::run(&core, &cases, &args.out, svc.model())?;
        core.shutdown();
        return Ok(());
    }
    let recorder = Arc::new(Recorder {
        inner: svc
            .model()
            .ok_or("the Offline AI is not available in this run")?,
        bodies: Mutex::new(Vec::new()),
        usage: Mutex::new(Usage::default()),
        chat_ms: Mutex::new(0),
    });
    svc.attach_model(Some(recorder.clone()));
    let warmup_on = match args.warmup.as_deref() {
        None | Some("on") => true,
        Some("off") => false,
        Some(other) => return Err(format!("--warmup takes on or off, not {other}")),
    };
    // A deterministic run does not warm up unless told to; the evaluation measures what people meet.
    intelligence::warm::set_forced(warmup_on);
    let mut results =
        std::fs::File::create(args.out.join("results.jsonl")).map_err(|e| e.to_string())?;
    let mut requests =
        std::fs::File::create(args.out.join("requests.jsonl")).map_err(|e| e.to_string())?;
    let started = Instant::now();
    let mut scores = Vec::new();
    let mut timings = Vec::new();
    for (i, case) in cases.iter().enumerate() {
        recorder.take();
        let mut o = run_case(&core, case);
        let (bodies, u, chat_ms) = recorder.take();
        // The stage times of the request plus the engine figures of the case, for the latency report.
        let mut t = o.timing.take().unwrap_or_default();
        t.id = case.id.clone();
        t.elapsed_ms = o.elapsed_ms;
        (t.calls, t.prompt_tokens, t.cached_tokens) = (u.calls, u.prompt_tokens, u.cached_tokens);
        (t.completion_tokens, t.prompt_ms, t.generation_ms) =
            (u.completion_tokens, u.prompt_ms, u.generation_ms);
        if case.kind != "retrieval" {
            timings.push(t.clone());
        }
        o.timing = Some(t);
        for (step, body) in bodies.iter().enumerate() {
            let line = json!({ "case": case.id, "kind": case.kind, "lang": case.lang, "step": step, "body": body });
            writeln!(requests, "{line}").map_err(|e| e.to_string())?;
        }
        let engine = json!({ "calls": u.calls, "promptTokens": u.prompt_tokens,
            "completionTokens": u.completion_tokens, "maxPromptTokens": u.max_prompt_tokens,
            "cachedTokens": u.cached_tokens, "promptMs": u.prompt_ms, "generationMs": u.generation_ms, "chatWallMs": chat_ms });
        let index = intelligence::service(&core).current().map(|ix| {
            let snap = ix.snapshot();
            json!({ "semanticReady": snap.semantic_ready, "model": snap.embedding_model })
        });
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
        let line = json!({ "case": case.id, "kind": case.kind, "lang": case.lang, "score": s, "route": o.route, "observation": {
            "offered": o.offered.len(), "steps": o.steps, "errorCode": o.error_code, "answer": o.answer,
            "proposed": o.proposed, "applied": o.applied, "endState": o.end_state, "rankedScenes": o.ranked_scenes, "sceneCount": o.scene_count,
            "elapsedMs": o.elapsed_ms, "engineMb": o.engine_mb, "setupError": o.setup_error,
            "timing": o.timing.as_ref().map(latency::timing_json) },
            "engine": engine, "index": index });
        writeln!(results, "{line}").map_err(|e| e.to_string())?;
        scores.push(s);
    }
    // Where the chat engine actually ran (the placement the app chose for this run).
    let placement = svc
        .manager()
        .map(|m| m.diagnostics().placement)
        .unwrap_or_else(|_| "unknown".into());
    eprintln!("ai-eval: engine place: {placement}");
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
        ("Search model".to_string(), format!("{choice:?}")),
        (
            "Engine place".to_string(),
            args.graphics
                .clone()
                .unwrap_or_else(|| "off (processor)".into()),
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
    let latency_report = latency::report(&timings);
    std::fs::write(
        args.out.join("latency.json"),
        serde_json::to_string_pretty(&latency_report).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    let mut report = markdown("AI evaluation report", &summary, &meta);
    report.push_str(&latency::markdown(&latency_report));
    std::fs::write(args.out.join("report.md"), report).map_err(|e| e.to_string())?;
    core.shutdown();
    eprintln!(
        "ai-eval: {} cases, report in {}",
        cases.len(),
        args.out.display()
    );
    Ok(())
}

/// The real engine, with every request body and the engine numbers of each case recorded for the
/// measurements (timings per case, and the exact prompts the engine experiments replay). It only
/// passes calls through; answers and the AI trace are unchanged.
struct Recorder {
    inner: Arc<dyn InferenceProvider>,
    bodies: Mutex<Vec<Value>>,
    usage: Mutex<Usage>,
    chat_ms: Mutex<u64>,
}

impl Recorder {
    /// The request bodies, the summed engine numbers and the wall time of the chat calls since
    /// the last case.
    fn take(&self) -> (Vec<Value>, Usage, u64) {
        (
            std::mem::take(&mut *self.bodies.lock().unwrap()),
            std::mem::take(&mut *self.usage.lock().unwrap()),
            std::mem::take(&mut *self.chat_ms.lock().unwrap()),
        )
    }
}

impl InferenceProvider for Recorder {
    fn chat(&self, req: &ChatRequest) -> AppResult<String> {
        self.bodies.lock().unwrap().push(build_body(req));
        let t = Instant::now();
        let r = self.inner.chat(req);
        *self.chat_ms.lock().unwrap() += t.elapsed().as_millis() as u64;
        r
    }
    fn model_reference(&self) -> String {
        self.inner.model_reference()
    }
    fn capabilities(&self) -> ProviderCapabilities {
        self.inner.capabilities()
    }
    fn take_usage(&self) -> Option<Usage> {
        let u = self.inner.take_usage();
        if let Some(u) = u {
            self.usage.lock().unwrap().add(u);
        }
        u
    }
}

/// Call one operation as the local person does.
fn op(core: &AppCore, name: &str, args: Value) -> Result<Value, String> {
    core.dispatch(&core.local_actor(), name, args)
        .map_err(|e| format!("{name}: {} {}", e.code_str(), e.message))
}

#[derive(Clone)]
struct Project {
    draft_id: String,
    scene_ids: Vec<String>,
    element_scene: std::collections::HashMap<String, usize>,
    text: String,
}

thread_local! {
    /// The project shared by retrieval cases of one sample script (they only read it).
    static SHARED: std::cell::RefCell<Option<(String, Project)>> = const { std::cell::RefCell::new(None) };
}

/// Retrieval and conversation cases without setup steps never change the project, so they share
/// one indexed copy of their sample or fixture instead of indexing it again for every case.
fn shared_project(core: &AppCore, case: &Case) -> Result<Project, String> {
    let cached = SHARED.with(|s| {
        s.borrow()
            .as_ref()
            .filter(|(sample, _)| *sample == case.project)
            .map(|(_, p)| p.clone())
    });
    if let Some(p) = cached.filter(|_| core.project_opt().is_some()) {
        return Ok(p);
    }
    let p = open_project(core, case)?;
    SHARED.with(|s| *s.borrow_mut() = Some((case.project.clone(), p.clone())));
    Ok(p)
}

/// A fresh project holding the case's sample script or fixture, with its index caught up.
fn open_project(core: &AppCore, case: &Case) -> Result<Project, String> {
    SHARED.with(|s| *s.borrow_mut() = None);
    if core.project_opt().is_some() {
        op(core, "project.close", json!({}))?;
    }
    op(
        core,
        "project.create",
        json!({ "title": case.id, "projectType": "Feature Film" }),
    )?;
    let draft_id = if openframe_ai_eval::fixture::is_fixture(&case.project) {
        // A fixture project (gold/fixtures): its screenplay and production data are built here.
        openframe_ai_eval::fixture::build(core, &case.project)?.draft_id
    } else {
        op(
            core,
            "screenplay.import_apply",
            json!({ "sample": case.project, "mode": "new_screenplay" }),
        )?;
        let ov = op(core, "screenplay.overview", json!({}))?;
        ov["drafts"]
            .as_array()
            .and_then(|d| d.iter().find(|d| d["isCurrent"] == true))
            .and_then(|d| d["id"].as_str())
            .ok_or("the imported screenplay has no current draft")?
            .to_string()
    };
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
    // A person opens the project and then types; the warm-up has that time. Without it the first
    // question would race the warm-up that the eval's own setup leaves no pause for.
    let waited = Instant::now();
    if !intelligence::warm::wait_idle(Duration::from_secs(180)) {
        eprintln!("ai-eval: the warm-up was still running after 180 s");
    } else if waited.elapsed() > Duration::from_millis(50) {
        let state = openframe_application::modules::ai::service(core)
            .manager()
            .map(|m| format!("{:?}", m.diagnostics().chat_runtime.state))
            .unwrap_or_default();
        eprintln!(
            "ai-eval: warm-up finished {} ms after the index (engine {state})",
            waited.elapsed().as_millis()
        );
    }
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

/// The approved gold cases are frozen (a change is a new review), so a setup written in a
/// shorthand the operation does not take is translated here: `vault.create` with `type` and
/// `text` becomes `itemType` and `body`.
fn setup_args(op: &str, mut args: Value) -> Value {
    if op == "vault.create"
        && let Some(o) = args.as_object_mut()
    {
        for (from, to) in [("type", "itemType"), ("text", "body")] {
            if let Some(v) = o.remove(from) {
                o.entry(to).or_insert(v);
            }
        }
    }
    args
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
    // Conversation cases only read too, so a 676-step fixture is built and indexed once per run.
    let opened =
        if matches!(case.kind.as_str(), "retrieval" | "conversation") && case.setup.is_empty() {
            shared_project(core, case)
        } else {
            open_project(core, case)
        };
    let p = match opened {
        Ok(p) => p,
        Err(e) => {
            o.setup_error = Some(e);
            return o;
        }
    };
    o.source_text = p.text.clone();
    o.scene_count = p.scene_ids.len();
    // Files a case writes with `harness.write_file` live in this folder until the case ends.
    let folder = match tempfile::tempdir() {
        Ok(d) => d,
        Err(e) => {
            o.setup_error = Some(format!("setup {e}"));
            return o;
        }
    };
    let mut files = std::collections::BTreeMap::new();
    let other = openframe_ai_eval::other_actor();
    for s in &case.setup {
        let args = openframe_ai_eval::substitute_files(&substitute(&s.args, &p), &files);
        let done = if s.op == "harness.write_file" {
            let field = |k: &str| args[k].as_str().unwrap_or_default().to_string();
            let name = field("name");
            openframe_ai_eval::write_case_file(folder.path(), &name, &field("text"))
                .map(|path| drop(files.insert(name, path)))
        } else if s.as_user.as_deref() == Some("other") {
            core.dispatch(&other, &s.op, setup_args(&s.op, args))
                .map(drop)
                .map_err(|e| format!("{}: {} {}", s.op, e.code_str(), e.message))
        } else {
            op(core, &s.op, setup_args(&s.op, args)).map(drop)
        };
        if let Err(e) = done {
            o.setup_error = Some(format!("setup {e}"));
            return o;
        }
    }
    if case.setup.iter().any(|s| {
        [
            "notes.",
            "vault.",
            "private_note.",
            "files.",
            "screenplay.import",
        ]
        .iter()
        .any(|p| s.op.starts_with(p))
    }) {
        intelligence::wait_idle(core, Duration::from_secs(60));
    }
    let actor = core.local_actor();
    if case.kind == "retrieval" {
        // Semantic search needs the project's vectors; without this wait a case would measure
        // word matching alone.
        let until = Instant::now() + Duration::from_secs(600);
        while let Some(ix) = intelligence::service(core).current() {
            if ix.snapshot().semantic_ready || Instant::now() > until {
                break;
            }
            std::thread::sleep(Duration::from_millis(250));
        }
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
    // Earlier requests of a conversation case go first in the same conversation; their answers
    // are ignored, they only become the history the real request is asked against.
    let conversation = (!case.before.is_empty()).then(|| format!("eval-{}", case.id));
    for earlier in &case.before {
        let _ = op(
            core,
            "ai.ask",
            json!({ "text": earlier, "scope": scope_json, "conversationId": conversation }),
        );
    }
    let t = Instant::now();
    let mut ask = json!({ "text": case.request, "scope": scope_json });
    if let Some(id) = &conversation {
        ask["conversationId"] = json!(id);
    }
    let asked = op(core, "ai.ask", ask);
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
    // The lane and intent numbers of this request, from its trace row.
    if let Some(request_id) = x["requestId"].as_str() {
        o.route = core
            .with_app_db(|c| Ok(openframe_ai_eval::route_from_trace(c, request_id)))
            .ok()
            .flatten();
    }
    // What the answer-type stage read for this request, from its trace row.
    if let Some(request_id) = x["requestId"].as_str() {
        o.turn = core
            .with_app_db(|c| Ok(openframe_ai_eval::turn_from_trace(c, request_id)))
            .ok()
            .flatten();
        // Stage times: the trace row, and the `<stage>_ms` numbers the request recorded.
        let times = core
            .with_app_db(|c| Ok(latency::times_from_trace(c, request_id)))
            .ok()
            .flatten()
            .unwrap_or_default();
        let stages = core
            .project()
            .ok()
            .and_then(|s| {
                s.store
                    .read(|c| {
                        Ok(c.query_row(
                            "SELECT scope_json FROM ai_request WHERE id = ?1",
                            [request_id],
                            |r| r.get::<_, String>(0),
                        )
                        .ok())
                    })
                    .ok()
                    .flatten()
            })
            .and_then(|j| serde_json::from_str::<Value>(&j).ok())
            .map(|v| latency::stages_from_scope(&v))
            .unwrap_or_default();
        o.timing = Some(latency::CaseTiming {
            length_class: times.length_class,
            turn_ms: times.turn_ms,
            dossier_ms: times.dossier_ms,
            passes: times.passes,
            stages,
            ..latency::CaseTiming::default()
        });
    }
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
