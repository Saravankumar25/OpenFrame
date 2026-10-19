//! Plain commands that need no model (AI specification section 19: the model is not invoked for
//! deterministic work that a tool answers; Windows phase W6 "recipes").
//!
//! A request such as "Create a task to scout the railway station." names its tool and its
//! arguments outright. The small bundled model did not always see that: the W5 baseline of
//! 2026-10-16 found it answering "Go to scene 5", "How many night scenes are there?" or "Add the
//! red kite to props" with a general search instead of the tool that does exactly that
//! (docs/release/evidence/w5/baseline/). Such a request becomes the tool's call directly. It still
//! runs as the person asking, through the same toolbox, schema validation, permission checks and
//! records, and a proposal still ends in a Proposed Changes card that changes nothing until it is
//! applied. Each shape is narrow on purpose; anything else goes to the model as before.

use std::sync::LazyLock;

use regex::Regex;
use serde_json::{Value, json};

use super::orchestrator::StepCall;

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("valid pattern")
}

static CREATE_TASK: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?(?:create|add|make)\s+(?:a\s+|an\s+)?(?:new\s+)?task(?:\s+(?:called|named|titled|to|for)\s+|\s*:\s*)(?P<what>.+?)\s*[.!]*\s*$"#,
    )
});

static TASK_SHORT: LazyLock<Regex> =
    LazyLock::new(|| re(r#"(?i)^\s*(?:remind\s+me\s+to|task\s*:)\s+(?P<what>.+?)\s*[.!]*\s*$"#));

static TASK_DONE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?mark\s+(?:the\s+)?task\s+(?P<task>.+?)\s+as\s+(?:done|complete|completed|finished)\s*[.!]*\s*$"#,
    )
});

static OPEN_SCENE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?(?:open|show(?:\s+me)?|go\s+to|jump\s+to|take\s+me\s+to|navigate\s+to)\s+scene\s+(?P<n>\d{1,5})\s*[.!]*\s*$"#,
    )
});

static OPEN_WORKSPACE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?(?:open|show(?:\s+me)?|go\s+to|take\s+me\s+to|switch\s+to)\s+(?:the\s+|my\s+)?(?P<w>[a-z][a-z ]{1,30}?)\s*[.!]*\s*$"#,
    )
});

static COUNT_SCENES: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:how\s+many\s+scenes\s+(?:are\s+)?(?:there\s+)?(?:in\s+)?(?:this|the|my)\s+(?:screenplay|script|current\s+draft|draft|film|short)|how\s+many\s+scenes\s+does\s+(?:the|this|my)\s+(?:screenplay|script|draft|film|short)\s+have|what(?:'s|\s+is)\s+the\s+scene\s+count(?:\s+(?:of|in)\s+(?:this|the|my)\s+(?:screenplay|script|current\s+draft|draft))?)\s*\?*\s*$"#,
    )
});

/// The plain question "How many scenes are there?" with no screenplay named, and its usual
/// variants (OpenFrameAI.docx section 2.4: the unqualified question fell through to a text search
/// for the word "scenes" and was answered "Nothing mentions scenes").
static COUNT_SCENES_PLAIN: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:(?:so|ok|okay|and|but|i\s+ask\s+you\s+again)[,\s]+)?(?:how\s+many\s+(?:total\s+)?scenes(?:\s+(?:are\s+there|do\s+(?:we|i)\s+have|in\s+total|total|there\s+are|exist))?(?:\s+(?:in\s+)?(?:the|this|my)\s+(?:screenplay|script|draft|film|movie|story|project))?|(?:what(?:'s|\s+is)\s+the\s+)?(?:total\s+)?(?:number\s+of\s+scenes|scene\s+count)|count\s+(?:the\s+)?scenes)\s*[?.!]*\s*(?:\[[^\]]{0,60}\])?\s*$"#,
    )
});

/// The same questions in the Indian languages the app supports (OpenFrameAI.docx section 5.3).
static COUNT_SCENES_INDIC: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?:ఎన్ని\s*(?:సీన్|సన్నివేశ)|कितने\s*(?:सीन|दृश्य)|எத்தனை\s*(?:காட்சி|சீன்)|ಎಷ್ಟು\s*(?:ದೃಶ್ಯ|ಸೀನ್)|എത്ര\s*(?:രംഗ|സീൻ|സീനു))"#,
    )
});

/// "How many characters are there?" counts the distinct speaking characters of the screenplay.
static COUNT_CHARACTERS: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:how\s+many\s+(?:total\s+)?(?:characters|speaking\s+(?:roles|characters|parts))(?:\s+(?:are\s+there|do\s+(?:we|i)\s+have|in\s+total|total|there\s+are))?(?:\s+(?:in\s+)?(?:the|this|my)\s+(?:screenplay|script|draft|film|movie|story|project))?|(?:what(?:'s|\s+is)\s+the\s+)?(?:total\s+)?(?:number\s+of\s+characters|character\s+count))\s*[?.!]*\s*$"#,
    )
});

static COUNT_CHARACTERS_INDIC: LazyLock<Regex> = LazyLock::new(|| {
    re(r#"(?:ఎన్ని\s*పాత్ర|कितने\s*(?:किरदार|पात्र)|எத்தனை\s*(?:கதாபாத்திர|கேரக்டர்)|ಎಷ್ಟು\s*ಪಾತ್ರ|എത്ര\s*കഥാപാത്ര)"#)
});

/// "Describe Karthik", "Tell me about Karthik", "Who is Karthik?", "What is Karthik like?",
/// "Describe Karthik's character" (OpenFrameAI.docx exhibits E07 and E08).
static DESCRIBE_CHARACTER: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?(?:(?:describe|analy[sz]e|summari[sz]e)\s+(?:the\s+)?(?:character\s+(?:of\s+)?)?(?P<a>[\p{L}][\p{L}'’ .-]{0,40}?)(?:'s|’s)?(?:\s+(?:character|as\s+a\s+character|personality|arc))?|tell\s+me\s+(?:more\s+)?about\s+(?:the\s+character\s+)?(?P<b>[\p{L}][\p{L}'’ .-]{0,40}?)|who\s+is\s+(?:the\s+character\s+)?(?P<c>[\p{L}][\p{L}'’ .-]{0,40}?)|what\s+is\s+(?P<d>[\p{L}][\p{L}'’ .-]{0,40}?)\s+like|(?:character\s+(?:analysis|profile|sketch)\s+(?:of|for)\s+)(?P<e>[\p{L}][\p{L}'’ .-]{0,40}?))\s*[?.!]*\s*$"#,
    )
});

/// Words that make a "describe X" request about something other than one character.
const NOT_A_CHARACTER: &[&str] = &[
    "scene",
    "scenes",
    "story",
    "plot",
    "act",
    "acts",
    "project",
    "script",
    "screenplay",
    "draft",
    "location",
    "locations",
    "film",
    "movie",
    "app",
    "openframe",
    "feature",
    "this",
    "that",
    "it",
    "them",
    "him",
    "her",
    "me",
    "you",
    "the",
    "a",
    "an",
    "main",
    "characters",
    "everyone",
    "theme",
    "themes",
    "ending",
    "climax",
    "opening",
    "beginning",
    "breakdown",
    "schedule",
    "budget",
    "shot",
    "shots",
    "board",
    "vault",
    "idea",
    "ideas",
    "page",
];

/// A character name in a describe request, or None when the words are not one name.
fn character_name(raw: &str) -> Option<String> {
    let name = unquote(raw).trim_end_matches(['\'', '’']).trim();
    let words: Vec<&str> = name.split_whitespace().collect();
    if words.is_empty() || words.len() > 3 {
        return None;
    }
    if words
        .iter()
        .any(|w| NOT_A_CHARACTER.contains(&w.to_lowercase().as_str()))
    {
        return None;
    }
    Some(name.to_string())
}

/// A greeting or thanks on its own. The assistant answers it at once, like a person would,
/// without looking anything up (the owner asked for a friendly chatbot for ordinary talk).
static SMALL_TALK: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?P<g>hi|hello|hey|hiya|good\s+(?:morning|afternoon|evening)|namaste|namaskaram|vanakkam)?(?P<t>thanks|thank\s+you|thank\s+u|thx|great|awesome|perfect|nice|cool|ok\s+thanks|okay\s+thanks)?(?:\s+(?:there|so\s+much|a\s+lot|openframe|assistant))?\s*[!.]*\s*$"#,
    )
});

/// Words that make an English request a question about using OpenFrame. Without one, a question is
/// about the person's own project, and the product guide is not offered: "What is the significance
/// of the transistor radio in the scene?" was answered from the guide (evaluation case
/// poison-en-001; OpenFrameAI.docx exhibit E07).
static PRODUCT_CUE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)\b(?:how\s+(?:do|can|should|would)\s+(?:i|we|you)|how\s+to|where\s+(?:is|are|do|does|can)|is\s+there\s+a\s+way|can\s+(?:i|you)\s+(?:export|import|change|turn|switch|undo|print|share|install)|openframe|open\s+frame|settings?|shortcuts?|keyboard|button|menu|install|download|offline\s+ai|export|import|back\s*up|restore|theme|dark\s+mode|light\s+mode|print|pdf|template|feature|workspace|sidebar|undo|redo|password|licen[cs]e|trial|app)\b"#,
    )
});

/// True when an English request may be about using OpenFrame itself. Requests in other languages
/// are left to the model (the guide answers "not found" when nothing matches).
pub(crate) fn may_be_product_question(text: &str) -> bool {
    openframe_ai_checks::detect_language(text).is_some_and(|l| l != "en")
        || PRODUCT_CUE.is_match(text)
}

/// The reply to small talk, or None when the text is a real request.
static SCENE_N: LazyLock<Regex> = LazyLock::new(|| re(r#"(?i)\bscene\s+(?P<n>\d{1,4})\b"#));

/// Words that make a request a change, not a question: those go to the proposal tools.
static CHANGE_WORDS: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)\b(?:create|add|delete|remove|move|rename|change|edit|write|rewrite|make|insert|schedule|reorder|replace|update|mark|split|merge|omit|restore|open|show|go\s+to|jump)\b"#,
    )
});

/// A question about one numbered scene, answered from that scene's own text (AI context brief,
/// questions 11 to 19): summary, speakers, speaker count, setting, time, first or last line,
/// first dialogue, or whether it mentions something. Classified by meaning words, not by one
/// fixed phrasing, so "Summarize Scene 3", "what happens in scene 3?" and "scene 3 recap" agree.
fn scene_intent(text: &str) -> Option<StepCall> {
    let n: u32 = SCENE_N
        .captures(text)?
        .name("n")?
        .as_str()
        .parse()
        .ok()
        .filter(|n| *n >= 1)?;
    if SCENE_N.find_iter(text).count() > 1 || text.chars().count() > 140 {
        return None;
    }
    let l = text.to_lowercase();
    if let Some(i) = l.find("mention") {
        let rest = l[i..].split_once(' ').map(|(_, r)| r).unwrap_or("");
        let what = rest
            .trim()
            .trim_start_matches("a ")
            .trim_start_matches("an ")
            .trim_start_matches("the ")
            .trim_end_matches(['?', '.', '!'])
            .trim();
        return (!what.is_empty() && !what.contains("scene")).then(|| {
            call(
                "scene_fact",
                json!({ "scene": n, "fact": "mentions", "text": what }),
            )
        });
    }
    // "Open Scene 3" or "Move Scene 3 to Day 2" are not questions about the scene's text.
    if CHANGE_WORDS.is_match(&l) {
        return None;
    }
    let has = |w: &[&str]| w.iter().any(|x| l.contains(x));
    let fact = if has(&[
        "summar",
        "what happens",
        "what happened",
        "recap",
        "describe",
        "tell me about",
        "about scene",
        "explain scene",
    ]) {
        "summary"
    } else if has(&["how many"]) && has(&["character", "people", "speaker", "speak", "talk"]) {
        "speaker_count"
    } else if has(&["who"]) && has(&["speak", "talk", "dialogue", "in scene", "in it"]) {
        "speakers"
    } else if has(&["last line", "final line", "ends with", "last thing"]) {
        "last_line"
    } else if has(&[
        "first dialogue",
        "first says",
        "first spoken",
        "first words",
    ]) {
        "first_dialogue"
    } else if has(&["first line", "opens with", "begins with", "starts with"]) {
        "first_line"
    } else if has(&["where", "location", "take place", "set in", "set at"]) {
        "setting"
    } else if has(&[
        "day or night",
        "night or day",
        "time of day",
        "what time",
        "is it night",
        "is it day",
        " night?",
        " day?",
    ]) {
        "time"
    } else {
        return None;
    };
    Some(call("scene_fact", json!({ "scene": n, "fact": fact })))
}

static DIALOGUE_COUNT: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*how\s+many\s+(?P<what>dialogues?|dialogue\s+blocks?|lines|times|speeches|words)\s+does\s+(?P<c>[\p{L}][\p{L}'’ .-]{0,40}?)\s+(?:have|speak|say|get|talk)(?:\s+in\s+(?:the\s+)?(?:script|screenplay|draft|film|movie|total))?(?:\s+in\s+scene\s+(?P<n>\d{1,4}))?\s*[?.!]*\s*$"#,
    )
});
static DIALOGUE_SCENES: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:(?:which|what|in\s+which)\s+scenes\s+does\s+(?P<a>[\p{L}][\p{L}'’ .-]{0,40}?)\s+(?:speak|talk|have\s+dialogue)(?:\s+in)?|(?:where|when|in\s+which\s+scene)\s+does\s+(?P<b>[\p{L}][\p{L}'’ .-]{0,40}?)\s+first\s+(?:speak|talk))\s*[?.!]*\s*$"#,
    )
});
static DIALOGUE_MOST: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*who\s+(?:has\s+the\s+most\s+(?:dialogue|lines|dialogues)|speaks\s+the\s+most|talks\s+the\s+most|has\s+most\s+dialogue)(?:\s+in\s+scene\s+(?P<n>\d{1,4}))?\s*[?.!]*\s*$"#,
    )
});
static DIALOGUE_FIRST: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*who\s+(?:speaks|talks)\s+first(?:\s+in\s+(?:the\s+)?(?:script|screenplay|film|movie))?\s*[?.!]*\s*$"#,
    )
});

/// Dialogue questions (AI context brief, questions 21 to 27): counted from the screenplay's
/// dialogue blocks, never from how often a name appears in the text.
fn dialogue_intent(text: &str) -> Option<StepCall> {
    if let Some(c) = DIALOGUE_COUNT.captures(text) {
        let name = character_name(c.name("c")?.as_str())?;
        let fact = if c.name("what")?.as_str().eq_ignore_ascii_case("words") {
            "words"
        } else {
            "blocks"
        };
        let mut args = json!({ "character": name, "fact": fact });
        if let Some(n) = c.name("n").and_then(|n| n.as_str().parse::<u32>().ok()) {
            args["scene"] = json!(n);
        }
        return Some(call("dialogue_fact", args));
    }
    if let Some(c) = DIALOGUE_SCENES.captures(text) {
        let (raw, fact) = match (c.name("a"), c.name("b")) {
            (Some(a), _) => (a.as_str(), "scenes"),
            (_, Some(b)) => (b.as_str(), "first_scene"),
            _ => return None,
        };
        let name = character_name(raw)?;
        return Some(call(
            "dialogue_fact",
            json!({ "character": name, "fact": fact }),
        ));
    }
    if let Some(c) = DIALOGUE_MOST.captures(text) {
        let mut args = json!({ "fact": "most" });
        if let Some(n) = c.name("n").and_then(|n| n.as_str().parse::<u32>().ok()) {
            args["scene"] = json!(n);
        }
        return Some(call("dialogue_fact", args));
    }
    DIALOGUE_FIRST
        .is_match(text)
        .then(|| call("dialogue_fact", json!({ "fact": "first_speaker" })))
}

static FOLLOW_SCENE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:and|what\s+about|how\s+about|same\s+for|now)\s+scene\s+(?P<n>\d{1,4})\s*[?.!]*\s*$"#,
    )
});
static FOLLOW_NAME: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:and|what\s+about|how\s+about|same\s+for|now)\s+(?P<c>[\p{L}][\p{L}'’.-]{0,30})\s*[?.!]*\s*$"#,
    )
});
static JUST_NUMBER: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:(?:just|only)\s+(?:give\s+me\s+|tell\s+me\s+)?(?:the\s+)?(?:number|count|answer)|(?:the\s+)?number\s+only)(?:\s*,?\s*(?:please|no\s+explanation))?\s*[?.!]*\s*$"#,
    )
});

/// A short follow-up rewritten into a full question from the previous one (AI context brief,
/// questions 91 and 92): "What about Scene 4?" after "Who speaks in Scene 3?" asks who speaks in
/// Scene 4; "And Ravi?" after "How many dialogue blocks does Meera have?" asks about Ravi.
pub(crate) fn follow_up(text: &str, previous: Option<&str>) -> Option<String> {
    let prev = previous?;
    if let Some(c) = FOLLOW_SCENE.captures(text) {
        let n = c.name("n")?.as_str();
        let m = SCENE_N.captures(prev)?.name("n")?;
        return Some(format!("{}{}{}", &prev[..m.start()], n, &prev[m.end()..]));
    }
    let c = FOLLOW_NAME.captures(text)?;
    let new = c.name("c")?.as_str();
    if NOT_A_CHARACTER.contains(&new.to_lowercase().as_str()) {
        return None;
    }
    // The name the previous question was about, from the same patterns that route it.
    let old = DIALOGUE_COUNT
        .captures(prev)
        .and_then(|c| c.name("c"))
        .or_else(|| {
            DIALOGUE_SCENES
                .captures(prev)
                .and_then(|c| c.name("a").or(c.name("b")))
        })
        .or_else(|| {
            DESCRIBE_CHARACTER
                .captures(prev)
                .and_then(|c| ["a", "b", "c", "d", "e"].iter().find_map(|g| c.name(g)))
        })?;
    Some(format!(
        "{}{}{}",
        &prev[..old.start()],
        new,
        &prev[old.end()..]
    ))
}

/// "Just give me the number": the first number of the previous answer, said on its own.
pub(crate) fn just_the_number(text: &str, previous_answer: Option<&str>) -> Option<String> {
    if !JUST_NUMBER.is_match(text) {
        return None;
    }
    // The counted number, not a number that is part of a name ("Draft 1", "Scene 3", "Day 2").
    const NAMED: &[&str] = &[
        "draft", "scene", "scenes", "act", "day", "episode", "sequence", "page", "shot",
    ];
    let words: Vec<&str> = previous_answer?.split_whitespace().collect();
    words.iter().enumerate().find_map(|(i, w)| {
        let n = w.trim_matches(|c: char| !c.is_ascii_digit());
        let named = i > 0
            && NAMED.contains(
                &words[i - 1]
                    .to_lowercase()
                    .trim_matches(|c: char| !c.is_alphanumeric()),
            );
        (!n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) && !named).then(|| format!("{n}."))
    })
}

pub(crate) fn small_talk(text: &str) -> Option<&'static str> {
    let c = SMALL_TALK.captures(text)?;
    if c.name("t").is_some() {
        Some("You're welcome. Ask me anything about your project whenever you like.")
    } else if c.name("g").is_some() {
        Some(
            "Hello! I can answer questions about your screenplay and production, describe characters, find scenes, and prepare changes for you to review. What would you like to know?",
        )
    } else {
        None
    }
}

static LOCATION_STATS: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:how\s+many\s+(?:(?:night|day|interior|exterior|dawn|dusk|morning|evening)(?:\s+(?:and|or)\s+(?:night|day|interior|exterior))?\s+scenes|(?:different\s+)?locations)(?:\s+(?:are\s+there|does\s+(?:the|this)\s+(?:script|screenplay|short|film)\s+(?:have|use)|(?:are\s+)?in\s+(?:the|this)\s+(?:script|screenplay)))?|which\s+location\s+is\s+used\s+(?:the\s+)?most|what\s+locations\s+are\s+(?:in|used\s+in)\s+(?:the|this)\s+(?:script|screenplay))\s*\?*\s*$"#,
    )
});

static WHO_SPEAKS: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:who\s+speaks(?:\s+in\s+(?:this|the)\s+(?:scene|script|screenplay))?|(?:list|show)\s+(?:me\s+)?the\s+characters\s+who\s+speak|which\s+characters\s+(?:have\s+dialogue|speak)|who\s+are\s+the\s+speaking\s+characters|how\s+many\s+(?:different\s+)?characters\s+speak)\s*[?.]*\s*$"#,
    )
});

static SCENES_WITH: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:find\s+(?:the\s+)?scenes\s+where|(?:in\s+)?which\s+scenes\s+(?:do|does)|scenes\s+where|where\s+do)\s+(?P<a>[^,?]+?)\s+and\s+(?P<b>[^,?]+?)\s+(?:both\s+)?(?:speak|talk|have\s+lines|have\s+dialogue)(?:\s+to\s+each\s+other)?\s*[?.!]*\s*$"#,
    )
});

static SCENES_HAVE_BOTH: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*which\s+scenes\s+have\s+both\s+(?P<a>[^,?]+?)\s+and\s+(?P<b>[^,?]+?)\s+(?:speaking|talking)\s*[?.!]*\s*$"#,
    )
});

static CATALOG_TO: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?add\s+(?P<what>.+?)\s+to\s+(?:the\s+)?(?P<cat>props|wardrobe|vehicles|animals|special\s+effects|vfx|sound|hair\s+and\s+makeup|extras)\s*[.!]*\s*$"#,
    )
});

static CATALOG_AS: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?(?:add|catalog(?:ue)?)\s+(?P<what>.+?)\s+as\s+(?:a\s+|an\s+)?(?P<cat>prop|wardrobe|costume|vehicle|animal|special\s+effect|sound\s+effect)\s*[.!]*\s*$"#,
    )
});

static CHARACTER: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?(?:add|create|make)\s+(?:a\s+)?(?:new\s+)?character(?:\s+(?:called|named)\s+|\s*:\s*)(?P<name>(?:(?:dr|mr|mrs|ms|prof)\.\s+)?[^,.]+?)(?:\s*,\s*(?P<desc>.+?))?\s*[.!]*\s*$"#,
    )
});

static CHARACTER_AS: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?add\s+(?P<name>[^,.]+?)\s+as\s+a\s+character(?:\s*,\s*(?P<desc>.+?))?\s*[.!]*\s*$"#,
    )
});

static REPLACE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?(?:replace|change)\s+(?:every\s+|all\s+)?['"‘“](?P<find>[^'"’”]{1,200})['"’”]\s+(?:with|to|by)\s+['"‘“](?P<replace>[^'"’”]{1,200})['"’”](?:\s+(?:throughout|everywhere|in)\b[^.!?]*)?\s*[.!]*\s*$"#,
    )
});

static NOTE_THAT: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?(?:make\s+a\s+note\s+that|remember\s+in\s+a\s+note\s+that|note\s+to\s+self\s*:|write\s+a\s+(?:project\s+)?note\s*:)\s+(?P<body>.+?)\s*[.!]*\s*$"#,
    )
});

static NOTE_TITLED: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?(?:save|make|create|add)\s+a\s+note\s+(?:titled|called|named)\s+(?P<title>.+?)\s+with\s+the\s+body\s*:\s*(?P<body>.+?)\s*[.!]*\s*$"#,
    )
});

static MOODBOARD: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?(?:create|make|add)\s+a\s+(?:new\s+)?mood\s?board\s+(?:called|named)\s+(?P<name>.+?)\s*[.!]*\s*$"#,
    )
});

static SCHEDULE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?(?:create|make)\s+a\s+(?:new\s+)?(?:shooting\s+)?schedule\s+(?:called|named)\s+(?P<name>.+?)\s*[.!]*\s*$"#,
    )
});

static STORYBOARD: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?(?:create|make)\s+a\s+(?:new\s+)?storyboard\s+for\s+scene\s+(?P<n>\d{1,5})\s*[.!]*\s*$"#,
    )
});

static LOCATION: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?add\s+(?:a\s+)?(?:new\s+)?location(?:\s+(?:called|named)\s+|\s*:\s*)(?P<name>.+?)\s*[.!]*\s*$"#,
    )
});

static PAGES: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:how\s+many\s+pages\s+(?:is|are\s+in|does)\s+(?:the|this|my)\s+(?:screenplay|script)(?:\s+have)?|what\s+is\s+the\s+page\s+(?:setup|count))\s*\?*\s*$"#,
    )
});

static MENTIONS: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*which\s+scenes\s+(?:mention|include|feature|refer\s+to|take\s+place\s+(?:at|in)|happen\s+(?:at|in)|are\s+set\s+(?:at|in))\s+(?:the\s+)?(?P<x>[^?]{1,120}?)\s*\?*\s*$"#,
    )
});

/// Plain questions that one read tool answers as a whole.
static PLAIN_READS: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    [
        (r#"(?i)^\s*(?:what(?:'s|\s+is)\s+in|show(?:\s+me)?)\s+(?:the\s+|my\s+)?idea\s+vault\s*[?.]*\s*$"#, "idea_vault"),
        (r#"(?i)^\s*(?:show|list)(?:\s+me)?\s+(?:the\s+|my\s+)?project\s+notes\s*[?.]*\s*$"#, "project_notes"),
        (r#"(?i)^\s*(?:what|which)\s+tasks\s+are\s+(?:still\s+)?(?:open|left|pending|not\s+done)\s*[?.]*\s*$"#, "project_tasks"),
        (r#"(?i)^\s*(?:what\s+(?:has\s+)?changed\s+recently|(?:show(?:\s+me)?|what\s+is)\s+(?:the\s+)?recent\s+activity)\s*[?.]*\s*$"#, "recent_activity"),
        (r#"(?i)^\s*(?:are\s+there\s+any|show(?:\s+me)?(?:\s+the)?|list(?:\s+the)?)\s+comments\s*[?.]*\s*$"#, "comments"),
        (r#"(?i)^\s*(?:which|what)\s+templates\s+(?:can\s+i\s+use|are\s+there|do\s+i\s+have)\s*[?.]*\s*$"#, "templates"),
        (r#"(?i)^\s*(?:(?:which|what)\s+drafts\s+(?:does|do)\s+(?:this|the|my)\s+(?:screenplay|script)\s+have|what\s+is\s+the\s+current\s+draft\s+called|(?:list|show)(?:\s+me)?\s+(?:the\s+)?drafts)\s*[?.]*\s*$"#, "list_drafts"),
        (r#"(?i)^\s*(?:list|show)(?:\s+me)?\s+(?:all\s+)?the\s+scenes\s+(?:of|in)\s+(?:the|this|my)\s+(?:screenplay|script)\s*[?.]*\s*$"#, "screenplay_scenes"),
        (r#"(?i)^\s*(?:list|show)(?:\s+me)?\s+(?:all\s+)?the\s+characters\s+on\s+the\s+story\s?board\s*[?.]*\s*$"#, "story_characters"),
        (r#"(?i)^\s*how\s+many\s+(?:props|costumes|vehicles|items)\s+are\s+(?:there\s+)?in\s+the\s+(?:production\s+)?catalog(?:ue)?\s*[?.]*\s*$"#, "production_statistics"),
        (r#"(?i)^\s*how\s+many\s+(?:acts|scene\s+cards|acts\s+and\s+scene\s+cards)\s+are\s+(?:there\s+)?on\s+the\s+story\s?board\s*[?.]*\s*$"#, "story_statistics"),
        (r#"(?i)^\s*how\s+far\s+along\s+is\s+the\s+breakdown\s*[?.]*\s*$"#, "breakdown_status"),
        // Plain wording of the same questions (AI context brief, questions 3, 4 and 31).
        (r#"(?i)^\s*how\s+many\s+(?:acts|sequences|beats|scene\s+cards)(?:\s+are\s+there|\s+do\s+(?:we|i)\s+have)?(?:\s+in\s+(?:the|this|my)\s+(?:story|project))?\s*[?.]*\s*$"#, "story_statistics"),
        (r#"(?i)^\s*how\s+many\s+drafts(?:\s+are\s+there|\s+do\s+(?:we|i)\s+have|\s+does\s+(?:the|this|my)\s+(?:screenplay|script)\s+have)?\s*[?.]*\s*$"#, "list_drafts"),
        (r#"(?i)^\s*(?:which|what)\s+(?:is\s+the\s+)?current\s+draft\s*[?.]*\s*$"#, "list_drafts"),
        (r#"(?i)^\s*how\s+many\s+(?:scenes\s+are\s+(?:omitted|marked\s+omitted)|omitted\s+scenes(?:\s+are\s+there)?)\s*[?.]*\s*$"#, "count_scenes"),
    ]
    .into_iter()
    .map(|(p, t)| (re(p), t))
    .collect()
});

/// "How do I export a PDF?": a question about OpenFrame itself, recognised only when it names a
/// part of the product, so "How do I make Lakshmi kinder?" still goes to the model.
static PRODUCT: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*how\s+(?:do\s+i|can\s+i|does)\s+.*\b(?:pdf|export|import|undo|redo|backup|back\s+up|restore|package|print|template|call\s+sheet|lock|unlock|password|settings|theme|shortcut|recently\s+deleted|fountain|fdx|docx|offline\s+ai|download)\b.*\?\s*$"#,
    )
});

static SCENE_CARD: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?(?:(?:add|create|make)\s+a\s+(?:new\s+)?|new\s+)scene\s+card\s*(?::|where|in\s+which|about)\s*(?P<d>.+?)\s*[.!]*\s*$"#,
    )
});

static RENAME: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?rename\s+(?:the\s+character\s+)?(?P<a>[^,.]{1,80}?)\s+to\s+(?P<b>[^,.]{1,80}?)\s*[.!]*\s*$"#,
    )
});

static COMMENT_SCENE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?(?:comment\s+on|leave\s+a\s+comment\s+on|add\s+a\s+comment\s+(?:to|on))\s+scene\s+(?P<n>\d{1,5})\s*:\s*(?P<t>.+?)\s*$"#,
    )
});

static VAULT_QUOTE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?(?:put|add|save)\s+(?:a\s+)?quote\s+(?:in|to|into)\s+the\s+idea\s+vault\s*:\s*(?P<t>.+?)\s*$"#,
    )
});

static VAULT_IDEA: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?(?:save\s+this\s+idea|add\s+an?\s+idea\s+to\s+the\s+(?:idea\s+)?vault)\s*:\s*(?P<t>.+?)\s*$"#,
    )
});

static PROJECT_STATUS: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?(?:mark|set)\s+the\s+project(?:\s+status)?\s+(?:as|to)\s+(?:in\s+)?(?P<s>[a-z][a-z -]{1,30}?)\s*[.!]*\s*$"#,
    )
});

static TIME_NOTE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?set\s+the\s+time\s+note\s+of\s+scene\s+(?P<n>\d{1,5})\s+to\s+['"‘“]?(?P<t>[^'"’”]{1,80}?)['"’”]?\s*[.!]*\s*$"#,
    )
});

static PROPS_FOR: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*what\s+(?:props|elements|breakdown\s+elements)\s+(?:might|does|would|will)\s+scene\s+(?P<n>\d{1,5})\s+need\s*\?*\s*$"#,
    )
});

static NEW_DRAFT: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?(?:create|make|start)\s+(?:a\s+copy\s+of\s+the\s+current\s+draft|a\s+new\s+draft)\s+(?:named|called)\s+(?P<name>.+?)\s*[.!]*\s*$"#,
    )
});

static NEW_SCENE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?add\s+a\s+(?:new\s+)?scene\s+(?:at\s+the\s+end|after\s+scene\s+(?P<n>\d{1,5}))\s*:\s*(?P<h>(?:INT|EXT|INT\./EXT|I/E)\.?\s+.+?)\s*\.?\s*$"#,
    )
});

static SCENE_CARD_LOT: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?add\s+a\s+scene\s+card\s+['"‘“](?P<d>[^'"’”]{1,200})['"’”]\s+to\s+the\s+parking\s+lot\s*[.!]*\s*$"#,
    )
});

static ACT: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?(?:add|create|make)\s+an?\s+(?:new\s+)?act\s+(?:called|named|titled)\s+(?P<t>.+?)\s*[.!]*\s*$"#,
    )
});

static RENAME_DRAFT: LazyLock<Regex> = LazyLock::new(|| {
    re(r#"(?i)^\s*(?:please\s+)?rename\s+the\s+current\s+draft\s+to\s+(?P<n>.+?)\s*[.!]*\s*$"#)
});

static PRIVATE_NOTE: LazyLock<Regex> = LazyLock::new(|| {
    re(r#"(?i)^\s*(?:please\s+)?(?:add|write)\s+a\s+private\s+note\s*:\s*(?P<t>.+?)\s*$"#)
});

static CAST: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?add\s+a\s+cast\s+member\s*:\s*(?P<p>[^,]{1,80}?)\s+(?:plays|as)\s+(?P<c>[^,]{1,80}?)\s*[.!]*\s*$"#,
    )
});

/// The project status a plain phrase means (the values of propose_project_status).
fn project_status(phrase: &str) -> Option<&'static str> {
    let p: String = phrase
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .collect();
    Some(match p.as_str() {
        "idea" => "Idea",
        "development" => "Development",
        "writing" => "Writing",
        "rewrite" | "rewriting" => "Rewrite",
        "shootingdraft" => "Shooting Draft",
        "preproduction" => "Pre-Production",
        "shootpreparation" => "Shoot Preparation",
        "shooting" | "production" => "Shooting",
        _ => return None,
    })
}

/// Round 2 (W6): the shapes the first harness rerun still found going to a general search.
fn recipe_more(text: &str) -> Option<StepCall> {
    let cap = |r: &Regex| r.captures(text);
    if let Some(c) = cap(&MENTIONS) {
        let x = unquote(c.name("x")?.as_str());
        return (!x.is_empty()).then(|| call("find_scenes_mentioning", json!({ "text": x })));
    }
    if let Some((_, tool)) = PLAIN_READS.iter().find(|(r, _)| r.is_match(text)) {
        return Some(call(tool, json!({})));
    }
    if cap(&PRODUCT).is_some() {
        return Some(call(
            "answer_product_question",
            json!({ "question": text.trim() }),
        ));
    }
    if let Some(c) = cap(&PROPS_FOR) {
        let n: u32 = c.name("n")?.as_str().parse().ok().filter(|n| *n >= 1)?;
        return Some(call("suggest_breakdown", json!({ "sceneNumber": n })));
    }
    if let Some(c) = cap(&SCENE_CARD) {
        let d = c.name("d")?.as_str().trim();
        // A card placed in an act or sequence needs that placement: the model fills it.
        let placed = d
            .to_lowercase()
            .split_whitespace()
            .any(|w| matches!(w, "act" | "sequence"));
        return (!d.is_empty() && !placed).then(|| {
            call(
                "propose_scene_card",
                json!({ "description": capitalise(d) }),
            )
        });
    }
    if let Some(c) = cap(&SCENE_CARD_LOT) {
        let d = c.name("d")?.as_str().trim();
        return Some(call("propose_scene_card", json!({ "description": d })));
    }
    if let Some(c) = cap(&ACT) {
        let t = unquote(c.name("t")?.as_str());
        return (!t.is_empty()).then(|| call("propose_act", json!({ "title": t })));
    }
    if let Some(c) = cap(&RENAME_DRAFT) {
        let n = unquote(c.name("n")?.as_str());
        return (!n.is_empty()).then(|| {
            call(
                "propose_rename_draft",
                json!({ "draft": "current", "name": n }),
            )
        });
    }
    if let Some(c) = cap(&PRIVATE_NOTE) {
        let t = c.name("t")?.as_str().trim();
        return (!t.is_empty())
            .then(|| call("propose_private_note", json!({ "text": capitalise(t) })));
    }
    if let Some(c) = cap(&COMMENT_SCENE) {
        let n: u32 = c.name("n")?.as_str().parse().ok().filter(|n| *n >= 1)?;
        let t = c.name("t")?.as_str().trim();
        return (!t.is_empty())
            .then(|| call("propose_comment", json!({ "sceneNumber": n, "text": t })));
    }
    if let Some(c) = cap(&VAULT_QUOTE) {
        let t = c.name("t")?.as_str().trim();
        return (!t.is_empty())
            .then(|| call("propose_vault_item", json!({ "type": "quote", "text": t })));
    }
    if let Some(c) = cap(&VAULT_IDEA) {
        let t = c.name("t")?.as_str().trim();
        return (!t.is_empty()).then(|| {
            call(
                "propose_vault_item",
                json!({ "type": "note", "text": capitalise(t) }),
            )
        });
    }
    if let Some(c) = cap(&PROJECT_STATUS) {
        let st = project_status(c.name("s")?.as_str())?;
        return Some(call("propose_project_status", json!({ "status": st })));
    }
    if let Some(c) = cap(&TIME_NOTE) {
        let n: u32 = c.name("n")?.as_str().parse().ok().filter(|n| *n >= 1)?;
        let t = c.name("t")?.as_str().trim();
        return (!t.is_empty()).then(|| {
            call(
                "propose_time_note",
                json!({ "sceneNumber": n, "timeNote": t }),
            )
        });
    }
    if let Some(c) = cap(&NEW_DRAFT) {
        let name = unquote(c.name("name")?.as_str());
        return (!name.is_empty()).then(|| call("propose_new_draft", json!({ "name": name })));
    }
    if let Some(c) = cap(&NEW_SCENE) {
        let heading = c.name("h")?.as_str().trim().trim_end_matches('.').trim();
        let mut args = json!({ "heading": heading });
        if let Some(n) = c.name("n").and_then(|n| n.as_str().parse::<u32>().ok()) {
            args["position"] = json!(n + 1);
        }
        return Some(call("propose_new_scene", args));
    }
    if let Some(c) = cap(&CAST) {
        let person = unquote(c.name("p")?.as_str());
        let character = unquote(c.name("c")?.as_str());
        return (!person.is_empty() && !character.is_empty()).then(|| {
            call(
                "propose_cast_member",
                json!({ "person": person, "characterName": character }),
            )
        });
    }
    if let Some(c) = cap(&RENAME) {
        let from = unquote(c.name("a")?.as_str());
        let mut to = unquote(c.name("b")?.as_str());
        for tail in [
            " across the project",
            " across the script",
            " throughout the project",
            " throughout the script",
            " in the script",
            " in the screenplay",
            " everywhere",
        ] {
            if to.to_lowercase().ends_with(tail) {
                to = to[..to.len() - tail.len()].trim();
            }
        }
        if to.split_whitespace().count() > 3 {
            return None;
        }
        // A draft, board or the project is renamed by other tools; this shape is a character.
        let other = [
            "draft", "project", "board", "schedule", "folder", "act", "sequence",
        ];
        if from.is_empty()
            || to.is_empty()
            || other
                .iter()
                .any(|w| from.to_lowercase().split_whitespace().any(|x| x == *w))
        {
            return None;
        }
        return Some(call(
            "propose_rename_character",
            json!({ "from": from, "to": to }),
        ));
    }
    None
}

/// The workspace a plain name means (`open_workspace`'s values).
fn workspace(name: &str) -> Option<&'static str> {
    let n = name.trim().to_lowercase();
    Some(match n.as_str() {
        "home" | "home page" => "home",
        "idea vault" | "vault" => "vault",
        "story" | "story board" | "storyboard" | "story outline" => "story",
        "screenplay" | "script" | "screenplay editor" => "screenplay",
        "breakdown" => "breakdown",
        "production" => "production",
        "call sheets" | "callsheets" | "call sheet" => "callsheets",
        "files" => "files",
        "notes" | "notes and tasks" | "tasks" => "notes",
        "activity" | "recent activity" => "activity",
        "recently deleted" | "trash" | "bin" => "trash",
        "settings" | "project settings" => "settings",
        _ => return None,
    })
}

/// The Production Catalog category a plain word means.
fn category(word: &str) -> Option<&'static str> {
    let w = word.trim().to_lowercase();
    Some(match w.as_str() {
        "prop" | "props" => "Props",
        "wardrobe" | "costume" => "Wardrobe",
        "vehicle" | "vehicles" => "Vehicles",
        "animal" | "animals" => "Animals",
        "special effect" | "special effects" => "Special Effects",
        "vfx" => "VFX",
        "sound" | "sound effect" => "Sound",
        "hair and makeup" => "Hair / Makeup",
        "extras" => "Extras / Background",
        _ => return None,
    })
}

fn call(tool: &str, arguments: Value) -> StepCall {
    StepCall {
        tool: tool.into(),
        arguments,
        done: true,
    }
}

/// The call a plain command stands for, if `text` is one; `offered` says whether a tool may be used.
pub(crate) fn direct_call(text: &str, offered: impl Fn(&str) -> bool) -> Option<StepCall> {
    if text.chars().count() > 300 || text.contains('\n') {
        return None;
    }
    let c = recipe(text)?;
    offered(&c.tool).then_some(c)
}

fn recipe(text: &str) -> Option<StepCall> {
    // Two requests joined in one ("rename Ravi and add a task") are the model's to plan.
    let lower = text.to_lowercase();
    if [
        " and add ",
        " and create ",
        " and make ",
        " and then ",
        " then ",
    ]
    .iter()
    .any(|j| lower.contains(j))
    {
        return None;
    }
    let cap = |r: &Regex| r.captures(text);
    if let Some(c) = cap(&CREATE_TASK).or_else(|| cap(&TASK_SHORT)) {
        let what = unquote(c.name("what")?.as_str());
        return (!what.is_empty())
            .then(|| call("propose_task", json!({ "title": capitalise(what) })));
    }
    if let Some(c) = cap(&TASK_DONE) {
        let task = unquote(c.name("task")?.as_str());
        return (!task.is_empty())
            .then(|| call("propose_task_done", json!({ "task": task, "done": true })));
    }
    if let Some(c) = cap(&OPEN_SCENE) {
        let n: u32 = c.name("n")?.as_str().parse().ok().filter(|n| *n >= 1)?;
        return Some(call("open_scene", json!({ "sceneNumber": n })));
    }
    if let Some(c) = cap(&STORYBOARD) {
        let n: u32 = c.name("n")?.as_str().parse().ok().filter(|n| *n >= 1)?;
        return Some(call("propose_storyboard", json!({ "sceneNumber": n })));
    }
    if cap(&COUNT_SCENES).is_some()
        || cap(&COUNT_SCENES_PLAIN).is_some()
        || COUNT_SCENES_INDIC.is_match(text)
    {
        return Some(call("count_scenes", json!({})));
    }
    if cap(&WHO_SPEAKS).is_none()
        && (cap(&COUNT_CHARACTERS).is_some() || COUNT_CHARACTERS_INDIC.is_match(text))
    {
        return Some(call("count_characters", json!({})));
    }
    if let Some(c) = dialogue_intent(text) {
        return Some(c);
    }
    if let Some(c) = scene_intent(text) {
        return Some(c);
    }
    if let Some(c) = cap(&DESCRIBE_CHARACTER) {
        let raw = ["a", "b", "c", "d", "e"]
            .iter()
            .find_map(|g| c.name(g))
            .map(|m| m.as_str())
            .unwrap_or("");
        if let Some(name) = character_name(raw) {
            return Some(call("character_profile", json!({ "name": name })));
        }
    }
    if cap(&LOCATION_STATS).is_some() {
        return Some(call("location_statistics", json!({})));
    }
    if cap(&WHO_SPEAKS).is_some() {
        return Some(call("list_characters", json!({})));
    }
    if cap(&PAGES).is_some() {
        return Some(call("screenplay_pages", json!({})));
    }
    if let Some(c) = cap(&SCENES_WITH).or_else(|| cap(&SCENES_HAVE_BOTH)) {
        let a = unquote(c.name("a")?.as_str());
        let b = unquote(c.name("b")?.as_str());
        return (!a.is_empty() && !b.is_empty())
            .then(|| call("scenes_with_characters", json!({ "characters": [a, b] })));
    }
    if let Some(c) = cap(&REPLACE) {
        let find = c.name("find")?.as_str();
        let replace = c.name("replace")?.as_str();
        return (!find.trim().is_empty()).then(|| {
            call(
                "propose_replace_text",
                json!({ "find": find, "replace": replace }),
            )
        });
    }
    if let Some(c) = cap(&CATALOG_TO).or_else(|| cap(&CATALOG_AS)) {
        let cat = category(c.name("cat")?.as_str())?;
        let name = article(unquote(c.name("what")?.as_str()));
        return (!name.is_empty()).then(|| {
            call(
                "propose_catalog_item",
                json!({ "name": name, "category": cat }),
            )
        });
    }
    if let Some(c) = cap(&CHARACTER).or_else(|| cap(&CHARACTER_AS)) {
        let name = unquote(c.name("name")?.as_str());
        if name.is_empty() || name.split_whitespace().count() > 5 {
            return None;
        }
        let mut args = json!({ "name": name });
        if let Some(d) = c
            .name("desc")
            .map(|d| d.as_str().trim())
            .filter(|d| !d.is_empty())
        {
            args["description"] = json!(capitalise(d));
        }
        return Some(call("propose_character", args));
    }
    if let Some(c) = cap(&NOTE_TITLED) {
        let title = unquote(c.name("title")?.as_str());
        let body = c.name("body")?.as_str().trim();
        return (!body.is_empty()).then(|| {
            call(
                "propose_project_note",
                json!({ "title": title, "body": capitalise(body) }),
            )
        });
    }
    if let Some(c) = cap(&NOTE_THAT) {
        let body = c.name("body")?.as_str().trim();
        return (!body.is_empty())
            .then(|| call("propose_project_note", json!({ "body": capitalise(body) })));
    }
    if let Some(c) = cap(&MOODBOARD) {
        let name = unquote(c.name("name")?.as_str());
        return (!name.is_empty()).then(|| call("propose_moodboard", json!({ "name": name })));
    }
    if let Some(c) = cap(&SCHEDULE) {
        let name = unquote(c.name("name")?.as_str());
        return (!name.is_empty()).then(|| call("propose_schedule", json!({ "name": name })));
    }
    if let Some(c) = cap(&LOCATION) {
        let name = unquote(c.name("name")?.as_str());
        return (!name.is_empty()).then(|| call("propose_location", json!({ "name": name })));
    }
    if let Some(c) = recipe_more(text) {
        return Some(c);
    }
    if let Some(c) = cap(&OPEN_WORKSPACE)
        && let Some(w) = workspace(c.name("w")?.as_str())
    {
        return Some(call("open_workspace", json!({ "workspace": w })));
    }
    None
}

fn unquote(s: &str) -> &str {
    s.trim()
        .trim_matches(|c| matches!(c, '"' | '\'' | '“' | '”' | '‘' | '’'))
        .trim()
}

/// "the red kite" names the item "red kite".
fn article(s: &str) -> &str {
    for a in ["the ", "a ", "an ", "The ", "A ", "An "] {
        if let Some(rest) = s.strip_prefix(a) {
            return rest.trim();
        }
    }
    s
}

fn capitalise(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(text: &str) -> Option<(String, serde_json::Value)> {
        direct_call(text, |_| true).map(|c| (c.tool, c.arguments))
    }

    fn tool(text: &str) -> Option<String> {
        call(text).map(|c| c.0)
    }

    #[test]
    fn unqualified_and_indian_language_scene_counts_are_the_count_tool() {
        for text in [
            "How many scenes are there?",
            "how many scenes are there ?",
            "I ask you again, how many scenes are there ?",
            "How many scenes?",
            "how many scenes do we have",
            "number of scenes",
            "What is the scene count?",
            "how many scenes are there ? [from tha fountain file]",
            "ఈ సినిమాలో ఎన్ని సీన్లు ఉన్నాయి?",
            "इस फिल्म में कितने सीन हैं?",
            "இந்த படத்தில் எத்தனை காட்சிகள் உள்ளன?",
        ] {
            assert_eq!(tool(text).as_deref(), Some("count_scenes"), "{text}");
        }
    }

    #[test]
    fn character_counts_and_descriptions_route_to_their_tools() {
        for text in [
            "How many characters are there?",
            "how many characters",
            "number of characters",
            "ఎన్ని పాత్రలు ఉన్నాయి?",
        ] {
            assert_eq!(tool(text).as_deref(), Some("count_characters"), "{text}");
        }
        assert_eq!(
            tool("How many characters speak?").as_deref(),
            Some("list_characters")
        );
        for (text, name) in [
            ("Describe Karthik", "Karthik"),
            ("describe karthik character", "karthik"),
            ("Describe Karthik's character.", "Karthik"),
            ("Tell me about Karthik", "Karthik"),
            ("Who is Vivek?", "Vivek"),
            ("What is Shruthi like?", "Shruthi"),
            ("character analysis of Kaushik", "Kaushik"),
        ] {
            assert_eq!(
                call(text),
                Some(("character_profile".to_string(), json!({ "name": name }))),
                "{text}"
            );
        }
        for text in [
            "Describe the story",
            "Tell me about this scene",
            "Describe the opening",
            "Who is the main character?",
            "Describe the plot of act one",
        ] {
            assert_ne!(tool(text).as_deref(), Some("character_profile"), "{text}");
        }
    }

    #[test]
    fn product_help_is_offered_only_for_questions_about_the_app() {
        for q in [
            "How do I export a PDF?",
            "Where is the theme setting?",
            "Can I undo a delete?",
            "How to lock a draft?",
        ] {
            assert!(may_be_product_question(q), "{q}");
        }
        for q in [
            "What is the significance of the transistor radio in the scene?",
            "Describe Karthik",
            "Why does Meera leave the bus?",
            "Which scenes are at night?",
        ] {
            assert!(!may_be_product_question(q), "{q}");
        }
    }

    #[test]
    fn scene_questions_route_to_the_scene_fact_tool_by_meaning() {
        for (q, fact) in [
            ("Summarize Scene 3.", "summary"),
            ("what happens in scene 3?", "summary"),
            ("Tell me about scene 3", "summary"),
            ("Who speaks in Scene 3?", "speakers"),
            ("How many characters are in Scene 3?", "speaker_count"),
            ("How many speaking characters in Scene 3?", "speaker_count"),
            ("Where is Scene 3 set?", "setting"),
            ("Where does scene 3 take place?", "setting"),
            ("Is Scene 3 day or night?", "time"),
            ("What is the last line in Scene 3?", "last_line"),
            ("What is the first line of scene 3?", "first_line"),
            ("What is the first dialogue in Scene 3?", "first_dialogue"),
        ] {
            assert_eq!(
                call(q),
                Some((
                    "scene_fact".to_string(),
                    json!({ "scene": 3, "fact": fact })
                )),
                "{q}"
            );
        }
        assert_eq!(
            call("Does Scene 3 mention a car?"),
            Some((
                "scene_fact".to_string(),
                json!({ "scene": 3, "fact": "mentions", "text": "car" })
            ))
        );
        for q in [
            "Open Scene 3",
            "Move Scene 3 to Day 2",
            "Delete Scene 3",
            "Compare scene 3 and scene 4",
        ] {
            assert_ne!(tool(q).as_deref(), Some("scene_fact"), "{q}");
        }
    }

    #[test]
    fn dialogue_questions_count_dialogue_blocks() {
        assert_eq!(
            call("How many dialogues does Ravi have?"),
            Some((
                "dialogue_fact".to_string(),
                json!({ "character": "Ravi", "fact": "blocks" })
            ))
        );
        assert_eq!(
            call("How many times does Ravi speak in Scene 3?"),
            Some((
                "dialogue_fact".to_string(),
                json!({ "character": "Ravi", "fact": "blocks", "scene": 3 })
            ))
        );
        assert_eq!(
            call("How many words does Ravi speak?"),
            Some((
                "dialogue_fact".to_string(),
                json!({ "character": "Ravi", "fact": "words" })
            ))
        );
        assert_eq!(
            call("Which scenes does Ravi speak in?"),
            Some((
                "dialogue_fact".to_string(),
                json!({ "character": "Ravi", "fact": "scenes" })
            ))
        );
        assert_eq!(
            call("Where does Ravi first speak?"),
            Some((
                "dialogue_fact".to_string(),
                json!({ "character": "Ravi", "fact": "first_scene" })
            ))
        );
        assert_eq!(
            call("Who has the most dialogue?"),
            Some(("dialogue_fact".to_string(), json!({ "fact": "most" })))
        );
        assert_eq!(
            call("Who speaks first?"),
            Some((
                "dialogue_fact".to_string(),
                json!({ "fact": "first_speaker" })
            ))
        );
    }

    #[test]
    fn follow_ups_reuse_the_previous_question() {
        assert_eq!(
            follow_up("What about Scene 4?", Some("Who speaks in Scene 3?")).as_deref(),
            Some("Who speaks in Scene 4?")
        );
        assert_eq!(
            follow_up("And Meera?", Some("How many dialogues does Ravi have?")).as_deref(),
            Some("How many dialogues does Meera have?")
        );
        assert_eq!(
            follow_up("And Meera?", Some("Describe Ravi")).as_deref(),
            Some("Describe Meera")
        );
        assert_eq!(
            follow_up(
                "What about Scene 4?",
                Some("How many characters are there?")
            ),
            None
        );
        assert_eq!(follow_up("And the story?", Some("Describe Ravi")), None);
        assert_eq!(
            just_the_number(
                "Just give me the number",
                Some("Draft 1 contains 130 scenes.")
            )
            .as_deref(),
            Some("130.")
        );
        assert_eq!(
            just_the_number("Describe Ravi", Some("Draft 1 contains 130 scenes.")),
            None
        );
    }

    #[test]
    fn greetings_and_thanks_get_a_friendly_reply_without_tools() {
        assert!(small_talk("hi").is_some());
        assert!(small_talk("Hello there!").is_some());
        assert!(small_talk("thanks").is_some());
        assert!(small_talk("Thank you so much.").is_some());
        assert!(small_talk("hi, how many scenes are there?").is_none());
        assert!(small_talk("Describe Karthik").is_none());
        assert!(small_talk("").is_none());
    }

    #[test]
    fn a_plain_create_task_request_is_the_task_proposal() {
        for (text, title) in [
            (
                "Create a task to scout the railway station.",
                "Scout the railway station",
            ),
            (
                "create a task to book the bus for the night shoot",
                "Book the bus for the night shoot",
            ),
            (
                "Please add a new task: check the bus timetable!",
                "Check the bus timetable",
            ),
            (
                "Add a task called “Confirm pier permit”.",
                "Confirm pier permit",
            ),
            (
                "Remind me to call the art department.",
                "Call the art department",
            ),
            ("Task: hire a generator", "Hire a generator"),
        ] {
            assert_eq!(
                call(text),
                Some(("propose_task".to_string(), json!({ "title": title }))),
                "{text}"
            );
        }
    }

    #[test]
    fn a_plain_mark_done_request_is_the_done_proposal() {
        assert_eq!(
            call("Mark the task Confirm pier permit as done."),
            Some((
                "propose_task_done".to_string(),
                json!({ "task": "Confirm pier permit", "done": true })
            ))
        );
    }

    #[test]
    fn navigation_is_the_open_tools() {
        assert_eq!(
            call("Jump to scene 12."),
            Some(("open_scene".into(), json!({ "sceneNumber": 12 })))
        );
        assert_eq!(
            call("take me to scene 3"),
            Some(("open_scene".into(), json!({ "sceneNumber": 3 })))
        );
        for (text, w) in [
            ("Open the Idea Vault.", "vault"),
            ("Go to the story board", "story"),
            ("Open Call Sheets.", "callsheets"),
            ("Show my notes and tasks.", "notes"),
            ("Open Recently Deleted.", "trash"),
            ("Take me to Breakdown.", "breakdown"),
        ] {
            assert_eq!(
                call(text),
                Some(("open_workspace".into(), json!({ "workspace": w }))),
                "{text}"
            );
        }
    }

    #[test]
    fn plain_questions_about_the_script_are_their_read_tools() {
        for (text, t) in [
            ("How many scenes are in the script?", "count_scenes"),
            ("What is the scene count of this draft?", "count_scenes"),
            ("How many exterior scenes are there?", "location_statistics"),
            (
                "How many day and night scenes are there?",
                "location_statistics",
            ),
            (
                "How many locations does this film use?",
                "location_statistics",
            ),
            ("Which location is used the most?", "location_statistics"),
            ("Who speaks in this scene?", "list_characters"),
            ("Which characters have dialogue?", "list_characters"),
            ("How many pages is this script?", "screenplay_pages"),
        ] {
            assert_eq!(tool(text).as_deref(), Some(t), "{text}");
        }
        assert_eq!(
            call("In which scenes do Rani and Gopal both speak?"),
            Some((
                "scenes_with_characters".into(),
                json!({ "characters": ["Rani", "Gopal"] })
            ))
        );
        assert_eq!(
            call("Which scenes have both Asha and Bala speaking?"),
            Some((
                "scenes_with_characters".into(),
                json!({ "characters": ["Asha", "Bala"] })
            ))
        );
    }

    #[test]
    fn plain_additions_are_their_proposals() {
        assert_eq!(
            call("Add the brass lamp to props."),
            Some((
                "propose_catalog_item".into(),
                json!({ "name": "brass lamp", "category": "Props" })
            ))
        );
        assert_eq!(
            call("Add Meena's silk saree to wardrobe"),
            Some((
                "propose_catalog_item".into(),
                json!({ "name": "Meena's silk saree", "category": "Wardrobe" })
            ))
        );
        assert_eq!(
            call("Add an auto rickshaw as a vehicle."),
            Some((
                "propose_catalog_item".into(),
                json!({ "name": "auto rickshaw", "category": "Vehicles" })
            ))
        );
        assert_eq!(
            call("Create a character named Inspector Rao, the station's new officer."),
            Some((
                "propose_character".into(),
                json!({ "name": "Inspector Rao", "description": "The station's new officer" })
            ))
        );
        assert_eq!(
            call("Add Lata as a character."),
            Some(("propose_character".into(), json!({ "name": "Lata" })))
        );
        assert_eq!(
            call("Replace 'Madurai' with 'Trichy' throughout the script."),
            Some((
                "propose_replace_text".into(),
                json!({ "find": "Madurai", "replace": "Trichy" })
            ))
        );
        assert_eq!(
            call("Make a note that the bridge scenes moved to Friday."),
            Some((
                "propose_project_note".into(),
                json!({ "body": "The bridge scenes moved to Friday" })
            ))
        );
        assert_eq!(
            call("Save a note titled Rain with the body: book two water tankers."),
            Some((
                "propose_project_note".into(),
                json!({ "title": "Rain", "body": "Book two water tankers" })
            ))
        );
        assert_eq!(
            tool("Create a moodboard called Harbour Dusk.").as_deref(),
            Some("propose_moodboard")
        );
        assert_eq!(
            tool("Create a shooting schedule called Unit B.").as_deref(),
            Some("propose_schedule")
        );
        assert_eq!(
            call("Create a storyboard for scene 7."),
            Some(("propose_storyboard".into(), json!({ "sceneNumber": 7 })))
        );
        assert_eq!(
            call("Add a location: Marina pier."),
            Some(("propose_location".into(), json!({ "name": "Marina pier" })))
        );
    }

    #[test]
    fn round_two_reads_and_proposals() {
        for (text, t) in [
            ("What is in the Idea Vault?", "idea_vault"),
            ("Show my project notes.", "project_notes"),
            ("Which tasks are still open?", "project_tasks"),
            ("What changed recently?", "recent_activity"),
            ("Are there any comments?", "comments"),
            ("Which templates can I use?", "templates"),
            ("Which drafts does this script have?", "list_drafts"),
            ("List the scenes of the script.", "screenplay_scenes"),
            ("Show the characters on the storyboard.", "story_characters"),
            (
                "How many vehicles are in the catalog?",
                "production_statistics",
            ),
            (
                "How many scene cards are on the story board?",
                "story_statistics",
            ),
            ("How far along is the breakdown?", "breakdown_status"),
            ("How many scenes does the screenplay have?", "count_scenes"),
            (
                "How do I import a Fountain file?",
                "answer_product_question",
            ),
        ] {
            assert_eq!(tool(text).as_deref(), Some(t), "{text}");
        }
        assert_eq!(
            call("Which scenes mention the lantern?"),
            Some((
                "find_scenes_mentioning".into(),
                json!({ "text": "lantern" })
            ))
        );
        assert_eq!(
            call("Which scenes take place at the harbour?"),
            Some((
                "find_scenes_mentioning".into(),
                json!({ "text": "harbour" })
            ))
        );
        assert_eq!(
            call("What props might scene 4 need?"),
            Some(("suggest_breakdown".into(), json!({ "sceneNumber": 4 })))
        );
        assert_eq!(
            call("Add a scene card: Ravi misses the last train."),
            Some((
                "propose_scene_card".into(),
                json!({ "description": "Ravi misses the last train" })
            ))
        );
        assert_eq!(
            call("Add an act called Act Four."),
            Some(("propose_act".into(), json!({ "title": "Act Four" })))
        );
        assert_eq!(
            call("Rename the current draft to Shooting Script."),
            Some((
                "propose_rename_draft".into(),
                json!({ "draft": "current", "name": "Shooting Script" })
            ))
        );
        assert_eq!(
            call("Start a new draft called Second Pass."),
            Some(("propose_new_draft".into(), json!({ "name": "Second Pass" })))
        );
        assert_eq!(
            call("Add a private note: I think act two is too long."),
            Some((
                "propose_private_note".into(),
                json!({ "text": "I think act two is too long." })
            ))
        );
        assert_eq!(
            call("Add a scene card 'Venkat repairs the radio' to the parking lot."),
            Some((
                "propose_scene_card".into(),
                json!({ "description": "Venkat repairs the radio" })
            ))
        );
        assert_eq!(
            call("New scene card: Suresh drives to Pedda Revu after the storm."),
            Some((
                "propose_scene_card".into(),
                json!({ "description": "Suresh drives to Pedda Revu after the storm" })
            ))
        );
        assert_eq!(
            call("Rename Ravi to Raghav across the project"),
            Some((
                "propose_rename_character".into(),
                json!({ "from": "Ravi", "to": "Raghav" })
            ))
        );
        assert_eq!(
            call("Rename Gopal to Govind."),
            Some((
                "propose_rename_character".into(),
                json!({ "from": "Gopal", "to": "Govind" })
            ))
        );
        assert_eq!(
            call("Comment on scene 3: is this too long?"),
            Some((
                "propose_comment".into(),
                json!({ "sceneNumber": 3, "text": "is this too long?" })
            ))
        );
        assert_eq!(
            tool("Put a quote in the Idea Vault: the sea keeps no secrets.").as_deref(),
            Some("propose_vault_item")
        );
        assert_eq!(
            call("Mark the project as pre-production."),
            Some((
                "propose_project_status".into(),
                json!({ "status": "Pre-Production" })
            ))
        );
        assert_eq!(
            call("Set the time note of scene 9 to 'Friday 06:10'."),
            Some((
                "propose_time_note".into(),
                json!({ "sceneNumber": 9, "timeNote": "Friday 06:10" })
            ))
        );
        assert_eq!(
            call("Create a copy of the current draft named Festival Cut."),
            Some((
                "propose_new_draft".into(),
                json!({ "name": "Festival Cut" })
            ))
        );
        assert_eq!(
            call("Add a scene after scene 4: INT. TEA STALL - DAY."),
            Some((
                "propose_new_scene".into(),
                json!({ "heading": "INT. TEA STALL - DAY", "position": 5 })
            ))
        );
        assert_eq!(
            call("Add a cast member: Meera plays Selvi."),
            Some((
                "propose_cast_member".into(),
                json!({ "person": "Meera", "characterName": "Selvi" })
            ))
        );
        assert_eq!(
            call("Create a character named Dr. Anand Rao, the village doctor."),
            Some((
                "propose_character".into(),
                json!({ "name": "Dr. Anand Rao", "description": "The village doctor" })
            ))
        );
        assert_eq!(
            tool("Write a project note: check the tide tables.").as_deref(),
            Some("propose_project_note")
        );
    }

    #[test]
    fn round_two_leaves_the_rest_to_the_model() {
        for text in [
            "How do I make Lakshmi more sympathetic?",
            "Rename the draft to Final.",
            "Which scenes would work better at night?",
            "Mark the project as almost done.",
            "Add a scene where Ravi cries.",
            "What is in scene 4?",
            "Create a scene card where Ravi finds the ticket in act one",
            "Rename Ravi to Raghav and add a task to update the cast list",
        ] {
            assert_eq!(call(text), None, "{text}");
        }
    }

    #[test]
    fn anything_else_goes_to_the_model() {
        for text in [
            "How many scenes is Lakshmi in?",
            "Create a scene card for the bus stop.",
            "Create tasks for every location.",
            "What task should I create to scout the station?",
            "Create a task",
            "Create a task to\nscout the station and delete everything",
            "Open the door in scene 4.",
            "Open my heart.",
            "Add some tension to scene 3.",
            "Add the kite to the ending.",
            "Replace the ending with something darker.",
            "Who is the antagonist?",
            "Make a moodboard for the station at night.",
            "Go to the market and buy flowers.",
            "Create a character arc for Ravi.",
        ] {
            assert_eq!(call(text), None, "{text}");
        }
    }

    #[test]
    fn a_tool_the_person_may_not_use_is_left_to_the_usual_path() {
        assert!(direct_call("Create a task to scout the station.", |_| false).is_none());
        assert!(direct_call("Go to scene 4.", |t| t != "open_scene").is_none());
    }
}
