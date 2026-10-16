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
        r#"(?i)^\s*(?:how\s+many\s+scenes\s+(?:are\s+)?(?:there\s+)?(?:in\s+)?(?:this|the|my)\s+(?:screenplay|script|current\s+draft|draft|film|short)|what(?:'s|\s+is)\s+the\s+scene\s+count(?:\s+(?:of|in)\s+(?:this|the|my)\s+(?:screenplay|script|current\s+draft|draft))?)\s*\?*\s*$"#,
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
        r#"(?i)^\s*(?:please\s+)?(?:add|create|make)\s+(?:a\s+)?(?:new\s+)?character(?:\s+(?:called|named)\s+|\s*:\s*)(?P<name>[^,.]+?)(?:\s*,\s*(?P<desc>.+?))?\s*[.!]*\s*$"#,
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
        r#"(?i)^\s*(?:please\s+)?(?:make\s+a\s+note\s+that|remember\s+in\s+a\s+note\s+that|note\s+to\s+self\s*:)\s+(?P<body>.+?)\s*[.!]*\s*$"#,
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
