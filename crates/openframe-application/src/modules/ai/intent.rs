//! Intent prototypes (AI harness rebuild plan, section 5.2 step 5).
//!
//! The regex routes know the wording they were written against. A reworded question ("Name the
//! most talkative person in the script") used to fall through to the model, which picked
//! `retrieve_context` for almost everything. `intents.json` holds about thirty intents, each with
//! short example questions in English and in Hindi, Spanish, French, German and Japanese. The app's
//! search model (the same embedder as project search; E5 takes the "query: " prefix) embeds every
//! example once, and a question is routed to the intent whose example it is closest to, when the
//! match is clear. No model writes anything here: this is a similarity lookup, then code.
//!
//! - A chat or recipe intent feeds the lane router.
//! - A tool intent becomes a direct call when code can fill every argument the tool needs from what
//!   `understand` found; otherwise the first agent step is offered only that tool (plus `clarify`
//!   and `final_answer`) and the model fills the arguments under the usual grammar.
//!
//! - In the middle band (score from 0.88 without the direct gate) the first agent step is offered only the
//!   tools of the two closest intents, `clarify` and `final_answer`; the trace keeps the gate.
//!
//! When the search model is missing, slow or fails, `classify_within` returns None and the rule
//! router decides, as before. A request never waits more than [`WAIT`] for this step.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use openframe_search::Embedder;
use parking_lot::Mutex;
use regex::Regex;
use serde::Deserialize;
use serde_json::{Value, json};

use super::orchestrator::StepCall;
use super::understand::{EntityKind, Understood};

/// The least cosine score a question needs against its best example (search model scale).
///
/// Every text in this app is about screenplays, so even a question with another meaning sits close
/// to the examples. E5 models put unrelated short texts at about 0.70 to 0.80, same-subject texts
/// that ask for something else at about 0.82 to 0.88, and paraphrases at 0.90 and above. The gate
/// is set at the top of the same-subject band: a wrong direct call gives an exact-looking wrong
/// answer, while a skipped intent only costs what the rule router and the model cost today.
/// Chosen with the stand-in embedder of `tests`, which maps word overlap into that band, and meant
/// to be tuned on the bench: the score, margin and choice of every request are in the trace.
pub(super) const SCORE_MIN: f32 = 0.90;
/// The least score for the middle band. On the bench the closest intent was right about 84 percent
/// of the time from 0.90 up and under half the time below 0.88, so a score in between, or one
/// without the lead a direct call needs, is not trusted with a call; the first agent step is only
/// offered the tools of the two closest intents and the model chooses between them.
pub(super) const BAND_MIN: f32 = 0.88;
/// The least lead over the best intent that asks for a DIFFERENT action (another tool, fact or
/// lane). Two intents that mean the same call never block each other. A close second place means
/// the question is between two readings, and the rule router decides instead. Three hundredths is
/// about the gap between a paraphrase of one example and the same-subject question of another tool.
pub(super) const MARGIN_MIN: f32 = 0.03;
/// A whole-screenplay recipe costs minutes, so its intent needs a clearer match than the rest.
pub(super) const RECIPE_EXTRA: f32 = 0.03;
/// How long a request waits for this step; a late result still fills the example cache.
pub(super) const WAIT: Duration = Duration::from_millis(1500);

/// Languages with their own examples besides English.
const LANGS: [&str; 5] = ["hi", "es", "fr", "de", "ja"];

#[derive(Debug, Deserialize)]
struct Raw {
    intent: String,
    #[serde(default)]
    tool: Option<String>,
    #[serde(default)]
    lane: Option<String>,
    #[serde(default)]
    fact: Option<String>,
    #[serde(default)]
    needs: Option<Need>,
    examples: Vec<String>,
    #[serde(default)]
    other: BTreeMap<String, Vec<String>>,
}

/// What the question must name for the intent to make sense. A question that names no scene cannot
/// be "who speaks in scene N", however close its words are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Need {
    Scene,
    Character,
    Characters,
}

/// What `understand` found in the question: the scenes of the draft and the characters it names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Have {
    pub scenes: usize,
    pub characters: usize,
}

impl Have {
    pub fn of(u: Option<&Understood>) -> Self {
        Self {
            scenes: u.map_or(0, |u| u.scenes.len()),
            characters: u.map_or(0, |u| {
                u.entities
                    .iter()
                    .filter(|e| e.kind == EntityKind::Character)
                    .count()
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Target {
    Tool(String),
    Chat,
    Recipe,
}

#[derive(Debug)]
pub(super) struct Intent {
    pub name: String,
    pub target: Target,
    /// The `fact` argument of `scene_fact` and `dialogue_fact` intents.
    pub fact: Option<String>,
    needs: Option<Need>,
    examples: Vec<String>,
    other: BTreeMap<String, Vec<String>>,
}

impl Intent {
    /// The question names what the intent needs.
    fn possible(&self, have: Have) -> bool {
        match self.needs {
            None => true,
            Some(Need::Scene) => have.scenes >= 1,
            Some(Need::Character) => have.characters >= 1,
            Some(Need::Characters) => have.characters >= 2,
        }
    }

    /// Two intents with the same key ask for the same action.
    fn key(&self) -> String {
        match &self.target {
            Target::Tool(t) => format!("{t}/{}", self.fact.as_deref().unwrap_or("")),
            Target::Chat => "chat".into(),
            Target::Recipe => "recipe".into(),
        }
    }
}

static INTENTS: LazyLock<Vec<Intent>> = LazyLock::new(|| {
    let raw: Vec<Raw> =
        serde_json::from_str(include_str!("intents.json")).expect("intents.json is valid");
    raw.into_iter()
        .map(|r| Intent {
            target: match (r.tool, r.lane.as_deref()) {
                (Some(t), None) => Target::Tool(t),
                (None, Some("chat")) => Target::Chat,
                (None, Some("recipe")) => Target::Recipe,
                _ => panic!("intent {} needs one tool or one lane", r.intent),
            },
            name: r.intent,
            fact: r.fact,
            needs: r.needs,
            examples: r.examples,
            other: r.other,
        })
        .collect()
});

pub(super) fn intents() -> &'static [Intent] {
    &INTENTS
}

/// What an intent name stands for: the tool it calls, or "chat" or "recipe". For the evaluation
/// report, which asks whether the closest intent was the right one.
pub fn intent_target(name: &str) -> Option<String> {
    // The trace may carry a gate after the name ("scene_speakers|top-two").
    let name = name.split('|').next().unwrap_or(name);
    intents()
        .iter()
        .find(|i| i.name == name)
        .map(|i| match &i.target {
            Target::Tool(t) => t.clone(),
            Target::Chat => "chat".to_string(),
            Target::Recipe => "recipe".to_string(),
        })
}

// ------------------------------------------------------------------ prototypes

/// (intent index, unit vector) for each example of one language.
type Prototypes = Arc<Vec<(usize, Vec<f32>)>>;

type Key = (String, &'static str);

static CACHE: LazyLock<Mutex<HashMap<Key, Prototypes>>> = LazyLock::new(Default::default);
static BUILDING: LazyLock<Mutex<HashSet<Key>>> = LazyLock::new(Default::default);

fn examples_of(lang: &str) -> Vec<(usize, &'static str)> {
    let mut out = Vec::new();
    for (i, intent) in intents().iter().enumerate() {
        let list = if lang == "en" {
            Some(&intent.examples)
        } else {
            intent.other.get(lang)
        };
        out.extend(list.into_iter().flatten().map(|t| (i, t.as_str())));
    }
    out
}

fn unit(mut v: Vec<f32>) -> Vec<f32> {
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if n > f32::EPSILON {
        v.iter_mut().for_each(|x| *x /= n);
    }
    v
}

/// The examples of one language, embedded once per search model. None while another thread is
/// still embedding them or when the model fails: the caller skips this step.
fn prototypes(e: &dyn Embedder, lang: &'static str) -> Option<Prototypes> {
    let key: Key = (e.model_id(), lang);
    if let Some(p) = CACHE.lock().get(&key) {
        return Some(p.clone());
    }
    if !BUILDING.lock().insert(key.clone()) {
        return None;
    }
    let made = (|| {
        let list = examples_of(lang);
        let texts: Vec<String> = list.iter().map(|(_, t)| (*t).to_string()).collect();
        let vectors = e.embed_queries(&texts).ok()?;
        if vectors.len() != texts.len() {
            return None;
        }
        Some(Arc::new(
            list.iter()
                .map(|(i, _)| *i)
                .zip(vectors.into_iter().map(unit))
                .collect::<Vec<_>>(),
        ))
    })();
    BUILDING.lock().remove(&key);
    let p = made?;
    CACHE.lock().insert(key, p.clone());
    Some(p)
}

// -------------------------------------------------------------------- ranking

/// The closest intent, how close, and the lead over the best intent that asks for another action.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Scored {
    pub index: usize,
    pub score: f32,
    pub margin: f32,
    /// The closest intent that asks for another action, when there is one.
    pub second: Option<usize>,
}

/// How far a score may be trusted. The trace keeps the gate that applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Gate {
    /// Strong and clear: a chat or recipe lane, or a direct call.
    Direct,
    /// Close but not clear: the first agent step chooses between the two closest tools.
    TopTwo,
    /// Not trusted: the rules and the model decide, as if the step had not run.
    None,
}

impl Gate {
    /// The suffix written after the intent name in the trace; a direct call and no gate have none
    /// (they differ by `intent_used`).
    pub fn suffix(self) -> &'static str {
        match self {
            Gate::TopTwo => "|top-two",
            Gate::Direct | Gate::None => "",
        }
    }
}

impl Scored {
    pub fn intent(&self) -> &'static Intent {
        &intents()[self.index]
    }

    /// The match is strong and clear enough to route by.
    pub fn accepted(&self) -> bool {
        let extra = if self.intent().target == Target::Recipe {
            RECIPE_EXTRA
        } else {
            0.0
        };
        self.score >= SCORE_MIN + extra && self.margin >= MARGIN_MIN
    }

    pub fn gate(&self) -> Gate {
        if self.accepted() {
            Gate::Direct
        } else if self.score >= BAND_MIN {
            Gate::TopTwo
        } else {
            Gate::None
        }
    }
}

/// Rank the intents the question can be about (see [`Need`]); the others are not candidates, and so
/// cannot be the runner-up either.
pub(super) fn rank(query: &[f32], protos: &[(usize, Vec<f32>)], have: Have) -> Option<Scored> {
    let list = intents();
    let mut best = vec![f32::NEG_INFINITY; list.len()];
    for (i, v) in protos.iter().filter(|(i, _)| list[*i].possible(have)) {
        let s: f32 = v.iter().zip(query).map(|(a, b)| a * b).sum();
        if s > best[*i] {
            best[*i] = s;
        }
    }
    let (top, score) = best
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, s)| s.is_finite())
        .max_by(|a, b| a.1.total_cmp(&b.1))?;
    let key = list[top].key();
    let (second, second_score) = best
        .iter()
        .copied()
        .enumerate()
        .filter(|(i, s)| s.is_finite() && list[*i].key() != key)
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map_or((None, 0.0), |(i, s)| (Some(i), s));
    Some(Scored {
        index: top,
        score,
        margin: score - second_score,
        second,
    })
}

/// Embed the question and rank it. None when the model cannot embed now.
pub(super) fn classify(e: &dyn Embedder, text: &str, have: Have) -> Option<Scored> {
    let lang = match openframe_ai_checks::detect_language(text) {
        Some(l) => LANGS.iter().copied().find(|x| *x == l),
        None => None,
    };
    let mut protos: Vec<(usize, Vec<f32>)> = prototypes(e, "en")?.as_ref().clone();
    if let Some(l) = lang {
        protos.extend(prototypes(e, l)?.iter().cloned());
    }
    let q = e.embed_queries(&[text.to_string()]).ok()?.pop()?;
    rank(&unit(q), &protos, have)
}

/// `classify` on a worker thread, waiting at most `wait`. A late result still fills the cache.
pub(super) fn classify_within(
    e: Arc<dyn Embedder>,
    text: &str,
    have: Have,
    wait: Duration,
) -> Option<Scored> {
    let text = text.to_string();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(classify(&*e, &text, have));
    });
    rx.recv_timeout(wait).ok().flatten()
}

// ------------------------------------------------------------------ direct calls

static QUOTED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?:"([^"]{2,80})"|\u{201C}([^\u{201D}]{2,80})\u{201D}|(?:^|[\s(])'([^']{2,80})'(?:$|[\s.,;:!?)]))"#,
    )
    .expect("valid pattern")
});

static TRIGGER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(?:mentions?|mentioning|mentioned|about|containing|contains?|including|includes?|featuring|references?|referring\s+to|involves?|involving|word|phrase|for|where)\s+(?:the\s+|a\s+|an\s+|any\s+|some\s+)?(?P<t>[\p{L}][\p{L}'\u{2019}-]*(?:\s+[\p{L}][\p{L}'\u{2019}-]*){0,3})",
    )
    .expect("valid pattern")
});

/// Words that end a noun phrase: "the old radio appears in" stops at "appears".
const PHRASE_END: &[&str] = &[
    "appear",
    "appears",
    "appearing",
    "come",
    "comes",
    "coming",
    "occur",
    "occurs",
    "is",
    "are",
    "was",
    "were",
    "show",
    "shows",
    "happen",
    "happens",
    "happening",
    "in",
    "on",
    "at",
    "of",
    "from",
    "and",
    "or",
    "but",
    "to",
    "scene",
    "scenes",
    "script",
    "screenplay",
    "film",
    "movie",
    "story",
    "draft",
    "project",
    "please",
    "used",
    "use",
    "uses",
    "take",
    "takes",
    "taking",
    "get",
    "gets",
    "exist",
    "exists",
    "up",
    "out",
];

/// Not a thing to look for.
const GENERIC: &[&str] = &[
    "anything",
    "something",
    "everything",
    "it",
    "them",
    "this",
    "that",
    "those",
    "these",
    "characters",
    "character",
    "people",
    "scenes",
    "scene",
    "lines",
    "dialogue",
    "words",
    "me",
    "us",
    "you",
];

/// What to look for in a "mentions ..." question: a quoted phrase, else the short noun phrase after
/// a word such as "mention", "about" or "for". None when neither is clear.
pub(super) fn search_term(text: &str) -> Option<String> {
    if let Some(c) = QUOTED.captures(text) {
        let q = (1..=3).find_map(|g| c.get(g)).map(|m| m.as_str().trim())?;
        return (!q.is_empty()).then(|| q.to_string());
    }
    let c = TRIGGER.captures(text)?;
    let words: Vec<&str> = c["t"]
        .split_whitespace()
        .take_while(|w| !PHRASE_END.contains(&w.to_lowercase().as_str()))
        // "the word storm", "the phrase hidden key": the term is what follows the label.
        .skip_while(|w| matches!(w.to_lowercase().as_str(), "word" | "phrase"))
        .collect();
    let term = words.join(" ");
    let lower = term.to_lowercase();
    (term.chars().count() >= 3 && !GENERIC.contains(&lower.as_str())).then_some(term)
}

/// The call an intent stands for, when code can fill every argument the tool needs. `required` is
/// the tool schema's required list. None leaves the arguments to the model.
pub(super) fn direct_call(
    intent: &Intent,
    understood: Option<&Understood>,
    text: &str,
    required: &[String],
) -> Option<StepCall> {
    let Target::Tool(tool) = &intent.target else {
        return None;
    };
    let characters: Vec<&str> = understood
        .map(|u| {
            u.entities
                .iter()
                .filter(|e| e.kind == EntityKind::Character)
                .map(|e| e.label.as_str())
                .collect()
        })
        .unwrap_or_default();
    let one_scene = understood
        .filter(|u| u.scenes.len() == 1)
        .and_then(|u| u.scenes.first().copied());
    let mut args = serde_json::Map::new();
    let mut needed: Vec<&str> = required.iter().map(String::as_str).collect();
    // dialogue_fact asks about a person for these facts although its schema does not say so.
    if tool == "dialogue_fact"
        && matches!(
            intent.fact.as_deref(),
            Some("blocks" | "words" | "scenes" | "first_scene")
        )
    {
        needed.push("character");
    }
    // The scene is optional in this tool's schema, but the intent is about one scene.
    if tool == "suggest_breakdown" {
        needed.push("sceneNumber");
    }
    for arg in needed {
        let value = match arg {
            "fact" => intent.fact.clone().map(Value::from),
            "scene" | "sceneNumber" => one_scene.map(|n| json!(n)),
            "text" => search_term(text).map(Value::from),
            "question" => Some(Value::from(text.trim())),
            "character" | "name" => match characters[..] {
                [one] => Some(Value::from(one)),
                _ => None,
            },
            // "Scenes where ALL the named characters speak": two or more names the project knows.
            "characters" => (characters.len() >= 2).then(|| json!(characters)),
            _ => None,
        }?;
        args.insert(arg.to_string(), value);
    }
    Some(StepCall {
        tool: tool.clone(),
        arguments: Value::Object(args),
        done: true,
    })
}

#[cfg(test)]
mod tests;
