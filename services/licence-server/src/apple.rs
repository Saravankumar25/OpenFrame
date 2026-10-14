//! Verification of StoreKit 2 signed transactions and app transactions (JWS), the App Store
//! Server way.
//!
//! A StoreKit 2 transaction, and the `AppTransaction`, is a compact JWS
//! (`header.payload.signature`, base64url). The header names `ES256` and carries an `x5c` chain of
//! three DER certificates: the leaf that signed the JWS, Apple's intermediate, and Apple Root CA
//! G3. This module checks, in order:
//!
//! 1. size and shape of the JWS, and `alg == "ES256"`;
//! 2. the chain: the root in `x5c` is byte-for-byte the pinned root, the intermediate is signed by
//!    the pinned root and is a CA with Apple's intermediate marker, the leaf is signed by the
//!    intermediate, is not a CA, and has Apple's receipt-signing marker;
//! 3. the ES256 signature over `header.payload` with the leaf key;
//! 4. only then the payload: bundle id, environment, product id, transaction ids, ownership and
//!    type for a transaction; bundle id, environment and the optional `appTransactionId` for the
//!    app transaction;
//! 5. the validity window of all three certificates at the payload's `signedDate`.
//!
//! Dates against the server clock (a purchase date not in the future and not before the release
//! placeholder) are checked in [`crate::answer`], which owns the clock.
//!
//! Step 5 follows Apple's own library in offline mode. OCSP revocation of the leaf is not checked
//! (see docs/engineering/38-mac-app-store-trial.md, "What is not done").

use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use ring::signature::{ECDSA_P256_SHA256_FIXED, UnparsedPublicKey};
use serde::{Deserialize, Serialize};
use x509_parser::prelude::*;

/// The app's bundle identifier. Every transaction must name it.
pub const BUNDLE_ID: &str = "com.openframe.studio";
/// Zero-price non-consumable that starts the 7-day trial.
pub const TRIAL_PRODUCT_ID: &str = "com.openframe.studio.trial7";
/// Paid non-consumable that unlocks the app for good.
pub const LIFETIME_PRODUCT_ID: &str = "com.openframe.studio.lifetime";

/// Upper bound for one JWS. Real StoreKit transactions are about 4 KiB.
pub const MAX_JWS_BYTES: usize = 16 * 1024;
/// Upper bound for one decoded certificate.
const MAX_CERT_BYTES: usize = 4 * 1024;
/// Upper bound for an Apple transaction id. Apple's ids are decimal digits.
const MAX_TRANSACTION_ID_BYTES: usize = 64;
/// Upper bound for an `appTransactionId`.
const MAX_APP_TRANSACTION_ID_BYTES: usize = 128;

/// Apple marker extension on the leaf that signs App Store receipts and transactions.
const LEAF_MARKER_OID: &str = "1.2.840.113635.100.6.11.1";
/// Apple marker extension on the Worldwide Developer Relations intermediate.
const INTERMEDIATE_MARKER_OID: &str = "1.2.840.113635.100.6.2.1";

/// Apple Root CA - G3, the root of the StoreKit 2 signing chain.
///
/// Source: <https://www.apple.com/certificateauthority/AppleRootCA-G3.cer> (DER, 583 bytes),
/// downloaded 2026-10-06 and converted to PEM.
/// SHA-256 fingerprint of the DER bytes:
/// 63:34:3A:BF:B8:9A:6A:03:EB:B5:7E:9B:3F:5F:A7:BE:7C:4F:5C:75:6F:30:17:B3:A8:C4:88:C3:65:3E:91:79
/// Valid from 2014-04-30 to 2039-04-30.
///
/// OWNER CHECK REQUIRED: compare this fingerprint with the one Apple publishes at
/// <https://www.apple.com/certificateauthority/> before the first production deployment.
/// `TrustAnchor::apple` refuses to start if the PEM below and the fingerprint constant disagree.
pub const APPLE_ROOT_CA_G3_PEM: &str = "-----BEGIN CERTIFICATE-----
MIICQzCCAcmgAwIBAgIILcX8iNLFS5UwCgYIKoZIzj0EAwMwZzEbMBkGA1UEAwwS
QXBwbGUgUm9vdCBDQSAtIEczMSYwJAYDVQQLDB1BcHBsZSBDZXJ0aWZpY2F0aW9u
IEF1dGhvcml0eTETMBEGA1UECgwKQXBwbGUgSW5jLjELMAkGA1UEBhMCVVMwHhcN
MTQwNDMwMTgxOTA2WhcNMzkwNDMwMTgxOTA2WjBnMRswGQYDVQQDDBJBcHBsZSBS
b290IENBIC0gRzMxJjAkBgNVBAsMHUFwcGxlIENlcnRpZmljYXRpb24gQXV0aG9y
aXR5MRMwEQYDVQQKDApBcHBsZSBJbmMuMQswCQYDVQQGEwJVUzB2MBAGByqGSM49
AgEGBSuBBAAiA2IABJjpLz1AcqTtkyJygRMc3RCV8cWjTnHcFBbZDuWmBSp3ZHtf
TjjTuxxEtX/1H7YyYl3J6YRbTzBPEVoA/VhYDKX1DyxNB0cTddqXl5dvMVztK517
IDvYuVTZXpmkOlEKMaNCMEAwHQYDVR0OBBYEFLuw3qFYM4iapIqZ3r6966/ayySr
MA8GA1UdEwEB/wQFMAMBAf8wDgYDVR0PAQH/BAQDAgEGMAoGCCqGSM49BAMDA2gA
MGUCMQCD6cHEFl4aXTQY2e3v9GwOAEZLuN+yRhHFD/3meoyhpmvOwgPUnPWTxnS4
at+qIxUCMG1mihDK1A3UT82NQz60imOlM27jbdoXt2QfyFMm+YhidDkLF1vLUagM
6BgD56KyKA==
-----END CERTIFICATE-----
";

/// Lower-case hex SHA-256 of the DER form of [`APPLE_ROOT_CA_G3_PEM`].
pub const APPLE_ROOT_CA_G3_SHA256: &str =
    "63343abfb89a6a03ebb57e9b3f5fa7be7c4f5c756f3017b3a8c488c3653e9179";

/// The App Store environment a transaction belongs to. `Xcode` and local testing transactions
/// are never accepted.
///
/// Apple spells it `Sandbox` and `Production` inside signed payloads ([`Environment::as_str`]).
/// The app's request and the signed answer spell it `sandbox` and `production`
/// ([`Environment::wire`]), which is how it serializes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Sandbox,
    Production,
}

impl Environment {
    /// Apple's spelling, as in a signed payload and in `OPENFRAME_APPLE_ENVIRONMENT`.
    pub fn as_str(self) -> &'static str {
        match self {
            Environment::Sandbox => "Sandbox",
            Environment::Production => "Production",
        }
    }

    /// Apple's spelling only.
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "Sandbox" => Some(Environment::Sandbox),
            "Production" => Some(Environment::Production),
            _ => None,
        }
    }

    /// The app's spelling, as in the request and the answer.
    pub fn wire(self) -> &'static str {
        match self {
            Environment::Sandbox => "sandbox",
            Environment::Production => "production",
        }
    }

    /// The app's spelling only. `xcode` and anything else is refused.
    pub fn parse_wire(text: &str) -> Option<Self> {
        match text {
            "sandbox" => Some(Environment::Sandbox),
            "production" => Some(Environment::Production),
            _ => None,
        }
    }
}

/// The pinned root certificate that every chain must end in.
#[derive(Debug, Clone)]
pub struct TrustAnchor {
    root_der: Vec<u8>,
}

impl TrustAnchor {
    /// Apple Root CA G3, checked against its fingerprint constant.
    pub fn apple() -> Result<Self, String> {
        let (_, pem) = x509_parser::pem::parse_x509_pem(APPLE_ROOT_CA_G3_PEM.as_bytes())
            .map_err(|_| "the embedded Apple root certificate is not valid PEM".to_string())?;
        let digest = ring::digest::digest(&ring::digest::SHA256, &pem.contents);
        if hex_lower(digest.as_ref()) != APPLE_ROOT_CA_G3_SHA256 {
            return Err(
                "the embedded Apple root certificate does not match its fingerprint".into(),
            );
        }
        Ok(Self::from_der(pem.contents))
    }

    /// Any root given as DER. Used by the tests with a locally generated root, and by a future
    /// root rotation.
    pub fn from_der(root_der: Vec<u8>) -> Self {
        Self { root_der }
    }
}

/// Why a signed record was refused. Each value maps to one plain error code in the response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JwsError {
    /// Not a compact JWS, too large, bad base64 or a header that is not JSON.
    Malformed,
    /// The header names an algorithm other than ES256.
    UnsupportedAlgorithm,
    /// The certificate chain does not lead to the pinned root, or a certificate is out of date.
    BadChain,
    /// The ES256 signature does not verify with the leaf key.
    BadSignature,
    /// The signed payload is not a transaction (or not an app transaction).
    BadPayload,
    WrongBundle,
    WrongEnvironment,
    UnknownProduct,
    /// A transaction id or original transaction id that is empty, too long or not digits, or an
    /// `appTransactionId` that is empty, too long or has characters Apple does not use.
    BadTransactionId,
    /// Family Sharing or another ownership type, or not a non-consumable.
    NotPurchased,
}

impl JwsError {
    pub fn code(self) -> &'static str {
        match self {
            JwsError::Malformed => "malformed_transaction",
            JwsError::UnsupportedAlgorithm => "unsupported_algorithm",
            JwsError::BadChain => "bad_certificate_chain",
            JwsError::BadSignature => "bad_signature",
            JwsError::BadPayload => "bad_payload",
            JwsError::WrongBundle => "wrong_bundle_id",
            JwsError::WrongEnvironment => "wrong_environment",
            JwsError::UnknownProduct => "unknown_product",
            JwsError::BadTransactionId => "bad_transaction_id",
            JwsError::NotPurchased => "not_purchased",
        }
    }
}

/// The fields of Apple's `JWSTransactionDecodedPayload` that the decision uses. Unknown fields are
/// ignored because Apple adds fields over time.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionPayload {
    pub transaction_id: String,
    pub original_transaction_id: String,
    pub bundle_id: String,
    pub product_id: String,
    /// Unix milliseconds, Apple's clock. The trial starts here.
    pub purchase_date: i64,
    /// The purchase date of this same transaction's original purchase. When it is earlier than
    /// `purchase_date`, the trial starts at it instead, so a restore can never move the start
    /// later. This is a field of the trial transaction, never of the app transaction.
    #[serde(default)]
    pub original_purchase_date: Option<i64>,
    #[serde(default)]
    pub revocation_date: Option<i64>,
    pub environment: String,
    #[serde(default)]
    pub in_app_ownership_type: Option<String>,
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    /// Unix milliseconds when Apple signed this JWS.
    pub signed_date: i64,
    /// Apple's `appTransactionId`: the same for every transaction of one Apple Account and app.
    /// Absent on transactions from macOS earlier than 15.4.
    #[serde(default)]
    pub app_transaction_id: Option<String>,
}

/// The fields of Apple's `JWSAppTransactionDecodedPayload` that the server uses.
///
/// `originalPurchaseDate` (the date the app was first downloaded) is deliberately not read: it is
/// never the trial start.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppTransactionPayload {
    pub bundle_id: String,
    /// The environment, in Apple's spelling (`Production`, `Sandbox`, `Xcode`).
    pub receipt_type: String,
    /// Absent before macOS 15.4.
    #[serde(default)]
    pub app_transaction_id: Option<String>,
    /// Unix milliseconds when Apple signed this JWS.
    pub signed_date: i64,
}

#[derive(Deserialize)]
struct Header {
    alg: String,
    x5c: Vec<String>,
}

/// A JWS whose chain and signature verified: the payload bytes, still unparsed, and the window in
/// Unix seconds in which all three certificates are valid.
struct SignedPayload {
    bytes: Vec<u8>,
    valid_from: i64,
    valid_until: i64,
}

impl SignedPayload {
    fn valid_at(&self, signed_date_ms: i64) -> bool {
        let at = signed_date_ms.div_euclid(1000);
        self.valid_from <= at && at <= self.valid_until
    }
}

/// Verify one StoreKit 2 transaction JWS and return its payload.
pub fn verify_transaction(
    jws: &str,
    anchor: &TrustAnchor,
    expected: Environment,
) -> Result<TransactionPayload, JwsError> {
    let signed = verify_jws(jws, anchor)?;
    let tx: TransactionPayload =
        serde_json::from_slice(&signed.bytes).map_err(|_| JwsError::BadPayload)?;
    if !signed.valid_at(tx.signed_date) {
        return Err(JwsError::BadChain);
    }
    if tx.bundle_id != BUNDLE_ID {
        return Err(JwsError::WrongBundle);
    }
    if tx.environment != expected.as_str() {
        return Err(JwsError::WrongEnvironment);
    }
    if tx.product_id != TRIAL_PRODUCT_ID && tx.product_id != LIFETIME_PRODUCT_ID {
        return Err(JwsError::UnknownProduct);
    }
    if !is_transaction_id(&tx.transaction_id)
        || !is_transaction_id(&tx.original_transaction_id)
        || !tx
            .app_transaction_id
            .as_deref()
            .is_none_or(is_app_transaction_id)
    {
        return Err(JwsError::BadTransactionId);
    }
    // Family Sharing is off for both products. A shared transaction is never an entitlement here.
    if tx
        .in_app_ownership_type
        .as_deref()
        .is_some_and(|o| o != "PURCHASED")
    {
        return Err(JwsError::NotPurchased);
    }
    if tx.kind.as_deref().is_some_and(|k| k != "Non-Consumable") {
        return Err(JwsError::NotPurchased);
    }
    Ok(tx)
}

/// Verify the `AppTransaction` JWS and return its payload.
pub fn verify_app_transaction(
    jws: &str,
    anchor: &TrustAnchor,
    expected: Environment,
) -> Result<AppTransactionPayload, JwsError> {
    let signed = verify_jws(jws, anchor)?;
    let app: AppTransactionPayload =
        serde_json::from_slice(&signed.bytes).map_err(|_| JwsError::BadPayload)?;
    if !signed.valid_at(app.signed_date) {
        return Err(JwsError::BadChain);
    }
    if app.bundle_id != BUNDLE_ID {
        return Err(JwsError::WrongBundle);
    }
    if app.receipt_type != expected.as_str() {
        return Err(JwsError::WrongEnvironment);
    }
    if !app
        .app_transaction_id
        .as_deref()
        .is_none_or(is_app_transaction_id)
    {
        return Err(JwsError::BadTransactionId);
    }
    Ok(app)
}

/// Apple transaction ids are decimal digits.
fn is_transaction_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= MAX_TRANSACTION_ID_BYTES && id.bytes().all(|b| b.is_ascii_digit())
}

fn is_app_transaction_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_APP_TRANSACTION_ID_BYTES
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

/// Steps 1 to 3: shape, chain to the pinned root, ES256 signature. Returns the payload bytes
/// unparsed, so nothing is read from a payload whose signature has not been checked.
fn verify_jws(jws: &str, anchor: &TrustAnchor) -> Result<SignedPayload, JwsError> {
    if jws.len() > MAX_JWS_BYTES {
        return Err(JwsError::Malformed);
    }
    let mut parts = jws.split('.');
    let (Some(h64), Some(p64), Some(s64), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(JwsError::Malformed);
    };
    let header_bytes = URL_SAFE_NO_PAD
        .decode(h64)
        .map_err(|_| JwsError::Malformed)?;
    let header: Header = serde_json::from_slice(&header_bytes).map_err(|_| JwsError::Malformed)?;
    if header.alg != "ES256" {
        return Err(JwsError::UnsupportedAlgorithm);
    }
    if header.x5c.len() != 3 {
        return Err(JwsError::BadChain);
    }
    let mut ders = Vec::with_capacity(3);
    for cert in &header.x5c {
        if cert.len() > MAX_CERT_BYTES * 2 {
            return Err(JwsError::BadChain);
        }
        let der = STANDARD.decode(cert).map_err(|_| JwsError::BadChain)?;
        if der.len() > MAX_CERT_BYTES {
            return Err(JwsError::BadChain);
        }
        ders.push(der);
    }
    // The chain must end in the pinned root, byte for byte.
    if ders[2] != anchor.root_der {
        return Err(JwsError::BadChain);
    }
    let leaf = parse_cert(&ders[0])?;
    let intermediate = parse_cert(&ders[1])?;
    let root = parse_cert(&ders[2])?;

    check_link(&intermediate, &root)?;
    check_link(&leaf, &intermediate)?;
    if !is_ca(&intermediate) || is_ca(&leaf) {
        return Err(JwsError::BadChain);
    }
    if !has_extension(&intermediate, INTERMEDIATE_MARKER_OID)
        || !has_extension(&leaf, LEAF_MARKER_OID)
    {
        return Err(JwsError::BadChain);
    }

    // ES256: ECDSA P-256 with SHA-256, signature as the 64-byte r || s.
    let signature = URL_SAFE_NO_PAD
        .decode(s64)
        .map_err(|_| JwsError::BadSignature)?;
    if signature.len() != 64 {
        return Err(JwsError::BadSignature);
    }
    let signing_input = &jws.as_bytes()[..h64.len() + 1 + p64.len()];
    let leaf_point: &[u8] = &leaf.public_key().subject_public_key.data;
    UnparsedPublicKey::new(&ECDSA_P256_SHA256_FIXED, leaf_point)
        .verify(signing_input, &signature)
        .map_err(|_| JwsError::BadSignature)?;

    let bytes = URL_SAFE_NO_PAD
        .decode(p64)
        .map_err(|_| JwsError::BadPayload)?;
    let certs = [&leaf, &intermediate, &root];
    let valid_from = certs
        .iter()
        .map(|c| c.validity().not_before.timestamp())
        .max()
        .unwrap_or(i64::MAX);
    let valid_until = certs
        .iter()
        .map(|c| c.validity().not_after.timestamp())
        .min()
        .unwrap_or(i64::MIN);
    Ok(SignedPayload {
        bytes,
        valid_from,
        valid_until,
    })
}

fn parse_cert(der: &[u8]) -> Result<X509Certificate<'_>, JwsError> {
    match X509Certificate::from_der(der) {
        Ok(([], cert)) => Ok(cert),
        _ => Err(JwsError::BadChain),
    }
}

/// `child` names `issuer` as its issuer and carries a valid signature from the issuer's key.
fn check_link(child: &X509Certificate<'_>, issuer: &X509Certificate<'_>) -> Result<(), JwsError> {
    if child.issuer().as_raw() != issuer.subject().as_raw() {
        return Err(JwsError::BadChain);
    }
    child
        .verify_signature(Some(issuer.public_key()))
        .map_err(|_| JwsError::BadChain)
}

fn is_ca(cert: &X509Certificate<'_>) -> bool {
    matches!(cert.basic_constraints(), Ok(Some(bc)) if bc.value.ca)
}

fn has_extension(cert: &X509Certificate<'_>, oid: &str) -> bool {
    cert.extensions()
        .iter()
        .any(|e| e.oid.to_id_string() == oid)
}

fn hex_lower(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_apple_root_matches_its_fingerprint() {
        let anchor = TrustAnchor::apple().expect("embedded root parses and matches");
        let (_, cert) = X509Certificate::from_der(&anchor.root_der).unwrap();
        assert!(cert.subject().to_string().contains("Apple Root CA - G3"));
        assert!(is_ca(&cert));
    }

    #[test]
    fn transaction_ids_are_bounded_digits_and_app_transaction_ids_are_bounded_tokens() {
        assert!(is_transaction_id("2000000000000001"));
        assert!(!is_transaction_id(""));
        assert!(!is_transaction_id("20000000x1"));
        assert!(!is_transaction_id(
            &"1".repeat(MAX_TRANSACTION_ID_BYTES + 1)
        ));
        assert!(is_app_transaction_id("705000000001"));
        assert!(!is_app_transaction_id(""));
        assert!(!is_app_transaction_id("70500 0001"));
        assert!(!is_app_transaction_id(
            &"7".repeat(MAX_APP_TRANSACTION_ID_BYTES + 1)
        ));
    }

    #[test]
    fn the_wire_spelling_is_lower_case_and_refuses_xcode() {
        assert_eq!(
            Environment::parse_wire("sandbox"),
            Some(Environment::Sandbox)
        );
        assert_eq!(
            Environment::parse_wire("production"),
            Some(Environment::Production)
        );
        assert_eq!(Environment::parse_wire("xcode"), None);
        assert_eq!(Environment::parse_wire("Production"), None);
        assert_eq!(
            serde_json::to_string(&Environment::Production).unwrap(),
            "\"production\""
        );
        assert_eq!(Environment::Sandbox.wire(), "sandbox");
    }
}
