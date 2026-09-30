//! OpenAI-compatible chat client for the loopback llama.cpp sidecar.
//!
//! Structured output is grammar-constrained: when a JSON schema is supplied
//! the server converts it to a GBNF grammar, so the model can only emit
//! schema-shaped JSON. The application still validates every field.

use std::time::Duration;

use openframe_domain::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Where the running sidecar listens (loopback only) and its per-launch key.
#[derive(Clone)]
pub struct Endpoint {
    pub base_url: String,
    pub api_key: String,
}

impl std::fmt::Debug for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Endpoint")
            .field("base_url", &self.base_url)
            .field("api_key", &"[redacted]")
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn system(c: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: c.into(),
        }
    }
    pub fn user(c: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: c.into(),
        }
    }
    pub fn assistant(c: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: c.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    /// When set, output is constrained to this JSON schema.
    pub json_schema: Option<Value>,
    pub max_tokens: u32,
    pub temperature: f32,
    pub timeout: Duration,
}

impl ChatRequest {
    pub fn new(messages: Vec<ChatMessage>) -> Self {
        Self {
            messages,
            json_schema: None,
            max_tokens: 512,
            temperature: 0.2,
            timeout: Duration::from_secs(180),
        }
    }
    pub fn with_schema(mut self, schema: Value) -> Self {
        self.json_schema = Some(schema);
        self
    }
    pub fn max_tokens(mut self, n: u32) -> Self {
        self.max_tokens = n;
        self
    }
}

pub fn build_body(req: &ChatRequest) -> Value {
    let mut body = json!({
        "model": "openframe-local",
        "messages": req.messages,
        "temperature": req.temperature,
        "max_tokens": req.max_tokens,
        "stream": false,
        // Qwen3 "thinking" is disabled: hidden reasoning is neither needed nor persisted.
        "chat_template_kwargs": { "enable_thinking": false },
    });
    if let Some(schema) = &req.json_schema {
        body["response_format"] = json!({
            "type": "json_schema",
            "json_schema": { "name": "openframe_response", "strict": true, "schema": schema }
        });
    }
    body
}

/// Remove any `<think>…</think>` block: chain-of-thought is never shown or stored.
pub fn strip_reasoning(content: &str) -> String {
    let mut out = content.to_string();
    while let Some(start) = out.find("<think>") {
        match out[start..].find("</think>") {
            Some(end) => out.replace_range(start..start + end + "</think>".len(), ""),
            None => out.truncate(start),
        }
    }
    out.trim().to_string()
}

pub fn unavailable(detail: impl Into<String>) -> AppError {
    AppError::ai(
        "unavailable",
        "AI is currently unavailable. OpenFrame's core workflows continue to work normally.",
    )
    .with_detail(detail)
    .retryable()
}

pub async fn chat(http: &reqwest::Client, ep: &Endpoint, req: &ChatRequest) -> AppResult<String> {
    let url = format!("{}/v1/chat/completions", ep.base_url);
    let resp = http
        .post(&url)
        .bearer_auth(&ep.api_key)
        .timeout(req.timeout)
        .json(&build_body(req))
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                AppError::ai("timeout", "Offline AI took too long to answer. Nothing was changed. Try a shorter or more specific request.")
                    .retryable()
            } else {
                unavailable(e.to_string())
            }
        })?;
    let status = resp.status();
    if !status.is_success() {
        return Err(unavailable(format!(
            "runtime answered HTTP {}",
            status.as_u16()
        )));
    }
    let v: Value = resp.json().await.map_err(|e| malformed(e.to_string()))?;
    let content = v
        .pointer("/choices/0/message/content")
        .and_then(|c| c.as_str())
        .ok_or_else(|| malformed("response has no message content"))?;
    Ok(strip_reasoning(content))
}

pub fn malformed(detail: impl Into<String>) -> AppError {
    AppError::ai("malformed", "Offline AI returned an answer OpenFrame couldn't use. Nothing was changed. Please try again.")
        .with_detail(detail)
        .retryable()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reasoning_is_stripped() {
        assert_eq!(
            strip_reasoning("<think>secret plan</think>\n{\"a\":1}"),
            "{\"a\":1}"
        );
        assert_eq!(strip_reasoning("answer <think>unterminated"), "answer");
        assert_eq!(strip_reasoning("plain"), "plain");
    }

    #[test]
    fn schema_requests_grammar_constrained_output() {
        let req =
            ChatRequest::new(vec![ChatMessage::user("hi")]).with_schema(json!({"type":"object"}));
        let body = build_body(&req);
        assert_eq!(body["response_format"]["type"], "json_schema");
        assert_eq!(body["chat_template_kwargs"]["enable_thinking"], false);
    }

    #[test]
    fn endpoint_debug_never_prints_key() {
        let ep = Endpoint {
            base_url: "http://127.0.0.1:1".into(),
            api_key: "supersecret".into(),
        };
        assert!(!format!("{ep:?}").contains("supersecret"));
    }
}
