//! Builds a complete demo project through the same core operations the app calls, from a content
//! folder that holds everything specific to the film. This program contains no film text.
//!
//! ```text
//! demo_project <content folder> <output folder>
//! ```
//!
//! The output folder receives `<title>.straft/`, `exports/`, `packages/` and `COVERAGE.md`. Scratch
//! data roots go in `STRAFT_DEMO_WORK`, or in `.work` beside the output folder. Add `--no-ai` to skip
//! Straft AI. The AI phase needs `.ai-assets/models` and `.build-cache/ai-runtime/macos-arm64`.

mod ai;
mod assets;
mod exports;
#[allow(dead_code)]
#[path = "../../gold.rs"]
mod gold;
#[allow(dead_code)]
#[path = "../../media.rs"]
mod media;
mod packages;
mod production;
mod project;
mod report;
mod schedule;
mod script;
mod story;
mod vault;
mod visual;
mod workspace;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::gold::Gold;

pub struct Ctx {
    pub repo: PathBuf,
    pub content: PathBuf,
    pub out: PathBuf,
    pub media: PathBuf,
    pub title: String,
    pub project: Value,
    pub vault: Value,
    pub story: Value,
    pub breakdown: Value,
    pub production: Value,
    pub visual: Value,
    pub script: Value,
    pub ai: Value,
    pub ids: BTreeMap<String, String>,
    pub cards: BTreeMap<u32, String>,
    pub scenes: BTreeMap<u32, String>,
    pub lineage: BTreeMap<u32, String>,
    pub chars: BTreeMap<String, String>,
    pub catalog: BTreeMap<String, String>,
    pub locations: BTreeMap<String, String>,
    pub cast: BTreeMap<String, String>,
    pub crew: BTreeMap<String, String>,
    pub days: BTreeMap<u32, String>,
    pub call_sheets: BTreeMap<u32, String>,
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
    /// A path under the output folder, with its parent folder made.
    pub fn outp(&self, rel: &str) -> String {
        let p = self.out.join(rel);
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d).ok();
        }
        p.to_string_lossy().into_owned()
    }
    pub fn media_path(&self, name: &str) -> String {
        self.media.join(name).to_string_lossy().into_owned()
    }
    /// Scene id for a scene number held in a JSON value.
    pub fn scene(&self, n: &Value) -> String {
        let n = n.as_u64().unwrap_or(0) as u32;
        self.scenes.get(&n).cloned().unwrap_or_default()
    }
}

fn read_json(p: &Path) -> Value {
    let text = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let no_ai = args.iter().any(|a| a == "--no-ai");
    let paths: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    let [content, out] = paths.as_slice() else {
        eprintln!("usage: demo_project <content folder> <output folder> [--no-ai]");
        std::process::exit(2);
    };
    let content = PathBuf::from(content)
        .canonicalize()
        .expect("content folder");
    std::fs::create_dir_all(out).expect("output folder");
    let out = PathBuf::from(out).canonicalize().expect("output folder");
    let work = std::env::var_os("STRAFT_DEMO_WORK").map_or_else(
        || out.parent().unwrap_or(&out).join(".work").join("build"),
        PathBuf::from,
    );
    // Every run starts from nothing, but only in a folder this program made.
    let marker = out.join(".demo-project-output");
    let has_files = std::fs::read_dir(&out).is_ok_and(|mut d| d.next().is_some());
    if has_files && !marker.exists() {
        eprintln!(
            "{} holds other files. Choose an empty folder.",
            out.display()
        );
        std::process::exit(2);
    }
    let _ = std::fs::remove_dir_all(&work);
    for entry in std::fs::read_dir(&out).expect("output folder").flatten() {
        let p = entry.path();
        if p.is_dir() {
            std::fs::remove_dir_all(&p).ok();
        } else {
            std::fs::remove_file(&p).ok();
        }
    }
    std::fs::create_dir_all(&work).expect("work folder");
    std::fs::write(&marker, b"Made by demo_project. Emptied on every run.\n").expect("marker");

    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository");
    let ai_bundle = repo.join(".build-cache/ai-runtime/macos-arm64");
    let ai_models = repo.join(".ai-assets/models");
    let ai_enabled = !no_ai && ai_bundle.is_dir() && ai_models.is_dir();

    let media_dir = content.join("media");
    assets::ensure(&read_json(&content.join("media.json")), &media_dir);

    let project_json = read_json(&content.join("project.json"));
    let mut ctx = Ctx {
        repo,
        title: gold::s(&project_json["title"]),
        project: project_json,
        vault: read_json(&content.join("vault.json")),
        story: read_json(&content.join("story.json")),
        breakdown: read_json(&content.join("breakdown.json")),
        production: read_json(&content.join("production.json")),
        visual: read_json(&content.join("visual.json")),
        script: read_json(&content.join("script.json")),
        ai: read_json(&content.join("ai.json")),
        content,
        out,
        media: media_dir,
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

    let g = Gold::open_as(
        &work.join("studio"),
        ai_enabled.then(|| ai_bundle.clone()),
        env!("CARGO_PKG_VERSION"),
    );
    let started = std::time::Instant::now();

    project::settings(&g, &mut ctx);
    project::create(&g, &mut ctx);
    vault::run(&g, &mut ctx);
    story::board(&g, &mut ctx);
    script::build_outline(&g, &mut ctx);
    story::after_build(&g, &mut ctx);
    script::import_and_write(&g, &mut ctx);
    story::story_days(&g, &mut ctx);
    script::drafts_and_review(&g, &mut ctx);
    production::people_and_places(&g, &mut ctx);
    script::lock_for_production(&g, &mut ctx);
    project::safety_copy(&g, &mut ctx);
    packages::producer_review(&g, &mut ctx);
    production::breakdown(&g, &mut ctx);
    visual::run(&g, &mut ctx);
    schedule::run(&g, &mut ctx);
    schedule::paperwork(&g, &mut ctx);
    production::budget(&g, &mut ctx);
    workspace::notes_tasks_templates_files(&g, &mut ctx);
    script::blue_revision(&g, &mut ctx);
    production::after_revision(&g, &mut ctx);
    schedule::after_revision(&g, &mut ctx);
    workspace::comments_history_trash_search(&g, &mut ctx);
    ai::run(&g, &mut ctx);
    script::teaser(&g, &mut ctx);
    exports::run(&g, &mut ctx);
    packages::run(&g, &mut ctx);
    project::finish(&g, &mut ctx);

    let build_secs = started.elapsed().as_secs();
    report::finish(&g, &ctx, &work, build_secs);
}
