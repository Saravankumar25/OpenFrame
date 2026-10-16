//! `engine-bench`: replay the real assistant requests that `ai-eval` recorded (`requests.jsonl`)
//! straight against the bundled `llama-server`, one setting changed at a time, and report the
//! engine's own timing fields as medians and ranges. Development only; the app never runs it.
//!
//! ```text
//! node scripts/cargo-queued.mjs run --release -p openframe-ai-eval --bin engine-bench -- \
//!     --requests <requests.jsonl> --exp <threads|card|reuse|schema|quant> --out <file.json>
//!     [--repeats 5] [--cases 8] [--threads 2,4,6,8,10,12,14] [--t 8] [--model <file.gguf>]
//! ```
//!
//! - `threads`: the processor engine at each thread count; fresh prompts (no cache).
//! - `card`: the processor (`--t`) against the Vulkan engine with every layer on `Vulkan0`.
//! - `reuse`: each recorded case replayed step by step, with the prompt cache on and off.
//! - `schema`: the same prompts with and without the JSON schema (grammar) constraint.
//! - `quant`: the processor engine (`--t`) with `--model` against the pinned chat model.
//!
//! Every run starts its own engine on 127.0.0.1, warms it with one request, then measures.
//! Engine and model files come from OPENFRAME_AI_BUNDLE_DIR (default `.build-cache/ai-bundle`)
//! and OPENFRAME_AI_ASSETS_DIR (default `.ai-assets`).

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const PORT: u16 = 47_931;

struct Args {
    requests: PathBuf,
    exp: String,
    out: PathBuf,
    repeats: usize,
    cases: usize,
    threads: Vec<usize>,
    t: usize,
    model: Option<PathBuf>,
}

fn parse_args() -> Result<Args, String> {
    let mut a = std::env::args().skip(1);
    let mut r = Args {
        requests: PathBuf::new(),
        exp: String::new(),
        out: PathBuf::new(),
        repeats: 5,
        cases: 8,
        threads: vec![2, 4, 6, 8, 10, 12, 14],
        t: 8,
        model: None,
    };
    while let Some(k) = a.next() {
        let mut v = || a.next().ok_or_else(|| format!("{k} needs a value"));
        match k.as_str() {
            "--requests" => r.requests = PathBuf::from(v()?),
            "--exp" => r.exp = v()?,
            "--out" => r.out = PathBuf::from(v()?),
            "--repeats" => r.repeats = v()?.parse().map_err(|_| "--repeats takes a number")?,
            "--cases" => r.cases = v()?.parse().map_err(|_| "--cases takes a number")?,
            "--t" => r.t = v()?.parse().map_err(|_| "--t takes a number")?,
            "--model" => r.model = Some(PathBuf::from(v()?)),
            "--threads" => {
                r.threads = v()?
                    .split(',')
                    .map(|s| s.trim().parse().map_err(|_| "--threads takes numbers"))
                    .collect::<Result<_, _>>()?
            }
            _ => return Err(format!("unknown argument {k}")),
        }
    }
    if r.requests.as_os_str().is_empty() || r.exp.is_empty() || r.out.as_os_str().is_empty() {
        return Err("--requests, --exp and --out are required".into());
    }
    Ok(r)
}

fn main() {
    if let Err(e) = run() {
        eprintln!("engine-bench: {e}");
        std::process::exit(1);
    }
}

// ------------------------------------------------------------------ engine

struct Engine {
    child: Child,
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The processor engine, or the Vulkan engine when `vulkan`.
fn server_exe(vulkan: bool) -> Result<PathBuf, String> {
    let bundle = std::env::var_os("OPENFRAME_AI_BUNDLE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root().join(".build-cache/ai-bundle"));
    let dirs = std::fs::read_dir(bundle.join("runtimes")).map_err(|e| e.to_string())?;
    for d in dirs.flatten() {
        let name = d.file_name().to_string_lossy().to_lowercase();
        if name.contains("vulkan") == vulkan {
            let exe = d.path().join("llama-server.exe");
            if exe.is_file() {
                return Ok(exe);
            }
        }
    }
    Err(format!(
        "no {} engine in {}",
        if vulkan { "Vulkan" } else { "processor" },
        bundle.display()
    ))
}

fn pinned_chat_model() -> Result<PathBuf, String> {
    let assets = std::env::var_os("OPENFRAME_AI_ASSETS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root().join(".ai-assets"));
    let lock = openframe_ai::AssetLock::embedded().map_err(|e| format!("{e:?}"))?;
    let m = lock.chat_model();
    Ok(assets.join("models").join(&m.model_id).join(&m.file.path))
}

/// Start an engine like the app does (context 8192, one slot, fixed seed) and wait until it
/// answers `/health`.
fn start(exe: &Path, model: &Path, threads: usize, device: Option<&str>) -> Result<Engine, String> {
    let mut cmd = Command::new(exe);
    cmd.args(["-m"]).arg(model).args([
        "--host",
        "127.0.0.1",
        "--port",
        &PORT.to_string(),
        "-c",
        "8192",
        "-t",
        &threads.to_string(),
        "--parallel",
        "1",
        "--no-webui",
        "--offline",
        "--reasoning",
        "off",
        "--alias",
        "openframe-local",
        "--seed",
        "42",
    ]);
    match device {
        Some(d) => cmd.args(["-ngl", "999", "--device", d]),
        None => cmd.args(["-ngl", "0"]),
    };
    let child = cmd
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("starting {}: {e}", exe.display()))?;
    let engine = Engine { child };
    let until = Instant::now() + Duration::from_secs(180);
    while Instant::now() < until {
        if let Ok((200, _)) = http("GET", "/health", None) {
            return Ok(engine);
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    Err("the engine did not become ready in 180 s".into())
}

/// One HTTP/1.1 request to the engine; returns the status and the parsed body.
fn http(method: &str, path: &str, body: Option<&Value>) -> Result<(u16, Value), String> {
    let mut s = TcpStream::connect(("127.0.0.1", PORT)).map_err(|e| e.to_string())?;
    s.set_read_timeout(Some(Duration::from_secs(600))).ok();
    let payload = body.map(Value::to_string).unwrap_or_default();
    write!(
        s,
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
        payload.len()
    )
    .map_err(|e| e.to_string())?;
    let mut r = BufReader::new(s);
    let mut status = String::new();
    r.read_line(&mut status).map_err(|e| e.to_string())?;
    let code = status
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    let mut line = String::new();
    loop {
        line.clear();
        if r.read_line(&mut line).map_err(|e| e.to_string())? == 0 || line == "\r\n" {
            break;
        }
    }
    let mut rest = Vec::new();
    r.read_to_end(&mut rest).map_err(|e| e.to_string())?;
    Ok((code, serde_json::from_slice(&rest).unwrap_or(Value::Null)))
}

/// The engine's own numbers for one request.
#[derive(Clone, Copy, Default)]
struct Timing {
    prompt_n: f64,
    cache_n: f64,
    prompt_ms: f64,
    predicted_n: f64,
    predicted_ms: f64,
    wall_ms: f64,
}

fn send(body: &Value, cache: bool) -> Result<Timing, String> {
    let mut b = body.clone();
    b["cache_prompt"] = json!(cache);
    let t = Instant::now();
    let (code, v) = http("POST", "/v1/chat/completions", Some(&b))?;
    let wall_ms = t.elapsed().as_secs_f64() * 1000.0;
    if code != 200 {
        return Err(format!("engine answered {code}: {v}"));
    }
    let n = |k: &str| v["timings"][k].as_f64().unwrap_or(0.0);
    Ok(Timing {
        prompt_n: n("prompt_n"),
        cache_n: n("cache_n"),
        prompt_ms: n("prompt_ms"),
        predicted_n: n("predicted_n"),
        predicted_ms: n("predicted_ms"),
        wall_ms,
    })
}

// ------------------------------------------------------------------ figures

/// Median, quartiles and range of a measure.
fn stats(mut v: Vec<f64>) -> Value {
    v.retain(|x| x.is_finite());
    if v.is_empty() {
        return Value::Null;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let q = |p: f64| {
        let i = p * (v.len() - 1) as f64;
        let (lo, hi) = (i.floor() as usize, i.ceil() as usize);
        v[lo] + (v[hi] - v[lo]) * (i - lo as f64)
    };
    json!({ "n": v.len(), "median": q(0.5), "p25": q(0.25), "p75": q(0.75), "min": v[0], "max": v[v.len() - 1] })
}

/// Prompt speed, generation speed and wall time over a set of timings.
fn summary(ts: &[Timing]) -> Value {
    let rate = |n: f64, ms: f64| {
        if ms > 0.0 && n > 0.0 {
            n * 1000.0 / ms
        } else {
            f64::NAN
        }
    };
    json!({
        "promptTokensPerSecond": stats(ts.iter().map(|t| rate(t.prompt_n, t.prompt_ms)).collect()),
        "generationTokensPerSecond": stats(ts.iter().map(|t| rate(t.predicted_n, t.predicted_ms)).collect()),
        "promptTokens": stats(ts.iter().map(|t| t.prompt_n + t.cache_n).collect()),
        "wallMs": stats(ts.iter().map(|t| t.wall_ms).collect()),
    })
}

/// Fresh prompts (cache off), `repeats` times over each body, after one warm-up request.
fn measure(bodies: &[Value], repeats: usize) -> Result<Vec<Timing>, String> {
    send(&bodies[0], false)?;
    let mut out = Vec::new();
    for r in 0..repeats {
        for (i, b) in bodies.iter().enumerate() {
            eprintln!(
                "  repeat {}/{repeats}, prompt {}/{}",
                r + 1,
                i + 1,
                bodies.len()
            );
            out.push(send(b, false)?);
        }
    }
    Ok(out)
}

// ------------------------------------------------------------------ run

fn run() -> Result<(), String> {
    let a = parse_args()?;
    let lines: Vec<Value> = std::fs::read_to_string(&a.requests)
        .map_err(|e| format!("{}: {e}", a.requests.display()))?
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    // The first step of each recorded case: a realistic fresh prompt (policy, tools, context).
    let mut firsts: Vec<Value> = lines
        .iter()
        .filter(|l| l["step"] == 0)
        .map(|l| l["body"].clone())
        .collect();
    firsts.truncate(a.cases);
    if firsts.is_empty() {
        return Err("no recorded requests".into());
    }
    let cpu = server_exe(false)?;
    let model = pinned_chat_model()?;
    let started = seconds_now();
    let result = match a.exp.as_str() {
        "threads" => {
            let mut rows = Vec::new();
            for &t in &a.threads {
                eprintln!("threads {t}");
                let _e = start(&cpu, &model, t, None)?;
                rows.push(
                    json!({ "threads": t, "figures": summary(&measure(&firsts, a.repeats)?) }),
                );
            }
            json!({ "runs": rows })
        }
        "card" => {
            let mut rows = Vec::new();
            {
                eprintln!("processor, {} threads", a.t);
                let _e = start(&cpu, &model, a.t, None)?;
                rows.push(json!({ "place": format!("processor, {} threads", a.t), "figures": summary(&measure(&firsts, a.repeats)?) }));
            }
            eprintln!("Vulkan0");
            let _e = start(&server_exe(true)?, &model, a.t, Some("Vulkan0"))?;
            rows.push(json!({ "place": "Vulkan0, every layer", "figures": summary(&measure(&firsts, a.repeats)?) }));
            json!({ "runs": rows })
        }
        "schema" => {
            let _e = start(&cpu, &model, a.t, None)?;
            let without: Vec<Value> = firsts
                .iter()
                .map(|b| {
                    let mut b = b.clone();
                    if let Some(o) = b.as_object_mut() {
                        o.remove("response_format");
                    }
                    b
                })
                .collect();
            let with_n = firsts
                .iter()
                .filter(|b| b.get("response_format").is_some())
                .count();
            json!({ "promptsWithSchema": with_n, "runs": [
                { "schema": true, "figures": summary(&measure(&firsts, a.repeats)?) },
                { "schema": false, "figures": summary(&measure(&without, a.repeats)?) },
            ]})
        }
        "quant" => {
            let other = a
                .model
                .clone()
                .ok_or("--model <file.gguf> is required for quant")?;
            let mut rows = Vec::new();
            for m in [&model, &other] {
                eprintln!("model {}", m.display());
                let _e = start(&cpu, m, a.t, None)?;
                rows.push(
                    json!({ "model": m.file_name().map(|n| n.to_string_lossy().into_owned()),
                    "bytes": std::fs::metadata(m).map(|x| x.len()).unwrap_or(0),
                    "figures": summary(&measure(&firsts, a.repeats)?) }),
                );
            }
            json!({ "runs": rows })
        }
        "reuse" => {
            let _e = start(&cpu, &model, a.t, None)?;
            send(&firsts[0], false)?;
            // Each case's steps in order, as the assistant sends them, cache on then off.
            let mut cases: Vec<(String, Vec<Value>)> = Vec::new();
            for l in &lines {
                let id = l["case"].as_str().unwrap_or_default().to_string();
                match cases.last_mut() {
                    Some((c, v)) if *c == id => v.push(l["body"].clone()),
                    _ => cases.push((id, vec![l["body"].clone()])),
                }
            }
            cases.retain(|(_, v)| v.len() > 1);
            cases.truncate(a.cases);
            let (mut on, mut off, mut share, mut per_case) =
                (Vec::new(), Vec::new(), Vec::new(), Vec::new());
            for _ in 0..a.repeats {
                for (id, steps) in &cases {
                    eprintln!("  case {id}, {} steps", steps.len());
                    let (mut on_ms, mut off_ms) = (0.0, 0.0);
                    // A different short prompt in between, so nothing is reused across cases.
                    send(
                        &json!({ "messages": [{ "role": "user", "content": "Reset." }], "max_tokens": 1 }),
                        true,
                    )?;
                    for (i, b) in steps.iter().enumerate() {
                        let t = send(b, true)?;
                        on_ms += t.wall_ms;
                        if i > 0 && t.prompt_n + t.cache_n > 0.0 {
                            share.push(t.cache_n / (t.prompt_n + t.cache_n));
                        }
                        on.push(t);
                    }
                    for b in steps {
                        let t = send(b, false)?;
                        off_ms += t.wall_ms;
                        off.push(t);
                    }
                    per_case.push(json!({ "case": id, "steps": steps.len(), "cacheOnMs": on_ms, "cacheOffMs": off_ms }));
                }
            }
            json!({ "cases": cases.len(),
                "reusedShareOfLaterSteps": stats(share),
                "cacheOn": summary(&on), "cacheOff": summary(&off),
                "promptMsCacheOn": stats(on.iter().map(|t| t.prompt_ms).collect()),
                "promptMsCacheOff": stats(off.iter().map(|t| t.prompt_ms).collect()),
                "perCase": per_case })
        }
        other => return Err(format!("unknown experiment {other}")),
    };
    let mut sys = sysinfo::System::new();
    sys.refresh_cpu_all();
    sys.refresh_memory();
    let report = json!({
        "experiment": a.exp, "startedUnixSeconds": started, "repeats": a.repeats, "prompts": firsts.len(),
        "computer": {
            "cpu": sys.cpus().first().map(|c| c.brand().trim().to_string()),
            "logicalCores": sys.cpus().len(),
            "physicalCores": sysinfo::System::physical_core_count(),
            "memoryMb": sys.total_memory() / 1_048_576,
            "os": sysinfo::System::long_os_version(),
        },
        "engine": cpu.parent().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()),
        "result": result,
    });
    if let Some(dir) = a.out.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&a.out, serde_json::to_string_pretty(&report).unwrap())
        .map_err(|e| e.to_string())?;
    eprintln!("engine-bench: wrote {}", a.out.display());
    Ok(())
}

fn seconds_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
