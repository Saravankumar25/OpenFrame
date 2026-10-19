//! The licensing server contract for the Mac App Store trial.
//!
//! During the trial the app sends the signed StoreKit JWS strings to OpenFrame's licensing server.
//! The server checks Apple's signature on each JWS, takes the trial transaction's Apple-recorded
//! purchase date, computes `trialEndsAt = purchaseDate + 168 h` in UTC and returns the remaining
//! seconds measured on its own clock. The Mac's clock is never sent and never used.
//!
//! The answer also carries a lease: how long the app may count down from this answer before it
//! must check again. The server sends [`DEFAULT_LEASE_SECONDS`], never more than the seconds left.
//!
//! Wire format, version 1 (the licence server in `services/licence-server` follows it, and the
//! shared fixture `services/licence-server/tests/fixtures/mac-validate-v1.json` proves both sides
//! agree on the same bytes):
//!
//! - Request: `{schema, nonce, environment, appTransactionJws, transactionJws}` where `nonce` is
//!   64 lowercase hex characters, `environment` is `"sandbox"` or `"production"` and
//!   `appTransactionJws` may be `null`.
//! - Response: `{payload, signature}`. `payload` is base64url (no padding) of the JSON
//!   `{schema, nonce, environment, serverTimeMs, trial, lifetime, revoked}` with `trial` either
//!   `null` or `{trialEndsAtMs, remainingSeconds, appleTransactionId, leaseSeconds}`. `signature`
//!   is base64url (no padding) of the Ed25519 signature over [`RESPONSE_DOMAIN`] followed by the
//!   payload bytes.
//!
//! The answer is signed with OpenFrame's Ed25519 response key. The app holds only the public key.
//! The signature covers the domain prefix [`RESPONSE_DOMAIN`] followed by the payload bytes exactly
//! as sent, so:
//!
//! - an edited response fails the signature check,
//! - a signature made for another OpenFrame message cannot be replayed as a response,
//! - a response recorded earlier fails the nonce check, because every request carries a fresh
//!   random nonce that the server must echo inside the signed payload.
//!
//! Only a correctly signed payload is parsed. The HTTP client lives in the desktop shell and
//! implements [`LicenceServer`].

use base64::Engine;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};

use super::bridge::StoreEnvironment;

/// Domain separation prefix for the signed response.
pub const RESPONSE_DOMAIN: &[u8] = b"openframe-mac-licence-response-v1\0";

/// Current request and response schema.
pub const SCHEMA: u32 = 1;

/// Length of the request nonce in bytes. Sent as lowercase hex.
pub const NONCE_BYTES: usize = 32;

/// Upper bound for the decoded response payload and for each encoded field.
pub const MAX_RESPONSE_BYTES: usize = 8 * 1024;

/// Upper bound for the number of transaction JWS strings in one request.
pub const MAX_REQUEST_TRANSACTIONS: usize = 16;

/// The lease the licence server grants by default: 6 hours of counting down before the app must
/// check again. The server never grants more than the seconds left in the trial.
pub const DEFAULT_LEASE_SECONDS: i64 = 6 * 3600;

const B64: base64::engine::GeneralPurpose = base64::engine::general_purpose::URL_SAFE_NO_PAD;

/// What the app sends. Carries no project data, no e-mail address and no device time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateRequest {
    pub schema: u32,
    /// Fresh random nonce for this request, lowercase hex of [`NONCE_BYTES`] bytes.
    pub nonce: String,
    /// The environment of this build. The server refuses transactions from any other.
    pub environment: StoreEnvironment,
    /// `AppTransaction.shared` JWS, when StoreKit returned it.
    pub app_transaction_jws: Option<String>,
    /// The JWS of every OpenFrame transaction StoreKit returned for this environment.
    pub transaction_jws: Vec<String>,
}

/// The server's verdict on the trial transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrialVerdict {
    /// Apple purchase date of the earliest trial transaction plus 168 hours, Unix ms UTC.
    pub trial_ends_at_ms: i64,
    /// Seconds left at `server_time_ms`, never negative.
    pub remaining_seconds: i64,
    /// The Apple transaction id the verdict was computed from.
    pub apple_transaction_id: String,
    /// How many seconds of monotonic session time this verdict may be counted down before the
    /// app must check again. The app uses at most `remaining_seconds` of it.
    pub lease_seconds: i64,
}

/// The signed part of a response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseBody {
    pub schema: u32,
    /// The request nonce, echoed.
    pub nonce: String,
    /// The environment the server validated against, echoed.
    pub environment: StoreEnvironment,
    /// The server's clock when it answered, Unix ms UTC.
    pub server_time_ms: i64,
    /// Present when a valid trial transaction was found.
    pub trial: Option<TrialVerdict>,
    /// Whether a valid, unrevoked lifetime transaction was found. Informational: the app takes
    /// the lifetime unlock from StoreKit's `currentEntitlements` only.
    pub lifetime: bool,
    /// Whether Apple reported a transaction in the request as refunded or revoked.
    pub revoked: bool,
}

/// A response as it arrives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateResponse {
    /// base64url (no padding) of the [`ResponseBody`] JSON bytes exactly as signed.
    pub payload: String,
    /// base64url (no padding) of the Ed25519 signature over `RESPONSE_DOMAIN || payload bytes`.
    pub signature: String,
}

/// Why the server call did not produce a response. No variant carries user data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerError {
    /// No connection, DNS failure or timeout.
    NoConnection,
    /// The server refused the request, for example because a JWS failed Apple's signature check.
    Rejected,
    /// The server answered with an error or is down.
    Unavailable,
}

/// The licensing server. The desktop shell implements this over HTTPS with a bounded timeout.
pub trait LicenceServer: Send + Sync {
    fn validate(&self, req: ValidateRequest) -> Result<ValidateResponse, ServerError>;
}

/// A boxed server client, so the desktop shell can hand one over without naming its type.
impl<T: LicenceServer + ?Sized> LicenceServer for Box<T> {
    fn validate(&self, req: ValidateRequest) -> Result<ValidateResponse, ServerError> {
        (**self).validate(req)
    }
}

/// Why a response was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseError {
    /// Bad base64, too large.
    Unreadable,
    /// The signature does not verify against the embedded key with the domain prefix.
    BadSignature,
    /// Signed, but not a response body of a known schema, or the values are not believable.
    BadBody,
    /// Signed and well formed, but for another request (replay).
    NonceMismatch,
}

/// Lowercase hex of a nonce, as sent in [`ValidateRequest::nonce`].
pub fn nonce_hex(nonce: &[u8; NONCE_BYTES]) -> String {
    hex::encode(nonce)
}

/// Verify a response and return its body.
///
/// Checks, in order: size, signature with the domain prefix (strict), body schema and
/// believable values, then the nonce. Only a correctly signed payload is parsed.
pub fn verify_response(
    resp: &ValidateResponse,
    public_key: &VerifyingKey,
    expected_nonce: &str,
) -> Result<ResponseBody, ResponseError> {
    // Base64 grows the size by a third; refuse anything that cannot decode to a bounded payload.
    if resp.payload.len() > MAX_RESPONSE_BYTES * 4 / 3 + 4 || resp.signature.len() > 128 {
        return Err(ResponseError::Unreadable);
    }
    let payload = B64
        .decode(&resp.payload)
        .map_err(|_| ResponseError::Unreadable)?;
    if payload.len() > MAX_RESPONSE_BYTES {
        return Err(ResponseError::Unreadable);
    }
    let sig_bytes = B64
        .decode(&resp.signature)
        .map_err(|_| ResponseError::BadSignature)?;
    let sig = Signature::from_slice(&sig_bytes).map_err(|_| ResponseError::BadSignature)?;
    let mut message = Vec::with_capacity(RESPONSE_DOMAIN.len() + payload.len());
    message.extend_from_slice(RESPONSE_DOMAIN);
    message.extend_from_slice(&payload);
    public_key
        .verify_strict(&message, &sig)
        .map_err(|_| ResponseError::BadSignature)?;
    let body: ResponseBody =
        serde_json::from_slice(&payload).map_err(|_| ResponseError::BadBody)?;
    if body.schema != SCHEMA || body.server_time_ms <= 0 {
        return Err(ResponseError::BadBody);
    }
    let trial_ok = body.trial.as_ref().is_none_or(|t| {
        t.remaining_seconds >= 0
            && t.lease_seconds >= 0
            && t.trial_ends_at_ms > 0
            && !t.apple_transaction_id.is_empty()
    });
    if !trial_ok {
        return Err(ResponseError::BadBody);
    }
    if expected_nonce.is_empty() || body.nonce != expected_nonce {
        return Err(ResponseError::NonceMismatch);
    }
    Ok(body)
}

/// Sign a response body. For tests only; the licensing server signs with its own copy
/// (`services/licence-server`), and the private key never ships.
#[cfg(test)]
pub fn sign_response(
    signing_key: &ed25519_dalek::SigningKey,
    body: &ResponseBody,
) -> ValidateResponse {
    use ed25519_dalek::Signer;
    let payload = serde_json::to_vec(body).expect("response body serializes");
    let mut message = Vec::with_capacity(RESPONSE_DOMAIN.len() + payload.len());
    message.extend_from_slice(RESPONSE_DOMAIN);
    message.extend_from_slice(&payload);
    let sig = signing_key.sign(&message);
    ValidateResponse {
        payload: B64.encode(payload),
        signature: B64.encode(sig.to_bytes()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn key() -> SigningKey {
        SigningKey::generate(&mut rand::rngs::OsRng)
    }

    fn body(nonce: &str) -> ResponseBody {
        ResponseBody {
            schema: SCHEMA,
            nonce: nonce.to_string(),
            environment: StoreEnvironment::Production,
            server_time_ms: 1_791_244_800_000,
            trial: Some(TrialVerdict {
                trial_ends_at_ms: 1_791_849_600_000,
                remaining_seconds: 604_800,
                apple_transaction_id: "2000000001".to_string(),
                lease_seconds: DEFAULT_LEASE_SECONDS,
            }),
            lifetime: false,
            revoked: false,
        }
    }

    #[test]
    fn a_signed_response_verifies_with_the_right_nonce() {
        let sk = key();
        let resp = sign_response(&sk, &body("n1"));
        assert_eq!(
            verify_response(&resp, &sk.verifying_key(), "n1"),
            Ok(body("n1"))
        );
    }

    #[test]
    fn an_edited_payload_or_another_key_is_refused() {
        let sk = key();
        let resp = sign_response(&sk, &body("n1"));
        let mut payload = B64.decode(&resp.payload).unwrap();
        let text = String::from_utf8(payload.clone()).unwrap();
        let edited = text.replace("604800", "999999");
        assert_ne!(edited, text);
        payload = edited.into_bytes();
        let forged = ValidateResponse {
            payload: B64.encode(payload),
            signature: resp.signature.clone(),
        };
        assert_eq!(
            verify_response(&forged, &sk.verifying_key(), "n1"),
            Err(ResponseError::BadSignature)
        );
        assert_eq!(
            verify_response(&resp, &key().verifying_key(), "n1"),
            Err(ResponseError::BadSignature)
        );
    }

    #[test]
    fn a_signature_without_the_domain_prefix_is_refused() {
        let sk = key();
        let payload = serde_json::to_vec(&body("n1")).unwrap();
        let resp = ValidateResponse {
            payload: B64.encode(&payload),
            signature: B64.encode(sk.sign(&payload).to_bytes()),
        };
        assert_eq!(
            verify_response(&resp, &sk.verifying_key(), "n1"),
            Err(ResponseError::BadSignature)
        );
    }

    #[test]
    fn nonce_mismatch_bad_body_and_oversize_are_refused() {
        let sk = key();
        let vk = sk.verifying_key();
        let resp = sign_response(&sk, &body("n1"));
        assert_eq!(
            verify_response(&resp, &vk, "n2"),
            Err(ResponseError::NonceMismatch)
        );
        assert_eq!(
            verify_response(&resp, &vk, ""),
            Err(ResponseError::NonceMismatch)
        );

        let mut b = body("n1");
        b.schema = 2;
        assert_eq!(
            verify_response(&sign_response(&sk, &b), &vk, "n1"),
            Err(ResponseError::BadBody)
        );
        let mut b = body("n1");
        b.trial.as_mut().unwrap().remaining_seconds = -5;
        assert_eq!(
            verify_response(&sign_response(&sk, &b), &vk, "n1"),
            Err(ResponseError::BadBody)
        );
        let mut b = body("n1");
        b.trial.as_mut().unwrap().lease_seconds = -1;
        assert_eq!(
            verify_response(&sign_response(&sk, &b), &vk, "n1"),
            Err(ResponseError::BadBody)
        );
        // A trial verdict without a lease is not a version 1 answer.
        let mut value = serde_json::to_value(body("n1")).unwrap();
        value["trial"]
            .as_object_mut()
            .unwrap()
            .remove("leaseSeconds");
        let payload = serde_json::to_vec(&value).unwrap();
        let mut message = RESPONSE_DOMAIN.to_vec();
        message.extend_from_slice(&payload);
        let no_lease = ValidateResponse {
            payload: B64.encode(&payload),
            signature: B64.encode(sk.sign(&message).to_bytes()),
        };
        assert_eq!(
            verify_response(&no_lease, &vk, "n1"),
            Err(ResponseError::BadBody)
        );

        let huge = ValidateResponse {
            payload: "A".repeat(MAX_RESPONSE_BYTES * 2),
            signature: resp.signature.clone(),
        };
        assert_eq!(
            verify_response(&huge, &vk, "n1"),
            Err(ResponseError::Unreadable)
        );
        let garbage = ValidateResponse {
            payload: "%%%".to_string(),
            signature: resp.signature,
        };
        assert_eq!(
            verify_response(&garbage, &vk, "n1"),
            Err(ResponseError::Unreadable)
        );
    }

    #[test]
    fn the_request_carries_no_device_time() {
        let req = ValidateRequest {
            schema: SCHEMA,
            nonce: nonce_hex(&[7; NONCE_BYTES]),
            environment: StoreEnvironment::Production,
            app_transaction_jws: None,
            transaction_jws: vec!["a.b.c".to_string()],
        };
        let json = serde_json::to_value(&req).unwrap();
        let mut keys: Vec<&str> = json
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "appTransactionJws",
                "environment",
                "nonce",
                "schema",
                "transactionJws"
            ]
        );
        assert_eq!(req.nonce.len(), NONCE_BYTES * 2);
    }
}
