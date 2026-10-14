//! OpenFrame licence server for the Mac App Store channel.
//!
//! One endpoint, `POST /v1/mac/validate`, takes the StoreKit 2 transactions (and the optional
//! `AppTransaction`) the app holds, checks each one against Apple's certificate chain, and answers
//! with a trial end time computed from the trial transaction's Apple purchase date and this
//! server's UTC clock, a lease (how long the app may count down before it checks again), a
//! lifetime flag and a revoked flag. The wire format is the app's
//! (`crates/openframe-entitlement/src/mac/server.rs`); `tests/fixtures/mac-validate-v1.json`
//! proves both sides against the same bytes. The answer is signed with an Ed25519 key that never
//! leaves the server.
//!
//! No database. Nothing is stored except one log line per request with Apple transaction ids.
//! Design and threat model: docs/engineering/38-mac-app-store-trial.md.

pub mod answer;
pub mod apple;
pub mod http;
pub mod ratelimit;

#[cfg(test)]
mod tests;
