//! The request checks, the trial and lifetime decision, and the signed answer the app checks.
//!
//! The wire format is the app's (`crates/openframe-entitlement/src/mac/server.rs`), version 1:
//!
//! - Request: `{schema, nonce, environment, appTransactionJws, transactionJws}`. `nonce` is 64
//!   lowercase hex characters; `environment` is `"sandbox"` or `"production"`;
//!   `appTransactionJws` may be absent or `null`; unknown fields are refused.
//! - Answer: `{payload, signature}`. `payload` is base64url (no padding) of the JSON
//!   [`ResponseBody`]; `signature` is base64url (no padding) of the Ed25519 signature over
//!   [`RESPONSE_DOMAIN`] followed by the payload bytes exactly as sent. The app verifies the
//!   signature over those bytes and only then parses them, so no canonical form is needed.
//!
//! Every time value in the decision comes from Apple (the signed `purchaseDate` of the trial
//! transaction) or from this server's UTC clock. Nothing the app sends about time is used, and the
//! app transaction's `originalPurchaseDate` (the app's first download) is never read.
//!
//! The domain prefix stops a signature made for another OpenFrame file (for example the installer
//! licence, `openframe-licence-v1`) from being replayed as a server answer.

use std::collections::HashSet;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};

use crate::apple::{
    Environment, LIFETIME_PRODUCT_ID, TRIAL_PRODUCT_ID, TransactionPayload, TrustAnchor,
    verify_app_transaction, verify_transaction,
};

/// Domain separation prefix for the signed answer.
pub const RESPONSE_DOMAIN: &[u8] = b"openframe-mac-licence-response-v1\0";

/// The request and answer schema this server speaks.
pub const SCHEMA: u32 = 1;

/// Trial length: 168 hours, counted from Apple's purchase date of the trial product.
pub const TRIAL_MS: i64 = 168 * 60 * 60 * 1000;

/// How long the app may count down from one answer before it must check again: 6 hours, never
/// more than the seconds left in the trial.
pub const DEFAULT_LEASE_SECONDS: i64 = 6 * 60 * 60;

/// Upper bound for the number of transaction JWS strings in one request.
pub const MAX_TRANSACTIONS: usize = 16;

/// Length of the request nonce: 32 random bytes as lowercase hex.
pub const NONCE_HEX_LEN: usize = 64;

/// How far Apple's clock may be ahead of this server's before a purchase date counts as "in the
/// future".
pub const MAX_CLOCK_SKEW_MS: i64 = 5 * 60 * 1000;

/// No genuine OpenFrame purchase can be older than this. PLACEHOLDER: 2026-01-01T00:00:00Z.
///
/// OWNER INPUT: before the first production deployment, set this to a date no later than the
/// first sandbox test purchase of the trial product (sandbox purchases happen before the App
/// Store release), for example the day the in-app purchases were created in App Store Connect.
pub const APP_STORE_RELEASE_PLACEHOLDER_MS: i64 = 1_767_225_600_000;

/// What the app sends.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValidateRequest {
    pub schema: u32,
    #[serde(default)]
    pub nonce: Option<String>,
    pub environment: String,
    #[serde(default)]
    pub app_transaction_jws: Option<String>,
    #[serde(default)]
    pub transaction_jws: Vec<String>,
}

/// The server's verdict on the trial transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrialVerdict {
    /// Apple purchase date of the trial transaction plus 168 hours, Unix ms UTC.
    pub trial_ends_at_ms: i64,
    /// Seconds left at `server_time_ms`, never negative.
    pub remaining_seconds: i64,
    /// Apple's `originalTransactionId` of the trial transaction.
    pub apple_transaction_id: String,
    /// [`DEFAULT_LEASE_SECONDS`], never more than `remaining_seconds`.
    pub lease_seconds: i64,
}

/// The signed part of the answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseBody {
    pub schema: u32,
    pub nonce: String,
    pub environment: Environment,
    pub server_time_ms: i64,
    pub trial: Option<TrialVerdict>,
    pub lifetime: bool,
    pub revoked: bool,
}

/// The answer as sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedResponse {
    pub payload: String,
    pub signature: String,
}

/// Why a request was refused. Errors are never signed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The request itself is wrong (HTTP 400).
    Request(&'static str),
    /// A signed record in the request failed a check (HTTP 422).
    Record(&'static str),
}

impl Refusal {
    pub fn code(self) -> &'static str {
        match self {
            Refusal::Request(c) | Refusal::Record(c) => c,
        }
    }
}

/// A request that passed every check: the answer to sign and the Apple original transaction ids
/// for the log line.
#[derive(Debug)]
pub struct Evaluated {
    pub body: ResponseBody,
    pub original_transaction_ids: Vec<String>,
}

/// Every check on a request, then the decision.
///
/// Request: schema, a 64-character lowercase hex nonce, the environment this deployment serves,
/// at most [`MAX_TRANSACTIONS`] transactions. Each signed record: Apple's chain and signature,
/// bundle id, environment, product id, transaction ids, ownership and type (see
/// [`crate::apple`]). Across the request: no transaction twice, one `appTransactionId` for every
/// record that carries one (the app transaction included), and purchase and revocation dates that
/// are believable against this server's clock.
pub fn evaluate(
    request: &ValidateRequest,
    anchor: &TrustAnchor,
    environment: Environment,
    now_ms: i64,
) -> Result<Evaluated, Refusal> {
    if request.schema != SCHEMA {
        return Err(Refusal::Request("unsupported_schema"));
    }
    let Some(nonce) = request.nonce.as_deref() else {
        return Err(Refusal::Request("missing_nonce"));
    };
    if !is_nonce(nonce) {
        return Err(Refusal::Request("bad_nonce"));
    }
    if Environment::parse_wire(&request.environment) != Some(environment) {
        return Err(Refusal::Request("wrong_environment"));
    }
    if request.transaction_jws.len() > MAX_TRANSACTIONS {
        return Err(Refusal::Request("too_many_transactions"));
    }

    let mut app_transaction_ids: Vec<String> = Vec::new();
    if let Some(jws) = &request.app_transaction_jws {
        let app = verify_app_transaction(jws, anchor, environment)
            .map_err(|e| Refusal::Record(e.code()))?;
        app_transaction_ids.extend(app.app_transaction_id);
    }

    let mut verified = Vec::with_capacity(request.transaction_jws.len());
    let mut seen = HashSet::new();
    for jws in &request.transaction_jws {
        let tx =
            verify_transaction(jws, anchor, environment).map_err(|e| Refusal::Record(e.code()))?;
        if !seen.insert(tx.transaction_id.clone()) {
            return Err(Refusal::Record("duplicate_transaction"));
        }
        if !dates_are_believable(&tx, now_ms) {
            return Err(Refusal::Record("bad_purchase_date"));
        }
        app_transaction_ids.extend(tx.app_transaction_id.clone());
        verified.push(tx);
    }
    // One Apple Account and app: every appTransactionId present must be the same.
    if app_transaction_ids.windows(2).any(|w| w[0] != w[1]) {
        return Err(Refusal::Record("app_transaction_mismatch"));
    }

    let original_transaction_ids = verified
        .iter()
        .map(|t| t.original_transaction_id.clone())
        .collect();
    Ok(Evaluated {
        body: decide(nonce.to_string(), environment, &verified, now_ms),
        original_transaction_ids,
    })
}

fn is_nonce(nonce: &str) -> bool {
    nonce.len() == NONCE_HEX_LEN
        && nonce
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// A purchase date (and an original purchase date) not in the future beyond the clock skew and
/// not before the release placeholder; a revocation date not before the purchase and not in the
/// future.
fn dates_are_believable(tx: &TransactionPayload, now_ms: i64) -> bool {
    let latest = now_ms.saturating_add(MAX_CLOCK_SKEW_MS);
    let in_range = |d: i64| (APP_STORE_RELEASE_PLACEHOLDER_MS..=latest).contains(&d);
    in_range(tx.purchase_date)
        && tx.original_purchase_date.is_none_or(in_range)
        && tx
            .revocation_date
            .is_none_or(|r| r >= trial_origin(tx) && r <= latest)
}

/// The start of a trial transaction: Apple's `purchaseDate`, or the same transaction's
/// `originalPurchaseDate` when that is earlier. Never later than either, so a restore cannot move
/// the start, and never the app transaction's date.
pub fn trial_origin(t: &TransactionPayload) -> i64 {
    t.original_purchase_date
        .map_or(t.purchase_date, |o| o.min(t.purchase_date))
}

/// Decide from verified transactions only.
///
/// - Lifetime: a lifetime transaction without a revocation date.
/// - Trial: the earliest non-revoked trial transaction (by start, then transaction id), ending
///   168 hours after its start; seconds left by this server's clock; a lease of
///   [`DEFAULT_LEASE_SECONDS`], never more than the seconds left.
/// - Revoked: no lifetime, and at least one known transaction carries a revocation date.
pub fn decide(
    nonce: String,
    environment: Environment,
    transactions: &[TransactionPayload],
    now_ms: i64,
) -> ResponseBody {
    let lifetime = transactions
        .iter()
        .any(|t| t.product_id == LIFETIME_PRODUCT_ID && t.revocation_date.is_none());

    let trial = transactions
        .iter()
        .filter(|t| t.product_id == TRIAL_PRODUCT_ID && t.revocation_date.is_none())
        .min_by(|a, b| {
            (trial_origin(a), &a.transaction_id).cmp(&(trial_origin(b), &b.transaction_id))
        })
        .map(|t| {
            let ends = trial_origin(t).saturating_add(TRIAL_MS);
            let remaining_seconds = ends.saturating_sub(now_ms).max(0) / 1000;
            TrialVerdict {
                trial_ends_at_ms: ends,
                remaining_seconds,
                apple_transaction_id: t.original_transaction_id.clone(),
                lease_seconds: DEFAULT_LEASE_SECONDS.min(remaining_seconds),
            }
        });

    let revoked = !lifetime && transactions.iter().any(|t| t.revocation_date.is_some());

    ResponseBody {
        schema: SCHEMA,
        nonce,
        environment,
        server_time_ms: now_ms,
        trial,
        lifetime,
        revoked,
    }
}

/// Sign an answer: the payload is the body's JSON bytes, the signature covers
/// `RESPONSE_DOMAIN || payload`.
pub fn sign_response(body: &ResponseBody, key: &SigningKey) -> SignedResponse {
    let payload = serde_json::to_vec(body).expect("response body serializes");
    let mut message = Vec::with_capacity(RESPONSE_DOMAIN.len() + payload.len());
    message.extend_from_slice(RESPONSE_DOMAIN);
    message.extend_from_slice(&payload);
    let signature = key.sign(&message);
    SignedResponse {
        payload: URL_SAFE_NO_PAD.encode(&payload),
        signature: URL_SAFE_NO_PAD.encode(signature.to_bytes()),
    }
}
