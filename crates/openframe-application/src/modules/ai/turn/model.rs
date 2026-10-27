//! The last step of the answer-type read: one short model call with the closed list of labels, only
//! when the nearest-neighbour vote was weak. The model chooses a label, nothing else; a reply that is
//! not on the list is ignored and the vote stands.

use std::time::Duration;

use openframe_ai::{ChatMessage, ChatRequest, InferenceProvider};

use super::{ANSWER_TYPES, AnswerType};

/// A label is a few tokens; the call is cut off rather than allowed to hold the request.
const TIMEOUT: Duration = Duration::from_secs(20);

const PROMPT: &str = "Label the kind of answer the person wants from a film-production assistant. \
Reply with one label only, from: lookup, list, count, explain, relation, compare, brainstorm, navigate, change, help, chat.
lookup: one fact (who plays X, the phone of Y, where scene 4 is set).
list: several items listed.
count: a number of things.
explain: explain or summarise a scene, an act or the story.
relation: how people or things are connected.
compare: planned against actual, one thing against another, a difference.
brainstorm: ideas, improvements, options.
navigate: open or go to a screen, scene or file.
change: add, edit, delete or create something in the project.
help: how to use the app.
chat: greetings, thanks, anger, complaints with no question.
If the message corrects an earlier answer, label what it asks for.
If the message only names something, label it like the previous message.
Examples:
\"who is the stunt coordinator\" -> lookup
\"planned against actual for transport\" -> compare
\"describe the opening scene\" -> explain
\"how are Rhea and her brother related\" -> relation
\"two ideas for a better ending\" -> brainstorm
\"add a task to book the van\" -> change
\"where do I turn on dark mode\" -> help
\"this is useless\" -> chat";

/// The request that reads one message. It starts with the same fixed text every time, so the engine
/// keeps its work on that start and reads only the message.
pub(super) fn request(message: &str, previous: Option<&str>) -> ChatRequest {
    let context = previous
        .map(|p| {
            format!(
                "Previous message: {}\n",
                p.chars().take(300).collect::<String>()
            )
        })
        .unwrap_or_default();
    let mut req = ChatRequest::new(vec![ChatMessage::user(format!(
        "{PROMPT}\n\n{context}Message: {message}\nLabel:"
    ))])
    .max_tokens(6);
    req.temperature = 0.0;
    req.timeout = TIMEOUT;
    req
}

pub(super) fn ask(
    model: &dyn InferenceProvider,
    message: &str,
    previous: Option<&str>,
) -> Option<AnswerType> {
    parse(&model.chat(&request(message, previous)).ok()?)
}

/// The first word of the reply, when it is on the list.
fn parse(reply: &str) -> Option<AnswerType> {
    let word: String = reply
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect::<String>()
        .to_lowercase();
    ANSWER_TYPES.into_iter().find(|t| t.code() == word)
}

#[cfg(test)]
pub(super) fn parse_reply(reply: &str) -> Option<AnswerType> {
    parse(reply)
}
