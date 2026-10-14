//! Model adapter boundary (AI spec §26): the orchestrator talks to a
//! `ChatModel`; swapping the local model never changes data semantics or
//! authorization (AI-AC-025).

use std::sync::Arc;

use openframe_domain::AppResult;

use crate::client::ChatRequest;
use crate::manager::AiManager;

pub trait ChatModel: Send + Sync {
    /// Run one chat completion and return the assistant text (reasoning stripped).
    fn chat(&self, req: &ChatRequest) -> AppResult<String>;
    /// Local model descriptor recorded with AI requests (a bundle and model id).
    fn model_reference(&self) -> String;
}

/// The bundled llama.cpp engine managed by OpenFrame. Every request goes through the manager,
/// which checks the bundled files before the first start.
pub struct LocalRuntimeModel {
    pub(crate) manager: Arc<AiManager>,
    pub(crate) reference: String,
}

impl ChatModel for LocalRuntimeModel {
    fn chat(&self, req: &ChatRequest) -> AppResult<String> {
        self.manager.chat(req)
    }
    fn model_reference(&self) -> String {
        self.reference.clone()
    }
}
