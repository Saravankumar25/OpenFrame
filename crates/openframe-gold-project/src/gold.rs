//! The recording harness: one `AppCore` on an isolated data root, and every operation call
//! counted, so the coverage report comes from what was actually called.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use openframe_application::events::{AppEvent, RecordingSink};
use openframe_application::{AppConfig, AppCore};
use openframe_domain::{Actor, AppError};
use serde_json::{Value, json};

/// One call that failed although the program expected it to work.
#[derive(Debug, Clone)]
pub struct Failure {
    pub phase: String,
    pub op: String,
    pub code: String,
    pub message: String,
    pub detail: String,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Count {
    pub ok: u32,
    /// Refusals the program asked for on purpose (validation and permission checks).
    pub expected_err: u32,
    pub unexpected_err: u32,
}

pub struct Gold {
    pub root: PathBuf,
    pub core: Arc<AppCore>,
    pub sink: Arc<RecordingSink>,
    pub counts: RefCell<BTreeMap<String, Count>>,
    pub failures: RefCell<Vec<Failure>>,
    pub phase: RefCell<String>,
    pub quiet: bool,
}

pub fn s(v: &Value) -> String {
    v.as_str().map(str::to_string).unwrap_or_default()
}

pub fn arr(v: &Value) -> Vec<Value> {
    v.as_array().cloned().unwrap_or_default()
}

impl Gold {
    /// A core over `root` (appdata, projects, global-vault), optionally with the Offline AI.
    pub fn open(root: &Path, ai_bundle: Option<PathBuf>) -> Gold {
        std::fs::create_dir_all(root).expect("data root");
        let sink = Arc::new(RecordingSink::default());
        let mut config = AppConfig::isolated(root);
        config.app_version = "0.1.0-gold".into();
        if let Some(b) = ai_bundle {
            config.ai_bundle_dir = Some(b);
            config.ai_model_source = Some(openframe_application::ModelSource::huggingface());
            config.ai_deterministic = true;
        }
        let core = AppCore::new(config, sink.clone()).expect("core");
        Gold {
            root: root.to_path_buf(),
            core,
            sink,
            counts: RefCell::new(BTreeMap::new()),
            failures: RefCell::new(Vec::new()),
            phase: RefCell::new(String::new()),
            quiet: false,
        }
    }

    pub fn phase(&self, name: &str) {
        *self.phase.borrow_mut() = name.to_string();
        if !self.quiet {
            println!("== {name}");
        }
    }

    pub fn actor(&self) -> Actor {
        self.core.local_actor()
    }

    fn bump(&self, op: &str, f: impl FnOnce(&mut Count)) {
        let mut c = self.counts.borrow_mut();
        f(c.entry(op.to_string()).or_default());
    }

    /// Call, count, and return the raw result.
    pub fn call(&self, op: &str, args: Value) -> Result<Value, AppError> {
        self.call_as(&self.actor(), op, args)
    }

    pub fn call_as(&self, actor: &Actor, op: &str, args: Value) -> Result<Value, AppError> {
        let r = self.core.dispatch(actor, op, args);
        match &r {
            Ok(_) => self.bump(op, |c| c.ok += 1),
            Err(_) => self.bump(op, |c| c.unexpected_err += 1),
        }
        r
    }

    /// Call expecting success. A failure is recorded for the report and `Null` returned, so one
    /// broken operation never hides the rest of the run.
    pub fn ok(&self, op: &str, args: Value) -> Value {
        match self.call(op, args) {
            Ok(v) => v,
            Err(e) => {
                self.record_failure(op, &e);
                Value::Null
            }
        }
    }

    pub fn record_failure(&self, op: &str, e: &AppError) {
        let f = Failure {
            phase: self.phase.borrow().clone(),
            op: op.to_string(),
            code: e.code.0.clone(),
            message: e.message.clone(),
            detail: e.detail.clone().unwrap_or_default(),
        };
        eprintln!("  FAILED {op}: {} {} {}", f.code, f.message, f.detail);
        self.failures.borrow_mut().push(f);
    }

    /// Call expecting a refusal (a validation or permission rule the project exercises on purpose).
    /// Returns the error code; an unexpected success is recorded as a failure.
    pub fn refused(&self, op: &str, args: Value) -> String {
        match self.core.dispatch(&self.actor(), op, args) {
            Ok(v) => {
                self.bump(op, |c| c.ok += 1);
                let e = AppError::new(
                    "gold.unexpected_success",
                    format!("expected a refusal, got {}", truncate(&v.to_string(), 200)),
                );
                self.record_failure(op, &e);
                String::new()
            }
            Err(e) => {
                self.bump(op, |c| c.expected_err += 1);
                if !self.quiet {
                    println!("  refused as expected {op}: {} {}", e.code.0, e.message);
                }
                e.code.0
            }
        }
    }

    /// Call an operation whose answer depends on the release channel (store-only purchase
    /// calls in an internal build): success and a refusal both count as exercised, and the
    /// answer is returned for the report.
    pub fn probe(&self, op: &str, args: Value) -> Value {
        match self.core.dispatch(&self.actor(), op, args) {
            Ok(v) => {
                self.bump(op, |c| c.ok += 1);
                v
            }
            Err(e) => {
                self.bump(op, |c| c.expected_err += 1);
                json!({ "refused": e.code.0, "message": e.message })
            }
        }
    }

    pub fn id(&self, v: &Value) -> String {
        if let Some(s) = v.as_str() {
            return s.to_string();
        }
        for k in ["id", "storyId", "itemId"] {
            if let Some(s) = v[k].as_str() {
                return s.to_string();
            }
        }
        if let Some(s) = v["comment"]["id"].as_str() {
            return s.to_string();
        }
        String::new()
    }

    /// Wait for a background task started by a long-running operation.
    pub fn wait_task(&self, started: &Value) -> Result<Value, AppError> {
        let Some(task_id) = started["taskId"].as_str() else {
            return Err(AppError::new("gold.no_task", "the operation did not start a task"));
        };
        let start = Instant::now();
        loop {
            {
                let events = self.sink.events.lock();
                for e in events.iter().rev() {
                    if let AppEvent::Task(t) = e
                        && t.task_id == task_id
                        && matches!(t.state.as_str(), "completed" | "failed" | "cancelled")
                    {
                        return match (&t.result, &t.error) {
                            (Some(r), _) if t.state == "completed" => Ok(r.clone()),
                            (_, Some(err)) => Err(err.clone()),
                            _ => Ok(t.result.clone().unwrap_or(Value::Null)),
                        };
                    }
                }
            }
            if start.elapsed() > Duration::from_secs(900) {
                return Err(AppError::new("gold.timeout", "the task did not finish in 15 minutes"));
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    /// Start a long-running operation and wait for it; failures are recorded.
    pub fn task(&self, op: &str, args: Value) -> Value {
        let started = self.ok(op, args);
        if started.is_null() {
            return Value::Null;
        }
        match self.wait_task(&started) {
            Ok(v) => v,
            Err(e) => {
                self.record_failure(&format!("{op} (task)"), &e);
                Value::Null
            }
        }
    }

    /// Read-only SQL on the open project database (verification only).
    pub fn sql_i64(&self, sql: &str) -> i64 {
        let Ok(p) = self.core.project() else { return -1 };
        p.store
            .read(|c| Ok(c.query_row(sql, [], |r| r.get::<_, i64>(0))?))
            .unwrap_or(-1)
    }

    pub fn sql_rows(&self, sql: &str) -> Vec<Vec<Value>> {
        let Ok(p) = self.core.project() else { return Vec::new() };
        p.store
            .read(|c| {
                let mut st = c.prepare(sql)?;
                let n = st.column_count();
                let rows = st.query_map([], |r| {
                    let mut out = Vec::with_capacity(n);
                    for i in 0..n {
                        let v: rusqlite::types::Value = r.get(i)?;
                        out.push(match v {
                            rusqlite::types::Value::Null => Value::Null,
                            rusqlite::types::Value::Integer(i) => json!(i),
                            rusqlite::types::Value::Real(f) => json!(f),
                            rusqlite::types::Value::Text(t) => json!(t),
                            rusqlite::types::Value::Blob(b) => json!(format!("blob:{}", b.len())),
                        });
                    }
                    Ok(out)
                })?;
                Ok(rows.collect::<Result<Vec<_>, _>>()?)
            })
            .unwrap_or_default()
    }

    pub fn shutdown(&self) {
        self.core.shutdown();
    }
}

pub fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        format!("{}...", s.chars().take(n).collect::<String>())
    }
}
