//! `POST /v1/mac/validate`.
//!
//! Request body (at most 64 KiB), the app's wire format version 1, unknown fields refused:
//! `{ "schema": 1, "nonce": "<64 lowercase hex>", "environment": "sandbox" | "production",
//!    "appTransactionJws": "<AppTransaction JWS>" | null, "transactionJws": ["<JWS>", ...] }`
//!
//! Response 200: `{ "payload": "<base64url JSON>", "signature": "<base64url Ed25519>" }` from
//! [`crate::answer::sign_response`]. Any other status carries `{ "error": "<code>" }` and no
//! signature, and the app treats it as "could not confirm".
//!
//! One log line per request: the outcome and the Apple original transaction ids. No address, no
//! nonce, no payload.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use axum::Router;
use axum::body::Bytes;
use axum::extract::{ConnectInfo, DefaultBodyLimit, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use ed25519_dalek::SigningKey;
use serde_json::json;

use crate::answer::{Refusal, ValidateRequest, evaluate, sign_response};
use crate::apple::{Environment, TrustAnchor};
use crate::ratelimit::RateLimiter;

pub const MAX_BODY_BYTES: usize = 64 * 1024;

/// Everything a request needs. Built once at start-up.
pub struct AppState {
    pub anchor: TrustAnchor,
    /// The one App Store environment this deployment serves. Sandbox and Production run as two
    /// deployments with two different signing keys.
    pub environment: Environment,
    pub signing_key: SigningKey,
    pub limiter: RateLimiter,
    /// Server UTC clock in Unix milliseconds. Replaced by a fixed clock in tests.
    pub clock: Box<dyn Fn() -> i64 + Send + Sync>,
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/mac/validate", post(validate))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .with_state(state)
}

/// The system clock as Unix milliseconds, or 0 if the clock is before 1970 (which ends any trial).
pub fn system_clock_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

fn error(status: StatusCode, code: &str) -> Response {
    (status, axum::Json(json!({ "error": code }))).into_response()
}

async fn validate(
    State(state): State<Arc<AppState>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    body: Bytes,
) -> Response {
    if !state.limiter.allow(peer.ip(), Instant::now()) {
        log("rate_limited", &[]);
        return error(StatusCode::TOO_MANY_REQUESTS, "rate_limited");
    }
    let Ok(request) = serde_json::from_slice::<ValidateRequest>(&body) else {
        log("bad_request", &[]);
        return error(StatusCode::BAD_REQUEST, "bad_request");
    };
    match evaluate(&request, &state.anchor, state.environment, (state.clock)()) {
        Ok(evaluated) => {
            log("ok", &evaluated.original_transaction_ids);
            (
                StatusCode::OK,
                axum::Json(sign_response(&evaluated.body, &state.signing_key)),
            )
                .into_response()
        }
        Err(refusal) => {
            log(refusal.code(), &[]);
            let status = match refusal {
                Refusal::Request(_) => StatusCode::BAD_REQUEST,
                Refusal::Record(_) => StatusCode::UNPROCESSABLE_ENTITY,
            };
            error(status, refusal.code())
        }
    }
}

fn log(outcome: &str, original_transaction_ids: &[String]) {
    // Transaction ids are digits from Apple and were checked; anything else is replaced so a log
    // line cannot be forged through a crafted id.
    let ids: Vec<&str> = original_transaction_ids
        .iter()
        .map(|id| {
            if id.len() <= 64 && id.bytes().all(|b| b.is_ascii_digit()) {
                id.as_str()
            } else {
                "invalid"
            }
        })
        .collect();
    eprintln!(
        "validate outcome={outcome} transactions=[{}]",
        ids.join(",")
    );
}
