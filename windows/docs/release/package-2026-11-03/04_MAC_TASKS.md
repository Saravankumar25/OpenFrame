# 04. Mac tasks (Mac App Store, sandboxed `.app`)

Read 02 first. This file is the Mac-specific design and work list. Minimum macOS is 14 (`Package.swift` has `.macOS(.v14)`; the UI uses `@Observable`). Apple silicon only (the AI engine is an arm64 Metal build).

## The big change in one paragraph

Today the Mac trial cannot count days without a licence server: `policy.rs` rule 6 says a trial transaction with no verified server verdict means "check online", and rules 7 to 9 take the seconds left from the server only. The owner has decided: **no licence server, no device binding, fully offline trial.** So the trial is now computed on the Mac from Apple-signed StoreKit data and the Mac's clock, with a local clock guard. This reverses one rule of the old design ("the wall clock is never an input"). Say so in a new ADR (next free number) that supersedes the licence-server decisions (search `docs/adr` for "licence server" and "doc 38"; fix every document that still describes the server).

## What stays from the real code

- Zero-price non-consumable that starts the trial, paid non-consumable that unlocks, no subscription, Family Sharing off.
- Only `.verified` StoreKit results cross the bridge (`keep_verified`). Unverified ones are dropped and counted.
- Apple's dates, never the app's first-download date: the trial starts at the trial transaction's own purchase date (the earlier of `purchaseDate` and `originalPurchaseDate` of that same transaction). `AppTransaction.originalPurchaseDate` is never used.
- `appTransactionID` is optional (macOS 15.4+), used only to link an account to the app.
- The environment comes from the verified `AppTransaction` (Production, Sandbox, Xcode). Keep this: **Apple's App Review tests the production build in the Sandbox environment**, so a sandbox purchase must work in a build that Apple reviews. Do not hard-code Production.
- Policy rules 1 to 5 of `policy.rs` (environment filter, lifetime wins, refunded lifetime gives Revoked, no trial gives not started, revoked trial gives Revoked) stay.
- Store mapping to `EntitlementState`, `notices::due`, screens and flags stay (file 02).

## Tasks

### M1. Remove the licence-server design (both trees)
Delete or retire, and fix every reference, test and doc:
- `services/licence-server` (in `mac/` and `windows/`), including its CI job, Dockerfile or deploy files, and the rate limiter, signing and Apple JWS code it holds.
- `crates/straft-entitlement/src/mac/server.rs` and its re-exports in `mac/mod.rs`; the server calls, nonces, lease, `ServerView`, `ValidateRequest`, `ResponseBody`, `TrialVerdict`, `verify_response`, `NonceSource`, `UnavailableServer`, `NoNonces`, and `DEFAULT_LEASE_SECONDS` in `adapter.rs` and `unavailable.rs`.
- `LicenceServerClient.swift`, `LicenceServerCallbacks`, `StoreRuntime.server`, `CoreStartOptions.licenceServer`, the signed JWS fields that existed only for the server (`signed_jws` on transactions can go, or stay if the bridge tests need it; decide and keep the bridge small).
- `src-tauri/src/platform/mac_inputs.rs` placeholders (`.invalid` URLs and the public-key placeholder; the file belongs to the retired Tauri Mac shell).
- `scripts/app-addresses.json` entries for `licence.straft.invalid` and `licence-sandbox.straft.invalid`; the governance test must stay green.
- Release workflow: remove any server secret or variable (`STRAFT_LICENCE_SIGNING_KEY` and friends) and any job that builds or deploys the server.
- `network.client` stays in the sandbox entitlements (the AI model download and the engine on `127.0.0.1` need it). Update its comment so it no longer mentions the licence server.

### M2. New offline policy (`MacTrialPolicy`)
Inputs now: verified transactions, a `store_reachable` flag, `now_wall_ms` (the Mac clock, UTC), `now_mono_s` (session monotonic clock), and the persisted guard record (M3). Rules, in order (1 to 5 are kept from today):

| # | Rule | Result |
|---|---|---|
| 1 | Ignore transactions from another environment than the build's, or not well formed (count as anomaly). | kept |
| 2 | A lifetime transaction with no revocation date. | `Purchased`. Set `guard.lifetime_seen = true`. |
| 3 | A lifetime transaction with a revocation date and no valid lifetime. | `Revoked` |
| 4 | No trial transaction, `store_reachable` true, `guard` has no trial. | `TrialNotStarted` |
| 5 | A trial transaction with a revocation date. | `Revoked` (kept) |
| 6 (new) | Store not reachable and no verified transaction came back. If `guard.lifetime_seen`: `Purchased`. Else if `guard.trial_start_ms` is set: continue with that start (rule 8). Else: `NeedsOnlineCheck` (first ever launch offline, or a new Mac). | offline behaviour |
| 7 (new) | Trial start = `min(purchaseDate, originalPurchaseDate)` of the earliest verified trial transaction. If the guard already holds a start, take the earlier of the two; a restore or a second trial transaction never moves the start later. Trial end = start + 168 hours. | start and end |
| 8 (new) | Clock guard (M3, rules G1 to G4). If the clock is behind the trusted floor: `ClockChanged`. | new state |
| 9 (new) | `now_wall_ms >= end`: `TrialEnded`. Else `TrialActive{ days_left = clamp(ceil((end - now) / 86 400 000), 1, 7) }`. | result |

Notes:
- "`store_reachable`" means the app could load the two products from the App Store in this evaluation (`Product.products(for:)` returned both). Do not infer it from `currentEntitlements`: **that sequence does not throw when the device is offline; it returns what the device cached**, and Apple notes it can be out of date until the device is online. So an empty list while offline proves nothing and must never read as "no trial yet".
- A buyer is never locked because the network is down (rule 6, `lifetime_seen`). A refund is noticed when the device is next online (`Transaction.updates` or the next refresh) and the state becomes `Revoked`.
- Seconds are never taken from a server any more. Remove the monotonic countdown and lease from the policy. The monotonic clock is still useful inside one session to notice the wall clock jumping backwards while the app runs (G3).
- The wall clock is now an input. Document this in the ADR and in the module header, replacing the sentence that says it never is.

### M3. The clock guard and the persisted record
Persist one small record in the Keychain (not a plain file: a file in the container is easy to restore from a backup; the Keychain survives app deletion and reinstall). Swift owns the Keychain; Rust asks through a bridge callback (`load_guard` and `save_guard`), in the same style as the other bridges. Use the data protection keychain in the app's own access group (`REPLACE_WITH_TEAM_ID.app.straftstudio.studio`, never change it after release).

Record (versioned, serde defaults, tolerate unknown fields, tolerate corruption by treating it as empty and counting an anomaly):
`{ v: 1, trial_start_ms?, trial_original_tx?, lifetime_seen, high_water_ms, last_apple_signed_ms }`

Rules:
- **G1. Floor.** `trusted_floor = max(high_water_ms, last_apple_signed_ms, signedDate of any transaction read now)`. `Transaction.signedDate` is the time Apple signed that record, so the real time is never earlier than it.
- **G2. Rollback.** If `now_wall_ms < trusted_floor - 5 minutes`, the state is `ClockChanged`. The 5 minutes absorbs normal clock corrections.
- **G3. Advance.** When the check passes, `high_water_ms = max(high_water_ms, now_wall_ms)`. Save at launch, when the app comes to the front, on `Transaction.updates`, and every 5 minutes while a trial is active. Also compare the wall clock with the session monotonic clock: if the wall clock moved back while the app was running, treat it as G2.
- **G4. Recovery from a wrong future clock** (a clock set to next year by accident, then fixed, would otherwise lock the trial for good). When `ClockChanged` is the result and the store is reachable, force a fresh signed read (`AppStore.sync()` as part of `Check again`). If the fresh `signedDate` is within 10 minutes of the wall clock, Apple's time confirms the clock is right: reset `high_water_ms` to `now_wall_ms` and re-evaluate. **Verify in the sandbox that a refreshed transaction carries a fresh `signedDate`.** If it does not, fall back to capping `high_water_ms` at `last_apple_signed_ms + 2 days`, and record which one you used in the ADR.
- The guard only ever raises `high_water_ms` and `last_apple_signed_ms`, never lowers them, and never moves `trial_start_ms` later.
- Accepted limits, written in the ADR: a person who deletes the Keychain item, or who uses a second Apple Account, gets more trial time. The owner accepts this. Do not add device binding, a server or DeviceCheck.

`ClockChanged` screen (new, cannot be closed, whole app locked like `TrialEnded`): title `Your Mac's clock looks wrong`; body `Straft Studio checks the date to count your free trial. Turn on Set time and date automatically in System Settings, then choose Check again. Your projects are safe.` Buttons `Check again` and `Need help? Contact us`. While it is open, re-check every 5 seconds without a click so it clears as soon as the clock is fixed. Mock 201.

### M4. StoreKit source (`StoreKitSource.swift`, `StoreKitBridgeImpl.swift`)
- Keep `Transaction.currentEntitlements`, `Transaction.updates`, `Storefront.updates`, `ownershipType == .purchased` (Family Sharing off).
- Add `signedDate` to the transaction record the bridge returns.
- Add `Product.products(for:)` for both product IDs, returning `displayPrice` (for M5) and the reachability signal.
- `AppStore.sync()` only for `Restore purchase` and `Check again`, never on every launch (it may show an Apple sign-in sheet).
- `Product.purchase()` for both products, using the `purchase` variant that takes the app's window as the purchase scene if the SDK offers one, otherwise plain `purchase()`. Handle `.pending` (Ask to Buy or approval) with plain words, `.userCancelled` quietly, `.success(.unverified)` as a refusal with plain words.
- After a trial purchase succeeds, finish the transaction (`transaction.finish()`) and re-evaluate at once.
- Revalidate on launch, foreground, wake, `Transaction.updates`, `Storefront.updates` (another Apple Account), every 5 minutes while a trial is active (cheap, local), and at the computed trial end.

### M5. Screens (Mac)
Mock numbers refer to `Trial_Licensing_Mocks/`.
- **First launch (187).** `Welcome to Straft Studio`. Card 1, Free trial: `7 days. Everything included. No payment now. When the trial ends, Straft Studio locks until you buy the full version. Your projects stay safe on this Mac. The full version is a one-time purchase of {price}.` Button `Start free trial`. Card 2: `Buy the full version. One-time purchase through the App Store. Pay once, use it for good.` Button `Buy full version`. Below: `Restore purchase`, and `Need help? Contact us`. `{price}` is `Product.displayPrice` of the lifetime product, read at run time. Enable `Start free trial` only after both products loaded. If they fail to load, show the Check screen (192), not a blank or a guess.
- **Chip (188) and reminder (189).** File 02, sections 3 and 4. The reminder shows on Home and over a project.
- **Lock (190).** `Your free trial has ended`, with `{price}` under `Buy full version`, plus `Restore purchase` and `Need help? Contact us`.
- **Check (192).** `Checking your purchase`. First launch offline lands here, with `Try again`.
- **Clock (201).** M3.
- **Apple's sheet (191).** Apple owned; not drawn by us.
- Never type a price anywhere. A test must fail if a currency symbol or a digit sequence next to a currency code appears in the Swift licence strings.

### M6. Identity and product IDs (decided)
- Bundle ID `app.straftstudio.studio`. Products `app.straftstudio.studio.trial7` and `app.straftstudio.studio.lifetime`. Change the constants in `StoreKitSource.swift`, `bridge.rs` (`TRIAL_PRODUCT_ID`, `LIFETIME_PRODUCT_ID`), the `.storekit` file, `Info.plist`, entitlements, build scripts and tests. The old `app.straftstudio.studio` names are internal-only; no released user has them, so no migration is needed. Internal testers' data from the old non-sandboxed disk image build does not carry over to the sandbox container; note this in the handoff.
- `StraftStudio.storekit`: product 1 `trial7`, display name exactly `7-day Trial` (Apple's guideline names the pattern "XX-day Trial"), price `0.00`, non-consumable, `familyShareable: false`. Product 2 `lifetime`, display name `Lifetime Access`, non-consumable, `familyShareable: false`, price a clearly marked test value (the real price is set in App Store Connect by the owner). Remove "No credit card" style marketing claims from product descriptions; keep them true and short.
- **Plan B, not built now.** The guideline-blessed free "XX-day Trial" item at price tier 0 is the plan. A 2025 forum post reported no free price point in App Store Connect, and this has not been confirmed either way. The owner will see the real answer when he creates the product. Keep the trial-start source behind one small interface (`TrialStartSource`) so that, if Apple has no free price, the fallback (start at `AppTransaction.originalPurchaseDate`, the first download) is a one-file change. Do not build the fallback unless the owner says so.
- App Review notes (write them to `docs/release/mac-app-review-notes.md`): how the trial works, how to restore, that the app is offline, that the AI model is an optional download the user starts, and a demo path for the reviewer (sandbox Apple Account, tap Start free trial, tap Buy full version).

### M7. Sandbox, privacy and the build for the Mac App Store
Current store entitlements (real file): app-sandbox, user-selected read-write, app-scope bookmarks, network client and server (the engine on `127.0.0.1`), audio input, print. Keep the minimum; add nothing without a reason written in the file.
- The store build keeps the user's work inside the app container (`StorageRules.canRelocate(storeBuild:)` is false). A Store update keeps the container; the bundle ID and access group must therefore never change (file 05).
- Add `PrivacyInfo.xcprivacy` (Apple requires a privacy manifest and "required reason" declarations for APIs such as `UserDefaults`, file timestamps, system boot time and disk space). Declare no tracking and no collected data. Audit the Swift and Rust code for required-reason API use and declare each with its reason code.
- `Info.plist`: `LSMinimumSystemVersion` 14.0, `LSApplicationCategoryType` (productivity or video, owner chooses; placeholder `REPLACE_WITH_APP_CATEGORY`), `ITSAppUsesNonExemptEncryption` false if the app only uses Apple's system HTTPS and no custom cryptography (confirm; if the app does use its own cryptography the export declaration changes), `NSHumanReadableCopyright` `Copyright 2026 Vighnajith Advisories Private Limited`, microphone usage string for voice notes.
- Signing and upload (`mac/.github/workflows/release.yml` is currently a copy of the Windows workflow and must be rewritten for the Mac): archive; sign the app and the `llama-server` helper with the Apple Distribution identity and the store entitlements (`StraftStudio-store.entitlements`, `llama-server-store.entitlements`; the helper inherits the sandbox); embed the provisioning profile; build the installer package with the Mac Installer Distribution identity; validate and upload through `xcrun altool` or Transporter or `xcodebuild -exportArchive` with `method: app-store-connect`. All identities, the Team ID and the profile are secrets or placeholders (file 06). The notarization step belongs to the internal disk image only; the App Store path does not need it.
- The internal disk image build stays as it is and keeps `directInstaller` (no screens).
- Build number: `CFBundleVersion` must increase on every upload; `CFBundleShortVersionString` is the public version.

### M8. Tests (all must exist)
Rust policy table tests (pure, with injected clocks): every row of M2; offline relaunch for trial and for buyer; first launch offline; clock back by 1 hour, 1 day, 7 days; clock forward then back (G4); a second trial transaction never moves the start; restore on a new Mac (guard empty, verified transaction present); revoked lifetime; corrupted or future-version guard record; sandbox transaction in a production build is ignored but accepted in a sandbox build.
Swift: fake `StoreKitSource` for every purchase result; no currency text in the licence strings; chip and reminder wording; reminder on Home.
StoreKit testing in Xcode with `StraftStudio.storekit` and `SKTestSession` (time rate to run a 7-day trial in minutes, refund, ask to buy, interrupted purchase). Manual sandbox checklist in `docs/release/mac-store-test.md`: reinstall, delete the app and its container, clock back and forward, airplane mode on day 7, refund, second Mac with the same Apple Account, second Apple Account (expected: new trial), Restore purchase.

### M9. Done when
- No server code, URL, secret or document remains; the app trial works with the network cable pulled after the first online start.
- All rows in M2 and the guard rules G1 to G4 are tested.
- The store build runs sandboxed with the entitlements above, the privacy manifest is present, and the pipeline produces a validated package from CI with placeholders.
- Mocks 187, 188, 189, 190, 192 and 201 match the code.
- File 05's Mac items are done and tested.
