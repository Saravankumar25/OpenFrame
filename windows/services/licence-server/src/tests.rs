//! End-to-end tests of `POST /v1/mac/validate` with a locally generated certificate chain.
//!
//! The chain mirrors Apple's: a root, an intermediate CA with Apple's intermediate marker
//! extension, and a P-256 leaf with Apple's signing marker. The server is given the test root as
//! its trust anchor. Nothing here touches the network.
//!
//! The shared fixture `tests/fixtures/mac-validate-v1.json` holds one request and the response this
//! server signs for it with [`TEST_RESPONSE_SEED`]. The entitlement crate parses and verifies the
//! same bytes (`crates/openframe-entitlement/src/mac/tests.rs`), so the app and the server are
//! proven against one wire format. Regenerate it with
//! `cargo test -- --ignored write_the_shared_wire_fixture`.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{Request, StatusCode};
use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
use http_body_util::BodyExt;
use rcgen::{BasicConstraints, CertificateParams, CustomExtension, DnType, IsCa, KeyPair};
use ring::rand::{SecureRandom, SystemRandom};
use ring::signature::{ECDSA_P256_SHA256_FIXED_SIGNING, EcdsaKeyPair};
use serde_json::{Value, json};
use tower::ServiceExt;

use crate::answer::{
    APP_STORE_RELEASE_PLACEHOLDER_MS, DEFAULT_LEASE_SECONDS, MAX_CLOCK_SKEW_MS, MAX_TRANSACTIONS,
    RESPONSE_DOMAIN, TRIAL_MS,
};
use crate::apple::{BUNDLE_ID, Environment, LIFETIME_PRODUCT_ID, TRIAL_PRODUCT_ID, TrustAnchor};
use crate::http::{AppState, MAX_BODY_BYTES, router};
use crate::ratelimit::RateLimiter;

const NOW_MS: i64 = 1_790_000_000_000; // 2026-09-21, the server's fixed clock in these tests
const DAY_MS: i64 = 24 * 60 * 60 * 1000;
const HOUR_S: i64 = 60 * 60;
const NONCE: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const APP_TX_ID: &str = "705000000001";

/// The server's response key in tests and in the shared fixture. Test only: never deployed.
const TEST_RESPONSE_SEED: [u8; 32] = *b"openframe-test-only-response-key";

const FIXTURE_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/mac-validate-v1.json"
);

const LEAF_MARKER: &[u64] = &[1, 2, 840, 113635, 100, 6, 11, 1];
const INTERMEDIATE_MARKER: &[u64] = &[1, 2, 840, 113635, 100, 6, 2, 1];

// ------------------------------------------------------------------ fake Apple chain

/// A fake Apple chain and the leaf key that signs transactions.
struct Pki {
    root_der: Vec<u8>,
    x5c: Vec<String>,
    leaf_key: EcdsaKeyPair,
}

fn ecdsa_p256() -> (EcdsaKeyPair, KeyPair) {
    let rng = SystemRandom::new();
    let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng).unwrap();
    let ring_key =
        EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8.as_ref(), &rng).unwrap();
    let rcgen_key = KeyPair::try_from(pkcs8.as_ref()).unwrap();
    (ring_key, rcgen_key)
}

fn params(cn: &str, ca: bool, marker: Option<&[u64]>) -> CertificateParams {
    let mut p = CertificateParams::new(Vec::<String>::new()).unwrap();
    p.distinguished_name.push(DnType::CommonName, cn);
    p.is_ca = if ca {
        IsCa::Ca(BasicConstraints::Unconstrained)
    } else {
        IsCa::ExplicitNoCa
    };
    if let Some(oid) = marker {
        // ASN.1 NULL, as in Apple's certificates.
        p.custom_extensions = vec![CustomExtension::from_oid_content(oid, vec![0x05, 0x00])];
    }
    p
}

fn pki_with(leaf_marker: bool) -> Pki {
    let (_, root_key) = ecdsa_p256();
    let root = params("Test Root CA", true, None)
        .self_signed(&root_key)
        .unwrap();
    let (_, int_key) = ecdsa_p256();
    let intermediate = params("Test Intermediate", true, Some(INTERMEDIATE_MARKER))
        .signed_by(&int_key, &root, &root_key)
        .unwrap();
    let (leaf_key, leaf_rcgen) = ecdsa_p256();
    let leaf = params("Test Leaf", false, leaf_marker.then_some(LEAF_MARKER))
        .signed_by(&leaf_rcgen, &intermediate, &int_key)
        .unwrap();
    let ders = [
        leaf.der().to_vec(),
        intermediate.der().to_vec(),
        root.der().to_vec(),
    ];
    Pki {
        root_der: ders[2].clone(),
        x5c: ders.iter().map(|d| STANDARD.encode(d)).collect(),
        leaf_key,
    }
}

fn pki() -> Pki {
    pki_with(true)
}

fn jws_with_key(pki: &Pki, key: &EcdsaKeyPair, payload: &Value) -> String {
    let header = json!({ "alg": "ES256", "x5c": pki.x5c });
    let input = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header).unwrap()),
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(payload).unwrap())
    );
    let sig = key.sign(&SystemRandom::new(), input.as_bytes()).unwrap();
    format!("{input}.{}", URL_SAFE_NO_PAD.encode(sig.as_ref()))
}

fn jws(pki: &Pki, payload: &Value) -> String {
    jws_with_key(pki, &pki.leaf_key, payload)
}

/// A transaction payload shaped like Apple's `JWSTransactionDecodedPayload`.
fn tx(product: &str, purchase_ms: i64) -> Value {
    json!({
        "transactionId": "2000000000000001",
        "originalTransactionId": "2000000000000001",
        "bundleId": BUNDLE_ID,
        "productId": product,
        "purchaseDate": purchase_ms,
        "originalPurchaseDate": purchase_ms,
        "quantity": 1,
        "type": "Non-Consumable",
        "inAppOwnershipType": "PURCHASED",
        "signedDate": NOW_MS - 1000,
        "environment": "Sandbox",
        "storefront": "USA",
        "appTransactionId": APP_TX_ID
    })
}

fn tx_id(mut t: Value, id: &str) -> Value {
    t["transactionId"] = json!(id);
    t["originalTransactionId"] = json!(id);
    t
}

/// An app transaction payload shaped like Apple's `JWSAppTransactionDecodedPayload`. Its
/// `originalPurchaseDate` (the app's first download) is far in the past on purpose: it must never
/// become the trial start.
fn app_tx() -> Value {
    json!({
        "bundleId": BUNDLE_ID,
        "appAppleId": 6_700_000_000_i64,
        "applicationVersion": "1",
        "originalApplicationVersion": "1",
        "originalPurchaseDate": NOW_MS - 300 * DAY_MS,
        "receiptType": "Sandbox",
        "receiptCreationDate": NOW_MS - 1000,
        "signedDate": NOW_MS - 1000,
        "appTransactionId": APP_TX_ID,
        "originalPlatform": "macOS"
    })
}

// ------------------------------------------------------------------ server under test

struct Server {
    app: axum::Router,
    public_key: VerifyingKey,
}

fn server_with(root_der: Vec<u8>, environment: Environment, now_ms: i64, seed: [u8; 32]) -> Server {
    let signing_key = SigningKey::from_bytes(&seed);
    let public_key = signing_key.verifying_key();
    let state = Arc::new(AppState {
        anchor: TrustAnchor::from_der(root_der),
        environment,
        signing_key,
        limiter: RateLimiter::new(1000, 1000.0, 1000),
        clock: Box::new(move || now_ms),
    });
    let app = router(state).layer(MockConnectInfo(SocketAddr::from(([203, 0, 113, 1], 40000))));
    Server { app, public_key }
}

fn server_for(pki: &Pki) -> Server {
    let mut seed = [0u8; 32];
    SystemRandom::new().fill(&mut seed).unwrap();
    server_with(pki.root_der.clone(), Environment::Sandbox, NOW_MS, seed)
}

fn request(env: &str, app: Option<String>, transactions: &[String]) -> Value {
    json!({
        "schema": 1,
        "nonce": NONCE,
        "environment": env,
        "appTransactionJws": app,
        "transactionJws": transactions
    })
}

async fn post_raw(server: &Server, body: Vec<u8>) -> (StatusCode, Vec<u8>) {
    let request = Request::post("/v1/mac/validate")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    let response = server.app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .to_vec();
    (status, bytes)
}

async fn post_value(server: &Server, body: &Value) -> (StatusCode, Value) {
    let (status, bytes) = post_raw(server, serde_json::to_vec(body).unwrap()).await;
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn post(server: &Server, transactions: &[String]) -> (StatusCode, Value) {
    post_value(server, &request("sandbox", None, transactions)).await
}

/// The check the app performs (`verify_response` in the entitlement crate): the signature over
/// `RESPONSE_DOMAIN || payload bytes`, strict, then the payload, then the nonce.
fn app_check(response: &Value, key: &VerifyingKey, nonce: &str) -> Result<Value, &'static str> {
    let payload = URL_SAFE_NO_PAD
        .decode(response["payload"].as_str().ok_or("no payload")?)
        .map_err(|_| "payload")?;
    let signature = URL_SAFE_NO_PAD
        .decode(response["signature"].as_str().ok_or("no signature")?)
        .map_err(|_| "signature")?;
    let signature = Signature::from_slice(&signature).map_err(|_| "signature")?;
    let mut message = RESPONSE_DOMAIN.to_vec();
    message.extend_from_slice(&payload);
    key.verify_strict(&message, &signature)
        .map_err(|_| "signature")?;
    let body: Value = serde_json::from_slice(&payload).map_err(|_| "body")?;
    if body["nonce"] != nonce {
        return Err("nonce");
    }
    Ok(body)
}

/// A 200 answer whose signature verifies and whose nonce is the one sent.
fn checked(server: &Server, status: StatusCode, response: &Value) -> Value {
    assert_eq!(status, StatusCode::OK, "{response}");
    let body = app_check(response, &server.public_key, NONCE).expect("the app's check passes");
    assert_eq!(body["schema"], 1);
    assert_eq!(body["environment"], "sandbox");
    assert_eq!(body["serverTimeMs"], NOW_MS);
    body
}

async fn refused(server: &Server, transactions: &[String]) -> (StatusCode, String) {
    let (status, body) = post(server, transactions).await;
    (status, body["error"].as_str().unwrap_or("").to_string())
}

// ------------------------------------------------------------------ the decision

#[tokio::test]
async fn a_running_trial_ends_168_hours_after_apples_purchase_date() {
    let pki = pki();
    let server = server_for(&pki);
    let bought = NOW_MS - DAY_MS;
    let (status, resp) = post(&server, &[jws(&pki, &tx(TRIAL_PRODUCT_ID, bought))]).await;
    let body = checked(&server, status, &resp);
    assert_eq!(body["trial"]["trialEndsAtMs"], bought + TRIAL_MS);
    assert_eq!(body["trial"]["remainingSeconds"], 6 * 24 * HOUR_S);
    assert_eq!(body["trial"]["appleTransactionId"], "2000000000000001");
    assert_eq!(body["trial"]["leaseSeconds"], DEFAULT_LEASE_SECONDS);
    assert_eq!(body["lifetime"], false);
    assert_eq!(body["revoked"], false);
    // The payload has exactly the fields the app reads.
    let mut keys: Vec<_> = body.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "environment",
            "lifetime",
            "nonce",
            "revoked",
            "schema",
            "serverTimeMs",
            "trial"
        ]
    );
    let mut keys: Vec<_> = body["trial"].as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "appleTransactionId",
            "leaseSeconds",
            "remainingSeconds",
            "trialEndsAtMs"
        ]
    );
}

#[tokio::test]
async fn an_expired_trial_has_zero_seconds_left_and_a_zero_lease() {
    let pki = pki();
    let server = server_for(&pki);
    let bought = NOW_MS - 8 * DAY_MS;
    let (status, resp) = post(&server, &[jws(&pki, &tx(TRIAL_PRODUCT_ID, bought))]).await;
    let body = checked(&server, status, &resp);
    assert_eq!(body["trial"]["remainingSeconds"], 0);
    assert_eq!(body["trial"]["leaseSeconds"], 0);
    assert_eq!(body["trial"]["trialEndsAtMs"], bought + TRIAL_MS);
    assert_eq!(body["lifetime"], false);
}

#[tokio::test]
async fn the_lease_is_six_hours_and_never_beyond_the_remaining_trial() {
    let pki = pki();
    let server = server_for(&pki);
    // One hour and a half left: the lease is the time left, not six hours.
    let bought = NOW_MS - TRIAL_MS + 90 * 60 * 1000;
    let (status, resp) = post(&server, &[jws(&pki, &tx(TRIAL_PRODUCT_ID, bought))]).await;
    let body = checked(&server, status, &resp);
    assert_eq!(body["trial"]["remainingSeconds"], 90 * 60);
    assert_eq!(body["trial"]["leaseSeconds"], 90 * 60);

    // Plenty left: six hours.
    let (status, resp) = post(&server, &[jws(&pki, &tx(TRIAL_PRODUCT_ID, NOW_MS))]).await;
    let body = checked(&server, status, &resp);
    assert_eq!(body["trial"]["remainingSeconds"], 168 * HOUR_S);
    assert_eq!(body["trial"]["leaseSeconds"], 6 * HOUR_S);
}

#[tokio::test]
async fn a_restore_with_a_later_purchase_date_cannot_move_the_trial_start() {
    let pki = pki();
    let server = server_for(&pki);
    let mut t = tx(TRIAL_PRODUCT_ID, NOW_MS - DAY_MS);
    t["originalPurchaseDate"] = json!(NOW_MS - 10 * DAY_MS);
    let (status, resp) = post(&server, &[jws(&pki, &t)]).await;
    let body = checked(&server, status, &resp);
    assert_eq!(body["trial"]["remainingSeconds"], 0);
    assert_eq!(
        body["trial"]["trialEndsAtMs"],
        NOW_MS - 10 * DAY_MS + TRIAL_MS
    );
}

#[tokio::test]
async fn the_trial_starts_at_the_trial_purchase_never_at_the_apps_first_download() {
    let pki = pki();
    let server = server_for(&pki);
    let bought = NOW_MS - 2 * DAY_MS;
    // The app transaction says the app was first downloaded 300 days ago.
    let app = jws(&pki, &app_tx());
    let body = request(
        "sandbox",
        Some(app),
        &[jws(&pki, &tx(TRIAL_PRODUCT_ID, bought))],
    );
    let (status, resp) = post_value(&server, &body).await;
    let body = checked(&server, status, &resp);
    assert_eq!(body["trial"]["trialEndsAtMs"], bought + TRIAL_MS);
    assert_eq!(body["trial"]["remainingSeconds"], 5 * 24 * HOUR_S);
}

#[tokio::test]
async fn the_earliest_trial_transaction_decides() {
    let pki = pki();
    let server = server_for(&pki);
    let txs = [
        jws(
            &pki,
            &tx_id(tx(TRIAL_PRODUCT_ID, NOW_MS - DAY_MS), "2000000000000009"),
        ),
        jws(
            &pki,
            &tx_id(
                tx(TRIAL_PRODUCT_ID, NOW_MS - 3 * DAY_MS),
                "2000000000000008",
            ),
        ),
    ];
    let (status, resp) = post(&server, &txs).await;
    let body = checked(&server, status, &resp);
    assert_eq!(body["trial"]["appleTransactionId"], "2000000000000008");
    assert_eq!(body["trial"]["remainingSeconds"], 4 * 24 * HOUR_S);
}

#[tokio::test]
async fn a_lifetime_purchase_unlocks() {
    let pki = pki();
    let server = server_for(&pki);
    let txs = [
        jws(&pki, &tx(TRIAL_PRODUCT_ID, NOW_MS - 20 * DAY_MS)),
        jws(
            &pki,
            &tx_id(tx(LIFETIME_PRODUCT_ID, NOW_MS - DAY_MS), "2000000000000002"),
        ),
    ];
    let (status, resp) = post(&server, &txs).await;
    let body = checked(&server, status, &resp);
    assert_eq!(body["lifetime"], true);
    assert_eq!(body["revoked"], false);
}

#[tokio::test]
async fn a_revoked_lifetime_purchase_is_revoked_and_not_lifetime() {
    let pki = pki();
    let server = server_for(&pki);
    let mut t = tx(LIFETIME_PRODUCT_ID, NOW_MS - 3 * DAY_MS);
    t["revocationDate"] = json!(NOW_MS - DAY_MS);
    t["revocationReason"] = json!(0);
    let (status, resp) = post(&server, &[jws(&pki, &t)]).await;
    let body = checked(&server, status, &resp);
    assert_eq!(body["lifetime"], false);
    assert_eq!(body["revoked"], true);
}

#[tokio::test]
async fn a_revoked_trial_gives_no_trial_and_revoked() {
    let pki = pki();
    let server = server_for(&pki);
    let mut t = tx(TRIAL_PRODUCT_ID, NOW_MS - 3 * DAY_MS);
    t["revocationDate"] = json!(NOW_MS - DAY_MS);
    let (status, resp) = post(&server, &[jws(&pki, &t)]).await;
    let body = checked(&server, status, &resp);
    assert_eq!(body["trial"], Value::Null);
    assert_eq!(body["revoked"], true);
}

#[tokio::test]
async fn no_transactions_means_no_trial_and_no_purchase() {
    let pki = pki();
    let server = server_for(&pki);
    let (status, resp) = post(&server, &[]).await;
    let body = checked(&server, status, &resp);
    assert_eq!(body["trial"], Value::Null);
    assert_eq!(body["lifetime"], false);
    assert_eq!(body["revoked"], false);
}

// ------------------------------------------------------------------ the checks on signed records

#[tokio::test]
async fn a_wrong_bundle_id_is_refused() {
    let pki = pki();
    let server = server_for(&pki);
    let mut t = tx(LIFETIME_PRODUCT_ID, NOW_MS);
    t["bundleId"] = json!("com.example.other");
    assert_eq!(
        refused(&server, &[jws(&pki, &t)]).await,
        (StatusCode::UNPROCESSABLE_ENTITY, "wrong_bundle_id".into())
    );

    let mut app = app_tx();
    app["bundleId"] = json!("com.example.other");
    let (status, resp) = post_value(&server, &request("sandbox", Some(jws(&pki, &app)), &[])).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(resp["error"], "wrong_bundle_id");
}

#[tokio::test]
async fn sandbox_data_is_refused_in_production_and_production_data_in_sandbox() {
    let pki = pki();
    let mut seed = [0u8; 32];
    SystemRandom::new().fill(&mut seed).unwrap();
    let sandbox = server_for(&pki);
    let production = server_with(pki.root_der.clone(), Environment::Production, NOW_MS, seed);

    // A production transaction sent to the sandbox deployment.
    let mut t = tx(LIFETIME_PRODUCT_ID, NOW_MS);
    t["environment"] = json!("Production");
    assert_eq!(
        refused(&sandbox, &[jws(&pki, &t)]).await,
        (StatusCode::UNPROCESSABLE_ENTITY, "wrong_environment".into())
    );
    // A sandbox transaction sent to the production deployment, with a production request.
    let sandbox_tx = jws(&pki, &tx(LIFETIME_PRODUCT_ID, NOW_MS));
    let (status, resp) = post_value(&production, &request("production", None, &[sandbox_tx])).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(resp["error"], "wrong_environment");
    // A sandbox app transaction sent to the production deployment.
    let app = jws(&pki, &app_tx());
    let (status, resp) = post_value(&production, &request("production", Some(app), &[])).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(resp["error"], "wrong_environment");
    // The request names the other environment.
    let (status, resp) = post_value(&sandbox, &request("production", None, &[])).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(resp["error"], "wrong_environment");
    let (status, resp) = post_value(&production, &request("sandbox", None, &[])).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(resp["error"], "wrong_environment");
    // Xcode is never accepted, in the request or in a payload, and Apple's spelling is not the
    // wire spelling.
    for env in ["xcode", "Sandbox"] {
        let (status, resp) = post_value(&sandbox, &request(env, None, &[])).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{env}");
        assert_eq!(resp["error"], "wrong_environment");
    }
    let mut t = tx(LIFETIME_PRODUCT_ID, NOW_MS);
    t["environment"] = json!("Xcode");
    assert_eq!(
        refused(&sandbox, &[jws(&pki, &t)]).await,
        (StatusCode::UNPROCESSABLE_ENTITY, "wrong_environment".into())
    );
}

#[tokio::test]
async fn a_signature_from_another_key_is_refused() {
    let pki = pki();
    let server = server_for(&pki);
    let (other_key, _) = ecdsa_p256();
    let forged = jws_with_key(&pki, &other_key, &tx(LIFETIME_PRODUCT_ID, NOW_MS));
    assert_eq!(
        refused(&server, &[forged]).await,
        (StatusCode::UNPROCESSABLE_ENTITY, "bad_signature".into())
    );
    let forged_app = jws_with_key(&pki, &other_key, &app_tx());
    let (status, resp) = post_value(&server, &request("sandbox", Some(forged_app), &[])).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(resp["error"], "bad_signature");
}

#[tokio::test]
async fn a_tampered_payload_is_refused() {
    let pki = pki();
    let server = server_for(&pki);
    let genuine = jws(&pki, &tx(TRIAL_PRODUCT_ID, NOW_MS - 8 * DAY_MS));
    let parts: Vec<&str> = genuine.split('.').collect();
    // Same signature, payload changed to a lifetime purchase.
    let edited =
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&tx(LIFETIME_PRODUCT_ID, NOW_MS)).unwrap());
    let tampered = format!("{}.{}.{}", parts[0], edited, parts[2]);
    assert_eq!(
        refused(&server, &[tampered]).await,
        (StatusCode::UNPROCESSABLE_ENTITY, "bad_signature".into())
    );
}

#[tokio::test]
async fn a_chain_from_another_root_or_without_apples_marker_is_refused() {
    let trusted = pki();
    let server = server_for(&trusted);
    // A complete, self-consistent chain under a root the server does not trust.
    let stranger = pki();
    assert_eq!(
        refused(&server, &[jws(&stranger, &tx(LIFETIME_PRODUCT_ID, NOW_MS))]).await,
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            "bad_certificate_chain".into()
        )
    );
    let (status, resp) = post_value(
        &server,
        &request("sandbox", Some(jws(&stranger, &app_tx())), &[]),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(resp["error"], "bad_certificate_chain");

    // The trusted root copied into the third x5c slot does not help a foreign intermediate.
    let mut spliced = pki();
    spliced.x5c[2] = trusted.x5c[2].clone();
    assert_eq!(
        refused(&server, &[jws(&spliced, &tx(LIFETIME_PRODUCT_ID, NOW_MS))]).await,
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            "bad_certificate_chain".into()
        )
    );

    // A leaf without Apple's signing marker is refused even under the trusted root.
    let unmarked = pki_with(false);
    let server = server_for(&unmarked);
    assert_eq!(
        refused(&server, &[jws(&unmarked, &tx(LIFETIME_PRODUCT_ID, NOW_MS))]).await,
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            "bad_certificate_chain".into()
        )
    );
}

#[tokio::test]
async fn family_shared_unknown_products_and_other_algorithms_are_refused() {
    let pki = pki();
    let server = server_for(&pki);
    let mut t = tx(LIFETIME_PRODUCT_ID, NOW_MS);
    t["inAppOwnershipType"] = json!("FAMILY_SHARED");
    assert_eq!(refused(&server, &[jws(&pki, &t)]).await.1, "not_purchased");

    assert_eq!(
        refused(
            &server,
            &[jws(&pki, &tx("com.openframe.studio.other", NOW_MS))]
        )
        .await
        .1,
        "unknown_product"
    );

    let genuine = jws(&pki, &tx(LIFETIME_PRODUCT_ID, NOW_MS));
    let parts: Vec<&str> = genuine.split('.').collect();
    let none_header = URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(&json!({ "alg": "none", "x5c": pki.x5c })).unwrap());
    assert_eq!(
        refused(&server, &[format!("{none_header}.{}.", parts[1])])
            .await
            .1,
        "unsupported_algorithm"
    );

    assert_eq!(
        refused(&server, &["not.a-jws".to_string()]).await.1,
        "malformed_transaction"
    );
}

#[tokio::test]
async fn bad_or_duplicate_transaction_ids_are_refused() {
    let pki = pki();
    let server = server_for(&pki);
    for id in ["", "20000000x1", &"1".repeat(65)] {
        let t = tx_id(tx(TRIAL_PRODUCT_ID, NOW_MS), id);
        assert_eq!(
            refused(&server, &[jws(&pki, &t)]).await,
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                "bad_transaction_id".into()
            ),
            "{id:?}"
        );
    }
    let mut t = tx(TRIAL_PRODUCT_ID, NOW_MS);
    t["appTransactionId"] = json!("");
    assert_eq!(
        refused(&server, &[jws(&pki, &t)]).await.1,
        "bad_transaction_id"
    );

    // The same transaction sent twice.
    let once = jws(&pki, &tx(TRIAL_PRODUCT_ID, NOW_MS - DAY_MS));
    assert_eq!(
        refused(&server, &[once.clone(), once]).await,
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            "duplicate_transaction".into()
        )
    );
}

#[tokio::test]
async fn app_transaction_ids_must_match_across_transactions_and_the_app_transaction() {
    let pki = pki();
    let server = server_for(&pki);
    let trial = tx(TRIAL_PRODUCT_ID, NOW_MS - DAY_MS);
    let mut other_account = tx_id(tx(LIFETIME_PRODUCT_ID, NOW_MS), "2000000000000002");
    other_account["appTransactionId"] = json!("705999999999");

    // Two transactions from two Apple Accounts in one request.
    assert_eq!(
        refused(&server, &[jws(&pki, &trial), jws(&pki, &other_account)]).await,
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            "app_transaction_mismatch".into()
        )
    );

    // A transaction from another account than the app transaction.
    let mut app = app_tx();
    app["appTransactionId"] = json!("705999999999");
    let (status, resp) = post_value(
        &server,
        &request("sandbox", Some(jws(&pki, &app)), &[jws(&pki, &trial)]),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(resp["error"], "app_transaction_mismatch");

    // All the same: accepted.
    let (status, resp) = post_value(
        &server,
        &request("sandbox", Some(jws(&pki, &app_tx())), &[jws(&pki, &trial)]),
    )
    .await;
    checked(&server, status, &resp);
}

#[tokio::test]
async fn a_missing_app_transaction_id_is_accepted_because_apple_sends_it_from_macos_15_4_only() {
    let pki = pki();
    let server = server_for(&pki);
    let mut trial = tx(TRIAL_PRODUCT_ID, NOW_MS - DAY_MS);
    trial.as_object_mut().unwrap().remove("appTransactionId");
    let mut app = app_tx();
    app.as_object_mut().unwrap().remove("appTransactionId");

    // Neither carries one.
    let (status, resp) = post_value(
        &server,
        &request("sandbox", Some(jws(&pki, &app)), &[jws(&pki, &trial)]),
    )
    .await;
    let body = checked(&server, status, &resp);
    assert_eq!(body["trial"]["remainingSeconds"], 6 * 24 * HOUR_S);

    // Only the app transaction carries one: nothing to compare it with, accepted.
    let (status, resp) = post_value(
        &server,
        &request("sandbox", Some(jws(&pki, &app_tx())), &[jws(&pki, &trial)]),
    )
    .await;
    checked(&server, status, &resp);

    // No app transaction at all.
    let (status, resp) = post(&server, &[jws(&pki, &trial)]).await;
    checked(&server, status, &resp);
}

#[tokio::test]
async fn a_purchase_date_in_the_future_or_before_the_release_placeholder_is_refused() {
    let pki = pki();
    let server = server_for(&pki);
    let future = tx(TRIAL_PRODUCT_ID, NOW_MS + MAX_CLOCK_SKEW_MS + 1);
    assert_eq!(
        refused(&server, &[jws(&pki, &future)]).await,
        (StatusCode::UNPROCESSABLE_ENTITY, "bad_purchase_date".into())
    );
    // Within the clock skew: accepted, and the trial is a full 168 hours from Apple's date.
    let (status, resp) = post(
        &server,
        &[jws(&pki, &tx(TRIAL_PRODUCT_ID, NOW_MS + MAX_CLOCK_SKEW_MS))],
    )
    .await;
    let body = checked(&server, status, &resp);
    assert_eq!(
        body["trial"]["trialEndsAtMs"],
        NOW_MS + MAX_CLOCK_SKEW_MS + TRIAL_MS
    );

    let ancient = tx(TRIAL_PRODUCT_ID, APP_STORE_RELEASE_PLACEHOLDER_MS - 1);
    assert_eq!(
        refused(&server, &[jws(&pki, &ancient)]).await.1,
        "bad_purchase_date"
    );
    let mut ancient_original = tx(TRIAL_PRODUCT_ID, NOW_MS - DAY_MS);
    ancient_original["originalPurchaseDate"] = json!(APP_STORE_RELEASE_PLACEHOLDER_MS - 1);
    assert_eq!(
        refused(&server, &[jws(&pki, &ancient_original)]).await.1,
        "bad_purchase_date"
    );
    // A revocation before the purchase or in the future.
    for revoked_at in [NOW_MS - 2 * DAY_MS, NOW_MS + MAX_CLOCK_SKEW_MS + 1] {
        let mut t = tx(LIFETIME_PRODUCT_ID, NOW_MS - DAY_MS);
        t["revocationDate"] = json!(revoked_at);
        assert_eq!(
            refused(&server, &[jws(&pki, &t)]).await.1,
            "bad_purchase_date"
        );
    }
}

// ------------------------------------------------------------------ the request

#[tokio::test]
async fn an_oversized_body_is_refused_before_parsing() {
    let pki = pki();
    let server = server_for(&pki);
    let mut body =
        format!(r#"{{"schema":1,"nonce":"{NONCE}","environment":"sandbox","transactionJws":[""#)
            .into_bytes();
    body.resize(MAX_BODY_BYTES + 1, b'a');
    let (status, _) = post_raw(&server, body).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn too_many_transactions_are_refused() {
    let pki = pki();
    let server = server_for(&pki);
    let many = vec!["a.b.c".to_string(); MAX_TRANSACTIONS + 1];
    assert_eq!(
        refused(&server, &many).await,
        (StatusCode::BAD_REQUEST, "too_many_transactions".into())
    );
}

#[tokio::test]
async fn a_missing_or_malformed_nonce_is_refused() {
    let pki = pki();
    let server = server_for(&pki);
    let (status, resp) = post_value(
        &server,
        &json!({ "schema": 1, "environment": "sandbox", "transactionJws": [] }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(resp["error"], "missing_nonce");

    for nonce in [
        "short".to_string(),
        NONCE.to_uppercase(),
        format!("{NONCE}00"),
        "g".repeat(64),
    ] {
        let mut body = request("sandbox", None, &[]);
        body["nonce"] = json!(nonce);
        let (status, resp) = post_value(&server, &body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{nonce}");
        assert_eq!(resp["error"], "bad_nonce");
    }
}

#[tokio::test]
async fn an_unknown_schema_and_a_client_clock_field_are_refused() {
    let pki = pki();
    let server = server_for(&pki);
    let mut body = request("sandbox", None, &[]);
    body["schema"] = json!(2);
    let (status, resp) = post_value(&server, &body).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(resp["error"], "unsupported_schema");

    let mut body = request("sandbox", None, &[]);
    body["clientTimeMs"] = json!(0);
    let (status, resp) = post_value(&server, &body).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(resp["error"], "bad_request");

    // The old field names are not accepted either.
    let (status, _) = post_value(
        &server,
        &json!({ "schema": 1, "nonce": NONCE, "environment": "sandbox", "transactions": [] }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// ------------------------------------------------------------------ the signed answer

#[tokio::test]
async fn an_edited_answer_or_an_answer_from_another_key_fails_the_apps_check() {
    let pki = pki();
    let server = server_for(&pki);
    let (status, resp) = post(
        &server,
        &[jws(&pki, &tx(TRIAL_PRODUCT_ID, NOW_MS - 8 * DAY_MS))],
    )
    .await;
    checked(&server, status, &resp);

    // The payload edited to claim a lifetime purchase, the signature kept.
    let mut body: Value = serde_json::from_slice(
        &URL_SAFE_NO_PAD
            .decode(resp["payload"].as_str().unwrap())
            .unwrap(),
    )
    .unwrap();
    body["lifetime"] = json!(true);
    let mut edited = resp.clone();
    edited["payload"] = json!(URL_SAFE_NO_PAD.encode(serde_json::to_vec(&body).unwrap()));
    assert_eq!(
        app_check(&edited, &server.public_key, NONCE),
        Err("signature")
    );

    let other = server_for(&pki);
    assert_eq!(app_check(&resp, &other.public_key, NONCE), Err("signature"));

    // Another request's nonce: a recorded answer played back.
    assert_eq!(
        app_check(&resp, &server.public_key, &"f".repeat(64)),
        Err("nonce")
    );

    // A signature without the domain prefix.
    let payload = URL_SAFE_NO_PAD
        .decode(resp["payload"].as_str().unwrap())
        .unwrap();
    let key = SigningKey::from_bytes(&TEST_RESPONSE_SEED);
    let bare = json!({
        "payload": resp["payload"],
        "signature": URL_SAFE_NO_PAD.encode(ed25519_dalek::Signer::sign(&key, &payload).to_bytes())
    });
    assert_eq!(
        app_check(&bare, &key.verifying_key(), NONCE),
        Err("signature")
    );
}

#[tokio::test]
async fn too_many_requests_from_one_address_are_limited() {
    let pki = pki();
    let mut seed = [0u8; 32];
    SystemRandom::new().fill(&mut seed).unwrap();
    let state = Arc::new(AppState {
        anchor: TrustAnchor::from_der(pki.root_der.clone()),
        environment: Environment::Sandbox,
        signing_key: SigningKey::from_bytes(&seed),
        limiter: RateLimiter::new(2, 0.0, 10),
        clock: Box::new(|| NOW_MS),
    });
    let server = Server {
        public_key: state.signing_key.verifying_key(),
        app: router(state).layer(MockConnectInfo(SocketAddr::from(([203, 0, 113, 9], 1)))),
    };
    assert_eq!(post(&server, &[]).await.0, StatusCode::OK);
    assert_eq!(post(&server, &[]).await.0, StatusCode::OK);
    assert_eq!(post(&server, &[]).await.0, StatusCode::TOO_MANY_REQUESTS);
}

// ------------------------------------------------------------------ the shared wire fixture

fn fixture_server(fixture: &Value) -> Server {
    let root = STANDARD
        .decode(fixture["appleTestRootDer"].as_str().unwrap())
        .unwrap();
    let now = fixture["serverTimeMs"].as_i64().unwrap();
    server_with(root, Environment::Sandbox, now, TEST_RESPONSE_SEED)
}

#[tokio::test]
async fn the_shared_fixture_request_produces_the_shared_fixture_response_byte_for_byte() {
    let text = std::fs::read_to_string(FIXTURE_PATH).expect("the shared fixture exists");
    let fixture: Value = serde_json::from_str(&text).unwrap();
    let server = fixture_server(&fixture);
    assert_eq!(
        fixture["responsePublicKey"],
        URL_SAFE_NO_PAD.encode(server.public_key.to_bytes()),
        "the fixture's public key is the test key's"
    );
    let (status, resp) = post_value(&server, &fixture["request"]).await;
    assert_eq!(status, StatusCode::OK, "{resp}");
    // Ed25519 is deterministic and the payload is the same JSON, so the bytes are identical.
    assert_eq!(resp, fixture["response"]);

    let nonce = fixture["request"]["nonce"].as_str().unwrap();
    let body = app_check(&resp, &server.public_key, nonce).unwrap();
    assert_eq!(body["environment"], "sandbox");
    assert_eq!(body["trial"]["leaseSeconds"], DEFAULT_LEASE_SECONDS);
    assert_eq!(body["lifetime"], false);
    assert_eq!(body["revoked"], false);
}

/// Writes `tests/fixtures/mac-validate-v1.json`. Run on purpose only:
/// `cargo test -- --ignored write_the_shared_wire_fixture`.
#[tokio::test]
#[ignore = "writes the shared fixture; run on purpose when the wire format changes"]
async fn write_the_shared_wire_fixture() {
    let pki = pki();
    let server_time = NOW_MS;
    let trial = tx(TRIAL_PRODUCT_ID, server_time - DAY_MS);
    let request = request("sandbox", Some(jws(&pki, &app_tx())), &[jws(&pki, &trial)]);
    let mut fixture = json!({
        "description": "One Mac licence check in wire format version 1: the request the app sends and the response the licence server signs for it with its test-only key. The server test replays the request and must produce this response byte for byte; the entitlement crate test parses the request with its own types and verifies the response with responsePublicKey. The certificates and keys are generated for tests and are not Apple's.",
        "wireVersion": 1,
        "serverTimeMs": server_time,
        "responsePublicKey": "",
        "appleTestRootDer": STANDARD.encode(&pki.root_der),
        "trialPurchaseDateMs": server_time - DAY_MS,
        "request": request,
        "response": Value::Null
    });
    let server = fixture_server(&fixture);
    fixture["responsePublicKey"] = json!(URL_SAFE_NO_PAD.encode(server.public_key.to_bytes()));
    let (status, resp) = post_value(&server, &fixture["request"]).await;
    assert_eq!(status, StatusCode::OK, "{resp}");
    fixture["response"] = resp;
    let mut text = serde_json::to_string_pretty(&fixture).unwrap();
    text.push('\n');
    std::fs::create_dir_all(std::path::Path::new(FIXTURE_PATH).parent().unwrap()).unwrap();
    std::fs::write(FIXTURE_PATH, text).unwrap();
}
