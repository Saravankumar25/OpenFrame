//! What is made ready when a project opens, so the first question does not pay for it: the vectors
//! of the labelled example messages, and the engine's work on the fixed start of each prompt the
//! turn sends. Nothing here changes what any prompt says.

use openframe_ai::ChatRequest;
use openframe_search::Embedder;

use super::{model, neighbour};

/// The fixed prompt starts the engine reads once ahead of the first question, one token asked of
/// each. A prompt of the writer joins this list when its fixed start is written (spec section 9,
/// item 1); the answer text of a prompt is thrown away.
pub fn prompts() -> Vec<ChatRequest> {
    let mut label = model::request("hello", None);
    label.max_tokens = 1;
    vec![label]
}

/// Embed the example messages with `e` now. They are kept per search model, so this happens once;
/// the first answer-type read then finds them. True when the vectors are ready.
pub fn examples(e: &dyn Embedder) -> bool {
    neighbour::prototypes(e).is_some()
}
