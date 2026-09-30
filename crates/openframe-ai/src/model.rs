//! Model adapter boundary (AI spec §26): the orchestrator talks to a
//! `ChatModel`; swapping the local model never changes data semantics or
//! authorization (AI-AC-025).

use std::sync::Arc;
use std::time::Duration;

use openframe_domain::AppResult;

use crate::client::{self, ChatRequest};
use crate::rt::AiRuntime;
use crate::supervisor::Supervisor;

pub trait ChatModel: Send + Sync {
    /// Run one chat completion and return the assistant text (reasoning stripped).
    fn chat(&self, req: &ChatRequest) -> AppResult<String>;
    /// Local model descriptor recorded with AI requests (a profile id).
    fn model_reference(&self) -> String;
}

/// The OpenFrame-managed llama.cpp sidecar.
pub struct LocalRuntimeModel {
    pub(crate) supervisor: Arc<Supervisor>,
    pub(crate) rt: Arc<AiRuntime>,
    pub(crate) reference: String,
    pub(crate) ready_timeout: Duration,
}

impl ChatModel for LocalRuntimeModel {
    fn chat(&self, req: &ChatRequest) -> AppResult<String> {
        let ep = self.supervisor.ensure_ready(self.ready_timeout)?;
        let _busy = self.supervisor.begin_request();
        let http = self.supervisor.http().clone();
        let req = req.clone();
        self.rt
            .run(async move { client::chat(&http, &ep, &req).await })
    }
    fn model_reference(&self) -> String {
        self.reference.clone()
    }
}
