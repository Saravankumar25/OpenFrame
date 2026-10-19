//! What the macOS shell hands to the Mac App Store adapter ([`MacInputs`]): the Swift StoreKit
//! bridge, the HTTPS client for the licence server, the system's secure random generator for
//! request nonces, and the monotonic session clock.
//!
//! NOT COMPILED ON A MAC YET (see `macos_storekit.rs`). Nothing here sends a request when it is
//! built: the HTTP client is created on the first check, which runs on the entitlement worker
//! thread (`revalidation.rs`).
//!
//! Network use: only [`LICENCE_SERVER_URL`] or [`LICENCE_SERVER_SANDBOX_URL`], over HTTPS, during
//! a trial check. TLS only (rustls with the bundled web PKI roots), no redirects, a 10 second
//! timeout and a 64 KiB answer cap. The request carries Apple's signed transactions and a nonce,
//! nothing from the user's projects.

use std::io::Read;
use std::sync::OnceLock;
use std::time::Duration;

use ed25519_dalek::VerifyingKey;
use openframe_entitlement::mac::server::NONCE_BYTES;
use openframe_entitlement::mac::{
    InstantClock, LicenceServer, MacInputs, NonceSource, ServerError, StoreEnvironment,
    ValidateRequest, ValidateResponse,
};
use rand::RngCore;

use super::macos_storekit::StoreKitChannel;

/// The production licence server's validate endpoint. PLACEHOLDER: the owner replaces it with
/// the deployed address (docs/engineering/38-mac-app-store-trial.md section 10). A `.invalid`
/// host never resolves, and no request is sent to it.
pub const LICENCE_SERVER_URL: &str = "https://licence.openframe.invalid/v1/mac/validate";

/// The sandbox deployment, for TestFlight, App Review and sandbox testers. PLACEHOLDER as above.
pub const LICENCE_SERVER_SANDBOX_URL: &str =
    "https://licence-sandbox.openframe.invalid/v1/mac/validate";

/// The licence server's Ed25519 response public key, 64 lowercase hex characters. PLACEHOLDER:
/// empty until the owner creates the key. With no valid key the adapter never sends a request
/// and the state is NeedsOnlineCheck (open and export only), with the reason shown in Settings.
pub const LICENCE_RESPONSE_PUBLIC_KEY_HEX: &str = "";

const TIMEOUT: Duration = Duration::from_secs(10);
const MAX_ANSWER_BYTES: u64 = 64 * 1024;

/// The inputs for the Mac App Store adapter. Sends nothing and calls no StoreKit function.
pub fn build() -> MacInputs {
    MacInputs {
        bridge: Box::new(StoreKitChannel::default()),
        server: Box::new(HttpLicenceServer::default()),
        nonces: Box::new(OsNonces),
        clock: Box::new(InstantClock::new()),
        response_key: response_key(LICENCE_RESPONSE_PUBLIC_KEY_HEX),
    }
}

/// The response key from its hex form, or `None` for the placeholder or a malformed value.
fn response_key(hex_key: &str) -> Option<VerifyingKey> {
    let bytes: [u8; 32] = hex::decode(hex_key).ok()?.try_into().ok()?;
    let key = VerifyingKey::from_bytes(&bytes).ok();
    if key.is_none() {
        tracing::warn!("the licence server response key is not valid; trials cannot be confirmed");
    }
    key
}

/// Nonces from the operating system's secure random generator. `None` when it fails, and the
/// request is then not sent.
struct OsNonces;

impl NonceSource for OsNonces {
    fn nonce(&self) -> Option<[u8; NONCE_BYTES]> {
        let mut bytes = [0u8; NONCE_BYTES];
        rand::rngs::OsRng.try_fill_bytes(&mut bytes).ok()?;
        Some(bytes)
    }
}

/// The licence server client: one POST per trial check, blocking, on the worker thread.
#[derive(Default)]
struct HttpLicenceServer {
    client: OnceLock<Option<reqwest::blocking::Client>>,
}

impl HttpLicenceServer {
    fn client(&self) -> Option<&reqwest::blocking::Client> {
        self.client
            .get_or_init(|| {
                reqwest::blocking::Client::builder()
                    .https_only(true)
                    .redirect(reqwest::redirect::Policy::none())
                    .timeout(TIMEOUT)
                    .connect_timeout(TIMEOUT)
                    .user_agent(concat!("OpenFrameStudio/", env!("CARGO_PKG_VERSION")))
                    .build()
                    .map_err(|_| tracing::warn!("the licence server client could not be built"))
                    .ok()
            })
            .as_ref()
    }
}

/// The endpoint for an environment. Xcode transactions are never sent anywhere.
fn endpoint(environment: StoreEnvironment) -> Option<&'static str> {
    let url = match environment {
        StoreEnvironment::Production => LICENCE_SERVER_URL,
        StoreEnvironment::Sandbox => LICENCE_SERVER_SANDBOX_URL,
        StoreEnvironment::Xcode => return None,
    };
    let host = url.strip_prefix("https://")?.split('/').next()?;
    (!host.ends_with(".invalid")).then_some(url)
}

impl LicenceServer for HttpLicenceServer {
    fn validate(&self, req: ValidateRequest) -> Result<ValidateResponse, ServerError> {
        if req.environment == StoreEnvironment::Xcode {
            return Err(ServerError::Rejected);
        }
        let Some(url) = endpoint(req.environment) else {
            tracing::warn!("the licence server address is not set in this build");
            return Err(ServerError::Unavailable);
        };
        let client = self.client().ok_or(ServerError::Unavailable)?;
        let response = client
            .post(url)
            .json(&req)
            .send()
            .map_err(|_| ServerError::NoConnection)?;
        let status = response.status();
        if !status.is_success() {
            tracing::info!(
                status = status.as_u16(),
                "the licence server refused the check"
            );
            return Err(match status.as_u16() {
                408 | 429 => ServerError::Unavailable,
                400..=499 => ServerError::Rejected,
                _ => ServerError::Unavailable,
            });
        }
        if response
            .content_length()
            .is_some_and(|len| len > MAX_ANSWER_BYTES)
        {
            return Err(ServerError::Unavailable);
        }
        let mut body = Vec::with_capacity(4096);
        response
            .take(MAX_ANSWER_BYTES + 1)
            .read_to_end(&mut body)
            .map_err(|_| ServerError::NoConnection)?;
        if body.len() as u64 > MAX_ANSWER_BYTES {
            return Err(ServerError::Unavailable);
        }
        // Only the shape is read here; the adapter checks the signature, nonce and environment.
        serde_json::from_slice(&body).map_err(|_| ServerError::Unavailable)
    }
}
