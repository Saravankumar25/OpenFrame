//! The lane router (AI harness rebuild plan, section 5.2).
//!
//! "Code decides, the model writes": which path a request takes is decided here, in code, from the
//! words of the request and what `understand` found in the draft. No model is asked. The choice,
//! the next best choice and how strong the rule was go into the trace (short codes and a number).
//!
//! Order of decisions:
//! 1. Greetings and refusals are answered before this runs (`commands`).
//! 2. Direct: a plain command or exact fact that names its tool (`commands::direct_route`). A
//!    route that only writes a short description (a scene summary, a character profile) steps aside
//!    when the person asked to explain or analyse, so the Chat lane can answer at length. A route
//!    that prepares a change is labelled Action.
//! 3. Action: a clear change verb. The proposal tools prepare a Change Set.
//! 4. Recipe: the whole screenplay, every act, start to end, as an explain, summary or theme question.
//! 5. Chat: a question about named people, scenes or acts, a scene in view, ideas, themes and
//!    critique. Also a question about a scene the draft does not have or a name the project does not
//!    know, because Chat answers those without a model call.
//! 6. Agent: lists, counts, finds, status, product questions and anything unclear.
//!
//! The text read here is the routing copy of the request, with misspellings corrected (`typo`). Where
//! these rules were not sure (the Agent lane, or Chat on a weak rule) the orchestrator lets intent
//! prototypes (`intent`) decide between Chat, Recipe and one tool before the lane runs.

use std::sync::LazyLock;

use regex::Regex;

use super::orchestrator::StepCall;
use super::tools::ToolOutput;
use super::types::AiResultStatus;
use super::understand::{EntityKind, Intent, Understood};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lane {
    Direct,
    Chat,
    Agent,
    Recipe,
    Action,
}

impl Lane {
    /// The short code written to the trace.
    pub(crate) fn code(self) -> &'static str {
        match self {
            Lane::Direct => "direct",
            Lane::Chat => "chat",
            Lane::Agent => "agent",
            Lane::Recipe => "recipe",
            Lane::Action => "action",
        }
    }
}

/// The lane, the next best lane and the strength of the rule (1.0 for a hard rule, 0.5 when the
/// question matched no clear kind).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Decision {
    pub lane: Lane,
    pub runner_up: Lane,
    pub margin: f64,
}

impl Decision {
    fn new(lane: Lane, runner_up: Lane, margin: f64) -> Self {
        Self {
            lane,
            runner_up,
            margin,
        }
    }
}

/// What the Chat and Recipe lanes hand back to the orchestrator. The orchestrator turns it into the
/// stored outcome; `passes` and `failed` are the verifier's numbers for the trace.
pub(crate) struct LaneAnswer {
    pub output: ToolOutput,
    pub status: AiResultStatus,
    pub model_ran: bool,
    pub passes: u32,
    pub failed: Vec<String>,
}

/// Everything the router looks at.
pub(crate) struct Inputs<'a> {
    pub text: &'a str,
    /// The call `direct_route` found, if any.
    pub direct: Option<&'a StepCall>,
    /// None when the project has no screenplay draft to read.
    pub understood: Option<&'a Understood>,
    /// A scene is in view (scope is the current scene).
    pub scene_scope: bool,
    /// The scope is the screenplay or the whole project.
    pub project_scope: bool,
    /// A capitalised name in the request that the project does not know.
    pub unknown_name: Option<&'a str>,
}

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("valid pattern")
}

/// A clear change verb at the start of the request, or as its main verb after a polite opening
/// ("Can you rename ...", "I want you to add ...").
static ACTION: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:(?:please|kindly)[\s,]+)?(?:(?:can|could|would|will)\s+you\s+(?:please\s+)?|i\s+(?:want|need|would\s+like)\s+(?:you\s+)?to\s+|let'?s\s+)?(?:(?:create|add|rename|delete|remove|move|schedule|set|change|mark|assign|attach|duplicate|archive|link)\b|make\s+(?:me\s+)?(?:a|an|the)?\s*(?:new\s+)?(?:task|note)\b)"#,
    )
});

pub(crate) fn is_action(text: &str) -> bool {
    ACTION.is_match(text)
}

/// Words of the production side of the project. A question about these belongs to the tools that
/// read the records, not to a story briefing.
static PRODUCTION: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)\b(?:props?|wardrobe|costumes?|cast(?:ing)?|crew|budget|schedule|shooting\s+days?|call\s+sheets?|tasks?|shots?|shot\s+list|storyboards?|break\s*down|vault|status|versions?|drafts?|notes?|files?|contacts?|permits?|equipment)\b"#,
    )
});

/// A request to find something: "Find every scene that mentions the battery", "Locate the scenes
/// containing the kite". It is a search however wide its words sound ("every scene").
static FIND_REQUEST: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?(?:(?:can|could|would)\s+you\s+)?(?:find|locate|search|look\s+for)\b|\bscenes?\s+(?:(?:that|which|where)\s+)?(?:mentions?|mentioning|contains?|containing|references?)\b"#,
    )
});

/// "Suggest breakdown elements for scene 22": a request for the production tools, not for story ideas.
static SUGGEST_PRODUCTION: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)\bsuggest\b.*\b(?:breakdown|elements?|props?|wardrobe|costumes?|vehicles?|shots?|shot\s+list|schedule|cast|crew|equipment)\b"#,
    )
});

/// The person asked to understand or explain, not only to find.
fn asks_to_explain(text: &str) -> bool {
    let l = text.to_lowercase();
    [
        "explain",
        "walk me through",
        "analy",
        "summari",
        "describe",
        "why ",
    ]
    .iter()
    .any(|w| l.contains(w))
}

/// The direct routes that only write a short description from the story.
fn writes_a_description(call: &StepCall) -> bool {
    call.tool == "character_profile"
        || (call.tool == "scene_fact"
            && call.arguments.get("fact").and_then(|f| f.as_str()) == Some("summary"))
}

/// The person asked for more than a line: "explain", "walk me through", "in detail".
fn wants_long_form(text: &str, u: &Understood) -> bool {
    use super::understand::Depth;
    let l = text.to_lowercase();
    u.depth == Depth::Deep
        || l.contains("explain")
        || l.contains("walk me through")
        || l.contains("walk through")
}

/// Does the request number a scene the draft has ("Scene 4", "scenes 3 to 5")?
fn numbered_scene(text: &str, u: &Understood) -> bool {
    let named = openframe_ai_checks::scene_refs(text);
    u.scenes.iter().any(|n| named.contains(&(*n as u32)))
}

pub(crate) fn decide(i: &Inputs) -> Decision {
    if let Some(call) = i.direct {
        let steps_aside =
            writes_a_description(call) && i.understood.is_some_and(|u| wants_long_form(i.text, u));
        if !steps_aside {
            // A plain command that prepares a change is an Action; the loop treats both alike.
            return if call.tool.starts_with("propose_") {
                Decision::new(Lane::Action, Lane::Direct, 1.0)
            } else {
                Decision::new(Lane::Direct, Lane::Agent, 1.0)
            };
        }
    }
    if is_action(i.text) {
        return Decision::new(Lane::Action, Lane::Agent, 1.0);
    }
    // A search stays a search (the S5 router sent "Find every scene that mentions the battery" to
    // Chat because "every scene" reads as a whole-story question).
    if FIND_REQUEST.is_match(i.text) && !asks_to_explain(i.text) {
        return Decision::new(Lane::Agent, Lane::Chat, 1.0);
    }
    let Some(u) = i.understood else {
        return Decision::new(Lane::Agent, Lane::Chat, 0.5);
    };
    if u.intent == Intent::Ideas && SUGGEST_PRODUCTION.is_match(i.text) {
        return Decision::new(Lane::Agent, Lane::Chat, 1.0);
    }
    let explains = matches!(u.intent, Intent::Explain | Intent::Whole | Intent::Theme);
    if u.whole_range && explains {
        return Decision::new(Lane::Recipe, Lane::Chat, 1.0);
    }
    let chat_intent = matches!(
        u.intent,
        Intent::Explain
            | Intent::Character
            | Intent::Relationship
            | Intent::Compare
            | Intent::Theme
            | Intent::Critique
            | Intent::Ideas
            | Intent::Other
    );
    let has_character = u.entities.iter().any(|e| e.kind == EntityKind::Character);
    // A scene counts when the person numbered it. "This scene" counts for every kind of question
    // but a loose one ("Give me the gist of this scene"), which the Agent lane's scene tools have
    // always answered. A place alone is a search, not a briefing.
    let numbered = numbered_scene(i.text, u);
    let by_phrase = !u.scenes.is_empty() && u.intent != Intent::Other;
    let places_only = !has_character && u.entities.iter().all(|e| e.kind == EntityKind::Location);
    let place_ok = matches!(
        u.intent,
        Intent::Ideas | Intent::Critique | Intent::Theme | Intent::Compare
    );
    let resolved = has_character
        || !u.acts.is_empty()
        || numbered
        || by_phrase
        || (!u.entities.is_empty() && places_only && place_ok);
    let production = matches!(u.intent, Intent::Other | Intent::Explain)
        && !has_character
        && PRODUCTION.is_match(i.text);
    // Chat answers these without a model call: say that the scene or name is not there, or ask
    // which character a pronoun means.
    let abstains = !u.missing_scenes.is_empty()
        || ((i.unknown_name.is_some() || u.pronoun_unresolved) && chat_intent);
    if abstains && !production {
        return Decision::new(Lane::Chat, Lane::Agent, 1.0);
    }
    // A scene in view is enough for the questions that are about that scene by nature.
    let in_view = i.scene_scope
        && matches!(
            u.intent,
            Intent::Character
                | Intent::Relationship
                | Intent::Compare
                | Intent::Theme
                | Intent::Critique
                | Intent::Ideas
        );
    let broad = matches!(u.intent, Intent::Ideas | Intent::Theme | Intent::Critique)
        && (i.scene_scope || i.project_scope);
    if chat_intent && (resolved || in_view || broad) && !production {
        let margin = if u.intent == Intent::Other { 0.5 } else { 1.0 };
        return Decision::new(Lane::Chat, Lane::Agent, margin);
    }
    let hard = matches!(u.intent, Intent::ListOrCount | Intent::Find);
    let runner_up = if resolved { Lane::Chat } else { Lane::Direct };
    Decision::new(Lane::Agent, runner_up, if hard { 1.0 } else { 0.5 })
}

#[cfg(test)]
mod tests;
