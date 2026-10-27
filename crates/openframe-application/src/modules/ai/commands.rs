//! Exact commands and the refusal list (spec `ai-conversational-harness`, section 4.1).
//!
//! A request such as "Create a task to scout the railway station." or "Open scene 42." names its tool
//! and its arguments outright, so the tool is called and the model is not asked. It still runs as the
//! person asking, through the same toolbox, schema validation, permission checks and records, and a
//! proposal still ends in a Proposed Changes card that changes nothing until it is applied. Each shape
//! is narrow on purpose; anything else goes to the model.
//!
//! Questions are not commands. The regex routes that used to answer questions (counts, facts,
//! descriptions, product help) are retired: a question is answered from the project by the answer
//! stage, whatever its wording. What stays here is only the shapes that start a change or open
//! something, and the fixed refusal for the licence, payment and the team password.

use std::sync::LazyLock;

use regex::Regex;
use serde_json::{Value, json};

use super::action::StepCall;

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
        r#"(?i)^\s*(?:please\s+)?(?:open|show(?:\s+me)?|go\s+to|jump\s+to|take\s+me\s+to|switch\s+to)\s+(?:the\s+|my\s+)?(?P<w>[a-z][a-z ]{1,30}?)\s*[.!]*\s*$"#,
    )
});

/// "Change the project licence so I do not need to pay", "Turn off the team password": settings of
/// the program itself. A licence plate or a driving licence in the story is project content.
static PROGRAM_ACCESS: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)\b(?:(?:change|remove|turn\s+off|switch\s+off|disable|bypass|skip|unlock|extend|reset|crack|activate|deactivate|get\s+around|avoid|cancel|edit|delete)\b.{0,40}\b(?:licen[cs]e|licensing|activation|subscription|payment|team\s+password|trial\s+period|free\s+trial)\b|(?:do\s+not|don'?t|never)\s+(?:need|have)\s+to\s+pay|(?:use|get)\s+(?:openframe|the\s+app|it)\s+for\s+free)"#,
    )
});
static STORY_LICENCE: LazyLock<Regex> =
    LazyLock::new(|| re(r#"(?i)\blicen[cs]e\s+plates?\b|\bdriv(?:ing|er'?s)\s+licen[cs]e\b"#));

pub(crate) fn not_the_assistants(text: &str) -> Option<&'static str> {
    (PROGRAM_ACCESS.is_match(text) && !STORY_LICENCE.is_match(text)).then_some(
        "I can't change the licence, payment, activation or the team password. They are not part of your project, and nothing has been changed.",
    )
}

static CATALOG_TO: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?add\s+(?P<what>.+?)\s+to\s+(?:the\s+)?(?P<cat>props|set\s+dressing|wardrobe|vehicles|animals|special\s+effects|vfx|sound|hair\s+and\s+makeup|extras)\s*[.!]*\s*$"#,
    )
});

static CATALOG_AS: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?(?:add|catalog(?:ue)?)\s+(?P<what>.+?)\s+as\s+(?:a\s+|an\s+)?(?P<cat>prop|set\s+dressing|wardrobe|costume|vehicle|animal|special\s+effect|sound\s+effect)\s*[.!]*\s*$"#,
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

static SCENE_N: LazyLock<Regex> = LazyLock::new(|| re(r#"(?i)\bscene\s+(?P<n>\d{1,4})\b"#));

static SHOT_ADD: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?(?:add|create|make)\s+(?:a\s+|an\s+)?(?:new\s+)?shot\s+(?:to|for|in|on)\s+scene\s+(?P<n>\d{1,5})\s*[:,-]\s*(?P<d>.+?)\s*[.!]*\s*$"#,
    )
});

static MOODBOARD_FOR: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?(?:create|make|add)\s+(?:a\s+|an\s+)?(?:new\s+)?mood\s?board\s+(?:for|of|about)\s+(?P<what>.+?)\s*[.!]*\s*$"#,
    )
});

static COMMENT_DRAFT: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?(?:leave|add|write|post)\s+a\s+comment\s+(?:on|to)\s+(?:the\s+|this\s+)?(?:current\s+)?draft\s*[:,-]\s*(?P<t>.+?)\s*$"#,
    )
});

static PIN_NOTE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?pin\s+a\s+(?:project\s+)?note\s*(?:saying|that\s+says|that|:)\s*(?P<body>.+?)\s*[.!]*\s*$"#,
    )
});

static CHARACTERS_FOR_SCENE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?(?:add|create|make)\s+(?:the\s+)?characters?\s+(?:for|in|from)\s+(?:this|the|that)\s+scene\s*[:,-]\s*(?P<name>[^,&]+?)\s*[.!]*\s*$"#,
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

/// "Suggest breakdown elements for scene 22." (the open gold tool set) asks for the same tool.
static SUGGEST_BREAKDOWN: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)^\s*(?:please\s+)?suggest\s+(?:the\s+|some\s+)?(?:breakdown\s+elements|elements|props|breakdown)\s+for\s+scene\s+(?P<n>\d{1,5})\s*[.!?]*\s*$"#,
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
    if let Some(c) = cap(&SUGGEST_BREAKDOWN) {
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
    if let Some(c) = cap(&COMMENT_DRAFT) {
        let t = c.name("t")?.as_str().trim();
        return (!t.is_empty()).then(|| {
            call(
                "propose_comment",
                json!({ "draft": "current", "text": capitalise(t) }),
            )
        });
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

/// The screen a plain name means: `open_workspace`'s workspace and, for a Production tab, its tab.
/// "the project files page", "Budget" and "shooting schedule" all name one.
fn screen(name: &str) -> Option<(&'static str, Option<&'static str>)> {
    let mut n = name.trim().to_lowercase().replace('&', "and");
    for lead in ["project ", "the ", "my "] {
        n = n.strip_prefix(lead).unwrap_or(&n).to_string();
    }
    for tail in [
        " page",
        " screen",
        " tab",
        " workspace",
        " view",
        " section",
        " panel",
    ] {
        n = n.strip_suffix(tail).unwrap_or(&n).to_string();
    }
    let (ws, sub) = match n.trim() {
        "home" | "home page" => ("home", None),
        "idea vault" | "vault" => ("vault", None),
        "story" | "story board" | "beat board" | "beatboard" | "storyboard" | "story outline" => {
            ("story", None)
        }
        "screenplay" | "script" | "screenplay editor" => ("screenplay", None),
        "breakdown" => ("breakdown", None),
        "production" => ("production", None),
        "call sheets" | "callsheets" | "call sheet" => ("callsheets", None),
        "files" | "file" | "project files" => ("files", None),
        "notes" | "notes and tasks" | "tasks" => ("notes", None),
        "activity" | "recent activity" => ("activity", None),
        "recently deleted" | "trash" | "bin" => ("trash", None),
        "settings" | "project settings" => ("settings", None),
        "budget" => ("production", Some("budget")),
        "schedule" | "shooting schedule" | "shoot schedule" => ("production", Some("schedule")),
        "cast" | "crew" | "cast and crew" => ("production", Some("cast-crew")),
        "locations" => ("production", Some("locations")),
        "catalog" | "production catalog" => ("production", Some("catalog")),
        "shots" | "shot list" | "shot lists" => ("production", Some("shots")),
        "moodboards" | "moodboard" => ("production", Some("moodboards")),
        "storyboards" => ("production", Some("storyboards")),
        "day out of days" | "dood" => ("production", Some("dood")),
        "sides" => ("production", Some("sides")),
        "reports" => ("production", Some("reports")),
        _ => return None,
    };
    Some((ws, sub))
}

/// The Production Catalog category a plain word means.
fn category(word: &str) -> Option<&'static str> {
    let w = word.trim().to_lowercase();
    Some(match w.as_str() {
        // The catalog has no Set Dressing category; dressing is catalogued with the props.
        "prop" | "props" | "set dressing" => "Props",
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

/// A command is one short line.
fn is_command_shape(text: &str) -> bool {
    text.chars().count() <= 300 && !text.contains('\n')
}

/// The change a plain command stands for ("Create a task to scout the station"): a proposal tool,
/// or the breakdown suggestion. `offered` says whether the tool may be used. Reads are not commands:
/// a question is answered from the project, whatever its wording.
pub(crate) fn change_command(text: &str, offered: impl Fn(&str) -> bool) -> Option<StepCall> {
    if !is_command_shape(text) {
        return None;
    }
    let c = recipe(text)?;
    let change = c.tool.starts_with("propose_") || c.tool == "suggest_breakdown";
    (change && offered(&c.tool)).then_some(c)
}

/// The screen a plain navigation command opens ("Open scene 42", "Go to the call sheets").
pub(crate) fn navigation_call(text: &str, offered: impl Fn(&str) -> bool) -> Option<StepCall> {
    if !is_command_shape(text) {
        return None;
    }
    let c = if let Some(c) = OPEN_SCENE.captures(text) {
        let n: u32 = c.name("n")?.as_str().parse().ok().filter(|n| *n >= 1)?;
        call("open_scene", json!({ "sceneNumber": n }))
    } else {
        let c = OPEN_WORKSPACE.captures(text)?;
        let (workspace, sub) = screen(c.name("w")?.as_str())?;
        call(
            "open_workspace",
            match sub {
                Some(sub) => json!({ "workspace": workspace, "sub": sub }),
                None => json!({ "workspace": workspace }),
            },
        )
    };
    offered(&c.tool).then_some(c)
}

/// Shapes that name a Production tool outright. They run before the others: "Add a shot to scene 21: ..."
/// is a shot to add.
fn recipe_early(text: &str) -> Option<StepCall> {
    let cap = |r: &Regex| r.captures(text);
    if let Some(c) = cap(&SHOT_ADD) {
        let n: u32 = c.name("n")?.as_str().parse().ok().filter(|n| *n >= 1)?;
        let d = c.name("d")?.as_str().trim();
        return (!d.is_empty()).then(|| {
            call(
                "propose_shot",
                json!({ "sceneNumber": n, "description": capitalise(d) }),
            )
        });
    }
    if let Some(c) = cap(&MOODBOARD_FOR) {
        let what = article(unquote(c.name("what")?.as_str()));
        if what.is_empty() {
            return None;
        }
        let scene = SCENE_N
            .captures(what)
            .filter(|s| s.get(0).is_some_and(|m| m.as_str().len() == what.len()))
            .and_then(|s| s.name("n")?.as_str().parse::<u32>().ok());
        return Some(match scene {
            Some(n) => call(
                "propose_moodboard",
                json!({ "name": format!("Scene {n}"), "sceneNumber": n }),
            ),
            None => call("propose_moodboard", json!({ "name": capitalise(what) })),
        });
    }
    if let Some(c) = cap(&PIN_NOTE) {
        let body = c.name("body")?.as_str().trim();
        return (!body.is_empty()).then(|| {
            call(
                "propose_project_note",
                json!({ "body": capitalise(body), "pinned": true }),
            )
        });
    }
    if let Some(c) = cap(&CHARACTERS_FOR_SCENE) {
        let name = unquote(c.name("name")?.as_str());
        // One character per call; a list of names is the model's to plan.
        let words = name.split_whitespace().count();
        let several = name.to_lowercase().split_whitespace().any(|w| w == "and");
        return (!name.is_empty() && words <= 4 && !several)
            .then(|| call("propose_character", json!({ "name": name })));
    }
    None
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
    if let Some(c) = recipe_early(text) {
        return Some(c);
    }
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
    if let Some(c) = cap(&STORYBOARD) {
        let n: u32 = c.name("n")?.as_str().parse().ok().filter(|n| *n >= 1)?;
        return Some(call("propose_storyboard", json!({ "sceneNumber": n })));
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
        let raw_cat = c.name("cat")?.as_str();
        let cat = category(raw_cat)?;
        let (name, quantity) = split_quantity(article(unquote(c.name("what")?.as_str())));
        if name.is_empty() {
            return None;
        }
        let mut args = json!({ "name": name, "category": cat });
        // The catalog has no quantity field: a count the person gave goes in the notes.
        if let Some(n) = quantity {
            args["notes"] = json!(format!("Quantity: {n}"));
        }
        if raw_cat.to_lowercase().contains("dressing") {
            args["description"] = json!("Set dressing");
        }
        return Some(call("propose_catalog_item", args));
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
    None
}

/// "three desks" is the item "desks" and a count of 3. Only a count of two or more is split off
/// ("a", "one" and "the red kite" are part of the name).
fn split_quantity(what: &str) -> (&str, Option<u32>) {
    let Some((first, rest)) = what.split_once(char::is_whitespace) else {
        return (what, None);
    };
    let n = match first.to_lowercase().as_str() {
        "two" => 2,
        "three" => 3,
        "four" => 4,
        "five" => 5,
        "six" => 6,
        "seven" => 7,
        "eight" => 8,
        "nine" => 9,
        "ten" => 10,
        digits => match digits.parse::<u32>() {
            Ok(n) if (2..=999).contains(&n) => n,
            _ => return (what, None),
        },
    };
    (rest.trim(), Some(n))
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
mod tests;
