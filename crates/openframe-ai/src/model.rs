//! Inference provider boundary (AI spec §26; ADR-0027 and ADR-0028 decision 10): the orchestrator
//! talks to an `InferenceProvider`; swapping the engine never changes data semantics or
//! authorization (AI-AC-025). The provider says what it can do, so the harness and the prompt
//! budget can adapt to it (for example a smaller context window).

use std::sync::Arc;

use openframe_domain::AppResult;

use crate::client::{ChatRequest, Usage};
use crate::manager::AiManager;

/// What an inference provider can do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderCapabilities {
    /// Tokens the context window holds, prompt and answer together.
    pub context_tokens: u32,
    /// Languages the provider declares it supports; `None` when it declares none, in which case
    /// the evaluation harness measures them (ADR-0028 decision 3).
    pub languages: Option<Vec<String>>,
    /// Whether a JSON schema constrains the output (grammar-constrained decoding).
    pub grammar: bool,
}

pub trait InferenceProvider: Send + Sync {
    /// Run one chat completion and return the assistant text (reasoning stripped).
    fn chat(&self, req: &ChatRequest) -> AppResult<String>;
    /// Local model descriptor recorded with AI requests (a bundle and model id).
    fn model_reference(&self) -> String;
    /// What this provider can do.
    fn capabilities(&self) -> ProviderCapabilities;
    /// Engine numbers of the calls since the last time this was asked, for the AI trace. A provider
    /// that has none (an in-process test model) reports nothing.
    fn take_usage(&self) -> Option<Usage> {
        None
    }
}

/// The bundled llama.cpp engine managed by OpenFrame. Every request goes through the manager,
/// which checks the bundled files before the first start.
pub struct LocalRuntimeModel {
    pub(crate) manager: Arc<AiManager>,
    pub(crate) reference: String,
}

impl InferenceProvider for LocalRuntimeModel {
    fn chat(&self, req: &ChatRequest) -> AppResult<String> {
        self.manager.chat(req)
    }
    fn model_reference(&self) -> String {
        self.reference.clone()
    }
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            context_tokens: self.manager.chat_context_tokens(),
            languages: None,
            // llama-server turns the request's JSON schema into a GBNF grammar (client.rs).
            grammar: true,
        }
    }
    fn take_usage(&self) -> Option<Usage> {
        Some(self.manager.take_usage())
    }
}
