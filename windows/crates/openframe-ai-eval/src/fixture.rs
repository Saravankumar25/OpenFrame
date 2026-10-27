//! Fixture projects for the evaluation harness (docs/specs/ai-conversational-harness.md, section 7.2).
//!
//! A gold case picks one with its `project` field, next to the four sample script ids:
//!
//! | `project` | What it holds |
//! |---|---|
//! | `low-tide-two-screenplays` | A production-complete fictional film: two screenplays and every production area. |
//! | `thin-project` | Only the teaser screenplay, to test honest abstention. |
//! | `big-project` | 500 generated scenes and 5,000 generated notes, to time indexing and fetches. |
//!
//! Everything is built through the real operations, so a fixture is exactly what a person could have
//! made in the app. `gold/fixtures/low-tide-two-screenplays/FACTS.md` lists every fact a case may ask
//! about. The setup is `setup.jsonl`: one step per line, `{ "op", "args", "as"?, "name"? }`. `as`
//! (`"other"`) runs the step as the second user; `name` keeps the result for later steps.
//!
//! Strings inside `args` that are exactly one of these are replaced before the step runs:
//! `$feature_screenplay`, `$feature_draft`, `$teaser_screenplay`, `$teaser_draft`, `$scene:N` (scene
//! N of the feature), `$tscene:N` (scene N of the teaser), `$ref:<name>.<path>` (a field of an
//! earlier result, `.id` for example) and `$file:<name>` (a file written by `harness.write_file`).
//! Steps named `harness.*` are handled here: `write_file`, `set_source`, `schedule_scenes`,
//! `callsheet_edit`, `breakdown_add`.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use openframe_application::core::AppCore;
use openframe_domain::Actor;
use serde::Deserialize;
use serde_json::{Value, json};

pub const LOW_TIDE: &str = "low-tide-two-screenplays";
pub const THIN: &str = "thin-project";
pub const BIG: &str = "big-project";

/// Scenes and notes of `big-project`.
pub const BIG_SCENES: usize = 500;
pub const BIG_NOTES: usize = 5_000;

/// True when `name` is a fixture id rather than a sample script id.
pub fn is_fixture(name: &str) -> bool {
    [LOW_TIDE, THIN, BIG].contains(&name)
}

/// The screenplay whose current draft a case reads: the feature of `low-tide-two-screenplays`,
/// the teaser of `thin-project`, the generated script of `big-project`.
#[derive(Debug, Clone)]
pub struct Built {
    pub screenplay_id: String,
    pub draft_id: String,
}

pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("gold")
        .join("fixtures")
}

/// Build the fixture into the open project of `core`.
pub fn build(core: &AppCore, name: &str) -> Result<Built, String> {
    match name {
        LOW_TIDE => low_tide(core),
        THIN => thin(core),
        BIG => big(core),
        other => Err(format!("unknown fixture {other}")),
    }
}

fn call(core: &AppCore, actor: &Actor, op: &str, args: Value) -> Result<Value, String> {
    core.dispatch(actor, op, args)
        .map_err(|e| format!("{op}: {} {}", e.code_str(), e.message))
}

fn read_fixture(folder: &str, file: &str) -> Result<String, String> {
    let p = fixtures_dir().join(folder).join(file);
    std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))
}

struct Imported {
    built: Built,
    scene_ids: Vec<String>,
}

fn import(core: &AppCore, text: &str) -> Result<Imported, String> {
    let me = core.local_actor();
    let r = call(
        core,
        &me,
        "screenplay.import_apply",
        json!({ "pastedText": text, "format": "fountain", "mode": "new_screenplay" }),
    )?;
    let field = |k: &str| {
        r[k].as_str()
            .map(str::to_string)
            .ok_or_else(|| format!("import result has no {k}"))
    };
    let built = Built {
        screenplay_id: field("screenplayId")?,
        draft_id: field("draftId")?,
    };
    let doc = call(
        core,
        &me,
        "screenplay.document",
        json!({ "draftId": built.draft_id }),
    )?;
    let scene_ids = doc["scenes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| s["id"].as_str().map(str::to_string))
        .collect();
    Ok(Imported { built, scene_ids })
}

fn thin(core: &AppCore) -> Result<Built, String> {
    Ok(import(core, &read_fixture(LOW_TIDE, "teaser.fountain")?)?.built)
}

// ------------------------------------------------------------------ low-tide-two-screenplays

#[derive(Deserialize)]
struct Step {
    op: String,
    #[serde(default)]
    args: Value,
    #[serde(default, rename = "as")]
    as_user: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

struct Run<'a> {
    core: &'a AppCore,
    me: Actor,
    other: Actor,
    feature: Built,
    teaser: Built,
    feature_scenes: Vec<String>,
    teaser_scenes: Vec<String>,
    refs: HashMap<String, Value>,
    files: BTreeMap<String, String>,
    dir: tempfile::TempDir,
}

fn low_tide(core: &AppCore) -> Result<Built, String> {
    let feature = import(core, &read_fixture(LOW_TIDE, "feature.fountain")?)?;
    let teaser = import(core, &read_fixture(LOW_TIDE, "teaser.fountain")?)?;
    let result = feature.built.clone();
    let mut run = Run {
        core,
        me: core.local_actor(),
        other: crate::other_actor(),
        feature: feature.built,
        teaser: teaser.built,
        feature_scenes: feature.scene_ids,
        teaser_scenes: teaser.scene_ids,
        refs: HashMap::new(),
        files: BTreeMap::new(),
        dir: tempfile::tempdir().map_err(|e| e.to_string())?,
    };
    let text = read_fixture(LOW_TIDE, "setup.jsonl")?;
    let trace = std::env::var_os("FIXTURE_TRACE").is_some();
    for (i, line) in text
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
    {
        let step: Step =
            serde_json::from_str(line).map_err(|e| format!("setup.jsonl:{}: {e}", i + 1))?;
        let out = run
            .step(&step)
            .map_err(|e| format!("setup.jsonl:{} ({}): {e}", i + 1, step.op))?;
        if trace {
            let shown: String = out.to_string().chars().take(160).collect();
            eprintln!("{} {} -> {shown}", i + 1, step.op);
        }
        if let Some(n) = &step.name {
            run.refs.insert(n.clone(), out);
        }
    }
    Ok(result)
}

impl Run<'_> {
    fn step(&mut self, s: &Step) -> Result<Value, String> {
        let args = self.sub(&s.args)?;
        match s.op.as_str() {
            "harness.write_file" => {
                let field = |k: &str| args[k].as_str().unwrap_or_default().to_string();
                let name = field("name");
                let path = crate::write_case_file(self.dir.path(), &name, &field("text"))?;
                self.files.insert(name, path);
                Ok(Value::Null)
            }
            "harness.set_source" => self.set_source(),
            "harness.schedule_scenes" => self.schedule_scenes(&args),
            "harness.callsheet_edit" => self.callsheet_edit(&args),
            "harness.breakdown_add" => self.breakdown_add(&args),
            _ => {
                let actor = if s.as_user.as_deref() == Some("other") {
                    &self.other
                } else {
                    &self.me
                };
                call(self.core, actor, &s.op, args)
            }
        }
    }

    /// The feature draft becomes the Production Source, as a producer would choose it.
    fn set_source(&self) -> Result<Value, String> {
        let current = call(self.core, &self.me, "production.source", json!({}))?;
        if current.is_null() {
            call(
                self.core,
                &self.me,
                "production.set_source",
                json!({ "draftId": self.feature.draft_id }),
            )
        } else {
            Ok(current)
        }
    }

    /// Put the listed feature scenes (ordinals) on a shoot day. The schedule is not strict, so the
    /// move goes through and its warnings (an actor's unavailable window, say) stay open: they are
    /// part of what the fixture teaches.
    fn schedule_scenes(&self, args: &Value) -> Result<Value, String> {
        let view = call(self.core, &self.me, "schedule.get", json!({}))?;
        let mut strip_of_scene = HashMap::new();
        for s in view["unscheduled"].as_array().into_iter().flatten() {
            if let (Some(strip), Some(scene)) = (s["id"].as_str(), s["sceneId"].as_str()) {
                strip_of_scene.insert(scene.to_string(), strip.to_string());
            }
        }
        let mut strips = Vec::new();
        for n in args["scenes"].as_array().into_iter().flatten() {
            let n = n.as_u64().unwrap_or(0) as usize;
            let scene = self
                .feature_scenes
                .get(n.wrapping_sub(1))
                .ok_or_else(|| format!("no feature scene {n}"))?;
            strips.push(
                strip_of_scene
                    .get(scene)
                    .cloned()
                    .ok_or_else(|| format!("scene {n} has no unscheduled strip"))?,
            );
        }
        call(
            self.core,
            &self.me,
            "schedule.move_strips",
            json!({ "stripIds": strips, "dayId": args["day"], "keepAnyway": false }),
        )
    }

    /// Add a breakdown element, reusing the catalog item of the same name and category and making a
    /// new one otherwise (the automatic choice refuses names that are merely similar).
    fn breakdown_add(&self, args: &Value) -> Result<Value, String> {
        let found = call(
            self.core,
            &self.me,
            "catalog.list",
            json!({ "category": args["category"], "search": args["name"] }),
        )?;
        let name = args["name"].as_str().unwrap_or_default();
        let items = found.as_array().or_else(|| found["items"].as_array());
        let existing = items.into_iter().flatten().find(|i| {
            i["name"]
                .as_str()
                .is_some_and(|n| n.eq_ignore_ascii_case(name))
        });
        let catalog = match existing.and_then(|i| i["id"].as_str()) {
            Some(id) => json!({ "mode": "existing", "catalogItemId": id }),
            None => json!({ "mode": "new" }),
        };
        let mut a = args.clone();
        a["catalog"] = catalog;
        call(self.core, &self.me, "breakdown.add_element", a)
    }

    /// Set fields of the call sheet of one shoot day: `set` maps dotted document paths
    /// (`crewCall`, `optional.crew`) to values.
    fn callsheet_edit(&self, args: &Value) -> Result<Value, String> {
        let list = call(self.core, &self.me, "callsheets.list", json!({}))?;
        let id = list
            .as_array()
            .into_iter()
            .flatten()
            .find(|r| r["dayId"] == args["day"] && r["replaced"] != true)
            .and_then(|r| r["id"].as_str().map(str::to_string))
            .ok_or("no call sheet for that day")?;
        let sheet = call(self.core, &self.me, "callsheets.get", json!({ "id": id }))?;
        let mut document = sheet["document"].clone();
        if !document.is_object() {
            return Err(format!("call sheet has no document: {sheet}"));
        }
        for (path, value) in args["set"].as_object().into_iter().flatten() {
            let mut at = &mut document;
            for key in path.split('.') {
                at = &mut at[key];
            }
            *at = value.clone();
        }
        call(
            self.core,
            &self.me,
            "callsheets.update",
            json!({ "id": id, "document": document, "expectedRev": sheet["rev"] }),
        )
    }

    fn sub(&self, v: &Value) -> Result<Value, String> {
        match v {
            Value::String(s) => self.sub_str(s),
            Value::Object(m) => m
                .iter()
                .map(|(k, x)| Ok((k.clone(), self.sub(x)?)))
                .collect::<Result<_, String>>()
                .map(Value::Object),
            Value::Array(a) => a
                .iter()
                .map(|x| self.sub(x))
                .collect::<Result<_, _>>()
                .map(Value::Array),
            _ => Ok(v.clone()),
        }
    }

    fn sub_str(&self, s: &str) -> Result<Value, String> {
        let scene = |ids: &[String], n: &str| -> Result<Value, String> {
            let n: usize = n.parse().map_err(|_| format!("bad scene number in {s}"))?;
            ids.get(n.wrapping_sub(1))
                .map(|id| json!(id))
                .ok_or_else(|| format!("no scene {n} for {s}"))
        };
        match s {
            "$feature_screenplay" => return Ok(json!(self.feature.screenplay_id)),
            "$feature_draft" => return Ok(json!(self.feature.draft_id)),
            "$teaser_screenplay" => return Ok(json!(self.teaser.screenplay_id)),
            "$teaser_draft" => return Ok(json!(self.teaser.draft_id)),
            _ => {}
        }
        if let Some(n) = s.strip_prefix("$scene:") {
            return scene(&self.feature_scenes, n);
        }
        if let Some(n) = s.strip_prefix("$tscene:") {
            return scene(&self.teaser_scenes, n);
        }
        if let Some(name) = s.strip_prefix("$file:") {
            return self
                .files
                .get(name)
                .map(|p| json!(p))
                .ok_or_else(|| format!("no file {name} was written"));
        }
        if let Some(path) = s.strip_prefix("$ref:") {
            let mut parts = path.split('.');
            let name = parts.next().unwrap_or_default();
            let mut cur = self
                .refs
                .get(name)
                .ok_or_else(|| format!("no earlier step is named {name}"))?;
            for p in parts {
                // An operation that returns just the new id has no `id` field to read.
                if p == "id" && cur.is_string() {
                    continue;
                }
                cur = match cur {
                    Value::Array(a) => p.parse::<usize>().ok().and_then(|i| a.get(i)),
                    other => other.get(p),
                }
                .ok_or_else(|| format!("{s}: no field {p} in {cur}"))?;
            }
            return Ok(cur.clone());
        }
        Ok(json!(s))
    }
}

// ------------------------------------------------------------------ big-project

const PLACES: [&str; 10] = [
    "HARBOUR",
    "MARKET",
    "LIGHTHOUSE",
    "SCHOOL YARD",
    "FERRY STAND",
    "CHAPEL",
    "BOAT SHED",
    "BEACH",
    "CLINIC",
    "RADIO HUT",
];
const TIMES: [&str; 4] = ["DAWN", "DAY", "DUSK", "NIGHT"];
const NAMES: [&str; 4] = ["MIRA", "DEV", "SARA", "KABIR"];
const WORDS: [&str; 16] = [
    "tide", "net", "lantern", "salt", "ferry", "bell", "rope", "gull", "rain", "oar", "chart",
    "kettle", "ledger", "signal", "anchor", "lamp",
];

/// A fixed phrase for a number: the same number always gives the same words.
fn phrase(n: usize, words: usize) -> String {
    (0..words)
        .map(|k| WORDS[(n * 7 + k * 5 + n / 16) % WORDS.len()])
        .collect::<Vec<_>>()
        .join(" ")
}

/// The Fountain text of `big-project`: `BIG_SCENES` scenes, deterministic.
pub fn big_script() -> String {
    let mut t = String::from(
        "Title: Generated Coast Drama\nCredit: Written by\nAuthor: OpenFrame evaluation fixture\n\n",
    );
    for i in 0..BIG_SCENES {
        let io = if i % 3 == 0 { "EXT." } else { "INT." };
        t += &format!(
            "{io} {} {} - {}\n\n",
            PLACES[i % PLACES.len()],
            i / PLACES.len() + 1,
            TIMES[i % TIMES.len()]
        );
        t += &format!(
            "The {} waits by the {}. Scene {} of the generated script.\n\n",
            phrase(i, 2),
            phrase(i + 3, 1),
            i + 1
        );
        for k in 0..(1 + i % 2) {
            t += &format!(
                "{}\nI saw the {} near the {}.\n\n",
                NAMES[(i + k) % NAMES.len()],
                phrase(i + k, 1),
                phrase(i + 9, 2)
            );
        }
    }
    t
}

fn big(core: &AppCore) -> Result<Built, String> {
    let built = import(core, &big_script())?;
    let me = core.local_actor();
    for i in 1..=BIG_NOTES {
        call(
            core,
            &me,
            "notes.create",
            json!({
                "title": format!("Note {i:04}"),
                "body": format!("Reminder {i}: check the {} before the {} scene, then ring the {}.", phrase(i, 2), phrase(i + 5, 1), phrase(i + 11, 1)),
                "pinned": false,
            }),
        )?;
    }
    Ok(built.built)
}
