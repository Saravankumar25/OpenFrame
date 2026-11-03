# ADR-0040: Offline Mac trial with a clock guard, no licence server

- **Status:** Accepted 2026-11-03 (owner decision, release package `OpenFrame_Release_Package/`).
- **Date:** 2026-11-03
- **Supersedes:** the licence server in [ADR-0018](ADR-0018-distribution-licensing-commerce.md) decision 3 and the
  whole of `docs/engineering/38-mac-app-store-trial.md` as a design for a server. Amends
  [ADR-0022](ADR-0022-mac-trial-end-lock.md): the lock stays and gains the clock lock.
- **Applies to:** the Mac App Store channel. The Windows channel is unchanged (ADR-0023).

## Context

The Mac trial could not count days without a licence server: a trial transaction with no verified server verdict meant
"check online", and the seconds left came from the server only. The owner decided there is no licence server, no device
binding, no accounts and no analytics. The app stays offline except for the model download the person starts and the
stores' own calls.

## Decision

1. **The trial is counted on the Mac from Apple-signed StoreKit data and the Mac's clock.** A zero-price non-consumable
   ("7-day Trial") starts it and a paid non-consumable ("Lifetime Access") unlocks the app for life. No subscription.
   Family Sharing off. This reverses the old rule "the wall clock is never an input".
2. **The start** is the earlier of `purchaseDate` and `originalPurchaseDate` of the earliest verified trial transaction,
   never the first download date. Once known it never moves later. The end is the start plus exactly 168 hours.
3. **The policy** is the table in `crates/openframe-entitlement/src/mac/policy.rs`, rules 1 to 9. "The store is reachable"
   means `Product.products(for:)` returned both products in this evaluation. `currentEntitlements` does not fail offline,
   so an empty list then proves nothing and never reads as "no trial yet".
4. **The clock guard** is one small record in the data protection Keychain, in the app's own access group, read and
   written by Swift through the StoreKit bridge: `{ v, env, trial_start_ms, trial_original_tx, lifetime_seen,
   high_water_ms, last_apple_signed_ms }`. It tolerates missing and unknown fields and reads corruption as empty, counted
   as an anomaly. `env` is added beyond the package's list so that a record written by a TestFlight or sandbox build is
   never used by a production build (test and real purchases never mix).
5. **Rules G1 to G4.** The floor is the latest of the high water mark, the last Apple signature and the signed date of
   anything read now. A clock more than 5 minutes behind the floor gives `ClockChanged`. The high water mark rises when
   the check passes and is saved at launch, on coming to the front, on `Transaction.updates` and every 5 minutes during a
   trial. The session's monotonic clock notices a wall clock moved back while the app runs. The clock lock cannot be closed
   and looks again every 5 seconds without a click, so it clears as soon as the clock is right.
6. **G4, recovery from a wrong future clock.** "Check again" runs `AppStore.sync()` and `AppTransaction.refresh()`. When a
   signature **newer than any seen before the sync** lies within 10 minutes of the clock, Apple's time confirms the clock:
   the high water mark comes back to the clock. The "newer than seen" condition is added beyond the package's wording,
   because a record's old signed date would otherwise let a person set the clock back to the day of the trial purchase and
   win the whole trial back (found by test).
   - **In use: the fresh signature path** (`G4_USES_FRESH_SIGNATURE = true` in `mac/adapter.rs`).
   - **Not verified:** that a refreshed record really carries a fresh `signedDate` can only be seen in the App Store
     sandbox, which needs the owner's developer account. If it does not, flip the constant: the fallback caps the high water
     mark at the last Apple signature plus 2 days (written and tested, `cap_high_water_at_apple_date`). The check is item 6
     of `docs/release/mac-store-test.md`.
7. **A buyer is never locked by the network or the clock.** Lifetime is decided before the clock guard. `lifetime_seen`
   keeps a buyer unlocked while the store shows nothing, online or offline. A refund arrives as a transaction with a
   revocation date (the Swift source also reads `Transaction.latest(for:)`, because `currentEntitlements` drops a refunded
   product), gives `Revoked` and clears `lifetime_seen`.
8. **The trial start source** sits behind one small interface, `TrialStartSource`. Plan B, not built: should App Store
   Connect have no free price, starting at the first download date is a change to one file.

## Disagreements with the package, settled here

- **Reachable, nothing listed, `lifetime_seen` set, no trial in the guard.** Package rule 4 read literally gives
  `TrialNotStarted`. Chosen: `Purchased`, because a buyer signed out of the App Store must not be sent back to the first
  launch screen, and a refund has its own path (decision 7).
- **Second Apple Account.** Package file 04 M8 expects a new trial for a second account, while rule 7 and the guard rules say
  the start never moves later. Chosen: rule 7. On one Keychain (same Mac, same macOS user) the earlier start is kept. A
  second account on another Mac or macOS user gets its own trial. The manual checklist states this.

## Accepted limits

- A person who deletes the Keychain item, or who uses a second Apple Account on another Mac or macOS user, gets more trial
  time. The owner accepts this. No device binding, no server, no DeviceCheck.

## Consequences

- `services/licence-server`, `mac/server.rs`, `LicenceServerClient.swift` and the retired Tauri Mac layer
  (`apps/desktop/src-tauri/src/platform`, `macos/StoreKitBridge`) are deleted, with their dependencies, addresses and
  documents.
- The privacy manifest declares no collected data.
- The trial works with the network cable pulled after the first online start.
- The Rust change must be repeated in the Windows tree (ADR-0034); the Windows tree keeps its own Store adapter.
