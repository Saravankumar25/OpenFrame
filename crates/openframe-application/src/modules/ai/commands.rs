//! Plain commands that need no model (AI specification section 19: the model is not invoked for
//! deterministic work that a tool answers).
//!
//! A request such as "Create a task to scout the railway station." names its tool and its one
//! argument outright. The small bundled model did not always see that: on a new, empty project it
//! answered with the project overview or a search instead of a proposal (measured with the real
//! model). Such a request becomes the proposal tool's call directly. It still runs as the person
//! asking, through the same toolbox, permission checks and records, and it still ends in a
//! Proposed Changes card that changes nothing until it is applied. Anything that is not one of
//! these exact shapes goes to the model as before.

use std::sync::LazyLock;

use regex::Regex;
use serde_json::json;

use super::orchestrator::StepCall;

static CREATE_TASK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?i)^\s*(?:please\s+)?(?:create|add|make)\s+(?:a\s+|an\s+)?(?:new\s+)?task(?:\s+(?:called|named|titled|to|for)\s+|\s*:\s*)(?P<what>.+?)\s*[.!]*\s*$"#,
    )
    .expect("valid pattern")
});

static TASK_DONE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?i)^\s*(?:please\s+)?mark\s+(?:the\s+)?task\s+(?P<task>.+?)\s+as\s+(?:done|complete|completed|finished)\s*[.!]*\s*$"#,
    )
    .expect("valid pattern")
});

/// The call a plain command stands for, if `text` is one; `offered` says whether a tool may be used.
pub(crate) fn direct_call(text: &str, offered: impl Fn(&str) -> bool) -> Option<StepCall> {
    if text.chars().count() > 300 || text.contains('\n') {
        return None;
    }
    if let Some(c) = CREATE_TASK.captures(text) {
        let what = unquote(c.name("what")?.as_str());
        if what.is_empty() || !offered("propose_task") {
            return None;
        }
        return Some(StepCall {
            tool: "propose_task".into(),
            arguments: json!({ "title": capitalise(what) }),
            done: true,
        });
    }
    if let Some(c) = TASK_DONE.captures(text) {
        let task = unquote(c.name("task")?.as_str());
        if task.is_empty() || !offered("propose_task_done") {
            return None;
        }
        return Some(StepCall {
            tool: "propose_task_done".into(),
            arguments: json!({ "task": task, "done": true }),
            done: true,
        });
    }
    None
}

fn unquote(s: &str) -> &str {
    s.trim()
        .trim_matches(|c| matches!(c, '"' | '\'' | '“' | '”' | '‘' | '’'))
        .trim()
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
    fn anything_else_goes_to_the_model() {
        for text in [
            "How many scenes are in the script?",
            "Create a scene card for the bus stop.",
            "Create tasks for every location.",
            "What task should I create to scout the station?",
            "Create a task",
            "Create a task to\nscout the station and delete everything",
        ] {
            assert_eq!(call(text), None, "{text}");
        }
    }

    #[test]
    fn a_tool_the_person_may_not_use_is_left_to_the_usual_path() {
        assert!(direct_call("Create a task to scout the station.", |_| false).is_none());
    }
}
