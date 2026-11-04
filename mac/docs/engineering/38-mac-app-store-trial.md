# Mac App Store trial and purchase

> Current design, rewritten 2026-11-03 for [ADR-0040](../adr/ADR-0040-offline-mac-trial-no-licence-server.md). The earlier design
> counted the trial on a server. Superseded server design: see git history. Shared entitlement rules:
> [36-entitlement-and-trial.md](36-entitlement-and-trial.md). Platform code rules: [37-platform-boundary.md](37-platform-boundary.md).

## 1. Decision

The Mac App Store build counts its 7-day trial on the Mac, from Apple-signed StoreKit data and the Mac's clock. There is no
licence server, no device binding, no account and no analytics. The app stays offline except for the model download the person
starts and the App Store's own calls. The Windows channel is unchanged (ADR-0023).

## 2. Identifiers

| Item | Value |
|---|---|
| Bundle id | `app.straftstudio.studio` |
| Trial product (zero-price non-consumable "7-day Trial") | `app.straftstudio.studio.trial7` |
| Paid product (non-consumable "Lifetime Access") | `app.straftstudio.studio.lifetime` |

No subscription. Family Sharing is off.

## 3. The rules

The rules are a pure function in `crates/straft-entitlement/src/mac/policy.rs`, rules 1 to 9. That file is the source of
truth; this page only summarises it.

1. Transactions from another environment than the build's (sandbox in production, for example) are ignored and counted as an
   anomaly.
2. A lifetime purchase without a revocation date wins over everything. A refunded or revoked one gives Revoked.
3. The trial start is the earlier of `purchaseDate` and `originalPurchaseDate` of the earliest verified trial transaction, or
   the guard's start when that is earlier. It never moves later. The end is the start plus exactly 168 hours.
4. When the App Store did not answer, an empty list proves nothing. A buyer stays unlocked, a known trial goes on from the
   guard's start, anything else needs an online check.
5. At or after the end: TrialEnded. After the trial or a refund the whole app locks until a purchase or a restore (ADR-0022).

## 4. Clock guard

One small record in the data protection Keychain, written by Swift through the StoreKit bridge: `v`, `env`,
`trial_start_ms`, `trial_original_tx`, `lifetime_seen`, `high_water_ms`, `last_apple_signed_ms`. Missing or unknown fields are
tolerated and corruption reads as empty.

- The floor is the latest of the high water mark, the last Apple signature and the signed date of anything read now.
- A clock more than 5 minutes behind the floor gives `ClockChanged`. A wall clock moved back while the app runs is caught by
  the session's monotonic clock.
- The clock lock cannot be closed and looks again every 5 seconds, so it clears as soon as the clock is right.
- "Check again" runs `AppStore.sync()` and `AppTransaction.refresh()`. A signature newer than any seen before the sync, within
  10 minutes of the clock, lets Apple's time confirm the clock (`G4_USES_FRESH_SIGNATURE` in `mac/adapter.rs`).
- A buyer is never locked by the network or the clock: lifetime is decided before the guard.

## 5. Code

| Part | Where |
|---|---|
| Policy, pure functions | `crates/straft-entitlement/src/mac/policy.rs` |
| Adapter and guard | `crates/straft-entitlement/src/mac/adapter.rs`, `mac/clock_guard.rs` |
| Bridge types | `crates/straft-entitlement/src/mac/bridge.rs` |
| StoreKit and Keychain in Swift | `apps/macos/Sources/StraftUI/Store/` |
| Trial start source (Plan B: first download date) | `TrialStartSource`, one file |

## 6. Accepted limits

Someone who deletes the Keychain item, or uses a second Apple Account on another Mac or macOS user, gets more trial time. The
owner accepts this (ADR-0040).

## 7. Verification

Policy and adapter are tested with a fake StoreKit. The real App Store sandbox needs the owner's developer account; the
checklist is the Mac store test list named in ADR-0040. Until it runs, the fresh signature path of rule G4 is not verified against Apple.
