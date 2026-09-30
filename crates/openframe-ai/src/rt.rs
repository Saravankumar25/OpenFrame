//! A small private tokio runtime for AI networking (downloads, loopback chat).
//!
//! The application layer is synchronous (Tauri calls it from blocking worker
//! threads). `run` executes a future on this runtime and waits through a std
//! channel, which is safe from any thread — including threads that are inside
//! another tokio runtime, where `block_on` would panic.

use std::future::Future;

use openframe_domain::{AppError, AppResult};

pub struct AiRuntime {
    rt: Option<tokio::runtime::Runtime>,
}

impl AiRuntime {
    pub fn new() -> AppResult<Self> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("of-ai")
            .enable_all()
            .build()
            .map_err(|e| AppError::internal(format!("AI runtime: {e}")))?;
        Ok(Self { rt: Some(rt) })
    }

    pub fn handle(&self) -> tokio::runtime::Handle {
        self.rt.as_ref().expect("runtime alive").handle().clone()
    }

    /// Run `fut` to completion and return its output (a panic inside the
    /// future becomes an internal error instead of tearing down the caller).
    pub fn run<T, F>(&self, fut: F) -> AppResult<T>
    where
        F: Future<Output = AppResult<T>> + Send + 'static,
        T: Send + 'static,
    {
        let (tx, rx) = std::sync::mpsc::channel();
        self.handle().spawn(async move {
            let _ = tx.send(fut.await);
        });
        rx.recv().unwrap_or_else(|_| {
            Err(AppError::internal(
                "AI background task stopped unexpectedly",
            ))
        })
    }
}

impl Drop for AiRuntime {
    fn drop(&mut self) {
        if let Some(rt) = self.rt.take() {
            rt.shutdown_background();
        }
    }
}
