//! Builds the gold reference project "Low Tide Radio" through the same core operations the app
//! calls, exports every output, then imports and restores it into fresh data roots and compares.
//!
//! ```text
//! cargo run -p openframe-gold-project --release            # build, export, verify, write reports
//! cargo run -p openframe-gold-project --release -- --no-ai # the same without the Offline AI
//! ```
//!
//! Inputs: `tests/gold-project/content/` (the film as readable files) and the local Offline AI
//! build inputs (`.ai-assets/models`, `.build-cache/ai-runtime/macos-arm64`). Outputs:
//! `tests/gold-project/outputs/`, `COVERAGE.md`, `VERIFY.md`. Scratch data lives in
//! `tests/gold-project/.work/` (git-ignored) and is rebuilt on every run.

mod coverage;
mod gold;
mod media;
mod phases;
mod verify;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::gold::Gold;

pub struct Ctx {
    pub repo: PathBuf,
    pub content: PathBuf,
    pub out: PathBuf,
    pub media: media::Media,
    pub story: Value,
    pub breakdown: Value,
    pub production: Value,
    /// Named ids (project, screenplay, drafts, schedule, budget, ...).
    pub ids: BTreeMap<String, String>,
    /// Scene number -> Story scene card id.
    pub cards: BTreeMap<u32, String>,
    /// Scene number -> scene id in the draft production works from.
    pub scenes: BTreeMap<u32, String>,
    /// Scene number -> scene lineage id (stable across drafts).
    pub lineage: BTreeMap<u32, String>,
    /// Character key -> Story character id.
    pub chars: BTreeMap<String, String>,
    /// Catalog item name -> catalog item id.
    pub catalog: BTreeMap<String, String>,
    /// Location key -> production location id.
    pub locations: BTreeMap<String, String>,
    /// Character cue -> cast member id.
    pub cast: BTreeMap<String, String>,
    pub crew: BTreeMap<String, String>,
    /// Shooting day number -> schedule day id.
    pub days: BTreeMap<u32, String>,
    pub call_sheets: BTreeMap<u32, String>,
    /// Facts the verification compares after import and restore.
    pub facts: BTreeMap<String, Value>,
    pub ai_enabled: bool,
}

impl Ctx {
    pub fn id(&self, key: &str) -> String {
        self.ids.get(key).cloned().unwrap_or_default()
    }
    pub fn set(&mut self, key: &str, v: impl Into<String>) {
        self.ids.insert(key.to_string(), v.into());
    }
    pub fn outp(&self, rel: &str) -> String {
        let p = self.out.join(rel);
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d).ok();
        }
        p.to_string_lossy().into_owned()
    }
}

fn read_json(p: &Path) -> Value {
    let text = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let no_ai = args.iter().any(|a| a == "--no-ai");
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let base = repo.join("tests/gold-project");
    let content = base.join("content");
    let out = base.join("outputs");
    let work = base.join(".work");
    // Every run starts from nothing, so the result never depends on an earlier run.
    let _ = std::fs::remove_dir_all(&work);
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    std::fs::create_dir_all(&work).unwrap();

    let ai_bundle = repo.join(".build-cache/ai-runtime/macos-arm64");
    let ai_models = repo.join(".ai-assets/models");
    let ai_enabled = !no_ai && ai_bundle.is_dir() && ai_models.is_dir();

    let media = media::ensure(&content.join("media"));
    let mut ctx = Ctx {
        repo: repo.clone(),
        story: read_json(&content.join("story.json")),
        breakdown: read_json(&content.join("breakdown.json")),
        production: read_json(&content.join("production.json")),
        content,
        out,
        media,
        ids: BTreeMap::new(),
        cards: BTreeMap::new(),
        scenes: BTreeMap::new(),
        lineage: BTreeMap::new(),
        chars: BTreeMap::new(),
        catalog: BTreeMap::new(),
        locations: BTreeMap::new(),
        cast: BTreeMap::new(),
        crew: BTreeMap::new(),
        days: BTreeMap::new(),
        call_sheets: BTreeMap::new(),
        facts: BTreeMap::new(),
        ai_enabled,
    };

    let g = Gold::open(&work.join("studio"), ai_enabled.then(|| ai_bundle.clone()));
    let started = std::time::Instant::now();
    phases::run_all(&g, &mut ctx);
    let build_secs = started.elapsed().as_secs();

    // Integrity and recoverability: import the package and restore the backup elsewhere.
    let mut report = verify::run(&g, &ctx, &work);
    g.shutdown();
    let source = report.source.clone();
    report
        .copies
        .push(verify::reopen_original(&work, &ctx, &source));
    coverage::write(&g, &ctx, &base, build_secs);
    verify::write(&report, &ctx, &base);
    let failures = g.failures.borrow().len();
    println!(
        "done in {}s: {} unexpected failures, verification {}",
        started.elapsed().as_secs(),
        failures,
        if report.ok() {
            "passed"
        } else {
            "found differences"
        }
    );
}
