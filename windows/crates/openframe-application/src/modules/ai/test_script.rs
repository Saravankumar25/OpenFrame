//! Test-only helpers: an in-memory project with the real migrations, and a model that replays one
//! scripted reply per call and records every request. No model, no files.

use rusqlite::Connection;

/// A project database with every real migration applied.
pub(super) fn project() -> Connection {
    let mut c = openframe_persistence::open_in_memory().unwrap();
    openframe_persistence::migrate::apply(&mut c, crate::schema::PROJECT_MIGRATIONS).unwrap();
    c
}

/// A model that replays one scripted reply per call and records every request, for the lane tests.
#[derive(Default)]
pub(super) struct Scripted {
    replies: std::sync::Mutex<std::collections::VecDeque<openframe_domain::AppResult<String>>>,
    seen: std::sync::Mutex<Vec<openframe_ai::ChatRequest>>,
}

impl Scripted {
    pub(super) fn with(replies: &[&str]) -> Self {
        let m = Self::default();
        for r in replies {
            m.replies.lock().unwrap().push_back(Ok((*r).to_string()));
        }
        m
    }
    pub(super) fn calls(&self) -> usize {
        self.seen.lock().unwrap().len()
    }
    pub(super) fn request(&self, i: usize) -> openframe_ai::ChatRequest {
        self.seen.lock().unwrap()[i].clone()
    }
}

impl openframe_ai::InferenceProvider for Scripted {
    fn chat(&self, req: &openframe_ai::ChatRequest) -> openframe_domain::AppResult<String> {
        self.seen.lock().unwrap().push(req.clone());
        self.replies.lock().unwrap().pop_front().unwrap_or_else(|| {
            Err(openframe_domain::AppError::ai(
                "unavailable",
                "The scripted model has no more replies.",
            ))
        })
    }
    fn model_reference(&self) -> String {
        "scripted".into()
    }
    fn capabilities(&self) -> openframe_ai::ProviderCapabilities {
        openframe_ai::ProviderCapabilities {
            context_tokens: 8192,
            languages: None,
            grammar: true,
        }
    }
}
