//! Start-up for the licence server. Configuration comes only from environment variables; the
//! signing key is never read from a file in the repository and never logged.
//!
//! | Variable | Meaning |
//! |---|---|
//! | `OPENFRAME_LICENCE_SIGNING_KEY` | Required. base64url (no padding) of the 32-byte Ed25519 seed. |
//! | `OPENFRAME_APPLE_ENVIRONMENT` | Required. `Production` or `Sandbox`. One deployment serves one. |
//! | `OPENFRAME_BIND` | Address to listen on. Default `127.0.0.1:8787`. |
//! | `OPENFRAME_TLS_CERT`, `OPENFRAME_TLS_KEY` | PEM files for TLS (rustls). Required unless the bind address is loopback. |

use std::net::SocketAddr;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::SigningKey;
use openframe_licence_server::apple::{Environment, TrustAnchor};
use openframe_licence_server::http::{AppState, router, system_clock_ms};
use openframe_licence_server::ratelimit::RateLimiter;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("licence server did not start: {message}");
            ExitCode::FAILURE
        }
    }
}

fn required(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|_| format!("{name} is not set"))
}

async fn run() -> Result<(), String> {
    let seed = URL_SAFE_NO_PAD
        .decode(required("OPENFRAME_LICENCE_SIGNING_KEY")?.trim())
        .map_err(|_| "OPENFRAME_LICENCE_SIGNING_KEY is not base64url".to_string())?;
    let seed: [u8; 32] = seed
        .try_into()
        .map_err(|_| "OPENFRAME_LICENCE_SIGNING_KEY must decode to 32 bytes".to_string())?;
    let signing_key = SigningKey::from_bytes(&seed);

    let environment = Environment::parse(required("OPENFRAME_APPLE_ENVIRONMENT")?.trim())
        .ok_or("OPENFRAME_APPLE_ENVIRONMENT must be Production or Sandbox")?;
    let addr: SocketAddr = std::env::var("OPENFRAME_BIND")
        .unwrap_or_else(|_| "127.0.0.1:8787".into())
        .parse()
        .map_err(|_| "OPENFRAME_BIND is not an address and port".to_string())?;
    let tls = match (
        std::env::var("OPENFRAME_TLS_CERT"),
        std::env::var("OPENFRAME_TLS_KEY"),
    ) {
        (Ok(cert), Ok(key)) => Some((cert, key)),
        (Err(_), Err(_)) => None,
        _ => return Err("set both OPENFRAME_TLS_CERT and OPENFRAME_TLS_KEY, or neither".into()),
    };
    if tls.is_none() && !addr.ip().is_loopback() {
        return Err("TLS is required on any address other than loopback".into());
    }

    let state = Arc::new(AppState {
        anchor: TrustAnchor::apple()?,
        environment,
        signing_key,
        // 10 requests at once, then one every 6 seconds per address; at most 100 000 addresses.
        limiter: RateLimiter::new(10, 1.0 / 6.0, 100_000),
        clock: Box::new(system_clock_ms),
    });
    // The public key is not a secret. Printing it lets the owner check which key the app embeds.
    eprintln!(
        "licence server: environment={} public_key={} listening on {addr}",
        environment.as_str(),
        URL_SAFE_NO_PAD.encode(state.signing_key.verifying_key().to_bytes())
    );
    let app = router(state).into_make_service_with_connect_info::<SocketAddr>();

    match tls {
        Some((cert, key)) => {
            let _ = rustls::crypto::ring::default_provider().install_default();
            let config = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert, key)
                .await
                .map_err(|e| format!("TLS files could not be loaded: {e}"))?;
            let mut server = axum_server::bind_rustls(addr, config);
            limit_slow_clients(&mut server);
            server.serve(app).await.map_err(|e| e.to_string())
        }
        None => {
            let mut server = axum_server::bind(addr);
            limit_slow_clients(&mut server);
            server.serve(app).await.map_err(|e| e.to_string())
        }
    }
}

/// A client that sends its request headers slower than this is dropped.
fn limit_slow_clients<A>(server: &mut axum_server::Server<A>) {
    server
        .http_builder()
        .http1()
        .timer(hyper_util::rt::TokioTimer::new())
        .header_read_timeout(Duration::from_secs(10));
}
