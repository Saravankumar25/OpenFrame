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
        r#"(?i)^\s*(?:please\s+)?(?:add|create|make)\s+a\s+(?:new\s+)?scene\s+card\s*(?::|where|in\s+which|about)\s*(?P<d>.+?)\s*[.!]*\s*$"#,
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
        r#"(?i)^\s*(?:please\s+)?(?:create|make)\s+(?:a\s+copy\s+of\s+the\s+current\s+draft|a\s+new\s+draft)\s+(?:named|called)\s+(?P<name>.+?)\s*[.!]*\s*$"#,
    )
});

static NEW_SCENE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?add\s+a\s+(?:new\s+)?scene\s+(?:at\s+the\s+end|after\s+scene\s+(?P<n>\d{1,5}))\s*:\s*(?P<h>(?:INT|EXT|INT\./EXT|I/E)\.?\s+.+?)\s*\.?\s*$"#,
    )
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
    if cap(&COUNT_SCENES).is_some() {
        return Some(call("count_scenes", json!({})));
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
